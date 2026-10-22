//! Les appels réseau vers les quatre sources, avec délais, tailles bornées
//! et limiteur de débit pour la NVD. Les adresses sont écrites ici : rien de
//! saisi par l'utilisateur ne choisit l'hôte appelé. Le client HTTP suit les
//! variables d'environnement `HTTPS_PROXY` / `HTTP_PROXY` / `NO_PROXY`.

use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

use super::nvd::{self, CpeRange, CveInfo};
use super::{debian, kev, osv};

const USER_AGENT: &str = concat!(
    "DumbMonit/",
    env!("CARGO_PKG_VERSION"),
    " (vulnerability scan; +https://github.com/laupernoe/dumbmonit)"
);
const TIMEOUT: Duration = Duration::from_secs(120);
const DEBIAN_TIMEOUT: Duration = Duration::from_secs(600);
const MAX_NVD_PAGE_BYTES: usize = 96 * 1024 * 1024;
const MAX_OSV_BYTES: usize = 32 * 1024 * 1024;
/// Espacement des requêtes NVD : 5 par 30 s sans clé, 50 avec.
const NVD_SPACING: Duration = Duration::from_millis(6_500);
const NVD_SPACING_WITH_KEY: Duration = Duration::from_millis(700);
/// Pages lues au plus pour un produit (2 000 CVE chacune).
const MAX_NVD_PAGES: usize = 10;

pub struct Fetcher {
    client: reqwest::Client,
    nvd_key: Option<String>,
    nvd_next: Mutex<Instant>,
}

fn describe(error: reqwest::Error) -> String {
    if error.is_timeout() {
        "timed out".to_string()
    } else if error.is_connect() {
        "connection failed (offline, DNS or proxy?)".to_string()
    } else {
        // Sans l'URL : elle n'apprend rien et alourdit l'interface.
        error.without_url().to_string()
    }
}

impl Fetcher {
    pub fn new(nvd_key: Option<String>) -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(3))
            .connect_timeout(Duration::from_secs(10))
            .timeout(TIMEOUT)
            .user_agent(USER_AGENT)
            .build()
            .map_err(|error| format!("HTTP client unavailable: {error}"))?;
        Ok(Self {
            client,
            nvd_key: nvd_key.filter(|k| !k.trim().is_empty()),
            nvd_next: Mutex::new(Instant::now()),
        })
    }

    pub fn has_nvd_key(&self) -> bool {
        self.nvd_key.is_some()
    }

    async fn send(&self, request: reqwest::RequestBuilder) -> Result<reqwest::Response, String> {
        let response = request.send().await.map_err(describe)?;
        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }
        Err(match status.as_u16() {
            429 => "rate limit reached".to_string(),
            403 if self.nvd_key.is_some() => "refused (403): check the NVD API key".to_string(),
            403 => "refused (403): rate limit reached".to_string(),
            404 => "not found (404)".to_string(),
            code => format!("answered {code}"),
        })
    }

    async fn bytes(&self, request: reqwest::RequestBuilder, max: usize) -> Result<Vec<u8>, String> {
        let mut response = self.send(request).await?;
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(describe)? {
            if body.len() + chunk.len() > max {
                return Err(format!("the answer is larger than {} MB", max / (1024 * 1024)));
            }
            body.extend_from_slice(&chunk);
        }
        Ok(body)
    }

    /// Une requête NVD, espacée des précédentes ; un refus de débit est
    /// retenté une fois après une fenêtre complète.
    async fn nvd_get(&self, url: &str) -> Result<Vec<u8>, String> {
        let mut attempt = 0;
        loop {
            {
                let mut next = self.nvd_next.lock().await;
                let wait = next.saturating_duration_since(Instant::now());
                tokio::time::sleep(wait).await;
                let spacing =
                    if self.nvd_key.is_some() { NVD_SPACING_WITH_KEY } else { NVD_SPACING };
                *next = Instant::now() + spacing;
            }
            let mut request = self.client.get(url);
            if let Some(key) = &self.nvd_key {
                request = request.header("apiKey", key);
            }
            match self.bytes(request, MAX_NVD_PAGE_BYTES).await {
                Err(error) if attempt == 0 && error.contains("rate limit") => {
                    attempt += 1;
                    tokio::time::sleep(Duration::from_secs(35)).await;
                }
                other => return other,
            }
        }
    }

    /// Les CVE d'un produit (tous ses CPE), plages réunies par CVE. `since` :
    /// seulement ce qui a changé depuis (différentiel).
    pub async fn nvd_product(
        &self,
        cpes: &[String],
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<(CveInfo, Vec<CpeRange>)>, String> {
        let now = Utc::now();
        let mut merged: BTreeMap<String, (CveInfo, Vec<CpeRange>)> = BTreeMap::new();
        for cpe in cpes {
            let mut start = 0;
            for _ in 0..MAX_NVD_PAGES {
                let body = self.nvd_get(&nvd::product_url(cpe, start, since, now)).await?;
                let (total, items) = nvd::parse_page(&body, cpes)?;
                let count = items.len();
                for item in items {
                    let entry = merged
                        .entry(item.info.id.clone())
                        .or_insert_with(|| (item.info.clone(), Vec::new()));
                    for range in item.ranges {
                        if !entry.1.contains(&range) {
                            entry.1.push(range);
                        }
                    }
                }
                start += nvd::PAGE;
                if count == 0 || start >= total {
                    break;
                }
            }
        }
        Ok(merged.into_values().collect())
    }

    /// La fiche d'une CVE (note CVSS).
    pub async fn nvd_cve(&self, id: &str) -> Result<Option<CveInfo>, String> {
        nvd::parse_single(&self.nvd_get(&nvd::cve_url(id)).await?)
    }

    pub async fn kev(&self) -> Result<Vec<kev::Entry>, String> {
        kev::parse(&self.bytes(self.client.get(kev::URL), kev::MAX_BYTES).await?)
    }

    /// Télécharge le tracker Debian dans `dir` puis le lit en flux, hors du
    /// fil d'exécution asynchrone.
    pub async fn debian(
        &self,
        dir: &Path,
        packages: HashSet<String>,
        releases: HashSet<String>,
    ) -> Result<Vec<debian::Entry>, String> {
        let path = dir.join("cve-debian-tracker.json.part");
        let result = self.download(&path).await;
        let parsed = match result {
            Ok(()) => {
                let path = path.clone();
                tokio::task::spawn_blocking(move || {
                    let file = std::fs::File::open(&path)
                        .map_err(|error| format!("temporary file unreadable: {error}"))?;
                    debian::parse(std::io::BufReader::new(file), &packages, &releases)
                })
                .await
                .map_err(|error| format!("Debian tracker parse aborted: {error}"))?
            }
            Err(error) => Err(error),
        };
        let _ = tokio::fs::remove_file(&path).await;
        parsed
    }

    async fn download(&self, path: &Path) -> Result<(), String> {
        let request = self.client.get(debian::URL).timeout(DEBIAN_TIMEOUT);
        let mut response = self.send(request).await?;
        let mut file = tokio::fs::File::create(path)
            .await
            .map_err(|error| format!("temporary file not created: {error}"))?;
        let mut written: u64 = 0;
        while let Some(chunk) = response.chunk().await.map_err(describe)? {
            written += chunk.len() as u64;
            if written > debian::MAX_BYTES {
                return Err("the Debian tracker is unexpectedly large".to_string());
            }
            file.write_all(&chunk)
                .await
                .map_err(|error| format!("temporary file not written: {error}"))?;
        }
        file.flush().await.map_err(|error| format!("temporary file not written: {error}"))?;
        Ok(())
    }

    pub async fn osv_batch(
        &self,
        queries: &[osv::Query],
    ) -> Result<Vec<Vec<osv::VulnRef>>, String> {
        let request = self.client.post(osv::QUERY_BATCH).json(&osv::batch_body(queries));
        osv::parse_batch(&self.bytes(request, MAX_OSV_BYTES).await?, queries.len())
    }

    pub async fn osv_vuln(&self, id: &str) -> Result<osv::Vuln, String> {
        // L'identifiant vient d'OSV lui-même ; on l'encode quand même.
        let url = format!("{}/{}", osv::VULN, id.replace(['/', '?', '#'], "_"));
        osv::parse_vuln(&self.bytes(self.client.get(url), MAX_OSV_BYTES).await?)
    }
}
