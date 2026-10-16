# Kubernetes / k3s {#kubernetes-k3s}

Um cluster através do seu servidor de API: nós que não estão Ready ou estão sob
pressão, pods em crash loop ou presos em Pending, Deployments, StatefulSets e
DaemonSets sem réplicas, claims de volume que nunca ficam ligados, e os eventos
de aviso da última hora.

O DumbMonit lê a API com o token de uma conta de serviço dedicada ligada a um
ClusterRole só de leitura: `get` e `list` em nodes, pods, workloads, claims de
volumes persistentes e eventos, nada mais. Nunca escreve, e o papel não dá
acesso a Secrets nem a ConfigMaps. Um monitor cobre todo o cluster, seja
qual for a distribuição: k3s, kubeadm, k0s, MicroK8s, Talos, ou um cluster
gerido cuja API o DumbMonit consiga alcançar.

A cada minuto o DumbMonit lê oito coleções, em paralelo: a versão, os nós, os
pods, os Deployments, StatefulSets e DaemonSets, os claims de volumes
persistentes, e os eventos do tipo `Warning` (filtrados pelo servidor de API).
As listas longas são lidas em páginas de 500.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_k8s_`.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `version_info` | valor 1 | `version` (`1.33.4+k3s1`) |
| `nodes`, `nodes_ready` | nós, e aqueles cuja condição `Ready` é verdadeira | |
| `node_ready` | 1 quando o nó está Ready | `node` |
| `node_pressure` | 1 quando o nó reporta essa pressão; apenas as condições que o nó declara | `node`, `condition`: `memory`, `disk`, `pid`, `network` |
| `node_unschedulable` | 1 quando o nó está em cordon | `node` |
| `node_info` | valor 1 | `node`, `kubelet_version` |
| `pods`, `pods_running`, `pods_pending`, `pods_failed`, `pods_crashlooping`, `pods_not_ready` | contagens de pods; os pods concluídos (`Succeeded`) ficam de fora | |
| `pod_ready`, `pod_crashlooping`, `pod_pending` | 1 ou 0 por pod | `namespace`, `pod`, `workload` |
| `pod_restarts` | reinícios de contentores do pod, um contador | `namespace`, `pod`, `workload` |
| `pod_blocked` | 1 enquanto um contentor espera por outro motivo que não o arranque: `ImagePullBackOff`, `CreateContainerConfigError`… | `namespace`, `pod`, `workload`, `reason` |
| `workload_desired`, `workload_ready`, `workload_unavailable` | réplicas pedidas, prontas, e em falta; para um DaemonSet, nós | `kind` (`Deployment`, `StatefulSet`, `DaemonSet`), `namespace`, `workload` |
| `pvc_pending` | 1 quando o claim está Pending **e** um pod precisa dele ou um evento de aviso lhe diz respeito | `namespace`, `pvc` |
| `pvc_bound` | 1 quando o claim está Bound | `namespace`, `pvc` |
| `pvcs`, `pvcs_pending` | claims, e os bloqueados como acima | |
| `warning_events` | eventos de aviso vistos na última hora | |
| `warning_events_by_reason` | o mesmo por motivo (`BackOff`, `FailedScheduling`, `ProvisioningFailed`…), no máximo vinte motivos | `reason` |

`workload` é o controlador do pod: o Deployment por trás do seu ReplicaSet
(`api` para `api-7c9696d669-8tpp6`), o StatefulSet, o DaemonSet, ou vazio para
um pod solto. Os pods são descritos um a um até 500, os problemas primeiro; as
contagens cobrem sempre todos os pods.

Um claim apenas `Pending` não é uma falha: uma storage class
`WaitForFirstConsumer`, a que o k3s traz (`local-path`), deixa um claim pendente
até o primeiro pod o usar. Só conta como bloqueado quando um pod o pede ou
quando um aviso como `ProvisioningFailed` lhe diz respeito.

As [regras incorporadas](../alerting/rules.md#kubernetes) que se aplicam:

- **Kubernetes node not ready**: a condição `Ready` de um nó é falsa ou
  desconhecida durante cinco minutos (Warning).
- **Kubernetes node under pressure**: pressão de memória, disco ou PID durante cinco
  minutos (Advisory).
- **Kubernetes pod crash looping**: um contentor em `CrashLoopBackOff` durante
  dez minutos (Warning). Alguns kubelets (k3s 1.37) mostram o contentor como
  `terminated` com motivo `Error` entre duas tentativas e nunca
  `CrashLoopBackOff`: um contentor que reiniciou pelo menos duas vezes e cuja
  última paragem, nos últimos dez minutos, terminou com um código de saída
  diferente de zero conta também como em crash loop. Entre duas tentativas o
  contentor reinicia e o estado desaparece por momentos; a regra lê o máximo ao
  longo de dez minutos para não recomeçar de cada vez.
- **Kubernetes pod stuck pending**: um pod Pending durante quinze minutos
  (Advisory).
- **Kubernetes pod restarting**: mais de cinco reinícios numa hora, para um
  pod que ainda não está em crash loop (Advisory).
- **Kubernetes workload missing replicas**: um Deployment, StatefulSet ou
  DaemonSet com menos réplicas disponíveis do que pede, durante dez minutos
  (Advisory, escala após uma hora). Uma atualização gradual retira uma
  réplica enquanto arranca outra; os dez minutos cobrem isso.
- **Kubernetes volume claim pending**: um claim bloqueado durante quinze
  minutos (Advisory).

Mais Monitor unreachable, quando o servidor de API deixa de responder.

## A página do monitor {#the-device-page}

O painel Cluster diz numa frase o que está errado, depois lista os nós (Ready
ou não, pressão, cordon, versão do kubelet), os pods que precisam de atenção
(crash loop, pending, bloqueados, não prontos, com os seus reinícios), os
workloads sem réplicas, os claims de volume bloqueados, e os eventos de aviso
da última hora por motivo. Um cluster saudável mostra as suas contagens e nada
mais.

## Criar uma conta de serviço só de leitura para o DumbMonit {#create-a-read-only-service-account-for-dumbmonit}

1. Crie uma conta de serviço, um ClusterRole que só pode fazer get e list de nodes, pods, workloads, claims de volume e eventos, e ligue-os. O papel não lê nenhum Secret nem ConfigMap. Num servidor k3s, execute cada comando como sudo k3s kubectl.

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

3. No DumbMonit, introduza o endereço do servidor de API, por exemplo «https://k3s.lan:6443». Cole o token como Token da conta de serviço e a CA em base64 como Certificado CA do cluster: só essa CA passa então a ser de confiança. Um servidor de API reporta todo o cluster.

!!! warning
    O certificado do servidor de API só lista os seus próprios nomes e endereços: um endereço que ele não liste falha a verificação. Use um que ele liste (o endereço IP de um nó, ou um nome adicionado com --tls-san no k3s), ou marque Aceitar um certificado não verificável.

### O mesmo como manifesto {#the-same-thing-as-a-manifest}

Para GitOps, ou para rever exatamente o que é concedido, aplique este manifesto
em vez dos três comandos `kubectl create` e do Secret do passo 2
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

Para verificar o papel antes de adicionar o monitor, pergunte ao servidor
de API o que a conta pode fazer; o primeiro comando tem de responder `yes`, o
segundo `no`:

```
kubectl auth can-i list pods --as=system:serviceaccount:kube-system:dumbmonit
kubectl auth can-i list secrets --as=system:serviceaccount:kube-system:dumbmonit
```

Um token feito com `kubectl create token dumbmonit -n kube-system` também
funciona, mas expira (após uma hora por predefinição): o monitor falha então
com «The API server refused the token (401)». O Secret acima é a forma de longa
duração que o Kubernetes documenta para este uso. Para revogar o acesso do
DumbMonit, apague o Secret, ou a conta de serviço.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Token da conta de serviço | O token impresso pelo primeiro comando `kubectl get secret`, enviado como `Authorization: Bearer`. |

Endereço: o servidor de API como URL (`https://k3s.lan:6443`), um nome de
anfitrião ou IP (`192.0.2.10`, porta 6443 então), ou `host:port`. Atrás de um
balanceador de carga ou de um k3s de alta disponibilidade, indique o endereço
que serve a API, o do seu kubeconfig.

Opções:

- **Porta** (6443) quando o endereço não indica nenhuma.
- **Certificado CA do cluster**: o valor em base64 de `ca.crt`, como o último
  comando do passo 2 o imprime; um certificado PEM também funciona, mesmo
  colado numa só linha. Com ele, o DumbMonit confia nessa CA e em mais nenhuma.
- **Aceitar um certificado não verificável**: ignora todas as verificações,
  para uma rede em que confia.
- **Tempo limite por pedido** (10 s).
- **Namespaces ignorados**: os pods, workloads, claims e eventos destes
  namespaces ficam de fora, por exemplo um namespace de CI cheio de pods de
  curta duração. Os eventos de nós são sempre mantidos.

## Resolução de problemas {#troubleshooting}

«The certificate of the API server could not be verified»: ou falta a CA (cole-a,
como no passo 2), ou o endereço não é um dos nomes do certificado do servidor.
O k3s lista `kubernetes`, `localhost`, o nome de anfitrião do nó e os seus
endereços IP; adicione outro nome com `--tls-san` (em
`/etc/rancher/k3s/config.yaml`: `tls-san: [k3s.lan]`) e reinicie o k3s.

«The service account may not read … (403)»: o ClusterRole ou a sua ligação está
em falta ou incompleta. A mensagem nomeia o recurso que o servidor de API
recusou; aplique de novo o manifesto acima.
