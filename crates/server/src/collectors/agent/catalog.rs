//! Mesures et vues du type « agent » pour la page de détail.
//!
//! Les autres types tiennent les leurs dans `crates/collectors/catalog/` ; l'agent
//! n'est pas un collecteur de ce paquet, son catalogue (`catalog.yaml`, même
//! format) est donc décrit ici, par le serveur. Quatre graphiques — CPU, mémoire,
//! E/S disque, réseau — et une rangée de chiffres ; les conteneurs gardent leur
//! panneau, le reste va sous « Toutes les mesures ».

use std::sync::OnceLock;

use dumbmonit_collectors::catalog::CatalogFile;
use dumbmonit_proto::Measures;

const CATALOG: &str = include_str!("catalog.yaml");

/// Le catalogue de l'agent, lu une fois.
pub fn measures() -> Option<Measures> {
    static MEASURES: OnceLock<Option<Measures>> = OnceLock::new();
    MEASURES
        .get_or_init(|| match CatalogFile::parse(CATALOG) {
            Ok(file) => Some(file.measures()),
            // Le test ci-dessous garantit la lecture : on ne fait que le noter.
            Err(error) => {
                tracing::error!(%error, "catalogue des mesures de l'agent ignoré");
                None
            }
        })
        .clone()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    /// Le texte de tous les fichiers `.rs` sous `dir`.
    fn sources(dir: &Path) -> String {
        let mut text = String::new();
        let mut pending = vec![dir.to_path_buf()];
        while let Some(dir) = pending.pop() {
            for entry in std::fs::read_dir(&dir).expect("répertoire lisible") {
                let path = entry.expect("entrée de répertoire").path();
                if path.is_dir() {
                    pending.push(path);
                } else if path.extension().is_some_and(|ext| ext == "rs") {
                    text.push_str(&std::fs::read_to_string(&path).expect("source lisible"));
                }
            }
        }
        text
    }

    #[test]
    fn le_catalogue_de_lagent_se_lit_et_garde_quatre_graphiques() {
        let file = CatalogFile::parse(CATALOG).expect("catalog.yaml lisible");
        let charts: Vec<_> =
            file.views.iter().filter(|view| view.view != "stat").map(|view| &*view.title).collect();
        assert_eq!(charts, ["CPU", "Memory", "Disk I/O", "Network"]);
        assert!(measures().is_some());
    }

    #[test]
    fn chaque_mesure_declaree_est_emise_par_lagent() {
        let file = CatalogFile::parse(CATALOG).expect("catalog.yaml lisible");
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../agent/src");
        let missing = file.unemitted(&sources(&root));
        assert!(missing.is_empty(), "jamais émis par crates/agent/src : {missing:?}");
    }
}
