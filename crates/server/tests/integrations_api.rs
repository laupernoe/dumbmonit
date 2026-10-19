//! Fichiers du bandeau pour sites tiers : `/api/integrations/*` demande une session.
//! Le contenu du zip est vérifié par les tests de `api/integrations.rs`.

mod common;

use axum::http::StatusCode;
use common::TestApp;

const URIS: [&str; 2] = ["/api/integrations/wordpress.zip", "/api/integrations/banner.php"];

#[tokio::test]
async fn les_telechargements_du_bandeau_demandent_une_session() {
    let app = TestApp::configured().await;
    for uri in URIS {
        assert_eq!(app.get(uri, None).await.status, StatusCode::UNAUTHORIZED, "{uri}");
    }
    let admin = app.admin_cookie().await;
    for uri in URIS {
        assert_eq!(app.get(uri, Some(&admin)).await.status, StatusCode::OK, "{uri}");
    }
}
