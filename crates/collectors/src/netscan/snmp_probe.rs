//! Interrogation SNMP d'un hôte pendant un balayage : les scalaires
//! d'identification, et le profil de collecte que l'auto-détection choisirait.
//!
//! On réutilise le collecteur SNMP existant (`crate::snmp`) et ses profils :
//! aucune logique dupliquée, et un équipement reconnu par un profil hérite de
//! son nom.

use std::collections::BTreeMap;
use std::net::IpAddr;
use std::time::Duration;

use dumbmonit_proto::{Credential, Target};

/// Ce qu'un agent SNMP a dit de lui-même, plus le profil reconnu.
#[derive(Debug, Default, Clone)]
pub struct SnmpIdentity {
    pub sysdescr: Option<String>,
    pub sysobjectid: Option<String>,
    /// Profil de collecte DumbMonit reconnu (`profiles/*.yaml`).
    pub profile_id: Option<String>,
}

/// Identifie un agent SNMP sur UDP 161. `None` si rien ne répond (community
/// refusée ou pas d'agent) : c'est le cas normal, sans erreur ni journal.
pub async fn identify(ip: IpAddr, community: &str, timeout: Duration) -> Option<SnmpIdentity> {
    let target = Target {
        id: 0,
        name: String::new(),
        address: ip.to_string(),
        kind: "snmp".into(),
        profile_id: None,
        parent_id: None,
        interval: Duration::from_secs(60),
        enabled: true,
        tags: BTreeMap::new(),
        credential: Credential::SnmpCommunity { community: community.to_string() },
        group_name: String::new(),
        position: 0,
    };
    let (identity, profile_id) = crate::snmp::identify(&target, timeout).await.ok()?;
    Some(SnmpIdentity {
        sysdescr: identity.sysdescr,
        sysobjectid: identity.sysobjectid,
        profile_id,
    })
}
