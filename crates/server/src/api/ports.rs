//! Ports réseau à ouvrir (Réglages › Système › Network ports).
//!
//! `GET /security/ports` — administrateur seulement. La liste est calculée depuis
//! la configuration réelle (variables d'environnement lues au démarrage et
//! machines relais connues), jamais écrite en dur : un écouteur désactivé est
//! rendu `off`, un écouteur lié à la boucle locale `local`, et seuls les ports
//! réellement écoutés entrent dans les blocs `docker-compose` et `ufw`.
//!
//! Écouteurs du serveur : l'interface et l'API en HTTPS (`DUMBMONIT_BIND`), le
//! canal des agents (`DUMBMONIT_AGENT_LISTEN`), syslog TCP et UDP, les traps SNMP,
//! les stockages embarqués (VictoriaMetrics, long terme, VictoriaLogs) et, sur
//! chaque machine relais, la passerelle de son site.

use std::net::{IpAddr, SocketAddr};

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use crate::api::ApiResult;
use crate::auth::middleware::AdminIdentity;
use crate::collectors::agent as agent_store;
use crate::logs::{central, syslog, traps};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/security/ports", get(overview))
}

/// Un port, avec ce que l'interface en dit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PortEntry {
    /// Identifiant stable : l'interface y associe le texte (un identifiant
    /// inconnu s'affiche tel quel).
    pub id: String,
    /// Port d'écoute dans le conteneur (ou sur la machine relais).
    pub port: u16,
    /// Port à ouvrir côté hôte (celui publié par le mappage Docker).
    pub host_port: u16,
    /// `tcp` ou `udp`.
    pub protocol: &'static str,
    /// `lan`, `wan_conditional` ou `never`.
    pub exposure: &'static str,
    /// `listening`, `off` (désactivé : rien à ouvrir) ou `local` (boucle locale).
    pub status: &'static str,
    /// Adresse d'écoute.
    pub bind: String,
    /// Nom du relais pour la passerelle d'un relais.
    pub subject: Option<String>,
    /// Faux : le port est sur une autre machine (relais), pas sur cet hôte.
    pub on_this_host: bool,
}

#[derive(Debug, Serialize)]
pub struct PortsView {
    pub ports: Vec<PortEntry>,
    /// Bloc `ports:` du docker-compose pour les ports écoutés et publiables.
    pub compose: String,
    /// Règles `ufw` pour les mêmes ports, avec un réseau à remplacer.
    pub ufw: String,
}

/// Écoute d'un récepteur de journaux, déjà lue dans l'environnement.
#[derive(Debug, Clone)]
pub struct LogInput {
    pub enabled: bool,
    pub bind: IpAddr,
    pub port: u16,
}

/// Tout ce dont la liste dépend. Pas d'accès à l'environnement ni au réseau
/// dans [`build`] : c'est ce qui la rend testable.
#[derive(Debug, Clone)]
pub struct Inputs {
    pub web: SocketAddr,
    /// Écoute et port publié du canal des agents, `None` s'il est coupé.
    pub agent: Option<(SocketAddr, u16)>,
    pub syslog: LogInput,
    pub syslog_udp: bool,
    pub syslog_tcp: bool,
    pub traps: LogInput,
    /// Adresses d'écoute des stockages embarqués (`host:port`), si lancés par ce serveur.
    pub metrics_store: Option<String>,
    pub long_store: Option<String>,
    pub logs_store: Option<String>,
    /// Relais dont la passerelle écoute : (nom, port).
    pub relay_gateways: Vec<(String, u16)>,
}

fn status_of(ip: IpAddr) -> &'static str {
    if ip.is_loopback() { "local" } else { "listening" }
}

/// Sépare `hôte:port` ; l'hôte vide, `0.0.0.0` et `::` valent « toutes les interfaces ».
fn split_listen(listen: &str) -> Option<(String, u16)> {
    let (host, port) = listen.trim().rsplit_once(':')?;
    let port = port.parse::<u16>().ok()?;
    Some((host.trim_matches(|c| c == '[' || c == ']').to_string(), port))
}

fn host_is_loopback(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

fn entry(
    id: &str,
    port: u16,
    protocol: &'static str,
    exposure: &'static str,
    status: &'static str,
    bind: String,
) -> PortEntry {
    PortEntry {
        id: id.into(),
        port,
        host_port: port,
        protocol,
        exposure,
        status,
        bind,
        subject: None,
        on_this_host: true,
    }
}

/// Un stockage embarqué n'est listé que s'il écoute ailleurs que sur la boucle
/// locale (c'est alors une ouverture involontaire à signaler).
fn store(id: &str, listen: &Option<String>) -> Option<PortEntry> {
    let (host, port) = split_listen(listen.as_deref()?)?;
    if host_is_loopback(&host) {
        return None;
    }
    let bind = if host.is_empty() { "0.0.0.0".to_string() } else { host };
    Some(entry(id, port, "tcp", "never", "listening", bind))
}

/// Construit la liste et les deux blocs à copier.
pub fn build(inputs: &Inputs) -> PortsView {
    let mut ports = Vec::new();

    ports.push(entry(
        "web",
        inputs.web.port(),
        "tcp",
        "wan_conditional",
        status_of(inputs.web.ip()),
        inputs.web.ip().to_string(),
    ));

    if let Some((listen, public)) = inputs.agent {
        let mut agents = entry(
            "agents",
            listen.port(),
            "tcp",
            "wan_conditional",
            status_of(listen.ip()),
            listen.ip().to_string(),
        );
        agents.host_port = public;
        ports.push(agents);
    }

    let log_status = |input: &LogInput, on: bool| {
        if input.enabled && on { status_of(input.bind) } else { "off" }
    };
    ports.push(entry(
        "syslog_udp",
        inputs.syslog.port,
        "udp",
        "lan",
        log_status(&inputs.syslog, inputs.syslog_udp),
        inputs.syslog.bind.to_string(),
    ));
    ports.push(entry(
        "syslog_tcp",
        inputs.syslog.port,
        "tcp",
        "lan",
        log_status(&inputs.syslog, inputs.syslog_tcp),
        inputs.syslog.bind.to_string(),
    ));
    ports.push(entry(
        "traps",
        inputs.traps.port,
        "udp",
        "lan",
        log_status(&inputs.traps, true),
        inputs.traps.bind.to_string(),
    ));

    ports.extend(store("metrics_store", &inputs.metrics_store));
    ports.extend(store("long_store", &inputs.long_store));
    ports.extend(store("logs_store", &inputs.logs_store));

    for (name, port) in &inputs.relay_gateways {
        let mut gateway = entry("relay_gateway", *port, "tcp", "lan", "listening", String::new());
        gateway.subject = Some(name.clone());
        gateway.on_this_host = false;
        ports.push(gateway);
    }

    let (compose, ufw) = blocks(&ports);
    PortsView { ports, compose, ufw }
}

/// Un port entre dans les blocs s'il est écouté ici et qu'on peut l'ouvrir
/// sans danger (jamais un stockage interne).
fn publishable(port: &PortEntry) -> bool {
    port.status == "listening" && port.on_this_host && port.exposure != "never"
}

fn blocks(ports: &[PortEntry]) -> (String, String) {
    let mut compose = String::from("ports:\n");
    let mut ufw =
        String::from("# Replace 192.168.1.0/24 with the network that must reach DumbMonit\n");
    for port in ports.iter().filter(|p| publishable(p)) {
        let suffix = if port.protocol == "udp" { "/udp" } else { "" };
        compose.push_str(&format!("  - \"{}:{}{}\"\n", port.host_port, port.port, suffix));
        ufw.push_str(&format!(
            "ufw allow from 192.168.1.0/24 to any port {} proto {}\n",
            port.host_port, port.protocol
        ));
    }
    (compose, ufw)
}

/// Lit la configuration réelle du processus.
async fn inputs(state: &AppState) -> ApiResult<Inputs> {
    let config = &state.config;
    let log_input = |flag: &str, bind: &str, port: &str, default: u16| LogInput {
        enabled: central::env_flag(flag, false),
        bind: central::env_bind(bind),
        port: central::env_port(port, default),
    };
    let relay_gateways = agent_store::list_hosts(&state.pool)
        .await?
        .into_iter()
        .filter_map(|(_, name, host)| host.relay_gateway_port.map(|port| (name, port)))
        .collect();
    Ok(Inputs {
        web: config.bind,
        agent: config
            .agent_listen
            .filter(|_| !config.demo)
            .map(|listen| (listen, config.agent_public_port.unwrap_or(listen.port()))),
        syslog: log_input(
            "DUMBMONIT_SYSLOG_ENABLED",
            "DUMBMONIT_SYSLOG_BIND",
            "DUMBMONIT_SYSLOG_PORT",
            syslog::DEFAULT_PORT,
        ),
        syslog_udp: central::env_flag("DUMBMONIT_SYSLOG_UDP", true),
        syslog_tcp: central::env_flag("DUMBMONIT_SYSLOG_TCP", true),
        traps: log_input(
            "DUMBMONIT_TRAPS_ENABLED",
            "DUMBMONIT_TRAPS_BIND",
            "DUMBMONIT_TRAPS_PORT",
            traps::DEFAULT_PORT,
        ),
        metrics_store: config.vm_embedded().then(|| config.vm_listen.clone()),
        long_store: config.vm_long_embedded().then(|| config.vm_long_listen.clone()),
        logs_store: Some(config.vlogs_listen.clone()),
        relay_gateways,
    })
}

/// `GET /api/security/ports`
async fn overview(_: AdminIdentity, State(state): State<AppState>) -> ApiResult<Json<PortsView>> {
    Ok(Json(build(&inputs(&state).await?)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Inputs {
        Inputs {
            web: "0.0.0.0:8080".parse().unwrap(),
            agent: Some(("0.0.0.0:8443".parse().unwrap(), 8443)),
            syslog: LogInput { enabled: false, bind: "0.0.0.0".parse().unwrap(), port: 5514 },
            syslog_udp: true,
            syslog_tcp: true,
            traps: LogInput { enabled: false, bind: "0.0.0.0".parse().unwrap(), port: 1162 },
            metrics_store: Some("127.0.0.1:8428".into()),
            long_store: Some("127.0.0.1:8429".into()),
            logs_store: Some("127.0.0.1:9428".into()),
            relay_gateways: vec![],
        }
    }

    fn find<'a>(view: &'a PortsView, id: &str) -> &'a PortEntry {
        view.ports.iter().find(|p| p.id == id).unwrap_or_else(|| panic!("port {id} absent"))
    }

    #[test]
    fn defaults_list_web_and_agents_and_mark_log_inputs_off() {
        let view = build(&base());
        assert_eq!(find(&view, "web").port, 8080);
        assert_eq!(find(&view, "agents").port, 8443);
        assert_eq!(find(&view, "syslog_udp").status, "off");
        assert_eq!(find(&view, "syslog_tcp").status, "off");
        assert_eq!(find(&view, "traps").status, "off");
        // Les stockages sur la boucle locale ne sont pas listés.
        assert!(view.ports.iter().all(|p| !p.id.ends_with("_store")));
        assert_eq!(view.compose, "ports:\n  - \"8080:8080\"\n  - \"8443:8443\"\n");
        assert!(view.ufw.contains("ufw allow from 192.168.1.0/24 to any port 8080 proto tcp\n"));
        assert!(!view.ufw.contains("5514"));
    }

    #[test]
    fn enabled_syslog_and_traps_join_the_blocks_with_their_protocol() {
        let mut inputs = base();
        inputs.syslog.enabled = true;
        inputs.syslog_tcp = false;
        inputs.traps.enabled = true;
        inputs.traps.port = 2162;
        let view = build(&inputs);
        assert_eq!(find(&view, "syslog_udp").status, "listening");
        assert_eq!(find(&view, "syslog_tcp").status, "off");
        assert_eq!(find(&view, "traps").port, 2162);
        assert!(view.compose.contains("\"5514:5514/udp\""));
        assert!(!view.compose.contains("\"5514:5514\""));
        assert!(view.compose.contains("\"2162:2162/udp\""));
        assert!(view.ufw.contains("port 5514 proto udp"));
    }

    #[test]
    fn the_agent_channel_is_absent_when_off_and_uses_the_published_port() {
        let mut inputs = base();
        inputs.agent = None;
        assert!(build(&inputs).ports.iter().all(|p| p.id != "agents"));
        inputs.agent = Some(("0.0.0.0:8443".parse().unwrap(), 9443));
        let view = build(&inputs);
        assert_eq!(find(&view, "agents").host_port, 9443);
        assert!(view.compose.contains("\"9443:8443\""));
    }

    #[test]
    fn a_loopback_listener_has_nothing_to_open() {
        let mut inputs = base();
        inputs.web = "127.0.0.1:8080".parse().unwrap();
        inputs.syslog.enabled = true;
        inputs.syslog.bind = "127.0.0.1".parse().unwrap();
        let view = build(&inputs);
        assert_eq!(find(&view, "web").status, "local");
        assert_eq!(find(&view, "syslog_udp").status, "local");
        assert!(!view.compose.contains("8080"));
        assert!(!view.compose.contains("5514"));
    }

    #[test]
    fn stores_appear_only_off_loopback_and_are_never_published() {
        let mut inputs = base();
        inputs.metrics_store = Some("0.0.0.0:8428".into());
        inputs.logs_store = Some(":9428".into());
        inputs.long_store = None;
        let view = build(&inputs);
        let metrics = find(&view, "metrics_store");
        assert_eq!((metrics.port, metrics.exposure), (8428, "never"));
        assert_eq!(find(&view, "logs_store").bind, "0.0.0.0");
        assert!(view.ports.iter().all(|p| p.id != "long_store"));
        assert!(!view.compose.contains("8428") && !view.ufw.contains("9428"));
    }

    #[test]
    fn a_relay_gateway_is_on_another_machine_and_stays_out_of_the_blocks() {
        let mut inputs = base();
        inputs.relay_gateways = vec![("Branch office".into(), 8443)];
        let view = build(&inputs);
        let gateway = find(&view, "relay_gateway");
        assert_eq!(gateway.subject.as_deref(), Some("Branch office"));
        assert!(!gateway.on_this_host);
        // Seul le port des agents (8443:8443) figure dans le bloc, pas la passerelle.
        assert_eq!(view.compose.matches("\"8443:8443\"").count(), 1);
    }

    #[test]
    fn listen_addresses_are_split_and_classified() {
        assert_eq!(split_listen("[::1]:8428"), Some(("::1".into(), 8428)));
        assert_eq!(split_listen(":9428"), Some((String::new(), 9428)));
        assert_eq!(split_listen("nonsense"), None);
        assert!(host_is_loopback("127.0.0.1") && host_is_loopback("::1"));
        assert!(!host_is_loopback("") && !host_is_loopback("10.0.0.2"));
    }
}
