//! Recherche de ville pour placer un lieu sur la carte du monde
//! (`GET /api/locations/geocode`), par l'API de géocodage d'Open-Meteo (sans
//! clé, données sous CC BY 4.0). La requête part du serveur, jamais du
//! navigateur ; l'hôte appelé est fixe, rien de saisi ne le choisit.
//!
//! Le module servait autrefois aussi la météo des widgets du mode mur ; ces
//! widgets ont disparu avec l'interface Homelab, seul le géocodage reste.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::api::{ApiError, ApiResult};

const GEOCODING: &str = "https://geocoding-api.open-meteo.com/v1/search";
const MAX_BYTES: usize = 256 * 1024;
const TIMEOUT: Duration = Duration::from_secs(8);
const USER_AGENT: &str =
    concat!("DumbMonit/", env!("CARGO_PKG_VERSION"), " (+https://github.com/laupernoe/dumbmonit)");

/// Une ville telle que le géocodage l'a rendue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Place {
    pub name: String,
    /// Région et pays, pour distinguer deux villes du même nom.
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub country_code: String,
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Deserialize)]
struct GeoResult {
    name: String,
    latitude: f64,
    longitude: f64,
    #[serde(default)]
    country_code: Option<String>,
    #[serde(default)]
    country: Option<String>,
    #[serde(default)]
    admin1: Option<String>,
}

#[derive(Deserialize)]
struct GeoAnswer {
    #[serde(default)]
    results: Vec<GeoResult>,
}

fn parse_places(body: &[u8]) -> Result<Vec<Place>, String> {
    let answer: GeoAnswer =
        serde_json::from_slice(body).map_err(|error| format!("unexpected answer: {error}"))?;
    Ok(answer
        .results
        .into_iter()
        .map(|r| {
            let region = [r.admin1, r.country]
                .into_iter()
                .flatten()
                .filter(|part| !part.is_empty() && *part != r.name)
                .collect::<Vec<_>>()
                .join(", ");
            Place {
                name: r.name,
                region,
                country_code: r.country_code.unwrap_or_default(),
                latitude: r.latitude,
                longitude: r.longitude,
            }
        })
        .collect())
}

/// Décrit une erreur réseau sans l'adresse, en 160 caractères au plus.
fn describe(error: reqwest::Error) -> String {
    if error.is_timeout() {
        return "no answer in time".into();
    }
    let error = error.without_url();
    let mut text = error.to_string();
    let mut cause = std::error::Error::source(&error);
    while let Some(inner) = cause {
        text.push_str(": ");
        text.push_str(&inner.to_string());
        cause = inner.source();
    }
    if text.chars().count() <= 160 {
        return text;
    }
    let cut: String = text.chars().take(159).collect();
    format!("{}…", cut.trim_end())
}

async fn fetch(query: &str, language: &str) -> Result<Vec<Place>, String> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(3))
        .connect_timeout(Duration::from_secs(5))
        .timeout(TIMEOUT)
        .user_agent(USER_AGENT)
        .build()
        .map_err(|error| format!("HTTP client unavailable: {error}"))?;
    let request = client.get(GEOCODING).query(&[
        ("name", query),
        ("count", "8"),
        ("language", language),
        ("format", "json"),
    ]);
    let mut response = request.send().await.map_err(describe)?;
    let status = response.status();
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err("the provider's rate limit was reached".into());
    }
    if !status.is_success() {
        return Err(format!("answered {}", status.as_u16()));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(describe)? {
        if body.len() + chunk.len() > MAX_BYTES {
            return Err("the answer is too large".into());
        }
        body.extend_from_slice(&chunk);
    }
    parse_places(&body)
}

/// Cherche une ville (deux lettres au moins), dans la langue demandée.
pub(crate) async fn search_places(q: &str, lang: Option<&str>) -> ApiResult<Vec<Place>> {
    let q = q.trim();
    if !(2..=80).contains(&q.chars().count()) {
        return Err(ApiError::BadRequest("Type at least two letters.".into()));
    }
    let lang = lang
        .map(|l| l.split(['-', '_']).next().unwrap_or("en").to_ascii_lowercase())
        .filter(|l| l.len() == 2 && l.bytes().all(|b| b.is_ascii_lowercase()))
        .unwrap_or_else(|| "en".into());
    fetch(q, &lang).await.map_err(ApiError::BadGateway)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_geocoding_answer() {
        let body = br#"{"results":[{"id":2659994,"name":"Lausanne","latitude":46.516,"longitude":6.63282,
            "country_code":"CH","timezone":"Europe/Zurich","country":"Suisse","admin1":"Canton de Vaud"},
            {"id":1,"name":"Monaco","latitude":43.7,"longitude":7.4,"country_code":"MC","country":"Monaco"}],
            "generationtime_ms":0.12}"#;
        let places = parse_places(body).unwrap();
        assert_eq!(places[0].region, "Canton de Vaud, Suisse");
        assert_eq!(places[1].region, "");
        assert!(parse_places(br#"{"generationtime_ms":0.1}"#).unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_short_query_is_refused_before_any_call() {
        assert!(search_places("a", None).await.is_err());
        assert!(search_places(&"x".repeat(81), Some("fr")).await.is_err());
    }
}
