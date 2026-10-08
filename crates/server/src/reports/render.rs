//! Rendu d'un rapport en courriel : HTML sobre (tableaux, styles en ligne, aucune
//! image ni ressource externe) et version texte. Tout est en anglais.
//!
//! Règle du module : aucune donnée n'entre dans le HTML sans passer par
//! [`esc`]. Noms d'équipements, de dossiers, de règles et du rapport sont saisis
//! par des utilisateurs (ou, pour les noms d'appareils, parfois découverts sur le
//! réseau) ; ils ne doivent jamais pouvoir injecter de balise.

use std::fmt::Write as _;

use chrono::{DateTime, TimeDelta, Utc};
use chrono_tz::Tz;

use crate::reports::collect::{
    DeviceStat, MAX_DEVICE_ROWS, MAX_INCIDENT_ROWS, ReportData, UNGROUPED,
};
use crate::reports::schedule::Frequency;

pub struct Rendered {
    pub subject: String,
    pub html: String,
    pub text: String,
}

/// Échappement HTML des cinq caractères sensibles.
pub fn esc(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for c in raw.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c if c.is_control() && c != '\n' => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

/// Texte d'une seule ligne, sans caractère de contrôle (objets d'e-mail, texte brut).
fn one_line(raw: &str) -> String {
    raw.chars().map(|c| if c.is_control() { ' ' } else { c }).collect::<String>().trim().to_string()
}

fn pct(value: Option<f64>) -> String {
    match value {
        Some(v) => format!("{:.2}%", v.clamp(0.0, 100.0)),
        None => "n/a".to_string(),
    }
}

/// Écart en points de pourcentage, ou « n/a ».
fn delta(now: Option<f64>, before: Option<f64>) -> String {
    match (now, before) {
        (Some(a), Some(b)) => {
            let d = a - b;
            if d.abs() < 0.005 {
                "no change".to_string()
            } else {
                format!("{}{:.2} pt", if d > 0.0 { "+" } else { "-" }, d.abs())
            }
        }
        _ => "n/a".to_string(),
    }
}

fn count_delta(now: usize, before: usize) -> String {
    match now.cmp(&before) {
        std::cmp::Ordering::Equal => "same as the previous period".to_string(),
        std::cmp::Ordering::Greater => format!("{} more than the previous period", now - before),
        std::cmp::Ordering::Less => format!("{} fewer than the previous period", before - now),
    }
}

/// Mot qui accompagne un pourcentage : l'état n'est jamais porté par la couleur seule.
fn verdict(value: Option<f64>) -> (&'static str, &'static str) {
    match value {
        None => ("No data", "#6b7280"),
        Some(v) if v >= 99.9 => ("Healthy", "#15803d"),
        Some(v) if v >= 99.0 => ("Degraded", "#b45309"),
        Some(_) => ("Poor", "#b91c1c"),
    }
}

fn duration(delta: TimeDelta) -> String {
    let secs = delta.num_seconds().max(0);
    if secs < 60 {
        return format!("{secs}s");
    }
    let (days, hours, minutes) = (secs / 86_400, secs % 86_400 / 3_600, secs % 3_600 / 60);
    match (days, hours) {
        (0, 0) => format!("{minutes}m"),
        (0, _) => format!("{hours}h {minutes:02}m"),
        _ => format!("{days}d {hours}h"),
    }
}

fn when(at: DateTime<Utc>, tz: Tz) -> String {
    at.with_timezone(&tz).format("%Y-%m-%d %H:%M").to_string()
}

fn range_label(data: &ReportData, tz: Tz) -> String {
    format!(
        "{} to {} ({})",
        when(data.period.start, tz),
        when(data.period.end, tz),
        data.period.end.with_timezone(&tz).format("%Z")
    )
}

fn frequency_word(frequency: Frequency) -> &'static str {
    match frequency {
        Frequency::Daily => "Daily",
        Frequency::Weekly => "Weekly",
        Frequency::Monthly => "Monthly",
    }
}

pub fn render(data: &ReportData, tz: Tz, preview: bool) -> Rendered {
    let incidents = data.incidents.len();
    let subject = format!(
        "{}[DumbMonit] {} report: {} availability, {} incident{}",
        if preview { "[Preview] " } else { "" },
        frequency_word(data.frequency),
        pct(data.fleet_uptime),
        incidents,
        if incidents == 1 { "" } else { "s" }
    );
    Rendered { subject, html: html(data, tz, preview), text: text(data, tz, preview) }
}

// --------------------------------------------------------------------------
// HTML
// --------------------------------------------------------------------------

const FONT: &str = "-apple-system,BlinkMacSystemFont,'Segoe UI',Helvetica,Arial,sans-serif";
const TH: &str = "padding:6px 8px;border-bottom:2px solid #d1d5db;text-align:left;font-size:12px;\
                  color:#4b5563;text-transform:uppercase;letter-spacing:0.04em";
const TD: &str = "padding:6px 8px;border-bottom:1px solid #e5e7eb;font-size:14px;color:#111827";

fn heading(out: &mut String, title: &str) {
    let _ = write!(
        out,
        "<tr><td style=\"padding:24px 24px 8px 24px;font-family:{FONT};font-size:16px;\
         font-weight:700;color:#111827\">{}</td></tr>",
        esc(title)
    );
}

fn table_open(out: &mut String, headers: &[&str]) {
    out.push_str(
        "<tr><td style=\"padding:0 24px\"><table role=\"presentation\" width=\"100%\" \
         cellpadding=\"0\" cellspacing=\"0\" style=\"border-collapse:collapse\"><tr>",
    );
    for header in headers {
        let _ = write!(out, "<th style=\"{TH}\" align=\"left\">{}</th>", esc(header));
    }
    out.push_str("</tr>");
}

fn table_close(out: &mut String) {
    out.push_str("</table></td></tr>");
}

fn cell(out: &mut String, content: &str) {
    let _ = write!(out, "<td style=\"{TD};font-family:{FONT}\">{content}</td>");
}

fn note(out: &mut String, message: &str) {
    let _ = write!(
        out,
        "<tr><td style=\"padding:8px 24px;font-family:{FONT};font-size:13px;color:#4b5563\">{}</td></tr>",
        esc(message)
    );
}

fn stat_cell(label: &str, value: &str, sub: &str, color: &str) -> String {
    format!(
        "<td valign=\"top\" style=\"padding:12px;border:1px solid #e5e7eb;font-family:{FONT}\">\
         <div style=\"font-size:12px;color:#4b5563;text-transform:uppercase;letter-spacing:0.04em\">{}</div>\
         <div style=\"font-size:24px;font-weight:700;color:{color};padding:2px 0\">{}</div>\
         <div style=\"font-size:12px;color:#4b5563\">{}</div></td>",
        esc(label),
        esc(value),
        esc(sub)
    )
}

fn uptime_cell(value: Option<f64>) -> String {
    let (word, color) = verdict(value);
    format!(
        "<span style=\"font-weight:700\">{}</span> <span style=\"color:{color};font-size:12px\">{}</span>",
        esc(&pct(value)),
        word
    )
}

fn html(data: &ReportData, tz: Tz, preview: bool) -> String {
    let mut out = String::with_capacity(8 * 1024);
    let (word, color) = verdict(data.fleet_uptime);
    let title = format!("{} report", frequency_word(data.frequency));
    let _ = write!(
        out,
        "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <meta name=\"color-scheme\" content=\"light\"><title>{}</title></head>\
         <body style=\"margin:0;padding:0;background:#f3f4f6\">\
         <table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" \
         style=\"background:#f3f4f6\"><tr><td align=\"center\" style=\"padding:16px 8px\">\
         <table role=\"presentation\" width=\"640\" cellpadding=\"0\" cellspacing=\"0\" \
         style=\"width:100%;max-width:640px;background:#ffffff;border:1px solid #e5e7eb\">",
        esc(&title)
    );

    let _ = write!(
        out,
        "<tr><td style=\"padding:24px 24px 0 24px;font-family:{FONT}\">\
         <div style=\"font-size:12px;color:#4b5563;text-transform:uppercase;letter-spacing:0.08em\">DumbMonit{}</div>\
         <div style=\"font-size:22px;font-weight:700;color:#111827;padding-top:4px\">{}</div>\
         <div style=\"font-size:13px;color:#4b5563;padding-top:4px\">{}</div></td></tr>",
        if preview { " &middot; preview" } else { "" },
        esc(&one_line(&data.name)),
        esc(&range_label(data, tz))
    );

    // Chiffres clés.
    let incidents = data.incidents.len();
    out.push_str(
        "<tr><td style=\"padding:16px 24px 0 24px\"><table role=\"presentation\" width=\"100%\" \
         cellpadding=\"0\" cellspacing=\"6\" style=\"border-collapse:separate\"><tr>",
    );
    out.push_str(&stat_cell(
        "Availability",
        &pct(data.fleet_uptime),
        &format!("{word}; {} vs previous", delta(data.fleet_uptime, data.fleet_previous_uptime)),
        color,
    ));
    out.push_str(&stat_cell(
        "Incidents",
        &incidents.to_string(),
        &count_delta(incidents, data.previous_incident_count),
        "#111827",
    ));
    out.push_str(&stat_cell(
        "Downtime",
        &duration(TimeDelta::seconds(data.downtime_secs)),
        "total across devices",
        "#111827",
    ));
    out.push_str("</tr></table></td></tr>");

    if !data.availability_known {
        note(
            &mut out,
            "Availability figures could not be read from the time-series database for this \
             report; incidents below come from the alert history.",
        );
    }

    // Dossiers.
    if data.groups.len() > 1 || data.groups.first().is_some_and(|g| g.name != UNGROUPED) {
        heading(&mut out, "Availability by group");
        table_open(&mut out, &["Group", "Devices", "Availability", "Previous"]);
        for group in &data.groups {
            out.push_str("<tr>");
            cell(&mut out, &esc(&group.name));
            cell(&mut out, &group.devices.to_string());
            cell(&mut out, &uptime_cell(group.uptime));
            cell(&mut out, &esc(&pct(group.previous_uptime)));
            out.push_str("</tr>");
        }
        table_close(&mut out);
    }

    // Équipements instables.
    let unstable = data.unstable();
    heading(&mut out, "Least stable devices");
    if unstable.is_empty() {
        note(&mut out, "No device had an incident in this period.");
    } else {
        table_open(&mut out, &["Device", "Incidents", "Downtime"]);
        for device in unstable {
            out.push_str("<tr>");
            cell(&mut out, &esc(&device.name));
            cell(&mut out, &device.incidents.to_string());
            cell(&mut out, &duration(TimeDelta::seconds(device.downtime_secs)));
            out.push_str("</tr>");
        }
        table_close(&mut out);
    }

    // Incidents.
    heading(&mut out, "Incidents");
    if data.incidents.is_empty() {
        note(&mut out, "No incident in this period.");
    } else {
        table_open(&mut out, &["Device", "Started", "Duration", "Severity", "Rule"]);
        for incident in data.incidents.iter().take(MAX_INCIDENT_ROWS) {
            let state = match incident.ended {
                None => " (ongoing)",
                Some(_) => "",
            };
            let started = if incident.began_before_period {
                format!("before {}", when(data.period.start, tz))
            } else {
                when(incident.started, tz)
            };
            out.push_str("<tr>");
            cell(&mut out, &esc(&incident.device));
            cell(&mut out, &esc(&started));
            cell(
                &mut out,
                &esc(&format!("{}{state}", duration(incident.duration(data.period.end)))),
            );
            cell(&mut out, &esc(&incident.severity));
            cell(&mut out, &esc(&incident.rule));
            out.push_str("</tr>");
        }
        table_close(&mut out);
        if data.incidents.len() > MAX_INCIDENT_ROWS {
            note(
                &mut out,
                &format!(
                    "{} more incident(s) not listed. See the alert history in DumbMonit.",
                    data.incidents.len() - MAX_INCIDENT_ROWS
                ),
            );
        }
    }

    // Disponibilité par équipement.
    heading(&mut out, "Availability by device");
    if data.devices.is_empty() {
        note(&mut out, "No device is being monitored.");
    } else {
        table_open(&mut out, &["Device", "Availability", "Previous", "Change"]);
        for device in data.devices.iter().take(MAX_DEVICE_ROWS) {
            out.push_str("<tr>");
            cell(&mut out, &esc(&device.name));
            cell(&mut out, &uptime_cell(device.uptime));
            cell(&mut out, &esc(&pct(device.previous_uptime)));
            cell(&mut out, &esc(&delta(device.uptime, device.previous_uptime)));
            out.push_str("</tr>");
        }
        table_close(&mut out);
        if data.devices.len() > MAX_DEVICE_ROWS {
            note(
                &mut out,
                &format!(
                    "{} more device(s) not listed; the best-performing ones are left out.",
                    data.devices.len() - MAX_DEVICE_ROWS
                ),
            );
        }
    }

    // Pied.
    let link = data
        .link
        .as_deref()
        .map(|url| {
            format!(" <a href=\"{0}\" style=\"color:#1d4ed8\">Open DumbMonit</a>.", esc(url))
        })
        .unwrap_or_default();
    let _ = write!(
        out,
        "<tr><td style=\"padding:24px;font-family:{FONT};font-size:12px;color:#6b7280\">\
         Availability is measured from the monitoring data of each device over the period; \
         the comparison is with the period of the same length just before it.{link} \
         You receive this because your address is on a DumbMonit report schedule.</td></tr>\
         </table></td></tr></table></body></html>"
    );
    out
}

// --------------------------------------------------------------------------
// Texte
// --------------------------------------------------------------------------

fn text(data: &ReportData, tz: Tz, preview: bool) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "DumbMonit {} report{}",
        frequency_word(data.frequency).to_lowercase(),
        if preview { " (preview)" } else { "" }
    );
    let _ = writeln!(out, "{}", one_line(&data.name));
    let _ = writeln!(out, "{}\n", range_label(data, tz));

    let _ = writeln!(
        out,
        "Availability: {} ({} vs previous)",
        pct(data.fleet_uptime),
        delta(data.fleet_uptime, data.fleet_previous_uptime)
    );
    let _ = writeln!(
        out,
        "Incidents:    {} ({})",
        data.incidents.len(),
        count_delta(data.incidents.len(), data.previous_incident_count)
    );
    let _ = writeln!(out, "Downtime:     {}\n", duration(TimeDelta::seconds(data.downtime_secs)));
    if !data.availability_known {
        let _ = writeln!(out, "Note: availability figures could not be read for this report.\n");
    }

    if data.groups.len() > 1 || data.groups.first().is_some_and(|g| g.name != UNGROUPED) {
        let _ = writeln!(out, "AVAILABILITY BY GROUP");
        for group in &data.groups {
            let _ = writeln!(
                out,
                "- {}: {} (previous {}, {} device(s))",
                one_line(&group.name),
                pct(group.uptime),
                pct(group.previous_uptime),
                group.devices
            );
        }
        out.push('\n');
    }

    let _ = writeln!(out, "LEAST STABLE DEVICES");
    let unstable = data.unstable();
    if unstable.is_empty() {
        let _ = writeln!(out, "None: no device had an incident.");
    }
    for device in unstable {
        let _ = writeln!(
            out,
            "- {}: {} incident(s), {} down",
            one_line(&device.name),
            device.incidents,
            duration(TimeDelta::seconds(device.downtime_secs))
        );
    }

    let _ = writeln!(out, "\nINCIDENTS");
    if data.incidents.is_empty() {
        let _ = writeln!(out, "None in this period.");
    }
    for incident in data.incidents.iter().take(MAX_INCIDENT_ROWS) {
        let _ = writeln!(
            out,
            "- {} | {} | {}{} | {} | {}",
            one_line(&incident.device),
            if incident.began_before_period {
                format!("before {}", when(data.period.start, tz))
            } else {
                when(incident.started, tz)
            },
            duration(incident.duration(data.period.end)),
            if incident.ended.is_none() { " (ongoing)" } else { "" },
            incident.severity,
            one_line(&incident.rule)
        );
    }
    if data.incidents.len() > MAX_INCIDENT_ROWS {
        let _ = writeln!(out, "(+{} more not listed)", data.incidents.len() - MAX_INCIDENT_ROWS);
    }

    let _ = writeln!(out, "\nAVAILABILITY BY DEVICE");
    let row = |device: &DeviceStat| {
        format!(
            "- {}: {} (previous {}, {})",
            one_line(&device.name),
            pct(device.uptime),
            pct(device.previous_uptime),
            delta(device.uptime, device.previous_uptime)
        )
    };
    for device in data.devices.iter().take(MAX_DEVICE_ROWS) {
        let _ = writeln!(out, "{}", row(device));
    }
    if data.devices.len() > MAX_DEVICE_ROWS {
        let _ = writeln!(out, "(+{} more not listed)", data.devices.len() - MAX_DEVICE_ROWS);
    }
    if let Some(url) = &data.link {
        let _ = writeln!(out, "\nOpen DumbMonit: {url}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reports::collect::{DeviceInfo, Inputs};
    use crate::reports::collect::{Incident, assemble};
    use crate::reports::schedule::Period;
    use std::collections::HashMap;

    fn at(iso: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(iso).expect("ISO date").with_timezone(&Utc)
    }

    fn hostile() -> ReportData {
        let period = Period::ending_at(at("2026-10-12T08:00:00Z"), Frequency::Weekly);
        let devices = vec![DeviceInfo {
            id: 1,
            name: "<script>alert(1)</script>".into(),
            group: "\"><img src=x onerror=alert(2)>".into(),
        }];
        let rules = HashMap::new();
        let uptime = HashMap::from([(1, 99.5)]);
        let mut data = assemble(Inputs {
            name: "<b>Team</b> & co",
            frequency: Frequency::Weekly,
            period,
            link: Some("https://mon.example.org/\"onmouseover=\"x".to_string()),
            devices: &devices,
            rule_names: &rules,
            uptime: &uptime,
            previous_uptime: &HashMap::new(),
            availability_known: true,
            incidents: &[],
        });
        data.incidents.push(Incident {
            device: "<i>nas</i>".into(),
            rule: "rule\"><svg onload=1>".into(),
            severity: "critical".into(),
            started: at("2026-10-06T10:00:00Z"),
            ended: None,
            began_before_period: false,
        });
        data
    }

    #[test]
    fn toute_donnee_est_echappee_dans_le_html() {
        let rendered = render(&hostile(), Tz::UTC, false);
        for forbidden in ["<script", "<img", "<svg", "<b>Team", "<i>nas", "onmouseover=\"x"] {
            assert!(!rendered.html.contains(forbidden), "{forbidden} leaked into the HTML");
        }
        assert!(rendered.html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(rendered.html.contains("&lt;b&gt;Team&lt;/b&gt; &amp; co"));
    }

    #[test]
    fn le_html_ne_charge_aucune_ressource_externe() {
        let rendered = render(&hostile(), Tz::UTC, false);
        assert!(!rendered.html.contains("<img"));
        assert!(!rendered.html.contains("<link"));
        assert!(!rendered.html.contains("src=\""));
        assert!(!rendered.html.contains("url("));
    }

    #[test]
    fn l_objet_et_le_texte_sont_sur_une_ligne_et_en_anglais() {
        let rendered = render(&hostile(), Tz::UTC, true);
        assert!(rendered.subject.starts_with("[Preview] [DumbMonit] Weekly report"));
        assert!(!rendered.subject.contains('\n'));
        assert!(rendered.text.contains("Availability: 99.50%"));
        assert!(rendered.text.contains("(ongoing)"));
    }

    #[test]
    fn l_echappement_neutralise_les_caracteres_de_controle() {
        assert_eq!(esc("a\r\nb\u{0}c"), "a \nb c");
        assert_eq!(esc("'\"&<>"), "&#39;&quot;&amp;&lt;&gt;");
    }
}
