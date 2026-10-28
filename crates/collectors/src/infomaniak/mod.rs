//! Infomaniak, par l'API publique (`https://api.infomaniak.com/1`).
//!
//! Une seule lecture : la liste des produits du compte (`GET /1/product`) :
//! domaines, hébergements, messagerie, sauvegardes… Chaque produit porte une
//! date d'expiration (`expired_at`) et quelques drapeaux (`is_locked`,
//! `has_maintenance`, `has_operation_in_progress`). C'est tout ce que l'API
//! publique dit de l'état d'un produit : il n'y a pas d'état fin (disponibilité,
//! charge) à lire, et ce module n'invente pas de panne. Il sert à ne pas laisser
//! un domaine ou un hébergement expirer sans que personne le voie.
//!
//! L'authentification est un jeton d'API (`Authorization: Bearer`), créé dans le
//! manager Infomaniak avec la portée « Produits » (et « Comptes » pour retrouver
//! l'identifiant du compte). Sans l'option `account_id`, la liste des comptes
//! (`GET /1/account`) est lue d'abord.
//!
//! Les 200 premiers produits sont décrits ; les comptes restent exacts au-delà.
//! L'API n'est lue qu'une page par compte (et par service si `service_names`
//! est renseignée).
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `account_id` | vide | Compte à lire ; vide = tous les comptes visibles du jeton (10 au plus). |
//! | `service_names` | vide | Services à lire (`domain, hosting`) ; vide = tous. |
//! | `region` | vide | Centre de données (`geneva`, `dc3-a`, `winterthur`…) : l'API des produits ne le dit pas ; publié en `infomaniak_region_products` pour la carte. |
//! | `request_timeout_seconds` | `15` | Délai par requête HTTP. |

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};
use serde_json::Value;

use crate::cloud;
use crate::rest::{MAX_NAMED, RestClient, flag, gauge, now_ms, number, text};
use crate::selfhosted::options::tag;

pub const DEFAULT_PORT: u16 = 443;
pub const DEFAULT_ADDRESS: &str = "api.infomaniak.com";
const PREFIX: &str = "infomaniak";
/// Comptes lus au plus, et fenêtre de l'« expiration prochaine » des chiffres clés.
const MAX_ACCOUNTS: usize = 10;
const SOON_SECONDS: f64 = 14.0 * 86_400.0;

#[derive(Default)]
pub struct InfomaniakCollector;

impl InfomaniakCollector {
    pub fn new() -> Self {
        Self
    }
}

/// Lit un document de l'API ; `hint` complète le message quand le jeton est refusé.
async fn read(client: &RestClient, path: &str, hint: &str) -> Result<Value, ProbeError> {
    let (status, body) = client.get(path).await?;
    match status.as_u16() {
        200..=299 => {}
        401 | 403 => {
            return Err(ProbeError::Auth(format!(
                "Infomaniak refused the API token on {path} ({status}): it is wrong, expired or \
                 revoked, or it lacks a scope. {hint}"
            )));
        }
        404 => {
            return Err(ProbeError::Config(format!(
                "Infomaniak knows nothing at {path}: check the account ID. {hint}"
            )));
        }
        _ => return Err(client.status_error(status, &body, path)),
    }
    let reply: Value = client.decode(&body, path)?;
    if reply.get("result").and_then(Value::as_str) == Some("error") {
        let error = reply.get("error").unwrap_or(&Value::Null);
        let code = text(error, "code").unwrap_or("unknown");
        let description = text(error, "description").unwrap_or_default();
        return Err(if code.contains("auth") || code.contains("forbidden") {
            ProbeError::Auth(format!("Infomaniak refused {path} ({code}): {description} {hint}"))
        } else {
            ProbeError::Protocol(format!("Infomaniak answered {code} on {path}: {description}"))
        });
    }
    Ok(reply)
}

/// Les éléments de la clé `data` d'une réponse.
fn data(reply: &Value) -> Vec<&Value> {
    reply.get("data").and_then(Value::as_array).into_iter().flatten().collect()
}

/// Identifiants des comptes à lire : l'option, sinon la liste du jeton.
async fn accounts(client: &RestClient, target: &Target) -> Result<Vec<String>, ProbeError> {
    if let Some(id) = tag(target, "account_id") {
        return Ok(vec![cloud::url_segment("account_id", id)?]);
    }
    let reply = read(
        client,
        "/1/account",
        "To list the accounts the token needs the Accounts scope; otherwise set the Account ID \
         option.",
    )
    .await?;
    let ids: Vec<String> = data(&reply)
        .iter()
        .filter_map(|account| number(account.get("id")))
        .map(|id| format!("{id:.0}"))
        .take(MAX_ACCOUNTS)
        .collect();
    if ids.is_empty() {
        return Err(ProbeError::Config(
            "Infomaniak lists no account for this token: set the Account ID option.".to_string(),
        ));
    }
    Ok(ids)
}

/// Les produits des comptes, un appel par compte (et par service demandé).
async fn products(target: &Target) -> Result<Vec<Value>, ProbeError> {
    let token = match &target.credential {
        Credential::ApiToken { token } if !token.trim().is_empty() => token.trim().to_string(),
        other => {
            return Err(ProbeError::Config(format!(
                "Infomaniak expects an API token, configured: {other}"
            )));
        }
    };
    let client = RestClient::for_target(target, "https", DEFAULT_PORT, "Infomaniak")?
        .with_header("Authorization", format!("Bearer {token}"));
    let services: Vec<String> = tag(target, "service_names")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| cloud::url_segment("service_names", name))
        .collect::<Result<_, _>>()?;
    let mut out = Vec::new();
    for account in accounts(&client, target).await? {
        let filters: Vec<String> = if services.is_empty() {
            vec![String::new()]
        } else {
            services.iter().map(|name| format!("&service_name={name}")).collect()
        };
        for filter in filters {
            let path = format!("/1/product?account_id={account}{filter}");
            let reply = read(&client, &path, "The token needs the Products scope.").await?;
            out.extend(data(&reply).into_iter().cloned());
        }
    }
    Ok(out)
}

#[async_trait]
impl Collector for InfomaniakCollector {
    fn kind(&self) -> &'static str {
        "infomaniak"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let region = tag(target, "region").map(str::to_string);
        Ok(samples(&products(target).await?, region.as_deref(), now_ms()))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        products(target).await?;
        Ok(Some("infomaniak".to_string()))
    }
}

/// Un horodatage en secondes : nombre Unix (secondes ou millisecondes), texte
/// numérique ou RFC 3339. Zéro, négatif ou absent : pas de date.
fn timestamp(value: Option<&Value>) -> Option<f64> {
    let seconds = match value? {
        Value::String(s) => match chrono::DateTime::parse_from_rfc3339(s.trim()) {
            Ok(at) => at.timestamp() as f64,
            Err(_) => number(value)?,
        },
        _ => number(value)?,
    };
    if seconds <= 0.0 {
        return None;
    }
    Some(if seconds > 100_000_000_000.0 { seconds / 1000.0 } else { seconds })
}

fn flagged(product: &Value, key: &str) -> bool {
    number(product.get(key)).is_some_and(|v| v != 0.0)
}

pub fn samples(products: &[Value], region: Option<&str>, ts_ms: i64) -> Vec<Sample> {
    let now_s = ts_ms as f64 / 1000.0;
    let mut out = Vec::new();
    let (mut expiring, mut expired, mut locked, mut maintenance) = (0, 0, 0, 0);
    for (index, product) in products.iter().enumerate() {
        let id = number(product.get("id")).map(|id| format!("{id:.0}")).unwrap_or_default();
        let service = text(product, "service_name").unwrap_or("unknown");
        let name = text(product, "customer_name")
            .or_else(|| text(product, "internal_name"))
            .map(str::to_string)
            .unwrap_or_else(|| format!("{service} {id}"));
        let left = timestamp(product.get("expired_at")).map(|at| at - now_s);
        let (is_locked, in_maintenance) =
            (flagged(product, "is_locked"), flagged(product, "has_maintenance"));
        expired += usize::from(left.is_some_and(|s| s < 0.0));
        expiring += usize::from(left.is_some_and(|s| (0.0..SOON_SECONDS).contains(&s)));
        locked += usize::from(is_locked);
        maintenance += usize::from(in_maintenance);
        if index >= MAX_NAMED {
            continue;
        }
        let named = |sample: Sample| {
            sample
                .with_label("product", name.as_str())
                .with_label("service", service)
                .with_label("id", id.as_str())
        };
        if let Some(left) = left {
            out.push(named(gauge("infomaniak_product_expiry_seconds", left, ts_ms)));
        }
        out.push(named(flag("infomaniak_product_locked", is_locked, ts_ms)));
        out.push(named(flag("infomaniak_product_maintenance", in_maintenance, ts_ms)));
        out.push(named(flag(
            "infomaniak_product_operation_in_progress",
            flagged(product, "has_operation_in_progress"),
            ts_ms,
        )));
        let detail = if left.is_some_and(|s| s < 0.0) {
            "Expired"
        } else if is_locked {
            "Locked"
        } else if in_maintenance {
            "Maintenance"
        } else if left.is_some_and(|s| s < SOON_SECONDS) {
            "Expiring soon"
        } else {
            "OK"
        };
        out.push(cloud::item(PREFIX, "Products", &name, detail == "OK", detail, ts_ms));
    }
    // L'API des produits ne dit pas où ils sont : la région vient du réglage `region`. Le point
    // vaut le nombre de produits, pour la carte des régions.
    if let Some(region) = region.map(str::trim).filter(|r| !r.is_empty()) {
        out.push(
            gauge("infomaniak_region_products", products.len() as f64, ts_ms)
                .with_label("region", region),
        );
    }
    out.push(gauge("infomaniak_products", products.len() as f64, ts_ms));
    out.push(gauge("infomaniak_products_expiring", expiring as f64, ts_ms));
    out.push(gauge("infomaniak_products_expired", expired as f64, ts_ms));
    out.push(gauge("infomaniak_products_locked", locked as f64, ts_ms));
    out.push(gauge("infomaniak_products_maintenance", maintenance as f64, ts_ms));
    out.push(cloud::figure(PREFIX, "Products", "", products.len() as f64, ts_ms));
    out.push(cloud::figure(PREFIX, "Expiring soon", "", expiring as f64, ts_ms));
    out.push(cloud::figure(PREFIX, "Expired", "", expired as f64, ts_ms));
    out
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::Router;
    use axum::extract::State;
    use axum::http::{HeaderMap, StatusCode, Uri};

    use super::*;
    use crate::rest::test_support::{find, serve, value};
    use crate::uptime::tags::test_support::cible;

    const ACCOUNTS: &str = include_str!("testdata/accounts.json");
    const PRODUCTS: &str = include_str!("testdata/products.json");

    /// 2026-09-21T14:13:20Z : cinq jours avant l'expiration d'acme.ch.
    const NOW_S: i64 = 1_790_000_000;

    struct World {
        mode: &'static str,
    }

    async fn fake(
        State(world): State<Arc<World>>,
        headers: HeaderMap,
        uri: Uri,
    ) -> (StatusCode, String) {
        if world.mode == "down" {
            return (StatusCode::BAD_GATEWAY, "bad gateway".to_string());
        }
        let bearer = headers.get("authorization").and_then(|v| v.to_str().ok());
        if bearer != Some("Bearer good-token") {
            return (
                StatusCode::UNAUTHORIZED,
                r#"{"result":"error","error":{"code":"not_authorized","description":"Invalid token"}}"#
                    .to_string(),
            );
        }
        let path = uri.path_and_query().map(|p| p.as_str()).unwrap_or("/");
        match path {
            "/1/account" if world.mode == "no_accounts_scope" => (
                StatusCode::FORBIDDEN,
                r#"{"result":"error","error":{"code":"forbidden","description":"Missing scope"}}"#
                    .to_string(),
            ),
            "/1/account" => (StatusCode::OK, ACCOUNTS.to_string()),
            "/1/product?account_id=77" => (StatusCode::OK, PRODUCTS.to_string()),
            "/1/product?account_id=77&service_name=hosting" => (
                StatusCode::OK,
                r#"{"result":"success","data":[{"id":1002,"service_name":"hosting","customer_name":"Site web Acme","expired_at":1807280000,"is_locked":false,"has_maintenance":false}]}"#
                    .to_string(),
            ),
            _ => (
                StatusCode::NOT_FOUND,
                r#"{"result":"error","error":{"code":"not_found","description":"Unknown"}}"#
                    .to_string(),
            ),
        }
    }

    async fn serveur(mode: &'static str) -> String {
        let app = Router::new().fallback(fake).with_state(Arc::new(World { mode }));
        serve(app).await
    }

    fn cible_infomaniak(address: &str, token: &str, tags: &[(&str, &str)]) -> Target {
        let mut target = cible("infomaniak", &format!("http://{address}"), tags);
        target.credential = Credential::ApiToken { token: token.into() };
        target
    }

    fn produits() -> Vec<Value> {
        let reply: Value = serde_json::from_str(PRODUCTS).unwrap();
        data(&reply).into_iter().cloned().collect()
    }

    #[test]
    fn produits_du_compte() {
        let s = samples(&produits(), Some("geneva"), NOW_S * 1000);
        assert_eq!(value(&s, "infomaniak_region_products", &[("region", "geneva")]), 4.0);
        let none = samples(&produits(), None, NOW_S * 1000);
        assert!(find(&none, "infomaniak_region_products", &[]).is_none());
        let acme = [("product", "acme.ch")];
        assert_eq!(value(&s, "infomaniak_product_expiry_seconds", &acme), 5.0 * 86_400.0);
        assert_eq!(
            value(&s, "infomaniak_product_expiry_seconds", &[("product", "old-acme.com")]),
            -86_400.0
        );
        assert_eq!(value(&s, "infomaniak_product_locked", &[("product", "old-acme.com")]), 1.0);
        assert_eq!(
            value(&s, "infomaniak_product_maintenance", &[("product", "Site web Acme")]),
            1.0
        );
        assert!(
            find(&s, "infomaniak_product_expiry_seconds", &[("product", "mail-acme")]).is_none(),
            "a free product never expires"
        );
        assert_eq!(
            value(&s, "infomaniak_product_operation_in_progress", &[("product", "mail-acme")]),
            1.0,
            "an empty customer name falls back to the internal name"
        );
        assert_eq!(value(&s, "infomaniak_products", &[]), 4.0);
        assert_eq!(value(&s, "infomaniak_products_expiring", &[]), 1.0);
        assert_eq!(value(&s, "infomaniak_products_expired", &[]), 1.0);
        assert_eq!(value(&s, "infomaniak_products_locked", &[]), 1.0);
        assert_eq!(value(&s, "infomaniak_products_maintenance", &[]), 1.0);
        let item = find(&s, "infomaniak_item_ok", &[("item", "acme.ch")]).unwrap();
        assert_eq!((item.value, item.labels["detail"].as_str()), (0.0, "Expiring soon"));
        let item = find(&s, "infomaniak_item_ok", &[("item", "old-acme.com")]).unwrap();
        assert_eq!(item.labels["detail"], "Expired");
        assert_eq!(value(&s, "infomaniak_figure", &[("name", "Expired")]), 1.0);
    }

    #[test]
    fn les_dates_arrivent_sous_plusieurs_formes() {
        assert_eq!(timestamp(Some(&serde_json::json!(1_790_000_000))), Some(1_790_000_000.0));
        assert_eq!(
            timestamp(Some(&serde_json::json!(1_790_000_000_000_i64))),
            Some(1_790_000_000.0)
        );
        assert_eq!(
            timestamp(Some(&serde_json::json!("2026-09-21T14:13:20Z"))),
            Some(1_790_000_000.0)
        );
        assert_eq!(timestamp(Some(&serde_json::json!(null))), None);
        assert_eq!(timestamp(Some(&serde_json::json!(0))), None);
        assert_eq!(timestamp(None), None);
    }

    #[tokio::test]
    async fn lit_les_comptes_puis_les_produits() {
        let address = serveur("ok").await;
        let target = cible_infomaniak(&address, "good-token", &[]);
        let s = InfomaniakCollector::new().probe(&target).await.unwrap();
        assert_eq!(value(&s, "infomaniak_products", &[]), 4.0);
        assert_eq!(value(&s, "infomaniak_products_locked", &[]), 1.0);
    }

    #[tokio::test]
    async fn l_option_account_id_et_le_filtre_de_service() {
        let address = serveur("no_accounts_scope").await;
        let tags = [("account_id", "77"), ("service_names", "hosting")];
        let target = cible_infomaniak(&address, "good-token", &tags);
        let s = InfomaniakCollector::new().probe(&target).await.unwrap();
        assert_eq!(value(&s, "infomaniak_products", &[]), 1.0);
        let bad = cible_infomaniak(&address, "good-token", &[("account_id", "7/7")]);
        let error = InfomaniakCollector::new().probe(&bad).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
    }

    #[tokio::test]
    async fn jeton_refuse() {
        let collector = InfomaniakCollector::new();
        let address = serveur("ok").await;
        let error = collector.probe(&cible_infomaniak(&address, "bad", &[])).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
        assert!(!error.means_down());
        // Sans la portée « Comptes » ni l'option : refus expliqué, pas une panne.
        let address = serveur("no_accounts_scope").await;
        let error =
            collector.probe(&cible_infomaniak(&address, "good-token", &[])).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
        let mut target = cible_infomaniak(&address, "x", &[]);
        target.credential = Credential::None;
        assert!(matches!(collector.probe(&target).await, Err(ProbeError::Config(_))));
    }

    #[tokio::test]
    async fn un_service_en_panne_est_une_panne() {
        let address = serveur("down").await;
        let target = cible_infomaniak(&address, "good-token", &[]);
        let error = InfomaniakCollector::new().probe(&target).await.unwrap_err();
        assert!(error.means_down(), "{error}");
    }
}
