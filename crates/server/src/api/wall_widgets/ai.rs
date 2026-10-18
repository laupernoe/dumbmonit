//! Consommation d'IA : Claude (Anthropic) et OpenAI, lue par les API
//! d'administration d'organisation, avec une clé admin par fournisseur.
//!
//! Ce que les API permettent vraiment (rien n'est inventé ici) :
//!
//! - **Anthropic** (`/v1/organizations/usage_report/messages` et
//!   `/v1/organizations/cost_report`, en-tête `anthropic-version: 2023-06-01`,
//!   clé `sk-ant-admin…` réservée aux organisations) : le coût exact en USD
//!   (en centimes, chaîne décimale) mais seulement par workspace ou par ligne
//!   de facturation, jamais par personne ; les jetons se regroupent par compte
//!   (`account_id`) ou par clé d'API (`api_key_id`). Les API ne donnent que des
//!   identifiants, pas de noms : le mappage vers un nom est saisi dans les
//!   réglages. Le coût par personne n'est donc pas lu : seuls les jetons le sont.
//! - **OpenAI** (`/v1/organization/usage/completions` et
//!   `/v1/organization/costs`, clé admin en `Bearer`) : le coût se regroupe par
//!   projet ou ligne de facturation, jamais par utilisateur ni par clé ; les
//!   jetons des requêtes de texte (Chat Completions, Responses) par
//!   `user_id`, `api_key_id`, projet ou modèle. `user_id` est vide pour le
//!   trafic par clé d'API : les noms viennent aussi d'un mappage saisi.
//! - Les abonnements grand public (Claude Pro/Max, ChatGPT Plus) et leurs
//!   limites hebdomadaires n'ont aucune API officielle : ils n'y figurent pas.
//!
//! Les deux API paginent par jeton `page` / `next_page` ; les pages sont suivies
//! jusqu'à [`MAX_PAGES`]. Les périodes tiennent en 31 seaux journaliers.

use std::collections::BTreeMap;
use std::future::Future;

use chrono::{Datelike, Days, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::http;

pub const CLAUDE_SOURCE: &str = "Anthropic";
pub const CLAUDE_SOURCE_URL: &str =
    "https://platform.claude.com/docs/en/api/admin-api/usage-cost/get-messages-usage-report";
pub const OPENAI_SOURCE: &str = "OpenAI";
pub const OPENAI_SOURCE_URL: &str = "https://platform.openai.com/docs/api-reference/usage";

const CLAUDE_BASE: &str = "https://api.anthropic.com";
const OPENAI_BASE: &str = "https://api.openai.com";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const MAX_BYTES: usize = 8 * 1024 * 1024;
/// Pages suivies au plus par lecture : au-delà, la réponse est suspecte.
const MAX_PAGES: usize = 20;
/// Seaux demandés par page : le maximum des deux API pour des seaux d'un jour.
const BUCKETS: &str = "31";
const MAX_PEOPLE_SHOWN: usize = 12;
pub const MAX_PEOPLE: usize = 30;

pub const PERIODS: [&str; 3] = ["today", "week", "month"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Claude,
    OpenAi,
}

impl Provider {
    pub fn source(self) -> (&'static str, &'static str) {
        match self {
            Provider::Claude => (CLAUDE_SOURCE, CLAUDE_SOURCE_URL),
            Provider::OpenAi => (OPENAI_SOURCE, OPENAI_SOURCE_URL),
        }
    }
}

/// Une personne (ou un service) : l'identifiant que l'API renvoie, et le nom
/// affiché sur le mur.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Person {
    pub id: String,
    pub name: String,
}

/// Nettoie la liste des mappages : espaces, doublons, longueurs.
pub fn clean_people(people: &mut Vec<Person>) -> Result<(), String> {
    for p in people.iter_mut() {
        p.id = p.id.trim().to_string();
        p.name = p.name.trim().chars().take(40).collect();
    }
    people.retain(|p| !p.id.is_empty() || !p.name.is_empty());
    if people.iter().any(|p| p.id.is_empty() || p.name.is_empty()) {
        return Err("Each person needs an identifier and a name.".into());
    }
    if people.iter().any(|p| p.id.len() > 100 || p.id.chars().any(|c| c.is_control())) {
        return Err("A person identifier does not look valid.".into());
    }
    let mut seen = std::collections::HashSet::new();
    people.retain(|p| seen.insert(p.id.clone()));
    if people.len() > MAX_PEOPLE {
        return Err(format!("At most {MAX_PEOPLE} people per provider."));
    }
    Ok(())
}

/// Premier jour de la période (UTC) : aujourd'hui, les 7 derniers jours
/// (aujourd'hui compris) ou le mois en cours.
pub fn period_start(period: &str, today: NaiveDate) -> NaiveDate {
    match period {
        "month" => today.with_day(1).unwrap_or(today),
        "week" => today.checked_sub_days(Days::new(6)).unwrap_or(today),
        _ => today,
    }
}

pub fn today() -> NaiveDate {
    Utc::now().date_naive()
}

// ------------------------------------------------------------------ données

/// Ce qui est lu, avant les noms : mis en cache tel quel, si bien que changer un
/// nom dans les réglages ne rappelle pas l'API.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Raw {
    pub cost_usd: f64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub groups: Vec<Group>,
}

/// Les jetons d'un groupe (compte, clé d'API…) et ses identifiants non vides,
/// du plus parlant au moins parlant.
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub ids: Vec<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PersonUsage {
    /// `None` : l'API ne dit pas qui (usage hors clé et hors compte).
    pub name: Option<String>,
    /// Un nom saisi dans les réglages ; sinon `name` est l'identifiant abrégé.
    pub mapped: bool,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Usage {
    /// `today`, `week` ou `month`.
    pub period: String,
    /// Premier jour (UTC, `AAAA-MM-JJ`).
    pub from: String,
    /// Coût total en dollars américains.
    pub cost_usd: f64,
    /// Jetons d'entrée (cache compris) et de sortie.
    pub input_tokens: u64,
    pub output_tokens: u64,
    /// Les plus gros consommateurs ; `None` hors affichage par personne.
    pub people: Option<Vec<PersonUsage>>,
}

fn shorten(id: &str) -> String {
    let chars: Vec<char> = id.chars().collect();
    if chars.len() <= 14 {
        return id.to_string();
    }
    let head: String = chars[..8].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{head}…{tail}")
}

impl Raw {
    pub fn finish(
        &self,
        period: &str,
        from: NaiveDate,
        people: &[Person],
        by_person: bool,
    ) -> Usage {
        let people = by_person.then(|| {
            let mut sums: BTreeMap<(Option<String>, bool), (u64, u64)> = BTreeMap::new();
            for group in &self.groups {
                let named = group
                    .ids
                    .iter()
                    .find_map(|id| people.iter().find(|p| &p.id == id).map(|p| p.name.clone()));
                let key = match named {
                    Some(name) => (Some(name), true),
                    None => (group.ids.first().map(|id| shorten(id)), false),
                };
                let entry = sums.entry(key).or_default();
                entry.0 += group.input_tokens;
                entry.1 += group.output_tokens;
            }
            let mut rows: Vec<PersonUsage> = sums
                .into_iter()
                .map(|((name, mapped), (input_tokens, output_tokens))| PersonUsage {
                    name,
                    mapped,
                    input_tokens,
                    output_tokens,
                })
                .collect();
            rows.sort_by(|a, b| {
                (b.input_tokens + b.output_tokens).cmp(&(a.input_tokens + a.output_tokens))
            });
            rows.truncate(MAX_PEOPLE_SHOWN);
            rows
        });
        Usage {
            period: period.to_string(),
            from: from.to_string(),
            cost_usd: self.cost_usd,
            input_tokens: self.input_tokens,
            output_tokens: self.output_tokens,
            people,
        }
    }
}

// --------------------------------------------------------------- pagination

/// Suit le jeton `next_page` jusqu'à la dernière page.
async fn paginate<T, F, Fut>(mut fetch: F) -> Result<Vec<T>, String>
where
    F: FnMut(Option<String>) -> Fut,
    Fut: Future<Output = Result<(Vec<T>, Option<String>), String>>,
{
    let mut all = Vec::new();
    let mut cursor = None;
    for _ in 0..MAX_PAGES {
        let (mut items, next) = fetch(cursor.take()).await?;
        all.append(&mut items);
        match next.filter(|n| !n.is_empty()) {
            Some(next) => cursor = Some(next),
            None => return Ok(all),
        }
    }
    Err("the provider returned more pages than expected".into())
}

fn into_groups(groups: BTreeMap<Vec<String>, (u64, u64)>) -> Vec<Group> {
    groups
        .into_iter()
        .map(|(ids, (input_tokens, output_tokens))| Group { ids, input_tokens, output_tokens })
        .collect()
}

// ---------------------------------------------------------------- Anthropic

#[derive(Deserialize)]
struct ClaudePage<R> {
    #[serde(default = "Vec::new")]
    data: Vec<ClaudeBucket<R>>,
    #[serde(default)]
    has_more: bool,
    #[serde(default)]
    next_page: Option<String>,
}

#[derive(Deserialize)]
struct ClaudeBucket<R> {
    #[serde(default = "Vec::new")]
    results: Vec<R>,
}

#[derive(Deserialize, Default)]
struct CacheCreation {
    #[serde(default)]
    ephemeral_1h_input_tokens: u64,
    #[serde(default)]
    ephemeral_5m_input_tokens: u64,
}

#[derive(Deserialize)]
struct ClaudeUsageRow {
    #[serde(default)]
    account_id: Option<String>,
    #[serde(default)]
    api_key_id: Option<String>,
    #[serde(default)]
    uncached_input_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: u64,
    #[serde(default)]
    cache_creation: Option<CacheCreation>,
    #[serde(default)]
    output_tokens: u64,
}

#[derive(Deserialize)]
struct ClaudeCostRow {
    /// En plus petite unité de la devise (centimes), chaîne décimale.
    amount: String,
    #[serde(default)]
    currency: String,
}

async fn claude_pages<R: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    url: &str,
    key: &str,
    query: &[(&str, String)],
) -> Result<Vec<R>, String> {
    paginate(|page| async move {
        let mut query = query.to_vec();
        query.push(("limit", BUCKETS.into()));
        if let Some(page) = page {
            query.push(("page", page));
        }
        let request = client
            .get(url)
            .header("x-api-key", key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .query(&query);
        let body = http::bytes(request, MAX_BYTES).await?;
        let page: ClaudePage<R> = http::json(&body)?;
        let next = page.has_more.then_some(page.next_page).flatten();
        Ok((page.data.into_iter().flat_map(|bucket| bucket.results).collect(), next))
    })
    .await
}

pub async fn fetch_claude(
    client: &reqwest::Client,
    base: &str,
    key: &str,
    from: NaiveDate,
    by_person: bool,
) -> Result<Raw, String> {
    let start = format!("{from}T00:00:00Z");
    let mut usage_query = vec![("starting_at", start.clone()), ("bucket_width", "1d".to_string())];
    if by_person {
        usage_query.push(("group_by[]", "account_id".into()));
        usage_query.push(("group_by[]", "api_key_id".into()));
    }
    let cost_query = vec![("starting_at", start), ("bucket_width", "1d".to_string())];
    let usage_url = format!("{base}/v1/organizations/usage_report/messages");
    let cost_url = format!("{base}/v1/organizations/cost_report");
    let (usage, costs) = tokio::try_join!(
        claude_pages::<ClaudeUsageRow>(client, &usage_url, key, &usage_query),
        claude_pages::<ClaudeCostRow>(client, &cost_url, key, &cost_query),
    )?;

    let mut raw = Raw::default();
    for row in costs.iter().filter(|r| r.currency.eq_ignore_ascii_case("usd")) {
        raw.cost_usd += row.amount.trim().parse::<f64>().unwrap_or(0.0) / 100.0;
    }
    let mut groups: BTreeMap<Vec<String>, (u64, u64)> = BTreeMap::new();
    for row in usage {
        let cache = row.cache_creation.unwrap_or_default();
        let input = row.uncached_input_tokens
            + row.cache_read_input_tokens
            + cache.ephemeral_1h_input_tokens
            + cache.ephemeral_5m_input_tokens;
        raw.input_tokens += input;
        raw.output_tokens += row.output_tokens;
        if by_person {
            let ids: Vec<String> = [row.account_id, row.api_key_id].into_iter().flatten().collect();
            let entry = groups.entry(ids).or_default();
            entry.0 += input;
            entry.1 += row.output_tokens;
        }
    }
    raw.groups = into_groups(groups);
    Ok(raw)
}

// ------------------------------------------------------------------- OpenAI

#[derive(Deserialize)]
struct OpenAiPage<R> {
    #[serde(default = "Vec::new")]
    data: Vec<OpenAiBucket<R>>,
    #[serde(default)]
    has_more: bool,
    #[serde(default)]
    next_page: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiBucket<R> {
    /// `results` dans les exemples de l'API, `result` dans son schéma.
    #[serde(default = "Vec::new", alias = "result")]
    results: Vec<R>,
}

#[derive(Deserialize)]
struct OpenAiUsageRow {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    user_id: Option<String>,
    #[serde(default)]
    api_key_id: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiCostRow {
    amount: Option<OpenAiAmount>,
}

#[derive(Deserialize)]
struct OpenAiAmount {
    #[serde(default)]
    value: f64,
    #[serde(default)]
    currency: String,
}

async fn openai_pages<R: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    url: &str,
    key: &str,
    query: &[(&str, String)],
) -> Result<Vec<R>, String> {
    paginate(|page| async move {
        let mut query = query.to_vec();
        query.push(("limit", BUCKETS.into()));
        if let Some(page) = page {
            query.push(("page", page));
        }
        let request = client.get(url).bearer_auth(key).query(&query);
        let body = http::bytes(request, MAX_BYTES).await?;
        let page: OpenAiPage<R> = http::json(&body)?;
        let next = page.has_more.then_some(page.next_page).flatten();
        Ok((page.data.into_iter().flat_map(|bucket| bucket.results).collect(), next))
    })
    .await
}

pub async fn fetch_openai(
    client: &reqwest::Client,
    base: &str,
    key: &str,
    from: NaiveDate,
    by_person: bool,
) -> Result<Raw, String> {
    let start = from.and_hms_opt(0, 0, 0).map(|t| t.and_utc().timestamp()).unwrap_or(0);
    let mut usage_query = vec![("start_time", start.to_string()), ("bucket_width", "1d".into())];
    if by_person {
        usage_query.push(("group_by", "user_id".into()));
        usage_query.push(("group_by", "api_key_id".into()));
    }
    let cost_query = vec![("start_time", start.to_string()), ("bucket_width", "1d".to_string())];
    let usage_url = format!("{base}/v1/organization/usage/completions");
    let cost_url = format!("{base}/v1/organization/costs");
    let (usage, costs) = tokio::try_join!(
        openai_pages::<OpenAiUsageRow>(client, &usage_url, key, &usage_query),
        openai_pages::<OpenAiCostRow>(client, &cost_url, key, &cost_query),
    )?;

    let mut raw = Raw::default();
    for amount in costs.into_iter().filter_map(|r| r.amount) {
        if amount.currency.eq_ignore_ascii_case("usd") {
            raw.cost_usd += amount.value;
        }
    }
    let mut groups: BTreeMap<Vec<String>, (u64, u64)> = BTreeMap::new();
    for row in usage {
        raw.input_tokens += row.input_tokens;
        raw.output_tokens += row.output_tokens;
        if by_person {
            let ids: Vec<String> = [row.user_id, row.api_key_id].into_iter().flatten().collect();
            let entry = groups.entry(ids).or_default();
            entry.0 += row.input_tokens;
            entry.1 += row.output_tokens;
        }
    }
    raw.groups = into_groups(groups);
    Ok(raw)
}

/// Lit le fournisseur sur son adresse publique.
pub async fn fetch(
    provider: Provider,
    client: &reqwest::Client,
    key: &str,
    from: NaiveDate,
    by_person: bool,
) -> Result<Raw, String> {
    match provider {
        Provider::Claude => fetch_claude(client, CLAUDE_BASE, key, from, by_person).await,
        Provider::OpenAi => fetch_openai(client, OPENAI_BASE, key, from, by_person).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::Query;
    use axum::http::{HeaderMap, Uri};
    use axum::{Json, Router};
    use serde_json::{Value, json};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    type Seen = Arc<Mutex<Vec<(String, HashMap<String, String>, HeaderMap)>>>;

    /// Un faux fournisseur sur le port local : sert les fixtures, note les appels.
    async fn serve(routes: Vec<(&'static str, Vec<Value>)>) -> (String, Seen) {
        let seen: Seen = Arc::default();
        // Un seul gestionnaire, qui choisit les pages d'après le chemin demandé.
        let handler = {
            let seen = seen.clone();
            move |uri: Uri, headers: HeaderMap, Query(q): Query<HashMap<String, String>>| {
                let routes = routes.clone();
                let seen = seen.clone();
                async move {
                    let path = uri.path().to_string();
                    let pages = &routes.iter().find(|(p, _)| *p == path).expect("chemin connu").1;
                    let index = match q.get("page").map(String::as_str) {
                        None => 0,
                        Some("page_2") => 1,
                        Some(other) => panic!("unexpected page {other}"),
                    };
                    seen.lock().unwrap().push((path, q, headers));
                    Json(pages[index].clone())
                }
            }
        };
        let app = Router::new().fallback(handler);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        (base, seen)
    }

    fn day() -> NaiveDate {
        NaiveDate::from_ymd_opt(2025, 8, 1).unwrap()
    }

    // Structures exactes de la documentation Anthropic (valeurs adaptées).
    fn claude_usage_pages() -> Vec<Value> {
        let row = |account: Value, key: Value, uncached, read, c1h, c5m, out| {
            json!({
                "account_id": account, "api_key_id": key,
                "cache_creation": {
                    "ephemeral_1h_input_tokens": c1h, "ephemeral_5m_input_tokens": c5m
                },
                "cache_read_input_tokens": read, "context_window": null,
                "inference_geo": null, "model": null, "output_tokens": out,
                "server_tool_use": { "web_search_requests": 0 },
                "service_account_id": null, "service_tier": null, "speed": null,
                "uncached_input_tokens": uncached, "workspace_id": null
            })
        };
        vec![
            json!({
                "data": [{
                    "starting_at": "2025-08-01T00:00:00Z", "ending_at": "2025-08-02T00:00:00Z",
                    "results": [
                        row(json!("user_01WCz1FkmYMm4gnmykNKUu3Q"), Value::Null, 1500, 200, 0, 100, 500),
                        row(Value::Null, json!("apikey_01Rj2N8SVvo6BePZj99NhmiT"), 4000, 0, 0, 0, 900),
                    ]
                }],
                "has_more": true,
                "next_page": "page_2"
            }),
            json!({
                "data": [
                    { "starting_at": "2025-08-02T00:00:00Z", "ending_at": "2025-08-03T00:00:00Z",
                      "results": [row(json!("user_01WCz1FkmYMm4gnmykNKUu3Q"), Value::Null, 500, 0, 0, 0, 100)] },
                    { "starting_at": "2025-08-03T00:00:00Z", "ending_at": "2025-08-04T00:00:00Z",
                      "results": [] }
                ],
                "has_more": false,
                "next_page": null
            }),
        ]
    }

    fn claude_cost_pages() -> Vec<Value> {
        let result = |amount: &str, description: &str| {
            json!({
                "amount": amount, "context_window": null, "cost_type": null, "currency": "USD",
                "description": description, "inference_geo": null, "model": null,
                "service_tier": null, "speed": null, "token_type": null, "workspace_id": null
            })
        };
        vec![
            json!({
                "data": [{
                    "starting_at": "2025-08-01T00:00:00Z", "ending_at": "2025-08-02T00:00:00Z",
                    "results": [result("123.78912", "Claude Opus 5.5 Usage - Input Tokens"), result("200", "web")]
                }],
                "has_more": true, "next_page": "page_2"
            }),
            json!({
                "data": [{
                    "starting_at": "2025-08-02T00:00:00Z", "ending_at": "2025-08-03T00:00:00Z",
                    "results": [result("76.21088", "x")]
                }],
                "has_more": false, "next_page": null
            }),
        ]
    }

    #[tokio::test]
    async fn claude_reads_cost_in_cents_tokens_and_follows_pages() {
        let (base, seen) = serve(vec![
            ("/v1/organizations/usage_report/messages", claude_usage_pages()),
            ("/v1/organizations/cost_report", claude_cost_pages()),
        ])
        .await;
        let client = http::client().unwrap();
        let raw = fetch_claude(&client, &base, "sk-ant-admin-test", day(), true).await.unwrap();

        // 123.78912 + 200 + 76.21088 centimes = 4,00 $.
        assert!((raw.cost_usd - 4.0).abs() < 1e-9, "{}", raw.cost_usd);
        // Entrée = sans cache + lu du cache + écrit au cache.
        assert_eq!(raw.input_tokens, 1500 + 200 + 100 + 4000 + 500);
        assert_eq!(raw.output_tokens, 500 + 900 + 100);

        let calls = seen.lock().unwrap();
        // Deux pages de chaque rapport.
        assert_eq!(calls.len(), 4);
        let (_, query, headers) = calls
            .iter()
            .find(|(p, q, _)| p.ends_with("messages") && !q.contains_key("page"))
            .unwrap();
        assert_eq!(headers["x-api-key"], "sk-ant-admin-test");
        assert_eq!(headers["anthropic-version"], "2023-06-01");
        assert_eq!(query["starting_at"], "2025-08-01T00:00:00Z");
        assert_eq!(query["bucket_width"], "1d");
        assert_eq!(query["limit"], "31");
        assert!(calls.iter().any(|(path, q, _)| {
            path.ends_with("cost_report") && q.get("page").is_some_and(|page| page == "page_2")
        }));
        drop(calls);

        let people = [Person { id: "user_01WCz1FkmYMm4gnmykNKUu3Q".into(), name: "Alice".into() }];
        let usage = raw.finish("week", day(), &people, true);
        let rows = usage.people.unwrap();
        assert_eq!(rows.len(), 2);
        let alice = rows.iter().find(|r| r.name.as_deref() == Some("Alice")).unwrap();
        assert!(alice.mapped);
        assert_eq!((alice.input_tokens, alice.output_tokens), (1500 + 200 + 100 + 500, 500 + 100));
        let key = rows.iter().find(|r| !r.mapped).unwrap();
        assert_eq!(key.name.as_deref(), Some("apikey_0…hmiT"));
        assert_eq!(key.output_tokens, 900);
    }

    #[tokio::test]
    async fn claude_without_grouping_asks_for_totals_only() {
        let (base, seen) = serve(vec![
            ("/v1/organizations/usage_report/messages", vec![claude_usage_pages().remove(1)]),
            ("/v1/organizations/cost_report", vec![claude_cost_pages().remove(1)]),
        ])
        .await;
        let client = http::client().unwrap();
        let raw = fetch_claude(&client, &base, "k", day(), false).await.unwrap();
        assert_eq!(raw.input_tokens, 500);
        assert!(raw.groups.is_empty());
        assert!(seen.lock().unwrap().iter().all(|(_, q, _)| !q.contains_key("group_by[]")));
        assert!(raw.finish("today", day(), &[], false).people.is_none());
    }

    // Structures exactes de la documentation OpenAI (`results`, `next_page`).
    fn openai_usage_pages() -> Vec<Value> {
        let row = |user: Value, key: Value, input, output| {
            json!({
                "object": "organization.usage.completions.result",
                "input_tokens": input, "output_tokens": output, "input_cached_tokens": 800,
                "input_audio_tokens": 0, "output_audio_tokens": 0, "num_model_requests": 5,
                "project_id": null, "user_id": user, "api_key_id": key, "model": null, "batch": null
            })
        };
        vec![
            json!({
                "object": "page",
                "data": [{
                    "object": "bucket", "start_time": 1754006400, "end_time": 1754092800,
                    "results": [
                        row(json!("user-abc"), Value::Null, 1000, 500),
                        row(Value::Null, json!("key_xyz"), 2000, 300),
                    ]
                }],
                "has_more": true, "next_page": "page_2"
            }),
            json!({
                "object": "page",
                "data": [{
                    "object": "bucket", "start_time": 1754092800, "end_time": 1754179200,
                    "results": [row(json!("user-abc"), Value::Null, 10, 5)]
                }],
                "has_more": false, "next_page": null
            }),
        ]
    }

    fn openai_cost_pages() -> Vec<Value> {
        let result = |value: f64, currency: &str| {
            json!({
                "object": "organization.costs.result",
                "amount": { "value": value, "currency": currency },
                "line_item": null, "project_id": null
            })
        };
        vec![json!({
            "object": "page",
            "data": [{
                "object": "bucket", "start_time": 1754006400, "end_time": 1754092800,
                "results": [result(0.06, "usd"), result(1.5, "usd"), result(99.0, "eur")]
            }],
            "has_more": false, "next_page": null
        })]
    }

    #[tokio::test]
    async fn openai_reads_cost_tokens_and_follows_pages() {
        let (base, seen) = serve(vec![
            ("/v1/organization/usage/completions", openai_usage_pages()),
            ("/v1/organization/costs", openai_cost_pages()),
        ])
        .await;
        let client = http::client().unwrap();
        let raw = fetch_openai(&client, &base, "sk-admin-test", day(), true).await.unwrap();

        assert!((raw.cost_usd - 1.56).abs() < 1e-9, "seuls les dollars comptent");
        assert_eq!(raw.input_tokens, 3010);
        assert_eq!(raw.output_tokens, 805);

        let calls = seen.lock().unwrap();
        assert_eq!(calls.len(), 3);
        let (_, query, headers) = calls
            .iter()
            .find(|(p, q, _)| p.ends_with("completions") && !q.contains_key("page"))
            .unwrap();
        assert_eq!(headers["authorization"], "Bearer sk-admin-test");
        assert_eq!(query["start_time"], "1754006400");
        assert_eq!(query["limit"], "31");
        drop(calls);

        let people = [Person { id: "key_xyz".into(), name: "CI".into() }];
        let rows = raw.finish("month", day(), &people, true).people.unwrap();
        // CI (2000 + 300 jetons) passe avant user-abc (1010 + 505), non mappé.
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name.as_deref(), Some("CI"));
        assert!(rows[0].mapped);
        assert_eq!(rows[1].name.as_deref(), Some("user-abc"));
        assert!(!rows[1].mapped);
    }

    #[test]
    fn the_schema_spelling_result_is_read_too() {
        let body = br#"{"object":"page","data":[{"object":"bucket","start_time":1,"end_time":2,
            "result":[{"object":"organization.usage.completions.result","input_tokens":7,
            "output_tokens":3,"num_model_requests":1}]}],"has_more":false,"next_page":null}"#;
        let page: OpenAiPage<OpenAiUsageRow> = http::json(body).unwrap();
        assert_eq!(page.data[0].results[0].input_tokens, 7);
    }

    #[tokio::test]
    async fn pagination_stops_and_gives_up() {
        let pages = paginate(|page| async move {
            Ok(match page.as_deref() {
                None => (vec![1], Some("a".to_string())),
                Some("a") => (vec![2], Some("b".to_string())),
                _ => (vec![3], None),
            })
        })
        .await
        .unwrap();
        assert_eq!(pages, [1, 2, 3]);

        let endless: Result<Vec<u8>, String> =
            paginate(|_| async { Ok((vec![0], Some("again".to_string()))) }).await;
        assert!(endless.unwrap_err().contains("more pages"));
        let failing: Result<Vec<u8>, String> =
            paginate(|_| async { Err("boom".to_string()) }).await;
        assert_eq!(failing.unwrap_err(), "boom");
    }

    #[test]
    fn periods_and_names_are_cleaned() {
        let wed = NaiveDate::from_ymd_opt(2026, 10, 14).unwrap();
        assert_eq!(period_start("today", wed), wed);
        assert_eq!(period_start("week", wed).to_string(), "2026-10-08");
        assert_eq!(period_start("month", wed).to_string(), "2026-10-01");
        let new_year = NaiveDate::from_ymd_opt(2026, 1, 3).unwrap();
        assert_eq!(period_start("week", new_year).to_string(), "2025-12-28");

        let mut people = vec![
            Person { id: " user_1 ".into(), name: " Alice ".into() },
            Person { id: "user_1".into(), name: "Doublon".into() },
            Person { id: String::new(), name: String::new() },
        ];
        clean_people(&mut people).unwrap();
        assert_eq!(people, [Person { id: "user_1".into(), name: "Alice".into() }]);
        let mut half = vec![Person { id: "x".into(), name: String::new() }];
        assert!(clean_people(&mut half).is_err());
        assert_eq!(shorten("short"), "short");
        assert_eq!(shorten("apikey_01Rj2N8SVvo6BePZj99NhmiT"), "apikey_0…hmiT");
    }
}
