# Kubernetes / k3s

Кластер через его API-сервер: узлы, которые не Ready или испытывают давление,
поды в цикле сбоев (crash loop) или застрявшие в Pending, Deployments,
StatefulSets и DaemonSets с недостающими репликами, запросы томов, которые так
и не привязываются, и события-предупреждения за последний час.

DumbMonit читает API с токеном выделенного сервисного аккаунта, привязанного к
ClusterRole только для чтения: `get` и `list` для узлов, подов, рабочих нагрузок,
запросов постоянных томов и событий, ничего больше. Он никогда не пишет, а роль
не даёт доступа к Secrets или ConfigMaps. Один монитор охватывает весь
кластер, независимо от дистрибутива: k3s, kubeadm, k0s, MicroK8s, Talos или
управляемый кластер, до API которого DumbMonit может дотянуться.

Каждую минуту DumbMonit параллельно читает восемь коллекций: версию, узлы, поды,
Deployments, StatefulSets и DaemonSets, запросы постоянных томов и события типа
`Warning` (отфильтрованные API-сервером). Длинные списки читаются страницами по
500.

## Что отслеживается

Все метрики имеют префикс `dumbmonit_k8s_`.

| Метрика | Что | Метки |
|---|---|---|
| `version_info` | значение 1 | `version` (`1.33.4+k3s1`) |
| `nodes`, `nodes_ready` | узлы и те, у которых условие `Ready` истинно | |
| `node_ready` | 1, когда узел Ready | `node` |
| `node_pressure` | 1, когда узел сообщает о таком давлении; только условия, которые узел объявляет | `node`, `condition`: `memory`, `disk`, `pid`, `network` |
| `node_unschedulable` | 1, когда узел закрыт для планирования (cordoned) | `node` |
| `node_info` | значение 1 | `node`, `kubelet_version` |
| `pods`, `pods_running`, `pods_pending`, `pods_failed`, `pods_crashlooping`, `pods_not_ready` | количества подов; завершённые поды (`Succeeded`) не учитываются | |
| `pod_ready`, `pod_crashlooping`, `pod_pending` | 1 или 0 на под | `namespace`, `pod`, `workload` |
| `pod_restarts` | перезапуски контейнеров пода, счётчик | `namespace`, `pod`, `workload` |
| `pod_blocked` | 1, пока контейнер ждёт по причине, отличной от запуска: `ImagePullBackOff`, `CreateContainerConfigError`… | `namespace`, `pod`, `workload`, `reason` |
| `workload_desired`, `workload_ready`, `workload_unavailable` | запрошенные, готовые и недостающие реплики; для DaemonSet — узлы | `kind` (`Deployment`, `StatefulSet`, `DaemonSet`), `namespace`, `workload` |
| `pvc_pending` | 1, когда запрос в состоянии Pending **и** он нужен какому-то поду или его касается событие-предупреждение | `namespace`, `pvc` |
| `pvc_bound` | 1, когда запрос Bound | `namespace`, `pvc` |
| `pvcs`, `pvcs_pending` | запросы и те, что застряли, как описано выше | |
| `warning_events` | события-предупреждения, замеченные за последний час | |
| `warning_events_by_reason` | то же по причинам (`BackOff`, `FailedScheduling`, `ProvisioningFailed`…), не более двадцати причин | `reason` |

`workload` — это контроллер пода: Deployment за его ReplicaSet (`api` для
`api-7c9696d669-8tpp6`), StatefulSet, DaemonSet или пусто для одиночного пода.
Поды описываются по одному до 500, сначала проблемные; количества всегда
охватывают каждый под.

Запрос, который просто `Pending`, — не неисправность: класс хранилища
`WaitForFirstConsumer`, который поставляет k3s (`local-path`), оставляет запрос
в Pending, пока его не использует первый под. Он считается застрявшим только
тогда, когда его требует какой-то под или его касается предупреждение вроде
`ProvisioningFailed`.

Применяемые [встроенные правила](../alerting/rules.md#kubernetes):

- **Kubernetes node not ready**: условие `Ready` узла ложно или неизвестно в
  течение пяти минут (Warning).
- **Kubernetes node under pressure**: давление по памяти, диску или PID в
  течение пяти минут (Advisory).
- **Kubernetes pod crash looping**: контейнер в `CrashLoopBackOff` в течение
  десяти минут (Warning). Некоторые kubelet (k3s 1.37) между двумя попытками
  показывают контейнер как `terminated` с причиной `Error` и никогда не
  `CrashLoopBackOff`: контейнер, перезапущенный как минимум дважды и чья
  последняя остановка в пределах десяти минут завершилась ненулевым кодом
  выхода, тоже считается зацикленным. Между двумя попытками контейнер
  перезапускается, и состояние на мгновение исчезает; правило читает максимум за
  десять минут, чтобы не начинать отсчёт заново каждый раз.
- **Kubernetes pod stuck pending**: под в Pending в течение пятнадцати минут
  (Advisory).
- **Kubernetes pod restarting**: более пяти перезапусков за час для пода,
  который ещё не находится в crash loop (Advisory).
- **Kubernetes workload missing replicas**: Deployment, StatefulSet или
  DaemonSet с меньшим числом доступных реплик, чем запрошено, в течение десяти
  минут (Advisory, эскалация через один час). Плавное обновление убирает одну
  реплику, пока запускает другую; десять минут это покрывают.
- **Kubernetes volume claim pending**: застрявший запрос в течение пятнадцати
  минут (Advisory).

Плюс Monitor unreachable, когда API-сервер перестаёт отвечать.

## Страница монитора

Панель Cluster одной фразой говорит, что не так, затем перечисляет узлы (Ready
или нет, давление, cordoned, версия kubelet), поды, требующие внимания (crash
loop, pending, blocked, not ready, с их перезапусками), рабочие нагрузки с
недостающими репликами, застрявшие запросы томов и события-предупреждения
последнего часа по причинам. Здоровый кластер показывает свои количества и
ничего больше.

## Создайте сервисный аккаунт только для чтения для DumbMonit

1. Создайте сервисный аккаунт, ClusterRole, которая может только выполнять get и list для узлов, подов, рабочих нагрузок, запросов томов и событий, и свяжите их. Роль не читает ни Secret, ни ConfigMap. На сервере k3s выполняйте каждую команду как sudo k3s kubectl.

    ```
    kubectl create serviceaccount dumbmonit -n kube-system
    kubectl create clusterrole dumbmonit-read --verb=get,list --resource=nodes,pods,persistentvolumeclaims,events,deployments.apps,statefulsets.apps,daemonsets.apps
    kubectl create clusterrolebinding dumbmonit-read --clusterrole=dumbmonit-read --serviceaccount=kube-system:dumbmonit
    ```

2. Выдайте ему токен, который не истекает: Secret типа service-account-token, который заполняет Kubernetes. Затем выведите токен и CA кластера в base64.

    ```
    echo '{"apiVersion":"v1","kind":"Secret","metadata":{"name":"dumbmonit-token","namespace":"kube-system","annotations":{"kubernetes.io/service-account.name":"dumbmonit"}},"type":"kubernetes.io/service-account-token"}' | kubectl apply -f -
    kubectl -n kube-system get secret dumbmonit-token -o jsonpath='{.data.token}' | base64 -d
    kubectl -n kube-system get secret dumbmonit-token -o jsonpath='{.data.ca\.crt}'
    ```

3. В DumbMonit введите адрес API-сервера, например "https://k3s.lan:6443". Вставьте токен как Service account token, а CA в base64 — как Cluster CA certificate: тогда доверяется только этот CA. Один API-сервер сообщает о всём кластере.

!!! warning
    Сертификат API-сервера перечисляет только его собственные имена и адреса: адрес, которого в нём нет, не проходит проверку. Используйте тот, что в нём перечислен (IP-адрес узла или имя, добавленное через --tls-san в k3s), или отметьте Accept an unverifiable certificate.

### То же самое в виде манифеста

Для GitOps или чтобы точно просмотреть, что выдаётся, примените этот манифест
вместо трёх команд `kubectl create` и Secret из шага 2
(`kubectl apply -f dumbmonit.yaml`), затем выведите токен и CA, как в шаге 2:

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

Чтобы проверить роль до добавления монитора, спросите у API-сервера, что
может делать аккаунт; первая команда должна ответить `yes`, вторая — `no`:

```
kubectl auth can-i list pods --as=system:serviceaccount:kube-system:dumbmonit
kubectl auth can-i list secrets --as=system:serviceaccount:kube-system:dumbmonit
```

Токен, созданный командой `kubectl create token dumbmonit -n kube-system`, тоже
работает, но истекает (по умолчанию через один час): тогда монитор даёт сбой
с сообщением "The API server refused the token (401)". Приведённый выше Secret —
это долгоживущая форма, которую Kubernetes документирует для такого случая.
Чтобы отозвать доступ DumbMonit, удалите Secret или сервисный аккаунт.

## Учётные данные

| Учётные данные | Поля |
|---|---|
| Токен сервисного аккаунта | Токен, выведенный первой командой `kubectl get secret`, отправляется как `Authorization: Bearer`. |

Адрес: API-сервер в виде URL (`https://k3s.lan:6443`), имя хоста или IP
(`192.0.2.10`, тогда порт 6443) или `host:port`. За балансировщиком нагрузки или
в высокодоступном k3s укажите адрес, который обслуживает API, тот, что в вашем
kubeconfig.

Опции:

- **Port** (6443), когда адрес его не задаёт.
- **Cluster CA certificate**: значение `ca.crt` в base64, как его выводит
  последняя команда шага 2; PEM-сертификат тоже подходит, даже вставленный в одну
  строку. С ним DumbMonit доверяет этому CA и никакому другому.
- **Accept an unverifiable certificate**: пропускает все проверки, для сети,
  которой вы доверяете.
- **Timeout per request** (10 с).
- **Ignored namespaces**: поды, рабочие нагрузки, запросы и события этих
  пространств имён не учитываются, например пространства имён CI, полного
  недолговечных подов. События узлов сохраняются всегда.

## Устранение неполадок

"The certificate of the API server could not be verified": либо отсутствует CA
(вставьте его, как в шаге 2), либо адрес не входит в число имён в сертификате
сервера. k3s перечисляет `kubernetes`, `localhost`, имя хоста узла и его
IP-адреса; добавьте другое имя через `--tls-san` (в
`/etc/rancher/k3s/config.yaml`: `tls-san: [k3s.lan]`) и перезапустите k3s.

"The service account may not read … (403)": ClusterRole или её привязка
отсутствует или неполна. Сообщение называет ресурс, который API-сервер
отклонил; примените манифест выше ещё раз.
