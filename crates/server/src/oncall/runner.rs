//! Le moteur des astreintes : ouvrir une escalade quand un canal d'astreinte
//! reçoit une alerte, la faire avancer à chaque cycle, prévenir de la
//! résolution, annoncer les relèves.
//!
//! Appelé par le moteur d'alerting ([`crate::alerting::engine`]). Comme lui,
//! rien de ce qui échoue ici ne doit interrompre la boucle : un contact en
//! panne est consigné dans le journal de l'escalade, et l'on passe au suivant.

use std::collections::{BTreeMap, HashMap, HashSet};

use anyhow::{Context, Result, bail};
use chrono::{DateTime, TimeDelta, Utc};
use serde_json::{Value, json};
use sqlx::SqlitePool;
use tracing::warn;

use crate::alerting::group::{AlertGroup, GroupItem, NotifyReason};
use crate::alerting::model::Severity;
use crate::alerting::notify_policy::Digest;
use crate::auth::users::{self, User};
use crate::crypto::Cipher;
use crate::db;
use crate::notify::action_link::{self, ActionKey};
use crate::notify::{self, ChannelConfig, Message, NotifyError};
use crate::oncall::escalation::{self, AlertView, PolicyDef, Progress, Stop, Target, Tick};
use crate::oncall::schedule::Override;
use crate::oncall::store::{self, ContactRow, EscalationRow, Event, Prefs, ScheduleRow, to_sql};

/// Types de canaux qu'un compte peut déclarer comme service personnel.
///
/// Ni webhook générique ni serveur SMTP propre : un compte lecteur n'a pas à
/// faire émettre au serveur des requêtes arbitraires. Le courriel et le SMS
/// passent par un canal d'équipe, avec la seule adresse changée.
pub const PERSONAL_KINDS: &[&str] = &[
    "ntfy",
    "pushover",
    "telegram",
    "gotify",
    "bark",
    "pushbullet",
    "signal",
    "discord",
    "slack",
    "matrix",
    "mattermost",
    "rocketchat",
    "teams",
    "googlechat",
    "zulip",
    "homeassistant",
    "apprise",
];

/// Destinataire qu'un canal d'équipe accepte de changer pour un contact : la
/// clé de réglage, si c'est une liste, et la nature de l'adresse.
pub fn recipient_key(kind: &str) -> Option<(&'static str, bool, &'static str)> {
    match kind {
        "smtp" => Some(("to", true, "email")),
        "twilio" => Some(("to", true, "phone")),
        "signal" => Some(("recipients", true, "phone")),
        "telegram" => Some(("chat_id", false, "chat_id")),
        _ => None,
    }
}

/// Ce dont un passage a besoin pour envoyer.
#[derive(Clone, Copy)]
pub struct Ctx<'a> {
    pub pool: &'a SqlitePool,
    pub cipher: &'a Cipher,
    pub http: &'a reqwest::Client,
    pub public_url: Option<&'a str>,
    pub action_key: Option<&'a ActionKey>,
}

/// Comptes, contacts, préférences, canaux et plannings, chargés une fois par
/// passage.
pub struct Directory {
    pub users: HashMap<i64, User>,
    pub contacts: Vec<ContactRow>,
    pub prefs: HashMap<i64, Prefs>,
    pub channels: Vec<ChannelConfig>,
    pub schedules: Vec<(ScheduleRow, Vec<Override>)>,
    /// Comptes d'astreinte à l'instant du chargement, tous plannings confondus.
    pub on_call_now: HashSet<i64>,
}

impl Directory {
    pub async fn load(pool: &SqlitePool, cipher: &Cipher, now: DateTime<Utc>) -> Result<Self> {
        let users = users::list(pool).await?.into_iter().map(|user| (user.id, user)).collect();
        let contacts = store::list_contacts(pool, cipher, None).await?;
        let prefs = store::all_prefs(pool).await?;
        let channels = db::alerts::list_channels(pool, cipher).await?;
        let schedules = load_schedules(pool, now).await?;
        let on_call_now = schedules
            .iter()
            .filter_map(|(row, overrides)| row.definition.on_call(overrides, now))
            .map(|slot| slot.user_id)
            .collect();
        Ok(Self { users, contacts, prefs, channels, schedules, on_call_now })
    }

    pub fn schedule(&self, id: i64) -> Option<&(ScheduleRow, Vec<Override>)> {
        self.schedules.iter().find(|(row, _)| row.id == id)
    }

    fn contacts_of(&self, user_id: i64) -> impl Iterator<Item = &ContactRow> {
        self.contacts.iter().filter(move |contact| contact.user_id == user_id)
    }
}

/// Plannings et leurs remplacements utiles autour de `now`.
pub async fn load_schedules(
    pool: &SqlitePool,
    now: DateTime<Utc>,
) -> Result<Vec<(ScheduleRow, Vec<Override>)>> {
    let schedules = store::list_schedules(pool).await?;
    let overrides = store::list_overrides(pool, None, Some(now - TimeDelta::days(1))).await?;
    Ok(schedules
        .into_iter()
        .map(|row| {
            let mine =
                overrides.iter().filter(|o| o.schedule_id == row.id).map(|o| o.pure()).collect();
            (row, mine)
        })
        .collect())
}

pub fn display_name(user: &User) -> String {
    if user.display_name.trim().is_empty() {
        user.username.clone()
    } else {
        user.display_name.trim().to_string()
    }
}

/// Configuration de canal qui atteint ce contact.
pub fn contact_config(
    channels: &[ChannelConfig],
    person: &str,
    contact: &ContactRow,
) -> Result<ChannelConfig, String> {
    let Some(channel_id) = contact.via_channel_id else {
        return Ok(ChannelConfig {
            id: 0,
            name: format!("{person}: {}", contact.label),
            kind: contact.kind.clone(),
            enabled: true,
            settings: contact.settings.clone(),
            secrets: contact.secrets.clone(),
        });
    };
    let channel = channels
        .iter()
        .find(|channel| channel.id == channel_id)
        .ok_or_else(|| "the team channel this contact uses no longer exists".to_string())?;
    if !channel.enabled {
        return Err(format!("the team channel \"{}\" is disabled", channel.name));
    }
    let (key, list, _) = recipient_key(&channel.kind)
        .ok_or_else(|| format!("the team channel \"{}\" cannot reach a person", channel.name))?;
    let address = contact
        .address
        .as_deref()
        .map(str::trim)
        .filter(|address| !address.is_empty())
        .ok_or_else(|| "this contact has no address".to_string())?;
    let mut config = channel.clone();
    config.name = format!("{} → {person}", channel.name);
    let mut settings = config.settings.as_object().cloned().unwrap_or_default();
    settings.insert(key.to_string(), if list { json!([address]) } else { json!(address) });
    // Un sujet de groupe Telegram n'a pas de sens dans une conversation privée.
    settings.remove("message_thread_id");
    config.settings = Value::Object(settings);
    Ok(config)
}

fn now_event(now: DateTime<Utc>, event: &str) -> Event {
    Event { at: to_sql(now), event: event.to_string(), ..Event::default() }
}

/// Message simple, hors alerte (relève, test).
fn plain_message(title: String, text: String, now: DateTime<Utc>) -> Message {
    Message {
        markdown: format!("**{title}**\n{text}"),
        title,
        text,
        severity: Severity::Info,
        resolved: false,
        target_name: "DumbMonit".to_string(),
        target_id: None,
        at: now,
        rule_name: "On-call".to_string(),
        fingerprint: "dumbmonit-oncall".to_string(),
        value: None,
        threshold: None,
        unit: String::new(),
        operator: String::new(),
        count: 1,
        link: None,
        actions: Vec::new(),
    }
}

/// Le message d'un appel ou d'une résolution, liens d'action compris.
fn alert_message(
    ctx: &Ctx<'_>,
    esc: &EscalationRow,
    item: &GroupItem,
    label: Option<String>,
    now: DateTime<Utc>,
) -> Message {
    let group = AlertGroup {
        target_id: esc.target_id,
        target_name: esc.target_name.clone(),
        target_kind: String::new(),
        target_tags: BTreeMap::new(),
        severity: item.severity,
        items: vec![item.clone()],
        channels: Vec::new(),
        at: now,
    };
    let digest =
        Digest { groups: vec![group], resolved_meanwhile: Vec::new(), hold: None, at: now };
    let mut message = notify::render_digest(&digest, ctx.public_url);
    if let Some(label) = label {
        message.title = format!("{label} {}", message.title);
    }
    if let (Some(key), Some(base)) = (ctx.action_key, ctx.public_url) {
        let alerts = notify::message::actionable_alerts(&digest);
        message.actions = action_link::build(key, base, &alerts, now);
    }
    message
}

// --------------------------------------------------------------------------
// Ouverture et résolution (depuis l'envoi d'un canal d'astreinte)
// --------------------------------------------------------------------------

/// Un canal d'astreinte reçoit un message : chaque alerte qui se déclare
/// ouvre une escalade (une seule par épisode), chaque résolution referme la
/// sienne et prévient ceux qui avaient été appelés.
pub async fn engage(
    ctx: &Ctx<'_>,
    channel: &ChannelConfig,
    digest: &Digest,
    now: DateTime<Utc>,
) -> Result<()> {
    let Some(policy_id) = store::policy_id_of(&channel.settings) else {
        bail!("the on-call channel \"{}\" has no escalation policy", channel.name);
    };
    if store::get_policy(ctx.pool, policy_id).await?.is_none() {
        bail!("the escalation policy {policy_id} of \"{}\" no longer exists", channel.name);
    }
    for group in &digest.groups {
        for item in &group.items {
            if item.reason == NotifyReason::Resolved {
                resolve(ctx, channel.id, item, now).await?;
                continue;
            }
            let episode = item.since.map(to_sql).unwrap_or_default();
            if store::escalation_exists(ctx.pool, channel.id, &item.fingerprint, &episode).await? {
                continue;
            }
            let raw = serde_json::to_string(item)?;
            let id = store::insert_escalation(
                ctx.pool,
                &store::NewEscalation {
                    policy_id,
                    channel_id: channel.id,
                    fingerprint: &item.fingerprint,
                    episode: &episode,
                    target_id: group.target_id,
                    target_name: &group.target_name,
                    rule_name: &item.rule_name,
                    severity: item.severity,
                    item: &raw,
                    now,
                },
            )
            .await?;
            let mut event = now_event(now, "started");
            event.via = channel.name.clone();
            event.kind = channel.kind.clone();
            event.detail = format!("{} on {}", item.rule_name, group.target_name);
            store::add_event(ctx.pool, id, &event).await?;
        }
    }
    for (_, item) in &digest.resolved_meanwhile {
        resolve(ctx, channel.id, item, now).await?;
    }
    Ok(())
}

/// Referme les escalades en cours d'une alerte et prévient ceux qui avaient
/// été joints, sur le moyen qui les avait joints.
async fn resolve(
    ctx: &Ctx<'_>,
    channel_id: i64,
    item: &GroupItem,
    now: DateTime<Utc>,
) -> Result<()> {
    let open = store::active_for(ctx.pool, channel_id, &item.fingerprint).await?;
    if open.is_empty() {
        return Ok(());
    }
    let directory = Directory::load(ctx.pool, ctx.cipher, now).await?;
    for esc in open {
        store::finish_escalation(ctx.pool, esc.id, "resolved", None, now).await?;
        let mut event = now_event(now, "resolved");
        event.detail = "The alert resolved.".into();
        store::add_event(ctx.pool, esc.id, &event).await?;

        let reached = store::events_of(ctx.pool, esc.id).await?;
        let mut resolved = item.clone();
        resolved.reason = NotifyReason::Resolved;
        let message = alert_message(ctx, &esc, &resolved, None, now);
        let mut contacts_done = HashSet::new();
        let mut channels_done = HashSet::new();
        for previous in reached.iter().filter(|e| e.event == "notified") {
            if let Some(contact_id) = previous.contact_id
                && contacts_done.insert(contact_id)
                && let Some(contact) = directory.contacts.iter().find(|c| c.id == contact_id)
                && let Some(user) = directory.users.get(&contact.user_id)
            {
                let outcome = send_contact(ctx, &directory, user, contact, &message, now).await;
                let mut event = contact_event(now, None, None, user, contact, &outcome);
                event.detail =
                    format!("resolution · {}", event.detail).trim_end_matches(" · ").to_string();
                store::add_event(ctx.pool, esc.id, &event).await?;
            }
            if let Some(channel_id) = previous.channel_id
                && channels_done.insert(channel_id)
                && let Some(config) = directory.channels.iter().find(|c| c.id == channel_id)
            {
                let outcome = send_channel(ctx, config, &message, now).await;
                let mut event = channel_event(now, None, None, config, &outcome);
                event.detail = "resolution".into();
                store::add_event(ctx.pool, esc.id, &event).await?;
            }
        }
    }
    Ok(())
}

// --------------------------------------------------------------------------
// Envois
// --------------------------------------------------------------------------

async fn send_contact(
    ctx: &Ctx<'_>,
    directory: &Directory,
    user: &User,
    contact: &ContactRow,
    message: &Message,
    now: DateTime<Utc>,
) -> Result<(), String> {
    let outcome = match contact_config(&directory.channels, &display_name(user), contact) {
        Ok(config) => match notify::deliver(ctx.http, &config, message).await.error {
            None => Ok(()),
            Some(error) => Err(error),
        },
        Err(error) => Err(error),
    };
    if let Err(error) = store::note_contact_use(
        ctx.pool,
        contact.id,
        outcome.as_ref().err().map(String::as_str),
        now,
    )
    .await
    {
        warn!(?error, "on-call contact outcome not recorded");
    }
    outcome
}

async fn send_channel(
    ctx: &Ctx<'_>,
    config: &ChannelConfig,
    message: &Message,
    now: DateTime<Utc>,
) -> Result<(), String> {
    if !config.enabled {
        return Err(format!("channel \"{}\" is disabled", config.name));
    }
    if config.kind == crate::oncall::CHANNEL_KIND {
        return Err("an on-call channel cannot be a step target".into());
    }
    let store = notify::webpush::Store { pool: ctx.pool, cipher: ctx.cipher };
    let report = notify::deliver_stored(store, ctx.http, config, message).await;
    if let Err(error) = db::alerts::record_delivery(ctx.pool, &report, now).await {
        warn!(?error, "delivery report not recorded");
    }
    report.error.map_or(Ok(()), Err)
}

fn contact_event(
    now: DateTime<Utc>,
    step: Option<usize>,
    round: Option<u32>,
    user: &User,
    contact: &ContactRow,
    outcome: &Result<(), String>,
) -> Event {
    let mut event = now_event(now, if outcome.is_ok() { "notified" } else { "failed" });
    event.step = step.map(|s| s as i64);
    event.round = round.map(i64::from);
    event.user_id = Some(user.id);
    event.user_name = Some(display_name(user));
    event.via = contact.label.clone();
    event.kind = contact.kind.clone();
    event.detail = outcome.as_ref().err().cloned().unwrap_or_default();
    event.contact_id = outcome.is_ok().then_some(contact.id);
    event
}

fn channel_event(
    now: DateTime<Utc>,
    step: Option<usize>,
    round: Option<u32>,
    config: &ChannelConfig,
    outcome: &Result<(), String>,
) -> Event {
    let mut event = now_event(now, if outcome.is_ok() { "notified" } else { "failed" });
    event.step = step.map(|s| s as i64);
    event.round = round.map(i64::from);
    event.via = config.name.clone();
    event.kind = config.kind.clone();
    event.detail = outcome.as_ref().err().cloned().unwrap_or_default();
    event.channel_id = outcome.is_ok().then_some(config.id);
    event
}

/// Une personne appelée par une étape, et pourquoi.
struct Callee {
    user_id: i64,
    on_call: bool,
    reason: String,
}

/// Qui une étape appelle à `now`, et ce qui a été sauté.
fn callees(
    directory: &Directory,
    targets: &[Target],
    now: DateTime<Utc>,
) -> (Vec<Callee>, Vec<i64>, Vec<String>) {
    let mut people: Vec<Callee> = Vec::new();
    let mut channels: Vec<i64> = Vec::new();
    let mut skipped = Vec::new();
    fn add(people: &mut Vec<Callee>, user_id: i64, on_call: bool, reason: String) {
        match people.iter_mut().find(|c| c.user_id == user_id) {
            Some(existing) => existing.on_call |= on_call,
            None => people.push(Callee { user_id, on_call, reason }),
        }
    }
    for target in targets {
        match *target {
            Target::Schedule { id } => match directory.schedule(id) {
                None => skipped.push(format!("schedule {id} no longer exists")),
                Some((row, overrides)) => match row.definition.on_call(overrides, now) {
                    None => skipped.push(format!("nobody is on call in \"{}\"", row.name)),
                    Some(slot) => {
                        add(&mut people, slot.user_id, true, format!("on call in \"{}\"", row.name))
                    }
                },
            },
            Target::User { id } => {
                let on_call = directory.on_call_now.contains(&id);
                add(&mut people, id, on_call, "named in the policy".to_string());
            }
            Target::Channel { id } => {
                if !channels.contains(&id) {
                    channels.push(id);
                }
            }
        }
    }
    (people, channels, skipped)
}

// --------------------------------------------------------------------------
// Le passage du moteur
// --------------------------------------------------------------------------

/// Fait avancer toutes les escalades en cours, puis annonce les relèves.
pub async fn tick(ctx: &Ctx<'_>, now: DateTime<Utc>) -> Result<()> {
    let escalations = store::active_escalations(ctx.pool).await?;
    let schedules_exist = !store::list_schedules(ctx.pool).await?.is_empty();
    if escalations.is_empty() && !schedules_exist {
        return Ok(());
    }
    let directory = Directory::load(ctx.pool, ctx.cipher, now).await?;
    if !escalations.is_empty() {
        let policies: HashMap<i64, PolicyDef> = store::list_policies(ctx.pool)
            .await?
            .into_iter()
            .map(|row| (row.id, row.definition))
            .collect();
        for esc in escalations {
            if let Err(error) = advance(ctx, &directory, &policies, &esc, now).await {
                warn!(escalation = esc.id, ?error, "on-call escalation step failed");
            }
        }
    }
    if let Err(error) = handoffs(ctx, &directory, now).await {
        warn!(?error, "on-call handoff notifications failed");
    }
    Ok(())
}

/// L'alerte telle que l'escalade la voit : même empreinte, même épisode.
async fn alert_view(pool: &SqlitePool, esc: &EscalationRow) -> Result<AlertView> {
    let Some(alert) = db::alerts::get_active(pool, &esc.fingerprint).await? else {
        return Ok(AlertView::default());
    };
    let episode = alert.state.firing_since.map(to_sql).unwrap_or_default();
    if !esc.episode.is_empty() && !episode.is_empty() && episode != esc.episode {
        // L'alerte s'est résolue puis a recommencé : c'est une autre escalade.
        return Ok(AlertView::default());
    }
    let state = alert.state;
    Ok(AlertView {
        active: true,
        acked_by: state
            .acked_until
            .map(|_| state.acked_by.clone().unwrap_or_else(|| "someone".to_string())),
        dismissed_by: state
            .dismissed_at
            .map(|_| state.dismissed_by.clone().unwrap_or_else(|| "someone".to_string())),
        silenced: state.silenced,
    })
}

async fn advance(
    ctx: &Ctx<'_>,
    directory: &Directory,
    policies: &HashMap<i64, PolicyDef>,
    esc: &EscalationRow,
    now: DateTime<Utc>,
) -> Result<()> {
    let pool = ctx.pool;
    let Some(policy) = policies.get(&esc.policy_id) else {
        store::finish_escalation(pool, esc.id, "cancelled", None, now).await?;
        let mut event = now_event(now, "cancelled");
        event.detail = "The escalation policy was deleted.".into();
        return store::add_event(pool, esc.id, &event).await;
    };
    let view = alert_view(pool, esc).await?;
    let progress = Progress {
        next_step: usize::try_from(esc.next_step).unwrap_or(0),
        round: u32::try_from(esc.round).unwrap_or(0),
        next_at: esc.next_at.unwrap_or(now),
    };
    match escalation::tick(policy, &progress, &view, now) {
        Tick::Wait => Ok(()),
        Tick::Stop(stop) => {
            let (status, by, detail) = match stop {
                Stop::Acked(by) => ("acked", Some(by.clone()), format!("Acknowledged by {by}.")),
                Stop::Resolved => ("resolved", None, "The alert resolved.".to_string()),
                Stop::Dismissed(by) => ("cancelled", None, format!("Dismissed by {by}.")),
                Stop::Silenced => {
                    ("cancelled", None, "Snoozed or covered by a maintenance window.".to_string())
                }
            };
            store::finish_escalation(pool, esc.id, status, by.as_deref(), now).await?;
            let mut event = now_event(now, status);
            event.user_name = by;
            event.detail = detail;
            store::add_event(pool, esc.id, &event).await
        }
        Tick::Exhausted => {
            store::finish_escalation(pool, esc.id, "exhausted", None, now).await?;
            let mut event = now_event(now, "exhausted");
            event.detail =
                format!("Nobody acknowledged after {} round(s) of the policy.", progress.round + 1);
            store::add_event(pool, esc.id, &event).await
        }
        Tick::Page { step, round, then } => {
            // L'étape est consignée d'abord comme faite : un envoi qui ferait
            // planter le passage ne doit pas rappeler les mêmes personnes en
            // boucle à chaque cycle.
            store::set_progress(pool, esc.id, then.next_step, then.round, then.next_at).await?;
            page(ctx, directory, esc, policy, step, round, now).await
        }
    }
}

/// Appelle une étape.
async fn page(
    ctx: &Ctx<'_>,
    directory: &Directory,
    esc: &EscalationRow,
    policy: &PolicyDef,
    step: usize,
    round: u32,
    now: DateTime<Utc>,
) -> Result<()> {
    let pool = ctx.pool;
    let item: GroupItem =
        serde_json::from_str(&esc.item).context("alert of the escalation unreadable")?;
    let label = (step > 0 || round > 0).then(|| format!("[On-call · step {}]", step + 1));
    let message = alert_message(ctx, esc, &item, label, now);
    let targets = policy.steps.get(step).map(|s| s.targets.as_slice()).unwrap_or_default();
    let (people, channels, skipped) = callees(directory, targets, now);
    for reason in skipped {
        let mut event = now_event(now, "skipped");
        event.step = Some(step as i64);
        event.round = Some(i64::from(round));
        event.detail = reason;
        store::add_event(pool, esc.id, &event).await?;
    }
    for callee in people {
        let skip = |detail: String, user: Option<&User>| {
            let mut event = now_event(now, "skipped");
            event.step = Some(step as i64);
            event.round = Some(i64::from(round));
            event.user_id = Some(callee.user_id);
            event.user_name = user.map(display_name);
            event.detail = detail;
            event
        };
        let Some(user) = directory.users.get(&callee.user_id).filter(|u| !u.disabled) else {
            let event = skip("account missing or disabled".to_string(), None);
            store::add_event(pool, esc.id, &event).await?;
            continue;
        };
        let quiet = directory.prefs.get(&user.id).and_then(|p| p.quiet_hours.as_ref());
        if !escalation::may_disturb(quiet, callee.on_call, esc.severity, now) {
            let event = skip(
                "quiet hours: outside on-call duty only critical alerts get through".to_string(),
                Some(user),
            );
            store::add_event(pool, esc.id, &event).await?;
            continue;
        }
        let contacts: Vec<&ContactRow> = directory
            .contacts_of(user.id)
            .filter(|c| c.enabled && esc.severity >= c.min_severity)
            .collect();
        if contacts.is_empty() {
            let event =
                skip(format!("{}: no contact method for this severity", callee.reason), Some(user));
            store::add_event(pool, esc.id, &event).await?;
            continue;
        }
        for contact in contacts {
            let outcome = send_contact(ctx, directory, user, contact, &message, now).await;
            let mut event = contact_event(now, Some(step), Some(round), user, contact, &outcome);
            if outcome.is_ok() {
                event.detail = callee.reason.clone();
            }
            store::add_event(pool, esc.id, &event).await?;
        }
    }
    for channel_id in channels {
        let Some(config) = directory.channels.iter().find(|c| c.id == channel_id) else {
            let mut event = now_event(now, "skipped");
            event.step = Some(step as i64);
            event.round = Some(i64::from(round));
            event.detail = format!("channel {channel_id} no longer exists");
            store::add_event(pool, esc.id, &event).await?;
            continue;
        };
        let outcome = send_channel(ctx, config, &message, now).await;
        let event = channel_event(now, Some(step), Some(round), config, &outcome);
        store::add_event(pool, esc.id, &event).await?;
    }
    Ok(())
}

// --------------------------------------------------------------------------
// Relèves
// --------------------------------------------------------------------------

/// Annonce les prises et fins de garde des plannings qui le demandent. Une
/// relève n'est annoncée qu'une fois : la dernière personne vue est gardée en
/// base, et le tout premier passage l'enregistre sans rien envoyer.
async fn handoffs(ctx: &Ctx<'_>, directory: &Directory, now: DateTime<Utc>) -> Result<()> {
    for (row, overrides) in &directory.schedules {
        let current = row.definition.current(overrides, now);
        let current_user = current.map(|(slot, _, _)| slot.user_id);
        let previous = store::shift_state(ctx.pool, row.id).await?;
        let Some(previous) = previous else {
            store::set_shift_state(ctx.pool, row.id, current_user, now).await?;
            continue;
        };
        if previous == current_user {
            continue;
        }
        store::set_shift_state(ctx.pool, row.id, current_user, now).await?;
        if !row.definition.handoff_notifications {
            continue;
        }
        let tz = row.definition.tz();
        if let Some((slot, _, until)) = current
            && let Some(user) = directory.users.get(&slot.user_id)
        {
            let until = until.map_or_else(
                || "until further notice".to_string(),
                |end| format!("until {}", crate::oncall::schedule::local_label(tz, end)),
            );
            let message = plain_message(
                format!("You are now on call: {}", row.name),
                format!("Your on-call shift on \"{}\" has started, {until}.", row.name),
                now,
            );
            notify_person(ctx, directory, user, true, &message, now).await;
        }
        if let Some(previous) = previous
            && let Some(user) = directory.users.get(&previous)
        {
            let next = current_user
                .and_then(|id| directory.users.get(&id))
                .map_or_else(|| "nobody".to_string(), display_name);
            let message = plain_message(
                format!("Your on-call shift has ended: {}", row.name),
                format!("Your shift on \"{}\" is over. Now on call: {next}.", row.name),
                now,
            );
            let on_call = directory.on_call_now.contains(&previous);
            notify_person(ctx, directory, user, on_call, &message, now).await;
        }
    }
    Ok(())
}

/// Envoie un message d'information sur tous les contacts actifs d'une personne,
/// dans le respect de ses heures calmes.
async fn notify_person(
    ctx: &Ctx<'_>,
    directory: &Directory,
    user: &User,
    on_call: bool,
    message: &Message,
    now: DateTime<Utc>,
) {
    if user.disabled {
        return;
    }
    let quiet = directory.prefs.get(&user.id).and_then(|p| p.quiet_hours.as_ref());
    if !escalation::may_disturb(quiet, on_call, Severity::Info, now) {
        return;
    }
    for contact in
        directory.contacts_of(user.id).filter(|c| c.enabled && c.min_severity <= Severity::Info)
    {
        if let Err(error) = send_contact(ctx, directory, user, contact, message, now).await {
            warn!(contact = contact.id, %error, "on-call handoff notice not delivered");
        }
    }
}

// --------------------------------------------------------------------------
// Tests manuels
// --------------------------------------------------------------------------

/// Message de test d'un contact, envoyé depuis « Mon compte ».
pub async fn test_contact(ctx: &Ctx<'_>, user: &User, contact: &ContactRow) -> Result<(), String> {
    let now = Utc::now();
    let channels =
        db::alerts::list_channels(ctx.pool, ctx.cipher).await.map_err(|e| e.to_string())?;
    let config = contact_config(&channels, &display_name(user), contact)?;
    let message = plain_message(
        format!("DumbMonit — test of your contact \"{}\"", contact.label),
        "This message confirms DumbMonit can reach you here when you are on call.".to_string(),
        now,
    );
    let outcome = notify::deliver(ctx.http, &config, &message).await.error.map_or(Ok(()), Err);
    if let Err(error) = store::note_contact_use(
        ctx.pool,
        contact.id,
        outcome.as_ref().err().map(String::as_str),
        now,
    )
    .await
    {
        warn!(?error, "on-call contact outcome not recorded");
    }
    outcome
}

/// Bouton « Envoyer un message de test » d'un canal d'astreinte : la première
/// étape de sa politique reçoit un message de test, sans escalade.
pub async fn test_channel(ctx: &Ctx<'_>, config: &ChannelConfig) -> Result<(), NotifyError> {
    let now = Utc::now();
    let policy_id = store::policy_id_of(&config.settings).ok_or_else(|| {
        NotifyError::Config("choose an escalation policy for this channel".into())
    })?;
    let internal = |error: anyhow::Error| NotifyError::Transport(error.to_string());
    let policy = store::get_policy(ctx.pool, policy_id)
        .await
        .map_err(internal)?
        .ok_or_else(|| NotifyError::Config(format!("escalation policy {policy_id} not found")))?;
    let directory = Directory::load(ctx.pool, ctx.cipher, now).await.map_err(internal)?;
    let targets = policy.definition.steps.first().map(|s| s.targets.as_slice()).unwrap_or_default();
    let (people, channels, mut problems) = callees(&directory, targets, now);
    let message = plain_message(
        format!("DumbMonit — test of the on-call policy \"{}\"", policy.name),
        format!("This test reaches the first step of \"{}\". No alert is firing.", policy.name),
        now,
    );
    let mut reached = 0;
    for callee in people {
        let Some(user) = directory.users.get(&callee.user_id).filter(|u| !u.disabled) else {
            continue;
        };
        let mut any = false;
        for contact in directory.contacts_of(user.id).filter(|c| c.enabled) {
            any = true;
            match send_contact(ctx, &directory, user, contact, &message, now).await {
                Ok(()) => reached += 1,
                Err(error) => {
                    problems.push(format!("{} ({}): {error}", display_name(user), contact.label))
                }
            }
        }
        if !any {
            problems.push(format!("{} has no contact method", display_name(user)));
        }
    }
    for channel_id in channels {
        if let Some(channel) = directory.channels.iter().find(|c| c.id == channel_id) {
            match send_channel(ctx, channel, &message, now).await {
                Ok(()) => reached += 1,
                Err(error) => problems.push(format!("{}: {error}", channel.name)),
            }
        }
    }
    if reached == 0 {
        let detail = if problems.is_empty() {
            "the first step reaches nobody".to_string()
        } else {
            problems.join("; ")
        };
        return Err(NotifyError::Config(detail));
    }
    Ok(())
}

/// Purge horaire : le journal des escalades terminées depuis 90 jours.
pub async fn maintenance(pool: &SqlitePool, now: DateTime<Utc>) -> Result<u64> {
    store::purge_escalations(pool, now - TimeDelta::days(90)).await
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::*;
    use crate::alerting::machine::EffectivePhase;
    use crate::auth::users::{NewUser, Role};
    use crate::oncall::escalation::Step;
    use crate::oncall::schedule::{Definition, Layer, Rotation, RotationKind, Unit, parse_local};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn at(minutes: i64) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-10-26T08:00:00Z").expect("instant").with_timezone(&Utc)
            + TimeDelta::minutes(minutes)
    }

    async fn fixture() -> (std::path::PathBuf, SqlitePool, Cipher) {
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir()
            .join(format!("dumbmonit-oncall-{}-{unique}/dumbmonit.db", std::process::id()));
        let pool = db::open(&path).await.expect("open test database");
        let cipher =
            db::init_cipher(&pool, "a-test-secret-that-is-long-enough").await.expect("cipher");
        (path, pool, cipher)
    }

    async fn account(pool: &SqlitePool, cipher: &Cipher, name: &str) -> i64 {
        let id = users::insert(
            pool,
            NewUser {
                username: name,
                display_name: "",
                role: Role::Operator,
                password_hash: None,
                oidc: None,
            },
        )
        .await
        .expect("insert user")
        .expect("new user");
        store::save_contact(
            pool,
            cipher,
            &store::ContactDraft {
                id: None,
                user_id: id,
                label: "Phone".into(),
                kind: "ntfy".into(),
                via_channel_id: None,
                address: None,
                // `.invalid` ne se résout jamais (RFC 2606) : l'envoi échoue vite,
                // et le journal dit quand même qui a été appelé.
                settings: json!({"topic": name, "server_url": "https://ntfy.invalid"}),
                secrets: None,
                enabled: true,
                min_severity: Severity::Info,
            },
        )
        .await
        .expect("contact");
        id
    }

    fn item(fingerprint: &str) -> GroupItem {
        GroupItem {
            fingerprint: fingerprint.to_string(),
            rule_uid: "cpu".into(),
            rule_name: "CPU".into(),
            severity: Severity::Warning,
            reason: NotifyReason::Firing,
            value: Some(95.0),
            score: None,
            unit: "%".into(),
            operator: ">".into(),
            threshold: 90.0,
            series_key: "s".into(),
            since: Some(at(0)),
            phase: EffectivePhase::Firing,
            note: None,
            detail: None,
        }
    }

    fn digest(item: GroupItem) -> Digest {
        Digest {
            groups: vec![AlertGroup {
                target_id: None,
                target_name: "nas".into(),
                target_kind: String::new(),
                target_tags: BTreeMap::new(),
                severity: item.severity,
                items: vec![item],
                channels: Vec::new(),
                at: at(0),
            }],
            resolved_meanwhile: Vec::new(),
            hold: None,
            at: at(0),
        }
    }

    /// Qui a été appelé (ou dont l'appel a échoué), dans l'ordre.
    async fn called(pool: &SqlitePool) -> Vec<(String, Option<i64>)> {
        let escalation = store::list_escalations(pool, None, None, 10).await.expect("list");
        let mut out = Vec::new();
        for esc in escalation.iter().rev() {
            for event in store::events_of(pool, esc.id).await.expect("events") {
                if matches!(event.event.as_str(), "notified" | "failed") {
                    out.push((event.event.clone(), event.user_id));
                }
            }
        }
        out
    }

    #[tokio::test]
    async fn l_escalade_appelle_l_astreinte_puis_l_etape_suivante_et_s_arrete_a_l_acquittement() {
        let (path, pool, cipher) = fixture().await;
        let alice = account(&pool, &cipher, "alice").await;
        let bob = account(&pool, &cipher, "bob").await;

        let definition = Definition {
            timezone: "Europe/Paris".into(),
            handoff_notifications: false,
            layers: vec![Layer {
                name: "Base".into(),
                rotation: Rotation {
                    kind: RotationKind::Weekly,
                    start: parse_local("2026-10-05T09:00").expect("local"),
                    length: 1,
                    unit: Unit::Days,
                },
                participants: vec![alice],
                restrictions: Vec::new(),
            }],
        };
        let schedule =
            store::save_schedule(&pool, None, "Ops", "", &definition).await.expect("schedule");
        let policy = PolicyDef {
            repeat: 0,
            steps: vec![
                Step { delay_minutes: 10, targets: vec![Target::Schedule { id: schedule }] },
                Step { delay_minutes: 10, targets: vec![Target::User { id: bob }] },
            ],
        };
        let policy_id =
            store::save_policy(&pool, None, "Default", "", &policy).await.expect("policy");
        let channel_id = db::alerts::upsert_channel(
            &pool,
            &cipher,
            &db::alerts::ChannelDraft {
                id: None,
                name: "On-call".into(),
                kind: crate::oncall::CHANNEL_KIND.into(),
                enabled: true,
                settings: json!({"policy_id": policy_id}),
                secrets: None,
            },
        )
        .await
        .expect("channel");
        let channel = db::alerts::get_channel(&pool, &cipher, channel_id)
            .await
            .expect("read channel")
            .expect("channel exists");
        sqlx::query(
            "INSERT INTO alert_state (fingerprint, rule_uid, target_id, series_key, labels, phase,
                 severity, firing_since, last_eval_at)
             VALUES ('cpu@1', 'cpu', NULL, 's', '{}', 'firing', 'warning', ?, ?)",
        )
        .bind(at(0).to_rfc3339())
        .bind(at(0).to_rfc3339())
        .execute(&pool)
        .await
        .expect("alert");

        let http = notify::http_client();
        let ctx =
            Ctx { pool: &pool, cipher: &cipher, http: &http, public_url: None, action_key: None };
        engage(&ctx, &channel, &digest(item("cpu@1")), at(0)).await.expect("engage");
        // Un rappel du même épisode n'ouvre pas une seconde escalade.
        engage(&ctx, &channel, &digest(item("cpu@1")), at(1)).await.expect("engage again");
        assert_eq!(store::active_escalations(&pool).await.expect("active").len(), 1);

        tick(&ctx, at(0)).await.expect("tick 0");
        assert_eq!(called(&pool).await.iter().map(|c| c.1).collect::<Vec<_>>(), vec![Some(alice)]);
        tick(&ctx, at(5)).await.expect("tick 5");
        assert_eq!(called(&pool).await.len(), 1, "nobody else before the delay");
        tick(&ctx, at(10)).await.expect("tick 10");
        assert_eq!(
            called(&pool).await.iter().map(|c| c.1).collect::<Vec<_>>(),
            vec![Some(alice), Some(bob)],
            "step 2 after 10 minutes"
        );

        let ack = db::alerts::Ack { until: at(600), by: "bob".into(), note: None };
        assert!(db::alerts::set_ack(&pool, "cpu@1", Some(&ack)).await.expect("ack"));
        tick(&ctx, at(15)).await.expect("tick 15");
        tick(&ctx, at(60)).await.expect("tick 60");
        let escalations = store::list_escalations(&pool, None, None, 10).await.expect("list");
        assert_eq!(escalations[0].status, "acked");
        assert_eq!(escalations[0].acked_by.as_deref(), Some("bob"));
        assert_eq!(called(&pool).await.len(), 2, "nobody is called after the ack");

        pool.close().await;
        let _ = std::fs::remove_dir_all(path.parent().expect("dir"));
    }

    #[tokio::test]
    async fn la_resolution_referme_l_escalade() {
        let (path, pool, cipher) = fixture().await;
        let alice = account(&pool, &cipher, "alice").await;
        let policy = PolicyDef {
            repeat: 2,
            steps: vec![Step { delay_minutes: 5, targets: vec![Target::User { id: alice }] }],
        };
        let policy_id = store::save_policy(&pool, None, "P", "", &policy).await.expect("policy");
        let channel = ChannelConfig {
            id: 42,
            name: "On-call".into(),
            kind: crate::oncall::CHANNEL_KIND.into(),
            enabled: true,
            settings: json!({"policy_id": policy_id}),
            secrets: Value::Null,
        };
        let http = notify::http_client();
        let ctx =
            Ctx { pool: &pool, cipher: &cipher, http: &http, public_url: None, action_key: None };
        engage(&ctx, &channel, &digest(item("disk@1")), at(0)).await.expect("engage");
        // Aucune alerte active en base : l'escalade constate la résolution.
        tick(&ctx, at(0)).await.expect("tick");
        let escalations = store::list_escalations(&pool, None, None, 10).await.expect("list");
        assert_eq!(escalations[0].status, "resolved");

        let mut resolved = item("disk@1");
        resolved.reason = NotifyReason::Resolved;
        engage(&ctx, &channel, &digest(resolved), at(1)).await.expect("resolution");
        assert!(store::active_escalations(&pool).await.expect("active").is_empty());

        pool.close().await;
        let _ = std::fs::remove_dir_all(path.parent().expect("dir"));
    }

    fn contact(via: Option<i64>, address: Option<&str>) -> ContactRow {
        ContactRow {
            id: 1,
            user_id: 1,
            label: "Phone".into(),
            kind: "ntfy".into(),
            via_channel_id: via,
            address: address.map(str::to_string),
            settings: json!({"topic": "alice-oncall"}),
            secrets: Value::Null,
            has_secret: false,
            enabled: true,
            min_severity: Severity::Info,
            position: 1,
            last_error: None,
            last_used_at: None,
        }
    }

    fn team(kind: &str, settings: Value) -> ChannelConfig {
        ChannelConfig {
            id: 9,
            name: "Team mail".into(),
            kind: kind.into(),
            enabled: true,
            settings,
            secrets: json!({"password": "s3cret"}),
        }
    }

    #[test]
    fn un_contact_via_un_canal_d_equipe_ne_change_que_le_destinataire() {
        let channels =
            [team("smtp", json!({"host": "smtp.example.org", "to": ["ops@example.org"]}))];
        let config =
            contact_config(&channels, "Alice", &contact(Some(9), Some(" alice@example.org ")))
                .expect("config");
        assert_eq!(config.settings["to"], json!(["alice@example.org"]));
        assert_eq!(config.settings["host"], "smtp.example.org");
        assert_eq!(config.secrets["password"], "s3cret", "the team's credentials are used");

        let telegram = [team("telegram", json!({"chat_id": "-100", "message_thread_id": "4"}))];
        let config =
            contact_config(&telegram, "Alice", &contact(Some(9), Some("12345"))).expect("config");
        assert_eq!(config.settings["chat_id"], "12345");
        assert!(config.settings.get("message_thread_id").is_none());
    }

    #[test]
    fn un_contact_mal_relie_est_explique() {
        let discord = [team("discord", json!({}))];
        assert!(
            contact_config(&discord, "A", &contact(Some(9), Some("x")))
                .unwrap_err()
                .contains("cannot reach")
        );
        assert!(
            contact_config(&[], "A", &contact(Some(9), Some("x")))
                .unwrap_err()
                .contains("no longer exists")
        );
        let smtp = [team("smtp", json!({}))];
        assert!(
            contact_config(&smtp, "A", &contact(Some(9), None)).unwrap_err().contains("no address")
        );
        let own = contact_config(&[], "Alice", &contact(None, None)).expect("own service");
        assert_eq!(own.kind, "ntfy");
        assert_eq!(own.settings["topic"], "alice-oncall");
    }
}
