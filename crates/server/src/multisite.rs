//! Sondes depuis plusieurs emplacements, avec confirmation avant d'alerter.
//!
//! Un moniteur de disponibilité peut être sondé à la fois par le serveur et par
//! un ou plusieurs agents relais, chacun depuis son réseau. Une « ronde » lance
//! la sonde partout en même temps ; quand chaque emplacement a répondu (ou que
//! l'échéance est passée), une décision de quorum dit ce que le moniteur est :
//!
//! - **en panne** si au moins N des M emplacements comptés échouent (N réglable,
//!   la majorité par défaut, ramené à M quand des emplacements sont exclus) ;
//! - **dégradé** si une minorité échoue : visible, sans alerte `service_down` ;
//! - **disponible** sinon.
//!
//! Un emplacement muet (relais qui n'est pas venu chercher la sonde ou n'a pas
//! répondu à temps), un agent sans mode relais, un agent saturé ou qui ne sait
//! pas exécuter la sonde (agent trop ancien, réglage refusé) est **exclu** du
//! quorum : il ne compte ni pour ni contre, et l'interface le signale.
//!
//! À la première dégradation (passage de « disponible » à « dégradé » ou « en
//! panne »), la décision n'est pas publiée : une ronde de confirmation repart
//! aussitôt de tous les emplacements, sans attendre l'intervalle, et c'est sa
//! décision qui est publiée. Un raté isolé d'un réseau ne sonne donc jamais.
//!
//! # Séries écrites
//!
//! - par emplacement : `probe_location_success` et
//!   `probe_location_duration_seconds`, étiquette `location` (`server` ou
//!   `agent-<id>`) ;
//! - la décision : les séries habituelles de la sonde (`probe_success` en tête),
//!   reprises d'un emplacement représentatif, `probe_success` portant le verdict
//!   du quorum — alertes, taux de disponibilité et pages de statut n'ont rien à
//!   savoir de plus — et `probe_degraded`, `probe_locations_failing`,
//!   `probe_locations_counted`.
//!
//! Tout l'état des rondes est en mémoire : un redémarrage recommence simplement
//! à la ronde suivante.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use chrono::{DateTime, Utc};
use dumbmonit_proto::{MetricKind, ProbeError, Sample, Target, TargetId};
use serde::Serialize;
use tokio::sync::{Semaphore, oneshot};
use tokio::time::Instant;
use tracing::{debug, info, warn};

use crate::collectors::Registry;
use crate::collectors::relay::{self, EnqueueError, Job, JobResult};
use crate::db;
use crate::db::multisite::MultisiteConfig;
use crate::state::AppState;

/// Types de moniteurs qui peuvent être sondés depuis plusieurs emplacements :
/// les sondes de disponibilité, qui écrivent `probe_success`.
pub const KINDS: [&str; 11] =
    ["http", "tcp", "dns", "ping", "tls", "smtp", "postgres", "mysql", "mqtt", "websocket", "ntp"];

/// Vrai si ce type de moniteur accepte plusieurs emplacements.
pub fn supports(kind: &str) -> bool {
    KINDS.contains(&kind)
}

/// Sondes multi-emplacements en attente ou en vol par agent, au-delà de quoi
/// l'emplacement est exclu de la ronde (« saturé ») plutôt que d'allonger la
/// file d'un petit site.
pub const MAX_PENDING_PER_AGENT: usize = 32;

/// Marge de la ronde au-delà de l'échéance des sondes relayées : leur
/// expiration (comptée « muette ») arrive ainsi avant la clôture de la ronde.
const ROUND_GRACE: Duration = Duration::from_secs(3);

/// Un endroit d'où la sonde part.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Location {
    /// Le serveur DumbMonit lui-même.
    Server,
    /// Un agent relais, par l'identifiant de sa cible `agent`.
    Agent(TargetId),
}

impl Location {
    /// Valeur de l'étiquette `location`.
    pub fn label(&self) -> String {
        match self {
            Self::Server => "server".to_string(),
            Self::Agent(id) => format!("agent-{id}"),
        }
    }

    pub fn agent_id(&self) -> Option<TargetId> {
        match self {
            Self::Server => None,
            Self::Agent(id) => Some(*id),
        }
    }
}

/// Emplacements d'un réglage, le serveur d'abord.
pub fn locations_of(config: &MultisiteConfig) -> Vec<Location> {
    let mut locations = Vec::with_capacity(config.locations());
    if config.server {
        locations.push(Location::Server);
    }
    let mut seen = HashSet::new();
    for agent in &config.agents {
        if seen.insert(*agent) {
            locations.push(Location::Agent(*agent));
        }
    }
    locations
}

/// Pourquoi un emplacement ne compte pas dans le quorum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Exclusion {
    /// Le relais n'est pas venu chercher la sonde, ou n'a pas répondu à temps.
    Silent,
    /// L'agent n'a pas activé le mode relais : il ne viendra jamais.
    NotRelay,
    /// L'agent a déjà trop de sondes en attente.
    Busy,
    /// L'emplacement n'a pas pu exécuter la sonde : type inconnu d'un agent
    /// ancien, réglage refusé.
    Error,
}

/// Ce qu'un emplacement a constaté.
#[derive(Debug, Clone)]
pub enum Outcome {
    Up { latency_ms: Option<u64>, samples: Vec<Sample> },
    Down { latency_ms: Option<u64>, samples: Vec<Sample>, reason: String },
    Excluded { reason: Exclusion, detail: Option<String> },
}

impl Outcome {
    pub fn is_up(&self) -> bool {
        matches!(self, Self::Up { .. })
    }

    fn samples(&self) -> &[Sample] {
        match self {
            Self::Up { samples, .. } | Self::Down { samples, .. } => samples,
            Self::Excluded { .. } => &[],
        }
    }
}

/// Verdict d'une ronde.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Up,
    Degraded,
    Down,
    /// Aucun emplacement n'a pu être compté.
    Unknown,
}

impl Verdict {
    /// Gravité, pour repérer une aggravation.
    fn rank(self) -> u8 {
        match self {
            Self::Up | Self::Unknown => 0,
            Self::Degraded => 1,
            Self::Down => 2,
        }
    }
}

/// Décision de quorum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decision {
    pub verdict: Verdict,
    /// Emplacements en échec nécessaires à la panne, après exclusions.
    pub quorum: usize,
    /// Emplacements comptés (les exclus en sont retirés).
    pub counted: usize,
    pub failing: usize,
}

/// Seuil de panne pour `configured` emplacements : celui réglé (borné au nombre
/// d'emplacements), sinon la majorité.
pub fn required(configured: usize, quorum: Option<u32>) -> usize {
    let configured = configured.max(1);
    match quorum {
        Some(quorum) => usize::try_from(quorum).unwrap_or(usize::MAX).clamp(1, configured),
        None => configured / 2 + 1,
    }
}

/// Décide d'après les constats d'une ronde. Les emplacements exclus ne comptent
/// pas ; s'il en reste moins que le seuil, il faut que tous ceux qui restent
/// échouent.
pub fn decide<'a>(
    configured: usize,
    quorum: Option<u32>,
    outcomes: impl IntoIterator<Item = &'a Outcome>,
) -> Decision {
    let (mut counted, mut failing) = (0usize, 0usize);
    for outcome in outcomes {
        match outcome {
            Outcome::Up { .. } => counted += 1,
            Outcome::Down { .. } => {
                counted += 1;
                failing += 1;
            }
            Outcome::Excluded { .. } => {}
        }
    }
    let needed = required(configured, quorum);
    if counted == 0 {
        return Decision { verdict: Verdict::Unknown, quorum: needed, counted, failing };
    }
    let needed = needed.min(counted);
    let verdict = if failing >= needed {
        Verdict::Down
    } else if failing > 0 {
        Verdict::Degraded
    } else {
        Verdict::Up
    };
    Decision { verdict, quorum: needed, counted, failing }
}

/// Faut-il confirmer avant de publier ? Oui pour une ronde ordinaire qui
/// aggrave le dernier verdict publié (première dégradation, première panne).
pub fn needs_confirmation(final_round: bool, published: Option<Verdict>, verdict: Verdict) -> bool {
    !final_round
        && matches!(verdict, Verdict::Degraded | Verdict::Down)
        && verdict.rank() > published.map_or(0, Verdict::rank)
}

// ------------------------------------------------------------------ état

/// État d'un emplacement, tel que l'interface l'affiche.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LocationState {
    Up,
    Down,
    Excluded,
    Pending,
}

/// Dernier constat d'un emplacement.
#[derive(Debug, Clone, Serialize)]
pub struct LocationStatus {
    pub state: LocationState,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
    pub excluded_reason: Option<Exclusion>,
    pub checked_at: DateTime<Utc>,
}

impl LocationStatus {
    fn from_outcome(outcome: &Outcome, at: DateTime<Utc>) -> Self {
        match outcome {
            Outcome::Up { latency_ms, .. } => Self {
                state: LocationState::Up,
                latency_ms: *latency_ms,
                error: None,
                excluded_reason: None,
                checked_at: at,
            },
            Outcome::Down { latency_ms, reason, .. } => Self {
                state: LocationState::Down,
                latency_ms: *latency_ms,
                error: Some(reason.clone()),
                excluded_reason: None,
                checked_at: at,
            },
            Outcome::Excluded { reason, detail } => Self {
                state: LocationState::Excluded,
                latency_ms: None,
                error: detail.clone(),
                excluded_reason: Some(*reason),
                checked_at: at,
            },
        }
    }
}

/// Ce qu'une ronde terminée rend à qui l'attendait (« Sonder maintenant »).
#[derive(Debug, Clone)]
pub struct Published {
    pub decision: Decision,
    pub series: Vec<String>,
}

pub struct Round {
    pub id: u64,
    pub target: Target,
    pub config: MultisiteConfig,
    expected: Vec<Location>,
    results: BTreeMap<Location, Outcome>,
    deadline: Instant,
    /// Ronde de confirmation, ou demandée à la main : sa décision est publiée
    /// telle quelle.
    final_round: bool,
    waiters: Vec<oneshot::Sender<Published>>,
}

impl Round {
    fn complete(&self) -> bool {
        self.expected.iter().all(|l| self.results.contains_key(l))
    }
}

#[derive(Default)]
struct Memory {
    locations: HashMap<Location, LocationStatus>,
    decision: Option<(Decision, DateTime<Utc>)>,
    /// Dernier verdict publié, pour repérer une aggravation.
    published: Option<Verdict>,
    confirming: bool,
}

#[derive(Default)]
struct Inner {
    next_id: u64,
    rounds: HashMap<TargetId, Round>,
    memory: HashMap<TargetId, Memory>,
    /// Agents qui ont déclaré le mode relais ; `None` tant que le planificateur
    /// ne l'a pas encore lu (chacun est alors présumé relais).
    relays: Option<HashSet<TargetId>>,
}

/// Issue d'une ronde close : ce qu'il faut en faire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Closing {
    Publish(Decision),
    /// Aggravation : une ronde de confirmation doit repartir.
    Confirm(Decision),
}

/// Rondes en cours et derniers constats, partagés par le planificateur, les
/// comptes rendus des relais et l'API.
#[derive(Default)]
pub struct MultiSite {
    inner: Mutex<Inner>,
    permits: OnceLock<Arc<Semaphore>>,
}

/// Instantané pour l'API.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub decision: Option<(Decision, DateTime<Utc>)>,
    pub confirming: bool,
    pub locations: Vec<(Location, Option<LocationStatus>)>,
}

impl MultiSite {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        // Rien n'est tenu à travers un `await`.
        self.inner.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Jetons d'exécution des sondes locales, partagés avec le planificateur.
    pub fn set_permits(&self, permits: Arc<Semaphore>) {
        let _ = self.permits.set(permits);
    }

    /// Agents qui ont déclaré le mode relais (rechargé par le planificateur).
    pub fn set_relays(&self, relays: HashSet<TargetId>) {
        self.lock().relays = Some(relays);
    }

    fn is_relay(&self, agent: TargetId) -> bool {
        self.lock().relays.as_ref().is_none_or(|relays| relays.contains(&agent))
    }

    /// Oublie ce qui concerne les cibles qui n'ont plus d'emplacements multiples.
    pub fn retain(&self, live: &HashSet<TargetId>) {
        let mut inner = self.lock();
        inner.memory.retain(|id, _| live.contains(id));
        inner.rounds.retain(|id, _| live.contains(id));
    }

    /// Ouvre une ronde, sauf si une autre est déjà en cours pour cette cible.
    pub fn begin(
        &self,
        target: &Target,
        config: &MultisiteConfig,
        final_round: bool,
        deadline: Instant,
        waiter: Option<oneshot::Sender<Published>>,
    ) -> Option<(u64, Vec<Location>)> {
        let mut inner = self.lock();
        if inner.rounds.contains_key(&target.id) {
            return None;
        }
        inner.next_id += 1;
        let id = inner.next_id;
        let expected = locations_of(config);
        inner.rounds.insert(
            target.id,
            Round {
                id,
                target: target.clone(),
                config: config.clone(),
                expected: expected.clone(),
                results: BTreeMap::new(),
                deadline,
                final_round,
                waiters: waiter.into_iter().collect(),
            },
        );
        inner.memory.entry(target.id).or_default();
        Some((id, expected))
    }

    /// Range un constat. Rend la ronde si c'était le dernier attendu.
    pub fn record(
        &self,
        target_id: TargetId,
        round_id: u64,
        location: Location,
        outcome: Outcome,
    ) -> Option<Round> {
        let mut inner = self.lock();
        let status = LocationStatus::from_outcome(&outcome, Utc::now());
        let round = inner.rounds.get_mut(&target_id).filter(|r| r.id == round_id)?;
        if !round.expected.contains(&location) {
            return None;
        }
        round.results.insert(location, outcome);
        let complete = round.complete();
        inner.memory.entry(target_id).or_default().locations.insert(location, status);
        if complete { inner.rounds.remove(&target_id) } else { None }
    }

    /// Clôt les rondes échues : les emplacements qui n'ont rien dit sont exclus
    /// comme muets.
    pub fn expire(&self, now: Instant) -> Vec<Round> {
        let mut inner = self.lock();
        let late: Vec<TargetId> =
            inner.rounds.iter().filter(|(_, r)| r.deadline <= now).map(|(id, _)| *id).collect();
        let mut closed = Vec::with_capacity(late.len());
        let at = Utc::now();
        for id in late {
            let Some(mut round) = inner.rounds.remove(&id) else { continue };
            for location in round.expected.clone() {
                if round.results.contains_key(&location) {
                    continue;
                }
                let outcome = Outcome::Excluded {
                    reason: Exclusion::Silent,
                    detail: Some(
                        "No answer from this location before the end of the round.".into(),
                    ),
                };
                inner
                    .memory
                    .entry(id)
                    .or_default()
                    .locations
                    .insert(location, LocationStatus::from_outcome(&outcome, at));
                round.results.insert(location, outcome);
            }
            closed.push(round);
        }
        closed
    }

    /// Décide d'une ronde close et dit s'il faut la publier ou la confirmer.
    pub fn close(&self, round: &Round) -> Closing {
        let decision = decide(round.expected.len(), round.config.quorum, round.results.values());
        let mut inner = self.lock();
        let memory = inner.memory.entry(round.target.id).or_default();
        if needs_confirmation(round.final_round, memory.published, decision.verdict) {
            memory.confirming = true;
            return Closing::Confirm(decision);
        }
        memory.confirming = false;
        memory.decision = Some((decision, Utc::now()));
        if decision.verdict != Verdict::Unknown {
            memory.published = Some(decision.verdict);
        }
        Closing::Publish(decision)
    }

    /// Ce que l'API montre d'une cible.
    pub fn snapshot(&self, target_id: TargetId, config: &MultisiteConfig) -> Snapshot {
        let inner = self.lock();
        let memory = inner.memory.get(&target_id);
        Snapshot {
            decision: memory.and_then(|m| m.decision),
            confirming: memory.is_some_and(|m| m.confirming),
            locations: locations_of(config)
                .into_iter()
                .map(|l| (l, memory.and_then(|m| m.locations.get(&l).cloned())))
                .collect(),
        }
    }
}

// --------------------------------------------------------------- conduite

/// Lance une ronde pour `target`. `None` si une ronde est déjà en cours. Le
/// récepteur rendu reçoit la décision publiée (pour « Sonder maintenant »).
pub fn start(
    state: &AppState,
    target: &Target,
    config: &MultisiteConfig,
    final_round: bool,
) -> Option<oneshot::Receiver<Published>> {
    let timeout = Registry::deadline_for(target, state.config.probe_timeout);
    let deadline = Instant::now() + relay::deadline_for(timeout) + ROUND_GRACE;
    let (tx, rx) = oneshot::channel();
    let (round_id, expected) =
        state.multisite().begin(target, config, final_round, deadline, Some(tx))?;
    let state = state.clone();
    let target = target.clone();
    tokio::spawn(async move {
        dispatch(state, target, round_id, expected, timeout, deadline).await;
    });
    Some(rx)
}

async fn dispatch(
    state: AppState,
    target: Target,
    round_id: u64,
    expected: Vec<Location>,
    timeout: Duration,
    deadline: Instant,
) {
    for location in expected {
        match location {
            Location::Server => {
                let state = state.clone();
                let target = target.clone();
                tokio::spawn(async move {
                    let permits = state.multisite().permits.get().cloned();
                    let _permit = match permits {
                        Some(permits) => match permits.acquire_owned().await {
                            Ok(permit) => Some(permit),
                            Err(_) => return, // arrêt du serveur
                        },
                        None => None,
                    };
                    let started = std::time::Instant::now();
                    let result = state.collectors.probe(&target, state.config.probe_timeout).await;
                    let outcome = from_local(result, started.elapsed());
                    crate::stats::stats().probe(&target.kind, !outcome.is_up());
                    deliver(&state, target.id, round_id, Location::Server, outcome).await;
                });
            }
            Location::Agent(agent) => {
                let excluded = if !state.multisite().is_relay(agent) {
                    Some((
                        Exclusion::NotRelay,
                        format!("Agent {agent} has not enabled relay mode."),
                    ))
                } else if state.relay().pending(agent) >= MAX_PENDING_PER_AGENT {
                    Some((
                        Exclusion::Busy,
                        format!(
                            "Agent {agent} already has {MAX_PENDING_PER_AGENT} probes or more waiting."
                        ),
                    ))
                } else {
                    // Le récepteur n'est pas attendu : le compte rendu passe par
                    // `relay::settle`, qui remet le constat à la ronde.
                    match state.relay().enqueue_round(
                        agent,
                        target.clone(),
                        timeout,
                        deadline - ROUND_GRACE,
                        round_id,
                    ) {
                        Ok(_) => None,
                        Err(EnqueueError::Busy) => Some((
                            Exclusion::Busy,
                            format!("Agent {agent} is still busy with the previous probe."),
                        )),
                    }
                };
                if let Some((reason, detail)) = excluded {
                    let outcome = Outcome::Excluded { reason, detail: Some(detail) };
                    deliver(&state, target.id, round_id, location, outcome).await;
                }
            }
        }
    }
}

/// Constat d'une sonde exécutée par le serveur.
pub fn from_local(result: Result<Vec<Sample>, ProbeError>, elapsed: Duration) -> Outcome {
    match result {
        Ok(samples) => from_samples(samples, Some(elapsed_ms(elapsed))),
        // Délai dépassé, hôte injoignable : une panne vue d'ici.
        Err(error) if error.means_down() => {
            Outcome::Down { latency_ms: None, samples: Vec::new(), reason: error.to_string() }
        }
        Err(error) => {
            Outcome::Excluded { reason: Exclusion::Error, detail: Some(error.to_string()) }
        }
    }
}

/// Constat d'une sonde confiée à un relais.
pub fn from_relay(state: &AppState, job: &Job, result: &JobResult) -> Outcome {
    match result {
        JobResult::Expired => Outcome::Excluded {
            reason: Exclusion::Silent,
            detail: Some(relay::expired_message(job)),
        },
        JobResult::Done(outcome) => match &outcome.error {
            Some(message) if db::targets::error_means_down(message) => Outcome::Down {
                latency_ms: Some(outcome.duration_ms),
                samples: Vec::new(),
                reason: message.clone(),
            },
            // Type inconnu d'un agent ancien, réglage refusé : l'emplacement ne
            // sait pas sonder, il ne dit rien du service.
            Some(message) => {
                Outcome::Excluded { reason: Exclusion::Error, detail: Some(message.clone()) }
            }
            None => {
                let samples = relay::prepare(
                    &state.collectors,
                    &job.target,
                    outcome.samples.clone(),
                    Utc::now().timestamp_millis(),
                );
                from_samples(samples, Some(outcome.duration_ms))
            }
        },
    }
}

fn elapsed_ms(elapsed: Duration) -> u64 {
    u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
}

/// Verdict d'une sonde qui a tourné : `probe_success = 0` est une panne.
fn from_samples(samples: Vec<Sample>, fallback_ms: Option<u64>) -> Outcome {
    let measured = samples
        .iter()
        .find(|s| s.metric == "probe_duration_seconds" && s.value.is_finite() && s.value >= 0.0)
        .map(|s| (s.value * 1000.0).round() as u64);
    let latency_ms = measured.or(fallback_ms);
    match db::targets::service_failure(&samples) {
        Some(reason) => Outcome::Down { latency_ms, samples, reason },
        None => Outcome::Up { latency_ms, samples },
    }
}

/// Remet le constat d'un emplacement à sa ronde, écrit ses séries propres, et
/// clôt la ronde s'il était le dernier attendu.
pub async fn deliver(
    state: &AppState,
    target_id: TargetId,
    round_id: u64,
    location: Location,
    outcome: Outcome,
) {
    let per_location = match state.multisite().lock().rounds.get(&target_id) {
        Some(round) if round.id == round_id => location_samples(
            &state.collectors,
            &round.target,
            location,
            &outcome,
            Utc::now().timestamp_millis(),
        ),
        _ => Vec::new(),
    };
    if !per_location.is_empty() {
        state.sink.send(per_location).await;
    }
    debug!(target = target_id, location = %location.label(), ?outcome, "location reported");
    if let Some(round) = state.multisite().record(target_id, round_id, location, outcome) {
        finish(state, round).await;
    }
}

/// Clôt les rondes échues. Appelé à chaque tour du planificateur.
pub async fn expire(state: &AppState, now: Instant) {
    for round in state.multisite().expire(now) {
        finish(state, round).await;
    }
}

async fn finish(state: &AppState, mut round: Round) {
    match state.multisite().close(&round) {
        Closing::Confirm(decision) => {
            info!(
                target = round.target.id,
                failing = decision.failing,
                counted = decision.counted,
                "failure seen, confirming from every location"
            );
            for waiter in round.waiters.drain(..) {
                let _ = waiter.send(Published { decision, series: Vec::new() });
            }
            if start(state, &round.target, &round.config, true).is_none() {
                debug!(target = round.target.id, "confirmation round already running");
            }
        }
        Closing::Publish(decision) => {
            let samples = aggregate(
                &state.collectors,
                &round.target,
                &round.results,
                decision,
                Utc::now().timestamp_millis(),
            );
            let series: Vec<String> = samples.iter().map(Sample::series_key).collect();
            if !samples.is_empty() {
                state.sink.send(samples).await;
            }
            let error = last_error(&round.results, decision);
            if let Err(error) =
                db::targets::record_probe(&state.pool, round.target.id, error.as_deref()).await
            {
                warn!(target = round.target.id, ?error, "cannot record the quorum decision");
            }
            debug!(target = round.target.id, ?decision, "quorum decision published");
            for waiter in round.waiters.drain(..) {
                let _ = waiter.send(Published { decision, series: series.clone() });
            }
        }
    }
}

/// Séries propres à un emplacement : succès et durée, étiquette `location`.
pub fn location_samples(
    registry: &Registry,
    target: &Target,
    location: Location,
    outcome: &Outcome,
    now_ms: i64,
) -> Vec<Sample> {
    let (up, latency_ms) = match outcome {
        Outcome::Up { latency_ms, .. } => (true, *latency_ms),
        Outcome::Down { latency_ms, .. } => (false, *latency_ms),
        Outcome::Excluded { .. } => return Vec::new(),
    };
    let (mut labels, ts_ms) = verdict_labels(registry, target, outcome.samples(), now_ms);
    labels.insert("location".to_string(), location.label());
    let mut samples = vec![Sample {
        metric: "probe_location_success".to_string(),
        labels: labels.clone(),
        value: if up { 1.0 } else { 0.0 },
        kind: MetricKind::Gauge,
        ts_ms,
    }];
    if let Some(ms) = latency_ms {
        samples.push(Sample {
            metric: "probe_location_duration_seconds".to_string(),
            labels,
            value: ms as f64 / 1000.0,
            kind: MetricKind::Gauge,
            ts_ms,
        });
    }
    samples
}

/// Étiquettes et horodatage du `probe_success` d'un lot, ou ceux de la cible
/// quand la sonde n'a rien écrit.
fn verdict_labels(
    registry: &Registry,
    target: &Target,
    samples: &[Sample],
    now_ms: i64,
) -> (BTreeMap<String, String>, i64) {
    match samples.iter().find(|s| s.metric == "probe_success") {
        Some(sample) => (sample.labels.clone(), sample.ts_ms),
        None => {
            let mut labels = registry.base_labels(target);
            labels.insert("probe".to_string(), target.kind.clone());
            (labels, now_ms)
        }
    }
}

/// Séries de la décision : celles d'un emplacement représentatif, d'accord avec
/// le verdict (le serveur d'abord), `probe_success` portant le quorum. Rien
/// quand aucun emplacement n'a pu être compté : la surveillance elle-même est
/// alors tombée, et `host_down` le dira.
pub fn aggregate(
    registry: &Registry,
    target: &Target,
    results: &BTreeMap<Location, Outcome>,
    decision: Decision,
    now_ms: i64,
) -> Vec<Sample> {
    if decision.verdict == Verdict::Unknown {
        return Vec::new();
    }
    let down = decision.verdict == Verdict::Down;
    // `BTreeMap` : le serveur passe avant les agents, puis les agents par
    // identifiant — le représentant ne change pas d'une ronde à l'autre.
    let representative = results
        .values()
        .filter(|o| !o.samples().is_empty())
        .find(|o| if down { matches!(o, Outcome::Down { .. }) } else { o.is_up() });
    let mut samples: Vec<Sample> = representative.map(|o| o.samples().to_vec()).unwrap_or_default();
    if !down {
        samples.retain(|s| s.metric != "probe_failure_info");
    }
    let (labels, ts_ms) = verdict_labels(registry, target, &samples, now_ms);
    let success = if down { 0.0 } else { 1.0 };
    let mut found = false;
    for sample in samples.iter_mut().filter(|s| s.metric == "probe_success") {
        sample.value = success;
        found = true;
    }
    let gauge = |metric: &str, value: f64| Sample {
        metric: metric.to_string(),
        labels: labels.clone(),
        value,
        kind: MetricKind::Gauge,
        ts_ms,
    };
    if !found {
        samples.push(gauge("probe_success", success));
    }
    samples.push(gauge(
        "probe_degraded",
        if decision.verdict == Verdict::Degraded { 1.0 } else { 0.0 },
    ));
    samples.push(gauge("probe_locations_failing", decision.failing as f64));
    samples.push(gauge("probe_locations_counted", decision.counted as f64));
    samples
}

/// Message enregistré sur la cible pour la décision publiée.
pub fn last_error(results: &BTreeMap<Location, Outcome>, decision: Decision) -> Option<String> {
    match decision.verdict {
        Verdict::Up | Verdict::Degraded => None,
        Verdict::Down => {
            let reason = results
                .values()
                .find_map(|o| match o {
                    Outcome::Down { reason, .. } => Some(reason.as_str()),
                    _ => None,
                })
                .unwrap_or("Service down: the check failed");
            Some(format!("{reason} (from {} of {} locations)", decision.failing, decision.counted))
        }
        Verdict::Unknown => Some(
            results
                .values()
                .find_map(|o| match o {
                    Outcome::Excluded { reason: Exclusion::Error, detail: Some(detail) } => {
                        Some(detail.clone())
                    }
                    _ => None,
                })
                .unwrap_or_else(|| "Timed out: no probe location answered in time".to_string()),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn up() -> Outcome {
        Outcome::Up { latency_ms: Some(12), samples: Vec::new() }
    }

    fn down() -> Outcome {
        Outcome::Down {
            latency_ms: Some(5000),
            samples: Vec::new(),
            reason: "Service down: the http check failed (timeout)".into(),
        }
    }

    fn silent() -> Outcome {
        Outcome::Excluded { reason: Exclusion::Silent, detail: None }
    }

    fn target(id: TargetId) -> Target {
        Target {
            id,
            name: format!("site-{id}"),
            address: "https://example.org".into(),
            kind: "http".into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            tags: BTreeMap::new(),
            credential: dumbmonit_proto::Credential::None,
            group_name: String::new(),
            position: 0,
        }
    }

    fn config(server: bool, agents: &[TargetId], quorum: Option<u32>) -> MultisiteConfig {
        MultisiteConfig { server, agents: agents.to_vec(), quorum }
    }

    #[test]
    fn la_majorite_est_le_seuil_par_defaut() {
        assert_eq!(required(2, None), 2);
        assert_eq!(required(3, None), 2);
        assert_eq!(required(4, None), 3);
        assert_eq!(required(5, None), 3);
        // Un seuil réglé est borné au nombre d'emplacements.
        assert_eq!(required(3, Some(1)), 1);
        assert_eq!(required(3, Some(9)), 3);
        assert_eq!(required(3, Some(0)), 1);
    }

    #[test]
    fn un_seul_emplacement_en_echec_est_une_degradation() {
        let decision = decide(3, None, &[up(), down(), up()]);
        assert_eq!(decision.verdict, Verdict::Degraded);
        assert_eq!((decision.failing, decision.counted, decision.quorum), (1, 3, 2));
    }

    #[test]
    fn le_quorum_atteint_est_une_panne() {
        assert_eq!(decide(3, None, &[down(), down(), up()]).verdict, Verdict::Down);
        assert_eq!(decide(3, Some(3), &[down(), down(), up()]).verdict, Verdict::Degraded);
        assert_eq!(decide(3, Some(1), &[down(), up(), up()]).verdict, Verdict::Down);
        assert_eq!(decide(2, None, &[up(), up()]).verdict, Verdict::Up);
    }

    #[test]
    fn un_emplacement_muet_est_exclu_du_quorum() {
        // Trois emplacements, un muet : il en reste deux, la majorité (2) tient.
        let decision = decide(3, None, &[silent(), down(), up()]);
        assert_eq!(decision.verdict, Verdict::Degraded);
        assert_eq!(decision.counted, 2);
        // Le muet ne compte pas comme un échec.
        assert_eq!(decide(3, None, &[silent(), silent(), up()]).verdict, Verdict::Up);
        // Moins d'emplacements comptés que le seuil : tous ceux qui restent.
        let decision = decide(3, Some(3), &[silent(), down(), down()]);
        assert_eq!(decision.verdict, Verdict::Down);
        assert_eq!(decision.quorum, 2);
        // Plus personne : on ne sait rien.
        assert_eq!(decide(2, None, &[silent(), silent()]).verdict, Verdict::Unknown);
    }

    #[test]
    fn seule_une_aggravation_demande_confirmation() {
        assert!(needs_confirmation(false, None, Verdict::Down));
        assert!(needs_confirmation(false, Some(Verdict::Up), Verdict::Degraded));
        assert!(needs_confirmation(false, Some(Verdict::Degraded), Verdict::Down));
        // Déjà en panne : on publie la suite sans redemander.
        assert!(!needs_confirmation(false, Some(Verdict::Down), Verdict::Down));
        assert!(!needs_confirmation(false, Some(Verdict::Down), Verdict::Degraded));
        assert!(!needs_confirmation(false, Some(Verdict::Up), Verdict::Up));
        // La ronde de confirmation publie ce qu'elle voit.
        assert!(!needs_confirmation(true, Some(Verdict::Up), Verdict::Down));
    }

    #[test]
    fn une_premiere_panne_est_confirmee_avant_d_etre_publiee() {
        let hub = MultiSite::new();
        let target = target(7);
        let config = config(true, &[3], None);
        let deadline = Instant::now() + Duration::from_secs(60);

        // Première ronde : tout va bien, publiée.
        let (id, _) = hub.begin(&target, &config, false, deadline, None).expect("ronde");
        assert!(hub.record(7, id, Location::Server, up()).is_none(), "l'agent n'a pas répondu");
        let round = hub.record(7, id, Location::Agent(3), up()).expect("ronde complète");
        assert!(matches!(hub.close(&round), Closing::Publish(d) if d.verdict == Verdict::Up));

        // Panne vue des deux emplacements : pas publiée, à confirmer.
        let (id, _) = hub.begin(&target, &config, false, deadline, None).expect("ronde");
        assert!(hub.begin(&target, &config, false, deadline, None).is_none(), "une à la fois");
        hub.record(7, id, Location::Server, down());
        let round = hub.record(7, id, Location::Agent(3), down()).expect("ronde complète");
        assert!(matches!(hub.close(&round), Closing::Confirm(d) if d.verdict == Verdict::Down));
        assert!(hub.snapshot(7, &config).confirming);

        // La confirmation voit la panne : publiée.
        let (id, _) = hub.begin(&target, &config, true, deadline, None).expect("confirmation");
        hub.record(7, id, Location::Server, down());
        let round = hub.record(7, id, Location::Agent(3), down()).expect("ronde complète");
        assert!(matches!(hub.close(&round), Closing::Publish(d) if d.verdict == Verdict::Down));
        let snapshot = hub.snapshot(7, &config);
        assert!(!snapshot.confirming);
        assert_eq!(snapshot.decision.map(|(d, _)| d.verdict), Some(Verdict::Down));

        // Toujours en panne au tour suivant : publiée sans nouvelle confirmation.
        let (id, _) = hub.begin(&target, &config, false, deadline, None).expect("ronde");
        hub.record(7, id, Location::Server, down());
        let round = hub.record(7, id, Location::Agent(3), down()).expect("ronde complète");
        assert!(matches!(hub.close(&round), Closing::Publish(_)));
    }

    #[test]
    fn une_confirmation_qui_ne_voit_rien_publie_le_retour() {
        let hub = MultiSite::new();
        let target = target(8);
        let config = config(true, &[3], None);
        let deadline = Instant::now() + Duration::from_secs(60);
        let (id, _) = hub.begin(&target, &config, false, deadline, None).unwrap();
        hub.record(8, id, Location::Server, down());
        let round = hub.record(8, id, Location::Agent(3), up()).unwrap();
        assert!(matches!(hub.close(&round), Closing::Confirm(d) if d.verdict == Verdict::Degraded));
        let (id, _) = hub.begin(&target, &config, true, deadline, None).unwrap();
        hub.record(8, id, Location::Server, up());
        let round = hub.record(8, id, Location::Agent(3), up()).unwrap();
        assert!(matches!(hub.close(&round), Closing::Publish(d) if d.verdict == Verdict::Up));
    }

    #[test]
    fn une_ronde_echue_exclut_les_emplacements_muets() {
        let hub = MultiSite::new();
        let target = target(9);
        let config = config(true, &[3, 4], None);
        let now = Instant::now();
        let (id, expected) =
            hub.begin(&target, &config, true, now + Duration::from_secs(1), None).unwrap();
        assert_eq!(expected, vec![Location::Server, Location::Agent(3), Location::Agent(4)]);
        hub.record(9, id, Location::Server, down());
        assert!(hub.expire(now).is_empty(), "pas encore échue");
        let rounds = hub.expire(now + Duration::from_secs(2));
        assert_eq!(rounds.len(), 1);
        // Deux muets exclus : seul le serveur compte, et il échoue.
        let closing = hub.close(&rounds[0]);
        assert!(
            matches!(closing, Closing::Publish(d) if d.verdict == Verdict::Down && d.counted == 1)
        );
        let snapshot = hub.snapshot(9, &config);
        let agent = snapshot.locations.iter().find(|(l, _)| *l == Location::Agent(4)).unwrap();
        let status = agent.1.as_ref().expect("constat");
        assert_eq!(status.state, LocationState::Excluded);
        assert_eq!(status.excluded_reason, Some(Exclusion::Silent));
    }

    #[test]
    fn un_constat_d_une_ronde_close_est_ignore() {
        let hub = MultiSite::new();
        let target = target(10);
        let config = config(true, &[3], None);
        let deadline = Instant::now() + Duration::from_secs(60);
        let (id, _) = hub.begin(&target, &config, false, deadline, None).unwrap();
        assert!(hub.record(10, id + 1, Location::Server, up()).is_none());
        assert!(hub.record(10, id, Location::Agent(99), up()).is_none(), "pas attendu");
    }

    fn success(value: f64) -> Sample {
        Sample::new("probe_success", value, MetricKind::Gauge, 1_000)
            .with_label("probe", "http")
            .with_label("target", "7")
    }

    #[test]
    fn la_disponibilite_publiee_suit_le_quorum() {
        let registry = Registry::new();
        let target = target(7);
        let failing = Outcome::Down {
            latency_ms: Some(5000),
            samples: vec![
                success(0.0),
                Sample::new("probe_failure_info", 1.0, MetricKind::Gauge, 1_000)
                    .with_label("reason", "timeout"),
            ],
            reason: "Service down: the http check failed (timeout)".into(),
        };
        let fine = Outcome::Up { latency_ms: Some(40), samples: vec![success(1.0)] };
        let results = BTreeMap::from([
            (Location::Server, failing.clone()),
            (Location::Agent(3), fine.clone()),
            (Location::Agent(4), fine),
        ]);

        // Dégradé : la disponibilité reste à 1, depuis un emplacement qui va bien.
        let decision = decide(3, None, results.values());
        assert_eq!(decision.verdict, Verdict::Degraded);
        let samples = aggregate(&registry, &target, &results, decision, 2_000);
        let value = |metric: &str| samples.iter().find(|s| s.metric == metric).map(|s| s.value);
        assert_eq!(value("probe_success"), Some(1.0));
        assert_eq!(value("probe_degraded"), Some(1.0));
        assert_eq!(value("probe_locations_failing"), Some(1.0));
        assert_eq!(value("probe_locations_counted"), Some(3.0));
        assert_eq!(value("probe_failure_info"), None);
        assert!(last_error(&results, decision).is_none());

        // En panne : 0, avec la raison de l'échec.
        let results = BTreeMap::from([
            (Location::Server, failing.clone()),
            (Location::Agent(3), failing),
            (Location::Agent(4), silent()),
        ]);
        let decision = decide(3, None, results.values());
        assert_eq!(decision.verdict, Verdict::Down);
        let samples = aggregate(&registry, &target, &results, decision, 2_000);
        let value = |metric: &str| samples.iter().find(|s| s.metric == metric).map(|s| s.value);
        assert_eq!(value("probe_success"), Some(0.0));
        assert_eq!(value("probe_degraded"), Some(0.0));
        assert_eq!(value("probe_failure_info"), Some(1.0));
        let message = last_error(&results, decision).expect("message");
        assert!(db::targets::error_means_service_down(&message), "{message}");
        assert!(message.ends_with("(from 2 of 2 locations)"), "{message}");

        // Personne n'a pu sonder : rien d'écrit, la surveillance est tombée.
        let results = BTreeMap::from([(Location::Server, silent())]);
        let decision = decide(1, None, results.values());
        assert!(aggregate(&registry, &target, &results, decision, 2_000).is_empty());
        assert!(db::targets::error_means_down(&last_error(&results, decision).unwrap()));
    }

    #[test]
    fn une_panne_sans_mesure_ecrit_quand_meme_un_zero() {
        let registry = Registry::new();
        let target = target(7);
        let results = BTreeMap::from([(Location::Server, down()), (Location::Agent(3), down())]);
        let decision = decide(2, None, results.values());
        let samples = aggregate(&registry, &target, &results, decision, 2_000);
        let success = samples.iter().find(|s| s.metric == "probe_success").expect("verdict");
        assert_eq!(success.value, 0.0);
        assert_eq!(success.labels.get("probe").map(String::as_str), Some("http"));
        assert_eq!(success.labels.get("target").map(String::as_str), Some("7"));
    }

    #[test]
    fn chaque_emplacement_a_ses_series() {
        let registry = Registry::new();
        let target = target(7);
        let outcome = Outcome::Up { latency_ms: Some(250), samples: vec![success(1.0)] };
        let samples = location_samples(&registry, &target, Location::Agent(3), &outcome, 2_000);
        assert_eq!(samples.len(), 2);
        assert!(
            samples.iter().all(|s| s.labels.get("location").map(String::as_str) == Some("agent-3"))
        );
        assert_eq!(samples[0].metric, "probe_location_success");
        assert_eq!(samples[1].metric, "probe_location_duration_seconds");
        assert_eq!(samples[1].value, 0.25);
        assert!(location_samples(&registry, &target, Location::Server, &silent(), 0).is_empty());
    }

    #[test]
    fn une_erreur_de_reglage_exclut_l_emplacement() {
        let outcome = from_local(Err(ProbeError::Config("unknown kind".into())), Duration::ZERO);
        assert!(matches!(outcome, Outcome::Excluded { reason: Exclusion::Error, .. }));
        let outcome = from_local(Err(ProbeError::Timeout(Duration::from_secs(5))), Duration::ZERO);
        assert!(matches!(outcome, Outcome::Down { .. }));
        let outcome = from_local(Ok(vec![success(0.0)]), Duration::from_millis(30));
        assert!(matches!(outcome, Outcome::Down { latency_ms: Some(30), .. }));
    }
}
