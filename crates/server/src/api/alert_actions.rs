//! Liens d'action des notifications : la page publique `/alert-action#<jeton>`
//! lit ce qu'un lien permet (`inspect`) puis, sur le clic de la personne,
//! l'exécute (`execute`). Voir [`crate::notify::action_link`] pour le jeton.
//!
//! Ces routes sont ouvertes — c'est leur raison d'être : agir depuis un
//! téléphone sans session. Ce qui les garde :
//!
//! - le jeton, signé, borné à un geste, à des alertes nommées et à 24 h, et
//!   à un seul usage (`alert_action_uses`) ;
//! - une seule réponse pour tout jeton refusé (forgé, altéré, expiré, ou
//!   fonction coupée) : rien à apprendre en essayant ;
//! - un plafond de refus par adresse de client ;
//! - l'en-tête anti-CSRF de l'interface et `Sec-Fetch-Site`, en plus du
//!   préflight CORS que ni l'un ni l'autre ne passent depuis une autre origine ;
//! - le jeton dans le corps, jamais dans l'URL : ni journal d'accès ni trace.
//!
//! Les deux routes sont des `POST` : la page affiche (et ne fait rien) tant
//! que personne n'appuie sur le bouton. Un préchargeur de liens (messagerie,
//! Slack, antivirus) ne charge que la page, sans jeton côté serveur.

use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, header};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::alerting::model::Severity;
use crate::api::alerts::{AckRequest, acknowledge};
use crate::api::{ApiError, ApiResult};
use crate::auth::audit;
use crate::auth::client_ip::ClientIp;
use crate::auth::middleware::{CSRF_HEADER, CSRF_VALUE};
use crate::db;
use crate::notify::action_link::{ACTOR, Action, ActionKey, Claims, MAX_TOKEN_LEN, Rejected};
use crate::notify::policy_store;
use crate::state::AppState;

/// Refus admis par adresse de client dans la fenêtre, avant 429.
const FAILURES_PER_CLIENT: usize = 20;
const WINDOW: Duration = Duration::from_secs(10 * 60);

/// Message unique de tout jeton refusé.
const INVALID: &str = "This link is invalid or has expired.";

pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/public/alert-action/inspect", post(inspect))
        .route("/public/alert-action", post(execute))
}

#[derive(Debug, Deserialize)]
pub struct TokenPayload {
    #[serde(default)]
    pub token: String,
}

/// Une alerte nommée par le lien, réduite à ce qu'il faut pour décider.
#[derive(Debug, Serialize)]
pub struct LinkedAlert {
    pub rule_name: String,
    pub target_name: Option<String>,
    pub severity: Severity,
    pub acked: bool,
    pub acked_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct ActionView {
    pub action: Action,
    pub expires_at: DateTime<Utc>,
    /// Le lien a déjà servi : la page le dit au lieu de proposer le bouton.
    pub used: bool,
    /// Alertes encore actives parmi celles du lien.
    pub alerts: Vec<LinkedAlert>,
}

#[derive(Debug, Serialize)]
pub struct ActionResult {
    pub action: Action,
    /// Fin de l'acquittement posé.
    pub until: DateTime<Utc>,
    /// Alertes acquittées par ce geste.
    pub acked: usize,
    pub alerts: Vec<LinkedAlert>,
}

// --------------------------------------------------------------------------
// Limitation des refus
// --------------------------------------------------------------------------

type Failures = HashMap<Option<IpAddr>, VecDeque<Instant>>;

static FAILURES: LazyLock<Mutex<Failures>> = LazyLock::new(|| Mutex::new(HashMap::new()));

fn prune(queue: &mut VecDeque<Instant>, now: Instant) {
    while queue.front().is_some_and(|at| now.duration_since(*at) >= WINDOW) {
        queue.pop_front();
    }
}

/// Vrai si ce client a épuisé ses refus dans la fenêtre.
fn over_budget(failures: &mut Failures, ip: Option<IpAddr>, now: Instant) -> bool {
    failures.retain(|_, queue| {
        prune(queue, now);
        !queue.is_empty()
    });
    failures.get(&ip).is_some_and(|queue| queue.len() >= FAILURES_PER_CLIENT)
}

/// Refuse d'emblée un client qui a trop échoué récemment.
fn check_budget(ip: Option<IpAddr>) -> ApiResult<()> {
    let mut failures = FAILURES.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if over_budget(&mut failures, ip, Instant::now()) {
        return Err(ApiError::TooManyRequests(
            "Too many invalid links from this address. Try again in a few minutes.".into(),
        ));
    }
    Ok(())
}

fn count_failure(ip: Option<IpAddr>) {
    let mut failures = FAILURES.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    failures.entry(ip).or_default().push_back(Instant::now());
}

// --------------------------------------------------------------------------
// Vérifications communes
// --------------------------------------------------------------------------

/// Seule la page de l'instance peut appeler : en-tête anti-CSRF exigé, et un
/// navigateur qui déclare une requête d'un autre site est refusé.
fn same_origin(headers: &HeaderMap) -> ApiResult<()> {
    let value = |name: &str| headers.get(name).and_then(|v| v.to_str().ok()).map(str::trim);
    let proof = value(CSRF_HEADER).is_some_and(|v| v.eq_ignore_ascii_case(CSRF_VALUE));
    let cross_site = value("sec-fetch-site").is_some_and(|site| site != "same-origin");
    if !proof || cross_site {
        return Err(ApiError::Forbidden("Cross-site request refused.".into()));
    }
    Ok(())
}

/// Vérifie le jeton et que la fonction est toujours en service. Tout refus
/// rend la même erreur, et compte contre le client.
async fn verified(state: &AppState, ip: Option<IpAddr>, token: &str) -> ApiResult<Claims> {
    check_budget(ip)?;
    let refuse = |reason: Rejected| {
        // Le jeton n'est jamais journalisé, seulement la raison.
        tracing::debug!(?reason, "alert action link refused");
        count_failure(ip);
        ApiError::NotFound(INVALID.into())
    };
    if token.len() > MAX_TOKEN_LEN {
        return Err(refuse(Rejected::Malformed));
    }
    let key = ActionKey::from_cipher(&state.cipher);
    let claims = key.verify(token, Utc::now()).map_err(refuse)?;
    // Couper la fonction (ou retirer l'URL publique) révoque les liens déjà
    // envoyés : c'est l'interrupteur d'urgence d'un lien qui aurait fui.
    let global = policy_store::load_global(&state.pool).await?;
    let env_url = crate::config::env_var("DUMBMONIT_PUBLIC_URL");
    if !global.action_links || global.public_url(env_url.as_deref()).is_none() {
        return Err(refuse(Rejected::BadSignature));
    }
    Ok(claims)
}

/// Les alertes du lien encore actives, avec ce qu'il faut pour les montrer.
async fn linked_alerts(state: &AppState, claims: &Claims) -> ApiResult<Vec<(String, LinkedAlert)>> {
    let rules = db::alerts::list_rules(&state.pool).await?;
    let now = Utc::now();
    let mut out = Vec::new();
    for fingerprint in &claims.fingerprints {
        let Some(alert) = db::alerts::get_active(&state.pool, fingerprint).await? else {
            continue;
        };
        let rule = rules.iter().find(|rule| rule.uid == alert.rule_uid);
        let target_name = match alert.target_id {
            Some(id) => {
                sqlx::query_scalar::<_, String>("SELECT name FROM targets WHERE id = ?")
                    .bind(id)
                    .fetch_optional(&state.pool)
                    .await?
            }
            None => None,
        };
        out.push((
            alert.fingerprint.clone(),
            LinkedAlert {
                rule_name: rule.map(|rule| rule.name.clone()).unwrap_or_default(),
                target_name,
                severity: rule.map_or(Severity::Warning, |rule| rule.severity),
                acked: alert.state.is_acked(now),
                acked_until: alert.state.acked_until,
            },
        ));
    }
    Ok(out)
}

async fn already_used(state: &AppState, claims: &Claims) -> ApiResult<bool> {
    let found: Option<i64> = sqlx::query_scalar("SELECT 1 FROM alert_action_uses WHERE nonce = ?")
        .bind(claims.nonce_hex())
        .fetch_optional(&state.pool)
        .await?;
    Ok(found.is_some())
}

/// Marque le lien comme utilisé. Faux s'il l'était déjà : deux clics
/// simultanés n'agissent qu'une fois (la clé primaire tranche).
async fn consume(state: &AppState, claims: &Claims, now: DateTime<Utc>) -> ApiResult<bool> {
    let stamp = |at: DateTime<Utc>| at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    sqlx::query("DELETE FROM alert_action_uses WHERE expires_at < ?")
        .bind(stamp(now))
        .execute(&state.pool)
        .await?;
    let inserted = sqlx::query(
        "INSERT INTO alert_action_uses (nonce, used_at, expires_at) VALUES (?, ?, ?)
         ON CONFLICT(nonce) DO NOTHING",
    )
    .bind(claims.nonce_hex())
    .bind(stamp(now))
    .bind(stamp(claims.expires_at))
    .execute(&state.pool)
    .await?
    .rows_affected();
    Ok(inserted == 1)
}

/// Réponses jamais mises en cache, ni par le navigateur ni par un mandataire.
fn no_store<T: Serialize>(body: T) -> Response {
    let mut response = Json(body).into_response();
    response.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

// --------------------------------------------------------------------------
// Routes
// --------------------------------------------------------------------------

/// `POST /api/public/alert-action/inspect` : ce que ferait le lien, sans rien
/// faire.
pub async fn inspect(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    headers: HeaderMap,
    Json(payload): Json<TokenPayload>,
) -> ApiResult<Response> {
    same_origin(&headers)?;
    let claims = verified(&state, ip, &payload.token).await?;
    let alerts = linked_alerts(&state, &claims).await?;
    Ok(no_store(ActionView {
        action: claims.action,
        expires_at: claims.expires_at,
        used: already_used(&state, &claims).await?,
        alerts: alerts.into_iter().map(|(_, alert)| alert).collect(),
    }))
}

/// `POST /api/public/alert-action` : exécute le geste du lien, une fois.
pub async fn execute(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    headers: HeaderMap,
    Json(payload): Json<TokenPayload>,
) -> ApiResult<Response> {
    same_origin(&headers)?;
    let claims = verified(&state, ip, &payload.token).await?;
    let alerts = linked_alerts(&state, &claims).await?;
    if alerts.is_empty() {
        return Err(ApiError::NotFound(
            "These alerts are no longer active: nothing to acknowledge.".into(),
        ));
    }
    let now = Utc::now();
    if !consume(&state, &claims, now).await? {
        return Err(ApiError::Conflict("This link has already been used.".into()));
    }

    let until = now + claims.action.duration();
    let audit_action = match claims.action {
        Action::Ack => "alert.acked",
        Action::Snooze => "alert.snoozed",
    };
    let mut acked = 0;
    let mut views = Vec::new();
    for (fingerprint, alert) in alerts {
        // Un « snooze » n'écourte jamais un acquittement plus long déjà posé.
        if alert.acked && alert.acked_until.is_some_and(|current| current >= until) {
            views.push(alert);
            continue;
        }
        let request = AckRequest::Set { until, note: None };
        match acknowledge(&state, &fingerprint, request, ACTOR).await {
            Ok(view) => {
                acked += 1;
                audit::record(&state.pool, Some(ACTOR), audit_action, Some(&fingerprint), ip).await;
                views.push(LinkedAlert {
                    acked: view.acked,
                    acked_until: view.acked_until,
                    ..alert
                });
            }
            // Résolue entre-temps : rien à acquitter, ce n'est pas une erreur.
            Err(ApiError::NotFound(_)) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(no_store(ActionResult { action: claims.action, until, acked, alerts: views }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (name, value) in pairs {
            map.insert(*name, HeaderValue::from_static(value));
        }
        map
    }

    #[test]
    fn a_client_that_keeps_failing_is_held_off_then_released() {
        let mut failures = Failures::new();
        let ip = Some(IpAddr::from([203, 0, 113, 9]));
        let other = Some(IpAddr::from([203, 0, 113, 10]));
        let start = Instant::now();
        for _ in 0..FAILURES_PER_CLIENT {
            assert!(!over_budget(&mut failures, ip, start));
            failures.entry(ip).or_default().push_back(start);
        }
        assert!(over_budget(&mut failures, ip, start));
        assert!(!over_budget(&mut failures, other, start), "per client");
        assert!(!over_budget(&mut failures, ip, start + WINDOW), "the window slides");
    }

    #[test]
    fn only_the_instance_page_may_call() {
        assert!(same_origin(&headers(&[("x-requested-with", "DumbMonit")])).is_ok());
        assert!(
            same_origin(&headers(&[
                ("x-requested-with", "DumbMonit"),
                ("sec-fetch-site", "same-origin")
            ]))
            .is_ok()
        );
        assert!(same_origin(&headers(&[])).is_err(), "no proof");
        assert!(same_origin(&headers(&[("sec-fetch-site", "same-origin")])).is_err());
        assert!(
            same_origin(&headers(&[
                ("x-requested-with", "DumbMonit"),
                ("sec-fetch-site", "cross-site")
            ]))
            .is_err()
        );
        assert!(
            same_origin(&headers(&[("x-requested-with", "DumbMonit"), ("sec-fetch-site", "none")]))
                .is_err(),
            "typed in the address bar: not a fetch from the page"
        );
    }
}
