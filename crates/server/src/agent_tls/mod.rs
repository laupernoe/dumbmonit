//! Canal chiffré des agents : TLS 1.3, certificats clients, autorité interne.
//!
//! Les agents (et les agents relais) parlent au serveur sur un port qui leur
//! est réservé (`DUMBMONIT_AGENT_LISTEN`, 8443 par défaut), conçu pour être
//! exposé sur Internet sans mandataire :
//!
//! - **TLS 1.3 seulement**, suites AEAD (AES-GCM, ChaCha20-Poly1305), échange
//!   de clés hybride post-quantique **X25519MLKEM768** — le seul que l'agent
//!   propose ; le serveur accepte aussi X25519 pour un `curl` de diagnostic ;
//! - **autorité interne** créée au premier démarrage ([`pki`]), clé chiffrée
//!   par la clé de l'instance ; aucune action de l'utilisateur ;
//! - **certificat client** par machine : la paire de clés est générée sur
//!   l'agent, le serveur signe sa clé publique pour 30 jours à l'enrôlement
//!   (jeton d'enrôlement, une fois) puis au renouvellement (sous le
//!   certificat en cours). Supprimer la machine révoque le certificat ;
//! - **épinglage** : l'agent ne reconnaît le serveur qu'à l'empreinte SHA-256
//!   de l'autorité, que porte la commande d'installation.
//!
//! L'ancien canal (les mêmes routes sur le port de l'interface, en clair ou
//! derrière un mandataire) reste accepté tant que le réglage « exiger mTLS »
//! n'est pas posé — il l'est d'office sur une installation neuve. Ses réponses
//! annoncent le canal chiffré ([`dumbmonit_proto::AGENT_TLS_HEADER`]) : un
//! agent à jour y migre de lui-même.

mod listener;
pub mod pki;
mod routes;
mod tunnel;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use anyhow::{Context, Result};
use dumbmonit_proto::{TargetId, TlsOffer};
use rustls::ServerConfig;
use rustls::crypto::CryptoProvider;
use rustls::server::WebPkiClientVerifier;
use sqlx::SqlitePool;

pub use listener::{AgentIo, AgentListener, AgentPeer, Transport, listen};
pub use routes::{AgentCert, router};

use crate::auth::rate_limit::Buckets;
use crate::state::AppState;

/// Réglage « exiger mTLS pour les agents » : `on` / `off`.
pub const SETTING_REQUIRED: &str = "agent_mtls_required";

/// Instant (ms) de la mise à jour qui a introduit le canal chiffré sur une
/// instance qui avait déjà des agents : point de départ de
/// `DUMBMONIT_AGENT_LEGACY_DAYS`.
pub const SETTING_LEGACY_SINCE: &str = "agent_legacy_since";

/// Groupe d'échange de clés que l'agent propose, seul.
pub const POST_QUANTUM_GROUP: &str = "X25519MLKEM768";

/// État du canal en mémoire, partagé par l'écoute et l'API.
pub struct AgentTls {
    /// Configuration TLS courante ; remplacée à chaque nouveau certificat de
    /// serveur, sans couper les connexions établies.
    server_config: RwLock<Option<Arc<ServerConfig>>>,
    /// Empreintes épinglables (autorité active, puis suivante en rotation).
    pins: RwLock<Vec<String>>,
    /// Tentatives d'enrôlement manquées, par adresse.
    limits: Mutex<Buckets>,
    /// File des connexions établies du port des agents : les tunnels des
    /// relais y déposent les leurs. `None` tant que le port n'écoute pas.
    ready: Mutex<Option<listener::Ready>>,
    /// Tunnels ouverts, par agent relais.
    tunnels: Arc<Mutex<HashMap<TargetId, usize>>>,
}

impl Default for AgentTls {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentTls {
    pub fn new() -> Self {
        Self {
            server_config: RwLock::new(None),
            pins: RwLock::new(Vec::new()),
            limits: Mutex::new(Buckets::new()),
            ready: Mutex::new(None),
            tunnels: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Configuration TLS à servir, une fois l'autorité chargée.
    pub fn server_config(&self) -> Option<Arc<ServerConfig>> {
        self.server_config.read().ok().and_then(|config| config.clone())
    }

    /// Empreintes à épingler.
    pub fn pins(&self) -> Vec<String> {
        self.pins.read().map(|pins| pins.clone()).unwrap_or_default()
    }

    pub(crate) fn limits(&self) -> std::sync::MutexGuard<'_, Buckets> {
        self.limits.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn set_ready(&self, ready: listener::Ready) {
        if let Ok(mut slot) = self.ready.lock() {
            *slot = Some(ready);
        }
    }

    fn ready(&self) -> Option<listener::Ready> {
        self.ready.lock().ok().and_then(|slot| slot.clone())
    }

    /// Réserve une place de tunnel pour ce relais, ou `None` s'il a atteint
    /// son plafond. La place se libère quand la connexion se ferme.
    fn reserve_tunnel(&self, relay: TargetId) -> Option<tunnel::Slot> {
        tunnel::Slot::reserve(&self.tunnels, relay, tunnel::MAX_TUNNELS_PER_RELAY)
    }

    /// Tunnels ouverts par un relais en ce moment.
    pub fn tunnels_of(&self, relay: TargetId) -> usize {
        self.tunnels.lock().map(|open| open.get(&relay).copied().unwrap_or(0)).unwrap_or(0)
    }

    fn install(&self, config: Arc<ServerConfig>, pins: Vec<String>) {
        if let Ok(mut slot) = self.server_config.write() {
            *slot = Some(config);
        }
        if let Ok(mut slot) = self.pins.write() {
            *slot = pins;
        }
    }
}

/// Annonce du canal à joindre aux réponses faites aux agents, si le canal est
/// ouvert et l'autorité chargée.
pub fn offer(state: &AppState) -> Option<TlsOffer> {
    let port = state.config.agent_port()?;
    let pins = state.agent_tls().pins();
    (!pins.is_empty()).then_some(TlsOffer { port, pins })
}

/// Fournisseur cryptographique du canal : aws-lc-rs, TLS 1.3 seulement (fixé
/// par la configuration), échange de clés hybride post-quantique d'abord.
pub fn provider() -> Arc<CryptoProvider> {
    use rustls::crypto::aws_lc_rs::{self, kx_group};
    Arc::new(CryptoProvider {
        kx_groups: vec![kx_group::X25519MLKEM768, kx_group::X25519, kx_group::SECP256R1],
        ..aws_lc_rs::default_provider()
    })
}

/// Prépare le canal au démarrage : autorité (créée au premier lancement),
/// réglage d'exigence par défaut, certificat du serveur.
pub async fn init(state: &AppState) -> Result<()> {
    let created = pki::ensure_active(&state.pool, &state.cipher).await?;
    if created {
        tracing::info!("agent channel: internal certificate authority created");
    }
    if read_setting(&state.pool, SETTING_REQUIRED).await?.is_none() {
        let hosts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agent_hosts")
            .fetch_one(&state.pool)
            .await
            .context("counting agents")?;
        if hosts == 0 {
            // Installation neuve : aucun agent ne parle encore l'ancien canal,
            // il n'y a rien à ménager.
            write_setting(&state.pool, SETTING_REQUIRED, "on").await?;
        } else {
            // Instance mise à jour : les agents en place continuent par
            // l'ancien canal et migrent d'eux-mêmes une fois à jour.
            write_setting(&state.pool, SETTING_REQUIRED, "off").await?;
            let now = chrono::Utc::now().timestamp_millis().to_string();
            write_setting(&state.pool, SETTING_LEGACY_SINCE, &now).await?;
        }
    }
    reload(state).await
}

/// Refait le certificat du serveur sous l'autorité active et recharge la
/// configuration TLS (démarrage, chaque jour, fin de rotation).
pub async fn reload(state: &AppState) -> Result<()> {
    let authorities = pki::authorities(&state.pool).await?;
    let active =
        authorities.iter().find(|ca| ca.state == "active").context("no active agent authority")?;
    let identity = pki::issue_server(active, &state.cipher)?;

    let mut roots = rustls::RootCertStore::empty();
    for authority in &authorities {
        roots
            .add(rustls::pki_types::CertificateDer::from(authority.cert_der.clone()))
            .context("trusting an agent authority")?;
    }
    let provider = provider();
    // Certificat client proposé mais pas exigé à la poignée de main : la route
    // d'enrôlement est la seule ouverte sans lui, toutes les autres le refusent
    // (`routes::require_certificate`).
    let verifier = WebPkiClientVerifier::builder_with_provider(Arc::new(roots), provider.clone())
        .allow_unauthenticated()
        .build()
        .context("agent certificate verifier")?;
    let mut config = ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])
        .context("TLS 1.3")?
        .with_client_cert_verifier(verifier)
        .with_single_cert(identity.chain, identity.key)
        .context("agent channel certificate")?;
    config.alpn_protocols = vec![b"http/1.1".to_vec()];

    let mut pins: Vec<String> = authorities
        .iter()
        .filter(|ca| ca.state == "active")
        .map(|ca| ca.fingerprint.clone())
        .collect();
    pins.extend(
        authorities.iter().filter(|ca| ca.state == "next").map(|ca| ca.fingerprint.clone()),
    );
    state.agent_tls().install(Arc::new(config), pins);
    Ok(())
}

/// Renouvelle chaque jour le certificat du serveur (il vit trente jours).
pub fn spawn_refresh(state: AppState) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(24 * 3600));
        tick.tick().await;
        loop {
            tick.tick().await;
            if let Err(error) = reload(&state).await {
                tracing::warn!(error = %format!("{error:#}"), "agent channel certificate not renewed");
            }
        }
    });
}

/// L'ancien canal est-il refusé ? Seulement si le canal chiffré est ouvert :
/// sans lui, l'exiger couperait tous les agents.
pub async fn mtls_required(state: &AppState) -> Result<bool> {
    if state.config.agent_listen.is_none() {
        return Ok(false);
    }
    if read_setting(&state.pool, SETTING_REQUIRED).await?.as_deref() == Some("on") {
        return Ok(true);
    }
    Ok(legacy_deadline(state)
        .await?
        .is_some_and(|until| chrono::Utc::now().timestamp_millis() > until))
}

/// Fin de la période de transition fixée par `DUMBMONIT_AGENT_LEGACY_DAYS`.
pub async fn legacy_deadline(state: &AppState) -> Result<Option<i64>> {
    let Some(days) = state.config.agent_legacy_days else { return Ok(None) };
    let since = read_setting(&state.pool, SETTING_LEGACY_SINCE)
        .await?
        .and_then(|raw| raw.parse::<i64>().ok());
    Ok(since.map(|since| since + i64::from(days) * 24 * 3600 * 1000))
}

/// Pose le réglage d'exigence.
pub async fn set_required(pool: &SqlitePool, required: bool) -> Result<()> {
    write_setting(pool, SETTING_REQUIRED, if required { "on" } else { "off" }).await
}

/// Lu tel que l'administrateur l'a posé (sans l'échéance automatique).
pub async fn required_setting(pool: &SqlitePool) -> Result<bool> {
    Ok(read_setting(pool, SETTING_REQUIRED).await?.as_deref() == Some("on"))
}

/// Note par quel canal une machine vient de parler, et par quel relais.
pub async fn note_channel(
    pool: &SqlitePool,
    target_id: TargetId,
    cert: Option<&AgentCert>,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp_millis();
    note_route(pool, target_id, cert.and_then(|cert| cert.via_relay)).await?;
    match cert {
        Some(cert) => {
            sqlx::query("UPDATE agent_hosts SET tls_seen_ms = ?, tls_group = ? WHERE target_id = ?")
                .bind(now)
                .bind(cert.group.as_deref())
                .bind(target_id)
                .execute(pool)
                .await?
        }
        None => {
            sqlx::query("UPDATE agent_hosts SET legacy_seen_ms = ? WHERE target_id = ?")
                .bind(now)
                .bind(target_id)
                .execute(pool)
                .await?
        }
    };
    Ok(())
}

/// Range le relais par lequel une machine à agent joint le serveur (`None` :
/// en direct). C'est `via_agent` de sa cible : la topologie en fait son parent,
/// et la suppression des alertes la tient pour « derrière » ce relais — s'il
/// tombe, la machine passe en `suppressed`, pas en panne. Sans effet quand rien
/// ne change.
pub async fn note_route(
    pool: &SqlitePool,
    target_id: TargetId,
    via_relay: Option<TargetId>,
) -> Result<()> {
    sqlx::query(
        "UPDATE targets SET via_agent = ? \
         WHERE id = ? AND kind = 'agent' AND via_agent IS NOT ?",
    )
    .bind(via_relay)
    .bind(target_id)
    .bind(via_relay)
    .execute(pool)
    .await
    .context("recording the relay of an agent")?;
    Ok(())
}

async fn read_setting(pool: &SqlitePool, key: &str) -> Result<Option<String>> {
    Ok(sqlx::query_scalar("SELECT value FROM app_settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?)
}

async fn write_setting(pool: &SqlitePool, key: &str, value: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO app_settings (key, value) VALUES (?, ?) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}
