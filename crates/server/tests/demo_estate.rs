//! Le parc fictif du mode démonstration interrogé par les vrais collecteurs :
//! chaque faux équipement doit donner une collecte complète, sans erreur de
//! lecture, et le serveur Redfish doit montrer sa panne.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use dumbmonit_proto::{Collector, Sample, Target};
use dumbmonit_server::collectors;
use dumbmonit_server::demo::estate;

fn value(samples: &[Sample], metric: &str) -> Option<f64> {
    samples.iter().find(|s| s.metric == metric).map(|s| s.value)
}

#[tokio::test]
async fn chaque_equipement_du_parc_fictif_se_collecte() {
    let estate = estate::start().await.expect("parc fictif démarré");
    dumbmonit_collectors::http::set_resolve_overrides(estate.resolve.clone());

    let registry: Vec<Arc<dyn Collector>> = vec![
        Arc::new(collectors::ProxmoxCollector::new()),
        Arc::new(collectors::SynologyCollector::new()),
        Arc::new(collectors::PbsCollector::new()),
        Arc::new(collectors::TruenasCollector::new()),
        Arc::new(collectors::OpnsenseCollector::new()),
        Arc::new(collectors::RedfishCollector::new()),
    ];

    assert_eq!(estate.devices.len(), 6);
    let mut problems = Vec::new();
    for (index, device) in estate.devices.iter().enumerate() {
        assert!(device.address.ends_with(".home.arpa"), "{}", device.address);
        let collector = registry
            .iter()
            .find(|c| c.kind() == device.kind)
            .unwrap_or_else(|| panic!("collecteur {}", device.kind));
        let target = Target {
            id: index as i64 + 1,
            name: device.name.into(),
            address: device.address.clone(),
            kind: device.kind.into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(device.interval_secs),
            enabled: true,
            tags: device
                .tags
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect::<BTreeMap<_, _>>(),
            credential: device.credential.clone(),
        };
        let samples =
            match tokio::time::timeout(Duration::from_secs(30), collector.probe(&target)).await {
                Ok(Ok(samples)) => samples,
                Ok(Err(error)) => {
                    problems.push(format!("{} : {error}", device.key));
                    continue;
                }
                Err(_) => {
                    problems.push(format!("{} : pas de réponse en 30 s", device.key));
                    continue;
                }
            };
        let errors: Vec<_> = samples
            .iter()
            .filter(|s| s.metric.ends_with("scrape_errors") && s.value > 0.0)
            .map(|s| (s.metric.clone(), s.value))
            .collect();
        eprintln!("{} : {} échantillons, erreurs {errors:?}", device.key, samples.len());
        if samples.len() <= 20 {
            problems.push(format!("{} : {} échantillons", device.key, samples.len()));
        }
        if !errors.is_empty() {
            problems.push(format!("{} : erreurs de lecture {errors:?}", device.key));
        }
        match device.key {
            "bmc" => {
                let failed: Vec<_> = samples
                    .iter()
                    .filter(|s| s.metric == "redfish_psu_health" && s.value == 2.0)
                    .collect();
                assert_eq!(failed.len(), 1, "une alimentation en panne");
                let healthy = samples
                    .iter()
                    .filter(|s| s.metric == "redfish_psu_health" && s.value == 0.0)
                    .count();
                assert_eq!(healthy, 1, "l'autre alimentation est saine");
            }
            "pve" => {
                let guests =
                    samples.iter().filter(|s| s.metric.starts_with("proxmox_guest_")).count();
                assert!(guests > 50, "invités Proxmox : {guests}");
                assert!(
                    samples.iter().any(|s| s.labels.values().any(|v| v == "nextcloud")),
                    "les invités portent leurs noms du parc"
                );
            }
            "nas" => assert_eq!(value(&samples, "abb_tasks"), Some(3.0)),
            _ => {}
        }
    }
    assert!(problems.is_empty(), "{problems:#?}");
}
