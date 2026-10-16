# Modo pared y música {#wall-mode-and-music}

`/wall` muestra solo el boletín, a pantalla completa, para una pantalla
que se mantiene encendida en una sala: la frase, la ventana
meteorológica, tres indicadores grandes y la lista *Needs you*,
actualizados cada 20 segundos, con la pantalla mantenida despierta.
++esc++ o **Exit** vuelve al resumen. Ábrelo desde la paleta de comandos
(«Wall mode») o escribiendo la URL. `?theme=dark` o `?theme=light` fuerza
un tema para esa pantalla sin tocar tu preferencia guardada.

![Modo pared](../assets/screenshots/wall-light.png){ loading=lazy }

Cada problema de la izquierda lleva el icono de producto de su monitor (las
mismas marcas que en las listas de monitores). La lista nunca desborda su
espacio: en una pantalla pequeña, o con muchos problemas, los recuadros grandes
se reducen a una línea cada uno y luego se pliegan en *Y N más*, en lugar de
solaparse con los monitores de abajo.

## Noticias en la pared {#news-on-the-wall}

Un administrador puede añadir **titulares** y un **canal en directo** a la pared
en **Ajustes → Noticias del mural**. Ambos están desactivados por defecto.

- **Cinta de titulares.** Una banda en la parte inferior de la pared desplaza los
  últimos titulares, cada uno con su fuente y su antigüedad, y se actualiza cada
  cinco minutos. Con *reducir movimiento* activado en el sistema de la pantalla,
  nada se desliza: un titular a la vez aparece y se desvanece.
- **Vídeo en directo.** Un canal en directo sin sonido en un recuadro, en la
  columna derecha bajo el reloj (o en medio de la escena), pequeño, mediano o
  grande. El recuadro se reduce al espacio disponible; nunca tapa el estado.

El servidor lee los feeds, no el navegador (los navegadores no pueden leer los
feeds de otros sitios). Sigue como máximo tres redirecciones, rechaza las
direcciones internas, se rinde tras ocho segundos o 2 MB por feed, y guarda la
respuesta en caché cinco minutos para todas las paredes. Lee RSS 2.0, RSS 1.0
(RDF) y Atom.

### Elegir las fuentes {#choosing-sources}

Elige un **país** (preseleccionado según la región y el idioma de tu navegador),
y marca después sus fuentes: hasta ocho feeds en total. Para usar cualquier otro
feed, pega su dirección RSS o Atom en **Añadir un feed propio** y pulsa
**Probar**: DumbMonit lo lee de verdad y muestra el último titular antes de que
lo añadas.

Para el vídeo, elige uno de los canales del país u **Otro enlace…** y pega una
dirección normal de YouTube, Dailymotion o Twitch (un vídeo, un directo, o la
página `/live` de un canal de YouTube); DumbMonit la convierte en el reproductor
incrustable. Solo se pueden mostrar estos tres reproductores. Un directo se ofrece
como preajuste solo cuando el canal tiene uno oficial y estable.

| País | Titulares | Vídeo en directo |
|---|---|---|
| Francia | BFM TV, franceinfo, Le Monde, France 24 | BFM TV, franceinfo, France 24, Euronews |
| Suiza | RTS Info, SRF News, Le Temps, NZZ | ninguno |
| Bélgica | RTBF, VRT NWS, La DH | ninguno |
| Alemania | Tagesschau, ZDFheute, Spiegel, Deutsche Welle | DW News |
| Reino Unido | BBC News, The Guardian | Sky News |
| Estados Unidos | NPR, PBS NewsHour, CBS News, The New York Times | ABC News Live |
| Canadá | Radio-Canada, CBC News | CBC News |
| España | RTVE, 20minutos, El País, El Mundo | ninguno |
| Italia | ANSA, RaiNews | ninguno |
| Portugal | RTP Notícias, Público | ninguno |
| Brasil | G1, Folha de S.Paulo | CNN Brasil |
| Rusia | Interfax, Meduza, Kommersant | ninguno |
| China | CGTN, South China Morning Post, Xinhua | CGTN |
| Internacional | Deutsche Welle, France 24, BBC News (world) | DW News, France 24 English, Euronews English |

El feed de BFM TV es `https://www.bfmtv.com/rss/news-24-7/`. La lista se
comprobó cuando se escribió; un sitio puede mover su feed en cualquier momento, y
entonces la cinta simplemente lo omite. Usa **Probar** con el reemplazo y añádelo
como feed propio.

La política de seguridad de contenido de la pared solo permite enmarcar los
reproductores de YouTube (`youtube-nocookie.com`), Dailymotion, Twitch, Spotify
y Deezer.

### API {#news-api}

`GET /api/wall/news` (lo que consultan las paredes), `GET|PUT
/api/wall/news/settings` y `POST /api/wall/news/test` (admin), `GET
/api/wall/news/presets`.

## Música en la pared {#music-on-the-wall}

La pared también puede reproducir música y mostrar qué está sonando,
para que puedas iniciar una lista de reproducción desde tu teléfono,
oírla desde la TV, y verla desde el otro lado de la sala. La música
siempre se sitúa en la esquina inferior de la ventana meteorológica,
sobre el cielo: nunca quita espacio al boletín ni a la lista *Needs
you*, y desaparece cuando nada suena. Cada tarjeta se pliega en una
pequeña píldora (la pantalla lo recuerda).

Hay dos formas, y puedes usar ambas.

| | Spotify Connect | Un enlace enviado a las pantallas |
|---|---|---|
| Iniciarlo desde | la app de Spotify en tu teléfono (**Devices → DumbMonit Wall**) | cualquier pantalla de DumbMonit, tu teléfono incluido (**Play on the wall**) |
| Servicios | Spotify | Spotify, Deezer, YouTube y YouTube Music |
| Necesita | Spotify Premium, una app de Spotify Developer propia, el teléfono en la misma cuenta de Spotify, la pared servida por HTTPS (o localhost) en un navegador con DRM | nada; sin haber iniciado sesión en la pared, Spotify y Deezer reproducen previsualizaciones de 30 segundos |
| Muestra qué suena | sí, dondequiera que suene: la pared, la app de Spotify de una TV, un altavoz cast | el propio reproductor del servicio |

Conectar Spotify y enviar enlaces son acciones de administrador; una
cuenta de visor en la pared reproduce y muestra todo.

### Spotify Connect: la pared se convierte en un altavoz {#spotify-connect-the-wall-becomes-a-speaker}

El objetivo: en tu teléfono, abre Spotify, pulsa el icono de
dispositivos, elige la TV — la música suena en la TV mientras la pared
muestra qué está sonando. La pared hace esto con el **Web Playback
SDK** de Spotify: el navegador de la pared se registra como un
dispositivo Spotify Connect. Eso solo funciona cuando **todo** esto se
cumple, y la pared te indica en su propia pantalla cuál falta:

| Necesita | Por qué | Cuando falta |
|---|---|---|
| **Spotify Premium** en la cuenta conectada | Spotify solo deja a las cuentas Premium usar altavoces de navegador | la pared muestra *Spotify Premium is required*; Settings muestra *Premium: No* |
| **El teléfono con sesión iniciada en la misma cuenta de Spotify** | un altavoz de navegador solo se lista para la cuenta cuyo token tiene — no se descubre en la red como un altavoz inteligente, y otros miembros de un plan Family no lo ven | el teléfono simplemente no lista la pared |
| **HTTPS** (o `http://localhost` en la propia pantalla) | los navegadores solo permiten medios protegidos (DRM) en una página segura | la pared muestra *opened over plain HTTP* |
| **Un navegador cuyo DRM funcione** (Widevine, PlayReady o FairPlay) | Spotify transmite audio cifrado | la pared muestra *This browser cannot play Spotify* |
| **Un toque o pulsación de tecla en la pared por carga de página** | los navegadores mantienen una página en silencio hasta que alguien interactúa con ella | la pared muestra *Tap to enable sound* |

Qué navegadores funcionan: Chrome, Edge y Firefox en un ordenador
(Firefox: permite *Play DRM-controlled content*), Safari en un Mac,
Chromium con Widevine en una Raspberry Pi (`libwidevinecdm0`). La
mayoría de los **navegadores de smart TV** (Samsung Tizen, LG webOS),
los **navegadores de kiosco** y muchas **compilaciones Linux de
Chromium** vienen sin un Widevine utilizable: no pueden ser un altavoz
de Spotify, haga lo que haga DumbMonit. Para una TV así, consulta
[Cuando el navegador de la TV no puede reproducir Spotify](#when-the-tvs-browser-cannot-play-spotify).

#### Configúralo {#set-it-up}

1. **Crea una app de Spotify.** Abre el
   [Spotify Developer Dashboard](https://developer.spotify.com/dashboard), crea
   una app (cualquier nombre), marca **Web API** y **Web Playback SDK**, y
   añade el URI de redirección que muestra **Settings → Wall music**:
    - DumbMonit servido por HTTPS: `https://<tu dirección>/api/music/spotify/callback`.
      Spotify vuelve directamente a DumbMonit después de que apruebes.
    - DumbMonit servido por HTTP plano: `http://127.0.0.1:8888/callback`.
      Spotify solo acepta `http://` para una dirección loopback (y no
      `localhost`), desde 2025. Nada responde en esa dirección: tras
      aprobar, tu navegador muestra una página de error cuya dirección
      lleva el código — cópialo y pégalo de vuelta en DumbMonit.
2. **Conecta.** En **Settings → Wall music**, pega el **Client ID** de
   la app (no hay secreto: DumbMonit usa Authorization Code con PKCE) y
   elige **Connect Spotify**, y luego aprueba en Spotify **con la
   cuenta que usa tu teléfono**.
3. **En la TV**, abre `/wall` de DumbMonit por HTTPS, con sesión
   iniciada (una cuenta de visor basta), y toca la pantalla o pulsa
   **OK** en el control remoto una vez: cualquier gesto desbloquea el
   sonido. Un navegador iniciado con
   `--autoplay-policy=no-user-gesture-required` (un kiosco) no necesita
   ningún toque.
4. **En tu teléfono**, en la app de Spotify, toca el icono de
   dispositivos y elige **DumbMonit Wall**. El sonido sale de la TV, y
   la pared muestra la portada, título y progreso.

**Settings → Wall music → Speaker** muestra si todo se cumple, sin
caminar hasta la TV: la cuenta, Premium, si Spotify lista el altavoz
entre los dispositivos de la cuenta ahora mismo (y qué dispositivos
lista), y cada pared que reportó en los últimos quince minutos — su
navegador («Chrome 130 on Linux»), si está lista, y por qué no. **Test
sound** hace que la cuenta reproduzca en el altavoz (reanuda lo que
sonaba por última vez); cada pared lista también tiene su propio **Test
sound**. En la propia pared, el panel **Music** tiene **Play here**, que
mueve lo que sea que suena en la cuenta a esta pantalla.

**El nombre.** Renombra el altavoz en **Settings → Wall music →
Speaker** («Living room TV»); cada pared recoge el nombre nuevo en
segundos. Para nombrar solo una pared, añade `?speaker=Kitchen` a su
dirección — la pantalla lo recuerda (`?speaker=` lo olvida).

**Una pared se mantiene encendida durante días.** La pared pide a
DumbMonit un token nuevo cuando el de Spotify caduca (cada hora); cuando
el dispositivo se desconecta (Wi-Fi, una tarjeta de red dormida) y no
vuelve en treinta segundos, la pared se registra de nuevo; un fallo
pasajero se reintenta por sí mismo (15 s, 30 s, … y luego cada cinco
minutos); y si Spotify deja de listar la pared mientras ella cree estar
lista, se registra de nuevo. Solo lo que esperar no puede arreglar — sin
DRM, sin Premium — se detiene definitivamente, hasta **Try again**.

DumbMonit conserva el token de refresco de Spotify cifrado con el
secreto de la instancia y nunca lo envía a un navegador. La pared solo
recibe un token de acceso de corta duración para su reproductor, y solo
a través de una sesión con inicio de sesión (un token de API no puede
obtener uno). **Disconnect** olvida la cuenta; para también eliminar
DumbMonit de tu cuenta de Spotify, usa
[spotify.com/account/apps](https://www.spotify.com/account/apps/).

También conviene saber:

- Una app personal de Spotify (*modo desarrollo*) pertenece a una
  cuenta Premium y admite hasta cinco cuentas: añade otras bajo **User
  Management** en el dashboard.
- **Reconecta cada seis meses.** Spotify termina una conexión seis
  meses después de haber sido aprobada; Settings muestra la fecha, y la
  pared lo indica cuando ocurre.

**Now playing funciona sin nada de eso.** El servidor de DumbMonit
pregunta a Spotify qué está sonando en la cuenta cada pocos segundos
(una petición para todas las paredes) y la pared muestra la portada,
título, artista, progreso y el dispositivo en el que suena — la propia
pared, la app de Spotify de una smart TV, un Chromecast, tu portátil —
incluso por HTTP plano y en un navegador que no puede reproducir
Spotify. Para usar solo eso, desactiva **Use this display as a Spotify
speaker** en el panel Music de la pared.

#### Cuando el navegador de la TV no puede reproducir Spotify {#when-the-tvs-browser-cannot-play-spotify}

Elige lo que coincida con tu configuración — en todos los casos la
pared sigue mostrando qué suena, porque *now playing* lee la cuenta, no
el altavoz:

- **La TV tiene una app de Spotify, o un Chromecast / Google TV / Fire
  TV stick.** Elígela en la lista de dispositivos de Spotify; la pared
  muestra «Playing on Living room TV». Esta es la opción más simple y
  fiable.
- **Un pequeño ordenador controla la TV** (una Raspberry Pi, un mini
  PC, un portátil viejo). Abre la pared en Chrome o Chromium con
  Widevine instalado: se convierte en el altavoz como se describe
  arriba.
- **Quieres un altavoz que esté siempre ahí, incluso sin un
  navegador.** Ejecuta un receptor Spotify Connect basado en
  [librespot](https://github.com/librespot-org/librespot) (MIT) — por
  ejemplo [raspotify](https://github.com/dtcooper/raspotify) o
  spotifyd — **en la máquina cuya salida de audio llega a la TV o al
  altavoz**. Aparece en la lista de dispositivos del teléfono para
  todos en la red.

DumbMonit no incluye tal receptor por sí mismo, a propósito: el
contenedor de DumbMonit normalmente se ejecuta en un servidor en un
armario, no en la máquina conectada a la TV, así que su sonido saldría
de la caja equivocada; un receptor necesita la tarjeta de sonido del
host y redes a nivel de host (multicast DNS) que un contenedor de
monitorización no debería tener; y librespot depende de protocolos no
oficiales de Spotify que se rompen de vez en cuando. Un receptor
instalado en la máquina correcta, junto a DumbMonit, hace mejor el
trabajo.

### Un enlace enviado a cada pared {#a-link-sent-to-every-wall}

En **Settings → Wall music** (o en el propio panel **Music** de la
pared), pega un enlace de Spotify, Deezer o YouTube a una pista, álbum,
lista de reproducción, episodio o vídeo y elige **Play on the wall**.
Cada pared lo reproduce en el reproductor embebido oficial del
servicio; un enlace enviado mientras una pared está abierta empieza por
sí solo donde el servicio lo permite (YouTube, Deezer) una vez que se
ha tocado la pared. **Stop** lo retira de cada pared.

Deezer y YouTube no tienen ningún equivalente de Spotify Connect que
una página web pueda usar — Deezer ya no acepta apps de desarrollador
nuevas y su API nunca expuso qué está sonando — así que un enlace es la
vía para ellos. Los reproductores embebidos reproducen previsualizaciones
de 30 segundos a menos que el navegador de la pared tenga sesión
iniciada en el servicio.

### Privacidad {#privacy}

Nada de terceros carga en la pared hasta que Spotify se conecta o se
define un enlace. La política de seguridad de contenido de la página
permite exactamente los tres reproductores embebidos, el script del
reproductor de Spotify (`sdk.scdn.co/spotify-player.js`) y su frame, y
cubre desde `i.scdn.co` — nada más, y nada fuera de la interfaz web.

### API {#api}

| Ruta | Quién | Qué |
|---|---|---|
| `GET /api/music/now` | cualquier sesión o token | Qué suena en la cuenta de Spotify (en caché unos segundos) y el enlace que reproducen las paredes |
| `PUT /api/music/link` `{"link": "…"}` | administrador | Cada pared reproduce este enlace |
| `DELETE /api/music/link` | administrador | Las paredes detienen el reproductor embebido |
| `GET /api/music/spotify` | cualquier sesión o token | La cuenta conectada: estado, Client ID, fecha límite de reconexión — nunca un token |
| `POST /api/music/spotify/authorize` | sesión de administrador | Inicia una conexión: `{"client_id", "redirect_uri"}` → URL de aprobación de Spotify |
| `POST /api/music/spotify/complete` `{"url": "…"}` | sesión de administrador | Termina una conexión loopback con la dirección donde aterrizó el navegador |
| `GET /api/music/spotify/callback` | el navegador del administrador | La redirección directa de Spotify (instalaciones HTTPS) |
| `DELETE /api/music/spotify` | administrador | Desconecta y olvida los tokens |
| `GET /api/music/spotify/token` | solo sesión de navegador | Un token de acceso de corta duración para el reproductor de la pared |
| `GET /api/music/speaker` | cualquier sesión o token | El altavoz: nombre, Premium, los dispositivos Spotify Connect de la cuenta, si el altavoz está listado, qué reportó cada pared |
| `PUT /api/music/speaker` `{"name": "…"}` | administrador | Renombra el altavoz (`null` vuelve a «DumbMonit Wall») |
| `POST /api/music/speaker/report` | solo sesión de navegador | Una pared reporta su altavoz (enviado cada minuto); la respuesta dice si Spotify lo lista |
| `POST /api/music/speaker/play` `{"device_id": "…"}` | solo sesión de navegador | Reproduce la cuenta en ese dispositivo, o en el dispositivo nombrado como el altavoz («Test sound», «Play here») |
