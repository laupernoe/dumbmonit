//! Persistance des scans planifiés et de l'inventaire.

use anyhow::{Context, Result};
use dumbmonit_proto::{DiscoveredHost, ScanDepth, TargetId};
use serde::Serialize;
use sqlx::{Row, SqlitePool};

use super::diff::{self, KnownDevice};

/// Horodatage SQL : UTC, sans suffixe, comme le reste de l'API.
const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%S', 'now')";

/// Un scan planifié.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schedule {
    pub id: i64,
    pub name: String,
    pub cidr: String,
    /// Agent relais qui balaie ; `None` : le serveur.
    pub agent_id: Option<TargetId>,
    pub interval_secs: i64,
    pub depth: ScanDepth,
    pub enabled: bool,
    pub initialized: bool,
    pub last_run_at: Option<String>,
    pub last_status: Option<String>,
    pub last_error: Option<String>,
}

/// Ce que l'utilisateur règle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleInput {
    pub name: String,
    pub cidr: String,
    pub agent_id: Option<TargetId>,
    pub interval_secs: i64,
    pub depth: ScanDepth,
    pub enabled: bool,
}

pub fn depth_name(depth: ScanDepth) -> &'static str {
    match depth {
        ScanDepth::Standard => "standard",
        ScanDepth::Extended => "extended",
    }
}

fn parse_depth(raw: &str) -> ScanDepth {
    if raw == "extended" { ScanDepth::Extended } else { ScanDepth::Standard }
}

// Les requêtes de ce module composent leurs colonnes à partir de constantes du
// module (`format!` sans donnée extérieure) : d'où les `AssertSqlSafe`.
const SCHEDULE_COLUMNS: &str = "id, name, cidr, agent_id, interval_secs, depth, enabled, \
     initialized, last_run_at, last_status, last_error";

fn schedule_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Schedule> {
    Ok(Schedule {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        cidr: row.try_get("cidr")?,
        agent_id: row.try_get("agent_id")?,
        interval_secs: row.try_get("interval_secs")?,
        depth: parse_depth(&row.try_get::<String, _>("depth")?),
        enabled: row.try_get::<i64, _>("enabled")? != 0,
        initialized: row.try_get::<i64, _>("initialized")? != 0,
        last_run_at: row.try_get("last_run_at")?,
        last_status: row.try_get("last_status")?,
        last_error: row.try_get("last_error")?,
    })
}

pub async fn list_schedules(pool: &SqlitePool) -> Result<Vec<Schedule>> {
    let sql = format!("SELECT {SCHEDULE_COLUMNS} FROM netscan_schedules ORDER BY id");
    let rows = sqlx::query(sqlx::AssertSqlSafe(sql))
        .fetch_all(pool)
        .await
        .context("lecture des scans planifiés")?;
    rows.iter().map(schedule_from_row).collect()
}

pub async fn get_schedule(pool: &SqlitePool, id: i64) -> Result<Option<Schedule>> {
    let sql = format!("SELECT {SCHEDULE_COLUMNS} FROM netscan_schedules WHERE id = ?");
    let row = sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("lecture d'un scan planifié")?;
    row.as_ref().map(schedule_from_row).transpose()
}

pub async fn count_schedules(pool: &SqlitePool) -> Result<i64> {
    Ok(sqlx::query_scalar("SELECT COUNT(*) FROM netscan_schedules").fetch_one(pool).await?)
}

pub async fn insert_schedule(pool: &SqlitePool, input: &ScheduleInput) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO netscan_schedules (name, cidr, agent_id, interval_secs, depth, enabled)
         VALUES (?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(&input.name)
    .bind(&input.cidr)
    .bind(input.agent_id)
    .bind(input.interval_secs)
    .bind(depth_name(input.depth))
    .bind(input.enabled)
    .fetch_one(pool)
    .await
}

/// Met à jour un scan. Changer de sous-réseau ou d'exécutant repart d'un
/// inventaire vide (c'est un autre périmètre) ; les autres réglages le gardent.
/// Rend `false` si le scan n'existe pas.
pub async fn update_schedule(
    pool: &SqlitePool,
    id: i64,
    input: &ScheduleInput,
) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let before: Option<(String, Option<i64>)> =
        sqlx::query_as("SELECT cidr, agent_id FROM netscan_schedules WHERE id = ?")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
    let Some((cidr, agent_id)) = before else { return Ok(false) };
    let moved = cidr != input.cidr || agent_id != input.agent_id;
    if moved {
        sqlx::query("DELETE FROM netscan_devices WHERE schedule_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query(
        "UPDATE netscan_schedules SET name = ?, cidr = ?, agent_id = ?, interval_secs = ?,
             depth = ?, enabled = ?, initialized = CASE WHEN ? THEN 0 ELSE initialized END
         WHERE id = ?",
    )
    .bind(&input.name)
    .bind(&input.cidr)
    .bind(input.agent_id)
    .bind(input.interval_secs)
    .bind(depth_name(input.depth))
    .bind(input.enabled)
    .bind(moved)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(true)
}

pub async fn delete_schedule(pool: &SqlitePool, id: i64) -> Result<bool> {
    let done =
        sqlx::query("DELETE FROM netscan_schedules WHERE id = ?").bind(id).execute(pool).await?;
    Ok(done.rows_affected() > 0)
}

/// Marque le début d'un balayage.
pub async fn record_run_start(pool: &SqlitePool, id: i64) -> Result<()> {
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "UPDATE netscan_schedules SET last_run_at = {NOW}, last_status = 'running',
             last_error = NULL WHERE id = ?"
    )))
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Au démarrage : un balayage resté « en cours » ne l'est plus.
pub async fn reset_interrupted(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "UPDATE netscan_schedules SET last_status = 'failed',
             last_error = 'interrupted by a server restart' WHERE last_status = 'running'",
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Marque la fin d'un balayage (`done`, `failed` ou `cancelled`).
pub async fn record_run_end(
    pool: &SqlitePool,
    id: i64,
    status: &str,
    error: Option<&str>,
) -> Result<()> {
    sqlx::query("UPDATE netscan_schedules SET last_status = ?, last_error = ? WHERE id = ?")
        .bind(status)
        .bind(error)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Demande un balayage au prochain tour : l'échéance est remise à zéro, sauf si
/// un balayage est en cours. Rend `false` si le scan n'existe pas.
pub async fn request_run(pool: &SqlitePool, id: i64) -> Result<bool> {
    let done = sqlx::query(
        "UPDATE netscan_schedules SET last_run_at = NULL
         WHERE id = ? AND COALESCE(last_status, '') != 'running'",
    )
    .bind(id)
    .execute(pool)
    .await?;
    if done.rows_affected() > 0 {
        return Ok(true);
    }
    let exists: Option<i64> = sqlx::query_scalar("SELECT id FROM netscan_schedules WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(exists.is_some())
}

// ------------------------------------------------------------ inventaire

/// Ce que l'application d'un balayage a changé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Applied {
    /// Le balayage a initialisé l'inventaire : rien n'a été signalé.
    pub initialized: bool,
    pub new_devices: usize,
    pub new_ports: usize,
}

/// Range le résultat d'un balayage terminé dans l'inventaire du scan `id`.
///
/// Le premier balayage qui trouve quelque chose initialise l'inventaire : tout
/// y entre « connu ». Ensuite, un appareil ou un port absent y entre
/// « inconnu » et sera signalé. Rien n'est jamais retiré : un appareil éteint
/// reste dans l'inventaire.
pub async fn apply_scan(pool: &SqlitePool, id: i64, hosts: &[DiscoveredHost]) -> Result<Applied> {
    let mut tx = pool.begin().await?;
    let initialized: i64 =
        sqlx::query_scalar("SELECT initialized FROM netscan_schedules WHERE id = ?")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .context("scan planifié disparu")?;
    let first_scan = initialized == 0;
    let known_flag = i64::from(first_scan);

    let device_rows =
        sqlx::query("SELECT id, ident, ip FROM netscan_devices WHERE schedule_id = ?")
            .bind(id)
            .fetch_all(&mut *tx)
            .await?;
    let port_rows = sqlx::query(
        "SELECT p.device_id, p.port, p.protocol FROM netscan_ports p
         JOIN netscan_devices d ON d.id = p.device_id WHERE d.schedule_id = ?",
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await?;
    let mut known: Vec<KnownDevice> = Vec::with_capacity(device_rows.len());
    for row in &device_rows {
        known.push(KnownDevice {
            id: row.try_get("id")?,
            ident: row.try_get("ident")?,
            ip: row.try_get("ip")?,
            ports: Vec::new(),
        });
    }
    for row in &port_rows {
        let device_id: i64 = row.try_get("device_id")?;
        if let Some(device) = known.iter_mut().find(|d| d.id == device_id) {
            let port: i64 = row.try_get("port")?;
            device.ports.push((u16::try_from(port).unwrap_or(0), row.try_get("protocol")?));
        }
    }

    let mut applied = Applied::default();
    for planned in diff::plan(&known, hosts) {
        let device_id = match planned.existing_id {
            Some(device_id) => {
                sqlx::query(sqlx::AssertSqlSafe(format!(
                    "UPDATE netscan_devices SET ident = ?, mac = COALESCE(?, mac), ip = ?,
                         hostname = COALESCE(?, hostname), vendor = COALESCE(?, vendor),
                         last_seen = {NOW}
                     WHERE id = ?"
                )))
                .bind(&planned.ident)
                .bind(&planned.mac)
                .bind(&planned.ip)
                .bind(&planned.hostname)
                .bind(&planned.vendor)
                .bind(device_id)
                .execute(&mut *tx)
                .await?;
                device_id
            }
            None => {
                applied.new_devices += usize::from(!first_scan);
                sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                    "INSERT INTO netscan_devices
                         (schedule_id, ident, mac, ip, hostname, vendor, known, first_seen,
                          last_seen)
                     VALUES (?, ?, ?, ?, ?, ?, ?, {NOW}, {NOW}) RETURNING id"
                )))
                .bind(id)
                .bind(&planned.ident)
                .bind(&planned.mac)
                .bind(&planned.ip)
                .bind(&planned.hostname)
                .bind(&planned.vendor)
                .bind(known_flag)
                .fetch_one(&mut *tx)
                .await?
            }
        };
        for port in &planned.ports {
            if port.is_new {
                // Les ports d'un appareil nouveau ne sont pas comptés à part :
                // l'alerte de l'appareil les couvre.
                applied.new_ports += usize::from(!first_scan && !planned.is_new);
                sqlx::query(sqlx::AssertSqlSafe(format!(
                    "INSERT INTO netscan_ports
                         (device_id, port, protocol, product, known, first_seen, last_seen)
                     VALUES (?, ?, ?, ?, ?, {NOW}, {NOW})
                     ON CONFLICT (device_id, port, protocol) DO UPDATE SET last_seen = {NOW}"
                )))
                .bind(device_id)
                .bind(i64::from(port.port))
                .bind(&port.protocol)
                .bind(&port.product)
                .bind(known_flag)
                .execute(&mut *tx)
                .await?;
            } else {
                sqlx::query(sqlx::AssertSqlSafe(format!(
                    "UPDATE netscan_ports SET last_seen = {NOW}, product = ?
                     WHERE device_id = ? AND port = ? AND protocol = ?"
                )))
                .bind(&port.product)
                .bind(device_id)
                .bind(i64::from(port.port))
                .bind(&port.protocol)
                .execute(&mut *tx)
                .await?;
            }
        }
    }

    // Un balayage qui ne trouve rien n'initialise rien : un relais coupé du
    // réseau ne doit pas faire alerter sur tout le sous-réseau à son retour.
    if first_scan && !hosts.is_empty() {
        sqlx::query("UPDATE netscan_schedules SET initialized = 1 WHERE id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        applied.initialized = true;
    }
    tx.commit().await?;
    Ok(applied)
}

/// Un port ouvert de l'inventaire.
#[derive(Debug, Clone, Serialize)]
pub struct PortView {
    pub id: i64,
    pub port: u16,
    pub protocol: String,
    pub product: String,
    pub known: bool,
    pub first_seen: String,
    pub last_seen: String,
}

/// Un appareil de l'inventaire.
#[derive(Debug, Clone, Serialize)]
pub struct DeviceView {
    pub id: i64,
    pub schedule_id: i64,
    pub ip: String,
    pub mac: Option<String>,
    pub hostname: Option<String>,
    pub vendor: Option<String>,
    pub known: bool,
    pub first_seen: String,
    pub last_seen: String,
    pub ports: Vec<PortView>,
}

/// L'inventaire (d'un scan, ou de tous), les éléments inconnus d'abord.
pub async fn inventory(pool: &SqlitePool, schedule_id: Option<i64>) -> Result<Vec<DeviceView>> {
    let rows = sqlx::query(
        "SELECT id, schedule_id, ip, mac, hostname, vendor, known, first_seen, last_seen
         FROM netscan_devices WHERE (?1 IS NULL OR schedule_id = ?1)
         ORDER BY known, first_seen DESC, id DESC",
    )
    .bind(schedule_id)
    .fetch_all(pool)
    .await?;
    let mut devices: Vec<DeviceView> = Vec::with_capacity(rows.len());
    for row in &rows {
        devices.push(DeviceView {
            id: row.try_get("id")?,
            schedule_id: row.try_get("schedule_id")?,
            ip: row.try_get("ip")?,
            mac: row.try_get("mac")?,
            hostname: row.try_get("hostname")?,
            vendor: row.try_get("vendor")?,
            known: row.try_get::<i64, _>("known")? != 0,
            first_seen: row.try_get("first_seen")?,
            last_seen: row.try_get("last_seen")?,
            ports: Vec::new(),
        });
    }
    let ports = sqlx::query(
        "SELECT p.id, p.device_id, p.port, p.protocol, p.product, p.known, p.first_seen,
                p.last_seen
         FROM netscan_ports p JOIN netscan_devices d ON d.id = p.device_id
         WHERE (?1 IS NULL OR d.schedule_id = ?1)
         ORDER BY p.port, p.protocol",
    )
    .bind(schedule_id)
    .fetch_all(pool)
    .await?;
    for row in &ports {
        let device_id: i64 = row.try_get("device_id")?;
        let Some(device) = devices.iter_mut().find(|d| d.id == device_id) else { continue };
        device.ports.push(PortView {
            id: row.try_get("id")?,
            port: u16::try_from(row.try_get::<i64, _>("port")?).unwrap_or(0),
            protocol: row.try_get("protocol")?,
            product: row.try_get("product")?,
            known: row.try_get::<i64, _>("known")? != 0,
            first_seen: row.try_get("first_seen")?,
            last_seen: row.try_get("last_seen")?,
        });
    }
    Ok(devices)
}

/// Décomptes d'un scan : appareils, appareils inconnus, ports inconnus.
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct Counts {
    pub devices: i64,
    pub unknown_devices: i64,
    pub unknown_ports: i64,
}

pub async fn counts(pool: &SqlitePool) -> Result<std::collections::HashMap<i64, Counts>> {
    let mut out: std::collections::HashMap<i64, Counts> = std::collections::HashMap::new();
    let devices = sqlx::query(
        "SELECT schedule_id, COUNT(*) AS total, COALESCE(SUM(known = 0), 0) AS unknown
         FROM netscan_devices GROUP BY schedule_id",
    )
    .fetch_all(pool)
    .await?;
    for row in &devices {
        let entry = out.entry(row.try_get("schedule_id")?).or_default();
        entry.devices = row.try_get("total")?;
        entry.unknown_devices = row.try_get("unknown")?;
    }
    let ports = sqlx::query(
        "SELECT d.schedule_id, COUNT(*) AS unknown FROM netscan_ports p
         JOIN netscan_devices d ON d.id = p.device_id
         WHERE p.known = 0 AND d.known = 1 GROUP BY d.schedule_id",
    )
    .fetch_all(pool)
    .await?;
    for row in &ports {
        out.entry(row.try_get("schedule_id")?).or_default().unknown_ports =
            row.try_get("unknown")?;
    }
    Ok(out)
}

/// Marque un appareil, et ses ports, comme connus. `false` : introuvable.
pub async fn mark_device_known(pool: &SqlitePool, id: i64) -> Result<bool> {
    let mut tx = pool.begin().await?;
    let done = sqlx::query("UPDATE netscan_devices SET known = 1 WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE netscan_ports SET known = 1 WHERE device_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(done.rows_affected() > 0)
}

/// Marque un port comme connu. `false` : introuvable.
pub async fn mark_port_known(pool: &SqlitePool, id: i64) -> Result<bool> {
    let done = sqlx::query("UPDATE netscan_ports SET known = 1 WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(done.rows_affected() > 0)
}

/// Marque tout comme connu (d'un scan, ou de tous). Rend le nombre d'éléments
/// (appareils et ports) qui étaient inconnus.
pub async fn mark_all_known(pool: &SqlitePool, schedule_id: Option<i64>) -> Result<u64> {
    let mut tx = pool.begin().await?;
    let ports = sqlx::query(
        "UPDATE netscan_ports SET known = 1 WHERE known = 0 AND device_id IN
             (SELECT id FROM netscan_devices WHERE (?1 IS NULL OR schedule_id = ?1))",
    )
    .bind(schedule_id)
    .execute(&mut *tx)
    .await?;
    let devices = sqlx::query(
        "UPDATE netscan_devices SET known = 1
         WHERE known = 0 AND (?1 IS NULL OR schedule_id = ?1)",
    )
    .bind(schedule_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(ports.rows_affected() + devices.rows_affected())
}

/// Un élément encore inconnu, pour la mesure qui porte l'alerte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unknown {
    Device { schedule_id: i64, cidr: String, ident: String },
    Port { schedule_id: i64, cidr: String, ident: String, port: u16, protocol: String },
}

/// Appareils inconnus, et ports inconnus des appareils déjà connus (ceux d'un
/// appareil inconnu sont couverts par l'alerte de l'appareil).
pub async fn unknown_items(pool: &SqlitePool) -> Result<Vec<Unknown>> {
    let mut out = Vec::new();
    let devices = sqlx::query(
        "SELECT d.schedule_id, s.cidr, d.ident FROM netscan_devices d
         JOIN netscan_schedules s ON s.id = d.schedule_id WHERE d.known = 0",
    )
    .fetch_all(pool)
    .await?;
    for row in &devices {
        out.push(Unknown::Device {
            schedule_id: row.try_get("schedule_id")?,
            cidr: row.try_get("cidr")?,
            ident: row.try_get("ident")?,
        });
    }
    let ports = sqlx::query(
        "SELECT d.schedule_id, s.cidr, d.ident, p.port, p.protocol FROM netscan_ports p
         JOIN netscan_devices d ON d.id = p.device_id
         JOIN netscan_schedules s ON s.id = d.schedule_id
         WHERE p.known = 0 AND d.known = 1",
    )
    .fetch_all(pool)
    .await?;
    for row in &ports {
        out.push(Unknown::Port {
            schedule_id: row.try_get("schedule_id")?,
            cidr: row.try_get("cidr")?,
            ident: row.try_get("ident")?,
            port: u16::try_from(row.try_get::<i64, _>("port")?).unwrap_or(0),
            protocol: row.try_get("protocol")?,
        });
    }
    Ok(out)
}
