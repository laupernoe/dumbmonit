//! Mise à jour automatique de l'agent lui-même.
//!
//! Le serveur propose, dans l'accusé de réception d'un lot, le binaire de son
//! propre build pour la plateforme de cette machine. L'agent :
//!
//! 1. décide s'il l'accepte ([`decide`]) : empreinte différente de la sienne,
//!    pas de retour en arrière non demandé, pas une version qui a déjà échoué
//!    ici, signature exigée sauf réglage explicite sur le canal chiffré ;
//! 2. le télécharge par son canal (chiffré et épinglé quand il l'a), vérifie
//!    l'empreinte SHA-256 et la signature Ed25519 ([`signature`]), puis le
//!    lance une fois (`--version`) pour s'assurer qu'il démarre ici ;
//! 3. le met en place sans jamais laisser la machine sans agent : Unix, copie
//!    de l'ancien à côté, renommage atomique, puis `exec` du nouveau dans le
//!    même processus — le gestionnaire de service ne voit rien ; Windows, un
//!    assistant (copie de l'ancien binaire, `--update-helper`) arrête le
//!    service, échange les fichiers et le redémarre ;
//! 4. garde l'ancien binaire : le nouveau est à l'essai ([`ledger`]) jusqu'à son
//!    premier lot accepté. Faute de quoi dans les deux minutes, ou s'il
//!    s'arrête à chaque démarrage, l'ancien est rétabli et l'échec rapporté au
//!    serveur, qui ne repropose pas cette version d'elle-même.
//!
//! Un agent en conteneur ne se remplace jamais : c'est l'image qu'il faut
//! mettre à jour, et l'identité le dit au serveur.

mod decide;
#[cfg(windows)]
mod helper;
pub mod ledger;
pub mod signature;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use anyhow::{Context, Result};
use dumbmonit_proto::{
    AgentUpdateOffer, SelfUpdateAbility, SelfUpdateInfo, SelfUpdateOutcome, UPDATE_TRIAL_SECS,
};
use sha2::{Digest, Sha256};
use tokio::time::Instant;
use tracing::{debug, info, warn};

pub use decide::{Current, Plan, decide};
use ledger::{Boot, Ledger, Trial};

use crate::client::PushClient;

/// Nom du registre, à côté du binaire.
const LEDGER_FILE: &str = "dumbmonit-agent.update.json";

/// Taille maximale acceptée d'un binaire téléchargé.
pub const MAX_BINARY_BYTES: u64 = 128 * 1024 * 1024;

/// Délai d'un téléchargement : un binaire de dix mégaoctets sur un lien lent.
pub const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(600);

/// Délai laissé au nouveau binaire pour répondre à `--version`.
const PREFLIGHT_TIMEOUT: Duration = Duration::from_secs(20);

/// Après un échec passager (téléchargement), pas de nouvel essai avant.
const RETRY_AFTER: Duration = Duration::from_secs(600);

/// Échecs passagers tolérés pour une même version avant de la tenir pour
/// ratée.
const MAX_TRANSIENT_FAILURES: u32 = 3;

/// Ce que l'agent fait une fois sa boucle arrêtée et ses mesures rangées.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AfterStop {
    /// Unix : remplacer le processus par ce binaire, mêmes arguments.
    Exec(PathBuf),
    /// Sortir sur ce code sans annoncer l'arrêt au gestionnaire de services
    /// Windows : il relance le service (actions de récupération).
    Exit(i32),
    /// Windows : s'arrêter normalement, l'assistant de mise à jour prend la
    /// suite.
    #[cfg_attr(not(windows), allow(dead_code))]
    Quit,
}

/// Le binaire qui tourne et son empreinte, calculés une fois.
fn own_binary() -> &'static (Option<PathBuf>, Option<String>) {
    static OWN: OnceLock<(Option<PathBuf>, Option<String>)> = OnceLock::new();
    OWN.get_or_init(|| {
        let exe = std::env::current_exe().ok().map(|exe| exe.canonicalize().unwrap_or(exe));
        let sha = exe.as_deref().and_then(|exe| std::fs::read(exe).ok()).map(|b| sha256(&b));
        (exe, sha)
    })
}

/// SHA-256 hexadécimal.
pub fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// Plateforme de ce binaire, au sens des noms de fichiers du serveur.
pub fn platform() -> Option<String> {
    let os = match std::env::consts::OS {
        os @ ("linux" | "windows" | "freebsd" | "macos") => os,
        _ => return None,
    };
    let arch = match std::env::consts::ARCH {
        arch @ ("x86_64" | "aarch64") => arch,
        _ => return None,
    };
    Some(format!("{os}-{arch}"))
}

/// Registre des mises à jour du binaire `exe`.
pub fn ledger_path(exe: &Path) -> PathBuf {
    exe.with_file_name(LEDGER_FILE)
}

/// Nouveau binaire avant sa mise en place. Sous Windows, l'extension `.exe`
/// reste en dernier : c'est elle qui le rend exécutable pour l'essai.
fn staged_path(exe: &Path) -> PathBuf {
    if cfg!(windows) {
        let stem = exe.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        exe.with_file_name(format!("{stem}.new.exe"))
    } else {
        ledger::with_suffix(exe, ".new")
    }
}

/// Copie de l'ancien binaire, gardée pour le retour en arrière.
fn backup_path(exe: &Path) -> PathBuf {
    ledger::with_suffix(exe, ".previous")
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// Vrai si l'agent tourne dans un conteneur : `DUMBMONIT_AGENT_CONTAINER`
/// (posée par l'image de l'agent), ou les marqueurs de Docker et Podman.
fn in_container() -> bool {
    let flagged = dumbmonit_proto::env::var("DUMBMONIT_AGENT_CONTAINER")
        .is_some_and(|value| !matches!(value.trim(), "" | "0" | "false" | "no"));
    flagged || Path::new("/.dockerenv").exists() || Path::new("/run/.containerenv").exists()
}

/// Vrai si l'agent peut créer un fichier dans `dir` (et l'y retirer).
fn writable(dir: &Path) -> bool {
    let probe = dir.join(".dumbmonit-agent-write-test");
    let created = std::fs::OpenOptions::new().write(true).create_new(true).open(&probe).is_ok();
    if created {
        let _ = std::fs::remove_file(&probe);
    }
    created
}

/// Capacité de ce binaire à se remplacer.
fn ability(enabled: bool, exe: Option<&Path>, container: bool) -> SelfUpdateAbility {
    if container {
        return SelfUpdateAbility::Container;
    }
    if !enabled {
        return SelfUpdateAbility::Disabled;
    }
    // macOS : binaires publiés à part, signés par Apple, que le serveur ne
    // livre pas ; le script d'installation les met à jour.
    if cfg!(target_os = "macos") || platform().is_none() {
        return SelfUpdateAbility::Unsupported;
    }
    match exe.and_then(Path::parent) {
        Some(dir) if writable(dir) => SelfUpdateAbility::Ready,
        Some(_) => SelfUpdateAbility::ReadOnly,
        None => SelfUpdateAbility::Unsupported,
    }
}

/// Au démarrage, avant même la configuration : le nouveau binaire à l'essai
/// compte ses démarrages, et rétablit l'ancien s'il en a trop fait.
///
/// Rend ce qu'il faut faire tout de suite (relancer l'ancien binaire), le cas
/// échéant.
pub fn on_start() -> Option<AfterStop> {
    let (exe, sha) = own_binary();
    let exe = exe.as_deref()?;
    let path = ledger_path(exe);
    if !path.exists() {
        return None;
    }
    let mut ledger = Ledger::load(&path);
    match ledger::on_boot(&mut ledger, sha.as_deref(), now_ms()) {
        Boot::Normal => None,
        Boot::Trial | Boot::Closed => {
            if let Err(error) = ledger.save(&path) {
                eprintln!("dumbmonit-agent: cannot record the update trial: {error:#}");
            }
            None
        }
        Boot::RollBack(reason) => {
            eprintln!("dumbmonit-agent: {reason}, restoring the previous agent");
            match ledger::roll_back(&mut ledger, &reason, now_ms()) {
                Ok(()) => {
                    if let Err(error) = ledger.save(&path) {
                        eprintln!("dumbmonit-agent: cannot record the roll-back: {error:#}");
                    }
                    Some(restart_into(exe))
                }
                Err(error) => {
                    eprintln!("dumbmonit-agent: roll-back failed: {error:#}");
                    None
                }
            }
        }
    }
}

/// Redémarrage sur le binaire `exe`, tel que la plateforme le permet.
fn restart_into(exe: &Path) -> AfterStop {
    if cfg!(windows) { AfterStop::Exit(1) } else { AfterStop::Exec(exe.to_path_buf()) }
}

/// Exécute ce qui suit l'arrêt de la boucle.
pub fn after_stop(action: AfterStop) -> Result<()> {
    match action {
        AfterStop::Quit => Ok(()),
        AfterStop::Exit(code) => std::process::exit(code),
        AfterStop::Exec(binary) => exec(&binary),
    }
}

#[cfg(unix)]
fn exec(binary: &Path) -> Result<()> {
    use std::os::unix::process::CommandExt as _;
    info!(binary = %binary.display(), "restarting the agent");
    let error = std::process::Command::new(binary).args(std::env::args_os().skip(1)).exec();
    Err(anyhow::Error::new(error).context("restarting the agent"))
}

#[cfg(not(unix))]
fn exec(_binary: &Path) -> Result<()> {
    std::process::exit(1)
}

/// `--update-helper <registre>` : l'assistant Windows de mise à jour.
pub fn run_helper(ledger: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        helper::run(ledger)
    }
    #[cfg(not(windows))]
    {
        let _ = ledger;
        anyhow::bail!("--update-helper is only used on Windows")
    }
}

/// Échec d'une tentative.
#[derive(Debug)]
enum Attempt {
    /// Passager (réseau) : nouvel essai plus tard.
    Transient(String),
    /// Définitif pour cette version.
    Fatal(String),
}

/// État de la mise à jour automatique pour la boucle de l'agent.
pub struct SelfUpdater {
    exe: Option<PathBuf>,
    sha256: Option<String>,
    platform: Option<String>,
    ability: SelfUpdateAbility,
    ledger_path: Option<PathBuf>,
    ledger: Ledger,
    keys: Vec<[u8; 32]>,
    /// Échéance de l'essai en cours, depuis ce démarrage.
    trial_deadline: Option<Instant>,
    retry_at: Option<Instant>,
    /// Échecs passagers de la version visée.
    transient: Option<(String, u32)>,
    /// Faux en test : l'essai `--version` lancerait un faux binaire.
    preflight: bool,
}

impl SelfUpdater {
    /// Pour le binaire qui tourne.
    pub fn new(enabled: bool) -> Self {
        let (exe, sha) = own_binary();
        Self::with(exe.clone(), sha.clone(), enabled, in_container())
    }

    /// Pour un binaire donné (tests).
    pub fn with(
        exe: Option<PathBuf>,
        sha256: Option<String>,
        enabled: bool,
        container: bool,
    ) -> Self {
        let ability = ability(enabled, exe.as_deref(), container);
        let ledger_path = exe.as_deref().map(ledger_path);
        let ledger = ledger_path.as_deref().map(Ledger::load).unwrap_or_default();
        let grace = if cfg!(windows) {
            // L'assistant Windows surveille l'essai et tranche le premier ;
            // l'agent ne prend le relais que s'il a disparu.
            Duration::from_secs(UPDATE_TRIAL_SECS + 90)
        } else {
            Duration::from_secs(UPDATE_TRIAL_SECS)
        };
        // À l'essai seulement si c'est bien le binaire essayé qui tourne : une
        // réinstallation à la main pendant l'essai n'a rien à confirmer.
        let on_trial = ledger.trial.as_ref().is_some_and(|trial| {
            sha256.as_deref().is_some_and(|sha| sha.eq_ignore_ascii_case(&trial.to_sha256))
        });
        let trial_deadline = on_trial.then(|| Instant::now() + grace);
        Self {
            exe,
            sha256,
            platform: platform(),
            ability,
            ledger_path,
            ledger,
            keys: signature::release_keys(),
            trial_deadline,
            retry_at: None,
            transient: None,
            preflight: true,
        }
    }

    /// Ce que l'identité annonce.
    pub fn info(&self) -> SelfUpdateInfo {
        SelfUpdateInfo {
            sha256: self.sha256.clone(),
            platform: self.platform.clone(),
            ability: self.ability,
            signing_keys: !self.keys.is_empty(),
            last: self.ledger.last.clone(),
        }
    }

    /// Échéance de l'essai en cours, s'il y en a un.
    pub fn trial_deadline(&self) -> Option<Instant> {
        self.trial_deadline
    }

    pub fn in_trial(&self) -> bool {
        self.trial_deadline.is_some()
    }

    fn save(&self) {
        let Some(path) = &self.ledger_path else { return };
        if let Err(error) = self.ledger.save(path) {
            warn!(error = %format!("{error:#}"), "cannot record the agent update");
        }
    }

    /// Premier lot accepté : l'essai est concluant. Rend vrai si quelque chose
    /// a changé.
    pub fn confirm(&mut self) -> bool {
        if self.trial_deadline.take().is_none() {
            return false;
        }
        let Some(trial) = self.ledger.trial.clone() else { return false };
        info!(from = trial.from_version, to = trial.to_version, "agent update confirmed");
        self.ledger.close(ledger::report(
            SelfUpdateOutcome::Succeeded,
            &trial.from_version,
            &trial.to_version,
            &trial.to_sha256,
            None,
            now_ms(),
        ));
        self.save();
        true
    }

    /// Échéance de l'essai sans lot accepté : l'ancien binaire est rétabli.
    pub fn expire_trial(&mut self) -> Option<AfterStop> {
        self.trial_deadline = None;
        let exe = self.exe.clone()?;
        let reason = format!(
            "the new agent had no batch accepted by the server within {UPDATE_TRIAL_SECS} s"
        );
        warn!(reason, "restoring the previous agent");
        if let Err(error) = ledger::roll_back(&mut self.ledger, &reason, now_ms()) {
            warn!(error = %format!("{error:#}"), "cannot restore the previous agent");
            return None;
        }
        self.save();
        Some(restart_into(&exe))
    }

    /// Examine une offre du serveur ; rend ce qu'il faut faire après l'arrêt
    /// quand le nouveau binaire est en place.
    pub async fn consider(
        &mut self,
        offer: &AgentUpdateOffer,
        client: &PushClient,
        key: &str,
    ) -> Option<AfterStop> {
        if self.in_trial() || self.retry_at.is_some_and(|at| Instant::now() < at) {
            return None;
        }
        let current = Current {
            version: env!("CARGO_PKG_VERSION"),
            sha256: self.sha256.as_deref(),
            ability: self.ability,
            failed: &self.ledger.failed,
            encrypted: client.is_encrypted(),
            has_keys: !self.keys.is_empty(),
        };
        match decide(offer, &current) {
            Plan::Skip(why) => {
                debug!(why, version = offer.version, "agent update not applied");
                None
            }
            Plan::Refuse(reason) => {
                self.refuse(offer, reason);
                None
            }
            Plan::Install => {
                info!(from = env!("CARGO_PKG_VERSION"), to = offer.version, "updating the agent");
                match self.install(offer, client, key).await {
                    Ok(after) => Some(after),
                    Err(Attempt::Transient(reason)) => {
                        self.transient_failure(offer, reason);
                        None
                    }
                    Err(Attempt::Fatal(reason)) => {
                        self.fail(offer, reason);
                        None
                    }
                }
            }
        }
    }

    /// Refus rapporté une fois par version et par cause.
    fn refuse(&mut self, offer: &AgentUpdateOffer, reason: String) {
        let already = self.ledger.last.as_ref().is_some_and(|last| {
            last.outcome == SelfUpdateOutcome::Refused
                && last.to_sha256.eq_ignore_ascii_case(&offer.sha256)
                && last.reason.as_deref() == Some(reason.as_str())
        });
        if already {
            debug!(reason, "agent update still refused");
            return;
        }
        warn!(version = offer.version, reason, "agent update refused");
        self.close(SelfUpdateOutcome::Refused, offer, reason);
    }

    fn fail(&mut self, offer: &AgentUpdateOffer, reason: String) {
        warn!(version = offer.version, reason, "agent update failed, keeping this version");
        self.close(SelfUpdateOutcome::Failed, offer, reason);
    }

    fn close(&mut self, outcome: SelfUpdateOutcome, offer: &AgentUpdateOffer, reason: String) {
        self.ledger.close(ledger::report(
            outcome,
            env!("CARGO_PKG_VERSION"),
            &offer.version,
            &offer.sha256,
            Some(reason),
            now_ms(),
        ));
        self.save();
    }

    fn transient_failure(&mut self, offer: &AgentUpdateOffer, reason: String) {
        let count = match &self.transient {
            Some((sha, count)) if sha.eq_ignore_ascii_case(&offer.sha256) => count + 1,
            _ => 1,
        };
        self.transient = Some((offer.sha256.clone(), count));
        if count >= MAX_TRANSIENT_FAILURES {
            self.fail(offer, format!("{reason} ({count} attempts)"));
            return;
        }
        self.retry_at = Some(Instant::now() + RETRY_AFTER);
        warn!(
            version = offer.version,
            reason,
            retry_in_s = RETRY_AFTER.as_secs(),
            "agent update postponed"
        );
    }

    /// Télécharge, vérifie et met en place le nouveau binaire.
    async fn install(
        &mut self,
        offer: &AgentUpdateOffer,
        client: &PushClient,
        key: &str,
    ) -> Result<AfterStop, Attempt> {
        let exe = self.exe.clone().ok_or_else(|| Attempt::Fatal("unknown binary path".into()))?;
        let bytes = client
            .download_update(key, &offer.file, MAX_BINARY_BYTES, DOWNLOAD_TIMEOUT)
            .await
            .map_err(|error| Attempt::Transient(format!("download failed: {error}")))?;
        let actual = sha256(&bytes);
        if !actual.eq_ignore_ascii_case(&offer.sha256) {
            return Err(Attempt::Transient(format!(
                "the downloaded binary does not match the announced SHA-256 (got {actual})"
            )));
        }
        match signature::verify(&bytes, offer.signature.as_deref(), &self.keys) {
            signature::Verdict::Valid => info!("agent update signature verified"),
            signature::Verdict::Invalid => {
                return Err(Attempt::Fatal(
                    "the signature does not match any release key of this agent".into(),
                ));
            }
            // Admis par `decide` : réglage du serveur, canal chiffré.
            signature::Verdict::Unsigned => {
                warn!("installing an unsigned agent build (allowed by the server)")
            }
        }
        let staged = staged_path(&exe);
        stage(&staged, &bytes).map_err(|error| Attempt::Fatal(format!("{error:#}")))?;
        if self.preflight
            && let Err(reason) = preflight(&staged, &offer.version).await
        {
            let _ = std::fs::remove_file(&staged);
            return Err(Attempt::Fatal(reason));
        }
        let trial = Trial {
            from_version: env!("CARGO_PKG_VERSION").to_string(),
            from_sha256: self.sha256.clone().unwrap_or_default(),
            to_version: offer.version.clone(),
            to_sha256: offer.sha256.to_ascii_lowercase(),
            binary: exe.clone(),
            backup: backup_path(&exe),
            staged: None,
            started_ms: now_ms(),
            boots: 0,
        };
        self.put_in_place(trial, staged)
            .map_err(|error| Attempt::Fatal(format!("cannot install the new binary: {error:#}")))
    }

    /// Unix : ancien binaire copié à côté, registre écrit, renommage atomique.
    #[cfg(not(windows))]
    fn put_in_place(&mut self, trial: Trial, staged: PathBuf) -> Result<AfterStop> {
        copy_synced(&trial.binary, &trial.backup)?;
        let exe = trial.binary.clone();
        self.ledger.trial = Some(trial);
        self.save();
        if let Err(error) = std::fs::rename(&staged, &exe) {
            self.ledger.trial = None;
            self.save();
            let _ = std::fs::remove_file(&staged);
            return Err(anyhow::Error::new(error).context("replacing the binary"));
        }
        ledger::sync_parent(&exe);
        Ok(AfterStop::Exec(exe))
    }

    /// Windows : le binaire en service ne peut pas être remplacé ; l'assistant
    /// (copie de ce binaire) arrête le service, échange, redémarre, surveille.
    #[cfg(windows)]
    fn put_in_place(&mut self, mut trial: Trial, staged: PathBuf) -> Result<AfterStop> {
        let assistant = trial.binary.with_file_name("dumbmonit-agent-update-helper.exe");
        copy_synced(&trial.binary, &assistant)?;
        trial.staged = Some(staged.clone());
        self.ledger.trial = Some(trial);
        self.save();
        let path = self.ledger_path.clone().context("no update ledger")?;
        if let Err(error) = helper::spawn(&assistant, &path) {
            self.ledger.trial = None;
            self.save();
            let _ = std::fs::remove_file(&staged);
            return Err(error);
        }
        Ok(AfterStop::Quit)
    }

    /// Désactive l'essai `--version` (tests).
    #[cfg(test)]
    pub fn without_preflight(mut self) -> Self {
        self.preflight = false;
        self
    }

    /// Remplace les clés de publication (tests).
    #[cfg(test)]
    pub fn with_keys(mut self, keys: Vec<[u8; 32]>) -> Self {
        self.keys = keys;
        self
    }
}

/// Écrit le nouveau binaire, exécutable, synchronisé sur disque.
fn stage(path: &Path, bytes: &[u8]) -> Result<()> {
    let _ = std::fs::remove_file(path);
    ledger::write_synced(path, bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
            .with_context(|| format!("making {} executable", path.display()))?;
    }
    Ok(())
}

/// Copie synchronisée sur disque (permissions comprises).
fn copy_synced(from: &Path, to: &Path) -> Result<()> {
    std::fs::copy(from, to)
        .with_context(|| format!("copying {} to {}", from.display(), to.display()))?;
    std::fs::File::open(to).and_then(|file| file.sync_all()).context("syncing the copy")?;
    Ok(())
}

/// Lance le nouveau binaire une fois (`--version`) : un binaire qui ne démarre
/// pas ici (architecture, bibliothèque manquante) est écarté avant d'avoir
/// remplacé quoi que ce soit.
async fn preflight(binary: &Path, version: &str) -> Result<(), String> {
    let run = tokio::process::Command::new(binary)
        .arg("--version")
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true)
        .output();
    let output = tokio::time::timeout(PREFLIGHT_TIMEOUT, run)
        .await
        .map_err(|_| "the new binary did not answer --version in time".to_string())?
        .map_err(|error| format!("the new binary does not start on this machine: {error}"))?;
    let printed = String::from_utf8_lossy(&output.stdout);
    let expected = format!("dumbmonit-agent {version}");
    if output.status.success() && printed.lines().any(|line| line.trim() == expected) {
        Ok(())
    } else {
        Err(format!(
            "the new binary does not report version {version} (printed '{}')",
            printed.trim()
        ))
    }
}

#[cfg(test)]
mod tests;
