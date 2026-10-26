//! Réglages de stockage : rétentions et interrupteurs, enregistrés dans
//! `app_settings`, avec repli sur l'environnement puis sur les défauts.

use anyhow::Result;
use serde::Serialize;
use sqlx::SqlitePool;

use crate::config::Config;

pub const METRICS_RETENTION_KEY: &str = "storage_metrics_retention_days";
pub const ROLLUP_ENABLED_KEY: &str = "storage_rollup_enabled";
pub const ROLLUP_RETENTION_KEY: &str = "storage_rollup_retention_days";
pub const LOGS_ENABLED_KEY: &str = "storage_logs_enabled";
pub const LOGS_RETENTION_KEY: &str = "storage_logs_retention_days";

/// Bornes des rétentions, en jours. Sept jours au moins en brut : en dessous,
/// les alertes de référence (apprentissage sur quatorze jours) n'ont plus de
/// quoi apprendre que dans les agrégats.
pub const MIN_RETENTION_DAYS: u32 = 7;
pub const MAX_RETENTION_DAYS: u32 = 3_650;
pub const MIN_ROLLUP_RETENTION_DAYS: u32 = 30;
pub const MAX_ROLLUP_RETENTION_DAYS: u32 = 3_650;
pub const MIN_LOGS_RETENTION_DAYS: u32 = 1;
pub const MAX_LOGS_RETENTION_DAYS: u32 = 3_650;
pub const DEFAULT_ROLLUP_RETENTION_DAYS: u32 = 730;
pub const DEFAULT_LOGS_RETENTION_DAYS: u32 = 30;

/// D'où vient une valeur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Settings,
    Environment,
    Default,
}

/// Une rétention : en jours pour l'interface, et telle que le processus la
/// reçoit (`-retentionPeriod`). Une valeur de l'environnement est transmise
/// telle quelle — `12` veut dire douze mois pour VictoriaMetrics, et la
/// convertir en jours raccourcirait de quelques jours une rétention existante.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Retention {
    pub days: u32,
    pub flag: String,
    pub source: Source,
}

impl Retention {
    fn from_days(days: u32, source: Source) -> Self {
        Self { days, flag: format!("{days}d"), source }
    }
}

/// Réglages effectifs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageSettings {
    pub metrics: Retention,
    pub rollup_enabled: bool,
    pub rollup: Retention,
    pub logs_enabled: bool,
    pub logs: Retention,
}

/// Durée `-retentionPeriod` en jours : `30d`, `2w`, `2y`, `720h`, ou un nombre
/// nu de mois (la convention de VictoriaMetrics). `None` si illisible.
pub fn parse_retention_days(raw: &str) -> Option<u32> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let split = raw.find(|c: char| !c.is_ascii_digit() && c != '.').unwrap_or(raw.len());
    let (number, unit) = raw.split_at(split);
    let number: f64 = number.parse().ok()?;
    let days = match unit {
        "" | "M" => number * 365.0 / 12.0,
        "h" => number / 24.0,
        "d" => number,
        "w" => number * 7.0,
        "y" => number * 365.0,
        _ => return None,
    };
    (days >= 1.0 && days.is_finite()).then(|| days.round().min(u32::MAX as f64) as u32)
}

/// Une valeur de l'environnement : gardée telle quelle si elle se lit, sinon
/// le défaut.
fn from_env(raw: &str, from_env: bool, default_days: u32) -> Retention {
    match parse_retention_days(raw) {
        Some(days) => Retention {
            days,
            flag: raw.trim().to_string(),
            source: if from_env { Source::Environment } else { Source::Default },
        },
        None => Retention::from_days(default_days, Source::Default),
    }
}

fn stored_days(value: Option<&str>, min: u32, max: u32) -> Option<u32> {
    value.and_then(|v| v.trim().parse::<u32>().ok()).filter(|days| (min..=max).contains(days))
}

fn stored_flag(value: Option<&str>) -> Option<bool> {
    match value.map(str::trim) {
        Some("on" | "true" | "1") => Some(true),
        Some("off" | "false" | "0") => Some(false),
        _ => None,
    }
}

/// Résout les réglages à partir des valeurs enregistrées (`lookup`) et de la
/// configuration d'environnement.
pub fn resolve(config: &Config, lookup: impl Fn(&str) -> Option<String>) -> StorageSettings {
    let metrics = match stored_days(
        lookup(METRICS_RETENTION_KEY).as_deref(),
        MIN_RETENTION_DAYS,
        MAX_RETENTION_DAYS,
    ) {
        Some(days) => Retention::from_days(days, Source::Settings),
        None => from_env(&config.vm_retention, config.vm_retention_from_env, 365),
    };
    let rollup = match stored_days(
        lookup(ROLLUP_RETENTION_KEY).as_deref(),
        MIN_ROLLUP_RETENTION_DAYS,
        MAX_ROLLUP_RETENTION_DAYS,
    ) {
        Some(days) => Retention::from_days(days, Source::Settings),
        None => from_env(
            &config.vm_long_retention,
            config.vm_long_retention_from_env,
            DEFAULT_ROLLUP_RETENTION_DAYS,
        ),
    };
    let logs = match stored_days(
        lookup(LOGS_RETENTION_KEY).as_deref(),
        MIN_LOGS_RETENTION_DAYS,
        MAX_LOGS_RETENTION_DAYS,
    ) {
        Some(days) => Retention::from_days(days, Source::Settings),
        None => Retention::from_days(DEFAULT_LOGS_RETENTION_DAYS, Source::Default),
    };
    StorageSettings {
        metrics,
        rollup_enabled: stored_flag(lookup(ROLLUP_ENABLED_KEY).as_deref()).unwrap_or(true),
        rollup,
        logs_enabled: stored_flag(lookup(LOGS_ENABLED_KEY).as_deref()).unwrap_or(false),
        logs,
    }
}

/// Lit les réglages en base.
pub async fn load(pool: &SqlitePool, config: &Config) -> Result<StorageSettings> {
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT key, value FROM app_settings WHERE key LIKE 'storage_%'")
            .fetch_all(pool)
            .await?;
    Ok(resolve(config, |key| rows.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())))
}

pub async fn read(pool: &SqlitePool, key: &str) -> Result<Option<String>> {
    Ok(sqlx::query_scalar("SELECT value FROM app_settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?)
}

pub async fn write(pool: &SqlitePool, key: &str, value: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO app_settings (key, value) VALUES (?, ?) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}

/// Vérifie une rétention demandée par l'interface.
pub fn check_days(what: &str, days: u32, min: u32, max: u32) -> Result<u32, String> {
    if (min..=max).contains(&days) {
        Ok(days)
    } else {
        Err(format!("The {what} retention must be between {min} and {max} days."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retention_periods_read_like_victoriametrics() {
        assert_eq!(parse_retention_days("12"), Some(365));
        assert_eq!(parse_retention_days("1"), Some(30));
        assert_eq!(parse_retention_days("30d"), Some(30));
        assert_eq!(parse_retention_days("2w"), Some(14));
        assert_eq!(parse_retention_days("2y"), Some(730));
        assert_eq!(parse_retention_days("720h"), Some(30));
        assert_eq!(parse_retention_days(" 90d "), Some(90));
        assert_eq!(parse_retention_days("1h"), None, "moins d'un jour");
        assert_eq!(parse_retention_days("abc"), None);
        assert_eq!(parse_retention_days("10x"), None);
        assert_eq!(parse_retention_days(""), None);
    }

    fn config() -> Config {
        let mut config = Config::from_env().unwrap();
        config.vm_retention = "12".into();
        config.vm_retention_from_env = false;
        config.vm_long_retention = "730d".into();
        config.vm_long_retention_from_env = false;
        config
    }

    #[test]
    fn defaults_keep_the_environment_value_untouched() {
        let settings = resolve(&config(), |_| None);
        assert_eq!(settings.metrics.days, 365);
        // Transmis tel quel : douze mois, pas 365 jours.
        assert_eq!(settings.metrics.flag, "12");
        assert_eq!(settings.metrics.source, Source::Default);
        assert!(settings.rollup_enabled);
        assert_eq!(settings.rollup.days, 730);
        assert!(!settings.logs_enabled, "VictoriaLogs reste coupé par défaut");
        assert_eq!(settings.logs.days, 30);
    }

    #[test]
    fn the_environment_is_named_as_the_source() {
        let mut config = config();
        config.vm_retention = "90d".into();
        config.vm_retention_from_env = true;
        let settings = resolve(&config, |_| None);
        assert_eq!(
            settings.metrics,
            Retention { days: 90, flag: "90d".into(), source: Source::Environment }
        );
    }

    #[test]
    fn stored_settings_win_and_bad_values_are_ignored() {
        let settings = resolve(&config(), |key| match key {
            METRICS_RETENTION_KEY => Some("30".into()),
            ROLLUP_ENABLED_KEY => Some("off".into()),
            ROLLUP_RETENTION_KEY => Some("5".into()), // sous le minimum : ignoré
            LOGS_ENABLED_KEY => Some("on".into()),
            LOGS_RETENTION_KEY => Some("n'importe quoi".into()),
            _ => None,
        });
        assert_eq!(settings.metrics.flag, "30d");
        assert_eq!(settings.metrics.source, Source::Settings);
        assert!(!settings.rollup_enabled);
        assert_eq!(settings.rollup.days, 730);
        assert!(settings.logs_enabled);
        assert_eq!(settings.logs.days, DEFAULT_LOGS_RETENTION_DAYS);
    }

    #[test]
    fn requested_retentions_are_bounded() {
        assert!(check_days("raw", 30, MIN_RETENTION_DAYS, MAX_RETENTION_DAYS).is_ok());
        assert!(check_days("raw", 3, MIN_RETENTION_DAYS, MAX_RETENTION_DAYS).is_err());
        assert!(check_days("raw", 4_000, MIN_RETENTION_DAYS, MAX_RETENTION_DAYS).is_err());
    }
}
