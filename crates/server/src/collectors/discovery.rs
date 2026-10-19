//! Balayages de découverte réseau, gardés en mémoire.
//!
//! Un balayage est une action d'administrateur : on donne un réseau, le moteur
//! partagé ([`dumbmonit_collectors::netscan`]) ouvre une à une les adresses et
//! reconnaît les produits sans jamais s'authentifier. Le serveur exécute
//! lui-même les balayages de son propre réseau ; ceux d'un site distant sont
//! confiés à l'agent relais de ce site, qui rapporte au fil de l'eau.
//!
//! Tout vit en mémoire, et les vingt derniers balayages seulement : un
//! redémarrage les oublie, c'est voulu. La community SNMP, elle, ne quitte
//! jamais la mémoire — ni journal, ni réponse d'API.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;

use dumbmonit_proto::{
    AgentCommand, Confidence, DiscoveredHost, DiscoveredService, DiscoveryReport, DiscoveryScanJob,
    ScanPhase, ScanProgress, TargetId, Transport,
};
use serde::Serialize;
use tokio::time::Instant;
use tracing::{info, warn};

/// Nombre de balayages conservés : les plus anciens tombent.
const MAX_JOBS: usize = 20;

/// Délai sans nouvelle d'un agent relais avant d'abandonner un balayage confié.
const AGENT_GRACE: Duration = Duration::from_secs(300);

/// Horodatage UTC sans suffixe, comme le reste de l'API (`parseServerDate`
/// ajoute le `Z` côté interface).
fn now() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

/// État d'un balayage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Running,
    Done,
    Failed,
    Cancelled,
}

impl JobStatus {
    fn is_final(self) -> bool {
        !matches!(self, Self::Running)
    }
}

/// Résumé d'un balayage, pour la liste.
#[derive(Debug, Clone, Serialize)]
pub struct JobSummary {
    pub id: String,
    pub cidr: String,
    pub agent_id: Option<TargetId>,
    pub agent_name: Option<String>,
    pub status: JobStatus,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub hosts_found: usize,
}

/// Un balayage complet, hôtes compris.
#[derive(Debug, Clone, Serialize)]
pub struct Job {
    #[serde(flatten)]
    pub summary: JobSummary,
    pub progress: ScanProgress,
    pub error: Option<String>,
    pub hosts: Vec<DiscoveredHost>,
}

struct Record {
    job: Job,
    cancel: Option<dumbmonit_collectors::netscan::Cancel>,
    /// Identifiant de la commande confiée à l'agent relais (balayage distant).
    command: Option<AgentCommand>,
    /// Échéance au-delà de laquelle un agent silencieux fait échouer le balayage.
    agent_deadline: Option<Instant>,
}

#[derive(Default)]
struct Inner {
    jobs: VecDeque<Record>,
    next_command_id: i64,
}

/// Registre des balayages, partagé par l'API et le planificateur.
#[derive(Default)]
pub struct Discovery {
    inner: Mutex<Inner>,
}

/// Pourquoi un balayage n'a pas pu être créé.
#[derive(Debug, PartialEq, Eq)]
pub enum CreateError {
    /// Un balayage tourne déjà pour cette cible (serveur, ou cet agent).
    Busy,
}

impl Discovery {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|poison| poison.into_inner())
    }

    /// Vrai si un balayage tourne déjà pour cette cible (serveur si `None`).
    fn busy(&self, agent_id: Option<TargetId>) -> bool {
        self.lock().jobs.iter().any(|r| {
            r.job.summary.agent_id == agent_id && r.job.summary.status == JobStatus::Running
        })
    }

    fn insert(&self, record: Record) {
        let mut inner = self.lock();
        inner.jobs.push_front(record);
        // On ne retire que des balayages terminés : un balayage en cours garde sa
        // place, même au-delà de la limite.
        while inner.jobs.len() > MAX_JOBS {
            let Some(pos) = inner.jobs.iter().rposition(|r| r.job.summary.status.is_final()) else {
                break;
            };
            inner.jobs.remove(pos);
        }
    }

    /// Prépare un enregistrement de balayage et l'insère. Rend l'id.
    fn create(
        &self,
        agent_id: Option<TargetId>,
        agent_name: Option<String>,
        cidr: String,
        total: usize,
    ) -> Result<String, CreateError> {
        if self.busy(agent_id) {
            return Err(CreateError::Busy);
        }
        let id = hex::encode(rand::random::<[u8; 16]>());
        let now = now();
        self.insert(Record {
            job: Job {
                summary: JobSummary {
                    id: id.clone(),
                    cidr,
                    agent_id,
                    agent_name,
                    status: JobStatus::Running,
                    started_at: now,
                    finished_at: None,
                    hosts_found: 0,
                },
                progress: ScanProgress { phase: ScanPhase::Sweep, probed: 0, total },
                error: None,
                hosts: Vec::new(),
            },
            cancel: None,
            command: None,
            agent_deadline: None,
        });
        Ok(id)
    }

    /// Enregistre le jeton d'annulation d'un balayage serveur.
    pub fn set_cancel(&self, id: &str, cancel: dumbmonit_collectors::netscan::Cancel) {
        if let Some(mut record) = self.find_mut(id) {
            record.get().cancel = Some(cancel);
        }
    }

    fn find_mut<'a>(&'a self, id: &str) -> Option<RecordGuard<'a>> {
        let inner = self.lock();
        inner
            .jobs
            .iter()
            .position(|r| r.job.summary.id == id)
            .map(|index| RecordGuard { inner, index })
    }

    /// Démarre un balayage exécuté par le serveur lui-même.
    pub fn start_local(&self, cidr: String, total: usize) -> Result<String, CreateError> {
        self.create(None, None, cidr, total)
    }

    /// Démarre un balayage confié à un agent relais. Rend l'id et la commande à
    /// remettre à l'agent au prochain appel de [`Discovery::take_commands`].
    pub fn start_agent(
        &self,
        agent_id: TargetId,
        agent_name: String,
        cidr: String,
        job: DiscoveryScanJob,
    ) -> Result<String, CreateError> {
        let id = self.create(Some(agent_id), Some(agent_name), cidr, 0)?;
        let mut inner = self.lock();
        inner.next_command_id += 1;
        let command_id = inner.next_command_id;
        let command = job.into_command(command_id, chrono::Utc::now().timestamp_millis());
        if let Some(record) = inner.jobs.iter_mut().find(|r| r.job.summary.id == id) {
            record.command = Some(command);
        }
        Ok(id)
    }

    /// Remet à l'agent les commandes de balayage qui l'attendent, et arme leur
    /// échéance de silence.
    pub fn take_commands(&self, agent_id: TargetId) -> Vec<AgentCommand> {
        let mut inner = self.lock();
        let deadline = Instant::now() + AGENT_GRACE;
        let mut commands = Vec::new();
        for record in inner.jobs.iter_mut() {
            if record.job.summary.agent_id == Some(agent_id)
                && record.job.summary.status == JobStatus::Running
                && record.agent_deadline.is_none()
                && let Some(command) = record.command.clone()
            {
                record.agent_deadline = Some(deadline);
                commands.push(command);
            }
        }
        commands
    }

    /// Range un compte rendu d'agent. Les hôtes sont déjà annotés par l'appelant.
    /// Rend `false` si le balayage n'existe plus ou a été annulé (l'agent doit
    /// alors s'arrêter).
    pub fn apply_report(
        &self,
        agent_id: TargetId,
        command_id: i64,
        report: DiscoveryReport,
    ) -> bool {
        let Some(mut record) = self.find_agent_command(agent_id, command_id) else {
            return false;
        };
        let job = &mut record.get().job;
        if job.summary.status != JobStatus::Running {
            return false;
        }
        record.get().agent_deadline = Some(Instant::now() + AGENT_GRACE);
        let job = &mut record.get().job;
        job.progress = report.progress;
        job.hosts.extend(report.hosts);
        job.summary.hosts_found = job.hosts.len();
        if report.finished || report.error.is_some() {
            if let Some(error) = report.error {
                job.summary.status = JobStatus::Failed;
                job.error = Some(error);
            } else {
                job.summary.status = JobStatus::Done;
                job.progress.phase = ScanPhase::Done;
            }
            job.summary.finished_at = Some(now());
        }
        true
    }

    /// Un agent trop ancien a reçu la commande mais l'a rendue illisible : on
    /// fait échouer le balayage avec un message clair. Rend `true` si l'id
    /// correspondait bien à un balayage confié à cet agent.
    pub fn fail_too_old(&self, agent_id: TargetId, command_id: i64) -> bool {
        let Some(mut record) = self.find_agent_command(agent_id, command_id) else {
            return false;
        };
        let job = &mut record.get().job;
        if job.summary.status != JobStatus::Running {
            return false;
        }
        job.summary.status = JobStatus::Failed;
        job.error = Some(
            "the relay agent is too old to run network discovery — update it to the latest version"
                .to_string(),
        );
        job.summary.finished_at = Some(now());
        true
    }

    fn find_agent_command(&self, agent_id: TargetId, command_id: i64) -> Option<RecordGuard<'_>> {
        let inner = self.lock();
        inner
            .jobs
            .iter()
            .position(|r| {
                r.job.summary.agent_id == Some(agent_id)
                    && r.command.as_ref().map(|c| c.id) == Some(command_id)
            })
            .map(|index| RecordGuard { inner, index })
    }

    /// Met à jour un balayage serveur : progression et nouvel hôte annoté.
    pub fn update_local(&self, id: &str, progress: ScanProgress, host: Option<DiscoveredHost>) {
        if let Some(mut record) = self.find_mut(id) {
            let job = &mut record.get().job;
            if job.summary.status != JobStatus::Running {
                return;
            }
            job.progress = progress;
            if let Some(host) = host {
                job.hosts.push(host);
                job.summary.hosts_found = job.hosts.len();
            }
        }
    }

    /// Clôt un balayage serveur.
    pub fn finish_local(&self, id: &str, error: Option<String>) {
        if let Some(mut record) = self.find_mut(id) {
            let job = &mut record.get().job;
            if job.summary.status != JobStatus::Running {
                return;
            }
            job.summary.finished_at = Some(now());
            match error {
                Some(error) => {
                    job.summary.status = JobStatus::Failed;
                    job.error = Some(error);
                }
                None => {
                    job.summary.status = JobStatus::Done;
                    job.progress.phase = ScanPhase::Done;
                }
            }
        }
    }

    /// Annule un balayage. Rend `false` s'il n'existe pas.
    pub fn cancel(&self, id: &str) -> bool {
        let Some(mut record) = self.find_mut(id) else { return false };
        if let Some(cancel) = record.get().cancel.clone() {
            cancel.cancel();
        }
        let job = &mut record.get().job;
        if job.summary.status == JobStatus::Running {
            job.summary.status = JobStatus::Cancelled;
            job.summary.finished_at = Some(now());
        }
        true
    }

    /// Les balayages, du plus récent au plus ancien.
    pub fn list(&self) -> Vec<JobSummary> {
        self.lock().jobs.iter().map(|r| r.job.summary.clone()).collect()
    }

    /// Un balayage complet.
    pub fn get(&self, id: &str) -> Option<Job> {
        self.lock().jobs.iter().find(|r| r.job.summary.id == id).map(|r| r.job.clone())
    }

    /// Fait échouer les balayages confiés à un agent resté silencieux trop
    /// longtemps. Appelé périodiquement par le planificateur.
    pub fn expire_silent(&self) {
        let instant = Instant::now();
        let mut inner = self.lock();
        for record in inner.jobs.iter_mut() {
            if record.job.summary.status == JobStatus::Running
                && record.agent_deadline.is_some_and(|deadline| deadline <= instant)
            {
                record.job.summary.status = JobStatus::Failed;
                record.job.error = Some(
                    "the relay agent stopped reporting — is it still running with relay enabled?"
                        .to_string(),
                );
                record.job.summary.finished_at = Some(now());
                warn!(id = %record.job.summary.id, "discovery job abandoned: relay agent silent");
            }
        }
    }
}

/// Garde qui tient le verrou le temps d'une modification d'un enregistrement.
struct RecordGuard<'a> {
    inner: std::sync::MutexGuard<'a, Inner>,
    index: usize,
}

impl RecordGuard<'_> {
    fn get(&mut self) -> &mut Record {
        &mut self.inner.jobs[self.index]
    }
}

/// Annote les services d'un hôte : la cible existante qui pointe déjà dessus, et
/// le nom à pré-remplir dans le formulaire. Jamais de secret.
pub fn annotate_host(host: &mut DiscoveredHost, targets: &[(TargetId, String)]) {
    let suggested_name = host.hostname.clone();
    for service in &mut host.services {
        annotate_service(service, &host.ip, targets);
        if let Some(prefill) = service.prefill.as_mut().and_then(|v| v.as_object_mut()) {
            // Nom suggéré : le DNS inverse s'il existe, sinon le produit.
            let name = suggested_name.clone().unwrap_or_else(|| service.product.clone());
            prefill.entry("name".to_string()).or_insert(serde_json::Value::String(name));
        }
    }
}

fn annotate_service(service: &mut DiscoveredService, ip: &str, targets: &[(TargetId, String)]) {
    let port = service.port;
    for (id, address) in targets {
        if address_points_at(address, ip, port) {
            service.monitored_target_id = Some(id.to_string());
            return;
        }
    }
}

/// Vrai si l'adresse d'une cible existante vise cet hôte (et ce port, s'il y en a
/// un d'explicite). Comparaison textuelle tolérante : IP nue, `hôte:port`, URL.
fn address_points_at(address: &str, ip: &str, port: u16) -> bool {
    let rest = address.trim();
    let rest = rest.split_once("://").map_or(rest, |(_, after)| after);
    let rest = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    let rest = rest.rsplit('@').next().unwrap_or(rest);
    let (host, host_port) = match rest.rsplit_once(':') {
        Some((h, p)) if p.chars().all(|c| c.is_ascii_digit()) => (h, p.parse::<u16>().ok()),
        _ => (rest, None),
    };
    if host != ip {
        return false;
    }
    // Un port explicite doit correspondre ; sans port, l'hôte suffit.
    host_port.is_none_or(|existing| existing == port)
}

/// Résumé d'un service pour un journal : sans rien de sensible.
pub fn describe(service: &DiscoveredService) -> String {
    let confidence = match service.confidence {
        Confidence::High => "high",
        Confidence::Medium => "medium",
        Confidence::Low => "low",
    };
    let proto = match service.protocol {
        Transport::Tcp => "tcp",
        Transport::Udp => "udp",
    };
    format!("{}/{proto} {} ({confidence})", service.port, service.product)
}

/// Bruit de journal au démarrage d'un balayage, sans la community.
pub fn log_started(cidr: &str, agent: Option<&str>) {
    match agent {
        Some(name) => info!(%cidr, agent = name, "discovery job queued for relay agent"),
        None => info!(%cidr, "discovery job started on the server"),
    }
}

#[cfg(test)]
mod tests {
    use dumbmonit_proto::ScanDepth;

    use super::*;

    #[test]
    fn address_matching_handles_ip_port_and_url() {
        assert!(address_points_at("192.168.1.20", "192.168.1.20", 8006));
        assert!(address_points_at("192.168.1.20:8006", "192.168.1.20", 8006));
        assert!(!address_points_at("192.168.1.20:22", "192.168.1.20", 8006));
        assert!(address_points_at("https://192.168.1.20:8006/", "192.168.1.20", 8006));
        assert!(address_points_at("http://user@192.168.1.20/x", "192.168.1.20", 80));
        assert!(!address_points_at("192.168.1.21", "192.168.1.20", 80));
    }

    #[test]
    fn jobs_keep_the_newest_and_drop_finished_over_the_cap() {
        let discovery = Discovery::new();
        // Un balayage serveur en cours, puis beaucoup de balayages terminés.
        let running = discovery.start_local("10.0.0.0/24".into(), 254).unwrap();
        for i in 0..MAX_JOBS + 5 {
            let id = discovery
                .start_agent(
                    i as i64 + 1,
                    format!("agent-{i}"),
                    "10.0.1.0/24".into(),
                    DiscoveryScanJob {
                        cidr: "10.0.1.0/24".into(),
                        snmp_community: None,
                        depth: ScanDepth::Standard,
                    },
                )
                .unwrap();
            discovery.finish_local(&id, None);
        }
        assert!(discovery.list().len() <= MAX_JOBS);
        // Le balayage en cours n'a pas été évincé.
        assert!(discovery.get(&running).is_some());
    }

    #[test]
    fn a_second_scan_on_the_same_target_is_refused() {
        let discovery = Discovery::new();
        discovery.start_local("10.0.0.0/24".into(), 254).unwrap();
        assert_eq!(discovery.start_local("10.0.0.0/24".into(), 254).err(), Some(CreateError::Busy));
        // Un autre agent n'est pas concerné.
        discovery
            .start_agent(
                7,
                "edge".into(),
                "10.9.0.0/24".into(),
                DiscoveryScanJob {
                    cidr: "10.9.0.0/24".into(),
                    snmp_community: None,
                    depth: ScanDepth::Standard,
                },
            )
            .unwrap();
    }

    #[test]
    fn an_old_agent_makes_the_job_fail_with_a_clear_message() {
        let discovery = Discovery::new();
        let id = discovery
            .start_agent(
                7,
                "edge".into(),
                "10.9.0.0/24".into(),
                DiscoveryScanJob {
                    cidr: "10.9.0.0/24".into(),
                    snmp_community: None,
                    depth: ScanDepth::Standard,
                },
            )
            .unwrap();
        let commands = discovery.take_commands(7);
        assert_eq!(commands.len(), 1);
        assert!(discovery.fail_too_old(7, commands[0].id));
        let job = discovery.get(&id).unwrap();
        assert_eq!(job.summary.status, JobStatus::Failed);
        assert!(job.error.unwrap().contains("too old"));
    }
}
