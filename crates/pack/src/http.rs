//! Les requêtes d'un paquet : une origine, un garde-fou, un plafond.
//!
//! L'URL de chaque requête est la racine de la cible suivie du chemin déclaré ;
//! elle est vérifiée *après* substitution des gabarits, et doit garder le
//! protocole, l'hôte et le port de la racine. Les redirections sont suivies à la
//! main, et seulement vers cette même origine. Le garde-fou d'adresses est
//! appliqué deux fois, comme pour les sondes HTTP : avant l'envoi (adresses
//! littérales) et à la résolution (noms, y compris ceux qui changeraient de
//! réponse entre les deux).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use dumbmonit_collectors::uptime::guard;
use dumbmonit_proto::{Credential, ProbeError};
use reqwest::header::{HeaderName, HeaderValue, LOCATION};
use reqwest::redirect::Policy;
use reqwest::{Client, StatusCode, Url};

use crate::MAX_BODY_BYTES;
use crate::compile::Source;
use crate::manifest::{Auth, Method};
use crate::template::{Context, Encoding};

/// Redirections suivies au plus pour une même source.
const MAX_REDIRECTS: usize = 3;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Un client par politique TLS et par garde-fou : ces deux réglages se figent à
/// la construction. Le pool de connexions est partagé par tous les paquets.
pub(crate) fn client(insecure_tls: bool, allow_private: bool) -> Result<Client, ProbeError> {
    static CLIENTS: OnceLock<Mutex<HashMap<(bool, bool), Client>>> = OnceLock::new();
    let mut cache =
        CLIENTS.get_or_init(Default::default).lock().unwrap_or_else(|poison| poison.into_inner());
    if let Some(existing) = cache.get(&(insecure_tls, allow_private)) {
        return Ok(existing.clone());
    }
    let client = Client::builder()
        .danger_accept_invalid_certs(insecure_tls)
        .redirect(Policy::none())
        .dns_resolver(Arc::new(guard::GuardedResolver { allow_private }))
        .connect_timeout(CONNECT_TIMEOUT)
        .pool_max_idle_per_host(2)
        .user_agent(concat!("DumbMonit/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| ProbeError::Config(format!("HTTP client unavailable: {error}")))?;
    cache.insert((insecure_tls, allow_private), client.clone());
    Ok(client)
}

/// Ce qu'il faut pour interroger une cible.
pub(crate) struct Request<'a> {
    pub client: &'a Client,
    pub base: &'a Url,
    pub context: &'a Context<'a>,
    pub allow_private: bool,
    pub timeout: Duration,
}

/// Même protocole, même hôte, même port.
pub(crate) fn same_origin(a: &Url, b: &Url) -> bool {
    a.scheme() == b.scheme()
        && a.host_str() == b.host_str()
        && a.port_or_known_default() == b.port_or_known_default()
}

/// La racine suivie du chemin, refusée si elle quitte l'origine.
pub(crate) fn join(base: &Url, path: &str) -> Result<Url, ProbeError> {
    let raw = format!("{}{path}", base.as_str().trim_end_matches('/'));
    let url = Url::parse(&raw)
        .map_err(|error| ProbeError::Config(format!("invalid request path \"{path}\": {error}")))?;
    if !same_origin(&url, base) {
        return Err(ProbeError::Config(format!(
            "the request path \"{path}\" leaves the device's address"
        )));
    }
    Ok(url)
}

fn origin(url: &Url) -> String {
    match url.port() {
        Some(port) => format!("{}://{}:{port}", url.scheme(), url.host_str().unwrap_or("")),
        None => format!("{}://{}", url.scheme(), url.host_str().unwrap_or("")),
    }
}

/// Exécute une source et rend le corps de sa réponse. `budget` compte les
/// requêtes restantes de l'interrogation ; chaque redirection en consomme une.
pub(crate) async fn fetch(
    request: &Request<'_>,
    source: &Source,
    budget: &mut usize,
) -> Result<Vec<u8>, ProbeError> {
    let path = source.path.render(request.context, Encoding::Url)?;
    let mut url = join(request.base, &path)?;
    let mut method = match source.method {
        Method::Get => reqwest::Method::GET,
        Method::Post => reqwest::Method::POST,
    };
    let mut with_body = true;

    for _ in 0..=MAX_REDIRECTS {
        if *budget == 0 {
            return Err(ProbeError::Config(format!(
                "more than {} requests in one probe",
                crate::MAX_REQUESTS
            )));
        }
        *budget -= 1;
        guard::vet_url(&url, request.allow_private, request.timeout).await?;

        let mut builder =
            request.client.request(method.clone(), url.clone()).timeout(request.timeout);
        for (name, template) in &source.headers {
            let value = template.render(request.context, Encoding::Raw)?;
            let name = HeaderName::from_bytes(name.as_bytes())
                .map_err(|_| ProbeError::Config(format!("invalid header name \"{name}\"")))?;
            let mut value = HeaderValue::from_str(&value).map_err(|_| {
                // La valeur n'est pas recopiée : elle peut contenir un secret.
                ProbeError::Config(format!("the value of header \"{name}\" is not valid"))
            })?;
            if template.uses_credential() {
                value.set_sensitive(true);
            }
            builder = builder.header(name, value);
        }
        builder = authenticate(builder, source.auth, request.context.credential);
        if with_body && let Some(body) = &source.body {
            builder = builder.body(body.render(request.context, Encoding::Raw)?);
        }

        let mut response =
            builder.send().await.map_err(|error| classify(error, request.timeout))?;
        let status = response.status();
        if status.is_redirection() {
            let next = response
                .headers()
                .get(LOCATION)
                .and_then(|location| location.to_str().ok())
                .and_then(|location| url.join(location).ok())
                .ok_or_else(|| {
                    ProbeError::Protocol(format!(
                        "HTTP {} without a usable Location",
                        status.as_u16()
                    ))
                })?;
            if !same_origin(&next, request.base) {
                return Err(ProbeError::Config(format!(
                    "redirect to {} refused: a pack only talks to the device's own address",
                    origin(&next)
                )));
            }
            if matches!(
                status,
                StatusCode::MOVED_PERMANENTLY | StatusCode::FOUND | StatusCode::SEE_OTHER
            ) {
                method = reqwest::Method::GET;
                with_body = false;
            }
            url = next;
            continue;
        }
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            return Err(ProbeError::Auth(format!("HTTP {} on {}", status.as_u16(), url.path())));
        }
        if !status.is_success() {
            return Err(ProbeError::Protocol(format!(
                "HTTP {} on {}",
                status.as_u16(),
                url.path()
            )));
        }
        return read_body(&mut response, request.timeout).await;
    }
    Err(ProbeError::Protocol(format!(
        "more than {MAX_REDIRECTS} redirects for source \"{}\"",
        source.id
    )))
}

fn authenticate(
    builder: reqwest::RequestBuilder,
    auth: Auth,
    credential: &Credential,
) -> reqwest::RequestBuilder {
    match (auth, credential) {
        (Auth::Auto | Auth::Bearer, Credential::ApiToken { token }) => builder.bearer_auth(token),
        (Auth::Auto | Auth::Basic, Credential::UsernamePassword { username, password }) => {
            builder.basic_auth(username, Some(password))
        }
        _ => builder,
    }
}

async fn read_body(
    response: &mut reqwest::Response,
    timeout: Duration,
) -> Result<Vec<u8>, ProbeError> {
    let too_big = || {
        ProbeError::Protocol(format!("response larger than {} MiB", MAX_BODY_BYTES / (1024 * 1024)))
    };
    if response.content_length().is_some_and(|length| length > MAX_BODY_BYTES as u64) {
        return Err(too_big());
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|error| classify(error, timeout))? {
        if body.len() + chunk.len() > MAX_BODY_BYTES {
            return Err(too_big());
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

/// Traduit un échec de transport. Un refus du garde-fou, remonté par le
/// résolveur, reste une erreur de configuration : ce n'est pas une panne.
fn classify(error: reqwest::Error, timeout: Duration) -> ProbeError {
    let mut cause: Option<&(dyn std::error::Error + 'static)> = Some(&error);
    while let Some(current) = cause {
        if let Some(ProbeError::Config(message)) = current.downcast_ref::<ProbeError>() {
            return ProbeError::Config(message.clone());
        }
        cause = current.source();
    }
    if error.is_timeout() {
        return ProbeError::Timeout(timeout);
    }
    let connect = error.is_connect();
    // La chaîne des causes, sans l'URL : c'est la dernière qui dit ce qui s'est
    // passé (connexion refusée, nom inconnu, certificat).
    let error = error.without_url();
    let mut message = error.to_string();
    let mut cause = std::error::Error::source(&error);
    while let Some(current) = cause {
        message.push_str(": ");
        message.push_str(&current.to_string());
        cause = current.source();
    }
    if connect { ProbeError::Unreachable(message) } else { ProbeError::Protocol(message) }
}
