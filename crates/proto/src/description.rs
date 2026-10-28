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
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Un texte des tables compilées, ou lu à l'exécution.
pub type Text = Cow<'static, str>;

// Sans `Eq` : les bornes des mesures (`MetricDescription::min`) sont des flottants.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
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
    /// Familles de mesures émises par ce type : unité, nature, sens d'un bon
    /// chiffre. Absent du JSON quand le type n'en déclare aucune ; l'interface
    /// devine alors d'après les suffixes des noms.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metrics: Vec<MetricDescription>,
    /// Vues recommandées pour la page de détail, dans l'ordre d'affichage. Absent
    /// du JSON quand le type n'en déclare aucune : l'interface trace alors tout.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub views: Vec<ViewDescription>,
    /// Prérequis de l'installation (et non de l'équipement) : ce qu'il faut ajouter
    /// au déploiement du serveur pour que ce type fonctionne pleinement. Absent
    /// du JSON quand il n'y en a aucun.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requirements: Vec<Requirement>,
}

/// Ce qu'un collecteur déclare de ses mesures : les familles et les vues qui les
/// montrent. C'est la part de [`KindDescription`] qu'un type compilé ne tient pas
/// des tables du serveur mais de son collecteur (`Collector::measures`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Measures {
    #[serde(default)]
    pub metrics: Vec<MetricDescription>,
    #[serde(default)]
    pub views: Vec<ViewDescription>,
}

/// Une famille de mesures, telle que le collecteur l'émet.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricDescription {
    /// Nom émis par le collecteur, sans le préfixe `dumbmonit_` que le serveur
    /// ajoute à l'écriture (`if_octets_in` est servi en `dumbmonit_if_octets_in`).
    pub name: Text,
    /// Libellé court, en anglais comme le reste du catalogue.
    #[serde(default, skip_serializing_if = "str::is_empty")]
    pub label: Text,
    /// `%`, `s`, `ms`, `B`, `B/s`, `°C`, `V`, `W`, `bool`, `days`, `timestamp`…
    /// Vide : un nombre sans unité. Pour un compteur, l'unité de la valeur
    /// cumulée (`B`) : l'interface trace son taux (`B/s`).
    #[serde(default, skip_serializing_if = "str::is_empty")]
    pub unit: Text,
    /// `gauge` (valeur du moment), `counter` (cumul qui ne fait que croître,
    /// tracé en taux), `info` (valeur 1, le sens est dans les étiquettes) ou
    /// `state` (code dont `states` donne le sens).
    #[serde(default = "gauge")]
    pub kind: Text,
    /// `higher` ou `lower` : le sens d'un bon chiffre ; vide quand il n'y en a pas.
    #[serde(default, skip_serializing_if = "str::is_empty")]
    pub better: Text,
    /// Bornes connues (0 et 100 pour un pourcentage).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    /// Famille qui donne le maximum de chaque série (`if_speed_bps` pour un débit).
    #[serde(default, skip_serializing_if = "str::is_empty")]
    pub max_from: Text,
    /// Étiquette qui nomme l'objet mesuré : `ifname`, `disk`, `vm`, `pool`…
    #[serde(default, skip_serializing_if = "str::is_empty")]
    pub entity: Text,
    /// Pour `kind == "state"` : le mot et le ton de chaque code.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub states: Vec<StateMap>,
}

/// Le sens d'un code d'état (`1` → « Serving », ton `ok`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateMap {
    pub value: f64,
    pub label: Text,
    /// `ok`, `warning`, `critical` ou `neutral`.
    #[serde(default, skip_serializing_if = "str::is_empty")]
    pub tone: Text,
}

/// Une vue recommandée : ce que la page de détail montre, et de quelles familles.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewDescription {
    /// `stat` (rangée de chiffres), `timeseries`, `topn`, `heatmap`, `gauge`,
    /// `table`… ; l'interface rend une vue inconnue en courbes.
    pub view: Text,
    #[serde(default, skip_serializing_if = "str::is_empty")]
    pub title: Text,
    /// Familles consommées, par leur `MetricDescription::name`.
    pub metrics: Vec<Text>,
    /// Étiquette qui sépare les entités (`topn`, `heatmap`) ; vide : chaque série.
    #[serde(default, skip_serializing_if = "str::is_empty")]
    pub group_by: Text,
    /// Réglages propres à la vue (`limit = "10"`…).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub options: BTreeMap<String, String>,
}

fn gauge() -> Text {
    Cow::Borrowed("gauge")
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
    /// Ports réseau à ouvrir entre le serveur (ou l'agent relais) et l'équipement.
    /// Absent du JSON quand le type n'en demande aucun.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ports: Vec<PortDescription>,
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

/// Un port réseau à ouvrir pour que le type fonctionne.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortDescription {
    pub port: u16,
    /// `tcp` ou `udp`.
    pub protocol: Text,
    /// `to_device` : du serveur DumbMonit (ou de l'agent relais) vers l'équipement ;
    /// `to_server` : de l'équipement (l'agent) vers le serveur.
    pub direction: Text,
    /// Ce qui passe par ce port, en quelques mots (« LDAPS », « API »).
    pub purpose: Text,
    /// Vrai si le port ne sert que pour une option ou un mode ; `purpose` le dit.
    pub optional: bool,
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
