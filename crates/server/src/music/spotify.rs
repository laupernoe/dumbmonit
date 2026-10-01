//! Le client Spotify : autorisation (Authorization Code + PKCE), jetons, profil
//! et lecture en cours.
//!
//! Rien ici ne touche à la base ni à l'état partagé : des fonctions qui parlent
//! HTTP et rendent des valeurs, pour que la logique se teste contre un faux
//! serveur. Le cache, la sérialisation des rafraîchissements et la persistance
//! vivent dans [`super::MusicHub`].
//!
//! Règles Spotify vérifiées en octobre 2026 (developer.spotify.com) :
//!
//! - URI de redirection en `https`, sauf une adresse de bouclage littérale en
//!   `http` (`http://127.0.0.1:PORT`) ; `localhost` est refusé depuis 2025.
//! - PKCE : ni secret client à l'échange, ni au rafraîchissement — seul le
//!   `client_id` est envoyé.
//! - Un jeton de rafraîchissement vit six mois à partir de l'autorisation, et
//!   le rafraîchir ne remet pas le compteur à zéro. Il n'est pas toujours
//!   renvoyé : on garde l'ancien quand la réponse n'en porte pas.
//! - Mode développement (applications personnelles) : propriétaire Premium,
//!   cinq utilisateurs au plus ; `GET /me` ne rend plus `product` ni `email`,
//!   on ne peut donc pas vérifier Premium côté serveur — le SDK le dit au mur.

use std::time::Duration;

use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

/// Nom sous lequel le mur s'annonce comme appareil Spotify Connect.
pub const SPEAKER_NAME: &str = "DumbMonit Wall";

/// Portées demandées : lecture par le SDK (`streaming`, `user-read-email`,
/// `user-read-private`), lecture en cours et appareils (`…-playback-state`,
/// `…-currently-playing`), et transfert de lecture vers le mur
/// (`user-modify-playback-state`).
pub const SCOPES: &[&str] = &[
    "streaming",
    "user-read-email",
    "user-read-private",
    "user-read-playback-state",
    "user-modify-playback-state",
    "user-read-currently-playing",
];

/// Redirection utilisée quand DumbMonit est servi en HTTP simple : Spotify
/// n'accepte pas `http://<ip-du-lan>`, mais accepte la boucle locale. Rien
/// n'écoute à cette adresse — le navigateur y affiche une erreur, et l'adresse
/// de cette page, code compris, est recollée dans DumbMonit.
pub const LOOPBACK_REDIRECT_URI: &str = "http://127.0.0.1:8888/callback";

/// Retour direct, quand DumbMonit est servi en HTTPS : relatif à l'origine.
pub const CALLBACK_PATH: &str = "/api/music/spotify/callback";

/// Durée de vie d'un jeton de rafraîchissement (« six mois »), arrondie par défaut.
pub const REFRESH_TOKEN_DAYS: i64 = 182;

/// Seule origine d'image que l'interface affiche (CSP `img-src`).
pub const COVER_ORIGIN: &str = "https://i.scdn.co";

/// Adresses de Spotify. Remplaçables pour éprouver le parcours complet contre
/// un faux serveur (`DUMBMONIT_SPOTIFY_ACCOUNTS_URL`,
/// `DUMBMONIT_SPOTIFY_API_URL`) ; aucune raison d'y toucher en production.
#[derive(Debug, Clone)]
pub struct Endpoints {
    pub accounts: String,
    pub api: String,
}

impl Endpoints {
    pub fn official() -> Self {
        Self {
            accounts: "https://accounts.spotify.com".into(),
            api: "https://api.spotify.com".into(),
        }
    }

    pub fn from_env() -> Self {
        let official = Self::official();
        let var = |key: &str| {
            crate::config::env_var(key).map(|url| url.trim().trim_end_matches('/').to_string())
        };
        Self {
            accounts: var("DUMBMONIT_SPOTIFY_ACCOUNTS_URL").unwrap_or(official.accounts),
            api: var("DUMBMONIT_SPOTIFY_API_URL").unwrap_or(official.api),
        }
    }
}

/// Ce qui peut mal tourner en parlant à Spotify, rangé par remède.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpotifyError {
    /// Aucun compte relié.
    NotConnected,
    /// Spotify ne reconnaît plus le jeton de rafraîchissement (révoqué depuis
    /// le compte, ou six mois écoulés) : il faut se reconnecter.
    Expired(String),
    /// Spotify a refusé la demande (application mal réglée, compte absent de la
    /// liste des utilisateurs de l'application…). Le message est le sien.
    Rejected(String),
    /// Trop de demandes : réessayer après ce délai.
    RateLimited(Duration),
    /// Spotify injoignable ou en panne.
    Unreachable(String),
}

impl SpotifyError {
    /// Phrase pour l'interface : ce qui se passe, et quoi faire.
    pub fn message(&self) -> String {
        match self {
            Self::NotConnected => "No Spotify account is connected.".into(),
            Self::Expired(why) => format!(
                "Spotify ended the connection ({why}). An admin needs to connect it again \
                 in Settings → Wall music."
            ),
            Self::Rejected(why) => format!("Spotify refused the request: {why}"),
            Self::RateLimited(wait) => {
                format!("Spotify asked to slow down; trying again in {} s.", wait.as_secs().max(1))
            }
            Self::Unreachable(why) => format!("Spotify could not be reached: {why}"),
        }
    }
}

/// Jetons rendus par `POST /api/token`.
#[derive(Debug, Clone, Deserialize)]
pub struct Tokens {
    pub access_token: String,
    #[serde(default = "default_expiry")]
    pub expires_in: u64,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
}

fn default_expiry() -> u64 {
    3600
}

#[derive(Debug, Default, Deserialize)]
struct OAuthError {
    #[serde(default)]
    error: String,
    #[serde(default)]
    error_description: Option<String>,
}

/// Erreur d'API Web : `{"error": {"status": 403, "message": "…"}}`.
#[derive(Debug, Default, Deserialize)]
struct ApiErrorBody {
    #[serde(default)]
    error: ApiErrorInner,
}

#[derive(Debug, Default, Deserialize)]
struct ApiErrorInner {
    #[serde(default)]
    message: String,
}

/// L'adresse d'autorisation où le navigateur est envoyé.
pub fn authorize_url(
    endpoints: &Endpoints,
    client_id: &str,
    redirect_uri: &str,
    state: &str,
    challenge: &str,
) -> String {
    let base = format!("{}/authorize", endpoints.accounts);
    let mut url = reqwest::Url::parse(&base)
        .unwrap_or_else(|_| reqwest::Url::parse("https://accounts.spotify.com/authorize").unwrap());
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", client_id)
        .append_pair("scope", &SCOPES.join(" "))
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("state", state)
        .append_pair("code_challenge_method", "S256")
        .append_pair("code_challenge", challenge);
    url.to_string()
}

/// Échange le code d'autorisation contre les jetons.
pub async fn exchange_code(
    http: &reqwest::Client,
    endpoints: &Endpoints,
    client_id: &str,
    code: &str,
    redirect_uri: &str,
    verifier: &str,
) -> Result<Tokens, SpotifyError> {
    token_request(
        http,
        endpoints,
        &[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", client_id),
            ("code_verifier", verifier),
        ],
    )
    .await
}

/// Nouveau jeton d'accès à partir du jeton de rafraîchissement.
pub async fn refresh(
    http: &reqwest::Client,
    endpoints: &Endpoints,
    client_id: &str,
    refresh_token: &str,
) -> Result<Tokens, SpotifyError> {
    token_request(
        http,
        endpoints,
        &[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", client_id),
        ],
    )
    .await
}

async fn token_request(
    http: &reqwest::Client,
    endpoints: &Endpoints,
    form: &[(&str, &str)],
) -> Result<Tokens, SpotifyError> {
    let response = http
        .post(format!("{}/api/token", endpoints.accounts))
        .form(form)
        .send()
        .await
        .map_err(|error| SpotifyError::Unreachable(short(&error)))?;
    let status = response.status();
    if status.is_success() {
        return response
            .json::<Tokens>()
            .await
            .map_err(|_| SpotifyError::Unreachable("unreadable token response".into()));
    }
    if let Some(wait) = retry_after(&response) {
        return Err(SpotifyError::RateLimited(wait));
    }
    let body: OAuthError = response.json().await.unwrap_or_default();
    let detail = match body.error_description.filter(|d| !d.trim().is_empty()) {
        Some(description) => description,
        None if !body.error.is_empty() => body.error.clone(),
        None => format!("HTTP {}", status.as_u16()),
    };
    match (status, body.error.as_str()) {
        // Le jeton de rafraîchissement (ou le code) n'est plus valable.
        (_, "invalid_grant") => Err(SpotifyError::Expired(detail)),
        (s, _) if s.is_server_error() => Err(SpotifyError::Unreachable(detail)),
        _ => Err(SpotifyError::Rejected(detail)),
    }
}

/// Le compte connecté, tel que `GET /v1/me` le décrit.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Profile {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub display_name: Option<String>,
}

pub async fn me(
    http: &reqwest::Client,
    endpoints: &Endpoints,
    access_token: &str,
) -> Result<Profile, SpotifyError> {
    let response = http
        .get(format!("{}/v1/me", endpoints.api))
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|error| SpotifyError::Unreachable(short(&error)))?;
    if response.status().is_success() {
        return response
            .json()
            .await
            .map_err(|_| SpotifyError::Unreachable("unreadable profile".into()));
    }
    Err(api_error(response).await)
}

/// Réponse de `GET /v1/me/player`.
#[derive(Debug, Clone, PartialEq)]
pub enum Player {
    /// 204 : aucune session de lecture active.
    Idle,
    State(Box<NowPlaying>),
    /// 401 : le jeton d'accès n'est plus accepté (à rafraîchir puis réessayer).
    Unauthorized,
}

pub async fn player(
    http: &reqwest::Client,
    endpoints: &Endpoints,
    access_token: &str,
) -> Result<Player, SpotifyError> {
    let response = http
        .get(format!("{}/v1/me/player", endpoints.api))
        .query(&[("additional_types", "episode")])
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|error| SpotifyError::Unreachable(short(&error)))?;
    match response.status() {
        StatusCode::NO_CONTENT => Ok(Player::Idle),
        StatusCode::UNAUTHORIZED => Ok(Player::Unauthorized),
        status if status.is_success() => {
            let text = response
                .text()
                .await
                .map_err(|_| SpotifyError::Unreachable("unreadable playback state".into()))?;
            if text.trim().is_empty() {
                return Ok(Player::Idle);
            }
            let raw: RawPlayback = serde_json::from_str(&text)
                .map_err(|_| SpotifyError::Unreachable("unreadable playback state".into()))?;
            Ok(raw.into_now_playing().map_or(Player::Idle, |now| Player::State(Box::new(now))))
        }
        _ => Err(api_error(response).await),
    }
}

async fn api_error(response: reqwest::Response) -> SpotifyError {
    if let Some(wait) = retry_after(&response) {
        return SpotifyError::RateLimited(wait);
    }
    let status = response.status();
    let body: ApiErrorBody = response.json().await.unwrap_or_default();
    let detail = match body.error.message.trim() {
        "" => format!("HTTP {}", status.as_u16()),
        message => message.to_string(),
    };
    if status.is_server_error() {
        SpotifyError::Unreachable(detail)
    } else {
        SpotifyError::Rejected(detail)
    }
}

/// 429 et son délai (`Retry-After`, en secondes), borné pour qu'un en-tête
/// farfelu ne fige pas le mur une journée.
fn retry_after(response: &reqwest::Response) -> Option<Duration> {
    if response.status() != StatusCode::TOO_MANY_REQUESTS {
        return None;
    }
    let seconds = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(30);
    Some(Duration::from_secs(seconds.clamp(1, 600)))
}

/// Erreur réseau sans l'URL complète (elle porterait la requête).
fn short(error: &reqwest::Error) -> String {
    if error.is_timeout() {
        "timed out".into()
    } else if error.is_connect() {
        "connection failed".into()
    } else {
        "network error".into()
    }
}

// ------------------------------------------------------------ lecture en cours

/// Ce qui joue, réduit à ce que le mur affiche.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NowPlaying {
    /// En lecture (`false` : en pause).
    pub playing: bool,
    /// `track`, `episode`, `ad` ou `unknown`.
    pub kind: String,
    pub title: String,
    /// Artistes d'un titre ; le nom de l'émission pour un épisode.
    pub artists: Vec<String>,
    /// Album d'un titre ; l'émission pour un épisode.
    pub album: Option<String>,
    /// Pochette, uniquement sur `https://i.scdn.co/` (CSP `img-src`).
    pub cover_url: Option<String>,
    pub duration_ms: u64,
    pub progress_ms: u64,
    /// Appareil de sortie : « Living room TV », « DumbMonit Wall »…
    pub device_name: Option<String>,
    /// Type d'appareil selon Spotify : `Computer`, `Smartphone`, `Speaker`, `TV`…
    pub device_type: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct RawPlayback {
    #[serde(default)]
    device: Option<RawDevice>,
    #[serde(default)]
    is_playing: bool,
    #[serde(default)]
    progress_ms: Option<u64>,
    #[serde(default)]
    currently_playing_type: Option<String>,
    #[serde(default)]
    item: Option<RawItem>,
}

#[derive(Debug, Default, Deserialize)]
struct RawDevice {
    #[serde(default)]
    name: Option<String>,
    #[serde(default, rename = "type")]
    kind: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct RawItem {
    #[serde(default)]
    name: String,
    #[serde(default)]
    duration_ms: u64,
    #[serde(default)]
    artists: Vec<RawNamed>,
    #[serde(default)]
    album: Option<RawAlbum>,
    /// Épisode : l'émission, et ses images propres.
    #[serde(default)]
    show: Option<RawAlbum>,
    #[serde(default)]
    images: Vec<RawImage>,
}

#[derive(Debug, Default, Deserialize)]
struct RawNamed {
    #[serde(default)]
    name: String,
}

#[derive(Debug, Default, Deserialize)]
struct RawAlbum {
    #[serde(default)]
    name: String,
    #[serde(default)]
    images: Vec<RawImage>,
}

#[derive(Debug, Default, Clone, Deserialize)]
struct RawImage {
    #[serde(default)]
    url: String,
    #[serde(default)]
    width: Option<u32>,
}

impl RawPlayback {
    /// `None` quand rien n'est à montrer (aucun élément, et ce n'est pas une pub).
    fn into_now_playing(self) -> Option<NowPlaying> {
        let kind = self.currently_playing_type.unwrap_or_else(|| "unknown".into());
        let device_name = self.device.as_ref().and_then(|d| d.name.clone());
        let device_type = self.device.as_ref().and_then(|d| d.kind.clone());
        let progress_ms = self.progress_ms.unwrap_or(0);
        let Some(item) = self.item else {
            // Une coupure publicitaire n'a pas d'élément, mais quelque chose joue.
            return (kind == "ad").then(|| NowPlaying {
                playing: self.is_playing,
                kind,
                title: "Advertisement".into(),
                artists: Vec::new(),
                album: None,
                cover_url: None,
                duration_ms: 0,
                progress_ms,
                device_name,
                device_type,
            });
        };
        let (artists, album, images) = match item.show {
            Some(show) => {
                let images = if item.images.is_empty() { show.images } else { item.images };
                (vec![show.name.clone()], Some(show.name), images)
            }
            None => {
                let album = item.album.unwrap_or_default();
                let artists = item
                    .artists
                    .into_iter()
                    .map(|a| a.name)
                    .filter(|name| !name.trim().is_empty())
                    .collect();
                let images = if album.images.is_empty() { item.images } else { album.images };
                (artists, Some(album.name).filter(|n| !n.trim().is_empty()), images)
            }
        };
        Some(NowPlaying {
            playing: self.is_playing,
            kind,
            title: item.name,
            artists,
            album,
            cover_url: pick_cover(&images),
            duration_ms: item.duration_ms,
            progress_ms: match item.duration_ms {
                0 => progress_ms,
                duration => progress_ms.min(duration),
            },
            device_name,
            device_type,
        })
    }
}

/// La plus petite image d'au moins 300 px (lisible de loin sans peser), sinon
/// la plus grande — et seulement si elle vient du CDN d'images de Spotify.
fn pick_cover(images: &[RawImage]) -> Option<String> {
    let allowed = || images.iter().filter_map(|image| Some((cover_url(&image.url)?, image.width)));
    let width = |(_, width): &(String, Option<u32>)| width.unwrap_or(0);
    allowed()
        .filter(|image| width(image) >= 300)
        .min_by_key(width)
        .or_else(|| allowed().max_by_key(width))
        .map(|(url, _)| url)
}

/// Une pochette sur `i.scdn.co`, seule origine d'image de l'interface. Spotify
/// sert aussi ses images depuis `image-cdn-*.spotifycdn.com`, sous le même
/// identifiant : celles-là sont ramenées sur `i.scdn.co` plutôt que d'ouvrir la
/// politique de contenu à un joker. Toute autre adresse est écartée.
pub fn cover_url(url: &str) -> Option<String> {
    let parsed = reqwest::Url::parse(url).ok()?;
    let host = parsed.host_str()?;
    let from_cdn = host == "i.scdn.co"
        || (host.starts_with("image-cdn-") && host.ends_with(".spotifycdn.com"));
    let id = parsed.path().strip_prefix("/image/")?;
    let clean = parsed.scheme() == "https"
        && from_cdn
        && parsed.port().is_none()
        && parsed.query().is_none()
        && !id.is_empty()
        && id.len() <= 64
        && id.bytes().all(|b| b.is_ascii_alphanumeric());
    clean.then(|| format!("{COVER_ORIGIN}/image/{id}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> Option<NowPlaying> {
        serde_json::from_str::<RawPlayback>(json).unwrap().into_now_playing()
    }

    #[test]
    fn a_track_is_reduced_to_what_the_wall_shows() {
        let now = parse(
            r#"{
                "device": {"id": "x", "name": "Living room TV", "type": "TV", "volume_percent": 40},
                "is_playing": true, "progress_ms": 61000, "timestamp": 1, "currently_playing_type": "track",
                "item": {"name": "Teardrop", "duration_ms": 330000,
                         "artists": [{"name": "Massive Attack"}, {"name": "Elizabeth Fraser"}],
                         "album": {"name": "Mezzanine", "images": [
                            {"url": "https://i.scdn.co/image/640", "width": 640, "height": 640},
                            {"url": "https://i.scdn.co/image/300", "width": 300, "height": 300},
                            {"url": "https://i.scdn.co/image/64", "width": 64, "height": 64}]}}
            }"#,
        )
        .unwrap();
        assert!(now.playing);
        assert_eq!(now.kind, "track");
        assert_eq!(now.title, "Teardrop");
        assert_eq!(now.artists, ["Massive Attack", "Elizabeth Fraser"]);
        assert_eq!(now.album.as_deref(), Some("Mezzanine"));
        assert_eq!(now.cover_url.as_deref(), Some("https://i.scdn.co/image/300"));
        assert_eq!((now.progress_ms, now.duration_ms), (61000, 330000));
        assert_eq!(now.device_name.as_deref(), Some("Living room TV"));
        assert_eq!(now.device_type.as_deref(), Some("TV"));
    }

    #[test]
    fn an_episode_shows_its_show() {
        let now = parse(
            r#"{"is_playing": false, "currently_playing_type": "episode",
                "item": {"name": "Ep. 12", "duration_ms": 10, "images": [{"url": "https://i.scdn.co/image/ep1", "width": 640}],
                         "show": {"name": "The Homelab Show", "images": []}}}"#,
        )
        .unwrap();
        assert!(!now.playing);
        assert_eq!(now.artists, ["The Homelab Show"]);
        assert_eq!(now.cover_url.as_deref(), Some("https://i.scdn.co/image/ep1"));
        assert_eq!(now.device_name, None);
    }

    #[test]
    fn covers_from_elsewhere_are_dropped() {
        let now = parse(
            r#"{"is_playing": true, "currently_playing_type": "track",
                "item": {"name": "x", "duration_ms": 1, "album": {"name": "a", "images": [
                    {"url": "https://evil.example/cover.png", "width": 300},
                    {"url": "http://i.scdn.co/image/insecure", "width": 640}]}}}"#,
        )
        .unwrap();
        assert_eq!(now.cover_url, None);
    }

    #[test]
    fn covers_from_the_other_spotify_cdn_are_brought_back_to_i_scdn_co() {
        let id = "ab67616d00001e02f907de96b9a4fbc04accc0d5";
        let same = format!("https://i.scdn.co/image/{id}");
        assert_eq!(cover_url(&same).as_deref(), Some(same.as_str()));
        assert_eq!(
            cover_url(&format!("https://image-cdn-fa.spotifycdn.com/image/{id}")).as_deref(),
            Some(same.as_str())
        );
        for refused in [
            format!("https://image-cdn-fa.spotifycdn.com.evil.test/image/{id}"),
            format!("https://i.scdn.co/other/{id}"),
            "https://i.scdn.co/image/".to_string(),
            "https://i.scdn.co/image/a\"b".to_string(),
            format!("https://i.scdn.co:8443/image/{id}"),
            format!("https://mosaic.scdn.co/640/{id}"),
        ] {
            assert_eq!(cover_url(&refused), None, "{refused}");
        }
    }

    #[test]
    fn nothing_to_show_without_an_item_except_an_ad() {
        assert_eq!(parse(r#"{"is_playing": true, "currently_playing_type": "track"}"#), None);
        let ad = parse(r#"{"is_playing": true, "currently_playing_type": "ad"}"#).unwrap();
        assert_eq!(ad.kind, "ad");
        assert_eq!(ad.title, "Advertisement");
    }

    #[test]
    fn the_authorize_url_carries_pkce_and_the_scopes() {
        let url = authorize_url(
            &Endpoints::official(),
            "0123456789abcdef0123456789abcdef",
            LOOPBACK_REDIRECT_URI,
            "state-1",
            "challenge-1",
        );
        let parsed = reqwest::Url::parse(&url).unwrap();
        assert_eq!(parsed.origin().ascii_serialization(), "https://accounts.spotify.com");
        assert_eq!(parsed.path(), "/authorize");
        let query: std::collections::HashMap<_, _> = parsed.query_pairs().into_owned().collect();
        assert_eq!(query["response_type"], "code");
        assert_eq!(query["code_challenge_method"], "S256");
        assert_eq!(query["code_challenge"], "challenge-1");
        assert_eq!(query["redirect_uri"], "http://127.0.0.1:8888/callback");
        assert_eq!(query["state"], "state-1");
        for scope in ["streaming", "user-read-playback-state", "user-modify-playback-state"] {
            assert!(query["scope"].split(' ').any(|s| s == scope), "{scope}");
        }
        assert!(!query.contains_key("client_secret"));
    }

    #[test]
    fn errors_say_what_to_do() {
        assert!(SpotifyError::Expired("revoked".into()).message().contains("connect it again"));
        assert!(SpotifyError::RateLimited(Duration::from_secs(12)).message().contains("12 s"));
    }
}
