//! Étalement des sondes de sites.
//!
//! Des centaines de sites importés d'un fichier CSV ne doivent jamais partir
//! d'un bloc : ni pour le réseau de l'instance, ni pour les sites eux-mêmes,
//! dont le pare-feu applicatif ou le CDN prendrait vite une rafale venue
//! d'une seule adresse pour une attaque. Trois bornes s'appliquent, avant
//! même le jeton général du planificateur :
//!
//! - un plafond de sondes de sites en vol, tous sites confondus
//!   (`DUMBMONIT_WEBSITE_MAX_CONCURRENT`) ;
//! - un plafond par domaine (`DUMBMONIT_WEBSITE_MAX_PER_HOST`), le domaine
//!   étant le nom enregistrable ([`domain_key`]) : `www.example.com` et
//!   `shop.example.com` partagent le leur ;
//! - un écart minimal entre deux départs vers un même domaine
//!   (`DUMBMONIT_WEBSITE_HOST_SPACING_SECS`, ou l'option
//!   `host_spacing_seconds` du site qui part).
//!
//! Un site encore en file n'est pas remis en file : sa sonde suivante attend
//! la fin de celle-ci plutôt que de s'empiler derrière.
//!
//! Le décalage de départ de chaque cible sur sa période reste celui du
//! planificateur, déterministe et réparti.

use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use dumbmonit_proto::{Target, TargetId};
use reqwest::Url;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::time::Instant;

use super::options;
use crate::config::WebsiteLimits;

/// Suffixes d'hébergeurs dont chaque client a son sous-domaine : il compte
/// comme un domaine à part entière, comme dans la partie privée de la Public
/// Suffix List. Sans eux, trois cents boutiques Shopify seraient interrogées
/// une à une.
const SHARED_SUFFIXES: &[&str] = &[
    "myshopify.com",
    "wixsite.com",
    "wordpress.com",
    "blogspot.com",
    "squarespace.com",
    "webflow.io",
    "github.io",
    "gitlab.io",
    "netlify.app",
    "vercel.app",
    "pages.dev",
    "web.app",
    "firebaseapp.com",
    "herokuapp.com",
    "azurewebsites.net",
    "cloudfront.net",
];

/// Deuxièmes niveaux courants des domaines nationaux (`example.co.uk`,
/// `example.com.br`) : le nom enregistrable y a trois étiquettes.
const SECOND_LEVELS: &[&str] = &[
    "ac", "asso", "biz", "co", "com", "edu", "go", "gob", "gouv", "gov", "info", "ltd", "mil",
    "ne", "net", "nom", "or", "org", "plc", "sch",
];

/// Clé de regroupement d'une adresse : son nom enregistrable, approché sans
/// la Public Suffix List entière, ou l'adresse IP telle quelle.
pub fn domain_key(url: &Url) -> String {
    let host = url.host_str().unwrap_or_default();
    let bare = host.trim_start_matches('[').trim_end_matches(']');
    if bare.parse::<IpAddr>().is_ok() {
        return bare.to_string();
    }
    let name = host.trim_end_matches('.').to_ascii_lowercase();
    let labels: Vec<&str> = name.split('.').filter(|l| !l.is_empty()).collect();
    let count = labels.len();
    let shared = SHARED_SUFFIXES
        .iter()
        .find(|suffix| name.len() > suffix.len() && name.ends_with(&format!(".{suffix}")));
    let national =
        count >= 3 && labels[count - 1].len() == 2 && SECOND_LEVELS.contains(&labels[count - 2]);
    let keep = match shared {
        Some(suffix) => suffix.split('.').count() + 1,
        None if national => 3,
        None => 2,
    };
    labels[count.saturating_sub(keep)..].join(".")
}

/// Ce que le planificateur lit d'une cible avant de la mettre en file : son
/// domaine et son écart propre. Une option fausse n'arrête rien ici : la
/// sonde la signalera comme erreur de configuration.
pub fn plan(target: &Target) -> (String, Option<Duration>) {
    let key = match options::parse_url(&target.address) {
        Ok(url) => domain_key(&url),
        Err(_) => target.address.trim().to_ascii_lowercase(),
    };
    (key, options::host_spacing(target).ok().flatten())
}

pub struct Throttle {
    limits: WebsiteLimits,
    global: Arc<Semaphore>,
    hosts: Mutex<HashMap<String, Arc<Host>>>,
    queued: Arc<Mutex<HashSet<TargetId>>>,
}

/// L'état d'un domaine.
struct Host {
    permits: Arc<Semaphore>,
    /// Avant cette date, aucune requête ne part vers ce domaine.
    next_start: Mutex<Instant>,
}

impl Host {
    /// Retient le prochain départ libre et repousse le suivant de `spacing`.
    /// Le créneau est pris sous le verrou : deux sondes admises ensemble par
    /// le plafond du domaine partent quand même à `spacing` d'écart.
    fn reserve(&self, now: Instant, spacing: Duration) -> Instant {
        let mut next = self.next_start.lock().unwrap_or_else(|e| e.into_inner());
        let start = (*next).max(now);
        *next = start + spacing;
        start
    }
}

/// Une cible en file ou en cours de sonde ; la rendre la libère.
pub struct Claim {
    id: TargetId,
    queued: Arc<Mutex<HashSet<TargetId>>>,
}

impl Drop for Claim {
    fn drop(&mut self) {
        self.queued.lock().unwrap_or_else(|e| e.into_inner()).remove(&self.id);
    }
}

/// Le droit de sonder, à garder pendant toute la sonde.
pub struct Pass {
    _global: OwnedSemaphorePermit,
    _host_permit: OwnedSemaphorePermit,
    /// Garde le domaine en table tant que la sonde court.
    _host: Arc<Host>,
    _claim: Claim,
}

impl Throttle {
    pub fn new(limits: WebsiteLimits) -> Self {
        Self {
            global: Arc::new(Semaphore::new(limits.max_concurrent.max(1))),
            limits,
            hosts: Mutex::new(HashMap::new()),
            queued: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn limits(&self) -> WebsiteLimits {
        self.limits
    }

    /// Met une cible en file, sauf si elle y est déjà ou en cours de sonde.
    pub fn claim(&self, id: TargetId) -> Option<Claim> {
        let mut queued = self.queued.lock().unwrap_or_else(|e| e.into_inner());
        queued.insert(id).then(|| Claim { id, queued: self.queued.clone() })
    }

    /// Attend son tour : d'abord le plafond du domaine, puis l'écart depuis le
    /// départ précédent vers ce domaine, enfin le plafond général. Le domaine
    /// passe en premier pour qu'un domaine chargé n'occupe pas les places des
    /// autres en attendant.
    pub async fn admit(&self, claim: Claim, key: &str, spacing: Option<Duration>) -> Pass {
        let spacing = spacing.unwrap_or(self.limits.host_spacing);
        let host = self.host(key);
        let host_permit =
            host.permits.clone().acquire_owned().await.expect("sémaphore de domaine jamais fermé");
        let start = host.reserve(Instant::now(), spacing);
        tokio::time::sleep_until(start).await;
        let global =
            self.global.clone().acquire_owned().await.expect("sémaphore des sites jamais fermé");
        Pass { _global: global, _host_permit: host_permit, _host: host, _claim: claim }
    }

    fn host(&self, key: &str) -> Arc<Host> {
        let mut hosts = self.hosts.lock().unwrap_or_else(|e| e.into_inner());
        // On oublie les domaines au repos dont l'écart est passé : la table ne
        // garde que ce qui contraint encore un départ.
        if hosts.len() > 64 {
            let now = Instant::now();
            hosts.retain(|_, host| {
                Arc::strong_count(host) > 1
                    || *host.next_start.lock().unwrap_or_else(|e| e.into_inner()) > now
            });
        }
        let per_host = self.limits.max_per_host.max(1);
        hosts
            .entry(key.to_string())
            .or_insert_with(|| {
                Arc::new(Host {
                    permits: Arc::new(Semaphore::new(per_host)),
                    next_start: Mutex::new(Instant::now()),
                })
            })
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits(max_concurrent: usize, max_per_host: usize, spacing: u64) -> WebsiteLimits {
        WebsiteLimits {
            max_concurrent,
            max_per_host,
            host_spacing: Duration::from_secs(spacing),
            change_check: Duration::from_secs(6 * 3600),
        }
    }

    fn key(address: &str) -> String {
        domain_key(&Url::parse(address).unwrap())
    }

    /// Vrai si `admit` rend la main avant une minute (temps simulé).
    async fn admitted(throttle: &Throttle, id: TargetId, domain: &str) -> Option<Pass> {
        let claim = throttle.claim(id).expect("cible libre");
        tokio::time::timeout(Duration::from_secs(60), throttle.admit(claim, domain, None))
            .await
            .ok()
    }

    #[test]
    fn domaines_enregistrables() {
        assert_eq!(key("https://www.example.com/"), "example.com");
        assert_eq!(key("https://shop.example.com/a"), "example.com");
        assert_eq!(key("https://EXAMPLE.com./"), "example.com");
        assert_eq!(key("https://www.example.co.uk/"), "example.co.uk");
        assert_eq!(key("https://boutique.example.com.br/"), "example.com.br");
        assert_eq!(key("https://www.mairie.gouv.fr/"), "mairie.gouv.fr");
        assert_eq!(key("https://a.b.example.fr/"), "example.fr");
        assert_eq!(key("https://localhost:8080/"), "localhost");
        assert_eq!(key("http://192.168.1.10:8080/"), "192.168.1.10");
        assert_eq!(key("http://[::1]:8080/"), "::1");
        // Les clients d'un hébergeur sont des domaines distincts.
        assert_eq!(key("https://une.myshopify.com/"), "une.myshopify.com");
        assert_ne!(key("https://une.myshopify.com/"), key("https://autre.myshopify.com/"));
        assert_eq!(key("https://www.blog.wordpress.com/"), "blog.wordpress.com");
        assert_eq!(key("https://wordpress.com/"), "wordpress.com");
    }

    #[test]
    fn plan_lit_le_domaine_et_l_ecart_propre() {
        let target = crate::website::options::tests::target(
            "shop.example.com/fr",
            &[("host_spacing_seconds", "30")],
        );
        assert_eq!(plan(&target), ("example.com".to_string(), Some(Duration::from_secs(30))));
        let faux = crate::website::options::tests::target(
            "example.com",
            &[("host_spacing_seconds", "beaucoup")],
        );
        assert_eq!(plan(&faux), ("example.com".to_string(), None), "la sonde signalera l'erreur");
    }

    #[test]
    fn une_cible_n_est_pas_mise_deux_fois_en_file() {
        let throttle = Throttle::new(limits(4, 1, 0));
        let claim = throttle.claim(7).unwrap();
        assert!(throttle.claim(7).is_none());
        assert!(throttle.claim(8).is_some(), "une autre cible passe");
        drop(claim);
        assert!(throttle.claim(7).is_some(), "libérée avec sa sonde");
    }

    #[tokio::test(start_paused = true)]
    async fn plafond_general() {
        let throttle = Throttle::new(limits(2, 1, 0));
        let a = admitted(&throttle, 1, "a.fr").await.unwrap();
        let _b = admitted(&throttle, 2, "b.fr").await.unwrap();
        assert!(admitted(&throttle, 3, "c.fr").await.is_none(), "deux en vol au plus");
        drop(a);
        assert!(admitted(&throttle, 4, "c.fr").await.is_some());
    }

    #[tokio::test(start_paused = true)]
    async fn plafond_par_domaine_sans_bloquer_les_autres() {
        let throttle = Throttle::new(limits(10, 1, 0));
        let a = admitted(&throttle, 1, "example.com").await.unwrap();
        assert!(admitted(&throttle, 2, "example.com").await.is_none(), "un seul par domaine");
        assert!(admitted(&throttle, 3, "autre.fr").await.is_some(), "les autres passent");
        drop(a);
        assert!(admitted(&throttle, 4, "example.com").await.is_some());
    }

    #[tokio::test(start_paused = true)]
    async fn ecart_minimal_entre_deux_departs_vers_un_domaine() {
        let throttle = Arc::new(Throttle::new(limits(10, 2, 5)));
        let started = Instant::now();
        let mut departs = Vec::new();
        let mut tasks = Vec::new();
        for id in 1..=3 {
            let throttle = throttle.clone();
            let claim = throttle.claim(id).unwrap();
            tasks.push(tokio::spawn(async move {
                let pass = throttle.admit(claim, "example.com", None).await;
                let at = Instant::now();
                tokio::time::sleep(Duration::from_secs(1)).await;
                drop(pass);
                at
            }));
        }
        for task in tasks {
            departs.push(task.await.unwrap().duration_since(started).as_secs());
        }
        departs.sort_unstable();
        assert_eq!(departs, [0, 5, 10], "deux places, mais cinq secondes entre deux départs");

        // L'écart propre d'un site s'applique au départ qui le suit.
        let throttle = Throttle::new(limits(10, 1, 0));
        let claim = throttle.claim(1).unwrap();
        drop(throttle.admit(claim, "ex.fr", Some(Duration::from_secs(30))).await);
        let before = Instant::now();
        let claim = throttle.claim(2).unwrap();
        drop(throttle.admit(claim, "ex.fr", None).await);
        assert_eq!(before.elapsed().as_secs(), 30);
        // Un autre domaine n'attend pas.
        let before = Instant::now();
        let claim = throttle.claim(3).unwrap();
        drop(throttle.admit(claim, "autre.fr", None).await);
        assert_eq!(before.elapsed().as_secs(), 0);
    }
}
