//! L'inventaire logiciel : pour chaque équipement, ce qui y est installé et
//! en quelle version, tiré de ce que les collecteurs ont **déjà** relevé —
//! séries `*_version_info` dans VictoriaMetrics, paquets de Proxmox
//! (`apt/versions`), inventaire quotidien de l'agent (paquets Debian, RPM,
//! Alpine ; build et programmes Windows). Rien n'est redemandé aux
//! équipements.

use std::collections::{BTreeMap, HashSet};

use dumbmonit_proto::{SoftwareInventory, TargetId};
use serde::{Deserialize, Serialize};

use super::catalog::{Catalog, MetricRule};
use super::osv;

/// Où chercher les failles d'un composant.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Origin {
    /// NVD, par CPE. `vendor_build` : version reconstruite par un éditeur
    /// ou une distribution, comparée à l'amont sans pouvoir conclure à la
    /// vulnérabilité (un correctif a pu être rétroporté).
    Nvd { product: String, cpes: Vec<String>, vendor_build: bool },
    /// Debian Security Tracker : paquet source et nom de code.
    Debian { package: String, release: String },
    /// OSV.dev : écosystème et paquet.
    Osv { ecosystem: String, package: String },
    /// Produit reconnu qu'aucune source ne couvre.
    None,
}

impl Origin {
    /// Libellé de l'API (`nvd`, `debian:bookworm`, `osv:Ubuntu:24.04:LTS`).
    pub fn label(&self) -> String {
        match self {
            Self::Nvd { .. } => "nvd".to_string(),
            Self::Debian { release, .. } => format!("debian:{release}"),
            Self::Osv { ecosystem, .. } => format!("osv:{ecosystem}"),
            Self::None => "none".to_string(),
        }
    }
}

/// Un logiciel installé sur un équipement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    pub target_id: TargetId,
    /// Nom affiché, unique par équipement : `Proxmox VE (pve1)`, `openssl`.
    pub name: String,
    pub version: String,
    pub origin: Origin,
}

/// Une série lue dans VictoriaMetrics : nom sans préfixe et étiquettes.
#[derive(Debug, Clone)]
pub struct Series {
    pub name: String,
    pub labels: BTreeMap<String, String>,
}

impl Series {
    fn label(&self, key: &str) -> Option<&str> {
        self.labels.get(key).map(String::as_str).filter(|v| !v.is_empty())
    }
}

/// Construit l'inventaire de tout le parc.
pub struct Builder<'a> {
    catalog: &'a Catalog,
    out: Vec<Component>,
    seen: HashSet<(TargetId, String)>,
}

impl<'a> Builder<'a> {
    pub fn new(catalog: &'a Catalog) -> Self {
        Self { catalog, out: Vec::new(), seen: HashSet::new() }
    }

    fn push(&mut self, target_id: TargetId, name: String, version: String, origin: Origin) {
        if version.is_empty() || !self.seen.insert((target_id, name.clone())) {
            return;
        }
        self.out.push(Component { target_id, name, version, origin });
    }

    fn product_origin(&self, key: &str, vendor_build: bool) -> Option<(String, Origin)> {
        let product = self.catalog.product(key)?;
        let origin = if product.cpes.is_empty() {
            Origin::None
        } else {
            Origin::Nvd { product: product.key.clone(), cpes: product.cpes.clone(), vendor_build }
        };
        Some((product.name.clone(), origin))
    }

    /// Les séries d'un équipement (`*_version_info`, paquets Proxmox).
    pub fn add_series(&mut self, target_id: TargetId, series: &[Series]) {
        // Version de Debian sous chaque nœud Proxmox, d'après la version
        // majeure de Proxmox VE qu'il fait tourner.
        let mut releases: BTreeMap<String, String> = BTreeMap::new();
        for one in series.iter().filter(|s| s.name == "proxmox_node_pve_version_info") {
            let (Some(node), Some(version)) = (one.label("node"), one.label("version")) else {
                continue;
            };
            let major = version.split('.').next().unwrap_or_default();
            if let Some(release) = self.catalog.proxmox_releases.get(major) {
                releases.insert(node.to_string(), release.clone());
            }
        }
        let catalog = self.catalog;
        for rule in &catalog.metrics {
            for one in series.iter().filter(|s| s.name == rule.metric) {
                if rule.when.iter().any(|(key, value)| one.label(key) != Some(value.as_str())) {
                    continue;
                }
                let Some(version) = one.label(&rule.label).and_then(|raw| rule.extract(raw)) else {
                    continue;
                };
                let node = one.label("node");
                let suffix = |name: &str| match node {
                    Some(node) => format!("{name} ({node})"),
                    None => name.to_string(),
                };
                match &rule.package_label {
                    Some(package_label) => {
                        let Some(package) = one.label(package_label) else { continue };
                        self.add_package(target_id, rule, package, &version, node, &releases);
                    }
                    None => {
                        let Some(key) = rule.product.as_deref() else { continue };
                        if let Some((name, origin)) = self.product_origin(key, rule.vendor_build) {
                            self.push(target_id, suffix(&name), version, origin);
                        }
                    }
                }
            }
        }
    }

    /// Un paquet d'un nœud Proxmox : produit du catalogue s'il en est un
    /// (`pve-qemu-kvm` → QEMU), sinon paquet Debian du même nom.
    fn add_package(
        &mut self,
        target_id: TargetId,
        rule: &MetricRule,
        package: &str,
        version: &str,
        node: Option<&str>,
        releases: &BTreeMap<String, String>,
    ) {
        let name = match node {
            Some(node) => format!("{package} ({node})"),
            None => package.to_string(),
        };
        if let Some(key) = rule.packages.get(package) {
            if let Some((_, origin)) = self.product_origin(key, rule.vendor_build) {
                self.push(target_id, name, version.to_string(), origin);
            }
            return;
        }
        let release = node.and_then(|node| releases.get(node));
        if let Some(release) = release {
            let origin = Origin::Debian { package: package.to_string(), release: release.clone() };
            self.push(target_id, name, version.to_string(), origin);
        }
    }

    /// L'inventaire quotidien d'un agent.
    pub fn add_agent(&mut self, target_id: TargetId, inventory: &SoftwareInventory) {
        if inventory.source == "windows" {
            self.add_windows(target_id, inventory);
            return;
        }
        let distro = inventory.distro_id.as_deref().unwrap_or_default();
        let version_id = inventory.distro_version.as_deref().unwrap_or_default();
        let debian_release = match distro {
            "debian" | "raspbian" => inventory
                .distro_codename
                .clone()
                .filter(|c| !c.is_empty())
                .or_else(|| self.catalog.debian_releases.get(version_id).cloned()),
            _ => None,
        };
        let ecosystem = osv::ecosystem(distro, version_id);
        for package in &inventory.packages {
            // Les avis Debian, Ubuntu et Alpine sont rangés par paquet
            // source ; ceux des RPM, par paquet binaire.
            let by_source = inventory.source != "rpm";
            let (name, version) = if by_source {
                (
                    package
                        .source_name
                        .as_deref()
                        .filter(|s| !s.is_empty())
                        .unwrap_or(&package.name),
                    package
                        .source_version
                        .as_deref()
                        .filter(|s| !s.is_empty())
                        .unwrap_or(&package.version),
                )
            } else {
                (package.name.as_str(), package.version.as_str())
            };
            let origin = if let Some(release) = &debian_release {
                Origin::Debian { package: name.to_string(), release: release.clone() }
            } else if let Some(ecosystem) = &ecosystem {
                Origin::Osv { ecosystem: ecosystem.clone(), package: name.to_string() }
            } else {
                continue;
            };
            self.push(target_id, name.to_string(), version.to_string(), origin);
        }
    }

    fn add_windows(&mut self, target_id: TargetId, inventory: &SoftwareInventory) {
        let catalog = self.catalog;
        if let (Some(product), Some(build)) = (&inventory.os_product, &inventory.os_build)
            && let Some((name, cpe)) = windows_cpe(product, inventory.os_release.as_deref(), build)
        {
            let origin = Origin::Nvd { product: cpe.clone(), cpes: vec![cpe], vendor_build: false };
            self.push(target_id, name, build.clone(), origin);
        }
        for package in &inventory.packages {
            let Some(rule) = catalog.windows_programs.iter().find(|p| p.matches(&package.name))
            else {
                continue;
            };
            if let Some((name, origin)) = self.product_origin(&rule.product, false) {
                self.push(target_id, name, package.version.clone(), origin);
            }
        }
    }

    pub fn finish(self) -> Vec<Component> {
        self.out
    }
}

/// Le CPE NVD d'une version de Windows, avec son nom lisible : la NVD range
/// les failles par version commerciale (`windows_11_23h2`,
/// `windows_server_2022`) et borne chacune par un numéro de build
/// (`versionEndExcluding: 10.0.22631.4317`).
pub fn windows_cpe(product: &str, release: Option<&str>, build: &str) -> Option<(String, String)> {
    let release = release.map(|r| r.trim().to_ascii_lowercase()).filter(|r| !r.is_empty());
    let lower = product.to_ascii_lowercase();
    if lower.contains("server") {
        let year = ["2025", "2022", "2019", "2016", "2012"].into_iter().find(|y| lower.contains(y));
        return match (year, release.as_deref()) {
            (Some(year), _) => Some((
                format!("Windows Server {year}"),
                format!("o:microsoft:windows_server_{year}"),
            )),
            // Les versions semestrielles (« Windows Server, version 23H2 »).
            (None, Some("23h2")) => Some((
                "Windows Server 23H2".to_string(),
                "o:microsoft:windows_server_2022_23h2".to_string(),
            )),
            _ => None,
        };
    }
    let release = release?;
    let generation = if lower.contains("windows 11") {
        "11"
    } else if lower.contains("windows 10") {
        // Le registre de Windows 11 dit encore « Windows 10 » : le build
        // tranche (22000 et au-delà).
        let build_number: u64 = build.split('.').nth(2).and_then(|b| b.parse().ok()).unwrap_or(0);
        if build_number >= 22000 { "11" } else { "10" }
    } else {
        return None;
    };
    Some((
        format!("Windows {generation} {}", release.to_ascii_uppercase()),
        format!("o:microsoft:windows_{generation}_{release}"),
    ))
}

#[cfg(test)]
mod tests {
    use dumbmonit_proto::InstalledPackage;

    use super::super::catalog::catalog;
    use super::*;

    fn series(name: &str, labels: &[(&str, &str)]) -> Series {
        Series {
            name: name.to_string(),
            labels: labels.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect(),
        }
    }

    #[test]
    fn les_series_d_un_cluster_proxmox_donnent_produit_et_paquets() {
        let mut builder = Builder::new(catalog());
        builder.add_series(
            1,
            &[
                series(
                    "proxmox_node_pve_version_info",
                    &[("node", "pve1"), ("version", "8.2.4"), ("release", "8.2")],
                ),
                series(
                    "proxmox_node_package_info",
                    &[("node", "pve1"), ("package", "pve-qemu-kvm"), ("version", "9.0.2-2")],
                ),
                series(
                    "proxmox_node_package_info",
                    &[("node", "pve1"), ("package", "lvm2"), ("version", "2.03.16-2")],
                ),
                // Un nœud dont la version de Proxmox est inconnue : ses
                // paquets Debian n'ont pas de version de Debian, ils sautent.
                series(
                    "proxmox_node_package_info",
                    &[("node", "pve2"), ("package", "lvm2"), ("version", "2.03.16-2")],
                ),
                series("unrelated_info", &[("version", "1.0")]),
            ],
        );
        let components = builder.finish();
        let names: Vec<&str> = components.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["Proxmox VE (pve1)", "pve-qemu-kvm (pve1)", "lvm2 (pve1)"]);
        assert_eq!(components[0].origin.label(), "nvd");
        assert!(matches!(
            &components[1].origin,
            Origin::Nvd { product, vendor_build: true, .. } if product == "qemu"
        ));
        assert_eq!(components[2].origin.label(), "debian:bookworm");
    }

    #[test]
    fn les_conditions_et_motifs_des_regles_s_appliquent() {
        let mut builder = Builder::new(catalog());
        builder.add_series(
            2,
            &[
                series("vsphere_info", &[("product", "esxi"), ("version", "8.0.3")]),
                series("redis_version_info", &[("product", "valkey"), ("version", "8.0.1")]),
                series("apache_version_info", &[("version", "Apache/2.4.58 (Unix)")]),
                series("truenas_version_info", &[("version", "25.04.2")]),
                series("nginx_version_info", &[("server", "nginx")]),
            ],
        );
        let components = builder.finish();
        let found: Vec<(&str, &str, String)> = components
            .iter()
            .map(|c| (c.name.as_str(), c.version.as_str(), c.origin.label()))
            .collect();
        assert_eq!(
            found,
            [
                ("TrueNAS", "25.04.2", "none".to_string()),
                ("VMware ESXi", "8.0.3", "nvd".to_string()),
                ("Apache HTTP Server", "2.4.58", "nvd".to_string()),
                ("Valkey", "8.0.1", "nvd".to_string()),
            ]
        );
    }

    fn package(name: &str, version: &str, source: Option<&str>) -> InstalledPackage {
        InstalledPackage {
            name: name.into(),
            version: version.into(),
            source_name: source.map(Into::into),
            ..InstalledPackage::default()
        }
    }

    #[test]
    fn un_agent_debian_est_lu_par_paquet_source() {
        let inventory = SoftwareInventory {
            source: "dpkg".into(),
            distro_id: Some("debian".into()),
            distro_version: Some("12".into()),
            packages: vec![
                package("libssl3", "3.0.15-1~deb12u1", Some("openssl")),
                package("openssl", "3.0.15-1~deb12u1", Some("openssl")),
                package("bash", "5.2.15-2+b7", None),
            ],
            ..SoftwareInventory::default()
        };
        let mut builder = Builder::new(catalog());
        builder.add_agent(3, &inventory);
        let components = builder.finish();
        let names: Vec<&str> = components.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["openssl", "bash"], "un composant par paquet source");
        assert_eq!(components[0].origin.label(), "debian:bookworm", "nom de code déduit de 12");

        let ubuntu = SoftwareInventory {
            source: "dpkg".into(),
            distro_id: Some("ubuntu".into()),
            distro_version: Some("24.04".into()),
            packages: vec![package("openssh-server", "1:9.6p1-3ubuntu13", Some("openssh"))],
            ..SoftwareInventory::default()
        };
        let mut builder = Builder::new(catalog());
        builder.add_agent(4, &ubuntu);
        let components = builder.finish();
        assert_eq!(components[0].origin.label(), "osv:Ubuntu:24.04:LTS");
        assert_eq!(components[0].name, "openssh");

        let arch = SoftwareInventory {
            source: "pacman".into(),
            distro_id: Some("arch".into()),
            packages: vec![package("openssl", "3.4.0-1", None)],
            ..SoftwareInventory::default()
        };
        let mut builder = Builder::new(catalog());
        builder.add_agent(5, &arch);
        assert!(builder.finish().is_empty(), "aucune source pour Arch");
    }

    #[test]
    fn un_agent_windows_donne_son_build_et_ses_programmes() {
        let inventory = SoftwareInventory {
            source: "windows".into(),
            os_product: Some("Windows 10 Pro".into()),
            os_release: Some("23H2".into()),
            os_build: Some("10.0.22631.4317".into()),
            packages: vec![
                package("Mozilla Firefox (x64 fr)", "131.0.3", None),
                package("Some Internal Tool", "1.0", None),
            ],
            ..SoftwareInventory::default()
        };
        let mut builder = Builder::new(catalog());
        builder.add_agent(6, &inventory);
        let components = builder.finish();
        let names: Vec<&str> = components.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["Windows 11 23H2", "Mozilla Firefox"]);
        assert!(matches!(
            &components[0].origin,
            Origin::Nvd { cpes, .. } if cpes == &["o:microsoft:windows_11_23h2"]
        ));
        assert_eq!(components[0].version, "10.0.22631.4317");
    }

    #[test]
    fn le_cpe_windows_suit_la_version_commerciale() {
        let cpe = |p: &str, r: Option<&str>, b: &str| windows_cpe(p, r, b).map(|(_, cpe)| cpe);
        assert_eq!(
            cpe("Windows Server 2022 Standard", Some("21H2"), "10.0.20348.2762").as_deref(),
            Some("o:microsoft:windows_server_2022")
        );
        assert_eq!(
            cpe("Windows 10 Pro", Some("22H2"), "10.0.19045.5011").as_deref(),
            Some("o:microsoft:windows_10_22h2")
        );
        assert_eq!(cpe("Windows 10 Pro", None, "10.0.19045.5011"), None);
        assert_eq!(cpe("Windows 8.1", Some("x"), "6.3.9600"), None);
    }
}
