//! Nginx et Apache httpd : divulgation de version par l'en-tête `Server`.
//!
//! `stub_status` et `mod_status` ne disent rien de la sécurité du serveur ;
//! la seule chose observable à distance, sans requête supplémentaire, est
//! l'en-tête `Server` que le collecteur capture déjà sur la même requête que
//! les compteurs. Une absence de série (`nginx_version_info` /
//! `apache_version_info`) sans autre donnée est `unknown` ; une absence alors
//! que les compteurs existent veut dire que le serveur ne l'a pas envoyé, ce
//! qui compte comme un succès.

use super::super::{Category, Check, CheckDef, Facts, SecurityProvider, Severity};

pub const NGINX_VERSION_HIDDEN: CheckDef = CheckDef {
    id: "nginx.version_hidden",
    title: "Server header does not disclose the version",
    category: Category::Exposure,
    severity: Severity::Low,
    reference: "https://nginx.org/en/docs/http/ngx_http_core_module.html#server_tokens",
    remediation: "Add \"server_tokens off;\" to the http block and reload Nginx. The header still \
                  names the product, just not the version an attacker could match to a known CVE.",
};

pub const APACHE_VERSION_HIDDEN: CheckDef = CheckDef {
    id: "apache.version_hidden",
    title: "Server header does not disclose the version",
    category: Category::Exposure,
    severity: Severity::Low,
    reference: "https://httpd.apache.org/docs/2.4/mod/core.html#servertokens",
    remediation: "Set \"ServerTokens Prod\" (and \"ServerSignature Off\" for error pages) in the \
                  main configuration and reload Apache.",
};

pub const DEFS: &[&CheckDef] = &[&NGINX_VERSION_HIDDEN, &APACHE_VERSION_HIDDEN];

pub struct Nginx;

impl SecurityProvider for Nginx {
    fn kinds(&self) -> &'static [&'static str] {
        &["nginx"]
    }

    fn metrics(&self) -> &'static [&'static str] {
        &["nginx_connections_active", "nginx_version_info"]
    }

    fn applies(&self, facts: &Facts) -> bool {
        !facts.is_empty()
    }

    fn evaluate(&self, _kind: &str, facts: &Facts) -> Vec<Check> {
        vec![version_check(
            &NGINX_VERSION_HIDDEN,
            facts,
            "nginx_connections_active",
            "nginx_version_info",
            "server",
        )]
    }
}

pub struct Apache;

impl SecurityProvider for Apache {
    fn kinds(&self) -> &'static [&'static str] {
        &["apache"]
    }

    fn metrics(&self) -> &'static [&'static str] {
        &["apache_workers_busy", "apache_version_info"]
    }

    fn applies(&self, facts: &Facts) -> bool {
        !facts.is_empty()
    }

    fn evaluate(&self, _kind: &str, facts: &Facts) -> Vec<Check> {
        vec![version_check(
            &APACHE_VERSION_HIDDEN,
            facts,
            "apache_workers_busy",
            "apache_version_info",
            "version",
        )]
    }
}

/// Un en-tête `Server` qui porte un chiffre est une version ; sans chiffre
/// (« nginx », « Apache »), le produit est connu mais pas la version — c'est
/// exactement ce que `server_tokens off` / `ServerTokens Prod` produisent.
fn version_check(
    def: &CheckDef,
    facts: &Facts,
    seen_metric: &str,
    version_metric: &str,
    label: &str,
) -> Check {
    let header = facts.all(version_metric).next().map(|fact| fact.label(label).to_string());
    match header {
        Some(value) if value.chars().any(|c| c.is_ascii_digit()) => {
            def.fail(format!("Server header discloses a version (\"{value}\")"))
        }
        Some(value) => def.pass(format!("Server header names the product only (\"{value}\")")),
        None if facts.has(seen_metric) => def.pass("No Server header observed"),
        None => def.unknown("No probe observed yet."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::Outcome;

    #[test]
    fn un_en_tete_avec_chiffre_divulgue_la_version() {
        let facts = Facts::from_pairs(&[
            ("nginx_connections_active", &[], 1.0),
            ("nginx_version_info", &[("server", "nginx/1.25.3")], 1.0),
        ]);
        let checks = Nginx.evaluate("nginx", &facts);
        assert_eq!(checks[0].result, Outcome::Fail);
        assert!(checks[0].evidence.contains("1.25.3"));
    }

    #[test]
    fn un_en_tete_sans_chiffre_ne_divulgue_rien() {
        let facts = Facts::from_pairs(&[
            ("nginx_connections_active", &[], 1.0),
            ("nginx_version_info", &[("server", "nginx")], 1.0),
        ]);
        assert_eq!(Nginx.evaluate("nginx", &facts)[0].result, Outcome::Pass);
    }

    #[test]
    fn labsence_den_tete_quand_le_serveur_repond_est_un_succes() {
        let facts = Facts::from_pairs(&[("nginx_connections_active", &[], 1.0)]);
        assert_eq!(Nginx.evaluate("nginx", &facts)[0].result, Outcome::Pass);
    }

    #[test]
    fn sans_aucune_mesure_le_controle_est_inconnu() {
        assert!(!Nginx.applies(&Facts::default()));
        let report = crate::security::build_report(1, "nginx", "nginx", &Facts::default());
        assert!(!report.supported);
    }

    #[test]
    fn apache_suit_la_meme_regle_avec_letiquette_version() {
        let facts = Facts::from_pairs(&[
            ("apache_workers_busy", &[], 1.0),
            ("apache_version_info", &[("version", "Apache/2.4.58 (Unix)")], 1.0),
        ]);
        assert_eq!(Apache.evaluate("apache", &facts)[0].result, Outcome::Fail);
    }
}
