//! Carte en direct d'un pare-feu OPNsense : d'où viennent les connexions que
//! ses règles bloquent ou laissent passer.
//!
//! - `GET /api/targets/{id}/opnsense/live?after=<seq>` : la fenêtre glissante
//!   (quinze minutes) agrégée par pays, règle et port, plus les évènements
//!   arrivés depuis `after`, pour l'animation ;
//! - `GET|PUT /api/geoip` : l'état de la base IP → pays, et son interrupteur.
//!
//! Le pare-feu n'est interrogé que si sa cible a l'option `live_map`, et
//! seulement tant que quelqu'un regarde : la première lecture démarre une tâche
//! qui lit le journal (`/api/diagnostics/firewall/log`, en lecture seule)
//! toutes les cinq secondes, et qui s'arrête d'elle-même une minute après la
//! dernière lecture. Le navigateur ne parle jamais au pare-feu.
//!
//! Rien n'est stocké : la fenêtre vit en mémoire, bornée en durée et en
//! nombre. Une adresse IP ne quitte jamais le serveur — ni vers le navigateur,
//! ni vers un service tiers : elle est réduite à son pays (base locale, voir
//! [`crate::geoip`]) ou à « réseau local » dès sa lecture.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use dumbmonit_collectors::opnsense::firewall_log::{self, LogEntry};
use dumbmonit_proto::{Target, TargetId};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::api::{ApiError, ApiResult};
use crate::auth::middleware::AdminIdentity;
use crate::db;
use crate::geoip::{self, GeoDb, GeoStatus, Origin};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/targets/{id}/opnsense/live", get(live))
        .route("/geoip", get(get_geoip).put(put_geoip))
}

/// Fenêtre glissante gardée en mémoire.
pub const WINDOW_SECS: i64 = 15 * 60;
/// Au-delà, les plus anciens évènements partent les premiers.
const MAX_EVENTS: usize = 20_000;
const POLL_EVERY: Duration = Duration::from_secs(5);
/// Après une erreur, on attend plus longtemps avant de réessayer.
const BACKOFF: Duration = Duration::from_secs(30);
/// Sans lecteur depuis une minute, la tâche s'arrête.
const IDLE: Duration = Duration::from_secs(60);
/// Lignes demandées à chaque lecture du journal.
const LINES_PER_POLL: usize = 500;
/// Évènements rendus à chaque lecture pour l'animation.
const MAX_RECENT: usize = 200;
/// Entrées des classements (règles, ports, pays).
const TOP: usize = 8;

/// Ce qui reste d'une ligne du journal : jamais l'adresse.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LiveEvent {
    pub seq: u64,
    /// Secondes Unix.
    pub at: i64,
    /// `block`, `pass` ou `other`.
    pub action: &'static str,
    /// Vrai quand la connexion part du réseau local vers l'extérieur.
    pub outbound: bool,
    /// Pays de l'autre bout (`CH`), `local` ou `unknown`.
    pub country: String,
    pub proto: String,
    pub port: Option<u16>,
    /// Description de la règle, ou son identifiant à défaut.
    pub rule: String,
}

/// Réduit une ligne du journal à ce que la carte montre.
///
/// L'autre bout est la source d'une ligne entrante ; pour une ligne sortante,
/// ou une ligne entrante venue du réseau local vers l'extérieur, c'est la
/// destination qui situe l'échange.
pub fn reduce(line: &LogEntry, db: Option<&GeoDb>, seq: u64, now: i64) -> LiveEvent {
    let src = geoip::classify(&line.src, db);
    let dst = geoip::classify(&line.dst, db);
    // Une ligne sortante (`out`, sur le WAN) porte en source l'adresse publique
    // du pare-feu lui-même : c'est la destination qui situe l'échange.
    let (remote, outbound) = match (line.dir.as_str(), src, dst) {
        ("out", _, Origin::Local) => (Origin::Local, false),
        ("out", _, other) => (other, true),
        (_, Origin::Local, Origin::Local) => (Origin::Local, false),
        (_, Origin::Local, other) => (other, true),
        (_, other, _) => (other, false),
    };
    let action = match line.action.as_str() {
        "block" | "reject" => "block",
        "pass" => "pass",
        _ => "other",
    };
    let at = chrono::DateTime::parse_from_rfc3339(line.timestamp.trim())
        .map(|t| t.timestamp())
        .unwrap_or(now);
    let rule = if !line.label.trim().is_empty() {
        line.label.trim().chars().take(120).collect()
    } else if !line.rid.trim().is_empty() {
        format!("rule {}", line.rid.trim().chars().take(8).collect::<String>())
    } else {
        String::new()
    };
    LiveEvent {
        seq,
        at,
        action,
        outbound,
        country: remote.code(),
        proto: line.protoname.trim().to_ascii_lowercase().chars().take(12).collect(),
        port: line.dstport,
        rule,
    }
}

#[derive(Default)]
struct Feed {
    events: VecDeque<LiveEvent>,
    seq: u64,
    /// Empreinte de la ligne la plus récente déjà lue.
    digest: Option<String>,
    polled_at: Option<i64>,
    error: Option<String>,
    running: bool,
    last_reader: Option<Instant>,
}

impl Feed {
    /// Ajoute les lignes lues (la plus récente d'abord) dans l'ordre du temps,
    /// et oublie ce qui sort de la fenêtre.
    fn absorb(&mut self, lines: &[LogEntry], db: Option<&GeoDb>, now: i64) {
        if let Some(first) = lines.first().filter(|l| !l.digest.is_empty()) {
            self.digest = Some(first.digest.clone());
        }
        for line in lines.iter().rev() {
            self.seq += 1;
            let event = reduce(line, db, self.seq, now);
            if event.at >= now - WINDOW_SECS {
                self.events.push_back(event);
            }
        }
        self.trim(now);
    }

    fn trim(&mut self, now: i64) {
        while self.events.front().is_some_and(|e| e.at < now - WINDOW_SECS) {
            self.events.pop_front();
        }
        while self.events.len() > MAX_EVENTS {
            self.events.pop_front();
        }
    }
}

static FEEDS: LazyLock<Mutex<HashMap<TargetId, Arc<Mutex<Feed>>>>> = LazyLock::new(Mutex::default);

fn feed(id: TargetId) -> Arc<Mutex<Feed>> {
    let mut feeds = FEEDS.lock().unwrap_or_else(|p| p.into_inner());
    feeds.entry(id).or_default().clone()
}

fn lock(feed: &Mutex<Feed>) -> std::sync::MutexGuard<'_, Feed> {
    feed.lock().unwrap_or_else(|p| p.into_inner())
}

/// Lance la lecture du journal si elle ne tourne pas déjà.
fn start(state: &AppState, target: Target, feed: Arc<Mutex<Feed>>) {
    {
        let mut f = lock(&feed);
        f.last_reader = Some(Instant::now());
        if f.running {
            return;
        }
        f.running = true;
    }
    let state = state.clone();
    tokio::spawn(async move {
        let id = target.id;
        loop {
            let digest = lock(&feed).digest.clone();
            let outcome = firewall_log::read(&target, digest.as_deref(), LINES_PER_POLL).await;
            let now = chrono::Utc::now().timestamp();
            let pause = {
                let mut f = lock(&feed);
                f.polled_at = Some(now);
                match outcome {
                    Ok(lines) => {
                        let db = geoip::current();
                        f.absorb(&lines, db.as_deref(), now);
                        f.error = None;
                        POLL_EVERY
                    }
                    Err(error) => {
                        f.error = Some(error.to_string());
                        f.trim(now);
                        BACKOFF
                    }
                }
            };
            geoip::ensure(&state.pool, &state.config.data_dir).await;
            tokio::time::sleep(pause).await;
            let mut f = lock(&feed);
            if f.last_reader.is_none_or(|t| t.elapsed() > IDLE) {
                f.running = false;
                debug!(target = id, "carte en direct : plus de lecteur, arrêt");
                return;
            }
        }
    });
}

#[derive(Debug, Serialize)]
pub struct Tally {
    pub blocked: u64,
    pub passed: u64,
    pub other: u64,
}

impl Tally {
    fn add(&mut self, action: &str) {
        match action {
            "block" => self.blocked += 1,
            "pass" => self.passed += 1,
            _ => self.other += 1,
        }
    }
    fn total(&self) -> u64 {
        self.blocked + self.passed + self.other
    }
}

#[derive(Debug, Serialize)]
pub struct CountryRow {
    /// `CH`, `local` ou `unknown`.
    pub code: String,
    #[serde(flatten)]
    pub tally: Tally,
}

#[derive(Debug, Serialize)]
pub struct RuleRow {
    pub rule: String,
    pub action: &'static str,
    pub count: u64,
}

#[derive(Debug, Serialize)]
pub struct PortRow {
    pub proto: String,
    pub port: u16,
    pub count: u64,
}

#[derive(Debug, Serialize)]
pub struct FirewallSpot {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Debug, Serialize)]
pub struct LiveView {
    /// Faux tant que l'option `live_map` de la cible est éteinte.
    pub enabled: bool,
    /// Vrai pour une cible interrogée par un relais : le serveur ne joint pas
    /// le pare-feu lui-même.
    pub relayed: bool,
    pub window_secs: i64,
    pub polled_at: Option<i64>,
    pub error: Option<String>,
    pub totals: Tally,
    pub countries: Vec<CountryRow>,
    pub rules: Vec<RuleRow>,
    pub ports: Vec<PortRow>,
    /// Évènements arrivés après `after`, les plus anciens d'abord.
    pub recent: Vec<LiveEvent>,
    /// Dernier numéro rendu : à renvoyer en `after` la fois suivante.
    pub seq: u64,
    /// L'emplacement du pare-feu sur la carte du monde, s'il en a un.
    pub firewall: Option<FirewallSpot>,
    pub geoip: GeoStatus,
}

/// Agrège la fenêtre : totaux, pays, règles et ports, les plus actifs d'abord.
pub fn summarize(
    events: &VecDeque<LiveEvent>,
    after: u64,
) -> (Tally, Vec<CountryRow>, Vec<RuleRow>, Vec<PortRow>, Vec<LiveEvent>) {
    let mut totals = Tally { blocked: 0, passed: 0, other: 0 };
    let mut countries: HashMap<&str, Tally> = HashMap::new();
    let mut rules: HashMap<(&str, &'static str), u64> = HashMap::new();
    let mut ports: HashMap<(&str, u16), u64> = HashMap::new();
    for e in events {
        totals.add(e.action);
        countries
            .entry(e.country.as_str())
            .or_insert(Tally { blocked: 0, passed: 0, other: 0 })
            .add(e.action);
        if !e.rule.is_empty() {
            *rules.entry((e.rule.as_str(), e.action)).or_default() += 1;
        }
        if let Some(port) = e.port
            && !e.outbound
        {
            *ports.entry((e.proto.as_str(), port)).or_default() += 1;
        }
    }
    let mut countries: Vec<CountryRow> = countries
        .into_iter()
        .map(|(code, tally)| CountryRow { code: code.into(), tally })
        .collect();
    countries.sort_by(|a, b| b.tally.total().cmp(&a.tally.total()).then(a.code.cmp(&b.code)));
    let mut rules: Vec<RuleRow> = rules
        .into_iter()
        .map(|((rule, action), count)| RuleRow { rule: rule.into(), action, count })
        .collect();
    rules.sort_by(|a, b| b.count.cmp(&a.count).then(a.rule.cmp(&b.rule)));
    rules.truncate(TOP);
    let mut ports: Vec<PortRow> = ports
        .into_iter()
        .map(|((proto, port), count)| PortRow { proto: proto.into(), port, count })
        .collect();
    ports.sort_by(|a, b| b.count.cmp(&a.count).then(a.port.cmp(&b.port)));
    ports.truncate(TOP);
    let fresh: Vec<LiveEvent> = events.iter().filter(|e| e.seq > after).cloned().collect();
    let recent = fresh[fresh.len().saturating_sub(MAX_RECENT)..].to_vec();
    (totals, countries, rules, ports, recent)
}

#[derive(Debug, Deserialize)]
struct LiveQuery {
    #[serde(default)]
    after: u64,
}

/// `GET /api/targets/{id}/opnsense/live`
async fn live(
    State(state): State<AppState>,
    Path(id): Path<TargetId>,
    Query(query): Query<LiveQuery>,
) -> ApiResult<Json<LiveView>> {
    let target = db::targets::get(&state.pool, &state.cipher, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Monitor {id} not found.")))?;
    if target.kind != "opnsense" {
        return Err(ApiError::BadRequest("This monitor is not an OPNsense firewall.".into()));
    }
    let enabled = firewall_log::live_map_enabled(&target) && target.enabled;
    let relayed = db::targets::relay_of(&state.pool, id).await?.is_some();
    let geo = geoip::status(&state.pool).await;
    let firewall = super::locations::location_for(&state, id).await?.map(|l| FirewallSpot {
        name: l.name,
        latitude: l.latitude,
        longitude: l.longitude,
    });
    let empty = || Tally { blocked: 0, passed: 0, other: 0 };
    if !enabled || relayed {
        return Ok(Json(LiveView {
            enabled,
            relayed,
            window_secs: WINDOW_SECS,
            polled_at: None,
            error: None,
            totals: empty(),
            countries: Vec::new(),
            rules: Vec::new(),
            ports: Vec::new(),
            recent: Vec::new(),
            seq: 0,
            firewall,
            geoip: geo,
        }));
    }
    geoip::ensure(&state.pool, &state.config.data_dir).await;
    let feed = feed(id);
    start(&state, target, feed.clone());
    // Lu avant de prendre le verrou : il ne doit jamais traverser un `.await`.
    let geo = geoip::status(&state.pool).await;
    let now = chrono::Utc::now().timestamp();
    let mut f = lock(&feed);
    f.trim(now);
    let (totals, countries, rules, ports, recent) = summarize(&f.events, query.after);
    Ok(Json(LiveView {
        enabled,
        relayed,
        window_secs: WINDOW_SECS,
        polled_at: f.polled_at,
        error: f.error.clone(),
        totals,
        countries,
        rules,
        ports,
        recent,
        seq: f.seq,
        firewall,
        geoip: geo,
    }))
}

/// `GET /api/geoip`
async fn get_geoip(State(state): State<AppState>) -> ApiResult<Json<GeoStatus>> {
    Ok(Json(geoip::status(&state.pool).await))
}

#[derive(Debug, Deserialize)]
struct GeoPayload {
    enabled: bool,
}

/// `PUT /api/geoip` — allumer ou éteindre la base (administrateurs).
async fn put_geoip(
    AdminIdentity(_): AdminIdentity,
    State(state): State<AppState>,
    Json(payload): Json<GeoPayload>,
) -> ApiResult<Json<GeoStatus>> {
    geoip::set_enabled(&state.pool, payload.enabled).await?;
    if payload.enabled {
        geoip::ensure(&state.pool, &state.config.data_dir).await;
    }
    Ok(Json(geoip::status(&state.pool).await))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOG: &str = include_str!("../../../collectors/src/opnsense/testdata/firewall_log.json");
    // Trié par adresse, comme le CSV de DB-IP.
    const CSV: &str = "45.155.0.0,45.155.255.255,NL\n\
        140.82.112.0,140.82.127.255,US\n\
        185.220.0.0,185.220.255.255,DE\n\
        2a02:1210::,2a02:1210:ffff:ffff:ffff:ffff:ffff:ffff,CH\n";

    fn lines() -> Vec<LogEntry> {
        serde_json::from_str(LOG).expect("journal")
    }

    #[test]
    fn une_ligne_devient_un_pays_sans_adresse() {
        let db = GeoDb::parse(CSV);
        let lines = lines();
        let e = reduce(&lines[0], Some(&db), 1, 0);
        assert_eq!(e.country, "DE");
        assert_eq!(e.action, "block");
        assert_eq!(e.port, Some(22));
        assert_eq!(e.at, 1_792_395_667, "l'heure du journal, fuseau compris");
        assert_eq!(e.rule, "Default deny / state violation rule");
        assert!(!e.outbound);
        let json = serde_json::to_string(&e).unwrap();
        assert!(!json.contains("185.220"), "aucune adresse ne sort : {json}");

        let v6 = reduce(&lines[1], Some(&db), 2, 0);
        assert_eq!((v6.country.as_str(), v6.action), ("CH", "pass"));
        let lan = reduce(&lines[3], Some(&db), 4, 0);
        assert_eq!(lan.country, "unknown", "1.1.1.1 n'est pas dans la petite base");
        assert!(lan.outbound, "du réseau local vers l'extérieur");
        let out = reduce(&lines[4], Some(&db), 5, 0);
        assert_eq!(out.country, "US", "le pare-feu sort vers GitHub : la destination compte");
        let carp = reduce(&lines[5], Some(&db), 6, 0);
        assert_eq!(carp.country, "local");
        assert_eq!(carp.rule, "", "ni description ni identifiant");
        let anonymous =
            reduce(&LogEntry { rid: "02f4bab031b57d1e".into(), ..LogEntry::default() }, None, 7, 9);
        assert_eq!(anonymous.rule, "rule 02f4bab0");
        assert_eq!(anonymous.at, 9, "sans heure lisible, maintenant");
        assert_eq!(anonymous.country, "unknown");
    }

    #[test]
    fn la_fenetre_glisse_et_s_agrege() {
        let db = GeoDb::parse(CSV);
        let lines = lines();
        let now = 1_792_395_667 + 10;
        let mut feed = Feed::default();
        feed.absorb(&lines, Some(&db), now);
        assert_eq!(feed.events.len(), 6);
        assert_eq!(feed.digest.as_deref(), Some(lines[0].digest.as_str()), "la plus récente");
        assert_eq!(feed.events.back().unwrap().at, 1_792_395_667, "dans l'ordre du temps");

        let (totals, countries, rules, ports, recent) = summarize(&feed.events, 0);
        assert_eq!((totals.blocked, totals.passed, totals.other), (2, 4, 0));
        assert_eq!(countries.len(), 6, "DE, CH, NL, US, le réseau local (CARP) et l'inconnu");
        assert_eq!(countries[0].code, "CH", "à égalité, par code");
        let de = countries.iter().find(|c| c.code == "DE").unwrap();
        assert_eq!(de.tally.blocked, 1);
        assert_eq!(rules[0].rule, "Default deny / state violation rule");
        assert_eq!(rules[0].count, 2);
        assert_eq!(rules[0].action, "block");
        assert!(
            ports.iter().all(|p| p.port != 443 || p.count == 1),
            "le 443 sortant ne compte pas"
        );
        assert!(ports.iter().any(|p| p.proto == "udp" && p.port == 5060));
        assert_eq!(recent.len(), 6);
        let (_, _, _, _, after) = summarize(&feed.events, 4);
        assert_eq!(after.len(), 2, "seulement ce qui est arrivé après");

        // Un quart d'heure plus tard, tout est sorti de la fenêtre.
        feed.trim(now + WINDOW_SECS + 60);
        assert!(feed.events.is_empty());
    }

    #[test]
    fn la_fenetre_est_bornee_en_nombre() {
        let mut feed = Feed::default();
        let line = LogEntry { action: "block".into(), ..LogEntry::default() };
        let many = vec![line; MAX_EVENTS + 50];
        feed.absorb(&many, None, 1000);
        assert_eq!(feed.events.len(), MAX_EVENTS);
        assert_eq!(feed.seq, (MAX_EVENTS + 50) as u64);
    }
}
