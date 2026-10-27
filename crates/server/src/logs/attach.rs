//! Rattache l'émetteur d'un message (syslog, trap) à une cible, par son adresse.
//!
//! La table est reconstruite chaque minute depuis les cibles : une adresse IP
//! littérale est reprise telle quelle, un nom d'hôte est résolu (court délai, au
//! plus cent noms). Quand plusieurs cibles partagent une adresse, l'agent puis
//! l'équipement SNMP priment sur les sondes (HTTP, ping…), puis la plus ancienne.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{OnceLock, RwLock};
use std::time::Duration;

use sqlx::SqlitePool;

/// Cible retrouvée : identifiant et nom.
pub type Found = (i64, String);

#[derive(Default)]
pub struct Resolver {
    by_ip: RwLock<HashMap<IpAddr, Found>>,
    by_name: RwLock<HashMap<String, Found>>,
}

static RESOLVER: OnceLock<Resolver> = OnceLock::new();

pub fn resolver() -> &'static Resolver {
    RESOLVER.get_or_init(Resolver::default)
}

impl Resolver {
    /// La cible d'une adresse, sinon celle dont le nom est `hint`.
    pub fn lookup(&self, ip: IpAddr, hint: Option<&str>) -> Option<Found> {
        let ip = ip.to_canonical();
        if let Some(found) = self.by_ip.read().ok().and_then(|map| map.get(&ip).cloned()) {
            return Some(found);
        }
        let hint = hint?.trim().to_lowercase();
        if hint.is_empty() {
            return None;
        }
        let short = hint.split('.').next().unwrap_or("").to_string();
        let map = self.by_name.read().ok()?;
        map.get(&hint).or_else(|| map.get(&short)).cloned()
    }

    fn replace(&self, by_ip: HashMap<IpAddr, Found>, by_name: HashMap<String, Found>) {
        if let Ok(mut held) = self.by_ip.write() {
            *held = by_ip;
        }
        if let Ok(mut held) = self.by_name.write() {
            *held = by_name;
        }
    }
}

/// L'hôte d'une adresse de cible : sans schéma, identifiants, chemin ni port.
pub fn host_part(address: &str) -> String {
    let mut rest = address.trim();
    if let Some((_, after)) = rest.split_once("://") {
        rest = after;
    }
    let rest = rest.split(['/', '?', '#']).next().unwrap_or("");
    let rest = rest.rsplit('@').next().unwrap_or("");
    if let Some(stripped) = rest.strip_prefix('[') {
        return stripped.split(']').next().unwrap_or("").to_string();
    }
    // Une IPv6 nue contient plusieurs deux-points : pas de port à retirer.
    if rest.matches(':').count() == 1 {
        return rest.split(':').next().unwrap_or("").to_string();
    }
    rest.to_string()
}

fn priority(kind: &str) -> u8 {
    match kind {
        "agent" => 0,
        "snmp" => 1,
        _ => 2,
    }
}

/// Reconstruit la table depuis les cibles.
pub async fn refresh(pool: &SqlitePool) -> anyhow::Result<()> {
    let rows: Vec<(i64, String, String, String)> =
        sqlx::query_as("SELECT id, name, address, kind FROM targets ORDER BY id")
            .fetch_all(pool)
            .await?;
    let mut by_ip: HashMap<IpAddr, (u8, Found)> = HashMap::new();
    let mut by_name: HashMap<String, (u8, Found)> = HashMap::new();
    let mut resolved = 0usize;
    for (id, name, address, kind) in rows {
        let rank = priority(&kind);
        let found: Found = (id, name.clone());
        let host = host_part(&address);
        if host.is_empty() {
            continue;
        }
        let mut addresses: Vec<IpAddr> = Vec::new();
        if let Ok(ip) = host.parse::<IpAddr>() {
            addresses.push(ip.to_canonical());
        } else if resolved < 100 && !host.contains(' ') && host.len() < 254 {
            resolved += 1;
            let lookup = tokio::net::lookup_host((host.as_str(), 0));
            if let Ok(Ok(found_ips)) = tokio::time::timeout(Duration::from_secs(1), lookup).await {
                addresses.extend(found_ips.map(|socket| socket.ip().to_canonical()));
            }
        }
        for ip in addresses {
            let better = by_ip.get(&ip).is_none_or(|(held, _)| rank < *held);
            if better {
                by_ip.insert(ip, (rank, found.clone()));
            }
        }
        for key in [name.to_lowercase(), host.to_lowercase()] {
            let better = by_name.get(&key).is_none_or(|(held, _)| rank < *held);
            if better {
                by_name.insert(key, (rank, found.clone()));
            }
        }
    }
    resolver().replace(
        by_ip.into_iter().map(|(ip, (_, found))| (ip, found)).collect(),
        by_name.into_iter().map(|(name, (_, found))| (name, found)).collect(),
    );
    Ok(())
}

/// Tient la table à jour tant que le serveur tourne.
pub fn spawn(pool: SqlitePool) {
    tokio::spawn(async move {
        loop {
            if let Err(error) = refresh(&pool).await {
                tracing::warn!(error = %format!("{error:#}"), "log sender table not refreshed");
            }
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_host_is_extracted_from_any_address_form() {
        assert_eq!(host_part("192.168.1.10"), "192.168.1.10");
        assert_eq!(host_part("https://nas.lan:5001/path?x=1"), "nas.lan");
        assert_eq!(host_part("user:pw@10.0.0.2:8080"), "10.0.0.2");
        assert_eq!(host_part("[fe80::1]:443"), "fe80::1");
        assert_eq!(host_part("fe80::1"), "fe80::1");
        assert_eq!(host_part("router.lan:161"), "router.lan");
    }

    #[test]
    fn a_sender_is_found_by_address_then_by_name() {
        let resolver = Resolver::default();
        let mut by_ip = HashMap::new();
        by_ip.insert("10.0.0.5".parse().unwrap(), (3, "NAS".to_string()));
        let mut by_name = HashMap::new();
        by_name.insert("switch".to_string(), (4, "Switch".to_string()));
        resolver.replace(by_ip, by_name);
        assert_eq!(resolver.lookup("10.0.0.5".parse().unwrap(), None).unwrap().0, 3);
        assert_eq!(resolver.lookup("::ffff:10.0.0.5".parse().unwrap(), None).unwrap().0, 3);
        assert!(resolver.lookup("10.9.9.9".parse().unwrap(), None).is_none());
        let by_hint = resolver.lookup("10.9.9.9".parse().unwrap(), Some("Switch.lan")).unwrap();
        assert_eq!(by_hint.0, 4);
    }
}
