//! Comparaison de versions : l'ordre de `dpkg` pour les paquets Debian (et
//! leurs dérivés), un ordre « par segments » pour tout le reste (semver,
//! numéros de build Windows, `24.7.5_3` d'OPNsense, `9.2p1` d'OpenSSH).
//!
//! Une version qu'on ne sait pas lire n'est jamais comparée « au mieux » :
//! [`Generic::parse`] répond `None`, et le verdict devient « à vérifier ».

use std::cmp::Ordering;

// ------------------------------------------------------------------ Debian

/// Compare deux versions Debian (`[époque:]amont[-révision]`) selon
/// l'algorithme de `dpkg --compare-versions`.
pub fn debian_cmp(a: &str, b: &str) -> Ordering {
    let (ea, ua, ra) = debian_split(a);
    let (eb, ub, rb) = debian_split(b);
    ea.cmp(&eb).then_with(|| verrevcmp(ua, ub)).then_with(|| verrevcmp(ra, rb))
}

fn debian_split(version: &str) -> (u64, &str, &str) {
    let version = version.trim();
    let (epoch, rest) = match version.split_once(':') {
        Some((epoch, rest)) if epoch.chars().all(|c| c.is_ascii_digit()) && !epoch.is_empty() => {
            (epoch.parse().unwrap_or(0), rest)
        }
        _ => (0, version),
    };
    match rest.rsplit_once('-') {
        Some((upstream, revision)) => (epoch, upstream, revision),
        None => (epoch, rest, ""),
    }
}

/// Poids d'un caractère non numérique : `~` avant tout, même la fin de
/// chaîne ; les lettres avant les autres signes.
fn order(c: Option<u8>) -> i32 {
    match c {
        None => 0,
        Some(b'~') => -1,
        Some(c) if c.is_ascii_digit() => 0,
        Some(c) if c.is_ascii_alphabetic() => i32::from(c),
        Some(c) => i32::from(c) + 256,
    }
}

fn verrevcmp(a: &str, b: &str) -> Ordering {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let (mut i, mut j) = (0, 0);
    while i < a.len() || j < b.len() {
        // Partie non numérique.
        while (i < a.len() && !a[i].is_ascii_digit()) || (j < b.len() && !b[j].is_ascii_digit()) {
            let ac = order(a.get(i).copied().filter(|c| !c.is_ascii_digit()));
            let bc = order(b.get(j).copied().filter(|c| !c.is_ascii_digit()));
            if ac != bc {
                return ac.cmp(&bc);
            }
            if i < a.len() && !a[i].is_ascii_digit() {
                i += 1;
            }
            if j < b.len() && !b[j].is_ascii_digit() {
                j += 1;
            }
        }
        // Partie numérique, zéros de tête ignorés.
        while i < a.len() && a[i] == b'0' {
            i += 1;
        }
        while j < b.len() && b[j] == b'0' {
            j += 1;
        }
        let start_i = i;
        let start_j = j;
        while i < a.len() && a[i].is_ascii_digit() {
            i += 1;
        }
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        let (na, nb) = (&a[start_i..i], &b[start_j..j]);
        let by_len = na.len().cmp(&nb.len());
        if by_len != Ordering::Equal {
            return by_len;
        }
        let by_digits = na.cmp(nb);
        if by_digits != Ordering::Equal {
            return by_digits;
        }
    }
    Ordering::Equal
}

/// Une version Debian bien formée : de quoi la comparer sans surprise.
pub fn debian_valid(version: &str) -> bool {
    let (_, upstream, _) = debian_split(version);
    upstream.starts_with(|c: char| c.is_ascii_digit())
}

/// La version amont d'un paquet de distribution : sans époque, sans
/// révision, sans suffixe de reconstruction (`+dfsg`, `~bpo12`, `+pve3`).
/// `1:9.2p1-2+deb12u3` → `9.2p1` ; `19.2.3-pve4` → `19.2.3`.
pub fn upstream_of(version: &str) -> &str {
    let (_, upstream, _) = debian_split(version);
    let end = upstream.find(['+', '~']).unwrap_or(upstream.len());
    &upstream[..end]
}

/// Vrai pour un paquet reconstruit par un éditeur ou une distribution, qui a
/// pu recevoir un correctif sans changer de version amont (Proxmox, Debian,
/// Ubuntu…). Une plage amont « vulnérable » ne suffit pas alors à conclure.
pub fn is_vendor_build(version: &str) -> bool {
    let (_, upstream, revision) = debian_split(version);
    !revision.is_empty() || upstream.contains(['+', '~'])
}

// ----------------------------------------------------------------- générique

#[derive(Debug, Clone, PartialEq, Eq)]
enum Segment {
    Number(u64),
    Text(String),
}

/// Une version « amont » découpée en segments numériques et alphabétiques.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Generic(Vec<Segment>);

/// Les mots qui, après les nombres, désignent une préversion : `1.0rc1`
/// précède `1.0`. Tout autre suffixe (`9.2p1`, `1.1.1w`) la suit.
const PRERELEASE: &[&str] = &["alpha", "beta", "rc", "pre", "preview", "dev", "snapshot"];

impl Generic {
    /// `None` si la chaîne ne commence pas par un chiffre (après un `v`
    /// éventuel) : « latest », « unknown », un nom de code.
    pub fn parse(raw: &str) -> Option<Self> {
        let raw = raw.trim();
        let raw = raw.strip_prefix(['v', 'V']).unwrap_or(raw);
        if !raw.starts_with(|c: char| c.is_ascii_digit()) {
            return None;
        }
        let mut segments = Vec::new();
        let mut current = String::new();
        let mut numeric = true;
        let flush = |current: &mut String, numeric: bool, segments: &mut Vec<Segment>| {
            if current.is_empty() {
                return;
            }
            let segment = if numeric {
                // Un nombre démesuré (date, empreinte) reste comparable.
                Segment::Number(current.parse().unwrap_or(u64::MAX))
            } else {
                Segment::Text(current.to_ascii_lowercase())
            };
            segments.push(segment);
            current.clear();
        };
        for c in raw.chars() {
            if c.is_ascii_digit() {
                if !numeric {
                    flush(&mut current, numeric, &mut segments);
                }
                numeric = true;
                current.push(c);
            } else if c.is_ascii_alphabetic() {
                if numeric {
                    flush(&mut current, numeric, &mut segments);
                }
                numeric = false;
                current.push(c);
            } else {
                flush(&mut current, numeric, &mut segments);
                // Une espace ou une parenthèse clôt la version : « 2.4.58 (Unix) ».
                if c.is_whitespace() || c == '(' {
                    break;
                }
            }
        }
        flush(&mut current, numeric, &mut segments);
        (!segments.is_empty()).then_some(Self(segments))
    }

    /// Les `n` premiers segments numériques (`8.0.3` → `[8, 0, 3]`).
    pub fn numbers(&self) -> Vec<u64> {
        self.0
            .iter()
            .map_while(|s| match s {
                Segment::Number(n) => Some(*n),
                Segment::Text(_) => None,
            })
            .collect()
    }
}

fn is_prerelease(segment: &Segment) -> bool {
    matches!(segment, Segment::Text(t) if PRERELEASE.contains(&t.as_str()))
}

impl Ord for Generic {
    fn cmp(&self, other: &Self) -> Ordering {
        let (a, b) = (&self.0, &other.0);
        for index in 0..a.len().max(b.len()) {
            let ordering = match (a.get(index), b.get(index)) {
                (Some(Segment::Number(x)), Some(Segment::Number(y))) => x.cmp(y),
                (Some(Segment::Text(x)), Some(Segment::Text(y))) => {
                    match (is_prerelease(&a[index]), is_prerelease(&b[index])) {
                        (true, false) => Ordering::Less,
                        (false, true) => Ordering::Greater,
                        _ => x.cmp(y),
                    }
                }
                // Un nombre l'emporte sur un suffixe de préversion, pas sur un
                // suffixe ordinaire : `1.0.1` > `1.0rc1`, `1.0p1` > `1.0.0`.
                (Some(Segment::Number(_)), Some(text @ Segment::Text(_))) => {
                    if is_prerelease(text) { Ordering::Greater } else { Ordering::Less }
                }
                (Some(text @ Segment::Text(_)), Some(Segment::Number(_))) => {
                    if is_prerelease(text) { Ordering::Less } else { Ordering::Greater }
                }
                // L'une s'arrête : `1.0` = `1.0.0`, `1.0rc1` < `1.0`, `1.0.1` > `1.0`.
                (Some(rest), None) => match rest {
                    Segment::Number(0) => Ordering::Equal,
                    Segment::Number(_) => Ordering::Greater,
                    text if is_prerelease(text) => Ordering::Less,
                    Segment::Text(_) => Ordering::Greater,
                },
                (None, Some(rest)) => match rest {
                    Segment::Number(0) => Ordering::Equal,
                    Segment::Number(_) => Ordering::Less,
                    text if is_prerelease(text) => Ordering::Greater,
                    Segment::Text(_) => Ordering::Less,
                },
                (None, None) => Ordering::Equal,
            };
            if ordering != Ordering::Equal {
                return ordering;
            }
        }
        Ordering::Equal
    }
}

impl PartialOrd for Generic {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Compare deux versions génériques ; `None` si l'une est illisible.
pub fn generic_cmp(a: &str, b: &str) -> Option<Ordering> {
    Some(Generic::parse(a)?.cmp(&Generic::parse(b)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use Ordering::{Equal, Greater, Less};

    #[test]
    fn l_ordre_debian_suit_dpkg() {
        // Cas de référence de dpkg (lib/dpkg/t/t-version.c).
        for (a, b, expected) in [
            ("1.0", "1.0", Equal),
            ("1.0-1", "1.0-2", Less),
            ("1:1.0", "2.0", Greater),
            ("1.0~rc1", "1.0", Less),
            ("1.0~rc1-1", "1.0-1", Less),
            ("1.0", "1.0-0", Equal),
            ("1.0a", "1.0", Greater),
            ("1.0+dfsg", "1.0", Greater),
            ("1.0.1", "1.0a", Greater),
            ("2.30-1", "2.4-1", Greater),
            ("0010", "10", Equal),
            ("3.0.15-1~deb12u1", "3.0.15-1", Less),
            ("3.0.15-1~deb12u1", "3.0.14-1", Greater),
            ("1:9.2p1-2+deb12u3", "1:9.2p1-2+deb12u2", Greater),
            ("1:9.2p1-2+deb12u10", "1:9.2p1-2+deb12u3", Greater),
            ("1:9.2p1-2", "9.9", Greater),
            ("2.03.31-2+pmx1", "2.03.31-2", Greater),
            ("6.1.106-3", "6.1.112-1", Less),
            ("1.2.3~bpo12+1", "1.2.3", Less),
        ] {
            assert_eq!(debian_cmp(a, b), expected, "{a} vs {b}");
            assert_eq!(debian_cmp(b, a), expected.reverse(), "{b} vs {a}");
        }
    }

    #[test]
    fn la_version_amont_perd_epoque_revision_et_suffixes() {
        assert_eq!(upstream_of("1:9.2p1-2+deb12u3"), "9.2p1");
        assert_eq!(upstream_of("19.2.3-pve4"), "19.2.3");
        assert_eq!(upstream_of("0.8.0+pve3"), "0.8.0");
        assert_eq!(upstream_of("11.0.3-2"), "11.0.3");
        assert_eq!(upstream_of("9.2.10"), "9.2.10");
        assert!(is_vendor_build("11.0.3-2"));
        assert!(is_vendor_build("0.8.0+pve3"));
        assert!(!is_vendor_build("9.2.10"));
        assert!(debian_valid("1:2.0-1") && !debian_valid("abc"));
    }

    #[test]
    fn l_ordre_generique_lit_semver_builds_et_suffixes() {
        for (a, b, expected) in [
            ("8.2.4", "8.2.10", Less),
            ("v0.107.79", "0.107.8", Greater),
            ("10.0.22631.4317", "10.0.22631.4460", Less),
            ("10.0.22631.4317", "10.0.22631.4317", Equal),
            ("1.0", "1.0.0", Equal),
            ("1.0.0-rc1", "1.0.0", Less),
            ("1.0.0-beta2", "1.0.0-beta10", Less),
            ("1.0.0", "1.0.0-rc1", Greater),
            ("9.2p1", "9.2", Greater),
            ("9.3", "9.2p1", Greater),
            ("24.7.5_3", "24.7.5", Greater),
            ("24.7.5_3", "24.7.6", Less),
            ("2.4.58 (Unix)", "2.4.58", Equal),
            ("130.0.6723.92", "130.0.6723.117", Less),
            ("1.1.1w", "1.1.1", Greater),
        ] {
            assert_eq!(generic_cmp(a, b), Some(expected), "{a} vs {b}");
            assert_eq!(generic_cmp(b, a), Some(expected.reverse()), "{b} vs {a}");
        }
        assert_eq!(generic_cmp("latest", "1.0"), None);
        assert_eq!(generic_cmp("", "1.0"), None);
        assert_eq!(Generic::parse("8.0.3").unwrap().numbers(), [8, 0, 3]);
    }
}
