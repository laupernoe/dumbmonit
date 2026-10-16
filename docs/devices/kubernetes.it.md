# Kubernetes / k3s

Un cluster attraverso il suo server API: nodi che non sono Pronti o sotto pressione, pod che crashano in loop o bloccati in Pending, Deployment, StatefulSet e DaemonSet con replica mancanti, claim di volume che non si legano mai, e gli eventi di avvertenza dell'ultima ora.

DumbMonit legge l'API con il token di un account di servizio dedicato legato a una ClusterRole di sola lettura: `get` e `list` su nodi, pod, carichi di lavoro, claim di volume persistente ed eventi, niente altro. Non scrive mai, e il ruolo non dà accesso a Secret o ConfigMap. Un monitor copre l'intero cluster, qualunque sia la distribuzione: k3s, kubeadm, k0s, MicroK8s, Talos, o un cluster gestito il cui API DumbMonit può raggiungere.

Ogni minuto DumbMonit legge otto collezioni, in parallelo: la versione, i nodi, i pod, i Deployment, StatefulSet e DaemonSet, i claim di volume persistente, e gli eventi di tipo `Warning` (filtrati dal server API). Le lunghe liste vengono lette in pagine di 500.

## Cosa controlla

Tutte le metriche sono prefissate `dumbmonit_k8s_`.

| Metrica | Cosa | Etichette |
|---|---|---|
| `version_info` | valore 1 | `version` (`1.33.4+k3s1`) |
| `nodes`, `nodes_ready` | nodi, e quelli la cui condizione `Ready` è vera | |
| `node_ready` | 1 quando il nodo è Pronto | `node` |
| `node_pressure` | 1 quando il nodo segnala quella pressione; solo le condizioni che il nodo dichiara | `node`, `condition`: `memory`, `disk`, `pid`, `network` |
| `node_unschedulable` | 1 quando il nodo è cordoned | `node` |
| `node_info` | valore 1 | `node`, `kubelet_version` |
| `pods`, `pods_running`, `pods_pending`, `pods_failed`, `pods_crashlooping`, `pods_not_ready` | conteggi di pod; i pod completati (`Succeeded`) sono lasciati fuori | |
| `pod_ready`, `pod_crashlooping`, `pod_pending` | 1 o 0 per pod | `namespace`, `pod`, `workload` |
| `pod_restarts` | riavvi di contenitore del pod, un contatore | `namespace`, `pod`, `workload` |
| `pod_blocked` | 1 mentre un contenitore attende per un motivo diverso dall'avvio: `ImagePullBackOff`, `CreateContainerConfigError`… | `namespace`, `pod`, `workload`, `reason` |
| `workload_desired`, `workload_ready`, `workload_unavailable` | replica chieste, pronte, e mancanti; per un DaemonSet, nodi | `kind` (`Deployment`, `StatefulSet`, `DaemonSet`), `namespace`, `workload` |
| `pvc_pending` | 1 quando il claim è Pending **e** un pod lo ha bisogno o un evento di avvertenza lo riguarda | `namespace`, `pvc` |
| `pvc_bound` | 1 quando il claim è Legato | `namespace`, `pvc` |
| `pvcs`, `pvcs_pending` | claim, e quelli bloccati come sopra | |
| `warning_events` | eventi di avvertenza visti nell'ultima ora | |
| `warning_events_by_reason` | lo stesso per motivo (`BackOff`, `FailedScheduling`, `ProvisioningFailed`…), venti motivi al massimo | `reason` |

`workload` è il controller del pod: il Deployment dietro il suo ReplicaSet (`api` per `api-7c9696d669-8tpp6`), lo StatefulSet, il DaemonSet, o vuoto per un pod semplice. I pod sono descritti uno per uno fino a 500, i problemi per primi; i conteggi sempre coprono ogni pod.

Un claim che è meramente `Pending` non è un guasto: una classe di storage `WaitForFirstConsumer`, quella che k3s spedisce (`local-path`), lascia un claim in sospeso fino a quando il primo pod non lo usa. Conta come bloccato solo quando un pod lo chiede o quando un'avvertenza come `ProvisioningFailed` lo riguarda.

Le [regole integrate](../alerting/rules.md#kubernetes) che si applicano:

- **Nodo Kubernetes non pronto**: la condizione `Ready` di un nodo è falsa o sconosciuta per cinque minuti (Avvertenza).
- **Nodo Kubernetes sotto pressione**: pressione di memoria, disco o PID per cinque minuti (Avviso).
- **Pod Kubernetes in crash loop**: un contenitore in `CrashLoopBackOff` per dieci minuti (Avvertenza). Alcuni kubelet (k3s 1.37) mostrano il contenitore come `terminated` con motivo `Error` tra due tentativi e mai `CrashLoopBackOff`: un contenitore che è riavviato almeno due volte e il cui ultimo arresto, entro dieci minuti, è terminato con un codice di uscita non zero conta come in crash loop anche. Tra due tentativi il contenitore si riavvia e lo stato scompare per un momento; la regola legge il massimo su dieci minuti così che non inizi da capo ogni volta.
- **Pod Kubernetes bloccato in sospeso**: un pod Pending per quindici minuti (Avviso).
- **Pod Kubernetes riavvio**: più di cinque riavvi in un'ora, per un pod che non è già in crash loop (Avviso).
- **Carico di lavoro Kubernetes con replica mancanti**: un Deployment, StatefulSet o DaemonSet con meno replica disponibili di quante chiede, per dieci minuti (Avviso, escalation dopo un'ora). Un aggiornamento graduale rimuove una replica mentre ne avvia un'altra; i dieci minuti la coprono.
- **Claim di volume Kubernetes bloccato**: un claim bloccato per quindici minuti (Avviso).

Più Monitor irraggiungibile, quando il server API smette di rispondere.

## La pagina del monitor

Il pannello del Cluster dice cosa è sbagliato in una frase, quindi elenca i nodi (Pronto o no, pressione, cordoned, versione kubelet), i pod che hanno bisogno di attenzione (crash looping, in sospeso, bloccato, non pronto, con i loro riavvi), i carichi di lavoro con replica mancanti, i claim di volume bloccati, e gli eventi di avvertenza dell'ultima ora per motivo. Un cluster sano mostra i suoi conteggi e niente altro.

## Crea un account di servizio di sola lettura per DumbMonit

1. Crea un account di servizio, una ClusterRole che può solo ottenere e elencare nodi, pod, carichi di lavoro, claim di volume e eventi, e legali insieme. Il ruolo non legge nessuno Secret e nessuno ConfigMap. Su un server k3s, esegui ogni comando come sudo k3s kubectl.

    ```
    kubectl create serviceaccount dumbmonit -n kube-system
    kubectl create clusterrole dumbmonit-read --verb=get,list --resource=nodes,pods,persistentvolumeclaims,events,deployments.apps,statefulsets.apps,daemonsets.apps
    kubectl create clusterrolebinding dumbmonit-read --clusterrole=dumbmonit-read --serviceaccount=kube-system:dumbmonit
    ```

2. Dagli un token che non scade: un Secret di tipo service-account-token, che Kubernetes riempie. Poi stampa il token, e la CA del cluster in base64.

    ```
    echo '{"apiVersion":"v1","kind":"Secret","metadata":{"name":"dumbmonit-token","namespace":"kube-system","annotations":{"kubernetes.io/service-account.name":"dumbmonit"}},"type":"kubernetes.io/service-account-token"}' | kubectl apply -f -
    kubectl -n kube-system get secret dumbmonit-token -o jsonpath='{.data.token}' | base64 -d
    kubectl -n kube-system get secret dumbmonit-token -o jsonpath='{.data.ca\.crt}'
    ```

3. In DumbMonit, inserisci l'indirizzo del server API, per esempio "https://k3s.lan:6443". Incolla il token come token dell'account di servizio e la CA in base64 come certificato della CA del cluster: solo quella CA è allora attendibile. Un server API segnala l'intero cluster.

!!! warning
    Il certificato del server API elenca solo i suoi nomi e indirizzi: un indirizzo che non elenca fallisce la verifica. Usane uno che elenca (un indirizzo IP del nodo, o un nome aggiunto con --tls-san su k3s), o spunta Accetta un certificato non verificabile.

### La stessa cosa come manifest

Per GitOps, o per rivedere esattamente cosa è concesso, applica questo manifest invece dei tre comandi `kubectl create` e del Secret del passo 2 (`kubectl apply -f dumbmonit.yaml`), poi stampa il token e la CA come nel passo 2:

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

Per controllare il ruolo prima di aggiungere il monitor, chiedi al server API cosa l'account può fare; il primo comando deve rispondere `yes`, il secondo `no`:

```
kubectl auth can-i list pods --as=system:serviceaccount:kube-system:dumbmonit
kubectl auth can-i list secrets --as=system:serviceaccount:kube-system:dumbmonit
```

Un token creato con `kubectl create token dumbmonit -n kube-system` funziona anche, ma scade (dopo un'ora per impostazione predefinita): il monitor allora fallisce con "Il server API ha rifiutato il token (401)". Il Secret sopra è la forma di lunga durata che Kubernetes documenta per questo uso. Per revocare l'accesso di DumbMonit, cancella il Secret, o l'account di servizio.

## Credenziali

| Credenziale | Campi |
|---|---|
| Token dell'account di servizio | Il token stampato dal primo comando `kubectl get secret`, inviato come `Authorization: Bearer`. |

Indirizzo: il server API come URL (`https://k3s.lan:6443`), un nome host o IP (`192.0.2.10`, porta 6443 allora), o `host:port`. Dietro un load balancer o k3s altamente disponibile, dai l'indirizzo che serve l'API, quello nel tuo kubeconfig.

Opzioni:

- **Porta** (6443) quando l'indirizzo non fornisce nessuno.
- **Certificato della CA del cluster**: il valore base64 di `ca.crt`, come il comando dell'ultimo passo 2 lo stampa; un certificato PEM funziona anche, anche incollato su una linea. Con esso, DumbMonit si fida di quella CA e nulla altro.
- **Accetta un certificato non verificabile**: salta ogni controllo, per una rete che fidi.
- **Timeout per richiesta** (10 s).
- **Spazi dei nomi ignorati**: pod, carichi di lavoro, claim ed eventi di questi spazi dei nomi sono lasciati fuori, per esempio uno spazio dei nomi di CI pieno di pod di breve durata. Gli eventi dei nodi sono sempre mantenuti.

## Risoluzione dei problemi

"Il certificato del server API non potrebbe essere verificato": o la CA è mancante (incollala, come il passo 2), o l'indirizzo non è uno dei nomi nel certificato del server. k3s elenca `kubernetes`, `localhost`, il nome host del nodo e i suoi indirizzi IP; aggiungi un altro nome con `--tls-san` (in `/etc/rancher/k3s/config.yaml`: `tls-san: [k3s.lan]`) e riavvia k3s.

"L'account di servizio potrebbe non leggere … (403)": la ClusterRole o la sua vincolazione è mancante o incompleta. Il messaggio nomina la risorsa che il server API ha rifiutato; applica il manifest sopra di nuovo.
