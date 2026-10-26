//! Ceph, par l'API REST du tableau de bord du gestionnaire (`ceph-mgr`,
//! module `dashboard`, port 8443 en HTTPS).
//!
//! Un utilisateur du tableau de bord au rôle intégré `read-only` ouvre une
//! session (`POST /api/auth`, nom et mot de passe dans le corps) ; le jeton
//! rendu part ensuite en `Authorization: Bearer`, jamais dans l'URL. Il est
//! gardé une heure (le tableau de bord le fait vivre huit heures par défaut) et
//! on ne le redemande qu'à son refus : chaque déconnexion inscrit le jeton sur
//! une liste de révocation que le gestionnaire écrit dans sa base, ce qui ne
//! vaut pas une écriture par minute. Chaque appel porte
//! `Accept: application/vnd.ceph.api.v1.0+json`, la version de l'API que le
//! tableau de bord documente.
//!
//! Une lecture obligatoire — `/api/health/minimal` : santé, contrôles, PG,
//! capacité — et quatre facultatives qu'un rôle plus restreint ou une version
//! différente peut refuser sans faire échouer le reste : les OSD (`/api/osd`),
//! les pools (`/api/pool`), les moniteurs (`/api/monitor`) et la version
//! (`/api/summary`).
//!
//! Les pannes que ce module existe pour voir :
//!
//! * **la santé de la grappe** (`HEALTH_WARN`, `HEALTH_ERR`) et les contrôles
//!   qui la détaillent ;
//! * **des OSD arrêtés** ou sortis de la grappe, **un OSD presque plein** ;
//! * **des PG qui ne sont plus `active+clean`** : données dégradées ou
//!   indisponibles ;
//! * **la capacité** de la grappe et de chaque pool ;
//! * **un moniteur hors du quorum**.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `scheme` | `https` | Protocole du tableau de bord. |
//! | `port` | `8443` | Port du tableau de bord (8080 en HTTP). |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable (celui d'origine est auto-signé). |
//! | `request_timeout_seconds` | `15` | Délai par requête HTTP. |

use std::time::Duration;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};
use serde_json::{Value, json};

use crate::rest::{MAX_NAMED, RestClient, TokenCache, flag, gauge, now_ms, number, text};

pub const DEFAULT_PORT: u16 = 8443;

const ACCEPT: &str = "application/vnd.ceph.api.v1.0+json";
const AUTH: &str = "/api/auth";
const HEALTH: &str = "/api/health/minimal";
const OSD: &str = "/api/osd";
const POOL: &str = "/api/pool?stats=true";
const MONITOR: &str = "/api/monitor";
const SUMMARY: &str = "/api/summary";
/// Durée pendant laquelle un jeton est gardé.
const TOKEN_LIFETIME: Duration = Duration::from_secs(3600);

#[derive(Default)]
pub struct CephCollector {
    tokens: TokenCache,
}

impl CephCollector {
    pub fn new() -> Self {
        Self::default()
    }
}

struct Session<'a> {
    collector: &'a CephCollector,
    client: RestClient,
    username: String,
    password: String,
    key: String,
}

impl Session<'_> {
    /// Connexion, ou jeton gardé.
    async fn login(&mut self, fresh: bool) -> Result<(), ProbeError> {
        if !fresh && let Some(token) = self.collector.tokens.get(&self.key) {
            self.client.set_header("Authorization", format!("Bearer {token}"));
            return Ok(());
        }
        let body = json!({ "username": self.username, "password": self.password });
        let (status, reply) = self.client.post_json(AUTH, &body).await?;
        if matches!(status.as_u16(), 400 | 401 | 403) {
            return Err(ProbeError::Auth(format!(
                "Ceph refused the login of {} ({status}). Check the user name and the password of \
                 the dashboard user, and that it has a role.",
                self.username
            )));
        }
        if !status.is_success() {
            return Err(self.client.status_error(status, &reply, AUTH));
        }
        let reply: Value = self.client.decode(&reply, AUTH)?;
        let token = text(&reply, "token")
            .ok_or_else(|| ProbeError::Protocol("Ceph returned no token".to_string()))?;
        self.collector.tokens.put(&self.key, token, TOKEN_LIFETIME);
        self.client.set_header("Authorization", format!("Bearer {token}"));
        Ok(())
    }

    async fn get(&mut self, path: &str) -> Result<Value, ProbeError> {
        let (mut status, mut body) = self.client.get(path).await?;
        if status.as_u16() == 401 {
            // Jeton gardé expiré ou révoqué : une seule reconnexion.
            self.collector.tokens.forget(&self.key);
            self.login(true).await?;
            (status, body) = self.client.get(path).await?;
        }
        if !status.is_success() {
            return Err(self.client.status_error(status, &body, path));
        }
        self.client.decode(&body, path)
    }
}

/// Une lecture facultative : un refus ou une route absente ne fait rien
/// échouer, une panne réelle si.
fn optional(outcome: Result<Value, ProbeError>) -> Result<Option<Value>, ProbeError> {
    match outcome {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.means_down() => Err(error),
        Err(error) => {
            tracing::debug!(%error, "Ceph : lecture facultative indisponible");
            Ok(None)
        }
    }
}

impl CephCollector {
    fn session(&self, target: &Target) -> Result<Session<'_>, ProbeError> {
        let (username, password) = match &target.credential {
            Credential::UsernamePassword { username, password } if !username.trim().is_empty() => {
                (username.trim().to_string(), password.clone())
            }
            other => {
                return Err(ProbeError::Config(format!(
                    "Ceph expects the user name and password of a dashboard user, configured: \
                     {other}"
                )));
            }
        };
        let client = RestClient::for_target(target, "https", DEFAULT_PORT, "Ceph")?
            .with_header("Accept", ACCEPT);
        let key = format!("{}:{username}", target.id);
        Ok(Session { collector: self, client, username, password, key })
    }
}

#[async_trait]
impl Collector for CephCollector {
    fn kind(&self) -> &'static str {
        "ceph"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let mut session = self.session(target)?;
        session.login(false).await?;
        let health = session.get(HEALTH).await?;
        if health.get("health").is_none() {
            return Err(ProbeError::Protocol(format!(
                "{} did not answer like the Ceph dashboard (no \"health\"): check the address and \
                 the port of the dashboard (8443).",
                session.client.url(HEALTH)
            )));
        }
        let replies = Replies {
            health,
            osds: optional(session.get(OSD).await)?,
            pools: optional(session.get(POOL).await)?,
            monitors: optional(session.get(MONITOR).await)?,
            summary: optional(session.get(SUMMARY).await)?,
        };
        Ok(samples(&replies, now_ms()))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let mut session = self.session(target)?;
        session.login(false).await?;
        session.get(HEALTH).await?;
        Ok(Some("ceph".to_string()))
    }
}

pub struct Replies {
    pub health: Value,
    pub osds: Option<Value>,
    pub pools: Option<Value>,
    pub monitors: Option<Value>,
    pub summary: Option<Value>,
}

/// Un nombre de statistique de pool : un nombre, ou `{"latest": …}`.
fn stat(item: &Value, key: &str) -> Option<f64> {
    let value = item.get("stats")?.get(key)?;
    number(value.get("latest").or(Some(value)))
}

fn list(value: Option<&Value>) -> Vec<&Value> {
    value.and_then(Value::as_array).into_iter().flatten().collect()
}

/// `ceph version 18.2.4 (…) reef (stable)` → `18.2.4`.
fn version(raw: &str) -> Option<&str> {
    raw.strip_prefix("ceph version ")?.split_whitespace().next()
}

fn percent(part: f64, total: f64) -> Option<f64> {
    (total > 0.0).then(|| (part / total * 100.0).clamp(0.0, 100.0))
}

/// Les contrôles de santé : un objet par nom (versions récentes) ou une liste
/// d'objets portant leur nom en `type` (plus anciennes). Chacun rend son nom,
/// sa sévérité et s'il est réduit au silence.
fn health_checks(health: &Value) -> Vec<(&str, &str, bool)> {
    let mut named: Vec<(&str, &Value)> = Vec::new();
    match health.get("checks") {
        Some(Value::Object(map)) => named.extend(map.iter().map(|(k, v)| (k.as_str(), v))),
        Some(Value::Array(items)) => {
            named.extend(items.iter().filter_map(|item| Some((text(item, "type")?, item))));
        }
        _ => {}
    }
    named
        .into_iter()
        .map(|(name, item)| {
            let severity = text(item, "severity").unwrap_or("HEALTH_WARN");
            let muted = item.get("muted").and_then(Value::as_bool).unwrap_or(false);
            (name, severity, muted)
        })
        .collect()
}

pub fn samples(r: &Replies, ts_ms: i64) -> Vec<Sample> {
    let g = |name: &str, value: f64| gauge(name, value, ts_ms);
    let mut out = Vec::new();
    let health = r.health.get("health").unwrap_or(&Value::Null);

    if let Some(raw) = r.summary.as_ref().and_then(|s| text(s, "version"))
        && let Some(version) = version(raw)
    {
        out.push(g("ceph_version_info", 1.0).with_label("version", version));
    }

    let status = text(health, "status").unwrap_or("");
    let level = match status {
        "HEALTH_OK" => Some(0.0),
        "HEALTH_WARN" => Some(1.0),
        "HEALTH_ERR" => Some(2.0),
        _ => None,
    };
    if let Some(level) = level {
        out.push(g("ceph_health_status", level));
        out.push(flag("ceph_health_ok", level == 0.0, ts_ms));
    }
    // Les contrôles réduits au silence par l'administrateur ne comptent pas.
    let all = health_checks(health);
    let checks: Vec<_> = all.into_iter().filter(|(_, _, muted)| !muted).collect();
    out.push(g("ceph_health_checks", checks.len() as f64));
    for (name, severity, _) in checks.into_iter().take(MAX_NAMED) {
        out.push(
            g("ceph_health_check_info", 1.0)
                .with_label("check", name)
                .with_label("severity", severity),
        );
    }

    osds(r, ts_ms, &mut out);
    pgs(&r.health, ts_ms, &mut out);
    capacity(&r.health, ts_ms, &mut out);
    monitors(r, ts_ms, &mut out);
    pools(r, ts_ms, &mut out);

    if let Some(hosts) = number(r.health.get("hosts")) {
        out.push(g("ceph_hosts", hosts));
    }
    if let Some(perf) = r.health.get("client_perf") {
        for (field, metric) in [
            ("read_bytes_sec", "ceph_client_read_bytes_per_second"),
            ("write_bytes_sec", "ceph_client_write_bytes_per_second"),
            ("read_op_per_sec", "ceph_client_read_ops_per_second"),
            ("write_op_per_sec", "ceph_client_write_ops_per_second"),
            ("recovering_bytes_per_sec", "ceph_recovering_bytes_per_second"),
        ] {
            if let Some(value) = number(perf.get(field)) {
                out.push(g(metric, value));
            }
        }
    }
    out
}

fn osds(r: &Replies, ts_ms: i64, out: &mut Vec<Sample>) {
    let g = |name: &str, value: f64| gauge(name, value, ts_ms);
    let Some(reply) = r.osds.as_ref() else { return };
    let items = list(Some(reply));
    let is_set = |item: &Value, key: &str| number(item.get(key)).is_some_and(|v| v >= 1.0);
    let total = items.len();
    let up = items.iter().filter(|item| is_set(item, "up")).count();
    let inside = items.iter().filter(|item| is_set(item, "in")).count();
    out.push(g("ceph_osds", total as f64));
    out.push(g("ceph_osds_up", up as f64));
    out.push(g("ceph_osds_in", inside as f64));
    out.push(g("ceph_osds_down", (total - up) as f64));
    for item in items.into_iter().take(MAX_NAMED) {
        let Some(id) = number(item.get("osd")) else { continue };
        let name = format!("osd.{}", id as u64);
        let host = item
            .pointer("/host/name")
            .and_then(Value::as_str)
            .or_else(|| item.get("host").and_then(Value::as_str))
            .unwrap_or("");
        let named =
            |sample: Sample| sample.with_label("osd", name.as_str()).with_label("host", host);
        out.push(named(flag("ceph_osd_up", is_set(item, "up"), ts_ms)));
        out.push(named(flag("ceph_osd_in", is_set(item, "in"), ts_ms)));
        let size = number(item.pointer("/stats/stat_bytes"));
        let used = number(item.pointer("/stats/stat_bytes_used"));
        if let (Some(size), Some(used)) = (size, used)
            && let Some(share) = percent(used, size)
        {
            out.push(named(g("ceph_osd_used_percent", share)));
        }
    }
}

fn pgs(health: &Value, ts_ms: i64, out: &mut Vec<Sample>) {
    let g = |name: &str, value: f64| gauge(name, value, ts_ms);
    let Some(info) = health.get("pg_info") else { return };
    if let Some(statuses) = info.get("statuses").and_then(Value::as_object) {
        let (mut total, mut clean) = (0.0, 0.0);
        for (state, count) in statuses {
            let count = number(Some(count)).unwrap_or(0.0);
            total += count;
            if state.contains("active") && state.contains("clean") {
                clean += count;
            }
        }
        out.push(g("ceph_pgs", total));
        out.push(g("ceph_pgs_clean", clean));
        out.push(g("ceph_pgs_unclean", total - clean));
        for (state, count) in statuses.iter().take(MAX_NAMED) {
            if let Some(count) = number(Some(count)) {
                out.push(g("ceph_pgs_in_state", count).with_label("state", state.as_str()));
            }
        }
    }
    if let Some(stats) = info.get("object_stats") {
        for (field, metric) in [
            ("num_objects", "ceph_objects"),
            ("num_objects_degraded", "ceph_objects_degraded"),
            ("num_objects_misplaced", "ceph_objects_misplaced"),
            ("num_objects_unfound", "ceph_objects_unfound"),
        ] {
            if let Some(value) = number(stats.get(field)) {
                out.push(g(metric, value));
            }
        }
    }
}

fn capacity(health: &Value, ts_ms: i64, out: &mut Vec<Sample>) {
    let g = |name: &str, value: f64| gauge(name, value, ts_ms);
    let Some(stats) = health.pointer("/df/stats") else { return };
    let Some(total) = number(stats.get("total_bytes")) else { return };
    let free = number(stats.get("total_avail_bytes"));
    let used = number(stats.get("total_used_raw_bytes"))
        .or_else(|| free.map(|free| (total - free).max(0.0)));
    out.push(g("ceph_capacity_bytes", total));
    if let Some(free) = free {
        out.push(g("ceph_capacity_available_bytes", free));
    }
    if let Some(used) = used {
        out.push(g("ceph_capacity_used_bytes", used));
        if let Some(share) = percent(used, total) {
            out.push(g("ceph_capacity_used_percent", share));
        }
    }
}

fn monitors(r: &Replies, ts_ms: i64, out: &mut Vec<Sample>) {
    let g = |name: &str, value: f64| gauge(name, value, ts_ms);
    let counts = match r.monitors.as_ref() {
        Some(reply) => {
            let inside = list(reply.get("in_quorum")).len();
            let outside = list(reply.get("out_quorum")).len();
            Some((inside, inside + outside))
        }
        None => {
            // À défaut, le résumé de la santé : quorum et carte des moniteurs.
            let status = r.health.get("mon_status");
            let inside = list(status.and_then(|s| s.get("quorum"))).len();
            let known = list(status.and_then(|s| s.pointer("/monmap/mons"))).len();
            (known > 0).then_some((inside, known))
        }
    };
    if let Some((inside, known)) = counts {
        out.push(g("ceph_mons", known as f64));
        out.push(g("ceph_mons_in_quorum", inside as f64));
        out.push(g("ceph_mons_out_of_quorum", known.saturating_sub(inside) as f64));
    }
}

fn pools(r: &Replies, ts_ms: i64, out: &mut Vec<Sample>) {
    let g = |name: &str, value: f64| gauge(name, value, ts_ms);
    let Some(reply) = r.pools.as_ref() else { return };
    let items = list(Some(reply));
    out.push(g("ceph_pools", items.len() as f64));
    for item in items.into_iter().take(MAX_NAMED) {
        let Some(name) = text(item, "pool_name") else { continue };
        let named = |sample: Sample| sample.with_label("pool", name);
        let bytes = stat(item, "bytes_used");
        // `percent_used` est une fraction ; sans elle, usage / (usage + libre).
        let used = stat(item, "percent_used").map(|fraction| fraction * 100.0).or_else(|| {
            let (used, free) = (bytes?, stat(item, "max_avail")?);
            percent(used, used + free)
        });
        if let Some(used) = used {
            out.push(named(g("ceph_pool_used_percent", used.clamp(0.0, 100.0))));
        }
        if let Some(bytes) = bytes {
            out.push(named(g("ceph_pool_used_bytes", bytes)));
        }
        if let Some(objects) = stat(item, "objects") {
            out.push(named(g("ceph_pool_objects", objects)));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use axum::Router;
    use axum::extract::Json;
    use axum::http::{HeaderMap, StatusCode, Uri};
    use axum::routing::{get, post};

    use super::*;
    use crate::rest::test_support::{find, serve, value};
    use crate::uptime::tags::test_support::cible;

    // Réponses construites d'après la documentation de l'API REST du tableau
    // de bord de Ceph (Reef) : une grappe de trois moniteurs dont un hors du
    // quorum, quatre OSD dont deux arrêtés (un sorti), des PG dégradés, un
    // contrôle réduit au silence, un pool presque plein.
    const HEALTH_JSON: &str = include_str!("testdata/health_minimal.json");
    const HEALTH_OK_JSON: &str = include_str!("testdata/health_ok.json");
    const OSD_JSON: &str = include_str!("testdata/osd.json");
    const POOL_JSON: &str = include_str!("testdata/pool.json");
    const MONITOR_JSON: &str = include_str!("testdata/monitor.json");
    const SUMMARY_JSON: &str = include_str!("testdata/summary.json");
    const AUTH_JSON: &str = include_str!("testdata/auth.json");

    fn json_of(raw: &str) -> Value {
        serde_json::from_str(raw).unwrap()
    }

    fn near(actual: f64, expected: f64) -> bool {
        (actual - expected).abs() < 1e-6
    }

    fn replies() -> Replies {
        Replies {
            health: json_of(HEALTH_JSON),
            osds: Some(json_of(OSD_JSON)),
            pools: Some(json_of(POOL_JSON)),
            monitors: Some(json_of(MONITOR_JSON)),
            summary: Some(json_of(SUMMARY_JSON)),
        }
    }

    #[test]
    fn sante_osd_pg_capacite_pools() {
        let s = samples(&replies(), 0);
        assert_eq!(find(&s, "ceph_version_info", &[]).unwrap().labels["version"], "18.2.4");
        assert_eq!(value(&s, "ceph_health_status", &[]), 1.0);
        assert_eq!(value(&s, "ceph_health_ok", &[]), 0.0);
        assert_eq!(value(&s, "ceph_health_checks", &[]), 2.0, "the muted check is left out");
        assert!(find(&s, "ceph_health_check_info", &[("check", "OSD_DOWN")]).is_some());
        assert!(find(&s, "ceph_health_check_info", &[("check", "POOL_APP_NOT_ENABLED")]).is_none());

        assert_eq!(value(&s, "ceph_osds", &[]), 4.0);
        assert_eq!(value(&s, "ceph_osds_up", &[]), 2.0);
        assert_eq!(value(&s, "ceph_osds_in", &[]), 3.0);
        assert_eq!(value(&s, "ceph_osds_down", &[]), 2.0);
        assert_eq!(value(&s, "ceph_osd_up", &[("osd", "osd.2")]), 0.0);
        assert_eq!(value(&s, "ceph_osd_in", &[("osd", "osd.3")]), 0.0);
        assert!(near(value(&s, "ceph_osd_used_percent", &[("osd", "osd.2")]), 90.0));

        assert_eq!(value(&s, "ceph_pgs", &[]), 128.0);
        assert_eq!(value(&s, "ceph_pgs_clean", &[]), 116.0);
        assert_eq!(value(&s, "ceph_pgs_unclean", &[]), 12.0);
        assert_eq!(
            value(&s, "ceph_pgs_in_state", &[("state", "active+undersized+degraded")]),
            12.0
        );
        assert_eq!(value(&s, "ceph_objects_degraded", &[]), 1200.0);

        assert!(near(value(&s, "ceph_capacity_used_percent", &[]), 75.0));
        assert_eq!(value(&s, "ceph_mons", &[]), 3.0);
        assert_eq!(value(&s, "ceph_mons_out_of_quorum", &[]), 1.0);
        assert_eq!(value(&s, "ceph_pools", &[]), 2.0);
        assert!(near(value(&s, "ceph_pool_used_percent", &[("pool", "rbd")]), 80.0));
        assert_eq!(value(&s, "ceph_client_write_ops_per_second", &[]), 45.0);
        assert_eq!(value(&s, "ceph_hosts", &[]), 3.0);
    }

    #[test]
    fn sante_correcte_sans_lectures_facultatives() {
        let r = Replies {
            health: json_of(HEALTH_OK_JSON),
            osds: None,
            pools: None,
            monitors: None,
            summary: None,
        };
        let s = samples(&r, 0);
        assert_eq!(value(&s, "ceph_health_ok", &[]), 1.0);
        assert_eq!(value(&s, "ceph_health_checks", &[]), 0.0, "list form, empty");
        assert_eq!(value(&s, "ceph_pgs_unclean", &[]), 0.0);
        assert!(near(value(&s, "ceph_capacity_used_percent", &[]), 10.0));
        assert!(find(&s, "ceph_osds", &[]).is_none());
        assert!(find(&s, "ceph_pools", &[]).is_none());
        assert!(find(&s, "ceph_version_info", &[]).is_none());
        assert!(find(&s, "ceph_mons", &[]).is_none());
    }

    #[test]
    fn quorum_lu_dans_la_sante_sans_la_page_des_moniteurs() {
        let mut r = replies();
        r.monitors = None;
        let s = samples(&r, 0);
        assert_eq!(value(&s, "ceph_mons_in_quorum", &[]), 2.0);
        assert_eq!(value(&s, "ceph_mons_out_of_quorum", &[]), 1.0);
    }

    #[test]
    fn controles_en_liste_des_versions_anciennes() {
        let health = json_of(
            r#"{"status":"HEALTH_ERR","checks":[{"type":"OSD_FULL","severity":"HEALTH_ERR","summary":{"message":"1 full osd(s)","count":1}}]}"#,
        );
        let checks = health_checks(&health);
        assert_eq!(checks, vec![("OSD_FULL", "HEALTH_ERR", false)]);
    }

    #[test]
    fn versions() {
        assert_eq!(version("ceph version 18.2.4 (abc) reef (stable)"), Some("18.2.4"));
        assert_eq!(version("autre chose"), None);
    }

    fn fake(logins: Arc<AtomicUsize>) -> Router {
        Router::new()
            .route(
                "/api/auth",
                post(move |Json(body): Json<Value>| {
                    let logins = logins.clone();
                    async move {
                        logins.fetch_add(1, Ordering::SeqCst);
                        if body["username"] == "dumbmonit" && body["password"] == "s3cret" {
                            (StatusCode::CREATED, AUTH_JSON)
                        } else {
                            (StatusCode::BAD_REQUEST, "{\"detail\":\"Invalid credentials\"}")
                        }
                    }
                }),
            )
            .fallback(get(|headers: HeaderMap, uri: Uri| async move {
                let bearer = headers.get("authorization").and_then(|v| v.to_str().ok());
                if bearer != Some("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.e30.signature") {
                    return (StatusCode::UNAUTHORIZED, "");
                }
                let accept = headers.get("accept").and_then(|v| v.to_str().ok());
                assert_eq!(accept, Some(ACCEPT), "one versioned Accept header");
                match uri.path() {
                    "/api/health/minimal" => (StatusCode::OK, HEALTH_JSON),
                    "/api/osd" => (StatusCode::OK, OSD_JSON),
                    // Rôle sans droit sur les pools, route absente pour le reste.
                    "/api/pool" => (StatusCode::FORBIDDEN, ""),
                    _ => (StatusCode::NOT_FOUND, ""),
                }
            }))
    }

    #[tokio::test]
    async fn interrogation_complete_et_jeton_garde() {
        let logins = Arc::new(AtomicUsize::new(0));
        let address = serve(fake(logins.clone())).await;
        let mut target = cible("ceph", &format!("http://{address}"), &[]);
        target.credential = Credential::UsernamePassword {
            username: "dumbmonit".into(),
            password: "s3cret".into(),
        };
        let collector = CephCollector::new();
        let s = collector.probe(&target).await.unwrap();
        assert_eq!(value(&s, "ceph_osds_down", &[]), 2.0);
        assert!(find(&s, "ceph_pools", &[]).is_none(), "refused optional read");
        collector.probe(&target).await.unwrap();
        assert_eq!(logins.load(Ordering::SeqCst), 1, "the token is kept");
    }

    #[tokio::test]
    async fn mauvais_mot_de_passe_est_une_erreur_de_configuration() {
        let address = serve(fake(Arc::new(AtomicUsize::new(0)))).await;
        let mut target = cible("ceph", &format!("http://{address}"), &[]);
        target.credential =
            Credential::UsernamePassword { username: "dumbmonit".into(), password: "wrong".into() };
        let error = CephCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
        assert!(!error.means_down());

        target.credential = Credential::None;
        let error = CephCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
    }
}
