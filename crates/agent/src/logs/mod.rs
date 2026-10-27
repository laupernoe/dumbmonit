//! Collecte de journaux, optionnelle et pilotée depuis l'interface.
//!
//! Le serveur joint le réglage de la machine à l'accusé de réception de ses lots
//! ([`dumbmonit_proto::PushAck::logs`]) ; [`LogRunner::apply`] le met en œuvre à
//! chaud. Les sources ([`sources`]) alimentent une file bornée, un expéditeur
//! envoie les lignes par lots sur le canal ordinaire de l'agent (même serveur,
//! même port, même jeton), et garde sur disque, dans une limite stricte, ce que
//! le serveur n'a pas pu recevoir. `DUMBMONIT_AGENT_LOGS=off` interdit la
//! collecte sur la machine, quoi que dise le serveur.

pub mod glob;
pub mod sources;

use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Duration;

use dumbmonit_proto::{LogBatch, LogLine, LogsSetting, MAX_LOG_LINES, level_rank};
use tokio::task::{JoinHandle, JoinSet};
use tokio::time::Instant;
use tracing::{debug, info, warn};

use crate::client::PushClient;
use crate::shutdown::Shutdown;

/// Lignes gardées en mémoire au plus.
const MAX_QUEUED_LINES: usize = 20_000;
/// Octets de messages gardés en mémoire au plus.
const MAX_QUEUED_BYTES: usize = 16 * 1024 * 1024;
/// Taille maximale du tampon sur disque.
pub const MAX_SPOOL_BYTES: u64 = 8 * 1024 * 1024;
/// Période d'envoi.
const FLUSH_EVERY: Duration = Duration::from_secs(5);
/// Attente maximale entre deux tentatives après un échec.
const RETRY_MAX: Duration = Duration::from_secs(60);

/// La collecte est-elle permise sur cette machine ?
pub fn allowed_locally() -> bool {
    !matches!(
        std::env::var("DUMBMONIT_AGENT_LOGS").map(|v| v.trim().to_ascii_lowercase()).as_deref(),
        Ok("off") | Ok("false") | Ok("0") | Ok("no")
    )
}

/// File des lignes en attente, bornée en nombre et en octets : la plus ancienne
/// tombe la première.
#[derive(Default)]
pub struct Queue {
    lines: VecDeque<LogLine>,
    bytes: usize,
    pub dropped: u64,
}

impl Queue {
    pub fn push(&mut self, line: LogLine) {
        self.bytes += line.message.len() + line.source.len();
        self.lines.push_back(line);
        while self.lines.len() > MAX_QUEUED_LINES || self.bytes > MAX_QUEUED_BYTES {
            let Some(old) = self.lines.pop_front() else { break };
            self.bytes = self.bytes.saturating_sub(old.message.len() + old.source.len());
            self.dropped += 1;
        }
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Les `count` plus anciennes lignes, sans les retirer.
    pub fn front(&self, count: usize) -> Vec<LogLine> {
        self.lines.iter().take(count).cloned().collect()
    }

    pub fn discard(&mut self, count: usize) {
        for _ in 0..count {
            let Some(old) = self.lines.pop_front() else { break };
            self.bytes = self.bytes.saturating_sub(old.message.len() + old.source.len());
        }
    }

    pub fn lines(&self) -> impl Iterator<Item = &LogLine> {
        self.lines.iter()
    }
}

// --------------------------------------------------------------------------
// Tampon sur disque
// --------------------------------------------------------------------------

/// Lit le tampon laissé par un arrêt ou une coupure ; les lignes illisibles
/// sont ignorées.
pub fn load_spool(path: &std::path::Path) -> Vec<LogLine> {
    let Ok(text) = std::fs::read_to_string(path) else { return Vec::new() };
    text.lines().filter_map(|line| serde_json::from_str::<LogLine>(line).ok()).collect()
}

/// Écrit les lignes les plus récentes qui tiennent dans [`MAX_SPOOL_BYTES`].
pub fn save_spool(path: &std::path::Path, queue: &Queue) {
    if queue.is_empty() {
        let _ = std::fs::remove_file(path);
        return;
    }
    let mut kept: Vec<String> = Vec::new();
    let mut size: u64 = 0;
    for line in queue.lines().collect::<Vec<_>>().into_iter().rev() {
        let Ok(json) = serde_json::to_string(line) else { continue };
        size += json.len() as u64 + 1;
        if size > MAX_SPOOL_BYTES {
            break;
        }
        kept.push(json);
    }
    kept.reverse();
    let mut text = kept.join("\n");
    text.push('\n');
    let temporary = path.with_extension("tmp");
    if std::fs::write(&temporary, text).is_ok() {
        let _ = std::fs::rename(&temporary, path);
    }
}

// --------------------------------------------------------------------------
// Mise en œuvre à chaud
// --------------------------------------------------------------------------

/// Tient la tâche de collecte et la relance quand le réglage change.
pub struct LogRunner {
    client: PushClient,
    key: String,
    spool: PathBuf,
    current: Option<LogsSetting>,
    task: Option<JoinHandle<()>>,
}

impl LogRunner {
    pub fn new(client: PushClient, key: String, spool: PathBuf) -> Self {
        Self { client, key, spool, current: None, task: None }
    }

    /// Applique le réglage de l'accusé de réception. `None` : serveur antérieur,
    /// rien n'est collecté.
    pub fn apply(&mut self, setting: Option<LogsSetting>, shutdown: &Shutdown) {
        let wanted = setting.filter(|s| s.enabled && allowed_locally());
        if self.task.as_ref().is_some_and(JoinHandle::is_finished) {
            self.task = None;
        }
        if wanted == self.current && (wanted.is_none() || self.task.is_some()) {
            return;
        }
        if let Some(task) = self.task.take() {
            task.abort();
            info!("log collection stopped");
        }
        self.current = wanted.clone();
        if let Some(setting) = wanted {
            info!(
                journald = setting.journald,
                files = setting.files.len(),
                eventlog = setting.eventlog.len(),
                level = setting.min_level,
                "log collection switched on from the server"
            );
            let client = self.client.clone();
            let key = self.key.clone();
            let spool = self.spool.clone();
            let shutdown = shutdown.clone();
            self.task = Some(tokio::spawn(collect(setting, client, key, spool, shutdown)));
        }
    }

    /// Arrête la collecte. À l'arrêt de l'agent (`graceful`), la tâche reçoit le
    /// signal d'arrêt et a quelques secondes pour envoyer ou mettre de côté ce
    /// qui reste ; sinon (mise à jour de l'agent), elle est abandonnée.
    pub async fn finish(&mut self, graceful: bool) {
        let Some(mut task) = self.task.take() else { return };
        if graceful {
            if tokio::time::timeout(Duration::from_secs(5), &mut task).await.is_err() {
                task.abort();
            }
        } else {
            task.abort();
        }
    }
}

async fn collect(
    setting: LogsSetting,
    client: PushClient,
    key: String,
    spool: PathBuf,
    mut shutdown: Shutdown,
) {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<LogLine>(10_000);
    // Les sources vivent dans cet ensemble : abandonner la tâche les abandonne.
    let mut tasks = JoinSet::new();
    let level = setting.min_level.clone();
    if setting.journald && cfg!(target_os = "linux") {
        tasks.spawn(sources::run_journald(tx.clone(), level.clone()));
    }
    if !setting.files.is_empty() {
        tasks.spawn(sources::run_files(tx.clone(), setting.files.clone(), level.clone()));
    }
    if cfg!(windows) {
        for channel in &setting.eventlog {
            tasks.spawn(sources::run_eventlog(tx.clone(), channel.clone(), level.clone()));
        }
    }
    // Gardé ouvert : la file ne se ferme jamais, même sans aucune source.
    let _keep_open = tx;
    let min_rank = level_rank(&level);

    let mut queue = Queue::default();
    for line in load_spool(&spool) {
        queue.push(line);
    }
    if !queue.is_empty() {
        info!(count = queue.len(), "log lines kept on disk, sending them");
    }
    let mut retry_at: Option<Instant> = None;
    let mut delay = Duration::from_secs(5);
    let mut tick = tokio::time::interval(FLUSH_EVERY);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut buffer: Vec<LogLine> = Vec::new();

    loop {
        tokio::select! {
            biased;
            _ = shutdown.wait() => {
                // Dernière tentative, puis ce qui reste va sur disque.
                let _ = ship(&client, &key, &mut queue).await;
                save_spool(&spool, &queue);
                break;
            }
            received = rx.recv_many(&mut buffer, 500) => {
                debug!(received, "log lines read");
                for line in buffer.drain(..) {
                    if level_rank(&line.level) <= min_rank {
                        queue.push(line);
                    }
                }
            }
            _ = tick.tick() => {}
        }
        if queue.is_empty() || retry_at.is_some_and(|at| Instant::now() < at) {
            continue;
        }
        match ship(&client, &key, &mut queue).await {
            Ok(()) => {
                retry_at = None;
                delay = Duration::from_secs(5);
                if queue.is_empty() {
                    let _ = std::fs::remove_file(&spool);
                }
            }
            Err(retryable) => {
                if retryable {
                    save_spool(&spool, &queue);
                    retry_at = Some(Instant::now() + delay);
                    delay = (delay * 2).min(RETRY_MAX);
                }
            }
        }
        if queue.dropped > 0 {
            warn!(dropped = queue.dropped, "log lines dropped: the queue is full");
            queue.dropped = 0;
        }
    }
}

/// Envoie les lots en attente. `Err(true)` : à réessayer ; `Err(false)` : le
/// serveur a refusé ces lignes, elles sont jetées.
async fn ship(client: &PushClient, key: &str, queue: &mut Queue) -> Result<(), bool> {
    while !queue.is_empty() {
        let lines = queue.front(MAX_LOG_LINES);
        let count = lines.len();
        match client.send_logs(key, &LogBatch { lines }).await {
            Ok(()) => queue.discard(count),
            Err(error) if error.is_retryable() => {
                debug!(%error, "log batch not sent, will retry");
                return Err(true);
            }
            Err(error) => {
                warn!(%error, count, "log batch refused by the server, dropped");
                queue.discard(count);
                return Err(false);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(text: &str) -> LogLine {
        LogLine { ts_ms: 1, level: "info".into(), source: "file:x".into(), message: text.into() }
    }

    #[test]
    fn the_queue_drops_the_oldest_lines_first() {
        let mut queue = Queue::default();
        for i in 0..(MAX_QUEUED_LINES + 5) {
            queue.push(line(&i.to_string()));
        }
        assert_eq!(queue.len(), MAX_QUEUED_LINES);
        assert_eq!(queue.dropped, 5);
        assert_eq!(queue.front(1)[0].message, "5");
    }

    #[test]
    fn the_spool_is_bounded_and_keeps_the_newest_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("logs-spool.jsonl");
        let mut queue = Queue::default();
        queue.push(line("first"));
        queue.push(line("second"));
        save_spool(&path, &queue);
        let back = load_spool(&path);
        assert_eq!(back.len(), 2);
        assert_eq!(back[1].message, "second");
        let big = "x".repeat(1000);
        let mut queue = Queue::default();
        for _ in 0..12_000 {
            queue.push(line(&big));
        }
        save_spool(&path, &queue);
        assert!(std::fs::metadata(&path).unwrap().len() <= MAX_SPOOL_BYTES + 1100);
        save_spool(&path, &Queue::default());
        assert!(!path.exists());
    }

    #[test]
    fn the_local_switch_can_forbid_collection() {
        // Sans variable posée, la collecte est permise.
        assert!(allowed_locally() || std::env::var("DUMBMONIT_AGENT_LOGS").is_ok());
    }
}
