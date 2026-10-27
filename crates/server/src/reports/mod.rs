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
pub mod logo;
pub mod pdf;
pub mod render;
pub mod schedule;
pub mod sla;
pub mod store;

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, Datelike, Timelike, Utc};
use tokio::time::{MissedTickBehavior, interval};
use tracing::{info, warn};

use crate::auth::clients;
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

/// Habillage d'un rapport SLA : le client (mode MSP : son nom et son logo),
/// sinon le nom de la page de statut choisie, à défaut « DumbMonit ».
pub async fn sla_options(
    state: &AppState,
    page_id: Option<i64>,
    client: Option<i64>,
    objective: Option<f64>,
    now: DateTime<Utc>,
) -> sla::Options {
    if let Some(client) = client
        && let Ok(found) = clients::refs(&state.pool, Some(std::slice::from_ref(&client))).await
        && let Some(found) = found.into_iter().next()
    {
        let stored = if found.has_logo {
            clients::logo(&state.pool, client).await.ok().flatten()
        } else {
            None
        };
        let logo = stored.and_then(|(_, bytes)| logo::for_pdf(&bytes));
        return sla::Options { brand: found.name, objective, generated: now, logo };
    }
    let brand = match page_id {
        Some(id) => db::status_pages::get_page(&state.pool, id).await.ok().flatten(),
        None => None,
    }
    .map_or_else(|| "DumbMonit".to_string(), |page| page.title);
    sla::Options { brand, objective, generated: now, logo: None }
}

/// Les fichiers d'un rapport SLA : (nom, type MIME, octets).
pub fn sla_attachments(
    data: &collect::ReportData,
    tz: chrono_tz::Tz,
    options: &sla::Options,
    pdf: bool,
    csv: bool,
) -> Vec<(String, String, Vec<u8>)> {
    let stem = sla::file_stem(data, tz);
    let mut files = Vec::new();
    if pdf {
        files.push((
            format!("{stem}.pdf"),
            "application/pdf".to_string(),
            sla::pdf(data, tz, options),
        ));
    }
    if csv {
        let monitors = sla::csv_monitors(data, options.objective).into_bytes();
        files.push((format!("{stem}-monitors.csv"), CSV_TYPE.to_string(), monitors));
        let incidents = sla::csv_incidents(data).into_bytes();
        files.push((format!("{stem}-incidents.csv"), CSV_TYPE.to_string(), incidents));
    }
    files
}

pub const CSV_TYPE: &str = "text/csv; charset=utf-8";

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
    // Mode MSP : le calendrier d'un client ne rapporte que sur ce client.
    let client = clients::owner_of(&state.pool, clients::Kind::Report, stored.id)
        .await
        .map_err(|_| "Cannot read the client of this schedule.".to_string())?
        .flatten();

    let sla = stored.kind == "sla";
    let (data, mut rendered) = if sla {
        // Le mois civil entier qui précède l'envoi, dans le fuseau du calendrier.
        let period = schedule::Period::previous_calendar_month(now, calendar.tz)
            .ok_or("Cannot work out the month to report.".to_string())?;
        sla_email(state, &stored.name, period, calendar.tz, client, preview, now).await?
    } else {
        let period = schedule::Period::ending_at(now, calendar.frequency);
        let data = collect::collect_period_for(
            state,
            &stored.name,
            calendar.frequency,
            period,
            now,
            client,
        )
        .await
        .map_err(|error| {
            warn!(%error, "report: data collection failed");
            "Cannot gather the report data.".to_string()
        })?;
        let rendered = render::render(&data, calendar.tz, preview);
        (data, rendered)
    };
    rendered.subject = rendered.subject.replace(['\r', '\n'], " ");

    let attachments = if sla {
        let options = sla_options(state, stored.page_id, client, stored.objective, now).await;
        sla_attachments(&data, calendar.tz, &options, stored.attach_pdf, stored.attach_csv)
    } else {
        Vec::new()
    };
    send_all(&smtp, &stored.recipients, &rendered, &attachments).await
}

/// Données et courriel d'un rapport SLA mensuel (un client, ou toute
/// l'instance).
async fn sla_email(
    state: &AppState,
    name: &str,
    period: schedule::Period,
    tz: chrono_tz::Tz,
    client: Option<i64>,
    preview: bool,
    now: DateTime<Utc>,
) -> Result<(collect::ReportData, render::Rendered), String> {
    let data = collect::collect_period_for(state, name, Frequency::Monthly, period, now, client)
        .await
        .map_err(|error| {
            warn!(%error, "report: data collection failed");
            "Cannot gather the report data.".to_string()
        })?;
    let mut rendered = render::render(&data, tz, preview);
    let count = data.incidents.len();
    rendered.subject = format!(
        "{}[DumbMonit] SLA report, {}: {} availability, {count} incident{}",
        if preview { "[Preview] " } else { "" },
        sla::month_label(&data, tz),
        data.fleet_uptime.map_or_else(|| "n/a".to_string(), |v| format!("{v:.2}%")),
        if count == 1 { "" } else { "s" }
    );
    Ok((data, rendered))
}

/// Envoie le courriel à chaque destinataire ; échec seulement si aucun ne l'a
/// reçu.
async fn send_all(
    smtp: &Smtp,
    recipients: &[String],
    rendered: &render::Rendered,
    attachments: &[(String, String, Vec<u8>)],
) -> Result<Delivery, String> {
    let mut delivery = Delivery { sent: 0, failed: 0 };
    let mut first_error = None;
    for recipient in recipients {
        let sent = if attachments.is_empty() {
            smtp.send_html(
                recipient,
                &rendered.subject,
                rendered.text.clone(),
                rendered.html.clone(),
            )
            .await
        } else {
            smtp.send_html_with_attachments(
                recipient,
                &rendered.subject,
                rendered.text.clone(),
                rendered.html.clone(),
                attachments.to_vec(),
            )
            .await
        };
        match sent {
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

/// Heure locale (dans le fuseau du client) à partir de laquelle le rapport SLA
/// mensuel d'un client part, le premier du mois.
const CLIENT_REPORT_HOUR: u32 = 8;

/// Mode MSP : envoie le rapport SLA du mois écoulé aux contacts de chaque client
/// qui l'a demandé. Un mois n'est servi qu'une fois (réclamé avant l'envoi,
/// comme les calendriers) ; un mois manqué n'est pas rattrapé le mois suivant.
pub async fn run_client_reports(state: &AppState, now: DateTime<Utc>) -> anyhow::Result<usize> {
    if notify::sending_disabled() || !clients::enabled(&state.pool).await? {
        return Ok(0);
    }
    let rows: ClientReportRows = sqlx::query_as(
        "SELECT id, name, contacts, sla_objective, report_timezone, report_last_period
         FROM clients WHERE sla_report = 1",
    )
    .fetch_all(&state.pool)
    .await?;
    let mut served = 0;
    for (id, name, contacts, objective, timezone, last) in rows {
        let Some(tz) = schedule::parse_timezone(&timezone) else { continue };
        let local = now.with_timezone(&tz);
        if local.day() == 1 && local.hour() < CLIENT_REPORT_HOUR {
            continue;
        }
        let Some(period) = schedule::Period::previous_calendar_month(now, tz) else { continue };
        let label = period.start.with_timezone(&tz).format("%Y-%m").to_string();
        if last.as_deref() == Some(label.as_str()) {
            continue;
        }
        let claimed = sqlx::query(
            "UPDATE clients SET report_last_period = ?
             WHERE id = ? AND (report_last_period IS NULL OR report_last_period <> ?)",
        )
        .bind(&label)
        .bind(id)
        .bind(&label)
        .execute(&state.pool)
        .await?;
        if claimed.rows_affected() == 0 {
            continue;
        }
        served += 1;
        let contacts: Vec<String> = serde_json::from_str(&contacts).unwrap_or_default();
        let outcome =
            send_client_report(state, id, &name, &contacts, objective, period, tz, now).await;
        let error = match outcome {
            Ok(delivery) if delivery.failed > 0 => {
                Some(format!("{} contact(s) could not be reached.", delivery.failed))
            }
            Ok(_) => None,
            Err(error) => Some(error),
        };
        if let Some(error) = &error {
            warn!(client = id, %error, "client SLA report not fully sent");
        } else {
            info!(client = id, "client SLA report sent");
        }
        sqlx::query("UPDATE clients SET report_last_error = ? WHERE id = ?")
            .bind(error)
            .bind(id)
            .execute(&state.pool)
            .await?;
    }
    Ok(served)
}

type ClientReportRows = Vec<(i64, String, String, Option<f64>, String, Option<String>)>;

/// Canal SMTP d'un client : le sien s'il en a un actif, sinon celui de
/// l'instance. Jamais celui d'un autre client.
async fn client_channel(state: &AppState, client: i64) -> Result<ChannelConfig, String> {
    let channels = db::alerts::list_channels(&state.pool, &state.cipher)
        .await
        .map_err(|_| "Cannot read the notification channels.".to_string())?;
    let owners = clients::owners(&state.pool, clients::Kind::Channel)
        .await
        .map_err(|_| "Cannot read the notification channels.".to_string())?;
    let usable = |c: &&ChannelConfig| c.enabled && c.kind == "smtp";
    channels
        .iter()
        .filter(usable)
        .find(|c| owners.get(&c.id) == Some(&client))
        .or_else(|| channels.iter().filter(usable).find(|c| !owners.contains_key(&c.id)))
        .cloned()
        .ok_or_else(|| "No enabled email (SMTP) notification channel is configured.".to_string())
}

#[allow(clippy::too_many_arguments)]
async fn send_client_report(
    state: &AppState,
    client: i64,
    name: &str,
    contacts: &[String],
    objective: Option<f64>,
    period: schedule::Period,
    tz: chrono_tz::Tz,
    now: DateTime<Utc>,
) -> Result<Delivery, String> {
    if contacts.is_empty() {
        return Err("Add at least one contact to this client.".to_string());
    }
    let channel = client_channel(state, client).await?;
    let smtp = Smtp::new(&channel).map_err(|error| error.to_string())?;
    let title = format!("{name} availability report");
    let (data, mut rendered) =
        sla_email(state, &title, period, tz, Some(client), false, now).await?;
    rendered.subject = rendered.subject.replace(['\r', '\n'], " ");
    let options = sla_options(state, None, Some(client), objective, now).await;
    let attachments = sla_attachments(&data, tz, &options, true, true);
    send_all(&smtp, contacts, &rendered, &attachments).await
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
            if let Err(error) = run_client_reports(&state, Utc::now()).await {
                warn!(?error, "client report pass failed");
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
            kind: "digest".into(),
            attach_pdf: true,
            attach_csv: true,
            page_id: None,
            objective: None,
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
