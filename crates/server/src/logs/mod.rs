//! Journaux de DumbMonit lui-même : conservés sur disque, exportables, et
//! expédiés vers un service de journaux.
//!
//! Tout part d'une couche `tracing` ([`layer`]) posée à côté de la sortie
//! console. Chaque évènement retenu par le filtre global (`DUMBMONIT_LOG`)
//! devient un [`Record`], expurgé de ce qui ressemble à un secret, puis confié
//! sans attendre :
//!
//! * à l'écrivain disque ([`files`]), un fil dédié qui tient un fichier par jour
//!   dans `/data/logs`, compresse les jours passés et applique la rétention ;
//! * à chaque destination configurée ([`ship`]), par une file bornée en mémoire.
//!
//! Aucune de ces remises ne bloque : une file pleine perd l'enregistrement et le
//! compte. Un serveur de surveillance qui se fige parce que Graylog ne répond
//! plus serait pire que des journaux incomplets.
//!
//! Les entrées du journal d'audit (`auth::audit`) passent par le même chemin
//! sous la cible [`AUDIT_TARGET`], et sont marquées `"kind":"audit"`.

pub mod attach;
pub mod catalog;
pub mod central;
pub mod export;
pub mod files;
pub mod scrub;
pub mod ship;
pub mod sinks;
pub mod store;
pub mod syslog;
pub mod traps;

use std::cell::Cell;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex, OnceLock};

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{Map, Value};
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::Context;

/// Cible des entrées du journal d'audit.
pub const AUDIT_TARGET: &str = "dumbmonit::audit";

/// Cible des messages réservés à la console : le code de première
/// configuration, qui n'a rien à faire dans un fichier conservé deux semaines ni
/// chez un service tiers.
pub const CONSOLE_ONLY_TARGET: &str = "dumbmonit::console";

/// Capacité de la file vers l'écrivain disque.
const FILE_QUEUE: usize = 16_384;

/// Niveau d'un enregistrement, du plus grave au plus bavard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Level {
    pub fn from_tracing(level: &tracing::Level) -> Self {
        match *level {
            tracing::Level::ERROR => Self::Error,
            tracing::Level::WARN => Self::Warn,
            tracing::Level::INFO => Self::Info,
            tracing::Level::DEBUG => Self::Debug,
            tracing::Level::TRACE => Self::Trace,
        }
    }

    /// Niveaux qu'un utilisateur peut choisir comme seuil d'une destination.
    pub const CHOICES: [&'static str; 4] = ["error", "warn", "info", "debug"];

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "error" => Some(Self::Error),
            "warn" | "warning" => Some(Self::Warn),
            "info" => Some(Self::Info),
            "debug" => Some(Self::Debug),
            "trace" => Some(Self::Trace),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
            Self::Trace => "trace",
        }
    }

    pub fn upper(self) -> &'static str {
        match self {
            Self::Error => "ERROR",
            Self::Warn => "WARN",
            Self::Info => "INFO",
            Self::Debug => "DEBUG",
            Self::Trace => "TRACE",
        }
    }

    /// Gravité syslog (RFC 5424, §6.2.1), reprise par GELF.
    pub fn syslog_severity(self) -> u8 {
        match self {
            Self::Error => 3,
            Self::Warn => 4,
            Self::Info => 6,
            Self::Debug | Self::Trace => 7,
        }
    }

    /// Vrai si un enregistrement de ce niveau passe le seuil `min`.
    pub fn passes(self, min: Self) -> bool {
        self <= min
    }
}

/// Un évènement, tel qu'il est écrit et expédié.
#[derive(Debug, Clone)]
pub struct Record {
    pub ts: DateTime<Utc>,
    pub level: Level,
    pub target: String,
    pub message: String,
    pub fields: Map<String, Value>,
    pub audit: bool,
}

impl Record {
    /// Enregistrement construit à la main (message de test d'une destination).
    pub fn new(level: Level, message: impl Into<String>) -> Self {
        Self {
            ts: Utc::now(),
            level,
            target: "dumbmonit".into(),
            message: message.into(),
            fields: Map::new(),
            audit: false,
        }
    }

    pub fn kind(&self) -> &'static str {
        if self.audit { "audit" } else { "app" }
    }

    /// Horodatage RFC 3339 en millisecondes, suffixe `Z`.
    pub fn ts_rfc3339(&self) -> String {
        self.ts.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    }

    /// La ligne JSON du fichier disque.
    pub fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.insert("ts".into(), Value::String(self.ts_rfc3339()));
        object.insert("level".into(), Value::String(self.level.upper().into()));
        object.insert("target".into(), Value::String(self.target.clone()));
        object.insert("msg".into(), Value::String(self.message.clone()));
        if !self.fields.is_empty() {
            object.insert("fields".into(), Value::Object(self.fields.clone()));
        }
        object.insert("kind".into(), Value::String(self.kind().into()));
        Value::Object(object)
    }

    pub fn to_line(&self) -> String {
        self.to_json().to_string()
    }

    /// Champs aplatis en chaînes, pour les formats qui n'acceptent que cela.
    pub fn flat_fields(&self) -> impl Iterator<Item = (&str, String)> {
        self.fields.iter().map(|(key, value)| {
            let text = match value {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            };
            (key.as_str(), text)
        })
    }
}

// --------------------------------------------------------------------------
// Réglages partagés avec l'écrivain disque
// --------------------------------------------------------------------------

/// Rétention par défaut, en jours.
pub const DEFAULT_RETENTION_DAYS: u32 = 14;
/// Plafond par défaut de l'ensemble des fichiers, en mégaoctets.
pub const DEFAULT_MAX_TOTAL_MB: u64 = 1024;
pub const MIN_RETENTION_DAYS: u32 = 1;
pub const MAX_RETENTION_DAYS: u32 = 365;
pub const MIN_TOTAL_MB: u64 = 16;
pub const MAX_TOTAL_MB: u64 = 102_400;

/// Rétention et plafond, modifiables à chaud depuis l'API.
#[derive(Debug)]
pub struct Limits {
    retention_days: AtomicU32,
    max_total_bytes: AtomicU64,
}

impl Default for Limits {
    fn default() -> Self {
        Self::new(DEFAULT_RETENTION_DAYS, DEFAULT_MAX_TOTAL_MB)
    }
}

impl Limits {
    pub fn new(retention_days: u32, max_total_mb: u64) -> Self {
        Self {
            retention_days: AtomicU32::new(retention_days),
            max_total_bytes: AtomicU64::new(max_total_mb * 1024 * 1024),
        }
    }

    pub fn retention_days(&self) -> u32 {
        self.retention_days.load(Ordering::Relaxed)
    }

    pub fn max_total_bytes(&self) -> u64 {
        self.max_total_bytes.load(Ordering::Relaxed)
    }

    pub fn set(&self, retention_days: u32, max_total_mb: u64) {
        self.retention_days.store(retention_days, Ordering::Relaxed);
        self.max_total_bytes.store(max_total_mb * 1024 * 1024, Ordering::Relaxed);
    }
}

// --------------------------------------------------------------------------
// Concentrateur global
// --------------------------------------------------------------------------

/// Ce que la couche `tracing` alimente. Global, car l'abonné est installé avant
/// que la configuration, la base ou le runtime n'existent.
pub struct Hub {
    file_tx: SyncSender<Arc<Record>>,
    file_rx: Mutex<Option<Receiver<Arc<Record>>>>,
    /// Enregistrements perdus avant le disque : file pleine ou plafond atteint.
    pub file_dropped: Arc<AtomicU64>,
    pub writer_active: std::sync::atomic::AtomicBool,
    pub limits: Arc<Limits>,
    pub routes: ship::Routes,
}

static HUB: OnceLock<Hub> = OnceLock::new();

/// Le concentrateur, créé au premier appel.
pub fn hub() -> &'static Hub {
    HUB.get_or_init(|| {
        let (file_tx, file_rx) = std::sync::mpsc::sync_channel(FILE_QUEUE);
        Hub {
            file_tx,
            file_rx: Mutex::new(Some(file_rx)),
            file_dropped: Arc::new(AtomicU64::new(0)),
            writer_active: std::sync::atomic::AtomicBool::new(false),
            limits: Arc::new(Limits::default()),
            routes: ship::Routes::default(),
        }
    })
}

/// Répertoire des fichiers de journaux.
pub fn dir(data_dir: &std::path::Path) -> std::path::PathBuf {
    data_dir.join("logs")
}

/// Mise en route une fois la base ouverte : rétention enregistrée, écrivain
/// disque, puis destinations. Aucun échec ici n'empêche le serveur de démarrer.
pub async fn start(
    pool: &sqlx::SqlitePool,
    cipher: &crate::crypto::Cipher,
    data_dir: &std::path::Path,
) {
    match store::limits(pool).await {
        Ok((retention, max_total)) => hub().limits.set(retention, max_total),
        Err(error) => {
            tracing::warn!(error = %format!("{error:#}"), "log retention settings unreadable, defaults used")
        }
    }
    match start_disk_writer(dir(data_dir)) {
        Ok(()) => tracing::info!(directory = %dir(data_dir).display(), "server logs kept on disk"),
        Err(error) => tracing::warn!(%error, "server logs cannot be written to disk"),
    }
    if let Err(error) = ship::reload(pool, cipher).await {
        tracing::warn!(error = %format!("{error:#}"), "log destinations not started");
    }
}

/// Démarre l'écrivain disque dans `dir` (une seule fois ; les appels suivants
/// ne font rien). Les enregistrements émis depuis le démarrage du processus
/// attendaient dans la file : ils sont écrits les premiers.
pub fn start_disk_writer(dir: std::path::PathBuf) -> std::io::Result<()> {
    let hub = hub();
    let Some(rx) = hub.file_rx.lock().unwrap_or_else(|e| e.into_inner()).take() else {
        return Ok(());
    };
    std::fs::create_dir_all(&dir)?;
    let writer = files::DiskWriter::new(dir, hub.limits.clone(), hub.file_dropped.clone());
    hub.writer_active.store(true, Ordering::Relaxed);
    std::thread::Builder::new()
        .name("dumbmonit-logs".into())
        .spawn(move || writer.run(rx))
        .map(|_| ())
}

thread_local! {
    /// Garde contre la réentrance : un évènement émis pendant le traitement
    /// d'un autre (par une bibliothèque appelée ici) est ignoré.
    static BUSY: Cell<bool> = const { Cell::new(false) };
}

/// Cibles jamais expédiées : le code d'expédition lui-même et la pile HTTP
/// qu'il emploie. Les expédier ferait de chaque envoi la source du suivant.
fn ship_excluded(target: &str) -> bool {
    const PREFIXES: [&str; 7] =
        ["dumbmonit_server::logs", "reqwest", "hyper", "h2", "rustls", "tokio_rustls", "tower"];
    PREFIXES.iter().any(|prefix| target.starts_with(prefix))
}

/// Couche `tracing` qui alimente le disque et les destinations.
pub struct LogLayer;

/// La couche à empiler sur l'abonné.
pub fn layer() -> LogLayer {
    let _ = hub();
    LogLayer
}

impl<S: Subscriber> tracing_subscriber::Layer<S> for LogLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        if BUSY.with(|busy| busy.replace(true)) {
            return;
        }
        let record = Arc::new(build_record(event));
        dispatch(record);
        BUSY.with(|busy| busy.set(false));
    }
}

/// Remet un enregistrement au disque et aux destinations, sans jamais attendre.
pub fn dispatch(record: Arc<Record>) {
    let hub = hub();
    match hub.file_tx.try_send(record.clone()) {
        Ok(()) => {}
        Err(TrySendError::Full(_)) => {
            hub.file_dropped.fetch_add(1, Ordering::Relaxed);
        }
        // L'écrivain n'existe plus : rien à compter, le disque est hors jeu.
        Err(TrySendError::Disconnected(_)) => {}
    }
    if !ship_excluded(&record.target) {
        hub.routes.offer(&record);
    }
}

fn build_record(event: &Event<'_>) -> Record {
    let metadata = event.metadata();
    let target = metadata.target();
    let level = Level::from_tracing(metadata.level());
    if target == CONSOLE_ONLY_TARGET {
        return Record {
            ts: Utc::now(),
            level,
            target: target.to_string(),
            message: "message shown on the console only".into(),
            fields: Map::new(),
            audit: false,
        };
    }
    let mut visitor = Collect::default();
    event.record(&mut visitor);
    let mut fields = Map::new();
    for (key, value) in visitor.fields {
        let value = match value {
            Value::String(text) => Value::String(scrub::text(&text)),
            other => other,
        };
        let value = scrub::field(&key, value);
        fields.insert(key, value);
    }
    Record {
        ts: Utc::now(),
        level,
        target: target.to_string(),
        message: scrub::text(&visitor.message),
        fields,
        audit: target == AUDIT_TARGET,
    }
}

#[derive(Default)]
struct Collect {
    message: String,
    fields: Vec<(String, Value)>,
}

impl Visit for Collect {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let text = format!("{value:?}");
        if field.name() == "message" {
            self.message = text;
        } else {
            self.fields.push((field.name().to_string(), Value::String(text)));
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        } else {
            self.fields.push((field.name().to_string(), Value::String(value.to_string())));
        }
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields.push((field.name().to_string(), Value::from(value)));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields.push((field.name().to_string(), Value::from(value)));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.fields.push((field.name().to_string(), Value::from(value)));
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.fields.push((field.name().to_string(), Value::from(value)));
    }
}

/// Nom d'hôte annoncé aux destinations : celui du conteneur.
pub fn hostname() -> &'static str {
    static NAME: OnceLock<String> = OnceLock::new();
    NAME.get_or_init(|| {
        std::env::var("HOSTNAME")
            .ok()
            .or_else(|| std::fs::read_to_string("/etc/hostname").ok())
            .map(|name| name.trim().to_string())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "dumbmonit".into())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::layer::SubscriberExt;

    /// Couche de test : capture les enregistrements construits.
    struct Capture(Arc<Mutex<Vec<Record>>>);

    impl<S: Subscriber> tracing_subscriber::Layer<S> for Capture {
        fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
            self.0.lock().unwrap().push(build_record(event));
        }
    }

    fn capture(emit: impl FnOnce()) -> Vec<Record> {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let subscriber = tracing_subscriber::registry().with(Capture(seen.clone()));
        tracing::subscriber::with_default(subscriber, emit);
        seen.lock().unwrap().clone()
    }

    #[test]
    fn un_evenement_devient_une_ligne_json_complete() {
        let records = capture(|| {
            tracing::warn!(target: "dumbmonit_server::scheduler", target_id = 4_i64, ok = false, kind = %"snmp", "probe failed");
        });
        let line: Value = serde_json::from_str(&records[0].to_line()).unwrap();
        assert_eq!(line["level"], "WARN");
        assert_eq!(line["msg"], "probe failed");
        assert_eq!(line["fields"]["target_id"], 4);
        assert_eq!(line["fields"]["ok"], false);
        assert_eq!(line["fields"]["kind"], "snmp");
        assert_eq!(line["kind"], "app");
        assert!(line["ts"].as_str().unwrap().ends_with('Z'));
    }

    #[test]
    fn l_audit_est_marque_et_la_console_seule_masquee() {
        let records = capture(|| {
            tracing::info!(target: AUDIT_TARGET, action = "login", "audit");
            tracing::warn!(target: CONSOLE_ONLY_TARGET, "First-run setup code: ABCD-EFGH");
        });
        assert_eq!(records[0].kind(), "audit");
        assert!(!records[1].to_line().contains("ABCD-EFGH"));
    }

    #[test]
    fn aucun_secret_ne_sort_d_un_evenement() {
        let records = capture(|| {
            tracing::warn!(
                error = %"error sending request for url (https://api.telegram.org/bot123456:AAH-secretpart/sendMessage)",
                password = "hunter2-very-secret",
                url = "https://admin:s3cr3t-pass@example.com/api?token=abcdef123456&x=1",
                "request failed with Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.payload"
            );
        });
        let line = records[0].to_line();
        for secret in [
            "AAH-secretpart",
            "hunter2-very-secret",
            "s3cr3t-pass",
            "abcdef123456",
            "eyJhbGciOiJIUzI1NiJ9",
        ] {
            assert!(!line.contains(secret), "{secret} présent dans {line}");
        }
        assert!(line.contains("example.com"));
    }

    #[test]
    fn seuils_de_niveau() {
        assert!(Level::Error.passes(Level::Info));
        assert!(Level::Info.passes(Level::Info));
        assert!(!Level::Debug.passes(Level::Info));
        assert_eq!(Level::parse("WARNING"), Some(Level::Warn));
    }
}
