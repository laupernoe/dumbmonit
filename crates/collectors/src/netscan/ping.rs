//! Détection des hôtes vivants par écho ICMP, avant le balayage des ports.
//!
//! Beaucoup d'équipements n'ouvrent aucun port courant (téléphone, ampoule,
//! console) mais répondent au ping : sans cette passe, ils seraient absents du
//! résultat. On réutilise la socket ICMP non privilégiée du collecteur `ping`
//! (`surge-ping`, plage `net.ipv4.ping_group_range` ou `CAP_NET_RAW`). Si le
//! système la refuse, on rend simplement un ensemble vide : le balayage continue
//! sans, jamais d'échec.

use std::collections::HashSet;
use std::net::IpAddr;
use std::time::Duration;

use futures::StreamExt;
use surge_ping::{Client, Config, PingIdentifier, PingSequence};
use tracing::debug;

use super::Cancel;

/// Échos envoyés par adresse avant de la déclarer muette.
const ATTEMPTS: u16 = 2;

/// Envoie des échos à chaque adresse IPv4 de `hosts` (concurrence bornée par
/// `concurrency`) et rend celles qui ont répondu. Vide si l'ICMP n'est pas permis.
pub async fn sweep(
    hosts: &[IpAddr],
    timeout: Duration,
    concurrency: usize,
    cancel: &Cancel,
) -> HashSet<IpAddr> {
    let targets: Vec<IpAddr> = hosts.iter().copied().filter(IpAddr::is_ipv4).collect();
    if targets.is_empty() || cancel.is_cancelled() {
        return HashSet::new();
    }
    let client = match Client::new(&Config::default()) {
        Ok(client) => client,
        Err(error) => {
            debug!(%error, "ICMP indisponible, découverte sans ping");
            return HashSet::new();
        }
    };

    futures::stream::iter(targets.into_iter().enumerate())
        .map(|(index, ip)| {
            let client = client.clone();
            async move {
                if cancel.is_cancelled() {
                    return None;
                }
                alive(&client, ip, index as u16, timeout).await.then_some(ip)
            }
        })
        .buffer_unordered(concurrency.max(1))
        .filter_map(|ip| async move { ip })
        .collect()
        .await
}

/// Vrai si l'adresse répond à l'un des échos.
async fn alive(client: &Client, ip: IpAddr, ident: u16, timeout: Duration) -> bool {
    let mut pinger = client.pinger(ip, PingIdentifier(ident)).await;
    pinger.timeout(timeout);
    for sequence in 0..ATTEMPTS {
        if pinger.ping(PingSequence(sequence), &[0u8; 16]).await.is_ok() {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn nothing_to_ping_or_cancelled_gives_an_empty_set() {
        let cancel = Cancel::new();
        assert!(sweep(&[], Duration::from_millis(50), 8, &cancel).await.is_empty());
        cancel.cancel();
        let hosts: Vec<IpAddr> = vec!["192.0.2.1".parse().unwrap()];
        assert!(sweep(&hosts, Duration::from_millis(50), 8, &cancel).await.is_empty());
    }
}
