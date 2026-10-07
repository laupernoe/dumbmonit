//! Spécification OpenAPI 3.1 de l'API, servie à `GET /api/openapi.json`.
//!
//! Elle est écrite à la main en YAML (`openapi.yaml`, embarqué dans le binaire)
//! plutôt que générée depuis les gestionnaires : la générer imposerait
//! d'annoter chacun des quelque deux cents gestionnaires et types de l'API, et
//! le YAML se relit comme une documentation. Le prix est le risque de dérive,
//! que le test de ce module ferme : il relit le texte de chaque `.route("…")`
//! déclaré dans `src/api/` et échoue si une route ou une méthode manque à la
//! spécification, ou si la spécification décrit une route qui n'existe pas.
//!
//! La spécification est publique et ne contient aucun secret : rien que la forme
//! de l'API, que le code source publié décrit déjà.
//!
//! Le module porte aussi la version de l'API ([`API_VERSION`]), annoncée dans la
//! spécification et dans l'en-tête `X-DumbMonit-Api-Version` de chaque réponse
//! de `/api` : un client sait ainsi à quel contrat il parle sans dupliquer les
//! routes sous un préfixe `/v1`.

use std::sync::LazyLock;

use axum::Json;
use axum::extract::Request;
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};

/// Version du contrat de l'API. `0.x` tant que le produit est en alpha : elle
/// change dès qu'une route ou un champ est retiré ou renommé ; un ajout ne la
/// change pas.
pub const API_VERSION: &str = "0.1";

/// En-tête qui annonce [`API_VERSION`] sur chaque réponse de `/api`.
pub const VERSION_HEADER: &str = "x-dumbmonit-api-version";

/// Le texte de la spécification, tel qu'il est maintenu.
const SPEC_YAML: &str = include_str!("openapi.yaml");

/// La spécification en JSON, complétée des versions, construite une fois.
static SPEC: LazyLock<Result<Value, String>> = LazyLock::new(|| build(SPEC_YAML));

fn build(yaml: &str) -> Result<Value, String> {
    let mut spec: Value = serde_yaml_ng::from_str(yaml).map_err(|error| error.to_string())?;
    let info = spec
        .get_mut("info")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "the specification has no info object".to_string())?;
    info.insert("version".into(), json!(API_VERSION));
    info.insert("x-server-version".into(), json!(env!("CARGO_PKG_VERSION")));
    Ok(spec)
}

/// `GET /api/openapi.json` — publique.
pub async fn spec() -> Response {
    match &*SPEC {
        Ok(spec) => {
            let mut response = Json(spec.clone()).into_response();
            // Elle ne change qu'avec le binaire : une heure de cache suffit, et
            // un générateur de clients qui la relit en boucle ne coûte rien.
            response
                .headers_mut()
                .insert(header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=3600"));
            response
        }
        Err(error) => {
            tracing::error!(%error, "the embedded OpenAPI specification does not parse");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "The API specification is unavailable." })),
            )
                .into_response()
        }
    }
}

/// Pose `X-DumbMonit-Api-Version` sur toute réponse de `/api`, erreurs comprises.
pub async fn version_header(request: Request, next: Next) -> Response {
    let api = request.uri().path().starts_with("/api/");
    let mut response = next.run(request).await;
    if api {
        response.headers_mut().insert(VERSION_HEADER, HeaderValue::from_static(API_VERSION));
    }
    response
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet, HashSet};
    use std::path::Path;

    use super::*;
    use crate::auth::middleware::{TOKEN_DENIED, is_operator_route};

    /// Routes que la spécification ne décrit volontairement pas :
    ///
    /// - le protocole des agents (`/api/ingest`, `/api/agent/commands*`,
    ///   `/api/agent/relay*`) : un échange entre le binaire de l'agent et le
    ///   serveur, authentifié par un jeton d'enregistrement, que nul client n'a
    ///   à appeler ;
    /// - les redirections de la connexion OIDC (`/api/auth/oidc/start` et
    ///   `/callback`), et le retour de Spotify (`/api/music/spotify/callback`) :
    ///   des pages qu'un navigateur traverse, pas une API ;
    /// - la distribution de l'agent (`/install.sh`, `/install.ps1`,
    ///   `/download/{name}`) : des fichiers, pas du JSON.
    const EXCLUDED: &[&str] = &[
        "/api/ingest",
        "/api/agent/commands",
        "/api/agent/commands/{id}",
        "/api/agent/relay",
        "/api/agent/relay/{id}",
        "/api/auth/oidc/start",
        "/api/auth/oidc/callback",
        "/api/music/spotify/callback",
        "/install.sh",
        "/install.ps1",
        "/download/{name}",
    ];

    /// Fichiers dont les routes sont montées hors de `/api`.
    const OUTSIDE_FILES: &[&str] = &["prometheus.rs"];
    /// Routes montées hors de `/api` par `mod.rs`.
    const OUTSIDE_PATHS: &[&str] = &["/install.sh", "/install.ps1", "/download/{name}"];

    /// Routes publiques, celles que le garde de session ne couvre pas.
    fn is_public(path: &str, method: &str) -> bool {
        matches!(
            path,
            "/api/health"
                | "/api/auth/status"
                | "/api/auth/setup"
                | "/api/auth/login"
                | "/api/auth/login/totp"
                | "/api/push/{token}"
                | "/api/openapi.json"
        ) || path.starts_with("/api/public/")
            || (path == "/api/mcp" && method != "post")
    }

    /// Routes fermées à tout jeton : celles du garde, plus les sauvegardes et la
    /// connexion à Spotify et son jeton pour le mur (refusées par leur
    /// gestionnaire, qui exige une session).
    fn is_session_only(path: &str) -> bool {
        let Some(rest) = path.strip_prefix("/api") else { return false };
        const BY_HANDLER: &[&str] = &[
            "/backup",
            "/music/spotify/authorize",
            "/music/spotify/complete",
            "/music/spotify/token",
            "/music/speaker/report",
            "/music/speaker/play",
        ];
        TOKEN_DENIED.iter().chain(BY_HANDLER.iter()).any(|prefix| {
            rest == *prefix || rest.strip_prefix(prefix).is_some_and(|r| r.starts_with('/'))
        })
    }

    /// Relit les `.route("…", …)` de tous les fichiers de `src/api`.
    fn declared_routes() -> BTreeMap<String, BTreeSet<String>> {
        let mut routes = BTreeMap::new();
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/api");
        visit(&dir, &mut routes);
        assert!(routes.len() > 50, "lecture des routes suspecte : {}", routes.len());
        routes
    }

    fn visit(dir: &Path, routes: &mut BTreeMap<String, BTreeSet<String>>) {
        for entry in std::fs::read_dir(dir).expect("dossier src/api") {
            let path = entry.expect("entrée").path();
            if path.is_dir() {
                visit(&path, routes);
                continue;
            }
            if path.extension().is_none_or(|ext| ext != "rs") {
                continue;
            }
            let file = path.file_name().unwrap().to_string_lossy().to_string();
            // Ce fichier-ci ne déclare aucune route ; ses commentaires et ses
            // tests en citent pour l'exemple.
            if file == "openapi.rs" {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("fichier source");
            for (route, methods) in routes_in(&text) {
                let route = route.replace("{*", "{");
                let outside = OUTSIDE_FILES.contains(&file.as_str())
                    || OUTSIDE_PATHS.contains(&route.as_str());
                let full = if outside { route } else { format!("/api{route}") };
                routes.entry(full).or_default().extend(methods);
            }
        }
    }

    /// Les appels `.route(` d'un texte : le chemin (premier littéral) et les
    /// méthodes (`get(`, `post(`, … qui ne sont pas la fin d'un autre nom).
    fn routes_in(text: &str) -> Vec<(String, BTreeSet<String>)> {
        let mut out = Vec::new();
        let mut rest = text;
        while let Some(start) = rest.find(".route(") {
            let body = &rest[start + ".route(".len()..];
            let mut depth = 1;
            let mut end = body.len();
            for (i, c) in body.char_indices() {
                match c {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end = i;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let call = &body[..end];
            rest = &body[end..];
            let Some(open) = call.find('"') else { continue };
            let Some(close) = call[open + 1..].find('"') else { continue };
            let path = call[open + 1..open + 1 + close].to_string();
            let after = &call[open + 1 + close + 1..];
            let mut methods = BTreeSet::new();
            for method in ["get", "post", "put", "delete", "patch"] {
                let needle = format!("{method}(");
                let mut from = 0;
                while let Some(at) = after[from..].find(&needle) {
                    let at = from + at;
                    let before = after[..at].chars().next_back();
                    if before.is_none_or(|c| !(c.is_alphanumeric() || c == '_' || c == ':')) {
                        methods.insert(method.to_string());
                    }
                    from = at + needle.len();
                }
            }
            out.push((path, methods));
        }
        out
    }

    fn spec_value() -> Value {
        SPEC.as_ref().expect("la spécification se lit").clone()
    }

    const METHODS: [&str; 5] = ["get", "post", "put", "delete", "patch"];

    #[test]
    fn the_specification_is_openapi_3_1_with_our_versions() {
        let spec = spec_value();
        assert!(
            spec["openapi"].as_str().is_some_and(|v| v.starts_with("3.1")),
            "{}",
            spec["openapi"]
        );
        assert_eq!(spec["info"]["version"], API_VERSION);
        assert_eq!(spec["info"]["x-server-version"], env!("CARGO_PKG_VERSION"));
        assert!(spec["components"]["securitySchemes"]["bearerAuth"].is_object());
        assert_eq!(spec["components"]["securitySchemes"]["bearerAuth"]["scheme"], "bearer");
        assert!(spec["components"]["schemas"]["Error"].is_object());
    }

    #[test]
    fn every_route_is_in_the_specification_and_nothing_else() {
        let spec = spec_value();
        let paths = spec["paths"].as_object().expect("paths");
        let declared = declared_routes();
        let mut problems = Vec::new();

        for (path, methods) in &declared {
            if EXCLUDED.contains(&path.as_str()) {
                continue;
            }
            let Some(item) = paths.get(path) else {
                problems.push(format!("route absente de la spécification : {path}"));
                continue;
            };
            for method in methods {
                if item.get(method).is_none() {
                    problems.push(format!("méthode absente : {} {path}", method.to_uppercase()));
                }
            }
            for method in METHODS {
                if item.get(method).is_some() && !methods.contains(method) {
                    problems.push(format!(
                        "méthode décrite mais non routée : {} {path}",
                        method.to_uppercase()
                    ));
                }
            }
        }
        for path in paths.keys() {
            if !declared.contains_key(path) {
                problems.push(format!("chemin décrit mais non routé : {path}"));
            }
        }
        for excluded in EXCLUDED {
            assert!(declared.contains_key(*excluded), "exclusion périmée : {excluded}");
        }
        assert!(
            problems.is_empty(),
            "dérive entre le routeur et openapi.yaml :\n{}",
            problems.join("\n")
        );
    }

    /// `x-roles: [admin, operator]` est posé exactement sur les écritures que le
    /// garde ouvre aux opérateurs (`OPERATOR_ROUTES`) : la documentation ne
    /// promet rien de plus, et n'oublie rien.
    #[test]
    fn the_operator_operations_match_the_guard() {
        let spec = spec_value();
        let mut problems = Vec::new();
        for (path, item) in spec["paths"].as_object().expect("paths") {
            // Un paramètre de chemin devient un segment quelconque, non vide.
            let concrete: String = path
                .split('/')
                .map(|segment| if segment.starts_with('{') { "x" } else { segment })
                .collect::<Vec<_>>()
                .join("/");
            for method in METHODS {
                let Some(op) = item.get(method) else { continue };
                let roles: Vec<&str> = op["x-roles"]
                    .as_array()
                    .map(|roles| roles.iter().filter_map(Value::as_str).collect())
                    .unwrap_or_default();
                let documented = roles.contains(&"operator");
                let http = axum::http::Method::from_bytes(method.to_uppercase().as_bytes())
                    .expect("méthode");
                let guarded = method != "get" && is_operator_route(&http, &concrete);
                if documented != guarded {
                    problems.push(format!(
                        "{} {path} : x-roles {roles:?}, garde opérateur {guarded}",
                        method.to_uppercase()
                    ));
                }
            }
        }
        assert!(problems.is_empty(), "{}", problems.join("\n"));
    }

    #[test]
    fn every_operation_states_who_may_call_it() {
        let spec = spec_value();
        let mut ids = HashSet::new();
        let mut problems = Vec::new();
        for (path, item) in spec["paths"].as_object().expect("paths") {
            for method in METHODS {
                let Some(op) = item.get(method) else { continue };
                let label = format!("{} {path}", method.to_uppercase());
                match op["operationId"].as_str() {
                    Some(id) if ids.insert(id.to_string()) => {}
                    Some(id) => problems.push(format!("{label} : operationId en double {id}")),
                    None => problems.push(format!("{label} : operationId manquant")),
                }
                let scope = op["x-token-scope"].as_str().unwrap_or("");
                let public = is_public(path, method);
                let expected = if public {
                    "public"
                } else if is_session_only(path) {
                    "none"
                } else if method == "get"
                    || (method == "post"
                        && (path == "/api/mcp" || path == "/prometheus/api/v1/{route}"))
                {
                    "read"
                } else {
                    "write"
                };
                if scope != expected {
                    problems
                        .push(format!("{label} : x-token-scope {scope:?}, attendu {expected:?}"));
                }
                let open = op.get("security").and_then(Value::as_array).is_some_and(Vec::is_empty);
                if open != public {
                    problems
                        .push(format!("{label} : security [] seulement pour une route publique"));
                }
                if !public {
                    for status in ["401", "403"] {
                        if op["responses"].get(status).is_none() {
                            problems.push(format!("{label} : réponse {status} non décrite"));
                        }
                    }
                }
                if op["responses"].as_object().is_none_or(|r| r.is_empty()) {
                    problems.push(format!("{label} : aucune réponse décrite"));
                }
            }
        }
        assert!(problems.is_empty(), "{}", problems.join("\n"));
    }

    #[test]
    fn every_reference_resolves() {
        fn walk(value: &Value, spec: &Value, broken: &mut Vec<String>) {
            match value {
                Value::Object(map) => {
                    if let Some(Value::String(reference)) = map.get("$ref") {
                        let target = reference
                            .strip_prefix('#')
                            .map(|pointer| spec.pointer(pointer).is_some())
                            .unwrap_or(false);
                        if !target {
                            broken.push(reference.clone());
                        }
                    }
                    map.values().for_each(|v| walk(v, spec, broken));
                }
                Value::Array(items) => items.iter().for_each(|v| walk(v, spec, broken)),
                _ => {}
            }
        }
        let spec = spec_value();
        let mut broken = Vec::new();
        walk(&spec, &spec, &mut broken);
        assert!(broken.is_empty(), "références introuvables : {broken:?}");
    }

    #[test]
    fn the_specification_holds_no_secret() {
        let text = SPEC_YAML.to_ascii_lowercase();
        for needle in ["dmt_0", "dmt_1", "dmt_a", "dmon_0", "password: \"", "begin private key"] {
            assert!(!text.contains(needle), "motif suspect dans la spécification : {needle}");
        }
    }

    #[test]
    fn the_route_reader_understands_our_layouts() {
        let text = r#"
            .route("/a/{id}", get(x::get_one).put(x::update).delete(x::delete))
            .route(
                "/b",
                post(alerts::delete_silence).layer(DefaultBodyLimit::max(16 * 1024 * 1024)),
            )
            .route("/c/{*route}", get(promql).post(promql))
        "#;
        let routes = routes_in(text);
        let as_vec = |set: &BTreeSet<String>| set.iter().cloned().collect::<Vec<_>>();
        assert_eq!(routes[0].0, "/a/{id}");
        assert_eq!(as_vec(&routes[0].1), ["delete", "get", "put"]);
        assert_eq!(routes[1].0, "/b");
        assert_eq!(as_vec(&routes[1].1), ["post"]);
        assert_eq!(as_vec(&routes[2].1), ["get", "post"]);
    }
}
