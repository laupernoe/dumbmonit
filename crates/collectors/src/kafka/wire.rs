//! Le protocole Kafka, juste ce qu'il faut pour superviser, écrit à la main.
//!
//! Pourquoi pas une crate : `librdkafka` (C++) ne se compile pas dans l'image
//! statique musl sans l'alourdir, et la crate Rust pure `rskafka` ne sait ni
//! lister les groupes de consommateurs ni lire leurs offsets validés, c'est-à-dire
//! le retard (lag), la moitié de ce qu'on cherche. Les cinq requêtes employées
//! sont minuscules, et elles sont prises dans leurs **versions non flexibles** :
//! entiers à largeur fixe, chaînes préfixées par leur longueur, sans champs
//! étiquetés. Ces versions sont encore servies par Kafka 4.0, dont le plancher
//! (KIP-896) est : Metadata v4, ListOffsets v1, OffsetFetch v1 ; ListGroups v0
//! et SASL inchangés.
//!
//! | Requête | Clé | Version |
//! |---|---|---|
//! | `Metadata` | 3 | 5 |
//! | `ListOffsets` | 2 | 1 |
//! | `OffsetFetch` | 9 | 2 |
//! | `ListGroups` | 16 | 0 |
//! | `SaslHandshake` | 17 | 1 |
//! | `SaslAuthenticate` | 36 | 0 |

use dumbmonit_proto::ProbeError;

pub const API_LIST_OFFSETS: i16 = 2;
pub const API_METADATA: i16 = 3;
pub const API_OFFSET_FETCH: i16 = 9;
pub const API_LIST_GROUPS: i16 = 16;
pub const API_SASL_HANDSHAKE: i16 = 17;
pub const API_SASL_AUTHENTICATE: i16 = 36;

pub const VERSION_METADATA: i16 = 5;
pub const VERSION_LIST_OFFSETS: i16 = 1;
pub const VERSION_OFFSET_FETCH: i16 = 2;
pub const VERSION_LIST_GROUPS: i16 = 0;
pub const VERSION_SASL_HANDSHAKE: i16 = 1;
pub const VERSION_SASL_AUTHENTICATE: i16 = 0;

/// Identifiant du client, vu dans les journaux et les quotas du courtier.
const CLIENT_ID: &str = "dumbmonit";

/// Taille maximale acceptée pour une réponse : la métadonnée d'un très gros
/// cluster pèse quelques mégaoctets, pas plus. Au-delà, ce n'est pas Kafka.
pub const MAX_FRAME: usize = 32 * 1024 * 1024;

/// Codes d'erreur Kafka que le module distingue.
pub const ERROR_NONE: i16 = 0;
pub const ERROR_UNSUPPORTED_VERSION: i16 = 35;
pub const ERROR_UNSUPPORTED_SASL_MECHANISM: i16 = 33;
pub const ERROR_SASL_AUTHENTICATION_FAILED: i16 = 58;

/// Écrit un corps de requête.
#[derive(Default)]
pub struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn i8(&mut self, value: i8) -> &mut Self {
        self.buf.push(value as u8);
        self
    }

    pub fn i16(&mut self, value: i16) -> &mut Self {
        self.buf.extend_from_slice(&value.to_be_bytes());
        self
    }

    pub fn i32(&mut self, value: i32) -> &mut Self {
        self.buf.extend_from_slice(&value.to_be_bytes());
        self
    }

    pub fn i64(&mut self, value: i64) -> &mut Self {
        self.buf.extend_from_slice(&value.to_be_bytes());
        self
    }

    /// Chaîne : longueur sur 16 bits, puis les octets.
    pub fn string(&mut self, value: &str) -> &mut Self {
        self.i16(value.len() as i16);
        self.buf.extend_from_slice(value.as_bytes());
        self
    }

    /// Chaîne absente (longueur -1).
    pub fn null_string(&mut self) -> &mut Self {
        self.i16(-1)
    }

    /// Octets : longueur sur 32 bits, puis les octets.
    pub fn bytes(&mut self, value: &[u8]) -> &mut Self {
        self.i32(value.len() as i32);
        self.buf.extend_from_slice(value);
        self
    }

    pub fn finish(self) -> Vec<u8> {
        self.buf
    }
}

/// Une requête complète : longueur, en-tête (version 1), corps.
pub fn frame(api_key: i16, version: i16, correlation: i32, body: &[u8]) -> Vec<u8> {
    let mut header = Writer::new();
    header.i16(api_key).i16(version).i32(correlation).string(CLIENT_ID);
    let header = header.finish();
    let mut out = Vec::with_capacity(4 + header.len() + body.len());
    out.extend_from_slice(&((header.len() + body.len()) as i32).to_be_bytes());
    out.extend_from_slice(&header);
    out.extend_from_slice(body);
    out
}

/// Lit un corps de réponse. Chaque lecture vérifie qu'il reste assez d'octets :
/// un service qui n'est pas Kafka donne une erreur, pas un dépassement.
pub struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

fn truncated() -> ProbeError {
    ProbeError::Protocol(
        "Truncated Kafka response: this is not a Kafka broker, or its version is not supported."
            .to_string(),
    )
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], ProbeError> {
        let end = self.pos.checked_add(n).filter(|end| *end <= self.buf.len());
        let end = end.ok_or_else(truncated)?;
        let slice = &self.buf[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    pub fn i8(&mut self) -> Result<i8, ProbeError> {
        Ok(self.take(1)?[0] as i8)
    }

    pub fn i16(&mut self) -> Result<i16, ProbeError> {
        let bytes = self.take(2)?;
        Ok(i16::from_be_bytes([bytes[0], bytes[1]]))
    }

    pub fn i32(&mut self) -> Result<i32, ProbeError> {
        let bytes = self.take(4)?;
        Ok(i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn i64(&mut self) -> Result<i64, ProbeError> {
        let bytes = self.take(8)?;
        let mut array = [0u8; 8];
        array.copy_from_slice(bytes);
        Ok(i64::from_be_bytes(array))
    }

    /// Chaîne ; `None` pour une chaîne absente (longueur -1).
    pub fn nullable_string(&mut self) -> Result<Option<String>, ProbeError> {
        let len = self.i16()?;
        if len < 0 {
            return Ok(None);
        }
        let bytes = self.take(len as usize)?;
        Ok(Some(String::from_utf8_lossy(bytes).into_owned()))
    }

    pub fn string(&mut self) -> Result<String, ProbeError> {
        Ok(self.nullable_string()?.unwrap_or_default())
    }

    /// Octets ; vides pour une valeur absente.
    pub fn bytes(&mut self) -> Result<Vec<u8>, ProbeError> {
        let len = self.i32()?;
        if len < 0 {
            return Ok(Vec::new());
        }
        Ok(self.take(len as usize)?.to_vec())
    }

    /// Nombre d'éléments d'un tableau. Il ne peut pas dépasser le nombre
    /// d'octets restants (un élément en pèse au moins un) : une longueur
    /// délirante ne réserve donc jamais de mémoire.
    pub fn array_len(&mut self) -> Result<usize, ProbeError> {
        let len = self.i32()?;
        if len <= 0 {
            return Ok(0);
        }
        let len = len as usize;
        if len > self.buf.len().saturating_sub(self.pos) {
            return Err(truncated());
        }
        Ok(len)
    }
}

// ----------------------------------------------------------------- Metadata

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokerInfo {
    pub id: i32,
    pub host: String,
    pub port: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionInfo {
    pub index: i32,
    /// -1 : aucun leader, la partition est hors ligne.
    pub leader: i32,
    pub replicas: usize,
    pub in_sync: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopicInfo {
    pub name: String,
    pub internal: bool,
    pub partitions: Vec<PartitionInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Cluster {
    pub cluster_id: Option<String>,
    pub controller: i32,
    pub brokers: Vec<BrokerInfo>,
    pub topics: Vec<TopicInfo>,
}

/// Toutes les rubriques, sans en créer une seule.
pub fn metadata_request() -> Vec<u8> {
    let mut body = Writer::new();
    body.i32(-1).i8(0);
    body.finish()
}

pub fn parse_metadata(body: &[u8]) -> Result<Cluster, ProbeError> {
    let mut r = Reader::new(body);
    r.i32()?; // throttle_time_ms
    let mut cluster = Cluster::default();
    for _ in 0..r.array_len()? {
        let id = r.i32()?;
        let host = r.string()?;
        let port = r.i32()?;
        r.nullable_string()?; // rack
        cluster.brokers.push(BrokerInfo { id, host, port });
    }
    cluster.cluster_id = r.nullable_string()?;
    cluster.controller = r.i32()?;
    for _ in 0..r.array_len()? {
        let error = r.i16()?;
        let name = r.string()?;
        let internal = r.i8()? != 0;
        let mut partitions = Vec::new();
        for _ in 0..r.array_len()? {
            r.i16()?; // error_code de la partition
            let index = r.i32()?;
            let leader = r.i32()?;
            let mut count = || -> Result<usize, ProbeError> {
                let n = r.array_len()?;
                for _ in 0..n {
                    r.i32()?;
                }
                Ok(n)
            };
            let replicas = count()?;
            let in_sync = count()?;
            count()?; // offline_replicas
            partitions.push(PartitionInfo { index, leader, replicas, in_sync });
        }
        // 29 : le compte ne peut pas décrire cette rubrique ; 3 : inconnue.
        if error == ERROR_NONE || !partitions.is_empty() {
            cluster.topics.push(TopicInfo { name, internal, partitions });
        }
    }
    Ok(cluster)
}

// ---------------------------------------------------------------- ListGroups

/// `(groupe, type de protocole)` ; `consumer` pour les consommateurs.
pub fn parse_list_groups(body: &[u8]) -> Result<(i16, Vec<(String, String)>), ProbeError> {
    let mut r = Reader::new(body);
    let error = r.i16()?;
    let mut groups = Vec::new();
    for _ in 0..r.array_len()? {
        groups.push((r.string()?, r.string()?));
    }
    Ok((error, groups))
}

// --------------------------------------------------------------- OffsetFetch

/// Les offsets validés d'un groupe, pour toutes ses rubriques.
pub fn offset_fetch_request(group: &str) -> Vec<u8> {
    let mut body = Writer::new();
    body.string(group).i32(-1);
    body.finish()
}

/// Offset d'une partition : `(rubrique, partition, offset)`.
pub type PartitionOffset = (String, i32, i64);

/// `(rubrique, partition, offset validé)`, hors partitions sans offset (-1).
pub fn parse_offset_fetch(body: &[u8]) -> Result<(i16, Vec<PartitionOffset>), ProbeError> {
    let mut r = Reader::new(body);
    let mut offsets = Vec::new();
    for _ in 0..r.array_len()? {
        let topic = r.string()?;
        for _ in 0..r.array_len()? {
            let partition = r.i32()?;
            let offset = r.i64()?;
            r.nullable_string()?; // metadata
            let error = r.i16()?;
            if error == ERROR_NONE && offset >= 0 {
                offsets.push((topic.clone(), partition, offset));
            }
        }
    }
    let error = r.i16()?;
    Ok((error, offsets))
}

// -------------------------------------------------------------- ListOffsets

/// Le dernier offset (celui du prochain message) de chaque partition donnée.
pub fn list_offsets_request(partitions: &[(String, i32)]) -> Vec<u8> {
    let mut by_topic: Vec<(&str, Vec<i32>)> = Vec::new();
    for (topic, partition) in partitions {
        match by_topic.iter_mut().find(|(name, _)| name == topic) {
            Some((_, list)) => list.push(*partition),
            None => by_topic.push((topic.as_str(), vec![*partition])),
        }
    }
    let mut body = Writer::new();
    body.i32(-1).i32(by_topic.len() as i32);
    for (topic, list) in by_topic {
        body.string(topic).i32(list.len() as i32);
        for partition in list {
            // -1 : l'offset le plus récent.
            body.i32(partition).i64(-1);
        }
    }
    body.finish()
}

/// `(rubrique, partition, dernier offset)` des partitions sans erreur.
pub fn parse_list_offsets(body: &[u8]) -> Result<Vec<PartitionOffset>, ProbeError> {
    let mut r = Reader::new(body);
    let mut ends = Vec::new();
    for _ in 0..r.array_len()? {
        let topic = r.string()?;
        for _ in 0..r.array_len()? {
            let partition = r.i32()?;
            let error = r.i16()?;
            r.i64()?; // timestamp
            let offset = r.i64()?;
            if error == ERROR_NONE && offset >= 0 {
                ends.push((topic.clone(), partition, offset));
            }
        }
    }
    Ok(ends)
}

// --------------------------------------------------------------------- SASL

pub fn sasl_handshake_request(mechanism: &str) -> Vec<u8> {
    let mut body = Writer::new();
    body.string(mechanism);
    body.finish()
}

/// Le code d'erreur de `SaslHandshake` (le reste, la liste des mécanismes, ne sert pas).
pub fn parse_error_code(body: &[u8]) -> Result<i16, ProbeError> {
    Reader::new(body).i16()
}

/// `SASL/PLAIN` : `\0utilisateur\0mot de passe`.
pub fn sasl_plain_request(username: &str, password: &str) -> Vec<u8> {
    let mut token = Vec::with_capacity(2 + username.len() + password.len());
    token.push(0);
    token.extend_from_slice(username.as_bytes());
    token.push(0);
    token.extend_from_slice(password.as_bytes());
    let mut body = Writer::new();
    body.bytes(&token);
    body.finish()
}

/// `(code d'erreur, message)` de `SaslAuthenticate`.
pub fn parse_sasl_authenticate(body: &[u8]) -> Result<(i16, Option<String>), ProbeError> {
    let mut r = Reader::new(body);
    let error = r.i16()?;
    let message = r.nullable_string()?;
    Ok((error, message))
}

#[cfg(test)]
/// Réponse `Metadata` v5 : deux courtiers, une rubrique de deux partitions
/// dont une sous-répliquée et une sans leader.
pub(crate) fn sample_metadata() -> Vec<u8> {
    let mut w = Writer::new();
    w.i32(0);
    w.i32(2);
    w.i32(1).string("kafka-1.lan").i32(9092).null_string();
    w.i32(2).string("kafka-2.lan").i32(9092).string("rack-a");
    w.string("cluster-abc");
    w.i32(1);
    w.i32(2);
    // Rubrique « orders » : partition 0 saine, partition 1 sans leader.
    w.i16(0).string("orders").i8(0).i32(2);
    w.i16(0).i32(0).i32(1);
    w.i32(2).i32(1).i32(2);
    w.i32(2).i32(1).i32(2);
    w.i32(0);
    w.i16(5).i32(1).i32(-1);
    w.i32(2).i32(1).i32(2);
    w.i32(1).i32(2);
    w.i32(1).i32(1);
    // Rubrique interne, une partition sous-répliquée.
    w.i16(0).string("__consumer_offsets").i8(1).i32(1);
    w.i16(0).i32(0).i32(2);
    w.i32(2).i32(1).i32(2);
    w.i32(1).i32(2);
    w.i32(0);
    w.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_metadonnee_est_lue() {
        let cluster = parse_metadata(&sample_metadata()).unwrap();
        assert_eq!(cluster.cluster_id.as_deref(), Some("cluster-abc"));
        assert_eq!(cluster.controller, 1);
        assert_eq!(cluster.brokers.len(), 2);
        assert_eq!(
            cluster.brokers[1],
            BrokerInfo { id: 2, host: "kafka-2.lan".into(), port: 9092 }
        );
        let orders = &cluster.topics[0];
        assert_eq!((orders.name.as_str(), orders.internal), ("orders", false));
        assert_eq!(orders.partitions[1].leader, -1);
        assert_eq!(orders.partitions[1].in_sync, 1);
        let internal = &cluster.topics[1];
        assert!(internal.internal);
        assert_eq!((internal.partitions[0].replicas, internal.partitions[0].in_sync), (2, 1));
    }

    #[test]
    fn une_reponse_tronquee_est_une_erreur_de_protocole() {
        let full = sample_metadata();
        for cut in [0, 3, 10, full.len() - 1] {
            let error = parse_metadata(&full[..cut]).unwrap_err();
            assert!(matches!(error, ProbeError::Protocol(_)), "{cut}");
        }
        // Une longueur de tableau délirante ne réserve rien.
        let mut hostile = Writer::new();
        hostile.i32(0).i32(i32::MAX);
        assert!(parse_metadata(&hostile.finish()).is_err());
    }

    #[test]
    fn l_en_tete_et_la_longueur_de_la_requete() {
        let body = metadata_request();
        let bytes = frame(API_METADATA, VERSION_METADATA, 7, &body);
        let length = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        assert_eq!(length, bytes.len() - 4);
        assert_eq!(&bytes[4..6], &API_METADATA.to_be_bytes());
        assert_eq!(&bytes[6..8], &VERSION_METADATA.to_be_bytes());
        assert_eq!(&bytes[8..12], &7i32.to_be_bytes());
        // « dumbmonit » : longueur sur deux octets, puis le texte.
        assert_eq!(&bytes[12..14], &9i16.to_be_bytes());
        assert_eq!(&bytes[14..23], b"dumbmonit");
        assert_eq!(&bytes[23..], &body[..]);
        // Toutes les rubriques (-1), sans création automatique.
        assert_eq!(body, [0xff, 0xff, 0xff, 0xff, 0]);
    }

    #[test]
    fn les_groupes_et_les_offsets() {
        let mut w = Writer::new();
        w.i16(0).i32(2);
        w.string("billing").string("consumer");
        w.string("connect-sink").string("sink");
        let (error, groups) = parse_list_groups(&w.finish()).unwrap();
        assert_eq!(error, 0);
        assert_eq!(groups[0], ("billing".to_string(), "consumer".to_string()));

        // OffsetFetch : une rubrique, deux partitions (une sans offset), erreur globale 0.
        let mut w = Writer::new();
        w.i32(1).string("orders").i32(2);
        w.i32(0).i64(120).null_string().i16(0);
        w.i32(1).i64(-1).null_string().i16(0);
        w.i16(0);
        let (error, offsets) = parse_offset_fetch(&w.finish()).unwrap();
        assert_eq!(error, 0);
        assert_eq!(offsets, vec![("orders".to_string(), 0, 120)]);
    }

    #[test]
    fn list_offsets_regroupe_par_rubrique() {
        let request = list_offsets_request(&[
            ("orders".into(), 0),
            ("orders".into(), 1),
            ("payments".into(), 0),
        ]);
        let mut r = Reader::new(&request);
        assert_eq!(r.i32().unwrap(), -1);
        assert_eq!(r.i32().unwrap(), 2);
        assert_eq!(r.string().unwrap(), "orders");
        assert_eq!(r.i32().unwrap(), 2);

        let mut w = Writer::new();
        w.i32(1).string("orders").i32(2);
        w.i32(0).i16(0).i64(-1).i64(500);
        w.i32(1).i16(6).i64(-1).i64(-1);
        assert_eq!(parse_list_offsets(&w.finish()).unwrap(), vec![("orders".to_string(), 0, 500)]);
    }

    #[test]
    fn sasl_plain_et_son_resultat() {
        let request = sasl_plain_request("dumbmonit", "secret");
        assert_eq!(&request[..4], &(17i32).to_be_bytes());
        assert_eq!(&request[4..], b"\0dumbmonit\0secret");
        let mut w = Writer::new();
        w.i16(ERROR_SASL_AUTHENTICATION_FAILED).string("Authentication failed").bytes(&[]);
        let (error, message) = parse_sasl_authenticate(&w.finish()).unwrap();
        assert_eq!(error, ERROR_SASL_AUTHENTICATION_FAILED);
        assert_eq!(message.as_deref(), Some("Authentication failed"));
    }
}
