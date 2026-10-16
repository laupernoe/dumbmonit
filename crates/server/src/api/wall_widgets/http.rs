//! Client des sources publiques des widgets (adresses fixes, écrites ici : rien
//! de saisi par l'utilisateur ne choisit l'hôte appelé).

use std::time::Duration;

use serde::de::DeserializeOwned;

use crate::api::wall_news::describe;

const TIMEOUT: Duration = Duration::from_secs(8);
const USER_AGENT: &str = concat!(
    "DumbMonit/",
    env!("CARGO_PKG_VERSION"),
    " (wall display; +https://github.com/laupernoe/dumbmonit)"
);

pub fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(3))
        .connect_timeout(Duration::from_secs(5))
        .timeout(TIMEOUT)
        .user_agent(USER_AGENT)
        .build()
        .map_err(|error| format!("HTTP client unavailable: {error}"))
}

/// Envoie la requête et lit la réponse, `max` octets au plus.
pub async fn bytes(request: reqwest::RequestBuilder, max: usize) -> Result<Vec<u8>, String> {
    let mut response = request.send().await.map_err(describe)?;
    let status = response.status();
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err("the provider's rate limit was reached".into());
    }
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Err(format!("the provider refused the request ({})", status.as_u16()));
    }
    if !status.is_success() {
        return Err(format!("answered {}", status.as_u16()));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(describe)? {
        if body.len() + chunk.len() > max {
            return Err(format!("the answer is larger than {} MB", max / (1024 * 1024)));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

pub fn json<T: DeserializeOwned>(body: &[u8]) -> Result<T, String> {
    serde_json::from_slice(body).map_err(|error| format!("unexpected answer: {error}"))
}
