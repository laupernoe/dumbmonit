//! CVE critiques : les dernières failles notées « critique » (CVSS ≥ 9) par la
//! NVD, et les dernières entrées du catalogue KEV de la CISA (failles
//! activement exploitées). Les deux sont publics et sans clé.
//!
//! - NVD (API CVE 2.0) : sans clé, 5 requêtes par fenêtre de 30 secondes ; on
//!   en fait une par heure, sur les sept derniers jours. Mention exigée :
//!   « This product uses the NVD API but is not endorsed or certified by the
//!   NVD. » (documentation du mur).
//! - CISA KEV : un fichier JSON (≈ 2 Mo), relu une fois par heure.
//!
//! Chaque source a son cache : l'une peut tomber sans éteindre l'autre.

use std::collections::HashSet;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use super::http;

pub const SOURCE_NVD: &str = "NVD";
pub const SOURCE_KEV: &str = "CISA KEV";
pub const SOURCE_URL: &str = "https://nvd.nist.gov";
const NVD: &str = "https://services.nvd.nist.gov/rest/json/cves/2.0";
const KEV: &str =
    "https://www.cisa.gov/sites/default/files/feeds/known_exploited_vulnerabilities.json";
const MAX_NVD_BYTES: usize = 8 * 1024 * 1024;
const MAX_KEV_BYTES: usize = 8 * 1024 * 1024;
const WINDOW_DAYS: i64 = 7;
const MAX_TITLE: usize = 110;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CveItem {
    pub id: String,
    pub title: String,
    /// Note CVSS de base (v3.1 de la NVD de préférence) ; absente pour une
    /// entrée du KEV que la NVD n'a pas encore notée ici.
    pub score: Option<f64>,
    /// Publication (NVD) ou ajout au catalogue (KEV), UTC sans suffixe.
    pub date: String,
    /// Au catalogue KEV : exploitée en ce moment.
    pub exploited: bool,
    /// Le KEV la sait utilisée par des rançongiciels.
    pub ransomware: bool,
    pub link: String,
}

// --------------------------------------------------------------------- NVD

#[derive(Deserialize)]
struct NvdAnswer {
    #[serde(default)]
    vulnerabilities: Vec<NvdEntry>,
}

#[derive(Deserialize)]
struct NvdEntry {
    cve: NvdCve,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NvdCve {
    id: String,
    published: String,
    #[serde(default)]
    vuln_status: String,
    #[serde(default)]
    descriptions: Vec<NvdText>,
    #[serde(default)]
    metrics: NvdMetrics,
}

#[derive(Deserialize)]
struct NvdText {
    lang: String,
    value: String,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NvdMetrics {
    #[serde(default)]
    cvss_metric_v31: Vec<NvdMetric>,
    #[serde(default)]
    cvss_metric_v30: Vec<NvdMetric>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NvdMetric {
    #[serde(default, rename = "type")]
    kind: String,
    cvss_data: NvdCvss,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NvdCvss {
    base_score: f64,
}

pub fn nvd_url(now: DateTime<Utc>) -> String {
    let format = |t: DateTime<Utc>| t.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    format!(
        "{NVD}?cvssV3Severity=CRITICAL&noRejected&resultsPerPage=500&pubStartDate={}&pubEndDate={}",
        format(now - Duration::days(WINDOW_DAYS)),
        format(now)
    )
}

/// Coupe un texte trop long entre deux mots, avec des points de suspension.
fn clip(text: &str, max: usize) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.chars().count() <= max {
        return text;
    }
    let mut out = String::new();
    for word in text.split(' ') {
        let len = out.chars().count() + word.chars().count() + usize::from(!out.is_empty());
        if len > max - 1 {
            break;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    if out.is_empty() {
        out = text.chars().take(max - 1).collect();
    }
    let out = out.trim_end_matches([',', ';', ':', ' ']);
    format!("{out}…")
}

/// Le début de la description, pour un titre court : la première phrase.
fn short_title(description: &str) -> String {
    let text = description.split_whitespace().collect::<Vec<_>>().join(" ");
    let first = match text.find(". ") {
        Some(end) if end >= 30 => &text[..end],
        _ => text.as_str(),
    };
    clip(first, MAX_TITLE)
}

/// La note de la NVD elle-même (`Primary`) d'abord, sinon la plus haute.
fn score(metrics: &NvdMetrics) -> Option<f64> {
    let all = || metrics.cvss_metric_v31.iter().chain(&metrics.cvss_metric_v30);
    all()
        .find(|m| m.kind == "Primary")
        .map(|m| m.cvss_data.base_score)
        .or_else(|| all().map(|m| m.cvss_data.base_score).reduce(f64::max))
}

fn utc(text: &str) -> String {
    // `2026-10-09T09:17:08.430` : on garde la seconde.
    text.get(..19).unwrap_or(text).to_string()
}

pub fn parse_nvd(body: &[u8]) -> Result<Vec<CveItem>, String> {
    let answer: NvdAnswer = http::json(body)?;
    let mut items: Vec<CveItem> = answer
        .vulnerabilities
        .into_iter()
        .map(|entry| entry.cve)
        .filter(|cve| cve.vuln_status != "Rejected")
        .filter_map(|cve| {
            let score = score(&cve.metrics).filter(|s| *s >= 9.0)?;
            let description = cve
                .descriptions
                .iter()
                .find(|d| d.lang == "en")
                .or_else(|| cve.descriptions.first())
                .map(|d| d.value.as_str())
                .unwrap_or_default();
            Some(CveItem {
                link: format!("https://nvd.nist.gov/vuln/detail/{}", cve.id),
                title: short_title(description),
                score: Some(score),
                date: utc(&cve.published),
                exploited: false,
                ransomware: false,
                id: cve.id,
            })
        })
        .collect();
    items.sort_by(|a, b| b.date.cmp(&a.date));
    Ok(items)
}

// --------------------------------------------------------------------- KEV

#[derive(Deserialize)]
struct KevAnswer {
    vulnerabilities: Vec<KevEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct KevEntry {
    #[serde(rename = "cveID")]
    cve_id: String,
    #[serde(default)]
    vulnerability_name: String,
    date_added: String,
    #[serde(default)]
    known_ransomware_campaign_use: String,
}

/// Les entrées du catalogue, les plus récentes d'abord (30 au plus).
pub fn parse_kev(body: &[u8]) -> Result<Vec<CveItem>, String> {
    let answer: KevAnswer = http::json(body)?;
    let mut items: Vec<CveItem> = answer
        .vulnerabilities
        .into_iter()
        .map(|entry| CveItem {
            link: format!("https://nvd.nist.gov/vuln/detail/{}", entry.cve_id),
            title: clip(&entry.vulnerability_name, MAX_TITLE),
            score: None,
            date: format!("{}T00:00:00", entry.date_added.get(..10).unwrap_or(&entry.date_added)),
            exploited: true,
            ransomware: entry.known_ransomware_campaign_use.eq_ignore_ascii_case("known"),
            id: entry.cve_id,
        })
        .collect();
    items.sort_by(|a, b| b.date.cmp(&a.date).then_with(|| b.id.cmp(&a.id)));
    items.truncate(30);
    Ok(items)
}

pub async fn fetch_nvd(client: &reqwest::Client) -> Result<Vec<CveItem>, String> {
    parse_nvd(&http::bytes(client.get(nvd_url(Utc::now())), MAX_NVD_BYTES).await?)
}

pub async fn fetch_kev(client: &reqwest::Client) -> Result<Vec<CveItem>, String> {
    parse_kev(&http::bytes(client.get(KEV), MAX_KEV_BYTES).await?)
}

/// Les deux listes réunies : une faille du KEV déjà dans la liste de la NVD y
/// prend la marque « exploitée » ; les plus récentes d'abord.
pub fn combine(nvd: Vec<CveItem>, kev: Vec<CveItem>, count: usize) -> Vec<CveItem> {
    let exploited: HashSet<&str> = kev.iter().map(|k| k.id.as_str()).collect();
    let ransomware: HashSet<&str> =
        kev.iter().filter(|k| k.ransomware).map(|k| k.id.as_str()).collect();
    let mut items: Vec<CveItem> = nvd
        .into_iter()
        .map(|mut item| {
            item.exploited |= exploited.contains(item.id.as_str());
            item.ransomware |= ransomware.contains(item.id.as_str());
            item
        })
        .collect();
    let known: HashSet<String> = items.iter().map(|i| i.id.clone()).collect();
    items.extend(kev.iter().filter(|k| !known.contains(&k.id)).cloned());
    items.sort_by(|a, b| b.date.cmp(&a.date).then_with(|| b.id.cmp(&a.id)));
    items.truncate(count);
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    const NVD_FIXTURE: &str = r#"{"resultsPerPage":3,"startIndex":0,"totalResults":3,"format":"NVD_CVE","version":"2.0",
      "vulnerabilities":[
        {"cve":{"id":"CVE-2026-76779","published":"2026-10-09T09:17:08.430","vulnStatus":"Analyzed",
          "descriptions":[{"lang":"es","value":"Hola"},{"lang":"en","value":"Dell Secure Connect Gateway (SCG) Policy Manager, versions prior to 5.34.00.16, contains an Improper Restriction of Excessive Authentication Attempts vulnerability. An unauthenticated attacker could exploit it."}],
          "metrics":{"cvssMetricV31":[
            {"source":"security_alert@emc.com","type":"Secondary","cvssData":{"version":"3.1","baseScore":7.4,"baseSeverity":"HIGH"}},
            {"source":"nvd@nist.gov","type":"Primary","cvssData":{"version":"3.1","baseScore":9.8,"baseSeverity":"CRITICAL"}}]}}},
        {"cve":{"id":"CVE-2026-80001","published":"2026-10-12T18:00:00.000","vulnStatus":"Received",
          "descriptions":[{"lang":"en","value":"SQL injection in Acme Router admin page."}],
          "metrics":{"cvssMetricV31":[{"source":"cna@acme","type":"Secondary","cvssData":{"baseScore":10.0}}]}}},
        {"cve":{"id":"CVE-2026-80002","published":"2026-10-13T08:00:00.000","vulnStatus":"Received",
          "descriptions":[{"lang":"en","value":"Low one."}],
          "metrics":{"cvssMetricV31":[{"source":"cna@acme","type":"Primary","cvssData":{"baseScore":5.0}}],
                     "cvssMetricV40":[{"source":"cna@acme","type":"Secondary","cvssData":{"baseScore":9.3}}]}}}
      ]}"#;

    const KEV_FIXTURE: &str = r#"{"title":"CISA Catalog of Known Exploited Vulnerabilities","catalogVersion":"2026.10.15",
      "count":3,"vulnerabilities":[
        {"cveID":"CVE-2024-1111","vendorProject":"Old","product":"Thing","vulnerabilityName":"Old Thing RCE",
         "dateAdded":"2024-01-10","knownRansomwareCampaignUse":"Unknown"},
        {"cveID":"CVE-2026-21589","vendorProject":"Atlassian","product":"Multiple Products",
         "vulnerabilityName":"Atlassian Multiple Products Files or Directories Accessible to External Parties Vulnerability",
         "dateAdded":"2026-10-15","knownRansomwareCampaignUse":"Known"},
        {"cveID":"CVE-2026-80001","vendorProject":"Acme","product":"Router","vulnerabilityName":"Acme Router SQL Injection",
         "dateAdded":"2026-10-14","knownRansomwareCampaignUse":"Unknown"}
      ]}"#;

    #[test]
    fn reads_critical_nvd_entries_with_the_nvd_score() {
        let items = parse_nvd(NVD_FIXTURE.as_bytes()).unwrap();
        let ids: Vec<&str> = items.iter().map(|i| i.id.as_str()).collect();
        // La troisième n'est critique qu'en CVSS 4 : la note 3.1 (5.0) l'écarte.
        assert_eq!(ids, ["CVE-2026-80001", "CVE-2026-76779"]);
        let dell = &items[1];
        assert_eq!(dell.score, Some(9.8));
        assert_eq!(dell.date, "2026-10-09T09:17:08");
        assert_eq!(
            dell.title,
            "Dell Secure Connect Gateway (SCG) Policy Manager, versions prior to 5.34.00.16, \
             contains an Improper…"
        );
        assert_eq!(dell.link, "https://nvd.nist.gov/vuln/detail/CVE-2026-76779");
        assert_eq!(items[0].score, Some(10.0));
        assert!(parse_nvd(b"<html>").is_err());
        assert_eq!(clip("short", 10), "short");
        assert_eq!(clip("one two, three", 10), "one two…");
    }

    #[test]
    fn reads_the_kev_catalogue_newest_first() {
        let items = parse_kev(KEV_FIXTURE.as_bytes()).unwrap();
        assert_eq!(items[0].id, "CVE-2026-21589");
        assert_eq!(items[0].date, "2026-10-15T00:00:00");
        assert!(items[0].exploited && items[0].ransomware);
        assert!(!items[1].ransomware);
        assert_eq!(items[2].id, "CVE-2024-1111");
    }

    #[test]
    fn combines_both_and_marks_exploited_ones() {
        let nvd = parse_nvd(NVD_FIXTURE.as_bytes()).unwrap();
        let kev = parse_kev(KEV_FIXTURE.as_bytes()).unwrap();
        let items = combine(nvd, kev, 3);
        let ids: Vec<&str> = items.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(ids, ["CVE-2026-21589", "CVE-2026-80001", "CVE-2026-76779"]);
        // Dans les deux : la note de la NVD et la marque du KEV, une seule fois.
        assert_eq!(items[1].score, Some(10.0));
        assert!(items[1].exploited);
        assert!(!items[2].exploited);
    }

    #[test]
    fn asks_nvd_for_the_last_week() {
        let now = DateTime::parse_from_rfc3339("2026-10-16T12:00:00Z").unwrap().with_timezone(&Utc);
        let url = nvd_url(now);
        assert!(url.contains("cvssV3Severity=CRITICAL"));
        assert!(url.contains("pubStartDate=2026-10-09T12:00:00.000Z"));
        assert!(url.contains("pubEndDate=2026-10-16T12:00:00.000Z"));
    }
}
