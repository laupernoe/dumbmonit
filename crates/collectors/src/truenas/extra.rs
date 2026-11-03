//! Mesures de profondeur : certificats, pool de démarrage, tâches de
//! sauvegarde distantes, applications et machines virtuelles.
//!
//! Même principe que le reste du collecteur : des structures tolérantes, des
//! fonctions pures de la réponse vers les échantillons, une série en moins quand
//! une valeur manque. Rien n'est déclenché, tout est lu.

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use dumbmonit_proto::{MetricKind, Sample};
use serde::Deserialize;
use serde_json::Value;

use super::model::{DateMs, Num, Pool, date_value};
use super::options::MAX_SERIES_PER_FAMILY;

fn gauge(name: &str, value: f64, labels: &[(&str, &str)], ts_ms: i64) -> Sample {
    let mut sample = Sample::new(name, value, MetricKind::Gauge, ts_ms);
    for (key, label) in labels {
        sample.labels.insert((*key).to_string(), (*label).to_string());
    }
    sample
}

fn flag(value: bool) -> f64 {
    if value { 1.0 } else { 0.0 }
}

// --------------------------------------------------------------------------
// Certificats
// --------------------------------------------------------------------------

/// Une entrée de `GET /api/v2.0/certificate`.
#[derive(Debug, Default, Deserialize)]
pub struct Certificate {
    #[serde(default)]
    pub id: Option<Num>,
    #[serde(default)]
    pub name: Option<String>,
    /// Date de fin : chaîne à la `openssl` (`"Tue Dec 30 20:39:46 2025"`) ou
    /// `{"$date": …}` selon la version.
    #[serde(default)]
    pub until: Option<Value>,
    /// Le texte d'une demande de signature, qui n'expire pas ; `null` sinon.
    #[serde(default, rename = "CSR")]
    pub csr: Option<Value>,
}

fn month_number(name: &str) -> Option<u32> {
    const MONTHS: &str = "janfebmaraprmayjunjulaugsepoctnovdec";
    let lower = name.to_ascii_lowercase();
    if lower.len() != 3 {
        return None;
    }
    // Trois lettres, à une position multiple de trois.
    let at = MONTHS.find(lower.as_str())?;
    (at % 3 == 0).then_some(at as u32 / 3 + 1)
}

/// Lit une date de fin de certificat, quelle qu'en soit la forme.
pub fn certificate_end(value: &Value) -> Option<i64> {
    if let Value::String(text) = value {
        // « Tue Dec 30 20:39:46 2025 » : le jour de semaine est ignoré, et les
        // espaces doublés devant un jour à un chiffre aussi.
        let tokens: Vec<&str> = text.split_whitespace().collect();
        if tokens.len() == 5 {
            let month = month_number(tokens[1])?;
            let day: u32 = tokens[2].parse().ok()?;
            let year: i32 = tokens[4].parse().ok()?;
            let time = NaiveTime::parse_from_str(tokens[3], "%H:%M:%S").ok()?;
            let date = NaiveDate::from_ymd_opt(year, month, day)?;
            return Some(NaiveDateTime::new(date, time).and_utc().timestamp());
        }
    }
    date_value(value)
}

/// L'identifiant du certificat de l'interface web, dans `GET system/general` :
/// `ui_certificate` est un objet `{id, …}` ou, selon la version, un nombre.
pub fn ui_certificate_id(general: &Value) -> Option<f64> {
    match general.get("ui_certificate")? {
        Value::Object(map) => map.get("id").and_then(Value::as_f64),
        other => other.as_f64(),
    }
}

/// Jours restants de chaque certificat (négatifs une fois expiré). `in_use` vaut
/// `1` pour celui qui sert l'interface web : c'est le seul sur lequel une règle
/// a un sens, un certificat de rechange périmé ne gêne personne.
pub fn certificate_samples(
    certificates: &[Certificate],
    ui_id: Option<f64>,
    now_s: i64,
    ts_ms: i64,
) -> Vec<Sample> {
    certificates
        .iter()
        .filter(|cert| !cert.csr.as_ref().is_some_and(|text| text.is_string()))
        .filter_map(|cert| {
            let end = certificate_end(cert.until.as_ref()?)?;
            let id = cert.id.map(|n| n.0);
            let name = cert
                .name
                .clone()
                .filter(|name| !name.trim().is_empty())
                .or_else(|| id.map(|id| format!("certificate {id}")))?;
            let in_use = ui_id.is_some() && id == ui_id;
            Some(gauge(
                "truenas_certificate_expiry_days",
                (end - now_s) as f64 / 86_400.0,
                &[("certificate", name.as_str()), ("in_use", if in_use { "1" } else { "0" })],
                ts_ms,
            ))
        })
        .take(MAX_SERIES_PER_FAMILY)
        .collect()
}

// --------------------------------------------------------------------------
// Pool de démarrage
// --------------------------------------------------------------------------

/// Le pool de démarrage (`GET boot/get_state`) : même forme qu'un pool. Un miroir
/// de démarrage dégradé continue de démarrer, comme un pool de données.
pub fn boot_samples(boot: &Pool, ts_ms: i64) -> Vec<Sample> {
    let mut samples = Vec::new();
    if let Some(healthy) = boot.healthy {
        samples.push(gauge("truenas_boot_pool_healthy", flag(healthy), &[], ts_ms));
    }
    if let (Some(size), Some(allocated)) = (boot.size, boot.allocated)
        && size.0 > 0.0
    {
        let used = 100.0 * allocated.0 / size.0;
        samples.push(gauge("truenas_boot_pool_used_percent", used, &[], ts_ms));
    }
    samples
}

// --------------------------------------------------------------------------
// Tâches de sauvegarde distantes : cloud sync, rsync, sauvegarde cloud
// --------------------------------------------------------------------------

/// Le dernier travail d'une tâche (`job`), `null` pour une tâche jamais lancée.
#[derive(Debug, Default, Deserialize)]
pub struct Job {
    /// `PENDING`, `WAITING`, `RUNNING`, `SUCCESS`, `FAILED`, `ABORTED`.
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub time_finished: Option<DateMs>,
}

/// Une entrée de `cloudsync`, `rsynctask` ou `cloud_backup` : le nom du champ
/// de description varie (`description` ou `desc`).
#[derive(Debug, Default, Deserialize)]
pub struct BackupTask {
    #[serde(default)]
    pub id: Option<Num>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub desc: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub job: Option<Job>,
}

/// `failed` vaut 1 quand le dernier travail a échoué ; l'âge est celui de sa
/// fin. Une tâche désactivée ne produit rien, une tâche jamais lancée non plus.
pub fn backup_task_samples(
    kind: &str,
    tasks: &[BackupTask],
    now_s: i64,
    ts_ms: i64,
) -> Vec<Sample> {
    let mut samples = Vec::new();
    let enabled = tasks.iter().filter(|task| task.enabled.unwrap_or(true));
    for task in enabled.take(MAX_SERIES_PER_FAMILY) {
        let Some(job) = &task.job else { continue };
        let name = task
            .description
            .clone()
            .or_else(|| task.desc.clone())
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| format!("{kind} {}", task.id.map(|n| n.0).unwrap_or(0.0)));
        let labels = [("kind", kind), ("task", name.as_str())];
        // En cours, en attente ou interrompue : ni succès ni échec.
        match job.state.as_deref() {
            Some("FAILED") => {
                samples.push(gauge("truenas_backup_task_failed", 1.0, &labels, ts_ms));
            }
            Some("SUCCESS") => {
                samples.push(gauge("truenas_backup_task_failed", 0.0, &labels, ts_ms));
            }
            _ => {}
        }
        if let Some(finished) = job.time_finished.and_then(|date| date.seconds()) {
            let age = (now_s - finished).max(0) as f64;
            samples.push(gauge("truenas_backup_task_last_run_age_seconds", age, &labels, ts_ms));
        }
    }
    samples
}

// --------------------------------------------------------------------------
// Applications et machines virtuelles
// --------------------------------------------------------------------------

/// Une entrée de `GET /api/v2.0/app`.
#[derive(Debug, Default, Deserialize)]
pub struct App {
    #[serde(default)]
    pub name: Option<String>,
    /// `RUNNING`, `STOPPED`, `DEPLOYING`, `CRASHED`.
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub upgrade_available: Option<bool>,
}

pub fn app_samples(apps: &[App], ts_ms: i64) -> Vec<Sample> {
    let mut samples = Vec::new();
    for app in apps.iter().take(MAX_SERIES_PER_FAMILY) {
        let Some(name) = app.name.clone().filter(|name| !name.trim().is_empty()) else {
            continue;
        };
        let labels = [("app", name.as_str())];
        let state = app.state.as_deref().unwrap_or("");
        samples.push(gauge("truenas_app_running", flag(state == "RUNNING"), &labels, ts_ms));
        samples.push(gauge("truenas_app_crashed", flag(state == "CRASHED"), &labels, ts_ms));
        if let Some(upgrade) = app.upgrade_available {
            samples.push(gauge("truenas_app_upgrade_available", flag(upgrade), &labels, ts_ms));
        }
    }
    samples
}

#[derive(Debug, Default, Deserialize)]
pub struct VmStatus {
    /// `RUNNING`, `STOPPED`, `SUSPENDED`, `ERROR`.
    #[serde(default)]
    pub state: Option<String>,
}

/// Une entrée de `GET /api/v2.0/vm`.
#[derive(Debug, Default, Deserialize)]
pub struct Vm {
    #[serde(default)]
    pub name: Option<String>,
    /// Démarre avec le NAS.
    #[serde(default)]
    pub autostart: Option<bool>,
    #[serde(default)]
    pub status: Option<VmStatus>,
}

/// Seules les machines qui démarrent avec le NAS sont mesurées : une machine
/// éteinte exprès n'a pas de série et ne sonne pas.
pub fn vm_samples(vms: &[Vm], ts_ms: i64) -> Vec<Sample> {
    vms.iter()
        .filter(|vm| vm.autostart.unwrap_or(false))
        .filter_map(|vm| {
            let name = vm.name.clone().filter(|name| !name.trim().is_empty())?;
            let state = vm.status.as_ref().and_then(|status| status.state.as_deref())?;
            let running = flag(state == "RUNNING");
            Some(gauge("truenas_vm_running", running, &[("vm", name.as_str())], ts_ms))
        })
        .take(MAX_SERIES_PER_FAMILY)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(samples: &[Sample], metric: &str, label: (&str, &str)) -> Option<f64> {
        let wanted = Some(label.1);
        samples
            .iter()
            .find(|s| s.metric == metric && s.labels.get(label.0).map(String::as_str) == wanted)
            .map(|s| s.value)
    }

    #[test]
    fn la_fin_d_un_certificat_se_lit_sous_les_deux_formes() {
        let text = serde_json::json!("Tue Dec 30 20:39:46 2025");
        assert_eq!(certificate_end(&text), Some(1_767_127_186));
        // Jour à un chiffre : openssl double l'espace.
        let padded = serde_json::json!("Sat Mar  7 10:11:12 2026");
        assert_eq!(certificate_end(&padded), Some(1_772_878_272));
        let object = serde_json::json!({"$date": 1_767_127_186_000_i64});
        assert_eq!(certificate_end(&object), Some(1_767_127_186));
        assert_eq!(certificate_end(&serde_json::json!("n'importe quoi")), None);
    }

    #[test]
    fn seul_le_certificat_de_l_interface_est_marque_en_service() {
        let certs: Vec<Certificate> = serde_json::from_str(
            r#"[
              {"id":1,"name":"truenas_default","until":"Tue Dec 30 20:39:46 2025","CSR":null},
              {"id":2,"name":"lan","until":"Sat Mar  7 10:11:12 2026","CSR":null},
              {"id":3,"name":"request","until":null,"CSR":"-----BEGIN CERTIFICATE REQUEST-----"}
            ]"#,
        )
        .unwrap();
        let general = serde_json::json!({"ui_certificate": {"id": 2, "name": "lan"}});
        let ui = ui_certificate_id(&general);
        assert_eq!(ui, Some(2.0));
        let samples = certificate_samples(&certs, ui, 1_767_127_186 - 86_400 * 10, 0);
        assert_eq!(samples.len(), 2, "la demande de signature n'expire pas");
        let default = samples.iter().find(|s| s.labels["certificate"] == "truenas_default");
        assert_eq!(default.unwrap().value, 10.0);
        assert_eq!(default.unwrap().labels["in_use"], "0");
        let lan = samples.iter().find(|s| s.labels["certificate"] == "lan").unwrap();
        assert_eq!(lan.labels["in_use"], "1");
        // Un nombre nu pour `ui_certificate` est aussi accepté.
        assert_eq!(ui_certificate_id(&serde_json::json!({"ui_certificate": 5})), Some(5.0));
        assert_eq!(ui_certificate_id(&serde_json::json!({})), None);
    }

    #[test]
    fn un_certificat_expire_a_des_jours_negatifs() {
        let certs: Vec<Certificate> =
            serde_json::from_str(r#"[{"id":1,"name":"old","until":"Tue Dec 30 20:39:46 2025"}]"#)
                .unwrap();
        let samples = certificate_samples(&certs, Some(1.0), 1_767_127_186 + 86_400, 0);
        assert_eq!(samples[0].value, -1.0);
    }

    #[test]
    fn le_pool_de_demarrage_degrade_et_plein() {
        let boot: Pool = serde_json::from_str(
            r#"{"name":"boot-pool","status":"DEGRADED","healthy":false,
                "size":1000,"allocated":850,"free":150}"#,
        )
        .unwrap();
        let samples = boot_samples(&boot, 0);
        assert_eq!(samples[0].metric, "truenas_boot_pool_healthy");
        assert_eq!(samples[0].value, 0.0);
        assert_eq!(samples[1].value, 85.0);
        assert!(boot_samples(&Pool::default(), 0).is_empty());
    }

    #[test]
    fn une_tache_distante_en_echec_et_une_tache_jamais_lancee() {
        let tasks: Vec<BackupTask> = serde_json::from_str(
            r#"[
              {"id":1,"description":"B2 nightly","enabled":true,
               "job":{"state":"FAILED","time_finished":{"$date":1790000000000}}},
              {"id":2,"description":"S3 weekly","enabled":true,
               "job":{"state":"SUCCESS","time_finished":{"$date":1789990000000}}},
              {"id":3,"description":"running","enabled":true,"job":{"state":"RUNNING"}},
              {"id":4,"description":"never","enabled":true,"job":null},
              {"id":5,"description":"off","enabled":false,"job":{"state":"FAILED"}}
            ]"#,
        )
        .unwrap();
        let samples = backup_task_samples("cloudsync", &tasks, 1_790_000_100, 0);
        let failed = "truenas_backup_task_failed";
        assert_eq!(value(&samples, failed, ("task", "B2 nightly")), Some(1.0));
        assert_eq!(value(&samples, failed, ("task", "S3 weekly")), Some(0.0));
        assert_eq!(value(&samples, failed, ("task", "running")), None);
        let age = "truenas_backup_task_last_run_age_seconds";
        assert_eq!(value(&samples, age, ("task", "B2 nightly")), Some(100.0));
        assert!(samples.iter().all(|s| s.labels["task"] != "never" && s.labels["task"] != "off"));
        assert!(samples.iter().all(|s| s.labels["kind"] == "cloudsync"));
    }

    #[test]
    fn une_tache_rsync_utilise_desc() {
        let tasks: Vec<BackupTask> = serde_json::from_str(
            r#"[{"id":9,"desc":"to-backup-box","enabled":true,"job":{"state":"FAILED"}}]"#,
        )
        .unwrap();
        let samples = backup_task_samples("rsync", &tasks, 0, 0);
        assert_eq!(samples[0].labels["task"], "to-backup-box");
    }

    #[test]
    fn une_application_plantee_et_une_mise_a_jour() {
        let apps: Vec<App> = serde_json::from_str(
            r#"[{"name":"plex","state":"RUNNING","upgrade_available":true},
                {"name":"nextcloud","state":"CRASHED","upgrade_available":false},
                {"name":"old","state":"STOPPED"}]"#,
        )
        .unwrap();
        let samples = app_samples(&apps, 0);
        assert_eq!(value(&samples, "truenas_app_running", ("app", "plex")), Some(1.0));
        assert_eq!(value(&samples, "truenas_app_crashed", ("app", "nextcloud")), Some(1.0));
        assert_eq!(value(&samples, "truenas_app_upgrade_available", ("app", "plex")), Some(1.0));
        assert_eq!(value(&samples, "truenas_app_upgrade_available", ("app", "old")), None);
    }

    #[test]
    fn seules_les_machines_a_demarrage_automatique_sont_mesurees() {
        let vms: Vec<Vm> = serde_json::from_str(
            r#"[{"name":"win11","autostart":true,"status":{"state":"STOPPED"}},
                {"name":"lab","autostart":false,"status":{"state":"STOPPED"}},
                {"name":"hass","autostart":true,"status":{"state":"RUNNING"}}]"#,
        )
        .unwrap();
        let samples = vm_samples(&vms, 0);
        assert_eq!(samples.len(), 2);
        assert_eq!(value(&samples, "truenas_vm_running", ("vm", "win11")), Some(0.0));
        assert_eq!(value(&samples, "truenas_vm_running", ("vm", "hass")), Some(1.0));
    }
}
