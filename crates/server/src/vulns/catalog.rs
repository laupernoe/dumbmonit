//! Le catalogue embarqué (`catalog.json`) : quels produits la NVD connaît
//! (CPE), quelles séries collectées portent leur version, quels programmes
//! Windows les représentent. Ajouter un produit, c'est ajouter quelques
//! lignes de données — aucune ligne de code.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use regex::Regex;
use serde::Deserialize;

const SOURCE: &str = include_str!("catalog.json");

#[derive(Debug, Deserialize)]
pub struct Product {
    pub key: String,
    pub name: String,
    /// `part:vendor:product` (`a:proxmox:virtual_environment`). Vide : le
    /// produit est inventorié, mais aucune source ne le couvre.
    #[serde(default)]
    pub cpes: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct MetricRule {
    /// Nom de la série, sans le préfixe `dumbmonit_`.
    pub metric: String,
    /// Étiquette qui porte la version.
    pub label: String,
    /// Produit du catalogue (absent pour une règle de paquets).
    #[serde(default)]
    pub product: Option<String>,
    /// Ne retenir que les séries dont ces étiquettes valent ceci.
    #[serde(default)]
    pub when: BTreeMap<String, String>,
    /// Expression dont le premier groupe extrait la version de l'étiquette.
    #[serde(default)]
    pub pattern: Option<String>,
    /// Règle de paquets : étiquette qui porte le nom du paquet.
    #[serde(default)]
    pub package_label: Option<String>,
    /// Règle de paquets : paquet → produit du catalogue.
    #[serde(default)]
    pub packages: BTreeMap<String, String>,
    /// Version reconstruite par l'éditeur (correctifs rétroportés possibles).
    #[serde(default)]
    pub vendor_build: bool,
    #[serde(skip)]
    regex: OnceLock<Option<Regex>>,
}

impl MetricRule {
    /// La version lue dans la valeur de l'étiquette ; `None` si le motif ne
    /// la trouve pas.
    pub fn extract(&self, raw: &str) -> Option<String> {
        let raw = raw.trim();
        let version = match &self.pattern {
            None => raw,
            Some(pattern) => {
                let regex = self.regex.get_or_init(|| Regex::new(pattern).ok()).as_ref()?;
                regex.captures(raw)?.get(1)?.as_str()
            }
        };
        let version = version.strip_prefix(['v', 'V']).unwrap_or(version).trim();
        (!version.is_empty()).then(|| version.to_string())
    }
}

#[derive(Debug, Deserialize)]
pub struct WindowsProgram {
    pub prefix: String,
    pub product: String,
    #[serde(default)]
    pub exclude: Option<String>,
}

impl WindowsProgram {
    pub fn matches(&self, display_name: &str) -> bool {
        let name = display_name.to_ascii_lowercase();
        name.starts_with(&self.prefix.to_ascii_lowercase())
            && self.exclude.as_ref().is_none_or(|ex| !name.contains(&ex.to_ascii_lowercase()))
    }
}

#[derive(Debug, Deserialize)]
pub struct Catalog {
    pub products: Vec<Product>,
    pub metrics: Vec<MetricRule>,
    pub windows_programs: Vec<WindowsProgram>,
    /// Version majeure de Proxmox (VE, PBS, PMG) → nom de code Debian.
    pub proxmox_releases: BTreeMap<String, String>,
    /// `VERSION_ID` de Debian → nom de code.
    pub debian_releases: BTreeMap<String, String>,
}

impl Catalog {
    pub fn product(&self, key: &str) -> Option<&Product> {
        self.products.iter().find(|p| p.key == key)
    }

    /// Les séries lues pour l'inventaire, dédoublonnées.
    pub fn metric_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.metrics.iter().map(|m| m.metric.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        names
    }
}

pub fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        // Vérifié par les tests : une erreur ici est une erreur de compilation
        // du catalogue, pas une donnée venue de l'extérieur.
        serde_json::from_str(SOURCE).expect("catalog.json is valid")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_catalogue_est_coherent() {
        let catalog = catalog();
        let mut keys = std::collections::HashSet::new();
        for product in &catalog.products {
            assert!(keys.insert(product.key.as_str()), "produit en double : {}", product.key);
            assert!(!product.name.is_empty());
            for cpe in &product.cpes {
                let parts: Vec<&str> = cpe.split(':').collect();
                assert_eq!(parts.len(), 3, "{cpe}");
                assert!(["a", "o", "h"].contains(&parts[0]), "{cpe}");
            }
        }
        for rule in &catalog.metrics {
            assert!(!rule.metric.starts_with("dumbmonit_"), "{} : sans préfixe", rule.metric);
            match &rule.product {
                Some(product) => assert!(catalog.product(product).is_some(), "{product}"),
                None => {
                    assert!(
                        rule.package_label.is_some(),
                        "{} : ni produit ni paquets",
                        rule.metric
                    );
                }
            }
            for product in rule.packages.values() {
                assert!(catalog.product(product).is_some(), "{product}");
            }
            if let Some(pattern) = &rule.pattern {
                assert!(Regex::new(pattern).is_ok(), "{pattern}");
            }
        }
        for program in &catalog.windows_programs {
            assert!(catalog.product(&program.product).is_some(), "{}", program.product);
        }
    }

    #[test]
    fn les_motifs_extraient_la_version() {
        let catalog = catalog();
        let rule = |metric: &str| catalog.metrics.iter().find(|m| m.metric == metric).unwrap();
        assert_eq!(rule("apache_version_info").extract("Apache/2.4.58 (Unix)").unwrap(), "2.4.58");
        assert_eq!(rule("nginx_version_info").extract("nginx/1.24.0").unwrap(), "1.24.0");
        assert_eq!(rule("nginx_version_info").extract("nginx"), None);
        assert_eq!(rule("k8s_version_info").extract("v1.33.4+k3s1").unwrap(), "1.33.4");
        assert_eq!(rule("mikrotik_info").extract("7.16.1 (stable)").unwrap(), "7.16.1");
        assert_eq!(
            rule("synology_system_info").extract("DSM 7.2.1-69057 Update 5").unwrap(),
            "7.2.1-69057"
        );
        assert_eq!(rule("adguard_version_info").extract("v0.107.79").unwrap(), "0.107.79");
    }

    #[test]
    fn les_programmes_windows_se_reconnaissent_au_nom() {
        let catalog = catalog();
        let find = |name: &str| {
            catalog.windows_programs.iter().find(|p| p.matches(name)).map(|p| p.product.as_str())
        };
        assert_eq!(find("Mozilla Firefox (x64 fr)"), Some("firefox"));
        assert_eq!(find("Mozilla Firefox ESR (x64 en-US)"), Some("firefox-esr"));
        assert_eq!(find("Microsoft Edge WebView2 Runtime"), None);
        assert_eq!(find("7-Zip 24.08 (x64)"), Some("7-zip"));
        assert_eq!(find("Notepad++ (64-bit x64)"), Some("notepad-plus-plus"));
        assert_eq!(find("Visual Studio Code"), None);
    }
}
