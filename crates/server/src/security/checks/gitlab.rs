//! GitLab self-managé : les deux réglages d'authentification exposés par
//! `/api/v4/application/settings`, et les migrations de base de données
//! laissées en attente après une mise à jour. Le collecteur ne lit, par cette
//! voie, ni les jetons d'accès laissés ouverts, ni l'état du pare-feu Git
//! (`push_rule`) : ces réglages demandent un jeton administrateur distinct et
//! sont listés dans la documentation plutôt que devinés ici.

use super::super::{Category, Check, CheckDef, Facts, SecurityProvider, Severity};
use super::zero_everywhere;

pub const TWO_FACTOR_REQUIRED: CheckDef = CheckDef {
    id: "gitlab.two_factor_required",
    title: "Two-factor authentication required",
    category: Category::Authentication,
    severity: Severity::High,
    reference: "https://docs.gitlab.com/ee/security/two_factor_authentication.html",
    remediation: "Enable \"Require all users to set up two-factor authentication\" in Admin area \
                  > Settings > General > Sign-up restrictions.",
};

pub const SIGNUP_DISABLED: CheckDef = CheckDef {
    id: "gitlab.signup_disabled",
    title: "Public sign-up disabled",
    category: Category::Exposure,
    severity: Severity::Medium,
    reference: "https://docs.gitlab.com/ee/administration/settings/sign_up_restrictions.html",
    remediation: "Turn off \"Sign-up enabled\" in Admin area > Settings > General > Sign-up \
                  restrictions unless this instance is meant to be open to the public.",
};

pub const MIGRATIONS_PENDING: CheckDef = CheckDef {
    id: "gitlab.migrations_pending",
    title: "Database migrations applied",
    category: Category::Patching,
    severity: Severity::Medium,
    reference: "https://docs.gitlab.com/ee/update/background_migrations.html",
    remediation: "Run the pending migrations (gitlab-rake db:migrate, or restart the instance \
                  after an upgrade) before the next release is installed on top of them.",
};

pub const DEFS: &[&CheckDef] = &[&TWO_FACTOR_REQUIRED, &SIGNUP_DISABLED, &MIGRATIONS_PENDING];

pub struct Gitlab;

impl SecurityProvider for Gitlab {
    fn kinds(&self) -> &'static [&'static str] {
        &["gitlab"]
    }

    fn metrics(&self) -> &'static [&'static str] {
        &["gitlab_two_factor_required", "gitlab_signup_enabled", "gitlab_migrations_pending"]
    }

    fn evaluate(&self, _kind: &str, facts: &Facts) -> Vec<Check> {
        vec![two_factor(facts), signup(facts), migrations(facts)]
    }
}

fn two_factor(facts: &Facts) -> Check {
    match facts.max("gitlab_two_factor_required") {
        None => TWO_FACTOR_REQUIRED.unknown("Not collected (a non-administrator token skips it)."),
        Some(value) => TWO_FACTOR_REQUIRED
            .verdict(value >= 1.0, if value >= 1.0 { "Required" } else { "Not required" }),
    }
}

fn signup(facts: &Facts) -> Check {
    match facts.max("gitlab_signup_enabled") {
        None => SIGNUP_DISABLED.unknown("Not collected (a non-administrator token skips it)."),
        Some(value) => {
            SIGNUP_DISABLED.verdict(value < 1.0, if value >= 1.0 { "Enabled" } else { "Disabled" })
        }
    }
}

fn migrations(facts: &Facts) -> Check {
    zero_everywhere(
        &MIGRATIONS_PENDING,
        facts,
        "gitlab_migrations_pending",
        "",
        "migration",
        "Not collected (a non-administrator token skips it).",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::Outcome;

    #[test]
    fn le_second_facteur_et_l_auto_inscription() {
        let secure = Facts::from_pairs(&[
            ("gitlab_two_factor_required", &[], 1.0),
            ("gitlab_signup_enabled", &[], 0.0),
        ]);
        let checks = Gitlab.evaluate("gitlab", &secure);
        assert_eq!(checks[0].result, Outcome::Pass);
        assert_eq!(checks[1].result, Outcome::Pass);

        let open = Facts::from_pairs(&[
            ("gitlab_two_factor_required", &[], 0.0),
            ("gitlab_signup_enabled", &[], 1.0),
        ]);
        let checks = Gitlab.evaluate("gitlab", &open);
        assert_eq!(checks[0].result, Outcome::Fail);
        assert_eq!(checks[1].result, Outcome::Fail);
    }

    #[test]
    fn les_migrations_en_attente_echouent_le_controle() {
        let pending = Facts::from_pairs(&[("gitlab_migrations_pending", &[], 2.0)]);
        assert_eq!(Gitlab.evaluate("gitlab", &pending)[2].result, Outcome::Fail);
        let none = Facts::from_pairs(&[("gitlab_migrations_pending", &[], 0.0)]);
        assert_eq!(Gitlab.evaluate("gitlab", &none)[2].result, Outcome::Pass);
    }

    #[test]
    fn sans_compte_administrateur_tout_est_inconnu() {
        let checks = Gitlab.evaluate("gitlab", &Facts::default());
        assert!(checks.iter().all(|c| c.result == Outcome::Unknown));
    }
}
