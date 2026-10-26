//! Sonde UDP (`kind = "udp"`) : envoie un datagramme et, au choix, vérifie la
//! réponse ou l'absence de refus.
//!
//! UDP n'a pas de connexion : « le port est ouvert » n'a de sens que si le
//! service répond, ou, faute de réponse, que personne n'a dit « fermé ».
//!
//! * **Avec `expect_response`** : le service doit répondre, et la réponse doit
//!   contenir la sous-chaîne (comparaison sur le texte, octets invalides
//!   remplacés). Pas de réponse : `timeout` ; autre contenu : `payload`.
//! * **Sans** : la sonde réussit si aucun refus ne revient dans le délai de
//!   grâce ([`GRACE`]). La socket est « connectée » pour que le noyau relaie le
//!   message ICMP *port unreachable* sous forme de `ConnectionRefused`. C'est
//!   un test négatif : un pare-feu qui jette les paquets sans rien dire passe
//!   pour un service en vie, ce qu'aucune sonde UDP muette ne peut distinguer.
//!
//! Il n'y a pas de forme « expression régulière » : la crate `regex` n'est pas
//! une dépendance des collecteurs (ni de l'agent relais), et la sous-chaîne
//! couvre les réponses de service (`pong`, un code de statut).
//!
//! # Adresse et étiquettes
//!
//! Adresse : `hôte:port`. Le port est **obligatoire**.
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `port` | — | Port, si l'adresse n'en précise pas. |
//! | `payload` | vide | Datagramme envoyé (1400 octets au plus). |
//! | `payload_encoding` | `text` | `text` (échappements `\n \r \t \0 \\ \xNN`) ou `hex`. |
//! | `expect_response` | — | Sous-chaîne que la réponse doit contenir. |
//! | `allow_private_targets` | `false` | Autorise la boucle locale (voir `guard`). |
//! | `timeout_seconds` | `5` | Délai propre à la sonde (1 à 60). |

pub(crate) mod options;

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use dumbmonit_proto::{Collector, ProbeError, Sample, Target};
use tokio::net::UdpSocket;
use tracing::debug;

use super::guard;
use super::outcome::{Failure, Report};
use options::Options;

/// Attente maximale d'un refus ICMP quand aucune réponse n'est attendue.
const GRACE: Duration = Duration::from_millis(1_500);

/// Taille du tampon de réception.
const MAX_RESPONSE_BYTES: usize = 4_096;

/// Collecteur de disponibilité d'un service UDP.
#[derive(Default)]
pub struct UdpCollector;

impl UdpCollector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Collector for UdpCollector {
    fn kind(&self) -> &'static str {
        "udp"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let options = Options::from_target(target)?;
        let mut report = Report::new(self.kind()).label("port", options.port.to_string());

        // Seul le garde-fou interrompt la sonde : c'est une erreur de configuration.
        exchange(&mut report, &options).await?;

        if let Some(detail) = report.detail() {
            debug!(target_id = target.id, host = %options.host, detail, "sonde UDP en échec");
        }
        Ok(report.finish())
    }
}

async fn exchange(report: &mut Report, options: &Options) -> Result<(), ProbeError> {
    let started = Instant::now();

    let resolved = tokio::time::timeout(
        options.timeout,
        tokio::net::lookup_host((options.host.as_str(), options.port)),
    )
    .await;
    let addresses: Vec<SocketAddr> = match resolved {
        Err(_) => {
            report.fail(Failure::Timeout, "name resolution too slow");
            return Ok(());
        }
        Ok(Err(error)) => {
            report.fail(Failure::Dns, format!("{}: {error}", options.host));
            return Ok(());
        }
        Ok(Ok(addresses)) => addresses.collect(),
    };
    // Avant toute socket, y compris UDP : un datagramme vers la boucle locale est
    // aussi une requête que l'hôte ne devrait pas pouvoir émettre à la demande.
    guard::vet(&options.host, &addresses, options.allow_private)?;
    let Some(address) = addresses.first().copied() else {
        report.fail(Failure::Dns, format!("{} resolves to no address", options.host));
        return Ok(());
    };

    let bind_address = if address.is_ipv6() { "[::]:0" } else { "0.0.0.0:0" };
    let socket = match UdpSocket::bind(bind_address).await {
        Ok(socket) => socket,
        Err(error) => {
            report.fail(Failure::Connect, format!("cannot open a UDP socket: {error}"));
            return Ok(());
        }
    };
    // « Connecter » une socket UDP ne parle pas au réseau : cela fixe le pair, et
    // fait remonter les erreurs ICMP et filtre les datagrammes d'autres sources.
    if let Err(error) = socket.connect(address).await {
        report.fail(Failure::Connect, format!("{address}: {error}"));
        return Ok(());
    }

    let remaining = options.timeout.saturating_sub(started.elapsed());
    let wait = if options.expect_response.is_some() { remaining } else { remaining.min(GRACE) };

    let sent_at = Instant::now();
    let roundtrip = async {
        socket.send(&options.payload).await?;
        let mut buf = vec![0u8; MAX_RESPONSE_BYTES];
        let read = socket.recv(&mut buf).await?;
        buf.truncate(read);
        Ok::<_, std::io::Error>(buf)
    };
    let outcome = tokio::time::timeout(wait, roundtrip).await;

    let Some(expected) = &options.expect_response else {
        match outcome {
            // Pas de réponse dans le délai de grâce, ou une réponse : rien n'a refusé.
            Err(_) => {}
            Ok(Ok(response)) => record_response(report, &response, sent_at),
            Ok(Err(error)) => report.fail(Failure::Connect, format!("{address}: {error}")),
        }
        return Ok(());
    };

    match outcome {
        Err(_) => report.fail(Failure::Timeout, format!("{address}: no reply")),
        Ok(Err(error)) => report.fail(Failure::Connect, format!("{address}: {error}")),
        Ok(Ok(response)) => {
            record_response(report, &response, sent_at);
            let text = String::from_utf8_lossy(&response);
            if !text.contains(expected.as_str()) {
                report.fail(
                    Failure::Payload,
                    format!("the reply does not contain \"{expected}\" ({} bytes)", response.len()),
                );
            }
        }
    }
    Ok(())
}

fn record_response(report: &mut Report, response: &[u8], sent_at: Instant) {
    report.gauge("udp_response_seconds", sent_at.elapsed().as_secs_f64());
    report.gauge("udp_response_bytes", response.len() as f64);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uptime::tags::test_support::cible;

    fn metric(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    fn reason(samples: &[Sample]) -> Option<String> {
        samples
            .iter()
            .find(|s| s.metric == "probe_failure_info")
            .and_then(|s| s.labels.get("reason").cloned())
    }

    /// Serveur qui répond `reponse` à chaque datagramme.
    async fn faux_serveur(reponse: &'static [u8]) -> u16 {
        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let port = socket.local_addr().unwrap().port();
        tokio::spawn(async move {
            let mut buf = [0u8; 512];
            while let Ok((_, from)) = socket.recv_from(&mut buf).await {
                let _ = socket.send_to(reponse, from).await;
            }
        });
        port
    }

    fn cible_locale(port: u16, tags: &[(&str, &str)]) -> Target {
        let mut all = vec![("timeout_seconds", "2"), ("allow_private_targets", "true")];
        all.extend_from_slice(tags);
        cible("udp", &format!("127.0.0.1:{port}"), &all)
    }

    #[test]
    fn le_collecteur_annonce_son_type() {
        assert_eq!(UdpCollector::new().kind(), "udp");
    }

    #[tokio::test]
    async fn la_boucle_locale_est_refusee_sans_loption() {
        let target = cible("udp", "127.0.0.1:5353", &[("timeout_seconds", "2")]);
        let error = UdpCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
        assert!(!error.means_down());
        assert!(error.to_string().contains("allow_private_targets"), "{error}");
    }

    #[tokio::test]
    async fn la_reponse_attendue_presente_reussit() {
        let port = faux_serveur(b"+PONG status=ok").await;
        let target = cible_locale(port, &[("payload", "PING"), ("expect_response", "PONG")]);
        let samples = UdpCollector::new().probe(&target).await.unwrap();
        assert_eq!(metric(&samples, "probe_success"), Some(1.0));
        assert_eq!(metric(&samples, "probe_udp_response_bytes"), Some(15.0));
        assert!(metric(&samples, "probe_udp_response_seconds").is_some());
    }

    #[tokio::test]
    async fn une_reponse_differente_echoue_en_payload() {
        let port = faux_serveur(b"-ERR").await;
        let target = cible_locale(port, &[("payload", "PING"), ("expect_response", "PONG")]);
        let samples = UdpCollector::new().probe(&target).await.unwrap();
        assert_eq!(metric(&samples, "probe_success"), Some(0.0));
        assert_eq!(reason(&samples).as_deref(), Some("payload"));
    }

    #[tokio::test]
    async fn un_service_muet_avec_reponse_attendue_echoue_en_timeout() {
        let muet = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let port = muet.local_addr().unwrap().port();
        let target = cible_locale(
            port,
            &[("expect_response", "x"), ("timeout_seconds", "1"), ("payload", "?")],
        );
        let samples = UdpCollector::new().probe(&target).await.unwrap();
        assert_eq!(reason(&samples).as_deref(), Some("timeout"));
        drop(muet);
    }

    #[tokio::test]
    async fn sans_reponse_attendue_le_silence_est_un_succes() {
        let muet = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let port = muet.local_addr().unwrap().port();
        let samples = UdpCollector::new().probe(&cible_locale(port, &[])).await.unwrap();
        assert_eq!(metric(&samples, "probe_success"), Some(1.0));
        drop(muet);
    }

    /// Port fermé : le noyau renvoie un ICMP *port unreachable*, vu comme un refus.
    #[tokio::test]
    async fn un_port_ferme_est_un_echec_mesure() {
        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let port = socket.local_addr().unwrap().port();
        drop(socket);
        let target = cible_locale(port, &[("payload", "hello")]);
        let samples = UdpCollector::new().probe(&target).await.expect("une mesure");
        assert_eq!(metric(&samples, "probe_success"), Some(0.0));
        assert_eq!(reason(&samples).as_deref(), Some("connect"));
    }

    #[tokio::test]
    async fn le_payload_hexadecimal_part_tel_quel() {
        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let port = socket.local_addr().unwrap().port();
        let recu = tokio::spawn(async move {
            let mut buf = [0u8; 16];
            let (read, from) = socket.recv_from(&mut buf).await.unwrap();
            socket.send_to(b"ok", from).await.unwrap();
            buf[..read].to_vec()
        });
        let target = cible_locale(
            port,
            &[("payload", "de ad be ef"), ("payload_encoding", "hex"), ("expect_response", "ok")],
        );
        let samples = UdpCollector::new().probe(&target).await.unwrap();
        assert_eq!(metric(&samples, "probe_success"), Some(1.0));
        assert_eq!(recu.await.unwrap(), vec![0xde, 0xad, 0xbe, 0xef]);
    }
}
