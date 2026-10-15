# HTTP-API {#http-api}

Alles, was die Web-Oberfläche tut, läuft über `/api/*`, und dieselbe API steht Ihren eigenen
Skripten, Dashboards und Assistenten offen: Es gibt keine private Hälfte. Es ist schlichtes JSON
über HTTP, authentifiziert durch ein Sitzungscookie (die Web-Oberfläche) oder durch ein API-Token
(alles andere).

!!! warning "Alpha"
    DumbMonit ist eine Alpha, und seine API ebenso: Routen und Felder können sich noch ändern.
    Die API-Version (unten) ändert sich, sobald eine Route oder ein Feld entfernt oder umbenannt
    wird, sodass ein Client es erkennen kann.

## OpenAPI-Beschreibung {#openapi-description}

Jede Instanz beschreibt ihre eigene API in einem
[OpenAPI-3.1](https://spec.openapis.org/oas/v3.1.0)-Dokument:

```
GET /api/openapi.json
```

Es ist öffentlich — es enthält kein Geheimnis, keine Adresse und nichts über Ihre Monitore — und es
ist die erschöpfende Referenz: jede Route, die ein Client aufrufen kann, ihre Parameter,
Anfrage- und Antwortformen, Fehlerantworten und der benötigte Scope (die Erweiterung
`x-token-scope` an jeder Operation: `public`, `read`, `write` oder `none` für Routen, die ein
Token nie aufrufen kann). Die Tabellen weiter unten auf dieser Seite sind eine lesbare
Zusammenfassung desselben.

Absichtlich ausgelassen: das Agent-Protokoll (`/api/ingest`, `/api/agent/commands*`,
`/api/agent/relay*`), das nur der [Agent](../devices/agent.md) spricht; die Browser-Weiterleitungen
der SSO-Anmeldung (`/api/auth/oidc/start`, `/api/auth/oidc/callback`) und die von Spotify
(`/api/music/spotify/callback`); und die Agent-Downloads (`/install.sh`, `/install.ps1`,
`/download/*`).

Laden Sie es in ein beliebiges OpenAPI-Werkzeug:

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

Zwei Zugänge, auf jeder geschützten Route akzeptiert:

- **Sitzungscookie** — was die Web-Oberfläche verwendet. Melden Sie sich mit einem Passwort (oder
  SSO) an und senden Sie das HttpOnly-Cookie `dumbmonit_session` (SameSite Lax; `Secure` bei
  `DUMBMONIT_COOKIE_SECURE=1`; 30 Tage). Jede zustandsändernde Anfrage (`POST`, `PUT`, `DELETE`)
  muss außerdem `X-Requested-With: DumbMonit` tragen — der Anti-CSRF-Nachweis, den die Oberfläche
  automatisch hinzufügt; ohne ihn wird die Anfrage mit `403` abgelehnt.
- **API-Token** — `Authorization: Bearer dmt_…`, für Skripte, Dashboards und Assistenten. Erstellt
  unter Einstellungen → **API & Assistenten** (dieselben Tokens, die der
  [MCP-Server](../using/assistant.md) verwendet) und einmalig angezeigt. Es ist kein Cookie im
  Spiel, daher ist nie ein Header `X-Requested-With` nötig.

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

Trägt eine Anfrage ein Bearer-Token, wird das Cookie ignoriert: Ein widerrufenes Token wird auch
aus einem angemeldeten Browser abgelehnt, statt still auf die Sitzung zurückzufallen.

Agent-Enrollment-Tokens (`dmon_…`) sind etwas anderes: Sie funktionieren nur auf den Agent-Routen
(`/api/ingest`, `/api/agent/commands/*`, `/api/agent/relay*`). Sie sagen, dass ein Rechner sprechen
darf, nicht *welcher* Rechner: Auf diesen Routen muss ein gebundener Rechner zusätzlich sein
eigenes Bindungsgeheimnis in `X-DumbMonit-Agent-Secret` (`dmab_…`) vorlegen, das der Server bei der
Anmeldung einmalig ausgibt. Siehe [Bindung](../devices/agent.md#binding-one-machine-one-agent).

Die Anmeldung ist ratenbegrenzt: Nach fünf fehlgeschlagenen Versuchen wird jeder weitere mit `429`
und einer Verzögerung `Retry-After` abgelehnt, die sich von 30 s bis auf 5 Minuten verdoppelt.
Erraten Sie Passwörter nie in einer Schleife.

## Scopes {#scopes}

Ein Token hat einen von zwei Scopes. Einen dritten, mächtigeren gibt es nicht: Was ein
Administrator mit seinem eigenen Konto tut, bleibt außerhalb der Reichweite jedes Tokens.

| | `read`-Token | `write`-Token |
|---|---|---|
| Jedes `GET`, das ein *Betrachter*-Konto ausführen kann (Monitore, Alarme, Metriken, Regeln, Kanäle ohne ihre Geheimnisse, Statusseiten …) | ja | ja |
| `POST`, `PUT`, `DELETE`, die ein *Administrator* ausführen kann (Monitore, Regeln, Stummschaltungen, Kanäle, Statusseiten, Packs, Prüfungen, Discovery …) | `403` | ja |
| `/metrics`, `/federate`, `/prometheus/*` ([Scraping](metrics.md#scraping-dumbmonit)) | ja | ja |
| MCP-Lese-/Schreibwerkzeuge ([Assistenten](../using/assistant.md#what-the-assistant-can-call)) | nur Lesen | beide |
| Konten, Sitzungen, Zwei-Faktor, SSO, Audit-Log (`/api/auth/*`, `/api/users/*`) | `403` | `403` |
| API-Tokens und Agent-Enrollment-Tokens (`/api/tokens/*`, `/api/agent/tokens/*`) | `403` | `403` |
| Sicherung und Wiederherstellung (`/api/backup/*`) | `403` | `403` |

Ein `read`-Token auf einer Schreibroute erhält `403` mit einer Meldung, die das Token und den
fehlenden Scope nennt. Da kein Token Tokens auflisten, erstellen oder widerrufen kann, kann ein
Token nie ein anderes ausstellen — geschweige denn ein mächtigeres.

## Token-Sicherheit {#token-security}

- **Einmal angezeigt, gehasht gespeichert.** Ein Token ist `dmt_` gefolgt von 128 Zufallsbits. Der
  Server behält nur seinen SHA-256-Digest (und die ersten Zeichen, `dmt_1a2b3c4d`, um es in einer
  Liste zu erkennen); das Klartext-Token existiert nur in der Antwort auf seine Erstellung.
  Vorgelegte Tokens werden in konstanter Zeit verglichen.
- **Ablauf.** Wählen Sie beim Erstellen 30 Tage, 90 Tage, 1 Jahr oder nie. Ein abgelaufenes Token
  erhält `401` mit `The API token "<name>" expired on …` und
  `WWW-Authenticate: Bearer error="invalid_token"`; es bleibt in der Liste, als abgelaufen markiert,
  bis Sie es widerrufen.
- **Erlaubte Netzwerke.** Beschränken Sie ein Token optional auf eine Liste von Adressen oder
  CIDR-Bereichen (`192.168.1.0/24, 10.8.0.5`, höchstens 32 Einträge). Von anderswo erhält es `403`
  mit `The API token "<name>" cannot be used from <address>.` Die geprüfte Adresse ist die des
  TCP-Clients oder — nur wenn die Verbindung von einem in `DUMBMONIT_TRUSTED_PROXIES` aufgeführten
  Proxy kommt — die, die dieser Proxy in `X-Forwarded-For` meldet. Hinter einem nicht deklarierten
  Reverse-Proxy scheint jede Anfrage vom Proxy zu kommen: Deklarieren Sie ihn, sonst weist die
  Beschränkung alle ab.
- **Letzte Verwendung.** Die Tokenliste zeigt, wann jedes Token zuletzt verwendet wurde und von
  welcher Adresse (höchstens einmal pro Minute aktualisiert oder sobald sich die Adresse ändert):
  Ein Token, das niemand mehr nutzt, ist eines zum Widerrufen.
- **An seinen Ersteller gebunden.** Ein Token merkt sich das Konto, das es erstellt hat. Wird dieses
  Konto deaktiviert, werden seine Tokens abgelehnt (`401`); wird es zum Operator oder Betrachter
  herabgestuft, wirken seine `write`-Tokens wie `read`-Tokens; wird es gelöscht, werden seine
  Tokens widerrufen.
- **Widerruf** ist sofort: Der nächste Aufruf erhält `401`.
- **Protokolliert.** Jeder mit einem Token ausgeführte Schreibvorgang wird mit Name und ID des
  Tokens, der Methode, dem Pfad (ohne Query-String) und dem Status ins Serverlog geschrieben — nie
  das Token, nie der Body. Das Erstellen und Widerrufen eines Tokens wird im Audit-Log aufgezeichnet
  (Einstellungen → Sicherheit).

## Ratenlimits {#rate-limits}

Jedes Token darf 120 Aufrufe pro Minute machen, REST und MCP zusammen. Darüber hinaus werden
Aufrufe mit `429` und einem Header `Retry-After` (Sekunden) abgelehnt, bis die Minute vorbei ist.
Jede Antwort auf eine mit einem Token authentifizierte Anfrage sagt, wo Sie stehen:

| Header | Bedeutung |
|---|---|
| `RateLimit-Limit` | Pro Fenster erlaubte Aufrufe (120). |
| `RateLimit-Remaining` | Im aktuellen Fenster verbleibende Aufrufe. |
| `RateLimit-Reset` | Sekunden, bis das Fenster neu beginnt. |

Ein Grafana-Dashboard mit vielen Panels und kurzem Refresh kann das Limit erreichen: Geben Sie ihm
ein eigenes Token oder verlängern Sie den Refresh.

## Cross-Origin-Anfragen (CORS) {#cross-origin-requests-cors}

Eine von einem anderen Origin ausgelieferte Seite — Ihr eigenes Dashboard, Swagger UI, ein
browserbasierter MCP-Client — kann die API **mit einem Bearer-Token aufrufen, nie mit einer
Sitzung**:

- Eine Antwort trägt `Access-Control-Allow-Origin` nur, wenn die Anfrage
  `Authorization: Bearer dmt_…` trug (oder beim öffentlichen `GET /api/openapi.json`).
  `Access-Control-Allow-Credentials` wird nie gesendet, sodass ein Browser das Sitzungscookie nie an
  einen Cross-Origin-Aufruf hängt;
- ein Preflight (`OPTIONS`) auf `/api/*` gelingt nur, wenn die angekündigten Header `authorization`
  einschließen: `204`, Methoden `GET, POST, PUT, DELETE`, Header
  `authorization, content-type, mcp-protocol-version, mcp-method, mcp-name`, 10 Minuten gecacht.
  `X-Requested-With` gehört nicht dazu, sodass eine Cookie-Sitzung nie von einem anderen Origin aus
  schreiben kann;
- `Retry-After`, `WWW-Authenticate`, die Header `RateLimit-*` und `X-DumbMonit-Api-Version` sind
  für das aufrufende Skript lesbar.

`DUMBMONIT_API_CORS_ORIGINS` legt fest, welche Origins erlaubt sind:

| Wert | Wirkung |
|---|---|
| nicht gesetzt (Standard) | Jeder Origin (`Access-Control-Allow-Origin: *`). Sicher, da nur ein Bearer-Token etwas öffnet. |
| `https://grafana.example.com,https://dash.lan` | Nur diese Origins, zurückgespiegelt. |
| `off` | Gar keine CORS-Header: nur Seiten desselben Origins und Nicht-Browser-Clients. |

Dieselbe Liste entscheidet, welchen `Origin` der [MCP-Endpunkt](../using/assistant.md#security-notes)
akzeptiert.

```js
// From a page on another origin. Never ship a write token in a public page.
const response = await fetch('https://monit.example.com/api/alerts', {
  headers: { Authorization: 'Bearer dmt_…' }
});
if (!response.ok) throw new Error((await response.json()).error);
const alerts = await response.json();
console.log(response.headers.get('RateLimit-Remaining'), 'calls left this minute');
```

## Versionierung {#versioning}

Jede Antwort unter `/api` trägt die API-Version:

```
X-DumbMonit-Api-Version: 0.1
```

Derselbe Wert ist `info.version` im OpenAPI-Dokument, neben `x-server-version`, der Version des
Servers, der geantwortet hat. Solange die API `0.x` ist, ist sie nicht eingefroren; die Version
ändert sich, sobald eine Route oder ein Feld entfernt oder umbenannt wird. Das Hinzufügen einer
Route oder eines Felds ändert sie nicht: Ignorieren Sie Felder, die Sie nicht kennen.

## Fehler {#errors}

Jeder Fehler ist JSON: `{"error": "message"}`, für Menschen geschrieben.

| Status | Wann |
|---|---|
| `400` | Die Anfrage ist fehlerhaft geformt oder ein Wert ist ungültig; die Meldung nennt ihn. |
| `401` | Keine gültige Sitzung oder kein gültiges Token: fehlendes, unbekanntes, widerrufenes oder abgelaufenes Token (mit `WWW-Authenticate: Bearer`), oder ein Token, dessen Ersteller deaktiviert ist. |
| `403` | Authentifiziert, aber nicht erlaubt: ein Betrachter, ein Operator außerhalb der Alarmbearbeitung, oder ein `read`-Token auf einer Schreibroute, ein Token auf einer Konto-, Token- oder Sicherungsroute, ein Token, das aus einem Netzwerk verwendet wird, aus dem es nicht erlaubt ist, ein Cookie-Schreibvorgang ohne `X-Requested-With`, oder jeder Schreibvorgang auf der öffentlichen Demo (mit `"demo": true`). |
| `404` | Die ID existiert nicht. Ein Pfad unter `/api`, der auf keine Route passt, antwortet `404 {"error": "Unknown API route: …"}` statt mit dem HTML der Web-Oberfläche. |
| `405` | Die Methode existiert auf diesem Pfad nicht (zum Beispiel `GET /api/mcp`). |
| `409` | Ein Konflikt mit dem aktuellen Zustand (ein Duplikat, ein bereits laufender Befehl …). |
| `429` | Ratenbegrenzt, mit `Retry-After`. |
| `500` | Ein Serverfehler; die Details stehen im Serverlog, nie in der Antwort. |

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

Der [MCP-Endpunkt](../using/assistant.md#protocol-details) antwortet stattdessen in JSON-RPC, mit
eigenen Fehlercodes.

## Konten, Anmeldung und Tokens {#accounts-sign-in-and-tokens}

In den folgenden Tabellen bedeutet *Sitzung* ein Sitzungscookie **oder** ein API-Token; *Admin* eine
Administrator-Sitzung oder ein `write`-Token; *Operator* eine Operator- oder Administrator-Sitzung
oder ein `write`-Token; *nur Sitzung* ein Sitzungscookie, nie ein Token.

**Rollen.** Ein Konto `viewer` führt jedes `GET` aus. Ein `operator` führt dieselben Lesezugriffe
plus die auf dieser Seite als *Operator* markierten Schreibvorgänge zur Alarmbearbeitung aus (und
`x-roles: [admin, operator]` im OpenAPI-Dokument): bestätigen, zurückstellen (Wartungsfenster), eine
Regel für einen Monitor ignorieren, aufgelösten Verlauf löschen. Jeder andere Schreibvorgang ist
Admins vorbehalten und antwortet einem Operator mit `403`.

| Methode | Route | Auth | Zweck |
|---|---|---|---|
| `GET` | `/api/auth/status` | öffentlich | `{"configured": bool, "authenticated": bool, "user": …, "oidc": {"enabled", "provider_name", "login_url"}}`. `configured: false` bedeutet eine frische Instanz: Bis der erste Admin existiert, antwortet jede mit *Sitzung* markierte Route `401` — nur `status`, `setup`, `login` und `health` sind erreichbar. |
| `POST` | `/api/auth/setup` | öffentlich | `{"setup_code": "…", "username": "admin", "password": "…"}`. Erstellt den ersten Administrator auf einer frischen Instanz (mindestens 12 Zeichen); `username` darf fehlen und ist dann `admin`. `setup_code` ist der einmalige Code, der beim Start in den Serverlogs ausgegeben wird (oder `DUMBMONIT_SETUP_CODE`); Groß-/Kleinschreibung, Bindestriche und Leerzeichen werden ignoriert. `204`; öffnet keine Sitzung. `401` bei falschem Code, `429` nach zu vielen, `409`, sobald ein Konto existiert. |
| `POST` | `/api/auth/login` | öffentlich | `{"username": "…", "password": "…"}`. `204` mit `Set-Cookie`. `401` bei falschem Passwort, `429` bei Ratenbegrenzung. Hat das Konto Zwei-Faktor aktiviert, stattdessen `200` mit `{"totp_required": true, "pending": "…"}` — ein Ticket, das fünf Minuten lebt und nach fünf falschen Codes stirbt: Beenden Sie mit `/api/auth/login/totp`. |
| `POST` | `/api/auth/login/totp` | öffentlich | `{"pending": "…", "code": "123456"}` — das Ticket aus dem ersten Schritt und ein Code (oder ein Wiederherstellungscode). `204` mit dem Sitzungscookie. |
| `GET` | `/api/auth/oidc/start` | öffentlich | Leitet den Browser zum Identitätsanbieter weiter. |
| `GET` | `/api/auth/oidc/callback` | öffentlich | Rückkehr vom Anbieter (`code`, `state`): öffnet die Sitzung und leitet zur Oberfläche weiter. |
| `GET` | `/api/auth/me` | nur Sitzung | Das aktuelle Konto: `id`, `username`, `display_name`, `role` (`admin`, `operator`, `viewer`), `auth` (`password`, `oidc`), `disabled`, `totp_enabled`, `created_at`, `last_login_at`. |
| `POST` | `/api/auth/logout` | nur Sitzung | Beendet die Sitzung und löscht das Cookie. |
| `POST` | `/api/auth/password` | nur Sitzung | `{"current_password": "…", "new_password": "…"}`. Meldet jede andere Sitzung ab. |
| `GET` | `/api/auth/totp` | nur Sitzung | Zwei-Faktor-Zustand des aktuellen Kontos: `enabled`, `pending`, `recovery_codes_left`. |
| `POST` | `/api/auth/totp/enroll` | nur Sitzung | `{"password": "…"}`. Schlägt ein Geheimnis vor: `secret` (base32), `otpauth_uri`, `issuer`, `account`. Nichts wird durchgesetzt, bis es verifiziert ist. |
| `POST` | `/api/auth/totp/verify` | nur Sitzung | `{"code": "123456"}`. Bestätigt die Anmeldung und liefert `{"recovery_codes": […]}` — einmalig angezeigt. |
| `DELETE` | `/api/auth/totp` | nur Sitzung | `{"password": "…"}`. Entfernt den zweiten Faktor. `204`. |
| `GET` | `/api/auth/audit?limit=200` | Admin, nur Sitzung | Die letzten Sicherheitsereignisse: `id`, `at`, `actor`, `action` (`login`, `login.failed`, `password.changed`, `token.created`, `user.updated`, …), `subject`, `ip`. |
| `GET` | `/api/auth/oidc/config` | Admin, nur Sitzung | Die SSO-Einstellungen und woher sie kommen (Umgebung oder Datenbank); das Client-Secret wird nie zurückgegeben. |
| `PUT` | `/api/auth/oidc/config` | Admin, nur Sitzung | `{"issuer", "client_id", "client_secret", "provider_name", "scopes", "auto_create", "admin_groups", "groups_claim", "public_url"}`. Ein fehlendes oder leeres `client_secret` behält das gespeicherte; ein fehlendes `auto_create` bedeutet aus. |
| `DELETE` | `/api/auth/oidc/config` | Admin, nur Sitzung | Vergisst die gespeicherten SSO-Einstellungen (Umgebungsvariablen, falls vorhanden, gelten wieder). `204`. |
| `POST` | `/api/auth/oidc/test` | Admin, nur Sitzung | Ruft das Discovery-Dokument des Anbieters ab und meldet, was es gefunden hat. |
| `GET` | `/api/users` | Admin, nur Sitzung | Jedes Konto, in der Form von `/api/auth/me`. |
| `POST` | `/api/users` | Admin, nur Sitzung | `{"username", "display_name", "role", "password"}`. `password` darf nur fehlen, wenn SSO aktiviert ist. `201`. |
| `PUT` | `/api/users/{id}` | Admin, nur Sitzung | Eines von `display_name`, `role`, `disabled`, `password`; ein fehlendes Feld behält seinen Wert. Der letzte Administrator kann weder herabgestuft noch deaktiviert werden. |
| `DELETE` | `/api/users/{id}` | Admin, nur Sitzung | `204`. Nicht Sie selbst, nicht der letzte Administrator. |
| `DELETE` | `/api/users/{id}/totp` | Admin, nur Sitzung | Setzt den zweiten Faktor eines anderen Kontos zurück (ein ausgesperrter Kollege). `204`. |
| `GET` | `/api/tokens` | nur Sitzung | Jedes API-Token: `id`, `name`, `prefix`, `scope`, `created_at`, `created_by` (das Konto, das es erstellt hat), `expires_at` (`null`: nie), `expired`, `allowed_networks` (leer: überall), `last_used_at`, `last_used_ip`, `revoked_at`. Nie das Geheimnis. |
| `POST` | `/api/tokens` | Admin, nur Sitzung | `{"name": "Grafana", "scope": "read", "expires_in_days": 90, "allowed_networks": ["192.168.1.0/24"]}`. `name` ist erforderlich (höchstens 80 Zeichen); `scope` ist `read` (Standard) oder `write`; `expires_in_days` reicht von 1 bis 3650, `null` oder fehlend für ein Token, das nie abläuft; `allowed_networks` nimmt bis zu 32 Adressen oder CIDR-Bereiche, in Netzwerkform gespeichert (`192.168.1.7/24` wird zu `192.168.1.0/24`), leer oder fehlend für überall. `201` mit den Tokenfeldern plus `secret` (einmalig angezeigt). `400` nennt ein ungültiges Netzwerk. |
| `DELETE` | `/api/tokens/{id}` | Admin, nur Sitzung | Widerrufen. `204`; `404`, wenn unbekannt oder bereits widerrufen. |

Ein Token, wie aufgelistet:

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

## Antwort-Header {#response-headers}

Neben `X-DumbMonit-Api-Version` ([Versionierung](#versioning)), den Headern `RateLimit-*`
([Ratenlimits](#rate-limits)) und den CORS-Headern ([Cross-Origin-Anfragen](#cross-origin-requests-cors))
trägt jede Antwort `X-Content-Type-Options: nosniff`, `Referrer-Policy: same-origin` und eine
`Content-Security-Policy`. Die Richtlinie geht von `default-src 'none'` aus und öffnet nur, was die
Oberfläche wirklich verwendet — alles von DumbMonit selbst ausgeliefert:

```
default-src 'none'; style-src 'self' 'unsafe-inline'; img-src 'self' data:;
font-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'self';
script-src 'self' 'nonce-<per response>'; frame-ancestors 'none'
```

Es ist kein Drittanbieter-Origin darin, und es sollte nie einer darin sein: Die Schriften, die
Diagrammbibliothek und jedes Asset werden im Binary mitgeliefert. Die Inline-Skripte der Seite (das
vor dem ersten Zeichnen angewendete Theme, der Bootstrap von SvelteKit) werden durch einen für jede
Antwort neu erzeugten Nonce erlaubt, sodass `'unsafe-inline'` nie für Skripte gilt. Für Stile gilt
es, und es lässt sich nicht entfernen: Svelte setzt `style=`-Attribute auf Elementen, die weder ein
Nonce noch ein Hash abdecken kann.

Öffentliche Statusseiten sind zum Einbetten gemacht: `/s/<slug>` und seine kompakte Ansicht
`/s/<slug>/embed` — und nur diese beiden — erhalten dieselbe Richtlinie ohne `frame-ancestors`.
Alles andere, die Seiten zur Abonnementbestätigung und zur Abmeldung unter `/s/` eingeschlossen,
trägt außerdem `X-Frame-Options: DENY`.

Wenn Sie DumbMonit hinter einen Reverse-Proxy stellen, lassen Sie ihn keinen zweiten Header
`Content-Security-Policy` hinzufügen: Browser setzen die Schnittmenge aller durch, und ein
Proxy-Standard ohne den Nonce hinterlässt eine leere Seite.

## Zustand {#health}

| Methode | Route | Auth | Zweck |
|---|---|---|---|
| `GET` | `/api/health` | öffentlich | Serverversion und Zustand seiner Abhängigkeiten. Immer `200`: Eine ausgefallene Komponente steht im Body. |
| `GET` | `/api/update` | Sitzung oder Token | Laufende Version, neuestes Release, `update_available`; der Server fragt GitHub höchstens einmal täglich. `POST /api/update/check` sieht sofort nach, `PUT /api/update/settings` schaltet die Prüfung an oder aus. Siehe [Update](../install/update.md). |

```json
{"status":"ok","version":"0.1.0","database":{"ok":true},"victoria":{"ok":true,"embedded":true}}
```

`status` ist `degraded`, wenn eine Komponente ausfällt; diese Komponente trägt dann eine
Zeichenkette `error`. `victoria.embedded` ist `true`, wenn der Server sein eigenes VictoriaMetrics
betreibt, `false`, wenn `DUMBMONIT_VM_URL` auf ein externes zeigt.

## Sicherung und Wiederherstellung {#backup-and-restore}

| Methode | Route | Auth | Zweck |
|---|---|---|---|
| `GET` | `/api/backup` | Admin, nur Sitzung | Was ein Bundle enthalten würde (`contents`, eine Zeile pro Abschnitt mit seiner Zahl), die Formatversion des Bundles, wo das Instanzgeheimnis liegt (`secret_source`: `file` oder `environment`) und der Zustand der geplanten lokalen Sicherungen (`schedule`). |
| `POST` | `/api/backup` | Admin, nur Sitzung | `{"passphrase": "…", "include_account_secrets": false}`. Liefert das Bundle selbst als JSON-Datei: einen Klartext-Header (`format`, `version`, `created_at`, `summary`, `kdf`) und eine `payload`, mit AES-256-GCM unter einem per Argon2id aus der Passphrase abgeleiteten Schlüssel verschlüsselt. Mindestens 16 Zeichen, sonst `400`. |
| `POST` | `/api/backup/restore` | Admin, nur Sitzung | `{"bundle": {…}, "passphrase": "…", "apply": false}`. Probelauf, sofern `apply` nicht `true` ist; beide führen denselben Code aus, der Probelauf innerhalb einer Transaktion, die zurückgerollt wird. Antwortet mit dem Bericht: pro Abschnitt `created`, `updated`, `skipped` und `notes`, plus `warnings`. `400` bei falscher Passphrase, einer veränderten Datei oder einem in einer neueren Formatversion geschriebenen Bundle. |
| `POST` | `/api/backup/local` | Admin, nur Sitzung | Schreibt sofort eine lokale Sicherung im Stil der geplanten und antwortet mit dem neuen `schedule`. |

Ein Bundle trägt jede Zugangsdaten der Instanz, mit der Passphrase neu verschlüsselt, daher lehnen
diese Routen API-Tokens mit einer Erklärung ab: Nur ein in der Web-Oberfläche angemeldeter
Administrator erhält eines. Siehe [Sicherung und Wiederherstellung](../install/backup.md).

## Monitortypen {#device-types}

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/api/collectors` | Jeder Monitortyp, die dieser Server überwachen kann: `kind`, `label`, `summary`, `examples`, `credential_types`, `address_hint`, `default_port`, `setup` (`title`, `steps`, `warning`, `doc_url`) und `options` (`key`, `label`, `help`, `placeholder`, `default`, `required`, `input`, `choices`). Von einem [Integrationspaket](../packs/index.md) mitgebrachte Arten werden genauso aufgeführt, als `pack.<id>`. |

## Integrationspakete {#integration-packs}

Admin-Sitzung oder API-Token (Scope `write` für Änderungen). Siehe
[Integrationspakete](../packs/index.md).

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/api/packs` | Installierte Pakete: `id`, `version`, `label`, `summary`, `kind` (`null` für ein Paket nur mit SNMP-Profilen), `enabled`, `installed_at`, `sha256`, `metrics`, `rules` (uids), `snmp_profiles`, `targets` (Monitore, die es verwenden), `warnings` und `error`, wenn das gespeicherte Paket die Validierung nicht mehr besteht. |
| `GET` | `/api/packs/{id}` | Ein Paket. |
| `POST` | `/api/packs` | Installieren oder aktualisieren. Body: das YAML (`application/yaml`, `text/plain` …) oder `{"yaml": "…"}` in JSON. `201` bei Erstellung, `200` bei Aktualisierung oder Unverändert, mit `outcome` (`created`, `updated`, `unchanged`), `pack`, `rules_added` und `restart_required` (SNMP-Profile). `400` listet jeden Validierungsfehler auf. |
| `PUT` | `/api/packs/{id}/enable`, `/api/packs/{id}/disable` | Den Monitortyp des Pakets registrieren oder zurückziehen; Monitore, Verlauf und Regeln bleiben. |
| `DELETE` | `/api/packs/{id}` | Deinstallieren, samt seinen Regeln. `409`, solange Monitore es verwenden. |

## Ziele (Monitore) {#targets-devices}

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/api/targets` | Jeden Monitor auflisten. |
| `POST` | `/api/targets` | Eines erstellen. `201` mit dem Monitor. Die Profilerkennung startet bei SNMP im Hintergrund. |
| `GET` | `/api/targets/{id}` | Ein Monitor. |
| `PUT` | `/api/targets/{id}` | Es ersetzen. Das Weglassen von `credential` behält die gespeicherten; das Weglassen von `profile_id` behält das erkannte Profil (`""` löscht es). `enabled: false` zu setzen löscht die Alarme des Monitors ohne Benachrichtigung. |
| `DELETE` | `/api/targets/{id}` | `204`. Löscht die Alarme des Monitors ohne Benachrichtigung und seine Zeitreihen aus VictoriaMetrics (nach bestem Bemühen). |
| `POST` | `/api/targets/{id}/probe` | Jetzt prüfen: `{"sample_count": 42, "series": ["if_octets_in{host=\"Core switch\",ifname=\"eth0\"}", …]}` — die Serienschlüssel, wie der Collector sie erzeugt hat, ohne das Präfix `dumbmonit_`, das der Writer auf dem Weg zu VictoriaMetrics hinzufügt. |
| `POST` | `/api/targets/{id}/discover` | Profilerkennung erneut ausführen: `{"profile_id": "host-resources"}` oder `null`. |

Ein Monitor, wie zurückgegeben:

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

Das Geheimnis selbst wird nie zurückgegeben: `credential_kind` ist ein Label. Server-Zeitstempel
sind UTC ohne Suffix. `error_kind` klassifiziert `last_error`: `down` (der Monitor hat nicht
geantwortet) oder `config` (unsere Seite — Zugangsdaten, Adresse, Option), was die Oberfläche als
*Nicht erreichbar* oder *Falsch konfiguriert* zeigt; `null`, wenn die letzte Prüfung erfolgreich
war.

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

Formen der Zugangsdaten (`type`): `none`; `snmp_community` (`community`); `snmp_v3` (`username`,
optional `auth: {protocol, passphrase}` mit `md5`, `sha1`, `sha224`, `sha256`, `sha384` oder
`sha512`, optional `privacy: {protocol, passphrase}` mit `des`, `aes128`, `aes192` oder `aes256`,
optional `context`); `api_token` (`token` oder die beiden Hälften `token_id` und `secret`, die der
Server für Proxmox VE und PBS zu `user@realm!name=secret` zusammenfügt); `username_password`
(`username`, `password`). `interval_secs` ist standardmäßig 60 und darf nicht unter 10 gehen.
`name` hat höchstens 200 Zeichen und `address` höchstens 253 (`400` darüber hinaus). `parent_id`
muss einen existierenden Monitor nennen (`400 Parent device N not found.`). Typoptionen kommen unter
ihrem Schlüssel in `tags`.

`via_agent` nennt den [Relay-Agenten](../install/remote-site.md), der den Monitor aus seinem eigenen
Netzwerk statt vom Server prüft; `null` (der Standard) bedeutet, dass der Server es prüft. Es muss
ein Monitor vom Typ `agent` sein (sonst `400`), ein Monitor kann sich nicht selbst weiterleiten, und ein
`agent`-Monitor kann nicht weitergeleitet werden. Bei `PUT` behält ein *fehlendes* `via_agent` den
gespeicherten Wert, wie `profile_id` und `credential`; senden Sie `null`, um zur direkten Prüfung
zurückzukehren. `POST /api/targets/{id}/probe` und `/discover` auf einem weitergeleiteten Monitor
laufen über das Relay und warten auf dessen Antwort (`409`, wenn bereits eine Prüfung läuft,
`400 Timed out: relay agent …`, wenn es nicht antwortet).

## Discovery {#discovery}

| Methode | Route | Auth | Zweck |
|---|---|---|---|
| `POST` | `/api/discovery` | Admin | `{"cidr": "192.168.1.0/24", "community": "public", "port": 161, "timeout_ms": 1000}`. Durchsucht ein Netzwerk nach SNMP-Geräten. `cidr` ist erforderlich; `community` ist standardmäßig `public`; `timeout_ms` wird auf 100–10.000 begrenzt. Netzwerke mit mehr als 4096 Adressen werden abgelehnt. Der Scan ist ein `POST` mit JSON-Body, damit die Community nie in einer URL oder Logzeile erscheint. |

```json
{
  "scanned": 254,
  "devices": [
    {"address": "192.168.1.10", "sysname": "sw-core", "sysdescr": "…", "sysobjectid": "1.3.6.1.4.1.8072.3.2.10", "suggested_profile": "host-resources"}
  ]
}
```

## Anleitung zum ersten Start {#first-run-guide}

Die drei Schritte, die die Übersicht auf einer neuen Instanz zeigt. Der Zustand wird auf der
Instanz gehalten, nicht im Browser, sodass das Überspringen eines Schritts ihn überall überspringt.

| Methode | Route | Auth | Zweck |
|---|---|---|---|
| `GET` | `/api/onboarding` | Sitzung | Wo die Instanz steht. |
| `PUT` | `/api/onboarding` | Admin | `{"skipped": true}` legt die ganze Anleitung beiseite; `{"dismissed": ["test"]}` legt einzelne Schritte beiseite. Ein fehlendes Feld behält seinen gespeicherten Wert; eine andere Schritt-ID als `device`, `channel` oder `test` ist ein `400`. Liefert dasselbe Dokument wie `GET`. |

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

`has_target`, `has_channel` und `notification_confirmed` werden bei jedem Lesen neu berechnet: Sie
sind Fakten über die Instanz, keine von der Oberfläche gesetzten Flags. `notification_confirmed` ist
wahr, sobald tatsächlich eine Nachricht über irgendeinen Kanal hinausgegangen ist — eine
Testnachricht zählt, ein nur erstellter Kanal nicht.

`complete` rastet ein und wird nie wieder falsch. Der Server setzt es, wenn die drei Schritte
erledigt sind (getan oder beiseitegelegt), und auch beim allerersten Lesen einer Instanz, die
bereits einen Monitor und einen Kanal hatte — sodass ein Upgrade einer etablierten Instanz ihr nie eine
Anleitung zum ersten Start zeigt.

## Metriken {#metrics}

Beide Routen reichen an VictoriaMetrics weiter und liefern Serien in der Prometheus-Form:
`[{"metric": {labels…}, "values": [[timestamp_seconds, "value"], …]}]`.

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/api/metrics/query?query=…` | Sofortige MetricsQL-Abfrage. |
| `GET` | `/api/metrics/query_range?query=…&start=…&end=…&step=…` | Bereichsabfrage. `start` und `end` sind **Millisekunden** seit der Epoche; `step` ist in Sekunden und wird so erweitert, dass eine Serie nie 2.000 Punkte überschreitet. |

```bash
curl -b cookies.txt -G http://localhost:8080/api/metrics/query \
  --data-urlencode 'query=avg by (target, host) (dumbmonit_cpu_usage_percent)'
```

```json
[{"metric":{"host":"ThinkpadE14","target":"3"},"values":[[1757941205,"12.5"]]}]
```

## Scraping (außerhalb von `/api`) {#scraping-outside-api}

Für ein bestehendes Prometheus oder Grafana. Beide nehmen ein `read`-API-Token in
`Authorization: Bearer dmt_…` (ein Sitzungscookie funktioniert ebenfalls); ein Scraper sendet keinen
Header `X-Requested-With` und braucht keinen — diese Routen lesen nur. Siehe die
[Metrikreferenz](metrics.md#scraping-dumbmonit) für eine kopierfertige `scrape_config` und die
Grafana-Datenquelle.

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/metrics` | Zustand der Instanz selbst im Prometheus-Textformat: Scheduler, Prüfungen, Schreibvorgänge, Alarmierung, Benachrichtigungen, Agenten, Datenbank, VictoriaMetrics. Einige Dutzend Serien, nie eine pro Monitor. |
| `GET` | `/federate` | Die Messungen, im selben Format. `match[]` wählt Serien aus (wiederholbar, höchstens 10, standardmäßig `{__name__=~"dumbmonit_.*"}`), `max_lookback` erweitert das Fenster, in dem der letzte Punkt gesucht wird (standardmäßig `5m`). `400` über 8 MiB, mit dem, was einzuschränken ist. |
| `GET`, `POST` | `/prometheus/api/v1/{route}` | Die Lesehälfte der Prometheus-API, an den Speicher weitergereicht, sodass Grafana `http://…/prometheus` als Prometheus-Datenquelle verwenden kann. Nur `query`, `query_range`, `query_exemplars`, `series`, `labels`, `label/{name}/values`, `metadata` und `status/buildinfo`; alles andere ist `404`. |

`DUMBMONIT_METRICS_PUBLIC=true` liefert die drei ohne Token aus — nur hinter einer Firewall, da sie
jede Messung jedes Monitors tragen.

## Weitere Routen {#further-routes}

Oben nicht im Detail aufgeführte Routen; das OpenAPI-Dokument nennt ihre Parameter und
Antwortformen. *read* und *write* sind der benötigte Token-Scope; *none* bedeutet, dass kein Token
akzeptiert wird (nur Sitzung).

| Methode | Route | Scope | Zweck |
|---|---|---|---|
| `POST` | `/api/targets/reorder` | write | Monitore neu ordnen. |
| `GET` | `/api/targets/{id}/security` | read | Sicherheitswert, Note und Prüfungen eines Monitors. |
| `GET` | `/api/security/summary` | read | Sicherheitswert jedes bewerteten Monitors, der schlechteste zuerst. |
| `GET` | `/api/targets/{id}/webchange/pages` | read | Von einer Website-Überwachung verfolgte Seiten. |
| `GET` | `/api/targets/{id}/webchange/changes` | read | Auf einer überwachten Website gefundene Änderungen, neueste zuerst. |
| `GET` | `/api/targets/{id}/webchange/changes/{change_id}` | read | Eine Änderung, mit beiden Versionen und dem Zeilen-Diff. |
| `GET` | `/api/targets/{id}/webchange/snapshots/{snapshot_id}/screenshot` | read | Der Screenshot eines Snapshots. |
| `POST` | `/api/targets/{id}/webchange/check` | write | Eine überwachte Website jetzt prüfen. |
| `GET` | `/api/targets/{id}/opnsense/gateways`, `/traffic`, `/health` | read | OPNsense: Gateways und WAN-Adressen; Schnittstellen, Firewall und DHCP; System, Dienste, Tunnel und Firmware. |
| `GET` | `/api/targets/{id}/truenas/storage`, `/protection`, `/health` | read | TrueNAS: Pools, Datasets und Platten; Scrubs und Datensicherungs-Tasks; System, Alarme und Dienste. |
| `GET` | `/api/targets/{id}/ad`, `/ad/findings` | read | Active Directory: Domäne, Controller, privilegierte Gruppen und Befunde; die rohen Sicherheitsbefunde. |
| `GET` | `/api/targets/{id}/redfish` | read | Über den Management-Controller gelesene Serverhardware. |
| `GET` | `/api/targets/{id}/observability` | read | Zustand eines Log- oder Metrikservers. |
| `GET` | `/api/targets/{id}/backend` | read | Zustand von Redis, MongoDB, RabbitMQ oder CrowdSec. |
| `GET` | `/api/targets/{id}/proxy` | read | Zustand eines Reverse-Proxys oder einer Domainregistrierung. |
| `GET` | `/api/public/status/{slug}/badge.svg`, `/response.svg`, `/uptime.svg` | public | Badges für Gesamtstatus, mittlere Antwortzeit und Uptime einer Statusseite. |
| `GET` | `/api/public/status/{slug}/components/{key}/badge.svg`, `/uptime.svg`, `/response.svg` | public | Dieselben Badges für einen Dienst der Seite. |
| `GET` | `/api/public/status/{slug}/banner.js` | public | Skript des Status-Banners für Websites Dritter. |
| `GET` | `/api/public/status/{slug}/rss` | public | Vorfälle als RSS-Feed. |
| `GET` | `/api/music/now` | read | Was die Wände abspielen (Raummonitor). |
| `PUT`, `DELETE` | `/api/music/link` | write | Jede Wand spielt diesen Spotify-, Deezer- oder YouTube-Link; oder stoppt den eingebetteten Player. |
| `GET`, `DELETE` | `/api/music/spotify` | read, write | Das verbundene Spotify-Konto (nie ein Token); trennen und seine Tokens vergessen. |
| `POST` | `/api/music/spotify/authorize`, `/complete` | none | Die Spotify-Verbindung starten; eine Loopback-Verbindung abschließen. Nur Sitzung. |
| `GET` | `/api/music/spotify/token` | none | Ein kurzlebiges Spotify-Zugriffstoken für den Player der Wand. Nur Sitzung. |
| `GET`, `PUT` | `/api/music/speaker` | read, write | Der Spotify-Lautsprecher der Wand, wie ihn die Einstellungen zeigen; ihn umbenennen (Admin). |
| `POST` | `/api/music/speaker/report`, `/play` | none | Eine Wand meldet den Zustand ihres Lautsprechers; das Konto darauf abspielen. Nur Sitzung. |

## Alarme {#alerts}

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/api/alerts` | Aktive Alarme (ausstehend, auslösend, unterdrückt, kürzlich aufgelöst). Alarme gelöschter oder pausierter Monitore werden nie aufgeführt. |
| `GET` | `/api/alerts/history?since=2026-09-01T00:00:00Z&limit=200&dismissed=false` | Phasenübergänge. `since` ist RFC 3339, standardmäßig die letzten sieben Tage; `limit` muss positiv sein. `dismissed` (Standard `false`) schließt mit den beiden folgenden Routen gelöschte Einträge aus; durch ein Löschen wird nie etwas zerstört, nur aus der Standardansicht verborgen. |
| `POST` | `/api/alerts/history/{id}/dismiss` | Löscht einen Eintrag `resolved` aus der Standard-Verlaufsansicht. `204`. `404` bei unbekannter ID oder einem noch aktiven Übergang. Operator; auditiert. |
| `POST` | `/api/alerts/history/dismiss-resolved` | Löscht jeden noch nicht gelöschten Eintrag `resolved` auf einmal („Alle aufgelösten löschen“). Liefert `{"dismissed": <count>}`. Operator; auditiert. |
| `POST` | `/api/alerts/{fingerprint}/ack` | Bestätigen: `{"duration_secs": 14400, "note": "…"}` oder `{"until": "2026-09-22T18:00:00Z"}` (eines von beiden; keines bedeutet 4 Stunden, höchstens 30 Tage). `{"until": null}` hebt es auf. Liefert den Alarm. Operator; auditiert. |
| `DELETE` | `/api/alerts/{fingerprint}/ack` | Die Bestätigung aufheben. Liefert den Alarm. Operator. |

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

Ein Verlaufseintrag hat `id`, `fingerprint`, `rule_uid`, `target_id`, `from_phase`, `to_phase`,
`severity`, `value`, `notified`, `reason` (leer oder warum nichts gesendet wurde:
`learning: would have fired`, `suppressed: device 2 unreachable`, `maintenance window`,
`acknowledged by admin`, `device removed or disabled`), `at` und `dismissed` (aus der aktiven Ansicht
gelöscht; die Zeile selbst bleibt erhalten). `acked` ist wahr, solange `acked_until` in der Zukunft
liegt: Erinnerungen und Eskalationen pausieren, die Auflösung wird weiterhin benachrichtigt und hebt
die Bestätigung auf. Ein `target_id`, das nicht mehr existiert, zeigt die Oberfläche als „(gelöschter
Monitor)“.

### Regeln {#rules}

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/api/alerts/rules` | Jede Regel. |
| `POST` | `/api/alerts/rules` | Erstellen. `201`. |
| `PUT` | `/api/alerts/rules/{id}` | Aktualisieren. Eine integrierte Regel behält ihre Abfrage. |
| `DELETE` | `/api/alerts/rules/{id}` | `204`. Integrierte Regeln lassen sich nicht löschen. |
| `POST` | `/api/alerts/rules/{id}/enable` | `{"enabled": false}`. Liefert die Regel. |

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

Zum Erstellen sind mindestens `name` und `query` nötig; `kind` (`threshold`, `anomaly`, `predict`),
`operator`, `threshold`, `for_secs`, `severity` (`info`, `warning`, `critical`), `selector`
(`{"kind":"all"}`, `{"kind":"ids","ids":[…]}` oder `{"kind":"labels","labels":{…}}`), `channels`
(Kanal-IDs; leer bedeutet jeder aktivierte Kanal), `unit`, `repeat_secs`, `escalate_after_secs` und
`enabled` sind optional.

### Stummschaltungen (Wartungsfenster) {#silences-maintenance-windows}

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

Ein einmaliger Zeitplan ist `{"kind": "once", "starts_at": "2026-09-20T22:00:00Z",
"ends_at": "2026-09-21T02:00:00Z"}`. Ein monatlicher ist `{"kind": "monthly", "days": [1],
"nth_weekdays": [{"nth": 1, "weekday": 6}], "start_minute": 120, "duration_minutes": 120,
"timezone": "Europe/Paris"}` — `nth` ist 1 bis 5 oder `-1` für den letzten des Monats; ein Monat
ohne dieses Vorkommen wird übersprungen. Tage sind 0 = Montag … 6 = Sonntag (1–31 bei einem
monatlichen `days`); Minuten gelten seit lokaler Mitternacht (0–1439). Bei einem wiederkehrenden
Zeitplan hat `timezone` (ein IANA-Name) Vorrang vor `utc_offset_minutes` und hält ein Fenster über
Sommerzeitwechsel hinweg zur lokalen Uhrzeit; ein unbekannter Zonenname wird abgelehnt.

### Überschreibungen pro Monitor {#per-device-overrides}

Eine Regel lässt sich für einen Monitor anpassen, ohne die Regel selbst anzufassen. `{"enabled": false}`
ist das, was die Aktion **Ignorieren** der Oberfläche (am Alarm, auf der Monitorseite oder an der
Regel) schreibt: Der Monitor wird aus genau dieser Regel dauerhaft, aber umkehrbar entfernt, ohne die
Regel für jemand anderen zu deaktivieren.

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/api/alerts/overrides?target_id=4` | Jede Überschreibung oder die eines Monitors: `rule_uid`, `target_id`, `threshold`, `clear_threshold`, `enabled`. |
| `GET` | `/api/alerts/rules/{id}/overrides` | Die Überschreibungen einer Regel. |
| `PUT` | `/api/alerts/rules/{id}/overrides/{target_id}` | `{"threshold": 95, "clear_threshold": 90, "enabled": true}` — jedes Feld optional; `null` bedeutet „wie die Regel“. Erstellt oder ersetzt. Ein Operator darf nur `{"enabled": false}` senden (ignorieren); die Schwellenüberschreibungen des Monitors, falls vorhanden, bleiben erhalten. |
| `DELETE` | `/api/alerts/rules/{id}/overrides/{target_id}` | `204`. Für einen Operator hebt es nur das Ignorieren auf: Schwellenüberschreibungen bleiben. |
| `GET` | `/api/targets/{id}/rules` | The rules that watch a monitor: those whose query reads a metric it wrote in the last 24 h. `{"status": "ok" \| "no_data" \| "unavailable", "rules": [{id, uid, name, description, severity, enabled, ignored, custom_threshold, firing}]}`. |

## Benachrichtigungsrichtlinie {#notification-policy}

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/api/notify/policy` | Die globale Richtlinie: `batch_window_secs` (0 = sofort senden), `max_per_hour` pro Kanal (0 = unbegrenzt), `flap_events`, `flap_window_secs`, `flap_hold_secs` (0 = keine Flatter-Erkennung), `public_url` (Links in Nachrichten; leer = `DUMBMONIT_PUBLIC_URL`), `escalate_after_secs` und `escalate_channel` (0 / `null` = keine Eskalation). |
| `PUT` | `/api/notify/policy` | Dieselben Felder, jedes optional: Ein fehlendes Feld behält seinen Wert. `"escalate_channel": null` schaltet die Eskalation aus. Liefert die Richtlinie. |
| `POST` | `/api/notify/match-preview` | `{"matcher": {…}}` → welche Monitore ein Routing-Filter eines Kanals auswählt: `devices` (`id`, `name`, `kind`, `tags`, `matched`), `matched`, `total`, `rules`, `excluded_rules`. |

## Benachrichtigungskanäle {#notification-channels}

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/api/notify/kinds` | Jeder Kanaltyp mit seinen Feldern `settings` und `secrets` (`key`, `label`, `required`, `input`, `help`, `placeholder`, `options`, `shape`, `default`) und einer `doc_url` wie `https://dumbmonit.readthedocs.io/en/latest/notifications/#discord`. |
| `GET` | `/api/notify/channels` | Jeder Kanal: `id`, `name`, `kind`, `enabled`, `settings`, `has_secret`, `last_error`, `last_sent_at`, `policy` (`min_severity`, `notify_resolved`, `min_interval_secs`, `quiet_hours`, `matcher`). Geheimnisse werden nie zurückgegeben. |
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

Die genauen Schlüssel pro Art kommen aus `/api/notify/kinds` und sind unter
[Benachrichtigungskanäle](../notifications.md) dokumentiert.

### Web-Push-Geräte {#web-push-devices}

Die Geräte des **angemeldeten Kontos** (nur Sitzung, Betrachter eingeschlossen; API-Tokens erhalten
`401`). Ein Kanal `webpush` sendet an sie; siehe
[Push-Benachrichtigungen (PWA)](../using/push-notifications.md).

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/api/webpush` | `{"public_key": "…", "devices": […]}`: der öffentliche VAPID-Schlüssel (base64url, der `applicationServerKey`, mit dem abonniert wird) und die Geräte des Kontos — `id`, `device` (`Firefox on Android`), `push_service` (Host), `fingerprint` (erste 16 Hex-Zeichen der SHA-256 des Endpunkts), `created_at`, `last_success_at`, `last_error`. Der Endpunkt, die Browserschlüssel und der private Schlüssel werden nie zurückgegeben. |
| `POST` | `/api/webpush/subscriptions` | `PushSubscription.toJSON()`: `{"endpoint": "https://…", "keys": {"p256dh": "…", "auth": "…"}}`. `201` mit dem Gerät. Der Endpunkt muss HTTPS auf einer öffentlichen Adresse sein: Private, Loopback- und Link-Local-Adressen erhalten `400`. Derselbe Endpunkt erneut ersetzt seine Zeile. |
| `DELETE` | `/api/webpush/subscriptions/{id}` | `204`; `404` für ein Gerät eines anderen Kontos. |
| `POST` | `/api/webpush/test` | `{}` oder `{"id": 3}`. Sendet eine Testbenachrichtigung: `{"ok", "delivered", "failed", "removed", "message"}`. Mit `404`/`410` beantwortete Abonnements werden entfernt. |

## E-Mail-Berichte {#email-reports}

Nur Administratoren, `GET` eingeschlossen (die Antwort listet Empfänger auf). Bearer-Tokens brauchen
für Änderungen den Scope `write`. Siehe [E-Mail-Berichte](../using/reports.md).

| Methode | Route | Auth | Zweck |
|---|---|---|---|
| `GET` | `/api/reports/schedules` | Admin | Jeder Bericht: `id`, `name`, `enabled`, `frequency`, `weekday` (0 ist Montag), `day_of_month`, `hour`, `timezone`, `recipients`, `channel_id`, `last_sent_at`, `last_error`, `next_run_at`. |
| `POST` | `/api/reports/schedules` | Admin | Dieselben schreibbaren Felder; jedes hat standardmäßig wöchentlich, Montag, 8:00, `UTC`. `recipients` enthält höchstens 20 Adressen und ist zum Aktivieren des Berichts erforderlich; `channel_id` muss ein Kanal `smtp` oder `null` sein. `201`; `409` über 10 Berichte hinaus. |
| `GET` | `/api/reports/schedules/{id}` | Admin | Ein Bericht. |
| `PUT` | `/api/reports/schedules/{id}` | Admin | Vollständiger Ersatz. |
| `DELETE` | `/api/reports/schedules/{id}` | Admin | `204`. |
| `POST` | `/api/reports/schedules/{id}/send-test` | Admin | Sendet eine Vorschau per E-Mail an die gespeicherten Empfänger: `{"sent": 2, "failed": 0}`. Berührt den Zeitplan nicht. `400` ohne nutzbaren E-Mail-Kanal; `409` innerhalb von 30 Sekunden nach der vorherigen Vorschau. |

## Statusseiten und Vorfälle {#status-pages-and-incidents}

| Methode | Route | Auth | Zweck |
|---|---|---|---|
| `GET` | `/api/status-pages` | Sitzung | Jede Seite mit ihren Einträgen: `page` (`id`, `slug`, `title`, `description`, `published`, `theme`, `show_uptime_days`, `created_at`, `updated_at`) und `items` (`id`, `page_id`, `target_id`, `label`, `group_name`, `position`). |
| `POST` | `/api/status-pages` | Admin | `{"title", "slug", "description", "published", "theme", "show_uptime_days"}`. `title` ist erforderlich (höchstens 120 Zeichen); `slug` (`^[a-z0-9-]{2,40}$`) wird aus dem Titel abgeleitet, wenn er fehlt; `theme` ist `auto` (Standard), `light` oder `dark`; `show_uptime_days` reicht von 7 bis 90 (standardmäßig 90). `201`; `409` bei bereits vergebenem Slug. |
| `GET` | `/api/status-pages/{id}` | Sitzung | Eine Seite mit ihren Einträgen. |
| `PUT` | `/api/status-pages/{id}` | Admin | Dieselben Felder, aber ein vollständiger Ersatz statt eines Patches: `title` ist erforderlich, und jedes fehlende Feld geht auf seinen Standard zurück (Slug neu aus dem Titel abgeleitet, leere Beschreibung, `published: false`, `theme: auto`, 90 Tage Uptime). |
| `DELETE` | `/api/status-pages/{id}` | Admin | `204`. Die öffentliche URL antwortet nicht mehr. |
| `POST`, `PUT` | `/api/status-pages`, `/api/status-pages/{id}` — Aussehen und Abonnements | Admin | Akzeptieren außerdem `accent` (`default`, `blue`, `teal`, `violet`, `rose`, `amber`), `scenes` (Szenen-IDs aus der Liste des Editors — Städte, die neuen und die antiken Weltwunder, z. B. `venice`, `paris`, `giza`, `tajmahal`, keine Duplikate; `[]` für keine; bei der Erstellung weggelassen: `["paris"]`), `simple` (Boolean, Standard `false`: Die öffentliche Seite zeigt keine Szene, kein Maskottchen und keine Animation und folgt der Hell-/Dunkel-Präferenz des Besuchers), `scene_rotation` (`visit`, `1m`, `10m` oder `1h`), `footer_text` (Klartext, 280 Zeichen), `homepage_url` (`http(s)://` oder leer), `subscribe_channel_id` (ein Kanal `smtp` oder `null`, um das E-Mail-Abonnement auszuschalten) und `domain` (der öffentliche Hostname der Seite, wie `status.example.com`, oder `null`/`""` für keinen — siehe [Eigene Domain](../using/status-pages.md#custom-domain)). `domain` wird in Kleinbuchstaben umgewandelt und verliert ein Präfix `https://` oder einen abschließenden Schrägstrich; ein Port, Pfad oder eine IP-Adresse ist `400`, die Adresse von DumbMonit selbst (seine öffentliche URL oder der Host, auf dem die Anfrage einging) ist `400`, und ein Name, den eine andere Seite verwendet, ist `409`. Anders als die übrigen Felder behalten diese acht ihren gespeicherten Wert, wenn sie bei einem `PUT` fehlen. Die Seitenansicht liefert sie mit `logo_type` (`image/png`, `image/jpeg`, `image/webp` oder `null`). |
| `GET`, `PUT`, `DELETE` | `/api/status-pages/{id}/logo` | Sitzung / Admin | Das Logo der Seite. `PUT` nimmt `{"data": "<base64 or data: URL>"}`: PNG, JPEG oder WebP, am Inhalt erkannt, höchstens 256 KiB, alles andere `400`. `204`. `GET` ist die Vorschau des Editors (Entwürfe eingeschlossen). |
| `GET` | `/api/status-pages/{id}/subscribers` | Sitzung | E-Mail-Abonnenten: `id`, `page_id`, `email`, `confirmed_at` (`null`, solange ausstehend), `created_at`. Ihre Tokens verlassen den Server nie. |
| `DELETE` | `/api/status-pages/{id}/subscribers/{subscriber}` | Admin | Entfernt eine Adresse. `204`. |
| `PUT` | `/api/status-pages/{id}/items` | Admin | `[{"target_id": 4, "label": "NAS", "group_name": "Storage"}, …]` — die vollständige geordnete Liste der auf der Seite gezeigten Monitore. |
| `GET` | `/api/incidents` | Sitzung | Jeder Vorfall mit seinen Updates: `incident` (`id`, `page_id`, `title`, `kind`, `status`, `severity`, `starts_at`, `ends_at`, `created_at`, `updated_at`) und `updates`. |
| `POST` | `/api/incidents` | Admin | `{"title", "kind", "status", "severity", "page_id", "starts_at", "ends_at", "body"}` — `body` ist das erste Update. `201`. |
| `PUT` | `/api/incidents/{id}` | Admin | Dieselben Felder; ein fehlendes Feld behält seinen Wert. |
| `DELETE` | `/api/incidents/{id}` | Admin | `204`. |
| `GET` | `/api/incidents/{id}/updates` | Sitzung | Die Zeitleiste: `id`, `incident_id`, `status`, `body`, `created_at`. |
| `POST` | `/api/incidents/{id}/updates` | Admin | `{"status": "monitoring", "body": "…"}`. Hängt ein Update an und setzt den Vorfall auf `status`. `201`. |
| `GET` | `/api/public/status/{slug}` | öffentlich | Das JSON-Dokument, aus dem eine veröffentlichte Seite gebaut wird (ebenso `badge.svg` und `rss` unter demselben Pfad). In `page` ist `show_uptime_days` das konfigurierte Maximum und `history_days` das tatsächlich gezeigte Fenster (das Alter der Daten der Seite, von 7 Tagen bis zu diesem Maximum), und `simple` spiegelt den einfachen Modus. Siehe [Statusseiten](../using/status-pages.md). Auf der eigenen Domain einer Seite (`Host` gleich ihrer `domain`) antworten nur die Routen `/api/public/status/{slug}/…` dieser Seite; jede andere Route dieser Referenz ist dort `404`. |
| `GET` | `/api/public/status/{slug}/uptime.svg`, `/response.svg` | öffentlich | Seiten-Badges: mittlere Uptime über `?days=` `1`, `7`, `30` (Standard) oder `90` — `400` über den Verlauf der Seite hinaus — und mittlere Antwortzeit. SVG, `Cache-Control: public, max-age=60`. |
| `GET` | `/api/public/status/{slug}/components/{key}/badge.svg`, `/uptime.svg`, `/response.svg` | öffentlich | Dieselben drei Badges für einen Dienst; `key` ist der `key` des Dienstes im öffentlichen Dokument (sein Label in URL-Form). `404` bei unbekanntem Schlüssel. |
| `GET` | `/api/public/status/{slug}/logo` | öffentlich | Das Logo der Seite mit seinem Bildtyp, einen Tag cachebar (die `logo_url` des Dokuments trägt eine Version). `404` ohne eines. |
| `POST` | `/api/public/status/{slug}/subscribe` | öffentlich | `{"email": "…"}`. Sendet einen Bestätigungslink über den SMTP-Kanal der Seite. Immer `202` mit derselben Meldung, egal in welchem Zustand die Adresse ist; `400` bei ungültiger Adresse; `404`, wenn die Seite kein E-Mail-Abonnement anbietet; `409`, wenn die Seite keine weiteren Abonnenten aufnehmen kann; `429` mit `Retry-After` über fünf Anfragen pro Client und insgesamt sechzig pro 15 Minuten hinaus. |
| `POST` | `/api/public/status/{slug}/confirm?token=…` | öffentlich | Bestätigt ein Abonnement (der Link der Bestätigungs-E-Mail öffnet `/s/{slug}/confirm`, dessen Schaltfläche hierher postet). `404` bei unbekanntem oder abgelaufenem Token. |
| `POST` | `/api/public/status/{slug}/unsubscribe?token=…` | öffentlich | Entfernt ein Abonnement; der Body wird ignoriert, es ist also auch das One-Click-Ziel von `List-Unsubscribe` nach RFC 8058. Immer `200`. |

## Agent-Tokens und Ingest {#agent-tokens-and-ingest}

| Methode | Route | Auth | Zweck |
|---|---|---|---|
| `GET` | `/api/agent/tokens` | nur Sitzung | Jedes Token: `id`, `name`, `prefix`, `created_at`, `last_used_at`, `revoked_at`, `max_uses` (`null` für ein Flotten-Token), `uses`, `expires_at`. |
| `POST` | `/api/agent/tokens` | Admin, nur Sitzung | `{"name": "Home fleet", "base_url": "http://server:8080", "reusable": false, "max_uses": null, "expires_in_days": null}`. `201` mit den Tokenfeldern plus `secret` (einmalig angezeigt), `install_linux` und `install_windows`. `base_url` ist die URL, die Agenten verwenden werden; sie ist standardmäßig die Lauschadresse. Ohne `reusable` ist das Token **einmalig verwendbar**: Es meldet einen Rechner an und keinen weiteren. Mit `reusable` begrenzt `max_uses` die Anmeldungen (`null`: kein Limit). `expires_in_days` stoppt *Anmeldungen* nach so vielen Tagen; bereits angemeldete Rechner melden weiter. `400` bei einer Zahl unter 1. |
| `DELETE` | `/api/agent/tokens/{id}` | Admin, nur Sitzung | Widerrufen. `204`; `404`, wenn unbekannt oder bereits widerrufen. |
| `POST` | `/api/ingest` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Empfängt einen Batch von Messwerten von einem Agenten (Bodies bis 16 MB). Nicht zum manuellen Aufruf gedacht. Der Header trägt das Bindungsgeheimnis des Rechners; ein Agent, der keines hat, lässt ihn weg, womit er um eines bittet. Die Antwort ergänzt `agent_secret` (das Geheimnis, genau einmal zurückgegeben, wenn der Rechner gebunden wird) und `bound`. `403`, wenn der Rechner an eine andere Agent-Installation gebunden ist oder das Token nicht mehr anmelden kann — die Meldung sagt, was zutrifft und was zu tun ist. |
| `GET` | `/api/agent/relay?key=…&wait=N` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | An einen Relay-Agenten (`relay: true`) delegierte Prüfungen. Bis zu `wait` Sekunden (höchstens 25) gehalten, wenn nichts aussteht. Jedes Element ist ein Befehl der Art `probe`, dessen `args` das Ziel, seine entschlüsselten Zugangsdaten, `timeout_secs` und `discover` tragen. Nie auf die Platte geschrieben. |
| `POST` | `/api/agent/relay/{id}?key=…` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Ergebnis einer delegierten Prüfung: `{"duration_ms", "error", "samples", "profile_id"}` (Bodies bis 16 MB). `204`; `404`, wenn die Prüfung abgelaufen ist oder zu einem anderen Agenten gehört. |
| `GET` | `/api/agent/commands?key=…&wait=N` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Für einen Agenten eingereihte Befehle (Container-Neustart oder -Update), bis zu `wait` Sekunden gehalten, wenn nichts aussteht. Nicht zum manuellen Aufruf gedacht. `key` allein beweist nichts — er ist öffentlich —, daher muss ein gebundener Rechner zusätzlich sein Bindungsgeheimnis vorlegen; sonst `403`, was verhindert, dass ein Rechner die Befehle eines anderen nimmt. |
| `POST` | `/api/agent/commands/{id}?key=…` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Fortschritt und Ergebnis eines Befehls, vom Agenten gemeldet. `result` wird auf die letzten 4 KB gekürzt, mit `[truncated by the server, beginning dropped]` am Anfang, wenn das geschieht. |
| `GET` | `/api/relays` | Sitzung | Jeder Agent-Monitor als mögliches Relay: `id`, `name`, `site`, `relay` (als `relay: true` deklariert), `last_seen_at`, `relayed` (darüber erreichte Monitore). Relays zuerst, dann nach Name. |

```json
{
  "id": 1, "name": "Home fleet", "prefix": "dmon_ab12",
  "created_at": "2026-09-15 13:00:00", "last_used_at": null, "revoked_at": null,
  "secret": "dmon_…",
  "install_linux": "curl -sSL http://server:8080/install.sh | sh -s -- --token=dmon_… --url=http://server:8080",
  "install_windows": "& ([scriptblock]::Create((irm http://server:8080/install.ps1))) -Token dmon_… -Url http://server:8080"
}
```

## Container und Befehle (Agent-Monitore) {#containers-and-commands-agent-devices}

Aktionen auf den Containern eines Rechners, auf dem der Agent läuft. Jede Route antwortet `404`,
wenn der Monitor kein Ziel vom Typ `agent` ist.

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/api/targets/{id}/agent` | Der Rechner, wie sein Agent ihn zuletzt beschrieben hat: `hostname`, `os`, `os_version`, `arch`, `agent_version`, `commands_supported`, `relay`, `site`, `relayed` (über diesen Agenten erreichte Monitore), `last_seen_at`, `binding`, `bound`, `bound_at`, `rebind_until`. `404`, bis ein Agent gemeldet hat. `commands_supported` ist nur `true`, wenn der Agent erklärt hat, dass er Befehle abruft (`commands: true`, der Standard aktueller Agenten); ein älterer Agent oder einer mit `commands: false` ergibt `false`. `binding` ist `bound`, `pending` (das Binary kann gebunden werden und wird es bei seinem nächsten Batch) oder `unsupported` (ein Agent, der älter als die Bindung ist: neu installieren). |
| `POST` | `/api/targets/{id}/agent/rebind` | Admin. Öffnet ein einstündiges Fenster, in dem sich dieser Rechner mit einem gültigen Enrollment-Token erneut binden kann — der Weg zurück, nachdem eine Neuinstallation das Geheimnis des Agenten mitgenommen hat. `{"rebind_until", "minutes"}`. Das Fenster schließt sich, sobald es genutzt wird, und das alte Geheimnis funktioniert nicht mehr. `404`, bis ein Agent gemeldet hat. |
| `GET` | `/api/targets/{id}/containers` | Jeder Container aus dem letzten Batch: `name`, `image`, `up`, `health`, `restart_count`, `uptime_seconds`, `image_age_seconds`, `update_available`, `policy`, `last_command`. |
| `PUT` | `/api/targets/{id}/containers/{name}/policy` | `{"auto_restart": bool, "auto_update": bool, "prune_old_image": bool, "only_in_maintenance": bool}`. Jedes Feld ist optional: Ein fehlendes Feld behält seinen gespeicherten Wert. Liefert die vollständige Richtlinie. |
| `POST` | `/api/targets/{id}/containers/{name}/restart` | Einen Neustart einreihen. `201` mit dem Befehl. `404`, wenn `name` nicht im Inventar des Agenten steht; `409`, wenn derselbe Befehl bereits eingereiht ist oder läuft oder wenn der Agent keine Befehle ausführen kann (siehe `commands_supported`). |
| `POST` | `/api/targets/{id}/containers/{name}/update` | Ein Update einreihen; optional `{"prune": bool}` (standardmäßig die Richtlinie). Dieselben Statuscodes wie beim Neustart. |
| `GET` | `/api/targets/{id}/commands` | Die letzten 20 Befehle, neueste zuerst: `id`, `kind` (`container.restart`, `container.update`), `args`, `status`, `requested_by` (ein Benutzer oder `policy`), `created_at`, `started_at`, `finished_at`, `result`. |
| `DELETE` | `/api/targets/{id}/commands/{command_id}` | Einen eingereihten Befehl abbrechen. `204`; `409`, sobald der Agent ihn aufgenommen hat oder wenn er bereits abgeschlossen ist; `404`, wenn er nicht zu diesem Monitor gehört. |

`status` geht `queued → running → done | failed`. Zwei weitere Endzustände existieren: `cancelled` (eine
Person hat den Befehl zurückgezogen) und `expired` (der Server hat nach zehn Minuten aufgegeben, weil
kein Agent ihn abgeholt hat — Agent gestoppt, zu alt für den Befehlskanal oder mit `commands: false`
installiert). Der Server lässt veraltete Befehle jede Minute von selbst verfallen; ein eingereihter
Befehl blockiert einen neuen nie länger als das.

## Proxmox-VE-Gäste {#proxmox-ve-guests}

Die Zeilen des Panels **Gäste** eines Proxmox-VE-Monitors, aus der letzten Prüfung im Metrikspeicher
zusammengesetzt (vier Sofortabfragen, vom Hypervisor wird nichts abgefragt). `404`, wenn der Monitor
nicht existiert, `400`, wenn er kein Ziel vom Typ `proxmox` ist.

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/api/targets/{id}/proxmox/guests` | Ein Objekt pro VM oder Container, nach Knoten, dann VMID sortiert: `vmid`, `name`, `node`, `kind` (`qemu`, `lxc`), `status` (`running`, `stopped`, `paused`, `suspended`, `template`, `unknown`), `cpu_percent` (der zugewiesenen Kerne), `cpu_count`, `memory_used_bytes`, `memory_total_bytes`, `memory_percent`, `balloon_bytes`, `disk_used_bytes`, `disk_total_bytes`, `disk_percent`, `agent` (`true` Gast-Agent hat geantwortet, `false` aktiviert, aber still, `null` keiner), `network_in_bps`, `network_out_bps`, `disk_read_bps`, `disk_write_bps`, `uptime_seconds`, `last_backup_age_seconds`, `ha_state`, `pool` (sein Ressourcenpool), `lock` (nur solange eine Sperre gehalten wird: `backup`, `migrate` …), `os` und `ip` (von innen im Gast gesehen, stündlich aktualisiert). Jeder unbekannte Wert ist `null` — eine VM ohne Gast-Agent hat `disk_total_bytes`, aber `disk_used_bytes: null`; ein gestoppter Gast behält seine Größen und verliert seine Messungen. |

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

## Proxmox-VE-Knoten und Ceph {#proxmox-ve-nodes-and-ceph}

Die Abschnitte **Knoten** und **Ceph** derselben Monitorseite, aus derselben letzten Prüfung gelesen.
Dieselben Statuscodes wie oben.

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/api/targets/{id}/proxmox/nodes` | `{"nodes": […], "fencing_state", "fencing_armed"}` — der Zustand des HA-Watchdogs (`armed`, `standby` …) und ob er einen verlorenen Knoten wirklich isolieren würde, dann ein Objekt pro Knoten, nach Name sortiert: `name`, `up`, `cpu_percent`, `cpu_iowait_percent`, `memory_percent`, `ksm_shared_bytes`, `rootfs_percent`, `uptime_seconds`, `version` (das auf diesem Knoten installierte Release von `pve-manager`), `services_down` (Unit-Namen der gestoppten oder fehlgeschlagenen Daemons), `interfaces_offline` (beim Booten zu startende Schnittstellen, die nicht aktiv sind), `thin_pools` (`name`, `vg`, `used_percent`, `metadata_used_percent`, `size_bytes`) und `volume_groups` (`name`, `used_percent`, `size_bytes`). Ein Knoten, der nicht mehr antwortet, behält seinen Eintrag mit `up: false`. |
| `GET` | `/api/targets/{id}/proxmox/ceph` | `available` (`false`, wenn der Cluster kein Ceph hat — die Oberfläche zeichnet dann gar keinen Abschnitt), `health` (0 OK, 1 WARN, 2 ERR, 3 unbekannt), `health_status`, `bytes_used`, `bytes_total`, `used_percent`, `osds_total`, `osds_up`, `osds_in`, `osds` (`name`, `host`, `device_class`, `up`, `in`, `used_percent`, `used_bytes`, `total_bytes`, `apply_latency_ms`, `commit_latency_ms`), `pools` (`name`, `used_percent`, `used_bytes`, `size`, `min_size`, `pg_num`, `pg_num_optimal`, `autoscale`), `filesystems` (CephFS-Namen), `flags` (derzeit gesetzte OSD-Flags, etwa `noout`) und `muted_checks` (stummgeschaltete Health-Checks, die `HEALTH_OK` nicht mehr erwähnt). |

## Proxmox Backup Server {#proxmox-backup-server}

Die Panels eines Monitors `pbs`, aus der letzten Prüfung gelesen (vom Server wird nur das Tasklog
abgefragt). `404`, wenn der Monitor nicht existiert, `400`, wenn er kein Ziel vom Typ `pbs` ist.

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/api/targets/{id}/pbs/calendar?days=30&offset=120` | `{"probed_at", "days", "offset_minutes", "groups": […]}`: ein Eintrag pro Backup-Gruppe (`datastore`, `namespace`, `backup_type`, `backup_id`, `name`, `count`, `last_time`, `last_size`, `last_verified`, `last_success`, `last_failure`, `retention`) mit einem Array `days` aus `{date, state, runs, snapshot}`. `offset` ist der UTC-Offset des Betrachters in Minuten, damit Tage um lokale Mitternacht geschnitten werden. |
| `GET` | `/api/targets/{id}/pbs/failures?days=30` | Fehlgeschlagene Tasks: `upid`, `worker_type`, `kind`, `worker_id`, `datastore`, `start`, `end`, `error`. |
| `GET` | `/api/targets/{id}/pbs/jobs` | `{"probed_at", "jobs": […]}`: Sync-, Verifizierungs-, Prune- und Garbage-Collection-Jobs mit ihrem Zeitplan und letzten Ergebnis. |
| `GET` | `/api/targets/{id}/pbs/health` | Datastores (Nutzung, geschätztes Voll-Datum), Platten (SMART, Wearout) und ZFS-Pools. |
| `GET` | `/api/targets/{id}/pbs/tasks/{upid}/log?lines=200` | Das Log eines Tasks, vom Server abgerufen: `{"upid", "lines": […]}`; `lines` begrenzt, wie viele zurückgegeben werden. |
| `GET` | `/api/targets/{id}/pbs/disks/smart?disk=/dev/sda` | SMART-Attribute einer Platte; `disk` ist erforderlich (`400` ohne). |
| `GET` | `/api/targets/{id}/pdm/remotes` | Proxmox Datacenter Manager: Summen des Bestands (`estate`) und eine Zeile pro föderierter Instanz (`id`, `kind`, `reachable`, `error`, `version`, `version_behind`, Knoten- und Gästezahlen, Speicher und Storage, `subscription`, `last_collection`, `tasks_failed`), nicht erreichbare zuerst. |
| `GET` | `/api/targets/{id}/pdm/failures?days=14` | Im gesamten Bestand fehlgeschlagene Tasks: `upid`, `remote`, `worker_type`, `kind`, `worker_id`, `node`, `start`, `end`, `error`. |
| `GET` | `/api/targets/{id}/pdm/health` | Der Konsolen-Host: CPU, Speicher, Root-Dateisystem, Laufzeit, Zertifikate, ausstehende Updates und Subscription. |
| `GET` | `/api/targets/{id}/pmg/queues` | Proxmox Mail Gateway: die vier Postfix-Queues in Lesereihenfolge (`queue`, `messages`, `domains`, `oldest_age_seconds`, `top_domains`, `stuck`), plus `total_messages` und ein gatewayweites `stuck`. `oldest_age_seconds` ist eine Untergrenze: qshape meldet Altersklassen. |
| `GET` | `/api/targets/{id}/pmg/traffic` | Die heutigen Summen (`mail`), die jüngste Verkehrskurve (`recent`), das Spam-Score-Histogramm, die heute abgefangenen Viren und die Quarantäne-Zahlen (`quarantine`). Nur Zahlen: kein Nachrichteninhalt. |
| `GET` | `/api/targets/{id}/pmg/health` | Ein Eintrag pro Knoten (System, Dienste, `signatures` mit `family`, `age_seconds` und `stale`, `expiring_certificates`, Updates, Subscription), die Mitglieder `cluster` mit ihrem Sync-Zustand und `stopped_services` über Knoten hinweg. |

## Synology {#synology}

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/api/targets/{id}/synology` | Übersicht eines Monitors `synology` aus der letzten Prüfung: `system` (Modell, DSM-Version, Laufzeit, Temperatur, CPU, Speicher), `volumes` (`name`, `fs_type`, `raid_type`, `status`, `severity`, `total_bytes`, `used_bytes`), `disks` (`model`, `serial`, `kind`, `ssd`, `status`, `smart_status`, `temperature_celsius`, `size_bytes`, `remaining_life_percent`, …) und `sampled_at`. `404`, wenn der Monitor nicht existiert, `400`, wenn es kein Synology ist. |
| `GET` | `/api/targets/{id}/synology/abb` | Active Backup for Business: `tasks` und `devices` — ein Bericht pro geschütztem Gerät mit seinem gelernten Rhythmus in `assessment` (`state`, `last_success_s`, `typical_interval_s`, 30-Tage-Zahlen). |

## Heartbeats (Push-Monitore) {#heartbeats-push-monitors}

| Methode | Route | Auth | Zweck |
|---|---|---|---|
| `GET` oder `POST` | `/api/push/{token}` | keine (das Token) | Der Aufruf, den ein Job bei jedem Lauf macht. Optional `?status=up\|down&msg=…` (Uptime-Kuma-kompatibel). `204` leer bei Erfolg, `404` bei unbekanntem Token, `400` bei einem anderen `status`, `429` mit `Retry-After` über sechzig Aufrufe pro Minute und Token hinaus. Siehe [Heartbeat](../devices/push.md). |
| `GET` | `/api/targets/{id}/push` | Sitzung | Der Heartbeat-Zustand eines Monitors `push`: `token`, `path` (`/api/push/<token>`), `last_seen_at`, `last_seen_age_secs`, `last_status`, `last_message`, `received_total`, `expected_interval_secs`, `grace_secs`, `settings_error`, `verdict` (`waiting`, `on_time`, `missed`, `reported_down`). Beim ersten Lesen erstellt. `400`, wenn der Monitor kein Heartbeat ist. |
| `POST` | `/api/targets/{id}/push/regenerate` | Admin | Neues Token; die vorherige URL antwortet ab da `404`. Derselbe Body wie beim Lesen. |

## Assistenten (MCP) {#assistants-mcp}

Der eingebaute [Model-Context-Protocol](https://modelcontextprotocol.io)-Server, mit dem ein
Assistent spricht. Er authentifiziert sich mit den obigen API-Tokens — dieselben Scopes, Abläufe,
Netzwerke und Ratenlimits — und nie mit einer Sitzung.

| Methode | Route | Auth | Zweck |
|---|---|---|---|
| `POST` | `/api/mcp` | API-Token | JSON-RPC 2.0 über den Streamable-HTTP-Transport, eine Anfrage pro `POST`, in JSON beantwortet. Protokoll `2026-07-28` (zustandslos: Jede Anfrage trägt ihre Version und Client-Fähigkeiten in `_meta`, plus die Header `MCP-Protocol-Version`, `Mcp-Method` und `Mcp-Name`; `server/discover`) sowie `2025-11-25`, `2025-06-18` und `2025-03-26` über den `initialize`-Handshake. Keine Sitzung: Es wird keine `Mcp-Session-Id` ausgegeben, eine gesendete wird ignoriert. Eine Benachrichtigung erhält `202` und keinen Body. |
| `GET`, `DELETE` | `/api/mcp` | öffentlich | `405` mit `Allow: POST` — es gibt keinen vom Server gesendeten Stream und keine zu beendende Sitzung; der Body sagt nur, was dieser Endpunkt ist. |

`tools` ist die einzige Fähigkeit — keine Ressourcen, keine Prompts, kein OAuth. Von den 27
Werkzeugen kann ein `read`-Token die 15 aufrufen, die nur lesen (`get_status`, `list_devices`,
`get_device`, `list_alerts`, `alert_history`, `query_metrics`, `list_silences`, `list_rules`,
`list_device_types`, `list_agents`, `list_containers`, `list_heartbeats`, `list_status_pages`,
`list_channels`, `list_packs`); die 12, die etwas ändern (`silence_device`, `remove_silence`,
`acknowledge_alert`, `probe_device`, `set_device_enabled`, `set_rule_enabled`, `add_device`,
`discover_network`, `schedule_maintenance`, `restart_container`, `test_channel`, `post_incident`),
brauchen ein `write`-Token. Die Werkzeuge rufen denselben Code auf wie die Web-Oberfläche. Versionen,
Header, Fehlercodes und die Sicherheitshinweise stehen unter
[Einen Assistenten verbinden](../using/assistant.md#protocol-details).

## Agent-Dateien (außerhalb von `/api`) {#agent-files-outside-api}

| Methode | Route | Zweck |
|---|---|---|
| `GET` | `/install.sh` | Der Installer für Linux, macOS und FreeBSD. Öffentlich. |
| `GET` | `/install.ps1` | Der Windows-Installer. Öffentlich. |
| `GET` | `/download/{name}` | `dumbmonit-agent-linux-x86_64`, `dumbmonit-agent-linux-aarch64`, `dumbmonit-agent-freebsd-x86_64`, `dumbmonit-agent-windows-x86_64.exe`, aus `DUMBMONIT_AGENT_DIR` ausgeliefert; `{name}.sha256` liefert seine Prüfsumme im Format von `sha256sum`. `404`, wenn die Datei fehlt. `dumbmonit-agent-macos-aarch64` und `dumbmonit-agent-macos-x86_64` (und ihre `.sha256`) antworten `307` auf dieselbe Datei im neuesten GitHub-Release. Öffentlich. |

Jeder andere Pfad wird von der Web-Oberfläche ausgeliefert, die Sie selbst zur Anmeldung auffordert.
