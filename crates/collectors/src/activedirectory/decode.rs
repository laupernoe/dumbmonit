//! Décodage des valeurs brutes d'Active Directory.
//!
//! Tout arrive en texte (ou en octets pour les SID) : des drapeaux
//! `userAccountControl` en entier décimal, des dates en FILETIME (centaines de
//! nanosecondes depuis 1601), des durées en FILETIME *négatif*, des dates
//! « generalized time » (`20261004123456.0Z`), des DN à découper en respectant
//! les virgules échappées. Rien ici ne parle au réseau : tout se teste sur des
//! chaînes.

use chrono::{NaiveDateTime, TimeZone, Utc};

// --------------------------------------------------------------------------
// userAccountControl
// --------------------------------------------------------------------------

/// Les bits de `userAccountControl` lus par l'analyse ([MS-ADTS] 2.2.16).
pub mod uac {
    pub const ACCOUNTDISABLE: u32 = 0x0000_0002;
    pub const LOCKOUT: u32 = 0x0000_0010;
    pub const PASSWD_NOTREQD: u32 = 0x0000_0020;
    pub const ENCRYPTED_TEXT_PWD_ALLOWED: u32 = 0x0000_0080;
    pub const NORMAL_ACCOUNT: u32 = 0x0000_0200;
    pub const WORKSTATION_TRUST_ACCOUNT: u32 = 0x0000_1000;
    pub const SERVER_TRUST_ACCOUNT: u32 = 0x0000_2000;
    pub const DONT_EXPIRE_PASSWORD: u32 = 0x0001_0000;
    pub const TRUSTED_FOR_DELEGATION: u32 = 0x0008_0000;
    pub const NOT_DELEGATED: u32 = 0x0010_0000;
    pub const USE_DES_KEY_ONLY: u32 = 0x0020_0000;
    pub const DONT_REQ_PREAUTH: u32 = 0x0040_0000;
    pub const TRUSTED_TO_AUTH_FOR_DELEGATION: u32 = 0x0100_0000;
    /// Contrôleur de domaine en lecture seule (RODC).
    pub const PARTIAL_SECRETS_ACCOUNT: u32 = 0x0400_0000;
}

/// Lit un `userAccountControl`. AD l'écrit en entier signé sur 32 bits ; une
/// valeur négative garde ses bits.
pub fn parse_uac(raw: &str) -> Option<u32> {
    let value: i64 = raw.trim().parse().ok()?;
    if value < i64::from(i32::MIN) || value > i64::from(u32::MAX) {
        return None;
    }
    Some(value as u32)
}

// --------------------------------------------------------------------------
// Dates et durées
// --------------------------------------------------------------------------

/// Écart entre l'origine FILETIME (1601-01-01) et l'époque Unix, en secondes.
const FILETIME_UNIX_OFFSET_S: i64 = 11_644_473_600;
const TICKS_PER_SECOND: i64 = 10_000_000;

/// Date FILETIME (`lastLogonTimestamp`, `pwdLastSet`, `lockoutTime`…) en
/// secondes Unix. `0` et `0x7FFFFFFFFFFFFFFF` signifient « jamais » : `None`.
pub fn filetime_to_unix(raw: &str) -> Option<i64> {
    let ticks: i64 = raw.trim().parse().ok()?;
    if ticks <= 0 || ticks == i64::MAX {
        return None;
    }
    Some(ticks / TICKS_PER_SECOND - FILETIME_UNIX_OFFSET_S)
}

/// Durée de stratégie (`maxPwdAge`, `lockoutDuration`…), stockée en FILETIME
/// négatif. `None` pour « jamais » : `0` ou `-9223372036854775808`, les deux
/// formes qu'AD emploie selon l'outil qui a écrit la stratégie.
pub fn interval_seconds(raw: &str) -> Option<i64> {
    let ticks: i64 = raw.trim().parse().ok()?;
    if ticks == 0 || ticks == i64::MIN {
        return None;
    }
    Some((ticks / TICKS_PER_SECOND).abs())
}

/// Date « generalized time » (`whenCreated`, `currentTime`) en secondes Unix :
/// `20261004123456.0Z`. AD écrit toujours en UTC ; la fraction est ignorée.
pub fn generalized_time_to_unix(raw: &str) -> Option<i64> {
    let raw = raw.trim();
    let digits = raw.get(..14)?;
    if !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let naive = NaiveDateTime::parse_from_str(digits, "%Y%m%d%H%M%S").ok()?;
    Some(Utc.from_utc_datetime(&naive).timestamp())
}

/// Date ISO 8601 des voisins de réplication (`2026-10-04T08:15:30Z`). L'origine
/// FILETIME (`1601-01-01T00:00:00Z`) signifie « jamais ».
pub fn iso_time_to_unix(raw: &str) -> Option<i64> {
    let parsed = chrono::DateTime::parse_from_rfc3339(raw.trim()).ok()?;
    let unix = parsed.timestamp();
    (unix > -FILETIME_UNIX_OFFSET_S).then_some(unix)
}

// --------------------------------------------------------------------------
// SID
// --------------------------------------------------------------------------

/// SID binaire (`objectSid`) en notation `S-1-5-21-…`.
///
/// Révision (1 octet), nombre de sous-autorités (1), autorité sur 6 octets
/// gros-boutistes, puis chaque sous-autorité sur 4 octets petits-boutistes.
pub fn sid_to_string(bytes: &[u8]) -> Option<String> {
    if bytes.len() < 8 {
        return None;
    }
    let revision = bytes[0];
    let count = usize::from(bytes[1]);
    if bytes.len() != 8 + 4 * count {
        return None;
    }
    let authority = bytes[2..8].iter().fold(0u64, |acc, b| (acc << 8) | u64::from(*b));
    let mut out = format!("S-{revision}-{authority}");
    let (chunks, _) = bytes[8..].as_chunks::<4>();
    for chunk in chunks {
        let sub = u32::from_le_bytes(*chunk);
        out.push_str(&format!("-{sub}"));
    }
    Some(out)
}

/// Dernière sous-autorité d'un SID : le RID (`500` pour l'administrateur
/// intégré, `501` pour l'invité, `502` pour krbtgt).
pub fn rid_of(sid: &str) -> Option<u32> {
    sid.rsplit('-').next()?.parse().ok()
}

// --------------------------------------------------------------------------
// Noms distinctifs
// --------------------------------------------------------------------------

/// Découpe un DN en composants `(type, valeur)`, en respectant les virgules
/// échappées (`CN=Dupont\, Jean,OU=…`). Les valeurs sont rendues sans leurs
/// échappements.
pub fn dn_components(dn: &str) -> Vec<(String, String)> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut escaped = false;
    for c in dn.chars() {
        if escaped {
            current.push('\\');
            current.push(c);
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == ',' {
            parts.push(std::mem::take(&mut current));
        } else {
            current.push(c);
        }
    }
    if !current.is_empty() || !parts.is_empty() {
        parts.push(current);
    }
    parts
        .into_iter()
        .filter_map(|part| {
            let (kind, value) = part.split_once('=')?;
            Some((kind.trim().to_ascii_uppercase(), unescape_dn_value(value.trim())))
        })
        .collect()
}

fn unescape_dn_value(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next) = chars.next() {
                out.push(next);
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Valeur du premier composant d'un DN : `CN=DC1,OU=…` donne `DC1`.
pub fn first_rdn_value(dn: &str) -> Option<String> {
    dn_components(dn).into_iter().next().map(|(_, value)| value)
}

/// Nom DNS d'un domaine à partir de son DN : `DC=corp,DC=example,DC=com` donne
/// `corp.example.com`.
pub fn dns_name_of(dn: &str) -> String {
    dn_components(dn)
        .into_iter()
        .filter(|(kind, _)| kind == "DC")
        .map(|(_, value)| value)
        .collect::<Vec<_>>()
        .join(".")
}

/// Le serveur et le site désignés par le DN d'un objet `nTDSDSA` (« NTDS
/// Settings ») : `CN=NTDS Settings,CN=DC1,CN=Servers,CN=Paris,CN=Sites,…`
/// donne `("DC1", "Paris")`. C'est sous cette forme qu'AD désigne le détenteur
/// d'un rôle FSMO et la source d'une réplication.
pub fn server_and_site_of_dsa(dn: &str) -> Option<(String, String)> {
    let components = dn_components(dn);
    let index = components
        .iter()
        .position(|(kind, value)| kind == "CN" && value.eq_ignore_ascii_case("Servers"))?;
    let server = components.get(index.checked_sub(1)?)?.1.clone();
    let site = components.get(index + 1)?.1.clone();
    Some((server, site))
}

/// Compare deux DN sans tenir compte de la casse ni des espaces autour des
/// composants, comme le fait l'annuaire.
pub fn same_dn(a: &str, b: &str) -> bool {
    let a = dn_components(a);
    let b = dn_components(b);
    a.len() == b.len()
        && a.iter().zip(&b).all(|((ka, va), (kb, vb))| ka == kb && va.eq_ignore_ascii_case(vb))
}

// --------------------------------------------------------------------------
// Niveaux fonctionnels
// --------------------------------------------------------------------------

/// Version de Windows Server correspondant à un niveau fonctionnel
/// (`domainFunctionality`, `forestFunctionality`).
pub fn functional_level_label(level: i64) -> String {
    match level {
        0 => "Windows 2000".to_string(),
        1 => "Windows Server 2003 interim".to_string(),
        2 => "Windows Server 2003".to_string(),
        3 => "Windows Server 2008".to_string(),
        4 => "Windows Server 2008 R2".to_string(),
        5 => "Windows Server 2012".to_string(),
        6 => "Windows Server 2012 R2".to_string(),
        7 => "Windows Server 2016".to_string(),
        10 => "Windows Server 2025".to_string(),
        other => format!("level {other}"),
    }
}

// --------------------------------------------------------------------------
// Erreurs de liaison
// --------------------------------------------------------------------------

/// Traduit le message de diagnostic d'un refus de liaison (code LDAP 49).
///
/// AD glisse la raison dans le texte, sous la forme `data 52e` : c'est la seule
/// façon de distinguer un mot de passe faux d'un compte expiré ou verrouillé.
pub fn explain_bind_failure(rc: u32, diagnostic: &str) -> String {
    let lower = diagnostic.to_ascii_lowercase();
    let code = lower
        .split("data ")
        .nth(1)
        .and_then(|rest| rest.split([',', ' ']).next())
        .unwrap_or_default()
        .to_string();
    let reason = match (rc, code.as_str()) {
        (49, "525") => "the account does not exist",
        (49, "52e") => "wrong user name or password",
        (49, "530") => "the account may not log on at this time of day",
        (49, "531") => "the account may not log on from this computer",
        (49, "532") => "the account's password has expired",
        (49, "533") => "the account is disabled",
        (49, "701") => "the account has expired",
        (49, "773") => "the account must change its password at next logon",
        (49, "775") => "the account is locked out",
        (49, _) => "invalid credentials",
        (8, _) => {
            "the domain controller requires a protected connection (LDAP signing): use LDAPS or StartTLS"
        }
        (53, _) => "the domain controller refused the bind",
        _ => "the bind was refused",
    };
    format!("{reason} (LDAP result {rc})")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_drapeaux_uac_se_lisent_bit_a_bit() {
        // 66048 = NORMAL_ACCOUNT | DONT_EXPIRE_PASSWORD : le cas le plus courant
        // d'un compte de service.
        let value = parse_uac("66048").unwrap();
        assert_ne!(value & uac::NORMAL_ACCOUNT, 0);
        assert_ne!(value & uac::DONT_EXPIRE_PASSWORD, 0);
        assert_eq!(value & uac::ACCOUNTDISABLE, 0);
        // 514 = compte désactivé.
        assert_ne!(parse_uac("514").unwrap() & uac::ACCOUNTDISABLE, 0);
        // 532480 = contrôleur de domaine : SERVER_TRUST + TRUSTED_FOR_DELEGATION.
        let dc = parse_uac("532480").unwrap();
        assert_ne!(dc & uac::SERVER_TRUST_ACCOUNT, 0);
        assert_ne!(dc & uac::TRUSTED_FOR_DELEGATION, 0);
        // 4194816 = NORMAL_ACCOUNT | DONT_REQ_PREAUTH.
        assert_ne!(parse_uac("4194816").unwrap() & uac::DONT_REQ_PREAUTH, 0);
        assert_eq!(parse_uac("pas un nombre"), None);
        assert_eq!(parse_uac("-1"), Some(u32::MAX));
    }

    #[test]
    fn filetime_vers_unix() {
        // 2026-10-04 00:00:00 UTC = 1 791 072 000 s Unix.
        let ticks = (1_791_072_000i64 + FILETIME_UNIX_OFFSET_S) * TICKS_PER_SECOND;
        assert_eq!(filetime_to_unix(&ticks.to_string()), Some(1_791_072_000));
        assert_eq!(filetime_to_unix("0"), None);
        assert_eq!(filetime_to_unix("9223372036854775807"), None);
        assert_eq!(filetime_to_unix(""), None);
    }

    #[test]
    fn les_durees_de_strategie_sont_negatives() {
        // 42 jours, la valeur par défaut de maxPwdAge.
        assert_eq!(interval_seconds("-36288000000000"), Some(42 * 86_400));
        // 30 minutes, lockoutDuration par défaut.
        assert_eq!(interval_seconds("-18000000000"), Some(1_800));
        assert_eq!(interval_seconds("-9223372036854775808"), None);
        assert_eq!(interval_seconds("0"), None);
    }

    #[test]
    fn generalized_time_vers_unix() {
        assert_eq!(generalized_time_to_unix("20261004000000.0Z"), Some(1_791_072_000));
        assert_eq!(generalized_time_to_unix("19700101000000Z"), Some(0));
        assert_eq!(generalized_time_to_unix("2026"), None);
        assert_eq!(generalized_time_to_unix("2026100400000x.0Z"), None);
    }

    #[test]
    fn date_iso_des_voisins_de_replication() {
        assert_eq!(iso_time_to_unix("2026-10-04T00:00:00Z"), Some(1_791_072_000));
        assert_eq!(iso_time_to_unix("1601-01-01T00:00:00Z"), None);
        assert_eq!(iso_time_to_unix("n'importe quoi"), None);
    }

    #[test]
    fn sid_binaire_vers_texte() {
        // S-1-5-21-1004336348-1177238915-682003330-512 (Domain Admins).
        let mut bytes = vec![1u8, 5, 0, 0, 0, 0, 0, 5];
        for sub in [21u32, 1_004_336_348, 1_177_238_915, 682_003_330, 512] {
            bytes.extend_from_slice(&sub.to_le_bytes());
        }
        let sid = sid_to_string(&bytes).unwrap();
        assert_eq!(sid, "S-1-5-21-1004336348-1177238915-682003330-512");
        assert_eq!(rid_of(&sid), Some(512));
        // S-1-5-32-544, BUILTIN\Administrators.
        let mut builtin = vec![1u8, 2, 0, 0, 0, 0, 0, 5];
        builtin.extend_from_slice(&32u32.to_le_bytes());
        builtin.extend_from_slice(&544u32.to_le_bytes());
        assert_eq!(sid_to_string(&builtin).unwrap(), "S-1-5-32-544");
        assert_eq!(sid_to_string(&[1, 5, 0]), None);
        assert_eq!(sid_to_string(&[1, 2, 0, 0, 0, 0, 0, 5, 1, 0, 0, 0]), None);
    }

    #[test]
    fn decoupage_des_dn() {
        let dn = r"CN=Dupont\, Jean,OU=Paris,DC=corp,DC=example,DC=com";
        let parts = dn_components(dn);
        assert_eq!(parts[0], ("CN".to_string(), "Dupont, Jean".to_string()));
        assert_eq!(first_rdn_value(dn).as_deref(), Some("Dupont, Jean"));
        assert_eq!(dns_name_of(dn), "corp.example.com");
        assert!(same_dn("cn=DC1, dc=CORP,dc=lan", "CN=dc1,DC=corp,DC=lan"));
        assert!(!same_dn("CN=DC1,DC=corp,DC=lan", "CN=DC2,DC=corp,DC=lan"));
    }

    #[test]
    fn serveur_et_site_dun_dsa() {
        let dn =
            "CN=NTDS Settings,CN=DC1,CN=Servers,CN=Paris,CN=Sites,CN=Configuration,DC=corp,DC=lan";
        assert_eq!(server_and_site_of_dsa(dn), Some(("DC1".to_string(), "Paris".to_string())));
        assert_eq!(server_and_site_of_dsa("CN=Users,DC=corp,DC=lan"), None);
    }

    #[test]
    fn niveaux_fonctionnels() {
        assert_eq!(functional_level_label(7), "Windows Server 2016");
        assert_eq!(functional_level_label(10), "Windows Server 2025");
        assert_eq!(functional_level_label(42), "level 42");
    }

    #[test]
    fn les_refus_de_liaison_sont_expliques() {
        let text = "80090308: LdapErr: DSID-0C090447, comment: AcceptSecurityContext error, data 52e, v4563";
        assert_eq!(explain_bind_failure(49, text), "wrong user name or password (LDAP result 49)");
        let text = "80090308: LdapErr: DSID-0C090447, comment: AcceptSecurityContext error, data 775, v4563";
        assert!(explain_bind_failure(49, text).starts_with("the account is locked out"));
        assert!(explain_bind_failure(49, "data 532, v4563").contains("password has expired"));
        assert!(explain_bind_failure(8, "").contains("LDAPS or StartTLS"));
        assert_eq!(explain_bind_failure(49, ""), "invalid credentials (LDAP result 49)");
    }
}
