//! Sonde SSH (`kind = "ssh"`) : bannière du serveur et, au choix, empreinte de
//! sa clé d'hôte.
//!
//! Une sonde TCP sait que le port 22 s'ouvre ; elle ne sait pas que `sshd` a été
//! remplacé par autre chose, ni que la machine derrière l'adresse a été
//! réinstallée avec de nouvelles clés (ou pire, usurpée). Cette sonde lit la
//! bannière `SSH-2.0-…` — version du protocole et logiciel — puis, si on le lui
//! demande, mène le début de l'échange de clés pour obtenir la clé d'hôte.
//!
//! # L'échange de clés, et jusqu'où elle va
//!
//! Après les bannières, chaque côté envoie un `KEXINIT`. La sonde propose
//! `curve25519-sha256` et envoie aussitôt `KEX_ECDH_INIT` avec une clé X25519
//! éphémère ; le serveur répond `KEX_ECDH_REPLY`, dont le premier champ est sa
//! clé publique d'hôte (`K_S`). La sonde en calcule l'empreinte SHA-256 — la
//! même que `ssh-keygen -lf` — puis ferme la connexion. **Elle ne vérifie pas la
//! signature** et ne termine pas l'échange : elle observe la clé présentée, comme
//! le ferait `ssh-keyscan`, sans rien authentifier. Aucun identifiant n'est jamais
//! envoyé.
//!
//! Le serveur présente la clé du premier algorithme que le client propose et
//! qu'il possède : `ssh-ed25519`, puis `ecdsa-sha2-nistp256`, `rsa-sha2-512`,
//! `rsa-sha2-256`. L'empreinte attendue doit être celle de cette clé-là ;
//! `host_key_type` force une famille quand le serveur en a plusieurs.
//!
//! # Adresse et étiquettes
//!
//! Adresse : `hôte` ou `hôte:port`.
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `port` | `22` | Port, si l'adresse n'en précise pas. |
//! | `expected_host_key` | — | Empreinte `SHA256:…` attendue ; différente : échec `host_key`. |
//! | `alert_on_change` | `false` | Publie l'empreinte même sans empreinte attendue. |
//! | `host_key_type` | `auto` | `auto`, `ed25519`, `ecdsa` ou `rsa`. |
//! | `allow_private_targets` | `false` | Autorise la boucle locale (voir `guard`). |
//! | `timeout_seconds` | `5` | Délai propre à la sonde (1 à 60). |
//!
//! # Métriques propres
//!
//! * `probe_ssh_banner_seconds` : délai jusqu'à la bannière ;
//! * `probe_ssh_protocol_info{version}`, `probe_ssh_software_info{software}` ;
//! * `probe_ssh_kex_seconds` : durée de l'échange de clés (si demandé) ;
//! * `probe_ssh_host_key_fingerprint` : 32 bits de l'empreinte, comme
//!   `probe_dns_answer_fingerprint` — une règle « la valeur a changé » suffit ;
//! * `probe_ssh_host_key_info{fingerprint}` : l'empreinte lisible ;
//! * `probe_ssh_host_key_match` : 1 ou 0, si `expected_host_key` est renseignée.

pub(crate) mod options;
mod wire;

use std::time::Instant;

use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::STANDARD_NO_PAD as BASE64_NO_PAD;
use dumbmonit_proto::{Collector, ProbeError, Sample, Target};
use ring::agreement::{EphemeralPrivateKey, X25519};
use ring::digest::{SHA256, digest};
use ring::rand::SystemRandom;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tracing::debug;

use super::outcome::{Failure, Report};
use super::session::{self, Deadline, LineReader, SessionError};
use options::Options;
use wire::PacketReader;

/// Identification que la sonde annonce.
const CLIENT_BANNER: &str = "SSH-2.0-DumbMonit";

/// Taille maximale d'une ligne avant la bannière. La RFC limite la bannière à
/// 255 octets ; le reste est de la marge pour un message d'accueil.
const MAX_LINE_BYTES: usize = 1024;

/// Lignes de texte tolérées avant la bannière (la RFC les autorise).
const MAX_PREAMBLE_LINES: usize = 16;

/// Messages `IGNORE` ou `DEBUG` tolérés entre deux messages utiles.
const MAX_SKIPPED_MESSAGES: usize = 8;

/// Collecteur de disponibilité d'un serveur SSH.
#[derive(Default)]
pub struct SshCollector;

impl SshCollector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Collector for SshCollector {
    fn kind(&self) -> &'static str {
        "ssh"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let options = Options::from_target(target)?;
        let mut report = Report::new(self.kind()).label("port", options.port.to_string());

        converse(&mut report, &options).await?;

        if let Some(detail) = report.detail() {
            debug!(target_id = target.id, host = %options.host, port = options.port, detail,
                "sonde SSH en échec");
        }
        Ok(report.finish())
    }
}

/// `Err` seulement quand le garde-fou refuse l'adresse.
async fn converse(report: &mut Report, options: &Options) -> Result<(), ProbeError> {
    let deadline = Deadline::starting_now(options.timeout);

    let connected = match session::connect(
        &options.host,
        options.port,
        options.allow_private,
        deadline,
    )
    .await
    {
        Ok(connected) => connected,
        Err(SessionError::Refused(error)) => return Err(error),
        Err(error) => {
            let (reason, detail) = error.failure();
            report.fail(reason, detail);
            return Ok(());
        }
    };
    report.gauge("connect_seconds", connected.connect.as_secs_f64());
    let mut stream = connected.stream;

    let mut reader = LineReader::new(MAX_LINE_BYTES);
    let banner_started = Instant::now();
    let banner = match read_banner(&mut stream, &mut reader, deadline).await {
        Ok(banner) => banner,
        Err((reason, detail)) => {
            report.fail(reason, detail);
            return Ok(());
        }
    };
    report.gauge("ssh_banner_seconds", banner_started.elapsed().as_secs_f64());

    let Some((protocol, software)) = parse_banner(&banner) else {
        report.fail(Failure::Protocol, "malformed SSH identification string");
        return Ok(());
    };
    report.gauge_with("ssh_protocol_info", 1.0, "version", &protocol);
    report.gauge_with("ssh_software_info", 1.0, "software", &software);
    if protocol != "2.0" && protocol != "1.99" {
        report.fail(
            Failure::Protocol,
            format!("unsupported SSH protocol version \"{protocol}\" (2.0 expected)"),
        );
        return Ok(());
    }

    if !options.wants_host_key() {
        // Se présenter avant de raccrocher évite au serveur un journal de
        // « did not receive identification string ».
        let _ = send(&mut stream, format!("{CLIENT_BANNER}\r\n").as_bytes(), deadline).await;
        return Ok(());
    }

    let kex_started = Instant::now();
    match fetch_host_key(&mut stream, reader.take_buffered(), options, deadline).await {
        Ok(blob) => {
            report.gauge("ssh_kex_seconds", kex_started.elapsed().as_secs_f64());
            record_host_key(report, options, &blob);
        }
        Err((reason, detail)) => report.fail(reason, detail),
    }
    Ok(())
}

/// Lit la ligne d'identification, en passant les lignes de texte qui la précèdent.
async fn read_banner(
    stream: &mut TcpStream,
    reader: &mut LineReader,
    deadline: Deadline,
) -> Result<String, (Failure, String)> {
    for _ in 0..MAX_PREAMBLE_LINES {
        let line = reader.line(stream, deadline).await.map_err(|error| error.failure())?;
        if line.starts_with("SSH-") {
            return Ok(line);
        }
    }
    Err((Failure::Protocol, "no SSH identification string: the service does not speak SSH".into()))
}

/// Découpe `SSH-<protocole>-<logiciel>[ commentaire]`.
fn parse_banner(banner: &str) -> Option<(String, String)> {
    let rest = banner.strip_prefix("SSH-")?;
    let (protocol, rest) = rest.split_once('-')?;
    let software: String = rest
        .split(' ')
        .next()
        .unwrap_or("")
        .chars()
        .filter(char::is_ascii_graphic)
        .take(64)
        .collect();
    let protocol: String = protocol.chars().filter(char::is_ascii_graphic).take(8).collect();
    if protocol.is_empty() || software.is_empty() {
        return None;
    }
    Some((protocol, software))
}

type Failed = (Failure, String);

async fn send(stream: &mut TcpStream, bytes: &[u8], deadline: Deadline) -> Result<(), Failed> {
    match deadline.wait(stream.write_all(bytes)).await {
        Err(error) => Err(error.failure()),
        Ok(Err(error)) => Err((Failure::Connect, error.to_string())),
        Ok(Ok(())) => Ok(()),
    }
}

/// Attend le message `expected`, en passant `IGNORE` et `DEBUG`.
async fn next_message(
    packets: &mut PacketReader,
    stream: &mut TcpStream,
    expected: u8,
    deadline: Deadline,
) -> Result<Vec<u8>, Failed> {
    for _ in 0..=MAX_SKIPPED_MESSAGES {
        let payload = packets.packet(stream, deadline).await.map_err(|error| error.failure())?;
        match payload.first().copied() {
            Some(wire::MSG_IGNORE | wire::MSG_DEBUG) => continue,
            Some(wire::MSG_DISCONNECT) => {
                return Err((Failure::Protocol, wire::describe_disconnect(&payload)));
            }
            Some(kind) if kind == expected => return Ok(payload),
            Some(kind) => {
                return Err((
                    Failure::Protocol,
                    format!("unexpected SSH message {kind} during the key exchange"),
                ));
            }
            None => return Err((Failure::Protocol, "empty SSH packet".to_string())),
        }
    }
    Err((Failure::Protocol, "too many SSH_MSG_IGNORE / SSH_MSG_DEBUG messages".to_string()))
}

/// Mène l'échange de clés jusqu'à la réponse du serveur et en rend `K_S`.
async fn fetch_host_key(
    stream: &mut TcpStream,
    leftover: Vec<u8>,
    options: &Options,
    deadline: Deadline,
) -> Result<Vec<u8>, Failed> {
    let algorithms = options.host_key_type.algorithms();
    let kexinit = wire::client_kexinit(&algorithms).map_err(|detail| (Failure::Connect, detail))?;

    let random_failure = |_| (Failure::Connect, "no random source available".to_string());
    // La clé privée n'a pas à survivre aux `await` : seule la partie publique part.
    let public = {
        let private =
            EphemeralPrivateKey::generate(&X25519, &SystemRandom::new()).map_err(random_failure)?;
        private.compute_public_key().map_err(random_failure)?
    };

    let mut hello = format!("{CLIENT_BANNER}\r\n").into_bytes();
    hello.extend_from_slice(&wire::encode_packet(&kexinit));
    send(stream, &hello, deadline).await?;

    let mut packets = PacketReader::new(leftover);
    let server = next_message(&mut packets, stream, wire::MSG_KEXINIT, deadline).await?;
    let server = wire::parse_kexinit(&server).map_err(|detail| (Failure::Protocol, detail))?;

    if !server.kex_algorithms.iter().any(|name| name.starts_with("curve25519-sha256")) {
        return Err((
            Failure::Protocol,
            format!(
                "the server does not offer curve25519-sha256 (offered: {})",
                server.kex_algorithms.join(", ")
            ),
        ));
    }
    if !algorithms.iter().any(|name| server.host_key_algorithms.iter().any(|s| s == name)) {
        return Err((
            Failure::Protocol,
            format!(
                "no host key algorithm in common (asked: {}; offered: {})",
                algorithms.join(", "),
                server.host_key_algorithms.join(", ")
            ),
        ));
    }

    let init = wire::ecdh_init(public.as_ref());
    send(stream, &wire::encode_packet(&init), deadline).await?;

    let reply = next_message(&mut packets, stream, wire::MSG_KEX_ECDH_REPLY, deadline).await?;
    wire::parse_ecdh_reply(&reply).map_err(|detail| (Failure::Protocol, detail))
}

/// Empreinte au format OpenSSH, et ses 32 premiers bits pour la métrique.
fn fingerprint_of(blob: &[u8]) -> (String, u32) {
    let hash = digest(&SHA256, blob);
    let bytes = hash.as_ref();
    let short = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    (format!("SHA256:{}", BASE64_NO_PAD.encode(bytes)), short)
}

fn record_host_key(report: &mut Report, options: &Options, blob: &[u8]) {
    let (fingerprint, short) = fingerprint_of(blob);
    // Posées avant la comparaison : c'est justement quand la clé change que
    // l'attente échoue, et l'empreinte doit alors être écrite.
    report.gauge("ssh_host_key_fingerprint", f64::from(short));
    report.gauge_with("ssh_host_key_info", 1.0, "fingerprint", &fingerprint);

    let Some(expected) = &options.expected_host_key else { return };
    let matches = *expected == fingerprint;
    report.gauge("ssh_host_key_match", if matches { 1.0 } else { 0.0 });
    if !matches {
        let kind = wire::key_type_of(blob).unwrap_or_else(|| "unknown".to_string());
        report.fail(
            Failure::HostKey,
            format!(
                "the {kind} host key is {fingerprint}, expected {expected}. If the server has \
                 several keys, set \"host_key_type\" to the family of the expected one."
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use tokio::io::AsyncReadExt;
    use tokio::net::TcpListener;

    use super::*;
    use crate::uptime::tags::test_support::cible;

    fn metric(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    fn label(samples: &[Sample], name: &str, key: &str) -> Option<String> {
        samples.iter().find(|s| s.metric == name).and_then(|s| s.labels.get(key).cloned())
    }

    fn reason(samples: &[Sample]) -> Option<String> {
        label(samples, "probe_failure_info", "reason")
    }

    /// Un blob de clé ed25519 factice (le contenu n'est pas validé par la sonde).
    fn blob_factice(octet: u8) -> Vec<u8> {
        let mut blob = Vec::new();
        wire::put_string(&mut blob, b"ssh-ed25519");
        wire::put_string(&mut blob, &[octet; 32]);
        blob
    }

    fn kexinit_serveur(kex: &str, host_keys: &str) -> Vec<u8> {
        let mut payload = vec![wire::MSG_KEXINIT];
        payload.extend_from_slice(&[0u8; 16]);
        for list in [kex, host_keys, "aes128-ctr", "aes128-ctr", "hmac-sha2-256", "hmac-sha2-256"] {
            wire::put_string(&mut payload, list.as_bytes());
        }
        for _ in 0..4 {
            wire::put_string(&mut payload, b"");
        }
        payload.push(0);
        payload.extend_from_slice(&0u32.to_be_bytes());
        payload
    }

    /// Faux `sshd` : bannière et `KEXINIT` en un seul envoi (le cas courant), puis
    /// `KEX_ECDH_REPLY` portant `blob` si le client a mené l'échange.
    async fn faux_sshd(banner: &'static str, kex: &'static str, blob: Vec<u8>) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut hello = format!("{banner}\r\n").into_bytes();
            hello.extend_from_slice(&wire::encode_packet(&kexinit_serveur(kex, "ssh-ed25519")));
            socket.write_all(&hello).await.unwrap();

            // Bannière du client, octet par octet pour ne pas empiéter sur la suite.
            let mut ligne = Vec::new();
            let mut octet = [0u8; 1];
            while !ligne.ends_with(b"\n") {
                if socket.read(&mut octet).await.unwrap_or(0) == 0 {
                    return;
                }
                ligne.push(octet[0]);
            }
            assert!(ligne.starts_with(b"SSH-2.0-"));

            let deadline = Deadline::starting_now(std::time::Duration::from_secs(5));
            let mut packets = PacketReader::new(Vec::new());
            let Ok(client_kexinit) = packets.packet(&mut socket, deadline).await else { return };
            assert_eq!(client_kexinit[0], wire::MSG_KEXINIT);
            let Ok(init) = packets.packet(&mut socket, deadline).await else { return };
            assert_eq!(init[0], wire::MSG_KEX_ECDH_INIT);
            let mut cursor = wire::Cursor::new(&init[1..]);
            assert_eq!(cursor.string().unwrap().len(), 32, "clé publique X25519");

            let mut reply = vec![wire::MSG_KEX_ECDH_REPLY];
            wire::put_string(&mut reply, &blob);
            wire::put_string(&mut reply, &[0u8; 32]);
            wire::put_string(&mut reply, b"signature");
            let _ = socket.write_all(&wire::encode_packet(&reply)).await;
            let _ = socket.shutdown().await;
        });
        port
    }

    fn cible_locale(port: u16, tags: &[(&str, &str)]) -> Target {
        let mut all = vec![("timeout_seconds", "3"), ("allow_private_targets", "true")];
        all.extend_from_slice(tags);
        cible("ssh", &format!("127.0.0.1:{port}"), &all)
    }

    #[test]
    fn le_collecteur_annonce_son_type() {
        assert_eq!(SshCollector::new().kind(), "ssh");
    }

    #[test]
    fn la_banniere_est_decoupee() {
        assert_eq!(
            parse_banner("SSH-2.0-OpenSSH_9.6p1 Ubuntu-3ubuntu13"),
            Some(("2.0".to_string(), "OpenSSH_9.6p1".to_string()))
        );
        assert_eq!(
            parse_banner("SSH-1.99-dropbear_2024.85"),
            Some(("1.99".to_string(), "dropbear_2024.85".to_string()))
        );
        for mauvaise in ["SSH-2.0", "SSH--x", "HTTP/1.1 200 OK", "SSH-2.0- "] {
            assert_eq!(parse_banner(mauvaise), None, "{mauvaise}");
        }
    }

    #[test]
    fn lempreinte_suit_le_format_dopenssh() {
        // SHA-256 de la chaîne vide : 47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=
        let (empreinte, court) = fingerprint_of(b"");
        assert_eq!(empreinte, "SHA256:47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU");
        assert_eq!(court, 0xe3b0_c442);
    }

    #[tokio::test]
    async fn la_boucle_locale_est_refusee_sans_loption() {
        let target = cible("ssh", "127.0.0.1:22", &[("timeout_seconds", "2")]);
        let error = SshCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
        assert!(!error.means_down());
        assert!(error.to_string().contains("allow_private_targets"), "{error}");
    }

    #[tokio::test]
    async fn la_banniere_seule_publie_protocole_et_logiciel() {
        let banniere = "SSH-2.0-OpenSSH_9.6p1 Ubuntu";
        let port = faux_sshd(banniere, "curve25519-sha256", blob_factice(1)).await;
        let samples = SshCollector::new().probe(&cible_locale(port, &[])).await.unwrap();
        assert_eq!(metric(&samples, "probe_success"), Some(1.0));
        assert_eq!(label(&samples, "probe_ssh_protocol_info", "version").as_deref(), Some("2.0"));
        assert_eq!(
            label(&samples, "probe_ssh_software_info", "software").as_deref(),
            Some("OpenSSH_9.6p1")
        );
        assert!(metric(&samples, "probe_ssh_banner_seconds").is_some());
        assert!(metric(&samples, "probe_ssh_host_key_fingerprint").is_none());
    }

    #[tokio::test]
    async fn lempreinte_attendue_identique_reussit() {
        let blob = blob_factice(7);
        let (empreinte, court) = fingerprint_of(&blob);
        let port = faux_sshd("SSH-2.0-test", "curve25519-sha256@libssh.org", blob).await;
        let target = cible_locale(port, &[("expected_host_key", &empreinte)]);
        let samples = SshCollector::new().probe(&target).await.unwrap();
        assert_eq!(metric(&samples, "probe_success"), Some(1.0), "{:?}", reason(&samples));
        assert_eq!(metric(&samples, "probe_ssh_host_key_match"), Some(1.0));
        assert_eq!(metric(&samples, "probe_ssh_host_key_fingerprint"), Some(f64::from(court)));
        assert_eq!(
            label(&samples, "probe_ssh_host_key_info", "fingerprint").as_deref(),
            Some(empreinte.as_str())
        );
        assert!(metric(&samples, "probe_ssh_kex_seconds").is_some());
    }

    #[tokio::test]
    async fn une_cle_differente_echoue_avec_sa_raison_et_publie_la_nouvelle_empreinte() {
        let (attendue, _) = fingerprint_of(&blob_factice(1));
        let (presentee, court) = fingerprint_of(&blob_factice(2));
        let port = faux_sshd("SSH-2.0-test", "curve25519-sha256", blob_factice(2)).await;
        let target = cible_locale(port, &[("expected_host_key", &attendue)]);
        let samples = SshCollector::new().probe(&target).await.unwrap();
        assert_eq!(metric(&samples, "probe_success"), Some(0.0));
        assert_eq!(reason(&samples).as_deref(), Some("host_key"));
        assert_eq!(metric(&samples, "probe_ssh_host_key_match"), Some(0.0));
        assert_eq!(metric(&samples, "probe_ssh_host_key_fingerprint"), Some(f64::from(court)));
        assert_eq!(
            label(&samples, "probe_ssh_host_key_info", "fingerprint").as_deref(),
            Some(presentee.as_str())
        );
    }

    #[tokio::test]
    async fn alert_on_change_publie_lempreinte_sans_attente() {
        let port = faux_sshd("SSH-2.0-test", "curve25519-sha256", blob_factice(3)).await;
        let target = cible_locale(port, &[("alert_on_change", "true")]);
        let samples = SshCollector::new().probe(&target).await.unwrap();
        assert_eq!(metric(&samples, "probe_success"), Some(1.0));
        assert!(metric(&samples, "probe_ssh_host_key_fingerprint").is_some());
        assert!(metric(&samples, "probe_ssh_host_key_match").is_none());
    }

    #[tokio::test]
    async fn un_serveur_sans_curve25519_est_signale() {
        let kex = "diffie-hellman-group14-sha1";
        let port = faux_sshd("SSH-2.0-ancien", kex, blob_factice(1)).await;
        let target = cible_locale(port, &[("alert_on_change", "true")]);
        let samples = SshCollector::new().probe(&target).await.unwrap();
        assert_eq!(reason(&samples).as_deref(), Some("protocol"));
    }

    #[tokio::test]
    async fn un_service_qui_ne_parle_pas_ssh_est_signale() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let _ = socket.write_all(b"220 mail.exemple.fr ESMTP\r\n").await;
            let _ = socket.shutdown().await;
        });
        let samples = SshCollector::new().probe(&cible_locale(port, &[])).await.unwrap();
        assert_eq!(metric(&samples, "probe_success"), Some(0.0));
        assert!(reason(&samples).is_some());
    }

    #[tokio::test]
    async fn un_serveur_injoignable_produit_un_echantillon_a_zero() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let samples = SshCollector::new().probe(&cible_locale(port, &[])).await.unwrap();
        assert_eq!(metric(&samples, "probe_success"), Some(0.0));
        assert_eq!(reason(&samples).as_deref(), Some("connect"));
    }
}
