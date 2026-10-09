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

/// Préfixe des étiquettes recopiées depuis les `tags` de la cible.
pub const TAG_LABEL_PREFIX: &str = "tag_";

/// Préfixe sous lequel une étiquette d'un échantillon est gardée quand elle porte
/// le nom d'une étiquette d'identité (`target`, `host`) avec une autre valeur :
/// la convention de Prometheus (`honor_labels: false`).
pub const EXPORTED_LABEL_PREFIX: &str = "exported_";

/// Au-delà, la valeur d'un tag n'est pas recopiée sur les séries : elle serait
/// répétée sur chacune (un certificat en base64 pèse plus d'un kilo-octet).
pub const MAX_TAG_LABEL_LEN: usize = 256;

impl Target {
    /// Étiquettes automatiquement ajoutées à tous les échantillons de cette cible.
    ///
    /// Elles sont appliquées par le pipeline et non par les collecteurs, afin qu'un
    /// collecteur ne puisse pas les oublier ni les écraser (voir [`apply_identity`]).
    /// Tous les tags sont recopiés, sauf ceux qui ressemblent à un secret ;
    /// [`Target::base_labels_except`] écarte en plus les options du type.
    pub fn base_labels(&self) -> BTreeMap<String, String> {
        self.base_labels_except(|_| false)
    }

    /// Les mêmes étiquettes, sans les tags que `skip` désigne : les options que le
    /// collecteur lit dans les tags (port, `insecure_tls`, certificat d'autorité…)
    /// sont des réglages, pas des étiquettes que l'utilisateur a voulu poser.
    pub fn base_labels_except(&self, skip: impl Fn(&str) -> bool) -> BTreeMap<String, String> {
        let mut labels = BTreeMap::new();
        labels.insert("target".to_string(), self.id.to_string());
        labels.insert("host".to_string(), self.name.clone());
        for (key, value) in &self.tags {
            if skip(key) || looks_secret(key) || value.len() > MAX_TAG_LABEL_LEN {
                continue;
            }
            // Préfixées pour ne jamais entrer en collision avec les étiquettes système.
            // La clé vient de l'utilisateur ou d'un agent à l'enrôlement : tout ce
            // qui sort de `[a-zA-Z0-9_]` devient `_`, faute de quoi le nom
            // d'étiquette serait invalide (et l'échantillon écarté à l'écriture),
            // voire injecterait du texte dans le flux d'import.
            let key: String = key
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
                .collect();
            labels.insert(format!("{TAG_LABEL_PREFIX}{key}"), value.clone());
        }
        labels
    }
}

/// Pose les étiquettes d'identité `base` sur celles d'un échantillon.
///
/// L'identité prime toujours : une machine ne peut pas écrire dans les séries
/// d'une autre en forgeant `target`. Mais une étiquette du collecteur qui porte
/// le même nom avec une autre valeur n'est plus perdue : elle est gardée sous
/// `exported_<nom>`, sans quoi toutes les séries d'un même équipement qui ne se
/// distinguent que par elle se confondraient en une seule. Les `tag_*` venus
/// d'ailleurs (un agent relais applique sa propre version) sont retirés : seuls
/// ceux de `base` font foi.
pub fn apply_identity(base: &BTreeMap<String, String>, labels: &mut BTreeMap<String, String>) {
    labels.retain(|key, _| !key.starts_with(TAG_LABEL_PREFIX) || base.contains_key(key));
    for (key, value) in base {
        if let Some(own) = labels.insert(key.clone(), value.clone())
            && own != *value
            && !key.starts_with(TAG_LABEL_PREFIX)
        {
            labels.insert(format!("{EXPORTED_LABEL_PREFIX}{key}"), own);
        }
    }
}

/// Vrai pour une clé de tag qui nomme un secret (`api_key`, `ca_cert`,
/// `password`…) : sa valeur n'a rien à faire dans la base de séries, ni dans les
/// notifications qui recopient les étiquettes.
fn looks_secret(key: &str) -> bool {
    const WORDS: [&str; 12] = [
        "secret",
        "password",
        "passwd",
        "pass",
        "token",
        "cert",
        "key",
        "apikey",
        "credential",
        "credentials",
        "pem",
        "private",
    ];
    key.split(|c: char| !c.is_ascii_alphanumeric())
        .any(|word| WORDS.iter().any(|secret| word.eq_ignore_ascii_case(secret)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(tags: &[(&str, &str)]) -> Target {
        Target {
            id: 7,
            name: "nas".into(),
            address: "10.0.0.2".into(),
            kind: "snmp".into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            tags: tags.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect(),
            credential: Credential::None,
            group_name: String::new(),
            position: 0,
        }
    }

    fn labels(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect()
    }

    /// Une étiquette `host` posée par un collecteur ne disparaît plus sous le nom
    /// de la cible : les séries par hôte se confondaient (rapports npm, vcsim).
    #[test]
    fn une_etiquette_homonyme_du_collecteur_est_gardee_sous_exported() {
        let base = target(&[]).base_labels();
        let mut sample = labels(&[("host", "broken.lab.local"), ("type", "proxy")]);
        apply_identity(&base, &mut sample);
        assert_eq!(sample["host"], "nas");
        assert_eq!(sample["target"], "7");
        assert_eq!(sample["exported_host"], "broken.lab.local");
        assert_eq!(sample["type"], "proxy");

        // Même valeur : rien à garder.
        let mut same = labels(&[("host", "nas")]);
        apply_identity(&base, &mut same);
        assert!(!same.contains_key("exported_host"));
    }

    /// Un `tag_*` absent de l'identité (posé par un agent relais d'après ses
    /// propres règles) est retiré ; ceux de la cible font foi.
    #[test]
    fn seuls_les_tags_de_la_cible_font_foi() {
        let base = target(&[("salle", "cave")]).base_labels();
        let mut sample = labels(&[("tag_port", "8443"), ("tag_salle", "grenier")]);
        apply_identity(&base, &mut sample);
        assert_eq!(sample.get("tag_port"), None);
        assert_eq!(sample["tag_salle"], "cave");
        assert!(!sample.contains_key("exported_tag_salle"));
    }

    /// Rapports vcsim, unifi, k3s : les options du type (`port`, `insecure_tls`,
    /// le certificat d'autorité en base64) ne sont plus recopiées sur chaque série.
    #[test]
    fn les_options_et_les_secrets_ne_deviennent_pas_des_etiquettes() {
        let ca = "LS0tLS1CRUdJTi".repeat(100);
        let notes = "x".repeat(MAX_TAG_LABEL_LEN + 1);
        let target = target(&[
            ("port", "8443"),
            ("insecure_tls", "true"),
            ("ca_cert", ca.as_str()),
            ("api_key", "s3cr3t"),
            ("role", "lab"),
            ("notes", notes.as_str()),
        ]);
        let options = ["port", "insecure_tls", "ca_cert"];
        let labels = target.base_labels_except(|key| options.contains(&key));
        let keys: Vec<_> = labels.keys().map(String::as_str).collect();
        assert_eq!(keys, ["host", "tag_role", "target"]);
        // Sans liste d'options, les secrets et les valeurs démesurées restent écartés.
        let all = target.base_labels();
        assert!(all.contains_key("tag_port"));
        assert!(!all.contains_key("tag_ca_cert"));
        assert!(!all.contains_key("tag_api_key"));
        assert!(!all.contains_key("tag_notes"));
    }

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
