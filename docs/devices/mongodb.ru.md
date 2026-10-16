# MongoDB

Сама документная база данных: набор реплик (участники, их состояние, отсутствие
primary, насколько каждый secondary отстаёт), соединения относительно лимита,
кэш WiredTiger, операции и assertions.

DumbMonit входит под учётной записью, которая имеет только встроенную роль
`clusterMonitor`, выполняет `serverStatus`, а также `replSetGetStatus`, если
сервер входит в набор реплик. Эта роль читает состояние сервера и репликации и
не даёт доступа ни к одному документу ни одной базы. Поддерживаются MongoDB 3.6
и новее; DumbMonit сам реализует wire-протокол, без драйвера.

За какими сбоями стоит следить: набор реплик, потерявший primary, не принимает
запись, пока выборы не завершатся успешно; отставший secondary — именно тот,
который будет избран в день, когда primary умрёт, и без последних записей; при
достижении лимита соединений новые клиенты отклоняются; если более 20% кэша
WiredTiger «грязные», запросам приходится самим вытеснять страницы, и всё
замедляется.

## За чем он следит

Все метрики имеют префикс `dumbmonit_mongodb_`. Суммарные значения — счётчики:
графики и правила превращают их в скорости.

| Метрика | Что | Метки |
|---|---|---|
| `version_info` | значение 1 | `version`, `process`, `storage_engine` |
| `role_info` | значение 1 | `role`: `primary`, `secondary`, `arbiter`, `standalone` |
| `uptime_seconds` | | |
| `connections_current`, `connections_available`, `connections_active` | | |
| `connections_used_percent` | текущие относительно текущих плюс доступных | |
| `connections_created_total`, `connections_rejected_total` | | |
| `operations_total` | операции с момента запуска | `op`: `insert`, `query`, `update`, `delete`, `getmore`, `command` |
| `asserts_total` | assertions с момента запуска; `type` — это `regular` (внутренние ошибки сервера), `warning`, `msg`, `user` (ошибки, возвращённые клиентам), `tripwire` или `rollovers`. Графика заслуживают `regular` и `user` | `type` |
| `queued_operations`, `active_clients` | операции, ожидающие блокировки, и клиенты, выполняющие операцию | |
| `wiredtiger_cache_max_bytes`, `wiredtiger_cache_used_bytes`, `wiredtiger_cache_used_percent` | кэш WiredTiger и степень его заполнения | |
| `wiredtiger_cache_dirty_bytes`, `wiredtiger_cache_dirty_percent` | изменённые страницы, ещё не записанные | |
| `wiredtiger_cache_read_bytes_total` | байты, прочитанные с диска в кэш | |
| `memory_resident_bytes` | | |
| `network_bytes_in_total`, `network_bytes_out_total` | | |
| `cursors_open`, `cursors_timed_out_total`, `documents_deleted_total` | | |
| `replset_info` | значение 1, в наборе реплик | `set`, `members` |
| `replset_primary_present` | 1, когда есть primary, 0, когда его нет | |
| `replset_member_state` | номер состояния участника | `member`, `state`: `primary`, `secondary`, `arbiter`, `recovering`, `down`… |
| `replset_member_health` | 1, когда участник доступен, 0, когда нет | `member` |
| `replset_member_lag_seconds` | насколько secondary отстаёт от primary | `member` |

Применимые [встроенные правила](../alerting/rules.md#databases-message-broker-and-security-engine):
MongoDB replica set without primary, MongoDB member unreachable,
MongoDB replication lag, MongoDB connections near the limit, MongoDB cache
under pressure, MongoDB internal errors, а также Monitor unreachable.

## Страница монитора

Панель над графиками показывает то, что сохранил опрос; открытие страницы
никогда не обращается к MongoDB. Она одной фразой говорит, что не так (набор
реплик, соединения, кэш, внутренние ошибки), затем показывает соединения,
операции в секунду, использованный кэш, резидентную память, операции в очереди
и время работы, затем состояние и отставание каждого участника набора реплик и
операции в секунду по типам.

## Создание учётной записи MongoDB с ролью clusterMonitor

1. В mongosh, войдя под пользователем, имеющим право создавать пользователей, создайте учётную запись только со встроенной ролью clusterMonitor. Она читает состояние сервера и репликации и не даёт доступа ни к одному документу ни одной базы.

    ```
    use admin
    db.createUser({user: "dumbmonit", pwd: passwordPrompt(), roles: [{role: "clusterMonitor", db: "admin"}]})
    ```

2. DumbMonit входит через SCRAM-SHA-256 — механизм по умолчанию для учётных записей, созданных в MongoDB 4.0 и новее. Если вы создали учётную запись не в базе admin, укажите Authentication database ниже.

3. В DumbMonit введите адрес одного mongod, например "db1.lan" или "db1.lan:27018". В наборе реплик добавьте каждого участника: каждый сообщает о своих соединениях и кэше, а любой из них сообщает состояние и отставание всех участников. Если сервер требует TLS, отметьте TLS ниже.

4. DumbMonit выполняет только serverStatus и replSetGetStatus. Он никогда не перечисляет коллекции и никогда не читает документы.

!!! warning
    Адрес mongodb+srv:// не разрешается: вместо него введите хост каждого участника. MongoDB старше 3.6 и маршрутизаторы mongos не поддерживаются.

## Учётные данные

| Учётные данные | Поля |
|---|---|
| Имя пользователя / пароль | Учётная запись `clusterMonitor`, созданная выше, вход через SCRAM-SHA-256. |
| Без аутентификации | Для сервера, запущенного без контроля доступа. |

Учётная запись, у которой есть только учётные данные SCRAM-SHA-1 (созданная до
MongoDB 4.0 и ни разу не обновлявшаяся), отклоняется: один раз обновите её
пароль командой
`db.updateUser("dumbmonit", {pwd: passwordPrompt(), mechanisms: ["SCRAM-SHA-256"]})`.

Адрес: имя хоста или IP (`db1.lan`), `host:port` или строка подключения
`mongodb://` с единственным хостом. Параметры: порт (27017), база данных
аутентификации (`admin`), TLS (выключено), проверка сертификата и тайм-аут всей
проверки (10 с).
