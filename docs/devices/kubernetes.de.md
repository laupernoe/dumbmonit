# Kubernetes / k3s {#kubernetes-k3s}

Ein Cluster über seinen API-Server: Knoten, die nicht Ready sind oder unter Druck stehen, Pods
in Crash-Schleife oder festhängend im Zustand Pending, Deployments, StatefulSets und DaemonSets
mit fehlenden Replikas, Volume Claims, die nie gebunden werden, und die Warnungsereignisse der
letzten Stunde.

DumbMonit liest die API mit dem Token eines dedizierten Service Accounts, der an eine
schreibgeschützte ClusterRole gebunden ist: `get` und `list` auf Knoten, Pods, Workloads,
Persistent Volume Claims und Ereignisse, sonst nichts. Es schreibt nie, und die Rolle gibt keinen
Zugriff auf Secrets oder ConfigMaps. Ein Monitor deckt den ganzen Cluster ab, gleich welche
Distribution: k3s, kubeadm, k0s, MicroK8s, Talos oder ein verwalteter Cluster, dessen API
DumbMonit erreichen kann.

Jede Minute liest DumbMonit parallel acht Sammlungen: die Version, die Knoten, die Pods, die
Deployments, StatefulSets und DaemonSets, die Persistent Volume Claims und die Ereignisse vom
Typ `Warning` (vom API-Server gefiltert). Lange Listen werden in Seiten zu 500 gelesen.

## Was überwacht wird {#what-it-watches}

Alle Metriken haben das Präfix `dumbmonit_k8s_`.

| Metrik | Was | Labels |
|---|---|---|
| `version_info` | Wert 1 | `version` (`1.33.4+k3s1`) |
| `nodes`, `nodes_ready` | Knoten und solche, deren Bedingung `Ready` wahr ist | |
| `node_ready` | 1, wenn der Knoten Ready ist | `node` |
| `node_pressure` | 1, wenn der Knoten diesen Druck meldet; nur die Bedingungen, die der Knoten deklariert | `node`, `condition`: `memory`, `disk`, `pid`, `network` |
| `node_unschedulable` | 1, wenn der Knoten gesperrt (cordoned) ist | `node` |
| `node_info` | Wert 1 | `node`, `kubelet_version` |
| `pods`, `pods_running`, `pods_pending`, `pods_failed`, `pods_crashlooping`, `pods_not_ready` | Pod-Zahlen; abgeschlossene Pods (`Succeeded`) bleiben außen vor | |
| `pod_ready`, `pod_crashlooping`, `pod_pending` | 1 oder 0 pro Pod | `namespace`, `pod`, `workload` |
| `pod_restarts` | Container-Neustarts des Pods, ein Zähler | `namespace`, `pod`, `workload` |
| `pod_blocked` | 1, solange ein Container aus einem anderen Grund als dem Start wartet: `ImagePullBackOff`, `CreateContainerConfigError` … | `namespace`, `pod`, `workload`, `reason` |
| `workload_desired`, `workload_ready`, `workload_unavailable` | angeforderte, bereite und fehlende Replikas; bei einem DaemonSet Knoten | `kind` (`Deployment`, `StatefulSet`, `DaemonSet`), `namespace`, `workload` |
| `pvc_pending` | 1, wenn der Claim Pending ist **und** ein Pod ihn braucht oder ein Warnungsereignis ihn betrifft | `namespace`, `pvc` |
| `pvc_bound` | 1, wenn der Claim Bound ist | `namespace`, `pvc` |
| `pvcs`, `pvcs_pending` | Claims und die wie oben festhängenden | |
| `warning_events` | in der letzten Stunde gesehene Warnungsereignisse | |
| `warning_events_by_reason` | dasselbe nach Grund (`BackOff`, `FailedScheduling`, `ProvisioningFailed` …), höchstens zwanzig Gründe | `reason` |

`workload` ist der Controller des Pods: das Deployment hinter seinem ReplicaSet (`api` für
`api-7c9696d669-8tpp6`), das StatefulSet, das DaemonSet oder leer bei einem nackten Pod. Pods
werden bis zu 500 einzeln beschrieben, Probleme zuerst; die Zahlen decken immer jeden Pod ab.

Ein Claim, der bloß `Pending` ist, ist kein Fehler: Eine Storage-Klasse mit `WaitForFirstConsumer`,
wie sie k3s mitliefert (`local-path`), lässt einen Claim ausstehend, bis der erste Pod ihn
verwendet. Er zählt erst als festhängend, wenn ein Pod ihn anfordert oder eine Warnung wie
`ProvisioningFailed` ihn betrifft.

Die zutreffenden [integrierten Regeln](../alerting/rules.md#kubernetes):

- **Kubernetes-Knoten nicht bereit**: Die Bedingung `Ready` eines Knotens ist fünf Minuten lang
  falsch oder unbekannt (Warnung).
- **Kubernetes-Knoten unter Druck**: Speicher-, Platten- oder PID-Druck seit fünf Minuten
  (Hinweis).
- **Kubernetes-Pod in Crash-Schleife**: ein Container zehn Minuten lang in `CrashLoopBackOff`
  (Warnung). Manche Kubelets (k3s 1.37) zeigen den Container zwischen zwei Versuchen als
  `terminated` mit Grund `Error` und nie `CrashLoopBackOff`: Ein Container, der mindestens
  zweimal neu gestartet wurde und dessen letzter Stopp innerhalb von zehn Minuten mit einem
  Exit-Code ungleich null endete, zählt ebenfalls als Crash-Schleife. Zwischen zwei Versuchen
  startet der Container neu und der Zustand verschwindet kurz; die Regel liest das Maximum über
  zehn Minuten, damit sie nicht jedes Mal von vorn beginnt.
- **Kubernetes-Pod hängt in Pending**: ein Pod fünfzehn Minuten lang Pending (Hinweis).
- **Kubernetes-Pod startet immer wieder neu**: mehr als fünf Neustarts in einer Stunde bei einem
  Pod, der nicht bereits in Crash-Schleife ist (Hinweis).
- **Kubernetes-Workload mit fehlenden Replikas**: ein Deployment, StatefulSet oder DaemonSet mit
  weniger verfügbaren Replikas als angefordert, zehn Minuten lang (Hinweis, eskaliert nach einer
  Stunde). Ein Rolling Update entfernt eine Replika, während es eine andere startet; die zehn
  Minuten decken das ab.
- **Kubernetes-Volume-Claim ausstehend**: ein festhängender Claim fünfzehn Minuten lang (Hinweis).

Dazu Monitor nicht erreichbar, wenn der API-Server nicht mehr antwortet.

## Die Monitorseite {#the-device-page}

Das Cluster-Panel nennt in einem Satz, was nicht stimmt, und listet dann die Knoten (Ready oder
nicht, Druck, gesperrt, Kubelet-Version), die Pods, die Aufmerksamkeit brauchen (in
Crash-Schleife, Pending, blockiert, nicht bereit, mit ihren Neustarts), die Workloads mit
fehlenden Replikas, die festhängenden Volume Claims und die Warnungsereignisse der letzten
Stunde nach Grund. Ein gesunder Cluster zeigt seine Zahlen und sonst nichts.

## Einen schreibgeschützten Service Account für DumbMonit erstellen {#create-a-read-only-service-account-for-dumbmonit}

1. Erstellen Sie einen Service Account, eine ClusterRole, die nur Knoten, Pods, Workloads, Volume Claims und Ereignisse abrufen und auflisten kann, und binden Sie beide aneinander. Die Rolle liest kein Secret und keine ConfigMap. Führen Sie auf einem k3s-Server jeden Befehl als sudo k3s kubectl aus.

    ```
    kubectl create serviceaccount dumbmonit -n kube-system
    kubectl create clusterrole dumbmonit-read --verb=get,list --resource=nodes,pods,persistentvolumeclaims,events,deployments.apps,statefulsets.apps,daemonsets.apps
    kubectl create clusterrolebinding dumbmonit-read --clusterrole=dumbmonit-read --serviceaccount=kube-system:dumbmonit
    ```

2. Geben Sie ihm ein Token, das nicht abläuft: ein Secret vom Typ service-account-token, das Kubernetes befüllt. Lassen Sie sich dann das Token und die Cluster-CA in Base64 ausgeben.

    ```
    echo '{"apiVersion":"v1","kind":"Secret","metadata":{"name":"dumbmonit-token","namespace":"kube-system","annotations":{"kubernetes.io/service-account.name":"dumbmonit"}},"type":"kubernetes.io/service-account-token"}' | kubectl apply -f -
    kubectl -n kube-system get secret dumbmonit-token -o jsonpath='{.data.token}' | base64 -d
    kubectl -n kube-system get secret dumbmonit-token -o jsonpath='{.data.ca\.crt}'
    ```

3. Geben Sie in DumbMonit die Adresse des API-Servers ein, zum Beispiel „https://k3s.lan:6443“. Fügen Sie das Token als Service-Account-Token und die Base64-CA als Cluster-CA-Zertifikat ein: Nur diese CA wird dann vertraut. Ein API-Server meldet den ganzen Cluster.

!!! warning
    Das Zertifikat des API-Servers listet nur seine eigenen Namen und Adressen: Eine nicht aufgeführte Adresse scheitert an der Überprüfung. Verwenden Sie eine aufgeführte (eine Knoten-IP-Adresse oder einen unter k3s mit --tls-san hinzugefügten Namen) oder haken Sie Nicht prüfbares Zertifikat akzeptieren an.

### Dasselbe als Manifest {#the-same-thing-as-a-manifest}

Für GitOps oder um genau zu prüfen, was gewährt wird, wenden Sie dieses Manifest statt der drei
`kubectl create`-Befehle und des Secrets aus Schritt 2 an (`kubectl apply -f dumbmonit.yaml`)
und lassen Sie sich dann Token und CA wie in Schritt 2 ausgeben:

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

Um die Rolle vor dem Hinzufügen des Monitors zu prüfen, fragen Sie den API-Server, was das Konto
darf; der erste Befehl muss `yes` antworten, der zweite `no`:

```
kubectl auth can-i list pods --as=system:serviceaccount:kube-system:dumbmonit
kubectl auth can-i list secrets --as=system:serviceaccount:kube-system:dumbmonit
```

Ein mit `kubectl create token dumbmonit -n kube-system` erstelltes Token funktioniert ebenfalls,
läuft aber ab (standardmäßig nach einer Stunde): Der Monitor scheitert dann mit „Der API-Server hat
das Token abgelehnt (401)“. Das obige Secret ist die langlebige Form, die Kubernetes für diesen
Zweck dokumentiert. Um den Zugriff von DumbMonit zu widerrufen, löschen Sie das Secret oder den
Service Account.

## Zugangsdaten {#credentials}

| Zugangsdaten | Felder |
|---|---|
| Service-Account-Token | Das vom ersten Befehl `kubectl get secret` ausgegebene Token, gesendet als `Authorization: Bearer`. |

Adresse: der API-Server als URL (`https://k3s.lan:6443`), ein Hostname oder eine IP (`192.0.2.10`,
dann Port 6443) oder `host:port`. Geben Sie hinter einem Load Balancer oder einem hochverfügbaren
k3s die Adresse an, die die API ausliefert, die aus Ihrer kubeconfig.

Optionen:

- **Port** (6443), wenn die Adresse keinen angibt.
- **Cluster-CA-Zertifikat**: der Base64-Wert von `ca.crt`, wie ihn der letzte Befehl von
  Schritt 2 ausgibt; ein PEM-Zertifikat funktioniert ebenfalls, auch in einer Zeile eingefügt.
  Damit vertraut DumbMonit dieser CA und sonst keiner.
- **Nicht prüfbares Zertifikat akzeptieren**: überspringt jede Prüfung, für ein Netzwerk, dem
  Sie vertrauen.
- **Timeout pro Anfrage** (10 s).
- **Ignorierte Namespaces**: Pods, Workloads, Claims und Ereignisse dieser Namespaces werden
  ausgelassen, zum Beispiel ein CI-Namespace voller kurzlebiger Pods. Knotenereignisse bleiben
  immer erhalten.

## Fehlerbehebung {#troubleshooting}

„Das Zertifikat des API-Servers konnte nicht überprüft werden“: Entweder fehlt die CA (fügen Sie
sie ein, wie in Schritt 2), oder die Adresse gehört nicht zu den Namen im Zertifikat des Servers.
k3s listet `kubernetes`, `localhost`, den Hostnamen des Knotens und seine IP-Adressen; fügen Sie
einen weiteren Namen mit `--tls-san` hinzu (in `/etc/rancher/k3s/config.yaml`:
`tls-san: [k3s.lan]`) und starten Sie k3s neu.

„Das Service Account darf … nicht lesen (403)“: Die ClusterRole oder ihre Bindung fehlt oder ist
unvollständig. Die Meldung nennt die Ressource, die der API-Server abgelehnt hat; wenden Sie das
obige Manifest erneut an.
