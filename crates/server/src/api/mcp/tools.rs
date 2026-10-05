//! Les outils que le serveur MCP expose à l'assistant.
//!
//! Chaque outil est une façade : il traduit des arguments JSON en appels aux
//! fonctions que l'interface web utilise déjà (`api::targets`, `api::alerts`,
//! `api::metrics`, `db::*`), puis rend un texte Markdown court — ce que le modèle
//! lit — doublé, quand c'est utile, d'un `structuredContent` JSON.
//!
//! Le vocabulaire est celui de l'interface : *reporting / unreachable / waiting*
//! pour un équipement, *info / advisory / warning* pour une alerte (la sévérité
//! interne `warning` s'affiche « advisory », `critical` s'affiche « warning »).

use std::collections::{BTreeMap, HashMap};

use axum::extract::{Path, Query, State};
use axum::{Extension, Json};
use chrono::{DateTime, SecondsFormat, TimeDelta, Utc};
use dumbmonit_proto::{Target, TargetId};
use serde_json::{Value, json};

use crate::alerting::model::Severity;
use crate::alerting::silence::Schedule;
use crate::api::alerts::{
    self, AckPayload, ActiveAlertView, EnablePayload, HistoryQuery, SilencePayload,
};
use crate::api::metrics::{self, RangeQuery};
use crate::api::{
    ApiError, agent_commands, channels, collectors, discovery, packs, status_pages, targets,
};
use crate::auth::middleware::{AdminIdentity, CurrentPrincipal, Principal};
use crate::auth::token::{ApiToken, Scope};
use crate::collectors::push::{self, Verdict};
use crate::db;
use crate::state::AppState;

/// Ce que l'assistant lit à l'initialisation, avant tout appel d'outil.
pub const INSTRUCTIONS: &str = "DumbMonit monitors a homelab or a small fleet: devices \
(SNMP, Proxmox, Synology, agents…) and services (HTTP, TCP, DNS, ping, TLS checks, \
heartbeats). Start with get_status to answer \"is everything fine?\". Device states are \
reporting, unreachable, waiting (no data yet), disabled, down (service check failing). \
Alert severities, from mild to serious, are info, advisory and warning; an alert \
\"building up\" is not firing yet; \"suppressed by parent\" means the device's parent is \
unreachable, so the alert is expected; \"acknowledged\" means someone knows and reminders \
are paused. Tools marked read-only never change anything; the others (silence_device, \
schedule_maintenance, remove_silence, acknowledge_alert, probe_device, set_device_enabled, \
set_rule_enabled, add_device, discover_network, restart_container, test_channel, \
post_incident) need a token with the write scope, and some act outside the instance \
(test_channel sends a real message, post_incident notifies status page subscribers): \
confirm with the user before calling them. Before add_device, call list_device_types \
with the kind to learn the address, credential and options it expects. Devices can be \
named by id or by name. Times are UTC, RFC 3339.";

/// Types de cibles qui surveillent un *service* : leur état vient du résultat de
/// la sonde (`dumbmonit_probe_success`), pas de la dernière interrogation.
const UPTIME_KINDS: [&str; 6] = ["http", "tcp", "dns", "ping", "tls", "push"];

/// Fenêtre de fraîcheur d'un résultat de sonde, alignée sur l'interface.
const PROBE_WINDOW: &str = "10m";

/// Nombre maximal de points renvoyés par série par `query_metrics` : un modèle
/// n'a que faire de deux mille points, et ils coûteraient autant de jetons.
const MAX_POINTS: usize = 60;

/// Plafonds de `query_metrics` et `alert_history`.
const MAX_RANGE_HOURS: f64 = 24.0 * 31.0;
const MAX_HISTORY_LIMIT: i64 = 500;
const DEFAULT_HISTORY_LIMIT: i64 = 50;

/// Durée maximale d'un silence posé par un assistant : au-delà d'une semaine,
/// c'est une règle à désactiver, pas une maintenance.
const MAX_SILENCE_HOURS: f64 = 24.0 * 7.0;

/// Durée d'un acquittement posé par un assistant quand il ne précise rien.
const DEFAULT_ACK_HOURS: f64 = 4.0;

// --------------------------------------------------------------------------
// Catalogue
// --------------------------------------------------------------------------

pub struct ToolSpec {
    pub name: &'static str,
    pub scope: Scope,
    /// Nom lisible, pour l'interface du client.
    title: &'static str,
    description: &'static str,
    input_schema: Value,
    /// Forme de `structuredContent` ; le serveur s'engage à la respecter.
    output_schema: Value,
    /// Écriture qui retire ou coupe quelque chose (silence supprimé, appareil
    /// désactivé, conteneur redémarré) plutôt que d'ajouter.
    destructive: bool,
    /// Rappeler l'outil avec les mêmes arguments ne change rien de plus.
    idempotent: bool,
    /// L'outil touche au monde extérieur (réseau balayé, message envoyé), pas
    /// seulement à l'instance.
    open_world: bool,
}

impl ToolSpec {
    fn read(
        name: &'static str,
        title: &'static str,
        description: &'static str,
        input_schema: Value,
        output_schema: Value,
    ) -> Self {
        Self {
            name,
            scope: Scope::Read,
            title,
            description,
            input_schema,
            output_schema,
            destructive: false,
            idempotent: true,
            open_world: false,
        }
    }

    fn write(
        name: &'static str,
        title: &'static str,
        description: &'static str,
        input_schema: Value,
        output_schema: Value,
    ) -> Self {
        Self {
            name,
            scope: Scope::Write,
            title,
            description,
            input_schema,
            output_schema,
            destructive: false,
            idempotent: false,
            open_world: false,
        }
    }

    fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }

    fn idempotent(mut self) -> Self {
        self.idempotent = true;
        self
    }

    fn open_world(mut self) -> Self {
        self.open_world = true;
        self
    }

    fn describe(&self) -> Value {
        json!({
            "name": self.name,
            "title": self.title,
            "description": self.description,
            "inputSchema": self.input_schema,
            "outputSchema": self.output_schema,
            "annotations": {
                "title": self.title,
                "readOnlyHint": self.scope == Scope::Read,
                "destructiveHint": self.destructive,
                "idempotentHint": self.idempotent,
                "openWorldHint": self.open_world,
            },
        })
    }
}

// --- Schémas -----------------------------------------------------------------

/// Argument « équipement » commun à plusieurs outils.
fn device_property() -> Value {
    json!({
        "type": ["string", "integer"],
        "description": "Device id (integer) or device name (case-insensitive; a unique \
                        substring is enough)."
    })
}

/// Aucun argument.
fn no_arguments() -> Value {
    json!({ "type": "object", "properties": {}, "additionalProperties": false })
}

fn nullable(kind: &str) -> Value {
    json!({ "type": [kind, "null"] })
}

fn object_of(properties: Value, required: &[&str]) -> Value {
    json!({ "type": "object", "properties": properties, "required": required })
}

fn list_of(item: Value) -> Value {
    json!({ "type": "array", "items": item })
}

/// Un équipement tel que `Device::summary` le rend.
fn device_schema() -> Value {
    object_of(
        json!({
            "id": { "type": "integer" },
            "name": { "type": "string" },
            "kind": { "type": "string" },
            "address": { "type": "string" },
            "state": { "type": "string", "enum": ["reporting", "unreachable", "waiting", "disabled", "down", "unknown"] },
            "enabled": { "type": "boolean" },
            "parent_id": nullable("integer"),
            "parent": nullable("string"),
            "tags": { "type": "object", "additionalProperties": { "type": "string" } },
            "interval_secs": { "type": "integer" },
            "last_probe_at": nullable("string"),
            "last_error": nullable("string"),
        }),
        &["id", "name", "kind", "address", "state", "enabled"],
    )
}

/// Une alerte telle que `alert_json` la rend.
fn alert_schema() -> Value {
    object_of(
        json!({
            "fingerprint": { "type": "string" },
            "rule_uid": { "type": "string" },
            "rule": { "type": "string" },
            "severity": { "type": "string", "enum": ["info", "advisory", "warning"] },
            "state": { "type": "string" },
            "device_id": nullable("integer"),
            "device": nullable("string"),
            "suppressed_by": nullable("string"),
            "silenced": { "type": "boolean" },
            "acknowledged": { "type": "boolean" },
            "acked_by": nullable("string"),
            "acked_until": nullable("string"),
            "ack_note": nullable("string"),
            "value": nullable("number"),
            "since": nullable("string"),
            "labels": { "type": "object" },
        }),
        &["fingerprint", "rule_uid", "rule", "severity", "state", "silenced", "acknowledged"],
    )
}

fn specs() -> Vec<ToolSpec> {
    vec![
        // --- Lecture ---------------------------------------------------------
        ToolSpec::read(
            "get_status",
            "Overall status",
            "The one-second answer to \"is everything fine?\": device counts (reporting, \
             unreachable, waiting), firing and building-up alerts, and a one-sentence \
             bulletin. Call this first.",
            no_arguments(),
            object_of(
                json!({
                    "sentence": { "type": "string" },
                    "devices": { "type": "object" },
                    "alerts": { "type": "object" },
                    "unreachable_devices": list_of(device_schema()),
                    "firing": list_of(alert_schema()),
                }),
                &["sentence", "devices", "alerts", "unreachable_devices", "firing"],
            ),
        ),
        ToolSpec::read(
            "list_devices",
            "List devices",
            "Lists monitored devices and services with id, kind, address, state (reporting / \
             unreachable / waiting / disabled / down), parent and tags. Filter by name \
             substring and/or state.",
            json!({
                "type": "object",
                "properties": {
                    "filter": { "type": "string", "description": "Case-insensitive substring matched against name, address, kind and tags." },
                    "state": { "type": "string", "enum": ["reporting", "unreachable", "waiting", "disabled", "down", "unknown"], "description": "Only devices in this state." }
                },
                "additionalProperties": false
            }),
            object_of(json!({ "devices": list_of(device_schema()) }), &["devices"]),
        ),
        ToolSpec::read(
            "get_device",
            "Device details",
            "Details of one device: configuration, state, last probe, active alerts on it, \
             and a 24-hour summary of its key metrics (CPU, memory, fullest disk; \
             availability and latency for a service check).",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": "Device id." },
                    "name": { "type": "string", "description": "Device name, if the id is unknown." }
                },
                "additionalProperties": false
            }),
            {
                let mut schema = device_schema();
                schema["properties"]["profile_id"] = nullable("string");
                schema["properties"]["metrics_24h"] = json!({ "type": "object" });
                schema["properties"]["alerts"] = list_of(alert_schema());
                schema["required"] = json!(["id", "name", "kind", "state", "metrics_24h", "alerts"]);
                schema
            },
        ),
        ToolSpec::read(
            "list_alerts",
            "List active alerts",
            "Active alerts: severity (info / advisory / warning), rule, device, since when, \
             current value, and whether it is suppressed by an unreachable parent. Alerts \
             still building up (condition true, hold time not elapsed) are included only \
             with include_pending.",
            json!({
                "type": "object",
                "properties": {
                    "include_pending": { "type": "boolean", "default": false, "description": "Also list alerts that are building up." },
                    "device": device_property()
                },
                "additionalProperties": false
            }),
            object_of(json!({ "alerts": list_of(alert_schema()) }), &["alerts"]),
        ),
        ToolSpec::read(
            "alert_history",
            "Alert history",
            "What happened: alert transitions (started firing, resolved…) most recent first. \
             Use it for \"what happened last night?\". Defaults to the last 24 hours.",
            json!({
                "type": "object",
                "properties": {
                    "since": { "type": "string", "description": "Lower bound, RFC 3339 (e.g. 2026-09-14T22:00:00Z). Takes precedence over hours." },
                    "hours": { "type": "number", "description": "Look back this many hours (default 24).", "minimum": 0 },
                    "device": device_property(),
                    "limit": { "type": "integer", "description": "Maximum number of entries (default 50, max 500).", "minimum": 1 }
                },
                "additionalProperties": false
            }),
            object_of(
                json!({
                    "since": { "type": "string" },
                    "entries": list_of(object_of(
                        json!({
                            "at": { "type": "string" },
                            "rule_uid": { "type": "string" },
                            "rule": nullable("string"),
                            "device_id": nullable("integer"),
                            "device": nullable("string"),
                            "from": { "type": "string" },
                            "to": { "type": "string" },
                            "severity": { "type": "string" },
                            "value": nullable("number"),
                            "notified": { "type": "boolean" },
                            "reason": { "type": "string" },
                        }),
                        &["at", "rule_uid", "from", "to", "severity", "notified"],
                    )),
                }),
                &["since", "entries"],
            ),
        ),
        ToolSpec::read(
            "query_metrics",
            "Query metrics",
            "Runs a MetricsQL/PromQL range query against the time series (VictoriaMetrics). \
             Metrics are prefixed dumbmonit_ and carry a target label with the device id, \
             e.g. dumbmonit_cpu_usage_percent{target=\"3\"}. Returns at most 60 points per \
             series. Prefer get_device for the usual CPU/memory/disk summary.",
            json!({
                "type": "object",
                "required": ["query"],
                "properties": {
                    "query": { "type": "string", "description": "MetricsQL expression." },
                    "range_hours": { "type": "number", "description": "How far back to look (default 1, max 744).", "minimum": 0 },
                    "step_secs": { "type": "integer", "description": "Sampling step in seconds; widened automatically to stay under 60 points.", "minimum": 1 }
                },
                "additionalProperties": false
            }),
            object_of(
                json!({
                    "query": { "type": "string" },
                    "step_secs": { "type": "integer" },
                    "series": list_of(object_of(
                        json!({ "metric": { "type": "object" }, "values": { "type": "array" } }),
                        &["metric", "values"],
                    )),
                }),
                &["query", "series"],
            ),
        ),
        ToolSpec::read(
            "list_silences",
            "List maintenance windows",
            "Lists maintenance windows (silences): id, name, device, schedule, and whether it \
             is active right now.",
            no_arguments(),
            object_of(
                json!({
                    "silences": list_of(object_of(
                        json!({
                            "id": { "type": "integer" },
                            "name": { "type": "string" },
                            "comment": { "type": "string" },
                            "device_id": nullable("integer"),
                            "device": nullable("string"),
                            "matchers": { "type": "object" },
                            "schedule": { "type": "object" },
                            "enabled": { "type": "boolean" },
                            "active_now": { "type": "boolean" },
                        }),
                        &["id", "name", "enabled", "active_now"],
                    )),
                }),
                &["silences"],
            ),
        ),
        ToolSpec::read(
            "list_rules",
            "List alert rules",
            "Lists alert rules: uid, name, kind (threshold / anomaly / predict), severity, \
             threshold, hold time, and whether it is enabled or built in.",
            no_arguments(),
            object_of(
                json!({
                    "rules": list_of(object_of(
                        json!({
                            "id": { "type": "integer" },
                            "uid": { "type": "string" },
                            "name": { "type": "string" },
                            "kind": { "type": "string" },
                            "severity": { "type": "string" },
                            "enabled": { "type": "boolean" },
                            "builtin": { "type": "boolean" },
                        }),
                        &["id", "uid", "name", "kind", "severity", "enabled", "builtin"],
                    )),
                }),
                &["rules"],
            ),
        ),
        ToolSpec::read(
            "list_device_types",
            "Device types",
            "The kinds of device this server can monitor (SNMP, Proxmox, Synology, agent, \
             HTTP/TCP/DNS/ping/TLS checks, heartbeats, integration packs…). Without \
             arguments: one line per kind. With kind: what to prepare, the address to give, \
             the accepted credentials and their fields, and the options — exactly what \
             add_device expects.",
            json!({
                "type": "object",
                "properties": {
                    "kind": { "type": "string", "description": "A kind from the list, e.g. snmp, proxmox, http." }
                },
                "additionalProperties": false
            }),
            object_of(
                json!({
                    "kinds": list_of(object_of(
                        json!({
                            "kind": { "type": "string" },
                            "label": { "type": "string" },
                            "summary": { "type": "string" },
                        }),
                        &["kind", "label", "summary"],
                    )),
                }),
                &["kinds"],
            ),
        ),
        ToolSpec::read(
            "list_agents",
            "List agents",
            "Machines running the DumbMonit agent: hostname, OS, agent version, whether it \
             relays probes for a remote site (and how many devices go through it), last \
             contact, binding state and whether it accepts commands.",
            no_arguments(),
            object_of(
                json!({
                    "agents": list_of(object_of(
                        json!({
                            "id": { "type": "integer" },
                            "name": { "type": "string" },
                            "hostname": { "type": "string" },
                            "os": { "type": "string" },
                            "agent_version": { "type": "string" },
                            "relay": { "type": "boolean" },
                            "site": nullable("string"),
                            "relayed": { "type": "integer" },
                            "last_seen_at": nullable("string"),
                            "binding": { "type": "string" },
                            "commands_supported": { "type": "boolean" },
                        }),
                        &["id", "name", "hostname", "relay", "relayed", "binding"],
                    )),
                }),
                &["agents"],
            ),
        ),
        ToolSpec::read(
            "list_containers",
            "List containers",
            "Docker containers of a machine that runs the agent: running or not, health, \
             restarts, uptime, whether an image update is available, and the last command \
             sent to each.",
            json!({
                "type": "object",
                "required": ["device"],
                "properties": { "device": device_property() },
                "additionalProperties": false
            }),
            object_of(
                json!({
                    "device_id": { "type": "integer" },
                    "device": { "type": "string" },
                    "containers": list_of(object_of(
                        json!({
                            "name": { "type": "string" },
                            "image": { "type": "string" },
                            "up": { "type": "boolean" },
                            "health": { "type": "string" },
                        }),
                        &["name", "image", "up", "health"],
                    )),
                }),
                &["device_id", "device", "containers"],
            ),
        ),
        ToolSpec::read(
            "list_heartbeats",
            "List heartbeats",
            "Heartbeat (push) monitors — jobs that call a secret URL each time they run: \
             expected interval, last call, what it reported, and the verdict (waiting, \
             on_time, missed, reported_down). The secret URL itself is never shown.",
            no_arguments(),
            object_of(
                json!({
                    "heartbeats": list_of(object_of(
                        json!({
                            "device_id": { "type": "integer" },
                            "device": { "type": "string" },
                            "enabled": { "type": "boolean" },
                            "verdict": { "type": "string" },
                            "expected_interval_secs": nullable("integer"),
                            "grace_secs": nullable("integer"),
                            "last_seen_at": nullable("string"),
                            "last_seen_age_secs": nullable("integer"),
                            "last_status": nullable("string"),
                            "last_message": nullable("string"),
                            "received_total": { "type": "integer" },
                            "settings_error": nullable("string"),
                        }),
                        &["device_id", "device", "enabled", "verdict", "received_total"],
                    )),
                }),
                &["heartbeats"],
            ),
        ),
        ToolSpec::read(
            "list_status_pages",
            "List status pages",
            "Status pages (title, public path /s/<slug>, published or draft, how many devices \
             they show) and incidents — open ones first, with their latest update.",
            no_arguments(),
            object_of(
                json!({
                    "pages": list_of(object_of(
                        json!({
                            "id": { "type": "integer" },
                            "title": { "type": "string" },
                            "slug": { "type": "string" },
                            "path": { "type": "string" },
                            "published": { "type": "boolean" },
                            "devices": { "type": "integer" },
                        }),
                        &["id", "title", "slug", "path", "published", "devices"],
                    )),
                    "incidents": list_of(object_of(
                        json!({
                            "id": { "type": "integer" },
                            "page_id": nullable("integer"),
                            "title": { "type": "string" },
                            "kind": { "type": "string" },
                            "status": { "type": "string" },
                            "severity": { "type": "string" },
                            "open": { "type": "boolean" },
                            "starts_at": { "type": "string" },
                            "ends_at": nullable("string"),
                            "latest_update": nullable("string"),
                        }),
                        &["id", "title", "kind", "status", "severity", "open"],
                    )),
                }),
                &["pages", "incidents"],
            ),
        ),
        ToolSpec::read(
            "list_channels",
            "List notification channels",
            "Notification channels (Discord, ntfy, email…): name, kind, enabled, when a \
             message last left, and the last error. Their secrets are never returned.",
            no_arguments(),
            object_of(
                json!({
                    "channels": list_of(object_of(
                        json!({
                            "id": { "type": "integer" },
                            "name": { "type": "string" },
                            "kind": { "type": "string" },
                            "enabled": { "type": "boolean" },
                            "has_secret": { "type": "boolean" },
                            "last_error": nullable("string"),
                            "last_sent_at": nullable("string"),
                        }),
                        &["id", "name", "kind", "enabled"],
                    )),
                }),
                &["channels"],
            ),
        ),
        ToolSpec::read(
            "list_packs",
            "List integration packs",
            "Installed integration packs (device types described in YAML): id, version, the \
             device kind they add, whether enabled, how many devices use them, and any error.",
            no_arguments(),
            object_of(
                json!({
                    "packs": list_of(object_of(
                        json!({
                            "id": { "type": "string" },
                            "version": { "type": "string" },
                            "label": { "type": "string" },
                            "kind": nullable("string"),
                            "enabled": { "type": "boolean" },
                            "targets": { "type": "integer" },
                            "error": nullable("string"),
                        }),
                        &["id", "version", "label", "enabled", "targets"],
                    )),
                }),
                &["packs"],
            ),
        ),
        // --- Écriture --------------------------------------------------------
        ToolSpec::write(
            "silence_device",
            "Silence a device",
            "Silences every alert of one device for a while (a one-off maintenance window \
             starting now). Alerts keep being evaluated but nothing is notified. Returns the \
             silence id and when it ends.",
            json!({
                "type": "object",
                "required": ["device"],
                "properties": {
                    "device": device_property(),
                    "hours": { "type": "number", "description": "Duration in hours (default 1, max 168).", "exclusiveMinimum": 0 },
                    "comment": { "type": "string", "description": "Why — shown in the UI next to the silence." }
                },
                "additionalProperties": false
            }),
            object_of(
                json!({
                    "id": { "type": "integer" },
                    "device_id": { "type": "integer" },
                    "device": { "type": "string" },
                    "starts_at": { "type": "string" },
                    "ends_at": { "type": "string" },
                }),
                &["id", "device_id", "device", "starts_at", "ends_at"],
            ),
        ),
        ToolSpec::write(
            "schedule_maintenance",
            "Schedule maintenance",
            "Plans a one-off maintenance window at a given time — for one device, or for the \
             whole instance when no device is named. Alerts in the window are evaluated but \
             not notified. At most 7 days long.",
            json!({
                "type": "object",
                "required": ["starts_at"],
                "properties": {
                    "starts_at": { "type": "string", "description": "Start, RFC 3339 (e.g. 2026-10-04T22:00:00Z)." },
                    "ends_at": { "type": "string", "description": "End, RFC 3339. Alternatively give hours." },
                    "hours": { "type": "number", "description": "Duration in hours when ends_at is not given (default 1, max 168).", "exclusiveMinimum": 0 },
                    "device": device_property(),
                    "name": { "type": "string", "description": "Short name shown in the UI." },
                    "comment": { "type": "string", "description": "Why." }
                },
                "additionalProperties": false
            }),
            object_of(
                json!({
                    "id": { "type": "integer" },
                    "name": { "type": "string" },
                    "device_id": nullable("integer"),
                    "device": nullable("string"),
                    "starts_at": { "type": "string" },
                    "ends_at": { "type": "string" },
                    "active_now": { "type": "boolean" },
                }),
                &["id", "name", "starts_at", "ends_at", "active_now"],
            ),
        ),
        ToolSpec::write(
            "remove_silence",
            "Remove a maintenance window",
            "Removes a maintenance window by id (see list_silences). Alerts on the device \
             notify again.",
            json!({
                "type": "object",
                "required": ["id"],
                "properties": { "id": { "type": "integer", "description": "Silence id." } },
                "additionalProperties": false
            }),
            object_of(
                json!({ "id": { "type": "integer" }, "removed": { "type": "boolean" } }),
                &["id", "removed"],
            ),
        )
        .destructive()
        .idempotent(),
        ToolSpec::write(
            "acknowledge_alert",
            "Acknowledge an alert",
            "Acknowledges one alert by fingerprint (see list_alerts): \"I know, stop reminding \
             me\". Reminders and escalations pause for the given hours (default 4, at most \
             720); the alert keeps being evaluated and its resolution is still notified. Pass \
             hours 0 to lift an acknowledgement. To mute a whole device, use silence_device.",
            json!({
                "type": "object",
                "required": ["fingerprint"],
                "properties": {
                    "fingerprint": { "type": "string", "description": "Alert fingerprint (see list_alerts)." },
                    "hours": { "type": "number", "description": "Duration in hours (default 4, max 720); 0 lifts the acknowledgement.", "minimum": 0 },
                    "note": { "type": "string", "description": "Why — shown in the UI next to the alert." }
                },
                "additionalProperties": false
            }),
            alert_schema(),
        ),
        ToolSpec::write(
            "probe_device",
            "Probe a device now",
            "Probes a device right now instead of waiting for its next scheduled poll, and \
             reports what was measured — or why it failed. Useful to check whether an outage \
             is over.",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": "Device id." },
                    "name": { "type": "string", "description": "Device name, if the id is unknown." }
                },
                "additionalProperties": false
            }),
            object_of(
                json!({
                    "device_id": { "type": "integer" },
                    "device": { "type": "string" },
                    "ok": { "type": "boolean" },
                    "sample_count": { "type": "integer" },
                    "series": list_of(json!({ "type": "string" })),
                }),
                &["device_id", "device", "ok", "sample_count", "series"],
            ),
        )
        .open_world(),
        ToolSpec::write(
            "set_device_enabled",
            "Enable or disable a device",
            "Enables or disables monitoring of a device. A disabled device is not polled and \
             raises no alert; its history is kept.",
            json!({
                "type": "object",
                "required": ["enabled"],
                "properties": {
                    "id": { "type": "integer", "description": "Device id." },
                    "name": { "type": "string", "description": "Device name, if the id is unknown." },
                    "enabled": { "type": "boolean" }
                },
                "additionalProperties": false
            }),
            object_of(
                json!({
                    "device_id": { "type": "integer" },
                    "device": { "type": "string" },
                    "enabled": { "type": "boolean" },
                }),
                &["device_id", "device", "enabled"],
            ),
        )
        .destructive()
        .idempotent(),
        ToolSpec::write(
            "set_rule_enabled",
            "Enable or disable an alert rule",
            "Enables or disables an alert rule by uid or id. Disabling a rule resolves its \
             active alerts at the next evaluation.",
            json!({
                "type": "object",
                "required": ["enabled"],
                "properties": {
                    "uid": { "type": "string", "description": "Rule uid (see list_rules)." },
                    "id": { "type": "integer", "description": "Rule id, alternatively." },
                    "enabled": { "type": "boolean" }
                },
                "additionalProperties": false
            }),
            object_of(
                json!({
                    "id": { "type": "integer" },
                    "uid": { "type": "string" },
                    "name": { "type": "string" },
                    "enabled": { "type": "boolean" },
                }),
                &["id", "uid", "name", "enabled"],
            ),
        )
        .destructive()
        .idempotent(),
        ToolSpec::write(
            "add_device",
            "Add a device",
            "Adds a device or service to monitor. Call list_device_types first (with the \
             kind) to learn the address format, the credential shape and the options. The \
             credential is stored encrypted and never returned; prefer the web interface \
             for secrets you would rather not type into a conversation.",
            json!({
                "type": "object",
                "required": ["name", "kind", "address"],
                "properties": {
                    "name": { "type": "string", "description": "Display name, unique and short (\"NAS\", \"Core switch\")." },
                    "kind": { "type": "string", "description": "Device type, from list_device_types (snmp, proxmox, http, ping…)." },
                    "address": { "type": "string", "description": "Host name, IP address or URL, as list_device_types describes for this kind." },
                    "interval_secs": { "type": "integer", "minimum": 10, "description": "Polling period in seconds (default 60)." },
                    "parent": device_property(),
                    "via_agent": device_property(),
                    "options": { "type": "object", "additionalProperties": { "type": ["string", "number", "boolean"] }, "description": "Kind-specific options by key (see list_device_types)." },
                    "tags": { "type": "object", "additionalProperties": { "type": "string" }, "description": "Free labels (role, site…)." },
                    "credential": { "type": "object", "description": "Credential object with its \"type\" (none, snmp_community, snmp_v3, api_token, username_password) and the fields list_device_types gives for it." },
                    "enabled": { "type": "boolean", "description": "Start monitoring right away (default true)." }
                },
                "additionalProperties": false
            }),
            object_of(
                json!({
                    "id": { "type": "integer" },
                    "name": { "type": "string" },
                    "address": { "type": "string" },
                    "kind": { "type": "string" },
                    "enabled": { "type": "boolean" },
                    "interval_secs": { "type": "integer" },
                    "credential_kind": { "type": "string" },
                }),
                &["id", "name", "address", "kind", "enabled", "interval_secs", "credential_kind"],
            ),
        ),
        ToolSpec::write(
            "discover_network",
            "Discover SNMP devices",
            "Scans a network (CIDR, 4096 addresses at most) for devices that answer SNMP, and \
             lists them with the profile auto-detection would apply. Nothing is added: use \
             add_device (kind snmp) for the ones to monitor. The server sends the probes.",
            json!({
                "type": "object",
                "required": ["cidr"],
                "properties": {
                    "cidr": { "type": "string", "description": "Network to scan, e.g. 192.168.1.0/24." },
                    "community": { "type": "string", "description": "SNMP v2c community (default public)." },
                    "port": { "type": "integer", "description": "UDP port (default 161)." },
                    "timeout_ms": { "type": "integer", "description": "Per-address timeout, 100 to 10000 ms." }
                },
                "additionalProperties": false
            }),
            object_of(
                json!({
                    "scanned": { "type": "integer" },
                    "devices": list_of(object_of(
                        json!({
                            "address": { "type": "string" },
                            "sysname": nullable("string"),
                            "sysdescr": nullable("string"),
                            "sysobjectid": nullable("string"),
                            "suggested_profile": nullable("string"),
                        }),
                        &["address"],
                    )),
                }),
                &["scanned", "devices"],
            ),
        )
        .idempotent()
        .open_world(),
        ToolSpec::write(
            "restart_container",
            "Restart a container",
            "Asks the agent of a machine to restart one of its Docker containers (see \
             list_containers). The command is queued; the agent runs it within seconds and \
             list_containers shows the outcome.",
            json!({
                "type": "object",
                "required": ["device", "container"],
                "properties": {
                    "device": device_property(),
                    "container": { "type": "string", "description": "Container name." }
                },
                "additionalProperties": false
            }),
            object_of(
                json!({
                    "command_id": { "type": "integer" },
                    "device_id": { "type": "integer" },
                    "device": { "type": "string" },
                    "container": { "type": "string" },
                    "status": { "type": "string" },
                }),
                &["command_id", "device_id", "device", "container", "status"],
            ),
        )
        .destructive(),
        ToolSpec::write(
            "test_channel",
            "Test a notification channel",
            "Sends a test message through one notification channel (see list_channels), by \
             the exact path real alerts take, and says whether it left.",
            json!({
                "type": "object",
                "required": ["channel"],
                "properties": {
                    "channel": { "type": ["string", "integer"], "description": "Channel id or name." }
                },
                "additionalProperties": false
            }),
            object_of(
                json!({
                    "channel_id": { "type": "integer" },
                    "channel": { "type": "string" },
                    "ok": { "type": "boolean" },
                    "message": { "type": "string" },
                }),
                &["channel_id", "channel", "ok", "message"],
            ),
        )
        .open_world(),
        ToolSpec::write(
            "post_incident",
            "Post an incident update",
            "Posts on the status pages: opens an incident (or announces a maintenance) with a \
             first message, or — with incident_id — adds an update to an existing one and \
             moves its status. Email subscribers of the page are notified. Incident statuses: \
             investigating, identified, monitoring, resolved; maintenance: scheduled, \
             in_progress, completed.",
            json!({
                "type": "object",
                "required": ["body"],
                "properties": {
                    "body": { "type": "string", "description": "The message readers will see." },
                    "incident_id": { "type": "integer", "description": "Add an update to this incident instead of opening a new one (see list_status_pages)." },
                    "title": { "type": "string", "description": "Title of a new incident." },
                    "page": { "type": ["string", "integer"], "description": "Status page id, slug or title for a new incident; omit for every page." },
                    "kind": { "type": "string", "enum": ["incident", "maintenance"], "description": "Default incident." },
                    "status": { "type": "string", "enum": ["investigating", "identified", "monitoring", "resolved", "scheduled", "in_progress", "completed"] },
                    "severity": { "type": "string", "enum": ["minor", "major"] }
                },
                "additionalProperties": false
            }),
            object_of(
                json!({
                    "id": { "type": "integer" },
                    "title": { "type": "string" },
                    "kind": { "type": "string" },
                    "status": { "type": "string" },
                    "severity": { "type": "string" },
                    "page_id": nullable("integer"),
                    "updates": { "type": "integer" },
                }),
                &["id", "title", "kind", "status", "severity", "updates"],
            ),
        )
        .open_world(),
    ]
}

/// Le catalogue tel que `tools/list` le renvoie.
pub fn catalogue() -> Vec<Value> {
    specs().iter().map(ToolSpec::describe).collect()
}

pub fn find(name: &str) -> Option<ToolSpec> {
    specs().into_iter().find(|spec| spec.name == name)
}

// --------------------------------------------------------------------------
// Résultats et erreurs
// --------------------------------------------------------------------------

pub struct ToolOutput {
    pub text: String,
    pub structured: Option<Value>,
}

impl ToolOutput {
    fn new(text: String, structured: Value) -> Self {
        Self { text, structured: Some(structured) }
    }
}

pub enum ToolError {
    /// L'outil n'a pas pu faire ce qu'on lui demandait ; le texte est pour
    /// l'assistant, qui le reformulera.
    Failed(String),
    /// Panne interne : journalisée, jamais détaillée au client.
    Internal(anyhow::Error),
}

impl From<ApiError> for ToolError {
    fn from(error: ApiError) -> Self {
        match error {
            ApiError::Internal(error) => Self::Internal(error),
            other => Self::Failed(other.into_parts().1),
        }
    }
}

impl From<anyhow::Error> for ToolError {
    fn from(error: anyhow::Error) -> Self {
        Self::Internal(error)
    }
}

type ToolResult = Result<ToolOutput, ToolError>;

fn failed(message: impl Into<String>) -> ToolError {
    ToolError::Failed(message.into())
}

/// Lecture tolérante des arguments : un modèle envoie volontiers `"3"` pour 3.
struct Args(Value);

impl Args {
    fn str(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty())
    }

    fn i64(&self, key: &str) -> Option<i64> {
        let value = self.0.get(key)?;
        value.as_i64().or_else(|| value.as_str()?.trim().parse().ok())
    }

    fn f64(&self, key: &str) -> Option<f64> {
        let value = self.0.get(key)?;
        value.as_f64().or_else(|| value.as_str()?.trim().parse().ok())
    }

    fn bool(&self, key: &str) -> Option<bool> {
        let value = self.0.get(key)?;
        value.as_bool().or_else(|| match value.as_str()?.trim() {
            "true" | "yes" | "on" | "1" => Some(true),
            "false" | "no" | "off" | "0" => Some(false),
            _ => None,
        })
    }

    fn required_bool(&self, key: &str) -> Result<bool, ToolError> {
        self.bool(key).ok_or_else(|| failed(format!("\"{key}\" (true or false) is required.")))
    }

    /// Désignation d'équipement : `device`, `id` ou `name`, au choix de l'appelant.
    fn device_ref(&self) -> Option<String> {
        if let Some(id) = self.i64("id") {
            return Some(id.to_string());
        }
        for key in ["device", "name"] {
            if let Some(text) = self.str(key) {
                return Some(text.to_string());
            }
            if let Some(id) = self.i64(key) {
                return Some(id.to_string());
            }
        }
        None
    }
}

// --------------------------------------------------------------------------
// Répartition
// --------------------------------------------------------------------------

pub async fn call(state: &AppState, token: &ApiToken, name: &str, arguments: Value) -> ToolResult {
    let args = Args(arguments);
    match name {
        "get_status" => get_status(state).await,
        "list_devices" => list_devices(state, &args).await,
        "get_device" => get_device(state, &args).await,
        "list_alerts" => list_alerts(state, &args).await,
        "alert_history" => alert_history(state, &args).await,
        "query_metrics" => query_metrics(state, &args).await,
        "list_silences" => list_silences(state).await,
        "list_rules" => list_rules(state).await,
        "list_device_types" => list_device_types(state, &args).await,
        "list_agents" => list_agents(state).await,
        "list_containers" => list_containers(state, &args).await,
        "list_heartbeats" => list_heartbeats(state).await,
        "list_status_pages" => list_status_pages(state).await,
        "list_channels" => list_channels(state).await,
        "list_packs" => list_packs(state, token).await,
        "silence_device" => silence_device(state, &args).await,
        "schedule_maintenance" => schedule_maintenance(state, &args).await,
        "remove_silence" => remove_silence(state, &args).await,
        "acknowledge_alert" => acknowledge_alert(state, &args).await,
        "probe_device" => probe_device(state, &args).await,
        "set_device_enabled" => set_device_enabled(state, &args).await,
        "set_rule_enabled" => set_rule_enabled(state, &args).await,
        "add_device" => add_device(state, &args).await,
        "discover_network" => discover_network(token, &args).await,
        "restart_container" => restart_container(state, token, &args).await,
        "test_channel" => test_channel(state, &args).await,
        "post_incident" => post_incident(state, &args).await,
        other => Err(failed(format!("Unknown tool: {other}"))),
    }
}

// --------------------------------------------------------------------------
// Équipements et leur état
// --------------------------------------------------------------------------

/// État d'un équipement, avec les mots de l'interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeviceState {
    Reporting,
    Unreachable,
    Waiting,
    Disabled,
    Down,
    Unknown,
}

impl DeviceState {
    fn word(self) -> &'static str {
        match self {
            Self::Reporting => "reporting",
            Self::Unreachable => "unreachable",
            Self::Waiting => "waiting",
            Self::Disabled => "disabled",
            Self::Down => "down",
            Self::Unknown => "unknown",
        }
    }

    fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "reporting" | "online" | "up" => Some(Self::Reporting),
            "unreachable" | "offline" => Some(Self::Unreachable),
            "waiting" | "pending" => Some(Self::Waiting),
            "disabled" => Some(Self::Disabled),
            "down" => Some(Self::Down),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }

    fn is_trouble(self) -> bool {
        matches!(self, Self::Unreachable | Self::Down)
    }
}

struct Device {
    target: Target,
    state: DeviceState,
    last_probe_at: Option<String>,
    last_error: Option<String>,
    /// Motif du dernier échec d'une sonde de service, quand il est connu.
    probe_reason: Option<String>,
}

impl Device {
    fn id(&self) -> TargetId {
        self.target.id
    }

    fn name(&self) -> &str {
        &self.target.name
    }

    fn summary(&self, parents: &HashMap<TargetId, String>) -> Value {
        json!({
            "id": self.target.id,
            "name": self.target.name,
            "kind": self.target.kind,
            "address": self.target.address,
            "state": self.state.word(),
            "enabled": self.target.enabled,
            "parent_id": self.target.parent_id,
            "parent": self.target.parent_id.and_then(|id| parents.get(&id)),
            "tags": self.target.tags,
            "interval_secs": self.target.interval.as_secs(),
            "last_probe_at": self.last_probe_at,
            "last_error": self.last_error,
        })
    }

    fn line(&self, parents: &HashMap<TargetId, String>) -> String {
        let mut line = format!(
            "- **{}** (#{}, {}, {}) — {}",
            self.target.name,
            self.target.id,
            self.target.kind,
            self.target.address,
            self.state.word()
        );
        if let Some(detail) = self.detail() {
            line.push_str(&format!(": {detail}"));
        }
        if let Some(at) = &self.last_probe_at {
            line.push_str(&format!("; last probe {at}"));
        }
        if let Some(parent) = self.target.parent_id.and_then(|id| parents.get(&id)) {
            line.push_str(&format!("; parent: {parent}"));
        }
        if !self.target.tags.is_empty() {
            line.push_str(&format!("; tags: {}", format_tags(&self.target.tags)));
        }
        line
    }

    /// Ce qui ne va pas, quand quelque chose ne va pas.
    fn detail(&self) -> Option<&str> {
        self.last_error.as_deref().or(self.probe_reason.as_deref())
    }
}

/// Même déduction que l'interface (`targetState`) : désactivé, erreur, jamais
/// interrogé, ou interrogation trop ancienne — plus de trois périodes, et jamais
/// moins de 90 secondes de tolérance.
fn classic_state(
    target: &Target,
    last_probe_at: Option<&str>,
    last_error: Option<&str>,
    now: DateTime<Utc>,
) -> DeviceState {
    if !target.enabled {
        return DeviceState::Disabled;
    }
    if last_error.is_some() {
        return DeviceState::Unreachable;
    }
    let Some(last) = last_probe_at.and_then(|raw| DateTime::parse_from_rfc3339(raw).ok()) else {
        return DeviceState::Waiting;
    };
    let tolerance = TimeDelta::seconds((target.interval.as_secs() * 3).max(90) as i64);
    if now - last.with_timezone(&Utc) > tolerance {
        DeviceState::Unreachable
    } else {
        DeviceState::Reporting
    }
}

/// Dernier résultat des sondes de service, par identifiant de cible.
///
/// Sans VictoriaMetrics, on retombe sur la déduction classique plutôt que
/// d'échouer : l'assistant a une réponse, un peu moins précise.
async fn probe_results(state: &AppState) -> HashMap<TargetId, (bool, Option<String>)> {
    let mut out = HashMap::new();
    let Ok(series) = state
        .victoria
        .query(&format!("last_over_time(dumbmonit_probe_success[{PROBE_WINDOW}])"))
        .await
    else {
        return out;
    };
    for serie in series {
        let Some(id) = serie.metric.get("target").and_then(|raw| raw.parse::<TargetId>().ok())
        else {
            continue;
        };
        let up = serie.value.1.parse::<f64>().map(|v| v >= 1.0).unwrap_or(false);
        out.insert(id, (up, None));
    }
    if out.values().any(|(up, _)| !*up)
        && let Ok(series) = state
            .victoria
            .query(&format!("tlast_over_time(dumbmonit_probe_failure_info[{PROBE_WINDOW}])"))
            .await
    {
        for serie in series {
            let Some(id) = serie.metric.get("target").and_then(|raw| raw.parse::<TargetId>().ok())
            else {
                continue;
            };
            if let Some((false, reason)) = out.get_mut(&id) {
                *reason = serie.metric.get("reason").cloned();
            }
        }
    }
    out
}

async fn load_devices(state: &AppState) -> Result<Vec<Device>, ToolError> {
    let targets = db::targets::list(&state.pool, &state.cipher).await?;
    let mut statuses = db::targets::statuses(&state.pool).await?;
    let has_services = targets.iter().any(|t| UPTIME_KINDS.contains(&t.kind.as_str()));
    let probes = if has_services { probe_results(state).await } else { HashMap::new() };
    let now = Utc::now();

    Ok(targets
        .into_iter()
        .map(|target| {
            let status = statuses.remove(&target.id);
            let last_probe_at = status.as_ref().and_then(|s| s.last_probe_at.clone());
            let last_error = status.as_ref().and_then(|s| s.last_error.clone());
            let classic =
                classic_state(&target, last_probe_at.as_deref(), last_error.as_deref(), now);
            let mut probe_reason = None;
            let state = if UPTIME_KINDS.contains(&target.kind.as_str()) && target.enabled {
                // Pour un service, `last_error` ne parle que de configuration ; la
                // vérité est dans le résultat de la sonde.
                match (last_error.is_some(), probes.get(&target.id)) {
                    (true, _) => DeviceState::Unreachable,
                    (false, Some((true, _))) => DeviceState::Reporting,
                    (false, Some((false, reason))) => {
                        probe_reason = reason.clone();
                        DeviceState::Down
                    }
                    // Un heartbeat sans verdict n'a pas encore été appelé : il attend.
                    (false, None) if classic == DeviceState::Waiting || target.kind == "push" => {
                        DeviceState::Waiting
                    }
                    (false, None) => DeviceState::Unknown,
                }
            } else {
                classic
            };
            Device { target, state, last_probe_at, last_error, probe_reason }
        })
        .collect())
}

fn parent_names(devices: &[Device]) -> HashMap<TargetId, String> {
    devices.iter().map(|d| (d.id(), d.name().to_string())).collect()
}

/// Retrouve un équipement par identifiant ou par nom.
///
/// Le nom est comparé sans la casse, d'abord exactement, puis comme sous-chaîne
/// unique : « the nas » doit suffire quand un seul équipement s'appelle « NAS ».
fn resolve<'a>(devices: &'a [Device], reference: &str) -> Result<&'a Device, ToolError> {
    if let Ok(id) = reference.parse::<TargetId>()
        && let Some(device) = devices.iter().find(|d| d.id() == id)
    {
        return Ok(device);
    }
    let wanted = reference.trim().to_lowercase();
    if let Some(device) = devices.iter().find(|d| d.name().to_lowercase() == wanted) {
        return Ok(device);
    }
    let partial: Vec<&Device> =
        devices.iter().filter(|d| d.name().to_lowercase().contains(&wanted)).collect();
    match partial.as_slice() {
        [device] => Ok(device),
        [] => Err(failed(format!(
            "No device matches \"{reference}\". Known devices: {}.",
            names(devices)
        ))),
        many => Err(failed(format!(
            "\"{reference}\" is ambiguous: {}. Use the id.",
            many.iter()
                .map(|d| format!("{} (#{})", d.name(), d.id()))
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

fn names(devices: &[Device]) -> String {
    if devices.is_empty() {
        return "none yet".to_string();
    }
    devices.iter().map(|d| format!("{} (#{})", d.name(), d.id())).collect::<Vec<_>>().join(", ")
}

fn resolve_arg<'a>(devices: &'a [Device], args: &Args) -> Result<&'a Device, ToolError> {
    let reference =
        args.device_ref().ok_or_else(|| failed("Name the device: \"id\" or \"name\"."))?;
    resolve(devices, &reference)
}

fn format_tags(tags: &BTreeMap<String, String>) -> String {
    tags.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(", ")
}

// --------------------------------------------------------------------------
// Alertes
// --------------------------------------------------------------------------

/// Le mot de l'interface pour une sévérité interne.
fn severity_word(severity: Severity) -> &'static str {
    match severity {
        Severity::Info => "info",
        Severity::Warning => "advisory",
        Severity::Critical => "warning",
    }
}

/// Le mot sur la plaque : sévérité, ou surcouche (suppressed, building up).
fn alert_word(alert: &ActiveAlertView) -> &'static str {
    use crate::alerting::machine::EffectivePhase;
    match alert.effective_phase {
        EffectivePhase::Suppressed => "suppressed by parent",
        EffectivePhase::Pending => "building up",
        _ if alert.learning => "learning",
        _ => severity_word(alert.severity),
    }
}

async fn active_alerts(state: &AppState) -> Result<Vec<ActiveAlertView>, ToolError> {
    let Json(alerts) = alerts::list_active(State(state.clone())).await?;
    Ok(alerts)
}

fn alert_json(alert: &ActiveAlertView, devices: &HashMap<TargetId, String>) -> Value {
    json!({
        "fingerprint": alert.fingerprint,
        "rule_uid": alert.rule_uid,
        "rule": alert.rule_name,
        "severity": severity_word(alert.severity),
        "state": alert_word(alert),
        "device_id": alert.target_id,
        "device": alert.target_id.and_then(|id| devices.get(&id)),
        "suppressed_by": alert.suppressed_by.and_then(|id| devices.get(&id)),
        "silenced": alert.silenced,
        "acknowledged": alert.acked,
        "acked_by": alert.acked.then_some(alert.acked_by.as_deref()).flatten(),
        "acked_until": alert.acked.then_some(alert.acked_until.map(rfc3339)).flatten(),
        "ack_note": alert.acked.then_some(alert.ack_note.as_deref()).flatten(),
        "value": alert.value,
        "since": alert.firing_since.or(alert.condition_since).map(rfc3339),
        "labels": alert.labels,
    })
}

fn alert_line(alert: &ActiveAlertView, devices: &HashMap<TargetId, String>) -> String {
    let device = alert
        .target_id
        .and_then(|id| devices.get(&id).cloned())
        .unwrap_or_else(|| "(no device)".to_string());
    let mut line = format!("- [{}] {} — {}", alert_word(alert), alert.rule_name, device);
    if let Some(since) = alert.firing_since.or(alert.condition_since) {
        line.push_str(&format!(", since {}", rfc3339(since)));
    }
    if let Some(value) = alert.value {
        line.push_str(&format!(", value {}", trim_float(value)));
    }
    let detail: Vec<String> = alert
        .labels
        .iter()
        .filter(|(k, _)| {
            !["target", "target_id", "host", "hostname", "instance", "__name__"]
                .contains(&k.as_str())
        })
        .map(|(k, v)| format!("{k}={v}"))
        .collect();
    if !detail.is_empty() {
        line.push_str(&format!(" ({})", detail.join(", ")));
    }
    if let Some(parent) = alert.suppressed_by.and_then(|id| devices.get(&id)) {
        line.push_str(&format!("; suppressed by {parent}"));
    }
    if alert.silenced {
        line.push_str("; silenced");
    }
    if alert.acked {
        line.push_str("; acknowledged");
        if let Some(who) = &alert.acked_by {
            line.push_str(&format!(" by {who}"));
        }
        if let Some(until) = alert.acked_until {
            line.push_str(&format!(" until {}", rfc3339(until)));
        }
        if let Some(note) = &alert.ack_note {
            line.push_str(&format!(" ({note})"));
        }
    }
    line
}

fn rfc3339(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn trim_float(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        format!("{value:.2}")
    }
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

// --------------------------------------------------------------------------
// Outils de lecture
// --------------------------------------------------------------------------

async fn get_status(state: &AppState) -> ToolResult {
    use crate::alerting::machine::EffectivePhase;

    let devices = load_devices(state).await?;
    let alerts = active_alerts(state).await?;
    let parents = parent_names(&devices);

    let count = |wanted: DeviceState| devices.iter().filter(|d| d.state == wanted).count();
    let reporting = count(DeviceState::Reporting);
    let unreachable = count(DeviceState::Unreachable) + count(DeviceState::Down);
    let waiting = count(DeviceState::Waiting);
    let disabled = count(DeviceState::Disabled);

    let firing: Vec<&ActiveAlertView> =
        alerts.iter().filter(|a| a.effective_phase == EffectivePhase::Firing).collect();
    let pending = alerts.iter().filter(|a| a.effective_phase == EffectivePhase::Pending).count();
    let suppressed =
        alerts.iter().filter(|a| a.effective_phase == EffectivePhase::Suppressed).count();
    let warnings = firing.iter().filter(|a| a.severity == Severity::Critical).count();
    let advisories = firing.iter().filter(|a| a.severity == Severity::Warning).count();
    let notices = firing.iter().filter(|a| a.severity == Severity::Info).count();

    // Un équipement injoignable dont la panne est déjà portée par une alerte
    // (règle « device unreachable ») compte une fois, pas deux — comme dans
    // l'interface.
    let voiced: std::collections::HashSet<TargetId> =
        firing.iter().filter_map(|a| a.target_id).collect();
    let silent_outages: Vec<&Device> =
        devices.iter().filter(|d| d.state.is_trouble() && !voiced.contains(&d.id())).collect();

    let sentence = if devices.is_empty() {
        "Nothing to watch yet.".to_string()
    } else {
        let mut parts = Vec::new();
        if warnings > 0 {
            parts.push(plural(warnings, "warning", "warnings"));
        }
        if advisories > 0 {
            parts.push(plural(advisories, "advisory", "advisories"));
        }
        if notices > 0 {
            parts.push(plural(notices, "notice", "notices"));
        }
        if !silent_outages.is_empty() {
            parts.push(format!("{} unreachable", silent_outages.len()));
        }
        if pending > 0 {
            parts.push(format!("{pending} building up"));
        }
        if !parts.is_empty() {
            format!("{}.", parts.join(", "))
        } else if reporting == 0 && waiting > 0 {
            "Waiting for the first reports.".to_string()
        } else {
            "Clear skies.".to_string()
        }
    };

    let mut text = format!(
        "**{sentence}**\n\nDevices: {} total — {reporting} reporting, {unreachable} unreachable, \
         {waiting} waiting, {disabled} disabled.\nAlerts: {warnings} warning(s), {advisories} \
         advisory(ies), {notices} notice(s) firing; {pending} building up; {suppressed} \
         suppressed by parent.",
        devices.len()
    );
    if !silent_outages.is_empty() || !firing.is_empty() {
        text.push_str("\n\nNeeds you:");
        for device in &silent_outages {
            text.push('\n');
            text.push_str(&device.line(&parents));
        }
        for alert in &firing {
            text.push('\n');
            text.push_str(&alert_line(alert, &parents));
        }
    }

    Ok(ToolOutput::new(
        text,
        json!({
            "sentence": sentence,
            "devices": { "total": devices.len(), "reporting": reporting, "unreachable": unreachable, "waiting": waiting, "disabled": disabled },
            "alerts": { "warnings": warnings, "advisories": advisories, "notices": notices, "building_up": pending, "suppressed": suppressed },
            "unreachable_devices": silent_outages.iter().map(|d| d.summary(&parents)).collect::<Vec<_>>(),
            "firing": firing.iter().map(|a| alert_json(a, &parents)).collect::<Vec<_>>(),
        }),
    ))
}

async fn list_devices(state: &AppState, args: &Args) -> ToolResult {
    let devices = load_devices(state).await?;
    let parents = parent_names(&devices);
    let filter = args.str("filter").map(str::to_lowercase);
    let wanted = match args.str("state") {
        None => None,
        Some(raw) => Some(DeviceState::parse(raw).ok_or_else(|| {
            failed(format!(
                "Unknown state \"{raw}\" (expected: reporting, unreachable, waiting, disabled, down, unknown)."
            ))
        })?),
    };

    let selected: Vec<&Device> = devices
        .iter()
        .filter(|d| wanted.is_none_or(|w| d.state == w))
        .filter(|d| {
            filter.as_deref().is_none_or(|needle| {
                d.name().to_lowercase().contains(needle)
                    || d.target.address.to_lowercase().contains(needle)
                    || d.target.kind.to_lowercase().contains(needle)
                    || format_tags(&d.target.tags).to_lowercase().contains(needle)
            })
        })
        .collect();

    let text = if selected.is_empty() {
        if devices.is_empty() {
            "No device is monitored yet.".to_string()
        } else {
            format!("No device matches. Known devices: {}.", names(&devices))
        }
    } else {
        let mut lines = vec![format!("{} of {} devices:", selected.len(), devices.len())];
        lines.extend(selected.iter().map(|d| d.line(&parents)));
        lines.join("\n")
    };

    Ok(ToolOutput::new(
        text,
        json!({ "devices": selected.iter().map(|d| d.summary(&parents)).collect::<Vec<_>>() }),
    ))
}

/// Une valeur instantanée lue dans VictoriaMetrics, ou rien.
async fn instant_value(state: &AppState, query: &str) -> Option<f64> {
    let series = state.victoria.query(query).await.ok()?;
    series
        .iter()
        .filter_map(|s| s.value.1.parse::<f64>().ok())
        .fold(None, |acc, v| Some(acc.map_or(v, |a: f64| a.max(v))))
}

/// Résumé 24 h des métriques clés d'un équipement.
///
/// Chaque collecteur nomme ses séries : les unions `or` couvrent SNMP
/// (HOST-RESOURCES), l'agent, Proxmox, Synology et PBS. Ce qui n'existe pas est
/// simplement absent du résumé.
async fn metrics_summary(state: &AppState, device: &Device) -> Vec<(String, f64, f64)> {
    let sel = format!("target=\"{}\"", device.id());
    let expressions: Vec<(&str, String)> = if UPTIME_KINDS.contains(&device.target.kind.as_str()) {
        vec![
            ("availability_percent", format!("dumbmonit_probe_success{{{sel}}} * 100")),
            ("latency_seconds", format!("dumbmonit_probe_duration_seconds{{{sel}}}")),
        ]
    } else {
        vec![
            (
                "cpu_percent",
                format!(
                    "max by (target) (dumbmonit_cpu_load_percent{{{sel}}} or \
                     dumbmonit_cpu_usage_percent{{{sel}}} or \
                     dumbmonit_proxmox_node_cpu_percent{{{sel}}})"
                ),
            ),
            (
                "memory_percent",
                format!(
                    "max by (target) (dumbmonit_memory_used_percent{{{sel}}} or \
                     dumbmonit_synology_memory_usage_percent{{{sel}}} or \
                     dumbmonit_pbs_node_memory_used_percent{{{sel}}} or \
                     100 * dumbmonit_memory_bytes_used{{{sel}}} / dumbmonit_memory_bytes_total{{{sel}}} or \
                     100 * dumbmonit_proxmox_node_memory_used_bytes{{{sel}}} / dumbmonit_proxmox_node_memory_total_bytes{{{sel}}})"
                ),
            ),
            (
                "disk_fullest_percent",
                format!(
                    "max by (target) (100 * dumbmonit_storage_bytes_used{{{sel}}} / dumbmonit_storage_bytes_total{{{sel}}} or \
                     dumbmonit_proxmox_storage_used_percent{{{sel}}} or \
                     dumbmonit_proxmox_node_rootfs_percent{{{sel}}} or \
                     dumbmonit_pbs_datastore_used_percent{{{sel}}})"
                ),
            ),
        ]
    };

    let mut out = Vec::new();
    for (name, expr) in expressions {
        let max = instant_value(state, &format!("max_over_time(({expr})[24h])")).await;
        let avg = instant_value(state, &format!("avg_over_time(({expr})[24h])")).await;
        if let (Some(max), Some(avg)) = (max, avg) {
            out.push((name.to_string(), max, avg));
        }
    }
    out
}

async fn get_device(state: &AppState, args: &Args) -> ToolResult {
    let devices = load_devices(state).await?;
    let device = resolve_arg(&devices, args)?;
    let parents = parent_names(&devices);
    let alerts = active_alerts(state).await?;
    let own: Vec<&ActiveAlertView> =
        alerts.iter().filter(|a| a.target_id == Some(device.id())).collect();
    let summary = metrics_summary(state, device).await;

    let mut text = device.line(&parents);
    text.push_str(&format!(
        "\nPolled every {} s; profile: {}; credential: {}.",
        device.target.interval.as_secs(),
        device.target.profile_id.as_deref().unwrap_or("not detected yet"),
        device.target.credential.kind_label()
    ));
    if summary.is_empty() {
        text.push_str("\n\nLast 24 h: no CPU/memory/disk series available for this device.");
    } else {
        text.push_str("\n\nLast 24 h:");
        for (name, max, avg) in &summary {
            text.push_str(&format!(
                "\n- {name}: max {}, average {}",
                trim_float(*max),
                trim_float(*avg)
            ));
        }
    }
    if own.is_empty() {
        text.push_str("\n\nNo active alert on this device.");
    } else {
        text.push_str(&format!("\n\n{}:", plural(own.len(), "active alert", "active alerts")));
        for alert in &own {
            text.push('\n');
            text.push_str(&alert_line(alert, &parents));
        }
    }

    let mut structured = device.summary(&parents);
    structured["profile_id"] = json!(device.target.profile_id);
    structured["metrics_24h"] = summary
        .iter()
        .map(|(name, max, avg)| (name.clone(), json!({ "max": max, "avg": avg })))
        .collect::<serde_json::Map<_, _>>()
        .into();
    structured["alerts"] = own.iter().map(|a| alert_json(a, &parents)).collect::<Vec<_>>().into();
    Ok(ToolOutput::new(text, structured))
}

async fn list_alerts(state: &AppState, args: &Args) -> ToolResult {
    use crate::alerting::machine::EffectivePhase;

    let devices = load_devices(state).await?;
    let parents = parent_names(&devices);
    let only = match args.device_ref() {
        Some(reference) => Some(resolve(&devices, &reference)?.id()),
        None => None,
    };
    let include_pending = args.bool("include_pending").unwrap_or(false);

    let alerts = active_alerts(state).await?;
    let selected: Vec<&ActiveAlertView> = alerts
        .iter()
        .filter(|a| include_pending || a.effective_phase != EffectivePhase::Pending)
        .filter(|a| only.is_none_or(|id| a.target_id == Some(id)))
        .collect();

    let text = if selected.is_empty() {
        match only {
            Some(id) => {
                format!("No active alert on {}.", parents.get(&id).cloned().unwrap_or_default())
            }
            None => "No active alert.".to_string(),
        }
    } else {
        let mut lines =
            vec![format!("{}:", plural(selected.len(), "active alert", "active alerts"))];
        lines.extend(selected.iter().map(|a| alert_line(a, &parents)));
        lines.join("\n")
    };
    Ok(ToolOutput::new(
        text,
        json!({ "alerts": selected.iter().map(|a| alert_json(a, &parents)).collect::<Vec<_>>() }),
    ))
}

async fn alert_history(state: &AppState, args: &Args) -> ToolResult {
    let devices = load_devices(state).await?;
    let parents = parent_names(&devices);
    let only = match args.device_ref() {
        Some(reference) => Some(resolve(&devices, &reference)?.id()),
        None => None,
    };
    let since = match args.str("since") {
        Some(raw) => raw.to_string(),
        None => {
            let hours = args.f64("hours").filter(|h| h.is_finite() && *h > 0.0).unwrap_or(24.0);
            let hours = hours.min(MAX_RANGE_HOURS);
            rfc3339(Utc::now() - TimeDelta::milliseconds((hours * 3_600_000.0) as i64))
        }
    };
    let limit = args.i64("limit").unwrap_or(DEFAULT_HISTORY_LIMIT).clamp(1, MAX_HISTORY_LIMIT);

    // L'historique n'est pas filtrable par équipement côté base : on demande plus
    // large et on filtre ici, borné pour ne pas rapatrier des mois.
    let fetch = if only.is_some() { (limit * 10).min(5_000) } else { limit };
    let Json(entries) = alerts::history(
        State(state.clone()),
        Query(HistoryQuery { since: Some(since.clone()), limit: Some(fetch) }),
    )
    .await?;
    let Json(rules) = alerts::list_rules(State(state.clone())).await?;
    let rule_names: HashMap<&str, &str> =
        rules.iter().map(|r| (r.uid.as_str(), r.name.as_str())).collect();

    let selected: Vec<_> = entries
        .iter()
        .filter(|e| only.is_none_or(|id| e.target_id == Some(id)))
        .take(limit as usize)
        .collect();

    let text = if selected.is_empty() {
        format!("No alert transition since {since}.")
    } else {
        let mut lines = vec![format!(
            "{} since {since} (most recent first):",
            plural(selected.len(), "transition", "transitions")
        )];
        for e in &selected {
            let device = e
                .target_id
                .and_then(|id| parents.get(&id).cloned())
                .unwrap_or_else(|| "(no device)".to_string());
            let mut line = format!(
                "- {} — {} on {}: {} → {} [{}]",
                e.at,
                rule_names.get(e.rule_uid.as_str()).copied().unwrap_or(e.rule_uid.as_str()),
                device,
                e.from_phase.as_str(),
                e.to_phase.as_str(),
                severity_word(e.severity)
            );
            if let Some(value) = e.value {
                line.push_str(&format!(", value {}", trim_float(value)));
            }
            if !e.notified && !e.reason.is_empty() {
                line.push_str(&format!(" (not notified: {})", e.reason));
            }
            lines.push(line);
        }
        lines.join("\n")
    };

    let structured: Vec<Value> = selected
        .iter()
        .map(|e| {
            json!({
                "at": e.at,
                "rule_uid": e.rule_uid,
                "rule": rule_names.get(e.rule_uid.as_str()),
                "device_id": e.target_id,
                "device": e.target_id.and_then(|id| parents.get(&id)),
                "from": e.from_phase.as_str(),
                "to": e.to_phase.as_str(),
                "severity": severity_word(e.severity),
                "value": e.value,
                "notified": e.notified,
                "reason": e.reason,
            })
        })
        .collect();
    Ok(ToolOutput::new(text, json!({ "since": since, "entries": structured })))
}

/// Ne garde qu'un point sur n pour rester sous [`MAX_POINTS`].
fn thin(values: Vec<(f64, String)>) -> Vec<(f64, String)> {
    if values.len() <= MAX_POINTS {
        return values;
    }
    let stride = values.len().div_ceil(MAX_POINTS);
    values.into_iter().step_by(stride).collect()
}

async fn query_metrics(state: &AppState, args: &Args) -> ToolResult {
    let query = args.str("query").ok_or_else(|| failed("\"query\" is required."))?.to_string();
    let range_hours = args.f64("range_hours").filter(|h| h.is_finite() && *h > 0.0).unwrap_or(1.0);
    if range_hours > MAX_RANGE_HOURS {
        return Err(failed(format!("\"range_hours\" is limited to {MAX_RANGE_HOURS} (31 days).")));
    }
    let end = Utc::now().timestamp_millis();
    let start = end - (range_hours * 3_600_000.0) as i64;
    // Le pas est choisi pour tenir en 60 points ; un pas plus fin demandé par
    // l'appelant est élargi silencieusement, comme le fait l'interface.
    let floor = ((end - start) / 1000 / MAX_POINTS as i64).max(1) as u64;
    let step = args.i64("step_secs").filter(|s| *s > 0).map_or(floor, |s| (s as u64).max(floor));

    let Json(series) = metrics::query_range(
        State(state.clone()),
        Query(RangeQuery { query: query.clone(), start, end, step: Some(step) }),
    )
    .await?;

    if series.is_empty() {
        return Ok(ToolOutput::new(
            format!("No series matched `{query}` over the last {range_hours} h."),
            json!({ "query": query, "series": [] }),
        ));
    }

    let mut lines = vec![format!(
        "{} for `{query}` over the last {range_hours} h (step {step} s):",
        plural(series.len(), "series", "series")
    )];
    let mut structured = Vec::new();
    for serie in series {
        let values = thin(serie.values);
        let numbers: Vec<f64> = values.iter().filter_map(|(_, v)| v.parse().ok()).collect();
        let labels =
            serie.metric.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(", ");
        if numbers.is_empty() {
            lines.push(format!("- {{{labels}}}: no numeric value"));
        } else {
            let min = numbers.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = numbers.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let avg = numbers.iter().sum::<f64>() / numbers.len() as f64;
            let last = numbers[numbers.len() - 1];
            lines.push(format!(
                "- {{{labels}}}: {} points, min {}, max {}, avg {}, last {}",
                numbers.len(),
                trim_float(min),
                trim_float(max),
                trim_float(avg),
                trim_float(last)
            ));
        }
        structured.push(json!({
            "metric": serie.metric,
            "values": values.iter().map(|(ts, v)| json!([ts, v.parse::<f64>().ok()])).collect::<Vec<_>>(),
        }));
    }
    Ok(ToolOutput::new(
        lines.join("\n"),
        json!({ "query": query, "step_secs": step, "series": structured }),
    ))
}

fn schedule_text(schedule: &Schedule) -> String {
    match schedule {
        Schedule::Once { starts_at, ends_at } => {
            format!("once, {} → {}", rfc3339(*starts_at), rfc3339(*ends_at))
        }
        Schedule::Weekly { days, start_minute, end_minute, utc_offset_minutes, timezone } => {
            let days: Vec<&str> =
                days.iter().filter_map(|d| WEEKDAYS.get(*d as usize).copied()).collect();
            format!(
                "weekly on {}, {:02}:{:02} → {:02}:{:02} ({})",
                days.join("/"),
                start_minute / 60,
                start_minute % 60,
                end_minute / 60,
                end_minute % 60,
                zone_text(timezone.as_deref(), *utc_offset_minutes)
            )
        }
        Schedule::Monthly {
            days,
            nth_weekdays,
            start_minute,
            duration_minutes,
            utc_offset_minutes,
            timezone,
        } => {
            let mut when: Vec<String> = days.iter().map(|day| format!("day {day}")).collect();
            for nth in nth_weekdays {
                let name = WEEKDAYS.get(nth.weekday as usize).copied().unwrap_or("?");
                let rank = if nth.nth < 0 { "last".to_string() } else { format!("#{}", nth.nth) };
                when.push(format!("{rank} {name}"));
            }
            format!(
                "monthly on {}, {:02}:{:02} for {} min ({})",
                when.join("/"),
                start_minute / 60,
                start_minute % 60,
                duration_minutes,
                zone_text(timezone.as_deref(), *utc_offset_minutes)
            )
        }
    }
}

const WEEKDAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/// Fuseau d'une fenêtre récurrente, tel qu'on le dit à un assistant.
fn zone_text(timezone: Option<&str>, utc_offset_minutes: i32) -> String {
    match timezone.map(str::trim).filter(|name| !name.is_empty()) {
        Some(name) => name.to_string(),
        None => format!("UTC{:+}", utc_offset_minutes / 60),
    }
}

async fn list_silences(state: &AppState) -> ToolResult {
    let devices = load_devices(state).await?;
    let parents = parent_names(&devices);
    let Json(silences) = alerts::list_silences(State(state.clone())).await?;

    let text = if silences.is_empty() {
        "No maintenance window.".to_string()
    } else {
        let mut lines = vec![format!(
            "{}:",
            plural(silences.len(), "maintenance window", "maintenance windows")
        )];
        for s in &silences {
            let scope = match s.target_id {
                Some(id) => parents.get(&id).cloned().unwrap_or_else(|| format!("device #{id}")),
                None if s.matchers.is_empty() => "whole instance".to_string(),
                None => format!("labels {}", format_tags(&s.matchers)),
            };
            lines.push(format!(
                "- #{} {} — {}; {}; {}{}{}",
                s.id,
                s.name,
                scope,
                schedule_text(&s.schedule),
                if s.active_now {
                    "active now"
                } else if s.enabled {
                    "not active now"
                } else {
                    "disabled"
                },
                if s.comment.is_empty() { String::new() } else { format!("; \"{}\"", s.comment) },
                ""
            ));
        }
        lines.join("\n")
    };
    let structured: Vec<Value> = silences
        .iter()
        .map(|s| {
            json!({
                "id": s.id,
                "name": s.name,
                "comment": s.comment,
                "device_id": s.target_id,
                "device": s.target_id.and_then(|id| parents.get(&id)),
                "matchers": s.matchers,
                "schedule": s.schedule,
                "enabled": s.enabled,
                "active_now": s.active_now,
            })
        })
        .collect();
    Ok(ToolOutput::new(text, json!({ "silences": structured })))
}

// --------------------------------------------------------------------------
// Outils d'écriture
// --------------------------------------------------------------------------

async fn silence_device(state: &AppState, args: &Args) -> ToolResult {
    let devices = load_devices(state).await?;
    let device = resolve_arg(&devices, args)?;
    let hours = args.f64("hours").unwrap_or(1.0);
    if !hours.is_finite() || hours <= 0.0 {
        return Err(failed("\"hours\" must be a positive number."));
    }
    if hours > MAX_SILENCE_HOURS {
        return Err(failed(format!(
            "\"hours\" is limited to {MAX_SILENCE_HOURS} (one week). To silence a device for \
             longer, disable it or its rules instead."
        )));
    }
    let comment = args.str("comment").unwrap_or("Silenced via the assistant").to_string();
    let starts_at = Utc::now();
    let ends_at = starts_at + TimeDelta::milliseconds((hours * 3_600_000.0) as i64);

    let (_, Json(silence)) = alerts::create_silence(
        State(state.clone()),
        Json(SilencePayload {
            name: format!("{} — silenced by assistant", device.name()),
            comment: Some(comment),
            target_id: Some(device.id()),
            matchers: BTreeMap::new(),
            schedule: json!({ "kind": "once", "starts_at": rfc3339(starts_at), "ends_at": rfc3339(ends_at) }),
            enabled: Some(true),
        }),
    )
    .await?;

    Ok(ToolOutput::new(
        format!(
            "Silence #{} created: alerts on {} (#{}) are muted until {} ({} h). Remove it \
             earlier with remove_silence.",
            silence.id,
            device.name(),
            device.id(),
            rfc3339(ends_at),
            trim_float(hours)
        ),
        json!({ "id": silence.id, "device_id": device.id(), "device": device.name(), "starts_at": rfc3339(starts_at), "ends_at": rfc3339(ends_at) }),
    ))
}

async fn remove_silence(state: &AppState, args: &Args) -> ToolResult {
    let id = args.i64("id").ok_or_else(|| failed("\"id\" (silence id) is required."))?;
    alerts::delete_silence(State(state.clone()), Path(id)).await?;
    Ok(ToolOutput::new(
        format!("Silence #{id} removed. Alerts it covered notify again."),
        json!({ "id": id, "removed": true }),
    ))
}

async fn acknowledge_alert(state: &AppState, args: &Args) -> ToolResult {
    let fingerprint = args
        .str("fingerprint")
        .ok_or_else(|| failed("\"fingerprint\" (see list_alerts) is required."))?
        .to_string();
    let hours = args.f64("hours").unwrap_or(DEFAULT_ACK_HOURS);
    if !hours.is_finite() || hours < 0.0 {
        return Err(failed("\"hours\" must be a number greater than or equal to 0."));
    }
    let payload = if hours == 0.0 {
        AckPayload { until: Some(None), duration_secs: None, note: None }
    } else {
        AckPayload {
            until: None,
            duration_secs: Some((hours * 3600.0).round() as i64),
            note: args.str("note").map(str::to_string),
        }
    };
    let request = payload.validate(Utc::now())?;
    let lifted = request == alerts::AckRequest::Clear;
    let alert = alerts::acknowledge(state, &fingerprint, request, "assistant").await?;

    let devices = load_devices(state).await?;
    let names: HashMap<TargetId, String> =
        devices.iter().map(|device| (device.id(), device.name().to_string())).collect();
    let text = if lifted {
        format!("Acknowledgement lifted on {}: reminders resume.", alert.rule_name)
    } else {
        format!(
            "Acknowledged {} ({}) until {}: no reminder until then; the resolution will still \
             be notified. Lift it earlier with hours 0.",
            alert.rule_name,
            alert
                .target_id
                .and_then(|id| names.get(&id).cloned())
                .unwrap_or_else(|| "no device".into()),
            alert.acked_until.map(rfc3339).unwrap_or_default()
        )
    };
    Ok(ToolOutput::new(text, alert_json(&alert, &names)))
}

async fn probe_device(state: &AppState, args: &Args) -> ToolResult {
    let devices = load_devices(state).await?;
    let device = resolve_arg(&devices, args)?;
    match targets::probe_now(State(state.clone()), Path(device.id())).await {
        Ok(Json(report)) => {
            let shown: Vec<&str> = report.series.iter().take(25).map(String::as_str).collect();
            let more = report.series.len().saturating_sub(shown.len());
            let mut text = format!(
                "{} (#{}) answered: {} samples across {} series.",
                device.name(),
                device.id(),
                report.sample_count,
                report.series.len()
            );
            if !shown.is_empty() {
                text.push_str(&format!("\nSeries: {}", shown.join(", ")));
                if more > 0 {
                    text.push_str(&format!(" … and {more} more"));
                }
            }
            Ok(ToolOutput::new(
                text,
                json!({ "device_id": device.id(), "device": device.name(), "ok": true, "sample_count": report.sample_count, "series": report.series }),
            ))
        }
        Err(ApiError::BadRequest(message)) => {
            Err(failed(format!("Probe of {} (#{}) failed: {message}", device.name(), device.id())))
        }
        Err(other) => Err(other.into()),
    }
}

async fn set_device_enabled(state: &AppState, args: &Args) -> ToolResult {
    let enabled = args.required_bool("enabled")?;
    let devices = load_devices(state).await?;
    let device = resolve_arg(&devices, args)?;
    db::targets::set_enabled(&state.pool, device.id(), enabled).await?;
    Ok(ToolOutput::new(
        format!(
            "{} (#{}) is now {}.",
            device.name(),
            device.id(),
            if enabled {
                "enabled: it will be polled at its next interval"
            } else {
                "disabled: no polling, no alerts"
            }
        ),
        json!({ "device_id": device.id(), "device": device.name(), "enabled": enabled }),
    ))
}

async fn list_rules(state: &AppState) -> ToolResult {
    let Json(rules) = alerts::list_rules(State(state.clone())).await?;
    let text = if rules.is_empty() {
        "No alert rule.".to_string()
    } else {
        let mut lines = vec![format!("{}:", plural(rules.len(), "rule", "rules"))];
        for r in &rules {
            let mut line = format!(
                "- `{}` (#{}) {} — {}, {}",
                r.uid,
                r.id,
                r.name,
                r.kind.as_str(),
                severity_word(r.severity)
            );
            if r.kind == crate::alerting::model::RuleKind::Threshold {
                line.push_str(&format!(
                    ", {} {}{}",
                    r.operator.as_str(),
                    trim_float(r.threshold),
                    r.unit
                ));
            }
            if r.for_secs > 0 {
                line.push_str(&format!(" for {} s", r.for_secs));
            }
            line.push_str(if r.enabled { "; enabled" } else { "; disabled" });
            if r.builtin {
                line.push_str("; built in");
            }
            lines.push(line);
        }
        lines.join("\n")
    };
    let structured: Vec<Value> = rules
        .iter()
        .map(|r| {
            json!({
                "id": r.id,
                "uid": r.uid,
                "name": r.name,
                "description": r.description,
                "kind": r.kind.as_str(),
                "severity": severity_word(r.severity),
                "query": r.query,
                "operator": r.operator.as_str(),
                "threshold": r.threshold,
                "unit": r.unit,
                "for_secs": r.for_secs,
                "enabled": r.enabled,
                "builtin": r.builtin,
            })
        })
        .collect();
    Ok(ToolOutput::new(text, json!({ "rules": structured })))
}

async fn set_rule_enabled(state: &AppState, args: &Args) -> ToolResult {
    let enabled = args.required_bool("enabled")?;
    let Json(rules) = alerts::list_rules(State(state.clone())).await?;
    let rule = if let Some(id) = args.i64("id") {
        rules.iter().find(|r| r.id == id)
    } else if let Some(uid) = args.str("uid") {
        rules.iter().find(|r| r.uid.eq_ignore_ascii_case(uid))
    } else {
        return Err(failed("Name the rule: \"uid\" or \"id\" (see list_rules)."));
    };
    let Some(rule) = rule else {
        return Err(failed(format!(
            "No such rule. Known rules: {}.",
            rules.iter().map(|r| r.uid.as_str()).collect::<Vec<_>>().join(", ")
        )));
    };
    let Json(updated) = alerts::set_rule_enabled(
        State(state.clone()),
        Path(rule.id),
        Json(EnablePayload { enabled }),
    )
    .await?;
    Ok(ToolOutput::new(
        format!(
            "Rule `{}` ({}) is now {}.",
            updated.uid,
            updated.name,
            if updated.enabled { "enabled" } else { "disabled" }
        ),
        json!({ "id": updated.id, "uid": updated.uid, "name": updated.name, "enabled": updated.enabled }),
    ))
}

// --------------------------------------------------------------------------
// Catalogue des types, agents, heartbeats, pages de statut, canaux, paquets
// --------------------------------------------------------------------------

async fn list_device_types(state: &AppState, args: &Args) -> ToolResult {
    let Json(kinds) = collectors::list(State(state.clone())).await;
    if let Some(wanted) = args.str("kind") {
        let Some(kind) = kinds.iter().find(|k| k.kind.eq_ignore_ascii_case(wanted)) else {
            return Err(failed(format!(
                "Unknown device type \"{wanted}\". Known types: {}.",
                kinds.iter().map(|k| &*k.kind).collect::<Vec<&str>>().join(", ")
            )));
        };
        let mut text = format!("**{}** (`{}`) — {}", kind.label, kind.kind, kind.summary);
        if !kind.address_hint.is_empty() {
            text.push_str(&format!("\nAddress, e.g.: {}", kind.address_hint));
            if kind.default_port > 0 {
                text.push_str(&format!(" (default port {})", kind.default_port));
            }
        }
        if kind.credentials.is_empty() {
            text.push_str("\nCredential: none.");
        } else {
            text.push_str("\nCredential (the `credential` object of add_device):");
            for credential in &kind.credentials {
                let fields: Vec<String> = credential
                    .fields
                    .iter()
                    .map(|f| {
                        let mut field = format!("{}{}", f.key, if f.required { "" } else { "?" });
                        if !f.choices.is_empty() {
                            field.push_str(&format!(
                                " ({})",
                                f.choices
                                    .iter()
                                    .map(AsRef::as_ref)
                                    .collect::<Vec<&str>>()
                                    .join("|")
                            ));
                        }
                        field
                    })
                    .collect();
                text.push_str(&format!(
                    "\n- type `{}` — {}{}",
                    credential.kind,
                    credential.label,
                    if fields.is_empty() {
                        String::new()
                    } else {
                        format!(": {}", fields.join(", "))
                    }
                ));
            }
        }
        if !kind.options.is_empty() {
            text.push_str("\nOptions (the `options` object of add_device):");
            for option in &kind.options {
                let mut line = format!("\n- `{}` — {}", option.key, option.label);
                if option.required {
                    line.push_str(" (required)");
                }
                if !option.default.is_empty() {
                    line.push_str(&format!("; default {}", option.default));
                }
                if !option.choices.is_empty() {
                    line.push_str(&format!(
                        "; one of {}",
                        option.choices.iter().map(AsRef::as_ref).collect::<Vec<&str>>().join(", ")
                    ));
                }
                if !option.help.is_empty() {
                    line.push_str(&format!(". {}", option.help));
                }
                text.push_str(&line);
            }
        }
        if !kind.setup.steps.is_empty() {
            text.push_str(&format!("\n{}:", kind.setup.title));
            for (index, step) in kind.setup.steps.iter().enumerate() {
                text.push_str(&format!("\n{}. {}", index + 1, step.replace('\n', " — ")));
            }
        }
        if !kind.setup.warning.is_empty() {
            text.push_str(&format!("\nNote: {}", kind.setup.warning));
        }
        let described = serde_json::to_value(kind).map_err(anyhow::Error::from)?;
        return Ok(ToolOutput::new(text, json!({ "kinds": [described] })));
    }

    let mut lines = vec![format!("{} device types:", kinds.len())];
    let mut structured = Vec::new();
    for kind in &kinds {
        let credentials: Vec<&str> = kind.credential_types.iter().map(AsRef::as_ref).collect();
        lines.push(format!(
            "- `{}` — {}: {}{}",
            kind.kind,
            kind.label,
            kind.summary,
            if credentials.is_empty() || credentials == ["none"] {
                String::new()
            } else {
                format!(" (credential: {})", credentials.join(" or "))
            }
        ));
        structured.push(json!({
            "kind": kind.kind,
            "label": kind.label,
            "summary": kind.summary,
            "credential_types": credentials,
            "options": kind.options.iter().map(|o| o.key.as_ref()).collect::<Vec<&str>>(),
        }));
    }
    lines
        .push("Call list_device_types with a kind for its address, credential and options.".into());
    Ok(ToolOutput::new(lines.join("\n"), json!({ "kinds": structured })))
}

async fn list_agents(state: &AppState) -> ToolResult {
    let hosts = crate::collectors::agent::list_hosts(&state.pool).await?;
    let counts = db::targets::relayed_counts(&state.pool).await?;
    if hosts.is_empty() {
        return Ok(ToolOutput::new(
            "No agent has reported yet. Install one from Settings → Agents.".into(),
            json!({ "agents": [] }),
        ));
    }
    let mut lines = vec![format!("{}:", plural(hosts.len(), "agent", "agents"))];
    let mut structured = Vec::new();
    for (id, name, info) in hosts {
        let relayed = counts.get(&id).copied().unwrap_or(0);
        let mut line = format!(
            "- **{name}** (#{id}) — {} on {}{}, agent {}",
            info.hostname,
            info.os,
            info.os_version.as_deref().map(|v| format!(" {v}")).unwrap_or_default(),
            info.agent_version
        );
        if info.relay {
            line.push_str(&format!(
                "; relay{} for {}",
                info.site.as_deref().map(|s| format!(" of site {s}")).unwrap_or_default(),
                plural(relayed, "device", "devices")
            ));
        }
        line.push_str(&format!(
            "; last seen {}; {}{}",
            info.last_seen_at.as_deref().unwrap_or("never"),
            info.binding_state(),
            if info.commands_supported() { "" } else { "; no remote commands" }
        ));
        lines.push(line);
        structured.push(json!({
            "id": id,
            "name": name,
            "hostname": &info.hostname,
            "os": &info.os,
            "os_version": &info.os_version,
            "arch": &info.arch,
            "agent_version": &info.agent_version,
            "relay": info.relay,
            "site": &info.site,
            "relayed": relayed,
            "last_seen_at": &info.last_seen_at,
            "binding": info.binding_state(),
            "commands_supported": info.commands_supported(),
        }));
    }
    Ok(ToolOutput::new(lines.join("\n"), json!({ "agents": structured })))
}

/// Un équipement de type `agent`, ou un échec qui dit lesquels le sont.
fn resolve_agent<'a>(devices: &'a [Device], args: &Args) -> Result<&'a Device, ToolError> {
    let device = resolve_arg(devices, args)?;
    if device.target.kind != "agent" {
        let agents: Vec<String> = devices
            .iter()
            .filter(|d| d.target.kind == "agent")
            .map(|d| format!("{} (#{})", d.name(), d.id()))
            .collect();
        return Err(failed(format!(
            "{} is a \"{}\" device, not a machine running the agent. Agents: {}.",
            device.name(),
            device.target.kind,
            if agents.is_empty() { "none yet".to_string() } else { agents.join(", ") }
        )));
    }
    Ok(device)
}

async fn list_containers(state: &AppState, args: &Args) -> ToolResult {
    let devices = load_devices(state).await?;
    let device = resolve_agent(&devices, args)?;
    let Json(containers) =
        agent_commands::list_containers(State(state.clone()), Path(device.id())).await?;
    let text = if containers.is_empty() {
        format!("No container reported by {} (#{}).", device.name(), device.id())
    } else {
        let mut lines = vec![format!(
            "{} on {} (#{}):",
            plural(containers.len(), "container", "containers"),
            device.name(),
            device.id()
        )];
        for c in &containers {
            let mut line = format!(
                "- **{}** ({}) — {}, health {}, {}",
                c.name,
                c.image,
                if c.up { "running" } else { "stopped" },
                c.health,
                plural(c.restart_count as usize, "restart", "restarts")
            );
            if c.update_available == Some(true) {
                line.push_str("; update available");
            }
            if let Some(command) = &c.last_command {
                line.push_str(&format!("; last command {} {}", command.kind, command.status));
            }
            lines.push(line);
        }
        lines.join("\n")
    };
    let containers = serde_json::to_value(&containers).map_err(anyhow::Error::from)?;
    Ok(ToolOutput::new(
        text,
        json!({ "device_id": device.id(), "device": device.name(), "containers": containers }),
    ))
}

fn verdict_word(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Waiting => "waiting",
        Verdict::OnTime => "on_time",
        Verdict::Missed => "missed",
        Verdict::ReportedDown => "reported_down",
    }
}

async fn list_heartbeats(state: &AppState) -> ToolResult {
    let devices = load_devices(state).await?;
    let now_ms = Utc::now().timestamp_millis();
    let mut lines = Vec::new();
    let mut structured = Vec::new();
    for device in devices.iter().filter(|d| d.target.kind == push::KIND) {
        // Lecture seule : le jeton n'est ni créé ni déchiffré pour être montré —
        // il ne quitte jamais ce module.
        let monitor = push::store::get(&state.pool, &state.cipher, device.id()).await?;
        let (settings, settings_error) = match push::Settings::from_target(&device.target) {
            Ok(settings) => (Some(settings), None),
            Err(error) => (None, Some(error.to_string())),
        };
        let last_seen_ms = monitor.as_ref().and_then(|m| m.last_seen_ms);
        let verdict = match (&monitor, settings) {
            (Some(m), Some(settings)) => {
                push::evaluate(m.last_seen_ms, m.last_status, now_ms, settings)
            }
            (Some(m), None) if m.last_seen_ms.is_some() => Verdict::OnTime,
            _ => Verdict::Waiting,
        };
        let age = last_seen_ms.map(|then| push::age(then, now_ms).as_secs());
        let mut line = format!(
            "- **{}** (#{}) — {}",
            device.name(),
            device.id(),
            if device.target.enabled { verdict_word(verdict) } else { "disabled" }
        );
        if let Some(settings) = settings {
            line.push_str(&format!(
                "; expected every {} s (+{} s grace)",
                settings.expected.as_secs(),
                settings.grace.as_secs()
            ));
        }
        match (&monitor, age) {
            (Some(m), Some(age)) => {
                line.push_str(&format!(
                    "; last call {age} s ago ({}){}",
                    m.last_status.as_str(),
                    if m.last_message.is_empty() {
                        String::new()
                    } else {
                        format!(": \"{}\"", m.last_message)
                    }
                ));
            }
            _ => line.push_str("; never called yet"),
        }
        if let Some(error) = &settings_error {
            line.push_str(&format!("; settings error: {error}"));
        }
        lines.push(line);
        structured.push(json!({
            "device_id": device.id(),
            "device": device.name(),
            "enabled": device.target.enabled,
            "verdict": verdict_word(verdict),
            "expected_interval_secs": settings.map(|s| s.expected.as_secs()),
            "grace_secs": settings.map(|s| s.grace.as_secs()),
            "last_seen_at": monitor.as_ref().and_then(|m| m.last_seen_at.clone()),
            "last_seen_age_secs": age,
            "last_status": monitor.as_ref().map(|m| m.last_status.as_str()),
            "last_message": monitor.as_ref().map(|m| m.last_message.clone()),
            "received_total": monitor.as_ref().map_or(0, |m| m.received_total),
            "settings_error": settings_error,
        }));
    }
    let text = if lines.is_empty() {
        "No heartbeat monitor. Add a device of kind push to watch a cron job or a backup \
         script."
            .to_string()
    } else {
        format!("{}:\n{}", plural(lines.len(), "heartbeat", "heartbeats"), lines.join("\n"))
    };
    Ok(ToolOutput::new(text, json!({ "heartbeats": structured })))
}

fn is_closed(status: &str) -> bool {
    matches!(status, "resolved" | "completed")
}

/// Dernier message du fil d'un incident.
fn latest_update(view: &status_pages::IncidentView) -> Option<String> {
    view.updates.iter().max_by_key(|u| (u.created_at.clone(), u.id)).map(|u| u.body.clone())
}

async fn list_status_pages(state: &AppState) -> ToolResult {
    let Json(pages) = status_pages::list_pages(State(state.clone())).await?;
    let Json(incidents) = status_pages::list_incidents(State(state.clone())).await?;

    let mut lines = Vec::new();
    if pages.is_empty() {
        lines.push("No status page.".to_string());
    } else {
        lines.push(format!("{}:", plural(pages.len(), "status page", "status pages")));
        for view in &pages {
            lines.push(format!(
                "- **{}** (#{}, /s/{}) — {}, {}",
                view.page.title,
                view.page.id,
                view.page.slug,
                if view.page.published { "published" } else { "draft" },
                plural(view.items.len(), "device", "devices")
            ));
        }
    }

    // Les incidents ouverts d'abord, puis les dix plus récents des autres.
    let mut ordered: Vec<&status_pages::IncidentView> = incidents.iter().collect();
    ordered.sort_by(|a, b| {
        is_closed(&a.incident.status)
            .cmp(&is_closed(&b.incident.status))
            .then_with(|| b.incident.updated_at.cmp(&a.incident.updated_at))
    });
    let open = ordered.iter().filter(|v| !is_closed(&v.incident.status)).count();
    let shown: Vec<&status_pages::IncidentView> = ordered.into_iter().take(open + 10).collect();
    if shown.is_empty() {
        lines.push("No incident.".to_string());
    } else {
        lines.push(format!("Incidents ({open} open):"));
        for view in &shown {
            let i = &view.incident;
            let mut line = format!(
                "- #{} [{}] {} — {} ({}), since {}",
                i.id, i.kind, i.title, i.status, i.severity, i.starts_at
            );
            if let Some(page) = i.page_id.and_then(|id| pages.iter().find(|p| p.page.id == id)) {
                line.push_str(&format!(" on {}", page.page.title));
            }
            if let Some(update) = latest_update(view) {
                line.push_str(&format!("; latest: \"{update}\""));
            }
            lines.push(line);
        }
    }

    Ok(ToolOutput::new(
        lines.join("\n"),
        json!({
            "pages": pages.iter().map(|v| json!({
                "id": v.page.id,
                "title": v.page.title,
                "slug": v.page.slug,
                "path": format!("/s/{}", v.page.slug),
                "published": v.page.published,
                "devices": v.items.len(),
            })).collect::<Vec<_>>(),
            "incidents": shown.iter().map(|v| json!({
                "id": v.incident.id,
                "page_id": v.incident.page_id,
                "title": v.incident.title,
                "kind": v.incident.kind,
                "status": v.incident.status,
                "severity": v.incident.severity,
                "open": !is_closed(&v.incident.status),
                "starts_at": v.incident.starts_at,
                "ends_at": v.incident.ends_at,
                "latest_update": latest_update(v),
            })).collect::<Vec<_>>(),
        }),
    ))
}

async fn list_channels(state: &AppState) -> ToolResult {
    let Json(channels) = channels::list(State(state.clone())).await?;
    let text = if channels.is_empty() {
        "No notification channel: alerts are not sent anywhere yet.".to_string()
    } else {
        let mut lines = vec![format!(
            "{}:",
            plural(channels.len(), "notification channel", "notification channels")
        )];
        for c in &channels {
            let mut line = format!(
                "- **{}** (#{}, {}) — {}",
                c.name,
                c.id,
                c.kind,
                if c.enabled { "enabled" } else { "disabled" }
            );
            if let Some(at) = &c.last_sent_at {
                line.push_str(&format!("; last sent {at}"));
            }
            if let Some(error) = &c.last_error {
                line.push_str(&format!("; last error: {error}"));
            }
            lines.push(line);
        }
        lines.join("\n")
    };
    // Les réglages restent au serveur : certains types y rangent des adresses
    // ou des identifiants que l'assistant n'a pas à connaître.
    let structured: Vec<Value> = channels
        .iter()
        .map(|c| {
            json!({
                "id": c.id,
                "name": c.name,
                "kind": c.kind,
                "enabled": c.enabled,
                "has_secret": c.has_secret,
                "last_error": c.last_error,
                "last_sent_at": c.last_sent_at,
            })
        })
        .collect();
    Ok(ToolOutput::new(text, json!({ "channels": structured })))
}

async fn list_packs(state: &AppState, token: &ApiToken) -> ToolResult {
    let principal = Extension(CurrentPrincipal(Principal::Token(token.clone())));
    let Json(packs) = packs::list(State(state.clone()), principal).await?;
    let text = if packs.is_empty() {
        "No integration pack installed.".to_string()
    } else {
        let mut lines =
            vec![format!("{}:", plural(packs.len(), "integration pack", "integration packs"))];
        for p in &packs {
            let mut line = format!(
                "- **{}** (`{}` {}) — {}, {}",
                p.label,
                p.id,
                p.version,
                if p.enabled { "enabled" } else { "disabled" },
                plural(p.targets.max(0) as usize, "device", "devices")
            );
            if let Some(kind) = &p.kind {
                line.push_str(&format!("; device kind `{kind}`"));
            }
            if let Some(error) = &p.error {
                line.push_str(&format!("; error: {error}"));
            }
            lines.push(line);
        }
        lines.join("\n")
    };
    let structured: Vec<Value> = packs
        .iter()
        .map(|p| {
            json!({
                "id": p.id,
                "version": p.version,
                "label": p.label,
                "summary": p.summary,
                "kind": p.kind,
                "enabled": p.enabled,
                "targets": p.targets,
                "rules": p.rules,
                "warnings": p.warnings,
                "error": p.error,
            })
        })
        .collect();
    Ok(ToolOutput::new(text, json!({ "packs": structured })))
}

// --------------------------------------------------------------------------
// Nouveaux outils d'écriture
// --------------------------------------------------------------------------

/// Valeur d'option en texte : un modèle envoie volontiers `161` ou `true`.
fn option_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.trim().to_string()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    }
}

async fn add_device(state: &AppState, args: &Args) -> ToolResult {
    let name = args.str("name").ok_or_else(|| failed("\"name\" is required."))?;
    let kind = args.str("kind").ok_or_else(|| {
        failed("\"kind\" is required: call list_device_types to see the possible values.")
    })?;
    let address = args.str("address").ok_or_else(|| failed("\"address\" is required."))?;

    let devices = load_devices(state).await?;
    let reference = |key: &str| -> Result<Option<TargetId>, ToolError> {
        match args.0.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Number(n)) => n
                .as_i64()
                .map(|id| resolve(&devices, &id.to_string()).map(|d| Some(d.id())))
                .unwrap_or_else(|| Err(failed(format!("\"{key}\" must be a device id or name.")))),
            Some(Value::String(text)) if text.trim().is_empty() => Ok(None),
            Some(Value::String(text)) => resolve(&devices, text).map(|d| Some(d.id())),
            Some(_) => Err(failed(format!("\"{key}\" must be a device id or name."))),
        }
    };
    let parent_id = reference("parent")?;
    let via_agent = reference("via_agent")?;

    let mut tags = BTreeMap::new();
    for key in ["tags", "options"] {
        match args.0.get(key) {
            None | Some(Value::Null) => {}
            Some(Value::Object(map)) => {
                for (k, v) in map {
                    let text = option_text(v).ok_or_else(|| {
                        failed(format!("\"{key}.{k}\" must be a string, a number or a boolean."))
                    })?;
                    tags.insert(k.clone(), text);
                }
            }
            Some(_) => return Err(failed(format!("\"{key}\" must be an object."))),
        }
    }

    let mut payload = json!({ "name": name, "address": address, "kind": kind, "tags": tags });
    if let Some(interval) = args.i64("interval_secs") {
        payload["interval_secs"] = json!(interval.max(0));
    }
    if let Some(parent) = parent_id {
        payload["parent_id"] = json!(parent);
    }
    if let Some(agent) = via_agent {
        payload["via_agent"] = json!(agent);
    }
    if let Some(enabled) = args.bool("enabled") {
        payload["enabled"] = json!(enabled);
    }
    match args.0.get("credential") {
        None | Some(Value::Null) => {}
        Some(credential @ Value::Object(_)) => payload["credential"] = credential.clone(),
        Some(_) => return Err(failed("\"credential\" must be an object with a \"type\".")),
    }
    // La désérialisation est celle de `POST /api/targets` : mêmes règles, mêmes
    // messages. Son texte d'erreur nomme un champ ou un type, jamais une valeur.
    let payload: targets::TargetPayload = serde_json::from_value(payload).map_err(|error| {
        failed(format!(
            "Invalid device: {error}. Call list_device_types with kind \"{kind}\" for the expected \
             shape."
        ))
    })?;

    let (_, Json(view)) = targets::create(State(state.clone()), Json(payload)).await?;
    let mut text = format!(
        "Added **{}** (#{}, {}, {}); credential: {}. It will be polled every {} s",
        view.name, view.id, view.kind, view.address, view.credential_kind, view.interval_secs
    );
    if let Some(agent) = view.via_agent.and_then(|id| devices.iter().find(|d| d.id() == id)) {
        text.push_str(&format!(" through the relay agent {}", agent.name()));
    }
    text.push_str("; call probe_device to check it right now.");
    let structured = serde_json::to_value(&view).map_err(anyhow::Error::from)?;
    Ok(ToolOutput::new(text, structured))
}

async fn discover_network(token: &ApiToken, args: &Args) -> ToolResult {
    let cidr =
        args.str("cidr").ok_or_else(|| failed("\"cidr\" is required (e.g. 192.168.1.0/24)."))?;
    let port = match args.i64("port") {
        None => None,
        Some(port) => {
            Some(u16::try_from(port).map_err(|_| failed("\"port\" must be between 1 and 65535."))?)
        }
    };
    let request = discovery::ScanRequest {
        cidr: cidr.to_string(),
        community: args.str("community").map(str::to_string),
        port,
        timeout_ms: args.i64("timeout_ms").map(|ms| ms.max(0) as u64),
    };
    let identity = AdminIdentity(Principal::Token(token.clone()));
    let Json(report) = discovery::scan(identity, Json(request)).await?;

    let text = if report.devices.is_empty() {
        format!("Scanned {} addresses in {cidr}: no device answered SNMP.", report.scanned)
    } else {
        let mut lines = vec![format!(
            "Scanned {} addresses in {cidr}: {} answered SNMP.",
            report.scanned,
            plural(report.devices.len(), "device", "devices")
        )];
        for d in &report.devices {
            lines.push(format!(
                "- {} — {}{}",
                d.address,
                d.sysname.as_deref().unwrap_or("(no name)"),
                d.suggested_profile
                    .as_deref()
                    .map(|p| format!(" (profile {p})"))
                    .unwrap_or_default()
            ));
        }
        lines.push("Use add_device with kind snmp to monitor one of them.".into());
        lines.join("\n")
    };
    let structured = serde_json::to_value(&report).map_err(anyhow::Error::from)?;
    Ok(ToolOutput::new(text, structured))
}

async fn restart_container(state: &AppState, token: &ApiToken, args: &Args) -> ToolResult {
    let container = args
        .str("container")
        .ok_or_else(|| failed("\"container\" (see list_containers) is required."))?
        .to_string();
    let devices = load_devices(state).await?;
    let device = resolve_agent(&devices, args)?;
    let who = Some(Extension(CurrentPrincipal(Principal::Token(token.clone()))));
    let (_, Json(command)) =
        agent_commands::restart(State(state.clone()), Path((device.id(), container.clone())), who)
            .await?;
    Ok(ToolOutput::new(
        format!(
            "Restart of container {container} on {} (#{}) queued as command #{}. The agent picks \
             it up within seconds; list_containers shows the outcome.",
            device.name(),
            device.id(),
            command.id
        ),
        json!({
            "command_id": command.id,
            "device_id": device.id(),
            "device": device.name(),
            "container": container,
            "status": command.status,
        }),
    ))
}

async fn test_channel(state: &AppState, args: &Args) -> ToolResult {
    let reference = args
        .str("channel")
        .map(str::to_string)
        .or_else(|| args.i64("channel").map(|id| id.to_string()))
        .or_else(|| args.i64("id").map(|id| id.to_string()))
        .ok_or_else(|| failed("Name the channel: \"channel\" (id or name, see list_channels)."))?;
    let Json(channels) = channels::list(State(state.clone())).await?;
    let wanted = reference.to_lowercase();
    let by_id = reference.parse::<i64>().ok().and_then(|id| channels.iter().find(|c| c.id == id));
    let exact = channels.iter().find(|c| c.name.to_lowercase() == wanted);
    let partial: Vec<_> =
        channels.iter().filter(|c| c.name.to_lowercase().contains(&wanted)).collect();
    let channel = match (by_id.or(exact), partial.as_slice()) {
        (Some(c), _) => c,
        (None, [c]) => *c,
        (None, []) => {
            return Err(failed(format!(
                "No channel matches \"{reference}\". Channels: {}.",
                if channels.is_empty() {
                    "none yet".to_string()
                } else {
                    channels
                        .iter()
                        .map(|c| format!("{} (#{})", c.name, c.id))
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            )));
        }
        (None, many) => {
            return Err(failed(format!(
                "\"{reference}\" is ambiguous: {}. Use the id.",
                many.iter()
                    .map(|c| format!("{} (#{})", c.name, c.id))
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
    };
    let Json(report) = channels::test(State(state.clone()), Path(channel.id)).await?;
    let text = if report.ok {
        format!("Test message sent through {} ({}).", channel.name, channel.kind)
    } else {
        format!("The test through {} ({}) failed: {}", channel.name, channel.kind, report.message)
    };
    Ok(ToolOutput::new(
        text,
        json!({ "channel_id": channel.id, "channel": channel.name, "ok": report.ok, "message": report.message }),
    ))
}

async fn post_incident(state: &AppState, args: &Args) -> ToolResult {
    let body = args.str("body").ok_or_else(|| failed("\"body\" (the message) is required."))?;
    let status = args.str("status").map(str::to_string);

    let (created, Json(view)) = if let Some(id) = args.i64("incident_id") {
        let payload = status_pages::UpdatePayload { status, body: body.to_string() };
        let (_, view) =
            status_pages::add_update(State(state.clone()), Path(id), Json(payload)).await?;
        (false, view)
    } else {
        let title = args.str("title").ok_or_else(|| {
            failed("\"title\" is required to open an incident; pass incident_id to update one.")
        })?;
        let page_id = match args.0.get("page") {
            None | Some(Value::Null) => None,
            Some(reference) => {
                let Json(pages) = status_pages::list_pages(State(state.clone())).await?;
                let wanted = match reference {
                    Value::Number(n) => n.to_string(),
                    Value::String(s) => s.trim().to_string(),
                    _ => return Err(failed("\"page\" must be a page id, slug or title.")),
                };
                let found = pages.iter().find(|p| {
                    p.page.id.to_string() == wanted
                        || p.page.slug.eq_ignore_ascii_case(&wanted)
                        || p.page.title.eq_ignore_ascii_case(&wanted)
                });
                match found {
                    Some(page) => Some(page.page.id),
                    None => {
                        return Err(failed(format!(
                            "No status page matches \"{wanted}\". Pages: {}.",
                            pages
                                .iter()
                                .map(|p| format!(
                                    "{} (#{}, {})",
                                    p.page.title, p.page.id, p.page.slug
                                ))
                                .collect::<Vec<_>>()
                                .join(", ")
                        )));
                    }
                }
            }
        };
        let payload = status_pages::IncidentPayload {
            title: title.to_string(),
            kind: args.str("kind").map(str::to_string),
            status,
            severity: args.str("severity").map(str::to_string),
            page_id,
            starts_at: None,
            ends_at: None,
            body: Some(body.to_string()),
        };
        let (_, view) = status_pages::create_incident(State(state.clone()), Json(payload)).await?;
        (true, view)
    };

    let i = &view.incident;
    let text = if created {
        format!(
            "Opened {} #{} \"{}\" ({}, {}) with its first message; subscribers of the page are \
             notified. Post updates with incident_id {}.",
            i.kind, i.id, i.title, i.status, i.severity, i.id
        )
    } else {
        format!("Update posted on {} #{} \"{}\": now {}.", i.kind, i.id, i.title, i.status)
    };
    Ok(ToolOutput::new(
        text,
        json!({
            "id": i.id,
            "title": i.title,
            "kind": i.kind,
            "status": i.status,
            "severity": i.severity,
            "page_id": i.page_id,
            "updates": view.updates.len(),
        }),
    ))
}

async fn schedule_maintenance(state: &AppState, args: &Args) -> ToolResult {
    let parse = |key: &str| -> Result<Option<DateTime<Utc>>, ToolError> {
        match args.str(key) {
            None => Ok(None),
            Some(raw) => DateTime::parse_from_rfc3339(raw)
                .map(|at| Some(at.with_timezone(&Utc)))
                .map_err(|_| {
                    failed(format!(
                        "\"{key}\" must be an RFC 3339 time, e.g. 2026-10-04T22:00:00Z."
                    ))
                }),
        }
    };
    let starts_at = parse("starts_at")?.ok_or_else(|| failed("\"starts_at\" is required."))?;
    let ends_at = match parse("ends_at")? {
        Some(end) => end,
        None => {
            let hours = args.f64("hours").unwrap_or(1.0);
            if !hours.is_finite() || hours <= 0.0 {
                return Err(failed("\"hours\" must be a positive number."));
            }
            starts_at + TimeDelta::milliseconds((hours * 3_600_000.0) as i64)
        }
    };
    if ends_at <= starts_at {
        return Err(failed("The window must end after it starts."));
    }
    if ends_at - starts_at > TimeDelta::milliseconds((MAX_SILENCE_HOURS * 3_600_000.0) as i64) {
        return Err(failed(
            "A maintenance window planned by an assistant lasts 7 days at most. For a recurring \
             window, use the web interface.",
        ));
    }
    if ends_at <= Utc::now() {
        return Err(failed("That window is already over."));
    }

    let devices = load_devices(state).await?;
    let device = match args.device_ref() {
        Some(reference) => Some(resolve(&devices, &reference)?),
        None => None,
    };
    let scope = device.map_or_else(|| "whole instance".to_string(), |d| d.name().to_string());
    let name =
        args.str("name").map(str::to_string).unwrap_or_else(|| format!("Maintenance — {scope}"));

    let (_, Json(silence)) = alerts::create_silence(
        State(state.clone()),
        Json(SilencePayload {
            name: name.clone(),
            comment: Some(args.str("comment").unwrap_or("Planned via the assistant").to_string()),
            target_id: device.map(Device::id),
            matchers: BTreeMap::new(),
            schedule: json!({ "kind": "once", "starts_at": rfc3339(starts_at), "ends_at": rfc3339(ends_at) }),
            enabled: Some(true),
        }),
    )
    .await?;

    Ok(ToolOutput::new(
        format!(
            "Maintenance window #{} \"{name}\" planned for {scope}: {} → {}{}. Remove it with \
             remove_silence.",
            silence.id,
            rfc3339(starts_at),
            rfc3339(ends_at),
            if silence.active_now { " (active now)" } else { "" }
        ),
        json!({
            "id": silence.id,
            "name": name,
            "device_id": device.map(Device::id),
            "device": device.map(|d| d.name().to_string()),
            "starts_at": rfc3339(starts_at),
            "ends_at": rfc3339(ends_at),
            "active_now": silence.active_now,
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn target(interval_secs: u64, enabled: bool) -> Target {
        Target {
            id: 1,
            name: "NAS".into(),
            address: "10.0.0.5".into(),
            kind: "dummy".into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(interval_secs),
            enabled,
            tags: BTreeMap::new(),
            credential: dumbmonit_proto::Credential::None,
            group_name: String::new(),
            position: 0,
        }
    }

    #[test]
    fn the_classic_state_follows_the_interface_rules() {
        let now = Utc::now();
        let fresh = rfc3339(now - TimeDelta::seconds(30));
        let stale = rfc3339(now - TimeDelta::seconds(600));

        assert_eq!(classic_state(&target(60, false), None, None, now), DeviceState::Disabled);
        assert_eq!(
            classic_state(&target(60, true), Some(&fresh), Some("timeout"), now),
            DeviceState::Unreachable
        );
        assert_eq!(classic_state(&target(60, true), None, None, now), DeviceState::Waiting);
        assert_eq!(
            classic_state(&target(60, true), Some(&fresh), None, now),
            DeviceState::Reporting
        );
        assert_eq!(
            classic_state(&target(60, true), Some(&stale), None, now),
            DeviceState::Unreachable
        );
        // La tolérance ne descend jamais sous 90 s, même à 10 s de période.
        let recent = rfc3339(now - TimeDelta::seconds(80));
        assert_eq!(
            classic_state(&target(10, true), Some(&recent), None, now),
            DeviceState::Reporting
        );
    }

    #[test]
    fn thinning_keeps_at_most_sixty_points() {
        let values: Vec<(f64, String)> = (0..1000).map(|i| (i as f64, i.to_string())).collect();
        let thinned = thin(values);
        assert!(thinned.len() <= MAX_POINTS, "{}", thinned.len());
        assert_eq!(thinned[0].0, 0.0);
        let short: Vec<(f64, String)> = (0..10).map(|i| (i as f64, i.to_string())).collect();
        assert_eq!(thin(short).len(), 10);
    }

    #[test]
    fn every_tool_has_a_schema_and_a_scope() {
        let specs = specs();
        assert_eq!(specs.len(), 27);
        let mut names = std::collections::HashSet::new();
        for spec in &specs {
            assert!(names.insert(spec.name), "nom en double : {}", spec.name);
            // Contraintes de nom de la spécification : 1 à 128 caractères sûrs.
            assert!(
                spec.name.len() <= 128
                    && spec.name.chars().all(|c| c.is_ascii_alphanumeric() || "_-.".contains(c)),
                "{}",
                spec.name
            );
            assert_eq!(spec.input_schema["type"], "object", "{}", spec.name);
            assert_eq!(spec.output_schema["type"], "object", "{}", spec.name);
            assert!(!spec.description.is_empty() && !spec.title.is_empty(), "{}", spec.name);
            // Les champs requis d'un schéma de sortie existent dans ses propriétés.
            for required in spec.output_schema["required"].as_array().unwrap() {
                let key = required.as_str().unwrap();
                assert!(
                    spec.output_schema["properties"].get(key).is_some(),
                    "{}: {key}",
                    spec.name
                );
            }
            if spec.scope == Scope::Read {
                assert!(!spec.destructive, "{}", spec.name);
            }
        }
        let writers: Vec<&str> =
            specs.iter().filter(|s| s.scope == Scope::Write).map(|s| s.name).collect();
        assert_eq!(
            writers,
            [
                "silence_device",
                "schedule_maintenance",
                "remove_silence",
                "acknowledge_alert",
                "probe_device",
                "set_device_enabled",
                "set_rule_enabled",
                "add_device",
                "discover_network",
                "restart_container",
                "test_channel",
                "post_incident"
            ]
        );
        // Chaque outil d'écriture est cité dans les instructions, pour que le
        // modèle sache lesquels demandent la portée `write`.
        for writer in &writers {
            assert!(INSTRUCTIONS.contains(writer), "{writer}");
        }
    }

    #[test]
    fn annotations_describe_the_tool() {
        let catalogue = catalogue();
        let tool = |name: &str| catalogue.iter().find(|t| t["name"] == name).unwrap().clone();
        assert_eq!(tool("get_status")["annotations"]["readOnlyHint"], true);
        assert_eq!(tool("remove_silence")["annotations"]["destructiveHint"], true);
        assert_eq!(tool("silence_device")["annotations"]["destructiveHint"], false);
        assert_eq!(tool("test_channel")["annotations"]["openWorldHint"], true);
        assert_eq!(tool("add_device")["title"], "Add a device");
        assert!(tool("list_devices")["outputSchema"]["properties"]["devices"].is_object());
    }

    #[test]
    fn option_values_become_text() {
        assert_eq!(option_text(&json!(" public ")).as_deref(), Some("public"));
        assert_eq!(option_text(&json!(161)).as_deref(), Some("161"));
        assert_eq!(option_text(&json!(true)).as_deref(), Some("true"));
        assert_eq!(option_text(&json!({ "a": 1 })), None);
    }

    #[test]
    fn arguments_are_read_tolerantly() {
        let args = Args(json!({ "id": "3", "enabled": "yes", "hours": "2.5", "name": "  " }));
        assert_eq!(args.i64("id"), Some(3));
        assert_eq!(args.bool("enabled"), Some(true));
        assert_eq!(args.f64("hours"), Some(2.5));
        assert_eq!(args.str("name"), None);
        assert_eq!(args.device_ref().as_deref(), Some("3"));
    }
}
