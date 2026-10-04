//! La musique du mode mur.
//!
//! Deux façons de faire jouer quelque chose sur l'écran de la pièce :
//!
//! - **Spotify Connect.** Un administrateur relie un compte Spotify (flux
//!   Authorization Code + PKCE, avec le client ID d'une application Spotify
//!   Developer qu'il a créée — pas de secret). Le serveur garde le jeton de
//!   rafraîchissement chiffré et s'en sert pour deux choses : lire la lecture en
//!   cours (`GET /v1/me/player`, mise en cache quelques secondes et partagée par
//!   tous les murs), quel que soit l'appareil qui joue, et remettre au mur un
//!   jeton d'accès court pour le Web Playback SDK, qui fait du navigateur un
//!   appareil « DumbMonit Wall » choisi depuis l'application du téléphone.
//! - **Un lien partagé** (Spotify, Deezer, YouTube) que les murs jouent dans le
//!   lecteur intégré du service : pour qui n'a pas Premium, ou pour Deezer et
//!   YouTube qui n'ont pas d'équivalent « Connect » sur le web.
//!
//! L'état transitoire — autorisations en cours, jeton d'accès, dernière lecture
//! lue, appareils du compte, rapports des murs — vit en mémoire dans
//! [`MusicHub`] : un processus unique suffit, et rien de tout cela ne mérite de
//! survivre à un redémarrage.
//!
//! **Rapports des murs.** Un mur qui échoue à devenir enceinte (page en HTTP,
//! navigateur sans DRM, compte sans Premium…) le dit sur son écran, mais
//! personne ne regarde la télé de près : il le rapporte aussi au serveur
//! (`POST /api/music/speaker/report`), et les réglages montrent, mur par mur,
//! où il en est et si Spotify le liste bien parmi les appareils du compte.

pub mod link;
pub mod spotify;
pub mod store;

use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::sync::Mutex;

use crate::auth::oidc::pkce;
use crate::crypto::Cipher;
use spotify::{Authorized, Device, Endpoints, NowPlaying, Player, SpotifyError};

/// Délai laissé pour revenir de Spotify avec un code (ou le recoller).
const PENDING_TTL: Duration = Duration::from_secs(10 * 60);
/// Autorisations en cours gardées au plus : seul un administrateur en ouvre.
const PENDING_MAX: usize = 32;
/// Durée pendant laquelle la lecture en cours est resservie sans redemander à
/// Spotify : les murs interrogent toutes les cinq secondes, chacun de leur côté.
const NOW_TTL: Duration = Duration::from_secs(4);
/// Un jeton d'accès qui expire dans moins que cela est renouvelé d'avance : le
/// SDK le garde jusqu'à ce qu'il échoue.
const TOKEN_MARGIN: Duration = Duration::from_secs(120);
/// Durée pendant laquelle la liste des appareils du compte est resservie.
const DEVICES_TTL: Duration = Duration::from_secs(10);
/// Un mur qui ne s'est pas rapporté depuis ce délai est oublié.
const REPORT_TTL: Duration = Duration::from_secs(15 * 60);
/// Murs suivis au plus (une maison n'en a que quelques-uns).
const REPORTS_MAX: usize = 16;

/// Pourquoi une opération n'a pas abouti.
#[derive(Debug)]
pub enum MusicError {
    Spotify(SpotifyError),
    Internal(anyhow::Error),
}

impl From<SpotifyError> for MusicError {
    fn from(error: SpotifyError) -> Self {
        Self::Spotify(error)
    }
}

impl From<anyhow::Error> for MusicError {
    fn from(error: anyhow::Error) -> Self {
        Self::Internal(error)
    }
}

/// Pourquoi une connexion a échoué, avec un code court pour l'URL de retour.
#[derive(Debug)]
pub enum ConnectError {
    /// `state` inconnu, expiré, déjà utilisé, ou ouvert par quelqu'un d'autre.
    State,
    /// Spotify (ou l'utilisateur) a refusé l'autorisation.
    Denied(String),
    /// Le code n'a pas pu être échangé.
    Spotify(SpotifyError),
    Internal(anyhow::Error),
}

impl ConnectError {
    pub fn reason(&self) -> &'static str {
        match self {
            Self::State => "state",
            Self::Denied(_) => "denied",
            Self::Spotify(_) => "exchange",
            Self::Internal(_) => "internal",
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::State => "This approval is unknown or expired (it is valid ten minutes, once). \
                            Start again from “Connect Spotify”."
                .into(),
            Self::Denied(why) => format!("Spotify did not grant access ({why})."),
            Self::Spotify(error) => error.message(),
            Self::Internal(_) => "The connection could not be saved. Check the server logs.".into(),
        }
    }
}

/// Paramètres du retour de Spotify, tels qu'ils arrivent dans l'URL.
#[derive(Debug, Default, Deserialize)]
pub struct CallbackParams {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

/// Relit les paramètres dans ce que l'utilisateur a collé : l'adresse complète
/// de la page d'arrivée (`http://127.0.0.1:8888/callback?code=…&state=…`), ou
/// seulement sa partie après `?`.
pub fn parse_pasted(text: &str) -> Option<CallbackParams> {
    let text = text.trim();
    if text.is_empty() || text.len() > 4096 {
        return None;
    }
    let url = reqwest::Url::parse(text).ok().or_else(|| {
        let query = text.split_once('?').map_or(text, |(_, query)| query);
        reqwest::Url::parse(&format!("http://pasted.invalid/?{query}")).ok()
    })?;
    let mut params = CallbackParams::default();
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "code" => params.code = Some(value.into_owned()),
            "state" => params.state = Some(value.into_owned()),
            "error" => params.error = Some(value.into_owned()),
            _ => {}
        }
    }
    (params.state.is_some() && (params.code.is_some() || params.error.is_some())).then_some(params)
}

/// Un client ID Spotify : 32 caractères hexadécimaux. On reste un peu plus
/// large que la forme actuelle, sans jamais accepter de quoi casser une URL.
pub fn check_client_id(client_id: &str) -> Result<(), &'static str> {
    let ok = (16..=64).contains(&client_id.len())
        && client_id.bytes().all(|b| b.is_ascii_alphanumeric());
    ok.then_some(()).ok_or(
        "Paste the Client ID shown on your app's page in the Spotify Developer Dashboard \
         (32 letters and digits).",
    )
}

/// L'URI de redirection proposée par l'interface : l'adresse de bouclage, ou
/// le retour direct en HTTPS sur l'origine même de la page (`Origin` de la
/// requête, quand le navigateur l'envoie). Spotify refuse toute autre forme
/// d'`http://`, et exige que l'URI soit déclarée dans l'application.
pub fn check_redirect_uri(uri: &str, origin: Option<&str>) -> Result<(), &'static str> {
    const REFUSED: &str = "Spotify only accepts an https:// address for the redirect, or \
                           http://127.0.0.1:8888/callback when DumbMonit is served over plain HTTP.";
    if uri == spotify::LOOPBACK_REDIRECT_URI {
        return Ok(());
    }
    let url = reqwest::Url::parse(uri).map_err(|_| REFUSED)?;
    let clean = url.scheme() == "https"
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
        && url.path() == spotify::CALLBACK_PATH;
    if !clean {
        return Err(REFUSED);
    }
    match origin {
        Some(origin) if !origin.eq_ignore_ascii_case(&url.origin().ascii_serialization()) => {
            Err("The redirect address must be on the address this page was opened from.")
        }
        _ => Ok(()),
    }
}

/// État du compte relié.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Connection {
    /// Aucun compte relié.
    Off,
    Connected,
    /// Relié, mais Spotify a révoqué ou laissé expirer le jeton : à reconnecter.
    Expired,
}

/// Ce que le mur lit de Spotify.
#[derive(Debug, Clone, Serialize)]
pub struct SpotifyNow {
    pub status: Connection,
    /// `None` : rien ne joue (ou rien n'a pu être lu).
    pub now_playing: Option<NowPlaying>,
    /// Problème passager ou remède attendu, en une phrase.
    pub error: Option<String>,
}

impl SpotifyNow {
    fn quiet(status: Connection) -> Self {
        Self { status, now_playing: None, error: None }
    }

    fn failed(status: Connection, error: String) -> Self {
        Self { status, now_playing: None, error: Some(error) }
    }
}

/// Le compte relié, tel que les réglages le montrent : jamais un jeton.
#[derive(Debug, Clone, Serialize)]
pub struct SpotifyView {
    pub status: Connection,
    pub client_id: Option<String>,
    pub account_name: Option<String>,
    pub connected_at: Option<String>,
    /// Date (UTC) à laquelle Spotify cessera d'honorer le jeton de
    /// rafraîchissement : se reconnecter avant.
    pub reconnect_by: Option<String>,
    /// Portées demandées que l'autorisation n'a pas accordées.
    pub missing_scopes: Vec<String>,
    /// Pourquoi la connexion a pris fin, quand elle a pris fin.
    pub last_error: Option<String>,
    pub speaker_name: String,
    pub scopes: &'static [&'static str],
    pub loopback_redirect_uri: &'static str,
    pub callback_path: &'static str,
}

impl SpotifyView {
    pub async fn load(pool: &SqlitePool) -> anyhow::Result<Self> {
        let account = store::account(pool).await?;
        let status = match &account {
            None => Connection::Off,
            Some(account) if account.refresh_token.is_none() => Connection::Expired,
            Some(_) => Connection::Connected,
        };
        let connected = status == Connection::Connected;
        Ok(Self {
            status,
            client_id: account.as_ref().map(|a| a.client_id.clone()),
            account_name: account.as_ref().and_then(|a| a.account_name.clone()),
            connected_at: account.as_ref().filter(|_| connected).map(|a| a.authorized_at.clone()),
            reconnect_by: account
                .as_ref()
                .filter(|_| connected)
                .and_then(|a| reconnect_by(&a.authorized_at)),
            missing_scopes: account
                .as_ref()
                .filter(|_| connected)
                .map(|a| missing_scopes(&a.scopes))
                .unwrap_or_default(),
            last_error: account.as_ref().and_then(|a| a.last_error.clone()),
            speaker_name: store::speaker_name(pool).await?,
            scopes: spotify::SCOPES,
            loopback_redirect_uri: spotify::LOOPBACK_REDIRECT_URI,
            callback_path: spotify::CALLBACK_PATH,
        })
    }
}

/// Six mois après l'autorisation, au format des dates du serveur.
fn reconnect_by(authorized_at: &str) -> Option<String> {
    let at = chrono::NaiveDateTime::parse_from_str(authorized_at, "%Y-%m-%d %H:%M:%S").ok()?;
    let by = at + chrono::TimeDelta::days(spotify::REFRESH_TOKEN_DAYS);
    Some(by.format("%Y-%m-%d %H:%M:%S").to_string())
}

fn missing_scopes(granted: &str) -> Vec<String> {
    let granted: Vec<&str> = granted.split_whitespace().collect();
    spotify::SCOPES
        .iter()
        .filter(|scope| !granted.contains(scope))
        .map(|scope| scope.to_string())
        .collect()
}

/// Un jeton d'accès, et quand il cesse de valoir.
#[derive(Debug, Clone)]
pub struct AccessToken {
    pub token: String,
    pub expires_at: Instant,
}

impl AccessToken {
    fn from_tokens(tokens: &spotify::Tokens) -> Self {
        Self {
            token: tokens.access_token.clone(),
            expires_at: Instant::now() + Duration::from_secs(tokens.expires_in.clamp(60, 86_400)),
        }
    }

    pub fn expires_in(&self) -> Duration {
        self.expires_at.saturating_duration_since(Instant::now())
    }
}

struct Pending {
    verifier: String,
    client_id: String,
    redirect_uri: String,
    user_id: i64,
    started: Instant,
}

/// Ce qu'un mur dit de son enceinte (`POST /api/music/speaker/report`).
#[derive(Debug, Clone, Deserialize)]
pub struct SpeakerReport {
    /// Identifiant aléatoire du mur, gardé par son navigateur.
    pub display: String,
    /// Le nom sous lequel il s'annonce.
    pub name: String,
    /// `off`, `unsupported`, `starting`, `ready` ou `error`.
    pub phase: String,
    /// Le son est débloqué (un geste a eu lieu sur la page).
    #[serde(default)]
    pub activated: bool,
    /// L'identifiant d'appareil que le SDK a reçu.
    #[serde(default)]
    pub device_id: Option<String>,
    /// Ce qui manque ou a échoué, en une phrase.
    #[serde(default)]
    pub problem: Option<String>,
    /// « Chrome 130 on Linux » : de quoi reconnaître l'écran.
    #[serde(default)]
    pub browser: Option<String>,
    /// `false` : le SDK a dit que le compte n'a pas Premium ; `true` : prêt.
    #[serde(default)]
    pub premium: Option<bool>,
}

/// Un mur tel que les réglages le montrent.
#[derive(Debug, Clone, Serialize)]
pub struct WallReport {
    pub display: String,
    pub name: String,
    pub phase: String,
    pub activated: bool,
    pub device_id: Option<String>,
    pub problem: Option<String>,
    pub browser: Option<String>,
    pub premium: Option<bool>,
    /// Spotify liste bien cet appareil (`None` : pas vérifiable).
    pub listed: Option<bool>,
    /// Dernier rapport, UTC sans suffixe.
    pub seen_at: String,
    #[serde(skip)]
    seen: Option<Instant>,
}

/// Coupe une chaîne venue d'un navigateur à une longueur raisonnable, sans
/// caractère de contrôle.
fn tidy(text: &str, max: usize) -> String {
    text.chars().filter(|c| !c.is_control()).take(max).collect::<String>().trim().to_string()
}

impl SpeakerReport {
    /// Vérifie et nettoie : rien de ce qu'envoie un mur n'est cru tel quel.
    pub fn check(self) -> Result<Self, &'static str> {
        let display_ok = (8..=64).contains(&self.display.len())
            && self.display.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-');
        if !display_ok {
            return Err("Unknown display.");
        }
        if !["off", "unsupported", "starting", "ready", "error"].contains(&self.phase.as_str()) {
            return Err("Unknown speaker phase.");
        }
        let device_id = self.device_id.filter(|id| {
            (1..=128).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric())
        });
        Ok(Self {
            name: tidy(&self.name, spotify::SPEAKER_NAME_MAX),
            problem: self.problem.map(|p| tidy(&p, 400)).filter(|p| !p.is_empty()),
            browser: self.browser.map(|b| tidy(&b, 80)).filter(|b| !b.is_empty()),
            device_id,
            ..self
        })
    }
}

/// L'enceinte, telle que les réglages la montrent (`GET /api/music/speaker`).
#[derive(Debug, Clone, Serialize)]
pub struct SpeakerStatus {
    pub speaker_name: String,
    pub status: Connection,
    pub account_name: Option<String>,
    /// `None` : inconnu (Spotify ne le dit plus aux applications personnelles,
    /// et aucun mur ne l'a encore appris).
    pub premium: Option<bool>,
    /// Les appareils Spotify Connect du compte, à l'instant.
    pub devices: Vec<Device>,
    /// Un appareil à ce nom (ou d'un mur prêt) est dans la liste.
    pub listed: bool,
    pub walls: Vec<WallReport>,
    /// Pourquoi la liste des appareils n'a pas pu être lue.
    pub error: Option<String>,
}

#[derive(Default)]
struct DevicesCache {
    value: Option<Vec<Device>>,
    at: Option<Instant>,
}

#[derive(Default)]
struct NowCache {
    value: Option<SpotifyNow>,
    at: Option<Instant>,
    /// Spotify a demandé de ralentir : rien avant cet instant.
    hold_until: Option<Instant>,
}

struct Inner {
    endpoints: Endpoints,
    http: reqwest::Client,
    pending: StdMutex<HashMap<String, Pending>>,
    /// Le verrou sérialise aussi les rafraîchissements : deux murs qui
    /// demandent un jeton au même moment n'en déclenchent qu'un.
    token: Mutex<Option<AccessToken>>,
    /// Même idée pour la lecture en cours : une requête à Spotify à la fois.
    now: Mutex<NowCache>,
    devices: Mutex<DevicesCache>,
    /// Derniers rapports des murs, par identifiant de mur.
    walls: StdMutex<HashMap<String, WallReport>>,
}

/// L'état en mémoire de la musique du mur, partagé par les gestionnaires HTTP.
#[derive(Clone)]
pub struct MusicHub(Arc<Inner>);

impl MusicHub {
    pub fn new(endpoints: Endpoints) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent(concat!("DumbMonit/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("client HTTP Spotify");
        Self(Arc::new(Inner {
            endpoints,
            http,
            pending: StdMutex::new(HashMap::new()),
            token: Mutex::new(None),
            now: Mutex::new(NowCache::default()),
            devices: Mutex::new(DevicesCache::default()),
            walls: StdMutex::new(HashMap::new()),
        }))
    }

    /// Les adresses officielles, sauf surcharge par l'environnement.
    pub fn from_env() -> Self {
        Self::new(Endpoints::from_env())
    }

    /// Commence une autorisation pour ce compte DumbMonit ; rend l'adresse où
    /// envoyer le navigateur.
    pub fn start(&self, client_id: &str, redirect_uri: &str, user_id: i64) -> String {
        let state = pkce::random_token();
        let challenge = pkce::Pkce::generate();
        let url = spotify::authorize_url(
            &self.0.endpoints,
            client_id,
            redirect_uri,
            &state,
            &challenge.challenge,
        );
        let mut pending = self.0.pending.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        pending.retain(|_, p| p.started.elapsed() < PENDING_TTL);
        if pending.len() >= PENDING_MAX
            && let Some(oldest) =
                pending.iter().min_by_key(|(_, p)| p.started).map(|(state, _)| state.clone())
        {
            pending.remove(&oldest);
        }
        pending.insert(
            state,
            Pending {
                verifier: challenge.verifier,
                client_id: client_id.to_string(),
                redirect_uri: redirect_uri.to_string(),
                user_id,
                started: Instant::now(),
            },
        );
        url
    }

    /// Termine une autorisation : vérifie `state` (une seule fois, par le compte
    /// qui l'a ouverte), échange le code, lit le profil et enregistre le tout.
    pub async fn finish(
        &self,
        pool: &SqlitePool,
        cipher: &Cipher,
        params: CallbackParams,
        user_id: i64,
    ) -> Result<(), ConnectError> {
        let pending = {
            let mut map = self.0.pending.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            params.state.as_deref().and_then(|state| map.remove(state))
        }
        .filter(|p| p.started.elapsed() < PENDING_TTL && p.user_id == user_id)
        .ok_or(ConnectError::State)?;

        if let Some(error) = params.error.filter(|e| !e.is_empty()) {
            return Err(ConnectError::Denied(error));
        }
        let code = params.code.filter(|code| !code.is_empty()).ok_or(ConnectError::State)?;

        let (http, endpoints) = (&self.0.http, &self.0.endpoints);
        let tokens = spotify::exchange_code(
            http,
            endpoints,
            &pending.client_id,
            &code,
            &pending.redirect_uri,
            &pending.verifier,
        )
        .await
        .map_err(ConnectError::Spotify)?;
        let refresh = tokens.refresh_token.clone().filter(|t| !t.is_empty()).ok_or_else(|| {
            ConnectError::Spotify(SpotifyError::Rejected("no refresh token was returned".into()))
        })?;
        let profile = spotify::me(http, endpoints, &tokens.access_token)
            .await
            .map_err(ConnectError::Spotify)?;
        let scopes = tokens.scope.clone().unwrap_or_else(|| spotify::SCOPES.join(" "));
        store::set_product(pool, profile.product.as_deref().filter(|p| !p.is_empty()))
            .await
            .map_err(ConnectError::Internal)?;
        store::save_connection(
            pool,
            cipher,
            &pending.client_id,
            &refresh,
            &scopes,
            &profile.id,
            profile.display_name.as_deref().filter(|name| !name.trim().is_empty()),
        )
        .await
        .map_err(ConnectError::Internal)?;

        *self.0.token.lock().await = Some(AccessToken::from_tokens(&tokens));
        *self.0.now.lock().await = NowCache::default();
        *self.0.devices.lock().await = DevicesCache::default();
        self.walls().clear();
        Ok(())
    }

    /// Oublie tout ce qui est en mémoire (après une déconnexion).
    pub async fn forget(&self) {
        *self.0.token.lock().await = None;
        *self.0.now.lock().await = NowCache::default();
        *self.0.devices.lock().await = DevicesCache::default();
        self.walls().clear();
    }

    fn walls(&self) -> std::sync::MutexGuard<'_, HashMap<String, WallReport>> {
        self.0.walls.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Les appareils Spotify Connect du compte, depuis le cache s'il est frais.
    pub async fn devices(
        &self,
        pool: &SqlitePool,
        cipher: &Cipher,
    ) -> Result<Vec<Device>, MusicError> {
        let mut cache = self.0.devices.lock().await;
        if let (Some(value), Some(at)) = (&cache.value, cache.at)
            && at.elapsed() < DEVICES_TTL
        {
            return Ok(value.clone());
        }
        for attempt in 0..2 {
            let token = self.access_token(pool, cipher).await?;
            match spotify::devices(&self.0.http, &self.0.endpoints, &token.token).await? {
                Authorized::Ok(devices) => {
                    cache.value = Some(devices.clone());
                    cache.at = Some(Instant::now());
                    return Ok(devices);
                }
                Authorized::Unauthorized if attempt == 0 => self.drop_token().await,
                Authorized::Unauthorized => break,
            }
        }
        Err(SpotifyError::Rejected("the access token was refused".into()).into())
    }

    /// Fait jouer le compte sur cet appareil.
    pub async fn transfer(
        &self,
        pool: &SqlitePool,
        cipher: &Cipher,
        device_id: &str,
    ) -> Result<(), MusicError> {
        for attempt in 0..2 {
            let token = self.access_token(pool, cipher).await?;
            match spotify::transfer(&self.0.http, &self.0.endpoints, &token.token, device_id)
                .await?
            {
                Authorized::Ok(()) => {
                    // Ce qui joue, et où, vient de changer.
                    *self.0.now.lock().await = NowCache::default();
                    *self.0.devices.lock().await = DevicesCache::default();
                    return Ok(());
                }
                Authorized::Unauthorized if attempt == 0 => self.drop_token().await,
                Authorized::Unauthorized => break,
            }
        }
        Err(SpotifyError::Rejected("the access token was refused".into()).into())
    }

    /// Enregistre le rapport d'un mur et lui dit si Spotify le liste.
    pub async fn report(
        &self,
        pool: &SqlitePool,
        cipher: &Cipher,
        report: SpeakerReport,
    ) -> Option<bool> {
        let listed = match (&report.device_id, report.phase.as_str()) {
            (Some(id), "ready") => match self.devices(pool, cipher).await {
                Ok(devices) => Some(devices.iter().any(|d| d.id.as_deref() == Some(id))),
                Err(_) => None,
            },
            _ => None,
        };
        let mut walls = self.walls();
        walls.retain(|_, wall| wall.seen.is_some_and(|seen| seen.elapsed() < REPORT_TTL));
        if !walls.contains_key(&report.display)
            && walls.len() >= REPORTS_MAX
            && let Some(oldest) =
                walls.iter().min_by_key(|(_, w)| w.seen).map(|(display, _)| display.clone())
        {
            walls.remove(&oldest);
        }
        walls.insert(
            report.display.clone(),
            WallReport {
                display: report.display,
                name: report.name,
                phase: report.phase,
                activated: report.activated,
                device_id: report.device_id,
                problem: report.problem,
                browser: report.browser,
                premium: report.premium,
                listed,
                seen_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                seen: Some(Instant::now()),
            },
        );
        listed
    }

    /// Les murs qui se sont rapportés récemment, le plus récent d'abord.
    pub fn wall_reports(&self) -> Vec<WallReport> {
        let mut walls: Vec<WallReport> = self
            .walls()
            .values()
            .filter(|wall| wall.seen.is_some_and(|seen| seen.elapsed() < REPORT_TTL))
            .cloned()
            .collect();
        walls.sort_by_key(|wall| std::cmp::Reverse(wall.seen));
        walls
    }

    /// L'appareil d'un mur prêt sous ce nom, ou à défaut un appareil du compte
    /// qui porte ce nom.
    pub async fn speaker_device(
        &self,
        pool: &SqlitePool,
        cipher: &Cipher,
        name: &str,
    ) -> Result<Option<String>, MusicError> {
        let devices = self.devices(pool, cipher).await?;
        let listed = |id: &str| devices.iter().any(|d| d.id.as_deref() == Some(id));
        let from_wall = self
            .wall_reports()
            .into_iter()
            .filter(|w| w.phase == "ready" && w.name == name)
            .filter_map(|w| w.device_id)
            .find(|id| listed(id));
        Ok(from_wall.or_else(|| {
            devices.iter().filter(|d| d.name == name && !d.is_restricted).find_map(|d| d.id.clone())
        }))
    }

    /// L'état complet de l'enceinte, pour les réglages.
    pub async fn speaker_status(
        &self,
        pool: &SqlitePool,
        cipher: &Cipher,
    ) -> anyhow::Result<SpeakerStatus> {
        let view = SpotifyView::load(pool).await?;
        let walls = self.wall_reports();
        let (devices, error) = match view.status {
            Connection::Connected => match self.devices(pool, cipher).await {
                Ok(devices) => (devices, None),
                Err(MusicError::Spotify(error)) => (Vec::new(), Some(error.message())),
                Err(MusicError::Internal(error)) => return Err(error),
            },
            _ => (Vec::new(), None),
        };
        let ready_ids: Vec<&str> = walls
            .iter()
            .filter(|w| w.phase == "ready")
            .filter_map(|w| w.device_id.as_deref())
            .collect();
        let listed = devices.iter().any(|d| {
            d.name == view.speaker_name || d.id.as_deref().is_some_and(|id| ready_ids.contains(&id))
        });
        let premium = match store::product(pool).await? {
            Some(product) => Some(product == "premium"),
            None => walls.iter().find_map(|w| w.premium),
        };
        Ok(SpeakerStatus {
            speaker_name: view.speaker_name,
            status: view.status,
            account_name: view.account_name,
            premium,
            devices,
            listed,
            walls,
            error,
        })
    }

    /// Un jeton d'accès valable encore au moins deux minutes, rafraîchi au besoin.
    pub async fn access_token(
        &self,
        pool: &SqlitePool,
        cipher: &Cipher,
    ) -> Result<AccessToken, MusicError> {
        let mut cached = self.0.token.lock().await;
        if let Some(token) = cached.as_ref()
            && token.expires_in() > TOKEN_MARGIN
        {
            return Ok(token.clone());
        }
        let account = store::account(pool).await?.ok_or(SpotifyError::NotConnected)?;
        let Some(refresh) = account.refresh_token(cipher)? else {
            let why = account.last_error.unwrap_or_else(|| "the authorization ended".into());
            return Err(SpotifyError::Expired(why).into());
        };
        match spotify::refresh(&self.0.http, &self.0.endpoints, &account.client_id, &refresh).await
        {
            Ok(tokens) => {
                if let Some(rotated) =
                    tokens.refresh_token.as_deref().filter(|t| !t.is_empty() && *t != refresh)
                {
                    store::replace_refresh_token(pool, cipher, rotated).await?;
                }
                let token = AccessToken::from_tokens(&tokens);
                *cached = Some(token.clone());
                Ok(token)
            }
            Err(SpotifyError::Expired(why)) => {
                tracing::warn!(reason = %why, "Spotify refused the refresh token");
                store::mark_expired(pool, &why).await?;
                *cached = None;
                Err(SpotifyError::Expired(why).into())
            }
            Err(other) => Err(other.into()),
        }
    }

    async fn drop_token(&self) {
        *self.0.token.lock().await = None;
    }

    /// La lecture en cours, servie depuis le cache tant qu'il est frais.
    pub async fn now(&self, pool: &SqlitePool, cipher: &Cipher) -> SpotifyNow {
        let mut cache = self.0.now.lock().await;
        let now = Instant::now();
        if let Some(value) = &cache.value {
            let fresh = cache.at.is_some_and(|at| now.duration_since(at) < NOW_TTL);
            let held = cache.hold_until.is_some_and(|until| now < until);
            if fresh || held {
                return value.clone();
            }
        }
        let (value, hold) = self.fetch_now(pool, cipher).await;
        let value = match (hold, cache.value.take()) {
            // Ralentir n'efface pas ce qu'on savait : le mur garde la carte.
            (Some(_), Some(previous)) => SpotifyNow { error: value.error, ..previous },
            (_, _) => value,
        };
        cache.hold_until = hold.map(|wait| now + wait);
        cache.at = Some(now);
        cache.value = Some(value.clone());
        value
    }

    async fn fetch_now(
        &self,
        pool: &SqlitePool,
        cipher: &Cipher,
    ) -> (SpotifyNow, Option<Duration>) {
        // Rien à demander à Spotify sans compte relié ou avec un jeton révoqué.
        match store::account(pool).await.context("lecture du compte Spotify") {
            Err(error) => {
                tracing::error!(error = %format!("{error:#}"), "Spotify now playing");
                return (SpotifyNow::failed(Connection::Off, "Internal error.".into()), None);
            }
            Ok(None) => return (SpotifyNow::quiet(Connection::Off), None),
            Ok(Some(account)) if account.refresh_token.is_none() => {
                let why = account.last_error.unwrap_or_else(|| "the authorization ended".into());
                return (
                    SpotifyNow::failed(Connection::Expired, SpotifyError::Expired(why).message()),
                    None,
                );
            }
            Ok(Some(_)) => {}
        }

        for attempt in 0..2 {
            let token = match self.access_token(pool, cipher).await {
                Ok(token) => token,
                Err(MusicError::Spotify(SpotifyError::Expired(why))) => {
                    let message = SpotifyError::Expired(why).message();
                    return (SpotifyNow::failed(Connection::Expired, message), None);
                }
                Err(error) => return failure(error),
            };
            match spotify::player(&self.0.http, &self.0.endpoints, &token.token).await {
                Ok(Player::Idle) => return (SpotifyNow::quiet(Connection::Connected), None),
                Ok(Player::State(playing)) => {
                    let now = SpotifyNow {
                        status: Connection::Connected,
                        now_playing: Some(*playing),
                        error: None,
                    };
                    return (now, None);
                }
                // Jeton refusé avant son terme : un rafraîchissement, une seule fois.
                Ok(Player::Unauthorized) if attempt == 0 => self.drop_token().await,
                Ok(Player::Unauthorized) => break,
                Err(error) => return failure(error.into()),
            }
        }
        failure(SpotifyError::Rejected("the access token was refused".into()).into())
    }
}

/// Une lecture qui a échoué sans changer l'état du compte.
fn failure(error: MusicError) -> (SpotifyNow, Option<Duration>) {
    match error {
        MusicError::Spotify(error) => {
            let hold = match &error {
                SpotifyError::RateLimited(wait) => Some(*wait),
                _ => None,
            };
            (SpotifyNow::failed(Connection::Connected, error.message()), hold)
        }
        MusicError::Internal(error) => {
            tracing::error!(error = %format!("{error:#}"), "Spotify now playing");
            (SpotifyNow::failed(Connection::Connected, "Internal error.".into()), None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_landing_address_is_read_back_whole_or_in_part() {
        let full = parse_pasted("http://127.0.0.1:8888/callback?code=AQB-x_1&state=s1").unwrap();
        assert_eq!(full.code.as_deref(), Some("AQB-x_1"));
        assert_eq!(full.state.as_deref(), Some("s1"));
        let query = parse_pasted(" ?code=c2&state=s2 ").unwrap();
        assert_eq!((query.code.as_deref(), query.state.as_deref()), (Some("c2"), Some("s2")));
        let bare = parse_pasted("code=c3&state=s3").unwrap();
        assert_eq!(bare.code.as_deref(), Some("c3"));
        let denied = parse_pasted("127.0.0.1:8888/callback?error=access_denied&state=s4").unwrap();
        assert_eq!(denied.error.as_deref(), Some("access_denied"));
        assert!(parse_pasted("").is_none());
        assert!(parse_pasted("http://127.0.0.1:8888/callback").is_none());
        assert!(parse_pasted("code=only").is_none(), "sans state, rien à vérifier");
    }

    #[test]
    fn redirect_uris_follow_spotify_rules() {
        let callback = "https://monit.example.org/api/music/spotify/callback";
        assert!(check_redirect_uri(spotify::LOOPBACK_REDIRECT_URI, None).is_ok());
        assert!(check_redirect_uri(callback, None).is_ok());
        assert!(check_redirect_uri(callback, Some("https://monit.example.org")).is_ok());
        assert!(check_redirect_uri(callback, Some("https://other.example.org")).is_err());
        for refused in [
            "http://192.168.1.10:8080/api/music/spotify/callback",
            "http://localhost:8888/callback",
            "https://monit.example.org/api/music/spotify/callback?x=1",
            "https://monit.example.org/elsewhere",
            "https://u:p@monit.example.org/api/music/spotify/callback",
            "javascript:alert(1)",
        ] {
            assert!(check_redirect_uri(refused, None).is_err(), "{refused}");
        }
    }

    #[test]
    fn wall_reports_are_checked_and_tidied() {
        let report = |display: &str, phase: &str| SpeakerReport {
            display: display.into(),
            name: " Living\u{7}room ".into(),
            phase: phase.into(),
            activated: true,
            device_id: Some("abc123".into()),
            problem: Some("x".repeat(1000)),
            browser: Some("Chrome 130 on Linux".into()),
            premium: Some(true),
        };
        let checked = report("display-0001", "ready").check().unwrap();
        assert_eq!(checked.name, "Livingroom");
        assert_eq!(checked.problem.as_deref().map(str::len), Some(400));
        assert!(report("short", "ready").check().is_err());
        assert!(report("display-0001", "dancing").check().is_err());
        let mut odd = report("display-0001", "ready");
        odd.device_id = Some("../../etc".into());
        assert_eq!(odd.check().unwrap().device_id, None);
    }

    #[test]
    fn client_ids_are_letters_and_digits() {
        assert!(check_client_id("0123456789abcdef0123456789abcdef").is_ok());
        assert!(check_client_id("short").is_err());
        assert!(check_client_id("0123456789abcdef0123456789abcde&").is_err());
    }

    #[test]
    fn the_refresh_token_lasts_six_months_from_the_authorization() {
        assert_eq!(reconnect_by("2026-10-01 12:00:00").as_deref(), Some("2027-04-01 12:00:00"));
        assert_eq!(reconnect_by("garbage"), None);
    }

    #[test]
    fn missing_scopes_are_named() {
        assert!(missing_scopes(&spotify::SCOPES.join(" ")).is_empty());
        assert_eq!(
            missing_scopes("streaming user-read-email user-read-private"),
            [
                "user-read-playback-state",
                "user-modify-playback-state",
                "user-read-currently-playing"
            ]
        );
    }
}
