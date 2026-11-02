//! File « à examiner » : ce que les balayages ont trouvé et que personne n'a
//! encore traité, et ce qu'on propose d'en faire.
//!
//! Un appareil de l'inventaire est dans l'un de ces états ([`Review`]) :
//!
//! - **à examiner** : nouveau (inconnu), ni surveillé ni ignoré ;
//! - **surveillé** : une cible existe déjà à son adresse ;
//! - **ignoré** : l'administrateur n'en veut pas, il ne revient jamais ;
//! - **connu** : dans l'inventaire de départ ou déjà validé.
//!
//! La logique est pure (pas de base, pas de réseau) pour rester testable ; la
//! route d'ajout en bloc (`api/netinventory.rs`) s'en sert.

use std::collections::{BTreeMap, HashSet};

use serde::Serialize;

use super::store::DeviceView;

/// Plafond d'appareils par appel en bloc.
pub const MAX_BULK: usize = 200;

/// Étiquette posée sur un moniteur ajouté sans les identifiants dont son type a
/// besoin : il est créé en pause, et cette étiquette le retrouve.
pub const PENDING_TAG: &str = "pending";
pub const PENDING_CREDENTIALS: &str = "credentials";

/// Où en est un appareil dans la file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Review {
    ToReview,
    Monitored,
    Ignored,
    Known,
}

/// Classe un appareil. `monitored` : adresses (IP ou noms d'hôte, en minuscules)
/// des cibles existantes.
pub fn classify(device: &DeviceView, monitored: &HashSet<String>) -> Review {
    if device.ignored {
        return Review::Ignored;
    }
    let by_ip = monitored.contains(&device.ip.to_lowercase());
    let by_name =
        device.hostname.as_deref().is_some_and(|h| monitored.contains(&h.to_lowercase()));
    if by_ip || by_name {
        return Review::Monitored;
    }
    if device.known { Review::Known } else { Review::ToReview }
}

/// Hôte d'une adresse de cible (`http://nas.lan:5000/x`, `10.0.0.5:161`,
/// `10.0.0.5`), en minuscules ; vide si rien d'exploitable.
fn host_of(address: &str) -> String {
    let rest = address.trim().split_once("://").map_or(address.trim(), |(_, r)| r);
    let rest = rest.split(['/', '?', '#']).next().unwrap_or("");
    let rest = rest.rsplit_once('@').map_or(rest, |(_, h)| h);
    let host = match rest.strip_prefix('[') {
        Some(v6) => v6.split(']').next().unwrap_or(""),
        None => rest.split(':').next().unwrap_or(""),
    };
    host.to_lowercase()
}

/// Hôtes des cibles existantes, pour [`classify`].
pub fn monitored_hosts<'a>(addresses: impl IntoIterator<Item = &'a str>) -> HashSet<String> {
    addresses.into_iter().map(host_of).filter(|h| !h.is_empty()).collect()
}

/// Nombre d'appareils à examiner.
pub fn to_review_count(devices: &[DeviceView], monitored: &HashSet<String>) -> usize {
    devices.iter().filter(|d| classify(d, monitored) == Review::ToReview).count()
}

/// Ce qu'on crée pour un appareil : type, adresse, nom, étiquettes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal {
    pub kind: String,
    pub name: String,
    pub address: String,
    pub tags: BTreeMap<String, String>,
    pub profile_id: Option<String>,
}

/// Clés qui ressemblent à un secret : jamais reprises du pré-remplissage.
fn secretish(key: &str) -> bool {
    let key = key.to_lowercase();
    ["pass", "secret", "token", "key", "credential", "community"].iter().any(|w| key.contains(w))
}

fn scalar(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Le type proposé pour un appareil : celui du premier port (par numéro) dont la
/// détection a reconnu un type, avec son pré-remplissage. `None` : rien de
/// reconnu, l'ajout en bloc ne devine pas.
pub fn proposal(device: &DeviceView) -> Option<Proposal> {
    let port = device.ports.iter().find(|p| p.kind.as_deref().is_some_and(|k| !k.is_empty()))?;
    let kind = port.kind.clone()?;
    let prefill: serde_json::Value =
        port.prefill.as_deref().and_then(|raw| serde_json::from_str(raw).ok()).unwrap_or_default();

    let text = |key: &str| {
        prefill.get(key).and_then(scalar).map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
    };
    let address = text("address").unwrap_or_else(|| device.ip.clone());
    let name = text("name").unwrap_or_else(|| {
        let label = device.hostname.as_deref().or(device.vendor.as_deref());
        match label {
            Some(label) => format!("{label} ({})", device.ip),
            None => device.ip.clone(),
        }
    });
    let mut tags = BTreeMap::new();
    if let Some(map) = prefill.get("tags").and_then(|v| v.as_object()) {
        for (key, value) in map {
            if secretish(key) {
                continue;
            }
            if let Some(value) = scalar(value) {
                tags.insert(key.clone(), value);
            }
        }
    }
    Some(Proposal { kind, name, address, tags, profile_id: text("profile_id") })
}

/// Issue d'un appareil dans un appel en bloc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// Moniteur créé.
    Added,
    /// Moniteur créé en pause : il lui manque des identifiants.
    PendingCredentials,
    /// Une cible existait déjà à cette adresse.
    AlreadyPresent,
    /// Appareil ignoré.
    Ignored,
    /// Appareil remis dans la file « à examiner ».
    Restored,
    Error,
}

/// Résultat d'un appareil.
#[derive(Debug, Clone, Serialize)]
pub struct ItemResult {
    pub device_id: i64,
    pub outcome: Outcome,
    /// Moniteur créé, ou celui qui existait déjà.
    pub target_id: Option<i64>,
    pub message: Option<String>,
}

impl ItemResult {
    pub fn new(device_id: i64, outcome: Outcome) -> Self {
        Self { device_id, outcome, target_id: None, message: None }
    }

    pub fn with_target(mut self, id: i64) -> Self {
        self.target_id = Some(id);
        self
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::super::store::PortView;
    use super::*;

    fn port(port: u16, kind: Option<&str>, prefill: Option<&str>) -> PortView {
        PortView {
            id: 1,
            port,
            protocol: "tcp".into(),
            product: "svc".into(),
            kind: kind.map(str::to_string),
            prefill: prefill.map(str::to_string),
            known: false,
            first_seen: String::new(),
            last_seen: String::new(),
        }
    }

    fn device(ip: &str, known: bool, ignored: bool, ports: Vec<PortView>) -> DeviceView {
        DeviceView {
            id: 1,
            ident: format!("ip:{ip}"),
            schedule_id: 1,
            ip: ip.into(),
            mac: None,
            hostname: None,
            vendor: None,
            known,
            ignored,
            mac_change_from: None,
            first_seen: String::new(),
            last_seen: String::new(),
            ports,
        }
    }

    #[test]
    fn les_hotes_des_cibles_se_lisent_dans_leur_adresse() {
        let hosts = monitored_hosts([
            "http://NAS.lan:5000/ui",
            "10.0.0.5:161",
            "10.0.0.6",
            "https://user@[fe80::1]:8443/",
            "",
        ]);
        for host in ["nas.lan", "10.0.0.5", "10.0.0.6", "fe80::1"] {
            assert!(hosts.contains(host), "{host}");
        }
        assert_eq!(hosts.len(), 4);
    }

    #[test]
    fn un_appareil_nouveau_est_a_examiner() {
        let monitored = HashSet::new();
        let d = device("10.0.0.5", false, false, vec![]);
        assert_eq!(classify(&d, &monitored), Review::ToReview);
    }

    #[test]
    fn un_appareil_deja_surveille_n_est_pas_a_examiner() {
        let monitored: HashSet<String> = ["10.0.0.5".to_string()].into();
        let d = device("10.0.0.5", false, false, vec![]);
        assert_eq!(classify(&d, &monitored), Review::Monitored);
        // Par nom d'hôte, sans tenir compte de la casse.
        let mut named = device("10.0.0.6", false, false, vec![]);
        named.hostname = Some("NAS.lan".into());
        let monitored: HashSet<String> = ["nas.lan".to_string()].into();
        assert_eq!(classify(&named, &monitored), Review::Monitored);
    }

    #[test]
    fn un_appareil_ignore_ne_revient_pas() {
        let monitored = HashSet::new();
        // Un appareil ignoré compte aussi comme connu côté inventaire.
        assert_eq!(classify(&device("10.0.0.5", true, true, vec![]), &monitored), Review::Ignored);
        // Et même s'il a une cible à son adresse.
        let monitored: HashSet<String> = ["10.0.0.5".to_string()].into();
        assert_eq!(classify(&device("10.0.0.5", true, true, vec![]), &monitored), Review::Ignored);
    }

    #[test]
    fn un_appareil_de_l_inventaire_de_depart_est_connu() {
        let d = device("10.0.0.5", true, false, vec![]);
        assert_eq!(classify(&d, &HashSet::new()), Review::Known);
    }

    #[test]
    fn le_compteur_ne_prend_que_les_appareils_a_examiner() {
        let monitored: HashSet<String> = ["10.0.0.2".to_string()].into();
        let devices = [
            device("10.0.0.1", false, false, vec![]),
            device("10.0.0.2", false, false, vec![]),
            device("10.0.0.3", true, true, vec![]),
            device("10.0.0.4", true, false, vec![]),
            device("10.0.0.5", false, false, vec![]),
        ];
        assert_eq!(to_review_count(&devices, &monitored), 2);
    }

    #[test]
    fn la_proposition_reprend_le_type_et_le_pre_remplissage_sans_secret() {
        let prefill = r#"{"name":"Proxmox 1","address":"10.0.0.5","profile_id":"pve",
            "tags":{"port":8006,"insecure_tls":true,"api_token":"x"}}"#;
        let d = device(
            "10.0.0.5",
            false,
            false,
            vec![port(22, None, None), port(8006, Some("proxmox"), Some(prefill))],
        );
        let p = proposal(&d).expect("type reconnu");
        assert_eq!(p.kind, "proxmox");
        assert_eq!(p.name, "Proxmox 1");
        assert_eq!(p.address, "10.0.0.5");
        assert_eq!(p.profile_id.as_deref(), Some("pve"));
        assert_eq!(p.tags.get("port").map(String::as_str), Some("8006"));
        assert_eq!(p.tags.get("insecure_tls").map(String::as_str), Some("true"));
        assert!(!p.tags.contains_key("api_token"));
    }

    #[test]
    fn sans_pre_remplissage_l_adresse_et_le_nom_viennent_de_l_appareil() {
        let mut d = device("10.0.0.7", false, false, vec![port(80, Some("http"), None)]);
        d.hostname = Some("cam".into());
        let p = proposal(&d).unwrap();
        assert_eq!((p.address.as_str(), p.name.as_str()), ("10.0.0.7", "cam (10.0.0.7)"));
    }

    #[test]
    fn sans_type_reconnu_rien_n_est_propose() {
        let ports = vec![port(22, None, None), port(80, Some(""), None)];
        let d = device("10.0.0.8", false, false, ports);
        assert!(proposal(&d).is_none());
    }
}
