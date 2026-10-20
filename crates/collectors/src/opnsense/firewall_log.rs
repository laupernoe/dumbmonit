//! Journal du pare-feu, pour la carte « en direct » de la page d'un OPNsense.
//!
//! `GET /api/diagnostics/firewall/log?limit=N&digest=D` rend les dernières
//! lignes de `filterlog` déjà découpées par `read_log.py` d'OPNsense, la plus
//! récente d'abord. Avec `digest` (l'empreinte MD5 d'une ligne déjà lue), la
//! lecture s'arrête sur cette ligne, qui est rendue elle aussi : c'est à
//! l'appelant de l'écarter. Droit nécessaire, en lecture seule :
//! « Diagnostics: Logs: Firewall: Live View » (`page-diagnostics-logs-firewall-dynamic`),
//! dont le motif `api/diagnostics/firewall/log*` accepte les paramètres.
//!
//! Contrairement au reste de l'intégration, ce journal porte des adresses :
//! il n'est lu que si l'option `live_map` est allumée, et seulement quand
//! quelqu'un regarde la carte. Le serveur n'en garde que ce que la carte montre
//! (pays, port, règle, action) ; aucune adresse n'en sort.

use dumbmonit_proto::{ProbeError, Target};
use serde::{Deserialize, Deserializer};

use super::OpnsenseCollector;
use super::options::Options;

/// Lignes lues au plus par appel : de quoi couvrir quelques secondes d'un
/// pare-feu très sollicité sans faire tourner le script d'OPNsense longtemps.
pub const MAX_LINES: usize = 1000;

/// Une ligne du journal, telle que `read_log.py` la découpe. Tout est facultatif :
/// une ligne CARP n'a pas de ports, une ligne ICMP non plus, une version
/// d'OPNsense plus ancienne n'a pas `label`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct LogEntry {
    /// `pass`, `block`, `rdr`, `nat`…
    #[serde(default)]
    pub action: String,
    /// `in` ou `out`.
    #[serde(default)]
    pub dir: String,
    #[serde(default)]
    pub interface: String,
    /// `tcp`, `udp`, `icmp`…
    #[serde(default)]
    pub protoname: String,
    #[serde(default)]
    pub src: String,
    #[serde(default)]
    pub dst: String,
    #[serde(default, deserialize_with = "lenient_port")]
    pub dstport: Option<u16>,
    /// Description de la règle, vide quand elle n'en a pas.
    #[serde(default)]
    pub label: String,
    /// Identifiant de la règle (empreinte), stable d'un rechargement à l'autre.
    #[serde(default)]
    pub rid: String,
    /// ISO 8601 avec fuseau : `2026-10-19T09:41:07+02:00`.
    #[serde(default, rename = "__timestamp__")]
    pub timestamp: String,
    /// MD5 de la ligne brute.
    #[serde(default, rename = "__digest__")]
    pub digest: String,
}

/// Les ports arrivent en chaînes (`"443"`), parfois vides.
fn lenient_port<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<u16>, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(match value {
        serde_json::Value::String(s) => s.trim().parse().ok(),
        serde_json::Value::Number(n) => n.as_u64().and_then(|n| u16::try_from(n).ok()),
        _ => None,
    })
}

/// Vrai quand la cible a demandé la carte en direct (option `live_map`).
pub fn live_map_enabled(target: &Target) -> bool {
    target
        .tags
        .get("live_map")
        .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "true" | "1" | "yes" | "on"))
        .unwrap_or(false)
}

/// Les lignes plus récentes que `after` (l'empreinte de la dernière ligne déjà
/// lue), la plus récente d'abord ; tout le haut du journal sans `after`.
pub async fn read(
    target: &Target,
    after: Option<&str>,
    limit: usize,
) -> Result<Vec<LogEntry>, ProbeError> {
    let options = Options::from_target(target)?;
    let client = OpnsenseCollector::new().client(target, &options)?;
    let limit = limit.clamp(1, MAX_LINES);
    // L'empreinte est un MD5 en hexadécimal : rien d'autre ne part dans l'adresse.
    let digest = after.filter(|d| d.len() == 32 && d.bytes().all(|b| b.is_ascii_hexdigit()));
    let path = match digest {
        Some(d) => format!("/api/diagnostics/firewall/log?limit={limit}&digest={d}"),
        None => format!("/api/diagnostics/firewall/log?limit={limit}"),
    };
    let lines: Vec<LogEntry> = client.get_or_empty(&path).await.map_err(missing_privilege)?;
    Ok(newer_than(lines, digest))
}

/// Privilège qu'exige le journal du pare-feu en direct.
const LIVE_VIEW_PRIVILEGE: &str = "Diagnostics: Logs: Firewall: Live View";

/// Un refus sur le journal veut presque toujours dire que le groupe de la clé n'a
/// pas ce privilège, que les autres lectures n'exigent pas : on le nomme, plutôt
/// que de renvoyer le message générique sur la clé.
fn missing_privilege(error: ProbeError) -> ProbeError {
    match error {
        ProbeError::Auth(_) => ProbeError::Auth(format!(
            "OPNsense refused the firewall log (403). Add the privilege \u{201c}{LIVE_VIEW_PRIVILEGE}\u{201d} \
             to the API user's group (System \u{2192} Access \u{2192} Groups \u{2192} Edit, \
             Assigned Privileges)."
        )),
        other => other,
    }
}

/// Écarte la ligne `after` et ce qui la suit (déjà lu).
pub fn newer_than(lines: Vec<LogEntry>, after: Option<&str>) -> Vec<LogEntry> {
    match after {
        None => lines,
        Some(digest) => lines.into_iter().take_while(|l| l.digest != digest).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("testdata/firewall_log.json");

    fn lines() -> Vec<LogEntry> {
        serde_json::from_str(FIXTURE).expect("journal lisible")
    }

    #[test]
    fn les_lignes_du_journal_se_lisent_quel_que_soit_le_protocole() {
        let lines = lines();
        assert_eq!(lines.len(), 6);
        let first = &lines[0];
        assert_eq!(first.action, "block");
        assert_eq!(first.dir, "in");
        assert_eq!(first.protoname, "tcp");
        assert_eq!(first.src, "185.220.101.34");
        assert_eq!(first.dstport, Some(22));
        assert_eq!(first.label, "Default deny / state violation rule");
        assert_eq!(first.digest.len(), 32);
        let icmp = lines.iter().find(|l| l.protoname == "icmp").expect("icmp");
        assert_eq!(icmp.dstport, None, "pas de port pour ICMP");
        let v6 = lines.iter().find(|l| l.src.contains(':')).expect("IPv6");
        assert_eq!(v6.dstport, Some(443));
        let carp = lines.iter().find(|l| l.protoname == "carp").expect("carp");
        assert!(carp.label.is_empty(), "une ancienne version n'a pas de label");
    }

    #[test]
    fn seules_les_lignes_plus_recentes_que_l_empreinte_sont_gardees() {
        let lines = lines();
        let third = lines[2].digest.clone();
        let newer = newer_than(lines.clone(), Some(&third));
        assert_eq!(newer.len(), 2, "la ligne de l'empreinte et les suivantes sont déjà lues");
        assert_eq!(newer_than(lines.clone(), None).len(), 6);
        assert_eq!(newer_than(lines, Some("inconnue")).len(), 6);
    }

    #[test]
    fn un_refus_nomme_le_privilege_manquant() {
        let error = missing_privilege(ProbeError::Auth("generic".into()));
        let ProbeError::Auth(message) = error else { panic!("une erreur d'authentification") };
        assert!(message.contains("Diagnostics: Logs: Firewall: Live View"), "{message}");
        assert!(message.contains("System \u{2192} Access \u{2192} Groups"), "{message}");
        assert!(matches!(
            missing_privilege(ProbeError::Protocol("x".into())),
            ProbeError::Protocol(_)
        ));
    }

    #[test]
    fn la_carte_est_eteinte_par_defaut() {
        let mut target = Target {
            id: 1,
            name: "fw".into(),
            address: "fw.lan".into(),
            kind: "opnsense".into(),
            profile_id: None,
            parent_id: None,
            interval: std::time::Duration::from_secs(60),
            enabled: true,
            tags: std::collections::BTreeMap::new(),
            credential: dumbmonit_proto::Credential::None,
            group_name: String::new(),
            position: 0,
        };
        assert!(!live_map_enabled(&target));
        target.tags.insert("live_map".into(), "true".into());
        assert!(live_map_enabled(&target));
    }
}
