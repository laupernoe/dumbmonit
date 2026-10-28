//! Faux Veeam Backup & Replication 13 (API REST, port 9419) : les réponses des
//! tests du collecteur (`veeam/testdata`), dates recalées. Le serveur du
//! bureau du client PME : une tâche de VM quotidienne, un serveur de fichiers
//! dont la dernière nuit a échoué, une copie hors site en cours.

use axum::Router;
use axum::http::{Method, StatusCode, Uri};
use axum::response::Response;

use super::{instant, json, replay};

macro_rules! capture {
    ($file:literal) => {
        include_str!(concat!("../../../../collectors/src/veeam/testdata/", $file))
    };
}

const TOKEN: &str = capture!("token.json");
const SERVER: &str = capture!("server_info.json");
const JOBS: &str = capture!("jobs_states.json");
const SESSIONS: &str = capture!("sessions.json");
const REPOSITORIES: &str = capture!("repositories_states.json");
const LICENSE: &str = capture!("license.json");
const RESTORE_POINTS: &str = capture!("restore_points.json");

/// Instant des réponses enregistrées.
const CAPTURE: &str = "2026-09-30T09:00:00+02:00";

/// Version publiée la plus récente (13.0.2, mai 2026) et noms du parc.
const WORDS: &[(&str, &str)] = &[
    ("12.3.1.1139", "13.0.2.29"),
    ("nas.example.lan", "nas.muller.home.arpa"),
    ("Example Ltd", "Muller Architectes SA"),
];

pub(super) fn router() -> Router {
    Router::new().fallback(handle)
}

async fn handle(method: Method, uri: Uri) -> Response {
    let path = uri.path().trim_end_matches('/');
    let body = match (method, path) {
        (Method::POST, "/api/oauth2/token") => TOKEN,
        (_, "/api/v1/serverInfo") => SERVER,
        (_, "/api/v1/jobs/states") => JOBS,
        (_, "/api/v1/sessions") => SESSIONS,
        (_, "/api/v1/backupInfrastructure/repositories/states") => REPOSITORIES,
        (_, "/api/v1/license") => LICENSE,
        (_, "/api/v1/restorePoints") => RESTORE_POINTS,
        _ => return json(StatusCode::NOT_FOUND, "{}".into()),
    };
    replay(body, instant(CAPTURE), WORDS)
}
