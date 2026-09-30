//! Compteurs de performance Windows (PDH).
//!
//! Certains logiciels Windows ne publient leur état que là : MDaemon, par
//! exemple, expose ses files d'attente, ses sessions et ses statistiques de
//! messages dans l'objet de performance « MDaemon », et nulle part ailleurs
//! (ni son API XML documentée, ni un fichier lisible). L'agent sait donc lire
//! des chemins de compteurs (`\Objet(Instance)\Compteur`), de deux façons :
//!
//! - une liste libre dans `agent.yaml` (`perf_counters`), publiée en
//!   `agent_perf_counter{counter="…"}` ;
//! - un jeu prédéfini pour MDaemon (`mdaemon: true`, ou `auto` — le défaut —
//!   quand le service Windows « MDaemon » existe), publié sous des noms stables
//!   (`mdaemon_queue_messages{queue="retry"}`…) que les règles et l'interface
//!   du serveur connaissent.
//!
//! # Découpage
//!
//! Les appels PDH vivent derrière le trait [`CounterSource`], dont la seule
//! implémentation réelle est compilée sous Windows. Tout le reste — analyse des
//! chemins, correspondance compteur → série, décision d'ouvrir ou de rouvrir la
//! requête, mise en forme — est pur et vérifié sous Linux avec une source
//! factice. Hors Windows, la source ne sait rien lire : une liste configurée y
//! est signalée une fois dans le journal, et rien n'est émis.
//!
//! # Noms des compteurs MDaemon
//!
//! L'aide de MDaemon (« MDaemon's Main Display ») confirme l'objet `MDaemon` et
//! ses familles de compteurs (sessions actives, messages en file, état des
//! serveurs, temps de fonctionnement, statistiques) sans en donner la liste.
//! Les noms exacts viennent de la liste publiée par Zen Software, revendeur
//! MDaemon, pour MDaemon 13.5 (« Monitor MDaemon server load using Perfmon »,
//! février 2014). Ils n'ont été vérifiés contre aucun serveur réel : un nom
//! refusé par PDH est signalé une fois dans le journal, et les autres sont lus
//! quand même.
//!
//! Les noms sont ajoutés par `PdhAddEnglishCounterW` : un Windows en français ou
//! en allemand traduit les noms des compteurs du système, et le même
//! `agent.yaml` doit fonctionner partout.

use std::collections::BTreeSet;

use dumbmonit_proto::{MetricKind, Sample};
use tracing::{debug, info, warn};

/// Objet de performance publié par MDaemon.
pub const MDAEMON_OBJECT: &str = "MDaemon";

/// Nom du service Windows du moteur de messagerie MDaemon.
pub const MDAEMON_SERVICE: &str = "MDaemon";

/// Série d'un compteur de la liste libre.
pub const CUSTOM_METRIC: &str = "agent_perf_counter";

/// Un cycle sur dix, l'agent vérifie si l'ensemble à lire a changé (MDaemon
/// installé ou désinstallé) et rouvre la requête si un compteur avait été
/// refusé — une DLL de compteurs s'enregistre parfois après le démarrage du
/// service, et l'agent ne doit pas exiger d'être redémarré pour la voir.
pub const REOPEN_EVERY: u64 = 10;

/// Plafond de la liste libre : chaque compteur est une série, et une liste
/// démesurée trahit presque toujours un joker mal placé.
pub const MAX_CUSTOM_COUNTERS: usize = 200;

/// Faut-il lire le jeu MDaemon ?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PresetMode {
    /// Seulement si le service Windows « MDaemon » existe.
    #[default]
    Auto,
    On,
    Off,
}

impl PresetMode {
    /// `true`, `false` ou `auto`, avec les mêmes synonymes que les autres
    /// drapeaux de l'agent.
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "auto" | "" => Some(Self::Auto),
            "1" | "true" | "oui" | "yes" | "on" => Some(Self::On),
            "0" | "false" | "non" | "no" | "off" => Some(Self::Off),
            _ => None,
        }
    }
}

/// Un compteur de la liste libre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomCounter {
    /// Chemin complet, `\Objet(Instance)\Compteur`.
    pub path: String,
    /// Valeur de l'étiquette `counter` ; le chemin lui-même quand rien n'est
    /// précisé.
    pub name: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PerfCountersConfig {
    pub counters: Vec<CustomCounter>,
    pub mdaemon: PresetMode,
}

// ------------------------------------------------------------------ chemins

/// Un chemin de compteur, découpé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CounterPath {
    /// `\\machine` en tête, rarement utile : l'agent lit sa propre machine.
    pub machine: Option<String>,
    pub object: String,
    /// `_Total`, `C:`… ; `None` pour un objet à instance unique.
    pub instance: Option<String>,
    pub counter: String,
}

/// Analyse `\Objet(Instance)\Compteur` (ou `\\machine\Objet…`).
///
/// L'instance est entre la première parenthèse ouvrante de l'objet et la
/// dernière fermante avant la barre qui précède le compteur : un nom
/// d'instance peut lui-même contenir des parenthèses (`Intel(R) Ethernet`).
pub fn parse_path(raw: &str) -> Result<CounterPath, String> {
    let path = raw.trim();
    let (machine, rest) = if let Some(after) = path.strip_prefix(r"\\") {
        let (machine, rest) = after
            .split_once('\\')
            .ok_or_else(|| format!("'{path}': no object after the machine"))?;
        (Some(machine.to_string()), rest)
    } else if let Some(rest) = path.strip_prefix('\\') {
        (None, rest)
    } else {
        return Err(format!("'{path}': a counter path starts with a backslash"));
    };
    let (object_part, counter) =
        rest.rsplit_once('\\').ok_or_else(|| format!("'{path}': no counter name"))?;
    let counter = counter.trim();
    if counter.is_empty() {
        return Err(format!("'{path}': no counter name"));
    }
    let (object, instance) = match object_part.find('(') {
        Some(open) => {
            let inner = object_part[open + 1..]
                .strip_suffix(')')
                .ok_or_else(|| format!("'{path}': unbalanced parentheses in the instance"))?;
            (&object_part[..open], Some(inner.to_string()))
        }
        None => (object_part, None),
    };
    let object = object.trim();
    if object.is_empty() {
        return Err(format!("'{path}': no object name"));
    }
    Ok(CounterPath {
        machine,
        object: object.to_string(),
        instance: instance.filter(|i| !i.is_empty()),
        counter: counter.to_string(),
    })
}

// ------------------------------------------------------------ correspondance

/// Un compteur à lire, et la série où sa valeur atterrit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CounterSpec {
    pub path: String,
    pub metric: String,
    pub labels: Vec<(String, String)>,
    pub kind: MetricKind,
}

impl CounterSpec {
    fn new(path: String, metric: &str, labels: &[(&str, &str)], kind: MetricKind) -> Self {
        Self {
            path,
            metric: metric.to_string(),
            labels: labels.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            kind,
        }
    }
}

/// Un compteur d'un jeu prédéfini : (compteur, série, étiquettes, nature).
type PresetCounter =
    (&'static str, &'static str, &'static [(&'static str, &'static str)], MetricKind);

/// Le jeu MDaemon.
///
/// Les compteurs « …/sec » ne sont pas lus : ce sont des dérivées que PDH
/// calcule entre deux lectures, alors que leurs totaux cumulés sont publiés
/// aussi — le taux se calcule à la lecture, sans état dans l'agent. « Active
/// MDaemon sessions » n'est pas lu non plus : c'est la somme des sessions SMTP,
/// POP3 et IMAP, et la publier sous la même série la compterait deux fois.
const MDAEMON_COUNTERS: &[PresetCounter] = &[
    // Files d'attente : nombre de messages.
    (
        "Inbound queue messages",
        "mdaemon_queue_messages",
        &[("queue", "inbound")],
        MetricKind::Gauge,
    ),
    ("Local queue messages", "mdaemon_queue_messages", &[("queue", "local")], MetricKind::Gauge),
    ("Remote queue messages", "mdaemon_queue_messages", &[("queue", "remote")], MetricKind::Gauge),
    ("Retry queue messages", "mdaemon_queue_messages", &[("queue", "retry")], MetricKind::Gauge),
    ("Bad queue messages", "mdaemon_queue_messages", &[("queue", "bad")], MetricKind::Gauge),
    (
        "Holding queue messages",
        "mdaemon_queue_messages",
        &[("queue", "holding")],
        MetricKind::Gauge,
    ),
    ("LAN queue messages", "mdaemon_queue_messages", &[("queue", "lan")], MetricKind::Gauge),
    (
        "Quarantine queue messages",
        "mdaemon_queue_messages",
        &[("queue", "quarantine")],
        MetricKind::Gauge,
    ),
    ("RAW queue messages", "mdaemon_queue_messages", &[("queue", "raw")], MetricKind::Gauge),
    // Files gelées par l'administrateur : 1 oui, 0 non.
    ("Inbound queue frozen", "mdaemon_queue_frozen", &[("queue", "inbound")], MetricKind::Gauge),
    ("Local queue frozen", "mdaemon_queue_frozen", &[("queue", "local")], MetricKind::Gauge),
    ("Remote queue frozen", "mdaemon_queue_frozen", &[("queue", "remote")], MetricKind::Gauge),
    // Sessions en cours.
    (
        "Active SMTP (in) sessions",
        "mdaemon_sessions_active",
        &[("protocol", "smtp_in")],
        MetricKind::Gauge,
    ),
    (
        "Active SMTP (out) sessions",
        "mdaemon_sessions_active",
        &[("protocol", "smtp_out")],
        MetricKind::Gauge,
    ),
    (
        "Active POP3 (in) sessions",
        "mdaemon_sessions_active",
        &[("protocol", "pop3_in")],
        MetricKind::Gauge,
    ),
    (
        "Active POP3 (out) sessions",
        "mdaemon_sessions_active",
        &[("protocol", "pop3_out")],
        MetricKind::Gauge,
    ),
    ("Active IMAP sessions", "mdaemon_sessions_active", &[("protocol", "imap")], MetricKind::Gauge),
    (
        "Active Webmail sessions",
        "mdaemon_sessions_active",
        &[("protocol", "webmail")],
        MetricKind::Gauge,
    ),
    // Totaux depuis le démarrage de MDaemon.
    (
        "SMTP sessions (in) total",
        "mdaemon_sessions_total",
        &[("protocol", "smtp_in")],
        MetricKind::Counter,
    ),
    (
        "SMTP sessions (out) total",
        "mdaemon_sessions_total",
        &[("protocol", "smtp_out")],
        MetricKind::Counter,
    ),
    ("POP3 sessions total", "mdaemon_sessions_total", &[("protocol", "pop3")], MetricKind::Counter),
    ("IMAP sessions total", "mdaemon_sessions_total", &[("protocol", "imap")], MetricKind::Counter),
    (
        "SMTP messages (in) total",
        "mdaemon_messages_total",
        &[("protocol", "smtp_in")],
        MetricKind::Counter,
    ),
    (
        "SMTP messages (out) total",
        "mdaemon_messages_total",
        &[("protocol", "smtp_out")],
        MetricKind::Counter,
    ),
    (
        "DomainPOP messages (in) total",
        "mdaemon_messages_total",
        &[("protocol", "domainpop_in")],
        MetricKind::Counter,
    ),
    (
        "spam accepted total",
        "mdaemon_filtered_messages_total",
        &[("filter", "spam"), ("verdict", "accepted")],
        MetricKind::Counter,
    ),
    (
        "spam refused total",
        "mdaemon_filtered_messages_total",
        &[("filter", "spam"), ("verdict", "refused")],
        MetricKind::Counter,
    ),
    (
        "Viruses accepted total",
        "mdaemon_filtered_messages_total",
        &[("filter", "virus"), ("verdict", "accepted")],
        MetricKind::Counter,
    ),
    (
        "Viruses refused total",
        "mdaemon_filtered_messages_total",
        &[("filter", "virus"), ("verdict", "refused")],
        MetricKind::Counter,
    ),
    (
        "DNSBL accepted total",
        "mdaemon_filtered_messages_total",
        &[("filter", "dnsbl"), ("verdict", "accepted")],
        MetricKind::Counter,
    ),
    (
        "DNSBL refused total",
        "mdaemon_filtered_messages_total",
        &[("filter", "dnsbl"), ("verdict", "refused")],
        MetricKind::Counter,
    ),
    // Serveurs internes de MDaemon : 1 actif, 0 inactif.
    ("SMTP server state", "mdaemon_server_active", &[("server", "smtp")], MetricKind::Gauge),
    ("POP3 server state", "mdaemon_server_active", &[("server", "pop3")], MetricKind::Gauge),
    ("IMAP server state", "mdaemon_server_active", &[("server", "imap")], MetricKind::Gauge),
    ("Web Mail server state", "mdaemon_server_active", &[("server", "webmail")], MetricKind::Gauge),
    (
        "Web Admin server state",
        "mdaemon_server_active",
        &[("server", "webadmin")],
        MetricKind::Gauge,
    ),
    (
        "ActiveSync server state",
        "mdaemon_server_active",
        &[("server", "activesync")],
        MetricKind::Gauge,
    ),
    (
        "AntiSpam server state",
        "mdaemon_server_active",
        &[("server", "antispam")],
        MetricKind::Gauge,
    ),
    (
        "AntiVirus server state",
        "mdaemon_server_active",
        &[("server", "antivirus")],
        MetricKind::Gauge,
    ),
    ("Minger server state", "mdaemon_server_active", &[("server", "minger")], MetricKind::Gauge),
    (
        "MultiPOP server state",
        "mdaemon_server_active",
        &[("server", "multipop")],
        MetricKind::Gauge,
    ),
    (
        "DomainPOP server state",
        "mdaemon_server_active",
        &[("server", "domainpop")],
        MetricKind::Gauge,
    ),
    // Le moteur lui-même.
    ("MDaemon running state", "mdaemon_running", &[], MetricKind::Gauge),
    ("MDaemon up time", "mdaemon_uptime_seconds", &[], MetricKind::Gauge),
];

/// Le jeu MDaemon, en chemins complets.
pub fn mdaemon_specs() -> Vec<CounterSpec> {
    MDAEMON_COUNTERS
        .iter()
        .map(|(counter, metric, labels, kind)| {
            CounterSpec::new(format!(r"\{MDAEMON_OBJECT}\{counter}"), metric, labels, *kind)
        })
        .collect()
}

/// La liste libre : une jauge par compteur, identifiée par son nom.
pub fn custom_specs(counters: &[CustomCounter]) -> Vec<CounterSpec> {
    counters
        .iter()
        .take(MAX_CUSTOM_COUNTERS)
        .map(|c| {
            CounterSpec::new(
                c.path.clone(),
                CUSTOM_METRIC,
                &[("counter", c.name.as_str())],
                MetricKind::Gauge,
            )
        })
        .collect()
}

/// Traduit les valeurs lues en échantillons. Une valeur absente ou non finie
/// ne produit rien : mieux vaut un trou qu'un zéro qui ferait croire à une
/// file vide.
pub fn samples(report: &PerfReport, now_ms: i64) -> Vec<Sample> {
    report
        .values
        .iter()
        .filter(|(_, value)| value.is_finite())
        .map(|(spec, value)| {
            let mut sample = Sample::new(&spec.metric, *value, spec.kind, now_ms);
            for (key, label) in &spec.labels {
                sample = sample.with_label(key, label);
            }
            sample
        })
        .collect()
}

// ------------------------------------------------------------------ lecture

/// Accès aux compteurs du système.
pub trait CounterSource {
    /// Remplace l'ensemble lu. Rend, pour chaque chemin et dans l'ordre, la
    /// raison de son refus s'il n'a pas pu être ajouté.
    fn open(&mut self, paths: &[String]) -> Vec<Result<(), String>>;
    /// Une lecture de tous les compteurs ouverts, dans l'ordre de `open`.
    /// `None` : pas de valeur ce cycle (compteur refusé, donnée invalide).
    fn read(&mut self) -> Vec<Option<f64>>;
    /// Le service Windows existe-t-il sur cette machine ?
    fn service_installed(&self, name: &str) -> bool;
}

/// Ce qu'un cycle a lu.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PerfReport {
    pub values: Vec<(CounterSpec, f64)>,
}

pub struct PerfCountersProbe<S: CounterSource> {
    config: PerfCountersConfig,
    source: S,
    /// Ensemble ouvert, dans l'ordre de la source.
    opened: Vec<CounterSpec>,
    /// Vrai si au moins un compteur a été refusé à la dernière ouverture.
    refused: bool,
    cycle: u64,
    /// Chemins déjà signalés en échec : un seul avertissement par chemin.
    announced: BTreeSet<String>,
    mdaemon_announced: Option<bool>,
}

/// La source de la plateforme : PDH sous Windows, rien ailleurs.
pub type PlatformProbe = PerfCountersProbe<PlatformSource>;

impl PlatformProbe {
    pub fn for_platform(config: &PerfCountersConfig) -> Self {
        PerfCountersProbe::new(config, PlatformSource::new())
    }
}

impl<S: CounterSource> PerfCountersProbe<S> {
    pub fn new(config: &PerfCountersConfig, source: S) -> Self {
        Self {
            config: config.clone(),
            source,
            opened: Vec::new(),
            refused: false,
            cycle: 0,
            announced: BTreeSet::new(),
            mdaemon_announced: None,
        }
    }

    /// Ce qu'il faut lire en ce moment.
    fn wanted(&mut self) -> Vec<CounterSpec> {
        let mut specs = custom_specs(&self.config.counters);
        let mdaemon = match self.config.mdaemon {
            PresetMode::On => true,
            PresetMode::Off => false,
            PresetMode::Auto => self.source.service_installed(MDAEMON_SERVICE),
        };
        if self.mdaemon_announced != Some(mdaemon) {
            if mdaemon {
                info!("reading the MDaemon performance counters");
            } else if self.mdaemon_announced.is_some() {
                info!("MDaemon is gone: its performance counters are no longer read");
            }
            self.mdaemon_announced = Some(mdaemon);
        }
        if mdaemon {
            specs.extend(mdaemon_specs());
        }
        specs
    }

    /// Un cycle. `None` : rien n'est configuré ni détecté.
    pub fn read(&mut self) -> Option<PerfReport> {
        let recheck = self.cycle.is_multiple_of(REOPEN_EVERY);
        self.cycle = self.cycle.wrapping_add(1);

        if recheck {
            let wanted = self.wanted();
            if wanted != self.opened || self.refused {
                self.reopen(wanted);
            }
        }
        if self.opened.is_empty() {
            return None;
        }
        let values = self.source.read();
        let values = self
            .opened
            .iter()
            .zip(values)
            .filter_map(|(spec, value)| value.map(|v| (spec.clone(), v)))
            .collect();
        Some(PerfReport { values })
    }

    fn reopen(&mut self, wanted: Vec<CounterSpec>) {
        let paths: Vec<String> = wanted.iter().map(|s| s.path.clone()).collect();
        let outcomes = if paths.is_empty() { Vec::new() } else { self.source.open(&paths) };
        self.refused = false;
        for (path, outcome) in paths.iter().zip(&outcomes) {
            if let Err(reason) = outcome {
                self.refused = true;
                if self.announced.insert(path.clone()) {
                    warn!(counter = path, reason, "performance counter not available");
                } else {
                    debug!(counter = path, reason, "performance counter still not available");
                }
            }
        }
        self.opened = wanted;
    }
}

// ----------------------------------------------------- source de la plateforme

#[cfg(not(windows))]
#[derive(Default)]
pub struct PlatformSource;

#[cfg(not(windows))]
impl PlatformSource {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(windows))]
impl CounterSource for PlatformSource {
    fn open(&mut self, paths: &[String]) -> Vec<Result<(), String>> {
        paths
            .iter()
            .map(|_| Err("performance counters exist only on Windows".to_string()))
            .collect()
    }

    fn read(&mut self) -> Vec<Option<f64>> {
        Vec::new()
    }

    fn service_installed(&self, _name: &str) -> bool {
        false
    }
}

#[cfg(windows)]
pub use pdh::PdhSource as PlatformSource;

#[cfg(windows)]
mod pdh {
    //! Lecture par la bibliothèque PDH (`pdh.dll`).
    //!
    //! Une requête reste ouverte d'un cycle à l'autre : un compteur de débit
    //! (`…/sec`) n'a de valeur qu'entre deux collectes, et rouvrir à chaque
    //! cycle le laisserait toujours sans valeur.

    use std::ptr;

    use windows_sys::Win32::System::Performance::{
        PDH_CSTATUS_BAD_COUNTERNAME, PDH_CSTATUS_NEW_DATA, PDH_CSTATUS_NO_COUNTER,
        PDH_CSTATUS_NO_INSTANCE, PDH_CSTATUS_NO_OBJECT, PDH_CSTATUS_VALID_DATA,
        PDH_FMT_COUNTERVALUE, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PdhAddEnglishCounterW,
        PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterValue, PdhOpenQueryW,
    };

    use super::CounterSource;

    /// `PDH_FMT_NOCAP100` : sans lui, un pourcentage au-delà de 100 (processeur
    /// sur plusieurs cœurs) serait écrêté. Absent des liaisons `windows-sys`.
    const PDH_FMT_NOCAP100: u32 = 0x0000_8000;

    pub struct PdhSource {
        query: PDH_HQUERY,
        counters: Vec<Option<PDH_HCOUNTER>>,
    }

    impl Default for PdhSource {
        fn default() -> Self {
            Self { query: ptr::null_mut(), counters: Vec::new() }
        }
    }

    impl PdhSource {
        pub fn new() -> Self {
            Self::default()
        }

        fn close(&mut self) {
            if !self.query.is_null() {
                // SAFETY : la requête a été ouverte par `PdhOpenQueryW` et n'est
                // fermée qu'ici ; fermer la requête libère ses compteurs.
                unsafe { PdhCloseQuery(self.query) };
                self.query = ptr::null_mut();
            }
            self.counters.clear();
        }
    }

    impl Drop for PdhSource {
        fn drop(&mut self) {
            self.close();
        }
    }

    fn describe(status: u32) -> String {
        match status {
            PDH_CSTATUS_NO_OBJECT => "no such performance object on this machine".to_string(),
            PDH_CSTATUS_NO_COUNTER => "no such counter in this object".to_string(),
            PDH_CSTATUS_NO_INSTANCE => "no such instance of this object".to_string(),
            PDH_CSTATUS_BAD_COUNTERNAME => "malformed counter path".to_string(),
            other => format!("PDH error 0x{other:08X}"),
        }
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    impl CounterSource for PdhSource {
        fn open(&mut self, paths: &[String]) -> Vec<Result<(), String>> {
            self.close();
            let mut query: PDH_HQUERY = ptr::null_mut();
            // SAFETY : source nulle = compteurs en temps réel ; `query` est un
            // emplacement valide que PDH remplit.
            let status = unsafe { PdhOpenQueryW(ptr::null(), 0, &mut query) };
            if status != 0 {
                let reason = describe(status);
                return paths.iter().map(|_| Err(reason.clone())).collect();
            }
            self.query = query;
            paths
                .iter()
                .map(|path| {
                    let path_w = wide(path);
                    let mut counter: PDH_HCOUNTER = ptr::null_mut();
                    // SAFETY : `path_w` est terminé par un zéro et vit jusqu'au
                    // retour ; `counter` est un emplacement valide.
                    let status = unsafe {
                        PdhAddEnglishCounterW(self.query, path_w.as_ptr(), 0, &mut counter)
                    };
                    if status == 0 {
                        self.counters.push(Some(counter));
                        Ok(())
                    } else {
                        self.counters.push(None);
                        Err(describe(status))
                    }
                })
                .collect()
        }

        fn read(&mut self) -> Vec<Option<f64>> {
            if self.query.is_null() {
                return self.counters.iter().map(|_| None).collect();
            }
            // SAFETY : requête ouverte, non encore fermée.
            let status = unsafe { PdhCollectQueryData(self.query) };
            if status != 0 {
                tracing::debug!(error = super::pdh_hex(status), "PDH collection failed");
                return self.counters.iter().map(|_| None).collect();
            }
            self.counters
                .iter()
                .map(|counter| {
                    let counter = (*counter)?;
                    // SAFETY : structure C entièrement initialisée à zéro, puis
                    // remplie par PDH ; `lpdwtype` est facultatif.
                    let mut value: PDH_FMT_COUNTERVALUE = unsafe { std::mem::zeroed() };
                    let status = unsafe {
                        PdhGetFormattedCounterValue(
                            counter,
                            PDH_FMT_DOUBLE | PDH_FMT_NOCAP100,
                            ptr::null_mut(),
                            &mut value,
                        )
                    };
                    let valid = value.CStatus == PDH_CSTATUS_VALID_DATA
                        || value.CStatus == PDH_CSTATUS_NEW_DATA;
                    if status == 0 && valid {
                        // SAFETY : `PDH_FMT_DOUBLE` demandé, c'est donc le
                        // champ `doubleValue` que PDH a écrit.
                        Some(unsafe { value.Anonymous.doubleValue })
                    } else {
                        None
                    }
                })
                .collect()
        }

        fn service_installed(&self, name: &str) -> bool {
            use windows_service::service::ServiceAccess;
            use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};
            ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
                .and_then(|manager| manager.open_service(name, ServiceAccess::QUERY_STATUS))
                .is_ok()
        }
    }
}

/// Code PDH lisible, pour le journal.
#[cfg_attr(not(windows), allow(dead_code))]
fn pdh_hex(status: u32) -> String {
    format!("0x{status:08X}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_counter_path_is_split_into_its_parts() {
        assert_eq!(
            parse_path(r"\Processor(_Total)\% Processor Time").unwrap(),
            CounterPath {
                machine: None,
                object: "Processor".into(),
                instance: Some("_Total".into()),
                counter: "% Processor Time".into(),
            }
        );
        let single = parse_path(r"\MDaemon\Retry queue messages").unwrap();
        assert_eq!(single.object, "MDaemon");
        assert_eq!(single.instance, None);
        assert_eq!(single.counter, "Retry queue messages");

        // Des parenthèses dans l'instance, et dans le nom du compteur.
        let nic =
            parse_path(r"\Network Interface(Intel(R) Ethernet I219-V)\Bytes Total/sec").unwrap();
        assert_eq!(nic.instance.as_deref(), Some("Intel(R) Ethernet I219-V"));
        let smtp = parse_path(r"\MDaemon\Active SMTP (in) sessions").unwrap();
        assert_eq!(smtp.counter, "Active SMTP (in) sessions");

        let remote = parse_path(r"\\MAIL01\MDaemon\Bad queue messages").unwrap();
        assert_eq!(remote.machine.as_deref(), Some("MAIL01"));
        assert_eq!(remote.object, "MDaemon");
    }

    #[test]
    fn a_malformed_path_is_refused_with_a_reason() {
        for bad in [
            "Processor\\% Processor Time",
            r"\Processor",
            r"\Processor(_Total\% Processor Time",
            r"\\MAIL01",
            r"\(x)\c",
            r"\Processor\",
        ] {
            assert!(parse_path(bad).is_err(), "{bad} should be refused");
        }
    }

    #[test]
    fn every_mdaemon_counter_is_a_valid_single_instance_path() {
        let specs = mdaemon_specs();
        assert!(specs.len() >= 40);
        for spec in &specs {
            let path = parse_path(&spec.path).expect(&spec.path);
            assert_eq!(path.object, MDAEMON_OBJECT);
            assert_eq!(path.instance, None, "{}", spec.path);
            assert!(spec.metric.starts_with("mdaemon_"), "{}", spec.metric);
        }
        // Deux compteurs dans la même série seraient écrasés l'un par l'autre.
        let mut keys: Vec<String> = specs
            .iter()
            .map(|s| {
                let mut sample = Sample::new(&s.metric, 0.0, s.kind, 0);
                for (k, v) in &s.labels {
                    sample = sample.with_label(k, v);
                }
                sample.series_key()
            })
            .collect();
        let total = keys.len();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), total, "two MDaemon counters share a series");
    }

    #[test]
    fn the_mdaemon_queues_the_rules_rely_on_are_mapped() {
        let specs = mdaemon_specs();
        let find = |counter: &str| {
            specs.iter().find(|s| s.path == format!(r"\MDaemon\{counter}")).cloned().unwrap()
        };
        for (counter, queue) in [
            ("Retry queue messages", "retry"),
            ("Bad queue messages", "bad"),
            ("Remote queue messages", "remote"),
            ("Local queue messages", "local"),
            ("Inbound queue messages", "inbound"),
            ("Holding queue messages", "holding"),
        ] {
            let spec = find(counter);
            assert_eq!(spec.metric, "mdaemon_queue_messages");
            assert_eq!(spec.labels, vec![("queue".to_string(), queue.to_string())]);
            assert_eq!(spec.kind, MetricKind::Gauge);
        }
        // Les totaux sont des compteurs : le taux se calcule à la lecture.
        assert_eq!(find("SMTP messages (in) total").kind, MetricKind::Counter);
        assert_eq!(find("spam refused total").metric, "mdaemon_filtered_messages_total");
        // Pas de « /sec » : ce sont des dérivées de PDH.
        assert!(specs.iter().all(|s| !s.path.ends_with("/sec")));
    }

    #[test]
    fn preset_modes_accept_the_agent_s_boolean_spellings_and_auto() {
        assert_eq!(PresetMode::parse("auto"), Some(PresetMode::Auto));
        assert_eq!(PresetMode::parse(" TRUE "), Some(PresetMode::On));
        assert_eq!(PresetMode::parse("no"), Some(PresetMode::Off));
        assert_eq!(PresetMode::parse("peut-être"), None);
    }

    /// Source factice : des chemins refusés, des valeurs fixées, un service
    /// présent ou non.
    #[derive(Default)]
    struct Fake {
        refused: Vec<String>,
        values: std::collections::BTreeMap<String, f64>,
        mdaemon_installed: bool,
        opened: Vec<String>,
        opens: usize,
    }

    impl CounterSource for Fake {
        fn open(&mut self, paths: &[String]) -> Vec<Result<(), String>> {
            self.opens += 1;
            self.opened = paths.to_vec();
            paths
                .iter()
                .map(
                    |p| {
                        if self.refused.contains(p) {
                            Err("no such counter".into())
                        } else {
                            Ok(())
                        }
                    },
                )
                .collect()
        }

        fn read(&mut self) -> Vec<Option<f64>> {
            self.opened
                .iter()
                .map(|p| if self.refused.contains(p) { None } else { self.values.get(p).copied() })
                .collect()
        }

        fn service_installed(&self, name: &str) -> bool {
            name == MDAEMON_SERVICE && self.mdaemon_installed
        }
    }

    fn value_of(samples: &[Sample], key: &str) -> Option<f64> {
        samples.iter().find(|s| s.series_key() == key).map(|s| s.value)
    }

    #[test]
    fn nothing_configured_and_no_mdaemon_means_no_reading_at_all() {
        let mut probe = PerfCountersProbe::new(&PerfCountersConfig::default(), Fake::default());
        assert_eq!(probe.read(), None);
        assert_eq!(probe.source.opens, 0, "no PDH query for nothing");
    }

    #[test]
    fn auto_mode_reads_the_mdaemon_preset_when_the_service_exists() {
        let mut fake = Fake { mdaemon_installed: true, ..Fake::default() };
        fake.values.insert(r"\MDaemon\Retry queue messages".into(), 12.0);
        fake.values.insert(r"\MDaemon\Bad queue messages".into(), 0.0);
        fake.values.insert(r"\MDaemon\SMTP messages (in) total".into(), 4_321.0);
        let mut probe = PerfCountersProbe::new(&PerfCountersConfig::default(), fake);
        let report = probe.read().expect("MDaemon detected");
        let samples = samples(&report, 1_000);

        assert_eq!(value_of(&samples, r#"mdaemon_queue_messages{queue="retry"}"#), Some(12.0));
        // Une file vide est une vraie valeur, publiée comme telle.
        assert_eq!(value_of(&samples, r#"mdaemon_queue_messages{queue="bad"}"#), Some(0.0));
        let total = samples.iter().find(|s| s.metric == "mdaemon_messages_total").unwrap();
        assert_eq!(total.kind, MetricKind::Counter);
        // Un compteur sans valeur ce cycle n'est pas publié à zéro.
        assert_eq!(value_of(&samples, r#"mdaemon_queue_messages{queue="remote"}"#), None);
        assert!(samples.iter().all(|s| s.ts_ms == 1_000));
    }

    #[test]
    fn auto_mode_stays_silent_without_the_service_and_off_wins_over_detection() {
        let mut probe = PerfCountersProbe::new(&PerfCountersConfig::default(), Fake::default());
        assert_eq!(probe.read(), None);

        let config = PerfCountersConfig { mdaemon: PresetMode::Off, ..Default::default() };
        let fake = Fake { mdaemon_installed: true, ..Fake::default() };
        let mut probe = PerfCountersProbe::new(&config, fake);
        assert_eq!(probe.read(), None);
    }

    #[test]
    fn custom_counters_are_published_under_their_name() {
        let config = PerfCountersConfig {
            counters: vec![
                CustomCounter {
                    path: r"\Memory\Available MBytes".into(),
                    name: "memory_available_mb".into(),
                },
                CustomCounter {
                    path: r"\Processor(_Total)\% Processor Time".into(),
                    name: r"\Processor(_Total)\% Processor Time".into(),
                },
            ],
            mdaemon: PresetMode::Off,
        };
        let mut fake = Fake::default();
        fake.values.insert(r"\Memory\Available MBytes".into(), 2_048.0);
        fake.values.insert(r"\Processor(_Total)\% Processor Time".into(), 142.0);
        let mut probe = PerfCountersProbe::new(&config, fake);
        let samples = samples(&probe.read().unwrap(), 0);
        assert_eq!(
            value_of(&samples, r#"agent_perf_counter{counter="memory_available_mb"}"#),
            Some(2_048.0)
        );
        // Au-delà de 100 % : pas d'écrêtage côté agent non plus.
        assert_eq!(
            value_of(
                &samples,
                r#"agent_perf_counter{counter="\Processor(_Total)\% Processor Time"}"#
            ),
            Some(142.0)
        );
    }

    #[test]
    fn a_refused_counter_does_not_stop_the_others_and_is_retried_later() {
        let config = PerfCountersConfig { mdaemon: PresetMode::On, ..Default::default() };
        let mut fake = Fake::default();
        fake.refused.push(r"\MDaemon\LAN queue messages".into());
        fake.values.insert(r"\MDaemon\Retry queue messages".into(), 3.0);
        let mut probe = PerfCountersProbe::new(&config, fake);

        let first = samples(&probe.read().unwrap(), 0);
        assert_eq!(value_of(&first, r#"mdaemon_queue_messages{queue="retry"}"#), Some(3.0));
        assert_eq!(value_of(&first, r#"mdaemon_queue_messages{queue="lan"}"#), None);
        assert_eq!(probe.source.opens, 1);

        // Les cycles suivants relisent la requête ouverte sans la rouvrir…
        for _ in 1..REOPEN_EVERY {
            probe.read();
        }
        assert_eq!(probe.source.opens, 1);
        // … et au dixième, le compteur refusé est retenté.
        probe.source.refused.clear();
        probe.source.values.insert(r"\MDaemon\LAN queue messages".into(), 1.0);
        let later = samples(&probe.read().unwrap(), 0);
        assert_eq!(probe.source.opens, 2);
        assert_eq!(value_of(&later, r#"mdaemon_queue_messages{queue="lan"}"#), Some(1.0));
        assert!(!probe.refused);
    }

    #[test]
    fn an_mdaemon_installed_later_is_picked_up_without_a_restart() {
        let mut probe = PerfCountersProbe::new(&PerfCountersConfig::default(), Fake::default());
        assert_eq!(probe.read(), None);
        probe.source.mdaemon_installed = true;
        probe.source.values.insert(r"\MDaemon\MDaemon running state".into(), 1.0);
        for _ in 1..REOPEN_EVERY {
            assert_eq!(probe.read(), None);
        }
        let samples = samples(&probe.read().expect("picked up"), 0);
        assert_eq!(value_of(&samples, "mdaemon_running"), Some(1.0));
    }

    #[test]
    fn the_custom_list_is_capped() {
        let counters: Vec<CustomCounter> = (0..MAX_CUSTOM_COUNTERS + 5)
            .map(|i| CustomCounter { path: format!(r"\Obj\C{i}"), name: format!("c{i}") })
            .collect();
        assert_eq!(custom_specs(&counters).len(), MAX_CUSTOM_COUNTERS);
    }

    #[test]
    fn off_windows_every_path_is_refused_with_an_explanation() {
        #[cfg(not(windows))]
        {
            let mut source = PlatformSource;
            let outcomes = source.open(&[r"\Memory\Available MBytes".to_string()]);
            assert!(outcomes[0].as_ref().unwrap_err().contains("only on Windows"));
            assert!(!source.service_installed(MDAEMON_SERVICE));
        }
    }
}
