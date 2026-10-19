//! Connexion par un annuaire LDAP ou Active Directory.
//!
//! L'utilisateur tape son identifiant et son mot de passe sur l'écran de
//! connexion habituel. Le serveur se lie à l'annuaire avec un compte de service,
//! cherche l'entrée de l'utilisateur, puis se lie **avec le DN trouvé et le mot
//! de passe saisi** : c'est l'annuaire, et lui seul, qui juge le mot de passe.
//! Les groupes de l'entrée donnent le rôle ; le compte DumbMonit est créé ou
//! mis à jour à chaque connexion (provisionnement juste-à-temps).
//!
//! Les comptes locaux gardent la priorité : un identifiant qui porte un mot de
//! passe local ne va jamais à l'annuaire. L'administrateur local reste ainsi
//! une issue de secours quand l'annuaire est injoignable ou mal configuré.
//!
//! La configuration suit les règles du SSO : l'environnement
//! (`DUMBMONIT_LDAP_*`) ne sert que tant qu'aucun réglage n'est enregistré
//! depuis l'interface, et le mot de passe du compte de service est chiffré avec
//! le secret d'instance, jamais renvoyé.

pub mod client;
pub mod filter;
pub mod login;
pub mod roles;

use std::time::Duration;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::auth::oidc::Source;
use crate::auth::settings;
use crate::crypto::Cipher;

/// Clé du réglage enregistré (JSON en clair) et du mot de passe de service (chiffré).
const SETTINGS_KEY: &str = "ldap";
const SECRET_KEY: &str = "ldap.bind_password";

/// Filtre par défaut : Active Directory, par `sAMAccountName`.
pub const DEFAULT_USER_FILTER: &str = "(&(objectClass=user)(sAMAccountName={username}))";
pub const DEFAULT_USERNAME_ATTRIBUTE: &str = "sAMAccountName";
pub const DEFAULT_DISPLAY_NAME_ATTRIBUTE: &str = "displayName";
pub const DEFAULT_EMAIL_ATTRIBUTE: &str = "mail";
/// Recherche de groupes par défaut : `groupOfNames` d'OpenLDAP.
pub const DEFAULT_GROUP_FILTER: &str = "(&(objectClass=groupOfNames)(member={dn}))";
pub const DEFAULT_TIMEOUT_SECONDS: u64 = 5;
/// Au-delà, une connexion qui ne répond pas ferait attendre l'écran de
/// connexion plus longtemps qu'un humain ne le supporte.
pub const MAX_TIMEOUT_SECONDS: u64 = 30;

/// D'où viennent les groupes de l'utilisateur.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GroupMode {
    /// L'attribut `memberOf` de l'entrée (AD, OpenLDAP avec l'overlay memberof).
    #[default]
    MemberOf,
    /// Une recherche de groupes dont l'utilisateur est membre (`group_filter`).
    Search,
}

impl GroupMode {
    fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "search" => Self::Search,
            _ => Self::MemberOf,
        }
    }
}

/// Configuration LDAP effective, quelle qu'en soit la source.
///
/// `Debug` est écrit à la main : ni le mot de passe ni le DN du compte de
/// service ne doivent pouvoir finir dans un journal.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LdapConfig {
    /// `ldaps://hôte:636` ou `ldap://hôte:389`.
    pub url: String,
    /// StartTLS sur une URL `ldap://`.
    pub starttls: bool,
    /// Vérification du certificat du serveur. Activée par défaut.
    pub tls_verify: bool,
    /// Autorité (PEM) qui a émis le certificat du serveur ; vide : les racines
    /// publiques et celles du système.
    pub ca_cert: String,
    /// Compte de service pour la recherche. Vide : recherche anonyme.
    pub bind_dn: String,
    /// Jamais sérialisé : chiffré à part, voir [`save`].
    #[serde(skip)]
    pub bind_password: String,
    pub base_dn: String,
    /// Filtre de recherche de l'utilisateur, avec `{username}`.
    pub user_filter: String,
    pub username_attribute: String,
    pub display_name_attribute: String,
    pub email_attribute: String,
    pub group_mode: GroupMode,
    /// Base de la recherche de groupes ; vide : `base_dn`.
    pub group_base_dn: String,
    /// Filtre de la recherche de groupes, avec `{dn}` et/ou `{username}`.
    pub group_filter: String,
    pub admin_groups: Vec<String>,
    pub operator_groups: Vec<String>,
    pub viewer_groups: Vec<String>,
    pub timeout_seconds: u64,
}

impl Default for LdapConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            starttls: false,
            tls_verify: true,
            ca_cert: String::new(),
            bind_dn: String::new(),
            bind_password: String::new(),
            base_dn: String::new(),
            user_filter: DEFAULT_USER_FILTER.to_string(),
            username_attribute: DEFAULT_USERNAME_ATTRIBUTE.to_string(),
            display_name_attribute: DEFAULT_DISPLAY_NAME_ATTRIBUTE.to_string(),
            email_attribute: DEFAULT_EMAIL_ATTRIBUTE.to_string(),
            group_mode: GroupMode::MemberOf,
            group_base_dn: String::new(),
            group_filter: DEFAULT_GROUP_FILTER.to_string(),
            admin_groups: Vec::new(),
            operator_groups: Vec::new(),
            viewer_groups: Vec::new(),
            timeout_seconds: DEFAULT_TIMEOUT_SECONDS,
        }
    }
}

impl std::fmt::Debug for LdapConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LdapConfig")
            .field("url", &self.url)
            .field("starttls", &self.starttls)
            .field("tls_verify", &self.tls_verify)
            .field("bind_dn", &"<redacted>")
            .field("bind_password", &"<redacted>")
            .field("base_dn", &self.base_dn)
            .field("user_filter", &self.user_filter)
            .field("group_mode", &self.group_mode)
            .finish_non_exhaustive()
    }
}

impl LdapConfig {
    /// Utilisable dès qu'il y a un serveur et une base de recherche.
    pub fn enabled(&self) -> bool {
        !self.url.is_empty() && !self.base_dn.is_empty()
    }

    /// Une connexion chiffrée : `ldaps://`, ou `ldap://` avec StartTLS.
    pub fn is_encrypted(&self) -> bool {
        self.url.to_ascii_lowercase().starts_with("ldaps://") || self.starttls
    }

    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_seconds.clamp(1, MAX_TIMEOUT_SECONDS))
    }

    /// Nettoie les champs et remet les valeurs par défaut des champs vides.
    pub fn normalized(mut self) -> Self {
        let trim = |value: &mut String| *value = value.trim().to_string();
        self.url = self.url.trim().trim_end_matches('/').to_string();
        trim(&mut self.bind_dn);
        trim(&mut self.base_dn);
        trim(&mut self.group_base_dn);
        trim(&mut self.user_filter);
        trim(&mut self.group_filter);
        trim(&mut self.ca_cert);
        let default = |value: &mut String, fallback: &str| {
            let trimmed = value.trim();
            *value = if trimmed.is_empty() { fallback.to_string() } else { trimmed.to_string() };
        };
        default(&mut self.user_filter, DEFAULT_USER_FILTER);
        default(&mut self.username_attribute, DEFAULT_USERNAME_ATTRIBUTE);
        default(&mut self.display_name_attribute, DEFAULT_DISPLAY_NAME_ATTRIBUTE);
        default(&mut self.email_attribute, DEFAULT_EMAIL_ATTRIBUTE);
        default(&mut self.group_filter, DEFAULT_GROUP_FILTER);
        // StartTLS n'a de sens que sur une connexion en clair.
        if self.url.to_ascii_lowercase().starts_with("ldaps://") {
            self.starttls = false;
        }
        if self.timeout_seconds == 0 {
            self.timeout_seconds = DEFAULT_TIMEOUT_SECONDS;
        }
        self.timeout_seconds = self.timeout_seconds.min(MAX_TIMEOUT_SECONDS);
        self.admin_groups = split_groups(&self.admin_groups);
        self.operator_groups = split_groups(&self.operator_groups);
        self.viewer_groups = split_groups(&self.viewer_groups);
        self
    }

    /// Vérifie ce qui se vérifie sans joindre le serveur. Le message est
    /// destiné à l'administrateur qui remplit le formulaire.
    pub fn validate(&self, allow_plain: bool) -> Result<(), String> {
        if self.url.is_empty() {
            return Ok(());
        }
        let lower = self.url.to_ascii_lowercase();
        if !(lower.starts_with("ldap://") || lower.starts_with("ldaps://")) {
            return Err("The server URL must start with ldaps:// or ldap://.".into());
        }
        if !self.is_encrypted() && !allow_plain {
            return Err("A plain ldap:// connection sends passwords in clear text: use ldaps://, \
                 turn on StartTLS, or set DUMBMONIT_LDAP_ALLOW_PLAIN=1 for a test directory."
                .into());
        }
        if self.base_dn.is_empty() {
            return Err("Enter the base DN to search users in.".into());
        }
        filter::check_template(&self.user_filter, &["{username}"])
            .map_err(|why| format!("User filter: {why}"))?;
        if self.group_mode == GroupMode::Search {
            filter::check_template(&self.group_filter, &["{dn}", "{username}"])
                .map_err(|why| format!("Group filter: {why}"))?;
        }
        if !self.ca_cert.is_empty() {
            dumbmonit_collectors::activedirectory::ldap_client_config(Some(&self.ca_cert), false)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}

/// Une liste de groupes saisie à la main. Les noms de groupes et les DN
/// contiennent des virgules : on sépare par point-virgule ou retour à la ligne,
/// et la virgule n'est un séparateur que pour un élément sans `=` (un nom court).
fn split_groups(groups: &[String]) -> Vec<String> {
    groups
        .iter()
        .flat_map(|group| group.split([';', '\n']))
        .flat_map(|item| if item.contains('=') { vec![item] } else { item.split(',').collect() })
        .map(str::trim)
        .filter(|group| !group.is_empty())
        .map(str::to_string)
        .collect()
}

/// Variables d'environnement lues au démarrage.
#[derive(Debug, Clone, Default)]
pub struct LdapEnv {
    pub config: LdapConfig,
}

impl LdapEnv {
    pub fn from_env() -> Self {
        let var = |key: &str| crate::config::env_var(key).unwrap_or_default();
        let defaults = LdapConfig::default();
        let tls_verify = var("DUMBMONIT_LDAP_TLS_VERIFY");
        let config = LdapConfig {
            url: var("DUMBMONIT_LDAP_URL"),
            starttls: flag(&var("DUMBMONIT_LDAP_STARTTLS")),
            // Seul un « non » explicite coupe la vérification.
            tls_verify: !matches!(
                tls_verify.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "no" | "off"
            ),
            ca_cert: var("DUMBMONIT_LDAP_CA_CERT"),
            bind_dn: var("DUMBMONIT_LDAP_BIND_DN"),
            bind_password: var("DUMBMONIT_LDAP_BIND_PASSWORD"),
            base_dn: var("DUMBMONIT_LDAP_BASE_DN"),
            user_filter: var("DUMBMONIT_LDAP_USER_FILTER"),
            username_attribute: var("DUMBMONIT_LDAP_USERNAME_ATTRIBUTE"),
            display_name_attribute: var("DUMBMONIT_LDAP_DISPLAY_NAME_ATTRIBUTE"),
            email_attribute: var("DUMBMONIT_LDAP_EMAIL_ATTRIBUTE"),
            group_mode: GroupMode::parse(&var("DUMBMONIT_LDAP_GROUP_MODE")),
            group_base_dn: var("DUMBMONIT_LDAP_GROUP_BASE_DN"),
            group_filter: var("DUMBMONIT_LDAP_GROUP_FILTER"),
            admin_groups: vec![var("DUMBMONIT_LDAP_ADMIN_GROUPS")],
            operator_groups: vec![var("DUMBMONIT_LDAP_OPERATOR_GROUPS")],
            viewer_groups: vec![var("DUMBMONIT_LDAP_VIEWER_GROUPS")],
            timeout_seconds: var("DUMBMONIT_LDAP_TIMEOUT_SECONDS")
                .trim()
                .parse()
                .unwrap_or(defaults.timeout_seconds),
        }
        .normalized();
        Self { config }
    }

    /// L'environnement décrit-il un annuaire ?
    pub fn is_set(&self) -> bool {
        !self.config.url.is_empty()
    }
}

fn flag(value: &str) -> bool {
    matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
}

#[derive(Debug, Clone)]
pub struct Resolved {
    pub config: LdapConfig,
    pub source: Source,
}

/// Configuration effective : le réglage enregistré, sinon l'environnement.
pub async fn resolve(pool: &SqlitePool, cipher: &Cipher, env: &LdapEnv) -> Result<Resolved> {
    if let Some(mut saved) = settings::get::<LdapConfig>(pool, SETTINGS_KEY).await? {
        saved.bind_password =
            settings::get_secret(pool, cipher, SECRET_KEY).await?.unwrap_or_default();
        return Ok(Resolved { config: saved.normalized(), source: Source::Settings });
    }
    if env.is_set() {
        return Ok(Resolved { config: env.config.clone(), source: Source::Env });
    }
    Ok(Resolved { config: LdapConfig::default(), source: Source::None })
}

/// Enregistre la configuration. Un mot de passe vide conserve celui déjà
/// stocké ; un DN de service vide (recherche anonyme) l'efface.
pub async fn save(pool: &SqlitePool, cipher: &Cipher, config: LdapConfig) -> Result<()> {
    let config = config.normalized();
    if config.bind_dn.is_empty() {
        settings::delete(pool, SECRET_KEY).await?;
    } else if !config.bind_password.is_empty() {
        settings::set_secret(pool, cipher, SECRET_KEY, &config.bind_password).await?;
    }
    settings::set(pool, SETTINGS_KEY, &config).await
}

/// Efface le mot de passe du compte de service enregistré.
pub async fn clear_secret(pool: &SqlitePool) -> Result<()> {
    settings::delete(pool, SECRET_KEY).await
}

/// Oublie le réglage enregistré : l'environnement reprend la main.
pub async fn clear(pool: &SqlitePool) -> Result<()> {
    settings::delete(pool, SETTINGS_KEY).await?;
    settings::delete(pool, SECRET_KEY).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(url: &str) -> LdapConfig {
        LdapConfig { url: url.into(), base_dn: "dc=example,dc=org".into(), ..LdapConfig::default() }
            .normalized()
    }

    #[test]
    fn defaults_are_active_directory_with_certificate_checks() {
        let config = LdapConfig::default();
        assert!(config.tls_verify, "la vérification du certificat est active par défaut");
        assert_eq!(config.user_filter, DEFAULT_USER_FILTER);
        assert!(!config.enabled());
        // Un réglage enregistré sans le champ garde la vérification.
        let saved: LdapConfig = serde_json::from_str(r#"{"url":"ldaps://dc"}"#).unwrap();
        assert!(saved.tls_verify);
    }

    #[test]
    fn plain_ldap_needs_starttls_or_an_explicit_exception() {
        assert!(config("ldaps://dc.example.org").validate(false).is_ok());
        assert!(config("ldap://dc.example.org").validate(false).is_err());
        assert!(config("ldap://dc.example.org").validate(true).is_ok());
        let mut starttls = config("ldap://dc.example.org");
        starttls.starttls = true;
        assert!(starttls.validate(false).is_ok());
        assert!(config("http://dc.example.org").validate(true).is_err());
        // StartTLS est sans objet sur ldaps://.
        let mut both = config("ldaps://dc.example.org");
        both.starttls = true;
        assert!(!both.normalized().starttls);
    }

    #[test]
    fn the_user_filter_must_carry_the_username() {
        let mut bad = config("ldaps://dc");
        bad.user_filter = "(objectClass=user)".into();
        assert!(bad.validate(false).is_err());
        let mut bad = config("ldaps://dc");
        bad.user_filter = "uid={username}".into();
        assert!(bad.validate(false).is_err(), "un filtre sans parenthèses");
    }

    #[test]
    fn group_lists_keep_distinguished_names_whole() {
        let groups = split_groups(&[
            "cn=Admins,ou=Groups,dc=example,dc=org; ops".into(),
            "noc, on-call".into(),
            "cn=SRE,dc=example,dc=org\nDomain Admins".into(),
        ]);
        assert_eq!(
            groups,
            vec![
                "cn=Admins,ou=Groups,dc=example,dc=org",
                "ops",
                "noc",
                "on-call",
                "cn=SRE,dc=example,dc=org",
                "Domain Admins"
            ]
        );
    }

    #[test]
    fn the_timeout_stays_short() {
        let mut config = config("ldaps://dc");
        config.timeout_seconds = 600;
        assert_eq!(config.normalized().timeout(), Duration::from_secs(MAX_TIMEOUT_SECONDS));
        let config = LdapConfig { timeout_seconds: 0, ..LdapConfig::default() };
        assert_eq!(config.normalized().timeout_seconds, DEFAULT_TIMEOUT_SECONDS);
    }

    #[test]
    fn debug_never_prints_the_service_account() {
        let mut config = config("ldaps://dc");
        config.bind_dn = "cn=svc-secret,dc=example,dc=org".into();
        config.bind_password = "mot-de-passe-de-service".into();
        let rendered = format!("{config:?}");
        assert!(!rendered.contains("svc-secret"), "{rendered}");
        assert!(!rendered.contains("mot-de-passe-de-service"), "{rendered}");
    }
}
