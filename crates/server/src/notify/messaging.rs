//! Messageries et SMS supplémentaires : WhatsApp (API Cloud de Meta, CallMeBot),
//! Threema Gateway, Webex, Vonage, Telnyx et ASPSMS.
//!
//! Même discipline que le reste du module : corps tronqués aux limites de chaque
//! service, secrets déclarés à [`Request`] pour être expurgés de toute erreur, et
//! aucune valeur de secret dans un message de configuration. Les SMS sont facturés
//! au message : le texte est réduit au strict nécessaire et les liens d'action
//! signés (trop longs pour un SMS) sont retirés, voir
//! [`crate::notify::catalog::action_style`].

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::notify::Notifier;
use crate::notify::channel::ChannelConfig;
use crate::notify::error::NotifyError;
use crate::notify::http::{self, Request};
use crate::notify::message::{Message, clip};
use crate::notify::secret::SecretString;

/// Version de l'API Graph de Meta utilisée pour WhatsApp.
const GRAPH_VERSION: &str = "v23.0";

/// Corps d'un SMS : sans émoji (qui force l'encodage UCS-2 et divise la capacité
/// d'un segment par deux), titre et détail sur une ligne.
fn sms_body(message: &Message, max: usize) -> String {
    let flat: String = format!("{} - {}", message.title, message.text)
        .chars()
        .filter(|c| !matches!(u32::from(*c), 0x2300..=0x2BFF | 0x1F000..=0x1FFFF | 0xFE0F))
        .map(|c| if c == '\n' { ' ' } else { c })
        .collect();
    clip(flat.trim(), max)
}

/// Destinataires obligatoires, au moins un.
fn recipients(channel: &ChannelConfig, example: &str) -> Result<Vec<String>, NotifyError> {
    let to: Vec<String> =
        channel.setting_list("to").iter().map(|n| n.trim().to_string()).collect();
    if to.is_empty() {
        return Err(NotifyError::Config(format!(
            "\"to\" must contain at least one recipient, for example {example}"
        )));
    }
    Ok(to)
}

/// Numéro sans `+`, espaces ni tirets : la forme que Meta et Vonage attendent.
fn digits(number: &str) -> String {
    number.chars().filter(char::is_ascii_digit).collect()
}

// --------------------------------------------------------------------------
// WhatsApp (API Cloud de Meta)
// --------------------------------------------------------------------------

/// WhatsApp Business Cloud API.
///
/// Meta n'autorise un texte libre que dans les 24 h qui suivent un message du
/// destinataire : le destinataire doit d'abord écrire au numéro de l'entreprise
/// (ou l'on passe par un modèle approuvé, hors du périmètre de ce canal).
pub struct WhatsApp {
    http: reqwest::Client,
    url: String,
    token: SecretString,
    to: Vec<String>,
}

/// Limite documentée du corps d'un message texte.
const WHATSAPP_BODY_MAX: usize = 4096;

impl WhatsApp {
    pub fn new(http: reqwest::Client, channel: &ChannelConfig) -> Result<Self, NotifyError> {
        let phone_number_id = channel.setting("phone_number_id")?;
        if !phone_number_id.chars().all(|c| c.is_ascii_digit()) {
            return Err(NotifyError::Config(
                "\"phone_number_id\" is the numeric identifier shown in the Meta console, not \
                 the phone number itself"
                    .to_string(),
            ));
        }
        let to: Vec<String> =
            recipients(channel, "+33612345678")?.iter().map(|n| digits(n)).collect();
        Ok(Self {
            url: format!("https://graph.facebook.com/{GRAPH_VERSION}/{phone_number_id}/messages"),
            token: channel.secret("token")?,
            to,
            http,
        })
    }

    fn payload(&self, message: &Message, recipient: &str) -> Value {
        json!({
            "messaging_product": "whatsapp",
            "recipient_type": "individual",
            "to": recipient,
            "type": "text",
            "text": {
                "preview_url": false,
                "body": clip(
                    &format!("{} {}\n{}", message.emoji(), message.title, message.text),
                    WHATSAPP_BODY_MAX,
                ),
            },
        })
    }

    fn explain(status: u16, body: &str) -> Option<String> {
        match status {
            401 => Some("Meta rejected the access token: it is invalid or expired".to_string()),
            // Code 131047 : hors fenêtre de 24 h ; la cause la plus fréquente d'un 400.
            400 if body.contains("131047") || body.contains("131026") => Some(
                "WhatsApp only delivers free text within 24 h of the recipient's last message: \
                 ask them to write to the business number first"
                    .to_string(),
            ),
            400 | 404 => Some(
                "Meta rejected the message: check the phone number ID and the recipient number"
                    .to_string(),
            ),
            _ => None,
        }
    }
}

#[async_trait]
impl Notifier for WhatsApp {
    fn kind(&self) -> &'static str {
        "whatsapp"
    }

    async fn send(&self, message: &Message) -> Result<(), NotifyError> {
        for recipient in &self.to {
            http::send(
                &self.http,
                Request::post(&self.url)
                    .json(self.payload(message, recipient))
                    .bearer(&self.token)
                    .explain(Self::explain),
            )
            .await?;
        }
        Ok(())
    }
}

// --------------------------------------------------------------------------
// WhatsApp via CallMeBot
// --------------------------------------------------------------------------

/// WhatsApp via CallMeBot : passerelle gratuite, sans compte Meta, où chaque
/// destinataire active lui-même sa clé en écrivant au bot.
pub struct CallMeBot {
    http: reqwest::Client,
    phone: String,
    api_key: SecretString,
}

const CALLMEBOT_BODY_MAX: usize = 1000;

impl CallMeBot {
    pub fn new(http: reqwest::Client, channel: &ChannelConfig) -> Result<Self, NotifyError> {
        Ok(Self { phone: channel.setting("phone")?, api_key: channel.secret("api_key")?, http })
    }

    fn query(&self, message: &Message) -> Vec<(String, String)> {
        vec![
            ("phone".to_string(), self.phone.clone()),
            (
                "text".to_string(),
                clip(
                    &format!("{} {}\n{}", message.emoji(), message.title, message.text),
                    CALLMEBOT_BODY_MAX,
                ),
            ),
            ("apikey".to_string(), self.api_key.expose().to_string()),
        ]
    }

    fn explain(status: u16, _body: &str) -> Option<String> {
        match status {
            203 | 401 | 403 => Some(
                "CallMeBot rejected the API key: send \"I allow callmebot to send me messages\" \
                 to the bot again to get a new one"
                    .to_string(),
            ),
            _ => None,
        }
    }
}

#[async_trait]
impl Notifier for CallMeBot {
    fn kind(&self) -> &'static str {
        "callmebot"
    }

    async fn send(&self, message: &Message) -> Result<(), NotifyError> {
        let mut request = Request::new(reqwest::Method::GET, "https://api.callmebot.com/whatsapp.php")
            .secret(&self.api_key)
            .explain(Self::explain);
        for (name, value) in self.query(message) {
            request = request.query_pair(name, value);
        }
        http::send(&self.http, request).await
    }
}

// --------------------------------------------------------------------------
// Threema Gateway (mode basique)
// --------------------------------------------------------------------------

/// Threema Gateway, mode basique : le serveur de Threema chiffre le message, ce
/// qui suffit à une alerte. Le mode « de bout en bout » exige une paire de clés
/// et n'apporte rien ici.
pub struct Threema {
    http: reqwest::Client,
    from: String,
    secret: SecretString,
    to: Vec<String>,
}

/// Limite du service : 3500 octets ; 1000 caractères restent toujours en dessous.
const THREEMA_BODY_MAX: usize = 1000;

impl Threema {
    pub fn new(http: reqwest::Client, channel: &ChannelConfig) -> Result<Self, NotifyError> {
        let from = channel.setting("from")?;
        // Un ID de Gateway fait huit caractères et commence par « * ».
        if from.len() != 8 || !from.starts_with('*') {
            return Err(NotifyError::Config(
                "\"from\" is your Gateway ID: eight characters starting with \"*\"".to_string(),
            ));
        }
        let to = recipients(channel, "ECHOECHO")?;
        if to.iter().any(|id| id.len() != 8) {
            return Err(NotifyError::Config(
                "each Threema ID in \"to\" has exactly eight characters".to_string(),
            ));
        }
        Ok(Self { from, secret: channel.secret("secret")?, to, http })
    }

    fn form(&self, message: &Message, recipient: &str) -> Vec<(String, String)> {
        vec![
            ("from".to_string(), self.from.clone()),
            ("to".to_string(), recipient.to_string()),
            ("secret".to_string(), self.secret.expose().to_string()),
            (
                "text".to_string(),
                clip(
                    &format!("{} {}\n{}", message.emoji(), message.title, message.text),
                    THREEMA_BODY_MAX,
                ),
            ),
        ]
    }

    fn explain(status: u16, _body: &str) -> Option<String> {
        Some(
            match status {
                401 => "Threema rejected the Gateway ID or its secret",
                402 => "the Threema Gateway account has no credits left",
                404 => "this Threema ID does not exist or cannot receive messages",
                413 => "the message is too long for Threema",
                _ => return None,
            }
            .to_string(),
        )
    }
}

#[async_trait]
impl Notifier for Threema {
    fn kind(&self) -> &'static str {
        "threema"
    }

    async fn send(&self, message: &Message) -> Result<(), NotifyError> {
        for recipient in &self.to {
            http::send(
                &self.http,
                Request::post("https://msgapi.threema.ch/send_simple")
                    .form(self.form(message, recipient))
                    .secret(&self.secret)
                    .explain(Self::explain),
            )
            .await?;
        }
        Ok(())
    }
}

// --------------------------------------------------------------------------
// Webex
// --------------------------------------------------------------------------

/// Webex : message d'un bot dans un espace.
pub struct Webex {
    http: reqwest::Client,
    token: SecretString,
    room_id: String,
}

/// Limite documentée : 7439 octets de Markdown.
const WEBEX_BODY_MAX: usize = 3000;

impl Webex {
    pub fn new(http: reqwest::Client, channel: &ChannelConfig) -> Result<Self, NotifyError> {
        Ok(Self { room_id: channel.setting("room_id")?, token: channel.secret("token")?, http })
    }

    fn payload(&self, message: &Message) -> Value {
        json!({ "roomId": self.room_id, "markdown": clip(&message.markdown, WEBEX_BODY_MAX) })
    }

    fn explain(status: u16, _body: &str) -> Option<String> {
        Some(
            match status {
                401 => "Webex rejected the bot token: it is invalid or was regenerated",
                403 | 404 => {
                    "Webex cannot reach this space: the bot must be a member of the room and \
                     the room ID must be correct"
                }
                _ => return None,
            }
            .to_string(),
        )
    }
}

#[async_trait]
impl Notifier for Webex {
    fn kind(&self) -> &'static str {
        "webex"
    }

    async fn send(&self, message: &Message) -> Result<(), NotifyError> {
        http::send(
            &self.http,
            Request::post("https://webexapis.com/v1/messages")
                .json(self.payload(message))
                .bearer(&self.token)
                .explain(Self::explain),
        )
        .await
    }
}

// --------------------------------------------------------------------------
// Vonage (SMS)
// --------------------------------------------------------------------------

/// SMS via Vonage. L'API répond 200 même quand elle refuse un message : l'échec
/// est dans le champ `status` de chaque élément de `messages`.
pub struct Vonage {
    http: reqwest::Client,
    api_key: String,
    api_secret: SecretString,
    from: String,
    to: Vec<String>,
}

const SMS_BODY_MAX: usize = 300;

impl Vonage {
    pub fn new(http: reqwest::Client, channel: &ChannelConfig) -> Result<Self, NotifyError> {
        let to: Vec<String> =
            recipients(channel, "+33612345678")?.iter().map(|n| digits(n)).collect();
        Ok(Self {
            api_key: channel.setting("api_key")?,
            api_secret: channel.secret("api_secret")?,
            from: channel.setting("from")?,
            to,
            http,
        })
    }

    fn form(&self, message: &Message, recipient: &str) -> Vec<(String, String)> {
        vec![
            ("api_key".to_string(), self.api_key.clone()),
            ("api_secret".to_string(), self.api_secret.expose().to_string()),
            ("from".to_string(), self.from.clone()),
            ("to".to_string(), recipient.to_string()),
            ("text".to_string(), sms_body(message, SMS_BODY_MAX)),
            ("type".to_string(), "unicode".to_string()),
        ]
    }

    /// Lit la réponse : `status` vaut « 0 » pour chaque SMS accepté.
    fn verify(body: &str) -> Option<String> {
        let parsed: Value = serde_json::from_str(body).ok()?;
        let messages = parsed.get("messages")?.as_array()?;
        messages.iter().find_map(|entry| {
            let status = entry.get("status").and_then(Value::as_str).unwrap_or("0");
            if status == "0" {
                return None;
            }
            let detail = entry.get("error-text").and_then(Value::as_str).unwrap_or("rejected");
            Some(format!("Vonage refused the SMS (status {status}): {detail}"))
        })
    }
}

#[async_trait]
impl Notifier for Vonage {
    fn kind(&self) -> &'static str {
        "vonage"
    }

    async fn send(&self, message: &Message) -> Result<(), NotifyError> {
        for recipient in &self.to {
            http::send(
                &self.http,
                Request::post("https://rest.nexmo.com/sms/json")
                    .form(self.form(message, recipient))
                    .secret(&self.api_secret)
                    .verify(Self::verify),
            )
            .await?;
        }
        Ok(())
    }
}

// --------------------------------------------------------------------------
// Telnyx (SMS)
// --------------------------------------------------------------------------

/// SMS via Telnyx, API v2.
pub struct Telnyx {
    http: reqwest::Client,
    api_key: SecretString,
    from: String,
    to: Vec<String>,
}

impl Telnyx {
    pub fn new(http: reqwest::Client, channel: &ChannelConfig) -> Result<Self, NotifyError> {
        let to = recipients(channel, "+33612345678")?;
        if to.iter().any(|n| !n.starts_with('+')) {
            return Err(NotifyError::Config(
                "Telnyx wants E.164 numbers in \"to\": a \"+\", the country code, then the number"
                    .to_string(),
            ));
        }
        Ok(Self { from: channel.setting("from")?, api_key: channel.secret("api_key")?, to, http })
    }

    fn payload(&self, message: &Message, recipient: &str) -> Value {
        json!({ "from": self.from, "to": recipient, "text": sms_body(message, SMS_BODY_MAX) })
    }

    fn explain(status: u16, _body: &str) -> Option<String> {
        Some(
            match status {
                401 => "Telnyx rejected the API key",
                403 => "this Telnyx number or profile is not allowed to send to this destination",
                422 => "Telnyx rejected the message: invalid number, or sender not SMS-enabled",
                _ => return None,
            }
            .to_string(),
        )
    }
}

#[async_trait]
impl Notifier for Telnyx {
    fn kind(&self) -> &'static str {
        "telnyx"
    }

    async fn send(&self, message: &Message) -> Result<(), NotifyError> {
        for recipient in &self.to {
            http::send(
                &self.http,
                Request::post("https://api.telnyx.com/v2/messages")
                    .json(self.payload(message, recipient))
                    .bearer(&self.api_key)
                    .explain(Self::explain),
            )
            .await?;
        }
        Ok(())
    }
}

// --------------------------------------------------------------------------
// ASPSMS
// --------------------------------------------------------------------------

/// SMS via ASPSMS (JSON). Comme Vonage, le service répond 200 et loge le résultat
/// dans `StatusCode` (« 1 » : envoyé).
pub struct AspSms {
    http: reqwest::Client,
    user: String,
    password: SecretString,
    originator: String,
    to: Vec<String>,
}

impl AspSms {
    pub fn new(http: reqwest::Client, channel: &ChannelConfig) -> Result<Self, NotifyError> {
        Ok(Self {
            user: channel.setting("userkey")?,
            password: channel.secret("password")?,
            originator: channel.setting("originator")?,
            to: recipients(channel, "+41791234567")?,
            http,
        })
    }

    fn payload(&self, message: &Message) -> Value {
        json!({
            "UserName": self.user,
            "Password": self.password.expose(),
            "Originator": self.originator,
            "Recipients": self.to,
            "MessageText": sms_body(message, SMS_BODY_MAX),
        })
    }

    fn verify(body: &str) -> Option<String> {
        let parsed: Value = serde_json::from_str(body).ok()?;
        let code = parsed.get("StatusCode").and_then(|c| match c {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            _ => None,
        })?;
        if code == "1" {
            return None;
        }
        let info = parsed.get("StatusInfo").and_then(Value::as_str).unwrap_or("rejected");
        Some(format!("ASPSMS refused the SMS (code {code}): {info}"))
    }
}

#[async_trait]
impl Notifier for AspSms {
    fn kind(&self) -> &'static str {
        "aspsms"
    }

    async fn send(&self, message: &Message) -> Result<(), NotifyError> {
        http::send(
            &self.http,
            Request::post("https://json.aspsms.com/SendSimpleTextSMS")
                .json(self.payload(message))
                .secret(&self.password)
                .verify(Self::verify),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notify::channel::test_config;
    use crate::notify::message::sample_message;

    fn client() -> reqwest::Client {
        reqwest::Client::new()
    }

    fn refus<T>(result: Result<T, NotifyError>) -> String {
        match result {
            Err(erreur) => erreur.to_string(),
            Ok(_) => panic!("invalid configuration accepted"),
        }
    }

    #[test]
    fn whatsapp_cible_l_api_graph_avec_des_numeros_nus() {
        let config = test_config(
            "whatsapp",
            json!({"phone_number_id": "1234567890", "to": ["+33 6 12 34 56 78"]}),
            json!({"token": "EAAGjeton"}),
        );
        let notifier = WhatsApp::new(client(), &config).unwrap();
        assert_eq!(
            notifier.url,
            "https://graph.facebook.com/v23.0/1234567890/messages"
        );
        assert_eq!(notifier.to, vec!["33612345678"]);
        let body = notifier.payload(&sample_message(false), "33612345678");
        assert_eq!(body["messaging_product"], "whatsapp");
        assert_eq!(body["type"], "text");
        assert_eq!(body["to"], "33612345678");
        assert!(body["text"]["body"].as_str().unwrap().contains("Disk full"));
    }

    #[test]
    fn whatsapp_refuse_un_numero_de_telephone_a_la_place_de_l_identifiant() {
        let config = test_config(
            "whatsapp",
            json!({"phone_number_id": "+33612345678", "to": ["33612345678"]}),
            json!({"token": "t"}),
        );
        assert!(refus(WhatsApp::new(client(), &config)).contains("phone_number_id"));
    }

    #[test]
    fn whatsapp_explique_la_fenetre_de_24_heures() {
        let text = WhatsApp::explain(400, r#"{"error":{"code":131047}}"#).unwrap();
        assert!(text.contains("24 h"));
    }

    #[test]
    fn callmebot_place_la_cle_dans_la_requete_sans_la_laisser_fuiter() {
        let config = test_config(
            "callmebot",
            json!({"phone": "+34123456789"}),
            json!({"api_key": "123456"}),
        );
        let notifier = CallMeBot::new(client(), &config).unwrap();
        let query = notifier.query(&sample_message(false));
        let names: Vec<&str> = query.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["phone", "text", "apikey"]);
        assert_eq!(query[0].1, "+34123456789");
    }

    #[test]
    fn threema_exige_des_identifiants_de_huit_caracteres() {
        let ok = test_config(
            "threema",
            json!({"from": "*MYGWAY1", "to": ["ECHOECHO"]}),
            json!({"secret": "s3cret"}),
        );
        let notifier = Threema::new(client(), &ok).unwrap();
        let form = notifier.form(&sample_message(false), "ECHOECHO");
        let get = |k: &str| form.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
        assert_eq!(get("from"), Some("*MYGWAY1"));
        assert_eq!(get("to"), Some("ECHOECHO"));
        assert_eq!(get("secret"), Some("s3cret"));
        let bad = test_config(
            "threema",
            json!({"from": "MYGWAY1", "to": ["ECHOECHO"]}),
            json!({"secret": "s"}),
        );
        assert!(refus(Threema::new(client(), &bad)).contains("Gateway ID"));
        let bad_to =
            test_config("threema", json!({"from": "*MYGWAY1", "to": ["ABC"]}), json!({"secret": "s"}));
        assert!(refus(Threema::new(client(), &bad_to)).contains("eight characters"));
    }

    #[test]
    fn webex_envoie_du_markdown_dans_l_espace_choisi() {
        let config =
            test_config("webex", json!({"room_id": "Y2lzY29zcGFyazovL3Jvb20"}), json!({"token": "t"}));
        let body = Webex::new(client(), &config).unwrap().payload(&sample_message(false));
        assert_eq!(body["roomId"], "Y2lzY29zcGFyazovL3Jvb20");
        assert!(body["markdown"].as_str().unwrap().starts_with("**nas"));
    }

    #[test]
    fn vonage_produit_le_formulaire_et_lit_le_statut_du_corps() {
        let config = test_config(
            "vonage",
            json!({"api_key": "ab12cd34", "from": "DumbMonit", "to": ["+33 6 12 34 56 78"]}),
            json!({"api_secret": "shh"}),
        );
        let notifier = Vonage::new(client(), &config).unwrap();
        assert_eq!(notifier.to, vec!["33612345678"]);
        let form = notifier.form(&sample_message(false), "33612345678");
        assert!(form.contains(&("to".to_string(), "33612345678".to_string())));
        assert!(form.contains(&("from".to_string(), "DumbMonit".to_string())));
        let text = &form.iter().find(|(n, _)| n == "text").unwrap().1;
        assert!(!text.contains('⚠'), "no emoji in an SMS: {text}");

        assert_eq!(Vonage::verify(r#"{"message-count":"1","messages":[{"status":"0"}]}"#), None);
        let refused = Vonage::verify(
            r#"{"messages":[{"status":"2","error-text":"Missing api_key"}]}"#,
        )
        .unwrap();
        assert!(refused.contains("Missing api_key") && refused.contains("status 2"));
    }

    #[test]
    fn telnyx_exige_le_format_e164() {
        let ok = test_config(
            "telnyx",
            json!({"from": "+15005550006", "to": ["+33612345678"]}),
            json!({"api_key": "KEY0123"}),
        );
        let body = Telnyx::new(client(), &ok).unwrap().payload(&sample_message(false), "+33612345678");
        assert_eq!(body["from"], "+15005550006");
        assert_eq!(body["to"], "+33612345678");
        let bad = test_config(
            "telnyx",
            json!({"from": "+15005550006", "to": ["0612345678"]}),
            json!({"api_key": "k"}),
        );
        assert!(refus(Telnyx::new(client(), &bad)).contains("E.164"));
    }

    #[test]
    fn aspsms_regroupe_les_destinataires_et_lit_le_code_de_statut() {
        let config = test_config(
            "aspsms",
            json!({"userkey": "ABCDE", "originator": "DumbMonit", "to": ["+41791234567", "+41797654321"]}),
            json!({"password": "pw"}),
        );
        let body = AspSms::new(client(), &config).unwrap().payload(&sample_message(false));
        assert_eq!(body["Recipients"], json!(["+41791234567", "+41797654321"]));
        assert_eq!(body["Originator"], "DumbMonit");
        assert_eq!(AspSms::verify(r#"{"StatusCode":"1","StatusInfo":"OK"}"#), None);
        let refused = AspSms::verify(r#"{"StatusCode":"3","StatusInfo":"Credits missing"}"#).unwrap();
        assert!(refused.contains("Credits missing"));
    }

    #[test]
    fn un_corps_de_sms_est_borne_et_tient_sur_une_ligne() {
        let mut message = sample_message(false);
        message.text = "x\n".repeat(400);
        let body = sms_body(&message, SMS_BODY_MAX);
        assert!(body.chars().count() <= SMS_BODY_MAX);
        assert!(!body.contains('\n'));
    }
}
