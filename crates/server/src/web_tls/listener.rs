//! Écoute mixte du port de l'interface : HTTP et HTTPS sur le même port.
//!
//! Une poignée de main TLS commence toujours par un enregistrement de type
//! `handshake` (octet `0x16`) ; une requête HTTP/1.1, par le nom d'une méthode
//! en ASCII. Le premier octet, lu sans être consommé (`peek`), décide donc :
//! un seul port à publier, `docker-compose.yml` inchangé, et les mandataires
//! inverses qui parlent HTTP au conteneur continuent comme avant.
//!
//! Comme pour le canal des agents, la lecture du premier octet et la poignée
//! de main se font hors de la boucle d'acceptation, bornées en durée et en
//! nombre : un client lent ne retient personne.

use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use axum::extract::connect_info::Connected;
use axum::serve::{IncomingStream, Listener};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
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

/// Connexions en cours de tri (premier octet, poignée de main) au plus.
const MAX_PENDING: usize = 1024;

/// Connexion servie : en clair ou sous TLS.
pub enum WebIo {
    Plain(TcpStream),
    Tls(Box<TlsStream<TcpStream>>),
}

/// Ce que la connexion dit de son client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WebPeer {
    pub addr: SocketAddr,
    /// Vrai si la connexion est chiffrée de bout en bout jusqu'à ce serveur.
    pub tls: bool,
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
        Ok(WebPeer { addr: self.local, tls: false })
    }
}

/// Démarre l'acceptation sur un port déjà ouvert.
pub fn listen(web: Arc<WebTls>, tcp: TcpListener) -> io::Result<WebListener> {
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

/// Trie une connexion : en clair, ou poignée de main TLS menée à son terme.
async fn classify(web: &WebTls, stream: TcpStream, addr: SocketAddr) -> Option<(WebIo, WebPeer)> {
    let plain = |stream| Some((WebIo::Plain(stream), WebPeer { addr, tls: false }));
    let Some(config) = web.server_config() else { return plain(stream) };
    let mut first = [0u8; 1];
    match tokio::time::timeout(FIRST_BYTE_TIMEOUT, stream.peek(&mut first)).await {
        Ok(Ok(read)) if read > 0 => {}
        // Fermée sans rien dire, erreur ou silence prolongé.
        _ => return None,
    }
    if first[0] != TLS_HANDSHAKE_RECORD {
        return plain(stream);
    }
    let acceptor = TlsAcceptor::from(config);
    match tokio::time::timeout(HANDSHAKE_TIMEOUT, acceptor.accept(stream)).await {
        Ok(Ok(tls)) => Some((WebIo::Tls(Box::new(tls)), WebPeer { addr, tls: true })),
        Ok(Err(error)) => {
            tracing::debug!(%addr, %error, "web port: TLS handshake refused");
            None
        }
        Err(_) => None,
    }
}

impl AsyncRead for WebIo {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Plain(stream) => Pin::new(stream).poll_read(cx, buf),
            Self::Tls(stream) => Pin::new(stream.as_mut()).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for WebIo {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            Self::Plain(stream) => Pin::new(stream).poll_write(cx, buf),
            Self::Tls(stream) => Pin::new(stream.as_mut()).poll_write(cx, buf),
        }
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            Self::Plain(stream) => Pin::new(stream).poll_write_vectored(cx, bufs),
            Self::Tls(stream) => Pin::new(stream.as_mut()).poll_write_vectored(cx, bufs),
        }
    }

    fn is_write_vectored(&self) -> bool {
        match self {
            Self::Plain(stream) => stream.is_write_vectored(),
            Self::Tls(stream) => stream.is_write_vectored(),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Plain(stream) => Pin::new(stream).poll_flush(cx),
            Self::Tls(stream) => Pin::new(stream.as_mut()).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Plain(stream) => Pin::new(stream).poll_shutdown(cx),
            Self::Tls(stream) => Pin::new(stream.as_mut()).poll_shutdown(cx),
        }
    }
}
