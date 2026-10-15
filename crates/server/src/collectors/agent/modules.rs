//! Services détectés par un agent, rangés chacun sous son propre équipement.
//!
//! L'agent pousse un seul lot par cycle : les mesures de la machine (processeur,
//! mémoire, disques, réseau, santé du système, SMART, capteurs) et, mêlées à
//! elles, celles des services qu'il a trouvés — Docker, Plakar, ZFS… Le serveur
//! les sépare ici, d'après le nom de la métrique : chaque service devient une
//! cible enfant de l'hôte (`kind = "agent_service"`, `parent_id` = l'hôte,
//! `profile_id` = le module), avec son statut, ses alertes et ses règles. Les
//! séries du service portent `target` = l'enfant, celles de la machine
//! `target` = l'hôte : rien n'est compté deux fois.
//!
//! Aucun changement de protocole n'est nécessaire pour cela : un agent de
//! n'importe quelle version est découpé de la même façon. Le seul ajout est
//! dans l'accusé de réception (`disabled_modules`), pour qu'un agent récent
//! cesse de collecter ce que l'utilisateur a décoché.
//!
//! L'historique d'avant la séparation reste sous l'hôte : VictoriaMetrics ne
//! permet pas de réétiqueter des séries déjà écrites.

use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, MetricKind, ProbeError, Sample, Target, TargetId};
use sqlx::{Row, SqlitePool};

use crate::alerting::model::{self, TargetSelector};
use crate::crypto::Cipher;
use crate::db;

/// Type des cibles nées d'un module d'agent.
pub const SERVICE_KIND: &str = "agent_service";

/// Un service que l'agent sait détecter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Module {
    /// Identifiant stable : le nom du module dans `agent.yaml` quand il en a un.
    pub id: &'static str,
    /// Nom affiché, et suffixe du nom de l'équipement créé.
    pub label: &'static str,
    /// Préfixes des métriques (sans `dumbmonit_`) qui appartiennent au module.
    pub prefixes: &'static [&'static str],
}

/// Les services qui deviennent des équipements. Ce qui n'y figure pas reste
/// sur l'hôte : c'est la machine elle-même (SMART, capteurs, santé du système,
/// unités systemd, compteurs Windows).
pub const MODULES: &[Module] = &[
    Module { id: "docker", label: "Docker", prefixes: &["container_"] },
    Module { id: "plakar", label: "Plakar", prefixes: &["backup_"] },
    Module { id: "backup_repos", label: "restic/Borg", prefixes: &["agent_backup_repo_"] },
    Module { id: "zfs", label: "ZFS", prefixes: &["agent_zfs_"] },
    Module { id: "libvirt", label: "libvirt", prefixes: &["agent_libvirt_"] },
    Module { id: "wireguard", label: "WireGuard", prefixes: &["agent_wireguard_"] },
    Module { id: "ipsec", label: "IPsec", prefixes: &["agent_ipsec_"] },
    Module { id: "speedtest", label: "Speed test", prefixes: &["agent_speedtest_"] },
];

/// Le module d'un identifiant, s'il existe.
pub fn find(id: &str) -> Option<&'static Module> {
    MODULES.iter().find(|module| module.id == id)
}

/// Le module auquel appartient une métrique de l'agent ; `None` : la machine.
pub fn module_of(metric: &str) -> Option<&'static Module> {
    MODULES.iter().find(|module| module.prefixes.iter().any(|prefix| metric.starts_with(prefix)))
}

/// Vrai si une requête de règle lit des métriques de ce module.
fn query_reads(query: &str, module: &Module) -> bool {
    module.prefixes.iter().any(|prefix| query.contains(&format!("dumbmonit_{prefix}")))
}

/// Un module d'une machine, tel que la base le connaît.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleState {
    pub module: String,
    /// `None` : automatique ; `Some(false)` : décoché ; `Some(true)` : voulu.
    pub enabled: Option<bool>,
    pub target_id: Option<TargetId>,
    pub last_seen_ms: Option<i64>,
}

impl ModuleState {
    /// Un appareil doit-il exister pour ce module ?
    pub fn wanted(&self) -> bool {
        match self.enabled {
            Some(on) => on,
            None => self.last_seen_ms.is_some(),
        }
    }
}

/// Les modules connus d'une machine, rangés par identifiant.
pub async fn states(pool: &SqlitePool, host: TargetId) -> Result<BTreeMap<String, ModuleState>> {
    let rows = sqlx::query(
        "SELECT module, enabled, target_id, last_seen_ms FROM agent_modules
         WHERE host_target_id = ?",
    )
    .bind(host)
    .fetch_all(pool)
    .await
    .context("reading the agent modules")?;
    rows.iter()
        .map(|row| {
            let state = ModuleState {
                module: row.try_get("module")?,
                enabled: row.try_get::<Option<i64>, _>("enabled")?.map(|v| v != 0),
                target_id: row.try_get("target_id")?,
                last_seen_ms: row.try_get("last_seen_ms")?,
            };
            Ok((state.module.clone(), state))
        })
        .collect()
}

/// Note qu'un lot contenait des mesures de ce module.
pub async fn mark_seen(pool: &SqlitePool, host: TargetId, module: &str, at_ms: i64) -> Result<()> {
    sqlx::query(
        "INSERT INTO agent_modules (host_target_id, module, last_seen_ms) VALUES (?, ?, ?)
         ON CONFLICT (host_target_id, module) DO UPDATE SET last_seen_ms = excluded.last_seen_ms",
    )
    .bind(host)
    .bind(module)
    .bind(at_ms)
    .execute(pool)
    .await
    .context("recording the module reading")?;
    Ok(())
}

/// Coche ou décoche un module. Décocher supprime son appareil ; cocher le crée,
/// même avant toute détection. Renvoie l'état obtenu, et l'appareil supprimé
/// s'il y en avait un (ses alertes et ses séries restent à oublier).
pub async fn set_enabled(
    pool: &SqlitePool,
    cipher: &Cipher,
    host: &Target,
    module: &'static Module,
    enabled: bool,
) -> Result<(ModuleState, Option<TargetId>)> {
    sqlx::query(
        "INSERT INTO agent_modules (host_target_id, module, enabled) VALUES (?, ?, ?)
         ON CONFLICT (host_target_id, module) DO UPDATE SET enabled = excluded.enabled",
    )
    .bind(host.id)
    .bind(module.id)
    .bind(i64::from(enabled))
    .execute(pool)
    .await
    .context("saving the module choice")?;

    let state = states(pool, host.id).await?.remove(module.id).context("module row")?;
    let mut removed = None;
    if enabled {
        ensure_device(pool, cipher, host, module, &state).await?;
    } else if let Some(child) = state.target_id {
        // Le déclencheur de la base remet `enabled = 0` et `target_id = NULL`.
        db::targets::delete(pool, child).await?;
        removed = Some(child);
        tracing::info!(hote = host.id, module = module.id, "module décoché, appareil supprimé");
    }
    let state = states(pool, host.id).await?.remove(module.id).context("module row")?;
    Ok((state, removed))
}

/// L'appareil du module, créé s'il manque. Renvoie son identifiant.
pub async fn ensure_device(
    pool: &SqlitePool,
    cipher: &Cipher,
    host: &Target,
    module: &'static Module,
    state: &ModuleState,
) -> Result<TargetId> {
    if let Some(id) = state.target_id {
        return Ok(id);
    }
    let address = format!("{}#{}", host.address, module.id);
    // Une cible restée sans ligne (base restaurée à moitié) est adoptée plutôt
    // que de buter sur `UNIQUE (kind, address)`.
    let existing = sqlx::query("SELECT id FROM targets WHERE kind = ? AND address = ?")
        .bind(SERVICE_KIND)
        .bind(&address)
        .fetch_optional(pool)
        .await?;
    let child = match existing {
        Some(row) => row.try_get::<TargetId, _>("id")?,
        None => {
            let input = db::targets::TargetInput {
                name: format!("{} · {}", host.name, module.label),
                address,
                kind: SERVICE_KIND.to_string(),
                profile_id: Some(module.id.to_string()),
                parent_id: Some(host.id),
                via_agent: None,
                interval: host.interval,
                enabled: true,
                // Les étiquettes de l'hôte suivent : les silences et les règles
                // qui les visent continuent de s'appliquer au service.
                tags: host.tags.clone(),
                credential: Some(Credential::None),
                group_name: host.group_name.clone(),
            };
            db::targets::create(pool, cipher, &input).await?
        }
    };
    sqlx::query("UPDATE agent_modules SET target_id = ? WHERE host_target_id = ? AND module = ?")
        .bind(child)
        .bind(host.id)
        .bind(module.id)
        .execute(pool)
        .await
        .context("linking the module to its device")?;
    carry_over(pool, host.id, child, module).await?;
    tracing::info!(hote = host.id, cible = child, module = module.id, "appareil de service créé");
    Ok(child)
}

/// Reporte sur le nouvel appareil ce qui visait l'hôte pour ce service : les
/// surcharges et sélecteurs des règles qui lisent ses métriques, les silences
/// de l'hôte, et les alertes en cours sur ses séries — reprises sous leur
/// nouvelle empreinte plutôt que résolues puis redéclenchées.
///
/// C'est ce qui fait passer une installation existante à la séparation sans
/// qu'aucun réglage ne se perde : la première fois que le module est vu après
/// la mise à jour, son appareil naît avec ce que l'hôte avait.
async fn carry_over(
    pool: &SqlitePool,
    host: TargetId,
    child: TargetId,
    module: &Module,
) -> Result<()> {
    let mut tx = pool.begin().await?;

    let rules = sqlx::query("SELECT id, uid, query, selector FROM alert_rules")
        .fetch_all(&mut *tx)
        .await
        .context("reading the rules")?;
    for row in &rules {
        let query: String = row.try_get("query")?;
        if !query_reads(&query, module) {
            continue;
        }
        let uid: String = row.try_get("uid")?;
        sqlx::query(
            "INSERT OR IGNORE INTO rule_overrides
                 (rule_uid, target_id, threshold, clear_threshold, enabled)
             SELECT rule_uid, ?, threshold, clear_threshold, enabled
             FROM rule_overrides WHERE rule_uid = ? AND target_id = ?",
        )
        .bind(child)
        .bind(&uid)
        .bind(host)
        .execute(&mut *tx)
        .await
        .context("copying the rule overrides")?;

        let selector: String = row.try_get("selector")?;
        if let Ok(TargetSelector::Ids { mut ids }) = serde_json::from_str(&selector)
            && ids.contains(&host)
            && !ids.contains(&child)
        {
            ids.push(child);
            sqlx::query("UPDATE alert_rules SET selector = ? WHERE id = ?")
                .bind(serde_json::to_string(&TargetSelector::Ids { ids })?)
                .bind(row.try_get::<i64, _>("id")?)
                .execute(&mut *tx)
                .await
                .context("extending the rule selector")?;
        }

        reassign_alerts(&mut tx, &uid, host, child).await?;
    }

    sqlx::query(
        "INSERT INTO silences (name, comment, target_id, matchers, schedule, enabled)
         SELECT name, comment, ?, matchers, schedule, enabled FROM silences WHERE target_id = ?",
    )
    .bind(child)
    .bind(host)
    .execute(&mut *tx)
    .await
    .context("copying the silences")?;

    tx.commit().await?;
    Ok(())
}

/// Déplace les alertes d'une règle de l'hôte vers le service.
async fn reassign_alerts(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    rule_uid: &str,
    host: TargetId,
    child: TargetId,
) -> Result<()> {
    let rows = sqlx::query(
        "SELECT fingerprint, labels FROM alert_state WHERE rule_uid = ? AND target_id = ?",
    )
    .bind(rule_uid)
    .bind(host)
    .fetch_all(&mut **tx)
    .await
    .context("reading the alerts to move")?;
    for row in rows {
        let old: String = row.try_get("fingerprint")?;
        let labels: String = row.try_get("labels")?;
        let mut labels: BTreeMap<String, String> =
            serde_json::from_str(&labels).unwrap_or_default();
        for key in model::TARGET_ID_LABELS {
            if let Some(value) = labels.get_mut(key) {
                *value = child.to_string();
            }
        }
        let series_key = model::series_key(&labels);
        let fingerprint = model::fingerprint(rule_uid, &model::identity_key(&labels));
        sqlx::query(
            "UPDATE OR IGNORE alert_state
             SET fingerprint = ?, target_id = ?, labels = ?, series_key = ?
             WHERE fingerprint = ?",
        )
        .bind(&fingerprint)
        .bind(child)
        .bind(serde_json::to_string(&labels)?)
        .bind(&series_key)
        .bind(&old)
        .execute(&mut **tx)
        .await
        .context("moving an alert")?;
        for statement in [
            "UPDATE OR IGNORE notify_queue SET fingerprint = ? WHERE fingerprint = ?",
            "UPDATE OR IGNORE notify_flap SET fingerprint = ? WHERE fingerprint = ?",
        ] {
            sqlx::query(statement)
                .bind(&fingerprint)
                .bind(&old)
                .execute(&mut **tx)
                .await
                .context("moving the pending notifications")?;
        }
    }
    Ok(())
}

/// Identifiants des appareils de service, chacun avec son hôte.
pub async fn service_hosts(pool: &SqlitePool) -> Result<BTreeMap<TargetId, TargetId>> {
    let rows = sqlx::query(
        "SELECT target_id, host_target_id FROM agent_modules WHERE target_id IS NOT NULL",
    )
    .fetch_all(pool)
    .await
    .context("reading the service devices")?;
    rows.iter().map(|row| Ok((row.try_get("target_id")?, row.try_get("host_target_id")?))).collect()
}

/// L'hôte et le module d'un appareil de service.
pub async fn owner(pool: &SqlitePool, child: TargetId) -> Result<Option<(TargetId, String)>> {
    let row = sqlx::query("SELECT host_target_id, module FROM agent_modules WHERE target_id = ?")
        .bind(child)
        .fetch_optional(pool)
        .await
        .context("reading the service owner")?;
    row.map(|row| Ok((row.try_get("host_target_id")?, row.try_get("module")?))).transpose()
}

/// Les modules décochés d'une machine, pour l'accusé de réception.
pub fn disabled(states: &BTreeMap<String, ModuleState>) -> Vec<String> {
    states.values().filter(|s| s.enabled == Some(false)).map(|s| s.module.clone()).collect()
}

/// Fraîcheur d'un appareil de service : le module a-t-il été vu récemment ?
///
/// Même logique que pour l'hôte ([`super::AgentCollector`]) : un service dont
/// l'agent ne rapporte plus rien (démon Docker arrêté, `wg` disparu) est
/// injoignable. Si l'hôte lui-même se tait, l'alerte du service est supprimée
/// par celle de son parent.
pub struct AgentServiceCollector {
    pool: SqlitePool,
}

impl AgentServiceCollector {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl Collector for AgentServiceCollector {
    fn kind(&self) -> &'static str {
        SERVICE_KIND
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let row = sqlx::query("SELECT last_seen_ms FROM agent_modules WHERE target_id = ?")
            .bind(target.id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ProbeError::Other(e.into()))?;
        let last_seen: Option<i64> = match row {
            Some(row) => row.try_get("last_seen_ms").map_err(|e| ProbeError::Other(e.into()))?,
            None => None,
        };
        let Some(last_seen) = last_seen else {
            return Err(ProbeError::Unreachable(
                "the agent has not reported this service on its machine".to_string(),
            ));
        };
        let now_ms = chrono::Utc::now().timestamp_millis();
        let age = Duration::from_millis(now_ms.saturating_sub(last_seen).max(0) as u64);
        let deadline = super::staleness_deadline(target.interval);
        if age > deadline {
            return Err(ProbeError::Unreachable(format!(
                "the agent stopped reporting this service {} s ago (limit {} s)",
                age.as_secs(),
                deadline.as_secs()
            )));
        }
        Ok(vec![Sample::new(
            "agent_last_batch_age_seconds",
            age.as_secs_f64(),
            MetricKind::Gauge,
            now_ms,
        )])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_service_metric_finds_its_module_and_machine_metrics_stay_on_the_host() {
        let module = |metric| module_of(metric).map(|m| m.id);
        assert_eq!(module("container_up"), Some("docker"));
        assert_eq!(module("backup_last_status"), Some("plakar"));
        assert_eq!(module("agent_backup_repo_reachable"), Some("backup_repos"));
        assert_eq!(module("agent_zfs_pool_health"), Some("zfs"));
        assert_eq!(module("agent_libvirt_domain_running"), Some("libvirt"));
        assert_eq!(module("agent_wireguard_peer_rx_bytes"), Some("wireguard"));
        assert_eq!(module("agent_ipsec_tunnel_up"), Some("ipsec"));
        assert_eq!(module("agent_speedtest_latency_ms"), Some("speedtest"));
        for host in [
            "cpu_usage_percent",
            "filesystem_used_percent",
            "if_octets_in",
            "agent_disk_smart_ok",
            "agent_sensor_temperature_celsius",
            "agent_updates_pending",
            "agent_os_info",
            "service_up",
            "agent_collect_seconds",
            "up",
        ] {
            assert_eq!(module(host), None, "{host} doit rester sur l'hôte");
        }
    }

    #[test]
    fn module_ids_are_unique_and_known_by_name() {
        let mut ids: Vec<_> = MODULES.iter().map(|m| m.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), MODULES.len());
        assert_eq!(find("zfs").map(|m| m.label), Some("ZFS"));
        assert!(find("cpu").is_none());
    }

    #[test]
    fn a_rule_reads_a_module_only_through_its_own_metrics() {
        let docker = find("docker").unwrap();
        let plakar = find("plakar").unwrap();
        assert!(query_reads("dumbmonit_container_up == bool 0", docker));
        assert!(!query_reads("dumbmonit_unraid_container_autostart_stopped", docker));
        assert!(query_reads("dumbmonit_backup_last_success_seconds > 1", plakar));
        assert!(!query_reads("dumbmonit_proxmox_backup_last_age_seconds", plakar));
        assert!(!query_reads("time() - dumbmonit_instance_backup_last_success_seconds", plakar));
    }

    #[test]
    fn a_module_device_is_wanted_when_seen_or_ticked_never_when_unticked() {
        let state = |enabled, seen: Option<i64>| ModuleState {
            module: "docker".into(),
            enabled,
            target_id: None,
            last_seen_ms: seen,
        };
        assert!(state(None, Some(1)).wanted());
        assert!(!state(None, None).wanted());
        assert!(state(Some(true), None).wanted());
        assert!(!state(Some(false), Some(1)).wanted());
    }
}
