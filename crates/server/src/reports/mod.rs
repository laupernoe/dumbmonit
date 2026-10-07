//! Rapports périodiques par courriel (hebdomadaire par défaut, quotidien ou
//! mensuel) : disponibilité par équipement et par dossier, incidents de la
//! période, équipements les plus instables, comparaison avec la période d'avant.
//!
//! - [`schedule`] : calendrier, période, règle « dû » (anti-doublon, pas de
//!   rattrapage rétroactif) — pur.
//! - [`collect`] : chiffres, depuis VictoriaMetrics et l'historique des alertes.
//! - [`render`] : courriel HTML et texte.
//! - [`store`] : table `report_schedules`.
//!
//! Le courriel part par un canal SMTP existant : les réglages du serveur de
//! messagerie restent ceux des notifications, sans second endroit à tenir.

pub mod collect;
pub mod render;
pub mod schedule;
pub mod store;

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use tokio::time::{MissedTickBehavior, interval};
use tracing::{info, warn};

use crate::db;
use crate::notify::smtp::Smtp;
use crate::notify::{self, ChannelConfig};
use crate::state::AppState;
use schedule::{Frequency, Schedule};
use store::StoredSchedule;

/// Destinataires par calendrier. Au-delà, c'est une liste de diffusion, et une
/// liste de diffusion se tient dans la messagerie, pas ici.
pub const MAX_RECIPIENTS: usize = 20;
/// Calendriers par instance.
pub const MAX_SCHEDULES: i64 = 10;
/// Délai minimal entre deux aperçus d'un même calendrier.
const PREVIEW_COOLDOWN: Duration = Duration::from_secs(30);

/// Adresse plausible et sûre à passer à un en-tête : une seule adresse, sans
/// espace, sans caractère de contrôle ni de séparation (`,`, `;`, `<`, `>`, `"`).
pub fn normalise_email(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty()
        || raw.len() > 254
        || raw.chars().any(|c| {
            c.is_whitespace() || c.is_control() || !c.is_ascii() || "<>,;\"'()[]\\".contains(c)
        })
    {
        return None;
    }
    let (local, domain) = raw.split_once('@')?;
    if local.is_empty() || local.len() > 64 || domain.contains('@') || !domain.contains('.') {
        return None;
    }
    let valid_domain = domain.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    });
    if !valid_domain {
        return None;
    }
    let normalised = format!("{local}@{}", domain.to_ascii_lowercase());
    lettre::Address::from_str(&normalised).ok()?;
    Some(normalised)
}

/// Valide et dédoublonne la liste de destinataires.
pub fn normalise_recipients(raw: &[String]) -> Result<Vec<String>, String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for entry in raw {
        let Some(email) = normalise_email(entry) else {
            let shown: String = entry.chars().filter(|c| !c.is_control()).take(60).collect();
            return Err(format!("\"{shown}\" is not a valid email address."));
        };
        if seen.insert(email.to_ascii_lowercase()) {
            out.push(email);
        }
    }
    if out.len() > MAX_RECIPIENTS {
        return Err(format!("A report can have at most {MAX_RECIPIENTS} recipients."));
    }
    Ok(out)
}

/// Calendrier prêt à calculer, ou `None` si la ligne stockée est illisible
/// (fréquence ou fuseau inconnus : ligne éditée à la main).
pub fn schedule_of(stored: &StoredSchedule) -> Option<Schedule> {
    Some(Schedule {
        frequency: Frequency::parse(&stored.frequency)?,
        weekday: stored.weekday,
        day_of_month: stored.day_of_month,
        hour: stored.hour,
        tz: schedule::parse_timezone(&stored.timezone)?,
    })
}

/// Prochain envoi prévu, pour l'interface.
pub fn next_run(stored: &StoredSchedule, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    if !stored.enabled || stored.recipients.is_empty() {
        return None;
    }
    let calendar = schedule_of(stored)?;
    // Un créneau antérieur à l'armement ne part jamais : le prochain est
    // forcément après `armed_at`.
    calendar.next_slot(now.max(stored.armed_at))
}

/// Canal SMTP à utiliser : celui du calendrier, sinon le premier actif.
async fn resolve_channel(
    state: &AppState,
    stored: &StoredSchedule,
) -> Result<ChannelConfig, String> {
    let channels = db::alerts::list_channels(&state.pool, &state.cipher)
        .await
        .map_err(|_| "Cannot read the notification channels.".to_string())?;
    let found = match stored.channel_id {
        Some(id) => channels.into_iter().find(|c| c.id == id),
        None => channels.into_iter().find(|c| c.enabled && c.kind == "smtp"),
    };
    match found {
        Some(channel) if channel.kind == "smtp" && channel.enabled => Ok(channel),
        Some(_) => Err("The selected channel is not an enabled email (SMTP) channel.".to_string()),
        None => Err("No enabled email (SMTP) notification channel is configured.".to_string()),
    }
}

/// Résultat d'un envoi : combien de destinataires l'ont reçu.
#[derive(Debug)]
pub struct Delivery {
    pub sent: usize,
    pub failed: usize,
}

/// Construit le rapport et l'envoie à chaque destinataire du calendrier.
///
/// Les erreurs renvoyées sont déjà expurgées (jamais de secret SMTP) et écrites
/// pour être montrées telles quelles à l'utilisateur.
pub async fn deliver(
    state: &AppState,
    stored: &StoredSchedule,
    preview: bool,
    now: DateTime<Utc>,
) -> Result<Delivery, String> {
    if notify::sending_disabled() {
        return Err(notify::DISABLED_MESSAGE.to_string());
    }
    if stored.recipients.is_empty() {
        return Err("Add at least one recipient first.".to_string());
    }
    let calendar = schedule_of(stored).ok_or("The schedule is unreadable.".to_string())?;
    let channel = resolve_channel(state, stored).await?;
    let smtp = Smtp::new(&channel).map_err(|error| error.to_string())?;

    let data = collect::collect(state, &stored.name, calendar.frequency, now)
        .await
        .map_err(|error| {
            warn!(%error, "report: data collection failed");
            "Cannot gather the report data.".to_string()
        })?;
    let rendered = render::render(&data, calendar.tz, preview);

    let mut delivery = Delivery { sent: 0, failed: 0 };
    let mut first_error = None;
    for recipient in &stored.recipients {
        match smtp
            .send_html(recipient, &rendered.subject, rendered.text.clone(), rendered.html.clone())
            .await
        {
            Ok(()) => delivery.sent += 1,
            Err(error) => {
                delivery.failed += 1;
                warn!(%error, "report: email not sent");
                first_error.get_or_insert(error.to_string());
            }
        }
    }
    match (delivery.sent, first_error) {
        (0, Some(error)) => Err(error),
        _ => Ok(delivery),
    }
}

static LAST_PREVIEW: LazyLock<Mutex<HashMap<i64, Instant>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Réserve un aperçu pour ce calendrier. Faux dans les trente secondes qui
/// suivent le précédent : le bouton ne doit pas servir à inonder une boîte.
pub fn try_reserve_preview(id: i64) -> bool {
    let mut map = LAST_PREVIEW.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let now = Instant::now();
    if map.get(&id).is_some_and(|at| now.duration_since(*at) < PREVIEW_COOLDOWN) {
        return false;
    }
    map.retain(|_, at| now.duration_since(*at) < PREVIEW_COOLDOWN);
    map.insert(id, now);
    true
}

/// Envoie les rapports échus à `now`. Rend le nombre de calendriers servis.
pub async fn run_due(state: &AppState, now: DateTime<Utc>) -> anyhow::Result<usize> {
    // Démonstration : aucun envoi ne part, inutile de réclamer des créneaux.
    if notify::sending_disabled() {
        return Ok(0);
    }
    let mut served = 0;
    for stored in store::list(&state.pool).await? {
        if !stored.enabled {
            continue;
        }
        let Some(calendar) = schedule_of(&stored) else { continue };
        let Some(slot) = calendar.due_slot(now, stored.armed_at, stored.last_sent_at) else {
            continue;
        };
        // Réclamé avant l'envoi : au pire un rapport est perdu (et l'erreur
        // affichée), jamais envoyé deux fois.
        if !store::claim(&state.pool, stored.id, slot, now).await? {
            continue;
        }
        served += 1;
        match deliver(state, &stored, false, now).await {
            Ok(delivery) => {
                info!(
                    schedule = stored.id,
                    sent = delivery.sent,
                    failed = delivery.failed,
                    "report sent"
                );
                if delivery.failed > 0 {
                    store::set_error(
                        &state.pool,
                        stored.id,
                        Some(&format!("{} recipient(s) could not be reached.", delivery.failed)),
                    )
                    .await?;
                }
            }
            Err(error) => {
                warn!(schedule = stored.id, %error, "report not sent");
                store::set_error(&state.pool, stored.id, Some(&error)).await?;
            }
        }
    }
    Ok(served)
}

/// Lance la boucle de vérification : une passe par minute.
pub fn spawn(state: AppState) {
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(60));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            if let Err(error) = run_due(&state, Utc::now()).await {
                warn!(?error, "report scheduler pass failed");
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use store::ScheduleInput;

    fn at(iso: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(iso).expect("ISO date").with_timezone(&Utc)
    }

    #[test]
    fn les_adresses_valides_sont_normalisees() {
        assert_eq!(normalise_email(" Ops@Example.ORG "), Some("Ops@example.org".to_string()));
        assert_eq!(
            normalise_email("a.b+tag@mail.example.org").as_deref(),
            Some("a.b+tag@mail.example.org")
        );
    }

    #[test]
    fn les_adresses_dangereuses_sont_refusees() {
        for bad in [
            "",
            "plain",
            "a@b",
            "a@@b.org",
            "a b@c.org",
            "a@b.org, c@d.org",
            "<a@b.org>",
            "a@b.org\r\nBcc: x@y.org",
            "\"a\"@b.org",
            "a@-b.org",
            "é@b.org",
        ] {
            assert_eq!(normalise_email(bad), None, "{bad:?} should be refused");
        }
    }

    #[test]
    fn la_liste_de_destinataires_est_dedoublonnee_et_bornee() {
        let list = vec!["a@b.org".to_string(), "A@B.org".to_string(), "c@d.org".to_string()];
        assert_eq!(normalise_recipients(&list).expect("valid").len(), 2);

        let many: Vec<String> = (0..=MAX_RECIPIENTS).map(|i| format!("u{i}@example.org")).collect();
        assert!(normalise_recipients(&many).is_err());
        assert!(normalise_recipients(&["nope".to_string()]).is_err());
    }

    fn input() -> ScheduleInput {
        ScheduleInput {
            name: "Weekly".into(),
            enabled: true,
            frequency: "weekly".into(),
            weekday: 0,
            day_of_month: 1,
            hour: 8,
            timezone: "UTC".into(),
            recipients: vec!["ops@example.org".into()],
            channel_id: None,
        }
    }

    #[tokio::test]
    async fn un_creneau_n_est_reclame_qu_une_fois() {
        let dir = tempfile::tempdir().expect("tempdir");
        let pool = db::open(&dir.path().join("t.db")).await.expect("db");
        let armed = at("2026-09-01T00:00:00Z");
        let id = store::insert(&pool, &input(), armed).await.expect("insert");

        let stored = store::get(&pool, id).await.expect("get").expect("row");
        let calendar = schedule_of(&stored).expect("schedule");
        let now = at("2026-10-05T08:01:00Z");
        let slot = calendar.due_slot(now, stored.armed_at, stored.last_sent_at).expect("due");

        assert!(store::claim(&pool, id, slot, now).await.expect("claim"));
        // Une seconde passe (ou un second processus) ne peut plus le réclamer…
        assert!(!store::claim(&pool, id, slot, now).await.expect("claim again"));
        // … et le calendrier ne le dit plus dû.
        let stored = store::get(&pool, id).await.expect("get").expect("row");
        assert_eq!(
            calendar.due_slot(at("2026-10-05T08:02:00Z"), stored.armed_at, stored.last_sent_at),
            None
        );
        // Le créneau suivant, lui, redevient réclamable.
        let next = at("2026-10-12T08:00:00Z");
        assert!(store::claim(&pool, id, next, at("2026-10-12T08:01:00Z")).await.expect("next"));
    }

    #[tokio::test]
    async fn un_calendrier_desactive_n_est_jamais_reclame() {
        let dir = tempfile::tempdir().expect("tempdir");
        let pool = db::open(&dir.path().join("t.db")).await.expect("db");
        let mut disabled = input();
        disabled.enabled = false;
        let id = store::insert(&pool, &disabled, at("2026-09-01T00:00:00Z")).await.expect("insert");
        let slot = at("2026-10-05T08:00:00Z");
        assert!(!store::claim(&pool, id, slot, slot).await.expect("claim"));
    }

    #[tokio::test]
    async fn modifier_l_horaire_rearme_sans_rattrapage() {
        let dir = tempfile::tempdir().expect("tempdir");
        let pool = db::open(&dir.path().join("t.db")).await.expect("db");
        let id = store::insert(&pool, &input(), at("2026-09-01T00:00:00Z")).await.expect("insert");
        let current = store::get(&pool, id).await.expect("get").expect("row");

        let mut moved = input();
        moved.hour = 9;
        let now = at("2026-10-05T08:30:00Z");
        store::update(&pool, &current, &moved, now).await.expect("update");
        let updated = store::get(&pool, id).await.expect("get").expect("row");
        assert_eq!(updated.armed_at, now);
        let calendar = schedule_of(&updated).expect("schedule");
        // Le créneau de 09:00 du jour est dans le futur : rien de dû à 08:31.
        assert_eq!(calendar.due_slot(at("2026-10-05T08:31:00Z"), updated.armed_at, None), None);
        // Changer seulement les destinataires ne réarme pas.
        let mut renamed = input();
        renamed.hour = 9;
        renamed.recipients = vec!["other@example.org".into()];
        store::update(&pool, &updated, &renamed, at("2026-10-06T00:00:00Z")).await.expect("update");
        let again = store::get(&pool, id).await.expect("get").expect("row");
        assert_eq!(again.armed_at, now);
    }

    #[test]
    fn un_apercu_ne_peut_pas_etre_rafale() {
        assert!(try_reserve_preview(987_654));
        assert!(!try_reserve_preview(987_654));
    }
}
