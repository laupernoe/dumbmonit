//! La mise à jour de bout en bout, contre un faux serveur : téléchargement,
//! vérifications, mise en place, essai, retour en arrière.

use std::path::{Path, PathBuf};
use std::time::Duration;

use dumbmonit_proto::{AgentUpdateOffer, SelfUpdateAbility, SelfUpdateOutcome};

use super::ledger::{Ledger, Trial};
use super::*;
use crate::client::PushClient;
use crate::testsupport::fake_http;

/// Clé publique de test (voir `signature.rs`).
const TEST_KEY: &str = "-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEA8/ZUyYZRHvnPqoa1ZNbEvPh85mZz0sKRhzS3dgloEHU=
-----END PUBLIC KEY-----
";

/// « Nouveau binaire » : un script qui répond à `--version` comme l'agent.
const NEW_BINARY: &str = "#!/bin/sh\necho dumbmonit-agent 9.9.9\n";
/// SHA-256 de [`NEW_BINARY`] (`sha256sum`).
const NEW_SHA: &str = "4c83650353c2a24cfaf11afd574aebbb9507d2196e7042fccd8741eea4f2e4c1";
/// Signature Ed25519 de [`NEW_BINARY`] par la clé de test (`openssl pkeyutl`).
const NEW_SIGNATURE: &str =
    "EumT6dPhjLjPpToBc63/zuPQf66QHWYBpRlwwEfhlD/omOEl9bIWTqvyc/sgslEf9TGzmf/AVOSxv5lLWYk4Dw==";
/// Signature valable, par la clé de test, d'un autre contenu.
const FOREIGN_SIGNATURE: &str =
    "MklaAxztG+jnCZhMAvBGIWI9TvW+cy8aw01ahKBP/SlGVjouUpUVgiA2M11nNfarU5E6Eb74UFsrZu2VQUQHAA==";
const OLD_BINARY: &str = "#!/bin/sh\necho dumbmonit-agent 0.0.1\n";

fn offer() -> AgentUpdateOffer {
    AgentUpdateOffer {
        version: "9.9.9".into(),
        build: None,
        file: "dumbmonit-agent-linux-x86_64".into(),
        sha256: NEW_SHA.into(),
        size: None,
        signature: Some(NEW_SIGNATURE.into()),
        allow_unsigned: false,
        allow_downgrade: false,
        force: false,
    }
}

fn installed(dir: &Path) -> PathBuf {
    let exe = dir.join("dumbmonit-agent");
    std::fs::write(&exe, OLD_BINARY).unwrap();
    exe
}

fn updater(exe: &Path) -> SelfUpdater {
    SelfUpdater::with(Some(exe.to_path_buf()), Some("0".repeat(64)), true, false)
        .with_keys(signature::parse_keys(TEST_KEY))
}

fn client(url: &str) -> PushClient {
    PushClient::new(url, "dmon_jeton", None, Duration::from_secs(5)).unwrap()
}

#[cfg(unix)]
#[tokio::test]
async fn a_signed_build_is_downloaded_checked_and_put_in_place() {
    let dir = tempfile::tempdir().unwrap();
    let exe = installed(dir.path());
    let (url, calls) = fake_http(|_| (200, NEW_BINARY.to_string())).await;
    let mut updater = updater(&exe);
    assert_eq!(updater.info().ability, SelfUpdateAbility::Ready);

    let after = updater.consider(&offer(), &client(&url), "machine-1").await;

    assert_eq!(after, Some(AfterStop::Exec(exe.clone())));
    let request = &calls.all()[0];
    assert_eq!(request.path, "/api/agent/update/download/dumbmonit-agent-linux-x86_64");
    assert_eq!(request.header("x-dumbmonit-agent-key"), Some("machine-1"));
    assert_eq!(std::fs::read_to_string(&exe).unwrap(), NEW_BINARY, "nouveau binaire en place");
    let backup = backup_path(&exe);
    assert_eq!(std::fs::read_to_string(&backup).unwrap(), OLD_BINARY, "ancien gardé");
    let trial = Ledger::load(&ledger_path(&exe)).trial.expect("essai en cours");
    assert_eq!((trial.to_version.as_str(), trial.to_sha256.as_str()), ("9.9.9", NEW_SHA));
    assert!(!staged_path(&exe).exists(), "plus de fichier intermédiaire");
}

#[cfg(unix)]
#[tokio::test]
async fn a_binary_that_does_not_report_the_announced_version_is_not_installed() {
    // Le script annonce 9.9.9 : une offre de 9.9.8 (signée par ailleurs) est
    // écartée par l'essai `--version`, l'ancien binaire reste.
    let dir = tempfile::tempdir().unwrap();
    let exe = installed(dir.path());
    let (url, _) = fake_http(|_| (200, NEW_BINARY.to_string())).await;
    let mut updater = updater(&exe);
    let wrong = AgentUpdateOffer { version: "9.9.8".into(), ..offer() };

    assert_eq!(updater.consider(&wrong, &client(&url), "k").await, None);
    assert_eq!(std::fs::read_to_string(&exe).unwrap(), OLD_BINARY);
    let last = updater.info().last.expect("échec rapporté");
    assert_eq!(last.outcome, SelfUpdateOutcome::Failed);
    let reason = last.reason.unwrap_or_default();
    assert!(reason.contains("9.9.8"), "{reason}");
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn a_binary_still_open_for_writing_is_tried_again_rather_than_refused() {
    // Tant qu'un descripteur en écriture reste ouvert, Linux refuse l'exécution
    // (ETXTBSY) : c'est ce que laisse un `fork` concurrent pendant `stage`.
    // L'essai `--version` attend qu'il se referme au lieu d'échouer.
    let dir = tempfile::tempdir().unwrap();
    let binary = dir.path().join("dumbmonit-agent.new");
    stage(&binary, NEW_BINARY.as_bytes()).unwrap();
    let writer = std::fs::OpenOptions::new().write(true).open(&binary).unwrap();
    let release = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        drop(writer);
    });
    assert_eq!(preflight(&binary, "9.9.9").await, Ok(()));
    release.await.unwrap();
}

#[tokio::test]
async fn a_download_that_does_not_match_its_checksum_is_retried_later() {
    let dir = tempfile::tempdir().unwrap();
    let exe = installed(dir.path());
    let (url, calls) = fake_http(|_| (200, "autre chose".to_string())).await;
    let mut updater = updater(&exe).without_preflight();

    assert_eq!(updater.consider(&offer(), &client(&url), "k").await, None);
    assert_eq!(std::fs::read_to_string(&exe).unwrap(), OLD_BINARY);
    assert!(updater.info().last.is_none(), "passager : rien de définitif");
    // Pas de nouvel essai dans la foulée.
    assert_eq!(updater.consider(&offer(), &client(&url), "k").await, None);
    assert_eq!(calls.all().len(), 1);
}

#[tokio::test]
async fn a_bad_signature_is_a_failure_that_is_not_retried() {
    let dir = tempfile::tempdir().unwrap();
    let exe = installed(dir.path());
    let (url, calls) = fake_http(|_| (200, NEW_BINARY.to_string())).await;
    let mut updater = updater(&exe).without_preflight();
    // Signature valable… d'un autre contenu.
    let forged = AgentUpdateOffer { signature: Some(FOREIGN_SIGNATURE.into()), ..offer() };

    assert_eq!(updater.consider(&forged, &client(&url), "k").await, None);
    assert_eq!(std::fs::read_to_string(&exe).unwrap(), OLD_BINARY);
    let last = updater.info().last.expect("échec rapporté");
    assert_eq!(last.outcome, SelfUpdateOutcome::Failed);
    assert!(last.reason.unwrap().contains("signature"));
    // La même version n'est plus retentée d'elle-même…
    assert_eq!(updater.consider(&forged, &client(&url), "k").await, None);
    assert_eq!(calls.all().len(), 1);
}

#[tokio::test]
async fn an_unsigned_build_over_the_plain_channel_is_refused_without_downloading() {
    let dir = tempfile::tempdir().unwrap();
    let exe = installed(dir.path());
    let (url, calls) = fake_http(|_| (200, NEW_BINARY.to_string())).await;
    let mut updater = updater(&exe);
    let unsigned = AgentUpdateOffer { signature: None, allow_unsigned: true, ..offer() };

    assert_eq!(updater.consider(&unsigned, &client(&url), "k").await, None);
    assert!(calls.all().is_empty(), "rien téléchargé");
    let last = updater.info().last.expect("refus rapporté");
    assert_eq!(last.outcome, SelfUpdateOutcome::Refused);
    assert!(Ledger::load(&ledger_path(&exe)).failed.is_empty(), "un refus se réévalue");
}

#[tokio::test]
async fn an_agent_in_a_container_never_replaces_itself() {
    let dir = tempfile::tempdir().unwrap();
    let exe = installed(dir.path());
    let (url, calls) = fake_http(|_| (200, NEW_BINARY.to_string())).await;
    let mut updater = SelfUpdater::with(Some(exe.clone()), None, true, true);
    assert_eq!(updater.info().ability, SelfUpdateAbility::Container);
    assert_eq!(updater.consider(&offer(), &client(&url), "k").await, None);
    assert!(calls.all().is_empty());
    // Désactivé localement : même chose, et dit comme tel.
    let disabled = SelfUpdater::with(Some(exe), None, false, false);
    assert_eq!(disabled.info().ability, SelfUpdateAbility::Disabled);
}

fn trial(exe: &Path) -> Trial {
    Trial {
        from_version: "0.0.1".into(),
        from_sha256: "0".repeat(64),
        to_version: "9.9.9".into(),
        to_sha256: NEW_SHA.into(),
        binary: exe.to_path_buf(),
        backup: backup_path(exe),
        staged: None,
        started_ms: 1,
        boots: 1,
    }
}

#[test]
fn the_first_accepted_batch_confirms_the_trial() {
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("dumbmonit-agent");
    std::fs::write(&exe, NEW_BINARY).unwrap();
    let ledger = Ledger { trial: Some(trial(&exe)), ..Ledger::default() };
    ledger.save(&ledger_path(&exe)).unwrap();

    let mut updater = SelfUpdater::with(Some(exe.clone()), Some(NEW_SHA.into()), true, false);
    assert!(updater.in_trial());
    assert!(updater.confirm());
    assert!(!updater.in_trial());
    assert!(!updater.confirm(), "une seule fois");
    let last = updater.info().last.expect("issue rapportée");
    assert_eq!(last.outcome, SelfUpdateOutcome::Succeeded);
    assert!(Ledger::load(&ledger_path(&exe)).trial.is_none());
}

#[cfg(unix)]
#[test]
fn without_an_accepted_batch_the_previous_binary_comes_back() {
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("dumbmonit-agent");
    std::fs::write(&exe, NEW_BINARY).unwrap();
    std::fs::write(backup_path(&exe), OLD_BINARY).unwrap();
    let ledger = Ledger { trial: Some(trial(&exe)), ..Ledger::default() };
    ledger.save(&ledger_path(&exe)).unwrap();

    let mut updater = SelfUpdater::with(Some(exe.clone()), Some(NEW_SHA.into()), true, false);
    assert!(updater.trial_deadline().is_some());

    assert_eq!(updater.expire_trial(), Some(AfterStop::Exec(exe.clone())));
    assert_eq!(std::fs::read_to_string(&exe).unwrap(), OLD_BINARY, "ancien rétabli");
    let stored = Ledger::load(&ledger_path(&exe));
    let last = stored.last.expect("retour en arrière rapporté");
    assert_eq!(last.outcome, SelfUpdateOutcome::RolledBack);
    assert!(stored.failed.contains(&NEW_SHA.to_string()), "pas retenté en boucle");
    assert!(!updater.in_trial());
}

#[test]
fn the_platform_names_a_file_the_server_ships() {
    let platform = platform().expect("plateforme de test connue");
    let file = dumbmonit_proto::agent_file_name(&platform);
    assert!(file.starts_with("dumbmonit-agent-"), "{file}");
}
