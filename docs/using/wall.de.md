# Wall-Modus und Musik

`/wall` zeigt nur das Bulletin, im Vollbild, für einen Bildschirm, der
in einem Raum an bleibt: den Satz, das Wetterfenster, drei große
Anzeigen und die *Needs you*-Liste, alle 20 Sekunden aktualisiert, mit
wachgehaltenem Bildschirm. ++esc++ oder **Exit** führt zur Übersicht
zurück. Öffnen Sie es aus der Befehlspalette ("Wall mode") oder durch
Eingabe der URL. `?theme=dark` oder `?theme=light` erzwingt ein Theme
für diese Anzeige, ohne Ihre gespeicherte Präferenz zu berühren.

![Wall-Modus](../assets/screenshots/wall-light.png){ loading=lazy }

Jedes Problem links trägt das Produktsymbol seines Geräts (dieselben Zeichen wie
in den Gerätelisten). Die Liste wächst nie über ihren Platz hinaus: Auf einem
kleinen Bildschirm oder bei vielen Problemen schrumpfen die großen Kacheln auf
je eine Zeile und fassen sich dann zu *Und N weitere* zusammen, statt die
darunterliegenden Geräte zu überdecken.

## Nachrichten an der Wand

Ein Admin kann der Wall unter **Settings → Wall-Nachrichten** **Schlagzeilen**
und einen **Live-Kanal** hinzufügen. Beides ist standardmäßig aus.

- **Schlagzeilen-Ticker.** Ein Band am unteren Rand der Wall lässt die
  neuesten Schlagzeilen durchlaufen, jeweils mit Quelle und Alter, und
  aktualisiert sich alle fünf Minuten. Ist im System der Anzeige *reduzierte
  Bewegung* aktiviert, gleitet nichts: Eine Schlagzeile nach der anderen blendet
  ein und aus.
- **Live-Video.** Ein stummer Live-Kanal in einer Kachel, in der rechten Spalte
  unter der Uhr (oder in der Mitte der Szene), klein, mittel oder groß. Die
  Kachel schrumpft auf den verbleibenden Platz; sie verdeckt nie den Status.

Der Server liest die Feeds, nicht der Browser (Browser können die Feeds anderer
Sites nicht lesen). Er folgt höchstens drei Weiterleitungen, lehnt interne
Adressen ab, gibt nach acht Sekunden oder 2 MB pro Feed auf und hält die Antwort
fünf Minuten für alle Walls im Cache. Er liest RSS 2.0, RSS 1.0 (RDF) und Atom.

### Quellen wählen

Wählen Sie ein **Land** (vorausgewählt anhand von Region und Sprache Ihres
Browsers) und haken Sie dann seine Quellen an: insgesamt bis zu acht Feeds. Um
einen beliebigen anderen Feed zu nutzen, fügen Sie seine RSS- oder
Atom-Adresse unter **Eigenen Feed hinzufügen** ein und drücken Sie **Testen**:
DumbMonit liest ihn wirklich und zeigt die neueste Schlagzeile, bevor Sie ihn
hinzufügen.

Für das Video wählen Sie einen der Kanäle des Landes oder **Anderer Link…** und
fügen eine normale YouTube-, Dailymotion- oder Twitch-Adresse ein (ein Video,
ein Live oder die `/live`-Seite eines YouTube-Kanals); DumbMonit wandelt sie in
den einbettbaren Player um. Nur diese drei Player können gezeigt werden. Ein
Live wird nur dann als Voreinstellung angeboten, wenn der Kanal einen stabilen
offiziellen hat.

| Land | Schlagzeilen | Live-Video |
|---|---|---|
| Frankreich | BFM TV, franceinfo, Le Monde, France 24 | BFM TV, franceinfo, France 24, Euronews |
| Schweiz | RTS Info, SRF News, Le Temps, NZZ | keine |
| Belgien | RTBF, VRT NWS, La DH | keine |
| Deutschland | Tagesschau, ZDFheute, Spiegel, Deutsche Welle | DW News |
| Vereinigtes Königreich | BBC News, The Guardian | Sky News |
| Vereinigte Staaten | NPR, PBS NewsHour, CBS News, The New York Times | ABC News Live |
| Kanada | Radio-Canada, CBC News | CBC News |
| Spanien | RTVE, 20minutos, El País, El Mundo | keine |
| Italien | ANSA, RaiNews | keine |
| Portugal | RTP Notícias, Público | keine |
| Brasilien | G1, Folha de S.Paulo | CNN Brasil |
| Russland | Interfax, Meduza, Kommersant | keine |
| China | CGTN, South China Morning Post, Xinhua | CGTN |
| International | Deutsche Welle, France 24, BBC News (world) | DW News, France 24 English, Euronews English |

Der Feed von BFM TV ist `https://www.bfmtv.com/rss/news-24-7/`. Die Liste wurde
bei ihrer Erstellung geprüft; eine Site kann ihren Feed jederzeit verschieben,
und dann überspringt der Ticker ihn einfach. **Testen** Sie den Ersatz und
fügen Sie ihn als eigenen Feed hinzu.

Die Content Security Policy der Wall erlaubt der Seite nur Player von YouTube
(`youtube-nocookie.com`), Dailymotion, Twitch, Spotify und Deezer einzubetten.

### API

`GET /api/wall/news` (was die Walls abfragen), `GET|PUT /api/wall/news/settings`
und `POST /api/wall/news/test` (Admin), `GET /api/wall/news/presets`.

## Musik an der Wand

Die Wall kann auch Musik abspielen und zeigen, was läuft, sodass Sie
eine Playlist von Ihrem Telefon starten, sie vom Fernseher hören und
vom anderen Ende des Raums sehen können. Musik sitzt immer in der
unteren Ecke des Wetterfensters, über dem Himmel: Sie nimmt dem
Bulletin oder der *Needs you*-Liste nie Platz weg, und verschwindet,
wenn nichts läuft. Jede Karte klappt sich zu einer kleinen Pille
zusammen (die Anzeige merkt sich das).

Es gibt zwei Wege, und Sie können beide nutzen.

| | Spotify Connect | Ein an die Walls gesendeter Link |
|---|---|---|
| Gestartet von | der Spotify-App auf Ihrem Telefon (**Devices → DumbMonit Wall**) | jedem DumbMonit-Bildschirm, Ihr Telefon eingeschlossen (**Play on the wall**) |
| Dienste | Spotify | Spotify, Deezer, YouTube und YouTube Music |
| Braucht | Spotify Premium, eine eigene Spotify-Developer-App, das Telefon mit demselben Spotify-Konto, die Wall über HTTPS (oder localhost) bedient in einem Browser mit DRM | nichts; ohne Einloggen auf der Wall spielen Spotify und Deezer 30-Sekunden-Vorschauen |
| Zeigt, was läuft | ja, wo auch immer es läuft: die Wall, die Spotify-App eines Fernsehers, ein Cast-Lautsprecher | der eigene Player des Dienstes |

Spotify zu verbinden und Links zu senden sind Admin-Aktionen; ein
Viewer-Konto auf der Wall spielt und zeigt alles.

### Spotify Connect: die Wall wird zum Lautsprecher

Das Ziel: Auf Ihrem Telefon öffnen Sie Spotify, tippen auf das
Geräte-Symbol, wählen den Fernseher — die Musik läuft auf dem
Fernseher, während die Wall zeigt, was läuft. Die Wall tut das mit
Spotifys **Web Playback SDK**: Der Browser der Wall registriert sich
selbst als Spotify-Connect-Gerät. Das funktioniert nur, wenn **alle**
diese Punkte zutreffen, und die Wall sagt auf ihrem eigenen Bildschirm,
welcher fehlt:

| Braucht | Warum | Wenn es fehlt |
|---|---|---|
| **Spotify Premium** auf dem verbundenen Konto | Spotify lässt nur Premium-Konten Browser-Lautsprecher nutzen | die Wall zeigt *Spotify Premium is required*; Settings zeigt *Premium: No* |
| **Das Telefon mit demselben Spotify-Konto eingeloggt** | ein Browser-Lautsprecher wird nur für das Konto gelistet, dessen Token er hält — er wird nicht wie ein smarter Lautsprecher im Netzwerk entdeckt, und andere Mitglieder eines Family-Plans sehen ihn nicht | das Telefon listet die Wall einfach nicht |
| **HTTPS** (oder `http://localhost` auf der Anzeige selbst) | Browser erlauben geschützte Medien (DRM) nur auf einer sicheren Seite | die Wall zeigt *opened over plain HTTP* |
| **Ein Browser mit funktionierendem DRM** (Widevine, PlayReady oder FairPlay) | Spotify streamt verschlüsseltes Audio | die Wall zeigt *This browser cannot play Spotify* |
| **Ein Tap oder Tastendruck auf der Wall pro Seitenladen** | Browser halten eine Seite stumm, bis jemand mit ihr interagiert | die Wall zeigt *Tap to enable sound* |

Welche Browser funktionieren: Chrome, Edge und Firefox auf einem
Computer (Firefox: *Play DRM-controlled content* erlauben), Safari
auf einem Mac, Chromium mit Widevine auf einem Raspberry Pi
(`libwidevinecdm0`). Die meisten **Smart-TV-Browser** (Samsung Tizen,
LG webOS), **Kiosk-Browser** und viele **Linux-Chromium-Builds**
liefern ohne nutzbares Widevine: Sie können kein Spotify-Lautsprecher
sein, egal was DumbMonit tut. Für einen solchen Fernseher siehe
[Wenn der Browser des Fernsehers Spotify nicht abspielen kann](#when-the-tvs-browser-cannot-play-spotify).

#### Einrichten

1. **Eine Spotify-App erstellen.** Öffnen Sie das
   [Spotify Developer Dashboard](https://developer.spotify.com/dashboard),
   erstellen Sie eine App (beliebiger Name), kreuzen Sie **Web API**
   und **Web Playback SDK** an, und fügen Sie die Redirect-URI hinzu,
   die **Settings → Wall music** zeigt:
    - DumbMonit über HTTPS bedient: `https://<Ihre Adresse>/api/music/spotify/callback`.
      Spotify kommt direkt zu DumbMonit zurück, nachdem Sie zugestimmt
      haben.
    - DumbMonit über einfaches HTTP bedient: `http://127.0.0.1:8888/callback`.
      Spotify akzeptiert `http://` seit 2025 nur für eine
      Loopback-Adresse (und nicht `localhost`). Auf dieser Adresse
      antwortet nichts: Nach Ihrer Zustimmung zeigt Ihr Browser eine
      Fehlerseite, deren Adresse den Code trägt — kopieren Sie ihn
      und fügen Sie ihn in DumbMonit ein.
2. **Verbinden.** Fügen Sie unter **Settings → Wall music** die
   **Client ID** der App ein (es gibt kein Secret: DumbMonit nutzt
   Authorization Code mit PKCE) und wählen Sie **Connect Spotify**,
   dann stimmen Sie auf Spotify **mit dem Konto zu, das Ihr Telefon
   nutzt**.
3. **Auf dem Fernseher** öffnen Sie DumbMonits `/wall` über HTTPS,
   eingeloggt (ein Viewer-Konto reicht), und tippen Sie einmal auf
   den Bildschirm oder drücken Sie **OK** auf der Fernbedienung:
   jede Geste schaltet den Ton frei. Ein mit
   `--autoplay-policy=no-user-gesture-required` gestarteter Browser
   (ein Kiosk) braucht keinen Tap.
4. **Auf Ihrem Telefon**, in der Spotify-App, tippen Sie auf das
   Geräte-Symbol und wählen Sie **DumbMonit Wall**. Der Ton kommt aus
   dem Fernseher, und die Wall zeigt das Cover, den Titel und den
   Fortschritt.

**Settings → Wall music → Speaker** zeigt, ob alles stimmt, ohne zum
Fernseher zu laufen: das Konto, Premium, ob Spotify den Lautsprecher
gerade unter den Geräten des Kontos listet (und welche Geräte es
listet), und jede Wall, die sich in den letzten fünfzehn Minuten
gemeldet hat — ihren Browser ("Chrome 130 on Linux"), ob sie bereit
ist, und warum nicht. **Test sound** lässt das Konto auf dem
Lautsprecher spielen (es setzt fort, was zuletzt lief); jede bereite
Wall hat auch ihr eigenes **Test sound**. Auf der Wall selbst hat das
**Music**-Panel **Play here**, das verschiebt, was auch immer auf dem
Konto spielt, auf diese Anzeige.

**Der Name.** Benennen Sie den Lautsprecher in **Settings → Wall
music → Speaker** um ("Living room TV"); jede Wall übernimmt den
neuen Namen innerhalb von Sekunden. Um eine einzelne Wall zu benennen,
fügen Sie ihrer Adresse `?speaker=Kitchen` hinzu — die Anzeige merkt
sich das (`?speaker=` vergisst es).

**Eine Wall bleibt tagelang an.** Die Wall fragt DumbMonit nach einem
frischen Token, wenn Spotifys abläuft (jede Stunde); wenn das Gerät
ausfällt (WLAN, eine schlafende Netzwerkkarte) und nicht innerhalb von
dreißig Sekunden zurückkommt, registriert die Wall sich erneut; ein
vorübergehender Fehlschlag wird von selbst wiederholt (15 s, 30 s, …
dann alle fünf Minuten); und wenn Spotify aufhört, die Wall zu
listen, während sie sich für bereit hält, registriert sie sich
erneut. Nur was Warten nicht beheben kann — kein DRM, kein Premium —
stoppt endgültig, bis **Try again**.

DumbMonit behält Spotifys Refresh-Token mit dem Instanz-Secret
verschlüsselt und sendet ihn nie an einen Browser. Die Wall erhält
nur ein kurzlebiges Access-Token für ihren Player, und nur über eine
eingeloggte Sitzung (ein API-Token kann keines erhalten).
**Disconnect** vergisst das Konto; um DumbMonit auch aus Ihrem
Spotify-Konto zu entfernen, nutzen Sie
[spotify.com/account/apps](https://www.spotify.com/account/apps/).

Auch gut zu wissen:

- Eine persönliche Spotify-App (*Development Mode*) gehört einem
  Premium-Konto und lässt bis zu fünf Konten zu: Fügen Sie andere
  unter **User Management** im Dashboard hinzu.
- **Alle sechs Monate neu verbinden.** Spotify beendet eine Verbindung
  sechs Monate nach ihrer Genehmigung; Settings zeigt das Datum, und
  die Wall sagt es, wenn es passiert.

**Now Playing funktioniert ohne all das.** DumbMonits Server fragt
Spotify alle paar Sekunden, was auf dem Konto läuft (ein Request für
alle Walls), und die Wall zeigt das Cover, den Titel, den Künstler,
den Fortschritt und das Gerät, auf dem es läuft — die Wall selbst,
die Spotify-App eines Smart-TVs, ein Chromecast, Ihr Laptop — sogar
über einfaches HTTP und in einem Browser, der Spotify nicht abspielen
kann. Um nur das zu nutzen, schalten Sie **Use this display as a
Spotify speaker** im Music-Panel der Wall aus.

#### Wenn der Browser des Fernsehers Spotify nicht abspielen kann {#when-the-tvs-browser-cannot-play-spotify}

Wählen Sie, was zu Ihrer Einrichtung passt — in jedem Fall zeigt die
Wall weiter, was läuft, weil *Now Playing* das Konto liest, nicht den
Lautsprecher:

- **Der Fernseher hat eine Spotify-App, oder einen Chromecast /
  Google TV / Fire-TV-Stick.** Wählen Sie das in Spotifys
  Geräteliste; die Wall zeigt "Playing on Living room TV". Das ist
  die einfachste und zuverlässigste Option.
- **Ein kleiner Computer steuert den Fernseher** (ein Raspberry Pi,
  ein Mini-PC, ein alter Laptop). Öffnen Sie die Wall in Chrome oder
  Chromium mit Widevine darauf: Sie wird der Lautsprecher, wie oben
  beschrieben.
- **Sie wollen einen Lautsprecher, der immer da ist, auch ohne
  Browser.** Betreiben Sie einen Spotify-Connect-Receiver auf Basis
  von [librespot](https://github.com/librespot-org/librespot) (MIT)
  — zum Beispiel [raspotify](https://github.com/dtcooper/raspotify)
  oder spotifyd — **auf der Maschine, deren Audio-Ausgang den
  Fernseher oder den Lautsprecher erreicht**. Er erscheint in der
  Geräteliste des Telefons für jeden im Netzwerk.

DumbMonit liefert absichtlich keinen solchen Receiver selbst: Der
DumbMonit-Container läuft meist auf einem Server im Schrank, nicht auf
der an den Fernseher angeschlossenen Maschine, daher käme sein Ton aus
der falschen Box; ein Receiver braucht die Soundkarte des Hosts und
Host-Networking (Multicast DNS), was ein Monitoring-Container nicht
haben sollte; und librespot hängt von inoffiziellen Spotify-
Protokollen ab, die von Zeit zu Zeit kaputtgehen. Ein auf der
richtigen Maschine, neben DumbMonit, installierter Receiver macht die
Arbeit besser.

### Ein an jede Wall gesendeter Link

Fügen Sie unter **Settings → Wall music** (oder dem eigenen
**Music**-Panel der Wall) einen Spotify-, Deezer- oder YouTube-Link
zu einem Track, Album, einer Playlist, Episode oder einem Video ein
und wählen Sie **Play on the wall**. Jede Wall spielt ihn im
offiziellen eingebetteten Player des Dienstes; ein Link, der gesendet
wird, während eine Wall offen ist, startet von selbst, wo der Dienst
es erlaubt (YouTube, Deezer), sobald die Wall angetippt wurde.
**Stop** entfernt ihn von jeder Wall.

Deezer und YouTube haben kein Spotify-Connect-Äquivalent, das eine
Webseite nutzen kann — Deezer akzeptiert keine neuen Developer-Apps
mehr, und seine API hat nie gezeigt, was läuft — daher ist ein Link
der Weg für sie. Die eingebetteten Player spielen 30-Sekunden-
Vorschauen, sofern der Browser der Wall nicht beim Dienst eingeloggt
ist.

### Datenschutz

Nichts von Drittanbietern lädt auf der Wall, bis Spotify verbunden
oder ein Link gesetzt ist. Die Content-Security-Policy der Seite
erlaubt genau die drei eingebetteten Player, Spotifys Player-Skript
(`sdk.scdn.co/spotify-player.js`) und seinen Frame, und Cover von
`i.scdn.co` — nichts sonst, und nichts außerhalb der Web-Oberfläche.

### API

| Route | Wer | Was |
|---|---|---|
| `GET /api/music/now` | jede Sitzung oder jedes Token | Was auf dem Spotify-Konto läuft (ein paar Sekunden gecacht) und der von den Walls gespielte Link |
| `PUT /api/music/link` `{"link": "…"}` | admin | Jede Wall spielt diesen Link |
| `DELETE /api/music/link` | admin | Die Walls stoppen den eingebetteten Player |
| `GET /api/music/spotify` | jede Sitzung oder jedes Token | Das verbundene Konto: Status, Client ID, Datum für erneutes Verbinden — nie ein Token |
| `POST /api/music/spotify/authorize` | Admin-Sitzung | Startet eine Verbindung: `{"client_id", "redirect_uri"}` → Spotifys Zustimmungs-URL |
| `POST /api/music/spotify/complete` `{"url": "…"}` | Admin-Sitzung | Schließt eine Loopback-Verbindung mit der Adresse ab, auf der der Browser gelandet ist |
| `GET /api/music/spotify/callback` | der Browser des Admins | Spotifys direkte Weiterleitung (HTTPS-Installationen) |
| `DELETE /api/music/spotify` | admin | Trennt und vergisst die Tokens |
| `GET /api/music/spotify/token` | nur Browser-Sitzung | Ein kurzlebiges Access-Token für den Player der Wall |
| `GET /api/music/speaker` | jede Sitzung oder jedes Token | Der Lautsprecher: Name, Premium, die Spotify-Connect-Geräte des Kontos, ob der Lautsprecher gelistet ist, was jede Wall gemeldet hat |
| `PUT /api/music/speaker` `{"name": "…"}` | admin | Benennt den Lautsprecher um (`null` geht zurück zu "DumbMonit Wall") |
| `POST /api/music/speaker/report` | nur Browser-Sitzung | Eine Wall meldet ihren Lautsprecher (jede Minute gesendet); die Antwort sagt, ob Spotify ihn listet |
| `POST /api/music/speaker/play` `{"device_id": "…"}` | nur Browser-Sitzung | Spielt das Konto auf diesem Gerät, oder auf dem wie der Lautsprecher benannten Gerät ("Test sound", "Play here") |
