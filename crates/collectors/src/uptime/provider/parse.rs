//! Lecture de ce que publie un fournisseur : le JSON d'une page Atlassian
//! Statuspage (`/api/v2/summary.json`) ou un flux RSS/Atom d'incidents (AWS
//! Health, `history.rss` d'une Statuspage, tout flux de ce genre).
//!
//! Ce module ne fait aucune entrée-sortie : il reçoit le texte et rend un
//! [`Reading`], ce qui le rend testable avec les réponses réelles collées en
//! chaîne.

use chrono::{DateTime, TimeDelta, Utc};
use serde::Deserialize;

/// Gravité d'un état de fournisseur, dans l'ordre : `None < Minor < Major < Critical`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Impact {
    None = 0,
    Minor = 1,
    Major = 2,
    Critical = 3,
}

impl Impact {
    /// Le niveau à partir duquel la sonde échoue (`fail_on`).
    pub fn parse_level(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "minor" => Some(Self::Minor),
            "major" => Some(Self::Major),
            "critical" => Some(Self::Critical),
            _ => None,
        }
    }

    /// `status.indicator` d'une Statuspage. Un mot inconnu ne fait pas échouer :
    /// mieux vaut ne rien dire qu'une fausse alerte.
    fn from_indicator(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "minor" => Self::Minor,
            "major" => Self::Major,
            "critical" => Self::Critical,
            _ => Self::None,
        }
    }

    /// `components[].status` d'une Statuspage. Une maintenance annoncée n'est pas
    /// une panne.
    fn from_component(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "degraded_performance" => Self::Minor,
            "partial_outage" => Self::Major,
            "major_outage" => Self::Critical,
            _ => Self::None,
        }
    }
}

/// Ce qu'on retient d'une réponse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reading {
    pub impact: Impact,
    /// Incidents non résolus.
    pub incidents: u32,
    /// Composants touchés (Statuspage seulement).
    pub affected: u32,
    /// Phrase du fournisseur ou titre de l'entrée : pour le journal.
    pub summary: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ReadError {
    /// La réponse n'est pas ce qu'on attendait (page HTML, JSON étranger).
    Format(String),
    /// Le filtre `component` ne retient rien : erreur de configuration.
    NoMatch(String),
}

#[derive(Deserialize)]
struct Summary {
    status: PageStatus,
    #[serde(default)]
    components: Vec<Component>,
    #[serde(default)]
    incidents: Vec<Incident>,
}

#[derive(Deserialize)]
struct PageStatus {
    indicator: String,
    #[serde(default)]
    description: String,
}

#[derive(Deserialize)]
struct Component {
    name: String,
    #[serde(default)]
    status: String,
    /// Un groupe de composants : son état n'est que celui de ses membres.
    #[serde(default)]
    group: bool,
}

#[derive(Deserialize)]
struct Incident {
    #[serde(default)]
    status: String,
}

fn contains_ci(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

/// Lit le JSON d'une Statuspage. Avec `component`, seuls les composants dont le
/// nom contient ce texte comptent ; sans, l'indicateur global de la page.
pub fn read_statuspage(body: &str, component: Option<&str>) -> Result<Reading, ReadError> {
    let summary: Summary = serde_json::from_str(body).map_err(|_| {
        ReadError::Format("not an Atlassian Statuspage summary (/api/v2/summary.json)".into())
    })?;
    let incidents = summary
        .incidents
        .iter()
        .filter(|incident| !matches!(incident.status.as_str(), "resolved" | "postmortem" | "completed"))
        .count() as u32;

    let (impact, affected) = match component {
        None => {
            let affected = summary
                .components
                .iter()
                .filter(|c| !c.group && Impact::from_component(&c.status) > Impact::None)
                .count() as u32;
            (Impact::from_indicator(&summary.status.indicator), affected)
        }
        Some(filter) => {
            let matching: Vec<&Component> = summary
                .components
                .iter()
                .filter(|c| !c.group && contains_ci(&c.name, filter))
                .collect();
            if matching.is_empty() {
                return Err(ReadError::NoMatch(format!(
                    "the provider lists no component whose name contains \"{filter}\""
                )));
            }
            let worst = matching.iter().map(|c| Impact::from_component(&c.status)).max();
            let affected = matching.iter().filter(|c| Impact::from_component(&c.status) > Impact::None).count();
            (worst.unwrap_or(Impact::None), affected as u32)
        }
    };
    Ok(Reading { impact, incidents, affected, summary: summary.status.description })
}

/// Une entrée de flux.
struct Entry {
    title: String,
    text: String,
    at: DateTime<Utc>,
}

fn child_text<'a>(node: roxmltree::Node<'a, '_>, names: &[&str]) -> Option<&'a str> {
    node.children()
        .find(|child| child.is_element() && names.contains(&child.tag_name().name()))
        .and_then(|child| child.text())
        .map(str::trim)
}

fn parse_date(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc2822(raw)
        .or_else(|_| DateTime::parse_from_rfc3339(raw))
        .ok()
        .map(|at| at.with_timezone(&Utc))
}

fn entries(body: &str) -> Result<Vec<Entry>, ReadError> {
    let doc = roxmltree::Document::parse(body)
        .map_err(|_| ReadError::Format("not an RSS or Atom feed".into()))?;
    let mut out = Vec::new();
    for node in doc.descendants().filter(|n| n.is_element()) {
        if !matches!(node.tag_name().name(), "item" | "entry") {
            continue;
        }
        let title = child_text(node, &["title"]).unwrap_or_default().to_string();
        let text = child_text(node, &["description", "summary", "content"]).unwrap_or_default();
        let Some(at) = child_text(node, &["pubDate", "updated", "published", "date"]).and_then(parse_date)
        else {
            continue;
        };
        out.push(Entry { title, text: text.to_string(), at });
    }
    Ok(out)
}

/// Lit un flux d'incidents. Seule l'entrée la plus récente de la fenêtre
/// compte : tant qu'elle n'est pas « résolue » (ou « operating normally » pour
/// AWS), un incident est ouvert. Un flux qui mêle plusieurs services se limite
/// avec `component`.
pub fn read_feed(
    body: &str,
    component: Option<&str>,
    now: DateTime<Utc>,
    window: TimeDelta,
) -> Result<Reading, ReadError> {
    let mut found = entries(body)?;
    if let Some(filter) = component {
        found.retain(|e| contains_ci(&e.title, filter) || contains_ci(&e.text, filter));
    }
    // Une date dans le futur proche est une horloge qui diffère, pas un événement à venir.
    found.retain(|e| now - e.at <= window && e.at - now <= TimeDelta::hours(1));
    found.sort_by_key(|e| std::cmp::Reverse(e.at));

    let Some(newest) = found.first() else {
        return Ok(Reading { impact: Impact::None, incidents: 0, affected: 0, summary: String::new() });
    };
    let text = format!("{} {}", newest.title, newest.text).to_lowercase();
    let resolved = ["resolved", "operating normally", "completed"].iter().any(|w| text.contains(w));
    if resolved {
        return Ok(Reading {
            impact: Impact::None,
            incidents: 0,
            affected: 0,
            summary: newest.title.clone(),
        });
    }
    let serious = ["disruption", "outage", "unavailable"].iter().any(|w| text.contains(w));
    Ok(Reading {
        impact: if serious { Impact::Major } else { Impact::Minor },
        incidents: 1,
        affected: 0,
        summary: newest.title.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    const SUMMARY: &str = r#"{
      "page": {"id": "x", "name": "Cloudflare", "url": "https://www.cloudflarestatus.com"},
      "status": {"indicator": "minor", "description": "Minor Service Outage"},
      "components": [
        {"name": "Cloudflare Sites and Services", "status": "operational", "group": true},
        {"name": "Workers", "status": "degraded_performance", "group": false},
        {"name": "Argo", "status": "operational"},
        {"name": "Zero Trust", "status": "under_maintenance"}
      ],
      "incidents": [
        {"name": "Elevated errors", "status": "investigating", "impact": "minor"},
        {"name": "Old one", "status": "resolved", "impact": "minor"}
      ],
      "scheduled_maintenances": []
    }"#;

    #[test]
    fn the_page_indicator_decides_without_a_filter() {
        let reading = read_statuspage(SUMMARY, None).unwrap();
        assert_eq!(reading.impact, Impact::Minor);
        assert_eq!(reading.incidents, 1, "a resolved incident is not open");
        assert_eq!(reading.affected, 1);
        assert_eq!(reading.summary, "Minor Service Outage");
    }

    #[test]
    fn a_component_filter_watches_that_part_only() {
        let workers = read_statuspage(SUMMARY, Some("workers")).unwrap();
        assert_eq!(workers.impact, Impact::Minor);
        let argo = read_statuspage(SUMMARY, Some("Argo")).unwrap();
        assert_eq!(argo.impact, Impact::None, "the page is degraded, Argo is not");
        // Une maintenance annoncée n'est pas une panne.
        assert_eq!(read_statuspage(SUMMARY, Some("zero trust")).unwrap().impact, Impact::None);
        assert!(matches!(read_statuspage(SUMMARY, Some("nope")), Err(ReadError::NoMatch(_))));
    }

    #[test]
    fn indicators_and_component_states_map_to_levels() {
        for (raw, expected) in [
            ("none", Impact::None),
            ("minor", Impact::Minor),
            ("major", Impact::Major),
            ("critical", Impact::Critical),
            ("maintenance", Impact::None),
            ("???", Impact::None),
        ] {
            assert_eq!(Impact::from_indicator(raw), expected, "{raw}");
        }
        assert_eq!(Impact::from_component("partial_outage"), Impact::Major);
        assert_eq!(Impact::from_component("major_outage"), Impact::Critical);
        assert!(Impact::Critical > Impact::Major && Impact::Major > Impact::Minor);
        assert_eq!(Impact::parse_level(" Major "), Some(Impact::Major));
        assert_eq!(Impact::parse_level("none"), None);
    }

    #[test]
    fn html_instead_of_json_is_a_format_error() {
        assert!(matches!(read_statuspage("<html></html>", None), Err(ReadError::Format(_))));
        assert!(matches!(read_statuspage(r#"{"hello": 1}"#, None), Err(ReadError::Format(_))));
    }

    const AWS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
    <rss version="2.0"><channel><title>AWS Service Status</title>
      <item><title>Service impact: Increased API Error Rates</title>
        <pubDate>Tue, 03 Nov 2026 10:00:00 PST</pubDate>
        <description>EC2 in us-east-1 is seeing a service disruption.</description></item>
      <item><title>Service is operating normally: [RESOLVED] Lambda latency</title>
        <pubDate>Mon, 02 Nov 2026 08:00:00 GMT</pubDate>
        <description>Lambda recovered.</description></item>
    </channel></rss>"#;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 3, 19, 0, 0).unwrap()
    }

    #[test]
    fn an_open_feed_entry_is_an_incident() {
        let reading = read_feed(AWS, None, now(), TimeDelta::hours(24)).unwrap();
        assert_eq!(reading.impact, Impact::Major, "\"disruption\" is serious");
        assert_eq!(reading.incidents, 1);
        assert!(reading.summary.contains("API Error Rates"));
    }

    #[test]
    fn old_or_resolved_entries_do_not_count() {
        // Hors fenêtre.
        let later = now() + TimeDelta::days(3);
        assert_eq!(read_feed(AWS, None, later, TimeDelta::hours(24)).unwrap().impact, Impact::None);
        // Filtre sur le service résolu.
        let lambda = read_feed(AWS, Some("lambda"), now() - TimeDelta::hours(5), TimeDelta::days(2)).unwrap();
        assert_eq!(lambda.impact, Impact::None);
        // Aucune entrée ne correspond.
        assert_eq!(read_feed(AWS, Some("rds"), now(), TimeDelta::days(2)).unwrap().incidents, 0);
    }

    #[test]
    fn atom_feeds_are_read_too() {
        let atom = r#"<feed xmlns="http://www.w3.org/2005/Atom"><title>Status</title>
          <entry><title>Degraded performance</title><updated>2026-11-03T18:30:00Z</updated>
          <summary>Investigating</summary></entry></feed>"#;
        let reading = read_feed(atom, None, now(), TimeDelta::hours(24)).unwrap();
        assert_eq!((reading.impact, reading.incidents), (Impact::Minor, 1));
    }

    #[test]
    fn a_page_that_is_not_a_feed_is_a_format_error() {
        assert!(matches!(read_feed("{}", None, now(), TimeDelta::hours(1)), Err(ReadError::Format(_))));
    }
}
