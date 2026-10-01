//! Traduction de l'état du cluster en échantillons.
//!
//! Tout est pur : les listes déjà lues entrent, les échantillons sortent. Les
//! pannes que ce module existe pour voir :
//!
//! * **un nœud `NotReady`**, ou sous pression (mémoire, disque, PID) : le
//!   kubelet expulse les pods ou n'en accepte plus ;
//! * **un pod en `CrashLoopBackOff`**, ou qui redémarre sans arrêt ;
//! * **un pod coincé en `Pending`** : pas de place, un volume qui ne vient pas ;
//! * **une charge de travail à qui il manque des réplicas** ;
//! * **un volume persistant jamais provisionné**.
//!
//! Les pods terminés (`Succeeded`) n'ont pas de série : un cluster qui fait
//! tourner des tâches planifiées en laisserait des centaines.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Duration, Utc};
use dumbmonit_proto::{MetricKind, Sample};

use super::model::{Claim, Event, List, Node, Pod, Version, Workload};

/// Pods décrits un par un, au plus ; les totaux couvrent toujours tout.
pub const MAX_PODS: usize = 500;
/// Charges de travail décrites une par une, au plus.
pub const MAX_WORKLOADS: usize = 300;
/// Raisons d'événements d'avertissement détaillées, au plus.
const MAX_EVENT_REASONS: usize = 20;
/// Fenêtre des événements d'avertissement comptés : Kubernetes les garde une
/// heure par défaut, on compte donc la dernière heure, quel que soit le réglage.
const EVENT_WINDOW_MINUTES: i64 = 60;

/// Conditions de pression d'un nœud, et le nom court de leur étiquette.
const PRESSURES: &[(&str, &str)] = &[
    ("MemoryPressure", "memory"),
    ("DiskPressure", "disk"),
    ("PIDPressure", "pid"),
    ("NetworkUnavailable", "network"),
];

/// Tout ce qu'une interrogation a lu.
#[derive(Debug, Default)]
pub struct Cluster {
    pub version: Version,
    pub nodes: Vec<Node>,
    pub pods: Vec<Pod>,
    pub deployments: Vec<Workload>,
    pub statefulsets: Vec<Workload>,
    pub daemonsets: Vec<Workload>,
    pub claims: Vec<Claim>,
    pub warnings: Vec<Event>,
}

impl Cluster {
    /// Retire les espaces de noms exclus par l'utilisateur.
    pub fn without_namespaces(mut self, excluded: &[String]) -> Self {
        if excluded.is_empty() {
            return self;
        }
        let keep = |ns: &str| !excluded.iter().any(|e| e == ns);
        self.pods.retain(|p| keep(&p.metadata.namespace));
        self.deployments.retain(|w| keep(&w.metadata.namespace));
        self.statefulsets.retain(|w| keep(&w.metadata.namespace));
        self.daemonsets.retain(|w| keep(&w.metadata.namespace));
        self.claims.retain(|c| keep(&c.metadata.namespace));
        self.warnings.retain(|e| {
            e.involved_object.namespace.is_empty() || keep(&e.involved_object.namespace)
        });
        self
    }
}

/// Le déploiement derrière un pod : `api-7c9696d669-8tpp6` appartient au
/// ReplicaSet `api-7c9696d669`, lui-même au déploiement `api` — le suffixe est
/// l'étiquette `pod-template-hash`. Vide pour un pod sans contrôleur.
pub fn workload_of(pod: &Pod) -> String {
    let Some(owner) = pod.metadata.owners.first() else { return String::new() };
    if owner.kind == "ReplicaSet"
        && let Some(hash) = pod.metadata.labels.get("pod-template-hash")
        && let Some(name) = owner.name.strip_suffix(&format!("-{hash}"))
    {
        return name.to_string();
    }
    owner.name.clone()
}

/// Un conteneur du pod attend dans une boucle de redémarrage.
pub fn crashlooping(pod: &Pod) -> bool {
    pod.status.containers.iter().chain(&pod.status.init_containers).any(|c| {
        c.state.waiting.as_ref().and_then(|w| w.reason.as_deref()) == Some("CrashLoopBackOff")
    })
}

/// Pourquoi un conteneur attend, quand ce n'est pas un simple démarrage :
/// image introuvable, configuration invalide… `None` si rien ne bloque.
pub fn blocked_reason(pod: &Pod) -> Option<&str> {
    pod.status.containers.iter().chain(&pod.status.init_containers).find_map(|c| {
        let reason = c.state.waiting.as_ref()?.reason.as_deref()?;
        (!matches!(reason, "ContainerCreating" | "PodInitializing" | "CrashLoopBackOff"))
            .then_some(reason)
    })
}

fn pod_ready(pod: &Pod) -> bool {
    pod.status.conditions.iter().any(|c| c.kind == "Ready" && c.status == "True")
}

fn condition_true(node: &Node, kind: &str) -> bool {
    node.status.conditions.iter().any(|c| c.kind == kind && c.status == "True")
}

pub fn samples(cluster: &Cluster, now: DateTime<Utc>) -> Vec<Sample> {
    let ts_ms = now.timestamp_millis();
    let gauge = |name: &str, value: f64| Sample::new(name, value, MetricKind::Gauge, ts_ms);
    let flag = |value: bool| if value { 1.0 } else { 0.0 };
    let mut out = Vec::new();

    let version = cluster.version.git_version.trim_start_matches('v');
    if !version.is_empty() {
        out.push(gauge("k8s_version_info", 1.0).with_label("version", version));
    }

    // Nœuds.
    let mut ready_nodes = 0;
    for node in &cluster.nodes {
        let name = node.metadata.name.as_str();
        let ready = condition_true(node, "Ready");
        ready_nodes += usize::from(ready);
        out.push(gauge("k8s_node_ready", flag(ready)).with_label("node", name));
        out.push(
            gauge("k8s_node_unschedulable", flag(node.spec.unschedulable)).with_label("node", name),
        );
        for (condition, short) in PRESSURES {
            // NetworkUnavailable n'existe que sous certains réseaux : pas de
            // série inventée quand le nœud ne la déclare pas.
            if node.status.conditions.iter().any(|c| c.kind == *condition) {
                out.push(
                    gauge("k8s_node_pressure", flag(condition_true(node, condition)))
                        .with_label("node", name)
                        .with_label("condition", *short),
                );
            }
        }
        let kubelet = node.status.node_info.kubelet_version.trim_start_matches('v');
        if !kubelet.is_empty() {
            out.push(
                gauge("k8s_node_info", 1.0)
                    .with_label("node", name)
                    .with_label("kubelet_version", kubelet),
            );
        }
    }
    out.push(gauge("k8s_nodes", cluster.nodes.len() as f64));
    out.push(gauge("k8s_nodes_ready", ready_nodes as f64));

    // Pods.
    let live: Vec<&Pod> = cluster.pods.iter().filter(|p| p.status.phase != "Succeeded").collect();
    let count = |f: &dyn Fn(&Pod) -> bool| live.iter().filter(|p| f(p)).count() as f64;
    out.push(gauge("k8s_pods", live.len() as f64));
    out.push(gauge("k8s_pods_running", count(&|p| p.status.phase == "Running")));
    out.push(gauge("k8s_pods_pending", count(&|p| p.status.phase == "Pending")));
    out.push(gauge("k8s_pods_failed", count(&|p| p.status.phase == "Failed")));
    out.push(gauge("k8s_pods_crashlooping", count(&|p| crashlooping(p))));
    out.push(gauge("k8s_pods_not_ready", count(&|p| p.status.phase == "Running" && !pod_ready(p))));
    // Les pods à problème d'abord : c'est eux qu'on veut voir si le plafond coupe.
    let mut detailed = live.clone();
    detailed.sort_by_key(|p| {
        (
            !(crashlooping(p) || p.status.phase != "Running" || !pod_ready(p)),
            p.metadata.name.clone(),
        )
    });
    for pod in detailed.into_iter().take(MAX_PODS) {
        let labelled = |sample: Sample| {
            sample
                .with_label("namespace", &pod.metadata.namespace)
                .with_label("pod", &pod.metadata.name)
                .with_label("workload", workload_of(pod))
        };
        let restarts: u64 = pod.status.containers.iter().map(|c| c.restart_count).sum();
        out.push(labelled(gauge("k8s_pod_ready", flag(pod_ready(pod)))));
        out.push(labelled(gauge("k8s_pod_crashlooping", flag(crashlooping(pod)))));
        out.push(labelled(gauge("k8s_pod_pending", flag(pod.status.phase == "Pending"))));
        out.push(labelled(Sample::new(
            "k8s_pod_restarts",
            restarts as f64,
            MetricKind::Counter,
            ts_ms,
        )));
        if let Some(reason) = blocked_reason(pod) {
            out.push(labelled(gauge("k8s_pod_blocked", 1.0)).with_label("reason", reason));
        }
    }

    // Charges de travail : Deployment et StatefulSet comptent leurs réplicas,
    // DaemonSet ses nœuds.
    let mut workloads = 0;
    for (kind, list) in [
        ("Deployment", &cluster.deployments),
        ("StatefulSet", &cluster.statefulsets),
        ("DaemonSet", &cluster.daemonsets),
    ] {
        for workload in list {
            if workloads >= MAX_WORKLOADS {
                break;
            }
            workloads += 1;
            let status = &workload.status;
            let (desired, ready, unavailable) = if kind == "DaemonSet" {
                let desired = status.desired_number_scheduled.unwrap_or(0);
                let available = status.number_available.unwrap_or(0);
                let missing =
                    status.number_unavailable.unwrap_or(desired.saturating_sub(available));
                (desired, status.number_ready.unwrap_or(0), missing)
            } else {
                let desired = workload.spec.replicas.unwrap_or(1);
                let available = status.available_replicas.unwrap_or(0);
                let missing = status.unavailable_replicas.unwrap_or(0);
                (
                    desired,
                    status.ready_replicas.unwrap_or(0),
                    missing.max(desired.saturating_sub(available)),
                )
            };
            let labelled = |sample: Sample| {
                sample
                    .with_label("kind", kind)
                    .with_label("namespace", &workload.metadata.namespace)
                    .with_label("workload", &workload.metadata.name)
            };
            out.push(labelled(gauge("k8s_workload_desired", desired as f64)));
            out.push(labelled(gauge("k8s_workload_ready", ready as f64)));
            out.push(labelled(gauge("k8s_workload_unavailable", unavailable as f64)));
        }
    }

    // Volumes persistants. Un volume `Pending` n'est un problème que si l'on
    // en a besoin : une classe `WaitForFirstConsumer` (celle de k3s) le laisse
    // en attente tant qu'aucun pod ne s'en sert, et c'est normal. Il compte
    // donc quand un pod le réclame, ou quand un avertissement le concerne
    // (`ProvisioningFailed`).
    let claimed: BTreeSet<(&str, &str)> = cluster
        .pods
        .iter()
        .flat_map(|p| {
            p.spec.volumes.iter().filter_map(move |v| {
                Some((p.metadata.namespace.as_str(), v.claim.as_ref()?.claim_name.as_str()))
            })
        })
        .collect();
    let warned: BTreeSet<(&str, &str)> = cluster
        .warnings
        .iter()
        .filter(|e| e.involved_object.kind == "PersistentVolumeClaim")
        .map(|e| (e.involved_object.namespace.as_str(), e.involved_object.name.as_str()))
        .collect();
    let mut pending_claims = 0;
    for claim in &cluster.claims {
        let key = (claim.metadata.namespace.as_str(), claim.metadata.name.as_str());
        let stuck =
            claim.status.phase == "Pending" && (claimed.contains(&key) || warned.contains(&key));
        pending_claims += usize::from(stuck);
        out.push(
            gauge("k8s_pvc_pending", flag(stuck))
                .with_label("namespace", &claim.metadata.namespace)
                .with_label("pvc", &claim.metadata.name),
        );
        out.push(
            gauge("k8s_pvc_bound", flag(claim.status.phase == "Bound"))
                .with_label("namespace", &claim.metadata.namespace)
                .with_label("pvc", &claim.metadata.name),
        );
    }
    out.push(gauge("k8s_pvcs", cluster.claims.len() as f64));
    out.push(gauge("k8s_pvcs_pending", pending_claims as f64));

    // Avertissements de la dernière heure, au total et par raison.
    let since = now - Duration::minutes(EVENT_WINDOW_MINUTES);
    let recent: Vec<&Event> = cluster
        .warnings
        .iter()
        .filter(|e| e.kind == "Warning" && e.last_seen().is_some_and(|t| t >= since))
        .collect();
    out.push(gauge("k8s_warning_events", recent.len() as f64));
    let mut by_reason: BTreeMap<&str, usize> = BTreeMap::new();
    for event in &recent {
        *by_reason.entry(event.reason.as_str()).or_default() += 1;
    }
    let mut by_reason: Vec<(&str, usize)> = by_reason.into_iter().collect();
    by_reason.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    for (reason, n) in by_reason.into_iter().take(MAX_EVENT_REASONS) {
        out.push(gauge("k8s_warning_events_by_reason", n as f64).with_label("reason", reason));
    }
    out
}

/// Lecture d'une liste JSON, pour les tests et le client.
pub fn parse_list<T: serde::de::DeserializeOwned>(
    text: &str,
) -> Result<List<T>, serde_json::Error> {
    serde_json::from_str(text)
}
