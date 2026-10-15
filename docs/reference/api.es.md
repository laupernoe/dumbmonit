# API HTTP {#http-api}

Todo lo que hace la interfaz web pasa por `/api/*`, y la misma API está
abierta a tus propios scripts, dashboards y asistentes: no hay ninguna
mitad privada. Es JSON plano sobre HTTP, autenticado por una cookie de
sesión (la interfaz web) o por un token de API (todo lo demás).

!!! warning "Alpha"
    DumbMonit es una alfa y también lo es su API: las rutas y campos
    todavía pueden cambiar. La versión de la API (abajo) cambia cada
    vez que se elimina o renombra una ruta o campo, para que un
    cliente lo note.

## Descripción OpenAPI {#openapi-description}

Cada instancia describe su propia API en un documento
[OpenAPI 3.1](https://spec.openapis.org/oas/v3.1.0):

```
GET /api/openapi.json
```

Es público — no lleva ningún secreto, ninguna dirección ni nada sobre
tus monitores — y es la referencia exhaustiva: cada ruta que un
cliente puede llamar, sus parámetros, las formas de petición y
respuesta, las respuestas de error y el alcance que necesita (la
extensión `x-token-scope` en cada operación: `public`, `read`,
`write`, o `none` para rutas que un token nunca puede llamar). Las
tablas más abajo en esta página son un resumen legible de lo mismo.

Dejado fuera a propósito: el protocolo de agente (`/api/ingest`,
`/api/agent/commands*`, `/api/agent/relay*`), que solo habla el
[agente](../devices/agent.md); las redirecciones de navegador del
inicio de sesión SSO (`/api/auth/oidc/start`,
`/api/auth/oidc/callback`) y la de Spotify
(`/api/music/spotify/callback`); y las descargas de agente (`/install.sh`,
`/install.ps1`, `/download/*`).

Cárgalo en cualquier herramienta OpenAPI:

```bash
# Explóralo, y prueba llamadas con tu token, en Swagger UI
docker run --rm -p 8081:8080 \
  -e SWAGGER_JSON_URL=http://monit.lan:8080/api/openapi.json swaggerapi/swagger-ui
# luego abre http://localhost:8081 y pega "dmt_…" bajo Authorize → bearerAuth

# Genera una biblioteca de cliente (aquí Python)
docker run --rm -v "$PWD:/local" openapitools/openapi-generator-cli generate \
  -i http://monit.lan:8080/api/openapi.json -g python -o /local/dumbmonit-client

# O simplemente lista lo que hay
curl -s http://monit.lan:8080/api/openapi.json | jq -r '.paths | keys[]'
```

Postman e Insomnia lo importan desde la misma URL.

## Autenticación {#authentication}

Dos formas de acceso, aceptadas en cada ruta protegida:

- **Cookie de sesión** — lo que usa la interfaz web. Inicia sesión con
  una contraseña (o SSO) y envía la cookie HttpOnly
  `dumbmonit_session` (SameSite Lax; `Secure` cuando
  `DUMBMONIT_COOKIE_SECURE=1`; 30 días). Cada petición que cambia
  estado (`POST`, `PUT`, `DELETE`) también debe llevar
  `X-Requested-With: DumbMonit` — la prueba anti-CSRF que añade
  automáticamente la UI; sin ella la petición se rechaza con `403`.
- **Token de API** — `Authorization: Bearer dmt_…`, para scripts,
  dashboards y asistentes. Creado en Settings → **API & assistants**
  (los mismos tokens que usa el [servidor MCP](../using/assistant.md))
  y mostrado una vez. No hay ninguna cookie implicada, así que nunca se
  necesita ninguna cabecera `X-Requested-With`.

```bash
# Token de API: una cabecera, nada más
curl -H 'Authorization: Bearer dmt_…' http://monit.lan:8080/api/targets
curl -H 'Authorization: Bearer dmt_…' -X POST http://monit.lan:8080/api/targets/4/probe

# Sesión: inicia sesión (204 y una cabecera Set-Cookie), luego envía la cookie
curl -c cookies.txt -X POST http://monit.lan:8080/api/auth/login \
  -H 'content-type: application/json' \
  -d '{"username":"admin","password":"…"}'
curl -b cookies.txt http://monit.lan:8080/api/targets
curl -b cookies.txt -X POST http://monit.lan:8080/api/targets \
  -H 'X-Requested-With: DumbMonit' -H 'content-type: application/json' \
  -d '{"name":"NAS","address":"nas.lan","kind":"snmp","credential":{"type":"snmp_community","community":"public"}}'
```

Cuando una petición lleva un token bearer, la cookie se ignora: un
token revocado se rechaza incluso desde un navegador con sesión
iniciada, en lugar de recurrir silenciosamente a la sesión.

Los tokens de inscripción de agente (`dmon_…`) son otra cosa: solo
funcionan en las rutas de agente (`/api/ingest`,
`/api/agent/commands/*`, `/api/agent/relay*`). Dicen que una máquina
puede hablar, no *cuál* máquina: en esas rutas una máquina vinculada
también debe presentar su propio secreto de vinculación en
`X-DumbMonit-Agent-Secret` (`dmab_…`), que el servidor entrega una vez
en la inscripción. Consulta
[Vinculación](../devices/agent.md#binding-one-machine-one-agent).

El inicio de sesión tiene limitación de tasa: tras cinco intentos
fallidos, cada intento adicional se rechaza con `429` y un retraso
`Retry-After` que se duplica de 30 s hasta 5 minutos. Nunca adivines
contraseñas en un bucle.

## Alcances {#scopes}

Un token tiene uno de dos alcances. No hay un tercero, más poderoso:
lo que un administrador hace en su propia cuenta se mantiene fuera del
alcance de cualquier token.

| | token `read` | token `write` |
|---|---|---|
| Cada `GET` que puede hacer una cuenta *viewer* (monitores, alertas, métricas, reglas, canales sin sus secretos, páginas de estado…) | sí | sí |
| `POST`, `PUT`, `DELETE` que puede hacer un *administrador* (monitores, reglas, silencios, canales, páginas de estado, packs, sondas, descubrimiento…) | `403` | sí |
| `/metrics`, `/federate`, `/prometheus/*` ([sondeo](metrics.md#scraping-dumbmonit)) | sí | sí |
| Herramientas MCP de lectura / escritura ([asistentes](../using/assistant.md#what-the-assistant-can-call)) | solo lectura | ambas |
| Cuentas, sesiones, doble factor, SSO, registro de auditoría (`/api/auth/*`, `/api/users/*`) | `403` | `403` |
| Tokens de API y tokens de inscripción de agente (`/api/tokens/*`, `/api/agent/tokens/*`) | `403` | `403` |
| Copia de seguridad y restauración (`/api/backup/*`) | `403` | `403` |

Un token `read` en una ruta de escritura recibe `403` con un mensaje
que nombra el token y el alcance faltante. Como ningún token puede
listar, crear o revocar tokens, un token nunca puede acuñar otro —
mucho menos uno más poderoso.

## Seguridad de tokens {#token-security}

- **Se muestra una vez, se guarda hasheado.** Un token es `dmt_`
  seguido de 128 bits aleatorios. El servidor solo conserva su digest
  SHA-256 (y los primeros caracteres, `dmt_1a2b3c4d`, para
  reconocerlo en una lista); el token en claro existe solo en la
  respuesta a su creación. Los tokens presentados se comparan en
  tiempo constante.
- **Caducidad.** Elige 30 días, 90 días, 1 año o nunca al crearlo. Un
  token caducado recibe `401` con `The API token "<name>" expired on
  …` y `WWW-Authenticate: Bearer error="invalid_token"`; se mantiene en
  la lista, marcado como caducado, hasta que lo revocas.
- **Redes permitidas.** Opcionalmente restringe un token a una lista
  de direcciones o rangos CIDR (`192.168.1.0/24, 10.8.0.5`, 32
  entradas como máximo). Desde cualquier otro sitio recibe `403` con
  `The API token "<name>" cannot be used from <address>.` La dirección
  comprobada es la del cliente TCP, o — solo cuando la conexión viene
  de un proxy listado en `DUMBMONIT_TRUSTED_PROXIES` — la que reporta
  ese proxy en `X-Forwarded-For`. Detrás de un proxy inverso que no has
  declarado, cada petición parece venir del proxy: declárarlo, o la
  restricción rechazará a todos.
- **Último uso.** La lista de tokens muestra cuándo se usó cada token
  por última vez y desde qué dirección (actualizado como máximo una
  vez por minuto, o en cuanto cambia la dirección): un token que nadie
  usa ya es uno a revocar.
- **Vinculado a su creador.** Un token recuerda la cuenta que lo creó.
  Si esa cuenta está desactivada, sus tokens se rechazan (`401`); si se
  degrada a operador o viewer, sus tokens `write` actúan como tokens
  `read`; si se elimina, sus tokens se revocan.
- **La revocación** es inmediata: la siguiente llamada recibe `401`.
- **Registrado.** Cada escritura hecha con un token se escribe en el
  registro del servidor con el nombre e id del token, el método, la
  ruta (sin la cadena de consulta) y el estado — nunca el token, nunca
  el cuerpo. Crear y revocar un token se registran en el registro de
  auditoría (Settings → Security).

## Límites de tasa {#rate-limits}

Cada token puede hacer 120 llamadas por minuto, REST y MCP juntos. Más
allá de eso, las llamadas se rechazan con `429` y una cabecera
`Retry-After` (segundos) hasta que termina el minuto. Cada respuesta a
una petición autenticada con token te dice dónde estás:

| Cabecera | Significado |
|---|---|
| `RateLimit-Limit` | Llamadas permitidas por ventana (120). |
| `RateLimit-Remaining` | Llamadas restantes en la ventana actual. |
| `RateLimit-Reset` | Segundos hasta que la ventana vuelve a empezar. |

Un dashboard de Grafana con muchos paneles y un refresco corto puede
alcanzar el límite: dale su propio token, o alarga el refresco.

## Peticiones entre orígenes (CORS) {#cross-origin-requests-cors}

Una página servida desde otro origen — tu propio dashboard, Swagger
UI, un cliente MCP basado en navegador — puede llamar a la API **con
un token bearer, nunca con una sesión**:

- una respuesta lleva `Access-Control-Allow-Origin` solo cuando la
  petición llevaba `Authorization: Bearer dmt_…` (o para el
  `GET /api/openapi.json` público). `Access-Control-Allow-Credentials`
  nunca se envía, así que un navegador nunca adjunta la cookie de
  sesión a una llamada entre orígenes;
- un preflight (`OPTIONS`) en `/api/*` tiene éxito solo cuando las
  cabeceras anunciadas incluyen `authorization`: `204`, métodos
  `GET, POST, PUT, DELETE`, cabeceras
  `authorization, content-type, mcp-protocol-version, mcp-method, mcp-name`,
  cacheado 10 minutos. `X-Requested-With` no está entre ellas, así que
  una sesión de cookie nunca puede hacer una escritura desde otro
  origen;
- `Retry-After`, `WWW-Authenticate`, las cabeceras `RateLimit-*` y
  `X-DumbMonit-Api-Version` son legibles por el script que llama.

`DUMBMONIT_API_CORS_ORIGINS` define qué orígenes se permiten:

| Valor | Efecto |
|---|---|
| sin definir (por defecto) | Cualquier origen (`Access-Control-Allow-Origin: *`). Seguro, ya que solo un token bearer abre algo. |
| `https://grafana.example.com,https://dash.lan` | Solo estos orígenes, devueltos tal cual. |
| `off` | Ninguna cabecera CORS en absoluto: solo páginas del mismo origen y clientes que no son navegadores. |

La misma lista decide qué `Origin` acepta el [endpoint
MCP](../using/assistant.md#security-notes).

```js
// Desde una página en otro origen. Nunca envíes un token de escritura en una página pública.
const response = await fetch('https://monit.example.com/api/alerts', {
  headers: { Authorization: 'Bearer dmt_…' }
});
if (!response.ok) throw new Error((await response.json()).error);
const alerts = await response.json();
console.log(response.headers.get('RateLimit-Remaining'), 'calls left this minute');
```

## Versionado {#versioning}

Cada respuesta bajo `/api` lleva la versión de la API:

```
X-DumbMonit-Api-Version: 0.1
```

El mismo valor es `info.version` en el documento OpenAPI, junto a
`x-server-version`, la versión del servidor que respondió. Mientras la
API sea `0.x` no está congelada; la versión cambia cada vez que se
elimina o renombra una ruta o un campo. Añadir una ruta o un campo no
la cambia: ignora los campos que no conozcas.

## Errores {#errors}

Cada error es JSON: `{"error": "mensaje"}`, escrito para una persona.

| Estado | Cuándo |
|---|---|
| `400` | La petición está mal formada o un valor es inválido; el mensaje lo nombra. |
| `401` | Sin sesión o token válido: token faltante, desconocido, revocado o caducado (con `WWW-Authenticate: Bearer`), o un token cuyo creador está desactivado. |
| `403` | Autenticado, pero no permitido: un viewer, un operador fuera de la gestión de alertas, o un token `read` en una ruta de escritura, un token en una ruta de cuenta, token o copia de seguridad, un token usado desde una red no permitida, una escritura de cookie sin `X-Requested-With`, o cualquier escritura en la demo pública (con `"demo": true`). |
| `404` | El id no existe. Una ruta bajo `/api` que no coincide con ninguna ruta responde `404 {"error": "Unknown API route: …"}` en lugar del HTML de la interfaz web. |
| `405` | El método no existe en esa ruta (`GET /api/mcp`, por ejemplo). |
| `409` | Un conflicto con el estado actual (un duplicado, un comando ya en ejecución…). |
| `429` | Limitado por tasa, con `Retry-After`. |
| `500` | Un fallo del servidor; los detalles están en el registro del servidor, nunca en la respuesta. |

```bash
$ curl -i -H 'Authorization: Bearer dmt_revoked…' http://monit.lan:8080/api/targets
HTTP/1.1 401 Unauthorized
www-authenticate: Bearer
{"error":"A valid API token is required."}

$ curl -i -X DELETE -H 'Authorization: Bearer dmt_…read…' http://monit.lan:8080/api/targets/4
HTTP/1.1 403 Forbidden
{"error":"This action needs a token with the \"write\" scope; the token \"Grafana\" is \"read\" only. Create a write token in Settings → API & assistants."}

$ curl -i -H 'Authorization: Bearer dmt_…' http://monit.lan:8080/api/alerts   # la 121.ª llamada este minuto
HTTP/1.1 429 Too Many Requests
retry-after: 17
ratelimit-limit: 120
ratelimit-remaining: 0
{"error":"Rate limit reached (120 calls per minute per token). Retry in 17 s."}
```

El [endpoint MCP](../using/assistant.md#protocol-details) responde en
JSON-RPC en su lugar, con sus propios códigos de error.

## Cuentas, inicio de sesión y tokens {#accounts-sign-in-and-tokens}

En las tablas de abajo, *session* significa una cookie de sesión **o**
un token de API; *admin* significa una sesión de administrador o un
token `write`; *operator* significa una sesión de operador o
administrador, o un token `write`; *session only* significa una
cookie de sesión, nunca un token.

**Roles.** Una cuenta `viewer` hace cada `GET`. Un `operator` hace las
mismas lecturas más las escrituras de gestión de alertas marcadas
*operator* en esta página (y `x-roles: [admin, operator]` en el
documento OpenAPI): confirmar, aplazar (ventanas de mantenimiento),
ignorar una regla para un monitor, limpiar historial resuelto.
Cualquier otra escritura es solo para administradores y responde
`403` a un operador.

| Método | Ruta | Auth | Propósito |
|---|---|---|---|
| `GET` | `/api/auth/status` | público | `{"configured": bool, "authenticated": bool, "user": …, "oidc": {"enabled", "provider_name", "login_url"}}`. `configured: false` significa una instancia nueva: hasta que exista el primer administrador, cada ruta marcada *session* responde `401` — solo se puede alcanzar `status`, `setup`, `login` y `health`. |
| `POST` | `/api/auth/setup` | público | `{"setup_code": "…", "username": "admin", "password": "…"}`. Crea el primer administrador en una instancia nueva (al menos 12 caracteres); `username` puede omitirse y entonces es `admin`. `setup_code` es el código de un solo uso impreso en los registros del servidor al arrancar (o `DUMBMONIT_SETUP_CODE`); mayúsculas, guiones y espacios se ignoran. `204`; no abre una sesión. `401` para un código incorrecto, `429` tras demasiados, `409` una vez que existe una cuenta. |
| `POST` | `/api/auth/login` | público | `{"username": "…", "password": "…"}`. `204` con `Set-Cookie`. `401` con una contraseña incorrecta, `429` cuando se limita por tasa. Cuando la cuenta tiene el doble factor activado, `200` con `{"totp_required": true, "pending": "…"}` en su lugar — un ticket que vive cinco minutos y muere tras cinco códigos incorrectos: termina con `/api/auth/login/totp`. |
| `POST` | `/api/auth/login/totp` | público | `{"pending": "…", "code": "123456"}` — el ticket del primer paso y un código (o un código de recuperación). `204` con la cookie de sesión. |
| `GET` | `/api/auth/oidc/start` | público | Redirige el navegador al proveedor de identidad. |
| `GET` | `/api/auth/oidc/callback` | público | Retorno desde el proveedor (`code`, `state`): abre la sesión y redirige a la UI. |
| `GET` | `/api/auth/me` | solo session | La cuenta actual: `id`, `username`, `display_name`, `role` (`admin`, `operator`, `viewer`), `auth` (`password`, `oidc`), `disabled`, `totp_enabled`, `created_at`, `last_login_at`. |
| `POST` | `/api/auth/logout` | solo session | Termina la sesión y borra la cookie. |
| `POST` | `/api/auth/password` | solo session | `{"current_password": "…", "new_password": "…"}`. Cierra sesión en cada otra sesión. |
| `GET` | `/api/auth/totp` | solo session | Estado de doble factor de la cuenta actual: `enabled`, `pending`, `recovery_codes_left`. |
| `POST` | `/api/auth/totp/enroll` | solo session | `{"password": "…"}`. Propone un secreto: `secret` (base32), `otpauth_uri`, `issuer`, `account`. Nada se aplica hasta que se verifica. |
| `POST` | `/api/auth/totp/verify` | solo session | `{"code": "123456"}`. Confirma la inscripción y devuelve `{"recovery_codes": […]}` — mostrado una vez. |
| `DELETE` | `/api/auth/totp` | solo session | `{"password": "…"}`. Elimina el segundo factor. `204`. |
| `GET` | `/api/auth/audit?limit=200` | admin, solo session | Los eventos de seguridad más recientes: `id`, `at`, `actor`, `action` (`login`, `login.failed`, `password.changed`, `token.created`, `user.updated`, …), `subject`, `ip`. |
| `GET` | `/api/auth/oidc/config` | admin, solo session | Los ajustes de SSO y de dónde vienen (entorno o base de datos); el secreto de cliente nunca se devuelve. |
| `PUT` | `/api/auth/oidc/config` | admin, solo session | `{"issuer", "client_id", "client_secret", "provider_name", "scopes", "auto_create", "admin_groups", "groups_claim", "public_url"}`. Un `client_secret` ausente o vacío conserva el guardado; un `auto_create` ausente significa desactivado. |
| `DELETE` | `/api/auth/oidc/config` | admin, solo session | Olvida los ajustes de SSO guardados (las variables de entorno, si las hay, vuelven a aplicar). `204`. |
| `POST` | `/api/auth/oidc/test` | admin, solo session | Obtiene el documento de descubrimiento del proveedor y reporta lo que encontró. |
| `GET` | `/api/users` | admin, solo session | Cada cuenta, con la forma de `/api/auth/me`. |
| `POST` | `/api/users` | admin, solo session | `{"username", "display_name", "role", "password"}`. `password` puede omitirse solo cuando SSO está activado. `201`. |
| `PUT` | `/api/users/{id}` | admin, solo session | Cualquiera de `display_name`, `role`, `disabled`, `password`; un campo omitido conserva su valor. No puede degradar ni desactivar al último administrador. |
| `DELETE` | `/api/users/{id}` | admin, solo session | `204`. Ni a ti mismo, ni al último administrador. |
| `DELETE` | `/api/users/{id}/totp` | admin, solo session | Restablece el segundo factor de otra cuenta (un colega bloqueado). `204`. |
| `GET` | `/api/tokens` | solo session | Cada token de API: `id`, `name`, `prefix`, `scope`, `created_at`, `created_by` (la cuenta que lo creó), `expires_at` (`null`: nunca), `expired`, `allowed_networks` (vacío: cualquier sitio), `last_used_at`, `last_used_ip`, `revoked_at`. Nunca el secreto. |
| `POST` | `/api/tokens` | admin, solo session | `{"name": "Grafana", "scope": "read", "expires_in_days": 90, "allowed_networks": ["192.168.1.0/24"]}`. `name` es obligatorio (80 caracteres como máximo); `scope` es `read` (por defecto) o `write`; `expires_in_days` va de 1 a 3650, `null` o ausente para un token que nunca caduca; `allowed_networks` toma hasta 32 direcciones o rangos CIDR, guardados en forma de red (`192.168.1.7/24` se convierte en `192.168.1.0/24`), vacío o ausente para cualquier sitio. `201` con los campos del token más `secret` (mostrado una vez). `400` nombra una red inválida. |
| `DELETE` | `/api/tokens/{id}` | admin, solo session | Revocar. `204`; `404` cuando es desconocido o ya revocado. |

Un token tal como se lista:

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

## Cabeceras de respuesta {#response-headers}

Además de `X-DumbMonit-Api-Version` ([versionado](#versioning)), las
cabeceras `RateLimit-*` ([límites de tasa](#rate-limits)) y las
cabeceras CORS ([peticiones entre orígenes](#cross-origin-requests-cors)),
cada respuesta lleva `X-Content-Type-Options: nosniff`,
`Referrer-Policy: same-origin` y una `Content-Security-Policy`. La
política parte de `default-src 'none'` y solo abre lo que la interfaz
realmente usa — todo servido por el propio DumbMonit:

```
default-src 'none'; style-src 'self' 'unsafe-inline'; img-src 'self' data:;
font-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'self';
script-src 'self' 'nonce-<per response>'; frame-ancestors 'none'
```

No hay ningún origen de terceros en ella, y nunca debería haberlo: las
fuentes, la biblioteca de gráficas y cada asset se distribuyen dentro
del binario. Los scripts en línea de la página (el tema aplicado
antes del primer render, el arranque de SvelteKit) se permiten por un
nonce regenerado en cada respuesta, así que `'unsafe-inline'` nunca se
aplica a los scripts. Sí se aplica a los estilos, y no se puede
eliminar: Svelte define atributos `style=` en los elementos, que ni un
nonce ni un hash pueden cubrir.

Las páginas de estado públicas están hechas para incrustarse:
`/s/<slug>` y su vista compacta `/s/<slug>/embed` — y solo esas dos —
obtienen la misma política sin `frame-ancestors`. Todo lo demás, las
páginas de confirmación de suscripción y baja bajo `/s/` incluidas,
también lleva `X-Frame-Options: DENY`.

Si pones DumbMonit detrás de un proxy inverso, no dejes que añada una
segunda cabecera `Content-Security-Policy`: los navegadores aplican la
intersección de todas ellas, y una política por defecto de un proxy
sin el nonce deja una página en blanco.

## Salud {#health}

| Método | Ruta | Auth | Propósito |
|---|---|---|---|
| `GET` | `/api/health` | público | Versión del servidor y el estado de sus dependencias. Siempre `200`: un componente fallido está en el cuerpo. |
| `GET` | `/api/update` | session o token | Versión en ejecución, última versión, `update_available`; el servidor pregunta a GitHub como máximo una vez al día. `POST /api/update/check` comprueba ahora, `PUT /api/update/settings` activa o desactiva la comprobación. Consulta [Actualización](../install/update.md). |

```json
{"status":"ok","version":"0.1.0","database":{"ok":true},"victoria":{"ok":true,"embedded":true}}
```

`status` es `degraded` cuando un componente falla; ese componente
entonces lleva una cadena `error`. `victoria.embedded` es `true`
cuando el servidor ejecuta su propia VictoriaMetrics, `false` cuando
`DUMBMONIT_VM_URL` apunta a una externa.

## Copia de seguridad y restauración {#backup-and-restore}

| Método | Ruta | Auth | Propósito |
|---|---|---|---|
| `GET` | `/api/backup` | admin, solo session | Qué contendría un paquete (`contents`, una línea por sección con su recuento), la versión del formato del paquete, dónde vive el secreto de la instancia (`secret_source`: `file` o `environment`) y el estado de las copias de seguridad locales programadas (`schedule`). |
| `POST` | `/api/backup` | admin, solo session | `{"passphrase": "…", "include_account_secrets": false}`. Devuelve el propio paquete como un archivo JSON: una cabecera en texto plano (`format`, `version`, `created_at`, `summary`, `kdf`) y un `payload` cifrado con AES-256-GCM bajo una clave derivada de la frase de paso por Argon2id. Al menos 16 caracteres, o `400`. |
| `POST` | `/api/backup/restore` | admin, solo session | `{"bundle": {…}, "passphrase": "…", "apply": false}`. Simulación a menos que `apply` sea `true`; ambas ejecutan el mismo código, la simulación dentro de una transacción que se revierte. Responde el informe: por sección, `created`, `updated`, `skipped` y `notes`, más `warnings`. `400` con una frase de paso incorrecta, un archivo modificado, o un paquete escrito en una versión de formato más reciente. |
| `POST` | `/api/backup/local` | admin, solo session | Escribe una copia local al estilo programado ahora mismo y responde el nuevo `schedule`. |

Un paquete lleva cada credencial de la instancia, recifrada con la
frase de paso, así que estas rutas rechazan los tokens de API con una
explicación: solo un administrador con sesión iniciada en la interfaz
web obtiene una. Consulta [Copia de seguridad y
restauración](../install/backup.md).

## Tipos de monitor {#device-types}

| Método | Ruta | Propósito |
|---|---|---|
| `GET` | `/api/collectors` | Cada tipo de monitor que este servidor puede monitorizar: `kind`, `label`, `summary`, `examples`, `credential_types`, `address_hint`, `default_port`, `setup` (`title`, `steps`, `warning`, `doc_url`) y `options` (`key`, `label`, `help`, `placeholder`, `default`, `required`, `input`, `choices`). Los tipos traídos por un [pack de integración](../packs/index.md) se listan igual, como `pack.<id>`. |

## Packs de integración {#integration-packs}

Sesión de administrador o token de API (alcance `write` para cambios).
Consulta [Packs de integración](../packs/index.md).

| Método | Ruta | Propósito |
|---|---|---|
| `GET` | `/api/packs` | Packs instalados: `id`, `version`, `label`, `summary`, `kind` (`null` para un pack solo con perfiles SNMP), `enabled`, `installed_at`, `sha256`, `metrics`, `rules` (uids), `snmp_profiles`, `targets` (monitores que lo usan), `warnings`, y `error` cuando el pack guardado ya no pasa la validación. |
| `GET` | `/api/packs/{id}` | Un pack. |
| `POST` | `/api/packs` | Instalar o actualizar. Cuerpo: el YAML (`application/yaml`, `text/plain`…) o `{"yaml": "…"}` en JSON. `201` cuando se crea, `200` cuando se actualiza o no cambia, con `outcome` (`created`, `updated`, `unchanged`), `pack`, `rules_added` y `restart_required` (perfiles SNMP). `400` lista cada error de validación. |
| `PUT` | `/api/packs/{id}/enable`, `/api/packs/{id}/disable` | Registra o retira el tipo de monitor del pack; monitores, historial y reglas permanecen. |
| `DELETE` | `/api/packs/{id}` | Desinstalar, con sus reglas. `409` mientras monitores lo usen. |

## Targets (monitores) {#targets-devices}

| Método | Ruta | Propósito |
|---|---|---|
| `GET` | `/api/targets` | Lista cada monitor. |
| `POST` | `/api/targets` | Crea uno. `201` con el monitor. La detección de perfil empieza en segundo plano para SNMP. |
| `GET` | `/api/targets/{id}` | Un monitor. |
| `PUT` | `/api/targets/{id}` | Lo sustituye. Omitir `credential` conserva el guardado; omitir `profile_id` conserva el perfil detectado (`""` lo borra). Definir `enabled: false` limpia las alertas del monitor sin notificar. |
| `DELETE` | `/api/targets/{id}` | `204`. Limpia las alertas del monitor sin notificar y elimina sus series temporales de VictoriaMetrics (best effort). |
| `POST` | `/api/targets/{id}/probe` | Sondear ahora: `{"sample_count": 42, "series": ["if_octets_in{host=\"Core switch\",ifname=\"eth0\"}", …]}` — las claves de serie tal como las produjo el recolector, sin el prefijo `dumbmonit_` que añade el escritor en su camino hacia VictoriaMetrics. |
| `POST` | `/api/targets/{id}/discover` | Vuelve a ejecutar la detección de perfil: `{"profile_id": "host-resources"}` o `null`. |

Un monitor tal como se devuelve:

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

El propio secreto nunca se devuelve: `credential_kind` es una
etiqueta. Los timestamps del servidor son UTC sin sufijo.
`error_kind` clasifica `last_error`: `down` (el monitor no
respondió) o `config` (de nuestro lado — credenciales, dirección,
opción), que es lo que la UI muestra como *Unreachable* o
*Misconfigured*; `null` cuando la última sonda tuvo éxito. Una comprobación de
servicio (HTTP, TCP, NTP, WebSocket…) que se ejecutó pero encontró el servicio
fallando registra `Service down: the <probe> check failed (<reason>)` con
`error_kind` `down`.

Crear uno:

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

Formas de credencial (`type`): `none`; `snmp_community`
(`community`); `snmp_v3` (`username`, `auth: {protocol, passphrase}`
opcional con `md5`, `sha1`, `sha224`, `sha256`, `sha384` o `sha512`,
`privacy: {protocol, passphrase}` opcional con `des`, `aes128`,
`aes192` o `aes256`, `context` opcional); `api_token` (`token`, o las
dos mitades `token_id` y `secret`, que el servidor une en
`user@realm!name=secret` para Proxmox VE y PBS); `username_password`
(`username`, `password`).
`interval_secs` por defecto es 60 y no puede bajar de 10. `name` tiene
como máximo 200 caracteres y `address` como máximo 253 (`400` más
allá). `parent_id` debe nombrar un monitor existente (`400 Parent
device N not found.`). Las opciones de tipo van en `tags` bajo su
clave.

`via_agent` nombra el [agente relay](../install/remote-site.md) que
sondea el monitor desde su propia red en lugar del servidor;
`null` (por defecto) significa que el servidor lo sondea. Debe ser un
monitor `agent` (`400` en caso contrario), un monitor no
puede hacerse relay a sí mismo, y un monitor `agent` no puede ser
objeto de relay. En `PUT`, un `via_agent` *omitido* conserva el valor
guardado, como `profile_id` y `credential`; envía `null` para volver
al sondeo directo. `POST /api/targets/{id}/probe` y `/discover` en un
monitor con relay pasan por el relay y esperan su respuesta (`409`
cuando una sonda ya está en vuelo, `400 Timed out: relay agent …`
cuando no responde).

## Descubrimiento {#discovery}

| Método | Ruta | Auth | Propósito |
|---|---|---|---|
| `POST` | `/api/discovery` | admin | `{"cidr": "192.168.1.0/24", "community": "public", "port": 161, "timeout_ms": 1000}`. Escanea una red buscando dispositivos SNMP. `cidr` es obligatorio; `community` por defecto `public`; `timeout_ms` se limita a 100–10 000. Las redes más grandes de 4096 direcciones se rechazan. El escaneo es un `POST` con un cuerpo JSON para que la comunidad nunca aparezca en una URL o una línea de registro. |

```json
{
  "scanned": 254,
  "devices": [
    {"address": "192.168.1.10", "sysname": "sw-core", "sysdescr": "…", "sysobjectid": "1.3.6.1.4.1.8072.3.2.10", "suggested_profile": "host-resources"}
  ]
}
```

## Guía de primer uso {#first-run-guide}

Los tres pasos que muestra el resumen en una instancia nueva. El
estado se conserva en la instancia, no en el navegador, así que omitir
un paso lo omite en todas partes.

| Método | Ruta | Auth | Propósito |
|---|---|---|---|
| `GET` | `/api/onboarding` | session | Dónde está la instancia. |
| `PUT` | `/api/onboarding` | admin | `{"skipped": true}` retira toda la guía; `{"dismissed": ["test"]}` retira pasos individuales. Un campo omitido conserva su valor guardado; un id de paso distinto de `device`, `channel` o `test` es un `400`. Devuelve el mismo documento que `GET`. |

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

`has_target`, `has_channel` y `notification_confirmed` se recalculan
en cada lectura: son hechos sobre la instancia, no flags que define la
interfaz. `notification_confirmed` es verdadero una vez que un mensaje
realmente ha salido de algún canal — un mensaje de prueba cuenta, un
canal que solo se creó no.

`complete` se fija y nunca vuelve a falso. El servidor lo define
cuando los tres pasos están resueltos (hechos, o descartados), y
también en la primera lectura de una instancia que ya tenía un
monitor y un canal — así que actualizar una instancia establecida
nunca le muestra una guía de primer uso.

## Métricas {#metrics}

Ambas rutas hacen de proxy hacia VictoriaMetrics y devuelven series en
la forma Prometheus: `[{"metric": {etiquetas…}, "values":
[[timestamp_seconds, "value"], …]}]`.

| Método | Ruta | Propósito |
|---|---|---|
| `GET` | `/api/metrics/query?query=…` | Consulta MetricsQL instantánea. |
| `GET` | `/api/metrics/query_range?query=…&start=…&end=…&step=…` | Consulta de rango. `start` y `end` son **milisegundos** desde la época; `step` está en segundos y se ensancha para que una serie nunca supere los 2000 puntos. |

```bash
curl -b cookies.txt -G http://localhost:8080/api/metrics/query \
  --data-urlencode 'query=avg by (target, host) (dumbmonit_cpu_usage_percent)'
```

```json
[{"metric":{"host":"ThinkpadE14","target":"3"},"values":[[1757941205,"12.5"]]}]
```

## Sondeo (fuera de `/api`) {#scraping-outside-api}

Para un Prometheus o Grafana existente. Ambos toman un token de API
`read` en `Authorization: Bearer dmt_…` (una cookie de sesión también
funciona); un scraper no envía ninguna cabecera `X-Requested-With` y
no necesita ninguna — estas rutas solo leen. Consulta la [referencia
de métricas](metrics.md#scraping-dumbmonit) para un `scrape_config`
listo para copiar y la fuente de datos de Grafana.

| Método | Ruta | Propósito |
|---|---|---|
| `GET` | `/metrics` | Salud de la propia instancia en el formato de exposición de texto de Prometheus: scheduler, sondas, escrituras, alertas, notificaciones, agentes, base de datos, VictoriaMetrics. Unas pocas decenas de series, nunca una por monitor. |
| `GET` | `/federate` | Las mediciones, en el mismo formato. `match[]` selecciona series (repetible, 10 como máximo, `{__name__=~"dumbmonit_.*"}` por defecto), `max_lookback` amplía la ventana en la que se busca el último punto (`5m` por defecto). `400` más allá de 8 MiB, con qué limitar. |
| `GET`, `POST` | `/prometheus/api/v1/{route}` | La mitad de lectura de la API de Prometheus, retransmitida al almacén, para que Grafana pueda usar `http://…/prometheus` como una fuente de datos Prometheus. Solo `query`, `query_range`, `query_exemplars`, `series`, `labels`, `label/{name}/values`, `metadata` y `status/buildinfo`; cualquier otra cosa es `404`. |

`DUMBMONIT_METRICS_PUBLIC=true` sirve las tres sin token — solo detrás
de un cortafuegos, ya que llevan cada medición de cada monitor.

## Otras rutas {#further-routes}

Rutas no detalladas arriba; el documento OpenAPI da sus parámetros y formas de
respuesta. *read* y *write* son el alcance de token necesario; *none* significa
que no se acepta ningún token (solo sesión).

| Método | Ruta | Alcance | Propósito |
|---|---|---|---|
| `POST` | `/api/targets/reorder` | write | Reordena los monitores. |
| `GET` | `/api/targets/{id}/security` | read | Puntuación de seguridad, nota y comprobaciones de un monitor. |
| `GET` | `/api/security/summary` | read | Puntuación de seguridad de cada monitor evaluado, el peor primero. |
| `GET` | `/api/targets/{id}/webchange/pages` | read | Páginas seguidas por una vigilancia de sitio web. |
| `GET` | `/api/targets/{id}/webchange/changes` | read | Cambios encontrados en un sitio web vigilado, el más reciente primero. |
| `GET` | `/api/targets/{id}/webchange/changes/{change_id}` | read | Un cambio, con ambas versiones y el diff por líneas. |
| `GET` | `/api/targets/{id}/webchange/snapshots/{snapshot_id}/screenshot` | read | La captura de pantalla de una instantánea. |
| `POST` | `/api/targets/{id}/webchange/check` | write | Comprueba ahora un sitio web vigilado. |
| `GET` | `/api/targets/{id}/opnsense/gateways`, `/traffic`, `/health` | read | OPNsense: pasarelas y direcciones WAN; interfaces, cortafuegos y DHCP; sistema, servicios, túneles y firmware. |
| `GET` | `/api/targets/{id}/truenas/storage`, `/protection`, `/health` | read | TrueNAS: pools, datasets y discos; scrubs y tareas de protección de datos; sistema, alertas y servicios. |
| `GET` | `/api/targets/{id}/ad`, `/ad/findings` | read | Active Directory: dominio, controladores, grupos privilegiados y hallazgos; los hallazgos de seguridad en bruto. |
| `GET` | `/api/targets/{id}/redfish` | read | Hardware del servidor leído desde su controlador de gestión. |
| `GET` | `/api/targets/{id}/observability` | read | Salud de un servidor de logs o métricas. |
| `GET` | `/api/targets/{id}/backend` | read | Salud de Redis, MongoDB, RabbitMQ o CrowdSec. |
| `GET` | `/api/targets/{id}/proxy` | read | Salud de un proxy inverso o de un registro de dominio. |
| `GET` | `/api/public/status/{slug}/badge.svg`, `/response.svg`, `/uptime.svg` | público | Badges de estado general, tiempo de respuesta medio y disponibilidad de una página de estado. |
| `GET` | `/api/public/status/{slug}/components/{key}/badge.svg`, `/uptime.svg`, `/response.svg` | público | Los mismos badges para un servicio de la página. |
| `GET` | `/api/public/status/{slug}/banner.js` | público | Script del banner de estado para sitios web de terceros. |
| `GET` | `/api/public/status/{slug}/rss` | público | Incidentes como feed RSS. |
| `GET` | `/api/music/now` | read | Lo que reproducen los muros (monitor de sala). |
| `PUT`, `DELETE` | `/api/music/link` | write | Cada muro reproduce este enlace de Spotify, Deezer o YouTube; o detiene el reproductor integrado. |
| `GET`, `DELETE` | `/api/music/spotify` | read, write | La cuenta de Spotify conectada (nunca un token); la desconecta y olvida sus tokens. |
| `POST` | `/api/music/spotify/authorize`, `/complete` | none | Inicia la conexión de Spotify; termina una conexión por loopback. Solo sesión. |
| `GET` | `/api/music/spotify/token` | none | Un token de acceso de Spotify de corta duración para el reproductor del muro. Solo sesión. |
| `GET`, `PUT` | `/api/music/speaker` | read, write | El altavoz de Spotify del muro tal como lo muestra Settings; renombrarlo (admin). |
| `POST` | `/api/music/speaker/report`, `/play` | none | Un muro reporta el estado de su altavoz; reproduce la cuenta en él. Solo sesión. |

## Alertas {#alerts}

| Método | Ruta | Propósito |
|---|---|---|
| `GET` | `/api/alerts` | Alertas activas (pending, firing, suppressed, recientemente resueltas). Las alertas de monitores eliminados o en pausa nunca se listan. `?target_id=` conserva solo las alertas de un monitor. |
| `GET` | `/api/alerts/history?since=2026-09-01T00:00:00Z&limit=200&dismissed=false` | Transiciones de fase. `since` es RFC 3339, por defecto los últimos siete días; `limit` debe ser positivo. `dismissed` (por defecto `false`) excluye entradas limpiadas con las dos rutas de abajo; nada se destruye nunca con una limpieza, solo se oculta de la vista por defecto. |
| `POST` | `/api/alerts/history/{id}/dismiss` | Limpia una entrada `resolved` de la vista de historial por defecto. `204`. `404` en un id desconocido o una transición todavía activa. Operador; auditado. |
| `POST` | `/api/alerts/history/dismiss-resolved` | Limpia cada entrada `resolved` aún no limpiada de golpe («Clear all resolved»). Devuelve `{"dismissed": <count>}`. Operador; auditado. |
| `POST` | `/api/alerts/{fingerprint}/ack` | Confirmar: `{"duration_secs": 14400, "note": "…"}` o `{"until": "2026-09-22T18:00:00Z"}` (uno de los dos; ninguno significa 4 horas, como máximo 30 días). `{"until": null}` la levanta. Devuelve la alerta. Operador; auditado. |
| `DELETE` | `/api/alerts/{fingerprint}/ack` | Levanta la confirmación. Devuelve la alerta. Operador. |

Una alerta activa:

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

Una entrada de historial tiene `id`, `fingerprint`, `rule_uid`,
`target_id`, `from_phase`, `to_phase`, `severity`, `value`,
`notified`, `reason` (vacío, o por qué no se envió nada: `learning:
would have fired`, `suppressed: device 2 unreachable`, `maintenance
window`, `acknowledged by admin`, `device removed or disabled`), `at`
y `dismissed` (limpiada de la vista activa; la fila en sí se
conserva). `acked` es verdadero mientras `acked_until` está en el
futuro: los recordatorios y escaladas se pausan, la resolución igual
se notifica y borra la confirmación.
Un `target_id` que ya no existe se muestra como «(deleted device)» por
la UI.

### Reglas {#rules}

| Método | Ruta | Propósito |
|---|---|---|
| `GET` | `/api/alerts/rules` | Cada regla. |
| `POST` | `/api/alerts/rules` | Crear. `201`. |
| `PUT` | `/api/alerts/rules/{id}` | Actualizar. Una regla integrada conserva su consulta. |
| `DELETE` | `/api/alerts/rules/{id}` | `204`. Las reglas integradas no se pueden eliminar. |
| `POST` | `/api/alerts/rules/{id}/enable` | `{"enabled": false}`. Devuelve la regla. |

Una regla:

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

Crear una necesita al menos `name` y `query`; `kind` (`threshold`,
`anomaly`, `predict`), `operator`, `threshold`, `for_secs`, `severity`
(`info`, `warning`, `critical`), `selector` (`{"kind":"all"}`,
`{"kind":"ids","ids":[…]}` o `{"kind":"labels","labels":{…}}`),
`channels` (ids de canal; vacío significa cada canal activado),
`unit`, `repeat_secs`, `escalate_after_secs` y `enabled` son
opcionales.

### Silencios (ventanas de mantenimiento) {#silences-maintenance-windows}

| Método | Ruta | Propósito |
|---|---|---|
| `GET` | `/api/alerts/silences` | Cada ventana, con `active_now`, `active_until` y `next_start_at` (RFC 3339, calculado por el servidor). |
| `POST` | `/api/alerts/silences` | Crear. `201`. Operador. |
| `DELETE` | `/api/alerts/silences/{id}` | `204`. Operador. |

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

Una programación puntual es `{"kind": "once", "starts_at":
"2026-09-20T22:00:00Z", "ends_at": "2026-09-21T02:00:00Z"}`. Una
mensual es `{"kind": "monthly", "days": [1], "nth_weekdays": [{"nth":
1, "weekday": 6}], "start_minute": 120, "duration_minutes": 120,
"timezone": "Europe/Paris"}` — `nth` es 1 a 5, o `-1` para el último
del mes; un mes sin esa ocurrencia se omite. Los días son 0 = lunes …
6 = domingo (1–31 para un `days` mensual); los minutos son desde la
medianoche local (0–1439). En una programación recurrente, `timezone`
(un nombre IANA) gana sobre `utc_offset_minutes` y es lo que mantiene
a una ventana en su hora local a través de los cambios de horario de
verano; un nombre de zona desconocido se rechaza.

### Anulaciones por monitor {#per-device-overrides}

Una regla se puede ajustar para un monitor sin tocar la regla en
sí. `{"enabled": false}` es lo que escribe la acción **Ignore** de la
UI (en la alerta, la página de monitor o la regla): el monitor
se retira de esa única regla para siempre, de forma reversible, sin
desactivar la regla para nadie más.

| Método | Ruta | Propósito |
|---|---|---|
| `GET` | `/api/alerts/overrides?target_id=4` | Cada anulación, o las de un monitor: `rule_uid`, `target_id`, `threshold`, `clear_threshold`, `enabled`. |
| `GET` | `/api/alerts/rules/{id}/overrides` | Las anulaciones de una regla. |
| `PUT` | `/api/alerts/rules/{id}/overrides/{target_id}` | `{"threshold": 95, "clear_threshold": 90, "enabled": true}` — cada campo opcional; `null` significa «como la regla». Crea o sustituye. Un operador solo puede enviar `{"enabled": false}` (ignore); las anulaciones de umbral del monitor, si las hay, se conservan. |
| `DELETE` | `/api/alerts/rules/{id}/overrides/{target_id}` | `204`. Para un operador, solo levanta el ignore: las anulaciones de umbral permanecen. |
| `GET` | `/api/targets/{id}/rules` | The rules that watch a device: those whose query reads a metric it wrote in the last 24 h. `{"status": "ok" \| "no_data" \| "unavailable", "rules": [{id, uid, name, description, severity, enabled, ignored, custom_threshold, firing}]}`. |

## Política de notificación {#notification-policy}

| Método | Ruta | Propósito |
|---|---|---|
| `GET` | `/api/notify/policy` | La política global: `batch_window_secs` (0 = enviar de inmediato), `max_per_hour` por canal (0 = sin límite), `flap_events`, `flap_window_secs`, `flap_hold_secs` (0 = sin detección de flapping), `public_url` (enlaces en los mensajes; vacío = `DUMBMONIT_PUBLIC_URL`), `escalate_after_secs` y `escalate_channel` (0 / `null` = sin escalada). |
| `PUT` | `/api/notify/policy` | Los mismos campos, cada uno opcional: un campo omitido conserva su valor. `"escalate_channel": null` desactiva la escalada. Devuelve la política. |
| `POST` | `/api/notify/match-preview` | `{"matcher": {…}}` → qué monitores selecciona un filtro de enrutado de canal: `devices` (`id`, `name`, `kind`, `tags`, `matched`), `matched`, `total`, `rules`, `excluded_rules`. |

## Canales de notificación {#notification-channels}

| Método | Ruta | Propósito |
|---|---|---|
| `GET` | `/api/notify/kinds` | Cada tipo de canal con sus campos `settings` y `secrets` (`key`, `label`, `required`, `input`, `help`, `placeholder`, `options`, `shape`, `default`) y un `doc_url` como `https://dumbmonit.readthedocs.io/en/latest/notifications/#discord`. |
| `GET` | `/api/notify/channels` | Cada canal: `id`, `name`, `kind`, `enabled`, `settings`, `has_secret`, `last_error`, `last_sent_at`, `policy` (`min_severity`, `notify_resolved`, `min_interval_secs`, `quiet_hours`, `matcher`). Los secretos nunca se devuelven. |
| `POST` | `/api/notify/channels` | Crear. `201`. |
| `PUT` | `/api/notify/channels/{id}` | Actualizar. Omitir `secrets` conserva los guardados; `"secrets": {}` los borra. En `policy`, omitir `matcher` conserva el filtro de enrutado y `"matcher": null` lo borra. |
| `DELETE` | `/api/notify/channels/{id}` | `204`. |
| `POST` | `/api/notify/channels/{id}/test` | Envía un mensaje de prueba: `{"ok": true, "message": "…"}`. |

```json
{
  "name": "Ops Discord",
  "kind": "discord",
  "enabled": true,
  "settings": {"username": "DumbMonit"},
  "secrets": {"webhook_url": "https://discord.com/api/webhooks/…"}
}
```

Las claves exactas por tipo vienen de `/api/notify/kinds` y están
documentadas en [Canales de notificación](../notifications.md).

### Dispositivos Web Push {#web-push-devices}

Los dispositivos de la **cuenta con sesión iniciada** (solo session,
viewers incluidos; los tokens de API reciben `401`). Un canal
`webpush` les envía; consulta [Notificaciones push
(PWA)](../using/push-notifications.md).

| Método | Ruta | Propósito |
|---|---|---|
| `GET` | `/api/webpush` | `{"public_key": "…", "devices": […]}`: la clave pública VAPID (base64url, el `applicationServerKey` con el que suscribirse) y los dispositivos de la cuenta — `id`, `device` (`Firefox on Android`), `push_service` (host), `fingerprint` (primeros 16 caracteres hex del SHA-256 del endpoint), `created_at`, `last_success_at`, `last_error`. El endpoint, las claves del navegador y la clave privada nunca se devuelven. |
| `POST` | `/api/webpush/subscriptions` | `PushSubscription.toJSON()`: `{"endpoint": "https://…", "keys": {"p256dh": "…", "auth": "…"}}`. `201` con el dispositivo. El endpoint debe ser HTTPS en una dirección pública: direcciones privadas, loopback y link-local reciben `400`. El mismo endpoint de nuevo sustituye su fila. |
| `DELETE` | `/api/webpush/subscriptions/{id}` | `204`; `404` para un dispositivo de otra cuenta. |
| `POST` | `/api/webpush/test` | `{}` o `{"id": 3}`. Envía una notificación de prueba: `{"ok", "delivered", "failed", "removed", "message"}`. Las suscripciones respondidas con `404`/`410` se eliminan. |

## Informes por correo {#email-reports}

Solo administradores, `GET` incluido (la respuesta lista
destinatarios). Los tokens bearer necesitan el alcance `write` para
cambios. Consulta [Informes por correo](../using/reports.md).

| Método | Ruta | Auth | Propósito |
|---|---|---|---|
| `GET` | `/api/reports/schedules` | admin | Cada informe: `id`, `name`, `enabled`, `frequency`, `weekday` (0 es lunes), `day_of_month`, `hour`, `timezone`, `recipients`, `channel_id`, `last_sent_at`, `last_error`, `next_run_at`. |
| `POST` | `/api/reports/schedules` | admin | Los mismos campos escribibles; cada uno por defecto semanal, lunes, 8:00, `UTC`. `recipients` contiene hasta 20 direcciones como máximo y es obligatorio para activar el informe; `channel_id` debe ser un canal `smtp` o `null`. `201`; `409` más allá de 10 informes. |
| `GET` | `/api/reports/schedules/{id}` | admin | Un informe. |
| `PUT` | `/api/reports/schedules/{id}` | admin | Sustitución completa. |
| `DELETE` | `/api/reports/schedules/{id}` | admin | `204`. |
| `POST` | `/api/reports/schedules/{id}/send-test` | admin | Envía por correo una vista previa a los destinatarios guardados: `{"sent": 2, "failed": 0}`. No toca la programación. `400` sin un canal de correo usable; `409` dentro de los 30 segundos de la vista previa anterior. |

## Páginas de estado e incidentes {#status-pages-and-incidents}

| Método | Ruta | Auth | Propósito |
|---|---|---|---|
| `GET` | `/api/status-pages` | session | Cada página con sus elementos: `page` (`id`, `slug`, `title`, `description`, `published`, `theme`, `show_uptime_days`, `created_at`, `updated_at`) e `items` (`id`, `page_id`, `target_id`, `label`, `group_name`, `position`). |
| `POST` | `/api/status-pages` | admin | `{"title", "slug", "description", "published", "theme", "show_uptime_days"}`. `title` es obligatorio (120 caracteres como máximo); `slug` (`^[a-z0-9-]{2,40}$`) se deriva del título cuando se omite; `theme` es `auto` (por defecto), `light` u `dark`; `show_uptime_days` va de 7 a 90 (90 por defecto). `201`; `409` en un slug ya ocupado. |
| `GET` | `/api/status-pages/{id}` | session | Una página con sus elementos. |
| `PUT` | `/api/status-pages/{id}` | admin | Los mismos campos, pero una sustitución completa en lugar de un parche: `title` es obligatorio y cada campo omitido vuelve a su valor por defecto (slug rederivado del título, descripción vacía, `published: false`, `theme: auto`, 90 días de disponibilidad). |
| `DELETE` | `/api/status-pages/{id}` | admin | `204`. La URL pública deja de responder. |
| `POST`, `PUT` | `/api/status-pages`, `/api/status-pages/{id}` — look y suscripciones | admin | También aceptan `accent` (`default`, `blue`, `teal`, `violet`, `rose`, `amber`), `scenes` (ids de escena de la lista del editor — ciudades, las nuevas maravillas y las maravillas antiguas, p. ej. `venice`, `paris`, `giza`, `tajmahal`, sin duplicados; `[]` para ninguna; omitido en la creación: `["paris"]`), `simple` (booleano, por defecto `false`: la página pública no muestra escena, mascota ni animación y sigue la preferencia claro/oscuro del visitante), `scene_rotation` (`visit`, `1m`, `10m` o `1h`), `footer_text` (texto plano, 280 caracteres), `homepage_url` (`http(s)://` o vacío), `subscribe_channel_id` (un canal `smtp`, o `null` para desactivar la suscripción por correo) y `domain` (el nombre de host público de la página, como `status.example.com`, o `null`/`""` para ninguno — consulta [Dominio personalizado](../using/status-pages.md#custom-domain)). `domain` se pasa a minúsculas y pierde un prefijo `https://` o una barra final; un puerto, ruta o dirección IP es `400`, la dirección del propio DumbMonit (su URL pública, o el host en el que llegó la petición) es `400`, y un nombre que usa otra página es `409`. A diferencia de los demás campos, estos ocho conservan su valor guardado cuando se omiten en un `PUT`. La vista de página los devuelve con `logo_type` (`image/png`, `image/jpeg`, `image/webp` o `null`). |
| `GET`, `PUT`, `DELETE` | `/api/status-pages/{id}/logo` | session / admin | El logo de la página. `PUT` toma `{"data": "<base64 o URL data:>"}`: PNG, JPEG o WebP reconocido por contenido, 256 KiB como máximo, cualquier otra cosa `400`. `204`. `GET` es la vista previa del editor (borradores incluidos). |
| `GET` | `/api/status-pages/{id}/subscribers` | session | Suscriptores por correo: `id`, `page_id`, `email`, `confirmed_at` (`null` mientras está pendiente), `created_at`. Sus tokens nunca salen del servidor. |
| `DELETE` | `/api/status-pages/{id}/subscribers/{subscriber}` | admin | Elimina una dirección. `204`. |
| `PUT` | `/api/status-pages/{id}/items` | admin | `[{"target_id": 4, "label": "NAS", "group_name": "Storage"}, …]` — la lista completa y ordenada de monitores mostrados en la página. |
| `GET` | `/api/incidents` | session | Cada incidente con sus actualizaciones: `incident` (`id`, `page_id`, `title`, `kind`, `status`, `severity`, `starts_at`, `ends_at`, `created_at`, `updated_at`) y `updates`. |
| `POST` | `/api/incidents` | admin | `{"title", "kind", "status", "severity", "page_id", "starts_at", "ends_at", "body"}` — `body` es la primera actualización. `201`. |
| `PUT` | `/api/incidents/{id}` | admin | Los mismos campos; un campo omitido conserva su valor. |
| `DELETE` | `/api/incidents/{id}` | admin | `204`. |
| `GET` | `/api/incidents/{id}/updates` | session | La cronología: `id`, `incident_id`, `status`, `body`, `created_at`. |
| `POST` | `/api/incidents/{id}/updates` | admin | `{"status": "monitoring", "body": "…"}`. Añade una actualización y mueve el incidente a `status`. `201`. |
| `GET` | `/api/public/status/{slug}` | público | El documento JSON a partir del cual se construye una página publicada (también `badge.svg` y `rss` bajo la misma ruta). En `page`, `show_uptime_days` es el máximo configurado e `history_days` la ventana realmente mostrada (la antigüedad de los datos de la página, desde 7 días hasta ese máximo) y `simple` refleja el modo simple. Consulta [Páginas de estado](../using/status-pages.md). En el dominio personalizado de una página (`Host` igual a su `domain`), solo responden las rutas `/api/public/status/{slug}/…` de esa página; cualquier otra ruta de esta referencia es `404` ahí. |
| `GET` | `/api/public/status/{slug}/uptime.svg`, `/response.svg` | público | Badges de página: disponibilidad media sobre `?days=` `1`, `7`, `30` (por defecto) o `90` — `400` más allá del historial de la página — y tiempo de respuesta medio. SVG, `Cache-Control: public, max-age=60`. |
| `GET` | `/api/public/status/{slug}/components/{key}/badge.svg`, `/uptime.svg`, `/response.svg` | público | Los mismos tres badges para un servicio; `key` es el `key` del servicio en el documento público (su label en forma de URL). `404` para una key desconocida. |
| `GET` | `/api/public/status/{slug}/logo` | público | El logo de la página con su tipo de imagen, cacheable por un día (el `logo_url` del documento lleva una versión). `404` sin uno. |
| `POST` | `/api/public/status/{slug}/subscribe` | público | `{"email": "…"}`. Envía un enlace de confirmación por el canal SMTP de la página. Siempre `202` con el mismo mensaje, sea cual sea el estado de la dirección; `400` para una dirección inválida; `404` cuando la página no ofrece suscripción por correo; `409` cuando la página no puede aceptar más suscriptores; `429` con `Retry-After` más allá de cinco peticiones por cliente y sesenta en total por 15 minutos. |
| `POST` | `/api/public/status/{slug}/confirm?token=…` | público | Confirma una suscripción (el enlace del correo de confirmación abre `/s/{slug}/confirm`, cuyo botón publica aquí). `404` para un token desconocido o caducado. |
| `POST` | `/api/public/status/{slug}/unsubscribe?token=…` | público | Elimina una suscripción; el cuerpo se ignora, así que también es el destino de un clic RFC 8058 de `List-Unsubscribe`. Siempre `200`. |

## Tokens de agente e ingesta {#agent-tokens-and-ingest}

| Método | Ruta | Auth | Propósito |
|---|---|---|---|
| `GET` | `/api/agent/tokens` | solo session | Cada token: `id`, `name`, `prefix`, `created_at`, `last_used_at`, `revoked_at`, `max_uses` (`null` para un token de flota), `uses`, `expires_at`. |
| `POST` | `/api/agent/tokens` | admin, solo session | `{"name": "Home fleet", "base_url": "http://server:8080", "reusable": false, "max_uses": null, "expires_in_days": null}`. `201` con los campos del token más `secret` (mostrado una vez), `install_linux` e `install_windows`. `base_url` es la URL que usarán los agentes; por defecto es la dirección de escucha. Sin `reusable`, el token es de **un solo uso**: inscribe una máquina y no más. Con él, `max_uses` limita las inscripciones (`null`: sin límite). `expires_in_days` detiene las *inscripciones* tras esos días; las máquinas ya inscritas siguen reportando. `400` en un recuento por debajo de 1. |
| `DELETE` | `/api/agent/tokens/{id}` | admin, solo session | Revocar. `204`; `404` cuando es desconocido o ya revocado. |
| `POST` | `/api/ingest` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Recibe un lote de muestras de un agente (cuerpos hasta 16 MB). No pensado para llamarse a mano. La cabecera lleva el secreto de vinculación de la máquina; un agente que no tiene ninguno la omite, que es cómo pide uno. La respuesta añade `agent_secret` (el secreto, devuelto exactamente una vez, cuando la máquina se vincula) y `bound`. `403` cuando la máquina está vinculada a otra instalación de agente, o cuando el token ya no puede inscribir — el mensaje dice cuál y qué hacer. |
| `GET` | `/api/agent/relay?key=…&wait=N` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Sondas delegadas a un agente relay (`relay: true`). Retenida hasta `wait` segundos (25 como máximo) cuando no hay nada pendiente. Cada elemento es un comando de tipo `probe` cuyos `args` llevan el target, su credencial descifrada, `timeout_secs` y `discover`. Nunca se escribe en disco. |
| `POST` | `/api/agent/relay/{id}?key=…` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Resultado de una sonda delegada: `{"duration_ms", "error", "samples", "profile_id"}` (cuerpos hasta 16 MB). `204`; `404` cuando la sonda caducó o pertenece a otro agente. |
| `GET` | `/api/agent/commands?key=…&wait=N` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Comandos en cola para un agente (reinicio o actualización de contenedor), retenidos hasta `wait` segundos cuando no hay nada pendiente. No pensado para llamarse a mano. `key` sola no prueba nada — es pública — así que una máquina vinculada también debe presentar su secreto de vinculación; `403` en caso contrario, que es lo que impide que una máquina tome los comandos de otra. |
| `POST` | `/api/agent/commands/{id}?key=…` | `Authorization: Bearer dmon_…` + `X-DumbMonit-Agent-Secret` | Progreso y resultado de un comando, reportado por el agente. `result` se trunca a los últimos 4 KB, con `[truncated by the server, beginning dropped]` al principio cuando eso ocurre. |
| `GET` | `/api/relays` | session | Cada monitor de agente como posible relay: `id`, `name`, `site`, `relay` (declarado `relay: true`), `last_seen_at`, `relayed` (monitores alcanzados a través de él). Relays primero, luego por nombre. |

```json
{
  "id": 1, "name": "Home fleet", "prefix": "dmon_ab12",
  "created_at": "2026-09-15 13:00:00", "last_used_at": null, "revoked_at": null,
  "secret": "dmon_…",
  "install_linux": "curl -sSL http://server:8080/install.sh | sh -s -- --token=dmon_… --url=http://server:8080",
  "install_windows": "& ([scriptblock]::Create((irm http://server:8080/install.ps1))) -Token dmon_… -Url http://server:8080"
}
```

## Contenedores y comandos (monitores de agente) {#containers-and-commands-agent-devices}

Acciones sobre los contenedores de una máquina que ejecuta el agente.
Cada ruta responde `404` cuando el monitor no es un target
`agent`.

| Método | Ruta | Propósito |
|---|---|---|
| `GET` | `/api/targets/{id}/agent` | La máquina tal como la describió su agente por última vez: `hostname`, `os`, `os_version`, `arch`, `agent_version`, `commands_supported`, `relay`, `site`, `relayed` (monitores alcanzados a través de este agente), `last_seen_at`, `binding`, `bound`, `bound_at`, `rebind_until`. `404` hasta que un agente haya reportado. `commands_supported` es `true` solo cuando el agente declaró que obtiene comandos (`commands: true`, lo por defecto de los agentes actuales); un agente más antiguo o uno con `commands: false` da `false`. `binding` es `bound`, `pending` (el binario se puede vincular y lo hará en su siguiente lote) o `unsupported` (un agente más antiguo que la vinculación: reinstálalo). |
| `POST` | `/api/targets/{id}/agent/rebind` | Admin. Abre una ventana de una hora durante la cual esta máquina puede volver a vincularse con un token de inscripción válido — la vía de vuelta tras una reinstalación que se llevó el secreto del agente. `{"rebind_until", "minutes"}`. La ventana se cierra en cuanto se usa, y el secreto antiguo deja de funcionar. `404` hasta que un agente haya reportado. |
| `GET` | `/api/targets/{id}/containers` | Cada contenedor del último lote: `name`, `image`, `up`, `health`, `restart_count`, `uptime_seconds`, `image_age_seconds`, `update_available`, `policy`, `last_command`. |
| `PUT` | `/api/targets/{id}/containers/{name}/policy` | `{"auto_restart": bool, "auto_update": bool, "prune_old_image": bool, "only_in_maintenance": bool}`. Cada campo es opcional: un campo omitido conserva su valor guardado. Devuelve la política completa. |
| `POST` | `/api/targets/{id}/containers/{name}/restart` | Encola un reinicio. `201` con el comando. `404` cuando `name` no está en el inventario del agente; `409` cuando el mismo comando ya está en cola o en ejecución, o cuando el agente no puede ejecutar comandos (consulta `commands_supported`). |
| `POST` | `/api/targets/{id}/containers/{name}/update` | Encola una actualización; `{"prune": bool}` opcional (por defecto según la política). Mismos códigos de estado que restart. |
| `GET` | `/api/targets/{id}/commands` | Los últimos 20 comandos, los más recientes primero: `id`, `kind` (`container.restart`, `container.update`), `args`, `status`, `requested_by` (un usuario, o `policy`), `created_at`, `started_at`, `finished_at`, `result`. |
| `DELETE` | `/api/targets/{id}/commands/{command_id}` | Cancela un comando en cola. `204`; `409` una vez que el agente lo ha recogido o cuando ya está cerrado; `404` cuando no pertenece a este monitor. |

`status` pasa por `queued → running → done | failed`. Existen otros dos
estados finales: `cancelled` (una persona retiró el comando) y
`expired` (el servidor se rindió tras diez minutos porque ningún
agente vino a buscarlo — agente detenido, demasiado antiguo para el
canal de comandos, o instalado con `commands: false`). El servidor
hace caducar los comandos obsoletos cada minuto por su cuenta; un
comando en cola nunca bloquea uno nuevo por más tiempo que eso.

## Invitados de Proxmox VE {#proxmox-ve-guests}

Las filas del panel **Guests** de un monitor Proxmox VE, montadas
a partir de la última sonda en el almacén de métricas (cuatro
consultas instantáneas, nada se pide al hipervisor). `404` cuando el
monitor no existe, `400` cuando no es un target `proxmox`.

| Método | Ruta | Propósito |
|---|---|---|
| `GET` | `/api/targets/{id}/proxmox/guests` | Un objeto por VM o contenedor, ordenados por nodo y luego VMID: `vmid`, `name`, `node`, `kind` (`qemu`, `lxc`), `status` (`running`, `stopped`, `paused`, `suspended`, `template`, `unknown`), `cpu_percent` (de los núcleos asignados), `cpu_count`, `memory_used_bytes`, `memory_total_bytes`, `memory_percent`, `balloon_bytes`, `disk_used_bytes`, `disk_total_bytes`, `disk_percent`, `agent` (`true` el agente invitado respondió, `false` activado pero en silencio, `null` ninguno), `network_in_bps`, `network_out_bps`, `disk_read_bps`, `disk_write_bps`, `uptime_seconds`, `last_backup_age_seconds`, `ha_state`, `pool` (su pool de recursos), `lock` (solo mientras se mantiene un bloqueo: `backup`, `migrate`…), `os` e `ip` (visto desde dentro del invitado, refrescado por hora). Cada valor desconocido es `null` — una VM sin agente invitado tiene `disk_total_bytes` pero `disk_used_bytes: null`; un invitado detenido conserva sus tamaños y pierde sus mediciones. |

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

## Nodos y Ceph de Proxmox VE {#proxmox-ve-nodes-and-ceph}

Las secciones **Nodes** y **Ceph** de la misma página de monitor,
leídas de la misma última sonda. Mismos códigos de estado que arriba.

| Método | Ruta | Propósito |
|---|---|---|
| `GET` | `/api/targets/{id}/proxmox/nodes` | `{"nodes": […], "fencing_state", "fencing_armed"}` — el estado del watchdog de HA (`armed`, `standby`…) y si realmente aislaría a un nodo perdido, luego un objeto por nodo, ordenados por nombre: `name`, `up`, `cpu_percent`, `cpu_iowait_percent`, `memory_percent`, `ksm_shared_bytes`, `rootfs_percent`, `uptime_seconds`, `version` (la versión de `pve-manager` instalada en ese nodo), `services_down` (nombres de unidad de los demonios detenidos o fallidos), `interfaces_offline` (interfaces configuradas para arrancar al inicio que no están activas), `thin_pools` (`name`, `vg`, `used_percent`, `metadata_used_percent`, `size_bytes`) y `volume_groups` (`name`, `used_percent`, `size_bytes`). Un nodo que dejó de responder conserva su entrada con `up: false`. |
| `GET` | `/api/targets/{id}/proxmox/ceph` | `available` (`false` cuando el clúster no tiene Ceph — la UI entonces no dibuja ninguna sección), `health` (0 OK, 1 WARN, 2 ERR, 3 desconocido), `health_status`, `bytes_used`, `bytes_total`, `used_percent`, `osds_total`, `osds_up`, `osds_in`, `osds` (`name`, `host`, `device_class`, `up`, `in`, `used_percent`, `used_bytes`, `total_bytes`, `apply_latency_ms`, `commit_latency_ms`), `pools` (`name`, `used_percent`, `used_bytes`, `size`, `min_size`, `pg_num`, `pg_num_optimal`, `autoscale`), `filesystems` (nombres CephFS), `flags` (flags de OSD activas ahora, como `noout`) y `muted_checks` (comprobaciones de salud silenciadas, que `HEALTH_OK` ya no menciona). |

## Proxmox Backup Server

Los paneles de un monitor `pbs`, leídos de la última sonda (nada
se pide al servidor salvo el registro de tarea). `404` cuando el
monitor no existe, `400` cuando no es un target `pbs`.

| Método | Ruta | Propósito |
|---|---|---|
| `GET` | `/api/targets/{id}/pbs/calendar?days=30&offset=120` | `{"probed_at", "days", "offset_minutes", "groups": […]}`: una entrada por grupo de copia (`datastore`, `namespace`, `backup_type`, `backup_id`, `name`, `count`, `last_time`, `last_size`, `last_verified`, `last_success`, `last_failure`, `retention`) con un array `days` de `{date, state, runs, snapshot}`. `offset` es el desfase UTC del visor en minutos, para que los días se corten en la medianoche local. |
| `GET` | `/api/targets/{id}/pbs/failures?days=30` | Tareas fallidas: `upid`, `worker_type`, `kind`, `worker_id`, `datastore`, `start`, `end`, `error`. |
| `GET` | `/api/targets/{id}/pbs/jobs` | `{"probed_at", "jobs": […]}`: jobs de sync, verify, prune y recolección de basura con su programación y último resultado. |
| `GET` | `/api/targets/{id}/pbs/health` | Datastores (uso, fecha estimada de llenado), discos (SMART, desgaste) y pools ZFS. |
| `GET` | `/api/targets/{id}/pbs/tasks/{upid}/log?lines=200` | El registro de una tarea, obtenido del servidor: `{"upid", "lines": […]}`; `lines` limita cuántas se devuelven. |
| `GET` | `/api/targets/{id}/pbs/disks/smart?disk=/dev/sda` | Atributos SMART de un disco; `disk` es obligatorio (`400` sin él). |
| `GET` | `/api/targets/{id}/pdm/remotes` | Proxmox Datacenter Manager: totales del patrimonio (`estate`) y una fila por instancia federada (`id`, `kind`, `reachable`, `error`, `version`, `version_behind`, recuentos de nodo y invitado, memoria y almacenamiento, `subscription`, `last_collection`, `tasks_failed`), las inaccesibles primero. |
| `GET` | `/api/targets/{id}/pdm/failures?days=14` | Tareas que fallaron en todo el patrimonio: `upid`, `remote`, `worker_type`, `kind`, `worker_id`, `node`, `start`, `end`, `error`. |
| `GET` | `/api/targets/{id}/pdm/health` | El host de la consola: CPU, memoria, sistema de archivos raíz, tiempo de actividad, certificados, actualizaciones pendientes y suscripción. |
| `GET` | `/api/targets/{id}/pmg/queues` | Proxmox Mail Gateway: las cuatro colas de Postfix en orden de lectura (`queue`, `messages`, `domains`, `oldest_age_seconds`, `top_domains`, `stuck`), más `total_messages` y un `stuck` de toda la pasarela. `oldest_age_seconds` es un límite inferior: qshape reporta rangos de antigüedad. |
| `GET` | `/api/targets/{id}/pmg/traffic` | Los totales de hoy (`mail`), la curva de tráfico reciente (`recent`), el histograma de puntuación de spam, los virus detectados hoy, y recuentos de cuarentena (`quarantine`). Solo recuentos: ningún contenido de mensaje. |
| `GET` | `/api/targets/{id}/pmg/health` | Una entrada por nodo (sistema, servicios, `signatures` con `family`, `age_seconds` y `stale`, `expiring_certificates`, actualizaciones, suscripción), los miembros del `cluster` con su estado de sincronización, y `stopped_services` entre nodos. |

## Synology

| Método | Ruta | Propósito |
|---|---|---|
| `GET` | `/api/targets/{id}/synology` | Resumen de un monitor `synology` de la última sonda: `system` (modelo, versión DSM, tiempo de actividad, temperatura, CPU, memoria), `volumes` (`name`, `fs_type`, `raid_type`, `status`, `severity`, `total_bytes`, `used_bytes`), `disks` (`model`, `serial`, `kind`, `ssd`, `status`, `smart_status`, `temperature_celsius`, `size_bytes`, `remaining_life_percent`, …) y `sampled_at`. `404` cuando el monitor no existe, `400` cuando no es un Synology. |
| `GET` | `/api/targets/{id}/synology/abb` | Active Backup for Business: `tasks`, y `devices` — un informe por dispositivo protegido con su ritmo aprendido en `assessment` (`state`, `last_success_s`, `typical_interval_s`, recuentos de 30 días). |

## Heartbeats (monitores push) {#heartbeats-push-monitors}

| Método | Ruta | Auth | Propósito |
|---|---|---|---|
| `GET` o `POST` | `/api/push/{token}` | ninguna (el token) | La llamada que hace un trabajo cada vez que se ejecuta. Opcional `?status=up\|down&msg=…` (compatible con Uptime Kuma). `204` vacío en éxito, `404` para un token desconocido, `400` para otro `status`, `429` con `Retry-After` más allá de sesenta llamadas por minuto por token. Consulta [Heartbeat](../devices/push.md). |
| `GET` | `/api/targets/{id}/push` | session | El monitor de un monitor `push`: `token`, `path` (`/api/push/<token>`), `last_seen_at`, `last_seen_age_secs`, `last_status`, `last_message`, `received_total`, `expected_interval_secs`, `grace_secs`, `settings_error`, `verdict` (`waiting`, `on_time`, `missed`, `reported_down`). Creado en la primera lectura. `400` cuando el monitor no es un heartbeat. |
| `POST` | `/api/targets/{id}/push/regenerate` | admin | Token nuevo; la URL anterior responde `404` a partir de entonces. Mismo cuerpo que la lectura. |

## Asistentes (MCP) {#assistants-mcp}

El servidor [Model Context Protocol](https://modelcontextprotocol.io)
integrado, el que habla un asistente. Se autentica con los tokens de
API de arriba — mismos alcances, caducidad, redes y límite de tasa — y
nunca con una sesión.

| Método | Ruta | Auth | Propósito |
|---|---|---|---|
| `POST` | `/api/mcp` | token de API | JSON-RPC 2.0 sobre el transporte Streamable HTTP, una petición por `POST`, respondida en JSON. Protocolo `2026-07-28` (sin estado: cada petición lleva su versión y capacidades de cliente en `_meta`, más las cabeceras `MCP-Protocol-Version`, `Mcp-Method` y `Mcp-Name`; `server/discover`), y `2025-11-25`, `2025-06-18` y `2025-03-26` a través del saludo `initialize`. Sin sesión: no se emite ningún `Mcp-Session-Id`, uno enviado se ignora. Una notificación recibe `202` y sin cuerpo. |
| `GET`, `DELETE` | `/api/mcp` | público | `405` con `Allow: POST` — no hay ningún stream de servidor ni ninguna sesión que terminar; el cuerpo solo indica qué es este endpoint. |

`tools` es la única capacidad — sin resources, sin prompts, sin OAuth.
De las 27 herramientas, un token `read` puede llamar a las 15 que solo
leen (`get_status`, `list_devices`, `get_device`, `list_alerts`,
`alert_history`, `query_metrics`, `list_silences`, `list_rules`,
`list_device_types`, `list_agents`, `list_containers`,
`list_heartbeats`, `list_status_pages`, `list_channels`,
`list_packs`); las 12 que cambian algo (`silence_device`,
`remove_silence`, `acknowledge_alert`, `probe_device`,
`set_device_enabled`, `set_rule_enabled`, `add_device`,
`discover_network`, `schedule_maintenance`, `restart_container`,
`test_channel`, `post_incident`) necesitan un token `write`. Las
herramientas llaman al mismo código que la interfaz web. Las
versiones, cabeceras, códigos de error y notas de seguridad están en
[Conectar un asistente](../using/assistant.md#protocol-details).

## Archivos de agente (fuera de `/api`) {#agent-files-outside-api}

| Método | Ruta | Propósito |
|---|---|---|
| `GET` | `/install.sh` | El instalador de Linux, macOS y FreeBSD. Público. |
| `GET` | `/install.ps1` | El instalador de Windows. Público. |
| `GET` | `/download/{name}` | `dumbmonit-agent-linux-x86_64`, `dumbmonit-agent-linux-aarch64`, `dumbmonit-agent-freebsd-x86_64`, `dumbmonit-agent-windows-x86_64.exe`, servidos desde `DUMBMONIT_AGENT_DIR`; `{name}.sha256` da su checksum en formato `sha256sum`. `404` si el archivo está ausente. `dumbmonit-agent-macos-aarch64` y `dumbmonit-agent-macos-x86_64` (y sus `.sha256`) responden `307` al mismo archivo en la última versión de GitHub. Público. |

Cualquier otra ruta la sirve la interfaz web, que te pide iniciar
sesión por sí misma.
