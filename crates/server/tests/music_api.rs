//! Tests d'intégration de la musique du mur, contre un faux Spotify monté en
//! local : échange du code (PKCE vérifié), rafraîchissement, profil et lecture
//! en cours sont de vraies requêtes HTTP. Seul l'écran d'approbation de Spotify
//! est court-circuité — le test relève `state` et le défi PKCE dans l'adresse
//! d'autorisation, comme le ferait le navigateur.

mod common;

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::extract::{Form, State};
use axum::http::{HeaderMap, Request, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use common::{TestApp, UNREACHABLE_VICTORIA};
use dumbmonit_server::music::MusicHub;
use dumbmonit_server::music::spotify::Endpoints;
use dumbmonit_server::state::{AppState, Inner};
use dumbmonit_server::{api, collectors, db, tsdb};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tower::ServiceExt;

const CLIENT_ID: &str = "0123456789abcdef0123456789abcdef";
const LOOPBACK: &str = "http://127.0.0.1:8888/callback";
const SECRET: &str = "secret-de-test-suffisamment-long";
const SCOPES: &str = "streaming user-read-email user-read-private user-read-playback-state \
                      user-modify-playback-state user-read-currently-playing";

// ---------------------------------------------------------------- faux Spotify

#[derive(Clone, Copy, PartialEq)]
enum PlayerMode {
    Idle,
    Playing,
    RateLimited,
}

struct FakeState {
    /// Code d'autorisation → (défi PKCE, URI de redirection) relevés par le test.
    codes: HashMap<String, (String, String)>,
    refresh_tokens: HashSet<String>,
    access_tokens: HashSet<String>,
    /// Durée de vie des jetons d'accès émis.
    access_ttl: u64,
    /// Remet un nouveau jeton de rafraîchissement à chaque rafraîchissement.
    rotate: bool,
    issued: usize,
    player: PlayerMode,
    player_calls: usize,
    refresh_calls: usize,
    /// Appareils Spotify Connect du compte (`GET /v1/me/player/devices`).
    devices: Vec<Value>,
    devices_calls: usize,
    /// Transferts reçus (`PUT /v1/me/player`) : (appareil, play).
    transfers: Vec<(String, bool)>,
}

type Fake = Arc<Mutex<FakeState>>;

fn bearer(headers: &HeaderMap) -> String {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or_default()
        .to_string()
}

fn oauth_error(error: &str, description: &str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        axum::Json(json!({ "error": error, "error_description": description })),
    )
        .into_response()
}

async fn token(State(fake): State<Fake>, Form(form): Form<HashMap<String, String>>) -> Response {
    let mut fake = fake.lock().unwrap();
    assert_eq!(form.get("client_id").map(String::as_str), Some(CLIENT_ID), "client_id envoyé");
    assert!(!form.contains_key("client_secret"), "PKCE : jamais de secret");
    let mut rotated = None;
    match form.get("grant_type").map(String::as_str) {
        Some("authorization_code") => {
            let code = form.get("code").cloned().unwrap_or_default();
            let Some((challenge, redirect)) = fake.codes.remove(&code) else {
                return oauth_error("invalid_grant", "Invalid authorization code");
            };
            let verifier = form.get("code_verifier").cloned().unwrap_or_default();
            if URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())) != challenge {
                return oauth_error("invalid_grant", "code_verifier was incorrect");
            }
            if form.get("redirect_uri") != Some(&redirect) {
                return oauth_error("invalid_grant", "Invalid redirect URI");
            }
            fake.refresh_tokens.insert("refresh-1".into());
            rotated = Some("refresh-1".to_string());
        }
        Some("refresh_token") => {
            fake.refresh_calls += 1;
            let presented = form.get("refresh_token").cloned().unwrap_or_default();
            if !fake.refresh_tokens.contains(&presented) {
                return oauth_error("invalid_grant", "Refresh token revoked");
            }
            if fake.rotate {
                fake.refresh_tokens.remove(&presented);
                let next = format!("refresh-{}", fake.refresh_calls + 1);
                fake.refresh_tokens.insert(next.clone());
                rotated = Some(next);
            }
        }
        other => panic!("grant_type inattendu : {other:?}"),
    }
    fake.issued += 1;
    let access = format!("access-{}", fake.issued);
    fake.access_tokens.insert(access.clone());
    let mut body = json!({
        "access_token": access,
        "token_type": "Bearer",
        "expires_in": fake.access_ttl,
        "scope": SCOPES,
    });
    if let Some(refresh) = rotated {
        body["refresh_token"] = json!(refresh);
    }
    axum::Json(body).into_response()
}

async fn me(State(fake): State<Fake>, headers: HeaderMap) -> Response {
    if !fake.lock().unwrap().access_tokens.contains(&bearer(&headers)) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    axum::Json(json!({ "id": "noe", "display_name": "Noé", "type": "user", "product": "premium" }))
        .into_response()
}

async fn devices(State(fake): State<Fake>, headers: HeaderMap) -> Response {
    let mut fake = fake.lock().unwrap();
    fake.devices_calls += 1;
    if !fake.access_tokens.contains(&bearer(&headers)) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    axum::Json(json!({ "devices": fake.devices })).into_response()
}

async fn transfer(
    State(fake): State<Fake>,
    headers: HeaderMap,
    axum::Json(body): axum::Json<Value>,
) -> Response {
    let mut fake = fake.lock().unwrap();
    if !fake.access_tokens.contains(&bearer(&headers)) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let id = body["device_ids"][0].as_str().unwrap_or_default().to_string();
    if !fake.devices.iter().any(|d| d["id"] == id.as_str()) {
        return (
            StatusCode::NOT_FOUND,
            axum::Json(json!({ "error": { "status": 404, "message": "Device not found" } })),
        )
            .into_response();
    }
    fake.transfers.push((id, body["play"].as_bool().unwrap_or(false)));
    StatusCode::NO_CONTENT.into_response()
}

async fn player(State(fake): State<Fake>, headers: HeaderMap) -> Response {
    let mut fake = fake.lock().unwrap();
    fake.player_calls += 1;
    if !fake.access_tokens.contains(&bearer(&headers)) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    match fake.player {
        PlayerMode::Idle => StatusCode::NO_CONTENT.into_response(),
        PlayerMode::RateLimited => {
            (StatusCode::TOO_MANY_REQUESTS, [(header::RETRY_AFTER, "30")]).into_response()
        }
        PlayerMode::Playing => axum::Json(json!({
            "device": { "id": "d1", "name": "Living room TV", "type": "TV", "volume_percent": 40 },
            "is_playing": true,
            "progress_ms": 61000,
            "timestamp": 1,
            "currently_playing_type": "track",
            "item": {
                "name": "Teardrop", "duration_ms": 330000,
                "artists": [{ "name": "Massive Attack" }],
                "album": { "name": "Mezzanine", "images": [
                    { "url": "https://i.scdn.co/image/640", "width": 640, "height": 640 },
                    { "url": "https://i.scdn.co/image/300", "width": 300, "height": 300 }
                ] }
            }
        }))
        .into_response(),
    }
}

async fn spawn_fake() -> (Fake, String) {
    let fake: Fake = Arc::new(Mutex::new(FakeState {
        codes: HashMap::new(),
        refresh_tokens: HashSet::new(),
        access_tokens: HashSet::new(),
        access_ttl: 3600,
        rotate: false,
        issued: 0,
        player: PlayerMode::Idle,
        player_calls: 0,
        refresh_calls: 0,
        devices: Vec::new(),
        devices_calls: 0,
        transfers: Vec::new(),
    }));
    let router = Router::new()
        .route("/api/token", post(token))
        .route("/v1/me", get(me))
        .route("/v1/me/player", get(player).put(transfer))
        .route("/v1/me/player/devices", get(devices))
        .with_state(fake.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    (fake, base)
}

// ------------------------------------------------------------------- harnais

struct MusicApp {
    app: TestApp,
    pool: sqlx::SqlitePool,
    fake: Fake,
    admin: String,
}

/// Une instance configurée (un administrateur connecté), dont la musique parle
/// au faux Spotify.
async fn music_app() -> MusicApp {
    let (fake, base) = spawn_fake().await;
    let dir = tempfile::tempdir().unwrap();
    let config = common::base_config(dir.path());
    let pool = db::open(&config.database_path()).await.unwrap();
    let cipher = db::init_cipher(&pool, SECRET).await.unwrap();
    let mut registry = collectors::Registry::new();
    registry.register(Arc::new(collectors::DummyCollector));
    let victoria = tsdb::Victoria::new(UNREACHABLE_VICTORIA.to_string()).unwrap();
    let sink = tsdb::spawn_writer(victoria.clone(), Duration::from_secs(60));
    let state = AppState::new(Inner {
        config,
        pool: pool.clone(),
        cipher,
        victoria,
        sink,
        collectors: registry,
    });
    let hub = MusicHub::new(Endpoints { accounts: base.clone(), api: base });
    let app = TestApp { router: api::router_with(state, hub), _dir: dir };
    app.create_admin().await;
    let admin = app.admin_cookie().await;
    MusicApp { app, pool, fake, admin }
}

impl MusicApp {
    /// Commence une autorisation et rend `(state, défi PKCE)` relevés dans
    /// l'adresse d'approbation.
    async fn authorize(&self, redirect_uri: &str) -> (String, String) {
        let reply = self
            .app
            .post(
                "/api/music/spotify/authorize",
                json!({ "client_id": CLIENT_ID, "redirect_uri": redirect_uri }),
                Some(&self.admin),
            )
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let url = reqwest::Url::parse(reply.body["authorize_url"].as_str().unwrap()).unwrap();
        assert_eq!(url.path(), "/authorize");
        let query: HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(query["client_id"], CLIENT_ID);
        assert_eq!(query["redirect_uri"], redirect_uri);
        assert_eq!(query["code_challenge_method"], "S256");
        (query["state"].clone(), query["code_challenge"].clone())
    }

    /// Ce que fait Spotify quand on clique « Accepter » : un code lié au défi.
    fn approve(&self, code: &str, challenge: &str, redirect_uri: &str) {
        self.fake
            .lock()
            .unwrap()
            .codes
            .insert(code.into(), (challenge.into(), redirect_uri.into()));
    }

    async fn connect(&self) {
        let (state, challenge) = self.authorize(LOOPBACK).await;
        self.approve("code-ok", &challenge, LOOPBACK);
        let landing = format!("{LOOPBACK}?code=code-ok&state={state}");
        let reply = self
            .app
            .post("/api/music/spotify/complete", json!({ "url": landing }), Some(&self.admin))
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    }

    async fn stored_refresh_token(&self) -> Option<Vec<u8>> {
        sqlx::query_scalar("SELECT refresh_token FROM music_spotify WHERE id = 1")
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }

    /// Requête brute : en-têtes choisis, réponse complète.
    async fn raw(&self, method: &str, uri: &str, headers: &[(&str, &str)]) -> Response {
        let mut builder = Request::builder().method(method).uri(uri);
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        self.app.router.clone().oneshot(builder.body(Body::empty()).unwrap()).await.unwrap()
    }
}

async fn json_of(response: Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

fn location(response: &Response) -> String {
    response.headers()[header::LOCATION].to_str().unwrap().to_string()
}

// --------------------------------------------------------------------- tests

#[tokio::test]
async fn connecting_by_pasting_the_landing_address_keeps_the_refresh_token_sealed() {
    let m = music_app().await;
    let status = m.app.get("/api/music/spotify", Some(&m.admin)).await;
    assert_eq!(status.status, StatusCode::OK);
    assert_eq!(status.body["status"], "off");
    assert_eq!(status.body["loopback_redirect_uri"], LOOPBACK);
    assert_eq!(status.body["speaker_name"], "DumbMonit Wall");

    // Un lecteur ne relie rien.
    let viewer = m.app.viewer_cookie(&m.admin).await;
    let refused = m
        .app
        .post(
            "/api/music/spotify/authorize",
            json!({ "client_id": CLIENT_ID, "redirect_uri": LOOPBACK }),
            Some(&viewer),
        )
        .await;
    assert_eq!(refused.status, StatusCode::FORBIDDEN);

    // Ni client ID farfelu, ni redirection en http vers le réseau local.
    for (client_id, redirect) in [
        ("not a client id", LOOPBACK),
        (CLIENT_ID, "http://192.168.1.10:8080/api/music/spotify/callback"),
        (CLIENT_ID, "http://localhost:8888/callback"),
    ] {
        let reply = m
            .app
            .post(
                "/api/music/spotify/authorize",
                json!({ "client_id": client_id, "redirect_uri": redirect }),
                Some(&m.admin),
            )
            .await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{client_id} {redirect}");
    }

    let (state, challenge) = m.authorize(LOOPBACK).await;
    m.approve("code-ok", &challenge, LOOPBACK);

    // Une adresse dont le `state` n'a jamais été émis est refusée.
    let forged = m
        .app
        .post(
            "/api/music/spotify/complete",
            json!({ "url": format!("{LOOPBACK}?code=code-ok&state=forged") }),
            Some(&m.admin),
        )
        .await;
    assert_eq!(forged.status, StatusCode::BAD_REQUEST);
    let garbage =
        m.app.post("/api/music/spotify/complete", json!({ "url": "hello" }), Some(&m.admin)).await;
    assert_eq!(garbage.status, StatusCode::BAD_REQUEST);
    assert!(garbage.body["error"].as_str().unwrap().contains("127.0.0.1:8888"));

    let landing = format!("{LOOPBACK}?code=code-ok&state={state}");
    let reply =
        m.app.post("/api/music/spotify/complete", json!({ "url": landing }), Some(&m.admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["status"], "connected");
    assert_eq!(reply.body["account_name"], "Noé");
    assert_eq!(reply.body["client_id"], CLIENT_ID);
    assert_eq!(reply.body["missing_scopes"], json!([]));
    assert!(reply.body["reconnect_by"].is_string());
    let text = reply.body.to_string();
    assert!(!text.contains("refresh-1") && !text.contains("access-"), "aucun jeton : {text}");

    // Chiffré en base, jamais en clair.
    let sealed = m.stored_refresh_token().await.expect("jeton stocké");
    assert!(!sealed.windows(9).any(|w| w == b"refresh-1"));

    // Un `state` ne sert qu'une fois.
    let replay =
        m.app.post("/api/music/spotify/complete", json!({ "url": landing }), Some(&m.admin)).await;
    assert_eq!(replay.status, StatusCode::BAD_REQUEST);

    // Le relevé ne montre jamais de jeton non plus.
    let status = m.app.get("/api/music/spotify", Some(&viewer)).await;
    assert_eq!(status.body["status"], "connected");
    assert!(!status.body.to_string().contains("refresh-"));
}

#[tokio::test]
async fn the_direct_callback_needs_the_session_that_started_it() {
    let m = music_app().await;
    let callback = "https://monit.example.org/api/music/spotify/callback";

    // L'origine de la page doit être celle de la redirection.
    let reply = m
        .app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/music/spotify/authorize")
                .header(header::COOKIE, &m.admin)
                .header("x-requested-with", "DumbMonit")
                .header(header::ORIGIN, "https://elsewhere.example.org")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "client_id": CLIENT_ID, "redirect_uri": callback }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reply.status(), StatusCode::BAD_REQUEST);

    let (state, challenge) = m.authorize(callback).await;
    m.approve("code-direct", &challenge, callback);
    let uri = format!("/api/music/spotify/callback?code=code-direct&state={state}");

    // Sans la session de départ : retour aux réglages, rien d'enregistré, et
    // la tentative reste valable pour son auteur.
    let anonymous = m.raw("GET", &uri, &[]).await;
    assert_eq!(anonymous.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&anonymous), "/settings?spotify=session#music");

    let back = m.raw("GET", &uri, &[("cookie", m.admin.as_str())]).await;
    assert_eq!(back.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&back), "/settings?spotify=connected#music");
    assert_eq!(m.app.get("/api/music/spotify", Some(&m.admin)).await.body["status"], "connected");

    // Rejouer le retour ne sert à rien.
    let replay = m.raw("GET", &uri, &[("cookie", m.admin.as_str())]).await;
    assert_eq!(location(&replay), "/settings?spotify=state#music");

    // L'utilisateur a refusé chez Spotify.
    let (state, _) = m.authorize(callback).await;
    let denied = m
        .raw(
            "GET",
            &format!("/api/music/spotify/callback?error=access_denied&state={state}"),
            &[("cookie", m.admin.as_str())],
        )
        .await;
    assert_eq!(location(&denied), "/settings?spotify=denied#music");
}

#[tokio::test]
async fn the_wall_reads_what_plays_anywhere_through_a_short_cache() {
    let m = music_app().await;

    // Rien de relié : le mur ne demande rien à Spotify.
    let now = m.app.get("/api/music/now", Some(&m.admin)).await;
    assert_eq!(now.status, StatusCode::OK);
    assert_eq!(now.body["spotify"]["status"], "off");
    assert_eq!(now.body["spotify"]["now_playing"], Value::Null);
    assert_eq!(now.body["speaker_name"], "DumbMonit Wall");
    assert_eq!(m.fake.lock().unwrap().player_calls, 0);

    m.connect().await;
    m.fake.lock().unwrap().player = PlayerMode::Playing;
    let viewer = m.app.viewer_cookie(&m.admin).await;
    let now = m.app.get("/api/music/now", Some(&viewer)).await;
    let playing = &now.body["spotify"]["now_playing"];
    assert_eq!(now.body["spotify"]["status"], "connected");
    assert_eq!(playing["title"], "Teardrop");
    assert_eq!(playing["artists"], json!(["Massive Attack"]));
    assert_eq!(playing["album"], "Mezzanine");
    assert_eq!(playing["cover_url"], "https://i.scdn.co/image/300");
    assert_eq!(playing["device_name"], "Living room TV");
    assert_eq!(playing["playing"], true);
    assert_eq!(playing["progress_ms"], 61000);

    // Dix murs qui interrogent en même temps : une seule requête à Spotify.
    for _ in 0..10 {
        m.app.get("/api/music/now", Some(&viewer)).await;
    }
    assert_eq!(m.fake.lock().unwrap().player_calls, 1);

    // Spotify demande de ralentir : la carte reste, avec le motif, et on attend.
    m.fake.lock().unwrap().player = PlayerMode::RateLimited;
    tokio::time::sleep(Duration::from_millis(4100)).await;
    let slowed = m.app.get("/api/music/now", Some(&viewer)).await;
    assert_eq!(slowed.body["spotify"]["now_playing"]["title"], "Teardrop");
    assert!(slowed.body["spotify"]["error"].as_str().unwrap().contains("slow down"));
    m.app.get("/api/music/now", Some(&viewer)).await;
    assert_eq!(m.fake.lock().unwrap().player_calls, 2, "Retry-After respecté");
}

#[tokio::test]
async fn nothing_playing_hides_the_card() {
    let m = music_app().await;
    m.connect().await;
    let now = m.app.get("/api/music/now", Some(&m.admin)).await;
    assert_eq!(now.body["spotify"]["status"], "connected");
    assert_eq!(now.body["spotify"]["now_playing"], Value::Null);
    assert_eq!(now.body["spotify"]["error"], Value::Null);
}

#[tokio::test]
async fn the_sdk_token_goes_to_a_session_never_to_an_api_token() {
    let m = music_app().await;

    // Rien de relié : 409, que le mur lit comme « pas d'enceinte ».
    let none = m.app.get("/api/music/spotify/token", Some(&m.admin)).await;
    assert_eq!(none.status, StatusCode::CONFLICT);

    m.connect().await;
    let viewer = m.app.viewer_cookie(&m.admin).await;
    let response = m.raw("GET", "/api/music/spotify/token", &[("cookie", viewer.as_str())]).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let body = json_of(response).await;
    assert!(body["access_token"].as_str().unwrap().starts_with("access-"));
    assert!(body["expires_in"].as_u64().unwrap() > 3000);
    assert!(body.get("refresh_token").is_none());

    let created = m
        .app
        .post("/api/tokens", json!({ "name": "script", "scope": "write" }), Some(&m.admin))
        .await;
    let secret = created.body["secret"].as_str().unwrap().to_string();
    let with_token = m
        .raw(
            "GET",
            "/api/music/spotify/token",
            &[("authorization", format!("Bearer {secret}").as_str())],
        )
        .await;
    assert_eq!(with_token.status(), StatusCode::UNAUTHORIZED);
    // Le jeton d'API lit bien la lecture en cours, lui.
    let read = m
        .raw("GET", "/api/music/now", &[("authorization", format!("Bearer {secret}").as_str())])
        .await;
    assert_eq!(read.status(), StatusCode::OK);

    let anonymous = m.raw("GET", "/api/music/spotify/token", &[]).await;
    assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn refresh_tokens_rotate_and_a_revoked_one_ends_the_connection() {
    let m = music_app().await;
    {
        let mut fake = m.fake.lock().unwrap();
        // Des jetons d'accès qui expirent sous la marge : chaque demande rafraîchit.
        fake.access_ttl = 60;
        fake.rotate = true;
    }
    m.connect().await;
    // Chaque rafraîchissement invalide le jeton présenté et en remet un neuf :
    // le second ne réussit que si le premier a bien été enregistré.
    for expected in 1..=2 {
        let token = m.app.get("/api/music/spotify/token", Some(&m.admin)).await;
        assert_eq!(token.status, StatusCode::OK, "{}", token.body);
        assert_eq!(m.fake.lock().unwrap().refresh_calls, expected);
    }
    assert!(!m.fake.lock().unwrap().refresh_tokens.contains("refresh-1"));

    // Le compte retire l'accès (ou six mois ont passé).
    m.fake.lock().unwrap().refresh_tokens.clear();
    let now = m.app.get("/api/music/now", Some(&m.admin)).await;
    assert_eq!(now.body["spotify"]["status"], "expired");
    assert!(now.body["spotify"]["error"].as_str().unwrap().contains("connect it again"));
    assert_eq!(m.stored_refresh_token().await, None);

    let status = m.app.get("/api/music/spotify", Some(&m.admin)).await;
    assert_eq!(status.body["status"], "expired");
    assert_eq!(status.body["last_error"], "Refresh token revoked");
    assert_eq!(status.body["client_id"], CLIENT_ID, "gardé pour se reconnecter en un geste");
    let token = m.app.get("/api/music/spotify/token", Some(&m.admin)).await;
    assert_eq!(token.status, StatusCode::CONFLICT);

    // Se reconnecter efface l'erreur.
    m.connect().await;
    let status = m.app.get("/api/music/spotify", Some(&m.admin)).await;
    assert_eq!(status.body["status"], "connected");
    assert_eq!(status.body["last_error"], Value::Null);
}

#[tokio::test]
async fn disconnecting_forgets_the_account() {
    let m = music_app().await;
    m.connect().await;
    let viewer = m.app.viewer_cookie(&m.admin).await;
    assert_eq!(
        m.app.delete("/api/music/spotify", Some(&viewer)).await.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        m.app.delete("/api/music/spotify", Some(&m.admin)).await.status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(m.app.get("/api/music/spotify", Some(&m.admin)).await.body["status"], "off");
    assert_eq!(m.app.get("/api/music/now", Some(&m.admin)).await.body["spotify"]["status"], "off");
    let token = m.app.get("/api/music/spotify/token", Some(&m.admin)).await;
    assert_eq!(token.status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn the_shared_link_is_checked_and_reaches_every_wall() {
    let m = music_app().await;
    let viewer = m.app.viewer_cookie(&m.admin).await;
    let link = "https://www.deezer.com/fr/playlist/1313621735";

    let refused = m.app.put("/api/music/link", json!({ "link": link }), Some(&viewer)).await;
    assert_eq!(refused.status, StatusCode::FORBIDDEN);
    let bad = m
        .app
        .put("/api/music/link", json!({ "link": "https://evil.example/x" }), Some(&m.admin))
        .await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST);

    let set =
        m.app.put("/api/music/link", json!({ "link": format!(" {link} ") }), Some(&m.admin)).await;
    assert_eq!(set.status, StatusCode::OK, "{}", set.body);
    assert_eq!(set.body["link"], link);
    assert_eq!(set.body["set_by"], "admin");

    // Tous les murs le voient, lecteurs compris.
    let now = m.app.get("/api/music/now", Some(&viewer)).await;
    assert_eq!(now.body["link"]["link"], link);

    assert_eq!(
        m.app.delete("/api/music/link", Some(&m.admin)).await.status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(m.app.get("/api/music/now", Some(&viewer)).await.body["link"], Value::Null);
}

// ----------------------------------------------------------------- l'enceinte

fn wall_device(id: &str, name: &str) -> Value {
    json!({ "id": id, "name": name, "type": "Computer", "is_active": false,
            "is_restricted": false, "volume_percent": 80 })
}

#[tokio::test]
async fn the_speaker_is_renamed_by_an_admin_for_every_wall() {
    let m = music_app().await;
    let viewer = m.app.viewer_cookie(&m.admin).await;

    let refused =
        m.app.put("/api/music/speaker", json!({ "name": "Kitchen" }), Some(&viewer)).await;
    assert_eq!(refused.status, StatusCode::FORBIDDEN);
    for bad in ["   x\ny", &"a".repeat(65)] {
        let reply = m.app.put("/api/music/speaker", json!({ "name": bad }), Some(&m.admin)).await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{bad:?}");
    }

    let renamed = m
        .app
        .put("/api/music/speaker", json!({ "name": "  Living room TV " }), Some(&m.admin))
        .await;
    assert_eq!(renamed.status, StatusCode::OK, "{}", renamed.body);
    assert_eq!(renamed.body["speaker_name"], "Living room TV");
    let now = m.app.get("/api/music/now", Some(&viewer)).await;
    assert_eq!(now.body["speaker_name"], "Living room TV");
    let account = m.app.get("/api/music/spotify", Some(&viewer)).await;
    assert_eq!(account.body["speaker_name"], "Living room TV");

    let reset = m.app.put("/api/music/speaker", json!({ "name": null }), Some(&m.admin)).await;
    assert_eq!(reset.body["speaker_name"], "DumbMonit Wall");
}

#[tokio::test]
async fn walls_report_their_speaker_and_learn_whether_spotify_lists_it() {
    let m = music_app().await;
    m.connect().await;
    m.fake.lock().unwrap().devices =
        vec![wall_device("wall1", "DumbMonit Wall"), wall_device("phone", "Pixel")];
    let viewer = m.app.viewer_cookie(&m.admin).await;

    let report = |display: &str, phase: &str, device: Option<&str>| {
        json!({
            "display": display, "name": "DumbMonit Wall", "phase": phase,
            "activated": true, "device_id": device, "browser": "Chrome 130 on Linux",
            "premium": if phase == "ready" { Some(true) } else { None },
            "problem": if phase == "unsupported" { Some("No DRM module.") } else { None },
        })
    };
    let listed = m
        .app
        .post(
            "/api/music/speaker/report",
            report("tv-000001", "ready", Some("wall1")),
            Some(&viewer),
        )
        .await;
    assert_eq!(listed.status, StatusCode::OK, "{}", listed.body);
    assert_eq!(listed.body["listed"], true);
    let ghost = m
        .app
        .post(
            "/api/music/speaker/report",
            report("tv-000002", "ready", Some("gone")),
            Some(&viewer),
        )
        .await;
    assert_eq!(ghost.body["listed"], false);
    let no_drm = m
        .app
        .post("/api/music/speaker/report", report("tv-000003", "unsupported", None), Some(&viewer))
        .await;
    assert_eq!(no_drm.body["listed"], Value::Null);
    assert_eq!(m.fake.lock().unwrap().devices_calls, 1, "la liste des appareils est mise en cache");

    let bad = m
        .app
        .post("/api/music/speaker/report", report("tv-000004", "dancing", None), Some(&viewer))
        .await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST);

    // Un jeton d'API ne se fait pas passer pour un mur.
    let created = m
        .app
        .post("/api/tokens", json!({ "name": "script", "scope": "write" }), Some(&m.admin))
        .await;
    let secret = created.body["secret"].as_str().unwrap().to_string();
    let mut request = Request::builder()
        .method("POST")
        .uri("/api/music/speaker/report")
        .header("authorization", format!("Bearer {secret}"))
        .header("content-type", "application/json")
        .header("x-requested-with", "DumbMonit");
    request = request.header("accept", "application/json");
    let response = m
        .app
        .router
        .clone()
        .oneshot(request.body(Body::from(report("tv-000005", "ready", None).to_string())).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let status = m.app.get("/api/music/speaker", Some(&viewer)).await;
    assert_eq!(status.status, StatusCode::OK, "{}", status.body);
    assert_eq!(status.body["status"], "connected");
    assert_eq!(status.body["premium"], true);
    assert_eq!(status.body["listed"], true);
    assert_eq!(status.body["devices"].as_array().unwrap().len(), 2);
    let walls = status.body["walls"].as_array().unwrap();
    assert_eq!(walls.len(), 3);
    let tv3 = walls.iter().find(|w| w["display"] == "tv-000003").unwrap();
    assert_eq!(tv3["phase"], "unsupported");
    assert_eq!(tv3["problem"], "No DRM module.");
    assert_eq!(tv3["browser"], "Chrome 130 on Linux");
}

#[tokio::test]
async fn play_here_transfers_the_account_to_the_speaker() {
    let m = music_app().await;
    let viewer = m.app.viewer_cookie(&m.admin).await;

    let unconnected = m.app.post("/api/music/speaker/play", json!({}), Some(&viewer)).await;
    assert_eq!(unconnected.status, StatusCode::CONFLICT);

    m.connect().await;
    m.fake.lock().unwrap().devices =
        vec![wall_device("phone", "Pixel"), wall_device("wall1", "DumbMonit Wall")];

    // Personne ne s'appelle ainsi chez Spotify : on dit quoi faire.
    m.app.put("/api/music/speaker", json!({ "name": "Kitchen" }), Some(&m.admin)).await;
    let missing = m.app.post("/api/music/speaker/play", json!({}), Some(&viewer)).await;
    assert_eq!(missing.status, StatusCode::CONFLICT);
    assert!(missing.body["error"].as_str().unwrap().contains("Kitchen"));
    m.app.put("/api/music/speaker", json!({ "name": null }), Some(&m.admin)).await;

    // « Test sound » : l'appareil qui porte le nom de l'enceinte.
    let played = m.app.post("/api/music/speaker/play", json!({}), Some(&viewer)).await;
    assert_eq!(played.status, StatusCode::OK, "{}", played.body);
    assert_eq!(played.body["device_id"], "wall1");

    // « Play here » depuis le mur : son propre identifiant.
    let here =
        m.app.post("/api/music/speaker/play", json!({ "device_id": "wall1" }), Some(&viewer)).await;
    assert_eq!(here.status, StatusCode::OK, "{}", here.body);
    assert_eq!(
        m.fake.lock().unwrap().transfers,
        [("wall1".to_string(), true), ("wall1".to_string(), true)]
    );

    // Un appareil que Spotify ne connaît pas : l'erreur de Spotify, traduite.
    let gone =
        m.app.post("/api/music/speaker/play", json!({ "device_id": "gone" }), Some(&viewer)).await;
    assert_eq!(gone.status, StatusCode::BAD_GATEWAY);
    assert!(gone.body["error"].as_str().unwrap().contains("Reload the wall"));
}
