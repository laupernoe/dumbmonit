# FortiGate

Fortinet FortiGate через REST API FortiOS: упавшие IPsec-туннели между
площадками, включённые интерфейсы, потерявшие связь, кластер HA
с рассинхронизацией или нехваткой участника, память, приближающаяся к режиму conserve mode, истекающие
лицензии FortiGuard и FortiCare и патч-релизы, предлагаемые для работающей
ветки FortiOS.

DumbMonit использует пользователя REST API с профилем доступа только на чтение, который
принимает вызовы только с адреса сервера DumbMonit (trusted host). Его
токен передаётся как `Authorization: Bearer`, а не в URL: FortiOS 7.4.5 и
7.6.1 всё равно отклоняют его в URL.

Каждую минуту DumbMonit читает три обязательные страницы — `/api/v2/monitor/system/status`,
`/api/v2/monitor/system/resource/usage` и `/api/v2/monitor/system/interface/select`
— и шесть необязательных, которые более узкий профиль либо FortiGate без
кластера или туннелей может отклонить или оставить пустыми, не нарушая остальное: 
административное состояние интерфейсов (`/api/v2/cmdb/system/interface`), статистика и контрольные суммы HA,
IPsec-туннели, состояние лицензий и доступные
прошивки.

!!! warning "Проверено только по документации производителя — на реальной системе ещё не тестировалось"

    Эта интеграция создана по документации FortiOS от Fortinet и по
    формам ответов, записанным в экспортёре Prometheus для
    FortiGate от сообщества, а не на работающем FortiGate. Каждый вызов — это чтение. Страницы
    лицензий и прошивок документированы хуже всего: расскажите нам, что
    сломалось.

## За чем он следит

Все метрики имеют префикс `dumbmonit_fortigate_`.

| Метрика | Метки | Значение |
|---|---|---|
| `version_info` | `version`, `build`, `model`, `hostname` | Значение 1. |
| `cpu_usage_percent`, `memory_used_percent`, `disk_used_percent` | | Текущее использование. |
| `sessions`, `session_setup_rate` | | Активные сессии, новые сессии в секунду. |
| `log_disk_ok` | | 0, когда диск журналов требует форматирования. |
| `interface_link_up` | `interface`, `alias` | Физическая связь. |
| `interface_enabled` | `interface`, `alias` | Административное состояние; каждый интерфейс считается включённым, если профиль не может прочитать конфигурацию. |
| `interface_down` | `interface`, `alias` | 1, когда интерфейс включён, имеет адрес и не имеет связи. Свободные порты без адреса не учитываются. |
| `interface_speed_mbps` | `interface`, `alias` | Пока связь есть. |
| `interface_bytes_in_total`, `interface_bytes_out_total`, `interface_errors_in_total`, `interface_errors_out_total` | `interface`, `alias` | Счётчики. |
| `interfaces_down` | | Количество. |
| `ha_members` | | Участники в кластере; 0 для отдельного FortiGate. |
| `ha_in_sync` | | При двух и более участниках: 1, когда у всех участников одинаковая контрольная сумма конфигурации. |
| `ha_member_cpu_usage_percent`, `ha_member_memory_used_percent` | `member` | По участнику. |
| `ipsec_tunnel_up` | `tunnel` | 1, когда поднята хотя бы одна фаза 2 туннеля. Dial-up-туннели (удалённый доступ) не учитываются. |
| `ipsec_phase2_up` | `tunnel`, `phase2` | По селектору фазы 2. |
| `ipsec_tunnel_bytes_in_total`, `ipsec_tunnel_bytes_out_total` | `tunnel` | Счётчики. |
| `ipsec_tunnel_info` | `tunnel`, `remote_gateway` | Значение 1. |
| `ipsec_tunnels`, `ipsec_tunnels_down` | | Количество. |
| `license_status_info` | `license`, `status` | Значение 1 для каждой лицензии, которой владеет FortiGate (`antivirus`, `ips`, `web_filtering`, `forticare_hardware`…); записи `no_license` не учитываются. |
| `license_expired` | `license` | 1, когда статус `expired`. |
| `license_expiry_seconds` | `license` | До истечения; отрицательное после истечения. |
| `firmware_update_available` | | 1, когда FortiGuard предлагает более новый патч-релиз работающей ветки (7.4.4 → 7.4.5). Новая ветка (7.6) — это проект, а не тревога. |
| `firmware_latest_info` | `version` | Значение 1: этот патч-релиз. |

Применяемые [встроенные правила](../alerting/rules.md#fortigate):

- **FortiGate IPsec tunnel down**: ни одной поднятой фазы 2 в течение пяти минут (Warning).
- **FortiGate interface without link**: в течение пяти минут (Advisory).
- **FortiGate HA out of sync**: в течение пятнадцати минут (Advisory).
- **FortiGate HA member lost**: участников меньше, чем в любой момент за последние
  24 часа (Warning). Ожидаемое число нигде не публикуется, поэтому участник,
  удалённый навсегда, перестаёт вызывать тревогу на следующий день.
- **FortiGate memory near conserve mode**: выше 85 % в течение десяти минут
  (Advisory). По умолчанию при 88 % FortiOS входит в conserve mode и перестаёт
  проверять новые сессии.
- **FortiGate CPU high**: выше 90 % в течение пятнадцати минут (Advisory).
- **FortiGate licence expiring**: осталось менее 30 дней или лицензия истекла
  (Advisory, напоминание ежедневно).
- **FortiGate firmware update**: предложен патч-релиз (Info, напоминание
  еженедельно).

## Страница монитора

Панель Firewall одним предложением сообщает, что не так, показывает CPU, память,
сессии и состояние HA, затем перечисляет IPsec-туннели (сначала упавшие),
интерфейсы без связи, лицензии, истекающие в течение 60 дней или истёкшие, и
доступный патч-релиз.

## Создайте пользователя REST API только для чтения для DumbMonit

1. В CLI FortiGate создайте профиль доступа, который может только читать состояние системы, сети и VPN и ничего больше.

    ```
    config system accprofile
    edit "dumbmonit-ro"
    set sysgrp read
    set netgrp read
    set vpngrp read
    set fwgrp none
    set loggrp none
    set utmgrp none
    set wanoptgrp none
    set wifi none
    next
    end
    ```

2. Создайте пользователя REST API dumbmonit с этим профилем, доверяя только адресу сервера DumbMonit (замените 192.0.2.10), затем сгенерируйте его токен: он выводится один раз.

    ```
    config system api-user
    edit "dumbmonit"
    set accprofile "dumbmonit-ro"
    set vdom "root"
    config trusthost
    edit 1
    set ipv4-trusthost 192.0.2.10 255.255.255.255
    next
    end
    next
    end
    execute api-user generate-key dumbmonit
    ```

3. В DumbMonit введите адрес FortiGate, например "fw.lan" (добавьте порт, если интерфейс администрирования не слушает 443), и вставьте токен как API token. FortiGate поставляется с самоподписанным сертификатом: отметьте «Принимать непроверяемый сертификат», если вы не установили собственный.

!!! warning
    Интерфейс считается упавшим, только когда он включён, имеет адрес и не имеет связи: свободные порты не учитываются. Обновления прошивки — это те, что FortiGuard предлагает самому FortiGate: без доступа к FortiGuard ни одно не сообщается.

## Учётные данные

| Учётные данные | Поля |
|---|---|
| API-токен | Токен, выведенный командой `execute api-user generate-key dumbmonit`, передаётся как `Authorization: Bearer`. |

Адрес: имя хоста или IP FortiGate, `host:port` или URL. Параметры:

- **Port** (443): HTTPS-порт интерфейса администрирования (`set
  admin-sport` в `config system global`).
- **VDOM**: виртуальный домен для чтения; пусто — тот, которому
  принадлежит пользователь REST API.
- **Принимать непроверяемый сертификат**: собственный сертификат FortiGate
  самоподписанный.
- **Тайм-аут на запрос** (15 с).

## Устранение неполадок

«FortiGate refused the API token (401)»: токен неверен или был
перегенерирован. Сгенерируйте новый командой `execute api-user generate-key
dumbmonit`.

«FortiGate refused … (403)»: адрес DumbMonit отсутствует среди доверенных
хостов пользователя, либо его профиль не может читать эту страницу. Необязательная страница,
отклонённая таким образом, просто не учитывается.

Нет данных по IPsec, лицензиям или прошивке: профиль не может их прочитать, либо
у FortiGate нет туннелей; остальное по-прежнему читается.
