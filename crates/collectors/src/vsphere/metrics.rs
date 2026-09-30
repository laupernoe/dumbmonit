//! Traduction de l'inventaire vSphere en échantillons.
//!
//! Tous les états énumérés deviennent des nombres stables, pour que les règles
//! comparent un seuil et que les graphes gardent une seule série :
//!
//! | Métrique | Valeurs |
//! |---|---|
//! | `*_status` (`overallStatus`) | 0 vert, 1 jaune, 2 rouge, 3 gris (inconnu) |
//! | `host_connection_state` | 0 connecté, 1 déconnecté par un administrateur, 2 ne répond plus |
//! | `host_power_state` | 0 allumé, 1 en veille, 2 éteint, 3 inconnu |
//! | `vm_power_state` | 0 allumée, 1 éteinte, 2 suspendue |
//! | `vm_tools_status` | 0 à jour, 1 ancienne version, 2 installés mais arrêtés, 3 absents |

use std::collections::{BTreeMap, HashMap};

use dumbmonit_proto::{MetricKind, Sample};

use super::soap::{About, AlarmState, MoRef, ObjectContent};

/// Au-delà, les VM ne portent plus de série propre : seuls les totaux restent.
pub const MAX_VMS: usize = 1000;

/// Au plus autant d'alarmes déclenchées portent une série nommée.
pub const MAX_ALARMS: usize = 64;

fn gauge(name: &str, value: f64, ts_ms: i64) -> Sample {
    Sample::new(name, value, MetricKind::Gauge, ts_ms)
}

pub fn status_code(status: &str) -> Option<f64> {
    match status {
        "green" => Some(0.0),
        "yellow" => Some(1.0),
        "red" => Some(2.0),
        "gray" => Some(3.0),
        _ => None,
    }
}

fn connection_code(state: &str) -> Option<f64> {
    match state {
        "connected" => Some(0.0),
        "disconnected" => Some(1.0),
        "notResponding" => Some(2.0),
        _ => None,
    }
}

fn host_power_code(state: &str) -> Option<f64> {
    match state {
        "poweredOn" => Some(0.0),
        "standBy" => Some(1.0),
        "poweredOff" => Some(2.0),
        "unknown" => Some(3.0),
        _ => None,
    }
}

fn vm_power_code(state: &str) -> Option<f64> {
    match state {
        "poweredOn" => Some(0.0),
        "poweredOff" => Some(1.0),
        "suspended" => Some(2.0),
        _ => None,
    }
}

fn tools_code(status: &str) -> Option<f64> {
    match status {
        "toolsOk" => Some(0.0),
        "toolsOld" => Some(1.0),
        "toolsNotRunning" => Some(2.0),
        "toolsNotInstalled" => Some(3.0),
        _ => None,
    }
}

/// Version et nature du point d'entrée : vCenter ou ESXi seul.
pub fn about_samples(about: &About, ts_ms: i64) -> Vec<Sample> {
    let product = if about.api_type == "HostAgent" { "esxi" } else { "vcenter" };
    vec![
        gauge("vsphere_info", 1.0, ts_ms)
            .with_label("product", product)
            .with_label("version", about.version.as_str())
            .with_label("build", about.build.as_str()),
    ]
}

/// Les noms des objets, pour nommer les entités des alarmes.
pub fn names(objects: &[ObjectContent]) -> HashMap<MoRef, String> {
    objects.iter().filter_map(|o| Some((o.obj.clone(), o.text("name")?.to_string()))).collect()
}

pub fn inventory_samples(objects: &[ObjectContent], ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let mut hosts = BTreeMap::<&str, f64>::new();
    let mut vms =
        BTreeMap::<&str, f64>::from([("poweredOn", 0.0), ("poweredOff", 0.0), ("suspended", 0.0)]);
    let mut templates = 0.0;
    let mut datastores = 0.0;
    let mut vm_series = 0usize;

    for object in objects {
        let Some(name) = object.text("name") else { continue };
        match object.obj.kind.as_str() {
            "HostSystem" => {
                let state = object.text("runtime.connectionState").unwrap_or("unknown");
                *hosts.entry(state).or_default() += 1.0;
                out.extend(host_samples(object, name, ts_ms));
            }
            "VirtualMachine" => {
                if object.flag("config.template") == Some(true) {
                    templates += 1.0;
                    continue;
                }
                let state = object.text("runtime.powerState").unwrap_or("unknown");
                *vms.entry(state).or_default() += 1.0;
                if vm_series < MAX_VMS {
                    vm_series += 1;
                    out.extend(vm_samples(object, name, ts_ms));
                }
            }
            "Datastore" => {
                datastores += 1.0;
                out.extend(datastore_samples(object, name, ts_ms));
            }
            _ => {}
        }
    }
    for state in ["connected", "disconnected", "notResponding"] {
        hosts.entry(state).or_default();
    }
    for (state, count) in hosts {
        out.push(gauge("vsphere_hosts", count, ts_ms).with_label("state", state));
    }
    for (state, count) in vms {
        out.push(gauge("vsphere_vms", count, ts_ms).with_label("state", state));
    }
    out.push(gauge("vsphere_templates", templates, ts_ms));
    out.push(gauge("vsphere_datastores", datastores, ts_ms));
    out
}

fn host_samples(host: &ObjectContent, name: &str, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let mut push = |metric: &str, value: Option<f64>| {
        if let Some(value) = value {
            out.push(gauge(metric, value, ts_ms).with_label("host", name));
        }
    };
    let connected = host.text("runtime.connectionState") == Some("connected");
    push(
        "vsphere_host_connection_state",
        host.text("runtime.connectionState").and_then(connection_code),
    );
    push("vsphere_host_power_state", host.text("runtime.powerState").and_then(host_power_code));
    push(
        "vsphere_host_maintenance",
        host.flag("runtime.inMaintenanceMode").map(|m| if m { 1.0 } else { 0.0 }),
    );
    push("vsphere_host_status", host.text("overallStatus").and_then(status_code));
    // Les statistiques d'un hôte déconnecté sont les dernières connues : les
    // republier ferait croire qu'il tourne encore.
    if connected {
        let mhz = host.number("summary.hardware.cpuMhz");
        let cores = host.number("summary.hardware.numCpuCores");
        let used_mhz = host.number("summary.quickStats.overallCpuUsage");
        if let (Some(mhz), Some(cores), Some(used)) = (mhz, cores, used_mhz)
            && mhz * cores > 0.0
        {
            push("vsphere_host_cpu_percent", Some((used / (mhz * cores) * 100.0).min(100.0)));
        }
        let memory = host.number("summary.hardware.memorySize");
        let used_mb = host.number("summary.quickStats.overallMemoryUsage");
        if let (Some(memory), Some(used)) = (memory, used_mb)
            && memory > 0.0
        {
            let used_bytes = used * 1024.0 * 1024.0;
            push("vsphere_host_memory_bytes", Some(memory));
            push("vsphere_host_memory_percent", Some((used_bytes / memory * 100.0).min(100.0)));
        }
        push("vsphere_host_uptime_seconds", host.number("summary.quickStats.uptime"));
    }
    out
}

fn vm_samples(vm: &ObjectContent, name: &str, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let power = vm.text("runtime.powerState").and_then(vm_power_code);
    if let Some(power) = power {
        out.push(gauge("vsphere_vm_power_state", power, ts_ms).with_label("vm", name));
    }
    if let Some(status) = vm.text("overallStatus").and_then(status_code) {
        out.push(gauge("vsphere_vm_status", status, ts_ms).with_label("vm", name));
    }
    // Les outils d'une VM éteinte sont forcément arrêtés : sans intérêt.
    if power == Some(0.0)
        && let Some(tools) = vm.text("guest.toolsStatus").and_then(tools_code)
    {
        out.push(gauge("vsphere_vm_tools_status", tools, ts_ms).with_label("vm", name));
    }
    out
}

fn datastore_samples(ds: &ObjectContent, name: &str, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let label = |sample: Sample| sample.with_label("datastore", name);
    if let Some(accessible) = ds.flag("summary.accessible") {
        out.push(label(gauge(
            "vsphere_datastore_accessible",
            if accessible { 1.0 } else { 0.0 },
            ts_ms,
        )));
    }
    if let Some(status) = ds.text("overallStatus").and_then(status_code) {
        out.push(label(gauge("vsphere_datastore_status", status, ts_ms)));
    }
    let capacity = ds.number("summary.capacity");
    let free = ds.number("summary.freeSpace");
    if let Some(capacity) = capacity {
        out.push(label(gauge("vsphere_datastore_capacity_bytes", capacity, ts_ms)));
    }
    if let Some(free) = free {
        out.push(label(gauge("vsphere_datastore_free_bytes", free, ts_ms)));
    }
    // Une banque inaccessible annonce 0 : ce n'est pas « pleine ».
    if let (Some(capacity), Some(free)) = (capacity, free)
        && capacity > 0.0
    {
        out.push(label(gauge(
            "vsphere_datastore_used_percent",
            ((capacity - free) / capacity * 100.0).clamp(0.0, 100.0),
            ts_ms,
        )));
    }
    out
}

/// Les alarmes déclenchées : un total par couleur (non acquittées), et une
/// série par alarme, nommée par la définition et l'objet qu'elle vise.
pub fn alarm_samples(
    states: &[AlarmState],
    entity_names: &HashMap<MoRef, String>,
    alarm_names: &HashMap<MoRef, String>,
    ts_ms: i64,
) -> Vec<Sample> {
    let mut counts = BTreeMap::from([("red", 0.0), ("yellow", 0.0)]);
    let mut acknowledged = 0.0;
    let mut out = Vec::new();
    for state in states {
        if state.acknowledged {
            acknowledged += 1.0;
        } else if let Some(count) = counts.get_mut(state.status.as_str()) {
            *count += 1.0;
        }
    }
    for (status, count) in counts {
        out.push(gauge("vsphere_alarms", count, ts_ms).with_label("status", status));
    }
    out.push(gauge("vsphere_alarms_acknowledged", acknowledged, ts_ms));
    for state in states.iter().filter(|s| !s.acknowledged).take(MAX_ALARMS) {
        let Some(code) = status_code(&state.status) else { continue };
        let entity =
            entity_names.get(&state.entity).cloned().unwrap_or_else(|| state.entity.id.clone());
        let alarm =
            alarm_names.get(&state.alarm).cloned().unwrap_or_else(|| state.alarm.id.clone());
        out.push(
            gauge("vsphere_alarm", code, ts_ms)
                .with_label("alarm", alarm)
                .with_label("entity", entity)
                .with_label("entity_type", entity_type(&state.entity.kind))
                .with_label("status", state.status.as_str()),
        );
    }
    out
}

fn entity_type(kind: &str) -> &'static str {
    match kind {
        "HostSystem" => "host",
        "VirtualMachine" => "vm",
        "Datastore" => "datastore",
        "ClusterComputeResource" | "ComputeResource" => "cluster",
        "Datacenter" => "datacenter",
        "Folder" => "folder",
        "ResourcePool" => "resource_pool",
        "Network" | "DistributedVirtualPortgroup" | "VmwareDistributedVirtualSwitch" => "network",
        _ => "other",
    }
}

#[cfg(test)]
mod tests {
    use super::super::soap::{self, decode_page, read};
    use super::*;

    const PAGES: [&str; 5] = [
        include_str!("testdata/vcsim_0.56.0/vcenter/inventory_page1.xml"),
        include_str!("testdata/vcsim_0.56.0/vcenter/inventory_page2.xml"),
        include_str!("testdata/vcsim_0.56.0/vcenter/inventory_page3.xml"),
        include_str!("testdata/vcsim_0.56.0/vcenter/inventory_page4.xml"),
        include_str!("testdata/vcsim_0.56.0/vcenter/inventory_page5.xml"),
    ];

    fn inventory() -> Vec<ObjectContent> {
        PAGES.iter().flat_map(|page| read(page, decode_page).unwrap().unwrap().objects).collect()
    }

    fn find<'a>(samples: &'a [Sample], metric: &str, key: &str, value: &str) -> Option<&'a Sample> {
        samples
            .iter()
            .find(|s| s.metric == metric && s.labels.get(key).map(String::as_str) == Some(value))
    }

    // L'inventaire capturé sur `vcsim` après avoir, avec govc, éteint une VM,
    // mis un hôte en maintenance, déconnecté un autre et changé une VM éteinte
    // en modèle.
    #[test]
    fn les_etats_des_hotes_sont_distingues() {
        let samples = inventory_samples(&inventory(), 0);
        let state =
            |host| find(&samples, "vsphere_host_connection_state", "host", host).unwrap().value;
        assert_eq!(state("DC0_H0"), 0.0);
        assert_eq!(state("DC0_C0_H1"), 1.0);
        let maintenance =
            find(&samples, "vsphere_host_maintenance", "host", "DC0_C0_H2").unwrap().value;
        assert_eq!(maintenance, 1.0);
        // Un hôte déconnecté ne publie plus de charge.
        assert!(find(&samples, "vsphere_host_cpu_percent", "host", "DC0_C0_H1").is_none());
        let cpu = find(&samples, "vsphere_host_cpu_percent", "host", "DC0_H0").unwrap().value;
        assert!((cpu - 67.0 / (2294.0 * 2.0) * 100.0).abs() < 1e-9);
        let memory = find(&samples, "vsphere_host_memory_percent", "host", "DC0_H0").unwrap().value;
        assert!(memory > 30.0 && memory < 40.0, "{memory}");
        assert_eq!(find(&samples, "vsphere_hosts", "state", "connected").unwrap().value, 3.0);
        assert_eq!(find(&samples, "vsphere_hosts", "state", "disconnected").unwrap().value, 1.0);
        assert_eq!(find(&samples, "vsphere_hosts", "state", "notResponding").unwrap().value, 0.0);
    }

    #[test]
    fn les_modeles_ne_comptent_pas_comme_des_vm() {
        let samples = inventory_samples(&inventory(), 0);
        assert_eq!(find(&samples, "vsphere_vms", "state", "poweredOn").unwrap().value, 2.0);
        assert_eq!(find(&samples, "vsphere_vms", "state", "poweredOff").unwrap().value, 1.0);
        assert_eq!(samples.iter().find(|s| s.metric == "vsphere_templates").unwrap().value, 1.0);
        assert!(find(&samples, "vsphere_vm_power_state", "vm", "DC0_C0_RP0_VM1").is_none());
        assert_eq!(
            find(&samples, "vsphere_vm_power_state", "vm", "DC0_H0_VM1").unwrap().value,
            1.0
        );
        // Outils absents sur une VM allumée ; rien pour une VM éteinte.
        assert_eq!(
            find(&samples, "vsphere_vm_tools_status", "vm", "DC0_H0_VM0").unwrap().value,
            3.0
        );
        assert!(find(&samples, "vsphere_vm_tools_status", "vm", "DC0_H0_VM1").is_none());
    }

    #[test]
    fn les_banques_de_donnees_disent_leur_remplissage() {
        let samples = inventory_samples(&inventory(), 0);
        let used =
            find(&samples, "vsphere_datastore_used_percent", "datastore", "LocalDS_0").unwrap();
        assert!(
            (used.value - (4398046511104.0 - 4355096838144.0) / 4398046511104.0 * 100.0).abs()
                < 1e-9
        );
        assert_eq!(
            find(&samples, "vsphere_datastore_accessible", "datastore", "LocalDS_0").unwrap().value,
            1.0
        );
    }

    #[test]
    fn les_alarmes_sont_nommees_et_les_acquittees_mises_a_part() {
        let objects = inventory();
        let states =
            read(include_str!("testdata/documented/alarm_states.xml"), soap::decode_alarm_states)
                .unwrap()
                .unwrap();
        let alarm_page = read(include_str!("testdata/documented/alarm_names.xml"), decode_page)
            .unwrap()
            .unwrap();
        let alarm_names: HashMap<MoRef, String> = alarm_page
            .objects
            .iter()
            .map(|o| (o.obj.clone(), o.text("info.name").unwrap().to_string()))
            .collect();
        let samples = alarm_samples(&states, &names(&objects), &alarm_names, 0);
        assert_eq!(find(&samples, "vsphere_alarms", "status", "red").unwrap().value, 1.0);
        assert_eq!(find(&samples, "vsphere_alarms", "status", "yellow").unwrap().value, 1.0);
        assert_eq!(
            samples.iter().find(|s| s.metric == "vsphere_alarms_acknowledged").unwrap().value,
            1.0
        );
        let host = find(&samples, "vsphere_alarm", "entity", "DC0_C0_H0").unwrap();
        assert_eq!(host.labels["alarm"], "Host connection and power state");
        assert_eq!(host.labels["entity_type"], "host");
        assert_eq!(host.value, 2.0);
        // L'alarme acquittée ne porte pas de série.
        assert!(find(&samples, "vsphere_alarm", "entity", "DC0_H0_VM0").is_none());
    }

    #[test]
    fn un_esxi_seul_se_presente_comme_tel() {
        let esx = read(
            include_str!("testdata/vcsim_0.56.0/esx/service_content.xml"),
            soap::decode_service_content,
        )
        .unwrap()
        .unwrap();
        let samples = about_samples(&esx.about, 0);
        assert_eq!(samples[0].labels["product"], "esxi");
        assert_eq!(samples[0].labels["version"], "6.5.0");
    }
}
