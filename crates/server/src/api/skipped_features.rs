//! Les fonctionnalités facultatives qu'un droit manquant a fait sauter.
//!
//! Sur Proxmox VE, Proxmox Backup Server et Proxmox Datacenter Manager, quelques
//! options appellent un endpoint qui exige un droit de plus que le minimum
//! documenté. Un 403 n'y est ni une erreur de sonde ni une alerte, mais
//! l'utilisateur doit pouvoir savoir qu'un droit manque : le collecteur publie
//! `<type>_feature_skipped_info{feature, privilege}` à chaque interrogation où
//! le refus a eu lieu, et cette route la relit pour la page du moniteur.
//!
//! Rien n'est interrogé sur l'équipement : la réponse reflète la dernière
//! collecte. Une liste vide veut dire « rien de sauté ».

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use dumbmonit_proto::TargetId;
use serde::Serialize;

use crate::api::{ApiError, ApiResult};
use crate::db;
use crate::state::AppState;
use crate::tsdb::InstantSeries;

pub fn routes() -> Router<AppState> {
    Router::new().route("/targets/{id}/skipped-features", get(skipped_features))
}

/// Types dont les collecteurs publient la série.
const KINDS: [&str; 3] = ["proxmox", "pbs", "pdm"];

#[derive(Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct SkippedFeature {
    /// Identifiant technique de la fonctionnalité (`guest_agent`, `updates`…).
    pub feature: String,
    /// Droit qui manque, tel que l'équipement le nomme.
    pub privilege: String,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct SkippedFeaturesView {
    pub features: Vec<SkippedFeature>,
}

/// Fenêtre de lecture : trois intervalles de sonde, entre cinq minutes et une
/// heure, comme pour les autres vues lues dans le magasin de métriques.
fn lookback(interval_secs: u64) -> u64 {
    (interval_secs * 3).clamp(300, 3600)
}

fn query(id: TargetId, kind: &str, window: u64) -> String {
    format!(
        "last_over_time(dumbmonit_{kind}_feature_skipped_info{{target=\"{id}\"}}[{window}s]) \
         keep_metric_names"
    )
}

/// Dédoublonne et trie les séries ; une série sans fonctionnalité est ignorée.
fn assemble(series: &[InstantSeries]) -> SkippedFeaturesView {
    let mut features: Vec<SkippedFeature> = series
        .iter()
        .filter_map(|series| {
            let feature = series.metric.get("feature").filter(|f| !f.is_empty())?.clone();
            let privilege = series.metric.get("privilege").cloned().unwrap_or_default();
            Some(SkippedFeature { feature, privilege })
        })
        .collect();
    features.sort();
    features.dedup();
    SkippedFeaturesView { features }
}

async fn skipped_features(
    State(state): State<AppState>,
    Path(id): Path<TargetId>,
) -> ApiResult<Json<SkippedFeaturesView>> {
    let target = db::targets::get(&state.pool, &state.cipher, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Monitor {id} not found.")))?;
    let Some(kind) = KINDS.into_iter().find(|kind| *kind == target.kind) else {
        return Ok(Json(SkippedFeaturesView { features: Vec::new() }));
    };
    let window = lookback(target.interval.as_secs());
    let series = state
        .victoria
        .query(&query(id, kind, window))
        .await
        .map_err(|error| ApiError::BadRequest(format!("{error:#}")))?;
    Ok(Json(assemble(&series)))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn serie(feature: &str, privilege: &str) -> InstantSeries {
        let mut metric = BTreeMap::new();
        metric.insert("feature".to_string(), feature.to_string());
        metric.insert("privilege".to_string(), privilege.to_string());
        InstantSeries { metric, value: (0.0, "1".to_string()) }
    }

    #[test]
    fn les_notes_sont_triees_et_dedoublonnees() {
        let view = assemble(&[
            serie("updates", "Sys.Modify"),
            serie("guest_agent", "VM.Monitor"),
            serie("updates", "Sys.Modify"),
            InstantSeries { metric: BTreeMap::new(), value: (0.0, "1".to_string()) },
        ]);
        let names: Vec<&str> = view.features.iter().map(|f| f.feature.as_str()).collect();
        assert_eq!(names, ["guest_agent", "updates"]);
        assert_eq!(view.features[1].privilege, "Sys.Modify");
    }

    #[test]
    fn la_requete_cible_le_type_et_la_cible() {
        let q = query(42, "pbs", 300);
        assert!(q.contains("dumbmonit_pbs_feature_skipped_info{target=\"42\"}[300s]"), "{q}");
    }
}
