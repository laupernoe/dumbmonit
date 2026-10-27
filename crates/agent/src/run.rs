//! Boucle de vie de l'agent : collecter, envoyer, recommencer.

use std::collections::BTreeSet;
use std::time::Duration;

use anyhow::{Context, Result};
use dumbmonit_proto::{
    AgentIdentity, AgentUpdateOffer, MAX_BATCH_SAMPLES, PushBatch, RelaySetting, Sample,
    SoftwareInventory,
};
use tokio::time::Instant;
use tracing::{debug, info, warn};

use crate::backoff::Backoff;
use crate::binding::Binding;
use crate::buffer::PendingBuffer;
use crate::client::{PushClient, PushError};
use crate::collect::backup_repos::BackupReposProbe;
use crate::collect::docker::DockerProbe;
use crate::collect::eventlog::{self, PlatformProbe as EventLogProbe};
use crate::collect::inventory::{self, PendingInventory};
use crate::collect::ipsec::IpsecProbe;
use crate::collect::libvirt::LibvirtProbe;
use crate::collect::mail_server::PlatformProbe as MailServerProbe;
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
use crate::logs::LogRunner;
use crate::relay::RelayRunner;
use crate::selfupdate::{AfterStop, SelfUpdater};
use crate::shutdown::Shutdown;
use crate::tls::TlsState;

/// Délai avant une nouvelle tentative d'enrôlement ou de renouvellement sur le
/// canal chiffré, après un échec.
const TLS_RETRY: Duration = Duration::from_secs(60);

/// Première temporisation après un échec d'envoi.
const BACKOFF_BASE: Duration = Duration::from_secs(2);

/// Plafond de la temporisation : trente secondes, décalage aléatoire compris.
///
/// Il était de cinq minutes. Le serveur déclare une machine muette après trois
/// périodes, au moins 90 s (`staleness_deadline`) : un plafond plus long que ce
/// délai transformait chaque coupure en panne apparente rallongée de plusieurs
/// minutes après le retour du serveur — 15 tentatives sur 53 min observées pour
/// une coupure réseau. Un agent toutes les 30 s ne pèse rien sur un serveur qui
/// redémarre ; le décalage aléatoire étale le parc.
pub(crate) const BACKOFF_MAX: Duration = Duration::from_secs(30);

/// Mesures gardées sur disque à l'arrêt quand le serveur ne répond pas, au plus.
/// Quelques mégaoctets : de quoi couvrir un redémarrage pendant une coupure,
/// pas de quoi remplir un disque.
const MAX_PERSISTED_SAMPLES: usize = 20_000;

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
    /// Dernier inventaire logiciel relevé, en attente d'être joint à un lot. Vidé
    /// quand le serveur l'a accepté.
    inventory: PendingInventory,
    /// Tâche de fond du relevé quotidien, lancée au premier cycle.
    inventory_task: Option<tokio::task::JoinHandle<()>>,
    /// Moniteurs déclarés par les étiquettes Docker au dernier cycle ; `None` :
    /// Docker illisible ou désactivé, le serveur ne doit alors rien archiver.
    declared: Option<Vec<dumbmonit_proto::DeclaredMonitor>>,
    /// Compteurs de performance Windows (liste libre, jeu MDaemon). Muet
    /// ailleurs, et muet sous Windows quand rien n'est configuré ni détecté.
    perf_counters: PerfCountersProbe,
    /// MDaemon et SecurityGateway : services, files, version (Windows).
    mail_server: MailServerProbe,
    /// Journal d'événements Windows : les règles reçues du serveur. Muet tant
    /// qu'il n'y en a aucune, c'est-à-dire partout ailleurs que sous Windows.
    eventlog: EventLogProbe,
    buffer: PendingBuffer,
    client: PushClient,
    backoff: Backoff,
    /// Commandes du serveur : redémarrage ou mise à jour d'un conteneur.
    commands: CommandRunner,
    /// Boucle du relais, lancée après le premier lot accepté — avant, le
    /// serveur ne nous connaît pas encore et refuserait la demande.
    relay: Option<tokio::task::JoinHandle<()>>,
    /// Mode relais réglé depuis l'interface, tel que le dernier accusé de
    /// réception le porte. `None` tant que le serveur n'en a rien dit (serveur
    /// antérieur) : seule la configuration locale compte alors.
    server_relay: Option<RelaySetting>,
    /// Collecte de journaux de cette machine, et le réglage que le dernier accusé
    /// de réception portait (`None` : serveur antérieur, rien n'est collecté).
    logs: LogRunner,
    server_logs: Option<dumbmonit_proto::LogsSetting>,
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
    /// Taille maximale d'un lot. Part de [`MAX_BATCH_SAMPLES`] et se réduit de
    /// moitié à chaque 413 : un mandataire au plafond de corps plus bas que le
    /// serveur ne bouge pas, inutile de revenir buter dessus.
    batch_limit: usize,
    /// Vrai une fois signalé qu'un lot a été entièrement écarté par le serveur,
    /// pour ne le dire qu'une fois par épisode.
    all_refused: bool,
    /// Début de la coupure en cours et dernière cause journalisée : la reprise
    /// dit combien de temps a duré la panne, un changement de cause est signalé.
    outage: Option<(Instant, String)>,
    /// Canal chiffré : URL, empreintes, certificat. `None` tant que ni la
    /// configuration ni le serveur n'en ont parlé (serveur antérieur).
    tls: Option<TlsState>,
    /// Où il est rangé, à côté du secret de liaison.
    tls_path: std::path::PathBuf,
    /// Pas de nouvelle tentative d'enrôlement ou de renouvellement avant.
    tls_retry_at: Option<Instant>,
    /// Vrai une fois un échec du canal chiffré signalé, pour ne le dire
    /// qu'une fois par épisode.
    tls_failed: bool,
    /// Mise à jour automatique de l'agent lui-même.
    updater: SelfUpdater,
    /// Dernière offre de mise à jour du serveur, examinée après l'envoi.
    update_offer: Option<AgentUpdateOffer>,
    /// Ce qu'il reste à faire une fois la boucle arrêtée (redémarrer sur le
    /// nouveau binaire, ou rétablir l'ancien).
    after_stop: Option<AfterStop>,
}

impl Agent {
    pub fn new(config: Config) -> Result<Self> {
        let mut identity = crate::identity::detect(&config);
        let updater = SelfUpdater::new(config.self_update);
        identity.self_update = Some(updater.info());
        let binding = Binding::load(&config.secret_path);
        let client = PushClient::new(
            &config.server_url,
            &config.token,
            binding.secret().map(str::to_string),
            HTTP_TIMEOUT,
        )
        .context("preparing the push client")?;
        let tls_path = crate::tls::beside(&config.secret_path);
        let tls = initial_tls(&config, TlsState::load(&tls_path));
        if let Some(state) = &tls {
            client.set_pins(state.pins.clone());
            if let Some(identity) = state.identity() {
                client
                    .use_tls(&state.url, Some(identity))
                    .context("preparing the agent channel")?;
            } else if state.locked || config.ca_pin.is_some() {
                // Pas encore (ou plus) de certificat, mais l'agent ne parle
                // qu'au canal chiffré : rien ne partira en clair d'ici là.
                client.use_tls(&state.url, None).context("preparing the agent channel")?;
            }
        }
        let commands = CommandRunner::new(
            config.commands,
            client.clone(),
            identity.key(),
            &config.docker_socket,
        )
        .with_actions(config.actions.clone());
        let logs = LogRunner::new(
            client.clone(),
            identity.key(),
            config.secret_path.with_file_name("logs-spool.jsonl"),
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
            inventory: PendingInventory::default(),
            inventory_task: None,
            declared: None,
            perf_counters: PerfCountersProbe::for_platform(&config.perf_counters),
            mail_server: MailServerProbe::for_platform(config.perf_counters.mdaemon),
            eventlog: EventLogProbe::for_platform(
                config.secret_path.with_file_name(eventlog::STATE_FILE),
            ),
            backoff: Backoff::new(BACKOFF_BASE, BACKOFF_MAX),
            commands,
            relay: None,
            server_relay: None,
            logs,
            server_logs: None,
            binding,
            binding_refused: false,
            disabled_modules: BTreeSet::new(),
            batch_limit: MAX_BATCH_SAMPLES,
            all_refused: false,
            outage: None,
            tls,
            tls_path,
            tls_retry_at: None,
            tls_failed: false,
            updater,
            update_offer: None,
            after_stop: None,
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
        self.declared = None;
        if self.config.docker && self.wants("docker") {
            snapshot.containers = self.docker.read().await;
            if let Some(inventory) = snapshot.containers.as_mut() {
                self.check_updates(&mut inventory.detailed);
                self.declared = Some(inventory.declared.clone());
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
        self.sync_inventory();
        snapshot.perf_counters = self.perf_counters.read();
        snapshot.mail_server = self.mail_server.read().await;

        // Un seul horodatage pour tout le cycle : c'est ce qui rend les séries
        // comparables entre elles à l'instant près.
        let now_ms = chrono::Utc::now().timestamp_millis();
        self.eventlog.read(now_ms);
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

    fn wants_inventory(&self) -> bool {
        self.config.inventory && self.wants("inventory")
    }

    /// Lance la tâche de relevé quotidien au premier cycle, l'arrête (et oublie
    /// ce qu'elle avait relevé) quand l'inventaire est coupé, en local ou depuis
    /// l'interface.
    fn sync_inventory(&mut self) {
        if self.wants_inventory() {
            if self.inventory_task.is_none() {
                self.inventory_task = Some(inventory::spawn(self.inventory.clone()));
            }
        } else if let Some(task) = self.inventory_task.take() {
            task.abort();
            *inventory::lock(&self.inventory) = None;
            info!("software inventory switched off");
        }
    }

    /// Oublie l'inventaire en attente s'il est encore celui qui vient d'être
    /// envoyé (le relevé suivant a pu le remplacer entre-temps).
    fn forget_inventory(&self, sent: Option<&SoftwareInventory>) {
        let Some(sent) = sent else { return };
        let mut pending = inventory::lock(&self.inventory);
        if pending.as_ref().is_some_and(|p| p.collected_at_ms == sent.collected_at_ms) {
            *pending = None;
        }
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
        self.restore_pending();
        self.probe.warm_up().await;

        let mut next_collect = Instant::now();
        let mut next_send = Instant::now();
        // Nouveau binaire à l'essai : sans lot accepté d'ici là, l'ancien
        // revient.
        let trial_deadline = self.updater.trial_deadline();
        if trial_deadline.is_some() {
            info!("new agent version on trial until its first accepted batch");
        }
        // Toute réponse favorable du serveur, y compris sur le canal du relais,
        // met fin à la temporisation : inutile d'attendre qu'elle expire.
        let reachable = self.client.reachable();

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
                            self.sync_relay(&shutdown);
                            self.logs.apply(self.server_logs.clone(), &shutdown);
                            if self.apply_update().await {
                                break;
                            }
                            Instant::now()
                        }
                        FlushOutcome::Idle => Instant::now(),
                        FlushOutcome::Failed(error) => {
                            let delay = self.backoff.next_delay();
                            self.note_failure(&error, delay);
                            Instant::now() + delay
                        }
                    };
                }

                _ = reachable.notified(), if self.outage.is_some() && !self.buffer.is_empty() => {
                    debug!("server answered on another channel, sending now");
                    next_send = Instant::now();
                }

                _ = tokio::time::sleep_until(trial_deadline.unwrap_or(next_collect)),
                    if trial_deadline.is_some() && self.updater.in_trial() => {
                    if let Some(action) = self.updater.expire_trial() {
                        self.after_stop = Some(action);
                        break;
                    }
                }
            }
        }

        self.commands.finish(SHUTDOWN_COMMAND_TIMEOUT).await;
        if let Some(relay) = self.relay.take() {
            if self.after_stop.is_some() {
                // Arrêt pour la mise à jour : le signal d'arrêt n'a pas été
                // donné, le relais ne rendrait pas la main de lui-même.
                relay.abort();
            } else {
                // Le relais observe le même signal d'arrêt : on attend
                // seulement qu'il ait rendu la main.
                let _ = relay.await;
            }
        }
        self.logs.finish(self.after_stop.is_none()).await;
        self.final_flush().await;
        if let Some(action) = self.after_stop.take() {
            info!("agent stopped for its update");
            return crate::selfupdate::after_stop(action);
        }
        info!("agent stopped");
        Ok(())
    }

    /// Examine l'offre de mise à jour du dernier accusé de réception. Vrai si
    /// le nouveau binaire est en place : la boucle s'arrête pour lui céder la
    /// place.
    async fn apply_update(&mut self) -> bool {
        let Some(offer) = self.update_offer.take() else { return false };
        let key = self.identity.key();
        let after = self.updater.consider(&offer, &self.client, &key).await;
        // L'issue (refus, échec) part avec le lot suivant.
        self.identity.self_update = Some(self.updater.info());
        match after {
            Some(action) => {
                self.after_stop = Some(action);
                true
            }
            None => false,
        }
    }

    /// Vrai si l'agent doit relayer : `relay: true` en local, qui prime, ou le
    /// réglage de l'interface.
    fn relay_wanted(&self) -> bool {
        self.config.relay || self.server_relay.as_ref().is_some_and(|r| r.enabled)
    }

    /// Site annoncé : celui de la configuration locale s'il y en a un, sinon
    /// celui réglé dans l'interface.
    fn relay_site(&self) -> Option<String> {
        self.config.site.clone().or_else(|| {
            self.server_relay
                .as_ref()
                .and_then(|r| r.site.as_deref())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        })
    }

    /// Adopte le mode relais que le serveur vient de renvoyer, et le reporte
    /// dans l'identité : le lot suivant dit au serveur ce qui est en vigueur.
    fn adopt_relay(&mut self, setting: Option<RelaySetting>) {
        // Un serveur antérieur ne dit rien : on garde ce qu'on avait.
        let Some(setting) = setting else { return };
        if self.server_relay.as_ref() != Some(&setting) {
            if self.config.relay && !setting.enabled {
                debug!("relay switched off from the server, but relay: true in agent.yaml wins");
            }
            self.server_relay = Some(setting);
        }
        self.identity.relay = self.relay_wanted();
        self.identity.site = self.relay_site();
    }

    /// Lance ou arrête la boucle du relais selon le réglage en vigueur, à chaud.
    fn sync_relay(&mut self, shutdown: &Shutdown) {
        // Une boucle qui a rendu la main (arrêt) ne compte plus.
        if self.relay.as_ref().is_some_and(|handle| handle.is_finished()) {
            self.relay = None;
        }
        match (self.relay_wanted(), self.relay.is_some()) {
            (true, false) => {
                let runner = RelayRunner::new(
                    self.client.clone(),
                    self.identity.key(),
                    RELAY_REQUEST_TIMEOUT,
                );
                if !self.config.relay {
                    info!(site = ?self.relay_site(), "relay mode switched on from the server");
                }
                self.relay = Some(tokio::spawn(runner.run(shutdown.clone())));
            }
            (false, true) => {
                // L'attente longue est simplement abandonnée ; une sonde déjà
                // confiée finit dans sa propre tâche et rend compte quand même.
                if let Some(handle) = self.relay.take() {
                    handle.abort();
                }
                info!("relay mode switched off from the server");
            }
            _ => {}
        }
    }

    /// Envoie un lot prélevé dans le tampon.
    async fn flush(&mut self) -> FlushOutcome {
        self.ensure_tls().await;
        if self.tls_only() && !self.client.is_encrypted() {
            // Jamais en clair une fois le canal chiffré exigé localement : les
            // mesures attendent l'enrôlement.
            return FlushOutcome::Failed(PushError::Transport(
                "waiting for the encrypted agent channel (enrolment pending)".to_string(),
            ));
        }
        let samples = self.buffer.take(self.batch_limit);
        if samples.is_empty() {
            return FlushOutcome::Idle;
        }
        let count = samples.len();
        let mut batch =
            PushBatch::new(self.identity.clone(), samples, chrono::Utc::now().timestamp_millis());
        if self.wants_inventory() {
            batch.inventory = inventory::lock(&self.inventory).clone();
        }
        batch.event_log = self.eventlog.report();
        batch.declared_monitors = self.declared.clone();

        match self.client.send(&batch).await {
            Ok(ack) => {
                if let Some((since, _)) = self.outage.take() {
                    info!(
                        attempts = self.backoff.attempts(),
                        outage_s = since.elapsed().as_secs(),
                        "connection to the server restored"
                    );
                }
                self.backoff.reset();
                self.binding_refused = false;
                debug!(target = ack.target_id, accepted = ack.accepted, "batch accepted");
                self.note_acceptance(count, ack.accepted);
                self.forget_inventory(batch.inventory.as_ref());
                self.eventlog.acknowledge(batch.event_log.as_ref());
                self.adopt_binding(ack.agent_secret.clone(), ack.bound);
                self.adopt_interval(ack.interval_secs);
                self.adopt_disabled_modules(ack.disabled_modules);
                self.adopt_relay(ack.relay);
                // `None` : serveur antérieur, ou machine qui n'est pas sous
                // Windows ; les règles connues restent.
                if let Some(rules) = ack.event_rules {
                    self.eventlog.set_rules(rules);
                }
                self.server_logs = ack.logs;
                // Nouveau binaire à l'essai : ce lot accepté le confirme.
                if self.updater.confirm() {
                    self.identity.self_update = Some(self.updater.info());
                }
                self.update_offer = ack.agent_update;
                FlushOutcome::Sent { count }
            }
            Err(PushError::Forbidden(message)) if message == dumbmonit_proto::MTLS_REQUIRED => {
                // L'ancien canal est fermé : l'annonce qui accompagne ce refus
                // déclenche la migration au prochain tour.
                self.buffer.restore(batch.samples);
                self.tls_retry_at = None;
                info!("the server requires the encrypted agent channel, switching");
                FlushOutcome::Failed(PushError::Forbidden(message))
            }
            Err(PushError::Forbidden(message)) => {
                self.buffer.restore(batch.samples);
                self.on_binding_refused(&message);
                FlushOutcome::Failed(PushError::Forbidden(message))
            }
            Err(PushError::CertificateRefused) => {
                self.buffer.restore(batch.samples);
                self.on_certificate_refused();
                FlushOutcome::Failed(PushError::CertificateRefused)
            }
            Err(PushError::TooLarge(message)) => {
                if batch.inventory.is_some() {
                    // L'inventaire est le premier suspect : on le lâche pour ce
                    // jour (relevé suivant demain) plutôt que de tailler les mesures.
                    warn!(reason = message, "inventory too large for the server, dropped");
                    self.forget_inventory(batch.inventory.as_ref());
                    self.buffer.restore(batch.samples);
                    return FlushOutcome::Failed(PushError::TooLarge(message));
                }
                if batch.event_log.is_some() {
                    // Puis les événements du journal, que rien ne permet de couper.
                    warn!(reason = message, "event log report too large for the server, dropped");
                    self.eventlog.acknowledge(batch.event_log.as_ref());
                    self.buffer.restore(batch.samples);
                    return FlushOutcome::Failed(PushError::TooLarge(message));
                }
                if count <= 1 {
                    // Une mesure seule ne se coupe plus : la garder bloquerait
                    // tout le tampon derrière elle.
                    warn!(reason = message, "measurement too large for the server, dropped");
                    return FlushOutcome::Sent { count };
                }
                self.buffer.restore(batch.samples);
                self.batch_limit = (count / 2).max(1);
                warn!(
                    reason = message,
                    batch_limit = self.batch_limit,
                    "batch too large for the server or a proxy, sending smaller batches"
                );
                FlushOutcome::Failed(PushError::TooLarge(message))
            }
            Err(error) if error.is_retryable() => {
                // Journalisé par la boucle, qui connaît le délai avant la reprise.
                self.buffer.restore(batch.samples);
                FlushOutcome::Failed(error)
            }
            Err(error) => {
                // Le serveur a compris et refusé : réémettre à l'identique ne ferait
                // que reproduire le refus, en gardant le tampon plein pour rien.
                warn!(%error, dropped = count, "batch rejected by the server, dropped");
                self.forget_inventory(batch.inventory.as_ref());
                self.eventlog.acknowledge(batch.event_log.as_ref());
                FlushOutcome::Sent { count }
            }
        }
    }

    /// Vrai si rien ne doit plus partir en clair : empreinte donnée par la
    /// commande d'installation, ou agent déjà enrôlé une fois.
    fn tls_only(&self) -> bool {
        self.config.ca_pin.is_some() || self.tls.as_ref().is_some_and(|state| state.locked)
    }

    /// Met le canal chiffré en place, ou le maintient : migration depuis
    /// l'ancien canal sur l'annonce du serveur, enrôlement, renouvellement,
    /// empreintes tenues à jour.
    async fn ensure_tls(&mut self) {
        if self.tls.is_none() {
            // Un serveur à jour annonce son canal chiffré sur l'ancien : un
            // agent installé avant migre de lui-même, en épinglant l'autorité
            // annoncée (confiance au premier usage).
            let Some(offer) = self.client.offer() else { return };
            let url = self.config.tls_url.clone().or_else(|| {
                dumbmonit_proto::agent_channel_url(&self.config.server_url, offer.port)
            });
            let Some(url) = url else { return };
            info!(url, "the server offers an encrypted agent channel, enrolling");
            self.client.set_pins(offer.pins.clone());
            self.tls = Some(TlsState::new(url, offer.pins));
        }
        let now_ms = chrono::Utc::now().timestamp_millis();
        if self.client.is_encrypted() {
            self.adopt_pins();
        }
        if self.tls_retry_at.is_some_and(|at| Instant::now() < at) {
            return;
        }
        let Some(state) = self.tls.clone() else { return };
        if state.usable(now_ms) {
            if state.needs_renewal(now_ms) {
                self.renew_certificate().await;
            }
            return;
        }
        self.enrol_tls(state).await;
    }

    /// Range les empreintes que le serveur vient d'annoncer sur le canal
    /// chiffré (rotation de l'autorité).
    fn adopt_pins(&mut self) {
        let pins = self.client.pins();
        let Some(state) = self.tls.as_mut() else { return };
        if pins.is_empty() || pins == state.pins {
            return;
        }
        info!(pins = pins.join(","), "pinned server authorities updated");
        state.pins = pins;
        if state.locked
            && let Err(error) = state.save(&self.tls_path)
        {
            warn!(error = %format!("{error:#}"), "cannot store the encrypted channel state");
        }
    }

    /// Enrôlement : une clé neuve, le jeton (et le secret de liaison), un
    /// certificat en retour.
    async fn enrol_tls(&mut self, mut state: TlsState) {
        let (public_key, key) = match crate::tls::new_key() {
            Ok(pair) => pair,
            Err(error) => {
                warn!(error = %format!("{error:#}"), "cannot generate the agent key");
                self.tls_retry_at = Some(Instant::now() + TLS_RETRY);
                return;
            }
        };
        let request =
            dumbmonit_proto::TlsEnrolRequest { identity: self.identity.clone(), public_key };
        match self.client.enrol(&state.url, &request).await {
            Ok(issued) => {
                if let Some(secret) = issued.agent_secret.clone() {
                    self.adopt_binding(Some(secret), true);
                }
                self.adopt_certificate(&mut state, issued, &key);
                info!(url = state.url, "enrolled on the encrypted agent channel (TLS 1.3, mTLS)");
            }
            Err(error) => {
                if let PushError::Forbidden(message) = &error {
                    self.on_binding_refused(message);
                }
                self.note_tls_failure("enrolment on the encrypted agent channel failed", &error);
            }
        }
    }

    /// Renouvellement sous le certificat en cours, avec une clé neuve.
    async fn renew_certificate(&mut self) {
        let Some(mut state) = self.tls.clone() else { return };
        let (public_key, key) = match crate::tls::new_key() {
            Ok(pair) => pair,
            Err(error) => {
                warn!(error = %format!("{error:#}"), "cannot generate the agent key");
                self.tls_retry_at = Some(Instant::now() + TLS_RETRY);
                return;
            }
        };
        match self.client.renew(public_key).await {
            Ok(issued) => {
                self.adopt_certificate(&mut state, issued, &key);
                info!("agent certificate renewed");
            }
            Err(PushError::CertificateRefused) => self.on_certificate_refused(),
            Err(error) => self.note_tls_failure("agent certificate renewal failed", &error),
        }
    }

    /// Range un certificat reçu et bascule le client dessus.
    fn adopt_certificate(
        &mut self,
        state: &mut TlsState,
        issued: dumbmonit_proto::TlsCertificate,
        key: &[u8],
    ) {
        use base64::Engine as _;
        state.certificate = Some(issued.certificate);
        state.key = Some(base64::engine::general_purpose::STANDARD.encode(key));
        state.not_after_ms = Some(issued.not_after_ms);
        if !issued.pins.is_empty() {
            state.pins = issued.pins;
        }
        state.locked = true;
        self.client.set_pins(state.pins.clone());
        if let Err(error) = state.save(&self.tls_path) {
            // Gardé en mémoire : l'agent marche, mais se réenrôlera au prochain
            // démarrage.
            warn!(error = %format!("{error:#}"), "cannot store the encrypted channel state");
        }
        if let Err(error) = self.client.use_tls(&state.url, state.identity()) {
            warn!(error = %format!("{error:#}"), "cannot switch to the encrypted agent channel");
        }
        self.tls = Some(state.clone());
        self.tls_retry_at = None;
        self.tls_failed = false;
    }

    /// Le serveur ne reconnaît plus notre certificat (machine supprimée,
    /// certificat remplacé) : on l'oublie, et on se réenrôle avec le jeton.
    fn on_certificate_refused(&mut self) {
        let Some(state) = self.tls.as_mut() else { return };
        if state.certificate.is_none() {
            return;
        }
        warn!("the server no longer accepts this agent certificate, enrolling again");
        state.drop_certificate();
        if let Err(error) = state.save(&self.tls_path) {
            warn!(error = %format!("{error:#}"), "cannot store the encrypted channel state");
        }
        if let Err(error) = self.client.use_tls(&state.url, None) {
            warn!(error = %format!("{error:#}"), "cannot reset the encrypted agent channel");
        }
        self.tls_retry_at = None;
    }

    fn note_tls_failure(&mut self, what: &str, error: &PushError) {
        self.tls_retry_at = Some(Instant::now() + TLS_RETRY);
        if self.tls_failed {
            debug!(%error, "{what}");
        } else {
            self.tls_failed = true;
            warn!(%error, retry_in_s = TLS_RETRY.as_secs(), "{what}");
        }
    }

    /// Journalise un échec d'envoi : chaque tentative laisse une ligne, avec sa
    /// cause (statut HTTP, erreur réseau, délai dépassé) et le délai avant la
    /// suivante. Fort au premier échec et à chaque changement de cause, en
    /// `info` ensuite — et seulement une fois sur dix passé la dixième, pour
    /// qu'une panne d'une nuit ne remplisse pas le journal.
    fn note_failure(&mut self, error: &PushError, retry_in: Duration) {
        let attempt = self.backoff.attempts();
        let cause = error.to_string();
        let retry_in_s = retry_in.as_secs();
        let buffered = self.buffer.len();
        let Some((_, last)) = self.outage.as_mut() else {
            self.outage = Some((Instant::now(), cause.clone()));
            // Ces deux refus ont déjà leur propre message, plus parlant.
            if !matches!(error, PushError::Forbidden(_) | PushError::TooLarge(_)) {
                warn!(error = %cause, buffered, retry_in_s, "send failed, measurements kept");
            }
            return;
        };
        if *last != cause {
            *last = cause.clone();
            warn!(error = %cause, attempt, buffered, retry_in_s, "send still failing");
        } else if attempt <= 10 || attempt.is_multiple_of(10) {
            info!(error = %cause, attempt, buffered, retry_in_s, "send failed again");
        } else {
            debug!(error = %cause, attempt, retry_in_s, "send failed again");
        }
    }

    /// Fichier des mesures en attente gardées à l'arrêt, à côté du secret de
    /// liaison : le seul répertoire où l'agent sait pouvoir écrire.
    fn pending_path(&self) -> std::path::PathBuf {
        self.config.secret_path.with_file_name("pending-samples.json")
    }

    /// Reprend les mesures qu'un arrêt précédent n'avait pas pu envoyer.
    fn restore_pending(&mut self) {
        let path = self.pending_path();
        let samples = PendingBuffer::load(&path);
        if !samples.is_empty() {
            info!(count = samples.len(), "measurements kept at the last shutdown, sending them");
            self.buffer.push(samples);
        }
    }

    /// Signale un lot non vide dont le serveur n'a gardé aucune mesure.
    ///
    /// Le serveur écarte sans erreur les mesures hors de `[maintenant - 30 j,
    /// maintenant + 1 h]` : une horloge déréglée sur la machine les fait toutes
    /// disparaître, et rien ne le disait. Renvoie vrai si l'avertissement part.
    fn note_acceptance(&mut self, sent: usize, accepted: usize) -> bool {
        if sent == 0 || accepted > 0 {
            self.all_refused = false;
            return false;
        }
        if self.all_refused {
            return false;
        }
        self.all_refused = true;
        warn!(
            sent,
            "the server kept none of the measurements of this batch: check this machine's \
             clock (timestamps more than 1 hour ahead or 30 days behind the server are discarded)"
        );
        true
    }

    /// Dernière tentative d'envoi avant de rendre la main au système.
    async fn final_flush(&mut self) {
        if self.buffer.is_empty() {
            return;
        }
        if let Ok(FlushOutcome::Sent { count }) =
            tokio::time::timeout(SHUTDOWN_FLUSH_TIMEOUT, self.flush()).await
        {
            info!(count, "pending measurements sent before shutdown");
        }
        if self.buffer.is_empty() {
            return;
        }
        // Le serveur n'a pas répondu (ou pas pour tout) : ce qui reste attend le
        // prochain démarrage sur disque plutôt que d'être perdu.
        let pending = self.buffer.len();
        let path = self.pending_path();
        match self.buffer.save(&path, MAX_PERSISTED_SAMPLES) {
            Ok(kept) => info!(
                kept,
                lost = pending.saturating_sub(kept),
                path = %path.display(),
                "the server did not answer before shutdown: measurements kept on disk"
            ),
            Err(error) => warn!(
                pending,
                %error,
                "measurements lost: the server did not answer before shutdown"
            ),
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

/// Canal chiffré au démarrage : l'état rangé, confronté à la configuration.
///
/// Une empreinte donnée par la commande d'installation qui ne figure pas dans
/// l'état rangé, c'est une réinstallation vers un autre serveur (ou une autre
/// autorité) : on repart d'elle.
fn initial_tls(config: &Config, stored: Option<TlsState>) -> Option<TlsState> {
    let configured_url = || {
        config.tls_url.clone().or_else(|| {
            dumbmonit_proto::agent_channel_url(
                &config.server_url,
                dumbmonit_proto::DEFAULT_AGENT_TLS_PORT,
            )
        })
    };
    match (stored, &config.ca_pin) {
        (Some(mut state), Some(pin)) if state.pins.contains(pin) => {
            if let Some(url) = config.tls_url.clone() {
                state.url = url;
            }
            Some(state)
        }
        (_, Some(pin)) => Some(TlsState::new(configured_url()?, vec![pin.clone()])),
        (Some(mut state), None) => {
            if let Some(url) = config.tls_url.clone() {
                state.url = url;
            }
            Some(state)
        }
        (None, None) => None,
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
            actions: crate::remediation::ActionsConfig::default(),
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
            inventory: false,
            self_update: true,
            backup_repos: crate::collect::backup_repos::BackupReposConfig::default(),
            perf_counters: crate::collect::perf_counters::PerfCountersConfig::default(),
            max_buffered_samples: 1_000,
            secret_path: std::path::PathBuf::from("/inexistant/agent-secret"),
            tls_url: None,
            ca_pin: None,
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

    fn setting(enabled: bool, site: Option<&str>) -> Option<RelaySetting> {
        Some(RelaySetting { enabled, site: site.map(str::to_string) })
    }

    #[tokio::test]
    async fn the_relay_mode_set_from_the_server_switches_on_and_off_without_restart() {
        let mut agent = Agent::new(config()).expect("agent");
        let (trigger, shutdown) = crate::shutdown::channel();
        assert!(agent.identity.relay_control, "ce binaire sait suivre le serveur");
        assert!(!agent.identity.relay_local);

        // Serveur antérieur : rien ne change.
        agent.adopt_relay(None);
        agent.sync_relay(&shutdown);
        assert!(agent.relay.is_none());
        assert!(!agent.identity.relay);

        // Allumé dans l'interface : la boucle démarre, l'identité l'annonce.
        agent.adopt_relay(setting(true, Some(" Agence de Lyon ")));
        agent.sync_relay(&shutdown);
        assert!(agent.relay.is_some(), "boucle du relais lancée à chaud");
        assert!(agent.identity.relay);
        assert_eq!(agent.identity.site.as_deref(), Some("Agence de Lyon"));

        // Un second accusé identique ne relance rien.
        agent.adopt_relay(setting(true, Some("Agence de Lyon")));
        agent.sync_relay(&shutdown);
        assert!(agent.relay.is_some());

        // Éteint : la boucle s'arrête, l'identité le dit au lot suivant.
        agent.adopt_relay(setting(false, None));
        agent.sync_relay(&shutdown);
        assert!(agent.relay.is_none(), "boucle du relais arrêtée à chaud");
        assert!(!agent.identity.relay);
        assert_eq!(agent.identity.site, None);
        trigger.fire();
    }

    #[tokio::test]
    async fn relay_true_in_the_local_configuration_wins_over_the_server() {
        let mut config = config();
        config.relay = true;
        config.site = Some("Datacenter".into());
        let mut agent = Agent::new(config).expect("agent");
        let (trigger, shutdown) = crate::shutdown::channel();
        assert!(agent.identity.relay_local, "le serveur doit savoir que c'est verrouillé");

        agent.adopt_relay(setting(false, Some("Ailleurs")));
        agent.sync_relay(&shutdown);
        assert!(agent.relay.is_some(), "le relais local ne s'éteint pas depuis le serveur");
        assert!(agent.identity.relay);
        assert_eq!(agent.identity.site.as_deref(), Some("Datacenter"), "le site local prime");
        trigger.fire();
    }

    #[tokio::test]
    async fn the_inventory_rides_the_next_batch_and_is_forgotten_once_accepted() {
        let dir = tempfile::tempdir().unwrap();
        let answer = std::sync::Arc::new(std::sync::Mutex::new((503u16, "{}".to_string())));
        let shared = answer.clone();
        let (url, calls) =
            crate::testsupport::fake_http(move |_| shared.lock().unwrap().clone()).await;
        let mut agent = agent_for(&url, &dir);
        let samples = agent.collect().await;
        // Activé après la collecte : la tâche de fond n'a pas à tourner ici.
        agent.config.inventory = true;
        let relever = || SoftwareInventory {
            collected_at_ms: 42,
            source: "dpkg".into(),
            ..SoftwareInventory::default()
        };
        *inventory::lock(&agent.inventory) = Some(relever());

        // Échec réseau côté serveur : l'inventaire part, puis reste en attente.
        agent.buffer.push(samples.clone());
        assert!(matches!(agent.flush().await, FlushOutcome::Failed(_)));
        assert!(calls.bodies("POST", "/")[0].contains("\"inventory\""));
        assert!(inventory::lock(&agent.inventory).is_some(), "conservé pour le lot suivant");

        // Accepté : il est oublié.
        *answer.lock().unwrap() = (200, ACK.to_string());
        assert!(matches!(agent.flush().await, FlushOutcome::Sent { .. }));
        assert!(calls.bodies("POST", "/")[1].contains("\"inventory\""));
        assert!(inventory::lock(&agent.inventory).is_none());

        // Le lot suivant n'en porte plus.
        agent.buffer.push(samples.clone());
        assert!(matches!(agent.flush().await, FlushOutcome::Sent { .. }));
        assert!(!calls.bodies("POST", "/")[2].contains("\"inventory\""));

        // Décoché dans l'interface : rien n'est joint, même s'il en reste un.
        *inventory::lock(&agent.inventory) = Some(relever());
        agent.adopt_disabled_modules(vec!["inventory".into()]);
        agent.buffer.push(samples);
        assert!(matches!(agent.flush().await, FlushOutcome::Sent { .. }));
        assert!(!calls.bodies("POST", "/")[3].contains("\"inventory\""));
    }

    #[tokio::test]
    async fn the_ack_of_a_server_switches_the_relay_on() {
        let dir = tempfile::tempdir().unwrap();
        let ack = r#"{"target_id":3,"accepted":1,"interval_secs":60,"bound":true,
            "relay":{"enabled":true,"site":"Lyon"}}"#;
        let (url, _calls) = crate::testsupport::fake_http(move |_| (200, ack.to_string())).await;
        let mut agent = agent_for(&url, &dir);
        let samples = agent.collect().await;
        agent.buffer.push(samples);
        assert!(matches!(agent.flush().await, FlushOutcome::Sent { .. }));
        assert!(agent.identity.relay);
        assert_eq!(agent.identity.site.as_deref(), Some("Lyon"));
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

    const PIN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    /// Un port local fermé : rien n'y répond.
    fn closed_port() -> u16 {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    }

    #[tokio::test]
    async fn with_a_pinned_server_nothing_is_ever_sent_in_clear() {
        // L'agent installé avec l'empreinte n'a pas encore pu s'enrôler (port
        // des agents injoignable) : ses mesures attendent, elles ne partent pas
        // en clair vers l'ancien canal pour autant.
        let dir = tempfile::tempdir().unwrap();
        let (url, calls) = crate::testsupport::fake_http(|_| (200, ACK.to_string())).await;
        let mut config = config();
        config.server_url = url;
        config.secret_path = dir.path().join("agent-secret");
        config.tls_url = Some(format!("https://127.0.0.1:{}", closed_port()));
        config.ca_pin = Some(PIN.to_string());
        let mut agent = Agent::new(config).expect("agent");
        assert!(agent.client.is_encrypted(), "le client ne connaît plus que le canal chiffré");
        let samples = agent.collect().await;
        agent.buffer.push(samples);

        assert!(matches!(agent.flush().await, FlushOutcome::Failed(_)));
        assert!(!agent.buffer.is_empty(), "les mesures sont gardées");
        assert!(calls.all().is_empty(), "rien n'est parti en clair : {:?}", calls.lines());
    }

    #[tokio::test]
    async fn an_agent_migrating_from_the_plain_channel_keeps_reporting_until_enrolled() {
        // Annonce reçue sur l'ancien canal, port des agents fermé : l'agent
        // tente l'enrôlement, échoue, et continue par l'ancien canal — il n'a
        // encore jamais été enrôlé, rien ne l'en empêche.
        let dir = tempfile::tempdir().unwrap();
        let (url, calls) = crate::testsupport::fake_http(|_| (200, ACK.to_string())).await;
        let mut agent = agent_for(&url, &dir);
        agent.client.set_offer(dumbmonit_proto::TlsOffer {
            port: closed_port(),
            pins: vec![PIN.to_string()],
        });
        let samples = agent.collect().await;
        agent.buffer.push(samples);

        assert!(matches!(agent.flush().await, FlushOutcome::Sent { .. }));
        let state = agent.tls.clone().expect("canal adopté");
        assert!(state.url.starts_with("https://127.0.0.1:"), "{state:?}");
        assert_eq!(state.pins, [PIN]);
        assert!(!state.locked);
        assert_eq!(calls.count("POST", "/api/ingest"), 1);
        assert!(agent.tls_retry_at.is_some(), "nouvel essai d'enrôlement plus tard");
    }

    #[tokio::test]
    async fn once_enrolled_an_agent_never_falls_back_to_the_plain_channel() {
        // Un état rangé verrouillé (déjà enrôlé une fois), certificat perdu :
        // l'agent doit se réenrôler, pas reparler en clair.
        let dir = tempfile::tempdir().unwrap();
        let mut state =
            TlsState::new(format!("https://127.0.0.1:{}", closed_port()), vec![PIN.into()]);
        state.locked = true;
        state.save(&crate::tls::beside(&dir.path().join("agent-secret"))).unwrap();
        let (url, calls) = crate::testsupport::fake_http(|_| (200, ACK.to_string())).await;
        let mut agent = agent_for(&url, &dir);
        let samples = agent.collect().await;
        agent.buffer.push(samples);

        assert!(matches!(agent.flush().await, FlushOutcome::Failed(_)));
        assert!(calls.all().is_empty(), "{:?}", calls.lines());
    }

    #[test]
    fn a_new_pin_in_the_install_command_replaces_the_stored_channel() {
        let mut stored = TlsState::new("https://ancien:8443".into(), vec!["f".repeat(64)]);
        stored.certificate = Some("Y2VydA==".into());
        stored.locked = true;
        let mut config = config();
        config.server_url = "http://serveur:8080".into();
        config.ca_pin = Some(PIN.to_string());
        let state = initial_tls(&config, Some(stored.clone())).expect("canal");
        assert_eq!(state.pins, [PIN]);
        assert_eq!(state.certificate, None, "autre serveur : on repart de zéro");
        assert_eq!(state.url, "https://serveur:8443");

        // La même empreinte : l'état rangé est gardé tel quel.
        config.ca_pin = Some("f".repeat(64));
        assert_eq!(initial_tls(&config, Some(stored.clone())), Some(stored));
        // Ni configuration ni état : pas de canal chiffré (serveur antérieur).
        config.ca_pin = None;
        assert_eq!(initial_tls(&config, None), None);
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
    async fn a_proxy_error_without_the_server_envelope_keeps_the_backlog() {
        // Un mandataire répond 404 pendant une maintenance : tout le retard
        // accumulé doit survivre, et repartir une fois le chemin rétabli.
        let dir = tempfile::tempdir().unwrap();
        let answer = std::sync::Arc::new(std::sync::Mutex::new((
            404u16,
            "<html><body>404 Not Found</body></html>".to_string(),
        )));
        let shared = answer.clone();
        let (url, _) = crate::testsupport::fake_http(move |_| shared.lock().unwrap().clone()).await;
        let mut agent = agent_for(&url, &dir);
        let samples = agent.collect().await;
        let count = samples.len();
        agent.buffer.push(samples);

        assert!(matches!(agent.flush().await, FlushOutcome::Failed(_)));
        assert_eq!(agent.buffer.len(), count, "le retard ne doit pas être jeté");

        *answer.lock().unwrap() = (200, ACK.to_string());
        assert!(matches!(agent.flush().await, FlushOutcome::Sent { .. }));
        assert!(agent.buffer.is_empty());
    }

    #[tokio::test]
    async fn a_batch_too_large_for_a_proxy_is_split_until_it_passes() {
        // Le mandataire refuse tout corps de plus de 3 mesures.
        let dir = tempfile::tempdir().unwrap();
        let (url, calls) = crate::testsupport::fake_http(|req| {
            let samples = req.body.matches("\"ts_ms\"").count();
            if samples > 3 {
                (413, "<html>413 Request Entity Too Large</html>".to_string())
            } else {
                (200, ACK.to_string())
            }
        })
        .await;
        let mut agent = agent_for(&url, &dir);
        let samples: Vec<Sample> = (0..10)
            .map(|i| Sample::new("cpu_usage_percent", 1.0, dumbmonit_proto::MetricKind::Gauge, i))
            .collect();
        agent.buffer.push(samples);

        let mut rounds = 0;
        while !agent.buffer.is_empty() {
            rounds += 1;
            assert!(rounds < 50, "le lot ne passe jamais");
            agent.flush().await;
        }
        // Rien n'a été perdu : les dix mesures sont arrivées, par lots de 3 au plus.
        let delivered: usize = calls
            .all()
            .iter()
            .map(|req| req.body.matches("\"ts_ms\"").count())
            .filter(|&n| n <= 3)
            .sum();
        assert_eq!(delivered, 10);
        assert!(agent.batch_limit <= 3);
    }

    #[tokio::test]
    async fn a_single_measurement_too_large_for_any_proxy_is_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let (url, _) =
            crate::testsupport::fake_http(|_| (413, "Request Entity Too Large".to_string())).await;
        let mut agent = agent_for(&url, &dir);
        agent.buffer.push(vec![Sample::new(
            "cpu_usage_percent",
            1.0,
            dumbmonit_proto::MetricKind::Gauge,
            1,
        )]);
        let mut rounds = 0;
        while !agent.buffer.is_empty() {
            rounds += 1;
            assert!(rounds < 50, "une mesure seule ne doit pas bloquer le tampon");
            agent.flush().await;
        }
    }

    #[test]
    fn a_batch_entirely_refused_is_reported_once_as_a_clock_problem() {
        let mut agent = Agent::new(config()).expect("agent");
        assert!(!agent.note_acceptance(10, 10));
        assert!(!agent.note_acceptance(0, 0), "un lot vide ne dit rien");
        assert!(agent.note_acceptance(10, 0), "tout refusé : on le dit");
        assert!(!agent.note_acceptance(10, 0), "une seule fois");
        assert!(!agent.note_acceptance(10, 10));
        assert!(agent.note_acceptance(10, 0), "et de nouveau après un retour à la normale");
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

    // ------------------------------------------- coupures et agent relais

    #[test]
    fn the_retry_delay_never_exceeds_a_third_of_the_server_silence_limit() {
        // Le serveur déclare une machine muette après 90 s au moins : un agent qui
        // attend plus longtemps entre deux tentatives fabrique des coupures.
        let mut backoff = Backoff::new(BACKOFF_BASE, BACKOFF_MAX);
        for _ in 0..50 {
            assert!(backoff.next_delay() <= Duration::from_secs(30));
        }
    }

    /// Accusé qui règle la période à 5 s, le plancher : les tests restent courts.
    const FAST_ACK: &str = r#"{"target_id":3,"accepted":1,"interval_secs":5,"bound":true}"#;

    /// Agent relais pointé sur le faux serveur, sans module ni commande.
    fn relay_agent_for(url: &str, dir: &tempfile::TempDir) -> Agent {
        let mut config = config();
        config.server_url = url.to_string();
        config.secret_path = dir.path().join("agent-secret");
        config.interval = Duration::from_secs(5);
        config.relay = true;
        config.modules = Some(Default::default());
        Agent::new(config).expect("agent")
    }

    /// Écoute qui accepte les connexions et ne répond jamais : l'équipement lent
    /// ou muet d'un site distant. Rend l'adresse et le nombre de connexions tenues.
    async fn tarpit() -> (std::net::SocketAddr, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("écoute");
        let address = listener.local_addr().expect("adresse");
        let held = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count = held.clone();
        tokio::spawn(async move {
            let mut sockets = Vec::new();
            while let Ok((socket, _)) = listener.accept().await {
                count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                sockets.push(socket);
            }
        });
        (address, held)
    }

    #[tokio::test]
    async fn a_slow_relayed_probe_never_delays_the_reports_of_the_relay_itself() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let (device, held) = tarpit().await;
        let target = dumbmonit_proto::Target {
            id: 41,
            name: "pve002".into(),
            address: format!("http://{device}/"),
            kind: "http".into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            // Budget de la sonde HTTP au maximum (5 s par défaut) : elle reste
            // pendue bien au-delà de la période des rapports.
            tags: [
                ("allow_private_targets".to_string(), "true".to_string()),
                ("timeout_seconds".to_string(), "60".to_string()),
            ]
            .into(),
            credential: dumbmonit_proto::Credential::None,
            group_name: String::new(),
            position: 0,
        };
        let job = dumbmonit_proto::ProbeJob { target, timeout_secs: 60, discover: false };
        let jobs = serde_json::to_string(&[job.into_command(1, 0)]).unwrap();
        let polls = std::sync::Arc::new(AtomicUsize::new(0));
        let seen = polls.clone();
        let (url, calls) = crate::testsupport::fake_http(move |req| match req.method.as_str() {
            "POST" if req.path == "/api/ingest" => (200, FAST_ACK.to_string()),
            // La sonde lente part au premier tour ; ensuite le canal est en panne,
            // pour que le relais temporise au lieu de tourner à vide.
            "GET" if seen.fetch_add(1, Ordering::SeqCst) == 0 => (200, jobs.clone()),
            _ => (503, "busy".to_string()),
        })
        .await;
        let dir = tempfile::tempdir().unwrap();
        let agent = relay_agent_for(&url, &dir);
        let (_trigger, shutdown) = crate::shutdown::channel();

        let check = async {
            let started = std::time::Instant::now();
            // La sonde est entre les mains de l'équipement muet…
            while held.load(Ordering::SeqCst) == 0 {
                assert!(started.elapsed() < Duration::from_secs(20), "sonde jamais lancée");
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            // …et le rapport suivant de la machine part à son heure (période de
            // 5 s), sans attendre la fin de la sonde (60 s).
            let before = calls.count("POST", "/api/ingest");
            let stuck = std::time::Instant::now();
            while calls.count("POST", "/api/ingest") <= before {
                assert!(
                    stuck.elapsed() < Duration::from_secs(9),
                    "rapport bloqué derrière la sonde relayée"
                );
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            let reported = calls.count("POST", "/api/agent/relay/1");
            assert_eq!(reported, 0, "la sonde devait être encore en vol");
        };
        tokio::select! {
            _ = agent.run(shutdown) => panic!("la boucle s'est arrêtée seule"),
            () = check => {}
        }
    }

    #[tokio::test]
    async fn the_relay_channel_coming_back_ends_the_retry_delay_at_once() {
        use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
        let ingests = std::sync::Arc::new(AtomicUsize::new(0));
        let failed = std::sync::Arc::new(AtomicBool::new(false));
        let answered = std::sync::Arc::new(AtomicBool::new(false));
        let (i, f, a) = (ingests.clone(), failed.clone(), answered.clone());
        let (url, calls) = crate::testsupport::fake_http(move |req| match req.method.as_str() {
            // Premier lot accepté (il lance le relais), le second refusé : la
            // coupure. Puis tout repasse.
            "POST" if req.path == "/api/ingest" => match i.fetch_add(1, Ordering::SeqCst) {
                1 => {
                    f.store(true, Ordering::SeqCst);
                    (503, "restarting".to_string())
                }
                _ => (200, FAST_ACK.to_string()),
            },
            // Le canal du relais répond une fois, après la coupure des mesures.
            "GET" if f.load(Ordering::SeqCst) && !a.swap(true, Ordering::SeqCst) => {
                (200, "[]".to_string())
            }
            _ => (503, "restarting".to_string()),
        })
        .await;
        let dir = tempfile::tempdir().unwrap();
        let mut agent = relay_agent_for(&url, &dir);
        // Sans le réveil, la reprise attendrait une minute entière.
        agent.backoff = Backoff::new(Duration::from_secs(60), Duration::from_secs(60));
        let (_trigger, shutdown) = crate::shutdown::channel();

        let check = async {
            let started = std::time::Instant::now();
            while !answered.load(Ordering::SeqCst) {
                assert!(started.elapsed() < Duration::from_secs(40), "relais jamais revenu");
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            let back = std::time::Instant::now();
            while calls.count("POST", "/api/ingest") < 3 {
                assert!(
                    back.elapsed() < Duration::from_secs(10),
                    "les mesures attendent la fin de la temporisation"
                );
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        };
        tokio::select! {
            _ = agent.run(shutdown) => panic!("la boucle s'est arrêtée seule"),
            () = check => {}
        }
    }
}
