//! Moniteurs déclarés par les étiquettes `dumbmonit.*` des conteneurs Docker.
//!
//! L'agent se contente de lire et de normaliser : il ne crée rien lui-même. Le
//! serveur reçoit la liste complète à chaque cycle et réconcilie (création, mise
//! à jour, archivage) après avoir validé chaque moniteur comme une création à la
//! main. Un conteneur n'est lu que s'il porte `dumbmonit.enable=true`.
//!
//! ```yaml
//! labels:
//!   dumbmonit.enable: "true"
//!   dumbmonit.name: "Gitea"
//!   dumbmonit.folder: "Apps/Dev"
//!   dumbmonit.tags: "env=prod,team=infra"
//!   dumbmonit.interval: "30s"
//!   dumbmonit.http.url: "http://{host}:3000/api/healthz"
//!   dumbmonit.tcp.port: "2222"
//! ```

use std::collections::BTreeMap;

use dumbmonit_proto::{DeclaredMonitor, MAX_DECLARED_MONITORS};

use super::docker::ContainerStat;

const PREFIX: &str = "dumbmonit.";
const COMPOSE_PROJECT: &str = "com.docker.compose.project";
const COMPOSE_SERVICE: &str = "com.docker.compose.service";

/// Options HTTP reconnues, `dumbmonit.http.<option>` → option du moniteur.
const HTTP_OPTIONS: &[(&str, &str)] = &[
    ("method", "method"),
    ("keyword", "keyword"),
    ("keyword_absent", "keyword_absent"),
    ("accepted_status", "accepted_status"),
    ("insecure_tls", "insecure_tls"),
    ("timeout", "timeout_seconds"),
];

/// Les moniteurs déclarés par tous les conteneurs, dans un ordre stable.
pub fn declared(containers: &[ContainerStat]) -> Vec<DeclaredMonitor> {
    let mut out: Vec<DeclaredMonitor> = containers.iter().flat_map(from_container).collect();
    out.sort_by(|a, b| a.key.cmp(&b.key));
    // Deux conteneurs qui se réclament de la même clé (deux services Compose sans
    // projet, même nom) : le premier l'emporte, les autres sont ignorés.
    out.dedup_by(|b, a| a.key == b.key);
    out.truncate(MAX_DECLARED_MONITORS);
    out
}

fn truthy(value: &str) -> bool {
    matches!(value.trim().to_ascii_lowercase().as_str(), "true" | "1" | "yes" | "on")
}

fn clean(value: Option<&String>) -> Option<&str> {
    value.map(|v| v.trim()).filter(|v| !v.is_empty())
}

/// `30`, `30s`, `5m`, `1h` → secondes.
pub fn parse_interval(text: &str) -> Option<u64> {
    let text = text.trim();
    let (number, unit) = match text.char_indices().find(|(_, c)| !c.is_ascii_digit()) {
        Some((at, _)) => text.split_at(at),
        None => (text, "s"),
    };
    let value: u64 = number.parse().ok()?;
    let factor = match unit.trim() {
        "" | "s" => 1,
        "m" => 60,
        "h" => 3600,
        _ => return None,
    };
    value.checked_mul(factor)
}

/// `env=prod,team=infra` ou `critique,prod` (sans valeur : `true`).
fn parse_tags(text: &str) -> BTreeMap<String, String> {
    text.split(',')
        .filter_map(|part| {
            let part = part.trim();
            if part.is_empty() {
                return None;
            }
            let (key, value) = part.split_once('=').unwrap_or((part, "true"));
            let key: String = key
                .trim()
                .to_lowercase()
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '-' })
                .collect();
            let key = key.trim_matches('-').to_string();
            (!key.is_empty()).then(|| (key, value.trim().to_string()))
        })
        .collect()
}

fn from_container(container: &ContainerStat) -> Vec<DeclaredMonitor> {
    let labels = &container.labels;
    if !labels.get("dumbmonit.enable").is_some_and(|v| truthy(v)) {
        return Vec::new();
    }
    let get = |suffix: &str| clean(labels.get(&format!("{PREFIX}{suffix}")));

    // Un service Compose garde sa clé quand son conteneur est recréé (le nom
    // aussi, d'ordinaire, mais pas avec `container_name` absent et `--scale`).
    let base = match (clean(labels.get(COMPOSE_PROJECT)), clean(labels.get(COMPOSE_SERVICE))) {
        (Some(project), Some(service)) => format!("{project}/{service}"),
        _ => container.name.clone(),
    };
    let title = get("name").map(str::to_string).unwrap_or_else(|| container.name.clone());
    let folder = get("folder").map(str::to_string).unwrap_or_default();
    let tags = get("tags").map(parse_tags).unwrap_or_default();
    let interval_secs = get("interval").and_then(parse_interval);
    let adopt = get("adopt").is_some_and(truthy);

    let make = |suffix: &str, kind: &str, address: String, extra: BTreeMap<String, String>| {
        let mut tags = tags.clone();
        // Les options du type l'emportent sur une étiquette utilisateur de même nom.
        tags.extend(extra);
        DeclaredMonitor {
            key: format!("{base}/{suffix}"),
            kind: kind.to_string(),
            address,
            // Plusieurs moniteurs pour un conteneur : le type les distingue.
            name: title.clone(),
            folder: folder.clone(),
            tags,
            interval_secs,
            adopt,
        }
    };

    let mut out = Vec::new();
    if let Some(url) = get("http.url") {
        let mut options = BTreeMap::new();
        for (label, option) in HTTP_OPTIONS {
            if let Some(value) = get(&format!("http.{label}")) {
                options.insert((*option).to_string(), value.to_string());
            }
        }
        out.push(make("http", "http", url.to_string(), options));
    }
    if let Some(port) = get("tcp.port").and_then(|p| p.parse::<u16>().ok()).filter(|p| *p > 0) {
        let host = get("tcp.host").unwrap_or("{host}");
        out.push(make("tcp", "tcp", format!("{host}:{port}"), BTreeMap::new()));
    }
    if let Some(host) = get("ping.host") {
        out.push(make("ping", "ping", host.to_string(), BTreeMap::new()));
    }
    // Plusieurs moniteurs d'un même conteneur partagent son nom : le type est
    // ajouté pour qu'ils restent distincts dans la liste.
    if out.len() > 1 {
        for monitor in &mut out {
            monitor.name = format!("{} ({})", monitor.name, monitor.kind.to_uppercase());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn container(name: &str, labels: &[(&str, &str)]) -> ContainerStat {
        ContainerStat {
            name: name.to_string(),
            labels: labels.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            ..ContainerStat::default()
        }
    }

    #[test]
    fn sans_enable_un_conteneur_est_ignore() {
        let c = container("web", &[("dumbmonit.http.url", "http://x")]);
        assert!(declared(&[c]).is_empty());
        let c = container("web", &[("dumbmonit.enable", "false"), ("dumbmonit.http.url", "http://x")]);
        assert!(declared(&[c]).is_empty());
    }

    #[test]
    fn un_moniteur_http_complet() {
        let c = container(
            "gitea",
            &[
                ("dumbmonit.enable", "true"),
                ("dumbmonit.name", "Gitea"),
                ("dumbmonit.folder", "Apps/Dev"),
                ("dumbmonit.tags", "env=prod, Critique"),
                ("dumbmonit.interval", "5m"),
                ("dumbmonit.http.url", "http://{host}:3000/healthz"),
                ("dumbmonit.http.keyword", "ok"),
            ],
        );
        let out = declared(&[c]);
        assert_eq!(out.len(), 1);
        let m = &out[0];
        assert_eq!(m.key, "gitea/http");
        assert_eq!(m.kind, "http");
        assert_eq!(m.address, "http://{host}:3000/healthz");
        assert_eq!(m.name, "Gitea");
        assert_eq!(m.folder, "Apps/Dev");
        assert_eq!(m.interval_secs, Some(300));
        assert_eq!(m.tags["env"], "prod");
        assert_eq!(m.tags["critique"], "true");
        assert_eq!(m.tags["keyword"], "ok");
        assert!(!m.adopt);
    }

    #[test]
    fn plusieurs_types_sur_un_conteneur_et_cle_compose() {
        let c = container(
            "stack-db-1",
            &[
                ("dumbmonit.enable", "1"),
                ("dumbmonit.tcp.port", "5432"),
                ("dumbmonit.ping.host", "db.lan"),
                ("dumbmonit.adopt", "true"),
                ("com.docker.compose.project", "stack"),
                ("com.docker.compose.service", "db"),
            ],
        );
        let out = declared(&[c]);
        let keys: Vec<&str> = out.iter().map(|m| m.key.as_str()).collect();
        assert_eq!(keys, ["stack/db/ping", "stack/db/tcp"]);
        assert_eq!(out[1].address, "{host}:5432");
        assert!(out.iter().all(|m| m.adopt));
        assert_eq!(out[1].name, "stack-db-1 (TCP)");
    }

    #[test]
    fn port_invalide_ou_nul_est_ignore() {
        for port in ["0", "70000", "abc"] {
            let c = container("x", &[("dumbmonit.enable", "true"), ("dumbmonit.tcp.port", port)]);
            assert!(declared(&[c]).is_empty(), "{port}");
        }
    }

    #[test]
    fn periodes() {
        assert_eq!(parse_interval("30"), Some(30));
        assert_eq!(parse_interval("30s"), Some(30));
        assert_eq!(parse_interval("2m"), Some(120));
        assert_eq!(parse_interval("1h"), Some(3600));
        assert_eq!(parse_interval("1d"), None);
        assert_eq!(parse_interval("soon"), None);
    }
}
