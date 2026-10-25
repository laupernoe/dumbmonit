//! Sonde RADIUS (`kind = "radius"`) : envoie un `Access-Request` (RFC 2865) avec
//! un compte de test et attend un `Access-Accept`.
//!
//! « Est-ce que mon portail Wi-Fi d'entreprise, mon VPN, mon switch 802.1X
//! peuvent encore authentifier quelqu'un ? » ne se voit pas à l'ouverture d'un
//! port UDP, qui ne répond jamais de toute façon. La sonde mène donc une vraie
//! authentification, avec un compte créé pour elle, et juge la réponse.
//!
//! # Verdict
//!
//! * `Access-Accept` : succès ;
//! * `Access-Reject` : le serveur fonctionne mais refuse le compte de test
//!   (`auth`) — compte verrouillé, mot de passe changé ; le `Reply-Message` du
//!   serveur, quand il en donne un, figure dans le détail ;
//! * `Access-Challenge` : le compte exige une seconde étape (OTP, EAP) qu'une
//!   sonde ne sait pas franchir (`protocol`) : prenez un compte sans double
//!   facteur ;
//! * réponse dont la signature ne se vérifie pas : le **secret partagé est
//!   probablement faux** (`bad_secret`). Ce n'est ni une panne réseau ni un refus
//!   de compte, et la raison le dit ;
//! * aucune réponse dans le délai : `timeout`. Beaucoup de serveurs se taisent
//!   quand le secret est faux ou quand l'adresse de la sonde n'est pas déclarée
//!   comme client RADIUS : c'est le premier endroit où chercher.
//!
//! Le mot de passe est masqué selon la RFC 2865 et la requête signée par un
//! `Message-Authenticator` (RFC 3579) placé en premier, comme le demande la parade
//! à « BlastRADIUS ». Une seule tentative, sans reprise : un datagramme perdu doit
//! se voir comme une panne.
//!
//! # Adresse, identifiants et étiquettes
//!
//! Adresse : le serveur RADIUS, `radius.lan` ou `radius.lan:1812`. Identifiant :
//! le secret partagé avec le serveur, et le nom et le mot de passe du compte de
//! test. Le secret et le mot de passe ne reviennent jamais par l'API.
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `port` | `1812` | Port UDP, si l'adresse n'en précise pas. |
//! | `nas_identifier` | `dumbmonit` | Valeur de `NAS-Identifier`. |
//! | `allow_private_targets` | `false` | Autorise la boucle locale (voir `guard`). |
//! | `timeout_seconds` | `5` | Délai propre à la sonde (1 à 60). |

#[cfg(test)]
mod fake;
pub(crate) mod options;
pub mod packet;

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use dumbmonit_proto::{Collector, ProbeError, Sample, Target};
use ring::rand::{SecureRandom, SystemRandom};
use tokio::net::UdpSocket;
use tracing::debug;

use super::guard;
use super::outcome::{Failure, Report};
use options::Options;
use packet::{
    ACCESS_ACCEPT, ACCESS_CHALLENGE, ACCESS_REJECT, DecodeError, Response, decode_response,
    encode_access_request,
};

/// Réponse lue, ou raison pour laquelle elle n'a pas été retenue.
type Outcome = Result<Response, DecodeError>;

/// Taille du tampon de réception : un paquet RADIUS ne dépasse pas 4 096 octets.
const RECEIVE_BUFFER: usize = 4096;

/// Collecteur de disponibilité par authentification RADIUS.
#[derive(Default)]
pub struct RadiusCollector;

impl RadiusCollector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Collector for RadiusCollector {
    fn kind(&self) -> &'static str {
        "radius"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let options = Options::from_target(target)?;
        let mut report = Report::new(self.kind()).label("port", options.port.to_string());

        // Seuls le garde-fou et la préparation de la requête interrompent la sonde
        // ici : ce sont des erreurs de configuration, pas des mesures.
        authenticate(&mut report, &options).await?;

        if let Some(detail) = report.detail() {
            debug!(target_id = target.id, host = %options.host, detail, "sonde RADIUS en échec");
        }
        Ok(report.finish())
    }
}

/// Résout l'hôte, envoie la requête, attend la réponse et remplit le rapport.
async fn authenticate(report: &mut Report, options: &Options) -> Result<(), ProbeError> {
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

    // Identifiant et authentificateur sont tirés au sort : l'authentificateur de
    // requête doit être imprévisible (RFC 2865 §3), il masque le mot de passe.
    let mut random = [0u8; 17];
    SystemRandom::new()
        .fill(&mut random)
        .map_err(|_| ProbeError::Config("no random source for the RADIUS request".to_string()))?;
    let id = random[0];
    let mut authenticator = [0u8; 16];
    authenticator.copy_from_slice(&random[1..]);

    let request = encode_access_request(
        id,
        &authenticator,
        options.shared_secret.as_bytes(),
        &options.username,
        &options.password,
        &options.nas_identifier,
    )
    .map_err(ProbeError::Config)?;

    let remaining = options.timeout.saturating_sub(started.elapsed());
    match exchange(address, &request, id, &authenticator, options, remaining).await {
        Ok((outcome, elapsed)) => judge(report, outcome, elapsed),
        Err((reason, detail)) => report.fail(reason, detail),
    }
    Ok(())
}

/// Envoie la requête et attend une réponse recevable.
///
/// Les datagrammes d'une autre source, ou portant l'identifiant d'une autre
/// requête, sont ignorés comme le veut la RFC 2865 : ils ne doivent ni faire
/// échouer la sonde ni lui faire croire que le serveur a répondu.
async fn exchange(
    address: SocketAddr,
    request: &[u8],
    id: u8,
    authenticator: &[u8; 16],
    options: &Options,
    timeout: Duration,
) -> Result<(Outcome, Duration), (Failure, String)> {
    let bind_address = if address.is_ipv6() { "[::]:0" } else { "0.0.0.0:0" };
    let socket = UdpSocket::bind(bind_address)
        .await
        .map_err(|error| (Failure::Connect, format!("cannot open a UDP socket: {error}")))?;

    let started = Instant::now();
    let roundtrip = async {
        socket.send_to(request, address).await?;
        let mut buffer = [0u8; RECEIVE_BUFFER];
        loop {
            let (read, from) = socket.recv_from(&mut buffer).await?;
            if from != address {
                continue;
            }
            let outcome = decode_response(
                &buffer[..read],
                authenticator,
                id,
                options.shared_secret.as_bytes(),
            );
            if matches!(outcome, Err(DecodeError::IdMismatch)) {
                continue;
            }
            return Ok::<_, std::io::Error>(outcome);
        }
    };

    match tokio::time::timeout(timeout, roundtrip).await {
        Err(_) => Err((
            Failure::Timeout,
            format!(
                "{address}: no reply. The server stays silent when the shared secret is wrong \
                 or when this host is not declared as a RADIUS client"
            ),
        )),
        Ok(Err(error)) => Err((Failure::Connect, format!("{address}: {error}"))),
        Ok(Ok(outcome)) => Ok((outcome, started.elapsed())),
    }
}

/// Traduit la réponse en métriques et en verdict.
fn judge(report: &mut Report, outcome: Outcome, elapsed: Duration) {
    let response = match outcome {
        Ok(response) => response,
        Err(DecodeError::BadAuthenticator(detail)) => {
            report
                .fail(Failure::BadSecret, format!("{detail}: the shared secret is probably wrong"));
            return;
        }
        Err(DecodeError::Malformed(detail)) => {
            report.fail(Failure::Protocol, format!("not a valid RADIUS reply: {detail}"));
            return;
        }
        Err(DecodeError::IdMismatch) => {
            report.fail(Failure::Protocol, "the reply answers another request");
            return;
        }
    };

    report.gauge("radius_response_seconds", elapsed.as_secs_f64());
    report.gauge("radius_response_code", f64::from(response.code));

    match response.code {
        ACCESS_ACCEPT => {}
        ACCESS_REJECT => {
            let reason = response.reply_message().map(|text| format!(" (\"{text}\")"));
            report.fail(
                Failure::Auth,
                format!(
                    "Access-Reject{}: the server refused the test account",
                    reason.unwrap_or_default()
                ),
            );
        }
        ACCESS_CHALLENGE => report.fail(
            Failure::Protocol,
            "Access-Challenge: the test account asks for a second step (one-time password, EAP) \
             that a probe cannot answer. Use an account without it",
        ),
        other => report.fail(
            Failure::Protocol,
            format!("unexpected RADIUS code {other} in reply to an Access-Request"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use dumbmonit_proto::Credential;

    use super::*;
    use crate::uptime::tags::test_support::cible;
    use fake::{Behaviour, FakeServer};

    const SECRET: &str = "testing123";

    fn target(address: &str, secret: &str, password: &str, tags: &[(&str, &str)]) -> Target {
        let mut all = vec![("allow_private_targets", "true"), ("timeout_seconds", "2")];
        all.extend_from_slice(tags);
        let mut target = cible("radius", address, &all);
        target.credential = Credential::Radius {
            shared_secret: secret.into(),
            username: "probe".into(),
            password: password.into(),
        };
        target
    }

    async fn probe(target: &Target) -> Vec<Sample> {
        RadiusCollector::new().probe(target).await.expect("une mesure, pas une erreur")
    }

    fn metric(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    fn reason(samples: &[Sample]) -> Option<String> {
        samples
            .iter()
            .find(|s| s.metric == "probe_failure_info")
            .and_then(|s| s.labels.get("reason").cloned())
    }

    #[test]
    fn le_collecteur_annonce_son_type() {
        assert_eq!(RadiusCollector::new().kind(), "radius");
    }

    #[tokio::test]
    async fn un_compte_accepte_reussit_et_publie_le_code() {
        let server = FakeServer::spawn(SECRET, "probe", "pw", Behaviour::Normal).await;
        let samples = probe(&target(&server.address, SECRET, "pw", &[])).await;
        assert_eq!(metric(&samples, "probe_success"), Some(1.0));
        assert_eq!(metric(&samples, "probe_radius_response_code"), Some(2.0));
        assert!(metric(&samples, "probe_radius_response_seconds").is_some());
        assert!(reason(&samples).is_none());
        let seen = server.last_request().expect("la requête a été reçue");
        assert_eq!(seen.username, "probe");
        assert_eq!(seen.password, "pw", "le mot de passe est démasqué avec le bon secret");
        assert_eq!(seen.nas_identifier, "dumbmonit");
        assert!(seen.signature_valid && seen.signature_first);
    }

    #[tokio::test]
    async fn un_mot_de_passe_refuse_se_distingue_dune_panne() {
        let server = FakeServer::spawn(SECRET, "probe", "pw", Behaviour::Normal).await;
        let samples = probe(&target(&server.address, SECRET, "mauvais", &[])).await;
        assert_eq!(metric(&samples, "probe_success"), Some(0.0));
        assert_eq!(reason(&samples).as_deref(), Some("auth"));
        assert_eq!(metric(&samples, "probe_radius_response_code"), Some(3.0));
    }

    #[tokio::test]
    async fn un_secret_faux_est_signale_comme_tel() {
        // Le serveur signe avec le sien : la signature ne se vérifie pas chez nous.
        let server = FakeServer::spawn("le-vrai-secret", "probe", "pw", Behaviour::Normal).await;
        let samples = probe(&target(&server.address, "un-autre-secret", "pw", &[])).await;
        assert_eq!(metric(&samples, "probe_success"), Some(0.0));
        assert_eq!(reason(&samples).as_deref(), Some("bad_secret"));
    }

    #[tokio::test]
    async fn un_defi_est_un_echec_de_protocole_lisible() {
        let server = FakeServer::spawn(SECRET, "probe", "pw", Behaviour::Challenge).await;
        let samples = probe(&target(&server.address, SECRET, "pw", &[])).await;
        assert_eq!(metric(&samples, "probe_success"), Some(0.0));
        assert_eq!(reason(&samples).as_deref(), Some("protocol"));
    }

    #[tokio::test]
    async fn un_serveur_muet_produit_un_echec_mesure_et_non_une_erreur() {
        let server = FakeServer::spawn(SECRET, "probe", "pw", Behaviour::Silent).await;
        let target = target(&server.address, SECRET, "pw", &[("timeout_seconds", "1")]);
        let samples = probe(&target).await;
        assert_eq!(metric(&samples, "probe_success"), Some(0.0));
        assert_eq!(reason(&samples).as_deref(), Some("timeout"));
    }

    /// Un datagramme d'une autre requête arrive avant la vraie réponse : il est
    /// ignoré, la sonde attend la bonne.
    #[tokio::test]
    async fn une_reponse_a_une_autre_requete_est_ignoree() {
        let server = FakeServer::spawn(SECRET, "probe", "pw", Behaviour::WrongIdFirst).await;
        let samples = probe(&target(&server.address, SECRET, "pw", &[])).await;
        assert_eq!(metric(&samples, "probe_success"), Some(1.0));
    }

    #[tokio::test]
    async fn un_paquet_qui_nest_pas_du_radius_est_un_echec_de_protocole() {
        let server = FakeServer::spawn(SECRET, "probe", "pw", Behaviour::Garbage).await;
        let samples = probe(&target(&server.address, SECRET, "pw", &[])).await;
        assert_eq!(metric(&samples, "probe_success"), Some(0.0));
        assert_eq!(reason(&samples).as_deref(), Some("protocol"));
    }

    #[tokio::test]
    async fn lidentifiant_de_nas_configure_part_dans_la_requete() {
        let server = FakeServer::spawn(SECRET, "probe", "pw", Behaviour::Normal).await;
        let target = target(&server.address, SECRET, "pw", &[("nas_identifier", "portail-1")]);
        probe(&target).await;
        assert_eq!(server.last_request().unwrap().nas_identifier, "portail-1");
    }

    #[tokio::test]
    async fn la_boucle_locale_est_refusee_sans_loption() {
        let mut target = cible("radius", "127.0.0.1", &[("timeout_seconds", "2")]);
        target.credential = Credential::Radius {
            shared_secret: SECRET.into(),
            username: "probe".into(),
            password: "pw".into(),
        };
        let error = RadiusCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
        assert!(!error.means_down());
        assert!(error.to_string().contains("allow_private_targets"), "{error}");
    }

    #[tokio::test]
    async fn sans_identifiant_radius_la_sonde_demande_den_fournir_un() {
        let target = cible("radius", "radius.lan", &[]);
        let error = RadiusCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
    }
}
