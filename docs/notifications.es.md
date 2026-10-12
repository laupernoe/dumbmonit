# Notificaciones de DumbMonit {#dumbmonit-notifications}

Esta guía explica cómo recibir las alertas de DumbMonit en el servicio
que elijas. No requiere ninguna habilidad de programación: cada
sección dice dónde encontrar la dirección o token a copiar, y qué
escribir en el formulario.

---

## Contenido {#contents}

- [Cómo funciona](#how-it-works)
- [Crear y probar un canal](#creating-and-testing-a-channel)
- [Settings o secrets: la diferencia](#settings-or-secrets-the-difference)
- [Certificados autofirmados](#self-signed-certificates)
- [El canal personalizado](#the-custom-channel)
  - [Variables disponibles](#available-variables)
  - [Ejemplo 1 — una API JSON interna](#example-1--an-internal-json-api)
  - [Ejemplo 2 — un servicio que espera un formulario](#example-2--a-service-that-expects-a-form)
  - [Ejemplo 3 — un SMS en texto plano a través de un operador](#example-3--a-plain-text-sms-through-a-carrier)
  - [Ejemplo 4 — una petición GET sin cuerpo](#example-4--a-get-request-without-a-body)
  - [Errores comunes de plantilla](#common-template-mistakes)
- [Canales soportados](#supported-channels)
- [Solución de problemas](#troubleshooting)

---

## Cómo funciona {#how-it-works}

Un **canal** es un destino: un canal de Discord, un número de
teléfono, una bandeja de correo. Creas tantos como quieras, y tus
reglas de alerta eligen a cuáles notificar.

Tres principios se aplican a cada canal:

1. **Las alertas se agrupan.** Un mensaje por dispositivo y por ciclo,
   incluso si cinco discos se llenan al mismo tiempo. Sin esto,
   silenciarías las notificaciones en una semana.
2. **Un canal roto nunca bloquea a otro.** El fallo se registra en la
   columna «último error» del canal, y la entrega continúa hacia los
   demás.
3. **Un servicio que no responde se abandona tras 15 segundos.** Una
   notificación que llega un minuto tarde ya no es útil, y el bucle de
   monitorización no debe esperar detrás de un servidor inaccesible.

---

## Crear y probar un canal {#creating-and-testing-a-channel}

1. Abre **Alerts → Notifications → Add channel** (`/alerts#notifications`).
2. Dale un nombre significativo («Discord — family», «SMS —
   on-call»). Este es el nombre que elegirás después en tus reglas de
   alerta.
3. Elige el tipo de canal, y luego rellena los campos descritos abajo.
4. Guarda, y luego haz clic en **Send test**.

El mensaje de prueba sigue **exactamente el mismo camino** que una
alerta real: la misma petición, el mismo token, el mismo formato. Si
llega, el canal funciona. Si falla, el mensaje de error nombra el
ajuste a corregir.

> **Un token nunca se vuelve a mostrar.** Una vez guardado, se cifra
> en la base de datos y la interfaz solo dice «configured». Para
> editar un canal sin volver a escribir su token, simplemente deja el
> campo vacío.

---

## Settings o secrets: la diferencia {#settings-or-secrets-the-difference}

Cada canal tiene dos grupos de campos.

| | **Settings** | **Secrets** |
|---|---|---|
| Contenido | dirección del servidor, nombre del canal, topic | tokens, contraseñas, URLs de webhook |
| Almacenamiento | texto plano | cifrado |
| Devuelto por la interfaz | sí | nunca |

La interfaz **se niega** a guardar un campo sensible entre los
settings, y te lo indica. Esto no es una formalidad: los settings se
devuelven tal cual por la API, así que un token guardado ahí se
publicaría cada vez que se abre la página.

Las tablas de la sección [Canales soportados](#supported-channels)
dicen, para cada canal, qué va dónde.

---

## Certificados autofirmados {#self-signed-certificates}

Por defecto DumbMonit rechaza todo servidor cuyo certificado no pueda
verificar. Para un servicio autoalojado servido con un certificado
autofirmado o de una autoridad privada, los canales de Matrix, Mattermost,
Rocket.Chat, Zulip, ntfy, Gotify, Bark, Apprise, Home Assistant, Signal y el
webhook personalizado tienen un ajuste **Accept an unverifiable certificate**
(`insecure_tls`, un booleano, desactivado por defecto).

Activarlo mantiene la conexión cifrada pero deja de comprobar la identidad del
servidor, así que úsalo solo para un servidor que controlas, en una red de
confianza. Cuando un certificado es la causa de un fallo, el error mostrado
para el canal lo nombra (por ejemplo «invalid peer certificate:
UnknownIssuer»).

---

## El canal personalizado {#the-custom-channel}

El canal **`webhook`** es la vía de escape de DumbMonit. Describes
toda la petición HTTP — método, dirección, cabeceras, tipo de
contenido y cuerpo —, lo cual te permite conectar casi cualquier
servicio, incluida una API interna que DumbMonit nunca conocerá.

**Sin ninguna configuración particular**, ya envía una carga JSON
estructurada que un script puede usar:

```json
{
  "source": "dumbmonit",
  "target": "nas",
  "rule": "Disk full",
  "severity": "warning",
  "status": "firing",
  "title": "nas — Disk full",
  "text": "⚠️ Warning · Disk full — /data — 95 % (threshold > 90 %)",
  "value": 95.0,
  "threshold": 90.0,
  "unit": "%",
  "fingerprint": "nas/disk-full",
  "count": 1,
  "at": "2026-09-01T14:12:05+00:00",
  "link": "https://dumbmonit.home/targets/42"
}
```

`link` apunta a la página del dispositivo, construida sobre la `public_url` de
la [política de notificación](alerting/notifications.md) (o
`DUMBMONIT_PUBLIC_URL`), o sobre la `base_url` del canal. Un mensaje sobre
varios dispositivos, y el mensaje de prueba, enlazan a la propia instancia. Sin
ninguna URL pública, `link` es `null`.

### Ajustes del canal personalizado {#custom-channel-settings}

| Ajuste | Obligatorio | Por defecto | Rol |
|---|---|---|---|
| `url` | sí (aquí o como secret) | — | Dirección a llamar. Puede contener variables. |
| `method` | no | `POST` | `POST`, `PUT` o `GET`. |
| `content_type` | no | `json` | `json`, `form` o `text`. También determina cómo se escapan las variables. |
| `body_template` | no | — | Plantilla del cuerpo. Cuando está ausente, se envía la carga de arriba. |
| `base_url` | no | — | Dirección pública de tu DumbMonit, para que `{{link}}` dé un enlace clicable. |
| `username` | no | — | Nombre de usuario para autenticación HTTP «Basic». Requiere el secret `password`. |

| Secret | Rol |
|---|---|
| `url` | Úsalo en lugar del ajuste `url` si la dirección ya contiene un token. |
| `token` | Enviado como una cabecera `Authorization: Bearer …`, a menos que tu plantilla use `{{token}}`. |
| `password` | Contraseña para autenticación «Basic». |
| `headers` | Cabeceras extra, como un objeto JSON `{"X-Api-Key": "value"}`. Guardado cifrado como el token y nunca mostrado de nuevo, así que las claves de API y las cabeceras `Authorization` van aquí. |

`headers` es un secret. Los canales guardados por 0.1.0-alpha.6 o anteriores,
que los guardaban en los settings, los tienen movidos a los secrets cifrados en
el primer arranque, y la API ya no los devuelve. Como todo secret, se conservan
cuando editas el canal sin volver a escribirlos; enviar `secrets` reemplaza el
conjunto completo.

DumbMonit sigue una redirección solo al mismo host (o de `http` a `https` en
ese host): las cabeceras y el token de un canal nunca van a otra dirección que
la que introdujiste.

### Variables disponibles {#available-variables}

Una variable se escribe entre **dobles llaves**: `{{title}}`. Los
espacios alrededor del nombre se toleran (`{{ title }}`). Las llaves
simples en un cuerpo JSON se mantienen literales, así que puedes
escribir tu JSON normalmente.

| Variable | Qué contiene | Ejemplo |
|---|---|---|
| `{{title}}` | Título completo del mensaje | `nas — Disk full` |
| `{{message}}` | Cuerpo en texto plano, una línea por alerta | `⚠️ Warning · Disk full — /data — 95 % (threshold > 90 %)` |
| `{{message_markdown}}` | El mismo cuerpo en Markdown | `**nas — Disk full**\n• ⚠️ …` |
| `{{rule}}` | Nombre de la regla de alerta | `Disk full` |
| `{{target}}` | Nombre del dispositivo monitorizado | `nas` |
| `{{target_id}}` | Identificador numérico del dispositivo | `42` |
| `{{severity}}` | Severidad técnica | `info`, `warning` o `critical` |
| `{{severity_fr}}` | Severidad como palabra legible | `information`, `avertissement`, `critique` |
| `{{status}}` | Estado técnico | `firing` o `resolved` |
| `{{status_fr}}` | Estado como palabra legible | `en cours` o `résolu` |
| `{{value}}` | Valor medido, unidad incluida | `95 %` |
| `{{value_raw}}` | Valor desnudo, usable como **número** JSON | `95` |
| `{{threshold}}` | Umbral cruzado, unidad incluida | `90 %` |
| `{{threshold_raw}}` | Umbral desnudo, usable como **número** JSON | `90` |
| `{{unit}}` | Unidad del valor | `%` |
| `{{operator}}` | Comparación del umbral | `>` |
| `{{count}}` | Número de alertas agrupadas en este mensaje | `3` |
| `{{fingerprint}}` | Identificador estable de la alerta | `nas/disk-full` |
| `{{timestamp}}` | Timestamp ISO 8601 | `2026-09-01T14:12:05+00:00` |
| `{{timestamp_unix}}` | Timestamp en segundos desde 1970 | `1788358325` |
| `{{date}}` | Timestamp legible, en UTC | `2026-09-01 14:12 UTC` |
| `{{priority}}` | Prioridad de 1 (baja) a 5 (alta) | `4` |
| `{{color}}` | Color de acento, sin almohadilla | `D98A00` |
| `{{color_hex}}` | Color de acento, con almohadilla | `#D98A00` |
| `{{emoji}}` | Pictograma de estado | `🔴`, `⚠️`, `ℹ️` o `✅` |
| `{{link}}` | Enlace al dispositivo en DumbMonit | `https://dumbmonit.home/targets/42` |
| `{{source}}` | Siempre `dumbmonit` | `dumbmonit` |
| `{{token}}` | El secret `token` del canal | *(tu token)* |

Dos observaciones útiles:

- **Algunas variables pueden estar vacías.** Una alerta de
  disponibilidad («host unreachable») no tiene ni valor ni umbral:
  `{{value}}` y `{{value_raw}}` entonces renderizan una cadena vacía,
  nunca `null` ni `NaN`. Si las colocas como un número JSON, gestiona
  ese caso — o usa `"{{value}}"` entre comillas.
- **`{{link}}` permanece vacío** hasta que rellenas `base_url`. Un
  enlace incorrecto perdería más tiempo que una alerta sin ninguno.

**Nunca necesitas escapar nada tú mismo.** Según el `content_type`
elegido, DumbMonit protege automáticamente los valores sustituidos:
comillas y saltos de línea escapados en JSON, codificación de
porcentaje en un formulario, sin transformación en texto plano. Una
regla llamada `"System" disk full` no romperá tu plantilla JSON.

### Ejemplo 1 — una API JSON interna {#example-1--an-internal-json-api}

Una aplicación casera expone `POST /incidents` y espera un token en
una cabecera personalizada.

**Settings**

```json
{
  "url": "https://api.internal.example.org/incidents",
  "method": "POST",
  "content_type": "json",
  "base_url": "https://dumbmonit.home",
  "body_template": "{\"title\": \"{{title}}\", \"device\": \"{{target}}\", \"severity\": \"{{severity}}\", \"state\": \"{{status}}\", \"value\": {{value_raw}}, \"threshold\": {{threshold_raw}}, \"detail\": \"{{message}}\", \"link\": \"{{link}}\", \"seen_at\": \"{{timestamp}}\"}"
}
```

**Secrets**

```json
{
  "token": "your-application-token",
  "headers": "{\"X-Application\": \"dumbmonit\"}"
}
```

El token se envía automáticamente como una cabecera
`Authorization: Bearer …`. Esto es lo que recibe la aplicación:

```json
{
  "title": "nas — Disk full",
  "device": "nas",
  "severity": "warning",
  "state": "firing",
  "value": 95,
  "threshold": 90,
  "detail": "⚠️ Warning · Disk full — /data — 95 % (threshold > 90 %)",
  "link": "https://dumbmonit.home/targets/42",
  "seen_at": "2026-09-01T14:12:05+00:00"
}
```

### Ejemplo 2 — un servicio que espera un formulario {#example-2--a-service-that-expects-a-form}

Muchas pasarelas SMS y APIs antiguas esperan
`application/x-www-form-urlencoded` en lugar de JSON, con el token en
el cuerpo.

**Settings**

```json
{
  "url": "https://gateway.example.org/api/send",
  "method": "POST",
  "content_type": "form",
  "body_template": "key={{token}}&recipient=0612345678&subject={{title}}&text={{emoji}} {{message}}&priority={{priority}}"
}
```

**Secrets**

```json
{ "token": "gateway-key" }
```

Como la plantilla ya usa `{{token}}`, DumbMonit **no añade** una
cabecera `Authorization`: algunos servicios rechazan una petición que
lleva dos autenticaciones. Los valores se codifican automáticamente,
los espacios se convierten en `+` y los caracteres acentuados en su
equivalente de codificación de porcentaje.

### Ejemplo 3 — un SMS en texto plano a través de un operador {#example-3--a-plain-text-sms-through-a-carrier}

Algunos servicios, como la API SMS de un operador o una automatización
casera, quieren una única línea de texto.

**Settings**

```json
{
  "url": "https://automation.home/alert",
  "method": "POST",
  "content_type": "text",
  "body_template": "{{emoji}} {{severity}} on {{target}}: {{rule}} ({{value}}, threshold {{operator}} {{threshold}}) — {{date}}"
}
```

El mensaje enviado, sin ningún escapado:

```
⚠️ warning on nas: Disk full (95 %, threshold > 90 %) — 01/09/2026 14:12 UTC
```

### Ejemplo 4 — una petición GET sin cuerpo {#example-4--a-get-request-without-a-body}

Cuando el servicio solo acepta parámetros de URL, pon las variables en
`url` y elige el método `GET`. Una petición `GET` no tiene cuerpo:
`body_template` entonces se rechaza, para que nadie crea que hace algo.

**Settings**

```json
{
  "url": "https://sms.example.org/send?user=noe&text={{title}} — {{value}}",
  "method": "GET"
}
```

**Secrets**

```json
{ "token": "api-key" }
```

Solo los **valores sustituidos** se codifican; el resto de la
dirección se copia tal cual. La petición realmente enviada se ve así:

```
GET https://sms.example.org/send?user=noe&text=nas%20%E2%80%94%20Disk%20full%20%E2%80%94%2095%20%25
```

### Errores comunes de plantilla {#common-template-mistakes}

Una plantilla defectuosa se **rechaza al guardar**, mientras estás
mirando el formulario, no la noche en que ocurre el incidente.

| Lo que escribes | Lo que ocurre |
|---|---|
| `{{severite}}` | Rechazado: variable desconocida «severite», seguido de la lista de nombres válidos. El nombre correcto es `severity`. |
| `{"t": "{{title"}` | Rechazado: «unclosed braces». Falta el `}}`. |
| `{{}}` o `{{  }}` | Rechazado: una variable sin nombre no significa nada. |
| `{"a": {"b": 1}}` | **Aceptado.** Las llaves simples son literales: escribe tu JSON normalmente. |
| `{{token}}` sin un secret `token` | Rechazado: la plantilla pide un token que el canal no tiene. |
| `method: GET` con `body_template` | Rechazado, con la alternativa: pon las variables en `url`. |
| `method: DELETE` | Rechazado: solo se aceptan `POST`, `PUT` y `GET`. |

---

## Canales soportados {#supported-channels}

### Discord

En Discord: **Channel settings → Integrations → Webhooks → New
webhook**, luego **Copy webhook URL**. Esta URL contiene el token: va
en los secrets.

- **Secrets** — `webhook_url`

### Slack

En <https://api.slack.com/apps>, crea una app, activa **Incoming
Webhooks**, luego **Add New Webhook to Workspace** y elige el canal.
Copia la URL `https://hooks.slack.com/services/…`.

- **Secrets** — `webhook_url`

### Telegram

Necesitas dos cosas: el **token del bot** (secret) y el **ID de chat**
(`chat_id`) donde debe escribir. El primero tarda un minuto en
obtenerse; el segundo depende de dónde quieras recibir las alertas.

#### 1. Crear el bot y obtener su token {#1-create-the-bot-and-get-its-token}

1. En Telegram, busca **@BotFather** y haz clic en *Start*.
2. Envía `/newbot`, y luego responde a las preguntas (un nombre, luego
   un nombre de usuario que debe terminar en `bot`).
3. BotFather termina con un mensaje como este:

   ```
   Done! Congratulations on your new bot. You will find it at t.me/my_homelab_bot.
   Use this token to access the HTTP API:
   63xxxxxx71:AAFoxxxxn0hwA-2TVSxxxNf4c
   ```

   La línea `63xxxxxx71:AAFoxxxxn0hwA-2TVSxxxNf4c` es el **token del
   bot**. No lo compartas: da control total del bot.

En los ejemplos de abajo, sustituye `<TOKEN>` por este token. Fíjate
en el prefijo `bot` pegado delante en las URLs:
`https://api.telegram.org/bot<TOKEN>/…`.

#### 2. Encontrar el `chat_id` {#2-find-the-chat_id}

**Chat privado** (el bot te escribe directamente)

1. Abre tu bot (`t.me/my_homelab_bot`) y haz clic en *Start*, o
   envíale cualquier mensaje. Sin esto, nunca podrá escribirte.
2. Abre en un navegador: `https://api.telegram.org/bot<TOKEN>/getUpdates`
3. Obtienes un documento JSON; busca `"chat"`:

   ```json
   {"ok":true,"result":[{"update_id":83xxxxx35,
     "message":{"message_id":2643,"from":{…},
       "chat":{"id":21xxxxx38,"first_name":"…","type":"private"},
       "date":1703062972,"text":"/start"}}]}
   ```

   El `chat_id` es el valor de `chat.id`: aquí `21xxxxx38` (un número
   positivo).

   Si `result` está vacío (`"result":[]`), envía otro mensaje al bot
   y recarga la página.

**Canal**

1. Añade el bot como administrador del canal (Manage channel →
   Administrators → Add).
2. Publica un mensaje en el canal.
3. Abre `https://api.telegram.org/bot<TOKEN>/getUpdates`: la respuesta
   contiene un bloque `channel_post` con
   `"chat":{"id":-1001xxxxxx062,"type":"channel"}`.

   El `chat_id` es ese número negativo, `-100` incluido:
   `-1001xxxxxx062`.

**Grupo**

La forma más fácil pasa por la app de escritorio:

1. Añade el bot al grupo.
2. Envía un mensaje en el grupo, clic derecho sobre él → *Copy message
   link*. Obtienes `https://t.me/c/194xxxx987/13` (o
   `https://t.me/c/194xxxx987/11/13` si el grupo tiene topics).
3. El número justo después de `/c/` es el identificador interno del
   grupo: `194xxxx987`. El `chat_id` a introducir es ese número
   **con el prefijo `-100`**: `-100194xxxx987`.

Sin la app de escritorio, el método `getUpdates` de arriba también
funciona: `message.chat.id` es directamente el número correcto (ya
negativo). Si el bot no ve los mensajes del grupo, pide a BotFather
`/setprivacy` → *Disable*, luego elimina el bot del grupo y vuelve a
añadirlo.

**Topic de grupo** (grupos en modo «Topics»)

El `chat_id` es el del grupo. Además, rellena **Group topic**
(`message_thread_id`) con el número del medio del enlace copiado: en
`https://t.me/c/194xxxx987/11/13`, el topic es `11`. Sin este ajuste,
el mensaje cae en el hilo *General*.

#### 3. Comprobar antes de guardar {#3-check-before-saving}

Abre en el navegador:

```
https://api.telegram.org/bot<TOKEN>/sendMessage?chat_id=<CHAT_ID>&text=test123
```

(añade `&message_thread_id=11` para un topic). Si llega `test123`, el
token y el `chat_id` son correctos: introdúcelos en DumbMonit y haz
clic en *Test*.

- **Settings** — `chat_id` (obligatorio), `message_thread_id` (topic,
  opcional), `api_base` (por defecto `https://api.telegram.org`,
  cámbialo solo para pasar por un relay)
- **Secrets** — `bot_token`

> **«Telegram cannot find this chat»** — para un chat privado, el bot
> nunca ha recibido un mensaje de ti: envíale `/start`. Para un grupo
> o un canal, probablemente falta el prefijo `-100`, o el bot no se
> ha añadido a él.
>
> **«thread not found»** — el `message_thread_id` no coincide con
> ningún topic del grupo, o el grupo no está en modo «Topics».

### Microsoft Teams

**Aviso: el método ha cambiado.** Los antiguos «Office 365
connectors» se retiraron en mayo de 2026; una URL
`outlook.office.com/webhook/…` ya no funciona.

El procedimiento actual:

1. En el canal de Teams, **⋯ → Workflows**.
2. Elige la plantilla **«Post to a channel when a webhook request is
   received»**.
3. Confirma los pasos, y luego **copia la URL HTTPS POST** mostrada al
   final.

- **Secrets** — `webhook_url`

DumbMonit envía una *Adaptive Card* envuelta, el único formato que
estos flujos muestran correctamente. Si obtienes un error que
menciona Power Automate, la URL probablemente viene de un conector
antiguo.

### Matrix

1. Crea una cuenta dedicada para DumbMonit en tu homeserver, y
   **invítala a la sala**. Una cuenta que no es miembro no puede
   escribir.
2. Obtén su token de acceso. En Element: **Settings → Help & About →
   Advanced → Access token**.
3. Obtén el identificador **interno** de la sala — no su nombre
   visible. En Element: **Room settings → Advanced → Internal room
   ID**. Empieza con `!` (por ejemplo `!aBcDeF:matrix.org`).

- **Settings** — `server_url` (por ejemplo `https://matrix.org`),
  `room_id`
- **Secrets** — `token`

### Mattermost

**Integrations → Incoming Webhooks → Add**. Elige el canal por
defecto, y luego copia la URL.

- **Settings** — `channel` (opcional, para escribir en otro sitio que
  no sea el canal por defecto), `username` (opcional, por defecto
  `DumbMonit`)
- **Secrets** — `webhook_url`

### Rocket.Chat

**Administration → Integrations → New integration → Incoming
webhook**. Elige la sala, activa la integración, y luego copia la
*Webhook URL*.

- **Settings** — `channel` (opcional), `alias` (opcional, por defecto
  `DumbMonit`)
- **Secrets** — `webhook_url`

### Google Chat

En el espacio de Google Chat: **nombre del espacio → Apps &
integrations → Webhooks → Add webhook**. Copia la URL, que ya
contiene la clave y el token.

- **Secrets** — `webhook_url`

### Zulip

1. Crea un **bot**: **Personal settings → Bots → Add a new bot**, tipo
   «Generic». Anota su dirección de correo y su clave de API.
2. Suscribe el bot al stream destino, de lo contrario el envío se
   rechazará.

- **Settings** — `server_url` (por ejemplo
  `https://your-org.zulipchat.com`), `email` (la dirección del bot),
  `stream` (el nombre del stream), `topic` (opcional, por defecto
  `DumbMonit`)
- **Secrets** — `api_key`

### ntfy

El canal más simple de todos: elige un nombre de topic difícil de
adivinar, y luego suscríbete a él desde la app ntfy.

- **Settings** — `topic`, `server_url` (opcional, por defecto
  `https://ntfy.sh`)
- **Secrets** — `token` (solo si tu servidor requiere autenticación)

> En la instancia pública, **cualquiera que conozca el nombre de un
> topic puede leerlo**. Elige un nombre largo y aleatorio, o alberga tu
> propio servidor.

### Gotify

En la interfaz de Gotify: **Apps → Create Application**. Copia el
token mostrado.

- **Settings** — `server_url` (por ejemplo `https://gotify.home`)
- **Secrets** — `token`

### Pushover

1. En <https://pushover.net>, tu **User Key** se muestra justo en la
   página de inicio.
2. Luego crea una aplicación: **Create an Application/API Token**.
   Obtienes un **token de aplicación**.

- **Settings** — `priority` (opcional, de -2 a 2), `sound` (opcional),
  `retry` y `expire` (solo para `priority: 2`)
- **Secrets** — `token` (token de aplicación), `user_key` (clave de
  usuario)

Sin un ajuste `priority`, DumbMonit elige según la severidad: `-1`
para un aviso o una resolución, `0` para un warning, `1` para una
alerta crítica. Un aviso por tanto no hace sonar el teléfono.

> La prioridad `2` («emergency») repite la notificación hasta que se
> confirma. DumbMonit entonces añade `retry` y `expire`
> automáticamente, sin los cuales Pushover rechazaría el mensaje.

### Pushbullet

En <https://www.pushbullet.com/#settings/account>, haz clic en
**Create Access Token**.

- **Settings** — `device_iden` (opcional; sin él, se notifica a todos
  tus dispositivos)
- **Secrets** — `token`

### Bark (iOS)

Abre la app Bark en tu iPhone: muestra una URL que contiene tu **clave
de dispositivo**, la cadena larga de caracteres después del nombre de
dominio.

- **Settings** — `server_url` (opcional, por defecto
  `https://api.day.app`), `group` (opcional), `level` (opcional),
  `sound` (opcional)
- **Secrets** — `token` (la clave de dispositivo)

Sin un ajuste `level`, DumbMonit elige: `passive` para un aviso,
`active` para un warning, `timeSensitive` para una alerta crítica. El
nivel `critical`, que atraviesa «No Molestar», nunca se elige
automáticamente — defínelo explícitamente si lo quieres.

### Web Push

Notificaciones nativas en tus teléfonos y ordenadores, directamente
desde tu propio DumbMonit, sin ninguna app que instalar y sin ninguna
cuenta de terceros: el navegador (instalado como app o no) recibe las
alertas incluso con todas las pestañas cerradas.

1. En cada teléfono u ordenador, abre DumbMonit y ve a **Settings →
   Push notifications → Enable on this device**, luego **Send a
   test**. En iPhone y iPad, primero añade DumbMonit a la pantalla de
   inicio. La guía paso a paso está en [Notificaciones push
   (PWA)](using/push-notifications.md).
2. Crea un canal **Web Push** aquí. No tiene ningún secreto: sus
   destinatarios son los dispositivos suscritos en el paso 1.

- **Settings** — `users` (opcional; una cuenta por línea, vacío: cada
  dispositivo suscrito de cada cuenta), `urgency` (`high` por
  defecto, `normal` o `low`), `ttl` (segundos que el servicio push
  conserva el mensaje para un dispositivo apagado, por defecto
  `86400`), `contact` (dirección `mailto:` o `https:` opcional dada a
  los servicios push; vacío: `DUMBMONIT_PUBLIC_URL` cuando es HTTPS,
  si no la página del proyecto)
- **Secrets** — ninguno. El par de claves VAPID del servidor se
  genera en el primer arranque y se guarda cifrado; la clave privada
  nunca sale del servidor.

El título de la notificación es la severidad y el dispositivo
(`Critical · nas01`), el cuerpo un resumen breve; un toque abre la
página **Alerts**. DumbMonit necesita HTTPS saliente hacia los
servicios push (`fcm.googleapis.com`, `*.notify.windows.com`,
`updates.push.services.mozilla.com`, `*.push.apple.com`); los
mensajes están cifrados de extremo a extremo, esos servicios no pueden
leerlos. Una suscripción que el servicio push reporta como
desaparecida (HTTP 404 o 410, por ejemplo tras eliminar la app) se
elimina automáticamente.

### Apprise

Apprise es una **pasarela**: con una sola configuración da acceso a
decenas de servicios que DumbMonit no implementa él mismo. Despliega
el contenedor `caronc/apprise`, y luego elige uno de los dos modos.

**Modo recomendado — configuración guardada en Apprise.** Crea una
configuración en la interfaz de Apprise, anota su clave, y no pongas
nada más aquí.

- **Settings** — `server_url`, `config_key`

**Modo directo — DumbMonit proporciona los destinos.** Las URLs de
Apprise a menudo contienen una contraseña (`mailto://user:password@…`):
por tanto van en los secrets.

- **Settings** — `server_url`
- **Secrets** — `urls` (una o más URLs de Apprise, separadas por
  comas)

Ajustes comunes opcionales: `tag` (para dirigirse solo a parte de los
destinos de una configuración) y `format` (`text` por defecto, o
`markdown`).

> Si la prueba responde «Apprise has no configuration under this
> config_key», la clave no coincide con nada. DumbMonit lo reporta en
> lugar de fingir que el mensaje se envió.

### Home Assistant

1. En Home Assistant: **tu perfil → Security → Long-lived access
   tokens → Create token**. Cópialo, no se volverá a mostrar.
2. Elige el servicio a llamar. Por defecto, DumbMonit crea una
   notificación persistente en el panel.

- **Settings** — `server_url` (por ejemplo
  `http://homeassistant.local:8123`), `service` (opcional, por
  defecto `persistent_notification.create`), `target` (opcional),
  `data` (opcional, objeto fusionado en la llamada)
- **Secrets** — `token`

Algunos servicios útiles:

| `service` | Efecto |
|---|---|
| `persistent_notification.create` | Banner en la interfaz de Home Assistant |
| `notify.notify` | Todos los destinos de notificación configurados |
| `notify.mobile_app_<device>` | La app móvil de un dispositivo específico |
| `light.turn_on` | Encender una luz — usa `data` para la entidad y el color |

El ajuste `data` se fusiona **al final**: por tanto puede sobrescribir
el título o el mensaje que preparó DumbMonit.

### Correo (SMTP)

El único canal que no depende de ningún servicio de terceros.

- **Settings** — `host`, `from`, `to` (lista de direcciones),
  `security` (`starttls` por defecto, o `tls`, o `none`), `port`
  (derivado de `security`: 587, 465 o 25), `username` (opcional)
- **Secrets** — `password`

> Con Gmail, la contraseña de la cuenta no funciona: crea una
> **contraseña de aplicación** en los ajustes de seguridad de tu
> cuenta de Google.

### Signal

Pasa por una instancia de
[`signal-cli-rest-api`](https://github.com/bbernhard/signal-cli-rest-api)
que tú alojas y donde primero registras tu número.

- **Settings** — `server_url` (por ejemplo `http://signal.home:8080`),
  `number` (el número de remitente registrado), `recipients` (lista
  de números o identificadores de grupo), `username` (opcional)
- **Secrets** — `password` (opcional, si tu instancia está protegida)

Todos los números se escriben en **formato internacional**,
`+33612345678`.

### SMS vía Twilio

En la consola de Twilio, la página de inicio muestra tu **Account
SID** y tu **Auth Token**. Luego compra un número que pueda enviar
SMS.

- **Settings** — `account_sid`, `from` (tu número de Twilio) **o**
  `messaging_service_sid`, `to` (lista de destinatarios)
- **Secrets** — `token` (el *Auth Token*)

El cuerpo del SMS se trunca a 300 caracteres: más allá de eso, Twilio
lo divide en segmentos facturados por separado.

> En una cuenta de prueba, Twilio solo envía a números que has
> verificado.

### PagerDuty

En PagerDuty: **Services → your service → Integrations → Add
integration**, y elige **Events API v2**. Copia la *Integration Key*.

- **Settings** — `region` (`eu` si tu cuenta está alojada en Europa),
  `source` (opcional, por defecto `dumbmonit`)
- **Secrets** — `token` (la *Integration Key*)

Una alerta abre un incidente; su resolución **lo cierra
automáticamente**. El emparejamiento se hace sobre la huella de la
alerta, con el prefijo `dumbmonit-`, así que un recordatorio nunca
abre un segundo incidente.

> Una clave de otro tipo («Events API v1», «REST API») recibe un
> rechazo breve. Comprueba que la integración es realmente de tipo
> *Events API v2*.

### Opsgenie

**Teams → your team → Integrations → Add integration → API**. Copia
la clave de API.

- **Settings** — `region` (`eu` para una cuenta europea),
  `responders` (lista de nombres de equipo, opcional), `tags` (lista,
  opcional)
- **Secrets** — `api_key`

Como con PagerDuty, una resolución cierra la alerta, identificada por
su alias.

> Atlassian ha fijado el **fin de servicio de Opsgenie para el 5 de
> abril de 2027**. Este canal se mantiene para instalaciones
> existentes; para una nueva, prefiere PagerDuty o un canal
> personalizado apuntando a Jira Service Management.

### Webhook personalizado {#custom-webhook}

Consulta [El canal personalizado](#the-custom-channel) arriba.

---

## Solución de problemas {#troubleshooting}

**La prueba falla de inmediato, con «missing setting …».**
Un campo obligatorio está vacío. El mensaje nombra el ajuste exacto.

**La prueba falla con «no answer within 15 s».**
El servidor destino es inaccesible desde DumbMonit. Si es un servicio
en tu red local, comprueba que el contenedor de DumbMonit puede
alcanzarlo — esto es frecuentemente un problema de redes Docker, no un
problema de configuración de canal.

**La prueba falla con un error de certificado.**
Tu servidor usa un certificado autofirmado o de una CA privada. Consulta
[Certificados autofirmados](#self-signed-certificates).

**La prueba tiene éxito, pero no recibo nada.**
El servicio aceptó el mensaje y lo envió a otro sitio: canal
incorrecto, topic de ntfy incorrecto, dispositivo de Pushbullet
incorrecto. Comprueba el ajuste de destino.

**El mensaje de error contiene `***`.**
Esto es intencional: tu token se eliminó del mensaje antes de
registrarlo. Ningún secreto aparece nunca en los registros, en la
columna «último error», ni en una respuesta de la API.

**¿Dónde puedo ver el último error de un canal?**
En la lista de canales: cada fila muestra la fecha de la última
entrega exitosa y, si lo hay, el error del último fallo.
