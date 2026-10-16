//! Boucle de vie de l'agent : collecter, envoyer, recommencer.

use std::collections::BTreeSet;
use std::time::Duration;

use anyhow::{Context, Result};
use dumbmonit_proto::{AgentIdentity, MAX_BATCH_SAMPLES, PushBatch, Sample};
use tokio::time::Instant;
use tracing::{debug, info, warn};

use crate::backoff::Backoff;
use crate::binding::Binding;
use crate::buffer::PendingBuffer;
use crate::client::{PushClient, PushError};
use crate::collect::backup_repos::BackupReposProbe;
use crate::collect::docker::DockerProbe;
use crate::collect::ipsec::IpsecProbe;
use crate::collect::libvirt::LibvirtProbe;
use crate::collect::perf_counters::PlatformProbe as PerfCountersProbe;
use crate::collect::plakar::PlakarProbe;
use crate::collect::registry::UpdateChecker;
use crate::collect::sensors::SensorsProbe;
use crate::collect::smart::SmartProbe;
use crate::collect::speedtest::SpeedtestProbe;
use crate::collect::system_health::SystemHealthProbe;
use crate::collect::wireguard::WireguardProbe;
use crate::collect::zfs::ZfsProbe;
use crate::collect::{SystemProbe, agent_samples, services};
use crate::commands::CommandRunner;
use crate::config::{Config, MIN_INTERVAL_SECS};
use crate::relay::RelayRunner;
use crate::shutdown::Shutdown;

/// Première temporisation après un échec d'envoi.
const BACKOFF_BASE: Duration = Duration::from_secs(5);

/// Plafond de la temporisation. Cinq minutes : au-delà, une machine revenue en
/// ligne mettrait trop longtemps à se manifester dans l'interface.
const BACKOFF_MAX: Duration = Duration::from_secs(300);

/// Délai maximal d'un envoi. Généreux, car un lot de rattrapage est volumineux et
/// part parfois sur une liaison lente.
const HTTP_TIMEOUT: Duration = Duration::from_secs(30);

/// Temps laissé au dernier envoi lors de l'arrêt. Court : systemd n'attend pas
/// indéfiniment, et être tué en essayant d'être poli ne rend service à personne.
const SHUTDOWN_FLUSH_TIMEOUT: Duration = Duration::from_secs(5);

/// Temps laissé à une commande en cours lors de l'arrêt. Une mise à jour coupée
/// en plein milieu laisserait un conteneur renommé : on attend, mais moins que
/// les 90 s au bout desquelles systemd tue le processus.
const SHUTDOWN_COMMAND_TIMEOUT: Duration = Duration::from_secs(60);

/// En `--once`, la commande éventuelle est menée à son terme : personne ne
/// viendra la finir après nous.
const ONCE_COMMAND_TIMEOUT: Duration = Duration::from_secs(900);

/// Délai d'une requête SNMP unitaire en mode relais. Le délai global de chaque
/// sonde vient du serveur avec la sonde elle-même.
const RELAY_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

pub struct Agent {
    config: Config,
    identity: AgentIdentity,
    probe: SystemProbe,
    health: SystemHealthProbe,
    docker: DockerProbe,
    updates: UpdateChecker,
    plakar: PlakarProbe,
    /// Dépôts restic et Borg déclarés dans la configuration.
    backup_repos: BackupReposProbe,
    /// Tunnels WireGuard : muet sans `wg` ni interface.
    wireguard: WireguardProbe,
    /// Tunnels IPsec : muet sans `swanctl` ni connexion.
    ipsec: IpsecProbe,
    /// Test de débit Internet : muet tant qu'il n'est pas activé.
    speedtest: SpeedtestProbe,
    /// Matériel : sondes de température, santé des disques, pools ZFS. Chacun
    /// se tait complètement là où la machine n'a rien à en dire.
    sensors: SensorsProbe,
    smart: SmartProbe,
    zfs: ZfsProbe,
    /// Domaines KVM et pools libvirt. Muet sans `virsh` ni domaine.
    libvirt: LibvirtProbe,
    /// Compteurs de performance Windows (liste libre, jeu MDaemon). Muet
    /// ailleurs, et muet sous Windows quand rien n'est configuré ni détecté.
    perf_counters: PerfCountersProbe,
    buffer: PendingBuffer,
    client: PushClient,
    backoff: Backoff,
    /// Commandes du serveur : redémarrage ou mise à jour d'un conteneur.
    commands: CommandRunner,
    /// Boucle du relais, lancée après le premier lot accepté — avant, le
    /// serveur ne nous connaît pas encore et refuserait la demande.
    relay: Option<tokio::task::JoinHandle<()>>,
    /// Période courante : celle de la configuration, jusqu'à ce que le serveur en
    /// demande une autre dans son accusé de réception.
    interval: Duration,
    /// Secret de liaison de cette machine : ce qui prouve au serveur que c'est
    /// bien elle qui parle, et pas une voisine porteuse du même jeton de parc.
    binding: Binding,
    /// Vrai une fois le refus de liaison signalé, pour ne le dire qu'une fois
    /// par panne plutôt qu'à chaque tentative.
    binding_refused: bool,
    /// Modules décochés dans l'interface, tels que le dernier accusé de
    /// réception les nomme : leur collecte est sautée. Le serveur écarte de
    /// toute façon leurs mesures ; ne pas les lire épargne la machine (un test
    /// de débit consomme de la bande passante, une vérification d'images
    /// interroge les registres).
    disabled_modules: BTreeSet<String>,
}

impl Agent {
    pub fn new(config: Config) -> Result<Self> {
        let identity = crate::identity::detect(&config);
        let binding = Binding::load(&config.secret_path);
        let client = PushClient::new(
            &config.server_url,
            &config.token,
            binding.secret().map(str::to_string),
            HTTP_TIMEOUT,
        )
        .context("preparing the push client")?;
        let commands = CommandRunner::new(
            config.commands,
            client.clone(),
            identity.key(),
            &config.docker_socket,
        );
        Ok(Self {
            interval: config.interval,
            buffer: PendingBuffer::new(config.max_buffered_samples),
            probe: SystemProbe::new(&config.probe),
            health: SystemHealthProbe::new(&config.system_health),
            docker: DockerProbe::new(&config.docker_socket, config.docker_max_containers),
            updates: UpdateChecker::new(),
            plakar: PlakarProbe::new(&config.plakar),
            backup_repos: BackupReposProbe::new(&config.backup_repos),
            wireguard: WireguardProbe::new(&config.wireguard),
            ipsec: IpsecProbe::new(&config.ipsec),
            speedtest: SpeedtestProbe::new(&config.speedtest),
            sensors: SensorsProbe::new(config.sensors),
            smart: SmartProbe::new(&config.smart),
            zfs: ZfsProbe::new(&config.zfs),
            libvirt: LibvirtProbe::new(&config.libvirt),
            perf_counters: PerfCountersProbe::for_platform(&config.perf_counters),
            backoff: Backoff::new(BACKOFF_BASE, BACKOFF_MAX),
            commands,
            relay: None,
            binding,
            binding_refused: false,
            disabled_modules: BTreeSet::new(),
            identity,
            client,
            config,
        })
    }

    /// Un cycle complet de mesure.
    pub async fn collect(&mut self) -> Vec<Sample> {
        let started = Instant::now();
        let mut snapshot = self.probe.read();
        snapshot.services = services::probe(&self.config.services).await;
        if self.config.docker && self.wants("docker") {
            snapshot.containers = self.docker.read().await;
            if let Some(inventory) = snapshot.containers.as_mut() {
                self.check_updates(&mut inventory.detailed);
            }
        }
        snapshot.system_health = self.health.read().await;
        if self.wants("plakar") {
            snapshot.backups = self.plakar.read().await;
        }
        if self.wants("backup_repos") {
            snapshot.backup_repos = self.backup_repos.read().await;
        }
        if self.wants("wireguard") {
            snapshot.wireguard = self.wireguard.read().await;
        }
        if self.wants("ipsec") {
            snapshot.ipsec = self.ipsec.read().await;
        }
        let speedtest = self.wants("speedtest");
        if speedtest {
            snapshot.speedtest = self.speedtest.read().await;
        }
        snapshot.sensors = self.sensors.read();
        snapshot.smart = self.smart.read().await;
        if self.wants("zfs") {
            snapshot.zfs = self.zfs.read().await;
        }
        if self.wants("libvirt") {
            snapshot.libvirt = self.libvirt.read().await;
        }
        snapshot.perf_counters = self.perf_counters.read();

        // Un seul horodatage pour tout le cycle : c'est ce qui rend les séries
        // comparables entre elles à l'instant près.
        let now_ms = chrono::Utc::now().timestamp_millis();
        let mut samples = snapshot.to_samples(now_ms);
        if speedtest {
            samples.extend(crate::collect::speedtest::threshold_samples(
                &self.config.speedtest,
                now_ms,
            ));
        }
        samples.extend(agent_samples(
            started.elapsed(),
            self.buffer.len(),
            self.buffer.dropped(),
            now_ms,
        ));
        samples
    }

    /// Faux pour un module que l'utilisateur a décoché dans l'interface.
    fn wants(&self, module: &str) -> bool {
        !self.disabled_modules.contains(module)
    }

    /// Adopte la liste des modules décochés que le serveur vient de renvoyer.
    fn adopt_disabled_modules(&mut self, disabled: Vec<String>) {
        let disabled: BTreeSet<String> = disabled.into_iter().collect();
        if disabled != self.disabled_modules {
            info!(
                modules = disabled.iter().map(String::as_str).collect::<Vec<_>>().join(","),
                "modules switched off from the server"
            );
            self.disabled_modules = disabled;
        }
    }

    /// Complète chaque conteneur avec le dernier verdict de mise à jour connu et
    /// relance, s'il y a lieu, la vérification de fond. Jamais d'attente ici.
    fn check_updates(&self, containers: &mut [crate::collect::docker::ContainerStat]) {
        if !self.config.docker_update_check {
            return;
        }
        let images: Vec<(String, Vec<String>)> = containers
            .iter()
            .map(|c| (c.image.clone(), self.docker.repo_digests(&c.image_id)))
            .collect();
        self.updates.refresh(images.clone());
        for (container, (image, digests)) in containers.iter_mut().zip(&images) {
            container.update_available = self.updates.result(image, digests);
        }
    }

    /// Collecte et envoie une seule fois. Sert au `--once` de vérification
    /// d'installation : le script sait ainsi tout de suite si le jeton est bon.
    pub async fn run_once(&mut self) -> Result<()> {
        self.probe.warm_up().await;
        let samples = self.collect().await;
        let count = samples.len();
        self.buffer.push(samples);
        match self.flush().await {
            FlushOutcome::Sent { .. } => {
                info!(samples = count, "batch sent");
                self.commands.poll().await;
                self.commands.finish(ONCE_COMMAND_TIMEOUT).await;
                Ok(())
            }
            FlushOutcome::Failed(error) => Err(anyhow::anyhow!("{error}")),
            FlushOutcome::Idle => Ok(()),
        }
    }

    pub async fn run(mut self, mut shutdown: Shutdown) -> Result<()> {
        info!(
            server = self.client.url(),
            host = self.identity.hostname,
            period_s = self.interval.as_secs(),
            "agent started"
        );
        match &self.config.modules {
            Some(modules) if modules.is_empty() => {
                info!("modules: none (system metrics only)")
            }
            Some(modules) => info!(
                modules = modules.iter().map(String::as_str).collect::<Vec<_>>().join(","),
                "modules chosen in the configuration"
            ),
            None => info!("modules: auto-detected (set 'modules' in agent.yaml to choose)"),
        }
        self.probe.warm_up().await;

        let mut next_collect = Instant::now();
        let mut next_send = Instant::now();

        loop {
            tokio::select! {
                // Priorité stricte à l'arrêt : sans cela, un tampon plein
                // enchaînerait les envois et repousserait indéfiniment la sortie.
                biased;

                _ = shutdown.wait() => break,

                _ = tokio::time::sleep_until(next_collect) => {
                    let samples = self.collect().await;
                    debug!(count = samples.len(), "collection cycle done");
                    self.buffer.push(samples);
                    next_collect = Instant::now() + self.interval;
                    // `next_send` n'est volontairement pas touché : s'il est déjà
                    // échu, la branche d'envoi partira au prochain tour ; s'il est
                    // dans le futur, c'est qu'une temporisation court, et une
                    // nouvelle mesure n'est pas une raison de harceler un serveur
                    // qu'on sait injoignable.
                }

                _ = tokio::time::sleep_until(next_send), if !self.buffer.is_empty() => {
                    next_send = match self.flush().await {
                        // Reste-t-il du retard ? On enchaîne, lot après lot.
                        FlushOutcome::Sent { .. } => {
                            // Le serveur vient de nous répondre : c'est le moment
                            // de lui demander s'il attend quelque chose de nous.
                            self.commands.poll().await;
                            self.start_relay(&shutdown);
                            Instant::now()
                        }
                        FlushOutcome::Idle => Instant::now(),
                        FlushOutcome::Failed(_) => Instant::now() + self.backoff.next_delay(),
                    };
                }
            }
        }

        self.commands.finish(SHUTDOWN_COMMAND_TIMEOUT).await;
        if let Some(relay) = self.relay.take() {
            // Le relais observe le même signal d'arrêt : on attend seulement
            // qu'il ait rendu la main.
            let _ = relay.await;
        }
        self.final_flush().await;
        info!("agent stopped");
        Ok(())
    }

    /// Lance la boucle du relais si la configuration le demande, une seule fois.
    fn start_relay(&mut self, shutdown: &Shutdown) {
        if !self.config.relay || self.relay.is_some() {
            return;
        }
        let runner =
            RelayRunner::new(self.client.clone(), self.identity.key(), RELAY_REQUEST_TIMEOUT);
        self.relay = Some(tokio::spawn(runner.run(shutdown.clone())));
    }

    /// Envoie un lot prélevé dans le tampon.
    async fn flush(&mut self) -> FlushOutcome {
        let samples = self.buffer.take(MAX_BATCH_SAMPLES);
        if samples.is_empty() {
            return FlushOutcome::Idle;
        }
        let count = samples.len();
        let batch =
            PushBatch::new(self.identity.clone(), samples, chrono::Utc::now().timestamp_millis());

        match self.client.send(&batch).await {
            Ok(ack) => {
                if self.backoff.attempts() > 0 {
                    info!(attempts = self.backoff.attempts(), "connection to the server restored");
                }
                self.backoff.reset();
                self.binding_refused = false;
                debug!(target = ack.target_id, accepted = ack.accepted, "batch accepted");
                self.adopt_binding(ack.agent_secret.clone(), ack.bound);
                self.adopt_interval(ack.interval_secs);
                self.adopt_disabled_modules(ack.disabled_modules);
                FlushOutcome::Sent { count }
            }
            Err(PushError::Forbidden(message)) => {
                self.buffer.restore(batch.samples);
                self.on_binding_refused(&message);
                FlushOutcome::Failed(PushError::Forbidden(message))
            }
            Err(error) if error.is_retryable() => {
                self.buffer.restore(batch.samples);
                // Le premier échec est signalé fort, les suivants en sourdine : une
                // panne d'une heure ne doit pas remplir le journal de la machine.
                if self.backoff.attempts() == 0 {
                    warn!(%error, buffered = self.buffer.len(), "send failed, measurements kept");
                } else {
                    debug!(%error, attempt = self.backoff.attempts() + 1, "send failed again");
                }
                FlushOutcome::Failed(error)
            }
            Err(error) => {
                // Le serveur a compris et refusé : réémettre à l'identique ne ferait
                // que reproduire le refus, en gardant le tampon plein pour rien.
                warn!(%error, dropped = count, "batch rejected by the server, dropped");
                FlushOutcome::Sent { count }
            }
        }
    }

    /// Dernière tentative d'envoi avant de rendre la main au système.
    async fn final_flush(&mut self) {
        if self.buffer.is_empty() {
            return;
        }
        let pending = self.buffer.len();
        match tokio::time::timeout(SHUTDOWN_FLUSH_TIMEOUT, self.flush()).await {
            Ok(FlushOutcome::Sent { count }) => {
                info!(count, "pending measurements sent before shutdown");
            }
            _ => warn!(pending, "measurements lost: the server did not answer before shutdown"),
        }
    }

    /// Range le secret de liaison que le serveur vient d'attribuer.
    ///
    /// Il n'arrive qu'une fois, dans l'accusé de réception du lot qui a lié la
    /// machine. Tout ce qui suit le présentera à chaque requête.
    fn adopt_binding(&mut self, issued: Option<String>, bound: bool) {
        let Some(secret) = issued else {
            if !bound {
                // Le serveur nous connaît sans nous avoir liés : c'est le cas
                // d'un enregistrement d'avant la liaison, pas une anomalie.
                debug!("this machine is not bound to this agent installation yet");
            }
            return;
        };
        self.binding.save(&secret);
        self.client.set_secret(Some(secret));
        info!("this machine is now bound to this agent installation");
    }

    /// Le serveur ne reconnaît pas notre liaison — ou quelqu'un a répondu 403.
    ///
    /// Le secret n'est oublié que sur le refus de liaison du serveur lui-même :
    /// le garder n'ouvre rien, et repartir sans lui est ce qu'il faut faire dès
    /// que quelqu'un aura cliqué « Allow re-enrolment » dans l'interface. Tout
    /// autre 403 — mandataire inverse, pare-feu applicatif — ne dit rien de la
    /// liaison : effacer le secret sur sa foi obligerait à réenrôler chaque
    /// machine à la main. Les mesures restent en tampon dans les deux cas,
    /// l'agent réessaie avec sa temporisation habituelle.
    fn on_binding_refused(&mut self, message: &str) {
        if message == dumbmonit_proto::BINDING_MISMATCH {
            self.binding.forget();
            self.client.set_secret(None);
        }
        if self.binding_refused {
            debug!(reason = message, "still refused by the server");
            return;
        }
        self.binding_refused = true;
        warn!(reason = message, "this agent is not recognised for this machine");
    }

    /// Adopte la période demandée par le serveur.
    ///
    /// C'est ce qui permet de régler la finesse d'échantillonnage d'une machine
    /// depuis l'interface, sans se connecter dessus ni redémarrer l'agent.
    fn adopt_interval(&mut self, wanted_secs: u64) {
        let wanted = Duration::from_secs(wanted_secs.max(MIN_INTERVAL_SECS));
        if wanted != self.interval {
            info!(
                old_s = self.interval.as_secs(),
                new_s = wanted.as_secs(),
                "sampling period adjusted by the server"
            );
            self.interval = wanted;
        }
    }
}

#[derive(Debug)]
enum FlushOutcome {
    /// Rien à envoyer.
    Idle,
    Sent {
        count: usize,
    },
    Failed(PushError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn config() -> Config {
        Config {
            server_url: "http://127.0.0.1:1".into(),
            token: "dmon_test".into(),
            interval: Duration::from_secs(30),
            hostname: Some("machine-de-test".into()),
            services: Vec::new(),
            tags: std::collections::BTreeMap::new(),
            modules: None,
            docker: false,
            docker_socket: std::path::PathBuf::from("/inexistant.sock"),
            docker_update_check: false,
            docker_max_containers: 200,
            commands: false,
            relay: false,
            site: None,
            probe: crate::collect::ProbeConfig::default(),
            system_health: crate::collect::system_health::SystemHealthConfig::default(),
            plakar: crate::collect::plakar::PlakarConfig::default(),
            sensors: false,
            smart: crate::collect::smart::SmartConfig::default(),
            zfs: crate::collect::zfs::ZfsConfig::default(),
            libvirt: crate::collect::libvirt::LibvirtConfig::default(),
            wireguard: crate::collect::wireguard::WireguardConfig::default(),
            ipsec: crate::collect::ipsec::IpsecConfig::default(),
            speedtest: crate::collect::speedtest::SpeedtestConfig::default(),
            backup_repos: crate::collect::backup_repos::BackupReposConfig::default(),
            perf_counters: crate::collect::perf_counters::PerfCountersConfig::default(),
            max_buffered_samples: 1_000,
            secret_path: std::path::PathBuf::from("/inexistant/agent-secret"),
            log_level: tracing::Level::INFO,
            deprecated_env: Vec::new(),
        }
    }

    #[test]
    fn the_server_can_tighten_or_relax_the_sampling_period() {
        let mut agent = Agent::new(config()).expect("agent");
        agent.adopt_interval(10);
        assert_eq!(agent.interval, Duration::from_secs(10));

        agent.adopt_interval(120);
        assert_eq!(agent.interval, Duration::from_secs(120));
    }

    #[test]
    fn an_absurd_period_from_the_server_is_clamped() {
        // Un serveur qui renverrait zéro transformerait l'agent en boucle folle sur
        // la machine surveillée : la borne est appliquée côté agent, par principe.
        let mut agent = Agent::new(config()).expect("agent");
        agent.adopt_interval(0);
        assert_eq!(agent.interval, Duration::from_secs(MIN_INTERVAL_SECS));
    }

    #[tokio::test]
    async fn a_module_unticked_on_the_server_is_no_longer_collected() {
        let mut config = config();
        config.speedtest.min_download_bytes_per_second = Some(1_000_000.0);
        let mut agent = Agent::new(config).expect("agent");
        let threshold = |samples: &[Sample]| {
            samples.iter().any(|s| s.metric == "agent_speedtest_download_min_bytes_per_second")
        };
        assert!(threshold(&agent.collect().await), "le seuil est publié tant que le module vit");

        agent.adopt_disabled_modules(vec!["speedtest".into()]);
        assert!(!agent.wants("speedtest"));
        assert!(agent.wants("docker"), "les autres modules continuent");
        assert!(!threshold(&agent.collect().await));

        // Recoché : l'accusé suivant ne le nomme plus, la collecte reprend.
        agent.adopt_disabled_modules(Vec::new());
        assert!(threshold(&agent.collect().await));
    }

    #[tokio::test]
    async fn a_collection_cycle_produces_metrics_for_the_local_machine() {
        let mut agent = Agent::new(config()).expect("agent");
        let samples = agent.collect().await;

        assert!(!samples.is_empty());
        // Le point qui compte : rien d'invalide ne doit atteindre le serveur.
        assert!(samples.iter().all(|s| s.value.is_finite()));
        assert!(samples.iter().any(|s| s.metric == "memory_total_bytes"));
        assert!(samples.iter().any(|s| s.metric == "agent_collect_seconds"));
        // Le périmètre par défaut : pas de détail par cœur, pas d'interface
        // virtuelle, pas de montage de conteneur, et une entrée d'E/S par disque.
        assert!(samples.iter().all(|s| s.metric != "cpu_core_usage_percent"));
        assert!(
            samples
                .iter()
                .all(|s| { s.labels.get("ifname").is_none_or(|name| !name.starts_with("veth")) })
        );
        assert!(samples.iter().all(|s| {
            s.labels.get("mountpoint").is_none_or(|m| !m.starts_with("/var/lib/docker/"))
        }));
        assert!(
            samples.iter().all(|s| {
                !s.metric.starts_with("disk_") || !s.labels.contains_key("mountpoint")
            })
        );
    }

    #[tokio::test]
    async fn the_mount_list_is_reread_only_every_tenth_cycle() {
        // Deux cycles consécutifs : le second ne relit pas la liste des montages,
        // et doit pourtant remonter les mêmes systèmes de fichiers que le premier.
        let mut probe = SystemProbe::new(&crate::collect::ProbeConfig::default());
        let first = probe.read();
        let second = probe.read();
        let mounts = |snapshot: &crate::collect::Snapshot| -> Vec<String> {
            snapshot.filesystems.iter().map(|fs| fs.mount_point.clone()).collect()
        };
        assert_eq!(mounts(&first), mounts(&second));
        assert_eq!(first.disk_io.len(), second.disk_io.len());
        assert_eq!(first.cpu.core_count, second.cpu.core_count);
    }

    #[tokio::test]
    async fn an_unreachable_server_keeps_the_measurements_in_the_buffer() {
        let mut agent = Agent::new(config()).expect("agent");
        let samples = agent.collect().await;
        let count = samples.len();
        agent.buffer.push(samples);

        // L'adresse configurée ne peut accepter aucune connexion : l'envoi échoue
        // à coup sûr, et c'est exactement le scénario « serveur redémarré ».
        match agent.flush().await {
            FlushOutcome::Failed(error) => {
                // Une panne de liaison se réessaie ; c'est ce qui distingue ce cas
                // d'un lot refusé, que l'agent abandonnerait.
                assert!(error.is_retryable(), "{error}");
            }
            other => panic!("échec attendu, obtenu {other:?}"),
        }
        assert_eq!(agent.buffer.len(), count, "aucune mesure ne doit être perdue");

        // La temporisation ne grandit qu'au moment où la boucle la consomme.
        assert_eq!(agent.backoff.attempts(), 0);
        agent.backoff.next_delay();
        assert_eq!(agent.backoff.attempts(), 1);
    }

    // ------------------------------------------------ contre un faux serveur

    const ACK: &str = r#"{"target_id":3,"accepted":1,"interval_secs":60,
        "agent_secret":"dmab_attribue","bound":true}"#;

    /// Agent pointé sur le faux serveur, avec un secret de liaison jetable.
    fn agent_for(url: &str, dir: &tempfile::TempDir) -> Agent {
        let mut config = config();
        config.server_url = url.to_string();
        config.secret_path = dir.path().join("agent-secret");
        Agent::new(config).expect("agent")
    }

    #[tokio::test]
    async fn the_binding_secret_issued_by_the_server_is_kept_then_forgotten_on_refusal() {
        let dir = tempfile::tempdir().unwrap();
        let answer = std::sync::Arc::new(std::sync::Mutex::new((200u16, ACK.to_string())));
        let shared = answer.clone();
        let (url, calls) =
            crate::testsupport::fake_http(move |_| shared.lock().unwrap().clone()).await;
        let mut agent = agent_for(&url, &dir);
        let samples = agent.collect().await;
        agent.buffer.push(samples);

        assert!(matches!(agent.flush().await, FlushOutcome::Sent { .. }));
        // Le secret est rangé sur disque, la période adoptée, le tampon vidé.
        assert_eq!(
            std::fs::read_to_string(dir.path().join("agent-secret")).unwrap().trim(),
            "dmab_attribue"
        );
        assert_eq!(agent.interval, Duration::from_secs(60));
        assert!(agent.buffer.is_empty());

        // Au tour suivant, le secret accompagne le lot.
        let samples = agent.collect().await;
        agent.buffer.push(samples);
        agent.flush().await;
        assert_eq!(calls.all()[1].header("x-dumbmonit-agent-secret"), Some("dmab_attribue"));

        // Le serveur ne reconnaît plus la liaison : on l'oublie, on garde les
        // mesures, et le refus n'est signalé qu'une fois.
        *answer.lock().unwrap() =
            (403, serde_json::json!({ "error": dumbmonit_proto::BINDING_MISMATCH }).to_string());
        let samples = agent.collect().await;
        let count = samples.len();
        agent.buffer.push(samples);
        assert!(matches!(agent.flush().await, FlushOutcome::Failed(PushError::Forbidden(_))));
        assert_eq!(agent.buffer.len(), count);
        assert!(agent.binding_refused);
        assert!(!dir.path().join("agent-secret").exists());
        assert!(matches!(agent.flush().await, FlushOutcome::Failed(_)));
    }

    #[tokio::test]
    async fn a_403_that_is_not_the_binding_refusal_keeps_the_secret() {
        // Un mandataire inverse ou un pare-feu applicatif répond 403 : rien ne
        // dit que le serveur a oublié la liaison. Effacer le secret obligerait à
        // réenrôler à la main toutes les machines du parc.
        let dir = tempfile::tempdir().unwrap();
        let answer = std::sync::Arc::new(std::sync::Mutex::new((200u16, ACK.to_string())));
        let shared = answer.clone();
        let (url, calls) =
            crate::testsupport::fake_http(move |_| shared.lock().unwrap().clone()).await;
        let mut agent = agent_for(&url, &dir);
        let samples = agent.collect().await;
        agent.buffer.push(samples);
        assert!(matches!(agent.flush().await, FlushOutcome::Sent { .. }));

        for body in [
            "<html><body>403 Forbidden</body></html>".to_string(),
            r#"{"error":"Request blocked by WAF"}"#.to_string(),
            r#"{"error":"Operator or admin role required."}"#.to_string(),
        ] {
            *answer.lock().unwrap() = (403, body.clone());
            let samples = agent.collect().await;
            let count = samples.len();
            agent.buffer.push(samples);
            assert!(matches!(agent.flush().await, FlushOutcome::Failed(PushError::Forbidden(_))));
            assert!(agent.buffer.len() >= count, "les mesures restent en tampon");
            assert_eq!(
                std::fs::read_to_string(dir.path().join("agent-secret")).unwrap().trim(),
                "dmab_attribue",
                "secret effacé sur un 403 étranger : {body}"
            );
        }
        // Et il accompagne toujours les requêtes suivantes.
        *answer.lock().unwrap() = (200, ACK.to_string());
        assert!(matches!(agent.flush().await, FlushOutcome::Sent { .. }));
        let last = calls.all().pop().expect("requête");
        assert_eq!(last.header("x-dumbmonit-agent-secret"), Some("dmab_attribue"));
    }

    #[tokio::test]
    async fn a_batch_the_server_rejects_is_dropped_rather_than_resent() {
        let dir = tempfile::tempdir().unwrap();
        let (url, calls) =
            crate::testsupport::fake_http(|_| (400, r#"{"error":"bad batch"}"#.to_string())).await;
        let mut agent = agent_for(&url, &dir);
        let samples = agent.collect().await;
        agent.buffer.push(samples);
        assert!(matches!(agent.flush().await, FlushOutcome::Sent { .. }));
        assert!(agent.buffer.is_empty(), "un lot refusé ne se réémet pas");
        assert!(matches!(agent.flush().await, FlushOutcome::Idle));
        assert_eq!(calls.all().len(), 1);
    }

    #[tokio::test]
    async fn run_once_reports_whether_the_server_accepted_the_batch() {
        let dir = tempfile::tempdir().unwrap();
        let (url, calls) = crate::testsupport::fake_http(|req| {
            if req.path == "/api/ingest" { (200, ACK.to_string()) } else { (200, "[]".into()) }
        })
        .await;
        let mut agent = agent_for(&url, &dir);
        agent.config.commands = true;
        agent.commands = CommandRunner::new(
            true,
            agent.client.clone(),
            agent.identity.key(),
            std::path::Path::new("/inexistant/docker.sock"),
        );
        agent.run_once().await.expect("lot accepté");
        // Le lot part, puis l'agent demande s'il a des commandes à exécuter.
        assert_eq!(calls.count("POST", "/api/ingest"), 1);
        assert_eq!(calls.count("GET", "/api/agent/commands"), 1);

        let dir = tempfile::tempdir().unwrap();
        let (url, _) = crate::testsupport::fake_http(|_| (503, "down".to_string())).await;
        let error = agent_for(&url, &dir).run_once().await.unwrap_err().to_string();
        assert!(error.contains("503"), "{error}");
    }

    #[tokio::test]
    async fn the_loop_collects_sends_and_flushes_before_stopping() {
        let dir = tempfile::tempdir().unwrap();
        let (url, calls) = crate::testsupport::fake_http(|_| (200, ACK.to_string())).await;
        let agent = agent_for(&url, &dir);
        let (trigger, shutdown) = crate::shutdown::channel();
        // Premier cycle : une collecte immédiate puis son envoi, et l'arrêt
        // est demandé dès que le lot est parti.
        let stopper = async {
            let started = std::time::Instant::now();
            while calls.count("POST", "/api/ingest") == 0 {
                assert!(started.elapsed() < Duration::from_secs(30), "aucun lot envoyé");
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            trigger.fire();
        };
        let (result, ()) = tokio::join!(agent.run(shutdown), stopper);
        result.expect("sortie propre");
    }

    #[tokio::test]
    async fn a_loop_that_cannot_reach_the_server_keeps_its_measurements_and_stops_anyway() {
        let dir = tempfile::tempdir().unwrap();
        let mut agent = agent_for("http://127.0.0.1:1", &dir);
        agent.config.modules = Some(Default::default());
        let (trigger, shutdown) = crate::shutdown::channel();
        let stopper = async {
            tokio::time::sleep(Duration::from_millis(1500)).await;
            trigger.fire();
        };
        let (result, ()) = tokio::join!(agent.run(shutdown), stopper);
        result.expect("sortie propre");
    }
}
