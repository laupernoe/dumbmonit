//! Sites web : certificat présenté (sondes `tls` et `http`), HTTPS et
//! en-têtes de sécurité de la réponse (sonde `http` en https, et type
//! `website` « santé et référencement »).
//!
//! La sonde négocie avec rustls, qui refuse tout ce qui est antérieur à
//! TLS 1.2 : un serveur limité à TLS 1.0/1.1 apparaît comme une sonde en
//! échec, pas comme un contrôle. Le contrôle de version ne peut donc dire que
//! « TLS 1.3 négocié ou non ».
//!
//! Les en-têtes ne sont lus que sur une réponse en https (en clair, HSTS n'a
//! pas de sens) : une sonde `http` en clair n'a donc aucun fait ici et pas de
//! note, plutôt qu'une note qui punirait un service interne.

use super::super::facts::list;
use super::super::{Category, Check, CheckDef, Facts, Guide, SecurityProvider, Severity};

pub const EXPIRY: CheckDef = CheckDef {
    id: "tls.expiry",
    title: "Certificate valid for 14+ days",
    category: Category::Encryption,
    severity: Severity::High,
    reference: "https://letsencrypt.org/docs/integration-guide/",
    remediation: "Renew the certificate and automate renewal (ACME). Renew at two thirds of the \
                  lifetime, as Let's Encrypt recommends, so a failed renewal leaves time to react.",
};

pub const TRUSTED: CheckDef = CheckDef {
    id: "tls.trusted",
    title: "Certificate chain trusted",
    category: Category::Encryption,
    severity: Severity::High,
    reference: "https://wiki.mozilla.org/Security/Server_Side_TLS",
    remediation: "Serve a certificate issued by a public or internally trusted authority, with the \
                  full intermediate chain. Self-signed certificates teach users to click through \
                  warnings.",
};

pub const TLS13: CheckDef = CheckDef {
    id: "tls.version",
    title: "TLS 1.3 negotiated",
    category: Category::Encryption,
    severity: Severity::Low,
    reference: "https://csrc.nist.gov/pubs/sp/800/52/r2/final",
    remediation: "Enable TLS 1.3 on the server (NIST SP 800-52r2 requires support for it; \
                  Mozilla's \"intermediate\" profile offers TLS 1.2 and 1.3).",
};

pub const HTTPS: CheckDef = CheckDef {
    id: "web.https",
    title: "Served over HTTPS",
    category: Category::Encryption,
    severity: Severity::High,
    reference: "https://developer.mozilla.org/en-US/docs/Web/Security/Practical_implementation_guides/TLS",
    remediation: "Serve the site over HTTPS and redirect every plain HTTP request to it \
                  (a free ACME certificate from Let's Encrypt is enough).",
};

pub const HSTS: CheckDef = CheckDef {
    id: "web.header.hsts",
    title: "Strict-Transport-Security header",
    category: Category::Encryption,
    severity: Severity::Medium,
    reference: "https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Strict-Transport-Security",
    remediation: "Send `Strict-Transport-Security: max-age=31536000` (add `includeSubDomains` once \
                  every subdomain is on HTTPS) so browsers never fall back to plain HTTP.",
};

pub const NOSNIFF: CheckDef = CheckDef {
    id: "web.header.nosniff",
    title: "X-Content-Type-Options: nosniff",
    category: Category::Configuration,
    severity: Severity::Low,
    reference: "https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/X-Content-Type-Options",
    remediation: "Send `X-Content-Type-Options: nosniff` so browsers do not guess content types.",
};

pub const FRAMING: CheckDef = CheckDef {
    id: "web.header.framing",
    title: "Protected against clickjacking",
    category: Category::Configuration,
    severity: Severity::Low,
    reference: "https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Content-Security-Policy/frame-ancestors",
    remediation: "Send `Content-Security-Policy: frame-ancestors 'self'` (or the older \
                  `X-Frame-Options: SAMEORIGIN`) so other sites cannot frame your pages.",
};

pub const CSP: CheckDef = CheckDef {
    id: "web.header.csp",
    title: "Content-Security-Policy header",
    category: Category::Configuration,
    severity: Severity::Low,
    reference: "https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/CSP",
    remediation: "Add a Content-Security-Policy, starting in report-only mode, to limit where \
                  scripts and styles may load from.",
};

pub const DEFS: &[&CheckDef] =
    &[&EXPIRY, &TRUSTED, &TLS13, &HTTPS, &HSTS, &NOSNIFF, &FRAMING, &CSP];

/// Par où commencer sur un site web.
pub const WEBSITE_GUIDE: Guide = Guide {
    product: "Website",
    anchor: "websites",
    steps: &[
        "Serve the site over HTTPS only, and redirect plain HTTP to HTTPS.",
        "Automate certificate renewal (ACME, e.g. Let's Encrypt) and serve the full chain, so the certificate never expires or shows a warning.",
        "Add security headers in the web server or reverse proxy: Strict-Transport-Security, X-Content-Type-Options: nosniff, and a Content-Security-Policy with frame-ancestors.",
        "Cross-check with an external scanner such as Mozilla HTTP Observatory, which tests far more than this score.",
    ],
};

/// Métrique des en-têtes, selon le type.
fn header_metric(kind: &str) -> &'static str {
    if kind == "website" { "website_security_header" } else { "probe_http_security_header" }
}

pub struct Website;

impl SecurityProvider for Website {
    fn kinds(&self) -> &'static [&'static str] {
        &["http", "tls", "website"]
    }

    fn metrics(&self) -> &'static [&'static str] {
        &[
            "probe_ssl_cert_expiry_days",
            "probe_ssl_cert_valid",
            "probe_tls_version_info",
            "probe_http_security_header",
            "website_https",
            "website_security_header",
        ]
    }

    /// Un service en clair n'a rien à noter ici.
    fn applies(&self, facts: &Facts) -> bool {
        !facts.is_empty()
    }

    fn guide(&self, _kind: &str, _facts: &Facts) -> Guide {
        WEBSITE_GUIDE
    }

    fn evaluate(&self, kind: &str, facts: &Facts) -> Vec<Check> {
        let mut checks = if kind == "website" {
            // Le type `website` ne lit pas le certificat : seule l'adresse
            // finale dit si le site est en https.
            vec![match facts.max("website_https") {
                None => HTTPS.unknown("Not checked yet."),
                Some(v) => HTTPS.verdict(
                    v >= 1.0,
                    if v >= 1.0 { "Final address is https" } else { "Final address is plain http" },
                ),
            }]
        } else {
            certificate(facts)
        };
        if kind != "tls" {
            checks.extend(headers(facts, header_metric(kind)));
        }
        checks
    }
}

/// Les contrôles d'en-têtes, absents tant qu'aucun n'a été lu.
fn headers(facts: &Facts, metric: &str) -> Vec<Check> {
    if !facts.has(metric) {
        return Vec::new();
    }
    let present =
        |header: &str| facts.all(metric).any(|f| f.label("header") == header && f.value >= 1.0);
    let seen = |def: &CheckDef, ok: bool, header: &str| {
        def.verdict(ok, if ok { format!("{header} sent") } else { format!("No {header}") })
    };
    let framing = {
        let mut sent = Vec::new();
        if present("frame-ancestors") {
            sent.push("CSP frame-ancestors".to_string());
        }
        if present("x-frame-options") {
            sent.push("X-Frame-Options".to_string());
        }
        if sent.is_empty() {
            FRAMING.fail("Neither X-Frame-Options nor CSP frame-ancestors")
        } else {
            FRAMING.pass(list(&sent))
        }
    };
    vec![
        seen(&HSTS, present("strict-transport-security"), "Strict-Transport-Security"),
        seen(&NOSNIFF, present("x-content-type-options"), "X-Content-Type-Options: nosniff"),
        framing,
        seen(&CSP, present("content-security-policy"), "Content-Security-Policy"),
    ]
}

fn certificate(facts: &Facts) -> Vec<Check> {
    let expiry = match facts.min("probe_ssl_cert_expiry_days") {
        None => EXPIRY.unknown("No certificate observed."),
        Some(days) if days <= 0.0 => EXPIRY.fail("The certificate has expired"),
        Some(days) => EXPIRY.verdict(days > 14.0, format!("Expires in {} days", days.floor())),
    };
    let trusted = match facts.min("probe_ssl_cert_valid") {
        None => TRUSTED.unknown("No certificate observed."),
        Some(valid) => TRUSTED.verdict(
            valid >= 1.0,
            if valid >= 1.0 {
                "Chain verified against known authorities"
            } else {
                "Untrusted chain (self-signed, unknown authority or name mismatch)"
            },
        ),
    };
    let version = match facts.all("probe_tls_version_info").map(|f| f.label("version")).next() {
        None => TLS13.unknown("Negotiated version not observed."),
        Some(version) => TLS13.verdict(version == "TLSv1.3", format!("Negotiated {version}")),
    };
    vec![expiry, trusted, version]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::Outcome;

    #[test]
    fn un_certificat_sain() {
        let facts = Facts::from_pairs(&[
            ("probe_ssl_cert_expiry_days", &[], 60.2),
            ("probe_ssl_cert_valid", &[], 1.0),
            ("probe_tls_version_info", &[("version", "TLSv1.3")], 1.0),
        ]);
        let checks = Website.evaluate("tls", &facts);
        assert!(checks.iter().all(|c| c.result == Outcome::Pass), "{checks:?}");
        assert_eq!(checks[0].evidence, "Expires in 60 days");
    }

    #[test]
    fn un_certificat_proche_auto_signe_en_tls_1_2() {
        let facts = Facts::from_pairs(&[
            ("probe_ssl_cert_expiry_days", &[], 5.0),
            ("probe_ssl_cert_valid", &[], 0.0),
            ("probe_tls_version_info", &[("version", "TLSv1.2")], 1.0),
        ]);
        let checks = Website.evaluate("tls", &facts);
        assert!(checks.iter().all(|c| c.result == Outcome::Fail), "{checks:?}");
        assert_eq!(checks[2].evidence, "Negotiated TLSv1.2");
        let expired = Facts::from_pairs(&[("probe_ssl_cert_expiry_days", &[], -3.0)]);
        assert_eq!(Website.evaluate("tls", &expired)[0].evidence, "The certificate has expired");
    }

    #[test]
    fn un_service_en_clair_n_est_pas_concerne() {
        assert!(!Website.applies(&Facts::default()));
        let report = crate::security::build_report(1, "site", "http", &Facts::default());
        assert!(!report.supported);
    }

    fn header(
        name: &'static str,
        value: f64,
    ) -> (&'static str, &'static [(&'static str, &'static str)], f64) {
        let labels: &'static [(&'static str, &'static str)] = match name {
            "strict-transport-security" => &[("header", "strict-transport-security")],
            "x-content-type-options" => &[("header", "x-content-type-options")],
            "content-security-policy" => &[("header", "content-security-policy")],
            "x-frame-options" => &[("header", "x-frame-options")],
            _ => &[("header", "frame-ancestors")],
        };
        ("website_security_header", labels, value)
    }

    #[test]
    fn un_site_en_https_sans_en_tetes() {
        let facts = Facts::from_pairs(&[
            ("website_https", &[], 1.0),
            header("strict-transport-security", 0.0),
            header("x-content-type-options", 1.0),
            header("content-security-policy", 0.0),
            header("x-frame-options", 0.0),
            header("frame-ancestors", 0.0),
        ]);
        let checks = Website.evaluate("website", &facts);
        let ids: Vec<&str> = checks.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "web.https",
                "web.header.hsts",
                "web.header.nosniff",
                "web.header.framing",
                "web.header.csp"
            ]
        );
        let results: Vec<Outcome> = checks.iter().map(|c| c.result).collect();
        assert_eq!(
            results,
            [Outcome::Pass, Outcome::Fail, Outcome::Pass, Outcome::Fail, Outcome::Fail]
        );
    }

    #[test]
    fn frame_ancestors_suffit_contre_le_clickjacking() {
        let facts =
            Facts::from_pairs(&[("website_https", &[], 1.0), header("frame-ancestors", 1.0)]);
        let checks = Website.evaluate("website", &facts);
        let framing = checks.iter().find(|c| c.id == "web.header.framing").unwrap();
        assert_eq!(
            (framing.result, framing.evidence.as_str()),
            (Outcome::Pass, "CSP frame-ancestors")
        );
    }

    #[test]
    fn un_site_en_clair_echoue_sur_https_sans_controle_d_en_tete() {
        let facts = Facts::from_pairs(&[("website_https", &[], 0.0)]);
        let checks = Website.evaluate("website", &facts);
        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0].result, Outcome::Fail);
    }

    #[test]
    fn la_sonde_http_lit_ses_propres_en_tetes() {
        let facts = Facts::from_pairs(&[
            ("probe_ssl_cert_expiry_days", &[], 60.0),
            ("probe_http_security_header", &[("header", "strict-transport-security")], 1.0),
        ]);
        let checks = Website.evaluate("http", &facts);
        assert_eq!(checks.len(), 7);
        let hsts = checks.iter().find(|c| c.id == "web.header.hsts").unwrap();
        assert_eq!(hsts.result, Outcome::Pass);
        // Une sonde `tls` n'a pas d'en-têtes : seulement le certificat.
        assert_eq!(Website.evaluate("tls", &facts).len(), 3);
    }
}
