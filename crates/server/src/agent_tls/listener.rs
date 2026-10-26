//! Écoute TLS du canal des agents.
//!
//! Les poignées de main se font hors de la boucle d'acceptation, chacune dans
//! sa tâche, bornées en durée et en nombre : un client lent ou un balayage de
//! ports ne retient personne d'autre. Seules les connexions établies sont
//! remises à axum, avec ce que la poignée de main a appris (certificat client,
//! groupe d'échange de clés).

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::connect_info::Connected;
use axum::serve::{IncomingStream, Listener};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Semaphore, mpsc};
use tokio_rustls::TlsAcceptor;
use tokio_rustls::server::TlsStream;

use super::pki;
use crate::state::AppState;

/// Durée maximale d'une poignée de main.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

/// Poignées de main simultanées au plus ; au-delà, la connexion est fermée.
const MAX_HANDSHAKES: usize = 256;

/// Ce que la connexion dit de son client.
#[derive(Debug, Clone)]
pub struct AgentPeer {
    pub addr: SocketAddr,
    /// Empreinte SHA-256 du certificat client présenté, déjà vérifié contre
    /// l'autorité par rustls (signature, échéance, usage).
    pub certificate: Option<String>,
    /// Groupe d'échange de clés négocié (`X25519MLKEM768` pour un agent).
    pub group: Option<String>,
}

impl Connected<IncomingStream<'_, AgentListener>> for AgentPeer {
    fn connect_info(stream: IncomingStream<'_, AgentListener>) -> Self {
        stream.remote_addr().clone()
    }
}

/// Connexions TLS établies, prêtes pour axum.
pub struct AgentListener {
    ready: mpsc::Receiver<(TlsStream<TcpStream>, AgentPeer)>,
    local: SocketAddr,
}

impl Listener for AgentListener {
    type Io = TlsStream<TcpStream>;
    type Addr = AgentPeer;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        match self.ready.recv().await {
            Some(connection) => connection,
            // La boucle d'acceptation s'est arrêtée : plus rien ne viendra.
            None => std::future::pending().await,
        }
    }

    fn local_addr(&self) -> std::io::Result<Self::Addr> {
        Ok(AgentPeer { addr: self.local, certificate: None, group: None })
    }
}

/// Démarre l'acceptation sur un port déjà ouvert.
pub fn listen(state: AppState, tcp: TcpListener) -> std::io::Result<AgentListener> {
    let local = tcp.local_addr()?;
    let (sender, ready) = mpsc::channel(64);
    tokio::spawn(accept_loop(state, tcp, sender));
    Ok(AgentListener { ready, local })
}

async fn accept_loop(
    state: AppState,
    tcp: TcpListener,
    ready: mpsc::Sender<(TlsStream<TcpStream>, AgentPeer)>,
) {
    let handshakes = Arc::new(Semaphore::new(MAX_HANDSHAKES));
    loop {
        if ready.is_closed() {
            return;
        }
        let (stream, addr) = match tcp.accept().await {
            Ok(accepted) => accepted,
            Err(error) => {
                tracing::debug!(%error, "agent channel: accept failed");
                tokio::time::sleep(Duration::from_millis(50)).await;
                continue;
            }
        };
        let Some(config) = state.agent_tls().server_config() else { continue };
        let Ok(permit) = handshakes.clone().try_acquire_owned() else {
            tracing::debug!(%addr, "agent channel: too many handshakes in flight");
            continue;
        };
        let ready = ready.clone();
        tokio::spawn(async move {
            let _permit = permit;
            let acceptor = TlsAcceptor::from(config);
            let tls = match tokio::time::timeout(HANDSHAKE_TIMEOUT, acceptor.accept(stream)).await {
                Ok(Ok(tls)) => tls,
                Ok(Err(error)) => {
                    tracing::debug!(%addr, %error, "agent channel: handshake refused");
                    return;
                }
                Err(_) => return,
            };
            let (_, connection) = tls.get_ref();
            let peer = AgentPeer {
                addr,
                certificate: connection
                    .peer_certificates()
                    .and_then(|chain| chain.first())
                    .map(|certificate| pki::fingerprint(certificate.as_ref())),
                group: connection
                    .negotiated_key_exchange_group()
                    .map(|group| format!("{:?}", group.name())),
            };
            let _ = ready.send((tls, peer)).await;
        });
    }
}
