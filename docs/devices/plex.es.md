# Plex Media Server

Qué está haciendo Plex y si está actualizado: streams en reproducción
ahora, cuántos están transcodificados y si por hardware, el ancho de banda
que reservan, streams desde fuera de casa, bibliotecas en escaneo, y
actualizaciones.

DumbMonit lee la propia API del servidor en el puerto 32400, nunca
plex.tv. Nunca detiene un stream, nunca escanea una biblioteca y nunca
cambia un ajuste.

Plex no publica el resultado de sus tareas de mantenimiento (el «butler»
solo da su programación) ni el estado de sus copias de seguridad de base
de datos, así que no hay nada que decir sobre eso.

## Qué vigila {#what-it-watches}

Todas las métricas tienen el prefijo `dumbmonit_plex_`.

| Métrica | Qué | Etiquetas |
|---|---|---|
| `version_info` | valor 1 | `version` |
| `update_available` | 1 cuando la propia comprobación de actualización de Plex encontró una versión más reciente; ausente hasta que Plex haya comprobado una vez | `latest_version` |
| `claimed` | 1 cuando el servidor está vinculado a una cuenta Plex | |
| `streams`, `streams_paused`, `streams_remote` | reproducciones ahora; en pausa; desde fuera de la red local | |
| `transcodes`, `transcodes_video`, `transcodes_hardware` | reproducciones reescodificadas (vídeo o audio); con el vídeo reescodificado; con aceleración por hardware | |
| `transcoder_active_video_sessions` | transcodificaciones de vídeo que el propio servidor cuenta | |
| `stream_bandwidth_bits_per_second` | ancho de banda reservado por todas las reproducciones | |
| `libraries`, `libraries_scanning` | bibliotecas; las que se escanean ahora | |
| `activities` | actividades en segundo plano en ejecución (escaneos, análisis, actualizaciones) | |
| `scrape_errors` | llamadas que fallaron en la última sonda | |

La [regla integrada](../alerting/rules.md#self-hosted-applications) que se
aplica: Plex update available, además de Monitor unreachable.

## La página del monitor {#the-device-page}

El panel sobre las gráficas lee lo que guardó la sonda; abrir la página
nunca llama a Plex. Da el estado de actualización y si el servidor está
vinculado, luego streams, transcodificaciones, transcodificaciones por
hardware, streams remotos, ancho de banda y bibliotecas.

## Dar acceso a DumbMonit al servidor Plex {#give-dumbmonit-access-to-the-plex-server}

1. DumbMonit lee la propia API del servidor en el puerto 32400, nunca
   plex.tv. Un servidor vinculado a una cuenta Plex pide un token
   (X-Plex-Token), y Plex no puede emitir uno limitado: es el token del
   propietario.

2. O bien evita el token: en Plex, Settings → Network → List of IP
   addresses and networks that are allowed without auth, añade la
   dirección del servidor DumbMonit, por ejemplo así, y elige No token
   abajo.

    ```
    10.0.0.20/32
    ```

3. O usa el token: en Plex Web, abre cualquier película → ⋯ → Get Info →
   View XML. La dirección de la página que se abre termina con
   X-Plex-Token= seguido del token; copia ese valor.

4. En DumbMonit, introduce la dirección del servidor, por ejemplo
   «plex.lan» (puerto 32400). DumbMonit solo lee las sesiones, las
   bibliotecas, el actualizador y la lista de actividad; nunca detiene un
   stream y nunca cambia un ajuste.

!!! warning
    El token es el de tu cuenta Plex: quien lo tenga puede administrar el
    servidor y todas las bibliotecas. Prefiere la red permitida de arriba
    cuando DumbMonit está en tu propia red.

La red permitida es también lo que hace que Plex responda sin token desde
Docker: el contenedor de DumbMonit alcanza Plex desde la dirección de su
red Docker, así que lista esa red, o la dirección del host cuando Plex
publica su puerto en el host.

## Credenciales {#credentials}

| Credencial | Campos |
|---|---|
| X-Plex-Token | Enviado en la cabecera `X-Plex-Token`. |
| Sin token (red permitida) | No se envía nada; Plex debe listar la dirección de DumbMonit entre las redes permitidas sin autenticación. |

Dirección: un nombre de host (`plex.lan`), `host:puerto`, o una URL
completa; se acepta un `/web` final. Opciones: protocolo (HTTP por
defecto; los propios certificados de Plex se emiten para nombres
`*.plex.direct`, así que HTTPS por dirección IP necesita desactivar la
comprobación de certificado), puerto (32400), comprobación de certificado y
tiempo de espera de la petición (15 s).

Validado contra Plex Media Server 1.43.4 (imagen oficial), un servidor sin
vincular que responde sin token, con una reproducción transcodificada
real. La vía del token sigue la cabecera `X-Plex-Token` documentada por
Plex y no se probó contra un servidor vinculado.
