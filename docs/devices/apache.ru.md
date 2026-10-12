# Apache httpd

Веб-сервер: занятые и свободные воркеры, scoreboard воркеров по состояниям, и
скорость запросов/байтов, когда включён ExtendedStatus.

DumbMonit читает встроенный `mod_status` Apache, запрашиваемый с `?auto`, так
что он отвечает простыми строками `Key: Value` вместо HTML-страницы,
предназначенной для браузера.

## Что отслеживается

Все метрики с префиксом `dumbmonit_apache_`. Метрики `_total` — это счётчики:
графики и правила превращают их в скорости. Всё ниже `workers_busy`/
`workers_idle` и `scoreboard` требует `ExtendedStatus On`.

| Метрика | Что | Метки |
|---|---|---|
| `workers_busy`, `workers_idle` | рабочие процессы или потоки, по состоянию | |
| `scoreboard` | слоты воркеров по состоянию scoreboard | `state` |
| `requests_total` | запросы, обслуженные с момента запуска Apache | |
| `sent_bytes_total` | байты, отправленные с момента запуска Apache | |
| `uptime_seconds` | время с момента запуска Apache | |
| `requests_per_second`, `bytes_per_second`, `bytes_per_request` | собственные скользящие средние Apache | |
| `cpu_load_ratio` | собственное значение `CPULoad` от Apache | |
| `connections_total`, `connections_async_writing`, `connections_async_keepalive`, `connections_async_closing` | счётчики асинхронных соединений (MPM event и worker) | |
| `version_info` | наличие; заголовок ответа `Server` | `version` |

`scoreboard` всегда суммируется до настроенного числа слотов воркеров:
состояния — waiting, open_slot, starting, reading, sending, keepalive,
dns_lookup, closing, logging, finishing и idle_cleanup.

## Страница устройства

Панель над графиками читает то, что сохранила проверка; открытие страницы
никогда не запрашивает Apache. Она показывает занятые и свободные воркеры,
разбивку scoreboard, и, при наличии ExtendedStatus, скорость запросов и байтов
за 24 часа.

## Включение mod_status в Apache

1. Убедитесь, что mod_status загружен (он идёт с Apache и обычно включён по умолчанию).

    ```
    a2enmod status
    ```

2. Добавьте location для статуса, ограниченный хостом DumbMonit; скорректируйте адрес под свой случай. На Debian и Ubuntu это идёт в отдельном файле, в других местах — в основной конфигурации.

    ```
    <Location "/server-status">
    SetHandler server-status
    Require ip 10.0.0.5
    </Location>
    ```

3. Включите счётчики запросов и байтов (необязательно, но стоит того): без этого сообщаются только занятые/свободные воркеры и scoreboard.

    ```
    ExtendedStatus On
    ```

4. Перезагрузите Apache и проверьте, что он отвечает, прежде чем настраивать DumbMonit.

    ```
    apachectl graceful
    curl -s 'http://apache.lan/server-status?auto'
    ```

5. В DumbMonit введите хост Apache, например «apache.lan», и путь, если он отличается от «/server-status» (DumbMonit сам добавляет «?auto»).

!!! warning
    Без ExtendedStatus запросы и байты не сообщаются вовсе, а не как ноль:
    график остаётся пустым, а не лжёт плоской линией. Apache не публикует
    собственную версию в mod_status; вместо этого DumbMonit читает её из
    заголовка ответа Server, который некоторые дистрибутивы намеренно
    обрезают.

На Debian и Ubuntu location для статуса обычно уже существует в
`/etc/apache2/mods-available/status.conf`; достаточно изменить там строку
`Require`, новый файл не нужен. Где конфигурация — единый файл (большинство
других дистрибутивов), добавьте блок в основной virtual host.

## Учётные данные

| Учётные данные | Поля |
|---|---|
| Без пароля | Ничего: страница статуса фильтруется так, что до неё доходит только DumbMonit. |
| Basic auth | Пользователь и пароль блока, защищающего страницу статуса, если он есть. |

Адрес: имя хоста или IP, с портом, если он отличается от 80. Опции: протокол
(по умолчанию HTTP), порт, путь статуса (по умолчанию `/server-status`,
`?auto` добавляется автоматически), проверка сертификата и таймаут запроса
(10 с).
