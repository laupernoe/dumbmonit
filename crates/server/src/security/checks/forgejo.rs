//! Forgejo et Gitea : les tâches planifiées de l'administration qui ne
//! tournent plus (nettoyage, vérification des dépôts, recherche de mise à
//! jour). Le collecteur ne lit, par cette voie, ni l'inscription ouverte, ni
//! le second facteur : Forgejo et Gitea ne les exposent pas par l'API, à la
//! différence de GitLab.

use super::super::{Category, Check, CheckDef, Facts, SecurityProvider, Severity};
use super::zero_everywhere;

pub const CRON_TASKS_RUNNING: CheckDef = CheckDef {
    id: "forgejo.cron_tasks_running",
    title: "Scheduled tasks run on time",
    category: Category::Configuration,
    severity: Severity::Medium,
    reference: "https://forgejo.org/docs/latest/admin/config-cheat-sheet/#cron-cron",
    remediation: "Check the Forgejo or Gitea process: a scheduled task (repository check, \
                  update checker, cleanup) stopped running at its own interval.",
};

pub const DEFS: &[&CheckDef] = &[&CRON_TASKS_RUNNING];

pub struct Forgejo;

impl SecurityProvider for Forgejo {
    fn kinds(&self) -> &'static [&'static str] {
        &["forgejo"]
    }

    fn metrics(&self) -> &'static [&'static str] {
        &["forgejo_cron_task_overdue"]
    }

    fn evaluate(&self, _kind: &str, facts: &Facts) -> Vec<Check> {
        vec![zero_everywhere(
            &CRON_TASKS_RUNNING,
            facts,
            "forgejo_cron_task_overdue",
            "task",
            "",
            "Not collected (a non-administrator token skips it).",
        )]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::Outcome;

    #[test]
    fn une_tache_en_retard_est_nommee() {
        let facts =
            Facts::from_pairs(&[("forgejo_cron_task_overdue", &[("task", "update_checker")], 1.0)]);
        let check = &Forgejo.evaluate("forgejo", &facts)[0];
        assert_eq!((check.result, check.evidence.as_str()), (Outcome::Fail, "update_checker"));
    }

    #[test]
    fn a_l_heure_le_controle_reussit() {
        let facts = Facts::from_pairs(&[
            ("forgejo_cron_task_overdue", &[("task", "update_checker")], 0.0),
            ("forgejo_cron_task_overdue", &[("task", "repo_health_check")], 0.0),
        ]);
        assert_eq!(Forgejo.evaluate("forgejo", &facts)[0].result, Outcome::Pass);
    }

    #[test]
    fn sans_donnee_le_controle_est_inconnu() {
        assert_eq!(Forgejo.evaluate("forgejo", &Facts::default())[0].result, Outcome::Unknown);
    }
}
