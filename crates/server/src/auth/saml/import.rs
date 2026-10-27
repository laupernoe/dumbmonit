//! Lecture des métadonnées d'un IdP, pour pré-remplir le formulaire.
//!
//! Rien n'est enregistré ni approuvé ici : l'administrateur relit l'entité,
//! l'URL et les certificats extraits avant de les enregistrer. La signature
//! éventuelle des métadonnées n'est donc pas vérifiée — c'est le certificat
//! retenu par l'administrateur qui fera foi.

use serde::Serialize;

use super::{certificate_der, pem_from_der};

/// Taille maximale acceptée : les métadonnées d'un IdP pèsent quelques Kio, une
/// fédération entière ne se colle pas dans un formulaire.
pub const MAX_METADATA_BYTES: usize = 512 * 1024;

const MD_NS: &str = "urn:oasis:names:tc:SAML:2.0:metadata";
const DS_NS: &str = "http://www.w3.org/2000/09/xmldsig#";
const REDIRECT: &str = "urn:oasis:names:tc:SAML:2.0:bindings:HTTP-Redirect";
const POST: &str = "urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST";

/// Un élément de cet espace de noms et de ce nom local.
fn is(node: &roxmltree::Node<'_, '_>, ns: &str, name: &str) -> bool {
    node.is_element() && node.tag_name().name() == name && node.tag_name().namespace() == Some(ns)
}

/// Ce que l'interface reçoit : les champs à reporter dans le formulaire.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Imported {
    pub idp_entity_id: String,
    pub idp_sso_url: String,
    pub idp_certificates: Vec<String>,
}

/// Extrait l'entité, le point d'entrée SSO et les certificats de signature du
/// premier `IDPSSODescriptor` du document. Message d'erreur en anglais, montré
/// tel quel.
pub fn parse(xml: &str) -> Result<Imported, String> {
    if xml.len() > MAX_METADATA_BYTES {
        return Err("The metadata document is too large.".into());
    }
    // `roxmltree` refuse les DTD par défaut : ni entité externe, ni bombe
    // d'expansion.
    let document = roxmltree::Document::parse(xml)
        .map_err(|error| format!("The metadata is not readable XML: {error}."))?;
    let descriptor = document
        .descendants()
        .find(|node| is(node, MD_NS, "IDPSSODescriptor"))
        .ok_or("No identity provider (IDPSSODescriptor) was found in this metadata.")?;
    let entity = descriptor
        .ancestors()
        .find(|node| is(node, MD_NS, "EntityDescriptor"))
        .and_then(|node| node.attribute("entityID"))
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .ok_or("The metadata has no entityID.")?;

    let location = |binding: &str| {
        descriptor
            .children()
            .filter(|node| is(node, MD_NS, "SingleSignOnService"))
            .find(|node| node.attribute("Binding") == Some(binding))
            .and_then(|node| node.attribute("Location"))
            .map(str::trim)
            .filter(|url| !url.is_empty())
            .map(str::to_string)
    };
    let sso = location(REDIRECT)
        .or_else(|| location(POST))
        .ok_or("The identity provider declares no HTTP-Redirect sign-in endpoint.")?;

    let mut certificates = Vec::new();
    for key in descriptor.children().filter(|node| is(node, MD_NS, "KeyDescriptor")) {
        // Sans `use`, la clé sert aux deux usages, signature comprise.
        if key.attribute("use").is_some_and(|usage| usage != "signing") {
            continue;
        }
        for cert in key.descendants().filter(|node| is(node, DS_NS, "X509Certificate")) {
            let text = cert.text().unwrap_or_default();
            if let Some(der) = certificate_der(text) {
                let pem = pem_from_der(&der);
                if !certificates.contains(&pem) {
                    certificates.push(pem);
                }
            }
        }
    }
    if certificates.is_empty() {
        return Err("The metadata holds no signing certificate.".into());
    }
    Ok(Imported {
        idp_entity_id: entity.to_string(),
        idp_sso_url: sso,
        idp_certificates: certificates,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CERT: &str = include_str!("../../../tests/fixtures/saml/okta-cert.pem");

    fn metadata(cert_b64: &str) -> String {
        format!(
            r#"<?xml version="1.0"?>
<md:EntityDescriptor xmlns:md="urn:oasis:names:tc:SAML:2.0:metadata" entityID="http://www.okta.com/exk1">
  <md:IDPSSODescriptor protocolSupportEnumeration="urn:oasis:names:tc:SAML:2.0:protocol">
    <md:KeyDescriptor use="encryption"><ds:KeyInfo xmlns:ds="http://www.w3.org/2000/09/xmldsig#"><ds:X509Data><ds:X509Certificate>AAAA</ds:X509Certificate></ds:X509Data></ds:KeyInfo></md:KeyDescriptor>
    <md:KeyDescriptor use="signing"><ds:KeyInfo xmlns:ds="http://www.w3.org/2000/09/xmldsig#"><ds:X509Data><ds:X509Certificate>
{cert_b64}
    </ds:X509Certificate></ds:X509Data></ds:KeyInfo></md:KeyDescriptor>
    <md:SingleSignOnService Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST" Location="https://example.okta.com/app/post"/>
    <md:SingleSignOnService Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-Redirect" Location="https://example.okta.com/app/sso"/>
  </md:IDPSSODescriptor>
</md:EntityDescriptor>"#
        )
    }

    #[test]
    fn the_entity_endpoint_and_signing_certificate_are_extracted() {
        let b64 = super::super::certificate_base64(CERT).unwrap();
        let imported = parse(&metadata(&b64)).expect("métadonnées lisibles");
        assert_eq!(imported.idp_entity_id, "http://www.okta.com/exk1");
        assert_eq!(imported.idp_sso_url, "https://example.okta.com/app/sso");
        assert_eq!(imported.idp_certificates.len(), 1);
        assert!(imported.idp_certificates[0].starts_with("-----BEGIN CERTIFICATE-----"));
    }

    #[test]
    fn a_doctype_is_refused() {
        let xml = r#"<?xml version="1.0"?><!DOCTYPE x [<!ENTITY a "b">]><md:EntityDescriptor xmlns:md="urn:oasis:names:tc:SAML:2.0:metadata" entityID="e"/>"#;
        assert!(parse(xml).is_err());
    }

    #[test]
    fn a_service_provider_document_is_refused() {
        let xml = r#"<md:EntityDescriptor xmlns:md="urn:oasis:names:tc:SAML:2.0:metadata" entityID="e"><md:SPSSODescriptor/></md:EntityDescriptor>"#;
        assert!(parse(xml).is_err());
    }
}
