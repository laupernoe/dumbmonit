//! Captures d'écran par un Chromium sans interface, piloté par le protocole
//! DevTools (CDP).
//!
//! L'image du serveur part de `scratch` : aucun navigateur n'y tient, et en
//! embarquer un tripleraient sa taille pour une fonction facultative. Le
//! navigateur est donc un conteneur à part (`chromedp/headless-shell`), que
//! `DUMBMONIT_BROWSER_URL` désigne (`http://browser:9222`). Sans lui, la
//! comparaison de texte fonctionne seule.
//!
//! Le client est volontairement minimal : une connexion WebSocket au
//! navigateur, une session « aplatie » par capture (`Target.attachToTarget`
//! avec `flatten`), et les cinq commandes nécessaires. Chromium refuse les
//! requêtes DevTools dont l'en-tête `Host` n'est ni une adresse IP ni
//! `localhost` : le nom du service est donc résolu, et l'on parle à son
//! adresse.
//!
//! Le navigateur résout et suit lui-même redirections, cadres et ressources :
//! le garde-fou du robot (`guard`) ne le couvre pas. Chaque requête de la page
//! est donc interceptée (`Fetch.enable`) et jugée par la même règle avant de
//! partir, l'adresse effectivement contactée est relue sur chaque réponse
//! (`Network.responseReceived`, contre un nom qui changerait de réponse entre
//! les deux), et l'adresse finale de la page est vérifiée : au moindre écart,
//! la capture est jetée plutôt qu'enregistrée.

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use dumbmonit_collectors::uptime::guard;
use futures::{SinkExt, StreamExt};
use reqwest::Url;
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::Message;

/// Largeur et hauteur de la fenêtre simulée : un portable courant.
const VIEWPORT_WIDTH: u32 = 1366;
const VIEWPORT_HEIGHT: u32 = 900;
/// Hauteur maximale d'une capture pleine page. Au-delà, l'image pèse
/// plusieurs mégaoctets pour un pied de page que personne ne regarde.
const MAX_HEIGHT: f64 = 5000.0;
/// Qualité JPEG : le texte reste lisible, le poids reste raisonnable.
const JPEG_QUALITY: u32 = 70;
/// Attente du chargement de la page.
const LOAD_TIMEOUT: Duration = Duration::from_secs(20);
/// Laisse aux scripts le temps de dessiner après l'événement `load`.
const SETTLE: Duration = Duration::from_millis(1500);
/// Délai maximal d'une commande.
const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);
/// Durée pendant laquelle la réponse « le navigateur est-il là ? » est
/// réutilisée : la liste des pages la pose à chaque affichage.
const AVAILABILITY_TTL: Duration = Duration::from_secs(30);

/// Type MIME des captures produites.
pub const CONTENT_TYPE: &str = "image/jpeg";

/// Le navigateur répond-il ? Réponse mise en cache [`AVAILABILITY_TTL`].
pub async fn available(browser_url: &str) -> bool {
    static CACHE: LazyLock<Mutex<HashMap<String, (Instant, bool)>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));
    if let Some((at, ok)) = CACHE.lock().unwrap_or_else(|p| p.into_inner()).get(browser_url)
        && at.elapsed() < AVAILABILITY_TTL
    {
        return *ok;
    }
    let ok = tokio::time::timeout(Duration::from_secs(3), websocket_url(browser_url))
        .await
        .is_ok_and(|result| result.is_ok());
    CACHE
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .insert(browser_url.to_string(), (Instant::now(), ok));
    ok
}

/// Règle réseau imposée au navigateur pendant une capture : celle du robot.
///
/// Sans `allow_private`, rien de ce que la page demande ne doit atteindre la
/// boucle locale, le lien local ou un service de métadonnées — ni la page
/// elle-même après redirection, ni un cadre, ni une image.
struct NetGuard {
    allow_private: bool,
    /// Verdict par hôte et port, le temps d'une capture.
    verdicts: HashMap<String, bool>,
    /// Première adresse interdite effectivement contactée, s'il y en a eu une.
    violation: Option<String>,
}

impl NetGuard {
    fn new(allow_private: bool) -> Self {
        Self { allow_private, verdicts: HashMap::new(), violation: None }
    }

    /// La page peut-elle demander cette URL ?
    async fn allows(&mut self, raw: &str) -> bool {
        if self.allow_private {
            return true;
        }
        let Ok(url) = Url::parse(raw) else { return false };
        match url.scheme() {
            // Rien ne sort du navigateur.
            "data" | "blob" | "about" => return true,
            "http" | "https" | "ws" | "wss" => {}
            // `file:`, `chrome:`, `ftp:`… : jamais.
            _ => return false,
        }
        let Some(host) = url.host_str() else { return false };
        let key = format!("{host}:{}", url.port_or_known_default().unwrap_or(0));
        if let Some(verdict) = self.verdicts.get(&key) {
            return *verdict;
        }
        let verdict = guard::vet_url(&url, false, Duration::from_secs(5)).await.is_ok();
        self.verdicts.insert(key, verdict);
        verdict
    }

    /// Relève l'adresse qu'une réponse dit avoir contactée.
    fn observe(&mut self, url: &str, remote: &str) {
        if self.allow_private || self.violation.is_some() {
            return;
        }
        let remote = remote.trim_matches(|c| c == '[' || c == ']');
        if let Ok(ip) = remote.parse::<IpAddr>()
            && guard::is_forbidden(ip)
        {
            self.violation = Some(format!("{url} was served from {ip}"));
        }
    }
}

/// Capture `page_url` en pleine page (hauteur bornée), au format JPEG.
///
/// `allow_private` reprend l'option `allow_private_targets` de la cible.
pub async fn capture(
    browser_url: &str,
    page_url: &str,
    allow_private: bool,
) -> Result<Vec<u8>, String> {
    let endpoint = websocket_url(browser_url).await?;
    let (socket, _) = tokio::time::timeout(
        Duration::from_secs(10),
        tokio_tungstenite::connect_async(endpoint.as_str()),
    )
    .await
    .map_err(|_| "the browser did not accept the connection in time".to_string())?
    .map_err(|error| format!("cannot connect to the browser: {error}"))?;
    let mut session =
        Session { socket, next_id: 0, events: Vec::new(), guard: NetGuard::new(allow_private) };

    let target = session.call("Target.createTarget", json!({ "url": "about:blank" }), None).await?
        ["targetId"]
        .as_str()
        .ok_or("the browser created no page")?
        .to_string();
    let outcome = shoot(&mut session, &target, page_url).await;
    // Toujours refermer l'onglet, même après un échec : un navigateur partagé
    // ne doit pas accumuler des pages ouvertes.
    let _ = session.call("Target.closeTarget", json!({ "targetId": target }), None).await;
    let _ = session.socket.close(None).await;
    outcome
}

async fn shoot(session: &mut Session, target: &str, page_url: &str) -> Result<Vec<u8>, String> {
    let attached = session
        .call("Target.attachToTarget", json!({ "targetId": target, "flatten": true }), None)
        .await?;
    let id = attached["sessionId"].as_str().ok_or("the browser gave no session")?.to_string();
    let sid = Some(id.as_str());

    session.call("Page.enable", json!({}), sid).await?;
    // Toute requête de la page s'arrête ici avant de partir, et n'en repart
    // que si l'hôte est permis (voir `Session::handle_event`).
    session.call("Network.enable", json!({}), sid).await?;
    session
        .call(
            "Fetch.enable",
            json!({ "patterns": [{ "urlPattern": "*", "requestStage": "Request" }] }),
            sid,
        )
        .await?;
    session
        .call(
            "Emulation.setDeviceMetricsOverride",
            json!({
                "width": VIEWPORT_WIDTH,
                "height": VIEWPORT_HEIGHT,
                "deviceScaleFactor": 1,
                "mobile": false
            }),
            sid,
        )
        .await?;
    let navigation = session.call("Page.navigate", json!({ "url": page_url }), sid).await?;
    if let Some(error) = navigation["errorText"].as_str().filter(|e| !e.is_empty()) {
        return Err(format!("the browser could not open the page: {error}"));
    }
    // Une page qui ne finit jamais de charger (flux, publicité) est capturée
    // dans l'état où elle est : mieux vaut une image incomplète que rien.
    let _ = session.wait_event("Page.loadEventFired", sid, LOAD_TIMEOUT).await;
    // La pause laisse aux scripts le temps de dessiner ; les requêtes qu'ils
    // lancent pendant ce temps doivent toujours être jugées.
    let _ = session.wait_event("DumbMonit.never", sid, SETTLE).await;

    // Adresse finale de la page, après redirections et scripts.
    let tree = session.call("Page.getFrameTree", json!({}), sid).await?;
    let final_url = tree["frameTree"]["frame"]["url"].as_str().unwrap_or_default().to_string();
    if !session.guard.allows(&final_url).await {
        return Err(format!(
            "the page ended up on {final_url}, a loopback or link-local address: screenshot \
             discarded (enable \"{}\" to allow it)",
            guard::OPTION
        ));
    }
    if let Some(violation) = session.guard.violation.take() {
        return Err(format!(
            "the page reached a loopback or link-local address ({violation}): screenshot \
             discarded (enable \"{}\" to allow it)",
            guard::OPTION
        ));
    }

    let metrics = session.call("Page.getLayoutMetrics", json!({}), sid).await?;
    let content = if metrics["cssContentSize"].is_object() {
        &metrics["cssContentSize"]
    } else {
        &metrics["contentSize"]
    };
    let height = content["height"]
        .as_f64()
        .unwrap_or(f64::from(VIEWPORT_HEIGHT))
        .clamp(f64::from(VIEWPORT_HEIGHT), MAX_HEIGHT)
        .ceil();
    let shot = session
        .call(
            "Page.captureScreenshot",
            json!({
                "format": "jpeg",
                "quality": JPEG_QUALITY,
                "captureBeyondViewport": true,
                "clip": { "x": 0, "y": 0, "width": VIEWPORT_WIDTH, "height": height, "scale": 1 }
            }),
            sid,
        )
        .await?;
    let data = shot["data"].as_str().ok_or("the browser returned no image")?;
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|error| format!("unreadable image from the browser: {error}"))
}

/// Adresse WebSocket du navigateur, lue sur `/json/version` et réécrite vers
/// l'adresse IP du service (voir l'en-tête du module).
async fn websocket_url(browser_url: &str) -> Result<String, String> {
    let base = Url::parse(browser_url.trim())
        .map_err(|error| format!("DUMBMONIT_BROWSER_URL is not a URL: {error}"))?;
    let host = base.host_str().ok_or("DUMBMONIT_BROWSER_URL has no host")?;
    let host = host.trim_matches(|c| c == '[' || c == ']');
    let port = base.port_or_known_default().unwrap_or(9222);
    let address: SocketAddr = tokio::net::lookup_host((host, port))
        .await
        .map_err(|error| format!("cannot resolve the browser host {host}: {error}"))?
        .min_by_key(|address| address.is_ipv6())
        .ok_or_else(|| format!("the browser host {host} has no address"))?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|error| error.to_string())?;
    let version: Value = client
        .get(format!("http://{address}/json/version"))
        .send()
        .await
        .map_err(|error| format!("the browser does not answer: {error}"))?
        .error_for_status()
        .map_err(|error| format!("the browser refused /json/version: {error}"))?
        .json()
        .await
        .map_err(|error| format!("unexpected /json/version answer: {error}"))?;
    let advertised = version["webSocketDebuggerUrl"]
        .as_str()
        .ok_or("the browser advertises no DevTools endpoint")?;
    let path = Url::parse(advertised)
        .map_err(|error| format!("unexpected DevTools endpoint {advertised}: {error}"))?
        .path()
        .to_string();
    Ok(format!("ws://{address}{path}"))
}

type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

struct Session {
    socket: Socket,
    next_id: u64,
    /// Événements reçus en attendant une réponse, relus par `wait_event`.
    events: Vec<Value>,
    guard: NetGuard,
}

impl Session {
    /// Traite sur-le-champ les événements de la garde réseau ; rend `false`
    /// pour les autres, que l'appelant conserve.
    ///
    /// Une requête interceptée reste suspendue tant qu'on ne l'a pas relâchée :
    /// elle doit donc l'être ici, au fil de l'eau, y compris pendant que l'on
    /// attend la réponse à `Page.navigate` — qui n'arrive qu'une fois la requête
    /// du document elle-même jugée. La réponse à `continueRequest` /
    /// `failRequest` n'est pas attendue : elle porte un identifiant que personne
    /// ne réclame, et sera ignorée.
    async fn handle_event(&mut self, value: &Value) -> Result<bool, String> {
        match value["method"].as_str() {
            Some("Fetch.requestPaused") => {
                let params = &value["params"];
                let request_id = params["requestId"].as_str().unwrap_or_default().to_string();
                let url = params["request"]["url"].as_str().unwrap_or_default().to_string();
                let session = value["sessionId"].as_str().map(str::to_string);
                let (method, params) = if self.guard.allows(&url).await {
                    ("Fetch.continueRequest", json!({ "requestId": request_id }))
                } else {
                    tracing::debug!(%url, "screenshot: request to a forbidden address blocked");
                    (
                        "Fetch.failRequest",
                        json!({ "requestId": request_id, "errorReason": "AccessDenied" }),
                    )
                };
                self.send(method, params, session.as_deref()).await?;
                Ok(true)
            }
            Some("Network.responseReceived") => {
                let response = &value["params"]["response"];
                self.guard.observe(
                    response["url"].as_str().unwrap_or_default(),
                    response["remoteIPAddress"].as_str().unwrap_or_default(),
                );
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// Envoie une commande sans attendre sa réponse ; rend son identifiant.
    async fn send(
        &mut self,
        method: &str,
        params: Value,
        session: Option<&str>,
    ) -> Result<u64, String> {
        self.next_id += 1;
        let id = self.next_id;
        let mut message = json!({ "id": id, "method": method, "params": params });
        if let Some(session) = session {
            message["sessionId"] = json!(session);
        }
        self.socket
            .send(Message::text(message.to_string()))
            .await
            .map_err(|error| format!("browser connection lost: {error}"))?;
        Ok(id)
    }

    async fn call(
        &mut self,
        method: &str,
        params: Value,
        session: Option<&str>,
    ) -> Result<Value, String> {
        let id = self.send(method, params, session).await?;
        let deadline = Instant::now() + COMMAND_TIMEOUT;
        loop {
            let value = self.next_message(deadline).await.map_err(|e| format!("{method}: {e}"))?;
            if value["id"].as_u64() == Some(id) {
                if let Some(error) = value.get("error") {
                    let text = error["message"].as_str().unwrap_or("unknown error");
                    return Err(format!("{method}: {text}"));
                }
                return Ok(value["result"].clone());
            }
            if self.handle_event(&value).await? {
                continue;
            }
            if value.get("method").is_some() && self.events.len() < 256 {
                self.events.push(value);
            }
        }
    }

    async fn wait_event(
        &mut self,
        name: &str,
        session: Option<&str>,
        timeout: Duration,
    ) -> Result<(), String> {
        let wanted = |value: &Value| {
            value["method"].as_str() == Some(name)
                && session.is_none_or(|s| value["sessionId"].as_str() == Some(s))
        };
        if let Some(at) = self.events.iter().position(wanted) {
            self.events.remove(at);
            return Ok(());
        }
        let deadline = Instant::now() + timeout;
        loop {
            let value = self.next_message(deadline).await?;
            if self.handle_event(&value).await? {
                continue;
            }
            if wanted(&value) {
                return Ok(());
            }
        }
    }

    async fn next_message(&mut self, deadline: Instant) -> Result<Value, String> {
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let message = tokio::time::timeout(left, self.socket.next())
                .await
                .map_err(|_| "the browser did not answer in time".to_string())?
                .ok_or("the browser closed the connection")?
                .map_err(|error| format!("browser connection lost: {error}"))?;
            if let Message::Text(text) = message {
                return serde_json::from_str(text.as_str())
                    .map_err(|error| format!("unreadable message from the browser: {error}"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Capture réelle, seulement quand un navigateur est fourni :
    /// `docker run -d --rm -p 9222:9222 chromedp/headless-shell`, puis
    /// `DUMBMONIT_TEST_BROWSER_URL=http://127.0.0.1:9222 cargo test capture_reelle`.
    /// Sans la variable, le test ne fait rien : la CI n'a pas de navigateur.
    #[tokio::test]
    async fn capture_reelle_si_un_navigateur_est_fourni() {
        let Ok(browser) = std::env::var("DUMBMONIT_TEST_BROWSER_URL") else { return };
        let page = std::env::var("DUMBMONIT_TEST_PAGE_URL")
            .unwrap_or_else(|_| "https://example.com/".to_string());
        assert!(available(&browser).await, "le navigateur ne répond pas");
        let image = capture(&browser, &page, false).await.expect("capture");
        assert!(image.starts_with(&[0xFF, 0xD8, 0xFF]), "une image JPEG");
        if let Ok(out) = std::env::var("DUMBMONIT_TEST_SCREENSHOT_OUT") {
            std::fs::write(out, &image).expect("écriture de la capture");
        }
    }

    #[tokio::test]
    async fn un_navigateur_absent_est_indisponible_sans_attendre() {
        assert!(!available("http://127.0.0.1:1").await);
        assert!(!available("pas une adresse").await);
        assert!(capture("http://127.0.0.1:1", "https://example.com/", false).await.is_err());
    }

    /// La page ne peut rien demander à la boucle locale, au lien local ni au
    /// service de métadonnées ; le réseau local d'un homelab reste permis.
    #[tokio::test]
    async fn la_garde_du_navigateur_suit_la_regle_du_robot() {
        let mut guard = NetGuard::new(false);
        for url in [
            "http://127.0.0.1:8428/api/v1/admin/tsdb/delete_series",
            "http://169.254.169.254/latest/meta-data/",
            "http://[::1]/",
            "http://localhost/",
            "file:///etc/passwd",
            "chrome://settings",
            "pas une url",
        ] {
            assert!(!guard.allows(url).await, "{url} aurait dû être refusée");
        }
        for url in
            ["http://192.168.1.10/", "https://93.184.216.34/", "data:text/plain,x", "about:blank"]
        {
            assert!(guard.allows(url).await, "{url} aurait dû passer");
        }

        // L'adresse effectivement contactée est relue sur chaque réponse.
        guard.observe("https://example.com/", "93.184.216.34");
        assert!(guard.violation.is_none());
        guard.observe("https://rebind.example/", "169.254.169.254");
        assert!(guard.violation.is_some());

        // L'option de la cible lève la garde.
        let mut open = NetGuard::new(true);
        assert!(open.allows("http://127.0.0.1/").await);
        open.observe("http://127.0.0.1/", "127.0.0.1");
        assert!(open.violation.is_none());
    }
}
