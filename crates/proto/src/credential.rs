use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Secret d'accès à une cible. Toujours chiffré au repos dans la base.
///
/// La désérialisation passe par [`CredentialInput`] : la forme stockée et
/// renvoyée est celle-ci, mais l'API accepte en entrée quelques variantes plus
/// commodes à saisir (un jeton Proxmox en deux champs, par exemple), et des
/// secrets laissés vides lors d'une modification (voir [`CredentialInput::resolve`]).
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", try_from = "CredentialInput")]
pub enum Credential {
    /// Aucune authentification (ICMP, HTTP public).
    None,
    /// SNMP v1 et v2c : une simple community.
    SnmpCommunity { community: String },
    /// SNMP v3 (USM).
    SnmpV3 {
        username: String,
        #[serde(default)]
        auth: Option<SnmpV3Auth>,
        #[serde(default)]
        privacy: Option<SnmpV3Privacy>,
        #[serde(default)]
        context: Option<String>,
    },
    /// Jeton porteur, pour les intégrations API (Proxmox, HTTP).
    ///
    /// Pour Proxmox VE et PBS, `token` est la forme complète `user@realm!nom=secret` :
    /// c'est ce que les collecteurs attendent, et ce que [`CredentialInput`]
    /// assemble quand l'identifiant et le secret arrivent séparément.
    ApiToken { token: String },
    /// Identifiants classiques, pour les API qui n'acceptent pas de jeton.
    UsernamePassword { username: String, password: String },
    /// RADIUS : le secret partagé avec le serveur, et le compte de test dont la
    /// sonde présente l'identifiant et le mot de passe.
    Radius { shared_secret: String, username: String, password: String },
}

/// Forme acceptée en entrée pour un [`Credential`].
///
/// Deux différences avec le type stocké :
///
/// - un `api_token` peut arriver soit sous la forme combinée `token`, soit en
///   deux morceaux `token_id` + `secret` — les deux champs que l'interface
///   présente pour Proxmox VE et PBS, parce que personne ne devine qu'il faut
///   coller `user@pve!nom=secret` dans une seule case ;
/// - chaque secret (community, phrases de passe, secret du jeton, mot de passe)
///   peut être absent ou vide : en modification, cela veut dire « garder celui
///   qui est enregistré », à condition que l'identité à laquelle il appartient
///   (utilisateur, identifiant de jeton) n'ait pas changé.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CredentialInput {
    None,
    SnmpCommunity {
        #[serde(default)]
        community: Option<String>,
    },
    SnmpV3 {
        username: String,
        #[serde(default)]
        auth: Option<SnmpV3AuthInput>,
        #[serde(default)]
        privacy: Option<SnmpV3PrivacyInput>,
        #[serde(default)]
        context: Option<String>,
    },
    ApiToken {
        #[serde(default)]
        token: Option<String>,
        #[serde(default)]
        token_id: Option<String>,
        #[serde(default)]
        secret: Option<String>,
    },
    UsernamePassword {
        username: String,
        #[serde(default)]
        password: Option<String>,
    },
    Radius {
        username: String,
        #[serde(default)]
        password: Option<String>,
        #[serde(default)]
        shared_secret: Option<String>,
    },
}

/// Authentification SNMP v3 reçue par l'API : la phrase de passe peut manquer.
#[derive(Deserialize)]
pub struct SnmpV3AuthInput {
    pub protocol: SnmpV3AuthProtocol,
    #[serde(default)]
    pub passphrase: Option<String>,
}

/// Chiffrement SNMP v3 reçu par l'API : la phrase de passe peut manquer.
#[derive(Deserialize)]
pub struct SnmpV3PrivacyInput {
    pub protocol: SnmpV3PrivacyProtocol,
    #[serde(default)]
    pub passphrase: Option<String>,
}

/// Résultat de [`CredentialInput::resolve`].
pub struct Resolved {
    pub credential: Credential,
    /// Vrai si au moins un secret vient de l'identifiant enregistré plutôt que
    /// de la requête : l'appelant doit alors vérifier que la destination n'a
    /// pas changé (un secret ne part jamais vers une destination nouvelle).
    pub reused_secret: bool,
}

/// Nettoie une valeur collée : espaces et sauts de ligne autour, guillemets
/// d'encadrement. Le message d'erreur d'un jeton copié « avec ses guillemets »
/// est trop opaque pour laisser la valeur telle quelle.
fn tidy(value: &str) -> &str {
    let value = value.trim();
    for quote in ['"', '\''] {
        if value.len() >= 2 && value.starts_with(quote) && value.ends_with(quote) {
            return value[1..value.len() - 1].trim();
        }
    }
    value
}

/// Vrai pour un identifiant de jeton Proxmox `user@realm!nom`.
///
/// Sert à reconnaître la partie publique d'un jeton enregistré : un jeton
/// porteur quelconque (base64 terminé par `=`, par exemple) ne doit jamais être
/// coupé en deux, sa première moitié serait encore un secret.
fn is_token_id(value: &str) -> bool {
    let plain = |part: &str| {
        !part.is_empty() && !part.chars().any(|c| c.is_whitespace() || "@!=".contains(c))
    };
    let Some((user, rest)) = value.split_once('@') else { return false };
    let Some((realm, name)) = rest.split_once('!') else { return false };
    plain(user) && plain(realm) && plain(name)
}

/// Un jeton Proxmox enregistré, coupé en identifiant public et secret.
fn split_token(token: &str) -> Option<(&str, &str)> {
    token.split_once('=').filter(|(id, _)| is_token_id(id))
}

/// Ce que l'on peut faire d'un secret laissé vide.
enum Keep<'a> {
    /// Rien d'enregistré pour cette identité : la valeur reçue est prise telle quelle.
    New,
    /// Un secret est enregistré, mais pour une autre identité, nommée ici.
    Refused(&'static str),
    /// Le secret enregistré, à reprendre.
    Saved(&'a str),
}

/// Le secret à enregistrer pour `field` : celui reçu s'il n'est pas vide, sinon
/// celui qui est enregistré quand c'est cohérent.
fn secret(
    given: Option<String>,
    keep: Keep<'_>,
    field: &str,
    reused: &mut bool,
) -> Result<String, String> {
    match (given, keep) {
        (Some(value), _) if !value.is_empty() => Ok(value),
        (_, Keep::Saved(saved)) => {
            *reused = true;
            Ok(saved.to_owned())
        }
        (_, Keep::Refused(identity)) => Err(format!(
            "Enter the {field} again: the saved one belongs to the previous {identity}."
        )),
        // Rétrocompatibilité : une valeur vide explicite reste acceptée sans rien d'enregistré.
        (Some(value), Keep::New) => Ok(value),
        (None, Keep::New) => Err(format!("The {field} is required.")),
    }
}

/// Une phrase de passe SNMP v3 enregistrée, reprise seulement pour le même utilisateur.
fn keep_phrase(same_user: bool, phrase: Option<&str>) -> Keep<'_> {
    match phrase {
        Some(phrase) if same_user => Keep::Saved(phrase),
        Some(_) => Keep::Refused("user name"),
        None => Keep::New,
    }
}

impl CredentialInput {
    /// Identifiant à enregistrer, `saved` étant celui qui l'est déjà (`None` à
    /// la création).
    ///
    /// Un secret absent ou vide reprend celui de `saved` quand le type est le
    /// même et que l'identité n'a pas changé (même utilisateur, même identifiant
    /// de jeton) ; si l'identité a changé, l'erreur demande de le ressaisir,
    /// plutôt que d'enregistrer un secret vide.
    pub fn resolve(self, saved: Option<&Credential>) -> Result<Resolved, String> {
        let mut reused = false;
        let credential = match self {
            Self::None => Credential::None,
            Self::SnmpCommunity { community } => {
                let keep = match saved {
                    Some(Credential::SnmpCommunity { community }) => Keep::Saved(community),
                    _ => Keep::New,
                };
                Credential::SnmpCommunity {
                    community: secret(community, keep, "community", &mut reused)?,
                }
            }
            Self::SnmpV3 { username, auth, privacy, context } => {
                // Les phrases de passe appartiennent à l'utilisateur USM : changer
                // de protocole les garde, changer d'utilisateur oblige à les ressaisir.
                let (saved_auth, saved_privacy) = match saved {
                    Some(Credential::SnmpV3 { username: before, auth, privacy, .. }) => {
                        let same = *before == username;
                        (
                            keep_phrase(same, auth.as_ref().map(|a| a.passphrase.as_str())),
                            keep_phrase(same, privacy.as_ref().map(|p| p.passphrase.as_str())),
                        )
                    }
                    _ => (Keep::New, Keep::New),
                };
                let auth = match auth {
                    Some(SnmpV3AuthInput { protocol, passphrase }) => Some(SnmpV3Auth {
                        protocol,
                        passphrase: secret(
                            passphrase,
                            saved_auth,
                            "authentication passphrase",
                            &mut reused,
                        )?,
                    }),
                    None => None,
                };
                let privacy = match privacy {
                    Some(SnmpV3PrivacyInput { protocol, passphrase }) => Some(SnmpV3Privacy {
                        protocol,
                        passphrase: secret(
                            passphrase,
                            saved_privacy,
                            "privacy passphrase",
                            &mut reused,
                        )?,
                    }),
                    None => None,
                };
                Credential::SnmpV3 { username, auth, privacy, context }
            }
            Self::UsernamePassword { username, password } => {
                let keep = match saved {
                    Some(Credential::UsernamePassword { username: before, password })
                        if *before == username =>
                    {
                        Keep::Saved(password)
                    }
                    Some(Credential::UsernamePassword { password, .. }) if !password.is_empty() => {
                        Keep::Refused("user name")
                    }
                    _ => Keep::New,
                };
                let password = secret(password, keep, "password", &mut reused)?;
                Credential::UsernamePassword { username, password }
            }
            Self::Radius { username, password, shared_secret } => {
                // Le secret partagé appartient au serveur, pas au compte : il
                // survit à un changement de nom d'utilisateur. Le mot de passe,
                // lui, appartient au compte.
                let (keep_secret, keep_password) = match saved {
                    Some(Credential::Radius { username: before, password, shared_secret }) => (
                        Keep::Saved(shared_secret),
                        if *before == username {
                            Keep::Saved(password)
                        } else if !password.is_empty() {
                            Keep::Refused("user name")
                        } else {
                            Keep::New
                        },
                    ),
                    _ => (Keep::New, Keep::New),
                };
                let shared_secret =
                    secret(shared_secret, keep_secret, "shared secret", &mut reused)?;
                let password = secret(password, keep_password, "password", &mut reused)?;
                Credential::Radius { shared_secret, username, password }
            }
            Self::ApiToken { token, token_id, secret } => {
                let token_id = token_id.as_deref().map(tidy).filter(|s| !s.is_empty());
                let secret = secret.as_deref().map(tidy).filter(|s| !s.is_empty());
                let token = token.as_deref().map(tidy).filter(|s| !s.is_empty());
                let saved_token = match saved {
                    Some(Credential::ApiToken { token }) => Some(token.as_str()),
                    _ => None,
                };
                let token = match (token_id, secret, token) {
                    // Un identifiant collé avec son secret (`user@pve!nom=secret`,
                    // la forme historique) est accepté tel quel plutôt que doublé.
                    (Some(id), secret, _) if id.contains('=') => match secret {
                        Some(secret) if !id.ends_with(&format!("={secret}")) => {
                            return Err("api_token: \"token_id\" already ends with a secret, \
                                        which differs from \"secret\""
                                .into());
                        }
                        _ => id.to_string(),
                    },
                    // Les deux morceaux : on assemble la forme que les collecteurs
                    // Proxmox attendent.
                    (Some(id), Some(secret), _) => format!("{id}={secret}"),
                    // L'identifiant seul : le secret enregistré, s'il appartient
                    // bien à ce jeton-là.
                    (Some(id), None, _) => match saved_token.and_then(split_token) {
                        Some((before, saved_secret)) if before == id => {
                            reused = true;
                            format!("{id}={saved_secret}")
                        }
                        Some(_) => {
                            return Err("Enter the token secret again: the saved one belongs \
                                        to the previous token ID."
                                .into());
                        }
                        None => {
                            return Err(
                                "api_token: \"secret\" is required with \"token_id\"".into()
                            );
                        }
                    },
                    (None, Some(_), _) => {
                        return Err("api_token: \"token_id\" is required with \"secret\"".into());
                    }
                    (None, None, Some(token)) => token.to_string(),
                    // Rien de saisi : le jeton enregistré, tel quel.
                    (None, None, None) => match saved_token {
                        Some(saved) if !saved.is_empty() => {
                            reused = true;
                            saved.to_owned()
                        }
                        _ => {
                            return Err(
                                "api_token: give \"token\", or \"token_id\" and \"secret\"".into(),
                            );
                        }
                    },
                };
                Credential::ApiToken { token }
            }
        };
        Ok(Resolved { credential, reused_secret: reused })
    }
}

/// Comme pour [`Credential`], `Debug` n'affiche que le type, jamais un secret.
impl std::fmt::Debug for CredentialInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self {
            Self::None => "none",
            Self::SnmpCommunity { .. } => "snmp_community",
            Self::SnmpV3 { .. } => "snmp_v3",
            Self::ApiToken { .. } => "api_token",
            Self::UsernamePassword { .. } => "username_password",
            Self::Radius { .. } => "radius",
        };
        write!(f, "CredentialInput({kind}, <redacted>)")
    }
}

impl TryFrom<CredentialInput> for Credential {
    type Error = String;

    fn try_from(input: CredentialInput) -> Result<Self, Self::Error> {
        input.resolve(None).map(|resolved| resolved.credential)
    }
}

impl Credential {
    /// Étiquette courte affichée dans l'interface, garantie sans secret.
    pub fn kind_label(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::SnmpCommunity { .. } => "SNMP community",
            Self::SnmpV3 { .. } => "SNMP v3",
            Self::ApiToken { .. } => "API token",
            Self::UsernamePassword { .. } => "Username / password",
            Self::Radius { .. } => "RADIUS",
        }
    }
}

impl Credential {
    /// Vrai si un secret est enregistré : community, phrase de passe, jeton ou
    /// mot de passe. L'interface l'annonce (« enregistré, laisser vide pour le
    /// garder ») sans jamais le recevoir.
    pub fn has_secret(&self) -> bool {
        match self {
            Self::None => false,
            Self::SnmpCommunity { community } => !community.is_empty(),
            Self::SnmpV3 { auth, privacy, .. } => auth.is_some() || privacy.is_some(),
            Self::ApiToken { token } => !token.is_empty(),
            Self::UsernamePassword { password, .. } => !password.is_empty(),
            Self::Radius { shared_secret, password, .. } => {
                !shared_secret.is_empty() || !password.is_empty()
            }
        }
    }

    /// Les parties non secrètes de l'identifiant, sous les clés des champs du
    /// formulaire, pour le pré-remplir en modification.
    ///
    /// Garantie sans secret : une community v1/v2c, une phrase de passe, un mot
    /// de passe ou un jeton porteur n'y figurent jamais. D'un jeton, seul
    /// l'identifiant Proxmox `user@realm!nom` est extrait — et seulement s'il en
    /// a exactement la forme.
    pub fn public_fields(&self) -> BTreeMap<&'static str, String> {
        let mut out = BTreeMap::new();
        match self {
            Self::None | Self::SnmpCommunity { .. } => {}
            Self::SnmpV3 { username, auth, privacy, context } => {
                out.insert("username", username.clone());
                if let Some(auth) = auth {
                    out.insert("auth_protocol", auth.protocol.key().to_owned());
                }
                if let Some(privacy) = privacy {
                    out.insert("privacy_protocol", privacy.protocol.key().to_owned());
                }
                if let Some(context) = context.as_ref().filter(|c| !c.is_empty()) {
                    out.insert("context", context.clone());
                }
            }
            Self::ApiToken { token } => {
                if let Some((id, _)) = split_token(token) {
                    out.insert("token_id", id.to_owned());
                }
            }
            Self::UsernamePassword { username, .. } | Self::Radius { username, .. } => {
                out.insert("username", username.clone());
            }
        }
        out
    }
}

/// `Debug` est implémenté à la main — et non dérivé — pour qu'un secret ne puisse
/// jamais atterrir dans un journal, un rapport de panique ou un message d'erreur.
/// Ne dérivez `Debug` sur aucun des types de ce fichier.
impl std::fmt::Debug for Credential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => f.write_str("Credential::None"),
            Self::SnmpCommunity { .. } => f.write_str("Credential::SnmpCommunity(<redacted>)"),
            Self::SnmpV3 { username, auth, privacy, .. } => write!(
                f,
                "Credential::SnmpV3 {{ username: {username:?}, auth: {}, privacy: {} }}",
                auth.as_ref().map_or("none", |a| a.protocol.as_str()),
                privacy.as_ref().map_or("none", |p| p.protocol.as_str()),
            ),
            Self::ApiToken { .. } => f.write_str("Credential::ApiToken(<redacted>)"),
            Self::UsernamePassword { username, .. } => {
                write!(
                    f,
                    "Credential::UsernamePassword {{ username: {username:?}, password: <redacted> }}"
                )
            }
            Self::Radius { username, .. } => write!(
                f,
                "Credential::Radius {{ username: {username:?}, shared_secret: <redacted>, \
                 password: <redacted> }}"
            ),
        }
    }
}

impl std::fmt::Display for Credential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.kind_label())
    }
}

impl std::fmt::Debug for SnmpV3Auth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SnmpV3Auth {{ protocol: {}, passphrase: <redacted> }}", self.protocol.as_str())
    }
}

impl std::fmt::Debug for SnmpV3Privacy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SnmpV3Privacy {{ protocol: {}, passphrase: <redacted> }}",
            self.protocol.as_str()
        )
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnmpV3Auth {
    pub protocol: SnmpV3AuthProtocol,
    pub passphrase: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SnmpV3AuthProtocol {
    Md5,
    Sha1,
    Sha224,
    Sha256,
    Sha384,
    Sha512,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnmpV3Privacy {
    pub protocol: SnmpV3PrivacyProtocol,
    pub passphrase: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SnmpV3PrivacyProtocol {
    Des,
    Aes128,
    Aes192,
    Aes256,
}

impl SnmpV3AuthProtocol {
    /// Valeur sérialisée (`sha256`), celle des choix du formulaire.
    pub fn key(&self) -> &'static str {
        match self {
            Self::Md5 => "md5",
            Self::Sha1 => "sha1",
            Self::Sha224 => "sha224",
            Self::Sha256 => "sha256",
            Self::Sha384 => "sha384",
            Self::Sha512 => "sha512",
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Md5 => "MD5",
            Self::Sha1 => "SHA-1",
            Self::Sha224 => "SHA-224",
            Self::Sha256 => "SHA-256",
            Self::Sha384 => "SHA-384",
            Self::Sha512 => "SHA-512",
        }
    }
}

impl SnmpV3PrivacyProtocol {
    /// Valeur sérialisée (`aes128`), celle des choix du formulaire.
    pub fn key(&self) -> &'static str {
        match self {
            Self::Des => "des",
            Self::Aes128 => "aes128",
            Self::Aes192 => "aes192",
            Self::Aes256 => "aes256",
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Des => "DES",
            Self::Aes128 => "AES-128",
            Self::Aes192 => "AES-192",
            Self::Aes256 => "AES-256",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Garde-fou : une régression ici exposerait des mots de passe dans les journaux.
    #[test]
    fn debug_never_leaks_a_secret() {
        let secrets = ["s3cr3t-community", "s3cr3t-auth", "s3cr3t-priv", "s3cr3t-token"];

        let credentials = vec![
            Credential::SnmpCommunity { community: secrets[0].into() },
            Credential::SnmpV3 {
                username: "monitor".into(),
                auth: Some(SnmpV3Auth {
                    protocol: SnmpV3AuthProtocol::Sha256,
                    passphrase: secrets[1].into(),
                }),
                privacy: Some(SnmpV3Privacy {
                    protocol: SnmpV3PrivacyProtocol::Aes128,
                    passphrase: secrets[2].into(),
                }),
                context: None,
            },
            Credential::ApiToken { token: secrets[3].into() },
            Credential::UsernamePassword {
                username: "admin".into(),
                password: "s3cr3t-password".into(),
            },
        ];

        for credential in &credentials {
            let rendered = format!("{credential:?} {credential}");
            for secret in secrets {
                assert!(!rendered.contains(secret), "le secret {secret} a fuité dans : {rendered}");
            }
            assert!(!rendered.contains("s3cr3t-password"), "mot de passe fuité : {rendered}");
        }
    }

    fn parse(json: &str) -> Result<Credential, String> {
        serde_json::from_str::<Credential>(json).map_err(|e| e.to_string())
    }

    /// L'interface envoie l'identifiant et le secret d'un jeton Proxmox dans deux
    /// champs : le serveur les assemble sous la forme que les collecteurs lisent.
    #[test]
    fn un_jeton_en_deux_morceaux_est_assemble() {
        let credential =
            parse(r#"{"type":"api_token","token_id":" dumbmonit@pve!monitor ","secret":"\"8f3a-1c9e\"\n"}"#)
                .unwrap();
        assert_eq!(
            credential,
            Credential::ApiToken { token: "dumbmonit@pve!monitor=8f3a-1c9e".into() }
        );
    }

    #[test]
    fn la_forme_combinee_reste_acceptee() {
        let combined = Credential::ApiToken { token: "dumbmonit@pve!monitor=8f3a-1c9e".into() };
        assert_eq!(
            parse(r#"{"type":"api_token","token":"dumbmonit@pve!monitor=8f3a-1c9e"}"#).unwrap(),
            combined
        );
        // Jeton complet collé dans le champ « Token ID », avec ou sans le secret répété.
        assert_eq!(
            parse(r#"{"type":"api_token","token_id":"dumbmonit@pve!monitor=8f3a-1c9e"}"#).unwrap(),
            combined
        );
        assert_eq!(
            parse(r#"{"type":"api_token","token_id":"dumbmonit@pve!monitor=8f3a-1c9e","secret":"8f3a-1c9e"}"#).unwrap(),
            combined
        );
        // Ce que le serveur renvoie et stocke ne change pas de forme.
        assert_eq!(
            serde_json::to_string(&combined).unwrap(),
            r#"{"type":"api_token","token":"dumbmonit@pve!monitor=8f3a-1c9e"}"#
        );
    }

    #[test]
    fn un_jeton_incomplet_est_refuse_avec_le_champ_manquant() {
        let err = parse(r#"{"type":"api_token","token_id":"dumbmonit@pve!monitor"}"#).unwrap_err();
        assert!(err.contains("secret"), "{err}");
        let err = parse(r#"{"type":"api_token","secret":"8f3a"}"#).unwrap_err();
        assert!(err.contains("token_id"), "{err}");
        let err = parse(r#"{"type":"api_token"}"#).unwrap_err();
        assert!(err.contains("token"), "{err}");
        let err =
            parse(r#"{"type":"api_token","token_id":"a@pve!b=one","secret":"two"}"#).unwrap_err();
        assert!(err.contains("differs"), "{err}");
    }

    #[test]
    fn les_autres_formes_traversent_sans_changement() {
        assert_eq!(parse(r#"{"type":"none"}"#).unwrap(), Credential::None);
        assert_eq!(
            parse(r#"{"type":"username_password","username":"dumbmonit","password":" p "}"#)
                .unwrap(),
            Credential::UsernamePassword { username: "dumbmonit".into(), password: " p ".into() }
        );
        assert_eq!(
            parse(r#"{"type":"snmp_v3","username":"monitor"}"#).unwrap(),
            Credential::SnmpV3 {
                username: "monitor".into(),
                auth: None,
                privacy: None,
                context: None
            }
        );
    }

    fn radius() -> Credential {
        Credential::Radius {
            shared_secret: "s3cr3t-shared".into(),
            username: "probe".into(),
            password: "s3cr3t-pw".into(),
        }
    }

    #[test]
    fn radius_ne_fuit_ni_dans_le_debug_ni_dans_la_partie_publique() {
        let credential = radius();
        let rendered = format!("{credential:?}");
        assert!(rendered.contains("probe"), "{rendered}");
        assert!(!rendered.contains("s3cr3t"), "{rendered}");
        let public = credential.public_fields();
        assert_eq!(public.len(), 1);
        assert_eq!(public.get("username").map(String::as_str), Some("probe"));
        assert!(credential.has_secret());
        assert_eq!(credential.kind_label(), "RADIUS");
        let input: CredentialInput = serde_json::from_str(
            r#"{"type":"radius","username":"probe","password":"s3cr3t-pw","shared_secret":"x"}"#,
        )
        .unwrap();
        assert!(!format!("{input:?}").contains("s3cr3t"));
    }

    #[test]
    fn radius_se_stocke_et_se_relit_sous_sa_forme_complete() {
        let json = serde_json::to_string(&radius()).unwrap();
        assert!(json.contains(r#""type":"radius""#), "{json}");
        let back: Credential = serde_json::from_str(&json).unwrap();
        assert_eq!(back, radius());
        // Sans mot de passe ni secret et sans rien d'enregistré : refusé.
        assert!(parse(r#"{"type":"radius","username":"probe"}"#).is_err());
    }

    #[test]
    fn radius_garde_chaque_secret_laisse_vide() {
        let resolved = resolve(r#"{"type":"radius","username":"probe"}"#, &radius()).unwrap();
        assert_eq!(resolved.credential, radius());
        assert!(resolved.reused_secret);
    }

    /// Changer de compte oblige à retaper son mot de passe, pas le secret du serveur.
    #[test]
    fn radius_change_de_compte_sans_perdre_le_secret_partage() {
        let err = resolve(r#"{"type":"radius","username":"autre"}"#, &radius()).err().unwrap();
        assert!(err.contains("Enter the password again"), "{err}");
        let resolved =
            resolve(r#"{"type":"radius","username":"autre","password":"neuf"}"#, &radius())
                .unwrap();
        let Credential::Radius { shared_secret, username, password } = &resolved.credential else {
            panic!("RADIUS attendu");
        };
        assert_eq!(shared_secret, "s3cr3t-shared");
        assert_eq!((username.as_str(), password.as_str()), ("autre", "neuf"));
    }

    #[test]
    fn debug_keeps_the_useful_context() {
        let credential = Credential::SnmpV3 {
            username: "monitor".into(),
            auth: Some(SnmpV3Auth { protocol: SnmpV3AuthProtocol::Sha256, passphrase: "x".into() }),
            privacy: None,
            context: None,
        };
        let rendered = format!("{credential:?}");
        assert!(rendered.contains("monitor"), "{rendered}");
        assert!(rendered.contains("SHA-256"), "{rendered}");
        assert!(rendered.contains("none"), "{rendered}");
    }

    fn input(json: &str) -> CredentialInput {
        serde_json::from_str(json).unwrap()
    }

    fn resolve(json: &str, saved: &Credential) -> Result<Resolved, String> {
        input(json).resolve(Some(saved))
    }

    fn proxmox() -> Credential {
        Credential::ApiToken { token: "dumbmonit@pve!monitor=8f3a-1c9e".into() }
    }

    fn snmp_v3() -> Credential {
        Credential::SnmpV3 {
            username: "monitor".into(),
            auth: Some(SnmpV3Auth { protocol: SnmpV3AuthProtocol::Sha256, passphrase: "a".into() }),
            privacy: Some(SnmpV3Privacy {
                protocol: SnmpV3PrivacyProtocol::Aes128,
                passphrase: "p".into(),
            }),
            context: None,
        }
    }

    /// Le formulaire renvoie l'identifiant de jeton seul : le secret enregistré
    /// est repris, il appartient bien à ce jeton.
    #[test]
    fn un_identifiant_de_jeton_seul_garde_le_secret() {
        let resolved =
            resolve(r#"{"type":"api_token","token_id":"dumbmonit@pve!monitor"}"#, &proxmox())
                .unwrap();
        assert_eq!(resolved.credential, proxmox());
        assert!(resolved.reused_secret);
        let resolved = resolve(
            r#"{"type":"api_token","token_id":"dumbmonit@pve!monitor","secret":""}"#,
            &proxmox(),
        )
        .unwrap();
        assert_eq!(resolved.credential, proxmox());
    }

    /// Un autre jeton n'hérite jamais du secret du précédent : l'erreur le dit.
    #[test]
    fn un_autre_identifiant_de_jeton_demande_le_secret() {
        let err = resolve(r#"{"type":"api_token","token_id":"autre@pve!monitor"}"#, &proxmox())
            .err()
            .unwrap();
        assert!(err.contains("Enter the token secret again"), "{err}");
        // Avec le nouveau secret, tout est remplacé.
        let resolved = resolve(
            r#"{"type":"api_token","token_id":"autre@pve!monitor","secret":"neuf"}"#,
            &proxmox(),
        )
        .unwrap();
        assert_eq!(
            resolved.credential,
            Credential::ApiToken { token: "autre@pve!monitor=neuf".into() }
        );
        assert!(!resolved.reused_secret);
    }

    #[test]
    fn un_jeton_porteur_laisse_vide_est_garde() {
        let saved = Credential::ApiToken { token: "3-AbCd==".into() };
        let resolved = resolve(r#"{"type":"api_token","token":""}"#, &saved).unwrap();
        assert_eq!(resolved.credential, saved);
        assert!(resolved.reused_secret);
        // Sans rien d'enregistré, l'erreur d'avant.
        assert!(input(r#"{"type":"api_token","token":""}"#).resolve(None).is_err());
    }

    #[test]
    fn un_mot_de_passe_vide_est_garde_pour_le_meme_utilisateur() {
        let saved =
            Credential::UsernamePassword { username: "root@pam".into(), password: "pw".into() };
        let resolved =
            resolve(r#"{"type":"username_password","username":"root@pam"}"#, &saved).unwrap();
        assert_eq!(resolved.credential, saved);
        assert!(resolved.reused_secret);
        let err =
            resolve(r#"{"type":"username_password","username":"autre@pve","password":""}"#, &saved)
                .err()
                .unwrap();
        assert!(err.contains("Enter the password again"), "{err}");
    }

    /// Cause d'une vraie perte : ressaisir la phrase d'authentification sans
    /// celle de chiffrement effaçait cette dernière. Elle est désormais gardée.
    #[test]
    fn snmp_v3_garde_chaque_phrase_laissee_vide() {
        let resolved = resolve(
            r#"{"type":"snmp_v3","username":"monitor",
                "auth":{"protocol":"sha512","passphrase":"nouvelle"},
                "privacy":{"protocol":"aes256"}}"#,
            &snmp_v3(),
        )
        .unwrap();
        let Credential::SnmpV3 { auth, privacy, .. } = &resolved.credential else { panic!() };
        let auth = auth.as_ref().unwrap();
        let privacy = privacy.as_ref().unwrap();
        assert_eq!(auth.protocol, SnmpV3AuthProtocol::Sha512);
        assert_eq!(auth.passphrase, "nouvelle");
        assert_eq!(privacy.protocol, SnmpV3PrivacyProtocol::Aes256);
        assert_eq!(privacy.passphrase, "p");
        assert!(resolved.reused_secret);
        // `privacy: null` reste la façon explicite de passer en authNoPriv.
        let resolved = resolve(
            r#"{"type":"snmp_v3","username":"monitor","auth":{"protocol":"sha256"},"privacy":null}"#,
            &snmp_v3(),
        )
        .unwrap();
        let Credential::SnmpV3 { auth, privacy, .. } = &resolved.credential else { panic!() };
        assert_eq!(auth.as_ref().unwrap().passphrase, "a");
        assert!(privacy.is_none());
        // Autre utilisateur : les phrases ne suivent pas.
        let err = resolve(
            r#"{"type":"snmp_v3","username":"autre","auth":{"protocol":"sha256"}}"#,
            &snmp_v3(),
        )
        .err()
        .unwrap();
        assert!(err.contains("Enter the authentication passphrase again"), "{err}");
    }

    #[test]
    fn une_community_vide_est_gardee() {
        let saved = Credential::SnmpCommunity { community: "s3cr3t".into() };
        let resolved = resolve(r#"{"type":"snmp_community"}"#, &saved).unwrap();
        assert_eq!(resolved.credential, saved);
        assert!(resolved.reused_secret);
        // Un autre type enregistré : rien à reprendre.
        assert!(resolve(r#"{"type":"snmp_community"}"#, &proxmox()).is_err());
    }

    /// Garde-fou : la partie publique ne contient jamais un secret.
    #[test]
    fn la_partie_publique_ne_contient_aucun_secret() {
        let cases = [
            (Credential::SnmpCommunity { community: "s3cr3t".into() }, vec![]),
            (
                snmp_v3(),
                vec![
                    ("auth_protocol", "sha256"),
                    ("privacy_protocol", "aes128"),
                    ("username", "monitor"),
                ],
            ),
            (proxmox(), vec![("token_id", "dumbmonit@pve!monitor")]),
            // Un jeton porteur avec `=` n'est jamais coupé.
            (Credential::ApiToken { token: "s3cr3t==".into() }, vec![]),
            (Credential::ApiToken { token: "a@b=s3cr3t".into() }, vec![]),
            (
                Credential::UsernamePassword {
                    username: "root@pam".into(),
                    password: "s3cr3t".into(),
                },
                vec![("username", "root@pam")],
            ),
            (Credential::None, vec![]),
        ];
        for (credential, expected) in cases {
            let public = credential.public_fields();
            let got: Vec<(&str, &str)> = public.iter().map(|(k, v)| (*k, v.as_str())).collect();
            assert_eq!(got, expected, "{credential:?}");
            for value in public.values() {
                assert!(!value.contains("s3cr3t") && !value.contains("8f3a"), "{value}");
            }
        }
        assert!(!Credential::None.has_secret());
        assert!(proxmox().has_secret());
    }
}
