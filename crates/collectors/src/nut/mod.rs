//! Onduleurs derrière NUT (Network UPS Tools), par le protocole réseau de
//! `upsd` (TCP 3493).
//!
//! NUT est le dénominateur commun des onduleurs branchés en USB : le NAS ou le
//! Raspberry Pi qui porte le câble fait tourner `upsd`, et c'est lui qu'on
//! interroge. Un même serveur publie souvent plusieurs onduleurs ; tous sont
//! lus en une connexion, chaque série portant l'étiquette `ups`.
//!
//! # Ce qui est envoyé
//!
//! `VER`, `LIST UPS`, `LIST VAR <ups>`, `LOGOUT` — et `USERNAME` / `PASSWORD`
//! seulement si un identifiant est configuré. Jamais `LOGIN` : il inscrirait
//! DumbMonit comme secondaire, que le primaire attend avant de couper le
//! courant. Jamais `SET`, `INSTCMD` ni `FSD`.
//!
//! `upsd` n'exige aucun identifiant pour lire : l'accès se règle par la
//! directive `LISTEN` et le pare-feu. `STARTTLS` n'est pas pris en charge :
//! la plupart des serveurs ne l'ont pas configuré (`ERR
//! FEATURE-NOT-CONFIGURED`), et rien de ce qui est lu n'est secret.
//!
//! # Erreurs
//!
//! * connexion refusée ou sans réponse : l'équipement est injoignable ;
//! * `ACCESS-DENIED` : `ProbeError::Auth` ;
//! * un onduleur demandé que `upsd` ne connaît pas : `ProbeError::Config` ;
//! * `DATA-STALE`, `DRIVER-NOT-CONNECTED` : des mesures
//!   (`nut_ups_data_stale = 1`), pas une panne de la cible — `upsd` répond, c'est
//!   le câble, le pilote ou l'onduleur qui se tait.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `port` | `3493` | Port de `upsd`, si l'adresse n'en donne pas. |
//! | `ups` | tous | Onduleurs à surveiller, séparés par des virgules. |
//! | `request_timeout_seconds` | `10` | Délai par échange. |

pub mod metrics;
pub mod protocol;

use std::time::Duration;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, MetricKind, ProbeError, Sample, Target};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tracing::warn;

use protocol::Session;

pub const DEFAULT_PORT: u16 = 3493;
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Au-delà, les onduleurs suivants sont ignorés (et signalés dans le journal).
pub const MAX_UPS: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Options {
    host: String,
    port: u16,
    /// Vide : tous les onduleurs du serveur.
    ups: Vec<String>,
    timeout: Duration,
}

fn tag<'a>(target: &'a Target, key: &str) -> Option<&'a str> {
    target.tags.get(key).map(|v| v.trim()).filter(|v| !v.is_empty())
}

fn parse_port(raw: &str) -> Result<u16, ProbeError> {
    match raw.trim().parse::<u16>() {
        Ok(port) if port > 0 => Ok(port),
        _ => Err(ProbeError::Config(format!("Invalid port: \"{raw}\""))),
    }
}

impl Options {
    fn from_target(target: &Target) -> Result<Self, ProbeError> {
        let (host, port_in_address) = parse_address(&target.address)?;
        let port = match port_in_address {
            Some(port) => port,
            None => tag(target, "port").map(parse_port).transpose()?.unwrap_or(DEFAULT_PORT),
        };
        let ups = tag(target, "ups")
            .map(|raw| {
                raw.split([',', ' '])
                    .map(str::trim)
                    .filter(|n| !n.is_empty())
                    // `ups@hôte`, la forme de `upsc`, est acceptée.
                    .map(|n| n.split('@').next().unwrap_or(n).to_string())
                    .collect()
            })
            .unwrap_or_default();
        let timeout = match tag(target, "request_timeout_seconds") {
            None => DEFAULT_REQUEST_TIMEOUT,
            Some(raw) => {
                let seconds: u64 =
                    raw.parse().ok().filter(|s| (1..=60).contains(s)).ok_or_else(|| {
                        ProbeError::Config(format!(
                            "Invalid timeout \"{raw}\": between 1 and 60 seconds"
                        ))
                    })?;
                Duration::from_secs(seconds)
            }
        };
        Ok(Self { host, port, ups, timeout })
    }
}

/// `nas.lan`, `nas.lan:3493`, `fd00::5`, `[fd00::5]:3493`, et la forme
/// `ups@nas.lan` de `upsc`, dont seul l'hôte est gardé.
fn parse_address(address: &str) -> Result<(String, Option<u16>), ProbeError> {
    let mut rest = address.trim();
    if let Some(stripped) = rest.strip_prefix("nut://") {
        rest = stripped;
    }
    let rest = rest.trim_end_matches('/');
    let rest = rest.rsplit_once('@').map_or(rest, |(_, host)| host);
    if rest.is_empty() {
        return Err(ProbeError::Config("Device address is empty".to_string()));
    }
    if let Some(inner) = rest.strip_prefix('[') {
        let (name, after) = inner
            .split_once(']')
            .ok_or_else(|| ProbeError::Config(format!("Invalid address: \"{address}\"")))?;
        let port = after.strip_prefix(':').map(parse_port).transpose()?;
        return Ok((name.to_string(), port));
    }
    if rest.matches(':').count() > 1 {
        return Ok((rest.to_string(), None));
    }
    match rest.split_once(':') {
        Some((name, port)) if !name.is_empty() => Ok((name.to_string(), Some(parse_port(port)?))),
        Some(_) => Err(ProbeError::Config(format!("Invalid address: \"{address}\""))),
        None => Ok((rest.to_string(), None)),
    }
}

fn gauge(name: &str, value: f64, ts_ms: i64) -> Sample {
    Sample::new(name, value, MetricKind::Gauge, ts_ms)
}

/// L'échange complet, sur un flux déjà ouvert.
async fn read_all<S: AsyncRead + AsyncWrite + Unpin>(
    session: &mut Session<S>,
    options: &Options,
    credential: &Credential,
    target_id: i64,
) -> Result<Vec<Sample>, ProbeError> {
    let now = chrono::Utc::now();
    let ts_ms = now.timestamp_millis();
    match credential {
        Credential::None => {}
        Credential::UsernamePassword { username, password } if !username.trim().is_empty() => {
            session.authenticate(username.trim(), password).await?;
        }
        other => {
            return Err(ProbeError::Config(format!(
                "NUT expects no credential, or an upsd user name and password; configured: {other}"
            )));
        }
    }

    let mut out = Vec::new();
    if let Some(version) = session.version().await? {
        out.push(gauge("nut_server_info", 1.0, ts_ms).with_label("version", version));
    }

    let served = session.list_ups().await?.map_err(|e| e.into_probe_error("LIST UPS"))?;
    out.push(gauge("nut_server_ups", served.len() as f64, ts_ms));
    let wanted: Vec<protocol::UpsEntry> = if options.ups.is_empty() {
        if served.is_empty() {
            return Err(ProbeError::Config(
                "This upsd serves no UPS: check ups.conf and that its driver is started."
                    .to_string(),
            ));
        }
        served
    } else {
        let mut wanted = Vec::new();
        for name in &options.ups {
            match served.iter().find(|entry| entry.name.eq_ignore_ascii_case(name)) {
                Some(entry) => wanted.push(entry.clone()),
                None => {
                    let known: Vec<&str> = served.iter().map(|e| e.name.as_str()).collect();
                    return Err(ProbeError::Config(format!(
                        "This upsd does not serve a UPS named \"{name}\". It serves: {}.",
                        if known.is_empty() { "none".to_string() } else { known.join(", ") }
                    )));
                }
            }
        }
        wanted
    };
    if wanted.len() > MAX_UPS {
        warn!(target_id, count = wanted.len(), max = MAX_UPS, "trop d'onduleurs, liste tronquée");
    }

    for entry in wanted.iter().take(MAX_UPS) {
        match session.list_vars(&entry.name).await? {
            Ok(vars) => {
                out.extend(metrics::ups_samples(&entry.name, &entry.description, &vars, now))
            }
            Err(error) if error.code == "DATA-STALE" => {
                out.extend(metrics::stale_samples(&entry.name, &entry.description, true, ts_ms))
            }
            Err(error) if error.code == "DRIVER-NOT-CONNECTED" => {
                out.extend(metrics::stale_samples(&entry.name, &entry.description, false, ts_ms))
            }
            Err(error) => {
                return Err(error.into_probe_error(&format!("LIST VAR {}", entry.name)));
            }
        }
    }
    session.logout().await;
    Ok(out)
}

async fn connect(options: &Options) -> Result<TcpStream, ProbeError> {
    let address = (options.host.as_str(), options.port);
    match tokio::time::timeout(options.timeout, TcpStream::connect(address)).await {
        Ok(Ok(stream)) => Ok(stream),
        Ok(Err(error)) => Err(ProbeError::Unreachable(format!(
            "upsd at {}:{}: {error}",
            options.host, options.port
        ))),
        Err(_) => Err(ProbeError::Timeout(options.timeout)),
    }
}

#[derive(Default)]
pub struct NutCollector;

impl NutCollector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Collector for NutCollector {
    fn kind(&self) -> &'static str {
        "nut"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let options = Options::from_target(target)?;
        let stream = connect(&options).await?;
        let mut session = Session::new(stream, options.timeout);
        read_all(&mut session, &options, &target.credential, target.id).await
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        self.probe(target).await?;
        Ok(Some("nut".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpListener;

    use super::*;

    fn target(address: &str, tags: &[(&str, &str)], credential: Credential) -> Target {
        Target {
            id: 9,
            name: "ups".into(),
            address: address.into(),
            kind: "nut".into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            tags: tags
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<BTreeMap<_, _>>(),
            credential,
            group_name: String::new(),
            position: 0,
        }
    }

    /// Un faux `upsd` : répond aux commandes reçues à partir des transcriptions
    /// capturées, et note ce qu'il a reçu.
    async fn fake_upsd(
        answers: Vec<(&'static str, String)>,
    ) -> (String, tokio::task::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let handle = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let (read, mut write) = socket.into_split();
            let mut lines = BufReader::new(read).lines();
            let mut received = Vec::new();
            while let Ok(Some(line)) = lines.next_line().await {
                received.push(line.clone());
                let answer = answers
                    .iter()
                    .find(|(command, _)| line.starts_with(command))
                    .map(|(_, answer)| answer.clone())
                    .unwrap_or_else(|| "ERR UNKNOWN-COMMAND\n".to_string());
                write.write_all(answer.as_bytes()).await.unwrap();
                if line == "LOGOUT" {
                    break;
                }
            }
            received
        });
        (address, handle)
    }

    fn server_answers() -> Vec<(&'static str, String)> {
        vec![
            ("VER", "Network UPS Tools upsd 2.8.2 - https://www.networkupstools.org/\n".into()),
            ("USERNAME", "OK\n".into()),
            ("PASSWORD", "OK\n".into()),
            ("LIST UPS", include_str!("testdata/list_ups.txt").into()),
            ("LIST VAR rack", include_str!("testdata/var_rack_on_battery.txt").into()),
            ("LIST VAR office", include_str!("testdata/err_data_stale.txt").into()),
            ("LOGOUT", "OK Goodbye\n".into()),
        ]
    }

    fn find<'a>(samples: &'a [Sample], name: &str, ups: &str) -> Option<&'a Sample> {
        samples
            .iter()
            .find(|s| s.metric == name && s.labels.get("ups").map(String::as_str) == Some(ups))
    }

    #[tokio::test]
    async fn deux_onduleurs_dont_un_muet() {
        let (address, server) = fake_upsd(server_answers()).await;
        let samples =
            NutCollector::new().probe(&target(&address, &[], Credential::None)).await.unwrap();
        assert_eq!(find(&samples, "nut_ups_on_battery", "rack").unwrap().value, 1.0);
        assert_eq!(find(&samples, "nut_ups_data_stale", "rack").unwrap().value, 0.0);
        assert_eq!(find(&samples, "nut_ups_data_stale", "office").unwrap().value, 1.0);
        assert_eq!(find(&samples, "nut_ups_driver_connected", "office").unwrap().value, 1.0);
        assert!(find(&samples, "nut_ups_on_battery", "office").is_none());
        let version = samples.iter().find(|s| s.metric == "nut_server_info").unwrap();
        assert_eq!(version.labels["version"], "2.8.2");
        assert_eq!(samples.iter().find(|s| s.metric == "nut_server_ups").unwrap().value, 2.0);
        let received = server.await.unwrap();
        // Lecture seule : ni LOGIN, ni identifiant quand aucun n'est configuré.
        assert!(received.iter().all(|l| !l.starts_with("LOGIN") && !l.starts_with("USERNAME")));
        assert_eq!(received.last().map(String::as_str), Some("LOGOUT"));
    }

    #[tokio::test]
    async fn un_seul_onduleur_choisi_et_un_identifiant() {
        let (address, server) = fake_upsd(server_answers()).await;
        let credential =
            Credential::UsernamePassword { username: "dumbmonit".into(), password: "p\"w".into() };
        let samples = NutCollector::new()
            .probe(&target(&address, &[("ups", "rack")], credential))
            .await
            .unwrap();
        assert!(samples.iter().all(|s| s.labels.get("ups").is_none_or(|u| u == "rack")));
        let received = server.await.unwrap();
        assert!(received.contains(&"USERNAME \"dumbmonit\"".to_string()));
        assert!(received.contains(&"PASSWORD \"p\\\"w\"".to_string()));
        assert!(!received.iter().any(|l| l.starts_with("LIST VAR office")));
    }

    #[tokio::test]
    async fn un_onduleur_inconnu_est_une_erreur_de_configuration() {
        let (address, _server) = fake_upsd(server_answers()).await;
        let error = NutCollector::new()
            .probe(&target(&address, &[("ups", "garage")], Credential::None))
            .await
            .unwrap_err();
        assert!(
            matches!(error, ProbeError::Config(ref m) if m.contains("rack, office")),
            "{error}"
        );
        assert!(!error.means_down());
    }

    #[tokio::test]
    async fn un_acces_refuse_n_est_pas_une_panne() {
        let mut answers = server_answers();
        answers.retain(|(c, _)| *c != "LIST UPS");
        answers.push(("LIST UPS", "ERR ACCESS-DENIED\n".into()));
        let (address, _server) = fake_upsd(answers).await;
        let error =
            NutCollector::new().probe(&target(&address, &[], Credential::None)).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
    }

    #[tokio::test]
    async fn un_pilote_arrete_est_une_mesure() {
        let mut answers = server_answers();
        answers.retain(|(c, _)| *c != "LIST VAR office");
        answers.push((
            "LIST VAR office",
            include_str!("testdata/err_driver_not_connected.txt").into(),
        ));
        let (address, _server) = fake_upsd(answers).await;
        let samples =
            NutCollector::new().probe(&target(&address, &[], Credential::None)).await.unwrap();
        assert_eq!(find(&samples, "nut_ups_driver_connected", "office").unwrap().value, 0.0);
        assert_eq!(find(&samples, "nut_ups_data_stale", "office").unwrap().value, 1.0);
    }

    #[tokio::test]
    async fn un_serveur_ferme_est_injoignable() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap().to_string();
        drop(listener);
        let error =
            NutCollector::new().probe(&target(&address, &[], Credential::None)).await.unwrap_err();
        assert!(error.means_down(), "{error}");
    }

    #[test]
    fn les_adresses_et_les_options() {
        let o = |address: &str, tags: &[(&str, &str)]| {
            Options::from_target(&target(address, tags, Credential::None))
        };
        let options = o("nas.lan", &[]).unwrap();
        assert_eq!((options.host.as_str(), options.port), ("nas.lan", 3493));
        assert!(options.ups.is_empty());
        assert_eq!(options.timeout, DEFAULT_REQUEST_TIMEOUT);
        assert_eq!(o("nas.lan:3494", &[("port", "1")]).unwrap().port, 3494);
        assert_eq!(o("ups@nas.lan", &[("port", "3500")]).unwrap().port, 3500);
        assert_eq!(o("ups@nas.lan", &[]).unwrap().host, "nas.lan");
        assert_eq!(o("[fd00::5]:3493", &[]).unwrap().host, "fd00::5");
        assert_eq!(o("fd00::5", &[]).unwrap().host, "fd00::5");
        assert_eq!(o("x", &[("ups", "rack, office@nas")]).unwrap().ups, ["rack", "office"]);
        for (key, value) in [("port", "0"), ("request_timeout_seconds", "90"), ("port", "abc")] {
            assert!(matches!(o("x", &[(key, value)]), Err(ProbeError::Config(_))), "{key}={value}");
        }
        assert!(o("  ", &[]).is_err());
    }

    #[tokio::test]
    async fn un_jeton_n_est_pas_un_identifiant_nut() {
        let (address, _server) = fake_upsd(server_answers()).await;
        let error = NutCollector::new()
            .probe(&target(&address, &[], Credential::ApiToken { token: "x".into() }))
            .await
            .unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)));
    }
}
