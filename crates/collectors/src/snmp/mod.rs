//! Collecteur SNMP.
//!
//! Trois couches, indépendantes et testables séparément :
//!
//! * [`session`] ouvre la session UDP, traduit les identifiants et fournit le walk
//!   construit au-dessus de GETBULK — que `snmp2` ne propose pas ;
//! * [`profile`] décrit en YAML ce qu'il faut lire sur une famille d'équipements et
//!   choisit automatiquement le profil le plus spécifique ;
//! * [`collect`] applique un profil à une session et produit les échantillons.
//!
//! Le collecteur ne pose aucune étiquette d'identité et ne gère pas le délai global :
//! le registre s'en charge pour tous les collecteurs.

mod collect;
mod discovery;
#[cfg(test)]
mod fixture;
mod oid;
mod pattern;
mod profile;
mod session;
#[cfg(test)]
mod testutil;
mod value;

use std::sync::{Arc, LazyLock};
use std::time::Duration;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, ProbeError, Sample, Target};
use tracing::{debug, warn};

use oid::ObjectId;
use session::Session;

pub use discovery::{DiscoveredDevice, Identity, ScanOptions, scan_network};
pub use profile::{Catalog, Profile};
// Type d'un champ public de `ScanOptions` : il doit être nommable par l'appelant.
pub use session::CommunityVersion;

/// Les trois scalaires d'identification, lus à chaque auto-détection.
static SYS_DESCR: LazyLock<ObjectId> =
    LazyLock::new(|| ObjectId::new(vec![1, 3, 6, 1, 2, 1, 1, 1, 0]));
static SYS_OBJECT_ID: LazyLock<ObjectId> =
    LazyLock::new(|| ObjectId::new(vec![1, 3, 6, 1, 2, 1, 1, 2, 0]));
static SYS_NAME: LazyLock<ObjectId> =
    LazyLock::new(|| ObjectId::new(vec![1, 3, 6, 1, 2, 1, 1, 5, 0]));

/// Délai accordé à un aller-retour SNMP.
///
/// Nettement inférieur au délai global d'interrogation : un walk enchaîne plusieurs
/// requêtes, et il vaut mieux abandonner celle qui traîne que perdre le lot entier.
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(3);

/// Renvois accordés par défaut à une requête restée sans réponse.
const DEFAULT_RETRIES: u32 = 1;

/// Bornes du délai par requête déduit du budget d'une interrogation.
const MIN_REQUEST_TIMEOUT: Duration = Duration::from_secs(1);
const MAX_REQUEST_TIMEOUT: Duration = Duration::from_secs(3);

/// Plafond des renvois d'une même requête.
const MAX_RETRIES: u32 = 2;

/// Découpe le budget global d'une interrogation en délai par requête et renvois.
///
/// Le délai par requête vaut le quart du budget, entre une et trois secondes : assez
/// court pour qu'un datagramme perdu laisse le temps de renvoyer, assez long pour un
/// agent lent. Les renvois — deux au plus — sont ceux que le budget peut absorber
/// pour une même requête ; le délai global du registre reste le garde-fou final.
pub fn request_timing(budget: Duration) -> (Duration, u32) {
    let per_request = (budget / 4).clamp(MIN_REQUEST_TIMEOUT, MAX_REQUEST_TIMEOUT);
    let attempts = (budget.as_millis() / per_request.as_millis().max(1)) as u32;
    let retries = attempts.saturating_sub(1).min(MAX_RETRIES);
    (per_request, retries)
}

/// Lit l'identité (`sysDescr`, `sysName`, `sysObjectID`) d'un équipement sans rien
/// enregistrer : c'est le « tester la connexion » du formulaire d'ajout.
///
/// Le profil proposé est celui que l'auto-détection choisirait. Comme `discover`,
/// une seconde tentative en v1 est faite quand v2c reste muet avec une community.
pub async fn identify(
    target: &Target,
    request_timeout: Duration,
) -> Result<(Identity, Option<String>), ProbeError> {
    let version = CommunityVersion::from_tags(&target.tags);
    let first = async {
        let mut session =
            Session::open(&target.address, &target.credential, request_timeout, version).await?;
        discovery::read_identity(&mut session).await
    }
    .await;
    let identity = match first {
        Err(error)
            if error.means_down()
                && matches!(
                    target.credential,
                    dumbmonit_proto::Credential::SnmpCommunity { .. }
                )
                && version == CommunityVersion::V2c =>
        {
            let mut session = Session::open(
                &target.address,
                &target.credential,
                request_timeout,
                CommunityVersion::V1,
            )
            .await?;
            discovery::read_identity(&mut session).await?
        }
        other => other?,
    };
    if identity.sysdescr.is_none() && identity.sysobjectid.is_none() {
        return Err(ProbeError::Config(
            "the device answered but did not report its identity: check that the SNMP view \
             grants access to the \"system\" group"
                .to_string(),
        ));
    }
    let sysobjectid = identity.sysobjectid.as_ref().and_then(|raw| raw.parse::<ObjectId>().ok());
    let profile = profile::embedded()
        .select(sysobjectid.as_ref(), identity.sysdescr.as_deref())
        .map(|profile| profile.id.clone());
    Ok((identity, profile))
}

/// Le collecteur SNMP, à enregistrer dans le registre au démarrage.
pub struct SnmpCollector {
    catalog: Arc<Catalog>,
    request_timeout: Duration,
    retries: u32,
}

impl Default for SnmpCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl SnmpCollector {
    /// Construit le collecteur avec les profils livrés, embarqués dans le binaire.
    pub fn new() -> Self {
        Self {
            catalog: profile::embedded_shared().clone(),
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
            retries: DEFAULT_RETRIES,
        }
    }

    /// Fixe le délai d'un aller-retour, sans toucher au nombre de renvois.
    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    /// Règle délai par requête et renvois d'après le budget global d'une
    /// interrogation (voir [`request_timing`]).
    pub fn with_probe_budget(mut self, budget: Duration) -> Self {
        (self.request_timeout, self.retries) = request_timing(budget);
        self
    }

    /// Remplace les profils livrés, par exemple par ceux-ci complétés à
    /// l'exécution ([`Catalog::add_source`]).
    pub fn with_catalog(mut self, catalog: Arc<Catalog>) -> Self {
        self.catalog = catalog;
        self
    }

    /// Les profils disponibles, pour peupler la liste déroulante de l'interface.
    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    async fn open(&self, target: &Target) -> Result<Session, ProbeError> {
        Session::open(
            &target.address,
            &target.credential,
            self.request_timeout,
            CommunityVersion::from_tags(&target.tags),
        )
        .await
        .map(|session| session.with_retries(self.retries))
    }

    /// Choisit un profil d'après l'identité annoncée par l'équipement.
    async fn detect(&self, session: &mut Session) -> Result<Option<String>, ProbeError> {
        let identity = discovery::read_identity(session).await?;

        // L'agent a répondu, mais sans rien dire de lui : c'est la signature d'une vue
        // SNMP qui n'accorde pas l'accès au groupe `system`. Proposer malgré tout le
        // profil de repli masquerait le vrai problème derrière des séries vides.
        if identity.sysdescr.is_none() && identity.sysobjectid.is_none() {
            return Ok(None);
        }

        let sysobjectid =
            identity.sysobjectid.as_ref().and_then(|raw| raw.parse::<ObjectId>().ok());

        let chosen = self
            .catalog
            .select(sysobjectid.as_ref(), identity.sysdescr.as_deref())
            .map(|profile| profile.id.clone());

        debug!(
            sysobjectid = identity.sysobjectid.as_deref().unwrap_or("?"),
            profile = chosen.as_deref().unwrap_or("aucun"),
            "auto-détection SNMP"
        );
        Ok(chosen)
    }
}

#[async_trait]
impl Collector for SnmpCollector {
    fn kind(&self) -> &'static str {
        "snmp"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let mut session = self.open(target).await?;

        // Une cible n'a pas encore de profil au tout premier passage : plutôt que de
        // ne rien remonter, on détecte à la volée. Le résultat n'est pas persisté —
        // c'est le rôle de l'API, qui appelle `discover` — mais la collecte démarre.
        let profile_id = match target.profile_id.clone() {
            Some(id) => id,
            None => self.detect(&mut session).await?.ok_or_else(|| {
                ProbeError::Config(
                    "could not identify this device: check that the SNMP view grants \
                     access to the \"system\" group, or pick a profile manually"
                        .to_string(),
                )
            })?,
        };

        let metrics = self.catalog.resolve(&profile_id).map_err(ProbeError::Config)?;
        let now_ms = chrono::Utc::now().timestamp_millis();
        let samples = collect::collect(&mut session, &metrics, now_ms).await?;

        if samples.is_empty() {
            // Un profil qui ne produit rien est presque toujours un mauvais choix de
            // profil ou une vue SNMP trop restreinte ; c'est une erreur de
            // configuration, pas une indisponibilité.
            warn!(target = target.id, profile = profile_id, "profil SNMP sans aucune mesure");
            return Err(ProbeError::Config(format!(
                "profile \"{profile_id}\" produced no measurement: check that it matches \
                 this device and that the SNMP view allows it"
            )));
        }
        Ok(samples)
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let mut session = self.open(target).await?;
        match self.detect(&mut session).await {
            Ok(profile) => Ok(profile),
            // Beaucoup d'équipements anciens ignorent purement et simplement un
            // message v2c. Une seconde tentative en v1 évite à l'utilisateur d'avoir
            // à deviner qu'il doit poser l'étiquette « snmp_version ».
            Err(error)
                if error.means_down()
                    && matches!(
                        target.credential,
                        dumbmonit_proto::Credential::SnmpCommunity { .. }
                    )
                    && CommunityVersion::from_tags(&target.tags) == CommunityVersion::V2c =>
            {
                debug!(target = target.id, "aucune réponse en v2c, seconde tentative en v1");
                let mut session = Session::open(
                    &target.address,
                    &target.credential,
                    self.request_timeout,
                    CommunityVersion::V1,
                )
                .await?
                .with_retries(self.retries);
                let profile = self.detect(&mut session).await?;
                warn!(
                    target = target.id,
                    "équipement joignable en SNMPv1 seulement : ajoutez l'étiquette \
                     « snmp_version = 1 » à la cible"
                );
                Ok(profile)
            }
            Err(error) => Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use dumbmonit_proto::Credential;

    use super::*;

    fn target(credential: Credential) -> Target {
        Target {
            id: 1,
            name: "sw1".to_string(),
            address: "192.0.2.1".to_string(),
            kind: "snmp".to_string(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            tags: BTreeMap::new(),
            credential,
            group_name: String::new(),
            position: 0,
        }
    }

    #[test]
    fn le_type_de_collecteur_est_celui_attendu_par_les_cibles() {
        assert_eq!(SnmpCollector::new().kind(), "snmp");
    }

    #[test]
    fn le_catalogue_livre_contient_les_profils_annonces() {
        let collector = SnmpCollector::new();
        let mut ids = collector.catalog().ids();
        ids.sort_unstable();
        assert_eq!(
            ids,
            vec![
                "ethernet-switch",
                "host-resources",
                "if-mib",
                "printer",
                "system",
                "ups",
                "zyxel"
            ]
        );
    }

    #[test]
    fn tous_les_profils_livres_declarent_des_oid_valides() {
        // Un OID mal écrit produirait un profil silencieusement vide en production ;
        // l'analyse le refuse, encore faut-il que le catalogue soit bien complet.
        for profile in SnmpCollector::new().catalog().profiles() {
            assert!(!profile.name.trim().is_empty(), "profil « {} » sans nom", profile.id);
            for metric in &profile.metrics {
                // LLDP-MIB est une MIB de l'IEEE, rangée sous iso.std.iso8802
                // (1.0.8802) et non sous l'arbre Internet.
                assert!(
                    metric.oid.arcs().starts_with(&[1, 3, 6, 1])
                        || metric.oid.arcs().starts_with(&[1, 0, 8802]),
                    "profil « {} », métrique « {} » : OID hors de l'arbre Internet",
                    profile.id,
                    metric.name
                );
                assert_eq!(
                    metric.walk,
                    metric.oid.arcs().last() != Some(&0),
                    "profil « {} », métrique « {} » : un OID terminé par « .0 » est une \
                     instance unique, pas une table",
                    profile.id,
                    metric.name
                );
            }
        }
    }

    #[test]
    fn une_cible_sans_identifiant_snmp_echoue_en_configuration() {
        let error = super::testutil::block_on_large_stack(async {
            SnmpCollector::new().probe(&target(Credential::None)).await.unwrap_err()
        });
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
        assert!(!error.means_down(), "{error}");
    }

    #[test]
    fn une_adresse_vide_echoue_en_configuration() {
        let error = super::testutil::block_on_large_stack(async {
            let mut target = target(Credential::SnmpCommunity { community: "public".to_string() });
            target.address = "  ".to_string();
            SnmpCollector::new().probe(&target).await.unwrap_err()
        });
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
    }

    #[test]
    fn un_equipement_muet_est_signale_comme_injoignable() {
        // 192.0.2.0/24 est réservé à la documentation (RFC 5737) : personne ne répond.
        let error = super::testutil::block_on_large_stack(async {
            let collector = SnmpCollector::new().with_request_timeout(Duration::from_millis(150));
            let target = target(Credential::SnmpCommunity { community: "s3cr3t".to_string() });
            collector.probe(&target).await.unwrap_err()
        });
        assert!(error.means_down(), "{error}");
        assert!(!error.to_string().contains("s3cr3t"), "la community a fuité : {error}");
    }

    #[test]
    fn le_budget_se_decoupe_en_requetes_courtes_avec_renvois() {
        let secs = Duration::from_secs;
        assert_eq!(request_timing(secs(10)), (Duration::from_millis(2500), 2));
        assert_eq!(request_timing(secs(30)), (secs(3), 2));
        assert_eq!(request_timing(secs(2)), (secs(1), 1));
        assert_eq!(request_timing(secs(1)), (secs(1), 0));
        // Jamais un délai par requête égal au budget entier dès qu'il le permet.
        let (per_request, retries) = request_timing(secs(10));
        assert!(per_request * (retries + 1) <= secs(10));
    }
}
