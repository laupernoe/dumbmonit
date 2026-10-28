//! Faux serveur Kopia (`kopia server`) : les réponses des tests du collecteur
//! (`kopia/testdata`), dates recalées. Un dépôt sur le NAS, quatre sources
//! dont une en cours d'envoi et une en pause.

use axum::Router;
use axum::http::{StatusCode, Uri};
use axum::response::Response;

use super::{instant, json, replay};

const STATUS: &str = include_str!("../../../../collectors/src/kopia/testdata/repo_status.json");
const SOURCES: &str = include_str!("../../../../collectors/src/kopia/testdata/sources.json");

/// Instant des réponses enregistrées.
const CAPTURE: &str = "2026-10-26T01:00:00Z";

const WORDS: &[(&str, &str)] =
    &[("\"hostname\":\"nas\"", "\"hostname\":\"nas.home.arpa\""), ("\"laptop\"", "\"laptop-sam\"")];

pub(super) fn router() -> Router {
    Router::new().fallback(handle)
}

async fn handle(uri: Uri) -> Response {
    let body = match uri.path().trim_end_matches('/') {
        "/api/v1/repo/status" => STATUS,
        "/api/v1/sources" => SOURCES,
        _ => return json(StatusCode::NOT_FOUND, "{}".into()),
    };
    replay(body, instant(CAPTURE), WORDS)
}
