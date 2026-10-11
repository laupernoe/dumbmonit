//! Lecture d'une page en suivant ses redirections, une à une.
//!
//! `reqwest` sait suivre les redirections seul, mais il ne dit pas par où il
//! est passé. On les suit donc à la main : chaque étape est notée (adresse et
//! statut), une boucle est reconnue dès qu'une adresse revient, et le nombre
//! d'étapes est plafonné. Une redirection n'est jamais une erreur en soi —
//! `http://` vers `https://`, le domaine nu vers `www.` sont le lot commun
//! des sites ; seuls la boucle et l'excès d'étapes en sont.
//!
//! Chaque étape passe le garde-fou des sondes de disponibilité : la boucle
//! locale et le lien local ne sont joignables qu'avec `allow_private_targets`,
//! redirection comprise.

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use dumbmonit_collectors::uptime::guard;
use dumbmonit_proto::ProbeError;
use reqwest::Url;
use reqwest::header::{HeaderMap, LOCATION};
use serde::{Deserialize, Serialize};

pub const USER_AGENT: &str = concat!("DumbMonit/", env!("CARGO_PKG_VERSION"));

/// Délai d'une requête, corps compris.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
/// Corps lu au plus : au-delà, la fin est ignorée.
pub const MAX_BODY_BYTES: usize = 5 * 1024 * 1024;

/// Une étape de redirection : l'adresse demandée et le statut 3xx qu'elle a
/// répondu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hop {
    pub url: String,
    pub status: u16,
}

/// La réponse d'une seule requête.
#[derive(Debug, Clone, Default)]
pub struct Answer {
    pub status: u16,
    pub headers: HeaderMap,
    /// Vide pour une redirection : son corps n'est pas lu.
    pub body: String,
}

impl Answer {
    fn location(&self) -> Option<&str> {
        if !(300..400).contains(&self.status) {
            return None;
        }
        self.headers.get(LOCATION).and_then(|v| v.to_str().ok()).map(str::trim)
    }
}

/// Le bout de la chaîne.
#[derive(Debug, Clone)]
pub struct Followed {
    /// Les redirections traversées, dans l'ordre ; vide sans redirection.
    pub chain: Vec<Hop>,
    pub final_url: Url,
    pub answer: Answer,
    /// Temps passé en requêtes, toutes étapes comprises.
    pub elapsed: Duration,
}

/// Ce qui empêche d'arriver au bout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FollowError {
    Timeout,
    Network(String),
    /// Garde-fou : adresse privée sans autorisation.
    Refused(String),
    /// Une adresse revient : la chaîne, adresse répétée comprise.
    Loop(Vec<String>),
    /// Plus de `max` redirections.
    TooMany {
        max: usize,
    },
    /// Un `Location` qui ne se lit pas comme une adresse web.
    BadLocation(String),
}

impl FollowError {
    pub fn message(&self) -> String {
        match self {
            Self::Timeout => format!("no answer within {}s", REQUEST_TIMEOUT.as_secs()),
            Self::Network(message) | Self::Refused(message) => message.clone(),
            Self::Loop(chain) => format!("redirect loop: {}", chain.join(" → ")),
            Self::TooMany { max } => format!("more than {max} redirects"),
            Self::BadLocation(location) => {
                format!("redirect to \"{location}\", which is not a web address")
            }
        }
    }
}

impl From<FollowError> for ProbeError {
    fn from(error: FollowError) -> Self {
        match error {
            FollowError::Timeout => ProbeError::Timeout(REQUEST_TIMEOUT),
            FollowError::Refused(message) => ProbeError::Config(message),
            other => ProbeError::Unreachable(other.message()),
        }
    }
}

/// Suit les redirections depuis `start`, au plus `max_redirects`, en
/// demandant chaque étape à `fetch`. Séparé du réseau pour être éprouvé sans
/// lui.
pub async fn follow<F, Fut>(
    start: Url,
    max_redirects: usize,
    mut fetch: F,
) -> Result<Followed, FollowError>
where
    F: FnMut(Url) -> Fut,
    Fut: Future<Output = Result<Answer, FollowError>>,
{
    let started = Instant::now();
    let mut chain: Vec<Hop> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut url = start;
    loop {
        seen.insert(url.to_string());
        let answer = fetch(url.clone()).await?;
        let Some(location) = answer.location() else {
            return Ok(Followed { chain, final_url: url, answer, elapsed: started.elapsed() });
        };
        let mut next = url
            .join(location)
            .ok()
            .filter(|next| matches!(next.scheme(), "http" | "https"))
            .ok_or_else(|| FollowError::BadLocation(location.to_string()))?;
        next.set_fragment(None);
        chain.push(Hop { url: url.to_string(), status: answer.status });
        if seen.contains(next.as_str()) {
            let mut urls: Vec<String> = chain.into_iter().map(|hop| hop.url).collect();
            urls.push(next.to_string());
            return Err(FollowError::Loop(urls));
        }
        if chain.len() > max_redirects {
            return Err(FollowError::TooMany { max: max_redirects });
        }
        url = next;
    }
}

/// Clients HTTP gardés d'une sonde à l'autre, un par réglage : en construire
/// un recharge les racines TLS, ce qui coûte, et l'on surveille parfois des
/// centaines de sites.
#[derive(Default)]
pub struct Clients {
    cache: Mutex<HashMap<(bool, String), reqwest::Client>>,
}

impl Clients {
    pub fn get(
        &self,
        allow_private: bool,
        user_agent: Option<&str>,
    ) -> Result<reqwest::Client, ProbeError> {
        let agent = user_agent.unwrap_or(USER_AGENT).to_string();
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(client) = cache.get(&(allow_private, agent.clone())) {
            return Ok(client.clone());
        }
        let client = reqwest::Client::builder()
            .dns_resolver(Arc::new(guard::GuardedResolver { allow_private }))
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(dumbmonit_collectors::http::CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .user_agent(agent.as_str())
            .build()
            .map_err(|error| ProbeError::Config(format!("HTTP client unavailable: {error}")))?;
        // Un réglage de plus par agent distinct : la table reste petite, sauf
        // abus, qu'on borne en repartant de zéro.
        if cache.len() > 64 {
            cache.clear();
        }
        cache.insert((allow_private, agent), client.clone());
        Ok(client)
    }
}

/// Une requête GET, sans suivre de redirection. Le corps n'est lu que pour
/// une réponse qui n'en est pas une, jusqu'à `max` octets.
pub async fn get(
    client: &reqwest::Client,
    url: &Url,
    allow_private: bool,
    max: usize,
) -> Result<Answer, FollowError> {
    guard::vet_url(url, allow_private, Duration::from_secs(5)).await.map_err(|error| {
        FollowError::Refused(match error {
            ProbeError::Config(message) | ProbeError::Unreachable(message) => message,
            other => other.to_string(),
        })
    })?;
    let mut response = client
        .get(url.clone())
        .header(reqwest::header::ACCEPT, "text/html,application/xhtml+xml;q=0.9,*/*;q=0.5")
        .send()
        .await
        .map_err(classify)?;
    let status = response.status().as_u16();
    let headers = response.headers().clone();
    let mut answer = Answer { status, headers, body: String::new() };
    if answer.location().is_some() {
        return Ok(answer);
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(classify)? {
        body.extend_from_slice(&chunk);
        if body.len() >= max {
            body.truncate(max);
            break;
        }
    }
    answer.body = String::from_utf8_lossy(&body).into_owned();
    Ok(answer)
}

/// Lit `start` jusqu'au bout de ses redirections.
pub async fn fetch(
    client: &reqwest::Client,
    start: &Url,
    max_redirects: usize,
    allow_private: bool,
) -> Result<Followed, FollowError> {
    follow(start.clone(), max_redirects, |url| async move {
        get(client, &url, allow_private, MAX_BODY_BYTES).await
    })
    .await
}

/// Message lisible d'une erreur `reqwest`, causes comprises : « error sending
/// request » seul ne dit rien (un certificat refusé se lit dans les causes).
fn classify(error: reqwest::Error) -> FollowError {
    if error.is_timeout() {
        return FollowError::Timeout;
    }
    let mut message = error.to_string();
    let mut source = std::error::Error::source(&error);
    while let Some(cause) = source {
        let text = cause.to_string();
        if !message.contains(&text) {
            message.push_str(": ");
            message.push_str(&text);
        }
        source = cause.source();
    }
    FollowError::Network(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::HeaderValue;

    /// Un faux Web : adresse → (statut, `Location`).
    fn web(routes: &[(&str, u16, Option<&str>)]) -> HashMap<String, (u16, Option<String>)> {
        routes
            .iter()
            .map(|(url, status, location)| {
                (url.to_string(), (*status, location.map(str::to_string)))
            })
            .collect()
    }

    async fn run(
        routes: &[(&str, u16, Option<&str>)],
        start: &str,
        max: usize,
    ) -> Result<Followed, FollowError> {
        let web = web(routes);
        follow(Url::parse(start).unwrap(), max, |url| {
            let found = web.get(url.as_str()).cloned();
            async move {
                let (status, location) =
                    found.ok_or_else(|| FollowError::Network(format!("no route to {url}")))?;
                let mut headers = HeaderMap::new();
                if let Some(location) = location {
                    headers.insert(LOCATION, HeaderValue::from_str(&location).unwrap());
                }
                Ok(Answer { status, headers, body: "<html>ok</html>".into() })
            }
        })
        .await
    }

    #[tokio::test]
    async fn http_vers_https_puis_domaine_nu_vers_www_n_est_pas_une_erreur() {
        let followed = run(
            &[
                ("http://example.com/", 301, Some("https://example.com/")),
                ("https://example.com/", 308, Some("https://www.example.com/")),
                ("https://www.example.com/", 200, None),
            ],
            "http://example.com/",
            10,
        )
        .await
        .unwrap();
        assert_eq!(followed.final_url.as_str(), "https://www.example.com/");
        assert_eq!(followed.answer.status, 200);
        assert_eq!(
            followed.chain,
            [
                Hop { url: "http://example.com/".into(), status: 301 },
                Hop { url: "https://example.com/".into(), status: 308 },
            ]
        );
    }

    #[tokio::test]
    async fn sans_redirection_la_chaine_est_vide() {
        let followed = run(&[("https://ex.fr/", 200, None)], "https://ex.fr/", 10).await.unwrap();
        assert!(followed.chain.is_empty());
        assert_eq!(followed.final_url.as_str(), "https://ex.fr/");
    }

    #[tokio::test]
    async fn redirection_relative_resolue() {
        let followed = run(
            &[
                ("https://ex.fr/a/b", 302, Some("../c?x=1#frag")),
                ("https://ex.fr/c?x=1", 200, None),
            ],
            "https://ex.fr/a/b",
            10,
        )
        .await
        .unwrap();
        assert_eq!(followed.final_url.as_str(), "https://ex.fr/c?x=1");
    }

    #[tokio::test]
    async fn une_boucle_est_reconnue() {
        let error = run(
            &[
                ("https://ex.fr/a", 302, Some("/b")),
                ("https://ex.fr/b", 302, Some("http://ex.fr/a")),
                ("http://ex.fr/a", 301, Some("https://ex.fr/a")),
            ],
            "https://ex.fr/a",
            10,
        )
        .await
        .unwrap_err();
        assert_eq!(
            error,
            FollowError::Loop(vec![
                "https://ex.fr/a".into(),
                "https://ex.fr/b".into(),
                "http://ex.fr/a".into(),
                "https://ex.fr/a".into(),
            ])
        );
        assert!(error.message().starts_with("redirect loop: https://ex.fr/a → "));
        assert!(matches!(ProbeError::from(error), ProbeError::Unreachable(_)));
    }

    #[tokio::test]
    async fn trop_de_redirections() {
        let routes: Vec<(String, u16, Option<String>)> = (0..6)
            .map(|i| (format!("https://ex.fr/{i}"), 301, Some(format!("/{}", i + 1))))
            .collect();
        let routes: Vec<(&str, u16, Option<&str>)> =
            routes.iter().map(|(u, s, l)| (u.as_str(), *s, l.as_deref())).collect();
        let error = run(&routes, "https://ex.fr/0", 3).await.unwrap_err();
        assert_eq!(error, FollowError::TooMany { max: 3 });
        // Trois redirections permises, trois suivies : c'est la limite, pas
        // une erreur.
        let mut ok = routes[..3].to_vec();
        ok.push(("https://ex.fr/3", 200, None));
        let followed = run(&ok, "https://ex.fr/0", 3).await.unwrap();
        assert_eq!(followed.chain.len(), 3);
    }

    #[tokio::test]
    async fn redirection_sans_location_ou_vers_autre_chose_que_le_web() {
        let followed = run(&[("https://ex.fr/", 304, None)], "https://ex.fr/", 10).await.unwrap();
        assert_eq!(followed.answer.status, 304);
        let error = run(&[("https://ex.fr/", 302, Some("mailto:a@b.c"))], "https://ex.fr/", 10)
            .await
            .unwrap_err();
        assert!(matches!(error, FollowError::BadLocation(_)));
    }
}
