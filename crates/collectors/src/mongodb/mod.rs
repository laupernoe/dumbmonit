//! MongoDB : `serverStatus` et, dans un jeu de réplicas, `replSetGetStatus`.
//!
//! Le compte recommandé porte le seul rôle intégré `clusterMonitor` : il lit
//! l'état du serveur et de la réplication, et aucun document d'aucune base.
//!
//! Les pannes que ce module existe pour voir :
//!
//! * **plus de primaire** : l'élection a échoué ou la majorité est perdue, et
//!   plus aucune écriture n'est acceptée ;
//! * **un membre injoignable ou une réplique en retard** : le jour où le
//!   primaire tombe, la réplique élue n'a pas les dernières écritures ;
//! * **des connexions au plafond** : les applications reçoivent des refus ;
//! * **le cache WiredTiger saturé** : au-delà de 95 % d'occupation ou de 20 %
//!   de pages modifiées, les requêtes elles-mêmes évincent des pages et
//!   ralentissent ;
//! * **des assertions** `regular` : des erreurs internes du serveur.
//!
//! Le pilote officiel n'est pas employé : il tirerait une pile entière
//! (résolution SRV, sessions, pool) pour deux commandes. Ce module parle
//! `OP_MSG` directement et s'authentifie en SCRAM-SHA-256 (voir [`wire`]).
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `port` | `27017` | Port, si l'adresse n'en précise pas. |
//! | `auth_source` | `admin` | Base où le compte a été créé. |
//! | `tls` | `false` | TLS dès la connexion. |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable. |
//! | `request_timeout_seconds` | `10` | Délai de toute l'interrogation. |

pub mod bson;
pub mod status;
mod wire;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};

use crate::socket::{Connection, SocketOptions};
use crate::uptime::tags;
use bson::{Bson, Document};
use wire::Wire;

pub const DEFAULT_PORT: u16 = 27017;
pub const DEFAULT_AUTH_SOURCE: &str = "admin";

#[derive(Default)]
pub struct MongodbCollector;

impl MongodbCollector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Collector for MongodbCollector {
    fn kind(&self) -> &'static str {
        "mongodb"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let replies = read(target).await?;
        let ts_ms = chrono::Utc::now().timestamp_millis();
        let mut samples = status::server_samples(&replies.server, ts_ms);
        if let Some(replset) = &replies.replset {
            samples.extend(status::replset_samples(replset, ts_ms));
        }
        Ok(samples)
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        read(target).await?;
        Ok(Some("mongodb".to_string()))
    }
}

pub struct Replies {
    pub server: Document,
    pub replset: Option<Document>,
}

pub async fn read(target: &Target) -> Result<Replies, ProbeError> {
    let options = SocketOptions::from_target(target, DEFAULT_PORT)?;
    let source = tags::tag(target, "auth_source").unwrap_or(DEFAULT_AUTH_SOURCE).to_string();
    let login = match &target.credential {
        Credential::None => None,
        Credential::UsernamePassword { username, password } if !username.trim().is_empty() => {
            Some((username.trim().to_string(), password.clone()))
        }
        other => {
            return Err(ProbeError::Config(format!(
                "MongoDB expects a user name and password, configured: {other}"
            )));
        }
    };
    let mut wire = Wire::new(Connection::open(&options, "MongoDB").await?);
    let result = dialogue(&mut wire, &source, login).await;
    wire.close().await;
    result
}

async fn dialogue(
    wire: &mut Wire,
    source: &str,
    login: Option<(String, String)>,
) -> Result<Replies, ProbeError> {
    if let Some((username, password)) = login {
        wire::authenticate(wire, source, &username, &password).await?;
    }
    // `tcmalloc` et `locks` sont écartés : volumineux, et rien n'y est lu.
    let command = Document::new()
        .with("serverStatus", Bson::Int32(1))
        .with("tcmalloc", Bson::Int32(0))
        .with("locks", Bson::Int32(0));
    let server = wire.command("admin", command).await?;
    if !wire::is_ok(&server) {
        return Err(command_error("serverStatus", &server));
    }
    let replset = if status::in_replica_set(&server) {
        let reply =
            wire.command("admin", Document::new().with("replSetGetStatus", Bson::Int32(1))).await?;
        if !wire::is_ok(&reply) {
            return Err(command_error("replSetGetStatus", &reply));
        }
        Some(reply)
    } else {
        None
    };
    Ok(Replies { server, replset })
}

fn command_error(command: &str, reply: &Document) -> ProbeError {
    let (code, message) = wire::error_of(reply);
    match code {
        // Unauthorized
        13 => ProbeError::Auth(format!(
            "MongoDB refused {command} ({message}): give the account the clusterMonitor role."
        )),
        // Unauthenticated
        18 => ProbeError::Auth(format!(
            "MongoDB requires authentication for {command}: add the monitoring account."
        )),
        _ => ProbeError::Protocol(format!("MongoDB refused {command}: {message}")),
    }
}

#[cfg(test)]
mod tests {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use super::*;
    use crate::uptime::tags::test_support::cible;

    /// Un faux `mongod` sans authentification, qui rend les réponses réelles
    /// d'un MongoDB 8.0.13 (`status.rs`).
    async fn faux_mongod(server_status: Vec<u8>) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            loop {
                let mut header = [0u8; 16];
                if socket.read_exact(&mut header).await.is_err() {
                    return;
                }
                let length = i32::from_le_bytes(header[0..4].try_into().unwrap()) as usize;
                let request_id = i32::from_le_bytes(header[4..8].try_into().unwrap());
                let mut rest = vec![0u8; length - 16];
                socket.read_exact(&mut rest).await.unwrap();
                let command = wire::parse_reply(&rest).unwrap();
                let body = match command.0.first().map(|(k, _)| k.as_str()) {
                    Some("serverStatus") => server_status.clone(),
                    Some("replSetGetStatus") => {
                        include_bytes!("testdata/mongodb_8.0.13_replset_status.bson").to_vec()
                    }
                    _ => Document::new()
                        .with("ok", Bson::Double(0.0))
                        .with("code", Bson::Int32(59))
                        .with("errmsg", Bson::String("no such command".into()))
                        .encode(),
                };
                let total = 16 + 5 + body.len();
                let mut reply = Vec::new();
                reply.extend_from_slice(&(total as i32).to_le_bytes());
                reply.extend_from_slice(&0i32.to_le_bytes());
                reply.extend_from_slice(&request_id.to_le_bytes());
                reply.extend_from_slice(&2013i32.to_le_bytes());
                reply.extend_from_slice(&[0, 0, 0, 0, 0]);
                reply.extend_from_slice(&body);
                socket.write_all(&reply).await.unwrap();
            }
        });
        address.to_string()
    }

    #[tokio::test]
    async fn interrogation_complete() {
        let status = include_bytes!("testdata/mongodb_8.0.13_server_status.bson").to_vec();
        let address = faux_mongod(status).await;
        let samples = MongodbCollector::new().probe(&cible("mongodb", &address, &[])).await.unwrap();
        assert!(samples.iter().any(|s| s.metric == "mongodb_replset_primary_present"));
        assert!(samples.iter().any(|s| s.metric == "mongodb_wiredtiger_cache_used_percent"));
    }

    #[tokio::test]
    async fn un_compte_sans_role_est_une_erreur_d_identifiant() {
        let refused = Document::new()
            .with("ok", Bson::Double(0.0))
            .with("code", Bson::Int32(13))
            .with("codeName", Bson::String("Unauthorized".into()))
            .with("errmsg", Bson::String("not authorized on admin".into()))
            .encode();
        let address = faux_mongod(refused).await;
        let error =
            MongodbCollector::new().probe(&cible("mongodb", &address, &[])).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(ref m) if m.contains("clusterMonitor")), "{error}");
        assert!(!error.means_down());
    }

    #[tokio::test]
    async fn un_service_qui_n_est_pas_mongodb() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap().to_string();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let _ = socket.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n").await;
        });
        let error =
            MongodbCollector::new().probe(&cible("mongodb", &address, &[])).await.unwrap_err();
        assert!(matches!(error, ProbeError::Protocol(_)), "{error}");
    }

    /// Contre une vraie instance : `DUMBMONIT_TEST_MONGODB=hôte:port`,
    /// `DUMBMONIT_TEST_MONGODB_PASSWORD` (utilisateur `dumbmonit`). Avec
    /// `DUMBMONIT_TEST_CAPTURE=répertoire`, les deux réponses y sont écrites en
    /// BSON, noms d'hôtes remplacés.
    #[tokio::test]
    #[ignore = "demande un MongoDB joignable"]
    async fn mongodb_reel() {
        let address = std::env::var("DUMBMONIT_TEST_MONGODB").unwrap();
        let password = std::env::var("DUMBMONIT_TEST_MONGODB_PASSWORD").unwrap();
        let mut target = cible("mongodb", &address, &[]);
        target.credential = Credential::UsernamePassword { username: "dumbmonit".into(), password };
        let samples = MongodbCollector::new().probe(&target).await.unwrap();
        for sample in &samples {
            println!("{} {:?} {}", sample.metric, sample.labels, sample.value);
        }
        if let Ok(dir) = std::env::var("DUMBMONIT_TEST_CAPTURE") {
            let replies = read(&target).await.unwrap();
            let host = replies.server.str("host").unwrap_or_default().to_string();
            let clean = |doc: &Document| pseudonymise(doc, &host);
            std::fs::write(
                format!("{dir}/mongodb_server_status.bson"),
                clean(&replies.server).encode(),
            )
            .unwrap();
            if let Some(replset) = &replies.replset {
                std::fs::write(format!("{dir}/mongodb_replset_status.bson"), clean(replset).encode())
                    .unwrap();
            }
        }
    }

    /// Remplace le nom d'hôte du conteneur et l'adresse des membres.
    fn pseudonymise(doc: &Document, host: &str) -> Document {
        fn walk(value: &Bson, host: &str) -> Bson {
            match value {
                Bson::String(text) => Bson::String(
                    text.replace(host, "db1").replace("mongo:27017", "db1.lan:27017"),
                ),
                Bson::Document(doc) => Bson::Document(Document(
                    doc.0.iter().map(|(k, v)| (k.clone(), walk(v, host))).collect(),
                )),
                Bson::Array(items) => Bson::Array(items.iter().map(|v| walk(v, host)).collect()),
                other => other.clone(),
            }
        }
        Document(doc.0.iter().map(|(k, v)| (k.clone(), walk(v, host))).collect())
    }
}
