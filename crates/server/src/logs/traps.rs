//! Récepteur de traps SNMP v1 et v2c (et des « inform » v2c, acquittés).
//!
//! Désactivé par défaut (`DUMBMONIT_TRAPS_ENABLED`). Port non privilégié (1162
//! par défaut : le port officiel, 162, exige des droits que le conteneur n'a
//! pas ; l'équipement émetteur peut viser 1162 ou un redirecteur). Chaque trap
//! devient une entrée de journal `service="trap"` : son nom (traps génériques
//! connues, sinon l'OID décrit par les profils SNMP embarqués), ses varbinds, et
//! la cible de l'émetteur quand l'adresse la désigne. Une règle « motif de
//! journal » sur `service:=trap` en fait une alerte.
//!
//! SNMPv3 n'est pas pris en charge : ses messages sont comptés comme refusés.
//!
//! Réglages : `DUMBMONIT_TRAPS_ENABLED`, `DUMBMONIT_TRAPS_PORT` (1162),
//! `DUMBMONIT_TRAPS_BIND`, `DUMBMONIT_TRAPS_COMMUNITIES` (liste séparée par des
//! virgules ; vide : toutes acceptées).

use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr};

use tokio::net::UdpSocket;

use super::attach;
use super::central::{self, Entry, central as hub, env_bind, env_flag, env_port};

pub const DEFAULT_PORT: u16 = 1162;
const MAX_DATAGRAM: usize = 65_535;
/// Longueur maximale du message d'une entrée.
const MAX_MESSAGE: usize = 2_048;
/// Varbinds décodés au plus par trap.
const MAX_VARBINDS: usize = 64;

const TAG_INTEGER: u8 = 0x02;
const TAG_OCTETS: u8 = 0x04;
const TAG_NULL: u8 = 0x05;
const TAG_OID: u8 = 0x06;
const TAG_SEQUENCE: u8 = 0x30;
const TAG_IPADDRESS: u8 = 0x40;
const TAG_COUNTER32: u8 = 0x41;
const TAG_GAUGE32: u8 = 0x42;
const TAG_TIMETICKS: u8 = 0x43;
const TAG_OPAQUE: u8 = 0x44;
const TAG_COUNTER64: u8 = 0x46;
const PDU_TRAP_V1: u8 = 0xA4;
const PDU_INFORM: u8 = 0xA6;
const PDU_TRAP_V2: u8 = 0xA7;
const PDU_RESPONSE: u8 = 0xA2;

/// Un trap décodé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trap {
    /// `v1` ou `v2c`.
    pub version: &'static str,
    pub community: String,
    /// OID de la trap (v2c), ou de l'entreprise (v1).
    pub trap_oid: Vec<u64>,
    /// Nom lisible, quand il est connu.
    pub name: String,
    /// Adresse déclarée par l'agent (v1).
    pub agent_addr: Option<String>,
    pub varbinds: Vec<(String, String)>,
    /// Vrai pour un « inform », qui attend une réponse.
    pub inform: bool,
    /// Position de l'étiquette du PDU dans le datagramme (pour y répondre).
    pub pdu_offset: usize,
}

// --------------------------------------------------------------------------
// BER
// --------------------------------------------------------------------------

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    end: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0, end: data.len() }
    }

    fn within(data: &'a [u8], start: usize, end: usize) -> Self {
        Self { data, pos: start, end }
    }

    fn done(&self) -> bool {
        self.pos >= self.end
    }

    /// Lit un TLV : `(étiquette, début du contenu, fin du contenu)`.
    fn tlv(&mut self) -> Option<(u8, usize, usize)> {
        if self.pos + 2 > self.end {
            return None;
        }
        let tag = self.data[self.pos];
        if tag & 0x1f == 0x1f {
            return None;
        }
        let first = self.data[self.pos + 1];
        let (length, header) = if first < 0x80 {
            (first as usize, 2)
        } else {
            let count = (first & 0x7f) as usize;
            if count == 0 || count > 4 || self.pos + 2 + count > self.end {
                return None;
            }
            let mut length = 0usize;
            for byte in &self.data[self.pos + 2..self.pos + 2 + count] {
                length = (length << 8) | *byte as usize;
            }
            (length, 2 + count)
        };
        let start = self.pos + header;
        let end = start.checked_add(length)?;
        if end > self.end {
            return None;
        }
        self.pos = end;
        Some((tag, start, end))
    }
}

fn integer(bytes: &[u8]) -> i64 {
    if bytes.is_empty() || bytes.len() > 8 {
        return 0;
    }
    let mut value: i64 = if bytes[0] & 0x80 != 0 { -1 } else { 0 };
    for byte in bytes {
        value = (value << 8) | *byte as i64;
    }
    value
}

fn unsigned(bytes: &[u8]) -> u64 {
    let mut value: u64 = 0;
    for byte in bytes.iter().skip_while(|b| **b == 0).take(8) {
        value = (value << 8) | *byte as u64;
    }
    value
}

/// Les arcs d'un OID encodé.
pub fn decode_oid(bytes: &[u8]) -> Option<Vec<u64>> {
    if bytes.is_empty() {
        return None;
    }
    let mut values: Vec<u64> = Vec::new();
    let mut current: u64 = 0;
    let mut open = false;
    for byte in bytes {
        current = current.checked_mul(128)?.checked_add((byte & 0x7f) as u64)?;
        if byte & 0x80 == 0 {
            values.push(current);
            current = 0;
            open = false;
        } else {
            open = true;
        }
    }
    if open {
        return None;
    }
    let first = values[0];
    let mut arcs = Vec::with_capacity(values.len() + 1);
    if first < 40 {
        arcs.extend([0, first]);
    } else if first < 80 {
        arcs.extend([1, first - 40]);
    } else {
        arcs.extend([2, first - 80]);
    }
    arcs.extend_from_slice(&values[1..]);
    Some(arcs)
}

pub fn dotted(arcs: &[u64]) -> String {
    arcs.iter().map(u64::to_string).collect::<Vec<_>>().join(".")
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::from("0x");
    for byte in bytes.iter().take(64) {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn text_or_hex(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) if text.chars().all(|c| !c.is_control() || c == '\n' || c == '\t') => {
            text.trim().chars().take(256).collect()
        }
        _ => hex(bytes),
    }
}

fn oid_name(arcs: &[u64]) -> String {
    let dotted = dotted(arcs);
    match well_known(arcs)
        .map(str::to_string)
        .or_else(|| dumbmonit_collectors::snmp::describe_oid(arcs))
    {
        Some(name) => format!("{name} ({dotted})"),
        None => dotted,
    }
}

fn value_text(tag: u8, bytes: &[u8]) -> String {
    match tag {
        TAG_INTEGER => integer(bytes).to_string(),
        TAG_OCTETS => text_or_hex(bytes),
        TAG_NULL => "null".to_string(),
        TAG_OID => decode_oid(bytes).map_or_else(|| hex(bytes), |arcs| oid_name(&arcs)),
        TAG_IPADDRESS if bytes.len() == 4 => {
            format!("{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3])
        }
        TAG_COUNTER32 | TAG_GAUGE32 | TAG_TIMETICKS | TAG_COUNTER64 => unsigned(bytes).to_string(),
        0x80 => "noSuchObject".to_string(),
        0x81 => "noSuchInstance".to_string(),
        0x82 => "endOfMibView".to_string(),
        TAG_OPAQUE => hex(bytes),
        _ => hex(bytes),
    }
}

/// Traps génériques (RFC 3418, SNMPv2-MIB).
fn well_known(arcs: &[u64]) -> Option<&'static str> {
    const BASE: [u64; 9] = [1, 3, 6, 1, 6, 3, 1, 1, 5];
    if arcs.len() == 10 && arcs[..9] == BASE {
        return match arcs[9] {
            1 => Some("coldStart"),
            2 => Some("warmStart"),
            3 => Some("linkDown"),
            4 => Some("linkUp"),
            5 => Some("authenticationFailure"),
            6 => Some("egpNeighborLoss"),
            _ => None,
        };
    }
    None
}

fn generic_v1(code: i64) -> &'static str {
    match code {
        0 => "coldStart",
        1 => "warmStart",
        2 => "linkDown",
        3 => "linkUp",
        4 => "authenticationFailure",
        5 => "egpNeighborLoss",
        _ => "enterpriseSpecific",
    }
}

/// Un varbind : son OID, sa valeur en texte, et la valeur elle-même quand c'est un OID.
type Varbind = (Vec<u64>, String, Option<Vec<u64>>);

fn read_varbinds(data: &[u8], start: usize, end: usize) -> Option<Vec<Varbind>> {
    let mut list = Reader::within(data, start, end);
    let mut out = Vec::new();
    while !list.done() && out.len() < MAX_VARBINDS {
        let (tag, s, e) = list.tlv()?;
        if tag != TAG_SEQUENCE {
            return None;
        }
        let mut pair = Reader::within(data, s, e);
        let (oid_tag, os, oe) = pair.tlv()?;
        if oid_tag != TAG_OID {
            return None;
        }
        let arcs = decode_oid(&data[os..oe])?;
        let (value_tag, vs, ve) = pair.tlv()?;
        let as_oid = if value_tag == TAG_OID { decode_oid(&data[vs..ve]) } else { None };
        out.push((arcs, value_text(value_tag, &data[vs..ve]), as_oid));
    }
    Some(out)
}

/// Décode un datagramme SNMP v1 ou v2c. `Err` dit pourquoi il est refusé.
pub fn decode(data: &[u8]) -> Result<Trap, &'static str> {
    let mut top = Reader::new(data);
    let (tag, start, end) = top.tlv().ok_or("not an SNMP message")?;
    if tag != TAG_SEQUENCE {
        return Err("not an SNMP message");
    }
    let mut message = Reader::within(data, start, end);
    let (t, s, e) = message.tlv().ok_or("truncated message")?;
    if t != TAG_INTEGER {
        return Err("not an SNMP message");
    }
    let version = integer(&data[s..e]);
    let version_name = match version {
        0 => "v1",
        1 => "v2c",
        3 => return Err("SNMPv3 traps are not supported"),
        _ => return Err("unknown SNMP version"),
    };
    let (t, s, e) = message.tlv().ok_or("truncated message")?;
    if t != TAG_OCTETS {
        return Err("no community");
    }
    let community = String::from_utf8_lossy(&data[s..e]).to_string();
    let pdu_offset = message.pos;
    let (pdu_tag, ps, pe) = message.tlv().ok_or("truncated PDU")?;
    let mut pdu = Reader::within(data, ps, pe);

    match (version, pdu_tag) {
        (0, PDU_TRAP_V1) => {
            let (t, s, e) = pdu.tlv().ok_or("truncated trap")?;
            if t != TAG_OID {
                return Err("malformed trap");
            }
            let enterprise = decode_oid(&data[s..e]).ok_or("malformed enterprise OID")?;
            let (t, s, e) = pdu.tlv().ok_or("truncated trap")?;
            let agent_addr = (t == TAG_IPADDRESS && e - s == 4)
                .then(|| format!("{}.{}.{}.{}", data[s], data[s + 1], data[s + 2], data[s + 3]));
            let (_, s, e) = pdu.tlv().ok_or("truncated trap")?;
            let generic = integer(&data[s..e]);
            let (_, s, e) = pdu.tlv().ok_or("truncated trap")?;
            let specific = integer(&data[s..e]);
            let _ticks = pdu.tlv().ok_or("truncated trap")?;
            let (t, s, e) = pdu.tlv().ok_or("truncated trap")?;
            if t != TAG_SEQUENCE {
                return Err("malformed trap");
            }
            let binds = read_varbinds(data, s, e).ok_or("malformed varbinds")?;
            let name = if generic == 6 {
                format!("enterpriseSpecific {}.{specific}", dotted(&enterprise))
            } else {
                generic_v1(generic).to_string()
            };
            Ok(Trap {
                version: version_name,
                community,
                trap_oid: enterprise,
                name,
                agent_addr,
                varbinds: binds.into_iter().map(|(oid, v, _)| (oid_name(&oid), v)).collect(),
                inform: false,
                pdu_offset,
            })
        }
        (1, PDU_TRAP_V2) | (1, PDU_INFORM) => {
            for _ in 0..3 {
                pdu.tlv().ok_or("truncated trap")?;
            }
            let (t, s, e) = pdu.tlv().ok_or("truncated trap")?;
            if t != TAG_SEQUENCE {
                return Err("malformed trap");
            }
            let binds = read_varbinds(data, s, e).ok_or("malformed varbinds")?;
            // snmpTrapOID.0 (1.3.6.1.6.3.1.1.4.1.0) nomme la trap.
            let trap_oid_name: [u64; 11] = [1, 3, 6, 1, 6, 3, 1, 1, 4, 1, 0];
            let mut trap_oid: Vec<u64> = Vec::new();
            let mut shown: Vec<(String, String)> = Vec::new();
            for (oid, value, as_oid) in binds {
                if oid == trap_oid_name {
                    trap_oid = as_oid.unwrap_or_default();
                    continue;
                }
                // sysUpTime.0 n'apporte rien au journal.
                if oid == [1, 3, 6, 1, 2, 1, 1, 3, 0] {
                    continue;
                }
                shown.push((oid_name(&oid), value));
            }
            let name = if trap_oid.is_empty() { "trap".to_string() } else { oid_name(&trap_oid) };
            Ok(Trap {
                version: version_name,
                community,
                trap_oid,
                name,
                agent_addr: None,
                varbinds: shown,
                inform: pdu_tag == PDU_INFORM,
                pdu_offset,
            })
        }
        _ => Err("not a trap"),
    }
}

/// La réponse à un « inform » : le même message, avec un PDU `Response`.
pub fn inform_response(data: &[u8], trap: &Trap) -> Option<Vec<u8>> {
    if !trap.inform || trap.pdu_offset >= data.len() {
        return None;
    }
    let mut reply = data.to_vec();
    reply[trap.pdu_offset] = PDU_RESPONSE;
    Some(reply)
}

/// Niveau d'une trap, d'après son nom.
fn level_of(trap: &Trap) -> &'static str {
    if trap.name.starts_with("linkDown") || trap.name.starts_with("authenticationFailure") {
        "warn"
    } else {
        "info"
    }
}

/// L'entrée de journal d'une trap reçue de `peer`.
pub fn entry(trap: &Trap, peer: IpAddr, now_ms: i64) -> (Entry, bool) {
    let origin_ip = trap
        .agent_addr
        .as_deref()
        .and_then(|addr| addr.parse::<IpAddr>().ok())
        .filter(|ip| !ip.is_unspecified());
    let found = attach::resolver()
        .lookup(peer, None)
        .or_else(|| origin_ip.and_then(|ip| attach::resolver().lookup(ip, None)));
    let mut message = trap.name.clone();
    for (name, value) in &trap.varbinds {
        message.push(' ');
        message.push_str(name);
        message.push('=');
        message.push_str(value);
        if message.len() >= MAX_MESSAGE {
            break;
        }
    }
    let mut end = message.len().min(MAX_MESSAGE);
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    message.truncate(end);
    let mut extra = BTreeMap::new();
    extra.insert("snmp_version".to_string(), trap.version.to_string());
    extra.insert("peer".to_string(), peer.to_canonical().to_string());
    if !trap.trap_oid.is_empty() {
        extra.insert("trap_oid".to_string(), dotted(&trap.trap_oid));
    }
    let attached = found.is_some();
    (
        Entry {
            ts_ms: now_ms,
            service: "trap",
            level: central::normalise_level(level_of(trap)),
            host: origin_ip.unwrap_or(peer).to_canonical().to_string(),
            origin: format!("trap:{}", peer.to_canonical()),
            message,
            target: found,
            extra,
        },
        attached,
    )
}

fn communities() -> Vec<String> {
    std::env::var("DUMBMONIT_TRAPS_COMMUNITIES")
        .unwrap_or_default()
        .split(',')
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty())
        .collect()
}

/// Lance le récepteur si l'environnement le demande.
pub fn spawn() {
    if !env_flag("DUMBMONIT_TRAPS_ENABLED", false) {
        return;
    }
    let address = SocketAddr::new(
        env_bind("DUMBMONIT_TRAPS_BIND"),
        env_port("DUMBMONIT_TRAPS_PORT", DEFAULT_PORT),
    );
    let allowed = communities();
    let input = central::traps_input();
    tokio::spawn(async move {
        let socket = match UdpSocket::bind(address).await {
            Ok(socket) => socket,
            Err(error) => {
                tracing::warn!(%address, %error, "SNMP trap port unavailable");
                input.set_error(Some(format!("UDP {address}: {error}")));
                return;
            }
        };
        tracing::info!(%address, "SNMP trap receiver listening (v1, v2c)");
        input.set_listening(vec![format!("udp/{}", address.port())]);
        let mut buffer = vec![0u8; MAX_DATAGRAM];
        loop {
            let (length, from) = match socket.recv_from(&mut buffer).await {
                Ok(received) => received,
                Err(error) => {
                    tracing::debug!(%error, "trap receive failed");
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    continue;
                }
            };
            let datagram = &buffer[..length];
            let trap = match decode(datagram) {
                Ok(trap) => trap,
                Err(why) => {
                    tracing::debug!(%from, why, "SNMP datagram refused");
                    input.note_rejected();
                    continue;
                }
            };
            if !allowed.is_empty() && !allowed.contains(&trap.community) {
                input.note_rejected();
                continue;
            }
            input.note_received();
            let (entry, attached) = entry(&trap, from.ip(), chrono::Utc::now().timestamp_millis());
            if !attached {
                input.note_unmatched();
            }
            hub().push(entry);
            if let Some(reply) = inform_response(datagram, &trap) {
                let _ = socket.send_to(&reply, from).await;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TLV court (longueur < 128).
    fn tlv(tag: u8, content: &[u8]) -> Vec<u8> {
        let mut out = vec![tag, content.len() as u8];
        out.extend_from_slice(content);
        out
    }

    fn oid(arcs: &[u64]) -> Vec<u8> {
        let mut bytes = vec![(arcs[0] * 40 + arcs[1]) as u8];
        for arc in &arcs[2..] {
            let mut chunk = vec![(*arc & 0x7f) as u8];
            let mut rest = *arc >> 7;
            while rest > 0 {
                chunk.push(((rest & 0x7f) as u8) | 0x80);
                rest >>= 7;
            }
            chunk.reverse();
            bytes.extend(chunk);
        }
        tlv(TAG_OID, &bytes)
    }

    fn v2c_link_down(version: u8, pdu: u8) -> Vec<u8> {
        let uptime =
            [oid(&[1, 3, 6, 1, 2, 1, 1, 3, 0]), tlv(TAG_TIMETICKS, &[0x01, 0x00])].concat();
        let trap = [oid(&[1, 3, 6, 1, 6, 3, 1, 1, 4, 1, 0]), oid(&[1, 3, 6, 1, 6, 3, 1, 1, 5, 3])]
            .concat();
        let if_index = [oid(&[1, 3, 6, 1, 2, 1, 2, 2, 1, 1, 2]), tlv(TAG_INTEGER, &[2])].concat();
        let binds = [tlv(0x30, &uptime), tlv(0x30, &trap), tlv(0x30, &if_index)].concat();
        let body = [
            tlv(TAG_INTEGER, &[1]),
            tlv(TAG_INTEGER, &[0]),
            tlv(TAG_INTEGER, &[0]),
            tlv(0x30, &binds),
        ]
        .concat();
        let message =
            [tlv(TAG_INTEGER, &[version]), tlv(TAG_OCTETS, b"public"), tlv(pdu, &body)].concat();
        tlv(0x30, &message)
    }

    #[test]
    fn a_v2c_link_down_trap_is_named_and_its_varbinds_kept() {
        let data = v2c_link_down(1, PDU_TRAP_V2);
        let trap = decode(&data).unwrap();
        assert_eq!(trap.version, "v2c");
        assert_eq!(trap.community, "public");
        assert!(trap.name.starts_with("linkDown"), "{}", trap.name);
        assert_eq!(dotted(&trap.trap_oid), "1.3.6.1.6.3.1.1.5.3");
        assert_eq!(trap.varbinds.len(), 1, "sysUpTime and snmpTrapOID are not repeated");
        assert!(trap.varbinds[0].1 == "2");
        assert!(!trap.inform);
        assert_eq!(level_of(&trap), "warn");
    }

    #[test]
    fn an_inform_is_answered_with_a_response_pdu() {
        let data = v2c_link_down(1, PDU_INFORM);
        let trap = decode(&data).unwrap();
        assert!(trap.inform);
        let reply = inform_response(&data, &trap).unwrap();
        assert_eq!(reply.len(), data.len());
        assert_eq!(reply[trap.pdu_offset], PDU_RESPONSE);
        assert_eq!(&reply[..trap.pdu_offset], &data[..trap.pdu_offset]);
        let plain = decode(&v2c_link_down(1, PDU_TRAP_V2)).unwrap();
        assert!(inform_response(&data, &plain).is_none());
    }

    #[test]
    fn a_v1_trap_is_decoded() {
        let enterprise = oid(&[1, 3, 6, 1, 4, 1, 8072, 3, 2, 10]);
        let binds = tlv(
            0x30,
            &tlv(0x30, &[oid(&[1, 3, 6, 1, 2, 1, 1, 5, 0]), tlv(TAG_OCTETS, b"core-sw")].concat()),
        );
        let body = [
            enterprise,
            tlv(TAG_IPADDRESS, &[10, 0, 0, 9]),
            tlv(TAG_INTEGER, &[6]),
            tlv(TAG_INTEGER, &[17]),
            tlv(TAG_TIMETICKS, &[5]),
            binds,
        ]
        .concat();
        let message =
            [tlv(TAG_INTEGER, &[0]), tlv(TAG_OCTETS, b"lab"), tlv(PDU_TRAP_V1, &body)].concat();
        let trap = decode(&tlv(0x30, &message)).unwrap();
        assert_eq!(trap.version, "v1");
        assert_eq!(trap.agent_addr.as_deref(), Some("10.0.0.9"));
        assert!(trap.name.contains("1.3.6.1.4.1.8072.3.2.10.17"), "{}", trap.name);
        assert_eq!(trap.varbinds[0].1, "core-sw");
    }

    #[test]
    fn v3_and_garbage_are_refused_without_panicking() {
        let v3 = tlv(0x30, &[tlv(TAG_INTEGER, &[3]), tlv(0x30, &[])].concat());
        assert_eq!(decode(&v3).unwrap_err(), "SNMPv3 traps are not supported");
        assert!(decode(&[]).is_err());
        assert!(decode(&[0x30, 0x84, 0xff, 0xff, 0xff, 0xff]).is_err());
        let good = v2c_link_down(1, PDU_TRAP_V2);
        for cut in 0..good.len() {
            let _ = decode(&good[..cut]);
        }
    }

    #[test]
    fn oids_and_integers_decode() {
        assert_eq!(decode_oid(&[0x2b, 6, 1]).unwrap(), vec![1, 3, 6, 1]);
        assert_eq!(decode_oid(&[0x2b, 0x86, 0x48]).unwrap(), vec![1, 3, 840]);
        assert!(decode_oid(&[0x2b, 0x86]).is_none());
        assert_eq!(integer(&[0xff]), -1);
        assert_eq!(integer(&[0x01, 0x00]), 256);
        assert_eq!(unsigned(&[0x00, 0xff, 0xff]), 65_535);
    }

    #[test]
    fn the_entry_names_the_trap_and_carries_the_sender() {
        let trap = decode(&v2c_link_down(1, PDU_TRAP_V2)).unwrap();
        let (entry, attached) = entry(&trap, "10.0.0.7".parse().unwrap(), 9);
        assert!(!attached);
        assert_eq!(entry.service, "trap");
        assert_eq!(entry.level, "warn");
        assert_eq!(entry.host, "10.0.0.7");
        assert!(entry.message.starts_with("linkDown"));
        assert_eq!(entry.extra["trap_oid"], "1.3.6.1.6.3.1.1.5.3");
    }
}
