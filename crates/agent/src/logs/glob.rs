//! Développement des jokers (`*`, `?`) d'un chemin, sans dépendance.
//!
//! Chaque composant du chemin peut en porter : `/var/log/*/error.log`,
//! `/var/log/nginx/*.log`. Pas de `**` : suivre des arborescences entières
//! coûterait cher à chaque tour de scrutation.

use std::path::{Component, Path, PathBuf};

/// Nombre maximal de fichiers suivis pour un même motif.
pub const MAX_FILES: usize = 64;

/// Nombre maximal de candidats gardés entre deux composants.
const MAX_CANDIDATES: usize = 512;

fn has_wildcard(name: &str) -> bool {
    name.contains('*') || name.contains('?')
}

/// Vrai si `text` correspond à `pattern` (`*` : toute suite, `?` : un caractère).
pub fn matches(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (mut pi, mut ti) = (0usize, 0usize);
    let mut star: Option<usize> = None;
    let mut mark = 0usize;
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

/// Les fichiers ordinaires que désigne un motif, triés.
pub fn expand(pattern: &str) -> Vec<PathBuf> {
    let mut bases: Vec<PathBuf> = vec![PathBuf::new()];
    for component in Path::new(pattern).components() {
        match component {
            Component::Normal(name) => {
                let name = name.to_string_lossy().to_string();
                if has_wildcard(&name) {
                    let mut next: Vec<PathBuf> = Vec::new();
                    for base in &bases {
                        let dir = if base.as_os_str().is_empty() {
                            PathBuf::from(".")
                        } else {
                            base.clone()
                        };
                        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
                        for entry in entries.flatten() {
                            let found = entry.file_name().to_string_lossy().to_string();
                            if matches(&name, &found) {
                                next.push(base.join(&found));
                                if next.len() >= MAX_CANDIDATES {
                                    break;
                                }
                            }
                        }
                    }
                    bases = next;
                } else {
                    for base in &mut bases {
                        base.push(&name);
                    }
                }
            }
            other => {
                for base in &mut bases {
                    base.push(other.as_os_str());
                }
            }
        }
    }
    let mut files: Vec<PathBuf> = bases.into_iter().filter(|path| path.is_file()).collect();
    files.sort();
    files.truncate(MAX_FILES);
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcards_match_like_a_shell() {
        assert!(matches("*.log", "error.log"));
        assert!(matches("app-?.log", "app-1.log"));
        assert!(!matches("app-?.log", "app-12.log"));
        assert!(matches("*", "anything"));
        assert!(matches("a*b*c", "aXXbYYc"));
        assert!(!matches("*.log", "error.log.1"));
        assert!(matches("error.log*", "error.log.1"));
    }

    #[test]
    fn a_pattern_expands_to_existing_files_only() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.log"), "x").unwrap();
        std::fs::write(dir.path().join("b.log"), "x").unwrap();
        std::fs::write(dir.path().join("c.txt"), "x").unwrap();
        std::fs::create_dir(dir.path().join("sub.log")).unwrap();
        let pattern = format!("{}/*.log", dir.path().display());
        let found = expand(&pattern);
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(found[0].ends_with("a.log"));
        let direct = expand(&format!("{}/c.txt", dir.path().display()));
        assert_eq!(direct.len(), 1);
        assert!(expand(&format!("{}/none*", dir.path().display())).is_empty());
    }

    #[test]
    fn a_wildcard_directory_is_followed() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("one")).unwrap();
        std::fs::create_dir(dir.path().join("two")).unwrap();
        std::fs::write(dir.path().join("one/error.log"), "x").unwrap();
        std::fs::write(dir.path().join("two/error.log"), "x").unwrap();
        let found = expand(&format!("{}/*/error.log", dir.path().display()));
        assert_eq!(found.len(), 2, "{found:?}");
    }
}
