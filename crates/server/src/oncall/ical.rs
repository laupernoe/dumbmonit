//! Export iCalendar (RFC 5545) des gardes d'une personne.
//!
//! Un abonnement de calendrier (Google, Apple, Thunderbird) relit l'URL de
//! temps en temps : chaque garde est un événement daté en UTC, avec un
//! identifiant stable tant que la garde ne bouge pas, pour que le calendrier
//! la mette à jour au lieu de la dupliquer.

use chrono::{DateTime, Utc};

use crate::oncall::schedule::Shift;

fn stamp(at: DateTime<Utc>) -> String {
    at.format("%Y%m%dT%H%M%SZ").to_string()
}

/// Échappe un texte selon la RFC 5545 (§ 3.3.11).
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            ';' => out.push_str("\\;"),
            ',' => out.push_str("\\,"),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            other => out.push(other),
        }
    }
    out
}

/// Plie une ligne à 75 octets, sans couper un caractère (§ 3.1).
fn fold(line: &str, out: &mut String) {
    let mut width = 0;
    for c in line.chars() {
        let len = c.len_utf8();
        if width + len > 75 {
            out.push_str("\r\n ");
            width = 1;
        }
        out.push(c);
        width += len;
    }
    out.push_str("\r\n");
}

/// Une garde à exporter : le planning et la garde.
pub struct Entry<'a> {
    pub schedule_id: i64,
    pub schedule_name: &'a str,
    pub shift: Shift,
}

/// Calendrier complet des gardes de `person`.
pub fn render(person: &str, entries: &[Entry<'_>], now: DateTime<Utc>) -> String {
    let mut out = String::new();
    let calendar_name = format!("On call · {person}");
    for line in [
        "BEGIN:VCALENDAR".to_string(),
        "VERSION:2.0".to_string(),
        "PRODID:-//DumbMonit//On-call//EN".to_string(),
        "CALSCALE:GREGORIAN".to_string(),
        "METHOD:PUBLISH".to_string(),
        format!("X-WR-CALNAME:{}", escape(&calendar_name)),
        "REFRESH-INTERVAL;VALUE=DURATION:PT1H".to_string(),
    ] {
        fold(&line, &mut out);
    }
    for entry in entries {
        let shift = &entry.shift;
        let uid = format!(
            "oncall-{}-{}-{}-{}@dumbmonit",
            entry.schedule_id,
            shift.user_id,
            shift.start.timestamp(),
            shift.end.timestamp()
        );
        let summary = if shift.override_id.is_some() {
            format!("On call (cover): {}", entry.schedule_name)
        } else {
            format!("On call: {}", entry.schedule_name)
        };
        for line in [
            "BEGIN:VEVENT".to_string(),
            format!("UID:{uid}"),
            format!("DTSTAMP:{}", stamp(now)),
            format!("DTSTART:{}", stamp(shift.start)),
            format!("DTEND:{}", stamp(shift.end)),
            format!("SUMMARY:{}", escape(&summary)),
            "TRANSP:OPAQUE".to_string(),
            "END:VEVENT".to_string(),
        ] {
            fold(&line, &mut out);
        }
    }
    fold("END:VCALENDAR", &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(raw: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(raw).expect("instant").with_timezone(&Utc)
    }

    #[test]
    fn chaque_garde_devient_un_evenement_echappe() {
        let shift = Shift {
            start: utc("2026-10-26T08:00:00Z"),
            end: utc("2026-11-02T08:00:00Z"),
            user_id: 3,
            layer: Some(0),
            override_id: None,
        };
        let entries = [Entry { schedule_id: 7, schedule_name: "Ops; nights, weekends", shift }];
        let ics = render("alice", &entries, utc("2026-10-25T10:00:00Z"));
        assert!(ics.starts_with("BEGIN:VCALENDAR\r\n"));
        assert!(ics.ends_with("END:VCALENDAR\r\n"));
        assert!(ics.contains("DTSTART:20261026T080000Z\r\n"));
        assert!(ics.contains("DTEND:20261102T080000Z\r\n"));
        assert!(ics.contains("SUMMARY:On call: Ops\\; nights\\, weekends\r\n"));
        assert!(ics.contains("UID:oncall-7-3-"));
    }

    #[test]
    fn une_ligne_longue_est_pliee_sans_couper_un_caractere() {
        let mut out = String::new();
        let long = format!("SUMMARY:{}", "é".repeat(60));
        fold(&long, &mut out);
        for line in out.split("\r\n").filter(|l| !l.is_empty()) {
            assert!(line.len() <= 75, "{} bytes", line.len());
        }
        assert_eq!(out.replace("\r\n ", "").trim_end(), long);
    }
}
