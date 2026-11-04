//! Bases de données présentes sur la machine, et fraîcheur de leurs sauvegardes.
//!
//! Tout se fait **sans identifiant** et en lecture seule : l'agent ne se connecte
//! à aucune base, n'exécute aucune requête SQL et ne lit aucun secret.
//!
//! **Détection.** Les services installés disent quels moteurs sont là : unités
//! systemd sous Linux, gestionnaire de services (et clés de registre de SQL
//! Server) sous Windows. Les ports en écoute complètent : un moteur lancé dans un
//! conteneur n'a pas de service, mais son port répond. Remontés pour chaque
//! moteur : nom, instance, version quand elle se lit sans identifiant (clé de
//! registre, nom du service, dossier d'installation), service en marche ou
//! arrêté, port.
//!
//! **Sauvegardes.** Une fraîcheur de fichier, rien de plus : l'agent cherche le
//! fichier (ou dossier) le plus récent parmi des chemins avec jokers — ceux de
//! `agent.yaml` (`databases.backups`), sinon des emplacements usuels par moteur —
//! et remonte sa date, sa taille et son âge. `overdue` vaut 1 quand cet âge
//! dépasse l'âge maximal choisi (36 h par défaut) ; c'est la série que lit la
//! règle d'alerte. Sans chemin déclaré, `overdue` n'existe que si un emplacement
//! usuel contient quelque chose : une base dont on ignore où elle est sauvegardée
//! ne déclenche pas d'alerte à tort (l'interface le dit, sans alarmer).
//!
//! Plus tard, avec identifiants : `msdb.dbo.backupset` pour SQL Server,
//! `pg_stat_archiver` pour PostgreSQL. Rien de cela ici.
//!
//! Les lectures du système (services, ports, registre, disque) sont séparées de
//! l'analyse : les fonctions pures de ce fichier se testent sans machine réelle.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use dumbmonit_proto::{MetricKind, Sample};
use tokio::task::JoinHandle;
use tracing::debug;

/// Période par défaut entre deux lectures. Une base ne change pas d'état à la
/// seconde, et le parcours des dossiers de sauvegarde n'est pas gratuit.
pub const DEFAULT_INTERVAL_SECS: u64 = 300;

/// En dessous, l'agent passerait son temps à parcourir des dossiers.
pub const MIN_INTERVAL_SECS: u64 = 60;

/// Âge maximal par défaut d'une sauvegarde : une sauvegarde quotidienne, plus la
/// marge d'une nuit lente.
pub const DEFAULT_MAX_AGE_HOURS: u64 = 36;

/// Plafond de bases remontées.
pub const MAX_DATABASES: usize = 32;

/// Plafond de chemins déclarés par moteur.
pub const MAX_PATHS_PER_ENGINE: usize = 32;

/// Entrées examinées au plus par motif : un dossier de dix mille dumps ne doit
/// pas figer la lecture.
const MAX_SCANNED_ENTRIES: usize = 20_000;

/// Attente au premier cycle, pour qu'un `--once` montre quelque chose.
const FIRST_CYCLE_WAIT: Duration = Duration::from_secs(8);

/// Délai des commandes lancées (systemctl, PowerShell).
const COMMAND_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Engine {
    Postgresql,
    /// MySQL et MariaDB : mêmes services, mêmes sauvegardes.
    Mysql,
    Sqlserver,
    Hfsql,
    Mongodb,
}

impl Engine {
    pub const ALL: [Engine; 5] =
        [Self::Postgresql, Self::Mysql, Self::Sqlserver, Self::Hfsql, Self::Mongodb];

    /// Valeur de l'étiquette `engine`, et clé dans `databases.backups`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Postgresql => "postgresql",
            Self::Mysql => "mysql",
            Self::Sqlserver => "sqlserver",
            Self::Hfsql => "hfsql",
            Self::Mongodb => "mongodb",
        }
    }

    /// Lit une clé de `databases.backups` ; plusieurs noms courants sont admis.
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "postgresql" | "postgres" | "pg" => Some(Self::Postgresql),
            "mysql" | "mariadb" => Some(Self::Mysql),
            "sqlserver" | "mssql" | "sql_server" => Some(Self::Sqlserver),
            "hfsql" => Some(Self::Hfsql),
            "mongodb" | "mongo" => Some(Self::Mongodb),
            _ => None,
        }
    }

    pub fn default_port(self) -> u16 {
        match self {
            Self::Postgresql => 5432,
            Self::Mysql => 3306,
            Self::Sqlserver => 1433,
            Self::Hfsql => 4900,
            Self::Mongodb => 27017,
        }
    }
}

/// Sauvegardes d'un moteur déclarées dans `agent.yaml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupRule {
    pub engine: Engine,
    /// Chemins, jokers `*` et `?` admis dans chaque segment.
    pub paths: Vec<String>,
    pub max_age_hours: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabasesConfig {
    /// Faux : ni détection ni série.
    pub enabled: bool,
    pub interval: Duration,
    pub backups: Vec<BackupRule>,
}

impl Default for DatabasesConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval: Duration::from_secs(DEFAULT_INTERVAL_SECS),
            backups: Vec::new(),
        }
    }
}

// ----------------------------------------------------------------- faits bruts

/// Un service du système, tel que le gestionnaire le décrit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceFact {
    pub name: String,
    /// Nom d'affichage (Windows) ; vide sous Linux.
    pub display: String,
    pub running: bool,
    /// Chemin de l'exécutable (Windows : `PathName`).
    pub path: Option<String>,
}

/// Une instance SQL Server lue dans le registre.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SqlInstanceFact {
    /// `MSSQLSERVER` pour l'instance par défaut.
    pub name: String,
    pub version: Option<String>,
    pub edition: Option<String>,
    /// Dossier de sauvegarde par défaut de l'instance.
    pub backup_dir: Option<String>,
    pub port: Option<u16>,
}

/// Tout ce que la machine a dit, avant interprétation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Facts {
    pub services: Vec<ServiceFact>,
    pub ports: BTreeSet<u16>,
    pub sql_instances: Vec<SqlInstanceFact>,
    /// Versions de PostgreSQL installées (noms de dossiers, Linux).
    pub pg_versions: Vec<String>,
}

/// Une base détectée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detected {
    pub engine: Engine,
    pub instance: String,
    pub version: Option<String>,
    pub running: bool,
    pub port: Option<u16>,
    /// Dossier de sauvegarde par défaut connu du moteur (SQL Server).
    pub backup_dir: Option<String>,
    /// Dossier de l'exécutable du service (HFSQL).
    pub exe_dir: Option<String>,
}

// -------------------------------------------------------------------- analyse

/// Mots qui désignent un service annexe d'un moteur, pas le moteur : agent,
/// routeur, exporteur de métriques, lanceur…
const SIDE_SERVICE_WORDS: &[&str] = &[
    "exporter",
    "router",
    "setup",
    "agent",
    "launcher",
    "browser",
    "writer",
    "olap",
    "integration",
    "reporting",
    "analysis",
    "fdhost",
    "telemetry",
    "vss",
    "pgbouncer",
    "pgadmin",
];

/// Ce qu'un nom de service dit du moteur : `(moteur, instance, version)`.
///
/// Reconnaît les noms Windows (`MSSQLSERVER`, `MSSQL$SQLEXPRESS`,
/// `postgresql-x64-16`, `MySQL80`, `MariaDB`, `MongoDB`, tout service HFSQL) et
/// les unités systemd (`postgresql@16-main.service`, `mariadb.service`,
/// `mssql-server.service`, `mongod.service`).
pub fn classify_service(name: &str, display: &str) -> Option<(Engine, String, Option<String>)> {
    let raw = name.trim();
    let base = raw.strip_suffix(".service").unwrap_or(raw);
    if base.is_empty() {
        return None;
    }
    let lower = base.to_ascii_lowercase();
    let lower_display = display.to_ascii_lowercase();
    // Seul le type du service est jugé (« SQLAgent » de `SQLAgent$PROD`), pas le
    // nom de l'instance : une instance « AGENTDB » reste une base.
    let head = lower.split(['$', '@']).next().unwrap_or("");
    if SIDE_SERVICE_WORDS.iter().any(|w| head.contains(w)) {
        return None;
    }

    // HFSQL d'abord : le nom du service de PC SOFT varie selon la version, son
    // nom d'affichage mentionne toujours HFSQL.
    if lower.contains("hfsql") || lower_display.contains("hfsql") {
        return Some((Engine::Hfsql, base.to_string(), None));
    }

    // SQL Server : instance par défaut, instance nommée, Linux, et l'ancien
    // « SQLEXPRESS » nu.
    if lower == "mssqlserver" || lower == "mssql-server" {
        return Some((Engine::Sqlserver, "MSSQLSERVER".to_string(), None));
    }
    if let Some(instance) = lower.strip_prefix("mssql$") {
        if instance.is_empty() {
            return None;
        }
        return Some((Engine::Sqlserver, base[6..].to_string(), None));
    }
    if lower == "sqlexpress" {
        return Some((Engine::Sqlserver, base.to_string(), None));
    }

    if let Some(rest) = lower.strip_prefix("postgresql") {
        return classify_postgresql(base, &base[10..], rest);
    }

    for prefix in ["mysql", "mariadb"] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            // `mysql`, `mysqld`, `mysql80`, `mariadb`, `mysql@dev`…, mais pas
            // `mysqlrouter` (déjà écarté) ni un nom qui n'a qu'un préfixe commun.
            let first = rest.chars().next();
            let ok = first.is_none_or(|c| c == 'd' || c == '@' || c == '-' || c.is_ascii_digit());
            if !ok {
                return None;
            }
            if let Some(at) = rest.strip_prefix('@').or_else(|| rest.strip_prefix("d@")) {
                if at.is_empty() {
                    return None;
                }
                return Some((Engine::Mysql, at.to_string(), None));
            }
            let digits: String = rest.chars().filter(|c| c.is_ascii_digit() || *c == '.').collect();
            let version = if digits.is_empty() { None } else { Some(dotted_version(&digits)) };
            return Some((Engine::Mysql, base.to_string(), version));
        }
    }

    if lower == "mongodb" || lower == "mongod" || lower.starts_with("mongod@") {
        return Some((Engine::Mongodb, base.to_string(), None));
    }
    None
}

fn classify_postgresql(
    base: &str,
    rest_raw: &str,
    rest: &str,
) -> Option<(Engine, String, Option<String>)> {
    // Unité à instance : `postgresql@16-main` → instance « 16-main », version 16.
    if let Some(instance) = rest.strip_prefix('@') {
        if instance.is_empty() {
            return None;
        }
        let version = leading_version(instance);
        return Some((Engine::Postgresql, rest_raw[1..].to_string(), version));
    }
    // `postgresql`, `postgresql-16`, `postgresql-x64-16`, `postgresql-9.6`.
    if rest.is_empty() {
        return Some((Engine::Postgresql, base.to_string(), None));
    }
    let tail = rest.strip_prefix('-')?;
    let version = tail.rsplit('-').next().and_then(leading_version);
    Some((Engine::Postgresql, base.to_string(), version))
}

/// « 16-main » → « 16 », « 9.6 » → « 9.6 », « x64 » → rien.
fn leading_version(text: &str) -> Option<String> {
    let version: String =
        text.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect::<String>();
    let version = version.trim_matches('.').to_string();
    if version.is_empty() { None } else { Some(version) }
}

/// « 80 » → « 8.0 », « 57 » → « 5.7 », « 10.6 » inchangé, « 8 » inchangé.
fn dotted_version(digits: &str) -> String {
    if digits.contains('.') || digits.len() != 2 {
        return digits.to_string();
    }
    format!("{}.{}", &digits[..1], &digits[1..])
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
/// Lit `systemctl list-units --type=service --all --plain --no-legend`.
pub fn parse_systemctl_units(text: &str) -> Vec<ServiceFact> {
    let mut services = Vec::new();
    for line in text.lines() {
        // Les unités en échec portent une puce selon la version de systemd.
        let line = line.trim_start_matches(|c: char| !c.is_ascii_alphanumeric());
        let mut fields = line.split_whitespace();
        let (Some(unit), Some(_load), Some(active), Some(sub)) =
            (fields.next(), fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        if !unit.ends_with(".service") {
            continue;
        }
        services.push(ServiceFact {
            name: unit.to_string(),
            display: String::new(),
            running: active == "active" && sub == "running",
            path: None,
        });
    }
    services
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
/// Lit `systemctl list-unit-files --type=service --no-legend` : les unités
/// installées mais jamais chargées (donc absentes de `list-units`) comptent pour
/// des services arrêtés.
pub fn parse_systemctl_unit_files(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let unit = fields.next()?;
            let state = fields.next().unwrap_or("");
            (unit.ends_with(".service") && state != "masked").then(|| unit.to_string())
        })
        .collect()
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
/// Lit `/proc/net/tcp` ou `tcp6` : les ports à l'état LISTEN (`0A`).
pub fn parse_proc_net_tcp(text: &str) -> BTreeSet<u16> {
    text.lines()
        .skip(1)
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let _slot = fields.next()?;
            let local = fields.next()?;
            let _remote = fields.next()?;
            let state = fields.next()?;
            if state != "0A" {
                return None;
            }
            u16::from_str_radix(local.rsplit(':').next()?, 16).ok()
        })
        .collect()
}

#[cfg_attr(not(windows), allow(dead_code))]
/// Sortie du script PowerShell : une ligne par fait, champs séparés par `|`.
///
/// - `SVC|nom|nom d'affichage|état|chemin`
/// - `SQL|instance|version|édition|dossier de sauvegarde|port`
/// - `PORT|numéro`
pub fn parse_windows_facts(text: &str) -> Facts {
    let mut facts = Facts::default();
    for line in text.lines() {
        let fields: Vec<&str> = line.trim().split('|').collect();
        let text_of = |i: usize| fields.get(i).map(|s| s.trim()).filter(|s| !s.is_empty());
        match fields.first().copied() {
            Some("SVC") => {
                let Some(name) = text_of(1) else { continue };
                facts.services.push(ServiceFact {
                    name: name.to_string(),
                    display: text_of(2).unwrap_or("").to_string(),
                    running: text_of(3).is_some_and(|s| s.eq_ignore_ascii_case("running")),
                    path: text_of(4).map(str::to_string),
                });
            }
            Some("SQL") => {
                let Some(name) = text_of(1) else { continue };
                facts.sql_instances.push(SqlInstanceFact {
                    name: name.to_string(),
                    version: text_of(2).map(str::to_string),
                    edition: text_of(3).map(str::to_string),
                    backup_dir: text_of(4).map(str::to_string),
                    port: text_of(5).and_then(|p| p.parse().ok()),
                });
            }
            Some("PORT") => {
                if let Some(port) = text_of(1).and_then(|p| p.parse().ok()) {
                    facts.ports.insert(port);
                }
            }
            _ => {}
        }
    }
    facts
}

/// Dossier de l'exécutable d'un service Windows : `PathName` est soit entre
/// guillemets (`"C:\Program Files\X\x.exe" -arg`), soit un chemin nu terminé par
/// `.exe`.
pub fn exe_dir(path_name: &str) -> Option<String> {
    let trimmed = path_name.trim();
    let exe = if let Some(quoted) = trimmed.strip_prefix('"') {
        quoted.split('"').next()?.to_string()
    } else {
        let lower = trimmed.to_ascii_lowercase();
        let end = lower.find(".exe").map(|i| i + 4).unwrap_or(trimmed.len());
        trimmed[..end].to_string()
    };
    let cut = exe.rfind(['\\', '/'])?;
    if cut == 0 { None } else { Some(exe[..cut].to_string()) }
}

/// Interprète les faits : une entrée par base. Pure, donc testable sans machine.
pub fn detect(facts: &Facts) -> Vec<Detected> {
    let mut found: Vec<Detected> = Vec::new();

    for service in &facts.services {
        let Some((engine, instance, version)) = classify_service(&service.name, &service.display)
        else {
            continue;
        };
        let key = instance.to_ascii_lowercase();
        if let Some(existing) =
            found.iter_mut().find(|d| d.engine == engine && d.instance.to_ascii_lowercase() == key)
        {
            existing.running |= service.running;
            continue;
        }
        found.push(Detected {
            engine,
            instance,
            version,
            running: service.running,
            port: None,
            backup_dir: None,
            exe_dir: service.path.as_deref().and_then(exe_dir),
        });
    }

    // L'unité `postgresql.service` de Debian n'est qu'une enveloppe : quand les
    // unités `postgresql@…` existent, elle ne désigne aucune base.
    let has_pg_instances =
        found.iter().any(|d| d.engine == Engine::Postgresql && d.instance != "postgresql");
    if has_pg_instances {
        found.retain(|d| !(d.engine == Engine::Postgresql && d.instance == "postgresql"));
    }

    // Registre SQL Server : version, édition, port, dossier de sauvegarde.
    for sql in &facts.sql_instances {
        if let Some(d) = found
            .iter_mut()
            .find(|d| d.engine == Engine::Sqlserver && d.instance.eq_ignore_ascii_case(&sql.name))
        {
            d.version = match (&sql.version, &sql.edition) {
                (Some(v), Some(e)) => Some(format!("{v} ({e})")),
                (Some(v), None) => Some(v.clone()),
                _ => d.version.clone(),
            };
            d.port = sql.port;
            d.backup_dir = sql.backup_dir.clone();
        }
    }

    // Ports : le port par défaut d'un moteur n'est attribué que s'il n'y a qu'une
    // seule instance de ce moteur — sinon on ne sait pas laquelle l'écoute.
    for engine in Engine::ALL {
        let count = found.iter().filter(|d| d.engine == engine).count();
        let listening = facts.ports.contains(&engine.default_port());
        if count == 1 && listening {
            if let Some(d) = found.iter_mut().find(|d| d.engine == engine) {
                d.port.get_or_insert(engine.default_port());
                // Une instance qui écoute est en marche, quoi qu'en dise l'unité
                // (service lancé à la main, ou sous un autre nom).
                d.running = true;
            }
        } else if count == 0 && listening {
            // Rien d'installé comme service mais le port répond (conteneur, binaire
            // lancé à la main) : la base existe quand même.
            found.push(Detected {
                engine,
                instance: "default".to_string(),
                version: None,
                running: true,
                port: Some(engine.default_port()),
                backup_dir: None,
                exe_dir: None,
            });
        }
    }

    // Une seule installation de PostgreSQL et une version lisible dans les
    // dossiers : c'est la sienne.
    let pg = found.iter().filter(|d| d.engine == Engine::Postgresql).count();
    if pg == 1
        && facts.pg_versions.len() == 1
        && let Some(d) = found.iter_mut().find(|d| d.engine == Engine::Postgresql)
    {
        d.version.get_or_insert_with(|| facts.pg_versions[0].clone());
    }

    found.sort_by(|a, b| a.engine.cmp(&b.engine).then_with(|| a.instance.cmp(&b.instance)));
    found.truncate(MAX_DATABASES);
    found
}

/// Les chemins de sauvegarde à parcourir pour une base, l'âge maximal toléré, et
/// si les chemins viennent de `agent.yaml` (`true`) ou des emplacements usuels.
pub fn backup_patterns(
    db: &Detected,
    config: &DatabasesConfig,
    windows: bool,
) -> (Vec<String>, u64, bool) {
    let declared: Vec<&BackupRule> =
        config.backups.iter().filter(|r| r.engine == db.engine).collect();
    if !declared.is_empty() {
        let paths = declared.iter().flat_map(|r| r.paths.iter().cloned()).collect();
        let age = declared.iter().map(|r| r.max_age_hours).min().unwrap_or(DEFAULT_MAX_AGE_HOURS);
        return (paths, age, true);
    }
    (default_patterns(db, windows), DEFAULT_MAX_AGE_HOURS, false)
}

fn join(dir: &str, leaf: &str, windows: bool) -> String {
    let sep = if windows { '\\' } else { '/' };
    format!("{}{sep}{leaf}", dir.trim_end_matches(['\\', '/']))
}

/// Emplacements usuels des sauvegardes, par moteur.
pub fn default_patterns(db: &Detected, windows: bool) -> Vec<String> {
    let fixed = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    match db.engine {
        Engine::Postgresql if !windows => fixed(&[
            "/var/backups/postgresql/*",
            "/var/lib/pgbackrest/backup/*/backup.info",
            "/var/lib/barman/*/base/*/backup.info",
        ]),
        Engine::Mysql if !windows => fixed(&[
            "/var/backups/mysql/*",
            "/var/backups/mysql/*/*",
            "/var/backups/mariadb/*",
            "/var/backups/*.sql.gz",
        ]),
        Engine::Mongodb if !windows => fixed(&["/var/backups/mongodb/*"]),
        Engine::Sqlserver => {
            let dir = db.backup_dir.clone().or_else(|| {
                // SQL Server pour Linux sauvegarde par défaut à côté des données.
                (!windows).then(|| "/var/opt/mssql/data".to_string())
            });
            match dir {
                Some(dir) => vec![join(&dir, "*.bak", windows), join(&dir, "*.trn", windows)],
                None => Vec::new(),
            }
        }
        // Dossier de sauvegarde du Centre de contrôle HFSQL : son emplacement se
        // règle dans le Centre, l'agent essaie les sous-dossiers usuels à côté du
        // serveur et laisse `agent.yaml` dire le reste.
        Engine::Hfsql => match &db.exe_dir {
            Some(dir) => vec![
                join(&join(dir, "Backup", windows), "*", windows),
                join(&join(dir, "Backups", windows), "*", windows),
                join(&join(dir, "Sauvegarde", windows), "*", windows),
            ],
            None => Vec::new(),
        },
        _ => Vec::new(),
    }
}

/// Verdict sur la fraîcheur : `(âge en secondes si un fichier existe, en retard)`.
/// `None` : rien à dire (aucun fichier, et l'emplacement n'était pas déclaré).
pub fn judge(
    now_ms: i64,
    newest_ms: Option<i64>,
    max_age_hours: u64,
    declared: bool,
) -> Option<(Option<u64>, bool)> {
    match newest_ms {
        Some(newest) => {
            let age = (now_ms - newest).max(0) as u64 / 1000;
            Some((Some(age), age > max_age_hours.saturating_mul(3600)))
        }
        None if declared => Some((None, true)),
        None => None,
    }
}

// ----------------------------------------------------------------- jokers

/// Le nom correspond-il au motif (`*` = tout, `?` = un caractère) ?
pub fn wildcard_match(pattern: &str, name: &str, case_insensitive: bool) -> bool {
    let (p, n): (Vec<char>, Vec<char>) = if case_insensitive {
        (pattern.to_lowercase().chars().collect(), name.to_lowercase().chars().collect())
    } else {
        (pattern.chars().collect(), name.chars().collect())
    };
    let (mut pi, mut ni) = (0usize, 0usize);
    let (mut star, mut mark) = (None::<usize>, 0usize);
    while ni < n.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == n[ni]) && p[pi] != '*' {
            pi += 1;
            ni += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ni;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ni = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

fn has_wildcard(segment: &str) -> bool {
    segment.contains(['*', '?'])
}

/// Sépare un motif en (racine, segments). Les `\` ne sont des séparateurs que sous
/// Windows : sous Linux, ce sont des caractères de nom de fichier.
pub fn split_pattern(pattern: &str, windows: bool) -> (String, Vec<String>) {
    let norm = if windows { pattern.trim().replace('\\', "/") } else { pattern.trim().to_string() };
    let (root, rest) = if norm.starts_with("//") {
        ("//".to_string(), norm.trim_start_matches('/').to_string())
    } else if norm.starts_with('/') {
        ("/".to_string(), norm.trim_start_matches('/').to_string())
    } else if norm.len() >= 2 && norm.as_bytes()[1] == b':' {
        (format!("{}/", &norm[..2]), norm[2..].trim_start_matches('/').to_string())
    } else {
        (String::new(), norm)
    };
    (root, rest.split('/').filter(|s| !s.is_empty()).map(str::to_string).collect())
}

/// Ce que l'agent retient d'une sauvegarde : sa date et sa taille.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundBackup {
    pub modified_ms: i64,
    /// `None` pour un dossier (sauvegarde par répertoire datée).
    pub size_bytes: Option<u64>,
}

/// Les entrées qui répondent à un motif, au plus [`MAX_SCANNED_ENTRIES`].
fn expand(pattern: &str, windows: bool) -> Vec<PathBuf> {
    let (root, segments) = split_pattern(pattern, windows);
    if segments.is_empty() {
        return Vec::new();
    }
    let mut current: Vec<PathBuf> = vec![PathBuf::from(if root.is_empty() { "." } else { &root })];
    let mut budget = MAX_SCANNED_ENTRIES;
    for (i, segment) in segments.iter().enumerate() {
        let last = i + 1 == segments.len();
        let mut next = Vec::new();
        for dir in &current {
            if !has_wildcard(segment) {
                let candidate = dir.join(segment);
                if candidate.exists() {
                    next.push(candidate);
                }
                continue;
            }
            let Ok(entries) = std::fs::read_dir(dir) else { continue };
            for entry in entries.flatten() {
                if budget == 0 {
                    break;
                }
                budget -= 1;
                let name = entry.file_name().to_string_lossy().into_owned();
                if !wildcard_match(segment, &name, windows) {
                    continue;
                }
                // Un segment intermédiaire doit être un dossier.
                if !last && !entry.path().is_dir() {
                    continue;
                }
                next.push(entry.path());
            }
        }
        current = next;
        if current.is_empty() {
            break;
        }
    }
    current
}

/// Le fichier ou dossier le plus récent parmi les motifs.
pub fn newest_match(patterns: &[String], windows: bool) -> Option<FoundBackup> {
    let mut best: Option<FoundBackup> = None;
    for pattern in patterns {
        for path in expand(pattern, windows) {
            let Some(found) = describe(&path) else { continue };
            if best.as_ref().is_none_or(|b| found.modified_ms > b.modified_ms) {
                best = Some(found);
            }
        }
    }
    best
}

fn describe(path: &Path) -> Option<FoundBackup> {
    let meta = std::fs::metadata(path).ok()?;
    let modified = meta.modified().ok()?;
    let modified_ms = modified.duration_since(UNIX_EPOCH).ok()?.as_millis() as i64;
    Some(FoundBackup { modified_ms, size_bytes: meta.is_file().then_some(meta.len()) })
}

// ------------------------------------------------------------------- rapport

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupStat {
    pub modified_ms: Option<i64>,
    pub size_bytes: Option<u64>,
    pub age_secs: Option<u64>,
    pub max_age_hours: u64,
    pub overdue: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseStat {
    pub db: Detected,
    /// `None` : aucun emplacement de sauvegarde connu pour cette base.
    pub backup: Option<BackupStat>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DatabasesReport {
    pub databases: Vec<DatabaseStat>,
}

/// Associe sauvegardes et bases. `lookup` cherche le fichier le plus récent : en
/// production c'est [`newest_match`], dans les tests une table.
pub fn build_report<F>(
    detected: Vec<Detected>,
    config: &DatabasesConfig,
    windows: bool,
    now_ms: i64,
    lookup: F,
) -> DatabasesReport
where
    F: Fn(&[String]) -> Option<FoundBackup>,
{
    let databases = detected
        .into_iter()
        .map(|db| {
            let (patterns, max_age_hours, declared) = backup_patterns(&db, config, windows);
            let found = if patterns.is_empty() { None } else { lookup(&patterns) };
            let backup =
                judge(now_ms, found.as_ref().map(|f| f.modified_ms), max_age_hours, declared).map(
                    |(age_secs, overdue)| BackupStat {
                        modified_ms: found.as_ref().map(|f| f.modified_ms),
                        size_bytes: found.as_ref().and_then(|f| f.size_bytes),
                        age_secs,
                        max_age_hours,
                        overdue,
                    },
                );
            DatabaseStat { db, backup }
        })
        .collect();
    DatabasesReport { databases }
}

/// Traduit le rapport en échantillons.
pub fn samples(report: &DatabasesReport, now_ms: i64) -> Vec<Sample> {
    let mut samples = Vec::with_capacity(report.databases.len() * 6);
    for stat in report.databases.iter().take(MAX_DATABASES) {
        let db = &stat.db;
        let gauge = |metric: &str, value: f64| {
            Sample::new(metric, value, MetricKind::Gauge, now_ms)
                .with_label("engine", db.engine.as_str())
                .with_label("instance", &db.instance)
        };
        samples.push(gauge("agent_database_up", f64::from(u8::from(db.running))));
        if let Some(port) = db.port {
            samples.push(gauge("agent_database_port", f64::from(port)));
        }
        if let Some(version) = &db.version {
            samples.push(gauge("agent_database_info", 1.0).with_label("version", version));
        }
        if let Some(backup) = &stat.backup {
            samples
                .push(gauge("agent_database_backup_overdue", f64::from(u8::from(backup.overdue))));
            samples.push(gauge(
                "agent_database_backup_max_age_seconds",
                (backup.max_age_hours * 3600) as f64,
            ));
            if let Some(age) = backup.age_secs {
                samples.push(gauge("agent_database_backup_age_seconds", age as f64));
            }
            if let Some(ms) = backup.modified_ms {
                samples.push(gauge(
                    "agent_database_backup_last_timestamp_seconds",
                    (ms / 1000) as f64,
                ));
            }
            if let Some(size) = backup.size_bytes {
                samples.push(gauge("agent_database_backup_size_bytes", size as f64));
            }
        }
    }
    samples
}

// -------------------------------------------------------------- lecture réelle

const IS_WINDOWS: bool = cfg!(windows);

/// Lit les faits de la machine. Rend `None` sur un système sans prise en charge.
#[cfg(target_os = "linux")]
async fn gather_facts() -> Option<Facts> {
    async fn systemctl(args: &[&str]) -> String {
        let output = tokio::time::timeout(
            COMMAND_TIMEOUT,
            tokio::process::Command::new("systemctl")
                .args(args)
                .env("LC_ALL", "C")
                .kill_on_drop(true)
                .output(),
        )
        .await;
        match output {
            Ok(Ok(output)) => String::from_utf8_lossy(&output.stdout).into_owned(),
            _ => String::new(),
        }
    }
    let units = systemctl(&[
        "list-units",
        "--type=service",
        "--all",
        "--plain",
        "--no-legend",
        "--no-pager",
    ])
    .await;
    let files =
        systemctl(&["list-unit-files", "--type=service", "--plain", "--no-legend", "--no-pager"])
            .await;

    let mut services = parse_systemctl_units(&units);
    for name in parse_systemctl_unit_files(&files) {
        if !services.iter().any(|s| s.name == name) {
            services.push(ServiceFact { name, display: String::new(), running: false, path: None });
        }
    }

    let mut ports = BTreeSet::new();
    for file in ["/proc/net/tcp", "/proc/net/tcp6"] {
        if let Ok(text) = tokio::fs::read_to_string(file).await {
            ports.extend(parse_proc_net_tcp(&text));
        }
    }

    let mut pg_versions = Vec::new();
    if let Ok(entries) = std::fs::read_dir("/usr/lib/postgresql") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                pg_versions.push(name);
            }
        }
    }
    Some(Facts { services, ports, sql_instances: Vec::new(), pg_versions })
}

/// Script PowerShell en lecture seule : services, registre SQL Server, ports.
#[cfg(windows)]
const FACTS_SCRIPT: &str = r#"
$ErrorActionPreference = 'SilentlyContinue'
Get-CimInstance Win32_Service | Where-Object {
  $_.Name -match '^(MSSQL|SQLEXPRESS|postgresql|MySQL|MariaDB|Mongo)' -or $_.DisplayName -match 'HFSQL|PC SOFT'
} | ForEach-Object { "SVC|$($_.Name)|$($_.DisplayName)|$($_.State)|$($_.PathName)" }
$base = 'HKLM:\SOFTWARE\Microsoft\Microsoft SQL Server'
$names = Get-ItemProperty "$base\Instance Names\SQL"
if ($names) {
  $names.PSObject.Properties | Where-Object { $_.Name -notlike 'PS*' } | ForEach-Object {
    $iid = $_.Value
    $setup = Get-ItemProperty "$base\$iid\Setup"
    $srv = Get-ItemProperty "$base\$iid\MSSQLServer"
    $tcp = Get-ItemProperty "$base\$iid\MSSQLServer\SuperSocketNetLib\Tcp\IPAll"
    "SQL|$($_.Name)|$($setup.Version)|$($setup.Edition)|$($srv.BackupDirectory)|$($tcp.TcpPort)"
  }
}
Get-NetTCPConnection -State Listen | Select-Object -ExpandProperty LocalPort -Unique | ForEach-Object { "PORT|$_" }
"#;

#[cfg(windows)]
async fn gather_facts() -> Option<Facts> {
    let mut command = tokio::process::Command::new("powershell.exe");
    command
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            FACTS_SCRIPT,
        ])
        .stdin(std::process::Stdio::null())
        .kill_on_drop(true)
        .creation_flags(0x0800_0000);
    let output = tokio::time::timeout(COMMAND_TIMEOUT, command.output()).await.ok()?.ok()?;
    if !output.status.success() {
        return None;
    }
    Some(parse_windows_facts(&String::from_utf8_lossy(&output.stdout)))
}

/// macOS, FreeBSD : pas de lecture prévue, le module reste muet.
#[cfg(not(any(target_os = "linux", windows)))]
async fn gather_facts() -> Option<Facts> {
    None
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

async fn read_all(config: DatabasesConfig) -> Option<DatabasesReport> {
    let facts = gather_facts().await?;
    let detected = detect(&facts);
    if detected.is_empty() {
        return None;
    }
    // Le parcours des dossiers bloque : hors du runtime asynchrone.
    tokio::task::spawn_blocking(move || {
        build_report(detected, &config, IS_WINDOWS, now_ms(), |patterns| {
            newest_match(patterns, IS_WINDOWS)
        })
    })
    .await
    .ok()
}

/// Sonde conservée d'un cycle à l'autre : le dernier rapport, et la lecture de
/// fond qui prépare le suivant (même schéma que ZFS).
pub struct DatabasesProbe {
    config: DatabasesConfig,
    last: Option<DatabasesReport>,
    refreshed_at: Option<tokio::time::Instant>,
    task: Option<JoinHandle<Option<DatabasesReport>>>,
    announced: bool,
}

impl DatabasesProbe {
    pub fn new(config: &DatabasesConfig) -> Self {
        if !config.enabled {
            debug!("database detection disabled by the configuration");
        }
        Self {
            config: config.clone(),
            last: None,
            refreshed_at: None,
            task: None,
            announced: false,
        }
    }

    /// Un cycle. `None` : aucune base sur cette machine, ou première lecture pas
    /// encore aboutie.
    pub async fn read(&mut self) -> Option<DatabasesReport> {
        if !self.config.enabled {
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
                    Ok(report) => {
                        if !self.announced && report.is_some() {
                            debug!("databases found on this machine");
                            self.announced = true;
                        }
                        self.last = report;
                    }
                    Err(error) => debug!(%error, "database reading aborted"),
                }
            }
        }
        self.last.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn svc(name: &str, display: &str, running: bool) -> ServiceFact {
        ServiceFact { name: name.into(), display: display.into(), running, path: None }
    }

    fn classify(name: &str) -> Option<(Engine, String, Option<String>)> {
        classify_service(name, "")
    }

    #[test]
    fn sql_server_service_names() {
        assert_eq!(classify("MSSQLSERVER"), Some((Engine::Sqlserver, "MSSQLSERVER".into(), None)));
        assert_eq!(
            classify("MSSQL$SQLEXPRESS"),
            Some((Engine::Sqlserver, "SQLEXPRESS".into(), None))
        );
        assert_eq!(classify("MSSQL$PROD01"), Some((Engine::Sqlserver, "PROD01".into(), None)));
        assert_eq!(classify("SQLEXPRESS"), Some((Engine::Sqlserver, "SQLEXPRESS".into(), None)));
        assert_eq!(
            classify("mssql-server.service"),
            Some((Engine::Sqlserver, "MSSQLSERVER".into(), None))
        );
        // Services annexes de SQL Server : jamais pris pour une base.
        for side in [
            "SQLAgent$SQLEXPRESS",
            "SQLBrowser",
            "SQLWriter",
            "MSSQLFDLauncher",
            "MSSQLServerOLAPService",
            "SQLTELEMETRY",
            "MSSQL$",
        ] {
            assert_eq!(classify(side), None, "{side}");
        }
    }

    #[test]
    fn postgresql_service_names_and_versions() {
        assert_eq!(
            classify("postgresql-x64-16"),
            Some((Engine::Postgresql, "postgresql-x64-16".into(), Some("16".into())))
        );
        assert_eq!(
            classify("postgresql-x64-9.6"),
            Some((Engine::Postgresql, "postgresql-x64-9.6".into(), Some("9.6".into())))
        );
        assert_eq!(
            classify("postgresql-16.service"),
            Some((Engine::Postgresql, "postgresql-16".into(), Some("16".into())))
        );
        assert_eq!(
            classify("postgresql@16-main.service"),
            Some((Engine::Postgresql, "16-main".into(), Some("16".into())))
        );
        assert_eq!(classify("postgresql@.service"), None);
        assert_eq!(classify("postgresql.service").map(|c| c.0), Some(Engine::Postgresql));
        assert_eq!(classify("postgresql-exporter.service"), None);
        assert_eq!(classify("pgbouncer.service"), None);
    }

    #[test]
    fn mysql_mariadb_mongodb_and_hfsql_names() {
        assert_eq!(
            classify("MySQL80"),
            Some((Engine::Mysql, "MySQL80".into(), Some("8.0".into())))
        );
        assert_eq!(
            classify("MySQL57"),
            Some((Engine::Mysql, "MySQL57".into(), Some("5.7".into())))
        );
        assert_eq!(classify("MariaDB"), Some((Engine::Mysql, "MariaDB".into(), None)));
        assert_eq!(classify("mariadb.service").map(|c| c.0), Some(Engine::Mysql));
        assert_eq!(classify("mysqld.service").map(|c| c.0), Some(Engine::Mysql));
        assert_eq!(classify("mysql@dev.service"), Some((Engine::Mysql, "dev".into(), None)));
        assert_eq!(classify("MySQLRouter"), None);
        assert_eq!(classify("mysqld_exporter.service"), None);
        assert_eq!(classify("mysqlfoo"), None);
        assert_eq!(classify("mongod.service").map(|c| c.0), Some(Engine::Mongodb));
        assert_eq!(classify("MongoDB").map(|c| c.0), Some(Engine::Mongodb));
        assert_eq!(
            classify_service("HFSQLCS28", "HFSQL Control Center 28").map(|c| c.0),
            Some(Engine::Hfsql)
        );
        assert_eq!(
            classify_service("pcs_svc", "PC SOFT HFSQL Server").map(|c| c.0),
            Some(Engine::Hfsql)
        );
        assert_eq!(classify("sshd.service"), None);
    }

    #[test]
    fn systemctl_output_is_parsed() {
        let units = "\
postgresql@16-main.service loaded active   running PostgreSQL Cluster 16-main
postgresql.service         loaded active   exited  PostgreSQL RDBMS
● mariadb.service          loaded failed   failed  MariaDB database server
sshd.service               loaded active   running OpenBSD Secure Shell server
not a unit line
";
        let services = parse_systemctl_units(units);
        assert_eq!(services.len(), 4);
        assert_eq!(services[0].name, "postgresql@16-main.service");
        assert!(services[0].running);
        assert!(!services[1].running, "exited n'est pas en marche");
        assert_eq!(services[2].name, "mariadb.service");
        assert!(!services[2].running);

        let files =
            "mongod.service enabled enabled\nsnapd.service masked enabled\nfoo.timer enabled\n";
        assert_eq!(parse_systemctl_unit_files(files), vec!["mongod.service".to_string()]);
    }

    #[test]
    fn listening_ports_are_read_from_proc() {
        let text = "  sl  local_address rem_address   st tx_queue rx_queue\n\
   0: 00000000:1538 00000000:0000 0A 00000000:00000000 00:00000000 00000000   113        0 1 1\n\
   1: 0100007F:0CEA 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 2 1\n\
   2: 0A00A8C0:A1B2 0A00A8C1:0050 01 00000000:00000000 00:00000000 00000000     0        0 3 1\n";
        let ports = parse_proc_net_tcp(text);
        assert_eq!(ports.into_iter().collect::<Vec<_>>(), vec![3306, 5432]);
    }

    #[test]
    fn windows_script_output_is_parsed() {
        let text = "\
SVC|MSSQL$SQLEXPRESS|SQL Server (SQLEXPRESS)|Running|\"C:\\Program Files\\Microsoft SQL Server\\MSSQL16.SQLEXPRESS\\MSSQL\\Binn\\sqlservr.exe\" -sSQLEXPRESS
SVC|postgresql-x64-16|postgresql-x64-16 - PostgreSQL Server 16|Stopped|\"C:\\Program Files\\PostgreSQL\\16\\bin\\pg_ctl.exe\" runservice
SQL|SQLEXPRESS|16.0.1000.6|Express Edition (64-bit)|C:\\SQLBackups|
PORT|1433
PORT|4900
garbage
";
        let facts = parse_windows_facts(text);
        assert_eq!(facts.services.len(), 2);
        assert!(facts.services[0].running);
        assert!(!facts.services[1].running);
        assert_eq!(facts.sql_instances[0].backup_dir.as_deref(), Some("C:\\SQLBackups"));
        assert_eq!(facts.sql_instances[0].port, None);
        assert_eq!(facts.ports.into_iter().collect::<Vec<_>>(), vec![1433, 4900]);
    }

    #[test]
    fn executable_directory_of_a_windows_service() {
        assert_eq!(
            exe_dir("\"C:\\Program Files\\PC SOFT\\HFSQL\\HFSQLCS.exe\" -service"),
            Some("C:\\Program Files\\PC SOFT\\HFSQL".to_string())
        );
        assert_eq!(exe_dir("D:\\HF\\hfsql.exe /run"), Some("D:\\HF".to_string()));
        assert_eq!(exe_dir("hfsql.exe"), None);
    }

    #[test]
    fn detection_merges_services_registry_and_ports() {
        let facts = Facts {
            services: vec![
                svc("MSSQL$SQLEXPRESS", "", true),
                svc("MSSQLSERVER", "", false),
                svc("postgresql-x64-16", "", true),
                svc("SQLBrowser", "", true),
            ],
            ports: [5432, 4900].into_iter().collect(),
            sql_instances: vec![SqlInstanceFact {
                name: "SQLEXPRESS".into(),
                version: Some("16.0.1000.6".into()),
                edition: Some("Express Edition".into()),
                backup_dir: Some("D:\\SQL".into()),
                port: Some(50123),
            }],
            pg_versions: vec![],
        };
        let found = detect(&facts);
        let by = |engine: Engine, instance: &str| {
            found.iter().find(|d| d.engine == engine && d.instance == instance).cloned()
        };
        let express = by(Engine::Sqlserver, "SQLEXPRESS").expect("express");
        assert!(express.running);
        assert_eq!(express.version.as_deref(), Some("16.0.1000.6 (Express Edition)"));
        assert_eq!(express.port, Some(50123));
        assert_eq!(express.backup_dir.as_deref(), Some("D:\\SQL"));
        let default = by(Engine::Sqlserver, "MSSQLSERVER").expect("default");
        assert!(!default.running);
        assert_eq!(default.port, None, "deux instances : le port 1433 n'est attribué à aucune");
        let pg = by(Engine::Postgresql, "postgresql-x64-16").expect("pg");
        assert_eq!(pg.port, Some(5432));
        // Port 4900 en écoute, aucun service : HFSQL est tout de même là.
        let hfsql = by(Engine::Hfsql, "default").expect("hfsql by port");
        assert!(hfsql.running);
        assert_eq!(hfsql.port, Some(4900));
        assert_eq!(found.len(), 4);
    }

    #[test]
    fn the_debian_wrapper_unit_is_not_a_database() {
        let facts = Facts {
            services: vec![
                svc("postgresql.service", "", true),
                svc("postgresql@16-main.service", "", true),
            ],
            ports: BTreeSet::new(),
            sql_instances: vec![],
            pg_versions: vec!["16".into()],
        };
        let found = detect(&facts);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].instance, "16-main");
        assert_eq!(found[0].version.as_deref(), Some("16"));
    }

    #[test]
    fn a_stopped_unit_with_a_listening_port_counts_as_running() {
        let facts = Facts {
            services: vec![svc("mariadb.service", "", false)],
            ports: [3306].into_iter().collect(),
            ..Facts::default()
        };
        assert!(detect(&facts)[0].running);
    }

    fn db(engine: Engine) -> Detected {
        Detected {
            engine,
            instance: "x".into(),
            version: None,
            running: true,
            port: None,
            backup_dir: None,
            exe_dir: None,
        }
    }

    #[test]
    fn default_backup_locations_per_engine() {
        let sql = Detected { backup_dir: Some("D:\\Backups\\".into()), ..db(Engine::Sqlserver) };
        assert_eq!(
            default_patterns(&sql, true),
            vec!["D:\\Backups\\*.bak".to_string(), "D:\\Backups\\*.trn".to_string()]
        );
        assert_eq!(
            default_patterns(&db(Engine::Sqlserver), false),
            vec!["/var/opt/mssql/data/*.bak", "/var/opt/mssql/data/*.trn"]
        );
        assert!(default_patterns(&db(Engine::Sqlserver), true).is_empty());
        assert!(
            default_patterns(&db(Engine::Postgresql), false)
                .contains(&"/var/backups/postgresql/*".to_string())
        );
        assert!(
            default_patterns(&db(Engine::Mysql), false)
                .contains(&"/var/backups/*.sql.gz".to_string())
        );
        let hf = Detected { exe_dir: Some("C:\\PC SOFT\\HFSQL".into()), ..db(Engine::Hfsql) };
        assert_eq!(default_patterns(&hf, true)[0], "C:\\PC SOFT\\HFSQL\\Backup\\*");
    }

    #[test]
    fn declared_paths_replace_the_defaults_and_the_strictest_age_wins() {
        let config = DatabasesConfig {
            backups: vec![
                BackupRule {
                    engine: Engine::Postgresql,
                    paths: vec!["/srv/dump/*.dump".into()],
                    max_age_hours: 48,
                },
                BackupRule {
                    engine: Engine::Postgresql,
                    paths: vec!["/srv/wal/*".into()],
                    max_age_hours: 24,
                },
            ],
            ..DatabasesConfig::default()
        };
        let (paths, age, declared) = backup_patterns(&db(Engine::Postgresql), &config, false);
        assert_eq!(paths, vec!["/srv/dump/*.dump", "/srv/wal/*"]);
        assert_eq!(age, 24);
        assert!(declared);
        let (paths, age, declared) = backup_patterns(&db(Engine::Mysql), &config, false);
        assert!(!paths.is_empty() && !declared);
        assert_eq!(age, DEFAULT_MAX_AGE_HOURS);
    }

    #[test]
    fn backup_age_and_verdict() {
        let now = 1_000_000_000_000;
        let hour = 3_600_000;
        // 3 h, limite 36 h : à jour.
        assert_eq!(judge(now, Some(now - 3 * hour), 36, false), Some((Some(3 * 3600), false)));
        // 40 h, limite 36 h : en retard.
        assert_eq!(judge(now, Some(now - 40 * hour), 36, true), Some((Some(40 * 3600), true)));
        // Pile à la limite : pas encore en retard.
        assert_eq!(judge(now, Some(now - 36 * hour), 36, true), Some((Some(36 * 3600), false)));
        // Fichier daté du futur (horloge) : âge nul.
        assert_eq!(judge(now, Some(now + hour), 36, true), Some((Some(0), false)));
        // Rien trouvé : en retard si l'emplacement était déclaré, muet sinon.
        assert_eq!(judge(now, None, 36, true), Some((None, true)));
        assert_eq!(judge(now, None, 36, false), None);
    }

    #[test]
    fn report_uses_the_lookup_and_emits_metrics() {
        let now = 2_000_000_000_000;
        let facts = Facts {
            services: vec![
                svc("postgresql@16-main.service", "", true),
                svc("mongod.service", "", true),
            ],
            ..Facts::default()
        };
        let report =
            build_report(detect(&facts), &DatabasesConfig::default(), false, now, |patterns| {
                patterns.iter().any(|p| p.contains("postgresql")).then(|| FoundBackup {
                    modified_ms: now - 2 * 3_600_000,
                    size_bytes: Some(1234),
                })
            });
        assert_eq!(report.databases.len(), 2);
        let pg = report.databases.iter().find(|d| d.db.engine == Engine::Postgresql).expect("pg");
        let backup = pg.backup.as_ref().expect("backup");
        assert_eq!(backup.age_secs, Some(7200));
        assert!(!backup.overdue);
        let mongo =
            report.databases.iter().find(|d| d.db.engine == Engine::Mongodb).expect("mongo");
        assert!(mongo.backup.is_none(), "emplacement usuel vide : pas de série overdue");

        let samples = samples(&report, now);
        let names: Vec<&str> = samples.iter().map(|s| s.metric.as_str()).collect();
        for expected in [
            "agent_database_up",
            "agent_database_info",
            "agent_database_backup_overdue",
            "agent_database_backup_age_seconds",
            "agent_database_backup_size_bytes",
            "agent_database_backup_last_timestamp_seconds",
        ] {
            assert!(names.contains(&expected), "{expected} manque");
        }
        assert_eq!(names.iter().filter(|n| **n == "agent_database_backup_overdue").count(), 1);
    }

    #[test]
    fn wildcards() {
        assert!(wildcard_match("*.bak", "db_full.bak", false));
        assert!(wildcard_match("*.sql.gz", "dump-2026.sql.gz", false));
        assert!(!wildcard_match("*.bak", "db.bak.tmp", false));
        assert!(wildcard_match("db?.dump", "db1.dump", false));
        assert!(!wildcard_match("db?.dump", "db12.dump", false));
        assert!(wildcard_match("*", "anything", false));
        assert!(wildcard_match("a*b*c", "aXXbYYc", false));
        assert!(!wildcard_match("*.BAK", "x.bak", false));
        assert!(wildcard_match("*.BAK", "x.bak", true));
    }

    #[test]
    fn patterns_are_split_by_platform() {
        assert_eq!(
            split_pattern("D:\\Backups\\SQL\\*.bak", true),
            (
                "D:/".to_string(),
                vec!["Backups".to_string(), "SQL".to_string(), "*.bak".to_string()]
            )
        );
        assert_eq!(
            split_pattern("/var/backups/postgresql/*", false),
            (
                "/".to_string(),
                vec![
                    "var".to_string(),
                    "backups".to_string(),
                    "postgresql".to_string(),
                    "*".to_string()
                ]
            )
        );
        assert_eq!(split_pattern("\\\\srv\\share\\*.bak", true).0, "//");
    }

    #[cfg(unix)]
    #[test]
    fn the_newest_entry_of_a_real_folder_is_found() {
        let dir = tempfile::tempdir().expect("tempdir");
        let old = dir.path().join("old.bak");
        let new = dir.path().join("new.bak");
        std::fs::write(&old, b"1").expect("write");
        std::fs::write(&new, b"12345").expect("write");
        std::fs::write(dir.path().join("ignored.txt"), b"x").expect("write");
        let past = SystemTime::now() - Duration::from_secs(10 * 3600);
        std::fs::File::options()
            .write(true)
            .open(&old)
            .expect("open")
            .set_modified(past)
            .expect("mtime");
        let pattern = format!("{}/*.bak", dir.path().display());
        let found = newest_match(&[pattern], false).expect("found");
        assert_eq!(found.size_bytes, Some(5));
        assert!(newest_match(&[format!("{}/*.dump", dir.path().display())], false).is_none());
        assert!(newest_match(&["/inexistant/dossier/*".to_string()], false).is_none());
    }

    #[tokio::test]
    async fn a_disabled_probe_reports_nothing() {
        let mut probe =
            DatabasesProbe::new(&DatabasesConfig { enabled: false, ..DatabasesConfig::default() });
        assert!(probe.read().await.is_none());
    }
}
