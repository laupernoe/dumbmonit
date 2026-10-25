//! Registre des mises à jour, rangé à côté du binaire.
//!
//! Il survit au remplacement du binaire et au redémarrage : c'est lui qui dit
//! au nouveau binaire qu'il est à l'essai, à l'ancien rétabli ce qui s'est
//! passé, et au serveur l'issue de la dernière tentative. À côté du binaire,
//! et non de la configuration, parce que c'est le seul dossier que le
//! remplacement exige de toute façon inscriptible — et que le nouveau binaire
//! le lit avant même d'avoir chargé sa configuration.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use dumbmonit_proto::{SelfUpdateOutcome, SelfUpdateReport};
use serde::{Deserialize, Serialize};

/// Démarrages du nouveau binaire tolérés sans lot accepté : au-delà, il
/// s'arrête au démarrage, l'ancien est rétabli.
pub const MAX_TRIAL_BOOTS: u32 = 3;

/// Empreintes en échec gardées en mémoire, au plus.
const MAX_FAILED: usize = 10;

/// Une mise à jour à l'essai : le nouveau binaire est en place, l'ancien
/// attend à côté.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trial {
    pub from_version: String,
    pub from_sha256: String,
    pub to_version: String,
    pub to_sha256: String,
    /// Binaire en service.
    pub binary: PathBuf,
    /// Copie de l'ancien binaire.
    pub backup: PathBuf,
    /// Nouveau binaire avant le remplacement (Windows : c'est l'assistant qui
    /// le met en place, service arrêté).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub staged: Option<PathBuf>,
    pub started_ms: i64,
    /// Démarrages du nouveau binaire depuis le remplacement.
    #[serde(default)]
    pub boots: u32,
}

/// Le registre complet.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ledger {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trial: Option<Trial>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last: Option<SelfUpdateReport>,
    /// Empreintes qui ont échoué ici : pas retentées d'elles-mêmes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failed: Vec<String>,
}

impl Ledger {
    /// Lit le registre ; absent ou illisible, un registre vide.
    pub fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    /// Écrit le registre : fichier temporaire, synchronisé, puis renommé —
    /// un arrêt brutal laisse l'ancien ou le nouveau, jamais un mélange.
    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_vec_pretty(self).context("encoding the update ledger")?;
        let temporary = path.with_extension("json.tmp");
        write_synced(&temporary, &json)?;
        std::fs::rename(&temporary, path).with_context(|| format!("replacing {}", path.display()))?;
        sync_parent(path);
        Ok(())
    }

    /// Clôt la tentative en cours (ou en note une sans essai, refus ou échec
    /// avant remplacement).
    pub fn close(&mut self, report: SelfUpdateReport) {
        self.trial = None;
        if report.outcome != SelfUpdateOutcome::Succeeded
            && report.outcome != SelfUpdateOutcome::Refused
            && !self.failed.iter().any(|sha| sha.eq_ignore_ascii_case(&report.to_sha256))
        {
            self.failed.push(report.to_sha256.to_ascii_lowercase());
            if self.failed.len() > MAX_FAILED {
                self.failed.remove(0);
            }
        }
        self.last = Some(report);
    }
}

/// Compte rendu d'une tentative.
pub fn report(
    outcome: SelfUpdateOutcome,
    from_version: &str,
    to_version: &str,
    to_sha256: &str,
    reason: Option<String>,
    at_ms: i64,
) -> SelfUpdateReport {
    SelfUpdateReport {
        outcome,
        from_version: from_version.to_string(),
        to_version: to_version.to_string(),
        to_sha256: to_sha256.to_ascii_lowercase(),
        reason,
        at_ms,
    }
}

/// Ce que le démarrage doit faire, d'après le registre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Boot {
    /// Rien en cours.
    Normal,
    /// Nouveau binaire à l'essai : il a jusqu'à l'échéance pour faire accepter
    /// un lot.
    Trial,
    /// Le nouveau binaire s'est arrêté trop souvent : rétablir l'ancien.
    RollBack(String),
    /// Le registre a changé (essai clos sans suite) : à réécrire.
    Closed,
}

/// Démarrage d'un binaire d'empreinte `own_sha256`, d'après le registre, qui
/// est mis à jour en conséquence.
pub fn on_boot(ledger: &mut Ledger, own_sha256: Option<&str>, now_ms: i64) -> Boot {
    let Some(trial) = ledger.trial.as_mut() else { return Boot::Normal };
    let own = own_sha256.unwrap_or_default();
    if own.eq_ignore_ascii_case(&trial.to_sha256) {
        trial.boots += 1;
        if trial.boots > MAX_TRIAL_BOOTS {
            return Boot::RollBack(format!(
                "the new agent stopped at startup {} times without reporting",
                trial.boots - 1
            ));
        }
        return Boot::Trial;
    }
    // L'ancien binaire tourne encore, ou de nouveau : le remplacement n'a pas
    // eu lieu (assistant Windows interrompu, échec du renommage).
    let trial = trial.clone();
    let reason = if own.eq_ignore_ascii_case(&trial.from_sha256) {
        "the new binary was never started"
    } else {
        "another binary is running than the one installed"
    };
    ledger.close(report(
        SelfUpdateOutcome::Failed,
        &trial.from_version,
        &trial.to_version,
        &trial.to_sha256,
        Some(reason.to_string()),
        now_ms,
    ));
    Boot::Closed
}

/// Rétablit l'ancien binaire et clôt l'essai sur un retour en arrière.
///
/// Unix : un renommage atomique remplace le binaire. Windows : le binaire en
/// cours d'exécution ne peut pas être écrasé, mais il peut être renommé — il
/// est d'abord mis de côté, l'ancien reprend sa place.
pub fn roll_back(ledger: &mut Ledger, reason: &str, now_ms: i64) -> Result<()> {
    let trial = ledger.trial.clone().context("no update on trial")?;
    if cfg!(windows) {
        let aside = with_suffix(&trial.binary, ".failed");
        let _ = std::fs::remove_file(&aside);
        std::fs::rename(&trial.binary, &aside).context("moving the new binary aside")?;
    }
    std::fs::rename(&trial.backup, &trial.binary).context("restoring the previous binary")?;
    sync_parent(&trial.binary);
    ledger.close(report(
        SelfUpdateOutcome::RolledBack,
        &trial.from_version,
        &trial.to_version,
        &trial.to_sha256,
        Some(reason.to_string()),
        now_ms,
    ));
    Ok(())
}

/// `chemin` + suffixe ajouté au nom complet : `agent.exe` → `agent.exe.new`.
pub fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    name.push(suffix);
    path.with_file_name(name)
}

/// Écrit un fichier et le synchronise sur disque avant de rendre la main.
pub fn write_synced(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write as _;
    let mut file = std::fs::File::create(path)
        .with_context(|| format!("creating {}", path.display()))?;
    file.write_all(bytes).with_context(|| format!("writing {}", path.display()))?;
    file.sync_all().with_context(|| format!("syncing {}", path.display()))?;
    Ok(())
}

/// Synchronise le dossier d'un fichier qu'on vient de renommer (Unix) : sans
/// cela, un arrêt brutal pourrait perdre le renommage lui-même.
pub fn sync_parent(path: &Path) {
    #[cfg(unix)]
    if let Some(dir) = path.parent()
        && let Ok(dir) = std::fs::File::open(dir)
    {
        let _ = dir.sync_all();
    }
    #[cfg(not(unix))]
    let _ = path;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trial(dir: &Path) -> Trial {
        Trial {
            from_version: "0.13.5".into(),
            from_sha256: "a".repeat(64),
            to_version: "0.14.0".into(),
            to_sha256: "b".repeat(64),
            binary: dir.join("dumbmonit-agent"),
            backup: dir.join("dumbmonit-agent.previous"),
            staged: None,
            started_ms: 1,
            boots: 0,
        }
    }

    #[test]
    fn the_ledger_survives_a_round_trip_and_a_missing_file_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dumbmonit-agent.update.json");
        assert_eq!(Ledger::load(&path), Ledger::default());
        let ledger = Ledger { trial: Some(trial(dir.path())), ..Ledger::default() };
        ledger.save(&path).unwrap();
        assert_eq!(Ledger::load(&path), ledger);
        std::fs::write(&path, b"{ abime").unwrap();
        assert_eq!(Ledger::load(&path), Ledger::default(), "illisible : vide");
    }

    #[test]
    fn the_new_binary_gets_a_few_starts_then_is_rolled_back() {
        let dir = tempfile::tempdir().unwrap();
        let mut ledger = Ledger { trial: Some(trial(dir.path())), ..Ledger::default() };
        let new = "b".repeat(64);
        for _ in 0..MAX_TRIAL_BOOTS {
            assert_eq!(on_boot(&mut ledger, Some(&new), 2), Boot::Trial);
        }
        assert!(matches!(on_boot(&mut ledger, Some(&new), 2), Boot::RollBack(_)));
    }

    #[test]
    fn the_old_binary_still_running_closes_the_trial_as_failed() {
        let dir = tempfile::tempdir().unwrap();
        let mut ledger = Ledger { trial: Some(trial(dir.path())), ..Ledger::default() };
        assert_eq!(on_boot(&mut ledger, Some(&"a".repeat(64)), 5), Boot::Closed);
        assert!(ledger.trial.is_none());
        let last = ledger.last.clone().unwrap();
        assert_eq!(last.outcome, SelfUpdateOutcome::Failed);
        assert_eq!(last.at_ms, 5);
        assert_eq!(ledger.failed, vec!["b".repeat(64)], "pas retentée d'elle-même");
        assert_eq!(on_boot(&mut ledger, Some(&"a".repeat(64)), 6), Boot::Normal);
    }

    #[test]
    fn a_roll_back_puts_the_previous_binary_back_and_remembers_the_failure() {
        let dir = tempfile::tempdir().unwrap();
        let trial = trial(dir.path());
        std::fs::write(&trial.binary, b"nouveau").unwrap();
        std::fs::write(&trial.backup, b"ancien").unwrap();
        let mut ledger = Ledger { trial: Some(trial.clone()), ..Ledger::default() };

        roll_back(&mut ledger, "no batch accepted within 120 s", 9).unwrap();

        assert_eq!(std::fs::read(&trial.binary).unwrap(), b"ancien");
        let last = ledger.last.clone().unwrap();
        assert_eq!(last.outcome, SelfUpdateOutcome::RolledBack);
        assert_eq!(last.reason.as_deref(), Some("no batch accepted within 120 s"));
        assert!(ledger.trial.is_none());
        assert!(ledger.failed.contains(&"b".repeat(64)));
    }

    #[test]
    fn a_refusal_is_reported_but_not_remembered_as_a_failure() {
        // Un refus (non signé, retour en arrière) se réévalue à chaque offre :
        // le réglage du serveur peut changer entre-temps.
        let mut ledger = Ledger::default();
        ledger.close(report(SelfUpdateOutcome::Refused, "1.0.0", "1.1.0", "C", None, 1));
        assert!(ledger.failed.is_empty());
        assert_eq!(ledger.last.as_ref().unwrap().to_sha256, "c");
        for n in 0..(MAX_FAILED + 3) {
            let sha = format!("{n:064}");
            ledger.close(report(SelfUpdateOutcome::Failed, "1.0.0", "1.1.0", &sha, None, 1));
        }
        assert_eq!(ledger.failed.len(), MAX_FAILED, "liste bornée");
    }

    #[test]
    fn suffixes_are_added_to_the_whole_file_name() {
        assert_eq!(
            with_suffix(Path::new("/opt/dm/dumbmonit-agent.exe"), ".new"),
            PathBuf::from("/opt/dm/dumbmonit-agent.exe.new")
        );
    }
}
