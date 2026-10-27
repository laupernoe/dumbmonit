//! Les cinq services de tickets : Jira, GLPI, Zammad, GitHub et GitLab Issues.
//!
//! Chacun n'expose que quatre gestes (voir [`Tracker`]) : créer, commenter,
//! fermer, rouvrir. Le cycle de vie — quand les faire — est dans le module
//! parent, identique pour les cinq.

use async_trait::async_trait;
use reqwest::Method;
use serde_json::{Value, json};

use super::{Draft, TicketRef, Tracker};
use crate::alerting::model::Severity;
use crate::notify::channel::ChannelConfig;
use crate::notify::error::NotifyError;
use crate::notify::http::{self, Explain, Request, check_url, join_url, percent_encode};
use crate::notify::message::clip;
use crate::notify::secret::SecretString;

// --------------------------------------------------------------------------
// Outils communs
// --------------------------------------------------------------------------

/// Envoie une requête et lit la réponse en JSON (`null` si elle est vide).
async fn run<'a>(
    client: &reqwest::Client,
    method: Method,
    url: &'a str,
    body: Option<Value>,
    explain: Explain,
    prepare: impl FnOnce(Request<'a>) -> Request<'a>,
) -> Result<Value, NotifyError> {
    let mut request = Request::new(method, url).explain(explain);
    if let Some(body) = body {
        request = request.json(body);
    }
    let text = http::exchange(client, prepare(request)).await?;
    if text.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&text).map_err(|_| NotifyError::Rejected {
        status: 200,
        detail: "the service did not answer with JSON: check that the address is the one of its \
                 API"
        .to_string(),
    })
}

fn unexpected(what: &str) -> NotifyError {
    NotifyError::Rejected {
        status: 200,
        detail: format!("the service accepted the request but gave no {what}"),
    }
}

/// Valeur d'un champ qui peut être une chaîne ou un nombre.
fn text_of(value: &Value) -> Option<String> {
    match value {
        Value::String(text) if !text.is_empty() => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

/// Réglage numérique saisi comme nombre ou comme texte.
pub(super) fn setting_number(
    config: &ChannelConfig,
    key: &str,
) -> Result<Option<i64>, NotifyError> {
    match config.settings.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(number)) => Ok(number.as_i64()),
        Some(Value::String(text)) if text.trim().is_empty() => Ok(None),
        Some(Value::String(text)) => text
            .trim()
            .parse()
            .map(Some)
            .map_err(|_| NotifyError::Config(format!("\"{key}\" must be a number"))),
        Some(_) => Err(NotifyError::Config(format!("\"{key}\" must be a number"))),
    }
}

fn base_url(config: &ChannelConfig, key: &str) -> Result<String, NotifyError> {
    let url = config.setting(key)?;
    check_url(&url, key)?;
    Ok(url.trim().trim_end_matches('/').to_string())
}

fn generic_hint(service: &'static str, status: u16) -> Option<String> {
    Some(match status {
        401 => format!("{service} rejected the credentials: check the token (and the e-mail)"),
        403 => format!("{service} refused: the token lacks the right to do this"),
        404 => format!(
            "{service} does not know this address, project or ticket: check the settings, and \
             that the token can see the project"
        ),
        _ => return None,
    })
}

// --------------------------------------------------------------------------
// Jira
// --------------------------------------------------------------------------

/// Jira Cloud (e-mail + jeton d'API) ou Data Center (jeton d'accès personnel, sans
/// e-mail).
///
/// L'API v2 est utilisée partout : elle accepte du texte brut, là où la v3 de
/// Jira Cloud impose le format de document Atlassian.
pub struct Jira {
    http: reqwest::Client,
    base: String,
    email: Option<String>,
    token: SecretString,
    project: String,
    issue_type: String,
    labels: Vec<String>,
    map_priority: bool,
    close_transition: String,
    reopen_transition: String,
}

impl Jira {
    pub fn new(http: reqwest::Client, config: &ChannelConfig) -> Result<Self, NotifyError> {
        Ok(Self {
            base: base_url(config, "server_url")?,
            // Avec une adresse : Jira Cloud (e-mail + jeton d'API). Sans : Data Center (PAT).
            email: config.setting_opt("email"),
            token: config.secret("api_token")?,
            project: config.setting("project")?,
            issue_type: config.setting_opt("issue_type").unwrap_or_else(|| "Task".to_string()),
            labels: config.setting_list("labels"),
            map_priority: config.setting_opt("priority").as_deref() != Some("none"),
            close_transition: config
                .setting_opt("close_transition")
                .unwrap_or_else(|| "Done".to_string()),
            reopen_transition: config
                .setting_opt("reopen_transition")
                .unwrap_or_else(|| "Reopen".to_string()),
            http,
        })
    }

    fn explain(status: u16, body: &str) -> Option<String> {
        if status == 400 && body.contains("errors") {
            return Some(
                "Jira rejected the issue: the project key, the issue type or the priority field \
                 does not exist or is not allowed for this project"
                    .to_string(),
            );
        }
        generic_hint("Jira", status)
    }

    async fn call(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value, NotifyError> {
        let url = join_url(&self.base, &format!("rest/api/2/{path}"));
        run(&self.http, method, &url, body, Self::explain, |request| match &self.email {
            Some(email) => request.basic(email.clone(), &self.token),
            None => request.bearer(&self.token),
        })
        .await
    }

    fn priority(severity: Severity) -> &'static str {
        match severity {
            Severity::Critical => "Highest",
            Severity::Warning => "Medium",
            Severity::Info => "Low",
        }
    }

    async fn transition(&self, key: &str, name: &str) -> Result<(), NotifyError> {
        let path = format!("issue/{}/transitions", percent_encode(key));
        let listed = self.call(Method::GET, &path, None).await?;
        let transitions = listed["transitions"].as_array().cloned().unwrap_or_default();
        let wanted = name.trim().to_lowercase();
        let found = transitions.iter().find(|candidate| {
            [&candidate["name"], &candidate["to"]["name"]]
                .iter()
                .any(|label| label.as_str().is_some_and(|l| l.trim().to_lowercase() == wanted))
        });
        let Some(id) = found.and_then(|candidate| text_of(&candidate["id"])) else {
            let available: Vec<&str> =
                transitions.iter().filter_map(|candidate| candidate["name"].as_str()).collect();
            return Err(NotifyError::Rejected {
                status: 400,
                detail: format!(
                    "Jira has no transition named \"{name}\" from this status (available: {})",
                    if available.is_empty() { "none".to_string() } else { available.join(", ") }
                ),
            });
        };
        self.call(Method::POST, &path, Some(json!({"transition": {"id": id}}))).await?;
        Ok(())
    }
}

#[async_trait]
impl Tracker for Jira {
    async fn create(&self, draft: &Draft) -> Result<TicketRef, NotifyError> {
        let mut fields = json!({
            "project": {"key": self.project},
            "summary": clip(&draft.title, 250),
            "description": draft.text,
            "issuetype": {"name": self.issue_type},
        });
        if !self.labels.is_empty() {
            fields["labels"] = json!(self.labels);
        }
        if self.map_priority {
            fields["priority"] = json!({"name": Self::priority(draft.severity)});
        }
        let created = self.call(Method::POST, "issue", Some(json!({"fields": fields}))).await?;
        let key = created["key"].as_str().ok_or_else(|| unexpected("issue key"))?.to_string();
        Ok(TicketRef {
            url: join_url(&self.base, &format!("browse/{key}")),
            label: key.clone(),
            id: key,
        })
    }

    async fn comment(&self, ticket: &TicketRef, text: &str) -> Result<(), NotifyError> {
        let path = format!("issue/{}/comment", percent_encode(&ticket.id));
        self.call(Method::POST, &path, Some(json!({"body": text}))).await.map(|_| ())
    }

    async fn close(&self, ticket: &TicketRef) -> Result<(), NotifyError> {
        self.transition(&ticket.id, &self.close_transition).await
    }

    async fn reopen(&self, ticket: &TicketRef) -> Result<(), NotifyError> {
        self.transition(&ticket.id, &self.reopen_transition).await
    }
}

// --------------------------------------------------------------------------
// GLPI
// --------------------------------------------------------------------------

/// GLPI, par son API REST historique (`apirest.php`).
///
/// L'API est sans état côté client : chaque geste ouvre une session
/// (`initSession`), agit, puis la ferme.
pub struct Glpi {
    http: reqwest::Client,
    /// Racine de GLPI, sans `apirest.php`.
    root: String,
    app_token: Option<SecretString>,
    user_token: SecretString,
    entity: Option<i64>,
    category: Option<i64>,
    solved_status: i64,
}

impl Glpi {
    pub fn new(http: reqwest::Client, config: &ChannelConfig) -> Result<Self, NotifyError> {
        let mut root = base_url(config, "server_url")?;
        for suffix in ["/apirest.php", "/apirest"] {
            if let Some(stripped) = root.strip_suffix(suffix) {
                root = stripped.to_string();
            }
        }
        Ok(Self {
            root,
            app_token: config.secret_opt("app_token"),
            user_token: config.secret("user_token")?,
            entity: setting_number(config, "entity_id")?,
            category: setting_number(config, "category_id")?,
            // 5 : résolu, 6 : clos.
            solved_status: if config.setting_opt("close_status").as_deref() == Some("closed") {
                6
            } else {
                5
            },
            http,
        })
    }

    fn explain(status: u16, body: &str) -> Option<String> {
        if body.contains("ERROR_WRONG_APP_TOKEN_PARAMETER") || body.contains("ERROR_APP_TOKEN") {
            return Some("GLPI rejected the app token".to_string());
        }
        if body.contains("ERROR_GLPI_LOGIN") {
            return Some("GLPI rejected the user token".to_string());
        }
        generic_hint("GLPI", status)
    }

    async fn call(
        &self,
        session: Option<&SecretString>,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value, NotifyError> {
        let url = join_url(&join_url(&self.root, "apirest.php"), path);
        run(&self.http, method, &url, body, Self::explain, |mut request| {
            if let Some(app) = &self.app_token {
                request = request.header("App-Token", app.expose()).secret(app);
            }
            match session {
                Some(token) => request.header("Session-Token", token.expose()).secret(token),
                None => request
                    .header("Authorization", format!("user_token {}", self.user_token.expose()))
                    .secret(&self.user_token),
            }
        })
        .await
    }

    /// Exécute `action` dans une session ouverte puis refermée.
    async fn with_session<T, F, Fut>(&self, action: F) -> Result<T, NotifyError>
    where
        F: FnOnce(SecretString) -> Fut,
        Fut: std::future::Future<Output = Result<T, NotifyError>>,
    {
        let opened = self.call(None, Method::GET, "initSession", None).await?;
        let token = opened["session_token"]
            .as_str()
            .map(SecretString::new)
            .ok_or_else(|| unexpected("session token"))?;
        let result = action(token.clone()).await;
        // La fermeture est un geste de courtoisie : GLPI expire les sessions seul.
        let _ = self.call(Some(&token), Method::GET, "killSession", None).await;
        result
    }

    /// Le texte de GLPI est du HTML : on échappe, puis on garde les retours à la ligne.
    fn html(text: &str) -> String {
        text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('\n', "<br />")
    }

    /// Urgence, impact et priorité GLPI (de 1, très basse, à 5, très haute).
    fn level(severity: Severity) -> i64 {
        match severity {
            Severity::Critical => 5,
            Severity::Warning => 3,
            Severity::Info => 2,
        }
    }

    async fn set_status(&self, ticket: &TicketRef, status: i64) -> Result<(), NotifyError> {
        self.with_session(|session| async move {
            let path = format!("Ticket/{}", percent_encode(&ticket.id));
            self.call(
                Some(&session),
                Method::PUT,
                &path,
                Some(json!({"input": {"status": status}})),
            )
            .await
            .map(|_| ())
        })
        .await
    }
}

#[async_trait]
impl Tracker for Glpi {
    async fn create(&self, draft: &Draft) -> Result<TicketRef, NotifyError> {
        self.with_session(|session| async move {
            let level = Self::level(draft.severity);
            let mut input = json!({
                "name": clip(&draft.title, 250),
                "content": Self::html(&draft.text),
                // 1 : incident.
                "type": 1,
                "urgency": level,
                "impact": level,
                "priority": level,
            });
            if let Some(entity) = self.entity {
                input["entities_id"] = json!(entity);
            }
            if let Some(category) = self.category {
                input["itilcategories_id"] = json!(category);
            }
            let created = self
                .call(Some(&session), Method::POST, "Ticket", Some(json!({"input": input})))
                .await?;
            let id = text_of(&created["id"]).ok_or_else(|| unexpected("ticket number"))?;
            Ok(TicketRef {
                url: format!("{}/front/ticket.form.php?id={id}", self.root),
                label: format!("#{id}"),
                id,
            })
        })
        .await
    }

    async fn comment(&self, ticket: &TicketRef, text: &str) -> Result<(), NotifyError> {
        self.with_session(|session| async move {
            let input =
                json!({"itemtype": "Ticket", "items_id": ticket.id, "content": Self::html(text)});
            self.call(Some(&session), Method::POST, "ITILFollowup", Some(json!({"input": input})))
                .await
                .map(|_| ())
        })
        .await
    }

    async fn close(&self, ticket: &TicketRef) -> Result<(), NotifyError> {
        self.set_status(ticket, self.solved_status).await
    }

    async fn reopen(&self, ticket: &TicketRef) -> Result<(), NotifyError> {
        // 1 : nouveau.
        self.set_status(ticket, 1).await
    }
}

// --------------------------------------------------------------------------
// Zammad
// --------------------------------------------------------------------------

/// Zammad, par son API REST et un jeton d'accès personnel.
pub struct Zammad {
    http: reqwest::Client,
    base: String,
    token: SecretString,
    group: String,
    customer: String,
    close_state: String,
    reopen_state: String,
}

impl Zammad {
    pub fn new(http: reqwest::Client, config: &ChannelConfig) -> Result<Self, NotifyError> {
        Ok(Self {
            base: base_url(config, "server_url")?,
            token: config.secret("token")?,
            group: config.setting("group")?,
            customer: config.setting("customer")?,
            close_state: config.setting_opt("close_state").unwrap_or_else(|| "closed".to_string()),
            reopen_state: config.setting_opt("reopen_state").unwrap_or_else(|| "open".to_string()),
            http,
        })
    }

    fn explain(status: u16, _body: &str) -> Option<String> {
        if status == 422 {
            return Some(
                "Zammad rejected the ticket: the group, the customer or the state does not exist"
                    .to_string(),
            );
        }
        generic_hint("Zammad", status)
    }

    async fn call(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value, NotifyError> {
        let url = join_url(&self.base, &format!("api/v1/{path}"));
        run(&self.http, method, &url, body, Self::explain, |request| {
            request
                .header("Authorization", format!("Token token={}", self.token.expose()))
                .secret(&self.token)
        })
        .await
    }

    fn priority_id(severity: Severity) -> i64 {
        // 1 basse, 2 normale, 3 haute.
        match severity {
            Severity::Critical => 3,
            Severity::Warning => 2,
            Severity::Info => 1,
        }
    }

    async fn set_state(&self, ticket: &TicketRef, state: &str) -> Result<(), NotifyError> {
        let path = format!("tickets/{}", percent_encode(&ticket.id));
        self.call(Method::PUT, &path, Some(json!({"state": state}))).await.map(|_| ())
    }
}

#[async_trait]
impl Tracker for Zammad {
    async fn create(&self, draft: &Draft) -> Result<TicketRef, NotifyError> {
        let body = json!({
            "title": clip(&draft.title, 250),
            "group": self.group,
            "customer": self.customer,
            "priority_id": Self::priority_id(draft.severity),
            "article": {
                "subject": clip(&draft.title, 250),
                "body": draft.text,
                "type": "note",
                "internal": true,
            },
        });
        let created = self.call(Method::POST, "tickets", Some(body)).await?;
        let id = text_of(&created["id"]).ok_or_else(|| unexpected("ticket id"))?;
        let number = text_of(&created["number"]).unwrap_or_else(|| id.clone());
        Ok(TicketRef {
            url: format!("{}/#ticket/zoom/{id}", self.base),
            label: format!("#{number}"),
            id,
        })
    }

    async fn comment(&self, ticket: &TicketRef, text: &str) -> Result<(), NotifyError> {
        let body = json!({
            "ticket_id": ticket.id.parse::<i64>().map_or_else(|_| json!(ticket.id), |id| json!(id)),
            "subject": "DumbMonit",
            "body": text,
            "type": "note",
            "internal": true,
        });
        self.call(Method::POST, "ticket_articles", Some(body)).await.map(|_| ())
    }

    async fn close(&self, ticket: &TicketRef) -> Result<(), NotifyError> {
        self.set_state(ticket, &self.close_state).await
    }

    async fn reopen(&self, ticket: &TicketRef) -> Result<(), NotifyError> {
        self.set_state(ticket, &self.reopen_state).await
    }
}

// --------------------------------------------------------------------------
// GitHub Issues
// --------------------------------------------------------------------------

/// GitHub Issues (github.com ou GitHub Enterprise Server), jeton à granularité fine.
pub struct GithubIssues {
    http: reqwest::Client,
    api: String,
    repository: String,
    token: SecretString,
    labels: Vec<String>,
}

impl GithubIssues {
    pub fn new(http: reqwest::Client, config: &ChannelConfig) -> Result<Self, NotifyError> {
        let repository = config.setting("repository")?;
        let parts: Vec<&str> = repository.trim().split('/').collect();
        if parts.len() != 2 || parts.iter().any(|part| part.trim().is_empty()) {
            return Err(NotifyError::Config(
                "\"repository\" must look like owner/name".to_string(),
            ));
        }
        let api = match config.setting_opt("api_url") {
            Some(_) => base_url(config, "api_url")?,
            None => "https://api.github.com".to_string(),
        };
        Ok(Self {
            api,
            repository: format!(
                "{}/{}",
                percent_encode(parts[0].trim()),
                percent_encode(parts[1].trim())
            ),
            token: config.secret("token")?,
            labels: config.setting_list("labels"),
            http,
        })
    }

    fn explain(status: u16, _body: &str) -> Option<String> {
        if status == 410 {
            return Some("this repository has its issues disabled".to_string());
        }
        generic_hint("GitHub", status)
    }

    async fn call(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value, NotifyError> {
        let url = join_url(&self.api, &format!("repos/{}/{path}", self.repository));
        run(&self.http, method, &url, body, Self::explain, |request| {
            request
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .bearer(&self.token)
        })
        .await
    }

    async fn set_state(&self, ticket: &TicketRef, body: Value) -> Result<(), NotifyError> {
        let path = format!("issues/{}", percent_encode(&ticket.id));
        self.call(Method::PATCH, &path, Some(body)).await.map(|_| ())
    }
}

#[async_trait]
impl Tracker for GithubIssues {
    async fn create(&self, draft: &Draft) -> Result<TicketRef, NotifyError> {
        let mut body = json!({"title": clip(&draft.title, 250), "body": draft.markdown});
        if !self.labels.is_empty() {
            body["labels"] = json!(self.labels);
        }
        let created = self.call(Method::POST, "issues", Some(body)).await?;
        let number = text_of(&created["number"]).ok_or_else(|| unexpected("issue number"))?;
        let url = created["html_url"].as_str().ok_or_else(|| unexpected("issue address"))?;
        Ok(TicketRef { id: number.clone(), label: format!("#{number}"), url: url.to_string() })
    }

    async fn comment(&self, ticket: &TicketRef, text: &str) -> Result<(), NotifyError> {
        let path = format!("issues/{}/comments", percent_encode(&ticket.id));
        self.call(Method::POST, &path, Some(json!({"body": text}))).await.map(|_| ())
    }

    async fn close(&self, ticket: &TicketRef) -> Result<(), NotifyError> {
        self.set_state(ticket, json!({"state": "closed", "state_reason": "completed"})).await
    }

    async fn reopen(&self, ticket: &TicketRef) -> Result<(), NotifyError> {
        self.set_state(ticket, json!({"state": "open"})).await
    }
}

// --------------------------------------------------------------------------
// GitLab Issues
// --------------------------------------------------------------------------

/// GitLab Issues (gitlab.com ou instance auto-hébergée).
pub struct GitlabIssues {
    http: reqwest::Client,
    base: String,
    project: String,
    token: SecretString,
    labels: Vec<String>,
}

impl GitlabIssues {
    pub fn new(http: reqwest::Client, config: &ChannelConfig) -> Result<Self, NotifyError> {
        Ok(Self {
            base: base_url(config, "server_url")?,
            // Un identifiant numérique ou un chemin « groupe/projet » : l'API veut
            // le second encodé.
            project: percent_encode(config.setting("project")?.trim().trim_matches('/')),
            token: config.secret("token")?,
            labels: config.setting_list("labels"),
            http,
        })
    }

    fn explain(status: u16, _body: &str) -> Option<String> {
        generic_hint("GitLab", status)
    }

    async fn call(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value, NotifyError> {
        let url = join_url(&self.base, &format!("api/v4/projects/{}/{path}", self.project));
        run(&self.http, method, &url, body, Self::explain, |request| {
            request.header("PRIVATE-TOKEN", self.token.expose()).secret(&self.token)
        })
        .await
    }

    async fn set_event(&self, ticket: &TicketRef, event: &str) -> Result<(), NotifyError> {
        let path = format!("issues/{}", percent_encode(&ticket.id));
        self.call(Method::PUT, &path, Some(json!({"state_event": event}))).await.map(|_| ())
    }
}

#[async_trait]
impl Tracker for GitlabIssues {
    async fn create(&self, draft: &Draft) -> Result<TicketRef, NotifyError> {
        let mut body = json!({"title": clip(&draft.title, 250), "description": draft.markdown});
        if !self.labels.is_empty() {
            body["labels"] = json!(self.labels.join(","));
        }
        let created = self.call(Method::POST, "issues", Some(body)).await?;
        let iid = text_of(&created["iid"]).ok_or_else(|| unexpected("issue number"))?;
        let url = created["web_url"].as_str().ok_or_else(|| unexpected("issue address"))?;
        Ok(TicketRef { id: iid.clone(), label: format!("#{iid}"), url: url.to_string() })
    }

    async fn comment(&self, ticket: &TicketRef, text: &str) -> Result<(), NotifyError> {
        let path = format!("issues/{}/notes", percent_encode(&ticket.id));
        self.call(Method::POST, &path, Some(json!({"body": text}))).await.map(|_| ())
    }

    async fn close(&self, ticket: &TicketRef) -> Result<(), NotifyError> {
        self.set_event(ticket, "close").await
    }

    async fn reopen(&self, ticket: &TicketRef) -> Result<(), NotifyError> {
        self.set_event(ticket, "reopen").await
    }
}
