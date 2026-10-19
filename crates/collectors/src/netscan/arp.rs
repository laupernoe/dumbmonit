//! Adresse MAC et fabricant d'un hôte, quand ils sont visibles.
//!
//! La MAC n'est lisible que sur le même segment de niveau 2 : c'est le cas en
//! mode réseau `host` (Linux), où `/proc/net/arp` tient la correspondance IP →
//! MAC apprise par le noyau. Derrière un routeur, ou dans un conteneur en
//! réseau ponté, elle est invisible : on rend alors `None`, sans erreur.

use std::collections::HashMap;
use std::net::Ipv4Addr;

/// Table IP → MAC lue dans `/proc/net/arp`. Vide hors Linux ou sans `/proc`.
#[derive(Debug, Default, Clone)]
pub struct ArpTable {
    by_ip: HashMap<Ipv4Addr, String>,
}

impl ArpTable {
    /// Lit la table du noyau. Toujours un succès : une absence de `/proc/net/arp`
    /// (autre système, conteneur sans cette vue) donne simplement une table vide.
    pub async fn load() -> Self {
        let raw = tokio::fs::read_to_string("/proc/net/arp").await.unwrap_or_default();
        Self::parse(&raw)
    }

    pub fn parse(raw: &str) -> Self {
        let mut by_ip = HashMap::new();
        // En-tête : « IP address  HW type  Flags  HW address  Mask  Device ».
        for line in raw.lines().skip(1) {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() < 4 {
                continue;
            }
            let (Ok(ip), mac) = (fields[0].parse::<Ipv4Addr>(), fields[3]) else { continue };
            // `00:00:00:00:00:00` = entrée incomplète (hôte jamais joint).
            if mac == "00:00:00:00:00:00" || !mac.contains(':') {
                continue;
            }
            by_ip.insert(ip, mac.to_ascii_lowercase());
        }
        Self { by_ip }
    }

    pub fn mac(&self, ip: Ipv4Addr) -> Option<&str> {
        self.by_ip.get(&ip).map(String::as_str)
    }
}

/// Fabricant d'une MAC, d'après le registre OUI de l'IEEE embarqué (`oui.tsv`).
///
/// La table ne retient que les fabricants courants en homelab et PME : elle
/// reste petite, et un préfixe inconnu rend `None` plutôt que d'alourdir le
/// binaire avec les dizaines de milliers d'OUI du registre complet.
pub fn vendor(mac: &str) -> Option<&'static str> {
    // `02:42:…` : adresse administrée localement de Docker, absente du registre.
    let hex: String = mac.chars().filter(|c| c.is_ascii_hexdigit()).take(6).collect::<String>();
    if hex.len() < 6 {
        return None;
    }
    let prefix = hex.to_ascii_uppercase();
    if prefix.starts_with("0242") {
        return Some("Docker");
    }
    if prefix.starts_with("5254") {
        return Some("QEMU/KVM");
    }
    OUI.iter().find(|(p, _)| *p == prefix).map(|(_, name)| *name)
}

/// Registre OUI embarqué : `(préfixe 6 hexa majuscules, nom court)`.
static OUI: &[(&str, &str)] = &include!("oui_table.rs");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arp_table_parses_complete_entries_only() {
        let raw = "IP address       HW type     Flags       HW address            Mask     Device
192.168.1.20     0x1         0x2         00:11:32:aa:bb:cc     *        eth0
192.168.1.30     0x1         0x0         00:00:00:00:00:00     *        eth0
192.168.1.40     0x1         0x2         bc:24:11:00:00:01     *        eth0";
        let table = ArpTable::parse(raw);
        assert_eq!(table.mac("192.168.1.20".parse().unwrap()), Some("00:11:32:aa:bb:cc"));
        // Entrée incomplète ignorée.
        assert_eq!(table.mac("192.168.1.30".parse().unwrap()), None);
    }

    #[test]
    fn locally_administered_prefixes_are_recognised() {
        assert_eq!(vendor("02:42:ac:11:00:02"), Some("Docker"));
        assert_eq!(vendor("52:54:00:12:34:56"), Some("QEMU/KVM"));
        assert_eq!(vendor("zz"), None);
    }
}
