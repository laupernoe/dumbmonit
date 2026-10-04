//! Note de sécurité d'un équipement et vue d'ensemble du parc
//! (`crate::security`).
//!
//! * `GET /targets/{id}/security` — note, lettre et contrôles d'un équipement.
//!   Un type sans contrôle répond `supported: false`, pas 404 : la page d'un
//!   équipement l'appelle quel que soit son type.
//! * `GET /security/summary` — tous les équipements notés, du pire au meilleur.

use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::routing::get;
use dumbmonit_proto::TargetId;
use serde::Serialize;

use crate::api::{ApiError, ApiResult};
use crate::db;
use crate::security::{self, Facts, Grade, Outcome, Report};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/targets/{id}/security", get(target_security))
        .route("/security/summary", get(summary))
}

async fn target_security(
    State(state): State<AppState>,
    Path(id): Path<TargetId>,
) -> ApiResult<Json<Report>> {
    let target = db::targets::get(&state.pool, &state.cipher, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Device {id} not found.")))?;
    let facts = if security::provider_for(&target.kind).is_some() {
        security::facts_for(&state, &target).await?
    } else {
        Facts::default()
    };
    Ok(Json(security::build_report(target.id, &target.name, &target.kind, &facts)))
}

#[derive(Debug, Serialize)]
pub struct Summary {
    pub evaluated_at: String,
    pub devices: Vec<SummaryRow>,
}

#[derive(Debug, Serialize)]
pub struct SummaryRow {
    pub target_id: TargetId,
    pub target_name: String,
    pub kind: String,
    pub score: Option<u32>,
    pub grade: Option<Grade>,
    pub capped: bool,
    pub pass: usize,
    pub fail: usize,
    pub unknown: usize,
    /// Jusqu'à trois titres de contrôles en échec, les plus lourds d'abord.
    pub top_failures: Vec<String>,
}

/// Du pire au meilleur ; les équipements non notés en dernier, par nom.
pub fn summarize(reports: Vec<Report>) -> Vec<SummaryRow> {
    let mut rows: Vec<SummaryRow> = reports
        .into_iter()
        .map(|report| SummaryRow {
            top_failures: report
                .checks
                .iter()
                .filter(|c| c.result == Outcome::Fail)
                .take(3)
                .map(|c| c.title.clone())
                .collect(),
            target_id: report.target_id,
            target_name: report.target_name,
            kind: report.kind,
            score: report.score,
            grade: report.grade,
            capped: report.capped,
            pass: report.counts.pass,
            fail: report.counts.fail,
            unknown: report.counts.unknown,
        })
        .collect();
    rows.sort_by(|a, b| {
        let key = |row: &SummaryRow| (row.score.is_none(), row.grade.map(std::cmp::Reverse));
        key(a)
            .cmp(&key(b))
            .then(a.score.cmp(&b.score))
            .then_with(|| a.target_name.to_lowercase().cmp(&b.target_name.to_lowercase()))
    });
    rows
}

async fn summary(State(state): State<AppState>) -> ApiResult<Json<Summary>> {
    let reports = security::all_reports(&state).await?;
    Ok(Json(Summary {
        evaluated_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        devices: summarize(reports),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(id: TargetId, name: &str, score: Option<u32>) -> Report {
        let mut report = security::build_report(id, name, "agent", &Facts::default());
        report.score = score;
        report.grade = score.map(security::grade_for);
        report
    }

    #[test]
    fn le_resume_va_du_pire_au_meilleur() {
        let rows = summarize(vec![
            report(1, "good", Some(95)),
            report(2, "unrated", None),
            report(3, "bad", Some(20)),
            report(4, "fair", Some(65)),
        ]);
        let names: Vec<&str> = rows.iter().map(|r| r.target_name.as_str()).collect();
        assert_eq!(names, ["bad", "fair", "good", "unrated"]);
    }

    #[test]
    fn la_lettre_prime_sur_la_note_quand_un_plafond_s_applique() {
        // La lettre décide avant la note : un 86 plafonné à C reste meilleur
        // qu'un D.
        let mut capped = report(1, "capped", Some(86));
        capped.grade = Some(Grade::C);
        let rows = summarize(vec![capped, report(2, "d", Some(50))]);
        assert_eq!(rows[0].target_name, "d");
    }
}
