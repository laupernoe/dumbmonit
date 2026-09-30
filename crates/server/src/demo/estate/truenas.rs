//! Faux TrueNAS SCALE 25.04 : les réponses d'un vrai NAS
//! (`truenas/testdata/truenas2504`), pool sain, horodatages recalés.

use axum::Router;
use axum::http::{StatusCode, Uri};
use axum::response::Response;
use serde_json::Value;

use super::{json, now_s, shift_times};

/// Instant de la capture (secondes).
const CAPTURE_S: i64 = 1_790_291_100;

macro_rules! capture {
    ($file:literal) => {
        include_str!(concat!("../../../../collectors/src/truenas/testdata/truenas2504/", $file))
    };
}

/// Chemin sous `/api/v2.0` → réponse enregistrée.
const ROUTES: &[(&str, &str)] = &[
    ("/system/info", capture!("system_info.json")),
    ("/pool", capture!("pool.json")),
    ("/pool/scrub", capture!("pool_scrub.json")),
    ("/alert/list", capture!("alert_list.json")),
    ("/replication", capture!("replication.json")),
    ("/pool/snapshottask", capture!("pool_snapshottask.json")),
    ("/service", capture!("service.json")),
    ("/pool/dataset", capture!("pool_dataset_flat.json")),
    ("/zfs/snapshot", capture!("zfs_snapshot_count.json")),
    ("/disk", capture!("disk_extra_pools.json")),
    ("/disk/temperatures", capture!("disk_temperatures.json")),
    ("/smart/test/results", capture!("smart_test_results.json")),
];

pub(super) fn router() -> Router {
    Router::new().fallback(handle)
}

async fn handle(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches("/api/v2.0");
    let path = path.trim_end_matches('/');
    let Some((_, body)) = ROUTES.iter().find(|(route, _)| *route == path) else {
        return json(StatusCode::NOT_FOUND, "{\"message\": \"Not found\"}".into());
    };
    let body = body.replace("\"truenas\"", "\"truenas.home.arpa\"");
    match serde_json::from_str::<Value>(&body) {
        Ok(mut doc) => {
            shift_times(&mut doc, now_s() - CAPTURE_S);
            json(StatusCode::OK, doc.to_string())
        }
        Err(_) => json(StatusCode::OK, body),
    }
}
