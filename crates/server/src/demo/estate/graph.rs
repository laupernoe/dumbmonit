//! Faux Microsoft Graph et service de connexion, sur le même hôte : le
//! locataire Microsoft 365 du client PME, lu par les collecteurs `m365` et
//! `entra` (étiquette `login_url`). Réponses des tests des collecteurs
//! (`m365/testdata`, `entra/testdata`), noms du parc, dates recalées.

use axum::Router;
use axum::http::{Method, StatusCode, Uri};
use axum::response::Response;

use super::{GRAPH, instant, json, replay};

/// Locataire : son domaine `onmicrosoft.com`.
pub(super) const TENANT: &str = "mullerarchi.onmicrosoft.com";

macro_rules! m365 {
    ($file:literal) => {
        include_str!(concat!("../../../../collectors/src/m365/testdata/", $file))
    };
}
macro_rules! entra {
    ($file:literal) => {
        include_str!(concat!("../../../../collectors/src/entra/testdata/", $file))
    };
}

/// Instant des réponses enregistrées.
const CAPTURE: &str = "2026-10-26T09:00:00Z";

const WORDS: &[(&str, &str)] = &[
    ("{{base}}", GRAPH),
    ("contoso.onmicrosoft.com", TENANT),
    ("contoso.com", "muller-archi.ch"),
    ("Contoso", "Muller Architectes"),
];

const TOKEN: &str =
    r#"{"token_type":"Bearer","expires_in":3599,"access_token":"demonstration-graph-token"}"#;

pub(super) fn router() -> Router {
    Router::new().fallback(handle)
}

async fn handle(method: Method, uri: Uri) -> Response {
    let path = uri.path().trim_end_matches('/');
    let next_page = uri.query().is_some_and(|q| q.contains("skiptoken"));
    let body = match (method, path) {
        (Method::POST, p) if p.ends_with("/oauth2/v2.0/token") => {
            return json(StatusCode::OK, TOKEN.into());
        }
        (_, "/v1.0/admin/serviceAnnouncement/healthOverviews") => m365!("health_overviews.json"),
        (_, "/v1.0/admin/serviceAnnouncement/issues") if next_page => m365!("issues_page2.json"),
        (_, "/v1.0/admin/serviceAnnouncement/issues") => m365!("issues_page1.json"),
        (_, "/v1.0/subscribedSkus") => m365!("subscribed_skus.json"),
        (_, "/v1.0/identityProtection/riskyUsers") => m365!("risky_users.json"),
        (_, "/v1.0/auditLogs/signIns") => m365!("signins.json"),
        (_, "/v1.0/applications") if next_page => entra!("applications_page2.json"),
        (_, "/v1.0/applications") => entra!("applications_page1.json"),
        (_, "/v1.0/organization") => entra!("organization.json"),
        (_, "/v1.0/reports/authenticationMethods/userRegistrationDetails") => {
            entra!("user_registration_details.json")
        }
        _ => return json(StatusCode::NOT_FOUND, r#"{"error":{"code":"NotFound"}}"#.into()),
    };
    replay(body, instant(CAPTURE), WORDS)
}
