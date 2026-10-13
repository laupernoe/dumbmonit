# Salud y SEO del sitio web

Vigila sitios web en masa, un dispositivo por sitio: sé informado cuando un sitio **se rompe**
(error, inalcanzable, certificado inválido, bucle de redirección), cuando su página **se queda en blanco**, cuando **cae de los motores de búsqueda** (`noindex`, `robots.txt`
bloqueando todo) y cuando los metadatos que Google muestra **cambian** (título,
descripción, URL canónica, dirección final). Cada sitio también se ordena por la
plataforma que ejecuta (WordPress, Shopify, Wix, Next.js…).

Para el contenido de una página en lugar de su salud, ver
[Cambios del sitio web](webchange.md).

## Configuralo

1. En la dirección, pega la página a verificar, generalmente la página de inicio, con o sin https://. Los redireccionamientos se siguen: http a https y example.com a www.example.com son normales, solo una redirección en bucle o un movimiento a otra dirección se reporta.
2. Opcionalmente completa "Texto esperado" con una frase que la página siempre debe mostrar, y ajusta "Página en blanco debajo" si la página es corta a propósito.
3. El primer control registra la referencia: dirección final, título, descripción, URL canónica, reglas de indexación. Un cambio se reporta solo una vez que **2 controles seguidos** (ajustable) lo han visto, por lo que las pruebas A/B y los cachés inestables permanecen silenciosos.
4. Para agregar muchos sitios a la vez, importa un archivo CSV desde la lista de dispositivos: cada sitio se archiva bajo `Websites/<categoría>` (E-commerce, Blog… o la categoría que des) o, cuando la página no lo dice, bajo `Websites/<plataforma>` (WordPress, Shopify…), a menos que des una carpeta.

!!! warning
    Solo se lee lo que el servidor envía: una página dibujada completamente por JavaScript se ve en blanco aquí. Baja "Página en blanco debajo" a 0 para tal sitio.

## Qué hace cada control

En cada intervalo de sondeo (60 s de forma predeterminada), la página se lee una vez:

- **Los redireccionamientos se siguen** uno por uno, hasta **Max redireccionamientos** (10 por
  defecto), y la cadena se registra: cada dirección y su estado. Ir de
  `http://` a `https://`, o de `example.com` a `www.example.com`, es lo que
  la mayoría de sitios hacen y nunca es un error. Un redireccionamiento que vuelve a una dirección
  ya vista es un **bucle**, y más redireccionamientos de los permitidos también es un error:
  ambos hacen que el control falle con la cadena en el mensaje, como un sitio que no
  responde.
- **La respuesta final**: su estado HTTP, el tiempo total (redireccionamientos incluidos)
  y el certificado TLS, verificado como lo hace un navegador. Un certificado
  rechazado hace que el control falle.
- **El texto visible** de la página, como en [Cambios del sitio web](webchange.md):
  los scripts, estilos y bloques ocultos se omiten. Por debajo de **Página en blanco debajo**
  caracteres (20 de forma predeterminada, por lo que las páginas legítimas cortas pasan) la página cuenta como en blanco. Con
  **Texto esperado**, esa frase debe aparecer en él (mayúsculas y espacios ignorados).
- **Los metadatos SEO** de la página final: `<title>`, descripción meta,
  URL canónica, etiquetas meta robots y encabezados `X-Robots-Tag` (`noindex`,
  `nofollow`, `none`), `og:title`, `og:description`, `<html lang>`, el número
  de `<h1>` y de alternativas `hreflang`.
- **Una vez por hora**, `robots.txt` del sitio final: ¿prohíbe a Googlebot
  (o `*`) rastrear `/`? Y el mapa del sitio que anuncia, o `/sitemap.xml`: ¿
  responde y se lee como un mapa del sitio?

El User-Agent es `DumbMonit/<version>` a menos que **User agent** diga lo contrario.

## Páginas casi vacías {#nearly-empty-pages}

Además del umbral absoluto (**Página en blanco debajo**), DumbMonit mantiene una referencia, que
se mueve lentamente, del tamaño del texto visible de cada página. Cuando el texto cae más de
**Página casi vacía: caída del texto** por ciento por debajo de ella (80 de forma predeterminada,
es decir, de 1000 caracteres a menos de 200) y se queda así durante **Confirmar un cambio tras**
controles seguidos, se dispara la alerta «Página de sitio web casi vacía». Detecta un tema que
perdió su contenido, un catálogo que ya no carga o un caché que sirve un cascarón, mientras la
página sigue respondiendo 200. Hace falta una referencia de al menos 200 caracteres; el
crecimiento y la reducción normales no hacen ruido, y una caída que dura más de una semana pasa
a ser la nueva referencia (el sitio se rehízo más corto). 0 lo desactiva.

## Cambios, confirmados dos veces

La dirección final y los metadatos se comparan con una referencia. Un valor
diferente visto una vez es solo **pendiente**; se convierte en un cambio cuando **Confirmar un cambio tras** controles seguidos
(2 de forma predeterminada, de 1 a 10) han leído el mismo valor nuevo. Un valor que vuelve atrás,
o un tercer valor, reinicia la cuenta. Una página servida en dos versiones (una prueba A/B), un caché que
duda o una respuesta truncada permanecen silenciosos. Los valores se comparan ignorando
espaciado y mayúsculas; las direcciones ignoran el esquema, un `www.` inicial, una
barra inclinada final y, con **Ignorar cadenas de consulta** (activado de forma predeterminada), parámetros de URL
como `?utm_source=…`.

Cada tipo de control puede desactivarse por separado, en las opciones «Metadatos vigilados»:
dirección final, título, descripción, URL canónica, directivas robots, Open Graph, idioma, número
de `<h1>`, `hreflang` y etiqueta de verificación de Google. Un campo desactivado no se compara,
no se registra y no genera alertas; al reactivarlo, parte de una referencia nueva. Todo está
activado de forma predeterminada. Un campo que desaparece de la página (valor vacío) es un cambio
como cualquier otro.

Un cambio confirmado se mantiene en el historial del dispositivo (campo, antes, después,
fecha), y su indicador (`dumbmonit_website_title_changed`…) permanece en 1 durante 24
horas. El primer control solo registra la referencia. Cambiar la dirección o
**Ignorar cadenas de consulta** inicia una nueva referencia en lugar de reportar un cambio.

## Opciones

| Opción | Predeterminado | Significado |
|---|---|---|
| Texto esperado | vacío | Una frase que la página debe mostrar. |
| Página en blanco debajo (caracteres) | 20 | Texto visible más corto que esto es una página en blanco; 0 desactiva el control. |
| Max redireccionamientos | 10 | Redireccionamientos seguidos como máximo (0 a 30). |
| User agent | `DumbMonit/<version>` | Enviado en lugar del propio de DumbMonit. |
| Ignorar cadenas de consulta | activado | Los parámetros de URL no cuentan cuando se comparan direcciones. |
| Categoría | vacío | Tu propia etiqueta del sitio, mostrada con él y en sus métricas (la fija la importación CSV). Vacío: adivinada, ver [Categorías](#categories). |
| Confirmar un cambio tras (controles) | 2 | Lecturas seguidas de un valor nuevo antes de reportarlo, de 1 a 10. |
| Página casi vacía: caída del texto (%) | 80 | Alerta cuando el texto visible cae tanto por debajo de su tamaño habitual, de 0 a 99; 0 lo desactiva. |
| Vigilar la dirección final, el título, la descripción, la URL canónica, las directivas robots, Open Graph, el idioma, el número de h1, hreflang, la etiqueta de verificación de Google | todo activado | Cada control de metadatos puede desactivarse por separado. |
| Permitir destinos loopback y link-local | desactivado | Para un sitio servido por el host de DumbMonit mismo. |

## Ritmo con cientos de sitios {#pacing-hundreds-of-sites}

Los controles nunca salen de golpe, para que un cortafuegos o una CDN delante de
los sitios no confunda DumbMonit con un ataque:

- **Repartidos en el intervalo**: cada sitio arranca en su propio desfase fijo
  dentro de su intervalo de sondeo, así que los sitios añadidos juntos (una
  importación CSV) no disparan en el mismo segundo.
- **Límite global**: como máximo `DUMBMONIT_WEBSITE_MAX_CONCURRENT` controles de
  sitios en curso a la vez (10 por defecto), además de
  `DUMBMONIT_MAX_CONCURRENT_PROBES`.
- **Por dominio**: como máximo `DUMBMONIT_WEBSITE_MAX_PER_HOST` controles en
  curso hacia un mismo dominio (1 por defecto), y al menos
  `DUMBMONIT_WEBSITE_HOST_SPACING_SECS` segundos entre dos de ellos (2 por
  defecto, o el valor **Esperar antes de la siguiente petición a este dominio**
  del sitio). El dominio es el nombre registrable: `www.example.com`,
  `shop.example.com` y `example.com` comparten uno, `example.co.uk` es uno;
  los subdominios de clientes de las plataformas de alojamiento
  (`*.myshopify.com`, `*.wordpress.com`, `*.github.io`, `*.netlify.app`…) cuentan
  cada uno como el suyo.

Un sitio cuyo control anterior sigue esperando su turno se salta una ronda en
lugar de ponerse dos veces en cola. Muchos sitios de un mismo dominio con un
intervalo corto se controlan, por tanto, menos a menudo de lo pedido: sube el
intervalo, o los límites por dominio, si la lista de dispositivos los muestra
con retraso. Consulta [Configuración](../reference/configuration.md#server).

## Detección de plataforma

La plataforma viene de, en orden: la etiqueta `<meta name="generator">`, encabezados
específicos de un host o motor (`x-shopify-*`, `x-wix-*`, `x-drupal-cache`,
`x-powered-by: Next.js`…), rastros en el marcado (`/wp-content/`,
`cdn.shopify.com`, `data-wf-site`, `__NEXT_DATA__`, `/_nuxt/`…) y finalmente el
host mismo (`*.myshopify.com`, `*.wixsite.com`…). Reconocidos: WordPress,
Shopify, Wix, Squarespace, Webflow, Ghost, Drupal, Joomla, PrestaShop, Magento,
Next.js, Nuxt, Gatsby, Hugo, Jekyll; cualquier otra cosa es **Otra**. La plataforma
es la etiqueta `platform` de `dumbmonit_website_info`.

## Categorías {#categories}

Cada sitio recibe una categoría, mostrada en su página, en la etiqueta `category` de
`dumbmonit_website_info` y como carpeta al importar. Puedes fijarla tú (opción **Categoría** o
columna CSV); si no, se adivina solo a partir de señales claras:

- **E-commerce**: Shopify, PrestaShop o Magento, marcado de WooCommerce o «añadir al carrito»,
  `og:type` en `product`.
- **Blog**: Ghost, `og:type` en `article`, un host `blog.` o una dirección `*.blogspot.com`,
  `*.substack.com`, `*.wordpress.com`.
- **Aplicación web**: Next.js o Nuxt, o una página que es solo un contenedor `#root` / `#app` vacío.
- **Sitio escaparate**: Wix, Squarespace o Webflow.

Una plataforma generalista (WordPress, Drupal, Hugo…) sin otra señal no se adivina: el sitio se
queda entonces bajo su plataforma (`Websites/WordPress`), y los de plataforma desconocida bajo
`Websites/Other`. Ordénalos a mano con las columnas `folder` o `category`, o muévelos en la lista
de dispositivos.

## Importar una lista de sitios (CSV)

Desde la lista de dispositivos o la página de agregar dispositivo, **Importar sitios web (CSV)** toma un
archivo o texto pegado con una fila de encabezado y hasta 1000 filas:

| Columna | Requerido | Significado |
|---|---|---|
| `url` | sí | Dirección del sitio. |
| `name` | no | Nombre del dispositivo; el host de forma predeterminada. |
| `folder` | no | Carpeta del dispositivo; vacío: `Websites/<categoría>` si hay categoría, si no `Websites/<plataforma>`, detectado leyendo el sitio una vez. |
| `category` | no | Texto libre (60 caracteres, sin `/`), guardado como la opción **Categoría**. Vacío: adivinada desde la página, ver [Categorías](#categories). |
| `tags` | no | `key=value;key=value`. |
| `interval` | no | Intervalo de sondeo. |
| `expect_text` | no | Igual que la opción. |

```csv
url,name,folder,tags,interval,expect_text,category
https://example.com,Example,,client=acme,60s,Example Inc.,
https://shop.example.org,Shop,Customers/Acme,,300s,Add to cart,
https://blog.example.net,,,,,,Clients
```

Una vista previa muestra cada fila y sus errores antes de que se cree nada; un sitio
ya supervisado (misma dirección final) se omite.

## Métricas

| Métrica | Significado |
|---|---|
| `dumbmonit_website_up` | 1 cuando la respuesta final es un éxito (2xx o 3xx), 0 en caso contrario. |
| `dumbmonit_website_status_code` | Estado HTTP de la respuesta final. |
| `dumbmonit_website_response_seconds` | Tiempo total, redireccionamientos incluidos. |
| `dumbmonit_website_redirect_hops` | Redireccionamientos seguidos. |
| `dumbmonit_website_info{platform,category}` | Siempre 1; las etiquetas llevan la plataforma y, si se conoce, la categoría. |
| `dumbmonit_website_text_chars` | Caracteres de texto visible (respuestas 2xx). |
| `dumbmonit_website_blank` | 1 cuando ese texto está por debajo del umbral. |
| `dumbmonit_website_expect_text_missing` | 1 cuando falta el texto esperado (opción establecida). |
| `dumbmonit_website_noindex` / `dumbmonit_website_nofollow` | Directivas de indexación. |
| `dumbmonit_website_h1_count` / `dumbmonit_website_hreflang_count` | Encabezados `<h1>`, alternativas `hreflang`. |
| `dumbmonit_website_title_changed`, `…_description_changed`, `…_canonical_changed`, `…_final_url_changed` | 1 durante 24 horas después de un cambio confirmado. |
| `dumbmonit_website_text_dropped` | 1 cuando el texto visible está muy por debajo de su tamaño habitual (opción activada). |
| `dumbmonit_website_robots_changed`, `…_open_graph_changed`, `…_lang_changed`, `…_h1_changed`, `…_hreflang_changed`, `…_google_verification_changed` | 1 durante 24 horas después de un cambio confirmado de esos metadatos. |
| `dumbmonit_website_robots_blocks_all` | 1 cuando `robots.txt` prohíbe todo a Googlebot. |
| `dumbmonit_website_sitemap_ok` | 1 cuando un mapa del sitio responde y se lee. |

## Alertas integradas

| Alerta | Cuándo | Severidad |
|---|---|---|
| Dispositivo inalcanzable | Sin respuesta, certificado rechazado, bucle de redirección o demasiados redireccionamientos | critical |
| Sitio web abajo | La respuesta final es un error 4xx o 5xx, durante 3 minutos | critical |
| Página de sitio web en blanco | Texto visible por debajo del umbral, durante 3 minutos | critical |
| Texto esperado de sitio web faltante | El texto esperado falta, durante 3 minutos | warning |
| Sitio web no indexable | `noindex` en la página, durante 10 minutos | critical |
| Sitio web bloqueado por robots.txt | `robots.txt` prohíbe todo, durante 10 minutos | critical |
| URL canónica del sitio web cambió | Cambio confirmado | warning |
| Dirección del sitio web cambió | El host o ruta final cambió, confirmado | warning |
| Título o descripción del sitio web cambió | Cambio confirmado | info |
| Página de sitio web casi vacía | El texto visible cayó muy por debajo de su tamaño habitual, durante 3 minutos | warning |
| Verificación de Google del sitio web cambió | La etiqueta `google-site-verification` cambió o desapareció, confirmado | warning |
| Marcado SEO del sitio web cambió | Directivas robots, Open Graph, idioma, número de h1 o hreflang cambiaron, confirmado | info |
| Sitio web lento | Más de 5 segundos, durante 10 minutos | warning |

El control se ejecuta en el servidor DumbMonit mismo: no puede pasar a través de un agente relé.
