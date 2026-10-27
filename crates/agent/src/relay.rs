//! Mode relais : interroger, pour le serveur, les équipements de notre réseau.
//!
//! Le serveur ne joint jamais l'agent. Une tâche de fond lui demande en boucle
//! s'il a des sondes à confier (attente longue côté serveur, réponse immédiate
//! dès qu'une cible est échue), les exécute avec les mêmes collecteurs que lui —
//! SNMP, Proxmox, PBS, Synology, HTTP, TCP, DNS, ping, TLS — et rapporte mesures
//! et verdict. Le serveur range tout cela sous l'équipement sondé, exactement
//! comme s'il l'avait interrogé lui-même.
//!
//! Indépendant de la boucle de collecte : une sonde lente ne retarde jamais les
//! mesures de la machine, et inversement.

use std::sync::Arc;
use std::time::Duration;

use dumbmonit_collectors::Registry;
use dumbmonit_proto::{AgentCommand, ProbeJob, ProbeOutcome, RELAY_POLL_HOLD_SECS};
use tokio::sync::Semaphore;
use tokio::time::Instant;
use tracing::{debug, info, warn};

use crate::backoff::Backoff;
use crate::client::PushClient;
use crate::shutdown::Shutdown;

/// Attente longue demandée au serveur. Le client lui laisse une marge pour
/// répondre au-delà.
const POLL_WAIT: Duration = Duration::from_secs(RELAY_POLL_HOLD_SECS);
const POLL_TIMEOUT: Duration = Duration::from_secs(RELAY_POLL_HOLD_SECS + 20);

/// Première temporisation quand le serveur ne répond pas, et son plafond : les
/// mêmes que pour les mesures (`run.rs`), sous le délai au-delà duquel le
/// serveur déclare les équipements du site injoignables. Une réponse favorable
/// sur l'autre canal y met fin aussitôt.
const BACKOFF_BASE: Duration = Duration::from_secs(2);
const BACKOFF_MAX: Duration = Duration::from_secs(30);

/// Sondes menées de front. Un site ordinaire compte quelques dizaines
/// d'équipements ; les mener toutes à la fois saturerait la liaison d'un petit
/// site plutôt que de l'aider.
const MAX_CONCURRENT_PROBES: usize = 8;

/// Délai maximal d'une sonde, quelle que soit la demande du serveur : un
/// collecteur qui ne répond pas doit rendre la main.
const MAX_PROBE_TIMEOUT: Duration = Duration::from_secs(120);

/// Délai laissé aux sondes en cours à l'arrêt.
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(15);

/// Boucle du relais, à lancer dans sa propre tâche.
pub struct RelayRunner {
    client: PushClient,
    key: String,
    registry: Arc<Registry>,
    permits: Arc<Semaphore>,
    backoff: Backoff,
}

impl RelayRunner {
    pub fn new(client: PushClient, key: String, request_timeout: Duration) -> Self {
        Self {
            client,
            key,
            registry: Arc::new(Registry::remote(request_timeout)),
            permits: Arc::new(Semaphore::new(MAX_CONCURRENT_PROBES)),
            backoff: Backoff::new(BACKOFF_BASE, BACKOFF_MAX),
        }
    }

    /// Types d'équipements que ce relais sait interroger.
    pub fn kinds(&self) -> Vec<Arc<str>> {
        self.registry.kinds()
    }

    pub async fn run(mut self, mut shutdown: Shutdown) {
        info!(kinds = ?self.kinds(), "relay mode enabled: waiting for probes from the server");
        let reachable = self.client.reachable();
        let mut down_since: Option<Instant> = None;
        loop {
            // Après une panne, un simple coup d'œil sans attente : la reprise se
            // constate (et se journalise) aussitôt, pas au bout d'une attente
            // longue de 25 s.
            let wait = if down_since.is_some() { Duration::ZERO } else { POLL_WAIT };
            let fetch = self.client.fetch_probes(&self.key, wait, POLL_TIMEOUT);
            let jobs = tokio::select! {
                biased;
                _ = shutdown.wait() => break,
                result = fetch => result,
            };
            match jobs {
                Ok(jobs) => {
                    if let Some(since) = down_since.take() {
                        info!(
                            attempts = self.backoff.attempts(),
                            outage_s = since.elapsed().as_secs(),
                            "relay channel restored"
                        );
                    }
                    self.backoff.reset();
                    for job in jobs {
                        self.spawn(job, &shutdown);
                    }
                }
                Err(error) => {
                    let delay = self.backoff.next_delay();
                    let attempt = self.backoff.attempts();
                    let retry_in_s = delay.as_secs();
                    if down_since.is_none() {
                        down_since = Some(Instant::now());
                        warn!(%error, retry_in_s, "cannot fetch relayed probes");
                    } else if attempt <= 10 || attempt.is_multiple_of(10) {
                        info!(%error, attempt, retry_in_s, "relay channel still down");
                    } else {
                        debug!(%error, attempt, retry_in_s, "relay channel still down");
                    }
                    tokio::select! {
                        biased;
                        _ = shutdown.wait() => break,
                        _ = tokio::time::sleep(delay) => {}
                        // Les mesures viennent de passer : le serveur est là.
                        _ = reachable.notified() => {}
                    }
                }
            }
        }

        // Les sondes en vol finissent (ou pas) dans le délai : leur compte rendu
        // arrivera, ou le serveur constatera l'échéance.
        let _ = tokio::time::timeout(
            SHUTDOWN_TIMEOUT,
            self.permits.acquire_many(MAX_CONCURRENT_PROBES as u32),
        )
        .await;
        info!("relay stopped");
    }

    /// Exécute une sonde dans sa propre tâche, sous le plafond de concurrence.
    fn spawn(&self, command: AgentCommand, shutdown: &Shutdown) {
        // Un balayage de découverte n'est pas une sonde : il ne compte pas dans
        // le plafond des sondes (il porte sa propre concurrence interne) et rend
        // compte sur sa route dédiée, au fil de l'eau.
        if command.kind == dumbmonit_proto::CMD_DISCOVERY_SCAN {
            let client = self.client.clone();
            let key = self.key.clone();
            let registry = self.registry.clone();
            let shutdown = shutdown.clone();
            tokio::spawn(async move {
                discovery::run(&client, &key, &registry, &command, shutdown).await;
            });
            return;
        }
        let client = self.client.clone();
        let key = self.key.clone();
        let registry = self.registry.clone();
        let permits = self.permits.clone();
        tokio::spawn(async move {
            let Ok(_permit) = permits.acquire().await else { return };
            let id = command.id;
            let outcome = execute(&registry, &command).await;
            if let Err(error) = client.report_probe(&key, id, &outcome).await {
                warn!(id, %error, "cannot report the probe outcome");
            }
        });
    }
}

/// Exécution d'un balayage de découverte confié à cet agent relais.
mod discovery {
    use std::sync::Arc;
    use std::time::Duration;

    use dumbmonit_collectors::Registry;
    use dumbmonit_collectors::netscan::{
        self, AGENT_MAX_CONCURRENCY, Cancel, ScanConfig, ScanUpdate,
    };
    use dumbmonit_proto::{AgentCommand, DiscoveryReport, DiscoveryScanJob, ScanProgress};
    use tracing::{debug, info, warn};

    use crate::client::PushClient;
    use crate::shutdown::Shutdown;

    /// Délai laissé au dernier compte rendu quand l'agent s'arrête en plein
    /// balayage : systemd n'attend pas indéfiniment.
    const FINAL_REPORT_ON_STOP: Duration = Duration::from_secs(2);

    /// Mène le balayage et en rend compte au serveur au fil de l'eau. Jamais de
    /// panique : une commande illisible devient un compte rendu d'erreur.
    ///
    /// L'arrêt de l'agent (`shutdown`) annule le balayage aussitôt, y compris
    /// pendant un compte rendu en cours : un `SIGTERM` ne doit jamais attendre la
    /// fin d'un /24.
    pub async fn run(
        client: &PushClient,
        key: &str,
        registry: &Registry,
        command: &AgentCommand,
        mut shutdown: Shutdown,
    ) {
        let id = command.id;
        let job = match DiscoveryScanJob::from_command(command) {
            Ok(job) => job,
            Err(error) => {
                let _ = client
                    .report_discovery(key, id, &failed(error))
                    .await
                    .map_err(|error| warn!(id, %error, "cannot report the discovery failure"));
                return;
            }
        };
        info!(id, cidr = %job.cidr, "relayed discovery scan started");

        let fingerprints = Arc::new(registry.fingerprints());
        let config = ScanConfig {
            depth: job.depth,
            community: job.snmp_community.clone().unwrap_or_else(|| "public".to_string()),
            // Machine modeste, service à mémoire bornée : peu de sockets à la fois.
            max_concurrency: AGENT_MAX_CONCURRENCY,
            ..ScanConfig::default()
        };
        let cancel = Cancel::new();
        let (tx, mut rx) = tokio::sync::mpsc::channel::<ScanUpdate>(256);
        let scan = {
            let config = config.clone();
            let cancel = cancel.clone();
            let cidr = job.cidr.clone();
            tokio::spawn(
                async move { netscan::scan(&cidr, &config, fingerprints, tx, cancel).await },
            )
        };

        let mut latest = ScanProgress::default();
        let mut since_progress = 0u32;
        let mut stopping = false;
        loop {
            let update = tokio::select! {
                biased;
                _ = shutdown.wait() => {
                    stopping = true;
                    break;
                }
                update = rx.recv() => update,
            };
            let Some(update) = update else { break };
            let report = match update {
                ScanUpdate::Host(host) => {
                    DiscoveryReport { progress: latest, hosts: vec![host], ..Default::default() }
                }
                ScanUpdate::Progress(progress) => {
                    latest = progress;
                    since_progress += 1;
                    // Un point d'avancement de temps en temps : assez pour animer
                    // la barre, pas au point de marteler le serveur.
                    if since_progress < 24 {
                        continue;
                    }
                    since_progress = 0;
                    DiscoveryReport { progress, ..Default::default() }
                }
            };
            // Un compte rendu par hôte, attendu avant de lire la suite : c'est la
            // contre-pression qui empêche le canal de gonfler.
            let sent = tokio::select! {
                biased;
                _ = shutdown.wait() => {
                    stopping = true;
                    break;
                }
                sent = client.report_discovery(key, id, &report) => sent,
            };
            match sent {
                Ok(true) => {}
                Ok(false) => {
                    debug!(id, "server cancelled the discovery scan");
                    cancel.cancel();
                    break;
                }
                Err(error) => warn!(id, %error, "cannot report discovery progress"),
            }
        }

        if stopping {
            // Arrêt de l'agent : le balayage est annulé puis abandonné sans
            // attendre ses sondes, et le serveur prévenu si cela tient en peu de
            // temps.
            cancel.cancel();
            scan.abort();
            let _ = scan.await;
            drop(rx);
            info!(id, "agent stopping, relayed discovery scan cancelled");
            let report = DiscoveryReport {
                progress: latest,
                finished: true,
                error: Some("the agent stopped during the scan".to_string()),
                ..Default::default()
            };
            let last = client.report_discovery(key, id, &report);
            let _ = tokio::time::timeout(FINAL_REPORT_ON_STOP, last).await;
            return;
        }

        // Vide ce qui reste dans le canal après une annulation, pour que la tâche
        // de balayage se termine proprement (elle s'arrête aussitôt annulée).
        while rx.recv().await.is_some() {}
        let error = match scan.await {
            Ok(Ok(_)) => None,
            Ok(Err(error)) => Some(error.to_string()),
            Err(_) => Some("the discovery scan stopped unexpectedly".to_string()),
        };
        let final_report =
            DiscoveryReport { progress: latest, finished: true, error, ..Default::default() };
        if let Err(error) = client.report_discovery(key, id, &final_report).await {
            warn!(id, %error, "cannot report the discovery completion");
        }
        info!(id, "relayed discovery scan done");
    }

    fn failed(error: String) -> DiscoveryReport {
        DiscoveryReport { finished: true, error: Some(error), ..Default::default() }
    }
}

/// Exécute une sonde et en fait le compte rendu, sans jamais paniquer : une
/// commande illisible ou un type inconnu deviennent une erreur rapportée.
pub async fn execute(registry: &Registry, command: &AgentCommand) -> ProbeOutcome {
    let started = Instant::now();
    let job = match ProbeJob::from_command(command) {
        Ok(job) => job,
        Err(error) => {
            return ProbeOutcome {
                duration_ms: 0,
                error: Some(error),
                samples: Vec::new(),
                profile_id: None,
                netconfig: None,
            };
        }
    };
    let timeout = Duration::from_secs(job.timeout_secs.max(1)).min(MAX_PROBE_TIMEOUT);
    let target = &job.target;
    debug!(id = command.id, target = target.id, kind = target.kind, "relayed probe started");

    // Sauvegarde de configuration : le texte relevé repart vers le serveur,
    // qui le chiffre et le range ; rien n'en est gardé ici. Le délai est
    // celui des options de la cible, borné comme toute sonde relayée.
    if !job.discover && target.kind == dumbmonit_collectors::netconfig::KIND {
        let fetch = dumbmonit_collectors::netconfig::relay_capture(target);
        let (error, capture) = match tokio::time::timeout(MAX_PROBE_TIMEOUT, fetch).await {
            Ok(result) => result,
            Err(_) => (
                Some(format!("Timed out after {MAX_PROBE_TIMEOUT:?}")),
                dumbmonit_proto::NetconfigCapture::default(),
            ),
        };
        let duration_ms = started.elapsed().as_millis() as u64;
        debug!(id = command.id, target = target.id, duration_ms, "relayed config fetch done");
        return ProbeOutcome {
            duration_ms,
            error,
            samples: Vec::new(),
            profile_id: None,
            netconfig: Some(capture),
        };
    }

    let (samples, profile_id, error) = if job.discover {
        match registry.discover(target, timeout).await {
            Ok(profile_id) => (Vec::new(), profile_id, None),
            Err(error) => (Vec::new(), None, Some(error.to_string())),
        }
    } else {
        match registry.probe(target, timeout).await {
            Ok(samples) => (samples, None, None),
            Err(error) => (Vec::new(), None, Some(error.to_string())),
        }
    };
    let duration_ms = started.elapsed().as_millis() as u64;
    match &error {
        None => debug!(
            id = command.id,
            target = target.id,
            samples = samples.len(),
            duration_ms,
            "relayed probe done"
        ),
        Some(error) => {
            debug!(id = command.id, target = target.id, %error, duration_ms, "relayed probe failed")
        }
    }
    ProbeOutcome { duration_ms, error, samples, profile_id, netconfig: None }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use dumbmonit_proto::{CMD_CONTAINER_RESTART, Credential, Target};

    use super::*;

    fn http_target(address: &str) -> Target {
        Target {
            id: 12,
            name: "site-web".into(),
            address: address.into(),
            kind: "http".into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            // Le garde-fou des moniteurs refuse l'adresse de bouclage sans cette
            // option ; ici le serveur de test tourne sur la machine du test.
            tags: BTreeMap::from([("allow_private_targets".to_string(), "true".to_string())]),
            credential: Credential::None,
            group_name: String::new(),
            position: 0,
        }
    }

    #[tokio::test]
    async fn a_command_that_is_not_a_probe_is_reported_as_an_error() {
        let registry = Registry::remote(Duration::from_secs(1));
        let command = AgentCommand {
            id: 1,
            kind: CMD_CONTAINER_RESTART.into(),
            args: serde_json::json!({}),
            created_at_ms: 0,
        };
        let outcome = execute(&registry, &command).await;
        assert!(outcome.error.as_deref().unwrap_or_default().contains("not a probe"));
        assert!(outcome.samples.is_empty());
    }

    #[tokio::test]
    async fn an_unknown_kind_is_reported_not_panicked() {
        let registry = Registry::remote(Duration::from_secs(1));
        let mut target = http_target("http://127.0.0.1:1/");
        target.kind = "agent".into();
        let command = ProbeJob { target, timeout_secs: 1, discover: false }.into_command(2, 0);
        let outcome = execute(&registry, &command).await;
        assert!(outcome.error.as_deref().unwrap_or_default().contains("no collector"));
    }

    #[tokio::test]
    async fn an_http_probe_against_a_local_server_yields_samples_with_the_target_identity() {
        // Un serveur HTTP minimal, le temps d'une requête.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("écoute");
        let address = listener.local_addr().expect("adresse");
        tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buffer = [0u8; 1024];
                let _ = socket.read(&mut buffer).await;
                let _ = socket
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
                    )
                    .await;
            }
        });

        let registry = Registry::remote(Duration::from_secs(2));
        let command = ProbeJob {
            target: http_target(&format!("http://{address}/")),
            timeout_secs: 5,
            discover: false,
        }
        .into_command(3, 0);
        let outcome = execute(&registry, &command).await;
        assert_eq!(outcome.error, None, "{:?}", outcome.error);
        assert!(outcome.samples.iter().any(|s| s.metric == "up"));
        assert!(
            outcome
                .samples
                .iter()
                .all(|s| s.labels.get("target").map(String::as_str) == Some("12"))
        );
    }

    #[tokio::test]
    async fn an_unreachable_device_is_reported_as_down() {
        // Un moniteur HTTP note l'indisponibilité dans ses mesures ; c'est une
        // intégration (ici Proxmox) qui échoue franchement quand rien ne répond.
        let registry = Registry::remote(Duration::from_secs(1));
        let mut target = http_target("127.0.0.1:1");
        target.kind = "proxmox".into();
        target.credential = Credential::ApiToken { token: "user@pam!token=secret".into() };
        let command = ProbeJob { target, timeout_secs: 2, discover: false }.into_command(4, 0);
        let outcome = execute(&registry, &command).await;
        let error = outcome.error.expect("échec attendu");
        // Le préfixe est ce que le serveur lit pour classer l'échec.
        assert!(
            error.starts_with("Monitor unreachable") || error.starts_with("Timed out"),
            "{error}"
        );
    }
}
