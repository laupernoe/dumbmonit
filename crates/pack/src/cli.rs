//! `dumbmonit pack lint` et `dumbmonit pack test`, pour écrire un paquet sans
//! serveur : la vérification et l'extraction sont celles de l'installation.

use std::path::Path;

use crate::Pack;
use crate::fixture::{self, EXPECTED};

const USAGE: &str = "usage:
  dumbmonit pack lint <dir|file>...     check packs without installing them
  dumbmonit pack test <dir> [--update]  replay fixtures/ and compare with expected.prom";

/// Point d'entrée ; `args` suit `pack`. Rend le code de sortie du processus.
pub fn run(args: &[String]) -> i32 {
    match args.first().map(String::as_str) {
        Some("lint") if args.len() > 1 => {
            let mut failed = false;
            for path in &args[1..] {
                failed |= lint(Path::new(path)).is_none();
            }
            i32::from(failed)
        }
        Some("test") if args.len() > 1 => {
            let update = args.iter().any(|arg| arg == "--update");
            let dirs: Vec<&String> = args[1..].iter().filter(|arg| *arg != "--update").collect();
            let mut failed = dirs.is_empty();
            for dir in dirs {
                failed |= !test(Path::new(dir), update);
            }
            i32::from(failed)
        }
        _ => {
            eprintln!("{USAGE}");
            2
        }
    }
}

/// Vérifie un paquet et dit ce qu'il contient ; `None` s'il est refusé.
fn lint(path: &Path) -> Option<(Pack, std::path::PathBuf)> {
    let (dir, yaml) = match fixture::load(path) {
        Ok(found) => found,
        Err(error) => {
            eprintln!("error: {error}");
            return None;
        }
    };
    match Pack::parse(&yaml) {
        Ok(pack) => {
            println!(
                "ok: {} {} ({}, {} metrics, {} rules{})",
                pack.id(),
                pack.version(),
                pack.kind(),
                pack.metric_names().count(),
                pack.rules().len(),
                match pack.snmp_profiles().len() {
                    0 => String::new(),
                    n => format!(", {n} SNMP profiles"),
                }
            );
            for warning in pack.warnings() {
                println!("  warning: {warning}");
            }
            Some((pack, dir))
        }
        Err(error) => {
            eprintln!("error: {} is not a valid pack:", path.display());
            for line in &error.errors {
                eprintln!("  - {line}");
            }
            None
        }
    }
}

fn test(path: &Path, update: bool) -> bool {
    let Some((pack, dir)) = lint(path) else { return false };
    let replay = match fixture::replay(&pack, &dir) {
        Ok(replay) => replay,
        Err(error) => {
            eprintln!("error: {error}");
            return false;
        }
    };
    for note in &replay.notes {
        println!("  note: {note}");
    }
    let expected_path = dir.join(EXPECTED);
    if update {
        return match std::fs::write(&expected_path, &replay.rendered) {
            Ok(()) => {
                println!("  wrote {}", expected_path.display());
                true
            }
            Err(error) => {
                eprintln!("error: {}: {error}", expected_path.display());
                false
            }
        };
    }
    let expected = match std::fs::read_to_string(&expected_path) {
        Ok(expected) => expected,
        Err(_) => {
            eprintln!(
                "error: {} is missing: run `dumbmonit pack test {} --update` once",
                expected_path.display(),
                path.display()
            );
            return false;
        }
    };
    let differences = fixture::diff(&expected, &replay.rendered);
    if differences.is_empty() {
        println!("  fixtures match {EXPECTED} ({} lines)", replay.rendered.lines().count());
        true
    } else {
        eprintln!("error: the extraction differs from {}:", expected_path.display());
        for line in differences {
            eprintln!("  {line}");
        }
        false
    }
}
