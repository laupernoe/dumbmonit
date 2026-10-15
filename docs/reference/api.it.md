# API HTTP

Tutto ciò che fa l'interfaccia web passa da `/api/*`, e la stessa API è aperta ai
tuoi script, dashboard e assistenti: non esiste una metà privata. È semplice JSON
su HTTP, autenticato da un cookie di sessione (l'interfaccia web) o da un token
API (tutto il resto).

!!! warning "Alpha"
    DumbMonit è in alpha e lo è anche la sua API: rotte e campi possono ancora
    cambiare. La versione dell'API (più sotto) cambia ogni volta che una rotta o un
    campo viene rimosso o rinominato, così un client può accorgersene.

## Descrizione OpenAPI

Ogni istanza descrive la propria API in un documento
[OpenAPI 3.1](https://spec.openapis.org/oas/v3.1.0):

```
GET /api/openapi.json
```

È pubblico — non contiene alcun segreto, alcun indirizzo e nulla sui tuoi
dispositivi — ed è il riferimento esaustivo: ogni rotta che un client può chiamare,
i suoi parametri, le forme di richiesta e risposta, le risposte di errore e
l'ambito che richiede (l'estensione `x-token-scope` su ogni operazione: `public`,
`read`, `write`, oppure `none` per le rotte che un token non può mai chiamare). Le
tabelle più avanti in questa pagina sono un riepilogo leggibile della stessa cosa.

Lasciati fuori di proposito: il protocollo dell'agent (`/api/ingest`,
`/api/agent/commands*`, `/api/agent/relay*`), che parla solo l'[agent](../devices/agent.md);
i reindirizzamenti del browser per l'accesso SSO (`/api/auth/oidc/start`,
`/api/auth/oidc/callback`) e quello di Spotify (`/api/music/spotify/callback`); e i
download dell'agent (`/install.sh`, `/install.ps1`, `/download/*`).

Caricalo in qualsiasi strumento OpenAPI:

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

Postman e Insomnia lo importano dallo stesso URL.

## Autenticazione {#authentication}

Due modi per entrare, accettati su ogni rotta protetta:

- **Cookie di sessione** — ciò che usa l'interfaccia web. Accedi con una password
  (o SSO) e invia il cookie HttpOnly `dumbmonit_session` (SameSite Lax; `Secure`
  quando `DUMBMONIT_COOKIE_SECURE=1`; 30 giorni). Ogni richiesta che modifica lo
  stato (`POST`, `PUT`, `DELETE`) deve inoltre portare `X-Requested-With: DumbMonit`
  — la prova anti-CSRF che l'interfaccia aggiunge automaticamente; senza di essa la
  richiesta viene rifiutata con `403`.
- **Token API** — `Authorization: Bearer dmt_…`, per script, dashboard e assistenti.
  Creato in Impostazioni → **API e assistenti** (gli stessi token usati dal
  [server MCP](../using/assistant.md)) e mostrato una sola volta. Non è coinvolto
  alcun cookie, quindi non serve mai l'header `X-Requested-With`.

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

Quando una richiesta porta un bearer token, il cookie viene ignorato: un token
revocato viene rifiutato anche da un browser con accesso effettuato, invece di
ripiegare silenziosamente sulla sessione.

I token di registrazione degli agent (`dmon_…`) sono una cosa diversa: funzionano
solo sulle rotte dell'agent (`/api/ingest`, `/api/agent/commands/*`,
`/api/agent/relay*`). Dicono che una macchina può parlare, non *quale* macchina: su
quelle rotte una macchina associata deve inoltre presentare il proprio segreto di
associazione in `X-DumbMonit-Agent-Secret` (`dmab_…`), che il server consegna una
sola volta alla registrazione. Vedi
[Associazione](../devices/agent.md#binding-one-machine-one-agent).

L'accesso è limitato in frequenza: dopo cinque tentativi falliti, ogni tentativo
successivo viene rifiutato con `429` e un ritardo `Retry-After` che raddoppia da
30 s fino a 5 minuti. Non indovinare mai le password in un ciclo.

## Ambiti

Un token ha uno di due ambiti. Non ce n'è un terzo, più potente: ciò che un
amministratore fa con il proprio account resta fuori dalla portata di ogni token.

| | token `read` | token `write` |
|---|---|---|
| Ogni `GET` che un account *osservatore* può fare (dispositivi, avvisi, metriche, regole, canali senza i loro segreti, pagine di stato…) | sì | sì |
| `POST`, `PUT`, `DELETE` che un *amministratore* può fare (dispositivi, regole, silenziamenti, canali, pagine di stato, pacchetti, probe, rilevamento…) | `403` | sì |
| `/metrics`, `/federate`, `/prometheus/*` ([scraping](metrics.md#scraping-dumbmonit)) | sì | sì |
| Strumenti MCP di lettura / scrittura ([assistenti](../using/assistant.md#what-the-assistant-can-call)) | solo lettura | entrambi |
| Account, sessioni, due fattori, SSO, registro di audit (`/api/auth/*`, `/api/users/*`) | `403` | `403` |
| Token API e token di registrazione degli agent (`/api/tokens/*`, `/api/agent/tokens/*`) | `403` | `403` |
| Backup e ripristino (`/api/backup/*`) | `403` | `403` |

Un token `read` su una rotta di scrittura riceve `403` con un messaggio che nomina
il token e l'ambito mancante. Poiché nessun token può elencare, creare o revocare
token, un token non può mai crearne un altro — tanto meno uno più potente.

## Sicurezza dei token

- **Mostrato una sola volta, memorizzato con hash.** Un token è `dmt_` seguito da
  128 bit casuali. Il server conserva solo il suo digest SHA-256 (e i primi
  caratteri, `dmt_1a2b3c4d`, per riconoscerlo in un elenco); il token in chiaro
  esiste solo nella risposta alla sua creazione. I token presentati sono confrontati
  in tempo costante.
- **Scadenza.** Scegli 30 giorni, 90 giorni, 1 anno o mai alla creazione. Un token
  scaduto riceve `401` con `The API token "<name>" expired on …` e
  `WWW-Authenticate: Bearer error="invalid_token"`; resta nell'elenco, marcato come
  scaduto, finché non lo revochi.
- **Reti consentite.** Facoltativamente, limita un token a un elenco di indirizzi o
  intervalli CIDR (`192.168.1.0/24, 10.8.0.5`, al massimo 32 voci). Da qualsiasi
  altro posto riceve `403` con `The API token "<name>" cannot be used from <address>.`
  L'indirizzo controllato è quello del client TCP, oppure — solo quando la
  connessione arriva da un proxy elencato in `DUMBMONIT_TRUSTED_PROXIES` — quello che
  quel proxy riporta in `X-Forwarded-For`. Dietro un reverse proxy che non hai
  dichiarato, ogni richiesta sembra provenire dal proxy: dichiaralo, o la
  restrizione rifiuterà tutti.
- **Ultimo utilizzo.** L'elenco dei token mostra quando ciascun token è stato usato
  l'ultima volta e da quale indirizzo (aggiornato al massimo una volta al minuto, o
  non appena l'indirizzo cambia): un token che nessuno usa più è un token da
  revocare.
- **Legato al suo creatore.** Un token ricorda l'account che lo ha creato. Se quell'account
  è disabilitato, i suoi token vengono rifiutati (`401`); se viene retrocesso a
  operatore o osservatore, i suoi token `write` agiscono come token `read`; se viene
  eliminato, i suoi token vengono revocati.
- **La revoca** è immediata: la chiamata successiva riceve `401`.
- **Registrato nei log.** Ogni scrittura effettuata con un token viene scritta nel
  log del server con il nome e l'id del token, il metodo, il percorso (senza query
  string) e lo stato — mai il token, mai il corpo. La creazione e la revoca di un
  token sono registrate nel registro di audit (Impostazioni → Sicurezza).

## Limiti di frequenza {#rate-limits}

Ogni token può effettuare 120 chiamate al minuto, REST e MCP insieme. Oltre, le
chiamate vengono rifiutate con `429` e un header `Retry-After` (secondi) fino al
termine del minuto. Ogni risposta a una richiesta autenticata con token indica a che
punto sei:

| Header | Significato |
|---|---|
| `RateLimit-Limit` | Chiamate consentite per finestra (120). |
| `RateLimit-Remaining` | Chiamate rimaste nella finestra corrente. |
| `RateLimit-Reset` | Secondi prima che la finestra ricominci. |

Una dashboard Grafana con molti pannelli e un aggiornamento breve può raggiungere il
limite: dalle un token tutto suo, oppure allunga l'aggiornamento.

## Richieste cross-origin (CORS) {#cross-origin-requests-cors}

Una pagina servita da un'altra origine — la tua dashboard, Swagger UI, un client MCP
nel browser — può chiamare l'API **con un bearer token, mai con una sessione**:

- una risposta porta `Access-Control-Allow-Origin` solo quando la richiesta portava
  `Authorization: Bearer dmt_…` (o per il pubblico `GET /api/openapi.json`).
  `Access-Control-Allow-Credentials` non viene mai inviato, quindi un browser non
  allega mai il cookie di sessione a una chiamata cross-origin;
- un preflight (`OPTIONS`) su `/api/*` riesce solo quando gli header annunciati
  includono `authorization`: `204`, metodi `GET, POST, PUT, DELETE`, header
  `authorization, content-type, mcp-protocol-version, mcp-method, mcp-name`, in cache
  per 10 minuti. `X-Requested-With` non è fra essi, quindi una sessione con cookie
  non può mai effettuare una scrittura da un'altra origine;
- `Retry-After`, `WWW-Authenticate`, gli header `RateLimit-*` e
  `X-DumbMonit-Api-Version` sono leggibili dallo script chiamante.

`DUMBMONIT_API_CORS_ORIGINS` stabilisce quali origini sono consentite:

| Valore | Effetto |
|---|---|
| non impostato (predefinito) | Qualsiasi origine (`Access-Control-Allow-Origin: *`). Sicuro, poiché solo un bearer token apre qualcosa. |
| `https://grafana.example.com,https://dash.lan` | Solo queste origini, restituite così come sono. |
| `off` | Nessun header CORS: solo pagine della stessa origine e client non browser. |

La stessa lista decide quale `Origin` accetta l'[endpoint MCP](../using/assistant.md#security-notes).

```js
// From a page on another origin. Never ship a write token in a public page.
const response = await fetch('https://monit.example.com/api/alerts', {
  headers: { Authorization: 'Bearer dmt_…' }
});
if (!response.ok) throw new Error((await response.json()).error);
const alerts = await response.json();
console.log(response.headers.get('RateLimit-Remaining'), 'calls left this minute');
```

## Versionamento {#versioning}

Ogni risposta sotto `/api` porta la versione dell'API:

```
X-DumbMonit-Api-Version: 0.1
```

Lo stesso valore è `info.version` nel documento OpenAPI, accanto a
`x-server-version`, la versione del server che ha risposto. Finché l'API è `0.x`
non è congelata; la versione cambia ogni volta che una rotta o un campo viene
rimosso o rinominato. Aggiungere una rotta o un campo non la cambia: ignora i campi
che non conosci.

## Errori

Ogni errore è JSON: `{"error": "message"}`, scritto per una persona.

| Stato | Quando |
|---|---|
| `400` | La richiesta è malformata o un valore non è valido; il messaggio lo indica. |
| `401` | Nessuna sessione o token valido: token mancante, sconosciuto, revocato o scaduto (con `WWW-Authenticate: Bearer`), oppure un token il cui creatore è disabilitato. |
| `403` | Autenticato, ma non autorizzato: un osservatore, un operatore al di fuori della gestione degli avvisi, o un token `read` su una rotta di scrittura, un token su una rotta di account, token o backup, un token usato da una rete da cui non è consentito, una scrittura con cookie senza `X-Requested-With`, o qualsiasi scrittura sulla demo pubblica (con `"demo": true`). |
| `404` | L'id non esiste. Un percorso sotto `/api` che non corrisponde ad alcuna rotta risponde `404 {"error": "Unknown API route: …"}` anziché l'HTML dell'interfaccia web. |
| `405` | Il metodo non esiste su quel percorso (`GET /api/mcp`, per esempio). |
| `409` | Un conflitto con lo stato corrente (un duplicato, un comando già in esecuzione…). |
| `429` | Limite di frequenza raggiunto, con `Retry-After`. |
| `500` | Un guasto del server; i dettagli sono nel log del server, mai nella risposta. |

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

L'[endpoint MCP](../using/assistant.md#protocol-details) risponde invece in
JSON-RPC, con i propri codici di errore.

## Account, accesso e token

Nelle tabelle seguenti, *sessione* significa un cookie di sessione **oppure** un
token API; *admin* significa una sessione di amministratore o un token `write`;
*operatore* significa una sessione di operatore o amministratore, o un token
`write`; *solo sessione* significa un cookie di sessione, mai un token.

**Ruoli.** Un account `viewer` effettua ogni `GET`. Un `operator` effettua le stesse
letture più le scritture di gestione degli avvisi marcate *operatore* in questa
pagina (e `x-roles: [admin, operator]` nel documento OpenAPI): presa in carico,
posticipo (finestre di manutenzione), ignorare una regola per un dispositivo,
cancellare lo storico risolto. Ogni altra scrittura è riservata agli amministratori
e risponde `403` a un operatore.

| Metodo | Rotta | Autenticazione | Scopo |
|---|---|---|---|
| `GET` | `/api/auth/status` | pubblica | `{"configured": bool, "authenticated": bool, "user": …, "oidc": {"enabled", "provider_name", "login_url"}}`. `configured: false` indica un'istanza nuova: finché non esiste il primo amministratore, ogni rotta marcata *sessione* risponde `401` — sono raggiungibili solo `status`, `setup`, `login` e `health`. |
| `POST` | `/api/auth/setup` | pubblica | `{"setup_code": "…", "username": "admin", "password": "…"}`. Crea il primo amministratore su un'istanza nuova (almeno 12 caratteri); `username` può essere omesso e vale allora `admin`. `setup_code` è il codice monouso stampato nei log del server all'avvio (o `DUMBMONIT_SETUP_CODE`); maiuscole, trattini e spazi vengono ignorati. `204`; non apre una sessione. `401` per un codice errato, `429` dopo troppi tentativi, `409` quando esiste già un account. |
| `POST` | `/api/auth/login` | pubblica | `{"username": "…", "password": "…"}`. `204` con `Set-Cookie`. `401` con password errata, `429` quando il limite di frequenza è raggiunto. Quando l'account ha i due fattori attivi, risponde invece `200` con `{"totp_required": true, "pending": "…"}` — un ticket che dura cinque minuti e muore dopo cinque codici errati: termina con `/api/auth/login/totp`. |
| `POST` | `/api/auth/login/totp` | pubblica | `{"pending": "…", "code": "123456"}` — il ticket del primo passaggio e un codice (o un codice di recupero). `204` con il cookie di sessione. |
| `GET` | `/api/auth/oidc/start` | pubblica | Reindirizza il browser al provider di identità. |
| `GET` | `/api/auth/oidc/callback` | pubblica | Ritorno dal provider (`code`, `state`): apre la sessione e reindirizza all'interfaccia. |
| `GET` | `/api/auth/me` | solo sessione | L'account corrente: `id`, `username`, `display_name`, `role` (`admin`, `operator`, `viewer`), `auth` (`password`, `oidc`), `disabled`, `totp_enabled`, `created_at`, `last_login_at`. |
| `POST` | `/api/auth/logout` | solo sessione | Termina la sessione e cancella il cookie. |
| `POST` | `/api/auth/password` | solo sessione | `{"current_password": "…", "new_password": "…"}`. Disconnette ogni altra sessione. |
| `GET` | `/api/auth/totp` | solo sessione | Stato dei due fattori dell'account corrente: `enabled`, `pending`, `recovery_codes_left`. |
| `POST` | `/api/auth/totp/enroll` | solo sessione | `{"password": "…"}`. Propone un segreto: `secret` (base32), `otpauth_uri`, `issuer`, `account`. Nulla viene imposto finché non è verificato. |
| `POST` | `/api/auth/totp/verify` | solo sessione | `{"code": "123456"}`. Conferma la registrazione e restituisce `{"recovery_codes": […]}` — mostrati una sola volta. |
| `DELETE` | `/api/auth/totp` | solo sessione | `{"password": "…"}`. Rimuove il secondo fattore. `204`. |
| `GET` | `/api/auth/audit?limit=200` | admin, solo sessione | Gli ultimi eventi di sicurezza: `id`, `at`, `actor`, `action` (`login`, `login.failed`, `password.changed`, `token.created`, `user.updated`, …), `subject`, `ip`. |
| `GET` | `/api/auth/oidc/config` | admin, solo sessione | Le impostazioni SSO e la loro origine (ambiente o database); il client secret non viene mai restituito. |
| `PUT` | `/api/auth/oidc/config` | admin, solo sessione | `{"issuer", "client_id", "client_secret", "provider_name", "scopes", "auto_create", "admin_groups", "groups_claim", "public_url"}`. Un `client_secret` assente o vuoto mantiene quello memorizzato; un `auto_create` assente significa disattivato. |
| `DELETE` | `/api/auth/oidc/config` | admin, solo sessione | Dimentica le impostazioni SSO memorizzate (le variabili d'ambiente, se presenti, tornano ad applicarsi). `204`. |
| `POST` | `/api/auth/oidc/test` | admin, solo sessione | Scarica il documento di discovery del provider e riporta ciò che ha trovato. |
| `GET` | `/api/users` | admin, solo sessione | Ogni account, nella forma di `/api/auth/me`. |
| `POST` | `/api/users` | admin, solo sessione | `{"username", "display_name", "role", "password"}`. `password` può essere omessa solo quando SSO è abilitato. `201`. |
| `PUT` | `/api/users/{id}` | admin, solo sessione | Uno qualsiasi tra `display_name`, `role`, `disabled`, `password`; un campo omesso mantiene il proprio valore. Non può retrocedere né disabilitare l'ultimo amministratore. |
| `DELETE` | `/api/users/{id}` | admin, solo sessione | `204`. Non se stessi, non l'ultimo amministratore. |
| `DELETE` | `/api/users/{id}/totp` | admin, solo sessione | Reimposta il secondo fattore di un altro account (un collega bloccato fuori). `204`. |
| `GET` | `/api/tokens` | solo sessione | Ogni token API: `id`, `name`, `prefix`, `scope`, `created_at`, `created_by` (l'account che lo ha creato), `expires_at` (`null`: mai), `expired`, `allowed_networks` (vuoto: ovunque), `last_used_at`, `last_used_ip`, `revoked_at`. Mai il segreto. |
| `POST` | `/api/tokens` | admin, solo sessione | `{"name": "Grafana", "scope": "read", "expires_in_days": 90, "allowed_networks": ["192.168.1.0/24"]}`. `name` è obbligatorio (al massimo 80 caratteri); `scope` è `read` (predefinito) o `write`; `expires_in_days` va da 1 a 3650, `null` o assente per un token che non scade mai; `allowed_networks` accetta fino a 32 indirizzi o intervalli CIDR, memorizzati in forma di rete (`192.168.1.7/24` diventa `192.168.1.0/24`), vuoto o assente per ovunque. `201` con i campi del token più `secret` (mostrato una sola volta). `400` indica una rete non valida. |
| `DELETE` | `/api/tokens/{id}` | admin, solo sessione | Revoca. `204`; `404` quando sconosciuto o già revocato. |

Un token come appare nell'elenco:

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

## Header di risposta

Oltre a `X-DumbMonit-Api-Version` ([versionamento](#versioning)), agli header
`RateLimit-*` ([limiti di frequenza](#rate-limits)) e agli header CORS
([richieste cross-origin](#cross-origin-requests-cors)), ogni risposta porta
`X-Content-Type-Options: nosniff`,
`Referrer-Policy: same-origin` e una `Content-Security-Policy`. La policy parte da
`default-src 'none'` e apre solo ciò che l'interfaccia usa davvero — tutto servito
da DumbMonit stesso:

```
default-src 'none'; style-src 'self' 'unsafe-inline'; img-src 'self' data:;
font-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'self';
script-src 'self' 'nonce-<per response>'; frame-ancestors 'none'
```

Non vi compare alcuna origine di terze parti, e non dovrebbe mai comparirne: i
font, la libreria dei grafici e ogni asset sono inclusi nel binario. Gli script
inline della pagina (il tema applicato prima del primo rendering, il bootstrap di
SvelteKit) sono consentiti da un nonce rigenerato a ogni risposta, quindi
`'unsafe-inline'` non si applica mai agli script. Si applica agli stili, e non può
essere rimosso: Svelte imposta attributi `style=` sugli elementi, che né un nonce
né un hash possono coprire.

Le pagine di stato pubbliche sono fatte per essere incorporate: `/s/<slug>` e la sua
vista compatta `/s/<slug>/embed` — e solo queste due — ricevono la stessa policy
senza `frame-ancestors`. Tutto il resto, comprese le pagine di conferma
dell'iscrizione e di annullamento sotto `/s/`, porta anche `X-Frame-Options: DENY`.

Se metti DumbMonit dietro un reverse proxy, non lasciare che aggiunga un secondo
header `Content-Security-Policy`: i browser applicano l'intersezione di tutti, e un
valore predefinito del proxy senza il nonce lascia una pagina vuota.

## Salute

| Metodo | Rotta | Autenticazione | Scopo |
|---|---|---|---|
| `GET` | `/api/health` | pubblica | Versione del server e stato delle sue dipendenze. Sempre `200`: un componente in errore è nel corpo. |
| `GET` | `/api/update` | sessione o token | Versione in esecuzione, ultima release, `update_available`; il server interroga GitHub al massimo una volta al giorno. `POST /api/update/check` controlla subito, `PUT /api/update/settings` attiva o disattiva il controllo. Vedi [Aggiornamento](../install/update.md). |

```json
{"status":"ok","version":"0.1.0","database":{"ok":true},"victoria":{"ok":true,"embedded":true}}
```

`status` è `degraded` quando un componente fallisce; quel componente porta allora
una stringa `error`. `victoria.embedded` è `true` quando il server esegue la propria
VictoriaMetrics, `false` quando `DUMBMONIT_VM_URL` punta a una esterna.

## Backup e ripristino

| Metodo | Rotta | Autenticazione | Scopo |
|---|---|---|---|
| `GET` | `/api/backup` | admin, solo sessione | Ciò che conterrebbe un bundle (`contents`, una riga per sezione con il suo conteggio), la versione del formato del bundle, dove risiede il segreto dell'istanza (`secret_source`: `file` o `environment`) e lo stato dei backup locali pianificati (`schedule`). |
| `POST` | `/api/backup` | admin, solo sessione | `{"passphrase": "…", "include_account_secrets": false}`. Restituisce il bundle stesso come file JSON: un'intestazione in chiaro (`format`, `version`, `created_at`, `summary`, `kdf`) e un `payload` cifrato con AES-256-GCM sotto una chiave derivata dalla passphrase con Argon2id. Almeno 16 caratteri, altrimenti `400`. |
| `POST` | `/api/backup/restore` | admin, solo sessione | `{"bundle": {…}, "passphrase": "…", "apply": false}`. Simulazione a meno che `apply` sia `true`; entrambi eseguono lo stesso codice, la simulazione dentro una transazione di cui viene fatto il rollback. Risponde con il report: per sezione, `created`, `updated`, `skipped` e `notes`, più `warnings`. `400` con una passphrase errata, un file modificato o un bundle scritto in una versione di formato più recente. |
| `POST` | `/api/backup/local` | admin, solo sessione | Scrive subito un backup locale in stile pianificato e risponde con il nuovo `schedule`. |

Un bundle contiene ogni credenziale dell'istanza, ricifrata con la passphrase,
quindi queste rotte rifiutano i token API con una spiegazione: solo un
amministratore con accesso all'interfaccia web ne ottiene uno. Vedi
[Backup e ripristino](../install/backup.md).

## Tipi di dispositivo

| Metodo | Rotta | Scopo |
|---|---|---|
| `GET` | `/api/collectors` | Ogni tipo di dispositivo che questo server può monitorare: `kind`, `label`, `summary`, `examples`, `credential_types`, `address_hint`, `default_port`, `setup` (`title`, `steps`, `warning`, `doc_url`) e `options` (`key`, `label`, `help`, `placeholder`, `default`, `required`, `input`, `choices`). I tipi portati da un [pacchetto di integrazione](../packs/index.md) sono elencati allo stesso modo, come `pack.<id>`. |

## Pacchetti di integrazione

Sessione di amministratore o token API (ambito `write` per le modifiche). Vedi
[Pacchetti di integrazione](../packs/index.md).

| Metodo | Rotta | Scopo |
|---|---|---|
| `GET` | `/api/packs` | Pacchetti installati: `id`, `version`, `label`, `summary`, `kind` (`null` per un pacchetto con soli profili SNMP), `enabled`, `installed_at`, `sha256`, `metrics`, `rules` (uid), `snmp_profiles`, `targets` (dispositivi che lo usano), `warnings`, ed `error` quando il pacchetto memorizzato non supera più la validazione. |
| `GET` | `/api/packs/{id}` | Un pacchetto. |
| `POST` | `/api/packs` | Installa o aggiorna. Corpo: il YAML (`application/yaml`, `text/plain`…) oppure `{"yaml": "…"}` in JSON. `201` quando creato, `200` quando aggiornato o invariato, con `outcome` (`created`, `updated`, `unchanged`), `pack`, `rules_added` e `restart_required` (profili SNMP). `400` elenca ogni errore di validazione. |
| `PUT` | `/api/packs/{id}/enable`, `/api/packs/{id}/disable` | Registra o ritira il tipo di dispositivo del pacchetto; dispositivi, storico e regole restano. |
| `DELETE` | `/api/packs/{id}` | Disinstalla, con le sue regole. `409` finché dei dispositivi lo usano. |

## Target (dispositivi)

| Metodo | Rotta | Scopo |
|---|---|---|
| `GET` | `/api/targets` | Elenca ogni dispositivo. |
| `POST` | `/api/targets` | Ne crea uno. `201` con il dispositivo. Il rilevamento del profilo parte in background per SNMP. |
| `GET` | `/api/targets/{id}` | Un dispositivo. |
| `PUT` | `/api/targets/{id}` | Lo sostituisce. Omettere `credential` mantiene quella memorizzata; omettere `profile_id` mantiene il profilo rilevato (`""` lo cancella). Impostare `enabled: false` cancella gli avvisi del dispositivo senza notificare. |
| `DELETE` | `/api/targets/{id}` | `204`. Cancella gli avvisi del dispositivo senza notificare ed elimina le sue serie temporali da VictoriaMetrics (best effort). |
| `POST` | `/api/targets/{id}/probe` | Esegue subito il probe: `{"sample_count": 42, "series": ["if_octets_in{host=\"Core switch\",ifname=\"eth0\"}", …]}` — le chiavi delle serie come le ha prodotte il collector, senza il prefisso `dumbmonit_` che il writer aggiunge nel tragitto verso VictoriaMetrics. |
| `POST` | `/api/targets/{id}/discover` | Riesegue il rilevamento del profilo: `{"profile_id": "host-resources"}` oppure `null`. |

Un dispositivo come viene restituito:

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

Il segreto stesso non viene mai restituito: `credential_kind` è un'etichetta. I
timestamp del server sono UTC senza suffisso. `error_kind` classifica `last_error`:
`down` (il dispositivo non ha risposto) oppure `config` (dalla nostra parte —
credenziali, indirizzo, opzione), che l'interfaccia mostra come *Irraggiungibile* o
*Configurazione errata*; `null` quando l'ultimo probe è riuscito.

Crearne uno:

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

Forme delle credenziali (`type`): `none`; `snmp_community` (`community`); `snmp_v3`
(`username`, `auth: {protocol, passphrase}` facoltativo con `md5`, `sha1`,
`sha224`, `sha256`, `sha384` o `sha512`, `privacy: {protocol, passphrase}`
facoltativo con `des`, `aes128`, `aes192` o `aes256`, `context` facoltativo);
`api_token` (`token`, oppure le due metà `token_id` e `secret`, che il server unisce
in `user@realm!name=secret` per Proxmox VE e PBS); `username_password` (`username`,
`password`). `interval_secs` vale 60 per impostazione predefinita e non può scendere
sotto 10. `name` è al massimo di 200 caratteri e `address` di 253 (`400` oltre).
`parent_id` deve indicare un dispositivo esistente (`400 Parent device N not
found.`). Le opzioni del tipo vanno in `tags` sotto la loro chiave.

`via_agent` indica l'[agent relay](../install/remote-site.md) che esegue il probe del
dispositivo dalla propria rete invece del server; `null` (il predefinito) significa
che lo interroga il server. Deve essere un dispositivo `agent` (altrimenti `400`),
un dispositivo non può fare da relay a se stesso, e un dispositivo `agent` non può
essere inoltrato. Su `PUT`, un `via_agent` *omesso* mantiene il valore memorizzato,
come `profile_id` e `credential`; invia `null` per tornare al probe diretto. `POST
/api/targets/{id}/probe` e `/discover` su un dispositivo con relay passano dal relay
e attendono la sua risposta (`409` quando un probe è già in corso, `400 Timed out:
relay agent …` quando non risponde).

## Rilevamento

| Metodo | Rotta | Autenticazione | Scopo |
|---|---|---|---|
| `POST` | `/api/discovery` | admin | `{"cidr": "192.168.1.0/24", "community": "public", "port": 161, "timeout_ms": 1000}`. Scansiona una rete in cerca di dispositivi SNMP. `cidr` è obbligatorio; `community` vale `public` per impostazione predefinita; `timeout_ms` è limitato a 100–10 000. Le reti con più di 4096 indirizzi vengono rifiutate. La scansione è un `POST` con corpo JSON in modo che la community non compaia mai in un URL o in una riga di log. |

```json
{
  "scanned": 254,
  "devices": [
    {"address": "192.168.1.10", "sysname": "sw-core", "sysdescr": "…", "sysobjectid": "1.3.6.1.4.1.8072.3.2.10", "suggested_profile": "host-resources"}
  ]
}
```

## Guida alla prima esecuzione

I tre passaggi che la panoramica mostra su un'istanza nuova. Lo stato è conservato
sull'istanza, non nel browser, quindi saltare un passaggio lo salta ovunque.

| Metodo | Rotta | Autenticazione | Scopo |
|---|---|---|---|
| `GET` | `/api/onboarding` | sessione | A che punto è l'istanza. |
| `PUT` | `/api/onboarding` | admin | `{"skipped": true}` mette da parte l'intera guida; `{"dismissed": ["test"]}` mette da parte singoli passaggi. Un campo omesso mantiene il valore memorizzato; un id di passaggio diverso da `device`, `channel` o `test` è un `400`. Restituisce lo stesso documento di `GET`. |

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

`has_target`, `has_channel` e `notification_confirmed` vengono ricalcolati a ogni
lettura: sono fatti sull'istanza, non flag impostati dall'interfaccia.
`notification_confirmed` è vero quando un messaggio è effettivamente uscito da un
canale — un messaggio di test conta, un canale appena creato no.

`complete` è latch e non torna mai a falso. Il server lo imposta quando i tre
passaggi sono risolti (completati o scartati), e anche alla primissima lettura di
un'istanza che aveva già un dispositivo e un canale — così l'aggiornamento di
un'istanza consolidata non le mostra mai una guida alla prima esecuzione.

## Metriche {#metrics}

Entrambe le rotte fanno da proxy a VictoriaMetrics e restituiscono le serie nella
forma di Prometheus: `[{"metric": {labels…}, "values": [[timestamp_seconds, "value"], …]}]`.

| Metodo | Rotta | Scopo |
|---|---|---|
| `GET` | `/api/metrics/query?query=…` | Query MetricsQL istantanea. |
| `GET` | `/api/metrics/query_range?query=…&start=…&end=…&step=…` | Query su intervallo. `start` e `end` sono in **millisecondi** dall'epoch; `step` è in secondi e viene ampliato in modo che una serie non superi mai 2 000 punti. |

```bash
curl -b cookies.txt -G http://localhost:8080/api/metrics/query \
  --data-urlencode 'query=avg by (target, host) (dumbmonit_cpu_usage_percent)'
```

```json
[{"metric":{"host":"ThinkpadE14","target":"3"},"values":[[1757941205,"12.5"]]}]
```

## Scraping (fuori da `/api`)

Per un Prometheus o Grafana esistente. Entrambi accettano un token API `read` in
`Authorization: Bearer dmt_…` (funziona anche un cookie di sessione); uno scraper non
invia alcun header `X-Requested-With` e non ne ha bisogno — queste rotte si limitano
a leggere. Vedi il [riferimento delle metriche](metrics.md#scraping-dumbmonit) per
uno `scrape_config` da copiare e incollare e per la sorgente dati Grafana.

| Metodo | Rotta | Scopo |
|---|---|---|
| `GET` | `/metrics` | Stato di salute dell'istanza stessa nel formato testuale di Prometheus: scheduler, probe, scritture, avvisi, notifiche, agent, database, VictoriaMetrics. Qualche decina di serie, mai una per dispositivo. |
| `GET` | `/federate` | Le misure, nello stesso formato. `match[]` seleziona le serie (ripetibile, al massimo 10, `{__name__=~"dumbmonit_.*"}` per impostazione predefinita), `max_lookback` amplia la finestra in cui viene cercato l'ultimo punto (`5m` per impostazione predefinita). `400` oltre 8 MiB, indicando cosa restringere. |
| `GET`, `POST` | `/prometheus/api/v1/{route}` | La metà in lettura dell'API Prometheus, inoltrata all'archivio, così Grafana può usare `http://…/prometheus` come sorgente dati Prometheus. Solo `query`, `query_range`, `query_exemplars`, `series`, `labels`, `label/{name}/values`, `metadata` e `status/buildinfo`; qualsiasi altra cosa è `404`. |

`DUMBMONIT_METRICS_PUBLIC=true` serve le tre senza token — solo dietro un firewall,
poiché contengono ogni misura di ogni dispositivo.

## Altre rotte

Rotte non dettagliate sopra; il documento OpenAPI ne fornisce i parametri e le forme
di risposta. *read* e *write* sono l'ambito del token necessario; *none* significa
che nessun token è accettato (solo sessione).

| Metodo | Rotta | Ambito | Scopo |
|---|---|---|---|
| `POST` | `/api/targets/reorder` | write | Riordina i dispositivi. |
| `GET` | `/api/targets/{id}/security` | read | Punteggio di sicurezza, voto e controlli di un dispositivo. |
| `GET` | `/api/security/summary` | read | Punteggio di sicurezza di ogni dispositivo valutato, dal peggiore. |
| `GET` | `/api/targets/{id}/webchange/pages` | read | Pagine seguite da un controllo di sito web. |
| `GET` | `/api/targets/{id}/webchange/changes` | read | Modifiche trovate su un sito web sorvegliato, dalla più recente. |
| `GET` | `/api/targets/{id}/webchange/changes/{change_id}` | read | Una modifica, con entrambe le versioni e il diff per righe. |
| `GET` | `/api/targets/{id}/webchange/snapshots/{snapshot_id}/screenshot` | read | Lo screenshot di uno snapshot. |
| `POST` | `/api/targets/{id}/webchange/check` | write | Controlla subito un sito web sorvegliato. |
| `GET` | `/api/targets/{id}/opnsense/gateways`, `/traffic`, `/health` | read | OPNsense: gateway e indirizzi WAN; interfacce, firewall e DHCP; sistema, servizi, tunnel e firmware. |
| `GET` | `/api/targets/{id}/truenas/storage`, `/protection`, `/health` | read | TrueNAS: pool, dataset e dischi; scrub e task di protezione dei dati; sistema, avvisi e servizi. |
| `GET` | `/api/targets/{id}/ad`, `/ad/findings` | read | Active Directory: dominio, controller, gruppi privilegiati e rilievi; i rilievi di sicurezza grezzi. |
| `GET` | `/api/targets/{id}/redfish` | read | Hardware del server letto dal suo controller di gestione. |
| `GET` | `/api/targets/{id}/observability` | read | Salute di un server di log o metriche. |
| `GET` | `/api/targets/{id}/backend` | read | Salute di Redis, MongoDB, RabbitMQ o CrowdSec. |
| `GET` | `/api/targets/{id}/proxy` | read | Salute di un reverse proxy o di una registrazione di dominio. |
| `GET` | `/api/public/status/{slug}/badge.svg`, `/response.svg`, `/uptime.svg` | public | Badge dello stato complessivo, del tempo di risposta medio e dell'uptime di una pagina di stato. |
| `GET` | `/api/public/status/{slug}/components/{key}/badge.svg`, `/uptime.svg`, `/response.svg` | public | Gli stessi badge per un singolo servizio della pagina. |
| `GET` | `/api/public/status/{slug}/banner.js` | public | Script del banner di stato per siti web di terze parti. |
| `GET` | `/api/public/status/{slug}/rss` | public | Incidenti come feed RSS. |
| `GET` | `/api/music/now` | read | Ciò che riproducono i wall (monitor da sala). |
| `PUT`, `DELETE` | `/api/music/link` | write | Ogni wall riproduce questo link Spotify, Deezer o YouTube; oppure ferma il player incorporato. |
| `GET`, `DELETE` | `/api/music/spotify` | read, write | L'account Spotify connesso (mai un token); lo disconnette e dimentica i suoi token. |
| `POST` | `/api/music/spotify/authorize`, `/complete` | none | Avvia la connessione di Spotify; termina una connessione loopback. Solo sessione. |
| `GET` | `/api/music/spotify/token` | none | Un token di accesso Spotify di breve durata per il player del wall. Solo sessione. |
| `GET`, `PUT` | `/api/music/speaker` | read, write | L'altoparlante Spotify del wall come lo mostrano le Impostazioni; lo rinomina (admin). |
| `POST` | `/api/music/speaker/report`, `/play` | none | Un wall riporta lo stato del suo altoparlante; vi riproduce l'account. Solo sessione. |

## Avvisi

| Metodo | Rotta | Scopo |
|---|---|---|
| `GET` | `/api/alerts` | Avvisi attivi (in attesa, scattati, soppressi, risolti di recente). Gli avvisi di dispositivi eliminati o in pausa non vengono mai elencati. |
| `GET` | `/api/alerts/history?since=2026-09-01T00:00:00Z&limit=200&dismissed=false` | Transizioni di fase. `since` è RFC 3339, per impostazione predefinita gli ultimi sette giorni; `limit` deve essere positivo. `dismissed` (predefinito `false`) esclude le voci cancellate con le due rotte seguenti; nulla viene mai distrutto da una cancellazione, solo nascosto alla vista predefinita. |
| `POST` | `/api/alerts/history/{id}/dismiss` | Cancella una voce `resolved` dalla vista predefinita dello storico. `204`. `404` per un id sconosciuto o una transizione ancora attiva. Operatore; sottoposta ad audit. |
| `POST` | `/api/alerts/history/dismiss-resolved` | Cancella in una volta tutte le voci `resolved` non ancora cancellate ("Cancella tutti i risolti"). Restituisce `{"dismissed": <count>}`. Operatore; sottoposta ad audit. |
| `POST` | `/api/alerts/{fingerprint}/ack` | Presa in carico: `{"duration_secs": 14400, "note": "…"}` oppure `{"until": "2026-09-22T18:00:00Z"}` (uno dei due; nessuno dei due significa 4 ore, al massimo 30 giorni). `{"until": null}` la annulla. Restituisce l'avviso. Operatore; sottoposta ad audit. |
| `DELETE` | `/api/alerts/{fingerprint}/ack` | Annulla la presa in carico. Restituisce l'avviso. Operatore. |

Un avviso attivo:

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

Una voce dello storico ha `id`, `fingerprint`, `rule_uid`, `target_id`,
`from_phase`, `to_phase`, `severity`, `value`, `notified`, `reason` (vuoto, oppure il
motivo per cui non è stato inviato nulla: `learning: would have fired`, `suppressed:
device 2 unreachable`, `maintenance window`, `acknowledged by admin`, `device removed
or disabled`), `at` e `dismissed` (cancellata dalla vista attiva; la riga stessa
viene conservata). `acked` è vero finché `acked_until` è nel futuro: promemoria ed
escalation sono in pausa, la risoluzione viene comunque notificata e cancella la
presa in carico. Un `target_id` che non esiste più è mostrato dall'interfaccia come
"(deleted device)".

### Regole

| Metodo | Rotta | Scopo |
|---|---|---|
| `GET` | `/api/alerts/rules` | Ogni regola. |
| `POST` | `/api/alerts/rules` | Crea. `201`. |
| `PUT` | `/api/alerts/rules/{id}` | Aggiorna. Una regola integrata mantiene la sua query. |
| `DELETE` | `/api/alerts/rules/{id}` | `204`. Le regole integrate non possono essere eliminate. |
| `POST` | `/api/alerts/rules/{id}/enable` | `{"enabled": false}`. Restituisce la regola. |

Una regola:

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

Crearne una richiede almeno `name` e `query`; `kind` (`threshold`, `anomaly`,
`predict`), `operator`, `threshold`, `for_secs`, `severity` (`info`, `warning`,
`critical`), `selector` (`{"kind":"all"}`, `{"kind":"ids","ids":[…]}` oppure
`{"kind":"labels","labels":{…}}`), `channels` (id dei canali; vuoto significa ogni
canale abilitato), `unit`, `repeat_secs`, `escalate_after_secs` ed `enabled` sono
facoltativi.

### Silenziamenti (finestre di manutenzione)

| Metodo | Rotta | Scopo |
|---|---|---|
| `GET` | `/api/alerts/silences` | Ogni finestra, con `active_now`, `active_until` e `next_start_at` (RFC 3339, calcolati dal server). |
| `POST` | `/api/alerts/silences` | Crea. `201`. Operatore. |
| `DELETE` | `/api/alerts/silences/{id}` | `204`. Operatore. |

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

Una pianificazione singola è `{"kind": "once", "starts_at": "2026-09-20T22:00:00Z",
"ends_at": "2026-09-21T02:00:00Z"}`. Una mensile è
`{"kind": "monthly", "days": [1], "nth_weekdays": [{"nth": 1, "weekday": 6}],
"start_minute": 120, "duration_minutes": 120, "timezone": "Europe/Paris"}` —
`nth` va da 1 a 5, oppure `-1` per l'ultimo del mese; un mese senza quell'occorrenza
viene saltato. I giorni sono 0 = lunedì … 6 = domenica (1–31 per i `days` di una
mensile); i minuti sono contati dalla mezzanotte locale (0–1439). In una
pianificazione ricorrente, `timezone` (un nome IANA) prevale su
`utc_offset_minutes` ed è ciò che mantiene una finestra alla sua ora locale
attraverso i cambi di ora legale; un nome di fuso sconosciuto viene rifiutato.

### Override per dispositivo {#per-device-overrides}

Una regola può essere regolata per un solo dispositivo senza toccare la regola
stessa. `{"enabled": false}` è ciò che scrive l'azione **Ignora** dell'interfaccia
(sull'avviso, sulla pagina del dispositivo o sulla regola): il dispositivo viene
escluso da quella singola regola in modo permanente, reversibile, senza disabilitare
la regola per nessun altro.

| Metodo | Rotta | Scopo |
|---|---|---|
| `GET` | `/api/alerts/overrides?target_id=4` | Ogni override, o quelli di un dispositivo: `rule_uid`, `target_id`, `threshold`, `clear_threshold`, `enabled`. |
| `GET` | `/api/alerts/rules/{id}/overrides` | Gli override di una regola. |
| `PUT` | `/api/alerts/rules/{id}/overrides/{target_id}` | `{"threshold": 95, "clear_threshold": 90, "enabled": true}` — ogni campo è facoltativo; `null` significa "come la regola". Crea o sostituisce. Un operatore può inviare solo `{"enabled": false}` (ignora); gli override di soglia del dispositivo, se presenti, vengono mantenuti. |
| `DELETE` | `/api/alerts/rules/{id}/overrides/{target_id}` | `204`. Per un operatore, annulla soltanto l'ignora: gli override di soglia restano. |
| `GET` | `/api/targets/{id}/rules` | The rules that watch a device: those whose query reads a metric it wrote in the last 24 h. `{"status": "ok" \| "no_data" \| "unavailable", "rules": [{id, uid, name, description, severity, enabled, ignored, custom_threshold, firing}]}`. |

## Politica di notifica

| Metodo | Rotta | Scopo |
|---|---|---|
| `GET` | `/api/notify/policy` | La politica globale: `batch_window_secs` (0 = invia subito), `max_per_hour` per canale (0 = illimitato), `flap_events`, `flap_window_secs`, `flap_hold_secs` (0 = nessun rilevamento del flapping), `public_url` (link nei messaggi; vuoto = `DUMBMONIT_PUBLIC_URL`), `escalate_after_secs` e `escalate_channel` (0 / `null` = nessuna escalation). |
| `PUT` | `/api/notify/policy` | Stessi campi, ognuno facoltativo: un campo omesso mantiene il proprio valore. `"escalate_channel": null` disattiva l'escalation. Restituisce la politica. |
| `POST` | `/api/notify/match-preview` | `{"matcher": {…}}` → quali dispositivi seleziona un filtro di instradamento di un canale: `devices` (`id`, `name`, `kind`, `tags`, `matched`), `matched`, `total`, `rules`, `excluded_rules`. |

## Canali di notifica

| Metodo | Rotta | Scopo |
|---|---|---|
| `GET` | `/api/notify/kinds` | Ogni tipo di canale con i suoi campi `settings` e `secrets` (`key`, `label`, `required`, `input`, `help`, `placeholder`, `options`, `shape`, `default`) e un `doc_url` come `https://dumbmonit.readthedocs.io/en/latest/notifications/#discord`. |
| `GET` | `/api/notify/channels` | Ogni canale: `id`, `name`, `kind`, `enabled`, `settings`, `has_secret`, `last_error`, `last_sent_at`, `policy` (`min_severity`, `notify_resolved`, `min_interval_secs`, `quiet_hours`, `matcher`). I segreti non vengono mai restituiti. |
| `POST` | `/api/notify/channels` | Crea. `201`. |
| `PUT` | `/api/notify/channels/{id}` | Aggiorna. Omettere `secrets` mantiene quelli memorizzati; `"secrets": {}` li cancella. In `policy`, omettere `matcher` mantiene il filtro di instradamento e `"matcher": null` lo cancella. |
| `DELETE` | `/api/notify/channels/{id}` | `204`. |
| `POST` | `/api/notify/channels/{id}/test` | Invia un messaggio di test: `{"ok": true, "message": "…"}`. |

```json
{
  "name": "Ops Discord",
  "kind": "discord",
  "enabled": true,
  "settings": {"username": "DumbMonit"},
  "secrets": {"webhook_url": "https://discord.com/api/webhooks/…"}
}
```

Le chiavi esatte per tipo provengono da `/api/notify/kinds` e sono documentate in
[Canali di notifica](../notifications.md).

### Dispositivi Web Push

I dispositivi dell'**account con accesso effettuato** (solo sessione, osservatori
inclusi; i token API ricevono `401`). Un canale `webpush` invia ad essi; vedi
[Notifiche push (PWA)](../using/push-notifications.md).

| Metodo | Rotta | Scopo |
|---|---|---|
| `GET` | `/api/webpush` | `{"public_key": "…", "devices": […]}`: la chiave pubblica VAPID (base64url, l'`applicationServerKey` con cui iscriversi) e i dispositivi dell'account — `id`, `device` (`Firefox on Android`), `push_service` (host), `fingerprint` (i primi 16 caratteri esadecimali dello SHA-256 dell'endpoint), `created_at`, `last_success_at`, `last_error`. L'endpoint, le chiavi del browser e la chiave privata non vengono mai restituiti. |
| `POST` | `/api/webpush/subscriptions` | `PushSubscription.toJSON()`: `{"endpoint": "https://…", "keys": {"p256dh": "…", "auth": "…"}}`. `201` con il dispositivo. L'endpoint deve essere HTTPS su un indirizzo pubblico: gli indirizzi privati, loopback e link-local ricevono `400`. Lo stesso endpoint di nuovo ne sostituisce la riga. |
| `DELETE` | `/api/webpush/subscriptions/{id}` | `204`; `404` per un dispositivo di un altro account. |
| `POST` | `/api/webpush/test` | `{}` oppure `{"id": 3}`. Invia una notifica di test: `{"ok", "delivered", "failed", "removed", "message"}`. Le iscrizioni a cui viene risposto `404`/`410` vengono rimosse. |

## Report via email {#email-reports}

Solo amministratori, `GET` compreso (la risposta elenca i destinatari). I bearer
token richiedono l'ambito `write` per le modifiche. Vedi
[Report via email](../using/reports.md).

| Metodo | Rotta | Autenticazione | Scopo |
|---|---|---|---|
| `GET` | `/api/reports/schedules` | admin | Ogni report: `id`, `name`, `enabled`, `frequency`, `weekday` (0 è lunedì), `day_of_month`, `hour`, `timezone`, `recipients`, `channel_id`, `last_sent_at`, `last_error`, `next_run_at`. |
| `POST` | `/api/reports/schedules` | admin | Gli stessi campi scrivibili; ciascuno vale per impostazione predefinita settimanale, lunedì, 8:00, `UTC`. `recipients` contiene al massimo 20 indirizzi ed è obbligatorio per abilitare il report; `channel_id` deve essere un canale `smtp` o `null`. `201`; `409` oltre 10 report. |
| `GET` | `/api/reports/schedules/{id}` | admin | Un report. |
| `PUT` | `/api/reports/schedules/{id}` | admin | Sostituzione completa. |
| `DELETE` | `/api/reports/schedules/{id}` | admin | `204`. |
| `POST` | `/api/reports/schedules/{id}/send-test` | admin | Invia via email un'anteprima ai destinatari salvati: `{"sent": 2, "failed": 0}`. Non tocca la pianificazione. `400` senza un canale email utilizzabile; `409` entro 30 secondi dall'anteprima precedente. |

## Pagine di stato e incidenti

| Metodo | Rotta | Autenticazione | Scopo |
|---|---|---|---|
| `GET` | `/api/status-pages` | sessione | Ogni pagina con le sue voci: `page` (`id`, `slug`, `title`, `description`, `published`, `theme`, `show_uptime_days`, `created_at`, `updated_at`) e `items` (`id`, `page_id`, `target_id`, `label`, `group_name`, `position`). |
| `POST` | `/api/status-pages` | admin | `{"title", "slug", "description", "published", "theme", "show_uptime_days"}`. `title` è obbligatorio (al massimo 120 caratteri); `slug` (`^[a-z0-9-]{2,40}$`) è derivato dal titolo quando omesso; `theme` è `auto` (predefinito), `light` o `dark`; `show_uptime_days` va da 7 a 90 (90 per impostazione predefinita). `201`; `409` per uno slug già in uso. |
| `GET` | `/api/status-pages/{id}` | sessione | Una pagina con le sue voci. |
| `PUT` | `/api/status-pages/{id}` | admin | Stessi campi, ma una sostituzione completa anziché una patch: `title` è obbligatorio e ogni campo omesso torna al valore predefinito (slug ricavato di nuovo dal titolo, descrizione vuota, `published: false`, `theme: auto`, 90 giorni di uptime). |
| `DELETE` | `/api/status-pages/{id}` | admin | `204`. L'URL pubblico smette di rispondere. |
| `POST`, `PUT` | `/api/status-pages`, `/api/status-pages/{id}` — aspetto e iscrizioni | admin | Accettano anche `accent` (`default`, `blue`, `teal`, `violet`, `rose`, `amber`), `scenes` (id di scena dall'elenco dell'editor — città, le nuove meraviglie e le meraviglie antiche, ad es. `venice`, `paris`, `giza`, `tajmahal`, senza duplicati; `[]` per nessuna; omesso alla creazione: `["paris"]`), `simple` (booleano, predefinito `false`: la pagina pubblica non mostra scena, mascotte o animazione e segue la preferenza chiaro/scuro del visitatore), `scene_rotation` (`visit`, `1m`, `10m` o `1h`), `footer_text` (testo semplice, 280 caratteri), `homepage_url` (`http(s)://` o vuoto), `subscribe_channel_id` (un canale `smtp`, oppure `null` per disattivare l'iscrizione via email) e `domain` (il nome host pubblico della pagina, come `status.example.com`, oppure `null`/`""` per nessuno — vedi [Dominio personalizzato](../using/status-pages.md#custom-domain)). `domain` viene convertito in minuscolo e perde un prefisso `https://` o una barra finale; una porta, un percorso o un indirizzo IP sono `400`, l'indirizzo di DumbMonit stesso (il suo URL pubblico, o l'host su cui è arrivata la richiesta) è `400`, e un nome che un'altra pagina usa è `409`. A differenza degli altri campi, queste otto chiavi mantengono il valore memorizzato quando omesse da un `PUT`. La vista della pagina le restituisce con `logo_type` (`image/png`, `image/jpeg`, `image/webp` o `null`). |
| `GET`, `PUT`, `DELETE` | `/api/status-pages/{id}/logo` | sessione / admin | Il logo della pagina. `PUT` accetta `{"data": "<base64 o data: URL>"}`: PNG, JPEG o WebP riconosciuti dal contenuto, al massimo 256 KiB, qualsiasi altra cosa `400`. `204`. `GET` è l'anteprima dell'editor (bozze comprese). |
| `GET` | `/api/status-pages/{id}/subscribers` | sessione | Iscritti via email: `id`, `page_id`, `email`, `confirmed_at` (`null` finché in attesa), `created_at`. I loro token non lasciano mai il server. |
| `DELETE` | `/api/status-pages/{id}/subscribers/{subscriber}` | admin | Rimuove un indirizzo. `204`. |
| `PUT` | `/api/status-pages/{id}/items` | admin | `[{"target_id": 4, "label": "NAS", "group_name": "Storage"}, …]` — l'elenco completo e ordinato dei dispositivi mostrati sulla pagina. |
| `GET` | `/api/incidents` | sessione | Ogni incidente con i suoi aggiornamenti: `incident` (`id`, `page_id`, `title`, `kind`, `status`, `severity`, `starts_at`, `ends_at`, `created_at`, `updated_at`) e `updates`. |
| `POST` | `/api/incidents` | admin | `{"title", "kind", "status", "severity", "page_id", "starts_at", "ends_at", "body"}` — `body` è il primo aggiornamento. `201`. |
| `PUT` | `/api/incidents/{id}` | admin | Stessi campi; un campo omesso mantiene il proprio valore. |
| `DELETE` | `/api/incidents/{id}` | admin | `204`. |
| `GET` | `/api/incidents/{id}/updates` | sessione | La cronologia: `id`, `incident_id`, `status`, `body`, `created_at`. |
| `POST` | `/api/incidents/{id}/updates` | admin | `{"status": "monitoring", "body": "…"}`. Aggiunge un aggiornamento e porta l'incidente a `status`. `201`. |
| `GET` | `/api/public/status/{slug}` | pubblica | Il documento JSON da cui è costruita una pagina pubblicata (anche `badge.svg` e `rss` sotto lo stesso percorso). In `page`, `show_uptime_days` è il massimo configurato e `history_days` la finestra effettivamente mostrata (l'età dei dati della pagina, da 7 giorni fino a quel massimo) e `simple` rispecchia la modalità semplice. Vedi [Pagine di stato](../using/status-pages.md). Sul dominio personalizzato di una pagina (`Host` uguale al suo `domain`), rispondono solo le rotte `/api/public/status/{slug}/…` di questa pagina; ogni altra rotta di questo riferimento è lì `404`. |
| `GET` | `/api/public/status/{slug}/uptime.svg`, `/response.svg` | pubblica | Badge della pagina: uptime medio su `?days=` `1`, `7`, `30` (predefinito) o `90` — `400` oltre lo storico della pagina — e tempo di risposta medio. SVG, `Cache-Control: public, max-age=60`. |
| `GET` | `/api/public/status/{slug}/components/{key}/badge.svg`, `/uptime.svg`, `/response.svg` | pubblica | Gli stessi tre badge per un singolo servizio; `key` è la `key` del servizio nel documento pubblico (la sua etichetta in forma di URL). `404` per una chiave sconosciuta. |
| `GET` | `/api/public/status/{slug}/logo` | pubblica | Il logo della pagina con il suo tipo di immagine, memorizzabile in cache per un giorno (il `logo_url` del documento porta una versione). `404` se non ce n'è uno. |
| `POST` | `/api/public/status/{slug}/subscribe` | pubblica | `{"email": "…"}`. Invia via email un link di conferma tramite il canale SMTP della pagina. Sempre `202` con lo stesso messaggio, qualunque sia lo stato dell'indirizzo; `400` per un indirizzo non valido; `404` quando la pagina non offre l'iscrizione via email; `409` quando la pagina non può accogliere altri iscritti; `429` con `Retry-After` oltre cinque richieste per client e sessanta in totale ogni 15 minuti. |
| `POST` | `/api/public/status/{slug}/confirm?token=…` | pubblica | Conferma un'iscrizione (il link dell'email di conferma apre `/s/{slug}/confirm`, il cui pulsante invia qui un POST). `404` per un token sconosciuto o scaduto. |
| `POST` | `/api/public/status/{slug}/unsubscribe?token=…` | pubblica | Rimuove un'iscrizione; il corpo viene ignorato, quindi è anche il bersaglio one-click RFC 8058 di `List-Unsubscribe`. Sempre `200`. |

## Token degli agent e ingest

| Metodo | Rotta | Autenticazione | Scopo |
|---|---|---|---|
| `GET` | `/api/agent/tokens` | solo sessione | Ogni token: `id`, `name`, `prefix`, `created_at`, `last_used_at`, `revoked_at`, `max_uses` (`null` per un token di flotta), `uses`, `expires_at`. |
| `POST` | `/api/agent/tokens` | admin, solo sessione | `{"name": "Home fleet", "base_url": "http://server:8080", "reusable": false, "max_uses": null, "expires_in_days": null}`. `201` con i campi del token più `secret` (mostrato una sola volta), `install_linux` e `install_windows`. `base_url` è l'URL che gli agent useranno; per impostazione predefinita è l'indirizzo di ascolto. Senza `reusable`, il token è **monouso**: registra una macchina e non di più. Con esso, `max_uses` limita le registrazioni (`null`: nessun limite). `expires_in_days` ferma le *registrazioni* dopo quel numero di giorni; le macchine già registrate continuano a riferire. `400` per un conteggio inferiore a 1. |
| `DELETE` | `/api/agent/tokens/{id}` | admin, solo sessione | Revoca. `204`; `404` quando sconosciuto o già revocato. |
| `POST` | `/api/ingest` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Riceve un batch di campioni da un agent (corpi fino a 16 MB). Non è pensato per essere chiamato a mano. L'header porta il segreto di associazione della macchina; un agent che non ne ha uno lo omette, ed è così che ne chiede uno. La risposta aggiunge `agent_secret` (il segreto, restituito esattamente una volta, quando la macchina viene associata) e `bound`. `403` quando la macchina è associata a un'altra installazione dell'agent, o quando il token non può più registrare — il messaggio dice quale, e cosa fare. |
| `GET` | `/api/agent/relay?key=…&wait=N` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Probe delegati a un agent relay (`relay: true`). Trattenuta fino a `wait` secondi (al massimo 25) quando nulla è in sospeso. Ogni elemento è un comando di tipo `probe` i cui `args` portano il target, la sua credenziale decifrata, `timeout_secs` e `discover`. Mai scritto su disco. |
| `POST` | `/api/agent/relay/{id}?key=…` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Esito di un probe delegato: `{"duration_ms", "error", "samples", "profile_id"}` (corpi fino a 16 MB). `204`; `404` quando il probe è scaduto o appartiene a un altro agent. |
| `GET` | `/api/agent/commands?key=…&wait=N` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Comandi in coda per un agent (riavvio o aggiornamento di un container), trattenuta fino a `wait` secondi quando nulla è in sospeso. Non è pensato per essere chiamato a mano. `key` da solo non prova nulla — è pubblica — quindi una macchina associata deve presentare anche il proprio segreto di associazione; altrimenti `403`, ed è ciò che impedisce a una macchina di prendere i comandi di un'altra. |
| `POST` | `/api/agent/commands/{id}?key=…` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Avanzamento ed esito di un comando, riportati dall'agent. `result` è troncato agli ultimi 4 KB, con `[truncated by the server, beginning dropped]` in testa quando succede. |
| `GET` | `/api/relays` | sessione | Ogni dispositivo agent come possibile relay: `id`, `name`, `site`, `relay` (dichiarato `relay: true`), `last_seen_at`, `relayed` (dispositivi raggiunti tramite esso). Prima i relay, poi per nome. |

```json
{
  "id": 1, "name": "Home fleet", "prefix": "dmon_ab12",
  "created_at": "2026-09-15 13:00:00", "last_used_at": null, "revoked_at": null,
  "secret": "dmon_…",
  "install_linux": "curl -sSL http://server:8080/install.sh | sh -s -- --token=dmon_… --url=http://server:8080",
  "install_windows": "& ([scriptblock]::Create((irm http://server:8080/install.ps1))) -Token dmon_… -Url http://server:8080"
}
```

## Container e comandi (dispositivi agent)

Azioni sui container di una macchina che esegue l'agent. Ogni rotta risponde `404`
quando il dispositivo non è un target `agent`.

| Metodo | Rotta | Scopo |
|---|---|---|
| `GET` | `/api/targets/{id}/agent` | La macchina come l'ha descritta l'ultima volta il suo agent: `hostname`, `os`, `os_version`, `arch`, `agent_version`, `commands_supported`, `relay`, `site`, `relayed` (dispositivi raggiunti tramite questo agent), `last_seen_at`, `binding`, `bound`, `bound_at`, `rebind_until`. `404` finché un agent non ha riferito. `commands_supported` è `true` solo quando l'agent ha dichiarato di recuperare comandi (`commands: true`, il predefinito degli agent attuali); un agent più vecchio o con `commands: false` dà `false`. `binding` è `bound`, `pending` (il binario può essere associato e lo sarà al suo prossimo batch) o `unsupported` (un agent anteriore all'associazione: reinstallalo). |
| `POST` | `/api/targets/{id}/agent/rebind` | Admin. Apre una finestra di un'ora durante la quale questa macchina può associarsi di nuovo con un token di registrazione valido — la via per rientrare dopo che una reinstallazione si è portata via il segreto dell'agent. `{"rebind_until", "minutes"}`. La finestra si chiude non appena viene usata, e il vecchio segreto smette di funzionare. `404` finché un agent non ha riferito. |
| `GET` | `/api/targets/{id}/containers` | Ogni container dall'ultimo batch: `name`, `image`, `up`, `health`, `restart_count`, `uptime_seconds`, `image_age_seconds`, `update_available`, `policy`, `last_command`. |
| `PUT` | `/api/targets/{id}/containers/{name}/policy` | `{"auto_restart": bool, "auto_update": bool, "prune_old_image": bool, "only_in_maintenance": bool}`. Ogni campo è facoltativo: un campo omesso mantiene il valore memorizzato. Restituisce la politica completa. |
| `POST` | `/api/targets/{id}/containers/{name}/restart` | Mette in coda un riavvio. `201` con il comando. `404` quando `name` non è nell'inventario dell'agent; `409` quando lo stesso comando è già in coda o in esecuzione, o quando l'agent non può eseguire comandi (vedi `commands_supported`). |
| `POST` | `/api/targets/{id}/containers/{name}/update` | Mette in coda un aggiornamento; `{"prune": bool}` facoltativo (il predefinito segue la politica). Stessi codici di stato del riavvio. |
| `GET` | `/api/targets/{id}/commands` | Gli ultimi 20 comandi, dal più recente: `id`, `kind` (`container.restart`, `container.update`), `args`, `status`, `requested_by` (un utente, oppure `policy`), `created_at`, `started_at`, `finished_at`, `result`. |
| `DELETE` | `/api/targets/{id}/commands/{command_id}` | Annulla un comando in coda. `204`; `409` quando l'agent lo ha già preso in carico o quando è già chiuso; `404` quando non appartiene a questo dispositivo. |

`status` procede `queued → running → done | failed`. Esistono altri due stati
finali: `cancelled` (una persona ha ritirato il comando) ed `expired` (il server ha
rinunciato dopo dieci minuti perché nessun agent è venuto a recuperarlo — agent
fermo, troppo vecchio per il canale di comandi, o installato con `commands: false`).
Il server fa scadere da solo i comandi obsoleti ogni minuto; un comando in coda non
blocca mai un nuovo comando per più di così.

## Guest Proxmox VE

Le righe del pannello **Guest** di un dispositivo Proxmox VE, assemblate dall'ultimo
probe nell'archivio delle metriche (quattro query istantanee, nulla chiesto
all'hypervisor). `404` quando il dispositivo non esiste, `400` quando non è un
target `proxmox`.

| Metodo | Rotta | Scopo |
|---|---|---|
| `GET` | `/api/targets/{id}/proxmox/guests` | Un oggetto per VM o container, ordinati per nodo poi per VMID: `vmid`, `name`, `node`, `kind` (`qemu`, `lxc`), `status` (`running`, `stopped`, `paused`, `suspended`, `template`, `unknown`), `cpu_percent` (dei core assegnati), `cpu_count`, `memory_used_bytes`, `memory_total_bytes`, `memory_percent`, `balloon_bytes`, `disk_used_bytes`, `disk_total_bytes`, `disk_percent`, `agent` (`true` il guest agent ha risposto, `false` abilitato ma silenzioso, `null` nessuno), `network_in_bps`, `network_out_bps`, `disk_read_bps`, `disk_write_bps`, `uptime_seconds`, `last_backup_age_seconds`, `ha_state`, `pool` (il suo resource pool), `lock` (solo mentre un lock è detenuto: `backup`, `migrate`…), `os` e `ip` (visti dall'interno del guest, aggiornati ogni ora). Ogni valore sconosciuto è `null` — una VM senza guest agent ha `disk_total_bytes` ma `disk_used_bytes: null`; un guest fermo conserva le sue dimensioni e perde le sue misure. |

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

## Nodi Proxmox VE e Ceph

Le sezioni **Nodi** e **Ceph** della stessa pagina del dispositivo, lette dallo
stesso ultimo probe. Stessi codici di stato di sopra.

| Metodo | Rotta | Scopo |
|---|---|---|
| `GET` | `/api/targets/{id}/proxmox/nodes` | `{"nodes": […], "fencing_state", "fencing_armed"}` — lo stato del watchdog HA (`armed`, `standby`…) e se isolerebbe davvero un nodo perso, poi un oggetto per nodo, ordinati per nome: `name`, `up`, `cpu_percent`, `cpu_iowait_percent`, `memory_percent`, `ksm_shared_bytes`, `rootfs_percent`, `uptime_seconds`, `version` (la release di `pve-manager` installata su quel nodo), `services_down` (nomi delle unit dei demoni fermi o falliti), `interfaces_offline` (interfacce impostate per avviarsi al boot che non sono attive), `thin_pools` (`name`, `vg`, `used_percent`, `metadata_used_percent`, `size_bytes`) e `volume_groups` (`name`, `used_percent`, `size_bytes`). Un nodo che ha smesso di rispondere conserva la propria voce con `up: false`. |
| `GET` | `/api/targets/{id}/proxmox/ceph` | `available` (`false` quando il cluster non ha Ceph — l'interfaccia non disegna allora alcuna sezione), `health` (0 OK, 1 WARN, 2 ERR, 3 sconosciuto), `health_status`, `bytes_used`, `bytes_total`, `used_percent`, `osds_total`, `osds_up`, `osds_in`, `osds` (`name`, `host`, `device_class`, `up`, `in`, `used_percent`, `used_bytes`, `total_bytes`, `apply_latency_ms`, `commit_latency_ms`), `pools` (`name`, `used_percent`, `used_bytes`, `size`, `min_size`, `pg_num`, `pg_num_optimal`, `autoscale`), `filesystems` (nomi dei CephFS), `flags` (flag OSD attualmente impostati, come `noout`) e `muted_checks` (controlli di salute silenziati, che `HEALTH_OK` non menziona più). |

## Proxmox Backup Server

I pannelli di un dispositivo `pbs`, letti dall'ultimo probe (nulla chiesto al server
tranne il log dei task). `404` quando il dispositivo non esiste, `400` quando non è
un target `pbs`.

| Metodo | Rotta | Scopo |
|---|---|---|
| `GET` | `/api/targets/{id}/pbs/calendar?days=30&offset=120` | `{"probed_at", "days", "offset_minutes", "groups": […]}`: una voce per gruppo di backup (`datastore`, `namespace`, `backup_type`, `backup_id`, `name`, `count`, `last_time`, `last_size`, `last_verified`, `last_success`, `last_failure`, `retention`) con un array `days` di `{date, state, runs, snapshot}`. `offset` è lo scarto UTC in minuti di chi guarda, in modo che i giorni siano tagliati alla mezzanotte locale. |
| `GET` | `/api/targets/{id}/pbs/failures?days=30` | Task falliti: `upid`, `worker_type`, `kind`, `worker_id`, `datastore`, `start`, `end`, `error`. |
| `GET` | `/api/targets/{id}/pbs/jobs` | `{"probed_at", "jobs": […]}`: job di sync, verifica, prune e garbage collection con la loro pianificazione e l'ultimo risultato. |
| `GET` | `/api/targets/{id}/pbs/health` | Datastore (utilizzo, data stimata di riempimento), dischi (SMART, usura) e pool ZFS. |
| `GET` | `/api/targets/{id}/pbs/tasks/{upid}/log?lines=200` | Il log di un task, scaricato dal server: `{"upid", "lines": […]}`; `lines` limita quante ne vengono restituite. |
| `GET` | `/api/targets/{id}/pbs/disks/smart?disk=/dev/sda` | Attributi SMART di un disco; `disk` è obbligatorio (`400` senza). |
| `GET` | `/api/targets/{id}/pdm/remotes` | Proxmox Datacenter Manager: totali del parco (`estate`) e una riga per istanza federata (`id`, `kind`, `reachable`, `error`, `version`, `version_behind`, conteggi di nodi e guest, memoria e storage, `subscription`, `last_collection`, `tasks_failed`), prima le irraggiungibili. |
| `GET` | `/api/targets/{id}/pdm/failures?days=14` | Task falliti in tutto il parco: `upid`, `remote`, `worker_type`, `kind`, `worker_id`, `node`, `start`, `end`, `error`. |
| `GET` | `/api/targets/{id}/pdm/health` | L'host della console: CPU, memoria, filesystem root, uptime, certificati, aggiornamenti in sospeso e sottoscrizione. |
| `GET` | `/api/targets/{id}/pmg/queues` | Proxmox Mail Gateway: le quattro code Postfix in ordine di lettura (`queue`, `messages`, `domains`, `oldest_age_seconds`, `top_domains`, `stuck`), più `total_messages` e un `stuck` a livello di gateway. `oldest_age_seconds` è un limite inferiore: qshape riporta intervalli di età. |
| `GET` | `/api/targets/{id}/pmg/traffic` | Totali di oggi (`mail`), la curva del traffico recente (`recent`), l'istogramma del punteggio spam, i virus bloccati oggi e i conteggi della quarantena (`quarantine`). Solo conteggi: nessun contenuto dei messaggi. |
| `GET` | `/api/targets/{id}/pmg/health` | Una voce per nodo (sistema, servizi, `signatures` con `family`, `age_seconds` e `stale`, `expiring_certificates`, aggiornamenti, sottoscrizione), i membri del `cluster` con il loro stato di sincronizzazione, e `stopped_services` tra i nodi. |

## Synology

| Metodo | Rotta | Scopo |
|---|---|---|
| `GET` | `/api/targets/{id}/synology` | Panoramica di un dispositivo `synology` dall'ultimo probe: `system` (modello, versione DSM, uptime, temperatura, CPU, memoria), `volumes` (`name`, `fs_type`, `raid_type`, `status`, `severity`, `total_bytes`, `used_bytes`), `disks` (`model`, `serial`, `kind`, `ssd`, `status`, `smart_status`, `temperature_celsius`, `size_bytes`, `remaining_life_percent`, …) e `sampled_at`. `404` quando il dispositivo non esiste, `400` quando non è un Synology. |
| `GET` | `/api/targets/{id}/synology/abb` | Active Backup for Business: `tasks`, e `devices` — un report per dispositivo protetto con la sua cadenza appresa in `assessment` (`state`, `last_success_s`, `typical_interval_s`, conteggi a 30 giorni). |

## Heartbeat (monitor push)

| Metodo | Rotta | Autenticazione | Scopo |
|---|---|---|---|
| `GET` o `POST` | `/api/push/{token}` | nessuna (il token) | La chiamata che un job effettua a ogni esecuzione. `?status=up\|down&msg=…` facoltativo (compatibile con Uptime Kuma). `204` vuoto in caso di successo, `404` per un token sconosciuto, `400` per un altro `status`, `429` con `Retry-After` oltre sessanta chiamate al minuto per token. Vedi [Heartbeat](../devices/push.md). |
| `GET` | `/api/targets/{id}/push` | sessione | Il monitor di un dispositivo `push`: `token`, `path` (`/api/push/<token>`), `last_seen_at`, `last_seen_age_secs`, `last_status`, `last_message`, `received_total`, `expected_interval_secs`, `grace_secs`, `settings_error`, `verdict` (`waiting`, `on_time`, `missed`, `reported_down`). Creato alla prima lettura. `400` quando il dispositivo non è un heartbeat. |
| `POST` | `/api/targets/{id}/push/regenerate` | admin | Nuovo token; il precedente URL risponde `404` da quel momento. Stesso corpo della lettura. |

## Assistenti (MCP)

Il server [Model Context Protocol](https://modelcontextprotocol.io) integrato,
quello con cui parla un assistente. Si autentica con i token API di cui sopra —
stessi ambiti, scadenza, reti e limite di frequenza — e mai con una sessione.

| Metodo | Rotta | Autenticazione | Scopo |
|---|---|---|---|
| `POST` | `/api/mcp` | token API | JSON-RPC 2.0 sul trasporto Streamable HTTP, una richiesta per `POST`, con risposta in JSON. Protocollo `2026-07-28` (stateless: ogni richiesta porta la propria versione e le capacità del client in `_meta`, più gli header `MCP-Protocol-Version`, `Mcp-Method` e `Mcp-Name`; `server/discover`), e `2025-11-25`, `2025-06-18` e `2025-03-26` tramite l'handshake `initialize`. Nessuna sessione: nessun `Mcp-Session-Id` viene emesso, uno inviato viene ignorato. Una notifica riceve `202` e nessun corpo. |
| `GET`, `DELETE` | `/api/mcp` | pubblica | `405` con `Allow: POST` — non c'è alcuno stream inviato dal server né sessione da terminare; il corpo dice soltanto cos'è questo endpoint. |

`tools` è l'unica capacità — niente risorse, niente prompt, niente OAuth. Dei 27
strumenti, un token `read` può chiamare i 15 che si limitano a leggere
(`get_status`, `list_devices`, `get_device`, `list_alerts`, `alert_history`,
`query_metrics`, `list_silences`, `list_rules`, `list_device_types`,
`list_agents`, `list_containers`, `list_heartbeats`, `list_status_pages`,
`list_channels`, `list_packs`); i 12 che modificano qualcosa
(`silence_device`, `remove_silence`, `acknowledge_alert`, `probe_device`,
`set_device_enabled`, `set_rule_enabled`, `add_device`, `discover_network`,
`schedule_maintenance`, `restart_container`, `test_channel`,
`post_incident`) richiedono un token `write`. Gli strumenti chiamano lo stesso
codice dell'interfaccia web. Versioni, header, codici di errore e note di sicurezza
sono in [Collegare un assistente](../using/assistant.md#protocol-details).

## File dell'agent (fuori da `/api`)

| Metodo | Rotta | Scopo |
|---|---|---|
| `GET` | `/install.sh` | L'installer per Linux, macOS e FreeBSD. Pubblico. |
| `GET` | `/install.ps1` | L'installer per Windows. Pubblico. |
| `GET` | `/download/{name}` | `dumbmonit-agent-linux-x86_64`, `dumbmonit-agent-linux-aarch64`, `dumbmonit-agent-freebsd-x86_64`, `dumbmonit-agent-windows-x86_64.exe`, serviti da `DUMBMONIT_AGENT_DIR`; `{name}.sha256` fornisce il suo checksum in formato `sha256sum`. `404` se il file è assente. `dumbmonit-agent-macos-aarch64` e `dumbmonit-agent-macos-x86_64` (e i loro `.sha256`) rispondono `307` verso lo stesso file nell'ultima release di GitHub. Pubblico. |

Ogni altro percorso è servito dall'interfaccia web, che ti chiede da sé di accedere.
