//! Synology DSM vu de l'intérieur : ce que l'agent du paquet `.spk` lit sur le
//! NAS lui-même, en plus de ce qu'il lit sur n'importe quelle machine Linux.
//!
//! Le paquet tourne sous un utilisateur dédié, pas en root
//! (`packaging/synology/conf/privilege`). Tout ce qui suit se lit donc sans
//! privilège, et chaque source qui manque se tait plutôt que de publier une
//! série fausse :
//!
//! - **Identité** : `/etc.defaults/VERSION` (version et build de DSM),
//!   `/etc.defaults/synoinfo.conf` ou `/etc/synoinfo.conf` (`upnpmodelname`),
//!   à défaut `/proc/sys/kernel/syno_hw_version`. Le numéro de série n'est
//!   jamais lu.
//! - **Grappes RAID** : `/proc/mdstat`. DSM pose `md0` (système) et `md1`
//!   (échange) sur tous les disques, en RAID 1 à seize emplacements dont la
//!   plupart vides : un `[16/4]` y est normal, seul un membre `(F)` les dit
//!   dégradées. Les autres grappes portent les données (groupes de stockage,
//!   caches SSD) : il leur manque un disque dès que `[n/m]` a `m < n`.
//! - **Volumes** : `/proc/mounts`, les montages `/volumeN`. Un volume que DSM
//!   juge planté est remonté en lecture seule : c'est le signe lisible sans
//!   l'API. Le remplissage, lui, vient des séries `filesystem_*` habituelles.
//! - **Disques** : `/sys/block` (modèle, taille, rotatif ou non) et, quand DSM
//!   les expose, `/run/synostorage/disks/<disque>/{temperature,remain_life,unc}`.
//!   Les disques USB sont écartés.
//! - **Paquets** : `/var/packages/<paquet>/INFO` (nom, version) et le témoin
//!   `/var/packages/<paquet>/enabled` (le paquet doit tourner). L'état réel
//!   vient de `synopkg status`, quand DSM le laisse lancer à cet utilisateur ;
//!   sinon il reste inconnu et aucune alerte « paquet arrêté » ne part.
//! - **Services** : SMB, NFS et SSH, par la présence de `smbd`, `nfsd`, `sshd`
//!   dans `/proc/*/comm`.
//! - **Onduleur** : `upsc`, quand le serveur NUT de DSM tourne (onduleur USB
//!   branché, ou serveur réseau activé). Les séries sont celles du collecteur
//!   NUT (`nut_*`), pour que ses règles et ses lectures s'appliquent telles
//!   quelles.
//!
//! Ce qui exige l'API DSM ou root et n'est donc pas lu ici : le verdict SMART
//! de DSM, les tâches Hyper Backup, les mises à jour de DSM et des paquets, les
//! connexions actives. Le collecteur Synology par identifiants les couvre.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use dumbmonit_proto::{MetricKind, Sample};
use tokio::task::JoinHandle;
use tracing::{debug, info};

/// Période des lectures par commande (onduleur, services).
pub const COMMANDS_INTERVAL: Duration = Duration::from_secs(60);

/// Période de la relecture des paquets : `synopkg status` lance le script de
/// chaque paquet, inutile de le faire à chaque battement.
pub const PACKAGES_INTERVAL: Duration = Duration::from_secs(300);

/// Après un `synopkg` refusé à cet utilisateur, on ne réessaie qu'au bout d'une
/// heure : la réponse ne changera pas d'ici là.
const SYNOPKG_RETRY: Duration = Duration::from_secs(3600);

const COMMAND_TIMEOUT: Duration = Duration::from_secs(15);

/// Attente au premier cycle, pour qu'un `--once` montre quelque chose.
const FIRST_CYCLE_WAIT: Duration = Duration::from_secs(5);

pub const MAX_PACKAGES: usize = 200;
pub const MAX_DISKS: usize = 64;
pub const MAX_ARRAYS: usize = 64;
const MAX_UPS: usize = 4;
const MAX_LABEL: usize = 64;

/// Services que l'on sait reconnaître à leur processus.
const SERVICES: &[(&str, &str)] = &[("smb", "smbd"), ("nfs", "nfsd"), ("ssh", "sshd")];

/// Racine des chemins lus : `/` sur le NAS, un répertoire jetable en test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    root: PathBuf,
}

impl Paths {
    pub fn system() -> Self {
        Self { root: PathBuf::from("/") }
    }

    #[cfg(test)]
    pub fn under(root: &Path) -> Self {
        Self { root: root.to_path_buf() }
    }

    fn at(&self, absolute: &str) -> PathBuf {
        self.root.join(absolute.trim_start_matches('/'))
    }
}

/// Vrai sur un Synology : le fichier de version de DSM et l'arborescence
/// `/usr/syno` n'existent nulle part ailleurs.
pub fn is_dsm(paths: &Paths) -> bool {
    paths.at("/etc.defaults/VERSION").is_file() && paths.at("/usr/syno").is_dir()
}

// ----------------------------------------------------------------- modèle

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DsmSystem {
    /// `DS920+`.
    pub model: Option<String>,
    /// `DSM 7.2.1-69057 Update 5`.
    pub version: Option<String>,
    /// `69057`.
    pub build: Option<String>,
}

/// État d'une grappe, dans l'ordre croissant de gravité.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaidState {
    /// Tous les membres présents ; une vérification (`check`) en cours en fait partie.
    Normal,
    /// Redondante, mais en resynchronisation ou en remodelage.
    Syncing,
    /// Un membre manque ou a échoué, reconstruction comprise : plus de redondance.
    Degraded,
    /// Grappe inactive : ses données sont inaccessibles.
    Failed,
}

impl RaidState {
    /// 0 normal, 1 resynchronisation, 2 dégradée, 3 inactive.
    pub fn value(self) -> f64 {
        match self {
            Self::Normal => 0.0,
            Self::Syncing => 1.0,
            Self::Degraded => 2.0,
            Self::Failed => 3.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RaidSync {
    /// `recovery`, `resync`, `reshape`, `check` ou `repair`.
    pub action: String,
    /// `None` : en attente (`resync=DELAYED`, `PENDING`).
    pub percent: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RaidArray {
    /// `md2`.
    pub name: String,
    /// `raid5`, `raid1`… ; vide pour une grappe inactive.
    pub level: String,
    /// `system` (md0), `swap` (md1) ou `data`.
    pub role: &'static str,
    pub active: bool,
    /// Membres listés, de rechange exclus, en échec compris.
    pub members: Vec<String>,
    /// Membres marqués `(F)`.
    pub failed: Vec<String>,
    /// `[n/m]` : emplacements prévus, membres en service.
    pub slots: Option<(u32, u32)>,
    pub sync: Option<RaidSync>,
    pub state: RaidState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolumeMount {
    /// `/volume1`.
    pub mount: String,
    pub fstype: String,
    pub read_only: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DiskStat {
    /// `sata1`, `sda`, `nvme0n1`.
    pub name: String,
    pub model: Option<String>,
    /// `hdd`, `ssd` ou `nvme`.
    pub kind: &'static str,
    pub size_bytes: Option<u64>,
    pub temperature_celsius: Option<f64>,
    pub remaining_life_percent: Option<f64>,
    pub unc_count: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageRun {
    Running,
    Stopped,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageStat {
    /// Nom du répertoire, identifiant du paquet : `HyperBackup`.
    pub id: String,
    /// Nom affiché : `Hyper Backup`.
    pub name: String,
    pub version: Option<String>,
    /// Témoin `enabled` présent : le paquet doit tourner.
    pub enabled: bool,
    /// Faux pour un paquet sans service (bibliothèque, pilote) : ni marche ni arrêt.
    pub startable: bool,
    pub run: PackageRun,
}

/// Un onduleur lu par `upsc` : son nom et ses variables.
pub type UpsReading = (String, Vec<(String, String)>);

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DsmReport {
    pub system: DsmSystem,
    pub arrays: Vec<RaidArray>,
    pub volumes: Vec<VolumeMount>,
    pub disks: Vec<DiskStat>,
    pub packages: Vec<PackageStat>,
    pub services: Vec<(&'static str, bool)>,
    pub ups: Vec<UpsReading>,
}

// ----------------------------------------------------------------- analyse

/// Fichiers `clé="valeur"` de DSM : `VERSION`, `synoinfo.conf`, `INFO`.
pub fn parse_kv(text: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let value = value.trim();
        let value = value.strip_prefix('"').and_then(|v| v.strip_suffix('"')).unwrap_or(value);
        out.insert(key.trim().to_string(), value.to_string());
    }
    out
}

/// `/etc.defaults/VERSION` → (`DSM 7.2.1-69057 Update 5`, `69057`).
pub fn parse_version(text: &str) -> (Option<String>, Option<String>) {
    let kv = parse_kv(text);
    let get = |key: &str| kv.get(key).map(String::as_str).filter(|v| !v.is_empty());
    let product = get("productversion").map(str::to_string).or_else(|| {
        let major = get("majorversion").or_else(|| get("major"))?;
        let minor = get("minorversion").or_else(|| get("minor")).unwrap_or("0");
        Some(match get("micro") {
            Some(micro) if micro != "0" => format!("{major}.{minor}.{micro}"),
            _ => format!("{major}.{minor}"),
        })
    });
    let build = get("buildnumber").map(str::to_string);
    let update = get("smallfixnumber").and_then(|n| n.parse::<u32>().ok()).filter(|n| *n > 0);
    let version = product.map(|product| {
        let mut version = format!("DSM {product}");
        if let Some(build) = &build {
            version = format!("{version}-{build}");
        }
        if let Some(update) = update {
            version = format!("{version} Update {update}");
        }
        version
    });
    (version, build)
}

/// Le modèle : `upnpmodelname` de `synoinfo.conf`, sinon `syno_hw_version`.
pub fn parse_model(synoinfo: Option<&str>, hw_version: Option<&str>) -> Option<String> {
    synoinfo
        .and_then(|text| parse_kv(text).remove("upnpmodelname"))
        .map(|model| model.trim().to_string())
        .filter(|model| !model.is_empty())
        .or_else(|| hw_version.map(str::trim).filter(|v| !v.is_empty()).map(str::to_string))
}

fn role_of(name: &str) -> &'static str {
    match name {
        "md0" => "system",
        "md1" => "swap",
        _ => "data",
    }
}

/// `/proc/mdstat` : une ligne d'en-tête par grappe (`md2 : active raid5 …`),
/// suivie de lignes de détail (`[4/3] [UUU_]`, progression d'une
/// reconstruction).
pub fn parse_mdstat(text: &str) -> Vec<RaidArray> {
    let mut arrays: Vec<RaidArray> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some((name, rest)) = trimmed.split_once(" : ")
            && name.starts_with("md")
            && !name.contains(char::is_whitespace)
        {
            arrays.push(parse_header(name, rest));
            continue;
        }
        if let Some(array) = arrays.last_mut() {
            parse_detail(array, trimmed);
        }
    }
    for array in &mut arrays {
        array.state = judge(array);
    }
    arrays
}

fn parse_header(name: &str, rest: &str) -> RaidArray {
    let mut tokens = rest.split_whitespace();
    let active = tokens.next() == Some("active");
    let mut level = String::new();
    let mut members = Vec::new();
    let mut failed = Vec::new();
    for token in tokens {
        // `(auto-read-only)`, `(read-only)` : un attribut de la grappe.
        if token.starts_with('(') {
            continue;
        }
        if let Some(open) = token.find('[') {
            let device = token[..open].to_string();
            let flags = &token[open..];
            if flags.contains("(S)") {
                continue;
            }
            if flags.contains("(F)") {
                failed.push(device.clone());
            }
            members.push(device);
        } else if level.is_empty() {
            level = token.to_string();
        }
    }
    RaidArray {
        name: name.to_string(),
        level,
        role: role_of(name),
        active,
        members,
        failed,
        slots: None,
        sync: None,
        state: RaidState::Normal,
    }
}

const SYNC_ACTIONS: &[&str] = &["recovery", "resync", "reshape", "check", "repair"];

fn parse_detail(array: &mut RaidArray, line: &str) {
    for token in line.split_whitespace() {
        if let Some(inner) = token.strip_prefix('[').and_then(|t| t.strip_suffix(']'))
            && let Some((total, working)) = inner.split_once('/')
            && let (Ok(total), Ok(working)) = (total.parse::<u32>(), working.parse::<u32>())
        {
            array.slots = Some((total, working));
        }
    }
    for action in SYNC_ACTIONS {
        let Some(position) = line.find(*action) else { continue };
        let after = line[position + action.len()..].trim_start();
        let Some(value) = after.strip_prefix('=') else { continue };
        let value = value.trim_start();
        let percent = value
            .split_once('%')
            .and_then(|(number, _)| number.trim().parse::<f64>().ok())
            .filter(|p| p.is_finite() && (0.0..=100.0).contains(p));
        array.sync = Some(RaidSync { action: (*action).to_string(), percent });
        return;
    }
}

fn judge(array: &RaidArray) -> RaidState {
    if !array.active {
        return RaidState::Failed;
    }
    // Les grappes système de DSM ont seize emplacements pour quelques disques.
    let missing = array.role == "data" && array.slots.is_some_and(|(total, up)| up < total);
    let recovering = array.sync.as_ref().is_some_and(|sync| sync.action == "recovery");
    if !array.failed.is_empty() || missing || recovering {
        return RaidState::Degraded;
    }
    match array.sync.as_ref().map(|sync| sync.action.as_str()) {
        Some("resync" | "reshape" | "repair") => RaidState::Syncing,
        _ => RaidState::Normal,
    }
}

fn is_volume_mount(mount: &str) -> bool {
    mount
        .strip_prefix("/volume")
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// `/proc/mounts` → les volumes de données (`/volumeN`). Le dernier montage
/// d'un même point l'emporte : c'est lui qui masque les précédents.
pub fn parse_mounts(text: &str) -> Vec<VolumeMount> {
    let mut seen: BTreeMap<String, VolumeMount> = BTreeMap::new();
    for line in text.lines() {
        let mut fields = line.split_whitespace();
        let (Some(_device), Some(mount), Some(fstype), Some(options)) =
            (fields.next(), fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        if !is_volume_mount(mount) {
            continue;
        }
        let read_only = options.split(',').any(|option| option == "ro");
        seen.insert(
            mount.to_string(),
            VolumeMount { mount: mount.to_string(), fstype: fstype.to_string(), read_only },
        );
    }
    seen.into_values().collect()
}

/// Un nombre lu dans un fichier de `/sys` ou de `/run/synostorage`.
pub fn parse_number(text: &str) -> Option<f64> {
    text.trim().parse::<f64>().ok().filter(|v| v.is_finite())
}

/// Un nom de disque interne : `sata1`, `sda`, `nvme0n1`. Ni partition, ni
/// `synoboot`, ni périphérique virtuel.
pub fn is_disk_name(name: &str) -> bool {
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if let Some(rest) = name.strip_prefix("sata") {
        return digits(rest);
    }
    if let Some(rest) = name.strip_prefix("nvme") {
        return rest.split_once('n').is_some_and(|(a, b)| digits(a) && digits(b));
    }
    if let Some(rest) = name.strip_prefix("sd") {
        return !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_lowercase());
    }
    false
}

/// `/var/packages/<paquet>/INFO` → le paquet, témoin `enabled` mis à part.
pub fn parse_info(id: &str, text: &str) -> PackageStat {
    let kv = parse_kv(text);
    let get = |key: &str| kv.get(key).map(|v| v.trim()).filter(|v| !v.is_empty());
    PackageStat {
        id: id.to_string(),
        name: get("displayname").or_else(|| get("package")).unwrap_or(id).to_string(),
        version: get("version").map(str::to_string),
        enabled: false,
        startable: !get("startable").is_some_and(|v| v.eq_ignore_ascii_case("no")),
        run: PackageRun::Unknown,
    }
}

/// Réponse de `synopkg status <paquet>`. DSM 7 écrit un objet JSON
/// (`"status":"running"` ou `"stop"`), d'anciennes versions une phrase
/// (`package X is started`) ; à défaut, le code de sortie suit la convention
/// des scripts `start-stop-status` (0 en marche, 3 arrêté). Un refus
/// (`permission denied`, `must be root`) rend `None`.
pub fn parse_synopkg_status(text: &str, code: Option<i32>) -> Option<PackageRun> {
    let lower = text.to_ascii_lowercase();
    const REFUSALS: &[&str] = &["permission", "denied", "not permitted", "must be root"];
    if REFUSALS.iter().any(|word| lower.contains(*word)) {
        return None;
    }
    let compact: String = lower.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.contains("\"status\":\"running\"") || compact.contains("\"is_running\":true") {
        return Some(PackageRun::Running);
    }
    if compact.contains("\"status\":\"stop") || compact.contains("\"is_running\":false") {
        return Some(PackageRun::Stopped);
    }
    if lower.contains("is not running") || lower.contains("is stopped") {
        return Some(PackageRun::Stopped);
    }
    if lower.contains("is started") || lower.contains("is running") {
        return Some(PackageRun::Running);
    }
    match code {
        Some(0) => Some(PackageRun::Running),
        Some(3) => Some(PackageRun::Stopped),
        _ => None,
    }
}

/// `upsc -l localhost` : un nom d'onduleur par ligne.
pub fn parse_upsc_list(stdout: &str) -> Vec<String> {
    stdout.lines().map(str::trim).filter(|name| is_safe_name(name)).map(str::to_string).collect()
}

/// `upsc <onduleur>@localhost` : des paires `variable: valeur`.
pub fn parse_upsc(stdout: &str) -> Vec<(String, String)> {
    stdout
        .lines()
        .filter_map(|line| line.split_once(": "))
        .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
        .filter(|(key, _)| !key.is_empty() && !key.contains(char::is_whitespace))
        .collect()
}

/// Les services présents parmi les noms de processus (`/proc/*/comm`).
pub fn services_from<'a>(comms: impl IntoIterator<Item = &'a str>) -> Vec<(&'static str, bool)> {
    let mut found = [false; SERVICES.len()];
    for comm in comms {
        let comm = comm.trim();
        for (index, (_, process)) in SERVICES.iter().enumerate() {
            if comm == *process {
                found[index] = true;
            }
        }
    }
    SERVICES.iter().zip(found).map(|((service, _), up)| (*service, up)).collect()
}

/// Nom de paquet ou d'onduleur passé en argument d'une commande : rien qui
/// puisse devenir une option.
fn is_safe_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('-')
        && name.len() <= 64
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

/// 0 normal, 1 à surveiller, 2 critique : la même échelle que
/// `synology_storage_health` du collecteur par API.
pub fn storage_health(report: &DsmReport) -> f64 {
    let mut worst: f64 = 0.0;
    for array in &report.arrays {
        let level = match array.state {
            RaidState::Normal => 0.0,
            RaidState::Syncing => 1.0,
            RaidState::Degraded | RaidState::Failed => 2.0,
        };
        worst = worst.max(level);
    }
    if report.volumes.iter().any(|volume| volume.read_only) {
        worst = 2.0;
    }
    let worn = report.disks.iter().any(|disk| {
        disk.remaining_life_percent.is_some_and(|life| life < 10.0)
            || disk.unc_count.is_some_and(|unc| unc > 0)
    });
    if worn {
        worst = worst.max(1.0);
    }
    worst
}

fn label(value: &str) -> String {
    value.trim().chars().take(MAX_LABEL).collect()
}

fn flag(on: bool) -> f64 {
    f64::from(u8::from(on))
}

/// Traduit le relevé en échantillons.
pub fn samples(report: &DsmReport, now_ms: i64) -> Vec<Sample> {
    let gauge = |metric: &str, value: f64| Sample::new(metric, value, MetricKind::Gauge, now_ms);
    let mut out = Vec::with_capacity(
        8 + report.arrays.len() * 4 + report.disks.len() * 5 + report.packages.len() * 4,
    );

    let mut info = gauge("agent_dsm_info", 1.0);
    if let Some(model) = &report.system.model {
        info = info.with_label("model", label(model));
    }
    if let Some(version) = &report.system.version {
        info = info.with_label("dsm_version", label(version));
    }
    if let Some(build) = &report.system.build {
        info = info.with_label("build", label(build));
    }
    out.push(info);
    out.push(gauge("agent_dsm_storage_health", storage_health(report)));

    for array in report.arrays.iter().take(MAX_ARRAYS) {
        let tag = |sample: Sample| sample.with_label("array", &array.name);
        let level = if array.level.is_empty() { "unknown" } else { array.level.as_str() };
        let status = gauge("agent_dsm_raid_status", array.state.value())
            .with_label("level", level)
            .with_label("role", array.role);
        out.push(tag(status));
        out.push(tag(gauge("agent_dsm_raid_members", array.members.len() as f64)));
        out.push(tag(gauge("agent_dsm_raid_failed_members", array.failed.len() as f64)));
        if let Some(sync) = &array.sync
            && let Some(percent) = sync.percent
        {
            let progress = gauge("agent_dsm_raid_sync_percent", percent);
            out.push(tag(progress.with_label("action", &sync.action)));
        }
        for member in &array.failed {
            let failed = gauge("agent_dsm_raid_failed_member_info", 1.0);
            out.push(tag(failed.with_label("member", member)));
        }
    }

    for volume in &report.volumes {
        out.push(
            gauge("agent_dsm_volume_read_only", flag(volume.read_only))
                .with_label("volume", &volume.mount)
                .with_label("fstype", &volume.fstype),
        );
    }

    for disk in report.disks.iter().take(MAX_DISKS) {
        let tag = |sample: Sample| sample.with_label("disk", &disk.name);
        let mut info = tag(gauge("agent_dsm_disk_info", 1.0)).with_label("kind", disk.kind);
        if let Some(model) = &disk.model {
            info = info.with_label("model", label(model));
        }
        out.push(info);
        if let Some(size) = disk.size_bytes {
            out.push(tag(gauge("agent_dsm_disk_size_bytes", size as f64)));
        }
        if let Some(celsius) = disk.temperature_celsius {
            out.push(tag(gauge("agent_dsm_disk_temperature_celsius", celsius)));
        }
        if let Some(life) = disk.remaining_life_percent {
            out.push(tag(gauge("agent_dsm_disk_remaining_life_percent", life)));
        }
        if let Some(unc) = disk.unc_count {
            out.push(tag(gauge("agent_dsm_disk_unc_count", unc as f64)));
        }
    }

    out.push(gauge("agent_dsm_packages", report.packages.len() as f64));
    for package in report.packages.iter().take(MAX_PACKAGES) {
        let tag = |sample: Sample| sample.with_label("package", &package.id);
        let mut info =
            tag(gauge("agent_dsm_package_info", 1.0)).with_label("name", label(&package.name));
        if let Some(version) = &package.version {
            info = info.with_label("version", label(version));
        }
        out.push(info);
        out.push(tag(gauge("agent_dsm_package_enabled", flag(package.enabled))));
        if !package.startable {
            continue;
        }
        let running = match package.run {
            PackageRun::Running => true,
            PackageRun::Stopped => false,
            PackageRun::Unknown => continue,
        };
        out.push(tag(gauge("agent_dsm_package_running", flag(running))));
        // Arrêté alors qu'il doit tourner : un paquet arrêté à la main n'a plus
        // de témoin `enabled` et ne déclenche rien.
        out.push(tag(gauge("agent_dsm_package_down", flag(package.enabled && !running))));
    }

    for (service, running) in &report.services {
        let sample = gauge("agent_dsm_service_running", flag(*running));
        out.push(sample.with_label("service", *service));
    }

    if !report.ups.is_empty() {
        let now = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(now_ms)
            .unwrap_or_else(chrono::Utc::now);
        for (name, vars) in &report.ups {
            out.extend(dumbmonit_collectors::nut::metrics::ups_samples(name, "", vars, now));
        }
    }
    out
}

// ----------------------------------------------------------------- lecture

fn read(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

fn read_trimmed(path: &Path) -> Option<String> {
    read(path).map(|text| text.trim().to_string()).filter(|text| !text.is_empty())
}

pub fn read_system(paths: &Paths) -> DsmSystem {
    let (version, build) = read(&paths.at("/etc.defaults/VERSION"))
        .map(|text| parse_version(&text))
        .unwrap_or_default();
    let synoinfo = read(&paths.at("/etc.defaults/synoinfo.conf"))
        .or_else(|| read(&paths.at("/etc/synoinfo.conf")));
    let hw = read(&paths.at("/proc/sys/kernel/syno_hw_version"));
    DsmSystem { model: parse_model(synoinfo.as_deref(), hw.as_deref()), version, build }
}

/// Les disques internes, rangés par nom. Un disque USB (son chemin dans
/// `/sys/devices` passe par `usb`) n'est pas un disque du NAS.
pub fn read_disks(paths: &Paths) -> Vec<DiskStat> {
    let Ok(entries) = std::fs::read_dir(paths.at("/sys/block")) else { return Vec::new() };
    let mut disks = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !is_disk_name(&name) {
            continue;
        }
        let block = entry.path();
        let through_usb = std::fs::read_link(&block)
            .is_ok_and(|target| target.to_string_lossy().contains("/usb"));
        if through_usb {
            continue;
        }
        let syno = paths.at(&format!("/run/synostorage/disks/{name}"));
        let rotational = read_trimmed(&block.join("queue/rotational"));
        let kind = if name.starts_with("nvme") {
            "nvme"
        } else if rotational.as_deref() == Some("0") {
            "ssd"
        } else {
            "hdd"
        };
        let size_bytes = read(&block.join("size"))
            .and_then(|text| text.trim().parse::<u64>().ok())
            .map(|sectors| sectors.saturating_mul(512))
            .filter(|bytes| *bytes > 0);
        let model = read_trimmed(&block.join("device/model"))
            .or_else(|| read_trimmed(&syno.join("model")));
        let number = |file: &str| read(&syno.join(file)).and_then(|text| parse_number(&text));
        disks.push(DiskStat {
            kind,
            size_bytes,
            model,
            temperature_celsius: number("temperature").filter(|c| (1.0..=100.0).contains(c)),
            remaining_life_percent: number("remain_life").filter(|p| (0.0..=100.0).contains(p)),
            unc_count: number("unc").filter(|n| *n >= 0.0).map(|n| n as u64),
            name,
        });
    }
    disks.sort_by(|a, b| natural_key(&a.name).cmp(&natural_key(&b.name)));
    disks.truncate(MAX_DISKS);
    disks
}

/// `sata2` avant `sata10`.
fn natural_key(name: &str) -> (String, u64) {
    let split = name.trim_end_matches(|c: char| c.is_ascii_digit()).len();
    (name[..split].to_string(), name[split..].parse().unwrap_or(0))
}

/// Les paquets installés, sans leur état réel.
pub fn list_packages(paths: &Paths) -> Vec<PackageStat> {
    let Ok(entries) = std::fs::read_dir(paths.at("/var/packages")) else { return Vec::new() };
    let mut packages: Vec<PackageStat> = entries
        .flatten()
        .filter_map(|entry| {
            let id = entry.file_name().to_string_lossy().to_string();
            if !is_safe_name(&id) {
                return None;
            }
            let dir = entry.path();
            let text = read(&dir.join("INFO"))?;
            let mut package = parse_info(&id, &text);
            package.enabled = dir.join("enabled").exists();
            Some(package)
        })
        .collect();
    packages.sort_by(|a, b| a.id.cmp(&b.id));
    packages.truncate(MAX_PACKAGES);
    packages
}

fn read_services(paths: &Paths) -> Vec<(&'static str, bool)> {
    let Ok(entries) = std::fs::read_dir(paths.at("/proc")) else { return Vec::new() };
    let comms: Vec<String> = entries
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().bytes().all(|b| b.is_ascii_digit()))
        .filter_map(|entry| read(&entry.path().join("comm")))
        .collect();
    services_from(comms.iter().map(String::as_str))
}

/// Ce que la tâche de fond rapporte.
#[derive(Debug, Default)]
struct CommandsReport {
    services: Vec<(&'static str, bool)>,
    ups: Vec<UpsReading>,
    /// `None` : paquets pas relus à ce tour.
    packages: Option<Vec<PackageStat>>,
    /// `Some(false)` : `synopkg` n'a rien répondu d'utilisable.
    synopkg_usable: Option<bool>,
}

async fn read_commands(paths: Paths, with_packages: bool, synopkg: bool) -> CommandsReport {
    let services = read_services(&paths);
    let ups = read_ups().await;
    let (packages, synopkg_usable) = if with_packages {
        let mut packages = list_packages(&paths);
        let usable = if synopkg {
            Some(read_package_states(&paths, &mut packages).await)
        } else {
            None
        };
        (Some(packages), usable)
    } else {
        (None, None)
    };
    CommandsReport { services, ups, packages, synopkg_usable }
}

/// Interroge `synopkg status` pour chaque paquet à service. Trois silences de
/// suite sans aucune réponse : `synopkg` est refusé à cet utilisateur, on
/// arrête là. Vrai si au moins une réponse a été comprise.
async fn read_package_states(paths: &Paths, packages: &mut [PackageStat]) -> bool {
    let full = paths.at("/usr/syno/bin/synopkg");
    let bin =
        if full.is_file() { full.to_string_lossy().to_string() } else { "synopkg".to_string() };
    let mut answered = 0usize;
    let mut silent = 0usize;
    for package in packages.iter_mut().filter(|package| package.startable) {
        if answered == 0 && silent >= 3 {
            break;
        }
        match synopkg_status(&bin, &package.id).await {
            Some(run) => {
                package.run = run;
                answered += 1;
            }
            None => silent += 1,
        }
    }
    answered > 0
}

async fn synopkg_status(bin: &str, package: &str) -> Option<PackageRun> {
    let output = tokio::time::timeout(
        COMMAND_TIMEOUT,
        tokio::process::Command::new(bin)
            .arg("status")
            .arg(package)
            .env("LC_ALL", "C")
            .kill_on_drop(true)
            .output(),
    )
    .await
    .ok()?
    .ok()?;
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    parse_synopkg_status(&text, output.status.code())
}

async fn read_ups() -> Vec<UpsReading> {
    let Some(list) = run_text("upsc", &["-l", "localhost"]).await else { return Vec::new() };
    let mut out = Vec::new();
    for name in parse_upsc_list(&list).into_iter().take(MAX_UPS) {
        let address = format!("{name}@localhost");
        if let Some(text) = run_text("upsc", &[address.as_str()]).await {
            let vars = parse_upsc(&text);
            if !vars.is_empty() {
                out.push((name, vars));
            }
        }
    }
    out
}

/// Sortie standard d'une commande réussie ; `None` sinon, sans bruit.
async fn run_text(bin: &str, args: &[&str]) -> Option<String> {
    let output = tokio::time::timeout(
        COMMAND_TIMEOUT,
        tokio::process::Command::new(bin)
            .args(args)
            .env("LC_ALL", "C")
            .kill_on_drop(true)
            .output(),
    )
    .await
    .ok()?
    .ok()?;
    output.status.success().then(|| String::from_utf8_lossy(&output.stdout).to_string())
}

pub struct DsmProbe {
    paths: Paths,
    /// `None` tant que la détection n'a pas eu lieu.
    detected: Option<bool>,
    services: Vec<(&'static str, bool)>,
    ups: Vec<UpsReading>,
    packages: Vec<PackageStat>,
    refreshed_at: Option<tokio::time::Instant>,
    packages_at: Option<tokio::time::Instant>,
    synopkg_retry_at: Option<tokio::time::Instant>,
    task: Option<JoinHandle<CommandsReport>>,
}

impl Default for DsmProbe {
    fn default() -> Self {
        Self::new(Paths::system())
    }
}

impl DsmProbe {
    pub fn new(paths: Paths) -> Self {
        Self {
            paths,
            detected: None,
            services: Vec::new(),
            ups: Vec::new(),
            packages: Vec::new(),
            refreshed_at: None,
            packages_at: None,
            synopkg_retry_at: None,
            task: None,
        }
    }

    /// Un cycle. `None` partout ailleurs que sur un Synology.
    pub async fn read(&mut self) -> Option<DsmReport> {
        let detected = *self.detected.get_or_insert_with(|| {
            let dsm = is_dsm(&self.paths);
            if dsm {
                info!("Synology DSM detected, reading volumes, RAID, disks and packages");
            }
            dsm
        });
        if !detected {
            return None;
        }

        self.refresh_commands().await;

        Some(DsmReport {
            system: read_system(&self.paths),
            arrays: read(&self.paths.at("/proc/mdstat"))
                .map(|text| parse_mdstat(&text))
                .unwrap_or_default(),
            volumes: read(&self.paths.at("/proc/mounts"))
                .map(|text| parse_mounts(&text))
                .unwrap_or_default(),
            disks: read_disks(&self.paths),
            packages: self.packages.clone(),
            services: self.services.clone(),
            ups: self.ups.clone(),
        })
    }

    async fn refresh_commands(&mut self) {
        let due = |at: Option<tokio::time::Instant>, every: Duration| {
            at.is_none_or(|at| at.elapsed() >= every)
        };
        if self.task.is_none() && due(self.refreshed_at, COMMANDS_INTERVAL) {
            let with_packages = due(self.packages_at, PACKAGES_INTERVAL);
            let synopkg =
                self.synopkg_retry_at.is_none_or(|at| tokio::time::Instant::now() >= at);
            if with_packages {
                self.packages_at = Some(tokio::time::Instant::now());
            }
            self.task =
                Some(tokio::spawn(read_commands(self.paths.clone(), with_packages, synopkg)));
        }
        let Some(task) = self.task.as_mut() else { return };
        let first_cycle = self.refreshed_at.is_none();
        if (task.is_finished() || first_cycle)
            && let Ok(outcome) = tokio::time::timeout(FIRST_CYCLE_WAIT, task).await
        {
            self.task = None;
            self.refreshed_at = Some(tokio::time::Instant::now());
            match outcome {
                Ok(report) => self.adopt(report),
                Err(error) => debug!(%error, "DSM reading aborted"),
            }
        }
    }

    fn adopt(&mut self, report: CommandsReport) {
        self.services = report.services;
        self.ups = report.ups;
        if let Some(packages) = report.packages {
            self.packages = packages;
        }
        match report.synopkg_usable {
            Some(false) => {
                if self.synopkg_retry_at.is_none() {
                    debug!("synopkg status is not available to this user, package states unknown");
                }
                self.synopkg_retry_at = Some(tokio::time::Instant::now() + SYNOPKG_RETRY);
            }
            Some(true) => self.synopkg_retry_at = None,
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MDSTAT_HEALTHY: &str = include_str!("testdata/dsm_mdstat_healthy.txt");
    const MDSTAT_DEGRADED: &str = include_str!("testdata/dsm_mdstat_degraded.txt");
    const VERSION: &str = include_str!("testdata/dsm_version.txt");
    const UPSC: &str = include_str!("testdata/dsm_upsc.txt");
    const MOUNTS: &str = include_str!("testdata/dsm_mounts.txt");

    fn value(samples: &[Sample], metric: &str, labels: &[(&str, &str)]) -> Option<f64> {
        samples
            .iter()
            .find(|s| {
                s.metric == metric
                    && labels.iter().all(|(k, v)| s.labels.get(*k).map(String::as_str) == Some(*v))
            })
            .map(|s| s.value)
    }

    #[test]
    fn the_version_file_gives_dsm_build_and_update() {
        let (version, build) = parse_version(VERSION);
        assert_eq!(version.as_deref(), Some("DSM 7.2.1-69057 Update 5"));
        assert_eq!(build.as_deref(), Some("69057"));

        let (version, _) = parse_version("majorversion=\"7\"\nminorversion=\"1\"\nmicro=\"0\"\n");
        assert_eq!(version.as_deref(), Some("DSM 7.1"), "sans productversion ni build");
        assert_eq!(parse_version("garbage"), (None, None));
    }

    #[test]
    fn the_model_prefers_synoinfo_then_the_kernel() {
        let synoinfo = "unique=\"synology_geminilake_920+\"\nupnpmodelname=\"DS920+\"\n";
        assert_eq!(parse_model(Some(synoinfo), Some("DS918+\n")).as_deref(), Some("DS920+"));
        let kernel = parse_model(Some("unique=\"x\"\n"), Some("DS918+\n"));
        assert_eq!(kernel.as_deref(), Some("DS918+"));
        assert_eq!(parse_model(None, None), None);
    }

    #[test]
    fn a_healthy_nas_has_normal_arrays_and_the_system_slots_are_not_missing_disks() {
        let arrays = parse_mdstat(MDSTAT_HEALTHY);
        let names: Vec<&str> = arrays.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["md2", "md1", "md0"]);
        for array in &arrays {
            assert_eq!(array.state, RaidState::Normal, "{}", array.name);
        }
        let md2 = &arrays[0];
        assert_eq!(md2.level, "raid5");
        assert_eq!(md2.role, "data");
        assert_eq!(md2.members.len(), 4);
        assert_eq!(md2.slots, Some((4, 4)));
        let md0 = &arrays[2];
        assert_eq!(md0.role, "system");
        assert_eq!(md0.slots, Some((16, 4)), "seize emplacements, quatre disques : normal");
    }

    #[test]
    fn a_failed_disk_degrades_the_data_array_and_the_rebuild_is_tracked() {
        let arrays = parse_mdstat(MDSTAT_DEGRADED);
        let md2 = arrays.iter().find(|a| a.name == "md2").unwrap();
        assert_eq!(md2.state, RaidState::Degraded);
        let sync = md2.sync.as_ref().unwrap();
        assert_eq!(sync.action, "recovery");
        assert_eq!(sync.percent, Some(12.6));

        let md3 = arrays.iter().find(|a| a.name == "md3").unwrap();
        assert_eq!(md3.failed, ["sata4p6"]);
        assert_eq!(md3.state, RaidState::Degraded);

        let md0 = arrays.iter().find(|a| a.name == "md0").unwrap();
        assert_eq!(md0.failed, ["sata4p1"]);
        assert_eq!(md0.state, RaidState::Degraded, "un (F) dégrade même la grappe système");

        let md4 = arrays.iter().find(|a| a.name == "md4").unwrap();
        assert_eq!(md4.state, RaidState::Failed);
        assert!(md4.members.is_empty(), "le disque de rechange ne compte pas");

        let md5 = arrays.iter().find(|a| a.name == "md5").unwrap();
        assert_eq!(md5.state, RaidState::Syncing);
        assert_eq!(md5.sync.as_ref().unwrap().percent, None, "resync=DELAYED");

        let md6 = arrays.iter().find(|a| a.name == "md6").unwrap();
        assert_eq!(md6.state, RaidState::Normal, "une vérification n'est pas une panne");
        assert_eq!(md6.level, "raid1", "(auto-read-only) n'est pas le niveau");
    }

    #[test]
    fn volumes_are_read_from_mounts_and_a_read_only_one_is_flagged() {
        let volumes = parse_mounts(MOUNTS);
        assert_eq!(
            volumes,
            vec![
                VolumeMount { mount: "/volume1".into(), fstype: "btrfs".into(), read_only: false },
                VolumeMount { mount: "/volume2".into(), fstype: "ext4".into(), read_only: true },
            ]
        );
    }

    #[test]
    fn only_internal_disk_names_are_kept() {
        for name in ["sata1", "sata12", "sda", "sdab", "nvme0n1", "nvme1n2"] {
            assert!(is_disk_name(name), "{name}");
        }
        for name in ["sata1p5", "sda5", "nvme0n1p1", "synoboot", "md2", "loop0", "zram0", "sata"]
        {
            assert!(!is_disk_name(name), "{name}");
        }
    }

    #[test]
    fn package_info_and_synopkg_answers() {
        let info = "package=\"HyperBackup\"\nversion=\"4.1.0-3718\"\n\
                    displayname=\"Hyper Backup\"\nmaintainer=\"Synology Inc.\"\n";
        let package = parse_info("HyperBackup", info);
        assert_eq!(package.name, "Hyper Backup");
        assert_eq!(package.version.as_deref(), Some("4.1.0-3718"));
        assert!(package.startable);
        assert!(!parse_info("Lib", "package=\"Lib\"\nstartable=\"no\"\n").startable);
        assert_eq!(parse_info("Bare", "").name, "Bare");

        let json = r#"{"package":"HyperBackup","status":"running"}"#;
        assert_eq!(parse_synopkg_status(json, Some(0)), Some(PackageRun::Running));
        let json = r#"{"package":"Plex", "status" : "stop"}"#;
        assert_eq!(parse_synopkg_status(json, Some(3)), Some(PackageRun::Stopped));
        let sentence = parse_synopkg_status("package Docker is started", None);
        assert_eq!(sentence, Some(PackageRun::Running));
        let sentence = parse_synopkg_status("package Docker is not running", Some(0));
        assert_eq!(sentence, Some(PackageRun::Stopped));
        assert_eq!(parse_synopkg_status("", Some(3)), Some(PackageRun::Stopped));
        assert_eq!(parse_synopkg_status("synopkg: Permission denied", Some(0)), None);
        assert_eq!(parse_synopkg_status("", Some(255)), None);
    }

    #[test]
    fn upsc_output_becomes_nut_series() {
        assert_eq!(parse_upsc_list("ups\n\n-bad\n"), ["ups"]);
        let vars = parse_upsc(UPSC);
        assert!(vars.iter().any(|(k, v)| k == "ups.status" && v == "OB DISCHRG"));
        assert!(!vars.iter().any(|(k, _)| k.starts_with("Init")));

        let report = DsmReport { ups: vec![("ups".into(), vars)], ..DsmReport::default() };
        let samples = samples(&report, 1_760_000_000_000);
        assert_eq!(value(&samples, "nut_ups_on_battery", &[("ups", "ups")]), Some(1.0));
        assert_eq!(value(&samples, "nut_battery_charge_percent", &[("ups", "ups")]), Some(87.0));
    }

    #[test]
    fn services_are_recognised_by_their_process() {
        let services = services_from(["synoscgi\n", "smbd\n", "nfsd\n", "nfsd\n"]);
        assert_eq!(services, [("smb", true), ("nfs", true), ("ssh", false)]);
    }

    #[test]
    fn samples_cover_storage_disks_and_packages() {
        let report = DsmReport {
            system: DsmSystem {
                model: Some("DS920+".into()),
                version: Some("DSM 7.2.1-69057 Update 5".into()),
                build: Some("69057".into()),
            },
            arrays: parse_mdstat(MDSTAT_DEGRADED),
            volumes: parse_mounts(MOUNTS),
            disks: vec![DiskStat {
                name: "sata1".into(),
                model: Some("WD80EFZZ".into()),
                kind: "hdd",
                size_bytes: Some(8_001_563_222_016),
                temperature_celsius: Some(41.0),
                remaining_life_percent: None,
                unc_count: Some(0),
            }],
            packages: vec![
                PackageStat {
                    id: "HyperBackup".into(),
                    name: "Hyper Backup".into(),
                    version: Some("4.1.0".into()),
                    enabled: true,
                    startable: true,
                    run: PackageRun::Stopped,
                },
                PackageStat {
                    id: "Plex".into(),
                    name: "Plex".into(),
                    version: None,
                    enabled: false,
                    startable: true,
                    run: PackageRun::Stopped,
                },
                PackageStat {
                    id: "SynologyDrive".into(),
                    name: "Synology Drive Server".into(),
                    version: None,
                    enabled: true,
                    startable: true,
                    run: PackageRun::Unknown,
                },
            ],
            services: vec![("smb", true)],
            ups: Vec::new(),
        };
        let samples = samples(&report, 0);
        assert!(samples.iter().all(|s| s.value.is_finite()));
        let info = samples.iter().find(|s| s.metric == "agent_dsm_info").unwrap();
        assert_eq!(info.labels.get("model").map(String::as_str), Some("DS920+"));
        assert!(!info.labels.contains_key("serial"), "jamais de numéro de série");
        assert_eq!(value(&samples, "agent_dsm_storage_health", &[]), Some(2.0));
        assert_eq!(value(&samples, "agent_dsm_raid_status", &[("array", "md2")]), Some(2.0));
        assert_eq!(value(&samples, "agent_dsm_raid_status", &[("array", "md4")]), Some(3.0));
        assert_eq!(
            value(
                &samples,
                "agent_dsm_raid_sync_percent",
                &[("array", "md2"), ("action", "recovery")]
            ),
            Some(12.6)
        );
        assert_eq!(
            value(&samples, "agent_dsm_raid_failed_member_info", &[("member", "sata4p6")]),
            Some(1.0)
        );
        let read_only = value(&samples, "agent_dsm_volume_read_only", &[("volume", "/volume2")]);
        assert_eq!(read_only, Some(1.0));
        assert_eq!(
            value(&samples, "agent_dsm_disk_temperature_celsius", &[("disk", "sata1")]),
            Some(41.0)
        );
        assert_eq!(value(&samples, "agent_dsm_packages", &[]), Some(3.0));
        let down = value(&samples, "agent_dsm_package_down", &[("package", "HyperBackup")]);
        assert_eq!(down, Some(1.0));
        assert_eq!(value(&samples, "agent_dsm_package_down", &[("package", "Plex")]), Some(0.0));
        assert_eq!(
            value(&samples, "agent_dsm_package_running", &[("package", "SynologyDrive")]),
            None,
            "état inconnu : pas de série plutôt qu'un arrêt supposé"
        );
        assert_eq!(value(&samples, "agent_dsm_service_running", &[("service", "smb")]), Some(1.0));
    }

    #[test]
    fn a_healthy_nas_reports_storage_health_zero() {
        let report = DsmReport {
            arrays: parse_mdstat(MDSTAT_HEALTHY),
            volumes: vec![VolumeMount {
                mount: "/volume1".into(),
                fstype: "btrfs".into(),
                read_only: false,
            }],
            ..DsmReport::default()
        };
        assert_eq!(storage_health(&report), 0.0);
    }

    fn write(root: &Path, path: &str, text: &str) {
        let full = root.join(path.trim_start_matches('/'));
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, text).unwrap();
    }

    #[test]
    fn disks_and_packages_are_read_from_the_file_tree() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "/sys/block/sata2/size", "15628053168\n");
        write(root, "/sys/block/sata2/queue/rotational", "1\n");
        write(root, "/sys/block/sata2/device/model", "ST8000VN004-2M2101  \n");
        write(root, "/run/synostorage/disks/sata2/temperature", "38\n");
        write(root, "/run/synostorage/disks/sata2/unc", "0\n");
        write(root, "/sys/block/nvme0n1/size", "976773168\n");
        write(root, "/sys/block/nvme0n1/queue/rotational", "0\n");
        write(root, "/run/synostorage/disks/nvme0n1/remain_life", "97\n");
        write(root, "/run/synostorage/disks/nvme0n1/temperature", "-1\n");
        write(root, "/sys/block/sata10/size", "100\n");
        write(root, "/sys/block/md2/size", "100\n");

        let info = "package=\"HyperBackup\"\ndisplayname=\"Hyper Backup\"\n";
        write(root, "/var/packages/HyperBackup/INFO", info);
        write(root, "/var/packages/HyperBackup/enabled", "");
        write(root, "/var/packages/Plex/INFO", "package=\"Plex\"\n");
        std::fs::create_dir_all(root.join("var/packages/NoInfo")).unwrap();

        let disks = read_disks(&Paths::under(root));
        let names: Vec<&str> = disks.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, ["nvme0n1", "sata2", "sata10"]);
        let sata2 = &disks[1];
        assert_eq!(sata2.kind, "hdd");
        assert_eq!(sata2.model.as_deref(), Some("ST8000VN004-2M2101"));
        assert_eq!(sata2.size_bytes, Some(15_628_053_168 * 512));
        assert_eq!(sata2.temperature_celsius, Some(38.0));
        assert_eq!(sata2.unc_count, Some(0));
        let nvme = &disks[0];
        assert_eq!(nvme.kind, "nvme");
        assert_eq!(nvme.remaining_life_percent, Some(97.0));
        assert_eq!(nvme.temperature_celsius, None, "une sonde à -1 ne dit rien");

        let packages = list_packages(&Paths::under(root));
        let ids: Vec<(&str, bool)> = packages.iter().map(|p| (p.id.as_str(), p.enabled)).collect();
        assert_eq!(ids, [("HyperBackup", true), ("Plex", false)]);
    }

    #[tokio::test]
    async fn outside_dsm_nothing_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let mut probe = DsmProbe::new(Paths::under(dir.path()));
        assert!(probe.read().await.is_none());

        write(dir.path(), "/etc.defaults/VERSION", VERSION);
        std::fs::create_dir_all(dir.path().join("usr/syno")).unwrap();
        assert!(is_dsm(&Paths::under(dir.path())));
    }

    #[tokio::test]
    async fn on_dsm_the_probe_reports_from_the_file_tree() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, "/etc.defaults/VERSION", VERSION);
        write(root, "/etc.defaults/synoinfo.conf", "upnpmodelname=\"DS920+\"\n");
        write(root, "/proc/mdstat", MDSTAT_HEALTHY);
        write(root, "/proc/mounts", MOUNTS);
        std::fs::create_dir_all(root.join("usr/syno")).unwrap();
        let mut probe = DsmProbe::new(Paths::under(root));
        let report = probe.read().await.expect("DSM detected");
        assert_eq!(report.system.model.as_deref(), Some("DS920+"));
        assert_eq!(report.arrays.len(), 3);
        assert_eq!(report.volumes.len(), 2);
    }
}
