//! Boucle d'évaluation et orchestration des entrées/sorties.
//!
//! Tout le raisonnement vit dans [`crate::alerting::cycle`] ; ce module se contente
//! d'aller chercher les données, d'appeler le cycle, puis d'écrire et de notifier.
//! La règle qui gouverne le fichier : rien de ce qui échoue ici — VictoriaMetrics
//! muet, canal en panne, base verrouillée — ne doit interrompre la boucle.

use std::collections::HashMap;
use std::time::Duration;

use anyhow::Result;
use chrono::{DateTime, TimeDelta, Utc};
use sqlx::SqlitePool;
use tokio::time::{MissedTickBehavior, interval};
use tracing::{debug, info, warn};

use crate::alerting::baseline;
use crate::alerting::cycle::{self, CycleInput, RuleObservations};
use crate::alerting::notify_policy::{self, Outgoing, Recipient};
use crate::alerting::source::MetricSource;
use crate::crypto::Cipher;
use crate::db;
use crate::notify;
use crate::notify::policy_store;
use crate::state::AppState;
use crate::stats::stats;

/// Réglages de la boucle, lus dans l'environnement.
///
/// Ils ne passent pas par `Config` : ce fichier appartient à un autre lot de
/// travail, et l'alerting n'a pas besoin d'y ajouter des champs pour fonctionner.
#[derive(Debug, Clone, Copy)]
pub struct AlertingConfig {
    /// Période d'évaluation.
    pub interval: Duration,
    /// Durée de conservation de l'historique des transitions.
    pub history_retention: Duration,
    /// Durée au-delà de laquelle la baseline d'une série disparue est oubliée.
    pub baseline_retention: Duration,
}

impl Default for AlertingConfig {
    fn default() -> Self {
        Self {
            // Trente secondes : assez réactif pour une alerte « injoignable » avec un
            // `for` d'une minute, assez espacé pour ne pas marteler VictoriaMetrics.
            interval: Duration::from_secs(30),
            history_retention: Duration::from_secs(90 * 24 * 3600),
            baseline_retention: Duration::from_secs(60 * 24 * 3600),
        }
    }
}

impl AlertingConfig {
    pub fn from_env() -> Self {
        let default = Self::default();
        Self {
            interval: env_secs("DUMBMONIT_ALERT_INTERVAL_SECS", default.interval)
                // Descendre sous dix secondes ne rendrait rien plus réactif : la
                // période d'interrogation minimale d'une cible est déjà de dix secondes.
                .max(Duration::from_secs(10)),
            // La variable est exprimée en jours, comme son nom l'indique.
            history_retention: env_days("DUMBMONIT_ALERT_HISTORY_DAYS", default.history_retention),
            baseline_retention: default.baseline_retention,
        }
    }
}

fn env_days(key: &str, default: Duration) -> Duration {
    crate::config::env_var(key)
        .and_then(|raw| raw.parse::<u64>().ok())
        .filter(|days| *days > 0)
        .map_or(default, |days| Duration::from_secs(days * 86_400))
}

fn env_secs(key: &str, default: Duration) -> Duration {
    crate::config::env_var(key)
        .and_then(|raw| raw.parse::<u64>().ok())
        .filter(|secs| *secs > 0)
        .map_or(default, Duration::from_secs)
}

/// Bilan d'un cycle, renvoyé pour les tests et les journaux.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct EvalReport {
    pub rules_evaluated: usize,
    pub rules_failed: usize,
    pub alerts_tracked: usize,
    pub firing: usize,
    pub suppressed: usize,
    pub silenced: usize,
    pub learning: usize,
    pub groups: usize,
    pub notifications_sent: usize,
    pub notifications_failed: usize,
}

/// Démarre la boucle d'évaluation.
///
/// À appeler depuis `main.rs`, exactement comme `scheduler::spawn`.
pub fn spawn(state: AppState) {
    let config = AlertingConfig::from_env();
    tokio::spawn(async move {
        run(state, config).await;
    });
}

async fn run(state: AppState, config: AlertingConfig) {
    match db::alerts::seed_builtin_rules(&state.pool).await {
        Ok(0) => debug!("built-in rules already present"),
        Ok(count) => info!(count, "built-in alert rules installed"),
        // Sans les règles livrées, l'utilisateur peut toujours créer les siennes :
        // ce n'est pas une raison pour renoncer à alerter.
        Err(error) => warn!(?error, "could not install built-in rules"),
    }

    let http = notify::http_client();
    let mut ticker = interval(config.interval);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    info!(interval = ?config.interval, "alerting engine started");

    let mut last_maintenance = Utc::now();

    loop {
        ticker.tick().await;
        let now = Utc::now();

        let started = std::time::Instant::now();
        let outcome = evaluate_once(&state.pool, &state.cipher, &state.victoria, &http, now).await;
        // Le cycle est publié sur `/metrics` même quand il échoue : sa durée et
        // le nombre de règles en échec sont précisément ce qu'on veut voir alors.
        let (evaluated, failed) = match &outcome {
            Ok(report) => (report.rules_evaluated, report.rules_failed),
            Err(_) => (0, 0),
        };
        stats().alerting_cycle(started.elapsed(), evaluated, failed);

        match outcome {
            Ok(report) => {
                if report.groups > 0 || report.rules_failed > 0 {
                    info!(
                        firing = report.firing,
                        supprimees = report.suppressed,
                        messages = report.groups,
                        envois = report.notifications_sent,
                        echecs = report.notifications_failed,
                        regles_en_echec = report.rules_failed,
                        "alerting cycle"
                    );
                } else {
                    debug!(alertes = report.alerts_tracked, "alerting cycle");
                }
            }
            // Une erreur ici vient de la base : on la journalise et on retente au
            // cycle suivant plutôt que de tuer la tâche pour de bon.
            Err(error) => warn!(?error, "alerting cycle failed"),
        }

        // L'entretien est horaire : purger à chaque cycle ferait des écritures
        // inutiles toutes les trente secondes pour supprimer zéro ligne.
        if now.signed_duration_since(last_maintenance) >= TimeDelta::hours(1) {
            last_maintenance = now;
            if let Err(error) = maintenance(&state.pool, &config, now).await {
                warn!(?error, "alerting tables maintenance failed");
            }
        }
    }
}

/// Purges périodiques.
async fn maintenance(pool: &SqlitePool, config: &AlertingConfig, now: DateTime<Utc>) -> Result<()> {
    let history_cutoff = now - TimeDelta::from_std(config.history_retention).unwrap_or_default();
    let removed = db::alerts::purge_history(pool, history_cutoff).await?;

    let baseline_cutoff = now - TimeDelta::from_std(config.baseline_retention).unwrap_or_default();
    let forgotten = db::alerts::purge_stale_baselines(pool, baseline_cutoff).await?;

    let expired = db::alerts::purge_expired_silences(pool, now).await?;

    // Le registre des notifications ne sert qu'aux fenêtres glissantes du pipeline
    // (plafond horaire, battement, délai minimal) : au-delà, il ne vaut rien.
    let global = policy_store::load_global(pool).await?;
    let channel_policies = policy_store::load_channel_policies(pool).await?;
    let lookback = policy_store::lookback(&global, &channel_policies);
    let ledger = policy_store::purge(pool, now, lookback).await?;

    debug!(
        historique = removed,
        baselines = forgotten,
        silences = expired,
        registre = ledger,
        "maintenance done"
    );
    Ok(())
}

/// Faits que les alertes « événement » annoncent un par un (voir
/// [`cycle::Occurrence`]). Aujourd'hui une seule source : les vérifications
/// de site qui trouvent des changements, pour la règle livrée
/// [`crate::webchange::ALERT_RULE`]. Une lecture qui échoue n'empêche pas le
/// cycle : l'alerte se comporte alors comme une alerte d'état ordinaire.
async fn occurrences(pool: &SqlitePool) -> HashMap<(String, i64), cycle::Occurrence> {
    let checks = match crate::webchange::store::changed_checks(pool).await {
        Ok(checks) => checks,
        Err(error) => {
            warn!(?error, "website checks not read for alerting");
            return HashMap::new();
        }
    };
    checks
        .into_iter()
        .filter_map(|(target, at, changes, summary)| {
            // `datetime('now')` de SQLite : UTC, sans fuseau.
            let at = chrono::NaiveDateTime::parse_from_str(&at, "%Y-%m-%d %H:%M:%S").ok()?;
            let occurrence =
                cycle::Occurrence { at: at.and_utc(), value: changes as f64, detail: summary };
            Some(((crate::webchange::ALERT_RULE.to_string(), target), occurrence))
        })
        .collect()
}

/// Exécute un cycle complet et renvoie son bilan.
///
/// Testable indépendamment de la boucle : `now` et la source de métriques sont des
/// paramètres, si bien qu'un test peut rejouer une journée entière en quelques
/// millisecondes avec une source factice.
pub async fn evaluate_once(
    pool: &SqlitePool,
    cipher: &Cipher,
    source: &dyn MetricSource,
    http: &reqwest::Client,
    now: DateTime<Utc>,
) -> Result<EvalReport> {
    let rules = db::alerts::list_enabled_rules(pool).await?;
    if rules.is_empty() {
        // Plus aucune règle : ce qui reste dans la table ne sera plus jamais
        // évalué, la purge doit passer quand même.
        db::alerts::purge_states(pool, now).await?;
        return Ok(EvalReport::default());
    }

    let targets = db::alerts::list_target_nodes(pool).await?;
    let previous = db::alerts::load_states(pool).await?;
    let silences = db::alerts::list_silences(pool).await?;
    let overrides = policy_store::list_overrides(pool, None, None).await?;
    let mut baselines = db::alerts::load_baselines(pool, baseline::bucket_of(now)).await?;

    let mut report = EvalReport::default();
    let mut observations = Vec::with_capacity(rules.len());

    // Les requêtes sont séquentielles : un homelab a une poignée de règles, et
    // sérialiser garantit qu'on ne submerge jamais VictoriaMetrics au démarrage,
    // quand toutes les règles sont évaluées pour la première fois.
    for rule in rules {
        let series = match source.instant(&rule.query).await {
            Ok(series) => {
                report.rules_evaluated += 1;
                Some(series)
            }
            Err(error) => {
                report.rules_failed += 1;
                // `None` gèle les alertes de la règle : voir `RuleObservations`.
                warn!(rule = %rule.uid, ?error, "alerting query failed");
                None
            }
        };
        observations.push(RuleObservations { rule, series });
    }

    // La politique globale est chargée avant le cycle : son délai d'escalade
    // participe à la décision de notifier, au même titre que le rappel d'une
    // règle.
    let global = policy_store::load_global(pool).await?;
    let occurrences = occurrences(pool).await;
    let outcome = cycle::plan_cycle(
        CycleInput {
            now,
            observations,
            targets,
            previous,
            silences,
            overrides,
            unacked_after: global.escalation().map(|(after, _)| after.to_std().unwrap_or_default()),
            occurrences,
        },
        &mut baselines,
    );

    report.alerts_tracked = outcome.alerts.len();
    for alert in &outcome.alerts {
        if alert.state.phase.is_active() {
            report.firing += 1;
        }
        if alert.state.suppressed {
            report.suppressed += 1;
        }
        if alert.state.silenced {
            report.silenced += 1;
        }
        if alert.state.learning {
            report.learning += 1;
        }
    }
    report.groups = outcome.groups.len();

    db::alerts::save_states(pool, &outcome.alerts).await?;
    db::alerts::touch_states(pool, &outcome.carried_over, now).await?;
    db::alerts::save_baselines(pool, &baselines, now).await?;

    // Politique de notification : ce qui a le droit de parler passe encore par
    // les filtres de canal, les heures calmes, la fenêtre de regroupement et le
    // plafond horaire avant de partir (voir `notify_policy`).
    let channels = db::alerts::list_channels(pool, cipher).await?;
    let channel_policies = policy_store::load_channel_policies(pool).await?;
    let lookback = policy_store::lookback(&global, &channel_policies);
    let ledger = policy_store::load_ledger(pool, now, lookback).await?;
    let recipients: Vec<Recipient> = channels
        .iter()
        .map(|channel| Recipient {
            id: channel.id,
            kind: channel.kind.clone(),
            enabled: channel.enabled,
            policy: channel_policies.get(&channel.id).cloned().unwrap_or_default(),
        })
        .collect();
    let plan = notify_policy::plan(&outcome.groups, &recipients, &global, &ledger, now);
    policy_store::apply_plan(pool, &plan).await?;

    let env_url = crate::config::env_var("DUMBMONIT_PUBLIC_URL");
    let public_url = global.public_url(env_url.as_deref());
    let window = TimeDelta::seconds(i64::from(global.batch_window_secs));
    // Liens d'action : seulement avec une URL publique, et tant que la
    // politique ne les coupe pas.
    let action_key = (global.action_links && public_url.is_some())
        .then(|| notify::action_link::ActionKey::from_cipher(cipher));
    send_outgoing(
        notify::webpush::Store { pool, cipher },
        http,
        &channels,
        &plan.outgoing,
        public_url.as_deref(),
        action_key.as_ref(),
        window,
        now,
        &mut report,
    )
    .await;

    // L'historique explique pourquoi une transition n'est pas partie : la raison
    // du cycle (suppression, maintenance, apprentissage) prime, sinon celle de la
    // politique (battement, filtre de canal, délai minimal).
    let mut history = outcome.history;
    for entry in &mut history {
        if entry.reason.is_empty()
            && !plan.announced.contains(&entry.fingerprint)
            && let Some(verdict) = plan.verdicts.get(&entry.fingerprint)
        {
            entry.reason = verdict.clone();
        }
    }
    db::alerts::record_history(pool, &history, &plan.announced).await?;
    // Relais d'escalade pris en charge : datés à part, pour qu'ils ne partent
    // qu'une fois par épisode (voir `group::decide`).
    let relayed: Vec<String> = outcome
        .groups
        .iter()
        .flat_map(|group| group.items.iter())
        .filter(|item| {
            item.reason == crate::alerting::group::NotifyReason::Unacked
                && plan.handled.contains(&item.fingerprint)
        })
        .map(|item| item.fingerprint.clone())
        .collect();
    db::alerts::mark_unacked_relayed(pool, &relayed, now).await?;
    // Tout ce que la politique a pris en charge — parti, en attente ou écarté —
    // est marqué : le cycle suivant ne doit pas le représenter.
    let fingerprints: Vec<String> = plan.handled.into_iter().collect();
    db::alerts::mark_notified(pool, &fingerprints, now).await?;

    // La purge vient en dernier : elle s'appuie sur `last_eval_at`, que les écritures
    // précédentes viennent de poser sur toutes les empreintes encore vivantes.
    let purged = db::alerts::purge_states(pool, now).await?;
    if purged > 0 {
        debug!(purged, "stale fingerprints removed");
    }

    Ok(report)
}

/// Envoie les messages décidés par la politique et consigne le résultat.
///
/// Un envoi raté remet ses lignes en file : le cycle suivant retente, exactement
/// le comportement attendu d'une panne passagère du service de notification.
#[allow(clippy::too_many_arguments)]
async fn send_outgoing(
    store: notify::webpush::Store<'_>,
    http: &reqwest::Client,
    channels: &[notify::ChannelConfig],
    outgoing: &[Outgoing],
    public_url: Option<&str>,
    action_key: Option<&notify::action_link::ActionKey>,
    window: TimeDelta,
    now: DateTime<Utc>,
    report: &mut EvalReport,
) {
    let pool = store.pool;
    for out in outgoing {
        let Some(config) = channels.iter().find(|channel| channel.id == out.channel_id) else {
            continue;
        };
        let mut message = notify::render_digest(&out.digest, public_url);
        if let (Some(key), Some(base)) = (action_key, public_url) {
            let alerts = notify::message::actionable_alerts(&out.digest);
            message.actions = notify::action_link::build(key, base, &alerts, now);
        }
        let delivery = notify::deliver_stored(store, http, config, &message).await;

        stats().notification(&config.kind, !delivery.is_success());
        if delivery.is_success() {
            report.notifications_sent += 1;
            policy_store::reset_failures(config.id);
            if let Err(error) =
                policy_store::record_sent(pool, out.channel_id, &out.queue_ids, &out.sent, now)
                    .await
            {
                warn!(?error, "sent notification not recorded");
            }
        } else {
            report.notifications_failed += 1;
            // Une erreur définitive (webhook révoqué, bot supprimé) est consignée par
            // `record_delivery` puis abandonnée : la réémettre à chaque cycle ne
            // ferait que risquer un 429 et noyer le canal une fois réparé.
            if delivery.transient {
                let delay = policy_store::retry_delay(policy_store::note_failure(config.id));
                if let Err(error) = policy_store::requeue(pool, &out.retry, window, delay).await {
                    warn!(?error, "failed notification not requeued");
                }
            } else {
                policy_store::reset_failures(config.id);
                warn!(channel = %config.name, "permanent delivery error, notification dropped");
            }
        }
        if let Err(error) = db::alerts::record_delivery(pool, &delivery, now).await {
            warn!(?error, "delivery report not recorded");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    use async_trait::async_trait;

    use super::*;
    use crate::alerting::model::RULE_HOST_DOWN;
    use crate::alerting::source::SeriesPoint;

    /// Source de métriques déterministe : le cycle complet — SQL compris — se teste
    /// donc sans VictoriaMetrics ni le moindre accès réseau.
    struct FakeSource {
        answers: Vec<(String, f64)>,
    }

    #[async_trait]
    impl MetricSource for FakeSource {
        async fn instant(&self, query: &str) -> Result<Vec<SeriesPoint>> {
            let Some((_, value)) =
                self.answers.iter().find(|(fragment, _)| query.contains(fragment.as_str()))
            else {
                return Ok(Vec::new());
            };
            let labels: BTreeMap<String, String> =
                [("__name__", "dumbmonit_test"), ("target", "1"), ("host", "nas")]
                    .iter()
                    .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                    .collect();
            Ok(vec![SeriesPoint { labels, value: *value, ts_ms: 0 }])
        }
    }

    /// Source qui échoue systématiquement, pour vérifier qu'une panne de la base de
    /// séries ne résout rien et ne fait pas tomber le cycle.
    struct BrokenSource;

    #[async_trait]
    impl MetricSource for BrokenSource {
        async fn instant(&self, _query: &str) -> Result<Vec<SeriesPoint>> {
            anyhow::bail!("VictoriaMetrics unreachable")
        }
    }

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_db() -> PathBuf {
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir()
            .join(format!("dumbmonit-alerting-{}-{unique}/dumbmonit.db", std::process::id()))
    }

    /// Monte une base réelle : migrations appliquées, une cible, les règles livrées.
    async fn fixture() -> (PathBuf, SqlitePool, Cipher) {
        let path = temp_db();
        let pool = db::open(&path).await.expect("open test database");
        let cipher =
            db::init_cipher(&pool, "a-test-secret-that-is-long-enough").await.expect("test cipher");

        sqlx::query(
            "INSERT INTO targets (id, name, address, kind, parent_id, tags)
             VALUES (1, 'nas', '10.0.0.1', 'dummy', NULL, '{\"role\":\"storage\"}')",
        )
        .execute(&pool)
        .await
        .expect("insert test target");

        db::alerts::seed_builtin_rules(&pool).await.expect("built-in rules");
        (path, pool, cipher)
    }

    async fn cleanup(path: &std::path::Path, pool: SqlitePool) {
        pool.close().await;
        if let Some(parent) = path.parent() {
            let _ = std::fs::remove_dir_all(parent);
        }
    }

    fn at(seconds: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_000 + seconds, 0).expect("valid timestamp")
    }

    #[tokio::test]
    async fn les_regles_livrees_sont_installees_une_seule_fois() {
        let (path, pool, _cipher) = fixture().await;

        let rules = db::alerts::list_enabled_rules(&pool).await.expect("read rules");
        assert!(rules.iter().any(|rule| rule.uid == RULE_HOST_DOWN));
        assert!(rules.iter().any(|rule| rule.uid == "disk_almost_full"));

        // Deuxième appel : aucune règle n'est réinsérée ni réécrite.
        let inserted = db::alerts::seed_builtin_rules(&pool).await.expect("second call");
        assert_eq!(inserted, 0);

        cleanup(&path, pool).await;
    }

    /// Une base installée avant la réduction par cible garde l'ancienne requête de
    /// `host_down` ; le démarrage suivant la remplace, sans toucher au seuil que
    /// l'utilisateur a réglé.
    #[tokio::test]
    async fn l_ancienne_requete_injoignable_est_remplacee_au_demarrage() {
        let (path, pool, _cipher) = fixture().await;
        sqlx::query(
            "UPDATE alert_rules SET query = 'time() - tlast_over_time(dumbmonit_up[7d])',
                 threshold = 600 WHERE uid = ?",
        )
        .bind(RULE_HOST_DOWN)
        .execute(&pool)
        .await
        .expect("simulate an older database");

        db::alerts::seed_builtin_rules(&pool).await.expect("seed again");
        let rules = db::alerts::list_rules(&pool).await.expect("read rules");
        let host_down = rules.iter().find(|r| r.uid == RULE_HOST_DOWN).expect("host_down");
        assert_eq!(host_down.query, crate::alerting::rules::HOST_DOWN_QUERY);
        assert_eq!(host_down.threshold, 600.0, "the user's threshold is kept");

        cleanup(&path, pool).await;
    }

    #[tokio::test]
    async fn un_cycle_complet_traverse_la_base_et_fait_avancer_l_etat() {
        let (path, pool, cipher) = fixture().await;
        let http = notify::http_client();

        // La cible est muette depuis dix minutes : la règle « injoignable » doit
        // passer en `pending`, son `for` étant d'une minute.
        let source = FakeSource { answers: vec![("tlast_over_time".to_string(), 600.0)] };

        let report =
            evaluate_once(&pool, &cipher, &source, &http, at(0)).await.expect("first cycle");
        assert!(report.rules_evaluated > 0);
        assert_eq!(report.rules_failed, 0);
        assert_eq!(report.firing, 0, "the one-minute `for` has not elapsed");
        assert_eq!(report.groups, 0);

        // Une minute plus tard, la condition tient : l'alerte part.
        let report =
            evaluate_once(&pool, &cipher, &source, &http, at(60)).await.expect("second cycle");
        assert_eq!(report.firing, 1);
        assert_eq!(report.groups, 1);

        let actives = db::alerts::list_active(&pool).await.expect("active alerts");
        let injoignable = actives
            .iter()
            .find(|alert| alert.rule_uid == RULE_HOST_DOWN)
            .expect("\"unreachable\" alert persisted");
        assert_eq!(injoignable.target_id, Some(1));
        assert_eq!(injoignable.state.notify_count, 1, "recorded despite the absence of a channel");

        cleanup(&path, pool).await;
    }

    /// Monte une alerte « injoignable » en `firing` sur la cible 1, en deux cycles.
    async fn firing_host_down(
        pool: &SqlitePool,
        cipher: &Cipher,
    ) -> crate::alerting::cycle::StoredAlert {
        let http = notify::http_client();
        let source = FakeSource { answers: vec![("tlast_over_time".to_string(), 600.0)] };
        evaluate_once(pool, cipher, &source, &http, at(0)).await.expect("first cycle");
        evaluate_once(pool, cipher, &source, &http, at(60)).await.expect("second cycle");
        db::alerts::list_active(pool)
            .await
            .expect("active alerts")
            .into_iter()
            .find(|alert| alert.rule_uid == RULE_HOST_DOWN)
            .expect("firing alert")
    }

    async fn host_down_rule_id(pool: &SqlitePool) -> i64 {
        db::alerts::list_rules(pool)
            .await
            .expect("rules")
            .into_iter()
            .find(|rule| rule.uid == RULE_HOST_DOWN)
            .expect("built-in rule")
            .id
    }

    /// Ce qu'un cycle parti avant un clic de l'utilisateur s'apprête à écrire.
    fn in_flight(
        stored: &crate::alerting::cycle::StoredAlert,
    ) -> crate::alerting::group::AlertOutcome {
        crate::alerting::group::AlertOutcome {
            fingerprint: stored.fingerprint.clone(),
            rule_uid: stored.rule_uid.clone(),
            rule_name: "unreachable".to_string(),
            target_id: stored.target_id,
            target_name: "nas".to_string(),
            series_key: stored.series_key.clone(),
            labels: stored.labels.clone(),
            state: stored.state.clone(),
            severity: crate::alerting::model::Severity::Critical,
            value: Some(600.0),
            score: None,
            unit: String::new(),
            operator: ">".to_string(),
            threshold: 0.0,
            channels: Vec::new(),
            repeat_interval: None,
            escalate_after: None,
            unacked_after: None,
            just_transitioned: false,
            recurred: false,
            detail: None,
        }
    }

    #[tokio::test]
    async fn desactiver_une_regle_solde_ses_alertes_avec_une_trace() {
        let (path, pool, cipher) = fixture().await;
        let alert = firing_host_down(&pool, &cipher).await;

        let id = host_down_rule_id(&pool).await;
        assert!(db::alerts::set_rule_enabled(&pool, id, false).await.expect("disable"));

        let states = db::alerts::load_states(&pool).await.expect("states");
        assert!(
            states.iter().all(|s| s.rule_uid != RULE_HOST_DOWN),
            "a disabled rule must not leave alerts behind"
        );
        let (to_phase, reason): (String, String) = sqlx::query_as(
            "SELECT to_phase, reason FROM alert_history WHERE fingerprint = ?
             ORDER BY id DESC LIMIT 1",
        )
        .bind(&alert.fingerprint)
        .fetch_one(&pool)
        .await
        .expect("history entry");
        assert_eq!(to_phase, "resolved");
        assert_eq!(reason, crate::alerting::cycle::DISABLED_RULE_REASON);

        cleanup(&path, pool).await;
    }

    #[tokio::test]
    async fn sans_regle_active_les_empreintes_restantes_sont_purgees() {
        let (path, pool, cipher) = fixture().await;
        firing_host_down(&pool, &cipher).await;
        // Toutes les règles coupées d'un coup, sans passer par l'API.
        sqlx::query("UPDATE alert_rules SET enabled = 0").execute(&pool).await.expect("disable");

        let http = notify::http_client();
        let source = FakeSource { answers: Vec::new() };
        evaluate_once(&pool, &cipher, &source, &http, at(120)).await.expect("empty cycle");
        assert!(db::alerts::load_states(&pool).await.expect("states").is_empty());

        cleanup(&path, pool).await;
    }

    #[tokio::test]
    async fn un_cycle_en_vol_ne_ressuscite_pas_les_alertes_d_une_cible_oubliee() {
        let (path, pool, cipher) = fixture().await;
        let alert = firing_host_down(&pool, &cipher).await;

        // L'utilisateur supprime l'équipement pendant que le cycle calcule.
        sqlx::query("UPDATE targets SET enabled = 0 WHERE id = 1")
            .execute(&pool)
            .await
            .expect("disable target");
        db::alerts::forget_target(&pool, 1, at(90)).await.expect("forget");
        db::alerts::save_states(&pool, &[in_flight(&alert)]).await.expect("late save");

        assert!(
            db::alerts::load_states(&pool).await.expect("states").is_empty(),
            "the late write must not bring the alert back"
        );

        cleanup(&path, pool).await;
    }

    #[tokio::test]
    async fn un_cycle_en_vol_ne_ressuscite_pas_les_alertes_d_une_regle_desactivee() {
        let (path, pool, cipher) = fixture().await;
        let alert = firing_host_down(&pool, &cipher).await;

        let id = host_down_rule_id(&pool).await;
        db::alerts::set_rule_enabled(&pool, id, false).await.expect("disable");
        db::alerts::save_states(&pool, &[in_flight(&alert)]).await.expect("late save");

        let states = db::alerts::load_states(&pool).await.expect("states");
        assert!(states.iter().all(|s| s.rule_uid != RULE_HOST_DOWN));

        cleanup(&path, pool).await;
    }

    #[tokio::test]
    async fn supprimer_une_regle_efface_ses_surcharges_par_equipement() {
        let (path, pool, _cipher) = fixture().await;
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO alert_rules (uid, name, kind, query, operator, threshold, severity)
             VALUES ('disk_full', 'Disk full', 'threshold', 'q', '>', 90, 'warning')
             RETURNING id",
        )
        .fetch_one(&pool)
        .await
        .expect("user rule");
        sqlx::query(
            "INSERT INTO rule_overrides (rule_uid, target_id, threshold) VALUES (?, 1, 99)",
        )
        .bind("disk_full")
        .execute(&pool)
        .await
        .expect("override");

        assert!(db::alerts::delete_rule(&pool, id).await.expect("delete"));
        let left: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM rule_overrides WHERE rule_uid = 'disk_full'")
                .fetch_one(&pool)
                .await
                .expect("count");
        assert_eq!(left, 0, "a rule recreated under the same uid must start clean");

        cleanup(&path, pool).await;
    }

    #[tokio::test]
    async fn le_relais_d_escalade_est_date_et_survit_aux_cycles() {
        let (path, pool, cipher) = fixture().await;
        let http = notify::http_client();
        let source = FakeSource { answers: vec![("tlast_over_time".to_string(), 600.0)] };

        evaluate_once(&pool, &cipher, &source, &http, at(0)).await.expect("first cycle");
        evaluate_once(&pool, &cipher, &source, &http, at(60)).await.expect("second cycle");
        let fingerprint = db::alerts::list_active(&pool)
            .await
            .expect("active alerts")
            .into_iter()
            .find(|alert| alert.rule_uid == RULE_HOST_DOWN)
            .expect("firing alert")
            .fingerprint;

        db::alerts::mark_unacked_relayed(&pool, std::slice::from_ref(&fingerprint), at(90))
            .await
            .expect("relay recorded");
        // Le cycle suivant réécrit la ligne : le relais doit y rester.
        evaluate_once(&pool, &cipher, &source, &http, at(120)).await.expect("third cycle");
        let stored = db::alerts::load_states(&pool)
            .await
            .expect("states")
            .into_iter()
            .find(|alert| alert.fingerprint == fingerprint)
            .expect("still stored");
        assert_eq!(stored.state.unacked_relayed_at, Some(at(90)));

        cleanup(&path, pool).await;
    }

    #[tokio::test]
    async fn une_panne_de_la_base_de_series_ne_resout_aucune_alerte() {
        let (path, pool, cipher) = fixture().await;
        let http = notify::http_client();
        let source = FakeSource { answers: vec![("tlast_over_time".to_string(), 600.0)] };

        evaluate_once(&pool, &cipher, &source, &http, at(0)).await.expect("initial cycle");
        evaluate_once(&pool, &cipher, &source, &http, at(60)).await.expect("trigger");
        assert_eq!(db::alerts::list_active(&pool).await.unwrap().len(), 1);

        // VictoriaMetrics tombe : l'alerte est gelée, ni résolue ni purgée.
        let report = evaluate_once(&pool, &cipher, &BrokenSource, &http, at(120))
            .await
            .expect("the cycle does not propagate query failures");
        assert!(report.rules_failed > 0);
        assert_eq!(report.groups, 0, "no misleading \"all clear\"");

        let actives = db::alerts::list_active(&pool).await.expect("active alerts");
        assert_eq!(actives.len(), 1, "the alert survives the outage");
        assert_eq!(actives[0].state.phase, crate::alerting::Phase::Firing);

        cleanup(&path, pool).await;
    }

    #[tokio::test]
    async fn les_secrets_d_un_canal_font_l_aller_retour_chiffres() {
        let (path, pool, cipher) = fixture().await;

        let id = db::alerts::upsert_channel(
            &pool,
            &cipher,
            &db::alerts::ChannelDraft {
                id: None,
                name: "home ntfy".to_string(),
                kind: "ntfy".to_string(),
                enabled: true,
                settings: serde_json::json!({"topic": "homelab"}),
                secrets: Some(serde_json::json!({"token": "tk_secret_very_long"})),
            },
        )
        .await
        .expect("create channel");

        let channels = db::alerts::list_channels(&pool, &cipher).await.expect("read");
        let channel = channels.iter().find(|c| c.id == id).expect("channel present");
        assert_eq!(
            channel.secret("token").expect("readable secret").expose(),
            "tk_secret_very_long"
        );

        // Le secret n'est pas stocké en clair : la colonne chiffrée ne le contient pas.
        let row = sqlx::query("SELECT secret_enc FROM notification_channels WHERE id = ?")
            .bind(id)
            .fetch_one(&pool)
            .await
            .expect("raw read");
        let raw: Vec<u8> = sqlx::Row::try_get(&row, "secret_enc").expect("encrypted column");
        assert!(
            !String::from_utf8_lossy(&raw).contains("tk_secret_very_long"),
            "the token must never reach the disk in clear text"
        );

        // Une mise à jour sans secret conserve celui déjà enregistré.
        db::alerts::upsert_channel(
            &pool,
            &cipher,
            &db::alerts::ChannelDraft {
                id: Some(id),
                name: "ntfy renamed".to_string(),
                kind: "ntfy".to_string(),
                enabled: true,
                settings: serde_json::json!({"topic": "homelab"}),
                secrets: None,
            },
        )
        .await
        .expect("rename");
        let channels = db::alerts::list_channels(&pool, &cipher).await.expect("re-read");
        let channel = channels.iter().find(|c| c.id == id).expect("channel present");
        assert_eq!(channel.name, "ntfy renamed");
        assert_eq!(channel.secret("token").expect("secret kept").expose(), "tk_secret_very_long");

        cleanup(&path, pool).await;
    }

    #[tokio::test]
    async fn les_baselines_sont_persistees_et_relues() {
        let (path, pool, cipher) = fixture().await;
        let http = notify::http_client();

        // Une règle d'anomalie sur une requête que la source factice reconnaît.
        let mut rule = crate::alerting::rules::builtin_rules()
            .into_iter()
            .find(|rule| rule.uid == "cpu_anomaly")
            .expect("built-in anomaly rule");
        rule.query = "anomaly_test".to_string();
        db::alerts::upsert_rule(&pool, &rule).await.expect("update query");

        let source = FakeSource { answers: vec![("anomaly_test".to_string(), 50.0)] };
        evaluate_once(&pool, &cipher, &source, &http, at(0)).await.expect("cycle");

        let store = db::alerts::load_baselines(&pool, baseline::bucket_of(at(0)))
            .await
            .expect("re-read baselines");
        let key = "dumbmonit_test{host=\"nas\",target=\"1\"}";
        let bucket = store.bucket(key, baseline::bucket_of(at(0))).expect("bucket persisted");
        assert_eq!(bucket.samples, 1);
        assert_eq!(bucket.ewma, 50.0);
        assert!(store.series(key).is_some(), "the series is dated for learning");

        cleanup(&path, pool).await;
    }

    #[test]
    fn la_periode_d_evaluation_ne_descend_pas_sous_dix_secondes() {
        // SAFETY : les tests d'un même binaire partagent l'environnement du processus ;
        // ce test ne lit qu'une variable qu'aucun autre n'utilise.
        unsafe {
            std::env::set_var("DUMBMONIT_ALERT_INTERVAL_SECS", "1");
        }
        assert_eq!(AlertingConfig::from_env().interval, Duration::from_secs(10));

        unsafe {
            std::env::set_var("DUMBMONIT_ALERT_INTERVAL_SECS", "120");
        }
        assert_eq!(AlertingConfig::from_env().interval, Duration::from_secs(120));

        unsafe {
            std::env::remove_var("DUMBMONIT_ALERT_INTERVAL_SECS");
        }
        assert_eq!(AlertingConfig::from_env().interval, Duration::from_secs(30));
    }

    #[test]
    fn une_valeur_d_environnement_absurde_retombe_sur_le_defaut() {
        unsafe {
            std::env::set_var("DUMBMONIT_ALERT_HISTORY_DAYS", "zero");
        }
        let config = AlertingConfig::from_env();
        assert_eq!(config.history_retention, AlertingConfig::default().history_retention);
        unsafe {
            std::env::remove_var("DUMBMONIT_ALERT_HISTORY_DAYS");
        }
    }
}
