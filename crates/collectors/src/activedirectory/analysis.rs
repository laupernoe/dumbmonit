//! De ce que l'annuaire a répondu à ce que la page montre.
//!
//! Tout ici travaille sur des structures simples (`model.rs`) et rend la vue
//! (`view.rs`) : décomptes, contrôleurs, groupes privilégiés et constats de
//! sécurité. Aucune entrée/sortie : chaque règle se teste sur un compte
//! construit à la main.
//!
//! Les constats sont des *faits bruts* — combien de comptes, lesquels — et non
//! un score : un score se calcule à partir d'eux, ailleurs.

use std::collections::{BTreeMap, HashSet};

use super::decode::{self, uac};
use super::model::{
    Account, DcComputer, DomainObject, DsaRecord, FsmoRecord, GroupRecord, Inventory, MemberKind,
    PRIVILEGED_GROUPS, RootDse,
};
use super::view::{
    DcView, DomainView, Finding, FindingSeverity, FsmoView, InventoryView, MemberView,
    NeighborView, PolicyView, PrivilegedGroupView, ProbeView, ReplicationView,
};

/// Âge du mot de passe de krbtgt au-delà duquel il est signalé : 180 jours.
pub const KRBTGT_MAX_AGE_SECONDS: i64 = 180 * 86_400;

/// Exemples gardés par constat.
pub const MAX_SAMPLES: usize = 20;

/// Membres listés par groupe privilégié (le décompte, lui, est complet).
pub const MAX_MEMBERS_LISTED: usize = 100;

/// Écart d'horloge au-delà duquel Kerberos refuse les tickets.
pub const MAX_CLOCK_SKEW_SECONDS: i64 = 300;

/// Réglages de l'analyse, lus sur la cible.
#[derive(Debug, Clone, Copy)]
pub struct Settings {
    pub stale_days_users: u32,
    pub stale_days_computers: u32,
}

/// Résultat de la vérification d'un contrôleur par TCP.
#[derive(Debug, Clone, PartialEq)]
pub enum Reach {
    Reachable,
    Unreachable(String),
    /// Le nom ne se résout pas depuis DumbMonit : on ne sait rien.
    Unresolved(String),
    NotChecked,
}

/// Ce que la partie rapide de l'interrogation a lu (chaque minute).
#[derive(Debug, Clone, Default)]
pub struct Observed {
    pub root: RootDse,
    pub domain: DomainObject,
    pub fsmo: Vec<FsmoRecord>,
    pub dsas: Vec<DsaRecord>,
    pub dc_computers: Vec<DcComputer>,
    pub groups: Vec<GroupRecord>,
    pub recycle_bin: Option<bool>,
    /// Par DN de DSA.
    pub reachability: Vec<(String, Reach)>,
    pub errors: Vec<String>,
}

/// Construit la vue complète. `view` arrive avec sa partie connexion remplie.
pub fn build(
    mut view: ProbeView,
    observed: &Observed,
    inventory: Option<&Inventory>,
    settings: Settings,
    now: i64,
) -> ProbeView {
    view.domain = Some(domain_view(observed, now));
    view.policy = Some(policy_view(&observed.domain));
    view.fsmo = fsmo_views(&observed.fsmo);
    view.dcs = dc_views(observed, &view.fsmo);
    view.replication = replication_view(observed);
    view.privileged_groups = group_views(observed);
    view.inventory = inventory.map(|inv| inventory_view(inv, &observed.domain, settings, now));
    view.findings = findings(observed, inventory, settings, now);
    view.errors = observed.errors.clone();
    view
}

// --------------------------------------------------------------------------
// Domaine, stratégie, contrôleurs
// --------------------------------------------------------------------------

fn domain_view(observed: &Observed, now: i64) -> DomainView {
    let root = &observed.root;
    DomainView {
        dns_name: decode::dns_name_of(&root.default_nc),
        naming_context: root.default_nc.clone(),
        forest_dns_name: decode::dns_name_of(&root.root_nc),
        forest_root: decode::same_dn(&root.default_nc, &root.root_nc),
        domain_sid: observed.domain.sid.clone(),
        domain_level: root.domain_functionality,
        domain_level_label: root.domain_functionality.map(decode::functional_level_label),
        forest_level: root.forest_functionality,
        forest_level_label: root.forest_functionality.map(decode::functional_level_label),
        dc_host_name: root.dns_host_name.clone(),
        dc_synchronized: root.is_synchronized,
        dc_global_catalog_ready: root.is_gc_ready,
        clock_skew_seconds: root.current_time.map(|dc| dc - now),
        recycle_bin_enabled: observed.recycle_bin,
    }
}

fn policy_view(domain: &DomainObject) -> PolicyView {
    PolicyView {
        min_password_length: domain.min_pwd_length,
        password_history_length: domain.pwd_history_length,
        max_password_age_seconds: domain.max_pwd_age_s,
        min_password_age_seconds: domain.min_pwd_age_s,
        complexity_required: domain.complexity_required(),
        lockout_threshold: domain.lockout_threshold,
        lockout_duration_seconds: domain.lockout_duration_s,
        lockout_window_seconds: domain.lockout_window_s,
        machine_account_quota: domain.machine_account_quota,
    }
}

/// Libellé lisible d'un rôle FSMO.
pub fn fsmo_label(role: &str) -> &'static str {
    match role {
        "schema" => "Schema master",
        "domain_naming" => "Domain naming master",
        "pdc" => "PDC emulator",
        "rid" => "RID master",
        "infrastructure" => "Infrastructure master",
        _ => "FSMO role",
    }
}

fn fsmo_views(records: &[FsmoRecord]) -> Vec<FsmoView> {
    records
        .iter()
        .map(|record| FsmoView {
            role: record.role.to_string(),
            label: fsmo_label(record.role).to_string(),
            holder: record.holder_dsa.as_deref().map(holder_name),
        })
        .collect()
}

/// Nom du contrôleur désigné par le DN de son « NTDS Settings ».
fn holder_name(dsa_dn: &str) -> String {
    decode::server_and_site_of_dsa(dsa_dn)
        .map(|(server, _)| server)
        .unwrap_or_else(|| dsa_dn.to_string())
}

/// Un DSA supprimé garde son DN marqué `\0ADEL:` (ou `DEL:` une fois
/// décodé) : un rôle qui pointe dessus n'a plus de détenteur.
fn orphaned_holder(dsa_dn: &str) -> bool {
    let upper = dsa_dn.to_ascii_uppercase();
    upper.contains("\\0ADEL:") || upper.contains("\nDEL:") || upper.contains("CNF:")
}

fn dc_views(observed: &Observed, fsmo: &[FsmoView]) -> Vec<DcView> {
    let queried = observed.root.ds_service_name.as_deref().unwrap_or_default();
    let mut dcs: Vec<DcView> = observed
        .dsas
        .iter()
        .map(|dsa| {
            let computer = observed.dc_computers.iter().find(|c| {
                match (&c.dns_host_name, &dsa.dns_host_name) {
                    (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
                    _ => c.name.trim_end_matches('$').eq_ignore_ascii_case(&dsa.server),
                }
            });
            let reach = observed
                .reachability
                .iter()
                .find(|(dn, _)| decode::same_dn(dn, &dsa.dsa_dn))
                .map(|(_, reach)| reach.clone())
                .unwrap_or(Reach::NotChecked);
            let is_queried = !queried.is_empty() && decode::same_dn(queried, &dsa.dsa_dn);
            let (reachability, detail) = match (is_queried, reach) {
                (true, _) | (_, Reach::Reachable) => ("reachable", None),
                (_, Reach::Unreachable(why)) => ("unreachable", Some(why)),
                (_, Reach::Unresolved(why)) => ("unresolved", Some(why)),
                (_, Reach::NotChecked) => ("not_checked", None),
            };
            DcView {
                name: dsa.server.clone(),
                host_name: dsa.dns_host_name.clone(),
                site: dsa.site.clone(),
                global_catalog: dsa.global_catalog,
                read_only: dsa.read_only,
                operating_system: computer.and_then(|c| c.operating_system.clone()),
                os_version: computer.and_then(|c| c.os_version.clone()),
                queried: is_queried,
                reachability: reachability.to_string(),
                reachability_detail: detail,
                inbound_connections: dsa.inbound_connections,
                roles: fsmo
                    .iter()
                    .filter(|role| role.holder.as_deref() == Some(dsa.server.as_str()))
                    .map(|role| role.role.clone())
                    .collect(),
            }
        })
        .collect();
    // Les injoignables d'abord, puis par nom.
    dcs.sort_by(|a, b| {
        (a.reachability == "unreachable")
            .cmp(&(b.reachability == "unreachable"))
            .reverse()
            .then_with(|| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()))
    });
    dcs
}

fn replication_view(observed: &Observed) -> ReplicationView {
    let neighbors: Vec<NeighborView> = observed
        .root
        .inbound_neighbors
        .iter()
        .map(|n| NeighborView {
            source: holder_name(&n.source_dsa_dn),
            naming_context: n.naming_context.clone(),
            last_success: n.last_success,
            last_attempt: n.last_attempt,
            last_result: n.last_result,
            consecutive_failures: n.consecutive_failures,
        })
        .collect();
    let status = if !neighbors.is_empty() {
        "readable"
    } else if observed.dsas.len() <= 1 {
        "single_dc"
    } else {
        "not_readable"
    };
    ReplicationView {
        status: status.to_string(),
        failing: neighbors.iter().filter(|n| n.consecutive_failures > 0).count(),
        neighbors,
    }
}

fn group_views(observed: &Observed) -> Vec<PrivilegedGroupView> {
    PRIVILEGED_GROUPS
        .iter()
        .map(|def| match observed.groups.iter().find(|g| g.def.rid == def.rid) {
            None => PrivilegedGroupView {
                name: def.name.to_string(),
                rid: def.rid,
                found: false,
                dn: None,
                member_count: 0,
                enabled_member_count: 0,
                nested_group_count: 0,
                members: Vec::new(),
                truncated: false,
            },
            Some(group) => {
                let accounts: Vec<_> =
                    group.members.iter().filter(|m| m.kind != MemberKind::Group).collect();
                let mut members: Vec<MemberView> = accounts
                    .iter()
                    .map(|m| MemberView { name: m.name.clone(), kind: m.kind, enabled: m.enabled })
                    .collect();
                members.sort_by(|a, b| {
                    b.enabled
                        .cmp(&a.enabled)
                        .then_with(|| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()))
                });
                let truncated = members.len() > MAX_MEMBERS_LISTED;
                members.truncate(MAX_MEMBERS_LISTED);
                PrivilegedGroupView {
                    name: def.name.to_string(),
                    rid: def.rid,
                    found: true,
                    dn: Some(group.dn.clone()),
                    member_count: accounts.len(),
                    enabled_member_count: accounts.iter().filter(|m| m.enabled).count(),
                    nested_group_count: group
                        .members
                        .iter()
                        .filter(|m| m.kind == MemberKind::Group)
                        .count(),
                    members,
                    truncated,
                }
            }
        })
        .collect()
}

/// Empreinte d'un groupe : les DN de ses membres effectifs, triés, hachés.
/// Change dès qu'un membre entre ou sort, même quand le nombre reste le même.
pub fn group_fingerprint(group: &GroupRecord) -> u32 {
    let mut dns: Vec<String> = group.members.iter().map(|m| m.dn.to_ascii_lowercase()).collect();
    dns.sort_unstable();
    dns.dedup();
    let digest = ring::digest::digest(&ring::digest::SHA256, dns.join("\n").as_bytes());
    let bytes = digest.as_ref();
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

// --------------------------------------------------------------------------
// Inventaire
// --------------------------------------------------------------------------

/// RID de krbtgt et du compte invité.
const RID_KRBTGT: u32 = 502;
const RID_GUEST: u32 = 501;

fn is_krbtgt(account: &Account) -> bool {
    account.rid() == Some(RID_KRBTGT) || account.name.eq_ignore_ascii_case("krbtgt")
}

fn krbtgt(inventory: &Inventory) -> Option<&Account> {
    inventory
        .users
        .iter()
        .find(|a| a.rid() == Some(RID_KRBTGT))
        .or_else(|| inventory.users.iter().find(|a| a.name.eq_ignore_ascii_case("krbtgt")))
}

/// Ordinateurs qui devraient porter LAPS : actifs, hors contrôleurs.
fn laps_eligible(inventory: &Inventory) -> impl Iterator<Item = &Account> {
    inventory.computers.iter().filter(|c| c.enabled() && !c.is_domain_controller())
}

fn inventory_view(
    inventory: &Inventory,
    domain: &DomainObject,
    settings: Settings,
    now: i64,
) -> InventoryView {
    let count = |iter: &mut dyn Iterator<Item = &Account>| iter.count() as u64;
    let users = &inventory.users;
    let computers = &inventory.computers;
    let krbtgt = krbtgt(inventory);
    InventoryView {
        refreshed_at: inventory.refreshed_at,
        users_total: users.len() as u64,
        users_enabled: count(&mut users.iter().filter(|a| a.enabled())),
        users_disabled: count(&mut users.iter().filter(|a| !a.enabled())),
        users_locked_out: count(
            &mut users.iter().filter(|a| a.locked_out(now, domain.lockout_duration_s)),
        ),
        users_password_never_expires: count(
            &mut users.iter().filter(|a| a.enabled() && a.has(uac::DONT_EXPIRE_PASSWORD)),
        ),
        users_stale: count(
            &mut users.iter().filter(|a| {
                a.enabled() && !is_krbtgt(a) && a.stale(now, settings.stale_days_users)
            }),
        ),
        computers_total: computers.len() as u64,
        computers_enabled: count(&mut computers.iter().filter(|a| a.enabled())),
        computers_stale: count(
            &mut computers
                .iter()
                .filter(|a| a.enabled() && a.stale(now, settings.stale_days_computers)),
        ),
        groups_total: inventory.groups_total,
        admin_count_accounts: count(&mut users.iter().filter(|a| a.admin_count)),
        krbtgt_password_last_set: krbtgt.and_then(|k| k.pwd_last_set),
        krbtgt_password_age_seconds: krbtgt.and_then(|k| k.pwd_last_set).map(|at| now - at),
        laps_schema: inventory.laps_legacy_schema || inventory.laps_windows_schema,
        laps_computers: count(&mut laps_eligible(inventory).filter(|c| c.laps)),
        laps_eligible_computers: count(&mut laps_eligible(inventory)),
        stale_days_users: settings.stale_days_users,
        stale_days_computers: settings.stale_days_computers,
    }
}

// --------------------------------------------------------------------------
// Constats
// --------------------------------------------------------------------------

struct Builder {
    findings: Vec<Finding>,
}

impl Builder {
    /// Un constat sur des objets. Toujours enregistré, à zéro quand rien n'est
    /// trouvé : un constat corrigé doit passer au vert, pas disparaître.
    fn push<'a>(
        &mut self,
        id: &str,
        severity: FindingSeverity,
        category: &str,
        title: &str,
        detail: &str,
        names: impl IntoIterator<Item = &'a str>,
    ) {
        let mut samples: Vec<String> = names.into_iter().map(str::to_string).collect();
        samples.sort_by_key(|name| name.to_ascii_lowercase());
        samples.dedup();
        let count = samples.len() as u64;
        samples.truncate(MAX_SAMPLES);
        self.findings.push(Finding {
            id: id.to_string(),
            severity,
            category: category.to_string(),
            title: title.to_string(),
            detail: detail.to_string(),
            count,
            samples,
        });
    }

    /// Un constat sur une propriété du domaine plutôt que sur des objets :
    /// `count` vaut 1 quand il est avéré, 0 sinon.
    fn fact(
        &mut self,
        id: &str,
        severity: FindingSeverity,
        category: &str,
        title: &str,
        detail: String,
        present: bool,
    ) {
        self.findings.push(Finding {
            id: id.to_string(),
            severity,
            category: category.to_string(),
            title: title.to_string(),
            detail,
            count: u64::from(present),
            samples: Vec::new(),
        });
    }
}

/// Systèmes dont le support a pris fin, en minuscules. Windows 10 est traité à
/// part : sa fin de support est récente et des mises à jour étendues existent.
const OBSOLETE_OS: &[&str] = &[
    "windows 2000",
    "windows xp",
    "windows vista",
    "windows 7",
    "windows 8",
    "windows server 2003",
    "windows server 2008",
    "windows server 2012",
    "windows embedded",
];

pub fn obsolete_os(os: &str) -> bool {
    let lower = os.to_ascii_lowercase();
    OBSOLETE_OS.iter().any(|name| lower.contains(name))
}

/// Longueur minimale recommandée par la référence du score.
pub const BASELINE_MIN_PASSWORD_LENGTH: i64 = 14;

/// Membres actifs, cumulés sur Domain Admins, Enterprise Admins et
/// Administrators, au-delà desquels les groupes privilégiés sont « trop
/// grands ».
pub const MAX_PRIVILEGED_MEMBERS: usize = 5;

/// Les constats, à zéro compris. Ceux qui dépendent de l'inventaire complet
/// n'apparaissent qu'une fois celui-ci lu : absents, ils signifient « pas
/// encore examiné », pas « rien trouvé ».
pub fn findings(
    observed: &Observed,
    inventory: Option<&Inventory>,
    settings: Settings,
    now: i64,
) -> Vec<Finding> {
    use FindingSeverity::{Critical, High, Low, Medium};
    let mut b = Builder { findings: Vec::new() };

    // --- Comptes privilégiés --------------------------------------------
    let privileged: HashSet<String> = observed
        .groups
        .iter()
        .flat_map(|g| g.members.iter())
        .filter(|m| m.kind != MemberKind::Group)
        .map(|m| m.dn.to_ascii_lowercase())
        .collect();
    if let Some(schema) = observed.groups.iter().find(|g| g.def.rid == 518) {
        b.push(
            "schema_admins_not_empty",
            Medium,
            "privileged",
            "Schema Admins is not empty",
            "Schema Admins is only needed while the schema is being extended. Keep it empty the rest of the time.",
            schema.members.iter().filter(|m| m.kind != MemberKind::Group).map(|m| m.name.as_str()),
        );
    }
    if !observed.groups.is_empty() {
        let mut members: Vec<&str> = observed
            .groups
            .iter()
            .filter(|g| matches!(g.def.rid, 512 | 519 | 544))
            .flat_map(|g| g.members.iter())
            .filter(|m| m.enabled && m.kind == MemberKind::User)
            .map(|m| m.name.as_str())
            .collect();
        members.sort_unstable();
        members.dedup();
        let too_many = members.len() > MAX_PRIVILEGED_MEMBERS;
        b.push(
            "privileged_group_size",
            Medium,
            "privileged",
            "Too many privileged accounts",
            &format!(
                "{} enabled accounts sit in Domain Admins, Enterprise Admins or Administrators, nesting included; more than {MAX_PRIVILEGED_MEMBERS} is more than a small team needs.",
                members.len()
            ),
            members.into_iter().filter(|_| too_many),
        );
    }

    // --- Infrastructure ---------------------------------------------------
    if let Some(synchronized) = observed.root.is_synchronized {
        b.fact(
            "dc_not_synchronized",
            High,
            "infrastructure",
            "Domain controller not synchronized",
            "The queried domain controller has not completed its initial replication: what it serves may be out of date.".to_string(),
            !synchronized,
        );
    }
    if let Some(skew) = observed.root.current_time.map(|dc| dc - now) {
        b.fact(
            "clock_skew",
            Medium,
            "infrastructure",
            "Clock skew above five minutes",
            format!(
                "The domain controller's clock is {} seconds away from the DumbMonit host's. Kerberos refuses tickets beyond five minutes; check which of the two clocks is wrong.",
                skew.abs()
            ),
            skew.abs() > MAX_CLOCK_SKEW_SECONDS,
        );
    }
    if let Some(level) = observed.root.domain_functionality {
        b.fact(
            "functional_level_old",
            Medium,
            "infrastructure",
            "Domain functional level below 2012 R2",
            format!(
                "The domain runs at the {} functional level: Protected Users protections, Kerberos armoring and other defences need 2012 R2 or later.",
                decode::functional_level_label(level)
            ),
            level < 6,
        );
    }
    if !observed.fsmo.is_empty() {
        b.push(
            "fsmo_role_orphaned",
            High,
            "infrastructure",
            "FSMO role held by a deleted domain controller",
            "A role points at a domain controller that no longer exists. Seize it on a live domain controller.",
            observed
                .fsmo
                .iter()
                .filter(|r| r.holder_dsa.as_deref().is_some_and(orphaned_holder))
                .map(|r| fsmo_label(r.role)),
        );
    }
    if !observed.reachability.is_empty() {
        let unreachable: Vec<String> = observed
            .reachability
            .iter()
            .filter(|(_, reach)| matches!(reach, Reach::Unreachable(_)))
            .map(|(dn, _)| holder_name(dn))
            .collect();
        b.push(
            "dc_unreachable",
            High,
            "infrastructure",
            "Domain controller unreachable",
            "DumbMonit could not open a connection to the LDAP port of these domain controllers.",
            unreachable.iter().map(String::as_str),
        );
    }
    if !observed.root.inbound_neighbors.is_empty() {
        let failing: Vec<String> = observed
            .root
            .inbound_neighbors
            .iter()
            .filter(|n| n.consecutive_failures > 0)
            .map(|n| {
                format!(
                    "{} ({}): {} failures, error {}",
                    holder_name(&n.source_dsa_dn),
                    decode::first_rdn_value(&n.naming_context).unwrap_or_default(),
                    n.consecutive_failures,
                    n.last_result
                )
            })
            .collect();
        b.push(
            "replication_failing",
            High,
            "infrastructure",
            "Inbound replication failing",
            "The queried domain controller cannot replicate from these partners. Changes made elsewhere do not reach it.",
            failing.iter().map(String::as_str),
        );
    }
    if let Some(enabled) = observed.recycle_bin {
        b.fact(
            "recycle_bin_disabled",
            Low,
            "hygiene",
            "Recycle Bin not enabled",
            "Without the AD Recycle Bin, a deleted user or group comes back without its group memberships.".to_string(),
            !enabled,
        );
    }

    // --- Stratégie de mot de passe ---------------------------------------
    let domain = &observed.domain;
    let mut weaknesses: Vec<String> = Vec::new();
    let mut serious = false;
    if let Some(length) = domain.min_pwd_length
        && length < BASELINE_MIN_PASSWORD_LENGTH
    {
        serious |= length < 8;
        weaknesses.push(format!("minimum length {length}"));
    }
    if domain.lockout_threshold == Some(0) {
        serious = true;
        weaknesses.push("no account lockout".to_string());
    }
    if domain.complexity_required() == Some(false) {
        weaknesses.push("complexity not required".to_string());
    }
    if domain.min_pwd_length.is_some() || domain.lockout_threshold.is_some() {
        b.push(
            "password_policy",
            if serious { High } else { Medium },
            "policy",
            "Default password policy below the baseline",
            &format!(
                "The baseline asks for at least {BASELINE_MIN_PASSWORD_LENGTH} characters, an account lockout threshold and complexity. Each sample is one gap in the default domain policy."
            ),
            weaknesses.iter().map(String::as_str),
        );
    }
    if let Some(quota) = domain.machine_account_quota {
        b.fact(
            "machine_account_quota",
            Low,
            "policy",
            "Any user can join computers to the domain",
            format!(
                "ms-DS-MachineAccountQuota is {quota}: every authenticated user can create that many computer accounts. Set it to 0 and delegate joining instead."
            ),
            quota > 0,
        );
    }

    let Some(inventory) = inventory else {
        return sorted(b.findings);
    };

    // --- Kerberos ---------------------------------------------------------
    if let Some(age) = krbtgt(inventory).and_then(|k| k.pwd_last_set).map(|at| now - at) {
        let old = age > KRBTGT_MAX_AGE_SECONDS;
        b.push(
            "krbtgt_password_age",
            High,
            "kerberos",
            "krbtgt password older than 180 days",
            &format!(
                "The krbtgt password was last changed {} days ago. Anyone who obtained its hash can forge tickets until it is changed twice.",
                age / 86_400
            ),
            ["krbtgt"].into_iter().filter(|_| old),
        );
    }
    let all = || inventory.users.iter().chain(inventory.computers.iter());
    let users = || inventory.users.iter().filter(|a| a.enabled());
    b.push(
        "asrep_roastable_users",
        High,
        "kerberos",
        "Kerberos pre-authentication disabled",
        "Anyone can request an encrypted blob for these accounts and crack their password offline (AS-REP roasting).",
        all().filter(|a| a.enabled() && a.has(uac::DONT_REQ_PREAUTH)).map(|a| a.name.as_str()),
    );
    b.push(
        "kerberoastable_privileged",
        Critical,
        "privileged",
        "Privileged accounts with a service principal name",
        "Any user can request a ticket for these privileged accounts and crack their password offline (Kerberoasting).",
        users()
            .filter(|a| {
                a.spn_count > 0 && !is_krbtgt(a) && privileged.contains(&a.dn.to_ascii_lowercase())
            })
            .map(|a| a.name.as_str()),
    );
    b.push(
        "kerberoastable_users",
        High,
        "kerberos",
        "User accounts with a service principal name",
        "Any user can request a ticket for these accounts and crack their password offline (Kerberoasting). Use long random passwords or group managed service accounts.",
        users().filter(|a| a.spn_count > 0 && !is_krbtgt(a)).map(|a| a.name.as_str()),
    );
    b.push(
        "unconstrained_delegation",
        Critical,
        "kerberos",
        "Unconstrained delegation",
        "These accounts (domain controllers aside) keep the Kerberos tickets of everyone who connects to them: compromising one exposes those users.",
        all()
            .filter(|a| {
                a.enabled() && a.has(uac::TRUSTED_FOR_DELEGATION) && !a.is_domain_controller()
            })
            .map(|a| a.name.as_str()),
    );
    b.push(
        "protocol_transition_delegation",
        Medium,
        "kerberos",
        "Constrained delegation with protocol transition",
        "These accounts can obtain tickets on behalf of any user, without that user's password, for the services they are allowed to delegate to.",
        all()
            .filter(|a| a.enabled() && a.has(uac::TRUSTED_TO_AUTH_FOR_DELEGATION))
            .map(|a| a.name.as_str()),
    );
    b.push(
        "des_only",
        Medium,
        "kerberos",
        "DES-only Kerberos encryption",
        "DES is broken; these accounts are flagged to use DES keys only.",
        all().filter(|a| a.enabled() && a.has(uac::USE_DES_KEY_ONLY)).map(|a| a.name.as_str()),
    );

    // --- Comptes ----------------------------------------------------------
    b.push(
        "guest_enabled",
        High,
        "accounts",
        "Guest account enabled",
        "The built-in Guest account is enabled.",
        users().filter(|a| a.rid() == Some(RID_GUEST)).map(|a| a.name.as_str()),
    );
    b.push(
        "reversible_encryption",
        High,
        "accounts",
        "Passwords stored with reversible encryption",
        "The domain keeps a recoverable copy of these passwords.",
        users().filter(|a| a.has(uac::ENCRYPTED_TEXT_PWD_ALLOWED)).map(|a| a.name.as_str()),
    );
    b.push(
        "password_not_required",
        Medium,
        "accounts",
        "Accounts allowed an empty password",
        "PASSWD_NOTREQD lets these accounts have an empty password, whatever the policy says.",
        users().filter(|a| a.has(uac::PASSWD_NOTREQD)).map(|a| a.name.as_str()),
    );
    b.push(
        "privileged_password_never_expires",
        Medium,
        "privileged",
        "Privileged accounts whose password never expires",
        "These members of privileged groups keep the same password indefinitely.",
        users()
            .filter(|a| {
                a.has(uac::DONT_EXPIRE_PASSWORD) && privileged.contains(&a.dn.to_ascii_lowercase())
            })
            .map(|a| a.name.as_str()),
    );
    b.push(
        "privileged_stale",
        Medium,
        "privileged",
        "Inactive privileged accounts",
        &format!(
            "These enabled members of privileged groups have not logged on for more than {} days. Disable the ones nobody uses.",
            settings.stale_days_users
        ),
        users()
            .filter(|a| {
                a.stale(now, settings.stale_days_users)
                    && privileged.contains(&a.dn.to_ascii_lowercase())
            })
            .map(|a| a.name.as_str()),
    );
    b.push(
        "admincount_orphans",
        Low,
        "privileged",
        "Former privileged accounts still protected by AdminSDHolder",
        "adminCount=1 but no longer in a privileged group: their permissions stay frozen and no longer inherit from their organizational unit.",
        inventory
            .users
            .iter()
            .filter(|a| {
                a.admin_count && !is_krbtgt(a) && !privileged.contains(&a.dn.to_ascii_lowercase())
            })
            .map(|a| a.name.as_str()),
    );
    b.push(
        "password_never_expires",
        Low,
        "accounts",
        "Passwords that never expire",
        "Enabled user accounts exempt from the password age policy.",
        users().filter(|a| a.has(uac::DONT_EXPIRE_PASSWORD)).map(|a| a.name.as_str()),
    );
    b.push(
        "stale_accounts",
        Medium,
        "accounts",
        "Inactive enabled accounts",
        &format!(
            "Enabled users with no logon for more than {} days and enabled computers for more than {} days (the last-logon date can lag by up to 14 days).",
            settings.stale_days_users, settings.stale_days_computers
        ),
        users()
            .filter(|a| !is_krbtgt(a) && a.stale(now, settings.stale_days_users))
            .chain(
                inventory
                    .computers
                    .iter()
                    .filter(|a| a.enabled() && a.stale(now, settings.stale_days_computers)),
            )
            .map(|a| a.name.as_str()),
    );

    // --- Hygiène ----------------------------------------------------------
    b.push(
        "obsolete_os",
        High,
        "hygiene",
        "Computers running an unsupported Windows",
        "These enabled computers run a Windows release that no longer receives security updates.",
        inventory
            .computers
            .iter()
            .filter(|c| c.enabled() && c.operating_system.as_deref().is_some_and(obsolete_os))
            .map(|c| c.name.as_str()),
    );
    b.push(
        "windows10_end_of_support",
        Medium,
        "hygiene",
        "Computers running Windows 10",
        "Windows 10 support ended in October 2025: only machines enrolled in Extended Security Updates still receive fixes.",
        inventory
            .computers
            .iter()
            .filter(|c| {
                c.enabled()
                    && c.operating_system
                        .as_deref()
                        .is_some_and(|os| os.to_ascii_lowercase().contains("windows 10"))
            })
            .map(|c| c.name.as_str()),
    );
    let laps_schema = inventory.laps_legacy_schema || inventory.laps_windows_schema;
    let covered = laps_eligible(inventory).filter(|c| c.laps).count();
    let none = !laps_schema || covered == 0;
    b.push(
        "laps_coverage",
        if none { High } else { Medium },
        "hygiene",
        "Computers without LAPS",
        if !laps_schema {
            "Neither Windows LAPS nor legacy LAPS is in the schema: local administrator passwords are likely the same on every machine."
        } else if covered == 0 {
            "The schema knows LAPS but no computer has a managed local administrator password: every machine likely shares the same one."
        } else {
            "These enabled computers have no LAPS password expiry: their local administrator password is not rotated."
        },
        laps_eligible(inventory).filter(|c| !c.laps).map(|c| c.name.as_str()),
    );

    sorted(b.findings)
}

/// Les plus graves d'abord, puis les plus nombreux ; les constats à zéro en
/// dernier.
fn sorted(mut findings: Vec<Finding>) -> Vec<Finding> {
    findings.sort_by(|a, b| {
        (b.count > 0)
            .cmp(&(a.count > 0))
            .then_with(|| b.severity.cmp(&a.severity))
            .then_with(|| b.count.cmp(&a.count))
            .then_with(|| a.id.cmp(&b.id))
    });
    findings
}

/// Nombre de constats avérés par gravité, zéros compris.
pub fn severity_counts(findings: &[Finding]) -> BTreeMap<&'static str, usize> {
    let mut counts = BTreeMap::from([("critical", 0), ("high", 0), ("low", 0), ("medium", 0)]);
    for finding in findings.iter().filter(|f| f.count > 0) {
        *counts.entry(finding.severity.as_str()).or_default() += 1;
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activedirectory::model::{Member, Neighbor, PrivilegedGroupDef};

    const NOW: i64 = 1_791_072_000;
    const DAY: i64 = 86_400;
    const SETTINGS: Settings = Settings { stale_days_users: 90, stale_days_computers: 90 };
    const DOMAIN_SID: &str = "S-1-5-21-1-2-3";

    fn user(name: &str, rid: u32, flags: u32) -> Account {
        Account {
            dn: format!("CN={name},CN=Users,DC=corp,DC=lan"),
            name: name.to_string(),
            sid: Some(format!("{DOMAIN_SID}-{rid}")),
            uac: uac::NORMAL_ACCOUNT | flags,
            last_logon: Some(NOW - DAY),
            pwd_last_set: Some(NOW - 10 * DAY),
            created: Some(NOW - 1000 * DAY),
            ..Account::default()
        }
    }

    fn computer(name: &str, flags: u32, os: &str) -> Account {
        Account {
            dn: format!("CN={name},CN=Computers,DC=corp,DC=lan"),
            name: format!("{name}$"),
            uac: uac::WORKSTATION_TRUST_ACCOUNT | flags,
            last_logon: Some(NOW - DAY),
            operating_system: Some(os.to_string()),
            ..Account::default()
        }
    }

    fn group(def: &PrivilegedGroupDef, members: &[&Account]) -> GroupRecord {
        GroupRecord {
            def: *def,
            dn: format!("CN={},CN=Users,DC=corp,DC=lan", def.name),
            members: members
                .iter()
                .map(|a| Member {
                    dn: a.dn.clone(),
                    name: a.name.clone(),
                    kind: MemberKind::User,
                    enabled: a.enabled(),
                })
                .collect(),
        }
    }

    fn def(rid: u32) -> &'static PrivilegedGroupDef {
        PRIVILEGED_GROUPS.iter().find(|g| g.rid == rid).unwrap()
    }

    fn observed() -> Observed {
        Observed {
            root: RootDse {
                default_nc: "DC=corp,DC=lan".into(),
                root_nc: "DC=corp,DC=lan".into(),
                domain_functionality: Some(7),
                current_time: Some(NOW),
                is_synchronized: Some(true),
                ..RootDse::default()
            },
            domain: DomainObject {
                sid: Some(DOMAIN_SID.into()),
                min_pwd_length: Some(14),
                lockout_threshold: Some(5),
                lockout_duration_s: Some(1_800),
                pwd_properties: Some(1),
                machine_account_quota: Some(0),
                ..DomainObject::default()
            },
            recycle_bin: Some(true),
            ..Observed::default()
        }
    }

    fn ids(findings: &[Finding]) -> Vec<&str> {
        findings.iter().map(|f| f.id.as_str()).collect()
    }

    fn find<'a>(findings: &'a [Finding], id: &str) -> &'a Finding {
        findings.iter().find(|f| f.id == id).unwrap_or_else(|| panic!("constat {id} absent"))
    }

    #[test]
    fn un_domaine_propre_ne_produit_aucun_constat() {
        let mut krbtgt = user("krbtgt", 502, uac::ACCOUNTDISABLE);
        krbtgt.pwd_last_set = Some(NOW - 30 * DAY);
        let mut pc = computer("PC1", 0, "Windows 11 Pro");
        pc.laps = true;
        let inventory = Inventory {
            refreshed_at: NOW,
            users: vec![user("alice", 1104, 0), krbtgt],
            computers: vec![pc],
            laps_windows_schema: true,
            ..Inventory::default()
        };
        let mut obs = observed();
        obs.groups = vec![group(def(512), &[&inventory.users[0]])];
        let findings = findings(&obs, Some(&inventory), SETTINGS, NOW);
        let raised: Vec<_> = findings.iter().filter(|f| f.count > 0).map(|f| f.id.as_str()).collect();
        assert!(raised.is_empty(), "{raised:?}");
        // Chaque constat examiné est publié, à zéro : un constat corrigé passe
        // au vert au lieu de disparaître.
        for id in [
            "kerberoastable_users",
            "asrep_roastable_users",
            "unconstrained_delegation",
            "krbtgt_password_age",
            "privileged_group_size",
            "laps_coverage",
            "password_policy",
            "stale_accounts",
        ] {
            assert_eq!(find(&findings, id).count, 0, "{id}");
        }
    }

    #[test]
    fn les_comptes_a_risque_sont_comptes_et_nommes() {
        let mut krbtgt = user("krbtgt", 502, uac::ACCOUNTDISABLE);
        krbtgt.pwd_last_set = Some(NOW - 400 * DAY);
        krbtgt.spn_count = 1;
        krbtgt.admin_count = true;
        let mut svc = user("svc-sql", 1105, uac::DONT_EXPIRE_PASSWORD);
        svc.spn_count = 2;
        let mut admin = user("admin-bob", 1106, uac::DONT_EXPIRE_PASSWORD);
        admin.spn_count = 1;
        admin.admin_count = true;
        admin.last_logon = Some(NOW - 200 * DAY);
        let mut orphan = user("old-admin", 1107, 0);
        orphan.admin_count = true;
        let roast = user("legacy", 1108, uac::DONT_REQ_PREAUTH);
        let disabled_roast = user("gone", 1109, uac::DONT_REQ_PREAUTH | uac::ACCOUNTDISABLE);
        let guest = user("Invité", 501, 0);
        let mut obs = observed();
        obs.groups = vec![group(def(512), &[&admin])];
        let inventory = Inventory {
            refreshed_at: NOW,
            users: vec![krbtgt, svc, admin, orphan, roast, disabled_roast, guest],
            computers: vec![
                computer(
                    "DC1",
                    uac::SERVER_TRUST_ACCOUNT | uac::TRUSTED_FOR_DELEGATION,
                    "Windows Server 2022",
                ),
                computer("APP1", uac::TRUSTED_FOR_DELEGATION, "Windows Server 2012 R2 Standard"),
                computer("OLD", uac::ACCOUNTDISABLE, "Windows XP Professional"),
                computer("PC10", 0, "Windows 10 Pro"),
            ],
            ..Inventory::default()
        };
        let f = findings(&obs, Some(&inventory), SETTINGS, NOW);

        assert_eq!(find(&f, "krbtgt_password_age").samples, ["krbtgt"]);
        assert_eq!(find(&f, "asrep_roastable_users").samples, ["legacy"]);
        // krbtgt n'est jamais « kerberoastable » : son SPN est structurel.
        assert_eq!(find(&f, "kerberoastable_users").samples, ["admin-bob", "svc-sql"]);
        assert_eq!(find(&f, "kerberoastable_privileged").samples, ["admin-bob"]);
        // Le contrôleur de domaine délègue par nature : écarté.
        assert_eq!(find(&f, "unconstrained_delegation").samples, ["APP1$"]);
        assert_eq!(find(&f, "privileged_password_never_expires").samples, ["admin-bob"]);
        assert_eq!(find(&f, "privileged_stale").samples, ["admin-bob"]);
        assert_eq!(find(&f, "admincount_orphans").samples, ["old-admin"]);
        // Le compte invité est reconnu à son RID, pas à son nom traduit.
        assert_eq!(find(&f, "guest_enabled").samples, ["Invité"]);
        // Un ordinateur désactivé n'est pas compté.
        assert_eq!(find(&f, "obsolete_os").samples, ["APP1$"]);
        assert_eq!(find(&f, "windows10_end_of_support").samples, ["PC10$"]);
        assert_eq!(find(&f, "laps_coverage").severity, FindingSeverity::High);
        assert_eq!(find(&f, "laps_coverage").samples, ["APP1$", "PC10$"]);
        assert_eq!(find(&f, "stale_accounts").samples, ["admin-bob"]);
        assert_eq!(find(&f, "kerberoastable_privileged").severity, FindingSeverity::Critical);

        // Les plus graves d'abord.
        let severities: Vec<_> =
            f.iter().filter(|x| x.count > 0).map(|x| x.severity).collect();
        let mut sorted = severities.clone();
        sorted.sort_by(|a, b| b.cmp(a));
        assert_eq!(severities, sorted);
    }

    #[test]
    fn la_strategie_et_l_infrastructure() {
        let mut obs = observed();
        obs.domain.min_pwd_length = Some(7);
        obs.domain.lockout_threshold = Some(0);
        obs.domain.pwd_properties = Some(0);
        obs.domain.machine_account_quota = Some(10);
        obs.root.domain_functionality = Some(4);
        obs.root.is_synchronized = Some(false);
        obs.root.current_time = Some(NOW + 600);
        obs.recycle_bin = Some(false);
        obs.fsmo = vec![FsmoRecord {
            role: "rid",
            holder_dsa: Some("CN=NTDS Settings\\0ADEL:1234,CN=DC9,CN=Servers,CN=S,CN=Sites,CN=Configuration,DC=corp,DC=lan".into()),
        }];
        obs.reachability = vec![(
            "CN=NTDS Settings,CN=DC2,CN=Servers,CN=Lyon,CN=Sites,CN=Configuration,DC=corp,DC=lan"
                .into(),
            Reach::Unreachable("connection refused".into()),
        )];
        obs.root.inbound_neighbors = vec![Neighbor {
            naming_context: "DC=corp,DC=lan".into(),
            source_dsa_dn: "CN=NTDS Settings,CN=DC2,CN=Servers,CN=Lyon,CN=Sites,CN=Configuration,DC=corp,DC=lan".into(),
            last_result: 1722,
            consecutive_failures: 4,
            ..Neighbor::default()
        }];
        let f = findings(&obs, None, SETTINGS, NOW);
        let policy = find(&f, "password_policy");
        assert_eq!(policy.severity, FindingSeverity::High);
        assert_eq!(policy.count, 3);
        for id in [
            "machine_account_quota",
            "functional_level_old",
            "dc_not_synchronized",
            "clock_skew",
            "recycle_bin_disabled",
            "fsmo_role_orphaned",
        ] {
            assert_eq!(find(&f, id).count, 1, "{id}");
        }
        assert_eq!(find(&f, "dc_unreachable").samples, ["DC2"]);
        assert_eq!(find(&f, "replication_failing").samples, ["DC2 (corp): 4 failures, error 1722"]);
        // Sans inventaire, aucun constat sur les comptes.
        assert!(!ids(&f).contains(&"asrep_roastable_users"));
    }

    #[test]
    fn la_vue_des_groupes_et_des_controleurs() {
        let alice = user("alice", 1104, 0);
        let bob = user("bob", 1105, uac::ACCOUNTDISABLE);
        let mut obs = observed();
        obs.root.ds_service_name = Some(
            "CN=NTDS Settings,CN=DC1,CN=Servers,CN=Paris,CN=Sites,CN=Configuration,DC=corp,DC=lan"
                .into(),
        );
        obs.groups = vec![group(def(512), &[&bob, &alice])];
        obs.groups[0].members.push(Member {
            dn: "CN=Tier0,DC=corp,DC=lan".into(),
            name: "Tier0".into(),
            kind: MemberKind::Group,
            enabled: true,
        });
        obs.dsas = vec![
            DsaRecord {
                dsa_dn: obs.root.ds_service_name.clone().unwrap(),
                server: "DC1".into(),
                site: "Paris".into(),
                dns_host_name: Some("dc1.corp.lan".into()),
                global_catalog: true,
                ..DsaRecord::default()
            },
            DsaRecord {
                dsa_dn: "CN=NTDS Settings,CN=DC2,CN=Servers,CN=Lyon,CN=Sites,CN=Configuration,DC=corp,DC=lan".into(),
                server: "DC2".into(),
                site: "Lyon".into(),
                dns_host_name: Some("dc2.corp.lan".into()),
                inbound_connections: 1,
                ..DsaRecord::default()
            },
        ];
        obs.reachability = vec![(obs.dsas[1].dsa_dn.clone(), Reach::Unreachable("timeout".into()))];
        obs.fsmo = vec![FsmoRecord { role: "pdc", holder_dsa: Some(obs.dsas[0].dsa_dn.clone()) }];
        obs.dc_computers = vec![DcComputer {
            dns_host_name: Some("DC1.corp.lan".into()),
            name: "DC1$".into(),
            operating_system: Some("Windows Server 2022 Standard".into()),
            os_version: None,
        }];
        let view = build(ProbeView::default(), &obs, None, SETTINGS, NOW);

        let da = view.privileged_groups.iter().find(|g| g.name == "Domain Admins").unwrap();
        assert!(da.found);
        assert_eq!(da.member_count, 2);
        assert_eq!(da.enabled_member_count, 1);
        assert_eq!(da.nested_group_count, 1);
        assert_eq!(da.members[0].name, "alice", "les comptes actifs d'abord");
        let ea = view.privileged_groups.iter().find(|g| g.name == "Enterprise Admins").unwrap();
        assert!(!ea.found);

        assert_eq!(view.dcs[0].name, "DC2", "l'injoignable d'abord");
        assert_eq!(view.dcs[0].reachability, "unreachable");
        assert_eq!(view.dcs[1].reachability, "reachable");
        assert!(view.dcs[1].queried);
        assert_eq!(view.dcs[1].roles, ["pdc"]);
        assert_eq!(view.dcs[1].operating_system.as_deref(), Some("Windows Server 2022 Standard"));
        assert_eq!(view.replication.status, "not_readable");
        let domain = view.domain.unwrap();
        assert_eq!(domain.dns_name, "corp.lan");
        assert!(domain.forest_root);
        assert_eq!(domain.clock_skew_seconds, Some(0));
    }

    #[test]
    fn l_empreinte_change_avec_les_membres_pas_avec_l_ordre() {
        let a = user("a", 1, 0);
        let b = user("b", 2, 0);
        let c = user("c", 3, 0);
        let ab = group(def(512), &[&a, &b]);
        let ba = group(def(512), &[&b, &a]);
        let ac = group(def(512), &[&a, &c]);
        assert_eq!(group_fingerprint(&ab), group_fingerprint(&ba));
        assert_ne!(group_fingerprint(&ab), group_fingerprint(&ac));
    }

    #[test]
    fn l_inventaire_compte_ce_qu_il_faut() {
        let mut locked = user("locked", 1200, 0);
        locked.lockout_time = Some(NOW - 60);
        let mut stale = user("stale", 1201, uac::DONT_EXPIRE_PASSWORD);
        stale.last_logon = Some(NOW - 365 * DAY);
        let mut krbtgt = user("krbtgt", 502, uac::ACCOUNTDISABLE);
        krbtgt.pwd_last_set = Some(NOW - 200 * DAY);
        krbtgt.last_logon = None;
        let mut laps = computer("PC1", 0, "Windows 11");
        laps.laps = true;
        let inventory = Inventory {
            refreshed_at: NOW,
            users: vec![locked, stale, krbtgt],
            computers: vec![
                laps,
                computer("PC2", 0, "Windows 11"),
                computer("DC1", uac::SERVER_TRUST_ACCOUNT, "Windows Server 2022"),
            ],
            groups_total: 42,
            laps_legacy_schema: true,
            ..Inventory::default()
        };
        let view = inventory_view(&inventory, &observed().domain, SETTINGS, NOW);
        assert_eq!(view.users_total, 3);
        assert_eq!(view.users_enabled, 2);
        assert_eq!(view.users_locked_out, 1);
        assert_eq!(view.users_password_never_expires, 1);
        assert_eq!(view.users_stale, 1);
        assert_eq!(view.krbtgt_password_age_seconds, Some(200 * DAY));
        assert_eq!(view.laps_eligible_computers, 2);
        assert_eq!(view.laps_computers, 1);
        assert_eq!(view.groups_total, 42);
        let f = findings(&observed(), Some(&inventory), SETTINGS, NOW);
        assert_eq!(find(&f, "laps_coverage").samples, ["PC2$"]);
        assert_eq!(find(&f, "laps_coverage").severity, FindingSeverity::Medium);
        assert_eq!(severity_counts(&f)["critical"], 0);
    }
}
