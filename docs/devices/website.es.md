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
3. El primer control registra la referencia: dirección final, título, descripción, URL canónica, reglas de indexación. Un cambio se reporta solo una vez que dos controles seguidos lo han visto, por lo que las pruebas A/B y los cachés inestables permanecen silenciosos.
4. Para agregar muchos sitios a la vez, importa un archivo CSV desde la lista de dispositivos: cada sitio se archiva bajo `Sitios web/<plataforma>` (WordPress, Shopify…) a menos que des una carpeta.

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

## Cambios, confirmados dos veces

La dirección final y los metadatos se comparan con una referencia. Un valor
diferente visto una vez es solo **pendiente**; se convierte en un cambio cuando el siguiente control
lo ve nuevamente. Una página servida en dos versiones (una prueba A/B), un caché que
duda o una respuesta truncada permanecen silenciosos. Los valores se comparan ignorando
espaciado y mayúsculas; las direcciones ignoran el esquema, un `www.` inicial, una
barra inclinada final y, con **Ignorar cadenas de consulta** (activado de forma predeterminada), parámetros de URL
como `?utm_source=…`.

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
| Permitir destinos loopback y link-local | desactivado | Para un sitio servido por el host de DumbMonit mismo. |

## Detección de plataforma

La plataforma viene de, en orden: la etiqueta `<meta name="generator">`, encabezados
específicos de un host o motor (`x-shopify-*`, `x-wix-*`, `x-drupal-cache`,
`x-powered-by: Next.js`…), rastros en el marcado (`/wp-content/`,
`cdn.shopify.com`, `data-wf-site`, `__NEXT_DATA__`, `/_nuxt/`…) y finalmente el
host mismo (`*.myshopify.com`, `*.wixsite.com`…). Reconocidos: WordPress,
Shopify, Wix, Squarespace, Webflow, Ghost, Drupal, Joomla, PrestaShop, Magento,
Next.js, Nuxt, Gatsby, Hugo, Jekyll; cualquier otra cosa es **Otra**. La plataforma
es la etiqueta `platform` de `dumbmonit_website_info`.

## Importar una lista de sitios (CSV)

Desde la lista de dispositivos o la página de agregar dispositivo, **Importar sitios web (CSV)** toma un
archivo o texto pegado con una fila de encabezado y hasta 1000 filas:

| Columna | Requerido | Significado |
|---|---|---|
| `url` | sí | Dirección del sitio. |
| `name` | no | Nombre del dispositivo; el host de forma predeterminada. |
| `folder` | no | Carpeta del dispositivo; vacío: `Sitios web/<plataforma>`, detectado leyendo el sitio una vez. |
| `tags` | no | `key=value;key=value`. |
| `interval` | no | Intervalo de sondeo. |
| `expect_text` | no | Igual que la opción. |

```csv
url,name,folder,tags,interval,expect_text
https://example.com,Example,,client=acme,60s,Example Inc.
https://shop.example.org,Shop,Customers/Acme,,300s,Add to cart
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
| `dumbmonit_website_info{platform}` | Siempre 1; la etiqueta lleva la plataforma. |
| `dumbmonit_website_text_chars` | Caracteres de texto visible (respuestas 2xx). |
| `dumbmonit_website_blank` | 1 cuando ese texto está por debajo del umbral. |
| `dumbmonit_website_expect_text_missing` | 1 cuando falta el texto esperado (opción establecida). |
| `dumbmonit_website_noindex` / `dumbmonit_website_nofollow` | Directivas de indexación. |
| `dumbmonit_website_h1_count` / `dumbmonit_website_hreflang_count` | Encabezados `<h1>`, alternativas `hreflang`. |
| `dumbmonit_website_title_changed`, `…_description_changed`, `…_canonical_changed`, `…_final_url_changed` | 1 durante 24 horas después de un cambio confirmado. |
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
| Sitio web lento | Más de 5 segundos, durante 10 minutos | warning |

El control se ejecuta en el servidor DumbMonit mismo: no puede pasar a través de un agente relé.
