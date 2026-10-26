//! Windows sans agent : WinRM (WS-Management) et requêtes WQL sur `root/cimv2`.
//!
//! La machine n'a rien à installer : le service WinRM de Windows répond à des
//! requêtes WMI (`Enumerate` puis `Pull`, voir [`wsman`]). Aucune commande ni
//! aucun script n'est exécuté sur la cible : seulement des `SELECT` fixes.
//!
//! # Transport et authentification
//!
//! * **HTTPS, port 5986** par défaut. Le trafic non chiffré (HTTP, port 5985)
//!   n'est utilisé que si l'étiquette `allow_unencrypted` est cochée : le serveur
//!   doit alors avoir `AllowUnencrypted = true`, ce qui est déconseillé.
//! * **NTLMv2** (en-tête `Negotiate` portant des jetons NTLM, voir [`ntlm`]) par
//!   défaut, ou **Basic** (HTTPS seulement, jamais en clair). Kerberos n'est pas
//!   pris en charge : il n'existe pas de client Kerberos en Rust pur, sous
//!   licence libre et sans bibliothèque C, qui tienne dans le binaire statique
//!   musl et l'image `scratch`.
//!
//! # Mêmes métriques que l'agent Windows
//!
//! `cpu_usage_percent`, `cpu_core_usage_percent`, `cpu_count`, `memory_*`,
//! `swap_*`, `filesystem_*`, `uptime_seconds`, `process_count` et `service_up`
//! portent les noms et étiquettes de l'agent : tableaux et règles marchent à
//! l'identique. S'y ajoutent `windows_version_info`,
//! `windows_services_auto_stopped`, `windows_eventlog_errors` et
//! `windows_eventlog_events`, ainsi que `winrm_section_readable{section}` quand
//! une lecture facultative est refusée.
//!
//! # Limites
//!
//! Les mises à jour en attente ne se lisent pas en WQL (l'agent de mise à jour
//! n'expose pas de classe WMI) : elles ne sont pas collectées. Le journal
//! d'événements est lu par `Win32_NTLogEvent` (journaux classiques) et réduit à
//! des comptes ; les règles d'Event ID détaillées restent le travail de l'agent.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `allow_unencrypted` | `false` | HTTP (5985) au lieu de HTTPS (5986). |
//! | `port` | 5986 / 5985 | Port WinRM. |
//! | `auth` | `ntlm` | `ntlm` ou `basic` (HTTPS seulement). |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable. |
//! | `request_timeout_seconds` | `15` | Délai par requête. |
//! | `services` | vide | Services suivis par nom (un `service_up` chacun). |
//! | `ignore_services` | vide | Services « Automatique » arrêtés à ne pas signaler. |
//! | `event_logs` | `System,Application` | Journaux comptés ; `none` = aucun. |
//! | `event_minutes` | `60` | Fenêtre de comptage des erreurs. |

mod ntlm;
mod wsman;

use std::collections::BTreeMap;
use std::time::Duration;

use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};
use reqwest::StatusCode;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, WWW_AUTHENTICATE};
use ring::rand::{SecureRandom, SystemRandom};

use crate::rest::{MAX_NAMED, flag, gauge, now_ms};
use crate::selfhosted::options::{base_url, parse_bool, parse_port, parse_timeout, tag};
use wsman::Row;

pub const DEFAULT_PORT: u16 = 5986;
pub const DEFAULT_HTTP_PORT: u16 = 5985;
pub const DEFAULT_EVENT_LOGS: &str = "System,Application";
pub const DEFAULT_EVENT_MINUTES: u64 = 60;

/// Taille des pages `Enumerate`/`Pull` et plafond de pages par requête.
const PAGE_SIZE: u32 = 256;
const MAX_PAGES: usize = 8;
/// Le journal d'événements est plafonné plus bas : un compte « au moins ».
const EVENT_PAGE_SIZE: u32 = 100;
const EVENT_MAX_PAGES: usize = 5;
const MAX_EVENT_LOGS: usize = 8;
const TOP_EVENTS: usize = 5;

/// Services qui démarrent « Automatique » mais s'arrêtent d'eux-mêmes (démarrage
/// différé, déclenché par un événement, mise à jour terminée) : ils ne sont pas
/// des pannes. `*` en tête ou en queue sert de joker.
const BUILTIN_IGNORED: &[&str] = &[
    "gupdate",
    "gupdatem",
    "edgeupdate",
    "edgeupdatem",
    "MapsBroker",
    "sppsvc",
    "TrustedInstaller",
    "clr_optimization_*",
    "GoogleChromeElevationService",
    "BrokerInfrastructure_*",
];

const OS_WQL: &str = "SELECT Caption, Version, BuildNumber, TotalVisibleMemorySize, \
    FreePhysicalMemory, LastBootUpTime, NumberOfProcesses FROM Win32_OperatingSystem";
const CPU_WQL: &str =
    "SELECT Name, PercentProcessorTime FROM Win32_PerfFormattedData_PerfOS_Processor";
const PAGEFILE_WQL: &str = "SELECT AllocatedBaseSize, CurrentUsage FROM Win32_PageFileUsage";
const DISKS_WQL: &str =
    "SELECT DeviceID, FileSystem, Size, FreeSpace FROM Win32_LogicalDisk WHERE DriveType = 3";
const SERVICES_WQL: &str = "SELECT Name, DisplayName, State, StartMode FROM Win32_Service";

#[derive(Default)]
pub struct WinrmCollector;

impl WinrmCollector {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthMode {
    Ntlm,
    Basic,
}

/// Réglages lus sur la cible.
#[derive(Debug, Clone)]
pub struct Options {
    endpoint: String,
    auth: AuthMode,
    insecure_tls: bool,
    timeout: Duration,
    pub services: Vec<String>,
    pub ignore_services: Vec<String>,
    pub event_logs: Vec<String>,
    pub event_minutes: u64,
}

/// Liste saisie à la main : virgules, points-virgules ou retours à la ligne.
fn list(raw: Option<&str>) -> Vec<String> {
    raw.map(|text| {
        text.split([',', ';', '\n'])
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(str::to_string)
            .collect()
    })
    .unwrap_or_default()
}

impl Options {
    pub fn from_target(target: &Target) -> Result<Self, ProbeError> {
        let unencrypted = parse_bool(tag(target, "allow_unencrypted"))?;
        let (scheme, default_port) =
            if unencrypted { ("http", DEFAULT_HTTP_PORT) } else { ("https", DEFAULT_PORT) };
        let port = parse_port(tag(target, "port"), default_port)?;
        let root = base_url(&target.address, scheme, port)?;
        if root.starts_with("http://") && !unencrypted {
            return Err(ProbeError::Config(
                "Unencrypted WinRM (http://) is refused: use the HTTPS listener on port 5986, or \
                 tick Allow unencrypted traffic knowing the password then travels in clear."
                    .to_string(),
            ));
        }
        let auth = match tag(target, "auth").map(str::to_ascii_lowercase).as_deref() {
            None | Some("ntlm") => AuthMode::Ntlm,
            Some("basic") => AuthMode::Basic,
            Some(other) => {
                return Err(ProbeError::Config(format!(
                    "Unknown authentication \"{other}\": ntlm or basic"
                )));
            }
        };
        if auth == AuthMode::Basic && root.starts_with("http://") {
            return Err(ProbeError::Config(
                "Basic authentication is only allowed over HTTPS: the password would travel in \
                 clear."
                    .to_string(),
            ));
        }
        let event_minutes = match tag(target, "event_minutes") {
            None => DEFAULT_EVENT_MINUTES,
            Some(raw) => {
                raw.parse::<u64>().ok().filter(|m| (1..=1440).contains(m)).ok_or_else(|| {
                    ProbeError::Config(format!(
                        "Invalid event window \"{raw}\": between 1 and 1440 minutes"
                    ))
                })?
            }
        };
        // Absent ou vide : les journaux par défaut ; `none` désactive le comptage.
        let event_logs = match tag(target, "event_logs") {
            Some(raw) if raw.eq_ignore_ascii_case("none") => Vec::new(),
            raw => list(Some(raw.unwrap_or(DEFAULT_EVENT_LOGS))),
        };
        if event_logs.len() > MAX_EVENT_LOGS {
            return Err(ProbeError::Config(format!(
                "At most {MAX_EVENT_LOGS} event logs can be counted"
            )));
        }
        for log in &event_logs {
            // Le nom est inséré dans la requête WQL : jeu de caractères fermé.
            if !log.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '_' | '.' | '-'))
            {
                return Err(ProbeError::Config(format!("Invalid event log name \"{log}\"")));
            }
        }
        Ok(Self {
            endpoint: format!("{root}/wsman"),
            auth,
            insecure_tls: parse_bool(tag(target, "insecure_tls"))?,
            timeout: parse_timeout(tag(target, "request_timeout_seconds"))?,
            services: list(tag(target, "services")),
            ignore_services: list(tag(target, "ignore_services")),
            event_logs,
            event_minutes,
        })
    }
}

/// Compte `DOMAINE\nom` → (nom, domaine). Un UPN (`nom@domaine`) ou un compte
/// local seul garde un domaine vide : Windows le résout lui-même.
fn split_account(username: &str) -> (&str, &str) {
    match username.split_once('\\') {
        Some((domain, user)) => (user, domain),
        None => (username, ""),
    }
}

fn random_bytes<const N: usize>() -> Result<[u8; N], ProbeError> {
    let mut bytes = [0u8; N];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| ProbeError::Config("No random source available".to_string()))?;
    Ok(bytes)
}

/// Identifiant de message WS-Addressing (forme UUID, aléatoire).
fn message_id() -> String {
    let b = random_bytes::<16>().unwrap_or([7; 16]);
    format!(
        "{:02X}{:02X}{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        b[0],
        b[1],
        b[2],
        b[3],
        b[4],
        b[5],
        b[6],
        b[7],
        b[8],
        b[9],
        b[10],
        b[11],
        b[12],
        b[13],
        b[14],
        b[15]
    )
}

/// Une session WinRM : un client HTTP à une seule connexion (l'authentification
/// NTLM est liée à la connexion TCP) et l'état de la poignée de main.
struct Session {
    http: reqwest::Client,
    options: Options,
    username: String,
    password: String,
    authenticated: bool,
}

impl Session {
    fn new(options: Options, username: String, password: String) -> Result<Self, ProbeError> {
        let http = reqwest::Client::builder()
            .danger_accept_invalid_certs(options.insecure_tls)
            .connect_timeout(Duration::from_secs(10))
            // L'authentification NTLM vaut pour la connexion : une seule, gardée.
            .pool_max_idle_per_host(1)
            .http1_only()
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(concat!("DumbMonit/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| ProbeError::Config(format!("HTTP client unavailable: {e}")))?;
        Ok(Self { http, options, username, password, authenticated: false })
    }

    fn transport(&self, error: &reqwest::Error) -> ProbeError {
        if error.is_timeout() {
            return ProbeError::Timeout(self.options.timeout);
        }
        let detail = format!("{error:?}").to_lowercase();
        if detail.contains("certificate") || detail.contains("unknownissuer") {
            return ProbeError::Config(format!(
                "The TLS certificate of {} is not trusted: install the authority that signed it \
                 or tick Accept an unverifiable certificate ({error})",
                self.options.endpoint
            ));
        }
        ProbeError::Unreachable(format!("cannot reach {}: {error}", self.options.endpoint))
    }

    /// Un POST ; rend statut, défis `WWW-Authenticate` et corps.
    async fn post(
        &self,
        body: String,
        authorization: Option<String>,
    ) -> Result<(StatusCode, Vec<String>, String), ProbeError> {
        let mut request = self
            .http
            .post(&self.options.endpoint)
            .header(CONTENT_TYPE, "application/soap+xml;charset=UTF-8")
            .timeout(self.options.timeout)
            .body(body);
        if let Some(value) = authorization {
            request = request.header(AUTHORIZATION, value);
        }
        let response = request.send().await.map_err(|e| self.transport(&e))?;
        let status = response.status();
        let challenges = response
            .headers()
            .get_all(WWW_AUTHENTICATE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .map(str::to_string)
            .collect();
        let text = response.text().await.map_err(|e| self.transport(&e))?;
        Ok((status, challenges, text))
    }

    /// Poignée de main NTLM en trois temps ; la requête réelle porte le
    /// troisième message.
    async fn ntlm_exchange(&mut self, body: String) -> Result<(StatusCode, String), ProbeError> {
        let (user, domain) = split_account(&self.username);
        let negotiate = format!("Negotiate {}", B64.encode(ntlm::negotiate_message()));
        let (status, challenges, _) = self.post(String::new(), Some(negotiate)).await?;
        if status != StatusCode::UNAUTHORIZED {
            return Err(ProbeError::Protocol(format!(
                "WinRM answered {status} to the NTLM handshake instead of a challenge"
            )));
        }
        let token = challenges
            .iter()
            .find_map(|header| {
                let (scheme, rest) = header.split_once(' ')?;
                (scheme.eq_ignore_ascii_case("negotiate") || scheme.eq_ignore_ascii_case("ntlm"))
                    .then(|| rest.trim().to_string())
            })
            .ok_or_else(|| {
                ProbeError::Config(
                    "WinRM offers no Negotiate/NTLM authentication: enable it on the service \
                     (winrm set winrm/config/service/auth @{Negotiate=\"true\"}) or use Basic over \
                     HTTPS."
                        .to_string(),
                )
            })?;
        let raw = B64
            .decode(token.as_bytes())
            .map_err(|_| ProbeError::Protocol("Invalid NTLM challenge encoding".to_string()))?;
        let challenge = ntlm::parse_challenge(&raw).ok_or_else(|| {
            ProbeError::Config(
                "WinRM did not answer with an NTLM challenge (Kerberos only?): DumbMonit \
                 supports NTLM and Basic."
                    .to_string(),
            )
        })?;
        let message = ntlm::authenticate_message(
            &challenge,
            user,
            domain,
            &self.password,
            random_bytes::<8>()?,
            ntlm::filetime(chrono::Utc::now().timestamp()),
        );
        let (status, _, text) =
            self.post(body, Some(format!("Negotiate {}", B64.encode(message)))).await?;
        Ok((status, text))
    }

    /// Envoie une enveloppe SOAP et rend le corps de la réponse.
    async fn send(&mut self, body: String) -> Result<String, ProbeError> {
        for attempt in 0..2 {
            let (status, text) = match self.options.auth {
                AuthMode::Basic => {
                    let credentials = B64.encode(format!("{}:{}", self.username, self.password));
                    let (status, _, text) =
                        self.post(body.clone(), Some(format!("Basic {credentials}"))).await?;
                    (status, text)
                }
                AuthMode::Ntlm if self.authenticated => {
                    let (status, _, text) = self.post(body.clone(), None).await?;
                    (status, text)
                }
                AuthMode::Ntlm => self.ntlm_exchange(body.clone()).await?,
            };
            if status == StatusCode::UNAUTHORIZED {
                // Connexion fermée côté serveur : une seule nouvelle poignée de main.
                if self.options.auth == AuthMode::Ntlm && self.authenticated && attempt == 0 {
                    self.authenticated = false;
                    continue;
                }
                return Err(ProbeError::Auth(format!(
                    "Windows refused the login of {} (401). Check the user name \
                     (DOMAIN\\user or user@domain), the password and that the account is in the \
                     Remote Management Users group.",
                    self.username
                )));
            }
            if self.options.auth == AuthMode::Ntlm {
                self.authenticated = true;
            }
            return self.interpret(status, text);
        }
        Err(ProbeError::Auth("Windows refused the login (401)".to_string()))
    }

    fn interpret(&self, status: StatusCode, text: String) -> Result<String, ProbeError> {
        match status.as_u16() {
            // Une faute SOAP arrive en 500 (parfois 400) : `parse_page` la traduit.
            200 | 400 | 500 => Ok(text),
            403 => Err(ProbeError::Auth(
                "WinRM answered 403: the account is not allowed to use remote management."
                    .to_string(),
            )),
            404 => Err(ProbeError::Config(format!(
                "No WinRM listener answered at {}: check the port and that the HTTPS listener \
                 exists (winrm enumerate winrm/config/listener).",
                self.options.endpoint
            ))),
            502..=504 => {
                Err(ProbeError::Unreachable(format!("{} answered {status}", self.options.endpoint)))
            }
            _ => Err(ProbeError::Protocol(format!("WinRM answered {status}"))),
        }
    }

    /// Une requête WQL, pages suivantes comprises (au plus `max_pages`).
    async fn query(
        &mut self,
        wql: &str,
        page_size: u32,
        max_pages: usize,
    ) -> Result<Vec<Row>, ProbeError> {
        let operation_secs = self.options.timeout.as_secs().saturating_sub(1).max(1);
        let endpoint = self.options.endpoint.clone();
        let mut request =
            wsman::enumerate_request(&endpoint, &message_id(), operation_secs, wql, page_size);
        let mut rows = Vec::new();
        for _ in 0..max_pages {
            let body = self.send(request).await?;
            let page = wsman::parse_page(&body)?;
            rows.extend(page.rows);
            match page.context {
                Some(context) => {
                    request = wsman::pull_request(
                        &endpoint,
                        &message_id(),
                        operation_secs,
                        &context,
                        page_size,
                    );
                }
                None => break,
            }
        }
        Ok(rows)
    }

    /// Lecture facultative : refusée ou en erreur de requête, elle vaut `None` ;
    /// une panne réelle (injoignable, délai) fait échouer toute l'interrogation.
    async fn optional(
        &mut self,
        section: &str,
        wql: &str,
        page_size: u32,
        max_pages: usize,
    ) -> Result<Option<Vec<Row>>, ProbeError> {
        match self.query(wql, page_size, max_pages).await {
            Ok(rows) => Ok(Some(rows)),
            Err(error) if error.means_down() => Err(error),
            Err(error) => {
                tracing::debug!(%error, section, "WinRM : lecture facultative indisponible");
                Ok(None)
            }
        }
    }
}

fn credentials(target: &Target) -> Result<(String, String), ProbeError> {
    match &target.credential {
        Credential::UsernamePassword { username, password } if !username.trim().is_empty() => {
            Ok((username.trim().to_string(), password.clone()))
        }
        other => Err(ProbeError::Config(format!(
            "WinRM expects a user name and password, configured: {other}"
        ))),
    }
}

/// Ce que les requêtes ont rendu ; `None` = section illisible.
pub struct Replies {
    pub os: Vec<Row>,
    pub cpu: Option<Vec<Row>>,
    pub pagefile: Option<Vec<Row>>,
    pub disks: Option<Vec<Row>>,
    pub services: Option<Vec<Row>>,
    /// Un relevé par journal compté : (journal, événements de la fenêtre).
    pub events: Vec<(String, Option<Vec<Row>>)>,
}

#[async_trait]
impl Collector for WinrmCollector {
    fn kind(&self) -> &'static str {
        "winrm"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let options = Options::from_target(target)?;
        let (username, password) = credentials(target)?;
        let mut session = Session::new(options.clone(), username, password)?;
        let os = session.query(OS_WQL, PAGE_SIZE, 1).await?;
        if os.is_empty() {
            return Err(ProbeError::Protocol("Win32_OperatingSystem returned nothing".to_string()));
        }
        let now = now_ms();
        let since =
            chrono::DateTime::from_timestamp_millis(now - options.event_minutes as i64 * 60 * 1000)
                .unwrap_or_default()
                .format("%Y%m%d%H%M%S.000000+000")
                .to_string();
        let replies = Replies {
            os,
            cpu: session.optional("cpu", CPU_WQL, PAGE_SIZE, MAX_PAGES).await?,
            pagefile: session.optional("pagefile", PAGEFILE_WQL, PAGE_SIZE, MAX_PAGES).await?,
            disks: session.optional("disks", DISKS_WQL, PAGE_SIZE, MAX_PAGES).await?,
            services: session.optional("services", SERVICES_WQL, PAGE_SIZE, MAX_PAGES).await?,
            events: {
                let mut events = Vec::new();
                for log in &options.event_logs {
                    let wql = event_wql(log, &since);
                    let rows = session
                        .optional("eventlog", &wql, EVENT_PAGE_SIZE, EVENT_MAX_PAGES)
                        .await?;
                    events.push((log.clone(), rows));
                }
                events
            },
        };
        Ok(samples(&replies, &options, now))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let options = Options::from_target(target)?;
        let (username, password) = credentials(target)?;
        let mut session = Session::new(options, username, password)?;
        session.query(OS_WQL, PAGE_SIZE, 1).await?;
        Ok(Some("winrm".to_string()))
    }
}

/// Requête de comptage d'un journal : les erreurs, ou les échecs d'audit pour
/// `Security`, depuis `since` (format DMTF, UTC).
fn event_wql(log: &str, since: &str) -> String {
    let kind = if log.eq_ignore_ascii_case("security") { "Audit Failure" } else { "Error" };
    format!(
        "SELECT EventCode, SourceName FROM Win32_NTLogEvent WHERE Logfile = '{log}' AND Type = \
         '{kind}' AND TimeGenerated >= '{since}'"
    )
}

fn number(row: &Row, key: &str) -> Option<f64> {
    row.get(key)?.trim().parse::<f64>().ok().filter(|v| v.is_finite())
}

fn percent(used: f64, total: f64) -> f64 {
    if total <= 0.0 { 0.0 } else { used * 100.0 / total }
}

/// Date WMI en secondes Unix : ISO 8601 (réponse WS-Man) ou DMTF
/// `yyyymmddHHMMSS.ffffff±mmm`.
pub fn parse_wmi_time(raw: &str) -> Option<i64> {
    let raw = raw.trim();
    if let Ok(time) = chrono::DateTime::parse_from_rfc3339(raw) {
        return Some(time.timestamp());
    }
    if raw.len() < 25 || !raw.is_char_boundary(14) {
        return None;
    }
    let naive = chrono::NaiveDateTime::parse_from_str(&raw[..14], "%Y%m%d%H%M%S").ok()?;
    let sign = match raw.as_bytes().get(21)? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let offset_minutes: i64 = raw.get(22..25)?.parse().ok()?;
    Some(naive.and_utc().timestamp() - sign * offset_minutes * 60)
}

/// Nom correspondant à un motif, avec `*` en tête ou en queue, sans casse.
fn matches_pattern(pattern: &str, name: &str) -> bool {
    let (pattern, name) = (pattern.to_lowercase(), name.to_lowercase());
    match (pattern.strip_prefix('*'), pattern.strip_suffix('*')) {
        (Some(rest), _) if rest.ends_with('*') => name.contains(rest.trim_end_matches('*')),
        (Some(rest), _) => name.ends_with(rest),
        (_, Some(rest)) => name.starts_with(rest),
        _ => name == pattern,
    }
}

fn readable(section: &str, ok: bool, ts_ms: i64) -> Sample {
    flag("winrm_section_readable", ok, ts_ms).with_label("section", section)
}

pub fn samples(r: &Replies, options: &Options, ts_ms: i64) -> Vec<Sample> {
    let g = |name: &str, value: f64| gauge(name, value, ts_ms);
    let mut out = Vec::new();

    if let Some(os) = r.os.first() {
        let label = |key: &str| os.get(key).cloned().unwrap_or_default();
        out.push(
            g("windows_version_info", 1.0)
                .with_label("caption", label("Caption"))
                .with_label("version", label("Version"))
                .with_label("build", label("BuildNumber")),
        );
        // Les tailles WMI sont en kilo-octets.
        if let (Some(total), Some(free)) =
            (number(os, "TotalVisibleMemorySize"), number(os, "FreePhysicalMemory"))
        {
            let (total, free) = (total * 1024.0, free * 1024.0);
            let used = (total - free).max(0.0);
            out.push(g("memory_total_bytes", total));
            out.push(g("memory_used_bytes", used));
            out.push(g("memory_available_bytes", free));
            out.push(g("memory_used_percent", percent(used, total)));
        }
        if let Some(boot) = os.get("LastBootUpTime").and_then(|t| parse_wmi_time(t)) {
            out.push(g("uptime_seconds", (ts_ms as f64 / 1000.0 - boot as f64).max(0.0)));
            out.push(g("boot_time_seconds", boot as f64));
        }
        if let Some(count) = number(os, "NumberOfProcesses") {
            out.push(g("process_count", count));
        }
    }

    match &r.cpu {
        Some(rows) => {
            out.push(readable("cpu", true, ts_ms));
            let mut cores = 0usize;
            for row in rows {
                let (Some(name), Some(load)) =
                    (row.get("Name"), number(row, "PercentProcessorTime"))
                else {
                    continue;
                };
                if name == "_Total" {
                    out.push(g("cpu_usage_percent", load));
                } else {
                    cores += 1;
                    if cores <= MAX_NAMED {
                        out.push(g("cpu_core_usage_percent", load).with_label("core", name));
                    }
                }
            }
            if cores > 0 {
                out.push(g("cpu_count", cores as f64));
            }
        }
        None => out.push(readable("cpu", false, ts_ms)),
    }

    match &r.pagefile {
        Some(rows) => {
            out.push(readable("pagefile", true, ts_ms));
            let mb = 1024.0 * 1024.0;
            let total: f64 = rows.iter().filter_map(|row| number(row, "AllocatedBaseSize")).sum();
            let used: f64 = rows.iter().filter_map(|row| number(row, "CurrentUsage")).sum();
            out.push(g("swap_total_bytes", total * mb));
            out.push(g("swap_used_bytes", used * mb));
            out.push(g("swap_used_percent", percent(used, total)));
        }
        None => out.push(readable("pagefile", false, ts_ms)),
    }

    match &r.disks {
        Some(rows) => {
            out.push(readable("disks", true, ts_ms));
            for row in rows.iter().take(MAX_NAMED) {
                let (Some(id), Some(size), Some(free)) =
                    (row.get("DeviceID"), number(row, "Size"), number(row, "FreeSpace"))
                else {
                    continue;
                };
                let used = (size - free).max(0.0);
                let named = |sample: Sample| {
                    sample
                        .with_label("mountpoint", format!("{id}\\"))
                        .with_label("device", id.as_str())
                        .with_label("fstype", row.get("FileSystem").cloned().unwrap_or_default())
                };
                out.push(named(g("filesystem_total_bytes", size)));
                out.push(named(g("filesystem_used_bytes", used)));
                out.push(named(g("filesystem_free_bytes", free)));
                out.push(named(g("filesystem_used_percent", percent(used, size))));
            }
        }
        None => out.push(readable("disks", false, ts_ms)),
    }

    match &r.services {
        Some(rows) => {
            out.push(readable("services", true, ts_ms));
            out.extend(service_samples(rows, options, ts_ms));
        }
        None => out.push(readable("services", false, ts_ms)),
    }

    if !r.events.is_empty() {
        out.push(readable("eventlog", r.events.iter().all(|(_, rows)| rows.is_some()), ts_ms));
    }
    for (log, rows) in &r.events {
        let Some(rows) = rows else { continue };
        out.push(g("windows_eventlog_errors", rows.len() as f64).with_label("log", log.as_str()));
        let mut counts: BTreeMap<(String, String), usize> = BTreeMap::new();
        for row in rows {
            let id = row.get("EventCode").cloned().unwrap_or_default();
            let source = row.get("SourceName").cloned().unwrap_or_default();
            *counts.entry((id, source)).or_default() += 1;
        }
        let mut ranked: Vec<_> = counts.into_iter().collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        for ((id, source), count) in ranked.into_iter().take(TOP_EVENTS) {
            out.push(
                g("windows_eventlog_events", count as f64)
                    .with_label("log", log.as_str())
                    .with_label("event_id", id)
                    .with_label("source", source),
            );
        }
    }
    out
}

fn service_samples(rows: &[Row], options: &Options, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let running = |row: &Row| row.get("State").is_some_and(|s| s.eq_ignore_ascii_case("Running"));
    let name_of = |row: &Row| row.get("Name").cloned().unwrap_or_default();

    // Services choisis : un service absent de la machine compte comme arrêté.
    let mut followed: Vec<String> = Vec::new();
    for wanted in &options.services {
        match rows.iter().find(|row| name_of(row).eq_ignore_ascii_case(wanted)) {
            Some(row) => {
                followed.push(name_of(row));
                out.push(
                    flag("service_up", running(row), ts_ms).with_label("service", name_of(row)),
                );
            }
            None => {
                followed.push(wanted.clone());
                out.push(flag("service_up", false, ts_ms).with_label("service", wanted.as_str()));
            }
        }
    }

    // Tous les services « Automatique » arrêtés, hors exceptions.
    let mut stopped = 0usize;
    for row in rows {
        let automatic = row.get("StartMode").is_some_and(|m| m.eq_ignore_ascii_case("Auto"));
        let name = name_of(row);
        if !automatic || running(row) || name.is_empty() {
            continue;
        }
        let display = row.get("DisplayName").map(String::as_str).unwrap_or_default();
        let ignored = BUILTIN_IGNORED
            .iter()
            .copied()
            .chain(options.ignore_services.iter().map(String::as_str))
            .any(|pattern| matches_pattern(pattern, &name) || matches_pattern(pattern, display));
        if ignored {
            continue;
        }
        stopped += 1;
        if followed.iter().any(|f| f.eq_ignore_ascii_case(&name)) || stopped > MAX_NAMED {
            continue;
        }
        out.push(flag("service_up", false, ts_ms).with_label("service", name.as_str()));
        out.push(
            flag("windows_service_auto_stopped", true, ts_ms).with_label("service", name.as_str()),
        );
    }
    out.push(gauge("windows_services_auto_stopped", stopped as f64, ts_ms));
    out
}

#[cfg(test)]
mod tests;
