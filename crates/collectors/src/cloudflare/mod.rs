//! Cloudflare, par l'API publique (`https://api.cloudflare.com/client/v4`).
//!
//! Un jeton d'API en lecture seule suffit. Ce que le module regarde, avec ce
//! qu'il faut au jeton :
//!
//! * **les zones** (`Zone → Zone → Read`) : une zone qui n'est plus `active`
//!   (en attente de délégation DNS, déplacée, suspendue) ne protège plus rien ;
//! * **les tunnels** (`Account → Cloudflare Tunnel → Read`) : `healthy`,
//!   `degraded`, `down` ou `inactive` (jamais lancé) ;
//! * **les certificats** (`Zone → SSL and Certificates → Read`) : la date
//!   d'expiration de chaque lot de certificats actif de chaque zone ;
//! * **le trafic** (`Zone → Analytics → Read`, GraphQL) : requêtes et réponses
//!   5xx de la dernière heure complète et de l'heure en cours, par zone, et les
//!   menaces arrêtées par pays d'origine (`countryMap`), toutes zones réunies,
//!   pour la carte des menaces. Les pays sont lus par une requête à part : si
//!   le schéma ou l'offre la refuse, seul le relevé des menaces manque ;
//! * **les répartiteurs de charge** (`Account → Load Balancing: Monitors And
//!   Pools → Read`) : santé de chaque groupe d'origines (« pool »).
//!
//! Seule la liste des zones est indispensable : un jeton qui n'a pas l'un des
//! autres droits (ou une offre qui n'inclut pas la fonction) n'empêche pas la
//! sonde, la série `cloudflare_<partie>_readable` passe simplement à 0 et les
//! séries de cette partie sont omises.
//!
//! C'est une API sur Internet : l'adresse de la cible est celle de l'API, pas
//! d'un appareil, et le garde-fou des adresses privées ne la concerne pas. Les
//! appels par zone sont bornés (`MAX_ZONE_CALLS`) et menés cinq à la fois pour
//! rester loin de la limite de 1 200 requêtes par 5 minutes de Cloudflare.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `account_id` | vide | Compte des tunnels et des pools ; vide = déduit de `/accounts`. |
//! | `analytics` | `true` | Lire le trafic par zone (appel GraphQL facultatif). |
//! | `request_timeout_seconds` | `15` | Délai par requête HTTP. |

use std::collections::BTreeMap;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};
use reqwest::StatusCode;
use serde_json::{Value, json};

use crate::cloud::{figure, item, url_segment};
use crate::rest::{MAX_NAMED, RestClient, flag, gauge, now_ms, number, seconds_until, text};
use crate::selfhosted::options::{parse_bool, tag};

pub const DEFAULT_PORT: u16 = 443;
pub const DEFAULT_ADDRESS: &str = "api.cloudflare.com";

const API: &str = "/client/v4";
/// Pages de 50 éléments lues au plus par liste.
const MAX_PAGES: usize = 20;
const MAX_ACCOUNTS: usize = 5;
/// Zones sur lesquelles on fait des appels propres (certificats, trafic, LB).
const MAX_ZONE_CALLS: usize = 50;
/// Zones par requête GraphQL (limite de zones par requête des offres basses).
const ANALYTICS_CHUNK: usize = 10;
/// Sous ce nombre de requêtes, un pourcentage d'erreurs 5xx n'a pas de sens.
const MIN_REQUESTS: f64 = 100.0;

const TRAFFIC_QUERY: &str = "query ($zones: [string!], $since: Time!, $until: Time!) { \
viewer { zones(filter: {zoneTag_in: $zones}) { zoneTag \
httpRequests1hGroups(limit: 10, filter: {datetime_geq: $since, datetime_lt: $until}) { \
sum { requests responseStatusMap { edgeResponseStatus requests } } } } } }";

/// Menaces par pays, sur le même jeu de données (`httpRequests1hGroups`,
/// `sum.countryMap`), interrogées à part : une erreur GraphQL sur ce champ rend
/// `data: null` pour toute la requête, et ne doit pas emporter le trafic.
const THREATS_QUERY: &str = "query ($zones: [string!], $since: Time!, $until: Time!) { \
viewer { zones(filter: {zoneTag_in: $zones}) { zoneTag \
httpRequests1hGroups(limit: 10, filter: {datetime_geq: $since, datetime_lt: $until}) { \
sum { countryMap { clientCountryName threats } } } } } }";

#[derive(Default)]
pub struct CloudflareCollector;

impl CloudflareCollector {
    pub fn new() -> Self {
        Self
    }
}

/// Réglages lus sur la cible.
struct Settings {
    account_id: Option<String>,
    analytics: bool,
}

impl Settings {
    fn read(target: &Target) -> Result<Self, ProbeError> {
        let account_id = tag(target, "account_id").map(|v| url_segment("account_id", v));
        let analytics = match tag(target, "analytics") {
            None => true,
            Some(raw) => parse_bool(Some(raw))?,
        };
        Ok(Self { account_id: account_id.transpose()?, analytics })
    }
}

fn build_client(target: &Target) -> Result<RestClient, ProbeError> {
    let mut client = RestClient::for_target(target, "https", DEFAULT_PORT, "Cloudflare")?;
    match &target.credential {
        Credential::ApiToken { token } if !token.trim().is_empty() => {
            client.set_header("Authorization", format!("Bearer {}", token.trim()));
            Ok(client)
        }
        other => {
            Err(ProbeError::Config(format!("Cloudflare expects an API token, configured: {other}")))
        }
    }
}

fn first_error(body: &str) -> (String, Option<u64>) {
    let reply: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    let message = reply.pointer("/errors/0/message").and_then(Value::as_str).unwrap_or_default();
    (message.to_string(), reply.pointer("/errors/0/code").and_then(Value::as_u64))
}

fn with_detail(message: &str, detail: &str) -> String {
    if detail.is_empty() { message.to_string() } else { format!("{message} ({detail})") }
}

/// Lit l'enveloppe `{success, errors, result}` de Cloudflare et traduit les
/// refus en erreurs typées.
fn envelope(
    client: &RestClient,
    status: StatusCode,
    body: &str,
    path: &str,
) -> Result<Value, ProbeError> {
    if status.is_success() {
        let reply: Value = client.decode(body, path)?;
        if reply.get("success").and_then(Value::as_bool) == Some(false) {
            let (detail, _) = first_error(body);
            return Err(ProbeError::Protocol(with_detail(
                &format!("Cloudflare reported an error on {path}"),
                &detail,
            )));
        }
        return Ok(reply);
    }
    let (detail, code) = first_error(body);
    match status.as_u16() {
        401 => Err(ProbeError::Auth(with_detail(
            "Cloudflare refused the API token (401): it is wrong, expired or revoked.",
            &detail,
        ))),
        // 6003 / 6111 : en-tête d'authentification mal formé ; 9106 / 9109 : jeton inconnu.
        400 if matches!(code, Some(6003 | 6111 | 9106 | 9109)) => {
            Err(ProbeError::Auth(with_detail(
                "Cloudflare could not read the API token: check that it was copied whole.",
                &detail,
            )))
        }
        403 => Err(ProbeError::Auth(with_detail(
            &format!(
                "Cloudflare refused {path} (403): give the API token the matching read \
                 permission, and check its client IP address filtering."
            ),
            &detail,
        ))),
        404 => Err(ProbeError::Config(with_detail(
            &format!("Cloudflare has no resource at {path}: check the account ID."),
            &detail,
        ))),
        _ => Err(client.status_error(status, body, path)),
    }
}

async fn call(client: &RestClient, path: &str) -> Result<Value, ProbeError> {
    let (status, body) = client.get(path).await?;
    envelope(client, status, &body, path)
}

/// Un appel dont la réponse reste associée au nom qui l'a demandé.
async fn call_named<'a>(
    client: &RestClient,
    name: &'a str,
    path: String,
) -> (&'a str, Result<Value, ProbeError>) {
    (name, call(client, &path).await)
}

/// Le tableau `result` d'une réponse, vide s'il manque.
fn results(reply: &Value) -> &[Value] {
    reply.get("result").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

/// Lit une liste paginée (`per_page=50`), au plus `MAX_PAGES` pages.
async fn list(client: &RestClient, path: &str) -> Result<Vec<Value>, ProbeError> {
    let separator = if path.contains('?') { '&' } else { '?' };
    let mut out = Vec::new();
    for page in 1..=MAX_PAGES {
        let reply = call(client, &format!("{path}{separator}per_page=50&page={page}")).await?;
        out.extend_from_slice(results(&reply));
        let total = reply.pointer("/result_info/total_pages").and_then(Value::as_u64).unwrap_or(1);
        if page as u64 >= total {
            break;
        }
    }
    Ok(out)
}

fn capital(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn flag_bool(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}

fn readable(part: &str, ok: bool, ts_ms: i64) -> Sample {
    flag(&format!("cloudflare_{part}_readable"), ok, ts_ms)
}

pub fn zone_samples(zones: &[Value], ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let (mut active, mut paused) = (0usize, 0usize);
    for (index, zone) in zones.iter().enumerate() {
        let status = text(zone, "status").unwrap_or("unknown");
        let is_paused = flag_bool(zone, "paused").unwrap_or(false);
        active += usize::from(status == "active");
        paused += usize::from(is_paused);
        if index >= MAX_NAMED {
            continue;
        }
        let Some(name) = text(zone, "name") else { continue };
        let plan = zone.pointer("/plan/name").and_then(Value::as_str).unwrap_or_default();
        let named = |sample: Sample| sample.with_label("zone", name);
        out.push(named(flag("cloudflare_zone_active", status == "active", ts_ms)));
        out.push(named(flag("cloudflare_zone_paused", is_paused, ts_ms)));
        out.push(
            named(gauge("cloudflare_zone_info", 1.0, ts_ms))
                .with_label("status", status)
                .with_label("plan", plan),
        );
        let detail = if is_paused { "Paused".to_string() } else { capital(status) };
        out.push(item("cloudflare", "Zones", name, status == "active", &detail, ts_ms));
    }
    out.push(gauge("cloudflare_zones", zones.len() as f64, ts_ms));
    out.push(gauge("cloudflare_zones_active", active as f64, ts_ms));
    out.push(gauge("cloudflare_zones_paused", paused as f64, ts_ms));
    out.push(figure("cloudflare", "Zones", "", zones.len() as f64, ts_ms));
    out
}

pub fn tunnel_samples(tunnels: &[Value], ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let (mut total, mut healthy, mut degraded, mut down) = (0usize, 0usize, 0usize, 0usize);
    for tunnel in tunnels {
        // Un tunnel supprimé garde parfois sa fiche : il n'existe plus.
        if tunnel.get("deleted_at").is_some_and(|v| !v.is_null()) {
            continue;
        }
        total += 1;
        let status = text(tunnel, "status").unwrap_or("unknown").to_ascii_lowercase();
        healthy += usize::from(status == "healthy");
        degraded += usize::from(status == "degraded");
        down += usize::from(status == "down");
        if total > MAX_NAMED {
            continue;
        }
        let Some(name) = text(tunnel, "name") else { continue };
        let connections =
            tunnel.get("connections").and_then(Value::as_array).map_or(0, Vec::len) as f64;
        let named = |sample: Sample| sample.with_label("tunnel", name);
        out.push(named(flag("cloudflare_tunnel_healthy", status == "healthy", ts_ms)));
        out.push(named(flag("cloudflare_tunnel_degraded", status == "degraded", ts_ms)));
        out.push(named(flag("cloudflare_tunnel_down", status == "down", ts_ms)));
        out.push(named(gauge("cloudflare_tunnel_connections", connections, ts_ms)));
        let ok = status != "down" && status != "degraded";
        out.push(item("cloudflare", "Tunnels", name, ok, &capital(&status), ts_ms));
    }
    out.push(gauge("cloudflare_tunnels", total as f64, ts_ms));
    out.push(gauge("cloudflare_tunnels_healthy", healthy as f64, ts_ms));
    out.push(gauge("cloudflare_tunnels_degraded", degraded as f64, ts_ms));
    out.push(gauge("cloudflare_tunnels_down", down as f64, ts_ms));
    out.push(figure("cloudflare", "Tunnels", "", total as f64, ts_ms));
    out
}

/// Expiration des lots de certificats actifs d'une zone : une série par nom
/// d'hôte, la plus proche si plusieurs lots couvrent le même.
pub fn certificate_samples(zone: &str, packs: &[Value], ts_ms: i64) -> Vec<Sample> {
    let mut expiry: BTreeMap<String, f64> = BTreeMap::new();
    for pack in packs {
        if text(pack, "status") != Some("active") {
            continue;
        }
        let host = pack
            .get("hosts")
            .and_then(Value::as_array)
            .and_then(|hosts| hosts.first())
            .and_then(Value::as_str)
            .or_else(|| text(pack, "id"));
        let Some(host) = host else { continue };
        let nearest = pack
            .get("certificates")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|certificate| text(certificate, "expires_on"))
            .filter_map(|date| seconds_until(date, ts_ms))
            .reduce(f64::min)
            .or_else(|| text(pack, "expires_on").and_then(|date| seconds_until(date, ts_ms)));
        let Some(left) = nearest else { continue };
        expiry.entry(host.to_string()).and_modify(|known| *known = known.min(left)).or_insert(left);
    }
    let mut out = Vec::new();
    for (host, left) in expiry.into_iter().take(MAX_NAMED) {
        out.push(
            gauge("cloudflare_certificate_expiry_seconds", left, ts_ms)
                .with_label("zone", zone)
                .with_label("host", host.as_str()),
        );
        let (ok, detail) = if left < 0.0 {
            (false, "Expired")
        } else if left < 14.0 * 86_400.0 {
            (false, "Expires within 14 days")
        } else {
            (true, "Valid")
        };
        out.push(item(
            "cloudflare",
            "Certificates",
            &format!("{host} ({zone})"),
            ok,
            detail,
            ts_ms,
        ));
    }
    out
}

/// Trafic de la fenêtre lue : `None` quand la réponse n'a pas la forme attendue
/// (erreurs GraphQL, zones refusées).
pub fn traffic_samples(
    reply: &Value,
    zones: &[(String, String)],
    ts_ms: i64,
) -> Option<Vec<Sample>> {
    let entries = reply.pointer("/data/viewer/zones").and_then(Value::as_array)?;
    let mut out = Vec::new();
    for entry in entries {
        let Some(zone_tag) = text(entry, "zoneTag") else { continue };
        let Some((_, name)) = zones.iter().find(|(id, _)| id == zone_tag) else { continue };
        let (mut requests, mut errors) = (0.0, 0.0);
        for group in
            entry.get("httpRequests1hGroups").and_then(Value::as_array).into_iter().flatten()
        {
            requests += number(group.pointer("/sum/requests")).unwrap_or(0.0);
            for status in group
                .pointer("/sum/responseStatusMap")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let code = number(status.get("edgeResponseStatus")).unwrap_or(0.0);
                if (500.0..600.0).contains(&code) {
                    errors += number(status.get("requests")).unwrap_or(0.0);
                }
            }
        }
        let named = |sample: Sample| sample.with_label("zone", name.as_str());
        out.push(named(gauge("cloudflare_zone_http_requests", requests, ts_ms)));
        out.push(named(gauge("cloudflare_zone_http_5xx", errors, ts_ms)));
        if requests >= MIN_REQUESTS {
            out.push(named(gauge(
                "cloudflare_zone_http_5xx_percent",
                errors / requests * 100.0,
                ts_ms,
            )));
        }
    }
    Some(out)
}

/// Menaces par pays de la réponse à `THREATS_QUERY`, une série par zone et par
/// pays (`merge_country_threats` les additionne ensuite) ; `None` quand la
/// réponse n'a pas la forme attendue.
pub fn threat_samples(
    reply: &Value,
    zones: &[(String, String)],
    ts_ms: i64,
) -> Option<Vec<Sample>> {
    let entries = reply.pointer("/data/viewer/zones").and_then(Value::as_array)?;
    let mut out = Vec::new();
    for entry in entries {
        let Some(zone_tag) = text(entry, "zoneTag") else { continue };
        if !zones.iter().any(|(id, _)| id == zone_tag) {
            continue;
        }
        for (country, count) in country_threats(entry) {
            let sample = gauge("cloudflare_country_threats", count, ts_ms);
            out.push(sample.with_label("country", country));
        }
    }
    Some(out)
}

/// Menaces d'une zone par pays d'origine (codes ISO alpha-2 ; `XX` inconnu et
/// `T1` Tor laissés de côté), sur les groupes horaires lus.
fn country_threats(entry: &Value) -> BTreeMap<String, f64> {
    let mut by_country = BTreeMap::new();
    let groups = entry.get("httpRequests1hGroups").and_then(Value::as_array).into_iter().flatten();
    for group in groups {
        let map = group.pointer("/sum/countryMap").and_then(Value::as_array).into_iter().flatten();
        for row in map {
            let code = text(row, "clientCountryName").unwrap_or_default().to_ascii_uppercase();
            let threats = number(row.get("threats")).unwrap_or(0.0);
            let valid = code.len() == 2 && code.bytes().all(|b| b.is_ascii_uppercase());
            if valid && code != "XX" && threats > 0.0 {
                *by_country.entry(code).or_default() += threats;
            }
        }
    }
    by_country
}

/// Additionne les menaces par pays de toutes les zones lues : une série par
/// pays pour la cible, jamais une par zone et par pays.
pub fn merge_country_threats(samples: Vec<Sample>) -> Vec<Sample> {
    let mut threats: BTreeMap<String, (f64, i64)> = BTreeMap::new();
    let mut out = Vec::with_capacity(samples.len());
    for sample in samples {
        if sample.metric == "cloudflare_country_threats" {
            let country = sample.labels.get("country").cloned().unwrap_or_default();
            let slot = threats.entry(country).or_insert((0.0, sample.ts_ms));
            slot.0 += sample.value;
        } else {
            out.push(sample);
        }
    }
    for (country, (count, ts_ms)) in threats {
        out.push(gauge("cloudflare_country_threats", count, ts_ms).with_label("country", country));
    }
    out
}

/// Santé des pools d'origines (un pool désactivé n'est pas une panne).
pub fn pool_samples(pools: &[Value], ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    for (index, pool) in pools.iter().enumerate() {
        if index >= MAX_NAMED {
            break;
        }
        let Some(name) = text(pool, "name") else { continue };
        let enabled = flag_bool(pool, "enabled").unwrap_or(true);
        let origins: Vec<&Value> = pool
            .get("origins")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|origin| flag_bool(origin, "enabled").unwrap_or(true))
            .collect();
        let origins_healthy =
            origins.iter().filter(|origin| flag_bool(origin, "healthy").unwrap_or(false)).count();
        let minimum = number(pool.get("minimum_origins")).unwrap_or(1.0).max(1.0);
        let healthy = flag_bool(pool, "healthy")
            .unwrap_or(!origins.is_empty() && origins_healthy as f64 >= minimum);
        let named = |sample: Sample| sample.with_label("pool", name);
        out.push(named(flag("cloudflare_lb_pool_enabled", enabled, ts_ms)));
        out.push(named(gauge("cloudflare_lb_pool_origins", origins.len() as f64, ts_ms)));
        out.push(named(gauge("cloudflare_lb_pool_origins_healthy", origins_healthy as f64, ts_ms)));
        let detail = if !enabled {
            "Disabled"
        } else if healthy {
            "Healthy"
        } else {
            "Unhealthy"
        };
        if enabled {
            out.push(named(flag("cloudflare_lb_pool_healthy", healthy, ts_ms)));
        }
        out.push(item(
            "cloudflare",
            "Load balancer pools",
            name,
            !enabled || healthy,
            detail,
            ts_ms,
        ));
    }
    out.push(gauge("cloudflare_lb_pools", pools.len() as f64, ts_ms));
    out
}

pub fn load_balancer_samples(zone: &str, balancers: &[Value], ts_ms: i64) -> Vec<Sample> {
    balancers
        .iter()
        .filter_map(|lb| {
            let name = text(lb, "name")?;
            let enabled = flag_bool(lb, "enabled").unwrap_or(true);
            Some(
                flag("cloudflare_load_balancer_enabled", enabled, ts_ms)
                    .with_label("zone", zone)
                    .with_label("load_balancer", name),
            )
        })
        .take(MAX_NAMED)
        .collect()
}

/// Identifiants de comptes distincts, bornés, sans caractère d'URL.
fn distinct_ids<'a>(ids: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for id in ids {
        if id.contains(['/', '?', '#', ' ']) || out.iter().any(|known| known == id) {
            continue;
        }
        out.push(id.to_string());
        if out.len() >= MAX_ACCOUNTS {
            break;
        }
    }
    out
}

/// Comptes dont on lit les tunnels et les pools : l'option, sinon `/accounts`,
/// sinon ceux qui possèdent les zones (un jeton limité aux zones ne lit pas
/// `/accounts`).
async fn account_ids(client: &RestClient, settings: &Settings, zones: &[Value]) -> Vec<String> {
    if let Some(id) = &settings.account_id {
        return vec![id.clone()];
    }
    if let Ok(accounts) = list(client, &format!("{API}/accounts")).await {
        let ids = distinct_ids(accounts.iter().filter_map(|a| text(a, "id")));
        if !ids.is_empty() {
            return ids;
        }
    }
    distinct_ids(zones.iter().filter_map(|z| z.pointer("/account/id").and_then(Value::as_str)))
}

/// Début de la fenêtre lue : l'heure complète précédente ; fin : maintenant.
fn traffic_window(ts_ms: i64) -> (String, String) {
    let secs = ts_ms / 1000;
    let iso = |at: i64| {
        chrono::DateTime::from_timestamp(at, 0)
            .unwrap_or_default()
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    };
    (iso(secs - secs.rem_euclid(3600) - 3600), iso(secs))
}

async fn gather(
    client: &RestClient,
    settings: &Settings,
    zones: &[Value],
    ts_ms: i64,
) -> Result<Vec<Sample>, ProbeError> {
    let mut out = zone_samples(zones, ts_ms);
    // Zones sur lesquelles on fait des appels propres : actives et non suspendues.
    let live: Vec<(String, String)> = zones
        .iter()
        .filter(|z| text(z, "status") == Some("active") && !flag_bool(z, "paused").unwrap_or(false))
        .filter_map(|z| Some((text(z, "id")?.to_string(), text(z, "name")?.to_string())))
        .take(MAX_ZONE_CALLS)
        .collect();
    let accounts = account_ids(client, settings, zones).await;

    // Tunnels.
    let mut tunnels = (!accounts.is_empty()).then(Vec::new);
    for account in &accounts {
        match list(client, &format!("{API}/accounts/{account}/cfd_tunnel?is_deleted=false")).await {
            Ok(found) => {
                if let Some(all) = tunnels.as_mut() {
                    all.extend(found);
                }
            }
            Err(error) => {
                // Un compte saisi à la main qui n'existe pas est une faute de réglage.
                if matches!(error, ProbeError::Config(_)) && settings.account_id.is_some() {
                    return Err(error);
                }
                tunnels = None;
            }
        }
    }
    if let Some(tunnels) = &tunnels {
        out.extend(tunnel_samples(tunnels, ts_ms));
    }
    out.push(readable("tunnels", tunnels.is_some(), ts_ms));

    // Certificats, une requête par zone.
    let packs = futures::future::join_all(live.iter().map(|(id, name)| {
        let path = format!("{API}/zones/{id}/ssl/certificate_packs?status=all");
        call_named(client, name.as_str(), path)
    }))
    .await;
    let mut certificates_ok = true;
    for (zone, reply) in &packs {
        match reply {
            Ok(reply) => out.extend(certificate_samples(zone, results(reply), ts_ms)),
            Err(_) => certificates_ok = false,
        }
    }
    out.push(readable("certificates", certificates_ok, ts_ms));

    // Trafic.
    if settings.analytics && !live.is_empty() {
        let (since, until) = traffic_window(ts_ms);
        let (mut analytics_ok, mut requests, mut errors) = (true, 0.0, 0.0);
        for chunk in live.chunks(ANALYTICS_CHUNK) {
            let ids: Vec<&str> = chunk.iter().map(|(id, _)| id.as_str()).collect();
            let body = json!({
                "query": TRAFFIC_QUERY,
                "variables": {"zones": ids, "since": since, "until": until},
            });
            let parsed = graphql(client, &body).await;
            match parsed.and_then(|reply| traffic_samples(&reply, chunk, ts_ms)) {
                Some(samples) => {
                    for sample in &samples {
                        match sample.metric.as_str() {
                            "cloudflare_zone_http_requests" => requests += sample.value,
                            "cloudflare_zone_http_5xx" => errors += sample.value,
                            _ => {}
                        }
                    }
                    out.extend(samples);
                }
                None => analytics_ok = false,
            }
            // Les pays, à part : un refus ici ne touche ni le trafic ni sa série
            // `analytics_readable`, la carte des menaces reste simplement vide.
            let body = json!({
                "query": THREATS_QUERY,
                "variables": {"zones": ids, "since": since, "until": until},
            });
            let threats = graphql(client, &body).await;
            if let Some(samples) = threats.and_then(|reply| threat_samples(&reply, chunk, ts_ms)) {
                out.extend(samples);
            }
        }
        out = merge_country_threats(out);
        if analytics_ok {
            out.push(figure("cloudflare", "Requests", "", requests, ts_ms));
            out.push(figure("cloudflare", "5xx responses", "", errors, ts_ms));
        }
        out.push(readable("analytics", analytics_ok, ts_ms));
    }

    // Répartiteurs de charge : les zones ne sont interrogées que si des pools existent.
    let mut pools = (!accounts.is_empty()).then(Vec::new);
    for account in &accounts {
        match call(client, &format!("{API}/accounts/{account}/load_balancers/pools")).await {
            Ok(reply) => {
                if let Some(all) = pools.as_mut() {
                    all.extend_from_slice(results(&reply));
                }
            }
            Err(_) => pools = None,
        }
    }
    let mut balancers_ok = pools.is_some();
    if let Some(pools) = &pools {
        out.extend(pool_samples(pools, ts_ms));
        if !pools.is_empty() {
            let found = futures::future::join_all(live.iter().map(|(id, name)| {
                let path = format!("{API}/zones/{id}/load_balancers");
                call_named(client, name.as_str(), path)
            }))
            .await;
            let mut count = 0usize;
            for (zone, reply) in &found {
                match reply {
                    Ok(reply) => {
                        count += results(reply).len();
                        out.extend(load_balancer_samples(zone, results(reply), ts_ms));
                    }
                    Err(_) => balancers_ok = false,
                }
            }
            out.push(gauge("cloudflare_load_balancers", count as f64, ts_ms));
        }
    }
    out.push(readable("lb", balancers_ok, ts_ms));
    Ok(out)
}

/// Une requête GraphQL ; `None` sur un refus HTTP ou une réponse illisible.
async fn graphql(client: &RestClient, body: &Value) -> Option<Value> {
    match client.post_json(&format!("{API}/graphql"), body).await {
        Ok((status, raw)) if status.is_success() => serde_json::from_str::<Value>(&raw).ok(),
        _ => None,
    }
}

async fn zones_of(client: &RestClient) -> Result<Vec<Value>, ProbeError> {
    list(client, &format!("{API}/zones")).await
}

#[async_trait]
impl Collector for CloudflareCollector {
    fn kind(&self) -> &'static str {
        "cloudflare"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let settings = Settings::read(target)?;
        let client = build_client(target)?;
        let zones = zones_of(&client).await?;
        gather(&client, &settings, &zones, now_ms()).await
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let client = build_client(target)?;
        zones_of(&client).await?;
        Ok(Some("cloudflare".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::extract::Path;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::{get, post};

    use super::*;
    use crate::rest::test_support::{find, serve, value};
    use crate::uptime::tags::test_support::cible;

    const ZONES: &str = include_str!("testdata/zones.json");
    const ACCOUNTS: &str = include_str!("testdata/accounts.json");
    const TUNNELS: &str = include_str!("testdata/tunnels.json");
    const CERTIFICATES: &str = include_str!("testdata/certificates.json");
    const POOLS: &str = include_str!("testdata/pools.json");
    const BALANCERS: &str = include_str!("testdata/load_balancers.json");
    const GRAPHQL: &str = include_str!("testdata/graphql.json");

    const ACCOUNT: &str = "01a7362d577a6c3019a474fd6f485823";
    const ZONE: &str = "023e105f4ecef8ad9ca31a8372d0c353";
    const REFUSED: &str = r#"{"success":false,"errors":[{"code":10000,"message":"Authentication error"}],"result":null}"#;
    const EMPTY: &str = r#"{"success":true,"errors":[],"result":[]}"#;

    fn now() -> i64 {
        chrono::DateTime::parse_from_rfc3339("2026-09-30T09:00:00Z").unwrap().timestamp_millis()
    }

    fn array(raw: &str) -> Vec<Value> {
        let reply: Value = serde_json::from_str(raw).unwrap();
        results(&reply).to_vec()
    }

    #[test]
    fn zones_actives_en_attente_et_suspendues() {
        let s = zone_samples(&array(ZONES), now());
        assert_eq!(value(&s, "cloudflare_zones", &[]), 4.0);
        assert_eq!(value(&s, "cloudflare_zones_active", &[]), 3.0);
        assert_eq!(value(&s, "cloudflare_zones_paused", &[]), 1.0);
        assert_eq!(value(&s, "cloudflare_zone_active", &[("zone", "example.com")]), 1.0);
        assert_eq!(value(&s, "cloudflare_zone_active", &[("zone", "staging.example.net")]), 0.0);
        assert_eq!(value(&s, "cloudflare_zone_paused", &[("zone", "old.example.io")]), 1.0);
        let info = find(&s, "cloudflare_zone_info", &[("zone", "example.com")]).unwrap();
        assert_eq!(info.labels["plan"], "Pro Website");
        let pending = find(&s, "cloudflare_item_ok", &[("item", "staging.example.net")]).unwrap();
        assert_eq!(pending.labels["detail"], "Pending");
        assert_eq!(pending.value, 0.0);
        assert_eq!(value(&s, "cloudflare_figure", &[("name", "Zones")]), 4.0);
    }

    #[test]
    fn tunnels_sains_degrades_en_panne_et_inactifs() {
        let s = tunnel_samples(&array(TUNNELS), now());
        assert_eq!(value(&s, "cloudflare_tunnels", &[]), 4.0);
        assert_eq!(value(&s, "cloudflare_tunnels_healthy", &[]), 1.0);
        assert_eq!(value(&s, "cloudflare_tunnels_degraded", &[]), 1.0);
        assert_eq!(value(&s, "cloudflare_tunnels_down", &[]), 1.0);
        assert_eq!(value(&s, "cloudflare_tunnel_healthy", &[("tunnel", "home-lab")]), 1.0);
        assert_eq!(value(&s, "cloudflare_tunnel_connections", &[("tunnel", "home-lab")]), 2.0);
        assert_eq!(value(&s, "cloudflare_tunnel_degraded", &[("tunnel", "office")]), 1.0);
        assert_eq!(value(&s, "cloudflare_tunnel_down", &[("tunnel", "nas")]), 1.0);
        assert_eq!(value(&s, "cloudflare_tunnel_down", &[("tunnel", "spare")]), 0.0);
        let spare = find(&s, "cloudflare_item_ok", &[("item", "spare")]).unwrap();
        assert_eq!(spare.value, 1.0, "a tunnel that was never run is not an outage");
        assert_eq!(spare.labels["detail"], "Inactive");
    }

    #[test]
    fn certificats_actifs_le_plus_proche_par_hote() {
        let s = certificate_samples("example.com", &array(CERTIFICATES), now());
        assert_eq!(
            value(&s, "cloudflare_certificate_expiry_seconds", &[("host", "example.com")]),
            7.0 * 86_400.0
        );
        // Deux certificats dans le lot : le plus proche (20 décembre) l'emporte.
        let api =
            value(&s, "cloudflare_certificate_expiry_seconds", &[("host", "api.example.com")]);
        assert_eq!(api, 81.0 * 86_400.0);
        assert!(
            find(&s, "cloudflare_certificate_expiry_seconds", &[("host", "new.example.com")])
                .is_none(),
            "a pack still being validated has no expiry to watch"
        );
        let soon =
            find(&s, "cloudflare_item_ok", &[("item", "example.com (example.com)")]).unwrap();
        assert_eq!(soon.labels["detail"], "Expires within 14 days");
    }

    #[test]
    fn trafic_et_taux_de_5xx() {
        let reply: Value = serde_json::from_str(GRAPHQL).unwrap();
        let zones = vec![
            (ZONE.to_string(), "example.com".to_string()),
            ("2b6f1f0a1d3c4e5f60718293a4b5c6d7".to_string(), "shop.example.com".to_string()),
        ];
        let s = traffic_samples(&reply, &zones, now()).unwrap();
        assert_eq!(value(&s, "cloudflare_zone_http_requests", &[("zone", "example.com")]), 1500.0);
        assert_eq!(value(&s, "cloudflare_zone_http_5xx", &[("zone", "example.com")]), 80.0);
        let percent = value(&s, "cloudflare_zone_http_5xx_percent", &[("zone", "example.com")]);
        assert!((percent - 80.0 / 1500.0 * 100.0).abs() < 1e-9, "{percent}");
        assert!(
            find(&s, "cloudflare_zone_http_5xx_percent", &[("zone", "shop.example.com")]).is_none(),
            "forty requests are too few for a percentage"
        );
        assert!(find(&s, "cloudflare_country_threats", &[]).is_none(), "read apart");
        // Menaces par pays : additionnées sur les zones, une série par pays.
        let threats = threat_samples(&reply, &zones, now()).unwrap();
        let merged = merge_country_threats(threats);
        assert_eq!(value(&merged, "cloudflare_country_threats", &[("country", "CN")]), 35.0);
        assert_eq!(value(&merged, "cloudflare_country_threats", &[("country", "US")]), 12.0);
        let countries: Vec<&str> = merged
            .iter()
            .filter(|s| s.metric == "cloudflare_country_threats")
            .map(|s| s.labels["country"].as_str())
            .collect();
        assert_eq!(countries, ["CN", "US"], "no unknown, Tor or zero-threat country");
        for sample in merged.iter().filter(|s| s.metric == "cloudflare_country_threats") {
            assert!(!sample.labels.contains_key("zone"), "{:?}", sample.labels);
        }
        let failed: Value =
            serde_json::from_str(r#"{"data":null,"errors":[{"message":"denied"}]}"#).unwrap();
        assert!(traffic_samples(&failed, &zones, now()).is_none());
        assert!(threat_samples(&failed, &zones, now()).is_none());
    }

    #[test]
    fn pools_sains_en_panne_et_desactives() {
        let s = pool_samples(&array(POOLS), now());
        assert_eq!(value(&s, "cloudflare_lb_pools", &[]), 3.0);
        assert_eq!(value(&s, "cloudflare_lb_pool_healthy", &[("pool", "web-eu")]), 1.0);
        assert_eq!(value(&s, "cloudflare_lb_pool_origins", &[("pool", "web-eu")]), 2.0);
        assert_eq!(value(&s, "cloudflare_lb_pool_origins_healthy", &[("pool", "web-eu")]), 1.0);
        assert_eq!(value(&s, "cloudflare_lb_pool_healthy", &[("pool", "web-us")]), 0.0);
        assert!(
            find(&s, "cloudflare_lb_pool_healthy", &[("pool", "standby")]).is_none(),
            "a disabled pool is not an outage"
        );
        let standby = find(&s, "cloudflare_item_ok", &[("item", "standby")]).unwrap();
        assert_eq!(standby.value, 1.0);
        let lb = load_balancer_samples("example.com", &array(BALANCERS), now());
        assert_eq!(
            value(&lb, "cloudflare_load_balancer_enabled", &[("load_balancer", "www.example.com")]),
            1.0
        );
    }

    #[test]
    fn la_fenetre_de_trafic_commence_a_l_heure_precedente() {
        let (since, until) = traffic_window(now());
        assert_eq!(since, "2026-09-30T08:00:00Z");
        assert_eq!(until, "2026-09-30T09:00:00Z");
    }

    fn authorized(headers: &HeaderMap) -> bool {
        headers.get("authorization").and_then(|v| v.to_str().ok()) == Some("Bearer good")
    }

    /// Faux API : `refuse` rend 403 sur tout sauf la liste des zones.
    fn app(refuse: bool) -> Router {
        let answer = move |headers: &HeaderMap, body: &str| {
            if !authorized(headers) {
                (StatusCode::UNAUTHORIZED, REFUSED.to_string())
            } else if refuse {
                (StatusCode::FORBIDDEN, REFUSED.to_string())
            } else {
                (StatusCode::OK, body.to_string())
            }
        };
        Router::new()
            .route(
                "/client/v4/zones",
                get(move |headers: HeaderMap| async move {
                    if authorized(&headers) {
                        (StatusCode::OK, ZONES.to_string())
                    } else {
                        (StatusCode::UNAUTHORIZED, REFUSED.to_string())
                    }
                }),
            )
            .route(
                "/client/v4/accounts",
                get(move |headers: HeaderMap| async move { answer(&headers, ACCOUNTS) }),
            )
            .route(
                "/client/v4/accounts/{id}/cfd_tunnel",
                get(move |Path(id): Path<String>, headers: HeaderMap| async move {
                    if id == ACCOUNT {
                        answer(&headers, TUNNELS)
                    } else {
                        (StatusCode::NOT_FOUND, REFUSED.to_string())
                    }
                }),
            )
            .route(
                "/client/v4/accounts/{id}/load_balancers/pools",
                get(move |headers: HeaderMap| async move { answer(&headers, POOLS) }),
            )
            .route(
                "/client/v4/zones/{id}/ssl/certificate_packs",
                get(move |Path(id): Path<String>, headers: HeaderMap| async move {
                    answer(&headers, if id == ZONE { CERTIFICATES } else { EMPTY })
                }),
            )
            .route(
                "/client/v4/zones/{id}/load_balancers",
                get(move |headers: HeaderMap| async move { answer(&headers, BALANCERS) }),
            )
            .route(
                "/client/v4/graphql",
                post(move |headers: HeaderMap| async move { answer(&headers, GRAPHQL) }),
            )
    }

    fn target(address: &str, token: &str, tags: &[(&str, &str)]) -> Target {
        let mut target = cible("cloudflare", &format!("http://{address}"), tags);
        target.credential = Credential::ApiToken { token: token.into() };
        target
    }

    #[tokio::test]
    async fn sonde_complete() {
        let address = serve(app(false)).await;
        let s = CloudflareCollector::new().probe(&target(&address, "good", &[])).await.unwrap();
        assert_eq!(value(&s, "cloudflare_zones", &[]), 4.0);
        assert_eq!(value(&s, "cloudflare_tunnels", &[]), 4.0);
        assert_eq!(value(&s, "cloudflare_tunnels_readable", &[]), 1.0);
        assert!(
            find(&s, "cloudflare_certificate_expiry_seconds", &[("zone", "example.com")]).is_some()
        );
        assert_eq!(value(&s, "cloudflare_certificates_readable", &[]), 1.0);
        assert!(find(&s, "cloudflare_zone_http_5xx_percent", &[("zone", "example.com")]).is_some());
        assert_eq!(value(&s, "cloudflare_analytics_readable", &[]), 1.0);
        assert_eq!(value(&s, "cloudflare_lb_pool_healthy", &[("pool", "web-us")]), 0.0);
        assert_eq!(value(&s, "cloudflare_load_balancers", &[]), 2.0);
        assert_eq!(value(&s, "cloudflare_lb_readable", &[]), 1.0);
        assert!(find(&s, "cloudflare_figure", &[("name", "Requests")]).is_some());
        assert!(
            s.iter().all(|sample| !sample.metric.starts_with("dumbmonit_")),
            "the server adds the dumbmonit_ prefix"
        );
    }

    #[tokio::test]
    async fn analytics_desactivee_ne_demande_pas_le_trafic() {
        let address = serve(app(false)).await;
        let tags = [("analytics", "false")];
        let s = CloudflareCollector::new().probe(&target(&address, "good", &tags)).await.unwrap();
        assert!(find(&s, "cloudflare_analytics_readable", &[]).is_none());
        assert!(find(&s, "cloudflare_zone_http_requests", &[]).is_none());
    }

    #[tokio::test]
    async fn sous_appels_refuses_sans_echec_de_la_sonde() {
        let address = serve(app(true)).await;
        let s = CloudflareCollector::new().probe(&target(&address, "good", &[])).await.unwrap();
        assert_eq!(value(&s, "cloudflare_zones", &[]), 4.0);
        for part in ["tunnels", "certificates", "analytics", "lb"] {
            assert_eq!(value(&s, &format!("cloudflare_{part}_readable"), &[]), 0.0, "{part}");
        }
        assert!(find(&s, "cloudflare_tunnel_down", &[]).is_none());
        assert!(find(&s, "cloudflare_certificate_expiry_seconds", &[]).is_none());
    }

    #[tokio::test]
    async fn jeton_refuse_donne_une_erreur_d_authentification() {
        let address = serve(app(false)).await;
        let error =
            CloudflareCollector::new().probe(&target(&address, "wrong", &[])).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
        assert!(!error.means_down());
        let forbidden = Router::new().route(
            "/client/v4/zones",
            get(|| async { (StatusCode::FORBIDDEN, REFUSED.to_string()) }),
        );
        let address = serve(forbidden).await;
        let error =
            CloudflareCollector::new().probe(&target(&address, "good", &[])).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
    }

    #[tokio::test]
    async fn panne_du_serveur_veut_dire_hors_service() {
        let broken = Router::new().route(
            "/client/v4/zones",
            get(|| async { (StatusCode::BAD_GATEWAY, "bad gateway".to_string()) }),
        );
        let address = serve(broken).await;
        let error =
            CloudflareCollector::new().probe(&target(&address, "good", &[])).await.unwrap_err();
        assert!(error.means_down(), "{error}");
    }

    #[tokio::test]
    async fn compte_saisi_introuvable_est_une_erreur_de_reglage() {
        let address = serve(app(false)).await;
        let tags = [("account_id", "deadbeef")];
        let error =
            CloudflareCollector::new().probe(&target(&address, "good", &tags)).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
        let tags = [("account_id", "a/b")];
        let error =
            CloudflareCollector::new().probe(&target(&address, "good", &tags)).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
    }

    #[tokio::test]
    async fn identifiants_d_un_autre_type_refuses() {
        let mut other = cible("cloudflare", "unreachable.invalid:1", &[]);
        other.credential =
            Credential::UsernamePassword { username: "u".into(), password: "p".into() };
        let error = CloudflareCollector::new().probe(&other).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
    }
}
