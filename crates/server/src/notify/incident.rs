//! Astreinte par webhook d'intégration : Grafana OnCall et SIGNL4.
//!
//! Comme PagerDuty, ces services ont un cycle de vie : une alerte ouvre un
//! incident et sa résolution le referme, rapprochées par un identifiant externe
//! stable (l'équipement, pas la règle). Le webhook contient le jeton : il est
//! déclaré secret et expurgé de toute erreur.

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::notify::Notifier;
use crate::notify::channel::ChannelConfig;
use crate::notify::error::NotifyError;
use crate::notify::http::{self, Request, check_url};
use crate::notify::message::{Message, clip};
use crate::notify::secret::SecretString;

/// Identifiant externe d'une alerte : le préfixe évite de refermer l'incident
/// d'un autre outil branché sur la même intégration.
fn external_id(message: &Message) -> String {
    format!("dumbmonit-{}", message.group_key())
}

// --------------------------------------------------------------------------
// Grafana OnCall
// --------------------------------------------------------------------------

/// Grafana OnCall, via une intégration « Webhook » (format JSON simple).
pub struct GrafanaOnCall {
    http: reqwest::Client,
    url: SecretString,
}

const ONCALL_MESSAGE_MAX: usize = 3000;

impl GrafanaOnCall {
    pub fn new(http: reqwest::Client, channel: &ChannelConfig) -> Result<Self, NotifyError> {
        let url = channel.secret("webhook_url")?;
        check_url(url.expose(), "webhook_url")?;
        Ok(Self { url, http })
    }

    fn payload(&self, message: &Message) -> Value {
        let mut body = json!({
            "alert_uid": external_id(message),
            "title": clip(&message.title, 250),
            // « alerting » ouvre ou met à jour le groupe, « ok » le résout.
            "state": if message.resolved { "ok" } else { "alerting" },
            "message": clip(&message.text, ONCALL_MESSAGE_MAX),
        });
        if let Some(link) = &message.link {
            body["link_to_upstream_details"] = json!(link);
        }
        body
    }

    fn explain(status: u16, _body: &str) -> Option<String> {
        match status {
            404 => Some(
                "Grafana OnCall does not know this integration URL: it may have been deleted"
                    .to_string(),
            ),
            _ => None,
        }
    }
}

#[async_trait]
impl Notifier for GrafanaOnCall {
    fn kind(&self) -> &'static str {
        "grafana_oncall"
    }

    async fn send(&self, message: &Message) -> Result<(), NotifyError> {
        http::send(
            &self.http,
            Request::post(self.url.expose())
                .json(self.payload(message))
                .secret(&self.url)
                .explain(Self::explain),
        )
        .await
    }
}

// --------------------------------------------------------------------------
// SIGNL4
// --------------------------------------------------------------------------

/// SIGNL4 : alerte mobile à l'équipe, via le webhook de l'équipe.
pub struct Signl4 {
    http: reqwest::Client,
    url: SecretString,
}

impl Signl4 {
    pub fn new(http: reqwest::Client, channel: &ChannelConfig) -> Result<Self, NotifyError> {
        let url = channel.secret("webhook_url")?;
        check_url(url.expose(), "webhook_url")?;
        Ok(Self { url, http })
    }

    fn payload(&self, message: &Message) -> Value {
        json!({
            "Title": clip(&message.title, 250),
            "Message": clip(&message.text, 3000),
            // Les champs « X-S4-* » pilotent SIGNL4 : « resolved » referme l'alerte
            // dont l'identifiant externe est le même.
            "X-S4-ExternalID": external_id(message),
            "X-S4-Status": if message.resolved { "resolved" } else { "new" },
            "X-S4-Service": message.target_name,
            "X-S4-SourceSystem": "DumbMonit",
            "X-S4-AlertingScenario": "single_ack",
        })
    }

    fn explain(status: u16, _body: &str) -> Option<String> {
        match status {
            401 | 403 | 404 => Some(
                "SIGNL4 rejected the team webhook: check the team secret in the URL".to_string(),
            ),
            _ => None,
        }
    }
}

#[async_trait]
impl Notifier for Signl4 {
    fn kind(&self) -> &'static str {
        "signl4"
    }

    async fn send(&self, message: &Message) -> Result<(), NotifyError> {
        http::send(
            &self.http,
            Request::post(self.url.expose())
                .json(self.payload(message))
                .secret(&self.url)
                .explain(Self::explain),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notify::channel::test_config;
    use crate::notify::message::sample_message;

    fn config(kind: &str) -> ChannelConfig {
        test_config(
            kind,
            json!({}),
            json!({"webhook_url": "https://example.org/integrations/v1/webhook/TOKEN/"}),
        )
    }

    #[test]
    fn grafana_oncall_ouvre_puis_resout_avec_le_meme_identifiant() {
        let notifier = GrafanaOnCall::new(reqwest::Client::new(), &config("grafana_oncall")).unwrap();
        let open = notifier.payload(&sample_message(false));
        let close = notifier.payload(&sample_message(true));
        assert_eq!(open["state"], "alerting");
        assert_eq!(close["state"], "ok");
        assert_eq!(open["alert_uid"], close["alert_uid"]);
        assert!(open["alert_uid"].as_str().unwrap().starts_with("dumbmonit-"));
    }

    #[test]
    fn signl4_pilote_le_cycle_de_vie_par_les_champs_x_s4() {
        let notifier = Signl4::new(reqwest::Client::new(), &config("signl4")).unwrap();
        let open = notifier.payload(&sample_message(false));
        let close = notifier.payload(&sample_message(true));
        assert_eq!(open["X-S4-Status"], "new");
        assert_eq!(close["X-S4-Status"], "resolved");
        assert_eq!(open["X-S4-ExternalID"], close["X-S4-ExternalID"]);
        assert_eq!(open["X-S4-Service"], "nas");
    }

    #[test]
    fn un_webhook_sans_schema_est_refuse_sans_etre_cite() {
        let bad = test_config("signl4", json!({}), json!({"webhook_url": "connect.signl4.com/TOKEN123"}));
        let Err(error) = Signl4::new(reqwest::Client::new(), &bad) else { panic!("accepted") };
        let text = error.to_string();
        assert!(text.contains("webhook_url") && !text.contains("TOKEN123"));
    }
}
