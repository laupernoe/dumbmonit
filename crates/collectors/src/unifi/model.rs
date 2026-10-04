//! Les deux API de UniFi Network, ramenées à un seul modèle.
//!
//! * L'**API d'intégration** officielle (`/integration/v1`, clé `X-API-KEY`)
//!   décrit les équipements par un état en toutes lettres (`ONLINE`,
//!   `PENDING_ADOPTION`…) et publie leurs statistiques une par une.
//! * L'**API classique** (`/api/s/<site>/stat/*`, session d'un administrateur)
//!   les décrit par un code numérique, et y ajoute ce que l'API d'intégration
//!   ne dit pas : l'état du WAN et d'Internet, les clients par type.
//!
//! Les deux se traduisent en [`Device`] et [`Health`], si bien que la suite ne
//! sait pas laquelle a répondu.

use serde::Deserialize;
use serde_json::Value;

/// État d'un équipement, commun aux deux API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceState {
    Online,
    Offline,
    /// Vu sur le réseau, pas encore adopté.
    Pending,
    /// Mise à jour ou provisionnement en cours : c'est passager.
    Updating,
    Adopting,
    AdoptionFailed,
    /// Point d'accès maillé qui a perdu son lien montant.
    Isolated,
    Other,
}

impl DeviceState {
    pub const ALL: [Self; 8] = [
        Self::Online,
        Self::Offline,
        Self::Pending,
        Self::Updating,
        Self::Adopting,
        Self::AdoptionFailed,
        Self::Isolated,
        Self::Other,
    ];

    pub fn word(self) -> &'static str {
        match self {
            Self::Online => "online",
            Self::Offline => "offline",
            Self::Pending => "pending",
            Self::Updating => "updating",
            Self::Adopting => "adopting",
            Self::AdoptionFailed => "adoption_failed",
            Self::Isolated => "isolated",
            Self::Other => "other",
        }
    }

    /// Code publié dans `unifi_device_state`.
    pub fn code(self) -> f64 {
        Self::ALL.iter().position(|s| *s == self).unwrap_or(7) as f64
    }

    /// Codes de l'API classique (`stat/device`, champ `state`).
    pub fn from_classic(code: i64) -> Self {
        match code {
            1 => Self::Online,
            0 | 6 => Self::Offline,
            2 => Self::Pending,
            4 | 5 => Self::Updating,
            7 => Self::Adopting,
            9 | 10 => Self::AdoptionFailed,
            11 => Self::Isolated,
            _ => Self::Other,
        }
    }

    /// États de l'API d'intégration.
    pub fn from_integration(state: &str) -> Self {
        match state {
            "ONLINE" => Self::Online,
            "OFFLINE" | "CONNECTION_INTERRUPTED" => Self::Offline,
            "PENDING_ADOPTION" => Self::Pending,
            "UPDATING" | "GETTING_READY" => Self::Updating,
            "ADOPTING" => Self::Adopting,
            "ADOPTION_FAILED" => Self::AdoptionFailed,
            "ISOLATED" => Self::Isolated,
            _ => Self::Other,
        }
    }

    /// Vrai pour un équipement adopté qui devrait répondre et ne répond pas.
    pub fn is_down(self) -> bool {
        matches!(self, Self::Offline | Self::Isolated | Self::AdoptionFailed)
    }
}

/// Nature d'un équipement, pour trier et pour les règles (processeur d'une
/// passerelle).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    Gateway,
    Switch,
    AccessPoint,
    Other,
}

impl DeviceType {
    pub fn word(self) -> &'static str {
        match self {
            Self::Gateway => "gateway",
            Self::Switch => "switch",
            Self::AccessPoint => "access_point",
            Self::Other => "other",
        }
    }

    fn from_classic(kind: &str) -> Self {
        match kind {
            "ugw" | "udm" | "uxg" => Self::Gateway,
            "usw" => Self::Switch,
            "uap" => Self::AccessPoint,
            _ => Self::Other,
        }
    }

    /// L'API d'intégration ne dit pas « passerelle » : le modèle le dit.
    fn from_integration(model: &str, features: &[String]) -> Self {
        let model = model.to_ascii_uppercase().replace(' ', "");
        const GATEWAYS: [&str; 8] = ["UDM", "UDR", "UCG", "UXG", "USG", "EFG", "UDW", "UX"];
        if GATEWAYS.iter().any(|prefix| model.starts_with(prefix)) {
            return Self::Gateway;
        }
        if features.iter().any(|f| f == "accessPoint") {
            return Self::AccessPoint;
        }
        if features.iter().any(|f| f == "switching") {
            return Self::Switch;
        }
        Self::Other
    }
}

#[derive(Debug, Clone)]
pub struct Device {
    pub name: String,
    pub mac: String,
    pub model: String,
    pub kind: DeviceType,
    pub state: DeviceState,
    pub firmware: Option<String>,
    pub upgradable: Option<bool>,
    pub uptime_seconds: Option<f64>,
    pub cpu_percent: Option<f64>,
    pub memory_percent: Option<f64>,
    pub clients: Option<f64>,
    /// Liens WAN d'une passerelle (`wan1`, `wan2`) et s'ils sont montés.
    pub wan_links: Vec<(String, bool)>,
}

/// Enveloppe de l'API classique : `{"meta":{"rc":"ok"},"data":[…]}`.
#[derive(Debug, Deserialize)]
pub struct Classic<T> {
    pub meta: Meta,
    #[serde(default = "Vec::new")]
    pub data: Vec<T>,
}

#[derive(Debug, Deserialize)]
pub struct Meta {
    pub rc: String,
    #[serde(default)]
    pub msg: Option<String>,
}

/// Un nombre que l'API classique écrit tantôt en nombre, tantôt en texte
/// (`"system-stats": {"cpu": "12.4"}`).
fn number(value: Option<&Value>) -> Option<f64> {
    match value? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn text(value: Option<&Value>) -> Option<String> {
    value.and_then(Value::as_str).map(str::to_string).filter(|s| !s.is_empty())
}

/// Un équipement de `stat/device`.
pub fn classic_device(raw: &Value) -> Option<Device> {
    let mac = text(raw.get("mac"))?;
    let stats = raw.get("system-stats");
    let adopted = raw.get("adopted").and_then(Value::as_bool).unwrap_or(true);
    let mut state =
        DeviceState::from_classic(raw.get("state").and_then(Value::as_i64).unwrap_or(-1));
    if !adopted && state == DeviceState::Offline {
        state = DeviceState::Pending;
    }
    let wan_links = ["wan1", "wan2", "wan3"]
        .iter()
        .filter_map(|key| {
            let link = raw.get(*key)?;
            // Un port WAN désactivé n'est pas une panne.
            if link.get("enable").and_then(Value::as_bool) == Some(false) {
                return None;
            }
            Some((key.to_string(), link.get("up").and_then(Value::as_bool)?))
        })
        .collect();
    Some(Device {
        name: text(raw.get("name")).unwrap_or_else(|| mac.clone()),
        model: text(raw.get("model")).unwrap_or_default(),
        kind: DeviceType::from_classic(raw.get("type").and_then(Value::as_str).unwrap_or("")),
        state,
        firmware: text(raw.get("version")),
        upgradable: raw.get("upgradable").and_then(Value::as_bool),
        uptime_seconds: number(raw.get("uptime")).filter(|_| state == DeviceState::Online),
        cpu_percent: number(stats.and_then(|s| s.get("cpu"))),
        memory_percent: number(stats.and_then(|s| s.get("mem"))),
        clients: number(raw.get("num_sta")),
        wan_links,
        mac,
    })
}

/// Un client connu du site, lu par `/rest/user` (API classique) : la liste
/// que le contrôleur garde de tout appareil qui s'est un jour connecté, pas
/// seulement de ceux qui sont en ligne maintenant — `stat/sta` ne donne que
/// ces derniers. Voir le commentaire de module sur [`ClientWatch`] : la
/// plupart ne sont jamais nommés ni annotés, et ne nourrissent pas le tableau
/// des appareils par défaut.
#[derive(Debug, Clone)]
pub struct ClientRecord {
    pub mac: String,
    /// Alias que l'utilisateur a donné au client dans UniFi : un signe fort
    /// qu'il lui importe.
    pub name: Option<String>,
    /// Nom annoncé par l'appareil lui-même (DHCP) : moins fiable, jamais
    /// choisi par l'utilisateur.
    pub hostname: Option<String>,
    /// Coché dans l'interface (« Notes ») : l'utilisateur l'a remarqué.
    pub noted: bool,
    /// Réservation DHCP en IP fixe : encore un geste délibéré.
    pub use_fixedip: bool,
    pub is_wired: Option<bool>,
    /// Dernière connexion, en secondes Unix ; déjà au format epoch dans
    /// `/rest/user`, à la différence des horodatages ISO 8601 d'autres API
    /// UniFi.
    pub last_seen: Option<i64>,
}

impl ClientRecord {
    /// Un client que l'utilisateur a marqué d'une façon ou d'une autre :
    /// nommé, réservé en IP fixe, ou annoté. Voir le commentaire de module de
    /// `collectors/unifi/mod.rs` sur le choix de ne pas suivre, par défaut,
    /// les centaines de clients transitoires d'un réseau domestique.
    pub fn cared_about(&self) -> bool {
        self.noted || self.use_fixedip || self.name.is_some()
    }

    /// Le plus parlant : l'alias choisi, sinon le nom annoncé par l'appareil,
    /// sinon son adresse MAC.
    pub fn display_name(&self) -> String {
        self.name.clone().or_else(|| self.hostname.clone()).unwrap_or_else(|| self.mac.clone())
    }
}

/// Une entrée de `/rest/user`.
pub fn classic_client(raw: &Value) -> Option<ClientRecord> {
    let mac = text(raw.get("mac"))?;
    Some(ClientRecord {
        mac,
        name: text(raw.get("name")),
        hostname: text(raw.get("hostname")),
        noted: raw.get("noted").and_then(Value::as_bool).unwrap_or(false),
        use_fixedip: raw.get("use_fixedip").and_then(Value::as_bool).unwrap_or(false),
        is_wired: raw.get("is_wired").and_then(Value::as_bool),
        last_seen: number(raw.get("last_seen")).map(|v| v as i64),
    })
}

/// Pagination de l'API d'intégration.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Paged<T> {
    #[serde(default)]
    pub offset: u64,
    #[serde(default)]
    pub count: u64,
    #[serde(default)]
    pub total_count: u64,
    #[serde(default = "Vec::new")]
    pub data: Vec<T>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    pub application_version: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Site {
    pub id: String,
    #[serde(default)]
    pub internal_reference: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationDevice {
    pub id: String,
    #[serde(default)]
    pub mac_address: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    pub state: String,
    #[serde(default)]
    pub firmware_version: Option<String>,
    #[serde(default)]
    pub firmware_updatable: Option<bool>,
    #[serde(default)]
    pub features: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Statistics {
    #[serde(default)]
    pub uptime_sec: Option<f64>,
    #[serde(default)]
    pub cpu_utilization_pct: Option<f64>,
    #[serde(default)]
    pub memory_utilization_pct: Option<f64>,
}

impl IntegrationDevice {
    pub fn into_device(self, stats: Option<Statistics>) -> Device {
        let mac = self.mac_address.unwrap_or_else(|| self.id.clone());
        let model = self.model.unwrap_or_default();
        let stats = stats.unwrap_or_default();
        Device {
            name: self.name.filter(|n| !n.is_empty()).unwrap_or_else(|| mac.clone()),
            kind: DeviceType::from_integration(&model, &self.features),
            state: DeviceState::from_integration(&self.state),
            firmware: self.firmware_version,
            upgradable: self.firmware_updatable,
            uptime_seconds: stats.uptime_sec,
            cpu_percent: stats.cpu_utilization_pct,
            memory_percent: stats.memory_utilization_pct,
            clients: None,
            wan_links: Vec::new(),
            model,
            mac,
        }
    }
}

/// `stat/health` : un état par sous-système et les nombres de clients.
#[derive(Debug, Clone, Default)]
pub struct Health {
    /// `(sous-système, état)`, état = `ok`, `warning`, `error` ou `unknown`.
    pub subsystems: Vec<(String, String)>,
    /// Passerelle présente : sans elle, WAN et Internet ne disent rien.
    pub has_gateway: bool,
    pub wan_ok: Option<bool>,
    pub internet_ok: Option<bool>,
    pub latency_ms: Option<f64>,
    /// Disponibilité sur 24 h de chaque WAN (`WAN`, `WAN2`), en pour cent.
    pub wan_availability: Vec<(String, f64)>,
    pub clients_wireless: Option<f64>,
    pub clients_wired: Option<f64>,
    pub clients_guest: Option<f64>,
    pub clients_vpn: Option<f64>,
}

pub fn health(entries: &[Value]) -> Health {
    let mut out = Health::default();
    let find = |name: &str| {
        entries.iter().find(|e| e.get("subsystem").and_then(Value::as_str) == Some(name))
    };
    for entry in entries {
        if let (Some(name), Some(status)) =
            (text(entry.get("subsystem")), text(entry.get("status")))
        {
            out.subsystems.push((name, status));
        }
    }
    let ok = |entry: Option<&Value>| {
        entry.and_then(|e| e.get("status")).and_then(Value::as_str).and_then(
            |status| match status {
                "ok" => Some(true),
                "error" => Some(false),
                _ => None,
            },
        )
    };
    let wan = find("wan");
    out.has_gateway = number(wan.and_then(|w| w.get("num_gw"))).unwrap_or(0.0) > 0.0;
    if out.has_gateway {
        out.wan_ok = ok(wan);
        out.internet_ok = ok(find("www"));
        out.latency_ms = number(find("www").and_then(|w| w.get("latency")));
        if let Some(Value::Object(stats)) = wan.and_then(|w| w.get("uptime_stats")) {
            for (name, stat) in stats {
                if let Some(availability) = number(stat.get("availability")) {
                    out.wan_availability.push((name.clone(), availability));
                }
            }
        }
    }
    let sum = |entry: Option<&Value>, keys: &[&str]| -> Option<f64> {
        let entry = entry?;
        let values: Vec<f64> = keys.iter().filter_map(|k| number(entry.get(*k))).collect();
        (!values.is_empty()).then(|| values.iter().sum())
    };
    out.clients_wireless = sum(find("wlan"), &["num_user"]);
    out.clients_wired = sum(find("lan"), &["num_user"]);
    out.clients_guest = sum(find("wlan"), &["num_guest"])
        .map(|w| w + sum(find("lan"), &["num_guest"]).unwrap_or(0.0));
    out.clients_vpn = sum(find("vpn"), &["remote_user_num_active"]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEVICES: &str = include_str!("testdata/documented/classic_stat_device.json");
    const HEALTH: &str = include_str!("testdata/documented/classic_stat_health.json");
    const HEALTH_EMPTY: &str = include_str!("testdata/network_10.6.106/stat_health_empty.json");
    const INTEGRATION: &str = include_str!("testdata/documented/integration_devices.json");
    const REST_USER: &str = include_str!("testdata/documented/rest_user.json");

    fn devices() -> Vec<Device> {
        let classic: Classic<Value> = serde_json::from_str(DEVICES).unwrap();
        classic.data.iter().filter_map(classic_device).collect()
    }

    #[test]
    fn les_codes_d_etat_classiques_sont_traduits() {
        let devices = devices();
        let state = |name: &str| devices.iter().find(|d| d.name == name).unwrap().state;
        assert_eq!(state("Gateway"), DeviceState::Online);
        assert_eq!(state("Living room AP"), DeviceState::Offline);
        assert_eq!(state("Garden AP"), DeviceState::Isolated);
        // Sans nom, un équipement en attente prend son adresse MAC.
        assert_eq!(state("74:83:c2:00:00:04"), DeviceState::Pending);
    }

    #[test]
    fn la_passerelle_porte_charge_mise_a_jour_et_liens_wan() {
        let gateway = devices().into_iter().find(|d| d.kind == DeviceType::Gateway).unwrap();
        assert_eq!(gateway.cpu_percent, Some(12.4));
        assert_eq!(gateway.memory_percent, Some(61.8));
        assert_eq!(gateway.upgradable, Some(true));
        assert_eq!(
            gateway.wan_links,
            vec![("wan1".to_string(), true), ("wan2".to_string(), false)]
        );
        let offline = devices().into_iter().find(|d| d.name == "Living room AP").unwrap();
        assert_eq!(offline.uptime_seconds, None);
        assert_eq!(offline.cpu_percent, None);
    }

    #[test]
    fn la_sante_dit_wan_internet_et_clients() {
        let classic: Classic<Value> = serde_json::from_str(HEALTH).unwrap();
        let health = health(&classic.data);
        assert!(health.has_gateway);
        assert_eq!(health.wan_ok, Some(true));
        assert_eq!(health.internet_ok, Some(true));
        assert_eq!(health.latency_ms, Some(13.0));
        assert_eq!(health.clients_wireless, Some(27.0));
        assert_eq!(health.clients_wired, Some(12.0));
        assert_eq!(health.clients_guest, Some(2.0));
        assert_eq!(health.clients_vpn, Some(1.0));
        assert!(health.wan_availability.contains(&("WAN2".to_string(), 0.0)));
    }

    #[test]
    fn un_controleur_sans_passerelle_ne_dit_rien_du_wan() {
        // Réponse réelle d'un UniFi Network 10.6.106 sans équipement adopté.
        let classic: Classic<Value> = serde_json::from_str(HEALTH_EMPTY).unwrap();
        assert_eq!(classic.meta.rc, "ok");
        let health = health(&classic.data);
        assert!(!health.has_gateway);
        assert_eq!(health.wan_ok, None);
        assert_eq!(health.internet_ok, None);
        assert_eq!(health.subsystems.len(), 5);
        assert!(health.subsystems.iter().all(|(_, status)| status == "unknown"));
    }

    #[test]
    fn l_api_d_integration_se_traduit_dans_le_meme_modele() {
        let paged: Paged<IntegrationDevice> = serde_json::from_str(INTEGRATION).unwrap();
        assert_eq!(paged.total_count, 4);
        let devices: Vec<Device> = paged.data.into_iter().map(|d| d.into_device(None)).collect();
        assert_eq!(devices[0].kind, DeviceType::Gateway);
        assert_eq!(devices[0].upgradable, Some(true));
        assert_eq!(devices[1].kind, DeviceType::Switch);
        assert_eq!(devices[2].kind, DeviceType::AccessPoint);
        assert_eq!(devices[2].state, DeviceState::Offline);
        assert_eq!(devices[3].state, DeviceState::Offline, "CONNECTION_INTERRUPTED");
        let stats: Statistics =
            serde_json::from_str(include_str!("testdata/documented/integration_statistics.json"))
                .unwrap();
        assert_eq!(stats.cpu_utilization_pct, Some(12.4));
        assert_eq!(stats.uptime_sec, Some(1_728_035.0));
    }

    #[test]
    fn un_client_connu_dit_s_il_est_remarque() {
        let classic: Classic<Value> = serde_json::from_str(REST_USER).unwrap();
        let clients: Vec<ClientRecord> = classic.data.iter().filter_map(classic_client).collect();
        assert_eq!(clients.len(), 4);
        let desktop = clients.iter().find(|c| c.mac == "aa:bb:cc:00:00:01").unwrap();
        assert!(desktop.cared_about(), "nommé, en IP fixe et annoté");
        assert_eq!(desktop.display_name(), "Noe's desktop");
        assert_eq!(desktop.is_wired, Some(true));
        assert_eq!(desktop.last_seen, Some(1_790_798_700));

        let printer = clients.iter().find(|c| c.mac == "aa:bb:cc:00:00:03").unwrap();
        assert!(printer.cared_about(), "nommé suffit, sans IP fixe ni note");

        let phone = clients.iter().find(|c| c.mac == "aa:bb:cc:00:00:02").unwrap();
        assert!(!phone.cared_about(), "seulement un nom d'hôte annoncé par l'appareil");
        assert_eq!(phone.display_name(), "iPhone-de-Bob", "retombe sur le nom d'hôte");

        let guest = clients.iter().find(|c| c.mac == "aa:bb:cc:00:00:04").unwrap();
        assert!(!guest.cared_about());
        assert_eq!(guest.display_name(), "android-xyz123");
    }

    #[test]
    fn les_codes_publies_restent_stables() {
        assert_eq!(DeviceState::Online.code(), 0.0);
        assert_eq!(DeviceState::Offline.code(), 1.0);
        assert_eq!(DeviceState::Pending.code(), 2.0);
        assert_eq!(DeviceState::Isolated.code(), 6.0);
        assert!(DeviceState::Isolated.is_down());
        assert!(!DeviceState::Pending.is_down());
        assert!(!DeviceState::Updating.is_down());
    }
}
