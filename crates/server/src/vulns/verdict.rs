//! Le verdict d'une CVE pour un composant : `vulnerable`, `patched` ou
//! `unknown` (« à vérifier »). Fonctions pures : chaque source a la sienne,
//! et le doute penche toujours vers « à vérifier », jamais vers une alerte.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use super::debian::Status;
use super::nvd::CpeRange;
use super::version::{Generic, debian_cmp, debian_valid, upstream_of};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Vulnerable,
    Patched,
    Unknown,
}

impl State {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Vulnerable => "vulnerable",
            Self::Patched => "patched",
            Self::Unknown => "unknown",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "vulnerable" => Some(Self::Vulnerable),
            "patched" => Some(Self::Patched),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }
}

/// Pourquoi ce verdict ; l'interface le traduit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    /// Installée < version qui corrige.
    BelowFixedVersion,
    /// Dans la plage vulnérable de la NVD, sans version corrigée connue.
    InAffectedRange,
    /// La distribution la sait ouverte : pas encore de correctif.
    NoFixAvailable,
    /// Installée ≥ version qui corrige.
    FixedVersionInstalled,
    /// Paquet reconstruit par un éditeur : un correctif a pu être rétroporté.
    VendorBuild,
    /// La source elle-même ne tranche pas.
    Undetermined,
    /// Version installée illisible.
    VersionUnparsed,
}

impl Reason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BelowFixedVersion => "below_fixed_version",
            Self::InAffectedRange => "in_affected_range",
            Self::NoFixAvailable => "no_fix_available",
            Self::FixedVersionInstalled => "fixed_version_installed",
            Self::VendorBuild => "vendor_build",
            Self::Undetermined => "undetermined",
            Self::VersionUnparsed => "version_unparsed",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        [
            Self::BelowFixedVersion,
            Self::InAffectedRange,
            Self::NoFixAvailable,
            Self::FixedVersionInstalled,
            Self::VendorBuild,
            Self::Undetermined,
            Self::VersionUnparsed,
        ]
        .into_iter()
        .find(|reason| reason.as_str() == raw)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    pub state: State,
    pub reason: Reason,
    pub fixed_version: Option<String>,
}

impl Verdict {
    fn new(state: State, reason: Reason, fixed_version: Option<String>) -> Self {
        Self { state, reason, fixed_version }
    }
}

fn wildcard(field: &str) -> bool {
    matches!(field, "*" | "-" | "")
}

/// Le numéro d'une mise à jour nommée dans le champ `update` d'un CPE
/// (`update_3`, `update3`, `u3`, `update_3b` → 3).
fn update_number(update: &str) -> Option<u64> {
    let digits: String = update
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

enum RangeOutcome {
    Affected(Option<String>),
    Fixed(Option<String>),
    Ambiguous,
    NotAffected,
}

fn bound(raw: &Option<String>) -> Result<Option<Generic>, ()> {
    match raw {
        None => Ok(None),
        Some(raw) => Generic::parse(raw).map(Some).ok_or(()),
    }
}

fn check_range(installed: &Generic, range: &CpeRange) -> RangeOutcome {
    let bounds = [
        &range.start_including,
        &range.start_excluding,
        &range.end_including,
        &range.end_excluding,
    ];
    if bounds.iter().all(|b| b.is_none()) {
        if wildcard(&range.version) {
            // « Toutes les versions » : la NVD n'a pas (encore) borné la
            // faille ; impossible de conclure.
            return RangeOutcome::Ambiguous;
        }
        let Some(exact) = Generic::parse(&range.version) else {
            return RangeOutcome::NotAffected;
        };
        if !wildcard(&range.update) {
            // `8.0` + `update_3` : la mise à jour nommée doit être celle
            // installée (`8.0.3`) ; le niveau de correctif dans cette mise à
            // jour (3b, 3c…) n'est pas lisible.
            let prefix = exact.numbers();
            let numbers = installed.numbers();
            let same_line = numbers.len() > prefix.len() && numbers[..prefix.len()] == prefix[..];
            return match (same_line, update_number(&range.update)) {
                (true, Some(update)) if numbers[prefix.len()] == update => RangeOutcome::Ambiguous,
                _ => RangeOutcome::NotAffected,
            };
        }
        return if installed.cmp(&exact) == Ordering::Equal {
            RangeOutcome::Affected(None)
        } else {
            RangeOutcome::NotAffected
        };
    }
    let (Ok(start_in), Ok(start_ex), Ok(end_in), Ok(end_ex)) = (
        bound(&range.start_including),
        bound(&range.start_excluding),
        bound(&range.end_including),
        bound(&range.end_excluding),
    ) else {
        return RangeOutcome::Ambiguous;
    };
    if start_in.as_ref().is_some_and(|s| installed < s)
        || start_ex.as_ref().is_some_and(|s| installed <= s)
    {
        return RangeOutcome::NotAffected;
    }
    if let Some(end) = &end_ex {
        return if installed < end {
            RangeOutcome::Affected(range.end_excluding.clone())
        } else {
            RangeOutcome::Fixed(range.end_excluding.clone())
        };
    }
    if let Some(end) = &end_in {
        return if installed <= end {
            RangeOutcome::Affected(None)
        } else {
            RangeOutcome::Fixed(None)
        };
    }
    // Ouverte vers le haut : toutes les versions depuis le début de plage.
    RangeOutcome::Affected(None)
}

/// Verdict NVD : la version installée confrontée aux plages vulnérables du
/// produit. `None` : la CVE ne concerne pas cette version.
pub fn nvd(version: &str, vendor_build: bool, ranges: &[CpeRange]) -> Option<Verdict> {
    if ranges.is_empty() {
        return None;
    }
    let upstream = if vendor_build { upstream_of(version) } else { version };
    let Some(installed) = Generic::parse(upstream) else {
        return Some(Verdict::new(State::Unknown, Reason::VersionUnparsed, None));
    };
    let mut affected: Option<Option<String>> = None;
    let mut ambiguous = false;
    let mut fixed: Option<Option<String>> = None;
    for range in ranges {
        match check_range(&installed, range) {
            RangeOutcome::Affected(fix) => {
                if affected.as_ref().is_none_or(Option::is_none) {
                    affected = Some(fix);
                }
            }
            RangeOutcome::Fixed(fix) => {
                if fixed.as_ref().is_none_or(Option::is_none) {
                    fixed = Some(fix);
                }
            }
            RangeOutcome::Ambiguous => ambiguous = true,
            RangeOutcome::NotAffected => {}
        }
    }
    if let Some(fix) = affected {
        if vendor_build {
            return Some(Verdict::new(State::Unknown, Reason::VendorBuild, fix));
        }
        let reason =
            if fix.is_some() { Reason::BelowFixedVersion } else { Reason::InAffectedRange };
        return Some(Verdict::new(State::Vulnerable, reason, fix));
    }
    if ambiguous {
        return Some(Verdict::new(State::Unknown, Reason::Undetermined, None));
    }
    fixed.map(|fix| Verdict::new(State::Patched, Reason::FixedVersionInstalled, fix))
}

/// Verdict du tracker Debian pour la version installée du paquet source.
pub fn debian(installed: &str, status: Status, fixed_version: Option<&str>) -> Verdict {
    if !debian_valid(installed) {
        return Verdict::new(
            State::Unknown,
            Reason::VersionUnparsed,
            fixed_version.map(Into::into),
        );
    }
    match (status, fixed_version) {
        (Status::Resolved, Some(fixed)) => {
            if debian_cmp(installed, fixed) == Ordering::Less {
                Verdict::new(State::Vulnerable, Reason::BelowFixedVersion, Some(fixed.into()))
            } else {
                Verdict::new(State::Patched, Reason::FixedVersionInstalled, Some(fixed.into()))
            }
        }
        (Status::Open, _) => Verdict::new(State::Vulnerable, Reason::NoFixAvailable, None),
        _ => Verdict::new(State::Unknown, Reason::Undetermined, None),
    }
}

/// Verdict OSV : l'avis a été renvoyé pour cette version, elle est donc
/// touchée ; reste à savoir si un correctif existe.
pub fn osv(fixed_version: Option<&str>) -> Verdict {
    match fixed_version {
        Some(fixed) => {
            Verdict::new(State::Vulnerable, Reason::BelowFixedVersion, Some(fixed.to_string()))
        }
        None => Verdict::new(State::Vulnerable, Reason::NoFixAvailable, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range(version: &str, update: &str, bounds: [Option<&str>; 4]) -> CpeRange {
        let [start_including, start_excluding, end_including, end_excluding] =
            bounds.map(|b| b.map(str::to_string));
        CpeRange {
            cpe: "a:x:y".into(),
            version: version.into(),
            update: update.into(),
            start_including,
            start_excluding,
            end_including,
            end_excluding,
        }
    }

    #[test]
    fn une_version_sous_la_borne_est_vulnerable_au_dela_corrigee() {
        let ranges = [range("*", "*", [None, None, None, Some("8.2.5")])];
        let verdict = nvd("8.2.4", false, &ranges).unwrap();
        assert_eq!(verdict.state, State::Vulnerable);
        assert_eq!(verdict.reason, Reason::BelowFixedVersion);
        assert_eq!(verdict.fixed_version.as_deref(), Some("8.2.5"));
        let verdict = nvd("8.2.10", false, &ranges).unwrap();
        assert_eq!(verdict.state, State::Patched);
        assert_eq!(verdict.fixed_version.as_deref(), Some("8.2.5"));
    }

    #[test]
    fn hors_de_la_branche_vulnerable_rien_n_est_dit() {
        let ranges = [range("*", "*", [Some("7.0"), None, None, Some("7.4.3")])];
        assert_eq!(nvd("6.9.1", false, &ranges), None);
        let inclusive = [range("*", "*", [None, None, Some("2.4.58"), None])];
        assert_eq!(nvd("2.4.58", false, &inclusive).unwrap().reason, Reason::InAffectedRange);
        assert_eq!(nvd("2.4.59", false, &inclusive).unwrap().state, State::Patched);
        assert_eq!(nvd("1.0", false, &[]), None);
    }

    #[test]
    fn une_version_exacte_ne_touche_qu_elle_meme() {
        let ranges = [range("6.49.6", "*", [None; 4]), range("6.49.7", "-", [None; 4])];
        assert_eq!(nvd("6.49.6", false, &ranges).unwrap().state, State::Vulnerable);
        assert_eq!(nvd("6.49.8", false, &ranges), None);
    }

    #[test]
    fn le_doute_donne_a_verifier_jamais_vulnerable() {
        // Toutes versions, sans borne.
        let all = [range("*", "*", [None; 4])];
        assert_eq!(nvd("1.2", false, &all).unwrap().reason, Reason::Undetermined);
        // ESXi 8.0 Update 3 : le niveau de correctif n'est pas lisible.
        let esxi = [range("8.0", "update_3", [None; 4])];
        assert_eq!(nvd("8.0.3", false, &esxi).unwrap().state, State::Unknown);
        assert_eq!(nvd("8.0.2", false, &esxi), None, "une autre mise à jour");
        // Paquet reconstruit par Proxmox : la plage amont ne suffit pas.
        let qemu = [range("*", "*", [None, None, None, Some("9.0.3")])];
        let verdict = nvd("9.0.2-2", true, &qemu).unwrap();
        assert_eq!((verdict.state, verdict.reason), (State::Unknown, Reason::VendorBuild));
        assert_eq!(nvd("9.1.0-1", true, &qemu).unwrap().state, State::Patched);
        // Version illisible.
        assert_eq!(nvd("latest", false, &qemu).unwrap().reason, Reason::VersionUnparsed);
    }

    #[test]
    fn une_plage_vulnerable_l_emporte_sur_une_plage_corrigee() {
        let ranges = [
            range("*", "*", [None, None, None, Some("1.0")]),
            range("*", "*", [Some("2.0"), None, None, Some("2.5")]),
        ];
        let verdict = nvd("2.1", false, &ranges).unwrap();
        assert_eq!(verdict.state, State::Vulnerable);
        assert_eq!(verdict.fixed_version.as_deref(), Some("2.5"));
    }

    #[test]
    fn le_tracker_debian_tient_compte_du_retroportage() {
        let fixed = Some("1:9.2p1-2+deb12u3");
        let patched = debian("1:9.2p1-2+deb12u7", Status::Resolved, fixed);
        assert_eq!(patched.state, State::Patched);
        let old = debian("1:9.2p1-2+deb12u2", Status::Resolved, fixed);
        assert_eq!((old.state, old.reason), (State::Vulnerable, Reason::BelowFixedVersion));
        let open = debian("6.1.106-3", Status::Open, None);
        assert_eq!((open.state, open.reason), (State::Vulnerable, Reason::NoFixAvailable));
        assert_eq!(debian("1.0-1", Status::Undetermined, None).state, State::Unknown);
        assert_eq!(debian("abc", Status::Resolved, fixed).reason, Reason::VersionUnparsed);
    }

    #[test]
    fn osv_et_libelles() {
        assert_eq!(osv(Some("1.2-3")).reason, Reason::BelowFixedVersion);
        assert_eq!(osv(None).reason, Reason::NoFixAvailable);
        for reason in [Reason::VendorBuild, Reason::VersionUnparsed, Reason::NoFixAvailable] {
            assert_eq!(Reason::parse(reason.as_str()), Some(reason));
        }
        assert_eq!(State::parse("patched"), Some(State::Patched));
        assert_eq!(update_number("update_3b"), Some(3));
        assert_eq!(update_number("-"), None);
    }
}
