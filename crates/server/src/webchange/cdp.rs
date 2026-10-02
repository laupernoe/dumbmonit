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

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

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

/// Capture `page_url` en pleine page (hauteur bornée), au format JPEG.
pub async fn capture(browser_url: &str, page_url: &str) -> Result<Vec<u8>, String> {
    let endpoint = websocket_url(browser_url).await?;
    let (socket, _) = tokio::time::timeout(
        Duration::from_secs(10),
        tokio_tungstenite::connect_async(endpoint.as_str()),
    )
    .await
    .map_err(|_| "the browser did not accept the connection in time".to_string())?
    .map_err(|error| format!("cannot connect to the browser: {error}"))?;
    let mut session = Session { socket, next_id: 0, events: Vec::new() };

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
    tokio::time::sleep(SETTLE).await;

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
}

impl Session {
    async fn call(
        &mut self,
        method: &str,
        params: Value,
        session: Option<&str>,
    ) -> Result<Value, String> {
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
        let image = capture(&browser, &page).await.expect("capture");
        assert!(image.starts_with(&[0xFF, 0xD8, 0xFF]), "une image JPEG");
        if let Ok(out) = std::env::var("DUMBMONIT_TEST_SCREENSHOT_OUT") {
            std::fs::write(out, &image).expect("écriture de la capture");
        }
    }

    #[tokio::test]
    async fn un_navigateur_absent_est_indisponible_sans_attendre() {
        assert!(!available("http://127.0.0.1:1").await);
        assert!(!available("pas une adresse").await);
        assert!(capture("http://127.0.0.1:1", "https://example.com/").await.is_err());
    }
}
