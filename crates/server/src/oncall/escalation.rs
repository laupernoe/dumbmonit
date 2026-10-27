//! Politiques d'escalade : qui appeler, dans quel ordre, et quand s'arrêter.
//!
//! Une politique est une suite d'étapes. Chaque étape appelle ses cibles — la
//! personne d'astreinte d'un planning, une personne nommée, un canal — puis
//! attend son délai ; sans acquittement, l'étape suivante part. Après la
//! dernière, la chaîne entière peut être rejouée `repeat` fois, puis
//! l'escalade s'épuise. Un acquittement, par n'importe lequel des
//! destinataires et par n'importe quel moyen, l'arrête ; la résolution aussi.
//!
//! Le module est pur : [`tick`] reçoit l'état de l'escalade et celui de
//! l'alerte, et dit quoi faire. Le moteur (`oncall::runner`) applique.

use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};

use crate::alerting::model::Severity;
use crate::alerting::silence::Schedule;

/// Étapes au plus par politique.
pub const MAX_STEPS: usize = 10;
/// Cibles au plus par étape.
pub const MAX_TARGETS: usize = 10;
/// Répétitions au plus de la chaîne complète.
pub const MAX_REPEAT: u32 = 9;
/// Délai maximal d'une étape : une journée.
pub const MAX_DELAY_MINUTES: u32 = 24 * 60;

/// Définition d'une politique, stockée en JSON dans `oncall_policies.definition`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PolicyDef {
    /// Nombre de fois où la chaîne est rejouée après sa dernière étape.
    pub repeat: u32,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Step {
    /// Attente après cette étape avant la suivante (ou la répétition).
    #[serde(default = "default_delay")]
    pub delay_minutes: u32,
    #[serde(default)]
    pub targets: Vec<Target>,
}

fn default_delay() -> u32 {
    15
}

/// Une cible d'étape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Target {
    /// La personne d'astreinte de ce planning au moment de l'appel.
    Schedule { id: i64 },
    /// Un compte, qu'il soit d'astreinte ou non.
    User { id: i64 },
    /// Un canal de notification existant.
    Channel { id: i64 },
}

impl PolicyDef {
    pub fn validate(&self) -> Result<(), String> {
        if self.steps.is_empty() {
            return Err("A policy needs at least one step.".into());
        }
        if self.steps.len() > MAX_STEPS {
            return Err(format!("A policy has at most {MAX_STEPS} steps."));
        }
        if self.repeat > MAX_REPEAT {
            return Err(format!("A policy repeats at most {MAX_REPEAT} times."));
        }
        for (index, step) in self.steps.iter().enumerate() {
            if step.targets.is_empty() {
                return Err(format!("Step {}: add at least one target.", index + 1));
            }
            if step.targets.len() > MAX_TARGETS {
                return Err(format!("Step {}: at most {MAX_TARGETS} targets.", index + 1));
            }
            if step.delay_minutes == 0 || step.delay_minutes > MAX_DELAY_MINUTES {
                return Err(format!(
                    "Step {}: the delay is between 1 and {MAX_DELAY_MINUTES} minutes.",
                    index + 1
                ));
            }
        }
        Ok(())
    }
}

/// Où en est une escalade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    /// Prochaine étape à appeler ; `steps.len()` quand la chaîne est finie et
    /// qu'il ne reste qu'à attendre le dernier délai.
    pub next_step: usize,
    pub round: u32,
    pub next_at: DateTime<Utc>,
}

impl Progress {
    /// Une escalade qui démarre appelle sa première étape tout de suite.
    pub fn start(now: DateTime<Utc>) -> Self {
        Self { next_step: 0, round: 0, next_at: now }
    }
}

/// L'alerte telle que l'escalade la voit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AlertView {
    /// Encore en cours (`pending` ou `firing`).
    pub active: bool,
    /// Qui a acquitté, si quelqu'un l'a fait.
    pub acked_by: Option<String>,
    /// Qui l'a écartée (« j'ai compris »).
    pub dismissed_by: Option<String>,
    /// Couverte par un silence ou une mise en sourdine.
    pub silenced: bool,
}

/// Pourquoi une escalade s'arrête avant de s'épuiser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stop {
    Acked(String),
    Resolved,
    Dismissed(String),
    Silenced,
}

/// Décision d'un passage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tick {
    /// Rien à faire avant `next_at`.
    Wait,
    Stop(Stop),
    /// Appeler l'étape `step` (tour `round`), puis passer à `then`.
    Page {
        step: usize,
        round: u32,
        then: Progress,
    },
    /// La chaîne et ses répétitions sont passées sans acquittement.
    Exhausted,
}

/// Décide du sort d'une escalade à `now`.
///
/// Une seule étape par passage, même en retard (serveur arrêté pendant une
/// heure) : réveiller trois personnes dans la même seconde ne remplacerait pas
/// les délais que la politique voulait laisser à chacune.
pub fn tick(
    policy: &PolicyDef,
    progress: &Progress,
    alert: &AlertView,
    now: DateTime<Utc>,
) -> Tick {
    if !alert.active {
        return Tick::Stop(Stop::Resolved);
    }
    if let Some(by) = &alert.acked_by {
        return Tick::Stop(Stop::Acked(by.clone()));
    }
    if let Some(by) = &alert.dismissed_by {
        return Tick::Stop(Stop::Dismissed(by.clone()));
    }
    if alert.silenced {
        return Tick::Stop(Stop::Silenced);
    }
    if now < progress.next_at {
        return Tick::Wait;
    }
    let count = policy.steps.len();
    if progress.next_step >= count {
        return Tick::Exhausted;
    }
    let step = progress.next_step;
    let delay = TimeDelta::minutes(i64::from(policy.steps[step].delay_minutes.max(1)));
    let next_at = now + delay;
    let then = if step + 1 < count {
        Progress { next_step: step + 1, round: progress.round, next_at }
    } else if progress.round < policy.repeat {
        Progress { next_step: 0, round: progress.round + 1, next_at }
    } else {
        Progress { next_step: count, round: progress.round, next_at }
    };
    Tick::Page { step, round: progress.round, then }
}

/// Peut-on déranger cette personne pour cette alerte ?
///
/// D'astreinte, toujours. Hors astreinte, ses heures calmes ne laissent passer
/// que le critique.
pub fn may_disturb(
    quiet_hours: Option<&Schedule>,
    on_call: bool,
    severity: Severity,
    now: DateTime<Utc>,
) -> bool {
    on_call || severity >= Severity::Critical || !quiet_hours.is_some_and(|q| q.covers(now))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(minutes: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(1_790_000_000, 0).expect("instant") + TimeDelta::minutes(minutes)
    }

    fn firing() -> AlertView {
        AlertView { active: true, ..AlertView::default() }
    }

    fn two_steps(repeat: u32) -> PolicyDef {
        PolicyDef {
            repeat,
            steps: vec![
                Step { delay_minutes: 10, targets: vec![Target::Schedule { id: 1 }] },
                Step { delay_minutes: 20, targets: vec![Target::User { id: 7 }] },
            ],
        }
    }

    /// Rejoue une escalade minute par minute et rend les appels (minute, étape,
    /// tour) et l'issue.
    fn simulate(
        policy: &PolicyDef,
        minutes: i64,
        alert_at: impl Fn(i64) -> AlertView,
    ) -> (Vec<(i64, usize, u32)>, Option<Tick>) {
        let mut progress = Progress::start(at(0));
        let mut pages = Vec::new();
        for minute in 0..=minutes {
            match tick(policy, &progress, &alert_at(minute), at(minute)) {
                Tick::Wait => {}
                Tick::Page { step, round, then } => {
                    pages.push((minute, step, round));
                    progress = then;
                }
                end => return (pages, Some(end)),
            }
        }
        (pages, None)
    }

    #[test]
    fn les_etapes_s_enchainent_selon_leurs_delais_puis_la_chaine_se_repete() {
        let (pages, end) = simulate(&two_steps(1), 200, |_| firing());
        assert_eq!(
            pages,
            vec![(0, 0, 0), (10, 1, 0), (30, 0, 1), (40, 1, 1)],
            "step 1 at once, step 2 after 10 min, the chain again after 20 more"
        );
        assert_eq!(end, Some(Tick::Exhausted), "exhausted 20 min after the last page");
        let (_, before) = simulate(&two_steps(1), 59, |_| firing());
        assert_eq!(before, None, "still waiting on the last delay at minute 59");
    }

    #[test]
    fn l_acquittement_par_n_importe_qui_arrete_l_escalade() {
        let (pages, end) = simulate(&two_steps(3), 500, |minute| AlertView {
            acked_by: (minute >= 15).then(|| "bob".to_string()),
            ..firing()
        });
        assert_eq!(pages, vec![(0, 0, 0), (10, 1, 0)], "nobody is called after the ack");
        assert_eq!(end, Some(Tick::Stop(Stop::Acked("bob".into()))));
    }

    #[test]
    fn la_resolution_l_ecartement_et_le_silence_arretent_aussi() {
        let (pages, end) = simulate(&two_steps(0), 100, |minute| AlertView {
            active: minute < 5,
            ..AlertView::default()
        });
        assert_eq!(pages, vec![(0, 0, 0)]);
        assert_eq!(end, Some(Tick::Stop(Stop::Resolved)));

        let (_, end) = simulate(&two_steps(0), 100, |minute| AlertView {
            dismissed_by: (minute >= 3).then(|| "alice".to_string()),
            ..firing()
        });
        assert_eq!(end, Some(Tick::Stop(Stop::Dismissed("alice".into()))));

        let (_, end) =
            simulate(&two_steps(0), 100, |minute| AlertView { silenced: minute >= 3, ..firing() });
        assert_eq!(end, Some(Tick::Stop(Stop::Silenced)));
    }

    #[test]
    fn un_retard_ne_fait_partir_qu_une_etape_par_passage() {
        let policy = two_steps(0);
        let progress = Progress::start(at(0));
        // Le serveur se réveille une heure plus tard : une étape, pas deux.
        let Tick::Page { step, then, .. } = tick(&policy, &progress, &firing(), at(60)) else {
            panic!("expected a page")
        };
        assert_eq!(step, 0);
        assert_eq!(then.next_at, at(70), "the delay counts from the actual page");
        assert_eq!(tick(&policy, &then, &firing(), at(61)), Tick::Wait);
    }

    #[test]
    fn une_politique_modifiee_en_cours_ne_rappelle_pas_une_etape_disparue() {
        let shorter = PolicyDef { repeat: 0, steps: two_steps(0).steps[..1].to_vec() };
        let progress = Progress { next_step: 1, round: 0, next_at: at(10) };
        assert_eq!(tick(&shorter, &progress, &firing(), at(10)), Tick::Exhausted);
    }

    #[test]
    fn les_heures_calmes_ne_laissent_passer_que_le_critique_hors_astreinte() {
        let quiet = Schedule::Once { starts_at: at(0), ends_at: at(600) };
        assert!(!may_disturb(Some(&quiet), false, Severity::Warning, at(5)));
        assert!(may_disturb(Some(&quiet), false, Severity::Critical, at(5)));
        assert!(may_disturb(Some(&quiet), true, Severity::Info, at(5)), "on call: always");
        assert!(may_disturb(Some(&quiet), false, Severity::Info, at(700)), "after quiet hours");
        assert!(may_disturb(None, false, Severity::Info, at(5)));
    }

    #[test]
    fn une_politique_incoherente_est_expliquee() {
        assert!(PolicyDef::default().validate().unwrap_err().contains("at least one step"));
        let mut policy = two_steps(0);
        assert!(policy.validate().is_ok());
        policy.steps[1].delay_minutes = 0;
        assert!(policy.validate().unwrap_err().contains("Step 2"));
        policy.steps[1].delay_minutes = 5;
        policy.steps[0].targets.clear();
        assert!(policy.validate().unwrap_err().contains("Step 1"));
        let parsed: PolicyDef = serde_json::from_str(
            r#"{"repeat":2,"steps":[{"delay_minutes":5,"targets":[{"type":"schedule","id":3},{"type":"channel","id":9}]}]}"#,
        )
        .expect("parsed");
        assert_eq!(parsed.steps[0].targets[1], Target::Channel { id: 9 });
    }
}
