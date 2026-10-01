//! Dépôts restic et Borg : le dépôt répond-il, et quel âge a son dernier
//! instantané.
//!
//! Contrairement à Plakar, rien n'est découvert : un dépôt restic ou Borg n'a
//! pas d'emplacement conventionnel, et il faut son mot de passe pour le lire.
//! Chaque dépôt est donc **déclaré** dans `agent.yaml`, avec la façon d'obtenir
//! ce mot de passe (fichier, commande, ou variables d'environnement). Le mot de
//! passe reste sur la machine : il est passé à `restic`/`borg` par
//! l'environnement du processus fils, jamais journalisé, jamais envoyé au
//! serveur — les séries ne portent que le nom du dépôt.
//!
//! Deux commandes, en lecture seule stricte :
//!
//! - `restic snapshots --json --latest 1 --no-lock --no-cache` : sans verrou,
//!   la lecture n'écrit rien dans le dépôt (un dépôt en lecture seule ou en
//!   ajout seul se lit donc), ne bute pas sur le verrou exclusif d'un `prune`,
//!   et n'a pas besoin d'un cache inscriptible — le service livré tourne avec
//!   `ProtectHome=yes`.
//! - `borg --bypass-lock list --json --last 1` : sans `--bypass-lock`, la
//!   lecture échouerait pendant toute la durée d'un `borg create`, qui tient le
//!   verrou exclusif — l'alerte « dépôt injoignable » sonnerait chaque nuit.
//!
//! L'emplacement du dépôt passe par `RESTIC_REPOSITORY`/`BORG_REPO` et non par
//! la ligne de commande : une URL `rest:https://user:secret@…` n'a rien à faire
//! dans `ps`.
//!
//! Ouvrir un dépôt distant prend du temps : la lecture tourne dans une tâche de
//! fond, au plus une fois par `interval`, et son dernier résultat est réémis à
//! chaque cycle — le même schéma que Plakar.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, FixedOffset, NaiveDateTime, Utc};
use dumbmonit_proto::{MetricKind, Sample};
use serde::Deserialize;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

/// Période par défaut entre deux lectures. Dix minutes : une sauvegarde
/// nocturne ne se surveille pas à la seconde.
pub const DEFAULT_INTERVAL_SECS: u64 = 600;

/// En dessous, l'agent passerait son temps à ouvrir des dépôts.
pub const MIN_INTERVAL_SECS: u64 = 60;

/// Délai de chaque commande. Un dépôt S3 ou SFTP lent mérite de la patience,
/// pas indéfiniment.
const COMMAND_TIMEOUT: Duration = Duration::from_secs(120);

/// Attente au tout premier cycle, pour qu'un `--once` montre quelque chose.
const FIRST_CYCLE_WAIT: Duration = Duration::from_secs(5);

/// Plafond de dépôts : chacun coûte un processus toutes les dix minutes.
pub const MAX_REPOS: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Restic,
    Borg,
}

impl Tool {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Restic => "restic",
            Self::Borg => "borg",
        }
    }
}

/// Un dépôt déclaré dans `agent.yaml`.
#[derive(Clone, PartialEq, Eq)]
pub struct RepoConfig {
    pub tool: Tool,
    /// Étiquette `repo` des séries. Par défaut, l'emplacement du dépôt.
    pub name: String,
    /// `RESTIC_REPOSITORY` ou `BORG_REPO`.
    pub repository: String,
    /// Fichier contenant le mot de passe (`RESTIC_PASSWORD_FILE` ; pour Borg,
    /// l'agent le lit et le passe en `BORG_PASSPHRASE`).
    pub password_file: Option<PathBuf>,
    /// Commande qui imprime le mot de passe (`RESTIC_PASSWORD_COMMAND`,
    /// `BORG_PASSCOMMAND`).
    pub password_command: Option<String>,
    /// Variables d'environnement ajoutées pour ce dépôt (clés S3, `BORG_RSH`…).
    pub env: BTreeMap<String, String>,
    /// Arguments ajoutés avant ceux de l'agent (`-o sftp.command=…`).
    pub args: Vec<String>,
    /// restic : `--host`, pour un dépôt partagé entre plusieurs machines.
    pub host: Option<String>,
    /// Borg : `--glob-archives`, pour un dépôt partagé entre plusieurs machines.
    pub glob_archives: Option<String>,
}

/// Les valeurs de `env` et la commande de mot de passe peuvent porter des
/// secrets : elles ne s'affichent jamais, pas même en `debug`.
impl fmt::Debug for RepoConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RepoConfig")
            .field("tool", &self.tool)
            .field("name", &self.name)
            .field("password_file", &self.password_file)
            .field("password_command", &self.password_command.as_ref().map(|_| "<redacted>"))
            .field("env", &self.env.keys().collect::<Vec<_>>())
            .field("host", &self.host)
            .field("glob_archives", &self.glob_archives)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupReposConfig {
    pub restic_bin: String,
    pub borg_bin: String,
    pub repos: Vec<RepoConfig>,
    pub interval: Duration,
}

impl Default for BackupReposConfig {
    fn default() -> Self {
        Self {
            restic_bin: "restic".to_string(),
            borg_bin: "borg".to_string(),
            repos: Vec::new(),
            interval: Duration::from_secs(DEFAULT_INTERVAL_SECS),
        }
    }
}

/// Le dernier instantané d'un dépôt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastSnapshot {
    pub time: DateTime<Utc>,
    /// Octets parcourus par la sauvegarde (restic 0.17 et suivants).
    pub bytes: Option<u64>,
}

/// Photographie d'un dépôt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoStat {
    pub tool: Tool,
    pub name: String,
    /// Faux : le dépôt n'a pas pu être lu (injoignable, mot de passe refusé,
    /// binaire absent…). La raison est dans le journal de l'agent.
    pub reachable: bool,
    /// `None` : dépôt vide, ou illisible.
    pub last: Option<LastSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BackupReposReport {
    pub repos: Vec<RepoStat>,
}

/// Traduit le rapport en échantillons.
pub fn samples(report: &BackupReposReport, now_ms: i64) -> Vec<Sample> {
    let mut samples = Vec::with_capacity(report.repos.len() * 3);
    for repo in report.repos.iter().take(MAX_REPOS) {
        let gauge = |metric: &str, value: f64| {
            Sample::new(metric, value, MetricKind::Gauge, now_ms)
                .with_label("tool", repo.tool.as_str())
                .with_label("repo", &repo.name)
        };
        samples.push(gauge("agent_backup_repo_reachable", f64::from(u8::from(repo.reachable))));
        if let Some(last) = &repo.last {
            let age = (now_ms - last.time.timestamp_millis()).max(0) as f64 / 1000.0;
            samples.push(gauge("agent_backup_repo_last_snapshot_age_seconds", age.round()));
            if let Some(bytes) = last.bytes {
                samples.push(gauge("agent_backup_repo_last_snapshot_bytes", bytes as f64));
            }
        }
    }
    samples
}

// ------------------------------------------------------------------ analyse

#[derive(Deserialize)]
struct ResticSnapshot {
    time: String,
    #[serde(default)]
    summary: Option<ResticSummary>,
}

#[derive(Deserialize)]
struct ResticSummary {
    #[serde(default)]
    total_bytes_processed: Option<u64>,
}

/// Lit `restic snapshots --json --latest 1` : un instantané par couple
/// (machine, chemins). Le plus récent de tous est retenu. `Err` : ce n'est pas
/// la sortie attendue.
pub fn parse_restic(text: &str) -> Result<Option<LastSnapshot>, String> {
    let snapshots: Vec<ResticSnapshot> =
        serde_json::from_str(text).map_err(|e| format!("unexpected restic output: {e}"))?;
    Ok(snapshots
        .into_iter()
        .filter_map(|s| {
            let time = DateTime::parse_from_rfc3339(&s.time).ok()?.with_timezone(&Utc);
            Some(LastSnapshot { time, bytes: s.summary.and_then(|m| m.total_bytes_processed) })
        })
        .max_by_key(|s| s.time))
}

#[derive(Deserialize)]
struct BorgList {
    archives: Vec<BorgArchive>,
}

#[derive(Deserialize)]
struct BorgArchive {
    #[serde(default)]
    time: Option<String>,
    #[serde(default)]
    start: Option<String>,
}

/// Lit `borg list --json --last 1`. Borg 1.x écrit l'heure **locale** de la
/// machine qui lance la commande, sans fuseau : `offset` est donc celui de
/// l'agent. Une heure avec fuseau (Borg 2) est prise telle quelle.
pub fn parse_borg(text: &str, offset: FixedOffset) -> Result<Option<LastSnapshot>, String> {
    let list: BorgList =
        serde_json::from_str(text).map_err(|e| format!("unexpected borg output: {e}"))?;
    Ok(list
        .archives
        .into_iter()
        .filter_map(|a| {
            let raw = a.time.or(a.start)?;
            let time = parse_borg_time(&raw, offset)?;
            Some(LastSnapshot { time, bytes: None })
        })
        .max_by_key(|s| s.time))
}

fn parse_borg_time(raw: &str, offset: FixedOffset) -> Option<DateTime<Utc>> {
    if let Ok(with_zone) = DateTime::parse_from_rfc3339(raw) {
        return Some(with_zone.with_timezone(&Utc));
    }
    let naive = NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S%.f").ok()?;
    Some(naive.and_local_timezone(offset).single()?.with_timezone(&Utc))
}

/// La raison d'un échec, en une ligne, tirée de la sortie d'erreur. restic 0.17+
/// écrit un objet `exit_error` en JSON ; Borg écrit du texte, précédé parfois
/// d'avertissements Python sans intérêt.
pub fn failure_reason(stderr: &str) -> String {
    #[derive(Deserialize)]
    struct ExitError {
        message_type: String,
        message: String,
    }
    for line in stderr.lines() {
        if let Ok(error) = serde_json::from_str::<ExitError>(line)
            && error.message_type == "exit_error"
        {
            return error.message.lines().next().unwrap_or("").trim().to_string();
        }
    }
    stderr
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.contains("Warning"))
        .unwrap_or("")
        .to_string()
}

// ------------------------------------------------------------------- lecture

/// Ce que rend une lecture : chaque dépôt, et la raison de son échec éventuel.
type Readings = Vec<(RepoStat, Option<String>)>;

/// Ce qui part vers le processus fils : binaire, arguments, environnement ajouté.
type Invocation = (String, Vec<String>, BTreeMap<String, String>);

/// Lecteur des dépôts, conservé d'un cycle à l'autre pour porter le dernier
/// rapport et la tâche de fond.
pub struct BackupReposProbe {
    config: BackupReposConfig,
    last: Option<BackupReposReport>,
    refreshed_at: Option<tokio::time::Instant>,
    task: Option<JoinHandle<Readings>>,
    /// Dernière raison d'échec dite, par dépôt : on ne la répète que si elle change.
    failures: BTreeMap<String, String>,
}

impl BackupReposProbe {
    pub fn new(config: &BackupReposConfig) -> Self {
        if !config.repos.is_empty() {
            let names: Vec<String> =
                config.repos.iter().map(|r| format!("{}:{}", r.tool.as_str(), r.name)).collect();
            info!(repos = ?names, "restic/Borg repositories to watch");
        }
        Self {
            config: config.clone(),
            last: None,
            refreshed_at: None,
            task: None,
            failures: BTreeMap::new(),
        }
    }

    /// Un cycle. `None` : aucun dépôt déclaré, ou première lecture pas encore
    /// aboutie.
    pub async fn read(&mut self) -> Option<BackupReposReport> {
        if self.config.repos.is_empty() {
            return None;
        }
        if self.task.is_none()
            && self.refreshed_at.is_none_or(|at| at.elapsed() >= self.config.interval)
        {
            self.task = Some(tokio::spawn(read_all(self.config.clone())));
        }
        if let Some(task) = self.task.as_mut() {
            let first_cycle = self.refreshed_at.is_none();
            if (task.is_finished() || first_cycle)
                && let Ok(outcome) = tokio::time::timeout(FIRST_CYCLE_WAIT, task).await
            {
                self.task = None;
                self.refreshed_at = Some(tokio::time::Instant::now());
                match outcome {
                    Ok(results) => self.last = Some(self.announce(results)),
                    Err(error) => debug!(%error, "restic/Borg reading aborted"),
                }
            }
        }
        self.last.clone()
    }

    fn announce(&mut self, results: Readings) -> BackupReposReport {
        let mut repos = Vec::with_capacity(results.len());
        for (stat, why) in results {
            let key = format!("{}:{}", stat.tool.as_str(), stat.name);
            match why {
                Some(why) => {
                    if self.failures.get(&key) != Some(&why) {
                        warn!(repo = key, %why, "cannot read the backup repository");
                        self.failures.insert(key, why);
                    }
                }
                None => {
                    if self.failures.remove(&key).is_some() {
                        info!(repo = key, "backup repository readable again");
                    }
                }
            }
            repos.push(stat);
        }
        BackupReposReport { repos }
    }
}

/// Lit tous les dépôts, l'un après l'autre : deux `restic` en parallèle
/// doubleraient la mémoire prise dans l'enveloppe du service.
async fn read_all(config: BackupReposConfig) -> Readings {
    let offset = *chrono::Local::now().offset();
    let mut results = Vec::with_capacity(config.repos.len());
    for repo in config.repos.iter().take(MAX_REPOS) {
        let outcome = read_repo(&config, repo, offset).await;
        let (reachable, last, why) = match outcome {
            Ok(last) => (true, last, None),
            Err(why) => (false, None, Some(why)),
        };
        results.push((RepoStat { tool: repo.tool, name: repo.name.clone(), reachable, last }, why));
    }
    results
}

async fn read_repo(
    config: &BackupReposConfig,
    repo: &RepoConfig,
    offset: FixedOffset,
) -> Result<Option<LastSnapshot>, String> {
    let (bin, args, env) = command_for(config, repo)?;
    let stdout = run(&bin, &args, &env).await?;
    match repo.tool {
        Tool::Restic => parse_restic(&stdout),
        Tool::Borg => parse_borg(&stdout, offset),
    }
}

/// Construit la commande d'un dépôt : binaire, arguments, environnement ajouté.
/// Séparé de l'exécution pour que ce qui part vers le processus fils se vérifie
/// sans lancer quoi que ce soit.
fn command_for(config: &BackupReposConfig, repo: &RepoConfig) -> Result<Invocation, String> {
    let mut env = repo.env.clone();
    let mut args = repo.args.clone();
    match repo.tool {
        Tool::Restic => {
            env.insert("RESTIC_REPOSITORY".into(), repo.repository.clone());
            if let Some(file) = &repo.password_file {
                env.insert("RESTIC_PASSWORD_FILE".into(), file.to_string_lossy().into_owned());
            }
            if let Some(command) = &repo.password_command {
                env.insert("RESTIC_PASSWORD_COMMAND".into(), command.clone());
            }
            args.extend(
                ["snapshots", "--json", "--latest", "1", "--no-lock", "--no-cache"]
                    .map(String::from),
            );
            if let Some(host) = &repo.host {
                args.extend(["--host".to_string(), host.clone()]);
            }
            Ok((config.restic_bin.clone(), args, env))
        }
        Tool::Borg => {
            env.insert("BORG_REPO".into(), repo.repository.clone());
            if let Some(file) = &repo.password_file {
                let passphrase = std::fs::read_to_string(file)
                    .map_err(|e| format!("cannot read {}: {e}", file.display()))?;
                env.insert(
                    "BORG_PASSPHRASE".into(),
                    passphrase.trim_end_matches(['\r', '\n']).into(),
                );
            }
            if let Some(command) = &repo.password_command {
                env.insert("BORG_PASSCOMMAND".into(), command.clone());
            }
            if let Some(dir) = borg_base_dir_fallback(&env) {
                env.insert("BORG_BASE_DIR".into(), dir.to_string_lossy().into_owned());
            }
            let mut full = vec!["--bypass-lock".to_string()];
            full.append(&mut args);
            full.extend(["list", "--json", "--last", "1"].map(String::from));
            if let Some(glob) = &repo.glob_archives {
                full.extend(["--glob-archives".to_string(), glob.clone()]);
            }
            Ok((config.borg_bin.clone(), full, env))
        }
    }
}

/// Borg écrit toujours sous `~/.config/borg` (registre de sécurité des dépôts
/// vus). Le service livré tourne avec `ProtectHome=yes` : `/root` y est
/// inaccessible et Borg refuserait de lister quoi que ce soit. Dans ce cas, et
/// seulement s'il n'a rien été choisi, la base passe dans le `/tmp` privé du
/// service.
fn borg_base_dir_fallback(env: &BTreeMap<String, String>) -> Option<PathBuf> {
    if env.contains_key("BORG_BASE_DIR") || std::env::var_os("BORG_BASE_DIR").is_some() {
        return None;
    }
    let home =
        env.get("HOME").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(PathBuf::from));
    if let Some(home) = home
        && writable_config_dir(&home)
    {
        return None;
    }
    Some(std::env::temp_dir().join("dumbmonit-borg"))
}

fn writable_config_dir(home: &Path) -> bool {
    std::fs::create_dir_all(home.join(".config").join("borg")).is_ok()
}

async fn run(bin: &str, args: &[String], env: &BTreeMap<String, String>) -> Result<String, String> {
    let mut command = tokio::process::Command::new(bin);
    command
        .args(args)
        .envs(env)
        .env("LC_ALL", "C")
        // Sans terminal, jamais de question : un mot de passe manquant échoue
        // proprement au lieu d'attendre une saisie.
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true);
    match tokio::time::timeout(COMMAND_TIMEOUT, command.output()).await {
        Ok(Ok(output)) if output.status.success() => {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        }
        Ok(Ok(output)) => {
            let code = output.status.code().unwrap_or(-1);
            let why = failure_reason(&String::from_utf8_lossy(&output.stderr));
            Err(format!("{bin} exited with {code}: {why}"))
        }
        Ok(Err(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            Err(format!("{bin} not found: install it or set its path in agent.yaml"))
        }
        Ok(Err(error)) => Err(format!("cannot start {bin}: {error}")),
        Err(_) => Err(format!("{bin} timed out after {}s", COMMAND_TIMEOUT.as_secs())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sorties réelles (restic 0.18.0, borg 1.4.0) : un dépôt restic partagé par
    /// deux machines, un dépôt Borg à deux archives, et les deux vides.
    const RESTIC: &str = include_str!("testdata/restic_snapshots_latest1.json");
    const RESTIC_EMPTY: &str = include_str!("testdata/restic_snapshots_empty.json");
    const BORG: &str = include_str!("testdata/borg_list_last1.json");
    const BORG_EMPTY: &str = include_str!("testdata/borg_list_empty.json");

    fn utc() -> FixedOffset {
        FixedOffset::east_opt(0).expect("offset")
    }

    fn repo(tool: Tool) -> RepoConfig {
        RepoConfig {
            tool,
            name: "nas".into(),
            repository: "sftp:backup@192.0.2.10:/srv/repo".into(),
            password_file: None,
            password_command: None,
            env: BTreeMap::new(),
            args: Vec::new(),
            host: None,
            glob_archives: None,
        }
    }

    #[test]
    fn restic_keeps_the_newest_snapshot_across_hosts() {
        let last = parse_restic(RESTIC).expect("json").expect("a snapshot");
        assert_eq!(last.time.to_rfc3339(), "2026-09-30T22:45:10.714280948+00:00");
        assert_eq!(last.bytes, Some(17));
        assert_eq!(parse_restic(RESTIC_EMPTY), Ok(None));
        assert!(parse_restic("Fatal: something").is_err());
    }

    #[test]
    fn borg_times_are_local_to_the_agent() {
        let last = parse_borg(BORG, utc()).expect("json").expect("an archive");
        assert_eq!(last.time.to_rfc3339(), "2026-09-30T22:45:20+00:00");
        let paris = FixedOffset::east_opt(2 * 3600).expect("offset");
        let local = parse_borg(BORG, paris).expect("json").expect("an archive");
        assert_eq!(local.time.to_rfc3339(), "2026-09-30T20:45:20+00:00");
        assert_eq!(parse_borg(BORG_EMPTY, utc()), Ok(None));
        let zoned = r#"{"archives":[{"name":"a","time":"2026-09-30T22:45:20.000000+02:00"}]}"#;
        let zoned = parse_borg(zoned, utc()).expect("json").expect("archive");
        assert_eq!(zoned.time.to_rfc3339(), "2026-09-30T20:45:20+00:00");
    }

    #[test]
    fn failure_reasons_come_from_real_error_outputs() {
        // restic 0.18, mot de passe faux (code 12) et dépôt injoignable.
        let wrong = "{\"message_type\":\"exit_error\",\"code\":12,\"message\":\"Fatal: wrong password or no key found\"}\n";
        assert_eq!(failure_reason(wrong), "Fatal: wrong password or no key found");
        let unreachable = "subprocess ssh: ssh: connect to host 192.0.2.50 port 22: Connection timed out\n\
            {\"message_type\":\"exit_error\",\"code\":1,\"message\":\"Fatal: unable to open repository at sftp:nobody@192.0.2.50:/srv/restic: unable to start the sftp session\"}\n";
        assert!(failure_reason(unreachable).starts_with("Fatal: unable to open repository"));
        // borg 1.4 sans phrase de passe : les avertissements Python sont sautés.
        let borg = "/usr/lib/python3.13/getpass.py:90: GetPassWarning: Can not control echo on the terminal.\n\
            Warning: Password input may be echoed.\n\
            Enter passphrase for key /repos/borg: can not acquire a passphrase: BORG_PASSPHRASE is not set.\n";
        assert!(failure_reason(borg).contains("BORG_PASSPHRASE is not set"));
        assert_eq!(
            failure_reason("Repository /repos/missing does not exist.\n"),
            "Repository /repos/missing does not exist."
        );
    }

    #[test]
    fn samples_carry_the_repo_name_and_never_a_secret() {
        let report = BackupReposReport {
            repos: vec![
                RepoStat {
                    tool: Tool::Restic,
                    name: "nas".into(),
                    reachable: true,
                    last: parse_restic(RESTIC).expect("json"),
                },
                RepoStat { tool: Tool::Borg, name: "offsite".into(), reachable: false, last: None },
                RepoStat { tool: Tool::Borg, name: "empty".into(), reachable: true, last: None },
            ],
        };
        let now = DateTime::parse_from_rfc3339("2026-10-01T22:45:10.714Z").expect("date");
        let series = samples(&report, now.timestamp_millis());
        assert_eq!(series.len(), 3 + 1 + 1);
        let age = series
            .iter()
            .find(|s| s.metric == "agent_backup_repo_last_snapshot_age_seconds")
            .expect("age");
        assert_eq!(age.value, 86_400.0);
        assert_eq!(age.labels.get("tool").map(String::as_str), Some("restic"));
        assert_eq!(age.labels.get("repo").map(String::as_str), Some("nas"));
        let down = series
            .iter()
            .find(|s| s.labels.get("repo").map(String::as_str) == Some("offsite"))
            .expect("offsite");
        assert_eq!((down.metric.as_str(), down.value), ("agent_backup_repo_reachable", 0.0));
    }

    #[test]
    fn restic_reads_without_lock_nor_cache_and_keeps_the_url_out_of_argv() {
        let mut nas = repo(Tool::Restic);
        nas.password_file = Some("/etc/dumbmonit/restic.pass".into());
        nas.host = Some("web-01".into());
        nas.args = vec!["-o".into(), "sftp.command=ssh -i /etc/dumbmonit/id nas".into()];
        let (bin, args, env) = command_for(&BackupReposConfig::default(), &nas).expect("command");
        assert_eq!(bin, "restic");
        assert_eq!(
            args,
            [
                "-o",
                "sftp.command=ssh -i /etc/dumbmonit/id nas",
                "snapshots",
                "--json",
                "--latest",
                "1",
                "--no-lock",
                "--no-cache",
                "--host",
                "web-01"
            ]
        );
        assert_eq!(env["RESTIC_REPOSITORY"], "sftp:backup@192.0.2.10:/srv/repo");
        assert_eq!(env["RESTIC_PASSWORD_FILE"], "/etc/dumbmonit/restic.pass");
        assert!(!args.iter().any(|a| a.contains("192.0.2.10")));
    }

    #[test]
    fn borg_bypasses_the_lock_and_reads_its_passphrase_file() {
        let dir = std::env::temp_dir().join(format!("dm-borg-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let file = dir.join("pass");
        std::fs::write(&file, "correct horse\n").expect("write");
        let mut offsite = repo(Tool::Borg);
        offsite.password_file = Some(file);
        offsite.glob_archives = Some("web-01-*".into());
        offsite.env.insert("BORG_BASE_DIR".into(), dir.to_string_lossy().into_owned());
        let (bin, args, env) =
            command_for(&BackupReposConfig::default(), &offsite).expect("command");
        assert_eq!(bin, "borg");
        assert_eq!(
            args,
            ["--bypass-lock", "list", "--json", "--last", "1", "--glob-archives", "web-01-*"]
        );
        assert_eq!(env["BORG_PASSPHRASE"], "correct horse", "trailing newline dropped");
        assert_eq!(env["BORG_REPO"], "sftp:backup@192.0.2.10:/srv/repo");
        std::fs::remove_dir_all(&dir).ok();

        offsite.password_file = Some("/nonexistent/pass".into());
        assert!(command_for(&BackupReposConfig::default(), &offsite).is_err());
    }

    #[test]
    fn borg_falls_back_to_a_private_base_dir_when_home_is_not_writable() {
        let env = BTreeMap::from([("HOME".to_string(), "/proc/nonexistent".to_string())]);
        if std::env::var_os("BORG_BASE_DIR").is_none() {
            assert_eq!(
                borg_base_dir_fallback(&env),
                Some(std::env::temp_dir().join("dumbmonit-borg"))
            );
        }
        let chosen = BTreeMap::from([("BORG_BASE_DIR".to_string(), "/srv/borg".to_string())]);
        assert_eq!(borg_base_dir_fallback(&chosen), None);
    }

    #[test]
    fn the_debug_form_hides_secrets() {
        let mut nas = repo(Tool::Restic);
        nas.env.insert("AWS_SECRET_ACCESS_KEY".into(), "s3cr3t-value".into());
        nas.password_command = Some("pass show backup --token=tok3n".into());
        let shown = format!("{nas:?}");
        assert!(!shown.contains("s3cr3t-value"));
        assert!(!shown.contains("tok3n"));
        assert!(shown.contains("AWS_SECRET_ACCESS_KEY"));
    }

    #[tokio::test]
    async fn a_missing_binary_makes_the_repo_unreachable() {
        let config = BackupReposConfig {
            restic_bin: "/nonexistent/restic".into(),
            repos: vec![repo(Tool::Restic)],
            ..BackupReposConfig::default()
        };
        let mut probe = BackupReposProbe::new(&config);
        let report = probe.read().await.expect("first cycle waits for the reading");
        assert_eq!(report.repos.len(), 1);
        assert!(!report.repos[0].reachable);
        assert!(probe.failures.contains_key("restic:nas"));

        let mut none = BackupReposProbe::new(&BackupReposConfig::default());
        assert!(none.read().await.is_none());
    }
}
