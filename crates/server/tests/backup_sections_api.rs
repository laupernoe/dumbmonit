//! Restauration section par section d'un lot écrit à la main : chaque section
//! est créée, relue à l'identique (rien ne bouge), puis modifiée (mise à jour),
//! et les références cassées ou les lignes invalides sont signalées au lieu
//! de faire échouer le tout. Le chemin HTTP est couvert par `backup_api.rs` ;
//! ici on appelle directement la restauration, avec un lot complet.

use dumbmonit_server::backup::{Bundle, RestoreReport, restore};
use dumbmonit_server::crypto::Cipher;
use dumbmonit_server::db;
use serde_json::{Value, json};
use sqlx::SqlitePool;

const SHELLY: &str = include_str!("../../../packs/shelly-plug/pack.yaml");

struct Instance {
    pool: SqlitePool,
    cipher: Cipher,
    _dir: tempfile::TempDir,
}

async fn instance() -> Instance {
    let dir = tempfile::tempdir().expect("répertoire temporaire");
    let pool = db::open(&dir.path().join("dumbmonit.db")).await.expect("base");
    let cipher = db::init_cipher(&pool, "secret-de-test-suffisamment-long").await.expect("clé");
    Instance { pool, cipher, _dir: dir }
}

fn bundle(value: Value) -> Bundle {
    serde_json::from_value(value).expect("lot lisible")
}

/// Le lot complet : chaque section a une ligne valide et, à côté, ce qui doit
/// être refusé ou signalé.
fn full_bundle() -> Value {
    json!({
        "version": 1,
        "created_at": "2026-10-01 10:00:00",
        "source_version": "9.9.9",
        "packs": [
            { "id": "shelly-plug", "version": "1.0.0", "yaml": SHELLY, "enabled": true },
            { "id": "cassé", "version": "1", "yaml": "pas: un paquet", "enabled": true }
        ],
        "users": [
            { "username": "marie", "display_name": "Marie", "role": "operator",
              "oidc_subject": "sub-1", "oidc_issuer": "https://sso.example",
              "password_hash": "$argon2id$v=19$m=19456,t=2,p=1$aaaa$bbbb",
              "totp_secret": "c2VjcmV0LXRvdHA=", "totp_enabled": true,
              "recovery_codes": ["AAECAw==", "pas du base64 !"] },
            { "username": "paul", "role": "viewer", "oidc_subject": "sub-1" },
            { "username": "intrus", "role": "emperor" }
        ],
        "targets": [
            { "kind": "dummy", "address": "10.0.0.2", "name": "Enfant",
              "parent": "dummy|10.0.0.1", "via_agent": "agent|fantome",
              "interval_secs": 60, "enabled": true, "tags": { "room": "cave" },
              "group_name": "Rack", "position": 3 },
            { "kind": "dummy", "address": "10.0.0.1", "name": "Parent",
              "interval_secs": 60, "enabled": true },
            { "kind": "snmp", "address": "10.0.0.3", "name": "Switch",
              "interval_secs": 120, "enabled": false,
              "credential": { "type": "snmp_community", "community": "secrète" } },
            { "kind": "dummy", "address": "10.0.0.4", "name": "Cassé",
              "interval_secs": 60, "enabled": true, "credential": { "type": "nope" } },
            { "kind": "dummy", "address": "10.0.0.5", "name": "Boucle",
              "parent": "dummy|10.0.0.5", "interval_secs": 60, "enabled": true }
        ],
        "channels": [
            { "name": "Discord", "kind": "discord", "enabled": true,
              "settings": {}, "secrets": { "webhook_url": "https://127.0.0.1:1/hook" },
              "policy": {} }
        ],
        "rules": [
            { "uid": "r-chaleur", "name": "Chaleur", "kind": "threshold",
              "query": "dumbmonit_temperature_celsius", "operator": ">", "threshold": 40.0,
              "for_secs": 300, "severity": "warning", "enabled": true,
              "selector": { "kind": "refs", "refs": ["dummy|10.0.0.1", "dummy|10.9.9.9"] },
              "channels": ["Discord", "Fantôme"] },
            { "uid": "r-orpheline", "name": "Orpheline", "kind": "threshold",
              "query": "up", "operator": "<", "threshold": 1.0, "severity": "info",
              "enabled": true, "selector": { "kind": "refs", "refs": ["dummy|absent"] } },
            { "uid": "r-tous", "name": "Tous", "kind": "threshold", "query": "up",
              "operator": "<", "threshold": 1.0, "severity": "info", "enabled": true,
              "selector": { "kind": "all" } },
            { "uid": "r-invalide", "name": "Invalide", "kind": "magie", "query": "up",
              "operator": "<", "threshold": 1.0, "severity": "info", "enabled": true }
        ],
        "rule_overrides": [
            { "rule_uid": "r-chaleur", "target": "dummy|10.0.0.1", "threshold": 55.0,
              "enabled": false },
            { "rule_uid": "r-chaleur", "target": "dummy|absent", "threshold": 1.0 }
        ],
        "notify_policy": { "grouping": true },
        "silences": [
            { "name": "Nuit", "target": "dummy|10.0.0.1", "enabled": true, "comment": "dodo",
              "schedule": { "kind": "weekly", "days": [0, 1], "start_minute": 0,
                            "end_minute": 360 } },
            { "name": "Globale", "enabled": true,
              "schedule": { "kind": "weekly", "days": [6], "start_minute": 0,
                            "end_minute": 60 } },
            { "name": "Perdue", "target": "dummy|absent", "enabled": true, "schedule": {} }
        ],
        "status_pages": [
            { "slug": "maison", "title": "Maison", "published": true, "theme": "auto",
              "show_uptime_days": 30, "accent": "default", "simple": true,
              "scenes": ["paris", "paris", "atlantide", "tokyo"],
              "scene_rotation": "jamais", "domain": "https://Status.Example.com/",
              "items": [
                { "target": "dummy|10.0.0.1", "label": "Le parent", "position": 1 },
                { "target": "dummy|absent", "label": "Fantôme" }
              ] },
            { "slug": "voisin", "title": "Voisin", "published": false, "theme": "dark",
              "show_uptime_days": 7, "domain": "status.example.com" },
            { "slug": "bizarre", "title": "Bizarre", "published": false, "theme": "light",
              "show_uptime_days": 7, "domain": "pas un/domaine" }
        ],
        "report_schedules": [
            { "name": "Hebdo", "enabled": true, "frequency": "weekly", "weekday": 2,
              "hour": 9, "timezone": "Europe/Paris",
              "recipients": ["Ops@Example.com", "ops@example.com"], "channel": "Discord" },
            { "name": "Sans canal", "enabled": true, "frequency": "daily",
              "recipients": ["a@example.com"], "channel": "Fantôme" },
            { "name": "Sans destinataire", "enabled": true, "frequency": "monthly",
              "day_of_month": 15 },
            { "name": "Invalide", "enabled": true, "frequency": "toujours",
              "recipients": ["a@example.com"] },
            { "name": "Mauvaise adresse", "enabled": true, "frequency": "daily",
              "recipients": ["pas une adresse"] }
        ],
        "incidents": [
            { "page": "maison", "title": "Panne de courant", "kind": "incident",
              "status": "resolved", "severity": "major", "starts_at": "2026-09-30T20:00:00Z",
              "ends_at": "2026-09-30T21:00:00Z",
              "updates": [
                { "status": "investigating", "body": "On regarde.",
                  "created_at": "2026-09-30T20:05:00Z" },
                { "status": "resolved", "body": "Rétabli.",
                  "created_at": "2026-09-30T21:00:00Z" }
              ] },
            { "title": "Maintenance globale", "kind": "maintenance", "status": "scheduled",
              "severity": "minor", "starts_at": "2026-11-01T00:00:00Z" },
            { "page": "inconnue", "title": "Orphelin", "kind": "incident",
              "status": "monitoring", "severity": "minor", "starts_at": "2026-09-01T00:00:00Z" }
        ],
        "agent_tokens": [
            { "name": "Portable", "token_hash": "f1e2d3c4b5a6978877665544332211009988776655443322110099887766554433",
              "prefix": "dmon_f1e2" },
            { "name": "Ancien", "token_hash": "0a0b0c0d", "prefix": "dmon_0a0b",
              "created_at": "2026-01-01T00:00:00Z", "revoked_at": "2026-02-01T00:00:00Z" }
        ],
        "api_tokens": [
            { "name": "Claude", "prefix": "dmak_abcd", "scope": "write",
              "token_hash": "AQIDBAUGBwgJCgsMDQ4PEBESExQVFhcYGRobHB0eHyA=", "user": "marie" },
            { "name": "Sans propriétaire", "prefix": "dmak_efgh", "scope": "read",
              "token_hash": "ICEiIyQlJicoKSorLC0uLzAxMjM0NTY3ODk6Ozw9Pj8=", "user": "disparu" },
            { "name": "Illisible", "prefix": "dmak_zzzz", "scope": "read",
              "token_hash": "***" },
            { "name": "Portée folle", "prefix": "dmak_yyyy", "scope": "root",
              "token_hash": "QUJDREVGR0hJSktMTU5PUFFSU1RVVldYWVo=" }
        ],
        "push_monitors": [
            { "target": "dummy|10.0.0.1", "token": "jeton-de-heartbeat" },
            { "target": "dummy|absent", "token": "orphelin" }
        ]
    })
}

fn section<'a>(report: &'a RestoreReport, name: &str) -> &'a dumbmonit_server::backup::SectionReport {
    report.sections.iter().find(|s| s.section == name).unwrap_or_else(|| panic!("section {name}"))
}

fn counts(report: &RestoreReport, name: &str) -> (usize, usize, usize) {
    let s = section(report, name);
    (s.created, s.updated, s.skipped)
}

async fn scalar(pool: &SqlitePool, query: &'static str) -> i64 {
    sqlx::query_scalar::<_, i64>(query).fetch_one(pool).await.expect("requête")
}

fn notes(report: &RestoreReport, name: &str) -> String {
    section(report, name).notes.join("\n")
}

#[tokio::test]
async fn un_lot_complet_se_restaure_section_par_section() {
    let it = instance().await;
    let report = restore(&it.pool, &it.cipher, &bundle(full_bundle()), true).await.expect("restauration");
    assert!(report.applied);
    assert_eq!(report.source_version, "9.9.9");

    // Équipements : le « cassé » est refusé, la boucle n'est pas son propre parent.
    assert_eq!(counts(&report, "targets"), (4, 0, 1));
    assert!(notes(&report, "targets").contains("agent|fantome"), "{}", notes(&report, "targets"));
    assert!(notes(&report, "targets").contains("Cassé"), "{}", notes(&report, "targets"));
    let child_parent: Option<i64> =
        sqlx::query_scalar("SELECT parent_id FROM targets WHERE address = '10.0.0.2'")
            .fetch_one(&it.pool)
            .await
            .unwrap();
    assert!(child_parent.is_some(), "le parent venu après l'enfant est retrouvé");
    let loop_parent: Option<i64> =
        sqlx::query_scalar("SELECT parent_id FROM targets WHERE address = '10.0.0.5'")
            .fetch_one(&it.pool)
            .await
            .unwrap();
    assert!(loop_parent.is_none());

    // Comptes : le rôle inconnu est refusé, le SSO déjà pris est délié.
    assert_eq!(counts(&report, "users"), (2, 0, 1));
    assert!(notes(&report, "users").contains("SSO identity"), "{}", notes(&report, "users"));
    assert_eq!(scalar(&it.pool, "SELECT COUNT(*) FROM totp_recovery_codes").await, 1);
    assert_eq!(scalar(&it.pool, "SELECT totp_enabled FROM users WHERE username = 'marie'").await, 1);

    // Paquets : le valide s'installe, le cassé est écarté avec la raison.
    assert_eq!(counts(&report, "packs"), (1, 0, 1));
    assert!(notes(&report, "packs").contains("validation"), "{}", notes(&report, "packs"));

    // Règles : sélecteur par références, canal fantôme retiré, sélecteur vide
    // remis sur « tous », valeur de CHECK refusée.
    assert_eq!(counts(&report, "rules"), (3, 0, 1));
    let rules = notes(&report, "rules");
    assert!(rules.contains("dummy|10.9.9.9") && rules.contains("Fantôme"), "{rules}");
    assert!(rules.contains("restored as applying to every device"), "{rules}");
    let selector: String =
        sqlx::query_scalar("SELECT selector FROM alert_rules WHERE uid = 'r-chaleur'")
            .fetch_one(&it.pool)
            .await
            .unwrap();
    assert!(selector.contains("\"ids\""), "{selector}");
    assert_eq!(counts(&report, "rule_overrides"), (1, 0, 1));
    assert_eq!(counts(&report, "notify_policy"), (1, 0, 0));

    assert_eq!(counts(&report, "silences"), (2, 0, 1));
    assert_eq!(counts(&report, "channels"), (1, 0, 0));

    // Pages de statut : décor filtré, domaine normalisé, domaine pris ou
    // invalide abandonné avec une note, service inconnu signalé.
    assert_eq!(counts(&report, "status_pages"), (3, 0, 0));
    let pages = notes(&report, "status_pages");
    assert!(pages.contains("already used by \"maison\""), "{pages}");
    assert!(pages.contains("pas un/domaine"), "{pages}");
    assert!(pages.contains("dummy|absent"), "{pages}");
    let scenes: String = sqlx::query_scalar("SELECT scenes FROM status_pages WHERE slug = 'maison'")
        .fetch_one(&it.pool)
        .await
        .unwrap();
    assert_eq!(scenes, "paris,tokyo");
    let domain: Option<String> =
        sqlx::query_scalar("SELECT domain FROM status_pages WHERE slug = 'maison'")
            .fetch_one(&it.pool)
            .await
            .unwrap();
    assert_eq!(domain.as_deref(), Some("status.example.com"));
    assert_eq!(scalar(&it.pool, "SELECT simple FROM status_pages WHERE slug = 'maison'").await, 1);
    assert_eq!(scalar(&it.pool, "SELECT COUNT(*) FROM status_page_items").await, 1);

    // Rapports : doublons d'adresses fusionnés, canal inconnu signalé, un rapport
    // sans destinataire est créé mais désactivé, les invalides sont écartés.
    assert_eq!(counts(&report, "report_schedules"), (3, 0, 2));
    let schedules = notes(&report, "report_schedules");
    assert!(schedules.contains("Fantôme") && schedules.contains("Invalide"), "{schedules}");
    let recipients: String =
        sqlx::query_scalar("SELECT recipients FROM report_schedules WHERE name = 'Hebdo'")
            .fetch_one(&it.pool)
            .await
            .unwrap();
    assert_eq!(serde_json::from_str::<Vec<String>>(&recipients).unwrap().len(), 1);
    assert_eq!(
        scalar(&it.pool, "SELECT enabled FROM report_schedules WHERE name = 'Sans destinataire'")
            .await,
        0
    );

    // Incidents : récit et mises à jour recopiés, page inconnue signalée.
    assert_eq!(counts(&report, "incidents"), (2, 0, 1));
    assert_eq!(scalar(&it.pool, "SELECT COUNT(*) FROM incident_updates").await, 2);

    // Jetons.
    assert_eq!(counts(&report, "agent_tokens"), (2, 0, 0));
    assert_eq!(counts(&report, "api_tokens"), (2, 0, 2));
    assert_eq!(
        scalar(&it.pool, "SELECT COUNT(*) FROM api_tokens WHERE user_id IS NOT NULL").await,
        1,
        "le propriétaire disparu laisse un jeton sans compte"
    );
    assert_eq!(counts(&report, "push_monitors"), (1, 0, 1));

    // Avertissements : pas de mot de passe dans le lot, secrets d'instance, notes.
    let warnings = report.warnings.join("\n");
    assert!(warnings.contains("no account passwords"), "{warnings}");
    assert!(warnings.contains("secret.key"), "{warnings}");
    assert!(warnings.contains("read the notes"), "{warnings}");
}

#[tokio::test]
async fn relire_le_meme_lot_ne_change_rien() {
    let it = instance().await;
    let lot = bundle(full_bundle());
    restore(&it.pool, &it.cipher, &lot, true).await.expect("première restauration");
    let report = restore(&it.pool, &it.cipher, &lot, true).await.expect("seconde restauration");

    for name in [
        "targets", "channels", "rules", "rule_overrides", "notify_policy", "silences",
        "status_pages", "report_schedules", "incidents", "agent_tokens", "api_tokens",
        "push_monitors", "packs",
    ] {
        let (created, updated, _) = counts(&report, name);
        assert_eq!((created, updated), (0, 0), "la section {name} a bougé : {report:#?}");
    }
    // Un compte existant n'est jamais touché.
    assert_eq!(counts(&report, "users").0, 0);
    assert!(notes(&report, "users").contains("already exists"), "{}", notes(&report, "users"));
}

#[tokio::test]
async fn un_lot_modifie_met_chaque_section_a_jour() {
    let it = instance().await;
    restore(&it.pool, &it.cipher, &bundle(full_bundle()), true).await.expect("première");

    let mut lot = full_bundle();
    lot["targets"][0]["name"] = json!("Enfant renommé");
    lot["targets"][2]["credential"]["community"] = json!("autre");
    lot["channels"][0]["settings"] = json!({ "mention": "@ops" });
    lot["rules"][0]["threshold"] = json!(45.0);
    lot["rule_overrides"][0]["threshold"] = json!(60.0);
    lot["notify_policy"] = json!({ "grouping": false });
    lot["silences"][0]["comment"] = json!("réveil");
    lot["status_pages"][0]["title"] = json!("Maison v2");
    lot["status_pages"][0]["items"][0]["label"] = json!("Le parent bis");
    lot["report_schedules"][0]["hour"] = json!(10);
    lot["push_monitors"][0]["token"] = json!("nouveau-jeton");
    lot["packs"][0]["enabled"] = json!(false);

    let report = restore(&it.pool, &it.cipher, &bundle(lot), true).await.expect("seconde");
    for (name, expected) in [
        ("targets", 2),
        ("channels", 1),
        ("rules", 1),
        ("rule_overrides", 1),
        ("notify_policy", 1),
        ("silences", 1),
        ("status_pages", 1),
        ("report_schedules", 1),
        ("push_monitors", 1),
        ("packs", 1),
    ] {
        assert_eq!(counts(&report, name).1, expected, "section {name} : {report:#?}");
    }
    assert_eq!(
        scalar(&it.pool, "SELECT COUNT(*) FROM targets WHERE name = 'Enfant renommé'").await,
        1
    );
    assert_eq!(scalar(&it.pool, "SELECT enabled FROM packs WHERE id = 'shelly-plug'").await, 0);
}

#[tokio::test]
async fn la_simulation_n_ecrit_rien_mais_rend_le_meme_compte_rendu() {
    let it = instance().await;
    let report = restore(&it.pool, &it.cipher, &bundle(full_bundle()), false).await.expect("simulation");
    assert!(!report.applied);
    assert_eq!(counts(&report, "targets").0, 4);
    assert_eq!(scalar(&it.pool, "SELECT COUNT(*) FROM targets").await, 0);
    assert_eq!(scalar(&it.pool, "SELECT COUNT(*) FROM status_pages").await, 0);
    assert_eq!(scalar(&it.pool, "SELECT COUNT(*) FROM users").await, 0);
}
