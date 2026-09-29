//! MDaemon Email Server : les services de messagerie, et l'API XML quand un
//! compte est fourni.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `services` | `smtp,imap,webmail` | Services vérifiés, `nom` ou `nom:port`. |
//! | `request_timeout_seconds` | `10` | Délai par connexion et par appel. |
//! | `api_port` | `444` | Port de l'administration à distance qui sert l'API XML. |
//! | `api_tls` | `true` | HTTPS vers l'API XML (sinon HTTP, sur le port 1000 en général). |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable. |

use std::time::{Duration, Instant};

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, MetricKind, ProbeError, Sample, Target};
use tracing::{debug, warn};

use super::address::{self, Host};
use super::ports::{self, ServiceCheck, ServiceSpec};
use super::xmlapi::{self, XmlApiClient};

/// Port HTTPS de l'administration à distance, qui sert `/MdMgmtWS/`.
const DEFAULT_API_PORT: u16 = 444;

/// Chemin de l'API XML.
const API_PATH: &str = "/MdMgmtWS/";

#[derive(Debug, Clone)]
pub(super) struct Options {
    pub host: Host,
    pub services: Vec<ServiceSpec>,
    pub request_timeout: Duration,
    pub insecure_tls: bool,
    pub api_url: String,
}

impl Options {
    pub fn from_target(target: &Target) -> Result<Self, ProbeError> {
        let host = address::parse_host(&target.address)?;
        let api_port = match address::tag(target, "api_port") {
            Some(raw) => address::parse_port(raw)?,
            None => host.port.unwrap_or(DEFAULT_API_PORT),
        };
        let api_tls = address::parse_bool_or(address::tag(target, "api_tls"), true)?;
        let scheme = if api_tls { "https" } else { "http" };
        Ok(Self {
            api_url: format!("{scheme}://{}:{api_port}{API_PATH}", host.for_url()),
            services: ports::parse_services(
                address::tag(target, "services"),
                ports::MDAEMON_SERVICES,
                ports::MDAEMON_DEFAULT,
            )?,
            request_timeout: address::parse_timeout(address::tag(
                target,
                "request_timeout_seconds",
            ))?,
            insecure_tls: address::parse_bool_or(address::tag(target, "insecure_tls"), false)?,
            host,
        })
    }
}

#[derive(Default)]
pub struct MdaemonCollector;

impl MdaemonCollector {
    pub fn new() -> Self {
        Self
    }

    /// Le client de l'API XML, ou `None` sans identifiant : la surveillance des
    /// ports se suffit à elle-même.
    fn api_client(
        &self,
        target: &Target,
        options: &Options,
    ) -> Result<Option<XmlApiClient>, ProbeError> {
        match &target.credential {
            Credential::None => Ok(None),
            Credential::UsernamePassword { username, password } => {
                if !username.contains('@') {
                    return Err(ProbeError::Config(
                        "MDaemon expects the full email address of the account as user name, \
                         for example \"dumbmonit@example.com\""
                            .to_string(),
                    ));
                }
                Ok(Some(XmlApiClient {
                    http: crate::http::client(options.insecure_tls)?,
                    url: options.api_url.clone(),
                    username: username.clone(),
                    password: password.clone(),
                    timeout: options.request_timeout,
                }))
            }
            other => Err(ProbeError::Config(format!(
                "MDaemon expects no credential (services only) or the email address and \
                 password of the account used for the XML API. Configured credential: {other}"
            ))),
        }
    }
}

#[async_trait]
impl Collector for MdaemonCollector {
    fn kind(&self) -> &'static str {
        "mdaemon"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let options = Options::from_target(target)?;
        let client = self.api_client(target, &options)?;
        if options.services.is_empty() && client.is_none() {
            return Err(ProbeError::Config(
                "Nothing to watch: list at least one service, or enter the account used \
                 for the XML API"
                    .to_string(),
            ));
        }
        let started = Instant::now();
        let ts_ms = chrono::Utc::now().timestamp_millis();

        let api_call = async {
            match &client {
                Some(client) => Some(client.call(xmlapi::OPERATION).await),
                None => None,
            }
        };
        let (checks, api) = futures::join!(
            ports::check_all(&options.host.name, &options.services, options.request_timeout),
            api_call
        );

        let mut samples = service_samples("mdaemon", &checks, ts_ms);
        let mut version: Option<(String, &str)> = None;
        let mut build: Option<String> = None;
        let mut api_answered = false;
        match api {
            None => {}
            Some(Ok(call)) => {
                api_answered = true;
                samples.push(Sample::new("mdaemon_api_up", 1.0, MetricKind::Gauge, ts_ms));
                samples.push(Sample::new(
                    "mdaemon_api_response_seconds",
                    call.seconds,
                    MetricKind::Gauge,
                    ts_ms,
                ));
                let ok = call.envelope.succeeded();
                if !ok {
                    // Le compte est entré mais n'a pas le droit d'appeler
                    // l'opération : la version reste lisible dans l'enveloppe.
                    debug!(
                        target_id = target.id,
                        status = call.envelope.status_value.as_deref().unwrap_or("?"),
                        "opération de l'API XML refusée"
                    );
                }
                samples.push(Sample::new(
                    "mdaemon_api_operation_ok",
                    if ok { 1.0 } else { 0.0 },
                    MetricKind::Gauge,
                    ts_ms,
                ));
                version = call.envelope.product_version.map(|v| (v, "xml_api"));
                build = call.envelope.service_version;
            }
            Some(Err(error)) if error.means_down() => {
                warn!(target_id = target.id, %error, "API XML MDaemon injoignable");
                samples.push(Sample::new("mdaemon_api_up", 0.0, MetricKind::Gauge, ts_ms));
            }
            // Mot de passe refusé, adresse non autorisée, mauvais port : c'est la
            // configuration qu'il faut corriger, et c'est ce qui doit s'afficher.
            Some(Err(error)) => return Err(error),
        }

        if !api_answered && !checks.is_empty() && checks.iter().all(|check| !check.up) {
            return Err(nothing_answered(&checks));
        }
        if !api_answered && checks.is_empty() {
            return Err(ProbeError::Unreachable("the MDaemon XML API does not answer".into()));
        }

        if version.is_none() {
            version = banner_version(&checks, "MDaemon").map(|v| (v, "smtp_banner"));
        }
        if let Some((version, source)) = version {
            let mut info = Sample::new("mdaemon_info", 1.0, MetricKind::Gauge, ts_ms)
                .with_label("version", version)
                .with_label("source", source);
            if let Some(build) = build {
                info = info.with_label("build", build);
            }
            samples.push(info);
        }
        samples.push(Sample::new(
            "mdaemon_scrape_duration_seconds",
            started.elapsed().as_secs_f64(),
            MetricKind::Gauge,
            ts_ms,
        ));
        Ok(samples)
    }
}

/// Une série par service : `…_service_up` (1 ou 0) et le temps de réponse
/// quand il y en a un.
pub(super) fn service_samples(prefix: &str, checks: &[ServiceCheck], ts_ms: i64) -> Vec<Sample> {
    let mut samples = Vec::with_capacity(checks.len() * 2);
    for check in checks {
        let port = check.port.to_string();
        samples.push(
            Sample::new(
                format!("{prefix}_service_up"),
                if check.up { 1.0 } else { 0.0 },
                MetricKind::Gauge,
                ts_ms,
            )
            .with_label("service", check.name.clone())
            .with_label("port", port.clone()),
        );
        if let Some(seconds) = check.seconds {
            samples.push(
                Sample::new(
                    format!("{prefix}_service_response_seconds"),
                    seconds,
                    MetricKind::Gauge,
                    ts_ms,
                )
                .with_label("service", check.name.clone())
                .with_label("port", port),
            );
        }
    }
    samples
}

/// Aucun service n'a répondu : l'équipement est injoignable, et le message dit
/// pourquoi, service par service.
pub(super) fn nothing_answered(checks: &[ServiceCheck]) -> ProbeError {
    let reasons: Vec<String> = checks
        .iter()
        .map(|check| {
            format!(
                "{} ({}): {}",
                check.name,
                check.port,
                check.reason.as_deref().unwrap_or("down")
            )
        })
        .collect();
    ProbeError::Unreachable(format!("no mail service answered. {}", reasons.join("; ")))
}

/// Version annoncée par la première bannière SMTP qui en donne une.
pub(super) fn banner_version(checks: &[ServiceCheck], product: &str) -> Option<String> {
    checks
        .iter()
        .filter(|check| check.up)
        .filter_map(|check| check.banner.as_deref())
        .find_map(|banner| ports::banner_version(banner, product))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use super::*;

    fn cible(address: &str, tags: &[(&str, &str)], credential: Credential) -> Target {
        Target {
            id: 9,
            name: "mail".into(),
            address: address.into(),
            kind: "mdaemon".into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            tags: tags
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect::<BTreeMap<_, _>>(),
            credential,
        }
    }

    fn compte() -> Credential {
        Credential::UsernamePassword {
            username: "dumbmonit@example.com".into(),
            password: "SECRET-MOT-DE-PASSE".into(),
        }
    }

    #[test]
    fn le_collecteur_annonce_son_type() {
        assert_eq!(MdaemonCollector::new().kind(), "mdaemon");
    }

    #[test]
    fn les_defauts_visent_l_administration_a_distance_en_https() {
        let options = Options::from_target(&cible("mail.lan", &[], Credential::None)).unwrap();
        assert_eq!(options.api_url, "https://mail.lan:444/MdMgmtWS/");
        assert_eq!(options.services.len(), 3);
        assert_eq!(options.request_timeout, Duration::from_secs(10));
        assert!(!options.insecure_tls);

        let options = Options::from_target(&cible(
            "mail.lan",
            &[("api_port", "1000"), ("api_tls", "false")],
            Credential::None,
        ))
        .unwrap();
        assert_eq!(options.api_url, "http://mail.lan:1000/MdMgmtWS/");
        let options =
            Options::from_target(&cible("[fd00::25]:8444", &[], Credential::None)).unwrap();
        assert_eq!(options.api_url, "https://[fd00::25]:8444/MdMgmtWS/");
    }

    #[test]
    fn un_identifiant_inadapte_est_refuse_sans_divulguer_le_secret() {
        let collector = MdaemonCollector::new();
        let target = cible("mail.lan", &[], Credential::ApiToken { token: "SECRET-JETON".into() });
        let options = Options::from_target(&target).unwrap();
        let error = collector.api_client(&target, &options).err().unwrap();
        assert!(matches!(error, ProbeError::Config(_)));
        assert!(!error.to_string().contains("SECRET-JETON"));

        let target = cible(
            "mail.lan",
            &[],
            Credential::UsernamePassword {
                username: "dumbmonit".into(),
                password: "SECRET".into(),
            },
        );
        let error = collector.api_client(&target, &options).err().unwrap();
        assert!(error.to_string().contains("full email address"));
        assert!(!error.to_string().contains("SECRET"));
    }

    /// Un faux MDaemon : SMTP qui s'annonce, et une API XML qui exige l'en-tête
    /// Basic de `dumbmonit@example.com` et renvoie l'enveloppe documentée.
    async fn faux_mdaemon() -> (u16, u16) {
        const OK: &str = include_str!("testdata/xmlapi-status-ok.xml");
        let smtp = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let smtp_port = smtp.local_addr().unwrap().port();
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = smtp.accept().await {
                let _ = socket.write_all(b"220 mx.test ESMTP MDaemon 26.0.4; ready\r\n").await;
                let mut sink = [0u8; 64];
                let _ = socket.read(&mut sink).await;
            }
        });
        let api = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let api_port = api.local_addr().unwrap().port();
        let expected = Arc::new(
            format!(
                "authorization: basic {}",
                base64_encode("dumbmonit@example.com:SECRET-MOT-DE-PASSE")
            )
            .to_lowercase(),
        );
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = api.accept().await {
                let expected = expected.clone();
                tokio::spawn(async move {
                    let mut request = Vec::new();
                    let mut chunk = [0u8; 4096];
                    // En-têtes puis corps : on lit jusqu'à la fin du document.
                    while !String::from_utf8_lossy(&request).contains("</MDaemon>") {
                        match socket.read(&mut chunk).await {
                            Ok(0) | Err(_) => return,
                            Ok(n) => request.extend_from_slice(&chunk[..n]),
                        }
                    }
                    let text = String::from_utf8_lossy(&request).to_lowercase();
                    let reply = if !text.starts_with("post /mdmgmtws/ ") {
                        "HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
                            .to_string()
                    } else if !text.contains(expected.as_str()) {
                        "HTTP/1.1 401 Unauthorized\r\ncontent-length: 0\r\nconnection: close\r\n\r\n".to_string()
                    } else {
                        format!(
                            "HTTP/1.1 200 OK\r\ncontent-type: text/xml\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{OK}",
                            OK.len()
                        )
                    };
                    let _ = socket.write_all(reply.as_bytes()).await;
                });
            }
        });
        (smtp_port, api_port)
    }

    fn base64_encode(raw: &str) -> String {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(raw)
    }

    fn valeur(samples: &[Sample], metric: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == metric).map(|s| s.value)
    }

    #[tokio::test]
    async fn une_interrogation_complete_lit_les_services_et_l_enveloppe() {
        let (smtp, api) = faux_mdaemon().await;
        let services = format!("smtp:{smtp}");
        let api_port = api.to_string();
        let target = cible(
            "127.0.0.1",
            &[("services", &services), ("api_port", &api_port), ("api_tls", "false")],
            compte(),
        );
        let samples = MdaemonCollector::new().probe(&target).await.unwrap();
        assert_eq!(valeur(&samples, "mdaemon_service_up"), Some(1.0));
        assert_eq!(valeur(&samples, "mdaemon_api_up"), Some(1.0));
        assert_eq!(valeur(&samples, "mdaemon_api_operation_ok"), Some(1.0));
        let info = samples.iter().find(|s| s.metric == "mdaemon_info").unwrap();
        // L'API prime sur la bannière : c'est la version exacte du service.
        assert_eq!(info.labels["version"], "23.5.1");
        assert_eq!(info.labels["build"], "23.5.1.6");
        assert_eq!(info.labels["source"], "xml_api");
    }

    #[tokio::test]
    async fn sans_compte_la_version_vient_de_la_banniere() {
        let (smtp, _) = faux_mdaemon().await;
        let services = format!("smtp:{smtp}");
        let target = cible("127.0.0.1", &[("services", &services)], Credential::None);
        let samples = MdaemonCollector::new().probe(&target).await.unwrap();
        assert!(valeur(&samples, "mdaemon_api_up").is_none(), "pas de compte, pas d'API");
        let info = samples.iter().find(|s| s.metric == "mdaemon_info").unwrap();
        assert_eq!(info.labels["version"], "26.0.4");
        assert_eq!(info.labels["source"], "smtp_banner");
    }

    #[tokio::test]
    async fn un_mot_de_passe_refuse_est_une_erreur_d_authentification() {
        let (smtp, api) = faux_mdaemon().await;
        let services = format!("smtp:{smtp}");
        let api_port = api.to_string();
        let target = cible(
            "127.0.0.1",
            &[("services", &services), ("api_port", &api_port), ("api_tls", "false")],
            Credential::UsernamePassword {
                username: "dumbmonit@example.com".into(),
                password: "FAUX".into(),
            },
        );
        let error = MdaemonCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
        assert!(!error.means_down());
    }

    #[tokio::test]
    async fn aucun_service_ne_repond_l_equipement_est_injoignable() {
        let closed = {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            listener.local_addr().unwrap().port()
        };
        let services = format!("smtp:{closed}");
        let target = cible("127.0.0.1", &[("services", &services)], Credential::None);
        let error = MdaemonCollector::new().probe(&target).await.unwrap_err();
        assert!(error.means_down(), "{error}");
        assert!(error.to_string().contains("smtp"));
    }

    #[tokio::test]
    async fn rien_a_surveiller_est_une_erreur_de_configuration() {
        let target = cible("127.0.0.1", &[("services", "none")], Credential::None);
        let error = MdaemonCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)));
    }
}
