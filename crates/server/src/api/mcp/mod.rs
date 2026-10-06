//! Serveur MCP (Model Context Protocol) intégré : `POST /api/mcp`.
//!
//! C'est par là qu'un assistant — Claude, ChatGPT, Cursor ou n'importe quel
//! client MCP — interroge l'instance (« tout va bien ? ») et, avec un jeton
//! `write`, agit (poser un silence, relancer une interrogation).
//!
//! Le serveur parle les deux époques du protocole sur le même point d'entrée
//! (« dual-era » dans la spécification) :
//!
//! - **2026-07-28**, sans état : chaque requête porte sa version et les
//!   capacités du client dans `params._meta`, recopiées dans les en-têtes
//!   `MCP-Protocol-Version`, `Mcp-Method` et `Mcp-Name` — que l'on vérifie
//!   contre le corps (400, `HeaderMismatch`, sinon). Pas de poignée de main ;
//!   `server/discover` dit ce que le serveur sait faire ; chaque résultat porte
//!   `resultType` et l'identité du serveur dans `_meta` ;
//! - **2025-11-25, 2025-06-18, 2025-03-26**, avec poignée de main
//!   `initialize`, mais sans session : aucun `Mcp-Session-Id` n'est délivré,
//!   celui qu'un client enverrait est ignoré. Chaque appel porte son jeton.
//!
//! Dans les deux cas : JSON-RPC 2.0 sur `POST`, une requête par corps, réponse
//! en JSON simple (le flux SSE est facultatif et n'apporte rien à des outils qui
//! répondent en une fois) ; `GET` et `DELETE` renvoient 405 ; capacité `tools`
//! seulement, ni ressources, ni invites, ni OAuth — le jeton d'API en
//! `Authorization: Bearer` joue ce rôle. L'en-tête `Origin`, quand il est
//! présent, doit être le nôtre ou admis par la politique CORS (403 sinon), comme
//! l'exige le transport contre le « DNS rebinding ».
//!
//! Le module ne contient aucune logique métier : les outils (voir [`tools`])
//! appellent les mêmes fonctions que l'interface web.

mod tools;

use std::sync::Arc;

use axum::Extension;
use axum::Json;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use base64::Engine;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::auth::cors::CorsPolicy;
use crate::auth::token::{ApiToken, Scope};
use crate::state::AppState;

/// Version de protocole la plus récente que ce serveur parle.
pub const PROTOCOL_VERSION: &str = "2026-07-28";

/// Versions « modernes » : sans état, métadonnées par requête.
const MODERN_VERSIONS: [&str; 1] = ["2026-07-28"];

/// Versions à poignée de main `initialize`, de la plus récente à la plus
/// ancienne. Ce serveur n'en utilise que le sous-ensemble commun (requêtes
/// simples, outils), il n'y a pas de raison de les refuser.
const LEGACY_VERSIONS: [&str; 3] = ["2025-11-25", "2025-06-18", "2025-03-26"];

/// Nom annoncé dans `serverInfo`.
const SERVER_NAME: &str = "DumbMonit";

/// Clés réservées de `_meta`.
mod meta {
    pub const PROTOCOL_VERSION: &str = "io.modelcontextprotocol/protocolVersion";
    pub const CLIENT_CAPABILITIES: &str = "io.modelcontextprotocol/clientCapabilities";
    pub const SERVER_INFO: &str = "io.modelcontextprotocol/serverInfo";
}

/// Durée pendant laquelle un client peut garder la liste des outils : elle ne
/// change qu'avec le binaire.
const LIST_TTL_MS: u64 = 300_000;

/// Codes d'erreur JSON-RPC et MCP.
mod code {
    pub const PARSE_ERROR: i64 = -32700;
    pub const INVALID_REQUEST: i64 = -32600;
    pub const METHOD_NOT_FOUND: i64 = -32601;
    pub const INVALID_PARAMS: i64 = -32602;
    pub const INTERNAL_ERROR: i64 = -32603;
    pub const HEADER_MISMATCH: i64 = -32020;
    pub const UNSUPPORTED_PROTOCOL_VERSION: i64 = -32022;
}

/// Toutes les versions acceptées, la plus récente d'abord.
fn supported_versions() -> Vec<&'static str> {
    MODERN_VERSIONS.iter().chain(LEGACY_VERSIONS.iter()).copied().collect()
}

fn is_supported(version: &str) -> bool {
    MODERN_VERSIONS.contains(&version) || LEGACY_VERSIONS.contains(&version)
}

/// Une requête ou une notification JSON-RPC, telle qu'elle arrive.
#[derive(Debug, Deserialize)]
struct RpcMessage {
    #[serde(default)]
    jsonrpc: Option<String>,
    /// Absent pour une notification. `Value` plutôt qu'un type précis : le
    /// protocole autorise chaînes et entiers, et l'on renvoie ce que l'on a reçu.
    #[serde(default)]
    id: Option<Value>,
    #[serde(default)]
    method: Option<String>,
    #[serde(default)]
    params: Option<Value>,
}

/// Une erreur JSON-RPC avec le statut HTTP qui l'accompagne.
struct Failure {
    status: StatusCode,
    code: i64,
    message: String,
    data: Option<Value>,
}

impl Failure {
    fn new(status: StatusCode, code: i64, message: impl Into<String>) -> Self {
        Self { status, code, message: message.into(), data: None }
    }

    fn unsupported(requested: &str) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: code::UNSUPPORTED_PROTOCOL_VERSION,
            message: format!(
                "Unsupported protocol version \"{requested}\" (supported: {})",
                supported_versions().join(", ")
            ),
            data: Some(json!({ "supported": supported_versions(), "requested": requested })),
        }
    }

    fn header_mismatch(message: String) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code::HEADER_MISMATCH, message)
    }

    fn into_response(self, id: Value) -> Response {
        let mut error = json!({ "code": self.code, "message": self.message });
        if let Some(data) = self.data {
            error["data"] = data;
        }
        (self.status, Json(json!({ "jsonrpc": "2.0", "id": id, "error": error }))).into_response()
    }
}

/// `GET` ou `DELETE /api/mcp` : ce transport ne propose ni flux serveur →
/// client, ni session à fermer.
///
/// La spécification demande un 405 ; le corps explique à l'humain qui a collé
/// l'URL dans un navigateur ce qu'il a trouvé.
pub async fn get() -> Response {
    let body = json!({
        "error": "This endpoint is the DumbMonit MCP server (Model Context Protocol, \
                  Streamable HTTP transport). Send JSON-RPC 2.0 requests with POST and \
                  an API token in the Authorization header.",
        "protocolVersion": PROTOCOL_VERSION,
        "supportedVersions": supported_versions(),
        "transport": "streamable-http",
        "streaming": false,
        "docs": "https://dumbmonit.readthedocs.io/en/latest/using/assistant/"
    });
    let mut response = (StatusCode::METHOD_NOT_ALLOWED, Json(body)).into_response();
    response.headers_mut().insert(header::ALLOW, "POST".parse().unwrap());
    response
}

fn header_str<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|v| v.to_str().ok()).map(str::trim)
}

/// Décode la forme sentinelle `=?base64?…?=` qu'un client emploie pour une
/// valeur qui ne tient pas telle quelle dans un en-tête.
fn decode_header_value(raw: &str) -> Option<String> {
    match raw.strip_prefix("=?base64?").and_then(|rest| rest.strip_suffix("?=")) {
        Some(encoded) => base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok()),
        None => Some(raw.to_string()),
    }
}

/// `POST /api/mcp` : une requête JSON-RPC, une réponse JSON.
pub async fn post(
    State(state): State<AppState>,
    Extension(token): Extension<ApiToken>,
    policy: Option<Extension<Arc<CorsPolicy>>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    // Protection contre le « DNS rebinding » : une page d'une autre origine ne
    // parle à ce serveur que si la politique CORS l'admet.
    if let Some(origin) = header_str(&headers, "origin") {
        let policy =
            policy.map(|Extension(p)| p).unwrap_or_else(|| Arc::new(CorsPolicy::from_env()));
        if !policy.accepts(origin, &headers) {
            tracing::warn!(%origin, token = %token.name, "mcp request from a refused origin");
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": code::INVALID_REQUEST,
                        "message": format!(
                            "Origin \"{origin}\" is not allowed (DUMBMONIT_API_CORS_ORIGINS)."
                        )
                    }
                })),
            )
                .into_response();
        }
    }

    let parsed: Value = match serde_json::from_slice(&body) {
        Ok(value) => value,
        Err(error) => {
            return Failure::new(
                StatusCode::BAD_REQUEST,
                code::PARSE_ERROR,
                format!("Parse error: {error}"),
            )
            .into_response(Value::Null);
        }
    };

    // Les lots ont été retirés du protocole en 2025-06-18 ; on le dit plutôt que
    // de traiter silencieusement le premier élément.
    if parsed.is_array() {
        return Failure::new(
            StatusCode::BAD_REQUEST,
            code::INVALID_REQUEST,
            "JSON-RPC batching is not supported by this server.",
        )
        .into_response(Value::Null);
    }

    let message: RpcMessage = match serde_json::from_value(parsed) {
        Ok(message) => message,
        Err(error) => {
            return Failure::new(
                StatusCode::BAD_REQUEST,
                code::INVALID_REQUEST,
                format!("Invalid request: {error}"),
            )
            .into_response(Value::Null);
        }
    };

    if message.jsonrpc.as_deref() != Some("2.0") {
        return Failure::new(
            StatusCode::BAD_REQUEST,
            code::INVALID_REQUEST,
            "Invalid request: \"jsonrpc\" must be \"2.0\".",
        )
        .into_response(message.id.unwrap_or(Value::Null));
    }

    let Some(method) = message.method else {
        // Une réponse envoyée par le client (à un `ping` du serveur, par exemple) :
        // on n'en émet pas, mais l'accuser de réception ne coûte rien.
        return StatusCode::ACCEPTED.into_response();
    };

    let header_version = header_str(&headers, "mcp-protocol-version");
    let id = match message.id {
        Some(id) => id,
        None => {
            // Notification (`notifications/initialized`, `notifications/cancelled`…) :
            // aucun corps de réponse, 202 comme le prévoit le transport — sauf
            // version inconnue, qu'il faut refuser.
            if let Some(version) = header_version
                && !is_supported(version)
            {
                return Failure::unsupported(version).into_response(Value::Null);
            }
            tracing::debug!(%method, token = %token.name, "mcp notification");
            return StatusCode::ACCEPTED.into_response();
        }
    };

    let params = message.params.unwrap_or(Value::Null);
    let request_meta = params.get("_meta");
    let meta_version = request_meta.and_then(|m| m.get(meta::PROTOCOL_VERSION));

    // Époque de la requête : moderne si elle porte sa version dans `_meta`, ou
    // si l'en-tête annonce une version moderne ; `initialize` est toujours de
    // l'ancienne époque.
    let header_is_modern = header_version.is_some_and(|v| MODERN_VERSIONS.contains(&v));
    let modern = method != "initialize" && (meta_version.is_some() || header_is_modern);

    let outcome = if modern {
        match validate_modern(&headers, &method, &params) {
            Ok(()) => dispatch(&state, &token, &method, params, true).await,
            Err(failure) => Err(failure),
        }
    } else {
        // Ancienne époque : un en-tête absent vaut « version précédente » et
        // l'on accepte ; une version inconnue reçoit 400, comme le veut le
        // transport.
        match header_version {
            Some(version) if !is_supported(version) => Err(Failure::unsupported(version)),
            _ => dispatch(&state, &token, &method, params, false).await,
        }
    };

    match outcome {
        Ok(mut result) => {
            if modern && let Some(object) = result.as_object_mut() {
                object.insert("resultType".into(), json!("complete"));
                let result_meta = object.entry("_meta").or_insert_with(|| json!({}));
                result_meta[meta::SERVER_INFO] = server_info();
            }
            (StatusCode::OK, Json(json!({ "jsonrpc": "2.0", "id": id, "result": result })))
                .into_response()
        }
        Err(failure) => failure.into_response(id),
    }
}

/// Vérifie ce que la révision 2026-07-28 exige d'une requête : sa version dans
/// `_meta` et dans l'en-tête, identiques ; les capacités du client ; et les
/// en-têtes `Mcp-Method` / `Mcp-Name` recopiés du corps.
fn validate_modern(headers: &HeaderMap, method: &str, params: &Value) -> Result<(), Failure> {
    let request_meta = params.get("_meta");
    let Some(version) =
        request_meta.and_then(|m| m.get(meta::PROTOCOL_VERSION)).and_then(Value::as_str)
    else {
        return Err(Failure::new(
            StatusCode::BAD_REQUEST,
            code::INVALID_PARAMS,
            format!("Invalid params: \"_meta\" must carry \"{}\".", meta::PROTOCOL_VERSION),
        ));
    };
    if !is_supported(version) {
        return Err(Failure::unsupported(version));
    }

    match header_str(headers, "mcp-protocol-version") {
        Some(header) if header == version => {}
        Some(header) => {
            return Err(Failure::header_mismatch(format!(
                "Header mismatch: MCP-Protocol-Version header value '{header}' does not match \
                 body value '{version}'"
            )));
        }
        None => {
            return Err(Failure::header_mismatch(
                "Header mismatch: the MCP-Protocol-Version header is required".into(),
            ));
        }
    }

    if request_meta.and_then(|m| m.get(meta::CLIENT_CAPABILITIES)).is_none_or(|c| !c.is_object()) {
        return Err(Failure::new(
            StatusCode::BAD_REQUEST,
            code::INVALID_PARAMS,
            format!("Invalid params: \"_meta\" must carry \"{}\".", meta::CLIENT_CAPABILITIES),
        ));
    }

    match header_str(headers, "mcp-method") {
        Some(header) if header == method => {}
        Some(header) => {
            return Err(Failure::header_mismatch(format!(
                "Header mismatch: Mcp-Method header value '{header}' does not match body value \
                 '{method}'"
            )));
        }
        None => {
            return Err(Failure::header_mismatch(
                "Header mismatch: the Mcp-Method header is required".into(),
            ));
        }
    }

    if method == "tools/call" {
        let body_name = params.get("name").and_then(Value::as_str).unwrap_or_default();
        match header_str(headers, "mcp-name").map(decode_header_value) {
            Some(Some(header)) if header == body_name => {}
            Some(Some(header)) => {
                return Err(Failure::header_mismatch(format!(
                    "Header mismatch: Mcp-Name header value '{header}' does not match body value \
                     '{body_name}'"
                )));
            }
            Some(None) => {
                return Err(Failure::header_mismatch(
                    "Header mismatch: the Mcp-Name header is not valid Base64".into(),
                ));
            }
            None => {
                return Err(Failure::header_mismatch(
                    "Header mismatch: the Mcp-Name header is required for tools/call".into(),
                ));
            }
        }
    }
    Ok(())
}

/// Exécute une méthode. `modern` règle les détails qui diffèrent d'une époque à
/// l'autre : statut d'une méthode inconnue, champs de cache de `tools/list`.
async fn dispatch(
    state: &AppState,
    token: &ApiToken,
    method: &str,
    params: Value,
    modern: bool,
) -> Result<Value, Failure> {
    match method {
        "initialize" => Ok(initialize(&params)),
        "server/discover" => Ok(discover()),
        "ping" => Ok(json!({})),
        "tools/list" => {
            let mut result = json!({ "tools": tools::catalogue() });
            if modern {
                result["ttlMs"] = json!(LIST_TTL_MS);
                result["cacheScope"] = json!("private");
            }
            Ok(result)
        }
        "tools/call" => call_tool(state, token, params).await,
        other => Err(Failure::new(
            // La révision 2026-07-28 veut un 404 pour une méthode inconnue ; les
            // précédentes répondaient en 200, l'erreur étant dans le corps.
            if modern { StatusCode::NOT_FOUND } else { StatusCode::OK },
            code::METHOD_NOT_FOUND,
            format!("Method not found: {other}"),
        )),
    }
}

/// Identité du serveur, telle que `serverInfo` la donne.
fn server_info() -> Value {
    json!({
        "name": SERVER_NAME,
        "title": "DumbMonit monitoring",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Self-hosted monitoring for homelabs and small teams: devices, \
                        services, alerts, maintenance windows and status pages.",
        "websiteUrl": "https://github.com/laupernoe/dumbmonit",
    })
}

fn capabilities() -> Value {
    json!({ "tools": { "listChanged": false } })
}

fn initialize(params: &Value) -> Value {
    // Négociation : si le client demande une version de l'époque `initialize`
    // que l'on connaît, on la lui rend ; sinon on annonce la plus récente de
    // cette époque et il décide.
    let requested = params.get("protocolVersion").and_then(Value::as_str).unwrap_or("");
    let version = if LEGACY_VERSIONS.contains(&requested) { requested } else { LEGACY_VERSIONS[0] };

    json!({
        "protocolVersion": version,
        "capabilities": capabilities(),
        "serverInfo": server_info(),
        "instructions": tools::INSTRUCTIONS,
    })
}

/// `server/discover` : versions, capacités et identité en une requête.
fn discover() -> Value {
    json!({
        "supportedVersions": supported_versions(),
        "capabilities": capabilities(),
        "instructions": tools::INSTRUCTIONS,
        "ttlMs": LIST_TTL_MS,
        "cacheScope": "private",
    })
}

async fn call_tool(state: &AppState, token: &ApiToken, params: Value) -> Result<Value, Failure> {
    let invalid = |message: String| Failure::new(StatusCode::OK, code::INVALID_PARAMS, message);
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return Err(invalid("Invalid params: \"name\" is required.".into()));
    };
    let Some(spec) = tools::find(name) else {
        return Err(invalid(format!("Unknown tool: {name}")));
    };
    let arguments = match params.get("arguments") {
        None | Some(Value::Null) => json!({}),
        Some(Value::Object(map)) => Value::Object(map.clone()),
        Some(_) => return Err(invalid("Invalid params: \"arguments\" must be an object.".into())),
    };

    // Le nom du jeton et l'outil, jamais les arguments : ils peuvent contenir ce
    // que l'utilisateur a dicté à son assistant, identifiants compris.
    let write = spec.scope == Scope::Write;
    tracing::info!(token = %token.name, token_id = token.id, tool = name, write, "mcp tool call");

    // Un jeton `read` qui tente d'écrire n'est pas une erreur de protocole : c'est
    // une réponse que l'assistant doit pouvoir lire et expliquer.
    if write && let Err(message) = token.require(Scope::Write) {
        return Ok(tool_failure(message));
    }

    match tools::call(state, token, name, arguments).await {
        Ok(output) => {
            let mut result = json!({
                "content": [{ "type": "text", "text": output.text }],
                "isError": false,
            });
            if let Some(structured) = output.structured {
                result["structuredContent"] = structured;
            }
            Ok(result)
        }
        Err(tools::ToolError::Failed(message)) => Ok(tool_failure(message)),
        Err(tools::ToolError::Internal(error)) => {
            tracing::error!(?error, tool = name, "mcp tool failed");
            Err(Failure::new(StatusCode::OK, code::INTERNAL_ERROR, "Internal server error."))
        }
    }
}

/// Résultat d'outil en échec : le texte est destiné à l'assistant, qui saura le
/// reformuler pour l'utilisateur.
fn tool_failure(message: String) -> Value {
    json!({ "content": [{ "type": "text", "text": message }], "isError": true })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sentinel_form_of_a_header_is_decoded() {
        assert_eq!(decode_header_value("get_status").as_deref(), Some("get_status"));
        assert_eq!(
            decode_header_value("=?base64?SGVsbG8sIOS4lueVjA==?=").as_deref(),
            Some("Hello, 世界")
        );
        assert_eq!(decode_header_value("=?base64?***?="), None);
    }

    #[test]
    fn versions_are_listed_newest_first() {
        let versions = supported_versions();
        assert_eq!(versions[0], PROTOCOL_VERSION);
        assert!(versions.contains(&"2025-06-18"));
        assert!(!is_supported("1999-01-01"));
    }
}
