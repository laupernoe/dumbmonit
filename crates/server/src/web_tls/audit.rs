//! Relevé de ce qui passe encore en clair, ou sans vérification du
//! certificat : l'écran « HTTPS et chiffrement » des réglages en fait la
//! liste, et la bannière de sécurité de l'interface son décompte.
//!
//! Rien n'est bloqué ici : ces réglages ont été choisis (un webhook interne en
//! HTTP, un Proxmox autosigné, un vieux commutateur en SNMP v2c) et le
//! resteront tant que l'administrateur ne les change pas. L'écran dit
//! seulement où, et pourquoi c'est un risque. Chaque constat porte un code
//! stable que l'interface traduit ; `subject` est un nom ou une adresse,
//! jamais un secret.

use std::net::IpAddr;

use anyhow::Result;
use serde::Serialize;
use serde_json::Value;

use crate::state::AppState;

/// Un constat.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Finding {
    /// `web`, `storage`, `notifications`, `monitors`, `logs`, `sign_in`,
    /// `agents`, `backups`.
    pub area: &'static str,
    /// Code stable, traduit par l'interface.
    pub code: &'static str,
    /// `warning` (des données ou des secrets passent en clair) ou `info`.
    pub severity: &'static str,
    /// Nom de l'élément concerné (canal, moniteur, destination…).
    pub subject: String,
    /// Lien vers l'écran où le corriger, s'il y en a un.
    pub link: Option<String>,
}

fn finding(
    area: &'static str,
    code: &'static str,
    severity: &'static str,
    subject: impl Into<String>,
    link: Option<String>,
) -> Finding {
    Finding { area, code, severity, subject: subject.into(), link }
}

/// Vrai pour une URL `http://` qui sort de la machine (la boucle locale ne
/// quitte pas l'hôte, elle n'est pas relevée).
pub fn is_plain_http(url: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(url.trim()) else { return false };
    if url.scheme() != "http" {
        return false;
    }
    let host = url.host_str().unwrap_or("").trim_matches(['[', ']']);
    !(host.eq_ignore_ascii_case("localhost")
        || host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback()))
}

fn flag(value: Option<&String>) -> bool {
    value.is_some_and(|v| truthy(v))
}

fn truthy(text: &str) -> bool {
    matches!(text.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
}

fn json_flag(value: Option<&Value>) -> bool {
    match value {
        Some(Value::Bool(on)) => *on,
        Some(Value::String(text)) => truthy(text),
        _ => false,
    }
}

fn json_text<'a>(settings: &'a serde_json::Map<String, Value>, key: &str) -> &'a str {
    settings.get(key).and_then(Value::as_str).map(str::trim).unwrap_or("")
}

/// Constats d'un moniteur.
pub fn target_findings(target: &dumbmonit_proto::Target) -> Vec<Finding> {
    use dumbmonit_proto::Credential;
    let link = Some(format!("/targets/{}", target.id));
    let warn = |code| finding("monitors", code, "warning", &target.name, link.clone());
    let mut found = Vec::new();
    if flag(target.tags.get("insecure_tls")) {
        found.push(warn("tls_unverified"));
    }
    match &target.credential {
        Credential::SnmpCommunity { .. } => found.push(warn("snmp_community")),
        Credential::SnmpV3 { privacy: None, .. } => found.push(warn("snmp_no_privacy")),
        _ => {}
    }
    let secret = matches!(
        target.credential,
        Credential::ApiToken { .. } | Credential::UsernamePassword { .. }
    );
    let scheme_http =
        target.tags.get("scheme").is_some_and(|s| s.trim().eq_ignore_ascii_case("http"));
    if secret && (scheme_http || is_plain_http(&target.address)) {
        found.push(warn("credentials_over_http"));
    }
    found
}

/// Constats d'un canal de notification.
pub fn channel_findings(channel: &crate::notify::ChannelConfig) -> Vec<Finding> {
    let link = Some("/settings/notifications#notifications-channels".to_string());
    let mut found = Vec::new();
    let Some(settings) = channel.settings.as_object() else { return found };
    if channel.kind == "smtp" {
        let security = json_text(settings, "security").to_ascii_lowercase();
        if matches!(security.as_str(), "none" | "plain") {
            found.push(finding("notifications", "smtp_plain", "warning", &channel.name, link));
        }
        return found;
    }
    let destination = crate::notify::catalog::destination_keys(&channel.kind);
    let plain = settings.iter().any(|(key, value)| {
        (destination.contains(&key.as_str()) || key == "url")
            && value.as_str().is_some_and(is_plain_http)
    });
    if plain {
        found.push(finding("notifications", "channel_http", "warning", &channel.name, link));
    }
    found
}

/// Constats d'une destination de journaux.
pub fn log_destination_findings(destination: &crate::logs::store::Destination) -> Vec<Finding> {
    let link = Some("/settings/system#logs".to_string());
    let settings = &destination.settings;
    let mut found = Vec::new();
    if json_flag(settings.get("insecure_tls")) {
        found.push(finding("logs", "tls_unverified", "warning", &destination.name, link.clone()));
    }
    let transport = json_text(settings, "transport").to_ascii_lowercase();
    let plain = match destination.kind.as_str() {
        "syslog" => transport != "tls",
        "gelf" => match transport.as_str() {
            "tcp" => !json_flag(settings.get("tls")),
            "http" => is_plain_http(json_text(settings, "address")),
            _ => true,
        },
        _ => is_plain_http(json_text(settings, "url")),
    };
    if plain {
        found.push(finding("logs", "log_plain", "warning", &destination.name, link));
    }
    found
}

/// Tout le relevé.
pub async fn collect(state: &AppState) -> Result<Vec<Finding>> {
    let mut found = Vec::new();
    let web = state.web_tls();

    // Interface.
    if web.server_config().is_none() {
        found.push(finding("web", "https_unavailable", "warning", "", None));
    } else {
        if !web.redirect_enabled() {
            found.push(finding("web", "https_redirect_off", "warning", "", None));
        }
        if web.summary().is_some_and(|s| s.source == super::Source::SelfSigned) {
            found.push(finding("web", "self_signed", "info", "", None));
        }
    }

    // Stockage des mesures.
    for url in [&state.config.victoria_url, &state.config.vm_long_url].into_iter().flatten() {
        if is_plain_http(url) {
            let host = reqwest::Url::parse(url)
                .ok()
                .and_then(|u| u.host_str().map(str::to_string))
                .unwrap_or_default();
            found.push(finding("storage", "victoria_http", "warning", host, None));
        }
    }

    // Connexion des utilisateurs.
    let oidc = crate::auth::oidc::resolve(&state.pool, &state.cipher, &state.config.oidc).await?;
    if oidc.config.enabled() && is_plain_http(&oidc.config.issuer) {
        found.push(finding("sign_in", "oidc_http", "warning", "OpenID Connect", None));
    }
    let ldap = crate::auth::ldap::resolve(&state.pool, &state.cipher, &state.config.ldap).await?;
    if ldap.config.enabled() {
        let url = ldap.config.url.trim().to_ascii_lowercase();
        if url.starts_with("ldap://") && !ldap.config.starttls {
            found.push(finding("sign_in", "ldap_plain", "warning", "LDAP", None));
        } else if !ldap.config.tls_verify {
            found.push(finding("sign_in", "ldap_unverified", "warning", "LDAP", None));
        }
    }
    let saml = crate::auth::saml::load(&state.pool).await?;
    if saml.enabled && is_plain_http(&saml.idp_sso_url) {
        found.push(finding("sign_in", "saml_http", "warning", "SAML", None));
    }

    // Moniteurs, canaux, journaux.
    for target in crate::db::targets::list(&state.pool, &state.cipher).await? {
        found.extend(target_findings(&target));
    }
    for channel in crate::db::alerts::list_channels(&state.pool, &state.cipher).await? {
        found.extend(channel_findings(&channel));
    }
    for destination in crate::logs::store::list(&state.pool).await? {
        found.extend(log_destination_findings(&destination));
    }

    // Agents.
    if state.config.agent_listen.is_some() && !crate::agent_tls::mtls_required(state).await? {
        let legacy: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM agent_hosts WHERE legacy_seen_ms IS NOT NULL \
             AND (tls_seen_ms IS NULL OR legacy_seen_ms > tls_seen_ms)",
        )
        .fetch_one(&state.pool)
        .await?;
        let severity = if legacy > 0 { "warning" } else { "info" };
        found.push(finding(
            "agents",
            "agents_legacy_allowed",
            severity,
            legacy.to_string(),
            Some("/settings/monitoring#agents".into()),
        ));
    }
    if crate::collectors::agent::selfupdate::settings(&state.pool).await?.allow_unsigned {
        let link = Some("/settings/monitoring#agents".into());
        found.push(finding("agents", "unsigned_updates", "warning", "", link));
    }

    // Sauvegardes locales : copie de la base et de `secret.key` côte à côte.
    if state.config.backup_enabled && state.config.backup_dir.is_some() {
        found.push(finding(
            "backups",
            "local_backup_with_key",
            "info",
            state.config.backup_path().display().to_string(),
            Some("/settings/system#backup".into()),
        ));
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dumbmonit_proto::{Credential, Target};

    fn target(address: &str, tags: &[(&str, &str)], credential: Credential) -> Target {
        Target {
            id: 7,
            name: "nas".into(),
            address: address.into(),
            kind: "proxmox".into(),
            profile_id: None,
            parent_id: None,
            interval: std::time::Duration::from_secs(60),
            enabled: true,
            tags: tags.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            credential,
            group_name: String::new(),
            position: 0,
        }
    }

    #[test]
    fn the_loopback_is_not_cleartext_on_the_network() {
        assert!(is_plain_http("http://192.168.1.5:8123/api"));
        assert!(is_plain_http("http://ntfy.example.org"));
        assert!(!is_plain_http("https://ntfy.example.org"));
        assert!(!is_plain_http("http://127.0.0.1:9000"));
        assert!(!is_plain_http("http://[::1]:9000"));
        assert!(!is_plain_http("http://localhost"));
        assert!(!is_plain_http("pas une url"));
    }

    #[test]
    fn credentials_sent_over_http_unverified_tls_and_snmp_communities_are_reported() {
        let token = || Credential::ApiToken { token: "x".into() };
        let codes = |t: &Target| target_findings(t).into_iter().map(|f| f.code).collect::<Vec<_>>();
        let http = target("pve.lan", &[("scheme", "http")], token());
        assert_eq!(codes(&http), ["credentials_over_http"]);
        let unverified = target("pve.lan", &[("insecure_tls", "true")], token());
        assert_eq!(codes(&unverified), ["tls_unverified"]);
        assert!(codes(&target("pve.lan", &[], token())).is_empty());
        let community = Credential::SnmpCommunity { community: "public".into() };
        assert_eq!(codes(&target("10.0.0.1", &[], community)), ["snmp_community"]);
        let v3 =
            Credential::SnmpV3 { username: "u".into(), auth: None, privacy: None, context: None };
        assert_eq!(codes(&target("10.0.0.1", &[], v3)), ["snmp_no_privacy"]);
        // Une sonde HTTP sans identifiants n'expose rien.
        assert!(codes(&target("http://site.example", &[], Credential::None)).is_empty());
    }

    #[test]
    fn a_plain_smtp_relay_or_an_http_webhook_is_reported() {
        let channel = |kind: &str, settings: Value| crate::notify::ChannelConfig {
            id: 1,
            name: "ops".into(),
            kind: kind.into(),
            enabled: true,
            settings,
            secrets: Value::Null,
        };
        let codes = |c| channel_findings(&c).into_iter().map(|f| f.code).collect::<Vec<_>>();
        let plain = serde_json::json!({ "host": "mail", "security": "none" });
        assert_eq!(codes(channel("smtp", plain)), ["smtp_plain"]);
        let starttls = serde_json::json!({ "host": "mail", "security": "starttls" });
        assert!(codes(channel("smtp", starttls)).is_empty());
        let http_hook = serde_json::json!({ "url": "http://hooks.example.org/x" });
        assert_eq!(codes(channel("webhook", http_hook)), ["channel_http"]);
        let https_hook = serde_json::json!({ "url": "https://hooks.example.org/x" });
        assert!(codes(channel("webhook", https_hook)).is_empty());
    }

    #[test]
    fn log_destinations_without_tls_are_reported() {
        let destination = |kind: &str, settings: Value| crate::logs::store::Destination {
            id: 1,
            name: "siem".into(),
            kind: kind.into(),
            enabled: true,
            min_level: "info".into(),
            settings: settings.as_object().cloned().unwrap_or_default(),
            has_secret: false,
        };
        let codes =
            |d| log_destination_findings(&d).into_iter().map(|f| f.code).collect::<Vec<_>>();
        use serde_json::json;
        assert_eq!(codes(destination("syslog", json!({ "transport": "udp" }))), ["log_plain"]);
        assert!(codes(destination("syslog", json!({ "transport": "tls" }))).is_empty());
        assert_eq!(codes(destination("gelf", json!({ "transport": "tcp" }))), ["log_plain"]);
        assert!(codes(destination("gelf", json!({ "transport": "tcp", "tls": true }))).is_empty());
        let loki = destination("loki", json!({ "url": "http://loki.lan:3100" }));
        assert_eq!(codes(loki), ["log_plain"]);
        assert_eq!(
            codes(destination("loki", json!({ "url": "https://loki.lan", "insecure_tls": true }))),
            ["tls_unverified"]
        );
    }
}
