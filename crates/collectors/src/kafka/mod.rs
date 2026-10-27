//! Apache Kafka (et compatibles : Redpanda, Confluent), par son protocole natif.
//!
//! # Pourquoi un client écrit à la main
//!
//! `librdkafka` est une bibliothèque C++ : la compiler dans l'image statique
//! musl, c'est y ajouter un compilateur C++, `cmake`, `zlib`, et un binaire de
//! l'agent relais nettement plus lourd. La crate Rust pure `rskafka` est
//! agréable mais ne sait ni lister les groupes de consommateurs ni lire leurs
//! offsets validés : or le retard (lag) est la moitié de ce qu'on veut voir. Le
//! sous-ensemble employé ici tient en six requêtes (voir [`wire`]), sans
//! dépendance nouvelle, et passe par la même couche TCP/TLS que Redis et
//! MongoDB (`socket.rs`).
//!
//! # Ce que ce module existe pour voir
//!
//! * **des partitions sous-répliquées** (moins de réplicas synchronisés que de
//!   réplicas) : un courtier est tombé ou en retard, la prochaine panne perd
//!   des données ;
//! * **des partitions hors ligne** (aucun leader) : lectures et écritures
//!   échouent pour cette partition ;
//! * **l'absence de contrôleur** : le cluster ne peut plus élire de leader ;
//! * **le retard des groupes de consommateurs** : l'écart entre le dernier
//!   offset de chaque partition et celui que le groupe a validé.
//!
//! Le retard est calculé au mieux : il demande de joindre chaque courtier à
//! l'adresse qu'il annonce (`advertised.listeners`). Si un courtier n'est pas
//! joignable de là où tourne DumbMonit, les métriques de santé du cluster
//! restent, le retard est omis ou partiel, et `kafka_unreachable_brokers` le dit.
//!
//! # Authentification
//!
//! Aucune, ou `SASL/PLAIN` (utilisateur et mot de passe), qui n'est acceptée
//! qu'avec TLS : sans lui, le mot de passe voyagerait en clair. `SASL/SCRAM` et
//! Kerberos ne sont pas pris en charge ; Kafka accepte plusieurs mécanismes sur
//! un même écouteur, ou plusieurs écouteurs.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `port` | `9092` | Port du courtier d'amorçage. |
//! | `tls` | `false` | L'écouteur n'accepte que TLS. |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable. |
//! | `request_timeout_seconds` | `10` | Délai de toute l'interrogation. |
//! | `max_groups` | `50` | Groupes dont on calcule le retard (0 à 500, 0 pour aucun). |

pub mod wire;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::Duration;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, MetricKind, ProbeError, Sample, Target};

use crate::socket::{Connection, SocketOptions};
use crate::uptime::tags;
use wire::{Cluster, Reader};

pub const DEFAULT_PORT: u16 = 9092;
pub const DEFAULT_MAX_GROUPS: u32 = 50;

/// Délai de connexion aux courtiers autres que celui d'amorçage : l'un d'eux,
/// injoignable, ne doit pas coûter tout le budget de l'interrogation.
const SECONDARY_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Default)]
pub struct KafkaCollector;

impl KafkaCollector {
    pub fn new() -> Self {
        Self
    }
}

type Sasl = Option<(String, String)>;

/// Une connexion à un courtier, avec le numéro de corrélation de ses requêtes.
struct Session {
    conn: Connection,
    correlation: i32,
}

impl Session {
    async fn open(options: &SocketOptions, sasl: &Sasl) -> Result<Self, ProbeError> {
        let conn = Connection::open(options, "Kafka").await?;
        let mut session = Self { conn, correlation: 0 };
        if let Some((username, password)) = sasl {
            session.authenticate(username, password).await?;
        }
        Ok(session)
    }

    /// Une requête, sa réponse (corps seul, sans la longueur ni la corrélation).
    async fn call(&mut self, api: i16, version: i16, body: &[u8]) -> Result<Vec<u8>, ProbeError> {
        self.correlation = self.correlation.wrapping_add(1);
        let request = wire::frame(api, version, self.correlation, body);
        self.conn.write_all(&request).await?;
        let mut length = [0u8; 4];
        self.conn.read_exact(&mut length).await?;
        let length = i32::from_be_bytes(length);
        if !(4..=wire::MAX_FRAME as i32).contains(&length) {
            return Err(ProbeError::Protocol(
                "The answer is not a Kafka response: check the port, and whether the listener \
                 expects TLS or SASL."
                    .to_string(),
            ));
        }
        let mut payload = vec![0u8; length as usize];
        self.conn.read_exact(&mut payload).await?;
        let correlation = Reader::new(&payload).i32()?;
        if correlation != self.correlation {
            return Err(ProbeError::Protocol(
                "Kafka answered a different request than the one sent.".to_string(),
            ));
        }
        Ok(payload.split_off(4))
    }

    async fn authenticate(&mut self, username: &str, password: &str) -> Result<(), ProbeError> {
        let reply = self
            .call(
                wire::API_SASL_HANDSHAKE,
                wire::VERSION_SASL_HANDSHAKE,
                &wire::sasl_handshake_request("PLAIN"),
            )
            .await?;
        match wire::parse_error_code(&reply)? {
            wire::ERROR_NONE => {}
            wire::ERROR_UNSUPPORTED_SASL_MECHANISM => {
                return Err(ProbeError::Config(
                    "The broker does not offer SASL/PLAIN on this listener. DumbMonit supports \
                     PLAIN only."
                        .to_string(),
                ));
            }
            code => {
                return Err(ProbeError::Protocol(format!("SASL handshake refused (code {code}).")));
            }
        }
        let reply = self
            .call(
                wire::API_SASL_AUTHENTICATE,
                wire::VERSION_SASL_AUTHENTICATE,
                &wire::sasl_plain_request(username, password),
            )
            .await?;
        match wire::parse_sasl_authenticate(&reply)? {
            (wire::ERROR_NONE, _) => Ok(()),
            (wire::ERROR_SASL_AUTHENTICATION_FAILED, message) => Err(ProbeError::Auth(format!(
                "Kafka refused the credentials{}. Check the user name and password.",
                message.map(|m| format!(" ({m})")).unwrap_or_default()
            ))),
            (code, _) => {
                Err(ProbeError::Protocol(format!("SASL authentication failed (code {code}).")))
            }
        }
    }

    async fn metadata(&mut self) -> Result<Cluster, ProbeError> {
        let reply = self
            .call(wire::API_METADATA, wire::VERSION_METADATA, &wire::metadata_request())
            .await?;
        wire::parse_metadata(&reply)
    }

    async fn close(self) {
        self.conn.close().await;
    }
}

/// Les connexions ouvertes aux courtiers, une par identifiant.
struct Pool<'a> {
    options: &'a SocketOptions,
    sasl: &'a Sasl,
    addresses: HashMap<i32, (String, u16)>,
    open: HashMap<i32, Session>,
}

impl<'a> Pool<'a> {
    fn new(options: &'a SocketOptions, sasl: &'a Sasl, cluster: &Cluster) -> Self {
        let addresses = cluster
            .brokers
            .iter()
            .filter_map(|b| Some((b.id, (b.host.clone(), u16::try_from(b.port).ok()?))))
            .collect();
        Self { options, sasl, addresses, open: HashMap::new() }
    }

    async fn session(&mut self, node: i32) -> Result<&mut Session, ProbeError> {
        if !self.open.contains_key(&node) {
            let (host, port) = self
                .addresses
                .get(&node)
                .cloned()
                .ok_or_else(|| ProbeError::Protocol(format!("Unknown Kafka broker {node}.")))?;
            let mut options = self.options.clone();
            options.host = host;
            options.port = port;
            options.timeout = options.timeout.min(SECONDARY_TIMEOUT);
            let session = Session::open(&options, self.sasl).await?;
            self.open.insert(node, session);
        }
        self.open
            .get_mut(&node)
            .ok_or_else(|| ProbeError::Protocol("Kafka session lost.".to_string()))
    }
}

/// Retard d'un groupe : messages en attente par rubrique.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupLag {
    pub group: String,
    pub topics: BTreeMap<String, i64>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LagReport {
    /// Groupes de consommateurs connus du cluster.
    pub total_groups: usize,
    pub groups: Vec<GroupLag>,
    pub unreachable_brokers: usize,
}

/// Le retard des groupes, au mieux : une erreur sur un courtier ou un groupe
/// écarte cet élément, jamais toute la collecte.
async fn consumer_lag(pool: &mut Pool<'_>, cluster: &Cluster, max_groups: usize) -> LagReport {
    let mut report = LagReport::default();

    // Chaque groupe est connu du courtier qui le coordonne : on les interroge tous.
    let mut listed: Vec<(i32, String)> = Vec::new();
    for broker in &cluster.brokers {
        let Ok(session) = pool.session(broker.id).await else {
            report.unreachable_brokers += 1;
            continue;
        };
        let Ok(reply) = session.call(wire::API_LIST_GROUPS, wire::VERSION_LIST_GROUPS, &[]).await
        else {
            continue;
        };
        if let Ok((wire::ERROR_NONE, groups)) = wire::parse_list_groups(&reply) {
            listed.extend(
                groups
                    .into_iter()
                    .filter(|(_, protocol)| protocol == "consumer")
                    .map(|(group, _)| (broker.id, group)),
            );
        }
    }
    listed.sort_by(|a, b| a.1.cmp(&b.1));
    report.total_groups = listed.len();
    listed.truncate(max_groups);

    let mut committed: Vec<(String, Vec<wire::PartitionOffset>)> = Vec::new();
    for (node, group) in listed {
        let Ok(session) = pool.session(node).await else { continue };
        let request = wire::offset_fetch_request(&group);
        let Ok(reply) =
            session.call(wire::API_OFFSET_FETCH, wire::VERSION_OFFSET_FETCH, &request).await
        else {
            continue;
        };
        if let Ok((wire::ERROR_NONE, offsets)) = wire::parse_offset_fetch(&reply) {
            committed.push((group, offsets));
        }
    }

    // Les derniers offsets, demandés au leader de chaque partition.
    let leaders: HashMap<(&str, i32), i32> = cluster
        .topics
        .iter()
        .flat_map(|t| t.partitions.iter().map(move |p| ((t.name.as_str(), p.index), p.leader)))
        .collect();
    let mut wanted: BTreeMap<i32, BTreeSet<(String, i32)>> = BTreeMap::new();
    for (_, offsets) in &committed {
        for (topic, partition, _) in offsets {
            if let Some(leader) = leaders.get(&(topic.as_str(), *partition))
                && *leader >= 0
            {
                wanted.entry(*leader).or_default().insert((topic.clone(), *partition));
            }
        }
    }
    let mut ends: HashMap<(String, i32), i64> = HashMap::new();
    for (leader, partitions) in wanted {
        let Ok(session) = pool.session(leader).await else {
            report.unreachable_brokers += 1;
            continue;
        };
        let partitions: Vec<(String, i32)> = partitions.into_iter().collect();
        let request = wire::list_offsets_request(&partitions);
        let Ok(reply) =
            session.call(wire::API_LIST_OFFSETS, wire::VERSION_LIST_OFFSETS, &request).await
        else {
            continue;
        };
        if let Ok(found) = wire::parse_list_offsets(&reply) {
            for (topic, partition, end) in found {
                ends.insert((topic, partition), end);
            }
        }
    }

    for (group, offsets) in committed {
        let mut topics: BTreeMap<String, i64> = BTreeMap::new();
        for (topic, partition, offset) in offsets {
            if let Some(end) = ends.get(&(topic.clone(), partition)) {
                *topics.entry(topic).or_insert(0) += (end - offset).max(0);
            }
        }
        report.groups.push(GroupLag { group, topics });
    }
    report.unreachable_brokers = report.unreachable_brokers.min(cluster.brokers.len());
    report
}

/// Ce que la collecte a rapporté.
pub struct Replies {
    pub cluster: Cluster,
    /// `None` si le retard n'a pas été demandé (`max_groups` à 0).
    pub lag: Option<LagReport>,
}

fn sasl(target: &Target, options: &SocketOptions) -> Result<Sasl, ProbeError> {
    match &target.credential {
        Credential::None => Ok(None),
        Credential::UsernamePassword { username, password } if !username.trim().is_empty() => {
            if !options.tls {
                return Err(ProbeError::Config(
                    "SASL/PLAIN sends the password in clear text: tick TLS (the listener must \
                     use SASL_SSL), or remove the credential for a PLAINTEXT listener."
                        .to_string(),
                ));
            }
            Ok(Some((username.trim().to_string(), password.clone())))
        }
        other => Err(ProbeError::Config(format!(
            "Kafka expects no credential or a user name and password (SASL/PLAIN), configured: \
             {other}"
        ))),
    }
}

pub async fn read(target: &Target) -> Result<Replies, ProbeError> {
    let options = SocketOptions::from_target(target, DEFAULT_PORT)?;
    let sasl = sasl(target, &options)?;
    let max_groups = tags::parse_u32(target, "max_groups", DEFAULT_MAX_GROUPS, 0..=500)? as usize;
    let work = async {
        let mut bootstrap = Session::open(&options, &sasl).await?;
        let cluster = bootstrap.metadata().await?;
        bootstrap.close().await;
        let lag = if max_groups == 0 {
            None
        } else {
            let mut pool = Pool::new(&options, &sasl, &cluster);
            Some(consumer_lag(&mut pool, &cluster, max_groups).await)
        };
        Ok::<Replies, ProbeError>(Replies { cluster, lag })
    };
    match tokio::time::timeout(options.timeout, work).await {
        Ok(result) => result,
        Err(_) => Err(ProbeError::Timeout(options.timeout)),
    }
}

#[async_trait]
impl Collector for KafkaCollector {
    fn kind(&self) -> &'static str {
        "kafka"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let replies = read(target).await?;
        Ok(samples(&replies, chrono::Utc::now().timestamp_millis()))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let options = SocketOptions::from_target(target, DEFAULT_PORT)?;
        let sasl = sasl(target, &options)?;
        let attempt = async {
            let mut session = Session::open(&options, &sasl).await?;
            session.metadata().await.map(|_| ())
        };
        match tokio::time::timeout(options.timeout, attempt).await {
            Ok(Ok(())) => Ok(Some("kafka".to_string())),
            Ok(Err(error)) => Err(error),
            Err(_) => Err(ProbeError::Timeout(options.timeout)),
        }
    }
}

pub fn samples(replies: &Replies, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let gauge = |name: &str, value: f64| Sample::new(name, value, MetricKind::Gauge, ts_ms);
    let cluster = &replies.cluster;

    out.push(
        gauge("kafka_cluster_info", 1.0)
            .with_label("cluster_id", cluster.cluster_id.clone().unwrap_or_default()),
    );
    out.push(gauge("kafka_brokers", cluster.brokers.len() as f64));
    out.push(gauge("kafka_controller_present", if cluster.controller >= 0 { 1.0 } else { 0.0 }));

    let mut topics = 0.0;
    let mut partitions = 0.0;
    let mut under_replicated = 0.0;
    let mut offline = 0.0;
    for topic in &cluster.topics {
        if !topic.internal {
            topics += 1.0;
        }
        let mut topic_under = 0.0;
        let mut topic_offline = 0.0;
        for partition in &topic.partitions {
            partitions += 1.0;
            if partition.leader < 0 {
                topic_offline += 1.0;
            }
            if partition.in_sync < partition.replicas {
                topic_under += 1.0;
            }
        }
        under_replicated += topic_under;
        offline += topic_offline;
        // Le détail seulement là où il y a un problème : une série par rubrique
        // saine n'apprendrait rien et alourdirait la base.
        if topic_under > 0.0 {
            out.push(
                gauge("kafka_topic_under_replicated_partitions", topic_under)
                    .with_label("topic", topic.name.clone()),
            );
        }
        if topic_offline > 0.0 {
            out.push(
                gauge("kafka_topic_offline_partitions", topic_offline)
                    .with_label("topic", topic.name.clone()),
            );
        }
    }
    out.push(gauge("kafka_topics", topics));
    out.push(gauge("kafka_partitions", partitions));
    out.push(gauge("kafka_under_replicated_partitions", under_replicated));
    out.push(gauge("kafka_offline_partitions", offline));

    if let Some(lag) = &replies.lag {
        out.push(gauge("kafka_consumer_groups", lag.total_groups as f64));
        out.push(gauge("kafka_unreachable_brokers", lag.unreachable_brokers as f64));
        for group in &lag.groups {
            out.push(
                gauge("kafka_consumer_group_lag_total", group.topics.values().sum::<i64>() as f64)
                    .with_label("group", group.group.clone()),
            );
            for (topic, lag) in &group.topics {
                out.push(
                    gauge("kafka_consumer_group_lag", *lag as f64)
                        .with_label("group", group.group.clone())
                        .with_label("topic", topic.clone()),
                );
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use super::wire::{Writer, sample_metadata};
    use super::*;
    use crate::uptime::tags::test_support::cible;

    fn find<'a>(samples: &'a [Sample], name: &str, labels: &[(&str, &str)]) -> Option<&'a Sample> {
        samples.iter().find(|s| {
            s.metric == name
                && labels.iter().all(|(k, v)| s.labels.get(*k).map(String::as_str) == Some(v))
        })
    }

    #[test]
    fn sante_du_cluster_avec_partitions_en_difficulte() {
        let cluster = wire::parse_metadata(&sample_metadata()).unwrap();
        let samples = samples(&Replies { cluster, lag: None }, 0);
        assert_eq!(find(&samples, "kafka_brokers", &[]).unwrap().value, 2.0);
        assert_eq!(find(&samples, "kafka_controller_present", &[]).unwrap().value, 1.0);
        // Les rubriques internes ne comptent pas dans le nombre de rubriques.
        assert_eq!(find(&samples, "kafka_topics", &[]).unwrap().value, 1.0);
        assert_eq!(find(&samples, "kafka_partitions", &[]).unwrap().value, 3.0);
        assert_eq!(find(&samples, "kafka_offline_partitions", &[]).unwrap().value, 1.0);
        assert_eq!(find(&samples, "kafka_under_replicated_partitions", &[]).unwrap().value, 2.0);
        let orders = find(&samples, "kafka_topic_offline_partitions", &[("topic", "orders")]);
        assert_eq!(orders.unwrap().value, 1.0);
        assert!(find(&samples, "kafka_consumer_groups", &[]).is_none());
        let info = find(&samples, "kafka_cluster_info", &[]).unwrap();
        assert_eq!(info.labels["cluster_id"], "cluster-abc");
    }

    #[test]
    fn sans_controleur() {
        let mut cluster = wire::parse_metadata(&sample_metadata()).unwrap();
        cluster.controller = -1;
        let samples = samples(&Replies { cluster, lag: None }, 0);
        assert_eq!(find(&samples, "kafka_controller_present", &[]).unwrap().value, 0.0);
    }

    #[test]
    fn le_retard_par_groupe_et_par_rubrique() {
        let cluster = wire::parse_metadata(&sample_metadata()).unwrap();
        let lag = LagReport {
            total_groups: 3,
            groups: vec![GroupLag {
                group: "billing".into(),
                topics: BTreeMap::from([("orders".to_string(), 40), ("refunds".to_string(), 2)]),
            }],
            unreachable_brokers: 1,
        };
        let samples = samples(&Replies { cluster, lag: Some(lag) }, 0);
        assert_eq!(find(&samples, "kafka_consumer_groups", &[]).unwrap().value, 3.0);
        assert_eq!(find(&samples, "kafka_unreachable_brokers", &[]).unwrap().value, 1.0);
        let total = find(&samples, "kafka_consumer_group_lag_total", &[("group", "billing")]);
        assert_eq!(total.unwrap().value, 42.0);
        let orders = find(&samples, "kafka_consumer_group_lag", &[("topic", "orders")]);
        assert_eq!(orders.unwrap().value, 40.0);
    }

    #[test]
    fn sasl_plain_exige_tls() {
        let mut target = cible("kafka", "kafka.lan", &[]);
        target.credential =
            Credential::UsernamePassword { username: "dumbmonit".into(), password: "x".into() };
        let options = SocketOptions::from_target(&target, DEFAULT_PORT).unwrap();
        assert!(matches!(sasl(&target, &options), Err(ProbeError::Config(_))));
        let mut target = cible("kafka", "kafka.lan", &[("tls", "true")]);
        target.credential =
            Credential::UsernamePassword { username: "dumbmonit".into(), password: "x".into() };
        let options = SocketOptions::from_target(&target, DEFAULT_PORT).unwrap();
        assert_eq!(sasl(&target, &options).unwrap().unwrap().0, "dumbmonit");
        target.credential = Credential::ApiToken { token: "t".into() };
        assert!(matches!(sasl(&target, &options), Err(ProbeError::Config(_))));
    }

    /// Un courtier de façade : répond aux cinq requêtes de lecture avec un
    /// cluster d'un nœud (lui-même), une rubrique saine et un groupe en retard.
    async fn fake_broker() -> u16 {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let mut metadata = Writer::new();
        metadata.i32(0).i32(1).i32(1).string("127.0.0.1").i32(i32::from(port)).null_string();
        metadata.string("c1").i32(1).i32(1);
        metadata.i16(0).string("orders").i8(0).i32(1);
        metadata.i16(0).i32(0).i32(1).i32(1).i32(1).i32(1).i32(1).i32(0);
        let metadata = metadata.finish();
        let mut groups = Writer::new();
        groups.i16(0).i32(1).string("billing").string("consumer");
        let groups = groups.finish();
        let mut fetch = Writer::new();
        fetch.i32(1).string("orders").i32(1).i32(0).i64(100).null_string().i16(0).i16(0);
        let fetch = fetch.finish();
        let mut ends = Writer::new();
        ends.i32(1).string("orders").i32(1).i32(0).i16(0).i64(-1).i64(150);
        let ends = ends.finish();

        tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else { break };
                let (metadata, groups, fetch, ends) =
                    (metadata.clone(), groups.clone(), fetch.clone(), ends.clone());
                tokio::spawn(async move {
                    loop {
                        let mut length = [0u8; 4];
                        if socket.read_exact(&mut length).await.is_err() {
                            break;
                        }
                        let mut payload = vec![0u8; i32::from_be_bytes(length) as usize];
                        if socket.read_exact(&mut payload).await.is_err() {
                            break;
                        }
                        let body = match i16::from_be_bytes([payload[0], payload[1]]) {
                            wire::API_METADATA => &metadata,
                            wire::API_LIST_GROUPS => &groups,
                            wire::API_OFFSET_FETCH => &fetch,
                            wire::API_LIST_OFFSETS => &ends,
                            _ => break,
                        };
                        let mut out = ((body.len() + 4) as i32).to_be_bytes().to_vec();
                        out.extend_from_slice(&payload[4..8]);
                        out.extend_from_slice(body);
                        if socket.write_all(&out).await.is_err() {
                            break;
                        }
                    }
                });
            }
        });
        port
    }

    #[tokio::test]
    async fn interrogation_complete_avec_retard() {
        let port = fake_broker().await;
        let target = cible("kafka", &format!("127.0.0.1:{port}"), &[]);
        let samples = KafkaCollector::new().probe(&target).await.unwrap();
        assert_eq!(find(&samples, "kafka_brokers", &[]).unwrap().value, 1.0);
        assert_eq!(find(&samples, "kafka_under_replicated_partitions", &[]).unwrap().value, 0.0);
        let lag = find(&samples, "kafka_consumer_group_lag", &[("group", "billing")]);
        assert_eq!(lag.unwrap().value, 50.0);
        assert_eq!(find(&samples, "kafka_unreachable_brokers", &[]).unwrap().value, 0.0);
        assert_eq!(
            KafkaCollector::new().discover(&target).await.unwrap().as_deref(),
            Some("kafka")
        );

        // Sans retard demandé : la santé du cluster seule.
        let target = cible("kafka", &format!("127.0.0.1:{port}"), &[("max_groups", "0")]);
        let samples = KafkaCollector::new().probe(&target).await.unwrap();
        assert!(find(&samples, "kafka_consumer_groups", &[]).is_none());
    }

    #[tokio::test]
    async fn un_service_qui_n_est_pas_kafka_est_une_erreur_de_protocole() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 64];
            let _ = socket.read(&mut buf).await;
            let _ = socket.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n").await;
        });
        let target = cible("kafka", &format!("127.0.0.1:{port}"), &[]);
        let error = KafkaCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Protocol(_)) && !error.means_down(), "{error}");
    }

    #[tokio::test]
    async fn un_port_ferme_est_une_panne() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let target = cible("kafka", &format!("127.0.0.1:{port}"), &[]);
        let error = KafkaCollector::new().probe(&target).await.unwrap_err();
        assert!(error.means_down(), "{error}");
    }
}
