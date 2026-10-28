//! Causes d'une machine « lente » : processus les plus gourmands, pression noyau
//! (PSI), inodes, dérive d'horloge et OOM kills.
//!
//! Tout se lit dans `/proc` (un fichier par processus, quelques lignes ailleurs) :
//! coût négligeable, aucune dépendance lourde. Seuls deux relevés lancent un
//! programme (`chronyc`, `timedatectl`), bornés par un délai strict et espacés
//! d'au moins une minute.
//!
//! Confidentialité : seuls les **noms** de processus (le `comm` du noyau, 15
//! caractères au plus) sont remontés, jamais les lignes de commande, qui peuvent
//! contenir des mots de passe ou des jetons. Les processus de même nom sont
//! additionnés (cinquante `php-fpm` ne forment qu'une ligne), et la liste est
//! plafonnée à [`TOP_N`].
//!
//! Sous Windows, la même carte est nourrie autrement : `sysinfo` pour les
//! processus (mêmes séries `agent_process_*`), trois compteurs de performance
//! pour la pression (`agent_windows_*` : leur sens n'est pas celui du PSI, ils
//! portent donc d'autres noms) et `w32tm /query /status /verbose` pour
//! l'horloge (mêmes séries `agent_clock_*`). NTFS n'a pas de table d'inodes :
//! aucune série `filesystem_inodes_*`, l'interface masque la section.
//!
//! Ailleurs (macOS…), la sonde se tait : mieux vaut aucune série qu'un zéro
//! trompeur. Seuls les analyseurs purs sont compilés et testés partout.

#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

use std::collections::BTreeMap;

use dumbmonit_proto::{MetricKind, Sample};

use crate::collect::FilesystemStat;

/// Nombre de processus remontés, par classement (CPU puis mémoire).
pub const TOP_N: usize = 10;

/// Durée minimale entre deux relevés d'horloge : deux processus lancés au plus
/// par minute, quelle que soit la période de collecte.
#[cfg(target_os = "linux")]
const CLOCK_EVERY: std::time::Duration = std::time::Duration::from_secs(60);

/// Un groupe de processus de même nom.
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessUsage {
    pub name: String,
    /// Pourcentage d'un cœur (100 = un cœur plein, plus avec plusieurs threads).
    pub cpu_percent: f64,
    pub rss_bytes: u64,
}

/// Les plus gourmands, par classement.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProcessTop {
    /// Vide au tout premier cycle : le pourcentage est un delta.
    pub by_cpu: Vec<ProcessUsage>,
    pub by_memory: Vec<ProcessUsage>,
}

/// Pression noyau d'une ressource (`/proc/pressure/<resource>`), moyenne sur 10 s.
#[derive(Debug, Clone, PartialEq)]
pub struct PressureStat {
    /// `cpu`, `memory` ou `io`.
    pub resource: &'static str,
    /// Part du temps où au moins une tâche attendait cette ressource.
    pub some_percent: f64,
    /// Part du temps où toutes les tâches attendaient. `None` : ligne absente
    /// (le CPU n'a pas de `full` avant Linux 5.13).
    pub full_percent: Option<f64>,
}

/// Inodes d'un système de fichiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InodeStat {
    pub mount_point: String,
    pub device: String,
    pub fs_type: String,
    pub total: u64,
    pub free: u64,
}

/// Horloge système face à la référence de temps.
#[derive(Debug, Clone, PartialEq)]
pub struct ClockStat {
    /// Écart en secondes, positif si l'horloge locale est en avance. `None` :
    /// seul l'état de synchronisation est connu (`timedatectl`).
    pub offset_seconds: Option<f64>,
    pub synchronized: bool,
}

/// Charge approchée d'une machine Windows, lue dans les compteurs de performance.
/// Pas d'équivalent exact du PSI : ce sont des files d'attente, pas des parts de
/// temps. Chaque champ absent (compteur refusé) ne produit aucune série.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WindowsLoad {
    /// `\System\Processor Queue Length` : threads prêts qui attendent un cœur.
    pub processor_queue: Option<f64>,
    /// `\Memory\Pages/sec` : pages lues ou écrites sur disque faute de mémoire.
    pub pages_per_second: Option<f64>,
    /// `\PhysicalDisk(_Total)\Current Disk Queue Length`.
    pub disk_queue: Option<f64>,
}

impl WindowsLoad {
    pub fn is_empty(&self) -> bool {
        self.processor_queue.is_none()
            && self.pages_per_second.is_none()
            && self.disk_queue.is_none()
    }
}

/// Ce que la sonde a pu lire. Chaque champ absent ne produit aucune série.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MachineStat {
    pub processes: Option<ProcessTop>,
    pub pressure: Vec<PressureStat>,
    pub inodes: Vec<InodeStat>,
    pub clock: Option<ClockStat>,
    /// Compteur cumulé de processus tués par l'OOM killer depuis le démarrage.
    pub oom_kills: Option<u64>,
    /// Windows seulement.
    pub windows_load: Option<WindowsLoad>,
}

impl MachineStat {
    pub fn is_empty(&self) -> bool {
        self.processes.is_none()
            && self.pressure.is_empty()
            && self.inodes.is_empty()
            && self.clock.is_none()
            && self.oom_kills.is_none()
            && self.windows_load.as_ref().is_none_or(WindowsLoad::is_empty)
    }
}

/// Traduit la photographie en échantillons.
pub fn samples(stat: &MachineStat, now_ms: i64) -> Vec<Sample> {
    let gauge = |metric: &str, value: f64| Sample::new(metric, value, MetricKind::Gauge, now_ms);
    let mut samples = Vec::with_capacity(32);

    if let Some(top) = &stat.processes {
        for usage in top.by_cpu.iter().take(TOP_N) {
            samples.push(
                gauge("agent_process_cpu_percent", usage.cpu_percent)
                    .with_label("process", &usage.name),
            );
        }
        for usage in top.by_memory.iter().take(TOP_N) {
            samples.push(
                gauge("agent_process_memory_bytes", usage.rss_bytes as f64)
                    .with_label("process", &usage.name),
            );
        }
    }

    for pressure in &stat.pressure {
        samples.push(
            gauge("agent_pressure_some_percent", pressure.some_percent)
                .with_label("resource", pressure.resource),
        );
        if let Some(full) = pressure.full_percent {
            samples.push(
                gauge("agent_pressure_full_percent", full)
                    .with_label("resource", pressure.resource),
            );
        }
    }

    for inode in &stat.inodes {
        let used = inode.total.saturating_sub(inode.free);
        let percent = if inode.total == 0 { 0.0 } else { used as f64 * 100.0 / inode.total as f64 };
        for (metric, value) in [
            ("filesystem_inodes_total", inode.total as f64),
            ("filesystem_inodes_used_percent", percent),
        ] {
            samples.push(
                gauge(metric, value)
                    .with_label("mountpoint", &inode.mount_point)
                    .with_label("device", &inode.device)
                    .with_label("fstype", &inode.fs_type),
            );
        }
    }

    if let Some(clock) = &stat.clock {
        if let Some(offset) = clock.offset_seconds {
            samples.push(gauge("agent_clock_offset_seconds", offset));
        }
        samples.push(gauge("agent_clock_synchronized", f64::from(u8::from(clock.synchronized))));
    }

    if let Some(load) = &stat.windows_load {
        for (metric, value) in [
            ("agent_windows_processor_queue_length", load.processor_queue),
            ("agent_windows_memory_pages_per_second", load.pages_per_second),
            ("agent_windows_disk_queue_length", load.disk_queue),
        ] {
            if let Some(value) = value.filter(|v| v.is_finite() && *v >= 0.0) {
                samples.push(gauge(metric, value));
            }
        }
    }

    if let Some(kills) = stat.oom_kills {
        let kills = kills as f64;
        samples.push(Sample::new("agent_oom_kills_total", kills, MetricKind::Counter, now_ms));
    }
    samples
}

// ---------------------------------------------------------------------------
// Analyseurs purs
// ---------------------------------------------------------------------------

/// Champs utiles d'un `/proc/<pid>/stat`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcStat {
    pub name: String,
    /// `utime + stime`, en ticks d'horloge.
    pub cpu_ticks: u64,
    pub rss_pages: u64,
}

/// Analyse une ligne `pid (comm) state ppid ...`. Le nom peut contenir espaces et
/// parenthèses : on coupe donc sur la **dernière** parenthèse fermante.
pub fn parse_proc_stat(text: &str) -> Option<ProcStat> {
    let open = text.find('(')?;
    let close = text.rfind(')')?;
    if close < open {
        return None;
    }
    let name = text[open + 1..close].trim().to_string();
    // Après le nom : état (champ 3) en position 0, donc utime (14) en 11, stime
    // (15) en 12 et rss (24) en 21.
    let fields: Vec<&str> = text[close + 1..].split_whitespace().collect();
    let utime: u64 = fields.get(11)?.parse().ok()?;
    let stime: u64 = fields.get(12)?.parse().ok()?;
    let rss: i64 = fields.get(21)?.parse().ok()?;
    if name.is_empty() {
        return None;
    }
    Some(ProcStat { name, cpu_ticks: utime + stime, rss_pages: rss.max(0) as u64 })
}

/// Analyse `/proc/pressure/<resource>` : `some avg10=0.12 avg60=...` et `full ...`.
pub fn parse_pressure(resource: &'static str, text: &str) -> Option<PressureStat> {
    let avg10 = |kind: &str| -> Option<f64> {
        let line = text.lines().find(|line| line.split_whitespace().next() == Some(kind))?;
        let value = line.split_whitespace().find_map(|part| part.strip_prefix("avg10="))?;
        value.parse::<f64>().ok().filter(|v| v.is_finite())
    };
    Some(PressureStat { resource, some_percent: avg10("some")?, full_percent: avg10("full") })
}

/// Valeur de `oom_kill` dans `/proc/vmstat` (absente avant Linux 4.13).
pub fn parse_oom_kill(vmstat: &str) -> Option<u64> {
    vmstat.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        if parts.next() != Some("oom_kill") {
            return None;
        }
        parts.next()?.parse().ok()
    })
}

/// Agrège par nom, classe, et garde les [`TOP_N`] premiers de chaque classement.
/// Sans `with_cpu` (premier cycle, pas de delta), aucun classement CPU n'est publié.
pub fn rank(groups: BTreeMap<String, (f64, u64)>, with_cpu: bool) -> ProcessTop {
    let usages: Vec<ProcessUsage> = groups
        .into_iter()
        .map(|(name, (cpu_percent, rss_bytes))| ProcessUsage { name, cpu_percent, rss_bytes })
        .collect();

    let mut by_cpu: Vec<ProcessUsage> = if with_cpu {
        usages.iter().filter(|u| u.cpu_percent >= 0.1).cloned().collect()
    } else {
        Vec::new()
    };
    by_cpu
        .sort_by(|a, b| b.cpu_percent.total_cmp(&a.cpu_percent).then_with(|| a.name.cmp(&b.name)));
    by_cpu.truncate(TOP_N);

    let mut by_memory: Vec<ProcessUsage> = usages.into_iter().filter(|u| u.rss_bytes > 0).collect();
    by_memory.sort_by(|a, b| b.rss_bytes.cmp(&a.rss_bytes).then_with(|| a.name.cmp(&b.name)));
    by_memory.truncate(TOP_N);

    ProcessTop { by_cpu, by_memory }
}

/// `chronyc tracking` : écart de l'horloge système et état de synchronisation.
pub fn parse_chrony_tracking(text: &str) -> Option<ClockStat> {
    let mut offset = None;
    let mut synchronized = true;
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else { continue };
        let value = value.trim();
        match key.trim() {
            "System time" => {
                // `0.000004321 seconds slow of NTP time` : « slow » = en retard.
                let mut parts = value.split_whitespace();
                let seconds: f64 = parts.next()?.parse().ok()?;
                if !seconds.is_finite() {
                    return None;
                }
                let sign = if parts.nth(1) == Some("slow") { -1.0 } else { 1.0 };
                offset = Some(seconds * sign);
            }
            "Leap status" => synchronized = !value.eq_ignore_ascii_case("Not synchronised"),
            _ => {}
        }
    }
    offset.map(|offset| ClockStat { offset_seconds: Some(offset), synchronized })
}

/// `timedatectl show --property=NTPSynchronized` : `NTPSynchronized=yes`.
pub fn parse_timedatectl_sync(text: &str) -> Option<ClockStat> {
    let value = text.lines().find_map(|line| line.trim().strip_prefix("NTPSynchronized="))?;
    match value.trim() {
        "yes" => Some(ClockStat { offset_seconds: None, synchronized: true }),
        "no" => Some(ClockStat { offset_seconds: None, synchronized: false }),
        _ => None,
    }
}

/// Regroupe par nom des lignes `(nom, % d'un cœur, octets résidents)`, déjà lues
/// par la plateforme. Les noms vides sont ignorés ; on additionne les doublons.
#[cfg(any(windows, test))]
pub fn group_by_name(
    rows: impl IntoIterator<Item = (String, f64, u64)>,
) -> BTreeMap<String, (f64, u64)> {
    let mut groups: BTreeMap<String, (f64, u64)> = BTreeMap::new();
    for (name, cpu, rss) in rows {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        let cpu = if cpu.is_finite() && cpu > 0.0 { cpu } else { 0.0 };
        let group = groups.entry(name.to_string()).or_insert((0.0, 0));
        group.0 += cpu;
        group.1 = group.1.saturating_add(rss);
    }
    groups
}

/// Chemins PDH de la pression approchée, dans l'ordre de [`windows_load_from`].
#[cfg(any(windows, test))]
pub const WINDOWS_LOAD_COUNTERS: [&str; 3] = [
    r"\System\Processor Queue Length",
    r"\Memory\Pages/sec",
    r"\PhysicalDisk(_Total)\Current Disk Queue Length",
];

/// Assemble les trois lectures PDH (même ordre que [`WINDOWS_LOAD_COUNTERS`]).
/// Une valeur négative ou non finie est écartée plutôt que remontée.
#[cfg(any(windows, test))]
pub fn windows_load_from(
    readings: &[crate::collect::perf_counters::Reading],
) -> Option<WindowsLoad> {
    use crate::collect::perf_counters::Reading;
    let value = |index: usize| match readings.get(index) {
        Some(Reading::One(v)) if v.is_finite() && *v >= 0.0 => Some(*v),
        _ => None,
    };
    let load =
        WindowsLoad { processor_queue: value(0), pages_per_second: value(1), disk_queue: value(2) };
    (!load.is_empty()).then_some(load)
}

/// `w32tm /query /status /verbose`. Les libellés sont traduits selon la langue de
/// Windows : on s'appuie donc sur la structure (ordre des lignes, nombres) et
/// seulement en complément sur « Phase Offset ».
///
/// - ligne 1, `Leap Indicator: 3(not synchronized)` : 3 = non synchronisée ;
/// - ligne 2, `Stratum: 0 (unspecified)` : strate 0 ou 16 = pas de source ;
/// - `Phase Offset: -0.0012345s` : premier relevé du deuxième bloc (après la
///   ligne vide), en secondes. Convention NTP (source moins local), inversée ici
///   pour suivre celle de [`ClockStat`] : positif = horloge locale en avance.
#[cfg(any(windows, test))]
pub fn parse_w32tm_status(text: &str) -> Option<ClockStat> {
    let value_of = |line: &str| line.split_once(':').map(|(_, v)| v.trim().to_string());
    let leading_number = |value: &str| -> Option<i64> {
        let digits: String = value
            .trim_start()
            .chars()
            .enumerate()
            .take_while(|(i, c)| c.is_ascii_digit() || (*i == 0 && *c == '-'))
            .map(|(_, c)| c)
            .collect();
        digits.parse().ok()
    };

    let cleaned = text.replace('\r', "");
    let mut blocks = cleaned
        .split("\n\n")
        .map(|block| block.lines().filter(|l| !l.trim().is_empty()).collect::<Vec<_>>());
    let first = blocks.next()?;
    let leap = leading_number(&value_of(first.first()?)?)?;
    let stratum = first.get(1).and_then(|l| value_of(l)).and_then(|v| leading_number(&v));
    let synchronized = leap != 3 && !matches!(stratum, Some(0 | 16));

    let parse_seconds = |value: &str| -> Option<f64> {
        let number = value.trim().strip_suffix('s')?.trim().replace(',', ".");
        number.parse::<f64>().ok().filter(|v| v.is_finite())
    };
    let named = cleaned.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        key.trim().eq_ignore_ascii_case("Phase Offset").then(|| value.to_string())
    });
    let offset = named.as_deref().and_then(parse_seconds).or_else(|| {
        let line = blocks.next()?.first().copied()?;
        parse_seconds(&value_of(line)?)
    });

    Some(ClockStat { offset_seconds: offset.map(|v| -v), synchronized })
}

// ---------------------------------------------------------------------------
// Sonde (Linux)
// ---------------------------------------------------------------------------

pub struct MachineProbe {
    #[cfg(target_os = "linux")]
    previous: std::collections::HashMap<u32, u64>,
    #[cfg(target_os = "linux")]
    last_read: Option<std::time::Instant>,
    #[cfg(target_os = "linux")]
    clock: Option<(std::time::Instant, Option<ClockStat>)>,
    #[cfg(windows)]
    windows: windows_probe::State,
}

impl MachineProbe {
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "linux")]
            previous: std::collections::HashMap::new(),
            #[cfg(target_os = "linux")]
            last_read: None,
            #[cfg(target_os = "linux")]
            clock: None,
            #[cfg(windows)]
            windows: windows_probe::State::new(),
        }
    }

    #[cfg(windows)]
    pub async fn read(&mut self, _filesystems: &[FilesystemStat]) -> Option<MachineStat> {
        // Pas d'inodes sous NTFS : `inodes` reste vide.
        let stat = self.windows.read().await;
        (!stat.is_empty()).then_some(stat)
    }

    #[cfg(not(any(target_os = "linux", windows)))]
    pub async fn read(&mut self, _filesystems: &[FilesystemStat]) -> Option<MachineStat> {
        None
    }

    #[cfg(target_os = "linux")]
    pub async fn read(&mut self, filesystems: &[FilesystemStat]) -> Option<MachineStat> {
        let clock = self.read_clock().await;
        let stat = MachineStat {
            processes: self.read_processes(),
            pressure: read_pressure(),
            inodes: read_inodes(filesystems),
            clock,
            oom_kills: std::fs::read_to_string("/proc/vmstat")
                .ok()
                .and_then(|text| parse_oom_kill(&text)),
            windows_load: None,
        };
        (!stat.is_empty()).then_some(stat)
    }

    #[cfg(target_os = "linux")]
    fn read_processes(&mut self) -> Option<ProcessTop> {
        use std::time::Instant;

        // USER_HZ vaut 100 sur toutes les architectures Linux courantes.
        const TICKS_PER_SECOND: f64 = 100.0;

        let now = Instant::now();
        let elapsed = self.last_read.map(|at| now.duration_since(at).as_secs_f64());
        let page_size = page_size();
        let mut current = std::collections::HashMap::new();
        let mut groups: BTreeMap<String, (f64, u64)> = BTreeMap::new();

        for entry in std::fs::read_dir("/proc").ok()?.flatten() {
            let Some(pid) = entry.file_name().to_str().and_then(|n| n.parse::<u32>().ok()) else {
                continue;
            };
            // Un processus qui meurt pendant la lecture n'est pas une erreur.
            let Ok(text) = std::fs::read_to_string(entry.path().join("stat")) else { continue };
            let Some(proc) = parse_proc_stat(&text) else { continue };
            let cpu = match (elapsed, self.previous.get(&pid)) {
                (Some(seconds), Some(before)) if seconds > 0.0 => {
                    proc.cpu_ticks.saturating_sub(*before) as f64 / TICKS_PER_SECOND / seconds
                        * 100.0
                }
                _ => 0.0,
            };
            current.insert(pid, proc.cpu_ticks);
            let group = groups.entry(proc.name).or_insert((0.0, 0));
            group.0 += cpu;
            group.1 += proc.rss_pages.saturating_mul(page_size);
        }

        self.previous = current;
        self.last_read = Some(now);
        Some(rank(groups, elapsed.is_some()))
    }

    /// Relevé d'horloge, au plus une fois par [`CLOCK_EVERY`] ; le dernier
    /// résultat (même vide) est réutilisé entre-temps.
    #[cfg(target_os = "linux")]
    async fn read_clock(&mut self) -> Option<ClockStat> {
        if let Some((at, stat)) = &self.clock
            && at.elapsed() < CLOCK_EVERY
        {
            return stat.clone();
        }
        let stat = match run("chronyc", &["tracking"]).await.and_then(|t| parse_chrony_tracking(&t))
        {
            Some(stat) => Some(stat),
            None => run("timedatectl", &["show", "--property=NTPSynchronized"])
                .await
                .and_then(|t| parse_timedatectl_sync(&t)),
        };
        self.clock = Some((std::time::Instant::now(), stat.clone()));
        stat
    }
}

impl Default for MachineProbe {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "linux")]
fn page_size() -> u64 {
    // SAFETY : `sysconf` ne lit aucune mémoire de l'appelant.
    let size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if size > 0 { size as u64 } else { 4096 }
}

#[cfg(target_os = "linux")]
fn read_pressure() -> Vec<PressureStat> {
    ["cpu", "memory", "io"]
        .into_iter()
        .filter_map(|resource| {
            let text = std::fs::read_to_string(format!("/proc/pressure/{resource}")).ok()?;
            parse_pressure(resource, &text)
        })
        .collect()
}

/// Inodes des montages déjà retenus pour l'espace disque. Les systèmes de
/// fichiers sans table d'inodes fixe (btrfs, xfs dynamique : `f_files` à 0) sont
/// ignorés.
#[cfg(target_os = "linux")]
fn read_inodes(filesystems: &[FilesystemStat]) -> Vec<InodeStat> {
    use std::ffi::CString;

    filesystems
        .iter()
        .filter_map(|fs| {
            let path = CString::new(fs.mount_point.as_str()).ok()?;
            // SAFETY : structure initialisée à zéro, remplie par `statvfs` ; le
            // chemin est une chaîne C valide pour la durée de l'appel.
            let mut buf: libc::statvfs = unsafe { std::mem::zeroed() };
            if unsafe { libc::statvfs(path.as_ptr(), &mut buf) } != 0 {
                return None;
            }
            let total = buf.f_files as u64;
            (total > 0).then(|| InodeStat {
                mount_point: fs.mount_point.clone(),
                device: fs.device.clone(),
                fs_type: fs.fs_type.clone(),
                total,
                free: buf.f_ffree as u64,
            })
        })
        .collect()
}

/// Lance un programme sans interpréteur, avec un délai strict. `None` : absent,
/// en échec ou trop long.
#[cfg(target_os = "linux")]
async fn run(program: &str, args: &[&str]) -> Option<String> {
    let command = tokio::process::Command::new(program)
        .args(args)
        .env("LC_ALL", "C")
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .output();
    let output =
        tokio::time::timeout(std::time::Duration::from_secs(3), command).await.ok()?.ok()?;
    output.status.success().then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Sonde Windows : `sysinfo` (processus), PDH (files d'attente), `w32tm`
/// (horloge). Non compilée ailleurs ; ses analyseurs, eux, sont testés partout.
#[cfg(windows)]
mod windows_probe {
    use std::time::{Duration, Instant};

    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

    use super::{
        ClockStat, MachineStat, ProcessTop, WINDOWS_LOAD_COUNTERS, WindowsLoad, group_by_name,
        parse_w32tm_status, rank, windows_load_from,
    };
    use crate::collect::perf_counters::{CounterSource, PlatformSource};

    /// Pas de fenêtre de console pour le processus fils.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const CLOCK_EVERY: Duration = Duration::from_secs(60);
    const W32TM_TIMEOUT: Duration = Duration::from_secs(5);

    pub struct State {
        system: System,
        /// Vrai dès qu'un premier relevé a posé la base du calcul de CPU.
        primed: bool,
        pdh: PlatformSource,
        /// Les compteurs de pression ont-ils été ajoutés à la requête PDH ?
        pdh_open: bool,
        clock: Option<(Instant, Option<ClockStat>)>,
    }

    impl State {
        pub fn new() -> Self {
            Self {
                system: System::new(),
                primed: false,
                pdh: PlatformSource::new(),
                pdh_open: false,
                clock: None,
            }
        }

        pub async fn read(&mut self) -> MachineStat {
            let clock = self.read_clock().await;
            MachineStat {
                processes: Some(self.read_processes()),
                windows_load: self.read_load(),
                clock,
                ..MachineStat::default()
            }
        }

        /// `sysinfo` donne le pourcentage d'un cœur depuis le relevé précédent :
        /// le tout premier cycle n'a donc pas de classement CPU.
        fn read_processes(&mut self) -> ProcessTop {
            self.system.refresh_processes_specifics(
                ProcessesToUpdate::All,
                true,
                ProcessRefreshKind::nothing().with_cpu().with_memory(),
            );
            let rows = self
                .system
                .processes()
                .iter()
                // Le processus 0 (« System Idle Process ») compte le temps
                // inoccupé : il dominerait tout classement.
                .filter(|(pid, _)| pid.as_u32() != 0)
                .map(|(_, process)| {
                    (
                        process.name().to_string_lossy().into_owned(),
                        f64::from(process.cpu_usage()),
                        process.memory(),
                    )
                })
                .collect::<Vec<_>>();
            let with_cpu = self.primed;
            self.primed = true;
            rank(group_by_name(rows), with_cpu)
        }

        fn read_load(&mut self) -> Option<WindowsLoad> {
            if !self.pdh_open {
                let paths: Vec<String> =
                    WINDOWS_LOAD_COUNTERS.iter().map(|p| (*p).to_string()).collect();
                self.pdh.open(&paths);
                self.pdh_open = true;
            }
            windows_load_from(&self.pdh.read())
        }

        async fn read_clock(&mut self) -> Option<ClockStat> {
            if let Some((at, stat)) = &self.clock
                && at.elapsed() < CLOCK_EVERY
            {
                return stat.clone();
            }
            let mut command = tokio::process::Command::new("w32tm.exe");
            command
                .args(["/query", "/status", "/verbose"])
                .stdin(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .creation_flags(CREATE_NO_WINDOW)
                .kill_on_drop(true);
            let stat = match tokio::time::timeout(W32TM_TIMEOUT, command.output()).await {
                Ok(Ok(output)) if output.status.success() => {
                    parse_w32tm_status(&String::from_utf8_lossy(&output.stdout))
                }
                _ => None,
            };
            self.clock = Some((Instant::now(), stat.clone()));
            stat
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value_of(samples: &[Sample], metric: &str, label: Option<(&str, &str)>) -> Option<f64> {
        samples
            .iter()
            .find(|s| {
                s.metric == metric
                    && label.is_none_or(|(k, v)| s.labels.get(k).is_some_and(|x| x == v))
            })
            .map(|s| s.value)
    }

    fn usage(name: &str, cpu_percent: f64, rss_bytes: u64) -> ProcessUsage {
        ProcessUsage { name: name.into(), cpu_percent, rss_bytes }
    }

    #[test]
    fn a_stat_line_with_spaces_and_parentheses_in_the_name_is_parsed() {
        let line = "4242 (Web Content (x)) S 1 4242 4242 0 -1 4194560 100 0 0 0 150 50 0 0 20 0 \
                    9 0 123456 1000000 2500 18446744073709551615 0 0 0 0 0 0 0 0 0 0 0 0 17 1 \
                    0 0 0\n";
        let proc = parse_proc_stat(line).expect("ligne valide");
        assert_eq!(proc.name, "Web Content (x)");
        assert_eq!(proc.cpu_ticks, 200);
        assert_eq!(proc.rss_pages, 2500);
        assert_eq!(parse_proc_stat("garbage"), None);
        assert_eq!(parse_proc_stat("1 (a) S 1 2"), None);
    }

    #[test]
    fn pressure_lines_are_read_with_and_without_full() {
        let both = "some avg10=1.50 avg60=0.80 avg300=0.20 total=12345\n\
                    full avg10=0.25 avg60=0.10 avg300=0.00 total=999\n";
        let stat = parse_pressure("memory", both).expect("psi");
        assert_eq!((stat.some_percent, stat.full_percent), (1.5, Some(0.25)));
        let cpu_only = "some avg10=12.00 avg60=3.00 avg300=1.00 total=1\n";
        let stat = parse_pressure("cpu", cpu_only).expect("psi");
        assert_eq!((stat.some_percent, stat.full_percent), (12.0, None));
        assert_eq!(parse_pressure("io", ""), None);
    }

    #[test]
    fn the_oom_kill_counter_is_found_in_vmstat() {
        let vmstat = "nr_free_pages 123\noom_kill 7\npgfault 99\n";
        assert_eq!(parse_oom_kill(vmstat), Some(7));
        assert_eq!(parse_oom_kill("nr_free_pages 123\n"), None);
    }

    #[test]
    fn the_ranking_keeps_ten_names_and_hides_idle_ones() {
        let mut groups = BTreeMap::new();
        for i in 0..15u64 {
            groups.insert(format!("p{i:02}"), (i as f64, (i + 1) * 1000));
        }
        groups.insert("idle".to_string(), (0.0, 0));
        let top = rank(groups.clone(), true);
        assert_eq!(top.by_cpu.len(), TOP_N);
        assert_eq!(top.by_cpu[0].name, "p14");
        assert_eq!(top.by_memory.len(), TOP_N);
        assert!(top.by_memory.iter().all(|u| u.name != "idle"));
        // Premier cycle : pas de pourcentage, la mémoire reste publiée.
        let first = rank(groups, false);
        assert!(first.by_cpu.is_empty());
        assert_eq!(first.by_memory.len(), TOP_N);
    }

    #[test]
    fn chrony_tracking_gives_a_signed_offset() {
        let slow = "Reference ID    : A9FEA97B (169.254.169.123)\n\
                    Stratum         : 4\n\
                    System time     : 0.000004321 seconds slow of NTP time\n\
                    Last offset     : -0.000002 seconds\n\
                    Leap status     : Normal\n";
        let clock = parse_chrony_tracking(slow).expect("chrony");
        assert_eq!(clock.offset_seconds, Some(-0.000004321));
        assert!(clock.synchronized);
        let fast = slow.replace("slow", "fast");
        assert_eq!(parse_chrony_tracking(&fast).and_then(|c| c.offset_seconds), Some(0.000004321));
        let lost = slow.replace("Normal", "Not synchronised");
        assert!(!parse_chrony_tracking(&lost).expect("chrony").synchronized);
        assert_eq!(parse_chrony_tracking("nothing useful"), None);
    }

    #[test]
    fn timedatectl_only_tells_whether_the_clock_is_synchronized() {
        let yes = parse_timedatectl_sync("NTPSynchronized=yes\n").expect("yes");
        assert_eq!((yes.synchronized, yes.offset_seconds), (true, None));
        assert!(!parse_timedatectl_sync("NTPSynchronized=no\n").expect("no").synchronized);
        assert_eq!(parse_timedatectl_sync("Timezone=UTC\n"), None);
    }

    #[test]
    fn a_full_reading_becomes_labelled_series_without_command_lines() {
        let stat = MachineStat {
            processes: Some(ProcessTop {
                by_cpu: vec![usage("postgres", 80.0, 5)],
                by_memory: vec![usage("java", 1.0, 900)],
            }),
            pressure: vec![PressureStat {
                resource: "io",
                some_percent: 4.0,
                full_percent: Some(2.0),
            }],
            inodes: vec![InodeStat {
                mount_point: "/".into(),
                device: "/dev/sda1".into(),
                fs_type: "ext4".into(),
                total: 1000,
                free: 100,
            }],
            clock: Some(ClockStat { offset_seconds: Some(0.5), synchronized: false }),
            oom_kills: Some(3),
            windows_load: None,
        };
        let out = samples(&stat, 0);
        let by = |metric, key, val| value_of(&out, metric, Some((key, val)));
        assert_eq!(by("agent_process_cpu_percent", "process", "postgres"), Some(80.0));
        assert_eq!(by("agent_process_memory_bytes", "process", "java"), Some(900.0));
        assert_eq!(by("agent_pressure_some_percent", "resource", "io"), Some(4.0));
        assert_eq!(by("agent_pressure_full_percent", "resource", "io"), Some(2.0));
        assert_eq!(by("filesystem_inodes_used_percent", "mountpoint", "/"), Some(90.0));
        assert_eq!(value_of(&out, "agent_clock_offset_seconds", None), Some(0.5));
        assert_eq!(value_of(&out, "agent_clock_synchronized", None), Some(0.0));
        let oom = out.iter().find(|s| s.metric == "agent_oom_kills_total").expect("oom");
        assert_eq!((oom.value, oom.kind), (3.0, MetricKind::Counter));
        assert!(out.iter().all(|s| s.value.is_finite()));
        assert!(MachineStat::default().is_empty());
    }

    const W32TM_EN: &str = "Leap Indicator: 0(no warning)\r\nStratum: 3 (secondary reference - syncd by (S)NTP)\r\nPrecision: -23 (119.209ns per tick)\r\nRoot Delay: 0.0312500s\r\nRoot Dispersion: 7.7918701s\r\nReferenceId: 0x0A000001 (source IP:  10.0.0.1)\r\nLast Successful Sync Time: 28/10/2026 10:00:00\r\nSource: ntp.example.org,0x8\r\nPoll Interval: 10 (1024s)\r\n\r\nPhase Offset: 0.0012345s\r\nClockRate: 0.0156250s\r\nState Machine: 2 (Sync)\r\n";
    /// Windows en français : libellés traduits, virgule décimale.
    const W32TM_FR: &str = "Indicateur de saut: 0(aucun avertissement)\r\nStrate: 3 (référence secondaire - synchronisée par (S)NTP)\r\nPrécision: -23 (119.209ns par tick)\r\nDélai racine: 0,0312500s\r\nSource: ntp.example.org,0x8\r\nIntervalle d'interrogation: 10 (1024s)\r\n\r\nDécalage de phase: -0,0500000s\r\nFréquence d'horloge: 0,0156250s\r\n";
    const W32TM_UNSYNC: &str = "Leap Indicator: 3(not synchronized)\nStratum: 0 (unspecified)\nSource: Free-running System Clock\n\nPhase Offset: 0.0000000s\n";

    #[test]
    fn w32tm_english_gives_offset_with_the_local_ahead_convention() {
        let clock = parse_w32tm_status(W32TM_EN).expect("clock");
        assert!(clock.synchronized);
        // Phase Offset > 0 = source en avance : l'horloge locale est en retard.
        assert_eq!(clock.offset_seconds, Some(-0.0012345));
    }

    #[test]
    fn w32tm_french_is_read_by_structure() {
        let clock = parse_w32tm_status(W32TM_FR).expect("clock");
        assert!(clock.synchronized);
        assert_eq!(clock.offset_seconds, Some(0.05));
    }

    #[test]
    fn w32tm_unsynchronised_and_garbage() {
        let clock = parse_w32tm_status(W32TM_UNSYNC).expect("clock");
        assert!(!clock.synchronized);
        assert_eq!(parse_w32tm_status("The service has not been started. (0x80070426)"), None);
        assert_eq!(parse_w32tm_status(""), None);
    }

    #[test]
    fn processes_of_the_same_name_are_added_and_idle_names_dropped() {
        let groups = group_by_name([
            ("chrome.exe".to_string(), 10.0, 100),
            ("chrome.exe".to_string(), f64::NAN, 50),
            ("  ".to_string(), 99.0, 1),
            ("sqlservr.exe".to_string(), 40.0, 900),
        ]);
        assert_eq!(groups.len(), 2);
        let top = rank(groups, true);
        assert_eq!(top.by_cpu[0].name, "sqlservr.exe");
        assert_eq!(top.by_memory[0].rss_bytes, 900);
        assert_eq!(top.by_memory[1], usage("chrome.exe", 10.0, 150));
    }

    #[test]
    fn windows_load_keeps_valid_counters_only() {
        use crate::collect::perf_counters::Reading;
        let load = windows_load_from(&[Reading::One(3.0), Reading::Missing, Reading::One(-1.0)])
            .expect("load");
        assert_eq!(load.processor_queue, Some(3.0));
        assert_eq!((load.pages_per_second, load.disk_queue), (None, None));
        assert_eq!(
            windows_load_from(&[Reading::Missing, Reading::Missing, Reading::Missing]),
            None
        );
        let out = samples(&MachineStat { windows_load: Some(load), ..MachineStat::default() }, 0);
        assert_eq!(value_of(&out, "agent_windows_processor_queue_length", None), Some(3.0));
        assert_eq!(out.len(), 1);
        let empty =
            MachineStat { windows_load: Some(WindowsLoad::default()), ..Default::default() };
        assert!(empty.is_empty());
    }
}
