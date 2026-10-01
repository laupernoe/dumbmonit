//! Vérification d'un paquet et compilation de ses chemins et expressions.
//!
//! Tout ce qui peut être refusé sans réseau l'est ici, une fois, à
//! l'installation : un paquet accepté ne peut plus échouer que pour une raison
//! qui tient à l'équipement.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::LazyLock;

use dumbmonit_proto::MetricKind;
use regex::Regex;
use serde_json_path::JsonPath;
use sha2::{Digest, Sha256};

use crate::manifest::{
    Auth, Format, Kind, Manifest, Method, MetricSpec, RuleSpec, SCHEMA, Scalar, SourceSpec,
    SourceType,
};
use crate::template::{CredentialField, Template};
use crate::{
    KIND_PREFIX, MAX_METRICS, MAX_REQUESTS, MAX_ROWS, MAX_RULES, MAX_YAML_BYTES, METRIC_PREFIX,
    Pack, PackError, SnmpProfile, prefix_for,
};

/// Options de connexion que tout paquet HTTP reçoit ; un paquet ne peut pas
/// déclarer les siennes sous ces noms.
pub(crate) const CONNECTION_OPTIONS: &[&str] =
    &["scheme", "port", "insecure_tls", "request_timeout_seconds", "allow_private_targets"];

/// Étiquettes posées par le serveur ou lues par l'alerting pour retrouver la
/// cible : une étiquette de paquet ne peut pas les imiter.
pub(crate) const RESERVED_LABELS: &[&str] =
    &["target", "target_id", "host", "hostname", "instance", "address", "job"];

/// En-têtes qu'une source ne peut pas fixer : ils touchent au transport, pas au
/// service interrogé.
const FORBIDDEN_HEADERS: &[&str] =
    &["host", "content-length", "transfer-encoding", "connection", "upgrade", "te"];

const CREDENTIAL_KINDS: &[&str] = &["none", "api_token", "username_password"];

static ID: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z][a-z0-9]*(-[a-z0-9]+)*$").expect("expression valide"));
static NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z][a-z0-9_]*$").expect("expression valide"));
static LABEL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z_][a-z0-9_]*$").expect("expression valide"));
static REFERENCE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bdumbmonit_[A-Za-z0-9_]+").expect("expression valide"));

/// Une source compilée.
#[derive(Debug)]
pub(crate) struct Source {
    pub id: String,
    /// Préfixe des noms d'échantillon du paquet (`shelly_plug_`).
    pub prefix: String,
    pub format: BodyFormat,
    pub method: Method,
    pub path: Template,
    pub headers: Vec<(String, Template)>,
    pub body: Option<Template>,
    pub auth: Auth,
    /// Familles Prometheus retenues : nom lu → nom écrit (sans préfixe).
    pub keep: Vec<(String, String)>,
    pub drop: Vec<(String, Regex)>,
    pub metrics: Vec<Metric>,
    pub discovers: Vec<Discover>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BodyFormat {
    Json,
    Text,
    Prometheus,
}

#[derive(Debug)]
pub(crate) struct Metric {
    /// Nom d'échantillon complet, préfixe du paquet compris.
    pub name: String,
    pub kind: MetricKind,
    pub extractor: Extractor,
    pub scale: Option<f64>,
    pub map: BTreeMap<String, f64>,
    pub labels: Vec<(String, LabelFrom)>,
}

#[derive(Debug)]
pub(crate) enum Extractor {
    Json(JsonPath),
    Regex(Regex),
    Prom(String),
}

#[derive(Debug)]
pub(crate) enum LabelFrom {
    /// JSONPath évalué sur l'objet qui contient la valeur.
    Json(JsonPath),
    /// Groupe nommé de l'expression régulière.
    Group(String),
    /// Étiquette de l'échantillon Prometheus.
    Prom(String),
}

#[derive(Debug)]
pub(crate) struct Discover {
    pub name: String,
    pub rows: JsonPath,
    pub max_rows: usize,
    pub labels: Vec<(String, JsonPath)>,
    pub metrics: Vec<RowMetric>,
}

#[derive(Debug)]
pub(crate) struct RowMetric {
    pub name: String,
    pub kind: MetricKind,
    pub path: JsonPath,
    pub scale: Option<f64>,
    pub map: BTreeMap<String, f64>,
}

/// Une règle d'alerte du paquet, prête à être installée.
#[derive(Debug, Clone, PartialEq)]
pub struct PackRule {
    /// `pack:<paquet>:<nom>`.
    pub uid: String,
    pub name: String,
    pub title: String,
    pub description: String,
    pub expr: String,
    /// `>`, `>=`, `<` ou `<=`.
    pub op: String,
    /// Seuil installé : le nombre écrit, ou la valeur par défaut de l'option citée.
    pub threshold: f64,
    /// L'option dont vient le seuil, s'il en vient une.
    pub threshold_option: Option<String>,
    pub for_secs: u64,
    /// `info`, `warning` ou `critical`.
    pub severity: String,
    pub unit: String,
}

/// Accumulateur d'erreurs, chacune précédée de l'endroit où elle se trouve.
#[derive(Default)]
struct Problems {
    errors: Vec<String>,
    warnings: Vec<String>,
}

impl Problems {
    fn error(&mut self, at: &str, message: impl std::fmt::Display) {
        self.errors.push(format!("{at}: {message}"));
    }

    fn warn(&mut self, at: &str, message: impl std::fmt::Display) {
        self.warnings.push(format!("{at}: {message}"));
    }
}

pub(crate) fn compile(yaml: &str) -> Result<Pack, PackError> {
    if yaml.len() > MAX_YAML_BYTES {
        return Err(PackError {
            errors: vec![format!(
                "the pack is {} KiB, more than the {} KiB allowed",
                yaml.len() / 1024,
                MAX_YAML_BYTES / 1024
            )],
        });
    }
    let manifest: Manifest = serde_yaml_ng::from_str(yaml)
        .map_err(|error| PackError { errors: vec![format!("invalid YAML: {error}")] })?;
    let mut problems = Problems::default();

    header(&manifest, &mut problems);
    let prefix = prefix_for(&manifest.id);
    let option_defaults = options(&manifest, &mut problems);
    credentials(&manifest, &mut problems);

    let mut produced = BTreeSet::new();
    // Séries d'histogramme ou de résumé des familles Prometheus retenues : des
    // noms qu'une règle peut citer, mais pas des métriques déclarées.
    let mut variants = BTreeSet::new();
    let mut sources = sources(&manifest, &prefix, &option_defaults, &mut problems);
    metrics(&manifest, &prefix, &mut sources, &mut produced, &mut variants, &mut problems);
    let snmp_profiles = snmp_profiles(&manifest, &prefix, &mut produced, &mut problems);

    if sources.is_empty() && snmp_profiles.is_empty() {
        problems.error("pack", "declares neither sources nor snmp_profiles: it collects nothing");
    }
    for source in &sources {
        if source.metrics.is_empty() && source.discovers.is_empty() && source.keep.is_empty() {
            problems.warn(&format!("sources[{}]", source.id), "no metric reads this source");
        }
    }

    let citable: BTreeSet<String> = produced.union(&variants).cloned().collect();
    let rules = rules(&manifest, &option_defaults, &citable, &mut problems);

    if !problems.errors.is_empty() {
        return Err(PackError { errors: problems.errors });
    }
    let kind = format!("{KIND_PREFIX}{}", manifest.id);
    Ok(Pack {
        sha256: sha256_hex(yaml),
        yaml: yaml.to_string(),
        kind,
        prefix,
        sources,
        rules,
        snmp_profiles,
        produced,
        warnings: problems.warnings,
        manifest,
    })
}

fn sha256_hex(text: &str) -> String {
    Sha256::digest(text.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn header(manifest: &Manifest, problems: &mut Problems) {
    if manifest.schema != SCHEMA {
        problems.error(
            "schema",
            format!(
                "version {} is not supported by this server (expected {SCHEMA})",
                manifest.schema
            ),
        );
    }
    if manifest.id.len() > 40 || !ID.is_match(&manifest.id) {
        problems.error(
            "id",
            format!(
                "\"{}\" must be 1 to 40 lowercase letters, digits and single dashes, \
                 starting with a letter",
                manifest.id
            ),
        );
    }
    if semver::Version::parse(&manifest.version).is_err() {
        problems.error(
            "version",
            format!("\"{}\" is not a semantic version (e.g. 1.0.0)", manifest.version),
        );
    }
    if let Some(requires) = &manifest.requires {
        match semver::VersionReq::parse(requires) {
            Err(error) => problems.error("requires", format!("\"{requires}\": {error}")),
            Ok(requirement) => {
                let mut current = semver::Version::parse(env!("CARGO_PKG_VERSION"))
                    .expect("version du paquet Cargo valide");
                // Une préversion (0.2.0-alpha.3) répond comme la version qu'elle
                // annonce : sinon `>=0.2` refuserait toute la série alpha.
                current.pre = semver::Prerelease::EMPTY;
                if !requirement.matches(&current) {
                    problems.error(
                        "requires",
                        format!(
                            "this pack needs DumbMonit {requires}, this server is {}",
                            env!("CARGO_PKG_VERSION")
                        ),
                    );
                }
            }
        }
    }
    if manifest.label.trim().is_empty() {
        problems.error("label", "must not be empty");
    }
    let url = &manifest.setup.doc_url;
    if !url.is_empty() && !url.starts_with("https://") && !url.starts_with("http://") {
        problems.error("setup.doc_url", "must be an http(s) URL");
    }
}

/// Vérifie les options et rend leurs valeurs par défaut.
fn options(manifest: &Manifest, problems: &mut Problems) -> BTreeMap<String, String> {
    let mut defaults = BTreeMap::new();
    for option in &manifest.options {
        let at = format!("options[{}]", option.key);
        if !NAME.is_match(&option.key) {
            problems.error(&at, "the key must be lowercase letters, digits and underscores");
        }
        if CONNECTION_OPTIONS.contains(&option.key.as_str()) {
            problems.error(&at, "this key is reserved for the connection settings every pack gets");
        }
        if defaults.insert(option.key.clone(), option.default.0.clone()).is_some() {
            problems.error(&at, "declared twice");
        }
        let default = option.default.0.as_str();
        match option.input.as_str() {
            "text" => {}
            "number" => {
                if !default.is_empty() && default.parse::<f64>().is_err() {
                    problems.error(&at, format!("default \"{default}\" is not a number"));
                }
            }
            "boolean" => {
                if !matches!(default, "" | "true" | "false") {
                    problems.error(&at, "the default of a boolean is true or false");
                }
            }
            "select" => {
                if option.choices.is_empty() {
                    problems.error(&at, "a select needs choices");
                } else if !default.is_empty() && !option.choices.iter().any(|c| c == default) {
                    problems.error(&at, format!("default \"{default}\" is not one of the choices"));
                }
            }
            other => problems
                .error(&at, format!("unknown input \"{other}\": text, number, boolean or select")),
        }
    }
    defaults
}

fn credentials(manifest: &Manifest, problems: &mut Problems) {
    let mut seen = HashSet::new();
    for credential in &manifest.credentials {
        let kind = credential.kind();
        if !CREDENTIAL_KINDS.contains(&kind) {
            problems.error(
                "credentials",
                format!("unknown kind \"{kind}\": none, api_token or username_password"),
            );
        }
        if !seen.insert(kind) {
            problems.error("credentials", format!("\"{kind}\" listed twice"));
        }
    }
}

fn accepts(manifest: &Manifest, kind: &str) -> bool {
    if manifest.credentials.is_empty() {
        return kind == "none";
    }
    manifest.credentials.iter().any(|credential| credential.kind() == kind)
}

fn sources(
    manifest: &Manifest,
    prefix: &str,
    options: &BTreeMap<String, String>,
    problems: &mut Problems,
) -> Vec<Source> {
    if manifest.sources.len() > MAX_REQUESTS {
        problems.error(
            "sources",
            format!(
                "{} sources, at most {MAX_REQUESTS} requests are allowed per probe",
                manifest.sources.len()
            ),
        );
    }
    let mut ids = HashSet::new();
    let mut compiled = Vec::new();
    for spec in &manifest.sources {
        let at = format!("sources[{}]", spec.id);
        if !NAME.is_match(&spec.id) {
            problems.error(&at, "the id must be lowercase letters, digits and underscores");
        }
        if !ids.insert(spec.id.as_str()) {
            problems.error(&at, "declared twice");
        }
        if let Some(source) = source(manifest, spec, prefix, options, &at, problems) {
            compiled.push(source);
        }
    }
    compiled
}

fn source(
    manifest: &Manifest,
    spec: &SourceSpec,
    prefix: &str,
    options: &BTreeMap<String, String>,
    at: &str,
    problems: &mut Problems,
) -> Option<Source> {
    let format = match (spec.kind, spec.format) {
        (SourceType::Http, None | Some(Format::Json)) => BodyFormat::Json,
        (SourceType::Http, Some(Format::Text)) => BodyFormat::Text,
        (SourceType::Prometheus, None) => BodyFormat::Prometheus,
        (SourceType::Prometheus, Some(_)) => {
            problems
                .error(at, "a prometheus source has no format: it is always the text exposition");
            BodyFormat::Prometheus
        }
    };
    if spec.kind == SourceType::Http
        && (!spec.keep.is_empty() || !spec.rename.is_empty() || !spec.drop.is_empty())
    {
        problems.error(at, "keep, rename and drop only apply to prometheus sources");
    }
    if spec.kind == SourceType::Prometheus && spec.method != Method::Get {
        problems.error(at, "a prometheus source is read with GET");
    }
    if spec.body.is_some() && spec.method == Method::Get {
        problems.error(at, "a body needs method: POST");
    }

    let path = template(&format!("{at}.path"), &spec.path, options, problems)?;
    if path.uses_credential() {
        problems.error(
            &format!("{at}.path"),
            "credentials may only appear in headers: a URL ends up in logs",
        );
    }
    check_path(&format!("{at}.path"), &path.literal(), problems);

    let mut headers = Vec::new();
    for (name, value) in &spec.headers {
        let where_ = format!("{at}.headers[{name}]");
        let lower = name.to_ascii_lowercase();
        if name.is_empty()
            || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
        {
            problems.error(&where_, "invalid header name");
        }
        if FORBIDDEN_HEADERS.contains(&lower.as_str()) {
            problems.error(&where_, "this header is managed by DumbMonit");
        }
        if lower == "authorization" && spec.auth != Auth::None {
            problems.error(&where_, "set auth: none to write the Authorization header yourself");
        }
        if let Some(value) = template(&where_, value, options, problems) {
            credential_declared(manifest, &value, &where_, problems);
            headers.push((name.clone(), value));
        }
    }

    let body = match &spec.body {
        None => None,
        Some(raw) => {
            let body = template(&format!("{at}.body"), raw, options, problems)?;
            if body.uses_credential() {
                problems.error(&format!("{at}.body"), "credentials may only appear in headers");
            }
            Some(body)
        }
    };

    match spec.auth {
        Auth::Bearer if !accepts(manifest, "api_token") => {
            problems.error(at, "auth: bearer needs api_token in credentials");
        }
        Auth::Basic if !accepts(manifest, "username_password") => {
            problems.error(at, "auth: basic needs username_password in credentials");
        }
        _ => {}
    }

    let mut keep = Vec::new();
    for family in &spec.keep {
        let written = spec.rename.get(family).unwrap_or(family);
        if !NAME.is_match(written) {
            problems.error(
                &format!("{at}.keep[{family}]"),
                format!(
                    "\"{written}\" is not a valid metric name: rename it (lowercase, digits, _)"
                ),
            );
        }
        keep.push((family.clone(), written.clone()));
    }
    let mut drop = Vec::new();
    for (label, pattern) in &spec.drop {
        match Regex::new(pattern) {
            Ok(regex) => drop.push((label.clone(), regex)),
            Err(error) => problems.error(&format!("{at}.drop[{label}]"), error),
        }
    }

    Some(Source {
        id: spec.id.clone(),
        prefix: prefix.to_string(),
        format,
        method: spec.method,
        path,
        headers,
        body,
        auth: spec.auth,
        keep,
        drop,
        metrics: Vec::new(),
        discovers: Vec::new(),
    })
}

fn template(
    at: &str,
    raw: &str,
    options: &BTreeMap<String, String>,
    problems: &mut Problems,
) -> Option<Template> {
    match Template::parse(raw) {
        Ok(template) => {
            for key in template.options() {
                if !options.contains_key(key) && !CONNECTION_OPTIONS.contains(&key) {
                    problems.error(at, format!("{{{{option.{key}}}}} is not a declared option"));
                }
            }
            Some(template)
        }
        Err(error) => {
            problems.error(at, error);
            None
        }
    }
}

fn credential_declared(
    manifest: &Manifest,
    template: &Template,
    at: &str,
    problems: &mut Problems,
) {
    let needs_token = template.credentials().any(|field| field == CredentialField::Token);
    let needs_login = template.credentials().any(|field| field != CredentialField::Token);
    if needs_token && !accepts(manifest, "api_token") {
        problems.error(at, "{{credential.token}} needs api_token in credentials");
    }
    if needs_login && !accepts(manifest, "username_password") {
        problems
            .error(at, "{{credential.username/password}} needs username_password in credentials");
    }
}

/// Un chemin relatif à la racine de la cible : il ne peut désigner ni un autre
/// hôte, ni un autre protocole.
fn check_path(at: &str, literal: &str, problems: &mut Problems) {
    if !literal.starts_with('/') || literal.starts_with("//") {
        problems.error(
            at,
            "must start with a single \"/\": requests always go to the target's own address",
        );
    }
    if literal.contains("://") || literal.contains('\\') || literal.contains('#') {
        problems.error(at, "must be a path on the target, not a URL");
    }
    if literal.chars().any(|c| c.is_whitespace() || c.is_control()) {
        problems.error(at, "must not contain spaces or control characters");
    }
}

fn metric_kind(kind: Kind) -> MetricKind {
    match kind {
        Kind::Gauge => MetricKind::Gauge,
        Kind::Counter => MetricKind::Counter,
    }
}

fn map(raw: &BTreeMap<Scalar, f64>) -> BTreeMap<String, f64> {
    raw.iter().map(|(key, value)| (key.0.clone(), *value)).collect()
}

fn json_path(at: &str, raw: &str, problems: &mut Problems) -> Option<JsonPath> {
    match JsonPath::parse(raw) {
        Ok(path) => Some(path),
        Err(error) => {
            problems.error(at, format!("invalid JSONPath \"{raw}\": {error}"));
            None
        }
    }
}

/// Nom écrit d'une métrique, préfixe compris ; enregistré dans `produced`.
fn claim(
    at: &str,
    name: &str,
    prefix: &str,
    produced: &mut BTreeSet<String>,
    problems: &mut Problems,
) -> String {
    if !NAME.is_match(name) || name.len() > 100 {
        problems.error(at, "the name must be lowercase letters, digits and underscores");
    }
    if name.starts_with(prefix) {
        problems.warn(
            at,
            format!(
                "\"{name}\" already starts with \"{prefix}\": names are prefixed automatically"
            ),
        );
    }
    let full = format!("{prefix}{name}");
    if !produced.insert(format!("{METRIC_PREFIX}{full}")) {
        problems.error(at, format!("{METRIC_PREFIX}{full} is produced twice"));
    }
    full
}

fn metrics(
    manifest: &Manifest,
    prefix: &str,
    sources: &mut [Source],
    produced: &mut BTreeSet<String>,
    variants: &mut BTreeSet<String>,
    problems: &mut Problems,
) {
    let declared =
        manifest.metrics.len() + manifest.discover.iter().map(|d| d.metrics.len()).sum::<usize>();
    if declared > MAX_METRICS {
        problems.error("metrics", format!("{declared} metrics, at most {MAX_METRICS}"));
    }

    // Familles Prometheus retenues en bloc : elles produisent aussi leurs
    // suffixes d'histogramme ou de résumé.
    for source in sources.iter() {
        for (family, written) in &source.keep {
            let at = format!("sources[{}].keep[{family}]", source.id);
            claim(&at, written, prefix, produced, problems);
            for suffix in crate::prom::FAMILY_SUFFIXES {
                variants.insert(format!("{METRIC_PREFIX}{prefix}{written}{suffix}"));
            }
        }
    }

    for spec in &manifest.metrics {
        let at = format!("metrics[{}]", spec.name);
        let name = claim(&at, &spec.name, prefix, produced, problems);
        let Some(source) = sources.iter_mut().find(|s| s.id == spec.source) else {
            problems.error(&at, format!("unknown source \"{}\"", spec.source));
            continue;
        };
        if let Some(metric) = metric(spec, name, source.format, &at, problems) {
            source.metrics.push(metric);
        }
    }

    for spec in &manifest.discover {
        let at = format!("discover[{}]", spec.name);
        let Some(source) = sources.iter_mut().find(|s| s.id == spec.source) else {
            problems.error(&at, format!("unknown source \"{}\"", spec.source));
            continue;
        };
        if source.format != BodyFormat::Json {
            problems.error(
                &at,
                "discover reads JSON: its source must be an http source in json format",
            );
        }
        if spec.max_rows == 0 || spec.max_rows > MAX_ROWS {
            problems.error(&at, format!("max_rows must be between 1 and {MAX_ROWS}"));
        }
        if spec.metrics.is_empty() {
            problems.error(&at, "declares no metric");
        }
        let rows = json_path(&format!("{at}.rows"), &spec.rows, problems);
        let mut labels = Vec::new();
        for (label, raw) in &spec.labels {
            let where_ = format!("{at}.labels[{label}]");
            check_label(&where_, label, problems);
            if let Some(path) = json_path(&where_, raw, problems) {
                labels.push((label.clone(), path));
            }
        }
        let mut row_metrics = Vec::new();
        for metric in &spec.metrics {
            let where_ = format!("{at}.metrics[{}]", metric.name);
            let name = claim(&where_, &metric.name, prefix, produced, problems);
            check_scale(&where_, metric.scale, problems);
            if let Some(path) = json_path(&where_, &metric.json, problems) {
                row_metrics.push(RowMetric {
                    name,
                    kind: metric_kind(metric.kind),
                    path,
                    scale: metric.scale,
                    map: map(&metric.map),
                });
            }
        }
        if let Some(rows) = rows {
            source.discovers.push(Discover {
                name: spec.name.clone(),
                rows,
                max_rows: spec.max_rows.clamp(1, MAX_ROWS),
                labels,
                metrics: row_metrics,
            });
        }
    }
}

fn check_label(at: &str, label: &str, problems: &mut Problems) {
    if !LABEL.is_match(label) || label.starts_with("__") {
        problems.error(at, "a label name is lowercase letters, digits and underscores");
    }
    if RESERVED_LABELS.contains(&label) || label.starts_with("tag_") {
        problems.error(at, format!("\"{label}\" is reserved: DumbMonit sets it itself"));
    }
}

fn check_scale(at: &str, scale: Option<f64>, problems: &mut Problems) {
    if scale.is_some_and(|scale| !scale.is_finite() || scale == 0.0) {
        problems.error(at, "scale must be a finite, non-zero number");
    }
}

fn metric(
    spec: &MetricSpec,
    name: String,
    format: BodyFormat,
    at: &str,
    problems: &mut Problems,
) -> Option<Metric> {
    check_scale(at, spec.scale, problems);
    let chosen = [spec.json.is_some(), spec.regex.is_some(), spec.prom.is_some()];
    if chosen.iter().filter(|set| **set).count() != 1 {
        problems.error(at, "set exactly one of json, regex or prom");
        return None;
    }
    let mut labels = Vec::new();
    let extractor = if let Some(raw) = &spec.json {
        if format != BodyFormat::Json {
            problems.error(at, "json needs an http source in json format");
        }
        for (label, from) in &spec.labels {
            let where_ = format!("{at}.labels[{label}]");
            check_label(&where_, label, problems);
            if let Some(path) = json_path(&where_, from, problems) {
                labels.push((label.clone(), LabelFrom::Json(path)));
            }
        }
        Extractor::Json(json_path(at, raw, problems)?)
    } else if let Some(raw) = &spec.regex {
        if format != BodyFormat::Text {
            problems.error(at, "regex needs an http source in text format");
        }
        let regex = match Regex::new(raw) {
            Ok(regex) => regex,
            Err(error) => {
                problems.error(at, error);
                return None;
            }
        };
        if regex.captures_len() < 2 {
            problems.error(at, "the expression needs a capture group for the value");
        }
        let groups: HashSet<&str> = regex.capture_names().flatten().collect();
        for (label, group) in &spec.labels {
            let where_ = format!("{at}.labels[{label}]");
            check_label(&where_, label, problems);
            if !groups.contains(group.as_str()) {
                problems.error(&where_, format!("no named group \"{group}\" in the expression"));
            }
            labels.push((label.clone(), LabelFrom::Group(group.clone())));
        }
        Extractor::Regex(regex)
    } else {
        if format != BodyFormat::Prometheus {
            problems.error(at, "prom needs a prometheus source");
        }
        for (label, from) in &spec.labels {
            let where_ = format!("{at}.labels[{label}]");
            check_label(&where_, label, problems);
            labels.push((label.clone(), LabelFrom::Prom(from.clone())));
        }
        Extractor::Prom(spec.prom.clone().unwrap_or_default())
    };
    Some(Metric {
        name,
        kind: metric_kind(spec.kind),
        extractor,
        scale: spec.scale,
        map: map(&spec.map),
        labels,
    })
}

fn snmp_profiles(
    manifest: &Manifest,
    prefix: &str,
    produced: &mut BTreeSet<String>,
    problems: &mut Problems,
) -> Vec<SnmpProfile> {
    let mut profiles = Vec::new();
    for (index, value) in manifest.snmp_profiles.iter().enumerate() {
        let at = format!("snmp_profiles[{index}]");
        let source = match serde_yaml_ng::to_string(value) {
            Ok(source) => source,
            Err(error) => {
                problems.error(&at, error);
                continue;
            }
        };
        let profile = match dumbmonit_collectors::snmp::Profile::parse(&source) {
            Ok(profile) => profile,
            Err(error) => {
                problems.error(&at, error);
                continue;
            }
        };
        let at = format!("snmp_profiles[{}]", profile.id);
        if profile.id != manifest.id && !profile.id.starts_with(&format!("{}-", manifest.id)) {
            problems.error(
                &at,
                format!("a profile id must be \"{0}\" or start with \"{0}-\"", manifest.id),
            );
        }
        for metric in &profile.metrics {
            if !metric.name.starts_with(prefix) {
                problems.error(
                    &format!("{at}.metrics[{}]", metric.name),
                    format!("SNMP metric names must start with \"{prefix}\""),
                );
            }
            produced.insert(format!("{METRIC_PREFIX}{}", metric.name));
        }
        profiles.push(SnmpProfile { id: profile.id.clone(), source });
    }

    // Avec les profils livrés : un identifiant déjà pris, ou un `include` qui ne
    // mène nulle part, ferait échouer le chargement au démarrage.
    let mut catalog = dumbmonit_collectors::SnmpCollector::new().catalog().clone();
    for profile in &profiles {
        if let Err(error) = catalog.add_source(&profile.id, &profile.source) {
            problems.error(&format!("snmp_profiles[{}]", profile.id), error);
        }
    }
    for profile in &profiles {
        if catalog.get(&profile.id).is_some()
            && let Err(error) = catalog.resolve(&profile.id)
        {
            problems.error(&format!("snmp_profiles[{}]", profile.id), error);
        }
    }
    profiles
}

fn rules(
    manifest: &Manifest,
    options: &BTreeMap<String, String>,
    produced: &BTreeSet<String>,
    problems: &mut Problems,
) -> Vec<PackRule> {
    if manifest.rules.len() > MAX_RULES {
        problems.error("rules", format!("{} rules, at most {MAX_RULES}", manifest.rules.len()));
    }
    let mut names = HashSet::new();
    let mut rules = Vec::new();
    for spec in &manifest.rules {
        let at = format!("rules[{}]", spec.name);
        if !NAME.is_match(&spec.name) {
            problems.error(&at, "the name must be lowercase letters, digits and underscores");
        }
        if !names.insert(spec.name.as_str()) {
            problems.error(&at, "declared twice");
        }
        if let Some(rule) = rule(manifest, spec, options, produced, &at, problems) {
            rules.push(rule);
        }
    }
    rules
}

fn rule(
    manifest: &Manifest,
    spec: &RuleSpec,
    options: &BTreeMap<String, String>,
    produced: &BTreeSet<String>,
    at: &str,
    problems: &mut Problems,
) -> Option<PackRule> {
    if !matches!(spec.op.as_str(), ">" | ">=" | "<" | "<=") {
        problems.error(at, format!("op \"{}\" must be >, >=, < or <=", spec.op));
    }
    if !matches!(spec.severity.as_str(), "info" | "warning" | "critical") {
        problems
            .error(at, format!("severity \"{}\" must be info, warning or critical", spec.severity));
    }
    let for_secs = match spec.for_duration.as_deref() {
        None => 0,
        Some(raw) => match parse_duration(raw) {
            Some(secs) => secs,
            None => {
                problems.error(at, format!("for: \"{raw}\" is not a duration like 30s, 5m or 1h"));
                0
            }
        },
    };

    let mut own = 0;
    for found in REFERENCE.find_iter(&spec.expr) {
        let name = found.as_str();
        if produced.contains(name) {
            own += 1;
        } else if name != "dumbmonit_up" {
            problems.error(at, format!("{name} is not produced by this pack"));
        }
    }
    if own == 0 {
        problems.error(
            at,
            format!(
                "the expression must use at least one metric of this pack, by its full name \
                 (e.g. {})",
                produced.iter().next().map(String::as_str).unwrap_or("dumbmonit_<pack>_<metric>")
            ),
        );
    }

    let (threshold, threshold_option) = match spec.threshold.0.trim().parse::<f64>() {
        Ok(value) if value.is_finite() => (value, None),
        Ok(_) => {
            problems.error(at, "the threshold must be finite");
            return None;
        }
        Err(_) => {
            let key = spec
                .threshold
                .0
                .trim()
                .strip_prefix("{{")
                .and_then(|rest| rest.strip_suffix("}}"))
                .map(str::trim)
                .and_then(|inner| inner.strip_prefix("option."));
            let Some(key) = key else {
                problems.error(at, "the threshold is a number or {{option.<key>}}");
                return None;
            };
            let Some(default) = options.get(key) else {
                problems.error(at, format!("{{{{option.{key}}}}} is not a declared option"));
                return None;
            };
            match default.parse::<f64>() {
                Ok(value) => (value, Some(key.to_string())),
                Err(_) => {
                    problems.error(
                        at,
                        format!("the option \"{key}\" used as threshold needs a numeric default"),
                    );
                    return None;
                }
            }
        }
    };

    Some(PackRule {
        uid: format!("pack:{}:{}", manifest.id, spec.name),
        name: spec.name.clone(),
        title: if spec.title.trim().is_empty() { spec.name.clone() } else { spec.title.clone() },
        description: spec.description.clone(),
        expr: spec.expr.clone(),
        op: spec.op.clone(),
        threshold,
        threshold_option,
        for_secs,
        severity: spec.severity.clone(),
        unit: spec.unit.clone(),
    })
}

/// `30s`, `5m`, `1h`, `1d` ou un nombre de secondes.
fn parse_duration(raw: &str) -> Option<u64> {
    let raw = raw.trim();
    if let Ok(secs) = raw.parse() {
        return Some(secs);
    }
    let (number, unit) = raw.split_at(raw.len().checked_sub(1)?);
    let number: u64 = number.parse().ok()?;
    let factor = match unit {
        "s" => 1,
        "m" => 60,
        "h" => 3_600,
        "d" => 86_400,
        _ => return None,
    };
    number.checked_mul(factor)
}
