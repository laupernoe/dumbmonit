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
use dumbmonit_proto::TargetId;
use rustls::ServerConfig;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;
use tokio::sync::{Semaphore, mpsc};
use tokio_rustls::TlsAcceptor;
use tokio_rustls::server::TlsStream;

use super::pki;
use crate::state::AppState;

/// Durée maximale d'une poignée de main.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

/// Poignées de main simultanées au plus ; au-delà, la connexion est fermée.
const MAX_HANDSHAKES: usize = 256;

/// Transport sous TLS : une connexion TCP directe, ou le tunnel qu'un agent
/// relais a ouvert pour un agent de son site.
pub trait Transport: AsyncRead + AsyncWrite + Unpin + Send + 'static {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send + 'static> Transport for T {}

/// Connexion établie, telle qu'axum la sert.
pub type AgentIo = TlsStream<Box<dyn Transport>>;

/// File des connexions établies, que les tunnels des relais alimentent aussi.
pub(crate) type Ready = mpsc::Sender<(AgentIo, AgentPeer)>;

/// Ce que la connexion dit de son client.
#[derive(Debug, Clone)]
pub struct AgentPeer {
    /// Adresse de la connexion : celle de l'agent, ou celle du relais pour une
    /// connexion arrivée par un tunnel.
    pub addr: SocketAddr,
    /// Empreinte SHA-256 du certificat client présenté, déjà vérifié contre
    /// l'autorité par rustls (signature, échéance, usage).
    pub certificate: Option<String>,
    /// Groupe d'échange de clés négocié (`X25519MLKEM768` pour un agent).
    pub group: Option<String>,
    /// Agent relais dont le tunnel porte cette connexion. La session TLS reste
    /// de bout en bout entre l'agent et le serveur : le relais n'en voit rien.
    pub via_relay: Option<TargetId>,
}

impl Connected<IncomingStream<'_, AgentListener>> for AgentPeer {
    fn connect_info(stream: IncomingStream<'_, AgentListener>) -> Self {
        stream.remote_addr().clone()
    }
}

/// Connexions TLS établies, prêtes pour axum.
pub struct AgentListener {
    ready: mpsc::Receiver<(AgentIo, AgentPeer)>,
    local: SocketAddr,
}

impl Listener for AgentListener {
    type Io = AgentIo;
    type Addr = AgentPeer;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        match self.ready.recv().await {
            Some(connection) => connection,
            // La boucle d'acceptation s'est arrêtée : plus rien ne viendra.
            None => std::future::pending().await,
        }
    }

    fn local_addr(&self) -> std::io::Result<Self::Addr> {
        Ok(AgentPeer { addr: self.local, certificate: None, group: None, via_relay: None })
    }
}

/// Démarre l'acceptation sur un port déjà ouvert.
pub fn listen(state: AppState, tcp: TcpListener) -> std::io::Result<AgentListener> {
    let local = tcp.local_addr()?;
    let (sender, ready) = mpsc::channel(64);
    state.agent_tls().set_ready(sender.clone());
    tokio::spawn(accept_loop(state, tcp, sender));
    Ok(AgentListener { ready, local })
}

async fn accept_loop(state: AppState, tcp: TcpListener, ready: Ready) {
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
            if let Some(connection) = handshake(config, Box::new(stream), addr, None).await {
                let _ = ready.send(connection).await;
            }
        });
    }
}

/// Poignée de main TLS bornée dans le temps, sur une connexion directe ou un
/// tunnel ; rend la connexion établie et ce qu'elle a appris.
pub(crate) async fn handshake(
    config: Arc<ServerConfig>,
    transport: Box<dyn Transport>,
    addr: SocketAddr,
    via_relay: Option<TargetId>,
) -> Option<(AgentIo, AgentPeer)> {
    let acceptor = TlsAcceptor::from(config);
    let tls = match tokio::time::timeout(HANDSHAKE_TIMEOUT, acceptor.accept(transport)).await {
        Ok(Ok(tls)) => tls,
        Ok(Err(error)) => {
            tracing::debug!(%addr, ?via_relay, %error, "agent channel: handshake refused");
            return None;
        }
        Err(_) => return None,
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
        via_relay,
    };
    Some((tls, peer))
}
