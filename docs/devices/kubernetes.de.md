# Kubernetes / k3s

Ein Cluster über seinen API-Server: nicht bereite oder unter Druck
stehende Knoten, Pods, die crash-loopen oder im Pending festsitzen,
Deployments, StatefulSets und DaemonSets mit fehlenden Replikas,
Volume Claims, die nie gebunden werden, und die Warning-Events der
letzten Stunde.

DumbMonit liest die API mit dem Token eines dedizierten Service
Accounts, gebunden an eine nur-lesende ClusterRole: `get` und `list`
auf Knoten, Pods, Workloads, Persistent Volume Claims und Events,
nichts sonst. Es schreibt nie, und die Rolle gibt keinen Zugriff auf
Secrets oder ConfigMaps. Ein Gerät deckt den gesamten Cluster ab,
welche Distribution auch immer: k3s, kubeadm, k0s, MicroK8s, Talos,
oder ein verwalteter Cluster, dessen API DumbMonit erreichen kann.

Jede Minute liest DumbMonit acht Collections, parallel: die Version,
die Knoten, die Pods, die Deployments, StatefulSets und DaemonSets,
die Persistent Volume Claims, und die Events vom Typ `Warning`
(vom API-Server gefiltert). Lange Listen werden in Seiten zu 500
gelesen.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_k8s_`.

| Metrik | Was | Labels |
|---|---|---|
| `version_info` | Wert 1 | `version` (`1.33.4+k3s1`) |
| `nodes`, `nodes_ready` | Knoten, und die mit wahrer `Ready`-Bedingung | |
| `node_ready` | 1, wenn der Knoten Ready ist | `node` |
| `node_pressure` | 1, wenn der Knoten diesen Druck meldet; nur die vom Knoten angegebenen Bedingungen | `node`, `condition`: `memory`, `disk`, `pid`, `network` |
| `node_unschedulable` | 1, wenn der Knoten cordoned ist | `node` |
| `node_info` | Wert 1 | `node`, `kubelet_version` |
| `pods`, `pods_running`, `pods_pending`, `pods_failed`, `pods_crashlooping`, `pods_not_ready` | Pod-Zählungen; abgeschlossene Pods (`Succeeded`) werden ausgelassen | |
| `pod_ready`, `pod_crashlooping`, `pod_pending` | 1 oder 0 pro Pod | `namespace`, `pod`, `workload` |
| `pod_restarts` | Container-Neustarts des Pods, ein Zähler | `namespace`, `pod`, `workload` |
| `pod_blocked` | 1, während ein Container aus einem anderen Grund als dem Starten wartet: `ImagePullBackOff`, `CreateContainerConfigError`… | `namespace`, `pod`, `workload`, `reason` |
| `workload_desired`, `workload_ready`, `workload_unavailable` | geforderte, bereite und fehlende Replikas; für ein DaemonSet, Knoten | `kind` (`Deployment`, `StatefulSet`, `DaemonSet`), `namespace`, `workload` |
| `pvc_pending` | 1, wenn der Claim Pending ist **und** ein Pod ihn braucht oder ein Warning-Event ihn betrifft | `namespace`, `pvc` |
| `pvc_bound` | 1, wenn der Claim Bound ist | `namespace`, `pvc` |
| `pvcs`, `pvcs_pending` | Claims, und die wie oben festsitzenden | |
| `warning_events` | in der letzten Stunde gesehene Warning-Events | |
| `warning_events_by_reason` | dasselbe nach Grund (`BackOff`, `FailedScheduling`, `ProvisioningFailed`…), höchstens zwanzig Gründe | `reason` |

`workload` ist der Controller des Pods: das Deployment hinter seinem
ReplicaSet (`api` für `api-7c9696d669-8tpp6`), das StatefulSet, das
DaemonSet, oder leer für einen blanken Pod. Pods werden einzeln bis zu
500 beschrieben, Probleme zuerst; Zählungen decken immer jeden Pod ab.

Ein Claim, der lediglich `Pending` ist, ist kein Fehler: Eine
`WaitForFirstConsumer`-Storage-Class, die k3s mitliefert
(`local-path`), lässt einen Claim pending, bis der erste Pod ihn
nutzt. Er zählt nur als festsitzend, wenn ein Pod ihn verlangt oder
wenn eine Warnung wie `ProvisioningFailed` ihn betrifft.

Die geltenden [eingebauten Regeln](../alerting/rules.md#kubernetes):

- **Kubernetes node not ready**: Die `Ready`-Bedingung eines Knotens
  ist seit fünf Minuten falsch oder unbekannt (Warning).
- **Kubernetes node under pressure**: Speicher-, Platten- oder
  PID-Druck seit fünf Minuten (Advisory).
- **Kubernetes pod crash looping**: ein Container in
  `CrashLoopBackOff` seit zehn Minuten (Warning). Zwischen zwei
  Versuchen startet der Container neu, und der Zustand verschwindet
  kurz; die Regel liest das Maximum über zehn Minuten, damit sie nicht
  jedes Mal neu beginnt.
- **Kubernetes pod stuck pending**: ein Pod Pending seit fünfzehn
  Minuten (Advisory).
- **Kubernetes pod restarting**: mehr als fünf Neustarts in einer
  Stunde, für einen Pod, der noch nicht crash-loopt (Advisory).
- **Kubernetes workload missing replicas**: ein Deployment,
  StatefulSet oder DaemonSet mit weniger verfügbaren Replikas als
  gefordert, seit zehn Minuten (Advisory, eskaliert nach einer
  Stunde). Ein Rolling Update entfernt eine Replika, während es eine
  andere startet; die zehn Minuten decken das ab.
- **Kubernetes volume claim pending**: ein festsitzender Claim seit
  fünfzehn Minuten (Advisory).

Plus Device unreachable, wenn der API-Server aufhört zu antworten.

## Die Geräteseite

Das Cluster-Panel sagt in einem Satz, was nicht stimmt, listet dann die
Knoten (Ready oder nicht, Druck, cordoned, Kubelet-Version), die
Pods, die Aufmerksamkeit brauchen (crash-loopend, pending, blockiert,
nicht ready, mit ihren Neustarts), die Workloads mit fehlenden
Replikas, die festsitzenden Volume Claims, und die Warning-Events der
letzten Stunde nach Grund. Ein gesunder Cluster zeigt seine Zählungen
und nichts sonst.

## Einen nur-lesenden Service Account für DumbMonit erstellen

1. Erstellen Sie einen Service Account, eine ClusterRole, die nur
   Knoten, Pods, Workloads, Volume Claims und Events get und list
   darf, und binden Sie sie zusammen. Die Rolle liest kein Secret und
   kein ConfigMap. Auf einem k3s-Server führen Sie jeden Befehl mit
   sudo k3s kubectl aus.

    ```
    kubectl create serviceaccount dumbmonit -n kube-system
    kubectl create clusterrole dumbmonit-read --verb=get,list --resource=nodes,pods,persistentvolumeclaims,events,deployments.apps,statefulsets.apps,daemonsets.apps
    kubectl create clusterrolebinding dumbmonit-read --clusterrole=dumbmonit-read --serviceaccount=kube-system:dumbmonit
    ```

2. Geben Sie ihm ein Token, das nicht abläuft: ein Secret vom Typ
   service-account-token, das Kubernetes befüllt. Geben Sie dann das
   Token und die Cluster-CA in base64 aus.

    ```
    echo '{"apiVersion":"v1","kind":"Secret","metadata":{"name":"dumbmonit-token","namespace":"kube-system","annotations":{"kubernetes.io/service-account.name":"dumbmonit"}},"type":"kubernetes.io/service-account-token"}' | kubectl apply -f -
    kubectl -n kube-system get secret dumbmonit-token -o jsonpath='{.data.token}' | base64 -d
    kubectl -n kube-system get secret dumbmonit-token -o jsonpath='{.data.ca\.crt}'
    ```

3. Geben Sie in DumbMonit die Adresse des API-Servers ein, zum
   Beispiel "https://k3s.lan:6443". Fügen Sie das Token als Service
   account token und die base64-CA als Cluster CA certificate ein:
   Nur dieser CA wird dann vertraut. Ein API-Server meldet den
   gesamten Cluster.

!!! warning
    Das Zertifikat des API-Servers listet nur seine eigenen Namen und
    Adressen: Eine Adresse, die es nicht listet, schlägt bei der
    Verifizierung fehl. Nutzen Sie eine, die es listet (eine
    Knoten-IP-Adresse, oder einen mit --tls-san auf k3s hinzugefügten
    Namen), oder kreuzen Sie Accept an unverifiable certificate an.

### Dasselbe als Manifest

Für GitOps, oder um genau zu prüfen, was gewährt wird, wenden Sie
dieses Manifest an, statt die drei `kubectl create`-Befehle und das
Secret aus Schritt 2 (`kubectl apply -f dumbmonit.yaml`), geben Sie
dann das Token und die CA wie in Schritt 2 aus:

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

Um die Rolle vor dem Hinzufügen des Geräts zu prüfen, fragen Sie den
API-Server, was das Konto darf; der erste Befehl muss `yes`
antworten, der zweite `no`:

```
kubectl auth can-i list pods --as=system:serviceaccount:kube-system:dumbmonit
kubectl auth can-i list secrets --as=system:serviceaccount:kube-system:dumbmonit
```

Ein mit `kubectl create token dumbmonit -n kube-system` erstelltes
Token funktioniert auch, läuft aber ab (standardmäßig nach einer
Stunde): Das Gerät schlägt dann mit "The API server refused the token
(401)" fehl. Das Secret oben ist die von Kubernetes für diesen Zweck
dokumentierte langlebige Form. Um DumbMonits Zugriff zu widerrufen,
löschen Sie das Secret, oder den Service Account.

## Credentials

| Credential | Felder |
|---|---|
| Service-Account-Token | Das vom ersten `kubectl get secret`-Befehl ausgegebene Token, als `Authorization: Bearer` gesendet. |

Adresse: der API-Server als URL (`https://k3s.lan:6443`), ein
Hostname oder eine IP (`192.0.2.10`, dann Port 6443), oder
`host:port`. Hinter einem Load Balancer oder einem hochverfügbaren
k3s geben Sie die Adresse an, die die API bedient, die in Ihrer
kubeconfig.

Optionen:

- **Port** (6443), wenn die Adresse keinen angibt.
- **Cluster CA certificate**: der base64-Wert von `ca.crt`, wie der
  letzte Befehl von Schritt 2 ihn ausgibt; ein PEM-Zertifikat
  funktioniert auch, selbst in einer Zeile eingefügt. Damit vertraut
  DumbMonit dieser CA und keiner anderen.
- **Accept an unverifiable certificate**: überspringt jede Prüfung,
  für ein Netzwerk, dem Sie vertrauen.
- **Timeout pro Request** (10 s).
- **Ignored namespaces**: Pods, Workloads, Claims und Events dieser
  Namespaces werden ausgelassen, zum Beispiel ein CI-Namespace voller
  kurzlebiger Pods. Knoten-Events werden immer behalten.

## Fehlerbehebung

"The certificate of the API server could not be verified": entweder
fehlt die CA (fügen Sie sie wie in Schritt 2 ein), oder die Adresse
ist keiner der Namen im Zertifikat des Servers. k3s listet
`kubernetes`, `localhost`, den Hostnamen des Knotens und seine
IP-Adressen; fügen Sie mit `--tls-san` einen weiteren Namen hinzu (in
`/etc/rancher/k3s/config.yaml`: `tls-san: [k3s.lan]`) und starten Sie
k3s neu.

"The service account may not read … (403)": Die ClusterRole oder ihre
Bindung fehlt oder ist unvollständig. Die Meldung benennt die vom
API-Server abgelehnte Ressource; wenden Sie das obige Manifest erneut
an.
