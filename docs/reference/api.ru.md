# HTTP API

Всё, что делает веб-интерфейс, проходит через `/api/*`, и тот же API открыт для
ваших собственных скриптов, дашбордов и ассистентов: закрытой половины нет. Это
обычный JSON поверх HTTP с аутентификацией по cookie сессии (веб-интерфейс) или
по API-токену (всё остальное).

!!! warning "Alpha"
    DumbMonit находится в стадии alpha, как и его API: маршруты и поля ещё могут
    меняться. Версия API (см. ниже) меняется всякий раз, когда маршрут или поле
    удаляется либо переименовывается, так что клиент может это заметить.

## Описание OpenAPI

Каждый экземпляр описывает свой API в документе
[OpenAPI 3.1](https://spec.openapis.org/oas/v3.1.0):

```
GET /api/openapi.json
```

Он публичный — не содержит ни секретов, ни адресов, ни сведений о ваших
устройствах — и является исчерпывающим справочником: каждый маршрут, который
может вызвать клиент, его параметры, формы запроса и ответа, ответы с ошибками и
необходимая область (расширение `x-token-scope` у каждой операции: `public`,
`read`, `write` или `none` для маршрутов, которые токен не может вызывать никогда).
Таблицы ниже на этой странице — удобочитаемая сводка того же самого.

Намеренно исключены: протокол агента (`/api/ingest`,
`/api/agent/commands*`, `/api/agent/relay*`), на котором говорит только
[агент](../devices/agent.md); браузерные перенаправления входа через SSO
(`/api/auth/oidc/start`, `/api/auth/oidc/callback`) и перенаправление Spotify
(`/api/music/spotify/callback`); а также загрузки агента
(`/install.sh`, `/install.ps1`, `/download/*`).

Загрузите его в любой инструмент OpenAPI:

```bash
# Browse it, and try calls with your token, in Swagger UI
docker run --rm -p 8081:8080 \
  -e SWAGGER_JSON_URL=http://monit.lan:8080/api/openapi.json swaggerapi/swagger-ui
# then open http://localhost:8081 and paste "dmt_…" under Authorize → bearerAuth

# Generate a client library (here Python)
docker run --rm -v "$PWD:/local" openapitools/openapi-generator-cli generate \
  -i http://monit.lan:8080/api/openapi.json -g python -o /local/dumbmonit-client

# Or just list what is there
curl -s http://monit.lan:8080/api/openapi.json | jq -r '.paths | keys[]'
```

Postman и Insomnia импортируют его по тому же URL.

## Аутентификация {#authentication}

Два способа входа, принимаются на каждом защищённом маршруте:

- **Cookie сессии** — используется веб-интерфейсом. Войдите с паролем (или через
  SSO) и отправляйте cookie `dumbmonit_session` с флагом HttpOnly (SameSite Lax;
  `Secure` при `DUMBMONIT_COOKIE_SECURE=1`; 30 дней). Каждый запрос, изменяющий
  состояние (`POST`, `PUT`, `DELETE`), должен также содержать
  `X-Requested-With: DumbMonit` — защиту от CSRF, которую интерфейс добавляет
  автоматически; без неё запрос отклоняется с кодом `403`.
- **API-токен** — `Authorization: Bearer dmt_…`, для скриптов, дашбордов и
  ассистентов. Создаётся в Settings → **API & assistants** (те же токены
  использует [MCP-сервер](../using/assistant.md)) и показывается один раз. Cookie
  не участвует, поэтому заголовок `X-Requested-With` никогда не нужен.

```bash
# API token: one header, nothing else
curl -H 'Authorization: Bearer dmt_…' http://monit.lan:8080/api/targets
curl -H 'Authorization: Bearer dmt_…' -X POST http://monit.lan:8080/api/targets/4/probe

# Session: sign in (204 and a Set-Cookie header), then send the cookie
curl -c cookies.txt -X POST http://monit.lan:8080/api/auth/login \
  -H 'content-type: application/json' \
  -d '{"username":"admin","password":"…"}'
curl -b cookies.txt http://monit.lan:8080/api/targets
curl -b cookies.txt -X POST http://monit.lan:8080/api/targets \
  -H 'X-Requested-With: DumbMonit' -H 'content-type: application/json' \
  -d '{"name":"NAS","address":"nas.lan","kind":"snmp","credential":{"type":"snmp_community","community":"public"}}'
```

Если запрос содержит bearer-токен, cookie игнорируется: отозванный токен
отклоняется даже из браузера с выполненным входом, а не незаметно подменяется
сессией.

Токены регистрации агентов (`dmon_…`) — это другое: они работают только на
маршрутах агента (`/api/ingest`, `/api/agent/commands/*`, `/api/agent/relay*`).
Они говорят о том, что машина может обращаться к серверу, но не о том, *какая*
именно: на этих маршрутах привязанная машина должна дополнительно предъявить
собственный секрет привязки в `X-DumbMonit-Agent-Secret` (`dmab_…`), который
сервер выдаёт один раз при регистрации. См.
[Привязка](../devices/agent.md#binding-one-machine-one-agent).

Вход ограничен по частоте: после пяти неудачных попыток каждая следующая
отклоняется с кодом `429` и задержкой `Retry-After`, которая удваивается от 30 с
до 5 минут. Никогда не подбирайте пароли в цикле.

## Области (scopes)

У токена одна из двух областей. Третьей, более мощной, нет: то, что
администратор делает от имени своей учётной записи, недоступно ни одному токену.

| | токен `read` | токен `write` |
|---|---|---|
| Любой `GET`, доступный учётной записи *viewer* (устройства, оповещения, метрики, правила, каналы без их секретов, страницы статуса…) | да | да |
| `POST`, `PUT`, `DELETE`, доступные *администратору* (устройства, правила, silences, каналы, страницы статуса, packs, зонды, обнаружение…) | `403` | да |
| `/metrics`, `/federate`, `/prometheus/*` ([сбор метрик](metrics.md#scraping-dumbmonit)) | да | да |
| Инструменты MCP на чтение / запись ([ассистенты](../using/assistant.md#what-the-assistant-can-call)) | только чтение | оба |
| Учётные записи, сессии, двухфакторная аутентификация, SSO, журнал аудита (`/api/auth/*`, `/api/users/*`) | `403` | `403` |
| API-токены и токены регистрации агентов (`/api/tokens/*`, `/api/agent/tokens/*`) | `403` | `403` |
| Резервное копирование и восстановление (`/api/backup/*`) | `403` | `403` |

Токен `read` на маршруте записи получает `403` с сообщением, называющим токен и
недостающую область. Поскольку ни один токен не может перечислять, создавать или
отзывать токены, токен никогда не создаст другой — тем более более мощный.

## Безопасность токенов

- **Показывается один раз, хранится в виде хэша.** Токен — это `dmt_`, за которым
  следуют 128 случайных бит. Сервер хранит только его дайджест SHA-256 (и первые
  символы, `dmt_1a2b3c4d`, чтобы узнавать его в списке); токен в открытом виде
  существует только в ответе на его создание. Предъявленные токены сравниваются
  за постоянное время.
- **Срок действия.** При создании выберите 30 дней, 90 дней, 1 год или бессрочно.
  Истёкший токен получает `401` с `The API token "<name>" expired on …` и
  `WWW-Authenticate: Bearer error="invalid_token"`; он остаётся в списке с
  пометкой «истёк», пока вы его не отзовёте.
- **Разрешённые сети.** При желании ограничьте токен списком адресов или
  диапазонов CIDR (`192.168.1.0/24, 10.8.0.5`, не более 32 записей). Из любого
  другого места он получает `403` с `The API token "<name>" cannot be used from <address>.`
  Проверяется адрес TCP-клиента или — только если соединение приходит от прокси
  из `DUMBMONIT_TRUSTED_PROXIES` — адрес, который этот прокси сообщает в
  `X-Forwarded-For`. За необъявленным обратным прокси каждый запрос выглядит как
  идущий от прокси: объявите его, иначе ограничение отклонит всех.
- **Последнее использование.** Список токенов показывает, когда каждый токен
  использовался в последний раз и с какого адреса (обновляется не чаще раза в
  минуту или сразу при смене адреса): токен, которым больше никто не
  пользуется, стоит отозвать.
- **Привязан к создателю.** Токен запоминает учётную запись, которая его создала.
  Если эта учётная запись отключена, её токены отклоняются (`401`); если её
  понизили до operator или viewer, её токены `write` работают как токены `read`;
  если она удалена, её токены отзываются.
- **Отзыв** мгновенный: следующий вызов получает `401`.
- **Журналирование.** Каждая запись, выполненная с токеном, попадает в журнал
  сервера с именем и id токена, методом, путём (без строки запроса) и статусом —
  но никогда не токен и не тело запроса. Создание и отзыв токена фиксируются в
  журнале аудита (Settings → Security).

## Ограничения частоты {#rate-limits}

Каждый токен может делать 120 вызовов в минуту, REST и MCP вместе. Сверх этого
вызовы отклоняются с кодом `429` и заголовком `Retry-After` (секунды) до конца
минуты. Каждый ответ на запрос с токеном сообщает, где вы находитесь:

| Заголовок | Значение |
|---|---|
| `RateLimit-Limit` | Допустимое число вызовов за окно (120). |
| `RateLimit-Remaining` | Сколько вызовов осталось в текущем окне. |
| `RateLimit-Reset` | Секунд до начала нового окна. |

Дашборд Grafana с множеством панелей и коротким интервалом обновления может
достичь лимита: выдайте ему отдельный токен или увеличьте интервал обновления.

## Кросс-доменные запросы (CORS) {#cross-origin-requests-cors}

Страница, загруженная с другого origin, — ваш собственный дашборд, Swagger UI,
браузерный MCP-клиент — может вызывать API **с bearer-токеном, но никогда с
сессией**:

- ответ содержит `Access-Control-Allow-Origin` только если запрос содержал
  `Authorization: Bearer dmt_…` (или для публичного `GET /api/openapi.json`).
  `Access-Control-Allow-Credentials` не отправляется никогда, поэтому браузер
  никогда не прикладывает cookie сессии к кросс-доменному вызову;
- preflight (`OPTIONS`) на `/api/*` успешен, только если объявленные заголовки
  включают `authorization`: `204`, методы `GET, POST, PUT, DELETE`, заголовки
  `authorization, content-type, mcp-protocol-version, mcp-method, mcp-name`,
  кэшируется на 10 минут. `X-Requested-With` среди них нет, поэтому сессия на
  cookie никогда не сможет выполнить запись с другого origin;
- `Retry-After`, `WWW-Authenticate`, заголовки `RateLimit-*` и
  `X-DumbMonit-Api-Version` доступны для чтения вызывающему скрипту.

`DUMBMONIT_API_CORS_ORIGINS` задаёт, какие origin разрешены:

| Значение | Эффект |
|---|---|
| не задано (по умолчанию) | Любой origin (`Access-Control-Allow-Origin: *`). Безопасно, поскольку что-либо открывает только bearer-токен. |
| `https://grafana.example.com,https://dash.lan` | Только эти origin, возвращаются как есть. |
| `off` | Никаких заголовков CORS: только страницы с того же origin и не-браузерные клиенты. |

Тот же список определяет, какой `Origin` принимает [MCP-эндпоинт](../using/assistant.md#security-notes).

```js
// From a page on another origin. Never ship a write token in a public page.
const response = await fetch('https://monit.example.com/api/alerts', {
  headers: { Authorization: 'Bearer dmt_…' }
});
if (!response.ok) throw new Error((await response.json()).error);
const alerts = await response.json();
console.log(response.headers.get('RateLimit-Remaining'), 'calls left this minute');
```

## Версионирование {#versioning}

Каждый ответ под `/api` содержит версию API:

```
X-DumbMonit-Api-Version: 0.1
```

То же значение — это `info.version` в документе OpenAPI, рядом с
`x-server-version`, версией сервера, который ответил. Пока API имеет версию
`0.x`, он не заморожен; версия меняется всякий раз, когда маршрут или поле
удаляется либо переименовывается. Добавление маршрута или поля версию не меняет:
игнорируйте неизвестные поля.

## Ошибки

Каждая ошибка — это JSON: `{"error": "message"}`, написанный для человека.

| Статус | Когда |
|---|---|
| `400` | Запрос некорректен или значение недопустимо; сообщение называет его. |
| `401` | Нет действительной сессии или токена: токен отсутствует, неизвестен, отозван или истёк (с `WWW-Authenticate: Bearer`), либо создатель токена отключён. |
| `403` | Аутентифицирован, но не разрешено: viewer, operator вне обработки оповещений, токен `read` на маршруте записи, токен на маршруте учётных записей, токенов или резервных копий, токен, используемый из недопустимой сети, запись по cookie без `X-Requested-With` или любая запись в публичной демонстрации (с `"demo": true`). |
| `404` | Такого id не существует. Путь под `/api`, не соответствующий ни одному маршруту, отвечает `404 {"error": "Unknown API route: …"}`, а не HTML веб-интерфейса. |
| `405` | Такого метода на этом пути нет (например, `GET /api/mcp`). |
| `409` | Конфликт с текущим состоянием (дубликат, уже выполняющаяся команда…). |
| `429` | Превышен лимит частоты, с `Retry-After`. |
| `500` | Сбой сервера; подробности в журнале сервера, но никогда в ответе. |

```bash
$ curl -i -H 'Authorization: Bearer dmt_revoked…' http://monit.lan:8080/api/targets
HTTP/1.1 401 Unauthorized
www-authenticate: Bearer
{"error":"A valid API token is required."}

$ curl -i -X DELETE -H 'Authorization: Bearer dmt_…read…' http://monit.lan:8080/api/targets/4
HTTP/1.1 403 Forbidden
{"error":"This action needs a token with the \"write\" scope; the token \"Grafana\" is \"read\" only. Create a write token in Settings → API & assistants."}

$ curl -i -H 'Authorization: Bearer dmt_…' http://monit.lan:8080/api/alerts   # the 121st call this minute
HTTP/1.1 429 Too Many Requests
retry-after: 17
ratelimit-limit: 120
ratelimit-remaining: 0
{"error":"Rate limit reached (120 calls per minute per token). Retry in 17 s."}
```

[MCP-эндпоинт](../using/assistant.md#protocol-details) отвечает вместо этого в
JSON-RPC, со своими кодами ошибок.

## Учётные записи, вход и токены

В таблицах ниже *session* означает cookie сессии **или** API-токен; *admin* —
сессию администратора или токен `write`; *operator* — сессию оператора или
администратора либо токен `write`; *session only* — cookie сессии, но никогда
токен.

**Роли.** Учётная запись `viewer` выполняет любые `GET`. `operator` выполняет те
же чтения плюс операции записи по обработке оповещений, помеченные на этой
странице как *operator* (и `x-roles: [admin, operator]` в документе OpenAPI):
подтверждение (acknowledge), откладывание (snooze, окна обслуживания),
игнорирование правила для устройства, очистка истории закрытых оповещений.
Любая другая запись доступна только администратору и возвращает оператору `403`.

| Метод | Маршрут | Auth | Назначение |
|---|---|---|---|
| `GET` | `/api/auth/status` | public | `{"configured": bool, "authenticated": bool, "user": …, "oidc": {"enabled", "provider_name", "login_url"}}`. `configured: false` означает новый экземпляр: пока не существует первого администратора, каждый маршрут с пометкой *session* отвечает `401` — доступны только `status`, `setup`, `login` и `health`. |
| `POST` | `/api/auth/setup` | public | `{"setup_code": "…", "username": "admin", "password": "…"}`. Создаёт первого администратора на новом экземпляре (не менее 12 символов); `username` можно опустить, тогда он равен `admin`. `setup_code` — одноразовый код, выводимый в журналы сервера при запуске (или `DUMBMONIT_SETUP_CODE`); регистр, дефисы и пробелы игнорируются. `204`; сессию не открывает. `401` при неверном коде, `429` после слишком многих попыток, `409`, когда учётная запись уже существует. |
| `POST` | `/api/auth/login` | public | `{"username": "…", "password": "…"}`. `204` с `Set-Cookie`. `401` при неверном пароле, `429` при превышении лимита частоты. Если у учётной записи включена двухфакторная аутентификация, вместо этого `200` с `{"totp_required": true, "pending": "…"}` — тикет живёт пять минут и аннулируется после пяти неверных кодов: завершите вход через `/api/auth/login/totp`. |
| `POST` | `/api/auth/login/totp` | public | `{"pending": "…", "code": "123456"}` — тикет с первого шага и код (или код восстановления). `204` с cookie сессии. |
| `GET` | `/api/auth/oidc/start` | public | Перенаправляет браузер к провайдеру удостоверений. |
| `GET` | `/api/auth/oidc/callback` | public | Возврат от провайдера (`code`, `state`): открывает сессию и перенаправляет в интерфейс. |
| `GET` | `/api/auth/me` | session only | Текущая учётная запись: `id`, `username`, `display_name`, `role` (`admin`, `operator`, `viewer`), `auth` (`password`, `oidc`), `disabled`, `totp_enabled`, `created_at`, `last_login_at`. |
| `POST` | `/api/auth/logout` | session only | Завершает сессию и очищает cookie. |
| `POST` | `/api/auth/password` | session only | `{"current_password": "…", "new_password": "…"}`. Завершает все остальные сессии. |
| `GET` | `/api/auth/totp` | session only | Состояние двухфакторной аутентификации текущей учётной записи: `enabled`, `pending`, `recovery_codes_left`. |
| `POST` | `/api/auth/totp/enroll` | session only | `{"password": "…"}`. Предлагает секрет: `secret` (base32), `otpauth_uri`, `issuer`, `account`. Ничего не применяется, пока не подтверждено. |
| `POST` | `/api/auth/totp/verify` | session only | `{"code": "123456"}`. Подтверждает регистрацию и возвращает `{"recovery_codes": […]}` — показываются один раз. |
| `DELETE` | `/api/auth/totp` | session only | `{"password": "…"}`. Удаляет второй фактор. `204`. |
| `GET` | `/api/auth/audit?limit=200` | admin, session only | Последние события безопасности: `id`, `at`, `actor`, `action` (`login`, `login.failed`, `password.changed`, `token.created`, `user.updated`, …), `subject`, `ip`. |
| `GET` | `/api/auth/oidc/config` | admin, session only | Настройки SSO и их источник (окружение или база данных); секрет клиента не возвращается никогда. |
| `PUT` | `/api/auth/oidc/config` | admin, session only | `{"issuer", "client_id", "client_secret", "provider_name", "scopes", "auto_create", "admin_groups", "groups_claim", "public_url"}`. Отсутствующий или пустой `client_secret` сохраняет прежний; отсутствующий `auto_create` означает «выключено». |
| `DELETE` | `/api/auth/oidc/config` | admin, session only | Забывает сохранённые настройки SSO (переменные окружения, если они есть, снова вступают в силу). `204`. |
| `POST` | `/api/auth/oidc/test` | admin, session only | Загружает discovery-документ провайдера и сообщает, что нашёл. |
| `GET` | `/api/users` | admin, session only | Все учётные записи в форме `/api/auth/me`. |
| `POST` | `/api/users` | admin, session only | `{"username", "display_name", "role", "password"}`. `password` можно опустить, только если включён SSO. `201`. |
| `PUT` | `/api/users/{id}` | admin, session only | Любое из `display_name`, `role`, `disabled`, `password`; опущенное поле сохраняет значение. Нельзя понизить или отключить последнего администратора. |
| `DELETE` | `/api/users/{id}` | admin, session only | `204`. Нельзя удалить себя и последнего администратора. |
| `DELETE` | `/api/users/{id}/totp` | admin, session only | Сбрасывает второй фактор другой учётной записи (заблокированный коллега). `204`. |
| `GET` | `/api/tokens` | session only | Все API-токены: `id`, `name`, `prefix`, `scope`, `created_at`, `created_by` (учётная запись, создавшая его), `expires_at` (`null`: бессрочный), `expired`, `allowed_networks` (пусто: откуда угодно), `last_used_at`, `last_used_ip`, `revoked_at`. Секрет — никогда. |
| `POST` | `/api/tokens` | admin, session only | `{"name": "Grafana", "scope": "read", "expires_in_days": 90, "allowed_networks": ["192.168.1.0/24"]}`. `name` обязательно (не более 80 символов); `scope` — `read` (по умолчанию) или `write`; `expires_in_days` от 1 до 3650, `null` или отсутствие — для бессрочного токена; `allowed_networks` принимает до 32 адресов или диапазонов CIDR, хранится в сетевой форме (`192.168.1.7/24` превращается в `192.168.1.0/24`), пусто или отсутствует — откуда угодно. `201` с полями токена плюс `secret` (показывается один раз). `400` указывает недопустимую сеть. |
| `DELETE` | `/api/tokens/{id}` | admin, session only | Отзыв. `204`; `404`, если токен неизвестен или уже отозван. |

Токен в списке:

```json
{
  "id": 3, "name": "Grafana", "prefix": "dmt_1a2b3c4d", "scope": "read",
  "created_at": "2026-10-01T09:12:00Z", "created_by": "admin",
  "expires_at": "2026-12-30T09:12:00Z", "expired": false,
  "allowed_networks": ["192.168.1.0/24"],
  "last_used_at": "2026-10-01T09:30:00Z", "last_used_ip": "192.168.1.20",
  "revoked_at": null
}
```

## Заголовки ответа

Помимо `X-DumbMonit-Api-Version` ([версионирование](#versioning)), заголовков
`RateLimit-*` ([ограничения частоты](#rate-limits)) и заголовков CORS
([кросс-доменные запросы](#cross-origin-requests-cors)), каждый ответ содержит
`X-Content-Type-Options: nosniff`,
`Referrer-Policy: same-origin` и `Content-Security-Policy`. Политика начинается
с `default-src 'none'` и открывает только то, что действительно использует
интерфейс — и всё это раздаётся самим DumbMonit:

```
default-src 'none'; style-src 'self' 'unsafe-inline'; img-src 'self' data:;
font-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'self';
script-src 'self' 'nonce-<per response>'; frame-ancestors 'none'
```

Сторонних origin в ней нет и быть не должно: шрифты, библиотека графиков и все
ресурсы поставляются внутри бинарного файла. Встроенные скрипты страницы (тема,
применяемая до первой отрисовки, начальная загрузка SvelteKit) разрешены nonce,
который генерируется заново для каждого ответа, поэтому `'unsafe-inline'` к
скриптам никогда не применяется. К стилям он применяется и не может быть
убран: Svelte устанавливает у элементов атрибуты `style=`, которые не могут
покрыть ни nonce, ни хэш.

Публичные страницы статуса предназначены для встраивания: `/s/<slug>` и её
компактный вид `/s/<slug>/embed` — и только они — получают ту же политику без
`frame-ancestors`. Всё остальное, включая страницы подтверждения подписки и
отписки под `/s/`, также содержит `X-Frame-Options: DENY`.

Если вы размещаете DumbMonit за обратным прокси, не позволяйте ему добавлять
второй заголовок `Content-Security-Policy`: браузеры применяют пересечение всех
таких политик, а политика прокси по умолчанию без nonce оставляет пустую
страницу.

## Состояние (Health)

| Метод | Маршрут | Auth | Назначение |
|---|---|---|---|
| `GET` | `/api/health` | public | Версия сервера и состояние его зависимостей. Всегда `200`: отказавший компонент указан в теле. |
| `GET` | `/api/update` | session или токен | Запущенная версия, последний релиз, `update_available`; сервер обращается к GitHub не чаще раза в день. `POST /api/update/check` проверяет сейчас, `PUT /api/update/settings` включает или выключает проверку. См. [Обновление](../install/update.md). |

```json
{"status":"ok","version":"0.1.0","database":{"ok":true},"victoria":{"ok":true,"embedded":true}}
```

`status` равен `degraded`, когда компонент отказывает; тогда у этого компонента
есть строка `error`. `victoria.embedded` равен `true`, когда сервер запускает
собственный VictoriaMetrics, и `false`, когда `DUMBMONIT_VM_URL` указывает на
внешний.

## Резервное копирование и восстановление

| Метод | Маршрут | Auth | Назначение |
|---|---|---|---|
| `GET` | `/api/backup` | admin, session only | Что содержал бы пакет (`contents`, одна строка на раздел с его количеством), версия формата пакета, где хранится секрет экземпляра (`secret_source`: `file` или `environment`) и состояние локальных резервных копий по расписанию (`schedule`). |
| `POST` | `/api/backup` | admin, session only | `{"passphrase": "…", "include_account_secrets": false}`. Возвращает сам пакет в виде JSON-файла: открытый заголовок (`format`, `version`, `created_at`, `summary`, `kdf`) и `payload`, зашифрованный AES-256-GCM ключом, выведенным из парольной фразы с помощью Argon2id. Не менее 16 символов, иначе `400`. |
| `POST` | `/api/backup/restore` | admin, session only | `{"bundle": {…}, "passphrase": "…", "apply": false}`. Пробный запуск, если `apply` не равно `true`; оба режима выполняют один и тот же код, пробный — внутри транзакции, которая откатывается. Отвечает отчётом: по разделам `created`, `updated`, `skipped` и `notes`, плюс `warnings`. `400` при неверной парольной фразе, изменённом файле или пакете, записанном в более новой версии формата. |
| `POST` | `/api/backup/local` | admin, session only | Немедленно записывает одну локальную копию в стиле плановой и отвечает новым `schedule`. |

Пакет содержит все учётные данные экземпляра, перешифрованные парольной
фразой, поэтому эти маршруты отклоняют API-токены с пояснением: доступ получает
только администратор, вошедший в веб-интерфейс. См.
[Резервное копирование и восстановление](../install/backup.md).

## Типы устройств

| Метод | Маршрут | Назначение |
|---|---|---|
| `GET` | `/api/collectors` | Все виды устройств, которые этот сервер может мониторить: `kind`, `label`, `summary`, `examples`, `credential_types`, `address_hint`, `default_port`, `setup` (`title`, `steps`, `warning`, `doc_url`) и `options` (`key`, `label`, `help`, `placeholder`, `default`, `required`, `input`, `choices`). Виды, добавленные [пакетом интеграции](../packs/index.md), перечисляются так же, как `pack.<id>`. |

## Пакеты интеграций

Сессия администратора или API-токен (область `write` для изменений). См.
[Пакеты интеграций](../packs/index.md).

| Метод | Маршрут | Назначение |
|---|---|---|
| `GET` | `/api/packs` | Установленные пакеты: `id`, `version`, `label`, `summary`, `kind` (`null` для пакета только с профилями SNMP), `enabled`, `installed_at`, `sha256`, `metrics`, `rules` (uid), `snmp_profiles`, `targets` (использующие его устройства), `warnings` и `error`, когда сохранённый пакет больше не проходит проверку. |
| `GET` | `/api/packs/{id}` | Один пакет. |
| `POST` | `/api/packs` | Установка или обновление. Тело: YAML (`application/yaml`, `text/plain`…) или `{"yaml": "…"}` в JSON. `201` при создании, `200` при обновлении или отсутствии изменений, с `outcome` (`created`, `updated`, `unchanged`), `pack`, `rules_added` и `restart_required` (профили SNMP). `400` перечисляет все ошибки проверки. |
| `PUT` | `/api/packs/{id}/enable`, `/api/packs/{id}/disable` | Зарегистрировать или отозвать тип устройства пакета; устройства, история и правила остаются. |
| `DELETE` | `/api/packs/{id}` | Удаление вместе с его правилами. `409`, пока его используют устройства. |

## Цели (устройства)

| Метод | Маршрут | Назначение |
|---|---|---|
| `GET` | `/api/targets` | Список всех устройств. |
| `POST` | `/api/targets` | Создать одно. `201` с устройством. Для SNMP определение профиля запускается в фоне. |
| `GET` | `/api/targets/{id}` | Одно устройство. |
| `PUT` | `/api/targets/{id}` | Заменить его. Если опустить `credential`, сохраняется прежний; если опустить `profile_id`, сохраняется определённый профиль (`""` очищает). Установка `enabled: false` очищает оповещения устройства без уведомлений. |
| `DELETE` | `/api/targets/{id}` | `204`. Очищает оповещения устройства без уведомлений и удаляет его временные ряды из VictoriaMetrics (по возможности). |
| `POST` | `/api/targets/{id}/probe` | Опросить сейчас: `{"sample_count": 42, "series": ["if_octets_in{host=\"Core switch\",ifname=\"eth0\"}", …]}` — ключи рядов в том виде, в каком их произвёл коллектор, без префикса `dumbmonit_`, который записывающий компонент добавляет на пути в VictoriaMetrics. |
| `POST` | `/api/targets/{id}/discover` | Повторить определение профиля: `{"profile_id": "host-resources"}` или `null`. |

Устройство в возвращаемом виде:

```json
{
  "id": 4,
  "name": "Core switch",
  "address": "192.168.1.2",
  "kind": "snmp",
  "profile_id": "host-resources",
  "parent_id": null,
  "via_agent": null,
  "interval_secs": 60,
  "enabled": true,
  "tags": {"role": "network"},
  "credential_kind": "SNMP community",
  "last_probe_at": "2026-09-15 13:20:05",
  "last_error": null,
  "error_kind": null
}
```

Сам секрет не возвращается никогда: `credential_kind` — это метка. Временные
метки сервера указаны в UTC без суффикса. `error_kind` классифицирует
`last_error`: `down` (устройство не ответило) или `config` (наша сторона —
учётные данные, адрес, параметр); именно это интерфейс показывает как
*Unreachable* или *Misconfigured*; `null`, когда последний опрос прошёл успешно.

Создание:

```json
{
  "name": "Core switch",
  "address": "192.168.1.10",
  "kind": "snmp",
  "interval_secs": 60,
  "parent_id": null,
  "tags": {},
  "credential": {"type": "snmp_community", "community": "public"}
}
```

Формы учётных данных (`type`): `none`; `snmp_community` (`community`); `snmp_v3`
(`username`, необязательный `auth: {protocol, passphrase}` с `md5`, `sha1`,
`sha224`, `sha256`, `sha384` или `sha512`, необязательный `privacy: {protocol,
passphrase}` с `des`, `aes128`, `aes192` или `aes256`, необязательный `context`);
`api_token` (`token` или две половины `token_id` и `secret`, которые сервер
объединяет в `user@realm!name=secret` для Proxmox VE и PBS);
`username_password` (`username`, `password`).
`interval_secs` по умолчанию равен 60 и не может быть меньше 10. `name` — не
более 200 символов, `address` — не более 253 (сверх этого `400`). `parent_id`
должен указывать на существующее устройство (`400 Parent device N not found.`).
Параметры типа помещаются в `tags` под своим ключом.

`via_agent` называет [релейного агента](../install/remote-site.md), который
опрашивает устройство из своей сети вместо сервера; `null` (по умолчанию)
означает, что опрашивает сервер. Это должно быть устройство `agent` (иначе
`400`), устройство не может ретранслировать само себя, а устройство `agent`
нельзя ретранслировать. При `PUT` *опущенный* `via_agent` сохраняет прежнее
значение, как `profile_id` и `credential`; отправьте `null`, чтобы вернуться к
прямому опросу. `POST
/api/targets/{id}/probe` и `/discover` на устройстве за релеем идут через релей
и ждут его ответа (`409`, когда опрос уже выполняется, `400 Timed out: relay agent …`,
когда он не отвечает).

## Обнаружение

| Метод | Маршрут | Auth | Назначение |
|---|---|---|---|
| `POST` | `/api/discovery` | admin | `{"cidr": "192.168.1.0/24", "community": "public", "port": 161, "timeout_ms": 1000}`. Сканирует сеть в поисках SNMP-устройств. `cidr` обязателен; `community` по умолчанию `public`; `timeout_ms` ограничивается диапазоном 100–10 000. Сети больше 4096 адресов отклоняются. Сканирование — это `POST` с JSON-телом, чтобы community никогда не появлялся в URL или строке журнала. |

```json
{
  "scanned": 254,
  "devices": [
    {"address": "192.168.1.10", "sysname": "sw-core", "sysdescr": "…", "sysobjectid": "1.3.6.1.4.1.8072.3.2.10", "suggested_profile": "host-resources"}
  ]
}
```

## Руководство первого запуска

Три шага, которые обзор показывает на новом экземпляре. Состояние хранится на
экземпляре, а не в браузере, поэтому пропуск шага пропускает его везде.

| Метод | Маршрут | Auth | Назначение |
|---|---|---|---|
| `GET` | `/api/onboarding` | session | Текущее состояние экземпляра. |
| `PUT` | `/api/onboarding` | admin | `{"skipped": true}` убирает всё руководство; `{"dismissed": ["test"]}` убирает отдельные шаги. Опущенное поле сохраняет своё значение; id шага, отличный от `device`, `channel` или `test`, — это `400`. Возвращает тот же документ, что и `GET`. |

```json
{
  "skipped": false,
  "complete": false,
  "completed_at": null,
  "dismissed": [],
  "has_target": true,
  "has_channel": true,
  "notification_confirmed": false
}
```

`has_target`, `has_channel` и `notification_confirmed` пересчитываются при
каждом чтении: это факты об экземпляре, а не флаги, которые устанавливает
интерфейс. `notification_confirmed` становится истинным, когда сообщение
действительно ушло через какой-либо канал — тестовое сообщение считается,
канал, который только создан, — нет.

`complete` фиксируется и никогда не возвращается в false. Сервер устанавливает
его, когда три шага завершены (выполнены или отклонены), а также при самом
первом чтении экземпляра, у которого уже были устройство и канал, — поэтому
обновление уже работающего экземпляра никогда не показывает ему руководство
первого запуска.

## Метрики {#metrics}

Оба маршрута проксируют VictoriaMetrics и возвращают ряды в форме Prometheus:
`[{"metric": {labels…}, "values": [[timestamp_seconds, "value"], …]}]`.

| Метод | Маршрут | Назначение |
|---|---|---|
| `GET` | `/api/metrics/query?query=…` | Мгновенный запрос MetricsQL. |
| `GET` | `/api/metrics/query_range?query=…&start=…&end=…&step=…` | Запрос диапазона. `start` и `end` — **миллисекунды** с начала эпохи; `step` в секундах и увеличивается так, чтобы ряд никогда не превышал 2000 точек. |

```bash
curl -b cookies.txt -G http://localhost:8080/api/metrics/query \
  --data-urlencode 'query=avg by (target, host) (dumbmonit_cpu_usage_percent)'
```

```json
[{"metric":{"host":"ThinkpadE14","target":"3"},"values":[[1757941205,"12.5"]]}]
```

## Сбор метрик (вне `/api`)

Для существующего Prometheus или Grafana. Оба принимают API-токен `read` в
`Authorization: Bearer dmt_…` (cookie сессии тоже работает); сборщик не
отправляет заголовок `X-Requested-With` и не нуждается в нём — эти маршруты
только читают. См. [справочник метрик](metrics.md#scraping-dumbmonit) с готовым
для копирования `scrape_config` и источником данных Grafana.

| Метод | Маршрут | Назначение |
|---|---|---|
| `GET` | `/metrics` | Состояние самого экземпляра в текстовом формате экспозиции Prometheus: планировщик, опросы, записи, оповещения, уведомления, агенты, база данных, VictoriaMetrics. Несколько десятков рядов, никогда по одному на устройство. |
| `GET` | `/federate` | Измерения в том же формате. `match[]` выбирает ряды (повторяемый, не более 10, по умолчанию `{__name__=~"dumbmonit_.*"}`), `max_lookback` расширяет окно, в котором ищется последняя точка (по умолчанию `5m`). `400` сверх 8 MiB, с указанием, что сузить. |
| `GET`, `POST` | `/prometheus/api/v1/{route}` | Читающая половина API Prometheus, ретранслируемая в хранилище, чтобы Grafana могла использовать `http://…/prometheus` как источник данных Prometheus. Только `query`, `query_range`, `query_exemplars`, `series`, `labels`, `label/{name}/values`, `metadata` и `status/buildinfo`; всё остальное — `404`. |

`DUMBMONIT_METRICS_PUBLIC=true` отдаёт все три без токена — только за
брандмауэром, поскольку они содержат каждое измерение каждого устройства.

## Другие маршруты

Маршруты, не описанные выше; документ OpenAPI приводит их параметры и формы
ответов. *read* и *write* — необходимая область токена; *none* означает, что
токен не принимается (только сессия).

| Метод | Маршрут | Область | Назначение |
|---|---|---|---|
| `POST` | `/api/targets/reorder` | write | Изменить порядок устройств. |
| `GET` | `/api/targets/{id}/security` | read | Оценка безопасности, класс и проверки одного устройства. |
| `GET` | `/api/security/summary` | read | Оценка безопасности каждого оценённого устройства, худшие первыми. |
| `GET` | `/api/targets/{id}/webchange/pages` | read | Страницы, отслеживаемые наблюдением за сайтом. |
| `GET` | `/api/targets/{id}/webchange/changes` | read | Изменения, найденные на отслеживаемом сайте, сначала новые. |
| `GET` | `/api/targets/{id}/webchange/changes/{change_id}` | read | Одно изменение с обеими версиями и построчным diff. |
| `GET` | `/api/targets/{id}/webchange/snapshots/{snapshot_id}/screenshot` | read | Скриншот снимка. |
| `POST` | `/api/targets/{id}/webchange/check` | write | Проверить отслеживаемый сайт сейчас. |
| `GET` | `/api/targets/{id}/opnsense/gateways`, `/traffic`, `/health` | read | OPNsense: шлюзы и WAN-адреса; интерфейсы, межсетевой экран и DHCP; система, службы, туннели и прошивка. |
| `GET` | `/api/targets/{id}/truenas/storage`, `/protection`, `/health` | read | TrueNAS: пулы, датасеты и диски; scrub и задачи защиты данных; система, оповещения и службы. |
| `GET` | `/api/targets/{id}/ad`, `/ad/findings` | read | Active Directory: домен, контроллеры, привилегированные группы и находки; необработанные находки безопасности. |
| `GET` | `/api/targets/{id}/redfish` | read | Аппаратное состояние сервера, прочитанное с его контроллера управления. |
| `GET` | `/api/targets/{id}/observability` | read | Состояние сервера логов или метрик. |
| `GET` | `/api/targets/{id}/backend` | read | Состояние Redis, MongoDB, RabbitMQ или CrowdSec. |
| `GET` | `/api/targets/{id}/proxy` | read | Состояние обратного прокси или регистрации домена. |
| `GET` | `/api/public/status/{slug}/badge.svg`, `/response.svg`, `/uptime.svg` | public | Бейджи общего статуса, среднего времени отклика и аптайма страницы статуса. |
| `GET` | `/api/public/status/{slug}/components/{key}/badge.svg`, `/uptime.svg`, `/response.svg` | public | Те же бейджи для одного сервиса страницы. |
| `GET` | `/api/public/status/{slug}/banner.js` | public | Скрипт баннера статуса для сторонних сайтов. |
| `GET` | `/api/public/status/{slug}/rss` | public | Инциденты в виде RSS-ленты. |
| `GET` | `/api/music/now` | read | Что играет на экранах (монитор комнаты). |
| `PUT`, `DELETE` | `/api/music/link` | write | Все экраны воспроизводят эту ссылку Spotify, Deezer или YouTube; либо останавливают встроенный плеер. |
| `GET`, `DELETE` | `/api/music/spotify` | read, write | Подключённая учётная запись Spotify (никогда не токен); отключить её и забыть её токены. |
| `POST` | `/api/music/spotify/authorize`, `/complete` | none | Начать подключение Spotify; завершить loopback-подключение. Только сессия. |
| `GET` | `/api/music/spotify/token` | none | Краткоживущий токен доступа Spotify для плеера экрана. Только сессия. |
| `GET`, `PUT` | `/api/music/speaker` | read, write | Колонка Spotify экрана в том виде, как её показывает Settings; переименовать её (admin). |
| `POST` | `/api/music/speaker/report`, `/play` | none | Экран сообщает состояние своей колонки; воспроизвести на ней учётную запись. Только сессия. |

## Оповещения

| Метод | Маршрут | Назначение |
|---|---|---|
| `GET` | `/api/alerts` | Активные оповещения (pending, firing, suppressed, недавно закрытые). Оповещения удалённых или приостановленных устройств не перечисляются никогда. |
| `GET` | `/api/alerts/history?since=2026-09-01T00:00:00Z&limit=200&dismissed=false` | Переходы между фазами. `since` — RFC 3339, по умолчанию последние семь дней; `limit` должен быть положительным. `dismissed` (по умолчанию `false`) исключает записи, очищенные двумя маршрутами ниже; очистка ничего не уничтожает, а лишь скрывает из вида по умолчанию. |
| `POST` | `/api/alerts/history/{id}/dismiss` | Убирает одну запись `resolved` из вида истории по умолчанию. `204`. `404` для неизвестного id или ещё активного перехода. Operator; попадает в аудит. |
| `POST` | `/api/alerts/history/dismiss-resolved` | Убирает сразу все ещё не очищенные записи `resolved` («Clear all resolved»). Возвращает `{"dismissed": <count>}`. Operator; попадает в аудит. |
| `POST` | `/api/alerts/{fingerprint}/ack` | Подтверждение: `{"duration_secs": 14400, "note": "…"}` или `{"until": "2026-09-22T18:00:00Z"}` (одно из двух; если ни одного — 4 часа, максимум 30 дней). `{"until": null}` снимает его. Возвращает оповещение. Operator; попадает в аудит. |
| `DELETE` | `/api/alerts/{fingerprint}/ack` | Снять подтверждение. Возвращает оповещение. Operator. |

Активное оповещение:

```json
{
  "fingerprint": "…",
  "rule_uid": "disk_almost_full",
  "rule_name": "Disk almost full",
  "severity": "warning",
  "target_id": 4,
  "series_key": "…",
  "labels": {"host": "Core switch", "mountpoint": "/", "target": "4"},
  "phase": "firing",
  "effective_phase": "suppressed",
  "suppressed": true,
  "suppressed_by": 2,
  "silenced": false,
  "learning": false,
  "acked": true,
  "acked_until": "2026-09-15T17:20:00Z",
  "acked_by": "admin",
  "ack_note": "replacing the disk",
  "value": 95.96,
  "score": null,
  "condition_since": "2026-09-15T12:00:00Z",
  "firing_since": "2026-09-15T12:15:00Z",
  "last_eval_at": "2026-09-15T13:20:00Z",
  "last_notified_at": null,
  "notify_count": 0
}
```

Запись истории содержит `id`, `fingerprint`, `rule_uid`, `target_id`,
`from_phase`, `to_phase`, `severity`, `value`, `notified`, `reason` (пусто или
причина, по которой ничего не было отправлено: `learning: would have fired`,
`suppressed: device 2 unreachable`, `maintenance window`, `acknowledged by admin`,
`device removed or disabled`), `at` и `dismissed` (убрано из активного вида;
сама строка сохраняется). `acked` истинно, пока `acked_until` находится в
будущем: напоминания и эскалации приостанавливаются, закрытие всё равно
уведомляется и снимает подтверждение.
`target_id`, которого больше не существует, интерфейс показывает как «(deleted device)».

### Правила

| Метод | Маршрут | Назначение |
|---|---|---|
| `GET` | `/api/alerts/rules` | Все правила. |
| `POST` | `/api/alerts/rules` | Создать. `201`. |
| `PUT` | `/api/alerts/rules/{id}` | Обновить. Встроенное правило сохраняет свой запрос. |
| `DELETE` | `/api/alerts/rules/{id}` | `204`. Встроенные правила удалить нельзя. |
| `POST` | `/api/alerts/rules/{id}/enable` | `{"enabled": false}`. Возвращает правило. |

Правило:

```json
{
  "id": 2, "uid": "cpu_high", "name": "High CPU",
  "description": "CPU load sustained above 90%.",
  "kind": "threshold",
  "query": "avg by (target, host) (dumbmonit_cpu_load_percent or dumbmonit_proxmox_node_cpu_percent or dumbmonit_cpu_usage_percent)",
  "operator": ">", "threshold": 90, "for_secs": 600,
  "severity": "warning",
  "selector": {"kind": "all"},
  "channels": [],
  "params": {"k": 3.5, "alpha": 0.05, "mad_floor_abs": 1e-6, "mad_floor_rel": 0.01, "min_samples": 3},
  "unit": "%", "repeat_secs": 21600, "escalate_after_secs": 3600,
  "enabled": true, "builtin": true
}
```

Для создания нужны как минимум `name` и `query`; `kind` (`threshold`,
`anomaly`, `predict`), `operator`, `threshold`, `for_secs`, `severity`
(`info`, `warning`, `critical`), `selector` (`{"kind":"all"}`,
`{"kind":"ids","ids":[…]}` или `{"kind":"labels","labels":{…}}`), `channels`
(id каналов; пусто означает все включённые каналы), `unit`, `repeat_secs`,
`escalate_after_secs` и `enabled` необязательны.

### Silences (окна обслуживания)

| Метод | Маршрут | Назначение |
|---|---|---|
| `GET` | `/api/alerts/silences` | Все окна, с `active_now`, `active_until` и `next_start_at` (RFC 3339, вычисляются сервером). |
| `POST` | `/api/alerts/silences` | Создать. `201`. Operator. |
| `DELETE` | `/api/alerts/silences/{id}` | `204`. Operator. |

```json
{
  "name": "Sunday backups",
  "comment": "NAS is busy",
  "target_id": 4,
  "matchers": {},
  "schedule": {"kind": "weekly", "days": [6], "start_minute": 120, "end_minute": 240, "timezone": "Europe/Paris"},
  "enabled": true
}
```

Разовое расписание — `{"kind": "once", "starts_at": "2026-09-20T22:00:00Z",
"ends_at": "2026-09-21T02:00:00Z"}`. Ежемесячное —
`{"kind": "monthly", "days": [1], "nth_weekdays": [{"nth": 1, "weekday": 6}],
"start_minute": 120, "duration_minutes": 120, "timezone": "Europe/Paris"}` —
`nth` от 1 до 5 или `-1` для последнего в месяце; месяц без такого вхождения
пропускается. Дни: 0 = понедельник … 6 = воскресенье (1–31 для `days`
ежемесячного расписания); минуты отсчитываются от местной полуночи (0–1439). В
повторяющемся расписании `timezone` (имя IANA) главнее `utc_offset_minutes` и
сохраняет окно на его местном часе при переходе на летнее/зимнее время;
неизвестное имя зоны отклоняется.

### Переопределения для отдельных устройств {#per-device-overrides}

Правило можно настроить для одного устройства, не меняя само правило.
`{"enabled": false}` — это то, что записывает действие **Ignore** интерфейса
(на оповещении, странице устройства или правиле): устройство исключается из
этого одного правила насовсем, но обратимо, без отключения правила для всех
остальных.

| Метод | Маршрут | Назначение |
|---|---|---|
| `GET` | `/api/alerts/overrides?target_id=4` | Все переопределения или переопределения одного устройства: `rule_uid`, `target_id`, `threshold`, `clear_threshold`, `enabled`. |
| `GET` | `/api/alerts/rules/{id}/overrides` | Переопределения одного правила. |
| `PUT` | `/api/alerts/rules/{id}/overrides/{target_id}` | `{"threshold": 95, "clear_threshold": 90, "enabled": true}` — каждое поле необязательно; `null` означает «как в правиле». Создаёт или заменяет. Operator может отправлять только `{"enabled": false}` (ignore); пороговые переопределения устройства, если они есть, сохраняются. |
| `DELETE` | `/api/alerts/rules/{id}/overrides/{target_id}` | `204`. Для operator только снимает ignore: пороговые переопределения остаются. |

## Политика уведомлений

| Метод | Маршрут | Назначение |
|---|---|---|
| `GET` | `/api/notify/policy` | Глобальная политика: `batch_window_secs` (0 = отправлять сразу), `max_per_hour` на канал (0 = без ограничений), `flap_events`, `flap_window_secs`, `flap_hold_secs` (0 = без обнаружения флаппинга), `public_url` (ссылки в сообщениях; пусто = `DUMBMONIT_PUBLIC_URL`), `escalate_after_secs` и `escalate_channel` (0 / `null` = без эскалации). |
| `PUT` | `/api/notify/policy` | Те же поля, каждое необязательно: опущенное поле сохраняет значение. `"escalate_channel": null` отключает эскалацию. Возвращает политику. |
| `POST` | `/api/notify/match-preview` | `{"matcher": {…}}` → какие устройства выбирает фильтр маршрутизации канала: `devices` (`id`, `name`, `kind`, `tags`, `matched`), `matched`, `total`, `rules`, `excluded_rules`. |

## Каналы уведомлений

| Метод | Маршрут | Назначение |
|---|---|---|
| `GET` | `/api/notify/kinds` | Все типы каналов с их полями `settings` и `secrets` (`key`, `label`, `required`, `input`, `help`, `placeholder`, `options`, `shape`, `default`) и `doc_url`, например `https://dumbmonit.readthedocs.io/en/latest/notifications/#discord`. |
| `GET` | `/api/notify/channels` | Все каналы: `id`, `name`, `kind`, `enabled`, `settings`, `has_secret`, `last_error`, `last_sent_at`, `policy` (`min_severity`, `notify_resolved`, `min_interval_secs`, `quiet_hours`, `matcher`). Секреты не возвращаются никогда. |
| `POST` | `/api/notify/channels` | Создать. `201`. |
| `PUT` | `/api/notify/channels/{id}` | Обновить. Если опустить `secrets`, сохраняются прежние; `"secrets": {}` очищает их. В `policy` отсутствие `matcher` сохраняет фильтр маршрутизации, а `"matcher": null` очищает его. |
| `DELETE` | `/api/notify/channels/{id}` | `204`. |
| `POST` | `/api/notify/channels/{id}/test` | Отправить тестовое сообщение: `{"ok": true, "message": "…"}`. |

```json
{
  "name": "Ops Discord",
  "kind": "discord",
  "enabled": true,
  "settings": {"username": "DumbMonit"},
  "secrets": {"webhook_url": "https://discord.com/api/webhooks/…"}
}
```

Точные ключи для каждого вида берутся из `/api/notify/kinds` и описаны в
[Каналы уведомлений](../notifications.md).

### Устройства Web Push

Устройства **текущей учётной записи** (только сессия, включая viewer; API-токены
получают `401`). Канал `webpush` отправляет на них; см.
[Push-уведомления (PWA)](../using/push-notifications.md).

| Метод | Маршрут | Назначение |
|---|---|---|
| `GET` | `/api/webpush` | `{"public_key": "…", "devices": […]}`: публичный ключ VAPID (base64url, `applicationServerKey` для подписки) и устройства учётной записи — `id`, `device` (`Firefox on Android`), `push_service` (хост), `fingerprint` (первые 16 шестнадцатеричных символов SHA-256 от endpoint), `created_at`, `last_success_at`, `last_error`. Endpoint, ключи браузера и закрытый ключ не возвращаются никогда. |
| `POST` | `/api/webpush/subscriptions` | `PushSubscription.toJSON()`: `{"endpoint": "https://…", "keys": {"p256dh": "…", "auth": "…"}}`. `201` с устройством. Endpoint должен быть HTTPS на публичном адресе: частные, loopback и link-local адреса получают `400`. Тот же endpoint повторно заменяет свою строку. |
| `DELETE` | `/api/webpush/subscriptions/{id}` | `204`; `404` для устройства другой учётной записи. |
| `POST` | `/api/webpush/test` | `{}` или `{"id": 3}`. Отправляет тестовое уведомление: `{"ok", "delivered", "failed", "removed", "message"}`. Подписки, ответившие `404`/`410`, удаляются. |

## Отчёты по email {#email-reports}

Только администраторы, включая `GET` (ответ содержит список получателей).
Bearer-токенам для изменений нужна область `write`. См.
[Отчёты по email](../using/reports.md).

| Метод | Маршрут | Auth | Назначение |
|---|---|---|---|
| `GET` | `/api/reports/schedules` | admin | Все отчёты: `id`, `name`, `enabled`, `frequency`, `weekday` (0 — понедельник), `day_of_month`, `hour`, `timezone`, `recipients`, `channel_id`, `last_sent_at`, `last_error`, `next_run_at`. |
| `POST` | `/api/reports/schedules` | admin | Те же записываемые поля; по умолчанию каждое — еженедельно, понедельник, 8:00, `UTC`. `recipients` содержит не более 20 адресов и обязательно, чтобы включить отчёт; `channel_id` должен быть каналом `smtp` или `null`. `201`; `409` сверх 10 отчётов. |
| `GET` | `/api/reports/schedules/{id}` | admin | Один отчёт. |
| `PUT` | `/api/reports/schedules/{id}` | admin | Полная замена. |
| `DELETE` | `/api/reports/schedules/{id}` | admin | `204`. |
| `POST` | `/api/reports/schedules/{id}/send-test` | admin | Отправляет предпросмотр по email сохранённым получателям: `{"sent": 2, "failed": 0}`. Расписание не затрагивает. `400` без пригодного email-канала; `409` в течение 30 секунд после предыдущего предпросмотра. |

## Страницы статуса и инциденты

| Метод | Маршрут | Auth | Назначение |
|---|---|---|---|
| `GET` | `/api/status-pages` | session | Все страницы с их элементами: `page` (`id`, `slug`, `title`, `description`, `published`, `theme`, `show_uptime_days`, `created_at`, `updated_at`) и `items` (`id`, `page_id`, `target_id`, `label`, `group_name`, `position`). |
| `POST` | `/api/status-pages` | admin | `{"title", "slug", "description", "published", "theme", "show_uptime_days"}`. `title` обязателен (не более 120 символов); `slug` (`^[a-z0-9-]{2,40}$`) выводится из заголовка, если опущен; `theme` — `auto` (по умолчанию), `light` или `dark`; `show_uptime_days` от 7 до 90 (по умолчанию 90). `201`; `409`, если slug уже занят. |
| `GET` | `/api/status-pages/{id}` | session | Одна страница с её элементами. |
| `PUT` | `/api/status-pages/{id}` | admin | Те же поля, но это полная замена, а не патч: `title` обязателен, и каждое опущенное поле возвращается к значению по умолчанию (slug выводится заново из заголовка, пустое описание, `published: false`, `theme: auto`, 90 дней аптайма). |
| `DELETE` | `/api/status-pages/{id}` | admin | `204`. Публичный URL перестаёт отвечать. |
| `POST`, `PUT` | `/api/status-pages`, `/api/status-pages/{id}` — внешний вид и подписки | admin | Также принимают `accent` (`default`, `blue`, `teal`, `violet`, `rose`, `amber`), `scenes` (id сцен из списка редактора — города, новые чудеса света и древние чудеса света, например `venice`, `paris`, `giza`, `tajmahal`, без дубликатов; `[]` — ни одной; если опущено при создании: `["paris"]`), `simple` (boolean, по умолчанию `false`: публичная страница не показывает сцену, талисман и анимацию и следует светлой/тёмной теме посетителя), `scene_rotation` (`visit`, `1m`, `10m` или `1h`), `footer_text` (простой текст, 280 символов), `homepage_url` (`http(s)://` или пусто), `subscribe_channel_id` (канал `smtp` или `null`, чтобы выключить подписку по email) и `domain` (публичное имя хоста страницы, например `status.example.com`, или `null`/`""`, если его нет — см. [Собственный домен](../using/status-pages.md#custom-domain)). `domain` приводится к нижнему регистру и теряет префикс `https://` или завершающий слеш; порт, путь или IP-адрес — это `400`, адрес самого DumbMonit (его публичный URL или хост, на который пришёл запрос) — `400`, а имя, которое использует другая страница, — `409`. В отличие от остальных полей, эти восемь сохраняют своё значение, если опущены в `PUT`. Вид страницы возвращает их вместе с `logo_type` (`image/png`, `image/jpeg`, `image/webp` или `null`). |
| `GET`, `PUT`, `DELETE` | `/api/status-pages/{id}/logo` | session / admin | Логотип страницы. `PUT` принимает `{"data": "<base64 или data: URL>"}`: PNG, JPEG или WebP, определяемые по содержимому, не более 256 KiB, всё остальное — `400`. `204`. `GET` — предпросмотр редактора (включая черновики). |
| `GET` | `/api/status-pages/{id}/subscribers` | session | Подписчики по email: `id`, `page_id`, `email`, `confirmed_at` (`null`, пока ожидает), `created_at`. Их токены никогда не покидают сервер. |
| `DELETE` | `/api/status-pages/{id}/subscribers/{subscriber}` | admin | Удаляет адрес. `204`. |
| `PUT` | `/api/status-pages/{id}/items` | admin | `[{"target_id": 4, "label": "NAS", "group_name": "Storage"}, …]` — полный упорядоченный список устройств, показываемых на странице. |
| `GET` | `/api/incidents` | session | Все инциденты с их обновлениями: `incident` (`id`, `page_id`, `title`, `kind`, `status`, `severity`, `starts_at`, `ends_at`, `created_at`, `updated_at`) и `updates`. |
| `POST` | `/api/incidents` | admin | `{"title", "kind", "status", "severity", "page_id", "starts_at", "ends_at", "body"}` — `body` — первое обновление. `201`. |
| `PUT` | `/api/incidents/{id}` | admin | Те же поля; опущенное поле сохраняет значение. |
| `DELETE` | `/api/incidents/{id}` | admin | `204`. |
| `GET` | `/api/incidents/{id}/updates` | session | Хронология: `id`, `incident_id`, `status`, `body`, `created_at`. |
| `POST` | `/api/incidents/{id}/updates` | admin | `{"status": "monitoring", "body": "…"}`. Добавляет обновление и переводит инцидент в `status`. `201`. |
| `GET` | `/api/public/status/{slug}` | public | JSON-документ, из которого строится опубликованная страница (также `badge.svg` и `rss` по тому же пути). В `page` `show_uptime_days` — настроенный максимум, `history_days` — реально показываемое окно (возраст данных страницы, от 7 дней до этого максимума), а `simple` отражает простой режим. См. [Страницы статуса](../using/status-pages.md). На собственном домене страницы (`Host` равен её `domain`) отвечают только маршруты `/api/public/status/{slug}/…` этой страницы; любой другой маршрут этого справочника там даёт `404`. |
| `GET` | `/api/public/status/{slug}/uptime.svg`, `/response.svg` | public | Бейджи страницы: средний аптайм за `?days=` `1`, `7`, `30` (по умолчанию) или `90` — `400` за пределами истории страницы — и среднее время отклика. SVG, `Cache-Control: public, max-age=60`. |
| `GET` | `/api/public/status/{slug}/components/{key}/badge.svg`, `/uptime.svg`, `/response.svg` | public | Те же три бейджа для одного сервиса; `key` — это `key` сервиса в публичном документе (его метка в виде для URL). `404` для неизвестного ключа. |
| `GET` | `/api/public/status/{slug}/logo` | public | Логотип страницы с типом изображения, кэшируется на сутки (`logo_url` документа содержит версию). `404`, если логотипа нет. |
| `POST` | `/api/public/status/{slug}/subscribe` | public | `{"email": "…"}`. Отправляет ссылку подтверждения через SMTP-канал страницы. Всегда `202` с одним и тем же сообщением, каково бы ни было состояние адреса; `400` для недопустимого адреса; `404`, когда страница не предлагает подписку по email; `409`, когда страница не может принять больше подписчиков; `429` с `Retry-After` сверх пяти запросов на клиента и шестидесяти в целом за 15 минут. |
| `POST` | `/api/public/status/{slug}/confirm?token=…` | public | Подтверждает подписку (ссылка из письма подтверждения открывает `/s/{slug}/confirm`, кнопка которой отправляет POST сюда). `404` для неизвестного или истёкшего токена. |
| `POST` | `/api/public/status/{slug}/unsubscribe?token=…` | public | Удаляет подписку; тело игнорируется, поэтому это также цель one-click по RFC 8058 для `List-Unsubscribe`. Всегда `200`. |

## Токены агентов и ingest

| Метод | Маршрут | Auth | Назначение |
|---|---|---|---|
| `GET` | `/api/agent/tokens` | session only | Все токены: `id`, `name`, `prefix`, `created_at`, `last_used_at`, `revoked_at`, `max_uses` (`null` для токена флота), `uses`, `expires_at`. |
| `POST` | `/api/agent/tokens` | admin, session only | `{"name": "Home fleet", "base_url": "http://server:8080", "reusable": false, "max_uses": null, "expires_in_days": null}`. `201` с полями токена плюс `secret` (показывается один раз), `install_linux` и `install_windows`. `base_url` — URL, которым будут пользоваться агенты; по умолчанию — адрес прослушивания. Без `reusable` токен **одноразовый**: он регистрирует одну машину и не более. С ним `max_uses` ограничивает число регистраций (`null`: без ограничений). `expires_in_days` прекращает *регистрации* через указанное число дней; уже зарегистрированные машины продолжают отправлять данные. `400` для значения меньше 1. |
| `DELETE` | `/api/agent/tokens/{id}` | admin, session only | Отзыв. `204`; `404`, если токен неизвестен или уже отозван. |
| `POST` | `/api/ingest` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Принимает пакет образцов от агента (тела до 16 МБ). Не предназначен для ручного вызова. Заголовок несёт секрет привязки машины; агент, у которого его нет, опускает его — так он и запрашивает секрет. Ответ добавляет `agent_secret` (секрет, возвращается ровно один раз, когда машина привязана) и `bound`. `403`, когда машина привязана к другой установке агента или когда токен больше не может регистрировать — сообщение говорит, что именно и что делать. |
| `GET` | `/api/agent/relay?key=…&wait=N` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Опросы, делегированные релейному агенту (`relay: true`). Удерживается до `wait` секунд (максимум 25), когда ничего не ожидает. Каждый элемент — команда вида `probe`, чьи `args` содержат цель, её расшифрованные учётные данные, `timeout_secs` и `discover`. Никогда не записывается на диск. |
| `POST` | `/api/agent/relay/{id}?key=…` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Результат делегированного опроса: `{"duration_ms", "error", "samples", "profile_id"}` (тела до 16 МБ). `204`; `404`, когда опрос истёк или принадлежит другому агенту. |
| `GET` | `/api/agent/commands?key=…&wait=N` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Команды, поставленные в очередь для агента (перезапуск или обновление контейнера), удерживается до `wait` секунд, когда ничего не ожидает. Не предназначен для ручного вызова. Один `key` ничего не доказывает — он публичен, — поэтому привязанная машина должна также предъявить свой секрет привязки; иначе `403`, что и мешает одной машине забирать команды другой. |
| `POST` | `/api/agent/commands/{id}?key=…` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Ход выполнения и результат команды, сообщаемые агентом. `result` усекается до последних 4 КБ, и в таком случае в начало добавляется `[truncated by the server, beginning dropped]`. |
| `GET` | `/api/relays` | session | Все устройства-агенты как возможные релеи: `id`, `name`, `site`, `relay` (объявлено `relay: true`), `last_seen_at`, `relayed` (устройства, доступные через него). Сначала релеи, затем по имени. |

```json
{
  "id": 1, "name": "Home fleet", "prefix": "dmon_ab12",
  "created_at": "2026-09-15 13:00:00", "last_used_at": null, "revoked_at": null,
  "secret": "dmon_…",
  "install_linux": "curl -sSL http://server:8080/install.sh | sh -s -- --token=dmon_… --url=http://server:8080",
  "install_windows": "& ([scriptblock]::Create((irm http://server:8080/install.ps1))) -Token dmon_… -Url http://server:8080"
}
```

## Контейнеры и команды (устройства-агенты)

Действия над контейнерами машины, на которой работает агент. Каждый маршрут
отвечает `404`, если устройство не является целью `agent`.

| Метод | Маршрут | Назначение |
|---|---|---|
| `GET` | `/api/targets/{id}/agent` | Машина в том виде, как её последний раз описал агент: `hostname`, `os`, `os_version`, `arch`, `agent_version`, `commands_supported`, `relay`, `site`, `relayed` (устройства, доступные через этого агента), `last_seen_at`, `binding`, `bound`, `bound_at`, `rebind_until`. `404`, пока агент ничего не сообщил. `commands_supported` равно `true` только когда агент заявил, что получает команды (`commands: true`, значение по умолчанию у текущих агентов); более старый агент или агент с `commands: false` даёт `false`. `binding` — `bound`, `pending` (бинарный файл может быть привязан и будет привязан со следующим пакетом) или `unsupported` (агент старше привязки: переустановите его). |
| `POST` | `/api/targets/{id}/agent/rebind` | Admin. Открывает часовое окно, в течение которого эта машина может привязаться заново с действительным токеном регистрации — путь назад после переустановки, унёсшей секрет агента. `{"rebind_until", "minutes"}`. Окно закрывается, как только оно использовано, а старый секрет перестаёт работать. `404`, пока агент ничего не сообщил. |
| `GET` | `/api/targets/{id}/containers` | Все контейнеры из последнего пакета: `name`, `image`, `up`, `health`, `restart_count`, `uptime_seconds`, `image_age_seconds`, `update_available`, `policy`, `last_command`. |
| `PUT` | `/api/targets/{id}/containers/{name}/policy` | `{"auto_restart": bool, "auto_update": bool, "prune_old_image": bool, "only_in_maintenance": bool}`. Каждое поле необязательно: опущенное поле сохраняет своё значение. Возвращает полную политику. |
| `POST` | `/api/targets/{id}/containers/{name}/restart` | Поставить перезапуск в очередь. `201` с командой. `404`, когда `name` нет в инвентаре агента; `409`, когда такая же команда уже в очереди или выполняется либо когда агент не может выполнять команды (см. `commands_supported`). |
| `POST` | `/api/targets/{id}/containers/{name}/update` | Поставить обновление в очередь; необязательно `{"prune": bool}` (по умолчанию — из политики). Те же коды статуса, что и у перезапуска. |
| `GET` | `/api/targets/{id}/commands` | Последние 20 команд, сначала новые: `id`, `kind` (`container.restart`, `container.update`), `args`, `status`, `requested_by` (пользователь или `policy`), `created_at`, `started_at`, `finished_at`, `result`. |
| `DELETE` | `/api/targets/{id}/commands/{command_id}` | Отменить команду в очереди. `204`; `409`, как только агент её забрал или когда она уже закрыта; `404`, когда она не принадлежит этому устройству. |

`status` проходит `queued → running → done | failed`. Существуют ещё два
конечных состояния: `cancelled` (человек отозвал команду) и `expired` (сервер
сдался через десять минут, потому что ни один агент не пришёл за ней — агент
остановлен, слишком стар для канала команд или установлен с `commands: false`).
Сервер сам каждую минуту помечает устаревшие команды как просроченные;
команда в очереди никогда не блокирует новую дольше этого срока.

## Гостевые системы Proxmox VE

Строки панели **Guests** устройства Proxmox VE, собранные из последнего опроса в
хранилище метрик (четыре мгновенных запроса, у гипервизора ничего не
запрашивается). `404`, когда устройства не существует, `400`, когда оно не
является целью `proxmox`.

| Метод | Маршрут | Назначение |
|---|---|---|
| `GET` | `/api/targets/{id}/proxmox/guests` | Один объект на ВМ или контейнер, отсортированы по узлу, затем по VMID: `vmid`, `name`, `node`, `kind` (`qemu`, `lxc`), `status` (`running`, `stopped`, `paused`, `suspended`, `template`, `unknown`), `cpu_percent` (от выделенных ядер), `cpu_count`, `memory_used_bytes`, `memory_total_bytes`, `memory_percent`, `balloon_bytes`, `disk_used_bytes`, `disk_total_bytes`, `disk_percent`, `agent` (`true` — агент гостя ответил, `false` — включён, но молчит, `null` — нет), `network_in_bps`, `network_out_bps`, `disk_read_bps`, `disk_write_bps`, `uptime_seconds`, `last_backup_age_seconds`, `ha_state`, `pool` (его пул ресурсов), `lock` (только пока удерживается блокировка: `backup`, `migrate`…), `os` и `ip` (видимые изнутри гостя, обновляются раз в час). Каждое неизвестное значение — `null`: у ВМ без агента гостя есть `disk_total_bytes`, но `disk_used_bytes: null`; остановленный гость сохраняет свои размеры и теряет измерения. |

```json
[
  {
    "vmid": 202, "name": "nextcloud", "node": "pve2", "kind": "lxc", "status": "running",
    "cpu_percent": 1.0, "cpu_count": 4,
    "memory_used_bytes": 1879048192, "memory_total_bytes": 4294967296, "memory_percent": 43.75, "balloon_bytes": null,
    "disk_used_bytes": 61203283968, "disk_total_bytes": 107374182400, "disk_percent": 57.0, "agent": null,
    "network_in_bps": 1024.0, "network_out_bps": 512.0, "disk_read_bps": 0.0, "disk_write_bps": 2048.0,
    "uptime_seconds": 3196800, "last_backup_age_seconds": 25200, "ha_state": "started",
    "pool": "production", "lock": null,
    "os": "Debian GNU/Linux 12 (bookworm)", "ip": "192.168.10.60"
  }
]
```

## Узлы Proxmox VE и Ceph

Разделы **Nodes** и **Ceph** той же страницы устройства, прочитанные из того же
последнего опроса. Те же коды статуса, что и выше.

| Метод | Маршрут | Назначение |
|---|---|---|
| `GET` | `/api/targets/{id}/proxmox/nodes` | `{"nodes": […], "fencing_state", "fencing_armed"}` — состояние HA-watchdog (`armed`, `standby`…) и то, действительно ли он изолировал бы потерянный узел, затем по одному объекту на узел, отсортированы по имени: `name`, `up`, `cpu_percent`, `cpu_iowait_percent`, `memory_percent`, `ksm_shared_bytes`, `rootfs_percent`, `uptime_seconds`, `version` (релиз `pve-manager`, установленный на этом узле), `services_down` (имена юнитов остановленных или упавших демонов), `interfaces_offline` (интерфейсы, которые должны подниматься при загрузке, но не подняты), `thin_pools` (`name`, `vg`, `used_percent`, `metadata_used_percent`, `size_bytes`) и `volume_groups` (`name`, `used_percent`, `size_bytes`). Узел, переставший отвечать, сохраняет свою запись с `up: false`. |
| `GET` | `/api/targets/{id}/proxmox/ceph` | `available` (`false`, когда в кластере нет Ceph — тогда интерфейс вообще не рисует раздел), `health` (0 OK, 1 WARN, 2 ERR, 3 неизвестно), `health_status`, `bytes_used`, `bytes_total`, `used_percent`, `osds_total`, `osds_up`, `osds_in`, `osds` (`name`, `host`, `device_class`, `up`, `in`, `used_percent`, `used_bytes`, `total_bytes`, `apply_latency_ms`, `commit_latency_ms`), `pools` (`name`, `used_percent`, `used_bytes`, `size`, `min_size`, `pg_num`, `pg_num_optimal`, `autoscale`), `filesystems` (имена CephFS), `flags` (установленные в данный момент флаги OSD, например `noout`) и `muted_checks` (заглушённые проверки здоровья, которые `HEALTH_OK` больше не упоминает). |

## Proxmox Backup Server

Панели устройства `pbs`, прочитанные из последнего опроса (у сервера ничего не
запрашивается, кроме журнала задач). `404`, когда устройства не существует,
`400`, когда оно не является целью `pbs`.

| Метод | Маршрут | Назначение |
|---|---|---|
| `GET` | `/api/targets/{id}/pbs/calendar?days=30&offset=120` | `{"probed_at", "days", "offset_minutes", "groups": […]}`: по одной записи на группу резервного копирования (`datastore`, `namespace`, `backup_type`, `backup_id`, `name`, `count`, `last_time`, `last_size`, `last_verified`, `last_success`, `last_failure`, `retention`) с массивом `days` из `{date, state, runs, snapshot}`. `offset` — смещение UTC наблюдателя в минутах, чтобы дни разделялись по местной полуночи. |
| `GET` | `/api/targets/{id}/pbs/failures?days=30` | Неудавшиеся задачи: `upid`, `worker_type`, `kind`, `worker_id`, `datastore`, `start`, `end`, `error`. |
| `GET` | `/api/targets/{id}/pbs/jobs` | `{"probed_at", "jobs": […]}`: задания sync, verify, prune и garbage-collection с их расписанием и последним результатом. |
| `GET` | `/api/targets/{id}/pbs/health` | Хранилища данных (использование, ожидаемая дата заполнения), диски (SMART, износ) и пулы ZFS. |
| `GET` | `/api/targets/{id}/pbs/tasks/{upid}/log?lines=200` | Журнал одной задачи, получаемый с сервера: `{"upid", "lines": […]}`; `lines` ограничивает число возвращаемых строк. |
| `GET` | `/api/targets/{id}/pbs/disks/smart?disk=/dev/sda` | Атрибуты SMART одного диска; `disk` обязателен (без него `400`). |
| `GET` | `/api/targets/{id}/pdm/remotes` | Proxmox Datacenter Manager: итоги по всему парку (`estate`) и по строке на каждый федеративный экземпляр (`id`, `kind`, `reachable`, `error`, `version`, `version_behind`, количества узлов и гостей, память и хранилище, `subscription`, `last_collection`, `tasks_failed`), недоступные первыми. |
| `GET` | `/api/targets/{id}/pdm/failures?days=14` | Задачи, завершившиеся неудачей по всему парку: `upid`, `remote`, `worker_type`, `kind`, `worker_id`, `node`, `start`, `end`, `error`. |
| `GET` | `/api/targets/{id}/pdm/health` | Хост консоли: CPU, память, корневая файловая система, аптайм, сертификаты, ожидающие обновления и подписка. |
| `GET` | `/api/targets/{id}/pmg/queues` | Proxmox Mail Gateway: четыре очереди Postfix в порядке чтения (`queue`, `messages`, `domains`, `oldest_age_seconds`, `top_domains`, `stuck`), плюс `total_messages` и общий для шлюза `stuck`. `oldest_age_seconds` — нижняя граница: qshape сообщает возрастные интервалы. |
| `GET` | `/api/targets/{id}/pmg/traffic` | Итоги за сегодня (`mail`), недавняя кривая трафика (`recent`), гистограмма спам-оценок, вирусы, пойманные сегодня, и счётчики карантина (`quarantine`). Только счётчики: без содержимого сообщений. |
| `GET` | `/api/targets/{id}/pmg/health` | Одна запись на узел (система, службы, `signatures` с `family`, `age_seconds` и `stale`, `expiring_certificates`, обновления, подписка), члены `cluster` с их состоянием синхронизации и `stopped_services` по всем узлам. |

## Synology

| Метод | Маршрут | Назначение |
|---|---|---|
| `GET` | `/api/targets/{id}/synology` | Обзор устройства `synology` по последнему опросу: `system` (модель, версия DSM, аптайм, температура, CPU, память), `volumes` (`name`, `fs_type`, `raid_type`, `status`, `severity`, `total_bytes`, `used_bytes`), `disks` (`model`, `serial`, `kind`, `ssd`, `status`, `smart_status`, `temperature_celsius`, `size_bytes`, `remaining_life_percent`, …) и `sampled_at`. `404`, когда устройства не существует, `400`, когда это не Synology. |
| `GET` | `/api/targets/{id}/synology/abb` | Active Backup for Business: `tasks` и `devices` — по одному отчёту на защищаемое устройство с выученной периодичностью в `assessment` (`state`, `last_success_s`, `typical_interval_s`, счётчики за 30 дней). |

## Heartbeats (push-мониторы)

| Метод | Маршрут | Auth | Назначение |
|---|---|---|---|
| `GET` или `POST` | `/api/push/{token}` | none (токен) | Вызов, который задание делает при каждом запуске. Необязательно `?status=up\|down&msg=…` (совместимо с Uptime Kuma). `204` пустой при успехе, `404` для неизвестного токена, `400` для другого `status`, `429` с `Retry-After` сверх шестидесяти вызовов в минуту на токен. См. [Heartbeat](../devices/push.md). |
| `GET` | `/api/targets/{id}/push` | session | Монитор устройства `push`: `token`, `path` (`/api/push/<token>`), `last_seen_at`, `last_seen_age_secs`, `last_status`, `last_message`, `received_total`, `expected_interval_secs`, `grace_secs`, `settings_error`, `verdict` (`waiting`, `on_time`, `missed`, `reported_down`). Создаётся при первом чтении. `400`, когда устройство не является heartbeat. |
| `POST` | `/api/targets/{id}/push/regenerate` | admin | Новый токен; прежний URL с этого момента отвечает `404`. Тело то же, что при чтении. |

## Ассистенты (MCP)

Встроенный сервер [Model Context Protocol](https://modelcontextprotocol.io),
тот, с которым общается ассистент. Он аутентифицируется описанными выше
API-токенами — те же области, срок действия, сети и ограничение частоты — и
никогда не использует сессию.

| Метод | Маршрут | Auth | Назначение |
|---|---|---|---|
| `POST` | `/api/mcp` | API-токен | JSON-RPC 2.0 поверх транспорта Streamable HTTP, один запрос на `POST`, ответ в JSON. Протокол `2026-07-28` (без состояния: каждый запрос несёт свою версию и возможности клиента в `_meta`, плюс заголовки `MCP-Protocol-Version`, `Mcp-Method` и `Mcp-Name`; `server/discover`), а также `2025-11-25`, `2025-06-18` и `2025-03-26` через рукопожатие `initialize`. Без сессии: `Mcp-Session-Id` не выдаётся, присланный игнорируется. Уведомление получает `202` и пустое тело. |
| `GET`, `DELETE` | `/api/mcp` | public | `405` с `Allow: POST` — потока, отправляемого сервером, и сессии для завершения нет; тело лишь сообщает, чем является этот эндпоинт. |

`tools` — единственная возможность: ни ресурсов, ни промптов, ни OAuth. Из 27
инструментов токен `read` может вызывать 15, которые только читают
(`get_status`, `list_devices`, `get_device`, `list_alerts`, `alert_history`,
`query_metrics`, `list_silences`, `list_rules`, `list_device_types`,
`list_agents`, `list_containers`, `list_heartbeats`, `list_status_pages`,
`list_channels`, `list_packs`); 12, которые что-то изменяют
(`silence_device`, `remove_silence`, `acknowledge_alert`, `probe_device`,
`set_device_enabled`, `set_rule_enabled`, `add_device`, `discover_network`,
`schedule_maintenance`, `restart_container`, `test_channel`,
`post_incident`), требуют токен `write`. Инструменты вызывают тот же код, что и
веб-интерфейс. Версии, заголовки, коды ошибок и замечания по безопасности
приведены в [Подключение ассистента](../using/assistant.md#protocol-details).

## Файлы агента (вне `/api`)

| Метод | Маршрут | Назначение |
|---|---|---|
| `GET` | `/install.sh` | Установщик для Linux, macOS и FreeBSD. Публичный. |
| `GET` | `/install.ps1` | Установщик для Windows. Публичный. |
| `GET` | `/download/{name}` | `dumbmonit-agent-linux-x86_64`, `dumbmonit-agent-linux-aarch64`, `dumbmonit-agent-freebsd-x86_64`, `dumbmonit-agent-windows-x86_64.exe`, раздаются из `DUMBMONIT_AGENT_DIR`; `{name}.sha256` даёт его контрольную сумму в формате `sha256sum`. `404`, если файл отсутствует. `dumbmonit-agent-macos-aarch64` и `dumbmonit-agent-macos-x86_64` (и их `.sha256`) отвечают `307` на тот же файл в последнем релизе GitHub. Публичный. |

Любой другой путь обслуживается веб-интерфейсом, который сам предложит вам войти.
