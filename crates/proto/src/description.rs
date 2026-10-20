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
    /// Prérequis de l'installation (et non de l'équipement) : ce qu'il faut ajouter
    /// au déploiement du serveur pour que ce type fonctionne pleinement. Absent
    /// du JSON quand il n'y en a aucun.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requirements: Vec<Requirement>,
}

/// Un prérequis côté serveur : un composant ou un droit que le déploiement doit
/// fournir (navigateur pour les captures, socket ICMP pour le ping).
///
/// Le serveur ne donne que des identifiants et des valeurs à copier ; les phrases
/// affichées sont traduites par l'interface d'après `id`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Requirement {
    /// Identifiant stable (`browser`, `icmp`), clé des textes de l'interface.
    pub id: Text,
    /// Vrai si le serveur a constaté que le prérequis est rempli.
    pub satisfied: bool,
    /// Section de la documentation qui l'explique.
    pub doc_url: Text,
    /// Bloc YAML à ajouter au fichier Compose, tel quel.
    pub compose_snippet: Text,
}

/// Notice de mise en route affichée à côté du formulaire.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetupDescription {
    pub title: Text,
    /// Une phrase par étape ; ce qui suit un saut de ligne est à copier tel quel.
    pub steps: Vec<Text>,
    /// Droits, rôles ou permissions à donner au compte de supervision, un par
    /// entrée : la première ligne est le droit (affiché en gras), la suivante,
    /// facultative, dit en quelques mots à quoi il sert. Absent du JSON quand le
    /// type n'en demande aucun.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permissions: Vec<Text>,
    pub warning: Text,
    /// Lien « Guide complet » : la page de la documentation, ou le lien d'un paquet.
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
    /// Sous-titre regroupant ce réglage avec d'autres dans « Plus d'options » ;
    /// vide : l'interface retombe sur un classement générique par préfixe de
    /// clé. Permet aux types avec beaucoup de réglages (Proxmox VE…) de les
    /// présenter sous quelques titres plutôt qu'en une seule colonne.
    #[serde(default)]
    pub group: Text,
    /// `checklist` : les cases à cocher. La valeur est la liste des `value` cochées,
    /// séparées par des virgules ; `-` quand aucune ne l'est ; absente : tout est
    /// coché. `default` (même format) est ce qu'un nouveau moniteur coche d'emblée.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<OptionItem>,
    /// `rows` : les colonnes d'une ligne que l'on ajoute à volonté. La valeur est
    /// un tableau JSON d'objets `{clé de colonne: valeur}`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub columns: Vec<OptionColumn>,
}

/// Une case à cocher d'un réglage `checklist`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptionItem {
    pub value: Text,
    pub label: Text,
    /// Une phrase qui dit à quoi sert l'entrée.
    pub help: Text,
    /// Courtes mentions affichées à côté du nom (type, importance…).
    pub badges: Vec<Text>,
}

/// Une colonne d'un réglage `rows`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptionColumn {
    pub key: Text,
    pub label: Text,
    pub placeholder: Text,
    /// `text`, `select` ou `boolean`.
    pub input: Text,
    /// Valeurs proposées quand `input == "select"` ; la première est le défaut.
    pub choices: Vec<Text>,
    /// Expression régulière (syntaxe JavaScript) que le texte saisi doit vérifier ;
    /// vide : aucune vérification côté interface.
    pub pattern: Text,
    /// Message affiché quand `pattern` n'est pas vérifié.
    pub invalid: Text,
    /// Deux lignes ne peuvent avoir la même valeur dans cette colonne.
    pub unique: bool,
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
