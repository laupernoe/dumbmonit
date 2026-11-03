//! Le cookie de session, et rien d'autre.
//!
//! Aucune bibliothèque de cookies n'est tirée pour si peu : nous n'en posons qu'un,
//! dont nous choisissons chaque attribut.

use axum::http::{HeaderMap, HeaderValue, header};

use crate::auth::session::SessionToken;

/// Nom du cookie. Préfixé par le produit pour ne pas entrer en collision avec un
/// autre service hébergé sur le même domaine, et par `__Host-` : le navigateur
/// ne l'accepte alors que `Secure`, sur `Path=/` et sans `Domain` — un autre
/// service du même domaine (ou un sous-domaine en clair) ne peut ni le poser
/// ni l'écraser. Le port ne sert que HTTPS : c'est le seul nom.
pub const NAME: &str = "__Host-dumbmonit_session";

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
/// `Secure` toujours : le cookie ne part jamais sur un lien `http://`.
pub fn set(token: &SessionToken) -> HeaderValue {
    header_value(format!(
        "{NAME}={}; Path=/; HttpOnly; SameSite=Strict; Max-Age={MAX_AGE_SECS}; Secure",
        token.cookie_value()
    ))
}

/// Construit l'en-tête qui efface le cookie côté navigateur.
///
/// Les attributs doivent être identiques à ceux de la pose, sans quoi certains
/// navigateurs conservent le cookie d'origine à côté de celui qui l'annule.
pub fn clear() -> HeaderValue {
    header_value(format!("{NAME}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0; Secure"))
}

/// Ajoute `Secure` à tout cookie posé qui ne l'aurait pas encore : le port ne
/// sert que HTTPS, un cookie ne doit jamais pouvoir repartir sur un lien
/// `http://` (lien mixte, rétrogradation sur un Wi-Fi hostile).
pub async fn always_secure(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    let mut upgraded: Vec<HeaderValue> = Vec::new();
    let mut changed = false;
    for value in headers.get_all(header::SET_COOKIE) {
        match value.to_str() {
            Ok(text) if !has_secure(text) => {
                changed = true;
                let text = format!("{text}; Secure");
                upgraded.push(HeaderValue::from_str(&text).unwrap_or(value.clone()));
            }
            _ => upgraded.push(value.clone()),
        }
    }
    if changed {
        headers.remove(header::SET_COOKIE);
        for value in upgraded {
            headers.append(header::SET_COOKIE, value);
        }
    }
    response
}

/// L'attribut `Secure` est-il déjà là ?
fn has_secure(cookie: &str) -> bool {
    cookie.split(';').skip(1).any(|attribute| attribute.trim().eq_ignore_ascii_case("secure"))
}

fn header_value(value: String) -> HeaderValue {
    // La valeur n'est faite que d'hexadécimal et de ponctuation ASCII : la
    // conversion ne peut pas échouer.
    HeaderValue::from_str(&value).expect("en-tête Set-Cookie en ASCII")
}

/// Extrait le jeton de session de l'en-tête `Cookie`, s'il s'y trouve.
pub fn extract(headers: &HeaderMap) -> Option<SessionToken> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    raw.split(';')
        .filter_map(|pair| pair.split_once('='))
        .find(|(name, _)| name.trim() == NAME)
        .and_then(|(_, value)| SessionToken::parse(value.trim()))
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
        let headers = headers_with_cookie("theme=dark; __Host-dumbmonit_session=abc.def; lang=fr");
        let token = extract(&headers).expect("jeton présent");
        assert_eq!(token.cookie_value(), "abc.def");
    }

    #[test]
    fn an_absent_or_unusable_cookie_yields_nothing() {
        assert!(extract(&HeaderMap::new()).is_none());
        assert!(extract(&headers_with_cookie("theme=dark")).is_none());
        assert!(extract(&headers_with_cookie("__Host-dumbmonit_session=sans-point")).is_none());
        // L'ancien nom, sans préfixe, n'ouvre plus rien.
        assert!(extract(&headers_with_cookie("dumbmonit_session=abc.def")).is_none());
    }

    #[test]
    fn the_attributes_match_the_intent() {
        let headers = headers_with_cookie("__Host-dumbmonit_session=abc.def");
        let token = extract(&headers).unwrap();

        let posed = set(&token).to_str().unwrap().to_string();
        assert!(posed.starts_with("__Host-dumbmonit_session=abc.def;"), "{posed}");
        assert!(posed.contains("HttpOnly"), "{posed}");
        assert!(posed.contains("SameSite=Strict"), "{posed}");
        assert!(posed.contains("Path=/"), "{posed}");
        assert!(posed.contains("Max-Age=2592000"), "{posed}");
        assert!(posed.contains("; Secure"), "{posed}");
        assert!(!posed.contains("Domain="), "{posed}");

        let cleared = clear().to_str().unwrap().to_string();
        assert!(cleared.contains("Max-Age=0") && cleared.contains("; Secure"), "{cleared}");
    }

    #[test]
    fn secure_is_recognised_whatever_its_case_and_place() {
        assert!(has_secure("a=b; Path=/; secure"));
        assert!(has_secure("a=b; Secure; HttpOnly"));
        assert!(!has_secure("a=b; Path=/; HttpOnly"));
        // Une valeur qui contient le mot n'est pas l'attribut.
        assert!(!has_secure("secure=1; Path=/"));
    }
}
