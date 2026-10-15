# AdGuard Home

El propio filtro DNS: si la protección está activada, si su servidor DNS
funciona, qué respondió y bloqueó, si sus listas de filtro aún se
actualizan, si sus servidores upstream responden, y si hay una versión
más reciente disponible.

Un filtro DNS roto no parece roto. Las páginas siguen abriéndose, con sus
anuncios. Los fallos habituales son silenciosos: protección pausada
«durante una hora» desde la app del teléfono y nunca reactivada, listas
de filtro que dejaron de descargarse hace semanas, un servidor
DNS-over-HTTPS upstream que ya no responde así que cada búsqueda espera
al siguiente.

DumbMonit lee la API REST de AdGuard Home (`/control`) con autenticación
básica HTTP. Probado contra AdGuard Home v0.107.79 (y la comprobación de
versión v0.107.78).

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_adguard_`. Las cifras de
consultas y bloqueos son **sumas sobre la ventana de estadísticas de
AdGuard Home** (24 horas a menos que se cambie en Settings → General
settings), no contadores: suben y bajan al desplazarse la ventana, y se
guardan tal cual.

| Métrica | Qué | Etiquetas |
|---|---|---|
| `running` | 1 cuando el servidor DNS está en ejecución | |
| `protection_enabled` | 1 cuando la protección está activada | |
| `protection_paused_seconds` | tiempo restante antes de que una protección pausada vuelva a activarse, 0 sin pausa | |
| `version_info` | valor 1 | `version` |
| `dns_queries` | consultas en la ventana de estadísticas | |
| `blocked_filtering` | consultas bloqueadas por las listas de filtro y reglas personalizadas | |
| `replaced_safebrowsing`, `replaced_parental`, `replaced_safesearch` | respuestas sustituidas por Browsing security, Parental control, Safe search | |
| `blocked_percent` | proporción de consultas bloqueadas (filtros, Browsing security, Parental control); ausente sin ninguna consulta | |
| `avg_processing_seconds` | tiempo promedio de respuesta | |
| `stats_window_seconds`, `stats_enabled` | la ventana de estadísticas y si se conservan estadísticas (0.107.30 y posteriores) | |
| `upstream_responses`, `upstream_avg_seconds` | respuestas y tiempo de respuesta promedio por upstream en la ventana, tal como lo muestra el Top upstreams del panel (como máximo 8) | `upstream` |
| `upstream_up` | 1 cuando la propia prueba de AdGuard Home de ese upstream tiene éxito; 0 cuando falla o no da ninguna respuesta en 4 segundos (como máximo 8). La etiqueta es el upstream tal como AdGuard Home lo escribe en sus estadísticas (`https://dns10.quad9.net:443/dns-query`, `192.0.2.1:53`); para un upstream que permaneció en silencio, la línea tal como está configurada. | `upstream` |
| `filtering_enabled` | 1 cuando el filtrado está activado en Settings → General settings | |
| `filters`, `filters_enabled` | listas de bloqueo configuradas, y activadas | |
| `filter_rules` | reglas cargadas desde las listas activadas | |
| `filter_rules_count`, `filter_update_age_seconds` | por lista activada (como máximo 32) | `filter` |
| `filter_oldest_update_age_seconds` | antigüedad de la actualización más antigua entre las listas activadas | |
| `filters_never_updated` | listas activadas que nunca se descargaron | |
| `filter_update_interval_hours` | con qué frecuencia se actualizan las listas; 0 desactiva las actualizaciones | |
| `update_check_enabled` | 0 cuando AdGuard Home se ejecuta con su comprobación de versión desactivada | |
| `update_available` | 1 cuando la última comprobación de versión anunció una versión más reciente; ausente cuando la comprobación está desactivada | `latest` |
| `scrape_errors` | llamadas secundarias que fallaron en la última comprobación | |

La comprobación de versión está desactivada en la imagen Docker oficial y
en la mayoría de los paquetes de distribución, que actualizan AdGuard
Home por sí mismos: ahí, `update_available` nunca se escribe y su regla
permanece en silencio.

Las estadísticas de upstream solo cuentan respuestas: un upstream que deja
de responder no deja rastro en ellas. Por eso DumbMonit pide a AdGuard
Home que pruebe cada upstream en cada comprobación, la misma prueba que
el botón Test upstreams en Settings → DNS settings: una consulta DNS de
AdGuard Home hacia ese upstream. Un upstream muerto mantendría a AdGuard
Home esperando el doble de su timeout de upstream (20 segundos por
defecto); DumbMonit deja de esperar tras 4 segundos y lo reporta como
caído. Desmarca la opción Test upstream servers para desactivar esto.

Las [reglas integradas](../alerting/rules.md#adguard-home) que se
aplican: AdGuard Home protection off, AdGuard Home DNS not running,
AdGuard Home upstream failing, AdGuard Home filter lists stale, AdGuard
Home update available, además de Monitor unreachable.

## La página del monitor {#the-device-page}

El panel sobre las gráficas lee lo que guardó la sonda; abrir la página
nunca se conecta a AdGuard Home. Indica si la protección y el servidor
DNS están activados (y cuándo termina una pausa), luego las consultas,
bloqueos y tiempo de respuesta en la ventana de estadísticas, los
servidores upstream con su resultado de prueba y tiempo de respuesta, y
las listas de filtro con sus reglas y última actualización.

## Añadir un usuario para DumbMonit en AdGuard Home {#add-a-user-for-dumbmonit-to-adguard-home}

1. AdGuard Home no tiene ninguna cuenta de solo lectura: cada usuario
   listado en AdGuardHome.yaml puede cambiar cada ajuste. Dale a
   DumbMonit un usuario propio, para poder eliminarlo sin tocar el
   tuyo. Primero crea un hash bcrypt de una contraseña larga y aleatoria
   (htpasswd viene con el paquete apache2-utils o httpd-tools):

    ```
    htpasswd -B -C 10 -n -b dumbmonit 'LONG-RANDOM-PASSWORD'
    ```

2. Detén AdGuard Home, abre AdGuardHome.yaml (junto al binario de
   AdGuardHome, o en la carpeta conf del volumen Docker) y añade esta
   línea bajo «users:», con la misma indentación que la línea «-
   name:» ya existente. Sustituye HASH por lo que imprimió htpasswd
   después de «dumbmonit:». Vuelve a iniciar AdGuard Home.

    ```
    - { name: dumbmonit, password: "HASH" }
    ```

    La sección queda entonces:

    ```yaml
    users:
      - { name: dumbmonit, password: "$2y$10$…" }
      - name: yourname
        password: $2a$10$…
    ```

3. Comprueba desde el host de DumbMonit que el usuario es aceptado:

    ```
    curl -u dumbmonit http://adguard.lan/control/status
    ```

4. En DumbMonit, introduce la dirección de la interfaz web de AdGuard
   Home, por ejemplo «adguard.lan» (puerto 80) o
   «http://adguard.lan:3000», y luego el usuario dumbmonit y su
   contraseña.

5. DumbMonit solo lee el estado, las estadísticas, las listas de filtro y
   la última comprobación de versión. Nunca lee el registro de
   consultas. En cada comprobación pide a AdGuard Home que pruebe cada
   servidor DNS upstream, como hace el botón Test upstreams; desmarca
   Test upstream servers abajo para detener eso.

!!! warning
    El usuario dumbmonit puede cambiar cada ajuste, como cualquier
    usuario de AdGuard Home: mantén su contraseña solo para DumbMonit.
    Viaja con cada petición, así que en una red no confiable activa
    HTTPS en Settings → Encryption settings. Tras cinco contraseñas
    incorrectas, AdGuard Home rechaza la dirección durante 15 minutos.

AdGuard Home debe estar detenido mientras editas el archivo: reescribe
AdGuardHome.yaml cada vez que cambia un ajuste, y descartaría una línea
añadida mientras se ejecuta.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| Nombre de usuario / contraseña | El usuario dedicado añadido a AdGuardHome.yaml, enviado como autenticación básica HTTP. |
| Sin autenticación | Solo para un AdGuard Home sin ningún usuario en AdGuardHome.yaml. |

Dirección: un nombre de host o IP (`adguard.lan`), `host:puerto`, o una
URL completa con un prefijo de ruta detrás de un proxy inverso
(`https://dns.lan/adguard`); se ignora un `/control` final. Opciones:
protocolo (HTTP por defecto), puerto (80), comprobación de certificado,
tiempo de espera de la petición (5 s) y la prueba de upstream
(activada).

## Llamadas {#calls}

| Llamada | Por qué |
|---|---|
| `GET /control/status` | en ejecución, protección, versión: la única llamada cuyo fallo hace fallar la comprobación |
| `GET /control/stats`, `GET /control/stats/config` | consultas, bloqueos, respuestas de upstream, ventana de estadísticas |
| `GET /control/filtering/status` | listas de filtro |
| `POST /control/version.json` con `{"recheck_now": false}` | el resultado de la última comprobación de versión, tal como la lee la interfaz web en cada carga de página; nunca fuerza una comprobación nueva |
| `GET /control/dns_info`, luego `POST /control/test_upstream_dns` una vez por upstream | los upstreams y servidores bootstrap configurados, y luego la prueba de upstream; no cambia ningún ajuste |
