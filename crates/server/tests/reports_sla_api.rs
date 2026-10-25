//! Rapport SLA : téléchargement à la demande (PDF, CSV) et calendriers de type
//! `sla`. VictoriaMetrics est injoignable ici : le rapport se construit quand
//! même, sans chiffres de disponibilité.

mod common;

use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::json;
use tower::ServiceExt;

use common::TestApp;

struct Raw {
    status: StatusCode,
    headers: HeaderMap,
    bytes: Vec<u8>,
}

async fn get(app: &TestApp, uri: &str, cookie: Option<&str>) -> Raw {
    let mut builder = Request::builder().uri(uri);
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, cookie);
    }
    let response = app.router.clone().oneshot(builder.body(Body::empty()).unwrap()).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.expect("corps").to_bytes().to_vec();
    Raw { status, headers, bytes }
}

#[tokio::test]
async fn a_pdf_is_built_for_a_chosen_period() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let uri = "/api/reports/sla?from=2024-10-01&to=2024-10-31&format=pdf&objective=99.9";
    let reply = get(&app, uri, Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.headers[header::CONTENT_TYPE], "application/pdf");
    assert_eq!(
        reply.headers[header::CONTENT_DISPOSITION],
        "attachment; filename=\"availability-2024-10.pdf\""
    );
    assert!(reply.bytes.starts_with(b"%PDF-1.4"));
    let text = String::from_utf8_lossy(&reply.bytes);
    assert!(text.contains("2024-10-01 to 2024-10-31 \\(UTC\\)"), "{text}");
}

#[tokio::test]
async fn the_default_period_is_the_previous_calendar_month() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let reply = get(&app, "/api/reports/sla?format=csv", Some(&admin)).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.headers[header::CONTENT_TYPE], "text/csv; charset=utf-8");
    let disposition = reply.headers[header::CONTENT_DISPOSITION].to_str().unwrap();
    assert!(disposition.contains("availability-") && disposition.ends_with("-monitors.csv\""));
    let text = String::from_utf8_lossy(&reply.bytes);
    assert!(text.starts_with("\u{feff}group,monitor,availability_percent"), "{text}");
}

#[tokio::test]
async fn the_csv_lists_monitors_and_the_incident_csv_has_its_header() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let created = app
        .post("/api/targets", json!({ "name": "NAS, main", "address": "10.0.0.5", "kind": "dummy" }), Some(&admin))
        .await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
    let monitors = get(&app, "/api/reports/sla?from=2024-10-01&to=2024-10-31&format=csv", Some(&admin)).await;
    let text = String::from_utf8_lossy(&monitors.bytes);
    assert!(text.contains("\"NAS, main\""), "{text}");
    let incidents =
        get(&app, "/api/reports/sla?from=2024-10-01&to=2024-10-31&format=incidents_csv", Some(&admin)).await;
    let text = String::from_utf8_lossy(&incidents.bytes);
    assert!(text.contains("monitor,rule,severity,started_utc"), "{text}");
}

#[tokio::test]
async fn bad_requests_are_refused_with_a_reason() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    for uri in [
        "/api/reports/sla?from=2024-10-31&to=2024-10-01",
        "/api/reports/sla?from=2023-01-01&to=2024-12-31",
        "/api/reports/sla?from=2024-10-01&to=2024-10-31&format=docx",
        "/api/reports/sla?from=2024-10-01&to=2024-10-31&tz=Mars/Olympus",
        "/api/reports/sla?from=2024-10-01&to=2024-10-31&objective=0",
        "/api/reports/sla?from=2024-10-01&to=2024-10-31&objective=101",
        "/api/reports/sla?from=2024-10-01&to=2024-10-31&page=9999",
        "/api/reports/sla?from=yesterday",
        "/api/reports/sla?to=2024-10-31",
        "/api/reports/sla?from=2099-01-01&to=2099-01-31",
    ] {
        let reply = get(&app, uri, Some(&admin)).await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{uri}");
    }
}

#[tokio::test]
async fn only_administrators_can_download() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let viewer = app.viewer_cookie(&admin).await;
    let uri = "/api/reports/sla?from=2024-10-01&to=2024-10-31";
    assert_eq!(get(&app, uri, None).await.status, StatusCode::UNAUTHORIZED);
    assert_eq!(get(&app, uri, Some(&viewer)).await.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn an_sla_schedule_is_monthly_and_keeps_its_options() {
    let app = TestApp::configured().await;
    let admin = app.admin_cookie().await;
    let weekly = app
        .post(
            "/api/reports/schedules",
            json!({ "name": "SLA", "kind": "sla", "frequency": "weekly", "enabled": false }),
            Some(&admin),
        )
        .await;
    assert_eq!(weekly.status, StatusCode::BAD_REQUEST, "{}", weekly.body);

    let page = app
        .post("/api/status-pages", json!({ "title": "Acme", "slug": "acme" }), Some(&admin))
        .await;
    let page_id = page.body["id"].as_i64().unwrap();
    let created = app
        .post(
            "/api/reports/schedules",
            json!({ "name": "Monthly SLA", "kind": "sla", "frequency": "monthly", "day_of_month": 2,
                    "enabled": false, "attach_csv": false, "objective": 99.9, "page_id": page_id }),
            Some(&admin),
        )
        .await;
    assert_eq!(created.status, StatusCode::CREATED, "{}", created.body);
    assert_eq!(created.body["kind"], "sla");
    assert_eq!(created.body["attach_pdf"], true);
    assert_eq!(created.body["attach_csv"], false);
    assert_eq!(created.body["objective"], 99.9);
    assert_eq!(created.body["page_id"], page_id);

    let digest = app.post("/api/reports/schedules", json!({ "name": "Weekly", "enabled": false }), Some(&admin)).await;
    assert_eq!(digest.body["kind"], "digest", "the default is the summary");

    for bad in [json!({ "kind": "other" }), json!({ "objective": 150 }), json!({ "page_id": 9999 })] {
        let mut body = json!({ "name": "x", "enabled": false, "frequency": "monthly" });
        body.as_object_mut().unwrap().extend(bad.as_object().unwrap().clone());
        let reply = app.post("/api/reports/schedules", body, Some(&admin)).await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{bad}");
    }
}
