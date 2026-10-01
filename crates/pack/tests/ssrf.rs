//! Une requête de paquet ne quitte jamais l'adresse de sa cible.
//!
//! Faux service HTTP sur la boucle locale : c'est justement l'adresse que le
//! garde-fou refuse par défaut, ce qui permet de vérifier le refus puis, option
//! levée, le reste des règles (redirections, plafond de taille, identifiants).

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::get;
use dumbmonit_pack::{Pack, PackCollector};
use dumbmonit_proto::{Collector, Credential, ProbeError, Target};

const PACK: &str = r#"
schema: 1
id: fake-api
version: 1.0.0
label: Fake API
credentials: [api_token]
options:
  - key: route
    label: Route
    default: ok
sources:
  - id: status
    type: http
    path: /{{option.route}}
    auth: none
    headers:
      X-Api-Key: "{{credential.token}}"
metrics:
  - name: value
    source: status
    json: $.value
"#;

const TOKEN: &str = "t0ken-que-personne-ne-doit-lire";

async fn serve(seen: Arc<Mutex<Vec<String>>>) -> SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("port libre");
    let address = listener.local_addr().expect("adresse locale");
    let other = format!("http://localhost:{}/ok", address.port());
    let record = move |headers: HeaderMap| {
        let key = headers.get("x-api-key").and_then(|v| v.to_str().ok()).unwrap_or("-");
        seen.lock().unwrap().push(key.to_string());
    };
    let ok = {
        let record = record.clone();
        move |headers: HeaderMap| async move {
            record(headers);
            ([(header::CONTENT_TYPE, "application/json")], r#"{"value": 42}"#)
        }
    };
    let router = Router::new()
        .route("/ok", get(ok))
        .route("/same", get(|| async { (StatusCode::FOUND, [(header::LOCATION, "/ok")]) }))
        .route(
            "/elsewhere",
            get(move || {
                let other = other.clone();
                async move { (StatusCode::FOUND, [(header::LOCATION, other)]).into_response() }
            }),
        )
        .route("/loop", get(|| async { (StatusCode::FOUND, [(header::LOCATION, "/loop")]) }))
        .route("/denied", get(|| async { StatusCode::UNAUTHORIZED }))
        .route("/huge", get(|| async { "x".repeat(dumbmonit_pack::MAX_BODY_BYTES + 1) }));
    tokio::spawn(async move { axum::serve(listener, router).await.expect("faux service") });
    address
}

fn target(address: SocketAddr, tags: &[(&str, &str)]) -> Target {
    Target {
        id: 1,
        name: "fake".into(),
        address: address.to_string(),
        kind: "pack.fake-api".into(),
        profile_id: None,
        parent_id: None,
        interval: Duration::from_secs(60),
        enabled: true,
        tags: tags.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect::<BTreeMap<_, _>>(),
        credential: Credential::ApiToken { token: TOKEN.into() },
    }
}

async fn probe(
    address: SocketAddr,
    tags: &[(&str, &str)],
) -> Result<Vec<dumbmonit_proto::Sample>, ProbeError> {
    let collector = PackCollector::new(Arc::new(Pack::parse(PACK).expect("paquet valide")));
    assert_eq!(collector.kind(), "pack.fake-api");
    collector.probe(&target(address, tags)).await
}

const ALLOW: (&str, &str) = ("allow_private_targets", "true");

#[tokio::test]
async fn la_boucle_locale_est_refusee_sans_option_explicite() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let address = serve(seen.clone()).await;
    let error = probe(address, &[]).await.unwrap_err();
    assert!(matches!(error, ProbeError::Config(_)), "{error:?}");
    assert!(error.to_string().contains("allow_private_targets"), "{error}");
    assert!(seen.lock().unwrap().is_empty(), "aucune requête ne doit être partie");
}

#[tokio::test]
async fn option_levee_la_mesure_est_lue_et_le_jeton_part_dans_len_tete() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let address = serve(seen.clone()).await;
    let samples = probe(address, &[ALLOW]).await.expect("mesure");
    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].metric, "fake_api_value");
    assert_eq!(samples[0].value, 42.0);
    assert_eq!(*seen.lock().unwrap(), [TOKEN]);
}

#[tokio::test]
async fn une_redirection_vers_la_meme_origine_est_suivie() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let address = serve(seen.clone()).await;
    let samples = probe(address, &[ALLOW, ("route", "same")]).await.expect("mesure");
    assert_eq!(samples[0].value, 42.0);
}

#[tokio::test]
async fn une_redirection_vers_un_autre_hote_est_refusee() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let address = serve(seen.clone()).await;
    let error = probe(address, &[ALLOW, ("route", "elsewhere")]).await.unwrap_err();
    assert!(matches!(error, ProbeError::Config(_)), "{error:?}");
    assert!(error.to_string().contains("redirect to http://localhost"), "{error}");
    assert!(seen.lock().unwrap().is_empty(), "l'autre hôte n'a pas été appelé");
}

#[tokio::test]
async fn les_redirections_sans_fin_sont_coupees() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let address = serve(seen).await;
    let error = probe(address, &[ALLOW, ("route", "loop")]).await.unwrap_err();
    assert!(error.to_string().contains("redirects"), "{error}");
}

#[tokio::test]
async fn une_option_ne_peut_pas_ajouter_de_segment_ni_changer_dhote() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let address = serve(seen.clone()).await;
    // Encodée, la valeur reste un seul segment du chemin : le service répond 404.
    let error = probe(address, &[ALLOW, ("route", "@localhost:1/ok")]).await.unwrap_err();
    assert!(error.to_string().contains("HTTP 404"), "{error}");
}

#[tokio::test]
async fn un_refus_dauthentification_ne_divulgue_pas_le_jeton() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let address = serve(seen).await;
    let error = probe(address, &[ALLOW, ("route", "denied")]).await.unwrap_err();
    assert!(matches!(error, ProbeError::Auth(_)), "{error:?}");
    assert!(!format!("{error} {error:?}").contains(TOKEN));
}

#[tokio::test]
async fn une_reponse_trop_grosse_est_refusee() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let address = serve(seen).await;
    let error = probe(address, &[ALLOW, ("route", "huge")]).await.unwrap_err();
    assert!(matches!(error, ProbeError::Protocol(_)), "{error:?}");
    assert!(error.to_string().contains("larger than 4 MiB"), "{error}");
}

#[tokio::test]
async fn un_identifiant_dun_autre_type_est_une_erreur_de_configuration() {
    let collector = PackCollector::new(Arc::new(Pack::parse(PACK).unwrap()));
    let mut target = target("127.0.0.1:9".parse().unwrap(), &[ALLOW]);
    target.credential = Credential::None;
    let error = collector.probe(&target).await.unwrap_err();
    assert!(matches!(error, ProbeError::Config(_)), "{error:?}");
}
