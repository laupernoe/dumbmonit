//! Serveur MCP, révision 2026-07-28 (sans état, métadonnées par requête,
//! `server/discover`, en-têtes recopiés du corps) et les outils ajoutés avec
//! elle. Chaque `structuredContent` reçu est validé contre l'`outputSchema`
//! que `tools/list` annonce : le serveur s'y engage, un client peut le vérifier.

mod common;

use std::collections::HashMap;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use common::TestApp;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

const MODERN: &str = "2026-07-28";

struct Mcp {
    app: TestApp,
    admin: String,
    reader: String,
    writer: String,
    schemas: HashMap<String, Value>,
}

struct Reply {
    status: StatusCode,
    body: Value,
}

/// `_meta` d'une requête moderne.
fn meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": MODERN,
        "io.modelcontextprotocol/clientInfo": { "name": "test", "version": "0" },
        "io.modelcontextprotocol/clientCapabilities": {}
    })
}

impl Mcp {
    async fn new() -> Self {
        let app = TestApp::configured().await;
        let admin = app.admin_cookie().await;
        let token = |scope: &'static str| {
            let app = &app;
            let admin = admin.clone();
            async move {
                let reply = app
                    .post(
                        "/api/tokens",
                        json!({ "name": format!("{scope} token"), "scope": scope }),
                        Some(&admin),
                    )
                    .await;
                assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);
                reply.body["secret"].as_str().unwrap().to_string()
            }
        };
        let reader = token("read").await;
        let writer = token("write").await;
        let mut mcp = Self { app, admin, reader, writer, schemas: HashMap::new() };
        let reply = mcp.modern(&mcp.reader.clone(), "tools/list", json!({})).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        for tool in reply.body["result"]["tools"].as_array().unwrap() {
            mcp.schemas
                .insert(tool["name"].as_str().unwrap().to_string(), tool["outputSchema"].clone());
        }
        mcp
    }

    /// Envoie un corps JSON-RPC avec les en-têtes donnés.
    async fn raw(&self, token: &str, headers: &[(&str, &str)], body: Value) -> Reply {
        let mut builder = Request::builder()
            .method("POST")
            .uri("/api/mcp")
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "application/json, text/event-stream");
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        let request = builder.body(Body::from(body.to_string())).unwrap();
        let response = self.app.router.clone().oneshot(request).await.expect("réponse");
        let status = response.status();
        let bytes = response.into_body().collect().await.expect("corps").to_bytes();
        Reply { status, body: serde_json::from_slice(&bytes).unwrap_or(Value::Null) }
    }

    /// Requête moderne bien formée : `_meta` et en-têtes en accord.
    async fn modern(&self, token: &str, method: &str, mut params: Value) -> Reply {
        params["_meta"] = meta();
        let name = params.get("name").and_then(Value::as_str).map(str::to_string);
        let mut headers = vec![("mcp-protocol-version", MODERN), ("mcp-method", method)];
        if let Some(name) = name.as_deref() {
            headers.push(("mcp-name", name));
        }
        self.raw(
            token,
            &headers,
            json!({ "jsonrpc": "2.0", "id": 7, "method": method, "params": params }),
        )
        .await
    }

    /// Appel d'outil moderne ; vérifie la forme du résultat et son schéma.
    async fn call(&self, token: &str, tool: &str, arguments: Value) -> Value {
        let reply =
            self.modern(token, "tools/call", json!({ "name": tool, "arguments": arguments })).await;
        assert_eq!(reply.status, StatusCode::OK, "{tool}: {}", reply.body);
        assert!(reply.body.get("error").is_none(), "{tool}: {}", reply.body);
        let result = reply.body["result"].clone();
        assert_eq!(result["resultType"], "complete", "{tool}");
        assert_eq!(result["_meta"]["io.modelcontextprotocol/serverInfo"]["name"], "DumbMonit");
        if result["isError"] == false {
            let schema = &self.schemas[tool];
            let problems = validate(schema, &result["structuredContent"], "$");
            assert!(
                problems.is_empty(),
                "{tool} ne respecte pas son outputSchema :\n{}\n{}",
                problems.join("\n"),
                result["structuredContent"]
            );
        }
        result
    }
}

fn text(result: &Value) -> String {
    result["content"][0]["text"].as_str().unwrap_or_default().to_string()
}

/// Validation JSON Schema réduite à ce que les schémas de sortie emploient :
/// `type` (simple ou liste), `properties`, `required`, `items`, `enum`,
/// `additionalProperties` (schéma).
fn validate(schema: &Value, value: &Value, at: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if let Some(kind) = schema.get("type") {
        let kinds: Vec<&str> = match kind {
            Value::String(k) => vec![k.as_str()],
            Value::Array(ks) => ks.iter().filter_map(Value::as_str).collect(),
            _ => vec![],
        };
        let matches = kinds.iter().any(|k| match *k {
            "object" => value.is_object(),
            "array" => value.is_array(),
            "string" => value.is_string(),
            "boolean" => value.is_boolean(),
            "null" => value.is_null(),
            "integer" => value.is_i64() || value.is_u64(),
            "number" => value.is_number(),
            _ => false,
        });
        if !matches {
            problems.push(format!("{at}: {value} n'est pas de type {kinds:?}"));
            return problems;
        }
    }
    if let Some(choices) = schema.get("enum").and_then(Value::as_array)
        && !choices.contains(value)
    {
        problems.push(format!("{at}: {value} hors de {choices:?}"));
    }
    if let Some(object) = value.as_object() {
        for required in schema.get("required").and_then(Value::as_array).into_iter().flatten() {
            let key = required.as_str().unwrap();
            if !object.contains_key(key) {
                problems.push(format!("{at}: champ requis absent « {key} »"));
            }
        }
        let properties = schema.get("properties").and_then(Value::as_object);
        for (key, item) in object {
            match properties.and_then(|p| p.get(key)) {
                Some(sub) => problems.extend(validate(sub, item, &format!("{at}.{key}"))),
                None => {
                    if let Some(extra) =
                        schema.get("additionalProperties").filter(|e| e.is_object())
                    {
                        problems.extend(validate(extra, item, &format!("{at}.{key}")));
                    }
                }
            }
        }
    }
    if let (Some(items), Some(array)) = (schema.get("items"), value.as_array()) {
        for (index, item) in array.iter().enumerate() {
            problems.extend(validate(items, item, &format!("{at}[{index}]")));
        }
    }
    problems
}

// --------------------------------------------------------------------------
// Protocole 2026-07-28
// --------------------------------------------------------------------------

#[tokio::test]
async fn server_discover_announces_versions_and_identity() {
    let mcp = Mcp::new().await;
    let reply = mcp.modern(&mcp.reader, "server/discover", json!({})).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let result = &reply.body["result"];
    assert_eq!(result["resultType"], "complete");
    let versions = result["supportedVersions"].as_array().unwrap();
    assert_eq!(versions[0], MODERN);
    assert!(versions.contains(&json!("2025-11-25")));
    assert!(result["capabilities"]["tools"].is_object());
    assert_eq!(result["_meta"]["io.modelcontextprotocol/serverInfo"]["name"], "DumbMonit");
    assert!(result["instructions"].as_str().unwrap().contains("get_status"));
    assert!(result["ttlMs"].is_u64());
    assert_eq!(result["cacheScope"], "private");
}

#[tokio::test]
async fn tools_list_is_cacheable_and_annotated() {
    let mcp = Mcp::new().await;
    let reply = mcp.modern(&mcp.reader, "tools/list", json!({})).await;
    let result = &reply.body["result"];
    assert_eq!(result["resultType"], "complete");
    assert!(result["ttlMs"].as_u64().is_some_and(|ttl| ttl > 0));
    assert_eq!(result["cacheScope"], "private");
    let tools = result["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 27);
    // Ordre déterministe, d'un appel à l'autre.
    let again = mcp.modern(&mcp.reader, "tools/list", json!({})).await;
    assert_eq!(again.body["result"]["tools"], result["tools"]);
    let readers = tools.iter().filter(|t| t["annotations"]["readOnlyHint"] == true).count();
    assert_eq!(readers, 15);
}

#[tokio::test]
async fn headers_must_mirror_the_body() {
    let mcp = Mcp::new().await;
    let body = |method: &str, params: Value| {
        let mut params = params;
        params["_meta"] = meta();
        json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params })
    };
    let call = body("tools/call", json!({ "name": "get_status", "arguments": {} }));

    // Sans Mcp-Method.
    let reply = mcp.raw(&mcp.reader, &[("mcp-protocol-version", MODERN)], call.clone()).await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    assert_eq!(reply.body["error"]["code"], -32020);

    // Mcp-Name différent du corps.
    let reply = mcp
        .raw(
            &mcp.reader,
            &[
                ("mcp-protocol-version", MODERN),
                ("mcp-method", "tools/call"),
                ("mcp-name", "list_devices"),
            ],
            call.clone(),
        )
        .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    assert_eq!(reply.body["error"]["code"], -32020);
    assert!(reply.body["error"]["message"].as_str().unwrap().contains("Mcp-Name"));

    // Mcp-Name sous sa forme Base64 : accepté.
    let reply = mcp
        .raw(
            &mcp.reader,
            &[
                ("mcp-protocol-version", MODERN),
                ("mcp-method", "tools/call"),
                ("mcp-name", "=?base64?Z2V0X3N0YXR1cw==?="),
            ],
            call.clone(),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["result"]["isError"], false);

    // Version de l'en-tête différente de celle du corps.
    let reply = mcp
        .raw(
            &mcp.reader,
            &[
                ("mcp-protocol-version", "2025-11-25"),
                ("mcp-method", "tools/call"),
                ("mcp-name", "get_status"),
            ],
            call,
        )
        .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    assert_eq!(reply.body["error"]["code"], -32020);
}

#[tokio::test]
async fn unsupported_versions_missing_metadata_and_unknown_methods() {
    let mcp = Mcp::new().await;

    let mut params = json!({ "_meta": meta() });
    params["_meta"]["io.modelcontextprotocol/protocolVersion"] = json!("2030-01-01");
    let reply = mcp
        .raw(
            &mcp.reader,
            &[("mcp-protocol-version", "2030-01-01"), ("mcp-method", "tools/list")],
            json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/list", "params": params }),
        )
        .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    assert_eq!(reply.body["id"], 3);
    assert_eq!(reply.body["error"]["code"], -32022);
    assert_eq!(reply.body["error"]["data"]["requested"], "2030-01-01");
    assert!(reply.body["error"]["data"]["supported"].as_array().unwrap().contains(&json!(MODERN)));

    // Capacités du client absentes : requête mal formée.
    let reply = mcp
        .raw(
            &mcp.reader,
            &[("mcp-protocol-version", MODERN), ("mcp-method", "tools/list")],
            json!({ "jsonrpc": "2.0", "id": 4, "method": "tools/list",
                    "params": { "_meta": { "io.modelcontextprotocol/protocolVersion": MODERN } } }),
        )
        .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    assert_eq!(reply.body["error"]["code"], -32602);

    // Méthode inconnue : 404 en révision moderne.
    let reply = mcp.modern(&mcp.reader, "resources/list", json!({})).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert_eq!(reply.body["error"]["code"], -32601);

    // Un en-tête moderne sans `_meta` : invalide aussi.
    let reply = mcp
        .raw(
            &mcp.reader,
            &[("mcp-protocol-version", MODERN), ("mcp-method", "tools/list")],
            json!({ "jsonrpc": "2.0", "id": 5, "method": "tools/list" }),
        )
        .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    assert_eq!(reply.body["error"]["code"], -32602);

    // Version inconnue en ancienne époque (en-tête seul) : 400 aussi.
    let reply = mcp
        .raw(
            &mcp.reader,
            &[("mcp-protocol-version", "1999-01-01")],
            json!({ "jsonrpc": "2.0", "id": 6, "method": "ping" }),
        )
        .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    assert_eq!(reply.body["error"]["code"], -32022);
}

#[tokio::test]
async fn the_mcp_endpoint_carries_rate_limit_headers_and_accepts_its_own_origin() {
    let mcp = Mcp::new().await;
    let request = Request::builder()
        .method("POST")
        .uri("/api/mcp")
        .header(header::AUTHORIZATION, format!("Bearer {}", mcp.reader))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::HOST, "monit.lan:8080")
        .header(header::ORIGIN, "http://monit.lan:8080")
        .body(Body::from(json!({ "jsonrpc": "2.0", "id": 1, "method": "ping" }).to_string()))
        .unwrap();
    let response = mcp.app.router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().contains_key("ratelimit-remaining"));
    assert!(response.headers().contains_key("x-dumbmonit-api-version"));
}

// --------------------------------------------------------------------------
// Outils existants, sous la révision moderne et contre leur schéma
// --------------------------------------------------------------------------

#[tokio::test]
async fn read_tools_match_their_output_schema() {
    let mcp = Mcp::new().await;
    let reply = mcp
        .app
        .post(
            "/api/targets",
            json!({ "name": "nas", "address": "nas.lan", "kind": "dummy" }),
            Some(&mcp.admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED);
    let id = reply.body["id"].as_i64().unwrap();
    mcp.app.post(&format!("/api/targets/{id}/probe"), json!({}), Some(&mcp.admin)).await;

    for (tool, arguments) in [
        ("get_status", json!({})),
        ("list_devices", json!({})),
        ("get_device", json!({ "name": "nas" })),
        ("list_alerts", json!({ "include_pending": true })),
        ("alert_history", json!({ "hours": 2 })),
        ("list_silences", json!({})),
        ("list_rules", json!({})),
        ("list_device_types", json!({})),
        ("list_device_types", json!({ "kind": "http" })),
        ("list_agents", json!({})),
        ("list_heartbeats", json!({})),
        ("list_status_pages", json!({})),
        ("list_channels", json!({})),
        ("list_packs", json!({})),
    ] {
        let result = mcp.call(&mcp.reader, tool, arguments).await;
        assert_eq!(result["isError"], false, "{tool}: {}", text(&result));
    }
}

#[tokio::test]
async fn the_device_type_catalogue_explains_what_add_device_expects() {
    let mcp = Mcp::new().await;
    let result = mcp.call(&mcp.reader, "list_device_types", json!({})).await;
    let kinds: Vec<&str> = result["structuredContent"]["kinds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|k| k["kind"].as_str().unwrap())
        .collect();
    assert!(kinds.contains(&"dummy") && kinds.contains(&"http"), "{kinds:?}");

    let result = mcp.call(&mcp.reader, "list_device_types", json!({ "kind": "HTTP" })).await;
    assert_eq!(result["structuredContent"]["kinds"][0]["kind"], "http");
    assert!(text(&result).contains("`http`"), "{}", text(&result));

    let result = mcp.call(&mcp.reader, "list_device_types", json!({ "kind": "toaster" })).await;
    assert_eq!(result["isError"], true);
    assert!(text(&result).contains("Known types"));
}

// --------------------------------------------------------------------------
// Nouveaux outils d'écriture
// --------------------------------------------------------------------------

#[tokio::test]
async fn add_device_needs_write_and_never_echoes_the_credential() {
    let mcp = Mcp::new().await;
    let arguments = json!({
        "name": "Core switch",
        "kind": "dummy",
        "address": "10.0.0.2",
        "interval_secs": 120,
        "options": { "role": "network", "port": 161 },
        "credential": { "type": "snmp_community", "community": "s3cr3t-community" }
    });

    let result = mcp.call(&mcp.reader, "add_device", arguments.clone()).await;
    assert_eq!(result["isError"], true);
    assert!(text(&result).contains("write"), "{}", text(&result));

    let result = mcp.call(&mcp.writer, "add_device", arguments).await;
    assert_eq!(result["isError"], false, "{}", text(&result));
    let device = &result["structuredContent"];
    assert_eq!(device["name"], "Core switch");
    assert_eq!(device["interval_secs"], 120);
    assert_eq!(device["tags"]["port"], "161");
    assert_eq!(device["credential_kind"], "SNMP community");
    assert!(!result.to_string().contains("s3cr3t-community"), "le secret ne ressort jamais");

    // Un enfant nommé par son parent.
    let result = mcp
        .call(
            &mcp.writer,
            "add_device",
            json!({ "name": "AP", "kind": "dummy", "address": "10.0.0.3", "parent": "core" }),
        )
        .await;
    assert_eq!(result["isError"], false, "{}", text(&result));
    assert_eq!(result["structuredContent"]["parent_id"], device["id"]);

    // Les erreurs de validation de l'API reviennent comme résultat d'outil.
    let result = mcp
        .call(&mcp.writer, "add_device", json!({ "name": "bad", "kind": "dummy", "address": "x", "credential": { "type": "nope" } }))
        .await;
    assert_eq!(result["isError"], true);
    assert!(text(&result).contains("Invalid device"), "{}", text(&result));
    let result = mcp
        .call(&mcp.writer, "add_device", json!({ "name": "x", "kind": "toaster", "address": "y" }))
        .await;
    assert_eq!(result["isError"], true, "{}", text(&result));

    let list = mcp.app.get("/api/targets", Some(&mcp.admin)).await;
    assert_eq!(list.body.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn maintenance_can_be_planned_for_a_device_or_the_whole_instance() {
    let mcp = Mcp::new().await;
    mcp.app
        .post(
            "/api/targets",
            json!({ "name": "nas", "address": "nas.lan", "kind": "dummy" }),
            Some(&mcp.admin),
        )
        .await;
    let start = chrono::Utc::now() + chrono::TimeDelta::days(2);
    let starts_at = start.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    let result = mcp
        .call(
            &mcp.writer,
            "schedule_maintenance",
            json!({ "starts_at": starts_at, "hours": 3, "comment": "upgrade" }),
        )
        .await;
    assert_eq!(result["isError"], false, "{}", text(&result));
    assert!(result["structuredContent"]["device_id"].is_null());
    assert_eq!(result["structuredContent"]["active_now"], false);
    assert!(text(&result).contains("whole instance"));

    let result = mcp
        .call(
            &mcp.writer,
            "schedule_maintenance",
            json!({ "device": "nas", "starts_at": chrono::Utc::now().to_rfc3339(), "ends_at": starts_at }),
        )
        .await;
    assert_eq!(result["isError"], false, "{}", text(&result));
    assert_eq!(result["structuredContent"]["device"], "nas");
    assert_eq!(result["structuredContent"]["active_now"], true);

    for arguments in [
        json!({ "starts_at": "tomorrow" }),
        json!({ "starts_at": starts_at, "hours": 200 }),
        json!({ "starts_at": "2020-01-01T00:00:00Z", "hours": 1 }),
        json!({ "starts_at": starts_at, "ends_at": "2020-01-01T00:00:00Z" }),
    ] {
        let result = mcp.call(&mcp.writer, "schedule_maintenance", arguments.clone()).await;
        assert_eq!(result["isError"], true, "{arguments}: {}", text(&result));
    }

    let silences = mcp.call(&mcp.reader, "list_silences", json!({})).await;
    assert_eq!(silences["structuredContent"]["silences"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn incidents_are_posted_on_status_pages() {
    let mcp = Mcp::new().await;
    let reply = mcp
        .app
        .post(
            "/api/status-pages",
            json!({ "title": "Home lab", "published": true }),
            Some(&mcp.admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);

    let result = mcp
        .call(
            &mcp.reader,
            "post_incident",
            json!({ "title": "NAS down", "body": "Looking into it." }),
        )
        .await;
    assert_eq!(result["isError"], true, "lecture seule : {}", text(&result));

    let result = mcp
        .call(
            &mcp.writer,
            "post_incident",
            json!({ "title": "NAS down", "body": "Looking into it.", "page": "home-lab", "severity": "major" }),
        )
        .await;
    assert_eq!(result["isError"], false, "{}", text(&result));
    let id = result["structuredContent"]["id"].as_i64().unwrap();
    assert_eq!(result["structuredContent"]["status"], "investigating");
    assert_eq!(result["structuredContent"]["updates"], 1);

    let result = mcp
        .call(
            &mcp.writer,
            "post_incident",
            json!({ "incident_id": id, "status": "resolved", "body": "Disk replaced." }),
        )
        .await;
    assert_eq!(result["isError"], false, "{}", text(&result));
    assert_eq!(result["structuredContent"]["status"], "resolved");
    assert_eq!(result["structuredContent"]["updates"], 2);

    let result = mcp.call(&mcp.reader, "list_status_pages", json!({})).await;
    let page = &result["structuredContent"]["pages"][0];
    assert_eq!(page["path"], "/s/home-lab");
    assert_eq!(page["published"], true);
    let incident = &result["structuredContent"]["incidents"][0];
    assert_eq!(incident["open"], false);
    assert_eq!(incident["latest_update"], "Disk replaced.");

    let result = mcp
        .call(&mcp.writer, "post_incident", json!({ "title": "x", "body": "y", "page": "nope" }))
        .await;
    assert_eq!(result["isError"], true);
    assert!(text(&result).contains("home-lab"), "{}", text(&result));
}

#[tokio::test]
async fn channels_are_listed_without_secrets_and_can_be_tested() {
    let mcp = Mcp::new().await;
    let reply = mcp
        .app
        .post(
            "/api/notify/channels",
            json!({
                "name": "Ops hook",
                "kind": "discord",
                "secrets": { "webhook_url": "https://127.0.0.1:1/hook/s3cr3t" }
            }),
            Some(&mcp.admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.body);

    let result = mcp.call(&mcp.reader, "list_channels", json!({})).await;
    assert_eq!(result["structuredContent"]["channels"][0]["name"], "Ops hook");
    assert!(!result.to_string().contains("s3cr3t"));

    let result = mcp.call(&mcp.reader, "test_channel", json!({ "channel": "ops" })).await;
    assert_eq!(result["isError"], true, "lecture seule");

    let result = mcp.call(&mcp.writer, "test_channel", json!({ "channel": "ops" })).await;
    // Le crochet est injoignable : l'échec revient comme résultat d'outil lisible.
    assert_eq!(result["isError"], true, "{}", text(&result));
    assert!(text(&result).contains("failed"), "{}", text(&result));
    assert!(!result.to_string().contains("s3cr3t"));

    let result = mcp.call(&mcp.writer, "test_channel", json!({ "channel": "pager" })).await;
    assert_eq!(result["isError"], true);
    assert!(text(&result).contains("Ops hook"));
}

#[tokio::test]
async fn agent_tools_explain_when_a_device_has_no_agent() {
    let mcp = Mcp::new().await;
    mcp.app
        .post(
            "/api/targets",
            json!({ "name": "nas", "address": "nas.lan", "kind": "dummy" }),
            Some(&mcp.admin),
        )
        .await;

    let result = mcp.call(&mcp.reader, "list_agents", json!({})).await;
    assert_eq!(result["isError"], false);
    assert!(text(&result).contains("No agent"));

    let result = mcp.call(&mcp.reader, "list_containers", json!({ "device": "nas" })).await;
    assert_eq!(result["isError"], true);
    assert!(text(&result).contains("not a machine running the agent"), "{}", text(&result));

    let result = mcp
        .call(&mcp.writer, "restart_container", json!({ "device": "nas", "container": "web" }))
        .await;
    assert_eq!(result["isError"], true);
    let result = mcp
        .call(&mcp.reader, "restart_container", json!({ "device": "nas", "container": "web" }))
        .await;
    assert!(text(&result).contains("write"), "la portée est vérifiée d'abord");
}

#[tokio::test]
async fn heartbeats_never_reveal_their_secret_url() {
    let mcp = Mcp::new().await;
    let reply = mcp
        .app
        .post(
            "/api/targets",
            json!({ "name": "nightly backup", "address": "backup-job", "kind": "dummy" }),
            Some(&mcp.admin),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED);
    // Sans type `push` dans ce registre de test, la liste est vide : l'outil
    // le dit et propose de quoi faire.
    let result = mcp.call(&mcp.reader, "list_heartbeats", json!({})).await;
    assert_eq!(result["isError"], false);
    assert!(text(&result).contains("No heartbeat monitor"));
    assert!(!result.to_string().contains("/api/push/"));
}

#[tokio::test]
async fn discovery_is_a_write_tool_and_validates_its_network() {
    let mcp = Mcp::new().await;
    let result = mcp.call(&mcp.reader, "discover_network", json!({ "cidr": "127.0.0.1/32" })).await;
    assert_eq!(result["isError"], true);

    let result =
        mcp.call(&mcp.writer, "discover_network", json!({ "cidr": "not-a-network" })).await;
    assert_eq!(result["isError"], true);
    assert!(text(&result).contains("Invalid network"), "{}", text(&result));

    let result = mcp.call(&mcp.writer, "discover_network", json!({ "cidr": "10.0.0.0/8" })).await;
    assert_eq!(result["isError"], true);
    assert!(text(&result).contains("too large"), "{}", text(&result));

    let result = mcp
        .call(
            &mcp.writer,
            "discover_network",
            json!({ "cidr": "127.0.0.1/32", "timeout_ms": 100, "community": "s3cr3t" }),
        )
        .await;
    assert_eq!(result["isError"], false, "{}", text(&result));
    assert_eq!(result["structuredContent"]["scanned"], 1);
    assert!(!result.to_string().contains("s3cr3t"));
}
