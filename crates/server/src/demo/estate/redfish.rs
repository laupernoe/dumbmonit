//! Faux contrôleur de gestion Redfish : le mockup DMTF des tests du
//! collecteur (`redfish/fixtures`), ramené à une seule panne — une
//! alimentation sur deux hors service, redondance perdue.

use std::sync::LazyLock;

use axum::Router;
use axum::http::{Method, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use serde_json::{Map, Value, json};

use super::json;

const FIXTURE: &str =
    include_str!("../../../../collectors/src/redfish/fixtures/dmtf-localstorage-degraded.json");

const SYSTEM: &str = "/redfish/v1/Systems/437XR1138R2";
const THERMAL: &str = "/redfish/v1/Chassis/1U/Thermal";
const POWER: &str = "/redfish/v1/Chassis/1U/Power";
const DRIVE: &str = "/redfish/v1/Chassis/1U/Drives/3F5A8C54207B7233";

static RESOURCES: LazyLock<Map<String, Value>> = LazyLock::new(|| {
    let mut doc: Value = serde_json::from_str(FIXTURE).unwrap_or_default();
    let mut resources = doc["resources"].as_object_mut().map(std::mem::take).unwrap_or_default();

    // Le serveur, sous son nom du parc.
    if let Some(system) = resources.get_mut(SYSTEM) {
        system["HostName"] = json!("r740.home.arpa");
        system["Name"] = json!("r740");
        system["Status"] =
            json!({"Health": "Warning", "HealthRollup": "Warning", "State": "Enabled"});
    }
    // Ventilateurs et températures sains : la panne est ailleurs.
    if let Some(thermal) = resources.get_mut(THERMAL) {
        if let Some(fan) = thermal.pointer_mut("/Fans/1") {
            fan["Reading"] = json!(2050);
            fan["Status"] = json!({"Health": "OK", "State": "Enabled"});
        }
        if let Some(cpu) = thermal.pointer_mut("/Temperatures/0") {
            cpu["ReadingCelsius"] = json!(39);
        }
    }
    // Deux alimentations, la première en panne.
    if let Some(power) = resources.get_mut(POWER)
        && let Some(first) = power.pointer("/PowerSupplies/0").cloned()
    {
        let mut second = first;
        second["@odata.id"] = json!(format!("{POWER}#/PowerSupplies/1"));
        second["MemberId"] = json!("1");
        second["Name"] = json!("Power Supply Bay 2");
        second["SerialNumber"] = json!("1Z0000002");
        second["LastPowerOutputWatts"] = json!(344);
        second["Status"] = json!({"Health": "OK", "State": "Enabled"});
        power["PowerSupplies"][0]["LastPowerOutputWatts"] = json!(0);
        power["PowerSupplies"][1] = second;
    }
    // Le disque qui annonçait sa fin est sain dans le parc fictif.
    if let Some(drive) = resources.get_mut(DRIVE) {
        drive["FailurePredicted"] = json!(false);
        drive["Status"] = json!({"Health": "OK", "State": "Enabled"});
    }
    resources
});

pub(super) fn router() -> Router {
    LazyLock::force(&RESOURCES);
    Router::new().fallback(handle)
}

async fn handle(method: Method, uri: Uri) -> Response {
    let path = uri.path().trim_end_matches('/');
    if path.ends_with("/SessionService/Sessions") && method == Method::POST {
        let location = format!("{path}/demonstration");
        return (
            StatusCode::CREATED,
            [
                ("x-auth-token", "demonstration-session"),
                (header::LOCATION.as_str(), location.as_str()),
                (header::CONTENT_TYPE.as_str(), "application/json"),
            ],
            json!({"Id": "demonstration"}).to_string(),
        )
            .into_response();
    }
    if method == Method::DELETE {
        return StatusCode::NO_CONTENT.into_response();
    }
    let found = RESOURCES.iter().find(|(key, _)| key.eq_ignore_ascii_case(path));
    match found {
        Some((_, value)) => json(StatusCode::OK, value.to_string()),
        None => json(StatusCode::NOT_FOUND, "{}".into()),
    }
}
