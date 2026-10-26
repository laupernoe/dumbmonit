//! QNAP (QTS et QuTS hero), par les scripts CGI de l'interface web.
//!
//! QNAP ne publie pas d'API REST documentée pour l'état du NAS : les clients
//! de référence (l'intégration Home Assistant `qnap`, qui s'appuie sur
//! `python-qnapstats`) lisent les mêmes pages XML que l'interface. Ce module
//! fait de même, en lecture seule.
//!
//! Une session par interrogation : `POST /cgi-bin/authLogin.cgi` (nom et mot
//! de passe encodé en base64 dans le corps, jamais dans l'URL) rend un
//! identifiant de session `authSid`, passé ensuite en paramètre `sid` des
//! lectures, puis la session est fermée (`authLogout.cgi`) même si une
//! lecture a échoué : QTS limite le nombre de sessions ouvertes. Comme l'URL
//! porte l'identifiant, il est retiré de tout message d'erreur.
//!
//! Une lecture obligatoire — `manaRequest.cgi?subfunc=sysinfo` : processeur,
//! mémoire, températures, durée de fonctionnement — et trois facultatives, qui
//! ne font pas échouer le reste si le compte ne peut pas les lire ou si la
//! version de QTS les sert autrement : les volumes (`chartReq.cgi`), les
//! disques et leur état SMART (`disk_manage.cgi`), la mise à jour du
//! micrologiciel (`sysRequest.cgi?subfunc=firm_update`).
//!
//! Les pannes que ce module existe pour voir :
//!
//! * **un volume qui se remplit** ;
//! * **un disque dont l'état SMART n'est plus bon**, ou trop chaud ;
//! * **la température** du processeur ou du boîtier ;
//! * **une mise à jour de QTS** proposée.
//!
//! L'état des grappes RAID (dégradée, reconstruction) n'est pas lu : aucun
//! client de référence ne le documente.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `scheme` | `https` | Protocole de l'interface web. |
//! | `port` | `443` | Port de l'interface web (8080 en HTTP). |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable (celui d'origine est auto-signé). |
//! | `request_timeout_seconds` | `15` | Délai par requête HTTP. |

use async_trait::async_trait;
use base64::Engine;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};
use roxmltree::{Document, Node};
use serde_json::Value;

use crate::rest::{MAX_NAMED, RestClient, flag, gauge, now_ms, number};

pub const DEFAULT_PORT: u16 = 443;

const LOGIN: &str = "/cgi-bin/authLogin.cgi";
const LOGOUT: &str = "/cgi-bin/authLogout.cgi?logout=1";
const SYSINFO: &str = "/cgi-bin/management/manaRequest.cgi?subfunc=sysinfo&hd=no&multicpu=1";
const VOLUMES: &str =
    "/cgi-bin/management/chartReq.cgi?chart_func=disk_usage&disk_select=all&include=all";
const DISKS: &str = "/cgi-bin/disk/disk_manage.cgi?func=extra_get&disk_list=1&smart=1";
const FIRMWARE: &str = "/cgi-bin/sys/sysRequest.cgi?subfunc=firm_update";
const MIB: f64 = 1024.0 * 1024.0;

#[derive(Default)]
pub struct QnapCollector;

impl QnapCollector {
    pub fn new() -> Self {
        Self
    }
}

/// Ce que la réponse de connexion apprend du NAS.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Login {
    pub sid: String,
    pub model: String,
    pub hostname: String,
    pub version: String,
    pub build: String,
}

/// Le texte du premier élément de ce nom sous `node`, sans blancs, s'il n'est
/// pas vide.
fn find<'a>(node: Node<'a, '_>, name: &str) -> Option<&'a str> {
    node.descendants()
        .find(|n| n.is_element() && n.tag_name().name() == name)
        .and_then(|n| n.text())
        .map(str::trim)
        .filter(|t| !t.is_empty())
}

/// Un nombre dans un texte (`7.8 %`, `36 C/96 F`).
fn value_of(raw: &str) -> Option<f64> {
    number(Some(&Value::String(raw.to_string())))
}

fn num(node: Node<'_, '_>, name: &str) -> Option<f64> {
    find(node, name).and_then(value_of)
}

/// Analyse la réponse de `authLogin.cgi`.
pub fn parse_login(body: &str) -> Result<Login, ProbeError> {
    let document = Document::parse(body).map_err(|_| {
        ProbeError::Protocol(
            "The answer of /cgi-bin/authLogin.cgi is not XML: this does not look like QTS, check \
             the address and the port (443, or 8080 over HTTP)."
                .to_string(),
        )
    })?;
    let root = document.root_element();
    if root.tag_name().name() != "QDocRoot" {
        return Err(ProbeError::Protocol(
            "The answer of /cgi-bin/authLogin.cgi is not a QTS answer: check the address and the \
             port."
                .to_string(),
        ));
    }
    if find(root, "need_2sv") == Some("1") {
        return Err(ProbeError::Config(
            "The QNAP account uses 2-step verification, which a monitor cannot answer: create a \
             dedicated account without it."
                .to_string(),
        ));
    }
    match (find(root, "authPassed"), find(root, "authSid")) {
        (Some("1"), Some(sid)) => {
            let firmware = root.descendants().find(|n| n.has_tag_name("firmware"));
            let from_firmware = |name: &str| {
                firmware.and_then(|node| find(node, name)).unwrap_or_default().to_string()
            };
            Ok(Login {
                sid: sid.to_string(),
                model: find(root, "modelName").unwrap_or_default().to_string(),
                hostname: find(root, "hostName").unwrap_or_default().to_string(),
                version: from_firmware("version"),
                build: from_firmware("build"),
            })
        }
        _ => Err(ProbeError::Auth(
            "QNAP refused the login: check the user name and the password of the DumbMonit \
             account, and that it is enabled."
                .to_string(),
        )),
    }
}

/// Une réponse de lecture dont l'identifiant de session n'est plus accepté :
/// QTS rend alors `authPassed` à 0 (ou la page de connexion).
fn session_refused(body: &str) -> bool {
    match Document::parse(body) {
        Ok(document) => find(document.root_element(), "authPassed") == Some("0"),
        Err(_) => false,
    }
}

/// Retire un secret d'un message d'erreur (l'identifiant de session est dans
/// l'URL, que `reqwest` recopie dans ses erreurs).
fn scrub(error: ProbeError, secret: &str) -> ProbeError {
    if secret.is_empty() {
        return error;
    }
    let clean = |message: String| message.replace(secret, "***");
    match error {
        ProbeError::Unreachable(message) => ProbeError::Unreachable(clean(message)),
        ProbeError::Protocol(message) => ProbeError::Protocol(clean(message)),
        ProbeError::Auth(message) => ProbeError::Auth(clean(message)),
        ProbeError::Config(message) => ProbeError::Config(clean(message)),
        other => other,
    }
}

struct Api {
    client: RestClient,
    username: String,
    password: String,
    sid: String,
}

impl Api {
    fn new(target: &Target) -> Result<Self, ProbeError> {
        let (username, password) = match &target.credential {
            Credential::UsernamePassword { username, password } if !username.trim().is_empty() => {
                (username.trim().to_string(), password.clone())
            }
            other => {
                return Err(ProbeError::Config(format!(
                    "QNAP expects a user name and password, configured: {other}"
                )));
            }
        };
        let client = RestClient::for_target(target, "https", DEFAULT_PORT, "QNAP")?;
        Ok(Self { client, username, password, sid: String::new() })
    }

    async fn login(&mut self) -> Result<Login, ProbeError> {
        let encoded = base64::engine::general_purpose::STANDARD.encode(&self.password);
        let form =
            [("user", self.username.as_str()), ("pwd", encoded.as_str()), ("serviceKey", "1")];
        let (status, body) = self.client.post_form(LOGIN, &form).await?;
        if !status.is_success() {
            return Err(self.client.status_error(status, &body, LOGIN));
        }
        let login = parse_login(&body)?;
        self.sid.clone_from(&login.sid);
        Ok(login)
    }

    /// Fermeture de la session, sans conséquence si elle échoue.
    async fn logout(&self) {
        if self.sid.is_empty() {
            return;
        }
        if let Err(error) = self.get(LOGOUT).await {
            tracing::debug!(%error, "QNAP : fermeture de session impossible");
        }
    }

    /// Un GET avec l'identifiant de session ; le corps est du XML.
    async fn get(&self, path: &str) -> Result<String, ProbeError> {
        let full = format!("{path}&sid={}", self.sid);
        let (status, body) =
            self.client.get(&full).await.map_err(|error| scrub(error, &self.sid))?;
        if !status.is_success() {
            return Err(scrub(self.client.status_error(status, &body, path), &self.sid));
        }
        if session_refused(&body) {
            return Err(ProbeError::Auth(format!(
                "QNAP refused the session on {path}: the account may lack the right to read it."
            )));
        }
        Ok(body)
    }

    /// Une lecture facultative : un refus ou une page inattendue ne fait rien
    /// échouer, une panne réelle si.
    async fn optional(&self, path: &str) -> Result<Option<String>, ProbeError> {
        match self.get(path).await {
            Ok(body) => Ok(Some(body)),
            Err(error) if error.means_down() => Err(error),
            Err(error) => {
                tracing::debug!(path, %error, "QNAP : lecture facultative indisponible");
                Ok(None)
            }
        }
    }

    async fn read(&self, login: Login) -> Result<Replies, ProbeError> {
        let sysinfo = self.get(SYSINFO).await?;
        if Document::parse(&sysinfo).is_err() {
            return Err(ProbeError::Protocol(format!(
                "QNAP did not answer in XML on {SYSINFO}: check the address and the port."
            )));
        }
        let (volumes, disks, firmware) =
            tokio::join!(self.optional(VOLUMES), self.optional(DISKS), self.optional(FIRMWARE),);
        Ok(Replies { login, sysinfo, volumes: volumes?, disks: disks?, firmware: firmware? })
    }
}

#[async_trait]
impl Collector for QnapCollector {
    fn kind(&self) -> &'static str {
        "qnap"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let mut api = Api::new(target)?;
        let login = api.login().await?;
        let replies = api.read(login).await;
        api.logout().await;
        Ok(samples(&replies?, now_ms()))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let mut api = Api::new(target)?;
        api.login().await?;
        let outcome = api.get(SYSINFO).await;
        api.logout().await;
        outcome?;
        Ok(Some("qnap".to_string()))
    }
}

pub struct Replies {
    pub login: Login,
    pub sysinfo: String,
    pub volumes: Option<String>,
    pub disks: Option<String>,
    pub firmware: Option<String>,
}

#[derive(Debug, PartialEq)]
enum Health {
    Good,
    Warning,
    Failed,
}

fn health(raw: &str) -> Health {
    let raw = raw.trim().to_ascii_lowercase();
    if matches!(raw.as_str(), "ok" | "good" | "normal" | "ready") {
        Health::Good
    } else if raw.contains("warn") {
        Health::Warning
    } else {
        Health::Failed
    }
}

pub fn samples(r: &Replies, ts_ms: i64) -> Vec<Sample> {
    let g = |name: &str, value: f64| gauge(name, value, ts_ms);
    let mut out = Vec::new();

    if let Ok(document) = Document::parse(&r.sysinfo) {
        let root = document.root_element();
        let model = find(root, "displayModelName")
            .or_else(|| find(root, "modelName"))
            .unwrap_or(r.login.model.as_str());
        let hostname = find(root, "server_name").unwrap_or(r.login.hostname.as_str());
        out.push(
            g("qnap_version_info", 1.0)
                .with_label("version", r.login.version.as_str())
                .with_label("build", r.login.build.as_str())
                .with_label("model", model)
                .with_label("hostname", hostname),
        );
        for (field, metric) in [
            ("cpu_usage", "qnap_cpu_usage_percent"),
            ("cpu_tempc", "qnap_cpu_temperature_celsius"),
            ("sys_tempc", "qnap_system_temperature_celsius"),
        ] {
            if let Some(value) = num(root, field) {
                out.push(g(metric, value));
            }
        }
        // Mémoire en Mio dans la réponse.
        if let (Some(total), Some(free)) = (num(root, "total_memory"), num(root, "free_memory"))
            && total > 0.0
        {
            out.push(g("qnap_memory_total_bytes", total * MIB));
            let used = ((total - free) / total * 100.0).clamp(0.0, 100.0);
            out.push(g("qnap_memory_used_percent", used));
        }
        let part = |name: &str| num(root, name);
        if let (Some(days), Some(hours), Some(minutes)) =
            (part("uptime_day"), part("uptime_hour"), part("uptime_min"))
        {
            let seconds = part("uptime_sec").unwrap_or(0.0);
            let uptime = days * 86400.0 + hours * 3600.0 + minutes * 60.0 + seconds;
            out.push(g("qnap_uptime_seconds", uptime));
        }
    }

    volumes(r, ts_ms, &mut out);
    disks(r, ts_ms, &mut out);

    if let Some(Ok(document)) = r.firmware.as_deref().map(Document::parse) {
        let newer = find(document.root_element(), "newVersion");
        out.push(flag("qnap_firmware_update_available", newer.is_some(), ts_ms));
        if let Some(version) = newer {
            out.push(g("qnap_firmware_latest_info", 1.0).with_label("version", version));
        }
    }
    out
}

fn volumes(r: &Replies, ts_ms: i64, out: &mut Vec<Sample>) {
    let g = |name: &str, value: f64| gauge(name, value, ts_ms);
    let Some(Ok(document)) = r.volumes.as_deref().map(Document::parse) else { return };
    let mut count = 0;
    for volume in document.descendants().filter(|n| n.is_element() && n.has_tag_name("volume")) {
        let (Some(total), Some(free)) = (num(volume, "totalsize"), num(volume, "freesize")) else {
            continue;
        };
        count += 1;
        if count > MAX_NAMED {
            continue;
        }
        let name = find(volume, "volumeLabel")
            .map(str::to_string)
            .or_else(|| find(volume, "volumeNo").map(|no| format!("volume {no}")))
            .unwrap_or_else(|| format!("volume {count}"));
        let named = |sample: Sample| sample.with_label("volume", name.as_str());
        out.push(named(g("qnap_volume_size_bytes", total)));
        out.push(named(g("qnap_volume_free_bytes", free)));
        if total > 0.0 {
            let used = ((total - free) / total * 100.0).clamp(0.0, 100.0);
            out.push(named(g("qnap_volume_used_percent", used)));
        }
    }
    out.push(g("qnap_volumes", count as f64));
}

fn disks(r: &Replies, ts_ms: i64, out: &mut Vec<Sample>) {
    let g = |name: &str, value: f64| gauge(name, value, ts_ms);
    let Some(Ok(document)) = r.disks.as_deref().map(Document::parse) else { return };
    let (mut count, mut unhealthy) = (0, 0);
    for entry in document.descendants().filter(|n| n.is_element() && n.has_tag_name("entry")) {
        let Some(slot) = find(entry, "HDNo") else { continue };
        // Un emplacement vide rend des tirets à la place du modèle.
        let Some(model) = find(entry, "Model").filter(|m| m.chars().any(|c| c.is_alphanumeric()))
        else {
            continue;
        };
        let state = health(find(entry, "Health").or_else(|| find(entry, "Status")).unwrap_or(""));
        count += 1;
        unhealthy += usize::from(state != Health::Good);
        if count > MAX_NAMED {
            continue;
        }
        let disk = format!("disk {slot}");
        let named =
            |sample: Sample| sample.with_label("disk", disk.as_str()).with_label("model", model);
        out.push(named(flag("qnap_disk_ok", state == Health::Good, ts_ms)));
        out.push(named(flag("qnap_disk_warning", state == Health::Warning, ts_ms)));
        if let Some(temperature) = num(entry, "Temperature") {
            out.push(named(g("qnap_disk_temperature_celsius", temperature)));
        }
    }
    out.push(g("qnap_disks", count as f64));
    out.push(g("qnap_disks_unhealthy", unhealthy as f64));
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use axum::Router;
    use axum::extract::Form;
    use axum::http::{StatusCode, Uri};
    use axum::routing::{get, post};

    use super::*;
    use crate::rest::test_support::{find as find_sample, serve, value};
    use crate::uptime::tags::test_support::cible;

    // Réponses reconstruites d'après ce que lisent `python-qnapstats` et
    // l'intégration Home Assistant `qnap` (noms des balises), pas capturées
    // sur un NAS : un NAS de quatre baies dont un volume presque plein, un
    // disque en avertissement, un disque anormal, un emplacement vide, une
    // mise à jour de QTS proposée.
    const AUTH_LOGIN: &str = include_str!("testdata/auth_login.xml");
    const AUTH_FAILED: &str = include_str!("testdata/auth_failed.xml");
    const AUTH_2SV: &str = include_str!("testdata/auth_2sv.xml");
    const SYSINFO_XML: &str = include_str!("testdata/sysinfo.xml");
    const VOLUMES_XML: &str = include_str!("testdata/volumes.xml");
    const DISKS_XML: &str = include_str!("testdata/disks.xml");
    const FIRMWARE_UPDATE: &str = include_str!("testdata/firmware_update.xml");
    const FIRMWARE_NONE: &str = include_str!("testdata/firmware_none.xml");

    const SID: &str = "q2x9k1abcd";

    fn replies() -> Replies {
        Replies {
            login: parse_login(AUTH_LOGIN).unwrap(),
            sysinfo: SYSINFO_XML.to_string(),
            volumes: Some(VOLUMES_XML.to_string()),
            disks: Some(DISKS_XML.to_string()),
            firmware: Some(FIRMWARE_UPDATE.to_string()),
        }
    }

    #[test]
    fn la_connexion() {
        let login = parse_login(AUTH_LOGIN).unwrap();
        assert_eq!(login.sid, SID);
        assert_eq!((login.model.as_str(), login.version.as_str()), ("TS-453D", "5.1.7"));
        assert!(matches!(parse_login(AUTH_FAILED), Err(ProbeError::Auth(_))));
        assert!(matches!(parse_login(AUTH_2SV), Err(ProbeError::Config(_))));
        assert!(matches!(parse_login("<html>login</html>"), Err(ProbeError::Protocol(_))));
        assert!(matches!(parse_login("<other/>"), Err(ProbeError::Protocol(_))));
    }

    #[test]
    fn systeme_volumes_disques_micrologiciel() {
        let s = samples(&replies(), 0);
        let info = find_sample(&s, "qnap_version_info", &[]).unwrap();
        assert_eq!(
            (info.labels["version"].as_str(), info.labels["model"].as_str()),
            ("5.1.7", "TS-453D")
        );
        assert_eq!(value(&s, "qnap_cpu_usage_percent", &[]), 7.8);
        assert_eq!(value(&s, "qnap_cpu_temperature_celsius", &[]), 52.0);
        assert_eq!(value(&s, "qnap_system_temperature_celsius", &[]), 41.0);
        assert_eq!(value(&s, "qnap_memory_used_percent", &[]), 75.0);
        assert_eq!(value(&s, "qnap_memory_total_bytes", &[]), 8000.0 * MIB);
        assert_eq!(value(&s, "qnap_uptime_seconds", &[]), 1_053_065.0);

        assert_eq!(value(&s, "qnap_volumes", &[]), 2.0);
        assert!(
            (value(&s, "qnap_volume_used_percent", &[("volume", "DataVol1")]) - 90.0).abs() < 1e-6
        );
        assert!(
            (value(&s, "qnap_volume_used_percent", &[("volume", "Backups")]) - 25.0).abs() < 1e-6
        );

        assert_eq!(value(&s, "qnap_disks", &[]), 3.0, "the empty slot is left out");
        assert_eq!(value(&s, "qnap_disks_unhealthy", &[]), 2.0);
        assert_eq!(value(&s, "qnap_disk_ok", &[("disk", "disk 1")]), 1.0);
        assert_eq!(value(&s, "qnap_disk_warning", &[("disk", "disk 2")]), 1.0);
        assert_eq!(value(&s, "qnap_disk_ok", &[("disk", "disk 3")]), 0.0);
        assert_eq!(value(&s, "qnap_disk_temperature_celsius", &[("disk", "disk 1")]), 36.0);

        assert_eq!(value(&s, "qnap_firmware_update_available", &[]), 1.0);
        assert_eq!(
            find_sample(&s, "qnap_firmware_latest_info", &[]).unwrap().labels["version"],
            "5.2.0.2802"
        );
    }

    #[test]
    fn sans_lectures_facultatives() {
        let mut r = replies();
        r.volumes = None;
        r.disks = None;
        r.firmware = Some(FIRMWARE_NONE.to_string());
        let s = samples(&r, 0);
        assert!(find_sample(&s, "qnap_volumes", &[]).is_none());
        assert!(find_sample(&s, "qnap_disks", &[]).is_none());
        assert_eq!(value(&s, "qnap_firmware_update_available", &[]), 0.0);
        assert!(find_sample(&s, "qnap_firmware_latest_info", &[]).is_none());
        assert_eq!(value(&s, "qnap_cpu_usage_percent", &[]), 7.8);
    }

    #[test]
    fn etats_smart() {
        assert_eq!(health("OK"), Health::Good);
        assert_eq!(health(" Warning "), Health::Warning);
        assert_eq!(health("Abnormal"), Health::Failed);
    }

    #[test]
    fn l_identifiant_de_session_est_retire_des_erreurs() {
        let error = ProbeError::Unreachable(format!("/x?sid={SID}: connection refused"));
        let message = scrub(error, SID).to_string();
        assert!(!message.contains(SID) && message.contains("***"), "{message}");
    }

    fn fake(logins: Arc<AtomicUsize>, logouts: Arc<AtomicUsize>, broken: bool) -> Router {
        Router::new()
            .route(
                LOGIN,
                post(move |Form(form): Form<HashMap<String, String>>| {
                    let logins = logins.clone();
                    async move {
                        logins.fetch_add(1, Ordering::SeqCst);
                        // « s3cret » en base64.
                        let good = form.get("user").map(String::as_str) == Some("dumbmonit")
                            && form.get("pwd").map(String::as_str) == Some("czNjcmV0");
                        if good { AUTH_LOGIN } else { AUTH_FAILED }
                    }
                }),
            )
            .fallback(get(move |uri: Uri| {
                let logouts = logouts.clone();
                async move {
                    let query = uri.query().unwrap_or_default();
                    if !query.contains(&format!("sid={SID}")) {
                        return (StatusCode::OK, AUTH_FAILED);
                    }
                    match uri.path() {
                        "/cgi-bin/authLogout.cgi" => {
                            logouts.fetch_add(1, Ordering::SeqCst);
                            (StatusCode::OK, "<QDocRoot version=\"1.0\"></QDocRoot>")
                        }
                        "/cgi-bin/management/manaRequest.cgi" if broken => {
                            (StatusCode::INTERNAL_SERVER_ERROR, "")
                        }
                        "/cgi-bin/management/manaRequest.cgi" => (StatusCode::OK, SYSINFO_XML),
                        "/cgi-bin/management/chartReq.cgi" => (StatusCode::OK, VOLUMES_XML),
                        // Compte sans droit sur les disques, route absente pour le reste.
                        "/cgi-bin/disk/disk_manage.cgi" => (StatusCode::FORBIDDEN, ""),
                        _ => (StatusCode::NOT_FOUND, ""),
                    }
                }
            }))
    }

    fn target(address: &str, password: &str) -> Target {
        let mut target = cible("qnap", &format!("http://{address}"), &[]);
        target.credential = Credential::UsernamePassword {
            username: "dumbmonit".into(),
            password: password.into(),
        };
        target
    }

    #[tokio::test]
    async fn interrogation_complete_et_deconnexion() {
        let (logins, logouts) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let address = serve(fake(logins.clone(), logouts.clone(), false)).await;
        let s = QnapCollector::new().probe(&target(&address, "s3cret")).await.unwrap();
        assert_eq!(value(&s, "qnap_volumes", &[]), 2.0);
        assert!(find_sample(&s, "qnap_disks", &[]).is_none(), "refused optional read");
        assert_eq!(logins.load(Ordering::SeqCst), 1);
        assert_eq!(logouts.load(Ordering::SeqCst), 1, "the session is closed");
    }

    #[tokio::test]
    async fn mauvais_mot_de_passe_et_panne_en_cours_de_route() {
        let (logins, logouts) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let address = serve(fake(logins.clone(), logouts.clone(), true)).await;

        let error = QnapCollector::new().probe(&target(&address, "wrong")).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
        assert!(!error.means_down());

        // Une erreur serveur sur la lecture obligatoire : panne, sans l'identifiant.
        let error = QnapCollector::new().probe(&target(&address, "s3cret")).await.unwrap_err();
        assert!(error.means_down(), "{error}");
        assert!(!error.to_string().contains(SID), "{error}");
        assert_eq!(logouts.load(Ordering::SeqCst), 1, "closed even after a failed read");
    }
}
