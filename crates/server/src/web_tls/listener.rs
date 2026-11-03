//! Écoute du port de l'interface : HTTPS seulement.
//!
//! Une poignée de main TLS commence toujours par un enregistrement de type
//! `handshake` (octet `0x16`). Le premier octet, lu sans être consommé
//! (`peek`), décide :
//!
//! - **TLS** : la poignée de main est menée à son terme, puis la connexion est
//!   confiée à l'application ;
//! - **tout le reste** (une requête HTTP en clair) ne voit **aucune**
//!   application : la ligne de requête et les en-têtes sont lus, bornés en
//!   taille et en durée, puis une redirection `308` vers `https://` sur le
//!   même hôte et le même port est renvoyée, et la connexion fermée. Sans
//!   `Host` lisible et sûr, c'est un `400` sans corps. Ni cookie, ni donnée,
//!   ni en-tête de l'application ne passent jamais en clair.
//!
//! Comme pour le canal des agents, la lecture du premier octet et la poignée
//! de main se font hors de la boucle d'acceptation, bornées en durée et en
//! nombre : un client lent ne retient personne.

use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::connect_info::Connected;
use axum::serve::{IncomingStream, Listener};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Semaphore, mpsc};
use tokio_rustls::TlsAcceptor;
use tokio_rustls::server::TlsStream;

use super::WebTls;

/// Premier octet d'un enregistrement TLS `handshake` (RFC 8446 §5.1).
const TLS_HANDSHAKE_RECORD: u8 = 0x16;

/// Délai laissé au client pour envoyer son premier octet.
const FIRST_BYTE_TIMEOUT: Duration = Duration::from_secs(10);

/// Durée maximale d'une poignée de main TLS.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

/// Connexions en cours de tri (premier octet, poignée de main, redirection)
/// au plus.
const MAX_PENDING: usize = 1024;

/// Taille maximale de la ligne de requête et des en-têtes d'une requête en
/// clair : de quoi lire un navigateur ordinaire, pas de quoi remplir la
/// mémoire.
const MAX_PLAIN_HEAD: usize = 8 * 1024;

/// Durée maximale pour lire la requête en clair, puis pour y répondre.
const PLAIN_TIMEOUT: Duration = Duration::from_secs(5);

/// Ce qu'on accepte encore de lire (et de jeter) après la réponse en clair,
/// pour que le client la voie avant la fermeture plutôt qu'une remise à zéro.
const PLAIN_DRAIN: usize = 64 * 1024;

/// Longueur maximale d'un nom d'hôte (RFC 1035).
const MAX_HOST_LEN: usize = 253;

/// Connexion servie : toujours chiffrée.
pub type WebIo = TlsStream<TcpStream>;

/// Ce que la connexion dit de son client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WebPeer {
    pub addr: SocketAddr,
}

impl Connected<IncomingStream<'_, WebListener>> for WebPeer {
    fn connect_info(stream: IncomingStream<'_, WebListener>) -> Self {
        *stream.remote_addr()
    }
}

/// Connexions triées, prêtes pour axum.
pub struct WebListener {
    ready: mpsc::Receiver<(WebIo, WebPeer)>,
    local: SocketAddr,
}

impl Listener for WebListener {
    type Io = WebIo;
    type Addr = WebPeer;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        match self.ready.recv().await {
            Some(connection) => connection,
            None => std::future::pending().await,
        }
    }

    fn local_addr(&self) -> io::Result<Self::Addr> {
        Ok(WebPeer { addr: self.local })
    }
}

/// Démarre l'acceptation sur un port déjà ouvert. Refuse sans certificat
/// chargé : le port ne sert jamais l'application en clair.
pub fn listen(web: Arc<WebTls>, tcp: TcpListener) -> io::Result<WebListener> {
    if web.server_config().is_none() {
        return Err(io::Error::other(
            "no HTTPS certificate is loaded: DumbMonit only serves HTTPS on the web port",
        ));
    }
    let local = tcp.local_addr()?;
    let (sender, ready) = mpsc::channel(128);
    tokio::spawn(accept_loop(web, tcp, sender));
    Ok(WebListener { ready, local })
}

async fn accept_loop(web: Arc<WebTls>, tcp: TcpListener, ready: mpsc::Sender<(WebIo, WebPeer)>) {
    let pending = Arc::new(Semaphore::new(MAX_PENDING));
    loop {
        if ready.is_closed() {
            return;
        }
        let (stream, addr) = match tcp.accept().await {
            Ok(accepted) => accepted,
            Err(error) => {
                tracing::debug!(%error, "web port: accept failed");
                tokio::time::sleep(Duration::from_millis(50)).await;
                continue;
            }
        };
        let _ = stream.set_nodelay(true);
        let Ok(permit) = pending.clone().try_acquire_owned() else {
            tracing::debug!(%addr, "web port: too many connections being opened");
            continue;
        };
        let ready = ready.clone();
        let web = web.clone();
        tokio::spawn(async move {
            let _permit = permit;
            if let Some(connection) = classify(&web, stream, addr).await {
                let _ = ready.send(connection).await;
            }
        });
    }
}

/// Trie une connexion : poignée de main TLS menée à son terme, ou requête en
/// clair renvoyée vers HTTPS puis fermée.
async fn classify(web: &WebTls, stream: TcpStream, addr: SocketAddr) -> Option<(WebIo, WebPeer)> {
    // Sans configuration (impossible après `listen`), rien n'est servi.
    let config = web.server_config()?;
    let mut first = [0u8; 1];
    match tokio::time::timeout(FIRST_BYTE_TIMEOUT, stream.peek(&mut first)).await {
        Ok(Ok(read)) if read > 0 => {}
        // Fermée sans rien dire, erreur ou silence prolongé.
        _ => return None,
    }
    if first[0] != TLS_HANDSHAKE_RECORD {
        redirect_plain(stream, addr).await;
        return None;
    }
    let acceptor = TlsAcceptor::from(config);
    match tokio::time::timeout(HANDSHAKE_TIMEOUT, acceptor.accept(stream)).await {
        Ok(Ok(tls)) => Some((tls, WebPeer { addr })),
        Ok(Err(error)) => {
            tracing::debug!(%addr, %error, "web port: TLS handshake refused");
            None
        }
        Err(_) => None,
    }
}

/// Répond à une requête en clair, sans rien de l'application, puis ferme.
async fn redirect_plain(mut stream: TcpStream, addr: SocketAddr) {
    let head = tokio::time::timeout(PLAIN_TIMEOUT, read_head(&mut stream)).await.ok().flatten();
    let response = plain_response(head.as_deref());
    let redirected = response.starts_with(REDIRECT_STATUS);
    tracing::debug!(%addr, redirected, "web port: plain HTTP request answered");
    let _ = tokio::time::timeout(PLAIN_TIMEOUT, async {
        stream.write_all(response.as_bytes()).await?;
        stream.shutdown().await?;
        // Le reste de la requête (un corps) est lu et jeté, borné : fermer
        // avec des octets non lus ferait perdre la réponse au client.
        let mut sink = [0u8; 4096];
        let mut drained = 0;
        while drained < PLAIN_DRAIN {
            match stream.read(&mut sink).await? {
                0 => break,
                read => drained += read,
            }
        }
        Ok::<(), io::Error>(())
    })
    .await;
}

/// Lit la ligne de requête et les en-têtes jusqu'à la ligne vide. `None` :
/// requête incomplète, connexion fermée ou en-tête trop long.
async fn read_head(stream: &mut TcpStream) -> Option<Vec<u8>> {
    let mut head = Vec::with_capacity(1024);
    let mut buffer = [0u8; 1024];
    loop {
        let read = stream.read(&mut buffer).await.ok()?;
        if read == 0 {
            return None;
        }
        head.extend_from_slice(&buffer[..read]);
        if let Some(end) = head_end(&head) {
            head.truncate(end);
            return Some(head);
        }
        if head.len() > MAX_PLAIN_HEAD {
            return None;
        }
    }
}

/// Fin de l'en-tête (ligne vide comprise), si elle est déjà arrivée et tient
/// dans la limite.
fn head_end(head: &[u8]) -> Option<usize> {
    let crlf = head.windows(4).position(|w| w == b"\r\n\r\n").map(|at| at + 4);
    let lf = head.windows(2).position(|w| w == b"\n\n").map(|at| at + 2);
    let end = match (crlf, lf) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    };
    end.filter(|end| *end <= MAX_PLAIN_HEAD)
}

/// Début de la réponse de redirection.
const REDIRECT_STATUS: &str = "HTTP/1.1 308 ";

/// Refus, sans corps.
const BAD_REQUEST: &str =
    "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";

/// Réponse à une requête en clair : `308` vers HTTPS, ou `400`.
fn plain_response(head: Option<&[u8]>) -> String {
    match head.and_then(redirect_location) {
        Some(location) => format!(
            "{REDIRECT_STATUS}Permanent Redirect\r\nLocation: {location}\r\n\
             Content-Length: 0\r\nConnection: close\r\n\r\n"
        ),
        None => BAD_REQUEST.to_string(),
    }
}

/// Adresse `https://` de la requête en clair, si elle est sûre à renvoyer.
///
/// - la requête est en ASCII, de forme `MÉTHODE /chemin HTTP/1.x` (forme
///   « origine » seulement : ni URL absolue de mandataire, ni `*`) ;
/// - le chemin n'a que des caractères visibles : aucune fin de ligne ni
///   espace ne peut glisser un en-tête dans la réponse ;
/// - un seul `Host`, fait d'un nom (lettres, chiffres, `-`, `.`) ou d'une
///   adresse IPv6 entre crochets, et d'un port facultatif : la redirection ne
///   peut viser que l'hôte reçu, jamais un autre (pas de `@`, de `/`, de `\`).
///
/// Le port est celui que le client a composé (celui de `Host`, 80 à défaut) :
/// c'est ce même port qui sert HTTPS, quelle que soit la publication Docker.
fn redirect_location(head: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(head).ok()?;
    if !text.is_ascii() {
        return None;
    }
    let mut lines = text.split('\n').map(|line| line.strip_suffix('\r').unwrap_or(line));
    let request = lines.next()?;
    let mut parts = request.split(' ');
    let (method, target, version) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some()
        || method.is_empty()
        || method.len() > 16
        || !method.bytes().all(|b| b.is_ascii_uppercase())
        || !matches!(version, "HTTP/1.0" | "HTTP/1.1")
        || !target.starts_with('/')
        || !target.bytes().all(|b| b.is_ascii_graphic())
    {
        return None;
    }
    let mut host = None;
    for line in lines {
        if line.is_empty() {
            break;
        }
        // Repli d'en-tête obsolète, caractère de contrôle : on ne devine pas.
        if line.starts_with([' ', '\t']) || line.bytes().any(|b| b.is_ascii_control() && b != b'\t')
        {
            return None;
        }
        let (name, value) = line.split_once(':')?;
        if name.is_empty() || !name.bytes().all(is_token) {
            return None;
        }
        if name.eq_ignore_ascii_case("host") {
            if host.is_some() {
                return None;
            }
            host = Some(value.trim_matches([' ', '\t']));
        }
    }
    let (name, port) = split_host(host?)?;
    Some(format!("https://{name}:{port}{target}"))
}

/// Caractère d'un nom d'en-tête (`token`, RFC 9110 §5.6.2).
fn is_token(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b)
}

/// Sépare et vérifie `Host` : nom (ou `[IPv6]`) et port, 80 à défaut.
fn split_host(host: &str) -> Option<(&str, u16)> {
    let (name, port) = match host.strip_prefix('[') {
        Some(rest) => {
            let (inside, after) = rest.split_once(']')?;
            let ipv6 = !inside.is_empty()
                && inside.len() <= 45
                && inside.contains(':')
                && inside.bytes().all(|b| b.is_ascii_hexdigit() || b == b':' || b == b'.');
            if !ipv6 {
                return None;
            }
            (&host[..inside.len() + 2], after)
        }
        None => match host.find(':') {
            Some(at) => (&host[..at], &host[at..]),
            None => (host, ""),
        },
    };
    let port = match port {
        "" => 80,
        _ => {
            let digits = port.strip_prefix(':')?;
            if digits.is_empty() || digits.len() > 5 || !digits.bytes().all(|b| b.is_ascii_digit())
            {
                return None;
            }
            digits.parse::<u16>().ok().filter(|port| *port != 0)?
        }
    };
    if !name.starts_with('[') {
        let valid = !name.is_empty()
            && name.len() <= MAX_HOST_LEN
            && !name.starts_with(['-', '.'])
            && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.');
        if !valid {
            return None;
        }
    }
    Some((name, port))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn location(head: &str) -> Option<String> {
        redirect_location(head.as_bytes())
    }

    #[test]
    fn a_plain_request_is_sent_to_https_on_the_host_and_port_it_dialled() {
        let head = "GET /alerts?state=open&x=%20 HTTP/1.1\r\nHost: nas.lan:8080\r\n\
                    Accept: text/html\r\nCookie: a=b\r\n\r\n";
        assert_eq!(location(head).as_deref(), Some("https://nas.lan:8080/alerts?state=open&x=%20"));
        // Sans port : celui que le client a composé, 80.
        let head = "HEAD / HTTP/1.0\nhost: monit.example.org\n\n";
        assert_eq!(location(head).as_deref(), Some("https://monit.example.org:80/"));
        // Une adresse IPv4, une IPv6.
        let head = "GET /x HTTP/1.1\r\nHost: 192.168.1.5:8080\r\n\r\n";
        assert_eq!(location(head).as_deref(), Some("https://192.168.1.5:8080/x"));
        let head = "GET / HTTP/1.1\r\nHost: [fd00::1]:8080\r\n\r\n";
        assert_eq!(location(head).as_deref(), Some("https://[fd00::1]:8080/"));
        let head = "POST /api/auth/login HTTP/1.1\r\nHost: [::1]\r\n\r\n";
        assert_eq!(location(head).as_deref(), Some("https://[::1]:80/api/auth/login"));
        // Un chemin qui commence par `//` reste un chemin de cet hôte.
        let head = "GET //evil.example/x HTTP/1.1\r\nHost: nas:8080\r\n\r\n";
        assert_eq!(location(head).as_deref(), Some("https://nas:8080//evil.example/x"));
    }

    #[test]
    fn a_crlf_in_the_request_cannot_add_a_header_to_the_answer() {
        // Fin de ligne dans le chemin : la ligne de requête n'a plus sa forme.
        assert_eq!(location("GET /a\r\nSet-Cookie: x=1 HTTP/1.1\r\nHost: nas\r\n\r\n"), None);
        // Retour chariot seul dans Host.
        let head = "GET / HTTP/1.1\r\nHost: nas\rSet-Cookie: x=1\r\n\r\n";
        assert_eq!(location(head), None);
        // Fin de ligne encodée : ce n'est plus un nom d'hôte.
        let head = "GET / HTTP/1.1\r\nHost: nas%0d%0aSet-Cookie:x\r\n\r\n";
        assert_eq!(location(head), None);
        // Caractère de contrôle, espace dans le chemin.
        assert_eq!(location("GET /a\u{7}b HTTP/1.1\r\nHost: nas\r\n\r\n"), None);
        assert_eq!(location("GET /a b HTTP/1.1\r\nHost: nas\r\n\r\n"), None);
        // Repli d'en-tête obsolète.
        assert_eq!(location("GET / HTTP/1.1\r\nHost: nas\r\n evil.example\r\n\r\n"), None);
        // Tout ce qui sort tient sur ses lignes.
        let response = plain_response(Some(b"GET /ok HTTP/1.1\r\nHost: nas:1\r\n\r\n"));
        assert_eq!(response.matches("\r\n").count(), 5, "{response}");
    }

    #[test]
    fn an_exotic_or_forged_host_gets_a_bare_400() {
        for host in [
            "evil.example/x",
            "user@evil.example",
            "nas.lan:8080@evil.example",
            "nas.lan\\evil.example",
            "nas.lan:80:81",
            "nas.lan:",
            "nas.lan:99999",
            "nas.lan:0",
            "nas.lan:+80",
            "-nas",
            ".nas",
            "[fd00::1",
            "[]",
            "[fd00::1]x",
            "[fd00::1]:8080x",
            "[evil.example]",
            "nås.lan",
            "",
        ] {
            let head = format!("GET / HTTP/1.1\r\nHost: {host}\r\n\r\n");
            assert_eq!(location(&head), None, "{host:?}");
            assert_eq!(plain_response(Some(head.as_bytes())), BAD_REQUEST, "{host:?}");
        }
        // Host absent, ou deux fois.
        assert_eq!(location("GET / HTTP/1.1\r\nAccept: */*\r\n\r\n"), None);
        assert_eq!(location("GET / HTTP/1.1\r\nHost: a\r\nHOST: b\r\n\r\n"), None);
        // Une URL absolue (mandataire) ou `*` ne choisit pas l'hôte.
        assert_eq!(location("GET http://evil.example/ HTTP/1.1\r\nHost: nas\r\n\r\n"), None);
        assert_eq!(location("OPTIONS * HTTP/1.1\r\nHost: nas\r\n\r\n"), None);
        // Pas du HTTP/1.x, ou une méthode qui n'en est pas une.
        assert_eq!(location("GET / HTTP/2.0\r\nHost: nas\r\n\r\n"), None);
        assert_eq!(location("get / HTTP/1.1\r\nHost: nas\r\n\r\n"), None);
        assert_eq!(location("GET / HTTP/1.1 extra\r\nHost: nas\r\n\r\n"), None);
        // Pas de corps dans le refus.
        assert!(BAD_REQUEST.ends_with("Content-Length: 0\r\nConnection: close\r\n\r\n"));
    }

    #[test]
    fn an_incomplete_or_overlong_request_is_refused() {
        assert_eq!(plain_response(None), BAD_REQUEST);
        assert_eq!(head_end(b"GET / HTTP/1.1\r\nHost: nas\r\n"), None);
        assert_eq!(head_end(b"GET / HTTP/1.1\r\n\r\nbody"), Some(18));
        let long = format!("GET /{} HTTP/1.1\r\nHost: nas\r\n\r\n", "a".repeat(MAX_PLAIN_HEAD));
        assert_eq!(head_end(long.as_bytes()), None, "au-delà de la limite");
    }

    /// Une connexion en clair servie par [`redirect_plain`].
    async fn plain_server() -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (stream, peer) = listener.accept().await.unwrap();
            redirect_plain(stream, peer).await;
        });
        addr
    }

    #[tokio::test]
    async fn the_plain_answer_carries_nothing_of_the_application() {
        let mut client = TcpStream::connect(plain_server().await).await.unwrap();
        client
            .write_all(b"GET /settings HTTP/1.1\r\nHost: nas.lan:8080\r\nCookie: s=1\r\n\r\n")
            .await
            .unwrap();
        client.shutdown().await.unwrap();
        let mut reply = String::new();
        client.read_to_string(&mut reply).await.unwrap();
        assert_eq!(
            reply,
            "HTTP/1.1 308 Permanent Redirect\r\nLocation: https://nas.lan:8080/settings\r\n\
             Content-Length: 0\r\nConnection: close\r\n\r\n"
        );

        // Une requête qui ne finit jamais son en-tête : 400 passé la limite.
        let mut client = TcpStream::connect(plain_server().await).await.unwrap();
        let _ = client.write_all(b"GET /").await;
        let _ = client.write_all(&vec![b'a'; MAX_PLAIN_HEAD + 2048]).await;
        let _ = client.shutdown().await;
        let mut reply = Vec::new();
        let _ = client.read_to_end(&mut reply).await;
        assert!(reply.starts_with(b"HTTP/1.1 400 "), "{:?}", String::from_utf8_lossy(&reply));
    }
}
