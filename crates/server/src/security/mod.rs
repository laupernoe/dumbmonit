//! Note de sécurité par équipement, à la manière de PingCastle ou d'un
//! « Secure Score » : chaque type d'équipement apporte des **contrôles**
//! tirés des bonnes pratiques de son éditeur, chaque contrôle réussit, échoue
//! ou reste inconnu, et la somme pondérée donne une note de 0 à 100 et une
//! lettre de A à F.
//!
//! # Facultative
//!
//! La note est un réglage d'instance (`app_settings`, clé [`SETTING_KEY`]),
//! coupé par défaut : sans lui, ni calcul périodique, ni série publiée, et
//! l'API répond `enabled: false` sans interroger VictoriaMetrics. Une
//! instance antérieure qui avait déjà des équipements notés l'a retrouvé
//! allumé à la migration `0055` : rien ne disparaît sans qu'on l'ait demandé.
//!
//! # Principe
//!
//! Rien n'est redemandé à l'équipement : les contrôles ne lisent que ce que
//! les collecteurs ont **déjà** enregistré dans VictoriaMetrics (dernière
//! valeur de chaque série sur [`LOOKBACK`]). Un contrôle dont la donnée
//! manque est `unknown` : il est montré, jamais compté — une note ne doit pas
//! baisser parce qu'un jeton de supervision n'a pas le droit de lire la liste
//! des mises à jour, ni monter parce qu'on ne sait rien. C'est une
//! indication, pas un audit : la documentation le dit, l'interface aussi.
//!
//! # Couverture
//!
//! Volontairement restreinte aux produits où quelques contrôles disent
//! quelque chose d'utile : machines sous agent (Linux, Windows), Proxmox VE
//! et Backup Server, Synology, OPNsense, sites web (TLS et en-têtes) et
//! Active Directory. Les autres types n'ont pas de note du tout.
//!
//! # Ajouter un type d'équipement
//!
//! 1. Écrire un [`SecurityProvider`] : les `kinds` qu'il couvre (tels que
//!    `Target::kind`), les noms de métriques qu'il lit (sans le préfixe
//!    `dumbmonit_`), la fonction pure `evaluate(kind, &Facts) -> Vec<Check>`
//!    et le [`Guide`] qui dit comment améliorer la note.
//! 2. Le déclarer dans [`providers`], et le documenter dans
//!    `docs/using/security-score.md` sous l'ancre du guide.
//!
//! Le reste — l'API, l'interface, la série `dumbmonit_security_score`, les
//! règles d'alerte livrées — ne connaît aucun type concret. Les constats
//! Active Directory (voir [`checks::activedirectory`]) passent par le même
//! chemin : une métrique conventionnelle par constat, convertie en contrôle.

pub mod checks;
mod facts;
mod score;

pub use facts::{Fact, Facts};
pub use score::{Grade, ScoreOutcome, grade_for, score_checks};

use std::collections::BTreeMap;
use std::time::Duration;

use dumbmonit_proto::{MetricKind, Sample, Target, TargetId};
use serde::Serialize;
use sqlx::SqlitePool;
use tracing::{debug, warn};

use crate::state::AppState;
use crate::tsdb::InstantSeries;

/// Fenêtre de lecture des dernières valeurs. Large : certaines données
/// (paquets en attente, abonnement) ne sont relues par les collecteurs que
/// toutes les quelques heures.
pub const LOOKBACK: &str = "6h";

/// Période de calcul de la série `dumbmonit_security_score`.
pub const SCORE_INTERVAL: Duration = Duration::from_secs(15 * 60);

/// Clé du réglage dans `app_settings` : `on` / `off`, absente = coupée.
pub const SETTING_KEY: &str = "security_score";

/// Page de documentation ; chaque [`Guide`] y ajoute son ancre.
pub const DOCS_URL: &str = "https://dumbmonit.readthedocs.io/en/latest/using/security-score/";

/// La note est-elle activée sur cette instance ?
pub async fn setting_enabled(pool: &SqlitePool) -> sqlx::Result<bool> {
    let value: Option<String> = sqlx::query_scalar("SELECT value FROM app_settings WHERE key = ?")
        .bind(SETTING_KEY)
        .fetch_optional(pool)
        .await?;
    Ok(value.as_deref() == Some("on"))
}

pub async fn set_setting_enabled(pool: &SqlitePool, enabled: bool) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO app_settings (key, value) VALUES (?, ?) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(SETTING_KEY)
    .bind(if enabled { "on" } else { "off" })
    .execute(pool)
    .await?;
    Ok(())
}

/// Familles de contrôles, celles d'un rapport PingCastle ou d'un CIS
/// Benchmark ramenées à l'échelle d'un homelab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    /// Ce qui est joignable de l'extérieur, ou ne filtre plus.
    Exposure,
    /// Mises à jour, micrologiciels, redémarrages en attente.
    Patching,
    /// Comptes, second facteur, délégations.
    Authentication,
    /// Certificats et protocoles.
    Encryption,
    /// Sauvegardes : existence, réussite, vérification.
    Backup,
    /// Réglages recommandés par l'éditeur.
    Configuration,
}

/// Gravité d'un contrôle ; elle fixe son poids dans la note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    pub fn weight(self) -> u32 {
        match self {
            Self::Low => 1,
            Self::Medium => 3,
            Self::High => 6,
            Self::Critical => 10,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Pass,
    Fail,
    /// La donnée n'est pas collectée (ou pas encore) : montré, pas compté.
    Unknown,
}

/// La définition d'un contrôle, indépendante de tout équipement.
#[derive(Debug)]
pub struct CheckDef {
    pub id: &'static str,
    pub title: &'static str,
    pub category: Category,
    pub severity: Severity,
    /// Page de l'éditeur (ou du référentiel) qui fonde la recommandation.
    pub reference: &'static str,
    pub remediation: &'static str,
}

impl CheckDef {
    fn with(&self, result: Outcome, evidence: impl Into<String>) -> Check {
        Check {
            id: self.id.to_string(),
            title: self.title.to_string(),
            category: self.category,
            severity: self.severity,
            weight: self.severity.weight(),
            result,
            evidence: evidence.into(),
            remediation: self.remediation.to_string(),
            reference: self.reference.to_string(),
        }
    }

    pub fn pass(&self, evidence: impl Into<String>) -> Check {
        self.with(Outcome::Pass, evidence)
    }

    pub fn fail(&self, evidence: impl Into<String>) -> Check {
        self.with(Outcome::Fail, evidence)
    }

    pub fn unknown(&self, evidence: impl Into<String>) -> Check {
        self.with(Outcome::Unknown, evidence)
    }

    /// Réussi si `ok`, échoué sinon, avec la même preuve.
    pub fn verdict(&self, ok: bool, evidence: impl Into<String>) -> Check {
        if ok { self.pass(evidence) } else { self.fail(evidence) }
    }
}

/// Un contrôle évalué, tel que l'API le sert.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Check {
    pub id: String,
    pub title: String,
    pub category: Category,
    pub severity: Severity,
    pub weight: u32,
    pub result: Outcome,
    pub evidence: String,
    pub remediation: String,
    pub reference: String,
}

/// Comment améliorer la note d'un type d'équipement : quelques étapes
/// concrètes, dans l'ordre où les faire, et l'ancre de sa section dans la
/// documentation. Les remédiations des contrôles disent quoi corriger ; le
/// guide dit par où commencer sur ce produit-là.
#[derive(Debug, Clone, Copy)]
pub struct Guide {
    /// Nom du produit tel que l'utilisateur le connaît.
    pub product: &'static str,
    pub steps: &'static [&'static str],
    /// Ancre dans [`DOCS_URL`], sans le `#`.
    pub anchor: &'static str,
}

/// Le guide tel que l'API le sert.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GuideView {
    pub product: String,
    pub steps: Vec<String>,
    pub doc_url: String,
}

impl From<Guide> for GuideView {
    fn from(guide: Guide) -> Self {
        Self {
            product: guide.product.to_string(),
            steps: guide.steps.iter().map(|s| (*s).to_string()).collect(),
            doc_url: format!("{DOCS_URL}#{}", guide.anchor),
        }
    }
}

/// Un type d'équipement qui sait se noter.
pub trait SecurityProvider: Send + Sync {
    /// Les `Target::kind` couverts.
    fn kinds(&self) -> &'static [&'static str];
    /// Les métriques lues, sans le préfixe `dumbmonit_`.
    fn metrics(&self) -> &'static [&'static str];
    /// Les contrôles, à partir des dernières valeurs connues. Pure : tout le
    /// test d'un contrôle se fait sur des `Facts` construits à la main.
    fn evaluate(&self, kind: &str, facts: &Facts) -> Vec<Check>;
    /// Les étapes d'amélioration ; les faits permettent de distinguer deux
    /// variantes d'un même type (agent Linux ou Windows).
    fn guide(&self, kind: &str, facts: &Facts) -> Guide;
    /// Ce type d'équipement est-il concerné, vu ses faits ? Par défaut oui.
    /// Un moniteur HTTP en clair n'a pas de certificat à noter : il n'a
    /// alors pas de carte sécurité du tout, plutôt qu'une note « inconnue ».
    fn applies(&self, _facts: &Facts) -> bool {
        true
    }
}

/// Tous les fournisseurs connus.
pub fn providers() -> &'static [&'static dyn SecurityProvider] {
    &[
        &checks::proxmox::Proxmox,
        &checks::pbs::Pbs,
        &checks::synology::Synology,
        &checks::firewall::Opnsense,
        &checks::tls::Website,
        &checks::agent::Agent,
        &checks::agent::AgentDocker,
        &checks::activedirectory::ActiveDirectory,
    ]
}

pub fn provider_for(kind: &str) -> Option<&'static dyn SecurityProvider> {
    providers().iter().copied().find(|provider| provider.kinds().contains(&kind))
}

/// Les types d'équipement notés, pour la documentation et l'interface.
pub fn supported_kinds() -> Vec<&'static str> {
    providers().iter().flat_map(|provider| provider.kinds().iter().copied()).collect()
}

/// Rapport complet d'un équipement.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    /// `false` : la note est coupée dans les réglages ; tout le reste est vide.
    pub enabled: bool,
    pub target_id: TargetId,
    pub target_name: String,
    pub kind: String,
    /// `false` : aucun contrôle n'existe pour ce type d'équipement.
    pub supported: bool,
    pub score: Option<u32>,
    pub grade: Option<Grade>,
    /// La lettre a été plafonnée par un contrôle critique en échec.
    pub capped: bool,
    pub evaluated_at: String,
    pub counts: Counts,
    pub checks: Vec<Check>,
    /// Comment améliorer la note de ce type d'équipement.
    pub guide: Option<GuideView>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Counts {
    pub pass: usize,
    pub fail: usize,
    pub unknown: usize,
}

/// Note un équipement à partir de ses faits. Les contrôles sont triés :
/// échecs d'abord (les plus lourds en tête), puis réussites, puis inconnus.
pub fn build_report(target_id: TargetId, name: &str, kind: &str, facts: &Facts) -> Report {
    let evaluated_at = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let Some(provider) = provider_for(kind).filter(|provider| provider.applies(facts)) else {
        return Report::empty(target_id, name, kind, true, evaluated_at);
    };
    let mut checks = provider.evaluate(kind, facts);
    sort_checks(&mut checks);
    let outcome = score_checks(&checks);
    let counts = Counts {
        pass: checks.iter().filter(|c| c.result == Outcome::Pass).count(),
        fail: checks.iter().filter(|c| c.result == Outcome::Fail).count(),
        unknown: checks.iter().filter(|c| c.result == Outcome::Unknown).count(),
    };
    Report {
        enabled: true,
        target_id,
        target_name: name.to_string(),
        kind: kind.to_string(),
        supported: true,
        score: outcome.score,
        grade: outcome.grade,
        capped: outcome.capped,
        evaluated_at,
        counts,
        checks,
        guide: Some(provider.guide(kind, facts).into()),
    }
}

impl Report {
    /// Rapport sans contrôle : type non noté, ou note coupée (`enabled`).
    pub fn empty(
        target_id: TargetId,
        name: &str,
        kind: &str,
        enabled: bool,
        evaluated_at: String,
    ) -> Self {
        Self {
            enabled,
            target_id,
            target_name: name.to_string(),
            kind: kind.to_string(),
            supported: false,
            score: None,
            grade: None,
            capped: false,
            evaluated_at,
            counts: Counts::default(),
            checks: Vec::new(),
            guide: None,
        }
    }
}

pub fn sort_checks(checks: &mut [Check]) {
    let rank = |outcome: Outcome| match outcome {
        Outcome::Fail => 0,
        Outcome::Pass => 1,
        Outcome::Unknown => 2,
    };
    checks.sort_by(|a, b| {
        rank(a.result)
            .cmp(&rank(b.result))
            .then(b.weight.cmp(&a.weight))
            .then_with(|| a.id.cmp(&b.id))
    });
}

// ------------------------------------------------------------- lecture

/// Sélecteur MetricsQL des métriques lues par un ensemble de fournisseurs.
fn name_pattern<'a>(names: impl Iterator<Item = &'a str>) -> String {
    let mut names: Vec<&str> = names.collect();
    names.sort_unstable();
    names.dedup();
    format!("dumbmonit_({})", names.join("|"))
}

fn instant_query(pattern: &str, target: Option<TargetId>) -> String {
    let target = target.map(|id| format!(r#", target="{id}""#)).unwrap_or_default();
    format!(r#"last_over_time({{__name__=~"{pattern}"{target}}}[{LOOKBACK}]) keep_metric_names"#)
}

/// Faits d'un seul équipement.
pub async fn facts_for(state: &AppState, target: &Target) -> anyhow::Result<Facts> {
    let Some(provider) = provider_for(&target.kind) else { return Ok(Facts::default()) };
    let pattern = name_pattern(provider.metrics().iter().copied());
    let series = state.victoria.query(&instant_query(&pattern, Some(target.id))).await?;
    Ok(Facts::from_series(&series))
}

/// Faits de tous les équipements, rangés par identifiant de cible : une seule
/// requête pour tout le parc.
pub async fn facts_by_target(state: &AppState) -> anyhow::Result<BTreeMap<TargetId, Facts>> {
    let pattern = name_pattern(providers().iter().flat_map(|p| p.metrics().iter().copied()));
    let series = state.victoria.query(&instant_query(&pattern, None)).await?;
    Ok(group_by_target(&series))
}

fn group_by_target(series: &[InstantSeries]) -> BTreeMap<TargetId, Facts> {
    let mut grouped: BTreeMap<TargetId, Vec<&InstantSeries>> = BTreeMap::new();
    for one in series {
        let Some(id) = one.metric.get("target").and_then(|raw| raw.parse().ok()) else {
            continue;
        };
        grouped.entry(id).or_default().push(one);
    }
    grouped.into_iter().map(|(id, list)| (id, Facts::from_series_refs(&list))).collect()
}

/// Rapports de tous les équipements notés et actifs.
pub async fn all_reports(state: &AppState) -> anyhow::Result<Vec<Report>> {
    let targets = crate::db::targets::list(&state.pool, &state.cipher).await?;
    let rated: Vec<&Target> =
        targets.iter().filter(|t| t.enabled && provider_for(&t.kind).is_some()).collect();
    if rated.is_empty() {
        return Ok(Vec::new());
    }
    let mut facts = facts_by_target(state).await?;
    Ok(rated
        .into_iter()
        .map(|target| {
            let facts = facts.remove(&target.id).unwrap_or_default();
            build_report(target.id, &target.name, &target.kind, &facts)
        })
        .filter(|report| report.supported)
        .collect())
}

/// Séries publiées pour un rapport : la note (absente quand rien n'est
/// noté, pour ne pas inventer de zéro) et le nombre d'échecs.
pub fn report_samples(report: &Report, ts_ms: i64) -> Vec<Sample> {
    let target = report.target_id.to_string();
    let mut out = Vec::new();
    if let Some(score) = report.score {
        out.push(
            Sample::new("security_score", f64::from(score), MetricKind::Gauge, ts_ms)
                .with_label("target", target.clone()),
        );
    }
    if report.supported {
        out.push(
            Sample::new(
                "security_checks_failed",
                report.counts.fail as f64,
                MetricKind::Gauge,
                ts_ms,
            )
            .with_label("target", target),
        );
    }
    out
}

/// Calcule la note de tout le parc toutes les [`SCORE_INTERVAL`] et la
/// publie : c'est ce qui donne un historique, et de quoi alerter. Note
/// coupée : le tour passe sans rien lire ni publier, les séries s'éteignent.
pub fn spawn(state: AppState) {
    tokio::spawn(async move {
        // Laisse aux collecteurs le temps d'une première passe.
        tokio::time::sleep(Duration::from_secs(120)).await;
        let mut ticker = tokio::time::interval(SCORE_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            match setting_enabled(&state.pool).await {
                Ok(true) => {}
                Ok(false) => continue,
                Err(error) => {
                    warn!(?error, "security score setting not read");
                    continue;
                }
            }
            match all_reports(&state).await {
                Ok(reports) => {
                    let ts_ms = chrono::Utc::now().timestamp_millis();
                    let samples: Vec<Sample> =
                        reports.iter().flat_map(|r| report_samples(r, ts_ms)).collect();
                    debug!(devices = reports.len(), "security scores computed");
                    if !samples.is_empty() {
                        state.sink.send(samples).await;
                    }
                }
                Err(error) => warn!(?error, "security scores not computed"),
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chaque_controle_a_un_identifiant_unique_et_une_reference_https() {
        let mut ids = std::collections::HashSet::new();
        for def in checks::all_defs() {
            assert!(ids.insert(def.id), "identifiant en double : {}", def.id);
            assert!(
                def.reference.starts_with("https://"),
                "{} : référence {}",
                def.id,
                def.reference
            );
            assert!(!def.remediation.is_empty(), "{} : remédiation vide", def.id);
            assert!(!def.title.is_empty());
        }
    }

    #[test]
    fn chaque_type_a_un_guide_avec_des_etapes_et_une_ancre() {
        for provider in providers() {
            for kind in provider.kinds() {
                let guide = provider.guide(kind, &Facts::default());
                assert!(!guide.steps.is_empty(), "{kind} : guide vide");
                assert!(!guide.product.is_empty());
                assert!(
                    !guide.anchor.is_empty()
                        && guide.anchor.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
                    "{kind} : ancre {}",
                    guide.anchor
                );
            }
        }
    }

    #[test]
    fn la_couverture_se_limite_aux_produits_principaux() {
        let mut kinds = supported_kinds();
        kinds.sort_unstable();
        assert_eq!(
            kinds,
            [
                "activedirectory",
                "agent",
                "agent_service",
                "http",
                "opnsense",
                "pbs",
                "proxmox",
                "synology",
                "tls",
                "website"
            ]
        );
    }

    #[test]
    fn chaque_type_n_a_qu_un_fournisseur() {
        let mut kinds = supported_kinds();
        let total = kinds.len();
        kinds.sort_unstable();
        kinds.dedup();
        assert_eq!(kinds.len(), total, "un type est noté par deux fournisseurs");
    }

    #[test]
    fn des_faits_vides_ne_donnent_que_des_inconnus() {
        // Le principe : sans donnée, rien n'est compté, ni en bien ni en mal.
        for provider in providers() {
            // Un fournisseur qui ne s'applique qu'en présence de certaines
            // séries (l'appareil Docker) n'a rien à noter sans elles.
            if !provider.applies(&Facts::default()) {
                continue;
            }
            for kind in provider.kinds() {
                let checks = provider.evaluate(kind, &Facts::default());
                assert!(!checks.is_empty(), "{kind} : aucun contrôle");
                assert!(checks.iter().all(|c| c.result == Outcome::Unknown), "{kind} : {checks:?}");
            }
        }
    }

    #[test]
    fn le_rapport_trie_les_echecs_en_tete_et_les_inconnus_en_queue() {
        let facts = Facts::from_pairs(&[
            ("agent_updates_pending", &[], 4.0),
            ("agent_security_updates_pending", &[], 0.0),
            ("agent_reboot_required", &[], 1.0),
        ]);
        let report = build_report(7, "web", "agent", &facts);
        assert!(report.supported && report.enabled);
        let guide = report.guide.as_ref().unwrap();
        assert!(guide.doc_url.starts_with(DOCS_URL) && guide.doc_url.contains('#'));
        let results: Vec<Outcome> = report.checks.iter().map(|c| c.result).collect();
        let first_pass = results.iter().position(|r| *r == Outcome::Pass).unwrap();
        let first_unknown = results.iter().position(|r| *r == Outcome::Unknown).unwrap();
        assert!(results[..first_pass].iter().all(|r| *r == Outcome::Fail));
        assert!(first_unknown > first_pass);
        // Le plus lourd des échecs d'abord : redémarrage (moyen) avant
        // mises à jour ordinaires (faible).
        assert!(report.checks[0].weight >= report.checks[1].weight);
        assert_eq!(report.counts.fail, 2);
        assert_eq!(report.counts.pass, 1);
    }

    #[test]
    fn un_type_inconnu_n_est_pas_note() {
        let report = build_report(1, "x", "ping", &Facts::default());
        assert!(!report.supported);
        assert!(report.guide.is_none());
        assert!(report.score.is_none() && report.grade.is_none());
        assert!(report_samples(&report, 0).is_empty());
    }

    #[test]
    fn la_note_est_publiee_par_cible() {
        let facts = Facts::from_pairs(&[("agent_security_updates_pending", &[], 0.0)]);
        let report = build_report(42, "web", "agent", &facts);
        let samples = report_samples(&report, 1000);
        let score = samples.iter().find(|s| s.metric == "security_score").unwrap();
        assert_eq!(score.labels["target"], "42");
        assert_eq!(score.value, 100.0);
        assert!(samples.iter().any(|s| s.metric == "security_checks_failed" && s.value == 0.0));
    }

    #[test]
    fn la_requete_lit_toutes_les_metriques_d_une_cible() {
        let query = instant_query(&name_pattern(["b", "a", "b"].into_iter()), Some(3));
        assert_eq!(
            query,
            r#"last_over_time({__name__=~"dumbmonit_(a|b)", target="3"}[6h]) keep_metric_names"#
        );
    }

    #[test]
    fn les_series_sont_rangees_par_cible() {
        let series: Vec<InstantSeries> = serde_json::from_value(serde_json::json!([
            { "metric": { "__name__": "dumbmonit_agent_reboot_required", "target": "1" }, "value": [1.0, "1"] },
            { "metric": { "__name__": "dumbmonit_agent_updates_pending", "target": "2" }, "value": [1.0, "3"] },
            { "metric": { "__name__": "dumbmonit_agent_updates_pending" }, "value": [1.0, "3"] }
        ]))
        .unwrap();
        let grouped = group_by_target(&series);
        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped[&1].max("agent_reboot_required"), Some(1.0));
        assert_eq!(grouped[&2].max("agent_updates_pending"), Some(3.0));
    }
}
