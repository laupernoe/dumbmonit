//! Lecture de `stub_status`, et de l'en-tête `Server`.
//!
//! Module purement fonctionnel : la mise en forme texte de `stub_status` se
//! teste donc sur des chaînes figées, sans serveur Nginx en face.

use dumbmonit_proto::{MetricKind, Sample};

/// Les six compteurs de `stub_status`, tels que documentés par Nginx :
/// <https://nginx.org/en/docs/http/ngx_http_stub_status_module.html>.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct StubStatus {
    pub active: f64,
    pub accepts: f64,
    pub handled: f64,
    pub requests: f64,
    pub reading: f64,
    pub writing: f64,
    pub waiting: f64,
}

/// Lit la page texte de `stub_status`.
///
/// Le format est fixe depuis la première version du module : quatre lignes,
/// dont la troisième porte trois nombres sans étiquette. Un module qui change
/// cela casserait aussi tous les tableaux de bord externes qui le lisent —
/// `nginx-prometheus-exporter` inclus — ce risque est donc faible.
pub fn parse_stub_status(text: &str) -> Result<StubStatus, String> {
    let mut lines = text.lines();

    let active_line = lines.next().ok_or("empty response")?;
    let active = active_line
        .trim()
        .strip_prefix("Active connections:")
        .ok_or_else(|| format!("missing \"Active connections:\" line, got \"{active_line}\""))?
        .trim()
        .parse::<f64>()
        .map_err(|_| format!("invalid active connections value in \"{active_line}\""))?;

    let _header = lines.next().ok_or("missing \"server accepts handled requests\" line")?;

    let counters_line = lines.next().ok_or("missing counters line")?;
    let mut counters = counters_line.split_whitespace();
    let accepts = next_number(&mut counters, "accepts")?;
    let handled = next_number(&mut counters, "handled")?;
    let requests = next_number(&mut counters, "requests")?;

    let rww_line = lines.next().ok_or("missing \"Reading / Writing / Waiting\" line")?;
    let (reading, writing, waiting) = parse_reading_writing_waiting(rww_line)?;

    Ok(StubStatus { active, accepts, handled, requests, reading, writing, waiting })
}

fn next_number<'a>(tokens: &mut impl Iterator<Item = &'a str>, name: &str) -> Result<f64, String> {
    let raw = tokens.next().ok_or_else(|| format!("missing \"{name}\" counter"))?;
    raw.parse().map_err(|_| format!("invalid \"{name}\" counter: \"{raw}\""))
}

/// `Reading: 6 Writing: 179 Waiting: 106` : trois paires `clé: valeur`, dans un
/// ordre fixe.
fn parse_reading_writing_waiting(line: &str) -> Result<(f64, f64, f64), String> {
    let mut values = [0.0; 3];
    let names = ["Reading:", "Writing:", "Waiting:"];
    let mut tokens = line.split_whitespace();
    for (slot, expected) in values.iter_mut().zip(names) {
        tokens
            .next()
            .filter(|t| *t == expected)
            .ok_or_else(|| format!("expected \"{expected}\" in \"{line}\""))?;
        let raw = tokens
            .next()
            .ok_or_else(|| format!("missing value after \"{expected}\" in \"{line}\""))?;
        *slot =
            raw.parse().map_err(|_| format!("invalid value after \"{expected}\": \"{raw}\""))?;
    }
    Ok((values[0], values[1], values[2]))
}

fn gauge(metric: &str, value: f64, ts_ms: i64) -> Sample {
    Sample::new(metric, value, MetricKind::Gauge, ts_ms)
}

fn counter(metric: &str, value: f64, ts_ms: i64) -> Sample {
    Sample::new(metric, value, MetricKind::Counter, ts_ms)
}

pub fn samples(status: &StubStatus, ts_ms: i64) -> Vec<Sample> {
    vec![
        gauge("nginx_connections_active", status.active, ts_ms),
        gauge("nginx_connections_reading", status.reading, ts_ms),
        gauge("nginx_connections_writing", status.writing, ts_ms),
        gauge("nginx_connections_waiting", status.waiting, ts_ms),
        counter("nginx_accepts_total", status.accepts, ts_ms),
        counter("nginx_handled_total", status.handled, ts_ms),
        counter("nginx_requests_total", status.requests, ts_ms),
    ]
}

/// Présence de l'en-tête `Server`, étiquette unique : un contrôle de sécurité
/// lit cette série pour juger la divulgation de version (`server_tokens`).
pub fn server_header_sample(server: &str, ts_ms: i64) -> Sample {
    gauge("nginx_version_info", 1.0, ts_ms).with_label("server", server)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXEMPLE: &str = "Active connections: 291 \n\
                            server accepts handled requests\n\
                            16630948 16630946 31070465 \n\
                            Reading: 6 Writing: 179 Waiting: 106 \n";

    #[test]
    fn lexemple_de_la_documentation_se_lit_entierement() {
        let status = parse_stub_status(EXEMPLE).unwrap();
        assert_eq!(
            status,
            StubStatus {
                active: 291.0,
                accepts: 16_630_948.0,
                handled: 16_630_946.0,
                requests: 31_070_465.0,
                reading: 6.0,
                writing: 179.0,
                waiting: 106.0,
            }
        );
    }

    #[test]
    fn une_ligne_manquante_est_signalee() {
        assert!(parse_stub_status("").is_err());
        assert!(parse_stub_status("Active connections: 1\n").is_err());
        assert!(parse_stub_status("not stub_status at all").is_err());
    }

    #[test]
    fn un_nombre_illisible_est_signale() {
        let cassee = EXEMPLE.replace("291", "beaucoup");
        let error = parse_stub_status(&cassee).unwrap_err();
        assert!(error.contains("active connections"), "{error}");
    }

    #[test]
    fn un_ordre_reading_writing_waiting_different_est_refuse() {
        let cassee = "Active connections: 1\nserver accepts handled requests\n1 1 1\n\
                       Writing: 1 Reading: 1 Waiting: 1\n";
        assert!(parse_stub_status(cassee).is_err());
    }

    #[test]
    fn les_compteurs_cumulatifs_sont_des_compteurs() {
        let samples = samples(&StubStatus::default(), 0);
        for metric in ["nginx_accepts_total", "nginx_handled_total", "nginx_requests_total"] {
            assert_eq!(
                samples.iter().find(|s| s.metric == metric).unwrap().kind,
                MetricKind::Counter,
                "{metric}"
            );
        }
        for metric in [
            "nginx_connections_active",
            "nginx_connections_reading",
            "nginx_connections_writing",
            "nginx_connections_waiting",
        ] {
            assert_eq!(
                samples.iter().find(|s| s.metric == metric).unwrap().kind,
                MetricKind::Gauge,
                "{metric}"
            );
        }
    }

    #[test]
    fn len_tete_serveur_porte_la_version_en_etiquette() {
        let sample = server_header_sample("nginx/1.25.3", 0);
        assert_eq!(sample.labels.get("server").map(String::as_str), Some("nginx/1.25.3"));
        assert_eq!(sample.value, 1.0);
    }
}
