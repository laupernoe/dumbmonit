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
use super::extras;
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
///
/// Les types de [`extras`] (commutateur, CrowdSec, Cloudflare, licence) sont
/// simulés en mémoire ; leurs domaines sont ajoutés à part, plus bas.
const CHECKS: &[(&str, &str, &str, Option<&str>)] = &[
    ("nextcloud.home.arpa", "http", "https://nextcloud.home.arpa/status.php", None),
    ("home-assistant.home.arpa", "http", "https://home-assistant.home.arpa/", None),
    (FLAKY_HOST, "http", "https://jellyfin.home.arpa/health", None),
    ("vault.home.arpa", "tls", "vault.home.arpa:443", None),
    ("nextcloud.home.arpa (TLS)", "tls", "nextcloud.home.arpa:443", None),
    ("printer.home.arpa", "ping", "printer.home.arpa", None),
    ("switch-attic.home.arpa", "ping", "switch-attic.home.arpa", None),
    ("ap-attic.home.arpa", "ping", "ap-attic.home.arpa", Some("switch-attic.home.arpa")),
    (extras::SWITCH_HOST, "snmp", "switch-core.home.arpa", None),
    (extras::CROWDSEC_HOST, "crowdsec", "http://crowdsec.home.arpa:8080", None),
    (extras::CLOUDFLARE_HOST, "cloudflare", "api.cloudflare.com", None),
    (extras::VEEAM_HOST, "veeam", "https://veeam.home.arpa:9419", None),
];

/// Ce que la suite du démarrage doit connaître du parc.
pub struct Seeded {
    /// Jeton d'enregistrement de l'agent simulé, en clair : il ne quitte pas le
    /// processus.
    pub agent_token: String,
    /// Secret de liaison remis à l'agent simulé à son premier lot : sans lui, le
    /// serveur refuserait les lots suivants, comme pour une vraie machine.
    pub agent_secret: Option<String>,
    pub targets: Vec<(TargetId, String, String)>,
}

pub async fn run(
    pool: &SqlitePool,
    cipher: &Cipher,
    sink: &SampleSink,
    devices: &[DemoDevice],
) -> Result<Seeded> {
    // L'origine des tendances relatives (remplissage, échéances) est posée avant
    // toute mesure, en direct comme dans l'historique.
    synthetic::anchor_ms();
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
    // La note de sécurité est facultative ; la démonstration la montre.
    crate::security::set_setting_enabled(pool, true).await?;

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
        let id = add_check(pool, cipher, name, kind, address, parent_id, 60).await?;
        targets.push((id, name.to_string(), kind.to_string()));
    }
    // Les noms de domaine surveillés : une consultation RDAP par heure.
    for (domain, _, _) in extras::DOMAINS {
        let id = add_check(pool, cipher, domain, "domain", domain, None, 3600).await?;
        targets.push((id, domain.to_string(), "domain".to_string()));
    }

    // L'agent s'enregistre lui-même au premier lot, comme sur une vraie machine.
    let (_, agent_token) = agent::create_token(
        pool,
        "docker01 (demo)",
        TokenPolicy { max_uses: None, expires_in_days: None },
    )
    .await?;
    let (agent_id, agent_secret) = push_agent_batch(pool, cipher, sink, &agent_token, None).await?;
    targets.push((agent_id, AGENT_HOST.to_string(), "agent".to_string()));

    seed_notifications(pool).await?;
    seed_status_pages(pool, &targets).await?;
    seed_alert_history(pool, &targets).await?;
    seed_network_inventory(pool, agent_id).await?;
    // Un emplacement, le site principal : la carte du monde et la carte en
    // direct du pare-feu ont de quoi se dessiner.
    sqlx::query(
        "INSERT INTO locations (name, address, latitude, longitude, main_site)
         VALUES ('Home', 'Lausanne, Switzerland', 46.5197, 6.6323, 1)",
    )
    .execute(pool)
    .await?;

    Ok(Seeded { agent_token, agent_secret, targets })
}

/// Crée une sonde synthétique : aucun identifiant, aucune connexion sortante.
async fn add_check(
    pool: &SqlitePool,
    cipher: &Cipher,
    name: &str,
    kind: &str,
    address: &str,
    parent_id: Option<TargetId>,
    interval_secs: u64,
) -> Result<TargetId> {
    let input = TargetInput {
        name: name.to_string(),
        address: address.to_string(),
        kind: kind.to_string(),
        profile_id: None,
        parent_id,
        via_agent: None,
        interval: Duration::from_secs(interval_secs),
        enabled: true,
        tags: BTreeMap::new(),
        credential: Some(Credential::None),
        group_name: String::new(),
    };
    db::targets::create(pool, cipher, &input).await
}

/// Un lot de l'agent simulé, par le vrai chemin d'ingestion. Rend la cible et
/// le secret de liaison remis au premier lot, s'il y en a un.
pub async fn push_agent_batch(
    pool: &SqlitePool,
    cipher: &Cipher,
    sink: &SampleSink,
    token: &str,
    secret: Option<&str>,
) -> Result<(TargetId, Option<String>)> {
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
        relay_control: false,
        relay_local: false,
        machine_id: Some("demo-docker01".to_string()),
        tags: BTreeMap::new(),
        binding_supported: true,
        self_update: None,
        relay_gateway: None,
        remediation: None,
    };
    let batch = PushBatch {
        protocol: PUSH_PROTOCOL_VERSION,
        identity,
        sent_at_ms: now,
        samples: synthetic::agent_samples(now),
        inventory: None,
        event_log: None,
        declared_monitors: None,
    };
    let bearer = format!("Bearer {token}");
    let ack = agent::ingest(pool, cipher, sink, Some(&bearer), secret, batch)
        .await
        .map_err(|error| anyhow::anyhow!("demo agent push refused: {error:?}"))?;
    Ok((ack.target_id, ack.agent_secret))
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

/// Réglages d'une page de statut de la démonstration.
struct PageSpec {
    slug: &'static str,
    title: &'static str,
    description: &'static str,
    /// Scène de nuit de la bannière (`paris`, `tokyo`…).
    scene: &'static str,
    /// Éléments : (cible par nom, libellé, groupe).
    items: &'static [(&'static str, &'static str, &'static str)],
    /// Types d'équipements simulés ajoutés : (type, libellé, groupe).
    kinds: &'static [(&'static str, &'static str, &'static str)],
}

const PAGES: &[PageSpec] = &[
    PageSpec {
        slug: "home",
        title: "Home services",
        description: "What the family can check before asking whether the internet is down.",
        scene: "paris",
        items: &[
            ("nextcloud.home.arpa", "Nextcloud", "Apps"),
            ("home-assistant.home.arpa", "Home Assistant", "Apps"),
            (FLAKY_HOST, "Jellyfin", "Apps"),
            ("printer.home.arpa", "Printer", "Network"),
        ],
        kinds: &[
            ("synology", "File storage", "Infrastructure"),
            ("proxmox", "Virtual machines", "Infrastructure"),
        ],
    },
    PageSpec {
        slug: "lab",
        title: "Lab and media",
        description: "The servers behind the media library and the nightly backups.",
        scene: "tokyo",
        items: &[
            (FLAKY_HOST, "Media server", "Media"),
            (AGENT_HOST, "Docker host", "Hosts"),
            (extras::SWITCH_HOST, "Core switch", "Network"),
        ],
        kinds: &[("truenas", "Media storage", "Media"), ("pbs", "Backups", "Hosts")],
    },
];

async fn seed_status_pages(
    pool: &SqlitePool,
    targets: &[(TargetId, String, String)],
) -> Result<()> {
    let find = |name: &str| targets.iter().find(|(_, n, _)| n == name).map(|(id, _, _)| *id);
    let mut first = None;
    for spec in PAGES {
        let page = db::status_pages::create_page(
            pool,
            &StatusPageInput {
                slug: spec.slug.to_string(),
                title: spec.title.to_string(),
                description: spec.description.to_string(),
                published: true,
                theme: "auto".to_string(),
                show_uptime_days: 30,
                accent: "default".to_string(),
                // Une scène de nuit par page : Paris, puis Tokyo.
                scenes: spec.scene.to_string(),
                scene_rotation: "visit".to_string(),
                simple: false,
                show_calendar: true,
                show_latency: true,
                footer_text: "Fictional data — DumbMonit live demo.".to_string(),
                homepage_url: String::new(),
                subscribe_channel_id: None,
                link_origin: String::new(),
                // La démonstration n'a pas de domaine à elle : les pages restent sous
                // `/s/<slug>`.
                domain: None,
                seo: Default::default(),
                groups: None,
                group_labels: None,
            },
        )
        .await?;
        if first.is_none() {
            first = Some(page);
        }

        let mut items = Vec::new();
        for (name, label, group) in spec.items {
            if let Some(target_id) = find(name) {
                items.push(PageItemInput {
                    target_id,
                    label: label.to_string(),
                    group_name: group.to_string(),
                });
            }
        }
        // Les équipements simulés ([`super::estate`]) : le NAS, l'hyperviseur…
        for (kind, label, group) in spec.kinds {
            if let Some((target_id, _, _)) = targets.iter().find(|(_, _, k)| k == kind) {
                items.push(PageItemInput {
                    target_id: *target_id,
                    label: label.to_string(),
                    group_name: group.to_string(),
                });
            }
        }
        db::status_pages::set_items(pool, page, &items).await?;
    }

    // Une maintenance passée, close : l'historique de la première page n'est pas vide.
    let Some(page) = first else { return Ok(()) };
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

/// L'inventaire du réseau de la maison : un balayage planifié qui a déjà tourné,
/// des appareils connus, un port inattendu sur l'imprimante et un appareil
/// nouveau, apparu depuis quelques heures. Les listes sont écrites telles quelles :
/// le balayage ne repartira pas avant une semaine, et il ne sortirait de toute
/// façon pas du processus.
async fn seed_network_inventory(pool: &SqlitePool, agent: TargetId) -> Result<()> {
    /// (adresse MAC, IP, nom, fabricant, connu, ports : (numéro, produit, connu)).
    type Host = (&'static str, &'static str, &'static str, &'static str, bool, &'static [Port]);
    type Port = (u16, &'static str, bool);
    #[rustfmt::skip]
    const HOSTS: &[Host] = &[
        ("00:0d:b9:5e:11:a4", "192.168.1.1", "fw.home.arpa", "PC Engines GmbH", true,
            &[(22, "OpenSSH 9.9", true), (443, "nginx", true)]),
        ("a4:bf:01:2c:77:10", "192.168.1.10", "pve.home.arpa", "Intel Corporate", true,
            &[(22, "OpenSSH 9.2", true), (8006, "Proxmox VE", true)]),
        ("00:11:32:aa:20:01", "192.168.1.20", "nas.home.arpa", "Synology Incorporated", true,
            &[(445, "Samba smbd", true), (5000, "Synology DSM", true)]),
        ("ac:1f:6b:03:21:ee", "192.168.1.21", "truenas.home.arpa", "Super Micro Computer", true,
            &[(443, "nginx", true)]),
        ("d8:bb:c1:44:30:0f", "192.168.1.30", "docker01.home.arpa", "Micro-Star INTL", true,
            &[(22, "OpenSSH 9.2", true), (443, "Traefik", true)]),
        ("00:1b:a9:6c:40:12", "192.168.1.40", "printer.home.arpa", "Brother Industries", true,
            &[(80, "Brother HTTP", true), (631, "CUPS", true), (9100, "JetDirect", false)]),
        ("f0:9f:c2:91:50:77", "192.168.1.50", "ap-livingroom", "Ubiquiti Networks", true, &[]),
        ("9c:8e:cd:15:61:2a", "192.168.1.61", "cam-hall", "Amcrest Industries", true,
            &[(554, "RTSP", true)]),
        ("f4:12:fa:3e:9d:b1", "192.168.1.87", "", "Espressif Inc.", false,
            &[(80, "lwIP httpd", false)]),
    ];
    let at = |ago: chrono::Duration| (Utc::now() - ago).format("%Y-%m-%dT%H:%M:%S").to_string();

    let schedule: i64 = sqlx::query_scalar(
        "INSERT INTO netscan_schedules
             (name, cidr, agent_id, interval_secs, depth, enabled, initialized, last_run_at,
              last_status)
         VALUES ('Home LAN', '192.168.1.0/24', ?, 604800, 'standard', 1, 1, ?, 'done')
         RETURNING id",
    )
    .bind(agent)
    .bind(at(chrono::Duration::minutes(8)))
    .fetch_one(pool)
    .await
    .context("demo netscan schedule")?;

    for (mac, ip, hostname, vendor, known, ports) in HOSTS {
        // Les appareils connus sont là depuis le premier balayage, il y a deux
        // semaines ; l'intrus, depuis trois heures.
        let first_seen =
            at(if *known { chrono::Duration::days(14) } else { chrono::Duration::hours(3) });
        let device: i64 = sqlx::query_scalar(
            "INSERT INTO netscan_devices
                 (schedule_id, ident, mac, ip, hostname, vendor, known, first_seen, last_seen)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(schedule)
        .bind(format!("mac:{mac}"))
        .bind(*mac)
        .bind(*ip)
        .bind(Some(*hostname).filter(|name| !name.is_empty()))
        .bind(*vendor)
        .bind(i64::from(*known))
        .bind(&first_seen)
        .bind(at(chrono::Duration::minutes(8)))
        .fetch_one(pool)
        .await
        .context("demo netscan device")?;
        for (port, product, port_known) in *ports {
            // Un port inattendu sur un appareil connu est apparu hier soir.
            let port_seen =
                if *port_known { first_seen.clone() } else { at(chrono::Duration::hours(14)) };
            sqlx::query(
                "INSERT INTO netscan_ports
                     (device_id, port, protocol, product, known, first_seen, last_seen)
                 VALUES (?, ?, 'tcp', ?, ?, ?, ?)",
            )
            .bind(device)
            .bind(i64::from(*port))
            .bind(*product)
            .bind(i64::from(*port_known))
            .bind(port_seen)
            .bind(at(chrono::Duration::minutes(8)))
            .execute(pool)
            .await
            .context("demo netscan port")?;
        }
    }
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
