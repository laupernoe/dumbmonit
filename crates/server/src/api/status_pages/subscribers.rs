//! Abonnés d'une page de statut : courriel, SMS, webhook signé ou Slack.
//!
//! Double confirmation : une destination saisie sur la page reçoit d'abord un
//! message de confirmation (lien par courriel ou Slack, code par SMS, requête
//! signée pour un webhook) et n'est prévenue des annonces qu'ensuite. Chaque
//! message porte un lien de désabonnement ; le courriel ajoute les en-têtes du
//! désabonnement en un clic.
//!
//! Le courriel passe par un canal SMTP, le SMS par un canal Vonage, Telnyx ou
//! ASPSMS déjà configurés dans les notifications ; webhook et Slack ne
//! demandent rien, le visiteur donne son URL. Sans canal ouvert, la page ne
//! propose que son flux RSS. Le point d'inscription est public : il est limité
//! par adresse de client, par page et globalement, pour que la page ne devienne
//! pas un relais de messages vers des inconnus.

use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use chrono::{TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::delivery::{self, SMS_KINDS};
use super::{format_timestamp, page_not_found, public_not_found};
use crate::api::{ApiError, ApiResult};
use crate::auth::client_ip::ClientIp;
use crate::auth::middleware::AdminIdentity;
use crate::db;
use crate::db::status_pages::{Incident, StatusPage, Subscriber};
use crate::notify::ChannelConfig;
use crate::notify::smtp::Smtp;
use crate::state::AppState;

/// Inscriptions admises par adresse de client dans la fenêtre.
const PER_CLIENT: usize = 5;
/// Inscriptions admises par page dans la fenêtre : un tiers qui change
/// d'adresse ne contourne pas la limite par client en visant une seule page.
const PER_PAGE: usize = 40;
/// Inscriptions admises en tout, toutes adresses confondues, dans la fenêtre :
/// le plafond du nombre de messages de confirmation qu'un tiers peut déclencher.
const GLOBAL: usize = 60;
/// Un SMS coûte de l'argent à l'administrateur : plafonds plus bas.
const SMS_PER_CLIENT: usize = 3;
const SMS_PER_PAGE: usize = 15;
const SMS_GLOBAL: usize = 25;
const WINDOW: Duration = Duration::from_secs(15 * 60);
/// Une demande en attente n'est pas renvoyée avant ce délai.
const RESEND_AFTER: TimeDelta = TimeDelta::minutes(10);
/// Une demande jamais confirmée est oubliée au bout de ce délai.
const PENDING_TTL: TimeDelta = TimeDelta::hours(48);
/// Un code SMS vaut ce délai, et ce nombre d'essais.
const CODE_TTL: TimeDelta = TimeDelta::minutes(15);
const CODE_ATTEMPTS: i64 = 5;
/// Plafond d'abonnés par page.
const MAX_SUBSCRIBERS: i64 = 10_000;

// --------------------------------------------------------------------------
// Limitation
// --------------------------------------------------------------------------

#[derive(Default)]
struct Limiter {
    per_client: HashMap<Option<IpAddr>, VecDeque<Instant>>,
    per_page: HashMap<i64, VecDeque<Instant>>,
    global: VecDeque<Instant>,
}

/// Plafonds d'une famille d'inscriptions : par client, par page, global.
struct Caps(usize, usize, usize);

const GENERAL: Caps = Caps(PER_CLIENT, PER_PAGE, GLOBAL);
const SMS: Caps = Caps(SMS_PER_CLIENT, SMS_PER_PAGE, SMS_GLOBAL);

static LIMITER: LazyLock<Mutex<Limiter>> = LazyLock::new(|| Mutex::new(Limiter::default()));
static SMS_LIMITER: LazyLock<Mutex<Limiter>> = LazyLock::new(|| Mutex::new(Limiter::default()));

fn prune(queue: &mut VecDeque<Instant>, now: Instant) {
    while queue.front().is_some_and(|at| now.duration_since(*at) >= WINDOW) {
        queue.pop_front();
    }
}

fn wait_of(queue: &VecDeque<Instant>, now: Instant) -> u64 {
    queue.front().map_or(1, |at| (WINDOW.saturating_sub(now.duration_since(*at))).as_secs() + 1)
}

/// Compte une tentative ; en cas de refus, rend le nombre de secondes à attendre.
fn admit(
    limiter: &Mutex<Limiter>,
    caps: &Caps,
    ip: Option<IpAddr>,
    page: i64,
    now: Instant,
) -> Result<(), u64> {
    let mut limiter = limiter.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let limiter = &mut *limiter;
    limiter.per_client.retain(|_, queue| {
        prune(queue, now);
        !queue.is_empty()
    });
    limiter.per_page.retain(|_, queue| {
        prune(queue, now);
        !queue.is_empty()
    });
    prune(&mut limiter.global, now);
    let client = limiter.per_client.entry(ip).or_default();
    if client.len() >= caps.0 {
        return Err(wait_of(client, now));
    }
    let on_page = limiter.per_page.entry(page).or_default();
    if on_page.len() >= caps.1 {
        return Err(wait_of(on_page, now));
    }
    if limiter.global.len() >= caps.2 {
        return Err(wait_of(&limiter.global, now));
    }
    client.push_back(now);
    on_page.push_back(now);
    limiter.global.push_back(now);
    Ok(())
}

fn too_many(retry_after: u64) -> Response {
    (
        StatusCode::TOO_MANY_REQUESTS,
        [(header::RETRY_AFTER, retry_after.to_string())],
        Json(json!({ "error": "Too many subscription requests. Try again later." })),
    )
        .into_response()
}

// --------------------------------------------------------------------------
// Canal et liens
// --------------------------------------------------------------------------

/// Canal SMTP actif choisi par la page, s'il y en a un.
async fn channel_of(state: &AppState, page: &StatusPage) -> Option<ChannelConfig> {
    let id = page.subscribe_channel_id?;
    match db::alerts::get_channel(&state.pool, &state.cipher, id).await {
        Ok(Some(channel)) if channel.enabled && channel.kind == "smtp" => Some(channel),
        Ok(_) => None,
        Err(error) => {
            tracing::warn!(%error, "status page: subscription channel unreadable");
            None
        }
    }
}

/// Canal SMS choisi par la page, s'il est actif et d'un type qui envoie des SMS.
pub async fn sms_channel_of(state: &AppState, page: &StatusPage) -> Option<ChannelConfig> {
    let id =
        db::status_pages::subscription_channels(&state.pool, page.id).await.ok()?.sms_channel_id?;
    match db::alerts::get_channel(&state.pool, &state.cipher, id).await {
        Ok(Some(channel)) if channel.enabled && SMS_KINDS.contains(&channel.kind.as_str()) => {
            Some(channel)
        }
        Ok(_) => None,
        Err(error) => {
            tracing::warn!(%error, "status page: SMS channel unreadable");
            None
        }
    }
}

/// Canaux d'abonnement réellement ouverts sur la page.
pub async fn available_kinds(state: &AppState, page: &StatusPage) -> Vec<&'static str> {
    let mut kinds = Vec::new();
    if channel_of(state, page).await.is_some() {
        kinds.push("email");
    }
    if sms_channel_of(state, page).await.is_some() {
        kinds.push("sms");
    }
    let open =
        db::status_pages::subscription_channels(&state.pool, page.id).await.unwrap_or_default();
    if open.webhook {
        kinds.push("webhook");
    }
    if open.slack {
        kinds.push("slack");
    }
    kinds
}

/// La page propose-t-elle au moins un abonnement ?
pub async fn offers_subscription(state: &AppState, page: &StatusPage) -> bool {
    !available_kinds(state, page).await.is_empty()
}

/// Base des liens des courriels : le domaine public de la page s'il y en a un
/// (servi en HTTPS par le mandataire), sinon l'URL publique réglée, sinon
/// l'origine vue par l'administrateur quand il a enregistré la page. Jamais
/// l'en-tête `Host` d'une requête publique : un tiers pourrait faire envoyer par
/// l'instance un courriel dont les liens mènent chez lui.
async fn link_base(state: &AppState, page: &StatusPage) -> Option<String> {
    if let Some(domain) = page.domain.as_deref() {
        return Some(format!("https://{domain}"));
    }
    let global = crate::notify::policy_store::load_global(&state.pool).await.ok();
    let env_url = crate::config::env_var("DUMBMONIT_PUBLIC_URL");
    if let Some(url) = global.and_then(|global| global.public_url(env_url.as_deref())) {
        return Some(url);
    }
    let origin = page.link_origin.trim().trim_end_matches('/');
    (!origin.is_empty()).then(|| origin.to_string())
}

/// Adresse de la page sous `base` : la racine de son domaine public, sinon
/// `/s/<slug>`. Les pages de confirmation et de désabonnement restent sous
/// `/s/<slug>/…`, servies aussi sur le domaine public.
fn page_url(base: &str, page: &StatusPage) -> String {
    match page.domain {
        Some(_) => format!("{base}/"),
        None => format!("{base}/s/{}", page.slug),
    }
}

fn new_token() -> String {
    hex::encode(rand::random::<[u8; 24]>())
}

/// Adresse courriel plausible, normalisée (domaine en minuscules), ou `None`.
pub fn normalise_email(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() || raw.len() > 254 || raw.chars().any(|c| c.is_whitespace() || c.is_control())
    {
        return None;
    }
    let (local, domain) = raw.rsplit_once('@')?;
    if local.is_empty()
        || local.len() > 64
        || local.contains('@')
        || !domain.contains('.')
        || domain.starts_with('.')
        || domain.ends_with('.')
        || domain.contains("..")
        || !domain.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-')
    {
        return None;
    }
    let email = format!("{local}@{}", domain.to_lowercase());
    email.parse::<lettre::Address>().ok().map(|_| email)
}

// --------------------------------------------------------------------------
// Public
// --------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct SubscribePayload {
    /// `email` (défaut), `sms`, `webhook` ou `slack`.
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    /// Clés publiques des services suivis ; absent ou vide : tous.
    #[serde(default)]
    pub components: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct TokenQuery {
    #[serde(default)]
    pub token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CodePayload {
    pub phone: String,
    pub code: String,
}

/// Page publiée qui propose au moins un abonnement, ou 404 : sans canal, la
/// route n'existe pas plus que pour une page absente.
async fn subscribable(state: &AppState, slug: &str) -> ApiResult<(StatusPage, Vec<&'static str>)> {
    let page = match db::status_pages::get_page_by_slug(&state.pool, slug).await? {
        Some(page) if page.published => page,
        _ => return Err(public_not_found()),
    };
    let kinds = available_kinds(state, &page).await;
    if kinds.is_empty() {
        return Err(public_not_found());
    }
    Ok((page, kinds))
}

/// La même réponse quel que soit l'état de la destination : la page ne dit à
/// personne qui y est abonné.
fn pending_reply(confirm: &str, message: &str, secret: Option<&str>) -> Response {
    let mut body = json!({ "status": "pending", "confirm": confirm, "message": message });
    if let Some(secret) = secret {
        body["secret"] = json!(secret);
    }
    (StatusCode::ACCEPTED, Json(body)).into_response()
}

const EMAIL_MESSAGE: &str =
    "Check your inbox: the subscription starts once you follow the link we sent.";
const SMS_MESSAGE: &str = "We sent a code by text message. Enter it to start the subscription.";
const WEBHOOK_MESSAGE: &str = "We sent a signed confirmation request to your URL. \
     The subscription starts once your endpoint calls the confirmation link it carries.";
const SLACK_MESSAGE: &str = "We posted a test message to your Slack channel. Follow the link in it to start the subscription.";

/// Services de la page par clé publique, dans l'ordre d'affichage : les mêmes
/// clés que celles du document public (voir `unique_key`).
async fn component_keys(state: &AppState, page: &StatusPage) -> ApiResult<Vec<(String, i64)>> {
    let items = db::status_pages::effective_items(&state.pool, page.id).await?;
    let present: std::collections::HashSet<i64> =
        sqlx::query_scalar::<_, i64>("SELECT id FROM targets")
            .fetch_all(&state.pool)
            .await
            .map_err(anyhow::Error::from)?
            .into_iter()
            .collect();
    let mut taken = std::collections::HashSet::new();
    Ok(items
        .into_iter()
        .filter(|item| present.contains(&item.target_id))
        .map(|item| (super::unique_key(&item.label, &mut taken), item.target_id))
        .collect())
}

/// Services choisis, en identifiants ; `None` : toute la page.
async fn chosen_components(
    state: &AppState,
    page: &StatusPage,
    keys: Option<&[String]>,
) -> ApiResult<Option<String>> {
    let Some(keys) = keys.filter(|keys| !keys.is_empty()) else { return Ok(None) };
    let known = component_keys(state, page).await?;
    let mut ids = Vec::new();
    for key in keys {
        let Some((_, id)) = known.iter().find(|(known_key, _)| known_key == key) else {
            return Err(ApiError::BadRequest("Unknown service in the selection.".into()));
        };
        if !ids.contains(id) {
            ids.push(*id);
        }
    }
    Ok(Some(serde_json::to_string(&ids).map_err(anyhow::Error::from)?))
}

fn hash_code(token: &str, code: &str) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(format!("{token}:{code}").as_bytes()))
}

pub async fn subscribe(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    Path(slug): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<SubscribePayload>,
) -> ApiResult<Response> {
    let kind = payload.kind.as_deref().unwrap_or("email").to_string();
    if !["email", "sms", "webhook", "slack"].contains(&kind.as_str()) {
        return Err(ApiError::BadRequest("Unknown subscription type.".into()));
    }
    let (page, kinds) = subscribable(&state, &slug).await?;
    let limiter = if kind == "sms" { (&SMS_LIMITER, &SMS) } else { (&LIMITER, &GENERAL) };
    if let Err(wait) = admit(limiter.0, limiter.1, ip, page.id, Instant::now()) {
        return Ok(too_many(wait));
    }
    // Sur une page privée, on ne s'abonne qu'en y ayant accès.
    super::access::authorize(&state, &page, &headers).await?;
    if !kinds.contains(&kind.as_str()) {
        return Err(ApiError::BadRequest("This page does not offer that subscription.".into()));
    }

    // Destination validée, selon le canal.
    let destination = match kind.as_str() {
        "email" => normalise_email(payload.email.as_deref().unwrap_or(""))
            .ok_or_else(|| ApiError::BadRequest("Enter a valid email address.".into()))?,
        "sms" => {
            delivery::normalise_phone(payload.phone.as_deref().unwrap_or("")).ok_or_else(|| {
                ApiError::BadRequest(
                    "Enter a phone number in international format, for example +33612345678."
                        .into(),
                )
            })?
        }
        "slack" => delivery::normalise_slack_url(payload.url.as_deref().unwrap_or("")).ok_or_else(
            || ApiError::BadRequest("Enter a Slack incoming webhook URL (hooks.slack.com).".into()),
        )?,
        _ => delivery::normalise_webhook_url(payload.url.as_deref().unwrap_or(""))
            .ok_or_else(|| ApiError::BadRequest("Enter a valid http(s) URL.".into()))?,
    };
    if matches!(kind.as_str(), "webhook" | "slack") && delivery::vet(&destination).await.is_err() {
        return Err(ApiError::BadRequest("This address cannot be used as a destination.".into()));
    }
    let components = chosen_components(&state, &page, payload.components.as_deref()).await?;
    let confirm_mode = match kind.as_str() {
        "sms" => "code",
        "webhook" => "signed_request",
        _ => "link",
    };
    let message = match kind.as_str() {
        "sms" => SMS_MESSAGE,
        "webhook" => WEBHOOK_MESSAGE,
        "slack" => SLACK_MESSAGE,
        _ => EMAIL_MESSAGE,
    };

    let now = Utc::now();
    db::status_pages::purge_unconfirmed(&state.pool, &format_timestamp(now - PENDING_TTL)).await?;

    match db::status_pages::find_subscriber(&state.pool, page.id, &destination).await? {
        // Déjà abonné : rien à envoyer, et rien à dire de plus.
        Some(existing) if existing.confirmed_at.is_some() => {
            return Ok(pending_reply(confirm_mode, message, None));
        }
        // Une demande récente attend déjà : pas de second message.
        Some(existing)
            if super::parse_stored(&existing.created_at)
                .is_some_and(|created| now - created < RESEND_AFTER) =>
        {
            return Ok(pending_reply(confirm_mode, message, None));
        }
        Some(_) => {}
        None => {
            if db::status_pages::count_subscribers(&state.pool, page.id).await? >= MAX_SUBSCRIBERS {
                return Err(ApiError::Conflict("This page cannot take more subscribers.".into()));
            }
        }
    }

    let Some(base) = link_base(&state, &page).await else {
        tracing::warn!(slug = %page.slug, "status page: no public URL, confirmation not sent");
        return Ok(pending_reply(confirm_mode, message, None));
    };
    let token = new_token();
    let code = format!("{:06}", rand::random::<u32>() % 1_000_000);
    let secret =
        (kind == "webhook").then(|| format!("whsec_{}", hex::encode(rand::random::<[u8; 24]>())));
    let sealed = match &secret {
        Some(secret) => Some(state.cipher.encrypt(secret.as_bytes())?),
        None => None,
    };
    let code_hash = (kind == "sms").then(|| hash_code(&token, &code));
    db::status_pages::upsert_pending_subscriber(
        &state.pool,
        &db::status_pages::PendingSubscriber {
            page_id: page.id,
            kind: &kind,
            destination: &destination,
            token: &token,
            components: components.as_deref(),
            secret: sealed.as_deref(),
            code_hash: code_hash.as_deref(),
        },
    )
    .await?;

    let confirm = format!("{base}/s/{}/confirm?token={token}", page.slug);
    let title = page.title.clone();
    let home = page_url(&base, &page);
    match kind.as_str() {
        "email" => {
            let channel = channel_of(&state, &page).await.ok_or_else(public_not_found)?;
            let subject = format!("Confirm your subscription to {title}");
            let body = format!(
                "Someone, hopefully you, asked to receive incident and maintenance updates from \
                 {title} by email.\n\nConfirm the subscription:\n{confirm}\n\n\
                 If you did not ask for this, ignore this message: without confirmation, the \
                 address is forgotten within two days.\n\n-- \n{title}\n{home}\n",
            );
            tokio::spawn(async move {
                let result = match Smtp::new(&channel) {
                    Ok(smtp) => smtp.send_to(&destination, &subject, body, None).await,
                    Err(error) => Err(error),
                };
                if let Err(error) = result {
                    tracing::warn!(%error, "status page: confirmation email not sent");
                }
            });
        }
        "sms" => {
            let channel = sms_channel_of(&state, &page).await.ok_or_else(public_not_found)?;
            let text =
                format!("{code} is your code to follow {title}. Ignore this if it is not you.");
            tokio::spawn(async move {
                if let Err(error) = delivery::send_sms(&channel, &destination, &title, &text).await
                {
                    tracing::warn!(%error, "status page: confirmation SMS not sent");
                }
            });
        }
        "webhook" => {
            let signing = secret.clone().unwrap_or_default();
            let body = json!({
                "type": "subscription.confirm",
                "page": { "slug": page.slug, "title": title, "url": home },
                "token": token,
                "confirm_url": confirm,
                "message": "POST to confirm_url (or open it) to start receiving updates. \
                            Ignore this request otherwise.",
            });
            tokio::spawn(async move {
                if let Err(error) =
                    delivery::send_webhook(&destination, &signing, "subscription.confirm", &body)
                        .await
                {
                    tracing::warn!(%error, "status page: confirmation request not sent");
                }
            });
        }
        _ => {
            let text = format!(
                "*{title}*\nThis channel asked to receive incident and maintenance updates. \
                 <{confirm}|Confirm the subscription>. Ignore this message otherwise: \
                 without confirmation, it is forgotten within two days."
            );
            tokio::spawn(async move {
                if let Err(error) = delivery::send_slack(&destination, &text).await {
                    tracing::warn!(%error, "status page: Slack confirmation not sent");
                }
            });
        }
    }
    Ok(pending_reply(confirm_mode, message, secret.as_deref()))
}

pub async fn confirm(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(query): Query<TokenQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let (page, _) = subscribable(&state, &slug).await?;
    let token = query.token.unwrap_or_default();
    let expired = || ApiError::NotFound("This confirmation link is no longer valid.".into());
    if token.is_empty() {
        return Err(expired());
    }
    let subscriber = db::status_pages::subscriber_by_token(&state.pool, page.id, &token)
        .await?
        .ok_or_else(expired)?;
    // Un abonné SMS se confirme par son code, pas par un lien.
    if subscriber.code_hash.is_some() {
        return Err(expired());
    }
    db::status_pages::confirm_subscriber(&state.pool, subscriber.id).await?;
    Ok(Json(json!({ "status": "confirmed" })))
}

/// Confirmation d'un abonné SMS par le code reçu.
pub async fn confirm_code(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<CodePayload>,
) -> ApiResult<Json<serde_json::Value>> {
    let (page, _) = subscribable(&state, &slug).await?;
    super::access::authorize(&state, &page, &headers).await?;
    let wrong = || ApiError::BadRequest("This code is wrong or has expired.".into());
    let phone = delivery::normalise_phone(&payload.phone).ok_or_else(wrong)?;
    let subscriber = db::status_pages::find_subscriber(&state.pool, page.id, &phone)
        .await?
        .filter(|s| s.kind == "sms")
        .ok_or_else(wrong)?;
    let Some(expected) = subscriber.code_hash.as_deref() else {
        // Déjà confirmé.
        return Ok(Json(json!({ "status": "confirmed" })));
    };
    let fresh = super::parse_stored(&subscriber.created_at)
        .is_some_and(|created| Utc::now() - created < CODE_TTL);
    if !fresh
        || db::status_pages::bump_code_attempts(&state.pool, subscriber.id).await? >= CODE_ATTEMPTS
    {
        return Err(wrong());
    }
    use subtle::ConstantTimeEq;
    let given = hash_code(&subscriber.token, payload.code.trim());
    if !bool::from(given.as_bytes().ct_eq(expected.as_bytes())) {
        return Err(wrong());
    }
    db::status_pages::confirm_subscriber(&state.pool, subscriber.id).await?;
    Ok(Json(json!({ "status": "confirmed" })))
}

/// Désabonnement : depuis le lien d'un message (la page `/s/<slug>/unsubscribe`
/// poste ici) ou directement par la messagerie (RFC 8058, corps ignoré).
///
/// Toujours la même réponse : un jeton inconnu est un désabonnement déjà fait.
/// La page n'a pas besoin de proposer l'abonnement pour qu'on s'en désinscrive.
pub async fn unsubscribe(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(query): Query<TokenQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let Some(page) = db::status_pages::get_page_by_slug(&state.pool, &slug).await? else {
        return Err(public_not_found());
    };
    let token = query.token.unwrap_or_default();
    if !token.is_empty()
        && let Some(subscriber) =
            db::status_pages::subscriber_by_token(&state.pool, page.id, &token).await?
    {
        db::status_pages::delete_subscriber(&state.pool, page.id, subscriber.id).await?;
    }
    Ok(Json(json!({ "status": "unsubscribed" })))
}

// --------------------------------------------------------------------------
// Administration
// --------------------------------------------------------------------------

/// Un abonné tel que l'administrateur le voit : destination masquée sauf pour
/// le courriel, jamais de jeton ni de secret.
#[derive(Debug, Serialize)]
pub struct SubscriberView {
    pub id: i64,
    pub page_id: i64,
    pub kind: String,
    /// Adresse complète pour un courriel ; numéro ou URL masqués sinon.
    pub destination: String,
    /// Adresse courriel, pour les abonnés par courriel seulement.
    pub email: Option<String>,
    /// Identifiants des services suivis ; `None` : tous.
    pub components: Option<Vec<i64>>,
    pub confirmed_at: Option<String>,
    pub created_at: String,
}

impl From<Subscriber> for SubscriberView {
    fn from(subscriber: Subscriber) -> Self {
        Self {
            destination: delivery::mask(&subscriber.kind, &subscriber.email),
            components: subscriber.component_ids(),
            email: (subscriber.kind == "email").then(|| subscriber.email.clone()),
            id: subscriber.id,
            page_id: subscriber.page_id,
            kind: subscriber.kind,
            confirmed_at: subscriber.confirmed_at,
            created_at: subscriber.created_at,
        }
    }
}

/// Destinations de personnes du public : données personnelles, réservées aux
/// administrateurs (et aux jetons `write`, qui en ont les droits).
pub async fn list(
    _: AdminIdentity,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<Json<Vec<SubscriberView>>> {
    db::status_pages::get_page(&state.pool, id).await?.ok_or_else(|| page_not_found(id))?;
    let subscribers = db::status_pages::list_subscribers(&state.pool, id).await?;
    Ok(Json(subscribers.into_iter().map(SubscriberView::from).collect()))
}

/// Cellule CSV : guillemets doublés, et une apostrophe devant ce qu'un tableur
/// prendrait pour une formule.
fn csv_cell(value: &str, numeric_ok: bool) -> String {
    let risky =
        value.starts_with(['=', '@', '\t', '\r']) || (!numeric_ok && value.starts_with(['+', '-']));
    let value = if risky { format!("'{value}") } else { value.to_string() };
    format!("\"{}\"", value.replace('"', "\"\""))
}

/// Export CSV des abonnés d'une page. Les URL (webhook, Slack) restent
/// masquées : la plupart contiennent un secret.
pub async fn export(
    _: AdminIdentity,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<Response> {
    let page =
        db::status_pages::get_page(&state.pool, id).await?.ok_or_else(|| page_not_found(id))?;
    let mut csv = String::from("type,destination,status,subscribed_at,services\r\n");
    for subscriber in db::status_pages::list_subscribers(&state.pool, id).await? {
        let destination = match subscriber.kind.as_str() {
            "email" | "sms" => subscriber.email.clone(),
            kind => delivery::mask(kind, &subscriber.email),
        };
        let services = subscriber
            .component_ids()
            .map(|ids| ids.iter().map(i64::to_string).collect::<Vec<_>>().join(" "))
            .unwrap_or_default();
        csv.push_str(&format!(
            "{},{},{},{},{}\r\n",
            csv_cell(&subscriber.kind, true),
            csv_cell(&destination, subscriber.kind == "sms"),
            if subscriber.confirmed_at.is_some() { "confirmed" } else { "pending" },
            csv_cell(subscriber.confirmed_at.as_deref().unwrap_or(&subscriber.created_at), true),
            csv_cell(&services, true),
        ));
    }
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"subscribers-{}.csv\"", page.slug),
            ),
        ],
        csv,
    )
        .into_response())
}

pub async fn remove(
    State(state): State<AppState>,
    Path((id, subscriber)): Path<(i64, i64)>,
) -> ApiResult<StatusCode> {
    if db::status_pages::delete_subscriber(&state.pool, id, subscriber).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound(format!("Subscriber {subscriber} not found.")))
    }
}

/// Réglages d'abonnement d'une page, hors canal courriel (réglé avec la page).
#[derive(Debug, Serialize, Deserialize)]
pub struct ChannelSettings {
    /// Canal Vonage, Telnyx ou ASPSMS qui envoie les SMS ; `null` : pas de SMS.
    #[serde(default)]
    pub sms_channel_id: Option<i64>,
    #[serde(default)]
    pub webhook: bool,
    #[serde(default)]
    pub slack: bool,
}

pub async fn get_settings(
    _: AdminIdentity,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<Json<ChannelSettings>> {
    db::status_pages::get_page(&state.pool, id).await?.ok_or_else(|| page_not_found(id))?;
    let open = db::status_pages::subscription_channels(&state.pool, id).await?;
    Ok(Json(ChannelSettings {
        sms_channel_id: open.sms_channel_id,
        webhook: open.webhook,
        slack: open.slack,
    }))
}

pub async fn put_settings(
    _: AdminIdentity,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(settings): Json<ChannelSettings>,
) -> ApiResult<Json<ChannelSettings>> {
    db::status_pages::get_page(&state.pool, id).await?.ok_or_else(|| page_not_found(id))?;
    if let Some(channel) = settings.sms_channel_id {
        let found = db::alerts::get_channel(&state.pool, &state.cipher, channel).await?;
        if !found.is_some_and(|c| SMS_KINDS.contains(&c.kind.as_str())) {
            return Err(ApiError::BadRequest(
                "SMS subscribers are texted through a Vonage, Telnyx or ASPSMS channel.".into(),
            ));
        }
    }
    db::status_pages::set_subscription_channels(
        &state.pool,
        id,
        &db::status_pages::SubscriptionChannels {
            sms_channel_id: settings.sms_channel_id,
            webhook: settings.webhook,
            slack: settings.slack,
        },
    )
    .await?;
    super::invalidate_cache();
    Ok(Json(settings))
}

// --------------------------------------------------------------------------
// Envoi des annonces
// --------------------------------------------------------------------------

pub(super) fn status_word(status: &str) -> &str {
    match status {
        "investigating" => "Investigating",
        "identified" => "Identified",
        "monitoring" => "Monitoring",
        "resolved" => "Resolved",
        "scheduled" => "Scheduled",
        "in_progress" => "In progress",
        "completed" => "Completed",
        other => other,
    }
}

/// L'abonné suit-il une annonce qui touche ces services ? Sans choix d'un côté
/// ou de l'autre, oui.
fn follows(subscriber: &Subscriber, affected: &[i64]) -> bool {
    match subscriber.component_ids() {
        Some(chosen) if !chosen.is_empty() && !affected.is_empty() => {
            chosen.iter().any(|id| affected.contains(id))
        }
        _ => true,
    }
}

/// Prévient les abonnés confirmés des pages concernées par une annonce : sa
/// page, ou toutes les pages publiées pour une annonce globale. L'envoi part en
/// tâche de fond : l'administrateur n'attend pas le serveur SMTP.
pub fn announce(state: &AppState, incident: Incident, message: Option<String>) {
    let state = state.clone();
    tokio::spawn(async move {
        if let Err(error) = announce_now(&state, &incident, message.as_deref()).await {
            tracing::warn!(%error, incident = incident.id, "status page: subscribers not notified");
        }
    });
}

async fn announce_now(
    state: &AppState,
    incident: &Incident,
    message: Option<&str>,
) -> anyhow::Result<()> {
    let pages: Vec<StatusPage> = match incident.page_id {
        Some(id) => db::status_pages::get_page(&state.pool, id).await?.into_iter().collect(),
        None => db::status_pages::list_pages(&state.pool).await?,
    };
    let affected = db::status_pages::incident_components(&state.pool, incident.id).await?;
    let mut queued = false;
    for page in pages.into_iter().filter(|page| page.published) {
        let subscribers: Vec<Subscriber> =
            db::status_pages::confirmed_subscribers(&state.pool, page.id)
                .await?
                .into_iter()
                .filter(|subscriber| follows(subscriber, &affected))
                .collect();
        if subscribers.is_empty() {
            continue;
        }
        let Some(base) = link_base(state, &page).await else {
            tracing::warn!(slug = %page.slug, "status page: no public URL, subscribers not notified");
            continue;
        };
        let page_url = page_url(&base, &page);

        // SMS, webhook, Slack : par la file, avec reprises.
        let announcement = delivery::Announcement {
            kind: incident.kind.clone(),
            incident_id: incident.id,
            title: incident.title.clone(),
            status: incident.status.clone(),
            severity: incident.severity.clone(),
            starts_at: incident.starts_at.clone(),
            ends_at: incident.ends_at.clone(),
            message: message.map(|m| m.trim().to_string()).filter(|m| !m.is_empty()),
            page_slug: page.slug.clone(),
            page_title: page.title.clone(),
            page_url: page_url.clone(),
            base: base.clone(),
        };
        let payload = serde_json::to_string(&announcement)?;
        for subscriber in subscribers.iter().filter(|s| s.kind != "email") {
            db::status_pages::enqueue_delivery(&state.pool, subscriber.id, &payload).await?;
            queued = true;
        }

        // Courriel : envoi direct.
        let emails: Vec<&Subscriber> = subscribers.iter().filter(|s| s.kind == "email").collect();
        if emails.is_empty() {
            continue;
        }
        let Some(channel) = channel_of(state, &page).await else { continue };
        let smtp = match Smtp::new(&channel) {
            Ok(smtp) => smtp,
            Err(error) => {
                tracing::warn!(%error, "status page: subscription channel misconfigured");
                continue;
            }
        };
        let kind = if incident.kind == "maintenance" { "Maintenance" } else { "Incident" };
        let status = status_word(&incident.status);
        let subject = format!("[{}] {kind}: {} ({status})", page.title, incident.title);
        let mut sent = 0usize;
        for subscriber in emails {
            let unsubscribe_page =
                format!("{base}/s/{}/unsubscribe?token={}", page.slug, subscriber.token);
            let one_click = format!(
                "{base}/api/public/status/{}/unsubscribe?token={}",
                page.slug, subscriber.token
            );
            let mut body = format!("{kind}: {}\nStatus: {status}\n", incident.title);
            if let Some(message) = message.filter(|m| !m.trim().is_empty()) {
                body.push('\n');
                body.push_str(message.trim());
                body.push('\n');
            }
            body.push_str(&format!(
                "\nFollow it on the status page:\n{page_url}\n\n-- \n\
                 You receive this because you subscribed to {title}.\n\
                 Unsubscribe in one click: {unsubscribe_page}\n",
                title = page.title,
            ));
            match smtp.send_to(&subscriber.email, &subject, body, Some(&one_click)).await {
                Ok(()) => sent += 1,
                Err(error) => tracing::warn!(%error, "status page: update email not sent"),
            }
        }
        tracing::info!(slug = %page.slug, sent, "status page: subscribers notified");
    }
    if queued {
        delivery::process_due(state).await;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_addresses_are_checked_and_normalised() {
        assert_eq!(normalise_email(" Alice@Example.ORG "), Some("Alice@example.org".into()));
        assert_eq!(normalise_email("a@b"), None);
        assert_eq!(normalise_email("no-at-sign.org"), None);
        assert_eq!(normalise_email("a b@c.org"), None);
        assert_eq!(normalise_email("a@c.org\r\nBcc: x@y.org"), None);
        assert_eq!(normalise_email(&format!("{}@c.org", "x".repeat(65))), None);
    }

    #[test]
    fn the_limiter_caps_one_client() {
        let ip: Option<IpAddr> = Some("203.0.113.77".parse().unwrap());
        let start = Instant::now();
        let limiter = Mutex::new(Limiter::default());
        for _ in 0..PER_CLIENT {
            assert!(admit(&limiter, &GENERAL, ip, 1, start).is_ok());
        }
        assert!(admit(&limiter, &GENERAL, ip, 1, start).is_err());
        // Une autre adresse n'est pas pénalisée.
        assert!(admit(&limiter, &GENERAL, Some("203.0.113.78".parse().unwrap()), 1, start).is_ok());
    }

    #[test]
    fn the_limiter_caps_one_page() {
        let limiter = Mutex::new(Limiter::default());
        let start = Instant::now();
        for n in 0..PER_PAGE {
            let ip: Option<IpAddr> = Some(format!("198.51.100.{}", n % 250).parse().unwrap());
            // Chaque client reste sous son plafond : seule la page sature.
            assert!(admit(&limiter, &Caps(100, PER_PAGE, 1000), ip, 7, start).is_ok());
        }
        let other: Option<IpAddr> = Some("198.51.100.250".parse().unwrap());
        assert!(admit(&limiter, &Caps(100, PER_PAGE, 1000), other, 7, start).is_err());
        assert!(admit(&limiter, &Caps(100, PER_PAGE, 1000), other, 8, start).is_ok());
    }
}
