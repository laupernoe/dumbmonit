//! LUN iSCSI et zvols d'un NAS, et ce qui les utilise.
//!
//! * `GET /targets/{id}/luns` — les LUN (Synology) ou zvols (TrueNAS) de la
//!   cible, relus dans leurs dernières séries ; pour un Synology, les volumes
//!   qui en portent, avec leur occupation effective (sans les réservations
//!   épaisses) et leur sur-allocation fine ; pour chaque LUN, l'équipement qui
//!   le consomme et **son** remplissage, quand on le connaît ;
//! * `PUT /targets/{id}/luns/{lun}/consumer` — le lien manuel « ce LUN est
//!   l'entité X de l'équipement Y » (famille de capacité + entité, comme
//!   `GET /targets/{id}/forecasts` les nomme) ;
//! * `DELETE /targets/{id}/luns/{lun}/consumer` — le retire.
//!
//! Un LUN épais est opaque vu du NAS. Sans consommateur connu, l'interface dit
//! « réservé, usage interne inconnu » et rien n'alerte sur lui ; avec un
//! consommateur, ce sont les règles de cet équipement (datastore presque plein,
//! système de fichiers plein) qui alertent déjà, sur la bonne donnée.
//!
//! Le rapprochement automatique ne se fait que s'il est sûr : une seule entité
//! d'une famille connue (datastore vSphere, système de fichiers d'un agent)
//! porte le nom du LUN **et** une taille de 90 à 101 % de la sienne. Sinon,
//! c'est le lien manuel.

use std::collections::{BTreeMap, HashSet};

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::alerting::capacity;
use crate::api::{ApiError, ApiResult};
use crate::auth::middleware::AdminIdentity;
use crate::auth::teams::TeamAccess;
use crate::db;
use crate::state::AppState;
use crate::tsdb::InstantSeries;

/// Profondeur de recherche de la dernière valeur (un NAS interrogé toutes les
/// dix minutes a des séries plus vieilles que la fenêtre par défaut).
const LOOKBACK: &str = "1h";

/// Familles dans lesquelles un LUN épais est cherché automatiquement : celles
/// qui publient une taille et dont l'entité porte un nom choisi par l'humain.
const AUTO_FAMILIES: &[&str] = &["vsphere_datastore", "filesystem"];

/// Taille du consommateur rapportée à celle du LUN : un système de fichiers est
/// un peu plus petit que son disque (métadonnées), jamais plus grand.
const AUTO_SIZE_MIN: f64 = 0.90;
const AUTO_SIZE_MAX: f64 = 1.01;

/// Longueur maximale d'un identifiant de LUN ou d'une entité.
const MAX_TEXT: usize = 512;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/targets/{id}/luns", get(list))
        .route("/targets/{id}/luns/{lun}/consumer", put(link).delete(unlink))
}

// --------------------------------------------------------------------- vues

#[derive(Debug, Default, Serialize)]
pub struct LunsView {
    pub luns: Vec<LunView>,
    /// Volumes Synology qui portent au moins un LUN ; vide ailleurs.
    pub volumes: Vec<LunVolumeView>,
    /// Horodatage (secondes Unix) de la mesure la plus récente.
    pub sampled_at: Option<f64>,
}

#[derive(Debug, Default, Serialize)]
pub struct LunView {
    /// L'étiquette `lun` : UUID de DSM, nom du zvol.
    pub id: String,
    pub name: String,
    /// Volume (Synology) ou pool (TrueNAS) hôte.
    pub host_volume: String,
    /// `thick`, `thin` ou `unknown`.
    pub provisioning: String,
    /// Le mot de DSM (`BLUN_THICK`, `BLUN`, `FILE`…) ; vide pour un zvol.
    pub lun_type: String,
    pub status: String,
    pub size_bytes: Option<f64>,
    /// Ce que le LUN occupe sur son volume : sa taille s'il est épais, son
    /// allocation s'il est fin.
    pub reserved_bytes: Option<f64>,
    /// Ce qui est écrit, pour un LUN fin.
    pub allocated_bytes: Option<f64>,
    /// Allocation sur taille, pour un LUN fin.
    pub used_percent: Option<f64>,
    pub mapped: Option<bool>,
    pub sessions: Option<f64>,
    pub targets: Vec<LunTargetView>,
    /// Ce qui utilise le LUN, quand on le sait.
    pub consumer: Option<LunConsumer>,
}

#[derive(Debug, Default, Serialize)]
pub struct LunTargetView {
    pub iqn: String,
    pub name: String,
    pub sessions: f64,
    /// IQN des initiateurs connectés.
    pub initiators: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct LunConsumer {
    /// `manual` (lien enregistré) ou `auto` (rapprochement nom + taille).
    pub source: &'static str,
    pub target_id: i64,
    pub target_name: String,
    pub family: String,
    pub family_label: String,
    pub entity: String,
    /// Remplissage vu par le consommateur ; `null` si ses séries manquent.
    pub used_percent: Option<f64>,
    pub used_bytes: Option<f64>,
    pub size_bytes: Option<f64>,
}

#[derive(Debug, Default, Serialize)]
pub struct LunVolumeView {
    pub volume: String,
    pub name: String,
    pub total_bytes: Option<f64>,
    pub used_bytes: Option<f64>,
    pub used_percent: Option<f64>,
    pub lun_count: Option<f64>,
    /// Somme des tailles des LUN épais.
    pub reserved_bytes: Option<f64>,
    pub thin_allocated_bytes: Option<f64>,
    pub thin_provisioned_bytes: Option<f64>,
    /// Espace promis aux LUN fins et pas encore écrit.
    pub thin_promised_bytes: Option<f64>,
    /// Données hors LUN.
    pub data_bytes: Option<f64>,
    pub effective_total_bytes: Option<f64>,
    pub effective_used_bytes: Option<f64>,
    pub effective_used_percent: Option<f64>,
    /// Espace promis rapporté à l'espace libre, en pourcentage.
    pub overcommit_percent: Option<f64>,
    pub fully_reserved: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct LinkBody {
    pub target_id: i64,
    pub family: String,
    pub entity: String,
}

// ------------------------------------------------------------- gestionnaires

/// Préfixe des séries de LUN selon le type de la cible.
fn lun_prefix(kind: &str) -> Option<&'static str> {
    match kind {
        "synology" => Some("dumbmonit_synology_lun_"),
        "truenas" => Some("dumbmonit_truenas_zvol_"),
        _ => None,
    }
}

async fn nas_prefix(state: &AppState, id: i64) -> ApiResult<(String, &'static str)> {
    let kind: Option<String> = sqlx::query_scalar("SELECT kind FROM targets WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(anyhow::Error::from)?;
    let kind = kind.ok_or_else(|| ApiError::NotFound(format!("Monitor {id} not found.")))?;
    let prefix = lun_prefix(&kind).ok_or_else(|| {
        ApiError::NotFound(format!("Monitor {id} is not a NAS with iSCSI LUNs or zvols."))
    })?;
    Ok((kind, prefix))
}

async fn latest(state: &AppState, pattern: &str, filter: Option<&str>) -> Vec<InstantSeries> {
    let selector = match filter {
        Some(filter) => format!(r#"__name__=~"{pattern}", {filter}"#),
        None => format!(r#"__name__=~"{pattern}""#),
    };
    let query = format!("last_over_time({{{selector}}}[{LOOKBACK}]) keep_metric_names");
    match state.victoria.query(&query).await {
        Ok(series) => series,
        Err(error) => {
            tracing::warn!(?error, pattern, "LUN series not read");
            Vec::new()
        }
    }
}

async fn list(
    TeamAccess(access): TeamAccess,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<Json<LunsView>> {
    let (kind, prefix) = nas_prefix(&state, id).await?;
    let target = format!(r#"target="{id}""#);
    let mut series = latest(&state, &format!("{prefix}.*"), Some(&target)).await;
    if kind == "synology" {
        series.extend(latest(&state, "dumbmonit_synology_volume_.*", Some(&target)).await);
    }
    let mut view = build_view(&series, prefix);

    let visible = access.visible_targets(&state.pool).await?;
    let can_see = |target: i64| visible.as_ref().is_none_or(|set| set.contains(&target));
    let links = db::lun_links::list_for(&state.pool, id).await?;
    let mut linked: HashSet<String> = HashSet::new();
    for link in links.iter().filter(|link| can_see(link.consumer_target_id)) {
        let Some(lun) = view.luns.iter_mut().find(|lun| lun.id == link.lun) else { continue };
        linked.insert(lun.id.clone());
        lun.consumer = Some(
            consumer(&state, "manual", link.consumer_target_id, &link.family, &link.entity).await,
        );
    }

    // Le rapprochement automatique, seulement pour ce qui reste sans lien et
    // dont on connaît la taille.
    let unlinked = view
        .luns
        .iter()
        .any(|lun| !linked.contains(&lun.id) && lun.size_bytes.is_some_and(|s| s > 0.0));
    if unlinked {
        let candidates = auto_candidates(&state, id).await;
        for lun in view.luns.iter_mut().filter(|lun| !linked.contains(&lun.id)) {
            let Some(size) = lun.size_bytes else { continue };
            let found = auto_match(&lun.name, size, &candidates);
            if let Some(candidate) = found.filter(|c| can_see(c.target_id)) {
                let (target_id, family, entity) =
                    (candidate.target_id, candidate.family, candidate.entity.clone());
                lun.consumer = Some(consumer(&state, "auto", target_id, family, &entity).await);
            }
        }
    }
    Ok(Json(view))
}

async fn link(
    AdminIdentity(me): AdminIdentity,
    State(state): State<AppState>,
    Path((id, lun)): Path<(i64, String)>,
    Json(body): Json<LinkBody>,
) -> ApiResult<StatusCode> {
    nas_prefix(&state, id).await?;
    let lun = lun.trim();
    let entity = body.entity.trim();
    if lun.is_empty() || lun.len() > MAX_TEXT {
        return Err(ApiError::BadRequest("The LUN identifier is empty or too long.".into()));
    }
    if entity.is_empty() || entity.len() > MAX_TEXT {
        return Err(ApiError::BadRequest("'entity' is empty or too long.".into()));
    }
    if capacity::family(&body.family).is_none() {
        return Err(ApiError::BadRequest(format!("Unknown capacity family: {}.", body.family)));
    }
    if body.target_id == id {
        return Err(ApiError::BadRequest(
            "A LUN is used by another monitor (a hypervisor, a server), not by the NAS itself."
                .into(),
        ));
    }
    let exists: Option<i64> = sqlx::query_scalar("SELECT id FROM targets WHERE id = ?")
        .bind(body.target_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(anyhow::Error::from)?;
    if exists.is_none() {
        return Err(ApiError::BadRequest(format!("Monitor {} not found.", body.target_id)));
    }
    db::lun_links::upsert(&state.pool, id, lun, body.target_id, &body.family, entity, &me.label())
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn unlink(
    _: AdminIdentity,
    State(state): State<AppState>,
    Path((id, lun)): Path<(i64, String)>,
) -> ApiResult<StatusCode> {
    nas_prefix(&state, id).await?;
    if !db::lun_links::delete(&state.pool, id, lun.trim()).await? {
        return Err(ApiError::NotFound("This LUN has no manual link.".into()));
    }
    Ok(StatusCode::NO_CONTENT)
}

// ------------------------------------------------------------- consommateur

/// Le consommateur et son remplissage, lu dans la famille de capacité.
async fn consumer(
    state: &AppState,
    source: &'static str,
    target_id: i64,
    family_id: &str,
    entity: &str,
) -> LunConsumer {
    let target_name: String = sqlx::query_scalar("SELECT name FROM targets WHERE id = ?")
        .bind(target_id)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();
    let family = capacity::family(family_id);
    let mut out = LunConsumer {
        source,
        target_id,
        target_name,
        family: family_id.to_string(),
        family_label: family.map_or_else(|| family_id.to_string(), |f| f.label.to_string()),
        entity: entity.to_string(),
        used_percent: None,
        used_bytes: None,
        size_bytes: None,
    };
    let Some(family) = family else { return out };
    let filter = format!("{{target=\"{target_id}\"}}");
    out.used_percent = entity_value(state, family.percent, &filter, entity).await;
    if let Some(used) = family.used {
        out.used_bytes = entity_value(state, used, &filter, entity).await;
    }
    if let Some(size) = family.size {
        out.size_bytes = entity_value(state, size, &filter, entity).await;
    }
    out
}

/// Dernière valeur de l'expression pour l'entité nommée `entity`.
async fn entity_value(state: &AppState, expr: &str, filter: &str, entity: &str) -> Option<f64> {
    let query = format!("last_over_time(({expr})[{LOOKBACK}:])");
    let series = state.victoria.query_fresh_filtered(&query, Some(filter)).await.ok()?;
    series
        .iter()
        .find(|s| capacity::entity(&s.metric) == entity)
        .and_then(|s| s.value.1.parse::<f64>().ok())
        .filter(|v| v.is_finite())
}

/// Une entité de capacité candidate au rapprochement automatique.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub target_id: i64,
    pub family: &'static str,
    pub entity: String,
    pub size: f64,
}

/// Les entités des familles de [`AUTO_FAMILIES`], hors le NAS lui-même.
async fn auto_candidates(state: &AppState, nas: i64) -> Vec<Candidate> {
    let mut out = Vec::new();
    for family in AUTO_FAMILIES.iter().filter_map(|id| capacity::family(id)) {
        let Some(size) = family.size else { continue };
        let query = format!("last_over_time(({size})[{LOOKBACK}:])");
        let Ok(series) = state.victoria.query_fresh_filtered(&query, None).await else {
            continue;
        };
        for item in series {
            let target = item.metric.get("target").and_then(|t| t.parse::<i64>().ok());
            let value = item.value.1.parse::<f64>().ok().filter(|v| v.is_finite() && *v > 0.0);
            let (Some(target_id), Some(size)) = (target, value) else { continue };
            if target_id == nas {
                continue;
            }
            out.push(Candidate {
                target_id,
                family: family.id,
                entity: capacity::entity(&item.metric),
                size,
            });
        }
    }
    out
}

/// Le dernier segment d'un nom, sans casse : `/mnt/LUN-01` et `LUN-01` se
/// valent, `E:\` reste `e:`.
fn short_name(name: &str) -> String {
    let trimmed = name.trim().trim_end_matches(['/', '\\']);
    trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed).to_ascii_lowercase()
}

/// Le seul candidat qui porte le nom du LUN et une taille compatible ; aucun
/// s'il y en a zéro ou plusieurs.
pub fn auto_match<'a>(name: &str, size: f64, candidates: &'a [Candidate]) -> Option<&'a Candidate> {
    let wanted = short_name(name);
    if wanted.is_empty() || size <= 0.0 {
        return None;
    }
    let mut found = candidates.iter().filter(|c| {
        let ratio = c.size / size;
        short_name(&c.entity) == wanted && (AUTO_SIZE_MIN..=AUTO_SIZE_MAX).contains(&ratio)
    });
    let first = found.next()?;
    found.next().is_none().then_some(first)
}

// ------------------------------------------------------------- assemblage

fn value(series: &InstantSeries) -> Option<f64> {
    series.value.1.parse::<f64>().ok().filter(|v| v.is_finite())
}

fn label(series: &InstantSeries, name: &str) -> String {
    series.metric.get(name).cloned().unwrap_or_default()
}

/// Reconstruit la vue à partir des dernières séries ; `prefix` est celui des
/// séries de LUN (`dumbmonit_synology_lun_`, `dumbmonit_truenas_zvol_`).
pub fn build_view(series: &[InstantSeries], prefix: &str) -> LunsView {
    let mut view = LunsView::default();
    let mut luns: BTreeMap<String, LunView> = BTreeMap::new();
    let mut targets: BTreeMap<(String, String), LunTargetView> = BTreeMap::new();
    let mut volumes: BTreeMap<String, LunVolumeView> = BTreeMap::new();

    for s in series {
        let name = label(s, "__name__");
        let v = value(s);
        view.sampled_at = Some(view.sampled_at.map_or(s.value.0, |at| at.max(s.value.0)));

        if let Some(rest) = name.strip_prefix(prefix) {
            let id = label(s, "lun");
            if id.is_empty() {
                continue;
            }
            let lun = luns.entry(id.clone()).or_insert_with(|| LunView {
                id: id.clone(),
                name: label(s, "name"),
                host_volume: label(s, "host_volume"),
                provisioning: "unknown".into(),
                ..Default::default()
            });
            match rest {
                "info" => {
                    lun.provisioning = Some(label(s, "provisioning"))
                        .filter(|p| !p.is_empty())
                        .unwrap_or_else(|| "unknown".into());
                    lun.lun_type = label(s, "lun_type");
                    lun.status = label(s, "status");
                }
                "size_bytes" => lun.size_bytes = v,
                "reserved_bytes" => lun.reserved_bytes = v,
                "allocated_bytes" => lun.allocated_bytes = v,
                "used_percent" => lun.used_percent = v,
                "mapped" => lun.mapped = v.map(|v| v != 0.0),
                "sessions" => lun.sessions = v,
                "target_sessions" => {
                    let key = (id.clone(), label(s, "target_iqn"));
                    targets.insert(
                        key,
                        LunTargetView {
                            iqn: label(s, "target_iqn"),
                            name: label(s, "target_name"),
                            sessions: v.unwrap_or(0.0),
                            initiators: label(s, "initiators")
                                .split(',')
                                .map(str::trim)
                                .filter(|i| !i.is_empty())
                                .map(str::to_string)
                                .collect(),
                        },
                    );
                }
                _ => {}
            }
            continue;
        }

        if let Some(rest) = name.strip_prefix("dumbmonit_synology_volume_") {
            let id = label(s, "volume");
            if id.is_empty() {
                continue;
            }
            let volume = volumes.entry(id.clone()).or_insert_with(|| LunVolumeView {
                volume: id.clone(),
                name: label(s, "name"),
                ..Default::default()
            });
            match rest {
                "total_bytes" => volume.total_bytes = v,
                "used_bytes" => volume.used_bytes = v,
                "used_percent" => volume.used_percent = v,
                "lun_count" => volume.lun_count = v,
                "lun_reserved_bytes" => volume.reserved_bytes = v,
                "lun_thin_allocated_bytes" => volume.thin_allocated_bytes = v,
                "lun_thin_provisioned_bytes" => volume.thin_provisioned_bytes = v,
                "thin_promised_bytes" => volume.thin_promised_bytes = v,
                "data_bytes" => volume.data_bytes = v,
                "effective_total_bytes" => volume.effective_total_bytes = v,
                "effective_used_bytes" => volume.effective_used_bytes = v,
                "effective_used_percent" => volume.effective_used_percent = v,
                "thin_overcommit_percent" => volume.overcommit_percent = v,
                "lun_fully_reserved" => volume.fully_reserved = v.map(|v| v != 0.0),
                _ => {}
            }
        }
    }

    for ((lun, _), target) in targets {
        if let Some(view) = luns.get_mut(&lun) {
            view.targets.push(target);
        }
    }
    let mut luns: Vec<LunView> = luns.into_values().collect();
    luns.sort_by(|a, b| a.host_volume.cmp(&b.host_volume).then_with(|| a.name.cmp(&b.name)));
    view.luns = luns;
    // Seuls les volumes qui portent un LUN : les autres sont déjà dans le panneau.
    view.volumes = volumes.into_values().filter(|v| v.lun_count.is_some_and(|n| n > 0.0)).collect();
    view
}

#[cfg(test)]
mod tests {
    use super::*;

    fn serie(name: &str, value: f64, labels: &[(&str, &str)]) -> InstantSeries {
        let mut metric = BTreeMap::new();
        metric.insert("__name__".to_string(), name.to_string());
        metric.insert("target".to_string(), "7".to_string());
        for (k, v) in labels {
            metric.insert((*k).to_string(), (*v).to_string());
        }
        InstantSeries { metric, value: (1_700_000_000.0, value.to_string()) }
    }

    const LUN: &[(&str, &str)] = &[("lun", "u1"), ("name", "vmfs"), ("host_volume", "volume_1")];

    #[test]
    fn la_vue_assemble_lun_cibles_et_volumes() {
        let mut info = LUN.to_vec();
        info.extend([("provisioning", "thick"), ("lun_type", "BLUN_THICK"), ("status", "normal")]);
        let mut target = LUN.to_vec();
        target.extend([
            ("target_iqn", "iqn.2000-01.com.synology:nas.t1"),
            ("target_name", "T1"),
            ("initiators", "iqn.a,iqn.b"),
        ]);
        let volume = [("volume", "volume_1"), ("name", "/volume1")];
        let series = vec![
            serie("dumbmonit_synology_lun_info", 1.0, &info),
            serie("dumbmonit_synology_lun_size_bytes", 9e12, LUN),
            serie("dumbmonit_synology_lun_reserved_bytes", 9e12, LUN),
            serie("dumbmonit_synology_lun_sessions", 2.0, LUN),
            serie("dumbmonit_synology_lun_target_sessions", 2.0, &target),
            serie("dumbmonit_synology_volume_total_bytes", 10e12, &volume),
            serie("dumbmonit_synology_volume_used_percent", 93.0, &volume),
            serie("dumbmonit_synology_volume_effective_used_percent", 30.0, &volume),
            serie("dumbmonit_synology_volume_lun_count", 1.0, &volume),
            serie("dumbmonit_synology_volume_lun_reserved_bytes", 9e12, &volume),
            // Un volume sans LUN n'apparaît pas.
            serie(
                "dumbmonit_synology_volume_effective_used_percent",
                12.0,
                &[("volume", "volume_2"), ("name", "/volume2")],
            ),
        ];
        let view = build_view(&series, "dumbmonit_synology_lun_");
        assert_eq!(view.luns.len(), 1);
        let lun = &view.luns[0];
        assert_eq!(lun.provisioning, "thick");
        assert_eq!(lun.size_bytes, Some(9e12));
        assert_eq!(lun.used_percent, None, "épais : opaque vu du NAS");
        assert_eq!(lun.targets.len(), 1);
        assert_eq!(lun.targets[0].initiators, vec!["iqn.a", "iqn.b"]);
        assert_eq!(view.volumes.len(), 1);
        let volume = &view.volumes[0];
        assert_eq!(volume.used_percent, Some(93.0));
        assert_eq!(volume.effective_used_percent, Some(30.0));
        assert_eq!(volume.reserved_bytes, Some(9e12));
    }

    #[test]
    fn les_zvols_se_lisent_avec_le_meme_code() {
        let labels = [("lun", "tank/vm"), ("name", "tank/vm"), ("host_volume", "tank")];
        let mut info = labels.to_vec();
        info.push(("provisioning", "thin"));
        let series = vec![
            serie("dumbmonit_truenas_zvol_info", 1.0, &info),
            serie("dumbmonit_truenas_zvol_used_percent", 25.0, &labels),
            serie("dumbmonit_truenas_zvol_count", 1.0, &[]),
        ];
        let view = build_view(&series, "dumbmonit_truenas_zvol_");
        assert_eq!(view.luns.len(), 1, "le décompte sans `lun` est ignoré");
        assert_eq!(view.luns[0].provisioning, "thin");
        assert_eq!(view.luns[0].used_percent, Some(25.0));
        assert!(view.volumes.is_empty());
    }

    fn candidate(target_id: i64, entity: &str, size: f64) -> Candidate {
        Candidate { target_id, family: "vsphere_datastore", entity: entity.to_string(), size }
    }

    #[test]
    fn le_rapprochement_automatique_exige_nom_et_taille_et_un_seul_candidat() {
        let size = 1000.0;
        let only = [candidate(3, "VMFS", 990.0), candidate(4, "other", 1000.0)];
        assert_eq!(auto_match("vmfs", size, &only).map(|c| c.target_id), Some(3));
        // Un point de montage dont le dernier segment est le nom du LUN.
        let mount = [candidate(5, "/mnt/vmfs", 950.0)];
        assert_eq!(auto_match("vmfs", size, &mount).map(|c| c.target_id), Some(5));
        // Bon nom, mauvaise taille.
        assert!(auto_match("vmfs", size, &[candidate(3, "vmfs", 500.0)]).is_none());
        assert!(auto_match("vmfs", size, &[candidate(3, "vmfs", 1100.0)]).is_none());
        // Deux candidats : rien n'est sûr.
        let twice = [candidate(3, "vmfs", 990.0), candidate(6, "vmfs", 995.0)];
        assert!(auto_match("vmfs", size, &twice).is_none());
        assert!(auto_match("", size, &only).is_none());
    }
}
