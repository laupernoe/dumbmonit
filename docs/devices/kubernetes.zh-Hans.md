# Kubernetes / k3s {#kubernetes--k3s}

通过其 API 服务器的集群：不是 Ready 或处于压力下的节点、Pod 崩溃循环或卡住的 Pending、缺少副本的 Deployment、StatefulSet 和 DaemonSet、永远不会绑定的卷声明，以及最后一小时的警告事件。

DumbMonit 使用专用服务账户的令牌读取 API，该令牌绑定到只读 ClusterRole：对节点、Pod、工作负载、持久卷声明和事件只有 `get` 和 `list`，别的没有。它永远不写，该角色不提供对 Secrets 或 ConfigMaps 的访问。一个设备覆盖整个集群，无论分发版本如何：k3s、kubeadm、k0s、MicroK8s、Talos 或 DumbMonit 可以到达其 API 的托管集群。

DumbMonit 每分钟平行读取八个集合：版本、节点、Pod、Deployment、StatefulSet 和 DaemonSet、持久卷声明和 `Warning` 类型的事件（由 API 服务器过滤）。长列表在 500 页面上读取。

## 监视什么 {#what-it-watches}

所有指标都带有前缀 `dumbmonit_k8s_`。

| 指标 | 说明 | 标签 |
|---|---|---|
| `version_info` | 值为 1 | `version` (`1.33.4+k3s1`) |
| `nodes`, `nodes_ready` | 节点，以及那些 `Ready` 条件为真的 | |
| `node_ready` | 当节点就绪时为 1 | `node` |
| `node_pressure` | 当节点报告压力时为 1；仅节点声明的条件 | `node`, `condition`: `memory`, `disk`, `pid`, `network` |
| `node_unschedulable` | 当节点被隔离时为 1 | `node` |
| `node_info` | 值为 1 | `node`, `kubelet_version` |
| `pods`, `pods_running`, `pods_pending`, `pods_failed`, `pods_crashlooping`, `pods_not_ready` | Pod 计数；已完成的 Pod（`Succeeded`）被忽略 | |
| `pod_ready`, `pod_crashlooping`, `pod_pending` | 每个 Pod 为 1 或 0 | `namespace`, `pod`, `workload` |
| `pod_restarts` | Pod 的容器重启次数，一个计数器 | `namespace`, `pod`, `workload` |
| `pod_blocked` | 当容器因启动以外的原因等待时为 1：`ImagePullBackOff`、`CreateContainerConfigError` 等 | `namespace`, `pod`, `workload`, `reason` |
| `workload_desired`, `workload_ready`, `workload_unavailable` | 要求的副本、就绪和缺失；对于 DaemonSet，节点 | `kind` (`Deployment`, `StatefulSet`, `DaemonSet`), `namespace`, `workload` |
| `pvc_pending` | 当声明为 Pending **且** Pod 需要它或警告事件关乎它时为 1 | `namespace`, `pvc` |
| `pvc_bound` | 当声明被绑定时为 1 | `namespace`, `pvc` |
| `pvcs`, `pvcs_pending` | 声明及那些如上所述卡住的 | |
| `warning_events` | 在最后一小时看到的警告事件 | |
| `warning_events_by_reason` | 相同按原因（`BackOff`、`FailedScheduling`、`ProvisioningFailed` 等），最多二十个原因 | `reason` |

`workload` 是 Pod 的控制器：其 ReplicaSet 后面的 Deployment（`api` 用于 `api-7c9696d669-8tpp6`）、StatefulSet、DaemonSet 或为空的裸 Pod。Pod 一个一个描述，最多 500 个，问题优先；计数始终覆盖每个 Pod。

仅仅是 `Pending` 的声明不是故障：一个 `WaitForFirstConsumer` 存储类，k3s 附带的那个（`local-path`），让声明待处理，直到第一个 Pod 使用它。它仅当 Pod 要求它或当 `ProvisioningFailed` 这样的警告关乎它时计为卡住。

适用的[内置规则](../alerting/rules.md#kubernetes)：

- **Kubernetes 节点不就绪**：节点的 `Ready` 条件为假或未知五分钟（警告）。
- **Kubernetes 节点处于压力下**：内存、磁盘或 PID 压力五分钟（通知）。
- **Kubernetes Pod 崩溃循环**：容器在 `CrashLoopBackOff` 中十分钟（警告）。某些 kubelet（k3s 1.37）在两次尝试之间将容器显示为 `terminated` 且原因为 `Error`，从不 `CrashLoopBackOff`：重启至少两次且其最后一次停止在十分钟内以非零退出代码结束的容器也计为崩溃循环。在两次尝试之间容器重启，状态消失瞬间；规则在十分钟内读取最大值，使其不在每次重启时重新开始。
- **Kubernetes Pod 卡住待处理**：Pod Pending 十五分钟（通知）。
- **Kubernetes Pod 重启**：一小时内超过五次重启，对于未已经崩溃循环的 Pod（通知）。
- **Kubernetes 工作负载缺少副本**：Deployment、StatefulSet 或 DaemonSet 可用副本少于它要求的，十分钟（通知，一小时后升级）。滚动更新在启动另一个时移除一个副本；十分钟涵盖它。
- **Kubernetes 卷声明待处理**：卡住的声明十五分钟（通知）。

加上设备不可达，当 API 服务器停止回答时。

## 设备页面 {#the-device-page}

集群面板用一句话说出什么是错的，然后列出节点（就绪或不，压力，隔离，kubelet 版本）、需要注意的 Pod（崩溃循环、待处理、受阻、未就绪、及其重启）、缺少副本的工作负载、卡住的卷声明，以及最后一小时按原因的警告事件。健康的集群显示其计数及别的什么也不。

## 为 DumbMonit 创建只读服务账户 {#create-a-read-only-service-account-for-dumbmonit}

1. 创建一个服务账户、一个 ClusterRole 仅可以 get 和 list 节点、Pod、工作负载、卷声明和事件，以及将它们绑定在一起。该角色读取没有 Secret 和 ConfigMap。在 k3s 服务器上，运行每个命令为 sudo k3s kubectl。

    ```
    kubectl create serviceaccount dumbmonit -n kube-system
    kubectl create clusterrole dumbmonit-read --verb=get,list --resource=nodes,pods,persistentvolumeclaims,events,deployments.apps,statefulsets.apps,daemonsets.apps
    kubectl create clusterrolebinding dumbmonit-read --clusterrole=dumbmonit-read --serviceaccount=kube-system:dumbmonit
    ```

2. 给它一个不过期的令牌：一个服务账户令牌类型的 Secret，Kubernetes 填充。然后打印令牌及集群 CA 在 base64 中。

    ```
    echo '{"apiVersion":"v1","kind":"Secret","metadata":{"name":"dumbmonit-token","namespace":"kube-system","annotations":{"kubernetes.io/service-account.name":"dumbmonit"}},"type":"kubernetes.io/service-account-token"}' | kubectl apply -f -
    kubectl -n kube-system get secret dumbmonit-token -o jsonpath='{.data.token}' | base64 -d
    kubectl -n kube-system get secret dumbmonit-token -o jsonpath='{.data.ca\.crt}'
    ```

3. 在 DumbMonit 中，输入 API 服务器的地址，例如"https://k3s.lan:6443"。粘贴令牌作为服务账户令牌和 base64 CA 作为集群 CA 证书：仅那个 CA 随后被信任。一个 API 服务器报告整个集群。

!!! warning
    API 服务器的证书仅列出其自己的名称和地址：它不列出的地址验证失败。使用它列出的（节点 IP 地址或名称添加到 k3s 的 --tls-san），或勾选"接受不可验证的证书"。

### 相同作为清单 {#the-same-thing-as-a-manifest}

对于 GitOps，或审查准确被授予的东西，应用这个清单而不是三个 `kubectl create` 命令和步骤 2 的 Secret（`kubectl apply -f dumbmonit.yaml`），然后按照步骤 2 打印令牌和 CA：

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

在添加设备之前检查该角色，询问 API 服务器该账户可以做什么；第一个命令必须回答 `yes`，第二个 `no`：

```
kubectl auth can-i list pods --as=system:serviceaccount:kube-system:dumbmonit
kubectl auth can-i list secrets --as=system:serviceaccount:kube-system:dumbmonit
```

一个令牌使用 `kubectl create token dumbmonit -n kube-system` 制作的也有效，但它过期（默认一小时后）：设备随后失败，"API 服务器拒绝了令牌（401）"。上面的 Secret 是长期形式 Kubernetes 为此使用文档。要撤销 DumbMonit 的访问，删除 Secret 或服务账户。

## 凭证 {#credentials}

| 凭证 | 字段 |
|---|---|
| 服务账户令牌 | 由第一个 `kubectl get secret` 命令打印的令牌，以 `Authorization: Bearer` 形式发送。 |

地址：API 服务器作为 URL（`https://k3s.lan:6443`）、主机名或 IP（`192.0.2.10`，随后端口 6443），或 `host:port`。在负载均衡器或高可用 k3s 后面，给出提供 API 的地址，您的 kubeconfig 中的那个。

选项：

- **端口** (6443) 当地址不给出。
- **集群 CA 证书**：`ca.crt` 的 base64 值，作为步骤 2 的最后一个命令打印它；一个 PEM 证书也有效，即使粘贴在一行上。有了它，DumbMonit 信任那个 CA 和别的什么也不。
- **接受不可验证的证书**：跳过每个检查，对于您信任的网络。
- **每个请求超时** (10 秒)。
- **忽略的命名空间**：这些命名空间的 Pod、工作负载、声明和事件被忽略，例如一个充满短期 Pod 的 CI 命名空间。节点事件始终被保留。

## 故障排除 {#troubleshooting}

"API 服务器的证书无法验证"：要么 CA 缺失（粘贴它，如在步骤 2），要么地址不是服务器证书中名称之一。k3s 列出 `kubernetes`、`localhost`、节点的主机名及其 IP 地址；使用 `--tls-san` 添加另一个名称（在 `/etc/rancher/k3s/config.yaml` 中：`tls-san: [k3s.lan]`）并重启 k3s。

"服务账户可能不读 …（403）"：ClusterRole 或其绑定缺失或不完整。消息命名 API 服务器拒绝的资源；再次应用上面的清单。
