//! Lecture de `robots.txt`, le minimum pour être poli.
//!
//! On retient les groupes qui nous nomment (`User-agent: DumbMonit`), à défaut
//! ceux de `*`, et dans ces groupes les règles `Allow` et `Disallow`, avec les
//! jokers `*` et l'ancre `$` que tous les grands robots comprennent. La règle
//! la plus longue l'emporte ; à longueur égale, `Allow` gagne (RFC 9309).
//! Les lignes `Sitemap:` sont relevées au passage, quel que soit leur groupe.

/// Jeton de produit par lequel un `robots.txt` peut nous viser.
pub const PRODUCT_TOKEN: &str = "dumbmonit";

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Robots {
    /// `(autorisé, motif)`.
    rules: Vec<(bool, String)>,
    /// Adresses des plans de site annoncés.
    pub sitemaps: Vec<String>,
}

impl Robots {
    /// Analyse un `robots.txt` pour notre jeton de produit.
    pub fn parse(text: &str) -> Self {
        struct Group {
            agents: Vec<String>,
            rules: Vec<(bool, String)>,
        }
        let mut groups: Vec<Group> = Vec::new();
        let mut sitemaps = Vec::new();
        // Vrai tant que les lignes `User-agent` s'enchaînent : elles forment
        // alors un seul groupe.
        let mut collecting_agents = false;

        for raw in text.lines() {
            let line = raw.split('#').next().unwrap_or_default().trim();
            let Some((key, value)) = line.split_once(':') else { continue };
            let key = key.trim().to_ascii_lowercase();
            let value = value.trim();
            match key.as_str() {
                "user-agent" => {
                    if !collecting_agents {
                        groups.push(Group { agents: Vec::new(), rules: Vec::new() });
                        collecting_agents = true;
                    }
                    if let Some(group) = groups.last_mut() {
                        group.agents.push(value.to_ascii_lowercase());
                    }
                }
                "allow" | "disallow" => {
                    collecting_agents = false;
                    // Un `Disallow:` vide n'interdit rien.
                    if value.is_empty() {
                        continue;
                    }
                    if let Some(group) = groups.last_mut() {
                        group.rules.push((key == "allow", value.to_string()));
                    }
                }
                "sitemap" => {
                    // `Sitemap: https://…` : le `:` du schéma a été pris pour
                    // le séparateur ; on recolle.
                    let full = line[line.find(':').map_or(0, |at| at + 1)..].trim();
                    if !full.is_empty() {
                        sitemaps.push(full.to_string());
                    }
                }
                _ => collecting_agents = false,
            }
        }

        let ours: Vec<&Group> =
            groups.iter().filter(|g| g.agents.iter().any(|a| a == PRODUCT_TOKEN)).collect();
        let chosen = if ours.is_empty() {
            groups.iter().filter(|g| g.agents.iter().any(|a| a == "*")).collect()
        } else {
            ours
        };
        let rules = chosen.into_iter().flat_map(|g| g.rules.iter().cloned()).collect();
        Self { rules, sitemaps }
    }

    /// `path` : chemin et requête (`/a/b?c=d`).
    pub fn allows(&self, path: &str) -> bool {
        let mut best: Option<(usize, bool)> = None;
        for (allow, pattern) in &self.rules {
            if !matches(pattern, path) {
                continue;
            }
            let length = pattern.len();
            best = match best {
                Some((best_len, best_allow))
                    if best_len > length || (best_len == length && best_allow) =>
                {
                    Some((best_len, best_allow))
                }
                _ => Some((length, *allow)),
            };
        }
        best.is_none_or(|(_, allow)| allow)
    }
}

/// Le motif couvre-t-il le début de `path` ? `*` vaut n'importe quelle suite,
/// `$` final ancre la fin.
fn matches(pattern: &str, path: &str) -> bool {
    let (pattern, anchored) = match pattern.strip_suffix('$') {
        Some(stripped) => (stripped, true),
        None => (pattern, false),
    };
    let parts: Vec<&str> = pattern.split('*').collect();
    let Some(first) = parts.first() else { return true };
    let Some(mut rest) = path.strip_prefix(first) else { return false };
    let last = parts.len() - 1;
    for (index, part) in parts.iter().enumerate().skip(1) {
        if index == last && anchored {
            return rest.ends_with(part);
        }
        match rest.find(part) {
            Some(at) => rest = &rest[at + part.len()..],
            None => return false,
        }
    }
    !anchored || rest.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_groupe_qui_nous_nomme_remplace_celui_de_tous() {
        let robots = Robots::parse(
            "User-agent: *\nDisallow: /\n\n# pour nous\nUser-agent: Googlebot\nUser-agent: DumbMonit\nDisallow: /prive/\n",
        );
        assert!(robots.allows("/docs/"));
        assert!(!robots.allows("/prive/a"));
    }

    #[test]
    fn a_defaut_le_groupe_etoile_sapplique() {
        let robots = Robots::parse(
            "User-agent: Googlebot\nDisallow: /\n\nUser-agent: *\nDisallow: /admin\nDisallow:\nAllow: /admin/public\n\nSitemap: https://ex.fr/sitemap.xml\n",
        );
        assert!(robots.allows("/"));
        assert!(!robots.allows("/admin"));
        assert!(!robots.allows("/admin/x"));
        assert!(robots.allows("/admin/public/y"), "la règle la plus longue l'emporte");
        assert_eq!(robots.sitemaps, ["https://ex.fr/sitemap.xml"]);
    }

    #[test]
    fn jokers_et_ancre() {
        let robots =
            Robots::parse("User-agent: *\nDisallow: /*.php$\nDisallow: /tmp*/cache\nAllow: /p\n");
        assert!(!robots.allows("/index.php"));
        assert!(robots.allows("/index.php?x=1"), "l'ancre exige la fin");
        assert!(!robots.allows("/tmp-1/cache/a"));
        assert!(robots.allows("/tmp/autre"));
        assert!(robots.allows("/p"));
    }

    #[test]
    fn a_longueur_egale_allow_gagne_et_sans_regle_tout_est_permis() {
        let robots = Robots::parse("User-agent: *\nDisallow: /a\nAllow: /a\n");
        assert!(robots.allows("/a"));
        assert!(Robots::parse("").allows("/n-importe-quoi"));
        assert!(Robots::parse("n'importe quoi\nsans: structure").allows("/"));
    }
}
