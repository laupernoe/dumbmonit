//! Actions de remédiation exécutées par l'agent : redémarrer un service, lancer
//! un script déclaré localement.
//!
//! Tout est refusé par défaut (`actions: false`). Même activées, ces actions
//! restent bornées par ce que l'administrateur de *la machine* a écrit :
//!
//! - un service ne se redémarre que s'il figure dans `action_services` ;
//! - un script ne s'exécute que s'il a un manifeste dans le dossier d'actions
//!   (`/etc/dumbmonit/actions.d/<nom>.yaml`, `C:\ProgramData\DumbMonit\actions.d`
//!   sous Windows). Le serveur n'envoie qu'un nom et des paramètres ; la
//!   commande, elle, n'existe que dans le manifeste.
//!
//! Aucune action ne passe par un interpréteur : le programme et ses arguments
//! sont donnés tels quels au système (`argv`), chaque paramètre devient un seul
//! argument `--clé=valeur`, l'environnement est vidé, l'entrée standard fermée,
//! la durée bornée et la sortie tronquée.

use std::collections::{BTreeMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use dumbmonit_proto::remediation::{
    self as rules, ACTION_TTL_SECS, ARG_EXPIRES_AT_MS, ARG_NONCE, SCRIPT_DEFAULT_TIMEOUT_SECS,
    SCRIPT_MAX_TIMEOUT_SECS,
};
use dumbmonit_proto::{AgentCommand, DeclaredScript, RemediationInfo};
use serde::Deserialize;
use tokio::io::{AsyncRead, AsyncReadExt};
use tracing::warn;

/// Délai d'un redémarrage de service : `systemctl restart` attend que l'unité
/// soit relancée, ce qui peut prendre une minute pour une base de données.
const SERVICE_TIMEOUT: Duration = Duration::from_secs(120);

/// Octets lus au plus sur chaque flux d'un processus ; le reste est lu et jeté,
/// pour que le processus ne se bloque pas sur un tuyau plein.
const CAPTURE_MAX: usize = 16 * 1024;

/// Taille maximale d'un manifeste : un fichier plus gros n'en est pas un.
const MANIFEST_MAX_BYTES: u64 = 16 * 1024;

/// Nombre de nonces retenus contre le rejeu. Une demande vit dix minutes ; mille
/// demandes en dix minutes sur une seule machine, c'est déjà une attaque.
const REPLAY_MEMORY: usize = 1024;

/// Tolérance d'horloge entre serveur et agent, en millisecondes.
const CLOCK_SKEW_MS: i64 = 120_000;

/// Réglages locaux des actions de remédiation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionsConfig {
    /// `actions: true` : sans lui, aucun service ni script.
    pub enabled: bool,
    /// Dossier des manifestes de scripts.
    pub dir: PathBuf,
    /// Services que le serveur peut faire redémarrer (`*` : tous).
    pub services: Vec<String>,
}

impl Default for ActionsConfig {
    fn default() -> Self {
        Self { enabled: false, dir: default_dir(None), services: Vec::new() }
    }
}

/// Dossier d'actions par défaut : `actions.d` à côté de agent.yaml.
pub fn default_dir(config_path: Option<&Path>) -> PathBuf {
    let base = config_path
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .or_else(|| crate::config::Config::default_path().parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("/etc/dumbmonit"));
    base.join("actions.d")
}

/// Manifeste d'un script, tel qu'écrit par l'administrateur de la machine.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Manifest {
    pub description: String,
    /// Programme (chemin absolu) puis arguments fixes. Jamais interprété.
    pub command: Vec<String>,
    pub timeout_secs: Option<u64>,
    /// Paramètre → valeurs permises.
    pub params: BTreeMap<String, Vec<String>>,
}

/// Un script retenu : son nom et son manifeste vérifié.
#[derive(Debug, Clone)]
pub struct Script {
    pub name: String,
    pub manifest: Manifest,
}

impl Script {
    fn timeout(&self) -> Duration {
        Duration::from_secs(
            self.manifest
                .timeout_secs
                .unwrap_or(SCRIPT_DEFAULT_TIMEOUT_SECS)
                .clamp(1, SCRIPT_MAX_TIMEOUT_SECS),
        )
    }

    fn declared(&self) -> DeclaredScript {
        DeclaredScript {
            name: self.name.clone(),
            description: self.manifest.description.clone(),
            params: self.manifest.params.clone(),
            timeout_secs: self.timeout().as_secs(),
        }
    }
}

/// Ce que l'agent déclare au serveur. Les scripts sont relus du disque : un
/// manifeste invalide est écarté (et journalisé), jamais à moitié retenu.
pub fn describe(config: &ActionsConfig) -> RemediationInfo {
    if !config.enabled {
        return RemediationInfo::default();
    }
    RemediationInfo {
        enabled: true,
        services: config.services.clone(),
        scripts: list_scripts(&config.dir).iter().map(Script::declared).collect(),
    }
    .sanitized()
}

/// Les scripts valides du dossier, par ordre de nom.
pub fn list_scripts(dir: &Path) -> Vec<Script> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    if let Err(error) = check_not_writable_by_others(dir) {
        warn!(dir = %dir.display(), %error, "actions directory ignored");
        return Vec::new();
    }
    let mut names: Vec<String> = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter_map(|file| file.strip_suffix(".yaml").map(str::to_string))
        .filter(|name| rules::validate_action_name(name).is_ok())
        .collect();
    names.sort();
    names.truncate(rules::MAX_DECLARED_SCRIPTS);
    names
        .into_iter()
        .filter_map(|name| match load_script(dir, &name) {
            Ok(script) => Some(script),
            Err(error) => {
                warn!(script = name, error = format!("{error:#}"), "action manifest ignored");
                None
            }
        })
        .collect()
}

/// Charge le manifeste d'un nom donné.
///
/// Le nom est d'abord validé (ni `/`, ni `\`, ni `.`), puis comparé aux entrées
/// réellement présentes dans le dossier : le chemin final n'est jamais construit
/// à partir d'une chaîne qui n'a pas été trouvée telle quelle sur le disque.
pub fn load_script(dir: &Path, name: &str) -> Result<Script> {
    rules::validate_action_name(name).map_err(anyhow::Error::msg)?;
    check_not_writable_by_others(dir)?;
    let file_name = format!("{name}.yaml");
    let listed = std::fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .filter_map(|entry| entry.ok())
        .any(|entry| entry.file_name().to_str() == Some(file_name.as_str()));
    if !listed {
        bail!("no action named '{name}' is declared on this machine");
    }
    let path = dir.join(&file_name);
    // Pas de lien symbolique : le manifeste est ce fichier-là, pas ce qu'un
    // lien désignerait demain.
    let meta = std::fs::symlink_metadata(&path).with_context(|| format!("{}", path.display()))?;
    if !meta.file_type().is_file() {
        bail!("{} is not a regular file", path.display());
    }
    if meta.len() > MANIFEST_MAX_BYTES {
        bail!("{} is too large for a manifest", path.display());
    }
    check_not_writable_by_others(&path)?;
    let text = std::fs::read_to_string(&path).with_context(|| format!("{}", path.display()))?;
    let manifest: Manifest = dumbmonit_collectors::yaml::from_str(&text)
        .with_context(|| format!("unreadable manifest {}", path.display()))?;
    validate_manifest(&manifest)?;
    Ok(Script { name: name.to_string(), manifest })
}

fn validate_manifest(manifest: &Manifest) -> Result<()> {
    let Some(program) = manifest.command.first() else {
        bail!("'command' is empty");
    };
    let program_path = Path::new(program);
    if !program_path.is_absolute() {
        bail!("the program '{program}' must be an absolute path");
    }
    if manifest.command.iter().any(|arg| arg.contains('\0')) {
        bail!("an argument contains a NUL byte");
    }
    let meta = std::fs::metadata(program_path).with_context(|| format!("program {program}"))?;
    if !meta.is_file() {
        bail!("the program '{program}' is not a file");
    }
    check_not_writable_by_others(program_path)?;
    if manifest.params.len() > rules::MAX_SCRIPT_PARAMS {
        bail!("too many parameters (max {})", rules::MAX_SCRIPT_PARAMS);
    }
    for (key, choices) in &manifest.params {
        rules::validate_param_key(key).map_err(anyhow::Error::msg)?;
        if choices.is_empty() || choices.len() > rules::MAX_PARAM_CHOICES {
            bail!("parameter '{key}' needs between 1 and {} values", rules::MAX_PARAM_CHOICES);
        }
        for choice in choices {
            rules::validate_param_value(choice).map_err(anyhow::Error::msg)?;
        }
    }
    Ok(())
}

/// Refuse un fichier ou un dossier que d'autres comptes que son propriétaire
/// peuvent modifier, ou dont le propriétaire n'est ni `root` ni l'agent : un
/// manifeste réinscriptible par n'importe qui serait une porte ouverte.
#[cfg(unix)]
fn check_not_writable_by_others(path: &Path) -> Result<()> {
    use std::os::unix::fs::MetadataExt;
    let meta = std::fs::metadata(path).with_context(|| format!("{}", path.display()))?;
    if meta.mode() & 0o022 != 0 {
        bail!("{} is writable by group or others (chmod go-w)", path.display());
    }
    if let Some(uid) = current_uid()
        && meta.uid() != 0
        && meta.uid() != uid
    {
        bail!("{} is owned by another account (uid {})", path.display(), meta.uid());
    }
    Ok(())
}

/// Sous Windows, les droits passent par des ACL que la bibliothèque standard ne
/// lit pas : la documentation demande de réserver le dossier aux
/// administrateurs, ce que fait l'installateur pour `C:\ProgramData\DumbMonit`.
#[cfg(not(unix))]
fn check_not_writable_by_others(path: &Path) -> Result<()> {
    std::fs::metadata(path).with_context(|| format!("{}", path.display()))?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn current_uid() -> Option<u32> {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata("/proc/self").ok().map(|meta| meta.uid())
}

#[cfg(all(unix, not(target_os = "linux")))]
fn current_uid() -> Option<u32> {
    None
}

// ------------------------------------------------------------ rejeu

/// Mémoire des demandes déjà acceptées : nonce et échéance.
///
/// Une demande de remédiation porte un nonce tiré au hasard par le serveur et
/// une échéance absolue. L'agent refuse une demande échue, une échéance trop
/// lointaine (un serveur ne la fixe jamais à plus de dix minutes) et un nonce
/// déjà vu : rejouer une réponse captée ne fait rien.
#[derive(Debug, Default)]
pub struct ReplayGuard {
    seen: VecDeque<(String, i64)>,
    index: HashSet<String>,
}

impl ReplayGuard {
    /// Vérifie et retient l'enveloppe d'une commande. `required` : une commande
    /// de service ou de script sans enveloppe est refusée ; une commande de
    /// conteneur d'un serveur antérieur n'en a pas, et reste acceptée.
    pub fn admit(&mut self, command: &AgentCommand, now_ms: i64, required: bool) -> Result<()> {
        self.forget_expired(now_ms);
        let nonce = command.arg_str(ARG_NONCE);
        let Some(nonce) = nonce else {
            if required {
                bail!("refused: the request carries no nonce");
            }
            return Ok(());
        };
        rules::validate_nonce(nonce).map_err(anyhow::Error::msg)?;
        let expires = command
            .args
            .get(ARG_EXPIRES_AT_MS)
            .and_then(|value| value.as_i64())
            .context("refused: the request carries no expiry")?;
        if expires <= now_ms {
            bail!("refused: the request expired");
        }
        if expires > now_ms + ACTION_TTL_SECS * 1000 + CLOCK_SKEW_MS {
            bail!("refused: the request expires too far in the future");
        }
        if self.index.contains(nonce) {
            bail!("refused: this request was already executed (replay)");
        }
        self.index.insert(nonce.to_string());
        self.seen.push_back((nonce.to_string(), expires));
        while self.seen.len() > REPLAY_MEMORY {
            if let Some((old, _)) = self.seen.pop_front() {
                self.index.remove(&old);
            }
        }
        Ok(())
    }

    fn forget_expired(&mut self, now_ms: i64) {
        while self.seen.front().is_some_and(|(_, expires)| *expires + CLOCK_SKEW_MS < now_ms) {
            if let Some((old, _)) = self.seen.pop_front() {
                self.index.remove(&old);
            }
        }
    }
}

// ------------------------------------------------------------ exécution

/// Issue d'un processus : succès, et sortie lisible déjà bornée.
#[derive(Debug)]
pub struct Outcome {
    pub success: bool,
    pub output: String,
}

/// Lance un script déclaré, avec des paramètres déjà vérifiés par l'appelant.
pub async fn run_script(script: &Script, params: &BTreeMap<String, String>) -> Result<Outcome> {
    script.declared().check_params(params).map_err(anyhow::Error::msg)?;
    let program = &script.manifest.command[0];
    let mut args: Vec<String> = script.manifest.command[1..].to_vec();
    args.extend(script_args(params));
    run_process(program, &args, script.timeout(), &[("DUMBMONIT_ACTION", script.name.as_str())])
        .await
}

/// Paramètres en arguments : un seul argument `--clé=valeur` chacun, par ordre
/// de clé. Collé au nom, une valeur ne peut pas devenir une option à elle seule.
pub fn script_args(params: &BTreeMap<String, String>) -> Vec<String> {
    params.iter().map(|(key, value)| format!("--{key}={value}")).collect()
}

/// Redémarre un service autorisé.
pub async fn restart_service(config: &ActionsConfig, name: &str) -> Result<Outcome> {
    rules::validate_service_name(name).map_err(anyhow::Error::msg)?;
    if !rules::service_allowed(&config.services, name) {
        bail!("service '{name}' is not listed in action_services on this machine");
    }
    restart_service_os(name).await
}

#[cfg(target_os = "linux")]
async fn restart_service_os(name: &str) -> Result<Outcome> {
    let systemctl = ["/usr/bin/systemctl", "/bin/systemctl"]
        .into_iter()
        .find(|path| Path::new(path).exists())
        .context("systemctl not found: only systemd is supported")?;
    // `--` : tout ce qui suit est un nom d'unité, jamais une option.
    let args = ["restart".to_string(), "--".to_string(), name.to_string()];
    let mut outcome = run_process(systemctl, &args, SERVICE_TIMEOUT, &[]).await?;
    if outcome.success {
        let check = ["is-active".to_string(), "--".to_string(), name.to_string()];
        let state = run_process(systemctl, &check, Duration::from_secs(15), &[]).await?;
        outcome.success = state.success;
        outcome.output =
            format!("{}\nstate: {}", outcome.output, state.output.trim()).trim().to_string();
    }
    Ok(outcome)
}

#[cfg(windows)]
async fn restart_service_os(name: &str) -> Result<Outcome> {
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
    let net = format!(r"{root}\System32\net.exe");
    // `net stop` échoue sur un service déjà arrêté : ce n'est pas une raison de
    // ne pas le démarrer.
    let stop =
        run_process(&net, &["stop".to_string(), name.to_string()], SERVICE_TIMEOUT, &[]).await?;
    let start =
        run_process(&net, &["start".to_string(), name.to_string()], SERVICE_TIMEOUT, &[]).await?;
    Ok(Outcome {
        success: start.success,
        output: format!("{}\n{}", stop.output, start.output).trim().to_string(),
    })
}

#[cfg(not(any(target_os = "linux", windows)))]
async fn restart_service_os(_name: &str) -> Result<Outcome> {
    bail!("restarting a service is only supported on Linux (systemd) and Windows")
}

/// Lance un programme sans shell, environnement vidé, entrée fermée, durée
/// bornée ; tué s'il dépasse. La sortie est bornée à la lecture.
async fn run_process(
    program: &str,
    args: &[String],
    timeout: Duration,
    env: &[(&str, &str)],
) -> Result<Outcome> {
    let mut command = tokio::process::Command::new(program);
    command
        .args(args)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    command.env("PATH", "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin");
    #[cfg(windows)]
    for key in ["SystemRoot", "SystemDrive", "windir", "PATH"] {
        if let Ok(value) = std::env::var(key) {
            command.env(key, value);
        }
    }
    command.env("LANG", "C");
    for (key, value) in env {
        command.env(key, value);
    }
    let mut child = command.spawn().with_context(|| format!("starting {program}"))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let run = async {
        let (out, err, status) = tokio::join!(drain(stdout), drain(stderr), child.wait());
        (out, err, status)
    };
    match tokio::time::timeout(timeout, run).await {
        Ok((out, err, status)) => {
            let status = status.context("waiting for the process")?;
            let mut output = String::from_utf8_lossy(&out).into_owned();
            let err = String::from_utf8_lossy(&err);
            if !err.trim().is_empty() {
                if !output.is_empty() && !output.ends_with('\n') {
                    output.push('\n');
                }
                output.push_str(&err);
            }
            let mut output = output.trim_end().to_string();
            if !status.success() {
                output.push_str(&format!("\nexit status: {status}"));
            }
            Ok(Outcome { success: status.success(), output: output.trim().to_string() })
        }
        Err(_) => {
            let _ = child.kill().await;
            Ok(Outcome {
                success: false,
                output: format!("killed: still running after {} s", timeout.as_secs()),
            })
        }
    }
}

/// Lit un flux jusqu'au bout, en ne gardant que ses [`CAPTURE_MAX`] premiers
/// octets.
async fn drain<R: AsyncRead + Unpin>(reader: Option<R>) -> Vec<u8> {
    let mut kept = Vec::new();
    let Some(mut reader) = reader else { return kept };
    let mut chunk = [0u8; 4096];
    loop {
        match reader.read(&mut chunk).await {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                let room = CAPTURE_MAX.saturating_sub(kept.len());
                kept.extend_from_slice(&chunk[..n.min(room)]);
            }
        }
    }
    kept
}

/// Paramètres d'une commande `script.run`, validés.
pub fn command_params(command: &AgentCommand) -> Result<BTreeMap<String, String>> {
    let mut params = BTreeMap::new();
    match command.args.get("params") {
        None | Some(serde_json::Value::Null) => {}
        Some(serde_json::Value::Object(map)) => {
            for (key, value) in map {
                let value = value.as_str().context("parameter values must be strings")?;
                params.insert(key.clone(), value.to_string());
            }
        }
        Some(_) => bail!("'params' must be an object"),
    }
    rules::validate_params(&params).map_err(anyhow::Error::msg)?;
    Ok(params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn command(kind: &str, args: serde_json::Value) -> AgentCommand {
        AgentCommand { id: 1, kind: kind.into(), args, created_at_ms: 0 }
    }

    const NONCE: &str = "0123456789abcdef0123456789abcdef";

    #[test]
    fn a_replayed_request_is_refused() {
        let mut guard = ReplayGuard::default();
        let now = 1_000_000;
        let cmd = command(
            "script.run",
            json!({ "name": "x", ARG_NONCE: NONCE, ARG_EXPIRES_AT_MS: now + 60_000 }),
        );
        guard.admit(&cmd, now, true).expect("première fois");
        let error = guard.admit(&cmd, now + 1, true).unwrap_err().to_string();
        assert!(error.contains("replay"), "{error}");
    }

    #[test]
    fn an_expired_or_unbounded_request_is_refused() {
        let mut guard = ReplayGuard::default();
        let now = 1_000_000;
        let expired = command("x", json!({ ARG_NONCE: NONCE, ARG_EXPIRES_AT_MS: now - 1 }));
        assert!(guard.admit(&expired, now, true).is_err());
        let far = command("x", json!({ ARG_NONCE: NONCE, ARG_EXPIRES_AT_MS: now + 86_400_000 }));
        assert!(guard.admit(&far, now, true).is_err());
        let bare = command("x", json!({ "name": "web" }));
        assert!(guard.admit(&bare, now, true).is_err(), "nonce required");
        assert!(guard.admit(&bare, now, false).is_ok(), "legacy container command");
        let bad = command("x", json!({ ARG_NONCE: "nope", ARG_EXPIRES_AT_MS: now + 1000 }));
        assert!(guard.admit(&bad, now, true).is_err());
    }

    #[test]
    fn the_replay_memory_is_bounded() {
        let mut guard = ReplayGuard::default();
        let now = 1_000_000;
        for i in 0..(REPLAY_MEMORY + 10) {
            let nonce = format!("{i:032x}");
            let cmd = command("x", json!({ ARG_NONCE: nonce, ARG_EXPIRES_AT_MS: now + 60_000 }));
            guard.admit(&cmd, now, true).unwrap();
        }
        assert_eq!(guard.seen.len(), REPLAY_MEMORY);
        assert_eq!(guard.index.len(), REPLAY_MEMORY);
    }

    #[test]
    fn parameters_become_single_arguments() {
        let params = BTreeMap::from([
            ("level".to_string(), "high".to_string()),
            ("db".to_string(), "main".to_string()),
        ]);
        assert_eq!(script_args(&params), vec!["--db=main", "--level=high"]);
        let injected = command("script.run", json!({ "params": { "level": "high; rm -rf /" } }));
        assert!(command_params(&injected).is_err());
        let option = command("script.run", json!({ "params": { "level": "--force" } }));
        assert!(command_params(&option).is_err());
        let not_text = command("script.run", json!({ "params": { "level": 3 } }));
        assert!(command_params(&not_text).is_err());
    }

    #[test]
    fn a_name_outside_the_directory_is_never_loaded() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["../etc/passwd", "/etc/passwd", "a/../../b", "..", "x.yaml"] {
            assert!(load_script(dir.path(), name).is_err(), "{name}");
        }
        assert!(load_script(dir.path(), "absent").is_err());
    }

    #[test]
    fn services_outside_the_allow_list_are_refused() {
        let config = ActionsConfig {
            enabled: true,
            dir: PathBuf::from("/nonexistent"),
            services: vec!["nginx".into()],
        };
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let error = rt.block_on(restart_service(&config, "sshd")).unwrap_err().to_string();
        assert!(error.contains("not listed"), "{error}");
        let error = rt.block_on(restart_service(&config, "--all")).unwrap_err().to_string();
        assert!(error.contains("invalid service name"), "{error}");
    }

    #[test]
    fn nothing_is_declared_while_actions_are_off() {
        let info = describe(&ActionsConfig::default());
        assert!(!info.enabled);
        assert!(info.scripts.is_empty());
    }

    #[cfg(unix)]
    fn write_manifest(dir: &Path, name: &str, body: &str, mode: u32) {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join(format!("{name}.yaml"));
        std::fs::write(&path, body).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
    }

    #[cfg(unix)]
    fn private_dir() -> tempfile::TempDir {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        dir
    }

    #[cfg(unix)]
    #[test]
    fn a_script_runs_without_a_shell_and_is_bounded() {
        let dir = private_dir();
        let echo = ["/bin/echo", "/usr/bin/echo"].into_iter().find(|p| Path::new(p).exists());
        let Some(echo) = echo else { return };
        write_manifest(
            dir.path(),
            "say",
            &format!("command: [\"{echo}\", \"fixed\"]\nparams:\n  word: [hello, world]\n"),
            0o600,
        );
        let scripts = list_scripts(dir.path());
        assert_eq!(scripts.len(), 1);
        let script = load_script(dir.path(), "say").unwrap();
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let params = BTreeMap::from([("word".to_string(), "hello".to_string())]);
        let outcome = rt.block_on(run_script(&script, &params)).unwrap();
        assert!(outcome.success);
        assert_eq!(outcome.output, "fixed --word=hello");
        let wrong = BTreeMap::from([("word".to_string(), "other".to_string())]);
        assert!(rt.block_on(run_script(&script, &wrong)).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn a_script_that_runs_too_long_is_killed() {
        let dir = private_dir();
        let sleep = ["/bin/sleep", "/usr/bin/sleep"].into_iter().find(|p| Path::new(p).exists());
        let Some(sleep) = sleep else { return };
        write_manifest(
            dir.path(),
            "nap",
            &format!("command: [\"{sleep}\", \"30\"]\ntimeout_secs: 1\n"),
            0o600,
        );
        let script = load_script(dir.path(), "nap").unwrap();
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let outcome = rt.block_on(run_script(&script, &BTreeMap::new())).unwrap();
        assert!(!outcome.success);
        assert!(outcome.output.contains("killed"), "{}", outcome.output);
    }

    #[cfg(unix)]
    #[test]
    fn a_manifest_writable_by_others_or_with_a_relative_program_is_refused() {
        let dir = private_dir();
        write_manifest(dir.path(), "open", "command: [\"/bin/true\"]\n", 0o666);
        assert!(load_script(dir.path(), "open").is_err());
        write_manifest(dir.path(), "relative", "command: [\"true\"]\n", 0o600);
        assert!(load_script(dir.path(), "relative").is_err());
        write_manifest(dir.path(), "shell", "command: \"rm -rf /\"\n", 0o600);
        assert!(load_script(dir.path(), "shell").is_err(), "a string is not an argv");
        write_manifest(dir.path(), "extra", "command: [\"/bin/true\"]\nshell: true\n", 0o600);
        assert!(load_script(dir.path(), "extra").is_err(), "unknown keys are refused");
    }
}
