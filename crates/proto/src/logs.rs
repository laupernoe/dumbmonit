//! Collecte des journaux par l'agent : réglage venu du serveur, lots envoyés.
//!
//! Le module est optionnel et se pilote depuis l'interface : le serveur joint le
//! réglage de la machine à l'accusé de réception de ses lots de mesures
//! ([`crate::PushAck::logs`]), et l'agent l'applique à chaud. Les lignes
//! repartent par le même canal que les mesures (même hôte, même port, même jeton),
//! sur [`LOGS_PATH`].

use serde::{Deserialize, Serialize};

/// Chemin de la route de réception des journaux, sur le canal des agents.
pub const LOGS_PATH: &str = "/api/agent/logs";

/// Nombre maximal de lignes par lot : garde-fou des deux côtés.
pub const MAX_LOG_LINES: usize = 1_000;

/// Longueur maximale d'un message, en octets ; l'agent tronque, le serveur aussi.
pub const MAX_LOG_MESSAGE: usize = 8 * 1024;

/// Une ligne de journal collectée sur la machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogLine {
    /// Instant de l'évènement, en millisecondes depuis l'époque Unix.
    pub ts_ms: i64,
    /// `error`, `warn`, `info` ou `debug`.
    pub level: String,
    /// Origine : `journald:<unité>`, `file:<chemin>`, `eventlog:<canal>`.
    pub source: String,
    pub message: String,
}

/// Un lot de lignes.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LogBatch {
    pub lines: Vec<LogLine>,
}

/// Réglage de la collecte de journaux d'une machine, tel que l'interface le fixe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LogsSetting {
    pub enabled: bool,
    /// Niveau minimal envoyé : `error`, `warn`, `info` ou `debug`.
    pub min_level: String,
    /// Lire le journal systemd (Linux).
    pub journald: bool,
    /// Fichiers suivis, avec jokers (`/var/log/nginx/*.log`), rotation comprise.
    pub files: Vec<String>,
    /// Canaux du Journal d'événements Windows (`System`, `Application`…).
    pub eventlog: Vec<String>,
}

impl Default for LogsSetting {
    fn default() -> Self {
        Self {
            enabled: false,
            min_level: "warn".to_string(),
            journald: true,
            files: Vec::new(),
            eventlog: Vec::new(),
        }
    }
}

/// Rang d'un niveau, du plus grave (0) au plus bavard (3). Un niveau inconnu
/// compte comme `info`.
pub fn level_rank(level: &str) -> u8 {
    match level {
        "error" | "err" | "critical" | "crit" | "alert" | "emerg" | "fatal" => 0,
        "warn" | "warning" => 1,
        "debug" | "trace" => 3,
        _ => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_setting_may_be_partial() {
        let setting: LogsSetting = serde_json::from_str(r#"{"enabled":true}"#).unwrap();
        assert!(setting.enabled);
        assert_eq!(setting.min_level, "warn");
        assert!(setting.journald);
        assert!(setting.files.is_empty());
    }

    #[test]
    fn levels_are_ranked_from_the_gravest() {
        assert!(level_rank("error") < level_rank("warn"));
        assert!(level_rank("warn") < level_rank("info"));
        assert!(level_rank("info") < level_rank("debug"));
        assert_eq!(level_rank("whatever"), level_rank("info"));
    }
}
