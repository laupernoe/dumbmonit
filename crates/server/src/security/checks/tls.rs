//! Moniteurs TLS : certificat présenté par un service (sondes `tls`,
//! `http`, `smtp`, `mqtt`, `websocket`).
//!
//! La sonde négocie avec rustls, qui refuse tout ce qui est antérieur à
//! TLS 1.2 : un serveur limité à TLS 1.0/1.1 apparaît comme une sonde en
//! échec, pas comme un contrôle. Le contrôle de version ne peut donc dire que
//! « TLS 1.3 négocié ou non ».

use super::super::{Category, Check, CheckDef, Facts, SecurityProvider, Severity};

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

pub const DEFS: &[&CheckDef] = &[&EXPIRY, &TRUSTED, &TLS13];

pub struct TlsMonitor;

impl SecurityProvider for TlsMonitor {
    fn kinds(&self) -> &'static [&'static str] {
        &["tls", "http", "smtp", "mqtt", "websocket"]
    }

    fn metrics(&self) -> &'static [&'static str] {
        &["probe_ssl_cert_expiry_days", "probe_ssl_cert_valid", "probe_tls_version_info"]
    }

    /// Un service en clair n'a rien à noter ici.
    fn applies(&self, facts: &Facts) -> bool {
        !facts.is_empty()
    }

    fn evaluate(&self, _kind: &str, facts: &Facts) -> Vec<Check> {
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
        let checks = TlsMonitor.evaluate("", &facts);
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
        let checks = TlsMonitor.evaluate("", &facts);
        assert!(checks.iter().all(|c| c.result == Outcome::Fail), "{checks:?}");
        assert_eq!(checks[2].evidence, "Negotiated TLSv1.2");
        let expired = Facts::from_pairs(&[("probe_ssl_cert_expiry_days", &[], -3.0)]);
        assert_eq!(TlsMonitor.evaluate("", &expired)[0].evidence, "The certificate has expired");
    }

    #[test]
    fn un_service_en_clair_n_est_pas_concerne() {
        assert!(!TlsMonitor.applies(&Facts::default()));
        let report = crate::security::build_report(1, "site", "http", &Facts::default());
        assert!(!report.supported);
    }
}
