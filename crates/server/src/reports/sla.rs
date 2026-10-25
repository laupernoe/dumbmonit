//! Rapport de disponibilité (SLA) : un PDF lisible par un client ou un
//! auditeur, et des CSV pour le tableur.
//!
//! Le contenu est celui de [`ReportData`] : disponibilité par moniteur et par
//! groupe, incidents de la période, temps de réponse moyen et au 95e
//! centile, et, si un objectif est donné, pour chaque moniteur s'il est tenu.
//! Aucun état n'est porté par la couleur seule : chaque pourcentage est suivi
//! d'un mot.
//!
//! Le PDF est écrit par [`super::pdf`] (texte et tableaux, polices standard,
//! sans dépendance) ; le logo n'y est pas incorporé, le nom de l'organisation
//! en tient lieu.

use chrono::{DateTime, Datelike, TimeDelta, Utc};
use chrono_tz::Tz;

use super::collect::{DeviceStat, ReportData, UNGROUPED};
use super::pdf::{self, Document, Font, PAGE_H, PAGE_W, Page, Rgb};
use super::render::{count_delta, delta, duration, one_line, verdict, when};

/// Plafonds : un PDF de plusieurs centaines de pages ne se lit plus.
pub const MAX_PDF_MONITORS: usize = 400;
pub const MAX_PDF_INCIDENTS: usize = 300;

/// Habillage et réglages d'un rapport.
pub struct Options {
    /// Nom affiché en tête : l'organisation, ou la page de statut choisie.
    pub brand: String,
    /// Objectif de disponibilité, en pour cent.
    pub objective: Option<f64>,
    /// Instant de génération, écrit dans le pied de page et les métadonnées.
    pub generated: DateTime<Utc>,
}

// --------------------------------------------------------------------------
// Noms de fichiers et étiquettes
// --------------------------------------------------------------------------

/// Racine du nom de fichier : `availability-2026-10` pour un mois civil entier,
/// sinon `availability-2026-10-05_2026-10-12`.
pub fn file_stem(data: &ReportData, tz: Tz) -> String {
    let start = data.period.start.with_timezone(&tz).date_naive();
    let last = (data.period.end - TimeDelta::seconds(1)).with_timezone(&tz).date_naive();
    let whole_month = start.day() == 1
        && last.month() == start.month()
        && last.year() == start.year()
        && last.succ_opt().is_some_and(|next| next.day() == 1);
    if whole_month {
        format!("availability-{}", start.format("%Y-%m"))
    } else {
        format!("availability-{}_{}", start.format("%Y-%m-%d"), last.format("%Y-%m-%d"))
    }
}

/// « 2026-10-01 to 2026-10-31 (UTC) », la fin étant le dernier jour inclus.
pub fn period_label(data: &ReportData, tz: Tz) -> String {
    let start = data.period.start.with_timezone(&tz);
    let last = (data.period.end - TimeDelta::seconds(1)).with_timezone(&tz);
    format!("{} to {} ({})", start.format("%Y-%m-%d"), last.format("%Y-%m-%d"), last.format("%Z"))
}

/// Mois d'un rapport mensuel : « October 2026 ».
pub fn month_label(data: &ReportData, tz: Tz) -> String {
    data.period.start.with_timezone(&tz).format("%B %Y").to_string()
}

fn pct(value: Option<f64>) -> String {
    value.map_or_else(|| "n/a".to_string(), |v| format!("{:.3}%", v.clamp(0.0, 100.0)))
}

fn ms(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_string(), |v| format!("{} ms", v.round() as i64))
}

/// Le mot qui accompagne un pourcentage : tenu / manqué avec un objectif, sinon
/// le verdict habituel des rapports.
fn status_word(uptime: Option<f64>, objective: Option<f64>) -> &'static str {
    match (uptime, objective) {
        (None, _) => "No data",
        (Some(value), Some(goal)) => {
            // Une tolérance d'un millionième : 99.9 contre 99.900000001.
            if value + 1e-6 >= goal { "Met" } else { "Missed" }
        }
        (Some(_), None) => verdict(uptime).0,
    }
}

// --------------------------------------------------------------------------
// CSV
// --------------------------------------------------------------------------

/// Une cellule CSV (RFC 4180). Une valeur qui commence par `=`, `+`, `-`, `@`
/// ou une tabulation serait lue comme une formule par un tableur : elle est
/// préfixée d'une apostrophe. Les noms de moniteurs viennent de l'utilisateur,
/// parfois du réseau.
pub fn csv_cell(raw: &str) -> String {
    let mut value = one_line(raw);
    if value.starts_with(['=', '+', '-', '@', '\t']) {
        value.insert(0, '\'');
    }
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value
    }
}

fn csv_row(cells: &[String]) -> String {
    let mut line = cells.iter().map(|c| csv_cell(c)).collect::<Vec<_>>().join(",");
    line.push_str("\r\n");
    line
}

fn number(value: Option<f64>, decimals: usize) -> String {
    value.map_or_else(String::new, |v| format!("{v:.decimals$}"))
}

/// Les moniteurs : une ligne chacun. Commence par une marque UTF-8 (BOM) pour
/// qu'Excel lise les accents.
pub fn csv_monitors(data: &ReportData, objective: Option<f64>) -> String {
    let mut out = String::from("\u{feff}");
    let mut header = vec![
        "group",
        "monitor",
        "availability_percent",
        "previous_availability_percent",
        "incidents",
        "downtime_seconds",
        "avg_response_ms",
        "p95_response_ms",
    ];
    if objective.is_some() {
        header.extend(["objective_percent", "objective"]);
    }
    out.push_str(&csv_row(&header.iter().map(|h| (*h).to_string()).collect::<Vec<_>>()));
    for device in &data.devices {
        let mut cells = vec![
            device.group.clone(),
            device.name.clone(),
            number(device.uptime, 3),
            number(device.previous_uptime, 3),
            device.incidents.to_string(),
            device.downtime_secs.to_string(),
            number(device.latency_avg_ms, 1),
            number(device.latency_p95_ms, 1),
        ];
        if let Some(goal) = objective {
            cells.push(number(Some(goal), 3));
            cells.push(status_word(device.uptime, objective).to_lowercase().replace(' ', "_"));
        }
        out.push_str(&csv_row(&cells));
    }
    out
}

/// Les incidents de la période, horodatés en UTC (lisibles par une machine).
pub fn csv_incidents(data: &ReportData) -> String {
    let iso = |at: DateTime<Utc>| at.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let mut out = String::from("\u{feff}");
    let header = [
        "monitor",
        "rule",
        "severity",
        "started_utc",
        "ended_utc",
        "duration_seconds",
        "ongoing",
        "began_before_period",
    ];
    out.push_str(&csv_row(&header.iter().map(|h| (*h).to_string()).collect::<Vec<_>>()));
    for incident in &data.incidents {
        out.push_str(&csv_row(&[
            incident.device.clone(),
            incident.rule.clone(),
            incident.severity.clone(),
            iso(incident.started),
            incident.ended.map(iso).unwrap_or_default(),
            incident.duration(data.period.end).num_seconds().max(0).to_string(),
            incident.ended.is_none().to_string(),
            incident.began_before_period.to_string(),
        ]));
    }
    out
}

// --------------------------------------------------------------------------
// PDF
// --------------------------------------------------------------------------

const LEFT: f64 = 40.0;
const RIGHT: f64 = PAGE_W - 40.0;
const TOP: f64 = PAGE_H - 44.0;
const BOTTOM: f64 = 56.0;
const ROW: f64 = 15.0;

const INK: Rgb = Rgb(17, 24, 39);
const MUTED: Rgb = Rgb(75, 85, 99);
const RULE: Rgb = Rgb(209, 213, 219);
const TINT: Rgb = Rgb(243, 244, 246);

fn word_color(word: &str) -> Rgb {
    match word {
        "Healthy" | "Met" => Rgb(21, 128, 61),
        "Degraded" => Rgb(180, 83, 9),
        "Poor" | "Missed" => Rgb(185, 28, 28),
        _ => MUTED,
    }
}

/// Les pages en construction, avec le curseur vertical.
struct Sheet {
    pages: Vec<Page>,
    page: Page,
    /// Ordonnée de la prochaine ligne (l'origine est en bas de la page).
    y: f64,
}

impl Sheet {
    fn new() -> Self {
        Self { pages: Vec::new(), page: Page::default(), y: TOP }
    }

    fn break_page(&mut self) {
        self.pages.push(std::mem::take(&mut self.page));
        self.y = TOP;
    }

    /// Passe à la page suivante s'il ne reste pas `height` points.
    fn need(&mut self, height: f64) -> bool {
        if self.y - height < BOTTOM {
            self.break_page();
            true
        } else {
            false
        }
    }

    fn heading(&mut self, text: &str) {
        self.need(60.0);
        self.y -= 22.0;
        self.page.text(LEFT, self.y, text, Font::Bold, 12.0, INK);
        self.y -= 8.0;
    }

    fn note(&mut self, text: &str) {
        self.need(ROW);
        self.y -= ROW;
        self.page.text(LEFT, self.y, text, Font::Regular, 9.0, MUTED);
    }
}

/// Une colonne de tableau.
struct Column {
    title: &'static str,
    x: f64,
    width: f64,
    /// Alignée à droite sur `x + width`.
    right: bool,
}

fn table_header(sheet: &mut Sheet, columns: &[Column]) {
    sheet.y -= ROW;
    sheet.page.rect_fill(LEFT, sheet.y - 4.0, RIGHT - LEFT, ROW, TINT);
    for column in columns {
        if column.right {
            sheet.page.text_right(
                column.x + column.width,
                sheet.y,
                column.title,
                Font::Bold,
                8.0,
                MUTED,
            );
        } else {
            sheet.page.text(column.x, sheet.y, column.title, Font::Bold, 8.0, MUTED);
        }
    }
}

/// Une ligne de tableau; le texte et sa couleur par colonne.
fn table_row(sheet: &mut Sheet, columns: &[Column], cells: &[(String, Rgb, Font)]) {
    sheet.y -= ROW;
    for (column, (text, color, font)) in columns.iter().zip(cells) {
        let shown = pdf::fit(text, *font, 8.5, column.width);
        if column.right {
            sheet.page.text_right(column.x + column.width, sheet.y, &shown, *font, 8.5, *color);
        } else {
            sheet.page.text(column.x, sheet.y, &shown, *font, 8.5, *color);
        }
    }
    sheet.page.line(LEFT, sheet.y - 4.0, RIGHT, sheet.y - 4.0, 0.4, RULE);
}

fn plain(text: impl Into<String>) -> (String, Rgb, Font) {
    (text.into(), INK, Font::Regular)
}

/// Un chiffre clé : étiquette, valeur, précision.
struct Stat {
    label: &'static str,
    value: String,
    sub: String,
    color: Rgb,
}

fn stat_box(page: &mut Page, x: f64, y: f64, width: f64, stat: &Stat) {
    page.rect_fill(x, y - 52.0, width, 56.0, TINT);
    page.text(x + 8.0, y - 10.0, &stat.label.to_uppercase(), Font::Bold, 7.0, MUTED);
    page.text(x + 8.0, y - 32.0, &stat.value, Font::Bold, 17.0, stat.color);
    let sub = pdf::fit(&stat.sub, Font::Regular, 7.5, width - 16.0);
    page.text(x + 8.0, y - 46.0, &sub, Font::Regular, 7.5, MUTED);
}

/// Le PDF du rapport.
pub fn pdf(data: &ReportData, tz: Tz, options: &Options) -> Vec<u8> {
    let mut sheet = Sheet::new();
    let (word, _) = verdict(data.fleet_uptime);
    let incidents = data.incidents.len();

    // En-tête.
    sheet.y -= 18.0;
    sheet.page.text(
        LEFT,
        sheet.y,
        &pdf::fit(&one_line(&options.brand), Font::Bold, 20.0, RIGHT - LEFT),
        Font::Bold,
        20.0,
        INK,
    );
    sheet.y -= 18.0;
    sheet.page.text(LEFT, sheet.y, "Availability report", Font::Bold, 12.0, MUTED);
    sheet.y -= 15.0;
    sheet.page.text(LEFT, sheet.y, &period_label(data, tz), Font::Regular, 10.0, INK);
    let report_name = one_line(&data.name);
    if !report_name.is_empty() {
        sheet.y -= 13.0;
        sheet.page.text(
            LEFT,
            sheet.y,
            &pdf::fit(&report_name, Font::Regular, 9.0, RIGHT - LEFT),
            Font::Regular,
            9.0,
            MUTED,
        );
    }
    sheet.y -= 10.0;
    sheet.page.line(LEFT, sheet.y, RIGHT, sheet.y, 1.0, INK);

    // Chiffres clés.
    sheet.y -= 14.0;
    let gap = 10.0;
    let width = (RIGHT - LEFT - 2.0 * gap) / 3.0;
    let top = sheet.y;
    let stats = [
        Stat {
            label: "Availability",
            value: pct(data.fleet_uptime),
            sub: format!(
                "{word}; {} vs previous",
                delta(data.fleet_uptime, data.fleet_previous_uptime)
            ),
            color: word_color(word),
        },
        Stat {
            label: "Incidents",
            value: incidents.to_string(),
            sub: count_delta(incidents, data.previous_incident_count),
            color: INK,
        },
        Stat {
            label: "Downtime",
            value: duration(TimeDelta::seconds(data.downtime_secs)),
            sub: "total across monitors".to_string(),
            color: INK,
        },
    ];
    for (index, stat) in stats.iter().enumerate() {
        stat_box(&mut sheet.page, LEFT + index as f64 * (width + gap), top, width, stat);
    }
    sheet.y -= 62.0;

    if let Some(goal) = options.objective {
        let measured: Vec<&DeviceStat> =
            data.devices.iter().filter(|d| d.uptime.is_some()).collect();
        let met = measured.iter().filter(|d| status_word(d.uptime, Some(goal)) == "Met").count();
        sheet.note(&format!(
            "SLA objective {goal:.3}%: met by {met} of {} monitor(s) with data. Fleet availability is {}.",
            measured.len(),
            status_word(data.fleet_uptime, Some(goal)).to_lowercase()
        ));
    }
    if !data.availability_known {
        sheet.note("Availability could not be read from the time-series database for this report.");
    }

    // Groupes.
    if data.groups.len() > 1 || data.groups.first().is_some_and(|g| g.name != UNGROUPED) {
        sheet.heading("Availability by group");
        let columns = [
            Column { title: "GROUP", x: LEFT, width: 250.0, right: false },
            Column { title: "MONITORS", x: 295.0, width: 60.0, right: true },
            Column { title: "AVAILABILITY", x: 365.0, width: 90.0, right: true },
            Column { title: "PREVIOUS", x: 465.0, width: 90.0, right: true },
        ];
        table_header(&mut sheet, &columns);
        for group in &data.groups {
            if sheet.need(ROW * 2.0) {
                table_header(&mut sheet, &columns);
            }
            table_row(
                &mut sheet,
                &columns,
                &[
                    plain(group.name.clone()),
                    plain(group.devices.to_string()),
                    plain(pct(group.uptime)),
                    plain(pct(group.previous_uptime)),
                ],
            );
        }
    }

    // Moniteurs.
    sheet.heading("Availability by monitor");
    if data.devices.is_empty() {
        sheet.note("No monitor is being monitored.");
    } else {
        let columns = [
            Column { title: "MONITOR", x: LEFT, width: 150.0, right: false },
            Column { title: "GROUP", x: 196.0, width: 78.0, right: false },
            Column { title: "AVAILABILITY", x: 280.0, width: 66.0, right: true },
            Column { title: "STATUS", x: 354.0, width: 44.0, right: false },
            Column { title: "AVG", x: 400.0, width: 40.0, right: true },
            Column { title: "P95", x: 442.0, width: 42.0, right: true },
            Column { title: "INC.", x: 486.0, width: 22.0, right: true },
            Column { title: "DOWNTIME", x: 512.0, width: 43.0, right: true },
        ];
        table_header(&mut sheet, &columns);
        for device in data.devices.iter().take(MAX_PDF_MONITORS) {
            if sheet.need(ROW * 2.0) {
                table_header(&mut sheet, &columns);
            }
            let word = status_word(device.uptime, options.objective);
            table_row(
                &mut sheet,
                &columns,
                &[
                    plain(device.name.clone()),
                    (device.group.clone(), MUTED, Font::Regular),
                    (pct(device.uptime), INK, Font::Bold),
                    (word.to_string(), word_color(word), Font::Bold),
                    plain(ms(device.latency_avg_ms)),
                    plain(ms(device.latency_p95_ms)),
                    plain(device.incidents.to_string()),
                    plain(duration(TimeDelta::seconds(device.downtime_secs))),
                ],
            );
        }
        if data.devices.len() > MAX_PDF_MONITORS {
            sheet.note(&format!(
                "{} more monitor(s) are in the CSV export.",
                data.devices.len() - MAX_PDF_MONITORS
            ));
        }
    }

    // Incidents.
    sheet.heading("Incidents");
    if data.incidents.is_empty() {
        sheet.note("No incident in this period.");
    } else {
        let columns = [
            Column { title: "MONITOR", x: LEFT, width: 130.0, right: false },
            Column { title: "STARTED", x: 176.0, width: 90.0, right: false },
            Column { title: "DURATION", x: 270.0, width: 62.0, right: false },
            Column { title: "SEVERITY", x: 338.0, width: 50.0, right: false },
            Column { title: "RULE", x: 394.0, width: 161.0, right: false },
        ];
        table_header(&mut sheet, &columns);
        for incident in data.incidents.iter().take(MAX_PDF_INCIDENTS) {
            if sheet.need(ROW * 2.0) {
                table_header(&mut sheet, &columns);
            }
            let started = if incident.began_before_period {
                "before period".to_string()
            } else {
                when(incident.started, tz)
            };
            let mut length = duration(incident.duration(data.period.end));
            if incident.ended.is_none() {
                length.push_str(" (ongoing)");
            }
            table_row(
                &mut sheet,
                &columns,
                &[
                    plain(incident.device.clone()),
                    plain(started),
                    plain(length),
                    plain(incident.severity.clone()),
                    plain(incident.rule.clone()),
                ],
            );
        }
        if data.incidents.len() > MAX_PDF_INCIDENTS {
            sheet.note(&format!(
                "{} more incident(s) are in the CSV export.",
                data.incidents.len() - MAX_PDF_INCIDENTS
            ));
        }
    }

    // Méthode.
    sheet.heading("How this is measured");
    for line in [
        "Availability is the share of the period during which each monitor was up: probes count the",
        "checks that succeeded, devices count the 5-minute slots in which they reported.",
        "The comparison is with the period of the same length just before it. Response times are",
        "the mean and 95th percentile of probe durations over the period.",
    ] {
        sheet.note(line);
    }

    // Pieds de page, une fois le nombre de pages connu.
    sheet.break_page();
    let pages = std::mem::take(&mut sheet.pages);
    let total = pages.len();
    let stamp = options.generated.format("%Y-%m-%d %H:%M UTC").to_string();
    let mut doc = Document::new(
        &format!("{} - availability report", one_line(&options.brand)),
        Some(options.generated.format("D:%Y%m%d%H%M%SZ").to_string()),
    );
    for (index, mut page) in pages.into_iter().enumerate() {
        page.line(LEFT, 44.0, RIGHT, 44.0, 0.4, RULE);
        page.text(LEFT, 32.0, &format!("DumbMonit - generated {stamp}"), Font::Regular, 8.0, MUTED);
        page.text_right(
            RIGHT,
            32.0,
            &format!("Page {} of {total}", index + 1),
            Font::Regular,
            8.0,
            MUTED,
        );
        doc.add_page(page);
    }
    doc.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reports::collect::{DeviceInfo, Inputs, assemble};
    use crate::reports::schedule::{Frequency, Period};
    use std::collections::HashMap;

    fn at(iso: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(iso).expect("ISO date").with_timezone(&Utc)
    }

    fn data_for(devices: Vec<DeviceInfo>, uptime: HashMap<i64, f64>) -> ReportData {
        let period = Period::custom(at("2026-10-01T00:00:00Z"), at("2026-11-01T00:00:00Z"));
        assemble(Inputs {
            name: "Monthly SLA",
            frequency: Frequency::Monthly,
            period,
            link: None,
            devices: &devices,
            rule_names: &HashMap::new(),
            uptime: &uptime,
            previous_uptime: &HashMap::new(),
            availability_known: true,
            incidents: &[],
            latency: &HashMap::from([(1, (120.4, 340.0))]),
        })
    }

    fn sample() -> ReportData {
        let devices = vec![
            DeviceInfo { id: 1, name: "Web, \"front\"".into(), group: "Edge".into() },
            DeviceInfo { id: 2, name: "=HYPERLINK(\"http://x\")".into(), group: String::new() },
            DeviceInfo { id: 3, name: "Idle".into(), group: "Edge".into() },
        ];
        data_for(devices, HashMap::from([(1, 99.95), (2, 98.5)]))
    }

    #[test]
    fn file_names_follow_the_period() {
        let data = sample();
        assert_eq!(file_stem(&data, Tz::UTC), "availability-2026-10");
        assert_eq!(period_label(&data, Tz::UTC), "2026-10-01 to 2026-10-31 (UTC)");
        assert_eq!(month_label(&data, Tz::UTC), "October 2026");
        let mut partial = sample();
        partial.period = Period::custom(at("2026-10-05T00:00:00Z"), at("2026-10-12T00:00:00Z"));
        assert_eq!(file_stem(&partial, Tz::UTC), "availability-2026-10-05_2026-10-11");
    }

    #[test]
    fn csv_cells_are_quoted_and_cannot_become_formulas() {
        assert_eq!(csv_cell("plain"), "plain");
        assert_eq!(csv_cell("a,b"), "\"a,b\"");
        assert_eq!(csv_cell("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_cell("=1+1"), "'=1+1");
        assert_eq!(csv_cell("@SUM(A1)"), "'@SUM(A1)");
        assert_eq!(csv_cell("-2"), "'-2");
        assert_eq!(csv_cell("two\nlines"), "two lines");
    }

    #[test]
    fn the_monitor_csv_has_one_row_per_monitor() {
        let csv = csv_monitors(&sample(), None);
        assert!(csv.starts_with("\u{feff}group,monitor,availability_percent,"));
        assert!(csv.lines().count() == 4, "{csv}");
        assert!(csv.contains("Edge,\"Web, \"\"front\"\"\",99.950,,0,0,120.4,340.0\r\n"), "{csv}");
        assert!(csv.contains("'=HYPERLINK("), "formulas are neutralised");
        assert!(!csv.contains("objective"));
        let with = csv_monitors(&sample(), Some(99.9));
        assert!(with.contains("objective_percent,objective"));
        assert!(with.contains(",99.900,met\r\n"), "{with}");
        assert!(with.contains(",99.900,missed\r\n"));
        assert!(with.contains(",99.900,no_data\r\n"));
    }

    #[test]
    fn the_objective_is_compared_with_a_tiny_tolerance() {
        assert_eq!(status_word(Some(99.9), Some(99.9)), "Met");
        assert_eq!(status_word(Some(99.899), Some(99.9)), "Missed");
        assert_eq!(status_word(None, Some(99.9)), "No data");
        assert_eq!(status_word(Some(99.95), None), "Healthy");
    }

    #[test]
    fn the_incident_csv_lists_incidents_in_utc() {
        let mut data = sample();
        data.incidents.push(crate::reports::collect::Incident {
            device: "NAS".into(),
            rule: "Monitor unreachable".into(),
            severity: "critical".into(),
            started: at("2026-10-06T10:05:00Z"),
            ended: Some(at("2026-10-06T11:05:00Z")),
            began_before_period: false,
        });
        let csv = csv_incidents(&data);
        assert!(csv.contains("NAS,Monitor unreachable,critical,2026-10-06T10:05:00Z,2026-10-06T11:05:00Z,3600,false,false\r\n"), "{csv}");
    }

    fn options() -> Options {
        Options {
            brand: "Acme (Ops)".into(),
            objective: Some(99.9),
            generated: at("2026-11-03T08:00:00Z"),
        }
    }

    #[test]
    fn the_pdf_names_the_organisation_and_the_period() {
        let bytes = pdf(&sample(), Tz::UTC, &options());
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.starts_with("%PDF-1.4"));
        assert!(text.contains("Acme \\(Ops\\)"));
        assert!(text.contains("Availability report"));
        assert!(text.contains("2026-10-01 to 2026-10-31 \\(UTC\\)"));
        assert!(text.contains("SLA objective 99.900%"));
        assert!(text.contains("Page 1 of 1"));
        assert!(text.contains("/CreationDate (D:20261103080000Z)"));
        // Deux appels donnent les mêmes octets : rien d'aléatoire.
        assert_eq!(bytes, pdf(&sample(), Tz::UTC, &options()));
    }

    #[test]
    fn a_long_table_runs_over_several_pages() {
        let devices: Vec<DeviceInfo> = (0..120)
            .map(|i| DeviceInfo { id: i, name: format!("monitor-{i}"), group: "G".into() })
            .collect();
        let uptime = (0..120).map(|i| (i, 99.0 + (i % 10) as f64 / 10.0)).collect();
        let bytes = pdf(&data_for(devices, uptime), Tz::UTC, &options());
        let text = String::from_utf8_lossy(&bytes);
        let pages = text.matches("/Type /Page ").count();
        assert!(pages >= 3, "{pages} pages");
        assert!(text.contains(&format!("Page {pages} of {pages}")));
    }

    #[test]
    fn an_empty_estate_still_gives_a_document() {
        let bytes = pdf(&data_for(Vec::new(), HashMap::new()), Tz::UTC, &options());
        assert!(String::from_utf8_lossy(&bytes).contains("No monitor is being monitored."));
    }
}
