//! Serveur OpenVPN, par son interface de gestion (le réglage `management
//! <adresse> <port> [fichier-de-mot-de-passe]` de `server.conf`).
//!
//! L'interface de gestion est faite pour un outil, pas pour les clients VPN
//! eux-mêmes : `version`, `state` et `status 3` suffisent à tout lire —
//! version du serveur, son état, et chaque client connecté avec son adresse
//! réelle, son adresse virtuelle et ses compteurs. Rien n'est jamais envoyé qui
//! changerait quoi que ce soit (`signal`, `kill`, `client-kill`…).
//!
//! # Mot de passe
//!
//! L'interface de gestion n'a pas de compte : au mieux un mot de passe unique,
//! partagé par quiconque s'y connecte. DumbMonit l'envoie si un identifiant de
//! type « jeton » est configuré ; sans identifiant, elle suppose l'interface
//! ouverte (ce qu'elle est par défaut quand `management` ne donne pas de
//! troisième argument).
//!
//! # Erreurs
//!
//! * connexion refusée ou sans réponse : l'équipement est injoignable ;
//! * mot de passe refusé : `ProbeError::Auth` ;
//! * une commande répond `ERROR: …` : `ProbeError::Protocol`, une panne
//!   inattendue de l'interface plutôt qu'un serveur hors ligne.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `port` | `7505` | Port de l'interface de gestion, si l'adresse n'en donne pas. |
//! | `request_timeout_seconds` | `10` | Délai par échange. |

pub mod metrics;
pub mod protocol;

use std::time::Duration;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};
use tokio::net::TcpStream;

use protocol::{Greeting, Session};

pub const DEFAULT_PORT: u16 = 7505;
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, PartialEq, Eq)]
struct Options {
    host: String,
    port: u16,
    timeout: Duration,
}

fn tag<'a>(target: &'a Target, key: &str) -> Option<&'a str> {
    target.tags.get(key).map(|v| v.trim()).filter(|v| !v.is_empty())
}

fn parse_port(raw: &str) -> Result<u16, ProbeError> {
    match raw.trim().parse::<u16>() {
        Ok(port) if port > 0 => Ok(port),
        _ => Err(ProbeError::Config(format!("Invalid port: \"{raw}\""))),
    }
}

/// `host`, `host:port`, `[ipv6]`, `[ipv6]:port`. Pas de schéma : l'interface de
/// gestion n'en a jamais eu.
fn parse_address(address: &str) -> Result<(String, Option<u16>), ProbeError> {
    let rest = address.trim();
    if rest.is_empty() {
        return Err(ProbeError::Config("Device address is empty".to_string()));
    }
    if let Some(inner) = rest.strip_prefix('[') {
        let (name, after) = inner
            .split_once(']')
            .ok_or_else(|| ProbeError::Config(format!("Invalid address: \"{address}\"")))?;
        let port = after.strip_prefix(':').map(parse_port).transpose()?;
        return Ok((name.to_string(), port));
    }
    if rest.matches(':').count() > 1 {
        return Ok((rest.to_string(), None));
    }
    match rest.split_once(':') {
        Some((name, port)) if !name.is_empty() => Ok((name.to_string(), Some(parse_port(port)?))),
        Some(_) => Err(ProbeError::Config(format!("Invalid address: \"{address}\""))),
        None => Ok((rest.to_string(), None)),
    }
}

impl Options {
    fn from_target(target: &Target) -> Result<Self, ProbeError> {
        let (host, port_in_address) = parse_address(&target.address)?;
        let port = match port_in_address {
            Some(port) => port,
            None => tag(target, "port").map(parse_port).transpose()?.unwrap_or(DEFAULT_PORT),
        };
        let timeout = match tag(target, "request_timeout_seconds") {
            None => DEFAULT_REQUEST_TIMEOUT,
            Some(raw) => {
                let seconds: u64 =
                    raw.parse().ok().filter(|s| (1..=60).contains(s)).ok_or_else(|| {
                        ProbeError::Config(format!(
                            "Invalid timeout \"{raw}\": between 1 and 60 seconds"
                        ))
                    })?;
                Duration::from_secs(seconds)
            }
        };
        Ok(Self { host, port, timeout })
    }
}

async fn connect(options: &Options) -> Result<TcpStream, ProbeError> {
    let address = (options.host.as_str(), options.port);
    match tokio::time::timeout(options.timeout, TcpStream::connect(address)).await {
        Ok(Ok(stream)) => Ok(stream),
        Ok(Err(error)) => Err(ProbeError::Unreachable(format!(
            "OpenVPN management interface at {}:{}: {error}",
            options.host, options.port
        ))),
        Err(_) => Err(ProbeError::Timeout(options.timeout)),
    }
}

/// L'échange complet, sur un flux déjà ouvert.
async fn read_all<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(
    session: &mut Session<S>,
    credential: &Credential,
) -> Result<Vec<Sample>, ProbeError> {
    let now = chrono::Utc::now();
    let ts_ms = now.timestamp_millis();

    match (session.greet().await?, credential) {
        (Greeting::PasswordRequired, Credential::ApiToken { token }) if !token.is_empty() => {
            session.authenticate(token).await?;
        }
        (Greeting::PasswordRequired, _) => {
            return Err(ProbeError::Config(
                "the management interface asks for a password: configure it as the credential"
                    .to_string(),
            ));
        }
        (Greeting::Ready, _) => {}
    }

    let mut out = Vec::new();
    if let Some(version) = session.version().await? {
        out.push(metrics::version_sample(&version, ts_ms));
    }
    if let Some(state) = session.state().await? {
        out.extend(metrics::state_samples(&state, now.timestamp(), ts_ms));
    }
    let clients = session.status3().await?;
    out.extend(metrics::client_samples(&clients, now.timestamp(), ts_ms));

    session.quit().await;
    Ok(out)
}

#[derive(Default)]
pub struct OpenvpnCollector;

impl OpenvpnCollector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Collector for OpenvpnCollector {
    fn kind(&self) -> &'static str {
        "openvpn"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let options = Options::from_target(target)?;
        let stream = connect(&options).await?;
        let mut session = Session::new(stream, options.timeout);
        read_all(&mut session, &target.credential).await
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        self.probe(target).await?;
        Ok(Some("openvpn".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpListener;

    use super::*;

    fn target(address: &str, tags: &[(&str, &str)], credential: Credential) -> Target {
        Target {
            id: 9,
            name: "vpn".into(),
            address: address.into(),
            kind: "openvpn".into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            tags: tags
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<BTreeMap<_, _>>(),
            credential,
            group_name: String::new(),
            position: 0,
        }
    }

    /// Un faux serveur de gestion : répond aux commandes reçues à partir des
    /// transcriptions capturées, et note ce qu'il a reçu.
    async fn fake_management(
        answers: Vec<(&'static str, String)>,
        greeting: &'static str,
    ) -> (String, tokio::task::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let handle = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let (read, mut write) = socket.into_split();
            write.write_all(greeting.as_bytes()).await.unwrap();
            let mut lines = BufReader::new(read).lines();
            let mut received = Vec::new();
            while let Ok(Some(line)) = lines.next_line().await {
                received.push(line.clone());
                let answer = answers
                    .iter()
                    .find(|(command, _)| line.starts_with(command))
                    .map(|(_, answer)| answer.clone())
                    .unwrap_or_else(|| "ERROR: unknown command\nEND\n".to_string());
                write.write_all(answer.as_bytes()).await.unwrap();
                if line == "quit" {
                    break;
                }
            }
            received
        });
        (address, handle)
    }

    fn server_answers() -> Vec<(&'static str, String)> {
        vec![
            (
                "version",
                "OpenVPN Version: OpenVPN 2.6.9 [git:v2.6.9/x] x86_64-pc-linux-gnu [SSL]\n\
                 Management Version: 5\nEND\n"
                    .to_string(),
            ),
            ("state", "1700000000,CONNECTED,SUCCESS,,,,,,\nEND\n".to_string()),
            (
                "status 3",
                include_str!("testdata/management_session.txt")
                    .lines()
                    .skip_while(|l| !l.starts_with("TITLE"))
                    .collect::<Vec<_>>()
                    .join("\n")
                    + "\n",
            ),
        ]
    }

    #[tokio::test]
    async fn an_open_interface_is_read_without_a_credential() {
        let (address, server) =
            fake_management(server_answers(), ">INFO:OpenVPN Management Interface Version 5\n")
                .await;
        let samples =
            OpenvpnCollector::new().probe(&target(&address, &[], Credential::None)).await.unwrap();
        assert_eq!(
            samples.iter().find(|s| s.metric == "openvpn_server_info").unwrap().labels["version"],
            "2.6.9"
        );
        assert_eq!(samples.iter().find(|s| s.metric == "openvpn_server_up").unwrap().value, 1.0);
        assert_eq!(
            samples.iter().find(|s| s.metric == "openvpn_clients_connected").unwrap().value,
            1.0
        );
        let received = server.await.unwrap();
        assert_eq!(received.last().map(String::as_str), Some("quit"));
        assert!(received.iter().all(|l| l != "kill" && !l.starts_with("client-kill")));
    }

    #[tokio::test]
    async fn a_password_protected_interface_is_authenticated() {
        let mut answers = server_answers();
        answers.insert(0, ("secret", "SUCCESS: password is correct\n".to_string()));
        let (address, server) = fake_management(answers, "ENTER PASSWORD:").await;
        let credential = Credential::ApiToken { token: "secret".to_string() };
        let samples =
            OpenvpnCollector::new().probe(&target(&address, &[], credential)).await.unwrap();
        assert!(samples.iter().any(|s| s.metric == "openvpn_server_info"));
        let received = server.await.unwrap();
        assert_eq!(received.first().map(String::as_str), Some("secret"));
    }

    #[tokio::test]
    async fn a_password_protected_interface_without_a_credential_is_a_config_error() {
        let (address, _server) = fake_management(server_answers(), "ENTER PASSWORD:").await;
        let error = OpenvpnCollector::new()
            .probe(&target(&address, &[], Credential::None))
            .await
            .unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)));
        assert!(!error.means_down());
    }

    #[tokio::test]
    async fn a_wrong_password_is_an_auth_error_not_a_config_error() {
        let mut answers = server_answers();
        answers.insert(0, ("wrong", "ERROR: bad password\n".to_string()));
        let (address, _server) = fake_management(answers, "ENTER PASSWORD:").await;
        let credential = Credential::ApiToken { token: "wrong".to_string() };
        let error =
            OpenvpnCollector::new().probe(&target(&address, &[], credential)).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)));
    }

    #[tokio::test]
    async fn a_closed_port_is_unreachable() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap().to_string();
        drop(listener);
        let error = OpenvpnCollector::new()
            .probe(&target(&address, &[], Credential::None))
            .await
            .unwrap_err();
        assert!(error.means_down(), "{error}");
    }

    #[test]
    fn addresses_and_options() {
        let o = |address: &str, tags: &[(&str, &str)]| {
            Options::from_target(&target(address, tags, Credential::None))
        };
        let options = o("vpn.lan", &[]).unwrap();
        assert_eq!((options.host.as_str(), options.port), ("vpn.lan", DEFAULT_PORT));
        assert_eq!(options.timeout, DEFAULT_REQUEST_TIMEOUT);
        assert_eq!(o("vpn.lan:7506", &[("port", "1")]).unwrap().port, 7506);
        assert_eq!(o("vpn.lan", &[("port", "7777")]).unwrap().port, 7777);
        assert_eq!(o("[fd00::5]:7505", &[]).unwrap().host, "fd00::5");
        assert_eq!(o("fd00::5", &[]).unwrap().host, "fd00::5");
        for (key, value) in [("port", "0"), ("request_timeout_seconds", "90"), ("port", "abc")] {
            assert!(matches!(o("x", &[(key, value)]), Err(ProbeError::Config(_))), "{key}={value}");
        }
        assert!(o("  ", &[]).is_err());
    }
}
