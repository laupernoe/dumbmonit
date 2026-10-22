//! Debian Security Tracker : pour chaque paquet source, chaque CVE et chaque
//! version de Debian, l'état (`resolved`, `open`, `undetermined`) et la
//! version qui corrige — celle du rétroportage dans la version stable, pas
//! celle de l'amont. C'est ce qui permet de dire « corrigé » d'un
//! `openssl 3.0.15-1~deb12u1` que la plage amont de la NVD dirait vulnérable.
//!
//! Le fichier complet (`/tracker/data/json`) pèse environ 85 Mo (14 Mo
//! compressé) : il est écrit sur disque puis lu en flux, sans jamais le tenir
//! en mémoire, et seuls les paquets et versions de Debian présents dans
//! l'inventaire sont gardés.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io::Read;

use serde::Deserialize;
use serde::de::{DeserializeSeed, IgnoredAny, MapAccess, Visitor};

use super::nvd::clip;

pub const URL: &str = "https://security-tracker.debian.org/tracker/data/json";
/// Garde-fou de téléchargement : le fichier fait 85 Mo à l'automne 2026.
pub const MAX_BYTES: u64 = 400 * 1024 * 1024;
const MAX_SUMMARY: usize = 400;

/// Une ligne du tracker, réduite à ce qui décide du verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub package: String,
    pub release: String,
    pub cve: String,
    pub status: Status,
    /// Version qui corrige, dans cette version de Debian.
    pub fixed_version: Option<String>,
    pub urgency: String,
    /// Debian a classé la faille sans annonce de sécurité (« Minor issue »,
    /// « postponed », non pris en charge) : rien ne viendra par le dépôt de
    /// sécurité.
    pub nodsa: bool,
    pub summary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Resolved,
    Open,
    Undetermined,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Resolved => "resolved",
            Self::Open => "open",
            Self::Undetermined => "undetermined",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "resolved" => Some(Self::Resolved),
            "open" => Some(Self::Open),
            "undetermined" => Some(Self::Undetermined),
            _ => None,
        }
    }
}

#[derive(Deserialize)]
struct RawCve {
    #[serde(default)]
    description: String,
    #[serde(default)]
    releases: HashMap<String, RawRelease>,
}

#[derive(Deserialize)]
struct RawRelease {
    status: String,
    #[serde(default)]
    fixed_version: Option<String>,
    #[serde(default)]
    urgency: String,
    #[serde(default)]
    nodsa: Option<String>,
}

/// Les entrées utiles d'une CVE pour une version de Debian, ou `None` quand
/// elle ne concerne pas cette version : non affectée (`fixed_version` à
/// `0`), jugée négligeable par Debian (`unimportant`), ou hors support.
fn keep(raw: &RawRelease) -> Option<(Status, Option<String>)> {
    let status = Status::parse(&raw.status)?;
    if matches!(raw.urgency.as_str(), "unimportant" | "end-of-life") {
        return None;
    }
    let fixed = raw.fixed_version.clone().filter(|v| !v.is_empty());
    if status == Status::Resolved && fixed.as_deref().is_none_or(|v| v == "0") {
        return None;
    }
    Some((status, fixed))
}

struct Tracker<'a> {
    packages: &'a HashSet<String>,
    releases: &'a HashSet<String>,
    out: &'a mut Vec<Entry>,
}

impl<'de> Visitor<'de> for Tracker<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("the Debian security tracker JSON map")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        while let Some(package) = map.next_key::<String>()? {
            if !self.packages.contains(&package) {
                map.next_value::<IgnoredAny>()?;
                continue;
            }
            let cves: HashMap<String, RawCve> = map.next_value()?;
            for (cve, raw) in cves {
                if !cve.starts_with("CVE-") {
                    continue;
                }
                for (release, detail) in &raw.releases {
                    if !self.releases.contains(release) {
                        continue;
                    }
                    let Some((status, fixed_version)) = keep(detail) else { continue };
                    self.out.push(Entry {
                        package: package.clone(),
                        release: release.clone(),
                        cve: cve.clone(),
                        status,
                        fixed_version,
                        urgency: detail.urgency.clone(),
                        nodsa: detail.nodsa.as_deref().is_some_and(|n| !n.is_empty()),
                        summary: clip(&raw.description, MAX_SUMMARY),
                    });
                }
            }
        }
        Ok(())
    }
}

impl<'de> DeserializeSeed<'de> for Tracker<'_> {
    type Value = ();

    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_map(self)
    }
}

/// Lit le tracker en flux et ne garde que `packages` (paquets source) pour
/// `releases` (noms de code : `bookworm`, `trixie`).
pub fn parse(
    reader: impl Read,
    packages: &HashSet<String>,
    releases: &HashSet<String>,
) -> Result<Vec<Entry>, String> {
    let mut out = Vec::new();
    let mut deserializer = serde_json::Deserializer::from_reader(reader);
    Tracker { packages, releases, out: &mut out }
        .deserialize(&mut deserializer)
        .map_err(|error| format!("unexpected Debian tracker answer: {error}"))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"{
      "openssh": {
        "CVE-2024-6387": {"description": "A security regression (CVE-2006-5051) was discovered in OpenSSH's server (sshd).",
          "scope": "local",
          "releases": {
            "bookworm": {"status": "resolved", "repositories": {"bookworm": "1:9.2p1-2+deb12u10"}, "fixed_version": "1:9.2p1-2+deb12u3", "urgency": "not yet assigned"},
            "bullseye": {"status": "resolved", "repositories": {}, "fixed_version": "0", "urgency": "not yet assigned"},
            "sid": {"status": "resolved", "repositories": {}, "fixed_version": "1:9.7p1-7", "urgency": "not yet assigned"}}},
        "CVE-2023-0002": {"description": "minor", "releases": {
            "bookworm": {"status": "open", "repositories": {}, "urgency": "unimportant"}}},
        "TEMP-0000000-ABCDEF": {"description": "", "releases": {"bookworm": {"status": "open", "urgency": "low"}}}
      },
      "linux": {
        "CVE-2026-1000": {"description": "kernel thing", "releases": {
          "bookworm": {"status": "open", "repositories": {}, "urgency": "not yet assigned", "nodsa": "Minor issue", "nodsa_reason": "postponed"}}}
      },
      "unwanted": {"CVE-2026-9": {"releases": {"bookworm": {"status": "open", "urgency": "high"}}}}
    }"#;

    fn set(items: &[&str]) -> HashSet<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn le_tracker_ne_garde_que_les_paquets_et_versions_demandes() {
        let mut entries =
            parse(FIXTURE.as_bytes(), &set(&["openssh", "linux"]), &set(&["bookworm", "bullseye"]))
                .unwrap();
        entries.sort_by(|a, b| a.cve.cmp(&b.cve));
        // Ni la négligeable, ni la non-affectée (bullseye, `0`), ni sid, ni
        // l'identifiant temporaire, ni le paquet non demandé.
        let found: Vec<(&str, &str)> =
            entries.iter().map(|e| (e.cve.as_str(), e.release.as_str())).collect();
        assert_eq!(found, [("CVE-2024-6387", "bookworm"), ("CVE-2026-1000", "bookworm")]);
        let ssh = &entries[0];
        assert_eq!(ssh.status, Status::Resolved);
        assert_eq!(ssh.fixed_version.as_deref(), Some("1:9.2p1-2+deb12u3"));
        assert!(!ssh.nodsa);
        assert!(ssh.summary.starts_with("A security regression"));
        let kernel = &entries[1];
        assert_eq!(kernel.status, Status::Open);
        assert!(kernel.nodsa && kernel.fixed_version.is_none());
        assert!(parse(&b"[1,2]"[..], &set(&[]), &set(&[])).is_err());
    }
}
