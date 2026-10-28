//! Les scans de découverte planifiés et les appareils connus ou ignorés
//! voyagent avec la sauvegarde ; les appareils encore « nouveaux » restent
//! à l'écart, et relire le même lot ne change rien.

use dumbmonit_proto::{DiscoveredHost, ScanDepth};
use dumbmonit_server::backup::{ExportOptions, RestoreReport, collect, restore};
use dumbmonit_server::db;
use dumbmonit_server::netinventory::store::{self, ScheduleInput};
use sqlx::SqlitePool;

async fn instance(dir: &tempfile::TempDir) -> (SqlitePool, dumbmonit_server::crypto::Cipher) {
    let pool = db::open(&dir.path().join("dumbmonit.db")).await.expect("base");
    let cipher = db::init_cipher(&pool, "secret-de-test-suffisamment-long").await.expect("clé");
    (pool, cipher)
}

fn counts(report: &RestoreReport, name: &str) -> (usize, usize, usize) {
    let section = report.sections.iter().find(|s| s.section == name).expect("section");
    (section.created, section.updated, section.skipped)
}

fn host(ip: &str, mac: &str) -> DiscoveredHost {
    DiscoveredHost {
        ip: ip.to_string(),
        hostname: Some(format!("h-{ip}")),
        mac: Some(mac.to_string()),
        vendor: Some("Acme".to_string()),
        services: Vec::new(),
    }
}

#[tokio::test]
async fn les_appareils_connus_et_ignores_sont_restaures_sans_les_nouveaux() {
    let source_dir = tempfile::tempdir().unwrap();
    let (source, cipher) = instance(&source_dir).await;
    let id = store::insert_schedule(
        &source,
        &ScheduleInput {
            name: "LAN".to_string(),
            cidr: "10.0.0.0/24".to_string(),
            agent_id: None,
            interval_secs: 7200,
            depth: ScanDepth::Extended,
            enabled: true,
        },
    )
    .await
    .unwrap();
    // Premier balayage : tout est connu. Le second amène un appareil nouveau, un
    // autre est ignoré.
    store::apply_scan(&source, id, &[host("10.0.0.2", "aa:aa:aa:aa:aa:aa")]).await.unwrap();
    store::apply_scan(
        &source,
        id,
        &[
            host("10.0.0.2", "aa:aa:aa:aa:aa:aa"),
            host("10.0.0.8", "bb:bb:bb:bb:bb:bb"),
            host("10.0.0.9", "cc:cc:cc:cc:cc:cc"),
        ],
    )
    .await
    .unwrap();
    let inventory = store::inventory(&source, Some(id)).await.unwrap();
    let guest = inventory.iter().find(|d| d.ip == "10.0.0.8").unwrap().id;
    assert!(store::mark_device_ignored(&source, guest).await.unwrap());

    let lot = collect(&source, &cipher, ExportOptions::default()).await.expect("export");
    assert_eq!(lot.netscan_schedules.len(), 1);
    let exported = &lot.netscan_schedules[0];
    assert_eq!((exported.cidr.as_str(), exported.interval_secs), ("10.0.0.0/24", 7200));
    // Le connu (premier balayage) et l'ignoré ; pas le « nouveau » 10.0.0.9.
    assert_eq!(exported.devices.len(), 2);
    assert!(exported.devices.iter().any(|d| d.ip == "10.0.0.8" && d.ignored));
    assert!(exported.devices.iter().all(|d| d.ip != "10.0.0.9"));
    assert_eq!(lot.summary().get("netscan_schedules"), Some(&1));

    let target_dir = tempfile::tempdir().unwrap();
    let (pool, cipher) = instance(&target_dir).await;
    let report = restore(&pool, &cipher, &lot, true).await.expect("restauration");
    assert_eq!(counts(&report, "netscan_schedules"), (1, 0, 0), "{report:#?}");

    let schedules = store::list_schedules(&pool).await.unwrap();
    assert_eq!(schedules.len(), 1);
    assert!(schedules[0].initialized, "le prochain balayage ne doit pas tout refaire « nouveau »");
    assert_eq!(schedules[0].depth, ScanDepth::Extended);
    let devices = store::inventory(&pool, Some(schedules[0].id)).await.unwrap();
    assert_eq!(devices.len(), 2);
    assert!(devices.iter().all(|d| d.known));
    assert!(devices.iter().any(|d| d.ip == "10.0.0.8" && d.ignored));
    assert!(devices.iter().any(|d| d.ip == "10.0.0.2" && !d.ignored));
    assert!(store::unknown_items(&pool).await.unwrap().is_empty());

    // Un balayage qui retrouve un appareil restauré ne le signale pas.
    let scan = [host("10.0.0.2", "aa:aa:aa:aa:aa:aa")];
    let applied = store::apply_scan(&pool, schedules[0].id, &scan).await.unwrap();
    assert_eq!(applied.new_devices, 0);

    let again = restore(&pool, &cipher, &lot, true).await.expect("seconde restauration");
    assert_eq!(counts(&again, "netscan_schedules"), (0, 0, 1), "rien ne bouge : {again:#?}");
}

#[tokio::test]
async fn une_decision_du_lot_rattrape_un_appareil_encore_nouveau() {
    let source_dir = tempfile::tempdir().unwrap();
    let (source, cipher) = instance(&source_dir).await;
    let input = ScheduleInput {
        name: String::new(),
        cidr: "10.0.0.0/24".to_string(),
        agent_id: None,
        interval_secs: 3600,
        depth: ScanDepth::Standard,
        enabled: true,
    };
    let id = store::insert_schedule(&source, &input).await.unwrap();
    store::apply_scan(&source, id, &[host("10.0.0.2", "aa:aa:aa:aa:aa:aa")]).await.unwrap();
    let lot = collect(&source, &cipher, ExportOptions::default()).await.expect("export");

    // Sur l'instance cible, le même appareil est déjà là mais « nouveau ».
    let target_dir = tempfile::tempdir().unwrap();
    let (pool, cipher) = instance(&target_dir).await;
    let target_id = store::insert_schedule(&pool, &input).await.unwrap();
    store::apply_scan(&pool, target_id, &[host("10.0.0.3", "dd:dd:dd:dd:dd:dd")]).await.unwrap();
    store::apply_scan(
        &pool,
        target_id,
        &[host("10.0.0.3", "dd:dd:dd:dd:dd:dd"), host("10.0.0.2", "aa:aa:aa:aa:aa:aa")],
    )
    .await
    .unwrap();
    assert_eq!(store::unknown_items(&pool).await.unwrap().len(), 1);

    let report = restore(&pool, &cipher, &lot, true).await.expect("restauration");
    assert_eq!(counts(&report, "netscan_schedules"), (0, 1, 0), "{report:#?}");
    assert!(store::unknown_items(&pool).await.unwrap().is_empty());
}
