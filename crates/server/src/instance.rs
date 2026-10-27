//! Réglages d'instance choisis au premier lancement : mode produit, nom,
//! langue et fuseau par défaut, et la fin de l'assistant de démarrage.
//!
//! Le **mode produit** ne change que la présentation : `homelab` simplifie
//! l'interface (navigation réduite, réglages avancés repliés), `enterprise`
//! ouvre l'interface dense et sombre avec les options avancées en vue. Aucune
//! donnée n'en dépend côté serveur, et passer de l'un à l'autre ne supprime
//! rien : c'est pour cela qu'il tient dans un simple réglage.
//!
//! Le tout est rangé sous une seule clé JSON de la table `settings`. Une
//! instance antérieure à ce réglage a été marquée par la migration 0180
//! (`homelab`, assistant terminé) pour ne pas se voir proposer un tutoriel de
//! premier lancement le jour de la mise à jour.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::auth::settings;

/// Clé du réglage dans la table `settings`.
pub const KEY: &str = "instance";

/// Longueur maximale du nom d'instance, en caractères.
pub const MAX_NAME_LEN: usize = 80;

/// Langues de l'interface, telles que Paraglide les nomme.
pub const LANGUAGES: [&str; 9] = ["en", "fr", "de", "es", "it", "pt", "pt-BR", "ru", "zh-Hans"];

/// Présentation choisie par l'administrateur.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProductMode {
    /// Interface simple, défaut des instances existantes.
    #[default]
    Homelab,
    /// Interface dense et sombre, options avancées en vue.
    Enterprise,
}

impl ProductMode {
    /// Lit la valeur envoyée par l'interface. `None` pour une valeur inconnue.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "homelab" => Some(Self::Homelab),
            "enterprise" => Some(Self::Enterprise),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Homelab => "homelab",
            Self::Enterprise => "enterprise",
        }
    }
}

/// Ce qui est écrit en base, et ce qui voyage dans la sauvegarde.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstanceSettings {
    #[serde(default)]
    pub product_mode: ProductMode,
    /// Vrai une fois l'assistant de démarrage terminé (ou passé en entier).
    #[serde(default)]
    pub setup_completed: bool,
    /// Date de fin de l'assistant, UTC sans suffixe.
    #[serde(default)]
    pub setup_completed_at: Option<String>,
    /// Nom affiché de l'instance. Vide : « DumbMonit ».
    #[serde(default)]
    pub name: String,
    /// Langue proposée par défaut aux nouveaux navigateurs. Vide : celle du navigateur.
    #[serde(default)]
    pub language: String,
    /// Fuseau IANA de l'instance (`Europe/Paris`). Vide : UTC.
    #[serde(default)]
    pub timezone: String,
}

/// Lit les réglages. Un réglage absent ou illisible rend les valeurs par
/// défaut : l'interface doit toujours pouvoir se charger.
pub async fn load(pool: &SqlitePool) -> InstanceSettings {
    match settings::get::<InstanceSettings>(pool, KEY).await {
        Ok(Some(stored)) => stored,
        Ok(None) => InstanceSettings::default(),
        Err(error) => {
            tracing::warn!(?error, "instance settings unreadable, defaults used");
            InstanceSettings::default()
        }
    }
}

/// Lit les réglages tels qu'ils sont en base, `None` s'ils n'ont jamais été
/// écrits. Sert à la sauvegarde, qui n'invente pas de section vide.
pub async fn stored(pool: &SqlitePool) -> Result<Option<InstanceSettings>> {
    settings::get::<InstanceSettings>(pool, KEY).await
}

pub async fn save(pool: &SqlitePool, value: &InstanceSettings) -> Result<()> {
    settings::set(pool, KEY, value).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_homelab_with_the_wizard_ahead() {
        let value = InstanceSettings::default();
        assert_eq!(value.product_mode, ProductMode::Homelab);
        assert!(!value.setup_completed);
    }

    #[test]
    fn the_mode_travels_in_lower_case() {
        let json = serde_json::to_value(InstanceSettings {
            product_mode: ProductMode::Enterprise,
            ..InstanceSettings::default()
        })
        .unwrap();
        assert_eq!(json["product_mode"], "enterprise");
    }

    #[test]
    fn a_partial_record_reads_with_defaults() {
        let value: InstanceSettings = serde_json::from_str(r#"{"setup_completed":true}"#).unwrap();
        assert_eq!(value.product_mode, ProductMode::Homelab);
        assert!(value.setup_completed);
        assert!(value.name.is_empty());
    }

    #[test]
    fn unknown_modes_are_refused() {
        assert_eq!(ProductMode::parse("enterprise"), Some(ProductMode::Enterprise));
        assert_eq!(ProductMode::parse(" homelab "), Some(ProductMode::Homelab));
        assert_eq!(ProductMode::parse("pro"), None);
    }
}
