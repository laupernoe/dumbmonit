# Conectar un asistente {#connect-an-assistant}

DumbMonit tiene un servidor [MCP](https://modelcontextprotocol.io)
integrado: el Model Context Protocol, la forma en que asistentes como
Claude, ChatGPT, Cursor o el agente de VS Code llaman a herramientas
externas. Conecta uno, y luego pregúntale cosas como:

- «¿Todo va bien?»
- «Silencia el NAS durante dos horas, voy a cambiar un disco.»
- «¿Qué pasó anoche?»
- «¿Cuán lleno está el servidor de copias, y está empeorando?»
- «Publica una actualización en la página de estado: el servidor de
  correo ha vuelto.»

El asistente ve lo que ve la interfaz — dispositivos, alertas,
historial, métricas, agentes, páginas de estado — y usa las mismas
palabras: *reporting*, *unreachable*, *waiting*, *advisory*,
*warning*, *suppressed by parent*. Se ejecuta dentro del servidor: nada
que instalar junto a DumbMonit, y las herramientas llaman al mismo
código que la interfaz web.

## 1. Crear un token {#1-create-a-token}

Settings → **API & assistants** (`/settings#assistant`). Nombra el
token según el asistente o la máquina en la que se ejecuta («Claude on
my laptop»), y luego elige:

| Ajuste | Opciones |
| --- | --- |
| **Scope** | **Read**: mirar — estado, dispositivos, alertas, historial, métricas, reglas, agentes, páginas de estado. Nunca puede cambiar nada. **Read and write**: también actuar — silenciar, confirmar, sondear, añadir un dispositivo, reiniciar un contenedor, publicar un incidente… (las [herramientas de escritura](#what-the-assistant-can-call)). |
| **Expires** | 30 días, 90 días (lo por defecto), 1 año o nunca. Un token caducado deja de funcionar; crea uno nuevo. |
| **Allowed networks** | Opcional. Direcciones o rangos CIDR desde los que se puede usar el token (`192.168.1.0/24, 10.8.0.5`). Vacío: desde cualquier sitio. |

El token tiene el aspecto de `dmt_` seguido de 32 caracteres
hexadecimales. Se muestra **una vez**, justo tras crearlo; DumbMonit
solo conserva un hash. Si lo pierdes, revócalo y crea otro.

Prefiere un token de lectura a menos que realmente quieras que el
asistente actúe.

El mismo token también abre la [API REST](../reference/api.md#authentication)
a scripts y dashboards — `curl -H "Authorization: Bearer dmt_…"
https://monit.example.lan/api/targets` — donde `read` ve lo que ve un
visor y `write` hace lo que hace un administrador, cuentas, tokens y
copias de seguridad aparte.

## 2. Pegar el fragmento {#2-paste-the-snippet}

La página de ajustes los rellena con la dirección de tu servidor y el
token nuevo. Abajo, `https://monit.example.lan` representa tu
servidor.

### Claude Code

```sh
claude mcp add --transport http dumbmonit https://monit.example.lan/api/mcp \
  --header "Authorization: Bearer dmt_…"
```

Añade `--scope user` para tenerlo en cada proyecto, no solo en el
actual.

### Claude Desktop

Claude Desktop inicia servidores locales desde su archivo de
configuración; [`mcp-remote`](https://www.npmjs.com/package/mcp-remote)
lo conecta con el endpoint HTTP de DumbMonit (necesita
[Node.js](https://nodejs.org)). DumbMonit se autentica con una
cabecera bearer en lugar de OAuth, así que añádelo a través de este
archivo en lugar de la pantalla de Connectors. Settings → Developer →
**Edit Config**, y luego añade a `claude_desktop_config.json` y
reinicia Claude Desktop:

```json
{
  "mcpServers": {
    "dumbmonit": {
      "command": "npx",
      "args": [
        "-y", "mcp-remote", "https://monit.example.lan/api/mcp",
        "--header", "Authorization:${DUMBMONIT_AUTH}"
      ],
      "env": { "DUMBMONIT_AUTH": "Bearer dmt_…" }
    }
  }
}
```

La cabecera pasa por una variable de entorno, sin espacio después de
los dos puntos, porque algunas plataformas dividen los argumentos por
espacios. Para un servidor alcanzado por HTTP plano en tu red, añade
`"--allow-http"` a `args` — la página de ajustes lo hace por ti cuando
ella misma se sirve por HTTP.

### VS Code

`.vscode/mcp.json` en un workspace (o el comando **MCP: Add Server** →
HTTP):

```json
{
  "servers": {
    "dumbmonit": {
      "type": "http",
      "url": "https://monit.example.lan/api/mcp",
      "headers": { "Authorization": "Bearer dmt_…" }
    }
  }
}
```

### Cursor y otros clientes {#cursor-and-other-clients}

`.cursor/mcp.json`, y la misma forma para la mayoría de clientes que
soportan servidores Streamable HTTP con cabeceras personalizadas:

```json
{
  "mcpServers": {
    "dumbmonit": {
      "url": "https://monit.example.lan/api/mcp",
      "headers": { "Authorization": "Bearer dmt_…" }
    }
  }
}
```

Un cliente que solo inicia servidores locales (stdio) puede usar el
puente `mcp-remote` mostrado para Claude Desktop.

### ChatGPT

Settings → Connectors → **Create** (modo desarrollador). URL del
servidor MCP: `https://monit.example.lan/api/mcp`, cabecera de
autorización: `Bearer dmt_…`.

ChatGPT se conecta desde los servidores de OpenAI, no desde tu
navegador: DumbMonit debe ser **accesible desde internet por HTTPS**.
Consulta las [notas de seguridad](#security-notes) antes de hacer eso.

### Cualquier otro cliente {#any-other-client}

Lo que necesita saber un cliente: el endpoint es
`https://<server>/api/mcp`, el transporte es Streamable HTTP, y cada
petición lleva `Authorization: Bearer dmt_…`. Una llamada en bruto,
para comprobar el token y la ruta de red desde una terminal:

```bash
curl -s https://monit.example.lan/api/mcp \
  -H 'Authorization: Bearer dmt_…' \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: tools/call' -H 'Mcp-Name: get_status' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_status","arguments":{},
       "_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28",
                "io.modelcontextprotocol/clientCapabilities":{}}}}'
```

## Qué puede llamar el asistente {#what-the-assistant-can-call}

Veintisiete herramientas. Cada una tiene un título, una descripción, un
esquema de entrada, un esquema de salida y anotaciones
(`readOnlyHint`, `destructiveHint`, `idempotentHint`), así que un
cliente puede distinguir cuáles solo miran y preguntarte antes de
ejecutar las demás.

| Herramienta | Alcance | Hace |
| --- | --- | --- |
| `get_status` | read | El boletín, la respuesta a «¿todo va bien?»: recuentos de dispositivo, alertas activas y acumulándose, una frase («2 advisories, 1 unreachable.»). |
| `list_devices` | read | Dispositivos y servicios con estado, padre y etiquetas; filtra por subcadena o estado. |
| `get_device` | read | Un dispositivo: configuración, estado, alertas activas, resumen de 24 horas de CPU / memoria / disco más lleno (disponibilidad y latencia para una comprobación de servicio). |
| `list_alerts` | read | Alertas activas con palabras de severidad, desde cuándo, valor, supresión, confirmación. |
| `alert_history` | read | Transiciones de alerta, las más recientes primero (`since` o `hours`, dispositivo opcional, límite). |
| `query_metrics` | read | Una consulta de rango MetricsQL, como máximo 60 puntos por serie. |
| `list_silences` | read | Ventanas de mantenimiento y si están activas ahora. |
| `list_rules` | read | Reglas de alerta con tipo, severidad, umbral, activada. |
| `list_device_types` | read | Los tipos de dispositivo que puede monitorizar este servidor, con las opciones y credenciales que espera cada uno — lo que necesita `add_device`. |
| `list_agents` | read | Máquinas que ejecutan el agente: SO, versión del agente, modo relay, sitio, dispositivos relayados, último contacto, enlace. |
| `list_containers` | read | Los contenedores Docker de una máquina que ejecuta el agente: estado, salud, reinicios, actualización disponible, último comando. |
| `list_heartbeats` | read | Monitores heartbeat: intervalo esperado, última llamada, veredicto (waiting, on time, missed, reported down). La URL secreta nunca se muestra. |
| `list_status_pages` | read | Páginas de estado, publicadas o no, con su ruta pública (`/s/<slug>`), los dispositivos que muestran y sus incidentes abiertos. |
| `list_channels` | read | Canales de notificación: tipo, activado, último error, último mensaje enviado. Nunca sus secretos. |
| `list_packs` | read | Packs de integración instalados: versión, tipo de dispositivo, activado, dispositivos que lo usan, errores. |
| `silence_device` | write | Una ventana de mantenimiento puntual en un dispositivo, empezando ahora (1 hora por defecto, como máximo una semana). |
| `remove_silence` | write | Elimina una ventana de mantenimiento. |
| `schedule_maintenance` | write | Una ventana de mantenimiento puntual en una fecha dada, en un dispositivo o toda la instancia, una semana como máximo. |
| `acknowledge_alert` | write | Confirma una alerta por huella durante un número de horas (4 por defecto): los recordatorios se pausan, la resolución igual se notifica. `hours: 0` la levanta. |
| `probe_device` | write | Sondea un dispositivo de inmediato y reporta qué se midió. |
| `set_device_enabled` | write | Activa o desactiva la monitorización de un dispositivo. |
| `set_rule_enabled` | write | Activa o desactiva una regla de alerta. |
| `add_device` | write | Añade un dispositivo: nombre, tipo, dirección, intervalo, padre, opciones, credenciales, agente relay. |
| `discover_network` | write | Escanea una red (CIDR, 4096 direcciones como máximo) buscando dispositivos SNMP y sugiere un perfil para cada uno. |
| `restart_container` | write | Pide al agente de una máquina que reinicie uno de sus contenedores. |
| `test_channel` | write | Envía un mensaje de prueba a través de un canal de notificación. |
| `post_incident` | write | Abre un incidente en una página de estado, o publica una actualización en uno existente. |

Un token de lectura que llama a una herramienta de escritura recibe un
rechazo claro como resultado de la herramienta (`isError: true`); el
asistente lo explica y nada cambia. Lo mismo vale para cualquier cosa
que la herramienta no pueda hacer — un dispositivo desconocido, un
nombre ambiguo, una ventana más larga que una semana: el mensaje dice
qué corregir.

## Detalles del protocolo {#protocol-details}

`POST /api/mcp` implementa MCP sobre el transporte **Streamable
HTTP**: JSON-RPC 2.0, una petición por `POST`, respondida con JSON
plano (sin server-sent events). No tiene estado: no se emite ningún
`Mcp-Session-Id`, uno que envía un cliente se ignora, y `GET` o
`DELETE` en el endpoint responden `405`. Solo se ofrece la capacidad
`tools` — sin resources, prompts, sampling ni OAuth.

Dos generaciones del protocolo se sirven en el mismo endpoint:

| Versiones | Cómo lo habla un cliente |
| --- | --- |
| `2026-07-28` | Sin estado. Cada petición lleva `io.modelcontextprotocol/protocolVersion` y `io.modelcontextprotocol/clientCapabilities` en `params._meta`, y las cabeceras `MCP-Protocol-Version`, `Mcp-Method` y, para `tools/call`, `Mcp-Name`, que debe coincidir con el cuerpo. Métodos: `server/discover`, `tools/list`, `tools/call`. Los resultados llevan `resultType: "complete"` y la identidad del servidor en `_meta` (`io.modelcontextprotocol/serverInfo`); `tools/list` también lleva `ttlMs` y `cacheScope`. |
| `2025-11-25`, `2025-06-18`, `2025-03-26` | El saludo `initialize` elige la versión (una desconocida obtiene la más nueva de estas tres); `notifications/initialized` se confirma con `202`; luego `ping`, `tools/list`, `tools/call`. |

Errores, además de los `401`, `403` y `429` de las [comprobaciones de
token](#security-notes), que vienen antes de cualquier JSON-RPC:

| HTTP | Código JSON-RPC | Cuándo |
| --- | --- | --- |
| `400` | `-32020` (desajuste de cabecera) | Una petición `2026-07-28` sin `MCP-Protocol-Version`, `Mcp-Method` o `Mcp-Name`, o con valores que difieren del cuerpo. |
| `400` | `-32022` (versión de protocolo no soportada) | Una versión que este servidor no habla; `error.data` lista las `supported` y la `requested`. |
| `400` | `-32602` (parámetros inválidos) | Una petición `2026-07-28` sin los campos `_meta` requeridos. |
| `404` | `-32601` (método no encontrado) | Una petición `2026-07-28` para un método que este servidor no implementa. |
| `200` | `-32602` | Una herramienta desconocida, o `arguments` que no es un objeto. |
| | `-32700`, `-32600` | JSON ilegible; no es una petición JSON-RPC 2.0 (los lotes no están soportados). |
| `202` | — | Una notificación: aceptada, sin cuerpo. |

Una herramienta que se ejecuta pero falla — alcance faltante,
dispositivo desconocido — no es un error de protocolo: responde
normalmente con `isError: true` y una frase que el asistente puede
transmitir.

## Notas de seguridad {#security-notes}

- **Un token es una contraseña.** Quien lo tenga lee todo lo que
  muestra la interfaz — y, con un token de escritura, cambia cualquier
  cosa que pueda un administrador a través de la API REST
  (dispositivos, reglas, canales), cuentas, tokens y copias de
  seguridad aparte. Manténlo fuera de chats compartidos, capturas de
  pantalla y repositorios. Revoca un token del que no estés seguro;
  crea otro en un minuto.
- **Prefiere lectura.** La mayoría de preguntas («¿todo va bien?») no
  necesitan ningún alcance de escritura. Crea un token de escritura
  solo para un asistente al que realmente quieras que actúe, y
  nómbralo para reconocerlo en la lista. La mayoría de clientes
  preguntan antes de ejecutar una herramienta que no está marcada de
  solo lectura; deja esa confirmación activada.
- **Dale una vida y una red.** Un token que caduca en 90 días y solo
  funciona desde tu LAN o VPN (`192.168.1.0/24`) vale poco para quien
  lo encuentre. Detrás de un proxy inverso, declara el proxy en
  `DUMBMONIT_TRUSTED_PROXIES`, de lo contrario cada llamada parece venir
  del proxy y una restricción de red las rechaza todas.
- **Vinculado a su creador.** Un token deja de funcionar cuando se
  desactiva la cuenta que lo creó, pierde su poder de escritura cuando
  esa cuenta ya no es administradora, y se revoca cuando esa cuenta se
  elimina.
- **Nada secreto vuelve.** Las herramientas nunca devuelven
  credenciales de dispositivo (solo su tipo), secretos de canal, URLs
  de heartbeat ni tokens. `add_device` sí recibe credenciales: viajan
  a través de la conversación de tu asistente, y por tanto a través de
  su proveedor. Para un dispositivo con credenciales sensibles,
  prefiere la interfaz web.
- **Navegadores y `Origin`.** Una petición que lleva una cabecera
  `Origin` (un cliente basado en navegador) se rechaza con `403` a
  menos que ese origen sea el propio del servidor o esté permitido por
  `DUMBMONIT_API_CORS_ORIGINS` (por defecto cualquier origen, ya que el
  token se requiere de todas formas). Lista tus orígenes ahí para
  limitarlo; consulta [peticiones entre
  orígenes](../reference/api.md#cross-origin-requests-cors).
- **Límite de tasa.** Cada token se limita a 120 llamadas por minuto,
  REST y MCP juntos: un asistente atascado en un bucle recibe `429`
  con `Retry-After`, no toda la atención de tu servidor.
- **HTTPS antes de internet.** Los connectors de ChatGPT necesitan una
  URL pública. Pon DumbMonit detrás de un proxy inverso con TLS
  (Caddy, Traefik, nginx…) y define `DUMBMONIT_COOKIE_SECURE=1`. Nunca
  expongas el puerto HTTP plano. Claude Desktop, Claude Code, VS Code
  y Cursor se ejecutan en tu máquina y pueden alcanzar una dirección
  LAN directamente, así que no necesitan ninguna exposición en
  absoluto.
- **Vigila «last used».** La lista de tokens muestra cuándo se usó
  cada token por última vez y desde qué dirección. Un token usado en un
  momento en que no estabas hablando con tu asistente, o desde una
  dirección que no conoces, merece una revocación.
- **Logs.** Cada llamada a herramienta se registra a nivel `info` con
  el nombre del token y la herramienta, nunca los argumentos; cada
  escritura a través de la API REST con el nombre del token, el
  método, la ruta y el estado, nunca el cuerpo. Crear y revocar tokens
  está en el registro de auditoría (Settings → Security).

## Solución de problemas {#troubleshooting}

| Síntoma | Causa y solución |
| --- | --- |
| `401` / «A valid API token is required.» | Falta la cabecera o está mal escrita (`Authorization: Bearer dmt_…`, un espacio después de `Bearer`), o el token fue revocado. Crea uno nuevo. |
| `401` / «The API token "…" expired on …» | Crea un token nuevo; elige una vida más larga si era demasiado corta. |
| `403` / «cannot be used from …» | El token está restringido a otras redes. La dirección en el mensaje es la que ve el servidor: detrás de un proxy inverso, declara el proxy en `DUMBMONIT_TRUSTED_PROXIES`. |
| `403` en cada llamada desde un cliente basado en navegador | Su `Origin` no está permitido: añádelo a `DUMBMONIT_API_CORS_ORIGINS`. |
| Una herramienta responde «needs a token with the "write" scope» | El token es de solo lectura, o su creador ya no es administrador. Crea un token de escritura si quieres que el asistente actúe. |
| `429` | 120 llamadas en un minuto: el asistente probablemente está en un bucle. Espera a `Retry-After`, luego haz una pregunta más limitada. |
| «Unsupported protocol version» | El cliente habla una versión que DumbMonit no conoce; actualiza el cliente. |
| Claude Desktop muestra el servidor como fallido | Node.js debe estar instalado para `npx`; por HTTP plano, `--allow-http` debe estar en `args`. |
| ChatGPT no puede conectar | Necesita una URL HTTPS pública; una dirección LAN o HTTP plano no funciona desde los servidores de OpenAI. |
