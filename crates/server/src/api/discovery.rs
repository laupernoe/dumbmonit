//! Découverte d'équipements SNMP sur le réseau.
//!
//! C'est le raccourci qui évite de saisir les équipements un par un : on donne un
//! réseau, on obtient la liste de ce qui répond, avec le profil que
//! l'auto-détection appliquerait.
//!
//! Un balayage est une action, pas une lecture : il part en `POST`, avec ses
//! paramètres dans le corps. Le garde de session en fait de ce fait une opération
//! d'administrateur, et la community SNMP ne traîne ni dans les journaux du
//! serveur ni dans ceux d'un mandataire, contrairement à une chaîne de requête.

use axum::Json;
use axum::extract::State;
use dumbmonit_proto::Credential;
use serde::{Deserialize, Serialize};

use crate::api::{ApiError, ApiResult};
use crate::auth::middleware::AdminIdentity;
use crate::collectors::snmp::{DiscoveredDevice, ScanOptions, scan_network};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ScanRequest {
    /// Réseau à balayer, en notation CIDR : `192.168.1.0/24`.
    pub cidr: String,
    /// Community SNMP essayée sur chaque adresse. `public` par défaut, qui reste la
    /// valeur d'usine de la majorité des équipements.
    #[serde(default)]
    pub community: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    /// Délai par adresse, en millisecondes.
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

/// `Debug` écrit à la main : la community est un secret, elle ne doit pas finir
/// dans une trace de requête.
impl std::fmt::Debug for ScanRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ScanRequest {{ cidr: {:?}, community: <redacted>, port: {:?}, timeout_ms: {:?} }}",
            self.cidr, self.port, self.timeout_ms
        )
    }
}

#[derive(Debug, Serialize)]
pub struct ScanResponse {
    pub devices: Vec<DiscoveredDevice>,
    /// Adresses effectivement sondées, pour que l'interface puisse expliquer
    /// « 254 adresses testées, 3 équipements trouvés ».
    pub scanned: usize,
}

/// `POST /api/discovery` — balaie un réseau à la recherche d'équipements SNMP.
/// Réservé aux administrateurs : c'est le serveur qui sonde, vers n'importe quel
/// réseau qu'il peut joindre.
///
/// L'opération est bornée par les garde-fous de [`ScanOptions`] : un `/16` saisi par
/// mégarde est refusé plutôt que de lancer soixante-cinq mille sondes.
pub async fn scan(
    _: AdminIdentity,
    Json(request): Json<ScanRequest>,
) -> ApiResult<Json<ScanResponse>> {
    let network: ipnet::IpNet = request
        .cidr
        .trim()
        .parse()
        .map_err(|_| ApiError::BadRequest(format!("Invalid network \"{}\".", request.cidr)))?;

    let credential = Credential::SnmpCommunity {
        community: request.community.unwrap_or_else(|| "public".to_string()),
    };

    let mut options = ScanOptions::default();
    if let Some(port) = request.port {
        options.port = port;
    }
    if let Some(timeout_ms) = request.timeout_ms {
        options.timeout = std::time::Duration::from_millis(timeout_ms.clamp(100, 10_000));
    }

    // Le nombre d'hôtes est calculé avant le balayage : c'est ce qui permet de
    // refuser un réseau trop large avec un message utile plutôt qu'après coup.
    let scanned = network.hosts().take(options.max_hosts + 1).count();
    if scanned > options.max_hosts {
        return Err(ApiError::BadRequest(format!(
            "Network too large: {} addresses at most. Choose a narrower prefix.",
            options.max_hosts
        )));
    }

    let devices = scan_network(network, &credential, &options)
        .await
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;

    Ok(Json(ScanResponse { devices, scanned }))
}

/// D'où vient une suggestion de réseau.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubnetSource {
    /// Réglage explicite `DUMBMONIT_SCAN_SUBNETS`.
    Config,
    /// Déduit des adresses IPv4 privées des équipements déjà surveillés.
    Devices,
    /// Réseau relevé dans la table de routage du serveur.
    Server,
}

/// Réseau proposé comme valeur par défaut du balayage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LocalSubnet {
    pub cidr: String,
    pub source: SubnetSource,
    /// Interface du serveur, pour la seule source `server`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interface: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LocalSubnets {
    /// Du plus fiable au moins fiable : réglage explicite, équipements connus,
    /// puis routes du serveur. Vide si rien n'est sûr : l'interface laisse alors le
    /// champ libre plutôt que de proposer le réseau d'un pont Docker.
    pub subnets: Vec<LocalSubnet>,
}

/// `GET /api/discovery/subnets` — réseaux à proposer pour le balayage.
/// Lecture seule, réservée aux administrateurs : c'est une carte du réseau.
pub async fn local_subnets(
    _: AdminIdentity,
    State(state): State<AppState>,
) -> ApiResult<Json<LocalSubnets>> {
    let targets = crate::db::targets::list(&state.pool, &state.cipher).await?;
    let addresses: Vec<&str> = targets.iter().map(|t| t.address.as_str()).collect();
    // Hors Linux, ou sans `/proc` : pas de routes, ce n'est pas une erreur.
    let table = tokio::fs::read_to_string("/proc/net/route").await.unwrap_or_default();
    let in_container = std::path::Path::new("/.dockerenv").exists();
    Ok(Json(LocalSubnets {
        subnets: suggest_subnets(&state.config.scan_subnets, &addresses, &table, in_container),
    }))
}

/// Assemble les suggestions, sans doublon, dans l'ordre réglage > équipements > routes.
pub fn suggest_subnets(
    configured: &[ipnet::Ipv4Net],
    addresses: &[&str],
    route_table: &str,
    in_container: bool,
) -> Vec<LocalSubnet> {
    let mut out: Vec<LocalSubnet> = Vec::new();
    let mut push = |cidr: String, source, interface| {
        if !out.iter().any(|s| s.cidr == cidr) {
            out.push(LocalSubnet { cidr, source, interface });
        }
    };
    for net in configured {
        push(net.to_string(), SubnetSource::Config, None);
    }
    for cidr in subnets_from_addresses(addresses) {
        push(cidr, SubnetSource::Devices, None);
    }
    // Dans un conteneur, les routes connectées sont celles du réseau du conteneur
    // (pont Compose en 10.x ou 192.168.x, réseau de pod…), pas le réseau local de
    // l'utilisateur : rien n'est proposé d'après elles. Le réglage et les équipements
    // connus restent les seules sources sûres.
    if !in_container {
        for (cidr, interface) in subnets_from_route_table(route_table, false) {
            push(cidr, SubnetSource::Server, Some(interface));
        }
    }
    out
}

/// Adresse IPv4 littérale de l'adresse d'un équipement : IP seule, `hôte:port`
/// ou URL. Un nom d'hôte ne donne rien (pas de résolution DNS ici).
fn literal_ipv4(address: &str) -> Option<std::net::Ipv4Addr> {
    let rest = address.trim();
    let rest = rest.split_once("://").map_or(rest, |(_, after)| after);
    let rest = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    let rest = rest.rsplit('@').next().unwrap_or(rest);
    let host = rest.split(':').next().unwrap_or(rest);
    host.parse().ok()
}

/// Regroupe en /24 les IPv4 privées des équipements ; les plus peuplés d'abord,
/// à égalité l'ordre numérique.
pub fn subnets_from_addresses(addresses: &[&str]) -> Vec<String> {
    let mut counts: std::collections::BTreeMap<u32, usize> = std::collections::BTreeMap::new();
    for ip in addresses.iter().filter_map(|a| literal_ipv4(a)) {
        if ip.is_private() {
            *counts.entry(u32::from(ip) & (u32::MAX << 8)).or_default() += 1;
        }
    }
    let mut groups: Vec<(u32, usize)> = counts.into_iter().collect();
    groups.sort_by_key(|g| std::cmp::Reverse(g.1));
    groups.into_iter().map(|(net, _)| format!("{}/24", std::net::Ipv4Addr::from(net))).collect()
}

/// Plus large préfixe proposé tel quel : au-delà de 4096 adresses, le balayage
/// refuserait le réseau (voir `ScanOptions::max_hosts`).
const WIDEST_PREFIX: u32 = 20;
/// Préfixe de repli pour un réseau plus large (un `/16` de pont Docker) : le premier
/// `/24`, qui reste modifiable.
const FALLBACK_PREFIX: u32 = 24;

/// Extrait les réseaux connectés de la table de routage du noyau
/// (`/proc/net/route`) : les routes sans passerelle, hors boucle locale et
/// lien-local. Aucune dépendance ni appel système : c'est la même information que
/// celle d'`ip route`.
pub fn subnets_from_route_table(table: &str, in_container: bool) -> Vec<(String, String)> {
    let mut found: Vec<(std::net::Ipv4Addr, u32, String)> = Vec::new();
    for line in table.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 8 {
            continue;
        }
        let parse = |hex: &str| u32::from_str_radix(hex, 16).ok();
        let (Some(destination), Some(gateway), Some(mask)) =
            (parse(fields[1]), parse(fields[2]), parse(fields[7]))
        else {
            continue;
        };
        // Le noyau écrit les adresses en hexadécimal, dans l'ordre des octets de
        // l'hôte (petit-boutiste sur toutes les plates-formes visées).
        let destination = std::net::Ipv4Addr::from(destination.swap_bytes());
        let mask = mask.swap_bytes();
        // Un masque non contigu n'est pas un préfixe ; une route par défaut (masque
        // nul) ou à passerelle n'est pas un réseau connecté.
        if gateway != 0 || mask == 0 || mask.leading_ones() + mask.trailing_zeros() != 32 {
            continue;
        }
        let mut prefix = mask.leading_ones();
        if destination.is_loopback() || destination.is_link_local() || prefix > 30 {
            continue;
        }
        // Ponts créés par Docker : jamais le réseau de l'utilisateur. Dans un conteneur,
        // tout le bloc 172.16/12 est suspect (Docker y puise ses plages).
        let docker_interface = ["docker", "br-", "veth"].iter().any(|p| fields[0].starts_with(p));
        if docker_interface
            || (in_container
                && destination.octets()[0] == 172
                && (16..=31).contains(&destination.octets()[1]))
        {
            continue;
        }
        let mut network = destination;
        if prefix < WIDEST_PREFIX {
            prefix = FALLBACK_PREFIX;
            network = std::net::Ipv4Addr::from(u32::from(destination) & (u32::MAX << 8));
        }
        if found.iter().any(|(n, p, _)| *n == network && *p == prefix) {
            continue;
        }
        found.push((network, prefix, fields[0].to_string()));
    }
    // Tri stable : à rang égal, l'ordre du noyau est conservé.
    found.sort_by_key(|(network, _, _)| match network.octets() {
        [192, 168, ..] => 0,
        [10, ..] => 1,
        [172, 16..=31, ..] => 3,
        _ => 2,
    });
    found
        .into_iter()
        .map(|(network, prefix, interface)| (format!("{network}/{prefix}"), interface))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: &str =
        "Iface\tDestination\tGateway\tFlags\tRefCnt\tUse\tMetric\tMask\tMTU\tWindow\tIRTT
eth0\t00000000\t0100A8C0\t0003\t0\t0\t100\t00000000\t0\t0\t0
docker0\t000011AC\t00000000\t0001\t0\t0\t0\t0000FFFF\t0\t0\t0
eth0\t0000A8C0\t00000000\t0001\t0\t0\t100\t00FFFFFF\t0\t0\t0
lo\t0000007F\t00000000\t0001\t0\t0\t0\t000000FF\t0\t0\t0
eth1\t0000FEA9\t00000000\t0001\t0\t0\t0\t0000FFFF\t0\t0\t0
";

    #[test]
    fn route_table_keeps_connected_networks_without_docker_bridges() {
        let cidrs = subnets_from_route_table(TABLE, false);
        // Pont docker0 écarté ; passerelle par défaut, boucle locale et lien-local aussi.
        assert_eq!(cidrs, vec![("192.168.0.0/24".to_string(), "eth0".to_string())]);
    }

    #[test]
    fn route_table_in_container_drops_the_whole_172_block() {
        let table =
            "Iface\tDestination\tGateway\tFlags\tRefCnt\tUse\tMetric\tMask\tMTU\tWindow\tIRTT
eth0\t000011AC\t00000000\t0001\t0\t0\t0\t0000FFFF\t0\t0\t0
eth1\t0000140A\t00000000\t0001\t0\t0\t0\t0000FFFF\t0\t0\t0
";
        // eth0 = 172.17.0.0/16 : seul le bloc 172 est écarté.
        assert_eq!(subnets_from_route_table(table, true).len(), 1);
        assert_eq!(subnets_from_route_table(table, false).len(), 2);
        // Seulement Docker : aucune suggestion.
        assert!(subnets_from_route_table(TABLE, true).len() == 1);
        assert!(suggest_subnets(&[], &[], &TABLE.replace("0000A8C0", "000011AC"), true).is_empty());
        // Dans un conteneur, même un réseau 192.168.x vu par le serveur n'est pas proposé :
        // c'est celui du conteneur, pas forcément celui de l'utilisateur.
        assert!(suggest_subnets(&[], &[], TABLE, true).is_empty());
        let known = suggest_subnets(&[], &["192.168.7.2"], TABLE, true);
        assert_eq!(known.len(), 1);
        assert_eq!(known[0].source, SubnetSource::Devices);
    }

    #[test]
    fn route_table_garbage_gives_nothing() {
        assert!(subnets_from_route_table("", false).is_empty());
        assert!(subnets_from_route_table("Iface\nx y z", false).is_empty());
    }

    #[test]
    fn addresses_group_by_24_most_devices_first() {
        let addresses = [
            "10.0.5.7",
            "192.168.1.10",
            "192.168.1.11:161",
            "https://192.168.1.12/status",
            "http://admin@192.168.1.13:8006/x",
            "nas.local",
            "8.8.8.8",
            "172.17.0.2",
            "169.254.1.1",
        ];
        assert_eq!(
            subnets_from_addresses(&addresses),
            vec!["192.168.1.0/24", "10.0.5.0/24", "172.17.0.0/24"]
        );
    }

    #[test]
    fn suggestions_are_ordered_by_source_and_deduplicated() {
        let configured = crate::config::parse_scan_subnets("10.1.0.0/16, bogus, 192.168.1.5/24");
        assert_eq!(configured.len(), 2);
        let got = suggest_subnets(&configured, &["192.168.1.9", "192.168.7.2"], TABLE, false);
        let view: Vec<_> = got.iter().map(|s| (s.cidr.as_str(), s.source)).collect();
        assert_eq!(
            view,
            vec![
                ("10.1.0.0/16", SubnetSource::Config),
                ("192.168.1.0/24", SubnetSource::Config),
                ("192.168.7.0/24", SubnetSource::Devices),
                ("192.168.0.0/24", SubnetSource::Server),
            ]
        );
        assert_eq!(got[3].interface.as_deref(), Some("eth0"));
        assert_eq!(got[0].interface, None);
    }

    #[test]
    fn debug_never_leaks_the_community() {
        let request = ScanRequest {
            cidr: "192.168.1.0/24".into(),
            community: Some("s3cr3t-community".into()),
            port: None,
            timeout_ms: None,
        };
        let rendered = format!("{request:?}");
        assert!(!rendered.contains("s3cr3t"), "{rendered}");
        assert!(rendered.contains("<redacted>"), "{rendered}");
    }
}
