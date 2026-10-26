//! Serveurs de messagerie Windows : MDaemon Email Server et SecurityGateway.
//!
//! Ce que seule une lecture locale peut dire : l'état des services Windows, la
//! version installée, le nombre de messages dans les dossiers de files et l'âge
//! du plus ancien, l'espace libre du volume qui porte les données. Les compteurs
//! de performance de MDaemon (files, sessions…) restent lus par
//! `perf_counters.rs` ; rien n'est dupliqué ici, les séries de ce module portent
//! le préfixe `agent_mail_`.
//!
//! # Détection
//!
//! Aucun réglage : une requête WMI filtrée sur les noms de services
//! (`MDaemon*`, `SecurityGateway*`) dit si un produit est présent, et le chemin
//! de l'exécutable du service donne le dossier d'installation. Tant que rien
//! n'est trouvé, la requête n'est refaite qu'un cycle sur dix. Le réglage
//! `mdaemon: false` de l'agent coupe aussi ce module.
//!
//! # Séries
//!
//! | Série | Étiquettes |
//! |---|---|
//! | `agent_mail_info` (1) | `product`, `version` (absente si inconnue) |
//! | `agent_mail_service_up` | `product`, `service`, `display`, `start` |
//! | `agent_mail_queue_files` | `product`, `queue` |
//! | `agent_mail_queue_oldest_seconds` | `product`, `queue` |
//! | `agent_mail_data_free_bytes` / `_total_bytes` | `product` |
//!
//! La logique (analyse de l'inventaire, chemins, comptage) est séparée de la
//! source de la plateforme pour être testée sous Linux ; seul le lancement de
//! PowerShell est propre à Windows.

use std::collections::BTreeMap;
use std::time::{Duration, SystemTime};

use dumbmonit_proto::{MetricKind, Sample};
use tracing::info;

use crate::collect::perf_counters::PresetMode;
use crate::collect::services::ServiceState;

/// Tant que rien n'est trouvé, la détection n'a lieu qu'un cycle sur dix.
const DETECT_EVERY: u64 = 10;

/// La version est relue tous les soixante cycles, ou si l'exécutable change.
const VERSION_REFRESH_EVERY: u64 = 60;

/// Plafond d'entrées examinées par dossier de file.
const MAX_QUEUE_ENTRIES: usize = 20_000;

/// Plafond de dossiers de file pour SecurityGateway, dont la liste n'est pas
/// connue d'avance.
const MAX_GENERIC_QUEUES: usize = 20;

/// Délai de PowerShell : une requête WMI filtrée répond en moins d'une seconde.
#[cfg(windows)]
const POWERSHELL_TIMEOUT: Duration = Duration::from_secs(20);

/// Les files de MDaemon, sous `<installation>\Queues\`.
const MDAEMON_QUEUES: &[&str] =
    &["Inbound", "Local", "Remote", "Retry", "Bad", "Holding", "LAN", "Quarantine", "Raw"];

/// Script de l'inventaire : une ligne `svc<TAB>nom<TAB>état<TAB>démarrage<TAB>
/// libellé<TAB>chemin` par service. Sans guillemets doubles, pour traverser sans
/// ambiguïté la ligne de commande de Windows.
const INVENTORY_SCRIPT: &str = "[Console]::OutputEncoding=[Text.Encoding]::UTF8; \
Get-CimInstance Win32_Service -Filter 'Name LIKE ''MDaemon%'' OR Name LIKE ''SecurityGateway%'' \
OR DisplayName LIKE ''%MDaemon%'' OR DisplayName LIKE ''%SecurityGateway%''' | \
ForEach-Object { ('svc',$_.Name,$_.State,$_.StartMode,$_.DisplayName,$_.PathName) -join [char]9 }";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Product {
    MDaemon,
    SecurityGateway,
}

impl Product {
    /// Valeur de l'étiquette `product`.
    pub fn label(self) -> &'static str {
        match self {
            Self::MDaemon => "mdaemon",
            Self::SecurityGateway => "securitygateway",
        }
    }

    fn exe_name(self) -> &'static str {
        match self {
            Self::MDaemon => "mdaemon.exe",
            Self::SecurityGateway => "securitygateway.exe",
        }
    }
}

/// Une ligne de l'inventaire, telle que WMI la donne.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawService {
    pub name: String,
    pub state: String,
    pub start_mode: String,
    pub display: String,
    pub path_name: String,
}

/// Un service Windows d'un produit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MailService {
    pub name: String,
    pub display: String,
    pub state: ServiceState,
    /// `auto`, `manual`, `disabled` ou `other`.
    pub start: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueStat {
    pub queue: String,
    pub files: u64,
    pub oldest_age_secs: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataVolume {
    pub free_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductReport {
    pub product: Product,
    pub version: Option<String>,
    pub services: Vec<MailService>,
    pub queues: Vec<QueueStat>,
    pub data: Option<DataVolume>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MailServerReport {
    pub products: Vec<ProductReport>,
}

/// Un volume du système : point de montage, taille, espace libre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Volume {
    pub mount: String,
    pub total: u64,
    pub free: u64,
}

// ------------------------------------------------------------ logique pure

/// Analyse la sortie du script d'inventaire. Les lignes vides, tronquées ou
/// étrangères sont ignorées.
pub fn parse_inventory(text: &str) -> Vec<RawService> {
    text.lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('\t').collect();
            if fields.len() < 5 || fields[0] != "svc" || fields[1].trim().is_empty() {
                return None;
            }
            Some(RawService {
                name: fields[1].trim().to_string(),
                state: fields[2].trim().to_string(),
                start_mode: fields[3].trim().to_string(),
                display: fields[4].trim().to_string(),
                path_name: fields.get(5).map(|p| p.trim().to_string()).unwrap_or_default(),
            })
        })
        .collect()
}

/// Le produit que nomme un champ, le premier cité s'il en nomme deux.
fn field_product(field: &str) -> Option<Product> {
    let lower = field.to_lowercase();
    let md = lower.find("mdaemon");
    let sg = lower.find("securitygateway");
    match (md, sg) {
        (Some(a), Some(b)) => Some(if a < b { Product::MDaemon } else { Product::SecurityGateway }),
        (Some(_), None) => Some(Product::MDaemon),
        (None, Some(_)) => Some(Product::SecurityGateway),
        (None, None) => None,
    }
}

/// Le produit d'un service : le chemin de l'exécutable d'abord, puis le nom,
/// puis le libellé.
pub fn classify(service: &RawService) -> Option<Product> {
    field_product(&service.path_name)
        .or_else(|| field_product(&service.name))
        .or_else(|| field_product(&service.display))
}

/// Chemin de l'exécutable dans le `PathName` d'un service : entre guillemets,
/// ou jusqu'à `.exe`.
pub fn exe_from_path_name(path_name: &str) -> Option<String> {
    let trimmed = path_name.trim();
    if let Some(rest) = trimmed.strip_prefix('"') {
        let end = rest.find('"')?;
        let exe = rest[..end].trim();
        return if exe.is_empty() { None } else { Some(exe.to_string()) };
    }
    let lower = trimmed.to_lowercase();
    let end = lower.find(".exe")? + ".exe".len();
    Some(trimmed[..end].to_string())
}

fn separator(path: &str) -> char {
    if path.contains('\\') { '\\' } else { '/' }
}

fn last_component(path: &str) -> &str {
    path.rsplit(['\\', '/']).next().unwrap_or(path)
}

/// Dossier d'installation : celui de l'exécutable, ou son parent quand
/// l'exécutable est dans un sous-dossier `App` (MDaemon).
pub fn install_root(exe: &str) -> Option<String> {
    let sep = separator(exe);
    let mut parts: Vec<&str> = exe.split(['\\', '/']).collect();
    parts.pop()?;
    if parts.last().is_some_and(|last| last.eq_ignore_ascii_case("app")) {
        parts.pop();
    }
    if parts.iter().all(|part| part.is_empty()) {
        return None;
    }
    Some(parts.join(&sep.to_string()))
}

fn service_state(raw: &str) -> ServiceState {
    match raw.trim().to_lowercase().as_str() {
        "running" => ServiceState::Running,
        "stopped" => ServiceState::Stopped,
        _ => ServiceState::Unknown,
    }
}

fn start_mode(raw: &str) -> &'static str {
    match raw.trim().to_lowercase().as_str() {
        "auto" => "auto",
        "manual" => "manual",
        "disabled" => "disabled",
        _ => "other",
    }
}

/// Ce que l'inventaire a trouvé pour un produit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detected {
    pub product: Product,
    pub services: Vec<MailService>,
    pub exe: Option<String>,
    pub root: Option<String>,
}

/// Range les services par produit et retrouve l'exécutable et le dossier.
pub fn detect(raw: &[RawService]) -> Vec<Detected> {
    let mut by_product: BTreeMap<Product, Vec<&RawService>> = BTreeMap::new();
    for service in raw {
        if let Some(product) = classify(service) {
            by_product.entry(product).or_default().push(service);
        }
    }
    by_product
        .into_iter()
        .map(|(product, list)| {
            let exes: Vec<String> =
                list.iter().filter_map(|s| exe_from_path_name(&s.path_name)).collect();
            let exe = exes
                .iter()
                .find(|exe| last_component(exe).eq_ignore_ascii_case(product.exe_name()))
                .or_else(|| exes.first())
                .cloned();
            let root = exe.as_deref().and_then(install_root);
            let mut services: Vec<MailService> = list
                .iter()
                .map(|s| MailService {
                    name: s.name.clone(),
                    display: s.display.clone(),
                    state: service_state(&s.state),
                    start: start_mode(&s.start_mode),
                })
                .collect();
            services.sort_by(|a, b| a.name.cmp(&b.name));
            services.dedup_by(|a, b| a.name == b.name);
            Detected { product, services, exe, root }
        })
        .collect()
}

/// Le volume qui porte un chemin : le point de montage le plus long qui en est
/// un préfixe, sans tenir compte de la casse ni du type de séparateur.
pub fn volume_for<'a>(path: &str, volumes: &'a [Volume]) -> Option<&'a Volume> {
    fn normal(text: &str) -> String {
        text.replace('\\', "/").trim_end_matches('/').to_lowercase()
    }
    let wanted = normal(path);
    volumes
        .iter()
        .filter(|volume| {
            let mount = normal(&volume.mount);
            wanted == mount || wanted.starts_with(&format!("{mount}/"))
        })
        .max_by_key(|volume| normal(&volume.mount).len())
}

/// Script qui lit la version du produit d'un exécutable.
fn version_script(exe: &str) -> String {
    format!("(Get-Item -LiteralPath '{}').VersionInfo.ProductVersion", exe.replace('\'', "''"))
}

/// Étiquette de file sûre : minuscules, chiffres, `_` et `-`.
fn sanitize_queue(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' })
        .collect()
}

/// Compte les `.msg` d'un dossier (sans descendre) et cherche le plus ancien.
/// `None` : dossier illisible ou absent.
fn scan_dir(dir: &str, queue: &str) -> Option<QueueStat> {
    let entries = std::fs::read_dir(dir).ok()?;
    let now = SystemTime::now();
    let mut files = 0u64;
    let mut oldest: Option<SystemTime> = None;
    for entry in entries.take(MAX_QUEUE_ENTRIES).flatten() {
        let path = entry.path();
        let is_msg = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("msg"));
        if !is_msg {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        if !meta.is_file() {
            continue;
        }
        files += 1;
        if let Ok(modified) = meta.modified() {
            oldest = Some(oldest.map_or(modified, |current| current.min(modified)));
        }
    }
    let oldest_age_secs = if files == 0 {
        None
    } else {
        oldest.map(|time| now.duration_since(time).unwrap_or(Duration::ZERO).as_secs())
    };
    Some(QueueStat { queue: queue.to_string(), files, oldest_age_secs })
}

/// Les files d'un produit, lues sur le disque (bloquant).
fn scan_queues(product: Product, root: &str) -> Vec<QueueStat> {
    let sep = separator(root);
    let queues_dir = format!("{root}{sep}Queues");
    match product {
        Product::MDaemon => MDAEMON_QUEUES
            .iter()
            .filter_map(|name| scan_dir(&format!("{queues_dir}{sep}{name}"), &sanitize_queue(name)))
            .collect(),
        Product::SecurityGateway => {
            let Ok(entries) = std::fs::read_dir(&queues_dir) else { return Vec::new() };
            let mut names: Vec<String> = entries
                .flatten()
                .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
                .filter_map(|entry| entry.file_name().into_string().ok())
                .collect();
            names.sort();
            names.truncate(MAX_GENERIC_QUEUES);
            names
                .iter()
                .filter_map(|name| {
                    scan_dir(&format!("{queues_dir}{sep}{name}"), &sanitize_queue(name))
                })
                .collect()
        }
    }
}

/// Traduit le rapport en échantillons.
pub fn samples(report: &MailServerReport, now_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    for product in &report.products {
        let label = product.product.label();
        let mut info = Sample::new("agent_mail_info", 1.0, MetricKind::Gauge, now_ms)
            .with_label("product", label);
        if let Some(version) = &product.version {
            info = info.with_label("version", version);
        }
        out.push(info);
        for service in &product.services {
            out.push(
                Sample::new(
                    "agent_mail_service_up",
                    service.state.as_value(),
                    MetricKind::Gauge,
                    now_ms,
                )
                .with_label("product", label)
                .with_label("service", &service.name)
                .with_label("display", &service.display)
                .with_label("start", service.start),
            );
        }
        for queue in &product.queues {
            out.push(
                Sample::new(
                    "agent_mail_queue_files",
                    queue.files as f64,
                    MetricKind::Gauge,
                    now_ms,
                )
                .with_label("product", label)
                .with_label("queue", &queue.queue),
            );
            if let Some(age) = queue.oldest_age_secs {
                out.push(
                    Sample::new(
                        "agent_mail_queue_oldest_seconds",
                        age as f64,
                        MetricKind::Gauge,
                        now_ms,
                    )
                    .with_label("product", label)
                    .with_label("queue", &queue.queue),
                );
            }
        }
        if let Some(data) = &product.data {
            out.push(
                Sample::new(
                    "agent_mail_data_free_bytes",
                    data.free_bytes as f64,
                    MetricKind::Gauge,
                    now_ms,
                )
                .with_label("product", label),
            );
            out.push(
                Sample::new(
                    "agent_mail_data_total_bytes",
                    data.total_bytes as f64,
                    MetricKind::Gauge,
                    now_ms,
                )
                .with_label("product", label),
            );
        }
    }
    out
}

// ------------------------------------------------------------------- source

/// Ce que la plateforme sait lire.
#[allow(async_fn_in_trait)]
pub trait MailSource {
    /// Sortie du script d'inventaire ; `None` : indisponible.
    async fn inventory(&self) -> Option<String>;
    /// Version du produit d'un exécutable.
    async fn file_version(&self, exe: &str) -> Option<String>;
    fn volumes(&self) -> Vec<Volume>;
}

#[derive(Default)]
pub struct PlatformSource;

/// Lance PowerShell et rend sa sortie standard. Windows seulement.
#[cfg(windows)]
async fn run_powershell(script: &str) -> Option<String> {
    let mut command = tokio::process::Command::new("powershell.exe");
    command
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script])
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true)
        .creation_flags(0x0800_0000);
    let output = tokio::time::timeout(POWERSHELL_TIMEOUT, command.output()).await.ok()?.ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Ailleurs, rien à lancer : le module reste muet.
#[cfg(not(windows))]
async fn run_powershell(_script: &str) -> Option<String> {
    None
}

impl MailSource for PlatformSource {
    async fn inventory(&self) -> Option<String> {
        run_powershell(INVENTORY_SCRIPT).await
    }

    async fn file_version(&self, exe: &str) -> Option<String> {
        let text = run_powershell(&version_script(exe)).await?;
        let version = text.trim();
        if version.is_empty() { None } else { Some(version.to_string()) }
    }

    fn volumes(&self) -> Vec<Volume> {
        sysinfo::Disks::new_with_refreshed_list()
            .list()
            .iter()
            .map(|disk| Volume {
                mount: disk.mount_point().to_string_lossy().into_owned(),
                total: disk.total_space(),
                free: disk.available_space(),
            })
            .collect()
    }
}

// -------------------------------------------------------------------- sonde

pub struct MailServerProbe<S: MailSource> {
    mode: PresetMode,
    source: S,
    cycle: u64,
    found: bool,
    /// Produits annoncés dans le journal.
    announced: Vec<Product>,
    /// Version par exécutable, avec le cycle de lecture.
    versions: BTreeMap<String, (u64, Option<String>)>,
}

/// La sonde de la plateforme.
pub type PlatformProbe = MailServerProbe<PlatformSource>;

impl PlatformProbe {
    pub fn for_platform(mode: PresetMode) -> Self {
        MailServerProbe::new(mode, PlatformSource)
    }
}

impl<S: MailSource> MailServerProbe<S> {
    pub fn new(mode: PresetMode, source: S) -> Self {
        Self {
            mode,
            source,
            cycle: 0,
            found: false,
            announced: Vec::new(),
            versions: BTreeMap::new(),
        }
    }

    /// Un cycle. `None` : désactivé, ou aucun produit sur cette machine.
    pub async fn read(&mut self) -> Option<MailServerReport> {
        if self.mode == PresetMode::Off {
            return None;
        }
        let cycle = self.cycle;
        self.cycle = self.cycle.wrapping_add(1);
        if !self.found && self.mode != PresetMode::On && !cycle.is_multiple_of(DETECT_EVERY) {
            return None;
        }
        let detected = match self.source.inventory().await {
            Some(text) => detect(&parse_inventory(&text)),
            None => Vec::new(),
        };
        self.found = !detected.is_empty();
        self.announce(&detected);
        if detected.is_empty() {
            return None;
        }

        let volumes = self.source.volumes();
        let mut products = Vec::with_capacity(detected.len());
        for found in detected {
            let version = match &found.exe {
                Some(exe) => self.version_of(exe, cycle).await,
                None => None,
            };
            let queues = match found.root.clone() {
                Some(root) => {
                    let product = found.product;
                    tokio::task::spawn_blocking(move || scan_queues(product, &root))
                        .await
                        .unwrap_or_default()
                }
                None => Vec::new(),
            };
            let data =
                found.root.as_deref().and_then(|root| volume_for(root, &volumes)).map(|volume| {
                    DataVolume { free_bytes: volume.free, total_bytes: volume.total }
                });
            products.push(ProductReport {
                product: found.product,
                version,
                services: found.services,
                queues,
                data,
            });
        }
        Some(MailServerReport { products })
    }

    async fn version_of(&mut self, exe: &str, cycle: u64) -> Option<String> {
        if let Some((read_at, version)) = self.versions.get(exe)
            && cycle.saturating_sub(*read_at) < VERSION_REFRESH_EVERY
        {
            return version.clone();
        }
        let version = self.source.file_version(exe).await;
        self.versions.insert(exe.to_string(), (cycle, version.clone()));
        version
    }

    /// Dit une fois qu'un produit apparaît ou disparaît.
    fn announce(&mut self, detected: &[Detected]) {
        let now: Vec<Product> = detected.iter().map(|d| d.product).collect();
        for product in &now {
            if !self.announced.contains(product) {
                info!(product = product.label(), "mail server found, reading services and queues");
            }
        }
        for product in &self.announced {
            if !now.contains(product) {
                info!(product = product.label(), "mail server is gone");
            }
        }
        self.announced = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INVENTORY: &str = "svc\tMDaemon\tRunning\tAuto\tMDaemon Email Server\t\"C:\\MDaemon\\App\\MDaemon.exe\" /service\r\n\
svc\tMDaemonXMPP\tStopped\tManual\tMDaemon XMPP\tC:\\MDaemon\\App\\MDXMPP.exe\r\n\
\r\n\
garbage line\r\n\
svc\tSecurityGateway\tRunning\tAuto\tSecurityGateway for Email Servers\t\"C:\\Program Files (x86)\\Alt-N Technologies\\SecurityGateway\\SecurityGateway.exe\"\r\n\
svc\tshort\tRunning\r\n";

    #[test]
    fn the_inventory_tolerates_noise_and_crlf() {
        let raw = parse_inventory(INVENTORY);
        assert_eq!(raw.len(), 3);
        assert_eq!(raw[0].name, "MDaemon");
        assert_eq!(raw[0].start_mode, "Auto");
        assert!(raw[2].path_name.ends_with("SecurityGateway.exe\""));
    }

    #[test]
    fn products_are_told_apart_and_grouped() {
        let detected = detect(&parse_inventory(INVENTORY));
        assert_eq!(detected.len(), 2);
        let md = &detected[0];
        assert_eq!(md.product, Product::MDaemon);
        assert_eq!(md.services.len(), 2);
        assert_eq!(md.exe.as_deref(), Some("C:\\MDaemon\\App\\MDaemon.exe"));
        assert_eq!(md.root.as_deref(), Some("C:\\MDaemon"));
        let sg = &detected[1];
        assert_eq!(sg.product, Product::SecurityGateway);
        assert_eq!(
            sg.root.as_deref(),
            Some("C:\\Program Files (x86)\\Alt-N Technologies\\SecurityGateway")
        );
        let xmpp = md.services.iter().find(|s| s.name == "MDaemonXMPP").unwrap();
        assert_eq!(xmpp.state, ServiceState::Stopped);
        assert_eq!(xmpp.start, "manual");
    }

    #[test]
    fn a_service_naming_both_products_goes_by_its_path_then_first_mention() {
        let raw = RawService {
            name: "X".into(),
            state: "Running".into(),
            start_mode: "Auto".into(),
            display: "MDaemon connector for SecurityGateway".into(),
            path_name: String::new(),
        };
        assert_eq!(classify(&raw), Some(Product::MDaemon));
        let by_path = RawService { path_name: "D:\\SecurityGateway\\sg.exe".into(), ..raw };
        assert_eq!(classify(&by_path), Some(Product::SecurityGateway));
    }

    #[test]
    fn the_executable_is_found_quoted_or_not() {
        assert_eq!(exe_from_path_name("\"C:\\A B\\x.exe\" -k").as_deref(), Some("C:\\A B\\x.exe"));
        assert_eq!(
            exe_from_path_name("C:\\MDaemon\\App\\MDaemon.exe /service").as_deref(),
            Some("C:\\MDaemon\\App\\MDaemon.exe")
        );
        assert_eq!(exe_from_path_name("rien"), None);
        assert_eq!(exe_from_path_name("\"\""), None);
    }

    #[test]
    fn the_install_root_skips_the_app_folder() {
        assert_eq!(install_root("C:\\MDaemon\\App\\MDaemon.exe").as_deref(), Some("C:\\MDaemon"));
        assert_eq!(install_root("C:\\MDaemon\\app\\MDaemon.exe").as_deref(), Some("C:\\MDaemon"));
        assert_eq!(install_root("D:\\SG\\sg.exe").as_deref(), Some("D:\\SG"));
        assert_eq!(install_root("/srv/mdaemon/App/MDaemon.exe").as_deref(), Some("/srv/mdaemon"));
        assert_eq!(install_root("sg.exe"), None);
    }

    #[test]
    fn the_volume_with_the_longest_mount_wins() {
        let volumes = vec![
            Volume { mount: "C:\\".into(), total: 100, free: 10 },
            Volume { mount: "D:\\".into(), total: 200, free: 20 },
            Volume { mount: "D:\\Mail".into(), total: 300, free: 30 },
        ];
        assert_eq!(volume_for("d:\\mail\\MDaemon", &volumes).unwrap().total, 300);
        assert_eq!(volume_for("D:\\MDaemon", &volumes).unwrap().total, 200);
        assert_eq!(volume_for("C:/MDaemon", &volumes).unwrap().total, 100);
        assert!(volume_for("E:\\x", &volumes).is_none());
        // `D:\Mailbox` n'est pas sous `D:\Mail`.
        assert_eq!(volume_for("D:\\Mailbox", &volumes).unwrap().total, 200);
    }

    #[test]
    fn the_version_script_escapes_quotes_and_has_no_double_quote() {
        let script = version_script("C:\\O'Neil\\x.exe");
        assert!(script.contains("O''Neil"));
        assert!(!INVENTORY_SCRIPT.contains('"'));
    }

    fn temp_root(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("dm-mail-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn queues_count_only_msg_files_and_skip_missing_folders() {
        let root = temp_root("md");
        let queues = root.join("Queues");
        std::fs::create_dir_all(queues.join("Inbound")).unwrap();
        std::fs::create_dir_all(queues.join("Bad")).unwrap();
        std::fs::write(queues.join("Inbound").join("a.msg"), "x").unwrap();
        std::fs::write(queues.join("Inbound").join("B.MSG"), "x").unwrap();
        std::fs::write(queues.join("Inbound").join("c.tmp"), "x").unwrap();
        let stats = scan_queues(Product::MDaemon, root.to_str().unwrap());
        let inbound = stats.iter().find(|q| q.queue == "inbound").unwrap();
        assert_eq!(inbound.files, 2);
        assert!(inbound.oldest_age_secs.is_some());
        let bad = stats.iter().find(|q| q.queue == "bad").unwrap();
        assert_eq!(bad.files, 0);
        assert_eq!(bad.oldest_age_secs, None);
        assert!(stats.iter().all(|q| q.queue != "retry"), "missing folder skipped");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn securitygateway_queues_are_its_subfolders() {
        let root = temp_root("sg");
        std::fs::create_dir_all(root.join("Queues").join("Delivery Queue")).unwrap();
        std::fs::write(root.join("Queues").join("Delivery Queue").join("1.msg"), "x").unwrap();
        std::fs::write(root.join("Queues").join("stray.txt"), "x").unwrap();
        let stats = scan_queues(Product::SecurityGateway, root.to_str().unwrap());
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].queue, "delivery_queue");
        assert_eq!(stats[0].files, 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_report_becomes_labelled_samples() {
        let report = MailServerReport {
            products: vec![ProductReport {
                product: Product::MDaemon,
                version: Some("24.5.1".into()),
                services: vec![MailService {
                    name: "MDaemon".into(),
                    display: "MDaemon Email Server".into(),
                    state: ServiceState::Running,
                    start: "auto",
                }],
                queues: vec![
                    QueueStat { queue: "bad".into(), files: 2, oldest_age_secs: Some(90) },
                    QueueStat { queue: "retry".into(), files: 0, oldest_age_secs: None },
                ],
                data: Some(DataVolume { free_bytes: 5, total_bytes: 10 }),
            }],
        };
        let out = samples(&report, 1_000);
        let find = |metric: &str| out.iter().filter(|s| s.metric == metric).collect::<Vec<_>>();
        let info = find("agent_mail_info")[0];
        assert_eq!(info.labels.get("version").map(String::as_str), Some("24.5.1"));
        assert_eq!(find("agent_mail_service_up")[0].value, 1.0);
        assert_eq!(find("agent_mail_service_up")[0].labels.get("start").unwrap(), "auto");
        assert_eq!(find("agent_mail_queue_files").len(), 2);
        assert_eq!(find("agent_mail_queue_oldest_seconds").len(), 1);
        assert_eq!(find("agent_mail_data_free_bytes")[0].value, 5.0);
        assert_eq!(find("agent_mail_data_total_bytes")[0].value, 10.0);
        assert!(out.iter().all(|s| s.ts_ms == 1_000));
    }

    struct Fake {
        text: Option<String>,
    }

    impl MailSource for Fake {
        async fn inventory(&self) -> Option<String> {
            self.text.clone()
        }
        async fn file_version(&self, _exe: &str) -> Option<String> {
            Some("1.2.3".into())
        }
        fn volumes(&self) -> Vec<Volume> {
            vec![Volume { mount: "C:\\".into(), total: 100, free: 40 }]
        }
    }

    #[tokio::test]
    async fn the_probe_is_silent_when_off_or_when_nothing_is_installed() {
        let mut off = MailServerProbe::new(PresetMode::Off, Fake { text: Some(INVENTORY.into()) });
        assert!(off.read().await.is_none());
        let mut empty = MailServerProbe::new(PresetMode::Auto, Fake { text: None });
        assert!(empty.read().await.is_none());
        // Rien trouvé : la détection ne revient qu'un cycle sur dix.
        for _ in 1..DETECT_EVERY {
            assert!(empty.read().await.is_none());
        }
    }

    #[tokio::test]
    async fn the_probe_reports_services_version_and_volume() {
        let fake = Fake { text: Some(INVENTORY.into()) };
        let mut probe = MailServerProbe::new(PresetMode::Auto, fake);
        let report = probe.read().await.expect("products detected");
        assert_eq!(report.products.len(), 2);
        let md = &report.products[0];
        assert_eq!(md.version.as_deref(), Some("1.2.3"));
        assert_eq!(md.data, Some(DataVolume { free_bytes: 40, total_bytes: 100 }));
        assert!(md.queues.is_empty(), "no such folder on the test machine");
        // Une fois trouvé, chaque cycle relit.
        assert!(probe.read().await.is_some());
    }
}
