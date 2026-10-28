//! Profils éprouvés sur des relevés de vrais équipements.
//!
//! Un relevé est un fichier `.snmprec` (format de snmpsim : `OID|type|valeur`,
//! trié) pris sur un matériel réel puis pseudonymisé. Il sert de [`Source`] à la
//! vraie fonction de collecte : la sélection du profil, les jointures d'index,
//! les filtres et les facteurs d'échelle tournent exactement comme face à
//! l'équipement, sans réseau.

use std::collections::BTreeMap;

use dumbmonit_proto::{ProbeError, Sample};

use super::collect::{Source, collect};
use super::oid::ObjectId;
use super::profile::embedded;
use super::value::SnmpValue;

/// Un relevé chargé en mémoire, dans l'ordre SNMP.
pub struct Recording {
    values: BTreeMap<ObjectId, SnmpValue>,
}

impl Recording {
    pub fn parse(source: &str) -> Self {
        let mut values = BTreeMap::new();
        for line in source.lines() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            let mut parts = line.splitn(3, '|');
            let (Some(oid), Some(tag), Some(raw)) = (parts.next(), parts.next(), parts.next())
            else {
                panic!("ligne de relevé illisible : {line}");
            };
            let oid: ObjectId = oid.parse().unwrap_or_else(|_| panic!("OID illisible : {oid}"));
            let value = match tag {
                "2" => SnmpValue::Integer(raw.parse().expect("entier")),
                "4" => SnmpValue::Bytes(raw.as_bytes().to_vec()),
                "4x" => SnmpValue::Bytes(
                    (0..raw.len())
                        .step_by(2)
                        .map(|i| u8::from_str_radix(&raw[i..i + 2], 16).expect("hexadécimal"))
                        .collect(),
                ),
                "6" => SnmpValue::Oid(raw.parse().expect("OID en valeur")),
                "64" => {
                    let octets: Vec<u8> =
                        raw.split('.').map(|o| o.parse().expect("IPv4")).collect();
                    SnmpValue::IpAddress([octets[0], octets[1], octets[2], octets[3]])
                }
                "65" | "70" => SnmpValue::Counter(raw.parse().expect("compteur")),
                "66" => SnmpValue::Unsigned(raw.parse().expect("jauge")),
                "67" => SnmpValue::Timeticks(raw.parse().expect("timeticks")),
                other => panic!("type snmprec non pris en charge : {other}"),
            };
            values.insert(oid, value);
        }
        Self { values }
    }

    fn scalar(&self, oid: &str) -> Option<&SnmpValue> {
        self.values.get(&oid.parse::<ObjectId>().ok()?)
    }

    /// Le profil que l'auto-détection choisirait pour cet équipement.
    pub fn detected_profile(&self) -> String {
        let sysobjectid = match self.scalar("1.3.6.1.2.1.1.2.0") {
            Some(SnmpValue::Oid(oid)) => Some(oid.clone()),
            _ => None,
        };
        let sysdescr = self.scalar("1.3.6.1.2.1.1.1.0").and_then(SnmpValue::as_label);
        embedded()
            .select(sysobjectid.as_ref(), sysdescr.as_deref())
            .map(|profile| profile.id.clone())
            .expect("aucun profil retenu")
    }

    /// Applique un profil livré au relevé, avec la vraie fonction de collecte.
    pub async fn collect_with(&mut self, profile: &str) -> Vec<Sample> {
        let metrics = embedded().resolve(profile).expect("profil livré");
        collect(self, &metrics, 0).await.expect("collecte sur relevé")
    }
}

impl Source for Recording {
    async fn get_many(
        &mut self,
        oids: &[ObjectId],
    ) -> Result<Vec<(ObjectId, SnmpValue)>, ProbeError> {
        Ok(oids
            .iter()
            .filter_map(|oid| self.values.get(oid).map(|value| (oid.clone(), value.clone())))
            .collect())
    }

    async fn walk(&mut self, base: &ObjectId) -> Result<Vec<(ObjectId, SnmpValue)>, ProbeError> {
        Ok(self
            .values
            .range(base.clone()..)
            .take_while(|(oid, _)| oid.starts_with(base))
            .filter(|(oid, _)| oid.is_inside(base))
            .map(|(oid, value)| (oid.clone(), value.clone()))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ZYXEL: &str = include_str!("fixtures/zyxel-xs1930-10.snmprec");
    const BMC: &str = include_str!("fixtures/bmc-supermicro-netsnmp.snmprec");
    const POE: &str = include_str!("fixtures/poe-switch-synthetic.snmprec");

    fn run(recording: &'static str, profile: Option<&'static str>) -> (String, Vec<Sample>) {
        super::super::testutil::block_on_large_stack(async move {
            let mut recording = Recording::parse(recording);
            let chosen = profile.map_or_else(|| recording.detected_profile(), str::to_string);
            let samples = recording.collect_with(&chosen).await;
            (chosen, samples)
        })
    }

    fn values<'a>(samples: &'a [Sample], metric: &str) -> Vec<&'a Sample> {
        samples.iter().filter(|s| s.metric == metric).collect()
    }

    fn on_port<'a>(samples: &'a [Sample], metric: &str, port: &str) -> Option<&'a Sample> {
        samples.iter().find(|s| {
            s.metric == metric && s.labels.get("ifname").map(String::as_str) == Some(port)
        })
    }

    #[test]
    fn le_zyxel_reel_recoit_le_profil_zyxel() {
        let (profile, _) = run(ZYXEL, None);
        assert_eq!(profile, "zyxel");
    }

    #[test]
    fn le_zyxel_reel_livre_processeur_et_memoire_de_sa_branche_privee() {
        let (_, samples) = run(ZYXEL, None);
        assert_eq!(values(&samples, "cpu_load_percent")[0].value, 38.0);
        assert_eq!(values(&samples, "zyxel_cpu_5min_percent")[0].value, 30.0);
        assert_eq!(values(&samples, "memory_usage_percent")[0].value, 8.0);
        let info = values(&samples, "zyxel_firmware_info")[0];
        assert_eq!(info.labels.get("model").map(String::as_str), Some("XS1930-10"));
        assert!(info.labels["firmware"].starts_with("V4.80(ABQE.4)"));
        // Le moniteur matériel (esMgmt.26) est au-delà de la troncature du relevé.
        assert!(values(&samples, "zyxel_temperature_celsius").is_empty());
    }

    #[test]
    fn le_zyxel_reel_detaille_les_erreurs_ethernet_par_port() {
        let (_, samples) = run(ZYXEL, None);
        // swp04 : lien tombé après avoir servi, 127 erreurs FCS et 214 de symbole.
        assert_eq!(on_port(&samples, "ether_fcs_errors", "swp04").unwrap().value, 127.0);
        assert_eq!(on_port(&samples, "ether_symbol_errors", "swp04").unwrap().value, 214.0);
        assert_eq!(on_port(&samples, "ether_duplex_status", "swp01").unwrap().value, 3.0);
        // swp08 et swp09 n'ont jamais eu de lien : écartés, comme par IF-MIB.
        assert!(on_port(&samples, "ether_fcs_errors", "swp08").is_none());
        assert!(on_port(&samples, "if_octets_in", "swp09").is_none());
        // Les compteurs 64 bits de trafic viennent bien de la ifXTable.
        assert_eq!(on_port(&samples, "if_octets_in", "swp00").unwrap().value, 23_075_353_854_792.0);
        assert!(on_port(&samples, "if_broadcast_packets_in", "swp00").is_some());
        // dot3StatsFrameTooLongs n'est pas lue : elle compte les trames jumbo.
        assert!(samples.iter().all(|s| !s.metric.contains("too_long")));
    }

    fn on_index<'a>(samples: &'a [Sample], metric: &str, index: &str) -> Option<&'a Sample> {
        samples.iter().find(|s| {
            s.metric == metric && s.labels.get("index").map(String::as_str) == Some(index)
        })
    }

    #[test]
    fn la_facade_du_zyxel_montre_tous_les_ports_physiques() {
        let (_, samples) = run(ZYXEL, None);
        let ports: Vec<_> = values(&samples, "switch_port_status")
            .into_iter()
            .filter_map(|s| s.labels.get("ifname").map(String::as_str))
            .collect();
        // Les dix ports en façade, y compris swp08 et swp09 jamais montés, que
        // if_oper_status écarte.
        assert_eq!(ports.len(), 10, "{ports:?}");
        assert_eq!(on_port(&samples, "switch_port_status", "swp09").unwrap().value, 2.0);
        assert_eq!(on_port(&samples, "switch_port_status", "swp00").unwrap().value, 1.0);
        // Les interfaces de VLAN (ifType 53) ne sont pas des ports.
        assert!(on_index(&samples, "switch_port_status", "30001").is_none());
        // Pas de PoE sur ce modèle : séries simplement absentes.
        assert!(values(&samples, "poe_port_status").is_empty());
    }

    #[test]
    fn le_commutateur_poe_livre_facade_poe_vlan_et_voisin() {
        let (_, samples) = run(POE, Some("ethernet-switch"));

        // Façade : quatre ports Ethernet, ni le VLAN (136) ni l'agrégat (161).
        let status = values(&samples, "switch_port_status");
        assert_eq!(status.len(), 4);
        let ge4 = on_port(&samples, "switch_port_status", "ge4").unwrap();
        assert_eq!(ge4.value, 2.0);
        assert_eq!(ge4.labels["admin"], "2");
        assert_eq!(ge4.labels["index"], "4");
        // ge3 n'a jamais eu de lien : absent d'IF-MIB, présent en façade.
        assert!(on_port(&samples, "if_oper_status", "ge3").is_none());
        assert_eq!(on_port(&samples, "switch_port_status", "ge3").unwrap().value, 2.0);
        assert_eq!(
            on_port(&samples, "switch_port_status", "ge1").unwrap().labels["ifalias"],
            "Hall AP"
        );
        assert_eq!(on_port(&samples, "if_speed_bps", "ge2").unwrap().value, 100_000_000.0);

        // PoE par port, indexé groupe.port.
        let poe1 = on_index(&samples, "poe_port_status", "1.1").unwrap();
        assert_eq!(poe1.value, 3.0, "deliveringPower");
        assert_eq!(poe1.labels["class"], "3");
        assert_eq!(on_index(&samples, "poe_port_status", "1.3").unwrap().value, 4.0, "fault");
        assert_eq!(on_index(&samples, "poe_port_status", "1.4").unwrap().labels["admin"], "2");
        // Totaux du groupe 1.
        assert_eq!(on_index(&samples, "poe_power_budget_watts", "1").unwrap().value, 60.0);
        assert_eq!(on_index(&samples, "poe_power_consumed_watts", "1").unwrap().value, 7.0);
        assert_eq!(on_index(&samples, "poe_main_status", "1").unwrap().value, 1.0);

        // VLAN natif par port du pont, et la table qui ramène à l'ifIndex.
        assert_eq!(on_index(&samples, "port_pvid", "3").unwrap().value, 20.0);
        assert_eq!(on_index(&samples, "bridge_port_info", "3").unwrap().value, 3.0);

        // Voisin LLDP sur le port local 1 (deuxième arc de l'index).
        let lldp = values(&samples, "lldp_neighbor_info");
        assert_eq!(lldp.len(), 1);
        assert_eq!(lldp[0].labels["index"], "0.1.1");
        assert_eq!(lldp[0].labels["remote_sysname"], "ap-hall");
        assert_eq!(lldp[0].labels["remote_port"], "eth0");
        assert_eq!(
            on_index(&samples, "lldp_local_port_info", "1").unwrap().labels["port_id"],
            "ge1"
        );
    }

    #[test]
    fn les_capteurs_entity_sensor_combinent_echelle_et_precision() {
        let (_, samples) = run(POE, Some("ethernet-switch"));
        let temp = on_index(&samples, "sensor_temperature_celsius", "1001").unwrap();
        assert!((temp.value - 45.3).abs() < 1e-9, "{}", temp.value);
        assert_eq!(temp.labels["sensor"], "Board temp");
        assert_eq!(on_index(&samples, "sensor_fan_rpm", "1002").unwrap().value, 4200.0);
        let volts = on_index(&samples, "sensor_voltage_volts", "1003").unwrap();
        assert!((volts.value - 12.05).abs() < 1e-9, "{}", volts.value);
        // Un capteur indisponible (état 2) n'émet rien, et chaque type va à sa famille.
        assert!(on_index(&samples, "sensor_temperature_celsius", "1004").is_none());
        assert!(on_index(&samples, "sensor_fan_rpm", "1001").is_none());
        assert_eq!(values(&samples, "sensor_temperature_celsius").len(), 1);
    }

    #[test]
    fn le_bmc_reel_reste_sur_host_resources_et_livre_memoire_et_charge() {
        let (profile, samples) = run(BMC, None);
        assert_eq!(profile, "host-resources", "Net-SNMP 8072 : le profil générique suffit");
        let total = values(&samples, "memory_bytes_total");
        assert_eq!(total.len(), 1);
        assert_eq!(total[0].value, 457_480.0 * 1024.0);
        let load: Vec<_> = values(&samples, "load_average");
        let one = load.iter().find(|s| s.labels["period"] == "Load-1").unwrap();
        assert!((one.value - 4.73).abs() < 1e-9);
        // hrProcessorLoad absente de cet agent : aucune charge processeur en %.
        assert!(values(&samples, "cpu_load_percent").is_empty());
        // Sous-interface de VLAN sur agrégat écartée, lien USB vers l'hôte conservé.
        assert!(on_port(&samples, "if_oper_status", "bond0.254").is_none());
        assert!(on_port(&samples, "if_oper_status", "usb0").is_some());
    }
}
