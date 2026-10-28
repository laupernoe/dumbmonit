//! Envoi des lots au serveur DumbMonit.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use anyhow::{Context, Result};
use dumbmonit_proto::{
    AGENT_KEY_HEADER, AGENT_SECRET_HEADER, AGENT_TLS_HEADER, AgentCommand, CERT_REFUSED,
    COMMANDS_PATH, CommandReport, INGEST_PATH, LOGS_PATH, LogBatch, ProbeOutcome, PushAck,
    PushBatch, RELAY_PATH, RELAY_TUNNEL_PATH, RELAY_TUNNEL_PROTOCOL, TLS_ENROL_PATH,
    TLS_RENEW_PATH, TlsCertificate, TlsEnrolRequest, TlsOffer, TlsRenewRequest,
};
use tokio::sync::Notify;

use crate::tls::{self, Pins};

/// Ce qui peut arriver à un envoi, et surtout ce qu'il faut en faire.
///
/// La distinction est celle qui décide du sort du tampon : un lot refusé pour son
/// contenu ne deviendra jamais valide et doit être jeté, alors qu'un serveur
/// injoignable finira par revenir et mérite qu'on garde les mesures.
#[derive(Debug)]
pub enum PushError {
    /// Serveur injoignable, en erreur, ou trop lent. On réessaie.
    Transport(String),
    /// Jeton refusé. On réessaie aussi — le jeton a pu être recréé côté serveur —
    /// mais en le disant clairement dans les journaux, car sans intervention
    /// humaine cette machine ne remontera plus jamais rien.
    Unauthorized,
    /// Le jeton est bon, mais le serveur ne reconnaît pas cette machine-ci :
    /// liaison inconnue, ou jeton qui ne peut plus enrôler. Réessayable, parce
    /// que quelqu'un peut ouvrir la fenêtre de réenrôlement dans l'interface
    /// pendant que l'agent patiente — mais le message doit être rendu tel quel,
    /// c'est lui qui dit quoi faire.
    Forbidden(String),
    /// Corps trop gros pour un intermédiaire (413). Le lot n'est pas fautif,
    /// sa taille si : on le garde, et on réessaie par morceaux plus petits.
    TooLarge(String),
    /// Certificat client refusé par le canal chiffré (révoqué, machine
    /// supprimée) : l'agent l'abandonne et se réenrôle avec son jeton.
    CertificateRefused,
    /// Le serveur a compris et refusé. Réessayer à l'identique est sans espoir.
    Rejected { status: u16, message: String },
}

impl PushError {
    /// Vrai si le lot mérite d'être conservé pour une nouvelle tentative.
    pub fn is_retryable(&self) -> bool {
        !matches!(self, Self::Rejected { .. })
    }
}

impl std::fmt::Display for PushError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(detail) => write!(f, "server unreachable: {detail}"),
            Self::Unauthorized => write!(f, "enrollment token rejected"),
            Self::Forbidden(message) => write!(f, "{message}"),
            Self::TooLarge(message) => {
                write!(f, "batch too large for the server or a proxy: {message}")
            }
            Self::Rejected { status, message } => write!(f, "batch rejected ({status}): {message}"),
            Self::CertificateRefused => write!(f, "agent certificate refused by the server"),
        }
    }
}

/// Le chemin vers le serveur : client HTTP et racine des URL.
struct Channel {
    http: reqwest::Client,
    /// Racine du serveur, sans barre oblique finale.
    base_url: String,
    /// Vrai sur le canal chiffré (TLS 1.3, autorité épinglée).
    encrypted: bool,
}

#[derive(Clone)]
pub struct PushClient {
    /// Canal courant, partagé par toutes les copies du client : quand l'agent
    /// passe au canal chiffré ou renouvelle son certificat, la boucle des
    /// commandes et celle du relais suivent à la requête suivante.
    channel: Arc<RwLock<Arc<Channel>>>,
    timeout: Duration,
    /// Empreintes épinglées, lues par le vérificateur TLS à chaque poignée de
    /// main ; tenues à jour par l'annonce du serveur sur le canal chiffré.
    pins: Pins,
    /// Dernière annonce du canal chiffré reçue du serveur.
    offer: Arc<Mutex<Option<TlsOffer>>>,
    token: String,
    /// Secret de liaison de cette machine, s'il en a un.
    ///
    /// Partagé entre les copies du client — la boucle de collecte, celle des
    /// commandes et celle du relais en tiennent chacune une — parce que le
    /// serveur peut l'attribuer en cours de route, dans l'accusé de réception
    /// d'un lot, et que les trois doivent le présenter dès l'instant d'après.
    secret: Arc<RwLock<Option<String>>>,
    /// Vrai une fois constaté que le serveur est antérieur à l'en-tête de clé :
    /// la clé repart alors dans l'URL, comme il l'attend.
    legacy_key: Arc<AtomicBool>,
    /// Réveillé à chaque réponse favorable du serveur, quel que soit le canal :
    /// la boucle des mesures en attente de reprise repart aussitôt que le relais
    /// a retrouvé le serveur, et inversement.
    reachable: Arc<Notify>,
}

impl PushClient {
    pub fn new(
        server_url: &str,
        token: &str,
        secret: Option<String>,
        timeout: Duration,
    ) -> Result<Self> {
        let channel = Channel {
            http: http_client(timeout, None)?,
            base_url: server_url.trim_end_matches('/').to_string(),
            encrypted: false,
        };
        Ok(Self {
            channel: Arc::new(RwLock::new(Arc::new(channel))),
            timeout,
            pins: Arc::new(RwLock::new(Vec::new())),
            offer: Arc::new(Mutex::new(None)),
            token: token.to_string(),
            secret: Arc::new(RwLock::new(secret)),
            legacy_key: Arc::new(AtomicBool::new(false)),
            reachable: Arc::new(Notify::new()),
        })
    }

    /// Canal courant.
    fn channel(&self) -> Arc<Channel> {
        self.channel
            .read()
            .map(|channel| channel.clone())
            .unwrap_or_else(|poisoned| poisoned.into_inner().clone())
    }

    /// Vrai une fois sur le canal chiffré.
    pub fn is_encrypted(&self) -> bool {
        self.channel().encrypted
    }

    /// Empreintes épinglées en ce moment.
    pub fn pins(&self) -> Vec<String> {
        self.pins.read().map(|pins| pins.clone()).unwrap_or_default()
    }

    /// Remplace les empreintes épinglées.
    pub fn set_pins(&self, pins: Vec<String>) {
        if let Ok(mut held) = self.pins.write() {
            *held = pins;
        }
    }

    /// Dernière annonce du canal chiffré, s'il y en a eu une.
    pub fn offer(&self) -> Option<TlsOffer> {
        self.offer.lock().ok().and_then(|offer| offer.clone())
    }

    /// Pose une annonce comme si le serveur l'avait envoyée (tests).
    #[cfg(test)]
    pub fn set_offer(&self, offer: TlsOffer) {
        if let Ok(mut held) = self.offer.lock() {
            *held = Some(offer);
        }
    }

    /// Passe au canal chiffré : `url` du port des agents, certificat client
    /// s'il y en a un. Plus aucune requête ne part en clair ensuite.
    pub fn use_tls(&self, url: &str, identity: Option<(Vec<u8>, Vec<u8>)>) -> Result<()> {
        let config = tls::client_config(self.pins.clone(), identity)?;
        let channel = Channel {
            http: http_client(self.timeout, Some(config))?,
            base_url: url.trim_end_matches('/').to_string(),
            encrypted: true,
        };
        if let Ok(mut held) = self.channel.write() {
            *held = Arc::new(channel);
        }
        Ok(())
    }

    /// Lit l'annonce du canal chiffré d'une réponse. Sur le canal chiffré, le
    /// serveur est authentifié : ses empreintes (rotation de l'autorité)
    /// remplacent celles qu'on épinglait. En clair, l'annonce est seulement
    /// gardée, pour la migration.
    fn observe(&self, response: &reqwest::Response, encrypted: bool) {
        let Some(offer) = response
            .headers()
            .get(AGENT_TLS_HEADER)
            .and_then(|value| value.to_str().ok())
            .and_then(TlsOffer::parse)
        else {
            return;
        };
        if encrypted {
            self.set_pins(offer.pins.clone());
        }
        if let Ok(mut held) = self.offer.lock() {
            *held = Some(offer);
        }
    }

    /// Enrôlement sur le canal chiffré : la clé publique de cette machine
    /// contre un certificat client. Le serveur est vérifié par `pins` ; le
    /// jeton (et le secret de liaison s'il y en a un) ne part qu'à lui.
    pub async fn enrol(
        &self,
        url: &str,
        request: &TlsEnrolRequest,
    ) -> Result<TlsCertificate, PushError> {
        let config = tls::client_config(self.pins.clone(), None)
            .map_err(|error| PushError::Transport(format!("{error:#}")))?;
        let http = http_client(self.timeout, Some(config))
            .map_err(|error| PushError::Transport(format!("{error:#}")))?;
        let target = format!("{}{TLS_ENROL_PATH}", url.trim_end_matches('/'));
        let response = self
            .identified(http.post(target).bearer_auth(&self.token))
            .json(request)
            .send()
            .await
            .map_err(|error| self.transport(&error))?;
        self.observe(&response, true);
        self.read_json(response).await
    }

    /// Renouvellement sous le certificat en cours, avec une nouvelle clé.
    pub async fn renew(&self, public_key: String) -> Result<TlsCertificate, PushError> {
        let channel = self.channel();
        let target = format!("{}{TLS_RENEW_PATH}", channel.base_url);
        let response = self
            .identified(channel.http.post(target).bearer_auth(&self.token))
            .json(&TlsRenewRequest { public_key })
            .send()
            .await
            .map_err(|error| self.transport(&error))?;
        self.observe(&response, channel.encrypted);
        self.read_json(response).await
    }

    /// Signal partagé « le serveur répond » : à attendre pendant une temporisation.
    pub fn reachable(&self) -> Arc<Notify> {
        self.reachable.clone()
    }

    /// Note une réponse favorable et réveille qui attendait le serveur.
    fn mark_reachable(&self) {
        self.reachable.notify_waiters();
    }

    /// Requête vers un canal d'agent (commandes, relais) : jeton, secret de
    /// liaison et clé d'identité, celle-ci dans son en-tête — jamais dans l'URL,
    /// sauf pour un serveur trop ancien pour lire l'en-tête.
    fn keyed(
        &self,
        method: reqwest::Method,
        path: &str,
        key: &str,
        query: &str,
    ) -> reqwest::RequestBuilder {
        let channel = self.channel();
        let mut url = format!("{}{path}", channel.base_url);
        let mut params: Vec<String> = Vec::new();
        if self.legacy_key.load(Ordering::Relaxed) {
            params.push(format!("key={}", percent_encode(key)));
        }
        if !query.is_empty() {
            params.push(query.to_string());
        }
        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }
        self.identified(
            channel
                .http
                .request(method, url)
                .bearer_auth(&self.token)
                .header(AGENT_KEY_HEADER, key),
        )
    }

    /// Envoie une requête de canal d'agent, et la rejoue une fois avec la clé dans
    /// l'URL si le serveur, antérieur à l'en-tête, la réclame là.
    async fn exchange(
        &self,
        build: impl Fn() -> reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, PushError> {
        let encrypted = self.is_encrypted();
        let response = build().send().await.map_err(|error| self.transport(&error))?;
        self.observe(&response, encrypted);
        if response.status() != reqwest::StatusCode::BAD_REQUEST
            || self.legacy_key.load(Ordering::Relaxed)
        {
            return Ok(response);
        }
        let body = response.text().await.unwrap_or_default();
        if !wants_legacy_key(&body) {
            return Err(rejection_of(reqwest::StatusCode::BAD_REQUEST, &body));
        }
        tracing::info!("older server: the machine key goes in the URL again");
        self.legacy_key.store(true, Ordering::Relaxed);
        build().send().await.map_err(|error| self.transport(&error))
    }

    /// Erreur de transport lisible : la cause d'abord, l'URL sans paramètres.
    fn transport(&self, error: &reqwest::Error) -> PushError {
        PushError::Transport(sanitise(&describe(error), &self.token))
    }

    /// Adopte le secret que le serveur vient d'attribuer.
    pub fn set_secret(&self, secret: Option<String>) {
        if let Ok(mut held) = self.secret.write() {
            *held = secret;
        }
    }

    /// Ajoute le secret de liaison à une requête, s'il y en a un.
    ///
    /// Un agent pas encore lié n'envoie pas l'en-tête du tout : c'est ce qui
    /// demande implicitement au serveur de lui en attribuer un.
    fn identified(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let secret = self.secret.read().ok().and_then(|held| held.clone());
        match secret {
            Some(secret) => request.header(AGENT_SECRET_HEADER, secret),
            None => request,
        }
    }

    /// URL effectivement appelée, pour la journalisation de démarrage.
    pub fn url(&self) -> String {
        ingest_url(&self.channel().base_url)
    }

    pub async fn send(&self, batch: &PushBatch) -> Result<PushAck, PushError> {
        let channel = self.channel();
        let response = self
            .identified(channel.http.post(ingest_url(&channel.base_url)).bearer_auth(&self.token))
            .json(batch)
            .send()
            .await
            .map_err(|error| self.transport(&error))?;
        self.observe(&response, channel.encrypted);

        let status = response.status();
        if status.is_success() {
            let ack = response
                .json::<PushAck>()
                .await
                .map_err(|error| PushError::Transport(format!("unreadable response: {error}")))?;
            self.mark_reachable();
            return Ok(ack);
        }

        Err(rejection(response).await)
    }

    /// Commandes en attente pour cette machine.
    ///
    /// La clé d'identité accompagne le jeton : celui-ci est partagé par tout un
    /// parc, il ne dit pas de quelle machine il s'agit.
    pub async fn fetch_commands(&self, key: &str) -> Result<Vec<AgentCommand>, PushError> {
        let path = commands_path(None);
        let response = self.exchange(|| self.keyed(reqwest::Method::GET, &path, key, "")).await?;
        self.read_json(response).await
    }

    /// Corps JSON d'une réponse favorable, ou qualification du refus.
    async fn read_json<T: serde::de::DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> Result<T, PushError> {
        if !response.status().is_success() {
            return Err(rejection(response).await);
        }
        let value = response
            .json::<T>()
            .await
            .map_err(|error| PushError::Transport(format!("unreadable response: {error}")))?;
        self.mark_reachable();
        Ok(value)
    }

    /// Réponse favorable sans corps attendu, ou qualification du refus.
    async fn read_empty(&self, response: reqwest::Response) -> Result<(), PushError> {
        if !response.status().is_success() {
            return Err(rejection(response).await);
        }
        self.mark_reachable();
        Ok(())
    }

    /// Lot de lignes de journal, sur le même canal que les commandes.
    pub async fn send_logs(&self, key: &str, batch: &LogBatch) -> Result<(), PushError> {
        let response = self
            .exchange(|| self.keyed(reqwest::Method::POST, LOGS_PATH, key, "").json(batch))
            .await?;
        self.read_empty(response).await
    }

    /// Compte rendu d'exécution d'une commande.
    pub async fn report_command(
        &self,
        key: &str,
        id: i64,
        report: &CommandReport,
    ) -> Result<(), PushError> {
        let path = commands_path(Some(id));
        let response = self
            .exchange(|| self.keyed(reqwest::Method::POST, &path, key, "").json(report))
            .await?;
        self.read_empty(response).await
    }
}

impl PushClient {
    /// Sondes que le serveur nous délègue (agent relais).
    ///
    /// Le serveur retient la réponse jusqu'à `wait` secondes s'il n'a rien à
    /// donner : `timeout` doit dépasser cette attente, sans quoi chaque tour
    /// vide finirait en erreur de transport.
    pub async fn fetch_probes(
        &self,
        key: &str,
        wait: Duration,
        timeout: Duration,
    ) -> Result<Vec<AgentCommand>, PushError> {
        let path = relay_path(None);
        let query = format!("wait={}", wait.as_secs());
        let response = self
            .exchange(|| self.keyed(reqwest::Method::GET, &path, key, &query).timeout(timeout))
            .await?;
        self.read_json(response).await
    }

    /// Mesures et verdict d'une sonde déléguée.
    pub async fn report_probe(
        &self,
        key: &str,
        id: i64,
        outcome: &ProbeOutcome,
    ) -> Result<(), PushError> {
        let path = relay_path(Some(id));
        let response = self
            .exchange(|| self.keyed(reqwest::Method::POST, &path, key, "").json(outcome))
            .await?;
        self.read_empty(response).await
    }

    /// Compte rendu progressif d'un balayage de découverte confié à cet agent.
    /// Rend `false` quand le serveur a répondu `410 Gone` : le balayage a été
    /// annulé, l'agent doit s'arrêter.
    pub async fn report_discovery(
        &self,
        key: &str,
        id: i64,
        report: &dumbmonit_proto::DiscoveryReport,
    ) -> Result<bool, PushError> {
        let path = format!("{}/{id}", dumbmonit_proto::DISCOVERY_REPORT_PATH);
        let response = self
            .exchange(|| self.keyed(reqwest::Method::POST, &path, key, "").json(report))
            .await?;
        if response.status() == reqwest::StatusCode::GONE {
            return Ok(false);
        }
        self.read_empty(response).await.map(|()| true)
    }
}

impl PushClient {
    /// Binaire proposé par le serveur pour la mise à jour de l'agent, par le
    /// canal courant (chiffré quand l'agent l'a) : au plus `max` octets.
    pub async fn download_update(
        &self,
        key: &str,
        file: &str,
        max: u64,
        timeout: Duration,
    ) -> Result<Vec<u8>, PushError> {
        let path = format!("{}/{file}", dumbmonit_proto::UPDATE_DOWNLOAD_PATH);
        let response = self
            .exchange(|| self.keyed(reqwest::Method::GET, &path, key, "").timeout(timeout))
            .await?;
        if !response.status().is_success() {
            return Err(rejection(response).await);
        }
        if response.content_length().is_some_and(|length| length > max) {
            return Err(PushError::TooLarge(format!("the agent binary exceeds {max} bytes")));
        }
        let bytes = response.bytes().await.map_err(|error| self.transport(&error))?;
        if bytes.len() as u64 > max {
            return Err(PushError::TooLarge(format!("the agent binary exceeds {max} bytes")));
        }
        self.mark_reachable();
        Ok(bytes.to_vec())
    }

    /// Ouvre, pour la passerelle du relais, un tunnel vers le port des agents
    /// du serveur : une mise à niveau HTTP sous le certificat client du relais.
    /// Le flux rendu est ensuite une connexion brute au port des agents — c'est
    /// l'agent du site qui y fait sa propre poignée de main TLS.
    ///
    /// Jamais par l'ancien canal en clair : seul un relais authentifié par son
    /// certificat peut ouvrir un tunnel.
    pub async fn open_tunnel(&self) -> Result<reqwest::Upgraded, PushError> {
        let channel = self.channel();
        if !channel.encrypted {
            return Err(PushError::Transport(
                "the relay is not on the encrypted agent channel yet".to_string(),
            ));
        }
        let target = format!("{}{RELAY_TUNNEL_PATH}", channel.base_url);
        let response = self
            .identified(channel.http.get(target).bearer_auth(&self.token))
            .header(reqwest::header::CONNECTION, "upgrade")
            .header(reqwest::header::UPGRADE, RELAY_TUNNEL_PROTOCOL)
            .send()
            .await
            .map_err(|error| self.transport(&error))?;
        if response.status() != reqwest::StatusCode::SWITCHING_PROTOCOLS {
            return Err(rejection(response).await);
        }
        self.mark_reachable();
        response.upgrade().await.map_err(|error| self.transport(&error))
    }

    /// Fichier d'installation (`install.sh`, `install.ps1`, `download/<nom>`)
    /// relu pour les machines du site, par le canal chiffré du relais. La
    /// réponse est rendue telle quelle, statut compris, pour être recopiée.
    pub async fn fetch_relay_file(&self, file: &str) -> Result<reqwest::Response, PushError> {
        let channel = self.channel();
        if !channel.encrypted {
            return Err(PushError::Transport(
                "the relay is not on the encrypted agent channel yet".to_string(),
            ));
        }
        let target = format!("{}{}", channel.base_url, crate::gateway::files_path(file));
        self.identified(channel.http.get(target).bearer_auth(&self.token))
            .timeout(RELAY_FILE_TIMEOUT)
            .send()
            .await
            .map_err(|error| self.transport(&error))
    }
}

/// Délai d'un fichier d'installation relu par le relais : un binaire d'agent
/// sur un lien lent.
const RELAY_FILE_TIMEOUT: Duration = Duration::from_secs(600);

/// Chemin des sondes déléguées, ou du compte rendu de l'une d'elles.
fn relay_path(id: Option<i64>) -> String {
    match id {
        Some(id) => format!("{RELAY_PATH}/{id}"),
        None => RELAY_PATH.to_string(),
    }
}

/// Vrai si un refus 400 vient d'un serveur qui attend la clé dans l'URL.
fn wants_legacy_key(body: &str) -> bool {
    server_error_message(body).is_some_and(|message| message.contains("'key' query parameter"))
}

/// Qualifie une réponse d'erreur du serveur.
async fn rejection(response: reqwest::Response) -> PushError {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    rejection_of(status, &body)
}

/// Qualifie un statut d'erreur et son corps.
fn rejection_of(status: reqwest::StatusCode, body: &str) -> PushError {
    let from_server = server_error_message(body).is_some();
    let message = extract_error_message(body);

    match status.as_u16() {
        // Sur le canal chiffré, un 401 peut viser le certificat plutôt que le
        // jeton : le serveur le dit par un message convenu.
        401 if message == CERT_REFUSED => PushError::CertificateRefused,
        401 => PushError::Unauthorized,
        // 403 : le jeton est bon, mais pas cette machine-là. Le message du
        // serveur dit quoi faire — il est rendu tel quel dans les journaux.
        403 => PushError::Forbidden(message),
        // 408, 429 et toute la famille 5xx traduisent un serveur momentanément
        // incapable de répondre : les mesures restent en tampon.
        408 | 429 | 500..=599 => PushError::Transport(format!("HTTP {status}: {message}")),
        // 413 ne vient jamais du serveur avec son enveloppe : c'est un plafond de
        // taille, le sien ou celui d'un mandataire. Le lot sera recoupé.
        413 => PushError::TooLarge(message),
        // Sans l'enveloppe `{"error": …}` du serveur, un 400 ou un 404 vient d'un
        // intermédiaire — chemin mal routé, maintenance, filtrage. Le lot n'y
        // est pour rien : jeter tout le retard sur cette foi serait une perte
        // sèche. Les mesures restent en tampon, l'agent réessaie.
        400 | 404 if !from_server => PushError::Transport(format!("HTTP {status}: {message}")),
        other => PushError::Rejected { status: other, message },
    }
}

/// Client HTTP d'un canal : en clair ou derrière l'ancien mandataire tel que
/// configuré, ou, avec `tls`, canal chiffré strict — HTTPS seulement, sans
/// redirection.
fn http_client(timeout: Duration, tls: Option<rustls::ClientConfig>) -> Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder()
        .timeout(timeout)
        // Le socket est réutilisé d'un envoi au suivant : sur un lien distant,
        // rétablir TLS toutes les trente secondes coûterait plus cher que la
        // collecte elle-même.
        .pool_idle_timeout(Duration::from_secs(300))
        .user_agent(concat!("dumbmonit-agent/", env!("CARGO_PKG_VERSION")));
    if let Some(config) = tls {
        builder = builder
            .tls_backend_preconfigured(config)
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none());
    }
    builder.build().context("building the HTTP client")
}

/// Construit l'URL d'ingestion à partir de la racine du serveur.
fn ingest_url(server_url: &str) -> String {
    format!("{}{INGEST_PATH}", server_url.trim_end_matches('/'))
}

/// Chemin de la file de commandes, ou du compte rendu d'une commande donnée.
///
/// La clé d'identité accompagne chaque requête (en-tête [`AGENT_KEY_HEADER`]) :
/// le jeton est partagé par tout un parc et ne dit pas de quelle machine il
/// s'agit.
fn commands_path(id: Option<i64>) -> String {
    match id {
        Some(id) => format!("{COMMANDS_PATH}/{id}"),
        None => COMMANDS_PATH.to_string(),
    }
}

/// Décrit une erreur de transport : la nature (délai, connexion…), puis la
/// chaîne des causes en commençant par la plus profonde — « Connection refused »
/// avant « error sending request » —, puis l'URL **sans ses paramètres**.
///
/// Le message brut de `reqwest` commence par l'URL complète : tronqué par
/// `systemctl status`, il ne montrait jamais la cause, et recopiait la clé de
/// la machine quand elle voyageait encore dans l'URL.
fn describe(error: &reqwest::Error) -> String {
    use std::error::Error as _;
    let nature = if error.is_timeout() {
        "timed out"
    } else if error.is_connect() {
        "cannot connect"
    } else if error.is_decode() {
        "unreadable response"
    } else if error.is_body() {
        "response interrupted"
    } else {
        "request failed"
    };
    let mut causes: Vec<String> = Vec::new();
    let mut source = error.source();
    while let Some(cause) = source {
        let text = cause.to_string();
        if !text.is_empty() && !causes.contains(&text) {
            causes.push(text);
        }
        source = cause.source();
    }
    causes.reverse();
    let mut message = nature.to_string();
    for cause in causes.iter().take(3) {
        message.push_str(": ");
        message.push_str(cause);
    }
    if let Some(url) = error.url() {
        let mut url = url.clone();
        url.set_query(None);
        url.set_fragment(None);
        let _ = url.set_password(None);
        let _ = url.set_username("");
        message.push_str(&format!(" ({url})"));
    }
    message
}

/// Encodage d'un paramètre de requête. Sans le `query` de reqwest — une
/// dépendance de plus pour un seul paramètre — les caractères non réservés
/// passent tels quels, tout le reste est échappé.
fn percent_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// Message du corps d'erreur `{"error": "..."}` renvoyé par l'API, si c'en est un.
///
/// Sa présence est ce qui distingue un refus du serveur DumbMonit d'une page
/// d'erreur produite par un intermédiaire.
fn server_error_message(body: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| value.get("error")?.as_str().map(str::to_string))
}

/// Message d'erreur à journaliser : celui du serveur, ou le début du corps brut.
fn extract_error_message(body: &str) -> String {
    server_error_message(body).unwrap_or_else(|| body.trim().chars().take(200).collect())
}

/// Retire le jeton d'un message avant qu'il n'atteigne les journaux.
///
/// Les messages de `reqwest` contiennent l'URL, et une erreur de configuration
/// pourrait un jour y glisser le jeton. Le filet est peu coûteux, et l'oubli
/// inverse ne se rattrape pas une fois les journaux partis en centralisation.
fn sanitise(message: &str, token: &str) -> String {
    if token.is_empty() {
        return message.to_string();
    }
    message.replace(token, "<redacted>")
}

#[cfg(test)]
#[path = "client_http_tests.rs"]
mod http_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ingest_url_is_built_without_a_double_slash() {
        assert_eq!(ingest_url("http://serveur:8080"), "http://serveur:8080/api/ingest");
        assert_eq!(ingest_url("http://serveur:8080/"), "http://serveur:8080/api/ingest");
        assert_eq!(
            ingest_url("https://mon.domaine/dumbmonit/"),
            "https://mon.domaine/dumbmonit/api/ingest"
        );
    }

    #[test]
    fn the_agent_channel_paths_follow_the_shared_contract() {
        assert_eq!(commands_path(None), "/api/agent/commands");
        assert_eq!(commands_path(Some(42)), "/api/agent/commands/42");
        assert_eq!(relay_path(None), "/api/agent/relay");
        assert_eq!(relay_path(Some(3)), "/api/agent/relay/3");
    }

    #[test]
    fn only_the_old_server_refusal_switches_the_key_back_to_the_url() {
        assert!(wants_legacy_key(r#"{"error":"the 'key' query parameter is required"}"#));
        assert!(!wants_legacy_key(r#"{"error":"the machine key is required (header x)"}"#));
        assert!(!wants_legacy_key("the 'key' query parameter is required"), "hors enveloppe");
    }

    #[test]
    fn a_rejected_batch_is_not_retried_but_an_outage_is() {
        assert!(PushError::Transport("connexion refusée".into()).is_retryable());
        assert!(PushError::Unauthorized.is_retryable());
        // Quelqu'un peut ouvrir la fenêtre de réenrôlement pendant qu'on patiente.
        assert!(PushError::Forbidden("rebind me".into()).is_retryable());
        assert!(!PushError::Rejected { status: 400, message: "lot vide".into() }.is_retryable());
    }

    #[test]
    fn the_binding_secret_travels_in_its_own_header_only_once_it_exists() {
        let client = PushClient::new("http://serveur:8080", "dmon_x", None, Duration::from_secs(1))
            .expect("client");
        assert!(client.secret.read().unwrap().is_none(), "pas encore lié : aucun en-tête");

        client.set_secret(Some("dmab_abc".into()));
        assert_eq!(client.secret.read().unwrap().as_deref(), Some("dmab_abc"));
        // Le secret est partagé : la boucle des commandes tient une copie du
        // client et doit le voir apparaître sans être reconstruite.
        let twin = client.clone();
        assert_eq!(twin.secret.read().unwrap().as_deref(), Some("dmab_abc"));
        twin.set_secret(None);
        assert!(client.secret.read().unwrap().is_none());
    }

    #[test]
    fn a_refusal_of_this_machine_is_told_apart_from_a_refused_token() {
        // 401 : le jeton. 403 : cette machine-ci. Les deux ne se réparent pas de
        // la même façon, et le message du serveur est ce qui le dit.
        assert!(matches!(PushError::Unauthorized, PushError::Unauthorized));
        let forbidden = PushError::Forbidden("Allow re-enrolment in the interface.".into());
        assert_eq!(forbidden.to_string(), "Allow re-enrolment in the interface.");
    }

    #[test]
    fn the_api_error_message_is_extracted_from_its_envelope() {
        assert_eq!(extract_error_message(r#"{"error":"jeton révoqué"}"#), "jeton révoqué");
    }

    #[test]
    fn a_non_json_error_body_is_kept_as_is_but_truncated() {
        let long = "x".repeat(500);
        assert_eq!(extract_error_message(&long).len(), 200);
        assert_eq!(extract_error_message("  502 Bad Gateway  "), "502 Bad Gateway");
    }

    #[test]
    fn the_token_is_scrubbed_from_transport_errors() {
        let message = "error sending request for url (http://s/api/ingest?t=dmon_secret)";
        let cleaned = sanitise(message, "dmon_secret");
        assert!(!cleaned.contains("dmon_secret"), "le jeton a fuité : {cleaned}");
    }

    #[test]
    fn error_messages_are_written_in_english() {
        assert_eq!(PushError::Unauthorized.to_string(), "enrollment token rejected");
        assert!(
            PushError::Transport("connexion refusée".into())
                .to_string()
                .starts_with("server unreachable")
        );
    }
}
