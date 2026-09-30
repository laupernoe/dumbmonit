//! Dialogue avec `mongod` : messages `OP_MSG` et authentification
//! SCRAM-SHA-256.
//!
//! `OP_MSG` (MongoDB 3.6 et suivants) : un en-tête de seize octets, des
//! drapeaux, puis une section de type 0 qui porte le document de commande. La
//! réponse a la même forme. La compression n'est jamais négociée, les réponses
//! arrivent donc toujours en clair.
//!
//! SCRAM-SHA-256 (RFC 7677) est le mécanisme par défaut des comptes créés
//! depuis MongoDB 4.0. Les calculs (PBKDF2, HMAC, SHA-256) passent par `ring`,
//! déjà présent pour TLS ; la préparation du mot de passe (SASLprep) par
//! `stringprep`, déjà employé par le pilote PostgreSQL. Aucune cryptographie
//! n'est écrite ici, seulement l'enchaînement des messages.

use std::num::NonZeroU32;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use dumbmonit_proto::ProbeError;
use ring::rand::SecureRandom;
use ring::{digest, hmac, pbkdf2};

use super::bson::{Bson, Document};
use crate::socket::Connection;

const OP_MSG: i32 = 2013;
/// Taille maximale d'une réponse : celle d'un document BSON, plus l'en-tête.
const MAX_MESSAGE: usize = 48 * 1024 * 1024;
/// En deçà, un serveur qui proposerait moins d'itérations serait suspect
/// (la RFC 7677 fixe 4096 comme minimum).
const MIN_ITERATIONS: u32 = 4096;

pub struct Wire {
    conn: Connection,
    next_id: i32,
}

impl Wire {
    pub fn new(conn: Connection) -> Self {
        Self { conn, next_id: 1 }
    }

    pub async fn close(self) {
        self.conn.close().await;
    }

    /// Envoie une commande à la base `db` et rend la réponse, `ok: 0` compris.
    pub async fn command(&mut self, db: &str, command: Document) -> Result<Document, ProbeError> {
        let command = command.with("$db", Bson::String(db.to_string()));
        let body = command.encode();
        let request_id = self.next_id;
        self.next_id += 1;

        let length = 16 + 4 + 1 + body.len();
        let mut message = Vec::with_capacity(length);
        message.extend_from_slice(&(length as i32).to_le_bytes());
        message.extend_from_slice(&request_id.to_le_bytes());
        message.extend_from_slice(&0i32.to_le_bytes());
        message.extend_from_slice(&OP_MSG.to_le_bytes());
        message.extend_from_slice(&0u32.to_le_bytes());
        message.push(0);
        message.extend_from_slice(&body);
        self.conn.write_all(&message).await?;

        let mut header = [0u8; 16];
        self.conn.read_exact(&mut header).await?;
        let length = i32::from_le_bytes(header[0..4].try_into().unwrap_or_default());
        let op_code = i32::from_le_bytes(header[12..16].try_into().unwrap_or_default());
        let length = usize::try_from(length).unwrap_or(0);
        if op_code != OP_MSG || !(21..=MAX_MESSAGE).contains(&length) {
            return Err(not_mongodb());
        }
        let mut rest = vec![0u8; length - 16];
        self.conn.read_exact(&mut rest).await?;
        parse_reply(&rest)
    }
}

/// Lit le corps d'une réponse `OP_MSG` (après l'en-tête) : drapeaux, puis la
/// section de type 0.
pub fn parse_reply(rest: &[u8]) -> Result<Document, ProbeError> {
    if rest.len() < 5 || rest[4] != 0 {
        return Err(not_mongodb());
    }
    Document::decode(&rest[5..]).map_err(|error| ProbeError::Protocol(error.to_string()))
}

fn not_mongodb() -> ProbeError {
    ProbeError::Protocol(
        "The server does not speak the MongoDB protocol (OP_MSG, MongoDB 3.6 or later). Check \
         the device type, the port, and whether the server expects TLS."
            .to_string(),
    )
}

/// Vrai si la commande a réussi.
pub fn is_ok(reply: &Document) -> bool {
    reply.number("ok") == Some(1.0)
}

/// Le motif d'une réponse `ok: 0`.
pub fn error_of(reply: &Document) -> (i64, String) {
    let code = reply.number("code").unwrap_or(0.0) as i64;
    let message = reply.str("errmsg").unwrap_or("unknown error").to_string();
    (code, message)
}

// ------------------------------------------------------------------ SCRAM

/// L'état d'un échange SCRAM-SHA-256, séparé du réseau pour être testé seul.
pub struct Scram {
    username: String,
    password: String,
    client_nonce: String,
    first_bare: String,
}

/// Ce que le client vérifie de la dernière réponse du serveur.
pub struct ScramFinal {
    pub message: String,
    server_signature: Vec<u8>,
}

impl Scram {
    pub fn new(username: &str, password: &str) -> Result<Self, ProbeError> {
        let mut nonce = [0u8; 24];
        ring::rand::SystemRandom::new()
            .fill(&mut nonce)
            .map_err(|_| ProbeError::Config("no random source for the SCRAM nonce".to_string()))?;
        Ok(Self::with_nonce(username, password, &STANDARD.encode(nonce)))
    }

    pub fn with_nonce(username: &str, password: &str, nonce: &str) -> Self {
        // RFC 5802 : `=` et `,` du nom d'utilisateur sont échappés.
        let name = username.replace('=', "=3D").replace(',', "=2C");
        let first_bare = format!("n={name},r={nonce}");
        Self {
            username: username.to_string(),
            password: password.to_string(),
            client_nonce: nonce.to_string(),
            first_bare,
        }
    }

    pub fn client_first(&self) -> String {
        format!("n,,{}", self.first_bare)
    }

    /// Calcule la preuve à partir du premier message du serveur.
    pub fn client_final(&self, server_first: &str) -> Result<ScramFinal, ProbeError> {
        let fields = scram_fields(server_first);
        let nonce = fields.iter().find(|(k, _)| *k == 'r').map(|(_, v)| *v);
        let salt = fields.iter().find(|(k, _)| *k == 's').map(|(_, v)| *v);
        let iterations = fields.iter().find(|(k, _)| *k == 'i').map(|(_, v)| *v);
        let (Some(nonce), Some(salt), Some(iterations)) = (nonce, salt, iterations) else {
            return Err(ProbeError::Protocol(format!(
                "unexpected SCRAM server message: {server_first}"
            )));
        };
        if !nonce.starts_with(&self.client_nonce) || nonce.len() <= self.client_nonce.len() {
            return Err(ProbeError::Protocol("SCRAM server nonce does not extend ours".to_string()));
        }
        let salt = STANDARD
            .decode(salt)
            .map_err(|_| ProbeError::Protocol("SCRAM salt is not base64".to_string()))?;
        let iterations: u32 = iterations
            .parse()
            .ok()
            .filter(|i| *i >= MIN_ITERATIONS)
            .ok_or_else(|| ProbeError::Protocol("SCRAM iteration count too low".to_string()))?;
        let password = stringprep::saslprep(&self.password).map_err(|_| {
            ProbeError::Config(
                "The password contains characters SCRAM-SHA-256 does not allow.".to_string(),
            )
        })?;

        let mut salted = [0u8; 32];
        pbkdf2::derive(
            pbkdf2::PBKDF2_HMAC_SHA256,
            NonZeroU32::new(iterations).unwrap_or(NonZeroU32::MIN),
            &salt,
            password.as_bytes(),
            &mut salted,
        );
        let salted_key = hmac::Key::new(hmac::HMAC_SHA256, &salted);
        let client_key = hmac::sign(&salted_key, b"Client Key");
        let stored_key = digest::digest(&digest::SHA256, client_key.as_ref());
        let without_proof = format!("c=biws,r={nonce}");
        let auth_message = format!("{},{server_first},{without_proof}", self.first_bare);
        let client_signature = hmac::sign(
            &hmac::Key::new(hmac::HMAC_SHA256, stored_key.as_ref()),
            auth_message.as_bytes(),
        );
        let proof: Vec<u8> = client_key
            .as_ref()
            .iter()
            .zip(client_signature.as_ref())
            .map(|(a, b)| a ^ b)
            .collect();
        let server_key = hmac::sign(&salted_key, b"Server Key");
        let server_signature = hmac::sign(
            &hmac::Key::new(hmac::HMAC_SHA256, server_key.as_ref()),
            auth_message.as_bytes(),
        );
        Ok(ScramFinal {
            message: format!("{without_proof},p={}", STANDARD.encode(proof)),
            server_signature: server_signature.as_ref().to_vec(),
        })
    }

    pub fn username(&self) -> &str {
        &self.username
    }
}

impl ScramFinal {
    /// Vérifie la signature du serveur : un faux serveur qui aurait accepté
    /// n'importe quelle preuve ne passe pas.
    pub fn verify(&self, server_final: &str) -> Result<(), ProbeError> {
        let fields = scram_fields(server_final);
        if let Some((_, error)) = fields.iter().find(|(k, _)| *k == 'e') {
            return Err(ProbeError::Auth(format!("MongoDB refused the login: {error}")));
        }
        let signature = fields
            .iter()
            .find(|(k, _)| *k == 'v')
            .and_then(|(_, v)| STANDARD.decode(v).ok())
            .ok_or_else(|| ProbeError::Protocol("SCRAM server signature missing".to_string()))?;
        if signature != self.server_signature {
            return Err(ProbeError::Protocol(
                "SCRAM server signature does not match: this is not the server holding the \
                 account."
                    .to_string(),
            ));
        }
        Ok(())
    }
}

fn scram_fields(message: &str) -> Vec<(char, &str)> {
    message
        .split(',')
        .filter_map(|part| {
            let mut chars = part.chars();
            let key = chars.next()?;
            part.get(1..)?.strip_prefix('=').map(|value| (key, value))
        })
        .collect()
}

/// Déroule l'authentification sur la base `source` (en général `admin`).
pub async fn authenticate(
    wire: &mut Wire,
    source: &str,
    username: &str,
    password: &str,
) -> Result<(), ProbeError> {
    let scram = Scram::new(username, password)?;
    let start = Document::new()
        .with("saslStart", Bson::Int32(1))
        .with("mechanism", Bson::String("SCRAM-SHA-256".to_string()))
        .with("payload", Bson::Binary(scram.client_first().into_bytes()))
        .with("autoAuthorize", Bson::Int32(1))
        .with("options", Bson::Document(Document::new().with("skipEmptyExchange", Bson::Bool(true))));
    let reply = wire.command(source, start).await?;
    let (conversation, server_first) = sasl_step(&reply, scram.username())?;

    let last = scram.client_final(&server_first)?;
    let next = Document::new()
        .with("saslContinue", Bson::Int32(1))
        .with("conversationId", conversation.clone())
        .with("payload", Bson::Binary(last.message.clone().into_bytes()));
    let reply = wire.command(source, next).await?;
    let (_, server_final) = sasl_step(&reply, scram.username())?;
    last.verify(&server_final)?;

    // Sans `skipEmptyExchange` (serveurs anciens), un dernier tour vide.
    if reply.path("done") != Some(&Bson::Bool(true)) {
        let empty = Document::new()
            .with("saslContinue", Bson::Int32(1))
            .with("conversationId", conversation)
            .with("payload", Bson::Binary(Vec::new()));
        let reply = wire.command(source, empty).await?;
        sasl_step(&reply, scram.username())?;
    }
    Ok(())
}

fn sasl_step(reply: &Document, username: &str) -> Result<(Bson, String), ProbeError> {
    if !is_ok(reply) {
        let (code, message) = error_of(reply);
        return Err(match code {
            // AuthenticationFailed, ou mécanisme absent du compte.
            18 | 334 => ProbeError::Auth(format!(
                "MongoDB refused the login of \"{username}\" with SCRAM-SHA-256 ({message}). \
                 Check the password and the authentication database."
            )),
            _ => ProbeError::Auth(format!("MongoDB refused the login: {message}")),
        });
    }
    let conversation = reply.get("conversationId").cloned().unwrap_or(Bson::Int32(1));
    let payload = match reply.get("payload") {
        Some(Bson::Binary(bytes)) => String::from_utf8_lossy(bytes).into_owned(),
        Some(Bson::String(text)) => text.clone(),
        _ => String::new(),
    };
    Ok((conversation, payload))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Vecteur de test de la RFC 7677, section 3.
    #[test]
    fn vecteur_de_la_rfc_7677() {
        let scram = Scram::with_nonce("user", "pencil", "rOprNGfwEbeRWgbNEkqO");
        assert_eq!(scram.client_first(), "n,,n=user,r=rOprNGfwEbeRWgbNEkqO");
        let server_first = "r=rOprNGfwEbeRWgbNEkqO%hvYDpWUa2RaTCAfuxFIlj)hNlF$k0,\
                            s=W22ZaJ0SNY7soEsUEjb6gQ==,i=4096";
        let last = scram.client_final(server_first).unwrap();
        assert_eq!(
            last.message,
            "c=biws,r=rOprNGfwEbeRWgbNEkqO%hvYDpWUa2RaTCAfuxFIlj)hNlF$k0,\
             p=dHzbZapWIk4jUhN+Ute9ytag9zjfMHgsqmmiz7AndVQ="
        );
        last.verify("v=6rriTRBi23WpRR/wtup+mMhUZUn/dB5nLTJRsjl95G4=").unwrap();
        assert!(last.verify("v=AAAA").is_err());
        assert!(matches!(last.verify("e=invalid-proof"), Err(ProbeError::Auth(_))));
    }

    #[test]
    fn un_serveur_qui_ne_prolonge_pas_le_nonce_est_refuse() {
        let scram = Scram::with_nonce("user", "pencil", "abc");
        assert!(scram.client_final("r=xyz123,s=QUJD,i=4096").is_err());
        assert!(scram.client_final("r=abc123,s=QUJD,i=10").is_err());
    }

    #[test]
    fn le_nom_d_utilisateur_est_echappe() {
        let scram = Scram::with_nonce("a,b=c", "x", "n");
        assert_eq!(scram.client_first(), "n,,n=a=2Cb=3Dc,r=n");
    }

    #[test]
    fn une_reponse_qui_n_est_pas_op_msg_est_refusee() {
        assert!(parse_reply(&[0, 0, 0, 0, 1]).is_err());
        let mut body = vec![0, 0, 0, 0, 0];
        body.extend(Document::new().with("ok", Bson::Double(1.0)).encode());
        assert!(is_ok(&parse_reply(&body).unwrap()));
    }
}
