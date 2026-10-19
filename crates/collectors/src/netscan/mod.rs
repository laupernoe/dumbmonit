//! Moteur de découverte réseau, partagé par le serveur et l'agent relais.
//!
//! Trois temps : une détection des hôtes vivants par ping ICMP (si le système
//! l'autorise) et par la table ARP, un balayage TCP connect d'une liste de ports courants (plus
//! SNMP en UDP), puis une empreinte anonyme sur chaque port ouvert (requête
//! HTTP sans vérifier le certificat, bannière, identification SNMP). Jamais de
//! tentative d'authentification. La reconnaissance est pilotée par les
//! empreintes que les collecteurs déclarent ([`Fingerprint`]) : le moteur ne
//! connaît aucun produit.
//!
//! La concurrence est bornée (un sémaphore global de connexions), l'annulation
//! est propre (drapeau partagé relu entre deux hôtes), et la progression est
//! poussée au fil de l'eau sur un canal : l'interface voit les hôtes apparaître
//! sans attendre la fin.

mod arp;
mod catalog;
mod match_engine;
mod ping;
mod ports;
mod probe;
#[cfg(test)]
mod sample_tests;
mod snmp_probe;

use std::net::IpAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use dumbmonit_proto::{
    DiscoveredHost, DiscoveredService, Fingerprint, FingerprintProbe, MAX_DISCOVERY_HOSTS,
    ScanDepth, ScanPhase, ScanProgress,
};
use ipnet::IpNet;
use tokio::sync::{Semaphore, mpsc};
use tracing::{debug, info};

pub use arp::{ArpTable, vendor};
pub use catalog::all as all_fingerprints;
pub use ports::{generic_fingerprints, tcp_ports};

/// Réglages d'un balayage.
#[derive(Debug, Clone)]
pub struct ScanConfig {
    pub depth: ScanDepth,
    /// Community SNMP essayée (jamais journalisée).
    pub community: String,
    /// Délai d'une connexion TCP et d'une requête.
    pub connect_timeout: Duration,
    /// Délai court de lecture d'une bannière spontanée.
    pub banner_timeout: Duration,
    /// Délai d'une identification SNMP.
    pub snmp_timeout: Duration,
    /// Délai d'attente d'un écho ICMP.
    pub ping_timeout: Duration,
    /// Connexions simultanées, tous hôtes confondus.
    pub concurrency: usize,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            depth: ScanDepth::Standard,
            community: "public".to_string(),
            connect_timeout: Duration::from_millis(700),
            banner_timeout: Duration::from_millis(400),
            snmp_timeout: Duration::from_millis(900),
            ping_timeout: Duration::from_millis(600),
            concurrency: 256,
        }
    }
}

/// Une mise à jour poussée pendant le balayage.
#[derive(Debug, Clone)]
pub enum ScanUpdate {
    Progress(ScanProgress),
    /// Un hôte vivant : service reconnu, MAC connue ou réponse au ping.
    Host(DiscoveredHost),
}

/// Jeton d'annulation : partagé avec l'appelant, relu entre deux hôtes.
#[derive(Debug, Default, Clone)]
pub struct Cancel(Arc<AtomicBool>);

impl Cancel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// Pourquoi un balayage n'a pas pu démarrer.
#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("invalid network \"{0}\"")]
    BadCidr(String),
    #[error("network too large: {0} addresses at most")]
    TooLarge(usize),
}

/// Les hôtes d'un réseau, borne de sécurité comprise. Erreur si le CIDR est
/// illisible ou dépasse [`MAX_DISCOVERY_HOSTS`].
pub fn hosts_of(cidr: &str) -> Result<Vec<IpAddr>, ScanError> {
    let network: IpNet = cidr.trim().parse().map_err(|_| ScanError::BadCidr(cidr.to_string()))?;
    let hosts: Vec<IpAddr> = network.hosts().take(MAX_DISCOVERY_HOSTS + 1).collect();
    if hosts.len() > MAX_DISCOVERY_HOSTS {
        return Err(ScanError::TooLarge(MAX_DISCOVERY_HOSTS));
    }
    Ok(hosts)
}

/// Balaie `cidr` et pousse hôtes et progression sur `sink`. Rend la liste
/// complète des hôtes trouvés (utile pour un appel synchrone ; l'interface, elle,
/// suit `sink`). S'arrête proprement dès que `cancel` est levé.
pub async fn scan(
    cidr: &str,
    config: &ScanConfig,
    fingerprints: Arc<Vec<Fingerprint>>,
    sink: mpsc::Sender<ScanUpdate>,
    cancel: Cancel,
) -> Result<Vec<DiscoveredHost>, ScanError> {
    let hosts = hosts_of(cidr)?;
    let total = hosts.len();
    let tcp = Arc::new(tcp_ports(config.depth, &fingerprints));
    info!(%cidr, hosts = total, ports = tcp.len(), "network discovery started");

    let permits = Arc::new(Semaphore::new(config.concurrency.max(1)));
    let probed = Arc::new(AtomicUsize::new(0));
    let config = Arc::new(config.clone());

    let _ = sink
        .send(ScanUpdate::Progress(ScanProgress { phase: ScanPhase::Sweep, probed: 0, total }))
        .await;

    // Hôtes vivants : ping d'abord (il réveille aussi la table ARP du noyau, lue
    // ensuite). Sans ICMP permis, l'ensemble est vide et on continue.
    let alive = Arc::new(ping::sweep(&hosts, config.ping_timeout, 128, &cancel).await);
    let arp = ArpTable::load().await;
    debug!(alive = alive.len(), "ping sweep done");

    let mut tasks = Vec::with_capacity(total);
    for ip in hosts {
        let alive = alive.clone();
        let permits = permits.clone();
        let tcp = tcp.clone();
        let config = config.clone();
        let fingerprints = fingerprints.clone();
        let sink = sink.clone();
        let cancel = cancel.clone();
        let probed = probed.clone();
        let mac = match ip {
            IpAddr::V4(v4) => arp.mac(v4).map(str::to_string),
            IpAddr::V6(_) => None,
        };
        tasks.push(tokio::spawn(async move {
            if cancel.is_cancelled() {
                return None;
            }
            let services = scan_host(ip, &tcp, &config, &fingerprints, &permits, &cancel).await;
            let done = probed.fetch_add(1, Ordering::Relaxed) + 1;
            let _ = sink
                .send(ScanUpdate::Progress(ScanProgress {
                    phase: ScanPhase::Sweep,
                    probed: done,
                    total,
                }))
                .await;

            if services.is_empty() && mac.is_none() && !alive.contains(&ip) {
                return None;
            }
            let vendor = mac.as_deref().and_then(vendor).map(str::to_string);
            let host = DiscoveredHost {
                ip: ip.to_string(),
                hostname: reverse_dns(ip, &config).await,
                mac,
                vendor,
                services,
            };
            let _ = sink.send(ScanUpdate::Host(host.clone())).await;
            Some(host)
        }));
    }

    let mut found = Vec::new();
    for task in tasks {
        if let Ok(Some(host)) = task.await {
            found.push(host);
        }
    }
    found.sort_by_key(|host| sort_key(&host.ip));
    let _ = sink
        .send(ScanUpdate::Progress(ScanProgress { phase: ScanPhase::Done, probed: total, total }))
        .await;
    info!(%cidr, found = found.len(), cancelled = cancel.is_cancelled(), "network discovery done");
    Ok(found)
}

/// Balaie un hôte : ports TCP (empreintes HTTP/bannière), puis SNMP.
async fn scan_host(
    ip: IpAddr,
    tcp: &[u16],
    config: &ScanConfig,
    fingerprints: &[Fingerprint],
    permits: &Arc<Semaphore>,
    cancel: &Cancel,
) -> Vec<DiscoveredService> {
    let mut services = Vec::new();

    // SNMP d'abord : il tient dans un permis et ne dépend pas des ports TCP.
    {
        let _permit = permits.acquire().await;
        if let Some(identity) =
            snmp_probe::identify(ip, &config.community, config.snmp_timeout).await
        {
            for fp in
                fingerprints.iter().filter(|f| matches!(f.probe, FingerprintProbe::Snmp { .. }))
            {
                if let Some(service) = match_engine::eval_snmp(fp, &ip.to_string(), &identity) {
                    services.push(service);
                }
            }
        }
    }

    let mut port_tasks = Vec::with_capacity(tcp.len());
    for &port in tcp {
        if cancel.is_cancelled() {
            break;
        }
        let permits = permits.clone();
        let config = config.clone();
        let fingerprints = fingerprints.to_vec();
        let ip_text = ip.to_string();
        port_tasks.push(tokio::spawn(async move {
            let _permit = permits.acquire().await.ok()?;
            scan_port(ip, &ip_text, port, &config, &fingerprints).await
        }));
    }
    for task in port_tasks {
        if let Ok(Some(mut found)) = task.await {
            services.append(&mut found);
        }
    }

    match_engine::keep_best(services)
}

/// Sonde un port et évalue les empreintes qui le visent.
async fn scan_port(
    ip: IpAddr,
    ip_text: &str,
    port: u16,
    config: &ScanConfig,
    fingerprints: &[Fingerprint],
) -> Option<Vec<DiscoveredService>> {
    // Empreintes candidates : celles qui visent ce port, plus les HTTP
    // génériques (ports vides) si le port peut parler HTTP.
    let candidates: Vec<&Fingerprint> = fingerprints
        .iter()
        .filter(|fp| {
            if fp.ports.contains(&port) {
                return true;
            }
            fp.ports.is_empty() && fp.probe.is_http() && ports::http_allowed(port)
        })
        .collect();
    if candidates.is_empty() {
        return None;
    }

    // Les chemins HTTP à interroger, sans doublon : ceux des empreintes HTTP
    // candidates, à condition que le port tolère une requête HTTP.
    let mut paths: Vec<String> = Vec::new();
    if ports::http_allowed(port) {
        for fp in &candidates {
            if let FingerprintProbe::Http { path, .. } = &fp.probe {
                let path = if path.is_empty() { "/" } else { path.as_str() };
                if !paths.iter().any(|p| p == path) {
                    paths.push(path.to_string());
                }
            }
        }
    }
    let want_banner = candidates.iter().any(|fp| {
        matches!(&fp.probe, FingerprintProbe::Banner { send: None, .. } | FingerprintProbe::Port)
    });
    // Délai de lecture de bannière : court sur un port HTTP (il ne dira rien),
    // plus long sur un port à bannière spontanée.
    let banner_to = if paths.is_empty() { config.connect_timeout } else { config.banner_timeout };

    let evidence =
        probe::observe(ip, port, config.connect_timeout, banner_to, &paths, want_banner).await;
    let evidence = evidence?;

    let mut services = Vec::new();
    for fp in &candidates {
        if let Some(service) = match_engine::eval_tcp(fp, ip_text, port, &evidence) {
            services.push(service);
        }
    }

    // Empreintes à bannière provoquée (Redis PING, PostgreSQL SSLRequest…) :
    // chacune ouvre sa propre connexion.
    for fp in &candidates {
        if let FingerprintProbe::Banner { send: Some(send), .. } = &fp.probe
            && let Some(response) =
                probe::exchange(ip, port, send, banner_to.max(config.connect_timeout)).await
            && let Some(service) = match_engine::eval_exchange(fp, ip_text, port, &response)
        {
            services.push(service);
        }
    }

    debug!(%ip, port, services = services.len(), "port probed");
    (!services.is_empty()).then_some(services)
}

/// DNS inverse d'un hôte, silencieux en cas d'échec. On ne tranche pas : un nom
/// absent reste `None`.
async fn reverse_dns(ip: IpAddr, _config: &ScanConfig) -> Option<String> {
    use hickory_resolver::Resolver;
    use hickory_resolver::proto::rr::{RData, RecordType};
    let name = match ip {
        IpAddr::V4(v4) => {
            let [a, b, c, d] = v4.octets();
            format!("{d}.{c}.{b}.{a}.in-addr.arpa.")
        }
        IpAddr::V6(_) => return None,
    };
    let resolver = Resolver::builder_tokio().ok()?.build().ok()?;
    let lookup =
        tokio::time::timeout(Duration::from_millis(600), resolver.lookup(name, RecordType::PTR))
            .await
            .ok()?
            .ok()?;
    lookup.answers().iter().find_map(|record| match &record.data {
        RData::PTR(ptr) => Some(ptr.0.to_string().trim_end_matches('.').to_string()),
        _ => None,
    })
}

/// Clé de tri d'une IP : IPv4 par octets, pour un affichage naturel.
fn sort_key(ip: &str) -> (u8, u128) {
    match ip.parse::<IpAddr>() {
        Ok(IpAddr::V4(v4)) => (0, u32::from(v4) as u128),
        Ok(IpAddr::V6(v6)) => (1, u128::from(v6)),
        Err(_) => (2, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosts_of_rejects_a_too_wide_network() {
        assert!(matches!(hosts_of("10.0.0.0/8"), Err(ScanError::TooLarge(_))));
        assert!(matches!(hosts_of("not-a-cidr"), Err(ScanError::BadCidr(_))));
        assert_eq!(hosts_of("192.168.1.0/24").unwrap().len(), 254);
        // Un /22 est la borne haute (1024 adresses, 1022 hôtes).
        assert!(hosts_of("10.0.0.0/22").is_ok());
    }

    #[test]
    fn cancel_is_observed() {
        let cancel = Cancel::new();
        assert!(!cancel.is_cancelled());
        cancel.cancel();
        assert!(cancel.is_cancelled());
    }

    #[tokio::test]
    async fn an_empty_network_scan_reports_done() {
        // 192.0.2.0/30 (RFC 5737) : personne ne répond.
        let (tx, mut rx) = mpsc::channel(64);
        let config = ScanConfig {
            connect_timeout: Duration::from_millis(60),
            banner_timeout: Duration::from_millis(30),
            snmp_timeout: Duration::from_millis(60),
            ping_timeout: Duration::from_millis(60),
            ..ScanConfig::default()
        };
        let fingerprints = Arc::new(generic_fingerprints());
        let found = scan("192.0.2.0/30", &config, fingerprints, tx, Cancel::new()).await.unwrap();
        assert!(found.is_empty());
        let mut saw_done = false;
        while let Some(update) = rx.recv().await {
            if let ScanUpdate::Progress(p) = update
                && p.phase == ScanPhase::Done
            {
                saw_done = true;
            }
        }
        assert!(saw_done);
    }
}
