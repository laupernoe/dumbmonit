//! Synology Photos : seulement la présence du paquet.
//!
//! Synology Photos expose une famille d'API bien documentée par la bibliothèque
//! communautaire `N4S4/synology-api` pour la consultation (`SYNO.Foto.Browse.*`,
//! `SYNO.Foto.Index`…), mais aucune de ses classes ne liste les appareils mobiles
//! qui sauvegardent leurs photos, ni leur dernière sauvegarde — recherché le
//! 5 octobre 2026 dans `synology_api/photos.py`
//! (<https://github.com/N4S4/synology-api/blob/master/synology_api/photos.py>) et
//! dans la documentation d'aide de Synology, sans trouver l'équivalent de la page
//! « Liste des clients » de Synology Drive. L'application mobile affiche l'état
//! de sa propre sauvegarde, mais rien ne le centralise côté NAS par une API.
//!
//! Ce module se limite donc à ce qui est honnête de publier : que le paquet est
//! installé et répond, lu non pas par un appel de plus — chacun exigerait de
//! connaître un compte configuré pour Photos, ce que le compte de supervision
//! n'est pas forcément — mais par la présence de son API dans le catalogue que
//! `SYNO.API.Info` annonce déjà. C'est le même indice que DumbMonit utilise pour
//! Hyper Backup et Active Backup for Business : un paquet arrêté ou absent
//! n'annonce pas son API, ce qui distingue « non installé » de « en échec ».

use dumbmonit_proto::{MetricKind, Sample};

/// Présente dès que le paquet Synology Photos tourne, quel que soit le compte
/// qui interroge : point d'entrée le plus stable de l'API, utilisé ici comme
/// simple indice de présence et jamais appelé.
pub const API_PRESENCE: &str = "SYNO.Foto.Index";

/// Le paquet est installé et son API répond au catalogue.
pub fn installed_sample(ts_ms: i64) -> Sample {
    Sample::new("synology_photos_installed", 1.0, MetricKind::Gauge, ts_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lechantillon_de_presence_vaut_toujours_un() {
        let sample = installed_sample(1000);
        assert_eq!(sample.metric, "synology_photos_installed");
        assert_eq!(sample.value, 1.0);
    }
}
