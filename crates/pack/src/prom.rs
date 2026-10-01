//! Lecture du format d'exposition Prometheus (texte, version 0.0.4).
//!
//! Juste ce qu'il faut pour un exportateur : lignes `# TYPE`, échantillons avec
//! étiquettes échappées, valeurs spéciales (`NaN`, `+Inf`). Les horodatages
//! éventuels sont ignorés : la mesure est datée à la collecte, comme toutes les
//! autres. Une ligne illisible est une erreur de protocole, pas un saut
//! silencieux : un exportateur qui écrit mal écrit mal partout.

use std::collections::{BTreeMap, HashMap};

/// Un échantillon lu, avec le type de sa famille.
#[derive(Debug, Clone, PartialEq)]
pub struct PromSample {
    pub name: String,
    pub labels: BTreeMap<String, String>,
    pub value: f64,
    /// Famille déclarée par `# TYPE` dont relève l'échantillon, s'il y en a une.
    pub family: Option<String>,
    /// `counter`, `gauge`, `histogram`, `summary`, `untyped`.
    pub family_type: String,
}

/// Suffixes qu'un histogramme ou un résumé ajoute au nom de sa famille.
pub const FAMILY_SUFFIXES: &[&str] = &["_bucket", "_sum", "_count", "_total", "_created"];

pub fn parse(text: &str) -> Result<Vec<PromSample>, String> {
    let mut types: HashMap<String, String> = HashMap::new();
    let mut samples = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(comment) = line.strip_prefix('#') {
            let mut words = comment.split_whitespace();
            if words.next() == Some("TYPE")
                && let (Some(name), Some(kind)) = (words.next(), words.next())
            {
                types.insert(name.to_string(), kind.to_ascii_lowercase());
            }
            continue;
        }
        let sample = parse_sample(line).map_err(|error| format!("line {}: {error}", number + 1))?;
        let family = family_of(&sample.0, &types);
        let family_type = family
            .as_ref()
            .and_then(|family| types.get(family))
            .cloned()
            .unwrap_or_else(|| "untyped".to_string());
        samples.push(PromSample {
            name: sample.0,
            labels: sample.1,
            value: sample.2,
            family,
            family_type,
        });
    }
    Ok(samples)
}

fn family_of(name: &str, types: &HashMap<String, String>) -> Option<String> {
    if types.contains_key(name) {
        return Some(name.to_string());
    }
    FAMILY_SUFFIXES
        .iter()
        .filter_map(|suffix| name.strip_suffix(suffix))
        .find(|base| types.contains_key(*base))
        .map(str::to_string)
}

type Parsed = (String, BTreeMap<String, String>, f64);

fn parse_sample(line: &str) -> Result<Parsed, String> {
    let name_end = line.find(|c: char| c == '{' || c.is_whitespace()).unwrap_or(line.len());
    let name = &line[..name_end];
    if name.is_empty()
        || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':')
        || name.starts_with(|c: char| c.is_ascii_digit())
    {
        return Err(format!("invalid metric name \"{name}\""));
    }
    let mut rest = &line[name_end..];
    let mut labels = BTreeMap::new();
    if let Some(inner) = rest.strip_prefix('{') {
        let (parsed, after) = parse_labels(inner)?;
        labels = parsed;
        rest = after;
    }
    let mut words = rest.split_whitespace();
    let raw = words.next().ok_or_else(|| format!("no value for \"{name}\""))?;
    let value = parse_value(raw).ok_or_else(|| format!("invalid value \"{raw}\""))?;
    Ok((name.to_string(), labels, value))
}

fn parse_value(raw: &str) -> Option<f64> {
    match raw {
        "+Inf" | "Inf" => Some(f64::INFINITY),
        "-Inf" => Some(f64::NEG_INFINITY),
        "NaN" => Some(f64::NAN),
        _ => raw.parse().ok(),
    }
}

/// Lit `a="1",b="x\"y"}` et rend les étiquettes et ce qui suit l'accolade.
fn parse_labels(mut rest: &str) -> Result<(BTreeMap<String, String>, &str), String> {
    let mut labels = BTreeMap::new();
    loop {
        rest = rest.trim_start();
        if let Some(after) = rest.strip_prefix('}') {
            return Ok((labels, after));
        }
        let eq = rest.find('=').ok_or("label without \"=\"")?;
        let key = rest[..eq].trim();
        if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(format!("invalid label name \"{key}\""));
        }
        rest = rest[eq + 1..].trim_start();
        rest = rest.strip_prefix('"').ok_or("label value without quotes")?;
        let mut value = String::new();
        let mut chars = rest.char_indices();
        let end = loop {
            match chars.next() {
                None => return Err("unterminated label value".into()),
                Some((at, '"')) => break at,
                Some((_, '\\')) => match chars.next() {
                    Some((_, 'n')) => value.push('\n'),
                    Some((_, other)) => value.push(other),
                    None => return Err("unterminated label value".into()),
                },
                Some((_, other)) => value.push(other),
            }
        };
        labels.insert(key.to_string(), value);
        rest = rest[end + 1..].trim_start();
        if let Some(after) = rest.strip_prefix(',') {
            rest = after;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_familles_et_leurs_echantillons_sont_reconnus() {
        let text = "# HELP node_load1 1m load average.\n\
                    # TYPE node_load1 gauge\n\
                    node_load1 0.42\n\
                    # TYPE http_duration histogram\n\
                    http_duration_bucket{le=\"+Inf\"} 3\n\
                    http_duration_sum 1.5\n\
                    orphan{a=\"x\\\"y\",b=\"z\"} NaN 1700000000000\n";
        let samples = parse(text).unwrap();
        assert_eq!(samples.len(), 4);
        assert_eq!(samples[0].family.as_deref(), Some("node_load1"));
        assert_eq!(samples[0].family_type, "gauge");
        assert_eq!(samples[1].family.as_deref(), Some("http_duration"));
        assert_eq!(samples[1].family_type, "histogram");
        assert_eq!(samples[1].labels["le"], "+Inf");
        assert_eq!(samples[3].family, None);
        assert_eq!(samples[3].labels["a"], "x\"y");
        assert!(samples[3].value.is_nan());
    }

    #[test]
    fn une_ligne_illisible_est_signalee_avec_son_numero() {
        let error = parse("ok 1\nbad{a=1} 2\n").unwrap_err();
        assert!(error.starts_with("line 2:"), "{error}");
    }
}
