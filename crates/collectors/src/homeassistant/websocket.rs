//! Un aller-retour sur l'API WebSocket de Home Assistant, pour les réparations.
//!
//! Les réparations (`repairs/list_issues`) n'existent pas dans l'API REST. Le
//! dialogue est court : `auth_required`, `auth` avec le jeton, `auth_ok`, une
//! commande, son `result`, fermeture. Les trames sont celles de la sonde
//! WebSocket (`uptime/websocket/frame.rs`) et la connexion celle des sondes
//! (`uptime/session.rs`) : rien de neuf côté protocole.
//!
//! Un compte non administrateur a le droit de lister les réparations.

use std::time::Duration;

use dumbmonit_proto::ProbeError;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::uptime::session::{self, Deadline, LineReader, Stream};
use crate::uptime::websocket::frame::{self, Opcode};

/// Une réparation, telle que `repairs/list_issues` la décrit.
#[derive(Debug, Clone, Deserialize)]
pub struct Issue {
    pub domain: String,
    pub issue_id: String,
    #[serde(default)]
    pub severity: Option<String>,
    #[serde(default)]
    pub ignored: bool,
    #[serde(default)]
    pub dismissed_version: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Issues {
    #[serde(default)]
    pub issues: Vec<Issue>,
}

/// Où joindre `/api/websocket`, déduit de la racine HTTP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    pub tls: bool,
    pub host: String,
    pub port: u16,
    /// Chemin complet de la requête d'ouverture.
    pub path: String,
}

impl Endpoint {
    pub fn from_base_url(base_url: &str) -> Result<Self, ProbeError> {
        let (tls, rest) = if let Some(rest) = base_url.strip_prefix("https://") {
            (true, rest)
        } else if let Some(rest) = base_url.strip_prefix("http://") {
            (false, rest)
        } else {
            return Err(ProbeError::Config(format!("Unexpected address: {base_url}")));
        };
        let (authority, prefix) = match rest.find('/') {
            Some(index) => (&rest[..index], rest[index..].trim_end_matches('/')),
            None => (rest, ""),
        };
        let default_port = if tls { 443 } else { 80 };
        let (host, port) = if let Some(inner) = authority.strip_prefix('[') {
            let (host, after) = inner
                .split_once(']')
                .ok_or_else(|| ProbeError::Config(format!("Unexpected address: {base_url}")))?;
            let port = after.strip_prefix(':').and_then(|p| p.parse().ok()).unwrap_or(default_port);
            (host.to_string(), port)
        } else {
            match authority.rsplit_once(':') {
                Some((host, port)) => (
                    host.to_string(),
                    port.parse().map_err(|_| {
                        ProbeError::Config(format!("Invalid port in the address: {base_url}"))
                    })?,
                ),
                None => (authority.to_string(), default_port),
            }
        };
        Ok(Self { tls, host, port, path: format!("{prefix}/api/websocket") })
    }

    fn host_header(&self) -> String {
        let host =
            if self.host.contains(':') { format!("[{}]", self.host) } else { self.host.clone() };
        let default = if self.tls { 443 } else { 80 };
        if self.port == default { host } else { format!("{host}:{}", self.port) }
    }
}

/// Lit la liste des réparations.
pub async fn list_issues(
    endpoint: &Endpoint,
    token: &str,
    insecure_tls: bool,
    timeout: Duration,
) -> Result<Issues, ProbeError> {
    let deadline = Deadline::starting_now(timeout);
    let unreachable = |error: session::SessionError| {
        let (_, detail) = error.failure();
        ProbeError::Unreachable(format!("Home Assistant WebSocket API: {detail}"))
    };
    // L'adresse vient de l'utilisateur, comme pour l'API REST : la boucle
    // locale est permise (Home Assistant sur la même machine).
    let connected = session::connect(&endpoint.host, endpoint.port, true, deadline)
        .await
        .map_err(unreachable)?;
    let mut stream = if endpoint.tls {
        let secured = session::upgrade(connected.stream, &endpoint.host, deadline)
            .await
            .map_err(unreachable)?;
        if !secured.trusted && !insecure_tls {
            return Err(ProbeError::Config(format!(
                "Home Assistant's certificate is not trusted ({}): enable \"Accept an \
                 unverifiable certificate\" if it is self-signed.",
                secured.trust_error.unwrap_or_default()
            )));
        }
        Stream::Tls(Box::new(secured.stream))
    } else {
        Stream::Plain(connected.stream)
    };
    let outcome = converse(&mut stream, endpoint, token, deadline).await;
    let _ = stream.write_all(&frame::close_frame(mask())).await;
    stream.close().await;
    outcome
}

fn mask() -> [u8; 4] {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0x5a5a_5a5a)
        .max(1);
    nanos.to_le_bytes()
}

fn protocol(detail: impl Into<String>) -> ProbeError {
    ProbeError::Protocol(format!("Home Assistant WebSocket API: {}", detail.into()))
}

async fn converse(
    stream: &mut Stream,
    endpoint: &Endpoint,
    token: &str,
    deadline: Deadline,
) -> Result<Issues, ProbeError> {
    let key = frame::make_key(u64::from(endpoint.port));
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
         Sec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\nUser-Agent: DumbMonit/{}\r\n\r\n",
        endpoint.path,
        endpoint.host_header(),
        env!("CARGO_PKG_VERSION")
    );
    let io = |error: std::io::Error| protocol(error.to_string());
    stream.write_all(request.as_bytes()).await.map_err(io)?;

    let mut reader = LineReader::new(8192);
    let mut lines = Vec::new();
    loop {
        let line = reader.line(stream, deadline).await.map_err(|e| protocol(e.failure().1))?;
        if line.is_empty() {
            break;
        }
        lines.push(line);
        if lines.len() > 64 {
            return Err(protocol("too many headers in the opening answer"));
        }
    }
    let handshake = frame::parse_handshake(&lines).map_err(protocol)?;
    if handshake.status != 101 || handshake.accept != frame::accept_for(&key) {
        return Err(protocol(format!(
            "the connection was not upgraded (HTTP {}): a reverse proxy in front of Home \
             Assistant must forward WebSocket",
            handshake.status
        )));
    }
    let mut buffer = reader.take_buffered();

    let hello = read_message(stream, &mut buffer, deadline).await?;
    if hello["type"] != "auth_required" {
        return Err(protocol(format!("unexpected greeting {hello}")));
    }
    send(stream, &json!({"type": "auth", "access_token": token})).await?;
    let auth = read_message(stream, &mut buffer, deadline).await?;
    if auth["type"] != "auth_ok" {
        return Err(ProbeError::Auth(
            "Home Assistant refused the access token on its WebSocket API.".to_string(),
        ));
    }
    send(stream, &json!({"id": 1, "type": "repairs/list_issues"})).await?;
    loop {
        let reply = read_message(stream, &mut buffer, deadline).await?;
        if reply["id"] != 1 || reply["type"] != "result" {
            continue;
        }
        if reply["success"] != true {
            return Err(protocol(format!("repairs/list_issues failed: {}", reply["error"])));
        }
        return serde_json::from_value(reply["result"].clone())
            .map_err(|error| protocol(format!("unexpected repairs answer: {error}")));
    }
}

async fn send(stream: &mut Stream, message: &Value) -> Result<(), ProbeError> {
    stream
        .write_all(&frame::text_frame(&message.to_string(), mask()))
        .await
        .map_err(|error| protocol(error.to_string()))
}

/// Lit le prochain message JSON, en recollant les fragments éventuels.
async fn read_message(
    stream: &mut Stream,
    buffer: &mut Vec<u8>,
    deadline: Deadline,
) -> Result<Value, ProbeError> {
    let mut message = Vec::new();
    loop {
        match frame::parse_frame(buffer).map_err(protocol)? {
            Some(received) => {
                buffer.drain(..received.consumed);
                match received.opcode {
                    Opcode::Text | Opcode::Continuation | Opcode::Binary => {
                        message.extend_from_slice(&received.payload);
                        // Le dernier fragment porte le bit FIN ; `parse_frame`
                        // ne l'expose pas, mais un message JSON complet se lit.
                        if let Ok(value) = serde_json::from_slice::<Value>(&message) {
                            return Ok(value);
                        }
                        if message.len() > frame::MAX_FRAME_BYTES {
                            return Err(protocol("message too large"));
                        }
                    }
                    Opcode::Close => return Err(protocol("the server closed the connection")),
                    _ => {}
                }
            }
            None => {
                if deadline.expired() {
                    return Err(protocol("no answer before the timeout"));
                }
                let mut chunk = [0u8; 8192];
                let read = deadline
                    .wait(stream.read(&mut chunk))
                    .await
                    .map_err(|_| protocol("no answer before the timeout"))?
                    .map_err(|error| protocol(error.to_string()))?;
                if read == 0 {
                    return Err(protocol("the server closed the connection"));
                }
                buffer.extend_from_slice(&chunk[..read]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_point_d_entree_se_deduit_de_la_racine_http() {
        let cases = [
            ("http://ha.lan:8123", false, "ha.lan", 8123, "/api/websocket"),
            ("https://proxy.lan/hass", true, "proxy.lan", 443, "/hass/api/websocket"),
            ("http://[fd00::5]:8123", false, "fd00::5", 8123, "/api/websocket"),
        ];
        for (base, tls, host, port, path) in cases {
            let endpoint = Endpoint::from_base_url(base).unwrap();
            assert_eq!(endpoint, Endpoint { tls, host: host.into(), port, path: path.into() });
        }
        assert_eq!(
            Endpoint::from_base_url("http://[fd00::5]:8123").unwrap().host_header(),
            "[fd00::5]:8123"
        );
        assert!(Endpoint::from_base_url("ftp://x").is_err());
    }

    #[test]
    fn la_liste_des_reparations_se_lit() {
        // Réponse réelle de Home Assistant 2026.9.4 au compte non administrateur.
        let result: Issues =
            serde_json::from_str(include_str!("testdata/ha_2026.9.4/repairs_list_issues.json"))
                .unwrap();
        // Les réparations ignorées sont lues aussi : le tri se fait au calcul.
        assert_eq!(result.issues.len(), 3);
        assert_eq!(result.issues[0].issue_id, "country_not_configured");
        assert_eq!(result.issues[0].severity.as_deref(), Some("warning"));
        assert!(result.issues[2].ignored);
    }
}
