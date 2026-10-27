//! Conversion d'une requête OTLP en échantillons.
//!
//! Les règles suivent la traduction OpenTelemetry → Prometheus, en plus simple :
//!
//! - une jauge, ou une somme non monotone, est une jauge ;
//! - une somme monotone cumulative est un compteur, nommé `…_total` ;
//! - un histogramme cumulatif devient `…_bucket{le}` (cumulés), `…_sum` et
//!   `…_count`, trois compteurs ;
//! - un résumé devient `…_sum`, `…_count` et une jauge `{quantile}` ;
//! - la temporalité *delta* (les compteurs remis à zéro à chaque envoi) n'est pas
//!   acceptée : la sommer demanderait un état ici. Le processeur
//!   `deltatocumulative` du collecteur OpenTelemetry fait cette conversion ;
//! - un histogramme exponentiel est rejeté.
//!
//! Les points refusés sont comptés (`Converted::rejected`) et renvoyés à
//! l'émetteur dans l'accusé de réception.

use std::collections::BTreeMap;

use dumbmonit_proto::{MetricKind, Sample};

use super::{
    AnyValue, ExportMetricsServiceRequest, HistogramDataPoint, KeyValue, NumberDataPoint,
    SummaryDataPoint, TEMPORALITY_CUMULATIVE, any_value, metric, number_data_point,
};
use crate::collectors::{MAX_BACKFILL_MS, MAX_SKEW_AHEAD_MS};

/// Échantillons produits par une requête, au plus. Le reste est rejeté : une
/// requête ne doit pas inonder VictoriaMetrics.
pub const MAX_SAMPLES: usize = 50_000;
/// Étiquettes conservées par série.
const MAX_LABELS: usize = 32;
/// Longueur maximale d'une valeur d'étiquette.
const MAX_LABEL_VALUE: usize = 256;

/// Attributs de ressource promus en étiquettes (`service.name` → `service_name`).
/// Les autres décrivent l'émetteur et feraient exploser la cardinalité.
const PROMOTED_RESOURCE: [&str; 3] = ["service.name", "service.namespace", "service.instance.id"];

#[derive(Debug, Default)]
pub struct Converted {
    pub samples: Vec<Sample>,
    /// Points de données reçus, retenus ou non.
    pub received: usize,
    /// Points de données refusés (temporalité delta, histogramme exponentiel,
    /// valeur manquante, horodatage hors fenêtre, plafond dépassé).
    pub rejected: usize,
}

/// Nom de métrique ou d'étiquette valide pour Prometheus : `[a-zA-Z_][a-zA-Z0-9_]*`
/// (les deux-points restent réservés aux règles d'enregistrement).
pub fn sanitize(name: &str) -> String {
    let mut out: String =
        name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' }).collect();
    if out.chars().next().is_none_or(|c| c.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

fn text_of(value: &AnyValue) -> Option<String> {
    Some(match value.value.as_ref()? {
        any_value::Value::StringValue(s) => s.clone(),
        any_value::Value::BoolValue(b) => b.to_string(),
        any_value::Value::IntValue(i) => i.to_string(),
        any_value::Value::DoubleValue(d) => d.to_string(),
        // Une liste, une table ou des octets ne font pas une étiquette.
        _ => return None,
    })
}

fn truncate(mut value: String) -> String {
    if value.len() > MAX_LABEL_VALUE {
        let mut end = MAX_LABEL_VALUE;
        while !value.is_char_boundary(end) {
            end -= 1;
        }
        value.truncate(end);
    }
    value
}

fn put_attributes(labels: &mut BTreeMap<String, String>, attributes: &[KeyValue]) {
    for attribute in attributes {
        if labels.len() >= MAX_LABELS {
            return;
        }
        if let Some(value) = attribute.value.as_ref().and_then(text_of) {
            labels.insert(sanitize(&attribute.key), truncate(value));
        }
    }
}

fn resource_labels(attributes: &[KeyValue]) -> BTreeMap<String, String> {
    let promoted: Vec<KeyValue> = attributes
        .iter()
        .filter(|a| PROMOTED_RESOURCE.contains(&a.key.as_str()))
        .cloned()
        .collect();
    let mut labels = BTreeMap::new();
    put_attributes(&mut labels, &promoted);
    labels
}

/// Valeur de `le` : la forme la plus courte, comme le fait Prometheus.
fn le(bound: f64) -> String {
    if bound.is_infinite() { "+Inf".to_string() } else { bound.to_string() }
}

struct Context<'a> {
    now_ms: i64,
    resource: &'a BTreeMap<String, String>,
    out: &'a mut Converted,
}

impl Context<'_> {
    /// Horodatage en ms, ou `None` hors de la fenêtre acceptée.
    fn time(&self, nanos: u64) -> Option<i64> {
        if nanos == 0 {
            return Some(self.now_ms);
        }
        let ms = i64::try_from(nanos / 1_000_000).ok()?;
        (ms >= self.now_ms - MAX_BACKFILL_MS && ms <= self.now_ms + MAX_SKEW_AHEAD_MS).then_some(ms)
    }

    fn labels(&self, attributes: &[KeyValue]) -> BTreeMap<String, String> {
        let mut labels = self.resource.clone();
        put_attributes(&mut labels, attributes);
        labels
    }

    fn push(
        &mut self,
        name: &str,
        value: f64,
        kind: MetricKind,
        ts_ms: i64,
        labels: &BTreeMap<String, String>,
    ) -> bool {
        if !value.is_finite() || self.out.samples.len() >= MAX_SAMPLES {
            return false;
        }
        let mut sample = Sample::new(name, value, kind, ts_ms);
        sample.labels = labels.clone();
        self.out.samples.push(sample);
        true
    }

    fn number(&mut self, name: &str, kind: MetricKind, point: &NumberDataPoint) {
        self.out.received += 1;
        let value = match point.value {
            Some(number_data_point::Value::AsDouble(d)) => d,
            Some(number_data_point::Value::AsInt(i)) => i as f64,
            None => return self.reject(1),
        };
        let Some(ts) = self.time(point.time_unix_nano) else { return self.reject(1) };
        let labels = self.labels(&point.attributes);
        if !self.push(name, value, kind, ts, &labels) {
            self.reject(1);
        }
    }

    fn histogram(&mut self, name: &str, point: &HistogramDataPoint) {
        self.out.received += 1;
        let Some(ts) = self.time(point.time_unix_nano) else { return self.reject(1) };
        // N bornes, N + 1 seaux : un compte de seaux incohérent est refusé.
        if !point.bucket_counts.is_empty()
            && point.bucket_counts.len() != point.explicit_bounds.len() + 1
        {
            return self.reject(1);
        }
        let labels = self.labels(&point.attributes);
        let mut ok = true;
        let mut cumulative = 0u64;
        let bucket = format!("{name}_bucket");
        for (index, count) in point.bucket_counts.iter().enumerate() {
            cumulative = cumulative.saturating_add(*count);
            let bound = point.explicit_bounds.get(index).copied().unwrap_or(f64::INFINITY);
            let mut with_le = labels.clone();
            with_le.insert("le".to_string(), le(bound));
            ok &= self.push(&bucket, cumulative as f64, MetricKind::Counter, ts, &with_le);
        }
        if let Some(sum) = point.sum {
            ok &= self.push(&format!("{name}_sum"), sum, MetricKind::Counter, ts, &labels);
        }
        ok &= self.push(
            &format!("{name}_count"),
            point.count as f64,
            MetricKind::Counter,
            ts,
            &labels,
        );
        if !ok {
            self.reject(1);
        }
    }

    fn summary(&mut self, name: &str, point: &SummaryDataPoint) {
        self.out.received += 1;
        let Some(ts) = self.time(point.time_unix_nano) else { return self.reject(1) };
        let labels = self.labels(&point.attributes);
        let mut ok = self.push(&format!("{name}_sum"), point.sum, MetricKind::Counter, ts, &labels);
        ok &= self.push(
            &format!("{name}_count"),
            point.count as f64,
            MetricKind::Counter,
            ts,
            &labels,
        );
        for quantile in &point.quantile_values {
            let mut with_quantile = labels.clone();
            with_quantile.insert("quantile".to_string(), quantile.quantile.to_string());
            ok &= self.push(name, quantile.value, MetricKind::Gauge, ts, &with_quantile);
        }
        if !ok {
            self.reject(1);
        }
    }

    fn reject(&mut self, points: usize) {
        self.out.rejected += points;
    }
}

/// Convertit une requête. `now_ms` borne les horodatages acceptés et date les
/// points qui n'en portent pas.
pub fn convert(request: &ExportMetricsServiceRequest, now_ms: i64) -> Converted {
    let mut out = Converted::default();
    for resource_metrics in &request.resource_metrics {
        let attributes =
            resource_metrics.resource.as_ref().map(|r| r.attributes.as_slice()).unwrap_or(&[]);
        let resource = resource_labels(attributes);
        for scope in &resource_metrics.scope_metrics {
            for metric in &scope.metrics {
                let name = sanitize(&metric.name);
                let mut context = Context { now_ms, resource: &resource, out: &mut out };
                match &metric.data {
                    Some(metric::Data::Gauge(gauge)) => {
                        for point in &gauge.data_points {
                            context.number(&name, MetricKind::Gauge, point);
                        }
                    }
                    Some(metric::Data::Sum(sum)) => {
                        if sum.aggregation_temporality != TEMPORALITY_CUMULATIVE {
                            context.out.received += sum.data_points.len();
                            context.reject(sum.data_points.len());
                        } else if sum.is_monotonic {
                            let name = if name.ends_with("_total") {
                                name.clone()
                            } else {
                                format!("{name}_total")
                            };
                            for point in &sum.data_points {
                                context.number(&name, MetricKind::Counter, point);
                            }
                        } else {
                            for point in &sum.data_points {
                                context.number(&name, MetricKind::Gauge, point);
                            }
                        }
                    }
                    Some(metric::Data::Histogram(histogram)) => {
                        if histogram.aggregation_temporality != TEMPORALITY_CUMULATIVE {
                            context.out.received += histogram.data_points.len();
                            context.reject(histogram.data_points.len());
                        } else {
                            for point in &histogram.data_points {
                                context.histogram(&name, point);
                            }
                        }
                    }
                    Some(metric::Data::ExponentialHistogram(histogram)) => {
                        context.out.received += histogram.data_points.len();
                        context.reject(histogram.data_points.len());
                    }
                    Some(metric::Data::Summary(summary)) => {
                        for point in &summary.data_points {
                            context.summary(&name, point);
                        }
                    }
                    None => {}
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use prost::Message;

    use super::*;
    use crate::otlp::Format;

    const NOW: i64 = 1_760_000_000_000;

    fn sample<'a>(c: &'a Converted, name: &str, extra: (&str, &str)) -> Option<&'a Sample> {
        c.samples.iter().find(|s| {
            s.metric == name
                && (extra.0.is_empty()
                    || s.labels.get(extra.0).map(String::as_str) == Some(extra.1))
        })
    }

    fn parse(json: &str) -> ExportMetricsServiceRequest {
        ExportMetricsServiceRequest::decode_body(json.as_bytes(), Format::Json).unwrap()
    }

    const JSON: &str = r#"{
      "resourceMetrics": [{
        "resource": { "attributes": [
          { "key": "service.name", "value": { "stringValue": "checkout" } },
          { "key": "host.name", "value": { "stringValue": "ignored" } } ] },
        "scopeMetrics": [{ "metrics": [
          { "name": "queue.depth", "gauge": { "dataPoints": [
            { "attributes": [ { "key": "queue", "value": { "stringValue": "a" } } ],
              "timeUnixNano": "1760000000000000000", "asInt": "7" } ] } },
          { "name": "http.requests", "sum": { "aggregationTemporality": 2, "isMonotonic": true,
            "dataPoints": [ { "timeUnixNano": "1760000000000000000", "asDouble": 41.5 } ] } },
          { "name": "temp", "sum": { "aggregationTemporality": "AGGREGATION_TEMPORALITY_CUMULATIVE",
            "dataPoints": [ { "asDouble": 3 } ] } },
          { "name": "latency", "histogram": { "aggregationTemporality": 2, "dataPoints": [
            { "timeUnixNano": "1760000000000000000", "count": "6", "sum": 2.5,
              "bucketCounts": ["1", "2", "3"], "explicitBounds": [0.1, 1.0] } ] } },
          { "name": "resets", "sum": { "aggregationTemporality": 1, "isMonotonic": true,
            "dataPoints": [ { "asInt": "1" } ] } },
          { "name": "expo", "exponentialHistogram": { "dataPoints": [ {"count": "1"} ] } }
        ] }]
      }]
    }"#;

    #[test]
    fn le_json_est_converti_en_series() {
        let c = convert(&parse(JSON), NOW);
        let depth = sample(&c, "queue_depth", ("queue", "a")).unwrap();
        assert_eq!((depth.value, depth.kind, depth.ts_ms), (7.0, MetricKind::Gauge, NOW));
        assert_eq!(depth.labels.get("service_name").map(String::as_str), Some("checkout"));
        assert!(!depth.labels.contains_key("host_name"), "attribut de ressource non promu");
        let total = sample(&c, "http_requests_total", ("", "")).unwrap();
        assert_eq!((total.value, total.kind), (41.5, MetricKind::Counter));
        assert_eq!(sample(&c, "temp", ("", "")).unwrap().kind, MetricKind::Gauge);
        // Seaux cumulés : 1, 3, 6.
        assert_eq!(sample(&c, "latency_bucket", ("le", "0.1")).unwrap().value, 1.0);
        assert_eq!(sample(&c, "latency_bucket", ("le", "1")).unwrap().value, 3.0);
        assert_eq!(sample(&c, "latency_bucket", ("le", "+Inf")).unwrap().value, 6.0);
        assert_eq!(sample(&c, "latency_sum", ("", "")).unwrap().value, 2.5);
        assert_eq!(sample(&c, "latency_count", ("", "")).unwrap().value, 6.0);
        // La temporalité delta et l'histogramme exponentiel sont rejetés.
        assert!(sample(&c, "resets_total", ("", "")).is_none());
        assert_eq!(c.rejected, 2);
        assert_eq!(c.received, 6);
    }

    #[test]
    fn le_protobuf_donne_le_meme_resultat_que_le_json() {
        let from_json = parse(JSON);
        let bytes = from_json.encode_to_vec();
        let from_proto =
            ExportMetricsServiceRequest::decode_body(&bytes, Format::Protobuf).unwrap();
        assert!(from_proto == from_json, "aller-retour protobuf");
        let a = convert(&from_json, NOW);
        let b = convert(&from_proto, NOW);
        assert_eq!(a.samples, b.samples);
    }

    #[test]
    fn un_horodatage_hors_fenetre_est_rejete() {
        let json = r#"{"resourceMetrics":[{"scopeMetrics":[{"metrics":[
            {"name":"old","gauge":{"dataPoints":[{"timeUnixNano":"1000000000","asInt":"1"}]}}]}]}]}"#;
        let c = convert(&parse(json), NOW);
        assert!(c.samples.is_empty());
        assert_eq!(c.rejected, 1);
    }

    #[test]
    fn les_noms_sont_assainis_et_les_valeurs_bornees() {
        assert_eq!(sanitize("http.server.duration"), "http_server_duration");
        assert_eq!(sanitize("9lives"), "_9lives");
        assert_eq!(sanitize(""), "_");
        let long = "é".repeat(200);
        assert!(truncate(long).len() <= MAX_LABEL_VALUE);
    }

    #[test]
    fn un_resume_donne_somme_compte_et_quantiles() {
        let json = r#"{"resourceMetrics":[{"scopeMetrics":[{"metrics":[
            {"name":"rpc","summary":{"dataPoints":[{"count":"4","sum":2.0,
              "quantileValues":[{"quantile":0.5,"value":0.2}]}]}}]}]}]}"#;
        let c = convert(&parse(json), NOW);
        assert_eq!(sample(&c, "rpc_count", ("", "")).unwrap().value, 4.0);
        assert_eq!(sample(&c, "rpc", ("quantile", "0.5")).unwrap().value, 0.2);
    }
}
