//! Sonde NTP (`kind = "ntp"`) : interroge un serveur de temps en SNTP (RFC
//! 4330), sans dépendance au-delà d'une socket UDP — le paquet de 48 octets
//! s'encode et se décode à la main dans [`packet`].
//!
//! # Ce que mesure la sonde
//!
//! Quatre horodatages — émission (T1), réception par le serveur (T2),
//! émission par le serveur (T3), réception (T4) — donnent l'écart d'horloge et
//! le délai aller-retour (RFC 5905 §8) :
//!
//! ```text
//! offset = ((T2 - T1) + (T3 - T4)) / 2
//! delay  = (T4 - T1) - (T3 - T2)
//! ```
//!
//! La sonde échoue (`probe_success = 0`) quand le serveur n'est pas
//! synchronisé (stratum 16, ou le « kiss-o'-death » du stratum 0) ou quand
//! l'écart dépasse le seuil toléré : c'est le même principe que la perte de
//! paquets du ping, un dépassement de seuil propre à la sonde plutôt qu'une
//! erreur de configuration.
//!
//! # Adresse et étiquettes
//!
//! Adresse : le nom ou l'adresse IP du serveur NTP.
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `port` | `123` | Port UDP du serveur. |
//! | `offset_threshold_ms` | `100` | Écart au-delà duquel la sonde échoue, en millisecondes. |
//! | `allow_private_targets` | `false` | Autorise la boucle locale et le lien local (voir `guard`). |
//! | `timeout_seconds` | `5` | Délai propre à la sonde (1 à 60). |

pub(crate) mod options;
pub mod packet;

use std::net::SocketAddr;
use std::time::Instant;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, ProbeError, Sample, Target};
use tokio::net::UdpSocket;
use tracing::debug;

use super::guard;
use super::outcome::{Failure, Report};
use options::Options;
use packet::{Response, is_kiss_of_death, is_unsynchronized, reference_id_string};

/// Collecteur de disponibilité par requête SNTP.
#[derive(Default)]
pub struct NtpCollector;

impl NtpCollector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Collector for NtpCollector {
    fn kind(&self) -> &'static str {
        "ntp"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let options = Options::from_target(target)?;
        let mut report = Report::new(self.kind()).label("port", options.port.to_string());

        // Seule une adresse refusée par le garde-fou interrompt la sonde ici :
        // c'est une erreur de configuration, pas une mesure.
        query(&mut report, &options).await?;

        if let Some(detail) = report.detail() {
            debug!(target_id = target.id, host = %options.host, detail, "sonde NTP en échec");
        }
        Ok(report.finish())
    }
}

/// Résout l'hôte, envoie la requête, attend la réponse et remplit le rapport.
///
/// `Err` seulement pour le garde-fou ; tout le reste (résolution, délai,
/// réponse illisible, horloge désynchronisée, écart trop grand) se traduit en
/// `probe_success = 0` via `report.fail`.
async fn query(report: &mut Report, options: &Options) -> Result<(), ProbeError> {
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
    guard::vet(&options.host, &addresses, options.allow_private)?;
    let Some(address) = addresses.first().copied() else {
        report.fail(Failure::Dns, format!("{} resolves to no address", options.host));
        return Ok(());
    };

    let remaining = options.timeout.saturating_sub(started.elapsed());
    match exchange(address, remaining).await {
        Ok((response, t1, t4)) => record(report, options, &response, t1, t4),
        Err((reason, detail)) => report.fail(reason, detail),
    }
    Ok(())
}

/// Ouvre une socket UDP éphémère, envoie la requête et attend la réponse —
/// une seule tentative : un paquet perdu doit se voir comme une panne, pas se
/// cacher derrière une reprise automatique.
async fn exchange(
    address: SocketAddr,
    timeout: std::time::Duration,
) -> Result<(Response, f64, f64), (Failure, String)> {
    let bind_address = if address.is_ipv6() { "[::]:0" } else { "0.0.0.0:0" };
    let socket = UdpSocket::bind(bind_address)
        .await
        .map_err(|error| (Failure::Connect, format!("cannot open a UDP socket: {error}")))?;

    let t1 = unix_now();
    let request = packet::encode_request(t1);

    let roundtrip = async {
        socket.send_to(&request, address).await?;
        let mut buf = [0u8; 512];
        let (read, from) = socket.recv_from(&mut buf).await?;
        Ok::<_, std::io::Error>((read, from, buf))
    };

    match tokio::time::timeout(timeout, roundtrip).await {
        Err(_) => Err((Failure::Timeout, format!("{address}: no reply"))),
        Ok(Err(error)) => Err((Failure::Connect, format!("{address}: {error}"))),
        Ok(Ok((read, from, buf))) => {
            let t4 = unix_now();
            if from.ip() != address.ip() {
                return Err((
                    Failure::Protocol,
                    format!("reply from {from}, expected {address} (spoofed or misrouted)"),
                ));
            }
            let response = packet::decode_response(&buf[..read])
                .map_err(|detail| (Failure::Protocol, format!("{address}: {detail}")))?;
            Ok((response, t1, t4))
        }
    }
}

/// Calcule l'écart et le délai, publie les métriques et déclare l'échec s'il y
/// a lieu — désynchronisation d'abord, écart ensuite : la désynchronisation
/// explique tout, un écart mesuré sur une horloge qui ne l'est pas serait un
/// faux signal.
fn record(report: &mut Report, options: &Options, response: &Response, t1: f64, t4: f64) {
    let offset = ((response.receive_timestamp - t1) + (response.transmit_timestamp - t4)) / 2.0;
    let delay = (t4 - t1) - (response.transmit_timestamp - response.receive_timestamp);

    report.gauge("ntp_offset_seconds", offset);
    report.gauge("ntp_delay_seconds", delay.max(0.0));
    report.gauge("ntp_stratum", f64::from(response.stratum));
    report.gauge("ntp_leap_indicator", response.leap_indicator.as_value());
    report.gauge("ntp_root_delay_seconds", response.root_delay_s);
    report.gauge("ntp_root_dispersion_seconds", response.root_dispersion_s);
    report.gauge_with(
        "ntp_reference_info",
        1.0,
        "reference_id",
        &reference_id_string(response.stratum, response.reference_id),
    );

    if is_kiss_of_death(response.stratum) {
        report.fail(
            Failure::Unsynchronized,
            format!(
                "the server refused to answer (kiss-o'-death \"{}\")",
                reference_id_string(response.stratum, response.reference_id)
            ),
        );
    } else if is_unsynchronized(response.stratum, response.leap_indicator) {
        report.fail(
            Failure::Unsynchronized,
            format!("stratum {} or unsynchronized leap indicator", response.stratum),
        );
    } else if offset.abs() > options.offset_threshold_s {
        report.fail(
            Failure::ClockOffset,
            format!(
                "offset {:.1} ms exceeds the {:.1} ms threshold",
                offset * 1_000.0,
                options.offset_threshold_s * 1_000.0
            ),
        );
    }
}

/// L'heure locale, en secondes depuis l'époque Unix, fractionnaire.
fn unix_now() -> f64 {
    let now = chrono::Utc::now();
    now.timestamp() as f64 + f64::from(now.timestamp_subsec_nanos()) / 1_000_000_000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uptime::tags::test_support::cible;
    use packet::LeapIndicator;

    #[test]
    fn le_collecteur_annonce_son_type() {
        assert_eq!(NtpCollector::new().kind(), "ntp");
    }

    fn reponse(stratum: u8, leap: LeapIndicator) -> Response {
        Response {
            leap_indicator: leap,
            version: 4,
            mode: 4,
            stratum,
            poll: 6,
            precision: -20,
            root_delay_s: 0.01,
            root_dispersion_s: 0.01,
            reference_id: *b"GPS\0",
            reference_timestamp: 0.0,
            origin_timestamp: 0.0,
            receive_timestamp: 1_000.0,
            transmit_timestamp: 1_000.0,
        }
    }

    fn options(threshold_ms: f64) -> Options {
        let target = cible("ntp", "ntp.lan", &[("offset_threshold_ms", &threshold_ms.to_string())]);
        Options::from_target(&target).unwrap()
    }

    #[test]
    fn un_serveur_synchronise_dans_le_seuil_reussit() {
        let mut report = Report::new("ntp");
        // T1 = 999.999, T4 = 1000.001 : écart quasi nul, délai positif.
        record(
            &mut report,
            &options(100.0),
            &reponse(2, LeapIndicator::NoWarning),
            999.999,
            1_000.001,
        );
        assert!(report.is_up());
        let samples = report.finish();
        let offset = samples.iter().find(|s| s.metric == "probe_ntp_offset_seconds").unwrap();
        assert!(offset.value.abs() < 0.01, "{}", offset.value);
    }

    #[test]
    fn un_stratum_seize_echoue_meme_sans_ecart() {
        let mut report = Report::new("ntp");
        record(
            &mut report,
            &options(100.0),
            &reponse(16, LeapIndicator::NoWarning),
            1_000.0,
            1_000.0,
        );
        assert!(!report.is_up());
        assert_eq!(report.detail().map(|d| d.contains("stratum 16")), Some(true));
    }

    #[test]
    fn un_kiss_of_death_echoue_avec_le_code_en_detail() {
        let mut report = Report::new("ntp");
        let mut reponse = reponse(0, LeapIndicator::NoWarning);
        reponse.reference_id = *b"RATE";
        record(&mut report, &options(100.0), &reponse, 1_000.0, 1_000.0);
        assert!(!report.is_up());
        assert!(report.detail().unwrap().contains("RATE"));
    }

    #[test]
    fn un_indicateur_de_correction_desynchronise_echoue() {
        let mut report = Report::new("ntp");
        record(
            &mut report,
            &options(100.0),
            &reponse(2, LeapIndicator::Unsynchronized),
            1_000.0,
            1_000.0,
        );
        assert!(!report.is_up());
    }

    #[test]
    fn un_ecart_au_dela_du_seuil_echoue() {
        let mut report = Report::new("ntp");
        // T1 = 1000.0, T4 = 1000.0, mais le serveur répond avec receive/transmit
        // à 1000.5 : écart de 500 ms, bien au-delà du seuil de 100 ms.
        let mut reponse = reponse(2, LeapIndicator::NoWarning);
        reponse.receive_timestamp = 1_000.5;
        reponse.transmit_timestamp = 1_000.5;
        record(&mut report, &options(100.0), &reponse, 1_000.0, 1_000.0);
        assert!(!report.is_up());
        assert!(report.detail().unwrap().contains("exceeds"));
    }

    /// La boucle locale n'est joignable que depuis l'hôte de supervision : sans
    /// l'option, la sonde refuse avant tout échange réseau.
    #[tokio::test]
    async fn la_boucle_locale_est_refusee_sans_loption() {
        let target = cible("ntp", "127.0.0.1", &[("timeout_seconds", "2")]);
        let error = NtpCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
        assert!(!error.means_down());
        assert!(error.to_string().contains("allow_private_targets"), "{error}");
    }

    /// Rien n'écoute sur ce port : la requête part, personne ne répond, la
    /// sonde doit écrire un zéro plutôt que planter sur le délai expiré.
    #[tokio::test]
    async fn un_serveur_muet_produit_un_echec_mesure_et_non_une_erreur() {
        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let port = socket.local_addr().unwrap().port();
        drop(socket); // personne n'écoute plus sur ce port

        let target = cible(
            "ntp",
            &format!("127.0.0.1:{port}"),
            &[("timeout_seconds", "1"), ("allow_private_targets", "true")],
        );
        let samples = NtpCollector::new().probe(&target).await.expect("une mesure, pas une erreur");
        let success = samples.iter().find(|s| s.metric == "probe_success").expect("probe_success");
        assert_eq!(success.value, 0.0);
    }
}
