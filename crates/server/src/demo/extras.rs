//! Équipements synthétiques de la démonstration qui n'ont pas de faux serveur
//! derrière eux : un commutateur SNMP et ses ports, CrowdSec et Cloudflare (la
//! carte des menaces), des noms de domaine et une licence (la frise des
//! échéances).
//!
//! Comme les sondes de [`super::synthetic`], chaque mesure ne dépend que du nom
//! de la cible et de l'horodatage : le direct, l'historique rétroactif
//! ([`super::backfill`]) et un redémarrage donnent les mêmes valeurs. Aucune
//! connexion ne part vers l'extérieur.

use std::f64::consts::TAU;

use dumbmonit_proto::Sample;

use super::synthetic::{activity, counter, cumulative, days_since_anchor, gauge, noise, seed};

/// Types de cible que ce module sait simuler.
pub const KINDS: &[&str] = &["snmp", "crowdsec", "cloudflare", "domain", "veeam"];

pub const SWITCH_HOST: &str = "switch-core.home.arpa";
pub const CROWDSEC_HOST: &str = "crowdsec.home.arpa";
pub const CLOUDFLARE_HOST: &str = "Cloudflare edge";
pub const VEEAM_HOST: &str = "veeam.home.arpa";

/// Domaines surveillés : (nom, jours restants au démarrage, bureau
/// d'enregistrement). Le dernier arrive à échéance : l'alerte se déclenche.
pub const DOMAINS: &[(&str, f64, &str)] = &[
    ("dupont.ch", 212.0, "Infomaniak Network SA"),
    ("dupont-photos.net", 41.0, "Gandi SAS"),
    ("homelab-notes.eu", 6.0, "OVH sas"),
];

/// Vrai pour les types de cible que [`samples`] simule.
pub fn handles(kind: &str) -> bool {
    KINDS.contains(&kind)
}

/// Ce que la sonde `kind` rapporterait pour `name` à l'instant `ts_ms`.
pub fn samples(kind: &str, name: &str, ts_ms: i64) -> Option<Vec<Sample>> {
    let mut out = match kind {
        "snmp" => switch(ts_ms),
        "crowdsec" => crowdsec(ts_ms),
        "cloudflare" => cloudflare(ts_ms),
        "domain" => domain(name, ts_ms),
        "veeam" => veeam(ts_ms),
        _ => return None,
    };
    // Temps de réponse : de quelques millisecondes (SNMP local) à près d'une demi-
    // seconde (RDAP), qui suit l'activité de la journée.
    let base = match kind {
        "snmp" => 0.012,
        "crowdsec" => 0.045,
        "cloudflare" => 0.31,
        "domain" => 0.42,
        _ => 0.19,
    };
    let load = activity(seed(name), ts_ms);
    out.push(gauge("probe_duration_seconds", base * (0.7 + 0.6 * load), ts_ms));
    Some(out)
}

// --------------------------------------------------------------- commutateur

/// Un port du commutateur : numéro (ifIndex), nom, description, état, vitesse,
/// voisin LLDP (nom, port distant), débits moyens entrant et sortant en octets par
/// seconde, puissance PoE, VLAN, erreurs par jour.
struct Port {
    index: u32,
    name: &'static str,
    alias: &'static str,
    up: bool,
    admin: bool,
    speed_bps: f64,
    half_duplex: bool,
    neighbor: Option<(&'static str, &'static str)>,
    rx: f64,
    tx: f64,
    poe_watts: Option<f64>,
    vlan: u32,
    errors_per_day: f64,
}

const G1: f64 = 1e9;
const M100: f64 = 1e8;

const fn port(index: u32, name: &'static str, alias: &'static str) -> Port {
    Port {
        index,
        name,
        alias,
        up: true,
        admin: true,
        speed_bps: G1,
        half_duplex: false,
        neighbor: None,
        rx: 0.0,
        tx: 0.0,
        poe_watts: None,
        vlan: 10,
        errors_per_day: 0.0,
    }
}

/// Seize ports gigabit : la liaison montante vers le pare-feu, les serveurs, des
/// points d'accès et caméras en PoE, un port coupé et quelques ports libres.
const PORTS: &[Port] = &[
    Port {
        neighbor: Some(("fw.home.arpa", "igb1")),
        rx: 5.5e7,
        tx: 2.2e7,
        vlan: 1,
        ..port(1, "Gi1", "Uplink to firewall")
    },
    Port {
        neighbor: Some(("nas.home.arpa", "eth0")),
        rx: 2.8e7,
        tx: 9.6e7,
        ..port(2, "Gi2", "NAS")
    },
    Port {
        neighbor: Some(("pve.home.arpa", "eno1")),
        speed_bps: 2.5e9,
        rx: 1.2e8,
        tx: 9.0e7,
        ..port(3, "Gi3", "Proxmox")
    },
    Port {
        neighbor: Some(("truenas.home.arpa", "igb0")),
        rx: 1.8e7,
        tx: 4.1e7,
        ..port(4, "Gi4", "TrueNAS")
    },
    Port {
        neighbor: Some(("docker01.home.arpa", "enp3s0")),
        rx: 1.2e7,
        tx: 1.9e7,
        ..port(5, "Gi5", "Docker host")
    },
    Port {
        neighbor: Some(("bmc-r740.home.arpa", "eth0")),
        speed_bps: M100,
        rx: 3.0e5,
        tx: 5.0e5,
        vlan: 20,
        ..port(6, "Gi6", "R740 iDRAC")
    },
    Port {
        speed_bps: M100,
        half_duplex: true,
        rx: 4.0e3,
        tx: 9.0e3,
        errors_per_day: 14.0,
        vlan: 30,
        ..port(7, "Gi7", "Printer")
    },
    Port { up: false, ..port(8, "Gi8", "") },
    Port {
        neighbor: Some(("ap-livingroom", "eth0")),
        rx: 6.5e6,
        tx: 1.4e7,
        poe_watts: Some(11.8),
        vlan: 30,
        ..port(9, "Gi9", "Wi-Fi living room")
    },
    Port {
        speed_bps: M100,
        rx: 2.2e5,
        tx: 1.2e6,
        poe_watts: Some(4.2),
        vlan: 40,
        ..port(10, "Gi10", "Camera hall")
    },
    Port {
        speed_bps: M100,
        rx: 2.0e5,
        tx: 1.4e6,
        poe_watts: Some(4.8),
        vlan: 40,
        ..port(11, "Gi11", "Camera garage")
    },
    Port { up: false, poe_watts: Some(0.0), ..port(12, "Gi12", "Spare PoE") },
    Port { up: false, ..port(13, "Gi13", "") },
    Port { up: false, admin: false, ..port(14, "Gi14", "Disabled") },
    Port { rx: 2.4e6, tx: 6.0e5, vlan: 30, ..port(15, "Gi15", "Workshop desk") },
    Port { up: false, ..port(16, "Gi16", "SFP uplink (unused)") },
];

fn switch(ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let days = days_since_anchor(ts_ms) + 30.0;
    let mut consumed = 0.0;
    for p in PORTS {
        let index = p.index.to_string();
        let on = |sample: Sample| {
            sample.with_label("index", index.as_str()).with_label("ifname", p.name)
        };
        out.push(
            gauge("switch_port_status", f64::from(u8::from(p.up)), ts_ms)
                .with_label("index", index.as_str())
                .with_label("ifname", p.name)
                .with_label("ifalias", p.alias)
                .with_label("admin", if p.admin { "1" } else { "2" }),
        );
        out.push(on(gauge("if_oper_status", if p.up { 1.0 } else { 2.0 }, ts_ms)));
        let bridge = gauge("bridge_port_info", f64::from(p.index), ts_ms);
        out.push(bridge.with_label("index", index.as_str()));
        let pvid = gauge("port_pvid", f64::from(p.vlan), ts_ms);
        out.push(pvid.with_label("index", index.as_str()));
        out.push(
            gauge("lldp_local_port_info", 1.0, ts_ms)
                .with_label("index", index.as_str())
                .with_label("port_id", p.name),
        );
        if let Some(watts) = p.poe_watts {
            let state = if watts > 0.0 { 3.0 } else { 2.0 };
            consumed += watts;
            out.push(
                gauge("poe_port_status", state, ts_ms)
                    .with_label("index", format!("1.{}", p.index))
                    .with_label("admin", "1")
                    .with_label("class", "3"),
            );
        }
        if !p.up {
            continue;
        }
        out.push(on(gauge("if_speed_bps", p.speed_bps, ts_ms)));
        out.push(
            gauge("ether_duplex_status", if p.half_duplex { 2.0 } else { 3.0 }, ts_ms)
                .with_label("index", index.as_str()),
        );
        if let Some((remote, remote_port)) = p.neighbor {
            out.push(
                gauge("lldp_neighbor_info", 1.0, ts_ms)
                    .with_label("index", format!("0.{}.1", p.index))
                    .with_label("remote_sysname", remote)
                    .with_label("remote_port", remote_port),
            );
        }
        for (metric, rate) in [("if_octets_in", p.rx), ("if_octets_out", p.tx)] {
            let seed = seed(&format!("{}{metric}", p.name));
            out.push(on(counter(metric, cumulative(seed, rate, ts_ms), ts_ms)));
        }
        for metric in ["if_errors_in", "if_errors_out"] {
            let share = if metric == "if_errors_in" { 0.7 } else { 0.3 };
            out.push(on(counter(metric, (p.errors_per_day * share * days).floor(), ts_ms)));
        }
    }
    let wobble = 0.4 * (TAU * days).sin();
    out.push(gauge("poe_power_budget_watts", 120.0, ts_ms).with_label("index", "1"));
    out.push(gauge("poe_power_consumed_watts", consumed + wobble, ts_ms).with_label("index", "1"));
    out
}

// ------------------------------------------------------------ carte des menaces

/// Alertes CrowdSec des dernières 24 heures par pays d'origine.
const CROWDSEC_COUNTRIES: &[(&str, f64)] = &[
    ("CN", 148.0),
    ("RU", 96.0),
    ("US", 71.0),
    ("BR", 44.0),
    ("IN", 39.0),
    ("VN", 31.0),
    ("NL", 27.0),
    ("DE", 22.0),
    ("KR", 19.0),
    ("ID", 16.0),
    ("TR", 12.0),
    ("FR", 9.0),
    ("UA", 7.0),
    ("EG", 5.0),
];

/// Menaces arrêtées par Cloudflare sur les deux dernières heures, par pays.
const CLOUDFLARE_COUNTRIES: &[(&str, f64)] = &[
    ("RU", 212.0),
    ("CN", 187.0),
    ("US", 96.0),
    ("DE", 58.0),
    ("BR", 51.0),
    ("IN", 47.0),
    ("NL", 33.0),
    ("FR", 28.0),
    ("SG", 22.0),
    ("GB", 19.0),
    ("JP", 14.0),
    ("PL", 11.0),
];

/// Un compte qui varie d'une heure à l'autre autour de `base`, sans jamais
/// tomber à zéro : la carte bouge sans changer de forme.
fn hourly(country: &str, base: f64, ts_ms: i64) -> f64 {
    let hour = ts_ms.div_euclid(3_600_000);
    (base * (0.8 + 0.4 * noise(seed(country), hour))).round().max(1.0)
}

fn crowdsec(ts_ms: i64) -> Vec<Sample> {
    let mut out = vec![
        gauge("crowdsec_lapi_up", 1.0, ts_ms),
        gauge("crowdsec_lapi_decisions_ok", 1.0, ts_ms),
        gauge("crowdsec_version_info", 1.0, ts_ms).with_label("version", "v1.7.2"),
        gauge("crowdsec_bouncers_seen", 2.0, ts_ms),
    ];
    let mut total = 0.0;
    for (country, base) in CROWDSEC_COUNTRIES {
        let count = hourly(country, *base, ts_ms);
        total += count;
        out.push(gauge("crowdsec_alerts_by_country", count, ts_ms).with_label("country", *country));
    }
    out.push(gauge("crowdsec_alerts_recent", total, ts_ms));
    out.push(gauge("crowdsec_alerts", total, ts_ms));
    out.push(gauge("crowdsec_decisions", (total * 1.6).round(), ts_ms));
    out
}

fn cloudflare(ts_ms: i64) -> Vec<Sample> {
    let mut out = vec![gauge("cloudflare_analytics_readable", 1.0, ts_ms)];
    for (country, base) in CLOUDFLARE_COUNTRIES {
        let count = hourly(country, *base, ts_ms);
        out.push(gauge("cloudflare_country_threats", count, ts_ms).with_label("country", *country));
    }
    out
}

// ---------------------------------------------------------------- échéances

/// Jours restants d'une échéance qui valait `at_start` au démarrage : elle se
/// rapproche d'un jour par jour, y compris dans l'historique.
fn remaining_days(at_start: f64, ts_ms: i64) -> f64 {
    (at_start - days_since_anchor(ts_ms)).max(0.0)
}

fn domain(name: &str, ts_ms: i64) -> Vec<Sample> {
    let (days, registrar) = DOMAINS
        .iter()
        .find(|(domain, _, _)| *domain == name)
        .map_or((120.0, "Example Registrar"), |(_, days, registrar)| (*days, *registrar));
    let left = remaining_days(days, ts_ms);
    vec![
        gauge("domain_rdap_ok", 1.0, ts_ms),
        gauge("domain_rdap_age_seconds", 40.0, ts_ms),
        gauge("domain_expiry_published", 1.0, ts_ms),
        gauge("domain_expiry_days", left, ts_ms),
        gauge("domain_expiry_timestamp_seconds", ts_ms as f64 / 1000.0 + left * 86_400.0, ts_ms),
        gauge("domain_on_hold", 0.0, ts_ms),
        gauge("domain_redemption", 0.0, ts_ms),
        gauge("domain_dnssec", 1.0, ts_ms),
        gauge("domain_status", 1.0, ts_ms).with_label("status", "client transfer prohibited"),
        gauge("domain_registrar_info", 1.0, ts_ms).with_label("registrar", registrar),
    ]
}

fn veeam(ts_ms: i64) -> Vec<Sample> {
    let seconds = |days: f64| remaining_days(days, ts_ms) * 86_400.0;
    vec![
        gauge("veeam_license_valid", 1.0, ts_ms),
        gauge("veeam_license_info", 1.0, ts_ms)
            .with_label("status", "Valid")
            .with_label("type", "Subscription")
            .with_label("edition", "Enterprise Plus"),
        gauge("veeam_license_expiry_seconds", seconds(58.0), ts_ms),
        gauge("veeam_support_expiry_seconds", seconds(17.0), ts_ms),
        gauge("veeam_license_instances_licensed", 25.0, ts_ms),
        gauge("veeam_license_instances_used", 19.0, ts_ms),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const TS: i64 = 1_790_000_000_000;

    fn find<'a>(samples: &'a [Sample], metric: &str) -> Vec<&'a Sample> {
        samples.iter().filter(|s| s.metric == metric).collect()
    }

    #[test]
    fn chaque_type_simule_repond_et_porte_un_temps_de_reponse() {
        for kind in KINDS {
            let samples = samples(kind, "any", TS).expect(kind);
            assert_eq!(find(&samples, "probe_duration_seconds").len(), 1, "{kind}");
        }
        assert!(samples("http", "any", TS).is_none());
        assert!(!handles("http"));
    }

    #[test]
    fn le_meme_instant_donne_les_memes_mesures() {
        for kind in KINDS {
            assert_eq!(samples(kind, "n", TS), samples(kind, "n", TS), "{kind}");
        }
    }

    #[test]
    fn le_commutateur_a_des_ports_avec_trafic_et_voisins() {
        let all = switch(TS);
        let status = find(&all, "switch_port_status");
        assert_eq!(status.len(), PORTS.len());
        assert!(status.iter().any(|s| s.value == 0.0), "des ports sans lien");
        assert!(status.iter().any(|s| s.labels.get("admin").is_some_and(|a| a == "2")));
        // Le trafic n'existe que sur les ports qui ont un lien, avec les deux
        // étiquettes que lisent le panneau (index) et la carte (ifname).
        let octets = find(&all, "if_octets_in");
        assert_eq!(octets.len(), PORTS.iter().filter(|p| p.up).count());
        assert!(octets.iter().all(|s| {
            s.labels.contains_key("index") && s.labels.contains_key("ifname")
        }));
        // Les voisins LLDP portent le nom des cibles du parc : la carte les relie.
        let neighbors: Vec<&str> = find(&all, "lldp_neighbor_info")
            .iter()
            .filter_map(|s| s.labels.get("remote_sysname").map(String::as_str))
            .collect();
        for host in ["fw.home.arpa", "nas.home.arpa", "pve.home.arpa", "docker01.home.arpa"] {
            assert!(neighbors.contains(&host), "{host}");
        }
        assert_eq!(find(&all, "poe_port_status").len(), 4);
    }

    #[test]
    fn les_noms_de_ports_et_les_index_sont_uniques() {
        let mut names: Vec<&str> = PORTS.iter().map(|p| p.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), PORTS.len());
        assert!(PORTS.windows(2).all(|w| w[0].index + 1 == w[1].index));
    }

    #[test]
    fn les_compteurs_de_ports_ne_reculent_jamais() {
        let mut last = 0.0;
        for step in 0..500 {
            let at = TS + step * 300_000;
            let value = find(&switch(at), "if_octets_out")[0].value;
            assert!(value >= last, "recul à l'étape {step}");
            last = value;
        }
    }

    #[test]
    fn la_carte_des_menaces_a_des_pays_et_une_source_par_type() {
        let crowd = crowdsec(TS);
        let countries = find(&crowd, "crowdsec_alerts_by_country");
        assert_eq!(countries.len(), CROWDSEC_COUNTRIES.len());
        assert!(countries.iter().all(|s| {
            s.value >= 1.0 && s.labels.get("country").is_some_and(|c| c.len() == 2)
        }));
        let total = find(&crowd, "crowdsec_alerts_recent")[0].value;
        assert_eq!(total, countries.iter().map(|s| s.value).sum::<f64>());
        let cloud = cloudflare(TS);
        assert_eq!(find(&cloud, "cloudflare_country_threats").len(), CLOUDFLARE_COUNTRIES.len());
        assert_eq!(find(&cloud, "cloudflare_analytics_readable")[0].value, 1.0);
    }

    #[test]
    fn les_echeances_se_rapprochent_d_un_jour_par_jour() {
        let now = super::super::synthetic::anchor_ms();
        let day = 86_400_000;
        for (name, days, _) in DOMAINS {
            let at = |ts| find(&domain(name, ts), "domain_expiry_days")[0].value;
            assert!((at(now) - days).abs() < 1e-9, "{name}");
            assert!((at(now - 10 * day) - (days + 10.0)).abs() < 1e-9, "{name}");
        }
        assert!(DOMAINS.iter().any(|(_, days, _)| *days < 14.0), "une échéance proche");
        let license = find(&veeam(now), "veeam_license_expiry_seconds")[0].value;
        assert!((license - 58.0 * 86_400.0).abs() < 1.0);
    }
}
