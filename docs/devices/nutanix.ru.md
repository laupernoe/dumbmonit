# Nutanix AHV

Кластеры Nutanix, считываемые через REST API v4 Prism Central. Один Prism
Central охватывает все зарегистрированные в нём кластеры.

Каждую минуту DumbMonit читает, с базовой HTTP-аутентификацией на порту 9440:

| Что | Путь |
|---|---|
| Кластеры | `/api/clustermgmt/v4.0/config/clusters` |
| Хосты | `/api/clustermgmt/v4.0/config/hosts` |
| ВМ AHV | `/api/vmm/v4.0/ahv/config/vms` |
| Контейнеры хранилища | `/api/clustermgmt/v4.0/config/storage-containers` |
| Уровень заполнения каждого контейнера | `/api/clustermgmt/v4.0/stats/storage-containers/{extId}` |
| Неустранённые оповещения | `/api/monitoring/v4.0/serviceability/alerts` |

Списки читаются по 100 элементов на страницу, до 1000 элементов каждый.
Уровень заполнения и оповещения необязательны: если роль учётной записи их
не разрешает, остальное всё равно собирается. Nutanix объявила API v1, v2.0 и
v3 устаревшими; развивается только v4, и ей нужен Prism Central pc.2024.3 или
новее.

!!! warning "Проверено только по документации производителя — пока не испытано на реальной системе"

    Эта интеграция построена на спецификациях API v4 от Nutanix (пути и поля
    в том виде, как их определяют официальные сгенерированные клиенты) и
    проверена на записанных ответах, а не на работающем Prism Central.
    Сообщите нам, если что-то не работает.

## Что отслеживается

Все метрики с префиксом `dumbmonit_nutanix_`.

| Метрика | Метки | Значение |
|---|---|---|
| `clusters` | | Кластеры, без самого Prism Central. |
| `cluster_info` | `cluster`, `version` | Значение 1: версия AOS каждого кластера. |
| `cluster_available` | `cluster` | 1, когда Prism Central достигает кластера. |
| `hosts` | | Хосты всех кластеров. |
| `host_info` | `host`, `cluster`, `hypervisor` | Значение 1: гипервизор, который запущен на каждом хосте. |
| `host_cpu_cores`, `host_memory_bytes` | `host`, `cluster` | Ёмкость. |
| `host_degraded` | `host`, `cluster` | 1, когда Nutanix помечает хост как деградировавший. |
| `host_maintenance` | `host`, `cluster` | 1, когда хост входит в режим обслуживания или находится в нём. |
| `vms`, `vms_running` | | ВМ AHV и сколько из них включено. |
| `vm_power_state_info` | `vm`, `cluster`, `state` | Значение 1: `on`, `off`, `paused` или `undetermined`. |
| `vm_running` | `vm`, `cluster` | 1, когда ВМ включена. |
| `vm_vcpus`, `vm_memory_bytes` | `vm`, `cluster` | Сокеты × ядра × потоки и настроенная память. |
| `storage_containers` | | Контейнеры хранилища, без внутренних контейнеров Nutanix. |
| `storage_container_capacity_bytes`, `storage_container_used_bytes`, `storage_container_used_percent` | `container`, `cluster` | Уровень заполнения по последней статистике за последние тридцать минут. |
| `alerts_unresolved`, `alerts_critical`, `alerts_warning` | | Неустранённые оповещения: все и по важности. |

Применяемые [встроенные правила](../alerting/rules.md#nutanix):

- **Nutanix host degraded**: хост помечен как деградировавший пять минут
  (Warning).
- **Nutanix storage container almost full**: контейнер заполнен на 90 %
  пятнадцать минут (Advisory).
- **Nutanix critical alerts**: Prism Central сообщает хотя бы об одном
  неустранённом критическом оповещении пять минут (Warning).

## Создайте пользователя только для чтения в Prism Central

1. В Prism Central откройте Admin Center > IAM > Identities и добавьте локального пользователя с длинным случайным паролем, с таким именем.

    ```
    dumbmonit
    ```

2. В разделе Admin Center > IAM > Authorization Policies создайте политику со встроенной ролью Prism Viewer для всех сущностей и назначьте её этому пользователю. Prism Viewer читает кластеры, хосты, ВМ, контейнеры хранилища и оповещения и ничего не изменяет.

3. В DumbMonit введите адрес Prism Central, например «prism-central.lan», с этим именем пользователя и паролем. Один Prism Central сообщает обо всех зарегистрированных в нём кластерах.

4. DumbMonit только читает API v4 (clustermgmt, vmm и monitoring, версия 4.0): он никогда не включает и не выключает ВМ, никогда не закрывает оповещение и никогда не изменяет контейнер хранилища.

!!! warning
    API v4 предоставляет Prism Central pc.2024.3 или новее; кластер, управляемый только Prism Element или более старым Prism Central, не поддерживается. Prism Central поставляется с самоподписанным сертификатом: отметьте Accept an unverifiable certificate, если хост DumbMonit не доверяет его центру сертификации.

## Учётные данные

| Учётные данные | Поля |
|---|---|
| Имя пользователя и пароль | Локальный пользователь `dumbmonit` с ролью Prism Viewer. |

Адрес: имя хоста или IP Prism Central, `host:port` или URL. Параметры:

- **Порт** (9440).
- **Принимать непроверяемый сертификат**: собственный сертификат Prism Central
  самоподписан.
- **Тайм-аут на запрос** (15 с).

## Диагностика

«Nutanix Prism Central refused the credentials … (401)»: неверное имя
пользователя или пароль.

«Nutanix Prism Central has no /api/clustermgmt/v4.0/…»: указан адрес Prism
Element или Prism Central старше pc.2024.3.

Нет `storage_container_used_percent`: роль не разрешает читать статистику;
контейнеры всё равно учитываются, с их настроенной ёмкостью.
