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
    HREFLANG,
    GOOGLE_VERIFICATION,
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
/// Langues des variantes `hreflang` (le nombre seul rate un remplacement).
pub const HREFLANG: &str = "hreflang";
/// Jetons `google-site-verification` : leur disparition coupe l'accès à la
/// Search Console.
pub const GOOGLE_VERIFICATION: &str = "google_verification";

/// Réglage `watch_*` → champs qu'il active ou coupe.
pub const WATCH_GROUPS: &[(&str, &[&str])] = &[
    ("watch_final_url", &[FINAL_URL]),
    ("watch_title", &[TITLE]),
    ("watch_description", &[DESCRIPTION]),
    ("watch_canonical", &[CANONICAL]),
    ("watch_robots", &[ROBOTS]),
    ("watch_open_graph", &[OG_TITLE, OG_DESCRIPTION]),
    ("watch_lang", &[LANG]),
    ("watch_h1", &[H1_COUNT]),
    ("watch_hreflang", &[HREFLANG_COUNT, HREFLANG]),
    ("watch_google_verification", &[GOOGLE_VERIFICATION]),
];

/// Une chute du texte n'est jugée que si la référence a au moins ce nombre de
/// caractères : une page courte varie trop, en proportion, pour s'y fier.
pub const MIN_TEXT_REFERENCE: f64 = 200.0;
/// Une chute qui dure plus longtemps devient la nouvelle référence : le site
/// a été refait plus court, ce n'est plus une panne.
pub const TEXT_ACCEPT_SECS: i64 = 7 * 24 * 3600;

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
        (HREFLANG, meta.hreflang.join(",")),
        (GOOGLE_VERIFICATION, meta.google_verification.join(",")),
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
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Tracker {
    /// Référence confirmée.
    pub confirmed: Values,
    /// Valeurs différentes de la référence, vues une fois.
    #[serde(default)]
    pub pending: Values,
    /// Lectures consécutives de la valeur en attente (absent : une).
    #[serde(default)]
    pub pending_count: BTreeMap<String, u32>,
    /// Taille de référence du texte visible, en caractères (moyenne lente).
    #[serde(default)]
    pub text_ref: Option<f64>,
    /// Lectures consécutives très en deçà de cette référence.
    #[serde(default)]
    pub text_low: u32,
    /// Début (secondes Unix) de la chute en cours.
    #[serde(default)]
    pub text_low_since: Option<i64>,
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
        Self { confirmed: current, ..Self::default() }
    }

    /// Repart d'une référence de métadonnées neuve, en gardant celle du texte.
    pub fn rebase(&mut self, current: Values) {
        self.confirmed = current;
        self.pending.clear();
        self.pending_count.clear();
    }

    /// Oublie les champs qui ne sont plus suivis : réactivés plus tard, ils
    /// repartent d'une référence neuve au lieu de signaler un vieux écart.
    pub fn retain(&mut self, watched: &dyn Fn(&str) -> bool) {
        self.confirmed.retain(|field, _| watched(field));
        self.pending.retain(|field, _| watched(field));
        self.pending_count.retain(|field, _| watched(field));
    }

    /// Suit la taille du texte visible. Rend `true` quand elle est tombée de
    /// plus de `drop_percent` % sous la référence depuis `confirm` lectures de
    /// suite. `drop_percent` à 0 : suivi coupé.
    pub fn observe_text(
        &mut self,
        chars: usize,
        drop_percent: u32,
        confirm: u32,
        now: i64,
    ) -> bool {
        let chars = chars as f64;
        if drop_percent == 0 {
            self.text_ref = None;
            self.text_low = 0;
            self.text_low_since = None;
            return false;
        }
        let Some(reference) = self.text_ref else {
            self.text_ref = Some(chars);
            return false;
        };
        let floor = reference * (1.0 - f64::from(drop_percent.min(100)) / 100.0);
        if reference >= MIN_TEXT_REFERENCE && chars < floor {
            self.text_low = self.text_low.saturating_add(1);
            let since = *self.text_low_since.get_or_insert(now);
            if now - since >= TEXT_ACCEPT_SECS {
                self.text_ref = Some(chars);
                self.text_low = 0;
                self.text_low_since = None;
                return false;
            }
            return self.text_low >= confirm.max(1);
        }
        self.text_low = 0;
        self.text_low_since = None;
        // Moyenne lente : le site grossit ou maigrit sans que cela alerte.
        self.text_ref = Some(reference * 0.9 + chars * 0.1);
        false
    }

    /// Compare une lecture à la référence ; rend les changements confirmés par
    /// cette lecture. Une valeur nouvelle doit être lue `confirm` fois de
    /// suite (au moins une).
    pub fn observe(&mut self, current: &Values, ignore_query: bool, confirm: u32) -> Vec<Change> {
        let confirm = confirm.max(1);
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
                self.pending_count.remove(field);
                // La forme lue peut avoir changé sans que le sens change
                // (casse, espaces) : on garde la dernière pour l'affichage.
                self.confirmed.insert(field.clone(), value.clone());
                continue;
            }
            let same_pending = self
                .pending
                .get(field)
                .is_some_and(|pending| normalise(field, pending, ignore_query) == key);
            let count = if same_pending {
                self.pending_count.get(field).copied().unwrap_or(1).saturating_add(1)
            } else {
                1
            };
            if count >= confirm {
                changes.push(Change {
                    field: field.clone(),
                    before: reference.clone(),
                    after: value.clone(),
                });
                self.confirmed.insert(field.clone(), value.clone());
                self.pending.remove(field);
                self.pending_count.remove(field);
            } else {
                self.pending.insert(field.clone(), value.clone());
                self.pending_count.insert(field.clone(), count);
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
        assert!(tracker.observe(&changed, true, 2).is_empty(), "première lecture : en attente");
        assert_eq!(tracker.pending.get(TITLE).map(String::as_str), Some("Soldes !"));
        let changes = tracker.observe(&changed, true, 2);
        assert_eq!(
            changes,
            [Change { field: TITLE.into(), before: "Accueil".into(), after: "Soldes !".into() }]
        );
        assert!(tracker.pending.is_empty());
        assert!(tracker.observe(&changed, true, 2).is_empty(), "rien de plus ensuite");
    }

    #[test]
    fn un_test_a_b_ne_confirme_rien() {
        let mut tracker = Tracker::baseline(reading(&[(TITLE, "A")]));
        for _ in 0..5 {
            assert!(tracker.observe(&reading(&[(TITLE, "B")]), true, 2).is_empty());
            assert!(tracker.observe(&reading(&[(TITLE, "A")]), true, 2).is_empty());
        }
        // Deux variantes nouvelles qui alternent non plus.
        for _ in 0..5 {
            assert!(tracker.observe(&reading(&[(TITLE, "B")]), true, 2).is_empty());
            assert!(tracker.observe(&reading(&[(TITLE, "C")]), true, 2).is_empty());
        }
    }

    #[test]
    fn casse_et_espaces_ne_comptent_pas() {
        let mut tracker = Tracker::baseline(reading(&[(DESCRIPTION, "Une  description")]));
        for _ in 0..3 {
            assert!(
                tracker
                    .observe(&reading(&[(DESCRIPTION, " une DESCRIPTION ")]), true, 2)
                    .is_empty()
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
            assert!(tracker.observe(&noisy, true, 2).is_empty());
        }
        let mut strict = Tracker::baseline(base);
        assert!(strict.observe(&noisy, false, 2).is_empty());
        let changes = strict.observe(&noisy, false, 2);
        assert_eq!(changes.len(), 2, "sans ignore_query, la requête compte");
    }

    #[test]
    fn un_changement_de_chemin_ou_d_hote_compte() {
        let mut tracker = Tracker::baseline(reading(&[(FINAL_URL, "https://www.ex.fr/")]));
        let moved = reading(&[(FINAL_URL, "https://parking.example.net/lander")]);
        assert!(tracker.observe(&moved, true, 2).is_empty());
        assert_eq!(tracker.observe(&moved, true, 2).len(), 1);
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
        assert!(tracker.observe(&reading(&[(TITLE, "A"), (LANG, "fr")]), true, 2).is_empty());
        assert_eq!(tracker.confirmed.get(LANG).map(String::as_str), Some("fr"));
    }

    #[test]
    fn le_nombre_de_lectures_de_confirmation_se_regle() {
        let changed = reading(&[(TITLE, "B")]);
        let mut tracker = Tracker::baseline(reading(&[(TITLE, "A")]));
        assert_eq!(tracker.observe(&changed, true, 1).len(), 1, "une lecture suffit");

        let mut tracker = Tracker::baseline(reading(&[(TITLE, "A")]));
        assert!(tracker.observe(&changed, true, 3).is_empty());
        assert!(tracker.observe(&changed, true, 3).is_empty());
        assert!(
            tracker.observe(&reading(&[(TITLE, "C")]), true, 3).is_empty(),
            "une autre valeur repart de zéro"
        );
        assert!(tracker.observe(&changed, true, 3).is_empty());
        assert!(tracker.observe(&changed, true, 3).is_empty());
        assert_eq!(tracker.observe(&changed, true, 3).len(), 1);
    }

    #[test]
    fn une_attente_ancienne_sans_compteur_compte_pour_une_lecture() {
        let mut tracker = Tracker::baseline(reading(&[(TITLE, "A")]));
        tracker.pending = reading(&[(TITLE, "B")]);
        assert_eq!(tracker.observe(&reading(&[(TITLE, "B")]), true, 2).len(), 1);
    }

    #[test]
    fn un_champ_coupe_est_oublie() {
        let mut tracker = Tracker::baseline(reading(&[(TITLE, "A"), (LANG, "fr")]));
        tracker.pending = reading(&[(LANG, "en")]);
        tracker.retain(&|field| field != LANG);
        assert!(!tracker.confirmed.contains_key(LANG) && tracker.pending.is_empty());
        assert!(tracker.confirmed.contains_key(TITLE));
    }

    #[test]
    fn la_chute_du_texte_est_confirmee_et_la_reference_suit_lentement() {
        let mut tracker = Tracker::default();
        assert!(!tracker.observe_text(1000, 80, 2, 0), "première lecture : référence");
        assert!(!tracker.observe_text(150, 80, 2, 60), "une lecture ne suffit pas");
        assert!(tracker.observe_text(150, 80, 2, 120), "la deuxième confirme");
        assert!(!tracker.observe_text(900, 80, 2, 180), "retour à la normale");
        assert_eq!(tracker.text_low, 0);
        // En deçà du seuil mais pas assez bas : pas une chute.
        assert!(!tracker.observe_text(300, 80, 2, 240));
        assert!(!tracker.observe_text(300, 80, 2, 300));
    }

    #[test]
    fn une_petite_page_ou_un_suivi_coupe_ne_signale_rien() {
        let mut tracker = Tracker::default();
        tracker.observe_text(100, 80, 1, 0);
        assert!(!tracker.observe_text(0, 80, 1, 60), "référence sous 200 caractères");
        let mut tracker = Tracker::default();
        tracker.observe_text(5000, 0, 1, 0);
        assert!(!tracker.observe_text(0, 0, 1, 60));
        assert_eq!(tracker.text_ref, None);
    }

    #[test]
    fn une_chute_durable_devient_la_reference() {
        let mut tracker = Tracker::default();
        tracker.observe_text(1000, 80, 1, 0);
        assert!(tracker.observe_text(100, 80, 1, 10));
        assert!(!tracker.observe_text(100, 80, 1, 10 + TEXT_ACCEPT_SECS));
        assert_eq!(tracker.text_ref, Some(100.0));
    }
}
