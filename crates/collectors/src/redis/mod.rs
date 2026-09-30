//! Redis et Valkey : ce que le serveur dit de lui-même par `INFO`.
//!
//! Une connexion, une authentification, une commande. Aucune clé n'est lue ni
//! écrite : le compte recommandé est un utilisateur ACL limité à `+info +ping`,
//! qui ne peut littéralement rien faire d'autre.
//!
//! Les pannes que ce module existe pour voir :
//!
//! * **la mémoire au plafond** (`maxmemory`) : selon la politique, Redis
//!   évince des clés en silence ou refuse toute écriture (`OOM command not
//!   allowed`) ;
//! * **des connexions refusées** : `maxclients` atteint, les applications
//!   reçoivent une erreur à la connexion ;
//! * **une réplication coupée ou en retard** : le lien d'une réplique vers son
//!   primaire, et le retard de chaque réplique vu du primaire ;
//! * **une persistance en échec** : le dernier `BGSAVE` ou la dernière écriture
//!   AOF a échoué (disque plein, droits), et Redis refuse alors les écritures si
//!   `stop-writes-on-bgsave-error` est actif.
//!
//! # Identifiants
//!
//! * un utilisateur et son mot de passe (Redis 6 et suivants, Valkey) :
//!   `AUTH utilisateur motdepasse` ;
//! * un mot de passe seul (`requirepass`, porté par un « jeton ») :
//!   `AUTH motdepasse` ;
//! * rien, pour un serveur sans authentification.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `port` | `6379` | Port, si l'adresse n'en précise pas. |
//! | `tls` | `false` | TLS dès la connexion (`tls-port`). |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable. |
//! | `request_timeout_seconds` | `10` | Délai de toute l'interrogation. |

pub mod info;
mod resp;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};

use crate::socket::{Connection, SocketOptions};
use info::Info;
use resp::{Client, Reply};

pub const DEFAULT_PORT: u16 = 6379;

#[derive(Default)]
pub struct RedisCollector;

impl RedisCollector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Collector for RedisCollector {
    fn kind(&self) -> &'static str {
        "redis"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let info = read_info(target).await?;
        Ok(info::samples(&info, chrono::Utc::now().timestamp_millis()))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        read_info(target).await?;
        Ok(Some("redis".to_string()))
    }
}

/// Ce que l'identifiant de la cible fait envoyer à `AUTH`.
fn auth_args(credential: &Credential) -> Result<Option<Vec<String>>, ProbeError> {
    match credential {
        Credential::None => Ok(None),
        Credential::UsernamePassword { username, password } => {
            let username = username.trim();
            if username.is_empty() {
                Ok(Some(vec![password.clone()]))
            } else {
                Ok(Some(vec![username.to_string(), password.clone()]))
            }
        }
        Credential::ApiToken { token } if !token.is_empty() => Ok(Some(vec![token.clone()])),
        other => Err(ProbeError::Config(format!(
            "Redis expects a user name and password, a password alone, or no credential; \
             configured: {other}"
        ))),
    }
}

pub async fn read_info(target: &Target) -> Result<Info, ProbeError> {
    let options = SocketOptions::from_target(target, DEFAULT_PORT)?;
    let auth = auth_args(&target.credential)?;
    let mut client = Client::new(Connection::open(&options, "Redis").await?);
    let result = dialogue(&mut client, auth).await;
    client.close().await;
    result
}

async fn dialogue(client: &mut Client, auth: Option<Vec<String>>) -> Result<Info, ProbeError> {
    if let Some(args) = auth {
        let mut command = vec!["AUTH"];
        command.extend(args.iter().map(String::as_str));
        match client.command(&command).await? {
            Reply::Simple(_) => {}
            Reply::Error(message) => return Err(auth_error(&message)),
            other => return Err(ProbeError::Protocol(format!("unexpected AUTH reply: {other:?}"))),
        }
    }
    match client.command(&["INFO"]).await? {
        Reply::Bulk(Some(body)) => {
            let info = Info::parse(&String::from_utf8_lossy(&body));
            if !info.looks_like_redis() {
                return Err(ProbeError::Protocol(
                    "INFO answered without redis_version: this does not look like Redis or \
                     Valkey."
                        .to_string(),
                ));
            }
            Ok(info)
        }
        Reply::Error(message) => Err(command_error(&message)),
        other => Err(ProbeError::Protocol(format!("unexpected INFO reply: {other:?}"))),
    }
}

/// Un refus d'`AUTH`. Le message du serveur ne cite ni le nom ni le mot de passe.
fn auth_error(message: &str) -> ProbeError {
    if message.starts_with("WRONGPASS") || message.contains("invalid password") {
        return ProbeError::Auth(
            "Redis refused the user name or password (WRONGPASS). Check the credential."
                .to_string(),
        );
    }
    ProbeError::Auth(format!("Redis refused AUTH: {message}"))
}

/// Un refus d'`INFO` : compte sans le droit, serveur qui exige un mot de passe,
/// mode protégé.
fn command_error(message: &str) -> ProbeError {
    let code = message.split_whitespace().next().unwrap_or_default();
    match code {
        "NOAUTH" => ProbeError::Auth(
            "Redis requires authentication: add the monitoring user's credential.".to_string(),
        ),
        "NOPERM" => ProbeError::Auth(
            "The Redis user may not run INFO: grant it +info (ACL SETUSER dumbmonit +info +ping)."
                .to_string(),
        ),
        "DENIED" => ProbeError::Config(
            "Redis runs in protected mode and refuses connections from other hosts: set a \
             password or bind it to the right interface."
                .to_string(),
        ),
        "LOADING" => ProbeError::Unreachable(
            "Redis is loading its dataset in memory and answers nothing else yet.".to_string(),
        ),
        _ => ProbeError::Protocol(format!("Redis refused INFO: {message}")),
    }
}

#[cfg(test)]
mod tests {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use super::*;
    use crate::uptime::tags::test_support::cible;

    /// Un faux Redis qui exige `AUTH dumbmonit secret`, puis sert la vraie
    /// réponse d'un Redis 8.2.1.
    async fn faux_redis() -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                tokio::spawn(async move {
                    let mut authed = false;
                    let mut buf = vec![0u8; 4096];
                    loop {
                        let Ok(n) = socket.read(&mut buf).await else { return };
                        if n == 0 {
                            return;
                        }
                        let request = String::from_utf8_lossy(&buf[..n]).to_string();
                        let reply: Vec<u8> = if request.contains("AUTH") {
                            if request.contains("dumbmonit") && request.contains("secret") {
                                authed = true;
                                b"+OK\r\n".to_vec()
                            } else {
                                b"-WRONGPASS invalid username-password pair or user is disabled.\r\n"
                                    .to_vec()
                            }
                        } else if request.contains("INFO") {
                            if authed {
                                let body = include_str!("testdata/redis_8.2.1_master.info");
                                let mut out = format!("${}\r\n", body.len()).into_bytes();
                                out.extend_from_slice(body.as_bytes());
                                out.extend_from_slice(b"\r\n");
                                out
                            } else {
                                b"-NOAUTH Authentication required.\r\n".to_vec()
                            }
                        } else {
                            b"-ERR unknown command\r\n".to_vec()
                        };
                        if socket.write_all(&reply).await.is_err() {
                            return;
                        }
                    }
                });
            }
        });
        address.to_string()
    }

    fn avec(address: &str, credential: Credential) -> Target {
        let mut target = cible("redis", address, &[]);
        target.credential = credential;
        target
    }

    #[tokio::test]
    async fn interrogation_complete() {
        let address = faux_redis().await;
        let credential =
            Credential::UsernamePassword { username: "dumbmonit".into(), password: "secret".into() };
        let samples = RedisCollector::new().probe(&avec(&address, credential)).await.unwrap();
        assert!(samples.iter().any(|s| s.metric == "redis_keys" && s.value == 202.0));

        let wrong =
            Credential::UsernamePassword { username: "dumbmonit".into(), password: "nope".into() };
        let error = RedisCollector::new().probe(&avec(&address, wrong)).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
        assert!(!error.means_down());

        let error = RedisCollector::new().probe(&avec(&address, Credential::None)).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(ref m) if m.contains("authentication")), "{error}");
    }

    #[tokio::test]
    async fn un_service_qui_n_est_pas_redis() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap().to_string();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let _ = socket.write_all(b"SSH-2.0-OpenSSH_9.6\r\n").await;
        });
        let error = RedisCollector::new().probe(&avec(&address, Credential::None)).await.unwrap_err();
        assert!(matches!(error, ProbeError::Protocol(_)), "{error}");
    }

    #[test]
    fn les_refus_sont_classes() {
        assert!(matches!(command_error("NOPERM User x has no permissions"), ProbeError::Auth(_)));
        assert!(matches!(command_error("DENIED Redis is running in protected mode"), ProbeError::Config(_)));
        assert!(command_error("LOADING Redis is loading the dataset in memory").means_down());
        assert!(matches!(auth_error("WRONGPASS invalid"), ProbeError::Auth(_)));
    }

    #[test]
    fn un_mot_de_passe_seul_passe_par_un_jeton() {
        assert_eq!(
            auth_args(&Credential::ApiToken { token: "pw".into() }).unwrap(),
            Some(vec!["pw".to_string()])
        );
        assert_eq!(auth_args(&Credential::None).unwrap(), None);
        assert!(auth_args(&Credential::SnmpCommunity { community: "public".into() }).is_err());
    }

    /// Contre une vraie instance : `DUMBMONIT_TEST_REDIS=hôte:port` et
    /// `DUMBMONIT_TEST_REDIS_PASSWORD` (utilisateur `dumbmonit`).
    #[tokio::test]
    #[ignore = "demande un Redis joignable"]
    async fn redis_reel() {
        let address = std::env::var("DUMBMONIT_TEST_REDIS").unwrap();
        let password = std::env::var("DUMBMONIT_TEST_REDIS_PASSWORD").unwrap();
        let credential = Credential::UsernamePassword { username: "dumbmonit".into(), password };
        let samples = RedisCollector::new().probe(&avec(&address, credential)).await.unwrap();
        for sample in &samples {
            println!("{} {:?} {}", sample.metric, sample.labels, sample.value);
        }
    }
}
