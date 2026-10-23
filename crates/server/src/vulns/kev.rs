//! Catalogue KEV de la CISA : les failles exploitées en ce moment. Un
//! composant vulnérable à l'une d'elles déclenche une alerte.

use serde::Deserialize;

pub const URL: &str =
    "https://www.cisa.gov/sites/default/files/feeds/known_exploited_vulnerabilities.json";
pub const MAX_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub cve: String,
    /// `2026-10-15`.
    pub date_added: String,
    pub ransomware: bool,
    pub name: String,
}

#[derive(Deserialize)]
struct Answer {
    vulnerabilities: Vec<RawEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawEntry {
    #[serde(rename = "cveID")]
    cve_id: String,
    #[serde(default)]
    vulnerability_name: String,
    #[serde(default)]
    date_added: String,
    #[serde(default)]
    known_ransomware_campaign_use: String,
}

pub fn parse(body: &[u8]) -> Result<Vec<Entry>, String> {
    let answer: Answer =
        serde_json::from_slice(body).map_err(|error| format!("unexpected KEV answer: {error}"))?;
    Ok(answer
        .vulnerabilities
        .into_iter()
        .filter(|raw| raw.cve_id.starts_with("CVE-"))
        .map(|raw| Entry {
            date_added: raw.date_added.get(..10).unwrap_or(&raw.date_added).to_string(),
            ransomware: raw.known_ransomware_campaign_use.eq_ignore_ascii_case("known"),
            name: raw.vulnerability_name,
            cve: raw.cve_id,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_catalogue_complet_est_lu() {
        let body = br#"{"title":"CISA Catalog","catalogVersion":"2026.10.15","count":2,"vulnerabilities":[
          {"cveID":"CVE-2024-6387","vendorProject":"OpenBSD","product":"OpenSSH","vulnerabilityName":"OpenSSH Race Condition",
           "dateAdded":"2026-10-01","knownRansomwareCampaignUse":"Known"},
          {"cveID":"CVE-2023-1","vulnerabilityName":"Other","dateAdded":"2023-01-10","knownRansomwareCampaignUse":"Unknown"}]}"#;
        let entries = parse(body).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].cve, "CVE-2024-6387");
        assert!(entries[0].ransomware && !entries[1].ransomware);
        assert_eq!(entries[0].date_added, "2026-10-01");
        assert!(parse(b"{}").is_err());
    }
}
