//! Journal d'événements Windows : les événements qui répondent aux règles du
//! serveur.
//!
//! Le serveur envoie les règles de la machine dans l'accusé de réception de
//! chaque lot ([`dumbmonit_proto::PushAck::event_rules`]). À chaque cycle,
//! l'agent interroge le journal de chaque règle avec une requête XPath
//! (`*[System[Provider[@Name='…'] and (EventID=4625) and EventRecordID>N]]`),
//! filtre le message par le motif de la règle, limite le débit et joint ce
//! qu'il a trouvé à son lot suivant.
//!
//! Le dernier `EventRecordID` lu de chaque règle est gardé sur disque, à côté
//! du secret de liaison : un redémarrage reprend où il s'était arrêté — un
//! 41 Kernel-Power écrit au démarrage qui suit un plantage est bien remonté.
//! Une règle nouvelle, elle, part du dernier événement présent : jamais
//! l'historique du journal.
//!
//! Tout ce qui ne touche pas à l'API Windows (requête, lecture du XML rendu,
//! motif, limite de débit, positions) est ici et testé sur toutes les
//! plateformes ; la lecture elle-même (`wevtapi.dll`) est dans [`wevt`], sous
//! Windows seulement.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use dumbmonit_proto::{
    EVENT_RULE_MAX_PER_MINUTE, EventLevel, EventLogDropped, EventLogEntry, EventLogReport,
    EventLogRule, MAX_EVENT_LOG_ENTRIES, MAX_EVENT_MESSAGE_CHARS, truncate_message,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// Nom du fichier des positions, à côté du secret de liaison.
pub const STATE_FILE: &str = "eventlog-state.json";

/// Événements lus au plus par règle et par cycle. Le reste attend le cycle
/// suivant, à partir du dernier lu.
const MAX_SCAN_PER_CYCLE: usize = 200;

/// Durée de la fenêtre de la limite de débit.
const RATE_WINDOW_MS: i64 = 60_000;

/// Un événement tel que la source le rend : son XML (`EvtRenderEventXml`) et
/// son message mis en forme, quand le fournisseur sait le donner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawEvent {
    pub xml: String,
    pub message: Option<String>,
}

/// Accès au journal, derrière un trait pour que tout le reste se teste sans
/// Windows.
pub trait EventSource {
    /// `EventRecordID` du dernier événement du journal ; `None` s'il est vide.
    fn latest_record_id(&mut self, channel: &str) -> Result<Option<u64>, String>;
    /// Les événements qui répondent à la requête, du plus ancien au plus
    /// récent, au plus `max`.
    fn query(&mut self, channel: &str, xpath: &str, max: usize) -> Result<Vec<RawEvent>, String>;
}

/// Requête XPath d'une règle, au-delà d'un `EventRecordID` s'il est donné.
///
/// Le serveur a déjà écarté les fournisseurs qui casseraient la requête
/// (guillemets, crochets) ; ils sont écartés ici aussi, par prudence.
pub fn xpath(rule: &EventLogRule, after: Option<u64>) -> String {
    let mut conditions = Vec::new();
    if let Some(provider) = rule.provider.as_deref().map(str::trim).filter(|p| !p.is_empty()) {
        let safe: String =
            provider.chars().filter(|c| !matches!(c, '\'' | '"' | '[' | ']')).collect();
        conditions.push(format!("Provider[@Name='{safe}']"));
    }
    if !rule.event_ids.is_empty() {
        let ids: Vec<String> = rule
            .event_ids
            .iter()
            .map(|range| {
                if range.from == range.to {
                    format!("EventID={}", range.from)
                } else {
                    format!("(EventID>={} and EventID<={})", range.from, range.to)
                }
            })
            .collect();
        conditions.push(format!("({})", ids.join(" or ")));
    }
    if !rule.levels.is_empty() {
        let mut values: Vec<u8> =
            rule.levels.iter().flat_map(|level| level.windows_values().iter().copied()).collect();
        values.sort_unstable();
        values.dedup();
        let levels: Vec<String> = values.iter().map(|v| format!("Level={v}")).collect();
        conditions.push(format!("({})", levels.join(" or ")));
    }
    if let Some(after) = after {
        conditions.push(format!("EventRecordID>{after}"));
    }
    if conditions.is_empty() {
        "*".to_string()
    } else {
        format!("*[System[{}]]", conditions.join(" and "))
    }
}

/// Ce que l'agent retient d'un événement rendu en XML.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsedEvent {
    pub provider: String,
    pub event_id: u32,
    pub level: u8,
    pub record_id: u64,
    pub time_created_ms: i64,
    pub channel: String,
    pub computer: String,
    /// Les `<Data>` de `EventData`, « Nom: valeur », une par ligne : le
    /// message de repli quand le fournisseur ne sait pas en donner un.
    pub data: String,
}

/// Décode les entités XML d'un texte.
fn unescape(text: &str) -> String {
    if !text.contains('&') {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        let after = &rest[start..];
        let Some(end) = after.find(';').filter(|end| *end <= 10) else {
            out.push('&');
            rest = &after[1..];
            continue;
        };
        let entity = &after[1..end];
        let decoded = match entity {
            "lt" => Some('<'),
            "gt" => Some('>'),
            "amp" => Some('&'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => entity
                .strip_prefix("#x")
                .or_else(|| entity.strip_prefix("#X"))
                .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                .or_else(|| entity.strip_prefix('#').and_then(|dec| dec.parse().ok()))
                .and_then(char::from_u32),
        };
        match decoded {
            Some(c) => {
                out.push(c);
                rest = &after[end + 1..];
            }
            None => {
                out.push('&');
                rest = &after[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Le texte du premier élément `<name …>texte</name>`.
fn element<'a>(xml: &'a str, name: &str) -> Option<&'a str> {
    let open = format!("<{name}");
    let mut from = 0;
    while let Some(at) = xml[from..].find(&open) {
        let start = from + at + open.len();
        let next = xml[start..].chars().next()?;
        // `<EventID>` ne doit pas répondre à `<EventIDs>` ni à `<EventIDx …>`.
        if next == '>' || next.is_whitespace() || next == '/' {
            let close = start + xml[start..].find('>')?;
            if xml[..close].ends_with('/') {
                return Some("");
            }
            let end_tag = format!("</{name}>");
            let end = close + 1 + xml[close + 1..].find(&end_tag)?;
            return Some(&xml[close + 1..end]);
        }
        from = start;
    }
    None
}

/// La valeur d'un attribut du premier élément `<name …>`.
fn attribute(xml: &str, name: &str, attr: &str) -> Option<String> {
    let open = format!("<{name}");
    let mut from = 0;
    while let Some(at) = xml[from..].find(&open) {
        let start = from + at + open.len();
        let tag_end = start + xml[start..].find('>')?;
        let tag = &xml[start..tag_end];
        // L'élément lui-même, sans attribut : rien à chercher plus loin.
        if tag.is_empty() || tag.starts_with('/') {
            return None;
        }
        if tag.starts_with(|c: char| c.is_whitespace()) {
            for quote in ['\'', '"'] {
                // L'espace devant : `Name=` ne doit pas répondre à `EventSourceName=`.
                let needle = format!(" {attr}={quote}");
                if let Some(pos) = tag.find(&needle) {
                    let value = &tag[pos + needle.len()..];
                    let end = value.find(quote)?;
                    return Some(unescape(&value[..end]));
                }
            }
            return None;
        }
        from = start;
    }
    None
}

/// Les `<Data>` de `EventData`, une ligne chacune.
fn event_data(xml: &str) -> String {
    let Some(data) = element(xml, "EventData") else { return String::new() };
    let mut lines = Vec::new();
    let mut rest = data;
    while let Some(at) = rest.find("<Data") {
        let after = &rest[at..];
        let Some(close) = after.find('>') else { break };
        let tag = &after[..close];
        if tag.ends_with('/') {
            rest = &after[close + 1..];
            continue;
        }
        let body = &after[close + 1..];
        let Some(end) = body.find("</Data>") else { break };
        let value = unescape(body[..end].trim());
        if !value.is_empty() {
            match attribute(after, "Data", "Name") {
                Some(name) if !name.is_empty() => lines.push(format!("{name}: {value}")),
                _ => lines.push(value),
            }
        }
        rest = &body[end..];
    }
    lines.join("\n")
}

/// Lit le XML d'un événement rendu par `EvtRender`. `None` sans `EventID`.
pub fn parse_event(xml: &str) -> Option<ParsedEvent> {
    let system = element(xml, "System")?;
    let text = |name: &str| element(system, name).map(str::trim);
    let event_id: u32 = text("EventID")?.parse().ok()?;
    let time_created_ms = attribute(system, "TimeCreated", "SystemTime")
        .and_then(|raw| chrono::DateTime::parse_from_rfc3339(&raw).ok())
        .map(|at| at.timestamp_millis())
        .unwrap_or(0);
    Some(ParsedEvent {
        provider: attribute(system, "Provider", "Name").unwrap_or_default(),
        event_id,
        level: text("Level").and_then(|v| v.parse().ok()).unwrap_or(0),
        record_id: text("EventRecordID").and_then(|v| v.parse().ok()).unwrap_or(0),
        time_created_ms,
        channel: element(system, "Channel").map(unescape).unwrap_or_default(),
        computer: element(system, "Computer").map(unescape).unwrap_or_default(),
        data: event_data(xml),
    })
}

/// Le motif d'une règle, prêt à servir.
#[derive(Debug)]
enum Matcher {
    Any,
    /// Sous-chaîne, déjà en minuscules.
    Substring(String),
    Regex(regex::Regex),
    /// Expression que ce binaire ne sait pas compiler : la règle ne répond à
    /// rien plutôt qu'à tout.
    Invalid,
}

impl Matcher {
    fn new(rule: &EventLogRule) -> Self {
        match rule.pattern.as_deref().map(str::trim).filter(|p| !p.is_empty()) {
            None => Self::Any,
            Some(pattern) if rule.regex => match regex::Regex::new(pattern) {
                Ok(regex) => Self::Regex(regex),
                Err(error) => {
                    warn!(rule = rule.id, %error, "event log rule pattern refused");
                    Self::Invalid
                }
            },
            Some(pattern) => Self::Substring(pattern.to_lowercase()),
        }
    }

    fn matches(&self, message: &str) -> bool {
        match self {
            Self::Any => true,
            Self::Substring(needle) => message.to_lowercase().contains(needle.as_str()),
            Self::Regex(regex) => regex.is_match(message),
            Self::Invalid => false,
        }
    }
}

/// Limite de débit par règle : au plus [`EVENT_RULE_MAX_PER_MINUTE`]
/// événements par fenêtre d'une minute.
#[derive(Debug, Default)]
pub struct RateLimiter {
    windows: BTreeMap<i64, (i64, u32)>,
}

impl RateLimiter {
    /// Vrai si l'événement peut partir.
    pub fn allow(&mut self, rule: i64, now_ms: i64) -> bool {
        let (started, count) = self.windows.entry(rule).or_insert((now_ms, 0));
        if now_ms - *started >= RATE_WINDOW_MS || now_ms < *started {
            *started = now_ms;
            *count = 0;
        }
        if *count >= EVENT_RULE_MAX_PER_MINUTE {
            return false;
        }
        *count += 1;
        true
    }

    fn retain(&mut self, keep: impl Fn(i64) -> bool) {
        self.windows.retain(|rule, _| keep(*rule));
    }
}

/// Où en est la lecture d'une règle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Position {
    channel: String,
    record_id: u64,
}

/// Ce qui est gardé sur disque.
#[derive(Debug, Default, Serialize, Deserialize)]
struct SavedState {
    #[serde(default)]
    positions: BTreeMap<i64, Position>,
}

struct Compiled {
    rule: EventLogRule,
    matcher: Matcher,
}

pub struct EventLogProbe<S: EventSource> {
    source: S,
    rules: Vec<Compiled>,
    positions: BTreeMap<i64, Position>,
    limiter: RateLimiter,
    /// Événements en attente d'un lot accepté, du plus ancien au plus récent.
    pending: Vec<EventLogEntry>,
    /// Écartés (limite de débit, attente trop longue) depuis le dernier lot
    /// accepté, par règle.
    dropped: BTreeMap<i64, u64>,
    state_path: Option<PathBuf>,
    dirty: bool,
    /// Échecs déjà signalés (règle, raison) : un avertissement par panne.
    announced: BTreeSet<(i64, String)>,
}

/// La source de la plateforme : `wevtapi.dll` sous Windows, rien ailleurs.
pub type PlatformProbe = EventLogProbe<PlatformSource>;

impl PlatformProbe {
    pub fn for_platform(state_path: PathBuf) -> Self {
        EventLogProbe::new(PlatformSource::new(), Some(state_path))
    }
}

impl<S: EventSource> EventLogProbe<S> {
    pub fn new(source: S, state_path: Option<PathBuf>) -> Self {
        let positions = state_path.as_deref().map(load).unwrap_or_default();
        Self {
            source,
            rules: Vec::new(),
            positions,
            limiter: RateLimiter::default(),
            pending: Vec::new(),
            dropped: BTreeMap::new(),
            state_path,
            dirty: false,
            announced: BTreeSet::new(),
        }
    }

    /// Adopte les règles que le serveur vient d'envoyer.
    pub fn set_rules(&mut self, rules: Vec<EventLogRule>) {
        let current: Vec<&EventLogRule> = self.rules.iter().map(|c| &c.rule).collect();
        if current.len() == rules.len() && current.iter().zip(&rules).all(|(a, b)| *a == b) {
            return;
        }
        info!(rules = rules.len(), "event log rules received from the server");
        let ids: BTreeSet<i64> = rules.iter().map(|rule| rule.id).collect();
        let before = self.positions.len();
        self.positions.retain(|id, _| ids.contains(id));
        self.dirty |= self.positions.len() != before;
        self.limiter.retain(|id| ids.contains(&id));
        self.announced.retain(|(id, _)| ids.contains(id));
        self.rules =
            rules.into_iter().map(|rule| Compiled { matcher: Matcher::new(&rule), rule }).collect();
        self.save();
    }

    /// Un cycle : interroge le journal de chaque règle.
    pub fn read(&mut self, now_ms: i64) {
        for index in 0..self.rules.len() {
            self.poll(index, now_ms);
        }
        self.save();
    }

    fn note_failure(&mut self, rule: i64, channel: &str, reason: String) {
        if self.announced.insert((rule, reason.clone())) {
            warn!(rule, channel, reason, "event log not readable");
        } else {
            debug!(rule, channel, reason, "event log still not readable");
        }
    }

    fn poll(&mut self, index: usize, now_ms: i64) {
        let rule = self.rules[index].rule.clone();
        let latest = match self.source.latest_record_id(&rule.channel) {
            Ok(latest) => latest.unwrap_or(0),
            Err(reason) => {
                self.note_failure(rule.id, &rule.channel, reason);
                return;
            }
        };
        let stored = self
            .positions
            .get(&rule.id)
            .filter(|position| position.channel == rule.channel)
            .map(|position| position.record_id);
        let after = match stored {
            // Règle nouvelle (ou journal changé) : seulement ce qui viendra.
            None => {
                self.announced.retain(|(id, _)| *id != rule.id);
                self.set_position(rule.id, &rule.channel, latest);
                return;
            }
            // Journal vidé : les numéros repartent de 1.
            Some(stored) if latest < stored => 0,
            Some(stored) if latest == stored => return,
            Some(stored) => stored,
        };
        let query = xpath(&rule, Some(after));
        let events = match self.source.query(&rule.channel, &query, MAX_SCAN_PER_CYCLE) {
            Ok(events) => events,
            Err(reason) => {
                self.note_failure(rule.id, &rule.channel, reason);
                return;
            }
        };
        self.announced.retain(|(id, _)| *id != rule.id);
        // Moins que le plafond : tout ce qui répond a été lu, la position peut
        // sauter au dernier événement du journal, même s'il ne répond pas.
        let complete = events.len() < MAX_SCAN_PER_CYCLE;
        let mut last = after;
        for raw in events {
            let Some(parsed) = parse_event(&raw.xml) else { continue };
            last = last.max(parsed.record_id);
            let message = raw
                .message
                .map(|m| m.trim().to_string())
                .filter(|m| !m.is_empty())
                .unwrap_or_else(|| parsed.data.clone());
            if !self.rules[index].matcher.matches(&message) {
                continue;
            }
            if !self.limiter.allow(rule.id, now_ms) {
                *self.dropped.entry(rule.id).or_default() += 1;
                continue;
            }
            self.push(EventLogEntry {
                rule_id: rule.id,
                channel: if parsed.channel.is_empty() {
                    rule.channel.clone()
                } else {
                    parsed.channel
                },
                provider: parsed.provider,
                event_id: parsed.event_id,
                level: parsed.level,
                record_id: parsed.record_id,
                time_created_ms: parsed.time_created_ms,
                computer: parsed.computer,
                message: truncate_message(&message, MAX_EVENT_MESSAGE_CHARS),
            });
        }
        let last = if complete { last.max(latest) } else { last };
        self.set_position(rule.id, &rule.channel, last);
    }

    fn push(&mut self, entry: EventLogEntry) {
        if self.pending.len() >= MAX_EVENT_LOG_ENTRIES {
            // Serveur injoignable depuis longtemps : le plus ancien cède la place.
            let oldest = self.pending.remove(0);
            *self.dropped.entry(oldest.rule_id).or_default() += 1;
        }
        self.pending.push(entry);
    }

    fn set_position(&mut self, rule: i64, channel: &str, record_id: u64) {
        let position = Position { channel: channel.to_string(), record_id };
        if self.positions.get(&rule) != Some(&position) {
            self.positions.insert(rule, position);
            self.dirty = true;
        }
    }

    fn save(&mut self) {
        if !self.dirty {
            return;
        }
        self.dirty = false;
        let Some(path) = &self.state_path else { return };
        let state = SavedState { positions: self.positions.clone() };
        let saved = serde_json::to_string(&state)
            .map_err(anyhow::Error::from)
            .and_then(|text| crate::binding::write_private(path, &text));
        if let Err(error) = saved {
            warn!(path = %path.display(), %error, "event log positions not saved");
        }
    }

    /// Ce qu'il faut joindre au prochain lot ; `None` quand il n'y a rien.
    pub fn report(&self) -> Option<EventLogReport> {
        let report = EventLogReport {
            events: self.pending.clone(),
            dropped: self
                .dropped
                .iter()
                .map(|(rule_id, count)| EventLogDropped { rule_id: *rule_id, count: *count })
                .collect(),
        };
        (!report.is_empty()).then_some(report)
    }

    /// Le serveur a accepté (ou définitivement refusé) ce rapport : ce qu'il
    /// contenait est oublié, ce qui est arrivé depuis reste en attente.
    pub fn acknowledge(&mut self, sent: Option<&EventLogReport>) {
        let Some(sent) = sent else { return };
        let count = sent.events.len().min(self.pending.len());
        let still_there = self.pending[..count] == sent.events[..count];
        if still_there {
            self.pending.drain(..count);
        } else {
            // Des événements ont été poussés hors de la file entre-temps : ce
            // qui a été envoyé n'est plus repérable, on retire ce qui est en tête.
            self.pending.retain(|entry| !sent.events.contains(entry));
        }
        for dropped in &sent.dropped {
            if let Some(count) = self.dropped.get_mut(&dropped.rule_id) {
                *count = count.saturating_sub(dropped.count);
                if *count == 0 {
                    self.dropped.remove(&dropped.rule_id);
                }
            }
        }
    }
}

/// Positions gardées sur disque ; rien si le fichier manque ou se lit mal.
fn load(path: &std::path::Path) -> BTreeMap<i64, Position> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return BTreeMap::new(),
        Err(error) => {
            warn!(path = %path.display(), %error, "event log positions unreadable");
            return BTreeMap::new();
        }
    };
    match serde_json::from_str::<SavedState>(&text) {
        Ok(state) => state.positions,
        Err(error) => {
            warn!(path = %path.display(), %error, "event log positions unreadable");
            BTreeMap::new()
        }
    }
}

/// Le niveau lisible d'une valeur de `System/Level`, pour les journaux.
#[cfg_attr(not(windows), allow(dead_code))]
fn level_name(level: u8) -> &'static str {
    EventLevel::from_windows(level).map(EventLevel::as_str).unwrap_or("verbose")
}

// ----------------------------------------------------- source de la plateforme

#[cfg(not(windows))]
#[derive(Default)]
pub struct PlatformSource;

#[cfg(not(windows))]
impl PlatformSource {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(windows))]
impl EventSource for PlatformSource {
    fn latest_record_id(&mut self, _channel: &str) -> Result<Option<u64>, String> {
        Err("the event log exists only on Windows".to_string())
    }

    fn query(
        &mut self,
        _channel: &str,
        _xpath: &str,
        _max: usize,
    ) -> Result<Vec<RawEvent>, String> {
        Err("the event log exists only on Windows".to_string())
    }
}

#[cfg(windows)]
pub use wevt::WevtSource as PlatformSource;

#[cfg(windows)]
mod wevt {
    //! Lecture par l'API Windows Event Log (`wevtapi.dll`) : `EvtQuery`,
    //! `EvtNext`, `EvtRender` (XML) et `EvtFormatMessage` (message lisible,
    //! dans la langue du système).

    use std::collections::HashMap;
    use std::ptr;

    use windows_sys::Win32::Foundation::{
        ERROR_ACCESS_DENIED, ERROR_EVT_CHANNEL_NOT_FOUND, ERROR_EVT_INVALID_QUERY,
        ERROR_EVT_UNRESOLVED_VALUE_INSERT, ERROR_INSUFFICIENT_BUFFER, ERROR_NO_MORE_ITEMS,
        GetLastError,
    };
    use windows_sys::Win32::System::EventLog::{
        EVT_HANDLE, EvtClose, EvtFormatMessage, EvtFormatMessageEvent, EvtNext,
        EvtOpenPublisherMetadata, EvtQuery, EvtQueryChannelPath, EvtQueryForwardDirection,
        EvtQueryReverseDirection, EvtRender, EvtRenderEventXml,
    };

    use super::{EventSource, RawEvent};

    /// Fournisseurs gardés ouverts au plus ; au-delà, le cache repart de zéro.
    const MAX_PUBLISHERS: usize = 64;

    /// Délai d'attente de `EvtNext`, en millisecondes.
    const NEXT_TIMEOUT_MS: u32 = 1_000;

    /// Une poignée du journal, fermée à la sortie de portée.
    struct Handle(EVT_HANDLE);

    impl Drop for Handle {
        fn drop(&mut self) {
            if self.0 != 0 {
                // SAFETY : poignée rendue par l'API et fermée une seule fois, ici.
                unsafe { EvtClose(self.0) };
            }
        }
    }

    #[derive(Default)]
    pub struct WevtSource {
        /// Métadonnées de chaque fournisseur, pour `EvtFormatMessage`. `None` :
        /// le fournisseur n'en a pas (le message se rabat sur `EventData`).
        publishers: HashMap<String, Option<Handle>>,
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn last_error() -> u32 {
        // SAFETY : aucune précondition.
        unsafe { GetLastError() }
    }

    fn describe(error: u32) -> String {
        match error {
            ERROR_EVT_CHANNEL_NOT_FOUND => "no such event log on this machine".to_string(),
            ERROR_ACCESS_DENIED => "access denied: the Security log needs the agent to run as \
                                    LocalSystem or in the Event Log Readers group"
                .to_string(),
            ERROR_EVT_INVALID_QUERY => "the event log refused the query".to_string(),
            other => format!("Windows error {other}"),
        }
    }

    fn open(channel: &str, xpath: &str, direction: u32) -> Result<Handle, String> {
        let channel = wide(channel);
        let xpath = wide(xpath);
        // SAFETY : chaînes terminées par un zéro, vivantes jusqu'au retour ;
        // session 0 : la machine locale.
        let handle = unsafe {
            EvtQuery(0, channel.as_ptr(), xpath.as_ptr(), EvtQueryChannelPath | direction)
        };
        if handle == 0 { Err(describe(last_error())) } else { Ok(Handle(handle)) }
    }

    fn next(results: &Handle, max: usize) -> Result<Vec<Handle>, String> {
        let mut out = Vec::new();
        let mut batch: [EVT_HANDLE; 16] = [0; 16];
        while out.len() < max {
            let wanted = (max - out.len()).min(batch.len()) as u32;
            let mut returned: u32 = 0;
            // SAFETY : `batch` peut recevoir `wanted` poignées ; `returned`
            // est un emplacement valide.
            let ok = unsafe {
                EvtNext(results.0, wanted, batch.as_mut_ptr(), NEXT_TIMEOUT_MS, 0, &mut returned)
            };
            if ok == 0 {
                let error = last_error();
                if error == ERROR_NO_MORE_ITEMS || !out.is_empty() {
                    break;
                }
                return Err(describe(error));
            }
            let returned = (returned as usize).min(batch.len());
            out.extend(batch[..returned].iter().map(|handle| Handle(*handle)));
            if returned == 0 {
                break;
            }
        }
        Ok(out)
    }

    fn render_xml(event: &Handle) -> Option<String> {
        let mut used: u32 = 0;
        let mut properties: u32 = 0;
        // SAFETY : premier appel sans tampon, qui rend la taille requise
        // (en octets) dans `used`.
        let ok = unsafe {
            EvtRender(0, event.0, EvtRenderEventXml, 0, ptr::null_mut(), &mut used, &mut properties)
        };
        if ok == 0 && last_error() != ERROR_INSUFFICIENT_BUFFER {
            return None;
        }
        let mut buffer: Vec<u16> = vec![0; (used as usize).div_ceil(2)];
        // SAFETY : `buffer` offre `buffer.len() * 2` octets, alignés pour des
        // `u16`.
        let ok = unsafe {
            EvtRender(
                0,
                event.0,
                EvtRenderEventXml,
                (buffer.len() * 2) as u32,
                buffer.as_mut_ptr().cast(),
                &mut used,
                &mut properties,
            )
        };
        if ok == 0 {
            return None;
        }
        let len = (used as usize / 2).min(buffer.len());
        Some(String::from_utf16_lossy(&buffer[..len]).trim_end_matches('\0').to_string())
    }

    fn format_message(publisher: EVT_HANDLE, event: &Handle) -> Option<String> {
        let mut used: u32 = 0;
        // SAFETY : premier appel sans tampon, qui rend la taille requise (en
        // caractères) dans `used`.
        let ok = unsafe {
            EvtFormatMessage(
                publisher,
                event.0,
                0,
                0,
                ptr::null(),
                EvtFormatMessageEvent,
                0,
                ptr::null_mut(),
                &mut used,
            )
        };
        if ok == 0 && last_error() != ERROR_INSUFFICIENT_BUFFER {
            return None;
        }
        if used == 0 {
            return None;
        }
        let mut buffer: Vec<u16> = vec![0; used as usize];
        // SAFETY : `buffer` offre `used` caractères.
        let ok = unsafe {
            EvtFormatMessage(
                publisher,
                event.0,
                0,
                0,
                ptr::null(),
                EvtFormatMessageEvent,
                buffer.len() as u32,
                buffer.as_mut_ptr(),
                &mut used,
            )
        };
        // Une insertion non résolue laisse tout de même un message lisible.
        if ok == 0 && last_error() != ERROR_EVT_UNRESOLVED_VALUE_INSERT {
            return None;
        }
        let len = (used as usize).min(buffer.len());
        let text = String::from_utf16_lossy(&buffer[..len]);
        let text = text.trim_end_matches('\0').trim();
        (!text.is_empty()).then(|| text.to_string())
    }

    impl WevtSource {
        pub fn new() -> Self {
            Self::default()
        }

        fn publisher(&mut self, provider: &str) -> EVT_HANDLE {
            if provider.is_empty() {
                return 0;
            }
            if self.publishers.len() >= MAX_PUBLISHERS && !self.publishers.contains_key(provider) {
                self.publishers.clear();
            }
            let handle = self.publishers.entry(provider.to_string()).or_insert_with(|| {
                let name = wide(provider);
                // SAFETY : nom terminé par un zéro ; session 0, pas de
                // fichier journal, langue du système.
                let handle =
                    unsafe { EvtOpenPublisherMetadata(0, name.as_ptr(), ptr::null(), 0, 0) };
                (handle != 0).then_some(Handle(handle))
            });
            handle.as_ref().map_or(0, |handle| handle.0)
        }
    }

    impl EventSource for WevtSource {
        fn latest_record_id(&mut self, channel: &str) -> Result<Option<u64>, String> {
            let results = open(channel, "*", EvtQueryReverseDirection)?;
            let events = next(&results, 1)?;
            Ok(events
                .first()
                .and_then(render_xml)
                .and_then(|xml| super::parse_event(&xml))
                .map(|event| event.record_id))
        }

        fn query(
            &mut self,
            channel: &str,
            xpath: &str,
            max: usize,
        ) -> Result<Vec<RawEvent>, String> {
            let results = open(channel, xpath, EvtQueryForwardDirection)?;
            let events = next(&results, max)?;
            let mut out = Vec::with_capacity(events.len());
            for event in &events {
                let Some(xml) = render_xml(event) else { continue };
                let Some(parsed) = super::parse_event(&xml) else { continue };
                tracing::debug!(
                    channel,
                    event_id = parsed.event_id,
                    level = super::level_name(parsed.level),
                    "event log match"
                );
                let publisher = self.publisher(&parsed.provider);
                let message = if publisher == 0 { None } else { format_message(publisher, event) };
                out.push(RawEvent { xml, message });
            }
            Ok(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use dumbmonit_proto::EventIdRange;

    use super::*;

    fn rule(id: i64) -> EventLogRule {
        EventLogRule {
            id,
            channel: "Security".into(),
            provider: Some("Microsoft-Windows-Security-Auditing".into()),
            event_ids: vec![
                EventIdRange { from: 4625, to: 4625 },
                EventIdRange { from: 4740, to: 4799 },
            ],
            levels: vec![EventLevel::Information, EventLevel::Error],
            pattern: None,
            regex: false,
        }
    }

    const XML: &str = "<Event xmlns='http://schemas.microsoft.com/win/2004/08/events/event'>\
        <System><Provider Name='Microsoft-Windows-Security-Auditing' \
        Guid='{54849625-5478-4994-a5ba-3e3b0328c30d}'/><EventID>4625</EventID>\
        <Version>0</Version><Level>0</Level><Task>12544</Task>\
        <TimeCreated SystemTime='2026-10-25T09:30:00.1234567Z'/>\
        <EventRecordID>81234</EventRecordID><Channel>Security</Channel>\
        <Computer>srv-ad.example.lan</Computer><Security/></System>\
        <EventData><Data Name='TargetUserName'>admin</Data>\
        <Data Name='IpAddress'>10.0.0.5</Data><Data Name='Empty'></Data>\
        <Data Name='Note'>a &lt;b&gt; &amp; c&#x21;</Data></EventData></Event>";

    fn event_xml(record_id: u64, event_id: u32) -> String {
        XML.replace("81234", &record_id.to_string())
            .replace("<EventID>4625", &format!("<EventID>{event_id}"))
    }

    #[test]
    fn la_requete_xpath_combine_fournisseur_identifiants_niveaux_et_position() {
        assert_eq!(
            xpath(&rule(1), Some(42)),
            "*[System[Provider[@Name='Microsoft-Windows-Security-Auditing'] and \
             (EventID=4625 or (EventID>=4740 and EventID<=4799)) and \
             (Level=0 or Level=2 or Level=4) and EventRecordID>42]]"
        );
        let mut bare = rule(1);
        bare.provider = Some("Bad']] or [(".into());
        bare.event_ids.clear();
        bare.levels.clear();
        assert_eq!(xpath(&bare, None), "*[System[Provider[@Name='Bad or (']]]");
        bare.provider = None;
        assert_eq!(xpath(&bare, None), "*");
    }

    #[test]
    fn le_xml_rendu_d_un_evenement_se_lit() {
        let event = parse_event(XML).expect("événement");
        assert_eq!(event.provider, "Microsoft-Windows-Security-Auditing");
        assert_eq!(event.event_id, 4625);
        assert_eq!(event.level, 0);
        assert_eq!(event.record_id, 81234);
        assert_eq!(event.channel, "Security");
        assert_eq!(event.computer, "srv-ad.example.lan");
        assert_eq!(
            event.time_created_ms,
            chrono::DateTime::parse_from_rfc3339("2026-10-25T09:30:00.123Z")
                .unwrap()
                .timestamp_millis()
        );
        assert_eq!(event.data, "TargetUserName: admin\nIpAddress: 10.0.0.5\nNote: a <b> & c!");
        assert!(parse_event("<Event><System></System></Event>").is_none());
        // Un identifiant avec qualificatifs, des guillemets doubles.
        let classic = "<Event><System><Provider Name=\"Service Control Manager\"/>\
            <EventID Qualifiers='16384'>7036</EventID><Level>4</Level></System></Event>";
        let event = parse_event(classic).unwrap();
        assert_eq!((event.provider.as_str(), event.event_id), ("Service Control Manager", 7036));
    }

    #[test]
    fn le_motif_cherche_sans_casse_ou_par_expression() {
        let mut with = rule(1);
        with.pattern = Some("ADMIN".into());
        assert!(Matcher::new(&with).matches("Account Name: admin"));
        assert!(!Matcher::new(&with).matches("Account Name: guest"));
        with.pattern = Some(r"^Account Name: (admin|root)$".into());
        with.regex = true;
        assert!(Matcher::new(&with).matches("Account Name: root"));
        with.pattern = Some("(".into());
        assert!(!Matcher::new(&with).matches("("), "une expression invalide ne répond à rien");
        with.pattern = Some("  ".into());
        assert!(Matcher::new(&with).matches("tout"));
    }

    #[test]
    fn la_limite_de_debit_compte_par_regle_et_par_minute() {
        let mut limiter = RateLimiter::default();
        let allowed = (0..15).filter(|_| limiter.allow(1, 1_000)).count();
        assert_eq!(allowed, EVENT_RULE_MAX_PER_MINUTE as usize);
        assert!(limiter.allow(2, 1_000), "une autre règle a son propre compte");
        assert!(!limiter.allow(1, 1_000 + RATE_WINDOW_MS - 1));
        assert!(limiter.allow(1, 1_000 + RATE_WINDOW_MS), "nouvelle fenêtre");
    }

    #[test]
    fn un_texte_echappe_se_decode() {
        assert_eq!(unescape("a &amp;&amp; b &#233; &bogus; &"), "a && b é &bogus; &");
    }

    /// Un journal en mémoire : les événements ajoutés, et les requêtes vues.
    #[derive(Default)]
    struct FakeLog {
        events: Vec<(u64, u32, Option<String>)>,
        queries: Vec<String>,
        failure: Option<String>,
    }

    impl EventSource for FakeLog {
        fn latest_record_id(&mut self, _channel: &str) -> Result<Option<u64>, String> {
            if let Some(failure) = &self.failure {
                return Err(failure.clone());
            }
            Ok(self.events.iter().map(|(id, _, _)| *id).max())
        }

        fn query(
            &mut self,
            _channel: &str,
            xpath: &str,
            max: usize,
        ) -> Result<Vec<RawEvent>, String> {
            self.queries.push(xpath.to_string());
            let after: u64 = xpath
                .split("EventRecordID>")
                .nth(1)
                .and_then(|rest| rest.trim_end_matches(']').parse().ok())
                .unwrap_or(0);
            Ok(self
                .events
                .iter()
                .filter(|(id, _, _)| *id > after)
                .take(max)
                .map(|(id, event_id, message)| RawEvent {
                    xml: event_xml(*id, *event_id),
                    message: message.clone(),
                })
                .collect())
        }
    }

    fn probe(log: FakeLog, path: Option<PathBuf>) -> EventLogProbe<FakeLog> {
        let mut probe = EventLogProbe::new(log, path);
        probe.set_rules(vec![rule(7)]);
        probe
    }

    #[test]
    fn une_regle_neuve_ne_remonte_que_les_evenements_a_venir() {
        let log =
            FakeLog { events: vec![(10, 4625, None), (11, 4625, None)], ..FakeLog::default() };
        let mut probe = probe(log, None);
        probe.read(0);
        assert!(probe.report().is_none(), "l'historique du journal n'est pas remonté");
        assert!(probe.source.queries.is_empty());

        probe.source.events.push((12, 4625, Some("An account failed to log on.".into())));
        probe.read(1_000);
        let report = probe.report().expect("rapport");
        assert_eq!(report.events.len(), 1);
        let event = &report.events[0];
        assert_eq!((event.rule_id, event.record_id, event.event_id), (7, 12, 4625));
        assert_eq!(event.message, "An account failed to log on.");
        assert_eq!(event.computer, "srv-ad.example.lan");
        assert!(probe.source.queries[0].ends_with("EventRecordID>11]]"));

        // Rien de neuf : pas de requête du tout.
        probe.read(2_000);
        assert_eq!(probe.source.queries.len(), 1);

        // Le serveur a accepté : la file se vide, ce qui arrive ensuite reste.
        probe.source.events.push((13, 4625, None));
        probe.read(3_000);
        probe.acknowledge(Some(&report));
        let rest = probe.report().expect("reste");
        assert_eq!(rest.events.len(), 1);
        assert_eq!(rest.events[0].record_id, 13);
        assert!(rest.events[0].message.contains("TargetUserName: admin"), "repli sur EventData");
    }

    #[test]
    fn la_limite_de_debit_compte_les_evenements_ecartes() {
        let mut probe = probe(FakeLog::default(), None);
        probe.read(0);
        probe.source.events = (1..=25).map(|id| (id, 4625, None)).collect();
        probe.read(1_000);
        let report = probe.report().unwrap();
        assert_eq!(report.events.len(), EVENT_RULE_MAX_PER_MINUTE as usize);
        assert_eq!(report.dropped, [EventLogDropped { rule_id: 7, count: 15 }]);
        probe.acknowledge(Some(&report));
        assert!(probe.report().is_none());
    }

    #[test]
    fn le_motif_filtre_et_un_journal_vide_repart_de_zero() {
        let mut probe = EventLogProbe::new(FakeLog::default(), None);
        let mut only_admin = rule(3);
        only_admin.pattern = Some("admin".into());
        probe.set_rules(vec![only_admin]);
        probe.source.events = vec![(500, 4625, None)];
        probe.read(0);
        probe.source.events = vec![
            (500, 4625, None),
            (501, 4625, Some("Account Name: guest".into())),
            (502, 4625, Some("Account Name: Admin".into())),
        ];
        probe.read(1_000);
        let report = probe.report().unwrap();
        assert_eq!(report.events.iter().map(|e| e.record_id).collect::<Vec<_>>(), [502]);
        probe.acknowledge(Some(&report));

        // Journal effacé : les numéros repartent de 1, rien ne doit se perdre.
        probe.source.events = vec![(1, 4625, Some("admin again".into()))];
        probe.read(2_000);
        assert_eq!(probe.report().unwrap().events[0].record_id, 1);
    }

    #[test]
    fn les_positions_survivent_a_un_redemarrage() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(STATE_FILE);
        let log = FakeLog { events: vec![(10, 4625, None)], ..FakeLog::default() };
        let mut first = probe(log, Some(path.clone()));
        first.read(0);
        drop(first);

        // L'agent était arrêté pendant ce 4625 : il est remonté au redémarrage.
        let log =
            FakeLog { events: vec![(10, 4625, None), (11, 4625, None)], ..FakeLog::default() };
        let mut again = probe(log, Some(path));
        again.read(1_000);
        let report = again.report().unwrap();
        assert_eq!(report.events.iter().map(|e| e.record_id).collect::<Vec<_>>(), [11]);
    }

    #[test]
    fn une_regle_retiree_oublie_sa_position_et_un_echec_ne_bloque_rien() {
        let log = FakeLog { failure: Some("access denied".into()), ..FakeLog::default() };
        let mut probe = probe(log, None);
        probe.read(0);
        probe.read(1_000);
        assert!(probe.report().is_none());
        assert_eq!(probe.announced.len(), 1, "un seul avertissement par panne");
        probe.source.failure = None;
        probe.read(2_000);
        assert!(probe.positions.contains_key(&7));
        probe.set_rules(Vec::new());
        assert!(probe.positions.is_empty());
        assert!(probe.announced.is_empty());
    }

    #[test]
    fn la_file_d_attente_est_bornee() {
        let mut probe = probe(FakeLog::default(), None);
        for record_id in 0..(MAX_EVENT_LOG_ENTRIES as u64 + 3) {
            probe.push(EventLogEntry {
                rule_id: 7,
                channel: "Security".into(),
                provider: String::new(),
                event_id: 4625,
                level: 0,
                record_id,
                time_created_ms: 0,
                computer: String::new(),
                message: String::new(),
            });
        }
        let report = probe.report().unwrap();
        assert_eq!(report.events.len(), MAX_EVENT_LOG_ENTRIES);
        assert_eq!(report.events[0].record_id, 3, "les plus anciens cèdent la place");
        assert_eq!(report.dropped, [EventLogDropped { rule_id: 7, count: 3 }]);
        assert_eq!(level_name(2), "error");
        assert_eq!(level_name(5), "verbose");
    }
}
