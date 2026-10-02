//! Différence ligne à ligne entre deux versions d'une page.
//!
//! Algorithme de Myers (« An O(ND) Difference Algorithm », 1986), le même que
//! `git diff` par défaut : il trouve le plus court script d'édition, donc le
//! diff le plus lisible. Le début et la fin communs sont retirés d'abord, ce
//! qui ramène le cas courant — une page dont un paragraphe a bougé — à quelques
//! lignes. Au-delà de [`MAX_EDIT_DISTANCE`] modifications, la page a été
//! refondue : on renonce au détail et tout l'ancien milieu est retiré, tout le
//! nouveau ajouté, plutôt que d'y passer des secondes et des mégaoctets.
//!
//! Pour l'affichage, les longues plages inchangées sont réduites à une ligne
//! `skip` qui dit combien de lignes elle cache, avec trois lignes de contexte
//! de part et d'autre de chaque modification.

use serde::Serialize;

/// Nombre de modifications au-delà duquel on ne cherche plus le diff minimal.
/// La mémoire de la recherche croît comme son carré : mille donnent environ
/// seize mégaoctets au pire.
const MAX_EDIT_DISTANCE: usize = 1000;

/// Lignes de contexte gardées autour d'une modification.
pub const CONTEXT: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edit {
    Equal,
    Insert,
    Delete,
}

/// Une ligne du diff, telle que l'API la rend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiffLine {
    /// `equal`, `insert`, `delete` ou `skip`.
    pub op: &'static str,
    /// Texte de la ligne ; vide pour `skip`.
    pub text: String,
    /// Nombre de lignes inchangées cachées, pour `skip` seulement.
    pub count: Option<usize>,
}

/// Script d'édition qui mène de `before` à `after`.
pub fn diff<'a>(before: &[&'a str], after: &[&'a str]) -> Vec<(Edit, &'a str)> {
    let prefix = before.iter().zip(after).take_while(|(a, b)| a == b).count();
    let suffix = before[prefix..]
        .iter()
        .rev()
        .zip(after[prefix..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let middle_before = &before[prefix..before.len() - suffix];
    let middle_after = &after[prefix..after.len() - suffix];

    let mut edits: Vec<(Edit, &'a str)> =
        before[..prefix].iter().map(|l| (Edit::Equal, *l)).collect();
    match myers(middle_before, middle_after) {
        Some(middle) => edits.extend(middle),
        None => {
            edits.extend(middle_before.iter().map(|l| (Edit::Delete, *l)));
            edits.extend(middle_after.iter().map(|l| (Edit::Insert, *l)));
        }
    }
    edits.extend(before[before.len() - suffix..].iter().map(|l| (Edit::Equal, *l)));
    edits
}

/// Lignes ajoutées et retirées.
pub fn counts(edits: &[(Edit, &str)]) -> (usize, usize) {
    let added = edits.iter().filter(|(e, _)| *e == Edit::Insert).count();
    let removed = edits.iter().filter(|(e, _)| *e == Edit::Delete).count();
    (added, removed)
}

/// Le diff à afficher : `context` lignes autour de chaque modification, le
/// reste des plages inchangées réduit à une ligne `skip`.
pub fn render(edits: &[(Edit, &str)], context: usize) -> Vec<DiffLine> {
    let line = |edit: Edit, text: &str| DiffLine {
        op: match edit {
            Edit::Equal => "equal",
            Edit::Insert => "insert",
            Edit::Delete => "delete",
        },
        text: text.to_string(),
        count: None,
    };
    let skip = |count: usize| DiffLine { op: "skip", text: String::new(), count: Some(count) };

    let mut out = Vec::new();
    let mut i = 0;
    while i < edits.len() {
        if edits[i].0 != Edit::Equal {
            out.push(line(edits[i].0, edits[i].1));
            i += 1;
            continue;
        }
        let start = i;
        while i < edits.len() && edits[i].0 == Edit::Equal {
            i += 1;
        }
        let run = &edits[start..i];
        let leading = start == 0;
        let trailing = i == edits.len();
        // Ce qu'on garde au début et à la fin de la plage.
        let (head, tail) = match (leading, trailing) {
            (true, true) => (0, 0),
            (true, false) => (0, context),
            (false, true) => (context, 0),
            (false, false) => (context, context),
        };
        let hidden = run.len().saturating_sub(head + tail);
        // Cacher une seule ligne derrière une ligne « skip » n'économise rien.
        if hidden < 2 {
            out.extend(run.iter().map(|(e, t)| line(*e, t)));
            continue;
        }
        out.extend(run[..head].iter().map(|(e, t)| line(*e, t)));
        out.push(skip(hidden));
        out.extend(run[run.len() - tail..].iter().map(|(e, t)| line(*e, t)));
    }
    out
}

/// Plus court script d'édition, ou `None` s'il dépasse [`MAX_EDIT_DISTANCE`].
fn myers<'a>(a: &[&'a str], b: &[&'a str]) -> Option<Vec<(Edit, &'a str)>> {
    let n = a.len() as isize;
    let m = b.len() as isize;
    let limit = MAX_EDIT_DISTANCE.min(a.len() + b.len());
    // `v[k + offset]` : abscisse la plus avancée atteinte sur la diagonale `k`.
    let offset = limit as isize + 1;
    let index = |k: isize| (k + offset) as usize;
    let mut v = vec![0isize; 2 * limit + 3];
    let mut trace: Vec<Vec<isize>> = Vec::new();

    for d in 0..=limit as isize {
        trace.push(v.clone());
        let mut k = -d;
        while k <= d {
            let mut x = if k == -d || (k != d && v[index(k - 1)] < v[index(k + 1)]) {
                v[index(k + 1)]
            } else {
                v[index(k - 1)] + 1
            };
            let mut y = x - k;
            while x < n && y < m && a[x as usize] == b[y as usize] {
                x += 1;
                y += 1;
            }
            v[index(k)] = x;
            if x >= n && y >= m {
                return Some(backtrack(a, b, &trace, d, index));
            }
            k += 2;
        }
    }
    None
}

fn backtrack<'a>(
    a: &[&'a str],
    b: &[&'a str],
    trace: &[Vec<isize>],
    distance: isize,
    index: impl Fn(isize) -> usize,
) -> Vec<(Edit, &'a str)> {
    let mut edits = Vec::new();
    let mut x = a.len() as isize;
    let mut y = b.len() as isize;
    for d in (1..=distance).rev() {
        let v = &trace[d as usize];
        let k = x - y;
        let previous_k =
            if k == -d || (k != d && v[index(k - 1)] < v[index(k + 1)]) { k + 1 } else { k - 1 };
        let previous_x = v[index(previous_k)];
        let previous_y = previous_x - previous_k;
        while x > previous_x && y > previous_y {
            edits.push((Edit::Equal, a[(x - 1) as usize]));
            x -= 1;
            y -= 1;
        }
        if x == previous_x {
            edits.push((Edit::Insert, b[(y - 1) as usize]));
            y -= 1;
        } else {
            edits.push((Edit::Delete, a[(x - 1) as usize]));
            x -= 1;
        }
    }
    while x > 0 && y > 0 {
        edits.push((Edit::Equal, a[(x - 1) as usize]));
        x -= 1;
        y -= 1;
    }
    edits.reverse();
    edits
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str) -> Vec<&str> {
        text.split(' ').filter(|l| !l.is_empty()).collect()
    }

    /// Rejoue le script : il doit mener exactement de l'un à l'autre.
    fn replay(edits: &[(Edit, &str)]) -> (Vec<String>, Vec<String>) {
        let before = edits.iter().filter(|(e, _)| *e != Edit::Insert).map(|(_, t)| t.to_string());
        let after = edits.iter().filter(|(e, _)| *e != Edit::Delete).map(|(_, t)| t.to_string());
        (before.collect(), after.collect())
    }

    #[test]
    fn le_script_est_minimal_et_rejouable() {
        let a = lines("a b c a b b a");
        let b = lines("c b a b a c");
        let edits = diff(&a, &b);
        let (before, after) = replay(&edits);
        assert_eq!(before, a);
        assert_eq!(after, b);
        // Distance d'édition connue de l'exemple de Myers : 5.
        let (added, removed) = counts(&edits);
        assert_eq!(added + removed, 5);
    }

    #[test]
    fn cas_limites() {
        assert!(diff(&[], &[]).is_empty());
        assert_eq!(diff(&[], &["x"]), [(Edit::Insert, "x")]);
        assert_eq!(diff(&["x"], &[]), [(Edit::Delete, "x")]);
        assert_eq!(diff(&["x", "y"], &["x", "y"]), [(Edit::Equal, "x"), (Edit::Equal, "y")]);
        assert_eq!(
            diff(&["a", "b", "c"], &["a", "B", "c"]),
            [(Edit::Equal, "a"), (Edit::Delete, "b"), (Edit::Insert, "B"), (Edit::Equal, "c")]
        );
    }

    #[test]
    fn une_refonte_complete_reste_correcte_au_dela_de_la_limite() {
        let a: Vec<String> = (0..1500).map(|i| format!("ancien {i}")).collect();
        let b: Vec<String> = (0..1500).map(|i| format!("nouveau {i}")).collect();
        let a: Vec<&str> = a.iter().map(String::as_str).collect();
        let b: Vec<&str> = b.iter().map(String::as_str).collect();
        let edits = diff(&a, &b);
        assert_eq!(counts(&edits), (1500, 1500));
        let (before, after) = replay(&edits);
        assert_eq!(before, a);
        assert_eq!(after, b);
    }

    #[test]
    fn les_longues_plages_inchangees_sont_reduites_avec_trois_lignes_de_contexte() {
        let before: Vec<String> = (1..=20).map(|i| format!("l{i}")).collect();
        let mut after = before.clone();
        after[9] = "L10".into();
        let before: Vec<&str> = before.iter().map(String::as_str).collect();
        let after: Vec<&str> = after.iter().map(String::as_str).collect();
        let rendered = render(&diff(&before, &after), CONTEXT);
        let shape: Vec<String> = rendered
            .iter()
            .map(|l| match l.count {
                Some(n) => format!("skip:{n}"),
                None => format!("{}:{}", l.op, l.text),
            })
            .collect();
        assert_eq!(
            shape,
            [
                "skip:6",
                "equal:l7",
                "equal:l8",
                "equal:l9",
                "delete:l10",
                "insert:L10",
                "equal:l11",
                "equal:l12",
                "equal:l13",
                "skip:7",
            ]
        );
    }

    #[test]
    fn une_plage_courte_entre_deux_modifications_reste_entiere() {
        let before = lines("a x1 x2 x3 x4 x5 x6 x7 b");
        let after = lines("A x1 x2 x3 x4 x5 x6 x7 B");
        let rendered = render(&diff(&before, &after), CONTEXT);
        // Sept lignes inchangées, trois de chaque côté : une seule serait
        // cachée, ce qui ne vaut pas une ligne « skip ».
        assert!(rendered.iter().all(|l| l.op != "skip"));
        assert_eq!(rendered.len(), 11);
        let before = lines("a x1 x2 x3 x4 x5 x6 x7 x8 b");
        let after = lines("A x1 x2 x3 x4 x5 x6 x7 x8 B");
        let rendered = render(&diff(&before, &after), CONTEXT);
        assert_eq!(rendered.iter().filter(|l| l.op == "skip").count(), 1);
        assert_eq!(rendered.iter().find(|l| l.op == "skip").and_then(|l| l.count), Some(2));
    }

    #[test]
    fn sans_modification_tout_est_cache() {
        let same = lines("a b c d e");
        let rendered = render(&diff(&same, &same), CONTEXT);
        assert_eq!(rendered, [DiffLine { op: "skip", text: String::new(), count: Some(5) }]);
    }
}
