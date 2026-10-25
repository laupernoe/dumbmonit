//! Calendriers iCalendar (RFC 5545) : les maintenances annoncées, lisibles par
//! Google Agenda, Outlook, Thunderbird ou n'importe quel client CalDAV en
//! abonnement.
//!
//! Le module ne connaît ni la base ni HTTP : il reçoit des événements déjà
//! choisis et produit le texte. Les règles qui comptent sont celles du format :
//! lignes terminées par CRLF, repliées à 75 octets sans couper un caractère,
//! texte échappé (`\`, `;`, `,`, retour à la ligne), dates en UTC.

use chrono::{DateTime, TimeDelta, Utc};

/// Durée d'un événement dont la fin n'est pas connue.
const DEFAULT_LENGTH: TimeDelta = TimeDelta::hours(1);

/// Un événement du calendrier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// Identifiant stable : l'agenda met à jour l'événement au lieu d'en créer un.
    pub uid: String,
    pub summary: String,
    pub description: String,
    pub start: DateTime<Utc>,
    pub end: Option<DateTime<Utc>>,
    /// Dernière modification, pour `LAST-MODIFIED` et `DTSTAMP`.
    pub modified: DateTime<Utc>,
    /// `scheduled`, `in_progress`, `completed` : repris en catégorie.
    pub status: String,
}

fn stamp(at: DateTime<Utc>) -> String {
    at.format("%Y%m%dT%H%M%SZ").to_string()
}

/// Échappe une valeur de type TEXT.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            ';' => out.push_str("\\;"),
            ',' => out.push_str("\\,"),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            _ => out.push(ch),
        }
    }
    out
}

/// Replie une ligne à 75 octets au plus : la suite commence par une espace.
/// Le découpage tombe toujours sur une frontière de caractère.
pub fn fold(line: &str) -> String {
    const LIMIT: usize = 75;
    let mut out = String::with_capacity(line.len() + line.len() / LIMIT * 3);
    let mut used = 0;
    for ch in line.chars() {
        let width = ch.len_utf8();
        if used + width > LIMIT {
            out.push_str("\r\n ");
            // La espace de continuation compte dans les 75 octets de la ligne suivante.
            used = 1;
        }
        out.push(ch);
        used += width;
    }
    out
}

/// Le calendrier complet. `name` : titre affiché par l'agenda.
pub fn calendar(name: &str, events: &[Event]) -> String {
    let mut lines = vec![
        "BEGIN:VCALENDAR".to_string(),
        "VERSION:2.0".to_string(),
        "PRODID:-//DumbMonit//Maintenance//EN".to_string(),
        "CALSCALE:GREGORIAN".to_string(),
        "METHOD:PUBLISH".to_string(),
        format!("X-WR-CALNAME:{}", escape(name)),
        // Les clients qui lisent ces propriétés rafraîchissent d'eux-mêmes.
        "REFRESH-INTERVAL;VALUE=DURATION:PT1H".to_string(),
        "X-PUBLISHED-TTL:PT1H".to_string(),
    ];
    for event in events {
        lines.push("BEGIN:VEVENT".to_string());
        lines.push(format!("UID:{}", event.uid));
        lines.push(format!("DTSTAMP:{}", stamp(event.modified)));
        lines.push(format!("LAST-MODIFIED:{}", stamp(event.modified)));
        lines.push(format!("DTSTART:{}", stamp(event.start)));
        // Une fin absente ou antérieure au début donne une heure : un événement
        // de durée négative est refusé par plusieurs clients.
        let end =
            event.end.filter(|end| *end > event.start).unwrap_or(event.start + DEFAULT_LENGTH);
        lines.push(format!("DTEND:{}", stamp(end)));
        lines.push(format!("SUMMARY:{}", escape(&event.summary)));
        if !event.description.is_empty() {
            lines.push(format!("DESCRIPTION:{}", escape(&event.description)));
        }
        lines.push("STATUS:CONFIRMED".to_string());
        lines.push("TRANSP:OPAQUE".to_string());
        lines.push(format!("CATEGORIES:MAINTENANCE,{}", event.status.to_ascii_uppercase()));
        lines.push("END:VEVENT".to_string());
    }
    lines.push("END:VCALENDAR".to_string());
    let mut out = String::new();
    for line in lines {
        out.push_str(&fold(&line));
        out.push_str("\r\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 3, h, 30, 0).unwrap()
    }

    fn event() -> Event {
        Event {
            uid: "maintenance-7@dumbmonit".into(),
            summary: "Switch upgrade, rack 2".into(),
            description: "Line one\nLine two\\; with a semicolon".into(),
            start: at(22),
            end: Some(at(23)),
            modified: at(10),
            status: "scheduled".into(),
        }
    }

    #[test]
    fn text_is_escaped() {
        assert_eq!(escape("a,b;c\\d\ne"), "a\\,b\\;c\\\\d\\ne");
    }

    #[test]
    fn long_lines_fold_at_75_octets_on_character_boundaries() {
        let line = format!("SUMMARY:{}", "é".repeat(60));
        let folded = fold(&line);
        for part in folded.split("\r\n") {
            assert!(part.len() <= 75, "{} octets : {part}", part.len());
        }
        // Rien n'est perdu au dépliage.
        assert_eq!(folded.replace("\r\n ", ""), line);
    }

    #[test]
    fn a_calendar_is_well_formed() {
        let text = calendar("Acme status", &[event()]);
        assert!(text.starts_with("BEGIN:VCALENDAR\r\n"));
        assert!(text.ends_with("END:VCALENDAR\r\n"));
        assert!(text.contains("X-WR-CALNAME:Acme status\r\n"));
        assert!(text.contains("UID:maintenance-7@dumbmonit\r\n"));
        assert!(text.contains("DTSTART:20261103T223000Z\r\n"));
        assert!(text.contains("DTEND:20261103T233000Z\r\n"));
        assert!(text.contains("SUMMARY:Switch upgrade\\, rack 2\r\n"));
        assert!(text.contains("DESCRIPTION:Line one\\nLine two\\; with a semicolon\r\n"));
        assert!(text.contains("CATEGORIES:MAINTENANCE,SCHEDULED\r\n"));
        assert_eq!(text.matches("BEGIN:VEVENT").count(), text.matches("END:VEVENT").count());
        assert_eq!(text.matches('\n').count(), text.matches("\r\n").count(), "CRLF only");
    }

    #[test]
    fn a_missing_or_inverted_end_gives_one_hour() {
        let mut open = event();
        open.end = None;
        assert!(calendar("x", &[open]).contains("DTEND:20261103T233000Z"));
        let mut inverted = event();
        inverted.end = Some(at(21));
        assert!(calendar("x", &[inverted]).contains("DTEND:20261103T233000Z"));
    }

    #[test]
    fn an_empty_calendar_is_still_a_calendar() {
        let text = calendar("Nothing planned", &[]);
        assert!(text.contains("BEGIN:VCALENDAR") && !text.contains("VEVENT"));
    }
}
