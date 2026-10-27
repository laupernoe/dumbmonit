//! Garde des objets « gérés par fichier ».
//!
//! Une règle, un canal, une fenêtre de maintenance ou une page de statut
//! déclarés par un fichier ne se modifient pas à la main : la prochaine
//! application écraserait le changement sans prévenir. Plutôt que d'ajouter le
//! même test à chaque gestionnaire, ce garde regarde la méthode et le chemin
//! de la requête, retrouve la clé naturelle de l'objet visé et refuse (409)
//! s'il est marqué. Les équipements ont déjà leur garde (`managed_by`, comme
//! pour les moniteurs des étiquettes Docker).
//!
//! Les lectures, l'envoi d'un test et la gestion des abonnés d'une page ne
//! changent pas ce que le fichier décrit : ils passent.

use axum::Json;
use axum::extract::{Request, State};
use axum::http::{Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use sqlx::SqlitePool;

use crate::state::AppState;

/// Message renvoyé pour un objet géré par fichier.
pub const MANAGED_MESSAGE: &str = "This object is managed by a configuration file. Change it \
     in the file and apply it again, or detach everything with POST /api/config/release.";

/// Objet visé par une requête d'écriture : sa nature et son identifiant.
fn target_of(path: &str) -> Option<(&'static str, i64)> {
    let path = path.strip_prefix("/api").unwrap_or(path);
    let segments: Vec<&str> = path.trim_matches('/').split('/').collect();
    let id = |raw: &str| raw.parse::<i64>().ok();
    match segments.as_slice() {
        ["alerts", "rules", raw, ..] => Some(("rule", id(raw)?)),
        ["alerts", "silences", raw] => Some(("silence", id(raw)?)),
        ["notify", "channels", raw] => Some(("channel", id(raw)?)),
        ["status-pages", raw] => Some(("status_page", id(raw)?)),
        ["status-pages", raw, "items" | "access" | "logo"] => Some(("status_page", id(raw)?)),
        _ => None,
    }
}

async fn is_managed(pool: &SqlitePool, kind: &str, id: i64) -> bool {
    let sql = match kind {
        "rule" => {
            "SELECT 1 FROM alert_rules o JOIN config_managed m
                   ON m.kind = 'rule' AND m.key = o.uid WHERE o.id = ?"
        }
        "channel" => {
            "SELECT 1 FROM notification_channels o JOIN config_managed m
                      ON m.kind = 'channel' AND m.key = o.name WHERE o.id = ?"
        }
        "status_page" => {
            "SELECT 1 FROM status_pages o JOIN config_managed m
                          ON m.kind = 'status_page' AND m.key = o.slug WHERE o.id = ?"
        }
        "silence" => {
            "SELECT 1 FROM silences s JOIN config_managed m ON m.kind = 'silence' AND m.key =
             CASE WHEN s.target_id IS NULL THEN s.name ELSE s.name || '|' ||
             (SELECT t.kind || '|' || t.address FROM targets t WHERE t.id = s.target_id) END
             WHERE s.id = ?"
        }
        _ => return false,
    };
    sqlx::query(sql).bind(id).fetch_optional(pool).await.ok().flatten().is_some()
}

pub async fn guard(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let writes = matches!(*request.method(), Method::PUT | Method::DELETE | Method::POST);
    if writes
        && let Some((kind, id)) = target_of(request.uri().path())
        && is_managed(&state.pool, kind, id).await
    {
        return (StatusCode::CONFLICT, Json(json!({ "error": MANAGED_MESSAGE }))).into_response();
    }
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_objects_a_file_can_declare_are_recognised() {
        assert_eq!(target_of("/api/alerts/rules/4"), Some(("rule", 4)));
        assert_eq!(target_of("/alerts/rules/4/enable"), Some(("rule", 4)));
        assert_eq!(target_of("/alerts/silences/9"), Some(("silence", 9)));
        assert_eq!(target_of("/notify/channels/2"), Some(("channel", 2)));
        assert_eq!(target_of("/status-pages/3/items"), Some(("status_page", 3)));
        // Test d'un canal, abonnés d'une page, création : rien à protéger.
        assert_eq!(target_of("/notify/channels/2/test"), None);
        assert_eq!(target_of("/status-pages/3/subscribers/7"), None);
        assert_eq!(target_of("/alerts/rules"), None);
        assert_eq!(target_of("/targets/1"), None);
    }
}
