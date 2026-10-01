//! Les paquets de référence du dépôt (`packs/`) passent la vérification et
//! leurs fixtures donnent exactement `expected.prom`.
//!
//! `DUMBMONIT_BLESS=1` réécrit les `expected.prom` après un changement voulu —
//! l'équivalent de `dumbmonit pack test <dir> --update`.

use std::path::PathBuf;

use dumbmonit_pack::Pack;
use dumbmonit_pack::fixture::{self, EXPECTED};

fn reference_dirs() -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packs");
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&root)
        .expect("répertoire packs/")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.join("pack.yaml").is_file())
        .collect();
    dirs.sort();
    dirs
}

#[test]
fn les_paquets_de_reference_sont_valides_et_rejouent_leurs_fixtures() {
    let dirs = reference_dirs();
    assert!(dirs.len() >= 3, "paquets de référence introuvables : {dirs:?}");
    for dir in dirs {
        let (dir, yaml) = fixture::load(&dir).expect("lecture du paquet");
        let pack = Pack::parse(&yaml).unwrap_or_else(|error| panic!("{}: {error}", dir.display()));
        assert!(pack.warnings().is_empty(), "{}: {:?}", dir.display(), pack.warnings());
        assert!(
            !pack.rules().is_empty(),
            "{}: un paquet de référence livre ses règles",
            dir.display()
        );

        let replay = fixture::replay(&pack, &dir).expect("rejeu des fixtures");
        assert!(replay.notes.is_empty(), "{}: {:?}", dir.display(), replay.notes);
        assert!(!replay.rendered.is_empty(), "{}: aucune mesure extraite", dir.display());

        let expected_path = dir.join(EXPECTED);
        if std::env::var_os("DUMBMONIT_BLESS").is_some() {
            std::fs::write(&expected_path, &replay.rendered).expect("écriture de expected.prom");
        }
        let expected = std::fs::read_to_string(&expected_path).expect("expected.prom");
        let differences = fixture::diff(&expected, &replay.rendered);
        assert!(
            differences.is_empty(),
            "{}: l'extraction a changé (DUMBMONIT_BLESS=1 pour l'accepter)\n{}",
            dir.display(),
            differences.join("\n")
        );

        // Chaque métrique citée par une règle sort bien des fixtures : une règle
        // de référence ne vise pas une série que personne n'a jamais vue.
        for rule in pack.rules() {
            let cited: Vec<&str> =
                pack.metric_names().filter(|name| rule.expr.contains(name)).collect();
            assert!(!cited.is_empty(), "{}: {}", dir.display(), rule.uid);
            for name in cited {
                assert!(
                    replay.rendered.contains(&format!("# TYPE {name} ")),
                    "{}: {} cite {name}, absent des fixtures",
                    dir.display(),
                    rule.uid
                );
            }
        }
    }
}
