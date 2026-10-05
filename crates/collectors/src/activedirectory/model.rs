//! Ce que l'annuaire a répondu, en structures simples.
//!
//! `client.rs` remplit ces structures à partir des entrées LDAP ; l'analyse
//! (`analysis.rs`) ne voit qu'elles. C'est ce qui rend toute la logique de
//! décision testable sans contrôleur de domaine : un test construit un
//! `Account` à la main, comme l'annuaire l'aurait rendu.

use std::collections::HashMap;

use super::decode::{self, uac};

/// Valeurs d'une entrée, par nom d'attribut (en minuscules).
///
/// LDAP ne garantit pas la casse des noms d'attributs rendus : AD rend
/// `sAMAccountName` tel que demandé, d'autres annuaires non. Tout est rangé en
/// minuscules pour que la lecture ne dépende pas de l'humeur du serveur.
#[derive(Debug, Clone, Default)]
pub struct Entry {
    pub dn: String,
    pub attrs: HashMap<String, Vec<String>>,
    pub bin_attrs: HashMap<String, Vec<Vec<u8>>>,
}

impl Entry {
    pub fn new(
        dn: String,
        attrs: HashMap<String, Vec<String>>,
        bin_attrs: HashMap<String, Vec<Vec<u8>>>,
    ) -> Self {
        Self {
            dn,
            attrs: attrs.into_iter().map(|(k, v)| (k.to_ascii_lowercase(), v)).collect(),
            bin_attrs: bin_attrs.into_iter().map(|(k, v)| (k.to_ascii_lowercase(), v)).collect(),
        }
    }

    /// Première valeur texte d'un attribut.
    pub fn first(&self, name: &str) -> Option<&str> {
        self.attrs
            .get(&name.to_ascii_lowercase())
            .and_then(|values| values.first())
            .map(String::as_str)
    }

    pub fn all(&self, name: &str) -> &[String] {
        self.attrs.get(&name.to_ascii_lowercase()).map(Vec::as_slice).unwrap_or_default()
    }

    pub fn has(&self, name: &str) -> bool {
        let key = name.to_ascii_lowercase();
        self.attrs.get(&key).is_some_and(|v| !v.is_empty())
            || self.bin_attrs.get(&key).is_some_and(|v| !v.is_empty())
    }

    pub fn int(&self, name: &str) -> Option<i64> {
        self.first(name)?.trim().parse().ok()
    }

    pub fn bool(&self, name: &str) -> Option<bool> {
        match self.first(name)?.trim().to_ascii_uppercase().as_str() {
            "TRUE" => Some(true),
            "FALSE" => Some(false),
            _ => None,
        }
    }

    /// `objectSid` est binaire ; certains annuaires le rendent déjà en texte.
    pub fn sid(&self) -> Option<String> {
        if let Some(bytes) = self.bin_attrs.get("objectsid").and_then(|v| v.first()) {
            return decode::sid_to_string(bytes);
        }
        self.first("objectSid").filter(|s| s.starts_with("S-")).map(str::to_string)
    }
}

// --------------------------------------------------------------------------
// RootDSE et objet domaine
// --------------------------------------------------------------------------

/// Le RootDSE du contrôleur interrogé : lisible avant toute autre requête, il
/// dit où sont les partitions et à quel niveau fonctionne la forêt.
#[derive(Debug, Clone, Default)]
pub struct RootDse {
    pub default_nc: String,
    pub config_nc: String,
    pub schema_nc: String,
    pub root_nc: String,
    pub dns_host_name: Option<String>,
    /// DN de l'objet « NTDS Settings » du contrôleur interrogé.
    pub ds_service_name: Option<String>,
    pub domain_functionality: Option<i64>,
    pub forest_functionality: Option<i64>,
    pub dc_functionality: Option<i64>,
    /// Faux tant que le contrôleur n'a pas fini sa première réplication.
    pub is_synchronized: Option<bool>,
    pub is_gc_ready: Option<bool>,
    pub current_time: Option<i64>,
    /// `msDS-ReplAllInboundNeighbors` : un document XML par voisin et par
    /// partition. Vide si le compte n'a pas le droit de le lire.
    pub inbound_neighbors: Vec<Neighbor>,
}

impl RootDse {
    pub fn from_entry(entry: &Entry) -> Self {
        let text = |name: &str| entry.first(name).map(str::to_string);
        Self {
            default_nc: text("defaultNamingContext").unwrap_or_default(),
            config_nc: text("configurationNamingContext").unwrap_or_default(),
            schema_nc: text("schemaNamingContext").unwrap_or_default(),
            root_nc: text("rootDomainNamingContext").unwrap_or_default(),
            dns_host_name: text("dnsHostName"),
            ds_service_name: text("dsServiceName"),
            domain_functionality: entry.int("domainFunctionality"),
            forest_functionality: entry.int("forestFunctionality"),
            dc_functionality: entry.int("domainControllerFunctionality"),
            is_synchronized: entry.bool("isSynchronized"),
            is_gc_ready: entry.bool("isGlobalCatalogReady"),
            current_time: entry.first("currentTime").and_then(decode::generalized_time_to_unix),
            inbound_neighbors: entry
                .all("msDS-ReplAllInboundNeighbors")
                .iter()
                .filter_map(|xml| Neighbor::parse(xml))
                .collect(),
        }
    }
}

/// L'objet domaine : la stratégie de mot de passe par défaut, le quota de
/// machines et le SID du domaine.
#[derive(Debug, Clone, Default)]
pub struct DomainObject {
    pub sid: Option<String>,
    pub min_pwd_length: Option<i64>,
    pub pwd_history_length: Option<i64>,
    /// `None` : les mots de passe n'expirent jamais.
    pub max_pwd_age_s: Option<i64>,
    pub min_pwd_age_s: Option<i64>,
    /// `0` : pas de verrouillage.
    pub lockout_threshold: Option<i64>,
    /// `None` : verrouillage jusqu'à déblocage manuel.
    pub lockout_duration_s: Option<i64>,
    pub lockout_window_s: Option<i64>,
    pub pwd_properties: Option<i64>,
    pub machine_account_quota: Option<i64>,
    /// Détenteur du rôle d'émulateur PDC (DN de son « NTDS Settings »).
    pub pdc_owner: Option<String>,
}

/// `pwdProperties` : complexité exigée.
pub const DOMAIN_PASSWORD_COMPLEX: i64 = 0x1;

impl DomainObject {
    pub fn from_entry(entry: &Entry) -> Self {
        let interval = |name: &str| entry.first(name).and_then(decode::interval_seconds);
        Self {
            sid: entry.sid(),
            min_pwd_length: entry.int("minPwdLength"),
            pwd_history_length: entry.int("pwdHistoryLength"),
            max_pwd_age_s: interval("maxPwdAge"),
            min_pwd_age_s: interval("minPwdAge"),
            lockout_threshold: entry.int("lockoutThreshold"),
            lockout_duration_s: interval("lockoutDuration"),
            lockout_window_s: interval("lockOutObservationWindow"),
            pwd_properties: entry.int("pwdProperties"),
            machine_account_quota: entry.int("ms-DS-MachineAccountQuota"),
            pdc_owner: entry.first("fSMORoleOwner").map(str::to_string),
        }
    }

    pub fn complexity_required(&self) -> Option<bool> {
        self.pwd_properties.map(|p| p & DOMAIN_PASSWORD_COMPLEX != 0)
    }
}

/// Un rôle FSMO et le DN du « NTDS Settings » de son détenteur.
#[derive(Debug, Clone, PartialEq)]
pub struct FsmoRecord {
    pub role: &'static str,
    pub holder_dsa: Option<String>,
}

// --------------------------------------------------------------------------
// Contrôleurs de domaine (partition de configuration)
// --------------------------------------------------------------------------

/// Un contrôleur de domaine tel que la partition de configuration le décrit :
/// un objet `server` sous un site, avec son enfant `nTDSDSA`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DsaRecord {
    /// DN de l'objet `nTDSDSA` (« NTDS Settings »).
    pub dsa_dn: String,
    pub server: String,
    pub site: String,
    pub dns_host_name: Option<String>,
    pub global_catalog: bool,
    pub read_only: bool,
    /// Connexions de réplication entrantes (`nTDSConnection`) sous ce DSA.
    pub inbound_connections: usize,
}

/// `options` d'un `nTDSDSA` : catalogue global.
pub const NTDSDSA_OPT_IS_GC: i64 = 0x1;

/// Ce que l'objet ordinateur d'un contrôleur ajoute : son système.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DcComputer {
    pub dns_host_name: Option<String>,
    pub name: String,
    pub operating_system: Option<String>,
    pub os_version: Option<String>,
}

// --------------------------------------------------------------------------
// Réplication
// --------------------------------------------------------------------------

/// Un voisin de réplication entrant, tel que `msDS-ReplAllInboundNeighbors`
/// le décrit (un document `DS_REPL_NEIGHBOR` par voisin et par partition).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Neighbor {
    pub naming_context: String,
    pub source_dsa_dn: String,
    pub last_success: Option<i64>,
    pub last_attempt: Option<i64>,
    /// Code Win32 de la dernière tentative ; `0` = succès.
    pub last_result: u32,
    pub consecutive_failures: u32,
}

impl Neighbor {
    /// Lit un document `<DS_REPL_NEIGHBOR>`. AD termine parfois la valeur par
    /// un caractère nul : il est écarté.
    pub fn parse(xml: &str) -> Option<Self> {
        let xml = xml.trim_matches(|c: char| c == '\0' || c.is_whitespace());
        let document = roxmltree::Document::parse(xml).ok()?;
        let root = document.root_element();
        if root.tag_name().name() != "DS_REPL_NEIGHBOR" {
            return None;
        }
        let field = |name: &str| {
            root.children()
                .find(|node| node.is_element() && node.tag_name().name() == name)
                .and_then(|node| node.text())
                .map(str::trim)
                .unwrap_or_default()
                .to_string()
        };
        Some(Self {
            naming_context: field("pszNamingContext"),
            source_dsa_dn: field("pszSourceDsaDN"),
            last_success: decode::iso_time_to_unix(&field("ftimeLastSyncSuccess")),
            last_attempt: decode::iso_time_to_unix(&field("ftimeLastSyncAttempt")),
            last_result: field("dwLastSyncResult").parse().unwrap_or(0),
            consecutive_failures: field("cNumConsecutiveSyncFailures").parse().unwrap_or(0),
        })
    }
}

// --------------------------------------------------------------------------
// Groupes privilégiés
// --------------------------------------------------------------------------

/// Les groupes privilégiés suivis, désignés par leur RID et non par leur nom :
/// le nom est traduit dans la langue de l'installation (« Admins du domaine »),
/// le RID ne change jamais.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrivilegedGroupDef {
    /// Nom anglais canonique, utilisé comme étiquette de série.
    pub name: &'static str,
    pub rid: u32,
    /// Groupe du conteneur Builtin (`S-1-5-32-…`), présent dans tout domaine.
    pub builtin: bool,
    /// Groupe qui n'existe que dans le domaine racine de la forêt.
    pub forest_root_only: bool,
}

pub const PRIVILEGED_GROUPS: &[PrivilegedGroupDef] = &[
    PrivilegedGroupDef { name: "Domain Admins", rid: 512, builtin: false, forest_root_only: false },
    PrivilegedGroupDef {
        name: "Enterprise Admins",
        rid: 519,
        builtin: false,
        forest_root_only: true,
    },
    PrivilegedGroupDef { name: "Schema Admins", rid: 518, builtin: false, forest_root_only: true },
    PrivilegedGroupDef { name: "Administrators", rid: 544, builtin: true, forest_root_only: false },
    PrivilegedGroupDef {
        name: "Account Operators",
        rid: 548,
        builtin: true,
        forest_root_only: false,
    },
    PrivilegedGroupDef {
        name: "Server Operators",
        rid: 549,
        builtin: true,
        forest_root_only: false,
    },
    PrivilegedGroupDef {
        name: "Print Operators",
        rid: 550,
        builtin: true,
        forest_root_only: false,
    },
    PrivilegedGroupDef {
        name: "Backup Operators",
        rid: 551,
        builtin: true,
        forest_root_only: false,
    },
];

impl PrivilegedGroupDef {
    /// SID du groupe dans un domaine donné.
    pub fn sid(&self, domain_sid: &str) -> String {
        if self.builtin {
            format!("S-1-5-32-{}", self.rid)
        } else {
            format!("{domain_sid}-{}", self.rid)
        }
    }
}

/// Un membre effectif (imbrication comprise) d'un groupe privilégié.
#[derive(Debug, Clone, PartialEq)]
pub struct Member {
    pub dn: String,
    pub name: String,
    pub kind: MemberKind,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemberKind {
    User,
    Computer,
    Group,
    Other,
}

impl Member {
    pub fn from_entry(entry: &Entry) -> Self {
        let classes: Vec<String> =
            entry.all("objectClass").iter().map(|c| c.to_ascii_lowercase()).collect();
        let kind = if classes.iter().any(|c| c == "computer") {
            MemberKind::Computer
        } else if classes.iter().any(|c| c == "group") {
            MemberKind::Group
        } else if classes.iter().any(|c| c == "user") {
            MemberKind::User
        } else {
            MemberKind::Other
        };
        let uac = entry.first("userAccountControl").and_then(decode::parse_uac).unwrap_or(0);
        let name = entry
            .first("sAMAccountName")
            .map(str::to_string)
            .or_else(|| decode::first_rdn_value(&entry.dn))
            .unwrap_or_else(|| entry.dn.clone());
        Self {
            dn: entry.dn.clone(),
            name,
            kind,
            enabled: matches!(kind, MemberKind::Group | MemberKind::Other)
                || uac & uac::ACCOUNTDISABLE == 0,
        }
    }
}

/// Un groupe privilégié trouvé dans le domaine, et tous ses membres effectifs.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupRecord {
    pub def: PrivilegedGroupDef,
    pub dn: String,
    pub members: Vec<Member>,
}

// --------------------------------------------------------------------------
// Comptes (utilisateurs et ordinateurs)
// --------------------------------------------------------------------------

/// Un compte utilisateur ou ordinateur, réduit à ce que l'analyse lit.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Account {
    pub dn: String,
    pub name: String,
    pub sid: Option<String>,
    pub uac: u32,
    /// `msDS-User-Account-Control-Computed` quand l'annuaire le rend : seul
    /// porteur fiable du bit de verrouillage.
    pub computed_uac: Option<u32>,
    /// `lastLogonTimestamp`, répliqué avec jusqu'à 14 jours de retard.
    pub last_logon: Option<i64>,
    pub pwd_last_set: Option<i64>,
    pub created: Option<i64>,
    pub lockout_time: Option<i64>,
    pub admin_count: bool,
    pub spn_count: usize,
    pub operating_system: Option<String>,
    pub dns_host_name: Option<String>,
    /// Une date d'expiration LAPS (historique ou Windows LAPS) est posée.
    pub laps: bool,
}

impl Account {
    pub fn from_entry(entry: &Entry) -> Self {
        let filetime = |name: &str| entry.first(name).and_then(decode::filetime_to_unix);
        Self {
            dn: entry.dn.clone(),
            name: entry
                .first("sAMAccountName")
                .map(str::to_string)
                .or_else(|| decode::first_rdn_value(&entry.dn))
                .unwrap_or_default(),
            sid: entry.sid(),
            uac: entry.first("userAccountControl").and_then(decode::parse_uac).unwrap_or(0),
            computed_uac: entry
                .first("msDS-User-Account-Control-Computed")
                .and_then(decode::parse_uac),
            last_logon: filetime("lastLogonTimestamp"),
            pwd_last_set: filetime("pwdLastSet"),
            created: entry.first("whenCreated").and_then(decode::generalized_time_to_unix),
            lockout_time: filetime("lockoutTime"),
            admin_count: entry.int("adminCount").is_some_and(|v| v > 0),
            spn_count: entry.all("servicePrincipalName").len(),
            operating_system: entry.first("operatingSystem").map(str::to_string),
            dns_host_name: entry.first("dNSHostName").map(str::to_string),
            laps: entry.has("ms-Mcs-AdmPwdExpirationTime")
                || entry.has("msLAPS-PasswordExpirationTime"),
        }
    }

    pub fn has(&self, flag: u32) -> bool {
        self.uac & flag != 0
    }

    pub fn enabled(&self) -> bool {
        !self.has(uac::ACCOUNTDISABLE)
    }

    /// Contrôleur de domaine, inscriptible ou en lecture seule.
    pub fn is_domain_controller(&self) -> bool {
        self.has(uac::SERVER_TRUST_ACCOUNT) || self.has(uac::PARTIAL_SECRETS_ACCOUNT)
    }

    pub fn rid(&self) -> Option<u32> {
        self.sid.as_deref().and_then(decode::rid_of)
    }

    /// Verrouillé en ce moment. Le bit `LOCKOUT` de `userAccountControl` n'est
    /// plus tenu à jour depuis Windows 2000 : on lit d'abord la valeur calculée,
    /// à défaut `lockoutTime` rapporté à la durée de verrouillage du domaine.
    pub fn locked_out(&self, now: i64, lockout_duration_s: Option<i64>) -> bool {
        if let Some(computed) = self.computed_uac {
            return computed & uac::LOCKOUT != 0;
        }
        match self.lockout_time {
            None => false,
            // Sans durée : verrouillé jusqu'à ce qu'un administrateur débloque.
            Some(at) => lockout_duration_s.is_none_or(|duration| at + duration > now),
        }
    }

    /// Inactif depuis plus de `days` jours : dernière connexion trop ancienne,
    /// ou jamais connecté et créé il y a plus longtemps que ça.
    pub fn stale(&self, now: i64, days: u32) -> bool {
        let limit = now - i64::from(days) * 86_400;
        match self.last_logon {
            Some(at) => at < limit,
            None => self.created.is_some_and(|at| at < limit),
        }
    }
}

/// L'inventaire complet, relu en tâche de fond à un rythme plus lent que la
/// santé (`inventory_minutes`) : énumérer tout l'annuaire chaque minute serait
/// une charge inutile pour les contrôleurs.
#[derive(Debug, Clone, Default)]
pub struct Inventory {
    pub refreshed_at: i64,
    pub users: Vec<Account>,
    pub computers: Vec<Account>,
    pub groups_total: u64,
    /// Le schéma connaît LAPS historique (`ms-Mcs-AdmPwd`).
    pub laps_legacy_schema: bool,
    /// Le schéma connaît Windows LAPS (`msLAPS-Password`).
    pub laps_windows_schema: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(dn: &str, attrs: &[(&str, &[&str])]) -> Entry {
        Entry::new(
            dn.to_string(),
            attrs
                .iter()
                .map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect()))
                .collect(),
            HashMap::new(),
        )
    }

    #[test]
    fn le_rootdse_se_lit_quelle_que_soit_la_casse() {
        let e = entry(
            "",
            &[
                ("defaultnamingcontext", &["DC=corp,DC=lan"]),
                ("configurationNamingContext", &["CN=Configuration,DC=corp,DC=lan"]),
                ("domainFunctionality", &["7"]),
                ("isSynchronized", &["TRUE"]),
                ("currentTime", &["20261004000000.0Z"]),
            ],
        );
        let root = RootDse::from_entry(&e);
        assert_eq!(root.default_nc, "DC=corp,DC=lan");
        assert_eq!(root.domain_functionality, Some(7));
        assert_eq!(root.is_synchronized, Some(true));
        assert_eq!(root.current_time, Some(1_791_072_000));
        assert!(root.inbound_neighbors.is_empty());
    }

    #[test]
    fn la_strategie_du_domaine() {
        let e = entry(
            "DC=corp,DC=lan",
            &[
                ("minPwdLength", &["7"]),
                ("maxPwdAge", &["-36288000000000"]),
                ("lockoutThreshold", &["0"]),
                ("lockoutDuration", &["-18000000000"]),
                ("pwdProperties", &["0"]),
                ("ms-DS-MachineAccountQuota", &["10"]),
            ],
        );
        let domain = DomainObject::from_entry(&e);
        assert_eq!(domain.min_pwd_length, Some(7));
        assert_eq!(domain.max_pwd_age_s, Some(42 * 86_400));
        assert_eq!(domain.lockout_duration_s, Some(1_800));
        assert_eq!(domain.complexity_required(), Some(false));
        assert_eq!(domain.machine_account_quota, Some(10));
    }

    #[test]
    fn un_voisin_de_replication() {
        let xml = "<DS_REPL_NEIGHBOR>\n\t<pszNamingContext>DC=corp,DC=lan</pszNamingContext>\n\t<pszSourceDsaDN>CN=NTDS Settings,CN=DC2,CN=Servers,CN=Default-First-Site-Name,CN=Sites,CN=Configuration,DC=corp,DC=lan</pszSourceDsaDN>\n\t<pszSourceDsaAddress>0b5e.._msdcs.corp.lan</pszSourceDsaAddress>\n\t<dwReplicaFlags>1879113840</dwReplicaFlags>\n\t<ftimeLastSyncSuccess>2026-10-03T23:50:00Z</ftimeLastSyncSuccess>\n\t<ftimeLastSyncAttempt>2026-10-04T00:00:00Z</ftimeLastSyncAttempt>\n\t<dwLastSyncResult>1722</dwLastSyncResult>\n\t<cNumConsecutiveSyncFailures>3</cNumConsecutiveSyncFailures>\n</DS_REPL_NEIGHBOR>\n\0";
        let n = Neighbor::parse(xml).unwrap();
        assert_eq!(n.naming_context, "DC=corp,DC=lan");
        assert!(n.source_dsa_dn.contains("CN=DC2"));
        assert_eq!(n.last_attempt, Some(1_791_072_000));
        assert_eq!(n.last_success, Some(1_791_072_000 - 600));
        assert_eq!(n.last_result, 1722);
        assert_eq!(n.consecutive_failures, 3);
        assert_eq!(Neighbor::parse("<autre/>"), None);
        assert_eq!(Neighbor::parse("pas du xml"), None);
    }

    #[test]
    fn un_membre_privilegie() {
        let e = entry(
            "CN=Alice,CN=Users,DC=corp,DC=lan",
            &[
                ("objectClass", &["top", "person", "organizationalPerson", "user"]),
                ("sAMAccountName", &["alice"]),
                ("userAccountControl", &["514"]),
            ],
        );
        let m = Member::from_entry(&e);
        assert_eq!(m.kind, MemberKind::User);
        assert_eq!(m.name, "alice");
        assert!(!m.enabled);
        let pc = entry("CN=PC1,DC=corp,DC=lan", &[("objectClass", &["user", "computer"])]);
        assert_eq!(Member::from_entry(&pc).kind, MemberKind::Computer);
        assert_eq!(Member::from_entry(&pc).name, "PC1");
    }

    #[test]
    fn verrouillage_par_lockouttime() {
        let now = 1_791_072_000;
        let mut a = Account { lockout_time: Some(now - 600), ..Account::default() };
        assert!(a.locked_out(now, Some(1_800)));
        assert!(!a.locked_out(now, Some(300)));
        assert!(a.locked_out(now, None), "sans durée, le verrou attend un administrateur");
        a.computed_uac = Some(0);
        assert!(!a.locked_out(now, None), "la valeur calculée prime");
        a.computed_uac = Some(uac::LOCKOUT);
        assert!(a.locked_out(now, Some(1)));
    }

    #[test]
    fn inactivite() {
        let now = 1_791_072_000;
        let day = 86_400;
        let recent = Account { last_logon: Some(now - 10 * day), ..Account::default() };
        assert!(!recent.stale(now, 90));
        let old = Account { last_logon: Some(now - 100 * day), ..Account::default() };
        assert!(old.stale(now, 90));
        let never_new = Account { created: Some(now - 5 * day), ..Account::default() };
        assert!(!never_new.stale(now, 90));
        let never_old = Account { created: Some(now - 200 * day), ..Account::default() };
        assert!(never_old.stale(now, 90));
    }

    #[test]
    fn sid_des_groupes_privilegies() {
        let da = PRIVILEGED_GROUPS[0];
        assert_eq!(da.sid("S-1-5-21-1-2-3"), "S-1-5-21-1-2-3-512");
        let admins = PRIVILEGED_GROUPS.iter().find(|g| g.name == "Administrators").unwrap();
        assert_eq!(admins.sid("S-1-5-21-1-2-3"), "S-1-5-32-544");
    }
}
