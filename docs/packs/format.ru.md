# Справочник по формату пака

Пак — это один YAML-файл `pack.yaml` в формате версии 1. Неизвестные ключи —
это ошибки, а не игнорируемые настройки: опечатка вроде `treshold` будет
показана.

```text
my-pack/
  pack.yaml          the pack itself: the only file the server needs
  fixtures/          one captured response per source, named after its id
    status.json
  expected.prom      what the extraction must produce from the fixtures
```

## Заголовок

| Ключ | Обязателен | Значение |
|---|---|---|
| `schema` | да | Всегда `1`. |
| `id` | да | Строчные буквы, цифры и одиночные дефисы, начинается с буквы, до 40 символов: `shelly-plug`. Тип устройства становится `pack.<id>`, префикс метрик — `dumbmonit_<id with underscores>_`. |
| `version` | да | [Семантическая версия](https://semver.org) пака: `1.2.0`. |
| `requires` | нет | Версии DumbMonit, с которыми работает пак, в виде требования semver: `">=0.2"`. Сервер вне этого диапазона отклоняет пак. |
| `label` | да | Название, показываемое в списке типов устройств. |
| `summary` | нет | Одно предложение: что отслеживает пак. |
| `examples` | нет | Продукты, к которым он применим, показываются как подсказки. |
| `address_hint` | нет | Плейсхолдер поля адреса: `192.168.1.50`. |
| `default_port` | нет | Порт, используемый, если в адресе он не указан. |
| `scheme` | нет | `http` (по умолчанию) или `https`; устройство может переопределить его. |
| `setup` | нет | Уведомление, показываемое рядом с формой устройства: `title`, `steps` (список; то, что идёт после перевода строки в шаге, показывается как копируемая команда), `warning`, `doc_url`. |
| `credentials` | нет | Допустимые типы учётных данных в порядке предпочтения: `none`, `api_token`, `username_password`. По умолчанию: `[none]`. Элементом может быть `{kind, label, help}`, чтобы изменить формулировки формы. |
| `options` | нет | Настройки, показываемые в форме устройства (ниже). |

## Опции

Опции сохраняются в устройстве и читаются через `{{option.<key>}}` в путях,
заголовках и телах запросов, а также в порогах правил.

```yaml
options:
  - key: switch_id          # lowercase, digits, underscores
    label: Switch channel
    help: 0 for a plug; 0, 1, 2… for each channel of a relay.
    input: number           # text (default), number, boolean or select
    default: 0
    required: false
    placeholder: ""
    choices: []             # the values of a select
```

Каждый пак, который опрашивает устройство, также получает пять опций
соединения, которые нельзя переобъявить: `scheme`, `port`, `insecure_tls`,
`request_timeout_seconds` (от 1 до 120, по умолчанию 10) и
`allow_private_targets`.

## Источники

Источник — это один запрос за опрос, всегда отправляемый на собственный адрес
устройства.

```yaml
sources:
  - id: status              # lowercase, digits, underscores
    type: http              # http or prometheus
    path: /rpc/Switch.GetStatus?id={{option.switch_id}}
    method: GET             # GET (default) or POST
    format: json            # http only: json (default) or text
    headers:
      Accept: application/json
    body: '{"id": {{option.switch_id}}}'   # POST only
    auth: auto              # auto (default), none, bearer or basic
```

- `path` должен начинаться с одного `/`. URL запроса — это адрес устройства,
  за которым следует путь; после подстановки он должен сохранить ту же схему,
  хост и порт, иначе запрос отклоняется. Значения опций кодируются в путях
  с помощью процентного кодирования, поэтому значение не может добавить
  сегмент или изменить хост.
- `auth: auto` отправляет учётные данные `api_token` как
  `Authorization: Bearer …`, а учётные данные `username_password` — как базовую
  аутентификацию. `none` ничего не отправляет неявно, поэтому учётные данные
  можно передать в заголовке: `X-Api-Key: "{{credential.token}}"`.
  `{{credential.token}}`, `{{credential.username}}` и `{{credential.password}}`
  допускаются **только в заголовках**: никогда в пути или теле, которые
  попадают в логи.
- Редиректы выполняются только на ту же схему, хост и порт, не более трёх.
  `401` и `403` сообщаются как ошибки аутентификации, другие статусы не 2xx —
  как ошибки протокола.
- За один опрос выполняется не более 16 запросов, и читается не более 4 MiB
  на ответ.

### Источники Prometheus

```yaml
sources:
  - id: metrics
    type: prometheus
    path: /metrics
    keep: [node_load1, node_memory_MemAvailable_bytes]
    rename:
      node_memory_MemAvailable_bytes: memory_available_bytes
    drop:
      fstype: ^(tmpfs|overlay)$
```

- `keep` перечисляет семейства, импортируемые как есть, со всеми их метками.
  Имя семейства также охватывает его серии histogram и summary (`_bucket`,
  `_sum`, `_count`). Счётчики (counters), гистограммы и суммы summary остаются
  счётчиками; всё остальное — gauge.
- `rename` задаёт сохранённому семейству имя в DumbMonit. Имя, не состоящее из
  строчных букв, цифр и подчёркиваний, необходимо переименовать.
- `drop` отбрасывает каждый сэмпл, чья метка соответствует регулярному
  выражению.
- Метки, которые DumbMonit задаёт сам (`target`, `host`, `instance`, `job`,
  `tag_*`…), сохраняются как `exported_<name>`.

## Метрики

У каждой метрики имя *без* префикса: `power_watts` в паке `shelly-plug`
записывается как `dumbmonit_shelly_plug_power_watts`. Имена состоят из
строчных букв, цифр и подчёркиваний и уникальны в пределах пака.

```yaml
metrics:
  - name: power_watts
    help: Active power drawn by the load.
    source: status
    kind: gauge             # gauge (default) or counter
    json: $.apower          # exactly one of json, regex or prom
    scale: 1                # multiplier, e.g. 0.001 for ms → s
    map: {on: 1, off: 0}    # text value → number
    labels: {}
```

| Экстрактор | Источник | Значение | `labels` |
|---|---|---|---|
| `json: <JSONPath>` | `http`, формат `json` | Каждый узел, подходящий под путь ([RFC 9535](https://www.rfc-editor.org/rfc/rfc9535) JSONPath, включая подстановочные знаки и фильтры), даёт одну серию. | метка → JSONPath, вычисляемый на объекте, который *содержит* значение: `$.sensors[*].value` с `sensor: $.name` помечает каждое значение соседним полем `name`. |
| `regex: <expression>` | `http`, формат `text` | Каждое совпадение даёт одну серию; значение — это группа с именем `value`, иначе первая группа. | метка → имя группы захвата. |
| `prom: <sample name>` | `prometheus` | Каждый сэмпл с точно таким именем. | метка → метка источника. Пусто: сохраняются все метки источника. |

Значения: числа берутся как есть; `true`/`false` становятся 1/0; текст
ищется в `map` или разбирается как число, если `map` нет. Значение, которое
отсутствует, равно `null` или не входит в `map`, не даёт в этом опросе ничего.

## Discover

`discover` превращает каждый элемент списка в набор серий с общими метками:
одна строка на диск, датчик или очередь.

```yaml
discover:
  - name: disks
    source: list
    rows: $.disks[*]        # JSONPath: one row per match
    max_rows: 100           # default 100, at most 1,000
    labels:                 # JSONPath evaluated on each row
      disk: $.name
      model: $.info.model
    metrics:
      - name: disk_used_bytes
        json: $.used        # JSONPath evaluated on each row
      - name: disk_healthy
        json: $.state
        map: {ok: 1, degraded: 0}
```

Метрики строк принимают `name`, `help`, `kind`, `json`, `scale` и `map`.
Строки сверх `max_rows` игнорируются.

## Правила

Правила устанавливаются вместе с паком как обычные правила оповещений с uid
`pack:<id>:<name>`. После этого они принадлежат вам: обновление пака добавляет
правила, которых ещё нет, и никогда не перезаписывает правило, которое вы
изменили или удалили.

```yaml
rules:
  - name: overheating       # lowercase, digits, underscores
    title: Shelly plug overheating
    description: The internal temperature stayed above the limit for five minutes.
    expr: dumbmonit_shelly_plug_temperature_celsius
    op: ">"                 # >, >=, < or <=
    threshold: "{{option.max_temperature}}"   # or a number
    for: 5m                 # 30s, 5m, 1h, 1d
    severity: warning       # info, warning (default) or critical
    unit: °C
```

- `expr` — это [MetricsQL](https://docs.victoriametrics.com/metricsql/), и он
  должен использовать хотя бы одну метрику пака по её полному имени. Любое
  другое имя `dumbmonit_*`, которое он упоминает, тоже должно создаваться
  паком (кроме `dumbmonit_up`): пак не может оповещать по сериям другого типа.
- `threshold` вида `{{option.x}}` устанавливает значение опции по умолчанию,
  а собственное значение этой опции у каждого устройства становится его порогом
  для этого правила (переопределение для устройства, перезаписываемое при
  каждом сохранении устройства).

## Профили SNMP

Пак может содержать профили SNMP, написанные точно так же, как встроенные в
[`profiles/`](https://github.com/laupernoe/dumbmonit/tree/main/profiles). Они
присоединяются к каталогу SNMP и применяются по `sysObjectID`, как и остальные;
собственный тип устройства они не создают.

```yaml
snmp_profiles:
  - id: acme-ups            # the pack id, or starting with "<pack id>-"
    name: ACME UPS
    match:
      sysobjectid: [1.3.6.1.4.1.99999]
    include: [system]
    metrics:
      - name: acme_ups_battery_percent   # must start with the pack prefix
        oid: 1.3.6.1.4.1.99999.1.1.0
        kind: gauge
```

Каталог SNMP читается при запуске: перезапустите DumbMonit после установки или
обновления пака с профилями SNMP.

## Лимиты

| Лимит | Значение |
|---|---|
| Размер пака | 256 KiB |
| Источники (запросы за опрос, включая редиректы) | 16 |
| Размер ответа | 4 MiB |
| Серий за опрос | 5 000 (остальные отбрасываются и записываются в лог) |
| Объявленных метрик | 256 |
| Правил | 64 |
| Строк на один `discover` | 1 000 |
| Длина значения метки | 200 байт |

## Фикстуры и ожидаемый вывод

`dumbmonit pack test <dir>` читает `fixtures/<source id>.json`, `.txt` или
`.prom`, выполняет то же извлечение, что и сервер, и сравнивает результат с
`expected.prom`: текстовый формат Prometheus, отсортированный, без временных
меток и без меток устройства (`target`, `host`, `tag_*`), которые добавляет
сервер. `--update` записывает `expected.prom` из фикстур; просмотрите diff
перед коммитом.
