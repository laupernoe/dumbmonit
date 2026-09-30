//! Faux cluster Proxmox VE : la capture `pve9` (trois nœuds, une trentaine de
//! VM) rejouée, nœuds et invités renommés à la mode d'un homelab.
//!
//! La capture ne couvre pas tous les nœuds ni toutes les VM : un point d'entrée
//! absent pour `pve1` est servi par celui de `pve3`, une VM sans capture par
//! celle d'une voisine, identité corrigée.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

use axum::Router;
use axum::http::{StatusCode, Uri};
use axum::response::Response;
use serde_json::{Value, json};

use super::pve_files::FILES;
use super::{json, now_s, replace_word, shift_times};

/// Instant de la capture : les horodatages sont décalés d'autant pour tomber
/// sur l'heure courante.
const CAPTURE_S: i64 = 1_790_263_400;

/// Noms d'origine de la capture → noms du parc fictif.
const RENAMES: &[(&str, &str)] = &[
    ("node1", "pve1"),
    ("node2", "pve2"),
    ("node3", "pve3"),
    ("MAIL01", "mail"),
    ("WEB01", "nextcloud"),
    ("SEG01", "vaultwarden"),
    ("ARCH01", "paperless"),
    ("AVE01", "immich"),
    ("MTB01", "jellyfin"),
    ("ASA01", "home-assistant"),
    ("LOG01", "grafana"),
    ("FW02", "opnsense-spare"),
    ("SAAS14", "gitea"),
    ("SAAS01", "authentik"),
    ("AIR01n", "adguard-2"),
    ("AIR01", "adguard"),
    ("SAAS04", "wiki"),
    ("FED02", "matrix"),
    ("ETR02", "mastodon"),
    ("AD01", "dc1"),
    ("AD02", "dc2"),
    ("SAAS15", "n8n"),
    ("3RO01", "minecraft"),
    ("LAC01", "unifi"),
    ("FW01", "wireguard"),
    ("NOV02", "frigate"),
    ("BKP01", "pbs-backup"),
    ("MIS01", "mealie"),
    ("LIC01", "k3s-1"),
    ("LIC02", "k3s-2"),
    ("ECO01", "homepage"),
    ("CNT01", "docker"),
    ("MACHINE-1", "pve1.home.arpa"),
    ("ISSUER-1", "Home CA"),
];

struct Capture {
    /// Point d'entrée encodé → corps renommé.
    bodies: HashMap<String, String>,
    /// VMID → (nœud d'origine, nom affiché).
    guests: HashMap<u64, (String, String)>,
}

static CAPTURE: LazyLock<Arc<Capture>> = LazyLock::new(|| {
    let bodies: HashMap<String, String> =
        FILES.iter().map(|(slug, body)| (slug.to_string(), rename(body))).collect();
    let mut guests = HashMap::new();
    if let Some(resources) = bodies.get("cluster-resources")
        && let Ok(doc) = serde_json::from_str::<Value>(resources)
    {
        for entry in doc["data"].as_array().into_iter().flatten() {
            if let (Some(vmid), Some(node), Some(name)) =
                (entry["vmid"].as_u64(), entry["node"].as_str(), entry["name"].as_str())
            {
                guests.insert(vmid, (node.to_string(), name.to_string()));
            }
        }
    }
    Arc::new(Capture { bodies, guests })
});

/// Renomme un corps de la capture, adresses pseudonymisées comprises
/// (`IP-4` → `10.0.10.14`).
fn rename(body: &str) -> String {
    let mut text = body.to_string();
    for (from, to) in RENAMES {
        text = replace_word(&text, from, to);
    }
    for n in (1..=40).rev() {
        text = replace_word(&text, &format!("IP-{n}"), &format!("10.0.10.{}", 10 + n));
    }
    text
}

pub(super) fn router() -> Router {
    LazyLock::force(&CAPTURE);
    Router::new().fallback(handle)
}

async fn handle(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches("/api2/json").trim_matches('/');
    let query = uri.query().unwrap_or_default();
    match lookup(path, query) {
        Some((body, true)) => json(StatusCode::OK, body),
        Some((body, false)) => replay(&body),
        None => json(
            StatusCode::NOT_IMPLEMENTED,
            json!({"data": null, "message": format!("Method 'GET /{path}' not implemented")})
                .to_string(),
        ),
    }
}

/// Le chemin tel que la capture le nomme : séparateurs en `-`, sans répétition.
fn slug(path: &str, query: &str) -> String {
    let raw = if query.is_empty() { path.to_string() } else { format!("{path}?{query}") };
    let mut out = String::new();
    for c in raw.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

/// Cherche le corps d'un point d'entrée, avec les replis décrits en tête ; le
/// booléen dit s'il est déjà daté d'aujourd'hui (construit ici) ou s'il sort
/// de la capture et doit être recalé.
fn lookup(path: &str, query: &str) -> Option<(String, bool)> {
    let capture = &*CAPTURE;
    // Les chemins arrivent avec les noms du parc : on revient à ceux de la capture
    // pour trouver le fichier.
    let mut segments: Vec<String> = path.split('/').map(str::to_string).collect();
    let renamed = segments.clone();
    for index in 1..segments.len() {
        let is_name = (index == 1 && segments[0] == "nodes") || segments[index - 1] == "storage";
        if let (true, Some((original, _))) =
            (is_name, RENAMES.iter().find(|(_, renamed)| *renamed == segments[index]))
        {
            segments[index] = original.to_string();
        }
    }
    if let Some(found) = synthetic(&renamed, query) {
        return Some(found);
    }
    let direct = slug(&segments.join("/"), query);
    if let Some(body) = capture.bodies.get(&direct) {
        return Some((body.clone(), false));
    }
    // `/version` n'a pas été capturé : c'est celui d'un nœud.
    if direct == "version" {
        return capture.bodies.get("nodes-node3-version").map(|body| (body.clone(), false));
    }

    // `nodes/<nœud>/…` : le même point d'entrée sur un autre nœud.
    if segments.first().map(String::as_str) != Some("nodes") || segments.len() < 3 {
        return None;
    }
    let vmid = match segments.get(2).map(String::as_str) {
        Some("qemu" | "lxc") => segments.get(3).and_then(|id| id.parse::<u64>().ok()),
        _ => None,
    };
    let mut candidates = Vec::new();
    for node in ["node3", "node2", "node1"] {
        let mut alt = segments.clone();
        alt[1] = node.to_string();
        if vmid.is_some() {
            // Une VM sans capture emprunte celle d'une voisine.
            for other in capture.guests.keys() {
                alt[3] = other.to_string();
                candidates.push(slug(&alt.join("/"), query));
            }
        } else {
            candidates.push(slug(&alt.join("/"), query));
        }
    }
    candidates.sort();
    let body = candidates.iter().find_map(|slug| capture.bodies.get(slug))?;
    let Some(vmid) = vmid else { return Some((body.clone(), false)) };

    // Identité de la VM demandée, pas de celle dont on a pris la réponse.
    let mut doc: Value = serde_json::from_str(body).ok()?;
    if let Some(data) = doc.get_mut("data").and_then(Value::as_object_mut) {
        if data.contains_key("vmid") {
            data.insert("vmid".into(), json!(vmid));
        }
        if let (true, Some((_, name))) = (data.contains_key("name"), capture.guests.get(&vmid)) {
            data.insert("name".into(), json!(name));
        }
    }
    Some((doc.to_string(), false))
}

/// Points d'entrée que la capture ne couvre pas, construits ici. `path` porte
/// les noms du parc (`pve1`, `pbs-backup`).
fn synthetic(path: &[String], query: &str) -> Option<(String, bool)> {
    let capture = &*CAPTURE;
    let parts: Vec<&str> = path.iter().map(String::as_str).collect();
    let now = now_s();
    let night = now - now.rem_euclid(86_400);
    let data = |value: Value| Some((json!({ "data": value }).to_string(), true));
    // Invités d'un nœud, par VMID.
    let guests_on = |node: &str| {
        let mut ids: Vec<u64> = capture
            .guests
            .iter()
            .filter(|(_, (host, _))| host == node)
            .map(|(vmid, _)| *vmid)
            .collect();
        ids.sort_unstable();
        ids
    };
    match parts.as_slice() {
        ["cluster", "backup-info", "not-backed-up"] => data(json!([])),
        // Pas de Ceph dans ce cluster : chaque appel le dit comme `ceph/status`.
        ["cluster", "ceph", ..] => {
            capture.bodies.get("cluster-ceph-status").map(|body| (body.clone(), false))
        }
        ["nodes", node, "tasks"] if query.contains("typefilter=vzdump") => {
            let tasks: Vec<Value> = guests_on(node)
                .into_iter()
                .enumerate()
                .flat_map(|(i, vmid)| {
                    (0..7).map(move |day| {
                        let start = night - day * 86_400 + 3600 + i as i64 * 120;
                        (vmid, start)
                    })
                })
                .filter(|(_, start)| *start <= now)
                .map(|(vmid, start)| {
                    json!({
                        "upid": format!("UPID:{node}:0001A2B3:00C0FFEE:{start:08X}:vzdump:{vmid}:root@pam:"),
                        "type": "vzdump", "id": vmid.to_string(), "node": node, "user": "root@pam",
                        "starttime": start, "endtime": start + 95, "status": "OK", "pstart": 1
                    })
                })
                .collect();
            data(Value::Array(tasks))
        }
        ["nodes", _, "apt", "repositories"] => data(json!({
            "digest": "0d1e2m3o",
            "errors": [],
            "files": [{"path": "/etc/apt/sources.list.d/proxmox.sources", "file-type": "sources",
                "repositories": [{"Enabled": 1, "Types": ["deb"],
                    "URIs": ["http://download.proxmox.com/debian/pve"], "Suites": ["trixie"],
                    "Components": ["pve-no-subscription"], "FileType": "sources"}]}],
            "infos": [{"path": "/etc/apt/sources.list.d/proxmox.sources", "index": 0,
                "property": "Components", "kind": "badge",
                "message": "The no-subscription repository is not recommended for production use!"}],
            "standard-repos": [
                {"handle": "enterprise", "name": "Enterprise", "status": 0},
                {"handle": "no-subscription", "name": "No-Subscription", "status": 1}
            ]
        })),
        ["nodes", _, "disks", "smart"] => {
            let slug = if query.contains("nvme") {
                "nodes-node3-disks-smart-disk-2Fdev-2Fnvme0n1"
            } else {
                "nodes-node2-disks-smart-disk-2Fdev-2Fsda"
            };
            capture.bodies.get(slug).map(|body| (body.clone(), false))
        }
        ["nodes", _, "qemu", vmid, "agent", "get-fsinfo"] => {
            let vmid: i64 = vmid.parse().ok()?;
            let total: i64 = 32 * 1_073_741_824 + (vmid % 7) * 16 * 1_073_741_824;
            let used = total / 100 * (35 + vmid % 40);
            data(json!({"result": [
                {"name": "sda1", "mountpoint": "/", "type": "ext4", "total-bytes": total,
                 "used-bytes": used, "disk": [{"bus-type": "scsi", "dev": "/dev/sda1",
                 "serial": "drive-scsi0"}]},
                {"name": "tmpfs", "mountpoint": "/run", "type": "tmpfs",
                 "total-bytes": 203_984_896, "used-bytes": 1_064_960, "disk": []}
            ]}))
        }
        ["nodes", node, "storage", storage, "content"] => {
            if *storage != "pbs-backup" {
                return data(json!([]));
            }
            let volumes: Vec<Value> = guests_on(node)
                .into_iter()
                .enumerate()
                .flat_map(|(i, vmid)| {
                    (0..7).map(move |day| (vmid, night - day * 86_400 + 3600 + i as i64 * 120))
                })
                .filter(|(_, ctime)| *ctime <= now)
                .map(|(vmid, ctime)| {
                    let stamp = chrono::DateTime::from_timestamp(ctime, 0)
                        .map(|t| t.format("%Y-%m-%dT%H:%M:%SZ").to_string())
                        .unwrap_or_default();
                    json!({
                        "volid": format!("pbs-backup:backup/vm/{vmid}/{stamp}"),
                        "format": "pbs-vm", "content": "backup", "ctime": ctime,
                        "size": 8_589_934_592_i64 + (vmid as i64 % 9) * 2_147_483_648,
                        "vmid": vmid, "subtype": "qemu", "verification": {"state": "ok"}
                    })
                })
                .collect();
            data(Value::Array(volumes))
        }
        _ => None,
    }
}

/// Sert un corps de la capture : une réponse en échec est rejouée avec son
/// statut, une réponse réussie avec ses horodatages ramenés à maintenant.
fn replay(body: &str) -> Response {
    let Ok(mut doc) = serde_json::from_str::<Value>(body) else {
        return json(StatusCode::OK, body.to_string());
    };
    if let Some(status) = doc.get("_http_status").and_then(Value::as_u64) {
        let status = StatusCode::from_u16(status as u16).unwrap_or(StatusCode::NOT_IMPLEMENTED);
        let text = doc.get("_body").and_then(Value::as_str).unwrap_or_default().to_string();
        return json(status, json!({"data": null, "message": text}).to_string());
    }
    shift_times(&mut doc, now_s() - CAPTURE_S);
    json(StatusCode::OK, doc.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_chemins_retrouvent_leur_capture() {
        assert_eq!(
            slug("nodes/node3/disks/smart", "disk=%2Fdev%2Fnvme0n1"),
            "nodes-node3-disks-smart-disk-2Fdev-2Fnvme0n1"
        );
        assert!(lookup("nodes/pve3/status", "").is_some());
        // pve1 n'a pas de statut capturé : celui de pve3 le remplace.
        assert!(lookup("nodes/pve1/status", "").is_some());
        let (body, _) = lookup("nodes/pve1/qemu/100/status/current", "").unwrap();
        assert!(body.contains("\"mail\""), "{body}");
    }
}
