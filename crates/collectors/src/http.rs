//! Clients HTTP partagés entre les collecteurs qui parlent à une API REST
//! (Proxmox VE, Proxmox Backup Server, Synology…).
//!
//! Un client `reqwest` porte son propre pool de connexions, son résolveur DNS
//! et sa configuration TLS : en construire un par collecteur — et *a fortiori*
//! deux, vérifié et non vérifié — multiplie d'autant les threads et les tampons
//! sans rien apporter. Deux clients pour tout le serveur suffisent : la politique
//! TLS est la seule chose qui ne se change pas par requête, tout le reste (délai,
//! en-têtes d'authentification) se règle sur le `RequestBuilder`.

use std::sync::OnceLock;
use std::time::Duration;

use dumbmonit_proto::ProbeError;

/// Délai d'établissement de la connexion TCP + TLS.
///
/// Un équipement éteint se manifeste par un `SYN` sans réponse : cinq secondes
/// suffisent à le constater sans faire attendre les autres cibles.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Durée de conservation d'une connexion inactive dans le pool.
///
/// Les cibles sont interrogées toutes les 30 à 60 s : garder la connexion
/// ouverte entre deux interrogations évite une poignée de main TLS complète à
/// chaque fois, ce qui est la part la plus coûteuse d'une collecte Proxmox.
const POOL_IDLE_TIMEOUT: Duration = Duration::from_secs(120);

static VERIFIED: OnceLock<reqwest::Client> = OnceLock::new();
static UNVERIFIED: OnceLock<reqwest::Client> = OnceLock::new();

/// Résolutions imposées aux clients partagés : nom d'hôte → adresse locale.
///
/// Ne sert qu'au mode démonstration (`DUMBMONIT_DEMO`), où les équipements
/// fictifs (`pve.home.arpa`…) sont servis par des répondeurs en mémoire sur
/// `127.0.0.1`. À poser **avant** la première collecte : un client déjà
/// construit ne les verrait pas.
static RESOLVE_OVERRIDES: OnceLock<Vec<(String, std::net::SocketAddr)>> = OnceLock::new();

/// Fixe les résolutions imposées ; sans effet si elles l'ont déjà été.
pub fn set_resolve_overrides(overrides: Vec<(String, std::net::SocketAddr)>) {
    let _ = RESOLVE_OVERRIDES.set(overrides);
}

/// Le client partagé, construit à la première demande.
///
/// `insecure_tls` désactive toute vérification du certificat présenté. C'est
/// indispensable face à une installation Proxmox ou Synology par défaut, qui
/// s'annonce avec un certificat auto-signé, mais cela expose la connexion à une
/// interception : l'option n'est jamais implicite, elle vient d'une étiquette de
/// la cible.
pub fn client(insecure_tls: bool) -> Result<reqwest::Client, ProbeError> {
    let cell = if insecure_tls { &UNVERIFIED } else { &VERIFIED };
    if let Some(existing) = cell.get() {
        return Ok(existing.clone());
    }
    let built = build(insecure_tls)?;
    // Une course entre deux cibles construirait deux clients ; le perdant est
    // simplement jeté, ce qui est sans conséquence.
    let _ = cell.set(built.clone());
    Ok(cell.get().cloned().unwrap_or(built))
}

fn build(accept_invalid_certs: bool) -> Result<reqwest::Client, ProbeError> {
    let mut builder = reqwest::Client::builder();
    for (host, address) in RESOLVE_OVERRIDES.get().into_iter().flatten() {
        builder = builder.resolve(host, *address);
    }
    builder
        .danger_accept_invalid_certs(accept_invalid_certs)
        .connect_timeout(CONNECT_TIMEOUT)
        .pool_idle_timeout(POOL_IDLE_TIMEOUT)
        // Un cluster de trois nœuds passe par un seul proxy : deux connexions par
        // hôte couvrent les appels parallèles sans en garder dix ouvertes.
        .pool_max_idle_per_host(2)
        .user_agent(concat!("DumbMonit/", env!("CARGO_PKG_VERSION")))
        .redirect(same_host_redirects())
        .build()
        .map_err(|error| ProbeError::Config(format!("HTTP client unavailable: {error}")))
}

/// Nombre maximal de redirections suivies vers le même hôte.
pub const MAX_REDIRECTS: usize = 5;

/// Politique de redirection des clients d'API : ne suivre que vers le même hôte.
///
/// `reqwest` suit par défaut jusqu'à dix redirections vers n'importe où, et ne
/// retire que `Authorization` et `Cookie` en changeant d'hôte : un `X-Api-Key`,
/// un `PRIVATE-TOKEN` ou un `X-Auth-Token` partirait chez la destination, et un
/// équipement compromis pourrait renvoyer la sonde vers la boucle locale (le
/// VictoriaMetrics embarqué) ou un service de métadonnées. Rester sur l'hôte
/// configuré ferme les deux portes : aucune adresse nouvelle n'est jamais
/// contactée, aucun identifiant ne quitte sa destination. Le passage de `http`
/// à `https` sur le même hôte reste admis, c'est la redirection la plus courante.
/// Une redirection refusée rend la réponse 3xx telle quelle, que l'appelant
/// rapporte comme un statut inattendu.
pub fn same_host_redirects() -> reqwest::redirect::Policy {
    reqwest::redirect::Policy::custom(|attempt| {
        let Some(origin) = attempt.previous().first() else { return attempt.stop() };
        if attempt.previous().len() > MAX_REDIRECTS {
            return attempt.stop();
        }
        if redirect_allowed(origin, attempt.url()) { attempt.follow() } else { attempt.stop() }
    })
}

/// Vrai si une redirection de `from` vers `to` reste sur le même hôte.
pub fn redirect_allowed(from: &reqwest::Url, to: &reqwest::Url) -> bool {
    let same_host = match (from.host_str(), to.host_str()) {
        (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
        _ => false,
    };
    if !same_host {
        return false;
    }
    let same_endpoint =
        from.scheme() == to.scheme() && from.port_or_known_default() == to.port_or_known_default();
    let upgrade = from.scheme() == "http" && to.scheme() == "https";
    same_endpoint || upgrade
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_deux_politiques_tls_donnent_un_client_utilisable() {
        assert!(client(false).is_ok());
        assert!(client(true).is_ok());
        // Un second appel ne reconstruit rien : la cellule est déjà remplie.
        assert!(VERIFIED.get().is_some());
        assert!(UNVERIFIED.get().is_some());
    }

    fn url(text: &str) -> reqwest::Url {
        reqwest::Url::parse(text).expect("URL de test valide")
    }

    #[test]
    fn une_redirection_ne_quitte_jamais_l_hote_configure() {
        let from = url("https://pve.lan:8006/api2/json/version");
        assert!(redirect_allowed(&from, &url("https://pve.lan:8006/autre")));
        assert!(redirect_allowed(&from, &url("https://PVE.lan:8006/autre")));
        assert!(redirect_allowed(&url("http://nas.lan/api"), &url("https://nas.lan/api")));
        assert!(!redirect_allowed(&from, &url("http://127.0.0.1:8428/api/v1/export")));
        assert!(!redirect_allowed(&from, &url("http://169.254.169.254/latest/meta-data")));
        assert!(!redirect_allowed(&from, &url("https://pve.lan:9999/")));
        assert!(!redirect_allowed(&url("https://nas.lan/api"), &url("http://nas.lan/api")));
    }

    /// Répondeur d'une seule requête, qui renvoie `response` et signale l'appel.
    async fn responder(
        response: String,
    ) -> (std::net::SocketAddr, tokio::sync::oneshot::Receiver<String>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (tx, rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buffer = vec![0u8; 4096];
                let read = socket.read(&mut buffer).await.unwrap_or(0);
                let _ = tx.send(String::from_utf8_lossy(&buffer[..read]).to_string());
                let _ = socket.write_all(response.as_bytes()).await;
            }
        });
        (address, rx)
    }

    #[tokio::test]
    async fn une_redirection_vers_un_autre_hote_n_est_pas_suivie_et_l_en_tete_ne_part_pas() {
        // « Autre hôte » : `localhost` au lieu de `127.0.0.1`, même machine mais
        // nom différent — c'est le nom qui compte pour la politique.
        let (victim, mut hit) =
            responder("HTTP/1.1 200 OK\r\ncontent-length: 2\r\n\r\nok".to_string()).await;
        let (origin, _) = responder(format!(
            "HTTP/1.1 302 Found\r\nlocation: http://localhost:{}/vole\r\ncontent-length: 0\r\n\r\n",
            victim.port()
        ))
        .await;

        let response = build(false)
            .unwrap()
            .get(format!("http://{origin}/api"))
            .header("X-Api-Key", "cle-tres-secrete")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 302, "the cross-host redirect must not be followed");
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(hit.try_recv().is_err(), "the redirect target must never be contacted");
    }
}
