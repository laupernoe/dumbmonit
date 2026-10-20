//! Syslog RFC 5424, sur UDP (RFC 5426), TCP (RFC 6587) ou TLS (RFC 5425).
//!
//! `<PRI>1 HORODATAGE HÔTE APPLICATION PID MSGID [DONNÉES] MESSAGE`, où
//! `PRI = facilité × 8 + gravité`. Les champs de l'évènement vont dans un
//! élément de données structurées `dumbmonit@32473` (32473 : numéro
//! d'entreprise réservé aux exemples, RFC 5612). Sur un flux, chaque message est
//! préfixé de sa longueur (« octet counting », seul cadrage de RFC 5425) ou,
//! au choix, terminé par un saut de ligne pour les récepteurs anciens.

use std::sync::Arc;

use async_trait::async_trait;

use super::{Sink, StreamSink, host_port, send_datagrams, truncate};
use crate::logs::catalog::{self, DestinationConfig, KindInfo};
use crate::logs::{Record, hostname};

/// Taille maximale d'un message UDP : bien en deçà de la limite IPv4, et que
/// les récepteurs courants acceptent.
const MAX_UDP: usize = 8 * 1024;
/// Taille maximale d'un message sur un flux.
const MAX_STREAM: usize = 64 * 1024;
const SD_ID: &str = "dumbmonit@32473";
const DEFAULT_APP: &str = "dumbmonit";

const FACILITIES: &[&str] = &[
    "local0", "local1", "local2", "local3", "local4", "local5", "local6", "local7", "user",
    "daemon",
];

pub fn describe() -> KindInfo {
    KindInfo {
        kind: "syslog",
        label: "Syslog (RFC 5424)",
        summary: "Any syslog receiver (rsyslog, syslog-ng, a SIEM) over UDP, TCP or TLS.",
        doc_url: catalog::doc("syslog"),
        settings: vec![
            catalog::destination(catalog::select(
                "transport",
                "Transport",
                "UDP (RFC 5426), TCP (RFC 6587) or TLS (RFC 5425).",
                &["udp", "tcp", "tls"],
            )),
            catalog::address(
                "host:port (514 for UDP and TCP, 6514 for TLS, if omitted).",
                "syslog.lan:514",
            ),
            catalog::select(
                "framing",
                "Framing",
                "TCP and TLS only: octet counting (RFC 6587, required by RFC 5425) or one message per line.",
                &["octet-counting", "newline"],
            ),
            catalog::select(
                "facility",
                "Facility",
                "Syslog facility of every message.",
                FACILITIES,
            ),
            catalog::with_default(
                catalog::text(
                    "app_name",
                    "Application name",
                    "APP-NAME field, 48 characters at most.",
                ),
                DEFAULT_APP,
            ),
            catalog::insecure_tls(),
            catalog::destination(catalog::textarea(
                "ca_cert",
                "CA certificate",
                "TLS only: PEM certificate of a private authority. Only this authority is trusted when set.",
            )),
        ],
        secrets: vec![],
    }
}

enum Transport {
    Udp { host: String, port: u16 },
    Stream { sink: StreamSink, octet_counting: bool },
}

pub struct Syslog {
    transport: Transport,
    facility: u8,
    app_name: String,
}

fn facility_code(name: &str) -> Option<u8> {
    match name {
        "user" => Some(1),
        "daemon" => Some(3),
        other => {
            let index: u8 = other.strip_prefix("local")?.parse().ok()?;
            (index <= 7).then_some(16 + index)
        }
    }
}

pub fn build(config: &DestinationConfig) -> Result<Arc<dyn Sink>, String> {
    let address = config.text("address").ok_or("the address is required")?;
    let facility_name = config.text_or("facility", FACILITIES[0]);
    let facility = facility_code(&facility_name)
        .ok_or_else(|| format!("unknown facility `{facility_name}`"))?;
    let app_name = printable(&config.text_or("app_name", DEFAULT_APP), 48);
    let octet_counting = config.text_or("framing", "octet-counting") != "newline";
    let transport = match config.text_or("transport", "udp").as_str() {
        "udp" => {
            let (host, port) = host_port(&address, 514)?;
            Transport::Udp { host, port }
        }
        "tcp" => {
            let (host, port) = host_port(&address, 514)?;
            Transport::Stream { sink: StreamSink::new(host, port, None), octet_counting }
        }
        "tls" => {
            let (host, port) = host_port(&address, 6514)?;
            let ca = config.text("ca_cert");
            let tls = super::tls_config(ca.as_deref(), config.flag("insecure_tls"))?;
            Transport::Stream { sink: StreamSink::new(host, port, Some(tls)), octet_counting }
        }
        other => return Err(format!("unknown syslog transport `{other}`")),
    };
    Ok(Arc::new(Syslog { transport, facility, app_name }))
}

/// Caractères ASCII imprimables seulement, `-` si vide (NILVALUE).
fn printable(text: &str, max: usize) -> String {
    let clean: String = text.chars().filter(|c| c.is_ascii_graphic()).take(max).collect();
    if clean.is_empty() { "-".into() } else { clean }
}

/// Nom de paramètre : ASCII imprimable sans `=`, espace, `]` ni `"`, 32 au plus.
fn param_name(key: &str) -> String {
    let clean: String = key
        .chars()
        .map(|c| if c.is_ascii_graphic() && !matches!(c, '=' | ']' | '"') { c } else { '_' })
        .take(32)
        .collect();
    if clean.is_empty() { "_".into() } else { clean }
}

/// Valeur de paramètre : `"`, `\` et `]` échappés.
fn param_value(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        if matches!(c, '"' | '\\' | ']') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

impl Syslog {
    /// Le message RFC 5424 d'un enregistrement, sans cadrage.
    pub fn format(&self, record: &Record) -> String {
        let pri = u16::from(self.facility) * 8 + u16::from(record.level.syslog_severity());
        let mut data = format!(
            "[{SD_ID} level=\"{}\" logger=\"{}\" kind=\"{}\"",
            record.level.as_str(),
            param_value(&record.target),
            record.kind()
        );
        for (key, value) in record.flat_fields() {
            data.push_str(&format!(" {}=\"{}\"", param_name(key), param_value(&value)));
        }
        data.push(']');
        format!(
            "<{pri}>1 {} {} {} {} {} {data} {}",
            record.ts_rfc3339(),
            printable(hostname(), 255),
            self.app_name,
            std::process::id(),
            record.kind(),
            record.message
        )
    }
}

#[async_trait]
impl Sink for Syslog {
    async fn send(&self, batch: &[Arc<Record>]) -> Result<(), String> {
        match &self.transport {
            Transport::Udp { host, port } => {
                let datagrams: Vec<Vec<u8>> = batch
                    .iter()
                    .map(|record| truncate(&self.format(record), MAX_UDP).as_bytes().to_vec())
                    .collect();
                send_datagrams(host, *port, &datagrams).await
            }
            Transport::Stream { sink, octet_counting } => {
                let frames = batch
                    .iter()
                    .map(|record| {
                        let message = self.format(record);
                        let message = truncate(&message, MAX_STREAM);
                        if *octet_counting {
                            format!("{} {message}", message.len()).into_bytes()
                        } else {
                            // Cadrage par saut de ligne : un message ne peut pas en contenir.
                            format!("{}\n", message.replace(['\r', '\n'], " ")).into_bytes()
                        }
                    })
                    .collect();
                sink.write_frames(frames).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logs::Level;
    use serde_json::Value;

    fn sink() -> Syslog {
        Syslog {
            transport: Transport::Udp { host: "x".into(), port: 514 },
            facility: 16,
            app_name: "dumbmonit".into(),
        }
    }

    #[test]
    fn message_rfc5424() {
        let mut record = Record::new(Level::Warn, "probe failed");
        record.fields.insert("note".into(), Value::from("a \"b\" ]c"));
        let line = sink().format(&record);
        // local0 (16) × 8 + warning (4) = 132.
        assert!(line.starts_with("<132>1 "), "{line}");
        let parts: Vec<&str> = line.splitn(8, ' ').collect();
        assert!(parts[1].ends_with('Z'));
        assert_eq!(parts[3], "dumbmonit");
        assert_eq!(parts[5], "app");
        assert!(line.contains(r#"note="a \"b\" \]c""#), "{line}");
        assert!(line.ends_with("] probe failed"));
    }

    #[test]
    fn facilites() {
        assert_eq!(facility_code("local7"), Some(23));
        assert_eq!(facility_code("user"), Some(1));
        assert_eq!(facility_code("local8"), None);
    }
}
