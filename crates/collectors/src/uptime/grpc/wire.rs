//! Le peu de protobuf et de gRPC dont la sonde a besoin, écrit à la main.
//!
//! Le protocole de santé standard (`grpc.health.v1`) ne manipule que deux
//! messages minuscules :
//!
//! ```text
//! message HealthCheckRequest  { string service = 1; }
//! message HealthCheckResponse { ServingStatus status = 1; }
//! ```
//!
//! Un schéma de plus de deux champs justifierait un générateur de code ; ici, un
//! octet d'étiquette, une longueur et quelques octets suffisent, et la sonde n'a
//! ni `prost` ni `tonic` à embarquer dans l'agent relais.
//!
//! Sur le fil, chaque message gRPC est précédé de cinq octets : un drapeau de
//! compression et la longueur du message sur quatre octets, big-endian.

/// Valeurs de l'énumération `ServingStatus` du protocole de santé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Serving {
    Unknown,
    Serving,
    NotServing,
    ServiceUnknown,
    /// Une valeur que ce protocole n'a pas définie (un serveur plus récent).
    Other(u64),
}

impl Serving {
    fn from_wire(value: u64) -> Self {
        match value {
            0 => Self::Unknown,
            1 => Self::Serving,
            2 => Self::NotServing,
            3 => Self::ServiceUnknown,
            other => Self::Other(other),
        }
    }

    /// Valeur numérique publiée en métrique.
    pub fn value(self) -> f64 {
        match self {
            Self::Unknown => 0.0,
            Self::Serving => 1.0,
            Self::NotServing => 2.0,
            Self::ServiceUnknown => 3.0,
            Self::Other(value) => value as f64,
        }
    }

    pub fn name(self) -> String {
        match self {
            Self::Unknown => "UNKNOWN".to_string(),
            Self::Serving => "SERVING".to_string(),
            Self::NotServing => "NOT_SERVING".to_string(),
            Self::ServiceUnknown => "SERVICE_UNKNOWN".to_string(),
            Self::Other(value) => format!("status {value}"),
        }
    }
}

/// Écrit un entier en varint (7 bits par octet, poids faible d'abord).
fn write_varint(out: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        out.push((value & 0x7f) as u8 | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}

/// Lit un varint ; rend la valeur et le nombre d'octets consommés.
fn read_varint(bytes: &[u8]) -> Result<(u64, usize), String> {
    let mut value = 0u64;
    for (index, byte) in bytes.iter().enumerate() {
        if index >= 10 {
            break;
        }
        value |= u64::from(byte & 0x7f) << (7 * index);
        if byte & 0x80 == 0 {
            return Ok((value, index + 1));
        }
    }
    Err("truncated or oversized varint".to_string())
}

/// `HealthCheckRequest { service }`. Un nom vide est la valeur par défaut de
/// proto3 : le champ est omis, et le message est vide (état du serveur entier).
pub fn encode_request(service: &str) -> Vec<u8> {
    let mut message = Vec::new();
    if !service.is_empty() {
        message.push(0x0a); // champ 1, type « longueur + octets »
        write_varint(&mut message, service.len() as u64);
        message.extend_from_slice(service.as_bytes());
    }
    message
}

/// Précède un message du préfixe gRPC de cinq octets (non compressé).
pub fn frame(message: &[u8]) -> Vec<u8> {
    let mut framed = Vec::with_capacity(5 + message.len());
    framed.push(0);
    framed.extend_from_slice(&(message.len() as u32).to_be_bytes());
    framed.extend_from_slice(message);
    framed
}

/// Extrait le premier message d'un corps gRPC. `None` quand le corps est vide
/// (réponse « Trailers-Only » ou serveur qui n'a rien renvoyé).
pub fn unframe(body: &[u8]) -> Result<Option<&[u8]>, String> {
    if body.is_empty() {
        return Ok(None);
    }
    if body.len() < 5 {
        return Err("the reply body is shorter than a gRPC message header".to_string());
    }
    if body[0] != 0 {
        return Err("the server compressed its reply, which the check did not ask for".to_string());
    }
    let length = u32::from_be_bytes([body[1], body[2], body[3], body[4]]) as usize;
    match body[5..].get(..length) {
        Some(message) => Ok(Some(message)),
        None => {
            Err(format!("the reply announces {length} bytes but only {} arrived", body.len() - 5))
        }
    }
}

/// `HealthCheckResponse { status }`. Un champ absent vaut `UNKNOWN` (0), la
/// valeur par défaut de proto3 ; les champs inconnus sont sautés.
pub fn decode_response(message: &[u8]) -> Result<Serving, String> {
    let mut status = 0u64;
    let mut position = 0;
    while position < message.len() {
        let (tag, used) = read_varint(&message[position..])?;
        position += used;
        let (field, wire_type) = (tag >> 3, tag & 7);
        let skip = match wire_type {
            0 => {
                let (value, used) = read_varint(&message[position..])?;
                if field == 1 {
                    status = value;
                }
                used
            }
            1 => 8,
            2 => {
                let (length, used) = read_varint(&message[position..])?;
                used + usize::try_from(length).map_err(|_| "field too long".to_string())?
            }
            5 => 4,
            other => return Err(format!("unsupported protobuf wire type {other}")),
        };
        position = position
            .checked_add(skip)
            .filter(|end| *end <= message.len())
            .ok_or_else(|| "a protobuf field runs past the end of the message".to_string())?;
    }
    Ok(Serving::from_wire(status))
}

/// Nom d'un code `grpc-status` (spécification gRPC), pour les messages d'échec.
pub fn status_name(code: u32) -> &'static str {
    match code {
        0 => "OK",
        1 => "CANCELLED",
        2 => "UNKNOWN",
        3 => "INVALID_ARGUMENT",
        4 => "DEADLINE_EXCEEDED",
        5 => "NOT_FOUND",
        6 => "ALREADY_EXISTS",
        7 => "PERMISSION_DENIED",
        8 => "RESOURCE_EXHAUSTED",
        9 => "FAILED_PRECONDITION",
        10 => "ABORTED",
        11 => "OUT_OF_RANGE",
        12 => "UNIMPLEMENTED",
        13 => "INTERNAL",
        14 => "UNAVAILABLE",
        15 => "DATA_LOSS",
        16 => "UNAUTHENTICATED",
        _ => "unknown status",
    }
}

/// Décode un `grpc-message` (pourcentage-encodé, RFC 3986), borné à 200 caractères.
pub fn decode_message(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut position = 0;
    while position < bytes.len() {
        let hex = bytes
            .get(position + 1..position + 3)
            .and_then(|pair| std::str::from_utf8(pair).ok())
            .and_then(|pair| u8::from_str_radix(pair, 16).ok());
        match (bytes[position], hex) {
            (b'%', Some(byte)) => {
                decoded.push(byte);
                position += 3;
            }
            (byte, _) => {
                decoded.push(byte);
                position += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded)
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(200)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn une_requete_sans_service_est_vide() {
        assert!(encode_request("").is_empty());
        assert_eq!(frame(&[]), [0, 0, 0, 0, 0]);
    }

    #[test]
    fn une_requete_avec_service_suit_le_format_protobuf() {
        // champ 1 (étiquette 0x0a), longueur 2, « db ».
        assert_eq!(encode_request("db"), [0x0a, 2, b'd', b'b']);
        let long = "s".repeat(200);
        let message = encode_request(&long);
        // 200 = 0xC8 0x01 en varint.
        assert_eq!(&message[..3], [0x0a, 0xc8, 0x01]);
        assert_eq!(message.len(), 3 + 200);
    }

    #[test]
    fn le_prefixe_grpc_porte_la_longueur_en_big_endian() {
        let framed = frame(&[1, 2, 3]);
        assert_eq!(framed, [0, 0, 0, 0, 3, 1, 2, 3]);
        assert_eq!(unframe(&framed).unwrap(), Some(&[1u8, 2, 3][..]));
    }

    #[test]
    fn un_corps_vide_ou_incoherent_est_traite_sans_paniquer() {
        assert_eq!(unframe(&[]).unwrap(), None);
        assert!(unframe(&[0, 0, 0]).is_err());
        assert!(unframe(&[1, 0, 0, 0, 0]).unwrap_err().contains("compressed"));
        assert!(unframe(&[0, 0, 0, 0, 9, 1]).unwrap_err().contains("announces"));
        assert!(unframe(&[0, 255, 255, 255, 255, 1]).is_err());
    }

    #[test]
    fn la_reponse_de_sante_se_decode() {
        assert_eq!(decode_response(&[0x08, 1]).unwrap(), Serving::Serving);
        assert_eq!(decode_response(&[0x08, 2]).unwrap(), Serving::NotServing);
        assert_eq!(decode_response(&[0x08, 3]).unwrap(), Serving::ServiceUnknown);
        assert_eq!(decode_response(&[0x08, 9]).unwrap(), Serving::Other(9));
        // Message vide : valeur par défaut de proto3.
        assert_eq!(decode_response(&[]).unwrap(), Serving::Unknown);
    }

    #[test]
    fn les_champs_inconnus_sont_sautes() {
        // champ 2 (longueur 3), champ 3 (varint 300), champ 1 = SERVING.
        let message = [0x12, 3, b'a', b'b', b'c', 0x18, 0xac, 0x02, 0x08, 1];
        assert_eq!(decode_response(&message).unwrap(), Serving::Serving);
        // champ 4 sur 32 bits et champ 5 sur 64 bits.
        let message = [0x25, 0, 0, 0, 0, 0x29, 0, 0, 0, 0, 0, 0, 0, 0, 0x08, 2];
        assert_eq!(decode_response(&message).unwrap(), Serving::NotServing);
    }

    #[test]
    fn un_message_tronque_est_refuse() {
        for message in [&[0x08][..], &[0x12, 9, 1][..], &[0x0d, 1, 2][..], &[0x0b][..], &[0x80][..]]
        {
            assert!(decode_response(message).is_err(), "{message:?}");
        }
    }

    #[test]
    fn les_valeurs_publiees_suivent_lenumeration() {
        assert_eq!(Serving::Serving.value(), 1.0);
        assert_eq!(Serving::NotServing.value(), 2.0);
        assert_eq!(Serving::Other(7).value(), 7.0);
        assert_eq!(Serving::ServiceUnknown.name(), "SERVICE_UNKNOWN");
        assert_eq!(Serving::Other(7).name(), "status 7");
    }

    #[test]
    fn les_codes_grpc_ont_un_nom() {
        assert_eq!(status_name(0), "OK");
        assert_eq!(status_name(12), "UNIMPLEMENTED");
        assert_eq!(status_name(14), "UNAVAILABLE");
        assert_eq!(status_name(99), "unknown status");
    }

    #[test]
    fn le_message_grpc_est_decode_et_borne() {
        assert_eq!(decode_message("service%20down%3A%20oops"), "service down: oops");
        assert_eq!(decode_message("100%"), "100%");
        assert_eq!(decode_message("%zz"), "%zz");
        assert_eq!(decode_message("a%0Ab"), "a b");
        assert_eq!(decode_message(&"x".repeat(500)).len(), 200);
    }
}
