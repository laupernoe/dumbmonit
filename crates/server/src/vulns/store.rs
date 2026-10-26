//! Lecture et écriture des tables `cve_*` et `agent_inventory`.

use std::collections::{HashMap, HashSet};

use dumbmonit_proto::{SoftwareInventory, TargetId};
use sqlx::{Row, SqlitePool};

use super::debian;
use super::nvd::{CpeRange, CveInfo};
use super::osv;

pub fn now() -> String {
    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

// ------------------------------------------------------------- inventaire

/// Garde le dernier inventaire d'un agent.
pub async fn save_agent_inventory(
    pool: &SqlitePool,
    target_id: TargetId,
    inventory: &SoftwareInventory,
) -> sqlx::Result<()> {
    let collected_at = chrono::DateTime::from_timestamp_millis(inventory.collected_at_ms)
        .unwrap_or_else(chrono::Utc::now)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    let json = serde_json::to_string(inventory).unwrap_or_default();
    sqlx::query(
        "INSERT INTO agent_inventory (target_id, collected_at, received_at, inventory) \
         VALUES (?, ?, ?, ?) ON CONFLICT(target_id) DO UPDATE SET \
         collected_at = excluded.collected_at, received_at = excluded.received_at, \
         inventory = excluded.inventory",
    )
    .bind(target_id)
    .bind(collected_at)
    .bind(now())
    .bind(json)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn agent_inventories(
    pool: &SqlitePool,
) -> sqlx::Result<Vec<(TargetId, SoftwareInventory)>> {
    let rows =
        sqlx::query("SELECT target_id, inventory FROM agent_inventory").fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let id: TargetId = row.get("target_id");
            let json: String = row.get("inventory");
            serde_json::from_str(&json).ok().map(|inventory| (id, inventory))
        })
        .collect())
}

// ----------------------------------------------------------------- sources

#[derive(Debug, Clone, Default)]
pub struct SourceState {
    pub name: String,
    pub last_attempt: Option<String>,
    pub last_success: Option<String>,
    pub last_error: Option<String>,
    pub cursor: Option<String>,
}

pub async fn source(pool: &SqlitePool, name: &str) -> sqlx::Result<SourceState> {
    let row = sqlx::query(
        "SELECT name, last_attempt, last_success, last_error, cursor FROM cve_sources \
         WHERE name = ?",
    )
    .bind(name)
    .fetch_optional(pool)
    .await?;
    Ok(row
        .map(|row| SourceState {
            name: row.get("name"),
            last_attempt: row.get("last_attempt"),
            last_success: row.get("last_success"),
            last_error: row.get("last_error"),
            cursor: row.get("cursor"),
        })
        .unwrap_or_else(|| SourceState { name: name.to_string(), ..SourceState::default() }))
}

/// Note un essai : réussi (avec le prochain curseur) ou en échec (l'erreur
/// est gardée, le dernier succès et le curseur aussi).
pub async fn record_source(
    pool: &SqlitePool,
    name: &str,
    outcome: Result<Option<&str>, &str>,
) -> sqlx::Result<()> {
    let now = now();
    match outcome {
        Ok(cursor) => {
            sqlx::query(
                "INSERT INTO cve_sources (name, last_attempt, last_success, last_error, cursor) \
                 VALUES (?, ?, ?, NULL, ?) ON CONFLICT(name) DO UPDATE SET \
                 last_attempt = excluded.last_attempt, last_success = excluded.last_success, \
                 last_error = NULL, cursor = COALESCE(excluded.cursor, cve_sources.cursor)",
            )
            .bind(name)
            .bind(&now)
            .bind(&now)
            .bind(cursor)
            .execute(pool)
            .await?;
        }
        Err(error) => {
            sqlx::query(
                "INSERT INTO cve_sources (name, last_attempt, last_error) VALUES (?, ?, ?) \
                 ON CONFLICT(name) DO UPDATE SET last_attempt = excluded.last_attempt, \
                 last_error = excluded.last_error",
            )
            .bind(name)
            .bind(&now)
            .bind(error)
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

/// Oublie le curseur d'une source : la prochaine lecture sera complète.
pub async fn reset_cursor(pool: &SqlitePool, name: &str) -> sqlx::Result<()> {
    sqlx::query("UPDATE cve_sources SET cursor = NULL WHERE name = ?")
        .bind(name)
        .execute(pool)
        .await?;
    Ok(())
}

// --------------------------------------------------------------- cve_info

/// Enregistre ce qu'une source dit d'une CVE. `scored` : la fiche vient de
/// la NVD (note officielle) ; sinon le résumé ne remplace qu'un vide et la
/// note n'est pas touchée.
pub async fn upsert_info(pool: &SqlitePool, info: &CveInfo, scored: bool) -> sqlx::Result<()> {
    if scored {
        sqlx::query(
            "INSERT INTO cve_info (id, summary, cvss, published, scored_at) VALUES (?, ?, ?, ?, ?) \
             ON CONFLICT(id) DO UPDATE SET \
             summary = CASE WHEN excluded.summary <> '' THEN excluded.summary ELSE cve_info.summary END, \
             cvss = excluded.cvss, \
             published = COALESCE(excluded.published, cve_info.published), \
             scored_at = excluded.scored_at",
        )
        .bind(&info.id)
        .bind(&info.summary)
        .bind(info.cvss)
        .bind(&info.published)
        .bind(now())
        .execute(pool)
        .await?;
    } else {
        sqlx::query(
            "INSERT INTO cve_info (id, summary, published) VALUES (?, ?, ?) \
             ON CONFLICT(id) DO UPDATE SET \
             summary = CASE WHEN cve_info.summary = '' THEN excluded.summary ELSE cve_info.summary END, \
             published = COALESCE(cve_info.published, excluded.published)",
        )
        .bind(&info.id)
        .bind(&info.summary)
        .bind(&info.published)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Ce qu'on sait de chaque CVE : (résumé, note, publication, notée ?).
pub type InfoMap = HashMap<String, (String, Option<f64>, Option<String>, bool)>;

pub async fn infos(pool: &SqlitePool) -> sqlx::Result<InfoMap> {
    let rows = sqlx::query("SELECT id, summary, cvss, published, scored_at FROM cve_info")
        .fetch_all(pool)
        .await?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let scored: Option<String> = row.get("scored_at");
            (
                row.get("id"),
                (row.get("summary"), row.get("cvss"), row.get("published"), scored.is_some()),
            )
        })
        .collect())
}

/// Note une fiche NVD introuvable : inutile de la redemander à chaque tour.
pub async fn mark_scored(pool: &SqlitePool, id: &str) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO cve_info (id, scored_at) VALUES (?, ?) \
         ON CONFLICT(id) DO UPDATE SET scored_at = excluded.scored_at",
    )
    .bind(id)
    .bind(now())
    .execute(pool)
    .await?;
    Ok(())
}

// --------------------------------------------------------------------- NVD

pub async fn save_nvd(
    pool: &SqlitePool,
    product: &str,
    items: &[(CveInfo, Vec<CpeRange>)],
) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    for (info, ranges) in items {
        if ranges.is_empty() {
            // La CVE ne nomme plus le produit (configuration corrigée).
            sqlx::query("DELETE FROM cve_nvd_product WHERE product = ? AND cve_id = ?")
                .bind(product)
                .bind(&info.id)
                .execute(&mut *tx)
                .await?;
            continue;
        }
        sqlx::query(
            "INSERT INTO cve_nvd_product (product, cve_id, ranges) VALUES (?, ?, ?) \
             ON CONFLICT(product, cve_id) DO UPDATE SET ranges = excluded.ranges",
        )
        .bind(product)
        .bind(&info.id)
        .bind(serde_json::to_string(ranges).unwrap_or_default())
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO cve_info (id, summary, cvss, published, scored_at) VALUES (?, ?, ?, ?, ?) \
             ON CONFLICT(id) DO UPDATE SET summary = excluded.summary, cvss = excluded.cvss, \
             published = excluded.published, scored_at = excluded.scored_at",
        )
        .bind(&info.id)
        .bind(&info.summary)
        .bind(info.cvss)
        .bind(&info.published)
        .bind(now())
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await
}

/// CVE NVD d'un produit : (CVE, plages).
pub async fn nvd_for(
    pool: &SqlitePool,
    product: &str,
) -> sqlx::Result<Vec<(String, Vec<CpeRange>)>> {
    let rows = sqlx::query("SELECT cve_id, ranges FROM cve_nvd_product WHERE product = ?")
        .bind(product)
        .fetch_all(pool)
        .await?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let ranges: String = row.get("ranges");
            (row.get("cve_id"), serde_json::from_str(&ranges).unwrap_or_default())
        })
        .collect())
}

pub async fn count(pool: &SqlitePool, table: &str) -> sqlx::Result<i64> {
    // `table` ne vient jamais de l'extérieur (liste fixe de l'appelant).
    sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT COUNT(*) FROM {table}")))
        .fetch_one(pool)
        .await
}

// ------------------------------------------------------------------ Debian

/// Remplace les entrées des paquets et versions relus.
pub async fn save_debian(
    pool: &SqlitePool,
    releases: &HashSet<String>,
    entries: &[debian::Entry],
) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    for release in releases {
        sqlx::query("DELETE FROM cve_debian WHERE release = ?")
            .bind(release)
            .execute(&mut *tx)
            .await?;
    }
    for entry in entries {
        sqlx::query(
            "INSERT OR REPLACE INTO cve_debian \
             (package, release, cve_id, status, fixed_version, urgency, nodsa) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&entry.package)
        .bind(&entry.release)
        .bind(&entry.cve)
        .bind(entry.status.as_str())
        .bind(&entry.fixed_version)
        .bind(&entry.urgency)
        .bind(entry.nodsa)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO cve_info (id, summary) VALUES (?, ?) ON CONFLICT(id) DO UPDATE SET \
             summary = CASE WHEN cve_info.summary = '' THEN excluded.summary \
             ELSE cve_info.summary END",
        )
        .bind(&entry.cve)
        .bind(&entry.summary)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await
}

#[derive(Debug, Clone)]
pub struct DebianRow {
    pub cve: String,
    pub status: debian::Status,
    pub fixed_version: Option<String>,
    pub nodsa: bool,
}

/// Entrées Debian rangées par (paquet, version de Debian).
pub async fn debian_all(
    pool: &SqlitePool,
) -> sqlx::Result<HashMap<(String, String), Vec<DebianRow>>> {
    let rows = sqlx::query(
        "SELECT package, release, cve_id, status, fixed_version, nodsa FROM cve_debian",
    )
    .fetch_all(pool)
    .await?;
    let mut out: HashMap<(String, String), Vec<DebianRow>> = HashMap::new();
    for row in rows {
        let status: String = row.get("status");
        let Some(status) = debian::Status::parse(&status) else { continue };
        out.entry((row.get("package"), row.get("release"))).or_default().push(DebianRow {
            cve: row.get("cve_id"),
            status,
            fixed_version: row.get("fixed_version"),
            nodsa: row.get::<i64, _>("nodsa") != 0,
        });
    }
    Ok(out)
}

// --------------------------------------------------------------------- OSV

pub async fn save_osv_query(
    pool: &SqlitePool,
    query: &osv::Query,
    vulns: &[osv::VulnRef],
) -> sqlx::Result<()> {
    let ids: Vec<&str> = vulns.iter().map(|v| v.id.as_str()).collect();
    sqlx::query(
        "INSERT OR REPLACE INTO cve_osv_query (ecosystem, package, version, vulns, fetched_at) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&query.ecosystem)
    .bind(&query.package)
    .bind(&query.version)
    .bind(serde_json::to_string(&ids).unwrap_or_default())
    .bind(now())
    .execute(pool)
    .await?;
    Ok(())
}

/// Avis OSV connus pour (écosystème, paquet, version).
pub async fn osv_queries(
    pool: &SqlitePool,
) -> sqlx::Result<HashMap<(String, String, String), Vec<String>>> {
    let rows = sqlx::query("SELECT ecosystem, package, version, vulns FROM cve_osv_query")
        .fetch_all(pool)
        .await?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let vulns: String = row.get("vulns");
            (
                (row.get("ecosystem"), row.get("package"), row.get("version")),
                serde_json::from_str(&vulns).unwrap_or_default(),
            )
        })
        .collect())
}

/// Oublie les questions OSV d'une version qui n'est plus installée depuis
/// un mois.
pub async fn prune_osv(pool: &SqlitePool) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM cve_osv_query WHERE fetched_at < datetime('now', '-30 days')")
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn osv_modified(pool: &SqlitePool) -> sqlx::Result<HashMap<String, String>> {
    let rows = sqlx::query("SELECT id, modified FROM cve_osv_vuln").fetch_all(pool).await?;
    Ok(rows.into_iter().map(|row| (row.get("id"), row.get("modified"))).collect())
}

pub async fn save_osv_vuln(pool: &SqlitePool, vuln: &osv::Vuln) -> sqlx::Result<()> {
    sqlx::query("INSERT OR REPLACE INTO cve_osv_vuln (id, modified, data) VALUES (?, ?, ?)")
        .bind(&vuln.id)
        .bind(&vuln.modified)
        .bind(serde_json::to_string(vuln).unwrap_or_default())
        .execute(pool)
        .await?;
    for id in vuln.ids() {
        upsert_info(
            pool,
            &CveInfo {
                id,
                summary: vuln.summary.clone(),
                cvss: None,
                published: vuln.published.clone(),
            },
            false,
        )
        .await?;
    }
    Ok(())
}

pub async fn osv_vulns(pool: &SqlitePool) -> sqlx::Result<HashMap<String, osv::Vuln>> {
    let rows = sqlx::query("SELECT id, data FROM cve_osv_vuln").fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let data: String = row.get("data");
            let vuln: osv::Vuln = serde_json::from_str(&data).ok()?;
            Some((row.get("id"), vuln))
        })
        .collect())
}

// --------------------------------------------------------------------- KEV

#[derive(Debug, Clone, Copy, Default)]
pub struct Kev {
    pub ransomware: bool,
}

pub async fn save_kev(pool: &SqlitePool, entries: &[super::kev::Entry]) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM cve_kev").execute(&mut *tx).await?;
    for entry in entries {
        sqlx::query(
            "INSERT OR REPLACE INTO cve_kev (cve_id, date_added, ransomware, name) \
             VALUES (?, ?, ?, ?)",
        )
        .bind(&entry.cve)
        .bind(&entry.date_added)
        .bind(entry.ransomware)
        .bind(&entry.name)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await
}

pub async fn kev_all(pool: &SqlitePool) -> sqlx::Result<HashMap<String, Kev>> {
    let rows = sqlx::query("SELECT cve_id, ransomware FROM cve_kev").fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|row| (row.get("cve_id"), Kev { ransomware: row.get::<i64, _>("ransomware") != 0 }))
        .collect())
}

// ---------------------------------------------------------------- résultats

#[derive(Debug, Clone, PartialEq)]
pub struct FindingRow {
    pub target_id: TargetId,
    pub component: String,
    pub cve_id: String,
    pub installed_version: String,
    pub fixed_version: Option<String>,
    pub state: String,
    pub reason: String,
    pub source: String,
    pub first_seen: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComponentRow {
    pub target_id: TargetId,
    pub component: String,
    pub version: String,
    pub origin: String,
}

/// Remplace le résultat du calcul ; une CVE déjà vue garde sa date de
/// première apparition.
pub async fn replace_results(
    pool: &SqlitePool,
    components: &[ComponentRow],
    findings: &[FindingRow],
) -> sqlx::Result<()> {
    let first_seen: HashMap<(TargetId, String, String), String> =
        sqlx::query("SELECT target_id, component, cve_id, first_seen FROM cve_findings")
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|row| {
                (
                    (row.get("target_id"), row.get("component"), row.get("cve_id")),
                    row.get("first_seen"),
                )
            })
            .collect();
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM cve_components").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM cve_findings").execute(&mut *tx).await?;
    for component in components {
        sqlx::query(
            "INSERT OR IGNORE INTO cve_components (target_id, component, version, origin) \
             VALUES (?, ?, ?, ?)",
        )
        .bind(component.target_id)
        .bind(&component.component)
        .bind(&component.version)
        .bind(&component.origin)
        .execute(&mut *tx)
        .await?;
    }
    for finding in findings {
        let key = (finding.target_id, finding.component.clone(), finding.cve_id.clone());
        let seen = first_seen.get(&key).cloned().unwrap_or_else(|| finding.first_seen.clone());
        sqlx::query(
            "INSERT OR IGNORE INTO cve_findings (target_id, component, cve_id, installed_version, \
             fixed_version, state, reason, source, first_seen) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(finding.target_id)
        .bind(&finding.component)
        .bind(&finding.cve_id)
        .bind(&finding.installed_version)
        .bind(&finding.fixed_version)
        .bind(&finding.state)
        .bind(&finding.reason)
        .bind(&finding.source)
        .bind(seen)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await
}

pub async fn findings(pool: &SqlitePool) -> sqlx::Result<Vec<FindingRow>> {
    let rows = sqlx::query(
        "SELECT f.target_id, f.component, f.cve_id, f.installed_version, f.fixed_version, \
         f.state, f.reason, f.source, f.first_seen FROM cve_findings f \
         JOIN targets t ON t.id = f.target_id",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| FindingRow {
            target_id: row.get("target_id"),
            component: row.get("component"),
            cve_id: row.get("cve_id"),
            installed_version: row.get("installed_version"),
            fixed_version: row.get("fixed_version"),
            state: row.get("state"),
            reason: row.get("reason"),
            source: row.get("source"),
            first_seen: row.get("first_seen"),
        })
        .collect())
}

pub async fn components(pool: &SqlitePool) -> sqlx::Result<Vec<ComponentRow>> {
    let rows = sqlx::query(
        "SELECT c.target_id, c.component, c.version, c.origin FROM cve_components c \
         JOIN targets t ON t.id = c.target_id",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| ComponentRow {
            target_id: row.get("target_id"),
            component: row.get("component"),
            version: row.get("version"),
            origin: row.get("origin"),
        })
        .collect())
}

// ------------------------------------------------------------ risques acceptés

#[derive(Debug, Clone)]
pub struct Ignore {
    pub note: String,
    pub created_by: String,
    pub created_at: String,
}

pub async fn ignores(pool: &SqlitePool) -> sqlx::Result<HashMap<(TargetId, String), Ignore>> {
    let rows =
        sqlx::query("SELECT target_id, cve_id, note, created_by, created_at FROM cve_ignores")
            .fetch_all(pool)
            .await?;
    Ok(rows
        .into_iter()
        .map(|row| {
            (
                (row.get("target_id"), row.get("cve_id")),
                Ignore {
                    note: row.get("note"),
                    created_by: row.get("created_by"),
                    created_at: row.get("created_at"),
                },
            )
        })
        .collect())
}

pub async fn add_ignore(
    pool: &SqlitePool,
    target_id: TargetId,
    cve_id: &str,
    note: &str,
    created_by: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO cve_ignores (target_id, cve_id, note, created_by, created_at) \
         VALUES (?, ?, ?, ?, ?) ON CONFLICT(target_id, cve_id) DO UPDATE SET \
         note = excluded.note, created_by = excluded.created_by, created_at = excluded.created_at",
    )
    .bind(target_id)
    .bind(cve_id)
    .bind(note)
    .bind(created_by)
    .bind(now())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn remove_ignore(
    pool: &SqlitePool,
    target_id: TargetId,
    cve_id: &str,
) -> sqlx::Result<bool> {
    let done = sqlx::query("DELETE FROM cve_ignores WHERE target_id = ? AND cve_id = ?")
        .bind(target_id)
        .bind(cve_id)
        .execute(pool)
        .await?;
    Ok(done.rows_affected() > 0)
}
