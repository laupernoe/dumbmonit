//! Diff d'inventaire : ce qu'un balayage a trouvé, rapproché de ce qu'on connaît.
//!
//! Pur : aucune base, aucun réseau. Le magasin ([`super::store`]) lui remet
//! l'inventaire existant et applique le plan rendu.

use std::collections::HashSet;

use dumbmonit_proto::{DiscoveredHost, Transport};

/// Un appareil de l'inventaire, tel que le magasin le relit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownDevice {
    pub id: i64,
    pub ident: String,
    pub ip: String,
    /// Ports connus : `(port, protocole)`.
    pub ports: Vec<(u16, String)>,
}

/// Un port vu pendant le balayage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedPort {
    pub port: u16,
    pub protocol: String,
    pub product: String,
    /// Absent de l'inventaire jusqu'ici.
    pub is_new: bool,
}

/// Un appareil vu pendant le balayage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedDevice {
    /// Ligne d'inventaire correspondante ; `None` : appareil nouveau.
    pub existing_id: Option<i64>,
    pub ident: String,
    pub mac: Option<String>,
    pub ip: String,
    pub hostname: Option<String>,
    pub vendor: Option<String>,
    pub is_new: bool,
    pub ports: Vec<PlannedPort>,
}

/// MAC normalisée : minuscules, `:` comme séparateur. `None` si vide.
pub fn normalize_mac(raw: &str) -> Option<String> {
    let mac = raw.trim().to_lowercase().replace('-', ":");
    (!mac.is_empty()).then_some(mac)
}

/// Identité d'un appareil : sa MAC si elle est visible, sinon son IP.
pub fn ident_of(mac: Option<&str>, ip: &str) -> String {
    match mac {
        Some(mac) => format!("mac:{mac}"),
        None => format!("ip:{ip}"),
    }
}

fn protocol_name(transport: Transport) -> &'static str {
    match transport {
        Transport::Tcp => "tcp",
        Transport::Udp => "udp",
    }
}

/// Rapproche les hôtes d'un balayage de l'inventaire.
///
/// - un hôte à MAC visible retrouve sa ligne par cette MAC, même si son IP a
///   changé ; à défaut, la ligne identifiée par la seule IP est promue (la MAC
///   n'était simplement pas visible aux balayages précédents) ;
/// - un hôte sans MAC retrouve la ligne qui a cette IP, que sa ligne soit
///   identifiée par MAC ou par IP (l'ARP peut manquer sans que l'appareil soit
///   nouveau) ;
/// - une ligne ne sert qu'une fois par balayage.
pub fn plan(known: &[KnownDevice], hosts: &[DiscoveredHost]) -> Vec<PlannedDevice> {
    let mut used: HashSet<i64> = HashSet::new();
    let mut idents: HashSet<String> = HashSet::new();
    let mut out: Vec<PlannedDevice> = Vec::new();
    for host in hosts {
        let mac = host.mac.as_deref().and_then(normalize_mac);
        let ip = host.ip.trim().to_string();
        let ident = ident_of(mac.as_deref(), &ip);
        let ip_ident = ident_of(None, &ip);

        let found = match &mac {
            Some(_) => known
                .iter()
                .find(|d| d.ident == ident && !used.contains(&d.id))
                .or_else(|| known.iter().find(|d| d.ident == ip_ident && !used.contains(&d.id))),
            None => known.iter().find(|d| d.ip == ip && !used.contains(&d.id)),
        };
        if let Some(device) = found {
            used.insert(device.id);
        }

        let mut ports: Vec<PlannedPort> = Vec::new();
        for service in &host.services {
            let protocol = protocol_name(service.protocol).to_string();
            if ports.iter().any(|p| p.port == service.port && p.protocol == protocol) {
                continue;
            }
            let seen = |device: &KnownDevice| {
                device.ports.iter().any(|(port, proto)| *port == service.port && *proto == protocol)
            };
            let is_new = !found.is_some_and(seen);
            ports.push(PlannedPort {
                port: service.port,
                protocol,
                product: service.product.clone(),
                is_new,
            });
        }
        ports.sort_by(|a, b| (a.port, &a.protocol).cmp(&(b.port, &b.protocol)));

        // Une ligne retrouvée par son IP garde son identité tant qu'aucune MAC ne
        // vient la promouvoir.
        let mut ident = match (found, &mac) {
            (Some(device), None) => device.ident.clone(),
            _ => ident,
        };
        // Deux hôtes d'un même balayage ne partagent pas une identité (ARP
        // proxy : plusieurs IP derrière une seule MAC) : le second retombe sur
        // son IP, et un doublon d'IP est ignoré.
        if !idents.insert(ident.clone()) {
            ident = ip_ident;
            if !idents.insert(ident.clone()) {
                continue;
            }
        }

        out.push(PlannedDevice {
            existing_id: found.map(|d| d.id),
            ident,
            mac,
            ip,
            hostname: host.hostname.clone(),
            vendor: host.vendor.clone(),
            is_new: found.is_none(),
            ports,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use dumbmonit_proto::{Confidence, DiscoveredService};

    use super::*;

    fn service(port: u16, product: &str) -> DiscoveredService {
        DiscoveredService {
            port,
            protocol: Transport::Tcp,
            product: product.to_string(),
            kind: None,
            version: None,
            confidence: Confidence::High,
            tls: false,
            detail: None,
            monitored_target_id: None,
            prefill: None,
        }
    }

    fn host(ip: &str, mac: Option<&str>, ports: &[u16]) -> DiscoveredHost {
        DiscoveredHost {
            ip: ip.to_string(),
            hostname: None,
            mac: mac.map(str::to_string),
            vendor: None,
            services: ports.iter().map(|p| service(*p, "svc")).collect(),
        }
    }

    fn device(id: i64, ident: &str, ip: &str, ports: &[u16]) -> KnownDevice {
        KnownDevice {
            id,
            ident: ident.to_string(),
            ip: ip.to_string(),
            ports: ports.iter().map(|p| (*p, "tcp".to_string())).collect(),
        }
    }

    #[test]
    fn un_appareil_absent_de_l_inventaire_est_nouveau() {
        let known = [device(1, "mac:aa:aa:aa:aa:aa:aa", "10.0.0.2", &[22])];
        let plan = plan(&known, &[host("10.0.0.9", Some("bb:bb:bb:bb:bb:bb"), &[80])]);
        assert_eq!(plan.len(), 1);
        assert!(plan[0].is_new);
        assert_eq!(plan[0].existing_id, None);
        assert_eq!(plan[0].ident, "mac:bb:bb:bb:bb:bb:bb");
        assert!(plan[0].ports[0].is_new);
    }

    #[test]
    fn un_nouveau_port_sur_un_appareil_connu_est_signale() {
        let known = [device(1, "mac:aa:aa:aa:aa:aa:aa", "10.0.0.2", &[22])];
        let plan = plan(&known, &[host("10.0.0.2", Some("AA:AA:AA:AA:AA:AA"), &[22, 3389])]);
        assert!(!plan[0].is_new);
        let new: Vec<u16> = plan[0].ports.iter().filter(|p| p.is_new).map(|p| p.port).collect();
        assert_eq!(new, vec![3389]);
    }

    #[test]
    fn un_appareil_qui_change_d_ip_mais_pas_de_mac_n_est_pas_nouveau() {
        let known = [device(7, "mac:aa:aa:aa:aa:aa:aa", "10.0.0.2", &[22])];
        let plan = plan(&known, &[host("10.0.0.50", Some("aa:aa:aa:aa:aa:aa"), &[22])]);
        assert!(!plan[0].is_new);
        assert_eq!(plan[0].existing_id, Some(7));
        assert_eq!(plan[0].ip, "10.0.0.50");
        assert!(plan[0].ports.iter().all(|p| !p.is_new));
    }

    #[test]
    fn une_mac_qui_apparait_promeut_la_ligne_identifiee_par_l_ip() {
        let known = [device(3, "ip:10.0.0.4", "10.0.0.4", &[80])];
        let plan = plan(&known, &[host("10.0.0.4", Some("cc:cc:cc:cc:cc:cc"), &[80])]);
        assert!(!plan[0].is_new);
        assert_eq!(plan[0].existing_id, Some(3));
        assert_eq!(plan[0].ident, "mac:cc:cc:cc:cc:cc:cc");
    }

    #[test]
    fn une_mac_qui_manque_ne_cree_pas_un_appareil_de_plus() {
        let known = [device(3, "mac:cc:cc:cc:cc:cc:cc", "10.0.0.4", &[80])];
        let plan = plan(&known, &[host("10.0.0.4", None, &[80])]);
        assert!(!plan[0].is_new);
        assert_eq!(plan[0].ident, "mac:cc:cc:cc:cc:cc:cc");
    }

    #[test]
    fn une_ligne_ne_sert_qu_une_fois_par_balayage() {
        // Le même hôte rapporté deux fois dans un balayage ne fait qu'un appareil,
        // déjà connu : ni doublon, ni fausse nouveauté.
        let known = [device(3, "ip:10.0.0.4", "10.0.0.4", &[])];
        let plan = plan(&known, &[host("10.0.0.4", None, &[]), host("10.0.0.4", None, &[])]);
        assert_eq!(plan.len(), 1);
        assert!(!plan[0].is_new);
    }

    #[test]
    fn deux_hotes_derriere_une_meme_mac_restent_deux_appareils() {
        let mac = Some("dd:dd:dd:dd:dd:dd");
        let plan = plan(&[], &[host("10.0.0.5", mac, &[]), host("10.0.0.6", mac, &[])]);
        assert_eq!(plan.len(), 2);
        assert_ne!(plan[0].ident, plan[1].ident);
    }

    #[test]
    fn les_services_en_double_sur_un_port_comptent_une_fois() {
        let mut h = host("10.0.0.4", None, &[80]);
        h.services.push(service(80, "autre"));
        let plan = plan(&[], &[h]);
        assert_eq!(plan[0].ports.len(), 1);
    }
}
