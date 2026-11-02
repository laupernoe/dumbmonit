//! HTTPS de l'interface et relevé de ce qui passe en clair (Settings →
//! Security).
//!
//! Administrateur et session seulement (`/security/https` est fermé aux jetons
//! d'API). Le port ne sert que HTTPS : il n'y a plus rien à régler ici, seulement
//! le certificat à montrer et ce qui, ailleurs, passe encore en clair.

use axum::extract::State;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use crate::api::{ApiError, ApiResult};
use crate::auth::middleware::AdminIdentity;
use crate::state::AppState;
use crate::web_tls::{self, CertSummary, audit::Finding};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/security/https", get(overview))
        .route("/security/https/certificate.pem", get(certificate))
}

#[derive(Debug, Serialize)]
pub struct HttpsView {
    /// Un certificat est chargé et servi (toujours, une fois démarré).
    pub available: bool,
    pub certificate: Option<CertSummary>,
    /// Dernière erreur de chargement (certificat fourni illisible…).
    pub error: Option<String>,
    /// Mandataires inverses de confiance déclarés.
    pub trusted_proxies: usize,
    /// Valeur de `Strict-Transport-Security`, envoyée sur toute réponse.
    pub hsts: &'static str,
    /// Ce qui passe encore en clair, ou sans vérification du certificat.
    pub findings: Vec<Finding>,
}

/// `GET /api/security/https`
async fn overview(_: AdminIdentity, State(state): State<AppState>) -> ApiResult<Json<HttpsView>> {
    let web = state.web_tls();
    Ok(Json(HttpsView {
        available: web.server_config().is_some(),
        certificate: web.summary(),
        error: web.error(),
        trusted_proxies: state.config.trusted_proxies.len(),
        hsts: web_tls::HSTS,
        findings: web_tls::audit::collect(&state).await?,
    }))
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
