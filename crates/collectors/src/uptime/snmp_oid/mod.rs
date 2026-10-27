//! Sonde SNMP sur un OID (`kind = "snmp_oid"`) : lit un objet précis d'un agent
//! SNMP et en juge la valeur.
//!
//! Là où le collecteur `snmp` décrit un équipement entier par un profil, celle-ci
//! répond à une question étroite : « cet objet vaut-il ce que j'attends ? » — la
//! température d'une sonde, l'état d'un onduleur, une chaîne de version. Elle
//! réutilise sans rien dupliquer la session SNMP du collecteur `snmp` : mêmes
//! identifiants (community v1/v2c, utilisateur v3), même gestion des renvois et des
//! réponses tardives.
//!
//! # Verdict
//!
//! * l'agent ne répond pas dans le délai : panne (`timeout`) ;
//! * l'agent répond `noSuchObject`, `noSuchInstance` ou `noSuchName` : l'OID n'existe
//!   pas sur cet équipement (`not_found`) — presque toujours une faute de frappe ou
//!   une vue SNMP trop restreinte ;
//! * `min` / `max` : une valeur numérique hors des bornes échoue (`threshold`), une
//!   valeur non numérique aussi (`payload`) ;
//! * `expect` : la valeur, lue comme du texte, doit contenir ce fragment, sans tenir
//!   compte de la casse (`payload`).
//!
//! Sans aucune de ces trois options, la sonde réussit dès que l'objet existe.
//!
//! # Adresse et étiquettes
//!
//! Adresse : l'équipement, `switch.lan` ou `switch.lan:1161`. L'identifiant est une
//! community ou un utilisateur SNMP v3.
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `oid` | — | Objet à lire, obligatoire (`1.3.6.1.2.1.1.3.0`). |
//! | `port` | `161` | Port UDP, si l'adresse n'en précise pas. |
//! | `min` / `max` | — | Bornes tolérées (incluses) pour une valeur numérique. |
//! | `expect` | — | Fragment que la valeur texte doit contenir. |
//! | `snmp_version` | `2c` | `1` pour un agent resté en SNMPv1 (community seulement). |
//! | `allow_private_targets` | `false` | Autorise la boucle locale (voir `guard`). |
//! | `timeout_seconds` | `5` | Délai propre à la sonde (1 à 60). |

pub(crate) mod options;

use std::net::SocketAddr;
use std::time::Instant;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, ProbeError, Sample, Target};
use tracing::debug;

use super::guard;
use super::outcome::{Failure, Report};
use crate::snmp::request_timing;
use crate::snmp::session::Session;
use crate::snmp::value::SnmpValue;
use options::Options;

/// Collecteur de disponibilité par lecture d'un OID.
#[derive(Default)]
pub struct SnmpOidCollector;

impl SnmpOidCollector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Collector for SnmpOidCollector {
    fn kind(&self) -> &'static str {
        "snmp_oid"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let options = Options::from_target(target)?;
        let mut report = Report::new(self.kind())
            .label("port", options.port.to_string())
            .label("oid", options.oid.to_string());

        // Seule une adresse refusée par le garde-fou interrompt la sonde ici :
        // c'est une erreur de configuration, pas une mesure.
        read(&mut report, target, &options).await?;

        if let Some(detail) = report.detail() {
            debug!(target_id = target.id, host = %options.host, detail, "sonde SNMP OID en échec");
        }
        Ok(report.finish())
    }
}

/// Résout l'hôte, interroge l'agent et remplit le rapport.
///
/// `Err` seulement pour le garde-fou et la configuration ; le reste (agent muet,
/// OID inconnu, valeur hors bornes) se traduit en `probe_success = 0`.
async fn read(report: &mut Report, target: &Target, options: &Options) -> Result<(), ProbeError> {
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

    // La session reçoit l'adresse déjà vérifiée, sous forme littérale : elle ne
    // résout donc rien une seconde fois.
    let remaining = options.timeout.saturating_sub(started.elapsed());
    let (request_timeout, retries) = request_timing(remaining);
    let query = async {
        let mut session = Session::open(
            &address.to_string(),
            &target.credential,
            request_timeout,
            options.version,
        )
        .await?
        .with_retries(retries);
        let asked = Instant::now();
        let value = session.get_value(&options.oid).await?;
        Ok::<_, ProbeError>((value, asked.elapsed()))
    };

    match tokio::time::timeout(remaining, Box::pin(query)).await {
        Err(_) => report.fail(Failure::Timeout, format!("{address}: no SNMP reply")),
        Ok(Err(ProbeError::Config(detail))) => return Err(ProbeError::Config(detail)),
        Ok(Err(error)) => report.fail(failure_of(&error), error.to_string()),
        Ok(Ok((value, elapsed))) => {
            report.gauge("snmp_query_seconds", elapsed.as_secs_f64());
            judge(report, options, &value);
        }
    }
    Ok(())
}

/// Raison d'échec exposée pour une erreur de la session SNMP.
fn failure_of(error: &ProbeError) -> Failure {
    match error {
        ProbeError::Timeout(_) => Failure::Timeout,
        ProbeError::Auth(_) => Failure::Auth,
        ProbeError::Protocol(_) => Failure::Protocol,
        _ => Failure::Connect,
    }
}

/// Publie la valeur lue et rend le verdict des bornes et du texte attendu.
fn judge(report: &mut Report, options: &Options, value: &SnmpValue) {
    if value.is_absent() {
        report.fail(
            Failure::NotFound,
            format!(
                "the agent has no object {}: check the OID and that the SNMP view grants it",
                options.oid
            ),
        );
        return;
    }

    let number = value.as_f64();
    if let Some(number) = number {
        report.gauge("snmp_value", number);
    }

    if options.min.is_some() || options.max.is_some() {
        let Some(number) = number else {
            report.fail(
                Failure::Payload,
                format!(
                    "the value \"{}\" is not a number, so it cannot be compared with min/max",
                    text_of(value)
                ),
            );
            return;
        };
        if let Some(min) = options.min.filter(|min| number < *min) {
            report.fail(Failure::Threshold, format!("value {number} is below the minimum {min}"));
            return;
        }
        if let Some(max) = options.max.filter(|max| number > *max) {
            report.fail(Failure::Threshold, format!("value {number} is above the maximum {max}"));
            return;
        }
    }

    if let Some(expected) = &options.expect {
        let text = text_of(value);
        if !text.to_lowercase().contains(&expected.to_lowercase()) {
            report.fail(
                Failure::Payload,
                format!("the value \"{text}\" does not contain \"{expected}\""),
            );
        }
    }
}

/// La valeur lue comme du texte, tronquée pour le journal.
fn text_of(value: &SnmpValue) -> String {
    let text = match value {
        SnmpValue::Bytes(bytes) => String::from_utf8_lossy(bytes).trim().to_string(),
        other => other.as_label().unwrap_or_default(),
    };
    text.chars().take(200).collect()
}

#[cfg(test)]
mod tests {
    use dumbmonit_proto::Credential;

    use super::*;
    use crate::snmp::testutil::{FakeAgent, block_on_large_stack};
    use crate::uptime::tags::test_support::cible;

    const OID: &str = "1.3.6.1.4.1.9999.1.0";

    fn options(tags: &[(&str, &str)]) -> Options {
        let mut all = vec![("oid", OID)];
        all.extend_from_slice(tags);
        let mut target = cible("snmp_oid", "agent.lan", &all);
        target.credential = Credential::SnmpCommunity { community: "public".into() };
        Options::from_target(&target).unwrap()
    }

    fn verdict(tags: &[(&str, &str)], value: SnmpValue) -> Report {
        let mut report = Report::new("snmp_oid");
        judge(&mut report, &options(tags), &value);
        report
    }

    #[test]
    fn le_collecteur_annonce_son_type() {
        assert_eq!(SnmpOidCollector::new().kind(), "snmp_oid");
    }

    #[test]
    fn sans_option_un_objet_present_suffit() {
        assert!(verdict(&[], SnmpValue::Integer(3)).is_up());
        assert!(verdict(&[], SnmpValue::Bytes(b"v1.2".to_vec())).is_up());
    }

    #[test]
    fn un_objet_absent_echoue_avec_la_raison_not_found() {
        for absent in [SnmpValue::NoSuchObject, SnmpValue::NoSuchInstance] {
            let report = verdict(&[], absent);
            assert!(!report.is_up());
            assert!(report.detail().unwrap().contains(OID));
            let samples = report.finish();
            let info = samples.iter().find(|s| s.metric == "probe_failure_info").unwrap();
            assert_eq!(info.labels.get("reason").map(String::as_str), Some("not_found"));
        }
    }

    #[test]
    fn les_bornes_numeriques_sont_incluses() {
        let tags = [("min", "10"), ("max", "20")];
        assert!(verdict(&tags, SnmpValue::Integer(10)).is_up());
        assert!(verdict(&tags, SnmpValue::Integer(20)).is_up());
        let trop_bas = verdict(&tags, SnmpValue::Integer(9));
        assert!(!trop_bas.is_up());
        assert!(trop_bas.detail().unwrap().contains("below"));
        let trop_haut = verdict(&tags, SnmpValue::Unsigned(21));
        assert!(trop_haut.detail().unwrap().contains("above"));
    }

    #[test]
    fn une_valeur_numerique_est_publiee_meme_en_echec() {
        let samples = verdict(&[("max", "5")], SnmpValue::Integer(40)).finish();
        let value = samples.iter().find(|s| s.metric == "probe_snmp_value").unwrap();
        assert_eq!(value.value, 40.0);
    }

    #[test]
    fn une_chaine_numerique_est_lue_comme_un_nombre() {
        let value = SnmpValue::Bytes(b" 42,5 ".to_vec());
        assert!(verdict(&[("max", "50")], value.clone()).is_up());
        assert!(!verdict(&[("max", "40")], value).is_up());
    }

    #[test]
    fn une_valeur_non_numerique_ne_passe_pas_les_bornes() {
        let report = verdict(&[("min", "1")], SnmpValue::Bytes(b"ok".to_vec()));
        assert!(!report.is_up());
        assert!(report.detail().unwrap().contains("not a number"));
        let samples = report.finish();
        assert!(!samples.iter().any(|s| s.metric == "probe_snmp_value"));
    }

    #[test]
    fn le_texte_attendu_ignore_la_casse() {
        let value = SnmpValue::Bytes(b"UPS Online (battery 100%)".to_vec());
        assert!(verdict(&[("expect", "online")], value.clone()).is_up());
        let report = verdict(&[("expect", "on battery")], value);
        assert!(!report.is_up());
        assert!(report.detail().unwrap().contains("on battery"));
    }

    #[test]
    fn les_raisons_suivent_la_nature_de_lerreur() {
        let un_delai = ProbeError::Timeout(std::time::Duration::from_secs(1));
        assert_eq!(failure_of(&un_delai), Failure::Timeout);
        assert_eq!(failure_of(&ProbeError::Auth("x".into())), Failure::Auth);
        assert_eq!(failure_of(&ProbeError::Protocol("x".into())), Failure::Protocol);
        assert_eq!(failure_of(&ProbeError::Unreachable("x".into())), Failure::Connect);
    }

    #[test]
    fn la_boucle_locale_est_refusee_sans_loption() {
        block_on_large_stack(async {
            let mut target = cible("snmp_oid", "127.0.0.1", &[("oid", OID)]);
            target.credential = Credential::SnmpCommunity { community: "public".into() };
            let error = SnmpOidCollector::new().probe(&target).await.unwrap_err();
            assert!(matches!(error, ProbeError::Config(_)), "{error}");
            assert!(!error.means_down());
            assert!(error.to_string().contains("allow_private_targets"), "{error}");
        });
    }

    fn cible_locale(address: &str, tags: &[(&str, &str)]) -> Target {
        let mut all = vec![("oid", OID), ("snmp_version", "1"), ("allow_private_targets", "true")];
        all.extend_from_slice(tags);
        let mut target = cible("snmp_oid", address, &all);
        target.credential = Credential::SnmpCommunity { community: "public".into() };
        target
    }

    fn success(samples: &[Sample]) -> f64 {
        samples.iter().find(|s| s.metric == "probe_success").expect("probe_success").value
    }

    #[test]
    fn un_agent_factice_repond_et_la_valeur_est_publiee() {
        block_on_large_stack(async {
            let agent = FakeAgent::spawn(vec![(OID, 37)], 0).await;
            let target = cible_locale(&agent.address, &[("max", "50")]);
            let samples = SnmpOidCollector::new().probe(&target).await.unwrap();
            assert_eq!(success(&samples), 1.0);
            let value = samples.iter().find(|s| s.metric == "probe_snmp_value").unwrap();
            assert_eq!(value.value, 37.0);
            assert!(samples.iter().any(|s| s.metric == "probe_snmp_query_seconds"));
        });
    }

    #[test]
    fn un_agent_factice_hors_bornes_produit_un_echec_mesure() {
        block_on_large_stack(async {
            let agent = FakeAgent::spawn(vec![(OID, 80)], 0).await;
            let target = cible_locale(&agent.address, &[("max", "50")]);
            let samples = SnmpOidCollector::new().probe(&target).await.unwrap();
            assert_eq!(success(&samples), 0.0);
            let info = samples.iter().find(|s| s.metric == "probe_failure_info").unwrap();
            assert_eq!(info.labels.get("reason").map(String::as_str), Some("threshold"));
        });
    }

    #[test]
    fn un_oid_inconnu_de_lagent_echoue_en_not_found() {
        block_on_large_stack(async {
            let agent = FakeAgent::spawn(vec![("1.3.6.1.4.1.9999.2.0", 1)], 0).await;
            let target = cible_locale(&agent.address, &[]);
            let samples = SnmpOidCollector::new().probe(&target).await.unwrap();
            assert_eq!(success(&samples), 0.0);
            let info = samples.iter().find(|s| s.metric == "probe_failure_info").unwrap();
            assert_eq!(info.labels.get("reason").map(String::as_str), Some("not_found"));
        });
    }

    /// Personne n'écoute : la requête part, rien ne revient, la sonde écrit un
    /// zéro plutôt que de planter sur le délai.
    #[test]
    fn un_agent_muet_produit_un_echec_mesure_et_non_une_erreur() {
        block_on_large_stack(async {
            let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
            let address = socket.local_addr().unwrap().to_string();
            drop(socket);
            let target = cible_locale(&address, &[("timeout_seconds", "1")]);
            let samples = SnmpOidCollector::new().probe(&target).await.expect("une mesure");
            assert_eq!(success(&samples), 0.0);
        });
    }
}
