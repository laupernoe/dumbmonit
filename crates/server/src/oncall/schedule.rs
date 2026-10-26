//! Plannings d'astreinte : qui est d'astreinte à un instant donné.
//!
//! Un planning est un fuseau, des couches et des remplacements. Chaque couche
//! fait tourner une liste ordonnée de participants (quotidien, hebdomadaire ou
//! sur mesure) à partir d'une première relève, et peut être restreinte à des
//! fenêtres horaires (« jours ouvrés 9 h – 18 h »). Là où plusieurs couches
//! sont actives, la dernière de la liste l'emporte : c'est ce qui permet de
//! poser une couche « nuits et week-ends » par-dessus une couche de base. Un
//! remplacement ponctuel l'emporte sur toutes les couches.
//!
//! Le module est pur : il ne lit ni la base ni l'horloge, et tout se teste avec
//! des instants fixes. La règle qui gouverne les rotations en jours et en
//! semaines est celle de l'horloge murale : « relève le lundi à 9 h à Paris »
//! reste à 9 h locales toute l'année, si bien qu'une garde dure 23 ou 25 heures
//! réelles la semaine d'un changement d'heure. Une rotation en heures, elle,
//! compte en temps absolu : « toutes les 12 heures » dure toujours 12 heures.

use std::collections::BTreeSet;
use std::str::FromStr;

use chrono::{
    DateTime, Datelike, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta, TimeZone, Timelike, Utc,
};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

/// Minutes dans une journée.
pub const MINUTES_PER_DAY: u32 = 24 * 60;
/// Couches au plus par planning : au-delà, plus personne ne sait lire qui
/// l'emporte.
pub const MAX_LAYERS: usize = 8;
/// Participants au plus par couche.
pub const MAX_PARTICIPANTS: usize = 50;
/// Fenêtres de restriction au plus par couche (deux par jour).
pub const MAX_WINDOWS: usize = 14;
/// Longueur maximale d'une garde sur mesure, quelle que soit l'unité : un an.
pub const MAX_SHIFT_HOURS: u32 = 366 * 24;
/// Garde-fou des énumérations de relèves sur une période.
const MAX_BOUNDARIES: usize = 20_000;

/// Définition d'un planning, stockée en JSON dans `oncall_schedules.definition`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Definition {
    /// Fuseau IANA (« Europe/Paris »). Invalide ou vide : UTC.
    pub timezone: String,
    /// Prévenir les personnes au début et à la fin de leur garde.
    pub handoff_notifications: bool,
    pub layers: Vec<Layer>,
}

/// Une couche de rotation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layer {
    #[serde(default)]
    pub name: String,
    pub rotation: Rotation,
    /// Comptes, dans l'ordre de la rotation. Un même compte peut y figurer
    /// plusieurs fois.
    #[serde(default)]
    pub participants: Vec<i64>,
    /// Vide : la couche est active en permanence ; sinon seulement dans ces
    /// fenêtres, lues dans le fuseau du planning.
    #[serde(default)]
    pub restrictions: Vec<Window>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RotationKind {
    Daily,
    Weekly,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Unit {
    Hours,
    #[default]
    Days,
    Weeks,
}

/// Rotation d'une couche.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rotation {
    pub kind: RotationKind,
    /// Première relève, à l'heure locale du planning. Son heure est l'heure de
    /// relève ; son jour de la semaine, celui de la relève hebdomadaire.
    #[serde(with = "local_minutes")]
    pub start: NaiveDateTime,
    /// Rotation sur mesure : longueur d'une garde, dans `unit`.
    #[serde(default = "one")]
    pub length: u32,
    #[serde(default)]
    pub unit: Unit,
}

fn one() -> u32 {
    1
}

/// Fenêtre hebdomadaire d'une couche restreinte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Window {
    /// 0 = lundi … 6 = dimanche : le jour où la fenêtre s'ouvre.
    pub days: Vec<u8>,
    pub start_minute: u32,
    /// Fin inférieure ou égale au début : la fenêtre passe minuit (18 h → 9 h).
    /// Début et fin égaux : la journée entière.
    pub end_minute: u32,
}

/// Remplacement ponctuel, tel que le calcul en a besoin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Override {
    pub id: i64,
    /// Qui prend la garde.
    pub user_id: i64,
    /// `Some(y)` : seulement quand `y` serait d'astreinte ; `None` : quiconque.
    pub replaces: Option<i64>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
}

/// Qui est d'astreinte, et pourquoi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Slot {
    pub user_id: i64,
    /// Couche qui l'a désigné, `None` pour un remplacement.
    pub layer: Option<usize>,
    pub override_id: Option<i64>,
}

/// La garde en cours : qui, depuis quand, jusqu'à quand (quand c'est connu).
pub type Current = (Slot, Option<DateTime<Utc>>, Option<DateTime<Utc>>);

/// Une garde continue : même personne, même origine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Shift {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub user_id: i64,
    pub layer: Option<usize>,
    pub override_id: Option<i64>,
}

/// (Dé)sérialisation de `YYYY-MM-DDTHH:MM`, ce que produit un
/// `<input type="datetime-local">` ; les secondes sont tolérées à la lecture.
mod local_minutes {
    use chrono::NaiveDateTime;
    use serde::{Deserialize, Deserializer, Serializer};

    const FORMAT: &str = "%Y-%m-%dT%H:%M";

    pub fn serialize<S: Serializer>(value: &NaiveDateTime, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&value.format(FORMAT).to_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<NaiveDateTime, D::Error> {
        let raw = String::deserialize(d)?;
        super::parse_local(&raw).ok_or_else(|| {
            serde::de::Error::custom(format!(
                "\"{raw}\" is not a local date and time (YYYY-MM-DDTHH:MM)"
            ))
        })
    }
}

/// Lit une date et heure locale, avec ou sans secondes.
pub fn parse_local(raw: &str) -> Option<NaiveDateTime> {
    let raw = raw.trim();
    NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M")
        .or_else(|_| NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S"))
        .ok()
}

/// Fuseau nommé, `None` s'il est inconnu.
pub fn parse_tz(name: &str) -> Option<Tz> {
    Tz::from_str(name.trim()).ok()
}

/// Instant absolu où l'horloge murale de `tz` marque `naive`.
///
/// L'heure qui existe deux fois (passage à l'heure d'hiver) est prise à sa
/// première occurrence ; l'heure qui n'existe pas (passage à l'heure d'été)
/// est repoussée au premier quart d'heure qui existe, c'est-à-dire au moment
/// où l'horloge repart.
pub fn local_instant(tz: Tz, naive: NaiveDateTime) -> DateTime<Utc> {
    if let Some(at) = tz.from_local_datetime(&naive).earliest() {
        return at.with_timezone(&Utc);
    }
    (1..=16)
        .find_map(|quarters| {
            tz.from_local_datetime(&(naive + TimeDelta::minutes(15 * quarters))).earliest()
        })
        .map_or_else(|| Utc.from_utc_datetime(&naive), |at| at.with_timezone(&Utc))
}

fn local_date(tz: Tz, at: DateTime<Utc>) -> NaiveDate {
    at.with_timezone(&tz).date_naive()
}

fn at_minute(date: NaiveDate, minute: u32) -> NaiveDateTime {
    let minute = minute % MINUTES_PER_DAY;
    date.and_time(NaiveTime::from_hms_opt(minute / 60, minute % 60, 0).unwrap_or(NaiveTime::MIN))
}

/// Pas d'une rotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pace {
    /// Temps absolu.
    Hours(i64),
    /// Jours de calendrier local, à heure murale constante.
    Days(i64),
}

impl Rotation {
    fn pace(&self) -> Pace {
        let length = i64::from(self.length.max(1));
        match self.kind {
            RotationKind::Daily => Pace::Days(1),
            RotationKind::Weekly => Pace::Days(7),
            RotationKind::Custom => match self.unit {
                Unit::Hours => Pace::Hours(length),
                Unit::Days => Pace::Days(length),
                Unit::Weeks => Pace::Days(7 * length),
            },
        }
    }

    /// Début de la garde n° `k` (0 : la première relève).
    pub fn shift_start(&self, tz: Tz, k: i64) -> DateTime<Utc> {
        match self.pace() {
            Pace::Hours(hours) => {
                local_instant(tz, self.start) + TimeDelta::hours(hours.saturating_mul(k))
            }
            Pace::Days(days) => {
                let date = self
                    .start
                    .date()
                    .checked_add_signed(TimeDelta::days(days.saturating_mul(k)))
                    .unwrap_or(self.start.date());
                local_instant(tz, date.and_time(self.start.time()))
            }
        }
    }

    /// Numéro de la garde en cours à `at`, `None` avant la première relève.
    pub fn index_at(&self, tz: Tz, at: DateTime<Utc>) -> Option<i64> {
        let first = self.shift_start(tz, 0);
        if at < first {
            return None;
        }
        let mut k = match self.pace() {
            Pace::Hours(hours) => (at - first).num_seconds() / (hours * 3600),
            Pace::Days(days) => {
                (local_date(tz, at) - self.start.date()).num_days().div_euclid(days)
            }
        };
        // L'estimation peut se tromper d'une garde autour de l'heure de relève
        // (avant 9 h le jour de la relève, on est encore dans la précédente) :
        // on la corrige sur les bornes réelles.
        while k > 0 && self.shift_start(tz, k) > at {
            k -= 1;
        }
        while self.shift_start(tz, k + 1) <= at {
            k += 1;
        }
        Some(k)
    }
}

impl Window {
    /// Bornes de la fenêtre qui s'ouvre ce jour local-là, s'il est retenu.
    pub fn span_on(&self, tz: Tz, date: NaiveDate) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
        let weekday = date.weekday().num_days_from_monday() as u8;
        if !self.days.contains(&weekday) {
            return None;
        }
        let start_minute = self.start_minute % MINUTES_PER_DAY;
        let end_minute = self.end_minute % MINUTES_PER_DAY;
        let end_date = if end_minute > start_minute { date } else { date.succ_opt()? };
        let start = local_instant(tz, at_minute(date, start_minute));
        let end = local_instant(tz, at_minute(end_date, end_minute));
        (end > start).then_some((start, end))
    }

    pub fn contains(&self, tz: Tz, at: DateTime<Utc>) -> bool {
        let today = local_date(tz, at);
        [Some(today), today.pred_opt()]
            .into_iter()
            .flatten()
            .filter_map(|date| self.span_on(tz, date))
            .any(|(start, end)| at >= start && at < end)
    }
}

impl Layer {
    fn active(&self, tz: Tz, at: DateTime<Utc>) -> bool {
        self.restrictions.is_empty() || self.restrictions.iter().any(|w| w.contains(tz, at))
    }

    /// Participant de cette couche à `at`, s'il y en a un.
    pub fn who(&self, tz: Tz, at: DateTime<Utc>) -> Option<i64> {
        if self.participants.is_empty() || !self.active(tz, at) {
            return None;
        }
        let k = self.rotation.index_at(tz, at)?;
        let n = self.participants.len() as i64;
        self.participants.get(k.rem_euclid(n) as usize).copied()
    }

    /// Instants où cette couche peut changer de titulaire dans `]from, to[`.
    fn boundaries(
        &self,
        tz: Tz,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        cuts: &mut BTreeSet<DateTime<Utc>>,
    ) {
        let mut push = |at: DateTime<Utc>| {
            if at > from && at < to {
                cuts.insert(at);
            }
        };
        let first = self.rotation.index_at(tz, from).unwrap_or(0);
        for k in (first..).take(MAX_BOUNDARIES) {
            let at = self.rotation.shift_start(tz, k);
            if at >= to {
                break;
            }
            push(at);
        }
        if self.restrictions.is_empty() {
            return;
        }
        let Some(mut date) = local_date(tz, from).pred_opt() else { return };
        let last = local_date(tz, to);
        let mut guard = 0;
        while date <= last && guard < MAX_BOUNDARIES {
            for window in &self.restrictions {
                if let Some((start, end)) = window.span_on(tz, date) {
                    push(start);
                    push(end);
                }
            }
            let Some(next) = date.succ_opt() else { break };
            date = next;
            guard += 1;
        }
    }
}

impl Definition {
    pub fn tz(&self) -> Tz {
        parse_tz(&self.timezone).unwrap_or(Tz::UTC)
    }

    /// Le titulaire désigné par les couches seules.
    fn layered(&self, tz: Tz, at: DateTime<Utc>) -> Option<Slot> {
        self.layers.iter().enumerate().rev().find_map(|(index, layer)| {
            layer.who(tz, at).map(|user_id| Slot { user_id, layer: Some(index), override_id: None })
        })
    }

    /// Qui est d'astreinte à `at`. Le remplacement le plus récent l'emporte.
    pub fn on_call(&self, overrides: &[Override], at: DateTime<Utc>) -> Option<Slot> {
        let base = self.layered(self.tz(), at);
        let mut active: Vec<&Override> =
            overrides.iter().filter(|o| at >= o.starts_at && at < o.ends_at).collect();
        active.sort_by_key(|o| std::cmp::Reverse(o.id));
        for over in active {
            let applies = match over.replaces {
                None => true,
                Some(replaced) => base.is_some_and(|slot| slot.user_id == replaced),
            };
            if applies {
                return Some(Slot {
                    user_id: over.user_id,
                    layer: None,
                    override_id: Some(over.id),
                });
            }
        }
        base
    }

    /// Les gardes de `[from, to[`, consécutives fusionnées.
    pub fn shifts(
        &self,
        overrides: &[Override],
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Vec<Shift> {
        if to <= from {
            return Vec::new();
        }
        let tz = self.tz();
        let mut cuts = BTreeSet::from([from, to]);
        for layer in &self.layers {
            layer.boundaries(tz, from, to, &mut cuts);
        }
        for over in overrides {
            for at in [over.starts_at, over.ends_at] {
                if at > from && at < to {
                    cuts.insert(at);
                }
            }
        }
        let cuts: Vec<DateTime<Utc>> = cuts.into_iter().collect();
        let mut out: Vec<Shift> = Vec::new();
        for pair in cuts.windows(2) {
            let (start, end) = (pair[0], pair[1]);
            let Some(slot) = self.on_call(overrides, start) else { continue };
            if let Some(last) = out.last_mut()
                && last.end == start
                && last.user_id == slot.user_id
                && last.layer == slot.layer
                && last.override_id == slot.override_id
            {
                last.end = end;
                continue;
            }
            out.push(Shift {
                start,
                end,
                user_id: slot.user_id,
                layer: slot.layer,
                override_id: slot.override_id,
            });
        }
        out
    }

    /// Les gardes d'une seule couche, sans remplacement : la ligne de cette
    /// couche dans le calendrier.
    pub fn layer_shifts(&self, index: usize, from: DateTime<Utc>, to: DateTime<Utc>) -> Vec<Shift> {
        let Some(layer) = self.layers.get(index) else { return Vec::new() };
        let alone = Definition { layers: vec![layer.clone()], ..self.clone() };
        alone
            .shifts(&[], from, to)
            .into_iter()
            .map(|shift| Shift { layer: Some(index), ..shift })
            .collect()
    }

    /// La garde en cours à `now`, avec ses bornes quand elles tombent dans un
    /// horizon raisonnable (deux semaines en arrière, cinq en avant).
    pub fn current(&self, overrides: &[Override], now: DateTime<Utc>) -> Option<Current> {
        let slot = self.on_call(overrides, now)?;
        let from = now - TimeDelta::days(14);
        let to = now + TimeDelta::days(35);
        let shift =
            self.shifts(overrides, from, to).into_iter().find(|s| s.start <= now && now < s.end)?;
        Some((
            slot,
            (shift.start > from).then_some(shift.start),
            (shift.end < to).then_some(shift.end),
        ))
    }

    /// Vérifie une définition soumise ; le message est destiné à l'utilisateur.
    pub fn validate(&self) -> Result<(), String> {
        if parse_tz(&self.timezone).is_none() {
            return Err(format!(
                "Unknown time zone \"{}\" (expected e.g. Europe/Paris).",
                self.timezone
            ));
        }
        if self.layers.is_empty() {
            return Err("A schedule needs at least one layer.".into());
        }
        if self.layers.len() > MAX_LAYERS {
            return Err(format!("A schedule has at most {MAX_LAYERS} layers."));
        }
        for (index, layer) in self.layers.iter().enumerate() {
            let name = if layer.name.trim().is_empty() {
                format!("Layer {}", index + 1)
            } else {
                layer.name.trim().to_string()
            };
            if layer.participants.is_empty() {
                return Err(format!("{name}: add at least one participant."));
            }
            if layer.participants.len() > MAX_PARTICIPANTS {
                return Err(format!("{name}: at most {MAX_PARTICIPANTS} participants."));
            }
            if layer.rotation.kind == RotationKind::Custom {
                let hours = match layer.rotation.unit {
                    Unit::Hours => layer.rotation.length,
                    Unit::Days => layer.rotation.length.saturating_mul(24),
                    Unit::Weeks => layer.rotation.length.saturating_mul(24 * 7),
                };
                if layer.rotation.length == 0 || hours > MAX_SHIFT_HOURS {
                    return Err(format!("{name}: a shift lasts from one hour to one year."));
                }
            }
            if layer.restrictions.len() > MAX_WINDOWS {
                return Err(format!("{name}: at most {MAX_WINDOWS} time windows."));
            }
            for window in &layer.restrictions {
                if window.days.is_empty() || window.days.iter().any(|day| *day > 6) {
                    return Err(format!(
                        "{name}: each time window needs days between Monday and Sunday."
                    ));
                }
                if window.start_minute >= MINUTES_PER_DAY || window.end_minute >= MINUTES_PER_DAY {
                    return Err(format!("{name}: times must be between 00:00 and 23:59."));
                }
            }
        }
        Ok(())
    }
}

/// Minute du jour local d'un instant, pour les messages.
pub fn local_label(tz: Tz, at: DateTime<Utc>) -> String {
    let local = at.with_timezone(&tz);
    format!(
        "{} {:02}:{:02} {}",
        local.format("%a %Y-%m-%d"),
        local.hour(),
        local.minute(),
        tz.name()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALICE: i64 = 1;
    const BOB: i64 = 2;
    const CAROL: i64 = 3;

    fn utc(raw: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(raw).expect("valid instant").with_timezone(&Utc)
    }

    fn local(raw: &str) -> NaiveDateTime {
        parse_local(raw).expect("valid local datetime")
    }

    fn layer(kind: RotationKind, start: &str, participants: &[i64]) -> Layer {
        Layer {
            name: String::new(),
            rotation: Rotation { kind, start: local(start), length: 1, unit: Unit::Days },
            participants: participants.to_vec(),
            restrictions: Vec::new(),
        }
    }

    fn schedule(timezone: &str, layers: Vec<Layer>) -> Definition {
        Definition { timezone: timezone.to_string(), handoff_notifications: false, layers }
    }

    fn who(def: &Definition, overrides: &[Override], at: &str) -> Option<i64> {
        def.on_call(overrides, utc(at)).map(|slot| slot.user_id)
    }

    #[test]
    fn une_rotation_hebdomadaire_passe_la_main_le_lundi_a_9_h_locales() {
        // Lundi 5 octobre 2026, 9 h à Paris (UTC+2).
        let def = schedule(
            "Europe/Paris",
            vec![layer(RotationKind::Weekly, "2026-10-05T09:00", &[ALICE, BOB, CAROL])],
        );
        assert_eq!(who(&def, &[], "2026-10-05T06:59:00Z"), None, "before the first handoff");
        assert_eq!(who(&def, &[], "2026-10-05T07:00:00Z"), Some(ALICE));
        assert_eq!(who(&def, &[], "2026-10-12T06:59:59Z"), Some(ALICE), "Monday 08:59 Paris");
        assert_eq!(who(&def, &[], "2026-10-12T07:00:00Z"), Some(BOB));
        assert_eq!(who(&def, &[], "2026-10-19T07:30:00Z"), Some(CAROL));
        // Après le passage à l'heure d'hiver (25 octobre), 9 h Paris = 8 h UTC.
        assert_eq!(who(&def, &[], "2026-10-26T07:30:00Z"), Some(CAROL), "08:30 Paris, CET");
        assert_eq!(who(&def, &[], "2026-10-26T08:00:00Z"), Some(ALICE), "back to the start");
    }

    #[test]
    fn la_releve_quotidienne_reste_a_l_heure_murale_aux_changements_d_heure() {
        let rotation = Rotation {
            kind: RotationKind::Daily,
            start: local("2026-03-27T09:00"),
            length: 1,
            unit: Unit::Days,
        };
        let tz = parse_tz("Europe/Paris").expect("tz");
        // 28 mars 9 h CET, 29 mars 9 h CEST (le changement a lieu à 2 h).
        let before = rotation.shift_start(tz, 1);
        let after = rotation.shift_start(tz, 2);
        assert_eq!(before, utc("2026-03-28T08:00:00Z"));
        assert_eq!(after, utc("2026-03-29T07:00:00Z"));
        assert_eq!(after - before, TimeDelta::hours(23), "the short day is a short shift");
        // Et l'automne : 24 → 25 octobre, journée de 25 heures.
        let autumn = Rotation { start: local("2026-10-24T09:00"), ..rotation };
        assert_eq!(autumn.shift_start(tz, 1) - autumn.shift_start(tz, 0), TimeDelta::hours(25));
        assert_eq!(autumn.index_at(tz, utc("2026-10-25T07:59:00Z")), Some(0), "08:59 CET");
        assert_eq!(autumn.index_at(tz, utc("2026-10-25T08:00:00Z")), Some(1), "09:00 CET");
    }

    #[test]
    fn une_rotation_en_heures_compte_en_temps_reel() {
        let mut twelve = layer(RotationKind::Custom, "2026-10-24T20:00", &[ALICE, BOB]);
        twelve.rotation.length = 12;
        twelve.rotation.unit = Unit::Hours;
        let def = schedule("Europe/Paris", vec![twelve]);
        // 24 oct. 20 h CEST = 18 h UTC. Douze heures réelles plus tard : 6 h UTC,
        // soit 7 h locales après le recul de l'horloge, et non 8 h.
        assert_eq!(who(&def, &[], "2026-10-25T05:59:00Z"), Some(ALICE));
        assert_eq!(who(&def, &[], "2026-10-25T06:00:00Z"), Some(BOB));
        assert_eq!(who(&def, &[], "2026-10-25T18:00:00Z"), Some(ALICE));
    }

    #[test]
    fn une_releve_dans_l_heure_qui_n_existe_pas_attend_que_l_horloge_reparte() {
        let tz = parse_tz("Europe/Paris").expect("tz");
        // 29 mars 2026, 2 h 30 n'existe pas à Paris : la relève a lieu à 3 h CEST.
        assert_eq!(local_instant(tz, local("2026-03-29T02:30")), utc("2026-03-29T01:00:00Z"));
        // 25 octobre, 2 h 30 existe deux fois : la première, en CEST.
        assert_eq!(local_instant(tz, local("2026-10-25T02:30")), utc("2026-10-25T00:30:00Z"));
    }

    #[test]
    fn la_couche_du_dessus_l_emporte_la_ou_elle_est_active() {
        // Base : Alice et Bob, une semaine chacun. Par-dessus : Carol les soirs de
        // semaine (18 h → 9 h) et les week-ends entiers.
        let base = layer(RotationKind::Weekly, "2026-10-05T09:00", &[ALICE, BOB]);
        let mut nights = layer(RotationKind::Weekly, "2026-10-05T09:00", &[CAROL]);
        nights.restrictions = vec![
            Window { days: vec![0, 1, 2, 3, 4], start_minute: 18 * 60, end_minute: 9 * 60 },
            Window { days: vec![5, 6], start_minute: 0, end_minute: 0 },
        ];
        let def = schedule("Europe/Paris", vec![base, nights]);
        assert_eq!(who(&def, &[], "2026-10-07T10:00:00Z"), Some(ALICE), "Wednesday noon");
        assert_eq!(who(&def, &[], "2026-10-07T17:00:00Z"), Some(CAROL), "Wednesday 19:00");
        assert_eq!(who(&def, &[], "2026-10-08T06:30:00Z"), Some(CAROL), "Thursday 08:30");
        assert_eq!(who(&def, &[], "2026-10-08T07:00:00Z"), Some(ALICE), "Thursday 09:00");
        assert_eq!(who(&def, &[], "2026-10-10T12:00:00Z"), Some(CAROL), "Saturday");
        // Vendredi 18 h → samedi 0 h : fenêtre du soir ; samedi : fenêtre du
        // week-end ; dimanche 23 h 59 encore Carol ; lundi 0 h → 9 h : la fenêtre
        // du soir ne s'ouvre pas le dimanche, retour à la base (Bob cette semaine
        // n'a pas encore commencé : relève lundi 9 h).
        assert_eq!(who(&def, &[], "2026-10-11T21:59:00Z"), Some(CAROL), "Sunday 23:59");
        assert_eq!(who(&def, &[], "2026-10-11T22:30:00Z"), Some(ALICE), "Monday 00:30");
        assert_eq!(who(&def, &[], "2026-10-12T07:00:00Z"), Some(BOB), "Monday 09:00");
    }

    fn cover(id: i64, user: i64, replaces: Option<i64>, from: &str, to: &str) -> Override {
        Override { id, user_id: user, replaces, starts_at: utc(from), ends_at: utc(to) }
    }

    #[test]
    fn un_remplacement_prend_la_garde_de_la_personne_nommee_seulement() {
        let def =
            schedule("UTC", vec![layer(RotationKind::Daily, "2026-10-01T00:00", &[ALICE, BOB])]);
        // Carol remplace Bob le 2 et le 3 octobre : Bob est de garde le 2, Alice le 3.
        let overrides =
            [cover(10, CAROL, Some(BOB), "2026-10-02T00:00:00Z", "2026-10-04T00:00:00Z")];
        assert_eq!(who(&def, &overrides, "2026-10-02T12:00:00Z"), Some(CAROL));
        assert_eq!(who(&def, &overrides, "2026-10-03T12:00:00Z"), Some(ALICE), "Alice's day");
        // Sans personne nommée, le remplacement couvre tout le monde.
        let anyone = [cover(11, CAROL, None, "2026-10-03T06:00:00Z", "2026-10-03T08:00:00Z")];
        assert_eq!(who(&def, &anyone, "2026-10-03T07:00:00Z"), Some(CAROL));
        assert_eq!(who(&def, &anyone, "2026-10-03T08:00:00Z"), Some(ALICE), "end excluded");
        // Deux remplacements qui se chevauchent : le plus récent gagne.
        let both = [
            cover(12, CAROL, None, "2026-10-03T00:00:00Z", "2026-10-04T00:00:00Z"),
            cover(13, BOB, None, "2026-10-03T10:00:00Z", "2026-10-03T11:00:00Z"),
        ];
        assert_eq!(who(&def, &both, "2026-10-03T10:30:00Z"), Some(BOB));
        let slot = def.on_call(&both, utc("2026-10-03T10:30:00Z")).expect("slot");
        assert_eq!(slot.override_id, Some(13));
        assert_eq!(slot.layer, None);
    }

    #[test]
    fn les_gardes_sont_decoupees_par_les_releves_et_les_remplacements() {
        let def =
            schedule("UTC", vec![layer(RotationKind::Daily, "2026-10-01T09:00", &[ALICE, BOB])]);
        let overrides = [cover(5, CAROL, None, "2026-10-02T12:00:00Z", "2026-10-02T14:00:00Z")];
        let shifts =
            def.shifts(&overrides, utc("2026-10-01T00:00:00Z"), utc("2026-10-03T00:00:00Z"));
        let summary: Vec<(i64, String, String)> = shifts
            .iter()
            .map(|s| {
                (s.user_id, s.start.format("%dT%H").to_string(), s.end.format("%dT%H").to_string())
            })
            .collect();
        assert_eq!(
            summary,
            vec![
                (ALICE, "01T09".into(), "02T09".into()),
                (BOB, "02T09".into(), "02T12".into()),
                (CAROL, "02T12".into(), "02T14".into()),
                (BOB, "02T14".into(), "03T00".into()),
            ],
            "nobody before the first handoff, then one shift per day"
        );
        // Un seul participant : les relèves ne coupent rien.
        let solo = schedule("UTC", vec![layer(RotationKind::Daily, "2026-10-01T09:00", &[ALICE])]);
        let one = solo.shifts(&[], utc("2026-10-02T00:00:00Z"), utc("2026-10-06T00:00:00Z"));
        assert_eq!(one.len(), 1);
    }

    #[test]
    fn la_garde_en_cours_donne_ses_bornes() {
        let def = schedule(
            "America/New_York",
            vec![layer(RotationKind::Weekly, "2026-11-02T08:00", &[ALICE, BOB])],
        );
        let (slot, since, until) = def.current(&[], utc("2026-11-05T12:00:00Z")).expect("on call");
        assert_eq!(slot.user_id, ALICE);
        // 8 h EST = 13 h UTC (l'heure d'hiver y commence le 1er novembre).
        assert_eq!(since, Some(utc("2026-11-02T13:00:00Z")));
        assert_eq!(until, Some(utc("2026-11-09T13:00:00Z")));
    }

    #[test]
    fn une_definition_incoherente_est_expliquee() {
        let mut def = schedule(
            "Mars/Olympus",
            vec![layer(RotationKind::Daily, "2026-10-01T09:00", &[ALICE])],
        );
        assert!(def.validate().unwrap_err().contains("time zone"));
        def.timezone = "Europe/Paris".into();
        assert!(def.validate().is_ok());
        def.layers[0].participants.clear();
        assert!(def.validate().unwrap_err().contains("participant"));
        def.layers[0].participants.push(BOB);
        def.layers[0].restrictions.push(Window { days: vec![7], start_minute: 0, end_minute: 60 });
        assert!(def.validate().unwrap_err().contains("Monday"));
    }

    #[test]
    fn la_definition_se_lit_comme_l_interface_l_ecrit() {
        let raw = r#"{"timezone":"Europe/Paris","handoff_notifications":true,"layers":[
            {"name":"Base","rotation":{"kind":"custom","start":"2026-10-05T09:00","length":2,"unit":"weeks"},
             "participants":[1,2],"restrictions":[{"days":[0,1,2,3,4],"start_minute":540,"end_minute":1080}]}]}"#;
        let def: Definition = serde_json::from_str(raw).expect("parsed");
        assert_eq!(def.layers[0].rotation.unit, Unit::Weeks);
        let back = serde_json::to_value(&def).expect("serialised");
        assert_eq!(back["layers"][0]["rotation"]["start"], "2026-10-05T09:00");
        // Deux semaines par garde : Bob prend la main le 19.
        assert_eq!(who(&def, &[], "2026-10-13T10:00:00Z"), Some(ALICE));
        assert_eq!(who(&def, &[], "2026-10-19T10:00:00Z"), Some(BOB));
        assert_eq!(who(&def, &[], "2026-10-19T17:00:00Z"), None, "19:00, outside the window");
    }
}
