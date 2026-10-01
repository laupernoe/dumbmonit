//! Gabarits `{{option.x}}` et `{{credential.token}}`.
//!
//! Deux sources de valeurs seulement : les options de la cible (ses `tags`, ou la
//! valeur par défaut déclarée) et son identifiant. Pas d'expression, pas de
//! filtre : un gabarit ne peut que recopier une valeur. L'identifiant n'est
//! admis que dans les en-têtes — la validation refuse ailleurs un gabarit qui le
//! cite —, ce qui le tient hors des URL, donc hors des journaux et des messages
//! d'erreur.

use std::collections::BTreeMap;

use dumbmonit_proto::{Credential, ProbeError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Template {
    parts: Vec<Part>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Part {
    Text(String),
    Option(String),
    Credential(CredentialField),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialField {
    Token,
    Username,
    Password,
}

impl CredentialField {
    fn name(self) -> &'static str {
        match self {
            Self::Token => "token",
            Self::Username => "username",
            Self::Password => "password",
        }
    }
}

/// Comment une valeur est insérée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    /// Chemin et requête d'URL : tout sauf les caractères non réservés est
    /// encodé, si bien qu'une valeur ne peut ni ajouter un segment, ni changer
    /// d'hôte, ni ouvrir une seconde requête.
    Url,
    /// En-tête ou corps : la valeur telle quelle.
    Raw,
}

/// Les valeurs disponibles pour une cible donnée.
pub struct Context<'a> {
    pub options: &'a BTreeMap<String, String>,
    pub credential: &'a Credential,
}

impl Template {
    pub fn parse(raw: &str) -> Result<Self, String> {
        let mut parts = Vec::new();
        let mut rest = raw;
        while let Some(start) = rest.find("{{") {
            if start > 0 {
                parts.push(Part::Text(rest[..start].to_string()));
            }
            let after = &rest[start + 2..];
            let end = after.find("}}").ok_or_else(|| format!("unclosed \"{{{{\" in \"{raw}\""))?;
            let inner = after[..end].trim();
            parts.push(parse_placeholder(inner)?);
            rest = &after[end + 2..];
        }
        if rest.contains("}}") {
            return Err(format!("stray \"}}}}\" in \"{raw}\""));
        }
        if !rest.is_empty() {
            parts.push(Part::Text(rest.to_string()));
        }
        Ok(Self { parts })
    }

    /// Vrai si le gabarit cite l'identifiant de la cible.
    pub fn uses_credential(&self) -> bool {
        self.parts.iter().any(|part| matches!(part, Part::Credential(_)))
    }

    /// Les champs d'identifiant cités.
    pub fn credentials(&self) -> impl Iterator<Item = CredentialField> + '_ {
        self.parts.iter().filter_map(|part| match part {
            Part::Credential(field) => Some(*field),
            _ => None,
        })
    }

    /// Les options citées.
    pub fn options(&self) -> impl Iterator<Item = &str> {
        self.parts.iter().filter_map(|part| match part {
            Part::Option(key) => Some(key.as_str()),
            _ => None,
        })
    }

    /// Le texte sans aucun gabarit, pour les vérifications de forme.
    pub fn literal(&self) -> String {
        self.parts
            .iter()
            .map(|part| match part {
                Part::Text(text) => text.as_str(),
                _ => "x",
            })
            .collect()
    }

    pub fn render(&self, context: &Context<'_>, encoding: Encoding) -> Result<String, ProbeError> {
        let mut out = String::new();
        for part in &self.parts {
            match part {
                Part::Text(text) => out.push_str(text),
                Part::Option(key) => {
                    let value = context.options.get(key).map(String::as_str).unwrap_or("");
                    match encoding {
                        Encoding::Url => encode_into(value, &mut out),
                        Encoding::Raw => out.push_str(value),
                    }
                }
                Part::Credential(field) => {
                    let value = credential_value(context.credential, *field)?;
                    match encoding {
                        Encoding::Url => encode_into(value, &mut out),
                        Encoding::Raw => out.push_str(value),
                    }
                }
            }
        }
        Ok(out)
    }
}

fn parse_placeholder(inner: &str) -> Result<Part, String> {
    if let Some(key) = inner.strip_prefix("option.") {
        if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(format!("invalid option name in \"{{{{{inner}}}}}\""));
        }
        return Ok(Part::Option(key.to_string()));
    }
    let field = match inner {
        "credential.token" => CredentialField::Token,
        "credential.username" => CredentialField::Username,
        "credential.password" => CredentialField::Password,
        _ => {
            return Err(format!(
                "unknown placeholder \"{{{{{inner}}}}}\": use {{{{option.<key>}}}}, \
                 {{{{credential.token}}}}, {{{{credential.username}}}} or \
                 {{{{credential.password}}}}"
            ));
        }
    };
    Ok(Part::Credential(field))
}

fn credential_value(credential: &Credential, field: CredentialField) -> Result<&str, ProbeError> {
    match (credential, field) {
        (Credential::ApiToken { token }, CredentialField::Token) => Ok(token),
        (Credential::UsernamePassword { username, .. }, CredentialField::Username) => Ok(username),
        (Credential::UsernamePassword { password, .. }, CredentialField::Password) => Ok(password),
        _ => Err(ProbeError::Config(format!(
            "This device type needs a credential providing \"{}\"",
            field.name()
        ))),
    }
}

fn encode_into(value: &str, out: &mut String) {
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn un_gabarit_recopie_les_options_et_lidentifiant() {
        let template = Template::parse("Bearer {{ credential.token }} / {{option.id}}").unwrap();
        let options = options(&[("id", "7")]);
        let credential = Credential::ApiToken { token: "s3cret".into() };
        let context = Context { options: &options, credential: &credential };
        assert_eq!(template.render(&context, Encoding::Raw).unwrap(), "Bearer s3cret / 7");
        assert!(template.uses_credential());
        assert_eq!(template.options().collect::<Vec<_>>(), ["id"]);
    }

    #[test]
    fn dans_une_url_une_valeur_ne_peut_pas_changer_dhote_ni_de_segment() {
        let template = Template::parse("/rpc/Switch.GetStatus?id={{option.id}}").unwrap();
        let options = options(&[("id", "0&x=@evil.lan/../#")]);
        let context = Context { options: &options, credential: &Credential::None };
        assert_eq!(
            template.render(&context, Encoding::Url).unwrap(),
            "/rpc/Switch.GetStatus?id=0%26x%3D%40evil.lan%2F..%2F%23"
        );
    }

    #[test]
    fn les_gabarits_mal_formes_sont_refuses() {
        assert!(Template::parse("{{option.x").is_err());
        assert!(Template::parse("x}}").is_err());
        assert!(Template::parse("{{target.address}}").is_err());
        assert!(Template::parse("{{option.a-b}}").is_err());
        assert!(Template::parse("{{credential.secret}}").is_err());
    }

    #[test]
    fn un_identifiant_absent_est_une_erreur_de_configuration() {
        let template = Template::parse("{{credential.password}}").unwrap();
        let options = BTreeMap::new();
        let context = Context { options: &options, credential: &Credential::None };
        let error = template.render(&context, Encoding::Raw).unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)));
    }
}
