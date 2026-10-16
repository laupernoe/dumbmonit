//! Météo : le temps qu'il fait maintenant (icône et température) dans une
//! ville, sous l'horloge du mur, par Open-Meteo (sans clé, usage non
//! commercial, données sous CC BY 4.0).
//!
//! Un appel, conditions actuelles seulement, mis en cache quinze minutes. La
//! recherche de ville passe par l'API de géocodage d'Open-Meteo, depuis les
//! réglages seulement.

use serde::{Deserialize, Serialize};

use super::http;

pub const SOURCE: &str = "Open-Meteo";
pub const SOURCE_URL: &str = "https://open-meteo.com";
const FORECAST: &str = "https://api.open-meteo.com/v1/forecast";
const GEOCODING: &str = "https://geocoding-api.open-meteo.com/v1/search";
const MAX_BYTES: usize = 256 * 1024;

/// La ville choisie dans les réglages (telle que le géocodage l'a rendue).
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

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WeatherData {
    pub name: String,
    /// `°C` ou `°F`.
    pub temperature_unit: String,
    pub temperature: f64,
    /// Code météo WMO (0 : ciel clair … 99 : orage avec grêle).
    pub code: i32,
    pub is_day: bool,
}

#[derive(Deserialize)]
struct Current {
    temperature_2m: Option<f64>,
    weather_code: Option<i32>,
    is_day: Option<i32>,
}

#[derive(Deserialize)]
struct Forecast {
    current: Current,
}

pub fn url(place: &Place, imperial: bool) -> String {
    let units = if imperial { "&temperature_unit=fahrenheit" } else { "" };
    format!(
        "{FORECAST}?latitude={:.4}&longitude={:.4}&current=temperature_2m,weather_code,is_day\
         &timezone=auto{units}",
        place.latitude, place.longitude,
    )
}

pub fn parse(body: &[u8], place: &Place, imperial: bool) -> Result<WeatherData, String> {
    let forecast: Forecast = http::json(body)?;
    let current = forecast.current;
    Ok(WeatherData {
        name: place.name.clone(),
        temperature_unit: if imperial { "°F" } else { "°C" }.into(),
        temperature: current.temperature_2m.ok_or("no temperature in the answer")?,
        code: current.weather_code.unwrap_or(0),
        is_day: current.is_day.unwrap_or(1) == 1,
    })
}

pub async fn fetch(
    client: &reqwest::Client,
    place: &Place,
    imperial: bool,
) -> Result<WeatherData, String> {
    let body = http::bytes(client.get(url(place, imperial)), MAX_BYTES).await?;
    parse(&body, place, imperial)
}

// ---------------------------------------------------------------- géocodage

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

pub fn parse_places(body: &[u8]) -> Result<Vec<Place>, String> {
    let answer: GeoAnswer = http::json(body)?;
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

pub async fn search(
    client: &reqwest::Client,
    query: &str,
    language: &str,
) -> Result<Vec<Place>, String> {
    let request = client.get(GEOCODING).query(&[
        ("name", query),
        ("count", "8"),
        ("language", language),
        ("format", "json"),
    ]);
    parse_places(&http::bytes(request, MAX_BYTES).await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn place() -> Place {
        Place {
            name: "Lausanne".into(),
            region: String::new(),
            country_code: "CH".into(),
            latitude: 46.516,
            longitude: 6.63282,
        }
    }

    #[test]
    fn reads_the_current_weather() {
        let body = br#"{"latitude":46.5,"longitude":6.6,"timezone":"Europe/Zurich",
            "current_units":{"temperature_2m":"C"},
            "current":{"time":"2026-10-16T16:30","interval":900,"temperature_2m":18.4,
                       "weather_code":3,"is_day":0}}"#;
        let data = parse(body, &place(), false).unwrap();
        assert_eq!(data.name, "Lausanne");
        assert_eq!(data.temperature, 18.4);
        assert_eq!(data.temperature_unit, "°C");
        assert_eq!(data.code, 3);
        assert!(!data.is_day);
        let f = parse(body, &place(), true).unwrap();
        assert_eq!(f.temperature_unit, "°F");
    }

    #[test]
    fn errors_are_named() {
        assert!(
            parse(br#"{"error":true,"reason":"Latitude must be in range"}"#, &place(), false)
                .is_err()
        );
        assert!(parse(br#"{"current":{"weather_code":1}}"#, &place(), false).is_err());
    }

    #[test]
    fn asks_for_the_current_conditions_only() {
        let url = url(&place(), true);
        assert!(url.starts_with(
            "https://api.open-meteo.com/v1/forecast?latitude=46.5160&longitude=6.6328"
        ));
        assert!(url.contains("current=temperature_2m,weather_code,is_day"));
        assert!(url.contains("temperature_unit=fahrenheit"));
        assert!(!url.contains("hourly") && !url.contains("daily"));
    }

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
}
