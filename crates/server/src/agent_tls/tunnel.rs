//! Tunnels des agents relais.
//!
//! Sur un site derrière un pare-feu, seul l'agent relais sort vers le serveur.
//! Les autres agents du site se connectent à sa passerelle, qui ouvre pour
//! chacun un tunnel ici : une mise à niveau HTTP sur le port des agents, sous
//! le certificat client du relais. Le flux mis à niveau est ensuite traité
//! comme une connexion neuve à ce port — poignée de main TLS 1.3, certificat
//! client de l'agent du site, mêmes routes et mêmes contrôles —, avec en plus
//! le relais qui le porte ([`AgentPeer::via_relay`]).
//!
//! Le relais ne fait que transporter du chiffré : il ne lit rien, ne peut rien
//! modifier, et ne peut pas se faire passer pour l'agent. Ce qu'il pourrait
//! tenter est borné ici : un nombre de tunnels par relais, la même limite de
//! poignées de main que le port lui-même, et les contrôles de
//! [`super::routes`] (un agent d'un autre client ne passe pas par lui).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use dumbmonit_proto::TargetId;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use super::listener::{self, Transport};
use crate::state::AppState;

/// Tunnels simultanés au plus par relais : de quoi servir un gros site (deux à
/// trois connexions gardées ouvertes par agent), pas de quoi épuiser le serveur.
pub const MAX_TUNNELS_PER_RELAY: usize = 256;

/// Place de tunnel réservée pour un relais ; rendue à la fermeture.
pub struct Slot {
    open: Arc<Mutex<HashMap<TargetId, usize>>>,
    relay: TargetId,
}

impl Slot {
    pub fn reserve(
        open: &Arc<Mutex<HashMap<TargetId, usize>>>,
        relay: TargetId,
        max: usize,
    ) -> Option<Self> {
        let mut counts = open.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let count = counts.entry(relay).or_insert(0);
        if *count >= max {
            return None;
        }
        *count += 1;
        Some(Self { open: open.clone(), relay })
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        let mut counts = self.open.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(count) = counts.get_mut(&self.relay) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                counts.remove(&self.relay);
            }
        }
    }
}

/// Transport qui garde sa place de tunnel tant qu'il vit.
struct Guarded<T> {
    inner: T,
    _slot: Slot,
}

impl<T: AsyncRead + Unpin> AsyncRead for Guarded<T> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl<T: AsyncWrite + Unpin> AsyncWrite for Guarded<T> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

/// Pourquoi un tunnel n'a pas été accordé.
#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Le port des agents n'écoute pas (ou plus).
    Closed,
    /// Ce relais a atteint son plafond de tunnels.
    Busy,
}

/// Tunnel accordé : sa place est réservée, il ne reste qu'à le brancher.
pub struct Grant {
    slot: Slot,
    ready: listener::Ready,
    config: Arc<rustls::ServerConfig>,
}

/// Réserve un tunnel pour `relay`, avant de répondre `101` au relais.
pub fn grant(state: &AppState, relay: TargetId) -> Result<Grant, Refusal> {
    let tls = state.agent_tls();
    let ready = tls.ready().ok_or(Refusal::Closed)?;
    let config = tls.server_config().ok_or(Refusal::Closed)?;
    let slot = tls.reserve_tunnel(relay).ok_or(Refusal::Busy)?;
    Ok(Grant { slot, ready, config })
}

impl Grant {
    /// Branche le flux mis à niveau sur le port des agents : poignée de main
    /// TLS de l'agent du site, puis service ordinaire.
    pub async fn attach<T>(self, upgraded: T, relay: TargetId, addr: SocketAddr)
    where
        T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let transport: Box<dyn Transport> = Box::new(Guarded { inner: upgraded, _slot: self.slot });
        if let Some(connection) =
            listener::handshake(self.config, transport, addr, Some(relay)).await
        {
            let _ = self.ready.send(connection).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_relay_cannot_hold_more_tunnels_than_its_ceiling_and_closing_frees_a_place() {
        let open = Arc::new(Mutex::new(HashMap::new()));
        let first = Slot::reserve(&open, 7, 2).expect("première place");
        let second = Slot::reserve(&open, 7, 2).expect("deuxième place");
        assert!(Slot::reserve(&open, 7, 2).is_none(), "plafond atteint");
        // Un autre relais a ses propres places.
        let other = Slot::reserve(&open, 8, 2).expect("autre relais");
        drop(first);
        let third = Slot::reserve(&open, 7, 2).expect("une place s'est libérée");
        drop((second, third, other));
        assert!(open.lock().unwrap().is_empty(), "plus rien d'ouvert");
    }
}
