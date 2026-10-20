//! Lecture du fichier `kuma.db` d'Uptime Kuma (SQLite, `data/kuma.db` en 1.x comme en 2.x).
//!
//! Le fichier est ouvert en lecture seule et « immuable » : jamais attaché à la base de
//! DumbMonit, jamais modifié, aucun fichier `-wal` ou `-shm` n'est créé à côté. Seules les
//! colonnes utiles de `monitor`, `tag`, `monitor_tag` (et le nombre de lignes de
//! `notification`) sont lues ; les moniteurs sont rendus sous la forme JSON de l'export 1.x
//! (`monitorList`), pour que `kuma_import::convert` reste la seule table de correspondance.
//!
//! Schéma vérifié sur `db/knex_init_db.js` et `db/knex_migrations/` d'Uptime Kuma 2.5.6
//! (1.23 : mêmes colonnes pour celles lues ici). Une colonne absente d'une vieille base est
//! lue comme `NULL`.

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use serde_json::{Map, Value, json};
use sqlx::{ConnectOptions, Row, sqlite::SqliteConnectOptions};

#[derive(Clone, Copy)]
enum Kind {
    Text,
    Int,
    Float,
    Bool,
    /// Tableau JSON stocké en texte.
    Json,
}

/// Colonne SQL, clé JSON de l'export 1.x, type.
const COLUMNS: &[(&str, &str, Kind)] = &[
    ("id", "id", Kind::Int),
    ("name", "name", Kind::Text),
    ("type", "type", Kind::Text),
    ("active", "active", Kind::Bool),
    ("parent", "parent", Kind::Int),
    ("interval", "interval", Kind::Int),
    ("url", "url", Kind::Text),
    ("method", "method", Kind::Text),
    ("hostname", "hostname", Kind::Text),
    ("port", "port", Kind::Int),
    ("keyword", "keyword", Kind::Text),
    ("invert_keyword", "invertKeyword", Kind::Bool),
    ("ignore_tls", "ignoreTls", Kind::Bool),
    ("upside_down", "upsideDown", Kind::Bool),
    ("maxredirects", "maxredirects", Kind::Int),
    ("timeout", "timeout", Kind::Float),
    ("json_path", "jsonPath", Kind::Text),
    ("expected_value", "expectedValue", Kind::Text),
    ("dns_resolve_type", "dns_resolve_type", Kind::Text),
    ("dns_resolve_server", "dns_resolve_server", Kind::Text),
    ("accepted_statuscodes_json", "accepted_statuscodes", Kind::Json),
    // Lues seulement pour savoir qu'elles existent (note « non importé ») : jamais copiées.
    ("basic_auth_user", "basic_auth_user", Kind::Text),
    ("headers", "headers", Kind::Text),
    ("body", "body", Kind::Text),
    ("oauth_client_id", "oauth_client_id", Kind::Text),
];

const MAGIC: &[u8; 16] = b"SQLite format 3\0";

/// Le fichier commence-t-il par l'en-tête SQLite ?
pub fn looks_like_sqlite(head: &[u8]) -> bool {
    head.len() >= MAGIC.len() && head[..MAGIC.len()] == MAGIC[..]
}

/// Moniteurs au format `monitorList`, et nombre de notifications du fichier.
pub struct KumaDb {
    pub root: Value,
    pub notifications: usize,
}

fn quote(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// Colonnes d'une table ; vide si la table n'existe pas. `table` est toujours une constante.
async fn columns_of(
    conn: &mut sqlx::SqliteConnection,
    table: &str,
) -> Result<HashSet<String>, sqlx::Error> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!("PRAGMA table_info({})", quote(table))))
        .fetch_all(&mut *conn)
        .await?;
    Ok(rows.iter().filter_map(|r| r.try_get::<String, _>("name").ok()).collect())
}

pub async fn read(path: &Path, max_monitors: usize) -> Result<KumaDb, String> {
    let unreadable = |e: sqlx::Error| format!("The database could not be read: {e}");

    let mut conn = SqliteConnectOptions::new()
        .filename(path)
        .read_only(true)
        .immutable(true)
        .create_if_missing(false)
        // Un fichier venu de l'extérieur : aucune fonction ni vue du schéma ne s'exécute.
        .pragma("trusted_schema", "OFF")
        .connect()
        .await
        .map_err(unreadable)?;

    let monitor_columns = columns_of(&mut conn, "monitor").await.map_err(unreadable)?;
    if !monitor_columns.contains("type") || !monitor_columns.contains("id") {
        return Err("This file is not an Uptime Kuma database (no \"monitor\" table).".into());
    }

    // Tout est lu en texte (`CAST`) puis converti ici : SQLite est faiblement typé.
    let select: Vec<String> = COLUMNS
        .iter()
        .map(|(column, _, _)| {
            if monitor_columns.contains(*column) {
                format!("CAST({} AS TEXT)", quote(column))
            } else {
                "NULL".to_string()
            }
        })
        .collect();
    let sql =
        format!("SELECT {} FROM monitor ORDER BY id LIMIT {}", select.join(", "), max_monitors + 1);
    let rows =
        sqlx::query(sqlx::AssertSqlSafe(sql)).fetch_all(&mut conn).await.map_err(unreadable)?;

    let mut monitors: Vec<Value> = Vec::with_capacity(rows.len());
    let mut index: BTreeMap<i64, usize> = BTreeMap::new();
    for row in &rows {
        let mut monitor = Map::new();
        for (i, (_, key, kind)) in COLUMNS.iter().enumerate() {
            let Some(raw) = row.try_get::<Option<String>, _>(i).ok().flatten() else { continue };
            let value = match kind {
                Kind::Text => Value::String(raw),
                Kind::Int => match raw.trim().parse::<i64>() {
                    Ok(n) => json!(n),
                    Err(_) => continue,
                },
                Kind::Float => match raw.trim().parse::<f64>() {
                    Ok(n) => json!(n),
                    Err(_) => continue,
                },
                Kind::Bool => json!(matches!(raw.trim(), "1" | "true" | "TRUE")),
                Kind::Json => match serde_json::from_str::<Value>(&raw) {
                    Ok(v) if v.is_array() => v,
                    _ => continue,
                },
            };
            monitor.insert((*key).to_string(), value);
        }
        monitor.insert("tags".into(), json!([]));
        if let Some(id) = monitor.get("id").and_then(Value::as_i64) {
            index.insert(id, monitors.len());
        }
        monitors.push(Value::Object(monitor));
    }

    // Étiquettes : le nom vient de `tag`, la valeur de `monitor_tag`.
    let tag_columns = columns_of(&mut conn, "tag").await.unwrap_or_default();
    let link_columns = columns_of(&mut conn, "monitor_tag").await.unwrap_or_default();
    if tag_columns.contains("name") && link_columns.contains("tag_id") {
        let links = sqlx::query(
            "SELECT CAST(mt.monitor_id AS INTEGER), CAST(t.name AS TEXT), CAST(mt.value AS TEXT) \
             FROM monitor_tag mt JOIN tag t ON t.id = mt.tag_id ORDER BY mt.id",
        )
        .fetch_all(&mut conn)
        .await
        .map_err(unreadable)?;
        for link in &links {
            let Some(position) = link
                .try_get::<Option<i64>, _>(0)
                .ok()
                .flatten()
                .and_then(|id| index.get(&id).copied())
            else {
                continue;
            };
            let name = link.try_get::<Option<String>, _>(1).ok().flatten().unwrap_or_default();
            let value = link.try_get::<Option<String>, _>(2).ok().flatten().unwrap_or_default();
            if let Some(tags) = monitors[position].get_mut("tags").and_then(Value::as_array_mut) {
                tags.push(json!({ "name": name, "value": value }));
            }
        }
    }

    // Notifications : comptées seulement (leur configuration porte des secrets).
    let notifications =
        if columns_of(&mut conn, "notification").await.unwrap_or_default().is_empty() {
            0
        } else {
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM notification")
                .fetch_one(&mut conn)
                .await
                .unwrap_or(0)
                .max(0) as usize
        };

    Ok(KumaDb { root: json!({ "monitorList": monitors }), notifications })
}
