//! Une vérification : un sujet, une liste, un verdict. Et la cadence à laquelle
//! on les enchaîne.

use std::net::Ipv4Addr;
use std::sync::Arc;
use std::time::Duration;

use futures::future::BoxFuture;
use futures::{FutureExt, StreamExt};
use tokio::sync::Mutex;
use tokio::time::Instant;

use super::dns::{Dns, Failure, Qtype};
use super::lists::{Code, List, Target};

/// Ce qu'une liste dit d'un sujet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Inscrit : codes rendus, leur sens, et le motif publié en `TXT` s'il y en a un.
    Listed { codes: Vec<Ipv4Addr>, meaning: String, reason: Option<String> },
    /// `NXDOMAIN` : absent de la liste.
    Clean,
    /// La liste refuse de répondre : on ne sait rien.
    Refused(String),
    /// Pas de réponse exploitable (délai, `SERVFAIL`, réponse hors de
    /// `127.0.0.0/8`) : on ne sait rien non plus, et surtout pas « absent ».
    Unknown(String),
    /// La liste ne couvre pas ce sujet (IPv6 sur une liste IPv4).
    NotApplicable,
}

impl Verdict {
    /// Nom stable, pour l'étiquette `state`.
    pub fn state(&self) -> &'static str {
        match self {
            Self::Listed { .. } => "listed",
            Self::Clean => "clean",
            Self::Refused(_) => "refused",
            Self::Unknown(_) => "unknown",
            Self::NotApplicable => "not_applicable",
        }
    }

    /// Valeur de la série `blacklist_check`.
    pub fn value(&self) -> f64 {
        match self {
            Self::Listed { .. } => 1.0,
            Self::Clean => 0.0,
            Self::Unknown(_) => -1.0,
            Self::Refused(_) => -2.0,
            Self::NotApplicable => -3.0,
        }
    }

    pub fn is_listed(&self) -> bool {
        matches!(self, Self::Listed { .. })
    }

    /// Ni oui ni non : délai, erreur ou refus.
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown(_) | Self::Refused(_))
    }
}

/// Lit la réponse `A` d'une liste. Pur.
pub fn interpret(list: &List, answer: Result<Vec<String>, Failure>) -> Verdict {
    let values = match answer {
        Ok(values) => values,
        Err(failure) => return Verdict::Unknown(failure.to_string()),
    };
    if values.is_empty() {
        return Verdict::Clean;
    }
    let mut codes = Vec::new();
    for value in &values {
        match value.parse::<Ipv4Addr>() {
            Ok(code) if code.octets()[0] == 127 => codes.push(code),
            // Un résolveur qui remplace `NXDOMAIN` par l'adresse d'une page de
            // recherche rend toute liste « positive » : on refuse de conclure.
            _ => {
                return Verdict::Unknown(format!(
                    "{} answered {value}, outside 127.0.0.0/8: the resolver rewrites answers",
                    list.zone
                ));
            }
        }
    }
    codes.sort_unstable();
    codes.dedup();
    let mut meanings = Vec::new();
    for code in &codes {
        match (list.interpret)(*code) {
            Code::Refused(why) => return Verdict::Refused(why),
            Code::Listed(meaning) => {
                if !meanings.contains(&meaning) {
                    meanings.push(meaning);
                }
            }
        }
    }
    Verdict::Listed { codes, meaning: meanings.join(", "), reason: None }
}

/// Espacement des questions : une toutes les `step`, quel que soit le nombre de
/// tâches qui attendent.
pub struct Pace {
    next: Mutex<Instant>,
    step: Duration,
}

impl Pace {
    pub fn per_second(rate: u32) -> Self {
        Self { next: Mutex::new(Instant::now()), step: Duration::from_secs(1) / rate.max(1) }
    }

    pub async fn wait(&self) {
        let at = {
            let mut next = self.next.lock().await;
            let at = (*next).max(Instant::now());
            *next = at + self.step;
            at
        };
        tokio::time::sleep_until(at).await;
    }
}

/// Le motif publié en `TXT`, nettoyé et borné : il finit en étiquette.
fn reason(values: Vec<String>) -> Option<String> {
    let text: String =
        values.join(" ").chars().map(|c| if c.is_control() { ' ' } else { c }).take(240).collect();
    let text = text.trim().to_string();
    (!text.is_empty()).then_some(text)
}

/// Une vérification complète : la question `A`, puis le motif si le sujet est inscrit.
pub async fn check(
    dns: &dyn Dns,
    pace: &Pace,
    subject: &Target,
    list: &'static List,
    dqs_key: Option<&str>,
) -> Verdict {
    let Some(name) = list.query_name(subject, dqs_key) else {
        return Verdict::NotApplicable;
    };
    pace.wait().await;
    let mut verdict = interpret(list, dns.query(&name, Qtype::A).await);
    if let Verdict::Listed { reason: slot, .. } = &mut verdict {
        pace.wait().await;
        // Un motif absent ou muet ne change rien au verdict.
        *slot = dns.query(&name, Qtype::Txt).await.ok().and_then(reason);
    }
    verdict
}

/// Un résultat : quel sujet, quelle liste, quel verdict.
#[derive(Debug, Clone)]
pub struct Outcome {
    pub subject: Target,
    pub list: &'static List,
    pub verdict: Verdict,
}

/// Toutes les vérifications, au plus `MAX_IN_FLIGHT` à la fois, au rythme de `pace`.
pub async fn run(
    dns: Arc<dyn Dns>,
    pace: Arc<Pace>,
    jobs: Vec<(Target, &'static List)>,
    dqs_key: Option<String>,
) -> Vec<Outcome> {
    // Des futurs déjà mis en boîte : une fermeture dans la chaîne du flux
    // empêcherait le compilateur de prouver que le passage est `Send`.
    let pending: Vec<BoxFuture<'static, Outcome>> = jobs
        .into_iter()
        .map(|(subject, list)| {
            let (dns, pace, key) = (dns.clone(), pace.clone(), dqs_key.clone());
            async move {
                let verdict = check(dns.as_ref(), &pace, &subject, list, key.as_deref()).await;
                Outcome { subject, list, verdict }
            }
            .boxed()
        })
        .collect();
    let mut outcomes: Vec<Outcome> = futures::stream::iter(pending)
        .buffer_unordered(super::options::MAX_IN_FLIGHT)
        .collect()
        .await;
    // Ordre stable : celui du catalogue, puis celui des sujets.
    // Les listes personnalisées viennent après le catalogue, dans l'ordre alphabétique.
    let rank = |list: &List| {
        super::lists::LISTS.iter().position(|l| l.id == list.id).unwrap_or(usize::MAX)
    };
    outcomes.sort_by(|a, b| {
        rank(a.list)
            .cmp(&rank(b.list))
            .then_with(|| a.list.id.cmp(b.list.id))
            .then_with(|| a.subject.to_string().cmp(&b.subject.to_string()))
    });
    outcomes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blacklist::dns::fake::FakeDns;
    use crate::blacklist::lists::by_id;

    fn ok(values: &[&str]) -> Result<Vec<String>, Failure> {
        Ok(values.iter().map(|v| v.to_string()).collect())
    }

    #[test]
    fn nxdomain_propre_delai_inconnu_refus_jamais_liste() {
        let zen = by_id("spamhaus_zen").unwrap();
        assert_eq!(interpret(zen, ok(&[])), Verdict::Clean);
        assert!(matches!(interpret(zen, Err(Failure::Timeout)), Verdict::Unknown(_)));
        let refused = interpret(zen, ok(&["127.255.255.254"]));
        assert!(
            matches!(refused, Verdict::Refused(ref why) if why.contains("public")),
            "{refused:?}"
        );
        assert!(!refused.is_listed());
        let hijacked = interpret(zen, ok(&["198.51.100.7"]));
        assert!(matches!(hijacked, Verdict::Unknown(_)));
        let listed = interpret(zen, ok(&["127.0.0.4", "127.0.0.2", "127.0.0.4"]));
        assert_eq!(
            listed,
            Verdict::Listed {
                codes: vec![Ipv4Addr::new(127, 0, 0, 2), Ipv4Addr::new(127, 0, 0, 4)],
                meaning: "SBL, XBL".into(),
                reason: None
            }
        );
    }

    #[tokio::test]
    async fn le_motif_txt_accompagne_une_inscription() {
        let dns = FakeDns::default();
        dns.answer("2.0.0.127.bl.spamcop.net.", Qtype::A, &["127.0.0.2"]).answer(
            "2.0.0.127.bl.spamcop.net.",
            Qtype::Txt,
            &["Blocked - see https://www.spamcop.net/bl.shtml?127.0.0.2"],
        );
        let pace = Pace::per_second(1000);
        let spamcop = by_id("spamcop").unwrap();
        let subject = Target::V4(Ipv4Addr::new(127, 0, 0, 2));
        let verdict = check(&dns, &pace, &subject, spamcop, None).await;
        let Verdict::Listed { reason: Some(reason), .. } = verdict else { panic!("{verdict:?}") };
        assert!(reason.starts_with("Blocked"));
        let clean = check(&dns, &pace, &Target::V4(Ipv4Addr::new(8, 8, 8, 8)), spamcop, None).await;
        assert_eq!(clean, Verdict::Clean);
        let v6 = Target::V6("2a00:1450::1".parse().unwrap());
        assert_eq!(check(&dns, &pace, &v6, spamcop, None).await, Verdict::NotApplicable);
        assert_eq!(dns.queries.load(std::sync::atomic::Ordering::SeqCst), 3, "rien pour l'IPv6");
    }

    #[tokio::test(start_paused = true)]
    async fn les_questions_sont_espacees() {
        let pace = Pace::per_second(10);
        let started = Instant::now();
        for _ in 0..5 {
            pace.wait().await;
        }
        assert!(started.elapsed() >= Duration::from_millis(400), "{:?}", started.elapsed());
    }
}
