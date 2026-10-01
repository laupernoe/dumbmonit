# Kubernetes / k3s

A cluster through its API server: nodes that are not Ready or under pressure,
pods crash looping or stuck Pending, Deployments, StatefulSets and DaemonSets
missing replicas, volume claims that never get bound, and the warning events
of the last hour.

DumbMonit reads the API with the token of a dedicated service account bound
to a read-only ClusterRole: `get` and `list` on nodes, pods, workloads,
persistent volume claims and events, nothing else. It never writes, and the
role gives no access to Secrets or ConfigMaps. One device covers the whole
cluster, whatever the distribution: k3s, kubeadm, k0s, MicroK8s, Talos, or a
managed cluster whose API DumbMonit can reach.

Every minute DumbMonit reads eight collections, in parallel: the version, the
nodes, the pods, the Deployments, StatefulSets and DaemonSets, the persistent
volume claims, and the events of type `Warning` (filtered by the API server).
Long lists are read in pages of 500.

## What it watches

All metrics are prefixed `dumbmonit_k8s_`.

| Metric | What | Labels |
|---|---|---|
| `version_info` | value 1 | `version` (`1.33.4+k3s1`) |
| `nodes`, `nodes_ready` | nodes, and those whose `Ready` condition is true | |
| `node_ready` | 1 when the node is Ready | `node` |
| `node_pressure` | 1 when the node reports that pressure; only the conditions the node declares | `node`, `condition`: `memory`, `disk`, `pid`, `network` |
| `node_unschedulable` | 1 when the node is cordoned | `node` |
| `node_info` | value 1 | `node`, `kubelet_version` |
| `pods`, `pods_running`, `pods_pending`, `pods_failed`, `pods_crashlooping`, `pods_not_ready` | pod counts; completed pods (`Succeeded`) are left out | |
| `pod_ready`, `pod_crashlooping`, `pod_pending` | 1 or 0 per pod | `namespace`, `pod`, `workload` |
| `pod_restarts` | container restarts of the pod, a counter | `namespace`, `pod`, `workload` |
| `pod_blocked` | 1 while a container waits for another reason than starting: `ImagePullBackOff`, `CreateContainerConfigError`… | `namespace`, `pod`, `workload`, `reason` |
| `workload_desired`, `workload_ready`, `workload_unavailable` | replicas asked for, ready, and missing; for a DaemonSet, nodes | `kind` (`Deployment`, `StatefulSet`, `DaemonSet`), `namespace`, `workload` |
| `pvc_pending` | 1 when the claim is Pending **and** a pod needs it or a warning event concerns it | `namespace`, `pvc` |
| `pvc_bound` | 1 when the claim is Bound | `namespace`, `pvc` |
| `pvcs`, `pvcs_pending` | claims, and those stuck as above | |
| `warning_events` | warning events seen in the last hour | |
| `warning_events_by_reason` | the same by reason (`BackOff`, `FailedScheduling`, `ProvisioningFailed`…), twenty reasons at most | `reason` |

`workload` is the controller of the pod: the Deployment behind its ReplicaSet
(`api` for `api-7c9696d669-8tpp6`), the StatefulSet, the DaemonSet, or empty
for a bare pod. Pods are described one by one up to 500, problems first;
counts always cover every pod.

A claim that is merely `Pending` is not a fault: a `WaitForFirstConsumer`
storage class, the one k3s ships (`local-path`), leaves a claim pending until
the first pod uses it. It only counts as stuck when a pod asks for it or when
a warning such as `ProvisioningFailed` concerns it.

The [built-in rules](../alerting/rules.md#kubernetes) that apply:

- **Kubernetes node not ready**: a node's `Ready` condition is false or
  unknown for five minutes (Warning).
- **Kubernetes node under pressure**: memory, disk or PID pressure for five
  minutes (Advisory).
- **Kubernetes pod crash looping**: a container in `CrashLoopBackOff` for ten
  minutes (Warning). Between two attempts the container restarts and the state
  disappears for a moment; the rule reads the maximum over ten minutes so that
  it does not start over each time.
- **Kubernetes pod stuck pending**: a pod Pending for fifteen minutes
  (Advisory).
- **Kubernetes pod restarting**: more than five restarts in an hour, for a pod
  that is not already crash looping (Advisory).
- **Kubernetes workload missing replicas**: a Deployment, StatefulSet or
  DaemonSet with fewer available replicas than it asks for, for ten minutes
  (Advisory, escalates after one hour). A rolling update removes one replica
  while it starts another; the ten minutes cover it.
- **Kubernetes volume claim pending**: a stuck claim for fifteen minutes
  (Advisory).

Plus Device unreachable, when the API server stops answering.

## The device page

The Cluster panel says what is wrong in a sentence, then lists the nodes (Ready
or not, pressure, cordoned, kubelet version), the pods that need attention
(crash looping, pending, blocked, not ready, with their restarts), the
workloads missing replicas, the stuck volume claims, and the warning events of
the last hour by reason. A healthy cluster shows its counts and nothing else.

## Create a read-only service account for DumbMonit

1. Create a service account, a ClusterRole that can only get and list nodes, pods, workloads, volume claims and events, and bind them together. The role reads no Secret and no ConfigMap. On a k3s server, run each command as sudo k3s kubectl.

    ```
    kubectl create serviceaccount dumbmonit -n kube-system
    kubectl create clusterrole dumbmonit-read --verb=get,list --resource=nodes,pods,persistentvolumeclaims,events,deployments.apps,statefulsets.apps,daemonsets.apps
    kubectl create clusterrolebinding dumbmonit-read --clusterrole=dumbmonit-read --serviceaccount=kube-system:dumbmonit
    ```

2. Give it a token that does not expire: a Secret of type service-account-token, which Kubernetes fills in. Then print the token, and the cluster CA in base64.

    ```
    echo '{"apiVersion":"v1","kind":"Secret","metadata":{"name":"dumbmonit-token","namespace":"kube-system","annotations":{"kubernetes.io/service-account.name":"dumbmonit"}},"type":"kubernetes.io/service-account-token"}' | kubectl apply -f -
    kubectl -n kube-system get secret dumbmonit-token -o jsonpath='{.data.token}' | base64 -d
    kubectl -n kube-system get secret dumbmonit-token -o jsonpath='{.data.ca\.crt}'
    ```

3. In DumbMonit, enter the address of the API server, for example "https://k3s.lan:6443". Paste the token as Service account token and the base64 CA as Cluster CA certificate: only that CA is then trusted. One API server reports the whole cluster.

!!! warning
    The API server's certificate only lists its own names and addresses: an address it does not list fails verification. Use one it lists (a node IP address, or a name added with --tls-san on k3s), or tick Accept an unverifiable certificate.

### The same thing as a manifest

For GitOps, or to review exactly what is granted, apply this manifest instead
of the three `kubectl create` commands and the Secret of step 2
(`kubectl apply -f dumbmonit.yaml`), then print the token and the CA as in
step 2:

```yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  name: dumbmonit
  namespace: kube-system
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: dumbmonit-read
rules:
  - apiGroups: [""]
    resources: ["nodes", "pods", "persistentvolumeclaims", "events"]
    verbs: ["get", "list"]
  - apiGroups: ["apps"]
    resources: ["deployments", "statefulsets", "daemonsets"]
    verbs: ["get", "list"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
  name: dumbmonit-read
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: ClusterRole
  name: dumbmonit-read
subjects:
  - kind: ServiceAccount
    name: dumbmonit
    namespace: kube-system
---
apiVersion: v1
kind: Secret
metadata:
  name: dumbmonit-token
  namespace: kube-system
  annotations:
    kubernetes.io/service-account.name: dumbmonit
type: kubernetes.io/service-account-token
```

To check the role before adding the device, ask the API server what the
account may do; the first command must answer `yes`, the second `no`:

```
kubectl auth can-i list pods --as=system:serviceaccount:kube-system:dumbmonit
kubectl auth can-i list secrets --as=system:serviceaccount:kube-system:dumbmonit
```

A token made with `kubectl create token dumbmonit -n kube-system` works too,
but it expires (after one hour by default): the device then fails with "The
API server refused the token (401)". The Secret above is the long-lived form
Kubernetes documents for this use. To revoke DumbMonit's access, delete the
Secret, or the service account.

## Credentials

| Credential | Fields |
|---|---|
| Service account token | The token printed by the first `kubectl get secret` command, sent as `Authorization: Bearer`. |

Address: the API server as a URL (`https://k3s.lan:6443`), a host name or IP
(`192.0.2.10`, port 6443 then), or `host:port`. Behind a load balancer or a
highly available k3s, give the address that serves the API, the one in your
kubeconfig.

Options:

- **Port** (6443) when the address gives none.
- **Cluster CA certificate**: the base64 value of `ca.crt`, as the last
  command of step 2 prints it; a PEM certificate works too, even pasted on one
  line. With it, DumbMonit trusts that CA and nothing else.
- **Accept an unverifiable certificate**: skips every check, for a network
  you trust.
- **Timeout per request** (10 s).
- **Ignored namespaces**: pods, workloads, claims and events of these
  namespaces are left out, for example a CI namespace full of short-lived
  pods. Node events are always kept.

## Troubleshooting

"The certificate of the API server could not be verified": either the CA is
missing (paste it, as in step 2), or the address is not one of the names in
the server's certificate. k3s lists `kubernetes`, `localhost`, the node's host
name and its IP addresses; add another name with `--tls-san` (in
`/etc/rancher/k3s/config.yaml`: `tls-san: [k3s.lan]`) and restart k3s.

"The service account may not read … (403)": the ClusterRole or its binding is
missing or incomplete. The message names the resource the API server refused;
apply the manifest above again.
