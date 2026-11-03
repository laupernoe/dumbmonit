//! Certificats, paquets et connexions de DSM : ce que le Panneau de configuration
//! montre au-delà du stockage.
//!
//! | Métrique | API (méthode) | Champs lus |
//! |---|---|---|
//! | `synology_certificate_expires_in_seconds`, `synology_certificate_broken` | `SYNO.Core.Certificate.CRT` (`list`) | `certificates[].valid_till`, `is_broken`, `is_default`, `desc`, `subject.common_name` |
//! | `synology_packages_installed`, `synology_package_running` | `SYNO.Core.Package` (`list`, `additional=["status"]`) | `packages[].id`, `name`, `additional.status` |
//! | `synology_connections`, `synology_connections_total` | `SYNO.Core.CurrentConnection` (`list`) | `items[].type`, `total` |
//!
//! # Ce que ces lectures valent
//!
//! Ces trois API ne sont pas documentées par Synology. Les noms et la forme des
//! réponses viennent de la bibliothèque communautaire `N4S4/synology-api`
//! (`certificate.py`, `core_package.py`, `core_sys_info.py`), pas d'un relevé de
//! NAS : contrairement au stockage (`testdata/dsm74/`), **elles n'ont pas été
//! validées sur un vrai DSM**. D'où des règles strictes :
//!
//! * **tout est facultatif et silencieux.** API absente du catalogue, refus,
//!   champ manquant ou forme inattendue : pas de série, un `debug!` au plus,
//!   jamais d'erreur comptée dans `synology_scrape_errors` ni de sonde en échec ;
//! * **aucune règle livrée ne dépend d'un champ douteux sans protection** : la
//!   règle d'expiration ne vise que des dates que l'on a su lire ;
//! * **cadence lente** pour les certificats et les paquets (une lecture par heure,
//!   comme [`super::security`]) : ils changent rarement. L'échéance d'un certificat
//!   est gardée en valeur absolue et le temps restant recalculé à chaque
//!   interrogation, donc exact même entre deux lectures.
//!
//! Le compte de connexions n'est pas mis en cache : c'est une photographie.
//! Il ne retient aucun nom d'utilisateur ni adresse, seulement le protocole.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use chrono::NaiveDateTime;
use dumbmonit_proto::{MetricKind, Sample};
use serde_json::Value;
use tracing::debug;

use super::client::DsmClient;

pub const API_CERTIFICATE: &str = "SYNO.Core.Certificate.CRT";
pub const API_PACKAGE: &str = "SYNO.Core.Package";
pub const API_CONNECTION: &str = "SYNO.Core.CurrentConnection";

/// Intervalle entre deux lectures réelles des certificats et des paquets.
pub const REFRESH: Duration = Duration::from_secs(3600);

/// Budget d'un tour de lecture lente.
const BUDGET: Duration = Duration::from_secs(15);

/// Certificats et paquets nommés au plus : au-delà, la cardinalité l'emporte sur
/// l'intérêt.
const MAX_CERTIFICATES: usize = 16;
const MAX_PACKAGES: usize = 64;
const MAX_PROTOCOLS: usize = 16;

/// Un certificat de DSM.
#[derive(Debug, Clone, PartialEq)]
pub struct Certificate {
    pub label: String,
    pub subject: String,
    pub is_default: bool,
    pub broken: bool,
    /// Fin de validité, en secondes Unix (UTC).
    pub expires_at: i64,
}

/// Un paquet installé.
#[derive(Debug, Clone, PartialEq)]
pub struct Package {
    pub id: String,
    pub name: String,
    pub running: bool,
}

/// Dernières lectures lentes d'une cible.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub at: Option<Instant>,
    pub certificates: Vec<Certificate>,
    pub packages: Option<Vec<Package>>,
}

impl Snapshot {
    pub fn is_fresh(&self, now: Instant) -> bool {
        self.at.is_some_and(|at| now.duration_since(at) < REFRESH)
    }

    /// Les séries à publier à `ts_ms`, le temps restant avant expiration étant
    /// recalculé à cet instant.
    pub fn samples(&self, ts_ms: i64) -> Vec<Sample> {
        let mut out = certificate_samples(&self.certificates, ts_ms / 1000, ts_ms);
        if let Some(packages) = &self.packages {
            out.extend(package_samples(packages, ts_ms));
        }
        out
    }
}

fn gauge(metric: &str, value: f64, ts_ms: i64) -> Sample {
    Sample::new(metric, value, MetricKind::Gauge, ts_ms)
}

// ---------------------------------------------------------------- lecture

/// Relit les certificats et les paquets, en gardant l'ancienne valeur de ce qui
/// n'a pas pu être lu.
pub async fn refresh(dsm: &DsmClient, mut snapshot: Snapshot) -> Snapshot {
    let read = async {
        if dsm.supports(API_CERTIFICATE) {
            match dsm.call::<Value>(API_CERTIFICATE, 1, "list", &[]).await {
                Ok(data) => snapshot.certificates = parse_certificates(&data),
                Err(error) => debug!(%error, "certificats DSM illisibles (non administrateur ?)"),
            }
        }
        if dsm.supports(API_PACKAGE) {
            let extra = [("additional", "[\"status\"]".to_string())];
            match dsm.call::<Value>(API_PACKAGE, 1, "list", &extra).await {
                Ok(data) => snapshot.packages = parse_packages(&data),
                Err(error) => debug!(%error, "paquets DSM illisibles"),
            }
        }
    };
    if tokio::time::timeout(BUDGET, read).await.is_err() {
        debug!("certificats et paquets DSM : délai dépassé, valeurs précédentes conservées");
    }
    snapshot.at = Some(Instant::now());
    snapshot
}

/// Connexions ouvertes en ce moment, comptées par protocole. Vide sur tout échec.
pub async fn connections(dsm: &DsmClient, ts_ms: i64) -> Vec<Sample> {
    if !dsm.supports(API_CONNECTION) {
        return Vec::new();
    }
    let extra = [("start", "0".to_string()), ("limit", "500".to_string())];
    let call = dsm.call::<Value>(API_CONNECTION, 1, "list", &extra);
    match tokio::time::timeout(Duration::from_secs(8), call).await {
        Ok(Ok(data)) => connection_samples(&data, ts_ms),
        Ok(Err(error)) => {
            debug!(%error, "connexions DSM illisibles");
            Vec::new()
        }
        Err(_) => Vec::new(),
    }
}

// ------------------------------------------------------------ traduction

/// Date de fin de validité d'un certificat : `"Sep  7 23:59:59 2024 GMT"` (la
/// forme d'OpenSSL, avec un jour complété par une espace), ou à défaut une date
/// ISO ou un horodatage Unix.
pub fn parse_valid_till(value: &Value) -> Option<i64> {
    if let Some(seconds) = value.as_i64().filter(|n| *n > 1_000_000_000) {
        return Some(seconds);
    }
    let text = value.as_str()?;
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = collapsed.strip_suffix(" GMT").unwrap_or(&collapsed);
    ["%b %d %H:%M:%S %Y", "%b %e %H:%M:%S %Y", "%Y-%m-%d %H:%M:%S", "%Y/%m/%d %H:%M:%S"]
        .into_iter()
        .find_map(|format| NaiveDateTime::parse_from_str(trimmed, format).ok())
        .map(|date| date.and_utc().timestamp())
}

fn flag(value: Option<&Value>) -> bool {
    match value {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_i64().is_some_and(|n| n != 0),
        _ => false,
    }
}

fn text(value: Option<&Value>) -> String {
    value.and_then(Value::as_str).map(str::trim).unwrap_or_default().to_string()
}

/// `data.certificates[]`. Un certificat sans date lisible est ignoré : mieux vaut
/// pas de série qu'une échéance inventée.
pub fn parse_certificates(data: &Value) -> Vec<Certificate> {
    let Some(list) = data.get("certificates").and_then(Value::as_array) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|entry| {
            let expires_at = parse_valid_till(entry.get("valid_till")?)?;
            let subject = text(entry.pointer("/subject/common_name"));
            let label = [text(entry.get("desc")), subject.clone(), text(entry.get("id"))]
                .into_iter()
                .find(|candidate| !candidate.is_empty())?;
            Some(Certificate {
                label,
                subject,
                is_default: flag(entry.get("is_default")),
                broken: flag(entry.get("is_broken")),
                expires_at,
            })
        })
        .take(MAX_CERTIFICATES)
        .collect()
}

/// `data.packages[]`. `None` quand la forme n'est pas celle d'une liste de
/// paquets, pour distinguer « aucun paquet » de « réponse incomprise ».
pub fn parse_packages(data: &Value) -> Option<Vec<Package>> {
    let list = data.get("packages")?.as_array()?;
    Some(
        list.iter()
            .filter_map(|entry| {
                let id = text(entry.get("id"));
                if id.is_empty() {
                    return None;
                }
                let name = match text(entry.get("name")) {
                    name if name.is_empty() => id.clone(),
                    name => name,
                };
                let status = text(entry.pointer("/additional/status"));
                Some(Package { id, name, running: status.eq_ignore_ascii_case("running") })
            })
            .collect(),
    )
}

pub fn certificate_samples(certificates: &[Certificate], now_s: i64, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    for certificate in certificates {
        let labels = |sample: Sample| {
            sample
                .with_label("certificate", certificate.label.as_str())
                .with_label("subject", certificate.subject.as_str())
                .with_label("default", if certificate.is_default { "1" } else { "0" })
        };
        // Négatif une fois périmé : la valeur reste utile (« depuis combien de
        // temps ») et une règle `< seuil` l'attrape aussi.
        let remaining = (certificate.expires_at - now_s) as f64;
        out.push(labels(gauge("synology_certificate_expires_in_seconds", remaining, ts_ms)));
        let broken = if certificate.broken { 1.0 } else { 0.0 };
        out.push(labels(gauge("synology_certificate_broken", broken, ts_ms)));
    }
    out
}

pub fn package_samples(packages: &[Package], ts_ms: i64) -> Vec<Sample> {
    let mut out = vec![gauge("synology_packages_installed", packages.len() as f64, ts_ms)];
    for package in packages.iter().take(MAX_PACKAGES) {
        out.push(
            gauge("synology_package_running", if package.running { 1.0 } else { 0.0 }, ts_ms)
                .with_label("package", package.id.as_str())
                .with_label("name", package.name.as_str()),
        );
    }
    out
}

/// `data.items[].type` compté par protocole (`smb`, `https`, `ssh`…). Rien si la
/// réponse n'a pas la forme attendue.
pub fn connection_samples(data: &Value, ts_ms: i64) -> Vec<Sample> {
    let Some(items) = data.get("items").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut by_protocol: BTreeMap<String, f64> = BTreeMap::new();
    for item in items {
        let protocol = text(item.get("type")).to_ascii_lowercase();
        let protocol = if protocol.is_empty() { "other".to_string() } else { protocol };
        *by_protocol.entry(protocol).or_default() += 1.0;
    }
    let total = data.get("total").and_then(Value::as_f64).unwrap_or(items.len() as f64);
    let mut out = vec![gauge("synology_connections_total", total, ts_ms)];
    for (protocol, count) in by_protocol.into_iter().take(MAX_PROTOCOLS) {
        out.push(gauge("synology_connections", count, ts_ms).with_label("protocol", protocol));
    }
    out
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    // Réponses construites d'après la forme décrite par la bibliothèque
    // `N4S4/synology-api` (voir la documentation du module), pas relevées sur un
    // NAS : elles fixent ce que le collecteur sait lire, pas ce que DSM renvoie.
    fn certificates() -> Value {
        json!({"certificates": [
            {"id": "sQuXG0", "desc": "nas.example.org (Let's Encrypt)", "is_default": true,
             "is_broken": false,
             "issuer": {"common_name": "R11", "organization": "Let's Encrypt"},
             "subject": {"common_name": "nas.example.org", "sub_alt_name": ["nas.example.org"]},
             "valid_from": "Aug  1 00:00:00 2026 GMT", "valid_till": "Oct 30 23:59:59 2026 GMT",
             "services": [{"display_name": "DSM Desktop Service", "service": "default"}]},
            {"id": "abc123", "desc": "", "is_default": false, "is_broken": true,
             "subject": {"common_name": "synology"},
             "valid_till": "Jan  5 08:00:00 2027 GMT"},
            {"id": "nodate", "desc": "no date", "subject": {"common_name": "x"}}
        ]})
    }

    #[test]
    fn les_dates_openssl_avec_jour_complete_dune_espace_se_lisent() {
        let seconds = parse_valid_till(&json!("Jan  5 08:00:00 2027 GMT")).unwrap();
        let date = chrono::DateTime::from_timestamp(seconds, 0).unwrap();
        assert_eq!(date.format("%Y-%m-%d %H:%M:%S").to_string(), "2027-01-05 08:00:00");
        assert!(parse_valid_till(&json!("Oct 30 23:59:59 2026 GMT")).is_some());
        assert!(parse_valid_till(&json!("2027-01-05 08:00:00")).is_some());
        assert_eq!(parse_valid_till(&json!(1_798_000_000)), Some(1_798_000_000));
        assert_eq!(parse_valid_till(&json!("bientôt")), None);
        assert_eq!(parse_valid_till(&json!(null)), None);
    }

    #[test]
    fn un_certificat_sans_date_est_ignore_et_les_autres_sont_nommes() {
        let list = parse_certificates(&certificates());
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].label, "nas.example.org (Let's Encrypt)");
        assert!(list[0].is_default && !list[0].broken);
        // Sans description : le sujet nomme le certificat.
        assert_eq!(list[1].label, "synology");
        assert!(list[1].broken);
    }

    #[test]
    fn le_temps_restant_est_recalcule_a_chaque_lecture() {
        let list = parse_certificates(&certificates());
        let expires = list[0].expires_at;
        let samples = certificate_samples(&list, expires - 10 * 86_400, 0);
        let left =
            samples.iter().find(|s| s.metric == "synology_certificate_expires_in_seconds").unwrap();
        assert_eq!(left.value, 10.0 * 86_400.0);
        assert_eq!(left.labels["default"], "1");
        assert_eq!(left.labels["subject"], "nas.example.org");
        // Une fois périmé, la valeur devient négative au lieu de disparaître.
        let samples = certificate_samples(&list, expires + 3600, 0);
        assert_eq!(samples[0].value, -3600.0);
        let broken: Vec<_> =
            samples.iter().filter(|s| s.metric == "synology_certificate_broken").collect();
        assert_eq!(broken.len(), 2);
        assert_eq!(broken[1].value, 1.0);
    }

    #[test]
    fn les_paquets_arretes_ne_comptent_pas_comme_en_marche() {
        let data = json!({"packages": [
            {"id": "HyperBackup", "name": "Hyper Backup", "version": "4.1.1-3873",
             "additional": {"status": "running"}},
            {"id": "SynologyDrive", "name": "Drive Server", "additional": {"status": "stop"}},
            {"id": "ContainerManager", "additional": {}},
            {"name": "sans identifiant"}
        ]});
        let packages = parse_packages(&data).unwrap();
        assert_eq!(packages.len(), 3);
        let samples = package_samples(&packages, 0);
        assert_eq!(samples[0].value, 3.0);
        let running: Vec<(&str, f64)> = samples
            .iter()
            .filter(|s| s.metric == "synology_package_running")
            .map(|s| (s.labels["package"].as_str(), s.value))
            .collect();
        assert_eq!(
            running,
            [("HyperBackup", 1.0), ("SynologyDrive", 0.0), ("ContainerManager", 0.0)]
        );
        // Une réponse d'une autre forme n'est pas « aucun paquet ».
        assert!(parse_packages(&json!({"total": 0})).is_none());
    }

    #[test]
    fn les_connexions_sont_comptees_par_protocole_sans_garder_de_nom() {
        let data = json!({"total": 4, "items": [
            {"type": "SMB", "who": "alice", "from": "192.168.1.20"},
            {"type": "SMB", "who": "bob", "from": "192.168.1.21"},
            {"type": "HTTPS", "who": "admin", "from": "192.168.1.5"},
            {"who": "inconnu"}
        ]});
        let samples = connection_samples(&data, 0);
        assert_eq!(samples[0].metric, "synology_connections_total");
        assert_eq!(samples[0].value, 4.0);
        let smb = samples.iter().find(|s| s.labels.get("protocol").is_some_and(|p| p == "smb"));
        assert_eq!(smb.unwrap().value, 2.0);
        assert!(samples.iter().any(|s| s.labels.get("protocol").is_some_and(|p| p == "other")));
        for sample in &samples {
            assert!(sample.labels.keys().all(|key| key == "protocol"), "{:?}", sample.labels);
        }
        assert!(connection_samples(&json!({"result": []}), 0).is_empty());
    }

    #[test]
    fn le_cliche_nest_frais_quapres_une_lecture() {
        let mut snapshot = Snapshot::default();
        assert!(!snapshot.is_fresh(Instant::now()));
        snapshot.at = Some(Instant::now());
        assert!(snapshot.is_fresh(Instant::now()));
        assert!(snapshot.samples(0).is_empty(), "rien de lu, rien à publier");
    }
}
