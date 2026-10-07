//! Calendrier d'un rapport : quand part le prochain, quelle période il couvre,
//! et à quelles conditions un créneau est dû.
//!
//! Tout est pur (aucune horloge, aucune base) pour être testé sans rien monter.

use std::str::FromStr;

use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeDelta, TimeZone, Utc};
use chrono_tz::Tz;

/// Un créneau manqué de plus de trois heures n'est pas envoyé après coup : un
/// rapport hebdomadaire qui arrive le mercredi au lieu du lundi est plus
/// trompeur qu'utile, et un redémarrage ne doit pas déclencher de rafale.
pub const GRACE: TimeDelta = TimeDelta::hours(3);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frequency {
    Daily,
    Weekly,
    Monthly,
}

impl Frequency {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "daily" => Some(Self::Daily),
            "weekly" => Some(Self::Weekly),
            "monthly" => Some(Self::Monthly),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
        }
    }

    /// Durée de la période couverte, en jours (le mensuel couvre 30 jours
    /// glissants, comme la fenêtre de 30 jours des pages de statut).
    pub fn span_days(self) -> i64 {
        match self {
            Self::Daily => 1,
            Self::Weekly => 7,
            Self::Monthly => 30,
        }
    }
}

/// Fenêtre couverte par un rapport, et celle qui la précède pour comparer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Period {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub previous_start: DateTime<Utc>,
}

impl Period {
    pub fn ending_at(end: DateTime<Utc>, frequency: Frequency) -> Self {
        let span = Duration::days(frequency.span_days());
        Self { start: end - span, end, previous_start: end - span - span }
    }

    pub fn span(&self) -> TimeDelta {
        self.end - self.start
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Schedule {
    pub frequency: Frequency,
    /// 0 = lundi.
    pub weekday: u32,
    /// 1 à 28 : jamais de jour qui manque certains mois.
    pub day_of_month: u32,
    pub hour: u32,
    pub tz: Tz,
}

/// Valide un nom de fuseau IANA.
pub fn parse_timezone(name: &str) -> Option<Tz> {
    let name = name.trim();
    if name.is_empty() { None } else { Tz::from_str(name).ok() }
}

impl Schedule {
    fn matches(&self, date: NaiveDate) -> bool {
        match self.frequency {
            Frequency::Daily => true,
            Frequency::Weekly => date.weekday().num_days_from_monday() == self.weekday,
            Frequency::Monthly => date.day() == self.day_of_month,
        }
    }

    /// Instant UTC de l'heure locale `hour` du jour `date`. Dans un trou de
    /// changement d'heure (l'heure n'existe pas), on prend l'heure suivante.
    fn slot_on(&self, date: NaiveDate) -> Option<DateTime<Utc>> {
        for hour in [self.hour, self.hour + 1] {
            let Some(naive) = date.and_hms_opt(hour % 24, 0, 0) else { continue };
            if let Some(local) = self.tz.from_local_datetime(&naive).earliest() {
                return Some(local.with_timezone(&Utc));
            }
        }
        None
    }

    /// Dernier créneau échu (≤ `now`).
    pub fn latest_slot(&self, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        let today = now.with_timezone(&self.tz).date_naive();
        (0..=62)
            .filter_map(|back| today.checked_sub_signed(Duration::days(back)))
            .filter(|date| self.matches(*date))
            .filter_map(|date| self.slot_on(date))
            .find(|slot| *slot <= now)
    }

    /// Prochain créneau (> `now`).
    pub fn next_slot(&self, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        let today = now.with_timezone(&self.tz).date_naive();
        (0..=62)
            .filter_map(|ahead| today.checked_add_signed(Duration::days(ahead)))
            .filter(|date| self.matches(*date))
            .filter_map(|date| self.slot_on(date))
            .find(|slot| *slot > now)
    }

    /// Créneau à envoyer maintenant, s'il y en a un.
    ///
    /// Refusé quand le créneau précède l'armement (pas de rétroactivité), a déjà
    /// été envoyé, ou est échu depuis plus que [`GRACE`].
    pub fn due_slot(
        &self,
        now: DateTime<Utc>,
        armed_at: DateTime<Utc>,
        last_sent: Option<DateTime<Utc>>,
    ) -> Option<DateTime<Utc>> {
        let slot = self.latest_slot(now)?;
        if slot <= armed_at || now - slot > GRACE {
            return None;
        }
        if last_sent.is_some_and(|sent| sent >= slot) {
            return None;
        }
        Some(slot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(iso: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(iso).expect("ISO date").with_timezone(&Utc)
    }

    fn weekly_monday_8(tz: &str) -> Schedule {
        Schedule {
            frequency: Frequency::Weekly,
            weekday: 0,
            day_of_month: 1,
            hour: 8,
            tz: parse_timezone(tz).expect("timezone"),
        }
    }

    #[test]
    fn la_periode_couvre_la_duree_de_la_frequence_et_la_precedente() {
        let end = at("2026-10-12T08:00:00Z");
        let weekly = Period::ending_at(end, Frequency::Weekly);
        assert_eq!(weekly.start, at("2026-10-05T08:00:00Z"));
        assert_eq!(weekly.previous_start, at("2026-09-28T08:00:00Z"));
        assert_eq!(Period::ending_at(end, Frequency::Daily).span(), Duration::days(1));
        assert_eq!(Period::ending_at(end, Frequency::Monthly).span(), Duration::days(30));
    }

    #[test]
    fn le_dernier_creneau_hebdomadaire_est_le_lundi_precedent() {
        let schedule = weekly_monday_8("UTC");
        // Mercredi 7 octobre 2026 : le lundi 5 à 08:00 est le dernier créneau.
        assert_eq!(
            schedule.latest_slot(at("2026-10-07T12:00:00Z")),
            Some(at("2026-10-05T08:00:00Z"))
        );
        // Lundi avant l'heure : on retombe sur le lundi d'avant.
        assert_eq!(
            schedule.latest_slot(at("2026-10-05T07:59:00Z")),
            Some(at("2026-09-28T08:00:00Z"))
        );
        assert_eq!(
            schedule.next_slot(at("2026-10-07T12:00:00Z")),
            Some(at("2026-10-12T08:00:00Z"))
        );
    }

    #[test]
    fn le_fuseau_decale_le_creneau_en_utc() {
        // Paris est en UTC+2 en octobre (heure d'été jusqu'au 25).
        let schedule = weekly_monday_8("Europe/Paris");
        assert_eq!(
            schedule.latest_slot(at("2026-10-07T12:00:00Z")),
            Some(at("2026-10-05T06:00:00Z"))
        );
    }

    #[test]
    fn le_mensuel_vise_le_jour_du_mois() {
        let schedule = Schedule {
            frequency: Frequency::Monthly,
            weekday: 0,
            day_of_month: 15,
            hour: 6,
            tz: Tz::UTC,
        };
        assert_eq!(
            schedule.latest_slot(at("2026-10-07T12:00:00Z")),
            Some(at("2026-09-15T06:00:00Z"))
        );
        assert_eq!(
            schedule.next_slot(at("2026-10-07T12:00:00Z")),
            Some(at("2026-10-15T06:00:00Z"))
        );
    }

    #[test]
    fn un_creneau_echu_depuis_peu_est_du_une_seule_fois() {
        let schedule = weekly_monday_8("UTC");
        let armed = at("2026-09-01T00:00:00Z");
        let now = at("2026-10-05T08:05:00Z");
        let slot = schedule.due_slot(now, armed, None).expect("due");
        assert_eq!(slot, at("2026-10-05T08:00:00Z"));
        // Réclamé à 08:05 : plus rien n'est dû ensuite, même une minute plus tard.
        assert_eq!(schedule.due_slot(at("2026-10-05T08:06:00Z"), armed, Some(now)), None);
        // Et rien non plus pendant la semaine qui suit.
        assert_eq!(schedule.due_slot(at("2026-10-09T10:00:00Z"), armed, Some(now)), None);
    }

    #[test]
    fn pas_de_rattrapage_apres_une_longue_interruption() {
        let schedule = weekly_monday_8("UTC");
        let armed = at("2026-09-01T00:00:00Z");
        // Le serveur redémarre mercredi : le créneau de lundi est trop ancien.
        assert_eq!(schedule.due_slot(at("2026-10-07T09:00:00Z"), armed, None), None);
        // Il redémarre dans la fenêtre de grâce : on envoie.
        assert!(schedule.due_slot(at("2026-10-05T09:30:00Z"), armed, None).is_some());
    }

    #[test]
    fn un_creneau_anterieur_a_l_armement_n_est_jamais_envoye() {
        let schedule = weekly_monday_8("UTC");
        // Créé lundi 08:02, donc après le créneau de 08:00.
        let armed = at("2026-10-05T08:02:00Z");
        assert_eq!(schedule.due_slot(at("2026-10-05T08:03:00Z"), armed, None), None);
        assert!(schedule.due_slot(at("2026-10-12T08:01:00Z"), armed, None).is_some());
    }

    #[test]
    fn un_trou_de_changement_d_heure_ne_perd_pas_le_creneau() {
        // 2026-03-29 02:30 n'existe pas à Paris.
        let schedule = Schedule {
            frequency: Frequency::Daily,
            weekday: 0,
            day_of_month: 1,
            hour: 2,
            tz: Tz::Europe__Paris,
        };
        assert!(schedule.latest_slot(at("2026-03-29T12:00:00Z")).is_some());
    }

    #[test]
    fn les_fuseaux_inconnus_sont_refuses() {
        assert!(parse_timezone("Europe/Paris").is_some());
        assert!(parse_timezone("Mars/Olympus").is_none());
        assert!(parse_timezone("  ").is_none());
    }
}
