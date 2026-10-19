//! Les listes noires DNS interrogées, et la lecture de leurs codes de réponse.
//!
//! Module pur : ni résolveur ni socket. Une liste se consulte en demandant
//! l'enregistrement `A` de `<sujet>.<zone>` ; `NXDOMAIN` veut dire « absent »,
//! une adresse de `127.0.0.0/8` « présent », et son dernier octet dit souvent
//! pourquoi. Certaines listes répondent aussi par des codes **d'erreur** qui
//! ressemblent à des inscriptions (Spamhaus `127.255.255.254` quand la question
//! arrive par un résolveur public, URIBL et SURBL `127.0.0.1` quand elles
//! refusent de répondre) : les prendre pour une inscription ferait croire à un
//! serveur listé partout. Ils sont lus ici comme des refus, jamais comme un
//! résultat.
//!
//! Choix des listes (vérifiés le 16 octobre 2026) : celles qui pèsent encore
//! sur la remise du courrier et que l'on peut interroger gratuitement à faible
//! volume. SORBS a fermé le 5 juin 2024 et n'y figure pas. Gmail, Outlook.com et
//! Yahoo ne publient aucune liste DNS : voir [`PROVIDERS`].

use std::net::{Ipv4Addr, Ipv6Addr};

/// Ce que l'on soumet à une liste.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subject {
    /// Une adresse IP (DNSBL).
    Ip,
    /// Un nom de domaine (RHSBL, listes d'URI).
    Domain,
}

/// Le sens d'un code de réponse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Code {
    /// Inscrit, avec le nom de la sous-liste ou le motif.
    Listed(String),
    /// La liste refuse de répondre (résolveur public, volume, clé invalide) :
    /// on ne sait rien du sujet.
    Refused(String),
}

/// Une liste noire DNS.
#[derive(Debug)]
pub struct List {
    /// Identifiant stable, utilisé dans l'option `lists` et l'étiquette `list`.
    pub id: &'static str,
    pub name: &'static str,
    /// Zone publique.
    pub zone: &'static str,
    /// Zone du Data Query Service de Spamhaus, à préfixer de la clé.
    pub dqs_zone: Option<&'static str>,
    pub subject: Subject,
    /// La liste répond pour les adresses IPv6 (format en quartets inversés).
    pub ipv6: bool,
    /// Liste « majeure » : une inscription bloque une part notable du courrier
    /// et déclenche l'alerte critique.
    pub major: bool,
    /// Une phrase (anglais, comme le catalogue du formulaire) : à quoi sert la liste.
    pub description: &'static str,
    /// Page de consultation ; `{ip}` ou `{domain}` y est remplacé.
    pub lookup_url: &'static str,
    /// Page de demande de suppression ; mêmes jokers.
    pub delist_url: &'static str,
    /// Lecture d'un code de `127.0.0.0/8`.
    pub interpret: fn(Ipv4Addr) -> Code,
}

impl List {
    /// La page de consultation pour ce sujet.
    pub fn lookup_for(&self, subject: &str) -> String {
        fill(self.lookup_url, subject)
    }

    /// La page de demande de suppression pour ce sujet.
    pub fn delist_for(&self, subject: &str) -> String {
        fill(self.delist_url, subject)
    }

    /// Le nom à résoudre pour ce sujet : adresse inversée (ou nom) suivie de la
    /// zone, avec un point final pour que la liste de recherche du système ne
    /// s'y ajoute jamais. `None` si la liste ne couvre pas ce sujet (IPv6 sur
    /// une liste IPv4, domaine sur une liste d'adresses).
    pub fn query_name(&self, subject: &Target, dqs_key: Option<&str>) -> Option<String> {
        let zone = match (self.dqs_zone, dqs_key) {
            (Some(dqs), Some(key)) => format!("{key}.{dqs}"),
            _ => self.zone.to_string(),
        };
        let prefix = match (subject, self.subject) {
            (Target::V4(ip), Subject::Ip) => reverse_v4(*ip),
            (Target::V6(ip), Subject::Ip) if self.ipv6 => reverse_v6(*ip),
            (Target::Domain(name), Subject::Domain) => name.clone(),
            _ => return None,
        };
        Some(format!("{prefix}.{zone}."))
    }
}

fn fill(template: &str, subject: &str) -> String {
    template.replace("{ip}", subject).replace("{domain}", subject)
}

/// Un sujet de vérification.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Target {
    V4(Ipv4Addr),
    V6(Ipv6Addr),
    Domain(String),
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::V4(ip) => ip.fmt(f),
            Self::V6(ip) => ip.fmt(f),
            Self::Domain(name) => f.write_str(name),
        }
    }
}

/// `192.0.2.1` → `1.2.0.192`.
pub fn reverse_v4(ip: Ipv4Addr) -> String {
    let [a, b, c, d] = ip.octets();
    format!("{d}.{c}.{b}.{a}")
}

/// Les 32 quartets de l'adresse, du dernier au premier (RFC 5782, section 2.4).
pub fn reverse_v6(ip: Ipv6Addr) -> String {
    let mut nibbles = Vec::with_capacity(32);
    for byte in ip.octets().iter().rev() {
        nibbles.push(format!("{:x}", byte & 0x0f));
        nibbles.push(format!("{:x}", byte >> 4));
    }
    nibbles.join(".")
}

/// Le refus de Spamhaus pour un résolveur public : l'interface le reconnaît à ce
/// texte pour afficher son propre message.
pub const PUBLIC_RESOLVER_REFUSAL: &str = "Spamhaus answered 127.255.255.254: it refuses queries sent through a public or open resolver (Google, Cloudflare, Quad9, a resolver that forwards to them, or one without an attributable reverse DNS)";

/// Codes d'erreur communs aux zones Spamhaus (`127.255.255.0/24`).
fn spamhaus_error(code: Ipv4Addr) -> Option<Code> {
    let [a, b, c, d] = code.octets();
    if (a, b, c) != (127, 255, 255) {
        return None;
    }
    Some(Code::Refused(
        match d {
            252 => "Spamhaus answered 127.255.255.252: the query was malformed (wrong zone name, or an invalid DQS key)",
            254 => PUBLIC_RESOLVER_REFUSAL,
            255 => "Spamhaus answered 127.255.255.255: this resolver sent too many queries",
            _ => "Spamhaus answered an error code from 127.255.255.0/24",
        }
        .to_string(),
    ))
}

fn last_octet(code: Ipv4Addr, prefix: (u8, u8, u8)) -> Option<u8> {
    let [a, b, c, d] = code.octets();
    ((a, b, c) == prefix).then_some(d)
}

fn listed(text: &str) -> Code {
    Code::Listed(text.to_string())
}

fn zen(code: Ipv4Addr) -> Code {
    if let Some(error) = spamhaus_error(code) {
        return error;
    }
    match last_octet(code, (127, 0, 0)) {
        Some(2) => listed("SBL"),
        Some(3) => listed("SBL CSS"),
        Some(4..=7) => listed("XBL"),
        Some(9) => listed("DROP"),
        Some(10 | 11) => listed("PBL"),
        _ => listed("listed"),
    }
}

fn dbl(code: Ipv4Addr) -> Code {
    if let Some(error) = spamhaus_error(code) {
        return error;
    }
    match last_octet(code, (127, 0, 1)) {
        Some(2) => listed("spam domain"),
        Some(4) => listed("phishing domain"),
        Some(5) => listed("malware domain"),
        Some(6) => listed("botnet C&C domain"),
        Some(102) => listed("abused legit domain (spam)"),
        Some(103) => listed("abused legit domain (redirector)"),
        Some(104) => listed("abused legit domain (phishing)"),
        Some(105) => listed("abused legit domain (malware)"),
        Some(106) => listed("abused legit domain (botnet C&C)"),
        Some(255) => Code::Refused(
            "the Spamhaus DBL answered 127.0.1.255: it was asked about an IP address".to_string(),
        ),
        _ => listed("listed"),
    }
}

fn plain(_code: Ipv4Addr) -> Code {
    listed("listed")
}

fn spamrats(code: Ipv4Addr) -> Code {
    match last_octet(code, (127, 0, 0)) {
        Some(36) => listed("RATS-Dyna"),
        Some(37) => listed("RATS-NoPtr"),
        Some(38) => listed("RATS-Spam"),
        Some(43) => listed("RATS-Auth"),
        _ => listed("listed"),
    }
}

/// Les sous-listes d'un code en masque de bits, dans l'ordre des bits.
fn bits(value: u8, names: &[(u8, &str)]) -> Code {
    let found: Vec<&str> =
        names.iter().filter(|(bit, _)| value & bit != 0).map(|(_, name)| *name).collect();
    if found.is_empty() { listed("listed") } else { listed(&found.join(", ")) }
}

fn surbl(code: Ipv4Addr) -> Code {
    match last_octet(code, (127, 0, 0)) {
        Some(1) => Code::Refused(
            "SURBL answered 127.0.0.1: it refuses queries from this resolver (public resolver or too many queries)"
                .to_string(),
        ),
        Some(value) => {
            bits(value, &[(8, "phishing"), (16, "malware"), (64, "abuse"), (128, "cracked")])
        }
        None => listed("listed"),
    }
}

fn uribl(code: Ipv4Addr) -> Code {
    match last_octet(code, (127, 0, 0)) {
        Some(1) => Code::Refused(
            "URIBL answered 127.0.0.1: it refuses queries from this resolver (public resolver or too many queries)"
                .to_string(),
        ),
        Some(value) => bits(value, &[(2, "black"), (4, "grey"), (8, "red")]),
        None => listed("listed"),
    }
}

/// Les listes, dans l'ordre d'affichage : adresses d'abord, majeures en tête.
pub static LISTS: &[List] = &[
    List {
        id: "spamhaus_zen",
        name: "Spamhaus ZEN",
        zone: "zen.spamhaus.org",
        dqs_zone: Some("zen.dq.spamhaus.net"),
        subject: Subject::Ip,
        ipv6: true,
        major: true,
        description: "The reference list for mail servers: spam sources, exploited machines and ranges that should not send mail directly.",
        lookup_url: "https://check.spamhaus.org/query/ip/{ip}",
        delist_url: "https://check.spamhaus.org/query/ip/{ip}",
        interpret: zen,
    },
    List {
        id: "barracuda",
        name: "Barracuda BRBL",
        zone: "b.barracudacentral.org",
        dqs_zone: None,
        subject: Subject::Ip,
        ipv6: false,
        major: true,
        description: "Reputation list of Barracuda appliances, used by many company mail gateways.",
        lookup_url: "https://www.barracudacentral.org/lookups/lookup-reputation",
        delist_url: "https://www.barracudacentral.org/rbl/removal-request",
        interpret: plain,
    },
    List {
        id: "spamcop",
        name: "SpamCop",
        zone: "bl.spamcop.net",
        dqs_zone: None,
        subject: Subject::Ip,
        ipv6: false,
        major: true,
        description: "Addresses reported by SpamCop users; entries expire by themselves after a day or two.",
        lookup_url: "https://www.spamcop.net/bl.shtml?{ip}",
        delist_url: "https://www.spamcop.net/bl.shtml?{ip}",
        interpret: plain,
    },
    List {
        id: "mailspike",
        name: "Mailspike",
        zone: "bl.mailspike.net",
        dqs_zone: None,
        subject: Subject::Ip,
        ipv6: false,
        major: false,
        description: "Reputation by sending behaviour; a listing there alone rarely blocks mail.",
        lookup_url: "https://mailspike.io/ip_verify",
        delist_url: "https://mailspike.io/ip_verify",
        interpret: plain,
    },
    List {
        id: "uceprotect_1",
        name: "UCEPROTECT Level 1",
        zone: "dnsbl-1.uceprotect.net",
        dqs_zone: None,
        subject: Subject::Ip,
        ipv6: false,
        major: false,
        description: "Single addresses seen sending spam; strict, and often contested.",
        lookup_url: "https://www.uceprotect.net/en/rblcheck.php?ipr={ip}",
        delist_url: "https://www.uceprotect.net/en/rblcheck.php?ipr={ip}",
        interpret: plain,
    },
    List {
        id: "psbl",
        name: "PSBL",
        zone: "psbl.surriel.com",
        dqs_zone: None,
        subject: Subject::Ip,
        ipv6: false,
        major: false,
        description: "Passive Spam Block List: addresses seen sending to spam traps.",
        lookup_url: "https://psbl.org/listing?ip={ip}",
        delist_url: "https://psbl.org/listing?ip={ip}",
        interpret: plain,
    },
    List {
        id: "spamrats",
        name: "SpamRATS",
        zone: "all.spamrats.com",
        dqs_zone: None,
        subject: Subject::Ip,
        ipv6: false,
        major: false,
        description: "Dynamic ranges, addresses without reverse DNS and spam sources.",
        lookup_url: "https://www.spamrats.com/bl?{ip}",
        delist_url: "https://www.spamrats.com/removal.php",
        interpret: spamrats,
    },
    List {
        id: "spamhaus_dbl",
        name: "Spamhaus DBL",
        zone: "dbl.spamhaus.org",
        dqs_zone: Some("dbl.dq.spamhaus.net"),
        subject: Subject::Domain,
        ipv6: false,
        major: true,
        description: "Domains used in spam, phishing and malware.",
        lookup_url: "https://check.spamhaus.org/query/domain/{domain}",
        delist_url: "https://check.spamhaus.org/query/domain/{domain}",
        interpret: dbl,
    },
    List {
        id: "surbl",
        name: "SURBL",
        zone: "multi.surbl.org",
        dqs_zone: None,
        subject: Subject::Domain,
        ipv6: false,
        major: false,
        description: "Domains seen in spam links, phishing and malware.",
        lookup_url: "https://www.surbl.org/surbl-analysis",
        delist_url: "https://www.surbl.org/surbl-analysis",
        interpret: surbl,
    },
    List {
        id: "uribl",
        name: "URIBL",
        zone: "multi.uribl.com",
        dqs_zone: None,
        subject: Subject::Domain,
        ipv6: false,
        major: false,
        description: "Domains advertised in spam messages.",
        lookup_url: "https://lookup.uribl.com/?domain={domain}",
        delist_url: "https://admin.uribl.com/",
        interpret: uribl,
    },
];

/// La liste qui porte cet identifiant.
pub fn by_id(id: &str) -> Option<&'static List> {
    LISTS.iter().find(|list| list.id == id)
}

/// Les identifiants connus, séparés par des virgules (messages d'erreur).
pub fn ids() -> String {
    LISTS.iter().map(|list| list.id).collect::<Vec<_>>().join(", ")
}

/// Préfixe de l'identifiant d'une liste ajoutée par l'utilisateur.
pub const CUSTOM_PREFIX: &str = "custom:";
/// Zones personnalisées au plus, pour borner le nombre de questions.
pub const MAX_CUSTOM: usize = 20;

/// Une liste ajoutée par l'utilisateur, telle qu'elle se saisit.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct CustomSpec {
    pub zone: String,
    /// `ip` (défaut) ou `domain`.
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub major: bool,
}

/// Une zone DNSBL valide : nom de domaine en minuscules, sans point final,
/// au moins deux étiquettes, jamais une adresse IP. Rend la forme normalisée.
pub fn normalize_zone(raw: &str) -> Result<String, String> {
    let zone = raw.trim().trim_end_matches('.').to_ascii_lowercase();
    if zone.is_empty() {
        return Err("a custom list needs a zone name".to_string());
    }
    if zone.parse::<std::net::IpAddr>().is_ok() {
        return Err(format!("\"{raw}\" is an IP address: enter the DNS zone of the list"));
    }
    // Le sujet (adresse inversée, jusqu'à 63 caractères pour un nom) se place devant.
    if zone.len() > 180 {
        return Err(format!("\"{raw}\" is too long for a DNS zone"));
    }
    let labels: Vec<&str> = zone.split('.').collect();
    let valid_label = |l: &&str| {
        (1..=63).contains(&l.len())
            && l.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            && !l.starts_with('-')
            && !l.ends_with('-')
    };
    if labels.len() < 2 || !labels.iter().all(valid_label) {
        return Err(format!(
            "\"{raw}\" is not a valid DNS zone name (for example dnsbl.example.org)"
        ));
    }
    // Un dernier élément sans lettre (`1.2.3.4`, `10.0`) n'est pas un domaine.
    if !labels.last().is_some_and(|tld| tld.bytes().any(|b| b.is_ascii_alphabetic())) {
        return Err(format!("\"{raw}\" is not a valid DNS zone name"));
    }
    Ok(zone)
}

/// Les listes personnalisées d'une saisie JSON (`[{"zone": "...", "subject":
/// "ip", "major": false}]`). Vide si la saisie est vide.
pub fn parse_custom(raw: &str) -> Result<Vec<&'static List>, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(Vec::new());
    }
    let specs: Vec<CustomSpec> = serde_json::from_str(raw).map_err(|e| {
        format!("\"custom_lists\" expects a JSON list of {{zone, subject, major}}: {e}")
    })?;
    if specs.len() > MAX_CUSTOM {
        return Err(format!("at most {MAX_CUSTOM} custom lists, got {}", specs.len()));
    }
    let mut chosen: Vec<&'static List> = Vec::new();
    for spec in &specs {
        let zone = normalize_zone(&spec.zone)?;
        let subject = match spec.subject.trim() {
            "" | "ip" => Subject::Ip,
            "domain" => Subject::Domain,
            other => {
                return Err(format!(
                    "the type of custom list {zone} must be \"ip\" or \"domain\", got \"{other}\""
                ));
            }
        };
        if LISTS.iter().any(|l| l.zone == zone || l.dqs_zone == Some(zone.as_str()))
            || chosen.iter().any(|l| l.zone == zone)
        {
            return Err(format!("the list {zone} is already there; remove the duplicate"));
        }
        chosen.push(custom(&zone, subject, spec.major));
    }
    Ok(chosen)
}

/// Les listes personnalisées déjà créées : une même saisie rend toujours le
/// même objet, si bien que la mémoire ne croît qu'avec le nombre de
/// définitions distinctes (le catalogue est `'static`, une liste aussi).
static CUSTOM: std::sync::Mutex<Vec<&'static List>> = std::sync::Mutex::new(Vec::new());

fn custom(zone: &str, subject: Subject, major: bool) -> &'static List {
    let mut known = CUSTOM.lock().unwrap_or_else(|p| p.into_inner());
    if let Some(list) =
        known.iter().find(|l| l.zone == zone && l.subject == subject && l.major == major)
    {
        return list;
    }
    let zone: &'static str = Box::leak(zone.to_string().into_boxed_str());
    let list: &'static List = Box::leak(Box::new(List {
        id: Box::leak(format!("{CUSTOM_PREFIX}{zone}").into_boxed_str()),
        name: zone,
        zone,
        dqs_zone: None,
        subject,
        ipv6: false,
        major,
        description: "Custom list",
        lookup_url: "",
        delist_url: "",
        interpret: plain,
    }));
    known.push(list);
    list
}

/// Une grande messagerie sans liste DNS publique : ce qu'elle propose aux
/// expéditeurs à la place. DumbMonit ne l'interroge pas ; il vérifie ce qui pèse
/// sur sa décision (DNS inverse, SPF, DMARC, listes ci-dessus) et donne ces liens.
pub struct Provider {
    pub id: &'static str,
    pub name: &'static str,
    /// `(nature du lien, adresse)` ; la nature est traduite par l'interface.
    pub links: &'static [(&'static str, &'static str)],
}

/// Pages vérifiées le 16 octobre 2026.
pub static PROVIDERS: &[Provider] = &[
    Provider {
        id: "gmail",
        name: "Gmail",
        links: &[
            ("postmaster", "https://postmaster.google.com/"),
            ("guidelines", "https://support.google.com/a/answer/81126"),
            ("contact", "https://support.google.com/mail/contact/gmail_bulk_sender_escalation"),
        ],
    },
    Provider {
        id: "outlook",
        name: "Outlook.com / Hotmail",
        links: &[
            ("snds", "https://sendersupport.olc.protection.outlook.com/snds/"),
            ("guidelines", "https://sendersupport.olc.protection.outlook.com/pm/"),
            ("contact", "https://olcsupport.office.com/"),
            ("delist_m365", "https://sender.office.com/"),
        ],
    },
    Provider {
        id: "yahoo",
        name: "Yahoo / AOL",
        links: &[
            ("hub", "https://senders.yahooinc.com/"),
            ("guidelines", "https://senders.yahooinc.com/best-practices/"),
            ("contact", "https://senders.yahooinc.com/contact/"),
        ],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(raw: &str) -> Ipv4Addr {
        raw.parse().unwrap()
    }

    #[test]
    fn les_noms_interroges_sont_inverses_et_complets() {
        let zen = by_id("spamhaus_zen").unwrap();
        let v4 = Target::V4(ip("192.0.2.10"));
        assert_eq!(zen.query_name(&v4, None).unwrap(), "10.2.0.192.zen.spamhaus.org.");
        assert_eq!(
            zen.query_name(&v4, Some("abcdefghij0123456789klmnop")).unwrap(),
            "10.2.0.192.abcdefghij0123456789klmnop.zen.dq.spamhaus.net."
        );
        let v6 = Target::V6("2001:db8::1".parse().unwrap());
        assert_eq!(
            zen.query_name(&v6, None).unwrap(),
            "1.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.0.8.b.d.0.1.0.0.2.zen.spamhaus.org."
        );
        // Barracuda ne répond pas en IPv6, et une liste d'adresses ignore les noms.
        assert_eq!(by_id("barracuda").unwrap().query_name(&v6, None), None);
        let domain = Target::Domain("example.com".into());
        assert_eq!(zen.query_name(&domain, None), None);
        assert_eq!(
            by_id("spamhaus_dbl").unwrap().query_name(&domain, None).unwrap(),
            "example.com.dbl.spamhaus.org."
        );
    }

    #[test]
    fn les_codes_d_erreur_ne_sont_jamais_des_inscriptions() {
        let zen = by_id("spamhaus_zen").unwrap();
        for code in ["127.255.255.252", "127.255.255.254", "127.255.255.255"] {
            assert!(matches!((zen.interpret)(ip(code)), Code::Refused(_)), "{code}");
        }
        let dbl = by_id("spamhaus_dbl").unwrap();
        assert!(matches!((dbl.interpret)(ip("127.255.255.254")), Code::Refused(_)));
        assert!(matches!((dbl.interpret)(ip("127.0.1.255")), Code::Refused(_)));
        assert!(matches!((by_id("uribl").unwrap().interpret)(ip("127.0.0.1")), Code::Refused(_)));
        assert!(matches!((by_id("surbl").unwrap().interpret)(ip("127.0.0.1")), Code::Refused(_)));
    }

    #[test]
    fn les_codes_disent_la_sous_liste() {
        let zen = by_id("spamhaus_zen").unwrap();
        assert_eq!((zen.interpret)(ip("127.0.0.2")), listed("SBL"));
        assert_eq!((zen.interpret)(ip("127.0.0.4")), listed("XBL"));
        assert_eq!((zen.interpret)(ip("127.0.0.11")), listed("PBL"));
        assert_eq!(
            (by_id("spamhaus_dbl").unwrap().interpret)(ip("127.0.1.4")),
            listed("phishing domain")
        );
        assert_eq!((by_id("spamrats").unwrap().interpret)(ip("127.0.0.37")), listed("RATS-NoPtr"));
        assert_eq!((by_id("uribl").unwrap().interpret)(ip("127.0.0.6")), listed("black, grey"));
        assert_eq!((by_id("surbl").unwrap().interpret)(ip("127.0.0.80")), listed("malware, abuse"));
    }

    #[test]
    fn chaque_liste_a_ses_liens_et_un_identifiant_unique() {
        let mut seen = std::collections::HashSet::new();
        for list in LISTS {
            assert!(seen.insert(list.id), "{} en double", list.id);
            assert!(list.lookup_url.starts_with("https://"), "{}", list.id);
            assert!(list.delist_url.starts_with("https://"), "{}", list.id);
        }
        assert_eq!(
            by_id("spamcop").unwrap().delist_for("192.0.2.1"),
            "https://www.spamcop.net/bl.shtml?192.0.2.1"
        );
        assert!(by_id("sorbs").is_none(), "SORBS a fermé en juin 2024");
        for provider in PROVIDERS {
            assert!(provider.links.iter().all(|(_, url)| url.starts_with("https://")));
        }
    }

    #[test]
    fn zones_personnalisees_valides_et_invalides() {
        assert_eq!(normalize_zone(" DNSBL.Example.ORG. ").unwrap(), "dnsbl.example.org");
        assert_eq!(normalize_zone("a-b.c_d.example.co.uk").unwrap(), "a-b.c_d.example.co.uk");
        for bad in [
            "",
            "example",
            "192.0.2.1",
            "2001:db8::1",
            "1.2.3.4",
            "a..b.org",
            "-a.example.org",
            "a-.example.org",
            "a b.example.org",
            "a/b.example.org",
        ] {
            assert!(normalize_zone(bad).is_err(), "{bad}");
        }
        assert!(normalize_zone(&format!("{}.example.org", "a".repeat(64))).is_err());
    }

    #[test]
    fn une_liste_personnalisee_se_construit_une_seule_fois() {
        let one = parse_custom(r#"[{"zone":"x.example.org","subject":"ip"}]"#).unwrap();
        let two = parse_custom(r#"[{"zone":"X.example.org.","major":false}]"#).unwrap();
        assert!(std::ptr::eq(one[0], two[0]), "la même saisie rend le même objet");
        assert_eq!(one[0].id, "custom:x.example.org");
        assert_eq!(
            one[0].query_name(&Target::V4("192.0.2.1".parse().unwrap()), None).as_deref(),
            Some("1.2.0.192.x.example.org.")
        );
        assert_eq!(one[0].query_name(&Target::Domain("a.test".into()), None), None);
        assert_eq!(parse_custom("  ").unwrap().len(), 0);
    }
}
