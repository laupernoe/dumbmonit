# Preguntas frecuentes {#faq}

## Perdí la contraseña {#i-lost-the-password}

Primero pregunta a otro administrador: en Settings → **Users**, **Edit**
en tu cuenta define una contraseña nueva y la muestra una vez, y **Reset
2FA** en la misma fila borra un segundo factor que te deja fuera. Esa es
la respuesta habitual — una instancia con dos cuentas de administrador
nunca necesita el resto de esta página.

Cuando nadie puede iniciar sesión ya, no hay ningún correo de
recuperación: arranca el servidor una vez con
`DUMBMONIT_RESET_PASSWORD=1`:

```bash
DUMBMONIT_RESET_PASSWORD=1 docker compose up -d dumbmonit
```

Cada cuenta y cada sesión se elimina al arrancar y la interfaz vuelve a
mostrar `/setup`. Crea el primer administrador, y luego arranca de nuevo
sin la variable (`docker compose up -d dumbmonit`, con la variable
indefinida o vacía en tu entorno). Los dispositivos, reglas, canales y
los ajustes de inicio de sesión único no se tocan.

## Sin datos después de añadir un dispositivo {#no-data-after-adding-a-device}

Dale un minuto: la primera sonda se ejecuta al intervalo del dispositivo
(60 s por defecto) y las escrituras llegan a VictoriaMetrics cada 5
segundos (`DUMBMONIT_FLUSH_INTERVAL_SECS`). Luego:

1. Abre la página del dispositivo y haz clic en **Probe now**. Reporta
   cuántas muestras y series produjo una sonda, o el error exacto.
2. *Misconfigured* con un error bajo la dirección es un problema de
   configuración: comunidad o contraseña incorrecta, certificado
   rechazado (activa «Accept an unverifiable certificate»), capacidad
   faltante. Se muestra, no se alerta.
3. *Unreachable* sin ningún error significa que el dispositivo no
   respondió en absoluto. Para SNMP, una comunidad incorrecta se parece
   exactamente a un dispositivo apagado, porque el dispositivo nunca
   responde a ella. Comprueba la comunidad, que SNMP esté activado, y
   que el dispositivo permita la dirección del servidor.
4. Comprueba que el contenedor puede alcanzar el dispositivo: las redes
   Docker y los cortafuegos son los culpables habituales.

## Ping dice «error de configuración» {#ping-says-configuration-error}

El monitor de ping necesita un socket de echo ICMP. El contenedor se
ejecuta sin ninguna capacidad, así que el archivo Compose permite esos
sockets con el sysctl `net.ipv4.ping_group_range` bajo el servicio
`dumbmonit`: asegúrate de que las líneas `sysctls:` sigan ahí, y luego
`docker compose up -d`. Consulta [Ping ICMP sin una
capacidad](install/docker.md#icmp-ping-without-a-capability).

## El agente no aparece {#the-agent-does-not-appear}

- El instalador hace un envío de prueba antes de iniciar el servicio y
  se detiene con un error explícito si la URL o el token son
  incorrectos. Corrige `/etc/dumbmonit/agent.yaml` y reinicia
  `dumbmonit-agent`.
- La URL en el comando de instalación es la que tu navegador usó para
  alcanzar la interfaz. Desde la máquina monitorizada, esa URL debe
  alcanzar el servidor: `curl http://server:8080/api/health` desde ahí
  te lo dice.
- Detrás de un proxy inverso, `/api/ingest` debe reenviarse y permitir
  cuerpos de hasta 16 MB (`client_max_body_size 16m` en nginx).
- `journalctl -u dumbmonit-agent -f` muestra cada envío rechazado. El
  propio token nunca aparece en el registro.
- Un token revocado detiene el agente en su siguiente envío; vuelve a
  ejecutar el instalador con un token nuevo.

Consulta [Agente para Linux, macOS, FreeBSD y Windows](devices/agent.md).

## Cambiar el puerto {#changing-the-port}

El contenedor escucha en el 8080; el puerto del host es el que publique
`docker-compose.yml`. Define `DUMBMONIT_PORT` (en un archivo `.env` junto
al archivo Compose, o en la línea de comandos):

```bash
DUMBMONIT_PORT=8099 docker compose up -d
```

Para cambiar el puerto *dentro* del contenedor, define `DUMBMONIT_BIND`,
por ejemplo `0.0.0.0:9000`, y ajusta el mapeo de puertos.

## ¿Por qué la alerta es «Advisory» cuando la regla dice «warning»? {#why-is-the-alert-advisory-when-the-rule-says-warning}

La API almacena tres severidades: `info`, `warning`, `critical`. La
interfaz las muestra con las palabras del boletín meteorológico: Info,
Advisory, Warning. Consulta
[Severidades](alerting/index.md#severities).

## Una alerta está «Suppressed by parent» {#an-alert-is-suppressed-by-parent}

El dispositivo tiene un dispositivo padre que es inaccesible, así que sus
propias alertas se retienen en lugar de enviarse. Arregla el padre; las
alertas del hijo se resuelven o se activan por su cuenta después, sin una
segunda notificación. Consulta [Supresión por
dependencia](alerting/index.md#dependency-suppression).

## «Unusual CPU» nunca se activa {#unusual-cpu-never-fires}

Es una referencia estacional que permanece en silencio durante sus
primeros 14 días en cada serie. Durante ese tiempo solo muestra lo que
habría activado. Consulta [Aprendizaje de
referencia](alerting/index.md#baseline-learning).

## ¿Puedo usar mi propia VictoriaMetrics? {#can-i-use-my-own-victoriametrics}

Sí: define `DUMBMONIT_VM_URL` con su dirección (`http://host:8428`) y la
VictoriaMetrics integrada no se inicia. Cada métrica tiene el prefijo
`dumbmonit_`, así que puede compartir la instancia con otras
herramientas. `GET /api/health` reporta `victoria.embedded: false` una
vez hecho el cambio.

## ¿DumbMonit necesita ser accesible desde internet? {#does-dumbmonit-need-to-be-reachable-from-the-internet}

No. Él mismo contacta a los dispositivos y a los servicios de
notificación; nada le devuelve la llamada, excepto los agentes, que
necesitan alcanzar la URL del servidor. Mantenlo en tu LAN o detrás de
una VPN; si lo expones, usa un proxy inverso con TLS y define
`DUMBMONIT_COOKIE_SECURE=1`.

## ¿Hay un token de API? {#is-there-an-api-token}

Sí. Settings → **API & assistants** crea tokens `dmt_…` con un alcance
`read` o `write`; envía uno como `Authorization: Bearer dmt_…` en
cualquier ruta y no se necesita ninguna cookie ni cabecera CSRF. El mismo
token conecta un [asistente](using/assistant.md) a través de MCP. Los
tokens nunca gestionan cuentas, ajustes de inicio de sesión ni otros
tokens. Consulta la [referencia de la API HTTP](reference/api.md#authentication).
