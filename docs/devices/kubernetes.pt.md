# Kubernetes / k3s {#kubernetes-k3s}

Um cluster através do seu servidor de API: nós que não estão Ready ou
sob pressão, pods em crash loop ou bloqueados em Pending, Deployments,
StatefulSets e DaemonSets sem réplicas, reivindicações de volume que
nunca ficam bound, e os eventos de aviso da última hora.

O DumbMonit lê a API com o token de uma conta de serviço dedicada
associada a uma ClusterRole só de leitura: `get` e `list` em nós, pods,
workloads, reivindicações de volume persistente e eventos, nada mais.
Nunca escreve, e a função não dá acesso a Secrets nem ConfigMaps. Um
dispositivo cobre todo o cluster, qualquer que seja a distribuição: k3s,
kubeadm, k0s, MicroK8s, Talos, ou um cluster gerido cuja API o
DumbMonit possa alcançar.

A cada minuto o DumbMonit lê oito coleções, em paralelo: a versão, os
nós, os pods, os Deployments, StatefulSets e DaemonSets, as
reivindicações de volume persistente, e os eventos do tipo `Warning`
(filtrados pelo servidor de API). Listas longas são lidas em páginas de
500.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_k8s_`.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `version_info` | valor 1 | `version` (`1.33.4+k3s1`) |
| `nodes`, `nodes_ready` | nós, e os cuja condição `Ready` é verdadeira | |
| `node_ready` | 1 quando o nó está Ready | `node` |
| `node_pressure` | 1 quando o nó reporta essa pressão; só as condições que o nó declara | `node`, `condition`: `memory`, `disk`, `pid`, `network` |
| `node_unschedulable` | 1 quando o nó está cordoned | `node` |
| `node_info` | valor 1 | `node`, `kubelet_version` |
| `pods`, `pods_running`, `pods_pending`, `pods_failed`, `pods_crashlooping`, `pods_not_ready` | contagens de pods; pods concluídos (`Succeeded`) são deixados de fora | |
| `pod_ready`, `pod_crashlooping`, `pod_pending` | 1 ou 0 por pod | `namespace`, `pod`, `workload` |
| `pod_restarts` | reinícios de contentor do pod, um contador | `namespace`, `pod`, `workload` |
| `pod_blocked` | 1 enquanto um contentor espera por outro motivo que não iniciar: `ImagePullBackOff`, `CreateContainerConfigError`… | `namespace`, `pod`, `workload`, `reason` |
| `workload_desired`, `workload_ready`, `workload_unavailable` | réplicas pedidas, prontas, e em falta; para um DaemonSet, nós | `kind` (`Deployment`, `StatefulSet`, `DaemonSet`), `namespace`, `workload` |
| `pvc_pending` | 1 quando a reivindicação está Pending **e** um pod precisa dela ou um evento de aviso a afeta | `namespace`, `pvc` |
| `pvc_bound` | 1 quando a reivindicação está Bound | `namespace`, `pvc` |
| `pvcs`, `pvcs_pending` | reivindicações, e as bloqueadas como acima | |
| `warning_events` | eventos de aviso vistos na última hora | |
| `warning_events_by_reason` | o mesmo por motivo (`BackOff`, `FailedScheduling`, `ProvisioningFailed`…), no máximo vinte motivos | `reason` |

`workload` é o controlador do pod: o Deployment por detrás do seu
ReplicaSet (`api` para `api-7c9696d669-8tpp6`), o StatefulSet, o
DaemonSet, ou vazio para um pod simples. Os pods são descritos um a um
até 500, os problemas primeiro; as contagens cobrem sempre todo pod.

Uma reivindicação que está apenas `Pending` não é uma falha: uma classe
de armazenamento `WaitForFirstConsumer`, a que o k3s traz
(`local-path`), deixa uma reivindicação pendente até o primeiro pod a
usar. Só conta como bloqueada quando um pod a pede ou quando um aviso
como `ProvisioningFailed` a afeta.

As [regras incorporadas](../alerting/rules.md#kubernetes) que se
aplicam:

- **Kubernetes node not ready**: a condição `Ready` de um nó é falsa ou
  desconhecida há cinco minutos (Warning).
- **Kubernetes node under pressure**: pressão de memória, disco ou PID
  durante cinco minutos (Advisory).
- **Kubernetes pod crash looping**: um contentor em `CrashLoopBackOff`
  durante dez minutos (Warning). Entre duas tentativas o contentor
  reinicia e o estado desaparece por um momento; a regra lê o máximo ao
  longo de dez minutos para que não recomece de cada vez.
- **Kubernetes pod stuck pending**: um pod Pending durante quinze
  minutos (Advisory).
- **Kubernetes pod restarting**: mais de cinco reinícios numa hora, para
  um pod que ainda não está em crash loop (Advisory).
- **Kubernetes workload missing replicas**: um Deployment, StatefulSet
  ou DaemonSet com menos réplicas disponíveis do que pede, durante dez
  minutos (Advisory, agrava depois de uma hora). Uma atualização
  gradual remove uma réplica enquanto inicia outra; os dez minutos
  cobrem isso.
- **Kubernetes volume claim pending**: uma reivindicação bloqueada
  durante quinze minutos (Advisory).

Mais Device unreachable, quando o servidor de API deixa de responder.

## A página do dispositivo {#the-device-page}

O painel Cluster diz o que está mal numa frase, depois lista os nós
(Ready ou não, pressão, cordoned, versão do kubelet), os pods que
precisam de atenção (crash loop, pending, bloqueados, não prontos, com
os seus reinícios), os workloads sem réplicas, as reivindicações de
volume bloqueadas, e os eventos de aviso da última hora por motivo. Um
cluster saudável mostra as suas contagens e nada mais.

## Criar uma conta de serviço só de leitura para o DumbMonit {#create-a-read-only-service-account-for-dumbmonit}

1. Crie uma conta de serviço, uma ClusterRole que só pode get e list nós, pods, workloads, reivindicações de volume e eventos, e associe-as. A função não lê nenhum Secret nem ConfigMap. Num servidor k3s, execute cada comando como sudo k3s kubectl.

    ```
    kubectl create serviceaccount dumbmonit -n kube-system
    kubectl create clusterrole dumbmonit-read --verb=get,list --resource=nodes,pods,persistentvolumeclaims,events,deployments.apps,statefulsets.apps,daemonsets.apps
    kubectl create clusterrolebinding dumbmonit-read --clusterrole=dumbmonit-read --serviceaccount=kube-system:dumbmonit
    ```

2. Dê-lhe um token que não expira: um Secret do tipo service-account-token, que o Kubernetes preenche. Depois imprima o token, e a CA do cluster em base64.

    ```
    echo '{"apiVersion":"v1","kind":"Secret","metadata":{"name":"dumbmonit-token","namespace":"kube-system","annotations":{"kubernetes.io/service-account.name":"dumbmonit"}},"type":"kubernetes.io/service-account-token"}' | kubectl apply -f -
    kubectl -n kube-system get secret dumbmonit-token -o jsonpath='{.data.token}' | base64 -d
    kubectl -n kube-system get secret dumbmonit-token -o jsonpath='{.data.ca\.crt}'
    ```

3. No DumbMonit, introduza o endereço do servidor de API, por exemplo «https://k3s.lan:6443». Cole o token como Service account token e a CA base64 como Cluster CA certificate: só essa CA é então confiada. Um servidor de API reporta todo o cluster.

!!! warning
    O certificado do servidor de API só lista os seus próprios nomes e endereços: um endereço que não lista falha a verificação. Use um que liste (um endereço IP de nó, ou um nome adicionado com --tls-san no k3s), ou marque Accept an unverifiable certificate.

### A mesma coisa como um manifesto {#the-same-thing-as-a-manifest}

Para GitOps, ou para rever exatamente o que é concedido, aplique este
manifesto em vez dos três comandos `kubectl create` e do Secret do
passo 2 (`kubectl apply -f dumbmonit.yaml`), depois imprima o token e a
CA como no passo 2:

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

Para verificar a função antes de adicionar o dispositivo, pergunte ao
servidor de API o que a conta pode fazer; o primeiro comando tem de
responder `yes`, o segundo `no`:

```
kubectl auth can-i list pods --as=system:serviceaccount:kube-system:dumbmonit
kubectl auth can-i list secrets --as=system:serviceaccount:kube-system:dumbmonit
```

Um token feito com `kubectl create token dumbmonit -n kube-system`
também funciona, mas expira (depois de uma hora por predefinição): o
dispositivo então falha com «The API server refused the token (401)».
O Secret acima é a forma de longa duração que o Kubernetes documenta
para este uso. Para revogar o acesso do DumbMonit, elimine o Secret, ou
a conta de serviço.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Token de conta de serviço | O token impresso pelo primeiro comando `kubectl get secret`, enviado como `Authorization: Bearer`. |

Endereço: o servidor de API como um URL (`https://k3s.lan:6443`), um
nome de anfitrião ou IP (`192.0.2.10`, porta 6443 então), ou
`anfitrião:porta`. Atrás de um balanceador de carga ou de um k3s de
alta disponibilidade, dê o endereço que serve a API, o do seu
kubeconfig.

Opções:

- **Port** (6443) quando o endereço não dá nenhuma.
- **Cluster CA certificate**: o valor base64 de `ca.crt`, como o último
  comando do passo 2 o imprime; um certificado PEM também funciona,
  mesmo colado numa linha. Com ele, o DumbMonit confia só nessa CA.
- **Accept an unverifiable certificate**: ignora toda verificação, para
  uma rede de confiança.
- **Timeout per request** (10 s).
- **Ignored namespaces**: pods, workloads, reivindicações e eventos
  destes namespaces são deixados de fora, por exemplo um namespace de
  CI repleto de pods de vida curta. Os eventos de nó são sempre
  mantidos.

## Resolução de problemas {#troubleshooting}

«The certificate of the API server could not be verified»: ou a CA está
em falta (cole-a, como no passo 2), ou o endereço não é um dos nomes no
certificado do servidor. O k3s lista `kubernetes`, `localhost`, o nome
de anfitrião do nó e os seus endereços IP; adicione outro nome com
`--tls-san` (em `/etc/rancher/k3s/config.yaml`: `tls-san: [k3s.lan]`) e
reinicie o k3s.

«The service account may not read … (403)»: a ClusterRole ou a sua
associação está em falta ou incompleta. A mensagem nomeia o recurso que
o servidor de API recusou; aplique o manifesto acima de novo.
