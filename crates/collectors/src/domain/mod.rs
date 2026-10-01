//! Expiration d'un nom de domaine, lue par RDAP (`kind = "domain"`).
//!
//! RDAP est le successeur normalisé de WHOIS (RFC 9082, 9083) : du JSON, servi
//! par le registre de chaque extension. Le registre compétent se trouve dans le
//! fichier d'amorçage de l'IANA (`https://data.iana.org/rdap/dns.json`,
//! RFC 9224), relu au plus une fois par jour. Aucun identifiant n'est requis.
//!
//! Ce que la sonde retient : la date d'expiration (et donc les jours restants),
//! les statuts EPP (`clientHold`, `redemptionPeriod`…), le bureau
//! d'enregistrement et la signature DNSSEC de la délégation. Un domaine en
//! `clientHold` ou `serverHold` n'est plus publié dans le DNS : tout ce qui en
//! dépend tombe, souvent pour une facture impayée ou une adresse de contact non
//! vérifiée. Un domaine en `redemptionPeriod` a déjà expiré.
//!
//! Les registres limitent le débit RDAP : la réponse est gardée en mémoire et
//! n'est redemandée que toutes les `refresh_hours`. Entre deux, les jours
//! restants sont recalculés à chaque passage. Un registre momentanément muet ne
//! coupe pas la série : la dernière réponse sert encore sept jours, avec
//! `domain_rdap_ok 0`.
//!
//! # Adresse et étiquettes
//!
//! Adresse : le domaine enregistré, `example.com` (pas `www.example.com`).
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `rdap_server` | amorçage IANA | Racine RDAP du registre, pour une extension que l'IANA ne liste pas (`https://rdap.nic.ch/`). |
//! | `refresh_hours` | `12` | Intervalle entre deux questions au registre (1 à 168). |
//! | `request_timeout_seconds` | `10` | Délai par requête HTTP. |

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, MetricKind, ProbeError, Sample, Target};
use serde::Deserialize;

use crate::uptime::tags;

pub const IANA_BOOTSTRAP: &str = "https://data.iana.org/rdap/dns.json";
pub const DEFAULT_REFRESH_HOURS: u32 = 12;
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// Le fichier d'amorçage change quelques fois par an : un jour suffit.
const BOOTSTRAP_TTL: Duration = Duration::from_secs(24 * 3600);
/// Au-delà, une réponse ancienne n'est plus servie à la place d'une neuve.
const STALE_LIMIT: Duration = Duration::from_secs(7 * 24 * 3600);
const RDAP_JSON: &str = "application/rdap+json, application/json";
const DAY_SECONDS: f64 = 86_400.0;

/// Les statuts qui retirent le domaine du DNS.
const HOLD: [&str; 2] = ["clientHold", "serverHold"];
/// Les statuts d'un domaine expiré, en passe d'être supprimé.
const REDEMPTION: [&str; 3] = ["redemptionPeriod", "pendingDelete", "pendingRestore"];

pub struct DomainCollector {
    bootstrap_url: String,
    /// Le fichier d'amorçage, avec l'heure de sa lecture.
    bootstrap: tokio::sync::Mutex<Option<(Instant, Bootstrap)>>,
    /// Dernière réponse par (domaine, registre), avec l'heure Unix de sa lecture.
    cache: Mutex<HashMap<(String, String), (i64, Registration)>>,
}

impl Default for DomainCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl DomainCollector {
    pub fn new() -> Self {
        Self::with_bootstrap(IANA_BOOTSTRAP)
    }

    /// Un fichier d'amorçage ailleurs que chez l'IANA (tests, miroir interne).
    pub fn with_bootstrap(url: &str) -> Self {
        Self {
            bootstrap_url: url.to_string(),
            bootstrap: tokio::sync::Mutex::new(None),
            cache: Mutex::new(HashMap::new()),
        }
    }
}

struct Settings {
    name: String,
    server: Option<String>,
    refresh: Duration,
    timeout: Duration,
}

/// Le nom tel que RDAP l'attend : minuscules, sans point final, en ASCII
/// (`xn--…` pour un nom internationalisé). Une URL collée est ramenée à son hôte.
pub fn normalize(address: &str) -> Result<String, ProbeError> {
    let raw = address.trim();
    let raw = raw.split_once("://").map_or(raw, |(_, rest)| rest);
    let raw = raw.split(['/', '?', '#']).next().unwrap_or_default().trim_end_matches('.');
    let invalid = || {
        ProbeError::Config(format!(
            "\"{address}\" is not a domain name: enter the registered domain, for example \
             \"example.com\""
        ))
    };
    if raw.is_empty() || !raw.contains('.') || raw.contains(':') {
        return Err(invalid());
    }
    // L'analyseur d'URL applique IDNA : le nom en sort en minuscules et en ASCII.
    let url = reqwest::Url::parse(&format!("http://{raw}/")).map_err(|_| invalid())?;
    // `domain()` ne rend rien pour une adresse IP.
    url.domain().map(str::to_string).ok_or_else(invalid)
}

fn settings(target: &Target) -> Result<Settings, ProbeError> {
    if !matches!(target.credential, Credential::None) {
        return Err(ProbeError::Config(
            "RDAP is public: a domain check takes no credential".to_string(),
        ));
    }
    let server = match tags::tag(target, "rdap_server") {
        None => None,
        Some(raw) => {
            let url = reqwest::Url::parse(raw).ok().filter(|u| u.scheme().starts_with("http"));
            let url = url.ok_or_else(|| {
                ProbeError::Config(format!(
                    "\"rdap_server\" expects the RDAP base URL of the registry, for example \
                     \"https://rdap.nic.ch/\", got \"{raw}\""
                ))
            })?;
            Some(url.to_string())
        }
    };
    let refresh = tags::parse_u32(target, "refresh_hours", DEFAULT_REFRESH_HOURS, 1..=168)?;
    let timeout = tags::parse_u32(
        target,
        "request_timeout_seconds",
        DEFAULT_REQUEST_TIMEOUT.as_secs() as u32,
        1..=120,
    )?;
    Ok(Settings {
        name: normalize(&target.address)?,
        server,
        refresh: Duration::from_secs(u64::from(refresh) * 3600),
        timeout: Duration::from_secs(u64::from(timeout)),
    })
}

#[async_trait]
impl Collector for DomainCollector {
    fn kind(&self) -> &'static str {
        "domain"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let settings = settings(target)?;
        let (registration, fresh, age) = self.registration(&settings).await?;
        let now = chrono::Utc::now();
        let mut out = samples(&registration, now.timestamp(), now.timestamp_millis());
        let gauge = |name: &str, value: f64| {
            Sample::new(name, value, MetricKind::Gauge, now.timestamp_millis())
        };
        out.push(gauge("domain_rdap_ok", if fresh { 1.0 } else { 0.0 }));
        out.push(gauge("domain_rdap_age_seconds", age.as_secs_f64()));
        Ok(out)
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let settings = settings(target)?;
        self.registration(&settings).await?;
        Ok(Some("domain".to_string()))
    }
}

impl DomainCollector {
    /// La réponse du registre : celle du cache si elle a moins de
    /// `refresh_hours`, sinon une neuve ; à défaut la dernière connue, tant
    /// qu'elle a moins de sept jours. Rend aussi « neuve ou non » et son âge.
    async fn registration(
        &self,
        settings: &Settings,
    ) -> Result<(Registration, bool, Duration), ProbeError> {
        let http = crate::http::client(false)?;
        let server = match &settings.server {
            Some(server) => server.clone(),
            None => self.server_for(&http, &settings.name, settings.timeout).await?,
        };
        let key = (settings.name.clone(), server.clone());
        let now = chrono::Utc::now().timestamp();
        let age = |at: i64| Duration::from_secs(now.saturating_sub(at).max(0) as u64);
        let cached = self.cache.lock().ok().and_then(|cache| cache.get(&key).cloned());
        if let Some((at, registration)) = &cached
            && age(*at) < settings.refresh
        {
            return Ok((registration.clone(), true, age(*at)));
        }
        match lookup(&http, &server, &settings.name, settings.timeout).await {
            Ok(registration) => {
                if let Ok(mut cache) = self.cache.lock() {
                    cache.insert(key, (now, registration.clone()));
                }
                Ok((registration, true, Duration::ZERO))
            }
            // Un domaine inconnu du registre n'est pas un incident passager.
            Err(error @ ProbeError::Config(_)) => Err(error),
            Err(error) => match cached {
                Some((at, registration)) if age(at) < STALE_LIMIT => {
                    tracing::debug!(domain = %settings.name, %error, "RDAP muet, réponse en cache");
                    Ok((registration, false, age(at)))
                }
                _ => Err(error),
            },
        }
    }

    /// Le registre RDAP d'un nom, d'après l'amorçage de l'IANA.
    async fn server_for(
        &self,
        http: &reqwest::Client,
        name: &str,
        timeout: Duration,
    ) -> Result<String, ProbeError> {
        let mut guard = self.bootstrap.lock().await;
        let stale = guard.as_ref().is_none_or(|(at, _)| at.elapsed() > BOOTSTRAP_TTL);
        if stale {
            match fetch_bootstrap(http, &self.bootstrap_url, timeout).await {
                Ok(bootstrap) => *guard = Some((Instant::now(), bootstrap)),
                // Un amorçage périmé vaut mieux que rien : il change rarement.
                Err(error) if guard.is_none() => return Err(error),
                Err(_) => {}
            }
        }
        let (_, bootstrap) = guard.as_ref().expect("amorçage chargé ci-dessus");
        bootstrap.server_for(name).ok_or_else(|| {
            let tld = name.rsplit('.').next().unwrap_or(name);
            ProbeError::Config(format!(
                "IANA lists no RDAP server for .{tld}. If the registry of .{tld} runs one, \
                 set its address in RDAP server (for .ch and .li: https://rdap.nic.ch/)."
            ))
        })
    }
}

// ------------------------------------------------------------- amorçage

#[derive(Debug, Deserialize)]
pub struct Bootstrap {
    services: Vec<(Vec<String>, Vec<String>)>,
}

impl Bootstrap {
    /// La racine RDAP du suffixe le plus long qui couvre `name`, en HTTPS de
    /// préférence (RFC 9224, section 4).
    pub fn server_for(&self, name: &str) -> Option<String> {
        let labels: Vec<&str> = name.split('.').collect();
        (1..labels.len()).find_map(|start| {
            let suffix = labels[start..].join(".");
            self.services.iter().find_map(|(suffixes, urls)| {
                if !suffixes.iter().any(|s| s.eq_ignore_ascii_case(&suffix)) {
                    return None;
                }
                urls.iter().find(|u| u.starts_with("https://")).or_else(|| urls.first()).cloned()
            })
        })
    }
}

async fn fetch_bootstrap(
    http: &reqwest::Client,
    url: &str,
    timeout: Duration,
) -> Result<Bootstrap, ProbeError> {
    let response = http
        .get(url)
        .timeout(timeout)
        .send()
        .await
        .map_err(|error| transport(&error, timeout, "the IANA RDAP bootstrap file"))?;
    if !response.status().is_success() {
        return Err(ProbeError::Unreachable(format!(
            "{url} answered {}: the RDAP servers cannot be looked up",
            response.status()
        )));
    }
    let body = response
        .text()
        .await
        .map_err(|error| transport(&error, timeout, "the IANA RDAP bootstrap file"))?;
    serde_json::from_str(&body).map_err(|error| {
        ProbeError::Protocol(format!("unreadable RDAP bootstrap file at {url}: {error}"))
    })
}

fn transport(error: &reqwest::Error, timeout: Duration, what: &str) -> ProbeError {
    if error.is_timeout() {
        ProbeError::Timeout(timeout)
    } else {
        ProbeError::Unreachable(format!("{what}: {error}"))
    }
}

// ------------------------------------------------------------ interrogation

async fn lookup(
    http: &reqwest::Client,
    server: &str,
    name: &str,
    timeout: Duration,
) -> Result<Registration, ProbeError> {
    let url = format!("{}/domain/{name}", server.trim_end_matches('/'));
    let response = http
        .get(&url)
        .header(reqwest::header::ACCEPT, RDAP_JSON)
        .timeout(timeout)
        .send()
        .await
        .map_err(|error| transport(&error, timeout, &url))?;
    let status = response.status();
    let body = response.text().await.map_err(|error| transport(&error, timeout, &url))?;
    match status.as_u16() {
        200..=299 => {}
        404 => {
            return Err(ProbeError::Config(format!(
                "the registry does not know \"{name}\" ({url} answered 404). Enter the \
                 registered domain itself, for example \"example.com\" rather than \
                 \"www.example.com\"; a domain that was deleted is no longer registered."
            )));
        }
        429 => {
            return Err(ProbeError::Protocol(format!(
                "the registry is limiting queries ({url} answered 429). Raise Refresh interval."
            )));
        }
        code => {
            let excerpt: String = body.chars().take(200).collect();
            return Err(ProbeError::Unreachable(format!(
                "{url} answered {code}: {}",
                excerpt.trim()
            )));
        }
    }
    Registration::parse(&body).map_err(|error| {
        ProbeError::Protocol(format!("unreadable RDAP answer from {url}: {error}"))
    })
}

// ------------------------------------------------------------- réponse

/// Ce que l'on retient d'une réponse RDAP.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Registration {
    /// Statuts EPP, en `camelCase` (`clientTransferProhibited`).
    pub statuses: Vec<String>,
    /// Expiration, en secondes Unix ; `None` si le registre ne la publie pas.
    pub expires_s: Option<i64>,
    pub registrar: Option<String>,
    /// Délégation signée (DNSSEC) ; `None` si le registre ne le dit pas.
    pub dnssec: Option<bool>,
}

#[derive(Deserialize)]
struct RawDomain {
    #[serde(rename = "objectClassName", default)]
    object_class: String,
    #[serde(default)]
    status: Vec<String>,
    #[serde(default)]
    events: Vec<RawEvent>,
    #[serde(default)]
    entities: Vec<RawEntity>,
    #[serde(rename = "secureDNS", default)]
    secure_dns: Option<RawSecureDns>,
}

#[derive(Deserialize)]
struct RawEvent {
    #[serde(rename = "eventAction", default)]
    action: String,
    #[serde(rename = "eventDate", default)]
    date: String,
}

#[derive(Deserialize)]
struct RawEntity {
    #[serde(default)]
    roles: Vec<String>,
    #[serde(rename = "vcardArray", default)]
    vcard: Option<serde_json::Value>,
    #[serde(default)]
    handle: Option<String>,
    #[serde(default)]
    events: Vec<RawEvent>,
}

#[derive(Deserialize)]
struct RawSecureDns {
    #[serde(rename = "delegationSigned", default)]
    delegation_signed: Option<bool>,
}

/// `client transfer prohibited` → `clientTransferProhibited` (RFC 8056).
pub fn epp_status(rdap: &str) -> String {
    let mut words = rdap.split_whitespace();
    let Some(first) = words.next() else { return String::new() };
    let mut out = first.to_string();
    if words.clone().next().is_none() {
        return out;
    }
    out = out.to_ascii_lowercase();
    for word in words {
        let mut chars = word.chars();
        if let Some(c) = chars.next() {
            out.extend(c.to_uppercase());
            out.push_str(&chars.as_str().to_ascii_lowercase());
        }
    }
    out
}

/// `fn`, à défaut `org`, d'une carte jCard (RFC 7095).
fn vcard_name(vcard: &serde_json::Value) -> Option<String> {
    let properties = vcard.get(1)?.as_array()?;
    let text = |key: &str| {
        properties.iter().find_map(|p| {
            (p.get(0)?.as_str()? == key)
                .then(|| p.get(3)?.as_str().map(str::trim).map(str::to_string))
                .flatten()
                .filter(|v| !v.is_empty())
        })
    };
    text("fn").or_else(|| text("org"))
}

impl Registration {
    pub fn parse(body: &str) -> Result<Self, String> {
        let raw: RawDomain = serde_json::from_str(body).map_err(|e| e.to_string())?;
        if !raw.object_class.is_empty() && raw.object_class != "domain" {
            return Err(format!("expected a domain object, got \"{}\"", raw.object_class));
        }
        let registrar = raw.entities.iter().find(|e| e.roles.iter().any(|r| r == "registrar"));
        let expiration = |events: &[RawEvent]| {
            events.iter().find(|e| e.action == "expiration").and_then(|e| {
                chrono::DateTime::parse_from_rfc3339(e.date.trim()).ok().map(|t| t.timestamp())
            })
        };
        Ok(Self {
            statuses: raw.status.iter().map(|s| epp_status(s)).filter(|s| !s.is_empty()).collect(),
            // Quelques registres rangent l'expiration chez le bureau
            // d'enregistrement plutôt que sur le domaine.
            expires_s: expiration(&raw.events)
                .or_else(|| registrar.and_then(|r| expiration(&r.events))),
            registrar: registrar
                .and_then(|r| r.vcard.as_ref().and_then(vcard_name).or_else(|| r.handle.clone())),
            dnssec: raw.secure_dns.and_then(|s| s.delegation_signed),
        })
    }
}

pub fn samples(registration: &Registration, now_s: i64, ts_ms: i64) -> Vec<Sample> {
    let gauge = |name: &str, value: f64| Sample::new(name, value, MetricKind::Gauge, ts_ms);
    let flag = |on: bool| if on { 1.0 } else { 0.0 };
    let has = |set: &[&str]| registration.statuses.iter().any(|s| set.contains(&s.as_str()));
    let mut out = Vec::new();
    out.push(gauge("domain_expiry_published", flag(registration.expires_s.is_some())));
    if let Some(expires) = registration.expires_s {
        out.push(gauge("domain_expiry_days", (expires - now_s) as f64 / DAY_SECONDS));
        out.push(gauge("domain_expiry_timestamp_seconds", expires as f64));
    }
    out.push(gauge("domain_on_hold", flag(has(&HOLD))));
    out.push(gauge("domain_redemption", flag(has(&REDEMPTION))));
    for status in &registration.statuses {
        out.push(gauge("domain_status", 1.0).with_label("status", status.clone()));
    }
    if let Some(registrar) = &registration.registrar {
        out.push(gauge("domain_registrar_info", 1.0).with_label("registrar", registrar.clone()));
    }
    if let Some(signed) = registration.dnssec {
        out.push(gauge("domain_dnssec", flag(signed)));
    }
    out
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use axum::Router;
    use axum::http::StatusCode;
    use axum::routing::get;

    use super::*;
    use crate::uptime::tags::test_support::cible;

    // Réponses RDAP réelles du 1er octobre 2026 (contacts retirés), et un
    // extrait du fichier d'amorçage de l'IANA publié le 28 septembre 2026.
    const COM: &str = include_str!("testdata/com_example.json");
    const ORG: &str = include_str!("testdata/org_wikipedia.json");
    const DEV: &str = include_str!("testdata/dev_web.json");
    const FR: &str = include_str!("testdata/fr_afnic.json");
    const CH: &str = include_str!("testdata/ch_switch.json");
    const BOOTSTRAP: &str = include_str!("testdata/iana_dns.json");
    /// 2026-10-01T00:00:00Z.
    const NOW: i64 = 1_790_812_800;

    fn find<'a>(samples: &'a [Sample], name: &str, labels: &[(&str, &str)]) -> Option<&'a Sample> {
        samples.iter().find(|s| {
            s.metric == name
                && labels.iter().all(|(k, v)| s.labels.get(*k).map(String::as_str) == Some(v))
        })
    }

    #[test]
    fn cinq_registres_reels() {
        let com = Registration::parse(COM).unwrap();
        assert_eq!(com.registrar.as_deref(), Some("RESERVED-Internet Assigned Numbers Authority"));
        assert_eq!(
            com.statuses,
            ["clientDeleteProhibited", "clientTransferProhibited", "clientUpdateProhibited"]
        );
        assert_eq!(com.dnssec, Some(true));
        let out = samples(&com, NOW, 0);
        // 2027-08-13T04:00:00Z.
        let days = find(&out, "domain_expiry_days", &[]).unwrap().value;
        assert!((days - 316.17).abs() < 0.01, "{days}");
        assert_eq!(find(&out, "domain_on_hold", &[]).unwrap().value, 0.0);

        let org = Registration::parse(ORG).unwrap();
        assert_eq!(org.registrar.as_deref(), Some("MarkMonitor Inc."));
        assert!(org.expires_s.is_some());
        let dev = Registration::parse(DEV).unwrap();
        assert!(dev.statuses.contains(&"renewPeriod".to_string()));
        let fr = Registration::parse(FR).unwrap();
        assert_eq!(fr.statuses, ["active"]);
        assert!(fr.expires_s.is_some());

        // SWITCH ne publie pas d'expiration, et nomme le bureau par `org`.
        let ch = Registration::parse(CH).unwrap();
        assert_eq!(ch.expires_s, None);
        assert_eq!(ch.registrar.as_deref(), Some("Gandi SAS"));
        let out = samples(&ch, NOW, 0);
        assert_eq!(find(&out, "domain_expiry_published", &[]).unwrap().value, 0.0);
        assert!(find(&out, "domain_expiry_days", &[]).is_none());
    }

    #[test]
    fn un_domaine_suspendu_ou_expire() {
        let mut raw: serde_json::Value = serde_json::from_str(COM).unwrap();
        raw["status"] = serde_json::json!(["client hold", "redemption period", "inactive"]);
        let registration = Registration::parse(&raw.to_string()).unwrap();
        let samples = samples(&registration, NOW, 0);
        assert_eq!(find(&samples, "domain_on_hold", &[]).unwrap().value, 1.0);
        assert_eq!(find(&samples, "domain_redemption", &[]).unwrap().value, 1.0);
        assert!(find(&samples, "domain_status", &[("status", "clientHold")]).is_some());
        assert!(find(&samples, "domain_status", &[("status", "inactive")]).is_some());
    }

    #[test]
    fn statuts_et_noms() {
        assert_eq!(epp_status("client transfer prohibited"), "clientTransferProhibited");
        assert_eq!(epp_status("serverHold"), "serverHold");
        assert_eq!(epp_status("active"), "active");
        assert_eq!(normalize("Example.COM.").unwrap(), "example.com");
        assert_eq!(normalize("https://example.com/path?q=1").unwrap(), "example.com");
        assert_eq!(normalize("bücher.example").unwrap(), "xn--bcher-kva.example");
        for bad in ["", "localhost", "10.0.0.1", "exa mple.com", "[::1]"] {
            assert!(matches!(normalize(bad), Err(ProbeError::Config(_))), "« {bad} »");
        }
    }

    #[test]
    fn l_amorcage_choisit_le_suffixe_le_plus_long_en_https() {
        let bootstrap: Bootstrap = serde_json::from_str(BOOTSTRAP).unwrap();
        assert_eq!(
            bootstrap.server_for("example.com").as_deref(),
            Some("https://rdap.verisign.com/com/v1/")
        );
        assert_eq!(
            bootstrap.server_for("web.dev").as_deref(),
            Some("https://pubapi.registry.google/rdap/")
        );
        assert_eq!(bootstrap.server_for("switch.ch"), None);
        let bootstrap: Bootstrap = serde_json::from_str(
            r#"{"services":[[["uk"],["http://a.test/"]],[["co.uk"],["http://b.test/","https://b.test/"]]]}"#,
        )
        .unwrap();
        assert_eq!(bootstrap.server_for("x.co.uk").as_deref(), Some("https://b.test/"));
        assert_eq!(bootstrap.server_for("x.uk").as_deref(), Some("http://a.test/"));
    }

    /// Un faux registre et un faux amorçage : la réponse est mise en cache,
    /// un nom inconnu est une erreur de configuration, un registre muet laisse
    /// servir la dernière réponse.
    #[tokio::test]
    async fn interrogation_complete_contre_un_faux_registre() {
        let queries = Arc::new(AtomicUsize::new(0));
        let failing = Arc::new(AtomicUsize::new(0));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/", listener.local_addr().unwrap());
        let bootstrap = format!(r#"{{"services":[[["com"],["{base}"]]]}}"#);
        let (q, f) = (queries.clone(), failing.clone());
        let app = Router::new().route("/dns.json", get(move || async move { bootstrap })).route(
            "/domain/{name}",
            get(move |axum::extract::Path(name): axum::extract::Path<String>| {
                let (q, f) = (q.clone(), f.clone());
                async move {
                    q.fetch_add(1, Ordering::SeqCst);
                    if f.load(Ordering::SeqCst) > 0 {
                        return (StatusCode::SERVICE_UNAVAILABLE, String::new());
                    }
                    match name.as_str() {
                        "example.com" => (StatusCode::OK, COM.to_string()),
                        _ => (StatusCode::NOT_FOUND, String::new()),
                    }
                }
            }),
        );
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let collector = DomainCollector::with_bootstrap(&format!("{base}dns.json"));
        let target = cible("domain", "example.com", &[]);
        let samples = collector.probe(&target).await.unwrap();
        assert!(find(&samples, "domain_expiry_days", &[]).is_some());
        assert_eq!(find(&samples, "domain_rdap_ok", &[]).unwrap().value, 1.0);
        collector.probe(&target).await.unwrap();
        assert_eq!(queries.load(Ordering::SeqCst), 1, "la réponse est gardée refresh_hours");

        let error = collector.probe(&cible("domain", "www.example.com", &[])).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(ref m) if m.contains("registered")), "{error}");
        assert!(!error.means_down());

        // Registre muet : sans cache, une panne ; avec, la dernière réponse.
        failing.store(1, Ordering::SeqCst);
        let fresh = DomainCollector::with_bootstrap(&format!("{base}dns.json"));
        let error = fresh.probe(&target).await.unwrap_err();
        assert!(error.means_down(), "{error}");
        if let Ok(mut cache) = collector.cache.lock() {
            for (at, _) in cache.values_mut() {
                *at -= 13 * 3600;
            }
        }
        let samples = collector.probe(&target).await.unwrap();
        assert_eq!(find(&samples, "domain_rdap_ok", &[]).unwrap().value, 0.0);
        assert!(find(&samples, "domain_expiry_days", &[]).is_some());

        // Une extension absente de l'amorçage, puis servie par `rdap_server`.
        failing.store(0, Ordering::SeqCst);
        let error = collector.probe(&cible("domain", "switch.ch", &[])).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(ref m) if m.contains(".ch")), "{error}");
        let target = cible("domain", "example.com", &[("rdap_server", &base)]);
        assert!(
            DomainCollector::with_bootstrap("http://127.0.0.1:9/").probe(&target).await.is_ok()
        );
    }

    /// Contre les vrais registres : `DUMBMONIT_TEST_DOMAIN=example.com`, et
    /// `DUMBMONIT_TEST_DOMAIN_RDAP` pour une extension hors amorçage.
    #[tokio::test]
    #[ignore = "demande un accès à Internet"]
    async fn domaine_reel() {
        let name = std::env::var("DUMBMONIT_TEST_DOMAIN").unwrap();
        let server = std::env::var("DUMBMONIT_TEST_DOMAIN_RDAP").unwrap_or_default();
        let tags: Vec<(&str, &str)> =
            if server.is_empty() { vec![] } else { vec![("rdap_server", server.as_str())] };
        for sample in DomainCollector::new().probe(&cible("domain", &name, &tags)).await.unwrap() {
            println!("{} {:?} {}", sample.metric, sample.labels, sample.value);
        }
    }
}
