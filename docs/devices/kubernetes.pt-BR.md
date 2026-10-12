# Kubernetes / k3s

Um cluster através de seu API server: nós que não estão Ready ou sob
pressão, pods em crash loop ou travados em Pending, Deployments,
StatefulSets e DaemonSets com réplicas faltando, reivindicações de volume
que nunca ficam vinculadas, e os eventos de aviso da última hora.

O DumbMonit lê a API com o token de uma conta de serviço dedicada
vinculada a um ClusterRole somente leitura: `get` e `list` em nós, pods,
workloads, reivindicações de volume persistente e eventos, nada mais. Ele
nunca escreve, e o papel não dá acesso a Secrets ou ConfigMaps. Um
dispositivo cobre todo o cluster, qualquer que seja a distribuição: k3s,
kubeadm, k0s, MicroK8s, Talos, ou um cluster gerenciado cuja API o
DumbMonit possa alcançar.

A cada minuto o DumbMonit lê oito coleções, em paralelo: a versão, os nós,
os pods, os Deployments, StatefulSets e DaemonSets, as reivindicações de
volume persistente, e os eventos do tipo `Warning` (filtrados pelo API
server). Listas longas são lidas em páginas de 500.

## O que ele observa

Todas as métricas têm o prefixo `dumbmonit_k8s_`.

| Métrica | O que é | Rótulos |
|---|---|---|
| `version_info` | valor 1 | `version` (`1.33.4+k3s1`) |
| `nodes`, `nodes_ready` | nós, e os cuja condição `Ready` é verdadeira | |
| `node_ready` | 1 quando o nó está Ready | `node` |
| `node_pressure` | 1 quando o nó reporta essa pressão; apenas as condições que o nó declara | `node`, `condition`: `memory`, `disk`, `pid`, `network` |
| `node_unschedulable` | 1 quando o nó está cordoned | `node` |
| `node_info` | valor 1 | `node`, `kubelet_version` |
| `pods`, `pods_running`, `pods_pending`, `pods_failed`, `pods_crashlooping`, `pods_not_ready` | contagens de pods; pods concluídos (`Succeeded`) são deixados de fora | |
| `pod_ready`, `pod_crashlooping`, `pod_pending` | 1 ou 0 por pod | `namespace`, `pod`, `workload` |
| `pod_restarts` | reinícios de contêiner do pod, um contador | `namespace`, `pod`, `workload` |
| `pod_blocked` | 1 enquanto um contêiner espera por outro motivo que não iniciar: `ImagePullBackOff`, `CreateContainerConfigError`… | `namespace`, `pod`, `workload`, `reason` |
| `workload_desired`, `workload_ready`, `workload_unavailable` | réplicas pedidas, prontas, e faltando; para um DaemonSet, nós | `kind` (`Deployment`, `StatefulSet`, `DaemonSet`), `namespace`, `workload` |
| `pvc_pending` | 1 quando a reivindicação está Pending **e** um pod precisa dela ou um evento de aviso a envolve | `namespace`, `pvc` |
| `pvc_bound` | 1 quando a reivindicação está Bound | `namespace`, `pvc` |
| `pvcs`, `pvcs_pending` | reivindicações, e as travadas como acima | |
| `warning_events` | eventos de aviso vistos na última hora | |
| `warning_events_by_reason` | o mesmo por motivo (`BackOff`, `FailedScheduling`, `ProvisioningFailed`…), vinte motivos no máximo | `reason` |

`workload` é o controlador do pod: o Deployment detrás de seu ReplicaSet
(`api` para `api-7c9696d669-8tpp6`), o StatefulSet, o DaemonSet, ou vazio
para um pod simples. Pods são descritos um a um até 500, problemas
primeiro; contagens sempre cobrem todo pod.

Uma reivindicação que está apenas `Pending` não é uma falha: uma classe de
armazenamento `WaitForFirstConsumer`, a que o k3s entrega
(`local-path`), deixa uma reivindicação pendente até o primeiro pod a
usar. Só conta como travada quando um pod a pede ou quando um aviso como
`ProvisioningFailed` a envolve.

As [regras integradas](../alerting/rules.md#kubernetes) que se aplicam:

- **Kubernetes node not ready**: a condição `Ready` de um nó é falsa ou
  desconhecida por cinco minutos (Warning).
- **Kubernetes node under pressure**: pressão de memória, disco ou PID
  por cinco minutos (Advisory).
- **Kubernetes pod crash looping**: um contêiner em `CrashLoopBackOff`
  por dez minutos (Warning). Entre duas tentativas o contêiner reinicia e
  o estado desaparece por um momento; a regra lê o máximo nos dez minutos
  para que não comece de novo a cada vez.
- **Kubernetes pod stuck pending**: um pod Pending por quinze minutos
  (Advisory).
- **Kubernetes pod restarting**: mais de cinco reinícios em uma hora,
  para um pod que ainda não está em crash loop (Advisory).
- **Kubernetes workload missing replicas**: um Deployment, StatefulSet ou
  DaemonSet com menos réplicas disponíveis do que pede, por dez minutos
  (Advisory, escala depois de uma hora). Uma atualização gradual remove
  uma réplica enquanto inicia outra; os dez minutos cobrem isso.
- **Kubernetes volume claim pending**: uma reivindicação travada por
  quinze minutos (Advisory).

Além de Device unreachable, quando o API server para de responder.

## A página do dispositivo

O painel Cluster diz o que está errado em uma frase, depois lista os nós
(Ready ou não, pressão, cordoned, versão do kubelet), os pods que
precisam de atenção (crash looping, pending, blocked, not ready, com seus
reinícios), os workloads com réplicas faltando, as reivindicações de
volume travadas, e os eventos de aviso da última hora por motivo. Um
cluster saudável mostra suas contagens e nada mais.

## Criar uma conta de serviço somente leitura para o DumbMonit

1. Crie uma conta de serviço, um ClusterRole que só pode fazer get e list
   em nós, pods, workloads, reivindicações de volume e eventos, e os
   vincule juntos. O papel não lê nenhum Secret e nenhum ConfigMap. Em um
   servidor k3s, execute cada comando como sudo k3s kubectl.

    ```
    kubectl create serviceaccount dumbmonit -n kube-system
    kubectl create clusterrole dumbmonit-read --verb=get,list --resource=nodes,pods,persistentvolumeclaims,events,deployments.apps,statefulsets.apps,daemonsets.apps
    kubectl create clusterrolebinding dumbmonit-read --clusterrole=dumbmonit-read --serviceaccount=kube-system:dumbmonit
    ```

2. Dê a ela um token que não expira: um Secret do tipo
   service-account-token, que o Kubernetes preenche. Depois imprima o
   token, e a CA do cluster em base64.

    ```
    echo '{"apiVersion":"v1","kind":"Secret","metadata":{"name":"dumbmonit-token","namespace":"kube-system","annotations":{"kubernetes.io/service-account.name":"dumbmonit"}},"type":"kubernetes.io/service-account-token"}' | kubectl apply -f -
    kubectl -n kube-system get secret dumbmonit-token -o jsonpath='{.data.token}' | base64 -d
    kubectl -n kube-system get secret dumbmonit-token -o jsonpath='{.data.ca\.crt}'
    ```

3. No DumbMonit, informe o endereço do API server, por exemplo
   "https://k3s.lan:6443". Cole o token como Token da conta de serviço e
   a CA base64 como Certificado CA do cluster: só essa CA é então
   confiável. Um API server reporta todo o cluster.

!!! warning
    O certificado do API server só lista seus próprios nomes e endereços:
    um endereço que não lista falha na verificação. Use um que ele lista
    (um endereço IP de nó, ou um nome adicionado com --tls-san no k3s), ou
    marque Aceitar um certificado não verificável.

### A mesma coisa como um manifesto

Para GitOps, ou para revisar exatamente o que é concedido, aplique este
manifesto em vez dos três comandos `kubectl create` e o Secret do passo 2
(`kubectl apply -f dumbmonit.yaml`), depois imprima o token e a CA como no
passo 2:

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

Para verificar o papel antes de adicionar o dispositivo, pergunte ao API
server o que a conta pode fazer; o primeiro comando precisa responder
`yes`, o segundo `no`:

```
kubectl auth can-i list pods --as=system:serviceaccount:kube-system:dumbmonit
kubectl auth can-i list secrets --as=system:serviceaccount:kube-system:dumbmonit
```

Um token feito com `kubectl create token dumbmonit -n kube-system`
também funciona, mas expira (depois de uma hora por padrão): o
dispositivo então falha com "The API server refused the token (401)". O
Secret acima é a forma de longa duração que o Kubernetes documenta para
este uso. Para revogar o acesso do DumbMonit, exclua o Secret, ou a conta
de serviço.

## Credenciais

| Credencial | Campos |
|---|---|
| Token da conta de serviço | O token impresso pelo primeiro comando `kubectl get secret`, enviado como `Authorization: Bearer`. |

Endereço: o API server como uma URL (`https://k3s.lan:6443`), um nome de
host ou IP (`192.0.2.10`, porta 6443 então), ou `host:porta`. Detrás de um
load balancer ou um k3s de alta disponibilidade, dê o endereço que serve
a API, o que está no seu kubeconfig.

Opções:

- **Porta** (6443) quando o endereço não dá nenhuma.
- **Certificado CA do cluster**: o valor base64 de `ca.crt`, como o
  último comando do passo 2 o imprime; um certificado PEM também
  funciona, mesmo colado em uma linha. Com ele, o DumbMonit confia
  naquela CA e em nada mais.
- **Aceitar um certificado não verificável**: pula toda verificação,
  para uma rede que você confia.
- **Tempo limite por requisição** (10 s).
- **Namespaces ignorados**: pods, workloads, reivindicações e eventos
  desses namespaces são deixados de fora, por exemplo um namespace de CI
  cheio de pods de vida curta. Eventos de nó são sempre mantidos.

## Solução de problemas

"The certificate of the API server could not be verified": ou a CA está
faltando (cole-a, como no passo 2), ou o endereço não é um dos nomes no
certificado do servidor. O k3s lista `kubernetes`, `localhost`, o nome de
host do nó e seus endereços IP; adicione outro nome com `--tls-san` (em
`/etc/rancher/k3s/config.yaml`: `tls-san: [k3s.lan]`) e reinicie o k3s.

"The service account may not read … (403)": o ClusterRole ou seu binding
está faltando ou incompleto. A mensagem nomeia o recurso que o API server
recusou; aplique o manifesto acima de novo.
