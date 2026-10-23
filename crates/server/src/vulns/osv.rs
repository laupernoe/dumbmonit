//! OSV.dev : les avis de sécurité des distributions autres que Debian
//! (Ubuntu, Alpine, AlmaLinux, Rocky Linux), interrogés par lot.
//!
//! - `POST /v1/querybatch` : pour chaque (écosystème, paquet, version), les
//!   identifiants des avis qui touchent **cette** version — OSV fait lui-même
//!   la comparaison, rétroportages compris, puisque ses données viennent des
//!   avis de la distribution.
//! - `GET /v1/vulns/{id}` : le détail d'un avis (CVE liées, résumé, version
//!   qui corrige), gardé en cache tant que sa date de modification ne bouge
//!   pas.

use serde::{Deserialize, Serialize};

use super::nvd::{clip, utc};

pub const QUERY_BATCH: &str = "https://api.osv.dev/v1/querybatch";
pub const VULN: &str = "https://api.osv.dev/v1/vulns";
/// Requêtes par lot : la limite de l'API.
pub const BATCH: usize = 1000;
const MAX_SUMMARY: usize = 400;

/// Une question du lot.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct Query {
    pub ecosystem: String,
    pub package: String,
    pub version: String,
}

/// Corps de `querybatch`.
pub fn batch_body(queries: &[Query]) -> serde_json::Value {
    let list: Vec<serde_json::Value> = queries
        .iter()
        .map(|q| {
            serde_json::json!({
                "package": { "name": q.package, "ecosystem": q.ecosystem },
                "version": q.version,
            })
        })
        .collect();
    serde_json::json!({ "queries": list })
}

#[derive(Deserialize)]
struct BatchAnswer {
    #[serde(default)]
    results: Vec<BatchResult>,
}

#[derive(Deserialize)]
struct BatchResult {
    #[serde(default)]
    vulns: Vec<VulnRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct VulnRef {
    pub id: String,
    #[serde(default)]
    pub modified: String,
}

/// Réponse de `querybatch` : une liste d'avis par question, dans l'ordre.
pub fn parse_batch(body: &[u8], expected: usize) -> Result<Vec<Vec<VulnRef>>, String> {
    let answer: BatchAnswer =
        serde_json::from_slice(body).map_err(|error| format!("unexpected OSV answer: {error}"))?;
    if answer.results.len() != expected {
        return Err(format!(
            "OSV answered {} results for {expected} queries",
            answer.results.len()
        ));
    }
    Ok(answer.results.into_iter().map(|r| r.vulns).collect())
}

/// Un avis, réduit à ce qu'on affiche et à ce qui décide du verdict.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Vuln {
    pub id: String,
    pub modified: String,
    /// CVE liées (l'avis lui-même s'il en est une). Vide : l'avis n'a pas de
    /// CVE, son propre identifiant en tient lieu.
    pub cves: Vec<String>,
    pub summary: String,
    pub published: Option<String>,
    /// (écosystème, paquet, version qui corrige).
    pub fixed: Vec<(String, String, String)>,
}

impl Vuln {
    /// La version qui corrige `package` dans `ecosystem`, si l'avis la donne.
    pub fn fixed_for(&self, ecosystem: &str, package: &str) -> Option<&str> {
        self.fixed
            .iter()
            .find(|(eco, name, _)| eco == ecosystem && name == package)
            .map(|(_, _, version)| version.as_str())
    }

    /// Les identifiants à montrer : les CVE, ou l'avis lui-même.
    pub fn ids(&self) -> Vec<String> {
        if self.cves.is_empty() { vec![self.id.clone()] } else { self.cves.clone() }
    }
}

#[derive(Deserialize)]
struct RawVuln {
    id: String,
    #[serde(default)]
    modified: String,
    #[serde(default)]
    published: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    details: String,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    upstream: Vec<String>,
    #[serde(default)]
    related: Vec<String>,
    #[serde(default)]
    affected: Vec<RawAffected>,
}

#[derive(Deserialize)]
struct RawAffected {
    #[serde(default)]
    package: Option<RawPackage>,
    #[serde(default)]
    ranges: Vec<RawRange>,
}

#[derive(Deserialize)]
struct RawPackage {
    #[serde(default)]
    ecosystem: String,
    #[serde(default)]
    name: String,
}

#[derive(Deserialize)]
struct RawRange {
    #[serde(default)]
    events: Vec<serde_json::Map<String, serde_json::Value>>,
}

pub fn parse_vuln(body: &[u8]) -> Result<Vuln, String> {
    let raw: RawVuln =
        serde_json::from_slice(body).map_err(|error| format!("unexpected OSV answer: {error}"))?;
    // `related` n'est lu que pour les avis groupés des RPM (ALSA, RLSA), qui
    // n'ont ni alias ni amont.
    let related: &[String] =
        if raw.aliases.is_empty() && raw.upstream.is_empty() { &raw.related } else { &[] };
    let mut cves: Vec<String> = std::iter::once(&raw.id)
        .chain(&raw.aliases)
        .chain(&raw.upstream)
        .chain(related)
        .filter(|id| id.starts_with("CVE-"))
        .cloned()
        .collect();
    cves.sort();
    cves.dedup();
    let fixed = raw
        .affected
        .iter()
        .filter_map(|affected| {
            let package = affected.package.as_ref()?;
            let fixed = affected.ranges.iter().flat_map(|r| &r.events).find_map(|event| {
                event.get("fixed").and_then(|v| v.as_str()).map(str::to_string)
            })?;
            Some((package.ecosystem.clone(), package.name.clone(), fixed))
        })
        .collect();
    let summary = if raw.summary.is_empty() { &raw.details } else { &raw.summary };
    Ok(Vuln {
        id: raw.id,
        modified: raw.modified,
        cves,
        summary: clip(summary, MAX_SUMMARY),
        published: utc(&raw.published),
        fixed,
    })
}

/// L'écosystème OSV d'une distribution, d'après `/etc/os-release` ; `None`
/// pour une distribution qu'OSV ne couvre pas (ou Debian, lue au tracker).
pub fn ecosystem(distro_id: &str, version_id: &str) -> Option<String> {
    let major = version_id.split('.').next().unwrap_or_default();
    match distro_id {
        "ubuntu" => {
            // Les versions LTS sont les « .04 » des années paires.
            let lts =
                version_id.ends_with(".04") && major.parse::<u32>().is_ok_and(|year| year % 2 == 0);
            Some(if lts {
                format!("Ubuntu:{version_id}:LTS")
            } else {
                format!("Ubuntu:{version_id}")
            })
        }
        "alpine" => {
            let mut parts = version_id.split('.');
            let (major, minor) = (parts.next()?, parts.next()?);
            Some(format!("Alpine:v{major}.{minor}"))
        }
        "almalinux" if !major.is_empty() => Some(format!("AlmaLinux:{major}")),
        "rocky" if !major.is_empty() => Some(format!("Rocky Linux:{major}")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_lot_rend_une_liste_par_question() {
        let body = br#"{"results":[{"vulns":[{"id":"UBUNTU-CVE-2024-6387","modified":"2024-07-02T00:00:00Z"}]},{}]}"#;
        let results = parse_batch(body, 2).unwrap();
        assert_eq!(results[0][0].id, "UBUNTU-CVE-2024-6387");
        assert!(results[1].is_empty());
        assert!(parse_batch(body, 3).is_err(), "une réponse incomplète est refusée");
        let queries = vec![Query {
            ecosystem: "Ubuntu:24.04:LTS".into(),
            package: "openssh".into(),
            version: "1:9.6p1-3ubuntu13".into(),
        }];
        let json = batch_body(&queries).to_string();
        assert!(json.contains(r#""ecosystem":"Ubuntu:24.04:LTS""#));
    }

    #[test]
    fn un_avis_donne_ses_cve_et_la_version_qui_corrige() {
        let body = br#"{"id":"UBUNTU-CVE-2024-6387","modified":"2024-07-02T00:00:00Z","published":"2024-07-01T08:15:00Z",
          "details":"A signal handler race condition was found in sshd.","upstream":["CVE-2024-6387"],
          "affected":[{"package":{"ecosystem":"Ubuntu:24.04:LTS","name":"openssh"},
            "ranges":[{"type":"ECOSYSTEM","events":[{"introduced":"0"},{"fixed":"1:9.6p1-3ubuntu13.3"}]}]},
            {"package":{"ecosystem":"Ubuntu:22.04:LTS","name":"openssh"},"ranges":[{"type":"ECOSYSTEM","events":[{"introduced":"0"}]}]}]}"#;
        let vuln = parse_vuln(body).unwrap();
        assert_eq!(vuln.cves, ["CVE-2024-6387"]);
        assert_eq!(vuln.published.as_deref(), Some("2024-07-01 08:15:00"));
        assert_eq!(vuln.fixed_for("Ubuntu:24.04:LTS", "openssh"), Some("1:9.6p1-3ubuntu13.3"));
        assert_eq!(vuln.fixed_for("Ubuntu:22.04:LTS", "openssh"), None, "pas encore corrigé");
        assert!(vuln.summary.starts_with("A signal handler"));

        let alsa = br#"{"id":"ALSA-2024:4312","related":["CVE-2024-6387","CVE-2024-6409"],"summary":"openssh security update","affected":[]}"#;
        let alsa = parse_vuln(alsa).unwrap();
        assert_eq!(alsa.ids(), ["CVE-2024-6387", "CVE-2024-6409"]);
        let ghsa = parse_vuln(br#"{"id":"ALPINE-2024-1","affected":[]}"#).unwrap();
        assert_eq!(ghsa.ids(), ["ALPINE-2024-1"]);
    }

    #[test]
    fn l_ecosysteme_suit_la_distribution() {
        assert_eq!(ecosystem("ubuntu", "24.04").as_deref(), Some("Ubuntu:24.04:LTS"));
        assert_eq!(ecosystem("ubuntu", "23.10").as_deref(), Some("Ubuntu:23.10"));
        assert_eq!(ecosystem("ubuntu", "25.04").as_deref(), Some("Ubuntu:25.04"));
        assert_eq!(ecosystem("alpine", "3.20.3").as_deref(), Some("Alpine:v3.20"));
        assert_eq!(ecosystem("almalinux", "9.4").as_deref(), Some("AlmaLinux:9"));
        assert_eq!(ecosystem("rocky", "9.4").as_deref(), Some("Rocky Linux:9"));
        assert_eq!(ecosystem("debian", "12"), None);
        assert_eq!(ecosystem("arch", ""), None);
    }
}
