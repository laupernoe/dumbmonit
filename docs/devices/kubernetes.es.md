# Kubernetes / k3s

Un clúster a través de su servidor de API: nodos que no están Ready o
bajo presión, pods en crash looping o atascados en Pending, Deployments,
StatefulSets y DaemonSets con réplicas faltantes, reclamaciones de
volumen que nunca se vinculan, y los eventos de advertencia de la última
hora.

DumbMonit lee la API con el token de una cuenta de servicio dedicada
vinculada a un ClusterRole de solo lectura: `get` y `list` sobre nodos,
pods, workloads, reclamaciones de volumen persistente y eventos, nada
más. Nunca escribe, y el rol no da acceso a Secrets ni ConfigMaps. Un
dispositivo cubre todo el clúster, sea la distribución que sea: k3s,
kubeadm, k0s, MicroK8s, Talos, o un clúster gestionado cuya API
DumbMonit pueda alcanzar.

Cada minuto DumbMonit lee ocho colecciones, en paralelo: la versión, los
nodos, los pods, los Deployments, StatefulSets y DaemonSets, las
reclamaciones de volumen persistente, y los eventos de tipo `Warning`
(filtrados por el servidor de API). Las listas largas se leen en
páginas de 500.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_k8s_`.

| Métrica | Qué | Etiquetas |
|---|---|---|
| `version_info` | valor 1 | `version` (`1.33.4+k3s1`) |
| `nodes`, `nodes_ready` | nodos, y aquellos cuya condición `Ready` es verdadera | |
| `node_ready` | 1 cuando el nodo está Ready | `node` |
| `node_pressure` | 1 cuando el nodo reporta esa presión; solo las condiciones que el nodo declara | `node`, `condition`: `memory`, `disk`, `pid`, `network` |
| `node_unschedulable` | 1 cuando el nodo está cordoned | `node` |
| `node_info` | valor 1 | `node`, `kubelet_version` |
| `pods`, `pods_running`, `pods_pending`, `pods_failed`, `pods_crashlooping`, `pods_not_ready` | recuentos de pods; los pods completados (`Succeeded`) se excluyen | |
| `pod_ready`, `pod_crashlooping`, `pod_pending` | 1 o 0 por pod | `namespace`, `pod`, `workload` |
| `pod_restarts` | reinicios de contenedor del pod, un contador | `namespace`, `pod`, `workload` |
| `pod_blocked` | 1 mientras un contenedor espera por otro motivo que arrancar: `ImagePullBackOff`, `CreateContainerConfigError`… | `namespace`, `pod`, `workload`, `reason` |
| `workload_desired`, `workload_ready`, `workload_unavailable` | réplicas pedidas, listas, y faltantes; para un DaemonSet, nodos | `kind` (`Deployment`, `StatefulSet`, `DaemonSet`), `namespace`, `workload` |
| `pvc_pending` | 1 cuando la reclamación está Pending **y** un pod la necesita o un evento de advertencia la concierne | `namespace`, `pvc` |
| `pvc_bound` | 1 cuando la reclamación está Bound | `namespace`, `pvc` |
| `pvcs`, `pvcs_pending` | reclamaciones, y las atascadas como arriba | |
| `warning_events` | eventos de advertencia vistos en la última hora | |
| `warning_events_by_reason` | lo mismo por motivo (`BackOff`, `FailedScheduling`, `ProvisioningFailed`…), veinte motivos como máximo | `reason` |

`workload` es el controlador del pod: el Deployment detrás de su
ReplicaSet (`api` para `api-7c9696d669-8tpp6`), el StatefulSet, el
DaemonSet, o vacío para un pod desnudo. Los pods se describen uno por
uno hasta 500, los problemas primero; los recuentos siempre cubren cada
pod.

Una reclamación que simplemente está `Pending` no es un fallo: una
storage class `WaitForFirstConsumer`, la que incluye k3s
(`local-path`), deja una reclamación pendiente hasta que el primer pod
la usa. Solo cuenta como atascada cuando un pod la pide o cuando una
advertencia como `ProvisioningFailed` la concierne.

Las [reglas integradas](../alerting/rules.md#kubernetes) que se
aplican:

- **Kubernetes node not ready**: la condición `Ready` de un nodo es
  falsa o desconocida durante cinco minutos (Warning).
- **Kubernetes node under pressure**: presión de memoria, disco o PID
  durante cinco minutos (Advisory).
- **Kubernetes pod crash looping**: un contenedor en
  `CrashLoopBackOff` durante diez minutos (Warning). Entre dos intentos
  el contenedor se reinicia y el estado desaparece por un momento; la
  regla lee el máximo sobre diez minutos para no reiniciarse cada vez.
- **Kubernetes pod stuck pending**: un pod en Pending durante quince
  minutos (Advisory).
- **Kubernetes pod restarting**: más de cinco reinicios en una hora,
  para un pod que no está ya en crash looping (Advisory).
- **Kubernetes workload missing replicas**: un Deployment, StatefulSet
  o DaemonSet con menos réplicas disponibles de las que pide, durante
  diez minutos (Advisory, escala tras una hora). Una actualización
  rolling retira una réplica mientras inicia otra; los diez minutos lo
  cubren.
- **Kubernetes volume claim pending**: una reclamación atascada durante
  quince minutos (Advisory).

Además de Device unreachable, cuando el servidor de API deja de
responder.

## La página del dispositivo {#the-device-page}

El panel Cluster indica en una frase qué va mal, luego lista los nodos
(Ready o no, presión, cordoned, versión de kubelet), los pods que
necesitan atención (crash looping, pending, blocked, not ready, con sus
reinicios), los workloads con réplicas faltantes, las reclamaciones de
volumen atascadas, y los eventos de advertencia de la última hora por
motivo. Un clúster sano muestra sus recuentos y nada más.

## Crear una cuenta de servicio de solo lectura para DumbMonit {#create-a-read-only-service-account-for-dumbmonit}

1. Crea una cuenta de servicio, un ClusterRole que solo pueda get y
   list sobre nodos, pods, workloads, reclamaciones de volumen y
   eventos, y vincúlalos. El rol no lee ningún Secret ni ConfigMap. En
   un servidor k3s, ejecuta cada comando como sudo k3s kubectl.

    ```
    kubectl create serviceaccount dumbmonit -n kube-system
    kubectl create clusterrole dumbmonit-read --verb=get,list --resource=nodes,pods,persistentvolumeclaims,events,deployments.apps,statefulsets.apps,daemonsets.apps
    kubectl create clusterrolebinding dumbmonit-read --clusterrole=dumbmonit-read --serviceaccount=kube-system:dumbmonit
    ```

2. Dale un token que no caduque: un Secret de tipo
   service-account-token, que Kubernetes rellena. Luego imprime el
   token, y la CA del clúster en base64.

    ```
    echo '{"apiVersion":"v1","kind":"Secret","metadata":{"name":"dumbmonit-token","namespace":"kube-system","annotations":{"kubernetes.io/service-account.name":"dumbmonit"}},"type":"kubernetes.io/service-account-token"}' | kubectl apply -f -
    kubectl -n kube-system get secret dumbmonit-token -o jsonpath='{.data.token}' | base64 -d
    kubectl -n kube-system get secret dumbmonit-token -o jsonpath='{.data.ca\.crt}'
    ```

3. En DumbMonit, introduce la dirección del servidor de API, por
   ejemplo «https://k3s.lan:6443». Pega el token como Service account
   token y la CA en base64 como Cluster CA certificate: entonces solo
   se confía en esa CA. Un servidor de API reporta todo el clúster.

!!! warning
    El certificado del servidor de API solo lista sus propios nombres y
    direcciones: una dirección que no lista falla la verificación. Usa
    una que liste (una dirección IP de nodo, o un nombre añadido con
    --tls-san en k3s), o marca Accept an unverifiable certificate.

### Lo mismo como un manifiesto {#the-same-thing-as-a-manifest}

Para GitOps, o para revisar exactamente qué se concede, aplica este
manifiesto en lugar de los tres comandos `kubectl create` y el Secret
del paso 2 (`kubectl apply -f dumbmonit.yaml`), y luego imprime el token
y la CA como en el paso 2:

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

Para comprobar el rol antes de añadir el dispositivo, pregunta al
servidor de API qué puede hacer la cuenta; el primer comando debe
responder `yes`, el segundo `no`:

```
kubectl auth can-i list pods --as=system:serviceaccount:kube-system:dumbmonit
kubectl auth can-i list secrets --as=system:serviceaccount:kube-system:dumbmonit
```

Un token hecho con `kubectl create token dumbmonit -n kube-system`
también funciona, pero caduca (tras una hora por defecto): el
dispositivo entonces falla con «The API server refused the token
(401)». El Secret de arriba es la forma de larga duración que
Kubernetes documenta para este uso. Para revocar el acceso de
DumbMonit, elimina el Secret, o la cuenta de servicio.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Token de cuenta de servicio | El token impreso por el primer comando `kubectl get secret`, enviado como `Authorization: Bearer`. |

Dirección: el servidor de API como una URL (`https://k3s.lan:6443`), un
nombre de host o IP (`192.0.2.10`, puerto 6443 entonces), o
`host:puerto`. Detrás de un balanceador de carga o un k3s de alta
disponibilidad, da la dirección que sirve la API, la de tu kubeconfig.

Opciones:

- **Port** (6443) cuando la dirección no da ninguno.
- **Cluster CA certificate**: el valor base64 de `ca.crt`, tal como lo
  imprime el último comando del paso 2; un certificado PEM también
  funciona, incluso pegado en una línea. Con él, DumbMonit confía solo
  en esa CA.
- **Accept an unverifiable certificate**: omite cada comprobación,
  para una red en la que confías.
- **Timeout per request** (10 s).
- **Ignored namespaces**: los pods, workloads, reclamaciones y eventos
  de estos namespaces se excluyen, por ejemplo un namespace de CI lleno
  de pods de corta duración. Los eventos de nodo siempre se conservan.

## Solución de problemas {#troubleshooting}

«The certificate of the API server could not be verified»: o falta la
CA (pégala, como en el paso 2), o la dirección no es uno de los nombres
del certificado del servidor. k3s lista `kubernetes`, `localhost`, el
nombre de host del nodo y sus direcciones IP; añade otro nombre con
`--tls-san` (en `/etc/rancher/k3s/config.yaml`: `tls-san: [k3s.lan]`) y
reinicia k3s.

«The service account may not read … (403)»: falta el ClusterRole o su
vinculación, o está incompleta. El mensaje nombra el recurso que
rechazó el servidor de API; vuelve a aplicar el manifiesto de arriba.
