//! Versions semver, comparées comme le fait semver.
//!
//! Partagées par le serveur (recherche de nouvelle version) et l'agent (mise
//! à jour automatique : jamais de retour en arrière sans demande explicite).

use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Ident {
    Num(u64),
    Text(String),
}

impl Ord for Ident {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Num(a), Self::Num(b)) => a.cmp(b),
            (Self::Text(a), Self::Text(b)) => a.cmp(b),
            // Semver : un identifiant numérique est plus petit qu'un alphanumérique.
            (Self::Num(_), Self::Text(_)) => Ordering::Less,
            (Self::Text(_), Self::Num(_)) => Ordering::Greater,
        }
    }
}

impl PartialOrd for Ident {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Version semver (`1.2.3`, `0.1.0-alpha.6`), préfixe `v` et métadonnées `+…` tolérés.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    core: [u64; 3],
    pre: Vec<Ident>,
}

impl Version {
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim().trim_start_matches('v');
        let text = text.split('+').next()?;
        let (core, pre) = match text.split_once('-') {
            Some((core, pre)) => (core, Some(pre)),
            None => (text, None),
        };
        let mut parts = core.split('.');
        let mut numbers = [0u64; 3];
        for slot in &mut numbers {
            *slot = parts.next()?.parse().ok()?;
        }
        if parts.next().is_some() {
            return None;
        }
        let pre = match pre {
            None => Vec::new(),
            Some(pre) => pre
                .split('.')
                .map(|id| match id.parse::<u64>() {
                    Ok(n) => Some(Ident::Num(n)),
                    Err(_) if !id.is_empty() => Some(Ident::Text(id.to_string())),
                    Err(_) => None,
                })
                .collect::<Option<Vec<_>>>()?,
        };
        Some(Self { core: numbers, pre })
    }

    pub fn is_prerelease(&self) -> bool {
        !self.pre.is_empty()
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.core.cmp(&other.core).then_with(|| match (self.pre.is_empty(), other.pre.is_empty()) {
            (true, true) => Ordering::Equal,
            // Une version sans pré-version est plus grande que `-alpha.N`.
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (false, false) => self.pre.cmp(&other.pre),
        })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Vrai si `candidate` est strictement plus ancienne que `current`. Une version
/// illisible d'un côté ou de l'autre n'est jamais dite plus ancienne : c'est
/// alors l'empreinte du binaire seule qui décide.
pub fn is_older(candidate: &str, current: &str) -> bool {
    match (Version::parse(candidate), Version::parse(current)) {
        (Some(candidate), Some(current)) => candidate < current,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_like_semver() {
        let v = |text: &str| Version::parse(text).expect(text);
        assert!(v("0.13.5") > v("0.13.4"));
        assert!(v("0.13.5") > v("0.13.5-rc.1"));
        assert!(v("0.13.5-rc.2") > v("0.13.5-rc.1"));
        assert!(v("v1.0.0+abc") == v("1.0.0"));
        assert!(Version::parse("1.2").is_none());
    }

    #[test]
    fn an_unreadable_version_is_never_older() {
        assert!(is_older("0.13.4", "0.13.5"));
        assert!(!is_older("0.13.5", "0.13.5"));
        assert!(!is_older("0.14.0", "0.13.5"));
        assert!(!is_older("dev", "0.13.5"));
        assert!(!is_older("0.13.4", "dev"));
    }
}
