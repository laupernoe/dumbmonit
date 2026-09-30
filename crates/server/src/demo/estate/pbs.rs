//! Faux Proxmox Backup Server. Aucune capture réelle n'existe dans le dépôt :
//! les réponses sont construites ici, dans la forme de l'API PBS 4, et datées
//! par rapport à l'heure courante — sauvegardes nocturnes des VM du cluster
//! fictif, vérification hebdomadaire, ramasse-miettes quotidien.

use axum::Router;
use axum::http::{StatusCode, Uri};
use axum::response::Response;
use serde_json::{Value, json};

use super::{json, now_s};

const STORE: &str = "main";
const OFFSITE: &str = "offsite";
const TIB: i64 = 1_099_511_627_776;
const GIB: i64 = 1_073_741_824;

/// Invités sauvegardés chaque nuit : `(type, identifiant)`.
const GUESTS: &[(&str, &str)] = &[
    ("vm", "100"),
    ("vm", "101"),
    ("vm", "102"),
    ("vm", "103"),
    ("vm", "104"),
    ("vm", "105"),
    ("vm", "106"),
    ("vm", "107"),
    ("vm", "110"),
    ("vm", "118"),
    ("vm", "122"),
    ("vm", "128"),
];

pub(super) fn router() -> Router {
    Router::new().fallback(handle)
}

fn data(value: Value) -> Response {
    json(StatusCode::OK, json!({ "data": value }).to_string())
}

async fn handle(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches("/api2/json").trim_end_matches('/');
    let now = now_s();
    let night = last_night(now);
    match path {
        "/version" => data(json!({"version": "4.0.14", "release": "4.0", "repoid": "a1b2c3d4"})),
        "/nodes/localhost/status" => data(json!({
            "uptime": 3_888_000, "cpu": 0.07, "wait": 0.01, "loadavg": ["0.42", "0.35", "0.31"],
            "cpuinfo": {"cpus": 8, "model": "Intel(R) Xeon(R) E-2236 CPU @ 3.40GHz", "sockets": 1},
            "memory": {"total": 32 * GIB, "used": 9 * GIB, "free": 23 * GIB},
            "swap": {"total": 8 * GIB, "used": 0, "free": 8 * GIB},
            "root": {"total": 60 * GIB, "used": 7 * GIB, "avail": 53 * GIB},
            "kversion": "Linux 6.14.8-2-pve #1 SMP PREEMPT_DYNAMIC PMX 6.14.8-2"
        })),
        "/status/datastore-usage" => data(json!([
            {
                "store": STORE, "total": 8 * TIB, "used": 5 * TIB + 300 * GIB,
                "avail": 3 * TIB - 300 * GIB, "estimated-full-date": now + 210 * 86_400,
                "mount-status": "nonremovable", "backend-type": "filesystem",
                "history-delta": 86_400,
                "history": (0..30).map(|i| json!(0.60 + 0.0023 * i as f64)).collect::<Vec<_>>()
            },
            {
                "store": OFFSITE, "total": 12 * TIB, "used": 4 * TIB, "avail": 8 * TIB,
                "estimated-full-date": now + 900 * 86_400, "mount-status": "nonremovable",
                "backend-type": "filesystem", "history-delta": 86_400,
                "history": (0..30).map(|i| json!(0.32 + 0.0007 * i as f64)).collect::<Vec<_>>()
            }
        ])),
        "/nodes/localhost/tasks" => data(Value::Array(tasks(now))),
        "/admin/sync" => data(json!([{
            "id": "s-offsite", "store": OFFSITE, "remote": "", "remote-store": STORE,
            "schedule": "daily 05:00", "comment": "Nightly copy to the offsite disk",
            "next-run": night + 86_400 + 3 * 3600,
            "last-run-state": "OK", "last-run-endtime": night + 3 * 3600 + 1400,
            "last-run-upid": upid("syncjob", "offsite:s-offsite", night + 3 * 3600)
        }])),
        "/admin/verify" => data(json!([{
            "id": "v-weekly", "store": STORE, "schedule": "sat 06:00",
            "comment": "Weekly verification", "next-run": night + 5 * 86_400,
            "last-run-state": "OK", "last-run-endtime": night - 2 * 86_400 + 4 * 3600 + 2100,
            "last-run-upid": upid("verificationjob", "main:v-weekly", night - 2 * 86_400)
        }])),
        "/admin/prune" => data(json!([{
            "id": "p-main", "store": STORE, "schedule": "daily 04:00", "comment": "Retention",
            "keep-last": 3, "keep-daily": 7, "keep-weekly": 4, "keep-monthly": 6,
            "next-run": night + 86_400 + 2 * 3600,
            "last-run-state": "OK", "last-run-endtime": night + 2 * 3600 + 40,
            "last-run-upid": upid("prunejob", "main:p-main", night + 2 * 3600)
        }])),
        "/admin/gc" => data(json!([{
            "store": STORE, "schedule": "daily", "next-run": night + 86_400 + 4 * 3600,
            "last-run-state": "OK", "last-run-endtime": night + 4 * 3600 + 780,
            "last-run-upid": upid("garbage_collection", STORE, night + 4 * 3600),
            "duration": 780, "index-data-bytes": 38 * TIB, "disk-bytes": 5 * TIB + 300 * GIB,
            "removed-bytes": 41 * GIB, "pending-bytes": 3 * GIB, "disk-chunks": 3_120_442,
            "removed-chunks": 21_804, "pending-chunks": 1_512, "still-bad": 0
        }])),
        "/config/datastore" => data(
            json!([{"name": STORE, "path": "/mnt/datastore/main"}, {"name": OFFSITE, "path": "/mnt/datastore/offsite"}]),
        ),
        "/nodes/localhost/apt/update" => data(
            json!([{"Package": "proxmox-backup-server", "Version": "4.0.15-1", "OldVersion": "4.0.14-1"}]),
        ),
        "/nodes/localhost/apt/versions" => data(json!([
            {"Package": "proxmox-backup-server", "OldVersion": "4.0.14-1", "Version": "4.0.15-1",
             "Title": "Proxmox Backup Server", "ExtraInfo": "running version: 4.0.14"},
            {"Package": "proxmox-kernel-6.14", "OldVersion": "6.14.8-2", "Version": "6.14.8-2",
             "Title": "Latest Proxmox kernel", "ExtraInfo": "running kernel: 6.14.8-2-pve"}
        ])),
        "/nodes/localhost/disks/list" => data(json!([
            {"name": "nvme0n1", "devpath": "/dev/nvme0n1", "model": "Samsung SSD 980 PRO 500GB",
             "serial": "S5GXNX0T000001", "size": 500 * GIB, "disk-type": "nvme", "used": "LVM",
             "health": "PASSED", "status": "passed", "wearout": 96},
            {"name": "sda", "devpath": "/dev/sda", "model": "WDC WD80EFZZ-68BTXN0",
             "serial": "WD-CA000001", "size": 8 * TIB, "disk-type": "hdd", "used": "ZFS",
             "health": "PASSED", "status": "passed"},
            {"name": "sdb", "devpath": "/dev/sdb", "model": "WDC WD80EFZZ-68BTXN0",
             "serial": "WD-CA000002", "size": 8 * TIB, "disk-type": "hdd", "used": "ZFS",
             "health": "PASSED", "status": "passed"},
            {"name": "sdc", "devpath": "/dev/sdc", "model": "ST12000VN0008-2YS101",
             "serial": "ZRT00001", "size": 12 * TIB, "disk-type": "hdd", "used": "ext4",
             "health": "PASSED", "status": "passed"}
        ])),
        "/nodes/localhost/disks/zfs" => data(json!([{
            "name": "backup", "health": "ONLINE", "size": 8 * TIB, "alloc": 5 * TIB + 300 * GIB,
            "free": 3 * TIB - 300 * GIB, "frag": 9, "dedup": 1.0
        }])),
        "/nodes/localhost/disks/smart" => data(json!({
            "health": "PASSED", "type": "ata",
            "attributes": [
                {"id": " 5", "name": "Reallocated_Sector_Ct", "value": 100, "worst": 100, "threshold": 5, "raw": "0", "normalized": 100, "flags": "PO--CK", "fail": "-"},
                {"id": "194", "name": "Temperature_Celsius", "value": 64, "worst": 55, "threshold": 0, "raw": "36", "normalized": 64, "flags": "-O---K", "fail": "-"}
            ]
        })),
        "/nodes/localhost/services" => data(json!([
            {"service": "proxmox-backup", "name": "proxmox-backup", "desc": "Proxmox Backup API Server", "state": "running", "unit-state": "enabled"},
            {"service": "proxmox-backup-proxy", "name": "proxmox-backup-proxy", "desc": "Proxmox Backup API Proxy Server", "state": "running", "unit-state": "enabled"},
            {"service": "chrony", "name": "chrony", "desc": "chrony, an NTP client/server", "state": "running", "unit-state": "enabled"},
            {"service": "postfix", "name": "postfix", "desc": "Postfix Mail Transport Agent", "state": "running", "unit-state": "enabled"},
            {"service": "smartmontools", "name": "smartmontools", "desc": "Self Monitoring and Reporting Technology", "state": "running", "unit-state": "enabled"}
        ])),
        "/nodes/localhost/certificates/info" => data(json!([{
            "filename": "proxy.pem", "fingerprint": "5a:3c:00:de:ad:be:ef:00",
            "issuer": "/CN=Home CA", "subject": "/CN=pbs.home.arpa",
            "notafter": now + 64 * 86_400, "notbefore": now - 301 * 86_400,
            "san": ["DNS:pbs.home.arpa", "IP Address:10.0.10.40"]
        }])),
        "/admin/traffic-control" => data(json!([])),
        p if p.starts_with("/admin/datastore/") => datastore(p, now),
        p if p.starts_with("/nodes/localhost/tasks/") && p.ends_with("/log") => data(json!([
            {"n": 1, "t": "starting backup on datastore 'main'"},
            {"n": 2, "t": "upload of 1.2 GiB in 41 s (29.9 MiB/s)"},
            {"n": 3, "t": "TASK OK"}
        ])),
        _ => json(StatusCode::NOT_FOUND, json!({"data": null, "message": "not found"}).to_string()),
    }
}

/// Minuit (UTC) du jour courant.
fn last_night(now: i64) -> i64 {
    now - now.rem_euclid(86_400)
}

fn upid(worker_type: &str, worker_id: &str, start: i64) -> String {
    let id = worker_id.replace(':', "\\x3a").replace('/', "-");
    format!("UPID:pbs:0000A1B2:00C0FFEE:00000000:{start:08X}:{worker_type}:{id}:root@pam:")
}

fn task(worker_type: &str, worker_id: &str, start: i64, duration: i64, status: &str) -> Value {
    json!({
        "upid": upid(worker_type, worker_id, start), "worker_type": worker_type,
        "worker_id": worker_id, "user": "pve@pbs!pve1", "starttime": start,
        "endtime": start + duration, "status": status
    })
}

/// Trente jours de tâches, les plus récentes en premier : chaque nuit une
/// sauvegarde par invité à 1 h, l'élagage à 2 h, la copie hors site à 3 h, le
/// ramasse-miettes à 4 h ; la vérification le samedi.
fn tasks(now: i64) -> Vec<Value> {
    let today = last_night(now);
    let mut out = Vec::new();
    for day in 0..30 {
        let night = today - day * 86_400;
        for (i, (kind, id)) in GUESTS.iter().enumerate() {
            let start = night + 3600 + i as i64 * 180;
            if start > now {
                continue;
            }
            out.push(task("backup", &format!("{STORE}:{kind}/{id}"), start, 140, "OK"));
        }
        for (worker_type, worker_id, hour, duration) in [
            ("prunejob", "main:p-main", 2, 40),
            ("syncjob", "offsite:s-offsite", 3, 1400),
            ("garbage_collection", STORE, 4, 780),
        ] {
            let start = night + hour * 3600;
            if start <= now {
                out.push(task(worker_type, worker_id, start, duration, "OK"));
            }
        }
        // Le jour 2 est un samedi fictif : la vérification hebdomadaire.
        if day % 7 == 2 {
            out.push(task("verificationjob", "main:v-weekly", night + 4 * 3600, 2100, "OK"));
        }
    }
    out.sort_by_key(|t| std::cmp::Reverse(t["starttime"].as_i64().unwrap_or_default()));
    out
}

/// `/admin/datastore/<store>/…` : décomptes, opérations en cours, espaces de
/// noms et instantanés.
fn datastore(path: &str, now: i64) -> Response {
    let rest = path.trim_start_matches("/admin/datastore/");
    let (store, what) = rest.split_once('/').unwrap_or((rest, ""));
    let groups = if store == STORE { GUESTS.len() } else { GUESTS.len() / 2 };
    match what {
        "status" => data(json!({
            "counts": {"vm": {"groups": groups, "snapshots": groups * 16}}
        })),
        "active-operations" => data(json!({"read": 0, "write": 0})),
        "namespace" => data(json!([{"ns": ""}])),
        "snapshots" => {
            let night = last_night(now);
            let mut out = Vec::new();
            for (kind, id) in GUESTS.iter().take(groups) {
                for day in 0..7 {
                    out.push(json!({
                        "backup-type": kind, "backup-id": id,
                        "backup-time": night - day * 86_400 + 3600,
                        "size": 34 * GIB, "verification": {"state": "ok"}, "protected": false
                    }));
                }
            }
            data(Value::Array(out))
        }
        "gc" => data(json!({"store": store, "last-run-state": "OK"})),
        _ => json(StatusCode::NOT_FOUND, json!({"data": null}).to_string()),
    }
}
