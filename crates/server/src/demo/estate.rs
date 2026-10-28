//! Le parc fictif du mode démonstration : de faux équipements servis en
//! mémoire, sur `127.0.0.1`, que les **vrais** collecteurs interrogent.
//!
//! Chaque faux répond à partir des réponses réelles enregistrées dans les tests
//! des collecteurs (`crates/collectors/src/*/testdata`), embarquées dans le
//! binaire : l'image est `FROM scratch`, rien n'est lu sur le disque. Les
//! horodatages sont recalés sur l'heure courante à chaque réponse, de sorte que
//! le parc ne vieillit pas : deux démarrages donnent le même état.
//!
//! Les adresses affichées sont des noms en `.home.arpa` sans port ; le client
//! HTTP partagé des collecteurs les résout vers le port local du faux
//! (`dumbmonit_collectors::http::set_resolve_overrides`).

use std::net::SocketAddr;

use anyhow::Context;
use axum::Router;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use dumbmonit_proto::Credential;
use serde_json::Value;

mod graph;
mod kopia;
mod omada;
mod opnsense;
mod pbs;
mod pve;
mod pve_files;
mod redfish;
mod synology;
mod truenas;
mod unifi;
mod veeam;

/// Types de cible des équipements simulés ci-dessous.
pub const KINDS: &[&str] = &[
    "proxmox", "synology", "pbs", "truenas", "opnsense", "redfish", "unifi", "kopia", "veeam",
    "omada", "m365", "entra",
];

/// Équipements du bureau du client PME (Muller Architectes, Genève), interrogés
/// par l'agent relais posé sur place (voir `super::workspace`).
pub const OFFICE_DEVICES: &[&str] = &["vbr01.muller.home.arpa", "omada.muller.home.arpa"];
/// Locataire Microsoft du client PME.
pub const M365_NAME: &str = "Microsoft 365 (muller-archi.ch)";
pub const ENTRA_NAME: &str = "Entra ID (muller-archi.ch)";
/// Adresse du faux Microsoft Graph : connexion et lectures sur le même hôte.
const GRAPH: &str = "http://graph.muller.home.arpa";

/// Un équipement du parc fictif, tel que la base doit l'enregistrer.
pub struct DemoDevice {
    /// Identifiant stable, repris par l'amorçage de la base.
    pub key: &'static str,
    pub name: &'static str,
    pub kind: &'static str,
    pub address: String,
    pub tags: Vec<(&'static str, String)>,
    pub credential: Credential,
    pub interval_secs: u64,
}

pub struct Estate {
    pub devices: Vec<DemoDevice>,
    /// Nom d'hôte → adresse locale du faux qui le sert.
    pub resolve: Vec<(String, SocketAddr)>,
}

/// Lance tous les faux équipements et décrit le parc.
pub async fn start() -> anyhow::Result<Estate> {
    let fakes: [(DemoDevice, Router); 12] = [
        (
            DemoDevice {
                key: "pve",
                name: "pve.home.arpa",
                kind: "proxmox",
                address: "http://pve.home.arpa".into(),
                tags: vec![],
                credential: Credential::ApiToken {
                    token: "monitoring@pve!dumbmonit=00000000-0000-0000-0000-000000000000".into(),
                },
                interval_secs: 60,
            },
            pve::router(),
        ),
        (
            DemoDevice {
                key: "nas",
                name: "nas.home.arpa",
                kind: "synology",
                address: "http://nas.home.arpa".into(),
                tags: vec![("scheme", "http".into())],
                credential: Credential::UsernamePassword {
                    username: synology::USERNAME.into(),
                    password: synology::PASSWORD.into(),
                },
                interval_secs: 60,
            },
            synology::router(),
        ),
        (
            DemoDevice {
                key: "pbs",
                name: "pbs.home.arpa",
                kind: "pbs",
                address: "http://pbs.home.arpa".into(),
                tags: vec![],
                credential: Credential::ApiToken {
                    token: "monitoring@pbs!dumbmonit=00000000-0000-0000-0000-000000000000".into(),
                },
                interval_secs: 60,
            },
            pbs::router(),
        ),
        (
            DemoDevice {
                key: "truenas",
                name: "truenas.home.arpa",
                kind: "truenas",
                address: "http://truenas.home.arpa".into(),
                tags: vec![("scheme", "http".into())],
                credential: Credential::ApiToken { token: "1-demonstration".into() },
                interval_secs: 60,
            },
            truenas::router(),
        ),
        (
            DemoDevice {
                key: "fw",
                name: "fw.home.arpa",
                kind: "opnsense",
                address: "http://fw.home.arpa".into(),
                tags: vec![("scheme", "http".into()), ("live_map", "true".into())],
                credential: Credential::UsernamePassword {
                    username: "demonstration-key".into(),
                    password: "demonstration-secret".into(),
                },
                interval_secs: 60,
            },
            opnsense::router(),
        ),
        (
            DemoDevice {
                key: "bmc",
                name: "bmc-r740.home.arpa",
                kind: "redfish",
                address: "http://bmc-r740.home.arpa".into(),
                tags: vec![],
                credential: Credential::UsernamePassword {
                    username: "monitoring".into(),
                    password: "demonstration".into(),
                },
                interval_secs: 60,
            },
            redfish::router(),
        ),
        (
            DemoDevice {
                key: "unifi",
                name: "unifi.home.arpa",
                kind: "unifi",
                address: "http://unifi.home.arpa".into(),
                tags: vec![],
                credential: Credential::ApiToken { token: unifi::KEY.into() },
                interval_secs: 60,
            },
            unifi::router(),
        ),
        (
            DemoDevice {
                key: "kopia",
                name: "kopia.home.arpa",
                kind: "kopia",
                address: "http://kopia.home.arpa".into(),
                tags: vec![],
                credential: Credential::UsernamePassword {
                    username: "kopia".into(),
                    password: "demonstration".into(),
                },
                interval_secs: 300,
            },
            kopia::router(),
        ),
        (
            DemoDevice {
                key: "veeam",
                name: "vbr01.muller.home.arpa",
                kind: "veeam",
                address: "http://vbr01.muller.home.arpa".into(),
                tags: vec![],
                credential: Credential::UsernamePassword {
                    username: r"MULLER\svc-dumbmonit".into(),
                    password: "demonstration".into(),
                },
                interval_secs: 300,
            },
            veeam::router(),
        ),
        (
            DemoDevice {
                key: "omada",
                name: "omada.muller.home.arpa",
                kind: "omada",
                address: "http://omada.muller.home.arpa".into(),
                tags: vec![],
                credential: Credential::UsernamePassword {
                    username: "demonstration-client".into(),
                    password: "demonstration-secret".into(),
                },
                interval_secs: 120,
            },
            omada::router(),
        ),
        (
            DemoDevice {
                key: "m365",
                name: M365_NAME,
                kind: "m365",
                address: GRAPH.into(),
                tags: vec![("tenant_id", graph::TENANT.into()), ("login_url", GRAPH.into())],
                credential: Credential::UsernamePassword {
                    username: "00000000-0000-0000-0000-00000000d3m0".into(),
                    password: "demonstration".into(),
                },
                interval_secs: 300,
            },
            graph::router(),
        ),
        (
            DemoDevice {
                key: "entra",
                name: ENTRA_NAME,
                kind: "entra",
                address: GRAPH.into(),
                tags: vec![("tenant_id", graph::TENANT.into()), ("login_url", GRAPH.into())],
                credential: Credential::UsernamePassword {
                    username: "00000000-0000-0000-0000-00000000d3m0".into(),
                    password: "demonstration".into(),
                },
                interval_secs: 900,
            },
            graph::router(),
        ),
    ];

    let mut devices = Vec::new();
    let mut resolve = Vec::new();
    for (device, router) in fakes {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .with_context(|| format!("binding the demo device {}", device.key))?;
        let local = listener.local_addr()?;
        tokio::spawn(async move {
            if let Err(error) = axum::serve(listener, router).await {
                tracing::warn!(%error, "faux équipement de démonstration arrêté");
            }
        });
        let host = device
            .address
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .trim_end_matches('/')
            .to_string();
        resolve.push((host, local));
        devices.push(device);
    }
    Ok(Estate { devices, resolve })
}

/// Réponse JSON d'un faux, telle quelle.
fn json(status: StatusCode, body: String) -> Response {
    (status, [(axum::http::header::CONTENT_TYPE, "application/json")], body).into_response()
}

/// Clés dont la valeur est un instant Unix, en secondes, à recaler.
fn is_time_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    [
        "time",
        "run",
        "date",
        "notafter",
        "notbefore",
        "expire",
        "last",
        "boot",
        "created",
        "ctime",
        "mtime",
        "end",
        "start",
        "since",
    ]
    .iter()
    .any(|word| key.contains(word))
}

/// Recale de `offset` secondes tout instant Unix d'un document : un nombre de
/// l'ordre de 2020–2033 sous une clé qui parle de temps. Les instants en
/// millisecondes (`$date` de TrueNAS) sont traités de même.
fn shift_times(value: &mut Value, offset: i64) {
    const SECONDS: std::ops::Range<i64> = 1_577_836_800..2_000_000_000;
    const MILLIS: std::ops::Range<i64> = 1_577_836_800_000..2_000_000_000_000;
    match value {
        Value::Array(items) => items.iter_mut().for_each(|item| shift_times(item, offset)),
        Value::Object(map) => {
            for (key, item) in map.iter_mut() {
                let wanted = is_time_key(key) || key == "$date";
                match item.as_i64() {
                    Some(n) if wanted && SECONDS.contains(&n) => *item = Value::from(n + offset),
                    Some(n) if wanted && MILLIS.contains(&n) => {
                        *item = Value::from(n + offset * 1000)
                    }
                    _ => shift_times(item, offset),
                }
            }
        }
        _ => {}
    }
}

/// Recale de `offset` secondes toute date RFC 3339 écrite en texte
/// (`2026-09-30T01:00:12.31+02:00`), quelle que soit sa clé : Veeam, Kopia et
/// Microsoft Graph datent tout ainsi.
fn shift_iso(value: &mut Value, offset: i64) {
    match value {
        Value::Array(items) => items.iter_mut().for_each(|item| shift_iso(item, offset)),
        Value::Object(map) => map.values_mut().for_each(|item| shift_iso(item, offset)),
        Value::String(text) => {
            let looks_like_date = text.len() >= 20
                && text.as_bytes()[4] == b'-'
                && text.as_bytes()[10] == b'T'
                && text.as_bytes()[..4].iter().all(u8::is_ascii_digit);
            if looks_like_date && let Ok(at) = chrono::DateTime::parse_from_rfc3339(text) {
                let moved = at.with_timezone(&chrono::Utc) + chrono::Duration::seconds(offset);
                *text = moved.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
            }
        }
        _ => {}
    }
}

/// Une réponse enregistrée, mots remplacés et dates recalées : capturée à
/// `capture_s`, elle a l'air d'avoir été lue à l'instant.
fn replay(body: &str, capture_s: i64, words: &[(&str, &str)]) -> Response {
    let mut text = body.to_string();
    for (from, to) in words {
        text = text.replace(from, to);
    }
    match serde_json::from_str::<Value>(&text) {
        Ok(mut doc) => {
            let offset = now_s() - capture_s;
            shift_times(&mut doc, offset);
            shift_iso(&mut doc, offset);
            json(StatusCode::OK, doc.to_string())
        }
        Err(_) => json(StatusCode::OK, text),
    }
}

/// Instant RFC 3339 d'une capture, en secondes Unix.
fn instant(text: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(text).map(|at| at.timestamp()).unwrap_or(0)
}

/// Remplace chaque occurrence de `from` qui n'est pas collée à une lettre ou un
/// chiffre : `AIR01` ne touche pas `AIR01n`, `node1` ne touche pas `node10`.
fn replace_word(text: &str, from: &str, to: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    let is_word = |c: Option<char>| c.is_some_and(|c| c.is_ascii_alphanumeric());
    while let Some(pos) = rest.find(from) {
        out.push_str(&rest[..pos]);
        let after = rest[pos + from.len()..].chars().next();
        if is_word(out.chars().last()) || is_word(after) {
            out.push_str(from);
        } else {
            out.push_str(to);
        }
        rest = &rest[pos + from.len()..];
    }
    out.push_str(rest);
    out
}

/// Niveau d'activité de l'équipement `name` à l'instant, dans [0, 1] : la même
/// journée que les sondes synthétiques.
fn synthetic_load(name: &str) -> f64 {
    let now = chrono::Utc::now().timestamp_millis();
    super::synthetic::activity(super::synthetic::seed(name), now)
}

/// L'heure courante, en secondes Unix.
fn now_s() -> i64 {
    chrono::Utc::now().timestamp()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn le_remplacement_respecte_les_mots() {
        assert_eq!(
            replace_word("\"AIR01\" \"AIR01n\"", "AIR01", "adguard"),
            "\"adguard\" \"AIR01n\""
        );
        assert_eq!(replace_word("node/node1 node10", "node1", "pve1"), "node/pve1 node10");
    }

    #[test]
    fn les_dates_en_texte_sont_recalees() {
        let mut doc = json!({"lastRun": "2026-09-30T01:00:12+02:00", "name": "2026 plan"});
        shift_iso(&mut doc, 3600);
        assert_eq!(doc["lastRun"], "2026-09-30T00:00:12Z");
        assert_eq!(doc["name"], "2026 plan");
    }

    #[test]
    fn seuls_les_instants_sont_recales() {
        let mut doc =
            json!({"starttime": 1_790_000_000, "size": 1_790_000_000, "x": [{"endtime": 5}]});
        shift_times(&mut doc, 10);
        assert_eq!(
            doc,
            json!({"starttime": 1_790_000_010, "size": 1_790_000_000, "x": [{"endtime": 5}]})
        );
    }
}
