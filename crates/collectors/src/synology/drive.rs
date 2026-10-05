//! Synology Drive : état du service, dossiers d'équipe et connexions actives.
//!
//! Synology ne documente pas cette API, pas plus qu'il ne documente Active Backup
//! for Business (voir [`super::abb`]). Les trois API retenues ici —
//! `SYNO.SynologyDrive` (`get_status`), `SYNO.SynologyDrive.TeamFolders` (`list`)
//! et `SYNO.SynologyDrive.Connection` (`list`) — viennent de la bibliothèque
//! communautaire `N4S4/synology-api`, qui les appelle depuis les mêmes écrans que
//! la console d'administration de Synology Drive
//! (<https://github.com/N4S4/synology-api/blob/master/synology_api/drive_admin_console.py>,
//! relue le 5 octobre 2026).
//!
//! Ce que cette source confirme, et ce qu'elle ne confirme pas :
//!
//! * elle confirme le nom des trois API, leurs méthodes, et l'enveloppe des deux
//!   listes (`{"data": {"items": [...], "total": N}, "success": true}`) ;
//! * elle **ne documente aucun champ** des objets `items` — ni pour un dossier
//!   d'équipe, ni pour une connexion. L'aide de Synology décrit la page
//!   « Liste des clients » de l'interface comme montrant le nom de l'appareil,
//!   l'utilisateur, le type d'application, l'état en ligne, l'adresse IP et la
//!   localisation, mais ce sont des colonnes d'écran, pas des noms de champ JSON.
//!
//! Devant ce manque, ce module reste volontairement court :
//!
//! * l'état du service se résume à la réussite de l'appel `get_status`, sans
//!   lire aucun champ de sa réponse ;
//! * les dossiers d'équipe sont comptés, et nommés quand la clé la plus probable
//!   (`name`) est présente — sinon un repli stable (`team_folder_<id ou index>`) ;
//! * les connexions actives ne sont **comptées** que globalement : brancher un
//!   appareil nommé sur [`crate::client_devices`] demanderait de connaître la clé
//!   qui porte son nom, son utilisateur et sa dernière synchronisation, et aucune
//!   source ne la donne. Mieux vaut ce compte seul qu'un nom ou une date inventés.
//!   Relire une vraie réponse de NAS pour confirmer la forme de `items` est le
//!   travail qui débloquerait un jour le détail par appareil ; ce jour-là, étendre
//!   ce module plutôt que deviner son contenu ici.
//!
//! Une réponse qui ne correspond pas à cette forme (version de DSM différente,
//! paquet non configuré) donne simplement moins de métriques, jamais une sonde en
//! échec : exactement la même tolérance que pour Active Backup for Business.

use dumbmonit_proto::{MetricKind, ProbeError, Sample};
use serde_json::Value;

use super::client::DsmClient;

/// État et configuration générale du service Synology Drive.
pub const API_STATUS: &str = "SYNO.SynologyDrive";
/// Dossiers d'équipe.
pub const API_TEAM_FOLDERS: &str = "SYNO.SynologyDrive.TeamFolders";
/// Connexions actives (postes et mobiles synchronisés).
pub const API_CONNECTION: &str = "SYNO.SynologyDrive.Connection";

/// Version demandée aux trois API : comme pour Active Backup for Business,
/// aucune n'est documentée, et rien n'indique qu'une version plus récente change
/// quoi que ce soit ici. Ramenée dans l'intervalle annoncé par le NAS par
/// [`super::model::ApiCatalog::resolve`].
const VERSION: u32 = 1;

/// Dossiers d'équipe nommés individuellement, au-delà desquels seul leur nombre
/// compte — même borne que les autres familles de collecteurs.
const MAX_NAMED: usize = 32;

fn gauge(metric: &str, value: f64, ts_ms: i64) -> Sample {
    Sample::new(format!("synology_drive_{metric}"), value, MetricKind::Gauge, ts_ms)
}

/// Interroge `get_status` : seule sa réussite compte, sa réponse n'étant pas
/// documentée.
pub async fn status(dsm: &DsmClient, ts_ms: i64) -> Result<Vec<Sample>, ProbeError> {
    let _: Value = dsm.call(API_STATUS, VERSION, "get_status", &[]).await?;
    Ok(vec![gauge("up", 1.0, ts_ms)])
}

/// Interroge et traduit la liste des dossiers d'équipe.
pub async fn team_folders(dsm: &DsmClient, ts_ms: i64) -> Result<Vec<Sample>, ProbeError> {
    let data: Value = dsm.call(API_TEAM_FOLDERS, VERSION, "list", &[]).await?;
    Ok(team_folder_samples(&data, ts_ms))
}

/// Interroge et traduit le nombre de connexions actives.
pub async fn connections(dsm: &DsmClient, ts_ms: i64) -> Result<Vec<Sample>, ProbeError> {
    let data: Value = dsm.call(API_CONNECTION, VERSION, "list", &[]).await?;
    Ok(connection_samples(&data, ts_ms))
}

/// Le nom le plus probable d'une entrée dont la forme exacte n'est pas connue.
fn item_name(item: &Value, fallback: &str) -> String {
    for key in ["name", "display_name", "folder_name", "team_folder_name"] {
        if let Some(name) = item.get(key).and_then(Value::as_str).filter(|n| !n.trim().is_empty()) {
            return name.to_string();
        }
    }
    fallback.to_string()
}

fn items_total(data: &Value) -> Option<f64> {
    data.get("total")
        .and_then(Value::as_f64)
        .or_else(|| data.get("items").and_then(Value::as_array).map(|items| items.len() as f64))
}

fn team_folder_samples(data: &Value, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    if let Some(total) = items_total(data) {
        out.push(gauge("team_folders", total, ts_ms));
    }
    if let Some(items) = data.get("items").and_then(Value::as_array) {
        for (index, item) in items.iter().take(MAX_NAMED).enumerate() {
            let fallback = format!("team_folder_{index}");
            out.push(
                gauge("team_folder_info", 1.0, ts_ms)
                    .with_label("name", item_name(item, &fallback)),
            );
        }
    }
    out
}

fn connection_samples(data: &Value, ts_ms: i64) -> Vec<Sample> {
    items_total(data).map(|total| vec![gauge("connections", total, ts_ms)]).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    #[test]
    fn les_dossiers_dequipe_sont_comptes_et_nommes_au_mieux() {
        let data = serde_json::json!({
            "total": 2,
            "items": [
                {"name": "Projets"},
                {"id": 7},
            ],
        });
        let samples = team_folder_samples(&data, 0);
        assert_eq!(value(&samples, "synology_drive_team_folders"), Some(2.0));
        let named: Vec<_> =
            samples.iter().filter(|s| s.metric == "synology_drive_team_folder_info").collect();
        assert_eq!(named.len(), 2);
        assert_eq!(named[0].labels["name"], "Projets");
        // Sans clé de nom reconnue, un repli stable plutôt qu'un champ vide.
        assert_eq!(named[1].labels["name"], "team_folder_1");
    }

    #[test]
    fn le_compte_de_dossiers_se_rabat_sur_la_taille_de_la_liste() {
        let data = serde_json::json!({"items": [{"name": "A"}, {"name": "B"}, {"name": "C"}]});
        assert_eq!(value(&team_folder_samples(&data, 0), "synology_drive_team_folders"), Some(3.0));
    }

    #[test]
    fn les_connexions_ne_publient_quun_compte_global() {
        let data = serde_json::json!({"total": 4, "items": [{}, {}, {}, {}]});
        let samples = connection_samples(&data, 0);
        assert_eq!(value(&samples, "synology_drive_connections"), Some(4.0));
        // Aucune métrique par appareil : la forme des entrées n'est pas connue.
        assert_eq!(samples.len(), 1);
    }

    #[test]
    fn une_reponse_sans_total_ni_liste_ne_publie_rien() {
        let data = serde_json::json!({});
        assert!(team_folder_samples(&data, 0).is_empty());
        assert!(connection_samples(&data, 0).is_empty());
    }
}
