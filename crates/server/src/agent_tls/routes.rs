//! Routes du port des agents.
//!
//! Le protocole des agents (lots, commandes, relais) y est servi tel que sur
//! le port de l'interface, mais derrière un certificat client valable. Seul
//! l'enrôlement s'en passe : c'est lui qui le délivre, contre le jeton
//! d'enrôlement. Rien d'autre n'est servi ici — ni interface, ni version, ni
//! documentation de l'API : un inconnu qui se connecte n'apprend rien.
//!
//! Un agent relais y ouvre aussi les tunnels des agents de son site
//! ([`super::tunnel`]) et y relit les fichiers d'installation qu'il leur sert.

use std::net::SocketAddr;
use std::time::Instant;

use axum::body::Body;
use axum::extract::rejection::JsonRejection;
use axum::extract::{ConnectInfo, Path, Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use dumbmonit_proto::{
    AGENT_SECRET_HEADER, AGENT_TLS_HEADER, CERT_REFUSED, RELAY_FILES_PATH, RELAY_TUNNEL_PATH,
    RELAY_TUNNEL_PROTOCOL, TargetId, TlsCertificate, TlsEnrolRequest, TlsRenewRequest,
};
use serde_json::json;

use super::{AgentPeer, offer, pki, tunnel};
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
    /// Agent relais dont le tunnel porte la connexion, s'il y en a un.
    pub via_relay: Option<TargetId>,
}

/// Chemin d'une route sous `/api`, depuis sa constante partagée.
fn under_api(path: &str) -> String {
    path.strip_prefix("/api").unwrap_or(path).to_string()
}

/// Routeur complet du port des agents.
pub fn router(state: AppState) -> Router {
    let files = under_api(RELAY_FILES_PATH);
    let certified = crate::api::agent_routes(&state)
        .route("/agent/tls/renew", post(renew))
        .route(&under_api(RELAY_TUNNEL_PATH), get(open_tunnel))
        .route(&format!("{files}/install.sh"), get(relay_install_sh))
        .route(&format!("{files}/install.ps1"), get(relay_install_ps1))
        .route(&format!("{files}/download/{{name}}"), get(relay_download))
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
            let via_relay = peer.as_ref().and_then(|peer| peer.via_relay);
            if let Some(relay) = via_relay {
                match relay_may_carry(&state, relay, target_id).await {
                    Ok(None) => {}
                    Ok(Some(refusal)) => return error(StatusCode::FORBIDDEN, refusal),
                    Err(cause) => {
                        tracing::error!(error = %format!("{cause:#}"), "agent relay check failed");
                        return error(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.");
                    }
                }
            }
            let group = peer.and_then(|peer| peer.group);
            request.extensions_mut().insert(AgentCert { target_id, group, via_relay });
            next.run(request).await
        }
        Ok(None) => error(StatusCode::UNAUTHORIZED, CERT_REFUSED),
        Err(cause) => {
            tracing::error!(error = %format!("{cause:#}"), "agent certificate check failed");
            error(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")
        }
    }
}

/// Refus d'un agent qui n'appartient pas au client de son relais.
const OTHER_CLIENT: &str = "This relay belongs to another client: this agent cannot connect \
     through it. Install it directly, or through a relay of its own client.";

/// Refus d'un relais qui n'en est pas (ou plus) un.
const NOT_A_RELAY: &str = "This agent is not a relay: switch relay mode on for it in DumbMonit \
     before installing agents through it.";

/// Un relais peut-il porter cette machine ? `None` : oui ; sinon, le refus.
///
/// Le relais doit l'être encore, et la machine appartenir au même client
/// (MSP) : un relais posé chez un client n'est pas une porte vers les machines
/// d'un autre.
async fn relay_may_carry(
    state: &AppState,
    relay: TargetId,
    target_id: TargetId,
) -> anyhow::Result<Option<&'static str>> {
    if relay == target_id {
        return Ok(Some(NOT_A_RELAY));
    }
    if !is_relay(state, relay).await? {
        return Ok(Some(NOT_A_RELAY));
    }
    let relay_client = crate::auth::clients::target_client(&state.pool, relay).await?;
    let machine_client = crate::auth::clients::target_client(&state.pool, target_id).await?;
    Ok((relay_client.is_none() || relay_client != machine_client).then_some(OTHER_CLIENT))
}

/// Vrai si la machine est un relais : déclaré par l'agent, ou demandé depuis
/// l'interface (ou par son jeton).
async fn is_relay(state: &AppState, target_id: TargetId) -> anyhow::Result<bool> {
    Ok(agent::host(&state.pool, target_id)
        .await?
        .is_some_and(|host| host.relay || host.relay_wanted))
}

/// `GET /api/agent/relay/tunnel` (mise à niveau HTTP) : tunnel d'un agent du
/// site de ce relais vers ce port. Réservé à un relais authentifié par son
/// certificat, et jamais depuis un tunnel.
async fn open_tunnel(
    State(state): State<AppState>,
    Extension(cert): Extension<AgentCert>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    mut request: Request,
) -> Response {
    if cert.via_relay.is_some() {
        return error(StatusCode::FORBIDDEN, "A tunnel cannot be opened through another tunnel.");
    }
    let wanted = request
        .headers()
        .get(header::UPGRADE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.eq_ignore_ascii_case(RELAY_TUNNEL_PROTOCOL));
    if !wanted {
        return error(
            StatusCode::UPGRADE_REQUIRED,
            "This route only opens a relay tunnel (Upgrade: dumbmonit-tunnel/1).",
        );
    }
    match is_relay(&state, cert.target_id).await {
        Ok(true) => {}
        Ok(false) => return error(StatusCode::FORBIDDEN, NOT_A_RELAY),
        Err(cause) => {
            tracing::error!(error = %format!("{cause:#}"), "agent relay check failed");
            return error(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.");
        }
    }
    let relay = cert.target_id;
    let grant = match tunnel::grant(&state, relay) {
        Ok(grant) => grant,
        Err(tunnel::Refusal::Busy) => {
            tracing::warn!(relay, "agent relay: tunnel ceiling reached");
            return error(StatusCode::TOO_MANY_REQUESTS, "Too many tunnels for this relay.");
        }
        Err(tunnel::Refusal::Closed) => {
            return error(StatusCode::SERVICE_UNAVAILABLE, "The agent channel is not ready.");
        }
    };
    let upgrade = hyper::upgrade::on(&mut request);
    tokio::spawn(async move {
        match upgrade.await {
            Ok(upgraded) => {
                grant.attach(hyper_util::rt::TokioIo::new(upgraded), relay, addr).await;
            }
            Err(cause) => tracing::debug!(relay, %cause, "agent relay: tunnel upgrade failed"),
        }
    });
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::SWITCHING_PROTOCOLS;
    let headers = response.headers_mut();
    headers.insert(header::CONNECTION, HeaderValue::from_static("upgrade"));
    headers.insert(header::UPGRADE, HeaderValue::from_static(RELAY_TUNNEL_PROTOCOL));
    response
}

/// Garde des fichiers d'installation relus par un relais : un relais, hors
/// tunnel. `None` : servi.
async fn relay_files_refused(state: &AppState, cert: &AgentCert) -> Option<Response> {
    if cert.via_relay.is_some() {
        return Some(error(StatusCode::FORBIDDEN, NOT_A_RELAY));
    }
    match is_relay(state, cert.target_id).await {
        Ok(true) => None,
        Ok(false) => Some(error(StatusCode::FORBIDDEN, NOT_A_RELAY)),
        Err(cause) => {
            tracing::error!(error = %format!("{cause:#}"), "agent relay check failed");
            Some(error(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error."))
        }
    }
}

async fn relay_install_sh(
    State(state): State<AppState>,
    Extension(cert): Extension<AgentCert>,
) -> Response {
    match relay_files_refused(&state, &cert).await {
        Some(refused) => refused,
        None => crate::api::agent_files::install_sh().await,
    }
}

async fn relay_install_ps1(
    State(state): State<AppState>,
    Extension(cert): Extension<AgentCert>,
) -> Response {
    match relay_files_refused(&state, &cert).await {
        Some(refused) => refused,
        None => crate::api::agent_files::install_ps1().await,
    }
}

async fn relay_download(
    State(state): State<AppState>,
    Extension(cert): Extension<AgentCert>,
    name: Path<String>,
) -> Response {
    if let Some(refused) = relay_files_refused(&state, &cert).await {
        return refused;
    }
    crate::api::agent_files::download(State(state), name).await
}

/// `POST /api/agent/tls/enroll` : jeton d'enrôlement (et secret de liaison
/// d'une machine déjà liée) contre un certificat client.
///
/// Les échecs comptent par adresse, comme les connexions à l'interface : un
/// script qui essaie des jetons est bloqué après quelques tentatives.
async fn enrol(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    peer: Option<Extension<AgentPeer>>,
    headers: HeaderMap,
    body: Result<Json<TlsEnrolRequest>, JsonRejection>,
) -> Response {
    let via_relay = peer.and_then(|Extension(peer)| peer.via_relay);
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
    if let Some(relay) = via_relay {
        // Par un relais : le jeton doit être celui d'un client que ce relais
        // sert. Un jeton inconnu est refusé plus bas, comme en direct.
        let checked = async {
            if !is_relay(&state, relay).await? {
                return Ok::<_, anyhow::Error>(Some(NOT_A_RELAY));
            }
            let relay_client = crate::auth::clients::target_client(&state.pool, relay).await?;
            let token_client = agent::token_client_of(&state.pool, bearer).await?;
            Ok(match token_client {
                Some(client) if Some(client) != relay_client => Some(OTHER_CLIENT),
                _ => None,
            })
        };
        match checked.await {
            Ok(None) => {}
            Ok(Some(refusal)) => return fail(StatusCode::FORBIDDEN, refusal),
            Err(cause) => {
                tracing::error!(error = %format!("{cause:#}"), "agent relay check failed");
                return error(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.");
            }
        }
    }
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
    if let Err(cause) = super::note_route(&state.pool, registration.target_id, via_relay).await {
        tracing::warn!(error = %format!("{cause:#}"), "agent relay not recorded");
    }
    tracing::info!(
        target = registration.target_id,
        host = request.identity.hostname,
        via_relay,
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
