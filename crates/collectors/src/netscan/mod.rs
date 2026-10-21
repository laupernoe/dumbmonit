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
//! La concurrence est bornée à deux niveaux : un sémaphore global plafonne les
//! sockets ouvertes simultanément ([`ScanConfig::max_concurrency`]), et les
//! couples hôte × port sont produits paresseusement, quelques hôtes à la fois,
//! quelques ports par hôte : un /24 × 69 ports ne matérialise jamais ses 17 000
//! sondes d'avance. Tout tient dans la tâche de l'appelant (aucun `spawn`), ce
//! qui convient à l'exécuteur mono-thread de l'agent. L'annulation est
//! coopérative et immédiate (le jeton réveille le balayage, qui abandonne les
//! sondes en vol), et les résultats partent au fil de l'eau sur un canal : rien
//! n'est accumulé au-delà de l'hôte en cours.

mod arp;
mod catalog;
mod match_engine;
mod ping;
mod ports;
mod probe;
#[cfg(test)]
mod sample_tests;
mod snmp_probe;

use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use dumbmonit_proto::{
    DiscoveredHost, DiscoveredService, Fingerprint, FingerprintProbe, MAX_DISCOVERY_HOSTS,
    ScanDepth, ScanPhase, ScanProgress,
};
use futures::StreamExt;
use hickory_resolver::{Resolver, TokioResolver};
use ipnet::IpNet;
use tokio::sync::{Notify, Semaphore, mpsc};
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
    /// Sockets ouvertes simultanément, tous hôtes confondus (TCP et SNMP). Une
    /// sonde de port tient une seule socket à la fois : ce plafond est donc
    /// aussi celui des connexions. L'agent relais le baisse à
    /// [`AGENT_MAX_CONCURRENCY`].
    pub max_concurrency: usize,
}

/// Plafond de sockets simultanées conseillé pour un agent relais : une machine
/// modeste, un service systemd à mémoire bornée.
pub const AGENT_MAX_CONCURRENCY: usize = 64;

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            depth: ScanDepth::Standard,
            community: "public".to_string(),
            connect_timeout: Duration::from_millis(700),
            banner_timeout: Duration::from_millis(400),
            snmp_timeout: Duration::from_millis(900),
            ping_timeout: Duration::from_millis(600),
            max_concurrency: 128,
        }
    }
}

impl ScanConfig {
    fn limits(&self) -> Limits {
        let sockets = self.max_concurrency.max(1);
        Limits {
            sockets,
            // Assez d'hôtes en vol pour garder les permis occupés pendant qu'un
            // hôte attend ses délais, pas au point de créer des milliers de
            // futures en attente du sémaphore.
            hosts: sockets.div_ceil(4).clamp(1, 32),
            ports_per_host: sockets.min(16),
        }
    }
}

/// Bornes effectives d'un balayage : au plus `hosts × ports_per_host` sondes en
/// vol (en mémoire), dont au plus `sockets` ont une socket ouverte.
#[derive(Debug, Clone, Copy)]
struct Limits {
    sockets: usize,
    hosts: usize,
    ports_per_host: usize,
}

/// Une mise à jour poussée pendant le balayage.
#[derive(Debug, Clone)]
pub enum ScanUpdate {
    Progress(ScanProgress),
    /// Un hôte vivant : service reconnu, MAC connue ou réponse au ping.
    Host(DiscoveredHost),
}

#[derive(Debug, Default)]
struct CancelState {
    flag: AtomicBool,
    notify: Notify,
}

/// Jeton d'annulation, partagé avec l'appelant. Le balayage le relit avant
/// chaque sonde et l'attend en parallèle : une annulation abandonne aussitôt
/// les sondes en vol, sans attendre leurs délais.
#[derive(Debug, Default, Clone)]
pub struct Cancel(Arc<CancelState>);

impl Cancel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.flag.store(true, Ordering::Release);
        self.0.notify.notify_waiters();
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.flag.load(Ordering::Acquire)
    }

    /// Se termine dès que l'annulation est demandée, immédiatement si elle l'a
    /// déjà été.
    pub async fn cancelled(&self) {
        loop {
            // `notified()` est créé avant de relire le drapeau : un
            // `notify_waiters` survenu entre les deux n'est pas perdu.
            let notified = self.0.notify.notified();
            if self.is_cancelled() {
                return;
            }
            notified.await;
        }
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

/// Balaie `cidr` et pousse hôtes et progression sur `sink`, au fil de l'eau.
/// Rend le nombre d'hôtes trouvés. S'arrête aussitôt que `cancel` est levé ou
/// que plus personne n'écoute `sink`.
pub async fn scan(
    cidr: &str,
    config: &ScanConfig,
    fingerprints: Arc<Vec<Fingerprint>>,
    sink: mpsc::Sender<ScanUpdate>,
    cancel: Cancel,
) -> Result<usize, ScanError> {
    let hosts = hosts_of(cidr)?;
    let total = hosts.len();
    let tcp = tcp_ports(config.depth, &fingerprints);
    let limits = config.limits();
    info!(
        %cidr,
        hosts = total,
        ports = tcp.len(),
        max_concurrency = limits.sockets,
        "network discovery started"
    );

    send(&sink, &cancel, ScanUpdate::Progress(progress(ScanPhase::Sweep, 0, total))).await;

    // Hôtes vivants : ping d'abord (il réveille aussi la table ARP du noyau, lue
    // ensuite). Sans ICMP permis, l'ensemble est vide et on continue.
    let alive = tokio::select! {
        biased;
        _ = cancel.cancelled() => HashSet::new(),
        alive = ping::sweep(&hosts, config.ping_timeout, limits.sockets, &cancel) => alive,
    };
    let known = Known { alive, arp: ArpTable::load().await };
    debug!(alive = known.alive.len(), "ping sweep done");

    let prober =
        NetProber { config, fingerprints: fingerprints.as_slice(), resolver: system_resolver() };
    let found = sweep_hosts(hosts, &tcp, &prober, &known, limits, &sink, &cancel).await;

    send(&sink, &cancel, ScanUpdate::Progress(progress(ScanPhase::Done, total, total))).await;
    info!(%cidr, found, cancelled = cancel.is_cancelled(), "network discovery done");
    Ok(found)
}

fn progress(phase: ScanPhase, probed: usize, total: usize) -> ScanProgress {
    ScanProgress { phase, probed, total }
}

/// Pousse une mise à jour. Un canal fermé signifie que plus personne n'écoute :
/// le balayage s'annule plutôt que de travailler pour rien.
async fn send(sink: &mpsc::Sender<ScanUpdate>, cancel: &Cancel, update: ScanUpdate) {
    if sink.send(update).await.is_err() {
        cancel.cancel();
    }
}

/// Ce que l'on sait d'avance des hôtes : réponses au ping, table ARP.
struct Known {
    alive: HashSet<IpAddr>,
    arp: ArpTable,
}

/// Les sondes d'un hôte, derrière une interface pour que les tests puissent
/// vérifier les bornes du pipeline sans réseau.
trait HostProber {
    /// Identification SNMP (une socket UDP).
    async fn snmp(&self, ip: IpAddr) -> Vec<DiscoveredService>;
    /// Sonde d'un port TCP (une socket à la fois).
    async fn port(&self, ip: IpAddr, port: u16) -> Option<Vec<DiscoveredService>>;
    /// DNS inverse d'un hôte retenu.
    async fn hostname(&self, ip: IpAddr) -> Option<String>;
}

/// Balaie les hôtes, quelques-uns à la fois, et pousse chacun dès qu'il est
/// terminé. Rend le nombre d'hôtes retenus. Abandonne tout dès l'annulation.
async fn sweep_hosts<P: HostProber>(
    hosts: Vec<IpAddr>,
    tcp: &[u16],
    prober: &P,
    known: &Known,
    limits: Limits,
    sink: &mpsc::Sender<ScanUpdate>,
    cancel: &Cancel,
) -> usize {
    let total = hosts.len();
    let permits = Semaphore::new(limits.sockets.max(1));
    let permits = &permits;
    let mut found = 0usize;

    let work = async {
        // `iter` + `buffer_unordered` : un hôte n'est entamé que lorsqu'une
        // place se libère, jamais tous d'avance.
        let mut results = futures::stream::iter(hosts)
            .map(move |ip| scan_host(ip, tcp, prober, permits, limits.ports_per_host, cancel))
            .buffer_unordered(limits.hosts.max(1));
        let mut probed = 0usize;
        while let Some((ip, services)) = results.next().await {
            probed += 1;
            let update = ScanUpdate::Progress(progress(ScanPhase::Sweep, probed, total));
            send(sink, cancel, update).await;
            if let Some(host) = finish_host(ip, services, known, prober).await {
                found += 1;
                send(sink, cancel, ScanUpdate::Host(host)).await;
            }
            if cancel.is_cancelled() {
                break;
            }
            // Point d'attente explicite : sur l'exécuteur mono-thread de
            // l'agent, une rafale de ports refusés (réponses immédiates) ne doit
            // pas affamer les autres tâches.
            tokio::task::yield_now().await;
        }
    };
    tokio::select! {
        biased;
        _ = cancel.cancelled() => {}
        _ = work => {}
    }
    found
}

/// Balaie un hôte : SNMP, puis ses ports TCP, quelques-uns à la fois.
async fn scan_host<P: HostProber>(
    ip: IpAddr,
    tcp: &[u16],
    prober: &P,
    permits: &Semaphore,
    ports_per_host: usize,
    cancel: &Cancel,
) -> (IpAddr, Vec<DiscoveredService>) {
    let mut services = Vec::new();

    // SNMP d'abord : il tient dans un permis et ne dépend pas des ports TCP.
    if let Ok(_permit) = permits.acquire().await
        && !cancel.is_cancelled()
    {
        services.extend(prober.snmp(ip).await);
    }

    let mut ports = futures::stream::iter(tcp.iter().copied())
        .map(move |port| async move {
            if cancel.is_cancelled() {
                return None;
            }
            let _permit = permits.acquire().await.ok()?;
            if cancel.is_cancelled() {
                return None;
            }
            prober.port(ip, port).await
        })
        .buffer_unordered(ports_per_host.max(1));
    while let Some(found) = ports.next().await {
        if let Some(found) = found {
            services.extend(found);
        }
        if cancel.is_cancelled() {
            break;
        }
    }

    (ip, match_engine::keep_best(services))
}

/// Construit l'hôte découvert, ou `None` s'il n'a donné aucun signe de vie.
async fn finish_host<P: HostProber>(
    ip: IpAddr,
    services: Vec<DiscoveredService>,
    known: &Known,
    prober: &P,
) -> Option<DiscoveredHost> {
    let mac = match ip {
        IpAddr::V4(v4) => known.arp.mac(v4).map(str::to_string),
        IpAddr::V6(_) => None,
    };
    if services.is_empty() && mac.is_none() && !known.alive.contains(&ip) {
        return None;
    }
    let vendor = mac.as_deref().and_then(vendor).map(str::to_string);
    Some(DiscoveredHost {
        ip: ip.to_string(),
        hostname: prober.hostname(ip).await,
        mac,
        vendor,
        services,
    })
}

/// Les vraies sondes réseau. Tout est emprunté : aucune copie des empreintes
/// par port ni par hôte.
struct NetProber<'a> {
    config: &'a ScanConfig,
    fingerprints: &'a [Fingerprint],
    /// Un seul résolveur par balayage (sa construction lit `resolv.conf`).
    resolver: Option<TokioResolver>,
}

impl HostProber for NetProber<'_> {
    async fn snmp(&self, ip: IpAddr) -> Vec<DiscoveredService> {
        // Plafond global : l'identification enchaîne plusieurs requêtes, et
        // retente en v1 si la v2c reste muette.
        let budget = self.config.snmp_timeout.saturating_mul(4);
        let identify = snmp_probe::identify(ip, &self.config.community, self.config.snmp_timeout);
        let Ok(Some(identity)) = tokio::time::timeout(budget, identify).await else {
            return Vec::new();
        };
        let ip_text = ip.to_string();
        self.fingerprints
            .iter()
            .filter(|f| matches!(f.probe, FingerprintProbe::Snmp { .. }))
            .filter_map(|fp| match_engine::eval_snmp(fp, &ip_text, &identity))
            .collect()
    }

    async fn port(&self, ip: IpAddr, port: u16) -> Option<Vec<DiscoveredService>> {
        scan_port(ip, &ip.to_string(), port, self.config, self.fingerprints).await
    }

    async fn hostname(&self, ip: IpAddr) -> Option<String> {
        reverse_dns(self.resolver.as_ref()?, ip).await
    }
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
    // La preuve peut tenir plusieurs corps HTTP : libérée avant les échanges.
    drop(evidence);

    // Empreintes à bannière provoquée (Redis PING, PostgreSQL SSLRequest…) :
    // chacune ouvre sa propre connexion, l'une après l'autre.
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

/// Résolveur système, construit une fois par balayage, à délais courts.
fn system_resolver() -> Option<TokioResolver> {
    let mut builder = Resolver::builder_tokio().ok()?;
    {
        let opts = builder.options_mut();
        opts.timeout = Duration::from_millis(600);
        opts.attempts = 1;
    }
    builder.build().ok()
}

/// DNS inverse d'un hôte, silencieux en cas d'échec. On ne tranche pas : un nom
/// absent reste `None`.
async fn reverse_dns(resolver: &TokioResolver, ip: IpAddr) -> Option<String> {
    use hickory_resolver::proto::rr::{RData, RecordType};
    let name = match ip {
        IpAddr::V4(v4) => {
            let [a, b, c, d] = v4.octets();
            format!("{d}.{c}.{b}.{a}.in-addr.arpa.")
        }
        IpAddr::V6(_) => return None,
    };
    let lookup =
        tokio::time::timeout(Duration::from_millis(800), resolver.lookup(name, RecordType::PTR))
            .await
            .ok()?
            .ok()?;
    lookup.answers().iter().find_map(|record| match &record.data {
        RData::PTR(ptr) => Some(ptr.0.to_string().trim_end_matches('.').to_string()),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

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
    async fn cancelled_wakes_waiters_and_returns_at_once_afterwards() {
        let cancel = Cancel::new();
        let waiter = {
            let cancel = cancel.clone();
            tokio::spawn(async move { cancel.cancelled().await })
        };
        tokio::task::yield_now().await;
        cancel.cancel();
        tokio::time::timeout(Duration::from_secs(2), waiter)
            .await
            .expect("le jeton doit réveiller ceux qui l'attendent")
            .unwrap();
        // Déjà levé : ne bloque pas.
        cancel.cancelled().await;
    }

    #[test]
    fn default_limits_stay_small() {
        let agent = ScanConfig { max_concurrency: AGENT_MAX_CONCURRENCY, ..ScanConfig::default() };
        let limits = agent.limits();
        assert_eq!(limits.sockets, 64);
        // Sondes en vol (en mémoire) bornées, loin des 254 × 69 d'un /24 étendu.
        assert!(limits.hosts * limits.ports_per_host <= 512);
        let zero = ScanConfig { max_concurrency: 0, ..ScanConfig::default() }.limits();
        assert_eq!((zero.sockets, zero.hosts, zero.ports_per_host), (1, 1, 1));
    }

    /// Sonde factice : compte les sondes simultanées et le pic atteint.
    #[derive(Default)]
    struct CountingProber {
        active: AtomicUsize,
        peak: AtomicUsize,
        calls: AtomicUsize,
        hold: Duration,
        /// Annule le balayage à la n-ième sonde (0 = jamais).
        cancel_at: usize,
        cancel: Cancel,
    }

    impl CountingProber {
        async fn occupy(&self) {
            let now = self.active.fetch_add(1, Ordering::SeqCst) + 1;
            self.peak.fetch_max(now, Ordering::SeqCst);
            let calls = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
            if self.cancel_at != 0 && calls == self.cancel_at {
                self.cancel.cancel();
            }
            tokio::time::sleep(self.hold).await;
            self.active.fetch_sub(1, Ordering::SeqCst);
        }
    }

    impl HostProber for CountingProber {
        async fn snmp(&self, _ip: IpAddr) -> Vec<DiscoveredService> {
            self.occupy().await;
            Vec::new()
        }

        async fn port(&self, _ip: IpAddr, _port: u16) -> Option<Vec<DiscoveredService>> {
            self.occupy().await;
            None
        }

        async fn hostname(&self, _ip: IpAddr) -> Option<String> {
            None
        }
    }

    fn nobody_known() -> Known {
        Known { alive: HashSet::new(), arp: ArpTable::default() }
    }

    #[tokio::test(start_paused = true)]
    async fn simultaneous_probes_never_exceed_the_socket_limit() {
        let hosts = hosts_of("192.0.2.0/26").unwrap();
        let ports: Vec<u16> = (1..=40).collect();
        let prober = CountingProber { hold: Duration::from_millis(20), ..Default::default() };
        let limits = Limits { sockets: 8, hosts: 4, ports_per_host: 8 };
        // Canal plus petit que le nombre de mises à jour : il est vidé à côté.
        let (tx, mut rx) = mpsc::channel(16);
        let drain = tokio::spawn(async move { while rx.recv().await.is_some() {} });

        let cancel = Cancel::new();
        let (count, known) = (hosts.len(), nobody_known());
        let found = sweep_hosts(hosts, &ports, &prober, &known, limits, &tx, &cancel).await;
        drop(tx);
        drain.await.unwrap();

        assert_eq!(found, 0);
        // Chaque hôte : une sonde SNMP et une par port, toutes menées.
        assert_eq!(prober.calls.load(Ordering::SeqCst), count * (ports.len() + 1));
        let peak = prober.peak.load(Ordering::SeqCst);
        assert!(peak <= limits.sockets, "pic de {peak} sondes pour {} permis", limits.sockets);
        assert!(peak > 1, "les sondes doivent tout de même se chevaucher");
    }

    #[tokio::test(start_paused = true)]
    async fn a_cancelled_sweep_stops_without_waiting_for_probes() {
        let hosts = hosts_of("192.0.2.0/24").unwrap();
        let ports: Vec<u16> = (1..=69).collect();
        let cancel = Cancel::new();
        // Chaque sonde durerait une heure : seule l'annulation peut finir vite.
        // Annulation à la 10e sonde, parmi les 16 SNMP entamées d'emblée (avant
        // que le temps virtuel n'avance).
        let prober = CountingProber {
            hold: Duration::from_secs(3600),
            cancel_at: 10,
            cancel: cancel.clone(),
            ..Default::default()
        };
        let limits = Limits { sockets: 64, hosts: 16, ports_per_host: 16 };
        let (tx, mut rx) = mpsc::channel(256);
        let drain = tokio::spawn(async move { while rx.recv().await.is_some() {} });

        let started = tokio::time::Instant::now();
        sweep_hosts(hosts, &ports, &prober, &nobody_known(), limits, &tx, &cancel).await;
        assert!(started.elapsed() < Duration::from_secs(1), "{:?}", started.elapsed());
        // Plus aucune sonde n'est entamée après l'annulation.
        assert!(prober.calls.load(Ordering::SeqCst) <= limits.sockets);
        drop(tx);
        drain.await.unwrap();
    }

    #[tokio::test]
    async fn a_closed_sink_cancels_the_sweep() {
        let hosts = hosts_of("192.0.2.0/24").unwrap();
        let ports: Vec<u16> = (1..=8).collect();
        let prober = CountingProber { hold: Duration::from_millis(1), ..Default::default() };
        let limits = Limits { sockets: 4, hosts: 2, ports_per_host: 2 };
        let (tx, rx) = mpsc::channel(4);
        drop(rx);
        let cancel = Cancel::new();
        sweep_hosts(hosts, &ports, &prober, &nobody_known(), limits, &tx, &cancel).await;
        assert!(cancel.is_cancelled());
        // Arrêt après le premier hôte terminé (au plus `hosts` entamés).
        assert!(prober.calls.load(Ordering::SeqCst) <= limits.hosts * (ports.len() + 1));
    }

    #[tokio::test]
    async fn a_real_scan_stops_soon_after_cancel() {
        // 192.0.2.0/24 (RFC 5737) : personne ne répond, chaque sonde irait au
        // bout de son délai.
        let config = ScanConfig {
            connect_timeout: Duration::from_secs(5),
            banner_timeout: Duration::from_secs(5),
            snmp_timeout: Duration::from_secs(5),
            ping_timeout: Duration::from_millis(50),
            max_concurrency: 16,
            ..ScanConfig::default()
        };
        let (tx, mut rx) = mpsc::channel(256);
        let cancel = Cancel::new();
        let fingerprints = Arc::new(generic_fingerprints());
        let scan = {
            let cancel = cancel.clone();
            tokio::spawn(
                async move { scan("192.0.2.0/24", &config, fingerprints, tx, cancel).await },
            )
        };
        tokio::time::sleep(Duration::from_millis(100)).await;
        cancel.cancel();
        let found = tokio::time::timeout(Duration::from_secs(3), scan)
            .await
            .expect("le balayage doit s'arrêter peu après l'annulation")
            .unwrap()
            .unwrap();
        assert_eq!(found, 0);
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
        assert_eq!(found, 0);
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
