//! Collecteurs DumbMonit, partagés entre le serveur et l'agent relais.
//!
//! Ajouter une intégration se résume à implémenter [`Collector`] et à l'enregistrer
//! dans un [`Registry`] : le planificateur, l'API et l'agent n'ont pas connaissance
//! des types concrets. Les collecteurs qui vivent ici sont ceux qui interrogent un
//! équipement *par le réseau* et peuvent donc tourner aussi bien sur le serveur que
//! sur un agent relais posé dans un autre site. Le collecteur « agent » (fraîcheur
//! des mesures poussées) reste côté serveur : il lit la base.

pub mod activedirectory;
pub mod adguard;
pub mod api_options;
pub mod caddy;
pub mod client_devices;
pub mod crowdsec;
pub mod domain;
pub mod dummy;
pub mod fortigate;
pub mod homeassistant;
pub mod http;
pub mod kubernetes;
pub mod mdaemon;
pub mod mikrotik;
pub mod mongodb;
pub mod npm;
pub mod nut;
pub mod observability;
pub mod opnsense;
pub mod pbs;
pub mod pdm;
pub mod pfsense;
pub mod pihole;
pub mod pmg;
pub mod proxmox;
pub mod rabbitmq;
pub mod redfish;
pub mod redis;
pub mod rest;
pub mod selfhosted;
pub mod snmp;
pub(crate) mod socket;
pub mod sophos;
pub mod synology;
pub mod tailscale;
pub mod traefik;
pub mod truenas;
pub mod unifi;
pub mod unraid;
pub mod uptime;
pub mod veeam;
pub mod vsphere;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use dumbmonit_proto::{Collector, MetricKind, ProbeError, Sample, Target};

pub use activedirectory::ActiveDirectoryCollector;
pub use adguard::AdguardCollector;
pub use caddy::CaddyCollector;
pub use crowdsec::CrowdsecCollector;
pub use domain::DomainCollector;
pub use dummy::DummyCollector;
pub use fortigate::FortigateCollector;
pub use homeassistant::HomeAssistantCollector;
pub use kubernetes::KubernetesCollector;
pub use mdaemon::{MdaemonCollector, SecurityGatewayCollector};
pub use mikrotik::MikrotikCollector;
pub use mongodb::MongodbCollector;
pub use npm::NpmCollector;
pub use nut::NutCollector;
pub use observability::{GraylogCollector, LokiCollector, VictoriaCollector};
pub use opnsense::OpnsenseCollector;
pub use pbs::PbsCollector;
pub use pdm::PdmCollector;
pub use pfsense::PfsenseCollector;
pub use pihole::PiholeCollector;
pub use pmg::PmgCollector;
pub use proxmox::ProxmoxCollector;
pub use rabbitmq::RabbitmqCollector;
pub use redfish::RedfishCollector;
pub use redis::RedisCollector;
pub use selfhosted::{
    ImmichCollector, JellyfinCollector, NextcloudCollector, PaperlessCollector, PlexCollector,
};
pub use snmp::SnmpCollector;
pub use socket::DEFAULT_TIMEOUT as SOCKET_DEFAULT_TIMEOUT;
pub use sophos::SophosCollector;
pub use synology::SynologyCollector;
pub use tailscale::TailscaleCollector;
pub use traefik::TraefikCollector;
pub use truenas::TruenasCollector;
pub use unifi::UnifiCollector;
pub use unraid::UnraidCollector;
pub use uptime::{
    DnsCollector, HttpCollector, MqttCollector, MysqlCollector, PingCollector, PostgresCollector,
    SmtpCollector, TcpCollector, TlsCollector, WebsocketCollector,
};
pub use veeam::VeeamCollector;
pub use vsphere::VsphereCollector;

/// Les collecteurs connus, par type de cible.
///
/// Deux étages : les collecteurs compilés, enregistrés au démarrage et figés
/// ensuite, et ceux qui naissent à l'exécution (paquets d'intégration), qu'une
/// installation ajoute et qu'une désinstallation retire sans redémarrage. Le
/// second étage est partagé entre les clones du registre : l'état du serveur en
/// garde un, l'API d'installation agit sur le même.
#[derive(Clone, Default)]
pub struct Registry {
    /// Clé possédée : un type peut naître à l'exécution (paquet d'intégration).
    collectors: HashMap<Arc<str>, Arc<dyn Collector>>,
    runtime: Arc<RwLock<Table>>,
}

/// Type de cible → collecteur.
type Table = HashMap<Arc<str>, Arc<dyn Collector>>;

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registre des collecteurs réseau, tel qu'un agent relais l'utilise : tout ce
    /// qui interroge un équipement à distance, sans le collecteur de démonstration
    /// ni celui des agents (qui lisent la base du serveur).
    pub fn remote(request_timeout: Duration) -> Self {
        let mut registry = Self::new();
        registry.register(Arc::new(SnmpCollector::new().with_request_timeout(request_timeout)));
        registry.register(Arc::new(ProxmoxCollector::new()));
        registry.register(Arc::new(PbsCollector::new()));
        registry.register(Arc::new(PmgCollector::new()));
        registry.register(Arc::new(PdmCollector::new()));
        registry.register(Arc::new(SynologyCollector::new()));
        registry.register(Arc::new(OpnsenseCollector::new()));
        registry.register(Arc::new(TruenasCollector::new()));
        registry.register(Arc::new(RedfishCollector::new()));
        registry.register(Arc::new(VictoriaCollector::metrics()));
        registry.register(Arc::new(VictoriaCollector::logs()));
        registry.register(Arc::new(LokiCollector::new()));
        registry.register(Arc::new(GraylogCollector::new()));
        registry.register(Arc::new(MdaemonCollector::new()));
        registry.register(Arc::new(SecurityGatewayCollector::new()));
        registry.register(Arc::new(NextcloudCollector::new()));
        registry.register(Arc::new(ImmichCollector::new()));
        registry.register(Arc::new(PaperlessCollector::new()));
        registry.register(Arc::new(JellyfinCollector::new()));
        registry.register(Arc::new(PlexCollector::new()));
        registry.register(Arc::new(PiholeCollector::new()));
        registry.register(Arc::new(AdguardCollector::new()));
        registry.register(Arc::new(NutCollector::new()));
        registry.register(Arc::new(MikrotikCollector::new()));
        registry.register(Arc::new(VsphereCollector::new()));
        registry.register(Arc::new(HomeAssistantCollector::new()));
        registry.register(Arc::new(UnifiCollector::new()));
        registry.register(Arc::new(RedisCollector::new()));
        registry.register(Arc::new(MongodbCollector::new()));
        registry.register(Arc::new(RabbitmqCollector::new()));
        registry.register(Arc::new(CrowdsecCollector::new()));
        registry.register(Arc::new(TraefikCollector::new()));
        registry.register(Arc::new(CaddyCollector::new()));
        registry.register(Arc::new(NpmCollector::new()));
        registry.register(Arc::new(DomainCollector::new()));
        registry.register(Arc::new(KubernetesCollector::new()));
        registry.register(Arc::new(ActiveDirectoryCollector::new()));
        registry.register(Arc::new(PfsenseCollector::new()));
        registry.register(Arc::new(UnraidCollector::new()));
        registry.register(Arc::new(VeeamCollector::new()));
        registry.register(Arc::new(TailscaleCollector::new()));
        registry.register(Arc::new(FortigateCollector::new()));
        registry.register(Arc::new(SophosCollector::new()));
        registry.register(Arc::new(HttpCollector::new()));
        registry.register(Arc::new(TcpCollector::new()));
        registry.register(Arc::new(DnsCollector::new()));
        registry.register(Arc::new(PingCollector::new()));
        registry.register(Arc::new(TlsCollector::new()));
        registry.register(Arc::new(SmtpCollector::new()));
        registry.register(Arc::new(PostgresCollector::new()));
        registry.register(Arc::new(MysqlCollector::new()));
        registry.register(Arc::new(MqttCollector::new()));
        registry.register(Arc::new(WebsocketCollector::new()));
        registry
    }

    pub fn register(&mut self, collector: Arc<dyn Collector>) -> &mut Self {
        self.collectors.insert(Arc::from(collector.kind()), collector);
        self
    }

    /// Ajoute ou remplace un collecteur défini à l'exécution.
    ///
    /// Un type compilé n'est jamais remplacé : un paquet ne peut pas changer en
    /// silence ce que collecte un type livré.
    pub fn register_runtime(&self, collector: Arc<dyn Collector>) -> Result<(), String> {
        let kind: Arc<str> = Arc::from(collector.kind());
        if self.collectors.contains_key(&kind) {
            return Err(format!("\"{kind}\" is a built-in device type"));
        }
        self.runtime_write().insert(kind, collector);
        Ok(())
    }

    /// Retire un collecteur défini à l'exécution ; vrai s'il était enregistré.
    pub fn unregister_runtime(&self, kind: &str) -> bool {
        self.runtime_write().remove(kind).is_some()
    }

    /// Les types définis à l'exécution, triés.
    pub fn runtime_kinds(&self) -> Vec<Arc<str>> {
        let mut kinds: Vec<_> = self.runtime_read().keys().cloned().collect();
        kinds.sort_unstable();
        kinds
    }

    pub fn get(&self, kind: &str) -> Option<Arc<dyn Collector>> {
        match self.collectors.get(kind) {
            Some(collector) => Some(collector.clone()),
            None => self.runtime_read().get(kind).cloned(),
        }
    }

    pub fn kinds(&self) -> Vec<Arc<str>> {
        let mut kinds: Vec<_> = self.collectors.keys().cloned().collect();
        kinds.extend(self.runtime_read().keys().cloned());
        kinds.sort_unstable();
        kinds
    }

    // Un verrou empoisonné ne protège qu'une table de pointeurs, toujours
    // cohérente entre deux instructions : on la reprend telle quelle.
    fn runtime_read(&self) -> std::sync::RwLockReadGuard<'_, Table> {
        self.runtime.read().unwrap_or_else(|poison| poison.into_inner())
    }

    fn runtime_write(&self) -> std::sync::RwLockWriteGuard<'_, Table> {
        self.runtime.write().unwrap_or_else(|poison| poison.into_inner())
    }

    /// Identifie une cible et propose le profil de collecte adapté.
    ///
    /// Même contrat que [`Registry::probe`] : le délai maximal est appliqué ici, pas
    /// dans les collecteurs.
    pub async fn discover(
        &self,
        target: &Target,
        timeout: Duration,
    ) -> Result<Option<String>, ProbeError> {
        let collector = self.get(&target.kind).ok_or_else(|| {
            ProbeError::Config(format!("no collector for type \"{}\"", target.kind))
        })?;

        tokio::time::timeout(timeout, collector.discover(target))
            .await
            .map_err(|_| ProbeError::Timeout(timeout))?
    }

    /// Interroge une cible avec le collecteur correspondant à son type, en
    /// appliquant le délai maximal et les étiquettes d'identité de la cible.
    ///
    /// C'est le seul chemin d'interrogation : les collecteurs ne peuvent donc ni
    /// oublier les étiquettes de base, ni s'affranchir du délai maximal.
    pub async fn probe(
        &self,
        target: &Target,
        timeout: Duration,
    ) -> Result<Vec<Sample>, ProbeError> {
        let collector = self.get(&target.kind).ok_or_else(|| {
            ProbeError::Config(format!(
                "no collector for type \"{}\" (available: {})",
                target.kind,
                self.kinds().join(", ")
            ))
        })?;

        let mut samples = tokio::time::timeout(timeout, collector.probe(target))
            .await
            .map_err(|_| ProbeError::Timeout(timeout))??;

        // Témoin de disponibilité, posé ici et non dans les collecteurs : c'est la
        // seule façon qu'il existe pour *toutes* les cibles, quel que soit le type,
        // et donc que l'alerte « équipement injoignable » fonctionne partout.
        //
        // Rien n'est écrit en cas d'échec : la série s'interrompt, et c'est cette
        // interruption que la règle détecte. Émettre `up 0` demanderait de pouvoir
        // écrire depuis un chemin d'erreur qui, lui, ne produit aucun échantillon.
        samples.push(Sample::new(
            "up",
            1.0,
            MetricKind::Gauge,
            chrono::Utc::now().timestamp_millis(),
        ));

        let base_labels = target.base_labels();
        for sample in &mut samples {
            for (key, value) in &base_labels {
                // Les étiquettes d'identité priment sur celles du collecteur.
                sample.labels.insert(key.clone(), value.clone());
            }
        }
        Ok(samples)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un collecteur dont le type est choisi à l'exécution.
    struct Runtime(String);

    #[async_trait::async_trait]
    impl Collector for Runtime {
        fn kind(&self) -> &str {
            &self.0
        }

        async fn probe(&self, _target: &Target) -> Result<Vec<Sample>, ProbeError> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn un_type_ne_a_lexecution_sajoute_et_se_retire_sans_toucher_aux_types_compiles() {
        let mut registry = Registry::new();
        registry.register(Arc::new(DummyCollector));
        let shared = registry.clone();

        shared.register_runtime(Arc::new(Runtime("pack.demo".into()))).unwrap();
        assert!(registry.get("pack.demo").is_some(), "l'étage d'exécution est partagé");
        assert_eq!(registry.kinds().len(), 2);
        assert_eq!(registry.runtime_kinds(), [Arc::from("pack.demo")]);

        let builtin = DummyCollector.kind().to_string();
        assert!(shared.register_runtime(Arc::new(Runtime(builtin.clone()))).is_err());
        assert!(registry.get(&builtin).is_some());

        assert!(shared.unregister_runtime("pack.demo"));
        assert!(!shared.unregister_runtime("pack.demo"));
        assert!(registry.get("pack.demo").is_none());
    }
}
