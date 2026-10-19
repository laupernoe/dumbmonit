//! Réglages de la connexion LDAP / Active Directory, réservés aux
//! administrateurs : lecture, enregistrement, oubli, et test.
//!
//! Le mot de passe du compte de service suit la règle des secrets : il n'est
//! jamais renvoyé (`has_bind_password`), un `PUT` sans lui conserve celui déjà
//! enregistré, et vider le DN de service (recherche anonyme) l'efface.

use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};

use crate::auth::ldap::client::{self, TestReport};
use crate::auth::ldap::{self, GroupMode, LdapConfig};
use crate::auth::middleware::AdminUser;
use crate::auth::oidc::Source;
use crate::auth::{AuthError, AuthResult};
use crate::state::AppState;

/// La configuration telle que l'écran des réglages la montre : sans le mot de
/// passe, avec sa provenance.
#[derive(Debug, Serialize)]
pub struct ConfigView {
    source: Source,
    enabled: bool,
    /// L'environnement décrit-il un annuaire ? Permet de proposer d'y revenir.
    env_available: bool,
    /// `DUMBMONIT_LDAP_ALLOW_PLAIN` : `ldap://` sans StartTLS est admis.
    allow_plain: bool,
    url: String,
    starttls: bool,
    tls_verify: bool,
    ca_cert: String,
    bind_dn: String,
    has_bind_password: bool,
    base_dn: String,
    user_filter: String,
    username_attribute: String,
    display_name_attribute: String,
    email_attribute: String,
    group_mode: GroupMode,
    group_base_dn: String,
    group_filter: String,
    admin_groups: Vec<String>,
    operator_groups: Vec<String>,
    viewer_groups: Vec<String>,
    timeout_seconds: u64,
}

fn view(state: &AppState, resolved: ldap::Resolved) -> ConfigView {
    let config = resolved.config;
    ConfigView {
        source: resolved.source,
        enabled: config.enabled(),
        env_available: state.config.ldap.is_set(),
        allow_plain: state.config.ldap_allow_plain,
        has_bind_password: !config.bind_password.is_empty(),
        url: config.url,
        starttls: config.starttls,
        tls_verify: config.tls_verify,
        ca_cert: config.ca_cert,
        bind_dn: config.bind_dn,
        base_dn: config.base_dn,
        user_filter: config.user_filter,
        username_attribute: config.username_attribute,
        display_name_attribute: config.display_name_attribute,
        email_attribute: config.email_attribute,
        group_mode: config.group_mode,
        group_base_dn: config.group_base_dn,
        group_filter: config.group_filter,
        admin_groups: config.admin_groups,
        operator_groups: config.operator_groups,
        viewer_groups: config.viewer_groups,
        timeout_seconds: config.timeout_seconds,
    }
}

async fn effective(state: &AppState) -> AuthResult<ldap::Resolved> {
    Ok(ldap::resolve(&state.pool, &state.cipher, &state.config.ldap).await?)
}

/// `GET /api/auth/ldap/config`
pub async fn get_config(
    State(state): State<AppState>,
    _: AdminUser,
) -> AuthResult<Json<ConfigView>> {
    let resolved = effective(&state).await?;
    Ok(Json(view(&state, resolved)))
}

/// Corps de `PUT` et du test. Un mot de passe absent ou vide conserve celui
/// déjà enregistré.
#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ConfigPayload {
    url: String,
    starttls: bool,
    /// Absent : vérifié. Couper la vérification doit être un choix.
    tls_verify: Option<bool>,
    ca_cert: String,
    bind_dn: String,
    bind_password: Option<String>,
    base_dn: String,
    user_filter: String,
    username_attribute: String,
    display_name_attribute: String,
    email_attribute: String,
    group_mode: GroupMode,
    group_base_dn: String,
    group_filter: String,
    admin_groups: Vec<String>,
    operator_groups: Vec<String>,
    viewer_groups: Vec<String>,
    timeout_seconds: u64,
}

impl std::fmt::Debug for ConfigPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ConfigPayload {{ url: {:?}, base_dn: {:?}, bind_dn: <redacted>, bind_password: \
             <redacted> }}",
            self.url, self.base_dn
        )
    }
}

impl ConfigPayload {
    fn into_config(self) -> LdapConfig {
        LdapConfig {
            url: self.url,
            starttls: self.starttls,
            tls_verify: self.tls_verify.unwrap_or(true),
            ca_cert: self.ca_cert,
            bind_dn: self.bind_dn,
            bind_password: self.bind_password.unwrap_or_default(),
            base_dn: self.base_dn,
            user_filter: self.user_filter,
            username_attribute: self.username_attribute,
            display_name_attribute: self.display_name_attribute,
            email_attribute: self.email_attribute,
            group_mode: self.group_mode,
            group_base_dn: self.group_base_dn,
            group_filter: self.group_filter,
            admin_groups: self.admin_groups,
            operator_groups: self.operator_groups,
            viewer_groups: self.viewer_groups,
            timeout_seconds: self.timeout_seconds,
        }
        .normalized()
    }
}

/// `PUT /api/auth/ldap/config`
pub async fn put_config(
    State(state): State<AppState>,
    _: AdminUser,
    Json(payload): Json<ConfigPayload>,
) -> AuthResult<Json<ConfigView>> {
    let config = payload.into_config();
    config.validate(state.config.ldap_allow_plain).map_err(AuthError::Invalid)?;
    // Le mot de passe enregistré n'est conservé que pour le même compte sur le
    // même serveur : sinon un autre DN, ou une autre adresse, en hériterait.
    let saved = effective(&state).await?;
    let keeps_secret = saved.source == Source::Settings
        && !saved.config.bind_password.is_empty()
        && same_account(&saved.config, &config);
    if config.bind_password.is_empty() && !keeps_secret {
        if !config.url.is_empty() && !config.bind_dn.is_empty() {
            return Err(AuthError::Invalid(
                "Enter the service account password (or clear the bind DN for anonymous search)."
                    .into(),
            ));
        }
        ldap::clear_secret(&state.pool).await?;
    }
    ldap::save(&state.pool, &state.cipher, config).await?;
    tracing::info!("LDAP settings saved");
    let resolved = effective(&state).await?;
    Ok(Json(view(&state, resolved)))
}

/// Même serveur, même compte de service : le mot de passe enregistré vaut encore.
fn same_account(saved: &LdapConfig, next: &LdapConfig) -> bool {
    saved.url.eq_ignore_ascii_case(&next.url) && saved.bind_dn == next.bind_dn
}

/// `DELETE /api/auth/ldap/config` — oublie le réglage enregistré ;
/// l'environnement reprend la main, ou l'annuaire s'éteint.
pub async fn delete_config(
    State(state): State<AppState>,
    _: AdminUser,
) -> AuthResult<Json<ConfigView>> {
    ldap::clear(&state.pool).await?;
    tracing::info!("LDAP settings cleared");
    let resolved = effective(&state).await?;
    Ok(Json(view(&state, resolved)))
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct TestPayload {
    /// Configuration à tester, telle qu'elle est dans le formulaire ; absente :
    /// la configuration effective.
    config: Option<ConfigPayload>,
    /// Utilisateur à chercher. Son mot de passe n'est jamais demandé.
    username: Option<String>,
}

impl std::fmt::Debug for TestPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TestPayload {{ config: {:?}, username: {:?} }}", self.config, self.username)
    }
}

/// `POST /api/auth/ldap/test` — connexion, liaison du compte de service, et
/// recherche d'un utilisateur de test : un diagnostic étape par étape.
pub async fn test(
    State(state): State<AppState>,
    _: AdminUser,
    payload: Option<Json<TestPayload>>,
) -> AuthResult<Json<TestReport>> {
    let payload = payload.map(|Json(payload)| payload).unwrap_or_default();
    let saved = effective(&state).await?.config;
    let config = match payload.config {
        Some(form) => {
            let mut config = form.into_config();
            // Le mot de passe enregistré ne part que vers le serveur et le
            // compte pour lesquels il a été enregistré : un formulaire modifié
            // ne doit pas pouvoir l'envoyer à une autre adresse.
            if config.bind_password.is_empty() && same_account(&saved, &config) {
                config.bind_password = saved.bind_password.clone();
            }
            config
        }
        None => saved,
    };
    let username = payload.username.as_deref();
    Ok(Json(client::test(&config, state.config.ldap_allow_plain, username).await))
}
