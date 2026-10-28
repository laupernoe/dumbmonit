//! Le reste du produit, rempli pour la démonstration : ce qui ne vient pas
//! d'une mesure mais d'un réglage ou d'un travail d'équipe — clients (mode
//! MSP), équipes et dossiers, comptes et astreinte, silences, consignes,
//! tableaux de bord, rapports planifiés, sites sur la carte, canaux de
//! notification (jamais utilisés : les envois sont coupés), failles connues.
//!
//! Deux clients :
//!
//! - **Dupont household** (le client 1, la maison) : le parc d'origine ;
//! - **Muller Architectes SA**, un bureau d'architectes de Genève : un
//!   contrôleur de domaine Windows, Veeam, un réseau Omada, Microsoft 365, et
//!   l'agent relais posé dans le bureau qui interroge ce qui ne se voit pas
//!   d'Internet.
//!
//! Tout est fixe et daté relativement au démarrage ; rien ne sort.

use std::collections::BTreeMap;
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use dumbmonit_proto::{Credential, TargetId};
use sqlx::SqlitePool;

use super::estate::{ENTRA_NAME, M365_NAME, OFFICE_DEVICES};
use super::synthetic::{AGENT_HOST, OFFICE_SITE, RELAY_HOST, WINDOWS_HOST};
use crate::auth::clients::MspSettings;
use crate::auth::users::{self, NewUser, Role};
use crate::crypto::Cipher;
use crate::db;
use crate::db::targets::TargetInput;

/// Le client PME, après le client 1 (la maison).
pub const OFFICE_CLIENT: &str = "Muller Architectes SA";
const HOME_CLIENT: &str = "Dupont household";

/// Sondes du bureau, confiées au relais : (nom, type, adresse).
pub const OFFICE_CHECKS: &[(&str, &str, &str)] = &[
    ("intranet.muller.home.arpa", "http", "https://intranet.muller.home.arpa/"),
    ("nas.muller.home.arpa", "ping", "nas.muller.home.arpa"),
    ("printer.muller.home.arpa", "ping", "printer.muller.home.arpa"),
    ("muller-archi.ch (TLS)", "tls", "muller-archi.ch:443"),
];

/// Dossiers de la liste des équipements : (dossier, noms).
const FOLDERS: &[(&str, &[&str])] = &[
    (
        "Infrastructure",
        &[
            "pve.home.arpa",
            "nas.home.arpa",
            "pbs.home.arpa",
            "truenas.home.arpa",
            "bmc-r740.home.arpa",
            "kopia.home.arpa",
            AGENT_HOST,
        ],
    ),
    (
        "Network",
        &[
            "fw.home.arpa",
            "unifi.home.arpa",
            "switch-core.home.arpa",
            "switch-attic.home.arpa",
            "ap-attic.home.arpa",
            "printer.home.arpa",
            "nextcloud.home.arpa (DNS)",
        ],
    ),
    (
        "Apps",
        &[
            "nextcloud.home.arpa",
            "home-assistant.home.arpa",
            "jellyfin.home.arpa",
            "vault.home.arpa",
            "nextcloud.home.arpa (TLS)",
        ],
    ),
    (
        "Internet",
        &[
            "crowdsec.home.arpa",
            "Cloudflare edge",
            "dupont.ch",
            "dupont-photos.net",
            "homelab-notes.eu",
            "dupont.ch (DNS)",
        ],
    ),
    (
        "Geneva office",
        &[
            WINDOWS_HOST,
            RELAY_HOST,
            "vbr01.muller.home.arpa",
            "omada.muller.home.arpa",
            "intranet.muller.home.arpa",
            "nas.muller.home.arpa",
            "printer.muller.home.arpa",
            "muller-archi.ch (TLS)",
        ],
    ),
    ("Microsoft 365", &[M365_NAME, ENTRA_NAME]),
];

/// Remplit tout ce qui n'est pas une mesure. `targets` reçoit les sondes du
/// bureau, que l'historique rétroactif doit connaître.
pub async fn run(
    pool: &SqlitePool,
    cipher: &Cipher,
    targets: &mut Vec<(TargetId, String, String)>,
) -> Result<()> {
    let find = |targets: &[(TargetId, String, String)], name: &str| {
        targets.iter().find(|(_, n, _)| n == name).map(|(id, _, _)| *id)
    };
    let relay = find(targets.as_slice(), RELAY_HOST).context("demo relay agent")?;

    // Les sondes du bureau passent par le relais : le serveur ne voit pas ce
    // réseau-là.
    for (name, kind, address) in OFFICE_CHECKS {
        let input = TargetInput {
            name: name.to_string(),
            address: address.to_string(),
            kind: kind.to_string(),
            profile_id: None,
            parent_id: None,
            via_agent: Some(relay),
            interval: Duration::from_secs(60),
            enabled: true,
            tags: BTreeMap::new(),
            credential: Some(Credential::None),
            group_name: String::new(),
        };
        let id = db::targets::create(pool, cipher, &input).await?;
        targets.push((id, name.to_string(), kind.to_string()));
    }
    for name in OFFICE_DEVICES {
        sqlx::query("UPDATE targets SET via_agent = ? WHERE name = ?")
            .bind(relay)
            .bind(*name)
            .execute(pool)
            .await?;
    }
    for (folder, names) in FOLDERS {
        for name in *names {
            sqlx::query("UPDATE targets SET group_name = ? WHERE name = ?")
                .bind(*folder)
                .bind(*name)
                .execute(pool)
                .await?;
        }
    }

    let office = seed_clients(pool).await?;
    let people = seed_people(pool, office).await?;
    let channels = seed_channels(pool).await?;
    seed_locations(pool).await?;
    seed_silences(pool, targets.as_slice()).await?;
    seed_oncall(pool, &people, &channels, office).await?;
    seed_runbooks(pool, office).await?;
    seed_dashboards(pool).await?;
    seed_reports(pool, &channels, office).await?;
    seed_vulnerabilities(pool, targets.as_slice()).await?;
    if let Some(docker) = find(targets.as_slice(), AGENT_HOST) {
        // Une machine qui chauffe un peu tous les soirs : son seuil est relevé.
        sqlx::query(
            "INSERT INTO rule_overrides (rule_uid, target_id, threshold, clear_threshold, enabled)
             VALUES ('cpu_high', ?, 92, 85, NULL)",
        )
        .bind(docker)
        .execute(pool)
        .await
        .context("demo rule override")?;
    }
    Ok(())
}

fn stamp(at: DateTime<Utc>) -> String {
    at.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Mode MSP : la maison et le bureau, chacun ses équipements.
async fn seed_clients(pool: &SqlitePool) -> Result<i64> {
    crate::auth::clients::save_settings(pool, &MspSettings { enabled: true }).await?;
    sqlx::query(
        "UPDATE clients SET name = ?, description = ?, contacts = ?, report_timezone = ?
         WHERE id = 1",
    )
    .bind(HOME_CLIENT)
    .bind("The family homelab: hypervisor, storage, media and home automation.")
    .bind(r#"["noe@dupont.ch"]"#)
    .bind("Europe/Zurich")
    .execute(pool)
    .await
    .context("demo home client")?;
    let office: i64 = sqlx::query_scalar(
        "INSERT INTO clients (name, description, contacts, monitor_quota, sla_report,
                              sla_objective, report_timezone)
         VALUES (?, ?, ?, 40, 1, 99.5, 'Europe/Zurich') RETURNING id",
    )
    .bind(OFFICE_CLIENT)
    .bind("Architecture firm in Geneva, 14 people. Managed since 2024.")
    .bind(r#"["it@muller-archi.ch","direction@muller-archi.ch"]"#)
    .fetch_one(pool)
    .await
    .context("demo office client")?;
    for (_, names) in FOLDERS
        .iter()
        .filter(|(folder, _)| *folder == "Geneva office" || *folder == "Microsoft 365")
    {
        for name in *names {
            sqlx::query("UPDATE targets SET client_id = ? WHERE name = ?")
                .bind(office)
                .bind(*name)
                .execute(pool)
                .await?;
        }
    }
    Ok(office)
}

/// Les comptes de la démonstration, en plus du visiteur : identifiés par leur
/// nom d'utilisateur. Aucun n'a de mot de passe : on ne peut pas s'y connecter.
struct People {
    noe: i64,
    sam: i64,
    claire: i64,
}

async fn seed_people(pool: &SqlitePool, office: i64) -> Result<People> {
    let mut ids = Vec::new();
    for (username, display_name, role) in [
        ("noe", "Noé Dupont", Role::Admin),
        ("sam", "Sam Rochat", Role::Operator),
        ("claire.muller", "Claire Muller", Role::Viewer),
    ] {
        let id = users::insert(
            pool,
            NewUser { username, display_name, role, password_hash: None, oidc: None },
        )
        .await?
        .with_context(|| format!("demo account {username}"))?;
        ids.push(id);
    }
    let people = People { noe: ids[0], sam: ids[1], claire: ids[2] };
    // Claire ne voit que son bureau.
    sqlx::query("INSERT OR IGNORE INTO client_members (client_id, user_id) VALUES (?, ?)")
        .bind(office)
        .bind(people.claire)
        .execute(pool)
        .await?;
    // Une équipe réseau, qui opère les dossiers réseau des deux clients.
    let team: i64 = sqlx::query_scalar(
        "INSERT INTO teams (name, description) VALUES ('Network', ?) RETURNING id",
    )
    .bind("Firewalls, switches and Wi-Fi, at home and at the office.")
    .fetch_one(pool)
    .await?;
    sqlx::query("INSERT INTO team_members (team_id, user_id) VALUES (?, ?)")
        .bind(team)
        .bind(people.sam)
        .execute(pool)
        .await?;
    for (folder, level) in [("Network", "operate"), ("Geneva office", "read")] {
        sqlx::query("INSERT INTO team_folder_grants (team_id, folder, level) VALUES (?, ?, ?)")
            .bind(team)
            .bind(folder)
            .bind(level)
            .execute(pool)
            .await?;
    }
    Ok(people)
}

/// Identifiants des canaux semés ici.
struct Channels {
    email: i64,
    teams: i64,
}

/// Canaux en plus des deux du semis : courriel, Teams, Telegram, Gotify,
/// PagerDuty. Aucun secret n'est rangé, et rien ne partira jamais.
async fn seed_channels(pool: &SqlitePool) -> Result<Channels> {
    let mut ids = Vec::new();
    for (name, kind, settings) in [
        (
            "Email (Dupont)",
            "smtp",
            r#"{"host":"mail.home.arpa","security":"starttls","port":587,"from":"monitoring@dupont.ch","to":["noe@dupont.ch"]}"#,
        ),
        ("Muller IT (Teams)", "teams", "{}"),
        ("Night shift (Telegram)", "telegram", r#"{"chat_id":"-1002345678901"}"#),
        ("Gotify (home)", "gotify", r#"{"server_url":"https://gotify.home.arpa"}"#),
        ("PagerDuty (MSP)", "pagerduty", r#"{"region":"eu","source":"dumbmonit-demo"}"#),
    ] {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO notification_channels (name, kind, enabled, settings)
             VALUES (?, ?, 1, ?) RETURNING id",
        )
        .bind(name)
        .bind(kind)
        .bind(settings)
        .fetch_one(pool)
        .await
        .with_context(|| format!("demo channel {name}"))?;
        ids.push(id);
    }
    Ok(Channels { email: ids[0], teams: ids[1] })
}

/// Le bureau de Genève sur la carte du monde : le site déclaré par le relais
/// et le contrôleur de domaine. La maison (Lausanne) est le site principal.
async fn seed_locations(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "INSERT INTO locations (name, address, latitude, longitude, site, main_site)
         VALUES ('Geneva office', 'Rue du Rhône 42, Geneva, Switzerland', 46.2035, 6.1466, ?, 0)",
    )
    .bind(OFFICE_SITE)
    .execute(pool)
    .await
    .context("demo office location")?;
    sqlx::query(
        "INSERT INTO locations (name, address, latitude, longitude, site, main_site)
         VALUES ('Offsite backups', 'Equinix ZH4, Zurich, Switzerland', 47.4241, 8.5525, '', 0)",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "UPDATE targets SET location_id = (SELECT id FROM locations WHERE name = 'Geneva office')
         WHERE client_id = (SELECT id FROM clients WHERE name = ?)",
    )
    .bind(OFFICE_CLIENT)
    .execute(pool)
    .await?;
    sqlx::query(
        "UPDATE targets SET location_id = (SELECT id FROM locations WHERE name = 'Offsite backups')
         WHERE name = 'pbs.home.arpa'",
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Un silence hebdomadaire (le nettoyage du NAS le dimanche) et un silence
/// ponctuel en cours (l'imprimante du bureau en réparation).
async fn seed_silences(pool: &SqlitePool, targets: &[(TargetId, String, String)]) -> Result<()> {
    let find = |name: &str| targets.iter().find(|(_, n, _)| n == name).map(|(id, _, _)| *id);
    let now = Utc::now();
    let ends = now + chrono::Duration::hours(3);
    let rfc = |at: DateTime<Utc>| at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let silences = [
        (
            "Sunday scrub",
            "The weekly data scrub slows the NAS down: no latency alerts meanwhile.",
            find("nas.home.arpa"),
            r#"{"kind":"weekly","days":[6],"start_minute":120,"end_minute":300,"utc_offset_minutes":0,"timezone":"Europe/Zurich"}"#
                .to_string(),
        ),
        (
            "Printer repair",
            "Technician on site until noon, the printer is unplugged.",
            find("printer.muller.home.arpa"),
            format!(
                r#"{{"kind":"once","starts_at":"{}","ends_at":"{}"}}"#,
                rfc(now - chrono::Duration::hours(1)),
                rfc(ends)
            ),
        ),
    ];
    for (name, comment, target, schedule) in silences {
        sqlx::query(
            "INSERT INTO silences (name, comment, target_id, matchers, schedule, enabled)
             VALUES (?, ?, ?, '{}', ?, 1)",
        )
        .bind(name)
        .bind(comment)
        .bind(target)
        .bind(schedule)
        .execute(pool)
        .await
        .context("demo silence")?;
    }
    Ok(())
}

/// Astreinte : une rotation hebdomadaire à la maison (Noé et Sam), les heures
/// de bureau du client PME, une politique d'escalade et un remplacement.
async fn seed_oncall(
    pool: &SqlitePool,
    people: &People,
    channels: &Channels,
    office: i64,
) -> Result<()> {
    // La relève a eu lieu lundi dernier à 9 h, heure de Zurich.
    let monday = {
        let today = Utc::now().date_naive();
        let back = chrono::Datelike::weekday(&today).num_days_from_monday();
        today - chrono::Duration::days(i64::from(back) + 7)
    };
    let start = format!("{}T09:00", monday.format("%Y-%m-%d"));
    let home = format!(
        r#"{{"timezone":"Europe/Zurich","handoff_notifications":true,"layers":[{{"name":"Week","rotation":{{"kind":"weekly","start":"{start}","length":1,"unit":"weeks"}},"participants":[{},{}],"restrictions":[]}}]}}"#,
        people.noe, people.sam
    );
    let office_hours = format!(
        r#"{{"timezone":"Europe/Zurich","handoff_notifications":false,"layers":[{{"name":"Office hours","rotation":{{"kind":"daily","start":"{start}","length":1,"unit":"days"}},"participants":[{}],"restrictions":[{{"days":[0,1,2,3,4],"start_minute":480,"end_minute":1080}}]}}]}}"#,
        people.sam
    );
    let home_id: i64 = sqlx::query_scalar(
        "INSERT INTO oncall_schedules (name, description, definition) VALUES (?, ?, ?)
         RETURNING id",
    )
    .bind("Homelab")
    .bind("One week each, handoff on Monday at 9:00.")
    .bind(&home)
    .fetch_one(pool)
    .await
    .context("demo on-call schedule")?;
    let office_id: i64 = sqlx::query_scalar(
        "INSERT INTO oncall_schedules (name, description, definition, client_id)
         VALUES (?, ?, ?, ?) RETURNING id",
    )
    .bind("Muller office hours")
    .bind("Weekdays 8:00 to 18:00, as in the service contract.")
    .bind(&office_hours)
    .bind(office)
    .fetch_one(pool)
    .await?;
    // Sam reprend la garde de Noé ce week-end.
    let saturday = Utc::now() + chrono::Duration::days(2);
    sqlx::query(
        "INSERT INTO oncall_overrides (schedule_id, user_id, replaces_user_id, starts_at, ends_at,
                                       note, created_by)
         VALUES (?, ?, ?, ?, ?, 'Noé away for the weekend', 'noe')",
    )
    .bind(home_id)
    .bind(people.sam)
    .bind(people.noe)
    .bind(saturday.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
    .bind((saturday + chrono::Duration::days(1)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
    .execute(pool)
    .await
    .context("demo on-call override")?;
    let policy = format!(
        r#"{{"repeat":1,"steps":[{{"delay_minutes":0,"targets":[{{"type":"schedule","id":{home_id}}}]}},{{"delay_minutes":15,"targets":[{{"type":"user","id":{}}},{{"type":"channel","id":{}}}]}}]}}"#,
        people.noe, channels.email
    );
    sqlx::query("INSERT INTO oncall_policies (name, description, definition) VALUES (?, ?, ?)")
        .bind("Critical at home")
        .bind("The person on call, then Noé and the family mailbox after 15 minutes.")
        .bind(policy)
        .execute(pool)
        .await
        .context("demo escalation policy")?;
    let office_policy = format!(
        r#"{{"repeat":0,"steps":[{{"delay_minutes":0,"targets":[{{"type":"schedule","id":{office_id}}},{{"type":"channel","id":{}}}]}}]}}"#,
        channels.teams
    );
    sqlx::query(
        "INSERT INTO oncall_policies (name, description, definition, client_id) VALUES (?, ?, ?, ?)",
    )
    .bind("Muller office")
    .bind("The office-hours rota and the Muller IT channel in Teams.")
    .bind(office_policy)
    .bind(office)
    .execute(pool)
    .await?;
    Ok(())
}

/// Une consigne : (titre, texte, liens, client).
type Runbook = (&'static str, &'static str, &'static [(&'static str, &'static str)], Option<i64>);

/// Consignes : ce qu'on fait quand telle alerte sonne.
async fn seed_runbooks(pool: &SqlitePool, office: i64) -> Result<()> {
    let runbooks: [Runbook; 4] = [
        (
            "A web service is down",
            "1. Open the monitor: is it the service or the host?\n\
             2. On docker01: `docker compose ps` in /srv/stacks/<app>.\n\
             3. Restart the container, then watch the probe turn green.\n\
             4. Still down after 10 minutes: post an incident on the Home services page.",
            &[("rule", "service_down"), ("kind", "http")],
            None,
        ),
        (
            "NAS: volume or disk warning",
            "1. DSM → Storage Manager: which drive, which volume?\n\
             2. A drive flagged *crashed*: replace it from the spare shelf (WD Red Plus 8 TB).\n\
             3. Let the repair finish before any reboot.\n\
             4. Check that the nightly Hyper Backup ran afterwards.",
            &[("kind", "synology"), ("kind", "truenas")],
            None,
        ),
        (
            "Network: switch or access point unreachable",
            "1. Several devices behind the same switch down at once: start with the switch.\n\
             2. Check the PoE budget on switch-core (Gi9 to Gi12).\n\
             3. Power-cycle the device from the switch port, not at the wall.",
            &[("group", "Network")],
            None,
        ),
        (
            "Muller: backup job failed",
            "1. Veeam console on VBR01 → History: read the error of the last session.\n\
             2. Retry the job once. A second failure: call Claire Muller before 18:00.\n\
             3. Note the incident in the monthly SLA report.",
            &[("kind", "veeam")],
            Some(office),
        ),
    ];
    for (title, body, links, client) in runbooks {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO runbooks (title, body, updated_by, client_id) VALUES (?, ?, 'noe', ?)
             RETURNING id",
        )
        .bind(title)
        .bind(body)
        .bind(client)
        .fetch_one(pool)
        .await
        .context("demo runbook")?;
        for (scope, value) in links {
            sqlx::query("INSERT INTO runbook_links (runbook_id, scope, value) VALUES (?, ?, ?)")
                .bind(id)
                .bind(*scope)
                .bind(*value)
                .execute(pool)
                .await?;
        }
    }
    Ok(())
}

/// Deux tableaux de bord : celui du salon, partagé, et un tableau réseau.
async fn seed_dashboards(pool: &SqlitePool) -> Result<()> {
    let homelab = r#"[
 {"id":"w1","type":"stat","title":"Docker host CPU","x":0,"y":0,"w":3,"h":2,"config":{"query":"dumbmonit_cpu_usage_percent{host=\"docker01.home.arpa\"}","unit":"%","warn":70,"crit":90,"direction":"above"}},
 {"id":"w2","type":"stat","title":"Data volume","x":3,"y":0,"w":3,"h":2,"config":{"query":"dumbmonit_filesystem_used_percent{mountpoint=\"/srv/data\"}","unit":"%","warn":85,"crit":95,"direction":"above"}},
 {"id":"w3","type":"stat","title":"Wi-Fi clients","x":6,"y":0,"w":3,"h":2,"config":{"query":"sum(dumbmonit_unifi_clients)","unit":"","direction":"above"}},
 {"id":"w4","type":"alerts","title":"Active alerts","x":9,"y":0,"w":3,"h":4,"config":{"limit":8}},
 {"id":"w5","type":"chart","title":"Web response time","x":0,"y":2,"w":9,"h":4,"config":{"query":"dumbmonit_probe_http_first_byte_seconds","unit":"s"}},
 {"id":"w6","type":"chart","title":"Internet uplink (switch-core Gi1)","x":0,"y":6,"w":6,"h":4,"config":{"query":"rate(dumbmonit_if_octets_in{ifname=\"Gi1\"}[5m]) * 8","unit":"bps"}},
 {"id":"w7","type":"chart","title":"Container memory","x":6,"y":6,"w":6,"h":4,"config":{"query":"dumbmonit_container_memory_bytes","unit":"B"}},
 {"id":"w8","type":"group","title":"Apps","x":0,"y":10,"w":6,"h":3,"config":{"folder":"Apps"}},
 {"id":"w9","type":"text","title":"Notes","x":6,"y":10,"w":6,"h":3,"config":{"markdown":"**Maintenance**: NAS scrub every Sunday 02:00-05:00.\nPhotos backup to PBS every night at 03:00."}}
]"#;
    let network = r#"[
 {"id":"n1","type":"chart","title":"Switch ports in","x":0,"y":0,"w":6,"h":4,"config":{"query":"rate(dumbmonit_if_octets_in{host=\"switch-core.home.arpa\"}[5m]) * 8","unit":"bps"}},
 {"id":"n2","type":"chart","title":"Switch ports out","x":6,"y":0,"w":6,"h":4,"config":{"query":"rate(dumbmonit_if_octets_out{host=\"switch-core.home.arpa\"}[5m]) * 8","unit":"bps"}},
 {"id":"n3","type":"stat","title":"PoE used","x":0,"y":4,"w":4,"h":2,"config":{"query":"dumbmonit_poe_power_consumed_watts","unit":"W","warn":90,"crit":110,"direction":"above"}},
 {"id":"n4","type":"chart","title":"Ping round trip","x":4,"y":4,"w":8,"h":4,"config":{"query":"dumbmonit_probe_icmp_rtt_seconds","unit":"s"}},
 {"id":"n5","type":"group","title":"Network","x":0,"y":8,"w":12,"h":3,"config":{"folder":"Network"}}
]"#;
    for (name, shared, range, widgets) in
        [("Living room", 1, 86_400, homelab), ("Network", 0, 21_600, network)]
    {
        // Relus et validés comme ceux de l'API : un tableau mal formé ferait
        // échouer le démarrage ici plutôt que dans l'interface.
        let parsed: Vec<crate::api::dashboards::Widget> =
            serde_json::from_str(widgets).context("demo dashboard widgets")?;
        let cleaned = crate::api::dashboards::clean_widgets(parsed)
            .map_err(|_| anyhow::anyhow!("demo dashboard {name}: invalid widgets"))?;
        sqlx::query(
            "INSERT INTO dashboards (name, shared, owner_id, range_secs, refresh_secs, widgets)
             VALUES (?, ?, 1, ?, 30, ?)",
        )
        .bind(name)
        .bind(shared)
        .bind(range)
        .bind(serde_json::to_string(&cleaned)?)
        .execute(pool)
        .await
        .context("demo dashboard")?;
    }
    Ok(())
}

/// Un résumé hebdomadaire pour la maison, le rapport SLA mensuel du bureau.
async fn seed_reports(pool: &SqlitePool, channels: &Channels, office: i64) -> Result<()> {
    let armed = (Utc::now() - chrono::Duration::days(20))
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let last = (Utc::now() - chrono::Duration::days(3))
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    sqlx::query(
        "INSERT INTO report_schedules (name, enabled, frequency, weekday, hour, timezone,
             recipients, channel_id, armed_at, last_sent_at, kind)
         VALUES ('Weekly homelab digest', 1, 'weekly', 0, 8, 'Europe/Zurich', ?, ?, ?, ?, 'digest')",
    )
    .bind(r#"["noe@dupont.ch"]"#)
    .bind(channels.email)
    .bind(&armed)
    .bind(&last)
    .execute(pool)
    .await
    .context("demo digest schedule")?;
    sqlx::query(
        "INSERT INTO report_schedules (name, enabled, frequency, day_of_month, hour, timezone,
             recipients, channel_id, armed_at, kind, attach_pdf, attach_csv, objective, client_id)
         VALUES ('Muller monthly SLA', 1, 'monthly', 1, 7, 'Europe/Zurich', ?, ?, ?, 'sla', 1, 1,
                 99.5, ?)",
    )
    .bind(r#"["it@muller-archi.ch","direction@muller-archi.ch"]"#)
    .bind(channels.email)
    .bind(&armed)
    .bind(office)
    .execute(pool)
    .await
    .context("demo SLA schedule")?;
    Ok(())
}

/// Verdict d'une CVE : (CVE, version corrigée, état, raison, source, vue il y a
/// … jours).
type Finding = (&'static str, Option<&'static str>, &'static str, &'static str, &'static str, i64);
/// Composant inventorié : (machine, composant, version, origine, verdict).
type Component = (&'static str, &'static str, &'static str, &'static str, Option<Finding>);

/// Failles connues (CVE), comme les aurait trouvées le calcul quotidien : les
/// verdicts sont écrits ici, aucune source n'est jamais téléchargée (le calcul
/// ne tourne pas en démonstration, voir `vulns::spawn`).
async fn seed_vulnerabilities(
    pool: &SqlitePool,
    targets: &[(TargetId, String, String)],
) -> Result<()> {
    let find = |name: &str| targets.iter().find(|(_, n, _)| n == name).map(|(id, _, _)| *id);
    crate::vulns::set_enabled(pool, true).await?;
    let now = Utc::now();
    #[rustfmt::skip]
    let infos: &[(&str, &str, f64, &str)] = &[
        ("CVE-2025-32463", "sudo before 1.9.17p1: local privilege escalation through the --chroot option.", 9.3, "2025-06-30"),
        ("CVE-2024-6387", "OpenSSH sshd signal handler race (regreSSHion): remote code execution as root.", 8.1, "2024-07-01"),
        ("CVE-2024-3094", "Malicious code in the xz-utils 5.6.0 and 5.6.1 release tarballs.", 10.0, "2024-03-29"),
        ("CVE-2025-33073", "Windows SMB client: improper access control allows elevation of privilege.", 8.8, "2025-06-10"),
    ];
    for (id, summary, cvss, published) in infos {
        sqlx::query(
            "INSERT INTO cve_info (id, summary, cvss, published, scored_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(*id)
        .bind(*summary)
        .bind(*cvss)
        .bind(*published)
        .bind(stamp(now - chrono::Duration::hours(5)))
        .execute(pool)
        .await
        .context("demo CVE")?;
    }
    for (id, added, name) in [
        (
            "CVE-2025-32463",
            "2025-09-29",
            "Sudo Inclusion of Functionality from Untrusted Control Sphere",
        ),
        ("CVE-2025-33073", "2025-10-20", "Microsoft Windows SMB Client Improper Access Control"),
    ] {
        sqlx::query(
            "INSERT INTO cve_kev (cve_id, date_added, ransomware, name) VALUES (?, ?, 0, ?)",
        )
        .bind(id)
        .bind(added)
        .bind(name)
        .execute(pool)
        .await?;
    }
    #[rustfmt::skip]
    let findings: &[Component] = &[
        (AGENT_HOST, "sudo", "1.9.16p2-3", "debian:trixie",
            Some(("CVE-2025-32463", Some("1.9.16p2-3+deb13u1"), "vulnerable", "below_fixed_version", "debian", 6))),
        (AGENT_HOST, "openssh-server", "1:10.0p1-7", "debian:trixie",
            Some(("CVE-2024-6387", Some("1:9.7p1-7"), "patched", "fixed_version_installed", "debian", 30))),
        (AGENT_HOST, "xz-utils", "5.8.1-1", "debian:trixie",
            Some(("CVE-2024-3094", Some("5.6.1+really5.4.5-1"), "patched", "fixed_version_installed", "debian", 30))),
        (AGENT_HOST, "docker-ce", "28.5.1", "nvd", None),
        (RELAY_HOST, "sudo", "1.9.16p2-3+deb13u1", "debian:trixie",
            Some(("CVE-2025-32463", Some("1.9.16p2-3+deb13u1"), "patched", "fixed_version_installed", "debian", 6))),
        (WINDOWS_HOST, "Windows Server 2025", "10.0.26100.32995", "nvd",
            Some(("CVE-2025-33073", None, "unknown", "vendor_build", "nvd", 9))),
    ];
    for (host, component, version, origin, finding) in findings {
        let Some(target) = find(*host) else { continue };
        sqlx::query(
            "INSERT OR IGNORE INTO cve_components (target_id, component, version, origin)
             VALUES (?, ?, ?, ?)",
        )
        .bind(target)
        .bind(*component)
        .bind(*version)
        .bind(*origin)
        .execute(pool)
        .await
        .context("demo CVE component")?;
        let Some((cve, fixed, state, reason, source, days)) = finding else { continue };
        sqlx::query(
            "INSERT INTO cve_findings (target_id, component, cve_id, installed_version,
                 fixed_version, state, reason, source, first_seen)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(target)
        .bind(*component)
        .bind(*cve)
        .bind(*version)
        .bind(*fixed)
        .bind(*state)
        .bind(*reason)
        .bind(*source)
        .bind(stamp(now - chrono::Duration::days(*days)))
        .execute(pool)
        .await
        .context("demo CVE finding")?;
    }
    for source in ["scan", "nvd", "debian", "osv", "kev"] {
        sqlx::query("INSERT INTO cve_sources (name, last_attempt, last_success) VALUES (?, ?, ?)")
            .bind(source)
            .bind(stamp(now - chrono::Duration::hours(5)))
            .bind(stamp(now - chrono::Duration::hours(5)))
            .execute(pool)
            .await?;
    }
    Ok(())
}
