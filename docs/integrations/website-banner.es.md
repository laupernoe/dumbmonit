# Banner para el sitio web {#website-banner}

Muestra un banner en tu propio sitio web cuando algún servicio está caído
(«2 services are down»), alimentado por una [página de estado](../using/status-pages.md)
publicada. Cuando todo va bien el banner permanece oculto, a menos que
pidas que se muestre siempre («All systems operational»).

Tres formas de añadirlo, de la más simple a la más controlada. Los archivos
listos para copiar están en la carpeta `integrations/` del repositorio.

## Fragmento HTML {#html-snippet}

Pega esto antes de `</body>`, con la dirección de tu servidor y la dirección
de la página:

```html
<script async
  src="https://monit.example.com/api/public/status/my-page/banner.js"
  data-position="top" data-show="issues" data-lang="en"></script>
```

| Atributo | Valores | Por defecto |
|---|---|---|
| `data-position` | `top`, `bottom` | `top` |
| `data-show` | `issues` (solo cuando algo falla), `always` | `issues` |
| `data-lang` | `en`, `fr`, `de`, `es`, `it`, `pt`, `pt-BR`, `ru`, `zh-Hans` | idioma de la página, si no `en` |

El script no tiene dependencias, se dibuja dentro de un Shadow DOM para que
tus estilos no lo afecten, sigue la preferencia de tema claro u oscuro del
visitante y puede descartarse (hasta que se cierre la pestaña del
navegador). Lee el documento JSON público de la página, que el servidor
permite consultar a cualquier sitio web (`Access-Control-Allow-Origin: *`,
sin cookies). Solo se abre así el documento público de la página de estado;
el resto de la API mantiene sus reglas.

## WordPress {#wordpress}

Copia `integrations/wordpress/dumbmonit-status-banner` en
`wp-content/plugins/`, actívalo y luego abre **Settings > DumbMonit Banner**
e introduce la URL de la página de estado
(`https://monit.example.com/api/public/status/my-page`), la posición y el
modo de visualización. El plugin obtiene el JSON desde el servidor, lo
cachea durante un minuto y escapa todo lo que imprime. WordPress rechaza
las peticiones a direcciones privadas, así que el servidor de DumbMonit debe
ser accesible en una dirección pública.

## Inclusión PHP {#php-include}

Para cualquier sitio PHP, sin framework:

```php
require __DIR__ . '/banner.php';
dumbmonit_banner('https://monit.example.com/api/public/status/my-page',
    ['position' => 'top', 'show' => 'issues', 'ttl' => 60]);
```

El helper solo acepta URLs `http` y `https`, usa un tiempo de espera de 3
segundos, no sigue redirecciones, mantiene una caché de archivo en el
directorio temporal y no imprime nada si no se puede alcanzar el servidor.
Consulta `integrations/php/example.php`.
