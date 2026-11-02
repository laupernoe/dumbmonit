//! Changements de configuration à faire après une mise à jour.
//!
//! Certaines versions demandent un geste hors de l'image : publier un port
//! dans le `docker-compose.yml`, ajouter une variable d'environnement. Une
//! mise à jour (`docker compose pull && up -d`) ne le fait jamais : le fichier
//! appartient à l'utilisateur. Le catalogue [`CATALOG`] liste ces gestes, par
//! version, avec l'explication et le bloc à recopier ; l'interface les
//! présente à l'administrateur juste après la mise à jour.
//!
//! Qui est concerné :
//!
//! * l'instance enregistre au démarrage la version qu'elle exécute
//!   ([`record_version`]) et celle dont elle vient (`upgrade_notes_from`) ;
//! * une note s'applique si sa version est dans la fenêtre `]from, courante]`
//!   (les pré-versions comptent pour leur version finale), **ou** si sa
//!   détection automatique ([`Check`]) constate que le changement manque
//!   encore — c'est ce qui rattrape une instance mise à jour il y a longtemps ;
//! * l'administrateur ferme une note ([`close_note`]) : elle ne revient plus
//!   jamais pour cette version. Tant qu'elle n'est pas fermée, l'interface la
//!   garde sous les yeux.
//!
//! Une instance neuve (aucun compte au premier démarrage) n'a rien à changer :
//! sa fenêtre est vide. Une instance antérieure à cette fonction, qui n'a
//! jamais enregistré sa version, est supposée venir de [`BASELINE`].

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use serde::Serialize;
use sqlx::SqlitePool;

use crate::state::AppState;
use crate::update::Version;

/// Dernière version publiée avant que les instances n'enregistrent la leur :
/// celle dont est supposée venir une instance existante sans historique.
pub const BASELINE: &str = "0.14.2";

const KEY_SEEN: &str = "upgrade_notes_seen";
const KEY_FROM: &str = "upgrade_notes_from";
const KEY_ANNOUNCED: &str = "upgrade_notes_announced";

// ----------------------------------------------------------------- catalogue

/// Vérification automatique d'un changement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Check {
    /// Le canal chiffré des agents répond-il depuis l'adresse qu'on lui connaît ?
    AgentChannelPort,
}

/// Un changement de déploiement demandé par une version.
#[derive(Debug)]
pub struct Note {
    /// Identifiant stable, en minuscules et tirets ; clé de la fermeture.
    pub id: &'static str,
    /// Version qui introduit le changement (`x.y.z`).
    pub version: &'static str,
    pub title: &'static str,
    pub summary: &'static str,
    /// Bloc `docker-compose.yml` à recopier.
    pub compose: &'static str,
    /// Commande à lancer ensuite, s'il y en a une.
    pub apply: Option<&'static str>,
    /// Page de la documentation qui détaille le changement.
    pub docs_url: Option<&'static str>,
    pub check: Option<Check>,
}

/// Les changements connus, du plus ancien au plus récent.
///
/// Ajouter une entrée suffit : l'interface rend n'importe quelle note.
pub const CATALOG: &[Note] = &[
    Note {
        id: "agent-channel-port",
        version: "0.14.0",
        title: "Publish the encrypted agent port (8443)",
        summary: "Since 0.14.0, agents talk to the server over a dedicated encrypted channel \
                  (TLS 1.3 with client certificates) on port 8443. A docker-compose.yml written \
                  before 0.14.0 does not publish that port: remote agents and relays cannot \
                  enrol or switch to the encrypted channel. Add the line marked new to the \
                  ports of the dumbmonit service, then recreate the container. Agents on the same \
                  Docker network are not affected.",
        compose: "services:\n  dumbmonit:\n    ports:\n      - \"8080:8080\"   # already \
                  there\n      - \"8443:8443\"   # new: encrypted agent channel",
        apply: Some("docker compose up -d"),
        docs_url: Some("https://dumbmonit.readthedocs.io/en/latest/install/agent-security/"),
        check: Some(Check::AgentChannelPort),
    },
    Note {
        id: "native-https",
        version: "0.15.0",
        title: "Encrypt the web interface (HTTPS)",
        summary: "DumbMonit now speaks HTTPS itself, on the same port 8080: nothing new to \
                  publish. It uses a self-signed certificate until you give it yours (mount it and \
                  set the two variables below). Behind a reverse proxy, list the proxy's address \
                  so DumbMonit trusts its X-Forwarded-* headers. Then turn on the HTTP to HTTPS \
                  redirect in Settings, System, HTTPS and encryption (new installations have it on).",
        compose: "services:\n  dumbmonit:\n    environment:\n      # optional: your own \
                  certificate\n      DUMBMONIT_TLS_CERT: /certs/fullchain.pem\n      \
                  DUMBMONIT_TLS_KEY: /certs/privkey.pem\n      # behind a reverse proxy only\n      \
                  DUMBMONIT_TRUSTED_PROXIES: 172.16.0.0/12\n    volumes:\n      - \
                  ./certs:/certs:ro",
        apply: Some("docker compose up -d"),
        docs_url: Some("https://dumbmonit.readthedocs.io/en/latest/install/https/"),
        check: None,
    },
];

// ------------------------------------------------------------------ versions

/// La version sans pré-version ni métadonnées : `0.15.0-alpha.3` → `0.15.0`.
/// Une note de la 0.15.0 s'applique déjà à ses pré-versions.
fn core(text: &str) -> Option<Version> {
    Version::parse(text.split(['-', '+']).next()?)
}

/// Vrai si une note de `note` concerne une instance passée de `from`
/// (exclu) à `current` (inclus).
pub fn in_window(note: &str, from: &str, current: &str) -> bool {
    match (core(note), core(from), core(current)) {
        (Some(note), Some(from), Some(current)) => note > from && note <= current,
        _ => false,
    }
}

/// Début de la fenêtre après un changement de version `prev` → `current`.
///
/// Elle avance à `prev`, sauf si une note de l'ancienne fenêtre est restée
/// ouverte (`unresolved`) : on ne perd pas de vue ce que l'administrateur n'a
/// pas encore traité parce qu'il a mis à jour deux fois de suite. Un simple
/// changement de pré-version ne la bouge pas ; un retour en arrière la recule.
pub fn next_from(stored_from: &str, prev: &str, current: &str, unresolved: bool) -> String {
    let (Some(stored), Some(before), Some(now)) = (core(stored_from), core(prev), core(current))
    else {
        return stored_from.to_string();
    };
    if now < before {
        return if stored > now { current.to_string() } else { stored_from.to_string() };
    }
    if now == before || unresolved {
        return stored_from.to_string();
    }
    prev.to_string()
}

// --------------------------------------------------------------------- états

/// Où en est une note pour cette instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// À traiter : dans la fenêtre, ou détectée comme manquante.
    Pending,
    /// Fermée par un administrateur.
    Closed,
    /// Rien à faire : hors fenêtre, ou la détection constate que c'est fait.
    Clear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DetectionState {
    /// Le changement est en place.
    Ok,
    /// Le changement manque.
    Missing,
    /// La vérification n'a pas pu conclure.
    Unknown,
}

/// Verdict d'une vérification automatique.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Detection {
    pub state: DetectionState,
    pub detail: String,
}

impl Detection {
    fn new(state: DetectionState, detail: impl Into<String>) -> Self {
        Self { state, detail: detail.into() }
    }
}

/// État d'une note : fermée avant tout ; sinon la détection tranche quand elle
/// conclut, et la fenêtre de version quand elle ne peut pas.
pub fn status(closed: bool, windowed: bool, detection: Option<&Detection>) -> Status {
    if closed {
        return Status::Closed;
    }
    match detection.map(|d| d.state) {
        Some(DetectionState::Missing) => Status::Pending,
        Some(DetectionState::Ok) => Status::Clear,
        Some(DetectionState::Unknown) | None if windowed => Status::Pending,
        Some(DetectionState::Unknown) | None => Status::Clear,
    }
}

// ----------------------------------------------------------------- détection

/// Ce que l'on a pu observer du canal des agents.
#[derive(Debug, Default)]
pub struct ChannelFacts {
    /// Port que les agents composent ; `None` quand le canal est coupé.
    pub port: Option<u16>,
    /// Adresse sondée, quand il y en avait une exploitable.
    pub host: Option<String>,
    /// Résultat du sondage TCP ; `None` quand il n'a pas pu avoir lieu.
    pub reachable: Option<bool>,
    /// Agents qui ont parlé par le canal chiffré ces 7 derniers jours.
    pub recent_tls_agents: i64,
}

/// Le verdict, d'après les observations.
pub fn judge_agent_port(facts: &ChannelFacts) -> Detection {
    let Some(port) = facts.port else {
        return Detection::new(
            DetectionState::Ok,
            "The encrypted agent channel is turned off (DUMBMONIT_AGENT_LISTEN=off): there is \
             nothing to publish.",
        );
    };
    match (&facts.host, facts.reachable) {
        (Some(host), Some(true)) => {
            Detection::new(DetectionState::Ok, format!("Port {port} of {host} answers."))
        }
        (Some(host), Some(false)) => Detection::new(
            DetectionState::Missing,
            format!(
                "Port {port} of {host} does not answer from the server. If it is published and \
                 your agents reach it, a NAT loop-back can hide it from here: mark the change \
                 as done."
            ),
        ),
        _ if facts.recent_tls_agents > 0 => Detection::new(
            DetectionState::Ok,
            format!(
                "{} agent(s) reported through the encrypted channel in the last 7 days.",
                facts.recent_tls_agents
            ),
        ),
        _ => Detection::new(
            DetectionState::Unknown,
            format!(
                "Cannot be checked from here: you reach the server through a local address. \
                 Make sure port {port} is published and reachable from your agents."
            ),
        ),
    }
}

/// Adresse que le serveur peut sonder : un nom ou une IP bien formés, qui ne
/// désignent pas le serveur lui-même (sonder `localhost` depuis le conteneur
/// réussit toujours et ne prouve rien).
pub fn probe_host(host: &str) -> Option<String> {
    let host = host.trim().to_ascii_lowercase();
    let bare = host.trim_start_matches('[').trim_end_matches(']');
    let well_formed = !bare.is_empty()
        && bare.len() <= 253
        && bare.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b':'));
    let local = bare == "localhost"
        || bare.ends_with(".localhost")
        || bare == "::1"
        || bare == "::"
        || bare == "0.0.0.0"
        || bare.starts_with("127.");
    (well_formed && !local).then_some(bare.to_string())
}

const PROBE_TTL: Duration = Duration::from_secs(60);
const PROBE_TIMEOUT: Duration = Duration::from_millis(1500);

/// Résultats récents des sondes : (hôte, port) → (instant du test, joignable).
type ProbeCache = HashMap<(String, u16), (Instant, bool)>;

static PROBES: LazyLock<Mutex<ProbeCache>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Une connexion TCP vers `host:port`, gardée une minute en mémoire pour que
/// le rechargement de la page n'attende pas un délai à chaque fois.
pub async fn probe(host: &str, port: u16, fresh: bool) -> bool {
    let key = (host.to_string(), port);
    if !fresh
        && let Ok(cache) = PROBES.lock()
        && let Some((at, reachable)) = cache.get(&key)
        && at.elapsed() < PROBE_TTL
    {
        return *reachable;
    }
    let connect = tokio::net::TcpStream::connect((host, port));
    let reachable = tokio::time::timeout(PROBE_TIMEOUT, connect).await.is_ok_and(|c| c.is_ok());
    if let Ok(mut cache) = PROBES.lock() {
        // Les clés viennent d'en-têtes `Host` : on borne la table.
        if cache.len() >= 32 {
            cache.clear();
        }
        cache.insert(key, (Instant::now(), reachable));
    }
    reachable
}

/// Exécute la vérification d'une note.
async fn run_check(state: &AppState, check: Check, host: Option<&str>, fresh: bool) -> Detection {
    match check {
        Check::AgentChannelPort => {
            let port = state.config.agent_port();
            // L'adresse publique réglée l'emporte sur celle du navigateur.
            let global = crate::notify::policy_store::load_global(&state.pool).await.ok();
            let env_url = crate::config::env_var("DUMBMONIT_PUBLIC_URL");
            let public = global
                .and_then(|global| global.public_url(env_url.as_deref()))
                .and_then(|url| crate::status_host::host_of_url(&url));
            let host = public.as_deref().or(host).and_then(probe_host);
            let reachable = match (port, &host) {
                (Some(port), Some(host)) => Some(probe(host, port, fresh).await),
                _ => None,
            };
            let since = chrono::Utc::now().timestamp_millis() - 7 * 24 * 3600 * 1000;
            let recent_tls_agents: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM agent_hosts WHERE tls_seen_ms > ?")
                    .bind(since)
                    .fetch_one(&state.pool)
                    .await
                    .unwrap_or(0);
            judge_agent_port(&ChannelFacts { port, host, reachable, recent_tls_agents })
        }
    }
}

// ------------------------------------------------------------------ stockage

async fn setting(pool: &SqlitePool, key: &str) -> Result<Option<String>> {
    sqlx::query_scalar("SELECT value FROM app_settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
        .with_context(|| format!("lecture du réglage {key}"))
}

async fn set_setting(pool: &SqlitePool, key: &str, value: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO app_settings (key, value) VALUES (?, ?) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await
    .with_context(|| format!("écriture du réglage {key}"))?;
    Ok(())
}

/// Une note fermée, telle que la base la garde et que la sauvegarde la porte.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct ClosedRow {
    pub version: String,
    pub note_id: String,
    /// UTC sans suffixe.
    pub closed_at: String,
    pub closed_by: String,
}

pub async fn closed_rows(pool: &SqlitePool) -> Result<Vec<ClosedRow>> {
    sqlx::query_as(
        "SELECT version, note_id, closed_at, closed_by FROM upgrade_notes_closed \
         ORDER BY version, note_id",
    )
    .fetch_all(pool)
    .await
    .context("lecture des changements fermés")
}

/// Ferme une note du catalogue. Vrai si elle venait d'être fermée.
pub async fn close_note(pool: &SqlitePool, note: &Note, by: &str) -> Result<bool> {
    let done = sqlx::query(
        "INSERT INTO upgrade_notes_closed (version, note_id, closed_by) VALUES (?, ?, ?) \
         ON CONFLICT (version, note_id) DO NOTHING",
    )
    .bind(note.version)
    .bind(note.id)
    .bind(by)
    .execute(pool)
    .await
    .context("fermeture du changement")?;
    Ok(done.rows_affected() > 0)
}

/// Enregistre la version exécutée, et celle dont l'instance vient.
///
/// Appelée à chaque démarrage ; sans effet tant que la version ne change pas.
pub async fn record_version(pool: &SqlitePool, current: &str) -> Result<()> {
    let seen = setting(pool, KEY_SEEN).await?;
    if seen.as_deref() == Some(current) {
        return Ok(());
    }
    let from = match seen {
        Some(prev) => {
            let stored = setting(pool, KEY_FROM).await?.unwrap_or_else(|| prev.clone());
            let closed = closed_rows(pool).await?;
            let unresolved = CATALOG.iter().any(|note| {
                in_window(note.version, &stored, &prev)
                    && !closed.iter().any(|c| c.version == note.version && c.note_id == note.id)
            });
            next_from(&stored, &prev, current, unresolved)
        }
        None => {
            let existing = crate::auth::users::count(pool).await? > 0;
            // Une instance neuve n'a rien à changer ; une instance déjà en
            // service sans historique vient de la dernière version connue.
            if existing { BASELINE.to_string() } else { current.to_string() }
        }
    };
    set_setting(pool, KEY_FROM, &from).await?;
    set_setting(pool, KEY_SEEN, current).await?;
    Ok(())
}

/// Note que la fenêtre a été montrée pour la version courante.
pub async fn mark_announced(pool: &SqlitePool, current: &str) -> Result<()> {
    set_setting(pool, KEY_ANNOUNCED, current).await
}

// ---------------------------------------------------------------------- vue

/// Une note, telle que l'interface la reçoit.
#[derive(Debug, Serialize)]
pub struct NoteView {
    pub id: &'static str,
    pub version: &'static str,
    pub title: &'static str,
    pub summary: &'static str,
    pub compose: &'static str,
    pub apply: Option<&'static str>,
    pub docs_url: Option<&'static str>,
    pub status: Status,
    /// Verdict de la détection automatique, quand la note en a une et n'est pas fermée.
    pub detection: Option<Detection>,
    /// Quand elle a été fermée, UTC sans suffixe.
    pub closed_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Overview {
    /// Version exécutée.
    pub current: String,
    /// Version dont l'instance vient : début de la fenêtre des notes.
    pub upgraded_from: Option<String>,
    /// La fenêtre doit s'ouvrir d'elle-même : des notes sont à traiter et elle
    /// n'a pas encore été montrée pour cette version.
    pub announce: bool,
    /// Nombre de notes à traiter.
    pub pending: usize,
    /// Toutes les notes du catalogue : à traiter d'abord, puis fermées, puis le reste.
    pub notes: Vec<NoteView>,
}

impl Overview {
    /// Identifiants des notes à traiter.
    pub fn pending_ids(&self) -> Vec<&'static str> {
        self.notes.iter().filter(|n| n.status == Status::Pending).map(|n| n.id).collect()
    }
}

/// L'état des notes pour cette instance. `host` est l'adresse par laquelle
/// l'administrateur joint le serveur ; `fresh` contourne la mémoire des sondages.
pub async fn overview(
    state: &AppState,
    current: &str,
    host: Option<&str>,
    fresh: bool,
) -> Result<Overview> {
    // Le mode démonstration n'a ni compose ni agents réels.
    if state.config.demo {
        return Ok(Overview {
            current: current.to_string(),
            upgraded_from: None,
            announce: false,
            pending: 0,
            notes: Vec::new(),
        });
    }
    record_version(&state.pool, current).await?;
    let from = setting(&state.pool, KEY_FROM).await?;
    let announced = setting(&state.pool, KEY_ANNOUNCED).await?;
    let closed = closed_rows(&state.pool).await?;

    let mut notes = Vec::with_capacity(CATALOG.len());
    for note in CATALOG {
        let closed_at = closed
            .iter()
            .find(|c| c.version == note.version && c.note_id == note.id)
            .map(|c| c.closed_at.clone());
        let windowed = from.as_deref().is_some_and(|from| in_window(note.version, from, current));
        // Inutile de sonder ce qui est déjà fermé.
        let detection = match (note.check, &closed_at) {
            (Some(check), None) => Some(run_check(state, check, host, fresh).await),
            _ => None,
        };
        notes.push(NoteView {
            id: note.id,
            version: note.version,
            title: note.title,
            summary: note.summary,
            compose: note.compose,
            apply: note.apply,
            docs_url: note.docs_url,
            status: status(closed_at.is_some(), windowed, detection.as_ref()),
            detection,
            closed_at,
        });
    }
    let rank = |status: Status| match status {
        Status::Pending => 0,
        Status::Closed => 1,
        Status::Clear => 2,
    };
    // Les plus récentes d'abord dans chaque groupe.
    notes.sort_by_key(|n| (rank(n.status), std::cmp::Reverse(core(n.version))));
    let pending = notes.iter().filter(|n| n.status == Status::Pending).count();
    Ok(Overview {
        current: current.to_string(),
        upgraded_from: from,
        announce: pending > 0 && announced.as_deref() != Some(current),
        pending,
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detection(state: DetectionState) -> Detection {
        Detection::new(state, "")
    }

    #[test]
    fn the_catalogue_is_well_formed() {
        let mut ids = std::collections::HashSet::new();
        let mut previous: Option<Version> = None;
        for note in CATALOG {
            assert!(ids.insert(note.id), "identifiant en double : {}", note.id);
            let kebab = |c: u8| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-';
            assert!(
                !note.id.is_empty() && note.id.bytes().all(kebab),
                "identifiant à écrire en minuscules et tirets : {}",
                note.id
            );
            let version = Version::parse(note.version).expect(note.version);
            assert!(!version.is_prerelease(), "{} : une version finale", note.id);
            assert_eq!(note.version.matches('.').count(), 2, "{} : x.y.z", note.id);
            if let Some(previous) = &previous {
                assert!(*previous <= version, "{} : du plus ancien au plus récent", note.id);
            }
            previous = Some(version);
            assert!(!note.title.is_empty() && !note.summary.is_empty(), "{}", note.id);
            assert!(note.compose.contains("services:"), "{} : un bloc compose", note.id);
            assert!(note.summary.len() < 700, "{} : une explication courte", note.id);
        }
    }

    #[test]
    fn the_agent_port_block_matches_the_shipped_compose_file() {
        let note = CATALOG.iter().find(|n| n.id == "agent-channel-port").expect("la note du port");
        assert_eq!(note.version, "0.14.0");
        assert_eq!(note.check, Some(Check::AgentChannelPort));
        let shipped = include_str!("../../../docker-compose.yml");
        assert!(shipped.contains("\"8443:8443\""), "le compose livré publie le port des agents");
        assert!(note.compose.contains("\"8443:8443\""));
        assert_eq!(note.compose.lines().count(), 5, "services, service, ports, deux lignes");
    }

    #[test]
    fn a_note_applies_between_the_previous_and_the_current_version() {
        // `from` exclu, `current` inclus.
        assert!(in_window("0.15.0", "0.14.3", "0.15.0"));
        assert!(in_window("0.14.4", "0.14.3", "0.15.0"));
        assert!(!in_window("0.14.3", "0.14.3", "0.15.0"), "déjà dans la version d'origine");
        assert!(!in_window("0.15.1", "0.14.3", "0.15.0"), "pas encore");
        assert!(!in_window("0.14.0", "0.14.3", "0.15.0"), "plus ancienne");
        // Une pré-version compte pour sa version finale.
        assert!(in_window("0.15.0", "0.14.3", "0.15.0-alpha.3"));
        assert!(!in_window("0.15.0", "0.15.0-alpha.1", "0.15.0-alpha.3"));
        // Version illisible : jamais de note.
        assert!(!in_window("0.15.0", "dev", "0.15.0"));
        assert!(!in_window("0.15", "0.14.3", "0.15.0"));
    }

    #[test]
    fn the_window_moves_only_when_nothing_is_left_open() {
        // Mise à jour normale : la fenêtre repart de la version quittée.
        assert_eq!(next_from("0.14.3", "0.15.0", "0.16.0", false), "0.15.0");
        // Une note de l'ancienne fenêtre est restée ouverte : on garde l'origine.
        assert_eq!(next_from("0.14.3", "0.15.0", "0.16.0", true), "0.14.3");
        // Une autre pré-version de la même version ne change rien.
        assert_eq!(next_from("0.14.3", "0.15.0-alpha.2", "0.15.0-alpha.3", false), "0.14.3");
        // Retour en arrière : la fenêtre ne dépasse jamais la version courante.
        assert_eq!(next_from("0.15.0", "0.16.0", "0.14.3", false), "0.14.3");
        assert_eq!(next_from("0.14.0", "0.16.0", "0.15.0", false), "0.14.0");
    }

    #[test]
    fn a_closed_note_stays_closed_and_detection_decides_otherwise() {
        let missing = detection(DetectionState::Missing);
        let ok = detection(DetectionState::Ok);
        let unknown = detection(DetectionState::Unknown);
        assert_eq!(status(true, true, Some(&missing)), Status::Closed);
        assert_eq!(status(true, false, None), Status::Closed);
        // Manque, même hors de la fenêtre : c'est ce qui rattrape une vieille mise à jour.
        assert_eq!(status(false, false, Some(&missing)), Status::Pending);
        // En place : rien à faire, même dans la fenêtre.
        assert_eq!(status(false, true, Some(&ok)), Status::Clear);
        // Sans conclusion ou sans détection : la fenêtre décide.
        assert_eq!(status(false, true, Some(&unknown)), Status::Pending);
        assert_eq!(status(false, true, None), Status::Pending);
        assert_eq!(status(false, false, Some(&unknown)), Status::Clear);
        assert_eq!(status(false, false, None), Status::Clear);
    }

    #[test]
    fn the_agent_port_verdict_follows_what_was_observed() {
        let facts = |port, host: Option<&str>, reachable, recent| ChannelFacts {
            port,
            host: host.map(str::to_string),
            reachable,
            recent_tls_agents: recent,
        };
        // Canal coupé volontairement : rien à publier.
        let off = judge_agent_port(&facts(None, None, None, 0));
        assert_eq!(off.state, DetectionState::Ok);
        // Le sondage tranche, dans un sens comme dans l'autre.
        let up = judge_agent_port(&facts(Some(8443), Some("monit.lan"), Some(true), 0));
        assert_eq!(up.state, DetectionState::Ok);
        assert!(up.detail.contains("8443") && up.detail.contains("monit.lan"));
        let down = judge_agent_port(&facts(Some(8443), Some("monit.lan"), Some(false), 3));
        assert_eq!(down.state, DetectionState::Missing);
        // Sans sondage : un agent vu en chiffré prouve que ça passe, sinon on ne sait pas.
        let seen = judge_agent_port(&facts(Some(8443), None, None, 2));
        assert_eq!(seen.state, DetectionState::Ok);
        let unknown = judge_agent_port(&facts(Some(8443), None, None, 0));
        assert_eq!(unknown.state, DetectionState::Unknown);
    }

    #[test]
    fn only_sensible_hosts_are_probed() {
        assert_eq!(probe_host("monit.example.com").as_deref(), Some("monit.example.com"));
        assert_eq!(probe_host(" Monit.LAN ").as_deref(), Some("monit.lan"));
        assert_eq!(probe_host("192.168.1.10").as_deref(), Some("192.168.1.10"));
        assert_eq!(probe_host("[fd00::10]").as_deref(), Some("fd00::10"));
        // Le serveur lui-même : sonder ne prouverait rien.
        for local in ["localhost", "127.0.0.1", "[::1]", "0.0.0.0", "app.localhost"] {
            assert_eq!(probe_host(local), None, "{local}");
        }
        // Pas un nom d'hôte.
        for bad in ["", "a b", "evil.com/path", "user@host", "host?x=1", "a;b"] {
            assert_eq!(probe_host(bad), None, "{bad}");
        }
    }

    #[tokio::test]
    async fn the_probe_tells_an_open_port_from_a_closed_one() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(probe("127.0.0.1", port, true).await);
        drop(listener);
        assert!(!probe("127.0.0.1", port, true).await);
    }
}
