//! Mise en forme des réponses v4 de Prism Central en échantillons.
//!
//! Chaque fonction lit les objets `data` d'une liste, et ignore un champ
//! absent ou mal typé plutôt que de paniquer : Prism omet les champs vides
//! (`omitempty` dans les clients officiels).

use std::collections::HashMap;

use dumbmonit_proto::Sample;
use serde_json::Value;

use super::Listing;
use crate::rest::{MAX_NAMED, flag, gauge, number, text};

/// `config.clusterFunction` contient `PRISM_CENTRAL` pour l'entrée qui décrit
/// Prism Central lui-même : ce n'est pas un cluster de calcul.
fn is_prism_central(cluster: &Value) -> bool {
    cluster
        .pointer("/config/clusterFunction")
        .and_then(Value::as_array)
        .is_some_and(|functions| functions.iter().any(|f| f.as_str() == Some("PRISM_CENTRAL")))
}

fn compute_clusters(clusters: &[Value]) -> impl Iterator<Item = &Value> {
    clusters.iter().filter(|cluster| !is_prism_central(cluster))
}

/// `extId` → nom, pour étiqueter les VM (qui ne portent que l'`extId` de leur
/// cluster).
pub fn cluster_names(clusters: &[Value]) -> HashMap<String, String> {
    clusters
        .iter()
        .filter_map(|cluster| {
            Some((text(cluster, "extId")?.to_string(), text(cluster, "name")?.to_string()))
        })
        .collect()
}

pub fn cluster_samples(clusters: &[Value], ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let mut count = 0usize;
    for cluster in compute_clusters(clusters) {
        count += 1;
        if count > MAX_NAMED {
            continue;
        }
        let Some(name) = text(cluster, "name") else { continue };
        let labelled = |sample: Sample| sample.with_label("cluster", name);
        let version = cluster
            .pointer("/config/buildInfo/version")
            .and_then(Value::as_str)
            .filter(|v| !v.trim().is_empty())
            .unwrap_or("unknown");
        out.push(labelled(
            gauge("nutanix_cluster_info", 1.0, ts_ms).with_label("version", version),
        ));
        if let Some(available) = cluster.pointer("/config/isAvailable").and_then(Value::as_bool) {
            out.push(labelled(flag("nutanix_cluster_available", available, ts_ms)));
        }
    }
    out.push(gauge("nutanix_clusters", count as f64, ts_ms));
    out
}

/// `hypervisor.state` : `ACROPOLIS_NORMAL` en temps normal, `ENTERING_…` ou
/// `ENTERED_MAINTENANCE_MODE` pendant une maintenance.
fn in_maintenance(host: &Value) -> bool {
    host.pointer("/hypervisor/state")
        .and_then(Value::as_str)
        .is_some_and(|state| state.contains("MAINTENANCE_MODE"))
}

pub fn host_samples(hosts: &Listing, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::with_capacity(hosts.items.len() * 5 + 1);
    for host in hosts.items.iter().take(MAX_NAMED) {
        let Some(name) = text(host, "hostName") else { continue };
        let cluster = host.pointer("/cluster/name").and_then(Value::as_str).unwrap_or("");
        let labelled =
            |sample: Sample| sample.with_label("host", name).with_label("cluster", cluster);
        let hypervisor = host.pointer("/hypervisor/fullName").and_then(Value::as_str).unwrap_or("");
        out.push(labelled(
            gauge("nutanix_host_info", 1.0, ts_ms).with_label("hypervisor", hypervisor),
        ));
        if let Some(cores) = number(host.get("numberOfCpuCores")) {
            out.push(labelled(gauge("nutanix_host_cpu_cores", cores, ts_ms)));
        }
        if let Some(memory) = number(host.get("memorySizeBytes")) {
            out.push(labelled(gauge("nutanix_host_memory_bytes", memory, ts_ms)));
        }
        if let Some(degraded) = host.get("isDegraded").and_then(Value::as_bool) {
            out.push(labelled(flag("nutanix_host_degraded", degraded, ts_ms)));
        }
        if host.pointer("/hypervisor/state").is_some() {
            out.push(labelled(flag("nutanix_host_maintenance", in_maintenance(host), ts_ms)));
        }
    }
    out.push(gauge("nutanix_hosts", hosts.total as f64, ts_ms));
    out
}

/// `powerState` : `ON`, `OFF`, `PAUSED` ou `UNDETERMINED`.
fn power_state(vm: &Value) -> &str {
    text(vm, "powerState").unwrap_or("UNDETERMINED")
}

pub fn vm_samples(vms: &Listing, clusters: &HashMap<String, String>, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::with_capacity(vms.items.len() * 5 + 2);
    let running = vms.items.iter().filter(|vm| power_state(vm) == "ON").count();
    for vm in vms.items.iter().take(MAX_NAMED) {
        let Some(name) = text(vm, "name") else { continue };
        let cluster = vm
            .pointer("/cluster/extId")
            .and_then(Value::as_str)
            .and_then(|id| clusters.get(id))
            .map(String::as_str)
            .unwrap_or("");
        let labelled =
            |sample: Sample| sample.with_label("vm", name).with_label("cluster", cluster);
        let state = power_state(vm);
        out.push(labelled(
            gauge("nutanix_vm_power_state_info", 1.0, ts_ms)
                .with_label("state", state.to_ascii_lowercase()),
        ));
        out.push(labelled(flag("nutanix_vm_running", state == "ON", ts_ms)));
        // vCPU = sockets × cœurs par socket × fils par cœur ; un champ absent
        // vaut 1, comme dans Prism.
        let factor = |key: &str| number(vm.get(key)).unwrap_or(1.0);
        if vm.get("numSockets").is_some() {
            let vcpus =
                factor("numSockets") * factor("numCoresPerSocket") * factor("numThreadsPerCore");
            out.push(labelled(gauge("nutanix_vm_vcpus", vcpus, ts_ms)));
        }
        if let Some(memory) = number(vm.get("memorySizeBytes")) {
            out.push(labelled(gauge("nutanix_vm_memory_bytes", memory, ts_ms)));
        }
    }
    out.push(gauge("nutanix_vms", vms.total as f64, ts_ms));
    out.push(gauge("nutanix_vms_running", running as f64, ts_ms));
    out
}

/// L'identifiant v4 d'un conteneur (`containerExtId`, à défaut `extId`).
pub fn container_id(container: &Value) -> Option<&str> {
    text(container, "containerExtId").or_else(|| text(container, "extId"))
}

/// Les conteneurs des utilisateurs : ceux que Nutanix marque internes
/// (`NutanixManagementShare`, …) ou en cours de retrait sont écartés.
pub fn watched_containers(containers: &[Value]) -> impl Iterator<Item = &Value> {
    containers.iter().filter(|container| {
        let set = |key: &str| container.get(key).and_then(Value::as_bool).unwrap_or(false);
        !set("isInternal") && !set("isMarkedForRemoval")
    })
}

/// La valeur la plus récente d'une série de statistiques v4
/// (`[{"timestamp": "…", "value": n}, …]`).
pub fn latest(series: Option<&Value>) -> Option<f64> {
    series?
        .as_array()?
        .iter()
        .filter_map(|point| Some((text(point, "timestamp")?, number(point.get("value"))?)))
        .max_by(|a, b| a.0.cmp(b.0))
        .map(|(_, value)| value)
}

/// `usage` : `(extId, (capacité, utilisé))` lus par les statistiques.
#[allow(clippy::type_complexity)]
pub fn storage_container_samples(
    containers: &[Value],
    usage: &[(String, (Option<f64>, Option<f64>))],
    ts_ms: i64,
) -> Vec<Sample> {
    let watched: Vec<&Value> = watched_containers(containers).collect();
    let mut out = Vec::with_capacity(watched.len() * 3 + 1);
    for container in watched.iter().take(MAX_NAMED) {
        let Some(name) = text(container, "name") else { continue };
        let cluster = text(container, "clusterName").unwrap_or("");
        let labelled =
            |sample: Sample| sample.with_label("container", name).with_label("cluster", cluster);
        let stats = container_id(container)
            .and_then(|id| usage.iter().find(|(known, _)| known == id))
            .map(|(_, stats)| *stats);
        let capacity = stats
            .and_then(|(capacity, _)| capacity)
            .or_else(|| number(container.get("maxCapacityBytes")))
            .filter(|capacity| *capacity > 0.0);
        let used = stats.and_then(|(_, used)| used);
        if let Some(capacity) = capacity {
            out.push(labelled(gauge("nutanix_storage_container_capacity_bytes", capacity, ts_ms)));
        }
        if let Some(used) = used {
            out.push(labelled(gauge("nutanix_storage_container_used_bytes", used, ts_ms)));
        }
        if let (Some(capacity), Some(used)) = (capacity, used) {
            out.push(labelled(gauge(
                "nutanix_storage_container_used_percent",
                100.0 * used / capacity,
                ts_ms,
            )));
        }
    }
    out.push(gauge("nutanix_storage_containers", watched.len() as f64, ts_ms));
    out
}

/// Alertes non résolues (le filtre est déjà posé côté Prism ; une alerte
/// résolue qui passerait quand même est écartée ici), par gravité :
/// `CRITICAL`, `WARNING`, `INFO`.
pub fn alert_samples(alerts: &Listing, ts_ms: i64) -> Vec<Sample> {
    let open: Vec<&Value> = alerts
        .items
        .iter()
        .filter(|alert| alert.get("isResolved").and_then(Value::as_bool) != Some(true))
        .collect();
    let count = |severity: &str| {
        open.iter().filter(|alert| text(alert, "severity") == Some(severity)).count() as f64
    };
    // Le total annoncé par Prism compte aussi ce qui dépasse les pages lues.
    let unresolved = alerts.total.saturating_sub(alerts.items.len() - open.len());
    vec![
        gauge("nutanix_alerts_unresolved", unresolved as f64, ts_ms),
        gauge("nutanix_alerts_critical", count("CRITICAL"), ts_ms),
        gauge("nutanix_alerts_warning", count("WARNING"), ts_ms),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::test_support::{find, value};

    fn listing(raw: &str) -> Listing {
        let body: Value = serde_json::from_str(raw).unwrap();
        let items = body["data"].as_array().cloned().unwrap_or_default();
        let total = body
            .pointer("/metadata/totalAvailableResults")
            .and_then(Value::as_u64)
            .map_or(items.len(), |n| n as usize);
        Listing { items, total }
    }

    const CLUSTERS: &str = include_str!("testdata/clusters.json");
    const HOSTS: &str = include_str!("testdata/hosts.json");
    const VMS: &str = include_str!("testdata/vms.json");
    const CONTAINERS: &str = include_str!("testdata/storage_containers.json");
    const CONTAINER_STATS: &str = include_str!("testdata/storage_container_stats.json");
    const ALERTS: &str = include_str!("testdata/alerts.json");

    #[test]
    fn prism_central_itself_is_not_a_cluster() {
        let samples = cluster_samples(&listing(CLUSTERS).items, 0);
        assert_eq!(value(&samples, "nutanix_clusters", &[]), 1.0);
        assert_eq!(
            find(&samples, "nutanix_cluster_info", &[("cluster", "homelab-cluster")])
                .unwrap()
                .labels["version"],
            "6.10.1"
        );
        assert_eq!(
            value(&samples, "nutanix_cluster_available", &[("cluster", "homelab-cluster")]),
            1.0
        );
        assert!(find(&samples, "nutanix_cluster_info", &[("cluster", "pc-main")]).is_none());
    }

    #[test]
    fn hosts_report_their_capacity_and_state() {
        let samples = host_samples(&listing(HOSTS), 0);
        assert_eq!(value(&samples, "nutanix_hosts", &[]), 3.0);
        let a1 = [("host", "ntnx-a1"), ("cluster", "homelab-cluster")];
        assert_eq!(value(&samples, "nutanix_host_cpu_cores", &a1), 24.0);
        assert_eq!(value(&samples, "nutanix_host_memory_bytes", &a1), 274_877_906_944.0);
        assert_eq!(value(&samples, "nutanix_host_degraded", &a1), 0.0);
        assert_eq!(value(&samples, "nutanix_host_maintenance", &[("host", "ntnx-a3")]), 1.0);
        assert_eq!(value(&samples, "nutanix_host_degraded", &[("host", "ntnx-a2")]), 1.0);
    }

    #[test]
    fn vms_are_counted_running_or_not_with_their_cluster() {
        let names = cluster_names(&listing(CLUSTERS).items);
        let samples = vm_samples(&listing(VMS), &names, 0);
        assert_eq!(value(&samples, "nutanix_vms", &[]), 3.0);
        assert_eq!(value(&samples, "nutanix_vms_running", &[]), 2.0);
        assert_eq!(value(&samples, "nutanix_vm_running", &[("vm", "db01")]), 0.0);
        assert_eq!(
            value(
                &samples,
                "nutanix_vm_memory_bytes",
                &[("vm", "web01"), ("cluster", "homelab-cluster")]
            ),
            4_294_967_296.0
        );
        // 2 sockets × 2 cœurs, fils par cœur absent : 4 vCPU.
        assert_eq!(value(&samples, "nutanix_vm_vcpus", &[("vm", "db01")]), 4.0);
    }

    #[test]
    fn storage_containers_report_their_fill_level_from_the_stats() {
        let stats: Value = serde_json::from_str(CONTAINER_STATS).unwrap();
        let usage = vec![(
            "c1a7e8a0-0000-4000-8000-000000000001".to_string(),
            (
                latest(stats["data"].get("storageCapacityBytes")),
                latest(stats["data"].get("storageUsageBytes")),
            ),
        )];
        let samples = storage_container_samples(&listing(CONTAINERS).items, &usage, 0);
        // Le conteneur interne n'est ni compté ni détaillé.
        assert_eq!(value(&samples, "nutanix_storage_containers", &[]), 2.0);
        assert!(
            find(
                &samples,
                "nutanix_storage_container_capacity_bytes",
                &[("container", "NutanixManagementShare")]
            )
            .is_none()
        );
        let used = value(
            &samples,
            "nutanix_storage_container_used_percent",
            &[("container", "default-container")],
        );
        // La dernière valeur (par horodatage), pas la première de la liste.
        assert!((used - 92.0).abs() < 0.01, "{used}");
        // Sans statistiques : la capacité configurée, pas de pourcentage.
        assert_eq!(
            value(&samples, "nutanix_storage_container_capacity_bytes", &[("container", "images")]),
            1_099_511_627_776.0
        );
        assert!(
            find(&samples, "nutanix_storage_container_used_percent", &[("container", "images")])
                .is_none()
        );
    }

    #[test]
    fn alerts_are_split_by_severity() {
        let samples = alert_samples(&listing(ALERTS), 0);
        assert_eq!(value(&samples, "nutanix_alerts_unresolved", &[]), 2.0);
        assert_eq!(value(&samples, "nutanix_alerts_critical", &[]), 1.0);
        assert_eq!(value(&samples, "nutanix_alerts_warning", &[]), 1.0);
    }
}
