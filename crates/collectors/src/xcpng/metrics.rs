//! Mise en forme des enregistrements XAPI en échantillons.
//!
//! Chaque fonction prend les objets `{référence: enregistrement}` tels que
//! `*.get_all_records` les rend, et ne suppose rien d'un champ qu'elle n'a pas
//! vérifié dans la documentation du projet xapi-project (voir le module) : un
//! champ absent ou mal typé est simplement ignoré, jamais une panique.

use dumbmonit_proto::Sample;
use serde_json::Value;

use crate::rest::{flag, gauge, number, text};

/// Au plus autant d'hôtes, de VM, de SR et de pools détaillés : un pool XCP-ng
/// de taille raisonnable en a quelques dizaines, pas des centaines.
pub const MAX_ITEMS: usize = 200;

fn records(value: &Value) -> Vec<(&str, &Value)> {
    value.as_object().into_iter().flatten().map(|(k, v)| (k.as_str(), v)).collect()
}

/// `host.cpu_info["cpu_count"]`, en texte chez XAPI.
fn cpu_count(host: &Value) -> Option<f64> {
    number(host.pointer("/cpu_info/cpu_count"))
}

/// `host.software_version["product_version"]`.
fn product_version(host: &Value) -> &str {
    host.pointer("/software_version/product_version").and_then(Value::as_str).unwrap_or("unknown")
}

pub fn host_samples(hosts: &Value, host_metrics: &Value, ts_ms: i64) -> Vec<Sample> {
    let hosts = records(hosts);
    let mut out = Vec::with_capacity(hosts.len() * 6 + 2);
    let mut enabled_count = 0usize;
    for (_, host) in hosts.iter().take(MAX_ITEMS) {
        let Some(name) = text(host, "name_label") else { continue };
        let labelled = |sample: Sample| sample.with_label("host", name);
        let enabled = host.get("enabled").and_then(Value::as_bool).unwrap_or(false);
        if enabled {
            enabled_count += 1;
        }
        out.push(labelled(flag("xcpng_host_enabled", enabled, ts_ms)));
        out.push(labelled(
            gauge("xcpng_host_info", 1.0, ts_ms).with_label("version", product_version(host)),
        ));
        if let Some(cores) = cpu_count(host) {
            out.push(labelled(gauge("xcpng_host_cpu_cores", cores, ts_ms)));
        }
        let metrics_ref = text(host, "metrics").filter(|r| *r != "OpaqueRef:NULL");
        let Some(metrics) = metrics_ref.and_then(|r| host_metrics.get(r)) else { continue };
        if let Some(live) = metrics.get("live").and_then(Value::as_bool) {
            out.push(labelled(flag("xcpng_host_live", live, ts_ms)));
        }
        let total = number(metrics.get("memory_total"));
        let free = number(metrics.get("memory_free")).filter(|f| *f > 0.0);
        if let Some(total) = total {
            out.push(labelled(gauge("xcpng_host_memory_total_bytes", total, ts_ms)));
        }
        if let (Some(total), Some(free)) = (total, free) {
            out.push(labelled(gauge("xcpng_host_memory_used_bytes", total - free, ts_ms)));
        }
    }
    out.push(gauge("xcpng_hosts", hosts.len() as f64, ts_ms));
    out.push(gauge("xcpng_hosts_enabled", enabled_count as f64, ts_ms));
    out
}

/// `VM.power_state` : `Halted`, `Paused`, `Running`, `Suspended` (seules
/// valeurs de l'énumération XenAPI — il n'existe pas d'état « Crashed »).
fn power_state_label(vm: &Value) -> &str {
    text(vm, "power_state").unwrap_or("Halted")
}

fn is_real_vm(vm: &Value) -> bool {
    let set = |key: &str| vm.get(key).and_then(Value::as_bool).unwrap_or(false);
    !set("is_a_template") && !set("is_a_snapshot") && !set("is_control_domain")
}

/// Vrai si la haute disponibilité est active sur l'un des pools lus (un
/// coordinateur n'en connaît qu'un, le sien).
pub fn pool_ha_enabled(pools: &Value) -> bool {
    records(pools)
        .iter()
        .any(|(_, pool)| pool.get("ha_enabled").and_then(Value::as_bool) == Some(true))
}

/// `ha_enabled` : la haute disponibilité du pool est active. Sans elle, une
/// priorité de redémarrage ne protège rien, et la série n'est pas émise.
pub fn vm_samples(vms: &Value, vm_metrics: &Value, ha_enabled: bool, ts_ms: i64) -> Vec<Sample> {
    let all: Vec<(&str, &Value)> =
        records(vms).into_iter().filter(|(_, vm)| is_real_vm(vm)).collect();
    let mut out = Vec::with_capacity(all.len() * 6 + 2);
    let mut running = 0usize;
    for (_, vm) in all.iter().take(MAX_ITEMS) {
        let Some(name) = text(vm, "name_label") else { continue };
        let labelled = |sample: Sample| sample.with_label("vm", name);
        let state = power_state_label(vm);
        let is_running = state == "Running";
        if is_running {
            running += 1;
        }
        out.push(labelled(
            gauge("xcpng_vm_power_state_info", 1.0, ts_ms)
                .with_label("state", state.to_ascii_lowercase()),
        ));
        out.push(labelled(flag("xcpng_vm_running", is_running, ts_ms)));
        // Protégée par la haute disponibilité du pool (`ha_restart_priority`
        // à `restart` ou `best-effort`) et arrêtée : HA ne l'a pas relancée.
        let ha_protected = ha_enabled && text(vm, "ha_restart_priority").is_some();
        if ha_protected {
            out.push(labelled(flag("xcpng_vm_ha_protected_down", state == "Halted", ts_ms)));
        }
        if let Some(vcpus) = number(vm.get("VCPUs_at_startup")) {
            out.push(labelled(gauge("xcpng_vm_vcpus", vcpus, ts_ms)));
        }
        let metrics_ref = text(vm, "metrics").filter(|r| *r != "OpaqueRef:NULL");
        let memory = metrics_ref
            .and_then(|r| vm_metrics.get(r))
            .and_then(|m| number(m.get("memory_actual")))
            .or_else(|| number(vm.get("memory_dynamic_max")));
        if let Some(memory) = memory {
            out.push(labelled(gauge("xcpng_vm_memory_bytes", memory, ts_ms)));
        }
    }
    out.push(gauge("xcpng_vms", all.len() as f64, ts_ms));
    out.push(gauge("xcpng_vms_running", running as f64, ts_ms));
    out
}

pub fn sr_samples(srs: &Value, ts_ms: i64) -> Vec<Sample> {
    let srs = records(srs);
    let mut out = Vec::with_capacity(srs.len() * 4);
    for (_, sr) in srs.iter().take(MAX_ITEMS) {
        let Some(name) = text(sr, "name_label") else { continue };
        // Un lecteur ISO (`content_type: "iso"`) n'a pas de remplissage utile à
        // surveiller : sa taille est celle du média inséré, pas d'un disque.
        if text(sr, "content_type") == Some("iso") {
            continue;
        }
        let labelled = |sample: Sample| sample.with_label("sr", name);
        let size = number(sr.get("physical_size")).filter(|s| *s > 0.0);
        let used = number(sr.get("physical_utilisation"));
        if let Some(size) = size {
            out.push(labelled(gauge("xcpng_sr_size_bytes", size, ts_ms)));
        }
        if let Some(used) = used {
            out.push(labelled(gauge("xcpng_sr_used_bytes", used, ts_ms)));
        }
        if let (Some(size), Some(used)) = (size, used) {
            out.push(labelled(gauge("xcpng_sr_used_percent", 100.0 * used / size, ts_ms)));
        }
        if let Some(shared) = sr.get("shared").and_then(Value::as_bool) {
            out.push(labelled(flag("xcpng_sr_shared", shared, ts_ms)));
        }
    }
    out.push(gauge("xcpng_srs", srs.len() as f64, ts_ms));
    out
}

pub fn pool_samples(pools: &Value, hosts: &Value, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    for (_, pool) in records(pools).into_iter().take(MAX_ITEMS) {
        let Some(name) = text(pool, "name_label") else { continue };
        let labelled = |sample: Sample| sample.with_label("pool", name);
        let master_name = text(pool, "master")
            .and_then(|r| hosts.get(r))
            .and_then(|host| text(host, "name_label"))
            .unwrap_or("unknown");
        out.push(labelled(gauge("xcpng_pool_info", 1.0, ts_ms).with_label("master", master_name)));
        if let Some(ha) = pool.get("ha_enabled").and_then(Value::as_bool) {
            out.push(labelled(flag("xcpng_pool_ha_enabled", ha, ts_ms)));
        }
        // Sans HA, `ha_overcommitted` ne veut rien dire : non émis.
        let ha = pool.get("ha_enabled").and_then(Value::as_bool) == Some(true);
        if let Some(overcommitted) = pool.get("ha_overcommitted").and_then(Value::as_bool)
            && ha
        {
            out.push(labelled(flag("xcpng_pool_ha_overcommitted", overcommitted, ts_ms)));
        }
        if let Some(tolerate) = number(pool.get("ha_host_failures_to_tolerate")) {
            out.push(labelled(gauge("xcpng_pool_ha_host_failures_to_tolerate", tolerate, ts_ms)));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::test_support::{find, value};

    fn result(raw: &str) -> Value {
        serde_json::from_str::<Value>(raw).unwrap()["result"].clone()
    }

    const HOSTS: &str = include_str!("testdata/hosts.json");
    const HOST_METRICS: &str = include_str!("testdata/host_metrics.json");
    const VMS: &str = include_str!("testdata/vms.json");
    const VM_METRICS: &str = include_str!("testdata/vm_metrics.json");
    const SRS: &str = include_str!("testdata/srs.json");
    const POOLS: &str = include_str!("testdata/pools.json");

    #[test]
    fn hosts_report_their_cpu_and_memory() {
        let samples = host_samples(&result(HOSTS), &result(HOST_METRICS), 0);
        assert_eq!(value(&samples, "xcpng_hosts", &[]), 2.0);
        assert_eq!(value(&samples, "xcpng_hosts_enabled", &[]), 1.0);
        assert_eq!(value(&samples, "xcpng_host_enabled", &[("host", "xcp01")]), 1.0);
        assert_eq!(value(&samples, "xcpng_host_enabled", &[("host", "xcp02")]), 0.0);
        assert_eq!(value(&samples, "xcpng_host_cpu_cores", &[("host", "xcp01")]), 8.0);
        assert_eq!(value(&samples, "xcpng_host_live", &[("host", "xcp01")]), 1.0);
        assert_eq!(value(&samples, "xcpng_host_live", &[("host", "xcp02")]), 0.0);
        assert_eq!(
            value(&samples, "xcpng_host_memory_used_bytes", &[("host", "xcp01")]),
            33_737_859_072.0 - 9_663_676_416.0
        );
        // xcp02 : `memory_free` à zéro est traité comme non lu, pas comme « tout
        // utilisé » : un hôte éteint ne doit pas prétendre être plein.
        assert!(find(&samples, "xcpng_host_memory_used_bytes", &[("host", "xcp02")]).is_none());
        assert_eq!(
            find(&samples, "xcpng_host_info", &[("host", "xcp01")]).unwrap().labels["version"],
            "8.2.1"
        );
    }

    #[test]
    fn templates_and_the_control_domain_are_not_vms() {
        let samples = vm_samples(&result(VMS), &result(VM_METRICS), true, 0);
        assert_eq!(value(&samples, "xcpng_vms", &[]), 3.0, "web01, db01, scratch01 only");
        assert!(find(&samples, "xcpng_vm_running", &[("vm", "Debian 12 template")]).is_none());
        assert!(
            find(&samples, "xcpng_vm_running", &[("vm", "Control domain on host: xcp01")])
                .is_none()
        );
    }

    #[test]
    fn a_running_vm_takes_its_memory_from_its_metrics() {
        let samples = vm_samples(&result(VMS), &result(VM_METRICS), true, 0);
        assert_eq!(value(&samples, "xcpng_vm_running", &[("vm", "web01")]), 1.0);
        assert_eq!(value(&samples, "xcpng_vm_vcpus", &[("vm", "web01")]), 2.0);
        // `memory_actual` (VM_metrics) plutôt que `memory_dynamic_max` (VM) : la
        // VM est allumée, son usage réel est connu.
        assert_eq!(value(&samples, "xcpng_vm_memory_bytes", &[("vm", "web01")]), 1_879_048_192.0);
    }

    #[test]
    fn a_halted_vm_falls_back_to_its_configured_memory() {
        let samples = vm_samples(&result(VMS), &result(VM_METRICS), true, 0);
        assert_eq!(value(&samples, "xcpng_vm_memory_bytes", &[("vm", "db01")]), 4_294_967_296.0);
    }

    #[test]
    fn only_ha_protected_vms_get_the_down_series() {
        let samples = vm_samples(&result(VMS), &result(VM_METRICS), true, 0);
        assert_eq!(value(&samples, "xcpng_vm_ha_protected_down", &[("vm", "db01")]), 1.0);
        assert_eq!(value(&samples, "xcpng_vm_ha_protected_down", &[("vm", "web01")]), 0.0);
        assert!(
            find(&samples, "xcpng_vm_ha_protected_down", &[("vm", "scratch01")]).is_none(),
            "no restart priority: not HA-protected"
        );
        let without_ha = vm_samples(&result(VMS), &result(VM_METRICS), false, 0);
        assert!(find(&without_ha, "xcpng_vm_ha_protected_down", &[]).is_none(), "HA off");
        assert!(pool_ha_enabled(&result(POOLS)));
    }

    #[test]
    fn storage_repositories_report_their_fill_level() {
        let samples = sr_samples(&result(SRS), 0);
        assert_eq!(value(&samples, "xcpng_srs", &[]), 3.0);
        let used = value(&samples, "xcpng_sr_used_percent", &[("sr", "Local storage")]);
        assert!((used - 96.0).abs() < 0.1, "{used}");
        assert_eq!(value(&samples, "xcpng_sr_shared", &[("sr", "NFS backup")]), 1.0);
        // Le lecteur ISO est compté, pas détaillé.
        assert!(find(&samples, "xcpng_sr_used_percent", &[("sr", "DVD drives")]).is_none());
    }

    #[test]
    fn the_pool_carries_its_master_and_ha_state() {
        let samples = pool_samples(&result(POOLS), &result(HOSTS), 0);
        assert_eq!(
            find(&samples, "xcpng_pool_info", &[("pool", "Production pool")]).unwrap().labels["master"],
            "xcp01"
        );
        assert_eq!(value(&samples, "xcpng_pool_ha_enabled", &[("pool", "Production pool")]), 1.0);
        assert_eq!(
            value(&samples, "xcpng_pool_ha_overcommitted", &[("pool", "Production pool")]),
            1.0
        );
    }
}
