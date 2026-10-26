//! Traitements du texte d'une configuration : nettoyage de la sortie d'un
//! terminal, lignes volatiles, masquage des secrets.

use std::sync::OnceLock;

use regex::Regex;

/// Séquences d'échappement d'un terminal (couleurs, déplacements du curseur,
/// effacements) : elles n'ont rien à faire dans une configuration.
fn ansi() -> &'static Regex {
    static ANSI: OnceLock<Regex> = OnceLock::new();
    ANSI.get_or_init(|| {
        Regex::new(r"\x1b\[[0-9;?]*[ -/]*[@-~]|\x1b[()][A-Za-z0-9]|\x1b[=>78EDMc]")
            .expect("ANSI expression")
    })
}

/// Nettoie la sortie brute d'un terminal : séquences d'échappement retirées,
/// retour chariot traité comme un retour en début de ligne (le texte qui suit
/// le dernier l'emporte, c'est ainsi qu'une invite de pagination s'efface),
/// retour arrière qui efface le caractère précédent, autres caractères de
/// contrôle supprimés. Les fins de ligne deviennent `\n`.
pub fn clean_terminal(raw: &str) -> String {
    let without = ansi().replace_all(raw, "");
    let mut out = String::with_capacity(without.len());
    for (index, line) in without.split('\n').enumerate() {
        if index > 0 {
            out.push('\n');
        }
        let line = line.split('\r').rfind(|part| !part.is_empty()).unwrap_or("");
        let mut kept: Vec<char> = Vec::with_capacity(line.len());
        for c in line.chars() {
            if c == '\u{8}' {
                kept.pop();
            } else if c == '\t' || !c.is_control() {
                kept.push(c);
            }
        }
        out.extend(kept);
    }
    out
}

/// Dernière ligne d'un texte déjà nettoyé.
pub fn last_line(text: &str) -> &str {
    text.rsplit('\n').next().unwrap_or("")
}

/// Retire les lignes que `rules` désignent et les espaces de fin de ligne.
pub fn strip_lines(text: &str, rules: &[Regex]) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let line = line.trim_end();
        if rules.iter().any(|rule| rule.is_match(line)) {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Forme comparable d'une configuration : sans les lignes volatiles
/// (horodatages, compteurs, valeurs rechiffrées à chaque lecture), sans
/// espaces de fin de ligne ni lignes vides en tête et en fin. Deux relevés de
/// même forme comparable sont la même version.
pub fn normalize(text: &str, volatile: &[Regex]) -> String {
    let stripped = strip_lines(text, volatile);
    let trimmed = stripped.trim_matches('\n');
    let mut out = String::with_capacity(trimmed.len() + 1);
    out.push_str(trimmed);
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

/// Masque les secrets : dans chaque ligne, le groupe `secret` de chaque règle
/// est remplacé par `tag(secret)`. L'appelant choisit l'étiquette — le serveur
/// y met une empreinte courte et keyed, pour qu'un secret modifié reste visible
/// dans un diff sans être révélé. Rend le texte et le nombre de valeurs masquées.
pub fn mask(text: &str, rules: &[Regex], tag: &dyn Fn(&str) -> String) -> (String, usize) {
    let mut out = String::with_capacity(text.len());
    let mut masked = 0usize;
    for (index, line) in text.split('\n').enumerate() {
        if index > 0 {
            out.push('\n');
        }
        // Plages à masquer dans cette ligne, toutes règles confondues.
        let mut spans: Vec<(usize, usize)> = Vec::new();
        for rule in rules {
            for captures in rule.captures_iter(line) {
                if let Some(secret) = captures.name("secret")
                    && !secret.as_str().is_empty()
                {
                    spans.push((secret.start(), secret.end()));
                }
            }
        }
        if spans.is_empty() {
            out.push_str(line);
            continue;
        }
        spans.sort_unstable();
        // Fusion des plages qui se chevauchent : deux règles qui visent le
        // même secret ne doivent pas en laisser dépasser un morceau.
        let mut merged: Vec<(usize, usize)> = Vec::with_capacity(spans.len());
        for (start, end) in spans {
            match merged.last_mut() {
                Some(last) if start <= last.1 => last.1 = last.1.max(end),
                _ => merged.push((start, end)),
            }
        }
        let mut cursor = 0;
        for (start, end) in merged {
            out.push_str(&line[cursor..start]);
            out.push_str(&tag(&line[start..end]));
            masked += 1;
            cursor = end;
        }
        out.push_str(&line[cursor..]);
    }
    (out, masked)
}

/// Sortie d'une commande en mode interactif : sans l'écho de la commande en
/// première ligne ni l'invite finale.
pub fn command_output(step: &str, command: &str) -> String {
    let cleaned = clean_terminal(step);
    let mut lines: Vec<&str> = cleaned.split('\n').collect();
    // L'invite qui a clos la commande.
    lines.pop();
    // L'écho de la commande (précédé de l'invite sur certains équipements).
    if lines.first().is_some_and(|first| first.trim_end().ends_with(command.trim())) {
        lines.remove(0);
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(patterns: &[&str]) -> Vec<Regex> {
        patterns.iter().map(|p| Regex::new(p).unwrap()).collect()
    }

    #[test]
    fn terminal_noise_is_removed() {
        let raw = "\x1b[1mconfig\x1b[0m line\r\n --More-- \r          \rnext\r\nab\u{8}c\n";
        assert_eq!(clean_terminal(raw), "config line\nnext\nac\n");
    }

    #[test]
    fn volatile_lines_do_not_count() {
        let volatile = rules(&["^! Last configuration change at "]);
        let a = "!\n! Last configuration change at 10:00\nhostname sw1  \n\n";
        let b = "!\n! Last configuration change at 11:30\nhostname sw1\n";
        assert_eq!(normalize(a, &volatile), normalize(b, &volatile));
        assert_ne!(normalize(a, &volatile), normalize("!\nhostname sw2\n", &volatile));
    }

    #[test]
    fn secrets_are_masked_and_counted() {
        let secret_rules = rules(&[
            r"\b(?:secret|password)(?: [0-9])? (?P<secret>\S+)",
            r"^\s*snmp-server community (?P<secret>\S+)",
        ]);
        let text = "enable secret 5 $1$abc$def\nsnmp-server community s3cret RO\nhostname sw1";
        let (masked, count) = mask(text, &secret_rules, &|_| "<masked>".to_string());
        assert_eq!(count, 2);
        assert!(!masked.contains("$1$abc$def"), "{masked}");
        assert!(!masked.contains("s3cret"), "{masked}");
        assert!(masked.contains("hostname sw1"));
        assert_eq!(masked.lines().count(), 3);
    }

    #[test]
    fn overlapping_rules_leave_nothing_behind() {
        let secret_rules = rules(&[r"key (?P<secret>\S+)", r"(?P<secret>abc\S*)"]);
        let (masked, _) = mask("key abcdef", &secret_rules, &|_| "*".to_string());
        assert_eq!(masked, "key *");
    }

    #[test]
    fn the_echo_and_the_prompt_are_dropped() {
        let step = "show running-config\r\nhostname sw1\r\n!\r\nsw1#";
        assert_eq!(command_output(step, "show running-config"), "hostname sw1\n!\n");
    }
}
