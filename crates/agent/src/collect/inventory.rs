//! Inventaire logiciel de la machine, pour la recherche de failles (CVE) côté
//! serveur.
//!
//! Une fois par jour, une tâche de fond relève ce qui est installé : la liste du
//! gestionnaire de paquets (`dpkg-query`, `rpm -qa`, `apk list -I`) sous Linux, la
//! liste des programmes installés et le numéro de build du système dans le
//! registre sous Windows. Le résultat est déposé dans un emplacement partagé
//! ([`PendingInventory`]) d'où la boucle d'envoi le joint au prochain lot.
//!
//! Comme pour la santé du système, la lecture (commandes, registre) et l'analyse
//! sont séparées : les analyseurs sont des fonctions pures, vérifiées sur des
//! sorties réelles et compilées partout, seule l'exécution est propre à chaque
//! système. Aucune commande ne passe par un interpréteur (hors PowerShell, pour
//! les correctifs Windows) et chacune est bornée par un délai strict. Un échec ne
//! produit pas d'inventaire ce jour-là, jamais d'erreur pour le cycle de mesure.

// Hors Linux et Windows, seules les fonctions pures sont compilées : les tests
// tournent partout, la lecture du système n'a de sens que sur ces deux-là.
#![cfg_attr(not(any(target_os = "linux", windows)), allow(dead_code))]

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use dumbmonit_proto::{InstalledPackage, MAX_INVENTORY_PACKAGES, SoftwareInventory};
use tracing::{debug, warn};

#[cfg(any(windows, test))]
use crate::collect::system_health::parse_reg_dword;

#[cfg(any(target_os = "linux", test))]
use crate::collect::system_health::{OsInfo, parse_os_release};

/// Attente avant le premier relevé : laisse l'agent s'installer (premiers lots,
/// enregistrement auprès du serveur) avant de lancer une commande coûteuse.
pub const FIRST_DELAY: Duration = Duration::from_secs(120);
/// Période entre deux relevés réussis.
pub const INTERVAL: Duration = Duration::from_secs(24 * 3600);
/// Nouvelle tentative après un relevé sans résultat.
pub const RETRY_DELAY: Duration = Duration::from_secs(3600);
/// Délai de chaque commande : `rpm -qa` ou `reg query /s` sont lents sur une
/// machine chargée.
const COMMAND_TIMEOUT: Duration = Duration::from_secs(60);

/// Dernier inventaire relevé et pas encore acquitté par le serveur.
pub type PendingInventory = Arc<Mutex<Option<SoftwareInventory>>>;

/// Verrou tolérant : un relevé qui aurait paniqué ne doit pas bloquer l'envoi.
pub fn lock(pending: &PendingInventory) -> MutexGuard<'_, Option<SoftwareInventory>> {
    pending.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Lance la tâche de fond : premier relevé après [`FIRST_DELAY`], puis un par
/// [`INTERVAL`]. Le résultat remplace celui qui attendrait encore.
pub fn spawn(pending: PendingInventory) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        tokio::time::sleep(FIRST_DELAY).await;
        loop {
            let wait = match collect_now().await {
                Some(inventory) => {
                    debug!(
                        source = inventory.source,
                        packages = inventory.packages.len(),
                        "software inventory collected"
                    );
                    *lock(&pending) = Some(inventory);
                    INTERVAL
                }
                None => RETRY_DELAY,
            };
            tokio::time::sleep(wait).await;
        }
    })
}

// ----------------------------------------------------------------- analyse

/// Trie, dédoublonne (nom, version) et tronque à [`MAX_INVENTORY_PACKAGES`].
pub fn finish_packages(mut packages: Vec<InstalledPackage>) -> Vec<InstalledPackage> {
    packages.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.version.cmp(&b.version)));
    packages.dedup_by(|a, b| a.name == b.name && a.version == b.version);
    if packages.len() > MAX_INVENTORY_PACKAGES {
        warn!(
            found = packages.len(),
            kept = MAX_INVENTORY_PACKAGES,
            "software inventory truncated"
        );
        packages.truncate(MAX_INVENTORY_PACKAGES);
    }
    packages
}

fn non_empty(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

/// Format de `dpkg-query` : nom, version, paquet source, version source, état.
#[cfg(any(target_os = "linux", test))]
const DPKG_FORMAT: &str =
    "${Package}\t${Version}\t${source:Package}\t${source:Version}\t${db:Status-Abbrev}\n";
/// Format de `rpm -qa` : nom, époque, version, révision, RPM source.
#[cfg(any(target_os = "linux", test))]
const RPM_FORMAT: &str = "%{NAME}\t%{EPOCHNUM}\t%{VERSION}\t%{RELEASE}\t%{SOURCERPM}\n";

/// Gestionnaire de paquets d'une distribution Linux.
#[cfg(any(target_os = "linux", test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageSource {
    Dpkg,
    Rpm,
    Apk,
}

#[cfg(any(target_os = "linux", test))]
impl PackageSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::Dpkg => "dpkg",
            Self::Rpm => "rpm",
            Self::Apk => "apk",
        }
    }

    fn command(self) -> (&'static str, &'static [&'static str]) {
        match self {
            Self::Dpkg => ("dpkg-query", &["-W", "-f", DPKG_FORMAT]),
            Self::Rpm => ("rpm", &["-qa", "--qf", RPM_FORMAT]),
            Self::Apk => ("apk", &["list", "-I"]),
        }
    }

    fn parse(self, stdout: &str) -> Vec<InstalledPackage> {
        match self {
            Self::Dpkg => parse_dpkg(stdout),
            Self::Rpm => parse_rpm(stdout),
            Self::Apk => parse_apk(stdout),
        }
    }
}

/// Gestionnaires à essayer, le plus probable d'abord : celui de la famille
/// annoncée par `/etc/os-release`, puis les autres au cas où le fichier mentirait
/// ou manquerait (conteneur minimal).
#[cfg(any(target_os = "linux", test))]
pub fn package_sources(os: &OsInfo) -> Vec<PackageSource> {
    let family = format!("{} {}", os.id, os.id_like).to_ascii_lowercase();
    let is = |names: &[&str]| family.split_whitespace().any(|f| names.contains(&f));
    let first = if is(&["debian", "ubuntu"]) {
        PackageSource::Dpkg
    } else if is(&["alpine"]) {
        PackageSource::Apk
    } else {
        PackageSource::Rpm
    };
    let mut sources = vec![first];
    for other in [PackageSource::Dpkg, PackageSource::Rpm, PackageSource::Apk] {
        if other != first {
            sources.push(other);
        }
    }
    sources
}

/// Retire un éventuel ` (version)` que certains `dpkg` accolent au nom source.
#[cfg(any(target_os = "linux", test))]
fn strip_parenthesis(text: &str) -> &str {
    text.split_once(" (").map_or(text, |(head, _)| head).trim()
}

/// Sortie de `dpkg-query -W -f '${Package}\t${Version}\t${source:Package}\t
/// ${source:Version}\t${db:Status-Abbrev}\n'`. Seuls les paquets installés sont
/// retenus (second caractère de l'état : `i`).
#[cfg(any(target_os = "linux", test))]
pub fn parse_dpkg(stdout: &str) -> Vec<InstalledPackage> {
    stdout
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let name = fields.next()?.trim();
            let version = fields.next()?.trim();
            let source = strip_parenthesis(fields.next()?);
            let source_version = strip_parenthesis(fields.next()?);
            let status = fields.next()?;
            if status.chars().nth(1) != Some('i') || name.is_empty() || version.is_empty() {
                return None;
            }
            Some(InstalledPackage {
                name: name.to_string(),
                version: version.to_string(),
                source_name: non_empty(source),
                source_version: non_empty(source_version).filter(|v| v != version),
                publisher: None,
            })
        })
        .collect()
}

/// Sortie de `rpm -qa --qf '%{NAME}\t%{EPOCHNUM}\t%{VERSION}\t%{RELEASE}\t
/// %{SOURCERPM}\n'`. La version est `VERSION-RELEASE`, précédée de `EPOCH:`
/// quand l'époque n'est pas nulle ; les clés GPG importées (`gpg-pubkey`) ne
/// sont pas des logiciels.
#[cfg(any(target_os = "linux", test))]
pub fn parse_rpm(stdout: &str) -> Vec<InstalledPackage> {
    stdout
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let name = fields.next()?.trim();
            let epoch = fields.next()?.trim();
            let version = fields.next()?.trim();
            let release = fields.next()?.trim();
            let source_rpm = fields.next().unwrap_or("").trim();
            if name.is_empty() || name == "gpg-pubkey" || version.is_empty() {
                return None;
            }
            let epoch = epoch.parse::<u64>().unwrap_or(0);
            let mut full = if epoch == 0 { String::new() } else { format!("{epoch}:") };
            full.push_str(version);
            if !release.is_empty() && release != "(none)" {
                full.push('-');
                full.push_str(release);
            }
            Some(InstalledPackage {
                name: name.to_string(),
                version: full,
                source_name: rpm_source_name(source_rpm),
                source_version: None,
                publisher: None,
            })
        })
        .collect()
}

/// Nom du paquet source d'un `SOURCERPM` (`bash-5.2.26-3.fc40.src.rpm` → `bash`).
#[cfg(any(target_os = "linux", test))]
fn rpm_source_name(source_rpm: &str) -> Option<String> {
    if source_rpm.is_empty() || source_rpm == "(none)" {
        return None;
    }
    let base = source_rpm.strip_suffix(".rpm").unwrap_or(source_rpm);
    let base = base.strip_suffix(".src").or_else(|| base.strip_suffix(".nosrc")).unwrap_or(base);
    let mut parts = base.rsplitn(3, '-');
    let (_release, _version, name) = (parts.next()?, parts.next()?, parts.next()?);
    non_empty(name)
}

/// Sortie de `apk list -I` : `musl-1.2.5-r0 x86_64 {musl} (MIT) [installed]`. La
/// version est les deux derniers segments séparés par un tiret (`1.2.5-r0`), le
/// paquet source l'origine entre accolades.
#[cfg(any(target_os = "linux", test))]
pub fn parse_apk(stdout: &str) -> Vec<InstalledPackage> {
    stdout
        .lines()
        .filter_map(|line| {
            let full = line.split_whitespace().next()?;
            let mut parts = full.rsplitn(3, '-');
            let release = parts.next()?;
            let version = parts.next()?;
            let name = parts.next()?;
            if name.is_empty() || version.is_empty() || release.is_empty() {
                return None;
            }
            let origin = line
                .split_once('{')
                .and_then(|(_, rest)| rest.split_once('}'))
                .and_then(|(origin, _)| non_empty(origin));
            Some(InstalledPackage {
                name: name.to_string(),
                version: format!("{version}-{release}"),
                source_name: origin,
                source_version: None,
                publisher: None,
            })
        })
        .collect()
}

/// Identité de la distribution pour l'inventaire : (id, version, nom de code).
/// Le nom de code d'un dérivé d'Ubuntu se lit dans `UBUNTU_CODENAME`.
#[cfg(any(target_os = "linux", test))]
fn distro_fields(os: &OsInfo) -> (Option<String>, Option<String>, Option<String>) {
    (
        non_empty(&os.id),
        non_empty(&os.version_id),
        non_empty(&os.version_codename).or_else(|| non_empty(&os.ubuntu_codename)),
    )
}

/// Une ligne de valeur de `reg query` : `    Nom    REG_SZ    données`.
#[cfg(any(windows, test))]
fn parse_reg_value(line: &str) -> Option<(&str, &str, &str)> {
    let line = line.trim();
    let at = line.find(" REG_")?;
    let name = line[..at].trim();
    let rest = line[at..].trim_start();
    let (kind, data) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
    Some((name, kind, data.trim()))
}

/// Valeur textuelle (`REG_SZ`, `REG_EXPAND_SZ`) d'une sortie de `reg query`.
#[cfg(any(windows, test))]
fn parse_reg_string(stdout: &str, value: &str) -> Option<String> {
    stdout.lines().find_map(|line| {
        let (name, kind, data) = parse_reg_value(line)?;
        (name.eq_ignore_ascii_case(value) && kind.ends_with("SZ")).then(|| non_empty(data))?
    })
}

/// Un sous-clé de `Uninstall` en cours de lecture.
#[cfg(any(windows, test))]
#[derive(Default)]
struct UninstallEntry {
    name: Option<String>,
    version: Option<String>,
    publisher: Option<String>,
    system_component: bool,
    is_update: bool,
}

#[cfg(any(windows, test))]
impl UninstallEntry {
    fn into_package(self) -> Option<InstalledPackage> {
        if self.system_component || self.is_update {
            return None;
        }
        Some(InstalledPackage {
            name: self.name?,
            version: self.version?,
            source_name: None,
            source_version: None,
            publisher: self.publisher,
        })
    }
}

/// Sortie de `reg query HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall
/// /s` : un bloc par programme, la ligne de clé en colonne 1 puis ses valeurs
/// indentées. Sont écartés les blocs sans nom ou sans version, les composants
/// système (`SystemComponent` = 1) et les mises à jour (`ParentKeyName`).
#[cfg(any(windows, test))]
pub fn parse_uninstall(stdout: &str) -> Vec<InstalledPackage> {
    let mut packages = Vec::new();
    let mut current: Option<UninstallEntry> = None;
    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if !line.starts_with(char::is_whitespace) {
            packages.extend(current.take().and_then(UninstallEntry::into_package));
            current = Some(UninstallEntry::default());
            continue;
        }
        let (Some(entry), Some((name, _, data))) = (current.as_mut(), parse_reg_value(line)) else {
            continue;
        };
        match name.to_ascii_lowercase().as_str() {
            "displayname" => entry.name = non_empty(data),
            "displayversion" => entry.version = non_empty(data),
            "publisher" => entry.publisher = non_empty(data),
            "systemcomponent" => {
                entry.system_component = data.eq_ignore_ascii_case("0x1") || data == "1";
            }
            "parentkeyname" => entry.is_update = true,
            _ => {}
        }
    }
    packages.extend(current.and_then(UninstallEntry::into_package));
    packages
}

/// Ce que `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion` dit du système.
#[cfg(any(windows, test))]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WindowsVersion {
    pub product: Option<String>,
    pub release: Option<String>,
    pub build: Option<String>,
}

/// Sortie de `reg query "HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion"`.
///
/// Le build complet est `majeur.mineur.CurrentBuild.UBR`. Le registre de Windows
/// 11 annonce encore « Windows 10 » dans `ProductName` : à partir du build
/// 22000, le nom est corrigé.
#[cfg(any(windows, test))]
pub fn parse_windows_version(stdout: &str) -> WindowsVersion {
    let build_number = parse_reg_string(stdout, "CurrentBuild")
        .or_else(|| parse_reg_string(stdout, "CurrentBuildNumber"));
    let (major, minor) = match (
        parse_reg_dword(stdout, "CurrentMajorVersionNumber"),
        parse_reg_dword(stdout, "CurrentMinorVersionNumber"),
    ) {
        (Some(major), Some(minor)) => (major.to_string(), minor.to_string()),
        // Avant Windows 10 : « 6.3 » dans `CurrentVersion`.
        _ => parse_reg_string(stdout, "CurrentVersion")
            .and_then(|v| v.split_once('.').map(|(a, b)| (a.to_string(), b.to_string())))
            .unwrap_or_else(|| ("10".into(), "0".into())),
    };
    let build = build_number.as_ref().map(|number| {
        let mut build = format!("{major}.{minor}.{number}");
        if let Some(ubr) = parse_reg_dword(stdout, "UBR") {
            build.push_str(&format!(".{ubr}"));
        }
        build
    });
    let mut product = parse_reg_string(stdout, "ProductName");
    let is_windows_11 =
        build_number.as_deref().and_then(|n| n.parse::<u32>().ok()).is_some_and(|n| n >= 22_000);
    if is_windows_11 && product.as_deref().is_some_and(|name| name.starts_with("Windows 10")) {
        product = product.map(|name| name.replacen("Windows 10", "Windows 11", 1));
    }
    WindowsVersion {
        product,
        release: parse_reg_string(stdout, "DisplayVersion")
            .or_else(|| parse_reg_string(stdout, "ReleaseId")),
        build,
    }
}

/// Sortie de `Get-HotFix | ForEach-Object { $_.HotFixID }` : ne garde que les
/// identifiants de la forme `KB1234567`, sans doublon.
#[cfg(any(windows, test))]
pub fn parse_hotfixes(stdout: &str) -> Vec<String> {
    let is_kb = |id: &str| {
        let digits = id.strip_prefix("KB").unwrap_or("");
        !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
    };
    let found: std::collections::BTreeSet<String> =
        stdout.lines().map(str::trim).filter(|id| is_kb(id)).map(str::to_string).collect();
    found.into_iter().collect()
}

// --------------------------------------------------------------- exécution

#[cfg(any(target_os = "linux", windows))]
enum RunError {
    /// Le programme n'est pas installé.
    Missing,
    /// Impossible à lancer, en erreur, ou trop long.
    Failed,
}

/// Lance un programme sans interpréteur, avec un délai strict, et rend sa sortie
/// standard s'il se termine avec succès.
#[cfg(any(target_os = "linux", windows))]
async fn run(program: &str, args: &[&str]) -> Result<String, RunError> {
    let mut command = tokio::process::Command::new(program);
    command
        .args(args)
        .env("LC_ALL", "C")
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    // Pas de fenêtre de console pour le processus fils.
    #[cfg(windows)]
    command.creation_flags(0x0800_0000);

    match tokio::time::timeout(COMMAND_TIMEOUT, command.output()).await {
        Ok(Ok(output)) if output.status.success() => {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        }
        Ok(Ok(output)) => {
            debug!(program, code = ?output.status.code(), "inventory command failed");
            Err(RunError::Failed)
        }
        Ok(Err(error)) if error.kind() == std::io::ErrorKind::NotFound => Err(RunError::Missing),
        Ok(Err(error)) => {
            debug!(program, %error, "inventory command could not be started");
            Err(RunError::Failed)
        }
        Err(_) => {
            debug!(program, timeout_s = COMMAND_TIMEOUT.as_secs(), "inventory command too slow");
            Err(RunError::Failed)
        }
    }
}

/// Relève l'inventaire maintenant. `None` : rien d'exploitable (gestionnaire de
/// paquets absent, commande en échec) ; la cause est au journal.
#[cfg(target_os = "linux")]
pub async fn collect_now() -> Option<SoftwareInventory> {
    let os = match tokio::fs::read_to_string("/etc/os-release").await {
        Ok(text) => parse_os_release(&text),
        Err(_) => tokio::fs::read_to_string("/usr/lib/os-release")
            .await
            .map(|text| parse_os_release(&text))
            .unwrap_or_default(),
    };
    for source in package_sources(&os) {
        let (program, args) = source.command();
        match run(program, args).await {
            Ok(stdout) => {
                let packages = finish_packages(source.parse(&stdout));
                if packages.is_empty() {
                    warn!(source = source.label(), "software inventory is empty, ignored");
                    return None;
                }
                let (distro_id, distro_version, distro_codename) = distro_fields(&os);
                return Some(SoftwareInventory {
                    collected_at_ms: chrono::Utc::now().timestamp_millis(),
                    source: source.label().to_string(),
                    distro_id,
                    distro_version,
                    distro_codename,
                    packages,
                    ..SoftwareInventory::default()
                });
            }
            Err(RunError::Missing) => continue,
            Err(RunError::Failed) => {
                warn!(source = source.label(), "software inventory could not be read");
                return None;
            }
        }
    }
    debug!("no supported package manager, no software inventory");
    None
}

#[cfg(windows)]
const CURRENT_VERSION_KEY: &str = r"HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion";
#[cfg(windows)]
const UNINSTALL_KEYS: [&str; 2] = [
    r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
    r"HKLM\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
];

/// Relève l'inventaire maintenant (voir la version Linux).
#[cfg(windows)]
pub async fn collect_now() -> Option<SoftwareInventory> {
    let version = match run("reg.exe", &["query", CURRENT_VERSION_KEY]).await {
        Ok(stdout) => parse_windows_version(&stdout),
        Err(_) => {
            warn!("Windows version could not be read, no software inventory");
            return None;
        }
    };
    let mut packages = Vec::new();
    for key in UNINSTALL_KEYS {
        // La clé WOW6432Node n'existe pas sur un système 32 bits : sans gravité.
        if let Ok(stdout) = run("reg.exe", &["query", key, "/s"]).await {
            packages.extend(parse_uninstall(&stdout));
        }
    }
    let packages = finish_packages(packages);
    if packages.is_empty() {
        warn!("no installed program found, no software inventory");
        return None;
    }
    // Facultatif : `Get-HotFix` est lent et peut être interdit.
    let hotfixes = match run(
        "powershell.exe",
        &[
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-HotFix | ForEach-Object { $_.HotFixID }",
        ],
    )
    .await
    {
        Ok(stdout) => parse_hotfixes(&stdout),
        Err(_) => Vec::new(),
    };
    Some(SoftwareInventory {
        collected_at_ms: chrono::Utc::now().timestamp_millis(),
        source: "windows".to_string(),
        os_product: version.product,
        os_release: version.release,
        os_build: version.build,
        hotfixes,
        packages,
        ..SoftwareInventory::default()
    })
}

/// Autres systèmes : pas d'inventaire.
#[cfg(not(any(target_os = "linux", windows)))]
pub async fn collect_now() -> Option<SoftwareInventory> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package(name: &str, version: &str) -> InstalledPackage {
        InstalledPackage {
            name: name.into(),
            version: version.into(),
            ..InstalledPackage::default()
        }
    }

    const DPKG: &str = "bash\t5.2.15-2+b7\tbash\t5.2.15-2+b7\tii \n\
libssl3\t3.0.15-1~deb12u1\topenssl\t3.0.15-1~deb12u1\tii \n\
libc6\t2.36-9+deb12u9\tglibc (2.36-9+deb12u9)\t2.36-9+deb12u9\tii \n\
base-files\t12.4+deb12u8\tbase-files\t12.4+deb12u8\tii \n\
libfoo1\t1.0-1+b2\tfoo\t1.0-1\tii \n\
oldpkg\t1.0\toldpkg\t1.0\trc \n\
halfpkg\t2.0\thalfpkg\t2.0\tiU \n\
\n\
ligne cassée\n";

    #[test]
    fn dpkg_keeps_installed_packages_with_their_source() {
        let packages = parse_dpkg(DPKG);
        let names: Vec<&str> = packages.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["bash", "libssl3", "libc6", "base-files", "libfoo1"]);

        let ssl = &packages[1];
        assert_eq!(ssl.version, "3.0.15-1~deb12u1");
        assert_eq!(ssl.source_name.as_deref(), Some("openssl"));
        assert_eq!(ssl.source_version, None, "identique à la version : omise");

        let libc = &packages[2];
        assert_eq!(libc.source_name.as_deref(), Some("glibc"), "suffixe (version) retiré");

        let foo = &packages[4];
        assert_eq!(foo.source_name.as_deref(), Some("foo"));
        assert_eq!(foo.source_version.as_deref(), Some("1.0-1"), "rebuild binNMU");
    }

    #[test]
    fn dpkg_without_source_leaves_the_fields_empty() {
        let packages = parse_dpkg("tool\t1.0\t\t\tii \n");
        assert_eq!(packages, vec![package("tool", "1.0")]);
    }

    const RPM: &str = "bash\t0\t5.2.26\t3.fc40\tbash-5.2.26-3.fc40.src.rpm\n\
openssl-libs\t1\t3.2.1\t2.fc40\topenssl-3.2.1-2.fc40.src.rpm\n\
kernel-core\t0\t6.8.5\t301.fc40\tkernel-6.8.5-301.fc40.src.rpm\n\
gpg-pubkey\t0\te8e40fde\t6371d1ab\t(none)\n\
local-tool\t0\t1.0\t1\t(none)\n";

    #[test]
    fn rpm_builds_versions_with_epoch_and_source_names() {
        let packages = parse_rpm(RPM);
        assert_eq!(packages.len(), 4, "gpg-pubkey écarté");
        assert_eq!(packages[0].version, "5.2.26-3.fc40", "pas d'époque nulle");
        assert_eq!(packages[0].source_name.as_deref(), Some("bash"));
        assert_eq!(packages[1].version, "1:3.2.1-2.fc40");
        assert_eq!(packages[1].source_name.as_deref(), Some("openssl"));
        assert_eq!(packages[2].source_name.as_deref(), Some("kernel"));
        assert_eq!(packages[3].source_name, None, "(none) : pas de source");
    }

    #[test]
    fn rpm_source_name_handles_dashes_in_the_name() {
        assert_eq!(
            rpm_source_name("python-dateutil-2.8.2-1.el9.src.rpm").as_deref(),
            Some("python-dateutil")
        );
        assert_eq!(rpm_source_name("garbage"), None);
    }

    const APK: &str = "musl-1.2.5-r0 x86_64 {musl} (MIT) [installed]\n\
libcrypto3-3.3.2-r0 x86_64 {openssl} (Apache-2.0) [installed]\n\
ca-certificates-bundle-20240705-r0 x86_64 {ca-certificates} (MPL-2.0 AND MIT) [installed]\n\
busybox-1.36.1-r29 x86_64 {busybox} (GPL-2.0-only) [installed]\n\
inconnu [installed]\n";

    #[test]
    fn apk_splits_name_version_and_origin() {
        let packages = parse_apk(APK);
        assert_eq!(packages.len(), 4);
        assert_eq!(packages[0].name, "musl");
        assert_eq!(packages[0].version, "1.2.5-r0");
        assert_eq!(packages[0].source_name.as_deref(), Some("musl"));
        assert_eq!(packages[1].name, "libcrypto3");
        assert_eq!(packages[1].source_name.as_deref(), Some("openssl"));
        assert_eq!(packages[2].name, "ca-certificates-bundle");
        assert_eq!(packages[2].version, "20240705-r0");
    }

    #[test]
    fn the_package_manager_follows_the_distribution_family() {
        let os = |id: &str, like: &str| OsInfo {
            id: id.into(),
            id_like: like.into(),
            ..OsInfo::default()
        };
        assert_eq!(package_sources(&os("ubuntu", "debian"))[0], PackageSource::Dpkg);
        assert_eq!(package_sources(&os("almalinux", "rhel centos fedora"))[0], PackageSource::Rpm);
        assert_eq!(package_sources(&os("alpine", ""))[0], PackageSource::Apk);
        let unknown = package_sources(&os("", ""));
        assert_eq!(unknown.len(), 3, "les trois sont essayés");
    }

    #[test]
    fn distro_codename_falls_back_to_ubuntu_codename() {
        let os = parse_os_release("ID=linuxmint\nVERSION_ID=\"22\"\nUBUNTU_CODENAME=noble\n");
        let (id, version, codename) = distro_fields(&os);
        assert_eq!(id.as_deref(), Some("linuxmint"));
        assert_eq!(version.as_deref(), Some("22"));
        assert_eq!(codename.as_deref(), Some("noble"));

        let debian = parse_os_release("ID=debian\nVERSION_ID=\"12\"\nVERSION_CODENAME=bookworm\n");
        assert_eq!(distro_fields(&debian).2.as_deref(), Some("bookworm"));
        assert_eq!(distro_fields(&parse_os_release("ID=alpine\n")).2, None);
    }

    const UNINSTALL: &str = "\r
HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\7-Zip\r
    DisplayName    REG_SZ    7-Zip 23.01 (x64)\r
    DisplayVersion    REG_SZ    23.01\r
    Publisher    REG_SZ    Igor Pavlov\r
\r
HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{GUID-1}\r
    DisplayName    REG_SZ    Microsoft Visual C++ 2015-2022 Redistributable (x64)\r
    DisplayVersion    REG_SZ    14.38.33135\r
    Publisher    REG_SZ    Microsoft Corporation\r
    SystemComponent    REG_DWORD    0x0\r
\r
HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Hidden\r
    DisplayName    REG_SZ    Composant caché\r
    DisplayVersion    REG_SZ    1.0\r
    SystemComponent    REG_DWORD    0x1\r
\r
HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\KB5044285\r
    DisplayName    REG_SZ    Update for Windows (KB5044285)\r
    DisplayVersion    REG_SZ    1.0\r
    ParentKeyName    REG_SZ    OperatingSystem\r
\r
HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\NoVersion\r
    DisplayName    REG_SZ    Sans version\r
\r
HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\NoName\r
    DisplayVersion    REG_SZ    3.0\r
\r
HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Dernier\r
    DisplayName    REG_SZ    Dernier programme\r
    DisplayVersion    REG_SZ    2.1.0\r
";

    #[test]
    fn uninstall_blocks_keep_real_programs_only() {
        let packages = parse_uninstall(UNINSTALL);
        let found: Vec<(&str, &str, Option<&str>)> = packages
            .iter()
            .map(|p| (p.name.as_str(), p.version.as_str(), p.publisher.as_deref()))
            .collect();
        assert_eq!(
            found,
            [
                ("7-Zip 23.01 (x64)", "23.01", Some("Igor Pavlov")),
                (
                    "Microsoft Visual C++ 2015-2022 Redistributable (x64)",
                    "14.38.33135",
                    Some("Microsoft Corporation")
                ),
                ("Dernier programme", "2.1.0", None),
            ]
        );
    }

    fn current_version(product: &str, build: &str) -> String {
        format!(
            "\nHKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\n\
    ProductName    REG_SZ    {product}\n\
    ReleaseId    REG_SZ    2009\n\
    DisplayVersion    REG_SZ    23H2\n\
    CurrentBuild    REG_SZ    {build}\n\
    CurrentBuildNumber    REG_SZ    {build}\n\
    CurrentMajorVersionNumber    REG_DWORD    0xa\n\
    CurrentMinorVersionNumber    REG_DWORD    0x0\n\
    UBR    REG_DWORD    0x10fd\n"
        )
    }

    #[test]
    fn windows_server_version_is_read_as_is() {
        let text = current_version("Windows Server 2022 Standard", "20348");
        let version = parse_windows_version(&text);
        assert_eq!(version.product.as_deref(), Some("Windows Server 2022 Standard"));
        assert_eq!(version.release.as_deref(), Some("23H2"));
        assert_eq!(version.build.as_deref(), Some("10.0.20348.4349"));
    }

    #[test]
    fn windows_11_is_not_reported_as_windows_10() {
        let version = parse_windows_version(&current_version("Windows 10 Pro", "22631"));
        assert_eq!(version.product.as_deref(), Some("Windows 11 Pro"));
        assert_eq!(version.build.as_deref(), Some("10.0.22631.4349"));

        let ten = parse_windows_version(&current_version("Windows 10 Pro", "19045"));
        assert_eq!(ten.product.as_deref(), Some("Windows 10 Pro"), "build < 22000 : inchangé");
    }

    #[test]
    fn windows_version_falls_back_to_release_id_and_legacy_numbers() {
        let text = "HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\n\
    ProductName    REG_SZ    Windows Server 2012 R2 Standard\n\
    ReleaseId    REG_SZ    1809\n\
    CurrentVersion    REG_SZ    6.3\n\
    CurrentBuildNumber    REG_SZ    9600\n";
        let version = parse_windows_version(text);
        assert_eq!(version.release.as_deref(), Some("1809"));
        assert_eq!(version.build.as_deref(), Some("6.3.9600"), "pas de UBR");
        assert_eq!(parse_windows_version("rien"), WindowsVersion::default());
    }

    #[test]
    fn only_kb_identifiers_are_kept_as_hotfixes() {
        let out = "KB5044285\r\nKB5043935\r\nKB5044285\r\n\r\nKB\r\nKB12a\r\nHotFixID\r\nkb123\r\n";
        assert_eq!(parse_hotfixes(out), ["KB5043935", "KB5044285"]);
    }

    #[test]
    fn packages_are_sorted_deduplicated_and_truncated() {
        let sorted = finish_packages(vec![
            package("zlib", "1.3"),
            package("bash", "5.2"),
            package("bash", "5.2"),
            package("bash", "5.1"),
        ]);
        let names: Vec<(&str, &str)> =
            sorted.iter().map(|p| (p.name.as_str(), p.version.as_str())).collect();
        assert_eq!(names, [("bash", "5.1"), ("bash", "5.2"), ("zlib", "1.3")]);

        let many: Vec<InstalledPackage> =
            (0..MAX_INVENTORY_PACKAGES + 50).map(|i| package(&format!("p{i:06}"), "1")).collect();
        assert_eq!(finish_packages(many).len(), MAX_INVENTORY_PACKAGES);
    }
}
