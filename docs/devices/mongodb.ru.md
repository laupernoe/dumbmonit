# MongoDB

Сама документная база данных: replica set (участники, их состояние,
отсутствующий primary, насколько каждый secondary отстаёт), подключения
относительно лимита, кэш WiredTiger, операции и ассерты.

DumbMonit входит с учётной записью, обладающей только встроенной ролью
`clusterMonitor`, выполняет `serverStatus` и `replSetGetStatus`, когда сервер
принадлежит replica set. Эта роль читает состояние сервера и репликации, и ни
одного документа ни из одной базы данных. Поддерживается MongoDB 3.6 и
новее; DumbMonit сам говорит по wire-протоколу, без драйвера.

Отказы, за которыми стоит следить: replica set, потерявший primary, не
принимает запись, пока не пройдут выборы; secondary, который отстаёт, — это
тот, кого выберут в день смерти primary, без последних записей; на пределе
подключений новые клиенты отклоняются; при более чем 20 % «грязного» кэша
WiredTiger запросам приходится вытеснять страницы самостоятельно, и всё
замедляется.

## Что отслеживается

Все метрики с префиксом `dumbmonit_mongodb_`. Суммарные значения — это
счётчики: графики и правила превращают их в скорости.

| Метрика | Что | Метки |
|---|---|---|
| `version_info` | значение 1 | `version`, `process`, `storage_engine` |
| `role_info` | значение 1 | `role`: `primary`, `secondary`, `arbiter`, `standalone` |
| `uptime_seconds` | | |
| `connections_current`, `connections_available`, `connections_active` | | |
| `connections_used_percent` | текущие относительно текущих плюс доступных | |
| `connections_created_total`, `connections_rejected_total` | | |
| `operations_total` | операции с момента запуска | `op`: `insert`, `query`, `update`, `delete`, `getmore`, `command` |
| `asserts_total` | ассерты с момента запуска; `regular` — внутренние ошибки сервера, `user` — ошибки, возвращённые клиентам | `type` |
| `queued_operations`, `active_clients` | операции, ожидающие блокировки; клиенты, выполняющие хотя бы одну | |
| `wiredtiger_cache_max_bytes`, `wiredtiger_cache_used_bytes`, `wiredtiger_cache_used_percent` | кэш WiredTiger и насколько он заполнен | |
| `wiredtiger_cache_dirty_bytes`, `wiredtiger_cache_dirty_percent` | изменённые страницы, ещё не записанные | |
| `wiredtiger_cache_read_bytes_total` | байты, прочитанные с диска в кэш | |
| `memory_resident_bytes` | | |
| `network_bytes_in_total`, `network_bytes_out_total` | | |
| `cursors_open`, `cursors_timed_out_total`, `documents_deleted_total` | | |
| `replset_info` | значение 1, в replica set | `set`, `members` |
| `replset_primary_present` | 1, когда есть primary, 0 — когда нет | |
| `replset_member_state` | номер состояния участника | `member`, `state`: `primary`, `secondary`, `arbiter`, `recovering`, `down`… |
| `replset_member_health` | 1, когда участник доступен, 0 — когда нет | `member` |
| `replset_member_lag_seconds` | насколько secondary отстаёт от primary | `member` |

Применяемые [встроенные правила](../alerting/rules.md#databases-message-broker-and-security-engine):
«Replica set MongoDB без основного узла», «Участник MongoDB недоступен»,
«Отставание репликации MongoDB», «Подключения MongoDB близки к лимиту»,
«Кэш MongoDB под нагрузкой», «Внутренние ошибки MongoDB», плюс «Устройство
недоступно».

## Страница устройства

Панель над графиками читает то, что сохранила проверка; открытие страницы
никогда не запрашивает MongoDB. Она сообщает, что не так, в одном предложении
(replica set, подключения, кэш, внутренние ошибки), затем — подключения,
операции в секунду, занятость кэша, резидентную память, операции в очереди и
время работы, затем — состояние и отставание каждого участника replica set и
операции в секунду по типу.

## Создание учётной записи MongoDB с ролью clusterMonitor

1. В mongosh, войдя пользователем, которому разрешено создавать пользователей, создайте учётную запись только с встроенной ролью clusterMonitor. Она читает состояние сервера и репликации, и ни одного документа ни из одной базы данных.

    ```
    use admin
    db.createUser({user: "dumbmonit", pwd: passwordPrompt(), roles: [{role: "clusterMonitor", db: "admin"}]})
    ```

2. DumbMonit входит через SCRAM-SHA-256, значение по умолчанию для учётных записей, созданных на MongoDB 4.0 и новее. Если вы создали учётную запись в другой базе данных, кроме admin, задайте Authentication database ниже.

3. В DumbMonit введите адрес одного mongod, например «db1.lan» или «db1.lan:27018». В replica set добавьте каждого участника: каждый сообщает о своих собственных подключениях и кэше, и любой из них сообщает состояние и отставание всех участников. Если сервер требует TLS, отметьте TLS ниже.

4. DumbMonit выполняет только serverStatus и replSetGetStatus. Он никогда не перечисляет коллекцию и никогда не читает документ.

!!! warning
    Адрес mongodb+srv:// не резолвится: вместо этого введите хост каждого участника. MongoDB старше 3.6 и роутеры mongos не поддерживаются.

## Учётные данные

| Учётные данные | Поля |
|---|---|
| Имя пользователя / пароль | Учётная запись `clusterMonitor`, созданная выше, вход через SCRAM-SHA-256. |
| Без аутентификации | Для сервера, запущенного без контроля доступа. |

Учётная запись, имеющая только учётные данные SCRAM-SHA-1 (созданная до
MongoDB 4.0 и никогда не обновлявшаяся), отклоняется: обновите её пароль один
раз с помощью
`db.updateUser("dumbmonit", {pwd: passwordPrompt(), mechanisms: ["SCRAM-SHA-256"]})`.

Адрес: имя хоста или IP (`db1.lan`), `host:port`, или строка подключения
`mongodb://` с одним хостом. Опции: порт (27017), база данных аутентификации
(`admin`), TLS (выключено), проверка сертификата, и таймаут всей проверки
(10 с).
