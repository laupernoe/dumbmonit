//! Cryptomonnaies : cours et variation sur 24 h, par l'API publique de CoinGecko.
//!
//! Un seul appel pour toutes les monnaies choisies (`/coins/markets`), mis en
//! cache une minute : un appel par minute au plus, quel que soit le nombre
//! d'écrans. Sans clé, CoinGecko partage une réserve d'appels par adresse IP
//! (de l'ordre de 10 à 30 par minute) ; une clé « Demo » gratuite, facultative,
//! passe par l'en-tête `x-cg-demo-api-key`.

use serde::{Deserialize, Serialize};

use super::http;

pub const SOURCE: &str = "CoinGecko";
pub const SOURCE_URL: &str = "https://www.coingecko.com";
const BASE: &str = "https://api.coingecko.com/api/v3/coins/markets";
const MAX_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Preset {
    pub id: &'static str,
    pub symbol: &'static str,
    pub name: &'static str,
}

const fn preset(id: &'static str, symbol: &'static str, name: &'static str) -> Preset {
    Preset { id, symbol, name }
}

/// Les principales monnaies proposées (identifiants CoinGecko).
pub const PRESETS: &[Preset] = &[
    preset("bitcoin", "BTC", "Bitcoin"),
    preset("ethereum", "ETH", "Ethereum"),
    preset("solana", "SOL", "Solana"),
    preset("ripple", "XRP", "XRP"),
    preset("binancecoin", "BNB", "BNB"),
    preset("cardano", "ADA", "Cardano"),
    preset("dogecoin", "DOGE", "Dogecoin"),
    preset("tron", "TRX", "TRON"),
    preset("the-open-network", "TON", "Toncoin"),
    preset("avalanche-2", "AVAX", "Avalanche"),
    preset("polkadot", "DOT", "Polkadot"),
    preset("chainlink", "LINK", "Chainlink"),
    preset("litecoin", "LTC", "Litecoin"),
    preset("stellar", "XLM", "Stellar"),
    preset("monero", "XMR", "Monero"),
    preset("tether", "USDT", "Tether"),
    preset("usd-coin", "USDC", "USDC"),
];

/// Devises d'affichage proposées (codes CoinGecko, en minuscules).
pub const CURRENCIES: &[&str] = &[
    "usd", "eur", "gbp", "chf", "jpy", "cad", "aud", "cny", "inr", "brl", "krw", "sek", "nok",
    "pln", "btc",
];

/// Un identifiant CoinGecko plausible (`bitcoin`, `avalanche-2`…).
pub fn valid_id(id: &str) -> bool {
    (1..=60).contains(&id.len())
        && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Coin {
    pub id: String,
    /// En capitales : `BTC`.
    pub symbol: String,
    pub name: String,
    pub price: f64,
    /// Variation sur 24 h, en pour cent.
    pub change_24h: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CryptoData {
    /// En capitales : `EUR`.
    pub currency: String,
    pub coins: Vec<Coin>,
}

#[derive(Deserialize)]
struct Market {
    id: String,
    symbol: String,
    name: String,
    current_price: Option<f64>,
    price_change_percentage_24h: Option<f64>,
}

pub fn url(ids: &[String], currency: &str) -> String {
    format!(
        "{BASE}?vs_currency={currency}&ids={}&price_change_percentage=24h&sparkline=false",
        ids.join(",")
    )
}

/// Lit la réponse de `/coins/markets`, dans l'ordre choisi par l'administrateur.
pub fn parse(body: &[u8], order: &[String], currency: &str) -> Result<CryptoData, String> {
    let markets: Vec<Market> = http::json(body)?;
    let mut coins: Vec<Coin> = markets
        .into_iter()
        .filter_map(|m| {
            Some(Coin {
                price: m.current_price?,
                id: m.id,
                symbol: m.symbol.to_uppercase(),
                name: m.name,
                change_24h: m.price_change_percentage_24h,
            })
        })
        .collect();
    coins.sort_by_key(|coin| order.iter().position(|id| *id == coin.id).unwrap_or(usize::MAX));
    if coins.is_empty() {
        return Err("no price for the chosen coins".into());
    }
    Ok(CryptoData { currency: currency.to_uppercase(), coins })
}

pub async fn fetch(
    client: &reqwest::Client,
    ids: &[String],
    currency: &str,
    key: Option<&str>,
) -> Result<CryptoData, String> {
    let mut request = client.get(url(ids, currency));
    if let Some(key) = key {
        request = request.header("x-cg-demo-api-key", key);
    }
    parse(&http::bytes(request, MAX_BYTES).await?, ids, currency)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"[
      {"id":"ethereum","symbol":"eth","name":"Ethereum","image":"https://x/eth.png","current_price":3550.12,
       "market_cap":1,"price_change_24h":-20.5,"price_change_percentage_24h":-0.57},
      {"id":"bitcoin","symbol":"btc","name":"Bitcoin","current_price":73119,"price_change_percentage_24h":1.03761},
      {"id":"dead-coin","symbol":"ded","name":"Dead","current_price":null,"price_change_percentage_24h":null}
    ]"#;

    #[test]
    fn reads_markets_in_the_chosen_order() {
        let order = vec!["bitcoin".to_string(), "ethereum".to_string(), "dead-coin".to_string()];
        let data = parse(FIXTURE.as_bytes(), &order, "eur").unwrap();
        assert_eq!(data.currency, "EUR");
        let symbols: Vec<&str> = data.coins.iter().map(|c| c.symbol.as_str()).collect();
        assert_eq!(symbols, ["BTC", "ETH"]);
        assert_eq!(data.coins[0].price, 73119.0);
        assert_eq!(data.coins[1].change_24h, Some(-0.57));
    }

    #[test]
    fn errors_are_named() {
        assert!(parse(b"[]", &[], "usd").is_err());
        assert!(parse(br#"{"status":{"error_code":429}}"#, &[], "usd").is_err());
    }

    #[test]
    fn builds_one_call_for_every_coin() {
        let ids = vec!["bitcoin".to_string(), "avalanche-2".to_string()];
        assert_eq!(
            url(&ids, "chf"),
            "https://api.coingecko.com/api/v3/coins/markets?vs_currency=chf\
             &ids=bitcoin,avalanche-2&price_change_percentage=24h&sparkline=false"
        );
        assert!(valid_id("avalanche-2"));
        assert!(!valid_id("Bitcoin"));
        assert!(!valid_id("a&b=c"));
        assert!(PRESETS.iter().all(|p| valid_id(p.id)));
    }
}
