//! Canal « Web Push » : les alertes arrivent comme des notifications natives du
//! téléphone ou de l'ordinateur, sans application dédiée, à travers le service
//! de push du navigateur (Google, Mozilla, Apple, Microsoft).
//!
//! Le navigateur s'abonne depuis Réglages → Push notifications : il remet une
//! adresse chez son service de push et deux clés ([`store`]). Le serveur
//! chiffre chaque message pour ces clés (RFC 8291, `aes128gcm` de la RFC 8188,
//! par `web-push-native`), le signe avec sa paire VAPID ([`vapid`], RFC 8292),
//! et le dépose à l'adresse — que le garde de [`endpoint`] a vérifiée publique.
//! Le service de push le réveille côté appareil ; le service worker de
//! l'interface (`web/static/sw.js`) l'affiche, même onglet fermé.
//!
//! Contrairement aux autres canaux, celui-ci a besoin de la base : ses
//! destinataires sont les abonnements, pas une adresse inscrite dans ses
//! réglages. Il passe donc par [`deliver`] avec un [`Store`] ; le notificateur
//! construit par [`crate::notify::build`] ne sert qu'à valider ses réglages.

pub mod endpoint;
pub mod store;
pub mod vapid;

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use serde_json::json;
use sqlx::SqlitePool;
use web_push_native::{Auth, WebPushBuilder, p256};

use crate::alerting::model::Severity;
use crate::crypto::Cipher;
use crate::notify::channel::ChannelConfig;
use crate::notify::error::NotifyError;
use crate::notify::message::Message;
use crate::notify::secret;
use crate::notify::{Notifier, SEND_TIMEOUT};

pub use store::Audience;
use vapid::VapidKeys;

/// Identifiant du canal dans `notification_channels.kind`.
pub const KIND: &str = "webpush";

/// Page ouverte par un clic sur la notification. Relative : le service worker
/// la résout sur l'origine de l'interface, quelle que soit l'adresse publique.
pub const CLICK_URL: &str = "/alerts";

/// Contact VAPID par défaut, quand ni le canal ni `DUMBMONIT_PUBLIC_URL` (en
/// HTTPS) n'en donnent un. Apple refuse une signature sans contact valide.
pub const DEFAULT_CONTACT: &str = "https://github.com/laupernoe/dumbmonit";

/// Durée de conservation par défaut chez le service de push quand l'appareil
/// est éteint : au-delà d'une journée, une alerte n'a plus rien d'urgent.
pub const DEFAULT_TTL_SECS: u64 = 86_400;
const MAX_TTL_SECS: u64 = 28 * 86_400;

/// Priorités admises par l'en-tête `Urgency` (RFC 8030 §5.3).
pub const URGENCIES: &[&str] = &["high", "normal", "low"];

/// Plafond du corps de notification affiché. Le message chiffré doit tenir
/// sous 4 Ko chez tous les services ; le texte reste loin en dessous.
const MAX_BODY_CHARS: usize = 300;

/// Ce dont le canal a besoin de la base : les abonnements et la clé VAPID.
#[derive(Clone, Copy)]
pub struct Store<'a> {
    pub pool: &'a SqlitePool,
    pub cipher: &'a Cipher,
}

/// Réglages du canal.
#[derive(Debug, Clone)]
pub struct Settings {
    /// Comptes dont les appareils reçoivent le canal ; vide : tous.
    pub users: Vec<String>,
    pub urgency: String,
    pub ttl: Duration,
    pub contact: String,
}

impl Settings {
    pub fn from_config(config: &ChannelConfig) -> Result<Self, NotifyError> {
        let urgency = config.setting_opt("urgency").unwrap_or_else(|| "high".to_string());
        if !URGENCIES.contains(&urgency.as_str()) {
            return Err(NotifyError::Config(format!(
                "\"urgency\" must be one of {}",
                URGENCIES.join(", ")
            )));
        }
        let ttl = match config.settings.get("ttl") {
            None | Some(serde_json::Value::Null) => DEFAULT_TTL_SECS,
            Some(value) => value
                .as_u64()
                .or_else(|| value.as_str().and_then(|text| text.trim().parse().ok()))
                .filter(|ttl| *ttl <= MAX_TTL_SECS)
                .ok_or_else(|| {
                    NotifyError::Config(format!(
                        "\"ttl\" must be a number of seconds between 0 and {MAX_TTL_SECS}"
                    ))
                })?,
        };
        let contact = match config.setting_opt("contact") {
            Some(contact) => {
                let contact = contact.trim().to_string();
                if !contact.starts_with("mailto:") && !contact.starts_with("https://") {
                    return Err(NotifyError::Config(
                        "\"contact\" must start with mailto: or https://".into(),
                    ));
                }
                contact
            }
            None => default_contact(),
        };
        Ok(Self {
            users: config.setting_list("users"),
            urgency,
            ttl: Duration::from_secs(ttl),
            contact,
        })
    }

    /// Réglages d'un envoi de test lancé depuis les réglages d'un compte.
    pub fn for_test() -> Self {
        Self {
            users: Vec::new(),
            urgency: "high".into(),
            ttl: Duration::from_secs(600),
            contact: default_contact(),
        }
    }
}

/// L'adresse publique de l'instance si elle est en HTTPS, sinon le projet.
fn default_contact() -> String {
    crate::config::env_var("DUMBMONIT_PUBLIC_URL")
        .map(|url| url.trim().trim_end_matches('/').to_string())
        .filter(|url| url.starts_with("https://"))
        .unwrap_or_else(|| DEFAULT_CONTACT.to_string())
}

/// Notificateur de validation : il porte les réglages éprouvés, mais l'envoi
/// réel passe par [`deliver`], qui a accès à la base.
pub struct WebPush {
    _settings: Settings,
}

impl WebPush {
    pub fn new(config: &ChannelConfig) -> Result<Self, NotifyError> {
        Ok(Self { _settings: Settings::from_config(config)? })
    }
}

#[async_trait]
impl Notifier for WebPush {
    fn kind(&self) -> &'static str {
        KIND
    }

    async fn send(&self, _message: &Message) -> Result<(), NotifyError> {
        Err(NotifyError::Config(
            "Web Push needs the server database; it is sent by the alert engine".into(),
        ))
    }
}

/// Contenu remis au service worker, en JSON.
///
/// Le titre dit la gravité et l'appareil, le corps reste court : c'est ce
/// qu'un écran verrouillé affiche. `url` est relative, `tag` regroupe les
/// notifications d'une même alerte (la résolution remplace le déclenchement).
pub fn payload(message: &Message) -> serde_json::Value {
    let state = if message.resolved {
        "Resolved"
    } else {
        match message.severity {
            Severity::Critical => "Critical",
            Severity::Warning => "Warning",
            Severity::Info => "Info",
        }
    };
    let mut title = format!("{state} · {}", message.target_name);
    if message.count > 1 {
        title.push_str(&format!(" (+{})", message.count - 1));
    }
    let body: Vec<&str> =
        message.text.lines().map(str::trim).filter(|line| !line.is_empty()).take(4).collect();
    json!({
        "title": secret::truncate(&title, 120),
        "body": secret::truncate(&body.join("\n"), MAX_BODY_CHARS),
        "url": CLICK_URL,
        "tag": message.fingerprint,
        "severity": message.severity.as_str(),
        "resolved": message.resolved,
        "timestamp": message.at.timestamp_millis(),
    })
}

/// Bilan d'un envoi vers tous les appareils visés.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub delivered: usize,
    pub failed: usize,
    /// Abonnements expirés (404/410), supprimés au passage.
    pub removed: usize,
    /// Dernière erreur rencontrée, déjà expurgée.
    pub last_error: Option<String>,
}

impl Outcome {
    /// Traduit le bilan en résultat de canal : un appareil atteint suffit.
    pub fn into_result(self) -> Result<Self, NotifyError> {
        if self.delivered > 0 {
            return Ok(self);
        }
        if self.failed == 0 && self.removed == 0 {
            return Err(NotifyError::Config(
                "no device is subscribed: open DumbMonit on a phone or computer, then \
                 Settings → Push notifications → Enable on this device"
                    .into(),
            ));
        }
        if self.failed == 0 {
            return Err(NotifyError::Config(format!(
                "every subscribed device had expired ({} removed): enable push again on them",
                self.removed
            )));
        }
        Err(NotifyError::Transport(
            self.last_error.unwrap_or_else(|| "every push service refused the message".into()),
        ))
    }
}

/// Ce qu'un service de push a répondu pour un abonnement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Delivery {
    Delivered,
    /// 404 ou 410 : l'abonnement n'existe plus, il faut l'oublier.
    Gone,
    Failed(String),
}

/// Lecture du code de retour d'un service de push.
pub fn classify(status: u16, detail: &str) -> Delivery {
    match status {
        200..=299 => Delivery::Delivered,
        404 | 410 => Delivery::Gone,
        413 => Delivery::Failed("the push service refused the message as too large".into()),
        401 | 403 => Delivery::Failed(format!(
            "the push service refused the VAPID signature ({status}){}",
            excerpt(detail)
        )),
        _ => Delivery::Failed(format!("the push service answered {status}{}", excerpt(detail))),
    }
}

fn excerpt(detail: &str) -> String {
    let detail = detail.trim();
    if detail.is_empty() { String::new() } else { format!(": {}", secret::truncate(detail, 120)) }
}

/// Client HTTP des envois : HTTPS seulement, pas de redirection, et un
/// résolveur qui refuse toute adresse non publique au moment de la connexion.
pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(SEND_TIMEOUT)
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .dns_resolver(Arc::new(endpoint::PublicOnlyResolver))
        .user_agent(concat!("DumbMonit/", env!("CARGO_PKG_VERSION")))
        .build()
        .unwrap_or_default()
}

/// Envoie le canal : chaque abonnement visé par ses réglages reçoit le message.
pub async fn deliver(
    store: Store<'_>,
    config: &ChannelConfig,
    message: &Message,
) -> Result<Outcome, NotifyError> {
    let settings = Settings::from_config(config)?;
    send(store, &client(), &Audience::Users(&settings.users), &settings, message)
        .await?
        .into_result()
}

/// Envoie un message à une audience, et tient la base à jour : dernier succès,
/// dernière erreur, et suppression des abonnements expirés.
pub async fn send(
    store: Store<'_>,
    http: &reqwest::Client,
    audience: &Audience<'_>,
    settings: &Settings,
    message: &Message,
) -> Result<Outcome, NotifyError> {
    let internal = |error: anyhow::Error| NotifyError::Config(format!("{error:#}"));
    let keys = vapid::load_or_create(store.pool, store.cipher).await.map_err(internal)?;
    let subscriptions =
        store::recipients(store.pool, store.cipher, audience).await.map_err(internal)?;
    let body = payload(message).to_string();

    let mut outcome = Outcome::default();
    for subscription in &subscriptions {
        let delivery = send_one(http, &keys, subscription, settings, &body).await;
        record(store.pool, subscription.id, &delivery, &mut outcome).await;
    }
    Ok(outcome)
}

/// Consigne la réponse du service de push pour un abonnement : dernier succès,
/// dernière erreur, ou suppression d'un abonnement expiré (404/410) — sans quoi
/// chaque alerte retenterait indéfiniment un navigateur désinstallé.
pub async fn record(pool: &SqlitePool, id: i64, delivery: &Delivery, outcome: &mut Outcome) {
    let recorded = match delivery {
        Delivery::Delivered => {
            outcome.delivered += 1;
            store::mark_success(pool, id, Utc::now()).await
        }
        Delivery::Gone => {
            outcome.removed += 1;
            tracing::info!(subscription = id, "expired push subscription removed");
            store::delete(pool, id).await
        }
        Delivery::Failed(error) => {
            outcome.failed += 1;
            outcome.last_error = Some(error.clone());
            store::mark_error(pool, id, error).await
        }
    };
    if let Err(error) = recorded {
        tracing::warn!(?error, "push subscription state not recorded");
    }
}

/// Un envoi vers un abonnement. Aucune erreur ne cite l'adresse complète :
/// elle porte le jeton de l'abonnement.
async fn send_one(
    http: &reqwest::Client,
    keys: &VapidKeys,
    subscription: &store::Subscription,
    settings: &Settings,
    body: &str,
) -> Delivery {
    let url = match endpoint::validate(&subscription.endpoint) {
        Ok(url) => url,
        Err(reason) => return Delivery::Failed(reason),
    };
    let Ok(ua_public) = p256::PublicKey::from_sec1_bytes(&subscription.p256dh) else {
        return Delivery::Failed("the browser key of this subscription is invalid".into());
    };
    if subscription.auth.len() != 16 {
        return Delivery::Failed("the browser secret of this subscription is invalid".into());
    }
    let auth = Auth::clone_from_slice(&subscription.auth);
    let Ok(uri) = subscription.endpoint.parse() else {
        return Delivery::Failed("the push endpoint is not a valid URL".into());
    };

    let request = match WebPushBuilder::new(uri, ua_public, auth)
        .with_valid_duration(settings.ttl)
        .build(body.as_bytes().to_vec())
    {
        Ok(request) => request,
        Err(error) => return Delivery::Failed(format!("encryption failed: {error}")),
    };
    let authorization = keys.authorization(&url, &settings.contact, Utc::now().timestamp());

    let (parts, content) = request.into_parts();
    // `TTL`, `Content-Encoding: aes128gcm` et le type du corps viennent de
    // `web-push-native` ; la signature et la priorité sont ajoutées ici.
    let builder = http
        .post(url)
        .headers(parts.headers)
        .body(content)
        .timeout(SEND_TIMEOUT)
        .header(reqwest::header::AUTHORIZATION, authorization)
        .header("Urgency", settings.urgency.as_str());

    match builder.send().await {
        Ok(response) => {
            let status = response.status().as_u16();
            let detail = if response.status().is_success() {
                String::new()
            } else {
                response.text().await.unwrap_or_default()
            };
            classify(status, &detail)
        }
        // `without_url` : l'adresse de l'abonnement ne doit pas finir dans la
        // colonne `last_error`, ni dans un journal.
        Err(error) => {
            Delivery::Failed(format!("push service unreachable: {}", error.without_url()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notify::channel::test_config;
    use crate::notify::message::test_message;

    #[test]
    fn les_reglages_par_defaut_suffisent() {
        let settings = Settings::from_config(&test_config(KIND, json!({}), json!({}))).unwrap();
        assert!(settings.users.is_empty());
        assert_eq!(settings.urgency, "high");
        assert_eq!(settings.ttl, Duration::from_secs(DEFAULT_TTL_SECS));
        assert!(
            settings.contact.starts_with("https://") || settings.contact.starts_with("mailto:")
        );
    }

    #[test]
    fn les_reglages_invalides_sont_expliques() {
        for (settings, key) in [
            (json!({"urgency": "now"}), "urgency"),
            (json!({"ttl": -1}), "ttl"),
            (json!({"ttl": 999_999_999}), "ttl"),
            (json!({"contact": "ops@example.com"}), "contact"),
        ] {
            let Err(error) = Settings::from_config(&test_config(KIND, settings, json!({}))) else {
                panic!("{key} should be refused")
            };
            assert!(error.to_string().contains(key), "{error}");
        }
        let ok = json!({"users": ["admin"], "urgency": "normal", "ttl": "600",
                        "contact": "mailto:ops@example.com"});
        let settings = Settings::from_config(&test_config(KIND, ok, json!({}))).unwrap();
        assert_eq!(settings.users, vec!["admin"]);
        assert_eq!(settings.ttl, Duration::from_secs(600));
    }

    #[test]
    fn le_contenu_dit_la_gravite_et_l_appareil_et_ouvre_les_alertes() {
        let mut message = test_message("phone");
        message.severity = Severity::Critical;
        message.target_name = "nas01".into();
        message.text = "Disk /volume1 above 95 %\n\nsecond line".into();
        let json = payload(&message);
        assert_eq!(json["title"], "Critical · nas01");
        assert_eq!(json["body"], "Disk /volume1 above 95 %\nsecond line");
        assert_eq!(json["url"], "/alerts", "relative URL");
        assert_eq!(json["tag"], message.fingerprint);

        message.resolved = true;
        message.count = 3;
        assert_eq!(payload(&message)["title"], "Resolved · nas01 (+2)");

        message.text = "x".repeat(5_000);
        let body = payload(&message)["body"].as_str().unwrap().chars().count();
        assert!(body <= MAX_BODY_CHARS + 1, "body kept short");
    }

    #[test]
    fn les_codes_de_retour_sont_lus_comme_le_prevoit_la_rfc_8030() {
        assert_eq!(classify(201, ""), Delivery::Delivered);
        assert_eq!(classify(404, ""), Delivery::Gone);
        assert_eq!(classify(410, "push subscription has unsubscribed or expired"), Delivery::Gone);
        assert!(matches!(classify(413, ""), Delivery::Failed(e) if e.contains("too large")));
        assert!(matches!(classify(403, "BadJwtToken"), Delivery::Failed(e) if e.contains("VAPID")));
        assert!(matches!(classify(503, ""), Delivery::Failed(e) if e.contains("503")));
    }

    #[tokio::test]
    async fn un_abonnement_expire_410_est_supprime_les_autres_sont_suivis() {
        let dir = tempfile::tempdir().unwrap();
        let pool = crate::db::open(&dir.path().join("db.sqlite")).await.unwrap();
        let cipher =
            crate::db::init_cipher(&pool, "a-test-secret-that-is-long-enough").await.unwrap();
        let user_id: i64 = sqlx::query_scalar(
            "INSERT INTO users (username, role) VALUES ('alice', 'viewer') RETURNING id",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let mut ids = Vec::new();
        for n in 0..3 {
            let endpoint = format!("https://push.example.com/sub/{n}");
            let device = store::upsert(&pool, &cipher, user_id, &endpoint, &[4; 65], &[1; 16], "x")
                .await
                .unwrap();
            ids.push(device.id);
        }

        let mut outcome = Outcome::default();
        record(&pool, ids[0], &classify(201, ""), &mut outcome).await;
        record(&pool, ids[1], &classify(500, "oops"), &mut outcome).await;
        record(&pool, ids[2], &classify(410, "Gone"), &mut outcome).await;
        assert_eq!((outcome.delivered, outcome.failed, outcome.removed), (1, 1, 1));

        let devices = store::list_for_user(&pool, user_id).await.unwrap();
        assert_eq!(devices.len(), 2, "the expired subscription is gone");
        assert!(devices.iter().all(|d| d.id != ids[2]));
        let ok = devices.iter().find(|d| d.id == ids[0]).unwrap();
        assert!(ok.last_success_at.is_some() && ok.last_error.is_none());
        let failed = devices.iter().find(|d| d.id == ids[1]).unwrap();
        assert!(failed.last_error.as_deref().is_some_and(|e| e.contains("500")));

        // 404 vaut 410 : même suppression.
        let mut outcome = Outcome::default();
        record(&pool, ids[1], &classify(404, ""), &mut outcome).await;
        assert_eq!(store::list_for_user(&pool, user_id).await.unwrap().len(), 1);

        // Les clés relues sont bien celles enregistrées, et chiffrées en base.
        let recipients = store::recipients(&pool, &cipher, &Audience::Users(&[])).await.unwrap();
        assert_eq!(recipients.len(), 1);
        assert_eq!(recipients[0].auth, vec![1; 16]);
        let stored: Vec<u8> = sqlx::query_scalar("SELECT auth_enc FROM webpush_subscriptions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_ne!(stored, vec![1; 16]);
        assert!(
            store::recipients(&pool, &cipher, &Audience::Users(&["bob".into()]))
                .await
                .unwrap()
                .is_empty(),
            "filtered by account"
        );
    }

    #[tokio::test]
    async fn la_migration_libere_les_types_de_canaux_sans_rien_perdre() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("db.sqlite");
        let old = crate::db::open_up_to(&path, 41).await.unwrap();
        sqlx::query("INSERT INTO notification_channels (id, name, kind) VALUES (7, 'Ops', 'ntfy')")
            .execute(&old)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO notify_queue (channel_id, fingerprint, target_name, hold, item, queued_at)
             VALUES (7, 'fp', 'nas', 'batch', '{}', '2026-10-07 10:00:00')",
        )
        .execute(&old)
        .await
        .unwrap();
        let refused =
            sqlx::query("INSERT INTO notification_channels (name, kind) VALUES ('M', 'matrix')")
                .execute(&old)
                .await;
        assert!(refused.is_err(), "the old constraint only knew seven kinds");
        old.close().await;

        let pool = crate::db::open(&path).await.unwrap();
        let kind: String =
            sqlx::query_scalar("SELECT kind FROM notification_channels WHERE id = 7")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(kind, "ntfy");
        let queued: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM notify_queue WHERE channel_id = 7")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(queued, 1, "the queue survives the rebuild");
        for kind in ["matrix", KIND] {
            sqlx::query("INSERT INTO notification_channels (name, kind) VALUES (?, ?)")
                .bind(format!("channel {kind}"))
                .bind(kind)
                .execute(&pool)
                .await
                .unwrap();
        }
        let fk: bool = sqlx::query_scalar("PRAGMA foreign_keys").fetch_one(&pool).await.unwrap();
        assert!(fk, "foreign keys are back on");
        sqlx::query("DELETE FROM notification_channels WHERE id = 7").execute(&pool).await.unwrap();
        let queued: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM notify_queue").fetch_one(&pool).await.unwrap();
        assert_eq!(queued, 0, "the cascade still points at the rebuilt table");
    }

    #[test]
    fn le_bilan_dit_pourquoi_rien_n_est_parti() {
        assert!(Outcome { delivered: 1, failed: 2, ..Outcome::default() }.into_result().is_ok());
        let none = Outcome::default().into_result().unwrap_err();
        assert!(none.to_string().contains("no device is subscribed"), "{none}");
        let expired = Outcome { removed: 2, ..Outcome::default() }.into_result().unwrap_err();
        assert!(expired.to_string().contains("expired"), "{expired}");
        let failed = Outcome { failed: 1, last_error: Some("boom".into()), ..Outcome::default() }
            .into_result()
            .unwrap_err();
        assert!(failed.is_transient(), "a refused push may work next cycle");
    }
}
