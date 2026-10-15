# Nginx

Веб-сервер и обратный прокси: активные и ожидающие соединения, скорости
accept/handled/request, и здоровье upstream, когда доступен коммерческий API
NGINX Plus.

DumbMonit читает встроенный модуль `stub_status` Nginx — несколько простых
текстовых счётчиков, без API и без учётной записи на правильно
отфильтрованной сети. Когда установлен коммерческий NGINX Plus, DumbMonit
может также читать его JSON API для здоровья upstream и счётчиков по зонам.

## Что отслеживается

Все метрики с префиксом `dumbmonit_nginx_`. Метрики `_total` — это счётчики:
графики и правила превращают их в скорости.

| Метрика | Что | Метки |
|---|---|---|
| `connections_active` | соединения, открытые сейчас | |
| `connections_reading`, `connections_writing`, `connections_waiting` | соединения по тому, что они делают | |
| `accepts_total`, `handled_total` | принятые соединения, и фактически обработанные | |
| `requests_total` | запросы, обслуженные с момента запуска Nginx | |
| `version_info` | наличие; заголовок ответа `Server`, когда Nginx его отправляет | `server` |
| `plus_upstream_server_up` | 1, когда NGINX Plus отмечает сервер как рабочий | `upstream`, `server` |
| `plus_upstream_server_active`, `plus_upstream_server_requests_total`, `plus_upstream_server_fails_total`, `plus_upstream_server_5xx_total` | счётчики NGINX Plus на сервер | `upstream`, `server` |
| `plus_zone_requests_total`, `plus_zone_5xx_total`, `plus_zone_received_bytes_total`, `plus_zone_sent_bytes_total` | счётчики NGINX Plus на зону | `zone` |

Разрыв между `accepts_total` и `handled_total` означает соединения, которые
Nginx отбросил до того, как смог их обслужить (обычно достигнут
`worker_connections`). Метрики `plus_*` появляются только когда отмечено
«Read the NGINX Plus API» и NGINX Plus отвечает; в open-source сборке нет
понятия «upstream», о котором можно было бы сообщить только через
`stub_status`.

## Страница монитора

Панель над графиками читает то, что сохранила проверка; открытие страницы
никогда не запрашивает Nginx. Она показывает активные, читающие, пишущие и
ожидающие соединения, скорости accept/handled/request за 24 часа, и, с NGINX
Plus, здоровье upstream и трафик по зонам.

## Включение модуля stub_status в Nginx

1. В Nginx stub_status встроен по умолчанию: нужен только location, который его включает. Добавьте это в блок server, который Nginx уже обслуживает (или в собственный блок server на отдельном порту), затем перезагрузите Nginx.

    ```
    location /basic_status {
    stub_status;
    }
    ```

2. Ограничьте этот location хостом DumbMonit, чтобы никто другой не мог его прочитать; скорректируйте адрес под свой случай.

    ```
    location /basic_status {
    stub_status;
    allow 10.0.0.5;
    deny all;
    }
    nginx -s reload
    ```

3. Проверьте, что он отвечает, прежде чем настраивать DumbMonit.

    ```
    curl -s http://nginx.lan/basic_status
    ```

4. В DumbMonit введите хост Nginx, например «nginx.lan», и путь, если он отличается от «/basic_status». Если установлен NGINX Plus, отметьте «Read the NGINX Plus API», чтобы добавить здоровье upstream и счётчики по зонам.

!!! warning
    stub_status считает только соединения и запросы: ошибка конфигурации в
    блоке server, или backend, до которого nginx не может достучаться, здесь
    не отображается. Open-source сборка не имеет понятия «upstream»: оно
    существует только в NGINX Plus и читается через его собственный API, когда
    он включён.

Пример выше ограничивает location по IP — самая простая настройка для
домашней лаборатории; блок `basic_auth` работает не хуже, если страница
статуса не может сидеть за собственным правилом файрвола. Сам `stub_status`
нельзя защитить паролем средствами самого Nginx напрямую — пароль, если он
нужен, принадлежит окружающему блоку `location`.

## Учётные данные

| Учётные данные | Поля |
|---|---|
| Без пароля | Ничего: страница статуса фильтруется так, что до неё доходит только DumbMonit. |
| Basic auth | Пользователь и пароль блока, защищающего страницу статуса, если он есть. |

Адрес: имя хоста или IP, с портом, если он отличается от 80. Опции: протокол
(по умолчанию HTTP), порт, путь статуса (по умолчанию `/basic_status`),
проверка сертификата, таймаут запроса (10 с), и переключатель с версией для
API NGINX Plus.
