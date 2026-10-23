//! NVD, API CVE 2.0 : les failles d'un produit (par CPE) et la note d'une
//! faille isolée.
//!
//! - Par produit : `virtualMatchString=cpe:2.3:<part>:<vendor>:<product>`
//!   renvoie toutes les CVE dont une configuration nomme ce produit, avec les
//!   plages de versions (`versionStartIncluding`, `versionEndExcluding`…).
//!   La première lecture est complète ; les suivantes ne demandent que ce qui
//!   a changé (`lastModStartDate`/`lastModEndDate`, 120 jours au plus).
//! - Par faille : `cveId=` donne la note CVSS d'une CVE venue d'une autre
//!   source (Debian, OSV), qui n'en porte pas.
//!
//! Sans clé : 5 requêtes par fenêtre de 30 s ; avec clé (réglage) : 50. Le
//! limiteur de débit est dans `fetch.rs`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const API: &str = "https://services.nvd.nist.gov/rest/json/cves/2.0";
/// Taille d'une page : le maximum accepté par l'API.
pub const PAGE: usize = 2000;
/// Écart maximal entre `lastModStartDate` et `lastModEndDate`.
pub const MAX_DELTA_DAYS: i64 = 120;
const MAX_SUMMARY: usize = 400;

/// Une CVE telle qu'on la garde : de quoi l'afficher.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CveInfo {
    pub id: String,
    pub summary: String,
    pub cvss: Option<f64>,
    /// UTC sans suffixe, à la seconde.
    pub published: Option<String>,
}

/// Une plage de versions vulnérables d'un produit, tirée d'un `cpeMatch`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CpeRange {
    /// `a:vendor:product` (partie, éditeur, produit), tel que le catalogue.
    pub cpe: String,
    /// Version fixe du CPE (`8.0`), `*` ou `-` pour « toutes ».
    pub version: String,
    /// Champ `update` du CPE (`update_3`, `*`, `-`).
    pub update: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_including: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_excluding: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_including: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_excluding: Option<String>,
}

/// Une CVE et les plages qui concernent les produits demandés.
#[derive(Debug, Clone, PartialEq)]
pub struct ProductCve {
    pub info: CveInfo,
    pub ranges: Vec<CpeRange>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Answer {
    #[serde(default)]
    total_results: usize,
    #[serde(default)]
    vulnerabilities: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    cve: Cve,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Cve {
    id: String,
    #[serde(default)]
    published: String,
    #[serde(default)]
    vuln_status: String,
    #[serde(default)]
    descriptions: Vec<Text>,
    #[serde(default)]
    metrics: Metrics,
    #[serde(default)]
    configurations: Vec<Configuration>,
}

#[derive(Deserialize)]
struct Text {
    lang: String,
    value: String,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Metrics {
    #[serde(default)]
    cvss_metric_v40: Vec<Metric>,
    #[serde(default)]
    cvss_metric_v31: Vec<Metric>,
    #[serde(default)]
    cvss_metric_v30: Vec<Metric>,
    #[serde(default)]
    cvss_metric_v2: Vec<Metric>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Metric {
    #[serde(default, rename = "type")]
    kind: String,
    cvss_data: CvssData,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CvssData {
    base_score: f64,
}

#[derive(Deserialize)]
struct Configuration {
    #[serde(default)]
    nodes: Vec<Node>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Node {
    #[serde(default)]
    negate: bool,
    #[serde(default)]
    cpe_match: Vec<CpeMatch>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CpeMatch {
    vulnerable: bool,
    criteria: String,
    version_start_including: Option<String>,
    version_start_excluding: Option<String>,
    version_end_including: Option<String>,
    version_end_excluding: Option<String>,
}

/// Note CVSS : v3.1, puis v3.0, puis v4.0, puis v2 ; dans chaque version,
/// celle de la NVD (`Primary`) d'abord, sinon la plus haute.
fn score(metrics: &Metrics) -> Option<f64> {
    [
        &metrics.cvss_metric_v31,
        &metrics.cvss_metric_v30,
        &metrics.cvss_metric_v40,
        &metrics.cvss_metric_v2,
    ]
    .into_iter()
    .find_map(|list| {
        list.iter()
            .find(|m| m.kind == "Primary")
            .map(|m| m.cvss_data.base_score)
            .or_else(|| list.iter().map(|m| m.cvss_data.base_score).reduce(f64::max))
    })
    .filter(|s| s.is_finite())
}

/// Coupe un texte trop long sur une limite de mot.
pub fn clip(text: &str, max: usize) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.chars().count() <= max {
        return text;
    }
    let cut: String = text.chars().take(max).collect();
    let cut = match cut.rfind(' ') {
        Some(space) if space > max / 2 => &cut[..space],
        _ => cut.as_str(),
    };
    format!("{}…", cut.trim_end_matches([',', ';', ':', '.']))
}

/// `2026-10-09T09:17:08.430` → `2026-10-09 09:17:08`.
pub fn utc(text: &str) -> Option<String> {
    let text = text.get(..19)?;
    Some(text.replace('T', " "))
}

/// Découpe un CPE 2.3 : `cpe:2.3:a:proxmox:virtual_environment:8.0:*:…`
/// → (`a:proxmox:virtual_environment`, `8.0`, `*`).
fn split_cpe(criteria: &str) -> Option<(String, String, String)> {
    // Les deux-points échappés (`\:`) n'apparaissent pas dans les produits
    // du catalogue ; un découpage simple suffit.
    let mut fields = criteria.strip_prefix("cpe:2.3:")?.split(':');
    let part = fields.next()?;
    let vendor = fields.next()?;
    let product = fields.next()?;
    let version = fields.next().unwrap_or("*");
    let update = fields.next().unwrap_or("*");
    Some((format!("{part}:{vendor}:{product}"), version.to_string(), update.to_string()))
}

fn info_of(cve: &Cve) -> CveInfo {
    let description = cve
        .descriptions
        .iter()
        .find(|d| d.lang == "en")
        .or_else(|| cve.descriptions.first())
        .map(|d| d.value.as_str())
        .unwrap_or_default();
    CveInfo {
        id: cve.id.clone(),
        summary: clip(description, MAX_SUMMARY),
        cvss: score(&cve.metrics),
        published: utc(&cve.published),
    }
}

/// Une page de réponse : le total annoncé et les CVE, réduites aux plages
/// des CPE de `products` (`a:vendor:product`). Les CVE rejetées sont
/// écartées.
pub fn parse_page(body: &[u8], products: &[String]) -> Result<(usize, Vec<ProductCve>), String> {
    let answer: Answer =
        serde_json::from_slice(body).map_err(|error| format!("unexpected NVD answer: {error}"))?;
    let items = answer
        .vulnerabilities
        .into_iter()
        .map(|entry| entry.cve)
        .filter(|cve| cve.vuln_status != "Rejected")
        .map(|cve| {
            let ranges = cve
                .configurations
                .iter()
                .flat_map(|configuration| &configuration.nodes)
                .filter(|node| !node.negate)
                .flat_map(|node| &node.cpe_match)
                .filter(|m| m.vulnerable)
                .filter_map(|m| {
                    let (cpe, version, update) = split_cpe(&m.criteria)?;
                    products.contains(&cpe).then(|| CpeRange {
                        cpe,
                        version,
                        update,
                        start_including: m.version_start_including.clone(),
                        start_excluding: m.version_start_excluding.clone(),
                        end_including: m.version_end_including.clone(),
                        end_excluding: m.version_end_excluding.clone(),
                    })
                })
                .collect();
            ProductCve { info: info_of(&cve), ranges }
        })
        .collect();
    Ok((answer.total_results, items))
}

/// La fiche d'une CVE isolée (`cveId=`), pour sa note.
pub fn parse_single(body: &[u8]) -> Result<Option<CveInfo>, String> {
    let answer: Answer =
        serde_json::from_slice(body).map_err(|error| format!("unexpected NVD answer: {error}"))?;
    Ok(answer.vulnerabilities.first().map(|entry| info_of(&entry.cve)))
}

fn iso(t: DateTime<Utc>) -> String {
    t.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

/// URL d'une page de CVE d'un produit ; `since` borne aux modifications
/// récentes (moins de [`MAX_DELTA_DAYS`] jours, sinon lecture complète).
pub fn product_url(
    cpe: &str,
    start: usize,
    since: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> String {
    let mut url = format!(
        "{API}?virtualMatchString=cpe:2.3:{cpe}&noRejected&resultsPerPage={PAGE}&startIndex={start}"
    );
    if let Some(since) = since.filter(|s| now - *s < chrono::Duration::days(MAX_DELTA_DAYS)) {
        url.push_str(&format!("&lastModStartDate={}&lastModEndDate={}", iso(since), iso(now)));
    }
    url.replace('\\', "%5C").replace('+', "%2B")
}

pub fn cve_url(id: &str) -> String {
    format!("{API}?cveId={id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"{"resultsPerPage":3,"startIndex":0,"totalResults":3,"format":"NVD_CVE","version":"2.0",
      "vulnerabilities":[
        {"cve":{"id":"CVE-2024-21545","published":"2024-09-25T01:15:22.010","vulnStatus":"Analyzed",
          "descriptions":[{"lang":"en","value":"Proxmox Virtual Environment is an open-source server management platform for enterprise virtualization. Insufficient safeguards against malicious API responses allow authenticated attackers to read arbitrary files."}],
          "metrics":{"cvssMetricV31":[{"source":"nvd@nist.gov","type":"Primary","cvssData":{"version":"3.1","baseScore":8.1,"baseSeverity":"HIGH"}}]},
          "configurations":[{"nodes":[{"operator":"OR","negate":false,"cpeMatch":[
            {"vulnerable":true,"criteria":"cpe:2.3:a:proxmox:virtual_environment:*:*:*:*:*:*:*:*","versionEndExcluding":"8.2.5","matchCriteriaId":"X"},
            {"vulnerable":true,"criteria":"cpe:2.3:a:proxmox:backup_server:*:*:*:*:*:*:*:*","versionEndExcluding":"3.2.8","matchCriteriaId":"Y"},
            {"vulnerable":false,"criteria":"cpe:2.3:o:debian:debian_linux:12.0:*:*:*:*:*:*:*","matchCriteriaId":"Z"}]}]}]}},
        {"cve":{"id":"CVE-2023-0001","published":"2023-01-01T00:00:00.000","vulnStatus":"Rejected",
          "descriptions":[{"lang":"en","value":"** REJECT **"}],"configurations":[]}},
        {"cve":{"id":"CVE-2025-1111","published":"2025-02-01T10:00:00.000","vulnStatus":"Awaiting Analysis",
          "descriptions":[{"lang":"en","value":"Something in ESXi."}],
          "metrics":{"cvssMetricV31":[{"source":"vmware","type":"Secondary","cvssData":{"baseScore":9.3}}]},
          "configurations":[{"nodes":[{"operator":"OR","negate":false,"cpeMatch":[
            {"vulnerable":true,"criteria":"cpe:2.3:o:vmware:esxi:8.0:update_3:*:*:*:*:*:*","matchCriteriaId":"E"}]}]}]}}
      ]}"#;

    #[test]
    fn une_page_garde_les_plages_des_produits_demandes() {
        let products = vec!["a:proxmox:virtual_environment".to_string(), "o:vmware:esxi".into()];
        let (total, items) = parse_page(FIXTURE.as_bytes(), &products).unwrap();
        assert_eq!(total, 3);
        let ids: Vec<&str> = items.iter().map(|i| i.info.id.as_str()).collect();
        assert_eq!(ids, ["CVE-2024-21545", "CVE-2025-1111"], "la rejetée est écartée");
        let pve = &items[0];
        assert_eq!(pve.info.cvss, Some(8.1));
        assert_eq!(pve.info.published.as_deref(), Some("2024-09-25 01:15:22"));
        assert_eq!(pve.ranges.len(), 1, "ni PBS (non demandé) ni Debian (non vulnérable)");
        assert_eq!(pve.ranges[0].end_excluding.as_deref(), Some("8.2.5"));
        assert_eq!(pve.ranges[0].version, "*");
        let esxi = &items[1];
        assert_eq!(esxi.info.cvss, Some(9.3), "faute de note Primary, la plus haute");
        assert_eq!(esxi.ranges[0].version, "8.0");
        assert_eq!(esxi.ranges[0].update, "update_3");
        assert!(parse_page(b"<html>", &products).is_err());
    }

    #[test]
    fn une_fiche_isolee_donne_la_note() {
        let info = parse_single(FIXTURE.as_bytes()).unwrap().unwrap();
        assert_eq!(info.id, "CVE-2024-21545");
        assert!(info.summary.starts_with("Proxmox Virtual Environment"));
        let empty = parse_single(br#"{"totalResults":0,"vulnerabilities":[]}"#).unwrap();
        assert!(empty.is_none());
    }

    #[test]
    fn l_url_borne_le_differentiel_a_cent_vingt_jours() {
        let now = DateTime::parse_from_rfc3339("2026-10-16T12:00:00Z").unwrap().to_utc();
        let recent = now - chrono::Duration::days(1);
        let url = product_url("a:notepad-plus-plus:notepad\\+\\+", 0, Some(recent), now);
        assert!(url.contains("virtualMatchString=cpe:2.3:a:notepad-plus-plus:notepad%5C%2B%5C%2B"));
        assert!(url.contains("lastModStartDate=2026-10-15T12:00:00.000Z"));
        assert!(url.contains("lastModEndDate=2026-10-16T12:00:00.000Z"));
        let old = now - chrono::Duration::days(200);
        assert!(!product_url("a:x:y", 2000, Some(old), now).contains("lastMod"));
        assert!(product_url("a:x:y", 2000, None, now).contains("startIndex=2000"));
        assert_eq!(clip("one two three", 7), "one two…");
        assert_eq!(utc("2026-10-09T09:17:08.430").as_deref(), Some("2026-10-09 09:17:08"));
    }
}
