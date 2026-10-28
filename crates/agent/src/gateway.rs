//! Passerelle d'un relais : les agents d'un site distant joignent le serveur à
//! travers lui.
//!
//! Sur un site derrière un pare-feu, seul le relais sort vers le serveur. Les
//! autres machines du site installent l'agent avec, pour adresse du canal
//! chiffré, celle du relais (`--tls-url=https://relais:8443`). La passerelle
//! écoute sur ce port local et, pour chaque connexion :
//!
//! - **TLS** (premier octet `0x16`, un ClientHello) : elle ouvre un tunnel vers
//!   le port des agents du serveur, *sous son propre certificat client*, et y
//!   recopie les octets dans les deux sens. La poignée de main TLS 1.3, le
//!   certificat client et l'épinglage de l'autorité sont ceux de l'agent du
//!   site, de bout en bout : le relais ne voit que du chiffré, ne peut rien y
//!   changer sans casser la session, et ne peut pas se faire passer pour
//!   l'agent (il n'a ni sa clé privée ni son jeton). L'enrôlement passe par le
//!   même chemin.
//! - **HTTP** (`GET`) : seulement `/install.sh`, `/install.ps1` et
//!   `/download/<binaire>`, relus auprès du serveur par le canal chiffré du
//!   relais. Le script vérifie le binaire contre l'empreinte SHA-256 que porte
//!   la commande d'installation affichée par le serveur.
//! - **Tout le reste** est fermé aussitôt.
//!
//! Ce n'est pas un mandataire ouvert : la destination est toujours le serveur
//! auquel le relais est lui-même enrôlé, rien dans la connexion entrante ne
//! peut la choisir. Le nombre de connexions simultanées et le débit de chaque
//! tunnel sont bornés ; une connexion muette est coupée.

use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use dumbmonit_proto::RELAY_FILES_PATH;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio::time::Instant;
use tracing::{debug, info, warn};

use crate::client::PushClient;
use crate::shutdown::Shutdown;

/// Connexions simultanées par défaut (tunnels et téléchargements confondus).
pub const DEFAULT_MAX_CONNECTIONS: usize = 64;

/// Débit par défaut de chaque tunnel, dans chaque sens, en Kio/s.
pub const DEFAULT_RATE_KIB: u32 = 4096;

/// Délai pour recevoir le premier octet, puis l'en-tête d'une requête HTTP :
/// une connexion qui n'a rien à dire ne retient pas une place.
const FIRST_BYTES_TIMEOUT: Duration = Duration::from_secs(10);

/// Silence au-delà duquel un tunnel est coupé. Bien au-delà de l'attente longue
/// des commandes et du relais (25 s) et de la durée de vie des connexions
/// gardées ouvertes par l'agent (5 min).
const IDLE_TIMEOUT: Duration = Duration::from_secs(15 * 60);

/// Taille maximale de l'en-tête d'une requête HTTP acceptée.
const MAX_REQUEST_HEAD: usize = 4096;

/// Taille d'une lecture, et donc dépassement maximal du débit sur une fenêtre.
const CHUNK: usize = 16 * 1024;

/// Réglages de la passerelle, lus dans l'agent.yaml du relais.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayConfig {
    /// Adresse d'écoute (`0.0.0.0:8443`). Prime sur le port réglé dans
    /// l'interface ; `None` : c'est l'interface qui décide.
    pub listen: Option<SocketAddr>,
    /// Connexions simultanées au plus.
    pub max_connections: usize,
    /// Débit de chaque tunnel, dans chaque sens, en Kio/s.
    pub rate_kib: u32,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self { listen: None, max_connections: DEFAULT_MAX_CONNECTIONS, rate_kib: DEFAULT_RATE_KIB }
    }
}

/// Flux bidirectionnel quelconque : la connexion d'un agent, ou le tunnel
/// ouvert vers le serveur.
pub trait Io: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Io for T {}

/// Ce vers quoi la passerelle mène : le serveur du relais, et lui seul.
pub trait Upstream: Send + Sync + 'static {
    /// Ouvre un tunnel vers le port des agents du serveur.
    fn open(&self) -> impl Future<Output = Result<Box<dyn Io>, String>> + Send;

    /// Relit un fichier d'installation (`install.sh`, `download/<nom>`) et écrit
    /// la réponse HTTP complète dans `out`.
    fn fetch(
        &self,
        file: &str,
        out: &mut (dyn AsyncWrite + Unpin + Send),
    ) -> impl Future<Output = Result<(), String>> + Send;
}

/// Le serveur, joint par le canal chiffré du relais.
pub struct ServerUpstream {
    client: PushClient,
}

impl ServerUpstream {
    pub fn new(client: PushClient) -> Self {
        Self { client }
    }
}

impl Upstream for ServerUpstream {
    async fn open(&self) -> Result<Box<dyn Io>, String> {
        let tunnel = self.client.open_tunnel().await.map_err(|error| error.to_string())?;
        Ok(Box::new(tunnel))
    }

    async fn fetch(
        &self,
        file: &str,
        out: &mut (dyn AsyncWrite + Unpin + Send),
    ) -> Result<(), String> {
        let mut response =
            self.client.fetch_relay_file(file).await.map_err(|error| error.to_string())?;
        let status = response.status();
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .filter(|value| value.bytes().all(|b| b.is_ascii_graphic() || b == b' '))
            .unwrap_or("application/octet-stream")
            .to_string();
        let mut head = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {content_type}\r\nConnection: close\r\nCache-Control: no-cache\r\n",
            status.as_u16(),
            status.canonical_reason().unwrap_or("")
        );
        if let Some(length) = response.content_length() {
            head.push_str(&format!("Content-Length: {length}\r\n"));
        }
        head.push_str("\r\n");
        out.write_all(head.as_bytes()).await.map_err(|error| error.to_string())?;
        while let Some(chunk) = response.chunk().await.map_err(|error| error.to_string())? {
            out.write_all(&chunk).await.map_err(|error| error.to_string())?;
        }
        out.flush().await.map_err(|error| error.to_string())
    }
}

/// Ouvre le port de la passerelle.
pub async fn bind(addr: SocketAddr) -> std::io::Result<TcpListener> {
    TcpListener::bind(addr).await
}

/// Accepte les agents du site jusqu'à l'arrêt. Abandonner ce futur (arrêt du
/// mode relais depuis l'interface) ferme aussi toutes les connexions en cours.
pub async fn serve<U: Upstream>(
    listener: TcpListener,
    upstream: Arc<U>,
    config: GatewayConfig,
    mut shutdown: Shutdown,
) {
    let local = listener.local_addr().ok();
    info!(listen = ?local, max = config.max_connections, rate_kib = config.rate_kib,
        "relay gateway open for the agents of this site");
    let slots = Arc::new(Semaphore::new(config.max_connections.max(1)));
    let rate = u64::from(config.rate_kib.max(1)) * 1024;
    let mut connections = JoinSet::new();
    loop {
        let accepted = tokio::select! {
            biased;
            _ = shutdown.wait() => break,
            // Ramasse les connexions terminées, pour que l'ensemble ne grossisse pas.
            Some(_) = connections.join_next(), if !connections.is_empty() => continue,
            accepted = listener.accept() => accepted,
        };
        let (socket, peer) = match accepted {
            Ok(accepted) => accepted,
            Err(error) => {
                debug!(%error, "relay gateway: accept failed");
                tokio::time::sleep(Duration::from_millis(50)).await;
                continue;
            }
        };
        let Ok(slot) = slots.clone().try_acquire_owned() else {
            warn!(%peer, "relay gateway: connection limit reached, connection refused");
            continue;
        };
        let upstream = upstream.clone();
        connections.spawn(async move {
            let _slot = slot;
            handle(upstream, socket, peer, rate).await;
        });
    }
    info!("relay gateway closed");
}

/// Ce que dit le premier octet d'une connexion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Opening {
    /// Enregistrement TLS « handshake » : un ClientHello.
    Tls,
    /// Une requête HTTP en clair (`GET`).
    Http,
    Other,
}

fn classify(first: u8) -> Opening {
    match first {
        0x16 => Opening::Tls,
        b'G' => Opening::Http,
        _ => Opening::Other,
    }
}

async fn handle<U: Upstream>(upstream: Arc<U>, mut socket: TcpStream, peer: SocketAddr, rate: u64) {
    let _ = socket.set_nodelay(true);
    let mut first = [0u8; 1];
    let opening = match tokio::time::timeout(FIRST_BYTES_TIMEOUT, socket.peek(&mut first)).await {
        Ok(Ok(1)) => classify(first[0]),
        _ => return,
    };
    match opening {
        Opening::Tls => {
            let tunnel = match upstream.open().await {
                Ok(tunnel) => tunnel,
                Err(error) => {
                    warn!(%peer, %error, "relay gateway: cannot open a tunnel to the server");
                    return;
                }
            };
            debug!(%peer, "relay gateway: tunnel opened");
            splice(socket, tunnel, rate, IDLE_TIMEOUT).await;
            debug!(%peer, "relay gateway: tunnel closed");
        }
        Opening::Http => serve_file(upstream.as_ref(), &mut socket, peer).await,
        Opening::Other => debug!(%peer, "relay gateway: neither TLS nor HTTP, connection closed"),
    }
}

/// Répond à une requête HTTP : un fichier d'installation, ou un refus.
async fn serve_file<U: Upstream>(upstream: &U, socket: &mut TcpStream, peer: SocketAddr) {
    let head = match tokio::time::timeout(FIRST_BYTES_TIMEOUT, read_head(socket)).await {
        Ok(Some(head)) => head,
        _ => return,
    };
    let Some(file) = requested_file(&head) else {
        let _ = socket
            .write_all(
                b"HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nContent-Length: 34\r\nConnection: close\r\n\r\nThis relay only serves the agent.\n",
            )
            .await;
        return;
    };
    debug!(%peer, file, "relay gateway: install file requested");
    if let Err(error) = upstream.fetch(&file, &mut *socket).await {
        warn!(%peer, file, %error, "relay gateway: cannot fetch the install file from the server");
        let _ = socket
            .write_all(
                b"HTTP/1.1 502 Bad Gateway\r\nContent-Type: text/plain\r\nContent-Length: 35\r\nConnection: close\r\n\r\nThe relay cannot reach the server.\n",
            )
            .await;
    }
    let _ = socket.shutdown().await;
}

/// Lit l'en-tête d'une requête jusqu'à la ligne vide, borné en taille.
async fn read_head(socket: &mut TcpStream) -> Option<Vec<u8>> {
    let mut head = Vec::with_capacity(512);
    let mut buffer = [0u8; 512];
    loop {
        let read = socket.read(&mut buffer).await.ok()?;
        if read == 0 {
            return None;
        }
        head.extend_from_slice(&buffer[..read]);
        if head.windows(4).any(|w| w == b"\r\n\r\n") || head.windows(2).any(|w| w == b"\n\n") {
            return Some(head);
        }
        if head.len() > MAX_REQUEST_HEAD {
            return None;
        }
    }
}

/// Le fichier demandé, tel que le serveur le connaît sous
/// [`RELAY_FILES_PATH`] — ou `None` pour tout ce qui n'est pas l'agent.
fn requested_file(head: &[u8]) -> Option<String> {
    let line = head.split(|&b| b == b'\n').next()?;
    let line = std::str::from_utf8(line).ok()?.trim_end_matches('\r');
    let mut parts = line.split(' ');
    let (method, target, version) = (parts.next()?, parts.next()?, parts.next()?);
    if method != "GET" || !version.starts_with("HTTP/1.") || parts.next().is_some() {
        return None;
    }
    let path = target.split(['?', '#']).next()?;
    match path {
        "/install.sh" => Some("install.sh".to_string()),
        "/install.ps1" => Some("install.ps1".to_string()),
        _ => {
            let name = path.strip_prefix("/download/")?;
            let valid = !name.is_empty()
                && name.len() <= 128
                && !name.starts_with('.')
                && name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'));
            valid.then(|| format!("download/{name}"))
        }
    }
}

/// Chemin, côté serveur, d'un fichier d'installation relu par le relais.
pub fn files_path(file: &str) -> String {
    format!("{RELAY_FILES_PATH}/{file}")
}

/// Recopie les octets dans les deux sens, au débit donné, jusqu'à la fin des
/// deux sens ou un silence trop long.
async fn splice<A: Io, B: Io>(a: A, b: B, rate: u64, idle: Duration) {
    let (a_read, a_write) = tokio::io::split(a);
    let (b_read, b_write) = tokio::io::split(b);
    let _ = tokio::join!(pump(a_read, b_write, rate, idle), pump(b_read, a_write, rate, idle));
}

/// Un sens du tunnel. À la fin du flux entrant, le sortant est fermé (demi-
/// fermeture), pour que l'autre bout le voie.
async fn pump<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
    mut from: R,
    mut to: W,
    rate: u64,
    idle: Duration,
) -> std::io::Result<()> {
    let mut buffer = vec![0u8; CHUNK];
    let mut pace = Pace::new(rate, Instant::now());
    loop {
        let read = match tokio::time::timeout(idle, from.read(&mut buffer)).await {
            Ok(result) => result?,
            Err(_) => break,
        };
        if read == 0 {
            break;
        }
        let wait = pace.wait_after(read, Instant::now());
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
        to.write_all(&buffer[..read]).await?;
    }
    to.shutdown().await
}

/// Limite de débit par fenêtre d'une seconde.
#[derive(Debug)]
struct Pace {
    bytes_per_sec: u64,
    window: Instant,
    sent: u64,
}

impl Pace {
    fn new(bytes_per_sec: u64, now: Instant) -> Self {
        Self { bytes_per_sec: bytes_per_sec.max(1), window: now, sent: 0 }
    }

    /// Attente à observer avant d'envoyer `n` octets de plus.
    fn wait_after(&mut self, n: usize, now: Instant) -> Duration {
        let second = Duration::from_secs(1);
        if now.duration_since(self.window) >= second {
            self.window = now;
            self.sent = 0;
        }
        self.sent += n as u64;
        if self.sent <= self.bytes_per_sec {
            return Duration::ZERO;
        }
        // Fenêtre pleine : on attend la suivante, qui commence avec ces octets.
        let next = self.window + second;
        let wait = next.saturating_duration_since(now);
        self.window = next;
        self.sent = n as u64;
        wait
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    #[test]
    fn the_first_byte_tells_tls_from_http_from_anything_else() {
        assert_eq!(classify(0x16), Opening::Tls);
        assert_eq!(classify(b'G'), Opening::Http);
        assert_eq!(classify(b'C'), Opening::Other, "pas de CONNECT : ce n'est pas un mandataire");
        assert_eq!(classify(b'P'), Opening::Other);
        assert_eq!(classify(0x05), Opening::Other, "pas de SOCKS");
    }

    #[test]
    fn only_the_agent_files_are_served() {
        let ask = |line: &str| requested_file(format!("{line}\r\nHost: x\r\n\r\n").as_bytes());
        assert_eq!(ask("GET /install.sh HTTP/1.1").as_deref(), Some("install.sh"));
        assert_eq!(ask("GET /install.ps1 HTTP/1.1").as_deref(), Some("install.ps1"));
        assert_eq!(
            ask("GET /download/dumbmonit-agent-linux-x86_64 HTTP/1.1").as_deref(),
            Some("download/dumbmonit-agent-linux-x86_64")
        );
        assert_eq!(
            ask("GET /download/dumbmonit-agent-linux-x86_64.sha256?x=1 HTTP/1.0").as_deref(),
            Some("download/dumbmonit-agent-linux-x86_64.sha256")
        );
        // Ni l'API, ni l'interface, ni une traversée de répertoire, ni une URL
        // absolue de mandataire.
        assert_eq!(ask("GET /api/targets HTTP/1.1"), None);
        assert_eq!(ask("GET / HTTP/1.1"), None);
        assert_eq!(ask("GET /download/../secret.key HTTP/1.1"), None);
        assert_eq!(ask("GET /download/.hidden HTTP/1.1"), None);
        assert_eq!(ask("GET /download/a%2Fb HTTP/1.1"), None);
        assert_eq!(ask("GET http://intranet/install.sh HTTP/1.1"), None);
        assert_eq!(ask("POST /install.sh HTTP/1.1"), None);
        assert_eq!(ask("GET /install.sh"), None);
    }

    #[test]
    fn the_pace_lets_a_window_through_then_waits_for_the_next() {
        let start = Instant::now();
        let mut pace = Pace::new(1000, start);
        assert_eq!(pace.wait_after(600, start), Duration::ZERO);
        assert_eq!(pace.wait_after(400, start), Duration::ZERO);
        // Au-delà du débit : on attend la fin de la fenêtre.
        let wait = pace.wait_after(100, start + Duration::from_millis(250));
        assert_eq!(wait, Duration::from_millis(750));
        // La fenêtre suivante compte déjà ces 100 octets.
        assert_eq!(pace.wait_after(900, start + Duration::from_millis(1000)), Duration::ZERO);
        // Une fenêtre écoulée repart de zéro.
        assert_eq!(pace.wait_after(1000, start + Duration::from_millis(2500)), Duration::ZERO);
    }

    /// Serveur factice : un écho pour les tunnels, un fichier fixe pour HTTP.
    struct Echo {
        opened: AtomicUsize,
    }

    impl Upstream for Echo {
        async fn open(&self) -> Result<Box<dyn Io>, String> {
            self.opened.fetch_add(1, Ordering::SeqCst);
            let (near, mut far) = tokio::io::duplex(64 * 1024);
            tokio::spawn(async move {
                let mut buffer = [0u8; 1024];
                loop {
                    match far.read(&mut buffer).await {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            if far.write_all(&buffer[..n]).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            });
            Ok(Box::new(near))
        }

        async fn fetch(
            &self,
            file: &str,
            out: &mut (dyn AsyncWrite + Unpin + Send),
        ) -> Result<(), String> {
            let body = format!("file={file}");
            let reply = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}", body.len());
            out.write_all(reply.as_bytes()).await.map_err(|e| e.to_string())
        }
    }

    async fn gateway(max: usize) -> (SocketAddr, Arc<Echo>, crate::shutdown::ShutdownTrigger) {
        let listener = bind("127.0.0.1:0".parse().unwrap()).await.expect("écoute");
        let addr = listener.local_addr().unwrap();
        let echo = Arc::new(Echo { opened: AtomicUsize::new(0) });
        let (trigger, shutdown) = crate::shutdown::channel();
        let config = GatewayConfig { listen: None, max_connections: max, rate_kib: 1024 };
        tokio::spawn(serve(listener, echo.clone(), config, shutdown));
        (addr, echo, trigger)
    }

    #[tokio::test]
    async fn a_tls_connection_is_carried_byte_for_byte_to_the_server() {
        let (addr, echo, _stop) = gateway(4).await;
        let mut socket = TcpStream::connect(addr).await.expect("connexion");
        // Un début de ClientHello, puis n'importe quoi : la passerelle ne lit rien.
        let sent = [0x16, 0x03, 0x01, 0x00, 0x05, 1, 2, 3, 4, 5];
        socket.write_all(&sent).await.unwrap();
        let mut back = [0u8; 10];
        tokio::time::timeout(Duration::from_secs(5), socket.read_exact(&mut back))
            .await
            .expect("délai")
            .expect("lecture");
        assert_eq!(back, sent);
        assert_eq!(echo.opened.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn an_install_file_is_served_but_nothing_else() {
        let (addr, echo, _stop) = gateway(4).await;
        let mut socket = TcpStream::connect(addr).await.expect("connexion");
        socket.write_all(b"GET /install.sh HTTP/1.1\r\nHost: relais\r\n\r\n").await.unwrap();
        let mut reply = String::new();
        socket.read_to_string(&mut reply).await.unwrap();
        assert!(reply.starts_with("HTTP/1.1 200"), "{reply}");
        assert!(reply.ends_with("file=install.sh"), "{reply}");

        let mut socket = TcpStream::connect(addr).await.expect("connexion");
        socket.write_all(b"GET /api/targets HTTP/1.1\r\n\r\n").await.unwrap();
        let mut reply = String::new();
        socket.read_to_string(&mut reply).await.unwrap();
        assert!(reply.starts_with("HTTP/1.1 404"), "{reply}");

        // Un CONNECT de mandataire est fermé sans réponse, et rien n'est ouvert.
        let mut socket = TcpStream::connect(addr).await.expect("connexion");
        socket.write_all(b"CONNECT intranet:22 HTTP/1.1\r\n\r\n").await.unwrap();
        let mut reply = Vec::new();
        let _ = socket.read_to_end(&mut reply).await;
        assert!(reply.is_empty());
        assert_eq!(echo.opened.load(Ordering::SeqCst), 0, "aucun tunnel pour autre chose que TLS");
    }

    #[tokio::test]
    async fn connections_beyond_the_limit_are_refused() {
        let (addr, _echo, _stop) = gateway(1).await;
        let mut first = TcpStream::connect(addr).await.expect("connexion");
        first.write_all(&[0x16, 0x03]).await.unwrap();
        let mut back = [0u8; 2];
        first.read_exact(&mut back).await.unwrap();

        // La seconde est fermée sans être servie.
        let mut second = TcpStream::connect(addr).await.expect("connexion");
        let _ = second.write_all(&[0x16, 0x03]).await;
        let mut buffer = [0u8; 2];
        let read = tokio::time::timeout(Duration::from_secs(5), second.read(&mut buffer))
            .await
            .expect("délai");
        assert!(matches!(read, Ok(0) | Err(_)), "{read:?}");
    }
}
