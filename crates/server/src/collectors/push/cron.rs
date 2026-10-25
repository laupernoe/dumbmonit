//! Planning d'un travail surveillé : une expression cron à cinq champs lue dans
//! un fuseau IANA, et le calcul de ses échéances.
//!
//! Écrit ici plutôt qu'emprunté : le besoin est étroit (« quelle est l'échéance
//! qui précède, ou qui suit, cet instant ? ») et les règles d'un changement
//! d'heure sont celles qu'on veut décider soi-même.
//!
//! # Syntaxe
//!
//! `minute heure jour-du-mois mois jour-de-semaine`, chaque champ étant une
//! liste séparée par des virgules de `*`, `a`, `a-b`, avec un pas facultatif
//! (`*/15`, `10-50/10`, `5/20`). Les mois se nomment `jan`..`dec`, les jours
//! `sun`..`sat` (0 ou 7 pour dimanche). Raccourcis : `@yearly`, `@annually`,
//! `@monthly`, `@weekly`, `@daily`, `@midnight`, `@hourly`. Comme cron, quand le
//! jour du mois et le jour de semaine sont tous deux restreints, une date
//! convient si l'un OU l'autre correspond.
//!
//! # Changements d'heure
//!
//! - Heure répétée (retour à l'heure d'hiver) : l'échéance a lieu une seule fois,
//!   à la première occurrence.
//! - Heure inexistante (passage à l'heure d'été) : l'échéance a lieu à la
//!   première minute qui existe après le trou.

use std::str::FromStr;

use chrono::{DateTime, Datelike, Duration, LocalResult, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;

/// Nombre de jours explorés avant de renoncer : huit ans couvrent un 29 février.
const SEARCH_DAYS: i64 = 3_000;

const MONTHS: [&str; 12] =
    ["jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"];
const WEEKDAYS: [&str; 7] = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"];

/// Une expression cron analysée, sous forme d'ensembles de valeurs permises.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CronExpr {
    minutes: u64,
    hours: u32,
    days_of_month: u32,
    months: u16,
    days_of_week: u8,
    /// Le champ commençait par `*` : il ne restreint pas la date.
    any_day_of_month: bool,
    any_day_of_week: bool,
}

impl CronExpr {
    /// Analyse une expression. L'erreur est lisible telle quelle.
    pub fn parse(raw: &str) -> Result<Self, String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err("the cron expression is empty".into());
        }
        let expanded = match trimmed.to_ascii_lowercase().as_str() {
            "@yearly" | "@annually" => "0 0 1 1 *",
            "@monthly" => "0 0 1 * *",
            "@weekly" => "0 0 * * 0",
            "@daily" | "@midnight" => "0 0 * * *",
            "@hourly" => "0 * * * *",
            other if other.starts_with('@') => {
                return Err(format!("unknown shortcut \"{trimmed}\""));
            }
            _ => trimmed,
        };
        let fields: Vec<&str> = expanded.split_whitespace().collect();
        let [minute, hour, day, month, weekday] = fields[..] else {
            return Err(format!(
                "expected 5 fields (minute hour day-of-month month day-of-week), got {}",
                fields.len()
            ));
        };
        let minutes = parse_field(minute, "minute", 0, 59, &[], 0)?;
        let hours = parse_field(hour, "hour", 0, 23, &[], 0)?;
        let days_of_month = parse_field(day, "day of month", 1, 31, &[], 0)?;
        let months = parse_field(month, "month", 1, 12, &MONTHS, 1)?;
        // 7 est un second nom du dimanche.
        let mut days_of_week = parse_field(weekday, "day of week", 0, 7, &WEEKDAYS, 0)?;
        if days_of_week & (1 << 7) != 0 {
            days_of_week = (days_of_week & !(1 << 7)) | 1;
        }
        Ok(Self {
            minutes,
            hours: hours as u32,
            days_of_month: days_of_month as u32,
            months: months as u16,
            days_of_week: days_of_week as u8,
            any_day_of_month: day.starts_with('*'),
            any_day_of_week: weekday.starts_with('*'),
        })
    }

    /// Vrai si la date (calendrier local) est un jour d'exécution.
    pub fn matches_date(&self, date: NaiveDate) -> bool {
        if self.months & (1 << date.month()) == 0 {
            return false;
        }
        let by_day = self.days_of_month & (1 << date.day()) != 0;
        let by_weekday = self.days_of_week & (1 << date.weekday().num_days_from_sunday()) != 0;
        match (self.any_day_of_month, self.any_day_of_week) {
            (false, false) => by_day || by_weekday,
            (false, true) => by_day,
            (true, false) => by_weekday,
            (true, true) => true,
        }
    }

    /// Heures et minutes locales permises, en ordre croissant.
    fn times(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
        (0..24u32).filter(move |h| self.hours & (1 << h) != 0).flat_map(move |h| {
            (0..60u32).filter(move |m| self.minutes & (1 << m) != 0).map(move |m| (h, m))
        })
    }
}

/// Lit un champ et rend l'ensemble de ses valeurs sous forme de masque de bits.
///
/// `names` donne les noms acceptés, le premier valant `names_from`.
fn parse_field(
    raw: &str,
    label: &str,
    min: u32,
    max: u32,
    names: &[&str],
    names_from: u32,
) -> Result<u64, String> {
    let value = |text: &str| -> Result<u32, String> {
        let lower = text.to_ascii_lowercase();
        if let Some(index) = names.iter().position(|name| *name == lower) {
            return Ok(index as u32 + names_from);
        }
        let number: u32 =
            text.parse().map_err(|_| format!("{label}: \"{text}\" is not a valid value"))?;
        if !(min..=max).contains(&number) {
            return Err(format!("{label}: {number} is out of range ({min}-{max})"));
        }
        Ok(number)
    };
    let mut set: u64 = 0;
    for item in raw.split(',') {
        if item.is_empty() {
            return Err(format!("{label}: empty item in \"{raw}\""));
        }
        let (range, step) = match item.split_once('/') {
            Some((range, step)) => {
                let step: u32 = step
                    .parse()
                    .map_err(|_| format!("{label}: \"{step}\" is not a valid step"))?;
                if step == 0 {
                    return Err(format!("{label}: the step cannot be zero"));
                }
                (range, Some(step))
            }
            None => (item, None),
        };
        let (from, to) = if range == "*" {
            (min, max)
        } else if let Some((from, to)) = range.split_once('-') {
            (value(from)?, value(to)?)
        } else {
            let single = value(range)?;
            // `5/20` : à partir de 5, jusqu'au bout du champ.
            if step.is_some() { (single, max) } else { (single, single) }
        };
        if from > to {
            return Err(format!("{label}: the range {from}-{to} runs backwards"));
        }
        let mut current = from;
        while current <= to {
            set |= 1 << current;
            current = current.saturating_add(step.unwrap_or(1));
        }
    }
    Ok(set)
}

/// Une expression cron placée dans un fuseau : le planning d'un travail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schedule {
    expr: CronExpr,
    tz: Tz,
    /// Textes d'origine, pour les afficher tels que l'utilisateur les a écrits.
    source: String,
    zone: String,
}

/// Valide un nom de fuseau IANA.
pub fn parse_timezone(name: &str) -> Option<Tz> {
    let name = name.trim();
    if name.is_empty() { None } else { Tz::from_str(name).ok() }
}

impl Schedule {
    pub fn new(expression: &str, timezone: &str) -> Result<Self, String> {
        let expr = CronExpr::parse(expression)?;
        let tz = parse_timezone(timezone)
            .ok_or_else(|| format!("\"{timezone}\" is not an IANA time zone (e.g. Europe/Paris)"))?;
        Ok(Self {
            expr,
            tz,
            source: expression.trim().to_string(),
            zone: timezone.trim().to_string(),
        })
    }

    pub fn expression(&self) -> &str {
        &self.source
    }

    pub fn timezone_name(&self) -> &str {
        &self.zone
    }

    /// Instants (UTC) du jour local `date`, en ordre croissant et sans doublon.
    fn instants_on(&self, date: NaiveDate) -> Vec<DateTime<Utc>> {
        let mut out: Vec<DateTime<Utc>> = Vec::new();
        for (hour, minute) in self.expr.times() {
            let Some(naive) = date.and_hms_opt(hour, minute, 0) else { continue };
            match self.tz.from_local_datetime(&naive) {
                LocalResult::Single(local) => out.push(local.with_timezone(&Utc)),
                // Heure répétée : une exécution à heure fixe n'a lieu qu'une fois
                // (la première) ; une expression sur toutes les heures suit le
                // temps réel et passe donc aux deux occurrences.
                LocalResult::Ambiguous(first, second) => {
                    out.push(first.with_timezone(&Utc));
                    if self.expr.hours == (1 << 24) - 1 {
                        out.push(second.with_timezone(&Utc));
                    }
                }
                // Heure inexistante : première minute valide après le trou.
                LocalResult::None => {
                    let shifted = (1..=180).find_map(|plus| {
                        self.tz.from_local_datetime(&(naive + Duration::minutes(plus))).earliest()
                    });
                    if let Some(shifted) = shifted {
                        out.push(shifted.with_timezone(&Utc));
                    }
                }
            }
        }
        out.sort();
        out.dedup();
        out
    }

    /// Première échéance strictement après `after_ms` (millisecondes Unix).
    pub fn next_after(&self, after_ms: i64) -> Option<i64> {
        let after = DateTime::from_timestamp_millis(after_ms)?;
        let start = after.with_timezone(&self.tz).date_naive() - Duration::days(1);
        for ahead in 0..SEARCH_DAYS {
            let date = start.checked_add_signed(Duration::days(ahead))?;
            if !self.expr.matches_date(date) {
                continue;
            }
            if let Some(found) = self.instants_on(date).into_iter().find(|at| *at > after) {
                return Some(found.timestamp_millis());
            }
        }
        None
    }

    /// Dernière échéance à `at_ms` ou avant (millisecondes Unix).
    pub fn at_or_before(&self, at_ms: i64) -> Option<i64> {
        let at = DateTime::from_timestamp_millis(at_ms)?;
        let start = at.with_timezone(&self.tz).date_naive() + Duration::days(1);
        for back in 0..SEARCH_DAYS {
            let date = start.checked_sub_signed(Duration::days(back))?;
            if !self.expr.matches_date(date) {
                continue;
            }
            if let Some(found) = self.instants_on(date).into_iter().rev().find(|t| *t <= at) {
                return Some(found.timestamp_millis());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(text: &str) -> i64 {
        DateTime::parse_from_rfc3339(text).unwrap().timestamp_millis()
    }

    fn schedule(expr: &str, tz: &str) -> Schedule {
        Schedule::new(expr, tz).unwrap_or_else(|e| panic!("{expr} : {e}"))
    }

    #[test]
    fn les_champs_acceptent_listes_plages_pas_et_noms() {
        let e = CronExpr::parse("0,30 8-10 * jan,mar-may mon-fri").unwrap();
        assert_eq!(e.minutes, (1 << 0) | (1 << 30));
        assert_eq!(e.hours, (1 << 8) | (1 << 9) | (1 << 10));
        assert_eq!(e.months, (1 << 1) | (1 << 3) | (1 << 4) | (1 << 5));
        assert_eq!(e.days_of_week, 0b0111110);
        let e = CronExpr::parse("*/15 * * * *").unwrap();
        assert_eq!(e.minutes, (1 << 0) | (1 << 15) | (1 << 30) | (1 << 45));
        let e = CronExpr::parse("10-50/20 * * * *").unwrap();
        assert_eq!(e.minutes, (1 << 10) | (1 << 30) | (1 << 50));
        let e = CronExpr::parse("5/20 * * * *").unwrap();
        assert_eq!(e.minutes, (1 << 5) | (1 << 25) | (1 << 45));
        // Sept et zéro nomment tous deux le dimanche ; les noms sont insensibles à la casse.
        assert_eq!(CronExpr::parse("0 0 * * 7").unwrap().days_of_week, 1);
        assert_eq!(CronExpr::parse("0 0 * * SUN").unwrap().days_of_week, 1);
        assert_eq!(CronExpr::parse("0 0 * * 5-7").unwrap().days_of_week, 0b1100001);
    }

    #[test]
    fn les_raccourcis_se_developpent() {
        for (shortcut, expanded) in [
            ("@daily", "0 0 * * *"),
            ("@midnight", "0 0 * * *"),
            ("@hourly", "0 * * * *"),
            ("@weekly", "0 0 * * 0"),
            ("@monthly", "0 0 1 * *"),
            ("@yearly", "0 0 1 1 *"),
            ("@ANNUALLY", "0 0 1 1 *"),
        ] {
            assert_eq!(CronExpr::parse(shortcut), CronExpr::parse(expanded), "{shortcut}");
        }
    }

    #[test]
    fn une_expression_invalide_donne_une_erreur_lisible() {
        for bad in [
            "",
            "   ",
            "* * * *",
            "* * * * * *",
            "60 * * * *",
            "* 24 * * *",
            "* * 0 * *",
            "* * 32 * *",
            "* * * 13 *",
            "* * * * 8",
            "*/0 * * * *",
            "*/x * * * *",
            "5-1 * * * *",
            "a * * * *",
            "1,,2 * * * *",
            "@reboot",
            "@sometimes",
            "* * * foo *",
        ] {
            let error = CronExpr::parse(bad).unwrap_err();
            assert!(!error.is_empty(), "{bad:?}");
        }
        assert!(Schedule::new("0 3 * * *", "Mars/Olympus").unwrap_err().contains("time zone"));
        assert!(Schedule::new("0 3 * * *", "").is_err());
    }

    #[test]
    fn echeance_quotidienne_en_utc() {
        let s = schedule("30 2 * * *", "UTC");
        assert_eq!(s.next_after(utc("2026-03-01T00:00:00Z")), Some(utc("2026-03-01T02:30:00Z")));
        // Strictement après : à l'instant exact, la suivante est le lendemain.
        assert_eq!(s.next_after(utc("2026-03-01T02:30:00Z")), Some(utc("2026-03-02T02:30:00Z")));
        assert_eq!(s.at_or_before(utc("2026-03-01T02:30:00Z")), Some(utc("2026-03-01T02:30:00Z")));
        assert_eq!(s.at_or_before(utc("2026-03-01T02:29:59Z")), Some(utc("2026-02-28T02:30:00Z")));
    }

    #[test]
    fn le_fuseau_decale_les_echeances() {
        // 03:00 à Paris : 02:00 UTC en hiver, 01:00 UTC en été.
        let s = schedule("0 3 * * *", "Europe/Paris");
        assert_eq!(s.next_after(utc("2026-01-10T00:00:00Z")), Some(utc("2026-01-10T02:00:00Z")));
        assert_eq!(s.next_after(utc("2026-07-10T00:00:00Z")), Some(utc("2026-07-10T01:00:00Z")));
    }

    #[test]
    fn jour_du_mois_et_jour_de_semaine_sont_un_ou() {
        // Le 13 OU un vendredi.
        let s = schedule("0 12 13 * fri", "UTC");
        // Le 1er mai 2026 est un vendredi.
        assert_eq!(s.next_after(utc("2026-04-30T13:00:00Z")), Some(utc("2026-05-01T12:00:00Z")));
        // Le mercredi 13 mai 2026.
        assert_eq!(s.next_after(utc("2026-05-12T13:00:00Z")), Some(utc("2026-05-13T12:00:00Z")));
        // Avec `*` sur l'un des deux, c'est un ET.
        let s = schedule("0 12 13 * *", "UTC");
        assert_eq!(s.next_after(utc("2026-05-13T12:00:00Z")), Some(utc("2026-06-13T12:00:00Z")));
    }

    #[test]
    fn heure_inexistante_au_passage_a_lheure_dete() {
        // Paris, 29 mars 2026 : 02:00 -> 03:00. 02:30 n'existe pas ce jour-là.
        let s = schedule("30 2 * * *", "Europe/Paris");
        // L'échéance a lieu juste après le trou : 03:00 locale = 01:00 UTC.
        assert_eq!(s.next_after(utc("2026-03-27T12:00:00Z")), Some(utc("2026-03-28T01:30:00Z")));
        assert_eq!(s.next_after(utc("2026-03-28T01:30:00Z")), Some(utc("2026-03-29T01:00:00Z")));
        assert_eq!(s.next_after(utc("2026-03-29T01:00:00Z")), Some(utc("2026-03-30T00:30:00Z")));
        assert_eq!(s.at_or_before(utc("2026-03-29T05:00:00Z")), Some(utc("2026-03-29T01:00:00Z")));
    }

    #[test]
    fn heure_repetee_au_retour_a_lheure_dhiver() {
        // Paris, 25 octobre 2026 : 03:00 -> 02:00, donc 02:30 passe deux fois.
        let s = schedule("30 2 * * *", "Europe/Paris");
        // Une seule exécution, la première (02:30 CEST = 00:30 UTC).
        assert_eq!(s.next_after(utc("2026-10-24T12:00:00Z")), Some(utc("2026-10-25T00:30:00Z")));
        assert_eq!(s.next_after(utc("2026-10-25T00:30:00Z")), Some(utc("2026-10-26T01:30:00Z")));
        // Une expression horaire, elle, suit les heures réelles sans doublon.
        let hourly = schedule("0 * * * *", "Europe/Paris");
        let mut at = utc("2026-10-25T21:30:00Z") - 24 * 3_600_000;
        let mut seen = Vec::new();
        for _ in 0..26 {
            at = hourly.next_after(at).unwrap();
            seen.push(at);
        }
        assert!(seen.windows(2).all(|w| w[1] - w[0] == 3_600_000), "{seen:?}");
    }

    #[test]
    fn le_29_fevrier_est_trouve() {
        let s = schedule("0 0 29 2 *", "UTC");
        assert_eq!(s.next_after(utc("2026-03-01T00:00:00Z")), Some(utc("2028-02-29T00:00:00Z")));
        assert_eq!(s.at_or_before(utc("2026-03-01T00:00:00Z")), Some(utc("2024-02-29T00:00:00Z")));
    }

    #[test]
    fn une_date_impossible_ne_boucle_pas() {
        let s = schedule("0 0 31 2 *", "UTC");
        assert_eq!(s.next_after(utc("2026-01-01T00:00:00Z")), None);
        assert_eq!(s.at_or_before(utc("2026-01-01T00:00:00Z")), None);
    }

    #[test]
    fn echeances_a_la_minute() {
        let s = schedule("* * * * *", "UTC");
        assert_eq!(s.next_after(utc("2026-05-01T10:00:30Z")), Some(utc("2026-05-01T10:01:00Z")));
        assert_eq!(s.at_or_before(utc("2026-05-01T10:00:30Z")), Some(utc("2026-05-01T10:00:00Z")));
    }

    #[test]
    fn echeance_hebdomadaire_et_mensuelle() {
        // Dimanche à 04:00.
        let s = schedule("0 4 * * sun", "UTC");
        // Le 3 mai 2026 est un dimanche.
        assert_eq!(s.next_after(utc("2026-05-01T00:00:00Z")), Some(utc("2026-05-03T04:00:00Z")));
        let s = schedule("@monthly", "UTC");
        assert_eq!(s.next_after(utc("2026-05-01T00:00:00Z")), Some(utc("2026-06-01T00:00:00Z")));
        assert_eq!(s.at_or_before(utc("2026-05-20T00:00:00Z")), Some(utc("2026-05-01T00:00:00Z")));
    }
}
