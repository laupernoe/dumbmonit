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
use dumbmonit_proto::Credential;
use serde::{Deserialize, Serialize};

use crate::api::{ApiError, ApiResult};
use crate::auth::middleware::AdminIdentity;
use crate::collectors::snmp::{DiscoveredDevice, ScanOptions, scan_network};

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

/// Réseau directement connecté au serveur, proposé comme valeur par défaut du
/// balayage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LocalSubnet {
    pub cidr: String,
    pub interface: String,
}

#[derive(Debug, Serialize)]
pub struct LocalSubnets {
    /// Du plus probable au moins probable : réseaux privés « maison » d'abord,
    /// puis le reste (les ponts Docker, en 172.x, ferment la marche).
    pub subnets: Vec<LocalSubnet>,
}

/// `GET /api/discovery/subnets` — réseaux IPv4 directement connectés au serveur.
/// Lecture seule, réservée aux administrateurs : c'est une carte du réseau.
pub async fn local_subnets(_: AdminIdentity) -> Json<LocalSubnets> {
    // Hors Linux, ou sans `/proc` : liste vide, l'interface laisse le champ libre.
    let table = tokio::fs::read_to_string("/proc/net/route").await.unwrap_or_default();
    Json(LocalSubnets { subnets: subnets_from_route_table(&table) })
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
pub fn subnets_from_route_table(table: &str) -> Vec<LocalSubnet> {
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
        .map(|(network, prefix, interface)| LocalSubnet {
            cidr: format!("{network}/{prefix}"),
            interface,
        })
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
    fn route_table_keeps_connected_networks_lan_first() {
        let subnets = subnets_from_route_table(TABLE);
        let cidrs: Vec<_> =
            subnets.iter().map(|s| (s.cidr.as_str(), s.interface.as_str())).collect();
        // 192.168.0.0/24 d'abord ; le /16 du pont Docker est ramené à un /24 ;
        // passerelle par défaut, boucle locale et lien-local écartés.
        assert_eq!(cidrs, vec![("192.168.0.0/24", "eth0"), ("172.17.0.0/24", "docker0")]);
    }

    #[test]
    fn route_table_garbage_gives_nothing() {
        assert!(subnets_from_route_table("").is_empty());
        assert!(subnets_from_route_table("Iface\nx y z").is_empty());
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
