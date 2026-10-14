//! Traduction des réponses de l'interface de gestion en échantillons.

use dumbmonit_proto::{MetricKind, Sample};

use super::protocol::{ClientRow, StateInfo};

/// Au-delà, les clients suivants sont ignorés (et signalés dans le journal) :
/// un détail par client pour un serveur d'accès distant qui en a des milliers
/// coûterait plus de séries que personne ne lirait.
pub const MAX_CLIENTS: usize = 256;

fn gauge(name: &str, value: f64, ts_ms: i64) -> Sample {
    Sample::new(name, value, MetricKind::Gauge, ts_ms)
}

fn counter(name: &str, value: u64, ts_ms: i64) -> Sample {
    Sample::new(name, value as f64, MetricKind::Counter, ts_ms)
}

pub fn version_sample(version: &str, ts_ms: i64) -> Sample {
    gauge("openvpn_server_info", 1.0, ts_ms).with_label("version", version)
}

/// `openvpn_server_up` vaut 1 pour `CONNECTED`, l'état d'un serveur qui a fini
/// son initialisation ; tout autre état (`RECONNECTING`, `EXITING`…) vaut 0.
pub fn state_samples(state: &StateInfo, now: i64, ts_ms: i64) -> Vec<Sample> {
    vec![
        gauge("openvpn_server_up", f64::from(u8::from(state.state == "CONNECTED")), ts_ms)
            .with_label("state", &state.state),
        gauge(
            "openvpn_server_state_age_seconds",
            now.saturating_sub(state.since).max(0) as f64,
            ts_ms,
        ),
    ]
}

/// L'adresse réelle, sans son port : le port change à chaque reconnexion d'un
/// client NAT, et n'identifie rien qu'`common_name` ne dise déjà.
fn without_port(address: &str) -> &str {
    if let Some(inner) = address.strip_prefix('[') {
        return inner.split(']').next().unwrap_or(address);
    }
    match address.rsplit_once(':') {
        // Une seule occurrence de ':' : forme IPv4 "hôte:port". Plusieurs
        // occurrences (IPv6 nu, sans port) sont laissées telles quelles.
        Some((host, _)) if address.matches(':').count() == 1 => host,
        _ => address,
    }
}

/// Traduit la liste des clients en échantillons. `MAX_CLIENTS` plafonne le
/// détail ; le compte total, lui, reste exact.
pub fn client_samples(clients: &[ClientRow], now: i64, ts_ms: i64) -> Vec<Sample> {
    let mut samples = Vec::with_capacity(clients.len() * 3 + 1);
    samples.push(gauge("openvpn_clients_connected", clients.len() as f64, ts_ms));
    for client in clients.iter().take(MAX_CLIENTS) {
        let labelled = |sample: Sample| {
            sample
                .with_label("common_name", &client.common_name)
                .with_label("real_address", without_port(&client.real_address))
        };
        samples.push(labelled(counter(
            "openvpn_client_bytes_received",
            client.bytes_received,
            ts_ms,
        )));
        samples.push(labelled(counter("openvpn_client_bytes_sent", client.bytes_sent, ts_ms)));
        samples.push(labelled(gauge(
            "openvpn_client_connected_seconds",
            now.saturating_sub(client.connected_since).max(0) as f64,
            ts_ms,
        )));
    }
    samples
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client(common_name: &str, real_address: &str) -> ClientRow {
        ClientRow {
            common_name: common_name.to_string(),
            real_address: real_address.to_string(),
            virtual_address: "10.8.0.2".to_string(),
            bytes_received: 10,
            bytes_sent: 20,
            connected_since: 900,
        }
    }

    #[test]
    fn the_port_is_stripped_from_the_real_address_label() {
        assert_eq!(without_port("203.0.113.5:54321"), "203.0.113.5");
        assert_eq!(without_port("[fd00::5]:54321"), "fd00::5");
        assert_eq!(without_port("fd00::5"), "fd00::5");
    }

    #[test]
    fn client_samples_carry_counters_and_an_age() {
        let clients = vec![client("laptop", "203.0.113.5:54321")];
        let samples = client_samples(&clients, 1_000, 1_000_000);
        assert_eq!(samples[0].metric, "openvpn_clients_connected");
        assert_eq!(samples[0].value, 1.0);
        let rx = samples.iter().find(|s| s.metric == "openvpn_client_bytes_received").unwrap();
        assert_eq!(rx.kind, MetricKind::Counter);
        assert_eq!(rx.labels.get("real_address").map(String::as_str), Some("203.0.113.5"));
        assert!(!rx.labels.contains_key("port"), "no port label: it would churn every reconnect");
        let age = samples.iter().find(|s| s.metric == "openvpn_client_connected_seconds").unwrap();
        assert_eq!(age.value, 100.0);
    }

    #[test]
    fn beyond_the_cap_clients_are_counted_but_not_detailed() {
        let clients: Vec<ClientRow> =
            (0..MAX_CLIENTS + 5).map(|i| client(&format!("c{i}"), "203.0.113.5:1")).collect();
        let samples = client_samples(&clients, 0, 0);
        assert_eq!(samples[0].value, (MAX_CLIENTS + 5) as f64);
        assert_eq!(
            samples.iter().filter(|s| s.metric == "openvpn_client_bytes_received").count(),
            MAX_CLIENTS
        );
    }

    #[test]
    fn the_server_state_is_up_only_when_connected() {
        let up = state_samples(&StateInfo { since: 500, state: "CONNECTED".into() }, 600, 0);
        assert_eq!(up[0].value, 1.0);
        assert_eq!(up[1].value, 100.0);
        let down = state_samples(&StateInfo { since: 500, state: "RECONNECTING".into() }, 600, 0);
        assert_eq!(down[0].value, 0.0);
    }
}
