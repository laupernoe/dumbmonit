//! Plan : ce qu'un fichier changerait, sans rien écrire.
//!
//! Le plan compare deux documents de même forme : l'instance telle qu'elle est
//! (relue par le code d'export de la sauvegarde, secrets déchiffrés en mémoire)
//! et le fichier. Il en tire, par clé naturelle, ce qui serait créé, modifié
//! ou — avec `prune` — supprimé. Les valeurs ne sortent jamais d'ici : un plan
//! nomme les champs qui diffèrent, jamais leur contenu, ce qui suffit à relire
//! un changement et ne peut pas laisser fuir un secret.

use std::collections::{BTreeMap, HashMap, HashSet};

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::{Map, Value, json};
use sqlx::{Row, SqlitePool};

use super::{ConfigDoc, MANAGED_BY_FILE, refs, silence_key};
use crate::backup::bundle::{Bundle, target_ref};
use crate::backup::export;
use crate::crypto::Cipher;

/// Une ligne du plan.
#[derive(Debug, Clone, Serialize)]
pub struct Change {
    pub section: String,
    pub key: String,
    /// `create`, `update` ou `delete`.
    pub action: String,
    /// Champs qui diffèrent (jamais leurs valeurs). `managed_by_file` : l'objet
    /// existe déjà et passerait sous la gestion du fichier.
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Plan {
    pub changes: Vec<Change>,
    pub create: usize,
    pub update: usize,
    pub delete: usize,
    pub unchanged: usize,
    /// Ce que le plan ne change pas mais qu'il faut savoir : objets gérés par
    /// fichier qui ne sont plus déclarés et que `prune` supprimerait.
    pub warnings: Vec<String>,
}

impl Plan {
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
}

/// L'instance telle que le fichier la voit.
pub struct Current {
    /// Contenu actuel, secrets en clair (mémoire seulement).
    pub doc: ConfigDoc,
    /// Pour chaque équipement, `managed_by` (`None` : saisi à la main).
    pub target_managed_by: HashMap<String, Option<String>>,
    /// Objets déjà gérés par fichier : `(kind, key)` de `config_managed`.
    pub managed: HashSet<(String, String)>,
}

/// Les équipements que le fichier n'a pas à connaître : ceux que les étiquettes
/// Docker créent et ceux qu'un agent a découverts. Ils se recréent seuls.
async fn foreign_targets(
    pool: &SqlitePool,
) -> Result<(HashSet<String>, HashMap<String, Option<String>>)> {
    let discovered: HashSet<i64> =
        sqlx::query("SELECT target_id FROM agent_modules WHERE target_id IS NOT NULL")
            .fetch_all(pool)
            .await
            .context("lecture des services découverts")?
            .iter()
            .filter_map(|row| row.try_get("target_id").ok())
            .collect();
    let rows = sqlx::query("SELECT id, kind, address, managed_by FROM targets")
        .fetch_all(pool)
        .await
        .context("lecture des équipements")?;
    let mut foreign = HashSet::new();
    let mut managed = HashMap::new();
    for row in &rows {
        let id: i64 = row.try_get("id")?;
        let reference =
            target_ref(&row.try_get::<String, _>("kind")?, &row.try_get::<String, _>("address")?);
        let by: Option<String> = row.try_get("managed_by")?;
        if discovered.contains(&id) || by.as_deref().is_some_and(|by| by != MANAGED_BY_FILE) {
            foreign.insert(reference.clone());
        }
        managed.insert(reference, by);
    }
    Ok((foreign, managed))
}

/// Relit l'instance.
pub async fn load_current(pool: &SqlitePool, cipher: &Cipher) -> Result<Current> {
    let bundle = export::collect(pool, cipher, export::ExportOptions::default()).await?;
    let (foreign, target_managed_by) = foreign_targets(pool).await?;
    let managed: HashSet<(String, String)> = sqlx::query("SELECT kind, key FROM config_managed")
        .fetch_all(pool)
        .await
        .context("lecture des objets gérés par fichier")?
        .iter()
        .filter_map(|row| Some((row.try_get("kind").ok()?, row.try_get("key").ok()?)))
        .collect();
    Ok(Current { doc: doc_from_bundle(bundle, &foreign), target_managed_by, managed })
}

fn doc_from_bundle(bundle: Bundle, foreign: &HashSet<String>) -> ConfigDoc {
    let mut status_pages = bundle.status_pages;
    for page in &mut status_pages {
        // Le mot de passe d'accès se règle dans l'interface et n'est jamais dans un fichier.
        page.access_password_hash = None;
        if page.access_mode.as_deref() == Some("password") {
            page.access_mode = None;
        }
    }
    ConfigDoc {
        version: super::SCHEMA_VERSION,
        targets: bundle
            .targets
            .into_iter()
            .filter(|t| !foreign.contains(&target_ref(&t.kind, &t.address)))
            .collect(),
        rules: bundle.rules.into_iter().filter(|rule| !rule.builtin).collect(),
        rule_overrides: bundle.rule_overrides,
        channels: bundle.channels,
        silences: bundle.silences,
        status_pages,
    }
}

/// Ramène le fichier à la forme que la base conserve : un champ omis prend la
/// valeur actuelle quand « omis » veut dire « ne pas toucher » (rang manuel,
/// référencement, accès) ou la valeur par défaut de la base. Le plan et
/// l'application passent par là : ils ne peuvent pas diverger.
pub fn normalize(desired: &mut ConfigDoc, current: &ConfigDoc) {
    let targets: HashMap<String, &crate::backup::bundle::BundleTarget> =
        current.targets.iter().map(|t| (target_ref(&t.kind, &t.address), t)).collect();
    for target in &mut desired.targets {
        let key = target_ref(&target.kind, &target.address);
        let old = targets.get(&key);
        // Le rang manuel dans `/targets` n'est pas dans le fichier.
        target.position = old.map_or(0, |t| t.position);
        // Emplacement de la carte et sondes multi-sites : réglés dans l'interface.
        target.location = old.and_then(|t| t.location.clone());
        if target.multisite.is_none() {
            target.multisite = old.and_then(|t| t.multisite.clone());
        }
        // Une référence que l'appelant n'a pas pu lire (plan d'un fichier collé
        // dans l'interface) vaut « secret inchangé ».
        if let (Some(credential), Some(old)) = (&mut target.credential, old) {
            refs::assume_unchanged(credential, old.credential.as_ref());
        }
    }

    let rules: HashMap<&str, &crate::backup::bundle::BundleRule> =
        current.rules.iter().map(|r| (r.uid.as_str(), r)).collect();
    for rule in &mut desired.rules {
        if rule.selector.is_null() {
            rule.selector = json!({ "kind": "all" });
        }
        if rule.params.is_null() {
            rule.params = rules.get(rule.uid.as_str()).map_or(json!({}), |old| old.params.clone());
        }
    }

    let channels: HashMap<&str, &crate::backup::bundle::BundleChannel> =
        current.channels.iter().map(|c| (c.name.as_str(), c)).collect();
    for channel in &mut desired.channels {
        let old = channels.get(channel.name.as_str());
        if channel.settings.is_null() {
            channel.settings = old.map_or(json!({}), |c| c.settings.clone());
        }
        if channel.policy.is_null() {
            channel.policy = old.map_or(json!({}), |c| c.policy.clone());
        }
        if let (Some(secrets), Some(old)) = (&mut channel.secrets, old) {
            refs::assume_unchanged(secrets, old.secrets.as_ref());
        }
    }

    for silence in &mut desired.silences {
        if silence.matchers.is_null() {
            silence.matchers = json!({});
        }
    }

    let pages: HashMap<&str, &crate::backup::bundle::BundleStatusPage> =
        current.status_pages.iter().map(|p| (p.slug.as_str(), p)).collect();
    for page in &mut desired.status_pages {
        page.access_password_hash = None;
        if let Some(old) = pages.get(page.slug.as_str())
            && page.seo.is_none()
        {
            page.seo = old.seo.clone();
        }
    }
}

fn as_map<T: Serialize>(value: &T) -> Map<String, Value> {
    match serde_json::to_value(value) {
        Ok(Value::Object(map)) => map,
        _ => Map::new(),
    }
}

/// Champs qui diffèrent entre deux objets (`null` vaut « absent »).
fn differing(current: &Map<String, Value>, wanted: &Map<String, Value>) -> Vec<String> {
    let mut keys: Vec<&String> = current.keys().chain(wanted.keys()).collect();
    keys.sort();
    keys.dedup();
    keys.into_iter()
        .filter(|key| {
            let a = current.get(*key).filter(|v| !v.is_null());
            let b = wanted.get(*key).filter(|v| !v.is_null());
            a != b
        })
        .cloned()
        .collect()
}

struct SectionDiff<'a> {
    name: &'static str,
    prune: bool,
    managed: &'a dyn Fn(&str) -> bool,
}

impl SectionDiff<'_> {
    fn run<T: Serialize>(
        &self,
        current: &[T],
        wanted: &[T],
        key: &dyn Fn(&T) -> String,
        plan: &mut Plan,
    ) {
        let current: BTreeMap<String, Map<String, Value>> =
            current.iter().map(|item| (key(item), as_map(item))).collect();
        let wanted: BTreeMap<String, Map<String, Value>> =
            wanted.iter().map(|item| (key(item), as_map(item))).collect();

        for (name, want) in &wanted {
            let (action, fields) = match current.get(name) {
                None => ("create", Vec::new()),
                Some(have) => {
                    let mut fields = differing(have, want);
                    if fields.is_empty() && (self.managed)(name) {
                        plan.unchanged += 1;
                        continue;
                    }
                    if !(self.managed)(name) {
                        fields.push("managed_by_file".to_string());
                    }
                    ("update", fields)
                }
            };
            plan.changes.push(Change {
                section: self.name.to_string(),
                key: name.clone(),
                action: action.to_string(),
                fields,
            });
            match action {
                "create" => plan.create += 1,
                _ => plan.update += 1,
            }
        }

        let mut forgotten = 0;
        for name in current.keys().filter(|name| !wanted.contains_key(*name)) {
            if !(self.managed)(name) {
                continue;
            }
            if self.prune {
                plan.changes.push(Change {
                    section: self.name.to_string(),
                    key: name.clone(),
                    action: "delete".to_string(),
                    fields: Vec::new(),
                });
                plan.delete += 1;
            } else {
                forgotten += 1;
            }
        }
        if forgotten > 0 {
            plan.warnings.push(format!(
                "{forgotten} {} managed by a file {} no longer declared; they are kept (use \
                 prune to delete them).",
                self.name,
                if forgotten == 1 { "is" } else { "are" }
            ));
        }
    }
}

/// Calcule le plan. `desired` doit déjà être normalisé ([`normalize`]).
pub fn diff(current: &Current, desired: &ConfigDoc, prune: bool) -> Plan {
    let mut plan = Plan::default();
    let in_table = |kind: &'static str| {
        move |key: &str| current.managed.contains(&(kind.to_string(), key.to_string()))
    };
    let target_managed = |key: &str| {
        current.target_managed_by.get(key).is_some_and(|by| by.as_deref() == Some(MANAGED_BY_FILE))
    };

    SectionDiff { name: "targets", prune, managed: &target_managed }.run(
        &current.doc.targets,
        &desired.targets,
        &|t| target_ref(&t.kind, &t.address),
        &mut plan,
    );
    SectionDiff { name: "channels", prune, managed: &in_table("channel") }.run(
        &current.doc.channels,
        &desired.channels,
        &|c| c.name.clone(),
        &mut plan,
    );
    SectionDiff { name: "rules", prune, managed: &in_table("rule") }.run(
        &current.doc.rules,
        &desired.rules,
        &|r| r.uid.clone(),
        &mut plan,
    );
    SectionDiff { name: "rule_overrides", prune, managed: &in_table("override") }.run(
        &current.doc.rule_overrides,
        &desired.rule_overrides,
        &|o| format!("{}|{}", o.rule_uid, o.target),
        &mut plan,
    );
    SectionDiff { name: "silences", prune, managed: &in_table("silence") }.run(
        &current.doc.silences,
        &desired.silences,
        &|s| silence_key(&s.name, s.target.as_deref()),
        &mut plan,
    );
    SectionDiff { name: "status_pages", prune, managed: &in_table("status_page") }.run(
        &current.doc.status_pages,
        &desired.status_pages,
        &|p| p.slug.clone(),
        &mut plan,
    );
    plan
}

/// Refuse ce qu'un fichier ne peut pas déclarer à cause de l'état de
/// l'instance : un équipement que les étiquettes Docker (ou un agent) gèrent déjà.
pub fn check_ownership(current: &Current, desired: &ConfigDoc) -> Result<(), String> {
    let mut problems = Vec::new();
    for target in &desired.targets {
        let key = target_ref(&target.kind, &target.address);
        if let Some(Some(by)) = current.target_managed_by.get(&key)
            && by != MANAGED_BY_FILE
        {
            problems.push(format!(
                "targets: {key} is already managed by {by}; remove it from the file."
            ));
        }
    }
    if problems.is_empty() { Ok(()) } else { Err(problems.join("\n")) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backup::bundle::BundleChannel;

    fn channel(name: &str, enabled: bool) -> BundleChannel {
        BundleChannel {
            name: name.into(),
            kind: "ntfy".into(),
            enabled,
            settings: json!({"topic": "x"}),
            secrets: Some(json!({"token": "abc"})),
            policy: json!({}),
        }
    }

    fn current_with(channels: Vec<BundleChannel>, managed: &[&str]) -> Current {
        Current {
            doc: ConfigDoc { version: 1, channels, ..ConfigDoc::default() },
            target_managed_by: HashMap::new(),
            managed: managed.iter().map(|k| ("channel".to_string(), (*k).to_string())).collect(),
        }
    }

    #[test]
    fn plan_names_fields_but_never_values() {
        let current = current_with(vec![channel("a", true), channel("old", true)], &["a", "old"]);
        let mut changed = channel("a", false);
        changed.secrets = Some(json!({"token": "different"}));
        let desired = ConfigDoc {
            version: 1,
            channels: vec![changed, channel("new", true)],
            ..ConfigDoc::default()
        };

        let plan = diff(&current, &desired, true);
        assert_eq!((plan.create, plan.update, plan.delete), (1, 1, 1));
        let update = plan.changes.iter().find(|c| c.action == "update").unwrap();
        assert_eq!(update.fields, vec!["enabled".to_string(), "secrets".to_string()]);
        let text = serde_json::to_string(&plan).unwrap();
        assert!(!text.contains("different") && !text.contains("abc"));
    }

    #[test]
    fn without_prune_nothing_is_deleted_and_unmanaged_is_never_pruned() {
        let current = current_with(vec![channel("mine", true), channel("hand", true)], &["mine"]);
        let desired = ConfigDoc { version: 1, ..ConfigDoc::default() };
        let plan = diff(&current, &desired, false);
        assert_eq!(plan.delete, 0);
        assert_eq!(plan.warnings.len(), 1);
        let plan = diff(&current, &desired, true);
        assert_eq!(plan.delete, 1);
        assert_eq!(plan.changes[0].key, "mine");
    }

    #[test]
    fn adopting_an_existing_object_is_an_update() {
        let current = current_with(vec![channel("a", true)], &[]);
        let desired =
            ConfigDoc { version: 1, channels: vec![channel("a", true)], ..ConfigDoc::default() };
        let plan = diff(&current, &desired, false);
        assert_eq!(plan.update, 1);
        assert_eq!(plan.changes[0].fields, vec!["managed_by_file".to_string()]);
    }

    #[test]
    fn a_converged_file_has_an_empty_plan() {
        let current = current_with(vec![channel("a", true)], &["a"]);
        let desired =
            ConfigDoc { version: 1, channels: vec![channel("a", true)], ..ConfigDoc::default() };
        let plan = diff(&current, &desired, true);
        assert!(plan.is_empty());
        assert_eq!(plan.unchanged, 1);
    }
}
