//! Ce que la page d'un domaine Active Directory montre au-delà des graphes.
//!
//! Le domaine, ses contrôleurs et leurs rôles, les groupes privilégiés avec
//! leurs membres, et les constats de sécurité se lisent dans ce que la sonde a
//! enregistré (`db::activedirectory`), jamais en réinterrogeant le contrôleur.
//!
//! `GET /targets/{id}/ad/findings` est la liste brute des constats, pensée pour
//! être consommée telle quelle (par un score, un export) : identifiant stable,
//! gravité, catégorie, titre, nombre d'objets et quelques exemples.
//!
//! Les dates sont en secondes Unix.

use std::collections::BTreeMap;

use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::routing::get;
use dumbmonit_collectors::activedirectory::{
    ConnectionView, DcView, DomainView, Finding, FsmoView, InventoryView, PolicyView,
    PrivilegedGroupView, ProbeView, ReplicationView, severity_counts,
};
use dumbmonit_proto::TargetId;
use serde::Serialize;

use crate::api::{ApiError, ApiResult};
use crate::db;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/targets/{id}/ad", get(overview))
        .route("/targets/{id}/ad/findings", get(findings))
}

/// La page entière, constats compris.
#[derive(Debug, Serialize, PartialEq)]
pub struct Overview {
    pub probed_at: Option<i64>,
    pub connection: Option<ConnectionView>,
    pub bind_error: Option<String>,
    pub domain: Option<DomainView>,
    pub policy: Option<PolicyView>,
    pub dcs: Vec<DcView>,
    pub fsmo: Vec<FsmoView>,
    pub replication: Option<ReplicationView>,
    pub privileged_groups: Vec<PrivilegedGroupView>,
    pub inventory: Option<InventoryView>,
    pub findings: Vec<Finding>,
    /// Nombre de constats par gravité avérés (`low`, `medium`, `high`, `critical`), zéros
    /// compris.
    pub finding_counts: BTreeMap<&'static str, usize>,
    pub errors: Vec<String>,
}

/// Les constats seuls.
#[derive(Debug, Serialize, PartialEq)]
pub struct Findings {
    pub probed_at: Option<i64>,
    /// Date de l'inventaire complet dont viennent les constats sur les comptes ;
    /// `None` tant qu'il n'a pas abouti (seuls les constats de stratégie et
    /// d'infrastructure sont alors présents).
    pub inventory_at: Option<i64>,
    pub findings: Vec<Finding>,
    pub counts: BTreeMap<&'static str, usize>,
}

pub fn build_overview(view: Option<ProbeView>) -> Overview {
    let Some(view) = view else {
        return Overview {
            probed_at: None,
            connection: None,
            bind_error: None,
            domain: None,
            policy: None,
            dcs: Vec::new(),
            fsmo: Vec::new(),
            replication: None,
            privileged_groups: Vec::new(),
            inventory: None,
            findings: Vec::new(),
            finding_counts: severity_counts(&[]),
            errors: Vec::new(),
        };
    };
    let finding_counts = severity_counts(&view.findings);
    let bound = view.bind_error.is_none();
    Overview {
        probed_at: Some(view.probed_at),
        connection: Some(view.connection),
        bind_error: view.bind_error,
        domain: view.domain,
        policy: view.policy,
        dcs: view.dcs,
        fsmo: view.fsmo,
        replication: bound.then_some(view.replication),
        privileged_groups: view.privileged_groups,
        inventory: view.inventory,
        findings: view.findings,
        finding_counts,
        errors: view.errors,
    }
}

pub fn build_findings(view: Option<ProbeView>) -> Findings {
    match view {
        None => Findings {
            probed_at: None,
            inventory_at: None,
            findings: Vec::new(),
            counts: severity_counts(&[]),
        },
        Some(view) => Findings {
            probed_at: Some(view.probed_at),
            inventory_at: view.inventory.as_ref().map(|inventory| inventory.refreshed_at),
            counts: severity_counts(&view.findings),
            findings: view.findings,
        },
    }
}

async fn overview(
    State(state): State<AppState>,
    Path(id): Path<TargetId>,
) -> ApiResult<Json<Overview>> {
    check(&state, id).await?;
    let view = db::activedirectory::load_view(&state.pool, id).await?;
    Ok(Json(build_overview(view)))
}

async fn findings(
    State(state): State<AppState>,
    Path(id): Path<TargetId>,
) -> ApiResult<Json<Findings>> {
    check(&state, id).await?;
    let view = db::activedirectory::load_view(&state.pool, id).await?;
    Ok(Json(build_findings(view)))
}

async fn check(state: &AppState, id: TargetId) -> ApiResult<()> {
    let target = db::targets::get(&state.pool, &state.cipher, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Device {id} not found.")))?;
    if target.kind != "activedirectory" {
        return Err(ApiError::BadRequest("This device is not an Active Directory domain.".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dumbmonit_collectors::activedirectory::FindingSeverity;

    fn finding(id: &str, severity: FindingSeverity) -> Finding {
        Finding {
            id: id.into(),
            severity,
            category: "kerberos".into(),
            title: id.into(),
            detail: String::new(),
            count: 2,
            samples: vec!["alice".into(), "bob".into()],
        }
    }

    #[test]
    fn avant_la_premiere_sonde_les_vues_sont_vides() {
        let overview = build_overview(None);
        assert!(overview.probed_at.is_none());
        assert_eq!(overview.finding_counts["critical"], 0);
        let findings = build_findings(None);
        assert!(findings.findings.is_empty());
        assert_eq!(findings.counts.len(), 4);
    }

    #[test]
    fn les_constats_sont_comptes_par_gravite() {
        let view = ProbeView {
            probed_at: 100,
            findings: vec![
                finding("asrep_roastable_users", FindingSeverity::High),
                finding("stale_accounts", FindingSeverity::Medium),
                finding("unconstrained_delegation", FindingSeverity::High),
            ],
            inventory: Some(InventoryView { refreshed_at: 90, ..InventoryView::default() }),
            ..ProbeView::default()
        };
        let findings = build_findings(Some(view.clone()));
        assert_eq!(findings.inventory_at, Some(90));
        assert_eq!(findings.counts["high"], 2);
        assert_eq!(findings.counts["medium"], 1);
        assert_eq!(findings.counts["critical"], 0);
        let overview = build_overview(Some(view));
        assert_eq!(overview.findings.len(), 3);
        assert!(overview.replication.is_some());
    }

    #[test]
    fn une_liaison_refusee_ne_montre_pas_de_replication() {
        let view = ProbeView {
            probed_at: 100,
            bind_error: Some("wrong user name or password (LDAP result 49)".into()),
            ..ProbeView::default()
        };
        let overview = build_overview(Some(view));
        assert!(overview.replication.is_none());
        assert!(overview.bind_error.is_some());
    }
}
