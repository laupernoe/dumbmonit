//! Ce que la page d'un domaine Active Directory montre, et ce que le serveur
//! conserve entre deux interrogations.
//!
//! Le collecteur ne connaît pas la base : il livre cette vue à un observateur
//! (`ProbeObserver`), que le serveur range dans SQLite. Les dates sont en
//! secondes Unix.

use async_trait::async_trait;
use dumbmonit_proto::Target;
use serde::{Deserialize, Serialize};

use super::model::MemberKind;

/// Destinataire des vues d'interrogation (le serveur les range en base).
#[async_trait]
pub trait ProbeObserver: Send + Sync {
    async fn observe(&self, target: &Target, view: &ProbeView);
}

/// Tout ce qu'une interrogation a vu.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ProbeView {
    pub probed_at: i64,
    pub connection: ConnectionView,
    /// Raison du refus de liaison du compte de service ; le reste de la vue est
    /// alors vide.
    pub bind_error: Option<String>,
    pub domain: Option<DomainView>,
    pub policy: Option<PolicyView>,
    pub dcs: Vec<DcView>,
    pub fsmo: Vec<FsmoView>,
    pub replication: ReplicationView,
    pub privileged_groups: Vec<PrivilegedGroupView>,
    /// `None` avant le premier inventaire complet (relu en tâche de fond).
    pub inventory: Option<InventoryView>,
    /// Constats de sécurité, les plus graves d'abord.
    pub findings: Vec<Finding>,
    /// Parties de l'annuaire qui n'ont pas pu être lues, en clair.
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ConnectionView {
    /// Contrôleur interrogé, tel que saisi.
    pub host: String,
    pub port: u16,
    /// `ldaps`, `starttls` ou `plain`.
    pub security: String,
    /// Faux quand le certificat n'est pas vérifié (`insecure_tls`) ou en clair.
    pub certificate_verified: bool,
    pub connect_seconds: Option<f64>,
    pub bind_seconds: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct DomainView {
    pub dns_name: String,
    pub naming_context: String,
    pub forest_dns_name: String,
    /// Vrai quand ce domaine est la racine de la forêt.
    pub forest_root: bool,
    pub domain_sid: Option<String>,
    pub domain_level: Option<i64>,
    pub domain_level_label: Option<String>,
    pub forest_level: Option<i64>,
    pub forest_level_label: Option<String>,
    /// Le contrôleur interrogé.
    pub dc_host_name: Option<String>,
    pub dc_synchronized: Option<bool>,
    pub dc_global_catalog_ready: Option<bool>,
    /// Écart entre l'horloge du contrôleur et celle de DumbMonit (secondes,
    /// positif quand le contrôleur avance). Kerberos refuse au-delà de 5 min.
    pub clock_skew_seconds: Option<i64>,
    pub recycle_bin_enabled: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PolicyView {
    pub min_password_length: Option<i64>,
    pub password_history_length: Option<i64>,
    /// `None` : les mots de passe n'expirent jamais.
    pub max_password_age_seconds: Option<i64>,
    pub min_password_age_seconds: Option<i64>,
    pub complexity_required: Option<bool>,
    /// `0` : aucun verrouillage après des échecs répétés.
    pub lockout_threshold: Option<i64>,
    /// `None` : verrouillé jusqu'au déblocage par un administrateur.
    pub lockout_duration_seconds: Option<i64>,
    pub lockout_window_seconds: Option<i64>,
    /// Ordinateurs qu'un utilisateur ordinaire peut joindre au domaine.
    pub machine_account_quota: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct DcView {
    pub name: String,
    pub host_name: Option<String>,
    pub site: String,
    pub global_catalog: bool,
    pub read_only: bool,
    pub operating_system: Option<String>,
    pub os_version: Option<String>,
    /// Le contrôleur que DumbMonit interroge.
    pub queried: bool,
    /// `reachable`, `unreachable`, `unresolved` (le nom ne se résout pas
    /// depuis DumbMonit) ou `not_checked`.
    pub reachability: String,
    pub reachability_detail: Option<String>,
    pub inbound_connections: usize,
    /// Rôles FSMO détenus.
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FsmoView {
    /// `schema`, `domain_naming`, `pdc`, `rid`, `infrastructure`.
    pub role: String,
    pub label: String,
    /// Nom du contrôleur détenteur.
    pub holder: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ReplicationView {
    /// `readable` : les voisins sont lus ; `not_readable` : le compte n'a pas le
    /// droit de les lire (ou le contrôleur n'a aucun partenaire alors qu'il a des
    /// connexions) ; `single_dc` : un seul contrôleur, rien à répliquer.
    pub status: String,
    pub neighbors: Vec<NeighborView>,
    pub failing: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct NeighborView {
    pub source: String,
    pub naming_context: String,
    pub last_success: Option<i64>,
    pub last_attempt: Option<i64>,
    pub last_result: u32,
    pub consecutive_failures: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrivilegedGroupView {
    /// Nom anglais canonique (`Domain Admins`), quel que soit la langue du
    /// domaine.
    pub name: String,
    pub rid: u32,
    /// Absent de ce domaine (Enterprise et Schema Admins vivent dans le
    /// domaine racine de la forêt).
    pub found: bool,
    pub dn: Option<String>,
    /// Membres effectifs, imbrication comprise, groupes exclus.
    pub member_count: usize,
    pub enabled_member_count: usize,
    pub nested_group_count: usize,
    /// Les membres, les comptes actifs d'abord, plafonnés.
    pub members: Vec<MemberView>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemberView {
    pub name: String,
    pub kind: MemberKind,
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct InventoryView {
    pub refreshed_at: i64,
    pub users_total: u64,
    pub users_enabled: u64,
    pub users_disabled: u64,
    pub users_locked_out: u64,
    pub users_password_never_expires: u64,
    pub users_stale: u64,
    pub computers_total: u64,
    pub computers_enabled: u64,
    pub computers_stale: u64,
    pub groups_total: u64,
    /// Comptes protégés par AdminSDHolder (`adminCount=1`).
    pub admin_count_accounts: u64,
    pub krbtgt_password_last_set: Option<i64>,
    pub krbtgt_password_age_seconds: Option<i64>,
    pub laps_schema: bool,
    /// Ordinateurs actifs (hors contrôleurs) avec une date d'expiration LAPS.
    pub laps_computers: u64,
    pub laps_eligible_computers: u64,
    pub stale_days_users: u32,
    pub stale_days_computers: u32,
}

/// Gravité d'un constat, sur l'échelle du score de sécurité
/// (`security::checks`) : low → medium → high → critical. L'interface la
/// traduit en ses propres mots (info, advisory, warning).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum FindingSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl FindingSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

/// Un constat de sécurité : un fait brut, compté, avec quelques exemples.
///
/// Les identifiants sont stables : ils nomment les séries `ad_finding` et
/// servent de clé à qui calcule un score à partir de ces constats.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Finding {
    pub id: String,
    pub severity: FindingSeverity,
    /// `privileged`, `kerberos`, `accounts`, `policy`, `hygiene`,
    /// `infrastructure`.
    pub category: String,
    pub title: String,
    /// Pourquoi c'est un risque, et quoi faire.
    pub detail: String,
    /// Objets concernés ; `0` : constat examiné, rien trouvé.
    pub count: u64,
    /// Quelques objets concernés (noms de compte), triés.
    #[serde(alias = "sample_objects")]
    pub samples: Vec<String>,
}
