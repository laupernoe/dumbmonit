//! HTTPS de l'interface : en-têtes de protection, cookie de session `__Host-`,
//! HSTS partout, écran des réglages, et le port qui ne sert que HTTPS (TLS 1.3
//! et 1.2 modernes) : une requête en clair n'y reçoit qu'une redirection.

mod common;

use std::sync::Arc;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{HeaderMap, Request, StatusCode, header};
use dumbmonit_server::state::{AppState, Inner};
use dumbmonit_server::web_tls::{self, WebPeer};
use dumbmonit_server::{api, backup, collectors, db, tsdb};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

use common::{PASSWORD, TestApp};

/// Instance de test dont le port sert HTTPS : certificat autosigné fabriqué
/// comme au premier démarrage.
async fn tls_app() -> (TestApp, AppState) {
    let dir = tempfile::tempdir().unwrap();
    let config = common::base_config(dir.path());
    let pool = db::open(&config.database_path()).await.unwrap();
    let cipher = db::init_cipher(&pool, "secret-de-test-suffisamment-long").await.unwrap();
    let victoria = tsdb::Victoria::new(common::UNREACHABLE_VICTORIA).unwrap();
    let sink = tsdb::spawn_writer(victoria.clone(), std::time::Duration::from_secs(60));
    let mut registry = collectors::Registry::new();
    registry.register(Arc::new(collectors::DummyCollector));
    let state = AppState::new(Inner { config, pool, cipher, victoria, sink, collectors: registry });
    web_tls::load(&state).await.expect("certificat autosigné");
    let app = TestApp { router: api::router(state.clone()), _dir: dir };
    app.create_admin().await;
    (app, state)
}

/// Requête telle que l'écoute réelle la remet : avec ce qu'elle sait de la
/// connexion.
fn request(
    method: &str,
    uri: &str,
    peer: Option<WebPeer>,
    headers: &[(&str, &str)],
) -> Request<Body> {
    let mut builder =
        Request::builder().method(method).uri(uri).header(header::HOST, "nas.lan:8080");
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let mut request = builder.body(Body::empty()).unwrap();
    if let Some(peer) = peer {
        request.extensions_mut().insert(ConnectInfo(peer));
    }
    request
}

fn peer() -> Option<WebPeer> {
    Some(WebPeer { addr: "192.168.1.20:51000".parse().unwrap() })
}

async fn send(app: &TestApp, request: Request<Body>) -> (StatusCode, HeaderMap, Vec<u8>) {
    let response = app.router.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = response.into_body().collect().await.unwrap().to_bytes().to_vec();
    (status, headers, body)
}

fn text<'a>(headers: &'a HeaderMap, name: &str) -> &'a str {
    headers.get(name).and_then(|v| v.to_str().ok()).unwrap_or("")
}

#[tokio::test]
async fn every_response_isolates_the_interface_and_api_answers_are_not_cached() {
    let app = TestApp::configured().await;
    let (_, headers, _) = send(&app, request("GET", "/api/auth/status", None, &[])).await;
    assert_eq!(text(&headers, "x-content-type-options"), "nosniff");
    assert_eq!(text(&headers, "cross-origin-opener-policy"), "same-origin");
    assert_eq!(text(&headers, "cross-origin-resource-policy"), "same-origin");
    assert_eq!(text(&headers, "cache-control"), "no-store");
    assert!(text(&headers, "permissions-policy").contains("camera=()"));
    let csp = text(&headers, "content-security-policy");
    assert!(csp.contains("frame-ancestors 'none'"), "{csp}");
    assert!(csp.contains("script-src 'self' 'nonce-"), "{csp}");
    // Le port ne sert que HTTPS : HSTS sur toute réponse.
    assert_eq!(text(&headers, "strict-transport-security"), web_tls::HSTS);

    // Ce qui est fait pour être intégré ailleurs reste lisible depuis ailleurs.
    let (_, headers, _) =
        send(&app, request("GET", "/api/public/status/home/badge.svg", None, &[])).await;
    assert_eq!(text(&headers, "cross-origin-resource-policy"), "cross-origin");
    assert_eq!(text(&headers, "strict-transport-security"), web_tls::HSTS);
}

#[tokio::test]
async fn the_session_cookie_is_always_host_prefixed_secure_and_strict() {
    let app = TestApp::configured().await;
    let body = json!({ "username": "admin", "password": PASSWORD }).to_string();
    let mut login =
        request("POST", "/api/auth/login", peer(), &[("content-type", "application/json")]);
    *login.body_mut() = Body::from(body);
    let (status, headers, _) = send(&app, login).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(text(&headers, "strict-transport-security"), web_tls::HSTS);
    let cookies: Vec<&str> =
        headers.get_all(header::SET_COOKIE).iter().filter_map(|v| v.to_str().ok()).collect();
    assert_eq!(cookies.len(), 1, "{cookies:?}");
    let session = cookies[0];
    assert!(session.starts_with("__Host-dumbmonit_session="), "{session}");
    for attribute in ["; Secure", "HttpOnly", "SameSite=Strict", "Path=/"] {
        assert!(session.contains(attribute), "{attribute} : {session}");
    }
    assert!(!session.contains("Domain="), "{session}");

    // Le cookie __Host- ouvre bien la session.
    let pair = session.split(';').next().unwrap();
    let (status, _, _) =
        send(&app, request("GET", "/api/auth/me", peer(), &[("cookie", pair)])).await;
    assert_eq!(status, StatusCode::OK);

    // La déconnexion l'efface sous le même nom, `Secure` compris.
    let logout = request(
        "POST",
        "/api/auth/logout",
        peer(),
        &[("cookie", pair), ("x-requested-with", "DumbMonit")],
    );
    let (_, headers, _) = send(&app, logout).await;
    let cleared = text(&headers, "set-cookie");
    assert!(cleared.starts_with("__Host-dumbmonit_session=;"), "{cleared}");
    assert!(cleared.contains("Max-Age=0") && cleared.contains("; Secure"), "{cleared}");
}

#[tokio::test]
async fn the_status_says_https_whatever_a_proxy_claims() {
    let app = common::setup_with(|config| {
        config.trusted_proxies = vec!["10.0.0.0/8".parse().unwrap()];
    })
    .await;
    for proto in ["https", "http"] {
        let asked = request("GET", "/api/auth/status", None, &[("x-forwarded-proto", proto)]);
        let (_, headers, body) = send(&app, asked).await;
        assert_eq!(text(&headers, "strict-transport-security"), web_tls::HSTS);
        let status: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(status["https"]["secure"], true, "{proto}");
    }
}

#[tokio::test]
async fn the_settings_show_the_certificate_and_nothing_to_switch_off() {
    let (app, _state) = tls_app().await;
    let admin = app.admin_cookie().await;

    let view = app.get("/api/security/https", Some(&admin)).await;
    assert_eq!(view.status, StatusCode::OK, "{}", view.body);
    assert_eq!(view.body["available"], true);
    assert_eq!(view.body["certificate"]["source"], "self_signed");
    let fingerprint = view.body["certificate"]["fingerprint_sha256"].as_str().unwrap();
    assert_eq!(fingerprint.len(), 95, "{fingerprint}");
    assert!(view.body["certificate"]["public_key_pin"].as_str().unwrap().starts_with("sha256//"));
    assert!(view.body["certificate"].get("der").is_none(), "le DER n'est pas dans la vue");
    assert!(view.body.get("redirect").is_none(), "plus de redirection à régler");
    let codes: Vec<&str> = view.body["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|f| f["code"].as_str())
        .collect();
    assert!(!codes.contains(&"https_redirect_off"), "{codes:?}");
    assert!(codes.contains(&"self_signed"), "{codes:?}");

    // Le clair ne se rallume pas : la route d'écriture n'existe plus.
    let reply = app.put("/api/security/https", json!({ "redirect": false }), Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::METHOD_NOT_ALLOWED, "{}", reply.body);

    // Le certificat se télécharge en PEM.
    let (status, _, body) = send(
        &app,
        request(
            "GET",
            "/api/security/https/certificate.pem",
            None,
            &[("cookie", admin.as_str()), ("x-requested-with", "DumbMonit")],
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(String::from_utf8(body).unwrap().starts_with("-----BEGIN CERTIFICATE-----"));

    // Un lecteur n'y a pas accès.
    let viewer = app.viewer_cookie(&admin).await;
    assert_eq!(app.get("/api/security/https", Some(&viewer)).await.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn the_install_commands_pin_the_self_signed_key_and_never_go_in_clear() {
    let (app, state) = tls_app().await;
    let admin = app.admin_cookie().await;
    let pin = state.web_tls().summary().unwrap().public_key_pin.unwrap();
    // Même une adresse en clair, copiée d'un ancien lien, devient HTTPS.
    let reply = app.get("/api/agent/install?base_url=http://nas.lan:8080", Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.body["base_url"], "https://nas.lan:8080");
    assert_eq!(reply.body["https_pin"], pin.as_str());
    let linux = reply.body["linux_template"].as_str().unwrap();
    let windows = reply.body["windows_template"].as_str().unwrap();
    for command in [linux, windows] {
        assert!(!command.contains("http://"), "{command}");
        assert!(command.contains(&pin), "{command}");
    }
    assert!(linux.starts_with(&format!("curl -sSL -k --pinnedpubkey {pin} https://nas.lan:8080/")));
    assert!(windows.contains("ServerCertificateValidationCallback"), "{windows}");
}

#[tokio::test]
async fn the_self_signed_certificate_survives_a_restart_with_its_key() {
    let (_app, state) = tls_app().await;
    let first = state.web_tls().summary().unwrap();
    // Second chargement, comme au redémarrage : même certificat.
    web_tls::load(&state).await.unwrap();
    let second = state.web_tls().summary().unwrap();
    assert_eq!(first.fingerprint_sha256, second.fingerprint_sha256);
    // La clé n'est jamais rangée en clair.
    let raw: Vec<u8> = sqlx::query_scalar("SELECT value FROM settings WHERE key = 'web_tls_key'")
        .fetch_one(&state.pool)
        .await
        .unwrap();
    assert!(!String::from_utf8_lossy(&raw).contains("PRIVATE KEY"));
    assert!(serde_json::from_slice::<Value>(&raw).is_err(), "pas un JSON lisible : chiffré");
}

#[tokio::test]
async fn the_backup_bundle_stays_sealed() {
    let source_dir = tempfile::tempdir().unwrap();
    let source = db::open(&source_dir.path().join("dumbmonit.db")).await.unwrap();
    let cipher = db::init_cipher(&source, "secret-de-test-suffisamment-long").await.unwrap();
    let lot = backup::collect(&source, &cipher, backup::ExportOptions::default()).await.unwrap();

    // Le lot exporté est chiffré : Argon2id (au moins 19 Mio) puis AES-256-GCM.
    let sealed = backup::bundle::seal(&lot, "une phrase de passe assez longue").unwrap();
    assert_eq!(sealed.cipher, "aes-256-gcm");
    assert_eq!(sealed.kdf.algorithm, "argon2id");
    assert!(sealed.kdf.m_cost >= 19 * 1024);
    assert!(backup::bundle::open(&sealed, "une autre phrase de passe longue").is_err());
    assert!(backup::bundle::open(&sealed, "une phrase de passe assez longue").is_ok());
}

// --- La socket réelle : HTTPS seulement ------------------------------------

mod wire {
    use std::sync::Arc;

    use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
    use rustls::crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature};
    use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
    use rustls::{ClientConfig, DigitallySignedStruct, SignatureScheme};

    /// Le certificat est autosigné : le test ne vérifie que le protocole.
    #[derive(Debug)]
    pub struct AcceptAll(pub Arc<CryptoProvider>);

    impl ServerCertVerifier for AcceptAll {
        fn verify_server_cert(
            &self,
            _: &CertificateDer<'_>,
            _: &[CertificateDer<'_>],
            _: &ServerName<'_>,
            _: &[u8],
            _: UnixTime,
        ) -> Result<ServerCertVerified, rustls::Error> {
            Ok(ServerCertVerified::assertion())
        }

        fn verify_tls12_signature(
            &self,
            message: &[u8],
            cert: &CertificateDer<'_>,
            dss: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, rustls::Error> {
            verify_tls12_signature(message, cert, dss, &self.0.signature_verification_algorithms)
        }

        fn verify_tls13_signature(
            &self,
            message: &[u8],
            cert: &CertificateDer<'_>,
            dss: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, rustls::Error> {
            verify_tls13_signature(message, cert, dss, &self.0.signature_verification_algorithms)
        }

        fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
            self.0.signature_verification_algorithms.supported_schemes()
        }
    }

    pub fn client(version: &'static rustls::SupportedProtocolVersion) -> ClientConfig {
        let provider = dumbmonit_server::web_tls::provider();
        ClientConfig::builder_with_provider(provider.clone())
            .with_protocol_versions(&[version])
            .unwrap()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(AcceptAll(provider)))
            .with_no_client_auth()
    }
}

#[tokio::test]
async fn the_web_port_only_speaks_modern_tls_and_redirects_plain_http() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let (app, state) = tls_app().await;
    let tcp = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = tcp.local_addr().unwrap();
    let listener = web_tls::listen(state.web_tls().clone(), tcp).unwrap();
    let router = app.router.clone();
    tokio::spawn(async move {
        let service = router.into_make_service_with_connect_info::<WebPeer>();
        let _ = axum::serve(listener, service).await;
    });
    const ASK: &[u8] =
        b"GET /api/auth/status HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";

    // En clair : une redirection vers HTTPS, et rien de l'application — ni
    // corps, ni cookie, ni en-tête de sécurité.
    let mut plain = tokio::net::TcpStream::connect(addr).await.unwrap();
    plain
        .write_all(b"GET /api/auth/status?x=1 HTTP/1.1\r\nHost: nas.lan:8080\r\n\r\n")
        .await
        .unwrap();
    plain.shutdown().await.unwrap();
    let mut answer = String::new();
    plain.read_to_string(&mut answer).await.unwrap();
    assert_eq!(
        answer,
        "HTTP/1.1 308 Permanent Redirect\r\nLocation: https://nas.lan:8080/api/auth/status?x=1\r\n\
         Content-Length: 0\r\nConnection: close\r\n\r\n"
    );
    // Sans Host : un refus nu.
    let mut plain = tokio::net::TcpStream::connect(addr).await.unwrap();
    plain.write_all(b"GET / HTTP/1.0\r\n\r\n").await.unwrap();
    plain.shutdown().await.unwrap();
    let mut answer = String::new();
    plain.read_to_string(&mut answer).await.unwrap();
    assert!(answer.starts_with("HTTP/1.1 400 "), "{answer}");
    assert!(answer.ends_with("Content-Length: 0\r\nConnection: close\r\n\r\n"), "{answer}");

    // TLS 1.3, échange de clés hybride post-quantique, puis TLS 1.2 AEAD.
    let versions = [(&rustls::version::TLS13, "TLSv1_3"), (&rustls::version::TLS12, "TLSv1_2")];
    for (version, expected) in versions {
        let connector = tokio_rustls::TlsConnector::from(Arc::new(wire::client(version)));
        let tcp = tokio::net::TcpStream::connect(addr).await.unwrap();
        let name = rustls::pki_types::ServerName::try_from("localhost").unwrap();
        let mut tls = connector.connect(name, tcp).await.expect("poignée de main");
        {
            let (_, session) = tls.get_ref();
            assert_eq!(format!("{:?}", session.protocol_version().unwrap()), expected);
            let suite = format!("{:?}", session.negotiated_cipher_suite().unwrap().suite());
            assert!(suite.contains("GCM") || suite.contains("CHACHA20"), "{suite}");
            if expected == "TLSv1_3" {
                let group = session.negotiated_key_exchange_group().unwrap().name();
                assert_eq!(format!("{group:?}"), "X25519MLKEM768");
            }
        }
        tls.write_all(ASK).await.unwrap();
        let mut answer = Vec::new();
        let _ = tls.read_to_end(&mut answer).await;
        let answer = String::from_utf8_lossy(&answer).to_ascii_lowercase();
        assert!(answer.starts_with("http/1.1 200"), "{answer}");
        assert!(answer.contains("strict-transport-security: max-age=31536000"), "{answer}");
        assert!(answer.contains("\"secure\":true"), "{answer}");
    }

    // Le healthcheck du conteneur sonde ce port en HTTPS.
    assert!(dumbmonit_server::healthcheck::probe(addr).await.is_ok());
}
