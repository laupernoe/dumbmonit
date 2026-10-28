//! Types du domaine de l'alerting : règles, sévérités, phases, empreintes.
//!
//! Ce module est volontairement dépourvu d'entrées/sorties : il est partagé par le
//! moteur, la couche base de données et l'API, et reste testable sans réseau.

use std::collections::BTreeMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Identifiant d'une cible, repris de la table `targets`.
pub type TargetId = i64;

/// `uid` de la règle « équipement injoignable ».
///
/// Le moteur a besoin de la reconnaître nominativement : c'est elle qui définit
/// l'ensemble des cibles considérées comme hors ligne, donc la racine de la
/// suppression par dépendance.
pub const RULE_HOST_DOWN: &str = "host_down";

/// `uid` de la règle « service en panne » : une sonde de disponibilité (ping,
/// HTTP, TCP, DNS…) qui tourne mais échoue (`probe_success = 0`).
///
/// La cible écrit encore des points, `host_down` ne part donc jamais pour elle ;
/// elle n'en est pas moins indisponible, et ses descendants avec elle.
pub const RULE_SERVICE_DOWN: &str = "service_down";

/// Étiquettes candidates pour rattacher une série à une cible, par ordre de
/// confiance décroissante.
///
/// Le collecteur pose les étiquettes d'identité via `Target::base_labels()`, dont
/// la composition exacte relève d'un autre jalon : accepter plusieurs conventions
/// évite de coupler le moteur d'alerting à ce détail.
pub const TARGET_ID_LABELS: [&str; 2] = ["target", "target_id"];
pub const TARGET_NAME_LABELS: [&str; 2] = ["host", "hostname"];
pub const TARGET_ADDRESS_LABELS: [&str; 2] = ["instance", "address"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    #[default]
    Warning,
    Critical,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Critical => "critical",
        }
    }

    /// Sévérité immédiatement supérieure, utilisée par l'escalade. `Critical` est un
    /// point fixe : au-delà, il n'y a plus rien à dire de plus fort.
    pub fn escalated(self) -> Self {
        match self {
            Self::Info => Self::Warning,
            Self::Warning | Self::Critical => Self::Critical,
        }
    }

    pub fn parse(raw: &str) -> Self {
        match raw {
            "info" => Self::Info,
            "critical" => Self::Critical,
            _ => Self::Warning,
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleKind {
    /// Comparaison directe du résultat de la requête à un seuil.
    Threshold,
    /// Écart à la baseline saisonnière, jalon 5.
    Anomaly,
    /// Extrapolation `predict_linear()`, entièrement calculée par VictoriaMetrics.
    Predict,
    /// Nombre de lignes de journal qui répondent à une requête LogsQL, sur une
    /// fenêtre glissante (`params.log_window_secs`), comparé à un seuil. Évaluée
    /// par VictoriaLogs, par cible ou en global.
    #[serde(rename = "log_pattern")]
    LogPattern,
    /// Prévision de remplissage : « plein dans N jours », calculée par
    /// [`crate::alerting::forecast`] sur l'historique des familles de capacité
    /// ([`crate::alerting::capacity`]), robuste aux trous et aux paliers. La
    /// valeur comparée au seuil est un nombre de jours.
    Forecast,
    /// Dérive de baseline : écart à la normale (même heure, mêmes jours des
    /// semaines précédentes, ou médiane mobile) calculé par
    /// [`crate::alerting::drift`]. La valeur comparée au seuil est le rapport
    /// entre la mesure et sa normale.
    Drift,
}

impl RuleKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Threshold => "threshold",
            Self::Anomaly => "anomaly",
            Self::Predict => "predict",
            Self::LogPattern => "log_pattern",
            Self::Forecast => "forecast",
            Self::Drift => "drift",
        }
    }

    pub fn parse(raw: &str) -> Self {
        match raw {
            "anomaly" => Self::Anomaly,
            "predict" => Self::Predict,
            "log_pattern" => Self::LogPattern,
            "forecast" => Self::Forecast,
            "drift" => Self::Drift,
            _ => Self::Threshold,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Operator {
    #[serde(rename = ">")]
    Gt,
    #[serde(rename = ">=")]
    Ge,
    #[serde(rename = "<")]
    Lt,
    #[serde(rename = "<=")]
    Le,
}

impl Operator {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Gt => ">",
            Self::Ge => ">=",
            Self::Lt => "<",
            Self::Le => "<=",
        }
    }

    pub fn parse(raw: &str) -> Self {
        match raw {
            ">=" => Self::Ge,
            "<" => Self::Lt,
            "<=" => Self::Le,
            _ => Self::Gt,
        }
    }

    /// Applique l'opérateur. Une valeur non finie n'est jamais une violation : c'est
    /// un trou de collecte, pas un dépassement de seuil.
    pub fn test(self, value: f64, threshold: f64) -> bool {
        if !value.is_finite() {
            return false;
        }
        match self {
            Self::Gt => value > threshold,
            Self::Ge => value >= threshold,
            Self::Lt => value < threshold,
            Self::Le => value <= threshold,
        }
    }

    /// Applique l'opérateur avec hystérésis.
    ///
    /// Une condition déjà vraie (`active`) ne retombe que lorsque la valeur repasse
    /// le seuil de retour dans le sens opposé : pour « > 90 » avec un retour à 80,
    /// la condition tient tant que la valeur reste au-dessus de 80. Sans seuil de
    /// retour, ou pour une condition encore fausse, c'est [`Self::test`] tel quel.
    /// Un seuil de retour incohérent (du mauvais côté du seuil) est ignoré plutôt
    /// que de rendre l'alerte impossible à résoudre.
    pub fn holds(self, value: f64, threshold: f64, clear: Option<f64>, active: bool) -> bool {
        if self.test(value, threshold) {
            return true;
        }
        let Some(clear) = clear.filter(|clear| clear.is_finite()) else { return false };
        if !active || !value.is_finite() {
            return false;
        }
        match self {
            Self::Gt | Self::Ge => clear < threshold && value >= clear,
            Self::Lt | Self::Le => clear > threshold && value <= clear,
        }
    }
}

impl std::fmt::Display for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Portée d'une règle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum TargetSelector {
    /// Toutes les cibles. C'est le défaut des règles livrées : le produit doit
    /// alerter utilement sans que personne n'ait rien configuré.
    #[default]
    All,
    Ids {
        ids: Vec<TargetId>,
    },
    /// Correspondance exacte sur les étiquettes de la cible (ses `tags`).
    Labels {
        labels: BTreeMap<String, String>,
    },
}

impl TargetSelector {
    /// Une série sans cible identifiée n'est retenue que par le sélecteur `All` :
    /// on ne peut pas prétendre qu'elle correspond à une liste ou à des étiquettes
    /// qu'on est incapable de lire.
    pub fn matches(&self, target: Option<&TargetNode>) -> bool {
        match self {
            Self::All => true,
            Self::Ids { ids } => target.is_some_and(|t| ids.contains(&t.id)),
            Self::Labels { labels } => target
                .is_some_and(|t| labels.iter().all(|(key, value)| t.tags.get(key) == Some(value))),
        }
    }
}

/// Réglages de la détection d'anomalie, stockés dans `alert_rules.params`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AnomalyParams {
    /// Seuil du score robuste au-delà duquel le point est jugé anormal.
    pub k: f64,
    /// Facteur de lissage de l'EWMA. 0,05 ≈ une constante de temps de vingt points
    /// dans le seau, soit une vingtaine de semaines : la baseline suit les dérives
    /// saisonnières sans se laisser réécrire par un incident isolé.
    pub alpha: f64,
    /// Plancher absolu de l'échelle. Obligatoire : sans lui, une série parfaitement
    /// plate a une MAD nulle, et tout écart, si minime soit-il, donne un score infini.
    pub mad_floor_abs: f64,
    /// Plancher relatif au niveau de la baseline, pour que le plancher garde un sens
    /// quelle que soit l'unité (des octets par seconde et des degrés Celsius
    /// n'ont pas la même échelle naturelle).
    pub mad_floor_rel: f64,
    /// Nombre minimal d'observations dans le seau avant de scorer.
    pub min_samples: u32,
    /// Règle « motif de journal » : fenêtre de comptage, en secondes.
    #[serde(default = "default_log_window")]
    pub log_window_secs: u64,
    /// Règle « motif de journal » : une alerte par cible (`true`) ou une seule
    /// pour l'ensemble.
    #[serde(default = "default_log_per_target")]
    pub log_per_target: bool,
    /// Règle de prévision : profondeur de l'historique régressé, en jours.
    #[serde(default = "default_forecast_window_days")]
    pub forecast_window_days: u32,
    /// Règle de prévision : en dessous de ce nombre de jours, l'alerte passe au
    /// moins en avertissement, quelle que soit la sévérité de la règle.
    #[serde(default = "default_forecast_warning_days")]
    pub forecast_warning_days: f64,
    /// Règle de prévision : en dessous de ce nombre de jours, l'alerte devient
    /// critique.
    #[serde(default = "default_forecast_critical_days")]
    pub forecast_critical_days: f64,
    /// Règle de prévision : confiance minimale (0 à 1) pour qu'une prévision ait
    /// le droit d'alerter. En dessous, elle reste visible dans l'API.
    #[serde(default = "default_forecast_min_confidence")]
    pub forecast_min_confidence: f64,
    /// Règle de prévision sur une requête personnalisée : valeur à laquelle la
    /// série est « pleine ». Les familles livrées sont en pourcentage (100).
    #[serde(default = "default_forecast_limit")]
    pub forecast_limit: f64,
    /// Règle de dérive : fenêtre de la mesure courante, en secondes.
    #[serde(default = "default_drift_window")]
    pub drift_window_secs: u64,
    /// Règle de dérive : nombre de semaines précédentes comparées à la même
    /// heure (saisonnalité). Zéro : médiane mobile sur sept jours seulement.
    #[serde(default = "default_drift_weeks")]
    pub drift_weeks: u32,
    /// Règle de dérive : nombre minimal de points dans la fenêtre courante.
    /// En dessous, la mesure est trop maigre pour être jugée.
    #[serde(default = "default_drift_min_samples")]
    pub drift_min_samples: u32,
    /// Règle de dérive : valeur en deçà de laquelle l'écart ne compte pas (une
    /// latence qui passe de 2 à 6 ms triple sans que personne ne s'en soucie).
    #[serde(default)]
    pub drift_min_value: f64,
    /// Règle de dérive : écart absolu minimal entre la mesure et sa normale.
    #[serde(default)]
    pub drift_min_delta: f64,
    /// Règle de dérive : sens surveillé.
    #[serde(default)]
    pub drift_direction: DriftDirection,
}

/// Sens d'une dérive surveillée.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DriftDirection {
    /// La mesure monte au-dessus de sa normale : latence, erreurs.
    #[default]
    Up,
    /// La mesure tombe sous sa normale : trafic, taux de cache.
    Down,
    Both,
}

/// Valeurs par défaut des règles de prévision : sept jours d'historique,
/// avertissement sous quatorze jours, critique sous sept.
pub const DEFAULT_FORECAST_WINDOW_DAYS: u32 = 7;
pub const DEFAULT_FORECAST_WARNING_DAYS: f64 = 14.0;
pub const DEFAULT_FORECAST_CRITICAL_DAYS: f64 = 7.0;
pub const DEFAULT_FORECAST_MIN_CONFIDENCE: f64 = 0.5;
/// Bornes de la fenêtre d'une règle de prévision, en jours.
pub const MIN_FORECAST_WINDOW_DAYS: u32 = 2;
pub const MAX_FORECAST_WINDOW_DAYS: u32 = 90;
/// Fenêtre par défaut de la mesure courante d'une règle de dérive.
pub const DEFAULT_DRIFT_WINDOW_SECS: u64 = 900;
pub const DEFAULT_DRIFT_WEEKS: u32 = 4;
pub const MAX_DRIFT_WEEKS: u32 = 8;
pub const DEFAULT_DRIFT_MIN_SAMPLES: u32 = 5;

fn default_forecast_window_days() -> u32 {
    DEFAULT_FORECAST_WINDOW_DAYS
}

fn default_forecast_warning_days() -> f64 {
    DEFAULT_FORECAST_WARNING_DAYS
}

fn default_forecast_critical_days() -> f64 {
    DEFAULT_FORECAST_CRITICAL_DAYS
}

fn default_forecast_min_confidence() -> f64 {
    DEFAULT_FORECAST_MIN_CONFIDENCE
}

fn default_forecast_limit() -> f64 {
    100.0
}

fn default_drift_window() -> u64 {
    DEFAULT_DRIFT_WINDOW_SECS
}

fn default_drift_weeks() -> u32 {
    DEFAULT_DRIFT_WEEKS
}

fn default_drift_min_samples() -> u32 {
    DEFAULT_DRIFT_MIN_SAMPLES
}

/// Fenêtre par défaut d'une règle « motif de journal » : cinq minutes.
pub const DEFAULT_LOG_WINDOW_SECS: u64 = 300;
/// Bornes de la fenêtre d'une règle « motif de journal ».
pub const MIN_LOG_WINDOW_SECS: u64 = 60;
pub const MAX_LOG_WINDOW_SECS: u64 = 86_400;

fn default_log_window() -> u64 {
    DEFAULT_LOG_WINDOW_SECS
}

fn default_log_per_target() -> bool {
    true
}

impl Default for AnomalyParams {
    fn default() -> Self {
        Self {
            k: 3.5,
            alpha: 0.05,
            mad_floor_abs: 1e-6,
            mad_floor_rel: 0.01,
            min_samples: 3,
            log_window_secs: DEFAULT_LOG_WINDOW_SECS,
            log_per_target: true,
            forecast_window_days: DEFAULT_FORECAST_WINDOW_DAYS,
            forecast_warning_days: DEFAULT_FORECAST_WARNING_DAYS,
            forecast_critical_days: DEFAULT_FORECAST_CRITICAL_DAYS,
            forecast_min_confidence: DEFAULT_FORECAST_MIN_CONFIDENCE,
            forecast_limit: 100.0,
            drift_window_secs: DEFAULT_DRIFT_WINDOW_SECS,
            drift_weeks: DEFAULT_DRIFT_WEEKS,
            drift_min_samples: DEFAULT_DRIFT_MIN_SAMPLES,
            drift_min_value: 0.0,
            drift_min_delta: 0.0,
            drift_direction: DriftDirection::Up,
        }
    }
}

/// Règle d'alerte telle qu'utilisée par le moteur.
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub id: i64,
    pub uid: String,
    pub name: String,
    pub description: String,
    pub kind: RuleKind,
    /// Expression MetricsQL évaluée en instantané. Pour une règle prédictive, c'est
    /// un `predict_linear(...)` complet : aucun calcul n'est refait côté Rust.
    pub query: String,
    pub operator: Operator,
    pub threshold: f64,
    /// Seuil de retour au calme (hystérésis). `None` : la condition retombe dès
    /// que le seuil n'est plus franchi. Renseigné, une alerte déjà active ne se
    /// résout que lorsque la valeur repasse de l'autre côté de ce seuil, ce qui
    /// évite qu'une valeur oscillant autour du seuil ne déclenche et résolve en
    /// boucle.
    pub clear_threshold: Option<f64>,
    /// Durée pendant laquelle la condition doit tenir avant de déclencher.
    pub for_duration: Duration,
    pub severity: Severity,
    pub selector: TargetSelector,
    pub channels: Vec<i64>,
    pub params: AnomalyParams,
    /// Suffixe d'affichage dans les messages : « % », « °C », « Go »…
    pub unit: String,
    /// Période de rappel tant que l'alerte reste active. `None` : pas de rappel.
    pub repeat_interval: Option<Duration>,
    /// Délai au-delà duquel la sévérité notifiée est relevée d'un cran.
    pub escalate_after: Option<Duration>,
    pub enabled: bool,
    pub builtin: bool,
}

impl Rule {
    /// La règle qui définit « cet équipement est hors ligne ».
    pub fn is_host_down(&self) -> bool {
        self.uid == RULE_HOST_DOWN
    }

    /// La règle dit « cette cible est indisponible », quelle qu'en soit la
    /// cause : plus aucune mesure (`host_down`) ou sonde de disponibilité en
    /// échec (`service_down`, un ping à 100 % de perte compris).
    ///
    /// C'est ce qui fait d'un parent la cause racine de la panne de ses
    /// descendants. Un simple avertissement (disque plein, processeur, service
    /// lent, certificat) n'en fait pas partie : l'équipement répond, ce qui est
    /// derrière lui aussi. Les règles des collecteurs qui parlent d'un composant
    /// interne (« Proxmox service down », un membre MongoDB injoignable…) n'en
    /// font pas partie non plus : la cible elle-même reste joignable.
    pub fn means_target_down(&self) -> bool {
        self.uid == RULE_HOST_DOWN || self.uid == RULE_SERVICE_DOWN
    }
}

/// Vue minimale d'une cible, suffisante pour l'alerting.
///
/// Le moteur ne dépend volontairement pas de `dumbmonit_proto::Target` : il n'a
/// besoin que de la filiation et des étiquettes, et les charger lui-même évite de
/// déchiffrer des identifiants d'équipement qu'il n'utilisera jamais.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetNode {
    pub id: TargetId,
    pub name: String,
    pub address: String,
    /// Collecteur qui interroge cette cible (`snmp`, `proxmox`, `agent`…).
    /// Le routage par étiquettes s'en sert pour dire « tous les hyperviseurs ».
    #[serde(default)]
    pub kind: String,
    pub parent_id: Option<TargetId>,
    /// Agent relais qui interroge cette cible. Il compte comme un parent pour
    /// la suppression : sans lui, l'équipement n'est plus observé du tout.
    #[serde(default)]
    pub via_agent: Option<TargetId>,
    pub tags: BTreeMap<String, String>,
    /// Une cible désactivée reste dans la topologie — elle peut être le parent
    /// d'une cible active — mais n'a plus le droit de porter une alerte.
    pub enabled: bool,
    /// Le dernier résultat de sonde enregistré dit « injoignable » (délai dépassé,
    /// équipement muet). C'est le verdict que l'interface affiche, et il tombe
    /// bien avant l'alerte `host_down` : celle-ci attend trois minutes sans
    /// mesure, plus sa durée de confirmation. Pour la suppression, ce verdict
    /// suffit — un relais déclaré muet par le serveur explique déjà pourquoi ses
    /// équipements ne répondent plus, sans attendre que sa propre alerte parte.
    #[serde(default)]
    pub unreachable: bool,
    /// Le dernier résultat de sonde enregistré dit « Service down » : la sonde de
    /// disponibilité a tourné et échoué. Comme `unreachable`, ce verdict tombe
    /// avant l'alerte `service_down` (qui attend sa durée de confirmation) et
    /// suffit pour masquer les descendants ; contrairement à lui, il n'étouffe pas
    /// les autres alertes de la cible elle-même.
    #[serde(default)]
    pub service_down: bool,
}

/// Empreinte stable d'une alerte : règle + série.
///
/// Le préfixe reste lisible dans les journaux, le suffixe condense la série pour
/// que la clé primaire garde une longueur bornée quelles que soient les étiquettes.
pub fn fingerprint(rule_uid: &str, series_key: &str) -> String {
    format!("{rule_uid}@{:016x}", fnv1a64(series_key))
}

/// FNV-1a 64 bits. Une empreinte n'a aucune exigence cryptographique ici : elle doit
/// seulement être stable entre deux démarrages, ce que `DefaultHasher` ne garantit pas.
fn fnv1a64(input: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Clé canonique d'une série, au format Prometheus, à partir de ses étiquettes.
///
/// L'étiquette `__name__` est extraite comme nom de métrique afin que la clé soit
/// identique à celle qu'aurait produite `Sample::series_key`.
pub fn series_key(labels: &BTreeMap<String, String>) -> String {
    render_key(labels, |_| true)
}

/// Préfixe des étiquettes recopiées depuis les `tags` de la cible par
/// `Target::base_labels()`.
pub const TAG_LABEL_PREFIX: &str = "tag_";

/// Nom d'étiquette (sans le préfixe) sous lequel un tag est exporté : tout ce
/// qui sort de `[a-zA-Z0-9_]` devient `_`, comme dans `Target::base_labels()`.
pub fn tag_label_key(tag: &str) -> String {
    tag.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' }).collect()
}

/// Vrai si la série porte l'identifiant de sa cible.
pub fn carries_target_id(labels: &BTreeMap<String, String>) -> bool {
    TARGET_ID_LABELS.iter().any(|label| labels.contains_key(*label))
}

/// Clé d'identité d'une alerte : la série, débarrassée de ce qui ne fait que
/// décrire la cible.
///
/// Dès qu'une série porte l'identifiant de sa cible, son nom (`host`) et ses
/// `tag_*` sont redondants — et changeants : renommer l'équipement ou retoucher
/// ses étiquettes crée une nouvelle série dans VictoriaMetrics, qui ne doit pas
/// devenir une nouvelle alerte pendant que l'ancienne, muette, passe pour une
/// panne. Sans identifiant, la clé de série sert telle quelle.
pub fn identity_key(labels: &BTreeMap<String, String>) -> String {
    if !carries_target_id(labels) {
        return series_key(labels);
    }
    render_key(labels, |label| {
        !TARGET_NAME_LABELS.contains(&label) && !label.starts_with(TAG_LABEL_PREFIX)
    })
}

fn render_key(labels: &BTreeMap<String, String>, keep: impl Fn(&str) -> bool) -> String {
    let name = labels.get("__name__").map(String::as_str).unwrap_or("");
    let mut key = String::with_capacity(name.len() + 16 * labels.len());
    key.push_str(name);
    let mut first = true;
    for (label, value) in
        labels.iter().filter(|(k, _)| k.as_str() != "__name__" && keep(k.as_str()))
    {
        key.push(if first { '{' } else { ',' });
        first = false;
        key.push_str(label);
        key.push_str("=\"");
        key.push_str(value);
        key.push('"');
    }
    if !first {
        key.push('}');
    }
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: TargetId, tags: &[(&str, &str)]) -> TargetNode {
        TargetNode {
            id,
            name: format!("device-{id}"),
            address: format!("10.0.0.{id}"),
            kind: "snmp".to_string(),
            parent_id: None,
            via_agent: None,
            tags: tags.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect(),
            enabled: true,
            unreachable: false,
            service_down: false,
        }
    }

    #[test]
    fn les_operateurs_comparent_comme_attendu() {
        assert!(Operator::Gt.test(91.0, 90.0));
        assert!(!Operator::Gt.test(90.0, 90.0));
        assert!(Operator::Ge.test(90.0, 90.0));
        assert!(Operator::Lt.test(0.0, 1.0));
        assert!(Operator::Le.test(1.0, 1.0));
    }

    #[test]
    fn l_hysteresis_ne_relache_qu_au_seuil_de_retour() {
        // Déclenche > 90, se résout < 80.
        assert!(Operator::Gt.holds(85.0, 90.0, Some(80.0), true), "still above the clear line");
        assert!(!Operator::Gt.holds(85.0, 90.0, Some(80.0), false), "not active: plain test");
        assert!(!Operator::Gt.holds(79.0, 90.0, Some(80.0), true), "below the clear line");
        assert!(Operator::Gt.holds(91.0, 90.0, Some(80.0), false));
        // Sens inverse : déclenche < 10, se résout > 20.
        assert!(Operator::Lt.holds(15.0, 10.0, Some(20.0), true));
        assert!(!Operator::Lt.holds(21.0, 10.0, Some(20.0), true));
        // Seuil de retour incohérent : ignoré.
        assert!(!Operator::Gt.holds(85.0, 90.0, Some(95.0), true));
        // Sans seuil de retour : comportement historique.
        assert!(!Operator::Gt.holds(85.0, 90.0, None, true));
    }

    #[test]
    fn une_valeur_non_finie_ne_declenche_jamais() {
        for operator in [Operator::Gt, Operator::Ge, Operator::Lt, Operator::Le] {
            assert!(!operator.test(f64::NAN, 0.0));
            assert!(!operator.test(f64::INFINITY, 0.0));
            assert!(!operator.test(f64::NEG_INFINITY, 0.0));
        }
    }

    #[test]
    fn l_escalade_plafonne_a_critical() {
        assert_eq!(Severity::Info.escalated(), Severity::Warning);
        assert_eq!(Severity::Warning.escalated(), Severity::Critical);
        assert_eq!(Severity::Critical.escalated(), Severity::Critical);
    }

    #[test]
    fn le_selecteur_par_etiquettes_exige_toutes_les_correspondances() {
        let selector = TargetSelector::Labels {
            labels: [("role".to_string(), "nas".to_string())].into_iter().collect(),
        };
        assert!(selector.matches(Some(&node(1, &[("role", "nas"), ("site", "home")]))));
        assert!(!selector.matches(Some(&node(2, &[("role", "switch")]))));
        assert!(!selector.matches(None));
    }

    #[test]
    fn le_selecteur_all_accepte_une_serie_sans_cible() {
        assert!(TargetSelector::All.matches(None));
        assert!(!TargetSelector::Ids { ids: vec![1] }.matches(None));
        assert!(TargetSelector::Ids { ids: vec![1] }.matches(Some(&node(1, &[]))));
    }

    #[test]
    fn les_empreintes_sont_stables_et_distinctes() {
        let a = fingerprint("cpu_high", "cpu{host=\"nas\"}");
        assert_eq!(a, fingerprint("cpu_high", "cpu{host=\"nas\"}"));
        assert_ne!(a, fingerprint("cpu_high", "cpu{host=\"router\"}"));
        assert_ne!(a, fingerprint("cpu_low", "cpu{host=\"nas\"}"));
        assert!(a.starts_with("cpu_high@"));
    }

    #[test]
    fn la_cle_de_serie_reprend_le_format_prometheus() {
        let labels: BTreeMap<String, String> =
            [("__name__", "dumbmonit_cpu_usage_percent"), ("host", "nas"), ("core", "0")]
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect();
        assert_eq!(series_key(&labels), "dumbmonit_cpu_usage_percent{core=\"0\",host=\"nas\"}");
    }

    #[test]
    fn la_cle_d_identite_ignore_le_nom_et_les_tags_quand_la_cible_est_identifiee() {
        let labels: BTreeMap<String, String> = [
            ("__name__", "dumbmonit_up"),
            ("host", "nas"),
            ("tag_role", "storage"),
            ("target", "4"),
            ("mountpoint", "/"),
        ]
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect();
        assert_eq!(identity_key(&labels), "dumbmonit_up{mountpoint=\"/\",target=\"4\"}");

        // Renommer l'équipement ne change pas l'identité de l'alerte.
        let mut renamed = labels.clone();
        renamed.insert("host".to_string(), "nas-2".to_string());
        assert_eq!(identity_key(&renamed), identity_key(&labels));
        assert_ne!(series_key(&renamed), series_key(&labels));

        // Sans identifiant de cible, la clé de série reste l'identité.
        let mut anonymous = labels.clone();
        anonymous.remove("target");
        assert_eq!(identity_key(&anonymous), series_key(&anonymous));
        assert!(!carries_target_id(&anonymous));
    }

    #[test]
    fn une_serie_sans_etiquette_garde_son_nom_seul() {
        let labels: BTreeMap<String, String> =
            [("__name__".to_string(), "dumbmonit_up".to_string())].into_iter().collect();
        assert_eq!(series_key(&labels), "dumbmonit_up");
    }

    #[test]
    fn le_selecteur_se_serialise_en_json_stable() {
        let json = serde_json::to_string(&TargetSelector::Ids { ids: vec![3, 7] }).unwrap();
        assert_eq!(json, r#"{"kind":"ids","ids":[3,7]}"#);
        assert_eq!(
            serde_json::from_str::<TargetSelector>(r#"{"kind":"all"}"#).unwrap(),
            TargetSelector::All
        );
    }
}
