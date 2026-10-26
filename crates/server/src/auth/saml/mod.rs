//! Connexion par SAML 2.0 : DumbMonit est fournisseur de service (SP).
//!
//! Le périmètre est celui d'OIDC : se connecter, rien d'autre — ni déconnexion
//! unique, ni assertions chiffrées, ni requêtes signées. Le fournisseur
//! d'identité (IdP) dit *qui* se présente ; DumbMonit décide du rôle et des
//! équipes à partir des groupes annoncés, à chaque connexion.
//!
//! La vérification XML (signature, canonicalisation, défenses contre
//! l'enveloppement de signature, refus du DOCTYPE, bornes du parseur) est
//! confiée à `saml-rs`, avec la politique la plus stricte qu'il propose : voir
//! [`flow::service_provider`]. Ce module ne fait que la configuration.
//!
//! La configuration vit dans le réglage `saml` (JSON en clair) : elle ne
//! contient aucun secret, les certificats de l'IdP étant publics.

pub mod flow;
pub mod import;

use std::time::SystemTime;

use anyhow::Result;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::auth::settings;

/// Clé du réglage enregistré.
const SETTINGS_KEY: &str = "saml";

pub const DEFAULT_BUTTON_LABEL: &str = "SAML";
/// Chemins des routes, relatifs à l'URL publique.
pub const METADATA_PATH: &str = "/api/auth/saml/metadata";
pub const ACS_PATH: &str = "/api/auth/saml/acs";

/// Noms d'attributs essayés quand l'administrateur n'en a pas choisi : ceux
/// d'Entra ID / ADFS (URI de revendication), d'Okta, de Keycloak et
/// d'Authentik, et les noms OID de l'annuaire (eduPerson / inetOrgPerson).
pub const DEFAULT_EMAIL_ATTRIBUTES: &[&str] = &[
    "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/emailaddress",
    "email",
    "mail",
    "Email",
    "emailAddress",
    "urn:oid:0.9.2342.19200300.100.1.3",
];
pub const DEFAULT_NAME_ATTRIBUTES: &[&str] = &[
    "http://schemas.microsoft.com/identity/claims/displayname",
    "displayName",
    "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/name",
    "name",
    "cn",
    "urn:oid:2.16.840.1.113730.3.1.241",
    "urn:oid:2.5.4.3",
];
pub const DEFAULT_GROUP_ATTRIBUTES: &[&str] = &[
    "http://schemas.microsoft.com/ws/2008/06/identity/claims/groups",
    "groups",
    "memberOf",
    "Role",
    "http://schemas.microsoft.com/ws/2008/06/identity/claims/role",
    "urn:oid:1.3.6.1.4.1.5923.1.5.1.1",
];

/// Configuration SAML, telle qu'elle est enregistrée et montrée à l'interface.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct SamlConfig {
    pub enabled: bool,
    /// `entityID` de l'IdP : l'`Issuer` attendu dans chaque réponse.
    pub idp_entity_id: String,
    /// Point d'entrée SSO de l'IdP (binding HTTP-Redirect).
    pub idp_sso_url: String,
    /// Certificats de signature de l'IdP, en PEM. Plusieurs pendant un
    /// renouvellement de certificat.
    pub idp_certificates: Vec<String>,
    /// `entityID` de DumbMonit chez l'IdP (l'audience attendue). Vide : l'URL
    /// des métadonnées.
    pub sp_entity_id: String,
    /// Accepter les réponses que l'IdP envoie sans demande préalable. Désactivé
    /// par défaut : ces réponses ne peuvent pas être liées au navigateur qui
    /// les reçoit (connexion forcée sous une autre identité).
    pub allow_idp_initiated: bool,
    /// Création de comptes à la première connexion. Désactivée par défaut.
    pub auto_create: bool,
    /// Libellé du bouton de l'écran de connexion.
    pub button_label: String,
    /// Noms des attributs ; vides : les noms courants sont essayés.
    pub attr_email: String,
    pub attr_name: String,
    pub attr_groups: String,
    pub admin_groups: Vec<String>,
    pub operator_groups: Vec<String>,
}

impl SamlConfig {
    /// Activée, et assez décrite pour qu'une connexion puisse aboutir.
    pub fn usable(&self) -> bool {
        self.enabled
            && !self.idp_entity_id.is_empty()
            && !self.idp_sso_url.is_empty()
            && !self.idp_certificates.is_empty()
    }

    /// Libellé du bouton, jamais vide.
    pub fn label(&self) -> &str {
        if self.button_label.is_empty() { DEFAULT_BUTTON_LABEL } else { &self.button_label }
    }

    /// Nettoie les champs saisis à la main.
    pub fn normalized(mut self) -> Self {
        for field in [
            &mut self.idp_entity_id,
            &mut self.idp_sso_url,
            &mut self.sp_entity_id,
            &mut self.button_label,
            &mut self.attr_email,
            &mut self.attr_name,
            &mut self.attr_groups,
        ] {
            *field = field.trim().to_string();
        }
        self.admin_groups = crate::auth::oidc::split_groups(&self.admin_groups);
        self.operator_groups = crate::auth::oidc::split_groups(&self.operator_groups);
        self.idp_certificates = self
            .idp_certificates
            .iter()
            .map(|pem| pem.trim().to_string())
            .filter(|pem| !pem.is_empty())
            .collect();
        self
    }

    /// Vérifie ce que l'administrateur a saisi ; rend la configuration avec ses
    /// certificats réécrits en PEM canonique, ou la raison du refus (en
    /// anglais : elle est montrée telle quelle).
    pub fn validated(mut self) -> Result<Self, String> {
        self = self.normalized();
        if self.idp_entity_id.chars().count() > 1024 || self.sp_entity_id.chars().count() > 1024 {
            return Err("An entity ID must be 1024 characters or fewer.".into());
        }
        if !self.idp_sso_url.is_empty() && !is_http_url(&self.idp_sso_url) {
            return Err("The identity provider sign-in URL must be an absolute http(s) URL.".into());
        }
        if self.button_label.chars().count() > 64 {
            return Err("The button label must be 64 characters or fewer.".into());
        }
        if self.idp_certificates.len() > 8 {
            return Err("At most 8 identity provider certificates can be stored.".into());
        }
        let mut certificates = Vec::with_capacity(self.idp_certificates.len());
        for (index, pem) in self.idp_certificates.iter().enumerate() {
            let der = certificate_der(pem).ok_or_else(|| {
                format!(
                    "Identity provider certificate #{} is not a PEM X.509 certificate.",
                    index + 1
                )
            })?;
            certificates.push(pem_from_der(&der));
        }
        certificates.dedup();
        self.idp_certificates = certificates;
        if self.enabled {
            if self.idp_entity_id.is_empty() {
                return Err("Enter the identity provider entity ID.".into());
            }
            if self.idp_sso_url.is_empty() {
                return Err("Enter the identity provider sign-in URL.".into());
            }
            if self.idp_certificates.is_empty() {
                return Err("Add the identity provider signing certificate: assertions must be \
                            signed and are checked against it."
                    .into());
            }
        }
        Ok(self)
    }
}

/// Une URL absolue `http(s)://hôte…`.
fn is_http_url(value: &str) -> bool {
    reqwest::Url::parse(value)
        .is_ok_and(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some())
}

/// Le DER d'un certificat donné en PEM (ou en base64 nu, comme dans les
/// métadonnées), s'il en a la forme.
pub fn certificate_der(text: &str) -> Option<Vec<u8>> {
    let body: String = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("-----"))
        .collect::<String>()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if body.is_empty() || body.len() > 64 * 1024 {
        return None;
    }
    let der = STANDARD.decode(body.as_bytes()).ok()?;
    // Une SEQUENCE DER, d'une taille plausible pour un certificat.
    (der.len() > 64 && der.first() == Some(&0x30)).then_some(der)
}

/// PEM canonique : base64 en lignes de 64 caractères.
pub fn pem_from_der(der: &[u8]) -> String {
    let body = STANDARD.encode(der);
    let mut pem = String::from("-----BEGIN CERTIFICATE-----\n");
    for chunk in body.as_bytes().chunks(64) {
        pem.push_str(std::str::from_utf8(chunk).unwrap_or_default());
        pem.push('\n');
    }
    pem.push_str("-----END CERTIFICATE-----");
    pem
}

/// Base64 nu d'un certificat, tel que les métadonnées le portent.
pub fn certificate_base64(pem: &str) -> Option<String> {
    certificate_der(pem).map(|der| STANDARD.encode(der))
}

/// Échappe une valeur placée dans un attribut ou un texte XML.
pub fn escape_xml(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// Configuration enregistrée, ou la configuration par défaut (désactivée).
pub async fn load(pool: &SqlitePool) -> Result<SamlConfig> {
    Ok(settings::get::<SamlConfig>(pool, SETTINGS_KEY).await?.unwrap_or_default().normalized())
}

pub async fn save(pool: &SqlitePool, config: &SamlConfig) -> Result<()> {
    settings::set(pool, SETTINGS_KEY, config).await
}

/// Adresses de DumbMonit côté SAML, dérivées de l'URL publique ou, à défaut,
/// de l'origine de la requête (comme l'URL de retour OIDC).
#[derive(Debug, Clone)]
pub struct Endpoints {
    pub metadata_url: String,
    pub acs_url: String,
    /// `entityID` effectif : celui configuré, sinon l'URL des métadonnées.
    pub entity_id: String,
}

impl Endpoints {
    pub fn new(config: &SamlConfig, base: &str) -> Self {
        let base = base.trim_end_matches('/');
        let metadata_url = format!("{base}{METADATA_PATH}");
        let entity_id = if config.sp_entity_id.is_empty() {
            metadata_url.clone()
        } else {
            config.sp_entity_id.clone()
        };
        Self { acs_url: format!("{base}{ACS_PATH}"), metadata_url, entity_id }
    }
}

/// Points d'accroche des tests d'intégration : jamais lus dans
/// l'environnement, seulement posés par le code qui monte le serveur. Ils
/// permettent de rejouer des réponses signées une fois pour toutes, dont
/// l'horodatage et l'`InResponseTo` sont figés.
#[derive(Debug, Clone, Default)]
pub struct TestHooks {
    /// Horloge de la validation des assertions.
    pub now: Option<SystemTime>,
    /// Identifiant attendu en `InResponseTo`, à la place de celui de la requête.
    pub request_id: Option<String>,
}

impl TestHooks {
    pub fn now(&self) -> SystemTime {
        self.now.unwrap_or_else(SystemTime::now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CERT: &str = include_str!("../../../tests/fixtures/saml/okta-cert.pem");

    #[test]
    fn a_certificate_is_read_from_pem_or_bare_base64() {
        let der = certificate_der(CERT).expect("PEM");
        assert_eq!(der[0], 0x30);
        let bare = certificate_base64(CERT).unwrap();
        assert_eq!(certificate_der(&bare), Some(der.clone()));
        assert!(pem_from_der(&der).starts_with("-----BEGIN CERTIFICATE-----\n"));
        assert!(certificate_der("not a certificate").is_none());
        assert!(certificate_der("").is_none());
    }

    #[test]
    fn an_enabled_configuration_needs_the_idp_and_its_certificate() {
        let base = SamlConfig {
            enabled: true,
            idp_entity_id: "https://idp.example.org".into(),
            idp_sso_url: "https://idp.example.org/sso".into(),
            idp_certificates: vec![CERT.into()],
            ..Default::default()
        };
        assert!(base.clone().validated().is_ok());
        let no_cert = SamlConfig { idp_certificates: vec![], ..base.clone() };
        assert!(no_cert.validated().is_err());
        let bad_url = SamlConfig { idp_sso_url: "javascript:alert(1)".into(), ..base.clone() };
        assert!(bad_url.validated().is_err());
        let bad_cert = SamlConfig { idp_certificates: vec!["xyz".into()], ..base.clone() };
        assert!(bad_cert.validated().is_err());
        // Désactivée, une configuration incomplète s'enregistre.
        let draft = SamlConfig { enabled: false, ..Default::default() };
        assert!(draft.validated().is_ok());
    }

    #[test]
    fn the_endpoints_follow_the_base_url() {
        let config = SamlConfig::default();
        let endpoints = Endpoints::new(&config, "https://monit.example.org/");
        assert_eq!(endpoints.acs_url, "https://monit.example.org/api/auth/saml/acs");
        assert_eq!(endpoints.entity_id, "https://monit.example.org/api/auth/saml/metadata");
        let pinned = SamlConfig { sp_entity_id: "urn:dumbmonit".into(), ..Default::default() };
        assert_eq!(Endpoints::new(&pinned, "http://x").entity_id, "urn:dumbmonit");
    }

    #[test]
    fn xml_values_are_escaped() {
        assert_eq!(escape_xml(r#"a"<b>&'"#), "a&quot;&lt;b&gt;&amp;&apos;");
    }
}
