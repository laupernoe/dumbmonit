//! Envoi aux abonnés d'une page de statut par SMS, webhook signé ou Slack.
//!
//! Les annonces passent par une file (`status_deliveries`) : une ligne par
//! abonné, reprise avec un délai croissant tant que la destination répond mal,
//! puis abandonnée. Les messages de confirmation (code SMS, requête signée,
//! message Slack) partent directement : le visiteur les attend.

use std::time::Duration;

use chrono::{TimeDelta, Utc};
use hmac::{Hmac, KeyInit, Mac};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use super::format_timestamp;
use crate::db;
use crate::db::status_pages::Subscriber;
use crate::notify::http::{self, Request};
use crate::notify::{ChannelConfig, Notifier, NotifyError, guard, message};
use crate::state::AppState;

/// Types de canal de notification capables d'envoyer un SMS.
pub const SMS_KINDS: [&str; 3] = ["vonage", "telnyx", "aspsms"];
/// Essais d'un envoi avant abandon.
const MAX_ATTEMPTS: i64 = 5;
/// Délai avant la reprise qui suit l'essai n° N (en secondes).
const BACKOFF: [i64; 4] = [60, 300, 1800, 7200];
/// Une annonce qui n'est pas partie au bout de ce délai n'a plus d'intérêt.
const MAX_AGE: TimeDelta = TimeDelta::hours(24);
/// Envois traités par passage, et pause entre deux : la file ne mitraille pas
/// un fournisseur SMS ni une URL tierce.
const BATCH: i64 = 25;
const PAUSE: Duration = Duration::from_millis(150);

/// Une annonce telle qu'elle est mise en file. Rendue au moment de l'envoi, par
/// canal : le SMS tient en une ligne, le webhook en JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Announcement {
    /// `incident` ou `maintenance`.
    pub kind: String,
    pub incident_id: i64,
    pub title: String,
    pub status: String,
    pub severity: String,
    pub starts_at: String,
    pub ends_at: Option<String>,
    pub message: Option<String>,
    pub page_slug: String,
    pub page_title: String,
    pub page_url: String,
    /// Base des liens (désabonnement).
    pub base: String,
}

// --------------------------------------------------------------------------
// Adresses et signatures
// --------------------------------------------------------------------------

/// Numéro E.164 (`+` puis 8 à 15 chiffres), espaces et tirets retirés.
pub fn normalise_phone(raw: &str) -> Option<String> {
    let cleaned: String =
        raw.trim().chars().filter(|c| !matches!(c, ' ' | '-' | '.' | '(' | ')')).collect();
    let digits = cleaned.strip_prefix('+')?;
    let valid = (8..=15).contains(&digits.len())
        && digits.chars().all(|c| c.is_ascii_digit())
        && !digits.starts_with('0');
    valid.then_some(cleaned)
}

/// URL d'un webhook entrant Slack : HTTPS, hôte Slack, chemin de service.
pub fn normalise_slack_url(raw: &str) -> Option<String> {
    let url = Url::parse(raw.trim()).ok()?;
    let host_ok = matches!(url.host_str(), Some("hooks.slack.com" | "hooks.slack-gov.com"));
    let path_ok = url.path().starts_with("/services/") || url.path().starts_with("/workflows/");
    (url.scheme() == "https"
        && host_ok
        && path_ok
        && url.username().is_empty()
        && url.password().is_none()
        && url.as_str().len() <= 300)
        .then(|| url.to_string())
}

/// URL d'un webhook générique : HTTP(S), sans identifiants intégrés.
pub fn normalise_webhook_url(raw: &str) -> Option<String> {
    let url = Url::parse(raw.trim()).ok()?;
    (matches!(url.scheme(), "http" | "https")
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none()
        && url.as_str().len() <= 500)
        .then(|| url.to_string())
}

/// Refuse une destination que la garde des envois sortants interdit.
pub async fn vet(url: &str) -> Result<(), NotifyError> {
    match Url::parse(url) {
        Ok(url) => guard::vet_url(&url).await,
        Err(_) => Err(NotifyError::Config("invalid URL".into())),
    }
}

/// Signature HMAC-SHA256 d'un webhook : `sha256=<hex>` sur `<horodatage>.<corps>`.
pub fn sign(secret: &str, timestamp: i64, body: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("clé de toute taille");
    mac.update(format!("{timestamp}.{body}").as_bytes());
    format!("sha256={}", hex::encode(mac.finalize().into_bytes()))
}

/// Numéro ou URL réduit à ce qu'un administrateur peut afficher.
pub fn mask(kind: &str, destination: &str) -> String {
    match kind {
        "sms" => {
            let chars: Vec<char> = destination.chars().collect();
            if chars.len() <= 5 {
                return "•".repeat(chars.len());
            }
            let head: String = chars[..3].iter().collect();
            let tail: String = chars[chars.len() - 2..].iter().collect();
            format!("{head}{}{tail}", "•".repeat(chars.len() - 5))
        }
        "webhook" | "slack" => Url::parse(destination)
            .ok()
            .and_then(|url| url.host_str().map(|host| format!("{}://{host}/…", url.scheme())))
            .unwrap_or_else(|| "…".into()),
        _ => destination.to_string(),
    }
}

// --------------------------------------------------------------------------
// Envois
// --------------------------------------------------------------------------

fn clip(text: &str, max: usize) -> String {
    let flat: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= max {
        return flat;
    }
    let mut cut: String = flat.chars().take(max.saturating_sub(1)).collect();
    cut.push('…');
    cut
}

/// SMS vers un seul numéro, par le canal SMS de la page.
pub async fn send_sms(
    channel: &ChannelConfig,
    phone: &str,
    title: &str,
    text: &str,
) -> Result<(), NotifyError> {
    let mut one = channel.clone();
    if let Some(settings) = one.settings.as_object_mut() {
        settings.insert("to".into(), serde_json::json!([phone]));
    }
    let notifier = crate::notify::build(&crate::notify::http_client(), &one)?;
    let mut msg = message::test_message(&channel.name);
    msg.title = title.to_string();
    msg.text = text.to_string();
    msg.markdown = format!("{title} {text}");
    msg.link = None;
    notifier.send(&msg).await
}

/// Requête signée vers un webhook d'abonné.
pub async fn send_webhook(
    url: &str,
    secret: &str,
    event: &str,
    body: &serde_json::Value,
) -> Result<(), NotifyError> {
    let body = body.to_string();
    let timestamp = Utc::now().timestamp();
    let signature = sign(secret, timestamp, &body);
    http::send(
        &crate::notify::http_client(),
        Request::post(url)
            .raw("application/json", body)
            .header("X-DumbMonit-Event", event)
            .header("X-DumbMonit-Timestamp", timestamp.to_string())
            .header("X-DumbMonit-Signature", signature),
    )
    .await
}

/// Message vers un webhook entrant Slack.
pub async fn send_slack(url: &str, text: &str) -> Result<(), NotifyError> {
    http::send(
        &crate::notify::http_client(),
        Request::post(url).json(serde_json::json!({ "text": text })),
    )
    .await
}

fn webhook_secret(state: &AppState, subscriber: &Subscriber) -> Option<String> {
    let sealed = subscriber.secret.as_deref()?;
    let clear = state.cipher.decrypt(sealed).ok()?;
    String::from_utf8(clear).ok()
}

/// Envoie une annonce à un abonné, selon son canal.
async fn deliver(
    state: &AppState,
    subscriber: &Subscriber,
    announcement: &Announcement,
) -> Result<(), NotifyError> {
    let unsubscribe = format!(
        "{}/s/{}/unsubscribe?token={}",
        announcement.base, announcement.page_slug, subscriber.token
    );
    let kind_word = if announcement.kind == "maintenance" { "Maintenance" } else { "Incident" };
    let status = super::subscribers::status_word(&announcement.status);
    let headline =
        format!("[{}] {kind_word}: {} ({status})", announcement.page_title, announcement.title);
    match subscriber.kind.as_str() {
        "sms" => {
            let page = db::status_pages::get_page(&state.pool, subscriber.page_id)
                .await
                .map_err(|e| NotifyError::Config(e.to_string()))?;
            let channel = match page {
                Some(page) => super::subscribers::sms_channel_of(state, &page).await,
                None => None,
            };
            let channel = channel
                .ok_or_else(|| NotifyError::Config("no SMS channel for this page".into()))?;
            let mut text = String::new();
            if let Some(extra) = announcement.message.as_deref().filter(|m| !m.trim().is_empty()) {
                text.push_str(&clip(extra, 60));
                text.push(' ');
            }
            text.push_str(&format!("Stop: {unsubscribe}"));
            send_sms(&channel, &subscriber.email, &clip(&headline, 110), &text).await
        }
        "webhook" => {
            let secret = webhook_secret(state, subscriber)
                .ok_or_else(|| NotifyError::Config("webhook secret unreadable".into()))?;
            let body = serde_json::json!({
                "type": announcement.kind,
                "page": {
                    "slug": announcement.page_slug,
                    "title": announcement.page_title,
                    "url": announcement.page_url,
                },
                "incident": {
                    "id": announcement.incident_id,
                    "title": announcement.title,
                    "status": announcement.status,
                    "severity": announcement.severity,
                    "starts_at": announcement.starts_at,
                    "ends_at": announcement.ends_at,
                },
                "message": announcement.message,
                "unsubscribe_url": unsubscribe,
            });
            send_webhook(&subscriber.email, &secret, &announcement.kind, &body).await
        }
        "slack" => {
            let mut text = format!("*{headline}*\n");
            if let Some(extra) = announcement.message.as_deref().filter(|m| !m.trim().is_empty()) {
                text.push_str(extra.trim());
                text.push('\n');
            }
            text.push_str(&format!(
                "<{}|Status page> · <{unsubscribe}|Unsubscribe>",
                announcement.page_url
            ));
            send_slack(&subscriber.email, &text).await
        }
        other => Err(NotifyError::Config(format!("unknown subscriber kind {other}"))),
    }
}

// --------------------------------------------------------------------------
// File
// --------------------------------------------------------------------------

static RUNNING: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Traite les envois échus. Un seul passage à la fois : un second appelant
/// repart sans rien faire, la file est déjà en cours de vidage.
pub async fn process_due(state: &AppState) {
    let Ok(_guard) = RUNNING.try_lock() else { return };
    let now = Utc::now();
    let stamp = format_timestamp(now);
    let _ = sqlx::query("DELETE FROM status_deliveries WHERE created_at < ?")
        .bind(format_timestamp(now - MAX_AGE))
        .execute(&state.pool)
        .await;
    let due = match db::status_pages::due_deliveries(&state.pool, &stamp, BATCH).await {
        Ok(due) => due,
        Err(error) => {
            tracing::warn!(%error, "status page: delivery queue unreadable");
            return;
        }
    };
    for delivery in due {
        let subscriber =
            match db::status_pages::subscriber_by_id(&state.pool, delivery.subscriber_id).await {
                Ok(Some(subscriber)) if subscriber.confirmed_at.is_some() => subscriber,
                // Désabonné entre-temps : rien à envoyer.
                Ok(_) => {
                    let _ = db::status_pages::finish_delivery(&state.pool, delivery.id).await;
                    continue;
                }
                Err(error) => {
                    tracing::warn!(%error, "status page: subscriber unreadable");
                    continue;
                }
            };
        let Ok(announcement) = serde_json::from_str::<Announcement>(&delivery.payload) else {
            let _ = db::status_pages::finish_delivery(&state.pool, delivery.id).await;
            continue;
        };
        match deliver(state, &subscriber, &announcement).await {
            Ok(()) => {
                let _ = db::status_pages::finish_delivery(&state.pool, delivery.id).await;
            }
            Err(error) => {
                let attempt = delivery.attempts + 1;
                if attempt >= MAX_ATTEMPTS {
                    tracing::warn!(%error, kind = %subscriber.kind, "status page: delivery given up");
                    let _ = db::status_pages::finish_delivery(&state.pool, delivery.id).await;
                } else {
                    tracing::warn!(%error, kind = %subscriber.kind, attempt, "status page: delivery will be retried");
                    let wait = BACKOFF[(delivery.attempts as usize).min(BACKOFF.len() - 1)];
                    let next = format_timestamp(Utc::now() + TimeDelta::seconds(wait));
                    let _ = db::status_pages::retry_delivery(&state.pool, delivery.id, &next).await;
                }
            }
        }
        tokio::time::sleep(PAUSE).await;
    }
}

/// Reprise périodique des envois en attente.
pub fn spawn(state: AppState) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(30));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            process_due(&state).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phone_numbers_are_e164() {
        assert_eq!(normalise_phone("+33 6 12 34 56 78"), Some("+33612345678".into()));
        assert_eq!(normalise_phone("0612345678"), None);
        assert_eq!(normalise_phone("+0123456789"), None);
        assert_eq!(normalise_phone("+33abc"), None);
        assert_eq!(normalise_phone("+1234567"), None);
    }

    #[test]
    fn slack_urls_must_be_incoming_webhooks() {
        assert!(normalise_slack_url("https://hooks.slack.com/services/T0/B0/xyz").is_some());
        assert!(normalise_slack_url("http://hooks.slack.com/services/T0/B0/xyz").is_none());
        assert!(normalise_slack_url("https://evil.example/services/T0/B0/xyz").is_none());
        assert!(normalise_slack_url("https://hooks.slack.com.evil.example/services/a").is_none());
    }

    #[test]
    fn destinations_are_masked_for_admins() {
        assert_eq!(mask("sms", "+33612345678"), "+33•••••••78");
        assert_eq!(
            mask("slack", "https://hooks.slack.com/services/T0/B0/secret"),
            "https://hooks.slack.com/…"
        );
        assert_eq!(mask("email", "a@b.org"), "a@b.org");
    }

    #[test]
    fn signature_is_stable() {
        let a = sign("secret", 1_700_000_000, "{}");
        assert_eq!(a, sign("secret", 1_700_000_000, "{}"));
        assert_ne!(a, sign("secret", 1_700_000_001, "{}"));
        assert!(a.starts_with("sha256=") && a.len() == 7 + 64);
    }
}
