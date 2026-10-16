# Páginas de estado {#status-pages}

![La página Status: tus páginas, y los anuncios bajo ellas](../assets/screenshots/status-light.png){ loading=lazy }

Una página de estado es la cara pública de tu monitorización: una
página que cualquiera puede abrir — sin iniciar sesión, sin cookie —
que indica si tus servicios están activos, muestra hasta 90 días de
historial diario de cada uno, lleva tus anuncios de incidente y
mantenimiento, y permite a los visitantes seguirlos por RSS o correo.
Los badges ponen la misma respuesta en un README; una vista compacta
cabe en una página de intranet.

Solo muestra lo que tú pones en ella: la etiqueta que le das a cada
servicio, su estado, su disponibilidad y su tiempo de respuesta. Los
nombres de monitor, direcciones, tipos e identificadores nunca
salen del servidor — ni en la página, ni en un badge, ni en un correo.

## 1. Crear una página {#1-create-a-page}

**Settings → Status pages** → **New page**. Esa página lista tus
páginas de estado (abrir, editar, eliminar) con los anuncios bajo
ellas; el editor se abre en su propia ruta (`/settings/status/new`,
`/settings/status/<id>`). El renderizado público está en `/s/<address>`.

| Campo | Qué hace |
| --- | --- |
| **Title** | El encabezado de la página («Home lab»). |
| **Address** | La última parte de la URL, `/s/<address>`. Sugerida a partir del título; letras minúsculas, dígitos y guiones, de 2 a 40 caracteres. |
| **Description** | Una frase bajo el título. Opcional. |
| **Theme** | Day, night, o sigue el sistema del visitante. |
| **History** | La ventana más larga que cubre la barra de historial (30, 60 o 90 días). La barra se adapta a la antigüedad de los datos de la página: muestra tantos días como el servicio más antiguo ha sido monitorizado, desde 7 días hasta este ajuste. Una página nunca dice más que esto — consulta [Badges](#badges). |
| **Simple mode** | Una página plana: sin escena, sin mascota, sin animación y sin huevos de pascua. Claro u oscuro sigue el sistema del visitante, con un interruptor en la cabecera; el ajuste Theme de la página se ignora. |
| **Published** | Desactivado = borrador: la página, sus badges y su logo responden «not found» a los visitantes hasta que la activas. |

Luego marca los monitores a mostrar. Para cada uno, define la
**label** que leerán los visitantes (por defecto el nombre del
monitor — puedes preferir «Website» a «nginx-front-01») y,
opcionalmente, un **group** («Network», «Storage»). Los grupos se
convierten en bloques en la página, en el orden en que organizas los
servicios.

Guarda, y el enlace está listo para copiar. La página se refresca por
sí misma cada minuto.

### Look

La sección **Look** del editor viste la página con tus colores sin
abrirla a código arbitrario:

| Ajuste | Qué hace |
| --- | --- |
| **Logo** | PNG, JPEG o WebP, hasta 256 KiB, mostrado junto al título. Cuadrado funciona mejor. SVG se rechaza: puede llevar script, y la página se sirve desde la misma dirección que la administración. El archivo se comprueba por su contenido, se guarda bajo `/data/status-pages/`, y se sirve solo como imagen. |
| **Accent** | Ink (por defecto), blue, teal, violet, rose o amber — la línea superior, los enlaces y el botón de suscripción. Cada uno se elige para seguir siendo legible de día y de noche. |
| **Organisation website** | Un enlace `http://` o `https://` mostrado junto al título, como el nombre de host del sitio. |
| **Footer text** | Texto plano bajo la página, hasta 280 caracteres: quién la gestiona, cómo contactarlos. Los saltos de línea se conservan; nada se interpreta como HTML o Markdown. |

### Scene

El ajuste **Scene** dibuja un horizonte de ciudad detrás del banner.
Las páginas nuevas empiezan con París, y las páginas existentes que no
tenían escena ahora también la muestran; desmárcalo para un banner
plano, o activa **Simple mode** para eliminar las escenas del todo.
Elige tantas como quieras, en el orden que prefieras. Ciudades:
Venecia, París, Tokio, Nueva York, Londres, Roma, Sídney, Dubái, San
Francisco, Barcelona, Ámsterdam, Istambul y Río de Janeiro. Las nuevas
siete maravillas: Chichén Itzá, Machu Picchu, la Gran Muralla, Petra y
el Taj Mahal (el Cristo Redentor de Río está en la escena de Río). Las
maravillas antiguas: Giza, Babilonia, Éfeso, Olimpia, Halicarnaso,
Rodas y Alejandría, más la Acrópolis de Atenas. Con más de una,
**Rotation** decide cuándo pasa la página a la siguiente: en cada
visita (por defecto), cada minuto, cada diez minutos o cada hora. Las
escenas son un conjunto cerrado de dibujos distribuidos con DumbMonit —
nada se sube ni se interpreta.

Deliberadamente no hay CSS o HTML personalizado: una página de estado
es pública y vive en el mismo origen que tu interfaz de
administración, bajo la misma [política de seguridad de
contenido](../reference/api.md).

## 2. Qué ven los visitantes {#2-what-visitors-see}

- Un banner con la respuesta de un segundo, leída de los servicios:
  **All systems operational**, **Partial outage** (algunos caídos o
  degradados), **Major outage** (todos caídos) o **Scheduled
  maintenance**. Un incidente abierto mientras todo servicio todavía
  responde se lee **Incident in progress**, matizado por su impacto.
- Los anuncios abiertos, la actualización más reciente primero, con la
  cronología de actualizaciones bajo un pliegue.
- Cada servicio con su placa de estado (Operational, Degraded, Down,
  Maintenance, No data), su **barra de historial** — un cuadrado por
  día — su disponibilidad en 24 h, 7 días, 30 días y (en una página de
  90 días) 90 días, y el tiempo de respuesta para las sondas de
  disponibilidad.
- **Past incidents**, por día, de los últimos 30 días.
- **Get updates**: el feed RSS, y un formulario de suscripción por
  correo cuando configuras uno (consulta [Suscriptores por
  correo](#email-subscribers)).

### La barra de historial {#the-history-bar}

Cada cuadrado es un día (UTC), matizado por la disponibilidad de ese
día: color completo en el 99.5 % y por encima, ámbar desde el 95 %,
rojo por debajo. **Un día sin ninguna medición es gris**, no verde —
un servicio añadido la semana pasada tiene 83 días grises, no 83
perfectos. Pasa el ratón sobre un cuadrado, o enfoca la barra y muévete
con las flechas (Home y End saltan a los extremos), para leer la
fecha, la disponibilidad, los **minutos de caída** y los incidentes
que tocaron ese día. Las palabras también están ahí para lectores de
pantalla: el estado nunca se cuenta solo por color.

Cómo se decide el estado:

| Estado | Significado |
| --- | --- |
| Operational | La última sonda tuvo éxito (sondas de disponibilidad), o el monitor reportó dentro de tres períodos de sondeo. |
| Degraded | Activo ahora, pero al menos una sonda falló en la última hora. |
| Down | La última sonda falló, el monitor dejó de reportar, o su configuración tiene un error. |
| Maintenance | Una ventana de mantenimiento está en curso y el servicio está caído: esperado, no una caída. O bien un anuncio en esta página, o una [ventana de mantenimiento](../alerting/maintenance.md) de alertas que cubre ese monitor. |
| No data | Nunca sondeado todavía, o desactivado. |

La disponibilidad para sondas (`http`, `tcp`, `dns`, `ping`, `tls`,
heartbeats) es la proporción de comprobaciones exitosas, de
`dumbmonit_probe_success`; la caída de un día son sus comprobaciones
fallidas multiplicadas por el intervalo de sonda. Para otros
monitores (SNMP, agent, Proxmox…) es la proporción de franjas de
cinco minutos en las que el monitor reportó al menos una vez
(`dumbmonit_up`), contado desde la primera medición — un monitor
añadido ayer no está «caído» durante los 89 días anteriores — y la
caída de un día son sus franjas silenciosas.

## 3. Anunciar incidentes y mantenimiento {#3-announce-incidents-and-maintenance}

En **Settings → Status pages**, bajo la lista de páginas, **Incidents and
maintenance** → **New announcement** (también ++ctrl+k++ → *Announce
an incident*).

- Un **incidente** tiene un título, un impacto (**minor** muestra la
  página como degradada, **major** como una caída) y pasa por
  *Investigating → Identified → Monitoring → Resolved*. Publica
  actualizaciones a medida que avanzas: cada una lleva un estado y un
  mensaje, y se convierte en la última línea que leen los visitantes.
  **Resolve** lo cierra con un mensaje final; entonces pasa a «Past
  incidents» durante 30 días.
- Una ventana de **maintenance** tiene un inicio y un fin y pasa por
  *Scheduled → In progress → Completed*. Mientras está en curso, el
  banner dice «Scheduled maintenance» y los servicios caídos se
  muestran como *Maintenance*.

Una [ventana de mantenimiento programada en Alerts](../alerting/maintenance.md)
hace lo mismo para el único monitor que cubre, sin un anuncio: ese
servicio se lee *Maintenance* en lugar de rojo mientras la ventana
está abierta, y el estado general de la página dice *Maintenance*
cuando nada más está caído o degradado. Solo las ventanas que nombran
un monitor se muestran así; el nombre y comentario de la ventana
permanecen privados.

Un anuncio se muestra en una página o en **todas las páginas**.

### Suscriptores por correo {#email-subscribers}

Los visitantes pueden suscribirse a tus anuncios por correo una vez
que eliges un **canal de correo (SMTP)** en el editor, bajo **Email
subscribers**. El canal es uno que ya configuraste bajo
[Notifications](../alerting/notifications.md); se reutilizan su
servidor y remitente, no sus propios destinatarios. Sin uno, la página
solo ofrece su feed RSS.

- **Doble confirmación (opt-in).** Un visitante introduce una
  dirección; DumbMonit envía un enlace de confirmación, y no se envía
  nada más hasta que se sigue. Una solicitud nunca confirmada se olvida
  tras dos días. El formulario responde la misma frase sea la
  dirección nueva, pendiente o ya suscrita, así que no le dice a nadie
  quién sigue tu página.
- **Qué se envía.** Un correo cuando se crea un anuncio, cuando se
  publica una actualización, y cuando cambia su estado — tanto para
  incidentes como mantenimiento, en esa página (o en cada página, para
  un anuncio mostrado en todas).
- **Darse de baja.** Cada correo termina con un enlace para darse de
  baja, y lleva las cabeceras `List-Unsubscribe` y
  `List-Unsubscribe-Post`, así que Gmail, Apple Mail y otros ofrecen su
  propio botón de un clic.
- **Límites.** El endpoint público de suscripción acepta cinco
  peticiones por dirección de cliente y sesenta en total por 15
  minutos, y no se envía correo dos veces a una dirección pendiente
  dentro de diez minutos: la página no se puede convertir en un cañón
  de correo.
- El editor lista los suscriptores (confirmados o pendientes) y
  elimina cualquiera de ellos.

Los enlaces en estos correos apuntan al [dominio
personalizado](#custom-domain) de la página cuando tiene uno
(`https://status.example.com/`). En caso contrario se construyen a
partir de la **URL pública** de la [política de
notificación](../alerting/notifications.md) (o
`DUMBMONIT_PUBLIC_URL`); sin una, de la dirección que usaste la última
vez que guardaste la página — nunca de la petición de un visitante.

## 4. Compartir el enlace {#4-share-the-link}

La página vive en `https://tu-servidor/s/<address>`. Todo bajo `/s/` y
`/api/public/` se sirve sin autenticación; el resto de DumbMonit se
mantiene detrás del inicio de sesión.

El panel **Share** del editor construye los fragmentos de abajo por
ti, con una vista previa.

### Badges

Cada página publicada sirve badges SVG — planos, con texto blanco
sobre color sólido, legibles tanto en el tema claro como oscuro de un
README de GitHub:

| Badge | URL |
| --- | --- |
| Estado de página | `/api/public/status/<address>/badge.svg` |
| Disponibilidad de página | `/api/public/status/<address>/uptime.svg?days=30` |
| Tiempo de respuesta de página | `/api/public/status/<address>/response.svg` |
| Estado de servicio | `/api/public/status/<address>/components/<service>/badge.svg` |
| Disponibilidad de servicio | `/api/public/status/<address>/components/<service>/uptime.svg?days=30` |
| Tiempo de respuesta de servicio | `/api/public/status/<address>/components/<service>/response.svg` |

`<service>` es la label del servicio en forma de URL: «Public API» se
convierte en `public-api` (una segunda «Public API» sería
`public-api-2`). También es la `key` de cada servicio en el documento
JSON. `days` es `1`, `7`, `30` (por defecto) o `90`, y nunca más que el
historial de la página: una página de 30 días rechaza un badge de 90
días. La disponibilidad y tiempo de respuesta de la página son la
media sobre sus servicios. Un servicio sin nada medido dice **no
data**, en gris.

En un README:

```markdown
[![Status](https://your-server/api/public/status/home-lab/badge.svg)](https://your-server/s/home-lab)
[![Uptime](https://your-server/api/public/status/home-lab/uptime.svg?days=30)](https://your-server/s/home-lab)
[![API](https://your-server/api/public/status/home-lab/components/public-api/uptime.svg)](https://your-server/s/home-lab)
```

Los badges se cachean durante un minuto (`Cache-Control: public,
max-age=60`), así que el proxy de imágenes de GitHub y tus visitantes
no le cuestan nada a la base de datos de series temporales.

### Embed

`/s/<address>/embed` es una vista compacta para un iframe: el estado
general y una línea por servicio con su estado y 30 días de historial,
y un enlace a la página completa en una pestaña nueva.

```html
<iframe src="https://your-server/s/home-lab/embed"
        title="Home lab status" width="100%" height="320" style="border:0"></iframe>
```

Añade `?theme=light` o `?theme=dark` para coincidir con la página
anfitriona, y `?history=0` para ocultar las barras. La página de
estado y su embed son las únicas direcciones de DumbMonit que una
página de terceros puede enmarcar; todo lo demás (las páginas de
confirmación y baja incluidas) responde con `frame-ancestors 'none'`
y `X-Frame-Options: DENY`.

### Dominio personalizado {#custom-domain}

Una página puede tener su propia dirección, como
`status.example.com`, de la forma en que lo hace Uptime Kuma. Escribe
el nombre en el campo **Public domain** del editor (la **i** junto a
él resume los pasos) y guarda. Cuando una petición alcanza DumbMonit
con ese nombre en su cabecera `Host`, DumbMonit muestra la página en
la raíz `/`, y nada más: en ese nombre la pantalla de inicio de
sesión, los ajustes, la API, los endpoints de los agentes, las
métricas y las demás páginas de estado todas responden `404 Not
found`, incluso para alguien con sesión iniciada, y no se lee ni
define ninguna cookie de sesión. Solo se sirven la página, su embed,
sus páginas de confirmación y baja, sus badges, feed y documento JSON,
y los propios archivos de la interfaz (`/_app/`, iconos).

El nombre se guarda en minúsculas, sin `https://` ni una barra final;
un puerto, una ruta o una dirección IP se rechazan. Dos páginas no
pueden compartir un nombre, y la dirección que usas para alcanzar
DumbMonit en sí (o su URL pública) no puede ser el dominio de una
página — guardarlo te dejaría fuera. Vacía el campo para eliminar el
dominio; la página se queda en `/s/<address>` de cualquier forma.

Para configurarlo:

1. **DNS.** Crea el registro para el nombre (o el nombre de host
   público de un Cloudflare Tunnel), apuntando a tu proxy inverso.
2. **Proxy.** Envía el nombre a la dirección de DumbMonit, por
   ejemplo `http://dumbmonit:8080`, **sin reescribir la ruta**: el
   proxy reenvía `/`, no `/s/<address>`.
3. **Cabecera Host.** Pasa la cabecera `Host` original: es cómo
   DumbMonit distingue el dominio de la página de su propia dirección.
   DumbMonit solo lee `Host`; `X-Forwarded-Host` cuenta solo desde un
   proxy listado en `DUMBMONIT_TRUSTED_PROXIES`.
4. **HTTPS** lo gestiona el proxy. DumbMonit enlaza a
   `https://<domain>/` en los correos de suscriptores, el feed RSS y
   el panel **Share** del editor.

Traefik (etiquetas Docker) — la cabecera `Host` se pasa por defecto:

```yaml
labels:
  - traefik.enable=true
  - traefik.http.routers.status.rule=Host(`status.example.com`)
  - traefik.http.routers.status.entrypoints=websecure
  - traefik.http.routers.status.tls.certresolver=letsencrypt
  - traefik.http.services.status.loadbalancer.server.port=8080
```

Nginx Proxy Manager — añade un **Proxy Host** con el dominio
`status.example.com`, esquema `http`, hostname de reenvío
`dumbmonit`, puerto `8080`, y un certificado SSL. Nada más: pasa
`Host` por defecto. Deja **Custom locations** vacío.

Cloudflare Tunnel — añade un **Public hostname** `status.example.com`
con el servicio `http://dumbmonit:8080` y sin ruta. El tunnel
conserva el `Host` del visitante; deja **HTTP Host Header** vacío en
los ajustes avanzados.

Caddy — `reverse_proxy` pasa `Host` y Caddy obtiene el certificado:

```caddyfile
status.example.com {
    reverse_proxy dumbmonit:8080
}
```

Nginx — `proxy_pass` envía el nombre del upstream a menos que se
indique otra cosa, así que define la cabecera:

```nginx
server {
    listen 443 ssl;
    server_name status.example.com;
    # ssl_certificate … ;

    location / {
        proxy_pass http://dumbmonit:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

**Solución de problemas.**

- *El dominio muestra la pantalla de inicio de sesión o el dashboard
  de DumbMonit*: la cabecera `Host` no llegó a DumbMonit — el proxy la
  reemplazó con el nombre del upstream (`dumbmonit:8080`). Añade
  `proxy_set_header Host $host;` (Nginx), elimina una **HTTP Host
  Header** personalizada (Cloudflare Tunnel), o elimina un middleware
  de reescritura de `Host` (Traefik). Nada privado se expone: el
  dashboard sigue pidiendo una contraseña.
- *Un simple `Not found` en el dominio*: el proxy reescribe la ruta
  (envía `/s/<address>` o añade un prefijo), o el dominio en el editor
  no coincide exactamente con el nombre en el navegador. Reenvía la
  ruta sin tocarla.
- *La página dice que no existe*: la página todavía es un borrador —
  publícala.
- *Ya no puedes alcanzar la interfaz de DumbMonit*: estás navegando a
  través del dominio de la página. Usa la propia dirección de
  DumbMonit, o vacía el **Public domain** de la página.

### Detrás de un proxy inverso {#behind-a-reverse-proxy}

Un [dominio personalizado](#custom-domain) es la forma más simple de
exponer solo la página de estado. Si prefieres mantener la ruta
`/s/<address>` en un nombre público, filtra las rutas en el proxy en
su lugar. Con Caddy, por ejemplo:

```caddyfile
status.example.com {
    @public path /s/* /api/public/* /_app/* /favicon.svg
    reverse_proxy @public dumbmonit:8080
    respond 404
}
```

Con nginx:

```nginx
location ~ ^/(s/|api/public/|_app/|favicon\.svg) {
    proxy_pass http://dumbmonit:8080;
    proxy_set_header Host $host;
    proxy_set_header X-Forwarded-Proto $scheme;
}
location / { return 404; }
```

`/_app/` lleva los scripts y estilos de la interfaz: la página los
necesita para renderizar. Reenvía `X-Forwarded-Proto` y
`X-Forwarded-Host` para que el feed RSS enlace a la dirección correcta,
y define la URL pública (ver arriba) para que los correos de
suscriptores también lo hagan.

### Feed y documento {#feed-and-document}

- `GET /api/public/status/<address>/rss` — un feed RSS de incidentes
  y ventanas de mantenimiento, un elemento por anuncio con su última
  actualización.
- `GET /api/public/status/<address>` — el documento JSON a partir del
  cual se construye la página, si quieres construir el tuyo: el
  título, descripción y aspecto de la página, el estado general, el
  `key`, `label`, `state`, disponibilidad (`uptime_24h`, `uptime_7d`,
  `uptime_30d`, `uptime_90d`), `latency_ms` e `history` (`date`,
  `uptime_pct`, `down_minutes`, `incidents` por día) de cada servicio,
  y los anuncios.

Cualquier sitio web puede leer ese documento desde el navegador (se
envía con `Access-Control-Allow-Origin: *` y sin cookies). `GET
/api/public/status/<address>/banner.js` es un script listo que lo
convierte en un banner: consulta [Banner para el sitio
web](../integrations/website-banner.md).

El documento se cachea durante 30 segundos en el servidor; los
badges, el feed y el embed todos leen ese mismo documento cacheado,
así que ninguno de ellos puede decir más que la página.
