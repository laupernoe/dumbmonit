//! Applications auto-hébergées : Nextcloud, Immich, Paperless-ngx, Jellyfin et
//! Plex Media Server.
//!
//! Ces applications tombent rarement d'un bloc. Elles tombent *de côté* :
//! Nextcloud reste coincé en maintenance après une mise à jour, Immich empile
//! des miniatures qu'aucun travailleur ne traite, Paperless perd son Redis et
//! n'importe plus rien, Jellyfin rate son scan de bibliothèque chaque nuit.
//! La page s'ouvre, personne ne remarque rien. Chaque module lit ce que
//! l'application dit d'elle-même par son API d'administration, avec l'accès le
//! plus restreint qu'elle permette, sans jamais lire un fichier, une photo,
//! un document ou un média.
//!
//! * [`nextcloud`] — `status.php` et l'application `serverinfo` (jeton `NC-Token`) ;
//! * [`immich`] — `/api/server/*` et les files de travaux (clé d'API restreinte) ;
//! * [`paperless`] — `/api/status/`, `/api/tasks/`, `/api/remote_version/` ;
//! * [`jellyfin`] — `/System/Info`, `/ScheduledTasks`, `/Sessions`, `/Plugins` ;
//! * [`plex`] — `/`, `/identity`, `/status/sessions`, `/library/sections`,
//!   `/updater/status`, `/activities`.
//!
//! # Principes
//!
//! * **Les erreurs sont classées pour l'alerting.** Un jeton refusé donne
//!   `ProbeError::Auth` ; une application en maintenance ou dont une dépendance
//!   est tombée est une mesure, pas une panne de transport.
//! * **Un seul appel condamne.** Le premier (celui qui prouve que
//!   l'application répond et que le jeton est bon) fait échouer la sonde ; les
//!   suivants sont des compléments, comptés dans `<produit>_scrape_errors`
//!   quand ils manquent.
//! * **La cardinalité est bornée** : des sommes, et au plus [`MAX_NAMED`]
//!   séries nommées (une application à mettre à jour, une tâche en échec).
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `scheme` | celui du produit | `https` pour Nextcloud, `http` ailleurs. |
//! | `port` | celui du produit | 443, 2283, 8000, 8096 ou 32400. |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable. |
//! | `request_timeout_seconds` | `15` | Délai par requête HTTP. |

mod client;
pub mod immich;
pub mod jellyfin;
pub mod nextcloud;
pub(crate) mod options;
pub mod paperless;
pub mod plex;

use dumbmonit_proto::{Credential, MetricKind, ProbeError, Sample, Target};
use serde_json::Value;

use client::{Auth, HttpClient};
use options::Options;

pub use immich::ImmichCollector;
pub use jellyfin::JellyfinCollector;
pub use nextcloud::NextcloudCollector;
pub use options::DEFAULT_REQUEST_TIMEOUT;
pub use paperless::PaperlessCollector;
pub use plex::PlexCollector;

/// Au plus autant de séries nommées par famille (applications à mettre à
/// jour, tâches en échec, files de travaux) : au-delà, seul leur nombre compte.
pub const MAX_NAMED: usize = 32;

fn gauge(name: &str, value: f64, ts_ms: i64) -> Sample {
    Sample::new(name, value, MetricKind::Gauge, ts_ms)
}

fn flag(name: &str, value: bool, ts_ms: i64) -> Sample {
    gauge(name, if value { 1.0 } else { 0.0 }, ts_ms)
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// Un nombre, qu'il arrive en nombre ou en texte (`"2260992"`) : les
/// applications PHP et Python ne sont pas toujours constantes là-dessus.
fn number(value: Option<&Value>) -> Option<f64> {
    match value? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        _ => None,
    }
}

fn http_client(
    target: &Target,
    default_scheme: &'static str,
    default_port: u16,
    auth: Auth,
    product: &'static str,
) -> Result<HttpClient, ProbeError> {
    let options = Options::from_target(target, default_scheme, default_port)?;
    Ok(HttpClient::new(
        crate::http::client(options.insecure_tls)?,
        options.base_url,
        auth,
        options.request_timeout,
        product,
    ))
}

/// Le jeton d'une cible, sans espace ni retour à la ligne collés avec.
fn token(credential: &Credential) -> Option<&str> {
    match credential {
        Credential::ApiToken { token } => Some(token.trim()).filter(|t| !t.is_empty()),
        _ => None,
    }
}

/// Compte les appels complémentaires qui ont échoué, sans faire échouer la sonde.
fn settle<T>(
    outcome: Result<T, ProbeError>,
    errors: &mut u32,
    target_id: dumbmonit_proto::TargetId,
    path: &str,
) -> Option<T> {
    match outcome {
        Ok(value) => Some(value),
        Err(error) => {
            *errors += 1;
            tracing::warn!(target_id, path, %error, "appel complémentaire indisponible");
            None
        }
    }
}

/// Compare deux versions `v3.2.4`, `3.2.10`, `35.0.2.1` numériquement, composant
/// par composant. `None` si l'une des deux ne se lit pas : mieux vaut ne rien
/// dire qu'annoncer une mise à jour qui n'existe pas.
fn newer(candidate: &str, current: &str) -> Option<bool> {
    fn parts(version: &str) -> Option<Vec<u64>> {
        let version = version.trim().trim_start_matches(['v', 'V']);
        let core = version.split(['-', '+', ' ']).next()?;
        core.split('.').map(|part| part.parse().ok()).collect()
    }
    let (mut a, mut b) = (parts(candidate)?, parts(current)?);
    let len = a.len().max(b.len());
    a.resize(len, 0);
    b.resize(len, 0);
    Some(a > b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_versions_se_comparent_composant_par_composant() {
        assert_eq!(newer("v3.2.10", "v3.2.4"), Some(true));
        assert_eq!(newer("v3.2.4", "3.2.4"), Some(false));
        assert_eq!(newer("35.0.2.1", "35.0.1.1"), Some(true));
        assert_eq!(newer("1.43.4.10903-e5521bd8c", "1.43.3.10896-cb3ebc72d"), Some(true));
        assert_eq!(newer("3.2", "3.2.0"), Some(false));
        assert_eq!(newer("latest", "3.2.0"), None);
    }

    #[test]
    fn un_nombre_se_lit_en_nombre_ou_en_texte() {
        assert_eq!(number(Some(&serde_json::json!(12))), Some(12.0));
        assert_eq!(number(Some(&serde_json::json!("2260992"))), Some(2_260_992.0));
        assert_eq!(number(Some(&serde_json::json!(true))), Some(1.0));
        assert_eq!(number(Some(&serde_json::json!(null))), None);
        assert_eq!(number(None), None);
    }
}
