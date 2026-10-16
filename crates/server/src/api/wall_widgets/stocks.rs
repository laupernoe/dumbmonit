//! Actions : dernier cours et variation du jour, par Finnhub.
//!
//! Aucune source de cours sans clé n'est à la fois fiable et autorisée (Stooq
//! demande désormais une clé, Yahoo interdit l'usage de son API non
//! documentée) : le widget demande une clé Finnhub gratuite, gardée chiffrée
//! comme les autres secrets et jamais renvoyée. L'offre gratuite couvre les
//! titres cotés aux États-Unis, à raison de 60 appels par minute ; un appel par
//! titre, mis en cache cinq minutes.

use serde::{Deserialize, Serialize};

use super::http;

pub const SOURCE: &str = "Finnhub";
pub const SOURCE_URL: &str = "https://finnhub.io";
const BASE: &str = "https://finnhub.io/api/v1/quote";
const MAX_BYTES: usize = 64 * 1024;

/// `AAPL`, `BRK.B`, `^GSPC`… en capitales.
pub fn valid_symbol(symbol: &str) -> bool {
    (1..=20).contains(&symbol.len())
        && symbol
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b".-:^=".contains(&b))
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Quote {
    pub symbol: String,
    pub price: f64,
    /// Variation depuis la clôture précédente, en valeur et en pour cent.
    pub change: Option<f64>,
    pub change_pct: Option<f64>,
    /// Heure du cours, UTC sans suffixe.
    pub at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StocksData {
    pub quotes: Vec<Quote>,
    /// Les symboles que la source ne connaît pas.
    pub unknown: Vec<String>,
}

#[derive(Deserialize)]
struct Answer {
    c: Option<f64>,
    d: Option<f64>,
    dp: Option<f64>,
    t: Option<i64>,
}

/// Un cours, ou `None` quand Finnhub ne connaît pas le symbole (tout à zéro).
pub fn parse(symbol: &str, body: &[u8]) -> Result<Option<Quote>, String> {
    let answer: Answer = http::json(body)?;
    let price = answer.c.unwrap_or(0.0);
    let at = answer.t.unwrap_or(0);
    if price == 0.0 && at == 0 {
        return Ok(None);
    }
    let at = chrono::DateTime::from_timestamp(at, 0)
        .filter(|_| at > 0)
        .map(|t| t.format("%Y-%m-%dT%H:%M:%S").to_string());
    Ok(Some(Quote {
        symbol: symbol.to_string(),
        price,
        change: answer.d,
        change_pct: answer.dp,
        at,
    }))
}

pub async fn fetch(
    client: &reqwest::Client,
    symbols: &[String],
    key: &str,
) -> Result<StocksData, String> {
    let mut tasks = tokio::task::JoinSet::new();
    for (index, symbol) in symbols.iter().enumerate() {
        let request =
            client.get(BASE).query(&[("symbol", symbol.as_str())]).header("X-Finnhub-Token", key);
        let symbol = symbol.clone();
        tasks.spawn(async move {
            let result = match http::bytes(request, MAX_BYTES).await {
                Ok(body) => parse(&symbol, &body),
                Err(error) => Err(error),
            };
            (index, symbol, result)
        });
    }
    let mut results = Vec::new();
    while let Some(done) = tasks.join_next().await {
        if let Ok(done) = done {
            results.push(done);
        }
    }
    results.sort_by_key(|(index, ..)| *index);
    let mut data = StocksData { quotes: Vec::new(), unknown: Vec::new() };
    let mut first_error = None;
    for (_, symbol, result) in results {
        match result {
            Ok(Some(quote)) => data.quotes.push(quote),
            Ok(None) => data.unknown.push(symbol),
            Err(error) => {
                first_error.get_or_insert(error);
            }
        }
    }
    match first_error {
        // Rien de lu : la clé est refusée ou la source est tombée.
        Some(error) if data.quotes.is_empty() => Err(error),
        _ => Ok(data),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_quote() {
        let body = br#"{"c":261.74,"d":-1.53,"dp":-0.5811,"h":263.31,"l":260.68,"o":261.07,"pc":263.27,"t":1760644800}"#;
        let quote = parse("AAPL", body).unwrap().unwrap();
        assert_eq!(quote.price, 261.74);
        assert_eq!(quote.change, Some(-1.53));
        assert_eq!(quote.change_pct, Some(-0.5811));
        assert_eq!(quote.at.as_deref(), Some("2025-10-16T20:00:00"));
    }

    #[test]
    fn an_unknown_symbol_is_all_zeros() {
        let body = br#"{"c":0,"d":null,"dp":null,"h":0,"l":0,"o":0,"pc":0,"t":0}"#;
        assert_eq!(parse("NOPE", body).unwrap(), None);
        assert!(parse("X", br#"{"error":"Please use an API key."}"#).unwrap().is_none());
        assert!(parse("X", b"<html>").is_err());
    }

    #[test]
    fn symbols_are_checked() {
        for ok in ["AAPL", "BRK.B", "^GSPC", "BINANCE:BTCUSDT", "MC.PA"] {
            assert!(valid_symbol(ok), "{ok}");
        }
        for bad in ["", "aapl", "A B", "A&token=x", "AAAAAAAAAAAAAAAAAAAAA"] {
            assert!(!valid_symbol(bad), "{bad}");
        }
    }
}
