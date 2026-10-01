//! Rejeu des fixtures d'un paquet (`dumbmonit pack test`).
//!
//! Disposition d'un paquet dans un dépôt :
//!
//! ```text
//! shelly-plug/
//!   pack.yaml
//!   fixtures/status.json     une réponse par source, nommée d'après son id
//!   expected.prom            ce que l'extraction doit en tirer
//! ```
//!
//! `expected.prom` est au format d'exposition, trié, sans horodatage ni
//! étiquettes d'identité de la cible : il ne change que si l'extraction change.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use dumbmonit_proto::{MetricKind, Sample};

use crate::{METRIC_PREFIX, Pack};

/// Extensions reconnues pour une fixture, dans l'ordre de recherche.
pub const FIXTURE_EXTENSIONS: &[&str] = &["json", "txt", "prom"];

/// Nom du fichier de référence, à la racine du paquet.
pub const EXPECTED: &str = "expected.prom";

/// Rend des échantillons au format de `expected.prom`.
pub fn render(samples: &[Sample]) -> String {
    let mut families: BTreeMap<String, (MetricKind, Vec<String>)> = BTreeMap::new();
    for sample in samples {
        let name = format!("{METRIC_PREFIX}{}", sample.metric);
        let mut line = name.clone();
        if !sample.labels.is_empty() {
            line.push('{');
            for (i, (key, value)) in sample.labels.iter().enumerate() {
                if i > 0 {
                    line.push(',');
                }
                let _ = write!(line, "{key}=\"{}\"", escape(value));
            }
            line.push('}');
        }
        let _ = write!(line, " {}", sample.value);
        families.entry(name).or_insert_with(|| (sample.kind, Vec::new())).1.push(line);
    }
    let mut out = String::new();
    for (name, (kind, mut lines)) in families {
        lines.sort();
        let kind = match kind {
            MetricKind::Gauge => "gauge",
            MetricKind::Counter => "counter",
        };
        let _ = writeln!(out, "# TYPE {name} {kind}");
        for line in lines {
            out.push_str(&line);
            out.push('\n');
        }
    }
    out
}

fn escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
}

/// Le YAML d'un paquet : `path` est le fichier lui-même ou le répertoire qui
/// contient `pack.yaml`. Rend aussi le répertoire du paquet.
pub fn load(path: &Path) -> Result<(PathBuf, String), String> {
    let file = if path.is_dir() {
        ["pack.yaml", "pack.yml"]
            .iter()
            .map(|name| path.join(name))
            .find(|candidate| candidate.is_file())
            .ok_or_else(|| format!("{}: no pack.yaml in this directory", path.display()))?
    } else {
        path.to_path_buf()
    };
    let yaml =
        std::fs::read_to_string(&file).map_err(|error| format!("{}: {error}", file.display()))?;
    let dir = file.parent().map(Path::to_path_buf).unwrap_or_default();
    Ok((dir, yaml))
}

/// Résultat du rejeu des fixtures.
#[derive(Debug)]
pub struct Replay {
    /// Ce que l'extraction a produit, au format de `expected.prom`.
    pub rendered: String,
    /// Sources sans fixture, fixtures sans source.
    pub notes: Vec<String>,
}

/// Rejoue `dir/fixtures/<source>.{json,txt,prom}` à travers l'extraction.
pub fn replay(pack: &Pack, dir: &Path) -> Result<Replay, String> {
    let fixtures = dir.join("fixtures");
    let mut bodies = BTreeMap::new();
    let mut notes = Vec::new();
    for id in pack.source_ids() {
        let found = FIXTURE_EXTENSIONS
            .iter()
            .map(|extension| fixtures.join(format!("{id}.{extension}")))
            .find(|candidate| candidate.is_file());
        match found {
            Some(file) => {
                let bytes =
                    std::fs::read(&file).map_err(|error| format!("{}: {error}", file.display()))?;
                bodies.insert(id.to_string(), bytes);
            }
            None => notes.push(format!("sources[{id}]: no fixture in {}", fixtures.display())),
        }
    }
    if let Ok(entries) = std::fs::read_dir(&fixtures) {
        for entry in entries.flatten() {
            let path = entry.path();
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
            if !pack.source_ids().any(|id| id == stem) {
                notes.push(format!("{}: no source with this id", path.display()));
            }
        }
    }
    let samples = pack.extract(&bodies, 0)?;
    Ok(Replay { rendered: render(&samples), notes })
}

/// Lignes présentes d'un seul côté, préfixées de `-` (attendues) ou `+` (obtenues).
pub fn diff(expected: &str, actual: &str) -> Vec<String> {
    let expected_lines: std::collections::BTreeSet<&str> = expected.lines().collect();
    let actual_lines: std::collections::BTreeSet<&str> = actual.lines().collect();
    let mut out: Vec<String> =
        expected_lines.difference(&actual_lines).map(|line| format!("- {line}")).collect();
    out.extend(actual_lines.difference(&expected_lines).map(|line| format!("+ {line}")));
    out
}
