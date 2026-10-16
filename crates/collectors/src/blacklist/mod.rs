//! Listes noires DNS : la réputation d'un serveur mail (`mail_blacklist`) et
//! celle d'une plage d'adresses publiques (`ip_blacklist`).
//!
//! Les deux moniteurs posent des questions `A` aux DNSBL du catalogue
//! (`lists.rs`) et lisent leurs codes. Trois règles tiennent l'ensemble :
//!
//! * `NXDOMAIN` veut dire « absent » ; une absence de réponse (délai,
//!   `SERVFAIL`, réponse hors de `127.0.0.0/8`) ne veut **pas** dire absent :
//!   la liste est marquée inconnue pour ce sujet ;
//! * un code de refus (Spamhaus `127.255.255.252/254/255`, URIBL et SURBL
//!   `127.0.0.1`) n'est jamais une inscription. Un refus de Spamhaus — presque
//!   toujours un résolveur public — fait échouer la vérification en erreur de
//!   configuration ([`ProbeError::Config`], qui n'alerte pas) : sans elle, le
//!   résultat le plus important manque. Celui d'une autre liste la laisse
//!   inconnue ;
//! * les listes limitent le débit et bannissent les balayages : les questions
//!   partent à `queries_per_second` (10 par défaut), huit au plus en vol, et un
//!   passage n'est refait que toutes les `refresh_hours` (6 par défaut).
//!
//! Le passage tourne en tâche de fond : une plage /24 demande près de deux mille
//! questions, bien plus que le délai d'une sonde. La sonde attend quelques
//! secondes qu'il finisse, puis réécrit le dernier résultat connu à chaque
//! intervalle, avec `blacklist_scan_running` tant qu'un passage est en cours.
//!
//! # Étiquettes
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `resolver` | système | Résolveur récursif à interroger, `10.0.0.53` ou `10.0.0.53:5353`. |
//! | `lists` | toutes | Identifiants des listes, séparés par des virgules. |
//! | `extra_addresses` | — | `mail_blacklist` : adresses d'envoi à vérifier en plus. |
//! | `refresh_hours` | `6` | Intervalle entre deux passages (1 à 168). |
//! | `queries_per_second` | `10` | Débit des questions (1 à 50). |
//! | `query_timeout_seconds` | `5` | Délai par question (1 à 30). |
//!
//! Le secret facultatif (`api_token`) est une clé du Data Query Service de
//! Spamhaus : les zones Spamhaus sont alors interrogées sous
//! `<clé>.zen.dq.spamhaus.net`, ce qui lève le refus des résolveurs publics.

pub mod check;
pub mod dns;
pub mod lists;
pub mod mail;
pub mod options;
pub mod range;
pub mod scan;

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, MetricKind, ProbeError, Sample, Target};
use futures::FutureExt;
use futures::future::{BoxFuture, Shared};

use check::{Outcome, Verdict};
use dns::{Dns, HickoryDns};
use options::Settings;
use scan::Scan;

pub use options::{
    DEFAULT_QUERIES_PER_SECOND, DEFAULT_QUERY_TIMEOUT_SECONDS, DEFAULT_REFRESH_HOURS,
};

/// Ce qu'une sonde attend un passage avant de rendre la main. Sous le délai
/// global du planificateur (dix secondes par défaut).
const WAIT: Duration = Duration::from_secs(5);
/// Un passage en erreur est retenté au bout de ce délai, pas de `refresh_hours`.
const ERROR_RETRY: Duration = Duration::from_secs(10 * 60);
/// Une cible qui n'a plus été sondée depuis une semaine est oubliée.
const FORGET: Duration = Duration::from_secs(7 * 24 * 3600);
/// Passages simultanés, toutes cibles confondues.
const PARALLEL_SCANS: usize = 2;

/// Fabrique le résolveur d'un passage (remplacée dans les tests).
pub type DnsFactory = Arc<dyn Fn(&Settings) -> Result<Arc<dyn Dns>, ProbeError> + Send + Sync>;

fn hickory(settings: &Settings) -> Result<Arc<dyn Dns>, ProbeError> {
    Ok(Arc::new(HickoryDns::new(settings.resolver, settings.query_timeout)?))
}

/// Ce qui ne doit pas passer pour un résultat : un refus de Spamhaus, ou un
/// résolveur qui n'a répondu à rien.
pub fn conclude(outcomes: &[Outcome]) -> Result<(), ProbeError> {
    if let Some((list, why)) = outcomes.iter().find_map(|o| match &o.verdict {
        Verdict::Refused(why) if o.list.dqs_zone.is_some() => Some((o.list, why)),
        _ => None,
    }) {
        return Err(ProbeError::Config(format!(
            "{why}. {} results would be wrong, so none are shown. Set Resolver to a recursive \
             resolver of your own (Unbound, Knot Resolver, BIND, PowerDNS Recursor: one that \
             resolves from the root servers, not one that forwards to 8.8.8.8 or 1.1.1.1), or \
             enter a free Spamhaus DQS key as the credential, or remove spamhaus_zen and \
             spamhaus_dbl from Lists.",
            list.name
        )));
    }
    let asked: Vec<&Outcome> =
        outcomes.iter().filter(|o| o.verdict != Verdict::NotApplicable).collect();
    if !asked.is_empty() && asked.iter().all(|o| matches!(o.verdict, Verdict::Unknown(_))) {
        let detail = match &asked[0].verdict {
            Verdict::Unknown(detail) => detail.clone(),
            _ => String::new(),
        };
        return Err(ProbeError::Unreachable(format!(
            "none of the {} blocklist queries got an answer ({detail}): the resolver is \
             unreachable or too slow",
            asked.len()
        )));
    }
    Ok(())
}

/// Une erreur gardée en mémoire ([`ProbeError`] n'est pas clonable).
#[derive(Clone)]
enum Stored {
    Config(String),
    Unreachable(String),
    Timeout(Duration),
    Other(String),
}

impl From<ProbeError> for Stored {
    fn from(error: ProbeError) -> Self {
        match error {
            ProbeError::Config(m) => Self::Config(m),
            ProbeError::Unreachable(m) => Self::Unreachable(m),
            ProbeError::Timeout(d) => Self::Timeout(d),
            other => Self::Other(other.to_string()),
        }
    }
}

impl From<Stored> for ProbeError {
    fn from(error: Stored) -> Self {
        match error {
            Stored::Config(m) => Self::Config(m),
            Stored::Unreachable(m) => Self::Unreachable(m),
            Stored::Timeout(d) => Self::Timeout(d),
            Stored::Other(m) => Self::Protocol(m),
        }
    }
}

#[derive(Clone)]
struct Done {
    at: Instant,
    result: Result<Arc<Scan>, Stored>,
}

#[derive(Default)]
struct Slot {
    done: Option<Done>,
    running: Option<Shared<BoxFuture<'static, ()>>>,
    touched: Option<Instant>,
}

type Slots = Arc<Mutex<HashMap<u64, Slot>>>;

/// Les passages en cours et le dernier résultat de chaque cible.
struct Scanner {
    dns: DnsFactory,
    slots: Slots,
    permits: Arc<tokio::sync::Semaphore>,
    wait: Duration,
}

impl Scanner {
    fn new(dns: DnsFactory) -> Self {
        Self {
            dns,
            slots: Arc::default(),
            permits: Arc::new(tokio::sync::Semaphore::new(PARALLEL_SCANS)),
            wait: WAIT,
        }
    }

    /// Une empreinte de la cible et de tout ce qui change le passage : modifier
    /// l'adresse, une option ou la clé relance aussitôt un passage.
    fn key(target: &Target) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (target.id, &target.kind, &target.address, &target.tags).hash(&mut hasher);
        if let Credential::ApiToken { token } = &target.credential {
            token.hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Le dernier passage de la cible, en lançant un nouveau s'il est périmé.
    /// `job` n'est appelé qu'alors : un passage frais ne coûte rien.
    async fn probe(
        &self,
        target: &Target,
        refresh: Duration,
        job: impl FnOnce() -> Result<BoxFuture<'static, Result<Scan, ProbeError>>, ProbeError>,
    ) -> Result<Vec<Sample>, ProbeError> {
        let key = Self::key(target);
        let now = Instant::now();
        let running = {
            let mut slots = self.slots.lock().unwrap_or_else(|p| p.into_inner());
            slots.retain(|_, slot| {
                slot.running.is_some() || slot.touched.is_some_and(|t| now - t < FORGET)
            });
            let slot = slots.entry(key).or_default();
            slot.touched = Some(now);
            let fresh = slot.done.as_ref().is_some_and(|done| {
                let ttl = if done.result.is_ok() { refresh } else { ERROR_RETRY.min(refresh) };
                done.at.elapsed() < ttl
            });
            if !fresh && slot.running.is_none() {
                slot.running = Some(self.spawn(key, job()?));
            }
            slot.running.clone()
        };
        if let Some(running) = running {
            let _ = tokio::time::timeout(self.wait, running).await;
        }
        let (done, running) = {
            let slots = self.slots.lock().unwrap_or_else(|p| p.into_inner());
            let slot = slots.get(&key);
            (slot.and_then(|s| s.done.clone()), slot.is_some_and(|s| s.running.is_some()))
        };
        let ts = chrono::Utc::now().timestamp_millis();
        match done {
            Some(Done { at, result: Ok(scan) }) => Ok(scan.samples(at.elapsed(), running, ts)),
            Some(Done { result: Err(error), .. }) => Err(error.into()),
            // Premier passage encore en cours : rien à montrer, sauf qu'il tourne.
            None => Ok(vec![Sample::new("blacklist_scan_running", 1.0, MetricKind::Gauge, ts)]),
        }
    }

    /// Lance le passage en tâche de fond : il survit à la sonde qui l'a
    /// demandé, et range lui-même son résultat.
    fn spawn(
        &self,
        key: u64,
        job: BoxFuture<'static, Result<Scan, ProbeError>>,
    ) -> Shared<BoxFuture<'static, ()>> {
        let (slots, permits) = (self.slots.clone(), self.permits.clone());
        let store = move |slots: &Slots, result: Result<Arc<Scan>, Stored>| {
            let mut slots = slots.lock().unwrap_or_else(|p| p.into_inner());
            let slot = slots.entry(key).or_default();
            slot.done = Some(Done { at: Instant::now(), result });
            slot.running = None;
        };
        let task = {
            let slots = slots.clone();
            tokio::spawn(async move {
                let _permit = permits.acquire_owned().await;
                let result = job.await.map(Arc::new).map_err(Stored::from);
                store(&slots, result);
            })
        };
        async move {
            // Une tâche qui a paniqué ne doit pas laisser le passage « en cours »
            // pour toujours.
            if task.await.is_err() {
                store(
                    &slots,
                    Err(Stored::Other("the blocklist check stopped unexpectedly".into())),
                );
            }
        }
        .boxed()
        .shared()
    }
}

macro_rules! collector {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        pub struct $name {
            scanner: Scanner,
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl $name {
            pub fn new() -> Self {
                Self::with_dns(Arc::new(hickory))
            }

            /// Un autre résolveur que celui du système (tests).
            pub fn with_dns(dns: DnsFactory) -> Self {
                Self { scanner: Scanner::new(dns) }
            }
        }
    };
}

collector!(MailBlacklistCollector, "Réputation d'un serveur mail.");
collector!(IpBlacklistCollector, "Liste noire d'une plage d'adresses.");

#[async_trait]
impl Collector for MailBlacklistCollector {
    fn kind(&self) -> &'static str {
        "mail_blacklist"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let settings = Settings::from_target(target)?;
        let input = mail::parse(&target.address)?;
        let job = || {
            let dns = (self.scanner.dns)(&settings)?;
            Ok(mail::scan(dns, input, settings.clone()).boxed())
        };
        self.scanner.probe(target, settings.refresh, job).await
    }
}

#[async_trait]
impl Collector for IpBlacklistCollector {
    fn kind(&self) -> &'static str {
        "ip_blacklist"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let settings = Settings::from_target(target)?;
        let addresses = range::parse(&target.address)?;
        let job = || {
            let dns = (self.scanner.dns)(&settings)?;
            Ok(range::scan(dns, addresses, settings.clone()).boxed())
        };
        self.scanner.probe(target, settings.refresh, job).await
    }
}

#[cfg(test)]
mod tests;
