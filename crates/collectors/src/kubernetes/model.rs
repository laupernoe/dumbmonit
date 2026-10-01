//! Réponses de l'API Kubernetes, réduites à ce que la supervision lit.
//!
//! Chaque structure ne déclare que ses champs utiles et tolère leur absence :
//! l'API omet les compteurs nuls (`readyReplicas` d'un déploiement dont rien ne
//! tourne n'existe pas, il ne vaut pas zéro), et deux formes d'événements
//! cohabitent dans la même liste (`lastTimestamp` et `count` d'un côté,
//! `eventTime` de l'autre, pour l'ordonnanceur).

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::Deserialize;

/// Une liste de l'API : les éléments, et le jeton de la page suivante.
#[derive(Debug, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
pub struct List<T> {
    #[serde(default)]
    pub items: Vec<T>,
    #[serde(default)]
    pub metadata: ListMeta,
}

#[derive(Debug, Default, Deserialize)]
pub struct ListMeta {
    #[serde(default, rename = "continue")]
    pub continue_token: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct Meta {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub namespace: String,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
    #[serde(default, rename = "ownerReferences")]
    pub owners: Vec<OwnerRef>,
    #[serde(default, rename = "creationTimestamp")]
    pub created: Option<DateTime<Utc>>,
}

#[derive(Debug, Default, Deserialize)]
pub struct OwnerRef {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct Version {
    #[serde(default, rename = "gitVersion")]
    pub git_version: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct Condition {
    #[serde(default, rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub reason: Option<String>,
}

// ------------------------------------------------------------------- nœuds

#[derive(Debug, Default, Deserialize)]
pub struct Node {
    #[serde(default)]
    pub metadata: Meta,
    #[serde(default)]
    pub spec: NodeSpec,
    #[serde(default)]
    pub status: NodeStatus,
}

#[derive(Debug, Default, Deserialize)]
pub struct NodeSpec {
    #[serde(default)]
    pub unschedulable: bool,
}

#[derive(Debug, Default, Deserialize)]
pub struct NodeStatus {
    #[serde(default)]
    pub conditions: Vec<Condition>,
    #[serde(default, rename = "nodeInfo")]
    pub node_info: NodeInfo,
}

#[derive(Debug, Default, Deserialize)]
pub struct NodeInfo {
    #[serde(default, rename = "kubeletVersion")]
    pub kubelet_version: String,
}

// -------------------------------------------------------------------- pods

#[derive(Debug, Default, Deserialize)]
pub struct Pod {
    #[serde(default)]
    pub metadata: Meta,
    #[serde(default)]
    pub spec: PodSpec,
    #[serde(default)]
    pub status: PodStatus,
}

#[derive(Debug, Default, Deserialize)]
pub struct PodSpec {
    #[serde(default, rename = "nodeName")]
    pub node_name: Option<String>,
    #[serde(default)]
    pub volumes: Vec<Volume>,
}

#[derive(Debug, Default, Deserialize)]
pub struct Volume {
    #[serde(default, rename = "persistentVolumeClaim")]
    pub claim: Option<ClaimRef>,
}

#[derive(Debug, Default, Deserialize)]
pub struct ClaimRef {
    #[serde(default, rename = "claimName")]
    pub claim_name: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct PodStatus {
    #[serde(default)]
    pub phase: String,
    #[serde(default)]
    pub conditions: Vec<Condition>,
    #[serde(default, rename = "containerStatuses")]
    pub containers: Vec<ContainerStatus>,
    #[serde(default, rename = "initContainerStatuses")]
    pub init_containers: Vec<ContainerStatus>,
}

#[derive(Debug, Default, Deserialize)]
pub struct ContainerStatus {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub ready: bool,
    #[serde(default, rename = "restartCount")]
    pub restart_count: u64,
    #[serde(default)]
    pub state: ContainerState,
}

#[derive(Debug, Default, Deserialize)]
pub struct ContainerState {
    #[serde(default)]
    pub waiting: Option<Waiting>,
}

#[derive(Debug, Default, Deserialize)]
pub struct Waiting {
    #[serde(default)]
    pub reason: Option<String>,
}

// ------------------------------------------------------ charges de travail

#[derive(Debug, Default, Deserialize)]
pub struct Workload {
    #[serde(default)]
    pub metadata: Meta,
    #[serde(default)]
    pub spec: WorkloadSpec,
    #[serde(default)]
    pub status: WorkloadStatus,
}

#[derive(Debug, Default, Deserialize)]
pub struct WorkloadSpec {
    /// Absent d'un DaemonSet ; vaut 1 par défaut ailleurs.
    #[serde(default)]
    pub replicas: Option<u64>,
}

/// Les champs des trois contrôleurs réunis : chacun ne remplit que les siens.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkloadStatus {
    // Deployment, StatefulSet.
    pub ready_replicas: Option<u64>,
    pub available_replicas: Option<u64>,
    pub unavailable_replicas: Option<u64>,
    // DaemonSet.
    pub desired_number_scheduled: Option<u64>,
    pub number_ready: Option<u64>,
    pub number_available: Option<u64>,
    pub number_unavailable: Option<u64>,
}

// ------------------------------------------------------------ volumes, etc.

#[derive(Debug, Default, Deserialize)]
pub struct Claim {
    #[serde(default)]
    pub metadata: Meta,
    #[serde(default)]
    pub status: ClaimStatus,
}

#[derive(Debug, Default, Deserialize)]
pub struct ClaimStatus {
    #[serde(default)]
    pub phase: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    #[serde(default)]
    pub metadata: Meta,
    #[serde(default, rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub involved_object: InvolvedObject,
    pub last_timestamp: Option<DateTime<Utc>>,
    pub event_time: Option<DateTime<Utc>>,
    pub series: Option<EventSeries>,
    #[serde(default)]
    pub count: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
pub struct InvolvedObject {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub namespace: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventSeries {
    pub last_observed_time: Option<DateTime<Utc>>,
}

impl Event {
    /// Dernière occurrence : les événements du kubelet datent par
    /// `lastTimestamp`, ceux de l'ordonnanceur par `eventTime` (et `series`
    /// quand ils se répètent) ; la création de l'objet sert de dernier recours.
    pub fn last_seen(&self) -> Option<DateTime<Utc>> {
        self.series
            .as_ref()
            .and_then(|s| s.last_observed_time)
            .or(self.last_timestamp)
            .or(self.event_time)
            .or(self.metadata.created)
    }
}
