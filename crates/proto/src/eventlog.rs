//! Journal d'événements Windows : règles poussées à l'agent, événements remontés.
//!
//! Le serveur garde, par machine, une liste de règles (journal, fournisseur,
//! identifiants d'événement, niveaux, motif). Il les joint à l'accusé de
//! réception de chaque lot d'un agent Windows ([`crate::PushAck::event_rules`]) ;
//! l'agent interroge le journal avec elles et joint à son lot suivant les
//! événements qui y répondent ([`crate::PushBatch::event_log`]).
//!
//! Les deux champs sont facultatifs : un agent antérieur ignore les règles, un
//! serveur antérieur n'en envoie pas et ignore les événements.

use serde::{Deserialize, Serialize};

/// Nombre maximal d'événements joints à un lot ; le serveur écarte le surplus.
pub const MAX_EVENT_LOG_ENTRIES: usize = 500;

/// Longueur maximale du message d'un événement, en caractères.
pub const MAX_EVENT_MESSAGE_CHARS: usize = 2_000;

/// Événements retenus par règle et par minute. Au-delà, l'agent les compte
/// comme supprimés ([`EventLogDropped`]) sans les envoyer : une attaque par
/// force brute (4625 en rafale) ne doit pas noyer le serveur ni les canaux.
pub const EVENT_RULE_MAX_PER_MINUTE: u32 = 10;

/// Nombre maximal d'identifiants ou de plages d'une règle. Le moteur XPath du
/// journal Windows refuse les requêtes trop longues.
pub const MAX_EVENT_ID_RANGES: usize = 20;

/// Niveau d'un événement, tel que l'Observateur d'événements l'affiche.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventLevel {
    Critical,
    Error,
    Warning,
    Information,
}

impl EventLevel {
    pub const ALL: [Self; 4] = [Self::Critical, Self::Error, Self::Warning, Self::Information];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Information => "information",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|level| level.as_str().eq_ignore_ascii_case(raw.trim()))
    }

    /// Valeurs de `System/Level` qui correspondent à ce niveau.
    ///
    /// `Information` couvre aussi 0 (« LogAlways ») : c'est le niveau de tous
    /// les événements d'audit du journal Sécurité, 4625 compris.
    pub fn windows_values(self) -> &'static [u8] {
        match self {
            Self::Critical => &[1],
            Self::Error => &[2],
            Self::Warning => &[3],
            Self::Information => &[0, 4],
        }
    }

    /// Le niveau d'une valeur de `System/Level` ; `None` pour « Verbose » (5)
    /// et au-delà.
    pub fn from_windows(level: u8) -> Option<Self> {
        match level {
            1 => Some(Self::Critical),
            2 => Some(Self::Error),
            3 => Some(Self::Warning),
            0 | 4 => Some(Self::Information),
            _ => None,
        }
    }
}

/// Un identifiant d'événement (`from == to`) ou une plage, bornes comprises.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventIdRange {
    pub from: u32,
    pub to: u32,
}

impl EventIdRange {
    pub fn contains(&self, id: u32) -> bool {
        self.from <= id && id <= self.to
    }
}

/// Lit une liste saisie à la main : « 4625, 4740, 7000-7099 ». Vide : tous
/// les identifiants.
pub fn parse_event_ids(spec: &str) -> Result<Vec<EventIdRange>, String> {
    let mut ranges = Vec::new();
    for part in spec.split([',', ';']).map(str::trim).filter(|p| !p.is_empty()) {
        let number = |raw: &str| {
            raw.trim()
                .parse::<u32>()
                .ok()
                .filter(|id| *id <= u32::from(u16::MAX))
                .ok_or_else(|| format!("\"{part}\" is not an event ID (0 to 65535)."))
        };
        let range = match part.split_once('-') {
            Some((from, to)) => {
                let (from, to) = (number(from)?, number(to)?);
                if from > to {
                    return Err(format!("\"{part}\": the range must go upwards."));
                }
                EventIdRange { from, to }
            }
            None => {
                let id = number(part)?;
                EventIdRange { from: id, to: id }
            }
        };
        ranges.push(range);
    }
    if ranges.len() > MAX_EVENT_ID_RANGES {
        return Err(format!("At most {MAX_EVENT_ID_RANGES} event IDs or ranges per rule."));
    }
    Ok(ranges)
}

/// Forme canonique d'une liste, celle que l'interface réaffiche.
pub fn format_event_ids(ranges: &[EventIdRange]) -> String {
    ranges
        .iter()
        .map(|range| {
            if range.from == range.to {
                range.from.to_string()
            } else {
                format!("{}-{}", range.from, range.to)
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Une règle telle que l'agent l'applique.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventLogRule {
    /// Identifiant de la règle sur le serveur, rendu avec chaque événement.
    pub id: i64,
    /// Journal interrogé : `System`, `Application`, `Security`, ou un canal
    /// (`Microsoft-Windows-TaskScheduler/Operational`).
    pub channel: String,
    /// Fournisseur (`Microsoft-Windows-Kernel-Power`) ; absent : tous.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Vide : tous les identifiants.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub event_ids: Vec<EventIdRange>,
    /// Vide : tous les niveaux.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub levels: Vec<EventLevel>,
    /// Motif cherché dans le message : sous-chaîne sans distinction de casse,
    /// ou expression régulière si `regex`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(default)]
    pub regex: bool,
}

/// Un événement qui a répondu à une règle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventLogEntry {
    pub rule_id: i64,
    pub channel: String,
    #[serde(default)]
    pub provider: String,
    pub event_id: u32,
    /// Valeur brute de `System/Level` (0 à 5).
    #[serde(default)]
    pub level: u8,
    /// `EventRecordID`, croissant dans un même journal.
    #[serde(default)]
    pub record_id: u64,
    /// `TimeCreated`, en millisecondes depuis l'époque Unix.
    pub time_created_ms: i64,
    /// Nom de la machine tel que l'événement l'écrit.
    #[serde(default)]
    pub computer: String,
    /// Message mis en forme, tronqué à [`MAX_EVENT_MESSAGE_CHARS`].
    #[serde(default)]
    pub message: String,
}

/// Événements d'une règle écartés par la limite de débit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventLogDropped {
    pub rule_id: i64,
    pub count: u64,
}

/// Ce qu'un lot rapporte du journal d'événements.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EventLogReport {
    #[serde(default)]
    pub events: Vec<EventLogEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dropped: Vec<EventLogDropped>,
}

impl EventLogReport {
    pub fn is_empty(&self) -> bool {
        self.events.is_empty() && self.dropped.is_empty()
    }
}

/// Tronque un texte à `max` caractères, sur une frontière de caractère, en
/// signalant la coupe.
pub fn truncate_message(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        Some((end, _)) => format!("{}…", text[..end].trim_end()),
        None => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn une_liste_d_identifiants_se_lit_et_se_reecrit() {
        let ranges = parse_event_ids(" 4625,4740 ; 7000 - 7099 ").unwrap();
        assert_eq!(
            ranges,
            [
                EventIdRange { from: 4625, to: 4625 },
                EventIdRange { from: 4740, to: 4740 },
                EventIdRange { from: 7000, to: 7099 },
            ]
        );
        assert_eq!(format_event_ids(&ranges), "4625, 4740, 7000-7099");
        assert!(ranges[2].contains(7034));
        assert!(!ranges[2].contains(7100));
        assert!(parse_event_ids("").unwrap().is_empty());
    }

    #[test]
    fn une_liste_invalide_est_refusee_avec_un_message() {
        assert!(parse_event_ids("abc").is_err());
        assert!(parse_event_ids("7099-7000").is_err());
        assert!(parse_event_ids("70000").is_err());
        let many = (1..=MAX_EVENT_ID_RANGES as u32 + 1).map(|i| i.to_string()).collect::<Vec<_>>();
        assert!(parse_event_ids(&many.join(",")).is_err());
    }

    #[test]
    fn les_niveaux_windows_se_traduisent_dans_les_deux_sens() {
        assert_eq!(EventLevel::from_windows(0), Some(EventLevel::Information));
        assert_eq!(EventLevel::from_windows(2), Some(EventLevel::Error));
        assert_eq!(EventLevel::from_windows(5), None);
        assert_eq!(EventLevel::Information.windows_values(), [0, 4]);
        assert_eq!(EventLevel::parse(" Warning "), Some(EventLevel::Warning));
        assert_eq!(EventLevel::parse("verbose"), None);
    }

    #[test]
    fn un_message_long_est_tronque_sur_un_caractere() {
        assert_eq!(truncate_message("éé", 5), "éé");
        assert_eq!(truncate_message("ééé", 2), "éé…");
    }

    #[test]
    fn une_regle_minimale_se_relit() {
        let rule: EventLogRule =
            serde_json::from_str(r#"{"id":3,"channel":"System"}"#).expect("règle");
        assert!(rule.event_ids.is_empty() && rule.levels.is_empty() && !rule.regex);
        let json = serde_json::to_string(&rule).unwrap();
        assert_eq!(json, r#"{"id":3,"channel":"System","regex":false}"#);
    }
}
