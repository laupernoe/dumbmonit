use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};

use crate::auth::ldap::LdapEnv;
use crate::auth::oidc::OidcEnv;

/// Configuration du serveur, entièrement pilotée par variables d'environnement —
/// c'est ce qui rend le déploiement Docker trivial : pas de fichier à monter.
#[derive(Debug, Clone)]
pub struct Config {
    /// Adresse d'écoute de l'API et de l'interface web.
    pub bind: SocketAddr,
    /// Répertoire persistant : base SQLite et clé de chiffrement.
    pub data_dir: PathBuf,
    /// URL de base d'une instance VictoriaMetrics externe (`DUMBMONIT_VM_URL`).
    ///
    /// `None` : le serveur lance lui-même le binaire VictoriaMetrics embarqué dans
    /// l'image et le pilote comme un processus enfant (voir [`crate::tsdb::embedded`]).
    pub victoria_url: Option<String>,
    /// Chemin du binaire VictoriaMetrics utilisé en mode embarqué.
    pub vm_binary: PathBuf,
    /// Adresse d'écoute HTTP du VictoriaMetrics embarqué. Sur la boucle locale par
    /// défaut : rien d'autre que ce serveur n'a à lui parler. L'overlay de
    /// développement l'ouvre sur `0.0.0.0` pour interroger MetricsQL à la main.
    pub vm_listen: String,
    /// Durée de rétention des séries du VictoriaMetrics embarqué, dans la syntaxe
    /// de son option `-retentionPeriod` (`12` = douze mois, `30d`, `2y`).
    pub vm_retention: String,
    /// Budget mémoire des caches du VictoriaMetrics embarqué (`-memory.allowedBytes`).
    pub vm_memory: String,
    /// Vrai si `DUMBMONIT_VM_RETENTION` est donnée : l'interface dit alors d'où
    /// vient la valeur tant qu'aucun réglage ne la remplace.
    pub vm_retention_from_env: bool,
    /// Idem pour `DUMBMONIT_VM_LONG_RETENTION`.
    pub vm_long_retention_from_env: bool,
    /// URL d'un VictoriaMetrics externe pour l'historique long terme
    /// (`DUMBMONIT_VM_LONG_URL`). `None` : un second processus embarqué, sous
    /// `/data/vm-long`, quand le premier l'est aussi.
    pub vm_long_url: Option<String>,
    /// Adresse d'écoute du VictoriaMetrics long terme embarqué.
    pub vm_long_listen: String,
    /// Rétention par défaut de l'historique long terme, avant tout réglage fait
    /// dans l'interface (même syntaxe que `vm_retention`).
    pub vm_long_retention: String,
    /// Budget mémoire du VictoriaMetrics long terme : il ne reçoit qu'un point
    /// toutes les cinq minutes par série, de petits caches suffisent.
    pub vm_long_memory: String,
    /// Binaire VictoriaLogs embarqué (`DUMBMONIT_VLOGS_BINARY`).
    pub vlogs_binary: PathBuf,
    /// Adresse d'écoute du VictoriaLogs embarqué.
    pub vlogs_listen: String,
    /// Budget mémoire du VictoriaLogs embarqué.
    pub vlogs_memory: String,
    /// Secret d'instance, fourni par l'environnement ou lu depuis `data_dir`.
    /// `None` ici signifie « à charger ou générer au démarrage ».
    pub secret: Option<String>,
    /// Nombre maximal d'interrogations simultanées, tous collecteurs confondus.
    pub max_concurrent_probes: usize,
    /// Délai au-delà duquel une interrogation est abandonnée.
    pub probe_timeout: Duration,
    /// Période de vidage du tampon d'écriture vers VictoriaMetrics.
    pub write_flush_interval: Duration,
    /// Taille de lot qui déclenche un envoi immédiat vers VictoriaMetrics, sans
    /// attendre l'échéance.
    pub write_flush_size: usize,
    /// Threads de travail du runtime Tokio.
    ///
    /// Tokio en crée un par cœur par défaut : sur un NAS à seize cœurs, ce sont
    /// seize piles et seize files locales pour un serveur qui passe l'essentiel de
    /// son temps à attendre le réseau. Quatre suffisent largement à un homelab ;
    /// la valeur reste réglable pour une instance qui interroge des centaines de
    /// cibles.
    pub workers: usize,
    /// Nombre maximal de connexions SQLite ouvertes en parallèle.
    ///
    /// Chaque connexion coûte un thread `sqlx` et son cache de pages : quatre
    /// suffisent à l'interface et au planificateur d'un homelab, la base ne
    /// servant qu'à la configuration et aux états d'alerte.
    pub db_pool_size: u32,
    /// Répertoire des binaires de l'agent servis sous `/download/`.
    ///
    /// Rempli à la construction de l'image ; s'il est vide ou absent, la commande
    /// d'installation échoue proprement avec un 404, sans empêcher le serveur de
    /// tourner.
    pub agent_dir: PathBuf,
    /// Réinitialisation des comptes demandée par l'environnement.
    ///
    /// Le seul moyen de reprendre la main sur une instance dont plus personne n'a
    /// le mot de passe : il n'y a pas de courriel de récupération. Le serveur
    /// efface tous les comptes et toutes les sessions au démarrage (les réglages
    /// SSO restent), puis l'interface repropose l'écran de première configuration.
    pub reset_password: bool,
    /// Code de première configuration imposé (`DUMBMONIT_SETUP_CODE`), pour un
    /// déploiement automatisé. Absent : le serveur en tire un à chaque
    /// démarrage et l'affiche dans son journal (voir [`crate::auth::setup_code`]).
    pub setup_code: Option<String>,
    /// Mode démonstration publique (`DUMBMONIT_DEMO`) : base recréée à chaque
    /// démarrage avec un parc fictif, toute écriture refusée, aucune
    /// notification envoyée. Voir [`crate::demo`].
    pub demo: bool,
    /// Mandataires inverses dont `X-Forwarded-For` est cru
    /// (`DUMBMONIT_TRUSTED_PROXIES`, adresses ou CIDR séparés par des virgules).
    ///
    /// Vide par défaut : l'adresse de la connexion TCP fait foi pour les
    /// compteurs de tentatives et le journal d'audit. Derrière un mandataire,
    /// tout le monde partagerait sinon le même seau — le sien.
    pub trusted_proxies: Vec<ipnet::IpNet>,
    /// Réseaux proposés d'office au balayage (`DUMBMONIT_SCAN_SUBNETS`, CIDR IPv4
    /// séparés par des virgules). Derrière le pont Docker, le serveur ne voit pas le
    /// réseau local : c'est le seul moyen de l'annoncer sans le deviner.
    pub scan_subnets: Vec<ipnet::Ipv4Net>,
    /// Accepte un fournisseur OIDC en `http://` (`DUMBMONIT_OIDC_ALLOW_HTTP`).
    ///
    /// Un émetteur en clair livre le code d'autorisation et le secret client à
    /// quiconque écoute le réseau ; ce n'est acceptable que pour un fournisseur
    /// de test sur la boucle locale.
    pub oidc_allow_http: bool,
    /// Ouvre `GET /metrics` et `GET /federate` sans jeton
    /// (`DUMBMONIT_METRICS_PUBLIC`).
    ///
    /// Faux par défaut : ces routes disent l'état de l'instance et rendent
    /// toutes les mesures des équipements, ce qui n'a pas à être lisible par
    /// quiconque joint le port. Ne l'ouvrir que sur un réseau où l'accès au
    /// port est déjà filtré.
    pub metrics_public: bool,
    /// Sauvegardes locales planifiées de la base (`DUMBMONIT_BACKUP_ENABLED`).
    ///
    /// Actives par défaut : une copie quotidienne de quelques mégaoctets est le
    /// prix le plus bas qu'on puisse payer pour oser une mise à jour, et
    /// personne n'active une option qu'il ne connaît pas encore.
    pub backup_enabled: bool,
    /// Répertoire des sauvegardes locales (`DUMBMONIT_BACKUP_DIR`). Vide :
    /// `<data_dir>/backups`, pour qu'un seul volume porte toute la persistance.
    pub backup_dir: Option<PathBuf>,
    /// Période entre deux sauvegardes locales (`DUMBMONIT_BACKUP_INTERVAL_HOURS`).
    pub backup_interval: Duration,
    /// Nombre de sauvegardes locales conservées (`DUMBMONIT_BACKUP_KEEP`).
    pub backup_keep: usize,
    /// Connexion OpenID Connect décrite par l'environnement (`DUMBMONIT_OIDC_*`,
    /// `DUMBMONIT_PUBLIC_URL`). Un réglage enregistré depuis l'interface l'emporte.
    pub oidc: OidcEnv,
    /// Annuaire LDAP / Active Directory décrit par l'environnement
    /// (`DUMBMONIT_LDAP_*`). Un réglage enregistré depuis l'interface l'emporte.
    pub ldap: LdapEnv,
    /// Horloge et identifiant de requête figés pour les tests de la connexion
    /// SAML. Jamais lus dans l'environnement : seul le code qui monte le
    /// serveur peut les poser (voir [`crate::auth::saml::TestHooks`]).
    #[doc(hidden)]
    pub saml_hooks: crate::auth::saml::TestHooks,
    /// Accepte un annuaire en `ldap://` sans StartTLS (`DUMBMONIT_LDAP_ALLOW_PLAIN`).
    ///
    /// Les mots de passe des utilisateurs passeraient en clair sur le réseau ;
    /// ce n'est acceptable que pour un annuaire de test.
    pub ldap_allow_plain: bool,
    /// Chromium sans interface qui fait les captures d'écran de la
    /// surveillance des sites (`DUMBMONIT_BROWSER_URL`, `http://browser:9222`).
    /// `None` : pas de capture, la comparaison de texte fonctionne seule.
    pub browser_url: Option<String>,
    /// Bornes des sondes de santé des sites (`website`), pour qu'une liste de
    /// centaines de sites ne parte jamais d'un bloc. Voir
    /// [`crate::website::throttle`].
    pub website: WebsiteLimits,
    /// Écoute du canal chiffré des agents (`DUMBMONIT_AGENT_LISTEN`,
    /// `0.0.0.0:8443` par défaut ; `off` le coupe). TLS 1.3 et certificats
    /// clients : c'est le seul port à ouvrir vers l'extérieur pour des agents
    /// distants.
    pub agent_listen: Option<SocketAddr>,
    /// Port que les agents composent pour joindre ce canal
    /// (`DUMBMONIT_AGENT_PUBLIC_PORT`), quand une redirection de port ou un
    /// mandataire TCP le publie sous un autre numéro. Par défaut, celui de
    /// l'écoute.
    pub agent_public_port: Option<u16>,
    /// Nombre de jours pendant lesquels l'ancien canal en clair reste accepté
    /// après la mise à jour d'une instance existante
    /// (`DUMBMONIT_AGENT_LEGACY_DAYS`). `None` : jusqu'à ce qu'un administrateur
    /// exige mTLS dans l'interface.
    pub agent_legacy_days: Option<u32>,
}

/// Réglages d'instance du collecteur `website`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WebsiteLimits {
    /// Sondes de sites en vol au plus, tous sites confondus
    /// (`DUMBMONIT_WEBSITE_MAX_CONCURRENT`).
    pub max_concurrent: usize,
    /// Sondes en vol au plus vers un même domaine
    /// (`DUMBMONIT_WEBSITE_MAX_PER_HOST`).
    pub max_per_host: usize,
    /// Écart minimal entre deux requêtes vers un même domaine
    /// (`DUMBMONIT_WEBSITE_HOST_SPACING_SECS`) ; l'option `host_spacing_seconds`
    /// d'un site le remplace pour ce site.
    pub host_spacing: Duration,
    /// Période par défaut de la comparaison des métadonnées
    /// (`DUMBMONIT_WEBSITE_CHANGE_CHECK_MINUTES`) ; l'option
    /// `change_check_minutes` d'un site la remplace pour ce site.
    pub change_check: Duration,
}

impl WebsiteLimits {
    /// Défauts : dix sites en vol, un seul par domaine, deux secondes entre
    /// deux requêtes vers un même domaine, métadonnées comparées toutes les six
    /// heures.
    fn from_env() -> Result<Self> {
        use crate::website::options as o;
        let spacing = env_parsed::<u64>(
            "DUMBMONIT_WEBSITE_HOST_SPACING_SECS",
            &o::DEFAULT_HOST_SPACING_SECS.to_string(),
        )?
        .min(u64::from(o::MAX_HOST_SPACING_SECS));
        let minutes = env_parsed::<u64>(
            "DUMBMONIT_WEBSITE_CHANGE_CHECK_MINUTES",
            &o::DEFAULT_CHANGE_CHECK_MINUTES.to_string(),
        )?
        .min(u64::from(o::MAX_CHANGE_CHECK_MINUTES));
        Ok(Self {
            max_concurrent: env_parsed::<usize>("DUMBMONIT_WEBSITE_MAX_CONCURRENT", "10")?
                .clamp(1, 256),
            max_per_host: env_parsed::<usize>("DUMBMONIT_WEBSITE_MAX_PER_HOST", "1")?.clamp(1, 16),
            host_spacing: Duration::from_secs(spacing),
            change_check: Duration::from_secs(minutes * 60),
        })
    }
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            bind: env_parsed("DUMBMONIT_BIND", "0.0.0.0:8080")?,
            data_dir: PathBuf::from(env_or("DUMBMONIT_DATA_DIR", "/data")),
            victoria_url: env_var("DUMBMONIT_VM_URL")
                .map(|url| url.trim_end_matches('/').to_string()),
            vm_binary: PathBuf::from(env_or("DUMBMONIT_VM_BINARY", "/victoria-metrics-prod")),
            vm_listen: env_or("DUMBMONIT_VM_LISTEN", "127.0.0.1:8428"),
            vm_retention: env_or("DUMBMONIT_VM_RETENTION", "12"),
            vm_memory: env_or("DUMBMONIT_VM_MEMORY", "256MB"),
            vm_retention_from_env: env_var("DUMBMONIT_VM_RETENTION").is_some(),
            vm_long_retention_from_env: env_var("DUMBMONIT_VM_LONG_RETENTION").is_some(),
            vm_long_url: env_var("DUMBMONIT_VM_LONG_URL")
                .map(|url| url.trim_end_matches('/').to_string())
                .filter(|url| !url.is_empty()),
            vm_long_listen: env_or("DUMBMONIT_VM_LONG_LISTEN", "127.0.0.1:8429"),
            vm_long_retention: env_or("DUMBMONIT_VM_LONG_RETENTION", "730d"),
            vm_long_memory: env_or("DUMBMONIT_VM_LONG_MEMORY", "64MB"),
            vlogs_binary: PathBuf::from(env_or("DUMBMONIT_VLOGS_BINARY", "/victoria-logs-prod")),
            vlogs_listen: env_or("DUMBMONIT_VLOGS_LISTEN", "127.0.0.1:9428"),
            vlogs_memory: env_or("DUMBMONIT_VLOGS_MEMORY", "64MB"),
            secret: env_var("DUMBMONIT_SECRET"),
            max_concurrent_probes: env_parsed("DUMBMONIT_MAX_CONCURRENT_PROBES", "64")?,
            probe_timeout: Duration::from_secs(env_parsed("DUMBMONIT_PROBE_TIMEOUT_SECS", "10")?),
            write_flush_interval: Duration::from_secs(env_parsed(
                "DUMBMONIT_FLUSH_INTERVAL_SECS",
                "5",
            )?),
            write_flush_size: env_parsed("DUMBMONIT_FLUSH_BATCH", "5000")?,
            workers: env_parsed::<usize>("DUMBMONIT_WORKERS", &default_workers().to_string())?
                .clamp(1, 256),
            db_pool_size: env_parsed::<u32>("DUMBMONIT_DB_POOL", "4")?.clamp(1, 64),
            agent_dir: PathBuf::from(env_or("DUMBMONIT_AGENT_DIR", "/agents")),
            reset_password: env_flag("DUMBMONIT_RESET_PASSWORD"),
            setup_code: env_var("DUMBMONIT_SETUP_CODE")
                .filter(|code| !crate::auth::setup_code::normalize(code).is_empty()),
            demo: env_flag("DUMBMONIT_DEMO"),
            trusted_proxies: crate::auth::client_ip::parse_trusted_proxies(&env_or(
                "DUMBMONIT_TRUSTED_PROXIES",
                "",
            )),
            scan_subnets: parse_scan_subnets(&env_or("DUMBMONIT_SCAN_SUBNETS", "")),
            oidc_allow_http: env_flag("DUMBMONIT_OIDC_ALLOW_HTTP"),
            metrics_public: env_flag("DUMBMONIT_METRICS_PUBLIC"),
            backup_enabled: env_flag_or("DUMBMONIT_BACKUP_ENABLED", true),
            backup_dir: env_var("DUMBMONIT_BACKUP_DIR")
                .map(|dir| dir.trim().to_string())
                .filter(|dir| !dir.is_empty())
                .map(PathBuf::from),
            // Bornée à une heure : au-delà, la sauvegarde coûterait plus que ce
            // qu'elle protège. Bornée à un an en haut pour rester un nombre.
            backup_interval: Duration::from_secs(
                env_parsed::<u64>("DUMBMONIT_BACKUP_INTERVAL_HOURS", "24")?.clamp(1, 8760) * 3600,
            ),
            backup_keep: env_parsed::<usize>("DUMBMONIT_BACKUP_KEEP", "7")?.clamp(1, 365),
            oidc: OidcEnv::from_env(),
            ldap: LdapEnv::from_env(),
            saml_hooks: Default::default(),
            ldap_allow_plain: env_flag("DUMBMONIT_LDAP_ALLOW_PLAIN"),
            browser_url: env_var("DUMBMONIT_BROWSER_URL")
                .map(|url| url.trim().trim_end_matches('/').to_string())
                .filter(|url| !url.is_empty()),
            website: WebsiteLimits::from_env()?,
            agent_listen: parse_agent_listen(env_var("DUMBMONIT_AGENT_LISTEN").as_deref())?,
            agent_public_port: env_var("DUMBMONIT_AGENT_PUBLIC_PORT")
                .map(|raw| {
                    raw.trim().parse::<u16>().ok().filter(|port| *port != 0).with_context(|| {
                        format!("DUMBMONIT_AGENT_PUBLIC_PORT: invalid port '{raw}'")
                    })
                })
                .transpose()?,
            agent_legacy_days: env_var("DUMBMONIT_AGENT_LEGACY_DAYS")
                .map(|raw| {
                    raw.trim().parse::<u32>().with_context(|| {
                        format!("DUMBMONIT_AGENT_LEGACY_DAYS: invalid value '{raw}'")
                    })
                })
                .transpose()?,
        })
    }

    /// Port que les agents composent pour le canal chiffré, s'il est ouvert.
    pub fn agent_port(&self) -> Option<u16> {
        let listen = self.agent_listen?;
        Some(self.agent_public_port.unwrap_or(listen.port()))
    }

    pub fn database_path(&self) -> PathBuf {
        self.data_dir.join(crate::db::DATABASE_FILE)
    }

    pub fn secret_path(&self) -> PathBuf {
        self.data_dir.join("secret.key")
    }

    /// Répertoire de stockage du VictoriaMetrics embarqué : sous `data_dir`, pour
    /// qu'un seul volume porte toute la persistance.
    pub fn vm_data_path(&self) -> PathBuf {
        self.data_dir.join("vm")
    }

    /// Répertoire du VictoriaMetrics long terme embarqué (agrégats 5 min).
    pub fn vm_long_data_path(&self) -> PathBuf {
        self.data_dir.join("vm-long")
    }

    /// Répertoire du VictoriaLogs embarqué.
    pub fn vlogs_data_path(&self) -> PathBuf {
        self.data_dir.join("vlogs")
    }

    /// URL effective du stockage long terme : l'externe s'il est désigné, sinon
    /// le processus embarqué — qui n'existe que si VictoriaMetrics l'est aussi.
    pub fn effective_long_url(&self) -> Option<String> {
        match &self.vm_long_url {
            Some(url) => Some(url.clone()),
            None if self.vm_embedded() => {
                Some(format!("http://{}", loopback_address(&self.vm_long_listen)))
            }
            None => None,
        }
    }

    /// Le stockage long terme est-il un processus lancé par ce serveur ?
    pub fn vm_long_embedded(&self) -> bool {
        self.vm_long_url.is_none() && self.vm_embedded()
    }

    /// URL du VictoriaLogs embarqué.
    pub fn vlogs_url(&self) -> String {
        format!("http://{}", loopback_address(&self.vlogs_listen))
    }

    /// Répertoire des sauvegardes locales.
    pub fn backup_path(&self) -> PathBuf {
        self.backup_dir.clone().unwrap_or_else(|| self.data_dir.join("backups"))
    }

    /// VictoriaMetrics est-il lancé par ce serveur plutôt que fourni de l'extérieur ?
    pub fn vm_embedded(&self) -> bool {
        self.victoria_url.is_none()
    }

    /// URL effective de VictoriaMetrics : celle de l'environnement, ou celle du
    /// processus embarqué.
    pub fn effective_victoria_url(&self) -> String {
        match &self.victoria_url {
            Some(url) => url.clone(),
            None => format!("http://{}", loopback_address(&self.vm_listen)),
        }
    }
}

/// Lit `DUMBMONIT_AGENT_LISTEN` : absente, l'écoute par défaut ; `off`
/// (ou `none`, `0`, vide), pas de canal des agents.
fn parse_agent_listen(raw: Option<&str>) -> Result<Option<SocketAddr>> {
    let Some(raw) = raw.map(str::trim) else {
        return Ok(Some(SocketAddr::from(([0, 0, 0, 0], dumbmonit_proto::DEFAULT_AGENT_TLS_PORT))));
    };
    if matches!(raw.to_ascii_lowercase().as_str(), "" | "off" | "none" | "0" | "false" | "no") {
        return Ok(None);
    }
    raw.parse::<SocketAddr>().map(Some).with_context(|| {
        format!("DUMBMONIT_AGENT_LISTEN: invalid address '{raw}' (e.g. 0.0.0.0:8443)")
    })
}

/// Adresse par laquelle joindre un processus local qui écoute sur `listen` :
/// `0.0.0.0:8428` ou `:8428` s'interrogent sur `127.0.0.1:8428`.
fn loopback_address(listen: &str) -> String {
    let (host, port) = listen.rsplit_once(':').unwrap_or((listen, "8428"));
    let host = match host.trim_matches(|c| c == '[' || c == ']') {
        "" | "0.0.0.0" | "::" | "[::]" => "127.0.0.1",
        other => other,
    };
    if host.contains(':') { format!("[{host}]:{port}") } else { format!("{host}:{port}") }
}

/// Threads de travail par défaut : au plus quatre, et jamais plus que de cœurs.
fn default_workers() -> usize {
    std::thread::available_parallelism().map_or(2, |n| n.get()).min(4)
}

/// Lecture d'une variable `DUMBMONIT_*`, avec repli sur `EZYMONIT_*` (voir
/// [`dumbmonit_proto::env`]). Tout le serveur passe par ici : c'est ce qui
/// garantit que l'ancien nom est accepté partout, et signalé une seule fois.
pub fn env_var(key: &str) -> Option<String> {
    dumbmonit_proto::env::var(key)
}

/// Lit `DUMBMONIT_SCAN_SUBNETS` : CIDR IPv4 séparés par des virgules, ramenés à
/// l'adresse du réseau. Une entrée illisible est ignorée (avec un avertissement)
/// plutôt que de bloquer le démarrage.
pub fn parse_scan_subnets(raw: &str) -> Vec<ipnet::Ipv4Net> {
    let mut found: Vec<ipnet::Ipv4Net> = Vec::new();
    for entry in raw.split(',').map(str::trim).filter(|entry| !entry.is_empty()) {
        match entry.parse::<ipnet::Ipv4Net>() {
            Ok(net) if !found.contains(&net.trunc()) => found.push(net.trunc()),
            Ok(_) => {}
            Err(_) => tracing::warn!(entry, "DUMBMONIT_SCAN_SUBNETS: entrée ignorée"),
        }
    }
    found
}

fn env_or(key: &str, default: &str) -> String {
    env_var(key).unwrap_or_else(|| default.to_string())
}

/// Drapeau booléen dont le défaut n'est pas « faux ».
///
/// Une valeur incomprise retombe sur le défaut plutôt que sur `false` : couper
/// les sauvegardes parce que quelqu'un a écrit `DUMBMONIT_BACKUP_ENABLED=oui`
/// serait la pire façon de traiter une faute de frappe.
pub fn env_flag_or(key: &str, default: bool) -> bool {
    match env_var(key).map(|v| v.trim().to_ascii_lowercase()) {
        None => default,
        Some(value) => match value.as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            other => {
                tracing::warn!(
                    key,
                    value = other,
                    default,
                    "unrecognised boolean value, keeping the default"
                );
                default
            }
        },
    }
}

/// Drapeau booléen : `1`, `true`, `yes` ou `on`, sans distinction de casse.
pub fn env_flag(key: &str) -> bool {
    env_var(key)
        .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

fn env_parsed<T>(key: &str, default: &str) -> Result<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let raw = env_or(key, default);
    raw.parse::<T>()
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("{key}: invalid value \"{raw}\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_agent_channel_listens_on_8443_unless_told_otherwise() {
        assert_eq!(parse_agent_listen(None).unwrap(), Some("0.0.0.0:8443".parse().unwrap()));
        assert_eq!(
            parse_agent_listen(Some(" 127.0.0.1:9443 ")).unwrap(),
            Some("127.0.0.1:9443".parse().unwrap())
        );
        assert_eq!(parse_agent_listen(Some("off")).unwrap(), None);
        assert_eq!(parse_agent_listen(Some("")).unwrap(), None);
        assert!(parse_agent_listen(Some("8443")).is_err(), "une adresse complète est attendue");
    }

    #[test]
    fn the_embedded_instance_is_reached_on_the_loopback() {
        assert_eq!(loopback_address("127.0.0.1:8428"), "127.0.0.1:8428");
        assert_eq!(loopback_address("0.0.0.0:8428"), "127.0.0.1:8428");
        assert_eq!(loopback_address(":8428"), "127.0.0.1:8428");
        assert_eq!(loopback_address("[::]:8428"), "127.0.0.1:8428");
        assert_eq!(loopback_address("10.0.0.5:9000"), "10.0.0.5:9000");
    }
}
