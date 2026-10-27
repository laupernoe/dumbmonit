//! Tickets automatiques : Jira, GLPI, Zammad, GitHub Issues et GitLab Issues.
//!
//! Ces canaux ne « notifient » pas : ils tiennent un ticket par alerte.
//!
//! - Une alerte qui s'ouvre crée le ticket (titre, description avec le lien
//!   vers DumbMonit, priorité tirée de la sévérité). L'identifiant externe est
//!   gardé par alerte et par canal (table `alert_tickets`) : tant que le ticket
//!   est ouvert, aucun second n'est créé.
//! - Un fait notable — escalade, relais sans acquittement, nouvelle
//!   constatation, acquittement par un humain — s'ajoute en commentaire.
//! - La résolution commente puis ferme le ticket (transition, statut ou état
//!   choisis sur le canal). Si l'alerte revient dans le délai `reopen_hours`, le
//!   ticket est rouvert plutôt que remplacé.
//!
//! Les erreurs suivent le chemin des autres canaux : elles remontent dans le
//! compte rendu d'envoi (`last_error` du canal) et, quand elles sont passagères,
//! le moteur remet l'envoi en file avec son délai croissant. Chaque geste est
//! rejouable sans doublon : le lien n'est inscrit qu'une fois le ticket créé.

pub mod providers;
mod store;

use async_trait::async_trait;
use chrono::{DateTime, TimeDelta, Utc};
use sqlx::SqlitePool;
use tracing::{debug, warn};

use super::channel::ChannelConfig;
use super::error::NotifyError;
use super::message::{Message, render};
use super::{DeliveryReport, Notifier};
use crate::alerting::group::{AlertGroup, GroupItem, NotifyReason};
use crate::alerting::model::Severity;
use crate::alerting::notify_policy::Digest;
use crate::crypto::Cipher;
pub use store::{TicketLink, links_by_fingerprint};

/// Types de canaux qui tiennent des tickets.
pub const KINDS: &[&str] = &["jira", "glpi", "zammad", "github_issues", "gitlab_issues"];

pub fn is_ticket_kind(kind: &str) -> bool {
    KINDS.contains(&kind)
}

/// Ticket externe, tel que le service le désigne.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TicketRef {
    /// Identifiant utilisé dans les appels d'API (clé Jira, numéro GLPI…).
    pub id: String,
    /// Libellé lisible (« PROJ-12 », « #34 »).
    pub label: String,
    /// Adresse du ticket dans l'interface du service.
    pub url: String,
}

/// Contenu d'un ticket à créer.
#[derive(Debug, Clone)]
pub struct Draft {
    pub title: String,
    /// Description en texte brut (Jira, GLPI, Zammad).
    pub text: String,
    /// Description en Markdown (GitHub, GitLab).
    pub markdown: String,
    pub severity: Severity,
}

impl Draft {
    fn from_message(message: &Message) -> Self {
        Self {
            title: message.title.clone(),
            text: message.text.clone(),
            markdown: message.markdown.clone(),
            severity: message.severity,
        }
    }
}

/// Les quatre gestes que le cycle de vie demande à un service de tickets.
#[async_trait]
pub trait Tracker: Send + Sync {
    async fn create(&self, draft: &Draft) -> Result<TicketRef, NotifyError>;
    async fn comment(&self, ticket: &TicketRef, text: &str) -> Result<(), NotifyError>;
    /// Ferme (transition, statut ou état configuré). Le commentaire est à part.
    async fn close(&self, ticket: &TicketRef) -> Result<(), NotifyError>;
    async fn reopen(&self, ticket: &TicketRef) -> Result<(), NotifyError>;
}

/// Instancie le service de tickets d'un canal.
pub fn tracker(
    http: &reqwest::Client,
    config: &ChannelConfig,
) -> Result<Box<dyn Tracker>, NotifyError> {
    let http = http.clone();
    Ok(match config.kind.as_str() {
        "jira" => Box::new(providers::Jira::new(http, config)?),
        "glpi" => Box::new(providers::Glpi::new(http, config)?),
        "zammad" => Box::new(providers::Zammad::new(http, config)?),
        "github_issues" => Box::new(providers::GithubIssues::new(http, config)?),
        "gitlab_issues" => Box::new(providers::GitlabIssues::new(http, config)?),
        other => return Err(NotifyError::UnknownKind(other.to_string())),
    })
}

/// Accès générique (`notify::build`) : ouvre un ticket sans le suivre.
///
/// Le moteur d'alerte n'y passe pas — il appelle [`deliver`], qui garde le lien
/// et ferme à la résolution. Ce chemin sert à valider la configuration d'un
/// canal et à tout appelant qui n'a pas la base sous la main.
pub struct Ticketing {
    kind: &'static str,
    tracker: Box<dyn Tracker>,
}

impl Ticketing {
    pub fn new(http: &reqwest::Client, config: &ChannelConfig) -> Result<Self, NotifyError> {
        let kind = KINDS
            .iter()
            .copied()
            .find(|kind| *kind == config.kind)
            .ok_or_else(|| NotifyError::UnknownKind(config.kind.clone()))?;
        Ok(Self { kind, tracker: tracker(http, config)? })
    }
}

#[async_trait]
impl Notifier for Ticketing {
    fn kind(&self) -> &'static str {
        self.kind
    }

    async fn send(&self, message: &Message) -> Result<(), NotifyError> {
        if message.resolved {
            return Ok(());
        }
        self.tracker.create(&Draft::from_message(message)).await.map(|_| ())
    }
}

/// Délai de réouverture du canal, en heures (0 : jamais).
fn reopen_window(config: &ChannelConfig) -> TimeDelta {
    let hours = providers::setting_number(config, "reopen_hours").ok().flatten().unwrap_or(0);
    TimeDelta::hours(hours.clamp(0, 24 * 90))
}

fn database_error(error: sqlx::Error) -> NotifyError {
    NotifyError::Transport(format!("database: {error}"))
}

/// Description du ticket d'une alerte : le rendu d'un message à une seule ligne.
fn draft_for(group: &AlertGroup, item: &GroupItem, public_url: Option<&str>) -> Draft {
    let mut single = group.clone();
    single.items = vec![item.clone()];
    single.severity = item.severity;
    let mut message = render(&single);
    if let Some(base) = public_url {
        message.attach_link(base);
    }
    Draft::from_message(&message)
}

fn device_link(group: Option<&AlertGroup>, public_url: Option<&str>) -> String {
    let Some(base) = public_url.map(|base| base.trim_end_matches('/')) else {
        return String::new();
    };
    match group.and_then(|group| group.target_id) {
        Some(id) => format!("\n{base}/targets/{id}"),
        None => format!("\n{base}"),
    }
}

/// Ce que le cycle d'un envoi fait pour une alerte.
struct Run<'a> {
    pool: &'a SqlitePool,
    tracker: &'a dyn Tracker,
    channel_id: i64,
    reopen: TimeDelta,
    public_url: Option<&'a str>,
    now: DateTime<Utc>,
}

impl Run<'_> {
    async fn apply(
        &self,
        group: Option<&AlertGroup>,
        target_name: &str,
        item: &GroupItem,
    ) -> Result<(), NotifyError> {
        let fingerprint = item.fingerprint.as_str();
        let linked =
            store::get(self.pool, self.channel_id, fingerprint).await.map_err(database_error)?;

        if item.reason == NotifyReason::Resolved {
            // Rien à fermer si le canal n'a jamais ouvert de ticket pour cette alerte.
            let Some(linked) = linked.filter(|linked| linked.open) else { return Ok(()) };
            let text = format!(
                "Resolved: \"{}\" on {target_name} is back to normal.{}",
                item.rule_name,
                device_link(group, self.public_url)
            );
            self.tracker.comment(&linked.ticket, &text).await?;
            self.tracker.close(&linked.ticket).await?;
            store::set_closed(self.pool, self.channel_id, fingerprint, self.now)
                .await
                .map_err(database_error)?;
            return Ok(());
        }

        match linked {
            // Dédoublonnage : le ticket existe, on ne le recrée pas ; seuls les
            // faits notables méritent un commentaire.
            Some(linked) if linked.open => {
                if let Some(text) = update_text(item, target_name) {
                    self.tracker.comment(&linked.ticket, &text).await?;
                }
                Ok(())
            }
            Some(linked)
                if self.reopen > TimeDelta::zero()
                    && linked.closed_at.is_some_and(|at| self.now - at <= self.reopen) =>
            {
                let reopened = async {
                    self.tracker.reopen(&linked.ticket).await?;
                    let text = format!(
                        "The alert \"{}\" on {target_name} came back: ticket reopened by \
                         DumbMonit.{}",
                        item.rule_name,
                        device_link(group, self.public_url)
                    );
                    self.tracker.comment(&linked.ticket, &text).await
                }
                .await;
                match reopened {
                    Ok(()) => {
                        store::set_reopened(self.pool, self.channel_id, fingerprint, self.now)
                            .await
                            .map_err(database_error)
                    }
                    // Réouverture impossible (transition absente, ticket supprimé) :
                    // mieux vaut un nouveau ticket qu'une alerte sans ticket.
                    Err(error) if !error.is_transient() => {
                        debug!(%error, "ticket cannot be reopened, opening a new one");
                        self.open_new(group, target_name, item).await
                    }
                    Err(error) => Err(error),
                }
            }
            _ => self.open_new(group, target_name, item).await,
        }
    }

    async fn open_new(
        &self,
        group: Option<&AlertGroup>,
        target_name: &str,
        item: &GroupItem,
    ) -> Result<(), NotifyError> {
        let draft = match group {
            Some(group) => draft_for(group, item, self.public_url),
            None => Draft {
                title: format!("{target_name} — {}", item.rule_name),
                text: item.rule_name.clone(),
                markdown: item.rule_name.clone(),
                severity: item.severity,
            },
        };
        let ticket = self.tracker.create(&draft).await?;
        store::set_open(self.pool, self.channel_id, &item.fingerprint, &ticket, self.now)
            .await
            .map_err(database_error)
    }
}

/// Commentaire d'un fait notable sur une alerte déjà suivie, `None` si rien à dire.
fn update_text(item: &GroupItem, target_name: &str) -> Option<String> {
    match item.reason {
        NotifyReason::Escalation => Some(format!(
            "Escalated: \"{}\" on {target_name} has lasted long enough to be raised to {}.",
            item.rule_name,
            item.severity.as_str()
        )),
        NotifyReason::Unacked => {
            Some(format!("Nobody acknowledged \"{}\" on {target_name} in time.", item.rule_name))
        }
        NotifyReason::Recurrence => Some(match &item.detail {
            Some(detail) => format!("New finding on {target_name}: {detail}"),
            None => format!("New finding for \"{}\" on {target_name}.", item.rule_name),
        }),
        _ => None,
    }
}

/// Résume plusieurs échecs en un seul message, sans en perdre le nombre.
fn summarize(errors: &[NotifyError]) -> Option<String> {
    let first = errors.first()?;
    Some(if errors.len() > 1 {
        format!("{first} (and {} more ticket operations failed)", errors.len() - 1)
    } else {
        first.to_string()
    })
}

/// Applique au canal ce que dit un message groupé : ouvrir, commenter, fermer.
pub async fn deliver(
    pool: &SqlitePool,
    http: &reqwest::Client,
    config: &ChannelConfig,
    digest: &Digest,
    public_url: Option<&str>,
    now: DateTime<Utc>,
) -> DeliveryReport {
    let report = |errors: Vec<NotifyError>| DeliveryReport {
        channel_id: config.id,
        channel_name: config.name.clone(),
        error: summarize(&errors),
        transient: errors.iter().any(NotifyError::is_transient),
    };
    if super::sending_disabled() {
        return DeliveryReport {
            channel_id: config.id,
            channel_name: config.name.clone(),
            error: Some(super::DISABLED_MESSAGE.to_string()),
            transient: false,
        };
    }
    let client = super::client_for(http, config);
    let tracker = match tracker(client, config) {
        Ok(tracker) => tracker,
        Err(error) => {
            warn!(channel = %config.name, kind = %config.kind, %error, "channel misconfigured");
            return report(vec![error]);
        }
    };
    let run = Run {
        pool,
        tracker: tracker.as_ref(),
        channel_id: config.id,
        reopen: reopen_window(config),
        public_url,
        now,
    };

    let mut errors = Vec::new();
    for group in &digest.groups {
        for item in &group.items {
            if let Err(error) = run.apply(Some(group), &group.target_name, item).await {
                warn!(channel = %config.name, kind = %config.kind, %error, "ticket sync failed");
                errors.push(error);
            }
        }
    }
    for (target_name, item) in &digest.resolved_meanwhile {
        if let Err(error) = run.apply(None, target_name, item).await {
            warn!(channel = %config.name, kind = %config.kind, %error, "ticket sync failed");
            errors.push(error);
        }
    }
    if let Err(error) = store::purge_closed(pool, now).await {
        debug!(%error, "closed ticket links not purged");
    }
    report(errors)
}

/// Commente les tickets ouverts d'une alerte qu'un humain vient d'acquitter.
///
/// Lancé en arrière-plan par l'API d'acquittement : un service de tickets lent
/// ne doit pas faire attendre le bouton. Les échecs sont consignés sur le canal
/// comme n'importe quel envoi.
pub async fn on_acknowledged(pool: &SqlitePool, cipher: &Cipher, fingerprint: &str, text: &str) {
    if super::sending_disabled() {
        return;
    }
    let open = match store::open_for(pool, fingerprint).await {
        Ok(open) if !open.is_empty() => open,
        Ok(_) => return,
        Err(error) => {
            warn!(%error, "ticket links not read");
            return;
        }
    };
    let http = super::http_client();
    for (channel_id, ticket) in open {
        let config = match crate::db::alerts::get_channel(pool, cipher, channel_id).await {
            Ok(Some(config)) if config.enabled => config,
            _ => continue,
        };
        let outcome = async {
            let tracker = tracker(super::client_for(&http, &config), &config)?;
            tracker.comment(&ticket, text).await
        }
        .await;
        let report = DeliveryReport {
            channel_id,
            channel_name: config.name.clone(),
            error: outcome.as_ref().err().map(ToString::to_string),
            transient: false,
        };
        if let Err(error) = &outcome {
            warn!(channel = %config.name, %error, "acknowledgement not added to the ticket");
        }
        if let Err(error) = crate::db::alerts::record_delivery(pool, &report, Utc::now()).await {
            warn!(?error, "delivery report not recorded");
        }
    }
}

/// Test d'un canal de tickets : crée un ticket de test puis le ferme.
pub async fn test_channel(
    http: &reqwest::Client,
    config: &ChannelConfig,
    public_url: Option<&str>,
) -> Result<(), NotifyError> {
    let tracker = tracker(super::client_for(http, config), config)?;
    if super::sending_disabled() {
        return Ok(());
    }
    let message = super::message::test_message_linked(&config.name, public_url);
    let ticket = tracker.create(&Draft::from_message(&message)).await?;
    let closed = async {
        tracker
            .comment(
                &ticket,
                "Test ticket from DumbMonit: it is closed right away, you can ignore it.",
            )
            .await?;
        tracker.close(&ticket).await
    }
    .await;
    closed.map_err(|error| {
        NotifyError::Config(format!(
            "the test ticket {} was created but could not be closed ({error}); close it by hand \
             and check the closing settings",
            ticket.label
        ))
    })
}
