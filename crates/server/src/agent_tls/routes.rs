//! Routes du port des agents.
//!
//! Le protocole des agents (lots, commandes, relais) y est servi tel que sur
//! le port de l'interface, mais derrière un certificat client valable. Seul
//! l'enrôlement s'en passe : c'est lui qui le délivre, contre le jeton
//! d'enrôlement. Rien d'autre n'est servi ici — ni interface, ni version, ni
//! documentation de l'API : un inconnu qui se connecte n'apprend rien.

use std::net::SocketAddr;
use std::time::Instant;

use axum::extract::rejection::JsonRejection;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Extension, Json, Router};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use dumbmonit_proto::{
    AGENT_SECRET_HEADER, AGENT_TLS_HEADER, CERT_REFUSED, TargetId, TlsCertificate, TlsEnrolRequest,
    TlsRenewRequest,
};
use serde_json::json;

use super::{AgentPeer, offer, pki};
use crate::auth::client_ip::ClientIp;
use crate::auth::rate_limit::Key;
use crate::collectors::agent::{self, IngestError};
use crate::state::AppState;

/// Machine authentifiée par son certificat client, déposée dans la requête.
#[derive(Debug, Clone)]
pub struct AgentCert {
    pub target_id: TargetId,
    /// Groupe d'échange de clés de la connexion.
    pub group: Option<String>,
}

/// Routeur complet du port des agents.
pub fn router(state: AppState) -> Router {
    let certified = crate::api::agent_routes(&state)
        .route("/agent/tls/renew", post(renew))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_certificate));
    let open = Router::new().route("/agent/tls/enroll", post(enrol));
    Router::new()
        .nest("/api", certified.merge(open))
        .fallback(not_found)
        .layer(middleware::from_fn(peer_extensions))
        .with_state(state)
}

/// Rend la connexion lisible par les extracteurs ordinaires : l'adresse
/// (`ConnectInfo<SocketAddr>`, pour les compteurs et les journaux) et ce que
/// la poignée de main a établi.
async fn peer_extensions(mut request: Request, next: Next) -> Response {
    if let Some(ConnectInfo(peer)) = request.extensions().get::<ConnectInfo<AgentPeer>>().cloned() {
        request.extensions_mut().insert(ConnectInfo::<SocketAddr>(peer.addr));
        request.extensions_mut().insert(peer);
    }
    next.run(request).await
}

/// Exige un certificat client délivré ici, non échu, et dont la machine
/// existe encore. Le certificat a déjà été vérifié par rustls ; reste à savoir
/// s'il n'a pas été révoqué entre-temps.
async fn require_certificate(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let peer = request.extensions().get::<AgentPeer>().cloned();
    let Some(fingerprint) = peer.as_ref().and_then(|peer| peer.certificate.clone()) else {
        return error(StatusCode::UNAUTHORIZED, CERT_REFUSED);
    };
    match pki::certificate_owner(&state.pool, &fingerprint).await {
        Ok(Some(target_id)) => {
            let group = peer.and_then(|peer| peer.group);
            request.extensions_mut().insert(AgentCert { target_id, group });
            next.run(request).await
        }
        Ok(None) => error(StatusCode::UNAUTHORIZED, CERT_REFUSED),
        Err(cause) => {
            tracing::error!(error = %format!("{cause:#}"), "agent certificate check failed");
            error(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
        }
    }
}

/// `POST /api/agent/tls/enroll` : jeton d'enrôlement (et secret de liaison
/// d'une machine déjà liée) contre un certificat client.
///
/// Les échecs comptent par adresse, comme les connexions à l'interface : un
/// script qui essaie des jetons est bloqué après quelques tentatives.
async fn enrol(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    headers: HeaderMap,
    body: Result<Json<TlsEnrolRequest>, JsonRejection>,
) -> Response {
    let keys: Vec<Key> = ip.map(Key::Ip).into_iter().collect();
    let now = Instant::now();
    if let Err(wait) = state.agent_tls().limits().check(&keys, now) {
        let mut response = error(StatusCode::TOO_MANY_REQUESTS, "Too many attempts, try later.");
        if let Ok(value) = HeaderValue::from_str(&wait.to_string()) {
            response.headers_mut().insert(header::RETRY_AFTER, value);
        }
        return response;
    }
    let fail = |status: StatusCode, message: &str| {
        state.agent_tls().limits().record_failure(&keys, Instant::now());
        error(status, message)
    };
    let Ok(Json(request)) = body else {
        return fail(StatusCode::BAD_REQUEST, "Unreadable enrolment request.");
    };
    let Some(public_key) = decode_key(&request.public_key) else {
        return fail(StatusCode::BAD_REQUEST, "Invalid public key.");
    };

    let bearer = headers.get(header::AUTHORIZATION).and_then(|value| value.to_str().ok());
    let secret = headers.get(AGENT_SECRET_HEADER).and_then(|value| value.to_str().ok());
    let registration =
        match agent::enrol(&state.pool, &state.cipher, bearer, secret, &request.identity).await {
            Ok(registration) => registration,
            Err(IngestError::Unauthorized) => {
                return fail(
                    StatusCode::UNAUTHORIZED,
                    "Enrollment token missing, unknown or revoked.",
                );
            }
            Err(IngestError::Forbidden(message)) => return fail(StatusCode::FORBIDDEN, &message),
            Err(IngestError::BadRequest(message)) => {
                return fail(StatusCode::BAD_REQUEST, &message);
            }
            Err(IngestError::Internal(cause)) => {
                tracing::error!(error = %format!("{cause:#}"), "agent enrolment failed");
                return error(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.");
            }
        };
    state.agent_tls().limits().record_success(&keys);
    tracing::info!(
        target = registration.target_id,
        host = request.identity.hostname,
        "agent enrolled on the encrypted channel"
    );
    issue(&state, registration.target_id, &public_key, registration.issued_secret).await
}

/// `POST /api/agent/tls/renew` : nouvelle clé publique, sous le certificat en
/// cours (le garde l'a déjà vérifié).
async fn renew(
    State(state): State<AppState>,
    Extension(cert): Extension<AgentCert>,
    body: Result<Json<TlsRenewRequest>, JsonRejection>,
) -> Response {
    let Ok(Json(request)) = body else {
        return error(StatusCode::BAD_REQUEST, "Unreadable renewal request.");
    };
    let Some(public_key) = decode_key(&request.public_key) else {
        return error(StatusCode::BAD_REQUEST, "Invalid public key.");
    };
    issue(&state, cert.target_id, &public_key, None).await
}

/// Signe la clé et rend le certificat, avec les empreintes à épingler.
async fn issue(
    state: &AppState,
    target_id: TargetId,
    public_key: &[u8],
    agent_secret: Option<String>,
) -> Response {
    match pki::issue_client(&state.pool, &state.cipher, target_id, public_key).await {
        Ok(issued) => {
            let body = TlsCertificate {
                certificate: BASE64.encode(&issued.der),
                not_after_ms: issued.not_after_ms,
                pins: state.agent_tls().pins(),
                agent_secret,
            };
            let mut response = Json(body).into_response();
            if let Some(offer) = offer(state)
                && let Ok(value) = HeaderValue::from_str(&offer.to_header())
            {
                response.headers_mut().insert(AGENT_TLS_HEADER, value);
            }
            response
        }
        Err(cause) => {
            tracing::error!(error = %format!("{cause:#}"), "agent certificate not issued");
            error(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
        }
    }
}

/// Clé publique en base64, contrôlée : un point P-256 valide, rien d'autre.
fn decode_key(raw: &str) -> Option<Vec<u8>> {
    let key = BASE64.decode(raw.trim()).ok()?;
    pki::check_public_key(&key).ok()?;
    Some(key)
}

async fn not_found() -> Response {
    error(StatusCode::NOT_FOUND, "Not found.")
}

fn error(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({ "error": message }))).into_response()
}
