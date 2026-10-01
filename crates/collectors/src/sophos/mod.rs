//! Sophos Firewall (SFOS), par son API XML (`/webconsole/APIController`,
//! port 4444).
//!
//! Volontairement minimal : l'API XML est une API de configuration. Elle rend
//! l'état réel du lien de chaque interface (`<Status>`, en lecture seule), et
//! c'est la seule donnée vivante qu'elle expose sûrement. L'état des tunnels
//! IPsec, de la grappe HA, des licences et la version du micrologiciel n'y
//! sont pas : `<Get><IPSecConnection>` ne rend que l'activation décidée par
//! l'administrateur, publiée comme telle et jamais présentée comme l'état du
//! tunnel. Pour le reste, SNMP.
//!
//! Chaque requête porte l'identifiant d'un administrateur dont le profil
//! d'accès est en lecture seule partout ; elle part en POST (champ `reqxml`),
//! jamais dans l'URL, où le mot de passe finirait dans les journaux.
//!
//! La panne que ce module existe pour voir : **une interface active, rattachée
//! à une zone, qui n'a plus de lien**.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `port` | `4444` | Port de la console d'administration. |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable (celui d'origine est auto-signé). |
//! | `request_timeout_seconds` | `15` | Délai par requête HTTP. |

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};

use crate::rest::{MAX_NAMED, RestClient, flag, gauge, now_ms};

pub const DEFAULT_PORT: u16 = 4444;
const PATH: &str = "/webconsole/APIController";

#[derive(Default)]
pub struct SophosCollector;

impl SophosCollector {
    pub fn new() -> Self {
        Self
    }
}

/// Échappe un texte pour un élément XML.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Le document `reqxml` d'une lecture.
pub fn request(username: &str, password: &str, entity: &str) -> String {
    format!(
        "<Request><Login><Username>{}</Username><Password>{}</Password></Login>\
         <Get><{entity}></{entity}></Get></Request>",
        escape(username),
        escape(password)
    )
}

struct Api {
    client: RestClient,
    username: String,
    password: String,
}

impl Api {
    fn new(target: &Target) -> Result<Self, ProbeError> {
        let (username, password) = match &target.credential {
            Credential::UsernamePassword { username, password } if !username.trim().is_empty() => {
                (username.trim().to_string(), password.clone())
            }
            other => {
                return Err(ProbeError::Config(format!(
                    "Sophos Firewall expects a user name and password, configured: {other}"
                )));
            }
        };
        let client = RestClient::for_target(target, "https", DEFAULT_PORT, "Sophos Firewall")?;
        Ok(Self { client, username, password })
    }

    async fn get(&self, entity: &str) -> Result<String, ProbeError> {
        let xml = request(&self.username, &self.password, entity);
        let (status, body) = self.client.post_form(PATH, &[("reqxml", xml.as_str())]).await?;
        if !status.is_success() {
            return Err(self.client.status_error(status, &body, PATH));
        }
        check(&body)?;
        Ok(body)
    }
}

/// Vérifie l'enveloppe : connexion acceptée, adresse autorisée.
pub fn check(body: &str) -> Result<(), ProbeError> {
    let document = roxmltree::Document::parse(body).map_err(|error| {
        ProbeError::Protocol(format!(
            "Sophos Firewall did not answer in XML ({error}): check the port (4444) and that the \
             API is enabled."
        ))
    })?;
    let root = document.root_element();
    if !root.has_tag_name("Response") {
        return Err(ProbeError::Protocol(
            "The answer is not a Sophos Firewall API response".to_string(),
        ));
    }
    // Statut global : adresse refusée (534), API désactivée, compte refusé (529)…
    if let Some(status) = root.children().find(|n| n.has_tag_name("Status")) {
        let code = status.attribute("code").unwrap_or_default();
        let message = status.text().unwrap_or_default().trim();
        return Err(match code {
            "534" => ProbeError::Auth(format!(
                "Sophos Firewall refused the request (534: {message}): add the DumbMonit address \
                 to the allowed IP addresses of the API configuration."
            )),
            "529" | "530" | "531" => ProbeError::Auth(format!(
                "Sophos Firewall refused the account ({code}: {message})."
            )),
            _ => ProbeError::Protocol(format!("Sophos Firewall answered {code}: {message}")),
        });
    }
    let login = root
        .descendants()
        .find(|n| n.has_tag_name("Login"))
        .and_then(|login| {
            login.children().find(|n| n.tag_name().name().eq_ignore_ascii_case("status"))
        })
        .and_then(|n| n.text())
        .unwrap_or_default();
    if !login.to_ascii_lowercase().contains("successful") {
        return Err(ProbeError::Auth(format!(
            "Sophos Firewall refused the login ({}): check the user name and password.",
            if login.is_empty() { "no login status" } else { login.trim() }
        )));
    }
    Ok(())
}

#[async_trait]
impl Collector for SophosCollector {
    fn kind(&self) -> &'static str {
        "sophos"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let api = Api::new(target)?;
        let (interfaces, ipsec) = tokio::join!(api.get("Interface"), api.get("IPSecConnection"));
        let interfaces = interfaces?;
        let ipsec = match ipsec {
            Ok(body) => Some(body),
            Err(error) if error.means_down() => return Err(error),
            Err(_) => None,
        };
        Ok(samples(&interfaces, ipsec.as_deref(), now_ms()))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        Api::new(target)?.get("Interface").await?;
        Ok(Some("sophos".to_string()))
    }
}

fn child_text<'a>(node: roxmltree::Node<'a, '_>, name: &str) -> Option<&'a str> {
    node.children()
        .find(|n| n.has_tag_name(name))
        .and_then(|n| n.text())
        .map(str::trim)
        .filter(|t| !t.is_empty())
}

/// Le lien d'après `<Status>` : `Connected, 1000 Mbps…` ou `Disconnected`.
fn link(status: &str) -> Option<bool> {
    let status = status.trim().to_ascii_lowercase();
    if status.starts_with("disconnected")
        || status.starts_with("down")
        || status.contains("unplugged")
    {
        Some(false)
    } else if status.starts_with("connected") || status.starts_with("up") {
        Some(true)
    } else {
        None
    }
}

pub fn samples(interfaces: &str, ipsec: Option<&str>, ts_ms: i64) -> Vec<Sample> {
    let g = |name: &str, value: f64| gauge(name, value, ts_ms);
    let mut out = Vec::new();
    let Ok(document) = roxmltree::Document::parse(interfaces) else { return out };
    if let Some(version) = document.root_element().attribute("APIVersion") {
        out.push(g("sophos_api_version_info", 1.0).with_label("api_version", version));
    }
    let (mut count, mut down) = (0, 0);
    for interface in document.root_element().children().filter(|n| n.has_tag_name("Interface")) {
        let Some(name) = child_text(interface, "Name") else { continue };
        count += 1;
        let zone = child_text(interface, "NetworkZone").unwrap_or("None");
        let enabled = child_text(interface, "InterfaceStatus") != Some("OFF");
        let zoned = !zone.eq_ignore_ascii_case("none");
        let link = child_text(interface, "Status").and_then(link);
        // Un port libre (sans zone) ou désactivé n'a pas à avoir de lien.
        let is_down = enabled && zoned && link == Some(false);
        down += usize::from(is_down);
        if count > MAX_NAMED {
            continue;
        }
        let named = |sample: Sample| sample.with_label("interface", name).with_label("zone", zone);
        if let Some(link) = link {
            out.push(named(flag("sophos_interface_link_up", link, ts_ms)));
        }
        out.push(named(flag("sophos_interface_enabled", enabled, ts_ms)));
        out.push(named(flag("sophos_interface_down", is_down, ts_ms)));
    }
    out.push(g("sophos_interfaces", count as f64));
    out.push(g("sophos_interfaces_down", down as f64));

    if let Some(Ok(document)) = ipsec.map(roxmltree::Document::parse) {
        let mut connections = 0;
        for connection in document.descendants().filter(|n| n.has_tag_name("IPSecConnection")) {
            let Some(configuration) =
                connection.children().find(|n| n.has_tag_name("Configuration"))
            else {
                continue;
            };
            let Some(name) = child_text(configuration, "Name") else { continue };
            connections += 1;
            if connections > MAX_NAMED {
                continue;
            }
            // L'activation décidée par l'administrateur, pas l'état du tunnel.
            let activated = child_text(configuration, "Status") == Some("Active");
            out.push(
                flag("sophos_ipsec_connection_activated", activated, ts_ms)
                    .with_label("connection", name),
            );
        }
        out.push(g("sophos_ipsec_connections", connections as f64));
    }
    out
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use axum::Router;
    use axum::extract::Form;
    use axum::routing::post;

    use super::*;
    use crate::rest::test_support::{find, serve, value};
    use crate::uptime::tags::test_support::cible;

    // Réponses construites d'après la documentation de l'API XML de SFOS
    // (balises `Interface` et `IPSecConnection`, statut 534) : deux ports
    // reliés, un port de DMZ sans lien, un port libre, un port désactivé ;
    // une connexion IPsec activée, une désactivée.
    const INTERFACES: &str = include_str!("testdata/interface.xml");
    const IPSEC: &str = include_str!("testdata/ipsec.xml");
    const LOGIN_FAILURE: &str = include_str!("testdata/login_failure.xml");
    const IP_NOT_ALLOWED: &str = include_str!("testdata/ip_not_allowed.xml");

    #[test]
    fn interfaces_et_connexions() {
        let s = samples(INTERFACES, Some(IPSEC), 0);
        assert_eq!(
            find(&s, "sophos_api_version_info", &[]).unwrap().labels["api_version"],
            "2000.2"
        );
        assert_eq!(value(&s, "sophos_interfaces", &[]), 5.0);
        assert_eq!(
            value(&s, "sophos_interface_link_up", &[("interface", "Port1"), ("zone", "LAN")]),
            1.0
        );
        assert_eq!(value(&s, "sophos_interface_down", &[("interface", "Port3")]), 1.0);
        assert_eq!(value(&s, "sophos_interface_down", &[("interface", "Port4")]), 0.0, "no zone");
        assert_eq!(
            value(&s, "sophos_interface_down", &[("interface", "Port5")]),
            0.0,
            "switched off"
        );
        assert_eq!(value(&s, "sophos_interfaces_down", &[]), 1.0);
        assert_eq!(value(&s, "sophos_ipsec_connections", &[]), 2.0);
        assert_eq!(
            value(&s, "sophos_ipsec_connection_activated", &[("connection", "old-branch")]),
            0.0
        );
    }

    #[test]
    fn enveloppes_refusees() {
        assert!(check(INTERFACES).is_ok());
        assert!(matches!(check(LOGIN_FAILURE), Err(ProbeError::Auth(_))));
        let error = check(IP_NOT_ALLOWED).unwrap_err();
        assert!(matches!(error, ProbeError::Auth(ref m) if m.contains("allowed IP")), "{error}");
        assert!(matches!(check("<html>login</html>"), Err(ProbeError::Protocol(_))));
        assert!(matches!(check("pas du xml"), Err(ProbeError::Protocol(_))));
    }

    #[test]
    fn le_mot_de_passe_est_echappe() {
        let xml = request("dumbmonit", "a<b&c", "Interface");
        assert!(xml.contains("<Password>a&lt;b&amp;c</Password>"));
        assert!(xml.contains("<Get><Interface></Interface></Get>"));
    }

    #[tokio::test]
    async fn interrogation_complete() {
        let app = Router::new().route(
            PATH,
            post(|Form(form): Form<HashMap<String, String>>| async move {
                let xml = form.get("reqxml").cloned().unwrap_or_default();
                if !xml.contains("<Password>s3cret</Password>") {
                    return LOGIN_FAILURE;
                }
                if xml.contains("<IPSecConnection>") { IPSEC } else { INTERFACES }
            }),
        );
        let address = serve(app).await;
        let mut target = cible("sophos", &format!("http://{address}"), &[]);
        target.credential = Credential::UsernamePassword {
            username: "dumbmonit".into(),
            password: "s3cret".into(),
        };
        let s = SophosCollector::new().probe(&target).await.unwrap();
        assert_eq!(value(&s, "sophos_interfaces_down", &[]), 1.0);
        assert_eq!(value(&s, "sophos_ipsec_connections", &[]), 2.0);

        target.credential =
            Credential::UsernamePassword { username: "dumbmonit".into(), password: "nope".into() };
        let error = SophosCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
    }
}
