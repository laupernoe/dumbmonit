//! Moniteurs créés par les étiquettes Docker des agents.
//!
//! L'agent envoie, à chaque lot, la liste complète des moniteurs que les
//! étiquettes `dumbmonit.*` de ses conteneurs déclarent (`DeclaredMonitor`). Le
//! serveur réconcilie :
//!
//! - **création** d'un moniteur absent, rattaché à l'agent comme parent (si la
//!   machine tombe, ses moniteurs ne sonnent pas en cascade) ;
//! - **mise à jour** de celui que la clé désigne déjà, réactivé s'il avait été archivé ;
//! - **archivage** (désactivé, daté, historique conservé) de celui que plus aucune
//!   étiquette ne déclare — jamais quand l'agent n'a pas pu lire Docker, jamais
//!   quand le module Docker est décoché pour la machine ;
//! - **adoption** : un moniteur existant de même type et adresse n'est repris
//!   que si l'étiquette `dumbmonit.adopt=true` le demande, sinon le doublon est signalé.
//!
//! Un moniteur géré ne se modifie que par ses étiquettes (voir [`refuse_if_managed`]) ;
//! « Release » le rend à la gestion manuelle.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use axum::extract::{Path, State};
use axum::http::StatusCode;
use dumbmonit_proto::{DeclaredMonitor, MAX_DECLARED_MONITORS, TargetId};

use crate::api::targets::TargetPayload;
use crate::api::{ApiError, ApiResult};
use crate::db;
use crate::state::AppState;

/// Types de moniteurs que les étiquettes peuvent déclarer.
const KINDS: &[&str] = &["http", "tcp", "ping"];

/// Délai au bout duquel une liste inchangée est quand même réconciliée : une
/// libération ou une suppression à la main doit être rattrapée.
const RECHECK: Duration = Duration::from_secs(300);

/// Dernière réconciliation par instance (répertoire de données) et par agent.
type Key = (std::path::PathBuf, TargetId);

static LAST: LazyLock<Mutex<HashMap<Key, (u64, Instant)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Ce qu'une réconciliation a fait ; sert aux journaux et aux tests.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub created: usize,
    pub updated: usize,
    pub adopted: usize,
    pub archived: usize,
    pub restored: usize,
    /// Déclarations refusées, avec la raison (`clé: raison`).
    pub rejected: Vec<String>,
}

/// Empreinte de la liste et de l'adresse de l'hôte.
fn fingerprint(host: &str, declared: &[DeclaredMonitor]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    host.hash(&mut hasher);
    format!("{declared:?}").hash(&mut hasher);
    hasher.finish()
}

/// Appelé après chaque lot accepté. Les erreurs sont journalisées, jamais
/// renvoyées : un lot de mesures ne doit pas échouer à cause d'une étiquette.
///
/// `hostname` est le nom que la machine donne d'elle-même : c'est ce que `{host}`
/// devient dans une adresse. Le nom de domaine du conteneur n'a pas de sens ici,
/// et l'adresse de la cible, elle, est l'identifiant de la machine.
pub async fn on_batch(
    state: &AppState,
    agent: TargetId,
    hostname: &str,
    declared: Vec<DeclaredMonitor>,
) {
    let host = hostname.trim().to_string();
    // Le nom est inséré dans des adresses : rien qui puisse en changer la forme.
    if !host.is_empty()
        && !host.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ':'))
    {
        tracing::warn!(agent, "host name unfit for monitor addresses, labels ignored");
        return;
    }
    let print = fingerprint(&host, &declared);
    let key: Key = (state.config.data_dir.clone(), agent);
    if let Ok(last) = LAST.lock()
        && last.get(&key).is_some_and(|(p, at)| *p == print && at.elapsed() < RECHECK)
    {
        return;
    }
    match reconcile(state, agent, &host, declared).await {
        Ok(outcome) => {
            if let Ok(mut last) = LAST.lock() {
                last.insert(key, (print, Instant::now()));
            }
            if outcome != Outcome::default() {
                tracing::info!(
                    agent,
                    created = outcome.created,
                    updated = outcome.updated,
                    adopted = outcome.adopted,
                    archived = outcome.archived,
                    restored = outcome.restored,
                    rejected = outcome.rejected.len(),
                    "monitors from Docker labels reconciled"
                );
            }
            for reason in &outcome.rejected {
                tracing::warn!(agent, "Docker label monitor ignored: {reason}");
            }
        }
        Err(error) => tracing::warn!(agent, %error, "Docker label monitors not reconciled"),
    }
}

/// Remplace `{host}` par l'hôte, entre crochets dans une adresse `hôte:port` ou
/// une URL quand c'est une IPv6.
pub fn resolve_address(address: &str, host: &str) -> String {
    let host = if host.contains(':') { format!("[{host}]") } else { host.to_string() };
    address.replace("{host}", &host)
}

pub async fn reconcile(
    state: &AppState,
    agent: TargetId,
    host: &str,
    declared: Vec<DeclaredMonitor>,
) -> anyhow::Result<Outcome> {
    let mut outcome = Outcome::default();
    let existing: HashMap<String, (TargetId, db::targets::Managed)> =
        db::targets::managed_of(&state.pool, agent)
            .await?
            .into_iter()
            .map(|(id, managed)| (managed.key.clone(), (id, managed)))
            .collect();
    let mut wanted: HashSet<String> = HashSet::new();

    for item in declared.into_iter().take(MAX_DECLARED_MONITORS) {
        wanted.insert(item.key.clone());
        let key = item.key.clone();
        if !KINDS.contains(&item.kind.as_str()) {
            outcome.rejected.push(format!("{key}: unsupported type \"{}\"", item.kind));
            continue;
        }
        let payload = TargetPayload {
            name: item.name.clone(),
            address: resolve_address(&item.address, host),
            kind: item.kind.clone(),
            profile_id: None,
            parent_id: Some(agent),
            via_agent: None,
            interval_secs: item.interval_secs,
            enabled: Some(true),
            tags: sanitized_tags(&item.tags),
            group_name: Some(Some(item.folder.clone())),
            location_id: None,
            credential: None,
        };
        let input = match payload.validate(state, None) {
            Ok(input) => input,
            Err(error) => {
                outcome.rejected.push(format!("{key}: {}", error.into_parts().1));
                continue;
            }
        };

        if let Some((id, managed)) = existing.get(&key) {
            // Mise en pause à la main : respectée. Seul un archivage la lève.
            let before = db::targets::get(&state.pool, &state.cipher, *id).await?;
            let Some(before) = before else { continue };
            let restored = managed.missing_since.is_some();
            let mut input = input;
            input.enabled = before.enabled || restored;
            if restored || changed(&before, &input) {
                if let Err(error) =
                    db::targets::update(&state.pool, &state.cipher, *id, &input).await
                {
                    outcome.rejected.push(format!("{key}: {error:#}"));
                    continue;
                }
                if restored {
                    db::targets::set_missing(&state.pool, *id, false).await?;
                    outcome.restored += 1;
                } else {
                    outcome.updated += 1;
                }
            }
            continue;
        }

        match db::targets::find_by_address(&state.pool, &input.kind, &input.address).await? {
            Some((id, None)) if item.adopt => {
                db::targets::update(&state.pool, &state.cipher, id, &input).await?;
                db::targets::set_managed(&state.pool, id, agent, &key).await?;
                outcome.adopted += 1;
            }
            Some((_, None)) => outcome.rejected.push(format!(
                "{key}: a monitor for this address already exists; set dumbmonit.adopt=true to \
                 let the labels manage it"
            )),
            Some(_) => outcome
                .rejected
                .push(format!("{key}: this address is already managed by other labels")),
            None => {
                let id = db::targets::create(&state.pool, &state.cipher, &input).await?;
                db::targets::set_managed(&state.pool, id, agent, &key).await?;
                outcome.created += 1;
            }
        }
    }

    for (key, (id, managed)) in &existing {
        if !wanted.contains(key) && managed.missing_since.is_none() {
            db::targets::set_missing(&state.pool, *id, true).await?;
            super::targets::forget_alerts_of(state, *id).await;
            outcome.archived += 1;
        }
    }
    Ok(outcome)
}

/// Les étiquettes de l'utilisateur ne doivent pas écraser ce que le serveur pose.
fn sanitized_tags(tags: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    tags.iter()
        .filter(|(key, value)| !key.is_empty() && value.len() <= 512)
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// La cible diffère-t-elle de ce que les étiquettes demandent ?
fn changed(before: &dumbmonit_proto::Target, input: &db::targets::TargetInput) -> bool {
    before.name != input.name
        || before.address != input.address
        || before.kind != input.kind
        || before.interval != input.interval
        || before.tags != input.tags
        || before.group_name != input.group_name
        || before.parent_id != input.parent_id
        || before.enabled != input.enabled
}

/// Message de refus d'une modification à la main.
pub const MANAGED_MESSAGE: &str = "This monitor is managed by Docker labels: change the labels \
     on the container, or release it first to edit it here.";

/// Refuse la modification ou la suppression d'un moniteur géré (hors archivé,
/// qui peut être supprimé).
pub async fn refuse_if_managed(
    state: &AppState,
    id: TargetId,
    deleting: bool,
) -> ApiResult<Option<db::targets::Managed>> {
    let managed = db::targets::managed_get(&state.pool, id).await?;
    match &managed {
        Some(m) if deleting && m.missing_since.is_some() => {}
        Some(_) => return Err(ApiError::Conflict(MANAGED_MESSAGE.into())),
        None => {}
    }
    Ok(managed)
}

/// `POST /api/targets/{id}/release` : rend un moniteur géré à la gestion manuelle.
pub async fn release(
    State(state): State<AppState>,
    Path(id): Path<TargetId>,
) -> ApiResult<StatusCode> {
    if !db::targets::release_managed(&state.pool, id).await? {
        return Err(ApiError::NotFound(format!("Monitor {id} is not managed by labels.")));
    }
    // L'archivage désactivait le moniteur : le libérer ne doit pas le laisser éteint
    // sans qu'on le sache, mais ne le rallume pas non plus (choix de l'utilisateur).
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l_hote_remplace_son_marqueur() {
        assert_eq!(resolve_address("http://{host}:3000/up", "nas.lan"), "http://nas.lan:3000/up");
        assert_eq!(resolve_address("{host}:22", "fd00::1"), "[fd00::1]:22");
        assert_eq!(resolve_address("db.lan:5432", "nas.lan"), "db.lan:5432");
    }

    #[test]
    fn l_empreinte_change_avec_la_liste_ou_l_hote() {
        let a = DeclaredMonitor { key: "a/http".into(), ..Default::default() };
        let b = DeclaredMonitor { key: "b/http".into(), ..Default::default() };
        let one = fingerprint("h", std::slice::from_ref(&a));
        assert_eq!(one, fingerprint("h", std::slice::from_ref(&a)));
        assert_ne!(one, fingerprint("h", &[a.clone(), b]));
        assert_ne!(one, fingerprint("other", &[a]));
    }
}
