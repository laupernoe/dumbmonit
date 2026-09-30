//! Description d'un type d'équipement, telle que `GET /api/collectors` la sert.
//!
//! C'est le format envoyé à l'interface pour tous les types. Ceux qui sont
//! compilés dans le serveur sont écrits dans ses propres tables
//! (`api/collectors.rs`) puis convertis sans copier leurs textes
//! ([`Text::Borrowed`]) ; un collecteur défini à l'exécution — un paquet
//! d'intégration, par exemple — n'y figure pas : il se décrit lui-même avec ces
//! structures ([`Text::Owned`]), via
//! [`Collector::description`](crate::Collector::description).
//!
//! L'ordre des champs est celui du JSON attendu par l'interface : ne pas le changer.

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

/// Un texte des tables compilées, ou lu à l'exécution.
pub type Text = Cow<'static, str>;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KindDescription {
    /// Valeur du champ `kind` d'une cible. Le serveur la remplace par le type sous
    /// lequel le collecteur est enregistré.
    pub kind: Text,
    /// Libellé destiné à l'affichage.
    pub label: Text,
    /// Une phrase disant à quoi sert ce type, pour la liste de choix.
    pub summary: Text,
    /// Exemples d'équipements concernés.
    pub examples: Vec<Text>,
    /// Les `kind` de `credentials`, dans l'ordre de préférence.
    pub credential_types: Vec<Text>,
    /// Les formes d'identifiant acceptées, champ par champ.
    pub credentials: Vec<CredentialDescription>,
    /// Adresse d'exemple, texte indicatif du champ.
    pub address_hint: Text,
    /// Port par défaut ; 0 s'il n'y en a pas.
    pub default_port: u16,
    pub setup: SetupDescription,
    /// Réglages lus dans `Target::tags`, dans l'ordre d'affichage.
    pub options: Vec<OptionDescription>,
}

/// Notice de mise en route affichée à côté du formulaire.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetupDescription {
    pub title: Text,
    /// Une phrase par étape ; ce qui suit un saut de ligne est à copier tel quel.
    pub steps: Vec<Text>,
    pub warning: Text,
    pub doc_url: Text,
}

/// Un réglage lu dans `Target::tags[key]`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptionDescription {
    pub key: Text,
    pub label: Text,
    pub help: Text,
    pub placeholder: Text,
    pub default: Text,
    pub required: bool,
    /// `text`, `number`, `boolean`, `select`…
    pub input: Text,
    pub choices: Vec<Text>,
}

/// Une forme d'identifiant acceptée (le `type` de `credential`), et ses champs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialDescription {
    pub kind: Text,
    pub label: Text,
    pub help: Text,
    pub fields: Vec<CredentialFieldDescription>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialFieldDescription {
    pub key: Text,
    pub label: Text,
    pub help: Text,
    pub placeholder: Text,
    /// `text`, `password` ou `select`.
    pub input: Text,
    pub choices: Vec<Text>,
    pub required: bool,
}
