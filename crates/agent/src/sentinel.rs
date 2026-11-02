//! Mode sentinelle : surveiller que le serveur répond, et alerter soi-même quand
//! ce n'est plus le cas.
//!
//! Le serveur désigne la sentinelle dans l'accusé de réception de ses lots
//! (adresses HTTPS, période, seuil, un canal de notification). Ensuite l'agent
//! n'a plus besoin de lui : il sonde en sortie, décide seul, et notifie par son
//! canal (webhook HTTPS ou ntfy) sans rien demander au serveur — celui-là même
//! qui est tombé. Aucune connexion entrante n'est ouverte.
//!
//! La décision ([`Decider`]) est séparée des entrées-sorties : elle ne connaît
//! ni le réseau ni l'horloge, elle reçoit des résultats et un instant.

use std::collections::{BTreeMap, VecDeque};
use std::time::Duration;

use dumbmonit_proto::{
    SENTINEL_MAX_INTERVAL_SECS, SENTINEL_MAX_THRESHOLD, SENTINEL_MAX_URLS,
    SENTINEL_MIN_INTERVAL_SECS, SentinelChannel, SentinelSetting, https_authority,
};
use tokio::task::{JoinHandle, JoinSet};
use tokio::time::Instant;
use tracing::{info, warn};

use crate::client::PushClient;
use crate::shutdown::Shutdown;

/// Délai d'une sonde : une réponse plus lente vaut un échec.
const PROBE_TIMEOUT: Duration = Duration::from_secs(10);
/// Délai d'envoi d'une notification.
const NOTIFY_TIMEOUT: Duration = Duration::from_secs(15);

/// Rappel tant que la panne dure.
const REMINDER: Duration = Duration::from_secs(3_600);
/// Attente avant de retenter un rappel que l'envoi a manqué.
const RETRY_AFTER: Duration = Duration::from_secs(300);
/// Notifications au plus par heure, rétablissement compris : un lien qui
/// clignote ne doit pas inonder le canal.
const MAX_PER_HOUR: usize = 6;
const RATE_WINDOW: Duration = Duration::from_secs(3_600);

// --------------------------------------------------------------------------
// Décision
// --------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeKind {
    /// Le seuil vient d'être atteint.
    Down,
    /// La panne dure toujours.
    Reminder,
    /// Plus aucune adresse n'est en panne.
    Recovered,
}

/// Une notification à envoyer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub kind: NoticeKind,
    /// Instant de la décision (relatif au démarrage de la tâche).
    pub at: Duration,
    /// Adresses en panne (vide pour un rétablissement).
    pub down: Vec<String>,
    /// Durée de la panne, pour un rétablissement.
    pub outage: Option<Duration>,
}

/// Décide quand notifier : seuil d'échecs consécutifs, rappel espacé,
/// rétablissement, limite de débit.
pub struct Decider {
    threshold: u32,
    /// Échecs consécutifs de chaque adresse.
    failures: BTreeMap<String, u32>,
    /// Une alerte de panne est partie (et pas encore son rétablissement).
    announced: bool,
    down_since: Option<Duration>,
    last_notice: Option<Duration>,
    /// Instants des notifications récentes, pour la limite de débit.
    recent: VecDeque<Duration>,
}

impl Decider {
    pub fn new(threshold: u32) -> Self {
        Self {
            threshold: threshold.max(1),
            failures: BTreeMap::new(),
            announced: false,
            down_since: None,
            last_notice: None,
            recent: VecDeque::new(),
        }
    }

    /// Prend les résultats d'un tour de sondage (`true` : l'adresse a répondu)
    /// et rend la notification à envoyer, s'il y en a une.
    pub fn observe(&mut self, now: Duration, results: &[(String, bool)]) -> Option<Notice> {
        // Une adresse retirée du réglage ne compte plus.
        self.failures.retain(|url, _| results.iter().any(|(u, _)| u == url));
        for (url, alive) in results {
            let count = self.failures.entry(url.clone()).or_insert(0);
            *count = if *alive { 0 } else { count.saturating_add(1) };
        }
        let down: Vec<String> = self
            .failures
            .iter()
            .filter(|(_, count)| **count >= self.threshold)
            .map(|(url, _)| url.clone())
            .collect();

        let kind = if !down.is_empty() {
            if !self.announced {
                NoticeKind::Down
            } else if self.last_notice.is_some_and(|at| now.saturating_sub(at) >= REMINDER) {
                NoticeKind::Reminder
            } else {
                return None;
            }
        } else if self.announced {
            NoticeKind::Recovered
        } else {
            return None;
        };

        if !self.may_send(now) {
            // L'état ne bouge pas : la notification sera retentée au tour suivant.
            return None;
        }
        self.recent.push_back(now);
        self.last_notice = Some(now);
        let outage = match kind {
            NoticeKind::Down => {
                self.announced = true;
                self.down_since = Some(now);
                None
            }
            NoticeKind::Reminder => None,
            NoticeKind::Recovered => {
                self.announced = false;
                self.down_since.take().map(|since| now.saturating_sub(since))
            }
        };
        Some(Notice { kind, at: now, down, outage })
    }

    /// L'envoi a échoué : remettre l'état comme si la notification n'était pas
    /// partie, pour la retenter (la limite de débit continue de compter).
    pub fn failed(&mut self, notice: &Notice) {
        match notice.kind {
            NoticeKind::Down => {
                self.announced = false;
                self.down_since = None;
            }
            NoticeKind::Recovered => {
                self.announced = true;
                self.down_since = notice.at.checked_sub(notice.outage.unwrap_or_default());
            }
            // Rappel manqué : nouvel essai dans `RETRY_AFTER`, pas à chaque tour.
            NoticeKind::Reminder => {
                self.last_notice = Some((notice.at + RETRY_AFTER).saturating_sub(REMINDER));
            }
        }
    }

    fn may_send(&mut self, now: Duration) -> bool {
        while self.recent.front().is_some_and(|at| now.saturating_sub(*at) >= RATE_WINDOW) {
            self.recent.pop_front();
        }
        self.recent.len() < MAX_PER_HOUR
    }
}

// --------------------------------------------------------------------------
// Notification
// --------------------------------------------------------------------------

/// Titre et texte d'une notification.
fn wording(notice: &Notice, host: &str) -> (String, String) {
    match notice.kind {
        NoticeKind::Down | NoticeKind::Reminder => {
            let list = notice.down.join(", ");
            let title = match notice.kind {
                NoticeKind::Down => "DumbMonit is not answering",
                _ => "DumbMonit is still not answering",
            };
            (title.to_string(), format!("Sentinel {host} cannot reach: {list}"))
        }
        NoticeKind::Recovered => {
            let after = notice.outage.map(|d| format!(" after {} min", d.as_secs() / 60));
            (
                "DumbMonit answers again".to_string(),
                format!("Sentinel {host} reaches DumbMonit again{}.", after.unwrap_or_default()),
            )
        }
    }
}

/// Requête de notification, prête à partir.
#[derive(Debug, PartialEq)]
pub struct Outgoing {
    pub url: String,
    pub headers: Vec<(&'static str, String)>,
    pub body: String,
}

/// Construit la requête du canal pour une notification.
pub fn outgoing(channel: &SentinelChannel, notice: &Notice, host: &str) -> Outgoing {
    let (title, message) = wording(notice, host);
    match channel {
        SentinelChannel::Webhook { url } => {
            let event = match notice.kind {
                NoticeKind::Down => "down",
                NoticeKind::Reminder => "reminder",
                NoticeKind::Recovered => "recovered",
            };
            let body = serde_json::json!({
                "source": "dumbmonit-sentinel",
                "event": event,
                "title": title,
                "message": message,
                "host": host,
                "urls": notice.down,
            });
            Outgoing {
                url: url.clone(),
                headers: vec![("content-type", "application/json".to_string())],
                body: body.to_string(),
            }
        }
        SentinelChannel::Ntfy { server, topic } => {
            let (priority, tags) = match notice.kind {
                NoticeKind::Recovered => ("default", "white_check_mark"),
                _ => ("high", "rotating_light"),
            };
            Outgoing {
                url: format!("{}/{topic}", server.trim_end_matches('/')),
                headers: vec![
                    ("title", title),
                    ("priority", priority.to_string()),
                    ("tags", tags.to_string()),
                ],
                body: message,
            }
        }
    }
}

async fn deliver(
    http: &reqwest::Client,
    channel: &SentinelChannel,
    notice: &Notice,
    host: &str,
) -> Result<(), String> {
    let request = outgoing(channel, notice, host);
    let mut builder = http.post(&request.url).timeout(NOTIFY_TIMEOUT).body(request.body);
    for (name, value) in request.headers {
        builder = builder.header(name, value);
    }
    // Jamais l'adresse dans l'erreur : pour un webhook, elle est le secret.
    let response = builder.send().await.map_err(|e| {
        if e.is_timeout() { "timeout".to_string() } else { "connection failed".to_string() }
    })?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("refused with HTTP {}", response.status().as_u16()))
    }
}

// --------------------------------------------------------------------------
// Réglage reçu : vérification
// --------------------------------------------------------------------------

/// Réglage nettoyé, ou la raison pour laquelle il n'est pas appliqué. Le serveur
/// a déjà validé : l'agent ne se fie pas pour autant à ce qu'il reçoit.
fn checked(setting: &SentinelSetting) -> Result<SentinelSetting, &'static str> {
    let Some(channel) = &setting.channel else {
        return Err("no notification channel (the server sends it on the encrypted channel only)");
    };
    let channel_ok = match channel {
        SentinelChannel::Webhook { url } => https_authority(url).is_some(),
        SentinelChannel::Ntfy { server, topic } => {
            https_authority(server).is_some()
                && !topic.is_empty()
                && topic.len() <= 64
                && topic.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        }
    };
    if !channel_ok {
        return Err("invalid notification channel");
    }
    let mut urls: Vec<String> = Vec::new();
    for url in &setting.urls {
        if https_authority(url).is_some() && !urls.contains(url) {
            urls.push(url.clone());
        }
    }
    urls.truncate(SENTINEL_MAX_URLS);
    if urls.is_empty() {
        return Err("no valid HTTPS address to watch");
    }
    Ok(SentinelSetting {
        enabled: true,
        urls,
        interval_secs: setting
            .interval_secs
            .clamp(SENTINEL_MIN_INTERVAL_SECS, SENTINEL_MAX_INTERVAL_SECS),
        threshold: setting.threshold.clamp(1, SENTINEL_MAX_THRESHOLD),
        channel: Some(channel.clone()),
    })
}

// --------------------------------------------------------------------------
// Mise en œuvre à chaud
// --------------------------------------------------------------------------

/// Tient la tâche de la sentinelle et la relance quand le réglage change.
pub struct SentinelRunner {
    client: PushClient,
    host: String,
    current: Option<SentinelSetting>,
    /// Dernier refus journalisé, pour ne le dire qu'une fois.
    refused: Option<&'static str>,
    task: Option<JoinHandle<()>>,
}

impl SentinelRunner {
    pub fn new(client: PushClient, host: String) -> Self {
        Self { client, host, current: None, refused: None, task: None }
    }

    /// Applique le réglage de l'accusé de réception. `None` ou désactivé :
    /// la sentinelle s'arrête.
    pub fn apply(&mut self, setting: Option<SentinelSetting>, shutdown: &Shutdown) {
        if self.task.as_ref().is_some_and(JoinHandle::is_finished) {
            self.task = None;
        }
        let wanted = match setting.filter(|s| s.enabled).map(|s| checked(&s)) {
            Some(Ok(setting)) => {
                self.refused = None;
                Some(setting)
            }
            Some(Err(reason)) => {
                if self.refused != Some(reason) {
                    warn!(reason, "sentinel not started");
                    self.refused = Some(reason);
                }
                None
            }
            None => None,
        };
        if wanted == self.current && (wanted.is_none() || self.task.is_some()) {
            return;
        }
        if let Some(task) = self.task.take() {
            task.abort();
            info!("sentinel stopped");
        }
        self.current = wanted.clone();
        if let Some(setting) = wanted {
            info!(
                urls = setting.urls.len(),
                interval_s = setting.interval_secs,
                threshold = setting.threshold,
                "sentinel switched on from the server"
            );
            self.task = Some(tokio::spawn(watch(
                setting,
                self.client.clone(),
                self.host.clone(),
                shutdown.clone(),
            )));
        }
    }
}

/// Sonde une adresse : toute réponse HTTP en dessous de 500 prouve que le
/// service est là (une page de connexion ou un 404 du port des agents aussi).
async fn probe(http: &reqwest::Client, url: &str) -> bool {
    match http.get(url).timeout(PROBE_TIMEOUT).send().await {
        Ok(response) => response.status().as_u16() < 500,
        Err(_) => false,
    }
}

async fn watch(setting: SentinelSetting, client: PushClient, host: String, mut shutdown: Shutdown) {
    let Some(channel) = setting.channel.clone() else { return };
    // Les notifications partent vers des services publics : autorités du
    // système, pas l'autorité épinglée du canal des agents.
    let Ok(notifier) = reqwest::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(concat!("dumbmonit-sentinel/", env!("CARGO_PKG_VERSION")))
        .build()
    else {
        warn!("sentinel: cannot build the notification client");
        return;
    };
    let started = Instant::now();
    let mut decider = Decider::new(setting.threshold);
    loop {
        // Le client est relu à chaque tour : le canal peut avoir changé
        // (certificat renouvelé) depuis le tour précédent.
        let http = client.probe_http();
        let mut probes = JoinSet::new();
        for url in &setting.urls {
            let (http, url) = (http.clone(), url.clone());
            probes.spawn(async move {
                let alive = probe(&http, &url).await;
                (url, alive)
            });
        }
        let mut results = Vec::with_capacity(setting.urls.len());
        while let Some(done) = probes.join_next().await {
            if let Ok(result) = done {
                results.push(result);
            }
        }
        if let Some(notice) = decider.observe(started.elapsed(), &results) {
            info!(kind = ?notice.kind, down = notice.down.len(), "sentinel notifies");
            if let Err(reason) = deliver(&notifier, &channel, &notice, &host).await {
                warn!(reason, "sentinel notification not delivered, will retry");
                decider.failed(&notice);
            }
        }
        tokio::select! {
            biased;
            _ = shutdown.wait() => break,
            _ = tokio::time::sleep(Duration::from_secs(setting.interval_secs)) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: u64 = 60;

    fn at(minutes: u64) -> Duration {
        Duration::from_secs(minutes * MIN)
    }

    fn results(alive: bool) -> Vec<(String, bool)> {
        vec![("https://a.example.org".to_string(), alive)]
    }

    #[test]
    fn nothing_is_sent_before_the_threshold() {
        let mut decider = Decider::new(3);
        assert!(decider.observe(at(0), &results(false)).is_none());
        assert!(decider.observe(at(1), &results(false)).is_none());
        let notice = decider.observe(at(2), &results(false)).expect("third failure");
        assert_eq!(notice.kind, NoticeKind::Down);
        assert_eq!(notice.down, vec!["https://a.example.org".to_string()]);
    }

    #[test]
    fn a_single_answer_resets_the_count() {
        let mut decider = Decider::new(3);
        decider.observe(at(0), &results(false));
        decider.observe(at(1), &results(false));
        assert!(decider.observe(at(2), &results(true)).is_none());
        assert!(decider.observe(at(3), &results(false)).is_none());
        assert!(decider.observe(at(4), &results(false)).is_none());
    }

    #[test]
    fn the_alert_is_sent_once_then_a_spaced_reminder() {
        let mut decider = Decider::new(1);
        assert_eq!(decider.observe(at(0), &results(false)).unwrap().kind, NoticeKind::Down);
        for minute in 1..60 {
            assert!(decider.observe(at(minute), &results(false)).is_none(), "minute {minute}");
        }
        let reminder = decider.observe(at(60), &results(false)).expect("reminder after an hour");
        assert_eq!(reminder.kind, NoticeKind::Reminder);
        assert!(decider.observe(at(61), &results(false)).is_none());
    }

    #[test]
    fn recovery_is_announced_once_with_the_outage_length() {
        let mut decider = Decider::new(1);
        decider.observe(at(0), &results(false));
        let notice = decider.observe(at(7), &results(true)).expect("recovery");
        assert_eq!(notice.kind, NoticeKind::Recovered);
        assert_eq!(notice.outage, Some(at(7)));
        assert!(decider.observe(at(8), &results(true)).is_none());
        // Une nouvelle panne repart de zéro.
        assert_eq!(decider.observe(at(9), &results(false)).unwrap().kind, NoticeKind::Down);
    }

    #[test]
    fn no_recovery_is_announced_for_a_failure_below_the_threshold() {
        let mut decider = Decider::new(3);
        decider.observe(at(0), &results(false));
        assert!(decider.observe(at(1), &results(true)).is_none());
    }

    #[test]
    fn the_rate_limit_holds_a_flapping_link_back() {
        let mut decider = Decider::new(1);
        let mut sent = 0;
        // Panne et rétablissement à chaque minute pendant une heure.
        for minute in 0..60 {
            if decider.observe(at(minute), &results(minute % 2 == 1)).is_some() {
                sent += 1;
            }
        }
        assert_eq!(sent, MAX_PER_HOUR);
        // Une heure plus tard, la fenêtre est libre : la notification retenue part.
        assert!(decider.observe(at(121), &results(false)).is_some());
    }

    #[test]
    fn a_failed_delivery_is_retried_on_the_next_round() {
        let mut decider = Decider::new(1);
        let notice = decider.observe(at(0), &results(false)).unwrap();
        decider.failed(&notice);
        let again = decider.observe(at(1), &results(false)).expect("retry");
        assert_eq!(again.kind, NoticeKind::Down);

        // Un rétablissement manqué est retenté aussi, avec la même durée de panne.
        let recovered = decider.observe(at(5), &results(true)).unwrap();
        decider.failed(&recovered);
        let retry = decider.observe(at(6), &results(true)).expect("retry of the recovery");
        assert_eq!(retry.kind, NoticeKind::Recovered);
        assert_eq!(retry.outage, Some(at(5)));
    }

    #[test]
    fn a_missed_reminder_is_retried_after_a_few_minutes_not_every_round() {
        let mut decider = Decider::new(1);
        decider.observe(at(0), &results(false));
        let reminder = decider.observe(at(60), &results(false)).unwrap();
        decider.failed(&reminder);
        assert!(decider.observe(at(61), &results(false)).is_none());
        assert!(decider.observe(at(65), &results(false)).is_some());
    }

    #[test]
    fn any_address_down_is_enough_and_all_must_recover() {
        let mut decider = Decider::new(1);
        let both = |a: bool, b: bool| {
            vec![("https://a.example.org".to_string(), a), ("https://b.example.org".to_string(), b)]
        };
        let notice = decider.observe(at(0), &both(true, false)).unwrap();
        assert_eq!(notice.down, vec!["https://b.example.org".to_string()]);
        assert!(decider.observe(at(1), &both(false, false)).is_none());
        assert_eq!(decider.observe(at(2), &both(true, true)).unwrap().kind, NoticeKind::Recovered);
    }

    fn notice(kind: NoticeKind) -> Notice {
        Notice {
            kind,
            at: Duration::ZERO,
            down: vec!["https://a.example.org".into()],
            outage: Some(at(3)),
        }
    }

    #[test]
    fn the_webhook_gets_a_json_body() {
        let channel = SentinelChannel::Webhook { url: "https://hooks.example.org/x".into() };
        let out = outgoing(&channel, &notice(NoticeKind::Down), "nas");
        assert_eq!(out.url, "https://hooks.example.org/x");
        let body: serde_json::Value = serde_json::from_str(&out.body).unwrap();
        assert_eq!(body["event"], "down");
        assert_eq!(body["host"], "nas");
        assert_eq!(body["urls"][0], "https://a.example.org");
    }

    #[test]
    fn ntfy_gets_a_plain_text_post_on_the_topic() {
        let channel =
            SentinelChannel::Ntfy { server: "https://ntfy.sh/".into(), topic: "t".into() };
        let out = outgoing(&channel, &notice(NoticeKind::Recovered), "nas");
        assert_eq!(out.url, "https://ntfy.sh/t");
        assert!(out.body.contains("after 3 min"), "{}", out.body);
        assert!(out.headers.iter().any(|(k, v)| *k == "priority" && v == "default"));
    }

    #[test]
    fn a_received_setting_is_checked_again() {
        let good = SentinelSetting {
            enabled: true,
            urls: vec!["https://a.example.org".into(), "http://insecure.example.org".into()],
            interval_secs: 1,
            threshold: 0,
            channel: Some(SentinelChannel::Webhook { url: "https://hooks.example.org/x".into() }),
        };
        let cleaned = checked(&good).expect("valid");
        assert_eq!(cleaned.urls, vec!["https://a.example.org".to_string()]);
        assert_eq!(cleaned.interval_secs, SENTINEL_MIN_INTERVAL_SECS);
        assert_eq!(cleaned.threshold, 1);

        let no_channel = SentinelSetting { channel: None, ..good.clone() };
        assert!(checked(&no_channel).is_err());
        let plain_webhook = SentinelSetting {
            channel: Some(SentinelChannel::Webhook { url: "http://hooks.example.org".into() }),
            ..good.clone()
        };
        assert!(checked(&plain_webhook).is_err());
        let only_plain = SentinelSetting { urls: vec!["http://a.example.org".into()], ..good };
        assert!(checked(&only_plain).is_err());
    }
}
