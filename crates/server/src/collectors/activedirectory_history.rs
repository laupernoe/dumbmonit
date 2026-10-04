//! Conservation de ce que la sonde Active Directory a vu.
//!
//! Le collecteur vit dans `dumbmonit-collectors` et ne connaît pas la base ; il
//! livre sa vue à un observateur. Celui-ci la range dans SQLite
//! (`db::activedirectory`), d'où l'API de la page relit les contrôleurs, les
//! groupes privilégiés et les constats sans réinterroger le contrôleur.

use async_trait::async_trait;
use dumbmonit_collectors::activedirectory::{ProbeObserver, ProbeView};
use dumbmonit_proto::Target;
use sqlx::SqlitePool;
use std::sync::Arc;
use tracing::warn;

use crate::db;

struct SqliteHistory {
    pool: SqlitePool,
}

#[async_trait]
impl ProbeObserver for SqliteHistory {
    async fn observe(&self, target: &Target, view: &ProbeView) {
        // Un échec d'écriture ne doit pas faire échouer la sonde : les métriques
        // sont déjà produites, seule la page prendra du retard.
        if let Err(error) = db::activedirectory::record_probe(&self.pool, target.id, view).await {
            warn!(target_id = target.id, ?error, "vue Active Directory non enregistrée");
        }
    }
}

/// L'observateur à donner au collecteur :
/// `ActiveDirectoryCollector::new().with_observer(…)`.
pub fn sqlite_observer(pool: SqlitePool) -> Arc<dyn ProbeObserver> {
    Arc::new(SqliteHistory { pool })
}
