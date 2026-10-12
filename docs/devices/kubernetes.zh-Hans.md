# Kubernetes / k3s

通过其 API 服务器监控一个集群:未就绪或承受压力的节点、崩溃循环
或卡在 Pending 状态的 Pod、缺少副本的 Deployment、StatefulSet 和
DaemonSet、永远无法绑定的卷声明,以及过去一小时内的警告事件。

DumbMonit 使用绑定到一个只读 ClusterRole 的专用服务账户的令牌
读取该 API:对节点、Pod、工作负载、持久卷声明和事件只执行 `get`
和 `list`,不做其他任何事。它从不写入任何内容,该角色也没有访问
Secret 或 ConfigMap 的权限。无论是哪种发行版——k3s、kubeadm、
k0s、MicroK8s、Talos,或任何 DumbMonit 能访问其 API 的托管
集群——一台设备就能覆盖整个集群。

DumbMonit 每分钟并行读取八个集合:版本、节点、Pod、Deployment、
StatefulSet 和 DaemonSet、持久卷声明,以及类型为 `Warning` 的
事件(由 API 服务器过滤)。较长的列表会以 500 条为一页读取。

## 监控内容 {#what-it-watches}

所有指标都以 `dumbmonit_k8s_` 为前缀。

| 指标 | 含义 | 标签 |
|---|---|---|
| `version_info` | 固定值 1 | `version`(`1.33.4+k3s1`) |
| `nodes`, `nodes_ready` | 节点数,以及 `Ready` 条件为真的节点数 | |
| `node_ready` | 该节点为 Ready 时为 1 | `node` |
| `node_pressure` | 该节点报告该压力时为 1;只统计该节点声明的条件 | `node`, `condition`:`memory`、`disk`、`pid`、`network` |
| `node_unschedulable` | 该节点被隔离(cordoned)时为 1 | `node` |
| `node_info` | 固定值 1 | `node`, `kubelet_version` |
| `pods`, `pods_running`, `pods_pending`, `pods_failed`, `pods_crashlooping`, `pods_not_ready` | Pod 计数;已完成的 Pod(`Succeeded`)不包含在内 | |
| `pod_ready`, `pod_crashlooping`, `pod_pending` | 每个 Pod 的值为 1 或 0 | `namespace`, `pod`, `workload` |
| `pod_restarts` | 该 Pod 容器重启的次数,计数器 | `namespace`, `pod`, `workload` |
| `pod_blocked` | 当某个容器因除启动之外的其他原因等待时为 1:`ImagePullBackOff`、`CreateContainerConfigError`…… | `namespace`, `pod`, `workload`, `reason` |
| `workload_desired`, `workload_ready`, `workload_unavailable` | 期望的副本数、已就绪的副本数,以及缺少的副本数;对 DaemonSet 则是节点数 | `kind`(`Deployment`、`StatefulSet`、`DaemonSet`)、`namespace`、`workload` |
| `pvc_pending` | 当该声明处于 Pending **且**有某个 Pod 需要它,或有某个警告事件涉及它时为 1 | `namespace`, `pvc` |
| `pvc_bound` | 当该声明处于 Bound 时为 1 | `namespace`, `pvc` |
| `pvcs`, `pvcs_pending` | 声明数,以及以上述方式卡住的声明数 | |
| `warning_events` | 过去一小时内看到的警告事件数 | |
| `warning_events_by_reason` | 同上,按原因划分(`BackOff`、`FailedScheduling`、`ProvisioningFailed`……),最多二十种原因 | `reason` |

`workload` 是该 Pod 的控制器:其 ReplicaSet 背后的 Deployment
(`api-7c9696d669-8tpp6` 对应 `api`)、StatefulSet、DaemonSet,
或对裸 Pod 为空。Pod 会逐一描述,最多 500 个,问题优先;计数
始终涵盖所有 Pod。

一个仅处于 `Pending` 状态的声明不算故障:`WaitForFirstConsumer`
存储类(k3s 自带的 `local-path` 就是这种),在第一个 Pod 使用它
之前会让它保持 pending。只有当某个 Pod 需要它,或某个警告(例如
`ProvisioningFailed`)涉及它时,才会算作卡住。

适用的[内置规则](../alerting/rules.md#kubernetes):

- **Kubernetes 节点未就绪**:某个节点的 `Ready` 条件为假或未知
  持续五分钟(警告级别)。
- **Kubernetes 节点承受压力**:内存、磁盘或 PID 压力持续五分钟
  (提示级别)。
- **Kubernetes Pod 崩溃循环**:某个容器处于 `CrashLoopBackOff`
  持续十分钟(警告级别)。在两次尝试之间容器会重启,状态会短暂
  消失;该规则读取十分钟内的最大值,因此不会每次都重新计时。
- **Kubernetes Pod 卡在 Pending**:某个 Pod 处于 Pending 持续
  十五分钟(提示级别)。
- **Kubernetes Pod 重启中**:一小时内重启超过五次,且该 Pod 尚未
  处于崩溃循环状态(提示级别)。
- **Kubernetes 工作负载缺少副本**:某个 Deployment、StatefulSet
  或 DaemonSet 的可用副本数少于期望值,持续十分钟(提示级别,
  一小时后升级)。滚动更新会先移除一个副本再启动另一个,这十分钟
  能覆盖这个过程。
- **Kubernetes 卷声明卡住**:某个声明卡住持续十五分钟(提示
  级别)。

以及当 API 服务器停止应答时的设备不可达。

## 设备页面 {#the-device-page}

Cluster 面板会用一句话说明哪里出了问题,然后列出节点(是否
Ready、压力情况、是否被隔离、kubelet 版本)、需要关注的 Pod
(崩溃循环、Pending、被阻塞、未就绪,及其重启次数)、缺少副本的
工作负载、卡住的卷声明,以及过去一小时内按原因划分的警告事件。
一个健康的集群只会显示其计数,不会显示其他内容。

## 为 DumbMonit 创建一个只读服务账户 {#create-a-read-only-service-account-for-dumbmonit}

1. 创建一个服务账户、一个只能获取和列出节点、Pod、工作负载、卷声明和事件的 ClusterRole,并把它们绑定在一起。该角色不读取任何 Secret 和任何 ConfigMap。在 k3s 服务器上,请把每条命令都以 sudo k3s kubectl 运行。

    ```
    kubectl create serviceaccount dumbmonit -n kube-system
    kubectl create clusterrole dumbmonit-read --verb=get,list --resource=nodes,pods,persistentvolumeclaims,events,deployments.apps,statefulsets.apps,daemonsets.apps
    kubectl create clusterrolebinding dumbmonit-read --clusterrole=dumbmonit-read --serviceaccount=kube-system:dumbmonit
    ```

2. 为它配置一个不会过期的令牌:一个 service-account-token 类型的 Secret,Kubernetes 会自动填充它。然后打印该令牌,以及 base64 编码的集群 CA。

    ```
    echo '{"apiVersion":"v1","kind":"Secret","metadata":{"name":"dumbmonit-token","namespace":"kube-system","annotations":{"kubernetes.io/service-account.name":"dumbmonit"}},"type":"kubernetes.io/service-account-token"}' | kubectl apply -f -
    kubectl -n kube-system get secret dumbmonit-token -o jsonpath='{.data.token}' | base64 -d
    kubectl -n kube-system get secret dumbmonit-token -o jsonpath='{.data.ca\.crt}'
    ```

3. 在 DumbMonit 中,输入 API 服务器的地址,例如 "https://k3s.lan:6443"。将该令牌粘贴为 Service account token,把 base64 的 CA 粘贴为 Cluster CA certificate:此后只会信任这一个 CA。一个 API 服务器就能报告整个集群的情况。

!!! warning "警告"
    API 服务器的证书只列出它自己的名称和地址:一个它没有列出的
    地址会导致验证失败。请使用它列出的地址之一(某个节点的 IP
    地址,或通过 --tls-san 在 k3s 中添加的名称),或勾选"接受
    无法验证的证书"。

### 以清单文件实现同样的效果 {#the-same-thing-as-a-manifest}

对于 GitOps,或为了准确审查授予了哪些权限,可以应用以下清单
文件,而不是执行三条 `kubectl create` 命令和第 2 步中的 Secret
(`kubectl apply -f dumbmonit.yaml`),然后像第 2 步那样打印令牌
和 CA:

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

在添加该设备之前检查该角色,可以向 API 服务器询问该账户能做
什么;第一条命令必须应答 `yes`,第二条必须应答 `no`:

```
kubectl auth can-i list pods --as=system:serviceaccount:kube-system:dumbmonit
kubectl auth can-i list secrets --as=system:serviceaccount:kube-system:dumbmonit
```

用 `kubectl create token dumbmonit -n kube-system` 生成的令牌
同样可用,但它会过期(默认一小时后):此时该设备会失败,并显示
"The API server refused the token (401)"。上面的 Secret 是
Kubernetes 文档为此用途推荐的长期有效形式。若要撤销 DumbMonit
的访问权限,请删除该 Secret,或删除该服务账户。

## 凭据 {#credentials}

| 凭据 | 字段 |
|---|---|
| 服务账户令牌 | 第一条 `kubectl get secret` 命令打印的令牌,以 `Authorization: Bearer` 发送。 |

地址:API 服务器的 URL(`https://k3s.lan:6443`)、主机名或 IP
(`192.0.2.10`,此时端口为 6443),或 `host:port`。位于负载均衡器
之后或高可用的 k3s 之后时,请提供服务该 API 的地址,即你
kubeconfig 中使用的那个。

选项:

- **端口**(6443),当地址中未给出时使用。
- **集群 CA 证书**:`ca.crt` 的 base64 值,即第 2 步最后一条命令
  打印的内容;一个 PEM 证书也可以,即使粘贴成一行也行。有了它,
  DumbMonit 只会信任这个 CA,不信任其他任何证书。
- **接受无法验证的证书**:跳过所有检查,适用于你信任的网络。
- **每次请求的超时时间**(10 秒)。
- **忽略的命名空间**:这些命名空间中的 Pod、工作负载、声明和
  事件会被排除在外,例如一个充满短生命周期 Pod 的 CI 命名空间。
  节点事件始终会被保留。

## 故障排查 {#troubleshooting}

"The certificate of the API server could not be verified":要么
CA 缺失(请像第 2 步那样粘贴它),要么该地址不在服务器证书所列
的名称之中。k3s 列出 `kubernetes`、`localhost`、节点的主机名及
其 IP 地址;可用 `--tls-san` 添加另一个名称(在
`/etc/rancher/k3s/config.yaml` 中:`tls-san: [k3s.lan]`),然后
重启 k3s。

"The service account may not read … (403)":该 ClusterRole 或其
绑定缺失或不完整。错误消息会指明 API 服务器拒绝的是哪个资源;
请再次应用上面的清单文件。
