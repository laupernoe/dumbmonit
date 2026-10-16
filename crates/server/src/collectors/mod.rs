//! Registre des collecteurs.
//!
//! Les collecteurs réseau (SNMP, Proxmox, PBS, PDM, Synology, sondes de disponibilité)
//! vivent dans la caisse `dumbmonit-collectors`, partagée avec l'agent relais ;
//! ils sont réexportés ici sous leurs anciens chemins (`collectors::proxmox`, …)
//! pour que le reste du serveur n'ait pas à savoir où ils habitent. Le collecteur
//! « agent » (fraîcheur des mesures poussées, commandes, jetons) reste ici : il
//! lit la base.
//!
//! Ajouter une intégration se résume à implémenter [`Collector`] et à l'enregistrer
//! dans le [`Registry`] : le planificateur et l'API n'ont pas connaissance des
//! types concrets.

pub mod activedirectory_history;
pub mod agent;
pub mod opnsense_history;
pub mod pbs_history;
pub mod pdm_history;
pub mod pmg_history;
pub mod push;
pub mod relay;
pub mod synology_history;
pub mod truenas_history;

/// Antériorité maximale d'un échantillon accepté.
///
/// Un agent rattrape au plus quelques heures de coupure. Au-delà, l'horloge de la
/// machine est fausse, et écrire ces points polluerait durablement les graphes
/// d'une période déjà passée.
pub(crate) const MAX_BACKFILL_MS: i64 = 30 * 24 * 3_600 * 1_000;

/// Avance maximale tolérée sur l'horloge du serveur.
pub(crate) const MAX_SKEW_AHEAD_MS: i64 = 3_600 * 1_000;

pub use dumbmonit_collectors::{ActiveDirectoryCollector, activedirectory};
pub use dumbmonit_collectors::{
    AdguardCollector, MikrotikCollector, NutCollector, PiholeCollector, adguard, mikrotik, nut,
    pihole,
};
pub use dumbmonit_collectors::{
    CaddyCollector, DomainCollector, NpmCollector, TraefikCollector, caddy, domain, npm, traefik,
};
pub use dumbmonit_collectors::{
    CrowdsecCollector, MongodbCollector, RabbitmqCollector, RedisCollector, crowdsec, mongodb,
    rabbitmq, redis,
};
pub use dumbmonit_collectors::{
    DnsCollector, DummyCollector, HttpCollector, OpnsenseCollector, PbsCollector, PdmCollector,
    PingCollector, PmgCollector, ProxmoxCollector, Registry, SnmpCollector, SynologyCollector,
    TcpCollector, TlsCollector, TruenasCollector, dummy, http, opnsense, pbs, pdm, pmg, proxmox,
    snmp, synology, truenas, uptime,
};
pub use dumbmonit_collectors::{
    ForgejoCollector, GitlabCollector, ImmichCollector, JellyfinCollector, NextcloudCollector,
    PaperlessCollector, PlexCollector, selfhosted,
};
pub use dumbmonit_collectors::{
    FortigateCollector, PfsenseCollector, SophosCollector, TailscaleCollector, UnraidCollector,
    VeeamCollector, fortigate, pfsense, sophos, tailscale, unraid, veeam,
};
pub use dumbmonit_collectors::{GraylogCollector, LokiCollector, VictoriaCollector, observability};
pub use dumbmonit_collectors::{
    HomeAssistantCollector, UnifiCollector, VsphereCollector, homeassistant, unifi, vsphere,
};
pub use dumbmonit_collectors::{KubernetesCollector, kubernetes};
pub use dumbmonit_collectors::{MdaemonCollector, SecurityGatewayCollector, mdaemon};
pub use dumbmonit_collectors::{
    MqttCollector, MysqlCollector, PostgresCollector, SmtpCollector, WebsocketCollector,
};
pub use dumbmonit_collectors::{OpenvpnCollector, openvpn};
pub use dumbmonit_collectors::{RedfishCollector, redfish};
#[allow(unused_imports)]
pub use dumbmonit_proto::Collector;

pub use agent::{AgentCollector, AgentServiceCollector};
pub use push::PushCollector;

/// Le registre des collecteurs compilés du serveur.
///
/// Il part de [`Registry::remote`], la liste partagée avec l'agent relais, pour
/// que les deux ne puissent plus diverger : un collecteur ajouté là est connu du
/// serveur sans autre retouche. Ne sont remplacés ici que les collecteurs qui
/// gardent un historique en base (même type, `register` écrase), et ne sont
/// ajoutés que ceux qui lisent la base du serveur : agents, moniteurs en
/// poussée, changements de site, démonstration.
///
/// Les profils SNMP des paquets activés sont fusionnés au catalogue ; les types
/// que ces paquets définissent, eux, sont activés par l'appelant, dans l'étage
/// d'exécution du registre.
pub fn builtin(
    pool: &sqlx::SqlitePool,
    config: &crate::config::Config,
    packs: &[std::sync::Arc<dumbmonit_pack::Pack>],
) -> Registry {
    use std::sync::Arc;

    let mut registry = Registry::remote(config.probe_timeout);

    let snmp = SnmpCollector::new().with_probe_budget(config.probe_timeout);
    let snmp = match crate::packs::snmp_catalog(snmp.catalog(), packs) {
        Some(catalog) => snmp.with_catalog(Arc::new(catalog)),
        None => snmp,
    };
    registry.register(Arc::new(snmp));

    // Collecteurs qui conservent leur vue pour la page de l'équipement.
    registry.register(Arc::new(
        PbsCollector::new().with_observer(pbs_history::sqlite_observer(pool.clone())),
    ));
    registry.register(Arc::new(
        PdmCollector::new().with_observer(pdm_history::sqlite_observer(pool.clone())),
    ));
    registry.register(Arc::new(
        PmgCollector::new().with_observer(pmg_history::sqlite_observer(pool.clone())),
    ));
    registry.register(Arc::new(
        SynologyCollector::new().with_abb_history(synology_history::sqlite_history(pool.clone())),
    ));
    registry.register(Arc::new(
        OpnsenseCollector::new().with_observer(opnsense_history::sqlite_observer(pool.clone())),
    ));
    registry.register(Arc::new(
        TruenasCollector::new().with_observer(truenas_history::sqlite_observer(pool.clone())),
    ));
    registry.register(Arc::new(
        ActiveDirectoryCollector::new()
            .with_observer(activedirectory_history::sqlite_observer(pool.clone())),
    ));

    // Machines équipées de l'agent : les mesures arrivent en push, ce collecteur
    // ne fait que constater leur fraîcheur.
    registry.register(Arc::new(AgentCollector::new(pool.clone())));
    // Services qu'un agent a détectés (Docker, ZFS…), chacun sous son hôte :
    // même contrôle de fraîcheur, par module.
    registry.register(Arc::new(AgentServiceCollector::new(pool.clone())));
    // Moniteurs en poussée (heartbeat) : le travail surveillé appelle une URL.
    registry.register(Arc::new(PushCollector::new(pool.clone())));
    // Changements du contenu d'un site : texte, instantanés et captures, gardés
    // en base et sous `<data>/webchange`.
    registry.register(Arc::new(crate::webchange::WebchangeCollector::new(
        crate::webchange::Context {
            pool: pool.clone(),
            data_dir: config.data_dir.clone(),
            browser_url: config.browser_url.clone(),
        },
    )));
    // Santé et référencement d'un site : référence des métadonnées et
    // historique des changements gardés en base.
    registry.register(Arc::new(
        crate::website::WebsiteCollector::new(pool.clone())
            .with_change_check(config.website.change_check),
    ));
    // Collecteur de démonstration : des graphes sans matériel, le temps de
    // configurer un premier équipement réel.
    registry.register(Arc::new(DummyCollector));
    declare_options(&mut registry);
    registry
}

/// Déclare au registre les options de chacun de ses types compilés, d'après les
/// tables de `api/collectors.rs` : elles ne seront pas recopiées en étiquettes.
/// Clé par type : remplacer un collecteur (démonstration) garde ses options.
fn declare_options(registry: &mut Registry) {
    for kind in registry.kinds() {
        let keys = crate::api::collector_option_keys(&kind);
        if !keys.is_empty() {
            registry.set_options(&kind, keys);
        }
    }
}
