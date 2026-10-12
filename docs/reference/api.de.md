# HTTP-API

Alles, was die Web-UI tut, läuft über `/api/*`, und dieselbe API steht
Ihren eigenen Skripten, Dashboards und Assistants offen: Es gibt
keine private Hälfte. Es ist einfaches JSON über HTTP,
authentifiziert durch ein Session-Cookie (die Web-UI) oder ein
API-Token (alles andere).

!!! warning "Alpha"
    DumbMonit ist eine Alpha, und seine API auch: Routen und Felder
    können sich noch ändern. Die API-Version (unten) ändert sich,
    wann immer eine Route oder ein Feld entfernt oder umbenannt wird,
    sodass ein Client es erkennen kann.

## OpenAPI-Beschreibung

Jede Instanz beschreibt ihre eigene API in einem
[OpenAPI-3.1](https://spec.openapis.org/oas/v3.1.0)-Dokument:

```
GET /api/openapi.json
```

Es ist öffentlich — es trägt kein Secret, keine Adresse und nichts
über Ihre Geräte — und ist die erschöpfende Referenz: jede Route, die
ein Client aufrufen kann, ihre Parameter, Request- und
Antwortformen, Fehlerantworten und der benötigte Scope (die
Erweiterung `x-token-scope` auf jeder Operation: `public`, `read`,
`write`, oder `none` für Routen, die ein Token nie aufrufen kann).
Die weiter unten auf dieser Seite stehenden Tabellen sind eine
lesbare Zusammenfassung desselben.

Absichtlich ausgelassen: das Agent-Protokoll (`/api/ingest`,
`/api/agent/commands*`, `/api/agent/relay*`), das nur der
[Agent](../devices/agent.md) spricht; die Browser-Weiterleitungen
des SSO-Logins (`/api/auth/oidc/start`, `/api/auth/oidc/callback`);
und die Agent-Downloads (`/install.sh`, `/install.ps1`,
`/download/*`).

Laden Sie es in jedes OpenAPI-Tool:

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

Postman und Insomnia importieren es von derselben URL.

## Authentifizierung {#authentication}

Zwei Wege hinein, auf jeder geschützten Route akzeptiert:

- **Session-Cookie** — was die Web-UI nutzt. Loggen Sie sich mit
  einem Passwort ein (oder SSO) und senden Sie das HttpOnly-Cookie
  `dumbmonit_session` (SameSite Lax; `Secure`, wenn
  `DUMBMONIT_COOKIE_SECURE=1`; 30 Tage). Jeder zustandsändernde
  Request (`POST`, `PUT`, `DELETE`) muss auch
  `X-Requested-With: DumbMonit` tragen — der Anti-CSRF-Beweis, den
  die UI automatisch hinzufügt; ohne ihn wird der Request mit `403`
  abgelehnt.
- **API-Token** — `Authorization: Bearer dmt_…`, für Skripte,
  Dashboards und Assistants. Erstellt unter Settings → **API &
  assistants** (dieselben Tokens, die der
  [MCP-Server](../using/assistant.md) nutzt) und einmal gezeigt.
  Kein Cookie ist involviert, daher wird nie ein
  `X-Requested-With`-Header benötigt.

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

Trägt ein Request ein Bearer-Token, wird das Cookie ignoriert: Ein
widerrufenes Token wird selbst von einem eingeloggten Browser
abgelehnt, statt leise auf die Sitzung zurückzufallen.

Agent-Enrollment-Tokens (`dmon_…`) sind etwas anderes: Sie
funktionieren nur auf den Agent-Routen (`/api/ingest`,
`/api/agent/commands/*`, `/api/agent/relay*`). Sie sagen, dass eine
Maschine sprechen darf, nicht *welche* Maschine: Auf diesen Routen
muss eine gebundene Maschine auch ihr eigenes Binding-Secret in
`X-DumbMonit-Agent-Secret` (`dmab_…`) präsentieren, das der Server
einmal bei der Registrierung ausgibt. Siehe
[Binding](../devices/agent.md#binding-one-machine-one-agent).

Login ist ratenbegrenzt: Nach fünf fehlgeschlagenen Versuchen wird
jeder weitere Versuch mit `429` und einer `Retry-After`-Verzögerung
abgelehnt, die sich von 30 s bis 5 Minuten verdoppelt. Erraten Sie
nie Passwörter in einer Schleife.

## Scopes

Ein Token hat einen von zwei Scopes. Es gibt keinen dritten,
mächtigeren: Was ein Administrator mit seinem eigenen Konto tut,
bleibt für jedes Token unerreichbar.

| | `read`-Token | `write`-Token |
|---|---|---|
| Jeder `GET`, den ein *Viewer*-Konto machen kann (Geräte, Alarme, Metriken, Regeln, Kanäle ohne ihre Secrets, Statusseiten…) | ja | ja |
| `POST`, `PUT`, `DELETE`, die ein *Administrator* machen kann (Geräte, Regeln, Silences, Kanäle, Statusseiten, Packs, Abfragen, Discovery…) | `403` | ja |
| `/metrics`, `/federate`, `/prometheus/*` ([Abfrage](metrics.md#scraping-dumbmonit)) | ja | ja |
| MCP-Lesewerkzeuge / Schreibwerkzeuge ([Assistants](../using/assistant.md#what-the-assistant-can-call)) | nur lesen | beide |
| Konten, Sitzungen, Zwei-Faktor, SSO, Audit-Log (`/api/auth/*`, `/api/users/*`) | `403` | `403` |
| API-Tokens und Agent-Enrollment-Tokens (`/api/tokens/*`, `/api/agent/tokens/*`) | `403` | `403` |
| Backup und Wiederherstellung (`/api/backup/*`) | `403` | `403` |

Ein `read`-Token auf einer Schreib-Route erhält `403` mit einer
Meldung, die das Token und den fehlenden Scope benennt. Da kein
Token Tokens listen, erstellen oder widerrufen kann, kann ein Token
nie ein anderes ausstellen — geschweige denn ein mächtigeres.

## Token-Sicherheit

- **Einmal gezeigt, gehasht gespeichert.** Ein Token ist `dmt_`
  gefolgt von 128 zufälligen Bits. Der Server behält nur sein
  SHA-256-Digest (und die ersten Zeichen, `dmt_1a2b3c4d`, um es in
  einer Liste zu erkennen); das Token im Klartext existiert nur in
  der Antwort auf seine Erstellung. Präsentierte Tokens werden in
  konstanter Zeit verglichen.
- **Ablauf.** Wählen Sie 30 Tage, 90 Tage, 1 Jahr oder nie beim
  Erstellen. Ein abgelaufenes Token erhält `401` mit `The API token
  "<name>" expired on …` und
  `WWW-Authenticate: Bearer error="invalid_token"`; es bleibt in der
  Liste, als abgelaufen markiert, bis Sie es widerrufen.
- **Erlaubte Netzwerke.** Beschränken Sie ein Token optional auf
  eine Liste von Adressen oder CIDR-Bereichen
  (`192.168.1.0/24, 10.8.0.5`, höchstens 32 Einträge). Von überall
  sonst erhält es `403` mit `The API token "<name>" cannot be used
  from <address>.` Die geprüfte Adresse ist die des TCP-Clients,
  oder — nur wenn die Verbindung von einem in
  `DUMBMONIT_TRUSTED_PROXIES` gelisteten Proxy kommt — die, die
  dieser Proxy in `X-Forwarded-For` meldet. Hinter einem nicht
  erklärten Reverse Proxy scheint jeder Request vom Proxy zu kommen:
  Erklären Sie ihn, oder die Beschränkung wird jeden ablehnen.
- **Letzte Nutzung.** Die Token-Liste zeigt, wann jedes Token zuletzt
  genutzt wurde und von welcher Adresse (höchstens einmal pro Minute
  aktualisiert, oder sobald sich die Adresse ändert): Ein von
  niemandem mehr genutztes Token ist eines zum Widerrufen.
- **An seinen Ersteller gebunden.** Ein Token erinnert sich an das
  Konto, das es erstellt hat. Ist dieses Konto deaktiviert, werden
  seine Tokens abgelehnt (`401`); wird es zu Operator oder Viewer
  degradiert, wirken seine `write`-Tokens als `read`-Tokens; wird es
  gelöscht, werden seine Tokens widerrufen.
- **Widerruf** ist sofort: Der nächste Aufruf erhält `401`.
- **Protokolliert.** Jeder mit einem Token gemachte Schreibvorgang
  wird ins Server-Log geschrieben, mit dem Namen und der ID des
  Tokens, der Methode, dem Pfad (ohne Query-String) und dem Status —
  nie dem Token, nie dem Body. Das Erstellen und Widerrufen eines
  Tokens wird im Audit-Log aufgezeichnet (Settings → Security).

## Ratenlimits

Jedes Token darf 120 Aufrufe pro Minute machen, REST und MCP
zusammen. Darüber hinaus werden Aufrufe mit `429` und einem
`Retry-After`-Header (Sekunden) abgelehnt, bis die Minute vorbei ist.
Jede Antwort auf einen Token-authentifizierten Request sagt, wo Sie
stehen:

| Header | Bedeutung |
|---|---|
| `RateLimit-Limit` | Erlaubte Aufrufe pro Fenster (120). |
| `RateLimit-Remaining` | Verbleibende Aufrufe im aktuellen Fenster. |
| `RateLimit-Reset` | Sekunden, bis das Fenster neu beginnt. |

Ein Grafana-Dashboard mit vielen Panels und kurzem Refresh kann das
Limit erreichen: Geben Sie ihm sein eigenes Token, oder verlängern
Sie den Refresh.

## Cross-Origin-Requests (CORS) {#cross-origin-requests-cors}

Eine von einer anderen Origin bediente Seite — Ihr eigenes
Dashboard, Swagger UI, ein browserbasierter MCP-Client — kann die
API **mit einem Bearer-Token aufrufen, nie mit einer Sitzung**:

- eine Antwort trägt `Access-Control-Allow-Origin` nur, wenn der
  Request `Authorization: Bearer dmt_…` trug (oder für das
  öffentliche `GET /api/openapi.json`).
  `Access-Control-Allow-Credentials` wird nie gesendet, daher hängt
  ein Browser nie das Session-Cookie an einen Cross-Origin-Aufruf;
- ein Preflight (`OPTIONS`) auf `/api/*` gelingt nur, wenn die
  angekündigten Header `authorization` einschließen: `204`,
  Methoden `GET, POST, PUT, DELETE`, Header `authorization,
  content-type, mcp-protocol-version, mcp-method, mcp-name`, 10
  Minuten gecacht. `X-Requested-With` ist nicht darunter, daher kann
  eine Cookie-Sitzung nie von einer anderen Origin aus schreiben;
- `Retry-After`, `WWW-Authenticate`, die `RateLimit-*`-Header und
  `X-DumbMonit-Api-Version` sind vom aufrufenden Skript lesbar.

`DUMBMONIT_API_CORS_ORIGINS` legt fest, welche Origins erlaubt sind:

| Wert | Wirkung |
|---|---|
| nicht gesetzt (Standard) | Jede Origin (`Access-Control-Allow-Origin: *`). Sicher, da nur ein Bearer-Token irgendetwas öffnet. |
| `https://grafana.example.com,https://dash.lan` | Nur diese Origins, zurückgegeben. |
| `off` | Überhaupt keine CORS-Header: nur Same-Origin-Seiten und Nicht-Browser-Clients. |

Dieselbe Liste entscheidet, welche `Origin` der
[MCP-Endpunkt](../using/assistant.md#security-notes) akzeptiert.

```js
// From a page on another origin. Never ship a write token in a public page.
const response = await fetch('https://monit.example.com/api/alerts', {
  headers: { Authorization: 'Bearer dmt_…' }
});
if (!response.ok) throw new Error((await response.json()).error);
const alerts = await response.json();
console.log(response.headers.get('RateLimit-Remaining'), 'calls left this minute');
```

## Versionierung

Jede Antwort unter `/api` trägt die API-Version:

```
X-DumbMonit-Api-Version: 0.1
```

Derselbe Wert ist `info.version` im OpenAPI-Dokument, neben
`x-server-version`, der Version des antwortenden Servers. Solange
die API `0.x` ist, ist sie nicht fixiert; die Version ändert sich,
wann immer eine Route oder ein Feld entfernt oder umbenannt wird.
Das Hinzufügen einer Route oder eines Felds ändert sie nicht:
Ignorieren Sie Felder, die Sie nicht kennen.

## Fehler

Jeder Fehler ist JSON: `{"error": "message"}`, für einen Menschen
geschrieben.

| Status | Wann |
|---|---|
| `400` | Der Request ist fehlerhaft oder ein Wert ist ungültig; die Meldung benennt ihn. |
| `401` | Keine gültige Sitzung oder Token: fehlend, unbekannt, widerrufen oder abgelaufenes Token (mit `WWW-Authenticate: Bearer`), oder ein Token, dessen Ersteller deaktiviert ist. |
| `403` | Authentifiziert, aber nicht erlaubt: ein Viewer, ein Operator außerhalb der Alarmbearbeitung, oder ein `read`-Token auf einer Schreib-Route, ein Token auf einer Konten-, Token- oder Backup-Route, ein Token, das von einem Netzwerk genutzt wird, von dem aus es nicht erlaubt ist, ein Cookie-Schreibvorgang ohne `X-Requested-With`, oder jeder Schreibvorgang auf der öffentlichen Demo (mit `"demo": true`). |
| `404` | Die ID existiert nicht. Ein Pfad unter `/api`, der zu keiner Route passt, antwortet mit `404 {"error": "Unknown API route: …"}` statt dem HTML der Web-UI. |
| `405` | Die Methode existiert auf diesem Pfad nicht (`GET /api/mcp`, zum Beispiel). |
| `409` | Ein Konflikt mit dem aktuellen Zustand (ein Duplikat, ein schon laufender Befehl…). |
| `429` | Ratenbegrenzt, mit `Retry-After`. |
| `500` | Ein Serverfehler; die Details stehen im Server-Log, nie in der Antwort. |

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

Der [MCP-Endpunkt](../using/assistant.md#protocol-details)
antwortet stattdessen in JSON-RPC, mit seinen eigenen Fehlercodes.

## Konten, Login und Tokens

In den Tabellen unten bedeutet *Sitzung* ein Session-Cookie **oder**
ein API-Token; *Admin* bedeutet eine Administrator-Sitzung oder ein
`write`-Token; *Operator* bedeutet eine Operator- oder
Administrator-Sitzung, oder ein `write`-Token; *nur Sitzung*
bedeutet ein Session-Cookie, nie ein Token.

**Rollen.** Ein `viewer`-Konto macht jeden `GET`. Ein `operator`
macht dieselben Lesevorgänge plus die auf dieser Seite als
*Operator* markierten Alarm-Schreibvorgänge (und `x-roles: [admin,
operator]` im OpenAPI-Dokument): bestätigen, stummschalten
(Wartungsfenster), eine Regel für ein Gerät ignorieren, behobene
Historie löschen. Jeder andere Schreibvorgang ist nur für Admins und
antwortet einem Operator mit `403`.

| Methode | Route | Auth | Zweck |
|---|---|---|---|
| `GET` | `/api/auth/status` | öffentlich | `{"configured": bool, "authenticated": bool, "user": …, "oidc": {"enabled", "provider_name", "login_url"}}`. `configured: false` bedeutet eine frische Instanz: Bis der erste Admin existiert, antwortet jede als *Sitzung* markierte Route mit `401` — nur `status`, `setup`, `login` und `health` sind erreichbar. |
| `POST` | `/api/auth/setup` | öffentlich | `{"setup_code": "…", "username": "admin", "password": "…"}`. Erstellt den ersten Administrator auf einer frischen Instanz (mindestens 12 Zeichen); `username` kann weggelassen werden und ist dann `admin`. `setup_code` ist der beim Start in den Server-Logs ausgegebene Einmalcode (oder `DUMBMONIT_SETUP_CODE`); Groß-/Kleinschreibung, Bindestriche und Leerzeichen werden ignoriert. `204`; öffnet keine Sitzung. `401` bei falschem Code, `429` nach zu vielen, `409`, sobald ein Konto existiert. |
| `POST` | `/api/auth/login` | öffentlich | `{"username": "…", "password": "…"}`. `204` mit `Set-Cookie`. `401` bei falschem Passwort, `429`, wenn ratenbegrenzt. Hat das Konto Zwei-Faktor aktiviert, stattdessen `200` mit `{"totp_required": true, "pending": "…"}` — ein Ticket, das fünf Minuten lebt und nach fünf falschen Codes stirbt: Abschließen mit `/api/auth/login/totp`. |
| `POST` | `/api/auth/login/totp` | öffentlich | `{"pending": "…", "code": "123456"}` — das Ticket aus dem ersten Schritt und ein Code (oder ein Recovery-Code). `204` mit dem Session-Cookie. |
| `GET` | `/api/auth/oidc/start` | öffentlich | Leitet den Browser zum Identity-Provider weiter. |
| `GET` | `/api/auth/oidc/callback` | öffentlich | Rückkehr vom Provider (`code`, `state`): öffnet die Sitzung und leitet zur UI weiter. |
| `GET` | `/api/auth/me` | nur Sitzung | Das aktuelle Konto: `id`, `username`, `display_name`, `role` (`admin`, `operator`, `viewer`), `auth` (`password`, `oidc`), `disabled`, `totp_enabled`, `created_at`, `last_login_at`. |
| `POST` | `/api/auth/logout` | nur Sitzung | Beendet die Sitzung und löscht das Cookie. |
| `POST` | `/api/auth/password` | nur Sitzung | `{"current_password": "…", "new_password": "…"}`. Loggt jede andere Sitzung aus. |
| `GET` | `/api/auth/totp` | nur Sitzung | Zwei-Faktor-Zustand des aktuellen Kontos: `enabled`, `pending`, `recovery_codes_left`. |
| `POST` | `/api/auth/totp/enroll` | nur Sitzung | `{"password": "…"}`. Schlägt ein Secret vor: `secret` (base32), `otpauth_uri`, `issuer`, `account`. Nichts wird erzwungen, bis es verifiziert ist. |
| `POST` | `/api/auth/totp/verify` | nur Sitzung | `{"code": "123456"}`. Bestätigt die Registrierung und gibt `{"recovery_codes": […]}` zurück — einmal gezeigt. |
| `DELETE` | `/api/auth/totp` | nur Sitzung | `{"password": "…"}`. Entfernt den zweiten Faktor. `204`. |
| `GET` | `/api/auth/audit?limit=200` | admin, nur Sitzung | Die neuesten Sicherheitsereignisse: `id`, `at`, `actor`, `action` (`login`, `login.failed`, `password.changed`, `token.created`, `user.updated`, …), `subject`, `ip`. |
| `GET` | `/api/auth/oidc/config` | admin, nur Sitzung | Die SSO-Einstellungen und ihre Quelle (Umgebung oder Datenbank); das Client-Secret wird nie zurückgegeben. |
| `PUT` | `/api/auth/oidc/config` | admin, nur Sitzung | `{"issuer", "client_id", "client_secret", "provider_name", "scopes", "auto_create", "admin_groups", "groups_claim", "public_url"}`. Ein fehlendes oder leeres `client_secret` behält das gespeicherte; ein fehlendes `auto_create` bedeutet aus. |
| `DELETE` | `/api/auth/oidc/config` | admin, nur Sitzung | Vergisst die gespeicherten SSO-Einstellungen (Umgebungsvariablen, falls vorhanden, greifen wieder). `204`. |
| `POST` | `/api/auth/oidc/test` | admin, nur Sitzung | Ruft das Discovery-Dokument des Providers ab und meldet, was gefunden wurde. |
| `GET` | `/api/users` | admin, nur Sitzung | Jedes Konto, in der Form von `/api/auth/me`. |
| `POST` | `/api/users` | admin, nur Sitzung | `{"username", "display_name", "role", "password"}`. `password` kann nur weggelassen werden, wenn SSO aktiviert ist. `201`. |
| `PUT` | `/api/users/{id}` | admin, nur Sitzung | Beliebige von `display_name`, `role`, `disabled`, `password`; ein weggelassenes Feld behält seinen Wert. Kann den letzten Administrator nicht degradieren oder deaktivieren. |
| `DELETE` | `/api/users/{id}` | admin, nur Sitzung | `204`. Nicht sich selbst, nicht den letzten Administrator. |
| `DELETE` | `/api/users/{id}/totp` | admin, nur Sitzung | Setzt den zweiten Faktor eines anderen Kontos zurück (ein ausgesperrter Kollege). `204`. |
| `GET` | `/api/tokens` | nur Sitzung | Jedes API-Token: `id`, `name`, `prefix`, `scope`, `created_at`, `created_by` (das erstellende Konto), `expires_at` (`null`: nie), `expired`, `allowed_networks` (leer: überall), `last_used_at`, `last_used_ip`, `revoked_at`. Nie das Secret. |
| `POST` | `/api/tokens` | admin, nur Sitzung | `{"name": "Grafana", "scope": "read", "expires_in_days": 90, "allowed_networks": ["192.168.1.0/24"]}`. `name` ist erforderlich (höchstens 80 Zeichen); `scope` ist `read` (Standard) oder `write`; `expires_in_days` läuft von 1 bis 3650, `null` oder fehlend für ein nie ablaufendes Token; `allowed_networks` nimmt bis zu 32 Adressen oder CIDR-Bereiche, in Netzform gespeichert (`192.168.1.7/24` wird `192.168.1.0/24`), leer oder fehlend für überall. `201` mit den Token-Feldern plus `secret` (einmal gezeigt). `400` benennt ein ungültiges Netzwerk. |
| `DELETE` | `/api/tokens/{id}` | admin, nur Sitzung | Widerrufen. `204`; `404`, wenn unbekannt oder schon widerrufen. |

Ein gelistetes Token:

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

## Antwort-Header

Neben `X-DumbMonit-Api-Version` ([Versionierung](#versioning)), den
`RateLimit-*`-Headern ([Ratenlimits](#rate-limits)) und den
CORS-Headern
([Cross-Origin-Requests](#cross-origin-requests-cors)) trägt jede
Antwort `X-Content-Type-Options: nosniff`,
`Referrer-Policy: same-origin` und eine `Content-Security-Policy`.
Die Richtlinie beginnt bei `default-src 'none'` und öffnet nur, was
die Oberfläche tatsächlich nutzt — alles von DumbMonit selbst
bedient:

```
default-src 'none'; style-src 'self' 'unsafe-inline'; img-src 'self' data:;
font-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'self';
script-src 'self' 'nonce-<per response>'; frame-ancestors 'none'
```

Es gibt keine Drittanbieter-Origin darin, und es sollte nie eine
geben: Die Schriften, die Diagramm-Bibliothek und jedes Asset werden
im Binary mitgeliefert. Die Inline-Skripte der Seite (das vor dem
ersten Rendern angewendete Theme, SvelteKits Bootstrap) werden von
einer für jede Antwort neu erzeugten Nonce erlaubt, daher gilt
`'unsafe-inline'` nie für Skripte. Es gilt für Stile, und kann nicht
entfernt werden: Svelte setzt `style=`-Attribute auf Elementen, die
weder eine Nonce noch ein Hash abdecken kann.

Öffentliche Statusseiten sind zum Einbetten gemacht: `/s/<slug>`
und seine kompakte Ansicht `/s/<slug>/embed` — und nur diese beiden —
erhalten dieselbe Richtlinie ohne `frame-ancestors`. Alles sonst,
die Abonnement-Bestätigungs- und Abmelde-Seiten unter `/s/`
eingeschlossen, trägt auch `X-Frame-Options: DENY`.

Stellen Sie DumbMonit hinter einen Reverse Proxy, lassen Sie ihn
keinen zweiten `Content-Security-Policy`-Header hinzufügen: Browser
erzwingen den Schnitt aller davon, und ein Proxy-Standard ohne die
Nonce lässt eine leere Seite.

## Health

| Methode | Route | Auth | Zweck |
|---|---|---|---|
| `GET` | `/api/health` | öffentlich | Serverversion und der Zustand seiner Abhängigkeiten. Immer `200`: eine fehlschlagende Komponente steht im Body. |
| `GET` | `/api/update` | Sitzung oder Token | Laufende Version, neuestes Release, `update_available`; der Server fragt GitHub höchstens einmal pro Tag. `POST /api/update/check` schaut jetzt nach, `PUT /api/update/settings` schaltet die Prüfung ein oder aus. Siehe [Update](../install/update.md). |

```json
{"status":"ok","version":"0.1.0","database":{"ok":true},"victoria":{"ok":true,"embedded":true}}
```

`status` ist `degraded`, wenn eine Komponente fehlschlägt; diese
Komponente trägt dann einen `error`-String. `victoria.embedded` ist
`true`, wenn der Server sein eigenes VictoriaMetrics betreibt,
`false`, wenn `DUMBMONIT_VM_URL` auf ein externes zeigt.

## Backup und Wiederherstellung

| Methode | Route | Auth | Zweck |
|---|---|---|---|
| `GET` | `/api/backup` | admin, nur Sitzung | Was ein Bündel enthalten würde (`contents`, eine Zeile pro Abschnitt mit ihrer Zählung), die Bündel-Formatversion, wo das Instanz-Secret lebt (`secret_source`: `file` oder `environment`) und der Zustand der geplanten lokalen Backups (`schedule`). |
| `POST` | `/api/backup` | admin, nur Sitzung | `{"passphrase": "…", "include_account_secrets": false}`. Gibt das Bündel selbst als JSON-Datei zurück: einen Klartext-Header (`format`, `version`, `created_at`, `summary`, `kdf`) und einen `payload`, mit AES-256-GCM unter einem von der Passphrase durch Argon2id abgeleiteten Schlüssel verschlüsselt. Mindestens 16 Zeichen, sonst `400`. |
| `POST` | `/api/backup/restore` | admin, nur Sitzung | `{"bundle": {…}, "passphrase": "…", "apply": false}`. Dry Run, sofern `apply` nicht `true` ist; beide führen denselben Code aus, der Dry Run innerhalb einer zurückgerollten Transaktion. Antwortet mit dem Bericht: pro Abschnitt, `created`, `updated`, `skipped` und `notes`, plus `warnings`. `400` bei falscher Passphrase, einer veränderten Datei, oder einem in einer neueren Formatversion geschriebenen Bündel. |
| `POST` | `/api/backup/local` | admin, nur Sitzung | Schreibt sofort ein Backup im Stil der geplanten lokalen, und antwortet mit dem neuen `schedule`. |

Ein Bündel trägt jedes Credential der Instanz, mit der Passphrase
neu verschlüsselt, daher lehnen diese Routen API-Tokens mit einer
Erklärung ab: Nur ein in der Web-Oberfläche eingeloggter
Administrator erhält eines. Siehe
[Backup und Wiederherstellung](../install/backup.md).

## Gerätearten

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/api/collectors` | Jede von diesem Server überwachbare Geräteart: `kind`, `label`, `summary`, `examples`, `credential_types`, `address_hint`, `default_port`, `setup` (`title`, `steps`, `warning`, `doc_url`) und `options` (`key`, `label`, `help`, `placeholder`, `default`, `required`, `input`, `choices`). Von einem [Integrationspaket](../packs/index.md) mitgebrachte Arten werden genauso gelistet, als `pack.<id>`. |

## Integrationspakete

Admin-Sitzung oder API-Token (`write`-Scope für Änderungen). Siehe
[Integrationspakete](../packs/index.md).

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/api/packs` | Installierte Pakete: `id`, `version`, `label`, `summary`, `kind` (`null` für ein Paket nur mit SNMP-Profilen), `enabled`, `installed_at`, `sha256`, `metrics`, `rules` (uids), `snmp_profiles`, `targets` (es nutzende Geräte), `warnings`, und `error`, wenn das gespeicherte Paket die Validierung nicht mehr besteht. |
| `GET` | `/api/packs/{id}` | Ein Paket. |
| `POST` | `/api/packs` | Installieren oder aktualisieren. Body: das YAML (`application/yaml`, `text/plain`…) oder `{"yaml": "…"}` in JSON. `201`, wenn erstellt, `200`, wenn aktualisiert oder unverändert, mit `outcome` (`created`, `updated`, `unchanged`), `pack`, `rules_added` und `restart_required` (SNMP-Profile). `400` listet jeden Validierungsfehler. |
| `PUT` | `/api/packs/{id}/enable`, `/api/packs/{id}/disable` | Registriert oder zieht die Geräteart des Pakets zurück; Geräte, Historie und Regeln bleiben. |
| `DELETE` | `/api/packs/{id}` | Deinstallieren, mit seinen Regeln. `409`, solange Geräte es nutzen. |

## Targets (Geräte)

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/api/targets` | Jedes Gerät listen. |
| `POST` | `/api/targets` | Eines erstellen. `201` mit dem Gerät. Die Profilerkennung startet für SNMP im Hintergrund. |
| `GET` | `/api/targets/{id}` | Ein Gerät. |
| `PUT` | `/api/targets/{id}` | Es ersetzen. Das Weglassen von `credential` behält das gespeicherte; das Weglassen von `profile_id` behält das erkannte Profil (`""` löscht es). `enabled: false` zu setzen löscht die Alarme des Geräts ohne Benachrichtigung. |
| `DELETE` | `/api/targets/{id}` | `204`. Löscht die Alarme des Geräts ohne Benachrichtigung und löscht seine Zeitreihen aus VictoriaMetrics (Best Effort). |
| `POST` | `/api/targets/{id}/probe` | Jetzt abfragen: `{"sample_count": 42, "series": ["if_octets_in{host=\"Core switch\",ifname=\"eth0\"}", …]}` — die Zeitreihen-Schlüssel, wie der Collector sie erzeugte, ohne das `dumbmonit_`-Präfix, das der Writer auf dem Weg zu VictoriaMetrics hinzufügt. |
| `POST` | `/api/targets/{id}/discover` | Profilerkennung erneut ausführen: `{"profile_id": "host-resources"}` oder `null`. |

Ein zurückgegebenes Gerät:

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

Das Secret selbst wird nie zurückgegeben: `credential_kind` ist ein
Label. Server-Zeitstempel sind UTC ohne Suffix. `error_kind`
klassifiziert `last_error`: `down` (das Gerät hat nicht geantwortet)
oder `config` (unsere Seite — Zugangsdaten, Adresse, Option), was die
UI als *Unreachable* oder *Misconfigured* zeigt; `null`, wenn die
letzte Abfrage gelang.

Eines erstellen:

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

Credential-Formen (`type`): `none`; `snmp_community`
(`community`); `snmp_v3` (`username`, optional `auth: {protocol,
passphrase}` mit `md5`, `sha1`, `sha224`, `sha256`, `sha384` oder
`sha512`, optional `privacy: {protocol, passphrase}` mit `des`,
`aes128`, `aes192` oder `aes256`, optional `context`);
`api_token` (`token`, oder die zwei Hälften `token_id` und `secret`,
die der Server für Proxmox VE und PBS zu
`user@realm!name=secret` verbindet); `username_password`
(`username`, `password`). `interval_secs` ist standardmäßig 60 und
kann nicht unter 10 gehen. `name` hat höchstens 200 Zeichen und
`address` höchstens 253 (`400` darüber hinaus). `parent_id` muss ein
existierendes Gerät benennen (`400 Parent device N not found.`).
Art-Optionen liegen in `tags` unter ihrem Schlüssel.

`via_agent` benennt den [Relay-Agent](../install/remote-site.md),
der das Gerät von seinem eigenen Netzwerk aus abfragt statt dem
Server; `null` (der Standard) bedeutet, der Server fragt es ab. Es
muss ein `agent`-Gerät sein (`400` sonst), ein Gerät kann sich nicht
selbst relayen, und ein `agent`-Gerät kann nicht relayt werden. Bei
`PUT` behält ein *weggelassenes* `via_agent` den gespeicherten Wert,
wie `profile_id` und `credential`; senden Sie `null`, um zur
direkten Abfrage zurückzukehren. `POST /api/targets/{id}/probe` und
`/discover` auf einem relayten Gerät laufen über das Relay und
warten auf seine Antwort (`409`, wenn bereits eine Abfrage läuft,
`400 Timed out: relay agent …`, wenn es nicht antwortet).

## Discovery

| Methode | Route | Auth | Zweck |
|---|---|---|---|
| `POST` | `/api/discovery` | admin | `{"cidr": "192.168.1.0/24", "community": "public", "port": 161, "timeout_ms": 1000}`. Ein Netzwerk nach SNMP-Geräten scannen. `cidr` ist erforderlich; `community` ist standardmäßig `public`; `timeout_ms` wird auf 100–10.000 begrenzt. Netzwerke größer als 4096 Adressen werden abgelehnt. Der Scan ist ein `POST` mit einem JSON-Body, sodass die Community nie in einer URL oder einer Log-Zeile erscheint. |

```json
{
  "scanned": 254,
  "devices": [
    {"address": "192.168.1.10", "sysname": "sw-core", "sysdescr": "…", "sysobjectid": "1.3.6.1.4.1.8072.3.2.10", "suggested_profile": "host-resources"}
  ]
}
```

## Erstlauf-Anleitung

Die drei Schritte, die die Übersicht auf einer neuen Instanz zeigt.
Der Zustand wird auf der Instanz gehalten, nicht im Browser, daher
überspringt das Überspringen eines Schritts ihn überall.

| Methode | Route | Auth | Zweck |
|---|---|---|---|
| `GET` | `/api/onboarding` | Sitzung | Wo die Instanz steht. |
| `PUT` | `/api/onboarding` | admin | `{"skipped": true}` räumt die gesamte Anleitung weg; `{"dismissed": ["test"]}` räumt einzelne Schritte weg. Ein weggelassenes Feld behält seinen gespeicherten Wert; eine andere Schritt-ID als `device`, `channel` oder `test` ist ein `400`. Gibt dasselbe Dokument wie `GET` zurück. |

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

`has_target`, `has_channel` und `notification_confirmed` werden bei
jedem Lesen neu berechnet: Es sind Fakten über die Instanz, keine
von der Oberfläche gesetzten Flags. `notification_confirmed` ist
wahr, sobald eine Nachricht tatsächlich einen Kanal verlassen hat —
eine Testnachricht zählt, ein nur erstellter Kanal nicht.

`complete` ist eingerastet und geht nie zu false zurück. Der Server
setzt es, wenn die drei Schritte erledigt sind (fertig, oder
weggeräumt), und auch beim allerersten Lesen einer Instanz, die
schon ein Gerät und einen Kanal hatte — daher zeigt ein Upgrade einer
etablierten Instanz nie eine Erstlauf-Anleitung.

## Metriken

Beide Routen leiten zu VictoriaMetrics weiter und geben Zeitreihen in
der Prometheus-Form zurück:
`[{"metric": {labels…}, "values": [[timestamp_seconds, "value"], …]}]`.

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/api/metrics/query?query=…` | Instant-MetricsQL-Abfrage. |
| `GET` | `/api/metrics/query_range?query=…&start=…&end=…&step=…` | Range-Abfrage. `start` und `end` sind **Millisekunden** seit der Epoche; `step` ist in Sekunden und wird so verbreitert, dass eine Zeitreihe nie 2.000 Punkte überschreitet. |

```bash
curl -b cookies.txt -G http://localhost:8080/api/metrics/query \
  --data-urlencode 'query=avg by (target, host) (dumbmonit_cpu_usage_percent)'
```

```json
[{"metric":{"host":"ThinkpadE14","target":"3"},"values":[[1757941205,"12.5"]]}]
```

## Scraping (außerhalb von `/api`)

Für ein bestehendes Prometheus oder Grafana. Beide nehmen ein
`read`-API-Token in `Authorization: Bearer dmt_…` (ein
Session-Cookie funktioniert auch); ein Scraper sendet keinen
`X-Requested-With`-Header und braucht keinen — diese Routen lesen
nur. Siehe die
[Metrik-Referenz](metrics.md#scraping-dumbmonit) für eine
copy-pastable `scrape_config` und die Grafana-Datenquelle.

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/metrics` | Gesundheit der Instanz selbst im Prometheus-Text-Expositionsformat: Scheduler, Abfragen, Schreibvorgänge, Alarmierung, Benachrichtigungen, Agents, Datenbank, VictoriaMetrics. Ein paar Dutzend Zeitreihen, nie eine pro Gerät. |
| `GET` | `/federate` | Die Messungen, im selben Format. `match[]` wählt Zeitreihen (wiederholbar, höchstens 10, standardmäßig `{__name__=~"dumbmonit_.*"}`), `max_lookback` erweitert das Fenster, in dem der letzte Punkt gesucht wird (standardmäßig `5m`). `400` über 8 MiB, mit dem Hinweis, was einzugrenzen ist. |
| `GET`, `POST` | `/prometheus/api/v1/{route}` | Die Lesehälfte der Prometheus-API, zum Store weitergeleitet, sodass Grafana `http://…/prometheus` als Prometheus-Datenquelle nutzen kann. Nur `query`, `query_range`, `query_exemplars`, `series`, `labels`, `label/{name}/values`, `metadata` und `status/buildinfo`; alles andere ist `404`. |

`DUMBMONIT_METRICS_PUBLIC=true` bedient die drei ohne Token — nur
hinter einer Firewall, da sie jede Messung jedes Geräts tragen.

## Alerts

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/api/alerts` | Aktive Alarme (pending, firing, suppressed, kürzlich resolved). Alarme gelöschter oder pausierter Geräte werden nie gelistet. |
| `GET` | `/api/alerts/history?since=2026-09-01T00:00:00Z&limit=200&dismissed=false` | Phasenübergänge. `since` ist RFC 3339, standardmäßig die letzten sieben Tage; `limit` muss positiv sein. `dismissed` (standardmäßig `false`) schließt mit den zwei Routen unten gelöschte Einträge aus; nichts wird je durch ein Löschen zerstört, nur aus der Standardansicht verborgen. |
| `POST` | `/api/alerts/history/{id}/dismiss` | Löscht einen `resolved`-Eintrag aus der Standard-History-Ansicht. `204`. `404` bei unbekannter ID oder einem noch aktiven Übergang. Operator; protokolliert. |
| `POST` | `/api/alerts/history/dismiss-resolved` | Löscht jeden noch nicht gelöschten `resolved`-Eintrag auf einmal ("Clear all resolved"). Gibt `{"dismissed": <count>}` zurück. Operator; protokolliert. |
| `POST` | `/api/alerts/{fingerprint}/ack` | Bestätigen: `{"duration_secs": 14400, "note": "…"}` oder `{"until": "2026-09-22T18:00:00Z"}` (eines der beiden; keines bedeutet 4 Stunden, höchstens 30 Tage). `{"until": null}` hebt es auf. Gibt den Alarm zurück. Operator; protokolliert. |
| `DELETE` | `/api/alerts/{fingerprint}/ack` | Bestätigung aufheben. Gibt den Alarm zurück. Operator. |

Ein aktiver Alarm:

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

Ein Historieneintrag hat `id`, `fingerprint`, `rule_uid`,
`target_id`, `from_phase`, `to_phase`, `severity`, `value`,
`notified`, `reason` (leer, oder warum nichts gesendet wurde:
`learning: would have fired`, `suppressed: device 2 unreachable`,
`maintenance window`, `acknowledged by admin`, `device removed or
disabled`), `at` und `dismissed` (aus der aktiven Ansicht gelöscht;
die Zeile selbst bleibt). `acked` ist wahr, während `acked_until` in
der Zukunft liegt: Erinnerungen und Eskalationen pausieren, die
Behebung wird trotzdem gemeldet und löscht die Bestätigung. Ein
`target_id`, das nicht mehr existiert, wird von der UI als
"(deleted device)" angezeigt.

### Regeln

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/api/alerts/rules` | Jede Regel. |
| `POST` | `/api/alerts/rules` | Erstellen. `201`. |
| `PUT` | `/api/alerts/rules/{id}` | Aktualisieren. Eine eingebaute Regel behält ihre Abfrage. |
| `DELETE` | `/api/alerts/rules/{id}` | `204`. Eingebaute Regeln können nicht gelöscht werden. |
| `POST` | `/api/alerts/rules/{id}/enable` | `{"enabled": false}`. Gibt die Regel zurück. |

Eine Regel:

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

Eine zu erstellen braucht mindestens `name` und `query`; `kind`
(`threshold`, `anomaly`, `predict`), `operator`, `threshold`,
`for_secs`, `severity` (`info`, `warning`, `critical`), `selector`
(`{"kind":"all"}`, `{"kind":"ids","ids":[…]}` oder
`{"kind":"labels","labels":{…}}`), `channels` (Kanal-IDs; leer
bedeutet jeder aktivierte Kanal), `unit`, `repeat_secs`,
`escalate_after_secs` und `enabled` sind optional.

### Silences (Wartungsfenster)

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/api/alerts/silences` | Jedes Fenster, mit `active_now`, `active_until` und `next_start_at` (RFC 3339, vom Server berechnet). |
| `POST` | `/api/alerts/silences` | Erstellen. `201`. Operator. |
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

Ein einmaliger Zeitplan ist `{"kind": "once", "starts_at":
"2026-09-20T22:00:00Z", "ends_at": "2026-09-21T02:00:00Z"}`. Ein
monatlicher ist `{"kind": "monthly", "days": [1], "nth_weekdays":
[{"nth": 1, "weekday": 6}], "start_minute": 120, "duration_minutes":
120, "timezone": "Europe/Paris"}` — `nth` ist 1 bis 5, oder `-1` für
den letzten des Monats; ein Monat ohne dieses Vorkommnis wird
übersprungen. Tage sind 0 = Montag … 6 = Sonntag (1–31 für ein
monatliches `days`); Minuten sind seit lokaler Mitternacht
(0–1439). Bei einem wiederkehrenden Zeitplan gewinnt `timezone` (ein
IANA-Name) über `utc_offset_minutes` und ist, was ein Fenster über
Sommerzeitänderungen hinweg bei seiner lokalen Stunde hält; ein
unbekannter Zonenname wird abgelehnt.

### Gerätespezifische Überschreibungen

Eine Regel kann für ein Gerät angepasst werden, ohne die Regel
selbst anzurühren. `{"enabled": false}` ist, was die
**Ignore**-Aktion der UI (am Alarm, der Geräteseite oder der Regel)
schreibt: Das Gerät wird endgültig, reversibel aus dieser einen
Regel entfernt, ohne die Regel für irgendjemand anderen zu
deaktivieren.

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/api/alerts/overrides?target_id=4` | Jede Überschreibung, oder die eines Geräts: `rule_uid`, `target_id`, `threshold`, `clear_threshold`, `enabled`. |
| `GET` | `/api/alerts/rules/{id}/overrides` | Die Überschreibungen einer Regel. |
| `PUT` | `/api/alerts/rules/{id}/overrides/{target_id}` | `{"threshold": 95, "clear_threshold": 90, "enabled": true}` — jedes Feld optional; `null` bedeutet "wie die Regel". Erstellt oder ersetzt. Ein Operator darf nur `{"enabled": false}` (ignorieren) senden; die Schwellenwert-Überschreibungen des Geräts, falls vorhanden, bleiben erhalten. |
| `DELETE` | `/api/alerts/rules/{id}/overrides/{target_id}` | `204`. Für einen Operator hebt es nur das Ignorieren auf: Schwellenwert-Überschreibungen bleiben. |

## Benachrichtigungsrichtlinie

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/api/notify/policy` | Die globale Richtlinie: `batch_window_secs` (0 = sofort senden), `max_per_hour` pro Kanal (0 = unbegrenzt), `flap_events`, `flap_window_secs`, `flap_hold_secs` (0 = keine Flackererkennung), `public_url` (Links in Nachrichten; leer = `DUMBMONIT_PUBLIC_URL`), `escalate_after_secs` und `escalate_channel` (0 / `null` = keine Eskalation). |
| `PUT` | `/api/notify/policy` | Dieselben Felder, jedes optional: Ein weggelassenes Feld behält seinen Wert. `"escalate_channel": null` schaltet die Eskalation aus. Gibt die Richtlinie zurück. |
| `POST` | `/api/notify/match-preview` | `{"matcher": {…}}` → welche Geräte ein Routing-Filter eines Kanals auswählt: `devices` (`id`, `name`, `kind`, `tags`, `matched`), `matched`, `total`, `rules`, `excluded_rules`. |

## Benachrichtigungskanäle

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/api/notify/kinds` | Jede Kanalart mit ihren `settings`- und `secrets`-Feldern (`key`, `label`, `required`, `input`, `help`, `placeholder`, `options`, `shape`, `default`) und einer `doc_url` wie `https://dumbmonit.readthedocs.io/en/latest/notifications/#discord`. |
| `GET` | `/api/notify/channels` | Jeder Kanal: `id`, `name`, `kind`, `enabled`, `settings`, `has_secret`, `last_error`, `last_sent_at`, `policy` (`min_severity`, `notify_resolved`, `min_interval_secs`, `quiet_hours`, `matcher`). Secrets werden nie zurückgegeben. |
| `POST` | `/api/notify/channels` | Erstellen. `201`. |
| `PUT` | `/api/notify/channels/{id}` | Aktualisieren. Das Weglassen von `secrets` behält die gespeicherten; `"secrets": {}` löscht sie. In `policy` behält das Weglassen von `matcher` den Routing-Filter, und `"matcher": null` löscht ihn. |
| `DELETE` | `/api/notify/channels/{id}` | `204`. |
| `POST` | `/api/notify/channels/{id}/test` | Eine Testnachricht senden: `{"ok": true, "message": "…"}`. |

```json
{
  "name": "Ops Discord",
  "kind": "discord",
  "enabled": true,
  "settings": {"username": "DumbMonit"},
  "secrets": {"webhook_url": "https://discord.com/api/webhooks/…"}
}
```

Die genauen Schlüssel je Art kommen von `/api/notify/kinds` und sind
in [Benachrichtigungskanäle](../notifications.md) dokumentiert.

### Web-Push-Geräte

Die Geräte des **eingeloggten Kontos** (nur Sitzung, Viewer
eingeschlossen; API-Tokens erhalten `401`). Ein `webpush`-Kanal
sendet an sie; siehe
[Push-Benachrichtigungen (PWA)](../using/push-notifications.md).

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/api/webpush` | `{"public_key": "…", "devices": […]}`: der öffentliche VAPID-Schlüssel (base64url, der `applicationServerKey` zum Abonnieren) und die Geräte des Kontos — `id`, `device` (`Firefox on Android`), `push_service` (Host), `fingerprint` (erste 16 Hex-Zeichen des SHA-256 des Endpunkts), `created_at`, `last_success_at`, `last_error`. Der Endpunkt, die Browser-Keys und der private Schlüssel werden nie zurückgegeben. |
| `POST` | `/api/webpush/subscriptions` | `PushSubscription.toJSON()`: `{"endpoint": "https://…", "keys": {"p256dh": "…", "auth": "…"}}`. `201` mit dem Gerät. Der Endpunkt muss HTTPS auf einer öffentlichen Adresse sein: private, Loopback- und Link-Local-Adressen erhalten `400`. Derselbe Endpunkt erneut ersetzt seine Zeile. |
| `DELETE` | `/api/webpush/subscriptions/{id}` | `204`; `404` für ein Gerät eines anderen Kontos. |
| `POST` | `/api/webpush/test` | `{}` oder `{"id": 3}`. Sendet eine Testbenachrichtigung: `{"ok", "delivered", "failed", "removed", "message"}`. Mit `404`/`410` beantwortete Abonnements werden entfernt. |

## E-Mail-Berichte

Nur Administratoren, `GET` eingeschlossen (die Antwort listet
Empfänger). Bearer-Tokens brauchen den Scope `write` für Änderungen.
Siehe [E-Mail-Berichte](../using/reports.md).

| Methode | Route | Auth | Zweck |
|---|---|---|---|
| `GET` | `/api/reports/schedules` | admin | Jeder Bericht: `id`, `name`, `enabled`, `frequency`, `weekday` (0 ist Montag), `day_of_month`, `hour`, `timezone`, `recipients`, `channel_id`, `last_sent_at`, `last_error`, `next_run_at`. |
| `POST` | `/api/reports/schedules` | admin | Dieselben beschreibbaren Felder; jedes ist standardmäßig wöchentlich, Montag, 8:00, `UTC`. `recipients` hält höchstens 20 Adressen und ist erforderlich, um den Bericht zu aktivieren; `channel_id` muss ein `smtp`-Kanal oder `null` sein. `201`; `409` über 10 Berichte hinaus. |
| `GET` | `/api/reports/schedules/{id}` | admin | Ein Bericht. |
| `PUT` | `/api/reports/schedules/{id}` | admin | Vollständiger Ersatz. |
| `DELETE` | `/api/reports/schedules/{id}` | admin | `204`. |
| `POST` | `/api/reports/schedules/{id}/send-test` | admin | Mailt eine Vorschau an die gespeicherten Empfänger: `{"sent": 2, "failed": 0}`. Rührt den Zeitplan nicht an. `400` ohne nutzbaren E-Mail-Kanal; `409` innerhalb von 30 Sekunden nach der vorherigen Vorschau. |

## Statusseiten und Vorfälle

| Methode | Route | Auth | Zweck |
|---|---|---|---|
| `GET` | `/api/status-pages` | Sitzung | Jede Seite mit ihren Elementen: `page` (`id`, `slug`, `title`, `description`, `published`, `theme`, `show_uptime_days`, `created_at`, `updated_at`) und `items` (`id`, `page_id`, `target_id`, `label`, `group_name`, `position`). |
| `POST` | `/api/status-pages` | admin | `{"title", "slug", "description", "published", "theme", "show_uptime_days"}`. `title` ist erforderlich (höchstens 120 Zeichen); `slug` (`^[a-z0-9-]{2,40}$`) wird aus dem Titel abgeleitet, wenn weggelassen; `theme` ist `auto` (Standard), `light` oder `dark`; `show_uptime_days` läuft von 7 bis 90 (standardmäßig 90). `201`; `409` bei einem schon vergebenen Slug. |
| `GET` | `/api/status-pages/{id}` | Sitzung | Eine Seite mit ihren Elementen. |
| `PUT` | `/api/status-pages/{id}` | admin | Dieselben Felder, aber ein vollständiger Ersatz statt eines Patches: `title` ist erforderlich, und jedes weggelassene Feld geht auf seinen Standard zurück (Slug aus dem Titel neu abgeleitet, leere Beschreibung, `published: false`, `theme: auto`, 90 Tage Uptime). |
| `DELETE` | `/api/status-pages/{id}` | admin | `204`. Die öffentliche URL hört auf zu antworten. |
| `POST`, `PUT` | `/api/status-pages`, `/api/status-pages/{id}` — Look und Abonnements | admin | Akzeptieren auch `accent` (`default`, `blue`, `teal`, `violet`, `rose`, `amber`), `scenes` (Scene-IDs aus der Liste des Editors — Städte, die neuen Weltwunder und die antiken Weltwunder, z. B. `venice`, `paris`, `giza`, `tajmahal`, keine Duplikate; `[]` für keine; beim Erstellen weggelassen: `["paris"]`), `simple` (Boolean, standardmäßig `false`: Die öffentliche Seite zeigt keine Scene, kein Mascot und keine Animation und folgt der Hell/Dunkel-Präferenz des Besuchers), `scene_rotation` (`visit`, `1m`, `10m` oder `1h`), `footer_text` (Klartext, 280 Zeichen), `homepage_url` (`http(s)://` oder leer), `subscribe_channel_id` (ein `smtp`-Kanal, oder `null`, um E-Mail-Abonnement auszuschalten) und `domain` (der öffentliche Hostname der Seite, wie `status.example.com`, oder `null`/`""` für keinen — siehe [Benutzerdefinierte Domain](../using/status-pages.md#custom-domain)). `domain` wird in Kleinbuchstaben umgewandelt und verliert ein `https://`-Präfix oder einen abschließenden Schrägstrich; ein Port, Pfad oder eine IP-Adresse ist `400`, die Adresse von DumbMonit selbst (seine öffentliche URL, oder der Host, auf dem der Request einging) ist `400`, und ein von einer anderen Seite genutzter Name ist `409`. Anders als die übrigen Felder behalten diese acht ihren gespeicherten Wert, wenn sie bei einem `PUT` weggelassen werden. Die Seitenansicht gibt sie mit `logo_type` zurück (`image/png`, `image/jpeg`, `image/webp` oder `null`). |
| `GET`, `PUT`, `DELETE` | `/api/status-pages/{id}/logo` | Sitzung / admin | Das Logo der Seite. `PUT` nimmt `{"data": "<base64 or data: URL>"}`: PNG, JPEG oder WebP, nach Inhalt erkannt, höchstens 256 KiB, alles andere `400`. `204`. `GET` ist die Vorschau des Editors (Entwürfe eingeschlossen). |
| `GET` | `/api/status-pages/{id}/subscribers` | Sitzung | E-Mail-Abonnenten: `id`, `page_id`, `email`, `confirmed_at` (`null`, während ausstehend), `created_at`. Ihre Tokens verlassen den Server nie. |
| `DELETE` | `/api/status-pages/{id}/subscribers/{subscriber}` | admin | Entfernt eine Adresse. `204`. |
| `PUT` | `/api/status-pages/{id}/items` | admin | `[{"target_id": 4, "label": "NAS", "group_name": "Storage"}, …]` — die vollständige, geordnete Liste der auf der Seite gezeigten Geräte. |
| `GET` | `/api/incidents` | Sitzung | Jeder Vorfall mit seinen Updates: `incident` (`id`, `page_id`, `title`, `kind`, `status`, `severity`, `starts_at`, `ends_at`, `created_at`, `updated_at`) und `updates`. |
| `POST` | `/api/incidents` | admin | `{"title", "kind", "status", "severity", "page_id", "starts_at", "ends_at", "body"}` — `body` ist das erste Update. `201`. |
| `PUT` | `/api/incidents/{id}` | admin | Dieselben Felder; ein weggelassenes Feld behält seinen Wert. |
| `DELETE` | `/api/incidents/{id}` | admin | `204`. |
| `GET` | `/api/incidents/{id}/updates` | Sitzung | Die Zeitleiste: `id`, `incident_id`, `status`, `body`, `created_at`. |
| `POST` | `/api/incidents/{id}/updates` | admin | `{"status": "monitoring", "body": "…"}`. Fügt ein Update hinzu und bewegt den Vorfall zu `status`. `201`. |
| `GET` | `/api/public/status/{slug}` | öffentlich | Das JSON-Dokument, aus dem eine veröffentlichte Seite gebaut wird (auch `badge.svg` und `rss` unter demselben Pfad). In `page` ist `show_uptime_days` das konfigurierte Maximum und `history_days` das tatsächlich gezeigte Fenster (das Alter der Daten der Seite, von 7 Tagen bis zu diesem Maximum), und `simple` spiegelt den Simple Mode. Siehe [Statusseiten](../using/status-pages.md). Auf der benutzerdefinierten Domain einer Seite (`Host` gleich ihrer `domain`) antworten nur die Routen `/api/public/status/{slug}/…` dieser Seite; jede andere Route dieser Referenz ist dort `404`. |
| `GET` | `/api/public/status/{slug}/uptime.svg`, `/response.svg` | öffentlich | Seiten-Badges: durchschnittliche Uptime über `?days=` `1`, `7`, `30` (Standard) oder `90` — `400` über die Historie der Seite hinaus — und durchschnittliche Antwortzeit. SVG, `Cache-Control: public, max-age=60`. |
| `GET` | `/api/public/status/{slug}/components/{key}/badge.svg`, `/uptime.svg`, `/response.svg` | öffentlich | Dieselben drei Badges für einen Dienst; `key` ist der `key` des Dienstes im öffentlichen Dokument (sein Label in URL-Form). `404` für einen unbekannten Key. |
| `GET` | `/api/public/status/{slug}/logo` | öffentlich | Das Logo der Seite mit seinem Bildtyp, einen Tag cacheable (das `logo_url` des Dokuments trägt eine Version). `404` ohne eines. |
| `POST` | `/api/public/status/{slug}/subscribe` | öffentlich | `{"email": "…"}`. Mailt einen Bestätigungslink über den SMTP-Kanal der Seite. Immer `202` mit derselben Meldung, egal welcher Zustand die Adresse hat; `400` für eine ungültige Adresse; `404`, wenn die Seite kein E-Mail-Abonnement anbietet; `409`, wenn die Seite keine weiteren Abonnenten aufnehmen kann; `429` mit `Retry-After` über fünf Requests pro Client und sechzig insgesamt pro 15 Minuten hinaus. |
| `POST` | `/api/public/status/{slug}/confirm?token=…` | öffentlich | Bestätigt ein Abonnement (der Link der Bestätigungs-E-Mail öffnet `/s/{slug}/confirm`, dessen Schaltfläche hierher postet). `404` für ein unbekanntes oder abgelaufenes Token. |
| `POST` | `/api/public/status/{slug}/unsubscribe?token=…` | öffentlich | Entfernt ein Abonnement; der Body wird ignoriert, daher ist es auch das RFC-8058-Ein-Klick-Ziel von `List-Unsubscribe`. Immer `200`. |

## Agent-Tokens und Ingest

| Methode | Route | Auth | Zweck |
|---|---|---|---|
| `GET` | `/api/agent/tokens` | nur Sitzung | Jedes Token: `id`, `name`, `prefix`, `created_at`, `last_used_at`, `revoked_at`, `max_uses` (`null` für ein Flotten-Token), `uses`, `expires_at`. |
| `POST` | `/api/agent/tokens` | admin, nur Sitzung | `{"name": "Home fleet", "base_url": "http://server:8080", "reusable": false, "max_uses": null, "expires_in_days": null}`. `201` mit den Token-Feldern plus `secret` (einmal gezeigt), `install_linux` und `install_windows`. `base_url` ist die URL, die Agents nutzen werden; standardmäßig die Horchadresse. Ohne `reusable` ist das Token **einmalig nutzbar**: Es registriert eine Maschine und nicht mehr. Mit ihm begrenzt `max_uses` die Registrierungen (`null`: kein Limit). `expires_in_days` stoppt *Registrierungen* nach so vielen Tagen; schon registrierte Maschinen melden weiter. `400` bei einer Zahl unter 1. |
| `DELETE` | `/api/agent/tokens/{id}` | admin, nur Sitzung | Widerrufen. `204`; `404`, wenn unbekannt oder schon widerrufen. |
| `POST` | `/api/ingest` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Empfängt einen Batch Samples von einem Agent (Bodies bis 16 MB). Nicht zum von-Hand-Aufrufen gedacht. Der Header trägt das Binding-Secret der Maschine; ein Agent ohne eines lässt ihn weg, was ist, wie er eines anfragt. Die Antwort fügt `agent_secret` hinzu (das Secret, genau einmal zurückgegeben, wenn die Maschine gebunden wird) und `bound`. `403`, wenn die Maschine an eine andere Agent-Installation gebunden ist, oder wenn das Token nicht mehr registrieren kann — die Meldung sagt, was und wie zu beheben. |
| `GET` | `/api/agent/relay?key=…&wait=N` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | An einen Relay-Agent delegierte Abfragen (`relay: true`). Bis zu `wait` Sekunden gehalten (höchstens 25), wenn nichts ausstehend ist. Jedes Element ist ein Befehl der Art `probe`, dessen `args` das Target, sein entschlüsseltes Credential, `timeout_secs` und `discover` tragen. Nie auf die Platte geschrieben. |
| `POST` | `/api/agent/relay/{id}?key=…` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Ergebnis einer delegierten Abfrage: `{"duration_ms", "error", "samples", "profile_id"}` (Bodies bis 16 MB). `204`; `404`, wenn die Abfrage abgelaufen ist oder einem anderen Agent gehört. |
| `GET` | `/api/agent/commands?key=…&wait=N` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Für einen Agent eingereihte Befehle (Container-Neustart oder Update), bis zu `wait` Sekunden gehalten, wenn nichts ausstehend ist. Nicht zum von-Hand-Aufrufen gedacht. `key` allein beweist nichts — er ist öffentlich — daher muss eine gebundene Maschine auch ihr Binding-Secret präsentieren; sonst `403`, was eine Maschine davon abhält, die Befehle einer anderen zu übernehmen. |
| `POST` | `/api/agent/commands/{id}?key=…` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Fortschritt und Ergebnis eines Befehls, vom Agent gemeldet. `result` wird auf die letzten 4 KB gekürzt, mit `[truncated by the server, beginning dropped]` am Anfang, wenn das passiert. |
| `GET` | `/api/relays` | Sitzung | Jedes Agent-Gerät als mögliches Relay: `id`, `name`, `site`, `relay` (deklariert `relay: true`), `last_seen_at`, `relayed` (durch es erreichte Geräte). Relays zuerst, dann nach Namen. |

```json
{
  "id": 1, "name": "Home fleet", "prefix": "dmon_ab12",
  "created_at": "2026-09-15 13:00:00", "last_used_at": null, "revoked_at": null,
  "secret": "dmon_…",
  "install_linux": "curl -sSL http://server:8080/install.sh | sh -s -- --token=dmon_… --url=http://server:8080",
  "install_windows": "& ([scriptblock]::Create((irm http://server:8080/install.ps1))) -Token dmon_… -Url http://server:8080"
}
```

## Container und Befehle (Agent-Geräte)

Aktionen auf den Containern einer Maschine, die den Agent
betreibt. Jede Route antwortet mit `404`, wenn das Gerät kein
`agent`-Target ist.

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/api/targets/{id}/agent` | Die Maschine, wie ihr Agent sie zuletzt beschrieb: `hostname`, `os`, `os_version`, `arch`, `agent_version`, `commands_supported`, `relay`, `site`, `relayed` (durch diesen Agent erreichte Geräte), `last_seen_at`, `binding`, `bound`, `bound_at`, `rebind_until`. `404`, bis ein Agent gemeldet hat. `commands_supported` ist nur `true`, wenn der Agent angegeben hat, dass er Befehle abholt (`commands: true`, der Standard aktueller Agents); ein älterer Agent oder einer mit `commands: false` gibt `false`. `binding` ist `bound`, `pending` (das Binary kann gebunden werden und wird es bei seinem nächsten Batch) oder `unsupported` (ein älterer Agent als Binding: neu installieren). |
| `POST` | `/api/targets/{id}/agent/rebind` | Admin. Öffnet ein einstündiges Fenster, während dem sich diese Maschine mit einem gültigen Enrollment-Token erneut binden kann — der Weg zurück nach einer Neuinstallation, die das Secret des Agents mitnahm. `{"rebind_until", "minutes"}`. Das Fenster schließt sich, sobald es genutzt wird, und das alte Secret funktioniert nicht mehr. `404`, bis ein Agent gemeldet hat. |
| `GET` | `/api/targets/{id}/containers` | Jeder Container aus dem letzten Batch: `name`, `image`, `up`, `health`, `restart_count`, `uptime_seconds`, `image_age_seconds`, `update_available`, `policy`, `last_command`. |
| `PUT` | `/api/targets/{id}/containers/{name}/policy` | `{"auto_restart": bool, "auto_update": bool, "prune_old_image": bool, "only_in_maintenance": bool}`. Jedes Feld ist optional: Ein weggelassenes Feld behält seinen gespeicherten Wert. Gibt die vollständige Richtlinie zurück. |
| `POST` | `/api/targets/{id}/containers/{name}/restart` | Einen Neustart einreihen. `201` mit dem Befehl. `404`, wenn `name` nicht im Inventar des Agents ist; `409`, wenn derselbe Befehl schon eingereiht ist oder läuft, oder wenn der Agent keine Befehle ausführen kann (siehe `commands_supported`). |
| `POST` | `/api/targets/{id}/containers/{name}/update` | Ein Update einreihen; optional `{"prune": bool}` (standardmäßig die Richtlinie). Dieselben Statuscodes wie restart. |
| `GET` | `/api/targets/{id}/commands` | Die letzten 20 Befehle, neueste zuerst: `id`, `kind` (`container.restart`, `container.update`), `args`, `status`, `requested_by` (ein Benutzer, oder `policy`), `created_at`, `started_at`, `finished_at`, `result`. |
| `DELETE` | `/api/targets/{id}/commands/{command_id}` | Einen eingereihten Befehl abbrechen. `204`; `409`, sobald der Agent ihn abgeholt hat oder wenn er schon abgeschlossen ist; `404`, wenn er nicht zu diesem Gerät gehört. |

`status` läuft `queued → running → done | failed`. Zwei weitere
Endzustände existieren: `cancelled` (eine Person hat den Befehl
zurückgezogen) und `expired` (der Server hat nach zehn Minuten
aufgegeben, weil kein Agent ihn abholte — Agent gestoppt, zu alt für
den Befehlskanal, oder mit `commands: false` installiert). Der
Server lässt veraltete Befehle jede Minute selbst ablaufen; ein
eingereihter Befehl blockiert nie länger als das einen neuen.

## Proxmox-VE-Gäste

Die Zeilen des **Guests**-Panels eines Proxmox-VE-Geräts,
zusammengesetzt aus der letzten Abfrage im Metrik-Store (vier
Instant-Abfragen, nichts vom Hypervisor verlangt). `404`, wenn das
Gerät nicht existiert, `400`, wenn es kein `proxmox`-Target ist.

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/api/targets/{id}/proxmox/guests` | Ein Objekt pro VM oder Container, nach Knoten dann VMID sortiert: `vmid`, `name`, `node`, `kind` (`qemu`, `lxc`), `status` (`running`, `stopped`, `paused`, `suspended`, `template`, `unknown`), `cpu_percent` (von den zugewiesenen Kernen), `cpu_count`, `memory_used_bytes`, `memory_total_bytes`, `memory_percent`, `balloon_bytes`, `disk_used_bytes`, `disk_total_bytes`, `disk_percent`, `agent` (`true` Guest-Agent antwortete, `false` aktiviert, aber still, `null` keiner), `network_in_bps`, `network_out_bps`, `disk_read_bps`, `disk_write_bps`, `uptime_seconds`, `last_backup_age_seconds`, `ha_state`, `pool` (sein Resource Pool), `lock` (nur, während ein Lock gehalten wird: `backup`, `migrate`…), `os` und `ip` (von innen des Gasts gesehen, stündlich aktualisiert). Jeder unbekannte Wert ist `null` — eine VM ohne Guest-Agent hat `disk_total_bytes`, aber `disk_used_bytes: null`; ein gestoppter Gast behält seine Größen und verliert seine Messungen. |

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

## Proxmox-VE-Knoten und Ceph

Die **Nodes**- und **Ceph**-Abschnitte derselben Geräteseite, aus
derselben letzten Abfrage gelesen. Dieselben Statuscodes wie oben.

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/api/targets/{id}/proxmox/nodes` | `{"nodes": […], "fencing_state", "fencing_armed"}` — der HA-Watchdog-Zustand (`armed`, `standby`…) und ob er einen verlorenen Knoten tatsächlich isolieren würde, dann ein Objekt pro Knoten, nach Namen sortiert: `name`, `up`, `cpu_percent`, `cpu_iowait_percent`, `memory_percent`, `ksm_shared_bytes`, `rootfs_percent`, `uptime_seconds`, `version` (die auf diesem Knoten installierte `pve-manager`-Version), `services_down` (Unit-Namen der gestoppten oder fehlgeschlagenen Daemons), `interfaces_offline` (zum Boot vorgesehene, nicht up Schnittstellen), `thin_pools` (`name`, `vg`, `used_percent`, `metadata_used_percent`, `size_bytes`) und `volume_groups` (`name`, `used_percent`, `size_bytes`). Ein nicht mehr antwortender Knoten behält seinen Eintrag mit `up: false`. |
| `GET` | `/api/targets/{id}/proxmox/ceph` | `available` (`false`, wenn der Cluster kein Ceph hat — die UI zeichnet dann überhaupt keinen Abschnitt), `health` (0 OK, 1 WARN, 2 ERR, 3 unknown), `health_status`, `bytes_used`, `bytes_total`, `used_percent`, `osds_total`, `osds_up`, `osds_in`, `osds` (`name`, `host`, `device_class`, `up`, `in`, `used_percent`, `used_bytes`, `total_bytes`, `apply_latency_ms`, `commit_latency_ms`), `pools` (`name`, `used_percent`, `used_bytes`, `size`, `min_size`, `pg_num`, `pg_num_optimal`, `autoscale`), `filesystems` (CephFS-Namen), `flags` (derzeit gesetzte OSD-Flags, wie `noout`) und `muted_checks` (stummgeschaltete Health-Checks, die `HEALTH_OK` nicht mehr erwähnt). |

## Proxmox Backup Server

Die Panels eines `pbs`-Geräts, aus der letzten Abfrage gelesen
(nichts vom Server verlangt außer dem Task-Log). `404`, wenn das
Gerät nicht existiert, `400`, wenn es kein `pbs`-Target ist.

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/api/targets/{id}/pbs/calendar?days=30&offset=120` | `{"probed_at", "days", "offset_minutes", "groups": […]}`: ein Eintrag pro Backup-Gruppe (`datastore`, `namespace`, `backup_type`, `backup_id`, `name`, `count`, `last_time`, `last_size`, `last_verified`, `last_success`, `last_failure`, `retention`) mit einem `days`-Array von `{date, state, runs, snapshot}`. `offset` ist der UTC-Versatz des Betrachters in Minuten, sodass Tage an lokaler Mitternacht geschnitten werden. |
| `GET` | `/api/targets/{id}/pbs/failures?days=30` | Fehlgeschlagene Tasks: `upid`, `worker_type`, `kind`, `worker_id`, `datastore`, `start`, `end`, `error`. |
| `GET` | `/api/targets/{id}/pbs/jobs` | `{"probed_at", "jobs": […]}`: Sync-, Verify-, Prune- und Garbage-Collection-Jobs mit ihrem Zeitplan und letzten Ergebnis. |
| `GET` | `/api/targets/{id}/pbs/health` | Datastores (Nutzung, geschätztes Voll-Datum), Platten (SMART, Verschleiß) und ZFS-Pools. |
| `GET` | `/api/targets/{id}/pbs/tasks/{upid}/log?lines=200` | Das Log eines Tasks, vom Server abgerufen: `{"upid", "lines": […]}`; `lines` begrenzt, wie viele zurückgegeben werden. |
| `GET` | `/api/targets/{id}/pbs/disks/smart?disk=/dev/sda` | SMART-Attribute einer Platte; `disk` ist erforderlich (`400` ohne es). |
| `GET` | `/api/targets/{id}/pdm/remotes` | Proxmox Datacenter Manager: Bestandstotals (`estate`) und eine Zeile pro föderierter Instanz (`id`, `kind`, `reachable`, `error`, `version`, `version_behind`, Knoten- und Gastzahlen, Speicher und Storage, `subscription`, `last_collection`, `tasks_failed`), nicht erreichbare zuerst. |
| `GET` | `/api/targets/{id}/pdm/failures?days=14` | Im Bestand fehlgeschlagene Tasks: `upid`, `remote`, `worker_type`, `kind`, `worker_id`, `node`, `start`, `end`, `error`. |
| `GET` | `/api/targets/{id}/pdm/health` | Der Konsolen-Host: CPU, Speicher, Root-Dateisystem, Uptime, Zertifikate, ausstehende Updates und Abonnement. |
| `GET` | `/api/targets/{id}/pmg/queues` | Proxmox Mail Gateway: die vier Postfix-Warteschlangen in Lesereihenfolge (`queue`, `messages`, `domains`, `oldest_age_seconds`, `top_domains`, `stuck`), plus `total_messages` und ein gatewayweites `stuck`. `oldest_age_seconds` ist eine untere Grenze: qshape meldet Altersbereiche. |
| `GET` | `/api/targets/{id}/pmg/traffic` | Die heutigen Totals (`mail`), die jüngere Verkehrskurve (`recent`), das Spam-Score-Histogramm, die heute gefangenen Viren, und Quarantäne-Zählungen (`quarantine`). Nur Zählungen: kein Nachrichteninhalt. |
| `GET` | `/api/targets/{id}/pmg/health` | Ein Eintrag pro Knoten (System, Dienste, `signatures` mit `family`, `age_seconds` und `stale`, `expiring_certificates`, Updates, Abonnement), die `cluster`-Mitglieder mit ihrem Sync-Zustand, und `stopped_services` über die Knoten hinweg. |

## Synology

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/api/targets/{id}/synology` | Übersicht eines `synology`-Geräts aus der letzten Abfrage: `system` (Modell, DSM-Version, Uptime, Temperatur, CPU, Speicher), `volumes` (`name`, `fs_type`, `raid_type`, `status`, `severity`, `total_bytes`, `used_bytes`), `disks` (`model`, `serial`, `kind`, `ssd`, `status`, `smart_status`, `temperature_celsius`, `size_bytes`, `remaining_life_percent`, …) und `sampled_at`. `404`, wenn das Gerät nicht existiert, `400`, wenn es kein Synology ist. |
| `GET` | `/api/targets/{id}/synology/abb` | Active Backup for Business: `tasks`, und `devices` — ein Bericht pro geschütztem Gerät mit seinem gelernten Rhythmus in `assessment` (`state`, `last_success_s`, `typical_interval_s`, 30-Tage-Zählungen). |

## Heartbeats (Push-Monitore)

| Methode | Route | Auth | Zweck |
|---|---|---|---|
| `GET` oder `POST` | `/api/push/{token}` | keine (das Token) | Der Aufruf, den ein Job bei jedem Lauf macht. Optional `?status=up\|down&msg=…` (Uptime-Kuma-kompatibel). `204` leer bei Erfolg, `404` für ein unbekanntes Token, `400` für einen anderen `status`, `429` mit `Retry-After` über sechzig Aufrufe pro Minute pro Token. Siehe [Heartbeat](../devices/push.md). |
| `GET` | `/api/targets/{id}/push` | Sitzung | Der Monitor eines `push`-Geräts: `token`, `path` (`/api/push/<token>`), `last_seen_at`, `last_seen_age_secs`, `last_status`, `last_message`, `received_total`, `expected_interval_secs`, `grace_secs`, `settings_error`, `verdict` (`waiting`, `on_time`, `missed`, `reported_down`). Beim ersten Lesen erstellt. `400`, wenn das Gerät kein Heartbeat ist. |
| `POST` | `/api/targets/{id}/push/regenerate` | admin | Neues Token; die vorherige URL antwortet von da an mit `404`. Derselbe Body wie das Lesen. |

## Assistants (MCP)

Der eingebaute [Model-Context-Protocol](https://modelcontextprotocol.io)-
Server, der, mit dem ein Assistant spricht. Er authentifiziert mit
den obigen API-Tokens — dieselben Scopes, Ablauf, Netzwerke und
Ratenlimit — und nie mit einer Sitzung.

| Methode | Route | Auth | Zweck |
|---|---|---|---|
| `POST` | `/api/mcp` | API-Token | JSON-RPC 2.0 über den Streamable-HTTP-Transport, ein Request pro `POST`, in JSON beantwortet. Protokoll `2026-07-28` (zustandslos: jeder Request trägt seine Version und Client-Fähigkeiten in `_meta`, plus die Header `MCP-Protocol-Version`, `Mcp-Method` und `Mcp-Name`; `server/discover`), und `2025-11-25`, `2025-06-18` und `2025-03-26` über den `initialize`-Handshake. Keine Sitzung: Keine `Mcp-Session-Id` wird ausgestellt, eine gesendete wird ignoriert. Eine Notification erhält `202` und keinen Body. |
| `GET`, `DELETE` | `/api/mcp` | öffentlich | `405` mit `Allow: POST` — es gibt keinen Server-Sent-Stream und keine zu beendende Sitzung; der Body sagt nur, was dieser Endpunkt ist. |

`tools` ist die einzige Fähigkeit — keine Resources, keine Prompts,
kein OAuth. Von den 27 Werkzeugen kann ein `read`-Token die 15 nur
lesenden aufrufen (`get_status`, `list_devices`, `get_device`,
`list_alerts`, `alert_history`, `query_metrics`, `list_silences`,
`list_rules`, `list_device_types`, `list_agents`,
`list_containers`, `list_heartbeats`, `list_status_pages`,
`list_channels`, `list_packs`); die 12, die etwas ändern
(`silence_device`, `remove_silence`, `acknowledge_alert`,
`probe_device`, `set_device_enabled`, `set_rule_enabled`,
`add_device`, `discover_network`, `schedule_maintenance`,
`restart_container`, `test_channel`, `post_incident`) brauchen ein
`write`-Token. Die Werkzeuge rufen denselben Code wie die Web-UI
auf. Versionen, Header, Fehlercodes und die Sicherheitshinweise
stehen in
[Einen Assistant verbinden](../using/assistant.md#protocol-details).

## Agent-Dateien (außerhalb von `/api`)

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/install.sh` | Der Linux-, macOS- und FreeBSD-Installer. Öffentlich. |
| `GET` | `/install.ps1` | Der Windows-Installer. Öffentlich. |
| `GET` | `/download/{name}` | `dumbmonit-agent-linux-x86_64`, `dumbmonit-agent-linux-aarch64`, `dumbmonit-agent-freebsd-x86_64`, `dumbmonit-agent-windows-x86_64.exe`, aus `DUMBMONIT_AGENT_DIR` bedient; `{name}.sha256` gibt seine Prüfsumme im `sha256sum`-Format. `404`, falls die Datei fehlt. `dumbmonit-agent-macos-aarch64` und `dumbmonit-agent-macos-x86_64` (und ihre `.sha256`) antworten mit `307` zur gleichen Datei des neuesten GitHub-Releases. Öffentlich. |

Jeder andere Pfad wird von der Web-UI bedient, die Sie selbst bittet,
sich einzuloggen.
