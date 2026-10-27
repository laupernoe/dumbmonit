//! Le catalogue des modèles de tableaux de bord : des données, pas du code.
//!
//! Chaque modèle liste des widgets dont la requête MetricsQL contient le
//! jeton [`SELECTOR`] (`$T`). À la génération (voir `mod.rs`), ce jeton devient
//! `target=~"1|2|3"` : les identifiants des équipements choisis, du type voulu
//! pour ce widget. Un widget sans équipement de son type est omis.
//!
//! Les noms de métriques sont ceux que produisent les collecteurs, préfixe
//! `dumbmonit_` compris ; la documentation de chaque type
//! (`docs/devices/*.md`) les liste. Les textes sont en anglais, comme ceux du
//! catalogue des collecteurs.

/// Jeton remplacé par le sélecteur d'équipements dans chaque requête.
pub const SELECTOR: &str = "$T";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Chart,
    Stat,
    /// Un widget « état d'un dossier » par dossier des équipements choisis.
    Folders,
    Alerts,
    Text,
}

/// Un widget du modèle. Sans `kinds`, il s'applique à tous les équipements
/// choisis ; sinon seulement à ceux de ces types.
#[derive(Debug, Clone, Copy)]
pub struct Spec {
    pub shape: Shape,
    pub id: &'static str,
    pub title: &'static str,
    pub kinds: &'static [&'static str],
    /// Requête MetricsQL (graphes et valeurs), ou texte Markdown (`Text`).
    pub body: &'static str,
    pub unit: &'static str,
    pub w: u32,
    pub h: u32,
    pub warn: Option<f64>,
    pub crit: Option<f64>,
    /// Valeur : mauvais quand elle est basse.
    pub below: bool,
}

impl Spec {
    const fn new(shape: Shape, id: &'static str, title: &'static str, w: u32, h: u32) -> Self {
        Self {
            shape,
            id,
            title,
            kinds: &[],
            body: "",
            unit: "",
            w,
            h,
            warn: None,
            crit: None,
            below: false,
        }
    }

    const fn of(mut self, kinds: &'static [&'static str]) -> Self {
        self.kinds = kinds;
        self
    }

    const fn unit(mut self, unit: &'static str) -> Self {
        self.unit = unit;
        self
    }

    /// Seuils d'une valeur : au-dessus de `warn` / `crit` c'est mauvais.
    const fn above(mut self, warn: f64, crit: f64) -> Self {
        self.warn = Some(warn);
        self.crit = Some(crit);
        self
    }

    /// Seuils d'une valeur : en dessous de `warn` / `crit` c'est mauvais.
    const fn below(mut self, warn: f64, crit: f64) -> Self {
        self.warn = Some(warn);
        self.crit = Some(crit);
        self.below = true;
        self
    }

    /// Seul le seuil critique : la valeur est bonne tant qu'elle est sous `crit`.
    const fn crit(mut self, crit: f64) -> Self {
        self.crit = Some(crit);
        self
    }
}

const fn chart(id: &'static str, title: &'static str, query: &'static str) -> Spec {
    let mut spec = Spec::new(Shape::Chart, id, title, 6, 5);
    spec.body = query;
    spec
}

const fn stat(id: &'static str, title: &'static str, query: &'static str) -> Spec {
    let mut spec = Spec::new(Shape::Stat, id, title, 3, 3);
    spec.body = query;
    spec
}

const fn folders(id: &'static str, title: &'static str) -> Spec {
    Spec::new(Shape::Folders, id, title, 4, 5)
}

const fn alerts(id: &'static str, title: &'static str) -> Spec {
    Spec::new(Shape::Alerts, id, title, 6, 5)
}

const fn text(id: &'static str, title: &'static str, markdown: &'static str) -> Spec {
    let mut spec = Spec::new(Shape::Text, id, title, 12, 2);
    spec.body = markdown;
    spec
}

/// Un modèle de tableau de bord.
#[derive(Debug, Clone, Copy)]
pub struct Template {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    /// Nom d'icône lucide, résolu par l'interface (inconnue : icône par défaut).
    pub icon: &'static str,
    /// Types d'équipements qui l'alimentent ; vide : tous.
    pub kinds: &'static [&'static str],
    /// Série qu'un équipement doit publier pour compter (Docker : seuls les
    /// hôtes qui remontent des conteneurs). `None` : le type suffit.
    pub needs_metric: Option<&'static str>,
    pub range_secs: i64,
    pub refresh_secs: i64,
    pub widgets: &'static [Spec],
}

const AGENT: &[&str] = &["agent"];
const SNMP: &[&str] = &["snmp"];
const FORTIGATE: &[&str] = &["fortigate"];
const OPNSENSE: &[&str] = &["opnsense"];
const UNIFI: &[&str] = &["unifi"];
const SYNOLOGY: &[&str] = &["synology"];
const TRUENAS: &[&str] = &["truenas"];
const PBS: &[&str] = &["pbs"];
const KOPIA: &[&str] = &["kopia"];
const DUPLICATI: &[&str] = &["duplicati"];
const HTTP_TLS: &[&str] = &["http", "tls"];
const HTTP: &[&str] = &["http"];
const WEBSITE: &[&str] = &["website"];

pub static TEMPLATES: &[Template] = &[
    Template {
        id: "homelab-overview",
        title: "Homelab overview",
        description: "Everything at a glance: devices up and down, active alerts, the state of \
                      each folder, and the load on your servers.",
        icon: "LayoutDashboard",
        kinds: &[],
        needs_metric: None,
        range_secs: 3600,
        refresh_secs: 30,
        widgets: &[
            stat("up", "Devices up", "count(dumbmonit_up{$T} == 1)"),
            stat("down", "Devices down", "count(dumbmonit_up{$T} == 0) or vector(0)").crit(1.0),
            stat("cpu", "CPU, servers average", "avg(dumbmonit_cpu_usage_percent{$T})")
                .of(AGENT)
                .unit("%")
                .above(80.0, 95.0),
            stat("disk", "Fullest disk", "max(dumbmonit_filesystem_used_percent{$T})")
                .of(AGENT)
                .unit("%")
                .above(85.0, 95.0),
            chart("up-trend", "Devices up", "sum(dumbmonit_up{$T})"),
            alerts("alerts", "Active alerts"),
            chart(
                "cpu-hosts",
                "CPU per server",
                "max by (target) (dumbmonit_cpu_usage_percent{$T})",
            )
            .of(AGENT)
            .unit("%"),
            chart(
                "mem-hosts",
                "Memory per server",
                "max by (target) (dumbmonit_memory_used_percent{$T})",
            )
            .of(AGENT)
            .unit("%"),
            folders("folders", "Folder"),
        ],
    },
    Template {
        id: "servers",
        title: "Linux and Windows servers",
        description: "CPU, memory, disks, network and services of the machines that run the \
                      DumbMonit agent.",
        icon: "Server",
        kinds: AGENT,
        needs_metric: None,
        range_secs: 6 * 3600,
        refresh_secs: 30,
        widgets: &[
            stat("up", "Servers up", "count(dumbmonit_up{$T} == 1)"),
            stat("cpu", "CPU, average", "avg(dumbmonit_cpu_usage_percent{$T})")
                .unit("%")
                .above(80.0, 95.0),
            stat("mem", "Memory, average", "avg(dumbmonit_memory_used_percent{$T})")
                .unit("%")
                .above(85.0, 95.0),
            stat("disk", "Fullest disk", "max(dumbmonit_filesystem_used_percent{$T})")
                .unit("%")
                .above(85.0, 95.0),
            chart("cpu-per", "CPU per server", "max by (target) (dumbmonit_cpu_usage_percent{$T})")
                .unit("%"),
            chart(
                "mem-per",
                "Memory per server",
                "max by (target) (dumbmonit_memory_used_percent{$T})",
            )
            .unit("%"),
            chart(
                "fs",
                "Disk usage per volume",
                "max by (target, mountpoint) (dumbmonit_filesystem_used_percent{$T})",
            )
            .unit("%"),
            chart(
                "load",
                "Load average, 1 minute (Linux and BSD)",
                "max by (target) (dumbmonit_load_average_1{$T})",
            ),
            chart(
                "net-in",
                "Network in",
                "sum by (target) (rate(dumbmonit_if_octets_in{$T}[5m])) * 8",
            )
            .unit("bit/s"),
            chart(
                "net-out",
                "Network out",
                "sum by (target) (rate(dumbmonit_if_octets_out{$T}[5m])) * 8",
            )
            .unit("bit/s"),
            chart(
                "services",
                "Stopped services",
                "count by (target) (dumbmonit_service_up{$T} == 0) or on() vector(0)",
            ),
        ],
    },
    Template {
        id: "network",
        title: "Network",
        description: "Traffic and errors on switch ports, firewall and gateway health, VPN \
                      tunnels and internet latency.",
        icon: "Network",
        kinds: &["snmp", "fortigate", "opnsense", "unifi"],
        needs_metric: None,
        range_secs: 6 * 3600,
        refresh_secs: 30,
        widgets: &[
            stat("up", "Network devices up", "count(dumbmonit_up{$T} == 1)"),
            stat("ports", "Ports up", "count(dumbmonit_if_oper_status{$T} == 1)").of(SNMP),
            stat(
                "errors",
                "Interface errors",
                "sum(rate(dumbmonit_if_errors_in{$T}[5m]) + rate(dumbmonit_if_errors_out{$T}[5m]))",
            )
            .of(SNMP)
            .unit("err/s")
            .above(1.0, 10.0),
            stat("gateways", "Gateways down", "sum(dumbmonit_opnsense_gateways_down{$T})")
                .of(OPNSENSE)
                .crit(1.0),
            chart(
                "traffic-in",
                "Inbound traffic, busiest interfaces",
                "topk(8, max by (target, ifname) (rate(dumbmonit_if_octets_in{$T}[5m])) * 8)",
            )
            .of(SNMP)
            .unit("bit/s"),
            chart(
                "traffic-out",
                "Outbound traffic, busiest interfaces",
                "topk(8, max by (target, ifname) (rate(dumbmonit_if_octets_out{$T}[5m])) * 8)",
            )
            .of(SNMP)
            .unit("bit/s"),
            chart(
                "if-errors",
                "Interface errors per device",
                "sum by (target) (rate(dumbmonit_if_errors_in{$T}[5m]) + \
                 rate(dumbmonit_if_errors_out{$T}[5m]))",
            )
            .of(SNMP)
            .unit("err/s"),
            chart(
                "fg-cpu",
                "Firewall CPU (FortiGate)",
                "max by (target) (dumbmonit_fortigate_cpu_usage_percent{$T})",
            )
            .of(FORTIGATE)
            .unit("%"),
            chart(
                "fg-mem",
                "Firewall memory (FortiGate)",
                "max by (target) (dumbmonit_fortigate_memory_used_percent{$T})",
            )
            .of(FORTIGATE)
            .unit("%"),
            chart(
                "fg-ipsec",
                "IPsec tunnels up (FortiGate)",
                "sum by (target) (dumbmonit_fortigate_ipsec_tunnel_up{$T})",
            )
            .of(FORTIGATE),
            chart(
                "opn-latency",
                "Gateway latency (OPNsense)",
                "max by (target, gateway) (dumbmonit_opnsense_gateway_delay_seconds{$T}) * 1000",
            )
            .of(OPNSENSE)
            .unit("ms"),
            chart(
                "opn-loss",
                "Gateway packet loss (OPNsense)",
                "max by (target, gateway) (dumbmonit_opnsense_gateway_loss_percent{$T})",
            )
            .of(OPNSENSE)
            .unit("%"),
            chart(
                "opn-states",
                "Firewall states in use (OPNsense)",
                "max by (target) (dumbmonit_opnsense_pf_states_used_percent{$T})",
            )
            .of(OPNSENSE)
            .unit("%"),
            chart(
                "unifi-latency",
                "Internet latency (UniFi)",
                "max by (target) (dumbmonit_unifi_internet_latency_seconds{$T}) * 1000",
            )
            .of(UNIFI)
            .unit("ms"),
        ],
    },
    Template {
        id: "virtualization",
        title: "Proxmox and virtualization",
        description: "Nodes, virtual machines and containers, storages, and how recent the \
                      backups of your Proxmox VE guests are.",
        icon: "Boxes",
        kinds: &["proxmox"],
        needs_metric: None,
        range_secs: 6 * 3600,
        refresh_secs: 60,
        widgets: &[
            stat("running", "Guests running", "sum(dumbmonit_proxmox_cluster_guests_running{$T})"),
            stat("total", "Guests", "sum(dumbmonit_proxmox_cluster_guests_total{$T})"),
            stat(
                "nodes-down",
                "Nodes down",
                "count(dumbmonit_proxmox_node_up{$T} == 0) or vector(0)",
            )
            .crit(1.0),
            stat("storage", "Fullest storage", "max(dumbmonit_proxmox_storage_used_percent{$T})")
                .unit("%")
                .above(80.0, 90.0),
            chart(
                "node-cpu",
                "CPU per node",
                "max by (target, node) (dumbmonit_proxmox_node_cpu_percent{$T})",
            )
            .unit("%"),
            chart(
                "node-mem",
                "Memory per node",
                "max by (target, node) (dumbmonit_proxmox_node_memory_percent{$T})",
            )
            .unit("%"),
            chart(
                "node-iowait",
                "Disk wait per node",
                "max by (target, node) (dumbmonit_proxmox_node_cpu_iowait_percent{$T})",
            )
            .unit("%"),
            chart(
                "guest-cpu",
                "Busiest guests, CPU",
                "topk(8, max by (target, name) (dumbmonit_proxmox_guest_cpu_percent{$T}))",
            )
            .unit("%"),
            chart(
                "guest-mem",
                "Fullest guests, memory",
                "topk(8, max by (target, name) (dumbmonit_proxmox_guest_memory_percent{$T}))",
            )
            .unit("%"),
            chart(
                "storages",
                "Storage usage",
                "max by (target, storage) (dumbmonit_proxmox_storage_used_percent{$T})",
            )
            .unit("%"),
            chart(
                "backup-age",
                "Oldest last backup of a guest",
                "max by (target) (dumbmonit_proxmox_backup_last_age_seconds{$T}) / 3600",
            )
            .unit("h"),
            chart(
                "no-backup",
                "Guests without any backup",
                "max by (target) (dumbmonit_proxmox_backup_guests_without_backup{$T})",
            ),
        ],
    },
    Template {
        id: "storage-backups",
        title: "Storage and backups",
        description: "NAS volumes and pools, backup server datastores, and how long ago each \
                      backup job last succeeded.",
        icon: "HardDrive",
        kinds: &["synology", "truenas", "pbs", "kopia", "duplicati"],
        needs_metric: None,
        range_secs: 24 * 3600,
        refresh_secs: 60,
        widgets: &[
            stat("up", "Storage and backup devices up", "count(dumbmonit_up{$T} == 1)"),
            stat(
                "kopia-errors",
                "Kopia sources with errors",
                "sum(dumbmonit_kopia_sources_with_errors{$T})",
            )
            .of(KOPIA)
            .crit(1.0),
            chart(
                "syno-volumes",
                "Synology volumes used",
                "max by (target, volume) (dumbmonit_synology_volume_used_percent{$T})",
            )
            .of(SYNOLOGY)
            .unit("%"),
            chart(
                "syno-backup",
                "Synology Hyper Backup, last success",
                "max by (target) (dumbmonit_synology_backup_last_success_age_seconds{$T}) / 3600",
            )
            .of(SYNOLOGY)
            .unit("h"),
            chart(
                "syno-temp",
                "Synology disk temperature",
                "max by (target) (dumbmonit_synology_disk_temperature_celsius{$T})",
            )
            .of(SYNOLOGY)
            .unit("°C"),
            chart(
                "tn-pools",
                "TrueNAS pools used",
                "max by (target, pool) (dumbmonit_truenas_pool_used_percent{$T})",
            )
            .of(TRUENAS)
            .unit("%"),
            chart(
                "tn-scrub",
                "TrueNAS, days since the last scrub",
                "max by (target, pool) (dumbmonit_truenas_pool_last_scrub_age_seconds{$T}) / 86400",
            )
            .of(TRUENAS)
            .unit("d"),
            chart(
                "tn-temp",
                "TrueNAS disk temperature",
                "max by (target) (dumbmonit_truenas_disk_temperature_celsius{$T})",
            )
            .of(TRUENAS)
            .unit("°C"),
            chart(
                "pbs-stores",
                "Proxmox Backup Server datastores used",
                "max by (target, datastore) (dumbmonit_pbs_datastore_used_percent{$T})",
            )
            .of(PBS)
            .unit("%"),
            chart(
                "pbs-age",
                "Proxmox Backup Server, oldest last backup",
                "max by (target, datastore) (dumbmonit_pbs_backup_last_age_seconds{$T}) / 3600",
            )
            .of(PBS)
            .unit("h"),
            chart(
                "kopia-age",
                "Kopia, time since the last snapshot",
                "max by (target, source) \
                 (dumbmonit_kopia_source_last_snapshot_age_seconds{$T}) / 3600",
            )
            .of(KOPIA)
            .unit("h"),
            chart(
                "dup-age",
                "Duplicati, time since the last success",
                "max by (target, backup) \
                 (dumbmonit_duplicati_backup_last_success_age_seconds{$T}) / 3600",
            )
            .of(DUPLICATI)
            .unit("h"),
        ],
    },
    Template {
        id: "websites-tls",
        title: "Websites and TLS",
        description: "Availability, response time and certificate expiry of your websites and \
                      HTTPS endpoints.",
        icon: "Globe",
        kinds: &["http", "tls", "website"],
        needs_metric: None,
        range_secs: 24 * 3600,
        refresh_secs: 60,
        widgets: &[
            stat(
                "availability",
                "Availability, 30 days",
                "avg(avg_over_time(dumbmonit_probe_success{$T}[30d])) * 100",
            )
            .of(HTTP_TLS)
            .unit("%")
            .below(99.9, 99.0),
            stat(
                "failing",
                "Checks failing",
                "count(dumbmonit_probe_success{$T} == 0) or vector(0)",
            )
            .of(HTTP_TLS)
            .crit(1.0),
            stat(
                "web-failing",
                "Websites failing",
                "count(dumbmonit_website_up{$T} == 0) or vector(0)",
            )
            .of(WEBSITE)
            .crit(1.0),
            stat(
                "cert",
                "Soonest certificate expiry",
                "min(dumbmonit_probe_ssl_cert_expiry_days{$T})",
            )
            .of(HTTP_TLS)
            .unit("days")
            .below(21.0, 7.0),
            chart(
                "latency",
                "Response time",
                "max by (target) (dumbmonit_probe_duration_seconds{$T}) * 1000",
            )
            .of(HTTP_TLS)
            .unit("ms"),
            chart(
                "ttfb",
                "Time to first byte",
                "max by (target) (dumbmonit_probe_http_first_byte_seconds{$T}) * 1000",
            )
            .of(HTTP)
            .unit("ms"),
            chart(
                "web-latency",
                "Website response time",
                "max by (target) (dumbmonit_website_response_seconds{$T}) * 1000",
            )
            .of(WEBSITE)
            .unit("ms"),
            chart(
                "uptime-1h",
                "Availability, last hour",
                "max by (target) (avg_over_time(dumbmonit_probe_success{$T}[1h])) * 100",
            )
            .of(HTTP_TLS)
            .unit("%"),
            chart(
                "cert-days",
                "Days left on certificates",
                "min by (target) (dumbmonit_probe_ssl_cert_expiry_days{$T})",
            )
            .of(HTTP_TLS)
            .unit("days"),
        ],
    },
    Template {
        id: "active-directory",
        title: "Active Directory",
        description: "Domain controller response, replication, account hygiene and the findings \
                      of the security checks.",
        icon: "Building2",
        kinds: &["activedirectory"],
        needs_metric: None,
        range_secs: 24 * 3600,
        refresh_secs: 60,
        widgets: &[
            stat("dcs", "Domain controllers", "max(dumbmonit_ad_dc_count{$T})"),
            stat("locked", "Locked-out accounts", "max(dumbmonit_ad_users_locked_out{$T})")
                .above(5.0, 20.0),
            stat("stale", "Stale accounts", "max(dumbmonit_ad_users_stale{$T})"),
            stat(
                "krbtgt",
                "krbtgt password age",
                "max(dumbmonit_ad_krbtgt_password_age_seconds{$T}) / 86400",
            )
            .unit("days")
            .above(180.0, 365.0),
            chart(
                "connect",
                "LDAP connection time",
                "max by (target) (dumbmonit_ad_connect_seconds{$T}) * 1000",
            )
            .unit("ms"),
            chart(
                "bind",
                "LDAP bind time",
                "max by (target) (dumbmonit_ad_bind_seconds{$T}) * 1000",
            )
            .unit("ms"),
            chart(
                "skew",
                "Clock skew",
                "max by (target) (abs(dumbmonit_ad_clock_skew_seconds{$T}))",
            )
            .unit("s"),
            chart(
                "replication",
                "Replication failures in a row",
                "max by (target, source) (dumbmonit_ad_replication_consecutive_failures{$T})",
            ),
            chart(
                "lockouts",
                "Locked-out accounts over time",
                "max by (target) (dumbmonit_ad_users_locked_out{$T})",
            ),
            chart(
                "findings",
                "Security findings",
                "max by (target, title) (dumbmonit_ad_finding_count{$T} > 0)",
            ),
        ],
    },
    Template {
        id: "kubernetes",
        title: "Kubernetes",
        description: "Nodes, pods, workloads and warning events of your clusters.",
        icon: "Container",
        kinds: &["kubernetes"],
        needs_metric: None,
        range_secs: 6 * 3600,
        refresh_secs: 30,
        widgets: &[
            stat("nodes", "Nodes ready", "sum(dumbmonit_k8s_nodes_ready{$T})"),
            stat("pods", "Pods running", "sum(dumbmonit_k8s_pods_running{$T})"),
            stat("crash", "Crash-looping pods", "sum(dumbmonit_k8s_pods_crashlooping{$T})")
                .crit(1.0),
            stat("not-ready", "Pods not ready", "sum(dumbmonit_k8s_pods_not_ready{$T})")
                .above(1.0, 5.0),
            stat("events", "Warning events, last hour", "sum(dumbmonit_k8s_warning_events{$T})")
                .above(5.0, 20.0),
            chart(
                "node-down",
                "Nodes not ready",
                "count by (target) (dumbmonit_k8s_node_ready{$T} == 0) or on() vector(0)",
            ),
            chart(
                "restarts",
                "Pod restarts in the last hour, top 8",
                "topk(8, max by (target, namespace, pod) \
                 (increase(dumbmonit_k8s_pod_restarts{$T}[1h])))",
            ),
            chart(
                "unavailable",
                "Workloads with missing replicas",
                "max by (target, namespace, workload) \
                 (dumbmonit_k8s_workload_unavailable{$T} > 0)",
            ),
            chart("pending", "Pending pods", "max by (target) (dumbmonit_k8s_pods_pending{$T})"),
        ],
    },
    Template {
        id: "docker",
        title: "Docker hosts",
        description: "Containers running and stopped on the hosts whose agent watches Docker.",
        icon: "Container",
        kinds: AGENT,
        needs_metric: Some("dumbmonit_container_count"),
        range_secs: 6 * 3600,
        refresh_secs: 30,
        widgets: &[
            stat("running", "Containers running", "sum(dumbmonit_container_running_count{$T})"),
            stat("total", "Containers", "sum(dumbmonit_container_count{$T})"),
            stat(
                "stopped",
                "Containers stopped",
                "count(dumbmonit_container_up{$T} == 0) or vector(0)",
            )
            .above(1.0, 5.0),
            stat(
                "updates",
                "Image updates available",
                "sum(dumbmonit_container_update_available{$T})",
            ),
            chart(
                "per-host",
                "Containers running per host",
                "max by (target) (dumbmonit_container_running_count{$T})",
            ),
            chart(
                "stopped-host",
                "Containers stopped per host",
                "count by (target) (dumbmonit_container_up{$T} == 0) or on() vector(0)",
            ),
        ],
    },
    Template {
        id: "sla",
        title: "SLA by folder or client",
        description: "Availability of the chosen devices over 7 and 30 days, the weakest links, \
                      and the state of each folder. Pick a folder or a client to report on it.",
        icon: "Gauge",
        kinds: &[],
        needs_metric: None,
        range_secs: 7 * 86400,
        refresh_secs: 300,
        widgets: &[
            stat(
                "sla-30",
                "Availability, 30 days",
                "avg(avg_over_time(dumbmonit_up{$T}[30d])) * 100",
            )
            .unit("%")
            .below(99.9, 99.0),
            stat("sla-7", "Availability, 7 days", "avg(avg_over_time(dumbmonit_up{$T}[7d])) * 100")
                .unit("%")
                .below(99.9, 99.0),
            stat(
                "downtime",
                "Estimated downtime, 30 days",
                "sum(1 - avg_over_time(dumbmonit_up{$T}[30d])) * 43200",
            )
            .unit("min")
            .above(45.0, 240.0),
            stat("down", "Devices down now", "count(dumbmonit_up{$T} == 0) or vector(0)").crit(1.0),
            chart(
                "weakest",
                "Weakest devices, availability over 30 days",
                "bottomk(10, avg_over_time(dumbmonit_up{$T}[30d]) * 100)",
            )
            .unit("%"),
            chart(
                "hourly",
                "Availability per device, hourly",
                "max by (target) (avg_over_time(dumbmonit_up{$T}[1h])) * 100",
            )
            .unit("%"),
            folders("folders", "Folder"),
            text(
                "note",
                "How it is measured",
                "Availability is the share of checks that found the device up over the window. \
                 A device that was disabled or not yet added is not counted for that period.",
            ),
        ],
    },
];

pub fn find(id: &str) -> Option<&'static Template> {
    TEMPLATES.iter().find(|template| template.id == id)
}
