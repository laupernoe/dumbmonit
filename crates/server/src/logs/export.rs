//! Export des journaux : une archive `.tar.gz` des N derniers jours.
//!
//! Les jours passés sont déjà compressés : ils entrent tels quels dans
//! l'archive (`.log.gz`), et la compression extérieure, au niveau le plus
//! rapide, ne fait presque que les recopier. Seuls le fichier du jour et
//! l'extrait du journal d'audit, en clair, sont réellement compressés. C'est ce
//! qui rend l'estimation fiable sans tout produire : la taille des archives
//! existantes, plus la compression effective des parties en clair.
//!
//! Le tar est écrit à la main (en-têtes ustar) : le format tient en quelques
//! lignes, et l'archive se produit ainsi en flux, sans fichier temporaire.

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

use chrono::{NaiveDate, Utc};
use flate2::Compression;
use flate2::write::GzEncoder;

use super::files::{self, LogFile};

/// Taille d'un bloc tar.
const BLOCK: usize = 512;

/// Les jours couverts par un export de `days` jours se terminant aujourd'hui.
pub fn period(today: NaiveDate, days: u32) -> (NaiveDate, NaiveDate) {
    (today - chrono::Days::new(u64::from(days.max(1)) - 1), today)
}

/// Fichiers concernés par l'export, du plus ancien au plus récent.
pub fn select(dir: &Path, today: NaiveDate, days: u32) -> io::Result<Vec<LogFile>> {
    let (from, to) = period(today, days);
    let mut chosen: Vec<LogFile> =
        files::list(dir)?.into_iter().filter(|file| file.day >= from && file.day <= to).collect();
    chosen.reverse();
    Ok(chosen)
}

/// Une pièce de l'archive.
pub enum Entry {
    /// Un fichier du disque, lu sur la longueur relevée à la sélection : le
    /// fichier du jour continue de grandir pendant l'export.
    File { name: String, path: std::path::PathBuf, len: u64, compressed: bool },
    /// Un contenu produit en mémoire (l'extrait du journal d'audit).
    Memory { name: String, data: Vec<u8> },
}

impl Entry {
    pub fn from_log(file: &LogFile) -> Self {
        Entry::File {
            name: file.name(),
            path: file.path.clone(),
            len: file.bytes,
            compressed: file.compressed,
        }
    }
}

/// Compteur d'octets, puits d'une compression dont seule la taille compte.
#[derive(Default)]
struct Counter(u64);

impl Write for Counter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0 += buf.len() as u64;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn compressed_size(mut reader: impl Read) -> io::Result<u64> {
    let mut encoder = GzEncoder::new(Counter::default(), Compression::fast());
    io::copy(&mut reader, &mut encoder)?;
    Ok(encoder.finish()?.0)
}

/// Taille estimée de l'archive, en octets.
pub fn estimate(entries: &[Entry]) -> io::Result<u64> {
    // En-tête gzip, deux blocs de fin, et un en-tête par pièce : quelques
    // dizaines d'octets chacun une fois compressés.
    let mut total: u64 = 20 + 64;
    for entry in entries {
        total += 64;
        total += match entry {
            Entry::File { len, compressed: true, .. } => {
                // Données déjà compressées : la passe extérieure les range en
                // blocs stockés, cinq octets de plus tous les 64 Kio.
                len + len / 65_535 * 5 + 5
            }
            Entry::File { path, len, compressed: false, .. } => {
                compressed_size(File::open(path)?.take(*len))?
            }
            Entry::Memory { data, .. } => compressed_size(data.as_slice())?,
        };
    }
    Ok(total)
}

/// En-tête ustar d'un fichier ordinaire.
fn header(name: &str, size: u64, mtime: i64) -> [u8; BLOCK] {
    let mut block = [0u8; BLOCK];
    let name = name.as_bytes();
    let n = name.len().min(99);
    block[..n].copy_from_slice(&name[..n]);
    let octal = |block: &mut [u8; BLOCK], offset: usize, width: usize, value: u64| {
        let text = format!("{value:0w$o}", w = width - 1);
        block[offset..offset + width - 1].copy_from_slice(text.as_bytes());
    };
    octal(&mut block, 100, 8, 0o600);
    octal(&mut block, 108, 8, 0);
    octal(&mut block, 116, 8, 0);
    octal(&mut block, 124, 12, size);
    octal(&mut block, 136, 12, mtime.max(0) as u64);
    block[156] = b'0';
    block[257..263].copy_from_slice(b"ustar\0");
    block[263..265].copy_from_slice(b"00");
    block[265..274].copy_from_slice(b"dumbmonit");
    block[297..306].copy_from_slice(b"dumbmonit");
    // Somme de contrôle : calculée avec son propre champ rempli d'espaces.
    block[148..156].copy_from_slice(b"        ");
    let sum: u32 = block.iter().map(|&b| u32::from(b)).sum();
    let text = format!("{sum:06o}\0 ");
    block[148..156].copy_from_slice(text.as_bytes());
    block
}

/// Écrit l'archive complète dans `out`.
pub fn write_archive(entries: &[Entry], out: impl Write) -> io::Result<()> {
    let mut encoder = GzEncoder::new(out, Compression::fast());
    let now = Utc::now().timestamp();
    for entry in entries {
        match entry {
            Entry::File { name, path, len, .. } => {
                encoder.write_all(&header(name, *len, now))?;
                let copied = io::copy(&mut File::open(path)?.take(*len), &mut encoder)?;
                // Un fichier raccourci entre-temps (ménage) : complété de zéros,
                // pour que l'archive reste lisible.
                if copied < *len {
                    io::copy(&mut io::repeat(0).take(len - copied), &mut encoder)?;
                }
                pad(&mut encoder, *len)?;
            }
            Entry::Memory { name, data } => {
                encoder.write_all(&header(name, data.len() as u64, now))?;
                encoder.write_all(data)?;
                pad(&mut encoder, data.len() as u64)?;
            }
        }
    }
    encoder.write_all(&[0u8; BLOCK * 2])?;
    encoder.finish()?.flush()
}

fn pad(out: &mut impl Write, len: u64) -> io::Result<()> {
    let rest = (len % BLOCK as u64) as usize;
    if rest != 0 {
        out.write_all(&vec![0u8; BLOCK - rest])?;
    }
    Ok(())
}

/// Puits qui découpe l'archive en morceaux remis à un canal : le corps de la
/// réponse HTTP les lit à mesure. Un client parti ferme le canal, l'écriture
/// échoue et la production s'arrête.
pub struct ChannelWriter {
    tx: tokio::sync::mpsc::Sender<io::Result<axum::body::Bytes>>,
    buffer: Vec<u8>,
}

const CHUNK: usize = 64 * 1024;

impl ChannelWriter {
    pub fn new(tx: tokio::sync::mpsc::Sender<io::Result<axum::body::Bytes>>) -> Self {
        Self { tx, buffer: Vec::with_capacity(CHUNK) }
    }

    fn send(&mut self) -> io::Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        let chunk =
            axum::body::Bytes::from(std::mem::replace(&mut self.buffer, Vec::with_capacity(CHUNK)));
        self.tx
            .blocking_send(Ok(chunk))
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "client gone"))
    }
}

impl Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.extend_from_slice(buf);
        if self.buffer.len() >= CHUNK {
            self.send()?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.send()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn day(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
    }

    /// Relit une archive tar.gz : (nom, contenu) de chaque pièce.
    fn untar(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
        let mut raw = Vec::new();
        flate2::read::GzDecoder::new(bytes).read_to_end(&mut raw).unwrap();
        let mut out = Vec::new();
        let mut offset = 0;
        while offset + BLOCK <= raw.len() && raw[offset] != 0 {
            let head = &raw[offset..offset + BLOCK];
            let name = String::from_utf8_lossy(&head[..100]).trim_end_matches('\0').to_string();
            let size_text = String::from_utf8_lossy(&head[124..135]).to_string();
            let size = usize::from_str_radix(size_text.trim_matches('\0'), 8).unwrap();
            let stored: u32 =
                u32::from_str_radix(String::from_utf8_lossy(&head[148..154]).trim(), 8).unwrap();
            let mut copy = head.to_vec();
            copy[148..156].copy_from_slice(b"        ");
            assert_eq!(stored, copy.iter().map(|&b| u32::from(b)).sum::<u32>(), "somme de {name}");
            offset += BLOCK;
            out.push((name, raw[offset..offset + size].to_vec()));
            offset += size.div_ceil(BLOCK) * BLOCK;
        }
        out
    }

    fn sample_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let mut lines = String::new();
        for i in 0..2000 {
            lines.push_str(&format!(
                "{{\"ts\":\"2026-10-20T10:00:{:02}Z\",\"level\":\"INFO\",\"msg\":\"probe ok {i}\"}}\n",
                i % 60
            ));
        }
        fs::write(files::path_for(dir.path(), day("2026-10-20"), false), &lines).unwrap();
        for date in ["2026-10-19", "2026-10-12", "2026-10-01"] {
            let path = files::path_for(dir.path(), day(date), false);
            fs::write(&path, &lines).unwrap();
            files::compress(&path).unwrap();
        }
        dir
    }

    #[test]
    fn la_selection_suit_le_nombre_de_jours() {
        let dir = sample_dir();
        let today = day("2026-10-20");
        let names = |days| -> Vec<String> {
            select(dir.path(), today, days).unwrap().iter().map(LogFile::name).collect()
        };
        assert_eq!(names(1), ["dumbmonit-2026-10-20.log"]);
        assert_eq!(names(7), ["dumbmonit-2026-10-19.log.gz", "dumbmonit-2026-10-20.log"]);
        assert_eq!(names(14).len(), 3);
        assert_eq!(names(30).len(), 4);
    }

    #[test]
    fn l_estimation_est_proche_de_la_taille_reelle() {
        let dir = sample_dir();
        let mut entries: Vec<Entry> = select(dir.path(), day("2026-10-20"), 30)
            .unwrap()
            .iter()
            .map(Entry::from_log)
            .collect();
        entries.push(Entry::Memory { name: "audit.jsonl".into(), data: b"{\"a\":1}\n".to_vec() });
        let estimated = estimate(&entries).unwrap();
        let mut archive = Vec::new();
        write_archive(&entries, &mut archive).unwrap();
        let real = archive.len() as u64;
        let gap = estimated.abs_diff(real) as f64 / real as f64;
        assert!(gap < 0.10 || estimated.abs_diff(real) < 2048, "estimé {estimated}, réel {real}");

        let pieces = untar(&archive);
        assert_eq!(pieces.len(), 5);
        assert_eq!(pieces[4].0, "audit.jsonl");
        let today = &pieces[3];
        assert_eq!(today.0, "dumbmonit-2026-10-20.log");
        assert!(String::from_utf8_lossy(&today.1).contains("probe ok 1999"));
        // Un jour passé reste un gzip valide une fois extrait.
        let mut text = String::new();
        flate2::read::GzDecoder::new(pieces[0].1.as_slice()).read_to_string(&mut text).unwrap();
        assert!(text.contains("probe ok 0"));
    }
}
