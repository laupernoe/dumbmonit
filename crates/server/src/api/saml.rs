//! Routes de la connexion SAML 2.0 (DumbMonit fournisseur de service).
//!
//! `start`, `acs` et `complete` sont des navigations : chaque échec se termine
//! par une redirection vers l'écran de connexion avec une raison courte, jamais
//! par un JSON que personne ne lirait. Le détail va dans le journal. Les
//! métadonnées sont publiques : l'IdP (ou son administrateur) les lit sans
//! compte. Les réglages sont réservés aux administrateurs.

use axum::Json;
use axum::extract::{Extension, Form, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{AppendHeaders, IntoResponse, Redirect, Response};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use super::oidc::{Origin, is_internal_path};
use crate::auth::client_ip::ClientIp;
use crate::auth::middleware::AdminUser;
use crate::auth::oidc::pkce;
use crate::auth::saml::flow::{self, Correlation, FlowError, Ticket};
use crate::auth::saml::{self, Endpoints, SamlConfig, import};
use crate::auth::{AuthError, AuthResult, AuthState, audit, cookie, oidc, session, users};
use crate::state::AppState;

/// Taille maximale du formulaire posté sur l'ACS. Une réponse d'Entra ID avec
/// quelques centaines de groupes tient en quelques dizaines de Kio ; au-delà,
/// c'est autre chose qu'une connexion.
pub const MAX_ACS_BYTES: usize = 256 * 1024;

/// Cookie qui lie une tentative au navigateur qui l'a lancée (voir
/// [`flow`]) : il porte l'empreinte du `RelayState`, pas le `RelayState`
/// lui-même, ne vit que le temps d'une connexion et n'est envoyé qu'aux routes
/// SAML.
const BINDING_COOKIE: &str = "dumbmonit_saml";
const BINDING_MAX_AGE_SECS: u64 = 600;
const COMPLETE_PATH: &str = "/api/auth/saml/complete";

fn fingerprint(relay_state: &str) -> String {
    hex::encode(Sha256::digest(relay_state.as_bytes()))
}

fn binding_cookie(relay_state: &str, secure: bool) -> HeaderValue {
    let mut value = format!(
        "{BINDING_COOKIE}={}; Path=/api/auth/saml; HttpOnly; SameSite=Lax; Max-Age={BINDING_MAX_AGE_SECS}",
        fingerprint(relay_state)
    );
    if secure {
        value.push_str("; Secure");
    }
    header_value(&value)
}

fn clear_binding_cookie(secure: bool) -> HeaderValue {
    let mut value =
        format!("{BINDING_COOKIE}=; Path=/api/auth/saml; HttpOnly; SameSite=Lax; Max-Age=0");
    if secure {
        value.push_str("; Secure");
    }
    header_value(&value)
}

/// Le navigateur qui revient porte-t-il l'empreinte attendue ?
fn binding_matches(headers: &HeaderMap, expected: &str) -> bool {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .filter(|(name, _)| *name == BINDING_COOKIE)
        .any(|(_, value)| bool::from(value.as_bytes().ct_eq(expected.as_bytes())))
}

fn header_value(value: &str) -> HeaderValue {
    HeaderValue::from_str(value).unwrap_or_else(|_| HeaderValue::from_static("/"))
}

/// URL publique de l'instance : `DUMBMONIT_PUBLIC_URL`, sinon celle des
/// réglages OIDC, sinon l'origine de la requête.
async fn base_url(state: &AppState, origin: &Origin) -> anyhow::Result<String> {
    let env = state.config.oidc.config.public_url.trim();
    if !env.is_empty() {
        return Ok(env.to_string());
    }
    let resolved = oidc::resolve(&state.pool, &state.cipher, &state.config.oidc).await?;
    if !resolved.config.public_url.is_empty() {
        return Ok(resolved.config.public_url);
    }
    Ok(origin.as_str().to_string())
}

async fn context(state: &AppState, origin: &Origin) -> anyhow::Result<(SamlConfig, Endpoints)> {
    let config = saml::load(&state.pool).await?;
    let endpoints = Endpoints::new(&config, &base_url(state, origin).await?);
    Ok((config, endpoints))
}

/// Renvoie à l'écran de connexion avec la raison de l'échec.
fn failure(error: FlowError) -> Response {
    match &error {
        FlowError::Invalid(detail) | FlowError::Expired(detail) => {
            tracing::warn!(reason = error.reason(), %detail, "SAML login refused")
        }
        FlowError::Internal(detail) => {
            tracing::error!(reason = error.reason(), detail = %format!("{detail:#}"), "SAML login failed")
        }
        _ => tracing::warn!(reason = error.reason(), "SAML login failed"),
    }
    Redirect::to(&format!("/login?error=saml&reason={}", error.reason())).into_response()
}

/// `GET /api/auth/saml/metadata` — les métadonnées du fournisseur de service.
pub async fn metadata(State(state): State<AppState>, origin: Origin) -> Response {
    let built = match context(&state, &origin).await {
        Ok((_, endpoints)) => flow::sp_metadata_xml(&endpoints),
        Err(error) => Err(FlowError::Internal(error)),
    };
    match built {
        Ok(xml) => (
            [(header::CONTENT_TYPE, HeaderValue::from_static("application/samlmetadata+xml"))],
            xml,
        )
            .into_response(),
        Err(error) => {
            tracing::error!(reason = error.reason(), ?error, "SAML metadata unavailable");
            (StatusCode::INTERNAL_SERVER_ERROR, "SAML metadata unavailable.").into_response()
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct StartParams {
    /// Page interne à rouvrir après la connexion.
    redirect: Option<String>,
}

/// `GET /api/auth/saml/start` — envoie le navigateur chez l'IdP.
pub async fn start(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthState>,
    origin: Origin,
    Query(params): Query<StartParams>,
) -> Response {
    let (config, endpoints) = match context(&state, &origin).await {
        Ok(found) => found,
        Err(error) => return failure(FlowError::Internal(error)),
    };
    let redirect = params.redirect.filter(|path| is_internal_path(path));
    match flow::start(&config, &endpoints, &state.config.saml_hooks, redirect) {
        Ok((url, relay_state, pending)) => {
            auth.saml().insert_pending(relay_state.clone(), pending);
            (
                StatusCode::SEE_OTHER,
                [
                    (header::SET_COOKIE, binding_cookie(&relay_state, auth.cookie_secure())),
                    (header::LOCATION, header_value(&url)),
                ],
            )
                .into_response()
        }
        Err(error) => failure(error),
    }
}

/// `POST /api/auth/saml/acs` — la réponse de l'IdP (binding HTTP-POST).
///
/// Hors du garde de session et de sa preuve d'origine : c'est l'IdP, d'un autre
/// site, qui fait poster ce formulaire. La signature de l'assertion tient lieu
/// d'authentification, et la session ne s'ouvre qu'à l'étape `complete`.
pub async fn acs(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthState>,
    origin: Origin,
    ClientIp(ip): ClientIp,
    Form(fields): Form<Vec<(String, String)>>,
) -> Response {
    let fields: Vec<(String, String)> = fields
        .into_iter()
        .filter(|(name, _)| name == "SAMLResponse" || name == "RelayState")
        .collect();
    let relay_state =
        fields.iter().find(|(name, _)| name == "RelayState").map(|(_, value)| value.clone());
    let (config, endpoints) = match context(&state, &origin).await {
        Ok(found) => found,
        Err(error) => return failure(FlowError::Internal(error)),
    };
    let pending = relay_state.as_deref().and_then(|relay| auth.saml().take_pending(relay));
    let now = state.config.saml_hooks.now();

    let verified = match (&pending, relay_state.as_deref()) {
        (Some(pending), Some(relay)) => flow::verify(
            &config,
            &endpoints,
            Correlation::Solicited { pending, relay_state: relay },
            fields,
            auth.saml(),
            now,
        )
        .map(|identity| (identity, pending.redirect.clone(), Some(fingerprint(relay)))),
        _ if config.allow_idp_initiated => {
            flow::verify(&config, &endpoints, Correlation::Unsolicited, fields, auth.saml(), now)
                .map(|identity| (identity, None, None))
        }
        // Un `RelayState` inconnu : une tentative expirée, ou déjà utilisée.
        (None, Some(_)) => Err(FlowError::State),
        _ => Err(FlowError::Unsolicited),
    };
    let (identity, redirect, binding) = match verified {
        Ok(found) => found,
        Err(error) => {
            let subject = format!("saml:{}", error.reason());
            audit::record(&state.pool, None, "login.failed", Some(&subject), ip).await;
            return failure(error);
        }
    };
    tracing::info!(name_id = %identity.name_id, issuer = %identity.issuer, "SAML assertion accepted");
    let ticket = pkce::random_token();
    auth.saml().insert_ticket(ticket.clone(), Ticket::new(identity, redirect, binding));
    (
        StatusCode::SEE_OTHER,
        [(header::LOCATION, header_value(&format!("{COMPLETE_PATH}?ticket={ticket}")))],
    )
        .into_response()
}

#[derive(Debug, Deserialize)]
pub struct CompleteParams {
    ticket: Option<String>,
}

/// `GET /api/auth/saml/complete` — ouvre la session, dans le navigateur qui a
/// commencé la connexion.
pub async fn complete(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthState>,
    ClientIp(ip): ClientIp,
    headers: HeaderMap,
    Query(params): Query<CompleteParams>,
) -> Response {
    let Some(ticket) = params.ticket.as_deref().and_then(|id| auth.saml().take_ticket(id)) else {
        return failure(FlowError::State);
    };
    if let Some(expected) = &ticket.binding
        && !binding_matches(&headers, expected)
    {
        return failure(FlowError::State);
    }
    let config = match saml::load(&state.pool).await {
        Ok(config) if config.usable() => config,
        Ok(_) => return failure(FlowError::NotConfigured),
        Err(error) => return failure(FlowError::Internal(error)),
    };
    let user = match flow::map_identity(&state.pool, &config, &ticket.identity).await {
        Ok(user) => user,
        Err(error) => {
            let subject = format!("saml:{}", error.reason());
            audit::record(&state.pool, None, "login.failed", Some(&subject), ip).await;
            return failure(error);
        }
    };
    // Un compte existe désormais : le garde de session ne doit plus tenir
    // l'instance pour vierge.
    auth.remember_configured(true);
    let token = match session::create(&state.pool, user.id).await {
        Ok(token) => token,
        Err(error) => return failure(FlowError::Internal(error)),
    };
    if let Err(error) = users::touch_login(&state.pool, user.id).await {
        return failure(FlowError::Internal(error));
    }
    audit::record(&state.pool, Some(&user.username), "login", Some("saml"), ip).await;
    tracing::info!(user = %user.username, role = user.role.as_str(), "SAML login succeeded");
    let destination = ticket.redirect.unwrap_or_else(|| "/".to_string());
    // `AppendHeaders` : deux `Set-Cookie` dans un même tableau se remplaceraient.
    (
        StatusCode::SEE_OTHER,
        [(header::LOCATION, header_value(&destination))],
        AppendHeaders([
            (header::SET_COOKIE, cookie::set(&token, auth.cookie_secure())),
            (header::SET_COOKIE, clear_binding_cookie(auth.cookie_secure())),
        ]),
    )
        .into_response()
}

// --- Réglages, réservés aux administrateurs ---------------------------------

/// La configuration, avec les adresses à déclarer chez l'IdP.
#[derive(Debug, Serialize)]
pub struct ConfigView {
    #[serde(flatten)]
    config: SamlConfig,
    sp_metadata_url: String,
    acs_url: String,
}

async fn view(state: &AppState, origin: &Origin) -> AuthResult<Json<ConfigView>> {
    let (config, endpoints) = context(state, origin).await?;
    Ok(Json(ConfigView {
        config,
        sp_metadata_url: endpoints.metadata_url,
        acs_url: endpoints.acs_url,
    }))
}

/// `GET /api/auth/saml/config`
pub async fn get_config(
    State(state): State<AppState>,
    _: AdminUser,
    origin: Origin,
) -> AuthResult<Json<ConfigView>> {
    view(&state, &origin).await
}

/// `PUT /api/auth/saml/config` — les champs absents prennent leur valeur par
/// défaut ; `sp_metadata_url` et `acs_url`, calculés, sont ignorés.
pub async fn put_config(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    ClientIp(ip): ClientIp,
    origin: Origin,
    Json(payload): Json<SamlConfig>,
) -> AuthResult<Json<ConfigView>> {
    let config = payload.validated().map_err(AuthError::Invalid)?;
    saml::save(&state.pool, &config).await?;
    audit::record(&state.pool, Some(&admin.username), "settings.saml", None, ip).await;
    tracing::info!(enabled = config.enabled, "SAML settings saved");
    view(&state, &origin).await
}

#[derive(Debug, Deserialize)]
pub struct ImportPayload {
    xml: String,
}

/// `POST /api/auth/saml/metadata/import` — extrait de métadonnées d'IdP de quoi
/// remplir le formulaire, sans rien enregistrer.
pub async fn import_metadata(
    _: AdminUser,
    Json(payload): Json<ImportPayload>,
) -> AuthResult<Json<import::Imported>> {
    import::parse(&payload.xml).map(Json).map_err(AuthError::Invalid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_completion_must_come_back_to_the_browser_that_started() {
        let cookie = binding_cookie("relais-1", false);
        let pair = cookie.to_str().unwrap().split(';').next().unwrap().to_string();
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, format!("autre=1; {pair}").parse().unwrap());
        assert!(binding_matches(&headers, &fingerprint("relais-1")));
        assert!(!binding_matches(&headers, &fingerprint("relais-2")));
        assert!(!binding_matches(&HeaderMap::new(), &fingerprint("relais-1")));
    }
}
