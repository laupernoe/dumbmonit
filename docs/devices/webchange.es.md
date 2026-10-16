# Cambios en sitios web {#website-changes}

Vigila el *contenido* de una página web, o de un sitio entero, y entérate
cuando cambie: una página de precios o condiciones, un sitio de
documentación, un aviso de mantenimiento de un proveedor, una página de
anuncios públicos, o tu propio sitio tras un despliegue. Cada cambio se
guarda con el texto antes y después, un diff de líneas y, cuando hay un
navegador disponible, una captura de cada versión.

Lo que se compara es el **texto visible** de cada página, no su HTML:
los scripts, estilos, comentarios, bloques `noscript`, `svg` y
`template` se descartan, los espacios se colapsan y cada bloque (párrafo,
encabezado, elemento de lista, fila de tabla) se convierte en una línea.
Una nueva compilación de la misma página, un atributo reordenado o un
nombre de asset con cache-busting nuevo no cambian nada; una frase
reformulada sí.

### Qué parte de la página {#what-part-of-the-page}

La **navegación, la cabecera, el pie y las barras laterales** del sitio se
excluyen: son iguales en todas las páginas, así que una nueva entrada de menú
marcaría todas las páginas como cambiadas en una sola comprobación. Siguen los
roles que da HTML: todo `nav`, un `header` o `footer` que no esté dentro de un
`article`, `aside`, `main`, `nav` o `section` (la cabecera de un artículo se
conserva), un `aside` fuera de `article` y `main`, y cualquier elemento con el
rol `navigation`, `banner`, `contentinfo` o `search`. Sus enlaces se siguen
igualmente para encontrar las páginas de un sitio. **Compare menus, header and
footer** desactiva esto.

**Watched area** limita la comparación a algunos elementos, por ejemplo `main` o
`#content`; una página donde no se encuentra ninguno se lee entera en lugar de
como vacía. **Excluded areas** excluye más elementos: un banner de cookies, un
cuadro de «últimas publicaciones», un hueco publicitario. Ambos admiten
selectores simples, separados por comas: una etiqueta (`main`), un id
(`#content`), clases (`.cookie-banner`), atributos (`[role=alert]`,
`[data-ad]`), o varios a la vez (`div.notice[role=alert]`). Los descendientes
(`main p`) y otros combinadores no se admiten.

## Página o sitio {#page-or-site}

**Page** (por defecto) lee la dirección del monitor y nada más.
También puede ser un documento de texto plano o JSON, comparado línea
por línea.

**Site** parte de la dirección y sigue cada página bajo la misma carpeta
(`https://example.com/docs/intro.html` cubre todo bajo `/docs/`; **Path
prefix** elige otra carpeta). Las páginas se encuentran:

1. en el sitemap del sitio: las que anuncia `robots.txt` (líneas
   `Sitemap:`), o `/sitemap.xml`, índices de sitemap incluidos (un
   nivel);
2. si no, o cuando el sitemap no lista nada bajo la carpeta, siguiendo
   los enlaces de cada página, en anchura primero.

Solo se siguen páginas del mismo origen (esquema, host y puerto). Cuando la
página de inicio redirige a otro host del mismo sitio (`example.com` a
`www.example.com`, `http` a `https`, o un subdominio como
`docs.example.com`), se considera que el sitio vive allí, y los enlaces al primer
host se leen en el segundo. Cuando redirige a otro sitio distinto (una página de
inicio de sesión como `login.microsoftonline.com`, un dominio de terceros), ese
sitio no se rastrea: solo se lee la página de inicio y ninguna página se reporta
como eliminada. Los enlaces a imágenes, hojas de estilo, scripts, PDFs,
archivos comprimidos y otros archivos se omiten por su extensión, y cualquier
respuesta que no sea HTML se ignora. Los fragmentos (`#section`), los
parámetros de seguimiento (`utm_…`, `fbclid`, `gclid` y similares) y las
queries vacías se eliminan, así que una página es una dirección; **Ignore query
strings** descarta toda la parte `?…`.
`robots.txt` se respeta para las páginas encontradas de esta forma: el
grupo que nombra `DumbMonit`, o si no el grupo `*`, con `Allow`,
`Disallow`, `*` y `$`.

El rastreo es educado: como máximo dos peticiones a la vez, medio
segundo entre dos lotes, el `User-Agent` `DumbMonit/<version>`, un
tiempo de espera de 15 segundos por petición y 5 MB leídos como máximo
por página. Se detiene en **Max pages** (25 por defecto, 200 como
máximo) o tras diez minutos. Cuando **Max pages** lo detuvo, la página del
monitor indica cuántas páginas se encontraron, para que la parte no
vigilada del sitio nunca sea una sorpresa. **User agent** reemplaza
`DumbMonit/<version>` para un sitio que rechaza a los robots.

## Cómo funciona una comprobación {#how-a-check-works}

El monitor se sondea como cualquier otro (cada minuto por defecto),
pero las páginas solo se leen cada **Check every** minutos (60 por
defecto, de 5 a 10080). Entre medias, el sondeo solo reporta el estado
de la última comprobación: el sitio no se consulta cada minuto. Una
comprobación se ejecuta en segundo plano; dos comprobaciones del mismo
monitor nunca se superponen, y **Check now** en la página del
monitor inicia una de inmediato.

La primera comprobación solo registra la **referencia**: se lee y
guarda cada página, nada se reporta. A partir de entonces, para cada
página:

| Situación | Qué se guarda |
|---|---|
| Mismo texto que la última vez (tras **Ignore lines**) | Nada nuevo; la página se marca comprobada. |
| Texto diferente | Una nueva captura y un evento **changed**, con el número de líneas añadidas y eliminadas. |
| Una página nunca vista antes, o vista de nuevo tras haber desaparecido | Una captura y un evento **new page**. |
| La página responde 404 o 410 | Un evento **removed page**, la última captura como «before». |
| Alcance de sitio: un rastreo completo ya no alcanza una página conocida | También un evento **removed page** («No longer linked»). Un rastreo detenido por **Max pages** o por su límite de tiempo nunca concluye que una página desapareció. |
| Cualquier otro error en una página (tiempo de espera, 403, 500 en una subpágina) | El error se muestra en la página; sin evento. |

Cambiar la dirección, el alcance, el prefijo de ruta, **Max pages**, las áreas
o **Ignore lines** hace que la siguiente comprobación sea una nueva referencia
en lugar de reportar cada página como cambiada (o, con un **Max pages** mayor,
cada página que añade como nueva). Una actualización de DumbMonit que lea las
páginas de otra forma hace lo mismo una vez.

Cuando una comprobación encuentra el **mismo cambio en varias páginas** (las
mismas líneas eliminadas y añadidas, como un banner), la página del monitor
lo muestra una sola vez, con las páginas en que se produjo.

### Ignore lines {#ignore-lines}

Las líneas que cambian en cada visita — un reloj, un contador de
visitantes, «actualizado hace 5 minutos», una cita aleatoria — se
reportarían en cada comprobación. **Ignore lines** contiene expresiones
regulares (sintaxis `regex` de Rust), una por línea o unidas con `|`;
cualquier línea de texto que coincida con una de ellas se excluye antes
de comparar. Por ejemplo:

```
Updated \d+ minutes ago|Visitors: \d+|© \d{4}
```

### Página de inicio inaccesible {#unreachable-start-page}

Cuando la propia página de inicio no se puede leer (sin respuesta,
conexión rechazada, tiempo de espera, HTTP 5xx), el monitor es
**inaccesible**: el sondeo falla, y la regla «Monitor unreachable» se
activa como para cualquier monitor. La página de inicio entonces se
reintenta en cada sondeo, así que su regreso se ve dentro de un minuto.
Una página de inicio que rechaza el acceso (401, 403) se reporta como
un problema de configuración, no como una caída. Los errores en otras
páginas del sitio solo se muestran en esas páginas.

Las direcciones que solo el propio host de DumbMonit puede alcanzar
(loopback, link-local, el servicio de metadatos de la nube) se rechazan
a menos que esté activado **Allow loopback and link-local targets**; las
direcciones de LAN privada siempre están permitidas.

## Alertas {#alerts}

La regla integrada **Website changed** (`webchange_detected`, Info) se
activa tras una comprobación que encontró al menos un cambio (página
cambiada, nueva o eliminada), y la notificación nombra las páginas:
«/pricing, /contact y 2 más». Mientras está activa, cada comprobación posterior
que encuentra cambios se notifica de nuevo, con sus propias páginas, incluso
sobre las mismas páginas; la alerta se resuelve en la primera comprobación que
no encuentra nada nuevo. Sigue siendo una sola alerta por monitor todo el
tiempo, así que reconocerla o posponerla desde la alerta cubre también las
comprobaciones siguientes. Lee
`dumbmonit_webchange_last_check_changes`; asígnala a un canal como
cualquier otra regla, o eleva su severidad, en **Alerts → Rules**.
Consulta [Reglas integradas](../alerting/rules.md#website-changes).

| Métrica | Qué |
|---|---|
| `dumbmonit_webchange_pages` | Páginas seguidas (páginas eliminadas excluidas). |
| `dumbmonit_webchange_pages_failed` | Páginas seguidas cuya última lectura falló. |
| `dumbmonit_webchange_last_check_changes` | Cambios encontrados por la última comprobación. |

Las métricas aparecen una vez hecha la primera comprobación.

## Capturas de pantalla {#screenshots}

La imagen de DumbMonit no tiene navegador: las capturas vienen de un
Chromium sin interfaz opcional, controlado a través del Chrome DevTools
Protocol. Añade uno como servicio `browser` junto a DumbMonit en
`docker-compose.yml` y apunta el servidor hacia él:

```yaml
services:
  dumbmonit:
    environment:
      DUMBMONIT_BROWSER_URL: http://browser:9222
  browser:
    image: chromedp/headless-shell:stable
    shm_size: 256m
    restart: unless-stopped
```

Después ejecuta `docker compose up -d`. Nada publica el puerto del navegador:
solo el servidor de DumbMonit habla con él, a través de la red de Compose.

`DUMBMONIT_BROWSER_URL` apunta DumbMonit hacia el navegador (cualquier
Chromium iniciado con `--remote-debugging-port`, por ejemplo
`chromedp/headless-shell`; el servidor lee `/json/version` y habla con
la dirección a la que resuelve el nombre de host).

Una captura se toma solo cuando una página obtiene una nueva captura (la
referencia, y cada cambio), nunca en una comprobación sin cambios: una
ventana de 1366 × 900, la página completa hasta 5000 píxeles, como JPEG.
La opción **Screenshots** las desactiva para un monitor. Sin
navegador, o cuando no responde, la comparación de texto funciona igual
y las capturas simplemente no tienen imagen; la página del monitor
lo indica.

El navegador carga la página como lo haría un visitante, con sus
scripts: lo que muestra puede diferir del texto que comparó DumbMonit,
que es el HTML que envió el servidor.

## Almacenamiento {#storage}

El texto de cada captura se guarda en la base de datos (1 MB como
máximo por página); las capturas de pantalla son archivos bajo
`/data/webchange/<id del monitor>/`. Por monitor, se conservan
los últimos 500 cambios; una captura se conserva mientras sea la más
reciente de una página o un cambio conservado la referencia, y todo lo
demás, archivos incluidos, se elimina tras cada comprobación. Eliminar
el monitor lo elimina todo.

## Vigilar un sitio web por cambios {#watch-a-website-for-changes}

Los pasos de abajo son los que muestra el aviso junto al formulario.

1. En la dirección, pega la página a vigilar, con http:// o https://.
   Para vigilar un sitio entero o una de sus secciones, pega su página
   de inicio y define «Scope» como «site»: se sigue cada página bajo la
   misma carpeta, hasta «Max pages».
2. Las líneas que cambian en cada visita (un reloj, un contador de
   visitantes, «actualizado hace 5 minutos») se reportarían cada vez:
   lístalas en «Ignore lines» como expresiones regulares.
3. La primera comprobación solo registra la referencia, nada se
   reporta. A partir de entonces, cada comprobación compara el texto
   visible página por página y guarda una copia antes/después de cada
   cambio, mostrada en la página del monitor; la alerta «Website
   changed» te avisa.
4. Para una captura de cada versión, añade un navegador sin interfaz a tu
   archivo Compose; las líneas exactas están en la documentación (Advanced
   options, «Screenshots of website changes»).
   Consulta [Capturas de pantalla de cambios en sitios web](../install/docker.md#website-screenshots).

!!! warning
    Solo se compara el texto que envía el servidor: una página dibujada
    enteramente por JavaScript muestra poco o nada a la comprobación.
    Las páginas detrás de un inicio de sesión tampoco son accesibles.

Credenciales: ninguna. Las comprobaciones siempre se ejecutan en el
servidor DumbMonit: una vigilancia de sitio web no puede pasar por un
agente relay.

## Opciones {#options}

| Opción | Por defecto | Significado |
|---|---|---|
| Scope (`scope`) | `page` | `page`: solo la dirección. `site`: cada página bajo la misma carpeta. |
| Max pages (`max_pages`) | `25` | Alcance de sitio: páginas leídas como máximo por comprobación, de 1 a 200. |
| Path prefix (`path_prefix`) | carpeta de la dirección | Alcance de sitio: solo páginas cuya ruta empieza con esto (`/docs/`). |
| Ignore lines (`ignore`) | — | Expresiones regulares; las líneas de texto coincidentes no se comparan. |
| Watched area (`content_selector`) | — | Selectores simples; solo se compara el texto dentro de ellos. |
| Excluded areas (`exclude_selector`) | — | Selectores simples; su texto no se compara. |
| Compare menus, header and footer (`keep_page_chrome`) | `false` | Comparar también la navegación, la cabecera, el pie y las barras laterales del sitio. |
| Ignore query strings (`ignore_query`) | `false` | Alcance de sitio: las direcciones que solo difieren en su parte `?…` son una sola página. |
| User agent (`user_agent`) | — | Se envía en lugar de `DumbMonit/<version>`. |
| Screenshots (`screenshots`) | `true` | Guardar una captura de cada versión nueva, cuando hay un navegador configurado. |
| Check every (`check_interval_minutes`) | `60` | Minutos entre dos comprobaciones, de 5 a 10080. |
| Accept an unverifiable certificate (`insecure_tls`) | `false` | Para un sitio propio detrás de un certificado autofirmado. |
| Allow loopback and link-local targets (`allow_private_targets`) | `false` | Alcanzar un sitio servido por el propio host de DumbMonit. |
