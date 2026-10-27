//! Accès aux cibles. C'est ici — et nulle part ailleurs — que les identifiants sont
//! chiffrés et déchiffrés, pour qu'aucun autre module n'ait à y penser.

use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

use anyhow::{Context, Result};
use dumbmonit_proto::{Credential, Target, TargetId};
use sqlx::{Row, SqlitePool, sqlite::SqliteRow};

use crate::crypto::Cipher;

/// Champs modifiables d'une cible, à la création comme à la mise à jour.
#[derive(Debug, Clone)]
pub struct TargetInput {
    pub name: String,
    pub address: String,
    pub kind: String,
    pub profile_id: Option<String>,
    pub parent_id: Option<TargetId>,
    /// Agent relais qui interroge cette cible à la place du serveur. `None` :
    /// le serveur s'en charge lui-même, comme pour toute cible ordinaire.
    pub via_agent: Option<TargetId>,
    pub interval: Duration,
    pub enabled: bool,
    pub tags: BTreeMap<String, String>,
    /// `None` signifie « conserver le secret déjà enregistré ».
    ///
    /// C'est ce qui permet de renommer une cible ou d'ajuster sa période sans
    /// ressaisir sa community — une modification bien plus fréquente que le
    /// changement d'identifiants. Pour effacer réellement un secret, on passe
    /// `Some(Credential::None)`.
    pub credential: Option<Credential>,
    /// Dossier plat et libre, affiché sur `/targets`. Vide : aucun dossier.
    pub group_name: String,
}

pub async fn list(pool: &SqlitePool, cipher: &Cipher) -> Result<Vec<Target>> {
    let rows = sqlx::query(
        "SELECT id, name, address, kind, profile_id, parent_id, interval_secs, enabled,
                tags, credential_enc, group_name, position
         FROM targets ORDER BY position, name",
    )
    .fetch_all(pool)
    .await
    .context("liste des cibles")?;
    rows.iter().map(|row| row_to_target(row, cipher)).collect()
}

/// Cibles actives, telles que consommées par le planificateur.
pub async fn list_enabled(pool: &SqlitePool, cipher: &Cipher) -> Result<Vec<Target>> {
    let rows = sqlx::query(
        "SELECT id, name, address, kind, profile_id, parent_id, interval_secs, enabled,
                tags, credential_enc, group_name, position
         FROM targets WHERE enabled = 1",
    )
    .fetch_all(pool)
    .await
    .context("liste des cibles actives")?;
    rows.iter().map(|row| row_to_target(row, cipher)).collect()
}

pub async fn get(pool: &SqlitePool, cipher: &Cipher, id: TargetId) -> Result<Option<Target>> {
    let row = sqlx::query(
        "SELECT id, name, address, kind, profile_id, parent_id, interval_secs, enabled,
                tags, credential_enc, group_name, position
         FROM targets WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("lecture d'une cible")?;
    row.as_ref().map(|row| row_to_target(row, cipher)).transpose()
}

/// Le quota d'équipements du client est atteint (mode MSP).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuotaReached {
    pub client: i64,
}

impl std::fmt::Display for QuotaReached {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the target quota of client {} is reached", self.client)
    }
}

impl std::error::Error for QuotaReached {}

/// Crée une cible dans le client de son parent ou de son relais, à défaut dans
/// le client par défaut : un équipement dérivé (module d'agent, moniteur géré
/// par étiquettes) reste ainsi chez le client de celui dont il dépend.
pub async fn create(pool: &SqlitePool, cipher: &Cipher, input: &TargetInput) -> Result<TargetId> {
    let mut client = None;
    for linked in [input.parent_id, input.via_agent].into_iter().flatten() {
        client = crate::auth::clients::target_client(pool, linked).await?;
        if client.is_some() {
            break;
        }
    }
    create_in(pool, cipher, input, client.unwrap_or(crate::auth::clients::DEFAULT_CLIENT)).await
}

/// Crée une cible chez le client `client`. Refusé ([`QuotaReached`]) quand le
/// client a atteint son quota d'équipements : la vérification et l'insertion
/// sont une seule instruction, deux créations simultanées ne le dépassent pas.
pub async fn create_in(
    pool: &SqlitePool,
    cipher: &Cipher,
    input: &TargetInput,
    client: i64,
) -> Result<TargetId> {
    // La position de départ place la nouvelle cible après toutes les autres :
    // un rang déjà trié à la main ne doit jamais recevoir une arrivée au hasard.
    let row = sqlx::query(
        "INSERT INTO targets
             (name, address, kind, profile_id, parent_id, via_agent, interval_secs, enabled, tags,
              credential_enc, group_name, position, client_id)
         SELECT ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
                (SELECT COALESCE(MAX(position), 0) + 1 FROM targets), c.id
         FROM clients c
         WHERE c.id = ?
           AND (c.monitor_quota IS NULL
                OR (SELECT COUNT(*) FROM targets t WHERE t.client_id = c.id) < c.monitor_quota)
         RETURNING id",
    )
    .bind(&input.name)
    .bind(&input.address)
    .bind(&input.kind)
    .bind(&input.profile_id)
    .bind(input.parent_id)
    .bind(input.via_agent)
    .bind(input.interval.as_secs() as i64)
    .bind(i64::from(input.enabled))
    .bind(serde_json::to_string(&input.tags)?)
    .bind(encrypt_credential(cipher, input.credential.as_ref().unwrap_or(&Credential::None))?)
    .bind(&input.group_name)
    .bind(client)
    .fetch_optional(pool)
    .await
    .context("création de la cible")?;

    let Some(row) = row else {
        if crate::auth::clients::exists(pool, client).await? {
            return Err(QuotaReached { client }.into());
        }
        anyhow::bail!("unknown client {client}");
    };
    row.try_get("id").context("identifiant de la cible créée")
}

pub async fn update(
    pool: &SqlitePool,
    cipher: &Cipher,
    id: TargetId,
    input: &TargetInput,
) -> Result<bool> {
    // Deux requêtes distinctes plutôt qu'une construite dynamiquement : sqlx refuse
    // le SQL assemblé à la volée, et la colonne du secret doit rester intacte quand
    // l'appelant n'en fournit pas de nouveau.
    let result = match &input.credential {
        Some(credential) => {
            sqlx::query(
                "UPDATE targets SET
                     name = ?, address = ?, kind = ?, profile_id = ?, parent_id = ?,
                     via_agent = ?, interval_secs = ?, enabled = ?, tags = ?, credential_enc = ?,
                     group_name = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                 WHERE id = ?",
            )
            .bind(&input.name)
            .bind(&input.address)
            .bind(&input.kind)
            .bind(&input.profile_id)
            .bind(input.parent_id)
            .bind(input.via_agent)
            .bind(input.interval.as_secs() as i64)
            .bind(i64::from(input.enabled))
            .bind(serde_json::to_string(&input.tags)?)
            .bind(encrypt_credential(cipher, credential)?)
            .bind(&input.group_name)
            .bind(id)
            .execute(pool)
            .await
        }
        None => {
            sqlx::query(
                "UPDATE targets SET
                     name = ?, address = ?, kind = ?, profile_id = ?, parent_id = ?,
                     via_agent = ?, interval_secs = ?, enabled = ?, tags = ?,
                     group_name = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                 WHERE id = ?",
            )
            .bind(&input.name)
            .bind(&input.address)
            .bind(&input.kind)
            .bind(&input.profile_id)
            .bind(input.parent_id)
            .bind(input.via_agent)
            .bind(input.interval.as_secs() as i64)
            .bind(i64::from(input.enabled))
            .bind(serde_json::to_string(&input.tags)?)
            .bind(&input.group_name)
            .bind(id)
            .execute(pool)
            .await
        }
    }
    .context("mise à jour de la cible")?;

    Ok(result.rows_affected() > 0)
}

/// Réécrit le rang manuel d'un lot de cibles d'après leur position dans `order` :
/// la première vaut 0, la suivante 1, etc.
///
/// N'écrase que les cibles listées : une réorganisation locale (un dossier, un
/// Haut/Bas) n'a pas besoin de connaître le rang des autres — `rack.ts` ne
/// compare jamais deux cibles de dossiers ou d'états différents entre elles,
/// donc des rangs numériquement identiques entre deux lots distincts sont sans
/// consequence.
pub async fn reorder(pool: &SqlitePool, order: &[TargetId]) -> Result<()> {
    let mut tx = pool.begin().await.context("ouverture de la transaction de réordonnancement")?;
    for (position, id) in order.iter().enumerate() {
        sqlx::query("UPDATE targets SET position = ? WHERE id = ?")
            .bind(position as i64)
            .bind(id)
            .execute(&mut *tx)
            .await
            .context("réordonnancement d'une cible")?;
    }
    tx.commit().await.context("validation du réordonnancement")?;
    Ok(())
}

/// Renomme un dossier : toutes les cibles qui y sont rangées et, dans la même
/// transaction, les pages de statut qui l'ont coché (la liaison se fait par
/// nom). Si une page cochait déjà le nouveau nom, la référence à l'ancien
/// disparaît au lieu de doubler. Retourne le nombre de cibles déplacées.
pub async fn rename_folder(pool: &SqlitePool, from: &str, to: &str) -> Result<u64> {
    let mut tx = pool.begin().await.context("ouverture de la transaction de renommage")?;
    let moved = sqlx::query(
        "UPDATE targets SET group_name = ?,
             updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
         WHERE group_name = ?",
    )
    .bind(to)
    .bind(from)
    .execute(&mut *tx)
    .await
    .context("renommage du dossier des cibles")?
    .rows_affected();
    sqlx::query("UPDATE OR IGNORE status_page_groups SET group_name = ? WHERE group_name = ?")
        .bind(to)
        .bind(from)
        .execute(&mut *tx)
        .await
        .context("renommage du dossier dans les pages de statut")?;
    // Les lignes que le conflit a épargnées : la page cochait déjà `to`.
    sqlx::query("DELETE FROM status_page_groups WHERE group_name = ?")
        .bind(from)
        .execute(&mut *tx)
        .await
        .context("purge de l'ancien nom de dossier des pages de statut")?;
    tx.commit().await.context("validation du renommage du dossier")?;
    Ok(moved)
}

pub async fn delete(pool: &SqlitePool, id: TargetId) -> Result<bool> {
    let result = sqlx::query("DELETE FROM targets WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("suppression de la cible")?;
    Ok(result.rows_affected() > 0)
}

/// Vrai si un message d'erreur de sonde enregistré signifie « équipement
/// injoignable » plutôt qu'une erreur de configuration.
///
/// Seul le texte est conservé en base ; les préfixes sont ceux de l'affichage de
/// `ProbeError`, dont `means_down` fait la même distinction, et du message des
/// sondes relayées restées sans réponse.
pub fn error_means_down(message: &str) -> bool {
    message.starts_with("Timed out") || message.starts_with("Monitor unreachable")
}

/// Préfixe du verdict d'une sonde de disponibilité en échec (voir
/// [`service_failure`]).
pub const SERVICE_DOWN_PREFIX: &str = "Service down";

/// Vrai si le message enregistré est celui d'un service qui ne répond pas
/// correctement. L'interface l'affiche comme une panne, pas comme une erreur
/// de configuration ; l'alerte, elle, reste `service_down` : la cible n'est pas
/// « injoignable » au sens de la topologie, sans quoi ses propres alertes de
/// service se croiraient étouffées par elle-même.
pub fn error_means_service_down(message: &str) -> bool {
    message.starts_with(SERVICE_DOWN_PREFIX)
}

/// Verdict à enregistrer pour une sonde de disponibilité qui a tourné mais
/// dont le résultat est négatif (`probe_success = 0`).
///
/// Ces sondes renvoient `Ok` avec un zéro plutôt qu'une erreur, pour que le
/// taux de disponibilité compte la panne (voir `uptime/outcome.rs`) : sans ce
/// passage, `last_error` restait vide pendant toute la panne. Les heartbeats
/// (`probe="push"`) ont leur propre règle et ne passent pas par ici.
pub fn service_failure(samples: &[dumbmonit_proto::Sample]) -> Option<String> {
    let failed = samples.iter().find(|s| {
        s.metric == "probe_success"
            && s.value == 0.0
            && s.labels.get("probe").map(String::as_str) != Some("push")
    })?;
    let probe = failed.labels.get("probe").map_or("service", String::as_str);
    let reason = samples
        .iter()
        .find(|s| s.metric == "probe_failure_info")
        .and_then(|s| s.labels.get("reason"))
        .map(|reason| reason.replace('_', " "));
    Some(match reason {
        Some(reason) => format!("{SERVICE_DOWN_PREFIX}: the {probe} check failed ({reason})"),
        None => format!("{SERVICE_DOWN_PREFIX}: the {probe} check failed"),
    })
}

/// Enregistre l'issue d'une interrogation. `error` à `None` signifie succès.
pub async fn record_probe(pool: &SqlitePool, id: TargetId, error: Option<&str>) -> Result<()> {
    sqlx::query(
        "UPDATE targets SET last_probe_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), last_error = ? WHERE id = ?",
    )
    .bind(error)
    .bind(id)
    .execute(pool)
    .await
    .context("enregistrement du résultat d'interrogation")?;
    Ok(())
}

/// Active ou désactive une cible sans toucher au reste de sa définition.
///
/// Le point d'entrée « modifier » exige la cible complète ; ici l'appelant (le
/// serveur MCP) ne connaît que l'identifiant et le drapeau, et ne doit surtout pas
/// réécrire le reste.
pub async fn set_enabled(pool: &SqlitePool, id: TargetId, enabled: bool) -> Result<bool> {
    let result = sqlx::query(
        "UPDATE targets SET enabled = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
         WHERE id = ?",
    )
    .bind(i64::from(enabled))
    .bind(id)
    .execute(pool)
    .await
    .context("activation de la cible")?;
    Ok(result.rows_affected() > 0)
}

/// Associe un profil détecté à une cible.
pub async fn set_profile(pool: &SqlitePool, id: TargetId, profile_id: &str) -> Result<()> {
    sqlx::query("UPDATE targets SET profile_id = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?")
        .bind(profile_id)
        .bind(id)
        .execute(pool)
        .await
        .context("association du profil")?;
    Ok(())
}

fn row_to_target(row: &SqliteRow, cipher: &Cipher) -> Result<Target> {
    let id: TargetId = row.try_get("id")?;
    let tags_json: String = row.try_get("tags")?;
    let credential_enc: Option<Vec<u8>> = row.try_get("credential_enc")?;

    Ok(Target {
        id,
        name: row.try_get("name")?,
        address: row.try_get("address")?,
        kind: row.try_get("kind")?,
        profile_id: row.try_get("profile_id")?,
        parent_id: row.try_get("parent_id")?,
        interval: Duration::from_secs(row.try_get::<i64, _>("interval_secs")?.max(1) as u64),
        enabled: row.try_get::<i64, _>("enabled")? != 0,
        tags: serde_json::from_str(&tags_json)
            .with_context(|| format!("étiquettes illisibles pour la cible {id}"))?,
        credential: decrypt_credential(cipher, credential_enc.as_deref())
            .with_context(|| format!("identifiants illisibles pour la cible {id}"))?,
        group_name: row.try_get("group_name")?,
        position: row.try_get("position")?,
    })
}

fn encrypt_credential(cipher: &Cipher, credential: &Credential) -> Result<Option<Vec<u8>>> {
    if matches!(credential, Credential::None) {
        return Ok(None);
    }
    let json = serde_json::to_vec(credential)?;
    Ok(Some(cipher.encrypt(&json)?))
}

fn decrypt_credential(cipher: &Cipher, data: Option<&[u8]>) -> Result<Credential> {
    match data {
        None => Ok(Credential::None),
        Some(bytes) => Ok(serde_json::from_slice(&cipher.decrypt(bytes)?)?),
    }
}

/// Relais des cibles : identifiant de la cible → identifiant de l'agent qui
/// l'interroge. Les cibles interrogées par le serveur n'y figurent pas.
///
/// Tenu à part de [`Target`] : le relais est une affaire d'acheminement propre au
/// serveur, et les collecteurs — qu'ils tournent ici ou sur l'agent — n'ont pas à
/// le connaître.
pub async fn relay_map(pool: &SqlitePool) -> Result<HashMap<TargetId, TargetId>> {
    let rows = sqlx::query("SELECT id, via_agent FROM targets WHERE via_agent IS NOT NULL")
        .fetch_all(pool)
        .await
        .context("lecture des relais")?;
    rows.into_iter().map(|row| Ok((row.try_get("id")?, row.try_get("via_agent")?))).collect()
}

/// Agent relais d'une cible, s'il y en a un.
pub async fn relay_of(pool: &SqlitePool, id: TargetId) -> Result<Option<TargetId>> {
    let row = sqlx::query("SELECT via_agent FROM targets WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("lecture du relais")?;
    Ok(row.and_then(|row| row.try_get::<Option<TargetId>, _>("via_agent").ok().flatten()))
}

/// Nombre de cibles relayées par chaque agent.
pub async fn relayed_counts(pool: &SqlitePool) -> Result<HashMap<TargetId, usize>> {
    let rows = sqlx::query(
        "SELECT via_agent, COUNT(*) AS n FROM targets WHERE via_agent IS NOT NULL GROUP BY via_agent",
    )
    .fetch_all(pool)
    .await
    .context("décompte des cibles relayées")?;
    rows.into_iter()
        .map(|row| Ok((row.try_get("via_agent")?, row.try_get::<i64, _>("n")?.max(0) as usize)))
        .collect()
}

/// Gestion d'un moniteur par les étiquettes d'un conteneur Docker.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Managed {
    /// Origine : `docker-labels`.
    pub by: String,
    /// Agent dont les conteneurs déclarent le moniteur.
    pub source: Option<TargetId>,
    pub key: String,
    /// Posé quand les étiquettes ne le déclarent plus (moniteur archivé).
    pub missing_since: Option<String>,
}

/// Valeur de `managed_by` pour les moniteurs nés des étiquettes Docker.
pub const MANAGED_DOCKER_LABELS: &str = "docker-labels";

fn managed_from_row(row: &SqliteRow) -> Result<Option<Managed>> {
    let by: Option<String> = row.try_get("managed_by")?;
    Ok(by.map(|by| Managed {
        by,
        source: row.try_get::<Option<i64>, _>("managed_source").ok().flatten(),
        key: row.try_get::<Option<String>, _>("managed_key").ok().flatten().unwrap_or_default(),
        missing_since: row.try_get::<Option<String>, _>("managed_missing_since").ok().flatten(),
    }))
}

/// Les moniteurs gérés d'un agent.
pub async fn managed_of(pool: &SqlitePool, source: TargetId) -> Result<Vec<(TargetId, Managed)>> {
    let rows = sqlx::query(
        "SELECT id, managed_by, managed_source, managed_key, managed_missing_since
         FROM targets WHERE managed_by IS NOT NULL AND managed_source = ?",
    )
    .bind(source)
    .fetch_all(pool)
    .await
    .context("lecture des moniteurs gérés")?;
    rows.iter()
        .filter_map(|row| match managed_from_row(row) {
            Ok(Some(managed)) => row.try_get("id").ok().map(|id| Ok((id, managed))),
            Ok(None) => None,
            Err(error) => Some(Err(error)),
        })
        .collect()
}

/// La gestion d'une cible, `None` si elle est libre.
pub async fn managed_get(pool: &SqlitePool, id: TargetId) -> Result<Option<Managed>> {
    let row = sqlx::query(
        "SELECT managed_by, managed_source, managed_key, managed_missing_since
         FROM targets WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("lecture de la gestion d'une cible")?;
    row.as_ref().map(managed_from_row).transpose().map(Option::flatten)
}

/// Place une cible sous la gestion des étiquettes d'un agent.
pub async fn set_managed(
    pool: &SqlitePool,
    id: TargetId,
    source: TargetId,
    key: &str,
) -> Result<()> {
    sqlx::query(
        "UPDATE targets SET managed_by = ?, managed_source = ?, managed_key = ?,
             managed_missing_since = NULL WHERE id = ?",
    )
    .bind(MANAGED_DOCKER_LABELS)
    .bind(source)
    .bind(key)
    .bind(id)
    .execute(pool)
    .await
    .context("marquage d'un moniteur géré")?;
    Ok(())
}

/// Rend une cible à la gestion manuelle. Vrai si elle était gérée.
pub async fn release_managed(pool: &SqlitePool, id: TargetId) -> Result<bool> {
    let result = sqlx::query(
        "UPDATE targets SET managed_by = NULL, managed_source = NULL, managed_key = NULL,
             managed_missing_since = NULL WHERE id = ? AND managed_by IS NOT NULL",
    )
    .bind(id)
    .execute(pool)
    .await
    .context("libération d'un moniteur géré")?;
    Ok(result.rows_affected() > 0)
}

/// Archive (désactive et date) ou réactive un moniteur géré.
pub async fn set_missing(pool: &SqlitePool, id: TargetId, missing: bool) -> Result<()> {
    let sql = if missing {
        "UPDATE targets SET enabled = 0,
             managed_missing_since = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
         WHERE id = ? AND managed_missing_since IS NULL"
    } else {
        "UPDATE targets SET enabled = 1, managed_missing_since = NULL
         WHERE id = ? AND managed_missing_since IS NOT NULL"
    };
    sqlx::query(sql).bind(id).execute(pool).await.context("archivage d'un moniteur géré")?;
    Ok(())
}

/// Cible de ce type et de cette adresse, gérée ou non.
pub async fn find_by_address(
    pool: &SqlitePool,
    kind: &str,
    address: &str,
) -> Result<Option<(TargetId, Option<String>)>> {
    let row = sqlx::query("SELECT id, managed_by FROM targets WHERE kind = ? AND address = ?")
        .bind(kind)
        .bind(address)
        .fetch_optional(pool)
        .await
        .context("recherche d'une cible par adresse")?;
    row.map(|row| Ok((row.try_get("id")?, row.try_get("managed_by")?))).transpose()
}

/// Issue de la dernière interrogation d'une cible, pour l'affichage.
#[derive(Debug, Clone)]
pub struct TargetStatus {
    pub managed: Option<Managed>,
    pub last_probe_at: Option<String>,
    pub last_error: Option<String>,
    /// Agent relais, recopié ici pour que l'API l'expose sans relire la cible.
    pub via_agent: Option<TargetId>,
}

/// Statut de toutes les cibles, indexé par identifiant.
pub async fn statuses(pool: &SqlitePool) -> Result<HashMap<TargetId, TargetStatus>> {
    let rows = sqlx::query(
        "SELECT id, last_probe_at, last_error, via_agent, managed_by, managed_source,
                managed_key, managed_missing_since FROM targets",
    )
    .fetch_all(pool)
    .await
    .context("lecture des statuts")?;

    rows.into_iter()
        .map(|row| {
            Ok((
                row.try_get("id")?,
                TargetStatus {
                    last_probe_at: row.try_get("last_probe_at")?,
                    last_error: row.try_get("last_error")?,
                    via_agent: row.try_get("via_agent")?,
                    managed: managed_from_row(&row)?,
                },
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use dumbmonit_proto::{MetricKind, Sample};

    use super::*;

    fn gauge(metric: &str, value: f64, probe: &str) -> Sample {
        Sample::new(metric, value, MetricKind::Gauge, 0).with_label("probe", probe)
    }

    /// Une sonde NTP ou WebSocket en échec renvoie `Ok` avec un zéro : son
    /// verdict doit quand même atteindre `last_error`, classé comme une panne.
    #[test]
    fn une_sonde_de_service_en_echec_laisse_un_verdict() {
        let samples = vec![
            gauge("probe_success", 0.0, "ntp"),
            gauge("probe_duration_seconds", 5.0, "ntp"),
            gauge("probe_failure_info", 1.0, "ntp").with_label("reason", "timeout"),
        ];
        let verdict = service_failure(&samples).expect("verdict");
        assert_eq!(verdict, "Service down: the ntp check failed (timeout)");
        assert!(error_means_service_down(&verdict));
        assert!(!error_means_down(&verdict), "the target itself is not unreachable");
    }

    #[test]
    fn une_sonde_reussie_ou_un_heartbeat_ne_laisse_aucun_verdict() {
        assert!(service_failure(&[gauge("probe_success", 1.0, "websocket")]).is_none());
        assert!(service_failure(&[gauge("probe_success", 0.0, "push")]).is_none());
        assert!(service_failure(&[gauge("up", 1.0, "snmp")]).is_none());
    }
}
