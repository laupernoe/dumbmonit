//! Paquets SSH (RFC 4253) : juste ce qu'il faut pour arriver à la clé d'hôte.
//!
//! Avant `NEWKEYS` le transport est en clair : un paquet est `uint32 longueur`,
//! `byte remplissage`, la charge utile, puis le remplissage, sans MAC. Tout ce que
//! lit ce module vient d'un serveur potentiellement hostile : chaque longueur est
//! bornée avant d'allouer, et un paquet tronqué est une erreur, jamais un panic.

use dumbmonit_proto::ProbeError;
use ring::rand::{SecureRandom, SystemRandom};
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::uptime::session::{Deadline, SessionError};

pub const MSG_DISCONNECT: u8 = 1;
pub const MSG_IGNORE: u8 = 2;
pub const MSG_DEBUG: u8 = 4;
pub const MSG_KEXINIT: u8 = 20;
pub const MSG_KEX_ECDH_INIT: u8 = 30;
pub const MSG_KEX_ECDH_REPLY: u8 = 31;

/// Taille maximale d'un paquet accepté (`packet_length`). La RFC impose aux
/// implémentations d'en gérer 35 000 ; la réponse d'échange de clés en fait
/// quelques centaines, même avec une clé RSA de 8 192 bits.
pub const MAX_PACKET_BYTES: usize = 35_000;

/// Algorithmes de clé d'hôte proposés, par ordre de préférence.
pub const HOST_KEY_ALGORITHMS: [&str; 4] =
    ["ssh-ed25519", "ecdsa-sha2-nistp256", "rsa-sha2-512", "rsa-sha2-256"];

/// Famille de clé d'hôte demandée par l'utilisateur.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyType {
    Auto,
    Ed25519,
    Ecdsa,
    Rsa,
}

impl KeyType {
    pub fn parse(raw: &str) -> Result<Self, ProbeError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "auto" | "" => Ok(Self::Auto),
            "ed25519" => Ok(Self::Ed25519),
            "ecdsa" => Ok(Self::Ecdsa),
            "rsa" => Ok(Self::Rsa),
            other => Err(ProbeError::Config(format!(
                "\"host_key_type\" expects \"auto\", \"ed25519\", \"ecdsa\" or \"rsa\", got \
                 \"{other}\""
            ))),
        }
    }

    /// Liste des algorithmes proposés pour cette famille.
    pub fn algorithms(self) -> Vec<&'static str> {
        match self {
            Self::Auto => HOST_KEY_ALGORITHMS.to_vec(),
            Self::Ed25519 => vec!["ssh-ed25519"],
            Self::Ecdsa => vec!["ecdsa-sha2-nistp256"],
            Self::Rsa => vec!["rsa-sha2-512", "rsa-sha2-256"],
        }
    }
}

/// Encode un paquet non chiffré autour de `payload`.
pub fn encode_packet(payload: &[u8]) -> Vec<u8> {
    // Longueur totale (hors MAC) multiple de 8, remplissage d'au moins 4 octets.
    let mut padding = 8 - (5 + payload.len()) % 8;
    if padding < 4 {
        padding += 8;
    }
    let length = 1 + payload.len() + padding;
    let mut out = Vec::with_capacity(4 + length);
    out.extend_from_slice(&(length as u32).to_be_bytes());
    out.push(padding as u8);
    out.extend_from_slice(payload);
    out.resize(4 + length, 0);
    out
}

/// Ajoute une `string` SSH (longueur sur quatre octets, puis les octets).
pub fn put_string(out: &mut Vec<u8>, value: &[u8]) {
    out.extend_from_slice(&(value.len() as u32).to_be_bytes());
    out.extend_from_slice(value);
}

/// Charge utile de notre `SSH_MSG_KEXINIT`.
pub fn client_kexinit(host_key_algorithms: &[&str]) -> Result<Vec<u8>, String> {
    let mut payload = vec![MSG_KEXINIT];
    let mut cookie = [0u8; 16];
    SystemRandom::new().fill(&mut cookie).map_err(|_| "no random source available".to_string())?;
    payload.extend_from_slice(&cookie);
    // Chiffrements et MAC : le serveur échoue (et se déconnecte) s'il n'y a aucun
    // point commun, avant même de répondre à l'échange de clés. On en propose donc
    // une liste large, bien que rien ne soit jamais chiffré par la sonde.
    let host_keys = host_key_algorithms.join(",");
    let lists: [&str; 10] = [
        "curve25519-sha256,curve25519-sha256@libssh.org",
        &host_keys,
        "chacha20-poly1305@openssh.com,aes256-ctr,aes192-ctr,aes128-ctr,aes256-gcm@openssh.com,\
         aes128-gcm@openssh.com",
        "chacha20-poly1305@openssh.com,aes256-ctr,aes192-ctr,aes128-ctr,aes256-gcm@openssh.com,\
         aes128-gcm@openssh.com",
        "hmac-sha2-256,hmac-sha2-512,hmac-sha1",
        "hmac-sha2-256,hmac-sha2-512,hmac-sha1",
        "none",
        "none",
        "",
        "",
    ];
    for list in lists {
        put_string(&mut payload, list.as_bytes());
    }
    payload.push(0); // first_kex_packet_follows
    payload.extend_from_slice(&0u32.to_be_bytes()); // réservé
    Ok(payload)
}

/// Charge utile de `SSH_MSG_KEX_ECDH_INIT` : notre clé publique éphémère.
pub fn ecdh_init(public_key: &[u8]) -> Vec<u8> {
    let mut payload = vec![MSG_KEX_ECDH_INIT];
    put_string(&mut payload, public_key);
    payload
}

/// Lecteur d'une charge utile, borné : chaque lecture vérifie ce qui reste.
pub struct Cursor<'a> {
    data: &'a [u8],
}

impl<'a> Cursor<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    pub fn u8(&mut self) -> Result<u8, String> {
        let (first, rest) = self.data.split_first().ok_or("truncated SSH message")?;
        self.data = rest;
        Ok(*first)
    }

    pub fn skip(&mut self, count: usize) -> Result<(), String> {
        if self.data.len() < count {
            return Err("truncated SSH message".to_string());
        }
        self.data = &self.data[count..];
        Ok(())
    }

    pub fn u32(&mut self) -> Result<u32, String> {
        if self.data.len() < 4 {
            return Err("truncated SSH message".to_string());
        }
        let (head, rest) = self.data.split_at(4);
        self.data = rest;
        Ok(u32::from_be_bytes([head[0], head[1], head[2], head[3]]))
    }

    /// Lit une `string`. La longueur annoncée ne peut pas dépasser ce qui reste.
    pub fn string(&mut self) -> Result<&'a [u8], String> {
        let length = self.u32()? as usize;
        if length > self.data.len() {
            return Err(format!(
                "SSH string of {length} bytes but only {} remain",
                self.data.len()
            ));
        }
        let (value, rest) = self.data.split_at(length);
        self.data = rest;
        Ok(value)
    }

    /// Lit une `name-list` (noms séparés par des virgules).
    pub fn name_list(&mut self) -> Result<Vec<String>, String> {
        let raw = self.string()?;
        Ok(String::from_utf8_lossy(raw)
            .split(',')
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .collect())
    }
}

/// Ce que dit le `SSH_MSG_KEXINIT` du serveur et qui nous intéresse.
#[derive(Debug, PartialEq, Eq)]
pub struct ServerKexInit {
    pub kex_algorithms: Vec<String>,
    pub host_key_algorithms: Vec<String>,
}

pub fn parse_kexinit(payload: &[u8]) -> Result<ServerKexInit, String> {
    let mut cursor = Cursor::new(payload);
    if cursor.u8()? != MSG_KEXINIT {
        return Err("not a SSH_MSG_KEXINIT message".to_string());
    }
    cursor.skip(16)?; // cookie
    let kex_algorithms = cursor.name_list()?;
    let host_key_algorithms = cursor.name_list()?;
    Ok(ServerKexInit { kex_algorithms, host_key_algorithms })
}

/// Blob de clé d'hôte (`K_S`) d'un `SSH_MSG_KEX_ECDH_REPLY`.
pub fn parse_ecdh_reply(payload: &[u8]) -> Result<Vec<u8>, String> {
    let mut cursor = Cursor::new(payload);
    if cursor.u8()? != MSG_KEX_ECDH_REPLY {
        return Err("not a SSH_MSG_KEX_ECDH_REPLY message".to_string());
    }
    let blob = cursor.string()?;
    if blob.is_empty() {
        return Err("empty host key".to_string());
    }
    Ok(blob.to_vec())
}

/// Nom de l'algorithme annoncé en tête d'un blob de clé d'hôte.
pub fn key_type_of(blob: &[u8]) -> Option<String> {
    let mut cursor = Cursor::new(blob);
    let name = cursor.string().ok()?;
    Some(String::from_utf8_lossy(name).chars().take(40).collect())
}

/// Lit des paquets SSH sur un flux, en conservant l'excédent d'une lecture à
/// l'autre : le serveur envoie souvent sa bannière et son `KEXINIT` d'un coup.
pub struct PacketReader {
    buffer: Vec<u8>,
}

impl PacketReader {
    /// `leftover` : octets déjà lus après la bannière.
    pub fn new(leftover: Vec<u8>) -> Self {
        Self { buffer: leftover }
    }

    /// Renvoie la charge utile du prochain paquet.
    pub async fn packet<S: AsyncRead + Unpin>(
        &mut self,
        stream: &mut S,
        deadline: Deadline,
    ) -> Result<Vec<u8>, SessionError> {
        self.fill(stream, 4, deadline).await?;
        let length =
            u32::from_be_bytes([self.buffer[0], self.buffer[1], self.buffer[2], self.buffer[3]])
                as usize;
        if !(5..=MAX_PACKET_BYTES).contains(&length) {
            return Err(SessionError::Io(format!(
                "invalid SSH packet length ({length}): the service probably does not speak SSH \
                 here, or insists on encryption before the key exchange"
            )));
        }
        self.fill(stream, 4 + length, deadline).await?;
        let packet: Vec<u8> = self.buffer.drain(..4 + length).collect();
        let padding = usize::from(packet[4]);
        if padding < 4 || padding + 1 > length {
            return Err(SessionError::Io(format!("invalid SSH padding length ({padding})")));
        }
        Ok(packet[5..4 + length - padding].to_vec())
    }

    async fn fill<S: AsyncRead + Unpin>(
        &mut self,
        stream: &mut S,
        wanted: usize,
        deadline: Deadline,
    ) -> Result<(), SessionError> {
        while self.buffer.len() < wanted {
            let mut chunk = [0u8; 2048];
            let read = deadline.wait(stream.read(&mut chunk)).await??;
            if read == 0 {
                return Err(SessionError::Io("the service closed the connection".to_string()));
            }
            self.buffer.extend_from_slice(&chunk[..read]);
        }
        Ok(())
    }
}

/// Décrit un `SSH_MSG_DISCONNECT` pour le journal.
pub fn describe_disconnect(payload: &[u8]) -> String {
    let mut cursor = Cursor::new(payload);
    let _ = cursor.u8();
    let code = cursor.u32().unwrap_or(0);
    let reason = cursor
        .string()
        .map(|raw| String::from_utf8_lossy(raw).chars().take(120).collect::<String>())
        .unwrap_or_default();
    format!("the server disconnected (code {code}): {reason}")
}

#[cfg(test)]
mod tests {
    use std::io::Cursor as IoCursor;
    use std::time::Duration;

    use super::*;

    fn deadline() -> Deadline {
        Deadline::starting_now(Duration::from_secs(5))
    }

    #[test]
    fn un_paquet_encode_a_une_longueur_multiple_de_huit_et_un_remplissage_valide() {
        for taille in 0..40 {
            let paquet = encode_packet(&vec![7u8; taille]);
            let longueur = u32::from_be_bytes([paquet[0], paquet[1], paquet[2], paquet[3]]);
            assert_eq!(paquet.len(), 4 + longueur as usize);
            assert_eq!(paquet.len() % 8, 0, "taille {taille}");
            assert!(paquet[4] >= 4, "taille {taille}");
        }
    }

    #[tokio::test]
    async fn un_paquet_fait_laller_retour() {
        let charge = vec![MSG_KEXINIT, 1, 2, 3, 4, 5];
        let mut source = IoCursor::new(encode_packet(&charge));
        let mut reader = PacketReader::new(Vec::new());
        assert_eq!(reader.packet(&mut source, deadline()).await.unwrap(), charge);
    }

    #[tokio::test]
    async fn les_octets_deja_lus_sont_reutilises() {
        let charge = vec![9, 9, 9];
        let paquet = encode_packet(&charge);
        let (debut, fin) = paquet.split_at(6);
        let mut source = IoCursor::new(fin.to_vec());
        let mut reader = PacketReader::new(debut.to_vec());
        assert_eq!(reader.packet(&mut source, deadline()).await.unwrap(), charge);
    }

    #[tokio::test]
    async fn une_longueur_aberrante_est_refusee_sans_allocation() {
        for longueur in [0u32, 4, 35_001, u32::MAX] {
            let mut source = IoCursor::new(longueur.to_be_bytes().to_vec());
            let mut reader = PacketReader::new(Vec::new());
            assert!(reader.packet(&mut source, deadline()).await.is_err(), "{longueur}");
        }
    }

    #[tokio::test]
    async fn un_remplissage_incoherent_ou_un_paquet_tronque_est_une_erreur() {
        let mut paquet = encode_packet(&[1, 2, 3]);
        paquet[4] = 3; // remplissage inférieur au minimum
        let mut reader = PacketReader::new(Vec::new());
        assert!(reader.packet(&mut IoCursor::new(paquet), deadline()).await.is_err());

        let mut paquet = encode_packet(&[1, 2, 3]);
        paquet[4] = 250; // remplissage plus grand que le paquet
        let mut reader = PacketReader::new(Vec::new());
        assert!(reader.packet(&mut IoCursor::new(paquet), deadline()).await.is_err());

        let paquet = encode_packet(&[1, 2, 3]);
        let mut reader = PacketReader::new(Vec::new());
        let coupe = paquet[..paquet.len() - 2].to_vec();
        assert!(reader.packet(&mut IoCursor::new(coupe), deadline()).await.is_err());
    }

    #[test]
    fn le_kexinit_du_client_est_relisible() {
        let charge = client_kexinit(&HOST_KEY_ALGORITHMS).unwrap();
        let lu = parse_kexinit(&charge).unwrap();
        assert!(lu.kex_algorithms.contains(&"curve25519-sha256".to_string()));
        assert_eq!(lu.host_key_algorithms, HOST_KEY_ALGORITHMS.map(String::from).to_vec());
    }

    #[test]
    fn des_chaines_trop_longues_ou_tronquees_ne_font_pas_paniquer() {
        let mut charge = vec![MSG_KEX_ECDH_REPLY];
        charge.extend_from_slice(&u32::MAX.to_be_bytes());
        assert!(parse_ecdh_reply(&charge).is_err());
        assert!(parse_ecdh_reply(&[MSG_KEX_ECDH_REPLY]).is_err());
        assert!(parse_ecdh_reply(&[]).is_err());
        assert!(parse_kexinit(&[MSG_KEXINIT, 0, 0]).is_err());
        assert!(parse_kexinit(&[MSG_IGNORE]).is_err());
    }

    #[test]
    fn la_reponse_ecdh_donne_le_blob_et_son_type() {
        let mut blob = Vec::new();
        put_string(&mut blob, b"ssh-ed25519");
        put_string(&mut blob, &[1u8; 32]);
        let mut charge = vec![MSG_KEX_ECDH_REPLY];
        put_string(&mut charge, &blob);
        put_string(&mut charge, &[2u8; 32]);
        let lu = parse_ecdh_reply(&charge).unwrap();
        assert_eq!(lu, blob);
        assert_eq!(key_type_of(&lu).as_deref(), Some("ssh-ed25519"));
    }

    #[test]
    fn le_type_de_cle_se_lit_et_se_valide() {
        assert_eq!(KeyType::parse("ECDSA").unwrap(), KeyType::Ecdsa);
        assert_eq!(KeyType::parse("").unwrap(), KeyType::Auto);
        assert!(KeyType::parse("dsa").is_err());
        assert_eq!(KeyType::Auto.algorithms()[0], "ssh-ed25519");
    }

    #[test]
    fn un_disconnect_est_decrit() {
        let mut charge = vec![MSG_DISCONNECT];
        charge.extend_from_slice(&2u32.to_be_bytes());
        put_string(&mut charge, b"no matching cipher");
        assert!(describe_disconnect(&charge).contains("no matching cipher"));
    }
}
