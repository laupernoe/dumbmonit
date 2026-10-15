# Notificaciones push (PWA) {#push-notifications-pwa}

DumbMonit puede hacer sonar tu teléfono u ordenador por sí mismo, como lo
hace una app nativa, sin instalar ninguna app ni crear una cuenta en
ningún sitio: el propio servicio push del navegador (Google, Mozilla,
Apple, Microsoft) transporta un mensaje cifrado de extremo a extremo
desde tu servidor hasta el dispositivo, y la notificación aparece incluso
con todas las pestañas de DumbMonit cerradas. Esto es el estándar **Web
Push**; el sitio también se puede instalar como una app (una *progressive
web app*, PWA) para que se abra desde un icono como cualquier otra.

Dos pasos: activa el push en cada dispositivo (esta página), y luego
decide qué alertas llegan ahí con un canal **Web Push**
([Canales de notificación](../notifications.md#web-push)).

## Antes de empezar {#before-you-start}

- **Se requiere HTTPS.** Los navegadores solo ofrecen push a un origen
  seguro. Pon DumbMonit detrás de un proxy inverso con un certificado
  (Caddy, Traefik, Nginx Proxy Manager…) y ábrelo a través de esa
  dirección. `http://localhost` también se acepta, lo cual solo es útil
  para probar en el propio servidor. En un `http://192.168.x.x` plano, la
  sección indica **Needs HTTPS**.
- **El servidor necesita HTTPS saliente** hacia los servicios push:
  `fcm.googleapis.com` (Chrome, y Edge en Android), `*.notify.windows.com`
  (Edge en Windows), `updates.push.services.mozilla.com` (Firefox) y
  `*.push.apple.com` (Safari, iPhone, iPad). Nada entra: tu DumbMonit no
  necesita ser accesible desde internet, el teléfono solo tiene que
  alcanzarlo cuando pulsas una notificación.
- **Define `DUMBMONIT_PUBLIC_URL`** con la dirección HTTPS que usas: se
  entrega a los servicios push como contacto del remitente, y es la
  dirección a la que apuntan los enlaces en otros canales.

## Activar en un teléfono u ordenador {#enable-on-a-phone-or-computer}

### Android, Windows, macOS, Linux

1. Abre DumbMonit en Chrome, Edge, Firefox o Safari e inicia sesión.
2. Opcional: instálalo como app (barra de direcciones → **Install**, o el
   menú del navegador → **Add to Home screen**). Las notificaciones
   funcionan de cualquiera de las dos formas.
3. **Settings → Push notifications → Enable on this device**, y permite
   las notificaciones cuando el navegador lo pregunte.
4. **Send a test.** Debería aparecer un *DumbMonit — test notification*
   en unos segundos.

### iPhone e iPad

Apple solo ofrece push a sitios añadidos a la pantalla de inicio (iOS y
iPadOS 16.4 o posteriores):

1. Abre DumbMonit en **Safari**.
2. **Share → Add to Home Screen**, y luego abre DumbMonit **desde el
   nuevo icono**.
3. Inicia sesión, luego **Settings → Push notifications → Enable on this
   device**, y permite las notificaciones.
4. **Send a test.**

En Safari sin instalar, la sección indica **Install first**.

## Elegir qué alertas llegan {#choose-which-alerts-arrive}

En **Settings → Notifications & reports**, añade un canal de tipo **Web Push**. Sin
ningún ajuste, cada dispositivo suscrito de cada cuenta lo recibe; lista
nombres de cuenta en **Accounts** para dirigirlo solo a algunas personas.
Se aplica la política de canal habitual (severidad mínima, horas de
silencio, filtro de enrutado…), así que puedes, por ejemplo, reservar un
teléfono solo para alertas críticas.

Una notificación muestra `Critical · nas01` con un resumen breve; al
pulsarla se abre la página **Alerts**. La resolución de una alerta
reemplaza su notificación en lugar de añadir una segunda.

## Dispositivos y cuentas {#devices-and-accounts}

Cada cuenta ve y gestiona sus propios dispositivos: los visores también
pueden activar el push en su teléfono, no cambia nada más. Una cuenta
guarda como máximo 10 dispositivos (elimina uno antiguo para añadir otro), y
se puede enviar una prueba una vez cada 30 segundos. La lista
muestra el navegador y sistema, el servicio push, cuándo se añadió el
dispositivo y cuándo recibió el último mensaje; **Remove** lo olvida (y,
en el dispositivo que estás usando, también desuscribe el navegador). Un
dispositivo que el servicio push reporta como desaparecido (se eliminó la
app, se borraron los datos del navegador) se elimina automáticamente en
el siguiente envío.

## Solución de problemas {#troubleshooting}

| Lo que ves | Qué hacer |
|---|---|
| **Needs HTTPS** | Abre DumbMonit a través de su dirección HTTPS. |
| **Install first** (iPhone, iPad) | Share → Add to Home Screen, ábrelo desde el icono. |
| **Blocked** | Las notificaciones fueron rechazadas para este sitio: permítelas en los ajustes del sitio del navegador (el candado en la barra de direcciones, o iOS Settings → Notifications → DumbMonit), y luego recarga. |
| **Not supported** | El navegador no tiene Web Push (versión antigua, algunos navegadores centrados en privacidad): usa un Chrome, Edge, Firefox o Safari reciente. |
| La prueba dice *push service unreachable* | El servidor no puede alcanzar el servicio push: comprueba su HTTPS saliente y el DNS. |
| La prueba dice *refused the VAPID signature* | Normalmente un reloj del servidor incorrecto; comprueba el NTP en el host. |
| Nada llega en Android cuando el teléfono duerme | La optimización de batería puede retrasar el navegador: exímelo, o define la **Urgency** del canal en `high`. |

## Privacidad y seguridad {#privacy-and-security}

- Los mensajes se cifran para cada dispositivo (RFC 8291) antes de salir
  del servidor: el servicio push solo ve un blob opaco, el dispositivo lo
  descifra.
- El servidor firma cada mensaje con su propio par de claves VAPID (RFC
  8292), generado en el primer arranque y guardado cifrado con el secreto
  de la instancia; la clave privada nunca se devuelve por la API. Una
  [restauración](../install/backup.md) en una máquina nueva genera un par
  de claves nuevo: activa el push de nuevo en cada dispositivo.
- La dirección de suscripción viene del navegador, así que el servidor
  solo acepta direcciones HTTPS en IPs públicas: las direcciones
  privadas, loopback y link-local se rechazan, tanto al suscribirse como
  en cada conexión posterior.
- El service worker (`/sw.js`) solo muestra notificaciones. No cachea
  nada y no intercepta ninguna petición: la interfaz y la API siempre
  vienen del servidor, nunca de una copia obsoleta.
