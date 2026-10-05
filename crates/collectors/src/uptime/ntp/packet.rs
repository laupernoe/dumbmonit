//! Encodage et décodage du paquet SNTP (RFC 4330 / RFC 5905), 48 octets.
//!
//! Module purement fonctionnel — aucune entrée-sortie — pour que le format
//! binaire se teste sans serveur NTP en face : c'est la seule partie où un
//! décalage d'un octet casserait silencieusement la mesure.
//!
//! ```text
//!  0                   1                   2                   3
//!  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |LI | VN  |Mode |    Stratum    |     Poll      |   Precision   |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                          Root Delay                          |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                       Root Dispersion                        |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                     Reference Identifier                     |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                    Reference Timestamp (64)                  |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                    Origin Timestamp (64)                     |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                    Receive Timestamp (64)                    |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                    Transmit Timestamp (64)                   |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! ```

/// Taille exacte d'un paquet NTP sans extension ni authentification.
pub const PACKET_SIZE: usize = 48;

/// Écart entre l'ère NTP (1ᵉʳ janvier 1900) et l'ère Unix (1ᵉʳ janvier 1970),
/// en secondes. Les deux ères partagent le même sens de décompte.
const NTP_UNIX_EPOCH_DELTA: f64 = 2_208_988_800.0;

/// Version NTP annoncée par le client. 4 est la version courante, comprise par
/// tout serveur depuis les années 2000 (y compris les serveurs stratum 1 publics).
const CLIENT_VERSION: u8 = 4;

/// Mode « client », envoyé dans la requête.
const MODE_CLIENT: u8 = 3;

/// Mode « serveur », attendu dans la réponse.
const MODE_SERVER: u8 = 4;

/// Indicateur de correction horaire (« leap second »), premiers deux bits du
/// premier octet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeapIndicator {
    /// Pas de correction annoncée : l'horloge du serveur est fiable.
    NoWarning,
    /// La dernière minute de la journée comptera 61 secondes.
    LastMinute61,
    /// La dernière minute de la journée comptera 59 secondes.
    LastMinute59,
    /// Horloge non synchronisée : l'alarme du protocole lui-même.
    Unsynchronized,
}

impl LeapIndicator {
    fn from_bits(bits: u8) -> Self {
        match bits & 0b11 {
            0 => Self::NoWarning,
            1 => Self::LastMinute61,
            2 => Self::LastMinute59,
            _ => Self::Unsynchronized,
        }
    }

    /// Valeur numérique publiée en métrique : l'ordre croissant suit la gravité.
    pub fn as_value(self) -> f64 {
        match self {
            Self::NoWarning => 0.0,
            Self::LastMinute61 => 1.0,
            Self::LastMinute59 => 2.0,
            Self::Unsynchronized => 3.0,
        }
    }
}

/// Réponse décodée, avec les quatre horodatages nécessaires au calcul de
/// l'écart et du délai (RFC 5905 §8).
#[derive(Debug, Clone, PartialEq)]
pub struct Response {
    pub leap_indicator: LeapIndicator,
    pub version: u8,
    pub mode: u8,
    pub stratum: u8,
    /// Intervalle de scrutation annoncé par le serveur, en secondes (puissance de deux).
    pub poll: i8,
    /// Précision de l'horloge du serveur, en secondes (puissance de deux, négative).
    pub precision: i8,
    pub root_delay_s: f64,
    pub root_dispersion_s: f64,
    /// Identifiant de référence brut : quatre octets, dont le sens dépend du
    /// stratum (voir [`reference_id_string`]).
    pub reference_id: [u8; 4],
    pub reference_timestamp: f64,
    /// T1 : l'horodatage que *nous* avions mis dans la requête, renvoyé tel quel.
    pub origin_timestamp: f64,
    /// T2 : réception par le serveur.
    pub receive_timestamp: f64,
    /// T3 : émission par le serveur.
    pub transmit_timestamp: f64,
}

/// Construit la requête SNTP. `originate_unix` est T1, l'horloge locale au
/// moment de l'émission : la réponse la renverra à l'identique dans le champ
/// « Origin Timestamp », ce qui permet de vérifier qu'elle correspond bien à
/// cette requête et non à une réponse tardive d'un échange précédent.
pub fn encode_request(originate_unix: f64) -> [u8; PACKET_SIZE] {
    let mut packet = [0u8; PACKET_SIZE];
    // LI = 0 (le client n'a pas d'avis), VN = 4, Mode = 3 (client).
    packet[0] = (CLIENT_VERSION << 3) | MODE_CLIENT;
    packet[40..48].copy_from_slice(&encode_timestamp(originate_unix));
    packet
}

/// Décode une réponse. Rejette ce qui n'a pas la taille d'un paquet NTP ou qui
/// n'annonce pas le mode « serveur » : un relais mal configuré ou un service
/// quelconque qui répond sur le port 123 doit se voir comme un protocole
/// inattendu, pas comme une horloge désynchronisée.
pub fn decode_response(buf: &[u8]) -> Result<Response, String> {
    if buf.len() < PACKET_SIZE {
        return Err(format!(
            "short NTP packet: {} bytes, expected at least {PACKET_SIZE}",
            buf.len()
        ));
    }
    let first = buf[0];
    let leap_indicator = LeapIndicator::from_bits(first >> 6);
    let version = (first >> 3) & 0b111;
    let mode = first & 0b111;
    if mode != MODE_SERVER {
        return Err(format!("unexpected NTP mode {mode} (expected {MODE_SERVER}, server)"));
    }

    Ok(Response {
        leap_indicator,
        version,
        mode,
        stratum: buf[1],
        poll: buf[2] as i8,
        precision: buf[3] as i8,
        root_delay_s: decode_short(&buf[4..8]),
        root_dispersion_s: decode_short(&buf[8..12]),
        reference_id: [buf[12], buf[13], buf[14], buf[15]],
        reference_timestamp: decode_timestamp(&buf[16..24]),
        origin_timestamp: decode_timestamp(&buf[24..32]),
        receive_timestamp: decode_timestamp(&buf[32..40]),
        transmit_timestamp: decode_timestamp(&buf[40..48]),
    })
}

/// Un identifiant de référence lisible : les quatre octets d'un stratum 0 ou 1
/// sont un code ASCII (« GPS\0 », « LOCL »), ceux d'un stratum supérieur
/// l'adresse IPv4 du serveur en amont.
pub fn reference_id_string(stratum: u8, bytes: [u8; 4]) -> String {
    if stratum <= 1 {
        let text: String = bytes
            .iter()
            .copied()
            .take_while(|b| *b != 0)
            .map(|b| if b.is_ascii_graphic() { b as char } else { '?' })
            .collect();
        if text.is_empty() { "unspecified".to_string() } else { text }
    } else {
        format!("{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3])
    }
}

/// Un stratum nul est un paquet « kiss-o'-death » : le serveur refuse de
/// répondre et le dit dans l'identifiant de référence (« DENY », « RSTR », …)
/// plutôt que de rester muet.
pub fn is_kiss_of_death(stratum: u8) -> bool {
    stratum == 0
}

/// Stratum 16 : le serveur tourne mais n'a jamais pu se synchroniser.
pub fn is_unsynchronized(stratum: u8, leap_indicator: LeapIndicator) -> bool {
    stratum == 0 || stratum >= 16 || leap_indicator == LeapIndicator::Unsynchronized
}

/// Format court (32 bits signés, 16.16) utilisé par Root Delay et Root
/// Dispersion : toujours positif en pratique, mais le calcul reste celui d'un
/// entier signé.
fn decode_short(bytes: &[u8]) -> f64 {
    let raw = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    f64::from(raw) / 65_536.0
}

/// Horodatage NTP 64 bits (32.32) vers un temps Unix en secondes.
fn decode_timestamp(bytes: &[u8]) -> f64 {
    let seconds = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let fraction = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    if seconds == 0 && fraction == 0 {
        return 0.0;
    }
    f64::from(seconds) - NTP_UNIX_EPOCH_DELTA + f64::from(fraction) / 4_294_967_296.0
}

/// Temps Unix en secondes vers un horodatage NTP 64 bits (32.32).
///
/// Une seule ère est gérée, celle en cours (le champ « secondes » déborde le
/// 7 février 2036) : largement suffisant pour une requête qui ne sert qu'à
/// dater « maintenant », jamais à interpréter l'horodatage d'autrui au-delà
/// de cette date.
fn encode_timestamp(unix_seconds: f64) -> [u8; 8] {
    let ntp_seconds = unix_seconds + NTP_UNIX_EPOCH_DELTA;
    let seconds = ntp_seconds.trunc().max(0.0) as u32;
    let fraction = (ntp_seconds.fract() * 4_294_967_296.0) as u32;
    let mut out = [0u8; 8];
    out[0..4].copy_from_slice(&seconds.to_be_bytes());
    out[4..8].copy_from_slice(&fraction.to_be_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La requête annonce la version 4 en mode client, et porte T1 dans le
    /// dernier champ — rien d'autre n'est rempli.
    #[test]
    fn la_requete_porte_la_version_le_mode_et_lhorodatage_demission() {
        let packet = encode_request(1_700_000_000.5);
        assert_eq!(packet[0], (4 << 3) | 3);
        assert!(packet[1..40].iter().all(|b| *b == 0), "rien d'autre n'est rempli");
        let transmit = decode_timestamp(&packet[40..48]);
        assert!((transmit - 1_700_000_000.5).abs() < 1e-6);
    }

    #[test]
    fn un_aller_retour_horodatage_est_stable_au_microseconde() {
        // Le dernier instant est la borne haute de l'ère NTP courante (champ
        // « secondes » sur 32 bits, qui déborde en 2036) : au-delà, encoder
        // saturerait silencieusement, ce que ce module ne prétend pas corriger.
        for instant in [0.0, 1.0, 1_700_000_000.123_456, 2_085_978_495.999_999] {
            let encoded = encode_timestamp(instant);
            let decoded = decode_timestamp(&encoded);
            assert!((decoded - instant).abs() < 1e-5, "{instant} -> {decoded}");
        }
    }

    /// Un vrai paquet de réponse, construit octet par octet d'après l'exemple
    /// de la RFC 5905 (stratum 1, référence « GPS »).
    #[test]
    fn une_reponse_valide_se_decode_entierement() {
        let mut buf = [0u8; PACKET_SIZE];
        // LI=0, VN=4, Mode=4 (serveur).
        buf[0] = (4 << 3) | 4;
        buf[1] = 1; // stratum 1
        buf[2] = 4; // poll = 2^4 = 16 s
        buf[3] = 0xF7u8; // precision = -9 (2^-9 s)
        buf[12..16].copy_from_slice(b"GPS\0");
        buf[24..32].copy_from_slice(&encode_timestamp(1_700_000_000.0)); // origin
        buf[32..40].copy_from_slice(&encode_timestamp(1_700_000_000.25)); // receive
        buf[40..48].copy_from_slice(&encode_timestamp(1_700_000_000.26)); // transmit

        let response = decode_response(&buf).unwrap();
        assert_eq!(response.leap_indicator, LeapIndicator::NoWarning);
        assert_eq!(response.version, 4);
        assert_eq!(response.mode, 4);
        assert_eq!(response.stratum, 1);
        assert_eq!(response.poll, 4);
        assert_eq!(response.precision, -9);
        assert_eq!(reference_id_string(response.stratum, response.reference_id), "GPS");
        assert!((response.origin_timestamp - 1_700_000_000.0).abs() < 1e-5);
        assert!((response.receive_timestamp - 1_700_000_000.25).abs() < 1e-5);
        assert!((response.transmit_timestamp - 1_700_000_000.26).abs() < 1e-5);
        assert!(!is_unsynchronized(response.stratum, response.leap_indicator));
    }

    #[test]
    fn un_stratum_secondaire_rend_son_identifiant_en_adresse_ip() {
        assert_eq!(reference_id_string(2, [10, 0, 0, 1]), "10.0.0.1");
    }

    #[test]
    fn un_mode_different_de_serveur_est_rejete() {
        let mut buf = [0u8; PACKET_SIZE];
        buf[0] = (4 << 3) | 3; // mode client, pas serveur
        let error = decode_response(&buf).unwrap_err();
        assert!(error.contains("mode"), "{error}");
    }

    #[test]
    fn un_paquet_trop_court_est_rejete() {
        assert!(decode_response(&[0u8; 10]).is_err());
    }

    #[test]
    fn le_stratum_seize_et_lindicateur_de_correction_signalent_une_desynchronisation() {
        assert!(is_unsynchronized(16, LeapIndicator::NoWarning));
        assert!(is_unsynchronized(0, LeapIndicator::NoWarning), "kiss-o'-death");
        assert!(is_unsynchronized(1, LeapIndicator::Unsynchronized));
        assert!(!is_unsynchronized(2, LeapIndicator::NoWarning));
        assert!(is_kiss_of_death(0));
        assert!(!is_kiss_of_death(1));
    }

    #[test]
    fn un_identifiant_de_reference_vide_est_signale() {
        assert_eq!(reference_id_string(1, [0, 0, 0, 0]), "unspecified");
    }
}
