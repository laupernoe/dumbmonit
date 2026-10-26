//! Serveur syslog intégré : UDP et TCP, RFC 3164 et RFC 5424.
//!
//! Désactivé par défaut (`DUMBMONIT_SYSLOG_ENABLED`). Port non privilégié par
//! défaut (5514) : le conteneur ne tourne pas en root. L'émetteur est rattaché
//! à une cible par son adresse IP (ou, à défaut, par le nom d'hôte du message),
//! et chaque message part dans VictoriaLogs sous `service="syslog"`.
//!
//! Réglages : `DUMBMONIT_SYSLOG_ENABLED`, `DUMBMONIT_SYSLOG_PORT` (5514),
//! `DUMBMONIT_SYSLOG_BIND` (toutes les interfaces), `DUMBMONIT_SYSLOG_UDP` et
//! `DUMBMONIT_SYSLOG_TCP` (actifs tous les deux par défaut).

use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio::io::{AsyncBufReadExt, AsyncReadExt};
use tokio::net::{TcpListener, UdpSocket};
use tokio::sync::Semaphore;

use super::attach;
use super::central::{self, Entry, central as hub, env_bind, env_flag, env_port};

/// Port d'écoute par défaut.
pub const DEFAULT_PORT: u16 = 5514;
/// Taille maximale d'un message accepté.
const MAX_MESSAGE: usize = 64 * 1024;
/// Connexions TCP simultanées au plus.
const MAX_CONNECTIONS: usize = 256;
/// Une connexion TCP muette plus longtemps est fermée.
const IDLE: Duration = Duration::from_secs(300);
/// Longueur maximale du texte gardé d'un message.
const MAX_TEXT: usize = 8 * 1024;

/// Un message syslog décodé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub facility: u8,
    pub severity: u8,
    /// Horodatage du message (RFC 5424 seulement) ; `None` : l'heure de réception.
    pub ts_ms: Option<i64>,
    pub host: Option<String>,
    pub app: Option<String>,
    pub text: String,
    pub format: &'static str,
}

const FACILITIES: [&str; 24] = [
    "kern", "user", "mail", "daemon", "auth", "syslog", "lpr", "news", "uucp", "cron", "authpriv",
    "ftp", "ntp", "audit", "alert", "clock", "local0", "local1", "local2", "local3", "local4",
    "local5", "local6", "local7",
];

pub fn facility_name(facility: u8) -> &'static str {
    FACILITIES.get(facility as usize).copied().unwrap_or("unknown")
}

fn nil(token: &str) -> Option<String> {
    let token = token.trim();
    if token.is_empty() || token == "-" { None } else { Some(token.to_string()) }
}

fn clip(text: &str) -> String {
    let mut end = text.len().min(MAX_TEXT);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].trim().to_string()
}

/// Découpe `token<espace>reste`.
fn token(input: &str) -> (&str, &str) {
    match input.split_once(' ') {
        Some((head, tail)) => (head, tail),
        None => (input, ""),
    }
}

/// Saute les données structurées d'un message RFC 5424 (`-` ou `[…][…]`).
fn skip_structured(input: &str) -> &str {
    if let Some(rest) = input.strip_prefix('-') {
        return rest.trim_start();
    }
    if !input.starts_with('[') {
        return input;
    }
    let bytes = input.as_bytes();
    let mut index = 0;
    let mut in_quotes = false;
    let mut end = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' if in_quotes => index += 1,
            b'"' => in_quotes = !in_quotes,
            b']' if !in_quotes => {
                end = index + 1;
                if bytes.get(end) != Some(&b'[') {
                    break;
                }
            }
            _ => {}
        }
        index += 1;
    }
    input.get(end..).unwrap_or("").trim_start()
}

fn month(name: &str) -> bool {
    ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
        .contains(&name)
}

/// Vrai si le texte commence par l'horodatage RFC 3164 (`Oct 11 22:14:15 `).
fn has_bsd_timestamp(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() > 16
        && text.get(..3).is_some_and(month)
        && bytes[3] == b' '
        && (bytes[4] == b' ' || bytes[4].is_ascii_digit())
        && bytes[5].is_ascii_digit()
        && bytes[6] == b' '
        && bytes[9] == b':'
        && bytes[12] == b':'
        && bytes[15] == b' '
}

/// Décode un message syslog (datagramme ou ligne TCP).
pub fn parse(raw: &str) -> Message {
    let raw = raw.trim_end_matches(['\n', '\r', '\0']);
    let (priority, rest) = match raw.strip_prefix('<').and_then(|r| r.split_once('>')) {
        Some((digits, rest)) if digits.len() <= 3 && digits.chars().all(|c| c.is_ascii_digit()) => {
            (digits.parse::<u16>().unwrap_or(13).min(191), rest)
        }
        _ => (13, raw),
    };
    let facility = (priority / 8) as u8;
    let severity = (priority % 8) as u8;

    // RFC 5424 : « 1 » (la version) puis l'horodatage RFC 3339.
    if let Some(body) = rest.strip_prefix("1 ") {
        let (stamp, body) = token(body);
        let (host, body) = token(body);
        let (app, body) = token(body);
        let (procid, body) = token(body);
        let (_msgid, body) = token(body);
        let text = skip_structured(body).trim_start_matches('\u{feff}');
        let ts_ms = DateTime::parse_from_rfc3339(stamp)
            .ok()
            .map(|at| at.with_timezone(&Utc).timestamp_millis());
        let app = match (nil(app), nil(procid)) {
            (Some(app), Some(pid)) => Some(format!("{app}[{pid}]")),
            (app, _) => app,
        };
        return Message {
            facility,
            severity,
            ts_ms,
            host: nil(host),
            app,
            text: clip(text),
            format: "rfc5424",
        };
    }

    // RFC 3164 : `Mmm dd hh:mm:ss HOST TAG[pid]: message`. L'heure de l'appareil,
    // sans fuseau, n'est pas crue : c'est l'heure de réception qui sert.
    let (host, body, format) = if has_bsd_timestamp(rest) {
        let after = rest.get(16..).unwrap_or("");
        let (host, body) = token(after);
        (nil(host), body, "rfc3164")
    } else {
        (None, rest, "raw")
    };
    let (app, text) = match body.find(':') {
        Some(colon) if colon > 0 && colon <= 64 && !body[..colon].contains(' ') => {
            (Some(body[..colon].to_string()), body[colon + 1..].trim_start())
        }
        _ => (None, body),
    };
    Message { facility, severity, ts_ms: None, host, app, text: clip(text), format }
}

/// L'entrée à écrire pour un message reçu de `peer`.
pub fn entry(message: Message, peer: IpAddr, transport: &str, now_ms: i64) -> (Entry, bool) {
    let found = attach::resolver().lookup(peer, message.host.as_deref());
    let host = message.host.clone().unwrap_or_else(|| peer.to_canonical().to_string());
    let mut extra = BTreeMap::new();
    extra.insert("facility".to_string(), facility_name(message.facility).to_string());
    extra.insert("peer".to_string(), peer.to_canonical().to_string());
    extra.insert("syslog_format".to_string(), message.format.to_string());
    if let Some(app) = &message.app {
        extra.insert("app".to_string(), app.clone());
    }
    let attached = found.is_some();
    // Un horodatage de l'appareil très éloigné de l'heure de réception est écarté.
    let ts_ms = message.ts_ms.filter(|ts| (*ts - now_ms).abs() < 3_600_000 * 24).unwrap_or(now_ms);
    (
        Entry {
            ts_ms,
            service: "syslog",
            level: central::level_of_severity(message.severity),
            host,
            origin: format!("{transport}:{}", peer.to_canonical()),
            message: message.text,
            target: found,
            extra,
        },
        attached,
    )
}

fn handle(raw: &[u8], peer: IpAddr, transport: &str) {
    let input = central::syslog_input();
    let text = String::from_utf8_lossy(raw);
    if text.trim().is_empty() {
        input.note_rejected();
        return;
    }
    let (entry, attached) = entry(parse(&text), peer, transport, Utc::now().timestamp_millis());
    input.note_received();
    if !attached {
        input.note_unmatched();
    }
    hub().push(entry);
}

/// Lance les récepteurs demandés par l'environnement.
pub fn spawn() {
    if !env_flag("DUMBMONIT_SYSLOG_ENABLED", false) {
        return;
    }
    let bind = env_bind("DUMBMONIT_SYSLOG_BIND");
    let port = env_port("DUMBMONIT_SYSLOG_PORT", DEFAULT_PORT);
    let address = SocketAddr::new(bind, port);
    let input = central::syslog_input();
    if env_flag("DUMBMONIT_SYSLOG_UDP", true) {
        tokio::spawn(async move {
            match UdpSocket::bind(address).await {
                Ok(socket) => {
                    tracing::info!(%address, "syslog server listening (UDP)");
                    let mut listening = input.listening();
                    listening.push(format!("udp/{port}"));
                    input.set_listening(listening);
                    let mut buffer = vec![0u8; MAX_MESSAGE];
                    loop {
                        match socket.recv_from(&mut buffer).await {
                            Ok((length, from)) => handle(&buffer[..length], from.ip(), "udp"),
                            Err(error) => {
                                tracing::debug!(%error, "syslog UDP receive failed");
                                tokio::time::sleep(Duration::from_millis(100)).await;
                            }
                        }
                    }
                }
                Err(error) => {
                    tracing::warn!(%address, %error, "syslog UDP port unavailable");
                    input.set_error(Some(format!("UDP {address}: {error}")));
                }
            }
        });
    }
    if env_flag("DUMBMONIT_SYSLOG_TCP", true) {
        tokio::spawn(async move {
            let listener = match TcpListener::bind(address).await {
                Ok(listener) => listener,
                Err(error) => {
                    tracing::warn!(%address, %error, "syslog TCP port unavailable");
                    input.set_error(Some(format!("TCP {address}: {error}")));
                    return;
                }
            };
            tracing::info!(%address, "syslog server listening (TCP)");
            let mut listening = input.listening();
            listening.push(format!("tcp/{port}"));
            input.set_listening(listening);
            let slots = Arc::new(Semaphore::new(MAX_CONNECTIONS));
            loop {
                let Ok((stream, from)) = listener.accept().await else {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                };
                let Ok(permit) = slots.clone().try_acquire_owned() else {
                    input.note_rejected();
                    continue;
                };
                tokio::spawn(async move {
                    let _permit = permit;
                    serve(stream, from.ip()).await;
                });
            }
        });
    }
}

/// Lit les messages d'une connexion TCP : comptage d'octets (`123 <34>…`) ou
/// une ligne par message.
async fn serve(stream: tokio::net::TcpStream, peer: IpAddr) {
    let mut reader = tokio::io::BufReader::new(stream);
    loop {
        let next = tokio::time::timeout(IDLE, read_frame(&mut reader)).await;
        match next {
            Ok(Ok(Some(frame))) => handle(&frame, peer, "tcp"),
            _ => return,
        }
    }
}

async fn read_frame<R>(reader: &mut tokio::io::BufReader<R>) -> std::io::Result<Option<Vec<u8>>>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let first = {
        let buffered = reader.fill_buf().await?;
        match buffered.first() {
            Some(byte) => *byte,
            None => return Ok(None),
        }
    };
    if first.is_ascii_digit() {
        // Comptage d'octets : la longueur, un espace, puis le message.
        let mut digits = Vec::new();
        let mut take = (&mut *reader).take(8);
        take.read_until(b' ', &mut digits).await?;
        let length = std::str::from_utf8(&digits)
            .ok()
            .and_then(|text| text.trim().parse::<usize>().ok())
            .filter(|length| *length > 0 && *length <= MAX_MESSAGE);
        let Some(length) = length else { return Ok(None) };
        let mut frame = vec![0u8; length];
        reader.read_exact(&mut frame).await?;
        return Ok(Some(frame));
    }
    let mut line = Vec::new();
    let mut take = (&mut *reader).take(MAX_MESSAGE as u64);
    let read = take.read_until(b'\n', &mut line).await?;
    if read == 0 {
        return Ok(None);
    }
    Ok(Some(line))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_rfc5424_message_is_decoded() {
        let raw = "<165>1 2026-10-26T10:15:30.003Z router.lan sshd 811 ID47 [exampleSDID@32473 iut=\"3\"] Failed password for root";
        let message = parse(raw);
        assert_eq!(message.facility, 20);
        assert_eq!(message.severity, 5);
        assert_eq!(message.host.as_deref(), Some("router.lan"));
        assert_eq!(message.app.as_deref(), Some("sshd[811]"));
        assert_eq!(message.text, "Failed password for root");
        assert_eq!(message.format, "rfc5424");
        assert!(message.ts_ms.is_some());
    }

    #[test]
    fn an_rfc5424_message_without_structured_data_or_fields() {
        let message = parse("<34>1 - - - - - - hello");
        assert_eq!(message.text, "hello");
        assert!(message.host.is_none());
        assert!(message.app.is_none());
        assert!(message.ts_ms.is_none());
        let bom = parse("<34>1 2026-10-26T10:15:30Z h a - - - \u{feff}with bom");
        assert_eq!(bom.text, "with bom");
    }

    #[test]
    fn an_rfc3164_message_is_decoded() {
        let message =
            parse("<34>Oct 11 22:14:15 mymachine su[123]: 'su root' failed on /dev/pts/8");
        assert_eq!(message.facility, 4);
        assert_eq!(message.severity, 2);
        assert_eq!(message.host.as_deref(), Some("mymachine"));
        assert_eq!(message.app.as_deref(), Some("su[123]"));
        assert_eq!(message.text, "'su root' failed on /dev/pts/8");
        assert_eq!(message.format, "rfc3164");
        let padded = parse("<13>Oct  1 08:00:00 host app: padded day");
        assert_eq!(padded.host.as_deref(), Some("host"));
        assert_eq!(padded.text, "padded day");
    }

    #[test]
    fn a_message_without_priority_or_header_is_kept_raw() {
        let message = parse("just some text: here\n");
        assert_eq!(message.facility, 1);
        assert_eq!(message.severity, 5);
        assert!(message.host.is_none());
        assert!(message.app.is_none(), "a space before the colon is not a tag");
        assert_eq!(message.text, "just some text: here");
        let weird = parse("<999>x");
        assert_eq!(weird.facility, 23);
    }

    #[test]
    fn structured_data_with_escapes_is_skipped() {
        let rest = skip_structured("[a b=\"x]y\\\"z\"][c d=\"1\"] message");
        assert_eq!(rest, "message");
        assert_eq!(skip_structured("-  text"), "text");
    }

    #[test]
    fn the_entry_carries_level_facility_and_peer() {
        let message = parse("<35>1 - gw app - - - link down");
        let (entry, attached) = entry(message, "10.1.2.3".parse().unwrap(), "udp", 5_000);
        assert!(!attached);
        assert_eq!(entry.level, "error");
        assert_eq!(entry.service, "syslog");
        assert_eq!(entry.host, "gw");
        assert_eq!(entry.origin, "udp:10.1.2.3");
        assert_eq!(entry.extra["facility"], "auth");
        assert_eq!(entry.ts_ms, 5_000);
    }

    #[tokio::test]
    async fn tcp_frames_are_read_with_counting_or_newlines() {
        let data: &[u8] = b"11 <34>hello x<13>second\nthird\n";
        let mut reader = tokio::io::BufReader::new(data);
        let first = read_frame(&mut reader).await.unwrap().unwrap();
        assert_eq!(first, b"<34>hello x");
        let second = read_frame(&mut reader).await.unwrap().unwrap();
        assert_eq!(second, b"<13>second\n");
        let third = read_frame(&mut reader).await.unwrap().unwrap_or_default();
        assert_eq!(third, b"third\n");
        assert!(read_frame(&mut reader).await.unwrap().is_none());
    }
}
