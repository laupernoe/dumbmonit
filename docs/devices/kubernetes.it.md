# Kubernetes / k3s

Un cluster tramite il suo API server: nodi non Ready o sotto pressione,
pod in crash loop o bloccati Pending, Deployment, StatefulSet e DaemonSet
mancanti di repliche, volume claim che non vengono mai bound, e gli
eventi di avviso dell'ultima ora.

DumbMonit legge l'API con il token di un service account dedicato
collegato a una ClusterRole di sola lettura: `get` e `list` su nodi, pod,
workload, persistent volume claim ed eventi, nient'altro. Non scrive mai,
e il ruolo non dà accesso a Secret o ConfigMap. Un dispositivo copre
l'intero cluster, qualunque sia la distribuzione: k3s, kubeadm, k0s,
MicroK8s, Talos, oppure un cluster gestito la cui API DumbMonit può
raggiungere.

Ogni minuto DumbMonit legge otto collezioni, in parallelo: la versione, i
nodi, i pod, i Deployment, StatefulSet e DaemonSet, i persistent volume
claim, e gli eventi di tipo `Warning` (filtrati dall'API server). Gli
elenchi lunghi vengono letti in pagine di 500.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_k8s_`.

| Metrica | Cosa | Etichette |
|---|---|---|
| `version_info` | valore 1 | `version` (`1.33.4+k3s1`) |
| `nodes`, `nodes_ready` | nodi, e quelli la cui condizione `Ready` è vera | |
| `node_ready` | 1 quando il nodo è Ready | `node` |
| `node_pressure` | 1 quando il nodo segnala quella pressione; solo le condizioni che il nodo dichiara | `node`, `condition`: `memory`, `disk`, `pid`, `network` |
| `node_unschedulable` | 1 quando il nodo è cordoned | `node` |
| `node_info` | valore 1 | `node`, `kubelet_version` |
| `pods`, `pods_running`, `pods_pending`, `pods_failed`, `pods_crashlooping`, `pods_not_ready` | conteggi di pod; i pod completati (`Succeeded`) vengono esclusi | |
| `pod_ready`, `pod_crashlooping`, `pod_pending` | 1 o 0 per pod | `namespace`, `pod`, `workload` |
| `pod_restarts` | riavvii dei container del pod, un contatore | `namespace`, `pod`, `workload` |
| `pod_blocked` | 1 mentre un container attende per un motivo diverso dall'avvio: `ImagePullBackOff`, `CreateContainerConfigError`… | `namespace`, `pod`, `workload`, `reason` |
| `workload_desired`, `workload_ready`, `workload_unavailable` | repliche richieste, pronte, e mancanti; per un DaemonSet, nodi | `kind` (`Deployment`, `StatefulSet`, `DaemonSet`), `namespace`, `workload` |
| `pvc_pending` | 1 quando il claim è Pending **e** un pod ne ha bisogno oppure un evento di avviso lo riguarda | `namespace`, `pvc` |
| `pvc_bound` | 1 quando il claim è Bound | `namespace`, `pvc` |
| `pvcs`, `pvcs_pending` | claim, e quelli bloccati come sopra | |
| `warning_events` | eventi di avviso visti nell'ultima ora | |
| `warning_events_by_reason` | lo stesso per motivo (`BackOff`, `FailedScheduling`, `ProvisioningFailed`…), al massimo venti motivi | `reason` |

`workload` è il controller del pod: il Deployment dietro il suo
ReplicaSet (`api` per `api-7c9696d669-8tpp6`), lo StatefulSet, il
DaemonSet, o vuoto per un pod nudo. I pod sono descritti uno per uno
fino a 500, i problemi prima; i conteggi coprono sempre ogni pod.

Un claim che è semplicemente `Pending` non è un guasto: una storage
class `WaitForFirstConsumer`, quella fornita da k3s (`local-path`),
lascia un claim in pending finché il primo pod non lo usa. Conta come
bloccato solo quando un pod lo richiede oppure quando un avviso come
`ProvisioningFailed` lo riguarda.

Le [regole integrate](../alerting/rules.md#kubernetes) che si applicano:

- **Kubernetes node not ready**: la condizione `Ready` di un nodo è
  falsa o sconosciuta per cinque minuti (Warning).
- **Kubernetes node under pressure**: pressione di memoria, disco o PID
  per cinque minuti (Advisory).
- **Kubernetes pod crash looping**: un container in `CrashLoopBackOff`
  per dieci minuti (Warning). Tra due tentativi il container si riavvia e
  lo stato scompare per un momento; la regola legge il massimo su dieci
  minuti così non ricomincia da capo ogni volta.
- **Kubernetes pod stuck pending**: un pod Pending per quindici minuti
  (Advisory).
- **Kubernetes pod restarting**: più di cinque riavvii in un'ora, per un
  pod che non è già in crash loop (Advisory).
- **Kubernetes workload missing replicas**: un Deployment, StatefulSet o
  DaemonSet con meno repliche disponibili di quante richiede, per dieci
  minuti (Advisory, aumenta dopo un'ora). Un rolling update rimuove una
  replica mentre ne avvia un'altra; i dieci minuti lo coprono.
- **Kubernetes volume claim pending**: un claim bloccato per quindici
  minuti (Advisory).

Più Device unreachable, quando l'API server smette di rispondere.

## La pagina del dispositivo

Il pannello Cluster indica cosa non va in una frase, poi elenca i nodi
(Ready o no, pressione, cordoned, versione kubelet), i pod che
richiedono attenzione (crash loop, pending, bloccati, non ready, con i
loro riavvii), i workload mancanti di repliche, i volume claim
bloccati, e gli eventi di avviso dell'ultima ora per motivo. Un cluster
sano mostra i suoi conteggi e nulla altro.

## Creare un service account di sola lettura per DumbMonit

1. Crea un service account, una ClusterRole che possa solo get e list su nodi, pod, workload, volume claim ed eventi, e collegali insieme. Il ruolo non legge alcun Secret o ConfigMap. Su un server k3s, esegui ogni comando come sudo k3s kubectl.

    ```
    kubectl create serviceaccount dumbmonit -n kube-system
    kubectl create clusterrole dumbmonit-read --verb=get,list --resource=nodes,pods,persistentvolumeclaims,events,deployments.apps,statefulsets.apps,daemonsets.apps
    kubectl create clusterrolebinding dumbmonit-read --clusterrole=dumbmonit-read --serviceaccount=kube-system:dumbmonit
    ```

2. Dagli un token che non scade: un Secret di tipo service-account-token, che Kubernetes compila. Poi stampa il token, e la CA del cluster in base64.

    ```
    echo '{"apiVersion":"v1","kind":"Secret","metadata":{"name":"dumbmonit-token","namespace":"kube-system","annotations":{"kubernetes.io/service-account.name":"dumbmonit"}},"type":"kubernetes.io/service-account-token"}' | kubectl apply -f -
    kubectl -n kube-system get secret dumbmonit-token -o jsonpath='{.data.token}' | base64 -d
    kubectl -n kube-system get secret dumbmonit-token -o jsonpath='{.data.ca\.crt}'
    ```

3. In DumbMonit, inserisci l'indirizzo dell'API server, per esempio "https://k3s.lan:6443". Incolla il token come Service account token e la CA base64 come Cluster CA certificate: solo quella CA viene allora fidata. Un API server riporta l'intero cluster.

!!! warning
    Il certificato dell'API server elenca solo i propri nomi e indirizzi: un indirizzo che non elenca fallisce la verifica. Usa uno che elenca (un indirizzo IP di nodo, oppure un nome aggiunto con --tls-san su k3s), oppure spunta Accept an unverifiable certificate.

### La stessa cosa come manifest

Per GitOps, oppure per rivedere esattamente cosa viene concesso, applica
questo manifest invece dei tre comandi `kubectl create` e del Secret del
passaggio 2 (`kubectl apply -f dumbmonit.yaml`), poi stampa il token e
la CA come nel passaggio 2:

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

Per verificare il ruolo prima di aggiungere il dispositivo, chiedi
all'API server cosa può fare l'account; il primo comando deve
rispondere `yes`, il secondo `no`:

```
kubectl auth can-i list pods --as=system:serviceaccount:kube-system:dumbmonit
kubectl auth can-i list secrets --as=system:serviceaccount:kube-system:dumbmonit
```

Un token creato con `kubectl create token dumbmonit -n kube-system`
funziona anch'esso, ma scade (dopo un'ora per impostazione predefinita):
il dispositivo allora fallisce con "The API server refused the token
(401)". Il Secret sopra è la forma a lunga durata che Kubernetes
documenta per questo uso. Per revocare l'accesso di DumbMonit, elimina il
Secret, oppure il service account.

## Credenziali

| Credenziale | Campi |
|---|---|
| Service account token | Il token stampato dal primo comando `kubectl get secret`, inviato come `Authorization: Bearer`. |

Indirizzo: l'API server come URL (`https://k3s.lan:6443`), un nome host
o IP (`192.0.2.10`, porta 6443 allora), oppure `host:porta`. Dietro un
load balancer o un k3s ad alta disponibilità, dai l'indirizzo che serve
l'API, quello nel tuo kubeconfig.

Opzioni:

- **Port** (6443) quando l'indirizzo non ne fornisce una.
- **Cluster CA certificate**: il valore base64 di `ca.crt`, come lo
  stampa l'ultimo comando del passaggio 2; funziona anche un certificato
  PEM, anche incollato su una riga. Con esso, DumbMonit fida solo quella
  CA.
- **Accept an unverifiable certificate**: salta ogni verifica, per una
  rete di cui ti fidi.
- **Timeout per request** (10 s).
- **Ignored namespaces**: pod, workload, claim ed eventi di questi
  namespace vengono esclusi, per esempio un namespace CI pieno di pod a
  breve durata. Gli eventi dei nodi vengono sempre mantenuti.

## Risoluzione dei problemi

"The certificate of the API server could not be verified": oppure la CA
manca (incollala, come nel passaggio 2), oppure l'indirizzo non è uno dei
nomi nel certificato del server. k3s elenca `kubernetes`, `localhost`,
il nome host del nodo e i suoi indirizzi IP; aggiungi un altro nome con
`--tls-san` (in `/etc/rancher/k3s/config.yaml`: `tls-san: [k3s.lan]`) e
riavvia k3s.

"The service account may not read … (403)": la ClusterRole o il suo
binding manca o è incompleto. Il messaggio nomina la risorsa che l'API
server ha rifiutato; applica di nuovo il manifest sopra.
