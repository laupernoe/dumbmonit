//! Garde des envois sortants contre le serveur lui-même.
//!
//! Un administrateur saisit l'URL d'un webhook, l'adresse d'un Gotify ou d'un
//! relais SMTP : rien ne l'empêche d'y mettre `http://127.0.0.1:8428/…`, l'API
//! sans authentification du VictoriaMetrics embarqué, ou l'adresse d'écoute de
//! ce serveur, puis de demander un « envoi de test ». La requête partirait alors
//! de l'intérieur du conteneur, là où ces points d'entrée se croient seuls.
//!
//! La règle est volontairement étroite : la boucle locale et les plages privées
//! restent permises — c'est là que vivent les services d'un homelab — et seules
//! sont refusées les adresses `IP:port` que ce processus sait être les siennes
//! (voir [`protect`]). La vérification a lieu à trois endroits :
//!
//! - avant l'envoi, sur l'URL saisie, après résolution du nom ([`vet_url`]) ;
//! - à chaque redirection, sur l'adresse de destination ([`redirects`]) ;
//! - au moment de la connexion, par le résolveur DNS du client ([`Resolver`]),
//!   ce qui ferme la porte au « DNS rebinding », un nom qui changerait de
//!   réponse entre la vérification et la connexion.
//!
//! Le résolveur ne reçoit que le nom, pas le port : celui de la requête en cours
//! lui est transmis par une variable locale à la tâche ([`scoped`]), mise à jour
//! à chaque redirection. Sans elle — une connexion établie en arrière-plan par
//! le pool — le résolveur se montre strict et refuse toute adresse qui
//! désignerait l'un de ces hôtes, quel que soit le port.

use std::cell::Cell;
use std::fmt;
use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::sync::RwLock;
use std::time::Duration;

use reqwest::Url;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use reqwest::redirect::Policy;

use crate::notify::error::NotifyError;

/// Adresses internes à ne jamais contacter, renseignées au démarrage.
static INTERNAL: RwLock<Vec<SocketAddr>> = RwLock::new(Vec::new());

/// Délai maximal de la résolution préalable : au-delà, l'envoi lui-même
/// échouera et le dira, et le résolveur du client reste en faction.
const LOOKUP_TIMEOUT: Duration = Duration::from_secs(5);

tokio::task_local! {
    /// Port de la requête en cours, pour le résolveur du client.
    static CURRENT_PORT: Cell<Option<u16>>;
}

/// Ajoute des adresses internes à la liste des destinations refusées.
///
/// Une adresse non spécifiée (`0.0.0.0`, `::`) ou de boucle locale couvre toute
/// la boucle locale sur ce port : un serveur à l'écoute sur `0.0.0.0:8080` se
/// joint aussi bien par `127.0.0.1:8080` que par `[::1]:8080` ou `0.0.0.0:8080`.
pub fn protect(addresses: impl IntoIterator<Item = SocketAddr>) {
    let mut internal = INTERNAL.write().unwrap_or_else(|poisoned| poisoned.into_inner());
    for address in addresses {
        if !internal.contains(&address) {
            internal.push(address);
        }
    }
}

/// Protège l'adresse d'écoute du serveur et celle de VictoriaMetrics.
///
/// L'instance externe (`DUMBMONIT_VM_URL`) est protégée elle aussi : aucun canal
/// de notification n'a de raison légitime d'écrire dans la base de métriques. Son
/// nom est résolu une fois, ici ; une résolution impossible au démarrage n'est
/// pas une erreur, la base pouvant démarrer après le serveur.
pub async fn protect_config(config: &crate::config::Config) {
    protect([config.bind]);
    let victoria = config.effective_victoria_url();
    if let Ok(url) = Url::parse(&victoria)
        && let (Some(host), Some(port)) = (host_of(&url), url.port_or_known_default())
    {
        protect(lookup(host, port).await);
    }
    tracing::debug!(
        internal = ?internal(),
        "outbound notifications kept away from these addresses"
    );
}

fn internal() -> Vec<SocketAddr> {
    INTERNAL.read().unwrap_or_else(|poisoned| poisoned.into_inner()).clone()
}

/// Refus d'une destination interne.
///
/// Un type à part plutôt qu'une chaîne : il traverse reqwest dans la chaîne des
/// causes, et [`refusal_in`] l'y retrouve pour en faire une erreur de
/// configuration plutôt qu'une panne passagère qu'on réessaierait.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    host: String,
    address: SocketAddr,
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\"{}\" resolves to {}, which is DumbMonit itself (its web interface or its \
             metrics database); notifications cannot be sent there",
            self.host, self.address
        )
    }
}

impl std::error::Error for Refused {}

impl From<Refused> for NotifyError {
    fn from(refused: Refused) -> Self {
        NotifyError::Config(refused.to_string())
    }
}

/// Ramène les adresses IPv4 encapsulées dans IPv6 (`::ffff:127.0.0.1`) à leur
/// forme IPv4, pour qu'elles ne passent pas à côté de la comparaison.
fn canonical(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(ip, IpAddr::V4),
        IpAddr::V4(_) => ip,
    }
}

/// Boucle locale ou adresse non spécifiée : une connexion vers `0.0.0.0` aboutit
/// sur la boucle locale.
fn is_local(ip: IpAddr) -> bool {
    ip.is_loopback() || ip.is_unspecified()
}

/// Vrai si `target` désigne la même machine que l'adresse interne `internal`.
fn same_host(internal: IpAddr, target: IpAddr) -> bool {
    let (internal, target) = (canonical(internal), canonical(target));
    if is_local(internal) { is_local(target) } else { internal == target }
}

/// Vrai si se connecter à `target` revient à joindre l'adresse interne `internal`.
pub fn reaches(internal: SocketAddr, target: SocketAddr) -> bool {
    internal.port() == target.port() && same_host(internal.ip(), target.ip())
}

/// Juge les adresses résolues d'un nom. Sans port connu, toute adresse qui
/// désigne l'un des hôtes internes est refusée, par prudence.
fn vet(
    host: &str,
    port: Option<u16>,
    addresses: &[SocketAddr],
    internal: &[SocketAddr],
) -> Result<(), Refused> {
    for address in addresses {
        let hit = internal.iter().find(|endpoint| match port {
            Some(port) => reaches(**endpoint, SocketAddr::new(address.ip(), port)),
            None => same_host(endpoint.ip(), address.ip()),
        });
        if hit.is_some() {
            let address = SocketAddr::new(address.ip(), port.unwrap_or(address.port()));
            return Err(Refused { host: host.to_string(), address });
        }
    }
    Ok(())
}

/// Hôte d'une URL, sans les crochets d'une adresse IPv6.
fn host_of(url: &Url) -> Option<&str> {
    url.host_str().map(|host| host.trim_matches(|c| c == '[' || c == ']'))
}

/// Résolution bornée dans le temps ; un échec donne une liste vide.
async fn lookup(host: &str, port: u16) -> Vec<SocketAddr> {
    if let Ok(ip) = host.parse::<IpAddr>() {
        return vec![SocketAddr::new(ip, port)];
    }
    match tokio::time::timeout(LOOKUP_TIMEOUT, tokio::net::lookup_host((host, port))).await {
        Ok(Ok(addresses)) => addresses.collect(),
        Ok(Err(_)) | Err(_) => Vec::new(),
    }
}

/// Vérifie qu'un hôte et un port ne désignent pas le serveur lui-même.
///
/// Une résolution qui échoue n'est pas un refus : l'envoi échouera de lui-même,
/// avec un message plus juste que celui qu'on pourrait écrire ici.
pub async fn vet_host(host: &str, port: u16) -> Result<(), NotifyError> {
    let internal = internal();
    if internal.is_empty() {
        return Ok(());
    }
    let host = host.trim().trim_matches(|c| c == '[' || c == ']');
    vet(host, Some(port), &lookup(host, port).await, &internal)?;
    Ok(())
}

/// Même vérification pour une URL d'envoi.
pub async fn vet_url(url: &Url) -> Result<(), NotifyError> {
    match (host_of(url), url.port_or_known_default()) {
        (Some(host), Some(port)) => vet_host(host, port).await,
        _ => Ok(()),
    }
}

/// Exécute un envoi en faisant connaître son port au résolveur du client.
pub async fn scoped<F: Future>(port: Option<u16>, send: F) -> F::Output {
    CURRENT_PORT.scope(Cell::new(port), send).await
}

/// Retrouve un refus dans la chaîne des causes d'une erreur.
pub fn refusal_in(error: &(dyn std::error::Error + 'static)) -> Option<Refused> {
    let mut cause = Some(error);
    while let Some(inner) = cause {
        if let Some(refused) = inner.downcast_ref::<Refused>() {
            return Some(refused.clone());
        }
        cause = inner.source();
    }
    None
}

/// Politique de redirection des canaux : celle des collecteurs (même hôte
/// seulement), plus le refus d'une destination interne.
///
/// Le passage de `http` à `https` peut changer de port sur le même hôte : le
/// nouveau port est transmis au résolveur, et une adresse IP littérale, qui ne
/// passe jamais par lui, est jugée ici même.
pub fn redirects() -> Policy {
    Policy::custom(|attempt| {
        let Some(origin) = attempt.previous().first() else { return attempt.stop() };
        if attempt.previous().len() > dumbmonit_collectors::http::MAX_REDIRECTS
            || !dumbmonit_collectors::http::redirect_allowed(origin, attempt.url())
        {
            return attempt.stop();
        }
        let Some(port) = attempt.url().port_or_known_default() else { return attempt.stop() };
        let _ = CURRENT_PORT.try_with(|current| current.set(Some(port)));
        let refused = host_of(attempt.url()).and_then(|host| {
            let ip = host.parse::<IpAddr>().ok()?;
            vet(host, Some(port), &[SocketAddr::new(ip, port)], &internal()).err()
        });
        match refused {
            Some(refused) => attempt.error(refused),
            None => attempt.follow(),
        }
    })
}

/// Résolveur DNS des clients de notification : celui du système, suivi du garde.
pub struct Resolver;

impl Resolve for Resolver {
    fn resolve(&self, name: Name) -> Resolving {
        // Lu ici, de façon synchrone : l'appel a lieu pendant que la requête est
        // en cours d'exécution, donc dans la portée posée par `scoped`.
        let port = CURRENT_PORT.try_with(Cell::get).ok().flatten();
        Box::pin(async move {
            let host = name.as_str().to_string();
            let addresses: Vec<SocketAddr> =
                tokio::net::lookup_host((host.as_str(), 0)).await?.collect();
            vet(&host, port, &addresses, &internal())?;
            Ok(Box::new(addresses.into_iter()) as Addrs)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(text: &str) -> SocketAddr {
        text.parse().unwrap()
    }

    #[test]
    fn la_boucle_locale_entiere_designe_un_service_local() {
        let vm = addr("127.0.0.1:8428");
        for target in ["127.0.0.1:8428", "127.0.0.2:8428", "[::1]:8428", "0.0.0.0:8428"] {
            assert!(reaches(vm, addr(target)), "{target} reaches the VM");
        }
        assert!(reaches(vm, addr("[::ffff:127.0.0.1]:8428")), "IPv4-mapped form");
        let server = addr("0.0.0.0:8080");
        for target in ["127.0.0.1:8080", "[::1]:8080", "[::]:8080"] {
            assert!(reaches(server, addr(target)), "{target} reaches the server");
        }
    }

    #[test]
    fn les_autres_ports_et_les_autres_hotes_restent_permis() {
        let vm = addr("127.0.0.1:8428");
        // Un service du homelab sur la boucle locale ou le réseau local.
        for target in ["127.0.0.1:8123", "[::1]:9000", "192.168.1.10:8428", "10.0.0.2:80"] {
            assert!(!reaches(vm, addr(target)), "{target} must stay allowed");
        }
        // Une adresse interne précise ne couvre que cette adresse.
        let external_vm = addr("192.168.1.20:8428");
        assert!(reaches(external_vm, addr("192.168.1.20:8428")));
        assert!(!reaches(external_vm, addr("192.168.1.21:8428")));
        assert!(!reaches(external_vm, addr("127.0.0.1:8428")));
    }

    #[test]
    fn une_resolution_qui_touche_une_adresse_interne_est_refusee() {
        let internal = [addr("127.0.0.1:8428"), addr("0.0.0.0:8080")];
        let rebound = [addr("203.0.113.5:0"), addr("127.0.0.1:0")];
        let refused = vet("evil.example", Some(8428), &rebound, &internal).unwrap_err();
        assert_eq!(refused.address, addr("127.0.0.1:8428"));
        let message = refused.to_string();
        assert!(message.contains("evil.example") && message.contains("DumbMonit itself"));

        assert!(vet("evil.example", Some(8080), &rebound, &internal).is_err());
        assert!(vet("nas.lan", Some(8123), &rebound, &internal).is_ok(), "other port");
        assert!(vet("ntfy.sh", Some(8428), &rebound[..1], &internal).is_ok(), "public host");
    }

    #[test]
    fn sans_port_connu_le_resolveur_est_strict() {
        let internal = [addr("127.0.0.1:8428")];
        assert!(vet("localhost", None, &[addr("127.0.0.1:0")], &internal).is_err());
        assert!(vet("nas.lan", None, &[addr("192.168.1.10:0")], &internal).is_ok());
    }

    #[test]
    fn un_refus_est_retrouve_dans_la_chaine_des_causes_et_devient_une_erreur_de_configuration() {
        #[derive(Debug)]
        struct Enveloppe(Refused);
        impl fmt::Display for Enveloppe {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("error sending request")
            }
        }
        impl std::error::Error for Enveloppe {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                Some(&self.0)
            }
        }
        let refused = Refused { host: "localhost".into(), address: addr("127.0.0.1:8428") };
        let found = refusal_in(&Enveloppe(refused.clone())).expect("found in the chain");
        assert_eq!(found, refused);
        let error = NotifyError::from(found);
        assert!(matches!(error, NotifyError::Config(_)));
        assert!(!error.is_transient(), "retrying cannot help");
    }
}
