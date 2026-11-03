//! Certificats du pare-feu (`POST /api/trust/cert/search`) : la validité qui
//! reste à chacun, pour qu'un certificat du portail captif, du VPN ou de
//! l'interface d'administration n'expire pas sans que personne l'ait vu venir.
//!
//! Ni le sujet, ni l'empreinte, ni la clé ne sortent d'ici : seul le nom que
//! l'administrateur a donné au certificat (sa description).

use dumbmonit_proto::{MetricKind, Sample};
use serde_json::Value;

use super::model::text_in;
use super::options::MAX_SERIES_PER_FAMILY;

/// Un certificat et le nombre de secondes avant son expiration (négatif une
/// fois expiré).
#[derive(Debug, Clone, PartialEq)]
pub struct CertificateExpiry {
    pub name: String,
    pub seconds_left: f64,
}

/// Date de fin de validité : secondes Unix (nombre ou chaîne), ou RFC 3339.
fn valid_to(row: &serde_json::Map<String, Value>) -> Option<f64> {
    for key in ["valid_to", "rfc3339_valid_to"] {
        let Some(raw) = row.get(key) else { continue };
        let seconds = match raw {
            Value::Number(n) => n.as_f64(),
            Value::String(text) => {
                let text = text.trim();
                let date = chrono::DateTime::parse_from_rfc3339(text).ok();
                text.parse::<f64>().ok().or(date.map(|d| d.timestamp() as f64))
            }
            _ => None,
        };
        if let Some(seconds) = seconds.filter(|s| *s > 0.0) {
            return Some(seconds);
        }
    }
    None
}

/// Un certificat explicitement marqué « inutilisé » ne compte pas : un ancien
/// certificat qui dort dans le magasin n'est pas une panne à venir.
fn unused(row: &serde_json::Map<String, Value>) -> bool {
    match row.get("in_use") {
        Some(Value::Bool(used)) => !used,
        Some(Value::Number(n)) => n.as_f64() == Some(0.0),
        Some(Value::String(s)) => matches!(s.trim(), "0" | "false"),
        _ => false,
    }
}

pub fn certificate_expiries(rows: &[Value], now_s: i64) -> Vec<CertificateExpiry> {
    rows.iter()
        .filter_map(Value::as_object)
        .filter(|row| !unused(row))
        .filter_map(|row| {
            let name = text_in(row, &["descr", "description", "commonname"])?;
            let end = valid_to(row)?;
            Some(CertificateExpiry { name, seconds_left: end - now_s as f64 })
        })
        .take(MAX_SERIES_PER_FAMILY)
        .collect()
}

pub fn certificate_samples(certificates: &[CertificateExpiry], ts_ms: i64) -> Vec<Sample> {
    certificates
        .iter()
        .map(|certificate| {
            Sample::new(
                "opnsense_certificate_expiry_seconds",
                certificate.seconds_left,
                MetricKind::Gauge,
                ts_ms,
            )
            .with_label("certificate", certificate.name.clone())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_790_000_000;

    fn rows() -> Vec<Value> {
        serde_json::from_str(
            r#"[
              {"uuid":"a","descr":"Web GUI","valid_to":"1791728000","in_use":"1"},
              {"uuid":"b","descr":"Old VPN server","valid_to":"1700000000","in_use":"0"},
              {"uuid":"c","descr":"Portal","rfc3339_valid_to":"2026-09-01T00:00:00+00:00"},
              {"uuid":"e","descr":"No date"}
            ]"#,
        )
        .unwrap()
    }

    #[test]
    fn la_validite_restante_se_calcule_et_les_certificats_dormants_sont_ignores() {
        let found = certificate_expiries(&rows(), NOW);
        let names: Vec<&str> = found.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["Web GUI", "Portal"]);
        assert_eq!(found[0].seconds_left, 1_791_728_000.0 - NOW as f64);
        // Déjà expiré : valeur négative, pas d'oubli.
        assert!(found[1].seconds_left < 0.0);
    }

    #[test]
    fn un_echantillon_par_certificat_nomme() {
        let samples = certificate_samples(&certificate_expiries(&rows(), NOW), 0);
        assert_eq!(samples.len(), 2);
        assert!(samples.iter().all(|s| s.metric == "opnsense_certificate_expiry_seconds"));
        assert_eq!(samples[0].labels["certificate"], "Web GUI");
    }
}
