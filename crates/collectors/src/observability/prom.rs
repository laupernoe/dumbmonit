//! Lecture du format d'exposition Prometheus (texte, version 0.0.4).
//!
//! VictoriaMetrics, VictoriaLogs et Loki publient leur propre santé sur
//! `/metrics`. Seul le texte est lu : c'est ce que les trois servent par défaut,
//! et le format protobuf ou OpenMetrics n'apporterait rien ici. Le lecteur est
//! volontairement tolérant : une ligne qu'il ne comprend pas est sautée, pas
//! fatale — un serveur plus récent qui ajoute une construction exotique ne doit
//! pas coûter toute la collecte.
//!
//! Deux particularités réelles sont couvertes : VictoriaMetrics sépare ses
//! étiquettes par `, ` (virgule *et* espace), et une valeur d'étiquette peut
//! contenir des virgules, des accolades ou des guillemets échappés.

use std::collections::BTreeMap;

/// Une ligne de mesure : nom, étiquettes, valeur. L'horodatage éventuel est
/// ignoré, la collecte date elle-même ses échantillons.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub name: String,
    pub labels: BTreeMap<String, String>,
    pub value: f64,
}

impl Line {
    pub fn label(&self, key: &str) -> Option<&str> {
        self.labels.get(key).map(String::as_str)
    }
}

/// Toutes les mesures d'une page `/metrics`.
#[derive(Debug, Default, Clone)]
pub struct Exposition {
    lines: Vec<Line>,
}

impl Exposition {
    pub fn parse(text: &str) -> Self {
        Self { lines: text.lines().filter_map(parse_line).collect() }
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Les lignes d'une famille.
    pub fn family<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Line> + 'a {
        self.lines.iter().filter(move |line| line.name == name)
    }

    /// Vrai si la famille existe, même à zéro.
    pub fn has(&self, name: &str) -> bool {
        self.family(name).next().is_some()
    }

    /// La somme d'une famille, ou `None` si elle n'existe pas sur ce serveur.
    pub fn sum(&self, name: &str) -> Option<f64> {
        self.sum_where(name, |_| true)
    }

    /// La somme des lignes d'une famille qui passent le filtre ; `None` si la
    /// famille n'existe pas du tout (une famille présente mais filtrée à vide
    /// donne `Some(0.0)` : le compteur existe, il n'a rien compté).
    pub fn sum_where(&self, name: &str, keep: impl Fn(&Line) -> bool) -> Option<f64> {
        let mut found = false;
        let mut total = 0.0;
        for line in self.family(name) {
            found = true;
            if keep(line) && line.value.is_finite() {
                total += line.value;
            }
        }
        found.then_some(total)
    }

    /// La somme d'une famille, regroupée par une étiquette.
    pub fn sum_by(&self, name: &str, label: &str) -> BTreeMap<String, f64> {
        let mut groups = BTreeMap::new();
        for line in self.family(name) {
            if line.value.is_finite() {
                let key = line.label(label).unwrap_or_default().to_string();
                *groups.entry(key).or_insert(0.0) += line.value;
            }
        }
        groups
    }

    /// La première ligne d'une famille.
    pub fn first(&self, name: &str) -> Option<&Line> {
        self.lines.iter().find(|line| line.name == name)
    }

    /// La valeur d'un drapeau de ligne de commande, tel que les produits
    /// VictoriaMetrics le publient : `flag{name="…", value="…"} 1`.
    pub fn flag(&self, flag: &str) -> Option<&str> {
        self.family("flag").find(|line| line.label("name") == Some(flag))?.label("value")
    }
}

fn parse_line(raw: &str) -> Option<Line> {
    let line = raw.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let name_end = line.find(|c: char| c == '{' || c.is_whitespace())?;
    let name = &line[..name_end];
    if name.is_empty() {
        return None;
    }
    let mut rest = &line[name_end..];
    let mut labels = BTreeMap::new();
    if rest.starts_with('{') {
        let (parsed, after) = parse_labels(&rest[1..])?;
        labels = parsed;
        rest = after;
    }
    let value = parse_value(rest.split_whitespace().next()?)?;
    Some(Line { name: name.to_string(), labels, value })
}

/// Lit `clé="valeur", …}` et rend ce qui suit l'accolade fermante.
fn parse_labels(mut text: &str) -> Option<(BTreeMap<String, String>, &str)> {
    let mut labels = BTreeMap::new();
    loop {
        text = text.trim_start_matches(|c: char| c == ',' || c.is_whitespace());
        if let Some(after) = text.strip_prefix('}') {
            return Some((labels, after));
        }
        let eq = text.find('=')?;
        let key = text[..eq].trim().to_string();
        text = text[eq + 1..].trim_start().strip_prefix('"')?;
        let mut value = String::new();
        let mut chars = text.char_indices();
        let end = loop {
            let (index, ch) = chars.next()?;
            match ch {
                '\\' => match chars.next()?.1 {
                    'n' => value.push('\n'),
                    other => value.push(other),
                },
                '"' => break index,
                other => value.push(other),
            }
        };
        labels.insert(key, value);
        text = &text[end + 1..];
    }
}

fn parse_value(raw: &str) -> Option<f64> {
    match raw {
        "+Inf" | "Inf" => Some(f64::INFINITY),
        "-Inf" => Some(f64::NEG_INFINITY),
        "NaN" => Some(f64::NAN),
        other => other.parse().ok(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_etiquettes_separees_par_virgule_et_espace_sont_lues() {
        let page = Exposition::parse(
            "# HELP x y\n\
             vm_app_version{version=\"victoria-metrics-v1\", short_version=\"v1.152.0\"} 1\n\
             vm_rows_ignored_total{reason=\"big_timestamp\"} 3\n\
             vm_rows_ignored_total{reason=\"small_timestamp\"} 4 1700000000000\n\
             process_resident_memory_bytes 1.5e+06\n",
        );
        assert_eq!(page.first("vm_app_version").unwrap().label("short_version"), Some("v1.152.0"));
        assert_eq!(page.sum("vm_rows_ignored_total"), Some(7.0));
        assert_eq!(page.sum("process_resident_memory_bytes"), Some(1_500_000.0));
        assert_eq!(page.sum("absent"), None);
    }

    #[test]
    fn une_valeur_d_etiquette_peut_contenir_des_caracteres_speciaux() {
        let page = Exposition::parse(r#"m{path="/a,b}", msg="dit \"non\"\n", z="\\"} 2"#);
        let line = page.first("m").unwrap();
        assert_eq!(line.label("path"), Some("/a,b}"));
        assert_eq!(line.label("msg"), Some("dit \"non\"\n"));
        assert_eq!(line.label("z"), Some("\\"));
        assert_eq!(line.value, 2.0);
    }

    #[test]
    fn une_ligne_illisible_est_sautee_sans_perdre_les_autres() {
        let page = Exposition::parse("bon 1\nm{cle=\"ouverte 2\nautre +Inf\nencore NaN\n");
        assert_eq!(page.sum("bon"), Some(1.0));
        assert!(!page.has("m"));
        assert_eq!(page.first("autre").unwrap().value, f64::INFINITY);
        // NaN présent mais jamais additionné.
        assert_eq!(page.sum("encore"), Some(0.0));
    }

    #[test]
    fn un_drapeau_se_lit_par_son_nom() {
        let page = Exposition::parse(
            "flag{name=\"storage.minFreeDiskSpaceBytes\", value=\"10000000\", is_set=\"false\"} 1",
        );
        assert_eq!(page.flag("storage.minFreeDiskSpaceBytes"), Some("10000000"));
        assert_eq!(page.flag("retentionPeriod"), None);
    }

    #[test]
    fn une_famille_presente_mais_filtree_vaut_zero() {
        let page = Exposition::parse("m{level=\"info\"} 5\n");
        assert_eq!(page.sum_where("m", |l| l.label("level") == Some("error")), Some(0.0));
        assert_eq!(page.sum_by("m", "level").get("info"), Some(&5.0));
    }
}
