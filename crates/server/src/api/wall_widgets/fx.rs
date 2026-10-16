//! Devises : taux du jour des principales monnaies face à une devise de base,
//! par Frankfurter (API ouverte, sans clé ni quota ; taux de référence
//! quotidiens des banques centrales, BCE en tête).
//!
//! Un appel sur les dix derniers jours (`/v2/rates?from=`) donne à la fois le
//! dernier taux et le précédent, donc la variation. Mis en cache une heure : les
//! taux ne changent qu'une fois par jour ouvré.

use std::collections::BTreeMap;

use chrono::{Days, NaiveDate};
use serde::{Deserialize, Serialize};

use super::http;

pub const SOURCE: &str = "Frankfurter";
pub const SOURCE_URL: &str = "https://frankfurter.dev";
const BASE: &str = "https://api.frankfurter.dev/v2/rates";
const MAX_BYTES: usize = 1024 * 1024;

/// Devises proposées (ISO 4217).
pub const CURRENCIES: &[&str] = &[
    "USD", "EUR", "GBP", "JPY", "CHF", "CNY", "CAD", "AUD", "NZD", "HKD", "SGD", "SEK", "NOK",
    "DKK", "PLN", "CZK", "HUF", "TRY", "INR", "BRL", "MXN", "ZAR", "KRW", "ILS", "THB",
];

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Rate {
    pub code: String,
    /// Une unité de la devise de base vaut `rate` unités de `code`.
    pub rate: f64,
    /// Le taux du jour ouvré précédent.
    pub previous: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FxData {
    pub base: String,
    /// Date des taux (celle de la banque centrale).
    pub date: String,
    pub rates: Vec<Rate>,
}

#[derive(Deserialize)]
struct Row {
    date: String,
    quote: String,
    rate: f64,
}

pub fn url(base: &str, quotes: &[String], today: NaiveDate) -> String {
    let from = today.checked_sub_days(Days::new(10)).unwrap_or(today);
    format!("{BASE}?base={base}&quotes={}&from={}", quotes.join(","), from.format("%Y-%m-%d"))
}

pub fn parse(body: &[u8], base: &str, order: &[String]) -> Result<FxData, String> {
    let rows: Vec<Row> = http::json(body)?;
    // Par devise : ses taux, du plus ancien au plus récent.
    let mut series: BTreeMap<String, Vec<(String, f64)>> = BTreeMap::new();
    for row in rows {
        series.entry(row.quote).or_default().push((row.date, row.rate));
    }
    let mut date = String::new();
    let mut rates = Vec::new();
    for code in order {
        let Some(points) = series.get_mut(code) else { continue };
        points.sort_by(|a, b| a.0.cmp(&b.0));
        let Some((last_date, rate)) = points.last().cloned() else { continue };
        let previous = points.len().checked_sub(2).map(|i| points[i].1);
        if last_date > date {
            date = last_date;
        }
        rates.push(Rate { code: code.clone(), rate, previous });
    }
    if rates.is_empty() {
        return Err("no rate for the chosen currencies".into());
    }
    Ok(FxData { base: base.to_string(), date, rates })
}

pub async fn fetch(
    client: &reqwest::Client,
    base: &str,
    quotes: &[String],
) -> Result<FxData, String> {
    let today = chrono::Utc::now().date_naive();
    let body = http::bytes(client.get(url(base, quotes, today)), MAX_BYTES).await?;
    parse(&body, base, quotes)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"[
      {"date":"2026-10-14","base":"EUR","quote":"CHF","rate":0.93328},
      {"date":"2026-10-14","base":"EUR","quote":"USD","rate":1.12},
      {"date":"2026-10-16","base":"EUR","quote":"CHF","rate":0.92861},
      {"date":"2026-10-15","base":"EUR","quote":"CHF","rate":0.92985},
      {"date":"2026-10-15","base":"EUR","quote":"USD","rate":1.1198},
      {"date":"2026-10-16","base":"EUR","quote":"USD","rate":1.1216}
    ]"#;

    #[test]
    fn latest_and_previous_rates_in_the_chosen_order() {
        let order = vec!["USD".to_string(), "CHF".to_string(), "JPY".to_string()];
        let data = parse(FIXTURE.as_bytes(), "EUR", &order).unwrap();
        assert_eq!(data.date, "2026-10-16");
        assert_eq!(data.rates.len(), 2);
        assert_eq!(
            data.rates[0],
            Rate { code: "USD".into(), rate: 1.1216, previous: Some(1.1198) }
        );
        assert_eq!(data.rates[1].previous, Some(0.92985));
    }

    #[test]
    fn a_single_day_has_no_previous_rate() {
        let body = br#"[{"date":"2026-10-16","base":"EUR","quote":"USD","rate":1.1216}]"#;
        let data = parse(body, "EUR", &["USD".to_string()]).unwrap();
        assert_eq!(data.rates[0].previous, None);
        assert!(parse(b"[]", "EUR", &["USD".to_string()]).is_err());
        assert!(parse(br#"{"status":422,"message":"invalid"}"#, "EUR", &[]).is_err());
    }

    #[test]
    fn asks_for_ten_days() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 16).unwrap();
        assert_eq!(
            url("CHF", &["USD".into(), "EUR".into()], today),
            "https://api.frankfurter.dev/v2/rates?base=CHF&quotes=USD,EUR&from=2026-10-06"
        );
        assert!(CURRENCIES.iter().all(|c| c.len() == 3));
    }
}
