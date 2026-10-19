//! Compression HTTP du routeur complet : le JSON de l'API est compressé quand
//! le client l'accepte, jamais autrement. Les exclusions (images, flux,
//! téléchargements, petites réponses) sont testées dans `api/mod.rs`.

mod common;

use axum::body::Body;
use axum::http::{Request, header};
use common::TestApp;
use http_body_util::BodyExt;
use tower::ServiceExt;

/// `(encodage annoncé, `Vary`, corps brut)` de `GET uri` avec cet `Accept-Encoding`.
async fn fetch(
    app: &TestApp,
    uri: &str,
    cookie: &str,
    accept: Option<&str>,
) -> (Option<String>, Option<String>, Vec<u8>) {
    let mut request = Request::builder().uri(uri).header(header::COOKIE, cookie);
    if let Some(accept) = accept {
        request = request.header(header::ACCEPT_ENCODING, accept);
    }
    let response = app.router.clone().oneshot(request.body(Body::empty()).unwrap()).await.unwrap();
    assert!(response.status().is_success(), "{uri} : {}", response.status());
    let text = |name| {
        response.headers().get(name).and_then(|value| value.to_str().ok()).map(str::to_string)
    };
    let (encoding, vary) = (text(header::CONTENT_ENCODING), text(header::VARY));
    let bytes = response.into_body().collect().await.unwrap().to_bytes().to_vec();
    (encoding, vary, bytes)
}

#[tokio::test]
async fn json_is_compressed_only_when_the_client_accepts_it() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let uri = "/api/notify/kinds";

    let (encoding, _, plain) = fetch(&app, uri, &admin, None).await;
    assert_eq!(encoding, None);
    assert!(plain.len() > 1024, "le catalogue doit dépasser le seuil de 1 Ko");

    let (encoding, vary, gzip) = fetch(&app, uri, &admin, Some("gzip")).await;
    assert_eq!(encoding.as_deref(), Some("gzip"));
    assert!(vary.unwrap().to_ascii_lowercase().contains("accept-encoding"));
    assert!(gzip.len() < plain.len() / 2, "gzip {} contre {} octets", gzip.len(), plain.len());

    let (encoding, _, brotli) = fetch(&app, uri, &admin, Some("br, gzip")).await;
    assert_eq!(encoding.as_deref(), Some("br"));
    assert!(brotli.len() < plain.len() / 2);
    println!("{uri} : {} octets bruts, {} gzip, {} brotli", plain.len(), gzip.len(), brotli.len());
}
