//! VictoriaMetrics embarqué : lancé et surveillé par le serveur lui-même.
//!
//! Sans `DUMBMONIT_VM_URL`, l'image se suffit : le binaire `victoria-metrics-prod`
//! qu'elle embarque est lancé comme processus enfant, sur la boucle locale, avec
//! ses données sous `/data/vm`. Un seul conteneur, un seul volume à sauvegarder.
//!
//! Le processus est traité comme une dépendance vitale : son journal remonte
//! dans le nôtre, il est relancé s'il meurt (avec un délai croissant pour ne
//! pas boucler sur une erreur de configuration), et il est arrêté proprement
//! avant nous — SIGTERM, vingt secondes de grâce, puis SIGKILL.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, Notify};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

/// Temps laissé au processus pour répondre sur `/health` au démarrage.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
/// Délai de grâce après SIGTERM avant de tuer le processus.
const STOP_GRACE: Duration = Duration::from_secs(20);
/// Premier délai avant relance après une sortie inattendue ; doublé à chaque
/// échec successif, jusqu'à [`MAX_RESTART_DELAY`].
const FIRST_RESTART_DELAY: Duration = Duration::from_secs(1);
const MAX_RESTART_DELAY: Duration = Duration::from_secs(30);
/// Un processus qui a tenu au moins ce temps remet le délai de relance à zéro.
const STABLE_AFTER: Duration = Duration::from_secs(60);
/// Attente maximale d'un redémarrage demandé (arrêt propre compris).
const RESTART_TIMEOUT: Duration = Duration::from_secs(60);

/// Ce que le processus est : la base de séries principale, celle de
/// l'historique long terme, ou VictoriaLogs. Les options diffèrent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flavor {
    /// VictoriaMetrics des mesures brutes.
    Metrics,
    /// VictoriaMetrics des agrégats cinq minutes (`/data/vm-long`).
    LongTerm,
    /// VictoriaLogs (`/data/vlogs`).
    Logs,
}

impl Flavor {
    /// Nom donné au processus dans notre journal.
    pub fn label(self) -> &'static str {
        match self {
            Self::Metrics => "victoria-metrics",
            Self::LongTerm => "victoria-metrics-long",
            Self::Logs => "victoria-logs",
        }
    }

    fn product(self) -> &'static str {
        match self {
            Self::Metrics | Self::LongTerm => "VictoriaMetrics",
            Self::Logs => "VictoriaLogs",
        }
    }
}

/// Réglages du processus embarqué, tirés de [`crate::config::Config`].
#[derive(Debug, Clone)]
pub struct EmbeddedConfig {
    pub flavor: Flavor,
    pub binary: PathBuf,
    pub data_path: PathBuf,
    pub listen: String,
    pub retention: String,
    pub memory: String,
    /// Clé tirée au hasard au démarrage, exigée par les routes d'administration
    /// de VictoriaMetrics (effacement de séries, instantanés, fusion forcée,
    /// pprof). Le processus n'écoute que sur la boucle locale, mais une requête
    /// relayée par erreur ne doit pas pouvoir y effacer quoi que ce soit : seul
    /// le client [`super::Victoria`] du serveur la connaît.
    pub admin_key: String,
    /// Adresse de `/health`, pour savoir quand le processus est prêt.
    pub health_url: String,
}

impl EmbeddedConfig {
    fn command(&self) -> Command {
        let mut command = Command::new(&self.binary);
        command
            .arg(format!("-storageDataPath={}", self.data_path.display()))
            .arg(format!("-httpListenAddr={}", self.listen))
            .arg(format!("-retentionPeriod={}", self.retention))
            .arg(format!("-memory.allowedBytes={}", self.memory));
        match self.flavor {
            Flavor::Metrics | Flavor::LongTerm => {
                command
                    .arg(format!("-deleteAuthKey={}", self.admin_key))
                    .arg(format!("-snapshotAuthKey={}", self.admin_key))
                    .arg(format!("-forceMergeAuthKey={}", self.admin_key))
                    .arg(format!("-pprofAuthKey={}", self.admin_key))
                    // Une instance DumbMonit n'interroge jamais plus de quelques
                    // milliers de séries : une requête qui s'emballe est coupée
                    // avant d'épuiser le budget mémoire ci-dessus.
                    .arg("-search.maxUniqueTimeseries=100000")
                    // Quatre requêtes simultanées suffisent à une interface plus
                    // le moteur d'alerting.
                    .arg("-search.maxConcurrentRequests=4");
                if self.flavor == Flavor::Metrics {
                    // Deux points à moins de dix secondes (réessai d'un agent,
                    // sonde exécutée deux fois) sont fusionnés : rien de perdu à
                    // 30 s de cadence.
                    command.arg("-dedup.minScrapeInterval=10s");
                } else {
                    // Un agrégat réécrit (reprise après un redémarrage) tombe
                    // sur le même horodatage : il remplace l'ancien au lieu de
                    // s'y ajouter.
                    command.arg("-dedup.minScrapeInterval=1m");
                }
            }
            Flavor::Logs => {}
        }
        command
            // Le journal du processus passe par le nôtre, sans horodatage ni
            // couleur : il en reçoit déjà de notre côté.
            .arg("-loggerDisableTimestamps=true")
            .arg("-loggerOutput=stderr")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            // Si le serveur meurt brutalement, le runtime tue l'enfant avec lui :
            // pas de processus orphelin qui garde le port et le verrou du
            // répertoire de données.
            .kill_on_drop(true);
        command
    }
}

/// Poignée sur le processus embarqué. Clonable : `main` la garde pour l'arrêt,
/// l'état partagé pour les redémarrages demandés depuis l'interface.
#[derive(Clone)]
pub struct EmbeddedVm {
    shared: Arc<Shared>,
}

struct Shared {
    /// Modifiable : un changement de rétention relance le processus avec la
    /// nouvelle valeur.
    config: std::sync::Mutex<EmbeddedConfig>,
    flavor: Flavor,
    http: reqwest::Client,
    /// Positionné par [`EmbeddedVm::stop`] : la sortie suivante est attendue,
    /// pas une panne, et le superviseur ne relance pas.
    stopping: AtomicBool,
    /// Positionné par [`EmbeddedVm::restart_with`] : la sortie suivante est
    /// voulue, le superviseur relance aussitôt, sans délai ni alarme.
    restarting: AtomicBool,
    /// Réveille le superviseur pendant son délai de relance : un arrêt demandé à
    /// ce moment n'a pas à attendre la fin du délai (jusqu'à trente secondes).
    stop_requested: Notify,
    /// Identifiant du processus courant, pour lui envoyer un signal. Remis à
    /// `None` dès que le processus est sorti : un pid périmé peut avoir été
    /// réattribué à un autre processus.
    pid: std::sync::Mutex<Option<u32>>,
    supervisor: Mutex<Option<JoinHandle<()>>>,
}

impl EmbeddedVm {
    /// Lance le processus et attend qu'il réponde. Échoue clairement si le
    /// binaire manque : c'est le cas d'une image reconstruite sans lui, ou d'un
    /// lancement hors conteneur sans `DUMBMONIT_VM_URL`.
    pub async fn start(config: EmbeddedConfig) -> Result<Self> {
        if !config.binary.is_file() {
            bail!(
                "{} binary not found at {}: set DUMBMONIT_VM_URL to use an external \
                 VictoriaMetrics, or DUMBMONIT_VM_BINARY / DUMBMONIT_VLOGS_BINARY to point at \
                 the binary",
                config.flavor.product(),
                config.binary.display()
            );
        }
        tokio::fs::create_dir_all(&config.data_path)
            .await
            .with_context(|| format!("creating {}", config.data_path.display()))?;

        let flavor = config.flavor;
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .context("HTTP client for the embedded process")?;
        let shared = Arc::new(Shared {
            config: std::sync::Mutex::new(config),
            flavor,
            http,
            stopping: AtomicBool::new(false),
            restarting: AtomicBool::new(false),
            stop_requested: Notify::new(),
            pid: std::sync::Mutex::new(None),
            supervisor: Mutex::new(None),
        });
        let mut child = shared.spawn_child()?;
        let snapshot = shared.config();
        info!(
            process = flavor.label(),
            binary = %snapshot.binary.display(),
            data = %snapshot.data_path.display(),
            listen = %snapshot.listen,
            retention = %snapshot.retention,
            "embedded process started"
        );

        shared.wait_ready(&mut child).await?;
        info!(process = flavor.label(), "embedded process ready");

        let supervisor = tokio::spawn(Shared::supervise(shared.clone(), child));
        *shared.supervisor.lock().await = Some(supervisor);
        Ok(Self { shared })
    }

    /// Rétention avec laquelle le processus tourne (ou va redémarrer).
    pub fn retention(&self) -> String {
        self.shared.config().retention
    }

    /// Le processus répond-il ?
    pub async fn healthy(&self) -> bool {
        self.shared.healthy().await
    }

    /// Redémarrage contrôlé avec une nouvelle rétention : arrêt propre
    /// (SIGTERM, le processus vide ses tampons), relance immédiate, et attente
    /// de `/health`. Les écritures échouées pendant ces quelques secondes sont
    /// gardées par le tampon d'écriture et renvoyées.
    pub async fn restart_with(&self, retention: &str) -> Result<()> {
        let old_pid = *self.shared.pid.lock().unwrap_or_else(|e| e.into_inner());
        self.shared.config.lock().unwrap_or_else(|e| e.into_inner()).retention =
            retention.to_string();
        info!(process = self.shared.flavor.label(), retention, "restarting with a new retention");
        self.shared.restarting.store(true, Ordering::SeqCst);
        if old_pid.is_some() {
            self.shared.signal(Signal::Term);
        } else {
            // Pas de processus en cours (relance en attente après une panne) :
            // le superviseur lancera de toute façon la nouvelle configuration.
            self.shared.stop_requested.notify_one();
        }
        let deadline = Instant::now() + RESTART_TIMEOUT;
        loop {
            let pid = *self.shared.pid.lock().unwrap_or_else(|e| e.into_inner());
            let replaced = pid.is_some() && pid != old_pid;
            if replaced && self.shared.healthy().await {
                info!(process = self.shared.flavor.label(), "restarted");
                return Ok(());
            }
            if Instant::now() >= deadline {
                bail!(
                    "{} did not come back within {:?} after the restart",
                    self.shared.flavor.product(),
                    RESTART_TIMEOUT
                );
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }

    /// Arrêt ordonné : SIGTERM, [`STOP_GRACE`] d'attente, puis SIGKILL.
    pub async fn stop(&self) {
        self.shared.stopping.store(true, Ordering::SeqCst);
        // `notify_one` garde un permis si le superviseur n'attend pas encore.
        self.shared.stop_requested.notify_one();
        let Some(supervisor) = self.shared.supervisor.lock().await.take() else { return };

        let label = self.shared.flavor.label();
        self.shared.signal(Signal::Term);
        info!(process = label, "stopping embedded process");
        if tokio::time::timeout(STOP_GRACE, supervisor).await.is_ok() {
            info!(process = label, "embedded process stopped");
            return;
        }
        warn!(process = label, grace = ?STOP_GRACE, "embedded process did not stop in time, killing it");
        self.shared.signal(Signal::Kill);
    }
}

#[derive(Clone, Copy)]
enum Signal {
    Term,
    Kill,
}

impl Shared {
    fn config(&self) -> EmbeddedConfig {
        self.config.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    async fn healthy(&self) -> bool {
        let url = self.config().health_url;
        matches!(self.http.get(url).send().await, Ok(response) if response.status().is_success())
    }

    fn spawn_child(&self) -> Result<Child> {
        let config = self.config();
        let mut child = config
            .command()
            .spawn()
            .with_context(|| format!("starting {}", config.binary.display()))?;
        *self.pid.lock().unwrap_or_else(|e| e.into_inner()) = child.id();
        let label = self.flavor.label();
        if let Some(stdout) = child.stdout.take() {
            tokio::spawn(forward_logs(stdout, label));
        }
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(forward_logs(stderr, label));
        }
        Ok(child)
    }

    /// Attend `/health`, en surveillant que le processus n'est pas mort entre-temps
    /// (port déjà pris, répertoire de données illisible…).
    async fn wait_ready(&self, child: &mut Child) -> Result<()> {
        let product = self.flavor.product();
        let deadline = Instant::now() + STARTUP_TIMEOUT;
        loop {
            if let Some(status) = child.try_wait().context("polling the embedded process")? {
                bail!("{product} exited during startup ({status}); see the lines above");
            }
            if self.healthy().await {
                return Ok(());
            }
            if Instant::now() >= deadline {
                bail!(
                    "{product} did not answer on {} within {:?}",
                    self.config().listen,
                    STARTUP_TIMEOUT
                );
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }

    /// Boucle de surveillance : relance le processus tant que l'arrêt n'a pas été
    /// demandé, avec un délai croissant entre deux pannes rapprochées.
    async fn supervise(shared: Arc<Self>, mut child: Child) {
        let label = shared.flavor.label();
        let mut delay = FIRST_RESTART_DELAY;
        let mut started = Instant::now();
        loop {
            let exit = child.wait().await;
            shared.clear_pid();
            if shared.stopping.load(Ordering::SeqCst) {
                return;
            }
            if shared.restarting.swap(false, Ordering::SeqCst) {
                // Redémarrage demandé (nouvelle rétention) : ni délai, ni alarme.
                child = match shared.spawn_child() {
                    Ok(child) => child,
                    Err(error) => {
                        error!(process = label, %error, "cannot restart the embedded process");
                        if !shared.pause(delay).await {
                            return;
                        }
                        // Le prochain tour retente, comme après une panne.
                        shared.restarting.store(true, Ordering::SeqCst);
                        continue;
                    }
                };
                started = Instant::now();
                delay = FIRST_RESTART_DELAY;
                continue;
            }
            if started.elapsed() >= STABLE_AFTER {
                delay = FIRST_RESTART_DELAY;
            }
            match exit {
                Ok(status) => {
                    error!(process = label, %status, ?delay, "embedded process exited, restarting")
                }
                Err(error) => {
                    error!(process = label, %error, ?delay, "cannot wait for the process, restarting")
                }
            }
            if !shared.pause(delay).await {
                return;
            }
            delay = (delay * 2).min(MAX_RESTART_DELAY);
            started = Instant::now();
            // Comptée avant la tentative : une relance qui échoue est justement
            // ce que la règle « redémarre en boucle » doit voir (`selfmon`).
            if shared.flavor == Flavor::Metrics {
                crate::stats::stats().vm_restarted();
            }
            // Un redémarrage demandé pendant le délai est servi par ce lancement,
            // qui lit déjà la nouvelle configuration.
            shared.restarting.store(false, Ordering::SeqCst);
            child = match shared.spawn_child() {
                Ok(child) => child,
                Err(error) => {
                    error!(process = label, %error, "cannot restart the embedded process");
                    // Sans processus à attendre, on temporise avant de réessayer.
                    if !shared.pause(delay).await {
                        return;
                    }
                    continue;
                }
            };
            // Arrêt demandé pendant le lancement : `stop` n'a peut-être pas vu le
            // nouveau pid, on lui envoie le signal nous-mêmes.
            if shared.stopping.load(Ordering::SeqCst) {
                shared.signal(Signal::Term);
            }
            info!(process = label, "embedded process restarted");
        }
    }

    /// Attend `delay`, ou moins si l'arrêt est demandé entre-temps. Renvoie
    /// `false` quand le superviseur doit s'arrêter.
    async fn pause(&self, delay: Duration) -> bool {
        tokio::select! {
            _ = tokio::time::sleep(delay) => {}
            _ = self.stop_requested.notified() => {}
        }
        !self.stopping.load(Ordering::SeqCst)
    }

    fn clear_pid(&self) {
        *self.pid.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    fn signal(&self, signal: Signal) {
        let Some(pid) = *self.pid.lock().unwrap_or_else(|e| e.into_inner()) else { return };
        #[cfg(unix)]
        {
            let signal = match signal {
                Signal::Term => libc::SIGTERM,
                Signal::Kill => libc::SIGKILL,
            };
            // SAFETY : `kill` n'a pas de précondition mémoire ; un pid périmé se
            // solde par ESRCH, qu'on ignore.
            let _ = unsafe { libc::kill(pid as libc::pid_t, signal) };
        }
        #[cfg(not(unix))]
        {
            let _ = (pid, signal);
        }
    }
}

/// Recopie le journal de VictoriaMetrics dans le nôtre, niveau par niveau.
///
/// Son format est `horodatage\tniveau\tfichier:ligne\tmessage` ; sans horodatage
/// (option passée au lancement), il reste `niveau\tfichier:ligne\tmessage`.
async fn forward_logs(stream: impl AsyncRead + Unpin, label: &'static str) {
    let mut lines = BufReader::new(stream).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let (level, message) = split_log_line(&line);
        match level {
            // Une requête annulée parce que le visiteur a quitté la page n'est pas un incident.
            "warn" if message.contains("context canceled") => debug!("{label}: {message}"),
            "warn" => warn!("{label}: {message}"),
            "error" | "fatal" | "panic" => error!("{label}: {message}"),
            _ => info!("{label}: {message}"),
        }
    }
}

fn split_log_line(line: &str) -> (&'static str, String) {
    let fields: Vec<&str> = line.split('\t').collect();
    // Avec horodatage, le niveau est en deuxième position ; sans, en première.
    let position = fields.iter().position(|field| level_of(field).is_some()).filter(|p| *p <= 1);
    let Some(position) = position else { return ("info", line.trim().to_string()) };
    let level = level_of(fields[position]).unwrap_or("info");
    // Le champ qui suit le niveau est l'emplacement dans les sources : sans
    // intérêt ici. Le message est tout ce qui vient après.
    let message = match fields.get(position + 2..) {
        Some(rest) if !rest.is_empty() => rest.join("\t"),
        _ => fields.get(position + 1..).map(|rest| rest.join("\t")).unwrap_or_default(),
    };
    (level, message.trim().to_string())
}

fn level_of(word: &str) -> Option<&'static str> {
    match word {
        "info" => Some("info"),
        "warn" => Some("warn"),
        "error" => Some("error"),
        "fatal" => Some("fatal"),
        "panic" => Some("panic"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn victoria_log_lines_keep_their_level_and_lose_the_source_location() {
        assert_eq!(
            split_log_line(
                "2026-09-16T10:00:00.000Z\tinfo\tVictoriaMetrics/app/main.go:12\tstarted"
            ),
            ("info", "started".to_string())
        );
        assert_eq!(
            split_log_line("warn\tlib/storage/storage.go:9\tslow disk\tdetails"),
            ("warn", "slow disk\tdetails".to_string())
        );
        assert_eq!(split_log_line("free text"), ("info", "free text".to_string()));
    }

    fn args(flavor: Flavor) -> Vec<String> {
        let config = EmbeddedConfig {
            flavor,
            binary: "/bin/true".into(),
            data_path: "/data/x".into(),
            listen: "127.0.0.1:1".into(),
            retention: "30d".into(),
            memory: "64MB".into(),
            admin_key: "k".into(),
            health_url: String::new(),
        };
        config
            .command()
            .as_std()
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }

    /// VictoriaLogs ne connaît pas les options d'administration de
    /// VictoriaMetrics : les lui passer l'empêcherait de démarrer.
    #[test]
    fn each_flavor_gets_its_own_options() {
        let metrics = args(Flavor::Metrics);
        assert!(metrics.contains(&"-retentionPeriod=30d".to_string()), "{metrics:?}");
        assert!(metrics.contains(&"-dedup.minScrapeInterval=10s".to_string()));
        assert!(metrics.iter().any(|arg| arg.starts_with("-deleteAuthKey=")));

        let long = args(Flavor::LongTerm);
        assert!(long.contains(&"-dedup.minScrapeInterval=1m".to_string()), "{long:?}");

        let logs = args(Flavor::Logs);
        assert!(logs.contains(&"-storageDataPath=/data/x".to_string()), "{logs:?}");
        assert!(logs.contains(&"-retentionPeriod=30d".to_string()));
        assert!(!logs.iter().any(|arg| arg.contains("AuthKey") || arg.starts_with("-search.")));
        assert!(!logs.iter().any(|arg| arg.starts_with("-dedup")));
    }

    /// Un arrêt demandé pendant le délai de relance doit rendre la main tout de
    /// suite, et le pid du processus sorti ne doit plus être signalé.
    #[cfg(unix)]
    #[tokio::test]
    async fn stop_during_the_restart_delay_returns_at_once() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let binary = dir.path().join("fake-vm");
        std::fs::write(&binary, "#!/bin/sh\nexit 1\n").unwrap();
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).unwrap();

        let shared = Arc::new(Shared {
            config: std::sync::Mutex::new(EmbeddedConfig {
                flavor: Flavor::Metrics,
                binary,
                data_path: dir.path().join("data"),
                listen: "127.0.0.1:0".into(),
                retention: "1d".into(),
                memory: "1MB".into(),
                admin_key: "test".into(),
                health_url: "http://127.0.0.1:9/health".into(),
            }),
            flavor: Flavor::Metrics,
            http: reqwest::Client::new(),
            stopping: AtomicBool::new(false),
            restarting: AtomicBool::new(false),
            stop_requested: Notify::new(),
            pid: std::sync::Mutex::new(None),
            supervisor: Mutex::new(None),
        });
        let child = shared.spawn_child().unwrap();
        let supervisor = tokio::spawn(Shared::supervise(shared.clone(), child));
        *shared.supervisor.lock().await = Some(supervisor);

        // Le faux binaire sort aussitôt : le pid doit être effacé, signe que le
        // superviseur est entré dans son délai de relance.
        tokio::time::timeout(Duration::from_secs(5), async {
            while shared.pid.lock().unwrap().is_some() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("le pid du processus sorti n'a pas été effacé");

        let vm = EmbeddedVm { shared };
        let started = Instant::now();
        vm.stop().await;
        assert!(
            started.elapsed() < Duration::from_millis(500),
            "stop() a attendu la fin du délai de relance ({:?})",
            started.elapsed()
        );
    }
}
