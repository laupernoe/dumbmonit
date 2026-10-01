//! Extraction des mesures d'une réponse, sans réseau.
//!
//! C'est le même code qui sert à la collecte et à `dumbmonit pack test` : ce que
//! les fixtures d'un paquet valident est exactement ce qui tournera en
//! production.

use std::collections::BTreeMap;

use dumbmonit_proto::{MetricKind, Sample};
use serde_json::Value;
use serde_json_path::NormalizedPath;

use crate::MAX_ROWS;
use crate::compile::{BodyFormat, Discover, Extractor, LabelFrom, Metric, RESERVED_LABELS, Source};
use crate::prom::{self, PromSample};

/// Longueur maximale d'une valeur d'étiquette lue dans une réponse.
const MAX_LABEL_LEN: usize = 200;

/// Une réponse décodée selon le format de sa source.
#[derive(Debug, Clone)]
pub enum Body {
    Json(Value),
    Text(String),
    Prometheus(Vec<PromSample>),
}

impl Source {
    pub(crate) fn decode(&self, bytes: &[u8]) -> Result<Body, String> {
        match self.format {
            BodyFormat::Json => serde_json::from_slice(bytes)
                .map(Body::Json)
                .map_err(|error| format!("source \"{}\": invalid JSON: {error}", self.id)),
            BodyFormat::Text => Ok(Body::Text(String::from_utf8_lossy(bytes).into_owned())),
            BodyFormat::Prometheus => prom::parse(&String::from_utf8_lossy(bytes))
                .map(Body::Prometheus)
                .map_err(|error| format!("source \"{}\": {error}", self.id)),
        }
    }

    /// Ajoute à `out` tout ce que cette source produit à partir de `body`.
    pub(crate) fn extract(&self, body: &Body, ts_ms: i64, out: &mut Vec<Sample>) {
        match body {
            Body::Json(document) => {
                for metric in &self.metrics {
                    json_metric(metric, document, ts_ms, out);
                }
                for discover in &self.discovers {
                    discover_rows(discover, document, ts_ms, out);
                }
            }
            Body::Text(text) => {
                for metric in &self.metrics {
                    regex_metric(metric, text, ts_ms, out);
                }
            }
            Body::Prometheus(samples) => {
                let kept: Vec<&PromSample> =
                    samples.iter().filter(|sample| !self.dropped(sample)).collect();
                self.keep_families(&kept, ts_ms, out);
                for metric in &self.metrics {
                    prom_metric(metric, &kept, ts_ms, out);
                }
            }
        }
    }

    fn dropped(&self, sample: &PromSample) -> bool {
        self.drop.iter().any(|(label, regex)| {
            sample.labels.get(label).is_some_and(|value| regex.is_match(value))
        })
    }

    fn keep_families(&self, samples: &[&PromSample], ts_ms: i64, out: &mut Vec<Sample>) {
        if self.keep.is_empty() {
            return;
        }
        let prefix = &self.prefix;
        for sample in samples {
            let Some(name) = self.kept_name(sample) else { continue };
            out.push(Sample {
                metric: format!("{prefix}{name}"),
                labels: exported_labels(&sample.labels),
                value: sample.value,
                kind: prom_kind(sample),
                ts_ms,
            });
        }
    }

    /// Nom écrit d'un échantillon retenu en bloc, sans préfixe.
    fn kept_name(&self, sample: &PromSample) -> Option<String> {
        for (family, written) in &self.keep {
            if &sample.name == family {
                return Some(written.clone());
            }
            if sample.family.as_deref() == Some(family.as_str()) {
                let suffix = &sample.name[family.len()..];
                return Some(format!("{written}{suffix}"));
            }
        }
        None
    }
}

fn prom_kind(sample: &PromSample) -> MetricKind {
    match sample.family_type.as_str() {
        "counter" | "histogram" => MetricKind::Counter,
        "summary" if sample.family.as_deref() != Some(sample.name.as_str()) => MetricKind::Counter,
        _ => MetricKind::Gauge,
    }
}

/// Étiquettes d'un exportateur : celles que DumbMonit réserve sont renommées
/// `exported_<nom>` plutôt que perdues ou laissées usurper l'identité de la cible.
fn exported_labels(labels: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    labels
        .iter()
        .map(|(key, value)| {
            let key = if RESERVED_LABELS.contains(&key.as_str())
                || key.starts_with("tag_")
                || key.starts_with("__")
            {
                format!("exported_{}", key.trim_start_matches('_'))
            } else {
                key.clone()
            };
            (key, truncate(value.clone()))
        })
        .collect()
}

fn truncate(mut value: String) -> String {
    if value.len() > MAX_LABEL_LEN {
        let mut end = MAX_LABEL_LEN;
        while !value.is_char_boundary(end) {
            end -= 1;
        }
        value.truncate(end);
    }
    value
}

/// Valeur numérique d'un nœud JSON : nombre, booléen, ou texte traduit par `map`
/// (ou lisible comme un nombre). `None` : rien à écrire.
fn number(value: &Value, map: &BTreeMap<String, f64>, scale: Option<f64>) -> Option<f64> {
    let raw = match value {
        Value::Number(number) => map.get(&number.to_string()).copied().or_else(|| number.as_f64()),
        Value::Bool(flag) => {
            Some(map.get(&flag.to_string()).copied().unwrap_or(if *flag { 1.0 } else { 0.0 }))
        }
        Value::String(text) => text_number(text, map),
        _ => None,
    }?;
    Some(raw * scale.unwrap_or(1.0))
}

fn text_number(text: &str, map: &BTreeMap<String, f64>) -> Option<f64> {
    let text = text.trim();
    if let Some(value) = map.get(text) {
        return Some(*value);
    }
    if !map.is_empty() {
        // Une énumération déclarée : une valeur hors liste n'est pas un nombre.
        return None;
    }
    text.parse().ok()
}

fn label_value(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(text) => Some(truncate(text.clone())),
        Value::Number(_) | Value::Bool(_) => Some(value.to_string()),
        other => Some(truncate(other.to_string())),
    }
}

/// L'objet qui contient le nœud désigné par `location`.
fn parent<'a>(document: &'a Value, location: &NormalizedPath<'_>) -> &'a Value {
    let pointer = location.to_json_pointer();
    let parent = match pointer.rfind('/') {
        Some(at) => &pointer[..at],
        None => "",
    };
    document.pointer(parent).unwrap_or(document)
}

fn json_metric(metric: &Metric, document: &Value, ts_ms: i64, out: &mut Vec<Sample>) {
    let Extractor::Json(path) = &metric.extractor else { return };
    for node in path.query_located(document).into_iter().take(MAX_ROWS) {
        let Some(value) = number(node.node(), &metric.map, metric.scale) else { continue };
        let container = parent(document, node.location());
        let mut sample = Sample::new(metric.name.clone(), value, metric.kind, ts_ms);
        for (label, from) in &metric.labels {
            if let LabelFrom::Json(path) = from
                && let Some(found) = path.query(container).first().and_then(label_value)
            {
                sample.labels.insert(label.clone(), found);
            }
        }
        out.push(sample);
    }
}

fn discover_rows(discover: &Discover, document: &Value, ts_ms: i64, out: &mut Vec<Sample>) {
    let rows = discover.rows.query(document).all();
    if rows.len() > discover.max_rows {
        tracing::debug!(
            discover = %discover.name,
            rows = rows.len(),
            max_rows = discover.max_rows,
            "lignes découvertes tronquées"
        );
    }
    for row in rows.into_iter().take(discover.max_rows) {
        let mut labels = BTreeMap::new();
        for (label, path) in &discover.labels {
            if let Some(found) = path.query(row).first().and_then(label_value) {
                labels.insert(label.clone(), found);
            }
        }
        for metric in &discover.metrics {
            let Some(node) = metric.path.query(row).first() else { continue };
            let Some(value) = number(node, &metric.map, metric.scale) else { continue };
            out.push(Sample {
                metric: metric.name.clone(),
                labels: labels.clone(),
                value,
                kind: metric.kind,
                ts_ms,
            });
        }
    }
}

fn regex_metric(metric: &Metric, text: &str, ts_ms: i64, out: &mut Vec<Sample>) {
    let Extractor::Regex(regex) = &metric.extractor else { return };
    let named_value = regex.capture_names().flatten().any(|name| name == "value");
    for captures in regex.captures_iter(text).take(MAX_ROWS) {
        let raw = if named_value { captures.name("value") } else { captures.get(1) };
        let Some(raw) = raw else { continue };
        let Some(value) = text_number(raw.as_str(), &metric.map) else { continue };
        let value = value * metric.scale.unwrap_or(1.0);
        let mut sample = Sample::new(metric.name.clone(), value, metric.kind, ts_ms);
        for (label, from) in &metric.labels {
            if let LabelFrom::Group(group) = from
                && let Some(found) = captures.name(group)
            {
                sample.labels.insert(label.clone(), truncate(found.as_str().to_string()));
            }
        }
        out.push(sample);
    }
}

fn prom_metric(metric: &Metric, samples: &[&PromSample], ts_ms: i64, out: &mut Vec<Sample>) {
    let Extractor::Prom(name) = &metric.extractor else { return };
    for sample in samples.iter().filter(|sample| &sample.name == name) {
        let labels = if metric.labels.is_empty() {
            exported_labels(&sample.labels)
        } else {
            metric
                .labels
                .iter()
                .filter_map(|(label, from)| match from {
                    LabelFrom::Prom(source) => sample
                        .labels
                        .get(source)
                        .map(|value| (label.clone(), truncate(value.clone()))),
                    _ => None,
                })
                .collect()
        };
        out.push(Sample {
            metric: metric.name.clone(),
            labels,
            value: sample.value * metric.scale.unwrap_or(1.0),
            kind: metric.kind,
            ts_ms,
        });
    }
}
