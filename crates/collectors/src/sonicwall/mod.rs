//! SonicWall (SonicOS 7), par l'API SonicOS (`/api/sonicos/…`).
//!
//! BÊTA : construit d'après ce que la documentation et les réponses de
//! SonicWall publient de l'API, sans appareil réel pour l'essayer, et
//! volontairement minimal. Seuls les chemins confirmés sont lus :
//! `POST /api/sonicos/auth` (ouverture de session), `GET /api/sonicos/version`
//! et `GET /api/sonicos/reporting/interfaces/ipv4/status` (état du lien de
//! chaque interface), puis `DELETE /api/sonicos/auth` (fermeture). L'état des
//! tunnels, de la grappe HA et des licences n'est pas lu : aucun chemin n'en
//! est publié avec certitude.
//!
//! Chaque requête porte l'identifiant d'un administrateur en `Authorization:
//! Basic` (jamais dans l'URL). SonicOS n'admet qu'une session d'API à la fois :
//! la session est donc toujours refermée, même quand une lecture a échoué.
//! Les réponses sont lues avec tolérance — la forme exacte de l'état des
//! interfaces varie selon les versions — : tout objet qui porte un `name` et
//! une chaîne d'état (`status` ou `link_status`) est une interface.
//!
//! La panne que ce module existe pour voir : **une interface rattachée à une
//! zone qui n'a plus de lien**.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `port` | `443` | Port de l'interface d'administration (HTTPS seulement). |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable (celui d'origine est auto-signé). |
//! | `request_timeout_seconds` | `15` | Délai par requête HTTP. |

use async_trait::async_trait;
use base64::Engine;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};
use serde_json::Value;

use crate::rest::{MAX_NAMED, RestClient, flag, gauge, now_ms};

pub const DEFAULT_PORT: u16 = 443;

const AUTH: &str = "/api/sonicos/auth";
const VERSION: &str = "/api/sonicos/version";
const INTERFACES: &str = "/api/sonicos/reporting/interfaces/ipv4/status";

#[derive(Default)]
pub struct SonicwallCollector;

impl SonicwallCollector {
    pub fn new() -> Self {
        Self
    }
}

struct Api {
    client: RestClient,
}

impl Api {
    fn new(target: &Target) -> Result<Self, ProbeError> {
        let (username, password) = match &target.credential {
            Credential::UsernamePassword { username, password } if !username.trim().is_empty() => {
                (username.trim(), password.as_str())
            }
            other => {
                return Err(ProbeError::Config(format!(
                    "SonicWall expects a user name and password, configured: {other}"
                )));
            }
        };
        let basic =
            base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"));
        let client = RestClient::for_target(target, "https", DEFAULT_PORT, "SonicWall")?
            .with_header("authorization", format!("Basic {basic}"));
        Ok(Self { client })
    }

    /// Ouvre la session : refuse un compte inconnu ou une API éteinte.
    async fn login(&self) -> Result<(), ProbeError> {
        let (status, body) = self.client.post_form(AUTH, &[]).await?;
        self.interpret(status, &body, AUTH).map(|_| ())
    }

    /// Referme la session ; un échec ici ne change rien au résultat.
    async fn logout(&self) {
        if let Err(error) = self.client.delete(AUTH).await {
            tracing::debug!(%error, "SonicWall : fermeture de session impossible");
        }
    }

    fn interpret(
        &self,
        status: reqwest::StatusCode,
        body: &str,
        path: &str,
    ) -> Result<Value, ProbeError> {
        match status.as_u16() {
            401 => Err(ProbeError::Auth(
                "SonicWall refused the account (401): check the user name and password, and that \
                 HTTP Basic is allowed under Device > Settings > Administration > Audit/SonicOS \
                 API."
                    .to_string(),
            )),
            403 => Err(ProbeError::Auth(format!(
                "SonicWall refused {path} (403): the SonicOS API is disabled, or the account \
                 cannot use it."
            ))),
            200..=299 => {
                let reply: Value = if body.trim().is_empty() {
                    Value::Null
                } else {
                    self.client.decode(body, path)?
                };
                envelope(&reply)?;
                Ok(reply)
            }
            _ => Err(self.client.status_error(status, body, path)),
        }
    }

    async fn get(&self, path: &str) -> Result<Value, ProbeError> {
        let (status, body) = self.client.get(path).await?;
        self.interpret(status, &body, path)
    }
}

/// Un échec annoncé dans l'enveloppe `{"status": {"success": false, "info": […]}}`.
pub fn envelope(reply: &Value) -> Result<(), ProbeError> {
    let Some(status) = reply.get("status").filter(|s| s.is_object()) else { return Ok(()) };
    if status.get("success").and_then(Value::as_bool) != Some(false) {
        return Ok(());
    }
    let info = status.pointer("/info/0");
    let code = info.and_then(|i| i.get("code")).and_then(Value::as_str).unwrap_or("unknown");
    let message = info.and_then(|i| i.get("message")).and_then(Value::as_str).unwrap_or("");
    let upper = code.to_ascii_uppercase();
    Err(
        if upper.contains("UNAUTHORIZED") || upper.contains("AUTH") || upper.contains("FORBIDDEN") {
            ProbeError::Auth(format!("SonicWall refused the request ({code}): {message}"))
        } else {
            ProbeError::Protocol(format!("SonicWall answered an error ({code}): {message}"))
        },
    )
}

#[async_trait]
impl Collector for SonicwallCollector {
    fn kind(&self) -> &'static str {
        "sonicwall"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let api = Api::new(target)?;
        api.login().await?;
        let result = Self::read(&api).await;
        api.logout().await;
        let (version, interfaces) = result?;
        Ok(samples(version.as_ref(), &interfaces, now_ms()))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let api = Api::new(target)?;
        let result = api.login().await;
        api.logout().await;
        result?;
        Ok(Some("sonicwall".to_string()))
    }
}

impl SonicwallCollector {
    /// Les lectures, session ouverte : l'état des interfaces est obligatoire,
    /// la version facultative.
    async fn read(api: &Api) -> Result<(Option<Value>, Value), ProbeError> {
        let interfaces = api.get(INTERFACES).await?;
        let version = match api.get(VERSION).await {
            Ok(reply) => Some(reply),
            Err(error) if error.means_down() => return Err(error),
            Err(error) => {
                tracing::debug!(%error, "SonicWall : version indisponible");
                None
            }
        };
        Ok((version, interfaces))
    }
}

/// La première chaîne non vide portée par l'une de ces clés, où que ce soit.
fn find_string<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a str> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(text) =
                    map.get(*key).and_then(Value::as_str).map(str::trim).filter(|t| !t.is_empty())
                {
                    return Some(text);
                }
            }
            map.values().find_map(|inner| find_string(inner, keys))
        }
        Value::Array(items) => items.iter().find_map(|inner| find_string(inner, keys)),
        _ => None,
    }
}

/// Les objets qui portent un nom et une chaîne d'état : les interfaces.
fn collect_interfaces<'a>(value: &'a Value, out: &mut Vec<(&'a str, &'a str, &'a str)>) {
    match value {
        Value::Object(map) => {
            let name = map.get("name").and_then(Value::as_str).map(str::trim);
            let state = ["link_status", "status"]
                .iter()
                .find_map(|key| map.get(*key).and_then(Value::as_str));
            if let (Some(name), Some(state)) = (name.filter(|n| !n.is_empty()), state) {
                let zone = map.get("zone").and_then(Value::as_str).map(str::trim).unwrap_or("");
                out.push((name, zone, state));
            } else {
                for inner in map.values() {
                    collect_interfaces(inner, out);
                }
            }
        }
        Value::Array(items) => items.iter().for_each(|inner| collect_interfaces(inner, out)),
        _ => {}
    }
}

/// Le lien d'après la chaîne d'état : `1 Gbps Full Duplex`, `No link`, `Link Down`.
fn link(state: &str) -> Option<bool> {
    let state = state.trim().to_ascii_lowercase();
    if state.contains("no link") || state.contains("down") || state.contains("disconnect") {
        Some(false)
    } else if state.contains("duplex")
        || state.contains("bps")
        || state.contains("link up")
        || state == "up"
    {
        Some(true)
    } else {
        None
    }
}

pub fn samples(version: Option<&Value>, interfaces: &Value, ts_ms: i64) -> Vec<Sample> {
    let g = |name: &str, value: f64| gauge(name, value, ts_ms);
    let mut out = Vec::new();
    if let Some(version) = version
        && let Some(firmware) = find_string(version, &["firmware_version", "firmware", "version"])
    {
        out.push(
            g("sonicwall_version_info", 1.0)
                .with_label("version", firmware)
                .with_label("model", find_string(version, &["model"]).unwrap_or("")),
        );
    }
    let mut found = Vec::new();
    collect_interfaces(interfaces, &mut found);
    let (mut count, mut down) = (0, 0);
    for (name, zone, state) in found {
        count += 1;
        let link = link(state);
        // Un port libre (sans zone, ou « Unassigned ») n'a pas à avoir de lien.
        let zoned = !zone.is_empty() && !zone.eq_ignore_ascii_case("unassigned");
        let is_down = zoned && link == Some(false);
        down += usize::from(is_down);
        if count > MAX_NAMED {
            continue;
        }
        let named = |sample: Sample| sample.with_label("interface", name).with_label("zone", zone);
        if let Some(link) = link {
            out.push(named(flag("sonicwall_interface_link_up", link, ts_ms)));
        }
        out.push(named(flag("sonicwall_interface_down", is_down, ts_ms)));
    }
    out.push(g("sonicwall_interfaces", count as f64));
    out.push(g("sonicwall_interfaces_down", down as f64));
    out
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::http::{HeaderMap, Method, StatusCode, Uri};
    use axum::routing::any;

    use super::*;
    use crate::rest::test_support::{find, serve, value};
    use crate::uptime::tags::test_support::cible;

    // Réponses reconstruites d'après ce que SonicWall publie de l'API SonicOS 7
    // (enveloppe `status.success`, état des interfaces en chaîne) : la forme
    // exacte des champs n'est pas documentée, elle est lue avec tolérance. Deux
    // ports reliés, un port de WAN sans lien, un port libre, un non attribué.
    const VERSION_JSON: &str = include_str!("testdata/version.json");
    const INTERFACES_JSON: &str = include_str!("testdata/interfaces_status.json");
    const AUTH_OK: &str = include_str!("testdata/auth_ok.json");
    const AUTH_FAILED: &str = include_str!("testdata/auth_failed.json");

    fn json(raw: &str) -> Value {
        serde_json::from_str(raw).unwrap()
    }

    #[test]
    fn interfaces_et_version() {
        let s = samples(Some(&json(VERSION_JSON)), &json(INTERFACES_JSON), 0);
        let info = find(&s, "sonicwall_version_info", &[]).unwrap();
        assert_eq!(
            (info.labels["version"].as_str(), info.labels["model"].as_str()),
            ("SonicOS Enhanced 7.0.1-5111", "NSa 2700")
        );
        assert_eq!(value(&s, "sonicwall_interfaces", &[]), 5.0);
        assert_eq!(value(&s, "sonicwall_interface_link_up", &[("interface", "X0")]), 1.0);
        assert_eq!(value(&s, "sonicwall_interface_down", &[("interface", "X1")]), 1.0);
        assert_eq!(value(&s, "sonicwall_interface_down", &[("interface", "X2")]), 0.0);
        assert_eq!(value(&s, "sonicwall_interface_down", &[("interface", "X3")]), 0.0, "no zone");
        let down = |name: &str| value(&s, "sonicwall_interface_down", &[("interface", name)]);
        assert_eq!(down("X4"), 0.0, "unassigned");
        assert_eq!(value(&s, "sonicwall_interfaces_down", &[]), 1.0);
    }

    #[test]
    fn une_forme_inconnue_ne_rend_que_des_comptes() {
        let s = samples(None, &json(r#"{"autre":"chose"}"#), 0);
        assert_eq!(value(&s, "sonicwall_interfaces", &[]), 0.0);
        assert!(find(&s, "sonicwall_version_info", &[]).is_none());
    }

    #[test]
    fn enveloppes_refusees() {
        assert!(envelope(&json(AUTH_OK)).is_ok());
        assert!(envelope(&json(VERSION_JSON)).is_ok());
        assert!(matches!(envelope(&json(AUTH_FAILED)), Err(ProbeError::Auth(_))));
        let other =
            json(r#"{"status":{"success":false,"info":[{"code":"E_INVALID","message":"x"}]}}"#);
        assert!(matches!(envelope(&other), Err(ProbeError::Protocol(_))));
    }

    #[tokio::test]
    async fn interrogation_complete_et_session_refermee() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let logouts = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&logouts);
        let app =
            Router::new().fallback(any(move |method: Method, headers: HeaderMap, uri: Uri| {
                let counter = Arc::clone(&counter);
                async move {
                    // « dumbmonit:s3cret » en Basic.
                    let expected = "Basic ZHVtYm1vbml0OnMzY3JldA==";
                    if headers.get("authorization").and_then(|v| v.to_str().ok()) != Some(expected)
                    {
                        return (StatusCode::UNAUTHORIZED, "");
                    }
                    match (method, uri.path()) {
                        (Method::POST, AUTH) => (StatusCode::OK, AUTH_OK),
                        (Method::DELETE, AUTH) => {
                            counter.fetch_add(1, Ordering::SeqCst);
                            (StatusCode::OK, AUTH_OK)
                        }
                        (Method::GET, INTERFACES) => (StatusCode::OK, INTERFACES_JSON),
                        // La version est facultative : un refus ne casse rien.
                        _ => (StatusCode::FORBIDDEN, ""),
                    }
                }
            }));
        let address = serve(app).await;
        let mut target = cible("sonicwall", &format!("http://{address}"), &[]);
        target.credential = Credential::UsernamePassword {
            username: "dumbmonit".into(),
            password: "s3cret".into(),
        };
        let s = SonicwallCollector::new().probe(&target).await.unwrap();
        assert_eq!(value(&s, "sonicwall_interfaces_down", &[]), 1.0);
        assert!(find(&s, "sonicwall_version_info", &[]).is_none());
        assert_eq!(logouts.load(Ordering::SeqCst), 1, "session closed");

        target.credential =
            Credential::UsernamePassword { username: "dumbmonit".into(), password: "nope".into() };
        let error = SonicwallCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
        assert!(!error.means_down());
    }
}
