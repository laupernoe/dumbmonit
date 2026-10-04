//! Les dernières valeurs connues d'un équipement, telles que les contrôles
//! les lisent.

use std::collections::BTreeMap;

use crate::tsdb::InstantSeries;

/// Une série : son nom sans préfixe, ses étiquettes, sa dernière valeur.
#[derive(Debug, Clone, PartialEq)]
pub struct Fact {
    pub name: String,
    pub labels: BTreeMap<String, String>,
    pub value: f64,
}

impl Fact {
    pub fn label(&self, key: &str) -> &str {
        self.labels.get(key).map(String::as_str).unwrap_or("")
    }
}

/// `(nom, étiquettes, valeur)`, pour construire des faits à la main.
pub type Pair<'a> = (&'a str, &'a [(&'a str, &'a str)], f64);

#[derive(Debug, Clone, Default)]
pub struct Facts {
    items: Vec<Fact>,
}

impl Facts {
    pub fn from_series(series: &[InstantSeries]) -> Self {
        Self::from_series_refs(&series.iter().collect::<Vec<_>>())
    }

    pub fn from_series_refs(series: &[&InstantSeries]) -> Self {
        let items = series
            .iter()
            .filter_map(|one| {
                let name = one.metric.get("__name__")?;
                let name = name.strip_prefix("dumbmonit_").unwrap_or(name).to_string();
                let value: f64 = one.value.1.parse().ok().filter(|v: &f64| v.is_finite())?;
                let labels = one
                    .metric
                    .iter()
                    .filter(|(key, _)| key.as_str() != "__name__")
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                Some(Fact { name, labels, value })
            })
            .collect();
        Self { items }
    }

    /// Construction à la main, pour les tests : `(nom, étiquettes, valeur)`.
    pub fn from_pairs(pairs: &[Pair<'_>]) -> Self {
        let items = pairs
            .iter()
            .map(|(name, labels, value)| Fact {
                name: (*name).to_string(),
                labels: labels.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect(),
                value: *value,
            })
            .collect();
        Self { items }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn all<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Fact> + 'a {
        self.items.iter().filter(move |fact| fact.name == name)
    }

    pub fn has(&self, name: &str) -> bool {
        self.all(name).next().is_some()
    }

    pub fn max(&self, name: &str) -> Option<f64> {
        self.all(name).map(|f| f.value).reduce(f64::max)
    }

    pub fn min(&self, name: &str) -> Option<f64> {
        self.all(name).map(|f| f.value).reduce(f64::min)
    }

    pub fn sum(&self, name: &str) -> Option<f64> {
        self.all(name).map(|f| f.value).reduce(|a, b| a + b)
    }

    /// Les séries de `name` dont la valeur vérifie `pred`, nommées par
    /// l'étiquette `label` (dédoublonnées, triées).
    pub fn names_where(&self, name: &str, label: &str, pred: impl Fn(f64) -> bool) -> Vec<String> {
        let mut out: Vec<String> = self
            .all(name)
            .filter(|f| pred(f.value))
            .map(|f| {
                let value = f.label(label);
                if value.is_empty() { "?".to_string() } else { value.to_string() }
            })
            .collect();
        out.sort();
        out.dedup();
        out
    }
}

/// « a, b, c and 2 more » : une preuve lisible sans liste interminable.
pub fn list(names: &[String]) -> String {
    const SHOWN: usize = 3;
    if names.len() <= SHOWN {
        names.join(", ")
    } else {
        format!("{} and {} more", names[..SHOWN].join(", "), names.len() - SHOWN)
    }
}

/// `1 update` / `3 updates`.
pub fn plural(count: f64, word: &str) -> String {
    let n = count.round() as i64;
    if n == 1 { format!("1 {word}") } else { format!("{n} {word}s") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_agregats_ignorent_les_autres_metriques() {
        let facts = Facts::from_pairs(&[
            ("a", &[("node", "n1")], 2.0),
            ("a", &[("node", "n2")], 5.0),
            ("b", &[], 9.0),
        ]);
        assert_eq!(facts.max("a"), Some(5.0));
        assert_eq!(facts.min("a"), Some(2.0));
        assert_eq!(facts.sum("a"), Some(7.0));
        assert_eq!(facts.max("c"), None);
        assert_eq!(facts.names_where("a", "node", |v| v > 3.0), ["n2"]);
    }

    #[test]
    fn la_liste_se_resume_au_dela_de_trois() {
        let names: Vec<String> = ["a", "b", "c", "d", "e"].iter().map(|s| s.to_string()).collect();
        assert_eq!(list(&names), "a, b, c and 2 more");
        assert_eq!(list(&names[..2]), "a, b");
        assert_eq!(plural(1.0, "update"), "1 update");
        assert_eq!(plural(3.0, "update"), "3 updates");
    }
}
