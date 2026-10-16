# Метрики

DumbMonit хранит каждое измерение в VictoriaMetrics и читает его обратно с помощью
MetricsQL через [прокси метрик](api.md#metrics) сервера. Эта страница
описывает именование, метки и срок хранения. Списки метрик по типам
находятся на [страницах мониторов](../devices/index.md).

## Именование

Каждая метрика имеет префикс `dumbmonit_`, поэтому DumbMonit может делить один экземпляр VictoriaMetrics
с другими инструментами без коллизий. Имена следуют схеме
`dumbmonit_<what>_<unit>`:

| Шаблон | Примеры |
|---|---|
| `…_bytes`, `…_bytes_total`, `…_bytes_used` | `dumbmonit_memory_bytes_total`, `dumbmonit_filesystem_used_bytes` |
| `…_percent` | `dumbmonit_cpu_usage_percent`, `dumbmonit_pbs_datastore_used_percent` |
| `…_seconds`, `…_age_seconds` | `dumbmonit_probe_duration_seconds`, `dumbmonit_proxmox_backup_last_age_seconds` |
| `…_celsius`, `…_volts`, `…_hertz`, `…_watts` | Показания ИБП и Synology |
| `…_info` | Ряды присутствия (значение 1), информация которых находится в метках: `dumbmonit_system_info`, `dumbmonit_pbs_version_info` |
| `…_status`, `…_up`, `…_running` | Перечисления или булевы значения: `dumbmonit_if_oper_status`, `dumbmonit_proxmox_guest_running` |

Семейства, специфичные для источника, несут его как второй префикс:
`dumbmonit_proxmox_*`, `dumbmonit_pbs_*`, `dumbmonit_synology_*`,
`dumbmonit_probe_*` (сервисы). Профили SNMP и агент используют простые имена
(`dumbmonit_if_octets_in`, `dumbmonit_cpu_usage_percent`).

Счётчики (октеты и пакеты интерфейсов, байты дисков, напечатанные страницы) хранятся
**как есть**, в том виде, как их сообщает монитор: применяйте `rate()` в собственных запросах, чтобы получить
пропускную способность. Страница монитора отображает каждый ряд `dumbmonit_*` монитора в том виде,
в каком он хранится, поэтому счётчик выглядит как растущая линия. Перезагрузка отображается как сброс
счётчика, а не как ложный всплеск.

## `dumbmonit_up`

После каждой успешной пробы сервер записывает `dumbmonit_up = 1` для
монитора. Когда проба неудачна, ничего не записывается: ряд просто прекращается, а
правило «Monitor unreachable» измеряет возраст последнего отсчёта
(`time() - tlast_over_time(dumbmonit_up[7d])`), оставляя только самый свежий ряд
каждого монитора (`bottomk by (target) (1, …)`): ряд, который больше не записывается после
смены тега или метки, остаётся в 7-дневном окне и не должен учитываться.

Для сервисов (`http`, `tcp`, `dns`, `ping`, `tls`) `dumbmonit_up = 1` означает
«монитор отработал», а `dumbmonit_probe_success` говорит, в порядке ли сервис.
Эти два сигнала намеренно разделены: `probe_success`, равный 0, означает,
что сервис недоступен, прерывание `up` означает, что недоступен мониторинг.

## `dumbmonit_security_score`

Каждые 15 минут сервер вычисляет [оценку безопасности](../using/security-score.md)
каждого монитора, тип которого имеет проверки безопасности, и записывает
`dumbmonit_security_score` (0–100, отсутствует, пока нечего оценивать) и
`dumbmonit_security_checks_failed`, оба с меткой `target`.

## Метки

| Метка | Кем задаётся | Значение |
|---|---|---|
| `target` | сервер | Идентификатор монитора, как в `/api/targets/{id}`. Каждое правило и каждый график выбирают по ней. |
| `host` | сервер | Имя монитора. |
| `tag_<key>` | сервер | Свободные теги монитора. Опции типа (`insecure_tls`, `port`…), теги, ключ которых похож на секрет (сопоставление по словам: `key`, `pass`, `cert`, `private`, `pem`…), и значения длиннее 256 символов не копируются. Никогда не помещайте секрет в тег. |
| `probe` | сервисы | `http`, `tcp`, `dns`, `ping` или `tls`. |
| метки отдельных рядов | сборщик | `ifname`, `mountpoint`, `device`, `fstype`, `core`, `service`, `container`, `node`, `vmid`, `name`, `storage`, `datastore`, `group`, `volume`, `disk`, `pool`, `repo`, `changes`, `reason`, `version`, `issuer`… как указано на странице каждого монитора. |

Метки идентичности всегда имеют приоритет над метками сборщика, а для агентов они задаются
при приёме: машина не может писать в ряды другой машины.

!!! note "Префикс до сентября 2026"
    Ряды, записанные EzyMonit, называются `ezymonit_…`. Префикс изменился вместе
    с названием продукта, и ничто не читает старые ряды: они остаются в
    VictoriaMetrics, пока срок хранения их не удалит, графики и правила начинаются
    заново с момента обновления, а пользовательские правила, всё ещё называющие `ezymonit_…`, нужно отредактировать.

## Срок хранения и хранилище

Встроенная VictoriaMetrics хранит данные 12 месяцев (`DUMBMONIT_VM_RETENTION`)
в `/data/vm`, в томе `dumbmonit-data`. Сервер группирует свои
записи и сбрасывает их каждые 5 секунд (`DUMBMONIT_FLUSH_INTERVAL_SECS`).
Удаление монитора не удаляет его ряды; они устаревают сами.

История оповещений хранится 90 дней, а базовые линии рядов, пропавших на
60 дней, — в SQLite (см. [справочник по конфигурации](configuration.md)).

## Запросы

- Через сервер: `GET /api/metrics/query` и `GET /api/metrics/query_range`,
  с cookie сессии. Результаты диапазона ограничены 2 000 точками на ряд.
- Из существующих Prometheus или Grafana: `GET /federate`, `GET /metrics` и
  источник данных `/prometheus`, с токеном API —
  [ниже](#scraping-dumbmonit).
- Напрямую, в разработке: оверлей `docker-compose.dev.yml` публикует
  встроенную VictoriaMetrics на `http://localhost:8428`, с её собственным интерфейсом на
  `http://localhost:8428/vmui/`. В продакшене она слушает только loopback контейнера;
  задайте `DUMBMONIT_VM_LISTEN=0.0.0.0:8428` и опубликуйте порт
  самостоятельно, если хотите подключить Grafana или другой инструмент к тем же данным.

```
# Interface throughput in bits per second
rate(dumbmonit_if_octets_in{target="4"}[5m]) * 8

# Availability of every service over 30 days
avg by (host) (avg_over_time(dumbmonit_probe_success[30d]))

# Every device that has not reported for 5 minutes
bottomk by (target) (1, time() - tlast_over_time(dumbmonit_up[7d])) > 300
```

## Скрейпинг DumbMonit {#scraping-dumbmonit}

Если у вас уже работает Prometheus или Grafana, второй стек не нужен.
Три маршрута находятся вне `/api`, там, где их ищет скрейпер:

| Маршрут | Что возвращает |
|---|---|
| `GET /metrics` | Состояние самого экземпляра DumbMonit: планировщик, пробы, записи, оповещения, уведомления, агенты, база данных. Несколько десятков рядов, всегда одно и то же число — никогда не по одному на монитор. |
| `GET /federate` | Измерения, выбранные через `match[]`, в точности так, как Prometheus федерирует другой Prometheus: последняя точка каждого выбранного ряда. |
| `GET /prometheus/api/v1/…` | Читающая половина API Prometheus, ретранслируемая в хранилище: то, что вызывает источник данных Grafana. |

Все три принимают `Authorization: Bearer dmt_…` с областью **`read`** — токен,
созданный в Settings → *API & assistants*, того же вида, что использует
[сервер MCP](../using/assistant.md). Cookie не задействованы, поэтому заголовок
`X-Requested-With` не нужен; cookie сессии тоже принимается, так что
вы можете открыть любой из них из браузера, в котором выполнен вход. Токен ограничен 120 вызовами
в минуту: этого более чем достаточно для скрейпа каждые 15 секунд и хватает для Grafana
с дашбордом из нескольких десятков панелей — загруженному дашборду выдайте отдельный токен, а не
делите один со скрейпером.

### Prometheus

```yaml
# prometheus.yml — DumbMonit's own health, then the measurements it collects.
scrape_configs:
  - job_name: dumbmonit
    metrics_path: /metrics
    scheme: http
    authorization:
      type: Bearer
      credentials: dmt_0123456789abcdef0123456789abcdef
    static_configs:
      - targets: ['dumbmonit.lan:8080']

  - job_name: dumbmonit-federate
    metrics_path: /federate
    scheme: http
    honor_labels: true
    scrape_interval: 60s
    authorization:
      type: Bearer
      credentials: dmt_0123456789abcdef0123456789abcdef
    params:
      'match[]':
        - '{__name__=~"dumbmonit_.*"}'
    static_configs:
      - targets: ['dumbmonit.lan:8080']
```

`honor_labels: true` сохраняет метки `target` и `host`, которые задаёт DumbMonit,
вместо того чтобы перезаписывать их собственными метками задания скрейпа. Поместите токен в файл
и используйте `authorization: {type: Bearer, credentials_file: /etc/prometheus/dumbmonit.token}`,
если не хотите держать его в конфигурации.

Без `match[]` `/federate` выбирает `{__name__=~"dumbmonit_.*"}` — каждый
ряд, который пишет DumbMonit, и ничего, что принадлежит другому инструменту, использующему ту же
VictoriaMetrics. Сузьте выборку, чтобы федерация оставалась небольшой:
`{__name__=~"dumbmonit_.*",target="4"}` для одного монитора,
`{__name__="dumbmonit_up"}` только для доступности. Не более 10 селекторов на
запрос, а ответ больше 8 МиБ отклоняется с сообщением, предлагающим
сузить выборку, — разбейте большой парк на несколько заданий с более узкими селекторами,
а не используйте одно, запрашивающее всё.
`max_lookback=15m` расширяет окно, в котором ищется последняя точка,
для мониторов, опрашиваемых реже, чем раз в пять минут.

```bash
curl -H 'Authorization: Bearer dmt_…' http://localhost:8080/metrics
curl -H 'Authorization: Bearer dmt_…' -G http://localhost:8080/federate \
  --data-urlencode 'match[]={__name__=~"dumbmonit_.*",target="4"}'
```

### Grafana

Направьте Grafana на **DumbMonit**, а не на VictoriaMetrics, которую он запускает: дочерний
процесс слушает loopback контейнера, не имеет собственной аутентификации
и предоставляет маршруты, которые *удаляют* ряды, так же охотно, как и те, что их читают.
DumbMonit ретранслирует читающую половину API Prometheus под `/prometheus`,
за тем же токеном, поэтому Grafana работает с обычным источником данных Prometheus:

1. **Connections → Data sources → Add → Prometheus**.
2. **URL**: `http://dumbmonit.lan:8080/prometheus` (Grafana сама добавляет
   `/api/v1/query`).
3. **HTTP headers**: добавьте один, `Authorization` = `Bearer dmt_…`. Grafana хранит
   его в зашифрованном виде и больше никогда не показывает.
4. **Save & test** — Grafana сообщает, что источник данных работает, и каждый
   ряд `dumbmonit_*` становится доступен в PromQL, с автодополнением имён
   метрик и меток.

Ретранслируются только читающие маршруты — `query`, `query_range`, `series`, `labels`,
значения меток, `metadata` и версия, которую Grafana запрашивает при проверке
соединения. Всё остальное, в первую очередь `admin/tsdb/delete_series`, отвечает
`404`: эта точка входа не может ничего изменить или стереть.

Если у вас уже работает Prometheus, другой путь работает так же хорошо: Prometheus
федерирует DumbMonit заданием выше, Grafana читает Prometheus, а
запросы идентичны.

### Метрики экземпляра

Всё, что предоставляет `GET /metrics`. Значения считываются в момент скрейпа;
счётчики `_total` монотонны и сбрасываются только при перезапуске сервера
(`dumbmonit_uptime_seconds` подскажет, когда это произошло).

| Метрика | Тип | Значение |
|---|---|---|
| `dumbmonit_build_info{version}` | gauge | Всегда 1. Метка `version` — версия работающего сервера, та же, что сообщает `/api/health`. |
| `dumbmonit_uptime_seconds` | gauge | Секунды с момента запуска процесса сервера. |
| `dumbmonit_scheduler_cycles_total` | counter | Выполненные циклы планировщика. Цикл тикает раз в секунду, так что это ещё и сигнал жизнеспособности. |
| `dumbmonit_scheduler_cycle_seconds` | gauge | Длительность последнего цикла. Обычно менее миллисекунды: цикл только распределяет задачи, он не ждёт проб. |
| `dumbmonit_scheduler_backlog` | gauge | Мониторы, которым ещё предстояла проба к концу последнего цикла. Ноль на исправном экземпляре; устойчивое ненулевое значение означает, что `DUMBMONIT_MAX_CONCURRENT_PROBES` слишком мало для числа мониторов и их интервалов. |
| `dumbmonit_probes_total{kind}` | counter | Выполненные пробы по типу монитора (`snmp`, `proxmox`, `http`…). Включает те, которые ретранслирующий агент выполнил от имени сервера. |
| `dumbmonit_probes_failed_total{kind}` | counter | Неудачные пробы по типу — недоступный монитор или ошибка конфигурации. `rate(dumbmonit_probes_failed_total[15m]) / rate(dumbmonit_probes_total[15m])` — доля неудач. |
| `dumbmonit_samples_written_total` | counter | Отсчёты, принятые VictoriaMetrics. |
| `dumbmonit_sample_writes_failed_total` | counter | Пакеты, которые VictoriaMetrics отклонила или на которые не ответила. Пакет остаётся в буфере и повторяется; растущее значение вместе с растущим `dumbmonit_samples_pending` означает, что хранилище недоступно или переполнено. |
| `dumbmonit_samples_pending` | gauge | Отсчёты, всё ещё находящиеся в буфере записи после последнего сброса. Обычно 0. |
| `dumbmonit_samples_dropped_total` | counter | Отсчёты, отброшенные ещё до попадания в буфер записи, потому что он был полон или закрыт. Не повторяются — растущее значение во время сбоя означает потерю данных, а не просто задержку. |
| `dumbmonit_alerting_cycles_total` | counter | Выполненные циклы оповещений (один каждые `DUMBMONIT_ALERT_INTERVAL_SECS`, по умолчанию 30 с). |
| `dumbmonit_alerting_cycle_seconds` | gauge | Длительность последнего цикла оповещений, включая время запросов. Приближение к интервалу означает, что правила запрашивают больше, чем хранилище способно отдать. |
| `dumbmonit_alerting_rules_evaluated` | gauge | Правила, оценённые в последнем цикле. |
| `dumbmonit_alerting_rules_failed` | gauge | Правила, чей запрос завершился неудачно в последнем цикле. Их оповещения заморожены, а не разрешены. Всё, кроме 0, заслуживает просмотра логов. |
| `dumbmonit_alerts{phase}` | gauge | Оповещения, отслеживаемые движком, по фазам: `ok`, `pending` (условие истинно, `for` ещё не истёк), `firing`, `resolved`. |
| `dumbmonit_alerts_suppressed` | gauge | Оповещения, скрытые, потому что родительский монитор недоступен (эффективная фаза `suppressed`). |
| `dumbmonit_alerts_silenced` | gauge | Оповещения, покрытые окном обслуживания. |
| `dumbmonit_alerts_learning` | gauge | Оповещения по базовой линии, всё ещё обучающиеся и потому молчащие. |
| `dumbmonit_notifications_total{kind}` | counter | Отправленные уведомления по типу канала (`email`, `discord`, `ntfy`…). |
| `dumbmonit_notifications_failed_total{kind}` | counter | Уведомления, которые канал отклонил, по типу. Они возвращаются в очередь и повторяются; устойчиво растущее значение — это сломанный вебхук или учётная запись SMTP. |
| `dumbmonit_agents` | gauge | Машины, зарегистрировавшие агента. |
| `dumbmonit_agents_stale` | gauge | Агенты, не отправлявшие ничего в течение пяти минут. |
| `dumbmonit_database_up` | gauge | 1, когда SQLite отвечает — та же проверка, что и `/api/health`. |
| `dumbmonit_database_bytes` | gauge | Размер `dumbmonit.db` и его журнала упреждающей записи. Не включает `/data/vm`. |
| `dumbmonit_victoriametrics_up` | gauge | 1, когда VictoriaMetrics отвечает — та же проверка, что и `/api/health`. 0, пока встроенный дочерний процесс перезапускается. |
| `dumbmonit_victoriametrics_embedded` | gauge | 1, когда этот сервер сам запускает VictoriaMetrics, 0, когда `DUMBMONIT_VM_URL` указывает на внешнюю. |

Ни одна метрика на этой странице не несёт метки отдельного монитора: документ имеет один и тот же
размер для одного монитора и для тысячи, и никакое имя монитора не утекает к тому, кто
может выполнять скрейп.

Два оповещения, которые стоит добавить на вашей стороне, когда DumbMonit скрейпится:

```
# The monitoring stopped monitoring
up{job="dumbmonit"} == 0

# Measurements are piling up instead of being stored
dumbmonit_samples_pending > 0 and increase(dumbmonit_sample_writes_failed_total[10m]) > 0
```

### Открытие маршрутов без токена

`DUMBMONIT_METRICS_PUBLIC=true` отдаёт `/metrics`, `/federate` и
`/prometheus/api/v1/…` любому, кто может достучаться до порта, — для тех, кто сам фильтрует
порт, — **любой в этой сети тогда сможет прочитать каждое измерение
каждого монитора и состояние экземпляра, поэтому включайте это, только когда что-то
другое уже ограничивает доступ к порту.**
