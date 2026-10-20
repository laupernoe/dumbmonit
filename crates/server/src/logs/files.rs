//! Fichiers de journaux sur disque : un par jour (UTC), compressés une fois le
//! jour passé, effacés au-delà de la rétention ou du plafond de taille.
//!
//! `dumbmonit-2026-10-20.log` est le fichier du jour, en clair (JSON, une ligne
//! par enregistrement) ; les jours précédents deviennent
//! `dumbmonit-2026-10-19.log.gz`. La compression passe par un fichier
//! temporaire renommé ensuite : un arrêt brutal laisse soit l'original, soit
//! l'archive complète, jamais une archive tronquée.

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use chrono::{NaiveDate, Utc};
use flate2::Compression;
use flate2::write::GzEncoder;

use super::{Limits, Record};

const PREFIX: &str = "dumbmonit-";
const PLAIN: &str = ".log";
const GZ: &str = ".log.gz";

/// Intervalle entre deux passages de ménage (rétention, plafond).
const MAINTENANCE_EVERY: Duration = Duration::from_secs(3600);

/// Un fichier de journal présent sur disque.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogFile {
    pub day: NaiveDate,
    pub path: PathBuf,
    pub bytes: u64,
    pub compressed: bool,
}

impl LogFile {
    /// Nom du fichier, sans le répertoire.
    pub fn name(&self) -> String {
        self.path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default()
    }
}

pub fn path_for(dir: &Path, day: NaiveDate, compressed: bool) -> PathBuf {
    dir.join(format!("{PREFIX}{}{}", day.format("%Y-%m-%d"), if compressed { GZ } else { PLAIN }))
}

fn parse_name(name: &str) -> Option<(NaiveDate, bool)> {
    let rest = name.strip_prefix(PREFIX)?;
    let (date, compressed) = if let Some(date) = rest.strip_suffix(GZ) {
        (date, true)
    } else {
        (rest.strip_suffix(PLAIN)?, false)
    };
    NaiveDate::parse_from_str(date, "%Y-%m-%d").ok().map(|day| (day, compressed))
}

/// Les fichiers du répertoire, du plus récent au plus ancien. Un répertoire
/// absent n'est pas une erreur : il n'y a simplement rien.
pub fn list(dir: &Path) -> io::Result<Vec<LogFile>> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut files = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some((day, compressed)) = parse_name(&name.to_string_lossy()) else { continue };
        let Ok(meta) = entry.metadata() else { continue };
        if !meta.is_file() {
            continue;
        }
        files.push(LogFile { day, path: entry.path(), bytes: meta.len(), compressed });
    }
    files.sort_by(|a, b| b.day.cmp(&a.day).then(a.compressed.cmp(&b.compressed)));
    Ok(files)
}

/// Compresse un fichier en clair et efface l'original.
pub fn compress(path: &Path) -> io::Result<PathBuf> {
    let target = PathBuf::from(format!("{}.gz", path.display()));
    let temporary = PathBuf::from(format!("{}.gz.tmp", path.display()));
    {
        let mut input = File::open(path)?;
        let mut encoder = GzEncoder::new(File::create(&temporary)?, Compression::default());
        io::copy(&mut input, &mut encoder)?;
        encoder.finish()?.sync_all()?;
    }
    fs::rename(&temporary, &target)?;
    fs::remove_file(path)?;
    Ok(target)
}

/// Compresse les fichiers en clair des jours antérieurs à `today` — ceux
/// qu'un arrêt a laissés là. Si l'archive existe déjà (arrêt entre le
/// renommage et l'effacement), l'original est seulement effacé.
pub fn compress_stale(dir: &Path, today: NaiveDate) -> io::Result<()> {
    for file in list(dir)? {
        if file.compressed || file.day >= today {
            continue;
        }
        if path_for(dir, file.day, true).exists() {
            fs::remove_file(&file.path)?;
        } else {
            compress(&file.path)?;
        }
    }
    Ok(())
}

/// Applique la rétention puis le plafond de taille ; rend les fichiers effacés.
///
/// La rétention garde `retention_days` jours, aujourd'hui compris. Le plafond
/// efface ensuite les plus anciens tant que le total le dépasse — sans jamais
/// toucher au fichier du jour, qui est celui qu'on écrit.
pub fn enforce(
    dir: &Path,
    today: NaiveDate,
    retention_days: u32,
    max_total_bytes: u64,
) -> io::Result<Vec<PathBuf>> {
    let mut removed = Vec::new();
    let oldest_kept = today - chrono::Days::new(u64::from(retention_days.max(1)) - 1);
    let mut kept = Vec::new();
    for file in list(dir)? {
        if file.day < oldest_kept {
            fs::remove_file(&file.path)?;
            removed.push(file.path);
        } else {
            kept.push(file);
        }
    }
    let mut total: u64 = kept.iter().map(|file| file.bytes).sum();
    // `kept` va du plus récent au plus ancien : on efface par la fin.
    while total > max_total_bytes {
        let Some(oldest) = kept.pop() else { break };
        if oldest.day >= today {
            break;
        }
        fs::remove_file(&oldest.path)?;
        total -= oldest.bytes;
        removed.push(oldest.path);
    }
    Ok(removed)
}

/// L'écrivain disque, qui tourne sur son propre fil.
pub struct DiskWriter {
    dir: PathBuf,
    limits: Arc<Limits>,
    dropped: Arc<AtomicU64>,
    day: Option<NaiveDate>,
    file: Option<BufWriter<File>>,
    bytes: u64,
    last_maintenance: Option<Instant>,
}

impl DiskWriter {
    pub fn new(dir: PathBuf, limits: Arc<Limits>, dropped: Arc<AtomicU64>) -> Self {
        Self { dir, limits, dropped, day: None, file: None, bytes: 0, last_maintenance: None }
    }

    /// Boucle du fil : écrit ce qui arrive, vide le tampon après chaque salve,
    /// et fait le ménage une fois par heure.
    pub fn run(mut self, rx: Receiver<Arc<Record>>) {
        loop {
            match rx.recv_timeout(Duration::from_secs(30)) {
                Ok(record) => {
                    self.write(&record);
                    while let Ok(record) = rx.try_recv() {
                        self.write(&record);
                    }
                    self.flush();
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    self.flush();
                    return;
                }
            }
            self.maintain(Utc::now().date_naive(), false);
        }
    }

    /// Écrit un enregistrement dans le fichier de son jour.
    pub fn write(&mut self, record: &Record) {
        let day = record.ts.date_naive();
        if self.day != Some(day) {
            self.flush();
            self.file = None;
            self.day = Some(day);
            // Nouveau jour : la veille est compressée et le ménage fait tout de suite.
            self.maintain(day, true);
            match self.open(day) {
                Ok((file, bytes)) => {
                    self.file = Some(BufWriter::new(file));
                    self.bytes = bytes;
                }
                Err(error) => eprintln!("logs: cannot open the log file: {error}"),
            }
        }
        // Le fichier du jour seul ne dépasse jamais le plafond total : au-delà,
        // le reste de la journée est perdu (et compté) plutôt que le disque rempli.
        let line = record.to_line();
        let Some(file) = self.file.as_mut() else {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            return;
        };
        if self.bytes + line.len() as u64 + 1 > self.limits.max_total_bytes() {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            return;
        }
        if file.write_all(line.as_bytes()).and_then(|()| file.write_all(b"\n")).is_err() {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            return;
        }
        self.bytes += line.len() as u64 + 1;
    }

    pub fn flush(&mut self) {
        if let Some(file) = self.file.as_mut() {
            let _ = file.flush();
        }
    }

    fn open(&self, day: NaiveDate) -> io::Result<(File, u64)> {
        fs::create_dir_all(&self.dir)?;
        let file =
            OpenOptions::new().create(true).append(true).open(path_for(&self.dir, day, false))?;
        let bytes = file.metadata()?.len();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = file.set_permissions(fs::Permissions::from_mode(0o600));
        }
        Ok((file, bytes))
    }

    /// Compression des jours passés, rétention et plafond.
    pub fn maintain(&mut self, today: NaiveDate, force: bool) {
        if !force && self.last_maintenance.is_some_and(|at| at.elapsed() < MAINTENANCE_EVERY) {
            return;
        }
        self.last_maintenance = Some(Instant::now());
        // Le jour courant est celui de l'écriture : un enregistrement arrivé
        // juste avant minuit ne fait pas compresser le fichier qu'on tient.
        let today = self.day.map_or(today, |day| day.max(today));
        if let Err(error) = compress_stale(&self.dir, today) {
            eprintln!("logs: compression failed: {error}");
        }
        if let Err(error) =
            enforce(&self.dir, today, self.limits.retention_days(), self.limits.max_total_bytes())
        {
            eprintln!("logs: retention failed: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logs::Level;
    use chrono::TimeZone;
    use std::io::Read;

    fn day(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
    }

    fn record_on(date: &str, message: &str) -> Record {
        let mut record = Record::new(Level::Info, message);
        let day = day(date);
        record.ts = Utc.from_utc_datetime(&day.and_hms_opt(12, 0, 0).unwrap());
        record
    }

    #[test]
    fn rotation_quotidienne_et_compression_de_la_veille() {
        let dir = tempfile::tempdir().unwrap();
        let mut writer = DiskWriter::new(
            dir.path().to_path_buf(),
            Arc::new(Limits::default()),
            Arc::new(AtomicU64::new(0)),
        );
        writer.write(&record_on("2026-10-18", "premier"));
        writer.write(&record_on("2026-10-19", "deuxième"));
        writer.write(&record_on("2026-10-20", "troisième"));
        writer.flush();

        let files = list(dir.path()).unwrap();
        let names: Vec<String> = files.iter().map(LogFile::name).collect();
        assert_eq!(
            names,
            [
                "dumbmonit-2026-10-20.log",
                "dumbmonit-2026-10-19.log.gz",
                "dumbmonit-2026-10-18.log.gz"
            ]
        );
        let mut text = String::new();
        flate2::read::GzDecoder::new(File::open(&files[1].path).unwrap())
            .read_to_string(&mut text)
            .unwrap();
        assert!(text.contains("deuxième"));
        assert!(fs::read_to_string(&files[0].path).unwrap().contains("troisième"));
    }

    #[test]
    fn la_retention_efface_les_jours_trop_anciens() {
        let dir = tempfile::tempdir().unwrap();
        for date in ["2026-10-01", "2026-10-06", "2026-10-07", "2026-10-20"] {
            fs::write(path_for(dir.path(), day(date), date != "2026-10-20"), b"x").unwrap();
        }
        let removed = enforce(dir.path(), day("2026-10-20"), 14, u64::MAX).unwrap();
        assert_eq!(removed.len(), 2, "le 1er et le 6 sortent, le 7 reste : {removed:?}");
        let left: Vec<NaiveDate> = list(dir.path()).unwrap().into_iter().map(|f| f.day).collect();
        assert_eq!(left, [day("2026-10-20"), day("2026-10-07")]);
    }

    #[test]
    fn le_plafond_efface_les_plus_anciens_mais_jamais_le_jour_courant() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(path_for(dir.path(), day("2026-10-18"), true), vec![0; 400]).unwrap();
        fs::write(path_for(dir.path(), day("2026-10-19"), true), vec![0; 400]).unwrap();
        fs::write(path_for(dir.path(), day("2026-10-20"), false), vec![0; 900]).unwrap();
        enforce(dir.path(), day("2026-10-20"), 30, 1000).unwrap();
        let left: Vec<NaiveDate> = list(dir.path()).unwrap().into_iter().map(|f| f.day).collect();
        assert_eq!(left, [day("2026-10-20")]);
    }

    #[test]
    fn au_dela_du_plafond_le_jour_courant_perd_et_compte() {
        let dir = tempfile::tempdir().unwrap();
        let dropped = Arc::new(AtomicU64::new(0));
        let limits = Arc::new(Limits::new(14, 16));
        limits.max_total_bytes.store(300, Ordering::Relaxed);
        let mut writer = DiskWriter::new(dir.path().to_path_buf(), limits, dropped.clone());
        for _ in 0..10 {
            writer.write(&record_on("2026-10-20", "une ligne de journal assez longue"));
        }
        writer.flush();
        assert!(dropped.load(Ordering::Relaxed) > 0);
        assert!(list(dir.path()).unwrap()[0].bytes <= 300);
    }

    #[test]
    fn un_fichier_oublie_en_clair_est_compresse_au_demarrage() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(path_for(dir.path(), day("2026-10-10"), false), b"{}\n").unwrap();
        compress_stale(dir.path(), day("2026-10-20")).unwrap();
        let files = list(dir.path()).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].compressed);
    }
}
