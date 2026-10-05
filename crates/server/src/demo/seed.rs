//! Le parc fictif de la démonstration, écrit dans une base neuve.
//!
//! Tout est fixe : les mêmes équipements, dans le même ordre (donc avec les
//! mêmes identifiants), la même page de statut, les mêmes incidents passés. Les
//! seules dates sont relatives au démarrage et calées sur l'heure pleine.

use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::{DateTime, Timelike, Utc};
use dumbmonit_proto::{AgentIdentity, Credential, PUSH_PROTOCOL_VERSION, PushBatch, TargetId};
use sqlx::SqlitePool;

use super::estate::DemoDevice;
use super::synthetic::{self, AGENT_HOST, FLAKY_HOST};
use super::{DEMO_PASSWORD, DEMO_USERNAME};
use crate::auth::password;
use crate::auth::users::{self, NewUser, Role};
use crate::collectors::agent::{self, TokenPolicy};
use crate::crypto::Cipher;
use crate::db;
use crate::db::status_pages::{IncidentInput, PageItemInput, StatusPageInput};
use crate::db::targets::TargetInput;
use crate::tsdb::SampleSink;

/// Sonde synthétique : (nom, type, adresse, parent désigné par son nom).
const CHECKS: &[(&str, &str, &str, Option<&str>)] = &[
    ("nextcloud.home.arpa", "http", "https://nextcloud.home.arpa/status.php", None),
    ("home-assistant.home.arpa", "http", "https://home-assistant.home.arpa/", None),
    (FLAKY_HOST, "http", "https://jellyfin.home.arpa/health", None),
    ("vault.home.arpa", "tls", "vault.home.arpa:443", None),
    ("nextcloud.home.arpa (TLS)", "tls", "nextcloud.home.arpa:443", None),
    ("printer.home.arpa", "ping", "printer.home.arpa", None),
    ("switch-attic.home.arpa", "ping", "switch-attic.home.arpa", None),
    ("ap-attic.home.arpa", "ping", "ap-attic.home.arpa", Some("switch-attic.home.arpa")),
];

/// Ce que la suite du démarrage doit connaître du parc.
pub struct Seeded {
    /// Jeton d'enregistrement de l'agent simulé, en clair : il ne quitte pas le
    /// processus.
    pub agent_token: String,
    pub targets: Vec<(TargetId, String, String)>,
}

pub async fn run(
    pool: &SqlitePool,
    cipher: &Cipher,
    sink: &SampleSink,
    devices: &[DemoDevice],
) -> Result<Seeded> {
    let hash = password::hash(DEMO_PASSWORD.to_string()).await?;
    users::insert(
        pool,
        NewUser {
            username: DEMO_USERNAME,
            display_name: "Demo visitor",
            // Administrateur pour que l'interface montre chaque commande ; le garde
            // de lecture seule refuse de toute façon chaque écriture.
            role: Role::Admin,
            password_hash: Some(&hash),
            oidc: None,
        },
    )
    .await?
    .context("demo account already exists in a fresh database")?;

    db::alerts::seed_builtin_rules(pool).await?;

    let mut targets = Vec::new();
    for device in devices {
        let input = TargetInput {
            name: device.name.to_string(),
            address: device.address.clone(),
            kind: device.kind.to_string(),
            profile_id: None,
            parent_id: None,
            via_agent: None,
            interval: Duration::from_secs(device.interval_secs),
            enabled: true,
            tags: device.tags.iter().map(|(k, v)| (k.to_string(), v.clone())).collect(),
            credential: Some(device.credential.clone()),
            group_name: String::new(),
        };
        let id = db::targets::create(pool, cipher, &input).await?;
        targets.push((id, device.name.to_string(), device.kind.to_string()));
    }
    for (name, kind, address, parent) in CHECKS {
        let parent_id = parent
            .and_then(|parent| targets.iter().find(|(_, name, _)| name == parent))
            .map(|(id, _, _)| *id);
        let input = TargetInput {
            name: name.to_string(),
            address: address.to_string(),
            kind: kind.to_string(),
            profile_id: None,
            parent_id,
            via_agent: None,
            interval: Duration::from_secs(60),
            enabled: true,
            tags: BTreeMap::new(),
            credential: Some(Credential::None),
            group_name: String::new(),
        };
        let id = db::targets::create(pool, cipher, &input).await?;
        targets.push((id, name.to_string(), kind.to_string()));
    }

    // L'agent s'enregistre lui-même au premier lot, comme sur une vraie machine.
    let (_, agent_token) = agent::create_token(
        pool,
        "docker01 (demo)",
        TokenPolicy { max_uses: None, expires_in_days: None },
    )
    .await?;
    let agent_id = push_agent_batch(pool, cipher, sink, &agent_token).await?;
    targets.push((agent_id, AGENT_HOST.to_string(), "agent".to_string()));

    seed_notifications(pool).await?;
    seed_status_page(pool, &targets).await?;
    seed_alert_history(pool, &targets).await?;

    Ok(Seeded { agent_token, targets })
}

/// Un lot de l'agent simulé, par le vrai chemin d'ingestion.
pub async fn push_agent_batch(
    pool: &SqlitePool,
    cipher: &Cipher,
    sink: &SampleSink,
    token: &str,
) -> Result<TargetId> {
    let now = Utc::now().timestamp_millis();
    let identity = AgentIdentity {
        hostname: AGENT_HOST.to_string(),
        os: "linux".to_string(),
        os_version: Some("Debian GNU/Linux 13 (trixie)".to_string()),
        kernel_version: Some("6.12.48-amd64".to_string()),
        arch: Some("x86_64".to_string()),
        agent_version: env!("CARGO_PKG_VERSION").to_string(),
        commands_enabled: Some(false),
        relay: false,
        site: None,
        machine_id: Some("demo-docker01".to_string()),
        tags: BTreeMap::new(),
        binding_supported: false,
    };
    let batch = PushBatch {
        protocol: PUSH_PROTOCOL_VERSION,
        identity,
        sent_at_ms: now,
        samples: synthetic::agent_samples(now),
    };
    let bearer = format!("Bearer {token}");
    let ack = agent::ingest(pool, cipher, sink, Some(&bearer), None, batch)
        .await
        .map_err(|error| anyhow::anyhow!("demo agent push refused: {error:?}"))?;
    Ok(ack.target_id)
}

/// Deux canaux pour que la page des notifications ne soit pas vide. Ils ne
/// partiront jamais : les envois sont coupés pour tout le processus.
async fn seed_notifications(pool: &SqlitePool) -> Result<()> {
    for (name, kind, settings) in [
        (
            "Phone (ntfy)",
            "ntfy",
            r#"{"server_url":"https://ntfy.home.arpa","topic":"homelab-alerts"}"#,
        ),
        (
            "Family chat (webhook)",
            "webhook",
            r#"{"url":"https://chat.home.arpa/hooks/monitoring"}"#,
        ),
    ] {
        sqlx::query(
            "INSERT INTO notification_channels (name, kind, enabled, settings) VALUES (?, ?, 1, ?)",
        )
        .bind(name)
        .bind(kind)
        .bind(settings)
        .execute(pool)
        .await
        .with_context(|| format!("demo channel {name}"))?;
    }
    Ok(())
}

async fn seed_status_page(pool: &SqlitePool, targets: &[(TargetId, String, String)]) -> Result<()> {
    let page = db::status_pages::create_page(
        pool,
        &StatusPageInput {
            slug: "home".to_string(),
            title: "Home services".to_string(),
            description: "What the family can check before asking whether the internet is down."
                .to_string(),
            published: true,
            theme: "auto".to_string(),
            show_uptime_days: 30,
            accent: "default".to_string(),
            footer_text: "Fictional data — DumbMonit live demo.".to_string(),
            homepage_url: String::new(),
            subscribe_channel_id: None,
            link_origin: String::new(),
        },
    )
    .await?;

    let find = |name: &str| targets.iter().find(|(_, n, _)| n == name).map(|(id, _, _)| *id);
    let mut items = Vec::new();
    for (name, label, group) in [
        ("nextcloud.home.arpa", "Nextcloud", "Apps"),
        ("home-assistant.home.arpa", "Home Assistant", "Apps"),
        (FLAKY_HOST, "Jellyfin", "Apps"),
        ("printer.home.arpa", "Printer", "Network"),
    ] {
        if let Some(target_id) = find(name) {
            items.push(PageItemInput {
                target_id,
                label: label.to_string(),
                group_name: group.to_string(),
            });
        }
    }
    // Les équipements simulés ([`super::estate`]) : le NAS et l'hyperviseur.
    for (kind, label) in [("synology", "File storage"), ("proxmox", "Virtual machines")] {
        if let Some((target_id, _, _)) = targets.iter().find(|(_, _, k)| k == kind) {
            items.push(PageItemInput {
                target_id: *target_id,
                label: label.to_string(),
                group_name: "Infrastructure".to_string(),
            });
        }
    }
    db::status_pages::set_items(pool, page, &items).await?;

    // Une maintenance passée, close : l'historique de la page n'est pas vide.
    let day = hour_floor(Utc::now()) - chrono::Duration::days(4);
    let start = day.with_hour(6).unwrap_or(day);
    let incident = db::status_pages::create_incident(
        pool,
        &IncidentInput {
            page_id: Some(page),
            title: "NAS firmware update".to_string(),
            kind: "maintenance".to_string(),
            status: "completed".to_string(),
            severity: "minor".to_string(),
            starts_at: stamp(start),
            ends_at: Some(stamp(start + chrono::Duration::minutes(40))),
        },
    )
    .await?;
    db::status_pages::add_update(
        pool,
        incident,
        "completed",
        "DSM updated and rebooted; shares were back after 12 minutes.",
        true,
    )
    .await?;
    Ok(())
}

/// Alerte passée : règle, cible, gravité, début, fin, valeur.
type PastAlert = (&'static str, Option<TargetId>, &'static str, DateTime<Utc>, DateTime<Utc>, f64);

/// Quelques alertes passées, résolues, pour que l'historique raconte une
/// semaine ordinaire. Les pannes de Jellyfin suivent le calendrier des sondes
/// synthétiques : l'historique et les courbes disent la même chose.
async fn seed_alert_history(
    pool: &SqlitePool,
    targets: &[(TargetId, String, String)],
) -> Result<()> {
    let find = |name: &str| targets.iter().find(|(_, n, _)| n == name).map(|(id, _, _)| *id);
    let now = Utc::now();
    let now_h = now.timestamp() / 3600;
    let mut events: Vec<PastAlert> = Vec::new();

    if let Some(jellyfin) = find(FLAKY_HOST) {
        for hour in (now_h - 7 * 24)..now_h {
            if hour % 97 == 0 {
                let start = DateTime::from_timestamp(hour * 3600 + 180, 0).unwrap_or(now);
                let end = DateTime::from_timestamp(hour * 3600 + 3660, 0).unwrap_or(now);
                events.push(("service_down", Some(jellyfin), "critical", start, end, 0.0));
            }
        }
    }
    if let Some(agent) = find(AGENT_HOST) {
        let start = hour_floor(now) - chrono::Duration::hours(53);
        events.push((
            "cpu_high",
            Some(agent),
            "warning",
            start,
            start + chrono::Duration::minutes(25),
            94.0,
        ));
    }

    for (index, (rule, target, severity, start, end, value)) in events.into_iter().enumerate() {
        let fingerprint = format!("demo-history-{index}");
        for (from, to, at) in [("pending", "firing", start), ("firing", "resolved", end)] {
            sqlx::query(
                "INSERT INTO alert_history
                     (fingerprint, rule_uid, target_id, from_phase, to_phase, severity, value,
                      notified, reason, at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, 0, '', ?)",
            )
            .bind(&fingerprint)
            .bind(rule)
            .bind(target)
            .bind(from)
            .bind(to)
            .bind(severity)
            .bind(value)
            .bind(stamp(at))
            .execute(pool)
            .await
            .context("demo alert history")?;
        }
    }
    Ok(())
}

fn hour_floor(at: DateTime<Utc>) -> DateTime<Utc> {
    DateTime::from_timestamp(at.timestamp() / 3600 * 3600, 0).unwrap_or(at)
}

/// Format des horodatages serveur : UTC sans suffixe.
fn stamp(at: DateTime<Utc>) -> String {
    at.format("%Y-%m-%d %H:%M:%S").to_string()
}
