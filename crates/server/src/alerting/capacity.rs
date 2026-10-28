//! Familles de capacité : tout ce qui se remplit et dont on veut savoir quand
//! ce sera plein.
//!
//! Chaque famille est une expression MetricsQL qui rend un **taux
//! d'occupation en pourcentage** (plein à 100), par entité : un point de
//! montage, un datastore, un pool, un volume, une licence. Les intégrations
//! n'ont rien à faire de plus que publier leur jauge : ajouter une famille, c'est
//! une ligne ici, et la prévision, l'alerte « plein dans N jours » et la route
//! `GET /api/targets/{id}/forecasts` en profitent aussitôt.
//!
//! `used` et `size` sont facultatifs : ce sont les grandeurs brutes (octets,
//! sièges) que l'interface affiche à côté du pourcentage quand l'intégration
//! les publie avec les mêmes étiquettes.

use std::collections::BTreeMap;

/// Une famille de capacité.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Family {
    /// Identifiant stable, utilisable dans la requête d'une règle de prévision.
    pub id: &'static str,
    /// Nom affiché.
    pub label: &'static str,
    /// Taux d'occupation en pourcentage. Un nom de métrique nu est regroupé avec
    /// les autres dans une seule requête ; une expression est interrogée seule.
    pub percent: &'static str,
    /// Grandeur utilisée, dans `unit`.
    pub used: Option<&'static str>,
    /// Grandeur totale, dans `unit`.
    pub size: Option<&'static str>,
    /// Unité de `used` et `size` : `bytes`, `seats`… Vide sans grandeur brute.
    pub unit: &'static str,
    /// Profondeur minimale de l'historique, en jours : une batterie vieillit sur
    /// des mois, pas sur une semaine.
    pub min_window_days: u32,
}

const fn pct(id: &'static str, label: &'static str, percent: &'static str) -> Family {
    Family { id, label, percent, used: None, size: None, unit: "", min_window_days: 0 }
}

const fn bytes(
    id: &'static str,
    label: &'static str,
    percent: &'static str,
    used: &'static str,
    size: &'static str,
) -> Family {
    Family {
        id,
        label,
        percent,
        used: Some(used),
        size: Some(size),
        unit: "bytes",
        min_window_days: 0,
    }
}

/// Identifiant de la famille d'une règle à requête personnalisée.
pub const CUSTOM: &str = "custom";

/// Toutes les familles connues.
pub const FAMILIES: &[Family] = &[
    // Agent (Linux, Windows) et SNMP (HOST-RESOURCES-MIB).
    bytes(
        "filesystem",
        "Filesystems",
        "dumbmonit_filesystem_used_percent",
        "dumbmonit_filesystem_used_bytes",
        "dumbmonit_filesystem_total_bytes",
    ),
    // Inodes de l'agent Linux : une table pleine bloque les écritures alors
    // qu'il reste de la place.
    pct("filesystem_inodes", "Filesystem inodes", "dumbmonit_filesystem_inodes_used_percent"),
    bytes(
        "snmp_storage",
        "Storage (SNMP)",
        "100 * dumbmonit_storage_bytes_used / dumbmonit_storage_bytes_total",
        "dumbmonit_storage_bytes_used",
        "dumbmonit_storage_bytes_total",
    ),
    bytes(
        "zfs_pool",
        "ZFS pools",
        "dumbmonit_agent_zfs_pool_used_percent",
        "dumbmonit_agent_zfs_pool_used_bytes",
        "dumbmonit_agent_zfs_pool_size_bytes",
    ),
    pct("libvirt_pool", "libvirt storage pools", "dumbmonit_agent_libvirt_pool_used_percent"),
    // Proxmox VE, Proxmox Backup Server, Proxmox Datacenter Manager.
    pct("proxmox_storage", "Proxmox storages", "dumbmonit_proxmox_storage_used_percent"),
    pct("proxmox_rootfs", "Proxmox root filesystems", "dumbmonit_proxmox_node_rootfs_percent"),
    pct(
        "proxmox_thinpool",
        "Proxmox LVM thin pools",
        "dumbmonit_proxmox_node_thinpool_used_percent",
    ),
    pct("proxmox_zfs", "Proxmox ZFS pools", "dumbmonit_proxmox_node_zfs_pool_used_percent"),
    pct("proxmox_lvm", "Proxmox volume groups", "dumbmonit_proxmox_node_lvm_vg_used_percent"),
    pct("proxmox_guest_fs", "Guest filesystems", "dumbmonit_proxmox_guest_fs_used_percent"),
    pct("proxmox_ceph_pool", "Proxmox Ceph pools", "dumbmonit_proxmox_ceph_pool_used_percent"),
    bytes(
        "pbs_datastore",
        "Backup datastores",
        "dumbmonit_pbs_datastore_used_percent",
        "dumbmonit_pbs_datastore_bytes_used",
        "dumbmonit_pbs_datastore_bytes_total",
    ),
    pct("pdm_rootfs", "Datacenter Manager root filesystems", "dumbmonit_pdm_node_rootfs_percent"),
    // NAS.
    bytes(
        "truenas_pool",
        "TrueNAS pools",
        "dumbmonit_truenas_pool_used_percent",
        "dumbmonit_truenas_pool_size_bytes - dumbmonit_truenas_pool_free_bytes",
        "dumbmonit_truenas_pool_size_bytes",
    ),
    // Zvols fins (sans `refreservation`) : ce qui est écrit, sur leur taille.
    bytes(
        "truenas_zvol",
        "TrueNAS thin zvols",
        "dumbmonit_truenas_zvol_used_percent",
        "dumbmonit_truenas_zvol_allocated_bytes",
        "dumbmonit_truenas_zvol_size_bytes",
    ),
    pct(
        "truenas_dataset_quota",
        "TrueNAS dataset quotas",
        "dumbmonit_truenas_dataset_quota_used_percent",
    ),
    // L'occupation *effective* : sans les réservations des LUN iSCSI épais, qui
    // remplissent un volume par construction (`collectors/synology/iscsi.rs`).
    // Sans LUN, elle vaut exactement l'occupation brute.
    bytes(
        "synology_volume",
        "Synology volumes",
        "dumbmonit_synology_volume_effective_used_percent",
        "dumbmonit_synology_volume_effective_used_bytes",
        "dumbmonit_synology_volume_effective_total_bytes",
    ),
    // LUN fins : leur allocation se remplit ; un LUN épais n'a pas de série ici,
    // le NAS ne voit pas l'intérieur.
    bytes(
        "synology_lun",
        "Synology thin LUNs",
        "dumbmonit_synology_lun_used_percent",
        "dumbmonit_synology_lun_allocated_bytes",
        "dumbmonit_synology_lun_size_bytes",
    ),
    bytes(
        "qnap_volume",
        "QNAP volumes",
        "dumbmonit_qnap_volume_used_percent",
        "dumbmonit_qnap_volume_size_bytes - dumbmonit_qnap_volume_free_bytes",
        "dumbmonit_qnap_volume_size_bytes",
    ),
    bytes(
        "unraid_array",
        "Unraid array",
        "dumbmonit_unraid_array_used_percent",
        "dumbmonit_unraid_array_used_bytes",
        "dumbmonit_unraid_array_size_bytes",
    ),
    pct("unraid_disk", "Unraid disks", "dumbmonit_unraid_disk_used_percent"),
    // Stockage distribué et hyperviseurs.
    bytes(
        "ceph_cluster",
        "Ceph cluster",
        "dumbmonit_ceph_capacity_used_percent",
        "dumbmonit_ceph_capacity_used_bytes",
        "dumbmonit_ceph_capacity_bytes",
    ),
    pct("ceph_pool", "Ceph pools", "dumbmonit_ceph_pool_used_percent"),
    bytes(
        "vsphere_datastore",
        "vSphere datastores",
        "dumbmonit_vsphere_datastore_used_percent",
        "dumbmonit_vsphere_datastore_capacity_bytes - dumbmonit_vsphere_datastore_free_bytes",
        "dumbmonit_vsphere_datastore_capacity_bytes",
    ),
    bytes(
        "nutanix_container",
        "Nutanix storage containers",
        "dumbmonit_nutanix_storage_container_used_percent",
        "dumbmonit_nutanix_storage_container_used_bytes",
        "dumbmonit_nutanix_storage_container_capacity_bytes",
    ),
    // Équipements réseau.
    pct("mikrotik_storage", "MikroTik storage", "dumbmonit_mikrotik_storage_used_percent"),
    pct("opnsense_disk", "OPNsense disk", "dumbmonit_opnsense_disk_used_percent"),
    pct("pfsense_disk", "pfSense disk", "dumbmonit_pfsense_disk_used_percent"),
    pct("fortigate_disk", "FortiGate disk", "dumbmonit_fortigate_disk_used_percent"),
    // Bases et applications.
    pct(
        "elasticsearch_disk",
        "Elasticsearch disks",
        "dumbmonit_elasticsearch_node_disk_used_percent",
    ),
    pct("mssql_volume", "SQL Server volumes", "dumbmonit_mssql_volume_used_percent"),
    pct(
        "victoriametrics_disk",
        "VictoriaMetrics disk",
        "100 - dumbmonit_victoriametrics_disk_headroom_percent",
    ),
    pct(
        "victorialogs_disk",
        "VictoriaLogs disk",
        "100 - dumbmonit_victorialogs_disk_headroom_percent",
    ),
    pct("immich_storage", "Immich storage", "dumbmonit_immich_storage_used_percent"),
    pct("frigate_storage", "Frigate storage", "dumbmonit_frigate_storage_used_percent"),
    pct("paperless_storage", "Paperless storage", "dumbmonit_paperless_storage_used_percent"),
    // Licences : des sièges, pas des octets.
    Family {
        id: "m365_licenses",
        label: "Microsoft 365 licences",
        percent: "dumbmonit_m365_license_used_percent",
        used: Some("dumbmonit_m365_license_consumed"),
        size: Some("dumbmonit_m365_license_enabled"),
        unit: "seats",
        min_window_days: 0,
    },
    // Onduleurs : la part de l'autonomie déjà « consommée » par rapport au seuil
    // d'arrêt. Elle monte vers 100 % à mesure que la batterie vieillit et que
    // l'autonomie mesurée se rapproche de l'autonomie minimale.
    Family {
        id: "ups_battery",
        label: "UPS battery runtime",
        percent: "100 * dumbmonit_nut_battery_runtime_low_seconds \
                  / dumbmonit_nut_battery_runtime_seconds",
        used: None,
        size: None,
        unit: "",
        min_window_days: 30,
    },
];

/// Retrouve une famille par son identifiant.
pub fn family(id: &str) -> Option<&'static Family> {
    FAMILIES.iter().find(|f| f.id == id)
}

/// Ce qu'une règle de prévision surveille.
#[derive(Debug, Clone, PartialEq)]
pub enum Selection {
    /// Des familles du catalogue (toutes pour `*`).
    Families(Vec<&'static Family>),
    /// Une expression écrite par l'utilisateur, pleine à `forecast_limit`.
    Custom(String),
}

/// Lit la requête d'une règle de prévision : `*` (ou vide) pour toutes les
/// familles, une liste d'identifiants séparés par des virgules, ou sinon une
/// expression MetricsQL personnalisée.
pub fn select(query: &str) -> Selection {
    let query = query.trim();
    if query.is_empty() || query == "*" {
        return Selection::Families(FAMILIES.iter().collect());
    }
    let ids: Vec<&str> = query.split(',').map(str::trim).filter(|id| !id.is_empty()).collect();
    let found: Vec<&'static Family> = ids.iter().filter_map(|id| family(id)).collect();
    if !ids.is_empty() && found.len() == ids.len() {
        Selection::Families(found)
    } else {
        Selection::Custom(query.to_string())
    }
}

/// Vrai si l'expression est un nom de métrique nu, regroupable par `__name__`.
pub fn is_bare_metric(expr: &str) -> bool {
    !expr.is_empty() && expr.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':')
}

/// Étiquettes qui décrivent l'équipement ou la collecte, pas l'entité.
const NOT_ENTITY: &[&str] =
    &["__name__", "target", "target_id", "host", "hostname", "instance", "job", "family"];

/// Étiquettes qui nomment le mieux une entité, par ordre de préférence.
const ENTITY_PRIORITY: &[&str] = &[
    "mountpoint",
    "datastore",
    "storage",
    "pool",
    "volume",
    "dataset",
    "container",
    "share",
    "disk",
    "path",
    "sku",
    "license",
    "name",
    "ups",
    "vmid",
    "node",
];

/// Nom lisible de l'entité d'une série : la meilleure étiquette descriptive,
/// sinon toutes les étiquettes restantes, sinon chaîne vide (l'équipement
/// lui-même).
pub fn entity(labels: &BTreeMap<String, String>) -> String {
    let describes = |key: &str| {
        !NOT_ENTITY.contains(&key) && !key.starts_with(crate::alerting::model::TAG_LABEL_PREFIX)
    };
    for key in ENTITY_PRIORITY {
        if let Some(value) = labels.get(*key).filter(|v| !v.is_empty()) {
            // Un invité Proxmox : le point de montage seul ne dit pas de quelle VM.
            if *key == "mountpoint"
                && let Some(guest) = labels.get("vmid").or_else(|| labels.get("guest"))
            {
                return format!("{guest} {value}");
            }
            return value.clone();
        }
    }
    labels
        .iter()
        .filter(|(key, value)| describes(key) && !value.is_empty())
        .map(|(_, value)| value.as_str())
        .collect::<Vec<_>>()
        .join(" · ")
}

/// Clé de rapprochement entre la série du pourcentage et celles de `used` et
/// `size` : les étiquettes sans le nom de métrique.
pub fn match_key(labels: &BTreeMap<String, String>) -> String {
    let mut stripped = labels.clone();
    stripped.remove("__name__");
    stripped.remove("family");
    crate::alerting::model::series_key(&stripped)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect()
    }

    #[test]
    fn les_identifiants_de_famille_sont_uniques() {
        let mut ids: Vec<&str> = FAMILIES.iter().map(|f| f.id).collect();
        ids.sort_unstable();
        let total = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), total);
        assert!(family(CUSTOM).is_none(), "`custom` is reserved");
    }

    /// Même garde-fou que pour les règles livrées : une famille qui vise une
    /// métrique que personne ne publie ne prévoirait jamais rien, en silence.
    #[test]
    fn les_familles_ne_visent_que_des_metriques_produites() {
        for family in FAMILIES {
            let exprs = [Some(family.percent), family.used, family.size];
            for expr in exprs.into_iter().flatten() {
                for word in expr.split(|c: char| !c.is_alphanumeric() && c != '_') {
                    if word.starts_with("dumbmonit_") {
                        assert!(
                            crate::alerting::rules::PRODUITES.contains(&word),
                            "family \"{}\" targets \"{word}\", which no collector produces",
                            family.id
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn la_requete_choisit_les_familles_ou_une_expression() {
        assert_eq!(select("*"), Selection::Families(FAMILIES.iter().collect()));
        assert_eq!(select(""), Selection::Families(FAMILIES.iter().collect()));
        match select("filesystem, pbs_datastore") {
            Selection::Families(found) => {
                let ids: Vec<&str> = found.iter().map(|f| f.id).collect();
                assert_eq!(ids, ["filesystem", "pbs_datastore"]);
            }
            other => panic!("unexpected {other:?}"),
        }
        assert_eq!(
            select("dumbmonit_kafka_log_size_bytes"),
            Selection::Custom("dumbmonit_kafka_log_size_bytes".to_string())
        );
        // Un identifiant inconnu dans la liste : c'est une expression, pas une
        // famille silencieusement ignorée.
        assert!(matches!(select("filesystem,unknown"), Selection::Custom(_)));
    }

    #[test]
    fn un_nom_nu_se_distingue_d_une_expression() {
        assert!(is_bare_metric("dumbmonit_filesystem_used_percent"));
        assert!(!is_bare_metric("100 - dumbmonit_victoriametrics_disk_headroom_percent"));
        assert!(!is_bare_metric("x{a=\"b\"}"));
        assert!(!is_bare_metric(""));
    }

    #[test]
    fn l_entite_prend_l_etiquette_la_plus_parlante() {
        let fs = labels(&[
            ("__name__", "dumbmonit_filesystem_used_percent"),
            ("target", "3"),
            ("host", "nas"),
            ("mountpoint", "/srv"),
            ("device", "/dev/sda1"),
            ("fstype", "ext4"),
        ]);
        assert_eq!(entity(&fs), "/srv");
        let guest = labels(&[("vmid", "101"), ("mountpoint", "/"), ("target", "1")]);
        assert_eq!(entity(&guest), "101 /");
        let other =
            labels(&[("target", "1"), ("tag_room", "cellar"), ("zone", "eu"), ("tier", "a")]);
        assert_eq!(entity(&other), "a · eu");
        assert_eq!(entity(&labels(&[("target", "1")])), "");
    }

    #[test]
    fn le_rapprochement_ignore_le_nom_de_metrique() {
        let percent = labels(&[("__name__", "a_percent"), ("target", "1"), ("pool", "tank")]);
        let size = labels(&[("target", "1"), ("pool", "tank")]);
        assert_eq!(match_key(&percent), match_key(&size));
    }
}
