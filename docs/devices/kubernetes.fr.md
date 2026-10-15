# Kubernetes / k3s

Un cluster via son serveur d'API : nœuds qui ne sont pas Ready ou sous
pression, pods en boucle de crash ou bloqués en Pending, Deployments,
StatefulSets et DaemonSets avec des répliques manquantes, demandes de
volume qui ne sont jamais liées, et les événements d'avertissement de la
dernière heure.

DumbMonit lit l'API avec le jeton d'un compte de service dédié lié à un
ClusterRole en lecture seule : `get` et `list` sur les nœuds, pods,
workloads, demandes de volume persistant et événements, rien d'autre. Il
n'écrit jamais, et le rôle ne donne aucun accès aux Secrets ni aux
ConfigMaps. Une seule surveillance couvre tout le cluster, quelle que soit la
distribution : k3s, kubeadm, k0s, MicroK8s, Talos, ou un cluster managé
dont l'API DumbMonit peut atteindre.

Chaque minute, DumbMonit lit huit collections, en parallèle : la version,
les nœuds, les pods, les Deployments, StatefulSets et DaemonSets, les
demandes de volume persistant, et les événements de type `Warning`
(filtrés par le serveur d'API). Les longues listes sont lues par pages de
500.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_k8s_`.

| Métrique | Quoi | Labels |
|---|---|---|
| `version_info` | valeur 1 | `version` (`1.33.4+k3s1`) |
| `nodes`, `nodes_ready` | nœuds, et ceux dont la condition `Ready` est vraie | |
| `node_ready` | 1 quand le nœud est Ready | `node` |
| `node_pressure` | 1 quand le nœud rapporte cette pression ; seulement les conditions que le nœud déclare | `node`, `condition` : `memory`, `disk`, `pid`, `network` |
| `node_unschedulable` | 1 quand le nœud est en cordon | `node` |
| `node_info` | valeur 1 | `node`, `kubelet_version` |
| `pods`, `pods_running`, `pods_pending`, `pods_failed`, `pods_crashlooping`, `pods_not_ready` | compteurs de pods ; les pods terminés (`Succeeded`) sont laissés de côté | |
| `pod_ready`, `pod_crashlooping`, `pod_pending` | 1 ou 0 par pod | `namespace`, `pod`, `workload` |
| `pod_restarts` | redémarrages de conteneur du pod, un compteur | `namespace`, `pod`, `workload` |
| `pod_blocked` | 1 tant qu'un conteneur attend pour une autre raison que démarrer : `ImagePullBackOff`, `CreateContainerConfigError`… | `namespace`, `pod`, `workload`, `reason` |
| `workload_desired`, `workload_ready`, `workload_unavailable` | répliques demandées, prêtes, et manquantes ; pour un DaemonSet, des nœuds | `kind` (`Deployment`, `StatefulSet`, `DaemonSet`), `namespace`, `workload` |
| `pvc_pending` | 1 quand la demande est Pending **et** qu'un pod en a besoin ou qu'un événement d'avertissement la concerne | `namespace`, `pvc` |
| `pvc_bound` | 1 quand la demande est Bound | `namespace`, `pvc` |
| `pvcs`, `pvcs_pending` | demandes, et celles bloquées comme ci-dessus | |
| `warning_events` | événements d'avertissement vus dans la dernière heure | |
| `warning_events_by_reason` | la même chose par raison (`BackOff`, `FailedScheduling`, `ProvisioningFailed`…), vingt raisons au plus | `reason` |

`workload` est le contrôleur du pod : le Deployment derrière son
ReplicaSet (`api` pour `api-7c9696d669-8tpp6`), le StatefulSet, le
DaemonSet, ou vide pour un pod nu. Les pods sont décrits un par un jusqu'à
500, les problèmes d'abord ; les compteurs couvrent toujours chaque pod.

Une demande simplement `Pending` n'est pas une panne : une classe de
stockage `WaitForFirstConsumer`, celle que k3s livre (`local-path`),
laisse une demande en attente jusqu'à ce que le premier pod l'utilise.
Elle ne compte comme bloquée que quand un pod en a besoin ou qu'un
avertissement comme `ProvisioningFailed` la concerne.

Les [règles intégrées](../alerting/rules.md#kubernetes) qui s'appliquent :

- **Nœud Kubernetes non prêt** : la condition `Ready` d'un nœud est fausse
  ou inconnue depuis cinq minutes (Avertissement).
- **Nœud Kubernetes sous pression** : pression mémoire, disque ou PID
  depuis cinq minutes (Avis).
- **Pod Kubernetes en boucle de crash** : un conteneur en
  `CrashLoopBackOff` depuis dix minutes (Avertissement). Entre deux
  tentatives le conteneur redémarre et l'état disparaît un instant ; la
  règle lit le maximum sur dix minutes pour ne pas repartir de zéro
  chaque fois.
- **Pod Kubernetes bloqué en attente** : un pod Pending depuis quinze
  minutes (Avis).
- **Pod Kubernetes redémarre** : plus de cinq redémarrages en une heure,
  pour un pod qui n'est pas déjà en boucle de crash (Avis).
- **Workload Kubernetes avec des répliques manquantes** : un Deployment,
  StatefulSet ou DaemonSet avec moins de répliques disponibles qu'il n'en
  demande, depuis dix minutes (Avis, escalade après une heure). Une mise à
  jour progressive retire une réplique pendant qu'elle en démarre une
  autre ; les dix minutes couvrent ça.
- **Demande de volume Kubernetes en attente** : une demande bloquée
  depuis quinze minutes (Avis).

Plus Surveillance inaccessible, quand le serveur d'API arrête de répondre.

## La page de la surveillance

Le panneau Cluster dit ce qui ne va pas en une phrase, puis liste les
nœuds (Ready ou non, pression, en cordon, version kubelet), les pods qui
ont besoin d'attention (boucle de crash, en attente, bloqués, non prêts,
avec leurs redémarrages), les workloads avec des répliques manquantes, les
demandes de volume bloquées, et les événements d'avertissement de la
dernière heure par raison. Un cluster en bonne santé montre ses compteurs
et rien d'autre.

## Créer un compte de service en lecture seule pour DumbMonit

1. Créez un compte de service, un ClusterRole qui ne peut que get et list les nœuds, pods, workloads, demandes de volume et événements, et liez-les ensemble. Le rôle ne lit aucun Secret ni ConfigMap. Sur un serveur k3s, lancez chaque commande avec sudo k3s kubectl.

    ```
    kubectl create serviceaccount dumbmonit -n kube-system
    kubectl create clusterrole dumbmonit-read --verb=get,list --resource=nodes,pods,persistentvolumeclaims,events,deployments.apps,statefulsets.apps,daemonsets.apps
    kubectl create clusterrolebinding dumbmonit-read --clusterrole=dumbmonit-read --serviceaccount=kube-system:dumbmonit
    ```

2. Donnez-lui un jeton qui n'expire pas : un Secret de type service-account-token, que Kubernetes remplit. Puis affichez le jeton, et la CA du cluster en base64.

    ```
    echo '{"apiVersion":"v1","kind":"Secret","metadata":{"name":"dumbmonit-token","namespace":"kube-system","annotations":{"kubernetes.io/service-account.name":"dumbmonit"}},"type":"kubernetes.io/service-account-token"}' | kubectl apply -f -
    kubectl -n kube-system get secret dumbmonit-token -o jsonpath='{.data.token}' | base64 -d
    kubectl -n kube-system get secret dumbmonit-token -o jsonpath='{.data.ca\.crt}'
    ```

3. Dans DumbMonit, saisissez l'adresse du serveur d'API, par exemple « https://k3s.lan:6443 ». Collez le jeton comme Jeton de compte de service et la CA en base64 comme Certificat CA du cluster : seule cette CA est alors de confiance. Un serveur d'API rapporte tout le cluster.

!!! warning
    Le certificat du serveur d'API ne liste que ses propres noms et adresses : une adresse qu'il ne liste pas échoue la vérification. Utilisez-en une qu'il liste (une adresse IP de nœud, ou un nom ajouté avec --tls-san sur k3s), ou cochez Accepter un certificat non vérifiable.

### La même chose comme manifeste

Pour GitOps, ou pour revoir exactement ce qui est accordé, appliquez ce
manifeste au lieu des trois commandes `kubectl create` et du Secret de
l'étape 2 (`kubectl apply -f dumbmonit.yaml`), puis affichez le jeton et la
CA comme à l'étape 2 :

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

Pour vérifier le rôle avant d'ajouter la surveillance, demandez au serveur
d'API ce que le compte peut faire ; la première commande doit répondre
`yes`, la seconde `no` :

```
kubectl auth can-i list pods --as=system:serviceaccount:kube-system:dumbmonit
kubectl auth can-i list secrets --as=system:serviceaccount:kube-system:dumbmonit
```

Un jeton fait avec `kubectl create token dumbmonit -n kube-system`
fonctionne aussi, mais il expire (après une heure par défaut) : la surveillance
échoue alors avec « The API server refused the token (401) ». Le Secret
ci-dessus est la forme longue durée que Kubernetes documente pour cet
usage. Pour révoquer l'accès de DumbMonit, supprimez le Secret, ou le
compte de service.

## Identifiants

| Identifiant | Champs |
|---|---|
| Jeton de compte de service | Le jeton affiché par la première commande `kubectl get secret`, envoyé comme `Authorization: Bearer`. |

Adresse : le serveur d'API comme URL (`https://k3s.lan:6443`), un nom
d'hôte ou une IP (`192.0.2.10`, port 6443 alors), ou `host:port`. Derrière
un répartiteur de charge ou un k3s hautement disponible, donnez l'adresse
qui sert l'API, celle dans votre kubeconfig.

Options :

- **Port** (6443) quand l'adresse n'en donne pas.
- **Certificat CA du cluster** : la valeur base64 de `ca.crt`, comme la
  dernière commande de l'étape 2 l'affiche ; un certificat PEM
  fonctionne aussi, même collé sur une ligne. Avec cela, DumbMonit fait
  confiance à cette CA et à rien d'autre.
- **Accepter un certificat non vérifiable** : ignore chaque vérification,
  pour un réseau de confiance.
- **Délai par requête** (10 s).
- **Namespaces ignorés** : pods, workloads, demandes et événements de ces
  namespaces sont laissés de côté, par exemple un namespace CI plein de
  pods de courte durée. Les événements de nœud sont toujours gardés.

## Dépannage

« The certificate of the API server could not be verified » : soit la CA
manque (collez-la, comme à l'étape 2), soit l'adresse n'est pas un des
noms dans le certificat du serveur. k3s liste `kubernetes`, `localhost`,
le nom d'hôte du nœud et ses adresses IP ; ajoutez un autre nom avec
`--tls-san` (dans `/etc/rancher/k3s/config.yaml` :
`tls-san: [k3s.lan]`) et redémarrez k3s.

« The service account may not read … (403) » : le ClusterRole ou sa
liaison manque ou est incomplet. Le message nomme la ressource que le
serveur d'API a refusée ; appliquez à nouveau le manifeste ci-dessus.
