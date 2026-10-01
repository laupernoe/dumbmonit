//! Journal dans un fichier, pour le service Windows.
//!
//! Un service Windows n'a ni console ni sortie d'erreur : ce que l'agent écrit
//! sur `stderr` y est perdu, à commencer par la raison d'un arrêt. Il écrit donc
//! dans `agent.log`, à côté de sa configuration.
//!
//! L'agent note chaque envoi, soit quelques centaines de kilo-octets par jour :
//! sans limite, le fichier finirait par peser plus que tout le reste. Passé
//! [`MAX_BYTES`], il devient `agent.log.old` (qui remplace le précédent) et un
//! nouveau commence — deux fichiers au plus, une vingtaine de mégaoctets en tout.
//!
//! Compilé sous Windows, et partout pour les tests : rien ici n'est propre à
//! Windows, et c'est sous Linux que tournent les tests de la chaîne d'intégration.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Nom du journal, à côté du fichier de configuration.
pub const FILE_NAME: &str = "agent.log";

/// Taille à partir de laquelle le journal est mis de côté.
const MAX_BYTES: u64 = 10 * 1024 * 1024;

/// Fichier journal à taille bornée. S'emploie derrière un `Mutex`, qui en fait
/// une destination pour `tracing_subscriber`.
pub struct LogFile {
    path: PathBuf,
    file: File,
    len: u64,
    max_bytes: u64,
}

impl LogFile {
    /// Ouvre (ou crée) `agent.log` dans le répertoire de `config_path`.
    pub fn beside(config_path: &Path) -> io::Result<Self> {
        let path = match config_path.parent() {
            Some(dir) if !dir.as_os_str().is_empty() => dir.join(FILE_NAME),
            _ => PathBuf::from(FILE_NAME),
        };
        Self::open(path, MAX_BYTES)
    }

    fn open(path: PathBuf, max_bytes: u64) -> io::Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        let len = file.metadata()?.len();
        Ok(Self { path, file, len, max_bytes })
    }

    fn previous(&self) -> PathBuf {
        let mut name = self.path.clone().into_os_string();
        name.push(".old");
        PathBuf::from(name)
    }

    /// Met le journal plein de côté et en commence un neuf.
    ///
    /// Si le renommage échoue — `agent.log.old` ouvert par quelqu'un qui le lit,
    /// par exemple —, le journal est vidé sur place : perdre l'historique vaut
    /// mieux que de laisser le fichier grossir sans fin.
    fn rotate(&mut self) -> io::Result<()> {
        if std::fs::rename(&self.path, self.previous()).is_ok() {
            self.file = OpenOptions::new().create(true).append(true).open(&self.path)?;
        } else {
            self.file.set_len(0)?;
        }
        self.len = 0;
        Ok(())
    }
}

impl Write for LogFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.len > 0 && self.len + buf.len() as u64 > self.max_bytes {
            self.rotate()?;
        }
        let written = self.file.write(buf)?;
        self.len += written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_log_sits_next_to_the_configuration() {
        let dir = tempfile::tempdir().unwrap();
        let mut log = LogFile::beside(&dir.path().join("agent.yaml")).unwrap();
        writeln!(log, "agent started").unwrap();
        let text = std::fs::read_to_string(dir.path().join(FILE_NAME)).unwrap();
        assert_eq!(text, "agent started\n");
    }

    #[test]
    fn a_restart_appends_rather_than_truncates() {
        // La raison d'un arrêt est la ligne qu'on vient chercher : le service qui
        // redémarre derrière ne doit pas l'effacer.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        writeln!(LogFile::open(path.clone(), 1024).unwrap(), "stopped on an error").unwrap();
        writeln!(LogFile::open(path.clone(), 1024).unwrap(), "agent started").unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text, "stopped on an error\nagent started\n");
    }

    #[test]
    fn a_full_log_is_set_aside_and_a_new_one_begins() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        let mut log = LogFile::open(path.clone(), 32).unwrap();
        log.write_all(b"0123456789012345678901234\n").unwrap();
        log.write_all(b"this line does not fit\n").unwrap();

        let current = std::fs::read_to_string(&path).unwrap();
        let previous = std::fs::read_to_string(dir.path().join("agent.log.old")).unwrap();
        assert_eq!(current, "this line does not fit\n");
        assert_eq!(previous, "0123456789012345678901234\n");

        // Une seconde rotation remplace l'ancien journal : jamais plus de deux.
        log.write_all(b"and neither does this one\n").unwrap();
        let previous = std::fs::read_to_string(dir.path().join("agent.log.old")).unwrap();
        assert_eq!(previous, "this line does not fit\n");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
    }

    #[test]
    fn a_line_longer_than_the_limit_is_still_written() {
        // Un fichier vide ne se met pas de côté : sinon une ligne trop longue
        // ferait tourner le journal à vide, sans jamais s'écrire.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        let mut log = LogFile::open(path.clone(), 4).unwrap();
        log.write_all(b"longer than four bytes\n").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "longer than four bytes\n");
        assert!(!dir.path().join("agent.log.old").exists());
    }
}
