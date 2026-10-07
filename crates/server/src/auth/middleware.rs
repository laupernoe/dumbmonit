//! Le garde posé devant les routes d'API qui touchent à l'instance.
//!
//! Il ne protège que `/api/**`. Les fichiers de l'interface restent servis
//! librement : c'est l'application elle-même qui affiche l'écran de connexion, et
//! elle doit donc pouvoir se charger avant que l'on sache qui la consulte.
//!
//! Il applique aussi la règle des rôles, en un seul endroit : toute écriture
//! (POST, PUT, DELETE) exige un administrateur, sauf ce que chacun fait pour
//! lui-même — se déconnecter, changer son mot de passe — et le traitement des
//! alertes, ouvert aussi aux opérateurs ([`OPERATOR_ROUTES`], liste fermée :
//! une route d'écriture qui n'y figure pas reste réservée à l'administrateur).
//! Les lectures sont ouvertes aux trois rôles ; les quelques lectures réservées
//! (liste des comptes, réglages SSO) le disent elles-mêmes avec l'extracteur
//! [`AdminUser`].
//!
//! Deux façons de se présenter : le cookie de session d'un navigateur, ou un
//! jeton d'API (`Authorization: Bearer dmt_…`) pour les scripts et les
//! assistants. Un jeton `read` vaut un lecteur, un jeton `write` vaut un
//! administrateur — à ceci près qu'un jeton ne gère jamais les comptes, les
//! sessions, le second facteur, le SSO ni les autres jetons ([`TOKEN_DENIED`]) :
//! ce sont des gestes que l'on fait soi-même, dans l'interface. Une requête
//! portée par un jeton n'a pas de cookie, donc pas de CSRF possible : la preuve
//! d'origine n'est exigée que des sessions.

use std::net::IpAddr;

use axum::extract::{Extension, FromRequestParts, Request, State};
use axum::http::request::Parts;
use axum::http::{HeaderMap, Method};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use sqlx::SqlitePool;

use crate::auth::session::SessionToken;
use crate::auth::token::{self, ApiToken, Scope};
use crate::auth::users::{self, User};
use crate::auth::{AuthError, AuthState, cookie, session};
use crate::state::AppState;

/// Session validée, déposée dans la requête pour les gestionnaires qui en ont
/// besoin (déconnexion, changement de mot de passe) : elle a déjà été vérifiée
/// contre la base, inutile de recommencer.
#[derive(Clone)]
pub struct CurrentSession(pub SessionToken);

/// Compte de la session, déposé de la même façon. Absent quand la requête est
/// portée par un jeton d'API : voir [`CurrentPrincipal`].
#[derive(Clone)]
pub struct CurrentUser(pub User);

/// Qui fait la requête : un compte connecté par session, ou un jeton d'API.
#[derive(Debug, Clone)]
pub enum Principal {
    User(User),
    Token(ApiToken),
}

impl Principal {
    /// Nom à inscrire dans les journaux : l'identifiant du compte, ou le nom du
    /// jeton précédé de `token:` pour qu'on ne le confonde pas avec un compte.
    pub fn label(&self) -> String {
        match self {
            Self::User(user) => user.username.clone(),
            Self::Token(token) => format!("token:{}", token.name),
        }
    }

    /// Un administrateur, ou un jeton `write` — ce qui revient au même.
    pub fn is_admin(&self) -> bool {
        match self {
            Self::User(user) => user.role.is_admin(),
            Self::Token(token) => token.scope.allows(Scope::Write),
        }
    }

    /// Peut traiter les alertes : administrateur ou opérateur, ou jeton `write`.
    /// Un jeton `read` ne le peut pas, quel que soit son propriétaire.
    pub fn can_operate(&self) -> bool {
        match self {
            Self::User(user) => user.role.can_operate(),
            Self::Token(token) => token.scope.allows(Scope::Write),
        }
    }
}

/// L'identité de la requête, quel qu'en soit le porteur. Toujours déposée par le
/// garde, là où [`CurrentUser`] et [`CurrentSession`] ne le sont que pour une
/// session.
#[derive(Clone)]
pub struct CurrentPrincipal(pub Principal);

/// Écritures que chacun peut faire sans être admin : sur son propre compte, et
/// les deux gestes d'un écran mural, qui tourne volontiers sous un compte
/// lecteur — rapporter l'état de son enceinte Spotify, et faire jouer le compte
/// Spotify dessus (« Play here »). Ce second geste n'ouvre rien de neuf : le
/// mur d'un lecteur reçoit déjà un jeton d'accès Spotify qui le permet
/// (`/music/spotify/token`). Ces deux routes refusent les jetons d'API.
const SELF_SERVICE: &[&str] = &[
    "/auth/logout",
    "/auth/password",
    "/auth/totp",
    "/auth/totp/enroll",
    "/auth/totp/verify",
    "/music/speaker/report",
    "/music/speaker/play",
];

/// Écritures ouvertes au rôle `operator`, en plus de [`SELF_SERVICE`] : le
/// traitement des alertes, rien de la configuration. Chaque entrée est une
/// méthode et un gabarit de chemin (sans le préfixe `/api`) où `*` vaut un
/// segment quelconque, non vide.
///
/// Liste fermée, refus par défaut : une route ajoutée demain à `api/mod.rs`
/// reste réservée à l'administrateur tant qu'on ne l'a pas inscrite ici. Un
/// gabarit ne doit couvrir que des routes de traitement d'alerte — y compris
/// celles qu'axum ferait correspondre au même chemin.
///
/// Les surcharges de règle par équipement (`/alerts/rules/*/overrides/*`) ne
/// sont ouvertes qu'en partie : le gestionnaire n'y accepte d'un opérateur que
/// le geste « ignorer » (`enabled: false`, sans seuil) et son annulation — voir
/// `api::alerts::put_override`.
pub const OPERATOR_ROUTES: &[(&str, &str)] = &[
    ("POST", "/alerts/*/ack"),
    ("DELETE", "/alerts/*/ack"),
    ("POST", "/alerts/silences"),
    ("DELETE", "/alerts/silences/*"),
    ("POST", "/alerts/history/dismiss-resolved"),
    ("POST", "/alerts/history/*/dismiss"),
    ("PUT", "/alerts/rules/*/overrides/*"),
    ("DELETE", "/alerts/rules/*/overrides/*"),
];

/// Préfixes de routes interdits aux jetons d'API, quelle que soit leur portée :
/// comptes et sessions (`/auth/**`, journal d'audit compris), gestion des
/// comptes (`/users/**`), jetons d'API et d'agents. Un jeton ne fabrique pas
/// d'autres secrets et ne touche pas à qui peut se connecter.
pub const TOKEN_DENIED: &[&str] = &["/auth", "/users", "/tokens", "/agent/tokens"];

const TOKEN_DENIED_MESSAGE: &str = "API tokens cannot manage accounts, sessions, two-factor \
                                    authentication, SSO or other tokens: sign in to the web \
                                    interface for that.";

/// En-tête que l'interface pose sur chaque écriture, et sa valeur attendue.
///
/// Un navigateur n'ajoute jamais un en-tête de ce nom à une requête tierce sans
/// l'accord CORS de ce serveur — qui ne l'accorde à personne. Sa présence prouve
/// donc que la requête vient de notre propre origine, ou d'un script qui a le
/// jeton de session sous la main : c'est la protection contre le CSRF, en
/// complément de `SameSite=Lax` sur le cookie.
pub const CSRF_HEADER: &str = "x-requested-with";
pub const CSRF_VALUE: &str = "DumbMonit";

/// Exige une session valide.
///
/// Tant qu'aucun compte n'existe, il ne peut pas y avoir de session : tout ce qui
/// est sous ce garde répond 401. L'interface n'a besoin, à ce stade, que des
/// routes publiques (`/auth/status`, `/auth/setup`, `/auth/login`) pour proposer
/// la création du premier compte. Ouvrir davantage laisserait quiconque joint le
/// port préparer l'instance — jetons, équipements, canaux — avant son propriétaire,
/// et ces préparatifs survivraient à la création du compte.
pub async fn require_session(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthState>,
    mut request: Request,
    next: Next,
) -> Response {
    auth.purge_once(&state.pool).await;

    match auth.is_configured(&state.pool).await {
        Err(error) => return error.into_response(),
        Ok(false) => {
            return AuthError::Unauthorized(
                "No account exists yet: create the first admin account, then sign in.".into(),
            )
            .into_response();
        }
        Ok(true) => {}
    }

    // Un jeton d'API présenté prime : on ne retombe pas sur le cookie s'il est
    // refusé, sinon un jeton révoqué dans un navigateur connecté passerait
    // inaperçu.
    if token::extract_bearer(request.headers()).is_some() {
        let method = request.method().clone();
        let path = request.uri().path().to_string();
        // Le routeur `/api` est imbriqué : le chemin vu ici a perdu son préfixe.
        // Le journal porte le chemin complet, sans chaîne de requête.
        let full_path = request
            .extensions()
            .get::<axum::extract::OriginalUri>()
            .map_or_else(|| path.clone(), |uri| uri.path().to_string());
        let ip = token::request_ip(request.extensions(), request.headers());
        let checked = match bearer_identity(&state, request.headers(), ip, &method, &path).await {
            Ok(checked) => checked,
            Err(response) => return *response,
        };
        let api_token = checked.token.clone();
        request.extensions_mut().insert(CurrentPrincipal(Principal::Token(checked.token)));
        let mut response = next.run(request).await;
        checked.rate.apply(response.headers_mut());
        if is_mutation(&method) {
            // Toute écriture faite par un jeton laisse une trace : qui (le nom et
            // l'identifiant du jeton, jamais le secret), quoi (méthode et chemin,
            // sans la chaîne de requête ni le corps), et le résultat.
            tracing::info!(
                token = %api_token.name,
                token_id = api_token.id,
                %method,
                path = %full_path,
                status = response.status().as_u16(),
                ip = ?ip,
                "API write by token"
            );
        }
        return response;
    }

    let (token, user) = match current_session(&state.pool, request.headers()).await {
        Ok(Some(found)) => found,
        Ok(None) => {
            return AuthError::Unauthorized("Authentication required.".into()).into_response();
        }
        Err(error) => return error.into_response(),
    };

    if is_mutation(request.method()) {
        if let Err(reason) = same_origin(request.headers()) {
            return AuthError::Forbidden(reason.into()).into_response();
        }
        let path = request.uri().path();
        let allowed = user.role.is_admin()
            || is_self_service(path)
            || (user.role.can_operate() && is_operator_route(request.method(), path));
        if !allowed {
            return AuthError::admin_required().into_response();
        }
    }

    request.extensions_mut().insert(CurrentSession(token));
    request.extensions_mut().insert(CurrentPrincipal(Principal::User(user.clone())));
    request.extensions_mut().insert(CurrentUser(user));
    next.run(request).await
}

/// Vérifie le jeton d'API d'une requête et ce qu'il a le droit de faire ici.
///
/// Jeton inconnu ou révoqué : 401 ; route interdite aux jetons : 403 ; écriture
/// avec un jeton `read` : 403 ; jeton qui boucle : 429 — les mêmes réponses que
/// le point d'entrée MCP, qui passe par la même vérification.
async fn bearer_identity(
    state: &AppState,
    headers: &HeaderMap,
    ip: Option<IpAddr>,
    method: &Method,
    path: &str,
) -> Result<token::Checked, Box<Response>> {
    let checked = token::check(&state.pool, headers, ip, Scope::Read)
        .await
        .map_err(|error| Box::new(error.into_response()))?;
    let refuse = |error: AuthError| {
        let mut response = error.into_response();
        checked.rate.apply(response.headers_mut());
        Box::new(response)
    };
    if is_token_denied(path) {
        return Err(refuse(AuthError::Forbidden(TOKEN_DENIED_MESSAGE.into())));
    }
    if is_mutation(method) {
        checked
            .token
            .require(Scope::Write)
            .map_err(|reason| refuse(AuthError::Forbidden(reason)))?;
    }
    Ok(checked)
}

/// Vrai si la route est de celles qu'un jeton d'API ne peut pas appeler.
fn is_token_denied(path: &str) -> bool {
    let path = path.strip_prefix("/api").unwrap_or(path);
    TOKEN_DENIED.iter().any(|prefix| {
        path == *prefix || path.strip_prefix(prefix).is_some_and(|rest| rest.starts_with('/'))
    })
}

/// Une écriture portée par le cookie de session doit prouver qu'elle vient de
/// chez nous : l'en-tête posé par l'interface, à défaut les métadonnées de
/// récupération du navigateur (`Sec-Fetch-Site`), à défaut une `Origin` qui
/// coïncide avec l'hôte. Sans rien de tout cela, c'est un formulaire tiers ou un
/// client qui n'a pas lu la documentation ; les deux sont refusés, avec le mot
/// de passe du remède.
fn same_origin(headers: &HeaderMap) -> Result<(), &'static str> {
    let value = |name: &str| headers.get(name).and_then(|v| v.to_str().ok()).map(str::trim);
    if value(CSRF_HEADER).is_some_and(|v| v.eq_ignore_ascii_case(CSRF_VALUE)) {
        return Ok(());
    }
    if let Some(site) = value("sec-fetch-site") {
        return match site {
            "same-origin" | "none" => Ok(()),
            _ => Err(CROSS_SITE),
        };
    }
    if let Some(origin) = value("origin") {
        let origin_host = origin.split_once("://").map_or(origin, |(_, rest)| rest);
        let origin_host = origin_host.split('/').next().unwrap_or(origin_host);
        return match value("host") {
            Some(host) if host.eq_ignore_ascii_case(origin_host) => Ok(()),
            _ => Err(CROSS_SITE),
        };
    }
    Err(NO_PROOF)
}

const CROSS_SITE: &str = "Cross-site request refused.";
const NO_PROOF: &str = "State-changing requests must carry the header \
                        `X-Requested-With: DumbMonit` (browsers add it automatically \
                        through the web interface).";

fn is_mutation(method: &Method) -> bool {
    matches!(*method, Method::POST | Method::PUT | Method::DELETE | Method::PATCH)
}

/// Le routeur `/api` est imbriqué : selon la couche, le chemin vu ici porte ou non
/// le préfixe. On tolère les deux plutôt que de dépendre de ce détail.
fn is_self_service(path: &str) -> bool {
    let path = path.strip_prefix("/api").unwrap_or(path);
    SELF_SERVICE.contains(&path)
}

/// Vrai si l'écriture `method path` fait partie de ce qu'un opérateur peut faire.
pub fn is_operator_route(method: &Method, path: &str) -> bool {
    let path = path.strip_prefix("/api").unwrap_or(path);
    let path = path.strip_suffix('/').unwrap_or(path);
    OPERATOR_ROUTES.iter().any(|(allowed, pattern)| {
        *allowed == method.as_str() && {
            let mut wanted = pattern.split('/');
            let mut actual = path.split('/');
            loop {
                match (wanted.next(), actual.next()) {
                    (None, None) => break true,
                    (Some("*"), Some(segment)) if !segment.is_empty() => {}
                    (Some(w), Some(a)) if w == a => {}
                    _ => break false,
                }
            }
        }
    })
}

/// Relit le cookie et confirme la session auprès de la base, puis charge le
/// compte. Une session dont le compte est désactivé ou a disparu est fermée.
///
/// Rendre `None` plutôt qu'une erreur pour un cookie absent, illisible, inconnu ou
/// expiré : de l'extérieur, ces cas sont le même — « pas authentifié » — et les
/// distinguer dans la réponse aiderait surtout celui qui cherche à deviner.
pub async fn current_session(
    pool: &SqlitePool,
    headers: &HeaderMap,
) -> Result<Option<(SessionToken, User)>, AuthError> {
    let Some(token) = cookie::extract(headers) else { return Ok(None) };
    let Some(user_id) = session::authenticate(pool, &token).await? else { return Ok(None) };
    match users::get(pool, user_id).await? {
        Some(user) if !user.disabled => Ok(Some((token, user))),
        _ => {
            session::delete(pool, token.id()).await?;
            Ok(None)
        }
    }
}

/// Extracteur : le compte courant, ou 401 s'il n'y en a pas.
pub struct Authenticated(pub User);

impl<S: Send + Sync> FromRequestParts<S> for Authenticated {
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<CurrentUser>()
            .map(|current| Self(current.0.clone()))
            .ok_or_else(|| AuthError::Unauthorized("Authentication required.".into()))
    }
}

/// Extracteur : l'identité de la requête — compte ou jeton — ou 401.
pub struct Identity(pub Principal);

impl<S: Send + Sync> FromRequestParts<S> for Identity {
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<CurrentPrincipal>()
            .map(|current| Self(current.0.clone()))
            .ok_or_else(|| AuthError::Unauthorized("Authentication required.".into()))
    }
}

/// Extracteur : une identité d'administrateur — compte `admin` ou jeton `write`.
pub struct AdminIdentity(pub Principal);

impl<S: Send + Sync> FromRequestParts<S> for AdminIdentity {
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Identity(principal) = Identity::from_request_parts(parts, state).await?;
        if !principal.is_admin() {
            return Err(AuthError::admin_required());
        }
        Ok(Self(principal))
    }
}

/// Extracteur : une identité qui peut traiter les alertes — compte `admin` ou
/// `operator`, ou jeton `write`. Réservé aux gestionnaires inscrits dans
/// [`OPERATOR_ROUTES`] ; partout ailleurs, c'est [`AdminIdentity`].
pub struct OperatorIdentity(pub Principal);

impl<S: Send + Sync> FromRequestParts<S> for OperatorIdentity {
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Identity(principal) = Identity::from_request_parts(parts, state).await?;
        if !principal.can_operate() {
            return Err(AuthError::operator_required());
        }
        Ok(Self(principal))
    }
}

/// Extracteur : le compte courant, à condition qu'il soit administrateur.
/// Réservé aux routes que les jetons ne peuvent pas appeler ; ailleurs,
/// [`AdminIdentity`] accepte aussi un jeton `write`.
pub struct AdminUser(pub User);

impl<S: Send + Sync> FromRequestParts<S> for AdminUser {
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Authenticated(user) = Authenticated::from_request_parts(parts, state).await?;
        if !user.role.is_admin() {
            return Err(AuthError::admin_required());
        }
        Ok(Self(user))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_service_routes_are_recognised_with_or_without_the_prefix() {
        assert!(is_self_service("/auth/logout"));
        assert!(is_self_service("/api/auth/password"));
        assert!(is_self_service("/auth/totp/enroll"));
        assert!(!is_self_service("/targets"));
        assert!(!is_self_service("/api/users"));
    }

    #[test]
    fn operators_get_alert_handling_and_nothing_else() {
        let yes = |m: Method, p: &str| assert!(is_operator_route(&m, p), "{m} {p}");
        let no = |m: Method, p: &str| assert!(!is_operator_route(&m, p), "{m} {p}");
        yes(Method::POST, "/alerts/abc123/ack");
        yes(Method::DELETE, "/api/alerts/abc123/ack");
        yes(Method::POST, "/alerts/silences");
        yes(Method::DELETE, "/alerts/silences/4");
        yes(Method::POST, "/alerts/history/dismiss-resolved");
        yes(Method::POST, "/alerts/history/9/dismiss");
        yes(Method::PUT, "/alerts/rules/2/overrides/7");
        yes(Method::DELETE, "/alerts/rules/2/overrides/7");

        no(Method::PUT, "/alerts/abc123/ack");
        no(Method::POST, "/alerts//ack");
        no(Method::POST, "/alerts/abc/ack/extra");
        no(Method::POST, "/alerts/rules");
        no(Method::PUT, "/alerts/rules/2");
        no(Method::DELETE, "/alerts/rules/2");
        no(Method::POST, "/alerts/rules/2/enable");
        no(Method::PUT, "/alerts/silences/4");
        no(Method::POST, "/targets");
        no(Method::PUT, "/targets/1");
        no(Method::POST, "/targets/1/probe");
        no(Method::POST, "/users");
        no(Method::PUT, "/users/1");
        no(Method::POST, "/tokens");
        no(Method::PUT, "/notify/policy");
        no(Method::POST, "/notify/channels");
        no(Method::PUT, "/settings");
        no(Method::POST, "/backup/restore");
        no(Method::POST, "/status-pages");
    }

    #[test]
    fn the_routes_denied_to_tokens_are_matched_by_prefix() {
        assert!(is_token_denied("/auth/me"));
        assert!(is_token_denied("/api/auth/audit"));
        assert!(is_token_denied("/users"));
        assert!(is_token_denied("/api/users/3/totp"));
        assert!(is_token_denied("/tokens/4"));
        assert!(is_token_denied("/agent/tokens"));
        assert!(!is_token_denied("/targets"));
        assert!(!is_token_denied("/api/agent/relay"));
        assert!(!is_token_denied("/authors"), "un préfixe, pas une sous-chaîne");
    }

    #[test]
    fn a_mutation_needs_a_proof_of_origin() {
        let mut headers = HeaderMap::new();
        assert!(same_origin(&headers).is_err(), "rien du tout : refusé");

        headers.insert("x-requested-with", "dumbmonit".parse().unwrap());
        assert!(same_origin(&headers).is_ok(), "l'en-tête de l'interface suffit");

        let mut fetch = HeaderMap::new();
        fetch.insert("sec-fetch-site", "cross-site".parse().unwrap());
        assert!(same_origin(&fetch).is_err());
        fetch.insert("sec-fetch-site", "same-origin".parse().unwrap());
        assert!(same_origin(&fetch).is_ok());

        let mut origin = HeaderMap::new();
        origin.insert("origin", "http://monit.lan:8080".parse().unwrap());
        origin.insert("host", "monit.lan:8080".parse().unwrap());
        assert!(same_origin(&origin).is_ok());
        origin.insert("origin", "http://evil.example".parse().unwrap());
        assert!(same_origin(&origin).is_err());
    }

    #[test]
    fn only_writes_are_mutations() {
        assert!(is_mutation(&Method::POST));
        assert!(is_mutation(&Method::DELETE));
        assert!(!is_mutation(&Method::GET));
        assert!(!is_mutation(&Method::HEAD));
    }
}
