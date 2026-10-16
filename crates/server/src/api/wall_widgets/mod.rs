//! Widgets du mode mur : cryptomonnaies, météo, actions, devises et CVE
//! critiques, lus par le serveur auprès de sources publiques.
//!
//! - `GET /api/wall/widgets` : ce que le mur interroge chaque minute — une
//!   section par widget allumé (`null` sinon), chacune avec ses données, sa
//!   date de lecture, son erreur éventuelle et sa source à citer.
//! - `GET|PUT /api/wall/widgets/settings` : les réglages (administrateurs).
//! - `GET /api/wall/widgets/catalog` : monnaies et devises proposées.
//! - `GET /api/wall/widgets/places?q=` : recherche de ville (administrateurs).
//!
//! Tout passe par le serveur : aucun écran ne parle à une source tierce (ni son
//! adresse IP ni ses limites d'appels n'entrent en jeu), et chaque source a son
//! cache ([`cache`]) à la mesure de ses conditions d'usage. Une source qui tombe
//! laisse sa dernière valeur à l'écran quelques heures, puis le widget dit
//! « indisponible » ; jamais un mur cassé.
//!
//! Les réglages vivent dans la table `settings` (clé `wall.widgets`) ; les clés
//! d'API facultatives (Finnhub, CoinGecko) y sont chiffrées sous leur propre
//! clé et ne reviennent jamais : `has_*_key` seulement. Tout est éteint par
//! défaut.

mod cache;
mod coins;
mod cve;
mod fx;
mod http;
mod stocks;
mod weather;

use std::sync::LazyLock;
use std::time::Duration;

use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::api::{ApiError, ApiResult};
use crate::auth::middleware::{AdminIdentity, Identity};
use crate::auth::settings;
use crate::state::AppState;
use cache::{Cache, Outcome};

const SETTINGS_KEY: &str = "wall.widgets";
const FINNHUB_KEY: &str = "wall.widgets.finnhub_key";
const COINGECKO_KEY: &str = "wall.widgets.coingecko_key";

const CRYPTO_TTL: Duration = Duration::from_secs(60);
const WEATHER_TTL: Duration = Duration::from_secs(15 * 60);
const STOCKS_TTL: Duration = Duration::from_secs(5 * 60);
const FX_TTL: Duration = Duration::from_secs(3600);
const CVE_TTL: Duration = Duration::from_secs(3600);

const MAX_COINS: usize = 10;
const MAX_PLACES: usize = 4;
const MAX_SYMBOLS: usize = 10;
const MAX_CURRENCIES: usize = 10;
const MAX_KEY_LEN: usize = 200;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/wall/widgets", get(widgets))
        .route("/wall/widgets/settings", get(get_settings).put(put_settings))
        .route("/wall/widgets/catalog", get(catalog))
        .route("/wall/widgets/places", get(places))
}

// ------------------------------------------------------------------ réglages

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CryptoSettings {
    pub enabled: bool,
    /// Identifiants CoinGecko, dans l'ordre d'affichage.
    pub coins: Vec<String>,
    /// Code CoinGecko en minuscules (`usd`, `eur`…).
    pub currency: String,
}

impl Default for CryptoSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            coins: vec!["bitcoin".into(), "ethereum".into(), "solana".into()],
            currency: "usd".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WeatherSettings {
    pub enabled: bool,
    pub places: Vec<weather::Place>,
    /// `metric` ou `imperial`.
    pub units: String,
}

impl Default for WeatherSettings {
    fn default() -> Self {
        Self { enabled: false, places: Vec::new(), units: "metric".into() }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StocksSettings {
    pub enabled: bool,
    pub symbols: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FxSettings {
    pub enabled: bool,
    pub base: String,
    pub currencies: Vec<String>,
}

impl Default for FxSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            base: "EUR".into(),
            currencies: ["USD", "GBP", "CHF", "JPY", "CNY"].map(String::from).to_vec(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CveSettings {
    pub enabled: bool,
    /// `both` (NVD et KEV), `nvd` (critiques) ou `kev` (exploitées).
    pub source: String,
    /// Lignes affichées.
    pub count: usize,
}

impl Default for CveSettings {
    fn default() -> Self {
        Self { enabled: false, source: "both".into(), count: 5 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WidgetSettings {
    /// Secondes passées sur chaque widget avant le suivant.
    pub rotate_seconds: u32,
    pub crypto: CryptoSettings,
    pub weather: WeatherSettings,
    pub stocks: StocksSettings,
    pub fx: FxSettings,
    pub cve: CveSettings,
}

impl Default for WidgetSettings {
    fn default() -> Self {
        Self {
            rotate_seconds: 20,
            crypto: CryptoSettings::default(),
            weather: WeatherSettings::default(),
            stocks: StocksSettings::default(),
            fx: FxSettings::default(),
            cve: CveSettings::default(),
        }
    }
}

/// Ce que `GET` rend : les réglages, et seulement la présence des clés.
#[derive(Debug, Serialize)]
pub struct SettingsView {
    #[serde(flatten)]
    pub settings: WidgetSettings,
    pub has_finnhub_key: bool,
    pub has_coingecko_key: bool,
}

/// Clés d'API reçues : absente, inchangée ; chaîne vide, effacée.
#[derive(Debug, Default, Deserialize)]
pub struct SecretsInput {
    pub finnhub: Option<String>,
    pub coingecko: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SettingsInput {
    #[serde(flatten)]
    pub settings: WidgetSettings,
    #[serde(default)]
    pub secrets: Option<SecretsInput>,
}

async fn load(state: &AppState) -> anyhow::Result<WidgetSettings> {
    Ok(settings::get(&state.pool, SETTINGS_KEY).await?.unwrap_or_default())
}

async fn secret(state: &AppState, key: &str) -> Option<String> {
    match settings::get_secret(&state.pool, &state.cipher, key).await {
        Ok(value) => value.filter(|v| !v.is_empty()),
        Err(error) => {
            tracing::warn!(%error, key, "clé d'API du mur illisible");
            None
        }
    }
}

async fn view(state: &AppState, settings: WidgetSettings) -> SettingsView {
    SettingsView {
        settings,
        has_finnhub_key: secret(state, FINNHUB_KEY).await.is_some(),
        has_coingecko_key: secret(state, COINGECKO_KEY).await.is_some(),
    }
}

async fn get_settings(
    State(state): State<AppState>,
    AdminIdentity(_): AdminIdentity,
) -> ApiResult<Json<SettingsView>> {
    let settings = load(&state).await?;
    Ok(Json(view(&state, settings).await))
}

async fn put_settings(
    State(state): State<AppState>,
    AdminIdentity(_): AdminIdentity,
    Json(payload): Json<SettingsInput>,
) -> ApiResult<Json<SettingsView>> {
    let mut settings = payload.settings;
    validate(&mut settings).map_err(ApiError::BadRequest)?;
    if let Some(secrets) = payload.secrets {
        for (key, value) in [(FINNHUB_KEY, secrets.finnhub), (COINGECKO_KEY, secrets.coingecko)] {
            let Some(value) = value.map(|v| v.trim().to_string()) else { continue };
            if value.is_empty() {
                settings::delete(&state.pool, key).await?;
            } else if value.len() > MAX_KEY_LEN || value.chars().any(|c| c.is_control()) {
                return Err(ApiError::BadRequest("That API key does not look valid.".into()));
            } else {
                settings::set_secret(&state.pool, &state.cipher, key, &value).await?;
            }
        }
    }
    settings::set(&state.pool, SETTINGS_KEY, &settings).await?;
    Ok(Json(view(&state, settings).await))
}

fn dedupe(values: &mut Vec<String>) {
    let mut seen = std::collections::HashSet::new();
    values.retain(|v| !v.is_empty() && seen.insert(v.clone()));
}

/// Nettoie et valide les réglages reçus.
pub fn validate(s: &mut WidgetSettings) -> Result<(), String> {
    if !(5..=300).contains(&s.rotate_seconds) {
        return Err("Each widget stays between 5 and 300 seconds.".into());
    }

    let crypto = &mut s.crypto;
    crypto.coins.iter_mut().for_each(|c| *c = c.trim().to_ascii_lowercase());
    dedupe(&mut crypto.coins);
    if crypto.coins.len() > MAX_COINS {
        return Err(format!("At most {MAX_COINS} coins."));
    }
    if let Some(bad) = crypto.coins.iter().find(|c| !coins::valid_id(c)) {
        return Err(format!("\"{bad}\" is not a CoinGecko coin id."));
    }
    crypto.currency = crypto.currency.trim().to_ascii_lowercase();
    if !coins::CURRENCIES.contains(&crypto.currency.as_str()) {
        return Err("Unknown display currency for the coins.".into());
    }
    if crypto.enabled && crypto.coins.is_empty() {
        return Err("Pick at least one coin, or turn the coins off.".into());
    }

    let weather = &mut s.weather;
    if weather.places.len() > MAX_PLACES {
        return Err(format!("At most {MAX_PLACES} places."));
    }
    for place in &mut weather.places {
        place.name = place.name.trim().chars().take(80).collect();
        place.region = place.region.trim().chars().take(120).collect();
        place.country_code = place.country_code.trim().chars().take(2).collect();
        let ok = !place.name.is_empty()
            && (-90.0..=90.0).contains(&place.latitude)
            && (-180.0..=180.0).contains(&place.longitude);
        if !ok {
            return Err("A place has no name or impossible coordinates.".into());
        }
    }
    if !matches!(weather.units.as_str(), "metric" | "imperial") {
        return Err("Unknown units.".into());
    }
    if weather.enabled && weather.places.is_empty() {
        return Err("Add at least one place, or turn the weather off.".into());
    }

    let stocks = &mut s.stocks;
    stocks.symbols.iter_mut().for_each(|c| *c = c.trim().to_ascii_uppercase());
    dedupe(&mut stocks.symbols);
    if stocks.symbols.len() > MAX_SYMBOLS {
        return Err(format!("At most {MAX_SYMBOLS} symbols."));
    }
    if let Some(bad) = stocks.symbols.iter().find(|c| !stocks::valid_symbol(c)) {
        return Err(format!("\"{bad}\" is not a ticker symbol."));
    }
    if stocks.enabled && stocks.symbols.is_empty() {
        return Err("Add at least one symbol, or turn the stocks off.".into());
    }

    let fx = &mut s.fx;
    fx.base = fx.base.trim().to_ascii_uppercase();
    fx.currencies.iter_mut().for_each(|c| *c = c.trim().to_ascii_uppercase());
    let base = fx.base.clone();
    fx.currencies.retain(|c| *c != base);
    dedupe(&mut fx.currencies);
    if !fx::CURRENCIES.contains(&fx.base.as_str()) {
        return Err("Unknown base currency.".into());
    }
    if fx.currencies.len() > MAX_CURRENCIES {
        return Err(format!("At most {MAX_CURRENCIES} currencies."));
    }
    if let Some(bad) = fx.currencies.iter().find(|c| !fx::CURRENCIES.contains(&c.as_str())) {
        return Err(format!("Unknown currency \"{bad}\"."));
    }
    if fx.enabled && fx.currencies.is_empty() {
        return Err("Pick at least one currency, or turn the rates off.".into());
    }

    if !matches!(s.cve.source.as_str(), "both" | "nvd" | "kev") {
        return Err("Unknown vulnerability source.".into());
    }
    if !(1..=10).contains(&s.cve.count) {
        return Err("Show between 1 and 10 vulnerabilities.".into());
    }
    Ok(())
}

// ---------------------------------------------------------------- catalogue

#[derive(Serialize)]
struct Catalog {
    coins: &'static [coins::Preset],
    coin_currencies: &'static [&'static str],
    fx_currencies: &'static [&'static str],
}

async fn catalog(Identity(_): Identity) -> Json<Catalog> {
    Json(Catalog {
        coins: coins::PRESETS,
        coin_currencies: coins::CURRENCIES,
        fx_currencies: fx::CURRENCIES,
    })
}

#[derive(Deserialize)]
struct PlaceQuery {
    q: String,
    #[serde(default)]
    lang: Option<String>,
}

/// `GET /api/wall/widgets/places?q=Lausanne&lang=fr` — recherche de ville.
async fn places(
    AdminIdentity(_): AdminIdentity,
    Query(query): Query<PlaceQuery>,
) -> ApiResult<Json<Vec<weather::Place>>> {
    let q = query.q.trim();
    if !(2..=80).contains(&q.chars().count()) {
        return Err(ApiError::BadRequest("Type at least two letters.".into()));
    }
    let lang = query
        .lang
        .as_deref()
        .map(|l| l.split(['-', '_']).next().unwrap_or("en").to_ascii_lowercase())
        .filter(|l| l.len() == 2 && l.bytes().all(|b| b.is_ascii_lowercase()))
        .unwrap_or_else(|| "en".into());
    let client = http::client().map_err(ApiError::BadGateway)?;
    let found = weather::search(&client, q, &lang).await.map_err(ApiError::BadGateway)?;
    Ok(Json(found))
}

// ------------------------------------------------------------------- le mur

/// Une section du mur : ses données (fraîches ou un peu anciennes), quand elles
/// ont été lues, et pourquoi la dernière lecture a échoué.
#[derive(Debug, Serialize)]
pub struct Section<T> {
    pub data: Option<T>,
    /// UTC sans suffixe.
    pub updated: Option<String>,
    pub error: Option<String>,
    /// Le widget attend une clé d'API (actions sans clé Finnhub).
    pub needs_key: bool,
    pub source: &'static str,
    pub source_url: &'static str,
}

impl<T> Section<T> {
    fn of(outcome: Outcome<T>, source: &'static str, source_url: &'static str) -> Self {
        Section {
            data: outcome.value,
            updated: outcome.fetched.map(|t| t.format("%Y-%m-%dT%H:%M:%S").to_string()),
            error: outcome.error,
            needs_key: false,
            source,
            source_url,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct WallWidgets {
    pub rotate_seconds: u32,
    pub crypto: Option<Section<coins::CryptoData>>,
    pub weather: Option<Section<weather::WeatherData>>,
    pub stocks: Option<Section<stocks::StocksData>>,
    pub fx: Option<Section<fx::FxData>>,
    pub cve: Option<Section<Vec<cve::CveItem>>>,
}

static CRYPTO: LazyLock<Cache<coins::CryptoData>> = LazyLock::new(Cache::default);
static WEATHER: LazyLock<Cache<weather::WeatherData>> = LazyLock::new(Cache::default);
static STOCKS: LazyLock<Cache<stocks::StocksData>> = LazyLock::new(Cache::default);
static FX: LazyLock<Cache<fx::FxData>> = LazyLock::new(Cache::default);
static NVD: LazyLock<Cache<Vec<cve::CveItem>>> = LazyLock::new(Cache::default);
static KEV: LazyLock<Cache<Vec<cve::CveItem>>> = LazyLock::new(Cache::default);

/// Une empreinte courte d'une clé d'API : le cache change avec elle sans la
/// garder en clair.
fn fingerprint(key: Option<&str>) -> String {
    key.map(|k| hex::encode(&Sha256::digest(k.as_bytes())[..6])).unwrap_or_default()
}

async fn crypto_section(
    client: &reqwest::Client,
    s: &CryptoSettings,
    key: Option<String>,
) -> Section<coins::CryptoData> {
    let cache_key = format!("{}|{}|{}", s.currency, s.coins.join(","), fingerprint(key.as_deref()));
    let outcome = CRYPTO
        .get(&cache_key, CRYPTO_TTL, || coins::fetch(client, &s.coins, &s.currency, key.as_deref()))
        .await;
    Section::of(outcome, coins::SOURCE, coins::SOURCE_URL)
}

async fn weather_section(
    client: &reqwest::Client,
    s: &WeatherSettings,
) -> Section<weather::WeatherData> {
    let imperial = s.units == "imperial";
    let cache_key = format!(
        "{}|{}",
        s.units,
        s.places
            .iter()
            .map(|p| format!("{}:{:.4},{:.4}", p.name, p.latitude, p.longitude))
            .collect::<Vec<_>>()
            .join(";")
    );
    let outcome =
        WEATHER.get(&cache_key, WEATHER_TTL, || weather::fetch(client, &s.places, imperial)).await;
    Section::of(outcome, weather::SOURCE, weather::SOURCE_URL)
}

async fn stocks_section(
    client: &reqwest::Client,
    s: &StocksSettings,
    key: Option<String>,
) -> Section<stocks::StocksData> {
    let Some(key) = key else {
        return Section {
            data: None,
            updated: None,
            error: Some("A free Finnhub API key is needed for stock quotes.".into()),
            needs_key: true,
            source: stocks::SOURCE,
            source_url: stocks::SOURCE_URL,
        };
    };
    let cache_key = format!("{}|{}", s.symbols.join(","), fingerprint(Some(&key)));
    let outcome =
        STOCKS.get(&cache_key, STOCKS_TTL, || stocks::fetch(client, &s.symbols, &key)).await;
    Section::of(outcome, stocks::SOURCE, stocks::SOURCE_URL)
}

async fn fx_section(client: &reqwest::Client, s: &FxSettings) -> Section<fx::FxData> {
    let cache_key = format!("{}|{}", s.base, s.currencies.join(","));
    let outcome = FX.get(&cache_key, FX_TTL, || fx::fetch(client, &s.base, &s.currencies)).await;
    Section::of(outcome, fx::SOURCE, fx::SOURCE_URL)
}

async fn cve_section(client: &reqwest::Client, s: &CveSettings) -> Section<Vec<cve::CveItem>> {
    let empty = || Outcome { value: None, fetched: None, error: None };
    let (nvd, kev) = tokio::join!(
        async {
            match s.source.as_str() {
                "kev" => empty(),
                _ => NVD.get("nvd", CVE_TTL, || cve::fetch_nvd(client)).await,
            }
        },
        async {
            match s.source.as_str() {
                "nvd" => empty(),
                _ => KEV.get("kev", CVE_TTL, || cve::fetch_kev(client)).await,
            }
        },
    );
    let (source, source_url) = match s.source.as_str() {
        "kev" => (cve::SOURCE_KEV, "https://www.cisa.gov/known-exploited-vulnerabilities-catalog"),
        "nvd" => (cve::SOURCE_NVD, cve::SOURCE_URL),
        _ => ("NVD · CISA KEV", cve::SOURCE_URL),
    };
    // Une source qui manque n'éteint pas l'autre ; rien des deux : indisponible.
    let error = nvd.error.clone().or_else(|| kev.error.clone());
    let updated = [nvd.fetched, kev.fetched].into_iter().flatten().min();
    let data = match (nvd.value, kev.value) {
        (None, None) => None,
        (n, k) => Some(cve::combine(n.unwrap_or_default(), k.unwrap_or_default(), s.count)),
    };
    Section::of(Outcome { value: data, fetched: updated, error }, source, source_url)
}

async fn widgets(
    State(state): State<AppState>,
    Identity(_): Identity,
) -> ApiResult<Json<WallWidgets>> {
    let s = load(&state).await?;
    let mut reply = WallWidgets {
        rotate_seconds: s.rotate_seconds,
        crypto: None,
        weather: None,
        stocks: None,
        fx: None,
        cve: None,
    };
    let any = s.crypto.enabled || s.weather.enabled || s.stocks.enabled || s.fx.enabled;
    if !any && !s.cve.enabled {
        return Ok(Json(reply));
    }
    let client = http::client().map_err(ApiError::BadGateway)?;
    let finnhub = if s.stocks.enabled { secret(&state, FINNHUB_KEY).await } else { None };
    let coingecko = if s.crypto.enabled { secret(&state, COINGECKO_KEY).await } else { None };
    let (crypto, weather, stocks, fx, cve) = tokio::join!(
        async {
            if s.crypto.enabled && !s.crypto.coins.is_empty() {
                Some(crypto_section(&client, &s.crypto, coingecko).await)
            } else {
                None
            }
        },
        async {
            if s.weather.enabled && !s.weather.places.is_empty() {
                Some(weather_section(&client, &s.weather).await)
            } else {
                None
            }
        },
        async {
            if s.stocks.enabled && !s.stocks.symbols.is_empty() {
                Some(stocks_section(&client, &s.stocks, finnhub).await)
            } else {
                None
            }
        },
        async {
            if s.fx.enabled && !s.fx.currencies.is_empty() {
                Some(fx_section(&client, &s.fx).await)
            } else {
                None
            }
        },
        async { if s.cve.enabled { Some(cve_section(&client, &s.cve).await) } else { None } },
    );
    reply.crypto = crypto;
    reply.weather = weather;
    reply.stocks = stocks;
    reply.fx = fx;
    reply.cve = cve;
    Ok(Json(reply))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_off_and_valid() {
        let mut s = WidgetSettings::default();
        assert!(validate(&mut s).is_ok());
        assert!(!s.crypto.enabled && !s.weather.enabled && !s.stocks.enabled);
        assert!(!s.fx.enabled && !s.cve.enabled);
    }

    #[test]
    fn settings_are_cleaned_and_checked() {
        let mut s = WidgetSettings::default();
        s.crypto.coins = vec![" Bitcoin ".into(), "bitcoin".into(), "ethereum".into()];
        s.crypto.currency = "EUR".into();
        s.stocks.symbols = vec!["aapl".into(), " msft".into(), "".into()];
        s.fx.base = "chf".into();
        s.fx.currencies = vec!["usd".into(), "CHF".into(), "eur".into(), "USD".into()];
        validate(&mut s).unwrap();
        assert_eq!(s.crypto.coins, ["bitcoin", "ethereum"]);
        assert_eq!(s.crypto.currency, "eur");
        assert_eq!(s.stocks.symbols, ["AAPL", "MSFT"]);
        assert_eq!(s.fx.base, "CHF");
        assert_eq!(s.fx.currencies, ["USD", "EUR"], "la base n'est pas sa propre cotation");

        let bad = |change: fn(&mut WidgetSettings)| {
            let mut s = WidgetSettings::default();
            change(&mut s);
            validate(&mut s).unwrap_err()
        };
        assert!(bad(|s| s.rotate_seconds = 2).contains("seconds"));
        assert!(bad(|s| s.crypto.coins = vec!["a&b".into()]).contains("CoinGecko"));
        assert!(bad(|s| s.crypto.currency = "xyz".into()).contains("currency"));
        assert!(bad(|s| s.weather.enabled = true).contains("place"));
        assert!(bad(|s| s.weather.units = "kelvin".into()).contains("units"));
        assert!(bad(|s| s.stocks.symbols = vec!["A B".into()]).contains("ticker"));
        assert!(bad(|s| s.fx.base = "XXX".into()).contains("base"));
        assert!(bad(|s| s.cve.source = "rss".into()).contains("source"));
        assert!(bad(|s| s.cve.count = 0).contains("between"));
        assert!(
            bad(|s| s.weather.places = vec![weather::Place {
                name: "Nowhere".into(),
                region: String::new(),
                country_code: String::new(),
                latitude: 120.0,
                longitude: 0.0,
            }])
            .contains("coordinates")
        );
    }

    #[test]
    fn the_api_key_never_appears_in_the_cache_key() {
        let print = fingerprint(Some("secret-key-123"));
        assert_eq!(print.len(), 12);
        assert!(!print.contains("secret"));
        assert_eq!(fingerprint(None), "");
    }
}
