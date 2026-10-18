//! Le dialogue avec l'annuaire : connexion, liaison, recherches.
//!
//! Aucune décision de compte n'est prise ici (voir `login`) ; ce module dit
//! seulement qui est l'utilisateur pour l'annuaire, et si son mot de passe est
//! accepté. Chaque opération a un délai court, et l'ensemble d'une connexion un
//! budget global : un annuaire qui ne répond pas ne doit pas figer l'écran de
//! connexion.
//!
//! Ni le mot de passe saisi ni celui du compte de service ne sortent de ce
//! module, pas même dans une erreur ; le DN du compte de service non plus.

use std::time::{Duration, Instant};

use dumbmonit_collectors::activedirectory::{decode::explain_bind_failure, ldap_client_config};
use ldap3::adapters::{Adapter, EntriesOnly};
use ldap3::{Ldap, LdapConnAsync, LdapConnSettings, LdapError, Scope, SearchEntry, SearchOptions};
use serde::Serialize;
use tokio::task::JoinHandle;

use super::filter;
use super::roles::{self, Decision};
use super::{GroupMode, LdapConfig};
use crate::auth::users::{self, Role};

/// Code LDAP `sizeLimitExceeded` : plus de résultats que demandé.
const SIZE_LIMIT_EXCEEDED: u32 = 4;
/// Code LDAP `noSuchObject` : la base de recherche n'existe pas.
const NO_SUCH_OBJECT: u32 = 32;
/// Plafond de la recherche de groupes : au-delà, le reste est ignoré.
const MAX_GROUPS: i32 = 1000;

/// L'utilisateur tel que l'annuaire le décrit.
#[derive(Debug, Clone, Serialize)]
pub struct DirectoryUser {
    pub dn: String,
    /// Identité stable : `guid:…` (AD), `uuid:…` (OpenLDAP), `dn:…` à défaut.
    #[serde(skip)]
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub email: Option<String>,
    /// DN des groupes dont il est membre.
    pub groups: Vec<String>,
}

/// Ce qui empêche de joindre ou d'interroger l'annuaire. Le message est
/// destiné à l'administrateur (journal, bouton « Tester »), jamais à l'écran de
/// connexion.
#[derive(Debug)]
pub enum DirectoryError {
    Config(String),
    Unreachable(String),
    Tls(String),
    Timeout(Duration),
    ServiceBind(String),
    Search(String),
}

impl std::fmt::Display for DirectoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(why) => write!(f, "configuration: {why}"),
            Self::Unreachable(why) => write!(f, "directory unreachable: {why}"),
            Self::Tls(why) => write!(f, "TLS: {why}"),
            Self::Timeout(after) => {
                write!(f, "the directory did not answer within {} s", after.as_secs())
            }
            Self::ServiceBind(why) => write!(f, "service account bind refused: {why}"),
            Self::Search(why) => write!(f, "search failed: {why}"),
        }
    }
}

/// Pourquoi une connexion est refusée alors que l'annuaire a bien répondu.
/// Aucune de ces raisons n'est montrée à l'utilisateur : toutes donnent le
/// même « identifiant ou mot de passe incorrect ».
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejection {
    /// Mot de passe vide : une liaison simple sans mot de passe est une liaison
    /// anonyme (RFC 4513 § 5.1.2), que beaucoup d'annuaires acceptent — ce
    /// serait un faux succès.
    EmptyPassword,
    InvalidUsername,
    NotFound,
    /// Le filtre trouve plusieurs entrées : on ne choisit pas au hasard.
    Ambiguous,
    WrongPassword,
}

impl Rejection {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EmptyPassword => "empty_password",
            Self::InvalidUsername => "invalid_username",
            Self::NotFound => "not_found",
            Self::Ambiguous => "ambiguous",
            Self::WrongPassword => "wrong_password",
        }
    }
}

pub enum Verdict {
    Accepted(DirectoryUser),
    Rejected(Rejection),
}

/// Refus de principe, avant tout appel réseau.
pub fn precheck(username: &str, password: &str) -> Option<Rejection> {
    if password.is_empty() {
        return Some(Rejection::EmptyPassword);
    }
    if users::validate_username(username).is_err() {
        return Some(Rejection::InvalidUsername);
    }
    None
}

/// Vérifie un identifiant et un mot de passe auprès de l'annuaire.
///
/// Déroulé : liaison du compte de service, recherche de l'utilisateur par le
/// filtre (exactement une entrée), recherche de ses groupes, puis liaison avec
/// **le DN trouvé et le mot de passe saisi**.
pub async fn authenticate(
    config: &LdapConfig,
    allow_plain: bool,
    username: &str,
    password: &str,
) -> Result<Verdict, DirectoryError> {
    if let Some(rejection) = precheck(username, password) {
        return Ok(Verdict::Rejected(rejection));
    }
    let budget = config.timeout().saturating_mul(4);
    tokio::time::timeout(budget, async {
        let mut conn = Connection::open(config, allow_plain).await?;
        let verdict = async {
            conn.service_bind(config).await?;
            let mut user = match conn.find_user(config, username).await? {
                Lookup::Found(user) => user,
                Lookup::NotFound => return Ok(Verdict::Rejected(Rejection::NotFound)),
                Lookup::Ambiguous => return Ok(Verdict::Rejected(Rejection::Ambiguous)),
            };
            // Les groupes se lisent avec le compte de service : l'utilisateur
            // n'a pas forcément le droit de parcourir l'annuaire.
            if config.group_mode == GroupMode::Search {
                user.groups = conn.search_groups(config, &user).await?;
            }
            match conn.bind(&user.dn, password).await? {
                Ok(()) => Ok(Verdict::Accepted(user)),
                Err(_) => Ok(Verdict::Rejected(Rejection::WrongPassword)),
            }
        }
        .await;
        conn.close().await;
        verdict
    })
    .await
    .map_err(|_| DirectoryError::Timeout(budget))?
}

/// Une étape du diagnostic.
#[derive(Debug, Serialize)]
pub struct TestStep {
    /// `connect`, `service_bind`, `user_search` ou `groups`.
    pub step: &'static str,
    pub ok: bool,
    pub detail: String,
    pub elapsed_ms: u64,
}

/// L'utilisateur de test, et ce que DumbMonit en ferait.
#[derive(Debug, Serialize)]
pub struct TestUser {
    #[serde(flatten)]
    pub entry: DirectoryUser,
    /// Rôle à la prochaine connexion ; `None` si l'accès serait refusé.
    pub role: Option<Role>,
    pub allowed: bool,
}

#[derive(Debug, Serialize)]
pub struct TestReport {
    pub ok: bool,
    pub steps: Vec<TestStep>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<TestUser>,
}

/// « Tester la connexion » : connexion, liaison du compte de service, et
/// recherche d'un utilisateur de test s'il est donné. Aucun mot de passe
/// d'utilisateur n'est demandé ni essayé.
pub async fn test(
    config: &LdapConfig,
    allow_plain: bool,
    test_username: Option<&str>,
) -> TestReport {
    let mut steps = Vec::new();
    let mut user = None;
    let step = |name, started: Instant, result: Result<String, DirectoryError>| TestStep {
        step: name,
        ok: result.is_ok(),
        detail: result.unwrap_or_else(|error| error.to_string()),
        elapsed_ms: started.elapsed().as_millis() as u64,
    };

    let started = Instant::now();
    let mut conn = match Connection::open(config, allow_plain).await {
        Ok(conn) => {
            let how = if config.url.to_ascii_lowercase().starts_with("ldaps://") {
                "connected over LDAPS"
            } else if config.starttls {
                "connected, StartTLS negotiated"
            } else {
                "connected in clear text (no TLS)"
            };
            steps.push(step("connect", started, Ok(how.to_string())));
            conn
        }
        Err(error) => {
            steps.push(step("connect", started, Err(error)));
            return TestReport { ok: false, steps, user: None };
        }
    };

    let started = Instant::now();
    let bound = conn.service_bind(config).await.map(|()| {
        if config.bind_dn.is_empty() {
            "anonymous search (no service account)".to_string()
        } else {
            "service account accepted".to_string()
        }
    });
    let bound_ok = bound.is_ok();
    steps.push(step("service_bind", started, bound));

    if bound_ok && let Some(username) = test_username.map(str::trim).filter(|u| !u.is_empty()) {
        let started = Instant::now();
        let found = conn.find_user(config, username).await;
        match found {
            Ok(Lookup::Found(mut entry)) => {
                steps.push(step("user_search", started, Ok(format!("found {}", entry.dn))));
                let started = Instant::now();
                let groups = match config.group_mode {
                    GroupMode::MemberOf => Ok(entry.groups.clone()),
                    GroupMode::Search => conn.search_groups(config, &entry).await,
                };
                match groups {
                    Ok(groups) => {
                        entry.groups = groups;
                        let detail = match entry.groups.len() {
                            0 => "member of no group".to_string(),
                            n => format!("member of {n} group(s)"),
                        };
                        steps.push(step("groups", started, Ok(detail)));
                        let (role, allowed) = match roles::decide(
                            &entry.groups,
                            &config.admin_groups,
                            &config.operator_groups,
                            &config.viewer_groups,
                            None,
                        ) {
                            Decision::Role(role) => (Some(role), true),
                            Decision::Keep => (Some(Role::Viewer), true),
                            Decision::Deny => (None, false),
                        };
                        user = Some(TestUser { entry, role, allowed });
                    }
                    Err(error) => steps.push(step("groups", started, Err(error))),
                }
            }
            Ok(Lookup::NotFound) => steps.push(TestStep {
                step: "user_search",
                ok: false,
                detail: format!(
                    "no entry matches the user filter for \"{username}\" under {}",
                    config.base_dn
                ),
                elapsed_ms: started.elapsed().as_millis() as u64,
            }),
            Ok(Lookup::Ambiguous) => steps.push(TestStep {
                step: "user_search",
                ok: false,
                detail: format!(
                    "several entries match \"{username}\": make the user filter more specific"
                ),
                elapsed_ms: started.elapsed().as_millis() as u64,
            }),
            Err(error) => steps.push(step("user_search", started, Err(error))),
        }
    }
    conn.close().await;
    let ok = steps.iter().all(|step| step.ok);
    TestReport { ok, steps, user }
}

enum Lookup {
    Found(DirectoryUser),
    NotFound,
    Ambiguous,
}

struct Connection {
    ldap: Ldap,
    driver: JoinHandle<()>,
    timeout: Duration,
}

impl Connection {
    async fn open(config: &LdapConfig, allow_plain: bool) -> Result<Self, DirectoryError> {
        if !config.enabled() {
            return Err(DirectoryError::Config("no server URL or base DN configured".into()));
        }
        config.validate(allow_plain).map_err(DirectoryError::Config)?;
        let timeout = config.timeout();
        let mut settings = LdapConnSettings::new().set_conn_timeout(timeout);
        if config.is_encrypted() {
            let ca = (!config.ca_cert.is_empty()).then_some(config.ca_cert.as_str());
            let tls = ldap_client_config(ca, !config.tls_verify)
                .map_err(|error| DirectoryError::Config(error.to_string()))?;
            settings = settings
                .set_config(tls)
                .set_starttls(config.starttls)
                .set_no_tls_verify(!config.tls_verify);
        }
        let (conn, ldap) =
            tokio::time::timeout(timeout, LdapConnAsync::with_settings(settings, &config.url))
                .await
                .map_err(|_| DirectoryError::Timeout(timeout))?
                .map_err(|error| connect_error(error, &config.url, timeout))?;
        let driver = tokio::spawn(async move {
            // La connexion s'arrête d'elle-même à la fermeture ; une erreur ici
            // remonte déjà par l'opération en cours.
            let _ = conn.drive().await;
        });
        Ok(Self { ldap, driver, timeout })
    }

    /// Liaison simple. `Err((rc, explication))` quand l'annuaire refuse.
    async fn bind(
        &mut self,
        dn: &str,
        password: &str,
    ) -> Result<Result<(), (u32, String)>, DirectoryError> {
        let result = self
            .ldap
            .with_timeout(self.timeout)
            .simple_bind(dn, password)
            .await
            .map_err(|error| op_error(error, self.timeout))?;
        if result.rc == 0 {
            Ok(Ok(()))
        } else {
            Ok(Err((result.rc, explain_bind_failure(result.rc, &result.text))))
        }
    }

    /// Liaison du compte de service, ou rien pour une recherche anonyme.
    async fn service_bind(&mut self, config: &LdapConfig) -> Result<(), DirectoryError> {
        if config.bind_dn.is_empty() {
            return Ok(());
        }
        // Un DN sans mot de passe serait une liaison « non authentifiée »
        // (RFC 4513 § 5.1.2) : acceptée sans rien prouver par certains annuaires.
        if config.bind_password.is_empty() {
            return Err(DirectoryError::Config(
                "the service account has a DN but no password".into(),
            ));
        }
        match self.bind(&config.bind_dn, &config.bind_password).await? {
            Ok(()) => Ok(()),
            Err((_, why)) => Err(DirectoryError::ServiceBind(why)),
        }
    }

    async fn find_user(
        &mut self,
        config: &LdapConfig,
        username: &str,
    ) -> Result<Lookup, DirectoryError> {
        let filter = filter::render(&config.user_filter, &[("username", username)]);
        let mut attrs = vec![
            "objectGUID".to_string(),
            "entryUUID".to_string(),
            config.username_attribute.clone(),
            config.display_name_attribute.clone(),
            config.email_attribute.clone(),
        ];
        if config.group_mode == GroupMode::MemberOf {
            attrs.push("memberOf".to_string());
        }
        // Deux suffisent pour savoir qu'il y en a plus d'une.
        let (entries, truncated) = self.search(&config.base_dn, &filter, attrs, 2).await?;
        if truncated || entries.len() > 1 {
            return Ok(Lookup::Ambiguous);
        }
        let Some(entry) = entries.into_iter().next() else { return Ok(Lookup::NotFound) };
        if entry.dn.trim().is_empty() {
            // Se lier avec un DN vide reviendrait à une liaison anonyme.
            return Ok(Lookup::NotFound);
        }
        Ok(Lookup::Found(describe(entry, config, username)))
    }

    async fn search_groups(
        &mut self,
        config: &LdapConfig,
        user: &DirectoryUser,
    ) -> Result<Vec<String>, DirectoryError> {
        let base =
            if config.group_base_dn.is_empty() { &config.base_dn } else { &config.group_base_dn };
        let filter = filter::render(
            &config.group_filter,
            &[("dn", user.dn.as_str()), ("username", user.username.as_str())],
        );
        // « 1.1 » : aucun attribut, le DN suffit.
        let (entries, _) = self.search(base, &filter, vec!["1.1".to_string()], MAX_GROUPS).await?;
        Ok(entries.into_iter().map(|entry| entry.dn).collect())
    }

    /// Recherche en sous-arbre, bornée en nombre et en temps. Rend les entrées
    /// et si la limite a été atteinte.
    async fn search(
        &mut self,
        base: &str,
        filter: &str,
        attrs: Vec<String>,
        limit: i32,
    ) -> Result<(Vec<SearchEntry>, bool), DirectoryError> {
        let timeout = self.timeout;
        let seconds = i32::try_from(timeout.as_secs()).unwrap_or(i32::MAX);
        let work = async {
            let adapters: Vec<Box<dyn Adapter<'_, String, Vec<String>>>> =
                vec![Box::new(EntriesOnly::new())];
            let mut stream = self
                .ldap
                .with_search_options(SearchOptions::new().sizelimit(limit).timelimit(seconds))
                .with_timeout(timeout)
                .streaming_search_with(adapters, base, Scope::Subtree, filter, attrs)
                .await?;
            let mut entries = Vec::new();
            while let Some(entry) = stream.next().await? {
                entries.push(SearchEntry::construct(entry));
            }
            Ok::<_, LdapError>((entries, stream.finish().await))
        };
        let budget = timeout.saturating_mul(2);
        let (entries, result) = tokio::time::timeout(budget, work)
            .await
            .map_err(|_| DirectoryError::Timeout(budget))?
            .map_err(|error| op_error(error, timeout))?;
        match result.rc {
            0 => Ok((entries, false)),
            SIZE_LIMIT_EXCEEDED => Ok((entries, true)),
            NO_SUCH_OBJECT => Err(DirectoryError::Search(format!(
                "the base DN {base} does not exist (LDAP result 32)"
            ))),
            rc => Err(DirectoryError::Search(format!(
                "LDAP result {rc} ({})",
                result.text.trim().trim_end_matches('\0')
            ))),
        }
    }

    async fn close(mut self) {
        let _ = tokio::time::timeout(Duration::from_secs(2), self.ldap.unbind()).await;
        self.driver.abort();
    }
}

/// Classe une erreur de connexion : certificat, délai, ou serveur injoignable.
fn connect_error(error: LdapError, url: &str, timeout: Duration) -> DirectoryError {
    let text = error.to_string();
    let lower = text.to_ascii_lowercase();
    if lower.contains("certificate")
        || lower.contains("unknownissuer")
        || matches!(error, LdapError::Rustls { .. })
    {
        return DirectoryError::Tls(format!(
            "certificate of {url} rejected ({text}). Paste the certificate of the authority \
             that issued it as CA certificate."
        ));
    }
    match error {
        LdapError::Timeout { .. } => DirectoryError::Timeout(timeout),
        LdapError::Io { source } if source.kind() == std::io::ErrorKind::TimedOut => {
            DirectoryError::Timeout(timeout)
        }
        LdapError::Io { source } if source.kind() == std::io::ErrorKind::InvalidData => {
            DirectoryError::Tls(format!("TLS failed with {url}: {source}"))
        }
        LdapError::LdapResult { result } => DirectoryError::Tls(format!(
            "the server refused StartTLS (LDAP result {}: {})",
            result.rc,
            result.text.trim()
        )),
        other => DirectoryError::Unreachable(format!("{url}: {other}")),
    }
}

fn op_error(error: LdapError, timeout: Duration) -> DirectoryError {
    match error {
        LdapError::Timeout { .. } => DirectoryError::Timeout(timeout),
        LdapError::Io { source } => DirectoryError::Unreachable(source.to_string()),
        LdapError::LdapResult { result } => DirectoryError::Search(format!(
            "LDAP result {} ({})",
            result.rc,
            result.text.trim().trim_end_matches('\0')
        )),
        other => DirectoryError::Search(other.to_string()),
    }
}

/// Valeur textuelle d'un attribut, sans tenir compte de la casse du nom.
fn text_values<'a>(entry: &'a SearchEntry, name: &str) -> Option<&'a Vec<String>> {
    entry.attrs.iter().find(|(key, _)| key.eq_ignore_ascii_case(name)).map(|(_, values)| values)
}

fn first_text(entry: &SearchEntry, name: &str) -> Option<String> {
    text_values(entry, name)
        .and_then(|values| values.first())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn first_binary<'a>(entry: &'a SearchEntry, name: &str) -> Option<&'a [u8]> {
    entry
        .bin_attrs
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .and_then(|(_, values)| values.first())
        .map(Vec::as_slice)
        .filter(|value| !value.is_empty())
}

/// Identité stable de l'entrée.
fn stable_id(entry: &SearchEntry) -> String {
    // `objectGUID` est binaire ; ldap3 le range parmi les attributs textuels
    // quand ses 16 octets forment par hasard de l'UTF-8 valide.
    if let Some(bytes) = first_binary(entry, "objectGUID") {
        return format!("guid:{}", hex::encode(bytes));
    }
    if let Some(values) = text_values(entry, "objectGUID")
        && let Some(value) = values.first().filter(|value| !value.is_empty())
    {
        return format!("guid:{}", hex::encode(value.as_bytes()));
    }
    if let Some(uuid) = first_text(entry, "entryUUID") {
        return format!("uuid:{}", uuid.to_ascii_lowercase());
    }
    format!("dn:{}", entry.dn.trim().to_lowercase())
}

fn describe(entry: SearchEntry, config: &LdapConfig, typed: &str) -> DirectoryUser {
    let username = first_text(&entry, &config.username_attribute)
        .filter(|name| users::validate_username(name).is_ok())
        .unwrap_or_else(|| typed.trim().to_string());
    let email = first_text(&entry, &config.email_attribute);
    // Les comptes DumbMonit n'ont pas de champ courriel : il sert de nom
    // affiché à défaut d'un vrai.
    let display_name = first_text(&entry, &config.display_name_attribute)
        .or_else(|| email.clone())
        .unwrap_or_default();
    let groups = if config.group_mode == GroupMode::MemberOf {
        text_values(&entry, "memberOf").cloned().unwrap_or_default()
    } else {
        Vec::new()
    };
    DirectoryUser { id: stable_id(&entry), dn: entry.dn, username, display_name, email, groups }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn entry(attrs: &[(&str, &[&str])], bin: &[(&str, &[u8])]) -> SearchEntry {
        SearchEntry {
            dn: "CN=Jane Doe,OU=Staff,DC=corp,DC=example".into(),
            attrs: attrs
                .iter()
                .map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect()))
                .collect::<HashMap<_, _>>(),
            bin_attrs: bin.iter().map(|(k, v)| (k.to_string(), vec![v.to_vec()])).collect(),
        }
    }

    #[test]
    fn an_empty_password_is_refused_before_any_network_call() {
        assert_eq!(precheck("jdoe", ""), Some(Rejection::EmptyPassword));
        assert_eq!(precheck("", "secret"), Some(Rejection::InvalidUsername));
        assert_eq!(precheck("j doe", "secret"), Some(Rejection::InvalidUsername));
        assert_eq!(precheck("jdoe", "secret"), None);
        // Des espaces sont un mot de passe, pas une absence de mot de passe.
        assert_eq!(precheck("jdoe", " "), None);
    }

    #[tokio::test]
    async fn authenticate_refuses_an_empty_password_even_without_a_directory() {
        // Configuration injoignable : si la vérification passait par le réseau,
        // le résultat serait une erreur, pas un refus.
        let config = LdapConfig {
            url: "ldaps://127.0.0.1:1".into(),
            base_dn: "dc=example,dc=org".into(),
            ..LdapConfig::default()
        };
        let verdict = authenticate(&config, false, "jdoe", "").await.expect("pas d'appel réseau");
        assert!(matches!(verdict, Verdict::Rejected(Rejection::EmptyPassword)));
    }

    #[test]
    fn the_stable_id_prefers_the_guid_then_the_uuid_then_the_dn() {
        let ad = entry(&[], &[("objectGUID", &[0xde, 0xad, 0xbe, 0xef])]);
        assert_eq!(stable_id(&ad), "guid:deadbeef");
        let openldap = entry(&[("entryUUID", &["0F1E-ABCD"])], &[]);
        assert_eq!(stable_id(&openldap), "uuid:0f1e-abcd");
        let bare = entry(&[], &[]);
        assert_eq!(stable_id(&bare), "dn:cn=jane doe,ou=staff,dc=corp,dc=example");
    }

    #[test]
    fn the_entry_is_described_with_its_attributes_whatever_their_case() {
        let config = LdapConfig::default();
        let raw = entry(
            &[
                ("samaccountname", &["jdoe"]),
                ("displayname", &["Jane Doe"]),
                ("MAIL", &["jane@corp.example"]),
                ("memberOf", &["CN=Admins,DC=corp", "CN=Staff,DC=corp"]),
            ],
            &[],
        );
        let user = describe(raw, &config, "JDOE");
        assert_eq!(user.username, "jdoe");
        assert_eq!(user.display_name, "Jane Doe");
        assert_eq!(user.email.as_deref(), Some("jane@corp.example"));
        assert_eq!(user.groups.len(), 2);

        // Sans attribut d'identifiant, l'identifiant tapé sert ; sans nom affiché,
        // le courriel.
        let raw = entry(&[("mail", &["jane@corp.example"])], &[]);
        let user = describe(raw, &config, " jdoe ");
        assert_eq!(user.username, "jdoe");
        assert_eq!(user.display_name, "jane@corp.example");
    }

    #[tokio::test]
    async fn an_unreachable_directory_is_an_error_not_a_rejection() {
        let config = LdapConfig {
            url: "ldap://127.0.0.1:1".into(),
            base_dn: "dc=example,dc=org".into(),
            timeout_seconds: 2,
            ..LdapConfig::default()
        };
        let outcome = authenticate(&config, true, "jdoe", "secret").await;
        assert!(outcome.is_err());
        // Et le texte clair est refusé sans exception explicite.
        let outcome = authenticate(&config, false, "jdoe", "secret").await;
        assert!(matches!(outcome, Err(DirectoryError::Config(_))));
    }
}
