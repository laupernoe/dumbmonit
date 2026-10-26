//! Réception OpenTelemetry (OTLP/HTTP), hors `/api` : c'est l'URL qu'un collecteur
//! OpenTelemetry ou un SDK construit lui-même (`<endpoint>/v1/metrics`).
//!
//! - `POST /api/otlp/v1/metrics` : métriques, en protobuf (`application/x-protobuf`)
//!   ou en JSON, avec ou sans `Content-Encoding: gzip`. Converties par
//!   [`crate::otlp::convert`] et écrites sous l'identité d'une cible.
//! - `POST /api/otlp/v1/logs` : journaux, transmis tels quels à l'ingestion OTLP du
//!   VictoriaLogs embarqué, s'il est activé (503 sinon).
//!
//! Authentification : un jeton d'API de portée `write` (`Authorization: Bearer
//! dmt_…`), jamais la session. La cible visée est désignée par l'en-tête
//! `X-DumbMonit-Target: <id>` (ou `?target=<id>`) : un exportateur OpenTelemetry
//! sait poser des en-têtes, pas toujours des paramètres. Les points écrits portent
//! `target=<id>` quoi que l'émetteur ait mis dans ses attributs.
//!
//! Limites : 8 Mio de corps (32 Mio décompressé), 50 000 échantillons par requête,
//! et le plafond d'appels par minute des jetons d'API.

use std::sync::LazyLock;
use std::time::Duration;

use axum::Router;
use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Query, Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use dumbmonit_proto::TargetId;
use serde::Deserialize;
use serde_json::json;

use crate::auth::token::{self, Scope};
use crate::collectors::otlp as activity;
use crate::db;
use crate::otlp::convert::convert;
use crate::otlp::{ExportMetricsServiceRequest, ExportMetricsServiceResponse, Format, gunzip};
use crate::state::AppState;

/// En-tête qui désigne la cible.
pub const TARGET_HEADER: &str = "x-dumbmonit-target";
/// Taille maximale d'un corps reçu.
const MAX_BODY_BYTES: usize = 8 * 1024 * 1024;
/// Délai de transmission d'un lot de journaux à VictoriaLogs.
const LOGS_TIMEOUT: Duration = Duration::from_secs(20);

static HTTP: LazyLock<reqwest::Client> =
    LazyLock::new(|| reqwest::Client::builder().timeout(LOGS_TIMEOUT).build().unwrap_or_default());

pub fn routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/otlp/v1/metrics", post(metrics))
        .route("/otlp/v1/logs", post(logs))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .route_layer(middleware::from_fn_with_state(state, require_write))
}

/// Exige un jeton `write` valide *avant* de lire le corps.
async fn require_write(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let ip = token::request_ip(request.extensions(), request.headers());
    match token::check(&state.pool, request.headers(), ip, Scope::Write).await {
        Ok(checked) => {
            // Un compte limité par ses équipes n'écrit pas sous l'identité d'une
            // cible quelconque : ses jetons sont refusés ici comme ailleurs.
            let principal = crate::auth::middleware::Principal::Token(checked.token.clone());
            match crate::auth::teams::access_for(&state.pool, &principal).await {
                Ok(access) if access.is_scoped() => {
                    return failure(
                        StatusCode::FORBIDDEN,
                        "OpenTelemetry ingestion is not available to the API tokens of \
                         team-restricted accounts: use a token of an unrestricted account.",
                    );
                }
                Ok(_) => {}
                Err(error) => {
                    tracing::error!(?error, "OTLP: team access lookup failed");
                    return failure(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.");
                }
            }
            let mut response = next.run(request).await;
            checked.rate.apply(response.headers_mut());
            response
        }
        Err(error) => error.into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct TargetQuery {
    target: Option<TargetId>,
}

fn failure(status: StatusCode, message: impl Into<String>) -> Response {
    (status, axum::Json(json!({ "error": message.into() }))).into_response()
}

/// La cible visée, si elle existe.
async fn resolve_target(
    state: &AppState,
    headers: &HeaderMap,
    query: &TargetQuery,
) -> Result<dumbmonit_proto::Target, Box<Response>> {
    let from_header = headers
        .get(TARGET_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<TargetId>().ok());
    let Some(id) = from_header.or(query.target) else {
        return Err(Box::new(failure(
            StatusCode::BAD_REQUEST,
            "Name the target that receives the data: send the header X-DumbMonit-Target with its \
             id (or ?target=<id>). Create an OpenTelemetry source in DumbMonit to get one.",
        )));
    };
    match db::targets::get(&state.pool, &state.cipher, id).await {
        Ok(Some(target)) => Ok(target),
        Ok(None) => {
            Err(Box::new(failure(StatusCode::NOT_FOUND, format!("There is no target {id}."))))
        }
        Err(error) => {
            tracing::error!(?error, "OTLP: target lookup failed");
            Err(Box::new(failure(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error.")))
        }
    }
}

fn header_text(headers: &HeaderMap, name: header::HeaderName) -> Option<&str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

async fn metrics(
    State(state): State<AppState>,
    Query(query): Query<TargetQuery>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let Some(format) = Format::from_content_type(header_text(&headers, header::CONTENT_TYPE))
    else {
        return failure(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Send application/x-protobuf or application/json.",
        );
    };
    let target = match resolve_target(&state, &headers, &query).await {
        Ok(target) => target,
        Err(response) => return *response,
    };
    let raw = match header_text(&headers, header::CONTENT_ENCODING).map(str::trim) {
        None | Some("") | Some("identity") => body.to_vec(),
        Some("gzip") => match gunzip(&body) {
            Ok(raw) => raw,
            Err(message) => return failure(StatusCode::BAD_REQUEST, message),
        },
        Some(other) => {
            return failure(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                format!("Content-Encoding \"{other}\" is not supported: use gzip or none."),
            );
        }
    };
    let request = match ExportMetricsServiceRequest::decode_body(&raw, format) {
        Ok(request) => request,
        Err(message) => return failure(StatusCode::BAD_REQUEST, message),
    };

    let converted = convert(&request, chrono::Utc::now().timestamp_millis());
    let base_labels = target.base_labels();
    let mut samples = converted.samples;
    for sample in &mut samples {
        dumbmonit_proto::apply_identity(&base_labels, &mut sample.labels);
    }
    let written = samples.len();
    state.sink.send(samples).await;
    activity::record(target.id, converted.received, converted.rejected);
    tracing::debug!(target = target.id, written, rejected = converted.rejected, "OTLP metrics");

    let why = "unsupported or out-of-range data points (delta temporality, exponential \
               histograms, timestamps beyond 30 days back or 1 hour ahead)";
    let (content_type, body) = match format {
        Format::Protobuf => (
            "application/x-protobuf",
            ExportMetricsServiceResponse::protobuf(converted.rejected, why),
        ),
        Format::Json => (
            "application/json",
            ExportMetricsServiceResponse::json(converted.rejected, why).to_string().into_bytes(),
        ),
    };
    (StatusCode::OK, [(header::CONTENT_TYPE, HeaderValue::from_static(content_type))], body)
        .into_response()
}

async fn logs(
    State(state): State<AppState>,
    Query(query): Query<TargetQuery>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if Format::from_content_type(header_text(&headers, header::CONTENT_TYPE)).is_none() {
        return failure(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Send application/x-protobuf or application/json.",
        );
    }
    let target = match resolve_target(&state, &headers, &query).await {
        Ok(target) => target,
        Err(response) => return *response,
    };
    let Some(base) = crate::storage::logs_url(&state).await else {
        return failure(
            StatusCode::SERVICE_UNAVAILABLE,
            "Log storage is not enabled: turn on VictoriaLogs in Settings → Storage.",
        );
    };
    let url = format!("{}/insert/opentelemetry/v1/logs", base.trim_end_matches('/'));
    let mut request = HTTP
        .post(url)
        .header(header::CONTENT_TYPE, header_text(&headers, header::CONTENT_TYPE).unwrap_or(""))
        .header("VL-Extra-Fields", format!("target={}", target.id))
        .body(body);
    if let Some(encoding) = header_text(&headers, header::CONTENT_ENCODING) {
        request = request.header(header::CONTENT_ENCODING, encoding);
    }
    match request.send().await {
        Ok(reply) if reply.status().is_success() => {
            let content_type = reply
                .headers()
                .get(header::CONTENT_TYPE)
                .cloned()
                .unwrap_or_else(|| HeaderValue::from_static("application/json"));
            let bytes = reply.bytes().await.unwrap_or_default();
            (StatusCode::OK, [(header::CONTENT_TYPE, content_type)], bytes).into_response()
        }
        Ok(reply) => {
            let status = reply.status();
            tracing::warn!(%status, "OTLP logs: VictoriaLogs refused the batch");
            // Un refus de format est définitif (400) ; le reste, à retenter.
            let mapped = if status.is_client_error() {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::BAD_GATEWAY
            };
            failure(mapped, format!("VictoriaLogs answered {status}."))
        }
        Err(error) => {
            tracing::warn!(%error, "OTLP logs: VictoriaLogs is unreachable");
            failure(StatusCode::BAD_GATEWAY, "VictoriaLogs is unreachable.")
        }
    }
}
