//! Disposition du mur : chaque widget dans une zone (barre du haut, barre du
//! bas, colonne de droite, ou masqué), dans l'ordre choisi par
//! l'administrateur. Des zones et un ordre, pas de grille libre.
//!
//! Un identifiant inconnu ou en double est écarté en silence (un ancien réglage,
//! un widget retiré comme la vidéo en direct) ; un widget absent de toutes les
//! zones (réglage d'avant cette option, widget ajouté depuis) est rangé dans sa
//! zone par défaut. La disposition n'allume rien : un widget éteint reste
//! invisible où qu'il soit rangé.

use serde::{Deserialize, Serialize};

/// Les widgets du mur, dans l'ordre par défaut de leur zone.
pub const WIDGETS: &[&str] =
    &["weather", "news", "cve", "crypto", "stocks", "fx", "claude", "openai"];

/// La zone d'un widget tant que l'administrateur n'en a pas choisi d'autre :
/// la météo, les titres et les vulnérabilités sous l'horloge, les cours et la
/// consommation d'IA dans la barre du bas.
fn default_zone(id: &str) -> Zone {
    match id {
        "weather" | "news" | "cve" => Zone::Right,
        _ => Zone::Bottom,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Zone {
    Bottom,
    Right,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Layout {
    pub top: Vec<String>,
    pub bottom: Vec<String>,
    pub right: Vec<String>,
    pub hidden: Vec<String>,
}

impl Layout {
    /// La disposition nettoyée et complète : chaque widget connu une fois et une
    /// seule, les absents à la fin de leur zone par défaut.
    pub fn resolve(self) -> Layout {
        let mut seen = std::collections::HashSet::new();
        let mut keep = |list: Vec<String>| -> Vec<String> {
            list.into_iter()
                .map(|id| id.trim().to_ascii_lowercase())
                .filter(|id| WIDGETS.contains(&id.as_str()) && seen.insert(id.clone()))
                .collect()
        };
        let mut out = Layout {
            top: keep(self.top),
            bottom: keep(self.bottom),
            right: keep(self.right),
            hidden: keep(self.hidden),
        };
        for id in WIDGETS {
            if seen.contains(*id) {
                continue;
            }
            let zone = match default_zone(id) {
                Zone::Bottom => &mut out.bottom,
                Zone::Right => &mut out.right,
            };
            zone.push((*id).to_string());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_places_every_widget_once() {
        let layout = Layout::default().resolve();
        assert_eq!(layout.right, ["weather", "news", "cve"]);
        assert_eq!(layout.bottom, ["crypto", "stocks", "fx", "claude", "openai"]);
        assert!(layout.top.is_empty() && layout.hidden.is_empty());
    }

    #[test]
    fn unknown_and_repeated_ids_are_dropped_and_missing_ones_added() {
        let layout = Layout {
            top: vec![" Crypto ".into(), "video".into(), "fx".into()],
            bottom: vec!["crypto".into()],
            right: vec![],
            hidden: vec!["news".into()],
        }
        .resolve();
        assert_eq!(layout.top, ["crypto", "fx"]);
        assert_eq!(layout.hidden, ["news"]);
        assert_eq!(layout.bottom, ["stocks", "claude", "openai"]);
        assert_eq!(layout.right, ["weather", "cve"]);
    }
}
