//! Mode démonstration publique (`DUMBMONIT_DEMO=1`).
//!
//! Une instance exposée sur Internet, que n'importe qui peut ouvrir avec le
//! compte `demo` / `demo`. Trois garanties, chacune tenue en un seul endroit :
//!
//! - **lecture seule** : [`guard`] refuse toute écriture (hors connexion et
//!   déconnexion) avant qu'elle n'atteigne un gestionnaire, et quelques lectures
//!   qui ont un effet ou révèlent les visiteurs (journal d'audit, SSO, points
//!   d'entrée des agents et des heartbeats) ;
//! - **rien ne sort** : les notifications sont coupées à la source
//!   (`notify::disable_sending`), et aucune cible ne peut être créée — donc
//!   aucune sonde vers une adresse choisie par un visiteur ;
//! - **rien ne dérive** : la base est recréée à chaque démarrage à partir d'un
//!   parc fictif fixe ([`seed`]), servi par des équipements simulés en mémoire
//!   ([`estate`]) et complété d'un historique synthétique déterministe
//!   ([`backfill`]).

pub mod backfill;
pub mod estate;
pub mod seed;
pub mod synthetic;

use axum::Json;
use axum::extract::{Request, State};
use axum::http::{Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::state::AppState;

/// Réponse à toute écriture refusée : l'interface l'affiche telle quelle.
pub const READ_ONLY_MESSAGE: &str = "This is a read-only demo: changes are disabled. Install DumbMonit to try it on your own \
     devices.";

/// Identifiants du compte partagé, affichés sur l'écran de connexion.
pub const DEMO_USERNAME: &str = "demo";
pub const DEMO_PASSWORD: &str = "demo";

/// Efface la base d'une démonstration précédente : chaque démarrage repart du
/// même parc, quoi qu'il se soit passé avant.
pub async fn reset_database(path: &std::path::Path) -> anyhow::Result<()> {
    for suffix in ["", "-wal", "-shm", "-journal"] {
        let file = std::path::PathBuf::from(format!("{}{suffix}", path.display()));
        match tokio::fs::remove_file(&file).await {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(
                    anyhow::Error::from(error).context(format!("removing {}", file.display()))
                );
            }
        }
    }
    Ok(())
}

/// Remplace les sondes de disponibilité par leurs équivalents synthétiques :
/// en démonstration, aucune ne doit ouvrir de connexion.
pub fn register_synthetic(registry: &mut crate::collectors::Registry) {
    for kind in ["http", "tls", "ping", "tcp", "dns"] {
        registry.register(std::sync::Arc::new(synthetic::SyntheticProbe::new(kind)));
    }
}

/// Tâches de fond de la démonstration : l'agent simulé pousse ses mesures par le
/// vrai chemin d'ingestion, et l'historique rétroactif est écrit une fois.
pub fn spawn(state: AppState, seeded: seed::Seeded) {
    let agent_state = state.clone();
    let token = seeded.agent_token;
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(30));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tick.tick().await;
            if let Err(error) = seed::push_agent_batch(
                &agent_state.pool,
                &agent_state.cipher,
                &agent_state.sink,
                &token,
            )
            .await
            {
                tracing::warn!(%error, "demo: simulated agent push failed");
            }
        }
    });
    backfill::spawn(state, seeded.targets);
}

/// Seules écritures permises : ouvrir et fermer sa session.
const ALLOWED_WRITES: &[&str] = &["/api/auth/login", "/api/auth/logout"];

/// Lectures refusées elles aussi. Le journal d'audit contiendrait les adresses
/// des autres visiteurs ; les routes des agents, des heartbeats et du SSO
/// modifient un état ou appellent l'extérieur malgré leur verbe `GET`.
const DENIED_READS: &[&str] = &[
    "/api/auth/audit",
    "/api/auth/oidc",
    "/api/push",
    "/api/agent/commands",
    "/api/agent/relay",
    "/api/ingest",
    "/api/mcp",
];

/// Vrai si la requête doit être refusée en mode démonstration.
pub fn refused(method: &Method, path: &str) -> bool {
    let matches = |prefix: &&str| {
        path == *prefix || path.strip_prefix(prefix).is_some_and(|rest| rest.starts_with('/'))
    };
    if DENIED_READS.iter().any(matches) {
        return true;
    }
    let read = matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS);
    !read && !ALLOWED_WRITES.contains(&path)
}

/// Le garde de lecture seule, posé sur tout le routeur.
///
/// Il passe avant l'authentification : une écriture est refusée de la même
/// façon, session ou pas, et aucun gestionnaire n'est jamais atteint.
pub async fn guard(State(state): State<AppState>, request: Request, next: Next) -> Response {
    if state.config.demo && refused(request.method(), request.uri().path()) {
        return (StatusCode::FORBIDDEN, Json(json!({ "error": READ_ONLY_MESSAGE, "demo": true })))
            .into_response();
    }
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toute_ecriture_est_refusee_sauf_la_connexion() {
        for method in [Method::POST, Method::PUT, Method::PATCH, Method::DELETE] {
            assert!(refused(&method, "/api/targets"));
            assert!(refused(&method, "/api/targets/1"));
            assert!(refused(&method, "/api/auth/password"));
            assert!(refused(&method, "/api/auth/totp/enroll"));
            assert!(refused(&method, "/api/backup/restore"));
            assert!(refused(&method, "/prometheus/api/v1/query"));
        }
        assert!(!refused(&Method::POST, "/api/auth/login"));
        assert!(!refused(&Method::POST, "/api/auth/logout"));
        assert!(refused(&Method::POST, "/api/auth/login/totp"));
    }

    #[test]
    fn les_lectures_passent_sauf_celles_qui_trahissent_ou_agissent() {
        assert!(!refused(&Method::GET, "/api/targets"));
        assert!(!refused(&Method::GET, "/api/alerts"));
        assert!(!refused(&Method::GET, "/"));
        assert!(!refused(&Method::GET, "/api/auth/status"));
        assert!(refused(&Method::GET, "/api/auth/audit"));
        assert!(refused(&Method::GET, "/api/auth/oidc/start"));
        assert!(refused(&Method::GET, "/api/push/abc"));
        assert!(refused(&Method::GET, "/api/agent/commands"));
        assert!(!refused(&Method::GET, "/api/pushover"), "préfixe exact seulement");
    }
}
