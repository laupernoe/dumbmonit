//! Collecteur « push » : le moniteur en poussée, ou interrupteur d'homme mort.
//!
//! La panne silencieuse la plus courante chez soi n'est pas un équipement qui ne
//! répond plus : c'est un travail qui ne tourne plus. Le cron de sauvegarde
//! désactivé par une mise à jour, le script qui plante avant d'écrire, la
//! routine domotique cassée par un renommage — rien ne se plaint, et on s'en
//! aperçoit le jour où l'on a besoin de la sauvegarde.
//!
//! Le principe est celui d'healthchecks.io ou du moniteur « push » d'Uptime Kuma :
//! le travail appelle une URL secrète à chaque exécution (`api/push.rs`),
//! et c'est *l'absence* d'appel qui déclenche l'alerte. Comme le collecteur
//! « agent », `probe` n'interroge rien : il relit la date du dernier appel et
//! la compare à la période attendue.
//!
//! # Pourquoi `probe_success` et non une série qui s'interrompt
//!
//! Le socle détecte une cible injoignable par l'interruption de sa série `up`.
//! Ici, la cible n'est pas un équipement : `probe` renvoie toujours `Ok` (le
//! contrôle a tourné), et c'est `probe_success` qui porte le verdict, comme pour
//! les moniteurs de disponibilité. On obtient ainsi un taux de disponibilité
//! (`avg_over_time`), un historique daté des retards, et l'alerte « équipement
//! injoignable » ne se déclenche jamais en doublon : la règle livrée
//! `push_missed` est la seule à parler.
//!
//! # Métriques produites
//!
//! Étiquetées `probe="push"` en plus des étiquettes d'identité :
//!
//! | Métrique | Sens |
//! |---|---|
//! | `probe_success` | 1 si le dernier appel est dans les temps et n'a pas signalé d'échec, 0 sinon. Absente tant qu'aucun appel n'a été reçu. |
//! | `probe_failure_info` | Présence ; `reason` vaut `missed` (appel en retard) ou `reported_down` (le script a dit `status=down`). |
//! | `push_last_seen_seconds` | Âge du dernier appel. |
//! | `push_received_total` | Nombre d'appels reçus depuis la création du jeton. |
//! | `push_run_failed` | 1 si la dernière exécution terminée a échoué (`/fail` ou code de sortie non nul), 0 sinon. Absente tant qu'aucune exécution n'est terminée. |
//! | `push_run_too_long` | 1 si une exécution dure plus que la durée maximale (en cours ou terminée), 0 sinon. Absente tant qu'aucune exécution n'est connue. |
//! | `push_last_run_duration_seconds` | Durée de la dernière exécution terminée, quand un `/start` l'a précédée. |
//!
//! # Planning
//!
//! Le retard se juge sur un intervalle (`expected_interval`) ou, à la place, sur
//! une expression cron (`schedule`, voir [`cron`]) lue dans un fuseau
//! (`timezone`) : le travail est en retard quand la dernière échéance, plus la
//! tolérance, est passée sans signe de vie. Les exécutions (début, fin, code,
//! sortie) sont rangées dans `push_runs` ([`store`]).

pub mod cron;
pub mod store;
pub mod token;

use std::collections::BTreeMap;
use std::time::Duration;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, MetricKind, ProbeError, Sample, Target};
use sqlx::SqlitePool;

pub use cron::Schedule;
pub use store::{Monitor, Run, RunStatus, Status};

/// Type de cible, tel qu'enregistré dans `Target::kind`.
pub const KIND: &str = "push";

/// Étiquette d'option : période attendue entre deux appels.
pub const OPTION_EXPECTED_INTERVAL: &str = "expected_interval";
/// Étiquette d'option : tolérance au-delà de la période avant de conclure au
/// retard — une durée (`15m`) ou un pourcentage de la période (`10%`).
pub const OPTION_GRACE: &str = "grace";
/// Étiquette d'option : expression cron (cinq champs) qui remplace l'intervalle.
pub const OPTION_SCHEDULE: &str = "schedule";
/// Étiquette d'option : fuseau IANA dans lequel se lit l'expression cron.
pub const OPTION_TIMEZONE: &str = "timezone";
/// Étiquette d'option : durée au-delà de laquelle une exécution est « trop longue ».
pub const OPTION_MAX_DURATION: &str = "max_duration";

pub const DEFAULT_EXPECTED_INTERVAL: &str = "24h";
pub const DEFAULT_GRACE: &str = "10%";
pub const DEFAULT_TIMEZONE: &str = "UTC";

/// Période attendue minimale. En dessous, ce n'est plus un travail planifié
/// qu'on surveille mais un flux, et le contrôle de fraîcheur (à la période de la
/// cible, dix secondes au mieux) ne suivrait pas.
const MIN_EXPECTED: Duration = Duration::from_secs(10);
/// Période attendue maximale : un an. Au-delà, l'option est plus sûrement une
/// faute de frappe qu'un travail annuel.
const MAX_EXPECTED: Duration = Duration::from_secs(366 * 24 * 3600);
/// Tolérance plancher. Un cron « toutes les cinq minutes » avec dix pour cent
/// de tolérance serait déclaré en retard pour trente secondes de dérive — la
/// charge de la machine à ce moment-là, rien de plus.
const MIN_GRACE: Duration = Duration::from_secs(60);
/// Un signe de vie arrivé un peu avant l'échéance (horloges qui dérivent) compte
/// quand même pour elle.
const EARLY_SLACK_MS: i64 = 30_000;

/// Tolérance telle qu'écrite : durée fixe, ou part de la période.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GraceSpec {
    Fixed(Duration),
    Percent(u64),
}

impl GraceSpec {
    fn parse(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        if let Some(percent) = trimmed.strip_suffix('%') {
            let percent: u64 = percent.trim().parse().ok()?;
            return (percent <= 1000).then_some(Self::Percent(percent));
        }
        parse_duration(trimmed).map(Self::Fixed)
    }

    /// Tolérance effective pour une période de `base`, jamais sous le plancher.
    fn resolve(self, base: Duration) -> Duration {
        let grace = match self {
            Self::Fixed(grace) => grace,
            Self::Percent(percent) => Duration::from_secs(base.as_secs() * percent / 100),
        };
        grace.max(MIN_GRACE)
    }
}

/// Réglages d'un moniteur, lus dans les étiquettes de la cible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub expected: Duration,
    pub grace: Duration,
    /// Planning cron : quand il est présent, il remplace l'intervalle pour juger
    /// du retard.
    pub schedule: Option<Schedule>,
    /// Durée d'exécution au-delà de laquelle l'alerte « trop long » se déclenche.
    pub max_duration: Option<Duration>,
    grace_spec: GraceSpec,
}

impl Settings {
    /// Réglages à intervalle fixe (sans cron ni durée maximale).
    pub fn interval(expected: Duration, grace: Duration) -> Self {
        Self {
            expected,
            grace,
            schedule: None,
            max_duration: None,
            grace_spec: GraceSpec::Fixed(grace),
        }
    }

    /// Âge du dernier appel au-delà duquel le travail est en retard (mode
    /// intervalle).
    pub fn deadline(&self) -> Duration {
        self.expected + self.grace
    }

    /// Tolérance à l'instant donné. En mode cron, un pourcentage se prend sur
    /// l'écart entre l'échéance qui précède et celle qui suit.
    pub fn grace_at(&self, now_ms: i64) -> Duration {
        let Some(schedule) = &self.schedule else { return self.grace };
        let gap = schedule
            .at_or_before(now_ms)
            .zip(schedule.next_after(now_ms))
            .map(|(before, after)| Duration::from_millis((after - before).max(0) as u64));
        self.grace_spec.resolve(gap.unwrap_or(self.expected))
    }

    pub fn from_target(target: &Target) -> Result<Self, ProbeError> {
        Self::from_tags(&target.tags)
    }

    /// Lit les options dans les étiquettes. Sert aussi à refuser, à
    /// l'enregistrement d'une cible, un réglage illisible.
    pub fn from_tags(tags: &BTreeMap<String, String>) -> Result<Self, ProbeError> {
        let tag =
            |key: &str| tags.get(key).map(|value| value.trim()).filter(|value| !value.is_empty());
        let expected = match tag(OPTION_EXPECTED_INTERVAL) {
            None => parse_duration(DEFAULT_EXPECTED_INTERVAL).expect("default interval"),
            Some(raw) => parse_duration(raw).ok_or_else(|| {
                ProbeError::Config(format!(
                    "\"{OPTION_EXPECTED_INTERVAL}\" expects a duration such as 30m, 1h or 2d, got \"{raw}\""
                ))
            })?,
        };
        if !(MIN_EXPECTED..=MAX_EXPECTED).contains(&expected) {
            return Err(ProbeError::Config(format!(
                "\"{OPTION_EXPECTED_INTERVAL}\" must be between {} s and {} days",
                MIN_EXPECTED.as_secs(),
                MAX_EXPECTED.as_secs() / 86_400
            )));
        }
        let raw_grace = tag(OPTION_GRACE).unwrap_or(DEFAULT_GRACE);
        let grace_spec = GraceSpec::parse(raw_grace).ok_or_else(|| {
            ProbeError::Config(format!(
                "\"{OPTION_GRACE}\" expects a duration (15m) or a percentage of the interval (10%), got \"{raw_grace}\""
            ))
        })?;
        let schedule = match tag(OPTION_SCHEDULE) {
            None => None,
            Some(expression) => {
                let timezone = tag(OPTION_TIMEZONE).unwrap_or(DEFAULT_TIMEZONE);
                Some(Schedule::new(expression, timezone).map_err(|error| {
                    ProbeError::Config(format!(
                        "\"{OPTION_SCHEDULE}\" / \"{OPTION_TIMEZONE}\": {error}"
                    ))
                })?)
            }
        };
        let max_duration = match tag(OPTION_MAX_DURATION) {
            None => None,
            Some(raw) => {
                let duration = parse_duration(raw).filter(|d| !d.is_zero()).ok_or_else(|| {
                    ProbeError::Config(format!(
                        "\"{OPTION_MAX_DURATION}\" expects a duration such as 30m or 2h, got \"{raw}\""
                    ))
                })?;
                Some(duration)
            }
        };
        Ok(Self {
            expected,
            grace: grace_spec.resolve(expected),
            schedule,
            max_duration,
            grace_spec,
        })
    }
}

/// Lit une durée courte : un entier (secondes) ou des composantes suffixées —
/// `90`, `30s`, `15m`, `1h`, `2d`, `1h30m`. Les espaces sont tolérés.
pub fn parse_duration(raw: &str) -> Option<Duration> {
    let compact: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.is_empty() {
        return None;
    }
    if let Ok(seconds) = compact.parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }
    let mut total: u64 = 0;
    let mut number = String::new();
    for c in compact.chars() {
        if c.is_ascii_digit() {
            number.push(c);
            continue;
        }
        let unit: u64 = match c {
            's' | 'S' => 1,
            'm' | 'M' => 60,
            'h' | 'H' => 3600,
            'd' | 'D' => 86_400,
            'w' | 'W' => 7 * 86_400,
            _ => return None,
        };
        let value: u64 = number.parse().ok()?;
        number.clear();
        total = total.checked_add(value.checked_mul(unit)?)?;
    }
    if !number.is_empty() {
        return None; // un nombre sans unité après une composante : « 1h30 »
    }
    Some(Duration::from_secs(total))
}

/// Ce que le contrôle de fraîcheur conclut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Aucun appel reçu depuis la création du jeton : le script n'est pas encore
    /// en place. Ni en panne ni en bonne santé — rien n'est écrit.
    Waiting,
    /// Le dernier appel est dans les temps.
    OnTime,
    /// Le dernier appel est plus vieux que la période attendue plus la tolérance.
    Missed,
    /// Le dernier appel a lui-même signalé un échec (`status=down`).
    ReportedDown,
}

impl Verdict {
    /// Motif d'échec, en jeton d'étiquette ; `None` quand tout va bien.
    pub fn failure_reason(self) -> Option<&'static str> {
        match self {
            Self::Missed => Some("missed"),
            Self::ReportedDown => Some("reported_down"),
            Self::Waiting | Self::OnTime => None,
        }
    }
}

/// Juge un moniteur d'après son dernier appel.
///
/// Un appel qui signale `down` l'emporte sur l'horloge : le script a tourné et
/// dit lui-même que ça s'est mal passé, inutile d'attendre qu'il soit en retard.
///
/// Avec un planning cron, le travail est en retard quand la dernière échéance
/// dont la tolérance est écoulée n'a reçu aucun signe de vie (début ou fin).
pub fn evaluate(
    last_seen_ms: Option<i64>,
    status: Status,
    now_ms: i64,
    settings: &Settings,
) -> Verdict {
    let Some(last_seen_ms) = last_seen_ms else { return Verdict::Waiting };
    if status == Status::Down {
        return Verdict::ReportedDown;
    }
    if let Some(schedule) = &settings.schedule {
        let grace_ms = settings.grace_at(now_ms).as_millis() as i64;
        let due = schedule.at_or_before(now_ms.saturating_sub(grace_ms).saturating_sub(1));
        return match due {
            Some(due) if last_seen_ms < due - EARLY_SLACK_MS => Verdict::Missed,
            _ => Verdict::OnTime,
        };
    }
    if age(last_seen_ms, now_ms) > settings.deadline() { Verdict::Missed } else { Verdict::OnTime }
}

/// Ce que les exécutions récentes disent, indépendamment de l'horloge des appels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RunVerdict {
    /// La dernière exécution terminée a échoué.
    pub failed: bool,
    /// Une exécution dépasse la durée maximale (en cours ou terminée).
    pub too_long: bool,
}

/// Durée au-delà de laquelle une exécution commencée mais jamais terminée est
/// jugée bloquée, faute de durée maximale réglée : une période (l'écart jusqu'à
/// l'échéance suivante en mode cron) plus la tolérance.
pub fn open_run_limit(settings: &Settings, started_ms: i64) -> Duration {
    if let Some(max) = settings.max_duration {
        return max;
    }
    match &settings.schedule {
        Some(schedule) => {
            let period = schedule
                .next_after(started_ms)
                .map(|next| Duration::from_millis((next - started_ms).max(0) as u64))
                .unwrap_or(settings.expected);
            period + settings.grace_at(started_ms)
        }
        None => settings.deadline(),
    }
}

/// Juge les exécutions d'un moniteur.
///
/// `status` est le dernier statut déclaré par un appel simple : quand il vaut
/// `down` (`?status=down`), c'est déjà le verdict « heartbeat manqué » qui parle,
/// l'échec n'est pas annoncé une seconde fois.
pub fn evaluate_runs(
    last_finished: Option<&Run>,
    running: Option<&Run>,
    status: Status,
    now_ms: i64,
    settings: &Settings,
) -> RunVerdict {
    let failed =
        status == Status::Up && last_finished.is_some_and(|r| r.status == RunStatus::Failed);
    let long_running = running.is_some_and(|run| {
        let started = run.started_ms.unwrap_or(now_ms);
        age(started, now_ms) > open_run_limit(settings, started)
    });
    let long_finished = match (settings.max_duration, last_finished) {
        (Some(max), Some(run)) => {
            run.duration_ms.is_some_and(|millis| millis.max(0) as u128 > max.as_millis())
        }
        _ => false,
    };
    RunVerdict { failed, too_long: long_running || long_finished }
}

/// Longueur maximale de la sortie citée dans le message d'une alerte.
const DETAIL_OUTPUT_CHARS: usize = 400;

/// Ce que l'alerte « échec » dit d'une exécution : le code de sortie puis la
/// fin de la sortie (c'est là qu'est l'erreur), sur un texte court.
pub fn failure_detail(run: &Run) -> String {
    let head = match run.exit_code {
        Some(code) => format!("exit code {code}"),
        None => "reported a failure".to_string(),
    };
    let output = run.output.trim();
    if output.is_empty() {
        return head;
    }
    let count = output.chars().count();
    let tail: String = output.chars().skip(count.saturating_sub(DETAIL_OUTPUT_CHARS)).collect();
    let ellipsis = if count > DETAIL_OUTPUT_CHARS { "…" } else { "" };
    format!("{head}: {ellipsis}{}", tail.trim_start())
}

/// Âge d'un instant, jamais négatif — une horloge qui recule ne doit pas
/// produire une durée absurde.
pub fn age(then_ms: i64, now_ms: i64) -> Duration {
    Duration::from_millis(now_ms.saturating_sub(then_ms).max(0) as u64)
}

pub struct PushCollector {
    pool: SqlitePool,
}

impl PushCollector {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl Collector for PushCollector {
    fn kind(&self) -> &'static str {
        KIND
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let settings = Settings::from_target(target)?;
        let now_ms = chrono::Utc::now().timestamp_millis();
        // Une cible sans moniteur (créée par un client qui ignore ce type) est
        // simplement en attente : la page de l'équipement créera le jeton.
        let freshness = store::freshness(&self.pool, target.id).await.map_err(ProbeError::Other)?;
        let (last_seen_ms, status, received_total) = match freshness {
            Some(f) => (f.last_seen_ms, f.last_status, f.received_total),
            None => (None, Status::Up, 0),
        };

        let verdict = evaluate(last_seen_ms, status, now_ms, &settings);
        let gauge = |metric: &str, value: f64| {
            Sample::new(metric, value, MetricKind::Gauge, now_ms).with_label("probe", KIND)
        };
        let mut samples = vec![
            Sample::new("push_received_total", received_total as f64, MetricKind::Counter, now_ms)
                .with_label("probe", KIND),
        ];
        let Some(last_seen_ms) = last_seen_ms else {
            // En attente du premier appel : ni succès ni échec, `up` seul est posé
            // par le registre. L'interface affiche « Waiting ».
            return Ok(samples);
        };
        let state = store::run_state(&self.pool, target.id).await.map_err(ProbeError::Other)?;
        let runs = evaluate_runs(
            state.last_finished.as_ref(),
            state.running.as_ref(),
            status,
            now_ms,
            &settings,
        );
        if state.last_finished.is_some() {
            samples.push(gauge("push_run_failed", if runs.failed { 1.0 } else { 0.0 }));
        }
        if state.last_finished.is_some() || state.running.is_some() {
            samples.push(gauge("push_run_too_long", if runs.too_long { 1.0 } else { 0.0 }));
        }
        if let Some(duration_ms) = state.last_finished.as_ref().and_then(|run| run.duration_ms) {
            samples.push(gauge("push_last_run_duration_seconds", duration_ms as f64 / 1000.0));
        }
        samples.push(gauge("push_last_seen_seconds", age(last_seen_ms, now_ms).as_secs_f64()));
        samples.push(gauge("probe_success", if verdict == Verdict::OnTime { 1.0 } else { 0.0 }));
        if let Some(reason) = verdict.failure_reason() {
            samples.push(gauge("probe_failure_info", 1.0).with_label("reason", reason));
        }
        Ok(samples)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use dumbmonit_proto::{Credential, TargetId};

    use super::*;

    fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    fn target(tags: &[(&str, &str)]) -> Target {
        Target {
            id: 1,
            name: "nightly backup".into(),
            address: "nightly-backup".into(),
            kind: KIND.into(),
            profile_id: None,
            parent_id: None,
            interval: secs(60),
            enabled: true,
            tags: tags
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<BTreeMap<_, _>>(),
            credential: Credential::None,
            group_name: String::new(),
            position: 0,
        }
    }

    #[test]
    fn les_durees_se_lisent_en_secondes_ou_avec_suffixes() {
        assert_eq!(parse_duration("90"), Some(secs(90)));
        assert_eq!(parse_duration("30s"), Some(secs(30)));
        assert_eq!(parse_duration("15m"), Some(secs(900)));
        assert_eq!(parse_duration("1h"), Some(secs(3600)));
        assert_eq!(parse_duration("24H"), Some(secs(86_400)));
        assert_eq!(parse_duration("2d"), Some(secs(172_800)));
        assert_eq!(parse_duration("1w"), Some(secs(604_800)));
        assert_eq!(parse_duration("1h 30m"), Some(secs(5400)));
        assert_eq!(parse_duration(""), None);
        assert_eq!(parse_duration("abc"), None);
        assert_eq!(parse_duration("1h30"), None, "composante sans unité");
        assert_eq!(parse_duration("-5m"), None);
        assert_eq!(parse_duration("1.5h"), None);
    }

    #[test]
    fn la_tolerance_est_un_pourcentage_ou_une_duree_avec_un_plancher() {
        let s = Settings::from_target(&target(&[])).unwrap();
        assert_eq!(s.expected, secs(86_400), "défaut : un jour");
        assert_eq!(s.grace, secs(8640), "10 % d'un jour");

        let s = Settings::from_target(&target(&[("expected_interval", "1h"), ("grace", "15m")]))
            .unwrap();
        assert_eq!(s.grace, secs(900));
        assert_eq!(s.deadline(), secs(4500));

        // Dix pour cent de cinq minutes font trente secondes : le plancher l'emporte.
        let s = Settings::from_target(&target(&[("expected_interval", "5m")])).unwrap();
        assert_eq!(s.grace, MIN_GRACE);

        let s =
            Settings::from_target(&target(&[("expected_interval", "1h"), ("grace", "0")])).unwrap();
        assert_eq!(s.grace, MIN_GRACE, "zéro reste borné par le plancher");
    }

    #[test]
    fn une_option_illisible_est_une_erreur_de_configuration_et_non_une_panne() {
        for tags in [
            [("expected_interval", "soon"), ("grace", "10%")],
            [("expected_interval", "1h"), ("grace", "beaucoup")],
            [("expected_interval", "5s"), ("grace", "10%")],
            [("expected_interval", "400d"), ("grace", "10%")],
            [("expected_interval", "1h"), ("grace", "5000%")],
        ] {
            let error = Settings::from_target(&target(&tags)).unwrap_err();
            assert!(!error.means_down(), "{tags:?} : {error}");
            assert!(matches!(error, ProbeError::Config(_)), "{tags:?} : {error}");
        }
    }

    #[test]
    fn le_verdict_suit_lage_du_dernier_appel() {
        let settings = Settings::interval(secs(3600), secs(600));
        let now = 10_000_000_000;
        assert_eq!(evaluate(None, Status::Up, now, &settings), Verdict::Waiting);
        assert_eq!(evaluate(Some(now - 5_000), Status::Up, now, &settings), Verdict::OnTime);
        // Dans la tolérance : en retard de cinq minutes sur une heure, dix tolérées.
        assert_eq!(evaluate(Some(now - 3_900_000), Status::Up, now, &settings), Verdict::OnTime);
        // Juste au-delà de la période plus la tolérance.
        assert_eq!(evaluate(Some(now - 4_200_001), Status::Up, now, &settings), Verdict::Missed);
        // Un appel qui déclare `down` n'attend pas l'horloge.
        assert_eq!(
            evaluate(Some(now - 1_000), Status::Down, now, &settings),
            Verdict::ReportedDown
        );
        // Une horloge qui recule ne donne pas un âge négatif.
        assert_eq!(evaluate(Some(now + 60_000), Status::Up, now, &settings), Verdict::OnTime);
    }

    #[test]
    fn les_motifs_dechec_sont_des_jetons_de_requete() {
        for verdict in [Verdict::Missed, Verdict::ReportedDown] {
            let reason = verdict.failure_reason().unwrap();
            assert!(reason.chars().all(|c| c.is_ascii_lowercase() || c == '_'), "{reason}");
        }
        assert_eq!(Verdict::OnTime.failure_reason(), None);
        assert_eq!(Verdict::Waiting.failure_reason(), None);
    }

    fn utc(text: &str) -> i64 {
        chrono::DateTime::parse_from_rfc3339(text).unwrap().timestamp_millis()
    }

    fn cron_settings(extra: &[(&str, &str)]) -> Settings {
        let mut tags = vec![("schedule", "30 2 * * *"), ("timezone", "Europe/Paris")];
        tags.extend_from_slice(extra);
        Settings::from_target(&target(&tags)).unwrap()
    }

    #[test]
    fn un_planning_cron_se_lit_avec_son_fuseau_et_sa_duree_maximale() {
        let s = cron_settings(&[("max_duration", "2h"), ("grace", "15m")]);
        assert_eq!(s.schedule.as_ref().unwrap().expression(), "30 2 * * *");
        assert_eq!(s.schedule.as_ref().unwrap().timezone_name(), "Europe/Paris");
        assert_eq!(s.max_duration, Some(secs(7200)));
        assert_eq!(s.grace_at(utc("2026-05-01T00:00:00Z")), secs(900));
        // Sans fuseau, UTC ; sans planning, ni cron ni durée maximale.
        let s = Settings::from_target(&target(&[("schedule", "@daily")])).unwrap();
        assert_eq!(s.schedule.as_ref().unwrap().timezone_name(), DEFAULT_TIMEZONE);
        let s = Settings::from_target(&target(&[])).unwrap();
        assert!(s.schedule.is_none() && s.max_duration.is_none());
        // Un pourcentage se prend sur l'écart entre échéances : 10 % de 24 h.
        let s = cron_settings(&[]);
        assert_eq!(s.grace_at(utc("2026-05-01T00:00:00Z")), secs(8640));
    }

    #[test]
    fn un_planning_illisible_est_une_erreur_de_configuration() {
        for tags in [
            [("schedule", "* * * *"), ("timezone", "UTC")],
            [("schedule", "0 3 * * *"), ("timezone", "Mars/Olympus")],
            [("schedule", "61 3 * * *"), ("timezone", "UTC")],
            [("schedule", "@reboot"), ("timezone", "UTC")],
            [("max_duration", "bientôt"), ("timezone", "UTC")],
            [("max_duration", "0"), ("timezone", "UTC")],
        ] {
            let error = Settings::from_target(&target(&tags)).unwrap_err();
            assert!(matches!(error, ProbeError::Config(_)), "{tags:?} : {error}");
            assert!(!error.means_down());
        }
    }

    #[test]
    fn avec_un_cron_le_retard_se_juge_sur_la_derniere_echeance() {
        // 02:30 à Paris, tolérance 15 min. En mai : 00:30 UTC.
        let s = cron_settings(&[("grace", "15m")]);
        let seen_last_night = utc("2026-05-02T00:30:05Z");
        // Dans la tolérance de l'échéance du 3 : rien à reprocher, même sans appel.
        let now = utc("2026-05-03T00:40:00Z");
        assert_eq!(evaluate(Some(seen_last_night), Status::Up, now, &s), Verdict::OnTime);
        // L'échéance du 3 + 15 min est passée sans appel : en retard.
        let now = utc("2026-05-03T00:46:00Z");
        assert_eq!(evaluate(Some(seen_last_night), Status::Up, now, &s), Verdict::Missed);
        // Un début (ou une fin) reçu à l'heure règle la question jusqu'au lendemain.
        let seen_on_time = utc("2026-05-03T00:30:02Z");
        assert_eq!(evaluate(Some(seen_on_time), Status::Up, now, &s), Verdict::OnTime);
        let next_day = utc("2026-05-04T00:44:00Z");
        assert_eq!(evaluate(Some(seen_on_time), Status::Up, next_day, &s), Verdict::OnTime);
        let next_day = utc("2026-05-04T00:46:00Z");
        assert_eq!(evaluate(Some(seen_on_time), Status::Up, next_day, &s), Verdict::Missed);
        // Un appel légèrement en avance (horloges) compte pour l'échéance.
        let early = utc("2026-05-03T00:29:45Z");
        assert_eq!(evaluate(Some(early), Status::Up, now, &s), Verdict::OnTime);
        // Jamais appelé : en attente. Échec déclaré : il l'emporte.
        assert_eq!(evaluate(None, Status::Up, now, &s), Verdict::Waiting);
        assert_eq!(evaluate(Some(seen_on_time), Status::Down, now, &s), Verdict::ReportedDown);
    }

    #[test]
    fn le_retard_cron_suit_le_changement_dheure() {
        // 02:30 à Paris le lundi 30 mars 2026 : 00:30 UTC (heure d'été), alors que
        // la veille, jour du changement, l'échéance tombait à 01:00 UTC.
        let s = cron_settings(&[("grace", "10m")]);
        let seen = utc("2026-03-29T01:00:30Z");
        let at = |now: &str| evaluate(Some(seen), Status::Up, utc(now), &s);
        assert_eq!(at("2026-03-30T00:35:00Z"), Verdict::OnTime);
        assert_eq!(at("2026-03-30T00:41:00Z"), Verdict::Missed);
    }

    fn run(status: RunStatus, started: Option<i64>, duration: Option<i64>) -> Run {
        Run {
            id: 1,
            target_id: 1,
            started_ms: started,
            finished_ms: duration.zip(started).map(|(d, s)| s + d),
            duration_ms: duration,
            status,
            exit_code: None,
            output: String::new(),
        }
    }

    #[test]
    fn un_echec_est_annonce_une_fois_et_pas_si_le_heartbeat_parle_deja() {
        let s = Settings::interval(secs(3600), secs(600));
        let now = 1_000_000_000;
        let failed = run(RunStatus::Failed, Some(now - 5_000), Some(1_000));
        let ok = run(RunStatus::Success, Some(now - 5_000), Some(1_000));
        assert!(evaluate_runs(Some(&failed), None, Status::Up, now, &s).failed);
        assert!(!evaluate_runs(Some(&ok), None, Status::Up, now, &s).failed);
        assert!(!evaluate_runs(None, None, Status::Up, now, &s).failed);
        // `status=down` : c'est déjà `reported_down` qui alerte.
        assert!(!evaluate_runs(Some(&failed), None, Status::Down, now, &s).failed);
    }

    #[test]
    fn trop_long_selon_la_duree_maximale_ou_un_debut_sans_fin() {
        let now = 1_000_000_000_000;
        // Durée maximale d'une heure.
        let mut s = Settings::interval(secs(86_400), secs(600));
        s.max_duration = Some(secs(3600));
        let slow = run(RunStatus::Success, Some(now - 10_000_000), Some(4_000_000));
        let quick = run(RunStatus::Success, Some(now - 10_000_000), Some(60_000));
        assert!(evaluate_runs(Some(&slow), None, Status::Up, now, &s).too_long);
        assert!(!evaluate_runs(Some(&quick), None, Status::Up, now, &s).too_long);
        let running = |ago_ms: i64| run(RunStatus::Running, Some(now - ago_ms), None);
        assert!(evaluate_runs(None, Some(&running(3_700_000)), Status::Up, now, &s).too_long);
        assert!(!evaluate_runs(None, Some(&running(1_800_000)), Status::Up, now, &s).too_long);

        // Sans durée maximale : un début sans fin au-delà d'une période + tolérance.
        let s = Settings::interval(secs(3600), secs(600));
        assert!(!evaluate_runs(None, Some(&running(4_100_000)), Status::Up, now, &s).too_long);
        assert!(evaluate_runs(None, Some(&running(4_300_000)), Status::Up, now, &s).too_long);
        // En mode cron, la période est l'écart jusqu'à l'échéance suivante.
        let s = cron_settings(&[("grace", "10m")]);
        let started = utc("2026-05-03T00:30:00Z");
        assert_eq!(open_run_limit(&s, started), secs(86_400 + 600));
    }

    #[test]
    fn le_message_dechec_cite_le_code_et_la_fin_de_la_sortie() {
        let mut failed = run(RunStatus::Failed, None, None);
        assert_eq!(failure_detail(&failed), "reported a failure");
        failed.exit_code = Some(23);
        assert_eq!(failure_detail(&failed), "exit code 23");
        failed.output = "  rsync: connection unexpectedly closed\n".into();
        assert_eq!(failure_detail(&failed), "exit code 23: rsync: connection unexpectedly closed");
        failed.output = format!("{}FIN", "x".repeat(1000));
        let detail = failure_detail(&failed);
        assert!(detail.starts_with("exit code 23: …x"), "{detail}");
        assert!(detail.ends_with("xFIN"));
        assert!(detail.chars().count() < 440);
    }

    #[tokio::test]
    async fn les_executions_alimentent_les_metriques_et_la_purge_les_borne() {
        let (pool, cipher, id, _dir) = pool().await;
        let tags = [("expected_interval", "1h"), ("max_duration", "10m")];
        let target = Target { id, ..target(&tags) };
        let monitor = store::ensure(&pool, &cipher, id).await.expect("moniteur");
        let hash = token::fingerprint(&monitor.token);
        let now = chrono::Utc::now().timestamp_millis();
        let value = |samples: &[Sample], metric: &str| {
            samples.iter().find(|s| s.metric == metric).map(|s| s.value)
        };

        // Début puis échec : durée connue, échec annoncé, rien de trop long.
        store::record_ping(&pool, &hash, now - 90_000, Status::Up, "").await.unwrap();
        store::start_run(&pool, id, now - 90_000).await.unwrap();
        store::record_ping(&pool, &hash, now - 30_000, Status::Up, "").await.unwrap();
        store::finish_run(&pool, id, now - 30_000, true, Some(2), "boom").await.unwrap();
        let samples = PushCollector::new(pool.clone()).probe(&target).await.expect("ok");
        assert_eq!(value(&samples, "push_run_failed"), Some(1.0));
        assert_eq!(value(&samples, "push_run_too_long"), Some(0.0));
        assert_eq!(value(&samples, "push_last_run_duration_seconds"), Some(60.0));
        assert_eq!(value(&samples, "probe_success"), Some(1.0), "il a bien appelé");

        // Un début resté sans fin au-delà de dix minutes : trop long.
        store::start_run(&pool, id, now - 700_000).await.unwrap();
        let samples = PushCollector::new(pool.clone()).probe(&target).await.expect("ok");
        assert_eq!(value(&samples, "push_run_too_long"), Some(1.0));

        // La fin sans début précédent : durée inconnue, succès.
        store::finish_run(&pool, id, now, false, Some(0), "").await.unwrap();
        let runs = store::list_runs(&pool, id, None, 10).await.unwrap();
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].status, RunStatus::Success, "{runs:?}");
        assert_eq!(runs[0].duration_ms, Some(700_000), "le début ouvert est refermé");

        store::finish_run(&pool, id, now, false, None, "").await.unwrap();
        let runs = store::list_runs(&pool, id, None, 10).await.unwrap();
        assert_eq!(runs[0].started_ms, None);
        assert_eq!(runs[0].duration_ms, None);

        // Un nouveau début remplace celui qui n'avait pas fini.
        store::start_run(&pool, id, now).await.unwrap();
        store::start_run(&pool, id, now + 1).await.unwrap();
        let runs = store::list_runs(&pool, id, None, 10).await.unwrap();
        assert_eq!(runs[0].status, RunStatus::Running);
        assert_eq!(runs[1].status, RunStatus::Lost);
    }

    #[tokio::test]
    async fn la_purge_garde_les_dernieres_executions_et_les_recentes() {
        let (pool, _cipher, id, _dir) = pool().await;
        let now = chrono::Utc::now().timestamp_millis();
        for n in 0..(store::KEEP_RUNS + 25) {
            store::finish_run(&pool, id, now - 1_000 + n, false, Some(0), "").await.unwrap();
        }
        let kept = store::list_runs(&pool, id, None, 1_000).await.unwrap();
        assert_eq!(kept.len() as i64, store::KEEP_RUNS);
        // Les plus récentes d'abord, et la pagination par curseur enchaîne.
        let page = store::list_runs(&pool, id, None, 50).await.unwrap();
        let next = store::list_runs(&pool, id, Some(page.last().unwrap().id), 50).await.unwrap();
        assert!(page[0].id > page[49].id && page[49].id > next[0].id);

        // Au-delà de l'âge maximal, une exécution disparaît à la purge suivante.
        let old = now - (store::KEEP_DAYS + 1) * 86_400_000;
        store::finish_run(&pool, id, old, false, Some(0), "ancienne").await.unwrap();
        store::finish_run(&pool, id, now, false, Some(0), "").await.unwrap();
        let kept = store::list_runs(&pool, id, None, 1_000).await.unwrap();
        assert!(kept.iter().all(|run| run.output != "ancienne"));
    }

    #[test]
    fn la_sortie_est_tronquee_sans_casser_lutf8() {
        assert_eq!(store::truncate_output(b"ok\n"), "ok\n");
        // Octets invalides remplacés plutôt que refusés.
        assert_eq!(store::truncate_output(&[b'a', 0xff, b'b']), "a\u{fffd}b");
        // 5000 caractères de 3 octets : on garde la fin, sans couper un caractère.
        let long = "€".repeat(5000);
        let cut = store::truncate_output(long.as_bytes());
        assert!(cut.len() <= store::MAX_OUTPUT_BYTES);
        assert!(cut.len() > store::MAX_OUTPUT_BYTES - 3);
        assert!(cut.chars().all(|c| c == '€'));
        let tail = format!("{}fin", "z".repeat(20_000));
        let cut = store::truncate_output(tail.as_bytes());
        assert_eq!(cut.len(), store::MAX_OUTPUT_BYTES);
        assert!(cut.ends_with("zfin"));
    }

    /// Base vierge avec une cible `push` ; renvoie aussi son identifiant.
    async fn pool() -> (SqlitePool, crate::crypto::Cipher, TargetId, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("répertoire temporaire");
        let pool = crate::db::open(&dir.path().join("test.db")).await.expect("base");
        let cipher = crate::db::init_cipher(&pool, "secret-de-test-suffisamment-long")
            .await
            .expect("chiffrement");
        let id = crate::db::targets::create(
            &pool,
            &cipher,
            &crate::db::targets::TargetInput {
                name: "nightly backup".into(),
                address: "nightly-backup".into(),
                kind: KIND.into(),
                profile_id: None,
                parent_id: None,
                via_agent: None,
                interval: secs(60),
                enabled: true,
                tags: Default::default(),
                credential: None,
                group_name: String::new(),
            },
        )
        .await
        .expect("cible");
        (pool, cipher, id, dir)
    }

    #[tokio::test]
    async fn une_cible_jamais_appelee_attend_sans_ecrire_de_verdict() {
        let (pool, cipher, id, _dir) = pool().await;
        let target = Target { id, ..target(&[("expected_interval", "1h")]) };

        // Sans moniteur du tout, puis avec un moniteur jamais appelé : même chose.
        for _ in 0..2 {
            let samples = PushCollector::new(pool.clone()).probe(&target).await.expect("ok");
            assert!(!samples.iter().any(|s| s.metric == "probe_success"), "{samples:?}");
            let total = samples.iter().find(|s| s.metric == "push_received_total").unwrap();
            assert_eq!(total.value, 0.0);
            store::ensure(&pool, &cipher, target.id).await.expect("moniteur");
        }
    }

    #[tokio::test]
    async fn un_appel_recent_est_un_succes_et_un_appel_ancien_un_echec_date() {
        let (pool, cipher, id, _dir) = pool().await;
        let target = Target { id, ..target(&[("expected_interval", "1h"), ("grace", "5m")]) };
        let monitor = store::ensure(&pool, &cipher, target.id).await.expect("moniteur");
        let hash = token::fingerprint(&monitor.token);
        let now = chrono::Utc::now().timestamp_millis();

        store::record_ping(&pool, &hash, now - 30_000, Status::Up, "").await.unwrap();
        let samples = PushCollector::new(pool.clone()).probe(&target).await.expect("ok");
        let success = samples.iter().find(|s| s.metric == "probe_success").expect("verdict");
        assert_eq!(success.value, 1.0);
        assert_eq!(success.labels.get("probe").map(String::as_str), Some(KIND));
        let age = samples.iter().find(|s| s.metric == "push_last_seen_seconds").unwrap();
        assert!((29.0..=40.0).contains(&age.value), "âge : {}", age.value);
        assert!(!samples.iter().any(|s| s.metric == "probe_failure_info"));

        // Deux heures de silence sur une période d'une heure : en retard, et le
        // point à zéro est bien écrit — c'est lui que la règle `push_missed` lit.
        store::record_ping(&pool, &hash, now - 7_200_000, Status::Up, "").await.unwrap();
        let samples = PushCollector::new(pool.clone()).probe(&target).await.expect("ok");
        assert_eq!(samples.iter().find(|s| s.metric == "probe_success").unwrap().value, 0.0);
        let info = samples.iter().find(|s| s.metric == "probe_failure_info").expect("motif");
        assert_eq!(info.labels.get("reason").map(String::as_str), Some("missed"));
        let total = samples.iter().find(|s| s.metric == "push_received_total").unwrap();
        assert_eq!(total.value, 2.0);

        // Le script signale lui-même un échec.
        store::record_ping(&pool, &hash, now, Status::Down, "rsync exit 23").await.unwrap();
        let samples = PushCollector::new(pool.clone()).probe(&target).await.expect("ok");
        assert_eq!(samples.iter().find(|s| s.metric == "probe_success").unwrap().value, 0.0);
        let info = samples.iter().find(|s| s.metric == "probe_failure_info").expect("motif");
        assert_eq!(info.labels.get("reason").map(String::as_str), Some("reported_down"));
    }

    #[tokio::test]
    async fn regenerer_le_jeton_coupe_lancienne_url_et_garde_le_compteur() {
        let (pool, cipher, id, _dir) = pool().await;
        let before = store::ensure(&pool, &cipher, id).await.expect("moniteur");
        let now = chrono::Utc::now().timestamp_millis();
        let old_hash = token::fingerprint(&before.token);
        assert_eq!(
            store::record_ping(&pool, &old_hash, now, Status::Up, "").await.unwrap(),
            Some(id)
        );

        let after = store::regenerate(&pool, &cipher, id).await.expect("régénération");
        assert_ne!(after.token, before.token);
        assert_eq!(after.received_total, 1);
        assert_eq!(store::record_ping(&pool, &old_hash, now, Status::Up, "").await.unwrap(), None);
        let new_hash = token::fingerprint(&after.token);
        assert_eq!(
            store::record_ping(&pool, &new_hash, now, Status::Up, "").await.unwrap(),
            Some(id)
        );
        assert_eq!(store::get(&pool, &cipher, id).await.unwrap().unwrap().received_total, 2);
    }

    #[tokio::test]
    async fn le_message_dun_appel_est_tronque() {
        let (pool, cipher, id, _dir) = pool().await;
        let monitor = store::ensure(&pool, &cipher, id).await.expect("moniteur");
        let hash = token::fingerprint(&monitor.token);
        let long = "x".repeat(store::MAX_MESSAGE_CHARS * 3);
        store::record_ping(&pool, &hash, 0, Status::Up, &long).await.unwrap();
        let stored = store::get(&pool, &cipher, id).await.unwrap().unwrap();
        assert_eq!(stored.last_message.chars().count(), store::MAX_MESSAGE_CHARS);
    }
}
