//! Secrets par référence.
//!
//! Un fichier de configuration se versionne : il ne porte jamais un secret en
//! clair. Les endroits qui en contiennent — l'identifiant d'un équipement
//! (`credential`) et les secrets d'un canal (`secrets`) — acceptent à la place
//! une référence, résolue au moment d'appliquer :
//!
//! * `${NOM}` : la variable d'environnement `NOM` ;
//! * `${file:/chemin}` : le contenu du fichier (secret Docker ou Kubernetes),
//!   sans le saut de ligne final. Un chemin relatif part du dossier du fichier.
//!
//! La résolution se fait là où le secret existe : par l'outil en ligne de
//! commande sur la machine de l'opérateur, ou par le serveur pour le dossier
//! surveillé. L'API, elle, refuse une référence restée telle quelle : un
//! administrateur ne doit pas pouvoir faire lire l'environnement du serveur
//! (donc `DUMBMONIT_SECRET`) à travers un canal qu'il contrôle.
//!
//! L'export fait l'inverse : il remplace chaque secret par une référence dont
//! le nom est dérivé de l'objet, et rend la liste des variables à définir.

use std::path::Path;

use serde_json::Value;

use super::ConfigDoc;
use crate::backup::bundle::target_ref;

/// Clés d'un identifiant d'équipement qui portent un secret.
const SECRET_KEYS: &[&str] = &["community", "token", "password", "passphrase", "secret"];

/// Premier `${…}` valide de `text` : début, fin (exclue) et contenu.
fn find_ref(text: &str) -> Option<(usize, usize, &str)> {
    let mut from = 0;
    while let Some(offset) = text[from..].find("${") {
        let start = from + offset;
        let rest = &text[start + 2..];
        if let Some(close) = rest.find('}') {
            let inner = &rest[..close];
            if is_reference(inner) {
                return Some((start, start + 2 + close + 1, inner));
            }
        }
        from = start + 2;
    }
    None
}

fn is_reference(inner: &str) -> bool {
    if let Some(path) = inner.strip_prefix("file:") {
        return !path.trim().is_empty();
    }
    let mut chars = inner.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Vrai si le texte contient encore une référence non résolue.
pub fn has_reference(text: &str) -> bool {
    find_ref(text).is_some()
}

/// Remplace chaque référence de `text`. `env` et `file` lisent la valeur ; une
/// valeur absente est une erreur qui nomme la référence (jamais son contenu).
pub fn resolve_str(
    text: &str,
    env: &dyn Fn(&str) -> Option<String>,
    file: &dyn Fn(&str) -> Result<String, String>,
) -> Result<String, String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some((start, end, inner)) = find_ref(rest) {
        out.push_str(&rest[..start]);
        let value = match inner.strip_prefix("file:") {
            Some(path) => file(path.trim())?,
            None => env(inner).ok_or_else(|| format!("the variable {inner} is not set"))?,
        };
        out.push_str(&value);
        rest = &rest[end..];
    }
    out.push_str(rest);
    Ok(out)
}

fn walk_strings(
    value: &mut Value,
    f: &mut dyn FnMut(&mut String) -> Result<(), String>,
) -> Result<(), String> {
    match value {
        Value::String(text) => f(text),
        Value::Array(items) => items.iter_mut().try_for_each(|item| walk_strings(item, f)),
        Value::Object(map) => map.values_mut().try_for_each(|item| walk_strings(item, f)),
        _ => Ok(()),
    }
}

/// Résout les références de tout le document avec l'environnement du processus
/// et le système de fichiers. `base` : dossier des chemins relatifs.
pub fn resolve_doc(doc: &mut ConfigDoc, base: Option<&Path>) -> Result<(), String> {
    let env = |name: &str| std::env::var(name).ok();
    let file = |path: &str| {
        let path = Path::new(path);
        let full = match base {
            Some(base) if path.is_relative() => base.join(path),
            _ => path.to_path_buf(),
        };
        std::fs::read_to_string(&full)
            .map(|text| text.trim_end_matches(['\n', '\r']).to_string())
            .map_err(|error| format!("the file {} cannot be read: {error}", full.display()))
    };
    let mut problems: Vec<String> = Vec::new();
    for target in &mut doc.targets {
        let owner = format!("targets {}", target_ref(&target.kind, &target.address));
        if let Some(credential) = &mut target.credential {
            collect(&owner, credential, &env, &file, &mut problems);
        }
    }
    for channel in &mut doc.channels {
        if let Some(secrets) = &mut channel.secrets {
            collect(&format!("channels {}", channel.name), secrets, &env, &file, &mut problems);
        }
    }
    if problems.is_empty() { Ok(()) } else { Err(problems.join("\n")) }
}

fn collect(
    owner: &str,
    value: &mut Value,
    env: &dyn Fn(&str) -> Option<String>,
    file: &dyn Fn(&str) -> Result<String, String>,
    problems: &mut Vec<String>,
) {
    let result = walk_strings(value, &mut |text| {
        *text = resolve_str(text, env, file)?;
        Ok(())
    });
    if let Err(error) = result {
        problems.push(format!("{owner}: {error}."));
    }
}

/// Les objets du document qui portent encore une référence non résolue.
pub fn unresolved(doc: &mut ConfigDoc) -> Vec<String> {
    let mut found = Vec::new();
    let mut check = |owner: String, value: &mut Value| {
        let _ = walk_strings(value, &mut |text| {
            if has_reference(text) && !found.contains(&owner) {
                found.push(owner.clone());
            }
            Ok(())
        });
    };
    for target in &mut doc.targets {
        if let Some(credential) = &mut target.credential {
            check(format!("targets {}", target_ref(&target.kind, &target.address)), credential);
        }
    }
    for channel in &mut doc.channels {
        if let Some(secrets) = &mut channel.secrets {
            check(format!("channels {}", channel.name), secrets);
        }
    }
    found
}

/// Nom de variable d'environnement dérivé de l'objet et du champ.
fn variable_name(parts: &[&str]) -> String {
    let mut name = String::from("DUMBMONIT");
    for part in parts {
        name.push('_');
        let mut last_underscore = true;
        for c in part.chars() {
            if c.is_ascii_alphanumeric() {
                name.push(c.to_ascii_uppercase());
                last_underscore = false;
            } else if !last_underscore {
                name.push('_');
                last_underscore = true;
            }
        }
        while name.ends_with('_') {
            name.pop();
        }
    }
    name
}

fn redact_value(
    value: &mut Value,
    owner: &[&str],
    path: &mut Vec<String>,
    all: bool,
    names: &mut Vec<String>,
) {
    match value {
        Value::String(text) => {
            let secret = all || path.last().is_some_and(|key| SECRET_KEYS.contains(&key.as_str()));
            // Une référence déjà posée, ou une valeur vide, reste telle quelle.
            if secret && !text.is_empty() && !has_reference(text) {
                let mut parts: Vec<&str> = owner.to_vec();
                parts.extend(path.iter().map(String::as_str));
                let name = variable_name(&parts);
                *text = format!("${{{name}}}");
                if !names.contains(&name) {
                    names.push(name);
                }
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter_mut().enumerate() {
                path.push(index.to_string());
                redact_value(item, owner, path, all, names);
                path.pop();
            }
        }
        Value::Object(map) => {
            for (key, item) in map.iter_mut() {
                path.push(key.clone());
                redact_value(item, owner, path, all, names);
                path.pop();
            }
        }
        _ => {}
    }
}

/// Remplace tous les secrets du document par des références et rend les noms de
/// variables à définir avant d'appliquer le fichier.
pub fn redact(doc: &mut ConfigDoc) -> Vec<String> {
    let mut names = Vec::new();
    for target in &mut doc.targets {
        let reference = target_ref(&target.kind, &target.address);
        if let Some(credential) = &mut target.credential {
            redact_value(
                credential,
                &["target", reference.as_str()],
                &mut Vec::new(),
                false,
                &mut names,
            );
        }
    }
    for channel in &mut doc.channels {
        if let Some(secrets) = &mut channel.secrets {
            redact_value(
                secrets,
                &["channel", channel.name.as_str()],
                &mut Vec::new(),
                true,
                &mut names,
            );
        }
    }
    names
}

/// Remplace, dans `desired`, chaque valeur qui porte une référence non résolue
/// par la valeur actuelle au même endroit. Sert au plan d'un fichier collé dans
/// l'interface : la référence ne peut pas être lue là, on suppose le secret
/// inchangé plutôt que d'annoncer une modification qu'on ne peut pas vérifier.
pub fn assume_unchanged(desired: &mut Value, current: Option<&Value>) {
    match desired {
        Value::String(text) if has_reference(text) => {
            if let Some(Value::String(old)) = current {
                *text = old.clone();
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter_mut().enumerate() {
                let old = current.and_then(|c| c.get(index));
                assume_unchanged(item, old);
            }
        }
        Value::Object(map) => {
            for (key, item) in map.iter_mut() {
                assume_unchanged(item, current.and_then(|c| c.get(key)));
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn env(name: &str) -> Option<String> {
        (name == "TOKEN").then(|| "s3cret".to_string())
    }

    fn no_file(path: &str) -> Result<String, String> {
        Err(format!("no file {path}"))
    }

    #[test]
    fn references_resolve_inside_a_string() {
        assert_eq!(resolve_str("a ${TOKEN} b", &env, &no_file).unwrap(), "a s3cret b");
        assert_eq!(resolve_str("plain $ text", &env, &no_file).unwrap(), "plain $ text");
        assert!(resolve_str("${MISSING}", &env, &no_file).unwrap_err().contains("MISSING"));
        assert!(resolve_str("${file:/x}", &env, &no_file).unwrap_err().contains("/x"));
    }

    #[test]
    fn template_placeholders_are_not_references() {
        assert!(!has_reference("${not a name}"));
        assert!(!has_reference("${}"));
        assert!(has_reference("x ${A_1}"));
    }

    #[test]
    fn redaction_never_keeps_a_secret() {
        let mut doc = ConfigDoc { version: 1, ..ConfigDoc::default() };
        doc.targets.push(crate::backup::bundle::BundleTarget {
            kind: "snmp".into(),
            address: "10.0.0.1".into(),
            name: "sw".into(),
            profile_id: None,
            parent: None,
            via_agent: None,
            interval_secs: 60,
            enabled: true,
            tags: Default::default(),
            credential: Some(json!({"type": "snmp_v3", "username": "bob",
                "auth": {"protocol": "sha", "passphrase": "hunter2"}})),
            group_name: String::new(),
            position: 0,
            location: None,
            multisite: None,
        });
        let names = redact(&mut doc);
        let text = serde_json::to_string(&doc.targets[0].credential).unwrap();
        assert!(!text.contains("hunter2"));
        assert!(text.contains("bob"));
        assert_eq!(names, vec!["DUMBMONIT_TARGET_SNMP_10_0_0_1_AUTH_PASSPHRASE".to_string()]);
    }

    #[test]
    fn unresolved_references_assume_the_current_value() {
        let mut desired = json!({"token": "${T}", "user": "x"});
        assume_unchanged(&mut desired, Some(&json!({"token": "old", "user": "y"})));
        assert_eq!(desired, json!({"token": "old", "user": "x"}));
    }
}
