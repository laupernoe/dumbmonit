# Settings

![Settings: siete pestañas, aquí Mi cuenta con apariencia, cuenta y seguridad y notificaciones push](../assets/screenshots/settings-light.png){ loading=lazy }

Settings contiene lo administrativo: tu cuenta, quién puede iniciar sesión y cómo, los tokens que dejan entrar a agentes y asistentes, hacer copia de seguridad de la instancia, el tema, y la salud del servidor. Se divide en siete pestañas, cada una con su propia dirección. Una barra lateral en pantalla ancha, una franja de chips en un teléfono y un buscador sobre los nombres de las secciones ayudan a encontrar una sección.

| Pestaña | Dirección | Qué contiene |
|---|---|---|
| Mi cuenta | `/settings/account` | Apariencia, Cuenta y seguridad (contraseña, dos factores, registro de seguridad), Notificaciones push |
| Equipo y acceso | `/settings/team` | Usuarios, Inicio de sesión único |
| Supervisión | `/settings/monitoring` | Agentes, Paquetes de integración, [Puntuación de seguridad](security-score.md) |
| Notificaciones e informes | `/settings/notifications` | Canales, Política de notificaciones, Horas de silencio ([Canales de notificación](../notifications.md)), Informes |
| Páginas de estado | `/settings/status` | Páginas de estado y anuncios ([Páginas de estado](status-pages.md)) |
| Pantalla de pared | `/settings/wall` | Modo pared, Música de la pared, Noticias de la pared ([Modo pared](overview.md#wall-mode)) |
| Sistema | `/settings/system` | Copia de seguridad, El propio DumbMonit, API y asistentes, Acerca de |

Cada sección conserva su ancla: `/settings#agents` redirige a `/settings/monitoring#agents`. Los enlaces antiguos (`/alerts#notifications`, `/status`) también redirigen al nuevo lugar.

## Appearance

Tres tarjetas, cada una previsualizando su propio tema: **System**
(sigue a tu dispositivo y cambia con él), **Day** (el tema «papel de
gráfica») y **Night** (el tema «radar»). La elección se guarda en el
navegador. El interruptor de la cabecera y el *Toggle theme* de la
paleta de comandos cambian entre día y noche.

**Open wall mode** se sitúa al final de la sección: solo el boletín, a
pantalla completa, para un monitor en la sala. ++esc++ lo abandona, y la
paleta de comandos lo abre desde cualquier sitio — consulta [Modo
pared](overview.md#wall-mode).

## Account & security {#account-security}

Esta es tu propia cuenta. La primera se crea en el primer arranque, y
se pueden añadir más abajo. Aquí cambias tu contraseña: contraseña
actual, contraseña nueva (al menos 12 caracteres; una frase entera es
más segura que una palabra complicada), confirmación. Cambiarla cierra
cada otra sesión. Las cuentas que inician sesión a través del proveedor
de identidad no tienen contraseña aquí. **Sign out** termina esta
sesión.

El inicio de sesión tiene limitación de tasa por dirección de cliente y
por cuenta: tras cinco intentos fallidos, cada intento adicional espera
más (30 s, duplicando, hasta 5 minutos). Detrás de un proxy inverso,
define `DUMBMONIT_TRUSTED_PROXIES` para que el limitador vea la
dirección real del cliente en lugar de la del proxy (consulta
[Configuración](../reference/configuration.md)). Las sesiones duran 30
días. Una contraseña olvidada se restablece con
`DUMBMONIT_RESET_PASSWORD=1`: consulta las [preguntas
frecuentes](../faq.md#i-lost-the-password).

### Autenticación de dos factores {#two-factor-authentication}

Las cuentas que inician sesión con contraseña pueden añadir un segundo
factor: un código de seis dígitos basado en tiempo (TOTP) de una app
autenticadora — Aegis, FreeOTP, Google Authenticator, 1Password,
Bitwarden… **Set up** pide tu contraseña, muestra un código QR para
escanear (o la clave para escribir), y activa el segundo factor una vez
que introduces un primer código de la app. Luego se muestran ocho
**códigos de recuperación** una vez: guárdalos, cada uno te permite
iniciar sesión una sola vez si pierdes el teléfono. La pantalla de
inicio de sesión pide el código (o un código de recuperación) después
de la contraseña; cinco códigos incorrectos cancelan el intento y
empiezas de nuevo desde la contraseña, y el límite de tasa de inicio de
sesión habitual sigue contando entre intentos. Un código se acepta una
vez: el que acabas de usar (incluido el que activó el segundo factor) no
te deja entrar de nuevo — espera al siguiente.

**Disable** pide la contraseña de nuevo. Los códigos de recuperación no
se pueden regenerar por sí solos: desactiva y vuelve a configurar para
obtener un juego nuevo. Un administrador puede restablecer el segundo
factor de otro usuario desde **Users** (*Reset 2FA*) cuando se pierden
tanto el teléfono como los códigos — eso cierra la sesión del usuario
en todas partes y deja la contraseña intacta. El secreto se guarda
cifrado con el secreto de la instancia; los códigos están hasheados.
Las cuentas de inicio de sesión único obtienen su segundo factor del
proveedor de identidad, no aquí.

Los administradores también encuentran el **registro de seguridad** en
esta sección: inicios de sesión y fallos, cambios de contraseña y
doble factor, cambios de token y cuenta, con la dirección del cliente.
Conserva las últimas 5000 entradas.

## Push notifications {#push-notifications}

Notificaciones nativas en este teléfono u ordenador, incluso con
DumbMonit cerrado: **Enable on this device**, luego **Send a test**.
Cada cuenta gestiona sus propios dispositivos aquí (también los
visores); qué alertas les llegan se define con un canal **Web Push** en
Settings → Notifications & reports. El botón **i** resume los requisitos: HTTPS (o
localhost), instalación en pantalla de inicio en iPhone y iPad, y los
dominios de servicio push que el servidor debe alcanzar. Guía completa:
[Notificaciones push (PWA)](push-notifications.md).

## Users

Solo administradores. Las cuentas que pueden iniciar sesión, y su rol:

| Rol | Lee todo | Gestiona alertas (ack, snooze, ignore, ventanas de mantenimiento, limpiar historial) | Cambia la configuración (monitores, reglas, canales, páginas de estado, ajustes, copias de seguridad, tokens, cuentas) |
|---|---|---|---|
| **Viewer** | sí | no | no |
| **Operator** | sí | sí | no |
| **Admin** | sí | sí | sí |

Un visor ve cada página con cada control que cambiaría algo oculto. Algunas cosas
siguen siendo solo de administrador incluso para leerlas: los tokens de API y
de agente, los correos de suscriptores de páginas de estado y las opciones de
monitor que parecen secretos (una opción cuyo nombre contiene `key`,
`token`, `password`…, las cabeceras extra de una comprobación HTTP, la
contraseña de una dirección `https://user:pass@host`), que los no
administradores leen como `***`. Un
operador es el rol de guardia: los controles de alerta están ahí, las
pantallas de configuración dicen *Operator — config is admin only*. Solo
un administrador crea cuentas o cambia un rol — un operador no puede
promover a nadie, ni a sí mismo. Las secciones *Users*, *Single
sign-on* y el registro de seguridad se mantienen solo para
administradores. **Add user** pide un nombre de usuario, un nombre
mostrado opcional, un rol y una contraseña; el botón junto al campo de
contraseña genera una aleatoria. La contraseña se muestra una vez, para
entregarla.

Cada fila lleva el rol, si la cuenta inicia sesión con contraseña o a
través del proveedor de identidad, si el doble factor está activado, y
el último inicio de sesión. **Edit** cambia el nombre mostrado, el rol y
la contraseña (un restablecimiento cierra la sesión de ese usuario en
todas partes y muestra la contraseña nueva una vez). El interruptor
**Enabled** suspende una cuenta sin eliminarla; **Reset 2FA** aparece
cuando la cuenta tiene un segundo factor; **Delete** la elimina para
siempre. El servidor conserva al menos un administrador activo, y los
controles reflejan esa regla en lugar de reportarla como un error: el
último administrador no se puede degradar, desactivar ni eliminar, y no
puedes desactivar ni eliminar tu propia cuenta.

## Single sign-on

Solo administradores. Inicio de sesión a través de un proveedor OpenID
Connect (Authelia, Authentik, Keycloak, Pocket ID, Google Workspace…).
Rellena el *provider name* (etiqueta el botón «Continue with …» en la
pantalla de inicio de sesión), la *issuer URL* — el descubrimiento se
lee de `{issuer}/.well-known/openid-configuration` — el *client id*, el
*client secret* (guardado cifrado, nunca vuelto a mostrar) y la
*public URL* a la que los navegadores alcanzan DumbMonit. El formulario
construye el URI de redirección a partir de esa URL pública: cópialo y
regístralo con tu proveedor.

**Test discovery** lee la configuración del proveedor sin iniciar
sesión a nadie, lista los endpoints que encontró, y avisa cuando el
proveedor firma ni con RS256 ni con ES256 — las únicas dos firmas que
acepta DumbMonit. Los ajustes guardados aquí tienen precedencia sobre
las variables de entorno `DUMBMONIT_OIDC_*`, que solo aplican mientras
nada esté guardado; **Forget saved settings** las borra y vuelve a las
variables cuando existen.

**Advanced.** El resto del formulario está bajo *Advanced*: *Scopes*
nombra qué pedirle al proveedor, *Groups claim* la reclamación del
token que lista los grupos del usuario, y *Create accounts on first
sign-in* decide si una persona desconocida obtiene una cuenta. Está
**desactivado por defecto**: solo las cuentas que ya existen pueden
iniciar sesión hasta que lo activas.

**Roles.** *Admin groups* lista los grupos cuyos miembros se convierten
en administradores; *Operator groups* los grupos cuyos demás miembros
se convierten en operadores; todos los demás son visores. Deja ambos
vacíos para seguir gestionando roles bajo **Users**. Con solo *Operator
groups* rellenado, los grupos nunca tocan el rol de administrador: los
administradores siguen siendo administradores, los demás se vuelven
operadores o visores. Los roles se reevalúan en cada inicio de sesión,
excepto que el último administrador activo nunca se degrada. Las
variables de entorno correspondientes son `DUMBMONIT_OIDC_ADMIN_GROUPS`
y `DUMBMONIT_OIDC_OPERATOR_GROUPS`.

**En qué cuenta aterriza un inicio de sesión.** La identidad se recuerda
por su sujeto de proveedor tras el primer inicio de sesión. En un
primer inicio de sesión:

- Una cuenta existente se vincula solo cuando el proveedor afirma
  `email_verified: true` y ese correo es el nombre de usuario de la
  cuenta. Un `preferred_username` o un correo no verificado nunca
  vincula: en la mayoría de proveedores el usuario los elige, así que
  `admin` del proveedor no debe convertirse en *tu* `admin`.
- Un **administrador local que tiene contraseña nunca se vincula
  automáticamente**, incluso con un correo verificado: el inicio de
  sesión crea una cuenta distinta en su lugar. Sigue gestionando a ese
  administrador con su contraseña, o crea un administrador SSO separado
  a través de los grupos de administrador.
- En caso contrario, con *Create accounts on first sign-in* activado,
  se crea una cuenta nueva (nombre de usuario del proveedor, con sufijo
  `-1`, `-2`… cuando está ocupado). Con él desactivado, solo puede
  iniciar sesión el vínculo de correo verificado de arriba.

**La creación de cuentas está desactivada a menos que la elijas.** Las
instalaciones nuevas empiezan con *Create accounts on first sign-in*
desactivado, y también cualquier configuración donde nunca se definió —
ajustes guardados antes de que existiera la opción, o configuraciones
solo por entorno sin `DUMBMONIT_OIDC_AUTO_CREATE`. Las versiones
anteriores trataban «nunca definido» como activado; el panel Single
sign-on ahora indica cuándo se aplica este valor por defecto. Un valor
que guardaste explícitamente se conserva tal cual.

## Agents

Tokens de inscripción para el [agente de Linux, macOS, FreeBSD y
Windows](../devices/agent.md). Crea uno con un nombre («File server»,
«Home fleet»): el token se muestra una vez, con los comandos de
instalación de Linux y Windows listos para copiar. Cada comando lleva
los checksums SHA-256 de los binarios de agente que distribuye este
servidor, y el instalador rechaza una descarga que no coincida; los
checksums también se listan bajo los comandos, para comparar a mano. La
lista muestra el prefijo de cada token, fecha de creación, último uso y
si fue revocado. **Revoke** detiene cada agente que use ese token en su
siguiente envío.

Un token puede inscribir varias máquinas. Revocarlo no elimina los
monitores. La lista de tokens, crear y revocar son para administradores;
los visores y operadores ven la sección sin la lista.

## Puntuación de seguridad {#security-score}

En **Settings → Supervisión**, bajo los agentes: una puntuación de lo bien reforzadas que están la instancia y los equipos supervisados. Consulta [Puntuación de seguridad](security-score.md).

## Reports

En **Settings → Notifications & reports**, bajo la política: correos
periódicos de disponibilidad para el equipo. Elige semanal,
diario o mensual, el día, hora y zona horaria, los destinatarios y el
canal de correo, y luego **Send a preview** para ver el resultado. Solo
administradores. Consulta [Informes por correo](reports.md).

## Backup

Tres cosas, en el orden en que las encuentras.

Un **aviso que no puedes pasar por alto**: `/data/secret.key` es lo que
descifra las credenciales de monitor, y una copia de la base de
datos sin él restaura una instancia que no puede hablar con nada. La
redacción cambia cuando el secreto viene de `DUMBMONIT_SECRET` en su
lugar — entonces no hay ningún archivo que copiar, y es tu gestor de
contraseñas el que tiene que guardarlo.

**Export the configuration** lista lo que contendría el paquete, con un
recuento por sección, y luego pide una frase de paso dos veces. El
archivo que se descarga contiene cada credencial de esta instancia,
cifrada solo con esa frase de paso: guárdalo donde guardas las
contraseñas. Las contraseñas de cuenta y secretos 2FA se excluyen a
menos que actives el interruptor.

**Restore a bundle** toma el archivo y su frase de paso y muestra
primero una **simulación**: una tabla de qué se crearía, actualizaría y
dejaría en paz, sección por sección, con una nota para cualquier cosa
que no pueda ubicar. Nada se escribe hasta que pulsas *Restore for
real*.

**Scheduled local backups** muestra si están activadas, dónde van, la
última ejecución y si tuvo éxito, y los archivos conservados con su
tamaño. **Back up now** escribe una de inmediato — lo que hay que hacer
antes de una actualización.

Todo aquí es solo para administradores. Consulta [Copia de seguridad y
restauración](../install/backup.md).

## El propio DumbMonit {#self-monitoring}

El estado del motor en este momento — planificador, base de datos SQLite, base
de métricas — con el espacio libre del volumen de datos, el tamaño de la base
y, para el VictoriaMetrics integrado, cuántas veces se ha reiniciado. Unas
reglas integradas ya le avisan cuando algo de esto falla.

Debajo, el **latido saliente**: una URL suya (Healthchecks.io, un monitor push
de Uptime Kuma, Better Stack…) que DumbMonit llama a intervalos regulares, solo
mientras está en buen estado, para que alguien desde fuera note cuando este
equipo deja de responder. **Probar** envía una llamada al momento; **Señalar
fallos** llama a la URL de fallo en lugar de callar. La URL se guarda cifrada y
no se vuelve a mostrar, solo su host. Solo un administrador puede cambiarla.
Véase [Watching DumbMonit itself](../install/self-monitoring.md).

## API & assistants

Un tipo de token para ambos: clientes MCP y la [API REST](../reference/api.md).
Nombra un token, elige su alcance y créalo — se muestra una vez. Un
token **read** solo puede mirar; un token **read and write** también
puede cambiar cosas — silenciar un monitor, ejecutar una sonda,
añadir o editar monitores, reglas y canales — pero nunca cuentas,
ajustes de inicio de sesión u otros tokens.

Bajo **Connect**, tres pestañas contienen los fragmentos, rellenados
con el token mientras sigue en pantalla: *Claude* (el comando `claude
mcp add` y el bloque `claude_desktop_config.json`), *ChatGPT* (la URL
del servidor MCP y la cabecera de autorización, con un aviso cuando
esta página no se sirve por HTTPS — ChatGPT se conecta desde los
servidores de OpenAI, así que la dirección debe ser accesible desde
internet) y *Cursor / other*, la misma forma para cualquier cliente que
hable Streamable HTTP con una cabecera bearer. **API access** muestra el
mismo token en la API REST como un ejemplo `curl`, y señala las rutas
que un Prometheus o un Grafana sondean con él. Consulta [Conectar un
asistente](assistant.md) y [Sondear
DumbMonit](../reference/metrics.md#scraping-dumbmonit).

## About

Versión del servidor y la salud de sus dos almacenes, la base de datos
(configuración de monitor e historial de alertas) y VictoriaMetrics
(cada medición en las gráficas), tal como lo reporta `GET /api/health`
y se refresca cada 15 segundos. Un almacén que deja de responder muestra
un aviso con el error y qué comprobar. La sección también enlaza a esta
documentación y nombra la licencia, Apache 2.0.
