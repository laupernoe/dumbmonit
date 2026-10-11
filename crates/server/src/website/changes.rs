//! Détection des changements, confirmée sur deux sondes.
//!
//! Chaque champ suivi (titre, description, adresse finale…) a une valeur de
//! référence, confirmée. Une valeur différente lue une fois est seulement mise
//! en attente ; elle ne devient un changement que si la sonde suivante la lit
//! à nouveau. Une page servie en deux versions (test A/B), un cache qui
//! hésite, une réponse tronquée ne déclenchent donc rien. Une sonde qui
//! retrouve la référence efface l'attente.
//!
//! Les valeurs gardées sont celles lues, pour l'affichage ; la comparaison se
//! fait sur leur forme normalisée (espaces réduites, casse ignorée, et pour
//! les adresses, paramètres d'URL ignorés si `ignore_query`).

use std::collections::BTreeMap;

use reqwest::Url;
use serde::{Deserialize, Serialize};

use super::seo::Metadata;

/// Champs suivis, dans l'ordre d'affichage.
pub const FIELDS: &[&str] = &[
    FINAL_URL,
    TITLE,
    DESCRIPTION,
    CANONICAL,
    ROBOTS,
    OG_TITLE,
    OG_DESCRIPTION,
    LANG,
    H1_COUNT,
    HREFLANG_COUNT,
];
pub const FINAL_URL: &str = "final_url";
pub const TITLE: &str = "title";
pub const DESCRIPTION: &str = "description";
pub const CANONICAL: &str = "canonical";
pub const ROBOTS: &str = "robots";
pub const OG_TITLE: &str = "og_title";
pub const OG_DESCRIPTION: &str = "og_description";
pub const LANG: &str = "lang";
pub const H1_COUNT: &str = "h1_count";
pub const HREFLANG_COUNT: &str = "hreflang_count";

/// Valeurs d'une lecture, champ par champ (chaîne vide : absent).
pub type Values = BTreeMap<String, String>;

/// Les valeurs à comparer d'une page lue.
pub fn values(final_url: &Url, meta: &Metadata) -> Values {
    let text = |value: &Option<String>| value.clone().unwrap_or_default();
    [
        (FINAL_URL, final_url.to_string()),
        (TITLE, text(&meta.title)),
        (DESCRIPTION, text(&meta.description)),
        (CANONICAL, text(&meta.canonical)),
        (ROBOTS, text(&meta.robots)),
        (OG_TITLE, text(&meta.og_title)),
        (OG_DESCRIPTION, text(&meta.og_description)),
        (LANG, text(&meta.lang)),
        (H1_COUNT, meta.h1_count.to_string()),
        (HREFLANG_COUNT, meta.hreflang_count.to_string()),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect()
}

/// Forme comparée d'une valeur.
pub fn normalise(field: &str, value: &str, ignore_query: bool) -> String {
    match field {
        FINAL_URL | CANONICAL => url_key(value, ignore_query),
        _ => value.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase(),
    }
}

/// Clé d'une adresse : hôte sans `www.`, chemin sans `/` final, et requête
/// si elle compte. Le schéma n'y est pas : passer de `http` à `https`, ou du
/// domaine nu à `www.`, n'est pas un déménagement.
pub fn url_key(value: &str, ignore_query: bool) -> String {
    let Ok(url) = Url::parse(value.trim()) else {
        return value.trim().to_lowercase();
    };
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host);
    let port = url.port().map(|p| format!(":{p}")).unwrap_or_default();
    let path = url.path().trim_end_matches('/');
    let mut key = format!("{host}{port}{path}");
    if !ignore_query && let Some(query) = url.query().filter(|q| !q.is_empty()) {
        key.push('?');
        key.push_str(query);
    }
    key
}

/// Ce que l'on garde d'une sonde à l'autre.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tracker {
    /// Référence confirmée.
    pub confirmed: Values,
    /// Valeurs différentes de la référence, vues une fois.
    #[serde(default)]
    pub pending: Values,
}

/// Un changement confirmé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub field: String,
    pub before: String,
    pub after: String,
}

impl Tracker {
    /// Une référence neuve : rien n'est signalé.
    pub fn baseline(current: Values) -> Self {
        Self { confirmed: current, pending: Values::new() }
    }

    /// Compare une lecture à la référence ; rend les changements confirmés par
    /// cette lecture.
    pub fn observe(&mut self, current: &Values, ignore_query: bool) -> Vec<Change> {
        let mut changes = Vec::new();
        for (field, value) in current {
            let key = normalise(field, value, ignore_query);
            let Some(reference) = self.confirmed.get(field) else {
                // Champ apparu avec une version plus récente : il entre dans
                // la référence sans bruit.
                self.confirmed.insert(field.clone(), value.clone());
                continue;
            };
            if normalise(field, reference, ignore_query) == key {
                self.pending.remove(field);
                // La forme lue peut avoir changé sans que le sens change
                // (casse, espaces) : on garde la dernière pour l'affichage.
                self.confirmed.insert(field.clone(), value.clone());
                continue;
            }
            let confirmed = self
                .pending
                .get(field)
                .is_some_and(|pending| normalise(field, pending, ignore_query) == key);
            if confirmed {
                changes.push(Change {
                    field: field.clone(),
                    before: reference.clone(),
                    after: value.clone(),
                });
                self.confirmed.insert(field.clone(), value.clone());
                self.pending.remove(field);
            } else {
                self.pending.insert(field.clone(), value.clone());
            }
        }
        changes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reading(pairs: &[(&str, &str)]) -> Values {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn un_changement_n_est_confirme_qu_a_la_deuxieme_sonde() {
        let mut tracker = Tracker::baseline(reading(&[(TITLE, "Accueil")]));
        let changed = reading(&[(TITLE, "Soldes !")]);
        assert!(tracker.observe(&changed, true).is_empty(), "première lecture : en attente");
        assert_eq!(tracker.pending.get(TITLE).map(String::as_str), Some("Soldes !"));
        let changes = tracker.observe(&changed, true);
        assert_eq!(
            changes,
            [Change { field: TITLE.into(), before: "Accueil".into(), after: "Soldes !".into() }]
        );
        assert!(tracker.pending.is_empty());
        assert!(tracker.observe(&changed, true).is_empty(), "rien de plus ensuite");
    }

    #[test]
    fn un_test_a_b_ne_confirme_rien() {
        let mut tracker = Tracker::baseline(reading(&[(TITLE, "A")]));
        for _ in 0..5 {
            assert!(tracker.observe(&reading(&[(TITLE, "B")]), true).is_empty());
            assert!(tracker.observe(&reading(&[(TITLE, "A")]), true).is_empty());
        }
        // Deux variantes nouvelles qui alternent non plus.
        for _ in 0..5 {
            assert!(tracker.observe(&reading(&[(TITLE, "B")]), true).is_empty());
            assert!(tracker.observe(&reading(&[(TITLE, "C")]), true).is_empty());
        }
    }

    #[test]
    fn casse_et_espaces_ne_comptent_pas() {
        let mut tracker = Tracker::baseline(reading(&[(DESCRIPTION, "Une  description")]));
        for _ in 0..3 {
            assert!(
                tracker.observe(&reading(&[(DESCRIPTION, " une DESCRIPTION ")]), true).is_empty()
            );
        }
        assert!(tracker.pending.is_empty());
    }

    #[test]
    fn les_parametres_d_url_sont_ignores_si_demande() {
        let base = reading(&[
            (FINAL_URL, "https://www.ex.fr/accueil/?utm_source=a"),
            (CANONICAL, "https://ex.fr/accueil"),
        ]);
        let noisy = reading(&[
            (FINAL_URL, "http://ex.fr/accueil?utm_source=b&session=42"),
            (CANONICAL, "https://www.ex.fr/accueil/?ref=x"),
        ]);
        let mut tracker = Tracker::baseline(base.clone());
        for _ in 0..3 {
            assert!(tracker.observe(&noisy, true).is_empty());
        }
        let mut strict = Tracker::baseline(base);
        assert!(strict.observe(&noisy, false).is_empty());
        let changes = strict.observe(&noisy, false);
        assert_eq!(changes.len(), 2, "sans ignore_query, la requête compte");
    }

    #[test]
    fn un_changement_de_chemin_ou_d_hote_compte() {
        let mut tracker = Tracker::baseline(reading(&[(FINAL_URL, "https://www.ex.fr/")]));
        let moved = reading(&[(FINAL_URL, "https://parking.example.net/lander")]);
        assert!(tracker.observe(&moved, true).is_empty());
        assert_eq!(tracker.observe(&moved, true).len(), 1);
    }

    #[test]
    fn cles_d_adresse() {
        assert_eq!(url_key("https://WWW.Ex.fr/a/", true), "ex.fr/a");
        assert_eq!(url_key("http://ex.fr:8080/a?b=1", false), "ex.fr:8080/a?b=1");
        assert_eq!(url_key("http://ex.fr/a?b=1", true), "ex.fr/a");
        assert_eq!(url_key("", true), "");
    }

    #[test]
    fn un_champ_nouveau_entre_dans_la_reference() {
        let mut tracker = Tracker::baseline(reading(&[(TITLE, "A")]));
        assert!(tracker.observe(&reading(&[(TITLE, "A"), (LANG, "fr")]), true).is_empty());
        assert_eq!(tracker.confirmed.get(LANG).map(String::as_str), Some("fr"));
    }
}
