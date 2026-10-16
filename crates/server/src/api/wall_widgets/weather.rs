//! Météo : conditions actuelles, prochaines heures et prochains jours, par
//! Open-Meteo (sans clé, usage non commercial, données sous CC BY 4.0).
//!
//! Un seul appel pour toutes les villes (coordonnées séparées par des virgules),
//! mis en cache quinze minutes. La recherche de ville passe par l'API de
//! géocodage d'Open-Meteo, depuis les réglages seulement.

use serde::{Deserialize, Serialize};

use super::http;

pub const SOURCE: &str = "Open-Meteo";
pub const SOURCE_URL: &str = "https://open-meteo.com";
const FORECAST: &str = "https://api.open-meteo.com/v1/forecast";
const GEOCODING: &str = "https://geocoding-api.open-meteo.com/v1/search";
const MAX_BYTES: usize = 2 * 1024 * 1024;
/// Heures montrées après l'heure en cours (toutes les trois heures).
const HOUR_STEPS: [usize; 4] = [3, 6, 9, 12];
/// Jours montrés après aujourd'hui.
const DAYS: usize = 4;

/// Une ville choisie dans les réglages (telle que le géocodage l'a rendue).
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
pub struct Now {
    pub temperature: f64,
    pub feels_like: Option<f64>,
    /// Code météo WMO (0 : ciel clair … 99 : orage avec grêle).
    pub code: i32,
    pub is_day: bool,
    pub wind: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Hour {
    /// Heure locale de la ville, `2026-10-16T19:00`.
    pub time: String,
    pub temperature: f64,
    pub code: i32,
    pub is_day: bool,
    /// Probabilité de précipitations, en pour cent.
    pub precipitation: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Day {
    /// Date locale, `2026-10-17`.
    pub date: String,
    pub code: i32,
    pub max: f64,
    pub min: f64,
    pub precipitation: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PlaceWeather {
    pub name: String,
    pub region: String,
    pub now: Now,
    pub hours: Vec<Hour>,
    pub days: Vec<Day>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WeatherData {
    /// `°C` ou `°F`.
    pub temperature_unit: String,
    /// `km/h` ou `mph`.
    pub wind_unit: String,
    pub places: Vec<PlaceWeather>,
}

#[derive(Deserialize)]
struct Current {
    temperature_2m: Option<f64>,
    apparent_temperature: Option<f64>,
    weather_code: Option<i32>,
    is_day: Option<i32>,
    wind_speed_10m: Option<f64>,
}

#[derive(Deserialize)]
struct Hourly {
    time: Vec<String>,
    temperature_2m: Vec<Option<f64>>,
    weather_code: Vec<Option<i32>>,
    #[serde(default)]
    precipitation_probability: Vec<Option<f64>>,
    #[serde(default)]
    is_day: Vec<Option<i32>>,
}

#[derive(Deserialize)]
struct Daily {
    time: Vec<String>,
    weather_code: Vec<Option<i32>>,
    temperature_2m_max: Vec<Option<f64>>,
    temperature_2m_min: Vec<Option<f64>>,
    #[serde(default)]
    precipitation_probability_max: Vec<Option<f64>>,
}

#[derive(Deserialize)]
struct Forecast {
    current: Current,
    hourly: Hourly,
    daily: Daily,
}

/// Une ville : un objet ; plusieurs : un tableau, dans l'ordre demandé.
#[derive(Deserialize)]
#[serde(untagged)]
enum OneOrMany {
    Many(Vec<Forecast>),
    One(Box<Forecast>),
}

pub fn url(places: &[Place], imperial: bool) -> String {
    let join = |f: fn(&Place) -> f64| {
        places.iter().map(|p| format!("{:.4}", f(p))).collect::<Vec<_>>().join(",")
    };
    let units = if imperial { "&temperature_unit=fahrenheit&wind_speed_unit=mph" } else { "" };
    format!(
        "{FORECAST}?latitude={}&longitude={}\
         &current=temperature_2m,apparent_temperature,weather_code,is_day,wind_speed_10m\
         &hourly=temperature_2m,weather_code,precipitation_probability,is_day\
         &daily=weather_code,temperature_2m_max,temperature_2m_min,precipitation_probability_max\
         &timezone=auto&forecast_days={}&forecast_hours=13{units}",
        join(|p| p.latitude),
        join(|p| p.longitude),
        DAYS + 1,
    )
}

fn at<T: Copy>(values: &[Option<T>], index: usize) -> Option<T> {
    values.get(index).copied().flatten()
}

fn read(place: &Place, forecast: Forecast) -> Option<PlaceWeather> {
    let current = forecast.current;
    let now = Now {
        temperature: current.temperature_2m?,
        feels_like: current.apparent_temperature,
        code: current.weather_code.unwrap_or(0),
        is_day: current.is_day.unwrap_or(1) == 1,
        wind: current.wind_speed_10m,
    };
    let hourly = forecast.hourly;
    let hours = HOUR_STEPS
        .iter()
        .filter_map(|&i| {
            Some(Hour {
                time: hourly.time.get(i)?.clone(),
                temperature: at(&hourly.temperature_2m, i)?,
                code: at(&hourly.weather_code, i)?,
                is_day: at(&hourly.is_day, i).unwrap_or(1) == 1,
                precipitation: at(&hourly.precipitation_probability, i),
            })
        })
        .collect();
    let daily = forecast.daily;
    let days = (1..=DAYS)
        .filter_map(|i| {
            Some(Day {
                date: daily.time.get(i)?.clone(),
                code: at(&daily.weather_code, i)?,
                max: at(&daily.temperature_2m_max, i)?,
                min: at(&daily.temperature_2m_min, i)?,
                precipitation: at(&daily.precipitation_probability_max, i),
            })
        })
        .collect();
    Some(PlaceWeather { name: place.name.clone(), region: place.region.clone(), now, hours, days })
}

pub fn parse(body: &[u8], places: &[Place], imperial: bool) -> Result<WeatherData, String> {
    let forecasts = match http::json::<OneOrMany>(body)? {
        OneOrMany::Many(all) => all,
        OneOrMany::One(one) => vec![*one],
    };
    if forecasts.len() != places.len() {
        return Err("the forecast does not match the chosen places".into());
    }
    let places: Vec<PlaceWeather> =
        places.iter().zip(forecasts).filter_map(|(place, f)| read(place, f)).collect();
    if places.is_empty() {
        return Err("no forecast in the answer".into());
    }
    Ok(WeatherData {
        temperature_unit: if imperial { "°F" } else { "°C" }.into(),
        wind_unit: if imperial { "mph" } else { "km/h" }.into(),
        places,
    })
}

pub async fn fetch(
    client: &reqwest::Client,
    places: &[Place],
    imperial: bool,
) -> Result<WeatherData, String> {
    let body = http::bytes(client.get(url(places, imperial)), MAX_BYTES).await?;
    parse(&body, places, imperial)
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

    fn place(name: &str) -> Place {
        Place {
            name: name.into(),
            region: String::new(),
            country_code: "CH".into(),
            latitude: 46.516,
            longitude: 6.63282,
        }
    }

    fn forecast(temperature: f64) -> String {
        format!(
            r#"{{"latitude":46.5,"longitude":6.6,"timezone":"Europe/Zurich",
            "current":{{"time":"2026-10-16T16:30","temperature_2m":{temperature},"apparent_temperature":18.8,
                       "weather_code":3,"is_day":1,"wind_speed_10m":3.1}},
            "hourly":{{"time":["2026-10-16T16:00","2026-10-16T17:00","2026-10-16T18:00","2026-10-16T19:00",
                     "2026-10-16T20:00","2026-10-16T21:00","2026-10-16T22:00","2026-10-16T23:00",
                     "2026-10-17T00:00","2026-10-17T01:00","2026-10-17T02:00","2026-10-17T03:00","2026-10-17T04:00"],
              "temperature_2m":[18.3,18.5,18.4,17.8,16.3,16.1,15.5,15.0,15.0,14.7,14.4,14.1,13.9],
              "weather_code":[3,3,3,3,3,1,0,0,0,0,0,0,2],
              "precipitation_probability":[0,0,0,0,0,0,0,0,0,0,0,0,3],
              "is_day":[1,1,1,0,0,0,0,0,0,0,0,0,0]}},
            "daily":{{"time":["2026-10-16","2026-10-17","2026-10-18","2026-10-19","2026-10-20"],
              "weather_code":[3,63,61,3,61],"temperature_2m_max":[18.5,15.7,11.0,14.4,16.4],
              "temperature_2m_min":[12.8,10.9,8.7,10.3,12.3],
              "precipitation_probability_max":[0,58,44,25,null]}}}}"#
        )
    }

    #[test]
    fn reads_one_place() {
        let data = parse(forecast(18.4).as_bytes(), &[place("Lausanne")], false).unwrap();
        assert_eq!(data.temperature_unit, "°C");
        let lausanne = &data.places[0];
        assert_eq!(lausanne.now.temperature, 18.4);
        assert!(lausanne.now.is_day);
        let hours: Vec<&str> = lausanne.hours.iter().map(|h| h.time.as_str()).collect();
        assert_eq!(
            hours,
            ["2026-10-16T19:00", "2026-10-16T22:00", "2026-10-17T01:00", "2026-10-17T04:00"]
        );
        assert!(!lausanne.hours[0].is_day);
        assert_eq!(lausanne.days.len(), 4);
        assert_eq!(lausanne.days[0].date, "2026-10-17");
        assert_eq!(lausanne.days[0].code, 63);
        assert_eq!(lausanne.days[3].precipitation, None);
    }

    #[test]
    fn reads_several_places_in_order() {
        let body = format!("[{},{}]", forecast(18.4), forecast(21.0));
        let data = parse(body.as_bytes(), &[place("Lausanne"), place("Paris")], true).unwrap();
        assert_eq!(data.temperature_unit, "°F");
        assert_eq!(data.wind_unit, "mph");
        assert_eq!(data.places[1].name, "Paris");
        assert_eq!(data.places[1].now.temperature, 21.0);
        // Une réponse qui ne correspond pas aux villes demandées est refusée.
        assert!(parse(forecast(1.0).as_bytes(), &[place("A"), place("B")], false).is_err());
        assert!(
            parse(br#"{"error":true,"reason":"Latitude must be in range"}"#, &[place("A")], false)
                .is_err()
        );
    }

    #[test]
    fn builds_a_single_call() {
        let url = url(&[place("Lausanne"), place("Paris")], true);
        assert!(url.starts_with("https://api.open-meteo.com/v1/forecast?latitude=46.5160,46.5160"));
        assert!(url.contains("&longitude=6.6328,6.6328"));
        assert!(url.contains("temperature_unit=fahrenheit"));
        assert!(url.contains("forecast_days=5"));
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
