//! Active Directory, en lecture seule, par LDAP.
//!
//! Un compte de service ordinaire — un simple utilisateur du domaine, sans
//! aucun droit d'administration — lit presque tout ce qu'un auditeur regarde :
//! les contrôleurs et leurs rôles, les groupes privilégiés et leurs membres,
//! la stratégie de mot de passe, et chaque compte avec ses drapeaux. Ce module
//! en tire l'état du domaine et des constats de sécurité bruts (comptes sans
//! pré-authentification Kerberos, délégation sans contrainte, mot de passe de
//! krbtgt trop ancien, LAPS absent…), que la page affiche et qu'un score peut
//! ensuite pondérer.
//!
//! # Ce que LDAP seul ne permet pas
//!
//! * **L'état de la réplication** vient de l'attribut construit
//!   `msDS-ReplAllInboundNeighbors` du RootDSE, que l'annuaire ne rend qu'à un
//!   compte ayant le droit « Monitor Active Directory Replication » ; sans lui,
//!   la page le dit, sans rien inventer. `repsFrom`/`repsTo` sont binaires et
//!   réservés : ils ne sont pas lus. Seul le contrôleur interrogé est vu.
//! * **Les journaux d'événements, les GPO appliquées, SYSVOL, DNS** : hors de
//!   portée de LDAP.
//! * **`lastLogonTimestamp`** n'est répliqué que tous les 9 à 14 jours : un
//!   compte « inactif » l'est à deux semaines près.
//!
//! # Deux rythmes
//!
//! La santé (connexion, liaison, RootDSE, contrôleurs, rôles FSMO, groupes
//! privilégiés, stratégie) est lue à chaque interrogation : quelques requêtes
//! de base. L'inventaire complet (tous les comptes et ordinateurs) est relu en
//! tâche de fond toutes les `inventory_minutes` : énumérer l'annuaire chaque
//! minute chargerait les contrôleurs pour rien, et ne tiendrait pas dans le
//! délai d'une interrogation sur un grand domaine.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `security` | `ldaps` | `ldaps` (636), `starttls` (389) ou `plain` (389, en clair). |
//! | `allow_plaintext` | `false` | Accord explicite pour `plain`. |
//! | `port` | selon `security` | Port, si l'adresse n'en précise pas. |
//! | `ca_cert` | | Autorité qui a émis le certificat du contrôleur (PEM). |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable. |
//! | `request_timeout_seconds` | `8` | Délai par opération LDAP. |
//! | `check_all_dcs` | `true` | Vérifie le port LDAP de chaque contrôleur du domaine. |
//! | `stale_days_users` | `90` | Inactivité d'un utilisateur, en jours. |
//! | `stale_days_computers` | `90` | Inactivité d'un ordinateur, en jours. |
//! | `inventory_minutes` | `15` | Rythme de l'inventaire complet. |

pub mod analysis;
mod client;
pub mod decode;
mod metrics;
pub mod model;
mod options;
mod tls;
pub mod view;

pub use analysis::severity_counts;
pub use options::{DEFAULT_INVENTORY_MINUTES, DEFAULT_REQUEST_TIMEOUT_SECONDS, DEFAULT_STALE_DAYS};
pub use view::{
    ConnectionView, DcView, DomainView, Finding, FindingSeverity, FsmoView, InventoryView,
    MemberView, NeighborView, PolicyView, PrivilegedGroupView, ProbeObserver, ProbeView,
    ReplicationView,
};

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use dumbmonit_proto::{Collector, ProbeError, Sample, Target, TargetId};
use futures::FutureExt;
use futures::future::{BoxFuture, Shared};
use tracing::debug;

use analysis::{Observed, Settings};
use client::{Bind, Session};
use model::Inventory;
use options::Options;

const PROFILE_ID: &str = "activedirectory";

/// Attente de l'inventaire lors de la toute première interrogation, pour que la
/// page soit complète dès l'ajout quand le domaine est petit. Au-delà,
/// l'inventaire arrive à l'interrogation suivante.
const FIRST_INVENTORY_WAIT: Duration = Duration::from_secs(4);

/// Durée maximale d'un inventaire en tâche de fond.
const INVENTORY_BUDGET: Duration = Duration::from_secs(300);

/// Délai avant de retenter un inventaire qui a échoué.
const INVENTORY_RETRY: Duration = Duration::from_secs(300);

#[derive(Default)]
pub struct ActiveDirectoryCollector {
    observer: Option<Arc<dyn ProbeObserver>>,
    cache: Arc<Mutex<HashMap<TargetId, Slot>>>,
}

/// L'inventaire d'une cible, et celui qui est en cours.
#[derive(Default)]
struct Slot {
    /// Contrôleur et compte de l'inventaire : un changement l'invalide.
    key: String,
    inventory: Option<Arc<Inventory>>,
    pending: Option<Shared<BoxFuture<'static, ()>>>,
    last_error: Option<String>,
    last_failure: Option<Instant>,
}

impl ActiveDirectoryCollector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Enregistre le destinataire des vues d'interrogation.
    pub fn with_observer(mut self, observer: Arc<dyn ProbeObserver>) -> Self {
        self.observer = Some(observer);
        self
    }

    fn slots(&self) -> std::sync::MutexGuard<'_, HashMap<TargetId, Slot>> {
        self.cache.lock().unwrap_or_else(|poison| poison.into_inner())
    }

    /// L'inventaire en cache, relancé en tâche de fond s'il a vieilli.
    async fn inventory(
        &self,
        target: &Target,
        options: &Options,
    ) -> (Option<Arc<Inventory>>, Option<String>) {
        let key = format!("{}|{}", options.url(), options.bind_user);
        let now = chrono::Utc::now().timestamp();
        let (pending, first) = {
            let mut slots = self.slots();
            let slot = slots.entry(target.id).or_default();
            if slot.key != key {
                *slot = Slot { key: key.clone(), ..Slot::default() };
            }
            let fresh = slot.inventory.as_ref().is_some_and(|inventory| {
                now - inventory.refreshed_at < options.inventory_interval.as_secs() as i64
            });
            let backing_off = slot.last_failure.is_some_and(|at| at.elapsed() < INVENTORY_RETRY);
            let running = slot.pending.as_ref().filter(|p| p.peek().is_none()).cloned();
            let pending = match running {
                Some(running) => Some(running),
                None if !fresh && !backing_off => {
                    let shared = self.spawn_refresh(target.id, key, options.clone());
                    slot.pending = Some(shared.clone());
                    Some(shared)
                }
                None => None,
            };
            (pending, slot.inventory.is_none())
        };
        if first && let Some(pending) = pending {
            let _ = tokio::time::timeout(FIRST_INVENTORY_WAIT, pending).await;
        }
        let slots = self.slots();
        let slot = slots.get(&target.id);
        (slot.and_then(|s| s.inventory.clone()), slot.and_then(|s| s.last_error.clone()))
    }

    fn spawn_refresh(
        &self,
        id: TargetId,
        key: String,
        options: Options,
    ) -> Shared<BoxFuture<'static, ()>> {
        let cache = self.cache.clone();
        let handle = tokio::spawn(async move {
            let result = tokio::time::timeout(INVENTORY_BUDGET, read_inventory(&options)).await;
            let mut slots = cache.lock().unwrap_or_else(|poison| poison.into_inner());
            let Some(slot) = slots.get_mut(&id).filter(|slot| slot.key == key) else { return };
            match result {
                Ok(Ok(inventory)) => {
                    slot.inventory = Some(Arc::new(inventory));
                    slot.last_error = None;
                    slot.last_failure = None;
                }
                Ok(Err(error)) => {
                    debug!(target_id = id, %error, "inventaire Active Directory en échec");
                    slot.last_error = Some(format!("Inventory: {error}"));
                    slot.last_failure = Some(Instant::now());
                }
                Err(_) => {
                    slot.last_error = Some(format!(
                        "Inventory: not finished after {} seconds",
                        INVENTORY_BUDGET.as_secs()
                    ));
                    slot.last_failure = Some(Instant::now());
                }
            }
        });
        async move {
            let _ = handle.await;
        }
        .boxed()
        .shared()
    }
}

#[async_trait]
impl Collector for ActiveDirectoryCollector {
    fn kind(&self) -> &'static str {
        "activedirectory"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let options = Options::from_target(target)?;
        let mut view = ProbeView {
            probed_at: chrono::Utc::now().timestamp(),
            connection: ConnectionView {
                host: options.host.clone(),
                port: options.port,
                security: options.security.as_str().to_string(),
                certificate_verified: options.verifies_certificate(),
                ..ConnectionView::default()
            },
            ..ProbeView::default()
        };

        let (mut session, connect_s) = client::connect(&options).await?;
        view.connection.connect_seconds = Some(connect_s);
        let started = Instant::now();
        let bind = session.bind(&options.bind_user, &options.bind_password).await;
        view.connection.bind_seconds = Some(started.elapsed().as_secs_f64());
        match bind {
            Err(error) => {
                session.close().await;
                return Err(error);
            }
            Ok(Bind::Refused(reason)) => {
                session.close().await;
                // Le contrôleur répond : la sonde réussit, et `ad_bind_ok` à 0
                // déclenche l'alerte dédiée — un mot de passe de compte de
                // service expiré est une panne de supervision, pas une faute de
                // frappe à afficher sans prévenir personne.
                view.bind_error = Some(reason);
                let samples =
                    metrics::bind_failed_samples(&view, chrono::Utc::now().timestamp_millis());
                self.observe(target, &view).await;
                return Ok(samples);
            }
            Ok(Bind::Accepted) => {}
        }

        let observed = read_health(&mut session).await;
        session.close().await;
        let mut observed = observed?;
        if options.check_all_dcs {
            observed.reachability = client::check_reachability(
                &observed.dsas,
                observed.root.ds_service_name.as_deref(),
                options.port,
                Duration::from_secs(2),
            )
            .await;
        }

        let (inventory, inventory_error) = self.inventory(target, &options).await;
        if let Some(error) = inventory_error {
            observed.errors.push(error);
        }
        let settings = Settings {
            stale_days_users: options.stale_days_users,
            stale_days_computers: options.stale_days_computers,
        };
        let now = chrono::Utc::now();
        let view =
            analysis::build(view, &observed, inventory.as_deref(), settings, now.timestamp());
        let samples = metrics::samples(&view, &observed, now.timestamp_millis());
        self.observe(target, &view).await;
        Ok(samples)
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let options = Options::from_target(target)?;
        let (mut session, _) = client::connect(&options).await?;
        let bind = session.bind(&options.bind_user, &options.bind_password).await;
        let root = match bind {
            Ok(Bind::Accepted) => client::root_dse(&mut session).await.map(|_| ()),
            Ok(Bind::Refused(reason)) => Err(ProbeError::Auth(reason)),
            Err(error) => Err(error),
        };
        session.close().await;
        root.map(|()| Some(PROFILE_ID.to_string()))
    }
}

impl ActiveDirectoryCollector {
    async fn observe(&self, target: &Target, view: &ProbeView) {
        if let Some(observer) = &self.observer {
            observer.observe(target, view).await;
        }
    }
}

/// La partie rapide : RootDSE et objet domaine obligatoires, le reste au mieux.
async fn read_health(session: &mut Session) -> Result<Observed, ProbeError> {
    let root = client::root_dse(session).await?;
    let domain = client::domain_object(session, &root).await?;
    let mut errors = Vec::new();
    let mut keep = |what: &str, error: ProbeError| errors.push(format!("{what}: {error}"));

    let fsmo = client::fsmo(session, &root, &domain).await.unwrap_or_else(|e| {
        keep("FSMO roles", e);
        Vec::new()
    });
    let dsas = client::domain_controllers(session, &root).await.unwrap_or_else(|e| {
        keep("Domain controllers", e);
        Vec::new()
    });
    let dc_computers = client::dc_computers(session, &root).await.unwrap_or_else(|e| {
        keep("Domain controller computers", e);
        Vec::new()
    });
    let groups = client::privileged_groups(session, &root, &domain).await.unwrap_or_else(|e| {
        keep("Privileged groups", e);
        Vec::new()
    });
    let recycle_bin = client::recycle_bin(session, &root).await.unwrap_or_else(|e| {
        keep("Recycle Bin", e);
        None
    });
    Ok(Observed {
        root,
        domain,
        fsmo,
        dsas,
        dc_computers,
        groups,
        recycle_bin,
        reachability: Vec::new(),
        errors,
    })
}

/// L'inventaire complet, sur sa propre connexion.
async fn read_inventory(options: &Options) -> Result<Inventory, ProbeError> {
    let (mut session, _) = client::connect(options).await?;
    let result = async {
        match session.bind(&options.bind_user, &options.bind_password).await? {
            Bind::Accepted => {}
            Bind::Refused(reason) => return Err(ProbeError::Auth(reason)),
        }
        let root = client::root_dse(&mut session).await?;
        client::inventory(&mut session, &root, chrono::Utc::now().timestamp()).await
    }
    .await;
    session.close().await;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uptime::tags::test_support::cible;
    use dumbmonit_proto::Credential;

    fn target(address: &str) -> Target {
        let mut target = cible("activedirectory", address, &[("request_timeout_seconds", "2")]);
        target.credential =
            Credential::UsernamePassword { username: "svc".into(), password: "x".into() };
        target
    }

    #[tokio::test]
    async fn un_controleur_qui_refuse_la_connexion_est_injoignable() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let error = ActiveDirectoryCollector::new()
            .probe(&target(&format!("127.0.0.1:{port}")))
            .await
            .unwrap_err();
        assert!(error.means_down(), "{error:?}");
    }

    #[tokio::test]
    async fn sans_identifiants_la_configuration_est_refusee() {
        let mut t = target("dc1.corp.lan");
        t.credential = Credential::None;
        let error = ActiveDirectoryCollector::new().probe(&t).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)));
    }
}
