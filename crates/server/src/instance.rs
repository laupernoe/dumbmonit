//! Réglages d'instance choisis au premier lancement : nom, langue et fuseau
//! par défaut, et la fin de l'assistant de démarrage.
//!
//! Le tout est rangé sous une seule clé JSON de la table `settings`. Une
//! instance antérieure à ce réglage a été marquée par la migration 0180
//! (assistant terminé) pour ne pas se voir proposer un tutoriel de premier
//! lancement le jour de la mise à jour.
//!
//! Il n'y a plus qu'une interface. L'ancien champ `product_mode`
//! (`homelab` / `enterprise`), encore présent dans les bases et les
//! sauvegardes d'avant, est simplement ignoré à la lecture (serde laisse
//! passer les champs inconnus) et n'est plus réécrit.

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

/// Ce qui est écrit en base, et ce qui voyage dans la sauvegarde.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstanceSettings {
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
    fn the_default_has_the_wizard_ahead() {
        let value = InstanceSettings::default();
        assert!(!value.setup_completed);
        assert!(value.name.is_empty());
    }

    #[test]
    fn a_partial_record_reads_with_defaults() {
        let value: InstanceSettings = serde_json::from_str(r#"{"setup_completed":true}"#).unwrap();
        assert!(value.setup_completed);
        assert!(value.name.is_empty());
    }

    #[test]
    fn the_old_product_mode_is_read_and_dropped() {
        let value: InstanceSettings = serde_json::from_str(
            r#"{"product_mode":"enterprise","setup_completed":true,"name":"Lab"}"#,
        )
        .unwrap();
        assert!(value.setup_completed);
        assert_eq!(value.name, "Lab");
        let json = serde_json::to_value(&value).unwrap();
        assert!(json.get("product_mode").is_none(), "{json}");
    }
}
