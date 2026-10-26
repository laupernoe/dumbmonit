//! Réseaux portés par les interfaces d'un pare-feu : la carte de topologie s'en
//! sert pour rattacher chaque équipement au pare-feu qui dessert son sous-réseau
//! et pour nommer les zones (CIDR, VLAN).

use dumbmonit_proto::{MetricKind, Sample};

/// Étiquette de VLAN d'un nom d'interface, quand le nom la porte :
/// `igb0.10`, `vlan0.20`, `ix1.100` (suffixe après le point) ou `vlan30`,
/// `vlan_40` (nombre collé à « vlan », sans zéro initial : `vlan01` est un
/// numéro d'instance, pas une étiquette).
pub fn vlan_hint(name: &str) -> Option<u16> {
    let lower = name.trim().to_lowercase();
    let valid = |digits: &str| -> Option<u16> {
        let tag: u16 = digits.parse().ok()?;
        (1..=4094).contains(&tag).then_some(tag)
    };
    if let Some((_, tail)) = lower.rsplit_once('.')
        && !tail.is_empty()
        && tail.bytes().all(|b| b.is_ascii_digit())
    {
        return valid(tail);
    }
    let (_, tail) = lower.rsplit_once("vlan")?;
    let digits: String =
        tail.trim_start_matches(['_', '-', ' ']).chars().take_while(char::is_ascii_digit).collect();
    if digits.len() < 2 || digits.starts_with('0') {
        return None;
    }
    valid(&digits)
}

/// Une série `<kind>_interface_address_info` : l'adresse (avec son préfixe)
/// d'une interface, son nom lisible et son VLAN quand on le connaît.
pub fn address_sample(
    metric: &str,
    interface: &str,
    description: &str,
    address: &str,
    vlan: Option<u16>,
    ts_ms: i64,
) -> Sample {
    let mut sample = Sample::new(metric, 1.0, MetricKind::Gauge, ts_ms)
        .with_label("interface", interface)
        .with_label("description", description)
        .with_label("address", address);
    if let Some(tag) = vlan {
        sample = sample.with_label("vlan", tag.to_string());
    }
    sample
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_vlan_se_lit_dans_le_nom_de_linterface() {
        assert_eq!(vlan_hint("igb0.10"), Some(10));
        assert_eq!(vlan_hint("vlan0.20"), Some(20));
        assert_eq!(vlan_hint("vlan30"), Some(30));
        assert_eq!(vlan_hint("VLAN_40"), Some(40));
        assert_eq!(vlan_hint("vlan01"), None, "numéro d'instance");
        assert_eq!(vlan_hint("igb1"), None);
        assert_eq!(vlan_hint("ix0.5000"), None);
        assert_eq!(vlan_hint("lan"), None);
    }
}
