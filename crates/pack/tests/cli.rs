//! `dumbmonit pack lint` et `dumbmonit pack test` : codes de sortie et effets sur
//! le disque, contre une copie jetable d'un paquet de référence.

use std::path::{Path, PathBuf};

use dumbmonit_pack::cli::run;

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

fn reference() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packs/shelly-plug")
}

/// Copie du paquet dans un répertoire propre à ce test, que `Drop` nettoie.
struct Copy(PathBuf);

impl Copy {
    fn new(name: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("dumbmonit-pack-cli-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        copy_dir(&reference(), &root);
        Self(root)
    }

    fn path(&self) -> &str {
        self.0.to_str().unwrap()
    }
}

impl Drop for Copy {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap().flatten() {
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[test]
fn sans_commande_connue_l_usage_est_rendu() {
    assert_eq!(run(&[]), 2);
    assert_eq!(run(&args(&["frobnicate"])), 2);
    // `lint` et `test` demandent au moins un chemin.
    assert_eq!(run(&args(&["lint"])), 2);
    assert_eq!(run(&args(&["test"])), 2);
}

#[test]
fn lint_accepte_un_paquet_valide_par_repertoire_ou_par_fichier() {
    let dir = reference();
    assert_eq!(run(&args(&["lint", dir.to_str().unwrap()])), 0);
    assert_eq!(run(&args(&["lint", dir.join("pack.yaml").to_str().unwrap()])), 0);
}

#[test]
fn lint_refuse_un_chemin_absent_et_un_paquet_invalide() {
    assert_eq!(run(&args(&["lint", "/inexistant/paquet"])), 1);

    let copy = Copy::new("invalide");
    std::fs::write(Path::new(copy.path()).join("pack.yaml"), "id: tout seul\n").unwrap();
    assert_eq!(run(&args(&["lint", copy.path()])), 1);
    // Un seul paquet refusé suffit à échouer, même si un autre passe.
    let good = reference();
    assert_eq!(run(&args(&["lint", good.to_str().unwrap(), copy.path()])), 1);

    // Un répertoire sans pack.yaml.
    let empty =
        std::env::temp_dir().join(format!("dumbmonit-pack-cli-vide-{}", std::process::id()));
    std::fs::create_dir_all(&empty).unwrap();
    assert_eq!(run(&args(&["lint", empty.to_str().unwrap()])), 1);
    let _ = std::fs::remove_dir_all(&empty);
}

#[test]
fn test_rejoue_les_fixtures_et_compare_a_expected_prom() {
    let copy = Copy::new("rejeu");
    assert_eq!(run(&args(&["test", copy.path()])), 0, "le paquet de référence est à jour");

    // Une sortie attendue qui diverge fait échouer, et --update la réécrit.
    let expected = Path::new(copy.path()).join("expected.prom");
    std::fs::write(&expected, "dumbmonit_rien 1\n").unwrap();
    assert_eq!(run(&args(&["test", copy.path()])), 1);
    assert_eq!(run(&args(&["test", copy.path(), "--update"])), 0);
    assert!(std::fs::read_to_string(&expected).unwrap().contains("dumbmonit_shelly_plug"));
    assert_eq!(run(&args(&["test", copy.path()])), 0);

    // Sans expected.prom, le message dit de lancer --update.
    std::fs::remove_file(&expected).unwrap();
    assert_eq!(run(&args(&["test", copy.path()])), 1);
}

#[test]
fn test_echoue_sans_repertoire_ou_sans_fixtures() {
    // Seulement --update : aucun paquet à rejouer.
    assert_eq!(run(&args(&["test", "--update"])), 1);
    assert_eq!(run(&args(&["test", "/inexistant/paquet"])), 1);

    let copy = Copy::new("sans-fixtures");
    std::fs::remove_dir_all(Path::new(copy.path()).join("fixtures")).unwrap();
    // Sources sans fixture : le rejeu le signale mais l'extraction est vide.
    let code = run(&args(&["test", copy.path()]));
    assert_eq!(code, 1, "l'attendu ne correspond plus à une extraction vide");
}
