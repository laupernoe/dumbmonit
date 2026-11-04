//! Réponses de l'API Pi-hole et mesures qu'on en tire.
//!
//! Tout ici est pur : des structures déjà désérialisées en entrée, des
//! échantillons en sortie. Les compteurs de requêtes de `/api/stats/summary`
//! ne sont **pas** des compteurs au sens Prometheus : FTL les calcule sur les
//! dernières 24 heures glissantes (`maxlogage`), ils montent et descendent. Ils
//! sont donc écrits en jauges, suffixés `_24h`.

use std::collections::BTreeMap;

use dumbmonit_proto::{MetricKind, Sample};
use serde::Deserialize;

// ------------------------------------------------------------------ modèle

/// `GET /api/stats/summary`.
#[derive(Debug, Deserialize)]
pub struct Summary {
    pub queries: Queries,
    #[serde(default)]
    pub clients: Option<Clients>,
    #[serde(default)]
    pub gravity: Option<Gravity>,
}

#[derive(Debug, Deserialize)]
pub struct Queries {
    pub total: f64,
    pub blocked: f64,
    pub percent_blocked: f64,
    #[serde(default)]
    pub unique_domains: Option<f64>,
    #[serde(default)]
    pub forwarded: Option<f64>,
    #[serde(default)]
    pub cached: Option<f64>,
    /// Requêtes par seconde, moyenne sur la dernière minute.
    #[serde(default)]
    pub frequency: Option<f64>,
    /// Réponses par type (`NXDOMAIN`, `SERVFAIL`…), sur 24 heures.
    #[serde(default)]
    pub replies: BTreeMap<String, f64>,
}

#[derive(Debug, Deserialize)]
pub struct Clients {
    #[serde(default)]
    pub active: Option<f64>,
    #[serde(default)]
    pub total: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct Gravity {
    /// Négatif quand la liste n'a pas pu être lue (base gravity absente).
    #[serde(default)]
    pub domains_being_blocked: Option<f64>,
    /// Horodatage Unix de la dernière reconstruction ; 0 si jamais.
    #[serde(default)]
    pub last_update: Option<f64>,
}

/// `GET /api/dns/blocking`.
#[derive(Debug, Deserialize)]
pub struct Blocking {
    /// `enabled`, `disabled`, `failed` ou `unknown`.
    pub blocking: String,
    /// Secondes avant le retour automatique à l'état inverse, s'il y a une minuterie.
    #[serde(default)]
    pub timer: Option<f64>,
}

/// `GET /api/info/version`.
#[derive(Debug, Deserialize)]
pub struct VersionReply {
    pub version: Versions,
}

#[derive(Debug, Default, Deserialize)]
pub struct Versions {
    #[serde(default)]
    pub core: Option<Component>,
    #[serde(default)]
    pub web: Option<Component>,
    #[serde(default)]
    pub ftl: Option<Component>,
    #[serde(default)]
    pub docker: Option<DockerVersion>,
}

#[derive(Debug, Default, Deserialize)]
pub struct Component {
    #[serde(default)]
    pub local: Option<Build>,
    #[serde(default)]
    pub remote: Option<Build>,
}

#[derive(Debug, Default, Deserialize)]
pub struct Build {
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub hash: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct DockerVersion {
    #[serde(default)]
    pub local: Option<String>,
    #[serde(default)]
    pub remote: Option<String>,
}

/// `GET /api/info/messages` : le « Pi-hole diagnosis » de l'interface.
#[derive(Debug, Deserialize)]
pub struct Messages {
    #[serde(default)]
    pub messages: Vec<Message>,
}

#[derive(Debug, Deserialize)]
pub struct Message {
    #[serde(rename = "type")]
    pub kind: String,
}

/// `GET /api/info/ftl`.
#[derive(Debug, Deserialize)]
pub struct FtlReply {
    pub ftl: Ftl,
}

#[derive(Debug, Deserialize)]
pub struct Ftl {
    #[serde(default)]
    pub uptime: Option<f64>,
    #[serde(default, rename = "%mem")]
    pub memory_percent: Option<f64>,
    #[serde(default, rename = "%cpu")]
    pub cpu_percent: Option<f64>,
    #[serde(default)]
    pub privacy_level: Option<f64>,
}

/// `GET /api/stats/upstreams`.
#[derive(Debug, Deserialize)]
pub struct UpstreamsReply {
    #[serde(default)]
    pub upstreams: Vec<Upstream>,
}

#[derive(Debug, Deserialize)]
pub struct Upstream {
    #[serde(default)]
    pub ip: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    /// -1 pour les deux entrées virtuelles `blocklist` et `cache`.
    #[serde(default)]
    pub port: Option<f64>,
    #[serde(default)]
    pub count: Option<f64>,
    #[serde(default)]
    pub statistics: Option<UpstreamStatistics>,
}

#[derive(Debug, Deserialize)]
pub struct UpstreamStatistics {
    /// Temps de réponse moyen, en secondes.
    #[serde(default)]
    pub response: Option<f64>,
}

/// `GET /api/lists` : les listes de blocage et d'autorisation.
#[derive(Debug, Deserialize)]
pub struct ListsReply {
    #[serde(default)]
    pub lists: Vec<List>,
}

#[derive(Debug, Deserialize)]
pub struct List {
    #[serde(default)]
    pub enabled: bool,
    /// Dernier téléchargement de gravity : 0 inconnu, 1 téléchargée, 2 inchangée,
    /// 3 injoignable (copie locale employée), 4 injoignable sans copie.
    #[serde(default)]
    pub status: Option<f64>,
    #[serde(default)]
    pub invalid_domains: Option<f64>,
}

/// `GET /api/dhcp/leases`.
#[derive(Debug, Deserialize)]
pub struct LeasesReply {
    #[serde(default)]
    pub leases: Vec<serde_json::Value>,
}

// ------------------------------------------------------------------ mesures

fn gauge(name: &str, value: f64, ts_ms: i64) -> Sample {
    Sample::new(name, value, MetricKind::Gauge, ts_ms)
}

fn flag(value: bool) -> f64 {
    if value { 1.0 } else { 0.0 }
}

fn push(out: &mut Vec<Sample>, name: &str, value: Option<f64>, ts_ms: i64) {
    if let Some(value) = value.filter(|v| v.is_finite()) {
        out.push(gauge(name, value, ts_ms));
    }
}

pub fn summary_samples(summary: &Summary, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let q = &summary.queries;
    push(&mut out, "pihole_queries_24h", Some(q.total), ts_ms);
    push(&mut out, "pihole_queries_blocked_24h", Some(q.blocked), ts_ms);
    push(&mut out, "pihole_queries_blocked_percent", Some(q.percent_blocked), ts_ms);
    push(&mut out, "pihole_queries_forwarded_24h", q.forwarded, ts_ms);
    push(&mut out, "pihole_queries_cached_24h", q.cached, ts_ms);
    push(&mut out, "pihole_unique_domains_24h", q.unique_domains, ts_ms);
    push(&mut out, "pihole_queries_per_second", q.frequency, ts_ms);
    // Un résolveur amont qui ne répond plus se voit d'abord ici : les
    // SERVFAIL grimpent pendant que le blocage, lui, continue de marcher.
    push(&mut out, "pihole_replies_servfail_24h", q.replies.get("SERVFAIL").copied(), ts_ms);
    if let Some(clients) = &summary.clients {
        push(&mut out, "pihole_clients_active", clients.active, ts_ms);
        push(&mut out, "pihole_clients_total", clients.total, ts_ms);
    }
    if let Some(gravity) = &summary.gravity {
        push(
            &mut out,
            "pihole_gravity_domains",
            gravity.domains_being_blocked.filter(|n| *n >= 0.0),
            ts_ms,
        );
        if let Some(last) = gravity.last_update.filter(|ts| *ts > 0.0) {
            out.push(gauge("pihole_gravity_last_update_timestamp_seconds", last, ts_ms));
            let now = ts_ms as f64 / 1000.0;
            out.push(gauge("pihole_gravity_age_seconds", (now - last).max(0.0), ts_ms));
        }
    }
    out
}

pub fn blocking_samples(blocking: &Blocking, ts_ms: i64) -> Vec<Sample> {
    let mut out =
        vec![gauge("pihole_blocking_enabled", flag(blocking.blocking == "enabled"), ts_ms)];
    push(&mut out, "pihole_blocking_timer_seconds", blocking.timer.filter(|t| *t > 0.0), ts_ms);
    out
}

fn text(value: &Option<String>) -> Option<&str> {
    value.as_deref().map(str::trim).filter(|v| !v.is_empty() && *v != "null")
}

/// Vrai, faux, ou inconnu tant que Pi-hole n'a pas encore interrogé GitHub
/// (la vérification tourne une fois par jour).
fn component_outdated(component: &Component) -> Option<bool> {
    let local = component.local.as_ref()?;
    let remote = component.remote.as_ref()?;
    // Hors de la branche `master`, Pi-hole compare les empreintes de commit :
    // les numéros de version n'avancent pas sur une branche de développement.
    let on_master = text(&local.branch).is_none_or(|branch| branch == "master");
    if on_master {
        Some(text(&local.version)? != text(&remote.version)?)
    } else {
        Some(text(&local.hash)? != text(&remote.hash)?)
    }
}

pub fn version_samples(versions: &Versions, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let mut info = gauge("pihole_version_info", 1.0, ts_ms);
    let components = [("core", &versions.core), ("web", &versions.web), ("ftl", &versions.ftl)];
    for (name, component) in components {
        if let Some(version) =
            component.as_ref().and_then(|c| c.local.as_ref()).and_then(|b| text(&b.version))
        {
            info = info.with_label(name, version);
        }
    }
    let docker = versions.docker.as_ref();
    if let Some(tag) = docker.and_then(|d| text(&d.local)) {
        info = info.with_label("docker", tag);
    }
    out.push(info);

    let mut flags = Vec::new();
    for (name, component) in components {
        if let Some(outdated) = component.as_ref().and_then(component_outdated) {
            flags.push((name, outdated));
        }
    }
    let docker_outdated = docker.and_then(|d| Some(text(&d.local)? != text(&d.remote)?));
    if let Some(outdated) = docker_outdated {
        flags.push(("docker", outdated));
    }
    for (name, outdated) in &flags {
        out.push(
            gauge("pihole_update_available", flag(*outdated), ts_ms).with_label("component", *name),
        );
    }
    // Ce qu'il y a à faire : dans un conteneur, seule l'image se met à jour —
    // core, web et FTL suivent avec elle ; ailleurs, `pihole -up` les prend tous.
    let actionable = if docker.and_then(|d| text(&d.local)).is_some() {
        docker_outdated.map(flag)
    } else if flags.is_empty() {
        None
    } else {
        Some(flags.iter().filter(|(_, outdated)| *outdated).count() as f64)
    };
    push(&mut out, "pihole_updates_available", actionable, ts_ms);
    out
}

pub fn message_samples(messages: &Messages, ts_ms: i64) -> Vec<Sample> {
    let mut out = vec![gauge("pihole_messages", messages.messages.len() as f64, ts_ms)];
    // Les types forment une liste fermée d'une quinzaine d'entrées (`LOAD`,
    // `RATE_LIMIT`, `GRAVITY_RESTORED`…) : la cardinalité reste bornée.
    let mut by_type: BTreeMap<&str, f64> = BTreeMap::new();
    for message in &messages.messages {
        *by_type.entry(message.kind.as_str()).or_default() += 1.0;
    }
    for (kind, count) in by_type {
        out.push(gauge("pihole_messages_by_type", count, ts_ms).with_label("type", kind));
    }
    out
}

pub fn ftl_samples(ftl: &Ftl, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    push(&mut out, "pihole_ftl_uptime_seconds", ftl.uptime, ts_ms);
    push(&mut out, "pihole_ftl_memory_percent", ftl.memory_percent, ts_ms);
    push(&mut out, "pihole_ftl_cpu_percent", ftl.cpu_percent, ts_ms);
    push(&mut out, "pihole_privacy_level", ftl.privacy_level, ts_ms);
    out
}

/// Au plus autant de résolveurs amont portent une série nommée.
pub const MAX_UPSTREAMS: usize = 8;

/// Les résolveurs réellement interrogés : `blocklist` et `cache` sont des
/// entrées virtuelles (port -1) que Pi-hole range avec eux.
pub fn upstream_samples(reply: &UpstreamsReply, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let resolvers = reply
        .upstreams
        .iter()
        .filter(|u| u.port.is_none_or(|port| port >= 0.0))
        .filter(|u| !matches!(text(&u.ip), Some("blocklist" | "cache")))
        .take(MAX_UPSTREAMS);
    for upstream in resolvers {
        let Some(ip) = text(&upstream.ip) else { continue };
        let name = match upstream.port {
            Some(port) => format!("{ip}:{port}"),
            None => ip.to_string(),
        };
        let count = upstream.count.filter(|n| n.is_finite());
        if let Some(count) = count {
            out.push(
                gauge("pihole_upstream_queries_24h", count, ts_ms)
                    .with_label("upstream", name.clone()),
            );
        }
        // Sans requête, la moyenne vaut 0 : ce n'est pas un résolveur rapide.
        let used = count.is_some_and(|n| n > 0.0);
        let response = upstream.statistics.as_ref().and_then(|s| s.response);
        if let Some(response) = response.filter(|r| used && r.is_finite()) {
            out.push(
                gauge("pihole_upstream_response_seconds", response, ts_ms)
                    .with_label("upstream", name),
            );
        }
    }
    out
}

/// L'état des listes activées. Sans le champ `status` (Pi-hole 6.0), on ne
/// dit rien des téléchargements plutôt que d'annoncer « zéro échec ».
pub fn list_samples(reply: &ListsReply, ts_ms: i64) -> Vec<Sample> {
    let enabled: Vec<&List> = reply.lists.iter().filter(|l| l.enabled).collect();
    let mut out = vec![gauge("pihole_lists_enabled", enabled.len() as f64, ts_ms)];
    if enabled.iter().any(|l| l.status.is_some()) {
        let failed = enabled.iter().filter(|l| matches!(l.status, Some(s) if s >= 3.0)).count();
        out.push(gauge("pihole_lists_download_failed", failed as f64, ts_ms));
    }
    let invalid: f64 = enabled.iter().filter_map(|l| l.invalid_domains).sum();
    out.push(gauge("pihole_lists_invalid_domains", invalid, ts_ms));
    out
}

/// Les baux DHCP en cours. Rien quand il n'y en a aucun : le serveur DHCP de
/// Pi-hole est éteint chez la plupart des installations.
pub fn dhcp_samples(reply: &LeasesReply, ts_ms: i64) -> Vec<Sample> {
    if reply.leases.is_empty() {
        return Vec::new();
    }
    vec![gauge("pihole_dhcp_leases", reply.leases.len() as f64, ts_ms)]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Réponses d'un Pi-hole 2026.09.0 (core v6.4.3, web v6.6, FTL v6.7.1)
    /// en conteneur, après quelques requêtes dont trois bloquées.
    const SUMMARY: &str = include_str!("testdata/2026.09.0/stats_summary.json");
    const BLOCKING: &str = include_str!("testdata/2026.09.0/dns_blocking.json");
    const BLOCKING_OFF: &str = include_str!("testdata/2026.09.0/dns_blocking_disabled.json");
    const VERSION: &str = include_str!("testdata/2026.09.0/info_version.json");
    const MESSAGES: &str = include_str!("testdata/2026.09.0/info_messages.json");
    const FTL: &str = include_str!("testdata/2026.09.0/info_ftl.json");

    /// 2026-09-30 00:00:00 UTC, quelques heures après la capture.
    const NOW_MS: i64 = 1_790_755_200_000;

    fn value(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    #[test]
    fn resume_reel() {
        let summary: Summary = serde_json::from_str(SUMMARY).unwrap();
        let samples = summary_samples(&summary, NOW_MS);
        assert_eq!(value(&samples, "pihole_queries_24h"), Some(8.0));
        assert_eq!(value(&samples, "pihole_queries_blocked_24h"), Some(3.0));
        assert_eq!(value(&samples, "pihole_queries_blocked_percent"), Some(37.5));
        assert_eq!(value(&samples, "pihole_queries_forwarded_24h"), Some(4.0));
        assert_eq!(value(&samples, "pihole_replies_servfail_24h"), Some(0.0));
        assert_eq!(value(&samples, "pihole_clients_active"), Some(1.0));
        assert_eq!(value(&samples, "pihole_gravity_domains"), Some(74761.0));
        assert_eq!(
            value(&samples, "pihole_gravity_age_seconds"),
            Some(1_790_755_200.0 - 1_790_751_549.0)
        );
        assert!(samples.iter().all(|s| s.kind == MetricKind::Gauge), "fenêtre glissante");
    }

    #[test]
    fn une_liste_jamais_construite_n_a_pas_d_age() {
        let summary: Summary = serde_json::from_str(
            r#"{"queries":{"total":0,"blocked":0,"percent_blocked":0},
                "gravity":{"domains_being_blocked":-2,"last_update":0}}"#,
        )
        .unwrap();
        let samples = summary_samples(&summary, NOW_MS);
        assert!(value(&samples, "pihole_gravity_age_seconds").is_none());
        assert!(value(&samples, "pihole_gravity_domains").is_none());
    }

    #[test]
    fn blocage_actif_puis_suspendu_cinq_minutes() {
        let on: Blocking = serde_json::from_str(BLOCKING).unwrap();
        let samples = blocking_samples(&on, 0);
        assert_eq!(value(&samples, "pihole_blocking_enabled"), Some(1.0));
        assert!(value(&samples, "pihole_blocking_timer_seconds").is_none());

        let off: Blocking = serde_json::from_str(BLOCKING_OFF).unwrap();
        let samples = blocking_samples(&off, 0);
        assert_eq!(value(&samples, "pihole_blocking_enabled"), Some(0.0));
        assert_eq!(value(&samples, "pihole_blocking_timer_seconds"), Some(300.0));

        let failed: Blocking = serde_json::from_str(r#"{"blocking":"failed"}"#).unwrap();
        assert_eq!(value(&blocking_samples(&failed, 0), "pihole_blocking_enabled"), Some(0.0));
    }

    #[test]
    fn versions_a_jour_en_conteneur() {
        let reply: VersionReply = serde_json::from_str(VERSION).unwrap();
        let samples = version_samples(&reply.version, 0);
        let info = samples.iter().find(|s| s.metric == "pihole_version_info").unwrap();
        assert_eq!(info.labels["core"], "v6.4.3");
        assert_eq!(info.labels["web"], "v6.6");
        assert_eq!(info.labels["ftl"], "v6.7.1");
        assert_eq!(info.labels["docker"], "2026.09.0");
        assert_eq!(value(&samples, "pihole_updates_available"), Some(0.0));
        let flags = samples.iter().filter(|s| s.metric == "pihole_update_available").count();
        assert_eq!(flags, 4);
    }

    #[test]
    fn une_image_plus_recente_compte_une_seule_mise_a_jour() {
        let reply: VersionReply = serde_json::from_str(
            &VERSION
                .replace(r#""remote": "2026.09.0""#, r#""remote": "2026.10.0""#)
                .replace(r#""version": "v6.7.1""#, r#""version": "v6.8""#),
        )
        .unwrap();
        let samples = version_samples(&reply.version, 0);
        assert_eq!(value(&samples, "pihole_updates_available"), Some(1.0));
    }

    #[test]
    fn hors_conteneur_chaque_composant_compte() {
        let reply: VersionReply = serde_json::from_str(
            r#"{"version":{
                "core":{"local":{"version":"v6.4.2","branch":"master"},"remote":{"version":"v6.4.3"}},
                "web":{"local":{"version":"v6.6","branch":"master"},"remote":{"version":"v6.6"}},
                "ftl":{"local":{"version":"v6.7.1","branch":"development","hash":"aaa"},
                       "remote":{"version":"v6.7.1","hash":"bbb"}},
                "docker":{"local":null,"remote":null}}}"#,
        )
        .unwrap();
        let samples = version_samples(&reply.version, 0);
        assert_eq!(value(&samples, "pihole_updates_available"), Some(2.0));
        assert!(
            !samples
                .iter()
                .any(|s| s.labels.get("component").map(String::as_str) == Some("docker"))
        );
    }

    #[test]
    fn verification_pas_encore_faite() {
        let reply: VersionReply = serde_json::from_str(
            r#"{"version":{"core":{"local":{"version":"v6.4.3"},"remote":{"version":null}}}}"#,
        )
        .unwrap();
        let samples = version_samples(&reply.version, 0);
        assert!(value(&samples, "pihole_updates_available").is_none());
        assert!(value(&samples, "pihole_version_info").is_some());
    }

    #[test]
    fn diagnostic_et_ftl_reels() {
        let messages: Messages = serde_json::from_str(MESSAGES).unwrap();
        let samples = message_samples(&messages, 0);
        assert_eq!(value(&samples, "pihole_messages"), Some(1.0));
        let load = samples.iter().find(|s| s.metric == "pihole_messages_by_type").unwrap();
        assert_eq!(load.labels["type"], "LOAD");

        let ftl: FtlReply = serde_json::from_str(FTL).unwrap();
        let samples = ftl_samples(&ftl.ftl, 0);
        assert!(value(&samples, "pihole_ftl_uptime_seconds").unwrap() > 0.0);
        assert_eq!(value(&samples, "pihole_privacy_level"), Some(0.0));
    }

    #[test]
    fn les_resolveurs_amont_sans_les_entrees_virtuelles() {
        let reply: UpstreamsReply =
            serde_json::from_str(include_str!("testdata/2026.09.0/stats_upstreams.json")).unwrap();
        let samples = upstream_samples(&reply, 0);
        let named: Vec<&Sample> =
            samples.iter().filter(|s| s.metric == "pihole_upstream_response_seconds").collect();
        // `blocklist` et `cache` écartés ; un résolveur sans requête n'a pas de temps.
        assert_eq!(named.len(), 1);
        assert_eq!(named[0].labels["upstream"], "127.0.0.1:5335");
        assert_eq!(named[0].value, 0.0231);
        let counts = samples.iter().filter(|s| s.metric == "pihole_upstream_queries_24h").count();
        assert_eq!(counts, 2);
    }

    #[test]
    fn les_listes_en_echec_sont_comptees_parmi_les_actives() {
        let reply: ListsReply =
            serde_json::from_str(include_str!("testdata/2026.09.0/lists.json")).unwrap();
        let samples = list_samples(&reply, 0);
        assert_eq!(value(&samples, "pihole_lists_enabled"), Some(3.0));
        // Statuts 3 et 4 ; la liste désactivée (statut 4) ne compte pas.
        assert_eq!(value(&samples, "pihole_lists_download_failed"), Some(2.0));
        assert_eq!(value(&samples, "pihole_lists_invalid_domains"), Some(2.0));

        let old: ListsReply =
            serde_json::from_str(r#"{"lists":[{"enabled":true,"invalid_domains":0}]}"#).unwrap();
        assert!(value(&list_samples(&old, 0), "pihole_lists_download_failed").is_none());
    }

    #[test]
    fn les_baux_dhcp_ne_sont_publies_que_s_il_y_en_a() {
        let reply: LeasesReply =
            serde_json::from_str(include_str!("testdata/2026.09.0/dhcp_leases.json")).unwrap();
        assert_eq!(value(&dhcp_samples(&reply, 0), "pihole_dhcp_leases"), Some(2.0));
        let empty: LeasesReply = serde_json::from_str(r#"{"leases":[]}"#).unwrap();
        assert!(dhcp_samples(&empty, 0).is_empty());
    }
}
