# Почтовый сервер MDaemon

Почтовые службы сервера MDaemon, проверяемые снаружи: SMTP, IMAP, POP3, веб-почта
и Remote Administration принимают соединения и отвечают приветствием так, как
должны. При наличии учётной записи DumbMonit также читает точную версию через
XML API. Если на сервере MDaemon установлен [агент](agent.md), та же страница
показывает ещё и почтовые очереди, сессии и итоги по сообщениям, которые MDaemon
публикует только в виде счётчиков производительности Windows.

!!! warning "Проверено только по документации"

    Эта интеграция создана по опубликованной документации MDaemon Technologies и
    приведённому там формату ответов XML API, а не на реальном сервере MDaemon.
    Каждый вызов — это чтение, а проверки почтовых портов следуют стандартам
    протоколов, но пока никто не запускал её на живом MDaemon. То же относится к
    счётчикам очередей, читаемым агентом: их имена взяты из опубликованного
    перечня счётчиков производительности MDaemon, а не с работающего сервера.
    Сообщайте нам, что не работает.

## Что отслеживается

Все метрики имеют префикс `dumbmonit_mdaemon_`.

| Метрика | Метки | Значение |
|---|---|---|
| `service_up` | `service`, `port` | 1, если порт принял соединение и, для SMTP, POP3 и IMAP, ответил приветствием `220`, `+OK` или `* OK`. 0, если он не ответил или ответил отказом в обслуживании (`421`, `554`, `-ERR`, `* BYE`). |
| `service_response_seconds` | `service`, `port` | Время до приветствия или до соединения для порта без приветствия. |
| `api_up` | | Только с учётной записью: 1, если XML API ответил, 0, если нет. |
| `api_response_seconds` | | Время, за которое XML API ответил. |
| `api_operation_ok` | | 1, если учётной записи разрешено выполнить вызов, 0, если MDaemon ответил статусом ошибки. |
| `info` (значение 1) | `version`, `build`, `source` | Версия: из XML API (`source="xml_api"`, `build` — сборка службы) или же из приветствия SMTP, когда сервер объявляет её там (`source="smtp_banner"`). |
| `scrape_duration_seconds` | | Время, которое заняла вся проверка. |

**Частичный сбой — это всё равно успешная проверка.** Одна остановленная служба
даёт `service_up 0` для этой службы, а остальные по-прежнему измеряются. Только
когда не отвечает ни одна служба и ни один API, монитор считается
*недоступным*.

**Отклонённая учётная запись — это ошибка конфигурации, а не сбой.** Неверный
пароль, адрес, который XML API не разрешает, или неверный порт показываются на
странице монитора с причиной и не рассылаются в уведомлениях.

**Очереди приходят от агента, а не от этого монитора.** MDaemon публикует
размеры своих очередей (входящая, локальная, удалённая, повторная, bad, holding,
карантин), число сессий, статистику сообщений и время работы в виде счётчиков
производительности Windows, в объекте с именем MDaemon, и в собственном Queue and
Statistics Manager. Ни в одной опубликованной документации MDaemon не описан
вызов XML API, который их возвращает, поэтому этот монитор не гадает: вместо
этого счётчики производительности читает [агент](agent.md), установленный на
сервере MDaemon. См. [Почтовые очереди через агента](#mail-queues-through-the-agent).
Лицензия и свободное место диска с почтовым хранилищем не доступны через
документированный интерфейс или счётчик производительности и не читаются. Агент
также следит за диском, на котором лежит почтовое хранилище; чтобы следить за
самой службой Windows, перечислите службу MDaemon в его настройке `services`.

Применимые [встроенные правила](../alerting/rules.md#mdaemon-and-securitygateway):
MDaemon mail service down, MDaemon XML API not answering, а также Device
unreachable; с агентом — MDaemon mail queue growing, MDaemon Bad queue not empty
и MDaemon Retry queue high.

## Страница монитора

Монитор MDaemon показывает панель **Mail services** над графиками, читаемую из
последнего сохранённого измерения: открытие страницы никогда не подключается к
серверу. Она перечисляет каждую отслеживаемую службу с её портом, словом и
цветом (Answering, Down) и временем ответа; затем версию и откуда она прочитана.
Остановленные службы идут первыми.

Когда агент на том же сервере сообщает счётчики MDaemon, следом идёт панель
**Mail queues**: число сообщений в каждой очереди (замороженная очередь,
непустая очередь Bad и более пятидесяти сообщений в Retry отмечаются словами),
активные сессии, сообщения и вердикты по спаму, вирусам и DNSBL за последние 24
часа, время работы MDaemon и внутренние серверы, которые MDaemon сообщает как
неактивные. Та же панель появляется на собственной странице агента.

## Почтовые очереди через агента {#mail-queues-through-the-agent}

1. Установите [агент](agent.md#install) на сервер Windows, на котором работает
   MDaemon. Больше ничего не требуется: агент замечает службу Windows с именем
   `MDaemon` и читает счётчики объекта производительности MDaemon в каждый период
   выборки. Чтобы принудительно включить или выключить это, задайте
   `mdaemon: true` или `mdaemon: false` в `agent.yaml`.
2. Очереди появляются на странице агента. Чтобы показать их и на этом мониторе
   MDaemon, DumbMonit ищет агента на той же машине: агента, заданного как
   родитель этого монитора, или relay-агента, который его опрашивает, или
   агента, имя хоста которого совпадает с адресом этого монитора (`mail` для
   `mail.example.com`), или, когда есть один монитор MDaemon и один агент,
   сообщающий счётчики MDaemon, этого агента. Если ничего не подошло,
   отредактируйте этот монитор и выберите агента как его **Parent monitor**:
   агент и MDaemon находятся на одной машине, поэтому когда агент замолкает,
   оповещения этого монитора тоже удерживаются.

Ряды принадлежат агенту, с его меткой `target`. Все имеют префикс
`dumbmonit_mdaemon_`:

| Метрика | Метки | Читаемые счётчики производительности |
|---|---|---|
| `queue_messages` | `queue`: `inbound`, `local`, `remote`, `retry`, `bad`, `holding`, `lan`, `quarantine`, `raw` | `Inbound queue messages`, `Local queue messages`, `Remote queue messages`, `Retry queue messages`, `Bad queue messages`, `Holding queue messages`, `LAN queue messages`, `Quarantine queue messages`, `RAW queue messages` |
| `queue_frozen` (1 — заморожена) | `queue`: `inbound`, `local`, `remote` | `Inbound queue frozen`, `Local queue frozen`, `Remote queue frozen` |
| `sessions_active` | `protocol`: `smtp_in`, `smtp_out`, `pop3_in`, `pop3_out`, `imap`, `webmail` | `Active SMTP (in) sessions`, `Active SMTP (out) sessions`, `Active POP3 (in) sessions`, `Active POP3 (out) sessions`, `Active IMAP sessions`, `Active Webmail sessions` |
| `sessions_total` (счётчик) | `protocol`: `smtp_in`, `smtp_out`, `pop3`, `imap` | `SMTP sessions (in) total`, `SMTP sessions (out) total`, `POP3 sessions total`, `IMAP sessions total` |
| `messages_total` (счётчик) | `protocol`: `smtp_in`, `smtp_out`, `domainpop_in` | `SMTP messages (in) total`, `SMTP messages (out) total`, `DomainPOP messages (in) total` |
| `filtered_messages_total` (счётчик) | `filter`: `spam`, `virus`, `dnsbl`; `verdict`: `accepted`, `refused` | `spam accepted total`, `spam refused total`, `Viruses accepted total`, `Viruses refused total`, `DNSBL accepted total`, `DNSBL refused total` |
| `server_active` (1 — активен) | `server`: `smtp`, `pop3`, `imap`, `webmail`, `webadmin`, `activesync`, `antispam`, `antivirus`, `minger`, `multipop`, `domainpop` | `SMTP server state`, `POP3 server state`, `IMAP server state`, `Web Mail server state`, `Web Admin server state`, `ActiveSync server state`, `AntiSpam server state`, `AntiVirus server state`, `Minger server state`, `MultiPOP server state`, `DomainPOP server state` |
| `running` (1 — работает) | | `MDaemon running state` |
| `uptime_seconds` | | `MDaemon up time` |

Итоги считаются с момента последнего запуска MDaemon: применяйте `increase()` или
`rate()` в запросах, как это делает страница для своих 24-часовых показателей.
Счётчики в секунду (`SMTP messages (in)/sec`…) не читаются, так как итоги дают те
же скорости. Счётчик, которого нет в вашей версии MDaemon или который назван
иначе, один раз записывается в лог агента (`performance counter not
available`), а остальные по-прежнему читаются; чтобы прочитать счётчик, которого
нет в пресете, перечислите его в `perf_counters` в `agent.yaml` (см.
[Счётчики производительности Windows](agent.md#windows-performance-counters)).

## Следить за почтовыми службами и, при желании, за XML API

1. На сервере ничего устанавливать не нужно. Без учётных данных DumbMonit подключается к почтовым портам снаружи и читает каждое приветствие: по умолчанию SMTP (25), IMAP (143) и веб-почта (3000). Добавьте POP3, MSA, порты TLS или Remote Administration в параметре Services в виде списка, например такого.

    ```
    smtp,msa,imap,pop3,imaps,webmail
    ```

2. Чтобы также читать версию через XML API, создайте в MDaemon отдельную учётную запись, названную, например, так, с длинным паролем, нигде больше не используемым. Дайте ей самый низкий уровень, который MDaemon принимает для XML API, и ничего сверх этого: DumbMonit вызывает только GetVersionInfo, который не читает ни один почтовый ящик и ничего не меняет.

    ```
    dumbmonit@example.com
    ```

3. Начиная с MDaemon 24, XML API отвечает только разрешённым адресам. Откройте Setup → XML API Service → Address Restrictions (Setup → XML API Management в MDaemon 26) и разрешите адрес, с которого подключается DumbMonit.

4. Проверьте с хоста DumbMonit, что учётная запись получает доступ к API. Команда запрашивает пароль и выводит ответ, начинающийся с `<MDaemon><API productversion=…>`.

    ```
    curl -k -u dumbmonit@example.com -H 'Content-Type: text/xml' --data '<MDaemon><API><Request version="20.0.0" echo="0" verbose="0"><Operation>GetVersionInfo</Operation><Parameters/></Request></API></MDaemon>' https://mail.example.com:444/MdMgmtWS/
    ```

5. В DumbMonit введите адрес сервера, например «mail.example.com», затем полный адрес электронной почты учётной записи как имя пользователя и её пароль. XML API доступен на HTTPS-порту Remote Administration, по умолчанию 444.

!!! warning
    В MDaemon нет роли только для чтения для XML API: никогда не используйте свою собственную учётную запись и не применяйте этот пароль ни в каком другом инструменте. Dynamic Screening может заблокировать адрес после повторных неудачных входов, поэтому неверный пароль здесь может привести к блокировке хоста DumbMonit: сначала проверьте командой выше. Remote Administration часто использует самоподписанный сертификат: если соединение отклонено по этой причине, отметьте в параметрах «Accept an unverifiable certificate». Размеры почтовых очередей этот монитор не читает: MDaemon публикует их только в виде счётчиков производительности Windows, которые читает агент DumbMonit, когда он установлен на почтовом сервере.

XML API обслуживается веб-сервером Remote Administration: по умолчанию порт 1000
по HTTP и 444 по HTTPS, по пути `/MdMgmtWS/`, или IIS, когда Remote
Administration работает на нём. Его полный справочник поставляется вместе с
сервером: откройте `https://mail.example.com:444/MdMgmtWS` в браузере или
загляните в `\MDaemon\Docs\API\XML API\`.

## Учётные данные

| Учётные данные | Поля |
|---|---|
| Services only | Ничего: почтовые порты проверяются без входа. |
| XML API account | Полный адрес электронной почты учётной записи и её пароль, отправляемые как базовая HTTP-аутентификация. |

Адрес: имя хоста или IP сервера, например `mail.example.com`. Порт, указанный в
адресе (`mail.example.com:8444`), принимается как порт XML API.

## Параметры

| Параметр | По умолчанию | Действие |
|---|---|---|
| `services` | `smtp,imap,webmail` | Что проверять, через запятую: `smtp` (25), `msa` (587), `smtps` (465), `pop3` (110), `pop3s` (995), `imap` (143), `imaps` (993), `webmail` (3000), `remote_admin` (1000), `remote_admin_https` (444), `xmpp` (5222). `name:port` меняет порт; любое другое имя с портом добавляет вашу собственную службу, проверяемую только соединением. `none` не проверяет ни одного порта. |
| `request_timeout_seconds` | `10` | Время, отведённое на каждое соединение, приветствие и вызов API, от 1 до 60. |
| `api_port` | `444` | Порт Remote Administration, обслуживающий `/MdMgmtWS/`. Используется только с учётной записью. |
| `api_tls` | `true` | HTTPS к XML API. Снимайте отметку только для Remote Administration по обычному HTTP (по умолчанию порт 1000): тогда пароль передаётся по сети открытым текстом. |
| `insecure_tls` | `false` | Принимать самоподписанный сертификат на XML API. |

Порты TLS (`smtps`, `pop3s`, `imaps`, `remote_admin_https`) проверяются только
соединением. Чтобы следить за сроком действия их сертификатов, добавьте
монитор [TLS certificate](services.md#tls) для каждого.
