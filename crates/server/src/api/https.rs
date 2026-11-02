//! HTTPS de l'interface et relevé de ce qui passe en clair (Settings →
//! Security).
//!
//! Administrateur et session seulement (`/security/https` est fermé aux jetons
//! d'API) : couper la redirection vers HTTPS est un geste de sécurité, il se
//! fait dans l'interface et entre au journal d'audit.

use axum::extract::{Extension, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::api::{ApiError, ApiResult};
use crate::auth::audit;
use crate::auth::client_ip::ClientIp;
use crate::auth::middleware::AdminIdentity;
use crate::state::AppState;
use crate::web_tls::{self, CertSummary, Transport, audit::Finding};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/security/https", get(overview).put(update))
        .route("/security/https/certificate.pem", get(certificate))
}

#[derive(Debug, Serialize)]
pub struct HttpsView {
    /// `DUMBMONIT_TLS` : le port de l'interface sert aussi HTTPS.
    pub enabled: bool,
    /// Un certificat est chargé et servi.
    pub available: bool,
    pub certificate: Option<CertSummary>,
    /// Dernière erreur de chargement (certificat fourni illisible…).
    pub error: Option<String>,
    /// Les navigations en clair sont renvoyées vers HTTPS.
    pub redirect: bool,
    /// Valeur imposée par `DUMBMONIT_HTTPS_REDIRECT`, qui l'emporte.
    pub redirect_forced: Option<bool>,
    /// Cette requête-ci arrive en HTTPS (direct ou mandataire de confiance).
    pub request_https: bool,
    /// Cette requête-ci est chiffrée jusqu'au serveur.
    pub request_tls: bool,
    /// Mandataires inverses de confiance déclarés.
    pub trusted_proxies: usize,
    /// Valeur de `Strict-Transport-Security` envoyée en HTTPS.
    pub hsts: &'static str,
    /// Ce qui passe encore en clair, ou sans vérification du certificat.
    pub findings: Vec<Finding>,
}

async fn view(state: &AppState, transport: Option<Transport>) -> ApiResult<HttpsView> {
    let web = state.web_tls();
    let transport = transport.unwrap_or_default();
    Ok(HttpsView {
        enabled: state.config.tls_enabled,
        available: web.server_config().is_some(),
        certificate: web.summary(),
        error: web.error(),
        redirect: web.redirect_enabled(),
        redirect_forced: state.config.https_redirect,
        request_https: transport.https,
        request_tls: transport.tls,
        trusted_proxies: state.config.trusted_proxies.len(),
        hsts: web_tls::HSTS,
        findings: web_tls::audit::collect(state).await?,
    })
}

/// `GET /api/security/https`
async fn overview(
    _: AdminIdentity,
    State(state): State<AppState>,
    transport: Option<Extension<Transport>>,
) -> ApiResult<Json<HttpsView>> {
    Ok(Json(view(&state, transport.map(|t| t.0)).await?))
}

#[derive(Debug, Deserialize)]
pub struct UpdatePayload {
    /// Renvoyer les navigations en clair vers HTTPS.
    pub redirect: bool,
}

/// `PUT /api/security/https`
async fn update(
    AdminIdentity(me): AdminIdentity,
    ClientIp(ip): ClientIp,
    State(state): State<AppState>,
    transport: Option<Extension<Transport>>,
    Json(payload): Json<UpdatePayload>,
) -> ApiResult<Json<HttpsView>> {
    if state.config.https_redirect.is_some() {
        return Err(ApiError::Conflict(
            "DUMBMONIT_HTTPS_REDIRECT is set in the environment: change it there.".into(),
        ));
    }
    if payload.redirect && state.web_tls().server_config().is_none() {
        return Err(ApiError::BadRequest(
            "HTTPS is not available on this server (DUMBMONIT_TLS=off, or no certificate): \
             nothing to redirect to."
                .into(),
        ));
    }
    crate::auth::settings::set(&state.pool, web_tls::SETTING_REDIRECT, &payload.redirect).await?;
    state.web_tls().set_redirect(payload.redirect);
    let action =
        if payload.redirect { "security.https_redirect_on" } else { "security.https_redirect_off" };
    audit::record(&state.pool, Some(&me.label()), action, None, ip).await;
    Ok(Json(view(&state, transport.map(|t| t.0)).await?))
}

/// `GET /api/security/https/certificate.pem` : le certificat servi, pour
/// l'ajouter aux autorités de confiance d'un poste ou le comparer.
async fn certificate(_: AdminIdentity, State(state): State<AppState>) -> ApiResult<Response> {
    let summary = state
        .web_tls()
        .summary()
        .ok_or_else(|| ApiError::NotFound("No HTTPS certificate is loaded.".into()))?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/x-pem-file"),
            (header::CONTENT_DISPOSITION, "attachment; filename=\"dumbmonit-https.pem\""),
        ],
        web_tls::cert::to_pem(&summary.der),
    )
        .into_response())
}
