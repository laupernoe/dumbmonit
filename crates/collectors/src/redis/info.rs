//! Lecture de la réponse à `INFO` et traduction en mesures.
//!
//! `INFO` rend des lignes `clé:valeur` groupées en sections (`# Memory`). Les
//! clés sont uniques d'une section à l'autre, ce qui permet de tout ranger dans
//! une seule table. Quelques valeurs sont elles-mêmes des listes `a=1,b=2` :
//! les bases (`db0:keys=201,expires=1,…`) et les répliques vues du primaire
//! (`slave0:ip=…,port=…,state=online,offset=…,lag=…`).
//!
//! Redis, Valkey et les forks compatibles (KeyDB, Dragonfly) rendent les mêmes
//! champs ; Valkey s'annonce par `server_name:valkey` et `valkey_version`, en
//! gardant `redis_version` figé à 7.2.4 pour les clients anciens.

use std::collections::BTreeMap;

use dumbmonit_proto::{MetricKind, Sample};

/// Nombre maximal de répliques décrites une par une.
const MAX_REPLICAS: usize = 32;

pub struct Info {
    fields: BTreeMap<String, String>,
}

impl Info {
    pub fn parse(text: &str) -> Self {
        let fields = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .filter_map(|line| line.split_once(':'))
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();
        Self { fields }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields.get(key).map(String::as_str)
    }

    pub fn number(&self, key: &str) -> Option<f64> {
        self.get(key)?.parse::<f64>().ok().filter(|v| v.is_finite())
    }

    /// Vrai si la réponse ressemble à celle d'un serveur Redis.
    pub fn looks_like_redis(&self) -> bool {
        self.get("redis_version").is_some() || self.get("valkey_version").is_some()
    }

    /// `redis` ou `valkey` (ou le nom annoncé par un fork), et sa version.
    pub fn product(&self) -> (&str, &str) {
        let name = self.get("server_name").unwrap_or("redis");
        let version = self
            .get(&format!("{name}_version"))
            .or_else(|| self.get("redis_version"))
            .unwrap_or("");
        (name, version)
    }

    fn with_prefix<'a>(&'a self, prefix: &'a str) -> impl Iterator<Item = (&'a str, &'a str)> {
        self.fields
            .iter()
            .filter(move |(key, _)| {
                key.strip_prefix(prefix).is_some_and(|rest| {
                    !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit())
                })
            })
            .map(|(key, value)| (key.as_str(), value.as_str()))
    }
}

/// `keys=201,expires=1,avg_ttl=0` → table.
fn pairs(value: &str) -> BTreeMap<&str, &str> {
    value.split(',').filter_map(|pair| pair.split_once('=')).collect()
}

pub fn samples(info: &Info, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let gauge = |name: &str, value: f64| Sample::new(name, value, MetricKind::Gauge, ts_ms);

    let (product, version) = info.product();
    let role = match info.get("role") {
        Some("slave") => "replica",
        Some(other) => other,
        None => "master",
    };
    out.push(
        gauge("redis_version_info", 1.0)
            .with_label("product", product)
            .with_label("version", version)
            .with_label("mode", info.get("redis_mode").unwrap_or("standalone")),
    );
    out.push(gauge("redis_role_info", 1.0).with_label("role", role));

    let mut push = |name: &str, key: &str, kind: MetricKind| {
        if let Some(value) = info.number(key) {
            out.push(Sample::new(name, value, kind, ts_ms));
        }
    };
    use MetricKind::{Counter, Gauge};
    push("redis_uptime_seconds", "uptime_in_seconds", Gauge);
    push("redis_connected_clients", "connected_clients", Gauge);
    push("redis_blocked_clients", "blocked_clients", Gauge);
    push("redis_max_clients", "maxclients", Gauge);
    push("redis_memory_used_bytes", "used_memory", Gauge);
    push("redis_memory_rss_bytes", "used_memory_rss", Gauge);
    push("redis_memory_peak_bytes", "used_memory_peak", Gauge);
    push("redis_memory_fragmentation_ratio", "mem_fragmentation_ratio", Gauge);
    push("redis_ops_per_second", "instantaneous_ops_per_sec", Gauge);
    push("redis_commands_processed_total", "total_commands_processed", Counter);
    push("redis_connections_received_total", "total_connections_received", Counter);
    push("redis_rejected_connections_total", "rejected_connections", Counter);
    push("redis_evicted_keys_total", "evicted_keys", Counter);
    push("redis_expired_keys_total", "expired_keys", Counter);
    push("redis_keyspace_hits_total", "keyspace_hits", Counter);
    push("redis_keyspace_misses_total", "keyspace_misses", Counter);
    push("redis_error_replies_total", "total_error_replies", Counter);
    push("redis_acl_denied_auth_total", "acl_access_denied_auth", Counter);
    push("redis_connected_replicas", "connected_slaves", Gauge);
    push("redis_loading", "loading", Gauge);
    push("redis_rdb_changes_since_last_save", "rdb_changes_since_last_save", Gauge);
    push("redis_rdb_last_save_timestamp_seconds", "rdb_last_save_time", Gauge);
    push("redis_aof_enabled", "aof_enabled", Gauge);
    push("redis_cluster_enabled", "cluster_enabled", Gauge);

    // `maxmemory:0` veut dire « sans limite » : ni plafond ni pourcentage.
    let used = info.number("used_memory");
    if let Some(max) = info.number("maxmemory").filter(|max| *max > 0.0) {
        out.push(gauge("redis_memory_max_bytes", max));
        if let Some(used) = used {
            out.push(gauge("redis_memory_used_percent", used / max * 100.0));
        }
    }
    if let Some(policy) = info.get("maxmemory_policy") {
        out.push(gauge("redis_maxmemory_policy_info", 1.0).with_label("policy", policy));
    }

    // Persistance : `ok` ou `err`. Un serveur sans RDB ni AOF dit `ok`.
    let status = |key: &str| info.get(key).map(|v| if v == "ok" { 1.0 } else { 0.0 });
    if let Some(ok) = status("rdb_last_bgsave_status") {
        out.push(gauge("redis_rdb_last_save_ok", ok));
    }
    if info.number("aof_enabled") == Some(1.0) {
        if let Some(ok) = status("aof_last_write_status") {
            out.push(gauge("redis_aof_last_write_ok", ok));
        }
        if let Some(ok) = status("aof_last_bgrewrite_status") {
            out.push(gauge("redis_aof_last_rewrite_ok", ok));
        }
    }

    // Réplique : le lien vers le primaire.
    if role == "replica" {
        let up = info.get("master_link_status") == Some("up");
        out.push(gauge("redis_master_link_up", if up { 1.0 } else { 0.0 }));
        if let Some(seconds) = info.number("master_last_io_seconds_ago").filter(|s| *s >= 0.0) {
            out.push(gauge("redis_master_last_io_seconds", seconds));
        }
        if let Some(seconds) = info.number("master_link_down_since_seconds").filter(|s| *s >= 0.0) {
            out.push(gauge("redis_master_link_down_seconds", seconds));
        }
        if let Some(sync) = info.number("master_sync_in_progress") {
            out.push(gauge("redis_master_sync_in_progress", sync));
        }
    }

    // Primaire : chacune de ses répliques, par adresse.
    let master_offset = info.number("master_repl_offset");
    for (_, value) in info.with_prefix("slave").take(MAX_REPLICAS) {
        let fields = pairs(value);
        let (Some(ip), Some(port)) = (fields.get("ip"), fields.get("port")) else { continue };
        let replica = format!("{ip}:{port}");
        let online = fields.get("state") == Some(&"online");
        out.push(
            gauge("redis_replica_online", if online { 1.0 } else { 0.0 })
                .with_label("replica", replica.clone()),
        );
        if let Some(lag) = fields.get("lag").and_then(|v| v.parse::<f64>().ok()) {
            out.push(
                gauge("redis_replica_lag_seconds", lag).with_label("replica", replica.clone()),
            );
        }
        if let (Some(master), Some(offset)) =
            (master_offset, fields.get("offset").and_then(|v| v.parse::<f64>().ok()))
        {
            out.push(
                gauge("redis_replica_lag_bytes", (master - offset).max(0.0))
                    .with_label("replica", replica),
            );
        }
    }

    // Taille du jeu de clés, par base et au total. Une base vide n'apparaît
    // pas dans `INFO` : le total est donc toujours écrit, zéro compris.
    let mut total = 0.0;
    let mut expiring = 0.0;
    for (key, value) in info.with_prefix("db") {
        let fields = pairs(value);
        let keys = fields.get("keys").and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
        let expires = fields.get("expires").and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
        total += keys;
        expiring += expires;
        out.push(gauge("redis_db_keys", keys).with_label("db", key));
    }
    out.push(gauge("redis_keys", total));
    out.push(gauge("redis_keys_expiring", expiring));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const MASTER: &str = include_str!("testdata/redis_8.2.1_master.info");
    const REPLICA: &str = include_str!("testdata/redis_8.2.1_replica.info");
    const VALKEY: &str = include_str!("testdata/valkey_8.1.3.info");

    fn value(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    fn label<'a>(samples: &'a [Sample], name: &str, key: &str) -> Option<&'a str> {
        samples
            .iter()
            .find(|s| s.metric == name)
            .and_then(|s| s.labels.get(key))
            .map(|v| v.as_str())
    }

    #[test]
    fn un_primaire_redis_reel() {
        let info = Info::parse(MASTER);
        assert!(info.looks_like_redis());
        let samples = samples(&info, 0);
        assert_eq!(label(&samples, "redis_version_info", "product"), Some("redis"));
        assert_eq!(label(&samples, "redis_version_info", "version"), Some("8.2.1"));
        assert_eq!(label(&samples, "redis_role_info", "role"), Some("master"));
        assert_eq!(value(&samples, "redis_memory_max_bytes"), Some(67_108_864.0));
        let percent = value(&samples, "redis_memory_used_percent").unwrap();
        assert!((0.1..10.0).contains(&percent), "{percent}");
        assert_eq!(label(&samples, "redis_maxmemory_policy_info", "policy"), Some("allkeys-lru"));
        assert_eq!(value(&samples, "redis_keys"), Some(202.0));
        assert_eq!(value(&samples, "redis_keys_expiring"), Some(1.0));
        assert_eq!(samples.iter().filter(|s| s.metric == "redis_db_keys").count(), 2);
        assert_eq!(value(&samples, "redis_connected_replicas"), Some(1.0));
        assert_eq!(value(&samples, "redis_replica_online"), Some(1.0));
        assert_eq!(label(&samples, "redis_replica_online", "replica"), Some("10.0.0.12:6379"));
        assert!(value(&samples, "redis_replica_lag_seconds").is_some());
        assert_eq!(value(&samples, "redis_replica_lag_bytes"), Some(0.0));
        assert_eq!(value(&samples, "redis_rdb_last_save_ok"), Some(1.0));
        assert_eq!(value(&samples, "redis_aof_last_write_ok"), Some(1.0));
        assert_eq!(value(&samples, "redis_rejected_connections_total"), Some(0.0));
        assert!(value(&samples, "redis_master_link_up").is_none(), "pas une réplique");
        let evicted = samples.iter().find(|s| s.metric == "redis_evicted_keys_total").unwrap();
        assert_eq!(evicted.kind, MetricKind::Counter);
    }

    #[test]
    fn une_replique_redis_reelle() {
        let samples = samples(&Info::parse(REPLICA), 0);
        assert_eq!(label(&samples, "redis_role_info", "role"), Some("replica"));
        assert_eq!(value(&samples, "redis_master_link_up"), Some(1.0));
        assert!(value(&samples, "redis_master_last_io_seconds").is_some());
        assert_eq!(value(&samples, "redis_master_sync_in_progress"), Some(0.0));
        assert!(value(&samples, "redis_master_link_down_seconds").is_none());
        // Pas de `maxmemory` : ni plafond, ni pourcentage.
        assert!(value(&samples, "redis_memory_max_bytes").is_none());
        assert!(value(&samples, "redis_memory_used_percent").is_none());
        // Pas d'AOF sur cette réplique : pas de statut d'écriture.
        assert!(value(&samples, "redis_aof_last_write_ok").is_none());
        assert_eq!(value(&samples, "redis_keys"), Some(202.0));
    }

    #[test]
    fn valkey_s_annonce_sous_son_nom() {
        let samples = samples(&Info::parse(VALKEY), 0);
        assert_eq!(label(&samples, "redis_version_info", "product"), Some("valkey"));
        assert_eq!(label(&samples, "redis_version_info", "version"), Some("8.1.3"));
        // Une instance vide : le total existe, à zéro.
        assert_eq!(value(&samples, "redis_keys"), Some(0.0));
    }

    #[test]
    fn un_lien_de_replication_coupe_et_une_sauvegarde_ratee() {
        let text = "# Server\r\nredis_version:7.4.0\r\n# Persistence\r\n\
                    rdb_last_bgsave_status:err\r\naof_enabled:1\r\naof_last_write_status:err\r\n\
                    aof_last_bgrewrite_status:ok\r\n# Replication\r\nrole:slave\r\n\
                    master_link_status:down\r\nmaster_last_io_seconds_ago:-1\r\n\
                    master_link_down_since_seconds:42\r\n";
        let samples = samples(&Info::parse(text), 0);
        assert_eq!(value(&samples, "redis_master_link_up"), Some(0.0));
        assert_eq!(value(&samples, "redis_master_link_down_seconds"), Some(42.0));
        assert!(value(&samples, "redis_master_last_io_seconds").is_none());
        assert_eq!(value(&samples, "redis_rdb_last_save_ok"), Some(0.0));
        assert_eq!(value(&samples, "redis_aof_last_write_ok"), Some(0.0));
        assert_eq!(value(&samples, "redis_aof_last_rewrite_ok"), Some(1.0));
    }
}
