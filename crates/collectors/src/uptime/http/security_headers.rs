//! Analyse des en-têtes de sécurité d'une réponse HTTPS.
//!
//! Fonction pure, partagée entre la sonde `http` et le collecteur `website` du
//! serveur : les deux émettent les mêmes séries `*_security_header{header=…}`.

use reqwest::header::HeaderMap;

/// Valeurs non vides d'un en-tête (toutes les lignes), jointes par une virgule.
fn joined(headers: &HeaderMap, name: &str) -> Option<String> {
    let values: Vec<&str> = headers
        .get_all(name)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .collect();
    if values.is_empty() { None } else { Some(values.join(", ")) }
}

/// Les cinq contrôles, dans l'ordre d'émission : (étiquette `header`, présent et valide).
pub fn security_headers(headers: &HeaderMap) -> [(&'static str, bool); 5] {
    let hsts = joined(headers, "strict-transport-security").is_some_and(|v| hsts_max_age(&v) > 0);
    let nosniff = joined(headers, "x-content-type-options")
        .is_some_and(|v| v.split(',').any(|part| part.trim().eq_ignore_ascii_case("nosniff")));
    let csp = joined(headers, "content-security-policy");
    let frame_options = joined(headers, "x-frame-options")
        .is_some_and(|v| v.eq_ignore_ascii_case("deny") || v.eq_ignore_ascii_case("sameorigin"));
    let frame_ancestors = csp.as_deref().is_some_and(|v| {
        v.split([';', ',']).any(|directive| {
            directive
                .split_whitespace()
                .next()
                .is_some_and(|name| name.eq_ignore_ascii_case("frame-ancestors"))
        })
    });

    [
        ("strict-transport-security", hsts),
        ("x-content-type-options", nosniff),
        ("content-security-policy", csp.is_some()),
        ("x-frame-options", frame_options),
        ("frame-ancestors", frame_ancestors),
    ]
}

/// Valeur de `max-age` (0 si absente ou illisible).
fn hsts_max_age(value: &str) -> u64 {
    value
        .split(';')
        .find_map(|directive| {
            let (key, val) = directive.split_once('=')?;
            key.trim()
                .eq_ignore_ascii_case("max-age")
                .then(|| val.trim().trim_matches('"').parse::<u64>().unwrap_or(0))
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderName, HeaderValue};

    fn map(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.append(
                HeaderName::from_bytes(k.as_bytes()).unwrap(),
                HeaderValue::from_str(v).unwrap(),
            );
        }
        h
    }

    fn get(result: &[(&'static str, bool); 5], name: &str) -> bool {
        result.iter().find(|(n, _)| *n == name).unwrap().1
    }

    #[test]
    fn absents() {
        let r = security_headers(&HeaderMap::new());
        assert!(r.iter().all(|(_, ok)| !ok));
    }

    #[test]
    fn tout_present_et_casse() {
        let r = security_headers(&map(&[
            ("Strict-Transport-Security", "max-age=31536000; includeSubDomains"),
            ("X-Content-Type-Options", "NoSniff"),
            ("Content-Security-Policy", "default-src 'self'; Frame-Ancestors 'none'"),
            ("X-Frame-Options", "sameorigin"),
        ]));
        assert!(r.iter().all(|(_, ok)| *ok));
    }

    #[test]
    fn hsts_max_age_zero() {
        let r = security_headers(&map(&[("strict-transport-security", "max-age=0")]));
        assert!(!get(&r, "strict-transport-security"));
        let r = security_headers(&map(&[("strict-transport-security", "includeSubDomains")]));
        assert!(!get(&r, "strict-transport-security"));
    }

    #[test]
    fn csp_report_only_ne_compte_pas() {
        let r = security_headers(&map(&[(
            "content-security-policy-report-only",
            "default-src 'self'; frame-ancestors 'none'",
        )]));
        assert!(!get(&r, "content-security-policy"));
        assert!(!get(&r, "frame-ancestors"));
    }

    #[test]
    fn csp_sans_frame_ancestors() {
        let r = security_headers(&map(&[("content-security-policy", "default-src 'self'")]));
        assert!(get(&r, "content-security-policy"));
        assert!(!get(&r, "frame-ancestors"));
    }

    #[test]
    fn valeurs_invalides() {
        let r = security_headers(&map(&[
            ("x-content-type-options", "sniff"),
            ("x-frame-options", "ALLOW-FROM https://a.example"),
            ("content-security-policy", "  "),
        ]));
        assert!(r.iter().all(|(_, ok)| !ok));
    }
}
