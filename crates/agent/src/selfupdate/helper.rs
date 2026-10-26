//! Assistant Windows de mise à jour (`--update-helper <registre>`).
//!
//! Windows ne laisse pas écraser l'exécutable d'un processus en cours, et un
//! service ne peut pas attendre son propre arrêt. L'agent copie donc son
//! binaire (l'ancien, connu pour marcher) sous un autre nom, le lance détaché,
//! puis s'arrête normalement. Cette copie :
//!
//! 1. attend l'arrêt du service (et le demande au besoin) ;
//! 2. met l'ancien binaire de côté (`.previous`) et le nouveau à sa place ;
//! 3. redémarre le service et surveille l'essai ;
//! 4. faute de premier lot accepté dans le délai, rétablit l'ancien binaire et
//!    redémarre le service une dernière fois.
//!
//! Ce fichier n'est compilé que sur Windows.

use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use dumbmonit_proto::{SelfUpdateOutcome, UPDATE_TRIAL_SECS};
use windows_service::service::{Service, ServiceAccess, ServiceState};
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

use super::ledger::{self, Ledger};
use crate::winsvc::SERVICE_NAME;

/// Processus détaché de la console et du groupe du service.
const DETACHED_PROCESS: u32 = 0x0000_0008;
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

/// Lance l'assistant, détaché du service qui va s'arrêter.
pub fn spawn(assistant: &Path, ledger_path: &Path) -> Result<()> {
    use std::os::windows::process::CommandExt as _;
    std::process::Command::new(assistant)
        .arg("--update-helper")
        .arg(ledger_path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
        .spawn()
        .context("starting the update helper")?;
    Ok(())
}

fn open_service() -> Result<Service> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .context("connecting to the service manager")?;
    manager
        .open_service(
            SERVICE_NAME,
            ServiceAccess::QUERY_STATUS | ServiceAccess::START | ServiceAccess::STOP,
        )
        .context("opening the DumbMonit service")
}

fn state(service: &Service) -> Option<ServiceState> {
    service.query_status().ok().map(|status| status.current_state)
}

/// Attend l'arrêt du service ; le demande s'il tarde.
fn wait_stopped(service: &Service) -> Result<()> {
    let started = Instant::now();
    let mut asked = false;
    loop {
        if state(service) == Some(ServiceState::Stopped) {
            return Ok(());
        }
        let waited = started.elapsed();
        if !asked && waited > Duration::from_secs(30) {
            asked = true;
            let _ = service.stop();
        }
        if waited > Duration::from_secs(120) {
            anyhow::bail!("the DumbMonit service does not stop");
        }
        std::thread::sleep(Duration::from_secs(1));
    }
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// Clôt l'essai sur un échec et redémarre le service tel qu'il est.
fn give_up(service: &Service, path: &Path, reason: String) -> Result<()> {
    let mut ledger = Ledger::load(path);
    if let Some(trial) = ledger.trial.clone() {
        if let Some(staged) = &trial.staged {
            let _ = std::fs::remove_file(staged);
        }
        ledger.close(ledger::report(
            SelfUpdateOutcome::Failed,
            &trial.from_version,
            &trial.to_version,
            &trial.to_sha256,
            Some(reason),
            now_ms(),
        ));
        ledger.save(path)?;
    }
    service.start::<&str>(&[]).context("starting the DumbMonit service")
}

pub fn run(path: &Path) -> Result<()> {
    let mut ledger = Ledger::load(path);
    let trial = ledger.trial.clone().context("no agent update to install")?;
    let staged = trial.staged.clone().context("no staged agent binary")?;
    let service = open_service()?;
    wait_stopped(&service)?;

    // Échange : l'ancien de côté, le nouveau à sa place.
    let _ = std::fs::remove_file(&trial.backup);
    if let Err(error) = std::fs::rename(&trial.binary, &trial.backup) {
        return give_up(&service, path, format!("cannot move the old binary aside: {error}"));
    }
    if let Err(error) = std::fs::rename(&staged, &trial.binary) {
        let _ = std::fs::rename(&trial.backup, &trial.binary);
        return give_up(&service, path, format!("cannot put the new binary in place: {error}"));
    }
    if let Some(current) = ledger.trial.as_mut() {
        current.staged = None;
    }
    ledger.save(path)?;
    service.start::<&str>(&[]).context("starting the updated DumbMonit service")?;

    // Essai : le nouvel agent clôt lui-même l'essai à son premier lot accepté.
    let deadline = Instant::now() + Duration::from_secs(UPDATE_TRIAL_SECS + 30);
    while Instant::now() < deadline {
        std::thread::sleep(Duration::from_secs(2));
        if Ledger::load(path).trial.is_none() {
            return Ok(());
        }
    }

    // Pas de lot accepté à temps : l'ancien binaire est rétabli.
    let _ = service.stop();
    wait_stopped(&service)?;
    let mut ledger = Ledger::load(path);
    if ledger.trial.is_some() {
        let reason = format!(
            "the new agent had no batch accepted by the server within {UPDATE_TRIAL_SECS} s"
        );
        ledger::roll_back(&mut ledger, &reason, now_ms())?;
        ledger.save(path)?;
    }
    service.start::<&str>(&[]).context("starting the DumbMonit service")
}
