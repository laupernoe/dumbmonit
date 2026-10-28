//! Restauration d'un lot dans une base.
//!
//! Trois principes, et ils tiennent tous les trois dans le compte rendu que
//! l'interface montre avant d'appliquer quoi que ce soit :
//!
//! * **rien n'est jamais supprimé**. Restaurer, c'est faire en sorte que ce que
//!   le lot décrit existe ; ce que l'instance a en plus reste ;
//! * **rien n'est jamais dupliqué**. Chaque chose a une clé naturelle — type +
//!   adresse pour un équipement, `uid` pour une règle, nom pour un canal — et
//!   c'est sur elle que l'on retrouve ce qui est déjà là. Restaurer deux fois
//!   le même lot ne crée rien la seconde fois ;
//! * **la simulation est le geste réel, annulé**. Le même code tourne dans les
//!   deux cas, dans une transaction que l'on valide ou que l'on annule. Le
//!   compte rendu d'une simulation ne peut donc pas mentir sur ce que
//!   l'application ferait.
//!
//! Les comptes font exception à « mettre à jour » : un compte qui existe déjà
//! est laissé tel quel. Écraser le rôle ou le mot de passe de l'administrateur
//! connecté au moyen d'un fichier serait le plus court chemin vers une instance
//! dont plus personne n'a la clé.

use std::collections::HashMap;

use anyhow::{Context, Result};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::Serialize;
use serde_json::Value;
use sqlx::{Row, Sqlite, SqlitePool, Transaction};

use crate::api::topology_edits::{self, CorrectionKind, RuleKind, RuleSpec, RuleStatus};
use crate::backup::bundle::*;
use crate::crypto::Cipher;

/// Sort d'une ligne du lot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreOutcome {
    Created,
    Updated,
    Skipped,
}

/// Ce qu'une section du lot a donné.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SectionReport {
    pub section: String,
    pub created: usize,
    pub updated: usize,
    pub skipped: usize,
    /// Ce qui mérite d'être lu ligne par ligne : une référence introuvable, une
    /// ligne refusée par la base.
    pub notes: Vec<String>,
}

impl SectionReport {
    pub(crate) fn new(section: &str) -> Self {
        Self { section: section.to_string(), ..Self::default() }
    }

    pub(crate) fn count(&mut self, outcome: RestoreOutcome) {
        match outcome {
            RestoreOutcome::Created => self.created += 1,
            RestoreOutcome::Updated => self.updated += 1,
            RestoreOutcome::Skipped => self.skipped += 1,
        }
    }

    pub(crate) fn refused(&mut self, what: &str, error: &anyhow::Error) {
        self.skipped += 1;
        self.notes.push(format!("{what} was refused by the database: {error}"));
    }
}

/// Compte rendu complet d'une restauration, simulée ou appliquée.
#[derive(Debug, Clone, Default, Serialize)]
pub struct RestoreReport {
    /// Faux : rien n'a été écrit, la transaction a été annulée.
    pub applied: bool,
    /// Date d'écriture du lot et version du serveur qui l'a produit.
    pub created_at: String,
    pub source_version: String,
    pub sections: Vec<SectionReport>,
    pub created: usize,
    pub updated: usize,
    pub skipped: usize,
    /// Ce que l'opérateur doit savoir avant, ou juste après, d'appliquer.
    pub warnings: Vec<String>,
}

impl RestoreReport {
    fn push(&mut self, section: SectionReport) {
        self.created += section.created;
        self.updated += section.updated;
        self.skipped += section.skipped;
        self.sections.push(section);
    }
}

/// Restaure un lot. `apply` faux : tout est calculé puis annulé.
pub async fn restore(
    pool: &SqlitePool,
    cipher: &Cipher,
    bundle: &Bundle,
    apply: bool,
) -> Result<RestoreReport> {
    let mut tx = pool.begin().await.context("ouverture de la transaction de restauration")?;

    let mut report = RestoreReport {
        applied: apply,
        created_at: bundle.created_at.clone(),
        source_version: bundle.source_version.clone(),
        ..RestoreReport::default()
    };

    report.push(restore_users(&mut tx, cipher, bundle).await?);
    // Les clients avant le reste : leurs membres sont des comptes, et les
    // rattachements, appliqués à la fin, les désignent par leur nom.
    let mut clients_report = restore_clients(&mut tx, bundle).await?;
    // Les paquets avant les équipements : ils en définissent les types.
    report.push(restore_packs(&mut tx, bundle).await?);
    report.push(restore_targets(&mut tx, cipher, bundle).await?);

    let refs = target_ids(&mut tx).await?;
    report.push(restore_locations(&mut tx, bundle, &refs).await?);
    report.push(restore_teams(&mut tx, bundle).await?);
    report.push(restore_topology_corrections(&mut tx, bundle, &refs).await?);
    report.push(restore_topology_rules(&mut tx, bundle, &refs).await?);
    report.push(restore_lun_links(&mut tx, bundle, &refs).await?);
    report.push(restore_dashboards(&mut tx, bundle).await?);

    report.push(restore_channels(&mut tx, cipher, bundle).await?);
    let channels = channel_ids(&mut tx).await?;

    report.push(restore_rules(&mut tx, bundle, &refs, &channels).await?);
    report.push(restore_overrides(&mut tx, bundle, &refs).await?);
    report.push(restore_notify_policy(&mut tx, bundle).await?);
    report.push(restore_instance(&mut tx, bundle).await?);
    report.push(restore_silences(&mut tx, bundle, &refs).await?);
    report.push(restore_status_pages(&mut tx, bundle, &refs).await?);
    report.push(restore_report_schedules(&mut tx, bundle, &channels).await?);
    report.push(restore_incidents(&mut tx, bundle).await?);
    report.push(restore_agent_tokens(&mut tx, bundle).await?);
    report.push(restore_api_tokens(&mut tx, bundle).await?);
    report.push(restore_push_monitors(&mut tx, cipher, bundle, &refs).await?);
    report.push(crate::backup::oncall::restore(&mut tx, cipher, &bundle.oncall).await?);
    report.push(restore_runbooks(&mut tx, bundle).await?);
    report.push(restore_remediation_actions(&mut tx, cipher, bundle, &refs).await?);
    report.push(restore_remediation_policies(&mut tx, bundle, &refs).await?);
    report.push(restore_netconfig_host_keys(&mut tx, bundle, &refs).await?);
    report.push(restore_netconfig_versions(&mut tx, cipher, bundle, &refs).await?);
    restore_client_assignments(&mut tx, bundle, &mut clients_report).await?;
    report.push(clients_report);

    warnings(bundle, &mut report);

    if apply {
        tx.commit().await.context("validation de la restauration")?;
        // Les domaines publics des pages ont pu changer.
        crate::status_host::invalidate();
        // Une sauvegarde ancienne peut ramener des en-têtes de webhook en clair.
        if let Err(error) = crate::db::alerts::lift_legacy_webhook_headers(pool, cipher).await {
            tracing::warn!(?error, "webhook headers of the backup left in settings");
        }
    } else {
        tx.rollback().await.context("annulation de la simulation")?;
    }
    Ok(report)
}

/// Applique, dans la transaction de l'appelant, les sections qu'un fichier de
/// configuration déclare (voir [`crate::gitops`]) : équipements, canaux, règles,
/// surcharges, fenêtres de maintenance et pages de statut. La validation et
/// l'annulation restent à l'appelant, qui doit aussi traiter les notes des
/// sections comme des refus.
pub(crate) async fn restore_config(
    tx: &mut Transaction<'_, Sqlite>,
    cipher: &Cipher,
    bundle: &Bundle,
) -> Result<Vec<SectionReport>> {
    let mut sections = vec![restore_targets(tx, cipher, bundle).await?];
    let refs = target_ids(tx).await?;
    sections.push(restore_channels(tx, cipher, bundle).await?);
    let channels = channel_ids(tx).await?;
    sections.push(restore_rules(tx, bundle, &refs, &channels).await?);
    sections.push(restore_overrides(tx, bundle, &refs).await?);
    sections.push(restore_silences(tx, bundle, &refs).await?);
    sections.push(restore_status_pages(tx, bundle, &refs).await?);
    Ok(sections)
}

fn warnings(bundle: &Bundle, report: &mut RestoreReport) {
    if !bundle.remediation_policies.is_empty() {
        report.warnings.push(
            "Automatic remediation policies are restored disarmed: an administrator must arm \
             each one again, in two steps."
                .to_string(),
        );
    }
    if bundle
        .users
        .iter()
        .any(|u| u.password_hash.is_none() && u.oidc_subject.is_none() && u.saml_subject.is_none())
    {
        report.warnings.push(
            "This backup holds no account passwords. Restored accounts cannot sign in until an \
             administrator sets a password for them, or until they sign in through SSO."
                .to_string(),
        );
    }
    if !bundle.targets.is_empty() {
        report.warnings.push(
            "Device credentials from the backup are now encrypted with this server's own \
             instance secret (/data/secret.key). Back that file up: without it they cannot be \
             read again."
                .to_string(),
        );
    }
    if report.sections.iter().any(|s| !s.notes.is_empty()) {
        report.warnings.push(
            "Some entries could not be restored as they were; read the notes of each section."
                .to_string(),
        );
    }
}

// --------------------------------------------------------------------------
// Équipements
// --------------------------------------------------------------------------

async fn target_ids(tx: &mut Transaction<'_, Sqlite>) -> Result<HashMap<String, i64>> {
    let rows = sqlx::query("SELECT id, kind, address FROM targets")
        .fetch_all(&mut **tx)
        .await
        .context("relecture des équipements")?;
    rows.iter()
        .map(|row| {
            let kind: String = row.try_get("kind")?;
            let address: String = row.try_get("address")?;
            Ok((target_ref(&kind, &address), row.try_get("id")?))
        })
        .collect()
}

async fn channel_ids(tx: &mut Transaction<'_, Sqlite>) -> Result<HashMap<String, i64>> {
    let rows = sqlx::query("SELECT id, name FROM notification_channels")
        .fetch_all(&mut **tx)
        .await
        .context("relecture des canaux")?;
    rows.iter().map(|row| Ok((row.try_get("name")?, row.try_get("id")?))).collect()
}

/// Normalise un identifiant du lot et vérifie qu'il a bien la forme attendue.
///
/// Stocker un JSON quelconque dans `credential_enc` ferait échouer toute
/// lecture de l'équipement plus tard, à un endroit qui n'expliquerait rien.
fn credential_bytes(value: &Value) -> Result<Vec<u8>> {
    let credential: dumbmonit_proto::Credential =
        serde_json::from_value(value.clone()).context("credential not understood")?;
    Ok(serde_json::to_vec(&credential)?)
}

async fn restore_targets(
    tx: &mut Transaction<'_, Sqlite>,
    cipher: &Cipher,
    bundle: &Bundle,
) -> Result<SectionReport> {
    let mut report = SectionReport::new("targets");
    // Le sort de chaque équipement, tenu de côté : le second passage peut
    // encore faire passer un équipement « inchangé » à « modifié » s'il lui
    // rattache un parent différent.
    let mut outcomes: HashMap<String, RestoreOutcome> = HashMap::new();

    // Premier passage : les équipements eux-mêmes, sans leurs liens. Un parent
    // peut apparaître après son enfant dans le lot.
    for target in &bundle.targets {
        match upsert_target(tx, cipher, target).await {
            Ok(outcome) => {
                outcomes.insert(target_ref(&target.kind, &target.address), outcome);
            }
            Err(error) => report.refused(&format!("Monitor \"{}\"", target.name), &error),
        }
    }

    // Second passage : parents et agents relais, maintenant que tout existe.
    let refs = target_ids(tx).await?;
    for target in &bundle.targets {
        let key = target_ref(&target.kind, &target.address);
        let Some(id) = refs.get(&key) else { continue };
        let parent = resolve(&mut report, &refs, target.parent.as_deref(), &target.name, "parent");
        let relay =
            resolve(&mut report, &refs, target.via_agent.as_deref(), &target.name, "relay agent");
        // Une cible ne peut pas être son propre parent : la chaîne de
        // suppression des alertes tournerait en rond.
        let parent = parent.filter(|p| p != id);
        let relay = relay.filter(|p| p != id);

        if let Some(multisite) = &target.multisite
            && restore_multisite(tx, &mut report, &refs, *id, &target.name, multisite).await?
            && outcomes.get(&key) == Some(&RestoreOutcome::Skipped)
        {
            outcomes.insert(key.clone(), RestoreOutcome::Updated);
        }

        let current = sqlx::query("SELECT parent_id, via_agent FROM targets WHERE id = ?")
            .bind(id)
            .fetch_one(&mut **tx)
            .await
            .context("lecture des rattachements")?;
        if current.try_get::<Option<i64>, _>("parent_id")? == parent
            && current.try_get::<Option<i64>, _>("via_agent")? == relay
        {
            continue;
        }

        sqlx::query("UPDATE targets SET parent_id = ?, via_agent = ? WHERE id = ?")
            .bind(parent)
            .bind(relay)
            .bind(id)
            .execute(&mut **tx)
            .await
            .context("rattachement des équipements")?;
        if outcomes.get(&key) == Some(&RestoreOutcome::Skipped) {
            outcomes.insert(key, RestoreOutcome::Updated);
        }
    }

    for outcome in outcomes.into_values() {
        report.count(outcome);
    }
    Ok(report)
}

/// Rétablit le réglage multi-emplacements d'un moniteur. Un agent absent est
/// laissé de côté (avec une note), un quorum devenu trop grand revient à la
/// majorité. Rend `true` si quelque chose a changé.
async fn restore_multisite(
    tx: &mut Transaction<'_, Sqlite>,
    report: &mut SectionReport,
    refs: &HashMap<String, i64>,
    id: i64,
    owner: &str,
    multisite: &BundleMultisite,
) -> Result<bool> {
    let mut agents: Vec<i64> = multisite
        .agents
        .iter()
        .filter_map(|a| resolve(report, refs, Some(a), owner, "probe location"))
        .filter(|a| *a != id)
        .collect();
    agents.sort_unstable();
    agents.dedup();
    let locations = agents.len() + usize::from(multisite.server);
    let quorum = multisite.quorum.filter(|q| *q as usize <= locations).map(i64::from);

    let current =
        sqlx::query("SELECT include_server, quorum FROM target_multisite WHERE target_id = ?")
            .bind(id)
            .fetch_optional(&mut **tx)
            .await
            .context("lecture de la sonde multi-emplacements")?;
    let current_agents = sqlx::query_scalar::<_, i64>(
        "SELECT agent_id FROM target_multisite_agents WHERE target_id = ? ORDER BY agent_id",
    )
    .bind(id)
    .fetch_all(&mut **tx)
    .await
    .context("lecture des agents de la sonde multi-emplacements")?;
    if let Some(row) = &current
        && (row.try_get::<i64, _>("include_server")? != 0) == multisite.server
        && row.try_get::<Option<i64>, _>("quorum")? == quorum
        && current_agents == agents
    {
        return Ok(false);
    }

    sqlx::query("DELETE FROM target_multisite WHERE target_id = ?")
        .bind(id)
        .execute(&mut **tx)
        .await
        .context("retrait de la sonde multi-emplacements")?;
    sqlx::query(
        "INSERT INTO target_multisite (target_id, include_server, quorum) VALUES (?, ?, ?)",
    )
    .bind(id)
    .bind(i64::from(multisite.server))
    .bind(quorum)
    .execute(&mut **tx)
    .await
    .context("restauration de la sonde multi-emplacements")?;
    for agent in agents {
        sqlx::query("INSERT INTO target_multisite_agents (target_id, agent_id) VALUES (?, ?)")
            .bind(id)
            .bind(agent)
            .execute(&mut **tx)
            .await
            .context("restauration d'un agent de la sonde multi-emplacements")?;
    }
    Ok(true)
}

fn resolve(
    report: &mut SectionReport,
    refs: &HashMap<String, i64>,
    reference: Option<&str>,
    owner: &str,
    role: &str,
) -> Option<i64> {
    let reference = reference?;
    match refs.get(reference) {
        Some(id) => Some(*id),
        None => {
            report.notes.push(format!(
                "Monitor \"{owner}\": its {role} ({reference}) is not in this backup and does not \
                 exist here; the link was left empty."
            ));
            None
        }
    }
}

async fn upsert_target(
    tx: &mut Transaction<'_, Sqlite>,
    cipher: &Cipher,
    target: &BundleTarget,
) -> Result<RestoreOutcome> {
    let credential = match &target.credential {
        None => None,
        Some(value) => Some(credential_bytes(value)?),
    };
    let tags = serde_json::to_string(&target.tags)?;

    let existing = sqlx::query(
        "SELECT id, name, profile_id, interval_secs, enabled, tags, credential_enc, group_name,
                position
         FROM targets WHERE kind = ? AND address = ?",
    )
    .bind(&target.kind)
    .bind(&target.address)
    .fetch_optional(&mut **tx)
    .await
    .context("recherche de l'équipement")?;

    let Some(row) = existing else {
        sqlx::query(
            "INSERT INTO targets (name, address, kind, profile_id, interval_secs, enabled, tags,
                                  credential_enc, group_name, position)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&target.name)
        .bind(&target.address)
        .bind(&target.kind)
        .bind(&target.profile_id)
        .bind(target.interval_secs.max(10))
        .bind(i64::from(target.enabled))
        .bind(&tags)
        .bind(credential.map(|plain| cipher.encrypt(&plain)).transpose()?)
        .bind(&target.group_name)
        .bind(target.position)
        .execute(&mut **tx)
        .await
        .context("création de l'équipement")?;
        return Ok(RestoreOutcome::Created);
    };

    let id: i64 = row.try_get("id")?;
    let stored_tags: String = row.try_get("tags")?;
    let stored_credential: Option<Vec<u8>> = row.try_get("credential_enc")?;
    let same = row.try_get::<String, _>("name")? == target.name
        && row.try_get::<Option<String>, _>("profile_id")? == target.profile_id
        && row.try_get::<i64, _>("interval_secs")? == target.interval_secs
        && (row.try_get::<i64, _>("enabled")? != 0) == target.enabled
        && serde_json::from_str::<Value>(&stored_tags).ok()
            == serde_json::from_str::<Value>(&tags).ok()
        && same_secret(cipher, stored_credential.as_deref(), target.credential.as_ref())
        && row.try_get::<String, _>("group_name")? == target.group_name
        && row.try_get::<i64, _>("position")? == target.position;

    if same {
        return Ok(RestoreOutcome::Skipped);
    }

    sqlx::query(
        "UPDATE targets SET name = ?, profile_id = ?, interval_secs = ?, enabled = ?, tags = ?,
             credential_enc = ?, group_name = ?, position = ?,
             updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
         WHERE id = ?",
    )
    .bind(&target.name)
    .bind(&target.profile_id)
    .bind(target.interval_secs.max(10))
    .bind(i64::from(target.enabled))
    .bind(&tags)
    .bind(credential.map(|plain| cipher.encrypt(&plain)).transpose()?)
    .bind(&target.group_name)
    .bind(target.position)
    .bind(id)
    .execute(&mut **tx)
    .await
    .context("mise à jour de l'équipement")?;
    Ok(RestoreOutcome::Updated)
}

/// Compare un secret chiffré en base au clair du lot, sans jamais les afficher.
fn same_secret(cipher: &Cipher, stored: Option<&[u8]>, wanted: Option<&Value>) -> bool {
    let decrypted = stored
        .and_then(|bytes| cipher.decrypt(bytes).ok())
        .and_then(|plain| serde_json::from_slice::<Value>(&plain).ok());
    match (decrypted, wanted) {
        (None, None) => true,
        (Some(left), Some(right)) => &left == right,
        _ => false,
    }
}

// --------------------------------------------------------------------------
// Canaux
// --------------------------------------------------------------------------

async fn restore_channels(
    tx: &mut Transaction<'_, Sqlite>,
    cipher: &Cipher,
    bundle: &Bundle,
) -> Result<SectionReport> {
    let mut report = SectionReport::new("channels");
    for channel in &bundle.channels {
        match upsert_channel(tx, cipher, channel).await {
            Ok(outcome) => report.count(outcome),
            Err(error) => report.refused(&format!("Channel \"{}\"", channel.name), &error),
        }
    }
    Ok(report)
}

async fn upsert_channel(
    tx: &mut Transaction<'_, Sqlite>,
    cipher: &Cipher,
    channel: &BundleChannel,
) -> Result<RestoreOutcome> {
    let settings = serde_json::to_string(&channel.settings)?;
    let policy = serde_json::to_string(&channel.policy)?;
    let secrets = match &channel.secrets {
        None => None,
        Some(value) => Some(cipher.encrypt(serde_json::to_vec(value)?.as_slice())?),
    };

    let existing = sqlx::query(
        "SELECT id, kind, enabled, settings, secret_enc, policy
                     FROM notification_channels WHERE name = ?",
    )
    .bind(&channel.name)
    .fetch_optional(&mut **tx)
    .await
    .context("recherche du canal")?;

    let Some(row) = existing else {
        sqlx::query(
            "INSERT INTO notification_channels (name, kind, enabled, settings, secret_enc, policy)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&channel.name)
        .bind(&channel.kind)
        .bind(i64::from(channel.enabled))
        .bind(&settings)
        .bind(secrets)
        .bind(&policy)
        .execute(&mut **tx)
        .await
        .context("création du canal")?;
        return Ok(RestoreOutcome::Created);
    };

    let id: i64 = row.try_get("id")?;
    let stored_settings: String = row.try_get("settings")?;
    let stored_policy: String = row.try_get("policy")?;
    let stored_secret: Option<Vec<u8>> = row.try_get("secret_enc")?;
    let same = row.try_get::<String, _>("kind")? == channel.kind
        && (row.try_get::<i64, _>("enabled")? != 0) == channel.enabled
        && serde_json::from_str::<Value>(&stored_settings).ok() == Some(channel.settings.clone())
        && serde_json::from_str::<Value>(&stored_policy).ok() == Some(channel.policy.clone())
        && same_secret(cipher, stored_secret.as_deref(), channel.secrets.as_ref());

    if same {
        return Ok(RestoreOutcome::Skipped);
    }

    sqlx::query(
        "UPDATE notification_channels
         SET kind = ?, enabled = ?, settings = ?, secret_enc = COALESCE(?, secret_enc),
             policy = ?, updated_at = datetime('now')
         WHERE id = ?",
    )
    .bind(&channel.kind)
    .bind(i64::from(channel.enabled))
    .bind(&settings)
    .bind(secrets)
    .bind(&policy)
    .bind(id)
    .execute(&mut **tx)
    .await
    .context("mise à jour du canal")?;
    Ok(RestoreOutcome::Updated)
}

// --------------------------------------------------------------------------
// Règles et surcharges
// --------------------------------------------------------------------------

/// Rend au sélecteur ses identifiants locaux.
fn selector_from_bundle(
    report: &mut SectionReport,
    value: &Value,
    refs: &HashMap<String, i64>,
    rule: &str,
) -> String {
    if value.get("kind").and_then(Value::as_str) == Some("refs") {
        let listed = value.get("refs").and_then(Value::as_array).cloned().unwrap_or_default();
        let mut ids = Vec::new();
        for reference in listed.iter().filter_map(Value::as_str) {
            match refs.get(reference) {
                Some(id) => ids.push(*id),
                None => report.notes.push(format!(
                    "Rule \"{rule}\": it targeted {reference}, which is not in this backup; that \
                     device was dropped from its scope."
                )),
            }
        }
        // Un sélecteur devenu vide viserait plus rien : la règle serait muette
        // sans le dire. On la remet sur tous les équipements, ce que le compte
        // rendu signale.
        if ids.is_empty() {
            report.notes.push(format!(
                "Rule \"{rule}\": none of the devices it targeted exist here; it was restored as \
                 applying to every device. Narrow it again if that is not what you want."
            ));
            return r#"{"kind":"all"}"#.to_string();
        }
        return serde_json::json!({ "kind": "ids", "ids": ids }).to_string();
    }
    value.to_string()
}

async fn restore_rules(
    tx: &mut Transaction<'_, Sqlite>,
    bundle: &Bundle,
    refs: &HashMap<String, i64>,
    channels: &HashMap<String, i64>,
) -> Result<SectionReport> {
    let mut report = SectionReport::new("rules");
    for rule in &bundle.rules {
        let selector = selector_from_bundle(&mut report, &rule.selector, refs, &rule.name);
        let mut ids = Vec::new();
        for name in &rule.channels {
            match channels.get(name) {
                Some(id) => ids.push(*id),
                None => report.notes.push(format!(
                    "Rule \"{}\": the channel \"{name}\" it sends to is not in this backup; it \
                     was dropped from the rule.",
                    rule.name
                )),
            }
        }
        match upsert_rule(tx, rule, &selector, &ids).await {
            Ok(outcome) => report.count(outcome),
            Err(error) => report.refused(&format!("Rule \"{}\"", rule.name), &error),
        }
    }
    Ok(report)
}

async fn upsert_rule(
    tx: &mut Transaction<'_, Sqlite>,
    rule: &BundleRule,
    selector: &str,
    channels: &[i64],
) -> Result<RestoreOutcome> {
    let channel_json = serde_json::to_string(channels)?;
    let params = serde_json::to_string(&rule.params)?;

    let existing = sqlx::query(
        "SELECT id, name, description, kind, query, operator, threshold, clear_threshold,
                for_secs, severity, selector, channels, params, unit, repeat_secs,
                escalate_after_secs, enabled
         FROM alert_rules WHERE uid = ?",
    )
    .bind(&rule.uid)
    .fetch_optional(&mut **tx)
    .await
    .context("recherche de la règle")?;

    if let Some(row) = &existing {
        let same = row.try_get::<String, _>("name")? == rule.name
            && row.try_get::<String, _>("description")? == rule.description
            && row.try_get::<String, _>("kind")? == rule.kind
            && row.try_get::<String, _>("query")? == rule.query
            && row.try_get::<String, _>("operator")? == rule.operator
            && row.try_get::<f64, _>("threshold")? == rule.threshold
            && row.try_get::<Option<f64>, _>("clear_threshold")? == rule.clear_threshold
            && row.try_get::<i64, _>("for_secs")? == rule.for_secs
            && row.try_get::<String, _>("severity")? == rule.severity
            && row.try_get::<String, _>("selector")? == selector
            && row.try_get::<String, _>("channels")? == channel_json
            // Les réglages (anomalie, prévision, dérive) se comparent en JSON
            // structuré : l'ordre des clés diffère entre la base et le lot.
            && serde_json::from_str::<Value>(&row.try_get::<String, _>("params")?).ok().as_ref()
                == Some(&rule.params)
            && row.try_get::<String, _>("unit")? == rule.unit
            && row.try_get::<Option<i64>, _>("repeat_secs")? == rule.repeat_secs
            && row.try_get::<Option<i64>, _>("escalate_after_secs")? == rule.escalate_after_secs
            && (row.try_get::<i64, _>("enabled")? != 0) == rule.enabled;
        if same {
            return Ok(RestoreOutcome::Skipped);
        }
    }

    sqlx::query(
        "INSERT INTO alert_rules
             (uid, name, description, kind, query, operator, threshold, clear_threshold,
              for_secs, severity, selector, channels, params, unit, repeat_secs,
              escalate_after_secs, enabled, builtin, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
         ON CONFLICT(uid) DO UPDATE SET
             name = excluded.name, description = excluded.description, kind = excluded.kind,
             query = excluded.query, operator = excluded.operator,
             threshold = excluded.threshold, clear_threshold = excluded.clear_threshold,
             for_secs = excluded.for_secs, severity = excluded.severity,
             selector = excluded.selector, channels = excluded.channels,
             params = excluded.params, unit = excluded.unit,
             repeat_secs = excluded.repeat_secs,
             escalate_after_secs = excluded.escalate_after_secs,
             enabled = excluded.enabled, updated_at = datetime('now')",
    )
    .bind(&rule.uid)
    .bind(&rule.name)
    .bind(&rule.description)
    .bind(&rule.kind)
    .bind(&rule.query)
    .bind(&rule.operator)
    .bind(rule.threshold)
    .bind(rule.clear_threshold)
    .bind(rule.for_secs)
    .bind(&rule.severity)
    .bind(selector)
    .bind(&channel_json)
    .bind(&params)
    .bind(&rule.unit)
    .bind(rule.repeat_secs)
    .bind(rule.escalate_after_secs)
    .bind(i64::from(rule.enabled))
    .bind(i64::from(rule.builtin))
    .execute(&mut **tx)
    .await
    .context("enregistrement de la règle")?;

    Ok(if existing.is_some() { RestoreOutcome::Updated } else { RestoreOutcome::Created })
}

async fn restore_overrides(
    tx: &mut Transaction<'_, Sqlite>,
    bundle: &Bundle,
    refs: &HashMap<String, i64>,
) -> Result<SectionReport> {
    let mut report = SectionReport::new("rule_overrides");
    for over in &bundle.rule_overrides {
        let Some(target_id) = refs.get(&over.target).copied() else {
            report.skipped += 1;
            report.notes.push(format!(
                "Per-monitor setting of rule \"{}\": its monitor ({}) is not in this backup.",
                over.rule_uid, over.target
            ));
            continue;
        };
        let existing = sqlx::query(
            "SELECT threshold, clear_threshold, enabled FROM rule_overrides
             WHERE rule_uid = ? AND target_id = ?",
        )
        .bind(&over.rule_uid)
        .bind(target_id)
        .fetch_optional(&mut **tx)
        .await
        .context("recherche de la surcharge")?;

        if let Some(row) = &existing {
            let enabled: Option<i64> = row.try_get("enabled")?;
            if row.try_get::<Option<f64>, _>("threshold")? == over.threshold
                && row.try_get::<Option<f64>, _>("clear_threshold")? == over.clear_threshold
                && enabled.map(|v| v != 0) == over.enabled
            {
                report.skipped += 1;
                continue;
            }
        }

        sqlx::query(
            "INSERT INTO rule_overrides (rule_uid, target_id, threshold, clear_threshold, enabled)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(rule_uid, target_id) DO UPDATE SET
                 threshold = excluded.threshold, clear_threshold = excluded.clear_threshold,
                 enabled = excluded.enabled, updated_at = datetime('now')",
        )
        .bind(&over.rule_uid)
        .bind(target_id)
        .bind(over.threshold)
        .bind(over.clear_threshold)
        .bind(over.enabled.map(i64::from))
        .execute(&mut **tx)
        .await
        .context("enregistrement de la surcharge")?;
        report.count(if existing.is_some() {
            RestoreOutcome::Updated
        } else {
            RestoreOutcome::Created
        });
    }
    Ok(report)
}

async fn restore_notify_policy(
    tx: &mut Transaction<'_, Sqlite>,
    bundle: &Bundle,
) -> Result<SectionReport> {
    let mut report = SectionReport::new("notify_policy");
    let Some(policy) = &bundle.notify_policy else { return Ok(report) };
    let value = serde_json::to_vec(policy)?;

    let existing = sqlx::query("SELECT value FROM settings WHERE key = ?")
        .bind(crate::notify::policy_store::POLICY_KEY)
        .fetch_optional(&mut **tx)
        .await
        .context("lecture de la politique de notification")?;
    if let Some(row) = &existing
        && row.try_get::<Vec<u8>, _>("value")? == value
    {
        report.skipped += 1;
        return Ok(report);
    }

    sqlx::query(
        "INSERT INTO settings (key, value) VALUES (?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value,
             updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')",
    )
    .bind(crate::notify::policy_store::POLICY_KEY)
    .bind(&value)
    .execute(&mut **tx)
    .await
    .context("écriture de la politique de notification")?;
    report.count(if existing.is_some() {
        RestoreOutcome::Updated
    } else {
        RestoreOutcome::Created
    });
    Ok(report)
}

/// Réglages d'instance (`crate::instance`). L'assistant de démarrage reste
/// terminé s'il l'était ici : restaurer une instance n'oblige pas à le refaire,
/// et un lot d'une instance déjà installée le marque terminé.
async fn restore_instance(
    tx: &mut Transaction<'_, Sqlite>,
    bundle: &Bundle,
) -> Result<SectionReport> {
    let mut report = SectionReport::new("instance");
    let Some(incoming) = &bundle.instance else { return Ok(report) };

    let existing = sqlx::query("SELECT value FROM settings WHERE key = ?")
        .bind(crate::instance::KEY)
        .fetch_optional(&mut **tx)
        .await
        .context("lecture des réglages d'instance")?;
    let current: Option<crate::instance::InstanceSettings> = match &existing {
        Some(row) => serde_json::from_slice(&row.try_get::<Vec<u8>, _>("value")?).ok(),
        None => None,
    };

    let mut next = incoming.clone();
    if let Some(current) = &current
        && current.setup_completed
        && !next.setup_completed
    {
        next.setup_completed = true;
        next.setup_completed_at.clone_from(&current.setup_completed_at);
    }
    if current.as_ref() == Some(&next) {
        report.skipped += 1;
        return Ok(report);
    }

    sqlx::query(
        "INSERT INTO settings (key, value) VALUES (?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value,
             updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')",
    )
    .bind(crate::instance::KEY)
    .bind(serde_json::to_vec(&next)?)
    .execute(&mut **tx)
    .await
    .context("écriture des réglages d'instance")?;
    report.count(if existing.is_some() {
        RestoreOutcome::Updated
    } else {
        RestoreOutcome::Created
    });
    Ok(report)
}

// --------------------------------------------------------------------------
// Silences, pages de statut, incidents
// --------------------------------------------------------------------------

async fn restore_silences(
    tx: &mut Transaction<'_, Sqlite>,
    bundle: &Bundle,
    refs: &HashMap<String, i64>,
) -> Result<SectionReport> {
    let mut report = SectionReport::new("silences");
    for silence in &bundle.silences {
        let target_id = match &silence.target {
            None => None,
            Some(reference) => match refs.get(reference) {
                Some(id) => Some(*id),
                None => {
                    report.skipped += 1;
                    report.notes.push(format!(
                        "Maintenance window \"{}\": its device ({reference}) is not in this \
                         backup.",
                        silence.name
                    ));
                    continue;
                }
            },
        };
        let matchers = serde_json::to_string(&silence.matchers)?;
        let schedule = serde_json::to_string(&silence.schedule)?;

        let existing = sqlx::query(
            "SELECT id, comment, matchers, schedule, enabled FROM silences
             WHERE name = ? AND target_id IS ?",
        )
        .bind(&silence.name)
        .bind(target_id)
        .fetch_optional(&mut **tx)
        .await
        .context("recherche du silence")?;

        match existing {
            Some(row) => {
                let same = row.try_get::<String, _>("comment")? == silence.comment
                    && row.try_get::<String, _>("matchers")? == matchers
                    && row.try_get::<String, _>("schedule")? == schedule
                    && (row.try_get::<i64, _>("enabled")? != 0) == silence.enabled;
                if same {
                    report.skipped += 1;
                    continue;
                }
                sqlx::query(
                    "UPDATE silences SET comment = ?, matchers = ?, schedule = ?, enabled = ?
                     WHERE id = ?",
                )
                .bind(&silence.comment)
                .bind(&matchers)
                .bind(&schedule)
                .bind(i64::from(silence.enabled))
                .bind(row.try_get::<i64, _>("id")?)
                .execute(&mut **tx)
                .await
                .context("mise à jour du silence")?;
                report.updated += 1;
            }
            None => {
                sqlx::query(
                    "INSERT INTO silences (name, comment, target_id, matchers, schedule, enabled)
                     VALUES (?, ?, ?, ?, ?, ?)",
                )
                .bind(&silence.name)
                .bind(&silence.comment)
                .bind(target_id)
                .bind(&matchers)
                .bind(&schedule)
                .bind(i64::from(silence.enabled))
                .execute(&mut **tx)
                .await
                .context("création du silence")?;
                report.created += 1;
            }
        }
    }
    Ok(report)
}

async fn restore_report_schedules(
    tx: &mut Transaction<'_, Sqlite>,
    bundle: &Bundle,
    channels: &HashMap<String, i64>,
) -> Result<SectionReport> {
    use crate::reports::schedule::{Frequency, parse_timezone};

    let mut report = SectionReport::new("report_schedules");
    for item in &bundle.report_schedules {
        // Une sauvegarde éditée à la main ne doit pas contourner les contrôles de l'API.
        let name = item.name.trim();
        let valid = !name.is_empty()
            && name.chars().count() <= 80
            && !name.chars().any(char::is_control)
            && Frequency::parse(&item.frequency).is_some()
            && item.weekday <= 6
            && (1..=28).contains(&item.day_of_month)
            && item.hour <= 23
            && parse_timezone(&item.timezone).is_some();
        let recipients = crate::reports::normalise_recipients(&item.recipients);
        let (true, Ok(recipients)) = (valid, recipients) else {
            report.skipped += 1;
            report
                .notes
                .push(format!("Report \"{}\" has invalid settings and was skipped.", item.name));
            continue;
        };
        let channel_id = match &item.channel {
            None => None,
            Some(channel) => match channels.get(channel) {
                Some(id) => Some(*id),
                None => {
                    report.notes.push(format!(
                        "Report \"{name}\": the channel \"{channel}\" is not in this backup; the \
                         first email channel will be used."
                    ));
                    None
                }
            },
        };
        let recipients_json = serde_json::to_string(&recipients)?;
        let enabled = item.enabled && !recipients.is_empty();

        let existing = sqlx::query(
            "SELECT id, enabled, frequency, weekday, day_of_month, hour, timezone, recipients,
                 channel_id
             FROM report_schedules WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(&mut **tx)
        .await
        .context("recherche du rapport planifié")?;

        match existing {
            Some(row) => {
                let same = (row.try_get::<i64, _>("enabled")? != 0) == enabled
                    && row.try_get::<String, _>("frequency")? == item.frequency
                    && row.try_get::<i64, _>("weekday")? == i64::from(item.weekday)
                    && row.try_get::<i64, _>("day_of_month")? == i64::from(item.day_of_month)
                    && row.try_get::<i64, _>("hour")? == i64::from(item.hour)
                    && row.try_get::<String, _>("timezone")? == item.timezone.trim()
                    && row.try_get::<String, _>("recipients")? == recipients_json
                    && row.try_get::<Option<i64>, _>("channel_id")? == channel_id;
                if same {
                    report.skipped += 1;
                    continue;
                }
                // Réarmé : un créneau antérieur à la restauration ne part pas après coup.
                sqlx::query(
                    "UPDATE report_schedules
                     SET enabled = ?, frequency = ?, weekday = ?, day_of_month = ?, hour = ?,
                         timezone = ?, recipients = ?, channel_id = ?,
                         armed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
                     WHERE id = ?",
                )
                .bind(i64::from(enabled))
                .bind(&item.frequency)
                .bind(i64::from(item.weekday))
                .bind(i64::from(item.day_of_month))
                .bind(i64::from(item.hour))
                .bind(item.timezone.trim())
                .bind(&recipients_json)
                .bind(channel_id)
                .bind(row.try_get::<i64, _>("id")?)
                .execute(&mut **tx)
                .await
                .context("mise à jour du rapport planifié")?;
                report.updated += 1;
            }
            None => {
                sqlx::query(
                    "INSERT INTO report_schedules
                         (name, enabled, frequency, weekday, day_of_month, hour, timezone,
                          recipients, channel_id, armed_at)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                )
                .bind(name)
                .bind(i64::from(enabled))
                .bind(&item.frequency)
                .bind(i64::from(item.weekday))
                .bind(i64::from(item.day_of_month))
                .bind(i64::from(item.hour))
                .bind(item.timezone.trim())
                .bind(&recipients_json)
                .bind(channel_id)
                .execute(&mut **tx)
                .await
                .context("création du rapport planifié")?;
                report.created += 1;
            }
        }
    }
    Ok(report)
}

async fn restore_status_pages(
    tx: &mut Transaction<'_, Sqlite>,
    bundle: &Bundle,
    refs: &HashMap<String, i64>,
) -> Result<SectionReport> {
    let mut report = SectionReport::new("status_pages");
    for page in &bundle.status_pages {
        // Une sauvegarde éditée à la main ne doit pas contourner le jeu fermé.
        let mut kept: Vec<&str> = Vec::new();
        for scene in &page.scenes {
            // Une scène retirée se replie sur la plus proche conservée.
            if let Some(canonical) = crate::db::status_pages::canonical_scene(scene)
                && !kept.contains(&canonical)
            {
                kept.push(canonical);
            }
        }
        let scenes = kept.join(",");
        let scene_rotation =
            if crate::db::status_pages::SCENE_ROTATIONS.contains(&page.scene_rotation.as_str()) {
                page.scene_rotation.as_str()
            } else {
                "visit"
            };
        // Domaine : revalidé (sauvegarde éditée à la main), et abandonné avec une
        // note s'il désigne déjà une autre page de cette instance.
        let mut domain = match page.domain.as_deref() {
            None => None,
            Some(raw) => match crate::status_host::normalise_domain(raw) {
                Ok(domain) => domain,
                Err(_) => {
                    report.notes.push(format!(
                        "Status page \"{}\": the domain {raw} is not valid and was left out.",
                        page.slug
                    ));
                    None
                }
            },
        };
        if let Some(name) = domain.as_deref() {
            let taken: Option<(String,)> =
                sqlx::query_as("SELECT slug FROM status_pages WHERE domain = ? AND slug != ?")
                    .bind(name)
                    .bind(&page.slug)
                    .fetch_optional(&mut **tx)
                    .await
                    .context("recherche du domaine d'une page de statut")?;
            if let Some((other,)) = taken {
                report.notes.push(format!(
                    "Status page \"{}\": the domain {name} is already used by \"{other}\" \
                     and was left out.",
                    page.slug
                ));
                domain = None;
            }
        }
        let existing = sqlx::query(
            "SELECT id, title, description, published, theme, show_uptime_days,
                 accent, scenes, scene_rotation, footer_text, homepage_url, domain
             FROM status_pages WHERE slug = ?",
        )
        .bind(&page.slug)
        .fetch_optional(&mut **tx)
        .await
        .context("recherche de la page de statut")?;

        let (id, mut outcome) = match existing {
            Some(row) => {
                let id: i64 = row.try_get("id")?;
                let same = row.try_get::<String, _>("title")? == page.title
                    && row.try_get::<String, _>("description")? == page.description
                    && (row.try_get::<i64, _>("published")? != 0) == page.published
                    && row.try_get::<String, _>("theme")? == page.theme
                    && row.try_get::<i64, _>("show_uptime_days")? == page.show_uptime_days
                    && row.try_get::<String, _>("accent")? == page.accent
                    && row.try_get::<String, _>("scenes")? == scenes
                    && row.try_get::<String, _>("scene_rotation")? == scene_rotation
                    && row.try_get::<String, _>("footer_text")? == page.footer_text
                    && row.try_get::<String, _>("homepage_url")? == page.homepage_url
                    && row.try_get::<Option<String>, _>("domain")? == domain;
                if !same {
                    sqlx::query(
                        "UPDATE status_pages SET title = ?, description = ?, published = ?,
                             theme = ?, show_uptime_days = ?, accent = ?, scenes = ?,
                             scene_rotation = ?, footer_text = ?, homepage_url = ?,
                             domain = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                         WHERE id = ?",
                    )
                    .bind(&page.title)
                    .bind(&page.description)
                    .bind(i64::from(page.published))
                    .bind(&page.theme)
                    .bind(page.show_uptime_days)
                    .bind(&page.accent)
                    .bind(&scenes)
                    .bind(scene_rotation)
                    .bind(&page.footer_text)
                    .bind(&page.homepage_url)
                    .bind(&domain)
                    .bind(id)
                    .execute(&mut **tx)
                    .await
                    .context("mise à jour de la page de statut")?;
                }
                (id, if same { RestoreOutcome::Skipped } else { RestoreOutcome::Updated })
            }
            None => {
                let row = sqlx::query(
                    "INSERT INTO status_pages
                         (slug, title, description, published, theme, show_uptime_days,
                          accent, scenes, scene_rotation, footer_text, homepage_url, domain)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
                )
                .bind(&page.slug)
                .bind(&page.title)
                .bind(&page.description)
                .bind(i64::from(page.published))
                .bind(&page.theme)
                .bind(page.show_uptime_days)
                .bind(&page.accent)
                .bind(&scenes)
                .bind(scene_rotation)
                .bind(&page.footer_text)
                .bind(&page.homepage_url)
                .bind(&domain)
                .fetch_one(&mut **tx)
                .await
                .context("création de la page de statut")?;
                (row.try_get("id")?, RestoreOutcome::Created)
            }
        };
        sqlx::query("UPDATE status_pages SET simple = ? WHERE id = ? AND simple != ?")
            .bind(i64::from(page.simple))
            .bind(id)
            .bind(i64::from(page.simple))
            .execute(&mut **tx)
            .await
            .context("mode simple de la page de statut")?;

        // Accès : revalidé contre le jeu fermé. Absent (page publique, ou ancienne
        // sauvegarde), la page garde son accès.
        if let Some(mode) = page.access_mode.as_deref()
            && crate::api::status_pages::access::MODES.contains(&mode)
        {
            // Sans empreinte, un mode « mot de passe » refuserait tout le monde.
            let mode = if mode == "password" && page.access_password_hash.is_none() {
                "users"
            } else {
                mode
            };
            sqlx::query(
                "UPDATE status_pages SET access_mode = ?, access_password_hash = ?
                 WHERE id = ? AND (access_mode != ? OR access_password_hash IS NOT ?)",
            )
            .bind(mode)
            .bind(&page.access_password_hash)
            .bind(id)
            .bind(mode)
            .bind(&page.access_password_hash)
            .execute(&mut **tx)
            .await
            .context("accès de la page de statut")?;
        }

        // Référencement : revalidé comme à la saisie ; absent (ancienne
        // sauvegarde), la page garde ses réglages.
        if let Some(seo) = &page.seo {
            match seo.clone().validate() {
                Ok(seo) => {
                    let current: String =
                        sqlx::query_scalar("SELECT seo FROM status_pages WHERE id = ?")
                            .bind(id)
                            .fetch_one(&mut **tx)
                            .await
                            .context("lecture du référencement de la page de statut")?;
                    if crate::status_seo::Seo::parse(&current) != seo {
                        sqlx::query("UPDATE status_pages SET seo = ? WHERE id = ?")
                            .bind(seo.to_json())
                            .bind(id)
                            .execute(&mut **tx)
                            .await
                            .context("référencement de la page de statut")?;
                        if outcome == RestoreOutcome::Skipped {
                            outcome = RestoreOutcome::Updated;
                        }
                    }
                }
                Err(why) => report.notes.push(format!(
                    "Status page \"{}\": the SEO settings were left out ({why})",
                    page.slug
                )),
            }
        }

        // Canaux d'abonnement : un canal de notification désigné par son nom
        // qui n'existe pas ici laisse le réglage tel qu'il est.
        if let Some(subscriptions) = &page.subscriptions {
            let channels = channel_ids(tx).await?;
            let named =
                |name: &Option<String>| name.as_ref().and_then(|n| channels.get(n).copied());
            let email = named(&subscriptions.email_channel);
            let sms = named(&subscriptions.sms_channel);
            for (wanted, got, label) in [
                (&subscriptions.email_channel, email, "email"),
                (&subscriptions.sms_channel, sms, "SMS"),
            ] {
                if wanted.is_some() && got.is_none() {
                    report.notes.push(format!(
                        "Status page \"{}\": the {label} subscription channel is not on this instance",
                        page.slug
                    ));
                }
            }
            sqlx::query(
                "UPDATE status_pages SET subscribe_webhook = ?, subscribe_slack = ?,
                     subscribe_channel_id = COALESCE(?, subscribe_channel_id),
                     subscribe_sms_channel_id = COALESCE(?, subscribe_sms_channel_id)
                 WHERE id = ?",
            )
            .bind(i64::from(subscriptions.webhook))
            .bind(i64::from(subscriptions.slack))
            .bind(email)
            .bind(sms)
            .bind(id)
            .execute(&mut **tx)
            .await
            .context("canaux d'abonnement de la page de statut")?;
        }

        // Dossiers cochés : ajoutés ou remis à leur rang, jamais retirés.
        for (position, name) in page.groups.iter().enumerate() {
            let name = name.trim();
            if name.is_empty() {
                continue;
            }
            let changed = sqlx::query(
                "INSERT INTO status_page_groups (page_id, group_name, position) VALUES (?, ?, ?)
                 ON CONFLICT(page_id, group_name) DO UPDATE SET position = excluded.position
                 WHERE position IS NOT excluded.position",
            )
            .bind(id)
            .bind(name)
            .bind(position as i64)
            .execute(&mut **tx)
            .await
            .context("enregistrement d'un groupe de page de statut")?;
            if changed.rows_affected() > 0 && outcome == RestoreOutcome::Skipped {
                outcome = RestoreOutcome::Updated;
            }
        }
        for entry in &page.group_labels {
            let Some(target_id) = refs.get(&entry.target).copied() else {
                report.notes.push(format!(
                    "Status page \"{}\": the monitor {} named in a group is not in this backup.",
                    page.slug, entry.target
                ));
                continue;
            };
            let changed = sqlx::query(
                "INSERT INTO status_page_group_labels (page_id, target_id, label) VALUES (?, ?, ?)
                 ON CONFLICT(page_id, target_id) DO UPDATE SET label = excluded.label
                 WHERE label IS NOT excluded.label",
            )
            .bind(id)
            .bind(target_id)
            .bind(&entry.label)
            .execute(&mut **tx)
            .await
            .context("enregistrement d'un nom public de groupe")?;
            if changed.rows_affected() > 0 && outcome == RestoreOutcome::Skipped {
                outcome = RestoreOutcome::Updated;
            }
        }

        for item in &page.items {
            let Some(target_id) = refs.get(&item.target).copied() else {
                report.notes.push(format!(
                    "Status page \"{}\": the monitor {} it listed is not in this backup.",
                    page.slug, item.target
                ));
                continue;
            };
            let changed = sqlx::query(
                "INSERT INTO status_page_items (page_id, target_id, label, group_name, position)
                 VALUES (?, ?, ?, ?, ?)
                 ON CONFLICT(page_id, target_id) DO UPDATE SET
                     label = excluded.label, group_name = excluded.group_name,
                     position = excluded.position
                 WHERE label IS NOT excluded.label OR group_name IS NOT excluded.group_name
                    OR position IS NOT excluded.position",
            )
            .bind(id)
            .bind(target_id)
            .bind(&item.label)
            .bind(&item.group_name)
            .bind(item.position)
            .execute(&mut **tx)
            .await
            .context("enregistrement d'un service de page de statut")?;
            if changed.rows_affected() > 0 && outcome == RestoreOutcome::Skipped {
                outcome = RestoreOutcome::Updated;
            }
        }
        report.count(outcome);
    }
    Ok(report)
}

async fn restore_incidents(
    tx: &mut Transaction<'_, Sqlite>,
    bundle: &Bundle,
) -> Result<SectionReport> {
    let mut report = SectionReport::new("incidents");
    let pages: HashMap<String, i64> = sqlx::query("SELECT id, slug FROM status_pages")
        .fetch_all(&mut **tx)
        .await
        .context("relecture des pages de statut")?
        .iter()
        .map(|row| Ok((row.try_get("slug")?, row.try_get("id")?)))
        .collect::<Result<_>>()?;

    for incident in &bundle.incidents {
        let page_id = match &incident.page {
            None => None,
            Some(slug) => match pages.get(slug) {
                Some(id) => Some(*id),
                None => {
                    report.skipped += 1;
                    report.notes.push(format!(
                        "Incident \"{}\": its status page ({slug}) is not in this backup.",
                        incident.title
                    ));
                    continue;
                }
            },
        };

        // Un incident est un fait daté, pas un réglage : s'il est déjà là, on
        // le laisse tel quel plutôt que de réécrire un récit.
        let existing = sqlx::query(
            "SELECT id FROM incidents WHERE page_id IS ? AND title = ? AND starts_at = ?",
        )
        .bind(page_id)
        .bind(&incident.title)
        .bind(&incident.starts_at)
        .fetch_optional(&mut **tx)
        .await
        .context("recherche de l'incident")?;
        if existing.is_some() {
            report.skipped += 1;
            continue;
        }

        let row = sqlx::query(
            "INSERT INTO incidents (page_id, title, kind, status, severity, starts_at, ends_at)
             VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(page_id)
        .bind(&incident.title)
        .bind(&incident.kind)
        .bind(&incident.status)
        .bind(&incident.severity)
        .bind(&incident.starts_at)
        .bind(&incident.ends_at)
        .fetch_one(&mut **tx)
        .await
        .context("création de l'incident")?;
        let id: i64 = row.try_get("id")?;

        for update in &incident.updates {
            sqlx::query(
                "INSERT INTO incident_updates (incident_id, status, body, created_at)
                 VALUES (?, ?, ?, ?)",
            )
            .bind(id)
            .bind(&update.status)
            .bind(&update.body)
            .bind(&update.created_at)
            .execute(&mut **tx)
            .await
            .context("création d'une mise à jour d'incident")?;
        }
        report.created += 1;
    }
    Ok(report)
}

// --------------------------------------------------------------------------
// Comptes et jetons
// --------------------------------------------------------------------------

async fn restore_users(
    tx: &mut Transaction<'_, Sqlite>,
    cipher: &Cipher,
    bundle: &Bundle,
) -> Result<SectionReport> {
    let mut report = SectionReport::new("users");
    for user in &bundle.users {
        if crate::auth::users::Role::parse(&user.role).is_none() {
            report.refused(
                &format!("Account \"{}\"", user.username),
                &anyhow::anyhow!("unknown role \"{}\"", user.role),
            );
            continue;
        }

        let existing = sqlx::query("SELECT id FROM users WHERE username = ? COLLATE NOCASE")
            .bind(&user.username)
            .fetch_optional(&mut **tx)
            .await
            .context("recherche du compte")?;
        if existing.is_some() {
            // Jamais écrasé : un fichier ne doit pas pouvoir changer le rôle ou
            // le mot de passe d'un compte existant.
            report.skipped += 1;
            report.notes.push(format!(
                "Account \"{}\" already exists here and was left untouched.",
                user.username
            ));
            continue;
        }

        // `oidc_subject` est unique : s'il est déjà pris, le compte est créé
        // sans son rattachement SSO plutôt que refusé en entier.
        let mut subject = user.oidc_subject.clone();
        if let Some(value) = &subject {
            let taken = sqlx::query("SELECT 1 FROM users WHERE oidc_subject = ?")
                .bind(value)
                .fetch_optional(&mut **tx)
                .await
                .context("recherche du rattachement SSO")?;
            if taken.is_some() {
                report.notes.push(format!(
                    "Account \"{}\": its SSO identity already belongs to another account here; \
                     it was restored without the link.",
                    user.username
                ));
                subject = None;
            }
        }

        // Même règle pour l'identité SAML, unique par IdP.
        let mut saml = user.saml_subject.clone().zip(user.saml_issuer.clone());
        if let Some((subject, issuer)) = &saml {
            let taken =
                sqlx::query("SELECT 1 FROM users WHERE saml_subject = ? AND saml_issuer = ?")
                    .bind(subject)
                    .bind(issuer)
                    .fetch_optional(&mut **tx)
                    .await
                    .context("recherche du rattachement SAML")?;
            if taken.is_some() {
                report.notes.push(format!(
                    "Account \"{}\": its SAML identity already belongs to another account here; \
                     it was restored without the link.",
                    user.username
                ));
                saml = None;
            }
        }
        let (saml_subject, saml_issuer) = saml.unzip();

        // `scim_id` est unique : déjà pris, le compte est restauré sans son
        // rattachement SCIM, que le fournisseur refera.
        let mut scim_id = user.scim_id.clone();
        if let Some(value) = &scim_id {
            let taken = sqlx::query("SELECT 1 FROM users WHERE scim_id = ?")
                .bind(value)
                .fetch_optional(&mut **tx)
                .await
                .context("recherche du rattachement SCIM")?;
            if taken.is_some() {
                report.notes.push(format!(
                    "Account \"{}\": its SCIM identity already belongs to another account here; \
                     it was restored without the link.",
                    user.username
                ));
                scim_id = None;
            }
        }

        let totp_secret = user
            .totp_secret
            .as_ref()
            .and_then(|raw| BASE64.decode(raw).ok())
            .map(|plain| cipher.encrypt(&plain))
            .transpose()?;

        let row = sqlx::query(
            "INSERT INTO users (username, display_name, role, password_hash, oidc_subject,
                                oidc_issuer, saml_subject, saml_issuer, disabled, totp_secret,
                                totp_enabled, scim_id, scim_external_id, scim_email,
                                disabled_by_scim)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(&user.username)
        .bind(&user.display_name)
        .bind(&user.role)
        .bind(&user.password_hash)
        .bind(&subject)
        .bind(&user.oidc_issuer)
        .bind(&saml_subject)
        .bind(&saml_issuer)
        .bind(i64::from(user.disabled))
        .bind(totp_secret.as_deref())
        .bind(i64::from(user.totp_enabled && totp_secret.is_some()))
        .bind(&scim_id)
        .bind(scim_id.as_ref().and(user.scim_external_id.as_ref()))
        .bind(scim_id.as_ref().and(user.scim_email.as_ref()))
        .bind(i64::from(user.disabled && user.disabled_by_scim))
        .fetch_one(&mut **tx)
        .await
        .context("création du compte")?;
        let id: i64 = row.try_get("id")?;

        for code in &user.recovery_codes {
            let Ok(hash) = BASE64.decode(code) else { continue };
            sqlx::query("INSERT INTO totp_recovery_codes (user_id, code_hash) VALUES (?, ?)")
                .bind(id)
                .bind(hash)
                .execute(&mut **tx)
                .await
                .context("création d'un code de secours")?;
        }
        report.created += 1;
    }
    Ok(report)
}

async fn restore_agent_tokens(
    tx: &mut Transaction<'_, Sqlite>,
    bundle: &Bundle,
) -> Result<SectionReport> {
    let mut report = SectionReport::new("agent_tokens");
    for token in &bundle.agent_tokens {
        let existing = sqlx::query("SELECT 1 FROM agent_tokens WHERE token_hash = ?")
            .bind(&token.token_hash)
            .fetch_optional(&mut **tx)
            .await
            .context("recherche du jeton d'agent")?;
        if existing.is_some() {
            report.skipped += 1;
            continue;
        }
        sqlx::query(
            "INSERT INTO agent_tokens (name, token_hash, prefix, created_at, revoked_at)
             VALUES (?, ?, ?, COALESCE(?, strftime('%Y-%m-%dT%H:%M:%SZ', 'now')), ?)",
        )
        .bind(&token.name)
        .bind(&token.token_hash)
        .bind(&token.prefix)
        .bind(&token.created_at)
        .bind(&token.revoked_at)
        .execute(&mut **tx)
        .await
        .context("création du jeton d'agent")?;
        report.created += 1;
    }
    Ok(report)
}

async fn restore_api_tokens(
    tx: &mut Transaction<'_, Sqlite>,
    bundle: &Bundle,
) -> Result<SectionReport> {
    let mut report = SectionReport::new("api_tokens");
    for token in &bundle.api_tokens {
        let Ok(hash) = BASE64.decode(&token.token_hash) else {
            report.refused(
                &format!("API token \"{}\"", token.name),
                &anyhow::anyhow!("its fingerprint is not valid base64"),
            );
            continue;
        };
        let existing = sqlx::query("SELECT 1 FROM api_tokens WHERE token_hash = ?")
            .bind(&hash)
            .fetch_optional(&mut **tx)
            .await
            .context("recherche du jeton d'API")?;
        if existing.is_some() {
            report.skipped += 1;
            continue;
        }
        let owner: Option<i64> = match &token.user {
            None => None,
            Some(username) => sqlx::query("SELECT id FROM users WHERE username = ? COLLATE NOCASE")
                .bind(username)
                .fetch_optional(&mut **tx)
                .await
                .context("recherche du propriétaire du jeton")?
                .map(|row| row.try_get("id"))
                .transpose()?,
        };
        match sqlx::query(
            "INSERT INTO api_tokens (name, prefix, token_hash, scope, created_at, revoked_at,
                                     user_id)
             VALUES (?, ?, ?, ?, COALESCE(?, strftime('%Y-%m-%dT%H:%M:%SZ', 'now')), ?, ?)",
        )
        .bind(&token.name)
        .bind(&token.prefix)
        .bind(&hash)
        .bind(&token.scope)
        .bind(&token.created_at)
        .bind(&token.revoked_at)
        .bind(owner)
        .execute(&mut **tx)
        .await
        {
            Ok(_) => report.created += 1,
            Err(error) => report.refused(&format!("API token \"{}\"", token.name), &error.into()),
        }
    }
    Ok(report)
}

/// Clés d'hôte mémorisées : identique, laissée ; différente, remplacée par
/// celle du lot (c'est elle que l'opérateur a validée).
async fn restore_netconfig_host_keys(
    tx: &mut Transaction<'_, Sqlite>,
    bundle: &Bundle,
    refs: &HashMap<String, i64>,
) -> Result<SectionReport> {
    let mut report = SectionReport::new("netconfig_host_keys");
    for entry in &bundle.netconfig_host_keys {
        let Some(target_id) = refs.get(&entry.target).copied() else {
            report.skipped += 1;
            report.notes.push(format!(
                "SSH host key: its device ({}) is not in this backup.",
                entry.target
            ));
            continue;
        };
        let existing: Option<Option<String>> =
            sqlx::query_scalar("SELECT host_key FROM netconfig_state WHERE target_id = ?")
                .bind(target_id)
                .fetch_optional(&mut **tx)
                .await
                .context("recherche de la clé d'hôte")?;
        if existing.as_ref().and_then(Option::as_deref) == Some(entry.host_key.as_str()) {
            report.skipped += 1;
            continue;
        }
        let outcome = sqlx::query(
            "INSERT INTO netconfig_state (target_id, host_key, host_key_pinned_at)
             VALUES (?, ?, datetime('now'))
             ON CONFLICT (target_id) DO UPDATE
             SET host_key = excluded.host_key, host_key_pinned_at = excluded.host_key_pinned_at,
                 host_key_seen = NULL, host_key_seen_at = NULL",
        )
        .bind(target_id)
        .bind(&entry.host_key)
        .execute(&mut **tx)
        .await
        .map(|_| match existing {
            Some(_) => RestoreOutcome::Updated,
            None => RestoreOutcome::Created,
        });
        match outcome {
            Ok(outcome) => report.count(outcome),
            Err(error) => {
                report.refused(&format!("SSH host key of {}", entry.target), &error.into())
            }
        }
    }
    Ok(report)
}

/// Versions des configurations réseau : rechiffrées avec la clé de cette
/// instance, empreinte de comparaison recalculée. Une version déjà présente
/// (même équipement, même date de relevé) est laissée.
async fn restore_netconfig_versions(
    tx: &mut Transaction<'_, Sqlite>,
    cipher: &Cipher,
    bundle: &Bundle,
    refs: &HashMap<String, i64>,
) -> Result<SectionReport> {
    let mut report = SectionReport::new("netconfig_versions");
    for entry in &bundle.netconfig_versions {
        let Some(target_id) = refs.get(&entry.target).copied() else {
            report.skipped += 1;
            continue;
        };
        let present: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM netconfig_versions WHERE target_id = ? AND fetched_at = ?",
        )
        .bind(target_id)
        .bind(&entry.fetched_at)
        .fetch_optional(&mut **tx)
        .await
        .context("recherche d'une version de configuration")?;
        let too_large = entry.content.len() > dumbmonit_collectors::netconfig::MAX_CONFIG_BYTES;
        if present.is_some() || too_large {
            report.skipped += 1;
            continue;
        }
        let sealed = crate::netconfig::seal(cipher, &entry.content)?;
        let digest = crate::netconfig::digest_of(cipher, &entry.profile, &entry.content);
        let outcome = sqlx::query(
            "INSERT INTO netconfig_versions
                 (target_id, fetched_at, last_seen_at, profile, digest, size_bytes, lines,
                  added, removed, content_enc)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(target_id)
        .bind(&entry.fetched_at)
        .bind(&entry.last_seen_at)
        .bind(&entry.profile)
        .bind(&digest)
        .bind(entry.content.len() as i64)
        .bind(entry.content.lines().count() as i64)
        .bind(entry.added)
        .bind(entry.removed)
        .bind(&sealed)
        .execute(&mut **tx)
        .await
        .map(|_| RestoreOutcome::Created);
        match outcome {
            Ok(outcome) => report.count(outcome),
            Err(error) => report.refused(
                &format!("Configuration version of {} ({})", entry.target, entry.fetched_at),
                &error.into(),
            ),
        }
    }
    Ok(report)
}

async fn restore_push_monitors(
    tx: &mut Transaction<'_, Sqlite>,
    cipher: &Cipher,
    bundle: &Bundle,
    refs: &HashMap<String, i64>,
) -> Result<SectionReport> {
    let mut report = SectionReport::new("push_monitors");
    for monitor in &bundle.push_monitors {
        let Some(target_id) = refs.get(&monitor.target).copied() else {
            report.skipped += 1;
            report.notes.push(format!(
                "Heartbeat token: its monitor ({}) is not in this backup.",
                monitor.target
            ));
            continue;
        };
        let fingerprint = crate::collectors::push::token::fingerprint(&monitor.token);
        let existing = sqlx::query("SELECT token_hash FROM push_monitors WHERE target_id = ?")
            .bind(target_id)
            .fetch_optional(&mut **tx)
            .await
            .context("recherche du moniteur en poussée")?;

        if let Some(row) = &existing
            && row.try_get::<String, _>("token_hash")? == fingerprint
        {
            report.skipped += 1;
            continue;
        }

        let encrypted = cipher.encrypt(monitor.token.as_bytes())?;
        let outcome = match &existing {
            Some(_) => sqlx::query(
                "UPDATE push_monitors SET token_hash = ?, token_enc = ? WHERE target_id = ?",
            )
            .bind(&fingerprint)
            .bind(&encrypted)
            .bind(target_id)
            .execute(&mut **tx)
            .await
            .map(|_| RestoreOutcome::Updated),
            None => sqlx::query(
                "INSERT INTO push_monitors (target_id, token_hash, token_enc) VALUES (?, ?, ?)",
            )
            .bind(target_id)
            .bind(&fingerprint)
            .bind(&encrypted)
            .execute(&mut **tx)
            .await
            .map(|_| RestoreOutcome::Created),
        };
        match outcome {
            Ok(outcome) => report.count(outcome),
            Err(error) => {
                report.refused(&format!("Heartbeat token of {}", monitor.target), &error.into())
            }
        }
    }
    Ok(report)
}

/// Paquets d'intégration : revérifiés par ce serveur-ci, qui peut être d'une
/// autre version que celui qui a écrit le lot. Un paquet identique (même
/// empreinte) est laissé tel quel ; un paquet différent remplace l'installé,
/// son état activé ou non compris. Les règles viennent de la section `rules`.
/// Emplacements, par nom (sans casse) : identique, laissé ; différent, mis à
/// jour. Puis les affectations des équipements du lot, qui ne comptent pas
/// comme des changements de la section : elles suivent les équipements.
async fn restore_locations(
    tx: &mut Transaction<'_, Sqlite>,
    bundle: &Bundle,
    refs: &HashMap<String, i64>,
) -> Result<SectionReport> {
    let mut report = SectionReport::new("locations");
    for entry in &bundle.locations {
        let name = entry.name.trim();
        let valid = !name.is_empty()
            && entry.latitude.is_finite()
            && (-90.0..=90.0).contains(&entry.latitude)
            && entry.longitude.is_finite()
            && (-180.0..=180.0).contains(&entry.longitude);
        if !valid {
            report.skipped += 1;
            report.notes.push(format!("location \"{name}\" has no valid name or coordinates"));
            continue;
        }
        let existing = sqlx::query(
            "SELECT id, address, latitude, longitude, site, main_site FROM locations
             WHERE name = ? COLLATE NOCASE",
        )
        .bind(name)
        .fetch_optional(&mut **tx)
        .await
        .context("recherche de l'emplacement")?;
        if let Some(row) = &existing
            && row.try_get::<String, _>("address")? == entry.address
            && row.try_get::<f64, _>("latitude")? == entry.latitude
            && row.try_get::<f64, _>("longitude")? == entry.longitude
            && row.try_get::<String, _>("site")? == entry.site
            && (row.try_get::<i64, _>("main_site")? != 0) == entry.main_site
        {
            report.skipped += 1;
            continue;
        }
        if entry.main_site {
            sqlx::query("UPDATE locations SET main_site = 0 WHERE name != ? COLLATE NOCASE")
                .bind(name)
                .execute(&mut **tx)
                .await
                .context("site principal")?;
        }
        let result = sqlx::query(
            "INSERT INTO locations (name, address, latitude, longitude, site, main_site)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(name) DO UPDATE SET address = excluded.address,
                 latitude = excluded.latitude, longitude = excluded.longitude,
                 site = excluded.site, main_site = excluded.main_site",
        )
        .bind(name)
        .bind(&entry.address)
        .bind(entry.latitude)
        .bind(entry.longitude)
        .bind(&entry.site)
        .bind(entry.main_site)
        .execute(&mut **tx)
        .await;
        match result {
            Ok(_) if existing.is_some() => report.count(RestoreOutcome::Updated),
            Ok(_) => report.count(RestoreOutcome::Created),
            Err(error) => report.refused(&format!("location \"{name}\""), &error.into()),
        }
    }
    for target in &bundle.targets {
        let Some(location) = target.location.as_deref() else { continue };
        let Some(id) = refs.get(&target_ref(&target.kind, &target.address)) else { continue };
        sqlx::query(
            "UPDATE targets SET location_id =
                 (SELECT id FROM locations WHERE name = ? COLLATE NOCASE)
             WHERE id = ?",
        )
        .bind(location)
        .bind(id)
        .execute(&mut **tx)
        .await
        .context("emplacement d'un équipement")?;
    }
    Ok(report)
}

/// Équipes : créées ou mises à jour par nom. Les droits sont remplacés par ceux
/// du lot ; les membres du lot sont ajoutés (un compte absent est signalé), sans
/// retirer ceux que l'instance a déjà.
async fn restore_teams(tx: &mut Transaction<'_, Sqlite>, bundle: &Bundle) -> Result<SectionReport> {
    let mut report = SectionReport::new("teams");
    if let Some(settings) = &bundle.teams_settings
        && serde_json::from_value::<crate::auth::teams::TeamSettings>(settings.clone()).is_ok()
    {
        sqlx::query(
            "INSERT INTO settings (key, value) VALUES (?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        )
        .bind(crate::auth::teams::SETTINGS_KEY)
        .bind(serde_json::to_vec(settings)?)
        .execute(&mut **tx)
        .await
        .context("réglages des équipes")?;
    }
    for team in &bundle.teams {
        let Ok(name) = crate::auth::teams::validate_name(&team.name) else {
            report.skipped += 1;
            report.notes.push(format!("team \"{}\" has no valid name", team.name));
            continue;
        };
        let existing: Option<i64> =
            sqlx::query_scalar("SELECT id FROM teams WHERE name = ? COLLATE NOCASE")
                .bind(&name)
                .fetch_optional(&mut **tx)
                .await
                .context("recherche de l'équipe")?;
        let id: i64 = match existing {
            Some(id) => {
                sqlx::query("UPDATE teams SET description = ?, idp_groups = ? WHERE id = ?")
                    .bind(&team.description)
                    .bind(serde_json::to_string(&team.idp_groups)?)
                    .bind(id)
                    .execute(&mut **tx)
                    .await
                    .context("mise à jour de l'équipe")?;
                report.count(RestoreOutcome::Updated);
                id
            }
            None => {
                let id = sqlx::query_scalar(
                    "INSERT INTO teams (name, description, idp_groups) VALUES (?, ?, ?)
                     RETURNING id",
                )
                .bind(&name)
                .bind(&team.description)
                .bind(serde_json::to_string(&team.idp_groups)?)
                .fetch_one(&mut **tx)
                .await
                .context("création de l'équipe")?;
                report.count(RestoreOutcome::Created);
                id
            }
        };
        // Rattachement SCIM : repris seulement sur une équipe qui n'en a pas, et
        // si aucune autre ne porte déjà cet identifiant.
        if let Some(scim_id) = &team.scim_id {
            sqlx::query(
                "UPDATE teams SET scim_id = ?, scim_external_id = ?, scim_created = ?
                 WHERE id = ? AND scim_id IS NULL
                   AND NOT EXISTS (SELECT 1 FROM teams WHERE scim_id = ?)",
            )
            .bind(scim_id)
            .bind(&team.scim_external_id)
            .bind(i64::from(team.scim_created))
            .bind(id)
            .bind(scim_id)
            .execute(&mut **tx)
            .await
            .context("rattachement SCIM de l'équipe")?;
        }
        sqlx::query("DELETE FROM team_folder_grants WHERE team_id = ?")
            .bind(id)
            .execute(&mut **tx)
            .await
            .context("droits de l'équipe")?;
        for grant in &team.grants {
            sqlx::query(
                "INSERT INTO team_folder_grants (team_id, folder, level) VALUES (?, ?, ?)
                 ON CONFLICT(team_id, folder) DO UPDATE SET level = excluded.level",
            )
            .bind(id)
            .bind(crate::auth::teams::normalize_folder(&grant.folder))
            .bind(grant.level.as_str())
            .execute(&mut **tx)
            .await
            .context("droit de l'équipe")?;
        }
        for member in &team.members {
            let source = match member.source.as_str() {
                "idp" | "scim" => member.source.as_str(),
                _ => "manual",
            };
            let user: Option<i64> =
                sqlx::query_scalar("SELECT id FROM users WHERE username = ? COLLATE NOCASE")
                    .bind(&member.username)
                    .fetch_optional(&mut **tx)
                    .await
                    .context("membre de l'équipe")?;
            let Some(user) = user else {
                report.notes.push(format!(
                    "team \"{name}\": account \"{}\" not found, membership skipped",
                    member.username
                ));
                continue;
            };
            sqlx::query(
                "INSERT INTO team_members (team_id, user_id, source) VALUES (?, ?, ?)
                 ON CONFLICT(team_id, user_id) DO NOTHING",
            )
            .bind(id)
            .bind(user)
            .bind(source)
            .execute(&mut **tx)
            .await
            .context("membre de l'équipe")?;
        }
    }
    Ok(report)
}

/// Tableaux de bord, par propriétaire et nom (sans casse) : identique, laissé ;
/// différent, mis à jour. Un tableau personnel dont le compte n'existe pas ici
/// est ignoré (les comptes sont restaurés avant).
async fn restore_dashboards(
    tx: &mut Transaction<'_, Sqlite>,
    bundle: &Bundle,
) -> Result<SectionReport> {
    use crate::api::dashboards::{DashboardPayload, validate};
    let mut report = SectionReport::new("dashboards");
    for entry in &bundle.dashboards {
        let name = entry.name.trim();
        let widgets = match serde_json::from_value(if entry.widgets.is_null() {
            serde_json::json!([])
        } else {
            entry.widgets.clone()
        }) {
            Ok(widgets) => widgets,
            Err(_) => {
                report.skipped += 1;
                report.notes.push(format!("dashboard \"{name}\" has an unreadable grid"));
                continue;
            }
        };
        let input = match validate(DashboardPayload {
            name: name.to_string(),
            shared: entry.shared,
            range_secs: entry.range_secs,
            refresh_secs: entry.refresh_secs,
            widgets,
        }) {
            Ok(input) => input,
            Err(_) => {
                report.skipped += 1;
                report.notes.push(format!("dashboard \"{name}\" is not valid"));
                continue;
            }
        };
        let owner_id: Option<i64> = match &entry.owner {
            Some(owner) => {
                sqlx::query_scalar("SELECT id FROM users WHERE username = ? COLLATE NOCASE")
                    .bind(owner)
                    .fetch_optional(&mut **tx)
                    .await
                    .context("propriétaire d'un tableau de bord")?
            }
            None => None,
        };
        if !entry.shared && owner_id.is_none() {
            report.skipped += 1;
            report.notes.push(format!("dashboard \"{name}\": its owner does not exist here"));
            continue;
        }
        let widgets = serde_json::to_string(&input.widgets).context("grille d'un tableau")?;
        let existing = sqlx::query(
            "SELECT id, range_secs, refresh_secs, widgets FROM dashboards
             WHERE name = ? COLLATE NOCASE AND shared = ? AND (shared = 1 OR owner_id = ?)",
        )
        .bind(&input.name)
        .bind(input.shared)
        .bind(owner_id)
        .fetch_optional(&mut **tx)
        .await
        .context("recherche du tableau de bord")?;
        if let Some(row) = &existing {
            if row.try_get::<i64, _>("range_secs")? == input.range_secs
                && row.try_get::<i64, _>("refresh_secs")? == input.refresh_secs
                && row.try_get::<String, _>("widgets")? == widgets
            {
                report.skipped += 1;
                continue;
            }
            let id: i64 = row.try_get("id")?;
            let result = sqlx::query(
                "UPDATE dashboards SET range_secs = ?, refresh_secs = ?, widgets = ?,
                     updated_at = strftime('%Y-%m-%d %H:%M:%S', 'now') WHERE id = ?",
            )
            .bind(input.range_secs)
            .bind(input.refresh_secs)
            .bind(&widgets)
            .bind(id)
            .execute(&mut **tx)
            .await;
            match result {
                Ok(_) => report.count(RestoreOutcome::Updated),
                Err(error) => report.refused(&format!("dashboard \"{name}\""), &error.into()),
            }
            continue;
        }
        let result = sqlx::query(
            "INSERT INTO dashboards (name, shared, owner_id, range_secs, refresh_secs, widgets)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&input.name)
        .bind(input.shared)
        .bind(owner_id)
        .bind(input.range_secs)
        .bind(input.refresh_secs)
        .bind(&widgets)
        .execute(&mut **tx)
        .await;
        match result {
            Ok(_) => report.count(RestoreOutcome::Created),
            Err(error) => report.refused(&format!("dashboard \"{name}\""), &error.into()),
        }
    }
    Ok(report)
}

/// Un nœud du lot, avec les identifiants d'ici ; `None` : son équipement
/// n'est pas là.
fn local_node(node: &BundleNode, refs: &HashMap<String, i64>) -> Option<(String, i64)> {
    let host = *refs.get(&node.target)?;
    Some((topology_edits::node_from_parts(host, node.child.as_deref())?, host))
}

/// Corrections de la carte : une correction identique déjà là est laissée.
/// Le parent d'alerte d'un rattachement voyage déjà avec l'équipement
/// (`parent`) : rien n'est réécrit dans `targets`.
async fn restore_topology_corrections(
    tx: &mut Transaction<'_, Sqlite>,
    bundle: &Bundle,
    refs: &HashMap<String, i64>,
) -> Result<SectionReport> {
    let mut report = SectionReport::new("topology_corrections");
    for entry in &bundle.topology_corrections {
        let Some(kind) = CorrectionKind::parse(&entry.kind) else {
            report.skipped += 1;
            report.notes.push(format!("map correction of unknown kind \"{}\"", entry.kind));
            continue;
        };
        let missing = |report: &mut SectionReport| {
            report.skipped += 1;
            report.notes.push(format!(
                "map correction on \"{}\": a device it points at is not in this instance",
                entry.to.target
            ));
        };
        let Some((to, to_target)) = local_node(&entry.to, refs) else {
            missing(&mut report);
            continue;
        };
        let (from, from_target) = match &entry.from {
            Some(node) => match local_node(node, refs) {
                Some((id, target)) => (Some(id), Some(target)),
                None => {
                    missing(&mut report);
                    continue;
                }
            },
            None => (None, None),
        };
        let same: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM topology_corrections
             WHERE kind = ? AND from_node IS ? AND to_node = ? AND source IS ? AND value IS ?",
        )
        .bind(kind.as_str())
        .bind(&from)
        .bind(&to)
        .bind(&entry.source)
        .bind(&entry.value)
        .fetch_optional(&mut **tx)
        .await
        .context("recherche de la correction")?;
        if same.is_some() {
            report.skipped += 1;
            continue;
        }
        let previous = entry.previous_parent.as_ref().and_then(|r| refs.get(r).copied());
        let result = sqlx::query(
            "INSERT INTO topology_corrections (kind, from_node, to_node, from_target, to_target,
                 source, value, use_for_alerts, previous_parent, author, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, COALESCE(NULLIF(?, ''), datetime('now')))",
        )
        .bind(kind.as_str())
        .bind(&from)
        .bind(&to)
        .bind(from_target)
        .bind(to_target)
        .bind(&entry.source)
        .bind(&entry.value)
        .bind(entry.use_for_alerts)
        .bind(previous)
        .bind(&entry.author)
        .bind(&entry.created_at)
        .execute(&mut **tx)
        .await;
        match result {
            Ok(_) => report.count(RestoreOutcome::Created),
            Err(error) => report.refused(&format!("map correction on \"{to}\""), &error.into()),
        }
    }
    Ok(report)
}

/// Liens de LUN, par (NAS, LUN) : identique, laissé ; différent, mis à jour.
async fn restore_lun_links(
    tx: &mut Transaction<'_, Sqlite>,
    bundle: &Bundle,
    refs: &HashMap<String, i64>,
) -> Result<SectionReport> {
    let mut report = SectionReport::new("lun_links");
    for entry in &bundle.lun_links {
        let target = refs.get(&entry.target).copied();
        let consumer = refs.get(&entry.consumer).copied();
        let (Some(target), Some(consumer)) = (target, consumer) else {
            report.skipped += 1;
            report.notes.push(format!(
                "LUN link \"{}\" on \"{}\": a device it points at is not in this instance",
                entry.lun, entry.target
            ));
            continue;
        };
        let existing: Option<(i64, String, String)> = sqlx::query_as(
            "SELECT consumer_target_id, family, entity FROM lun_links
             WHERE target_id = ? AND lun = ?",
        )
        .bind(target)
        .bind(&entry.lun)
        .fetch_optional(&mut **tx)
        .await
        .context("recherche du lien de LUN")?;
        let wanted = (consumer, entry.family.clone(), entry.entity.clone());
        let outcome = match &existing {
            Some(row) if *row == wanted => {
                report.count(RestoreOutcome::Skipped);
                continue;
            }
            Some(_) => RestoreOutcome::Updated,
            None => RestoreOutcome::Created,
        };
        let result = sqlx::query(
            "INSERT INTO lun_links (target_id, lun, consumer_target_id, family, entity, author)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT (target_id, lun) DO UPDATE SET
                consumer_target_id = excluded.consumer_target_id,
                family = excluded.family,
                entity = excluded.entity,
                author = excluded.author",
        )
        .bind(target)
        .bind(&entry.lun)
        .bind(consumer)
        .bind(&entry.family)
        .bind(&entry.entity)
        .bind(&entry.author)
        .execute(&mut **tx)
        .await;
        match result {
            Ok(_) => report.count(outcome),
            Err(error) => report.refused(&format!("LUN link \"{}\"", entry.lun), &error.into()),
        }
    }
    Ok(report)
}

/// Règles de la carte, par leur clé : même état, laissée ; autre état, mise à jour.
async fn restore_topology_rules(
    tx: &mut Transaction<'_, Sqlite>,
    bundle: &Bundle,
    refs: &HashMap<String, i64>,
) -> Result<SectionReport> {
    let mut report = SectionReport::new("topology_rules");
    for entry in &bundle.topology_rules {
        let (Some(kind), Some(status)) =
            (RuleKind::parse(&entry.kind), RuleStatus::parse(&entry.status))
        else {
            report.skipped += 1;
            report.notes.push(format!("map rule of unknown kind \"{}\"", entry.kind));
            continue;
        };
        let parent = match &entry.parent {
            Some(node) => match local_node(node, refs) {
                Some(found) => Some(found),
                None => {
                    report.skipped += 1;
                    report.notes.push(format!(
                        "map rule: the device \"{}\" it depends on is not in this instance",
                        node.target
                    ));
                    continue;
                }
            },
            None => None,
        };
        let spec = RuleSpec {
            kind,
            subnet: entry.subnet.clone(),
            address: entry.address.clone(),
            target_kind: entry.target_kind.clone(),
            source: entry.source.clone(),
            parent: parent.as_ref().map(|(id, _)| id.clone()),
        };
        let key = spec.key();
        let existing: Option<String> =
            sqlx::query_scalar("SELECT status FROM topology_rules WHERE key = ?")
                .bind(&key)
                .fetch_optional(&mut **tx)
                .await
                .context("recherche de la règle de carte")?;
        if existing.as_deref() == Some(status.as_str()) {
            report.skipped += 1;
            continue;
        }
        let result = sqlx::query(
            "INSERT INTO topology_rules (key, kind, status, subnet, address, target_kind, source,
                 parent_node, parent_target, evidence, author, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, COALESCE(NULLIF(?, ''), datetime('now')))
             ON CONFLICT(key) DO UPDATE SET status = excluded.status",
        )
        .bind(&key)
        .bind(kind.as_str())
        .bind(status.as_str())
        .bind(&spec.subnet)
        .bind(&spec.address)
        .bind(&spec.target_kind)
        .bind(&spec.source)
        .bind(&spec.parent)
        .bind(parent.map(|(_, target)| target))
        .bind(entry.evidence)
        .bind(&entry.author)
        .bind(&entry.created_at)
        .execute(&mut **tx)
        .await;
        match result {
            Ok(_) if existing.is_some() => report.count(RestoreOutcome::Updated),
            Ok(_) => report.count(RestoreOutcome::Created),
            Err(error) => report.refused("map rule", &error.into()),
        }
    }
    Ok(report)
}

/// Clients (mode MSP) : créés ou mis à jour par nom. Les membres du lot sont
/// ajoutés (un compte absent est signalé), sans retirer ceux de l'instance.
async fn restore_clients(
    tx: &mut Transaction<'_, Sqlite>,
    bundle: &Bundle,
) -> Result<SectionReport> {
    let mut report = SectionReport::new("clients");
    if let Some(settings) = &bundle.msp_settings
        && serde_json::from_value::<crate::auth::clients::MspSettings>(settings.clone()).is_ok()
    {
        sqlx::query(
            "INSERT INTO settings (key, value) VALUES (?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        )
        .bind(crate::auth::clients::SETTINGS_KEY)
        .bind(serde_json::to_vec(settings)?)
        .execute(&mut **tx)
        .await
        .context("réglage du mode MSP")?;
    }
    for client in &bundle.clients {
        let Ok(name) = crate::auth::clients::validate_name(&client.name) else {
            report.skipped += 1;
            report.notes.push(format!("client \"{}\" has no valid name", client.name));
            continue;
        };
        // Une sauvegarde éditée à la main ne contourne pas les contrôles de l'API.
        let contacts = crate::reports::normalise_recipients(&client.contacts).unwrap_or_default();
        let quota = client.monitor_quota.filter(|q| (0..=1_000_000).contains(q));
        let objective = client.sla_objective.filter(|v| v.is_finite() && *v > 0.0 && *v <= 100.0);
        let valid_tz = crate::reports::schedule::parse_timezone(&client.report_timezone).is_some();
        let timezone =
            if valid_tz { client.report_timezone.trim().to_string() } else { "UTC".to_string() };
        let logo = match (&client.logo, &client.logo_type) {
            (Some(data), Some(_)) => BASE64
                .decode(data)
                .ok()
                .filter(|bytes| bytes.len() <= crate::auth::clients::MAX_LOGO_BYTES)
                .and_then(|bytes| {
                    crate::api::status_pages::branding::sniff(&bytes).map(|kind| (kind, bytes))
                }),
            _ => None,
        };
        let existing: Option<i64> =
            sqlx::query_scalar("SELECT id FROM clients WHERE name = ? COLLATE NOCASE")
                .bind(&name)
                .fetch_optional(&mut **tx)
                .await
                .context("recherche du client")?;
        let id: i64 = match existing {
            Some(id) => id,
            None => {
                report.count(RestoreOutcome::Created);
                sqlx::query_scalar("INSERT INTO clients (name) VALUES (?) RETURNING id")
                    .bind(&name)
                    .fetch_one(&mut **tx)
                    .await
                    .context("création du client")?
            }
        };
        let changed = sqlx::query(
            "UPDATE clients SET description = ?1, contacts = ?2, monitor_quota = ?3, idp_groups = ?4,
                    sla_report = ?5, sla_objective = ?6, report_timezone = ?7,
                    logo_type = ?8, logo = ?9
             WHERE id = ?10
               AND (description IS NOT ?1 OR contacts IS NOT ?2 OR monitor_quota IS NOT ?3
                    OR idp_groups IS NOT ?4 OR sla_report IS NOT ?5 OR sla_objective IS NOT ?6
                    OR report_timezone IS NOT ?7 OR logo_type IS NOT ?8 OR logo IS NOT ?9)",
        )
        .bind(&client.description)
        .bind(serde_json::to_string(&contacts)?)
        .bind(quota)
        .bind(serde_json::to_string(&client.idp_groups)?)
        .bind(i64::from(client.sla_report))
        .bind(objective)
        .bind(&timezone)
        .bind(logo.as_ref().map(|(kind, _)| *kind))
        .bind(logo.as_ref().map(|(_, bytes)| bytes.clone()))
        .bind(id)
        .execute(&mut **tx)
        .await
        .context("mise à jour du client")?
        .rows_affected();
        // Un client déjà identique ne compte pas comme modifié : une seconde
        // restauration du même lot ne change rien.
        if existing.is_some() {
            report.count(if changed > 0 {
                RestoreOutcome::Updated
            } else {
                RestoreOutcome::Skipped
            });
        }
        for member in &client.members {
            let source = match member.source.as_str() {
                "idp" | "scim" => member.source.as_str(),
                _ => "manual",
            };
            let user: Option<i64> =
                sqlx::query_scalar("SELECT id FROM users WHERE username = ? COLLATE NOCASE")
                    .bind(&member.username)
                    .fetch_optional(&mut **tx)
                    .await
                    .context("membre du client")?;
            let Some(user) = user else {
                report.notes.push(format!(
                    "client \"{name}\": account \"{}\" not found, membership skipped",
                    member.username
                ));
                continue;
            };
            sqlx::query(
                "INSERT INTO client_members (client_id, user_id, source) VALUES (?, ?, ?)
                 ON CONFLICT(client_id, user_id) DO NOTHING",
            )
            .bind(id)
            .bind(user)
            .bind(source)
            .execute(&mut **tx)
            .await
            .context("membre du client")?;
        }
    }
    Ok(report)
}

/// Rattachements aux clients, une fois tous les objets restaurés. Les liens
/// qui franchiraient une frontière entre clients (parent, relais, composant de
/// page…) sont ensuite coupés, comme après un déplacement.
async fn restore_client_assignments(
    tx: &mut Transaction<'_, Sqlite>,
    bundle: &Bundle,
    report: &mut SectionReport,
) -> Result<()> {
    use crate::auth::clients::Kind;
    if bundle.client_assignments.is_empty() {
        return Ok(());
    }
    let names: Vec<(i64, String)> = sqlx::query_as("SELECT id, name FROM clients")
        .fetch_all(&mut **tx)
        .await
        .context("relecture des clients")?;
    let by_name: HashMap<String, i64> =
        names.into_iter().map(|(id, name)| (name.to_lowercase(), id)).collect();
    let refs = target_ids(tx).await?;
    for assignment in &bundle.client_assignments {
        let Some(client) = by_name.get(&assignment.client.to_lowercase()).copied() else {
            report.notes.push(format!(
                "client \"{}\" is not in this backup: \"{}\" left where it is",
                assignment.client, assignment.key
            ));
            continue;
        };
        let done = match assignment.kind {
            Kind::Target => match refs.get(&assignment.key) {
                Some(id) => sqlx::query("UPDATE targets SET client_id = ? WHERE id = ?")
                    .bind(client)
                    .bind(*id)
                    .execute(&mut **tx)
                    .await?
                    .rows_affected(),
                None => 0,
            },
            Kind::StatusPage => sqlx::query("UPDATE status_pages SET client_id = ? WHERE slug = ?")
                .bind(client)
                .bind(&assignment.key)
                .execute(&mut **tx)
                .await?
                .rows_affected(),
            Kind::AgentToken => {
                sqlx::query("UPDATE agent_tokens SET client_id = ? WHERE token_hash = ?")
                    .bind(client)
                    .bind(&assignment.key)
                    .execute(&mut **tx)
                    .await?
                    .rows_affected()
            }
            Kind::Channel => {
                sqlx::query("UPDATE notification_channels SET client_id = ? WHERE name = ?")
                    .bind(client)
                    .bind(&assignment.key)
                    .execute(&mut **tx)
                    .await?
                    .rows_affected()
            }
            Kind::Rule => sqlx::query("UPDATE alert_rules SET client_id = ? WHERE uid = ?")
                .bind(client)
                .bind(&assignment.key)
                .execute(&mut **tx)
                .await?
                .rows_affected(),
            Kind::Report => sqlx::query("UPDATE report_schedules SET client_id = ? WHERE name = ?")
                .bind(client)
                .bind(&assignment.key)
                .execute(&mut **tx)
                .await?
                .rows_affected(),
            Kind::Silence => {
                let (name, target) =
                    assignment.key.split_once('\n').unwrap_or((assignment.key.as_str(), ""));
                let target = if target.is_empty() { None } else { refs.get(target).copied() };
                sqlx::query("UPDATE silences SET client_id = ? WHERE name = ? AND target_id IS ?")
                    .bind(client)
                    .bind(name)
                    .bind(target)
                    .execute(&mut **tx)
                    .await?
                    .rows_affected()
            }
            Kind::Dashboard => sqlx::query(
                "UPDATE dashboards SET client_id = ? WHERE shared = 1 AND name = ? COLLATE NOCASE",
            )
            .bind(client)
            .bind(&assignment.key)
            .execute(&mut **tx)
            .await?
            .rows_affected(),
            Kind::Schedule => {
                sqlx::query("UPDATE oncall_schedules SET client_id = ? WHERE name = ?")
                    .bind(client)
                    .bind(&assignment.key)
                    .execute(&mut **tx)
                    .await?
                    .rows_affected()
            }
            Kind::EscalationPolicy => {
                sqlx::query("UPDATE oncall_policies SET client_id = ? WHERE name = ?")
                    .bind(client)
                    .bind(&assignment.key)
                    .execute(&mut **tx)
                    .await?
                    .rows_affected()
            }
            Kind::Runbook => sqlx::query("UPDATE runbooks SET client_id = ? WHERE title = ?")
                .bind(client)
                .bind(&assignment.key)
                .execute(&mut **tx)
                .await?
                .rows_affected(),
            Kind::RemediationAction => {
                sqlx::query("UPDATE remediation_actions SET client_id = ? WHERE name = ?")
                    .bind(client)
                    .bind(&assignment.key)
                    .execute(&mut **tx)
                    .await?
                    .rows_affected()
            }
        };
        if done == 0 {
            report.notes.push(format!(
                "\"{}\" ({}) is not in this instance: its client was not restored",
                assignment.key,
                assignment.kind.key()
            ));
        }
    }
    crate::auth::clients::sever_cross_client_links(tx).await?;
    Ok(())
}

async fn restore_packs(tx: &mut Transaction<'_, Sqlite>, bundle: &Bundle) -> Result<SectionReport> {
    let mut report = SectionReport::new("packs");
    for entry in &bundle.packs {
        let pack = match dumbmonit_pack::Pack::parse(&entry.yaml) {
            Ok(pack) => pack,
            Err(error) => {
                report.skipped += 1;
                report.notes.push(format!(
                    "pack \"{}\" does not pass this server's validation: {}",
                    entry.id,
                    error.errors.join("; ")
                ));
                continue;
            }
        };
        let existing = sqlx::query("SELECT sha256, enabled FROM packs WHERE id = ?")
            .bind(pack.id())
            .fetch_optional(&mut **tx)
            .await
            .context("recherche du paquet")?;
        if let Some(row) = &existing
            && row.try_get::<String, _>("sha256")? == pack.sha256()
            && (row.try_get::<i64, _>("enabled")? != 0) == entry.enabled
        {
            report.skipped += 1;
            continue;
        }
        let result = sqlx::query(
            "INSERT INTO packs (id, version, yaml, sha256, enabled) VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET version = excluded.version, yaml = excluded.yaml,
                 sha256 = excluded.sha256, enabled = excluded.enabled,
                 installed_at = datetime('now')",
        )
        .bind(pack.id())
        .bind(pack.version())
        .bind(pack.yaml())
        .bind(pack.sha256())
        .bind(i64::from(entry.enabled))
        .execute(&mut **tx)
        .await;
        match result {
            Ok(_) if existing.is_some() => report.count(RestoreOutcome::Updated),
            Ok(_) => report.count(RestoreOutcome::Created),
            Err(error) => report.refused(&format!("pack \"{}\"", entry.id), &error.into()),
        }
    }
    Ok(report)
}

/// Procédures, par titre : même texte et mêmes rattachements, laissée ;
/// sinon, remplacée.
async fn restore_runbooks(
    tx: &mut Transaction<'_, Sqlite>,
    bundle: &Bundle,
) -> Result<SectionReport> {
    use crate::remediation::runbooks::RunbookDraft;
    let mut report = SectionReport::new("runbooks");
    for entry in &bundle.runbooks {
        let draft = RunbookDraft {
            title: entry.title.clone(),
            body: entry.body.clone(),
            links: entry.links.clone(),
        };
        let draft = match draft.validate() {
            Ok(draft) => draft,
            Err(why) => {
                report.skipped += 1;
                report.notes.push(format!("runbook \"{}\": {why}", entry.title));
                continue;
            }
        };
        let existing: Option<(i64, String)> =
            sqlx::query_as("SELECT id, body FROM runbooks WHERE title = ? ORDER BY id LIMIT 1")
                .bind(&draft.title)
                .fetch_optional(&mut **tx)
                .await
                .context("recherche de la procédure")?;
        let id = match &existing {
            Some((id, body)) => {
                let links: Vec<(String, String)> = sqlx::query_as(
                    "SELECT scope, value FROM runbook_links WHERE runbook_id = ?
                     ORDER BY scope, value",
                )
                .bind(id)
                .fetch_all(&mut **tx)
                .await
                .context("lecture des rattachements")?;
                let same_links = links.len() == draft.links.len()
                    && links
                        .iter()
                        .zip(&draft.links)
                        .all(|((scope, value), link)| *scope == link.scope && *value == link.value);
                if *body == draft.body && same_links {
                    report.skipped += 1;
                    continue;
                }
                sqlx::query(
                    "UPDATE runbooks SET body = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%S', 'now')
                     WHERE id = ?",
                )
                .bind(&draft.body)
                .bind(id)
                .execute(&mut **tx)
                .await
                .context("mise à jour de la procédure")?;
                *id
            }
            None => sqlx::query_scalar(
                "INSERT INTO runbooks (title, body, updated_by) VALUES (?, ?, 'backup')
                 RETURNING id",
            )
            .bind(&draft.title)
            .bind(&draft.body)
            .fetch_one(&mut **tx)
            .await
            .context("création de la procédure")?,
        };
        sqlx::query("DELETE FROM runbook_links WHERE runbook_id = ?")
            .bind(id)
            .execute(&mut **tx)
            .await
            .context("effacement des rattachements")?;
        for link in &draft.links {
            sqlx::query("INSERT INTO runbook_links (runbook_id, scope, value) VALUES (?, ?, ?)")
                .bind(id)
                .bind(&link.scope)
                .bind(&link.value)
                .execute(&mut **tx)
                .await
                .context("écriture d'un rattachement")?;
        }
        report.count(if existing.is_some() {
            RestoreOutcome::Updated
        } else {
            RestoreOutcome::Created
        });
    }
    Ok(report)
}

/// Actions, par nom. Une action revalidée par le catalogue de ce serveur ; une
/// action modifiée désarme les politiques qui l'emploient.
async fn restore_remediation_actions(
    tx: &mut Transaction<'_, Sqlite>,
    cipher: &Cipher,
    bundle: &Bundle,
    refs: &HashMap<String, i64>,
) -> Result<SectionReport> {
    let mut report = SectionReport::new("remediation_actions");
    for entry in &bundle.remediation_actions {
        let spec = match crate::remediation::ActionSpec::parse(&entry.kind, &entry.params) {
            Ok(spec) => spec,
            Err(why) => {
                report.skipped += 1;
                report.notes.push(format!("action \"{}\": {why}", entry.name));
                continue;
            }
        };
        if let Err(why) = crate::remediation::validate_secrets(spec.kind(), &entry.secrets) {
            report.skipped += 1;
            report.notes.push(format!("action \"{}\": {why}", entry.name));
            continue;
        }
        let target = match &entry.target {
            Some(reference) => match refs.get(reference) {
                Some(id) => Some(*id),
                None => {
                    report.skipped += 1;
                    report.notes.push(format!(
                        "action \"{}\": the device \"{reference}\" is not in this instance",
                        entry.name
                    ));
                    continue;
                }
            },
            None => None,
        };
        let params = spec.to_params().to_string();
        let secrets = if entry.secrets.values().any(|value| !value.is_empty()) {
            Some(cipher.encrypt(&serde_json::to_vec(&entry.secrets)?)?)
        } else {
            None
        };
        let existing: Option<(i64, String, String, Option<i64>)> = sqlx::query_as(
            "SELECT id, kind, params, target_id FROM remediation_actions WHERE name = ?",
        )
        .bind(&entry.name)
        .fetch_optional(&mut **tx)
        .await
        .context("recherche de l'action")?;
        match existing {
            Some((_, kind, current, target_id))
                if kind == spec.kind() && current == params && target_id == target =>
            {
                report.skipped += 1;
            }
            Some((id, ..)) => {
                sqlx::query(
                    "UPDATE remediation_actions SET kind = ?, params = ?, secrets = ?, target_id = ?,
                         updated_at = strftime('%Y-%m-%dT%H:%M:%S', 'now')
                     WHERE id = ?",
                )
                .bind(spec.kind())
                .bind(&params)
                .bind(&secrets)
                .bind(target)
                .bind(id)
                .execute(&mut **tx)
                .await
                .context("mise à jour de l'action")?;
                sqlx::query(
                    "UPDATE remediation_policies SET enabled = 0, armed_by = NULL, armed_at = NULL
                     WHERE action_id = ?",
                )
                .bind(id)
                .execute(&mut **tx)
                .await
                .context("désarmement des politiques")?;
                report.count(RestoreOutcome::Updated);
            }
            None => {
                let result = sqlx::query(
                    "INSERT INTO remediation_actions (name, kind, params, secrets, target_id)
                     VALUES (?, ?, ?, ?, ?)",
                )
                .bind(&entry.name)
                .bind(spec.kind())
                .bind(&params)
                .bind(&secrets)
                .bind(target)
                .execute(&mut **tx)
                .await;
                match result {
                    Ok(_) => report.count(RestoreOutcome::Created),
                    Err(error) => report.refused("action", &error.into()),
                }
            }
        }
    }
    Ok(report)
}

/// Politiques : toujours créées désarmées, jamais en double (même action,
/// même règle, même équipement).
async fn restore_remediation_policies(
    tx: &mut Transaction<'_, Sqlite>,
    bundle: &Bundle,
    refs: &HashMap<String, i64>,
) -> Result<SectionReport> {
    let mut report = SectionReport::new("remediation_policies");
    for entry in &bundle.remediation_policies {
        let action: Option<i64> =
            sqlx::query_scalar("SELECT id FROM remediation_actions WHERE name = ?")
                .bind(&entry.action)
                .fetch_optional(&mut **tx)
                .await
                .context("recherche de l'action de la politique")?;
        let Some(action) = action else {
            report.skipped += 1;
            report.notes.push(format!("policy: the action \"{}\" is missing", entry.action));
            continue;
        };
        let target = match &entry.target {
            Some(reference) => match refs.get(reference) {
                Some(id) => Some(*id),
                None => {
                    report.skipped += 1;
                    report.notes.push(format!(
                        "policy: the device \"{reference}\" is not in this instance"
                    ));
                    continue;
                }
            },
            None => None,
        };
        let exists: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM remediation_policies
             WHERE action_id = ? AND rule_uid = ? AND target_id IS ?",
        )
        .bind(action)
        .bind(&entry.rule_uid)
        .bind(target)
        .fetch_optional(&mut **tx)
        .await
        .context("recherche de la politique")?;
        if exists.is_some() {
            report.skipped += 1;
            continue;
        }
        let result = sqlx::query(
            "INSERT INTO remediation_policies
                 (action_id, rule_uid, target_id, after_minutes, max_attempts, window_minutes,
                  enabled)
             VALUES (?, ?, ?, ?, ?, ?, 0)",
        )
        .bind(action)
        .bind(&entry.rule_uid)
        .bind(target)
        .bind(entry.after_minutes)
        .bind(entry.max_attempts)
        .bind(entry.window_minutes)
        .execute(&mut **tx)
        .await;
        match result {
            Ok(_) => report.count(RestoreOutcome::Created),
            Err(error) => report.refused("policy", &error.into()),
        }
    }
    Ok(report)
}
