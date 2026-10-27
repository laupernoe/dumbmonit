//! Le déroulé d'une connexion SAML : départ vers l'IdP (`AuthnRequest` en
//! HTTP-Redirect), retour sur l'ACS (`Response` en HTTP-POST), puis ouverture
//! de la session sur une seconde navigation.
//!
//! Pourquoi deux étapes au retour : l'IdP renvoie le navigateur par un
//! formulaire POST venu d'un autre site, et un cookie `SameSite=Lax` n'y est pas
//! joint. L'ACS vérifie donc l'assertion, met l'identité de côté sous un ticket
//! à usage unique et redirige vers `/api/auth/saml/complete` : cette navigation
//! GET, elle, porte le cookie qui lie la tentative au navigateur qui l'a
//! lancée. Sans ce lien, un attaquant pourrait faire poster à sa victime une
//! réponse obtenue pour son propre compte et la connecter sous son identité.
//!
//! Comme pour OIDC, l'état en cours vit en mémoire : un processus unique suffit,
//! et une tentative interrompue par un redémarrage se recommence.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime};

use anyhow::anyhow;
use saml_rs::error::SubjectConfirmationReason;
use saml_rs::{
    AcsEndpoint, AssertionSignaturePolicy, AudienceValidationPolicy, AuthnRequest,
    AuthnRequestSigningPolicy, BrowserInput, ClockSkew, EntityId, FormField, IdpDescriptor,
    MetadataTrustPolicy, PendingAuthnRequest, PendingSnapshot, RelayStateParam, ReplayCache,
    ReplayKey, ReplayPolicy, Saml, SamlError, SamlValidationContext, Sp, SpConfig,
    SpValidationPolicy, SsoResponse, SsoSession, StartSso, VerifiedXmlSignatureCoverage,
    XmlSignatureProfile,
};
use sqlx::SqlitePool;
use tracing::{info, warn};

use super::{
    DEFAULT_EMAIL_ATTRIBUTES, DEFAULT_GROUP_ATTRIBUTES, DEFAULT_NAME_ATTRIBUTES, Endpoints,
    SamlConfig, TestHooks, certificate_base64, escape_xml,
};
use crate::auth::oidc::{pkce, roles};
use crate::auth::teams;
use crate::auth::users::{self, Role, User};

/// Délai laissé à l'utilisateur pour revenir de l'IdP.
pub const PENDING_TTL: Duration = Duration::from_secs(10 * 60);
/// Au-delà, les tentatives les plus anciennes sont abandonnées.
const PENDING_MAX: usize = 1000;
/// Entre l'ACS et `complete`, il n'y a qu'une redirection.
const TICKET_TTL: Duration = Duration::from_secs(2 * 60);
const TICKET_MAX: usize = 1000;
/// Identifiants d'assertions retenus contre le rejeu. Ils ne sont inscrits
/// qu'après une vérification complète : seul un IdP de confiance peut remplir
/// la table, au rythme des connexions réelles.
const REPLAY_MAX: usize = 100_000;
/// Tolérance d'horloge entre l'IdP et nous, dans les deux sens.
const CLOCK_SKEW_MS: i64 = 2 * 60 * 1000;
/// Rétention des identifiants dont l'assertion ne donne pas d'échéance.
const REPLAY_RETENTION: Duration = Duration::from_secs(60 * 60);

const NAMEID_TRANSIENT: &str = "urn:oasis:names:tc:SAML:2.0:nameid-format:transient";
const NAMEID_EMAIL: &str = "urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress";
const BINDING_REDIRECT: &str = "urn:oasis:names:tc:SAML:2.0:bindings:HTTP-Redirect";
const BINDING_POST: &str = "urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST";

/// Une connexion commencée, en attente de la réponse de l'IdP. Indexée par le
/// `RelayState` envoyé, que l'IdP renvoie tel quel.
#[derive(Debug, Clone)]
pub struct PendingRequest {
    /// `ID` de l'`AuthnRequest`, attendu en `InResponseTo`.
    pub request_id: String,
    pub idp_entity_id: String,
    /// ACS annoncé dans la requête : `Destination` et `Recipient` attendus.
    pub acs_url: String,
    /// Page interne demandée avant la connexion, à rouvrir ensuite.
    pub redirect: Option<String>,
    started_at: Instant,
}

impl PendingRequest {
    fn to_saml(&self, relay_state: &str) -> Result<PendingAuthnRequest, SamlError> {
        let snapshot = PendingSnapshot::<AuthnRequest>::authn_request(
            self.request_id.clone(),
            RelayStateParam::try_from_option(Some(relay_state.to_string()))?,
            self.idp_entity_id.clone(),
            "post",
            self.acs_url.clone(),
            "post",
        );
        PendingAuthnRequest::from_snapshot(snapshot)
    }
}

/// Ce qu'une assertion vérifiée dit de la personne.
#[derive(Debug, Clone)]
pub struct Identity {
    /// `entityID` de l'IdP émetteur.
    pub issuer: String,
    /// `NameID` brut, pour le journal.
    pub name_id: String,
    /// `NameID` stable, pour lier le compte ; `None` s'il est transitoire.
    pub subject: Option<String>,
    pub email: Option<String>,
    pub display_name: String,
    pub groups: Vec<String>,
}

/// Une identité vérifiée par l'ACS, en attente de la navigation `complete`.
#[derive(Debug, Clone)]
pub struct Ticket {
    pub identity: Identity,
    pub redirect: Option<String>,
    /// Empreinte attendue dans le cookie de liaison ; `None` pour une connexion
    /// initiée par l'IdP, qui n'a pas de navigateur de départ.
    pub binding: Option<String>,
    created_at: Instant,
}

impl Ticket {
    pub fn new(identity: Identity, redirect: Option<String>, binding: Option<String>) -> Self {
        Self { identity, redirect, binding, created_at: Instant::now() }
    }
}

/// Identifiants déjà acceptés, avec leur échéance.
#[derive(Debug, Default)]
pub struct ReplayStore {
    seen: HashMap<String, SystemTime>,
    /// Horloge de la validation en cours, pour purger ce qui a expiré.
    now: Option<SystemTime>,
}

impl ReplayCache for ReplayStore {
    fn check_and_store(&mut self, key: ReplayKey, expires_at: SystemTime) -> Result<(), SamlError> {
        if let Some(now) = self.now {
            self.seen.retain(|_, until| *until > now);
        }
        let key = key.cache_key();
        if self.seen.contains_key(&key) {
            return Err(SamlError::ReplayDetected { key });
        }
        if self.seen.len() >= REPLAY_MAX {
            // Évincer une entrée vivante rouvrirait un rejeu : mieux vaut refuser.
            return Err(SamlError::Invalid("replay cache is full".into()));
        }
        self.seen.insert(key, expires_at);
        Ok(())
    }
}

/// L'état SAML en mémoire, partagé par les requêtes (voir `AuthState`).
#[derive(Debug, Default)]
pub struct Memory {
    pending: Mutex<HashMap<String, PendingRequest>>,
    tickets: Mutex<HashMap<String, Ticket>>,
    replay: Mutex<ReplayStore>,
}

impl Memory {
    pub fn insert_pending(&self, relay_state: String, pending: PendingRequest) {
        let mut map = self.pending.lock().unwrap_or_else(PoisonError::into_inner);
        map.retain(|_, pending| pending.started_at.elapsed() < PENDING_TTL);
        if map.len() >= PENDING_MAX
            && let Some(oldest) =
                map.iter().min_by_key(|(_, pending)| pending.started_at).map(|(key, _)| key.clone())
        {
            map.remove(&oldest);
        }
        map.insert(relay_state, pending);
    }

    /// Retire et rend la tentative si elle existe et n'a pas expiré : un
    /// `RelayState` ne sert qu'une fois.
    pub fn take_pending(&self, relay_state: &str) -> Option<PendingRequest> {
        let pending =
            self.pending.lock().unwrap_or_else(PoisonError::into_inner).remove(relay_state)?;
        (pending.started_at.elapsed() < PENDING_TTL).then_some(pending)
    }

    pub fn insert_ticket(&self, id: String, ticket: Ticket) {
        let mut map = self.tickets.lock().unwrap_or_else(PoisonError::into_inner);
        map.retain(|_, ticket| ticket.created_at.elapsed() < TICKET_TTL);
        if map.len() >= TICKET_MAX
            && let Some(oldest) =
                map.iter().min_by_key(|(_, ticket)| ticket.created_at).map(|(key, _)| key.clone())
        {
            map.remove(&oldest);
        }
        map.insert(id, ticket);
    }

    pub fn take_ticket(&self, id: &str) -> Option<Ticket> {
        let ticket = self.tickets.lock().unwrap_or_else(PoisonError::into_inner).remove(id)?;
        (ticket.created_at.elapsed() < TICKET_TTL).then_some(ticket)
    }
}

/// Pourquoi une connexion a échoué. Le code court part dans `?reason=` de
/// l'écran de connexion ; le détail va dans le journal.
#[derive(Debug)]
pub enum FlowError {
    NotConfigured,
    /// Tentative inconnue, expirée, ou revenue dans un autre navigateur.
    State,
    /// Réponse non sollicitée alors que l'initiation par l'IdP est désactivée.
    Unsolicited,
    /// Réponse refusée par la vérification (signature, émetteur, audience,
    /// destination, `InResponseTo`, algorithme…).
    Invalid(String),
    /// Assertion hors de sa fenêtre de validité.
    Expired(String),
    /// Assertion déjà présentée.
    Replay,
    NoAccount,
    Disabled,
    Internal(anyhow::Error),
}

impl FlowError {
    pub fn reason(&self) -> &'static str {
        match self {
            Self::NotConfigured => "not_configured",
            Self::State => "state",
            Self::Unsolicited => "unsolicited",
            Self::Invalid(_) => "invalid_response",
            Self::Expired(_) => "expired",
            Self::Replay => "replay",
            Self::NoAccount => "no_account",
            Self::Disabled => "disabled",
            Self::Internal(_) => "internal",
        }
    }
}

impl From<anyhow::Error> for FlowError {
    fn from(error: anyhow::Error) -> Self {
        Self::Internal(error)
    }
}

/// Une erreur de `saml-rs` lors de la vérification d'une réponse.
fn rejected(error: SamlError) -> FlowError {
    match &error {
        SamlError::ReplayDetected { .. } => FlowError::Replay,
        SamlError::TimeWindowInvalid { .. }
        | SamlError::ExpiredSession
        | SamlError::SubjectConfirmationInvalid {
            reason: SubjectConfirmationReason::TimeWindowInvalid,
        } => FlowError::Expired(error.to_string()),
        _ => FlowError::Invalid(error.to_string()),
    }
}

/// Une erreur de `saml-rs` en préparant nos propres messages : la
/// configuration enregistrée est inutilisable.
fn setup(error: SamlError) -> FlowError {
    FlowError::Internal(anyhow!("SAML configuration unusable: {error}"))
}

/// Le fournisseur de service, avec la politique de vérification la plus
/// stricte de `saml-rs` :
///
/// - l'assertion consommée doit être couverte par une signature vérifiée :
///   la sienne, ou celle de la `Response` racine qui la contient (Keycloak, par
///   défaut, ne signe que le document). Seul le contenu couvert par la
///   signature est lu : une assertion non signée glissée à côté d'une assertion
///   signée, ou une réponse signée enveloppée dans une autre, est refusée
///   (XSW) ;
/// - profil `StrictRsaSha2` : une seule référence locale `#id`, RSA-SHA256/384/512
///   et condensé SHA-256 ou plus (SHA-1 refusé), canonicalisation exclusive,
///   transformations limitées à enveloped-signature et exc-c14n ;
/// - audience obligatoire et égale à notre `entityID` ;
/// - clés de vérification prises dans la configuration seulement, jamais
///   dans le `KeyInfo` de la réponse.
///
/// Les requêtes ne sont pas signées : aucun IdP courant ne l'exige, et DumbMonit
/// n'aurait qu'une clé de plus à garder.
pub fn service_provider(endpoints: &Endpoints) -> Result<Saml<Sp>, SamlError> {
    let policy = SpValidationPolicy {
        // En HTTP-POST, `saml-rs` exige toujours une signature de l'assertion
        // ou de la réponse ; `check_session` vérifie ensuite laquelle.
        assertions: AssertionSignaturePolicy::AllowUnsignedForCompatibility,
        xml_signatures: XmlSignatureProfile::StrictRsaSha2,
        audience: AudienceValidationPolicy::Validate,
        authn_requests: AuthnRequestSigningPolicy::DoNotSignForCompatibility,
        ..SpValidationPolicy::recommended()
    };
    let config = SpConfig::builder(EntityId::try_new(endpoints.entity_id.clone())?)
        .acs_endpoint(AcsEndpoint::post(endpoints.acs_url.clone())?)
        .validation(policy)
        .build()?;
    Saml::sp(config)
}

/// Les métadonnées de l'IdP, reconstruites à partir de la configuration : son
/// entité, son point d'entrée et ses seuls certificats de confiance.
pub fn idp_metadata_xml(config: &SamlConfig) -> String {
    let mut keys = String::new();
    for pem in &config.idp_certificates {
        if let Some(body) = certificate_base64(pem) {
            keys.push_str(&format!(
                "<md:KeyDescriptor use=\"signing\"><ds:KeyInfo><ds:X509Data>\
                 <ds:X509Certificate>{body}</ds:X509Certificate>\
                 </ds:X509Data></ds:KeyInfo></md:KeyDescriptor>"
            ));
        }
    }
    let entity = escape_xml(&config.idp_entity_id);
    let sso = escape_xml(&config.idp_sso_url);
    format!(
        "<md:EntityDescriptor xmlns:md=\"urn:oasis:names:tc:SAML:2.0:metadata\" \
         xmlns:ds=\"http://www.w3.org/2000/09/xmldsig#\" entityID=\"{entity}\">\
         <md:IDPSSODescriptor WantAuthnRequestsSigned=\"false\" \
         protocolSupportEnumeration=\"urn:oasis:names:tc:SAML:2.0:protocol\">{keys}\
         <md:SingleSignOnService Binding=\"{BINDING_REDIRECT}\" Location=\"{sso}\"/>\
         <md:SingleSignOnService Binding=\"{BINDING_POST}\" Location=\"{sso}\"/>\
         </md:IDPSSODescriptor></md:EntityDescriptor>"
    )
}

fn idp_descriptor(config: &SamlConfig) -> Result<IdpDescriptor, SamlError> {
    IdpDescriptor::from_metadata_xml_for(
        EntityId::try_new(config.idp_entity_id.clone())?,
        &idp_metadata_xml(config),
        // Ces métadonnées sont les nôtres, écrites à l'instant à partir de la
        // configuration de l'administrateur : il n'y a pas de signature à
        // vérifier, ce sont les certificats eux-mêmes qui font foi.
        MetadataTrustPolicy::UnsignedForCompatibility,
    )
}

/// Les métadonnées du fournisseur de service, à donner à l'IdP.
///
/// `WantAssertionsSigned` y reste à `true` : c'est ce que l'on demande à l'IdP
/// (et ce qu'un IdP qui importe ces métadonnées réglera), même si une réponse
/// signée seulement à la racine est acceptée. Les métadonnées du SP ne sont pas
/// signées : les retoucher n'invalide rien.
pub fn sp_metadata_xml(endpoints: &Endpoints) -> Result<String, FlowError> {
    Ok(service_provider(endpoints)
        .map_err(setup)?
        .metadata_xml()
        .replace("WantAssertionsSigned=\"false\"", "WantAssertionsSigned=\"true\""))
}

/// Construit la redirection vers l'IdP. Rend l'URL, le `RelayState` (clé de la
/// tentative, à lier au navigateur) et la tentative à mémoriser.
pub fn start(
    config: &SamlConfig,
    endpoints: &Endpoints,
    hooks: &TestHooks,
    redirect: Option<String>,
) -> Result<(String, String, PendingRequest), FlowError> {
    if !config.usable() {
        return Err(FlowError::NotConfigured);
    }
    let sp = service_provider(endpoints).map_err(setup)?;
    let idp = idp_descriptor(config).map_err(setup)?;
    let relay_state = pkce::random_token();
    let relay = RelayStateParam::try_from_option(Some(relay_state.clone())).map_err(setup)?;
    let started = sp.start_sso(&idp, StartSso::redirect().relay_state(relay)).map_err(setup)?;
    let url = started.outbound.redirect_url().map_err(setup)?.to_string();
    let request_id = hooks
        .request_id
        .clone()
        .unwrap_or_else(|| started.pending.request_id().as_str().to_string());
    let pending = PendingRequest {
        request_id,
        idp_entity_id: config.idp_entity_id.clone(),
        acs_url: endpoints.acs_url.clone(),
        redirect,
        started_at: Instant::now(),
    };
    Ok((url, relay_state, pending))
}

/// À quoi la réponse doit répondre.
pub enum Correlation<'a> {
    /// Une requête que nous avons envoyée, retrouvée par son `RelayState`.
    Solicited { pending: &'a PendingRequest, relay_state: &'a str },
    /// Une réponse sans requête (initiée par l'IdP).
    Unsolicited,
}

/// Vérifie la réponse postée sur l'ACS et en extrait l'identité.
///
/// Tout est synchrone : le cache anti-rejeu reste verrouillé le temps de la
/// vérification, sans jamais traverser d'`await`.
pub fn verify(
    config: &SamlConfig,
    endpoints: &Endpoints,
    correlation: Correlation<'_>,
    fields: Vec<(String, String)>,
    memory: &Memory,
    now: SystemTime,
) -> Result<Identity, FlowError> {
    if !config.usable() {
        return Err(FlowError::NotConfigured);
    }
    let sp = service_provider(endpoints).map_err(setup)?;
    let idp = idp_descriptor(config).map_err(setup)?;
    let input = BrowserInput::<SsoResponse>::post(
        fields.into_iter().map(|(name, value)| FormField::new(name, value)).collect(),
    );

    let unsolicited = matches!(correlation, Correlation::Unsolicited);
    let session = {
        let mut store = memory.replay.lock().unwrap_or_else(PoisonError::into_inner);
        store.now = Some(now);
        let validation = SamlValidationContext::new(now, ReplayPolicy::RequireCache(&mut *store))
            .with_clock_skew(ClockSkew::from_millis(-CLOCK_SKEW_MS, CLOCK_SKEW_MS))
            .with_replay_retention(REPLAY_RETENTION);
        match correlation {
            Correlation::Solicited { pending, relay_state } => {
                let pending = pending.to_saml(relay_state).map_err(setup)?;
                sp.finish_sso(&idp, &pending, input, validation)
            }
            Correlation::Unsolicited => sp.accept_unsolicited_sso(&idp, input, validation),
        }
        .map_err(rejected)?
    };

    check_session(config, &session, unsolicited)?;
    Ok(identity_of(config, &session))
}

/// Contrôles redondants avec ceux de `saml-rs`, gardés ici pour qu'une
/// régression de la bibliothèque ou de sa politique ne passe pas en silence.
fn check_session(
    config: &SamlConfig,
    session: &SsoSession,
    unsolicited: bool,
) -> Result<(), FlowError> {
    if session.issuer().as_str() != config.idp_entity_id {
        return Err(FlowError::Invalid("issuer mismatch".into()));
    }
    if unsolicited && session.in_response_to().is_some() {
        return Err(FlowError::Invalid("unsolicited response carries InResponseTo".into()));
    }
    // L'assertion est protégée par sa propre signature ou par celle de la
    // racine `Response` (qui couvre tout son sous-arbre, assertion comprise).
    // Une signature posée sur un élément enveloppé ne couvre ni l'une ni
    // l'autre, et `saml-rs` ne la rapporte pas.
    let signatures = session.verified_xml_signatures();
    let assertion_protected = signatures.iter().any(|signature| {
        matches!(
            signature.coverage(),
            VerifiedXmlSignatureCoverage::ConsumedAssertion
                | VerifiedXmlSignatureCoverage::ResponseRoot
                | VerifiedXmlSignatureCoverage::ResponseRootAndConsumedAssertion
        )
    });
    if !assertion_protected {
        return Err(FlowError::Invalid("neither the assertion nor the response is signed".into()));
    }
    if let Some(weak) =
        signatures.iter().find(|signature| !strong_algorithm(signature.algorithm_uri()))
    {
        return Err(FlowError::Invalid(format!(
            "signature algorithm not allowed: {}",
            weak.algorithm_uri()
        )));
    }
    Ok(())
}

/// RSA ou ECDSA avec SHA-256 ou plus.
fn strong_algorithm(uri: &str) -> bool {
    matches!(
        uri,
        "http://www.w3.org/2001/04/xmldsig-more#rsa-sha256"
            | "http://www.w3.org/2001/04/xmldsig-more#rsa-sha384"
            | "http://www.w3.org/2001/04/xmldsig-more#rsa-sha512"
            | "http://www.w3.org/2001/04/xmldsig-more#ecdsa-sha256"
            | "http://www.w3.org/2001/04/xmldsig-more#ecdsa-sha384"
            | "http://www.w3.org/2001/04/xmldsig-more#ecdsa-sha512"
    )
}

/// Les valeurs du premier attribut présent parmi `names` (nom comparé sans
/// tenir compte de la casse), vides retirées.
fn attribute_values(session: &SsoSession, names: &[&str]) -> Vec<String> {
    let attributes = session.attributes().as_slice();
    for name in names {
        let values: Vec<String> = attributes
            .iter()
            .filter(|attribute| attribute.name().eq_ignore_ascii_case(name))
            .flat_map(|attribute| attribute.values().iter())
            .map(|value| value.as_str().trim().to_string())
            .filter(|value| !value.is_empty())
            .collect();
        if !values.is_empty() {
            return values;
        }
    }
    Vec::new()
}

fn names<'a>(configured: &'a str, defaults: &[&'a str]) -> Vec<&'a str> {
    if configured.is_empty() { defaults.to_vec() } else { vec![configured] }
}

/// Une adresse plausible : une arobase, rien d'invisible, une longueur sage.
fn plausible_email(value: &str) -> bool {
    let value = value.trim();
    value.len() <= 254
        && value
            .split_once('@')
            .is_some_and(|(local, domain)| !local.is_empty() && domain.contains('.'))
        && !value.chars().any(|c| c.is_whitespace() || c.is_control())
}

fn identity_of(config: &SamlConfig, session: &SsoSession) -> Identity {
    let name_id = session.name_id();
    let format = name_id.format().map(|format| format.as_uri().to_string());
    let value = name_id.value().trim().to_string();
    let transient = format.as_deref() == Some(NAMEID_TRANSIENT);
    let email = attribute_values(session, &names(&config.attr_email, DEFAULT_EMAIL_ATTRIBUTES))
        .into_iter()
        .find(|email| plausible_email(email))
        .or_else(|| {
            (format.as_deref() == Some(NAMEID_EMAIL) && plausible_email(&value))
                .then(|| value.clone())
        });
    let display_name =
        attribute_values(session, &names(&config.attr_name, DEFAULT_NAME_ATTRIBUTES))
            .into_iter()
            .next()
            .unwrap_or_default();
    let groups = attribute_values(session, &names(&config.attr_groups, DEFAULT_GROUP_ATTRIBUTES));
    Identity {
        issuer: session.issuer().as_str().to_string(),
        subject: (!transient && !value.is_empty()).then(|| value.clone()),
        name_id: value,
        email,
        display_name,
        groups,
    }
}

/// Retrouve ou crée le compte, applique le rôle et les équipes des groupes.
pub async fn map_identity(
    pool: &SqlitePool,
    config: &SamlConfig,
    identity: &Identity,
) -> Result<User, FlowError> {
    let role_for = |current| {
        roles::role_for_groups(
            &identity.groups,
            &config.admin_groups,
            &config.operator_groups,
            current,
        )
    };

    let known = match &identity.subject {
        Some(subject) => users::by_saml(pool, &identity.issuer, subject).await?,
        None => None,
    };
    let mut user = match known {
        Some(user) => user,
        None => match find_linkable(pool, identity).await? {
            // Un compte désactivé n'est pas rattaché au passage.
            Some(user) if user.disabled => return Err(FlowError::Disabled),
            Some(user) => {
                if let Some(subject) = &identity.subject {
                    users::link_saml(pool, user.id, &identity.issuer, subject).await?;
                    info!(user = %user.username, "SAML identity linked to an existing account");
                }
                user
            }
            None => {
                if !config.auto_create {
                    return Err(FlowError::NoAccount);
                }
                create_account(pool, identity, role_for(None).unwrap_or(Role::Viewer)).await?
            }
        },
    };
    if user.disabled {
        return Err(FlowError::Disabled);
    }

    // Le rôle suit les groupes à chaque connexion — sauf pour le dernier
    // administrateur, qu'une erreur de groupe chez l'IdP ne doit pas rétrograder.
    if let Some(role) = role_for(Some(user.role))
        && role != user.role
    {
        let last_admin = user.role.is_admin() && users::active_admin_count(pool).await? <= 1;
        if last_admin {
            warn!(user = %user.username, "SAML groups would demote the last admin; role kept");
        } else {
            users::set_role(pool, user.id, role).await?;
            user.role = role;
        }
    }
    teams::sync_idp_groups(pool, user.id, &identity.groups).await?;
    crate::auth::clients::sync_idp_groups(pool, user.id, &identity.groups).await?;
    Ok(user)
}

/// Le compte local auquel rattacher l'identité, par son adresse : un compte
/// qui n'a pas déjà une autre identité SAML, et jamais un administrateur qui a
/// un mot de passe — celui-là garde la main, le SSO crée un compte distinct.
async fn find_linkable(pool: &SqlitePool, identity: &Identity) -> anyhow::Result<Option<User>> {
    let Some(email) = identity.email.as_deref() else { return Ok(None) };
    let Some(user) = users::by_username(pool, email).await? else { return Ok(None) };
    let same_identity = user.saml_issuer.as_deref() == Some(identity.issuer.as_str())
        && user.saml_subject.as_deref() == identity.subject.as_deref();
    if user.saml_subject.is_some() && !same_identity {
        return Ok(None);
    }
    if user.role.is_admin() && user.password_hash.is_some() {
        warn!(
            user = %user.username,
            "SAML login matches a local admin by email; not linked — a distinct account is \
             created instead"
        );
        return Ok(None);
    }
    Ok(Some(user))
}

async fn create_account(
    pool: &SqlitePool,
    identity: &Identity,
    role: Role,
) -> Result<User, FlowError> {
    // L'adresse, sinon le `NameID` : le premier qui fait un identifiant valable.
    let base = [identity.email.as_deref(), Some(identity.name_id.as_str())]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|name| users::validate_username(name).is_ok())
        .map(str::to_string)
        .ok_or(FlowError::NoAccount)?;
    let link = identity.subject.as_deref().map(|subject| (identity.issuer.as_str(), subject));
    for attempt in 0..5u32 {
        let username = if attempt == 0 { base.clone() } else { format!("{base}-{attempt}") };
        let created =
            users::insert_saml(pool, &username, &identity.display_name, role, link).await?;
        if let Some(id) = created {
            info!(user = %username, role = role.as_str(), "account created from SAML login");
            return users::get(pool, id).await?.ok_or_else(|| {
                FlowError::Internal(anyhow!("account {id} vanished right after creation"))
            });
        }
    }
    Err(FlowError::Internal(anyhow!("could not find a free username for {base}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pending(started_at: Instant) -> PendingRequest {
        PendingRequest {
            request_id: "_r".into(),
            idp_entity_id: "https://idp".into(),
            acs_url: "http://x/acs".into(),
            redirect: None,
            started_at,
        }
    }

    #[test]
    fn a_relay_state_is_consumed_once_and_expires() {
        let memory = Memory::default();
        memory.insert_pending("a".into(), pending(Instant::now()));
        assert!(memory.take_pending("a").is_some());
        assert!(memory.take_pending("a").is_none());
        memory.insert_pending(
            "old".into(),
            pending(Instant::now() - PENDING_TTL - Duration::from_secs(1)),
        );
        assert!(memory.take_pending("old").is_none());
    }

    #[test]
    fn the_replay_store_refuses_a_second_use_until_expiry() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let mut store = ReplayStore { now: Some(now), ..Default::default() };
        let key = || ReplayKey::AssertionId(saml_rs::AssertionId::try_new("_a1").unwrap());
        assert!(store.check_and_store(key(), now + Duration::from_secs(60)).is_ok());
        assert!(matches!(
            store.check_and_store(key(), now + Duration::from_secs(60)),
            Err(SamlError::ReplayDetected { .. })
        ));
        store.now = Some(now + Duration::from_secs(120));
        assert!(store.check_and_store(key(), now + Duration::from_secs(300)).is_ok());
    }

    #[test]
    fn only_sha2_signature_algorithms_are_strong() {
        assert!(strong_algorithm("http://www.w3.org/2001/04/xmldsig-more#rsa-sha256"));
        assert!(!strong_algorithm("http://www.w3.org/2000/09/xmldsig#rsa-sha1"));
        assert!(!strong_algorithm("http://www.w3.org/2000/09/xmldsig#dsa-sha1"));
    }

    #[test]
    fn emails_are_checked_for_shape() {
        assert!(plausible_email("alice@example.org"));
        assert!(!plausible_email("alice"));
        assert!(!plausible_email("@example.org"));
        assert!(!plausible_email("a b@example.org"));
    }

    #[test]
    fn idp_values_are_escaped_in_the_rebuilt_metadata() {
        let config = SamlConfig {
            idp_entity_id: "https://idp/\"><x".into(),
            idp_sso_url: "https://idp/sso?a=1&b=2".into(),
            ..Default::default()
        };
        let xml = idp_metadata_xml(&config);
        assert!(xml.contains("entityID=\"https://idp/&quot;&gt;&lt;x\""));
        assert!(xml.contains("Location=\"https://idp/sso?a=1&amp;b=2\""));
    }
}
