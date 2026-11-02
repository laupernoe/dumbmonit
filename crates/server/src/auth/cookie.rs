//! Le cookie de session, et rien d'autre.
//!
//! Aucune bibliothèque de cookies n'est tirée pour si peu : nous n'en posons qu'un,
//! dont nous choisissons chaque attribut.

use axum::http::{HeaderMap, HeaderValue, header};

use crate::auth::session::SessionToken;

/// Nom du cookie. Préfixé par le produit pour ne pas entrer en collision avec un
/// autre service hébergé sur le même domaine.
pub const NAME: &str = "dumbmonit_session";

/// Nom du cookie en HTTPS. Le préfixe `__Host-` oblige le navigateur à ne
/// l'accepter que `Secure`, sur `Path=/` et sans `Domain` : un autre service du
/// même domaine (ou un sous-domaine en clair) ne peut ni le poser ni
/// l'écraser.
pub const HOST_NAME: &str = "__Host-dumbmonit_session";

/// Trente jours, en accord avec la durée de vie de la session en base. Le
/// navigateur oublie le cookie au moment même où le serveur oublie la session.
const MAX_AGE_SECS: i64 = 30 * 24 * 60 * 60;

/// Construit l'en-tête `Set-Cookie` d'une session ouverte.
///
/// `HttpOnly` met le jeton hors de portée de tout JavaScript, donc d'une éventuelle
/// injection dans l'interface. `SameSite=Strict` : le cookie ne part sur aucune
/// requête née ailleurs, pas même une navigation GET depuis un lien tiers.
/// L'interface est une application monopage — la page elle-même se charge sans
/// session, et ce sont ses propres appels à `/api` qui portent le cookie — si
/// bien que rien ne casse, retour d'une connexion OIDC ou SAML compris : la
/// réponse du fournisseur pose le cookie, l'interface s'en sert ensuite.
pub fn set(token: &SessionToken, secure: bool) -> HeaderValue {
    let mut value = format!(
        "{NAME}={}; Path=/; HttpOnly; SameSite=Strict; Max-Age={MAX_AGE_SECS}",
        token.cookie_value()
    );
    if secure {
        value.push_str("; Secure");
    }
    header_value(value)
}

/// Construit l'en-tête qui efface le cookie côté navigateur.
///
/// Les attributs doivent être identiques à ceux de la pose, sans quoi certains
/// navigateurs conservent le cookie d'origine à côté de celui qui l'annule.
pub fn clear(secure: bool) -> HeaderValue {
    let mut value = format!("{NAME}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0");
    if secure {
        value.push_str("; Secure");
    }
    header_value(value)
}

/// Ajoute `Secure` aux cookies posés pour un navigateur qui nous joint en HTTPS.
///
/// `DUMBMONIT_COOKIE_SECURE` reste éteint par défaut, parce qu'un cookie
/// `Secure` n'est jamais renvoyé en HTTP simple et rendrait la connexion
/// impossible sur un réseau local. Mais quand la requête montre qu'elle vient
/// d'une page HTTPS, l'attribut ne coûte rien et empêche le navigateur de
/// renvoyer la session de trente jours sur un lien `http://` (lien mixte,
/// rétrogradation sur un Wi-Fi hostile). Les indices retenus :
///
/// - `X-Forwarded-Proto: https`, seulement si la connexion vient d'un
///   mandataire de confiance (`DUMBMONIT_TRUSTED_PROXIES`) ;
/// - un en-tête `Origin` en `https://`, que le navigateur pose lui-même sur le
///   `POST` de connexion. Le falsifier ne sert à rien : il ne ferait que durcir
///   le cookie de celui qui le falsifie.
///
/// En HTTPS, le cookie de session prend en plus le préfixe `__Host-`
/// ([`HOST_NAME`]), et l'ancien nom est effacé dans la même réponse.
pub async fn secure_when_https(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let https = request_is_https(&request);
    let mut response = next.run(request).await;
    if https {
        let headers = response.headers_mut();
        let mut upgraded: Vec<HeaderValue> = Vec::new();
        for value in headers.get_all(header::SET_COOKIE) {
            let Ok(text) = value.to_str() else {
                upgraded.push(value.clone());
                continue;
            };
            let (text, session) = match text.strip_prefix(&format!("{NAME}=")) {
                Some(rest) => (format!("{HOST_NAME}={rest}"), true),
                None => (text.to_string(), false),
            };
            let text = match text.to_ascii_lowercase().contains("; secure") {
                true => text,
                false => format!("{text}; Secure"),
            };
            upgraded.push(HeaderValue::from_str(&text).unwrap_or(value.clone()));
            if session {
                // Le cookie posé en clair avant le passage en HTTPS disparaît.
                upgraded.push(HeaderValue::from_static(
                    "dumbmonit_session=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0",
                ));
            }
        }
        if !upgraded.is_empty() {
            headers.remove(header::SET_COOKIE);
            for value in upgraded {
                headers.append(header::SET_COOKIE, value);
            }
        }
    }
    response
}

fn request_is_https(request: &axum::extract::Request) -> bool {
    // Connexion chiffrée jusqu'ici, ou mandataire de confiance qui le dit
    // (établi par `web_tls::transport`).
    if request.extensions().get::<crate::web_tls::Transport>().is_some_and(|t| t.https) {
        return true;
    }
    let headers = request.headers();
    let text = |name: &str| headers.get(name).and_then(|value| value.to_str().ok());
    if text("origin").is_some_and(|origin| origin.trim().starts_with("https://")) {
        return true;
    }
    let trusted = request
        .extensions()
        .get::<crate::auth::AuthState>()
        .map(|auth| auth.trusted_proxies().to_vec())
        .unwrap_or_default();
    let peer = request
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|info| info.0.ip());
    peer.is_some_and(|peer| trusted.iter().any(|net| net.contains(&peer)))
        && text("x-forwarded-proto")
            .and_then(|value| value.split(',').next())
            .is_some_and(|proto| proto.trim().eq_ignore_ascii_case("https"))
}

fn header_value(value: String) -> HeaderValue {
    // La valeur n'est faite que d'hexadécimal et de ponctuation ASCII : la
    // conversion ne peut pas échouer.
    HeaderValue::from_str(&value).expect("en-tête Set-Cookie en ASCII")
}

/// Extrait le jeton de session de l'en-tête `Cookie`, s'il s'y trouve : le
/// cookie `__Host-` d'abord, l'ancien nom (session ouverte en HTTP) sinon.
pub fn extract(headers: &HeaderMap) -> Option<SessionToken> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    let find = |wanted: &str| {
        raw.split(';')
            .filter_map(|pair| pair.split_once('='))
            .find(|(name, _)| name.trim() == wanted)
            .and_then(|(_, value)| SessionToken::parse(value.trim()))
    };
    find(HOST_NAME).or_else(|| find(NAME))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers_with_cookie(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, HeaderValue::from_str(value).unwrap());
        headers
    }

    #[test]
    fn the_cookie_is_read_among_others() {
        let headers = headers_with_cookie("theme=dark; dumbmonit_session=abc.def; lang=fr");
        let token = extract(&headers).expect("jeton présent");
        assert_eq!(token.cookie_value(), "abc.def");
    }

    #[test]
    fn an_absent_or_unusable_cookie_yields_nothing() {
        assert!(extract(&HeaderMap::new()).is_none());
        assert!(extract(&headers_with_cookie("theme=dark")).is_none());
        assert!(extract(&headers_with_cookie("dumbmonit_session=sans-point")).is_none());
    }

    #[test]
    fn the_attributes_match_the_intent() {
        let headers = headers_with_cookie("dumbmonit_session=abc.def");
        let token = extract(&headers).unwrap();

        let posed = set(&token, false).to_str().unwrap().to_string();
        assert!(posed.contains("HttpOnly"), "{posed}");
        assert!(posed.contains("SameSite=Strict"), "{posed}");
        assert!(posed.contains("Path=/"), "{posed}");
        assert!(posed.contains("Max-Age=2592000"), "{posed}");
        // Sans TLS déclaré, pas de `Secure` : le produit vit en HTTP sur un LAN.
        assert!(!posed.contains("Secure"), "{posed}");

        assert!(set(&token, true).to_str().unwrap().contains("; Secure"));
        assert!(clear(false).to_str().unwrap().contains("Max-Age=0"));
    }

    #[test]
    fn the_host_prefixed_cookie_wins_over_the_plain_one() {
        let headers =
            headers_with_cookie("dumbmonit_session=old.one; __Host-dumbmonit_session=new.one");
        assert_eq!(extract(&headers).unwrap().cookie_value(), "new.one");
        let headers = headers_with_cookie("__Host-dumbmonit_session=new.one");
        assert_eq!(extract(&headers).unwrap().cookie_value(), "new.one");
    }
}
