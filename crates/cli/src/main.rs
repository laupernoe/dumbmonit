//! `dumbmonit-cli` : pilote une instance DumbMonit depuis un terminal ou un
//! pipeline de déploiement.
//!
//! Le cœur est la configuration en fichiers (GitOps) : `plan` montre ce qu'un
//! fichier changerait, `apply` l'applique, `export` écrit l'instance sous forme
//! de fichier. Les références de secrets (`${VARIABLE}`, `${file:/chemin}`) sont
//! lues ici, sur la machine de l'opérateur, puis envoyées au serveur : un secret
//! n'est jamais dans le fichier versionné.
//!
//! Volontairement sans analyseur d'arguments ni runtime asynchrone : un client
//! HTTP bloquant, un lecteur YAML et serde suffisent.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

const USAGE: &str = "\
dumbmonit-cli: manage a DumbMonit instance from files and scripts.

USAGE
  dumbmonit-cli [--url URL] [--token TOKEN] [--json] <command>

COMMANDS
  login --url URL [--token TOKEN]   Save the server address and an admin API token
  export [-o FILE]                  Write the instance as a configuration file
  plan  FILE|DIR... [--prune]       Show what the files would change (changes nothing)
  apply FILE|DIR... [--prune]       Apply the files, in one transaction
  targets list                      List monitors
  silence create --name NAME --duration 2h [--target-id ID] [--match k=v]... [--comment TEXT]
                                    Open a maintenance window starting now

OPTIONS
  --url URL      Server address (or DUMBMONIT_URL)
  --token TOKEN  Admin API token (or DUMBMONIT_TOKEN); `login` stores it
  --json         Machine-readable output
  --prune        Delete file-managed objects that are no longer declared
  --check        With plan: exit with status 2 if anything would change
  -h, --help     This text

A FILE may be `-` (standard input). A DIR applies every .yaml/.yml file in it as one
document. Secrets in files are references: ${NAME} reads an environment variable,
${file:/path} reads a file (trailing newline removed).
";

#[derive(Default)]
struct Args {
    positional: Vec<String>,
    values: HashMap<String, Vec<String>>,
    switches: HashSet<String>,
}

/// Options qui ne portent pas de valeur.
const SWITCHES: &[&str] = &["json", "prune", "check", "help"];

fn parse_args(raw: &[String]) -> Result<Args, String> {
    let mut args = Args::default();
    let mut iter = raw.iter();
    while let Some(arg) = iter.next() {
        let name = match arg.as_str() {
            "-h" => "help",
            "-o" => "output",
            other if other.starts_with("--") && other.len() > 2 => &other[2..],
            _ => {
                args.positional.push(arg.clone());
                continue;
            }
        };
        if let Some((key, value)) = name.split_once('=') {
            args.values.entry(key.to_string()).or_default().push(value.to_string());
        } else if SWITCHES.contains(&name) {
            args.switches.insert(name.to_string());
        } else {
            let value = iter.next().ok_or_else(|| format!("--{name} needs a value"))?;
            args.values.entry(name.to_string()).or_default().push(value.clone());
        }
    }
    Ok(args)
}

impl Args {
    fn value(&self, name: &str) -> Option<&str> {
        self.values.get(name).and_then(|v| v.last()).map(String::as_str)
    }

    fn switch(&self, name: &str) -> bool {
        self.switches.contains(name)
    }
}

// --------------------------------------------------------------------------
// Connexion enregistrée
// --------------------------------------------------------------------------

fn settings_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|home| Path::new(&home).join(".config")))?;
    Some(base.join("dumbmonit").join("cli.json"))
}

fn load_settings() -> Value {
    settings_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_else(|| json!({}))
}

fn save_settings(url: &str, token: &str) -> Result<PathBuf, String> {
    let path = settings_path().ok_or("no home directory to store the login in")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    let body = serde_json::to_string_pretty(&json!({ "url": url, "token": token }))
        .map_err(|e| e.to_string())?;
    std::fs::write(&path, body).map_err(|e| format!("{}: {e}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(path)
}

struct Client {
    url: String,
    token: String,
    agent: ureq::Agent,
}

impl Client {
    fn connect(args: &Args) -> Result<Self, String> {
        let stored = load_settings();
        let pick = |flag: &str, env: &str, key: &str| -> Option<String> {
            args.value(flag)
                .map(str::to_string)
                .or_else(|| std::env::var(env).ok().filter(|v| !v.is_empty()))
                .or_else(|| stored[key].as_str().map(str::to_string))
        };
        let url = pick("url", "DUMBMONIT_URL", "url")
            .ok_or("no server address: use --url, DUMBMONIT_URL or `dumbmonit-cli login`")?;
        let token = pick("token", "DUMBMONIT_TOKEN", "token")
            .ok_or("no API token: use --token, DUMBMONIT_TOKEN or `dumbmonit-cli login`")?;
        Ok(Self::new(&url, &token))
    }

    fn new(url: &str, token: &str) -> Self {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            // Les erreurs HTTP portent un message utile dans leur corps : on le lit.
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(120)))
            .build()
            .into();
        Self { url: url.trim_end_matches('/').to_string(), token: token.to_string(), agent }
    }

    /// Appelle l'API. Rend le corps (texte) ou le message d'erreur du serveur.
    fn call(&self, method: &str, path: &str, body: Option<&Value>) -> Result<String, String> {
        let url = format!("{}{path}", self.url);
        let auth = format!("Bearer {}", self.token);
        let response = match (method, body) {
            ("GET", _) => self.agent.get(&url).header("Authorization", &auth).call(),
            ("DELETE", _) => self.agent.delete(&url).header("Authorization", &auth).call(),
            (_, Some(body)) => self
                .agent
                .post(&url)
                .header("Authorization", &auth)
                .header("Content-Type", "application/json")
                .send(body.to_string()),
            (_, None) => self
                .agent
                .post(&url)
                .header("Authorization", &auth)
                .header("Content-Type", "application/json")
                .send("{}"),
        };
        let mut response = response.map_err(|e| format!("{method} {url}: {e}"))?;
        let status = response.status().as_u16();
        let text = response.body_mut().read_to_string().map_err(|e| format!("{url}: {e}"))?;
        if (200..300).contains(&status) {
            return Ok(text);
        }
        let message = serde_json::from_str::<Value>(&text)
            .ok()
            .and_then(|v| v["error"].as_str().map(str::to_string))
            .unwrap_or_else(|| text.trim().to_string());
        Err(match status {
            401 => format!("{message} (the token is missing, wrong or revoked)"),
            403 => format!("{message} (an administrator or `write` API token is required)"),
            _ => format!("{message} (HTTP {status})"),
        })
    }

    fn json(&self, method: &str, path: &str, body: Option<&Value>) -> Result<Value, String> {
        let text = self.call(method, path, body)?;
        serde_json::from_str(&text).map_err(|e| format!("unexpected answer from {path}: {e}"))
    }
}

// --------------------------------------------------------------------------
// Fichiers de configuration
// --------------------------------------------------------------------------

const SECTIONS: &[&str] =
    &["targets", "rules", "rule_overrides", "channels", "silences", "status_pages"];

fn yaml_files(path: &Path) -> Result<Vec<PathBuf>, String> {
    if !path.is_dir() {
        return Ok(vec![path.to_path_buf()]);
    }
    let mut files: Vec<PathBuf> = std::fs::read_dir(path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|p| {
            p.is_file()
                && p.extension().and_then(|e| e.to_str()).is_some_and(|e| {
                    e.eq_ignore_ascii_case("yaml") || e.eq_ignore_ascii_case("yml")
                })
        })
        .collect();
    files.sort();
    if files.is_empty() {
        return Err(format!("{}: no .yaml file found", path.display()));
    }
    Ok(files)
}

/// Lit et fusionne les fichiers, puis résout les références de secrets.
fn load_config(inputs: &[String]) -> Result<Value, String> {
    if inputs.is_empty() {
        return Err("give at least one file or directory (or `-` for standard input)".into());
    }
    let mut merged: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    let mut version = None;
    for input in inputs {
        let sources: Vec<(String, String, Option<PathBuf>)> = if input == "-" {
            let mut text = String::new();
            std::io::stdin().read_to_string(&mut text).map_err(|e| format!("stdin: {e}"))?;
            vec![("standard input".into(), text, None)]
        } else {
            let mut out = Vec::new();
            for path in yaml_files(Path::new(input))? {
                let text = std::fs::read_to_string(&path)
                    .map_err(|e| format!("{}: {e}", path.display()))?;
                out.push((path.display().to_string(), text, path.parent().map(Path::to_path_buf)));
            }
            out
        };
        for (name, text, base) in sources {
            let mut doc: Value = serde_saphyr::from_str(&text)
                .map_err(|e| format!("{name}: not valid YAML: {e}"))?;
            let object = doc.as_object_mut().ok_or(format!("{name}: expected a mapping"))?;
            let file_version = object.remove("version").and_then(|v| v.as_u64());
            match (version, file_version) {
                (_, None) => return Err(format!("{name}: missing `version: 1`")),
                (Some(a), Some(b)) if a != b => {
                    return Err(format!("{name}: version {b} differs from {a}"));
                }
                (_, Some(b)) => version = Some(b),
            }
            for (key, value) in std::mem::take(object) {
                if !SECTIONS.contains(&key.as_str()) {
                    return Err(format!("{name}: unknown section `{key}`"));
                }
                let items = value.as_array().ok_or(format!("{name}: `{key}` must be a list"))?;
                for mut item in items.clone() {
                    resolve_item(&key, &mut item, base.as_deref())
                        .map_err(|e| format!("{name}: {e}"))?;
                    merged.entry(key.clone()).or_default().push(item);
                }
            }
        }
    }
    let mut out = serde_json::Map::new();
    out.insert("version".into(), json!(version.unwrap_or(1)));
    for (key, items) in merged {
        out.insert(key, Value::Array(items));
    }
    Ok(Value::Object(out))
}

/// Résout les références dans les seuls endroits qui portent des secrets.
fn resolve_item(section: &str, item: &mut Value, base: Option<&Path>) -> Result<(), String> {
    let (field, owner) = match section {
        "targets" => ("credential", "target"),
        "channels" => ("secrets", "channel"),
        _ => return Ok(()),
    };
    let label = item["name"].as_str().or(item["address"].as_str()).unwrap_or("?").to_string();
    if let Some(value) = item.get_mut(field) {
        resolve_value(value, base).map_err(|e| format!("{owner} \"{label}\": {e}"))?;
    }
    Ok(())
}

fn resolve_value(value: &mut Value, base: Option<&Path>) -> Result<(), String> {
    match value {
        Value::String(text) => *text = resolve_str(text, base)?,
        Value::Array(items) => {
            for item in items {
                resolve_value(item, base)?;
            }
        }
        Value::Object(map) => {
            for item in map.values_mut() {
                resolve_value(item, base)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Premier `${…}` valide : (début, fin exclue, contenu).
fn find_ref(text: &str) -> Option<(usize, usize, &str)> {
    let mut from = 0;
    while let Some(offset) = text[from..].find("${") {
        let start = from + offset;
        let rest = &text[start + 2..];
        if let Some(close) = rest.find('}') {
            let inner = &rest[..close];
            let name = inner.strip_prefix("file:").map_or_else(
                || {
                    let mut chars = inner.chars();
                    chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
                },
                |path| !path.trim().is_empty(),
            );
            if name {
                return Some((start, start + 2 + close + 1, inner));
            }
        }
        from = start + 2;
    }
    None
}

fn resolve_str(text: &str, base: Option<&Path>) -> Result<String, String> {
    let mut out = String::new();
    let mut rest = text;
    while let Some((start, end, inner)) = find_ref(rest) {
        out.push_str(&rest[..start]);
        match inner.strip_prefix("file:") {
            Some(path) => {
                let path = Path::new(path.trim());
                let full = match base {
                    Some(base) if path.is_relative() => base.join(path),
                    _ => path.to_path_buf(),
                };
                let content = std::fs::read_to_string(&full)
                    .map_err(|e| format!("cannot read {}: {e}", full.display()))?;
                out.push_str(content.trim_end_matches(['\n', '\r']));
            }
            None => out.push_str(
                &std::env::var(inner).map_err(|_| format!("the variable {inner} is not set"))?,
            ),
        }
        rest = &rest[end..];
    }
    out.push_str(rest);
    Ok(out)
}

// --------------------------------------------------------------------------
// Commandes
// --------------------------------------------------------------------------

fn print_plan(plan: &Value, applied: bool) {
    let count = |key: &str| plan[key].as_u64().unwrap_or(0);
    let verb = if applied { "Applied" } else { "Plan" };
    println!(
        "{verb}: {} to create, {} to update, {} to delete, {} unchanged",
        count("create"),
        count("update"),
        count("delete"),
        count("unchanged")
    );
    for change in plan["changes"].as_array().into_iter().flatten() {
        let mark = match change["action"].as_str() {
            Some("create") => "+",
            Some("delete") => "-",
            _ => "~",
        };
        let fields: Vec<&str> =
            change["fields"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
        let detail =
            if fields.is_empty() { String::new() } else { format!("  ({})", fields.join(", ")) };
        println!(
            "  {mark} {:<14} {}{detail}",
            change["section"].as_str().unwrap_or("?"),
            change["key"].as_str().unwrap_or("?")
        );
    }
    for warning in plan["warnings"].as_array().into_iter().flatten().filter_map(Value::as_str) {
        println!("  note: {warning}");
    }
    for owner in plan["unresolved"].as_array().into_iter().flatten().filter_map(Value::as_str) {
        println!("  warning: secret references of {owner} were not resolved");
    }
}

fn changes_pending(plan: &Value) -> bool {
    plan["changes"].as_array().is_some_and(|c| !c.is_empty())
}

fn run_config(command: &str, args: &Args) -> Result<ExitCode, String> {
    let client = Client::connect(args)?;
    let config = load_config(&args.positional[1..])?;
    let body = json!({ "config": config, "prune": args.switch("prune") });
    let applying = command == "apply";
    let path = if applying { "/api/config/apply" } else { "/api/config/plan" };
    let plan = client.json("POST", path, Some(&body))?;
    if args.switch("json") {
        println!("{}", serde_json::to_string_pretty(&plan).map_err(|e| e.to_string())?);
    } else {
        print_plan(&plan, applying);
    }
    if !applying && args.switch("check") && changes_pending(&plan) {
        return Ok(ExitCode::from(2));
    }
    Ok(ExitCode::SUCCESS)
}

fn run_export(args: &Args) -> Result<ExitCode, String> {
    let client = Client::connect(args)?;
    let text = client.call("GET", "/api/config/export", None)?;
    match args.value("output") {
        Some(path) => {
            std::fs::write(path, &text).map_err(|e| format!("{path}: {e}"))?;
            eprintln!(
                "Wrote {path}. Secrets are references: define the variables listed at its top."
            );
        }
        None => {
            std::io::stdout().write_all(text.as_bytes()).map_err(|e| e.to_string())?;
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn run_login(args: &Args) -> Result<ExitCode, String> {
    let url = args
        .value("url")
        .map(str::to_string)
        .or_else(|| std::env::var("DUMBMONIT_URL").ok())
        .ok_or("login needs --url")?;
    let token = match args
        .value("token")
        .map(str::to_string)
        .or_else(|| std::env::var("DUMBMONIT_TOKEN").ok())
    {
        Some(token) => token,
        None => {
            eprint!("Admin API token (Settings → API tokens, scope write): ");
            let _ = std::io::stderr().flush();
            let mut line = String::new();
            std::io::stdin().read_line(&mut line).map_err(|e| e.to_string())?;
            line.trim().to_string()
        }
    };
    if token.is_empty() {
        return Err("empty token".into());
    }
    // Vérifie la connexion avant de la garder.
    let client = Client::new(&url, &token);
    let status = client.json("GET", "/api/config/status", None)?;
    let path = save_settings(url.trim_end_matches('/'), &token)?;
    println!(
        "Signed in to {url} (configuration schema {}). Saved to {}.",
        status["schema_version"],
        path.display()
    );
    Ok(ExitCode::SUCCESS)
}

fn run_targets(args: &Args) -> Result<ExitCode, String> {
    if args.positional.get(1).map(String::as_str) != Some("list") {
        return Err("usage: dumbmonit-cli targets list".into());
    }
    let client = Client::connect(args)?;
    let targets = client.json("GET", "/api/targets", None)?;
    if args.switch("json") {
        println!("{}", serde_json::to_string_pretty(&targets).map_err(|e| e.to_string())?);
        return Ok(ExitCode::SUCCESS);
    }
    println!("{:>5}  {:<14} {:<28} {:<8} ADDRESS", "ID", "KIND", "NAME", "STATE");
    for target in targets.as_array().into_iter().flatten() {
        let state = if target["enabled"].as_bool() == Some(false) {
            "paused"
        } else if target["managed"]["by"] == "config-file" {
            "file"
        } else if !target["managed"].is_null() {
            "labels"
        } else {
            "active"
        };
        println!(
            "{:>5}  {:<14} {:<28} {:<8} {}",
            target["id"],
            target["kind"].as_str().unwrap_or(""),
            target["name"].as_str().unwrap_or(""),
            state,
            target["address"].as_str().unwrap_or("")
        );
    }
    Ok(ExitCode::SUCCESS)
}

/// `90s`, `15m`, `2h`, `1d`, ou un nombre de minutes.
fn parse_duration(text: &str) -> Result<u64, String> {
    let text = text.trim();
    let (digits, unit) = match text.char_indices().find(|(_, c)| !c.is_ascii_digit()) {
        Some((at, _)) => text.split_at(at),
        None => (text, "m"),
    };
    let number: u64 = digits.parse().map_err(|_| format!("`{text}` is not a duration (try 2h)"))?;
    let seconds = match unit {
        "s" => number,
        "m" => number * 60,
        "h" => number * 3600,
        "d" => number * 86_400,
        _ => return Err(format!("`{text}` is not a duration (try 90s, 15m, 2h, 1d)")),
    };
    if seconds == 0 {
        return Err("the duration must be positive".into());
    }
    Ok(seconds)
}

/// Date RFC 3339 en UTC d'un instant Unix (jours civils de H. Hinnant).
fn rfc3339(unix: u64) -> String {
    let days = (unix / 86_400) as i64;
    let rest = unix % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

fn run_silence(args: &Args) -> Result<ExitCode, String> {
    if args.positional.get(1).map(String::as_str) != Some("create") {
        return Err("usage: dumbmonit-cli silence create --name NAME --duration 2h".into());
    }
    let name = args.value("name").ok_or("silence create needs --name")?;
    let seconds = parse_duration(args.value("duration").ok_or("silence create needs --duration")?)?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_secs();

    let mut matchers = serde_json::Map::new();
    for pair in args.values.get("match").into_iter().flatten() {
        let (key, value) =
            pair.split_once('=').ok_or(format!("--match {pair}: expected key=value"))?;
        matchers.insert(key.to_string(), json!(value));
    }
    let target_id = match args.value("target-id") {
        Some(id) => Some(id.parse::<i64>().map_err(|_| format!("--target-id {id}: not a number"))?),
        None => None,
    };
    let body = json!({
        "name": name,
        "comment": args.value("comment").unwrap_or(""),
        "target_id": target_id,
        "matchers": matchers,
        "schedule": { "kind": "once", "starts_at": rfc3339(now), "ends_at": rfc3339(now + seconds) },
        "enabled": true,
    });
    let client = Client::connect(args)?;
    let silence = client.json("POST", "/api/alerts/silences", Some(&body))?;
    if args.switch("json") {
        println!("{}", serde_json::to_string_pretty(&silence).map_err(|e| e.to_string())?);
    } else {
        println!(
            "Maintenance window \"{}\" created (id {}), until {}.",
            silence["name"].as_str().unwrap_or(name),
            silence["id"],
            rfc3339(now + seconds)
        );
    }
    Ok(ExitCode::SUCCESS)
}

fn main() -> ExitCode {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let args = match parse_args(&raw) {
        Ok(args) => args,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    let command = args.positional.first().map(String::as_str);
    if args.switch("help") || command.is_none() || command == Some("help") {
        print!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let result = match command.unwrap_or_default() {
        "login" => run_login(&args),
        "export" => run_export(&args),
        "plan" => run_config("plan", &args),
        "apply" => run_config("apply", &args),
        "targets" => run_targets(&args),
        "silence" => run_silence(&args),
        other => Err(format!("unknown command `{other}` (see --help)")),
    };
    match result {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_parse() {
        assert_eq!(parse_duration("90s"), Ok(90));
        assert_eq!(parse_duration("2h"), Ok(7200));
        assert_eq!(parse_duration("15"), Ok(900));
        assert!(parse_duration("0m").is_err());
        assert!(parse_duration("soon").is_err());
    }

    #[test]
    fn dates_are_rfc3339_utc() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339(1_709_210_096), "2024-02-29T12:34:56Z");
    }

    #[test]
    fn references_resolve_from_the_environment() {
        let path = std::env::var("PATH").unwrap();
        assert_eq!(resolve_str("x-${PATH}-y", None).unwrap(), format!("x-{path}-y"));
        assert!(resolve_str("${DM_CLI_TEST_MISSING}", None).is_err());
        assert_eq!(resolve_str("{{ not a ref }}", None).unwrap(), "{{ not a ref }}");
    }

    #[test]
    fn arguments_split_into_values_switches_and_words() {
        let raw: Vec<String> = ["plan", "a.yaml", "--prune", "--url", "http://x", "--match=a=b"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let args = parse_args(&raw).unwrap();
        assert_eq!(args.positional, vec!["plan", "a.yaml"]);
        assert!(args.switch("prune"));
        assert_eq!(args.value("url"), Some("http://x"));
        assert_eq!(args.value("match"), Some("a=b"));
    }
}
