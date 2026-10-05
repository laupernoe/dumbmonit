use std::collections::BTreeMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::Credential;

/// Identifiant d'une cible. Correspond au `rowid` SQLite.
pub type TargetId = i64;

/// Un équipement surveillé : switch, NAS, serveur, onduleur.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    pub id: TargetId,
    /// Nom affiché, modifiable par l'utilisateur.
    pub name: String,
    /// Adresse IP ou nom d'hôte, éventuellement suivi d'un port (`10.0.0.1:161`).
    pub address: String,
    /// Type de collecteur à utiliser : `snmp`, `agent`, `proxmox`, `synology`.
    pub kind: String,
    /// Profil de collecte appliqué. `None` tant que l'auto-détection n'a pas eu lieu.
    pub profile_id: Option<String>,
    /// Cible dont dépend celle-ci. Si le parent est injoignable, les alertes de cette
    /// cible sont supprimées plutôt que notifiées — c'est le mécanisme anti-cascade.
    pub parent_id: Option<TargetId>,
    /// Période d'interrogation.
    pub interval: Duration,
    pub enabled: bool,
    pub tags: BTreeMap<String, String>,
    pub credential: Credential,
    /// Dossier plat et libre, affiché sur `/targets`. Vide : aucun dossier.
    pub group_name: String,
    /// Rang manuel, départagé par l'interface seulement entre cibles de même
    /// état (et, de premier niveau, du même dossier) : un équipement en panne
    /// reste toujours en tête, quel que soit l'ordre choisi.
    pub position: i64,
}

impl Target {
    /// Étiquettes automatiquement ajoutées à tous les échantillons de cette cible.
    ///
    /// Elles sont appliquées par le pipeline et non par les collecteurs, afin qu'un
    /// collecteur ne puisse pas les oublier ni les écraser.
    pub fn base_labels(&self) -> BTreeMap<String, String> {
        let mut labels = BTreeMap::new();
        labels.insert("target".to_string(), self.id.to_string());
        labels.insert("host".to_string(), self.name.clone());
        for (key, value) in &self.tags {
            // Préfixées pour ne jamais entrer en collision avec les étiquettes système.
            // La clé vient de l'utilisateur ou d'un agent à l'enrôlement : tout ce
            // qui sort de `[a-zA-Z0-9_]` devient `_`, faute de quoi le nom
            // d'étiquette serait invalide (et l'échantillon écarté à l'écriture),
            // voire injecterait du texte dans le flux d'import.
            let key: String = key
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
                .collect();
            labels.insert(format!("tag_{key}"), value.clone());
        }
        labels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_keys_become_valid_label_names() {
        let target = Target {
            id: 7,
            name: "nas".into(),
            address: "10.0.0.2".into(),
            kind: "snmp".into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            tags: BTreeMap::from([
                ("rack-location".to_string(), "b2".to_string()),
                ("x\"} 0 0\ndumbmonit_up{target".to_string(), "12".to_string()),
            ]),
            credential: Credential::None,
            group_name: String::new(),
            position: 0,
        };
        let labels = target.base_labels();
        assert_eq!(labels.get("tag_rack_location").map(String::as_str), Some("b2"));
        assert!(labels.keys().all(|k| k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')));
    }
}
