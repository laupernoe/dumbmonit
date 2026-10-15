# Statusseiten

![Die Status-Seite: Ihre Seiten, und die Ankündigungen darunter](../assets/screenshots/status-light.png){ loading=lazy }

Eine Statusseite ist die öffentliche Seite Ihrer Überwachung: eine
Seite, die jeder öffnen kann — kein Login, kein Cookie — die sagt, ob
Ihre Dienste laufen, bis zu 90 Tage tägliche Historie für jeden zeigt,
Ihre Vorfall- und Wartungsankündigungen trägt, und Besuchern erlaubt,
ihnen per RSS oder E-Mail zu folgen. Badges bringen dieselbe Antwort
in ein README; eine kompakte Ansicht passt auf eine Intranet-Seite.

Sie zeigt nur, was Sie darauf legen: das Label, das Sie jedem Dienst
geben, seinen Zustand, seine Uptime und seine Antwortzeit.
Monitornamen, Adressen, Arten und Kennungen verlassen den Server nie
— nicht auf der Seite, nicht in einem Badge, nicht in einer E-Mail.

## 1. Eine Seite erstellen

**Settings → Status pages** → **New page**. Diese Seite listet
Ihre Statusseiten (öffnen, bearbeiten, löschen) mit den Ankündigungen
darunter; der Editor öffnet sich auf seiner eigenen Route
(`/settings/status/new`, `/settings/status/<id>`). Die öffentliche Darstellung liegt
unter `/s/<address>`.

| Feld | Was es tut |
| --- | --- |
| **Title** | Die Überschrift der Seite ("Home lab"). |
| **Address** | Der letzte Teil der URL, `/s/<address>`. Aus dem Titel vorgeschlagen; Kleinbuchstaben, Ziffern und Bindestriche, 2 bis 40 Zeichen. |
| **Description** | Ein Satz unter dem Titel. Optional. |
| **Theme** | Day, Night, oder dem System des Besuchers folgen. |
| **History** | Das längste von der Historienleiste abgedeckte Fenster (30, 60 oder 90 Tage). Die Leiste passt sich dem Alter der Daten der Seite an: Sie zeigt so viele Tage, wie der älteste Dienst überwacht wird, von 7 Tagen bis zu dieser Einstellung. Eine Seite sagt nie mehr als das — siehe [Badges](#badges). |
| **Simple mode** | Eine schlichte Seite: keine Scene, kein Mascot, keine Animation und keine Easter Eggs. Hell oder dunkel folgt dem System des Besuchers, mit einem Umschalter im Header; die Theme-Einstellung der Seite wird ignoriert. |
| **Published** | Aus = Entwurf: Die Seite, ihre Badges und ihr Logo antworten Besuchern "not found", bis Sie es einschalten. |

Kreuzen Sie dann die anzuzeigenden Monitore an. Für jedes setzen Sie
das **Label**, das Besucher lesen werden (standardmäßig der
Monitorname — Sie bevorzugen vielleicht "Website" statt
"nginx-front-01"), und optional eine **Group** ("Network",
"Storage"). Gruppen werden zu Blöcken auf der Seite, in der
Reihenfolge, in der Sie die Dienste anordnen.

Speichern, und der Link ist bereit zum Kopieren. Die Seite
aktualisiert sich jede Minute selbst.

### Look

Der Abschnitt **Look** des Editors kleidet die Seite in Ihre Farben,
ohne sie für beliebigen Code zu öffnen:

| Einstellung | Was sie tut |
| --- | --- |
| **Logo** | PNG, JPEG oder WebP, bis 256 KiB, neben dem Titel angezeigt. Quadratisch funktioniert am besten. SVG wird abgelehnt: Es kann Skripte tragen, und die Seite wird von derselben Adresse wie die Administration bedient. Die Datei wird anhand ihres Inhalts geprüft, unter `/data/status-pages/` gespeichert, und nur als Bild bedient. |
| **Accent** | Ink (Standard), blue, teal, violet, rose oder amber — die obere Linie, die Links und die Subscribe-Schaltfläche. Jede ist so gewählt, dass sie bei Tag und Nacht lesbar bleibt. |
| **Organisation website** | Ein `http://`- oder `https://`-Link neben dem Titel, als Hostname der Site angezeigt. |
| **Footer text** | Klartext unter der Seite, bis 280 Zeichen: wer sie betreibt, wie man sie erreicht. Zeilenumbrüche werden behalten; nichts wird als HTML oder Markdown interpretiert. |

### Scene

Die Einstellung **Scene** zeichnet eine Stadtsilhouette hinter dem
Banner. Neue Seiten starten mit Paris, und bestehende Seiten ohne
Scene zeigen sie jetzt auch; entfernen Sie das Häkchen für ein
schlichtes Banner, oder aktivieren Sie **Simple mode**, um Scenes
ganz fallen zu lassen. Wählen Sie so viele Sie wollen, in der
gewünschten Reihenfolge. Städte: Venedig, Paris, Tokio, New York,
London, Rom, Sydney, Dubai, San Francisco, Barcelona, Amsterdam,
Istanbul und Rio de Janeiro. Die neuen sieben Weltwunder: Chichén
Itzá, Machu Picchu, die Große Mauer, Petra und der Taj Mahal (Rios
Christus-Statue ist in der Rio-Scene). Die antiken Weltwunder: Gizeh,
Babylon, Ephesus, Olympia, Halikarnassos, Rhodos und Alexandria,
plus die Athener Akropolis. Mit mehr als einer entscheidet
**Rotation**, wann die Seite zur nächsten wechselt: bei jedem Besuch
(Standard), jede Minute, alle zehn Minuten oder jede Stunde. Die
Scenes sind eine geschlossene Menge von Zeichnungen, mit DumbMonit
mitgeliefert — nichts wird hochgeladen oder interpretiert.

Es gibt absichtlich kein benutzerdefiniertes CSS oder HTML: Eine
Statusseite ist öffentlich und lebt auf derselben Origin wie Ihre
Admin-Oberfläche, unter derselben
[Content-Security-Policy](../reference/api.md).

## 2. Was Besucher sehen

- Ein Banner mit der Ein-Sekunden-Antwort, aus den Diensten gelesen:
  **All systems operational**, **Partial outage** (manche down oder
  degraded), **Major outage** (alle down) oder **Scheduled
  maintenance**. Ein offener Vorfall, während jeder Dienst noch
  antwortet, liest sich **Incident in progress**, nach seiner
  Auswirkung getönt.
- Die offenen Ankündigungen, neuestes Update zuerst, mit der
  Zeitleiste der Updates unter einer Falte.
- Jeder Dienst mit seiner Zustandsanzeige (Operational, Degraded,
  Down, Maintenance, No data), seiner **Historienleiste** — ein
  Quadrat pro Tag — seiner Uptime über 24 h, 7 Tage, 30 Tage und (auf
  einer 90-Tage-Seite) 90 Tage, und der Antwortzeit für
  Uptime-Abfragen.
- **Past incidents**, nach Tag, für die letzten 30 Tage.
- **Get updates**: der RSS-Feed, und ein E-Mail-Abonnementformular,
  wenn Sie eines einrichten (siehe
  [E-Mail-Abonnenten](#email-subscribers)).

### Die Historienleiste

Jedes Quadrat ist ein Tag (UTC), nach der Uptime dieses Tages
getönt: volle Farbe bei 99,5 % und höher, gelb ab 95 %, rot darunter.
**Ein Tag ohne jede Messung ist grau**, nicht grün — ein letzte
Woche hinzugefügter Dienst hat 83 graue Tage, nicht 83 perfekte.
Fahren Sie über ein Quadrat, oder fokussieren Sie die Leiste und
bewegen Sie sich mit den Pfeiltasten (Home und End springen zu den
Enden), um Datum, Uptime, die **Ausfallminuten** und die diesen Tag
betreffenden Vorfälle zu lesen. Die Worte sind auch für Screenreader
da: Status wird nie allein durch Farbe mitgeteilt.

Wie der Zustand entschieden wird:

| Zustand | Bedeutung |
| --- | --- |
| Operational | Die letzte Abfrage gelang (Uptime-Abfragen), oder der Monitor meldete sich innerhalb von drei Abfrageperioden. |
| Degraded | Jetzt up, aber mindestens eine Abfrage schlug innerhalb der letzten Stunde fehl. |
| Down | Die letzte Abfrage schlug fehl, der Monitor hörte auf zu melden, oder seine Konfiguration ist fehlerhaft. |
| Maintenance | Ein Wartungsfenster läuft, und der Dienst ist down: erwartet, kein Ausfall. Entweder eine Ankündigung auf dieser Seite, oder ein alarmierendes [Wartungsfenster](../alerting/maintenance.md), das diesen Monitor abdeckt. |
| No data | Noch nie abgefragt, oder deaktiviert. |

Uptime für Abfragen (`http`, `tcp`, `dns`, `ping`, `tls`,
Heartbeats) ist der Anteil erfolgreicher Checks, aus
`dumbmonit_probe_success`; der Ausfall eines Tages ist seine
fehlgeschlagenen Checks mal dem Abfrageintervall. Für andere Monitore
(SNMP, Agent, Proxmox…) ist es der Anteil der Fünf-Minuten-Slots, in
denen der Monitor sich mindestens einmal meldete (`dumbmonit_up`),
gezählt von der ersten Messung — ein gestern hinzugefügter Monitor ist
für die 89 Tage davor nicht "down" — und der Ausfall eines Tages ist
seine stillen Slots.

## 3. Vorfälle und Wartung ankündigen

Auf **Settings → Status pages**, unter der Liste der Seiten, **Incidents
and maintenance** → **New announcement** (auch ++ctrl+k++ →
*Announce an incident*).

- Ein **Incident** hat einen Titel, eine Auswirkung (**minor** zeigt
  die Seite als degraded, **major** als Ausfall) und läuft durch
  *Investigating → Identified → Monitoring → Resolved*. Posten Sie
  Updates im Verlauf: Jedes trägt einen Status und eine Nachricht,
  und wird die neueste Zeile, die Besucher lesen. **Resolve**
  schließt ihn mit einer Abschlussnachricht; er wandert dann für
  30 Tage zu "Past incidents".
- Ein **Maintenance**-Fenster hat einen Start und ein Ende und läuft
  durch *Scheduled → In progress → Completed*. Während es läuft,
  sagt das Banner "Scheduled maintenance", und down gegangene
  Dienste zeigen sich als *Maintenance*.

Ein [in Alerts geplantes Wartungsfenster](../alerting/maintenance.md)
tut dasselbe für den einen abgedeckten Monitor, ohne Ankündigung: Dieser
Dienst liest sich *Maintenance* statt rot, während das Fenster offen
ist, und der Gesamtzustand der Seite sagt *Maintenance*, wenn sonst
nichts down oder degraded ist. Nur Fenster, die ein Monitor benennen,
zeigen sich so; der Name und Kommentar des Fensters bleiben privat.

Eine Ankündigung wird auf einer Seite oder auf **allen Seiten**
angezeigt.

### E-Mail-Abonnenten {#email-subscribers}

Besucher können Ihre Ankündigungen per E-Mail abonnieren, sobald Sie
im Editor unter **Email subscribers** einen **E-Mail-Kanal (SMTP)**
wählen. Der Kanal ist einer, den Sie schon unter
[Notifications](../alerting/notifications.md) konfiguriert haben;
sein Server und Absender werden wiederverwendet, seine eigenen
Empfänger nicht. Ohne einen bietet die Seite nur ihren RSS-Feed.

- **Double Opt-in.** Ein Besucher gibt eine Adresse ein; DumbMonit
  mailt einen Bestätigungslink, und nichts sonst wird gesendet, bis
  er gefolgt wird. Eine nie bestätigte Anfrage wird nach zwei Tagen
  vergessen. Das Formular antwortet mit demselben Satz, ob die
  Adresse neu, ausstehend oder schon abonniert ist, sodass es
  niemandem sagt, wer Ihrer Seite folgt.
- **Was gesendet wird.** Eine E-Mail, wenn eine Ankündigung erstellt
  wird, wenn ein Update gepostet wird, und wenn sich ihr Status
  ändert — für Incidents und Maintenance gleich, auf dieser Seite
  (oder auf jeder Seite, für eine auf allen angezeigte Ankündigung).
- **Abmelden.** Jede E-Mail endet mit einem Abmelde-Link, und trägt
  die Header `List-Unsubscribe` und `List-Unsubscribe-Post`, sodass
  Gmail, Apple Mail und andere ihre eigene Ein-Klick-Schaltfläche
  bieten.
- **Limits.** Der öffentliche Subscribe-Endpunkt akzeptiert fünf
  Requests pro Client-Adresse und sechzig insgesamt pro 15 Minuten,
  und eine ausstehende Adresse wird nicht zweimal innerhalb von zehn
  Minuten gemailt: Die Seite kann nicht zu einer Mail-Kanone
  gemacht werden.
- Der Editor listet Abonnenten (bestätigt oder ausstehend) und
  entfernt jeden davon.

Die Links in diesen E-Mails zeigen auf die
[benutzerdefinierte Domain](#custom-domain) der Seite, wenn sie eine
hat (`https://status.example.com/`). Sonst werden sie aus der
**öffentlichen URL** der
[Benachrichtigungsrichtlinie](../alerting/notifications.md) gebaut
(oder `DUMBMONIT_PUBLIC_URL`); ohne eine, aus der Adresse, die Sie
beim letzten Speichern der Seite nutzten — nie aus dem Request eines
Besuchers.

## 4. Den Link teilen

Die Seite lebt unter `https://ihr-server/s/<address>`. Alles unter
`/s/` und `/api/public/` wird ohne Authentifizierung bedient; der
Rest von DumbMonit bleibt hinter dem Login.

Das **Share**-Panel des Editors baut die unten stehenden Snippets für
Sie, mit einer Vorschau.

### Badges {#badges}

Jede veröffentlichte Seite bedient SVG-Badges — flach, mit weißem
Text auf durchgehender Farbe, lesbar in beiden Themes eines
GitHub-READMEs, hell und dunkel:

| Badge | URL |
| --- | --- |
| Seitenstatus | `/api/public/status/<address>/badge.svg` |
| Seiten-Uptime | `/api/public/status/<address>/uptime.svg?days=30` |
| Seiten-Antwortzeit | `/api/public/status/<address>/response.svg` |
| Dienststatus | `/api/public/status/<address>/components/<service>/badge.svg` |
| Dienst-Uptime | `/api/public/status/<address>/components/<service>/uptime.svg?days=30` |
| Dienst-Antwortzeit | `/api/public/status/<address>/components/<service>/response.svg` |

`<service>` ist das Label des Dienstes in URL-Form: "Public API"
wird zu `public-api` (ein zweites "Public API" wäre
`public-api-2`). Es ist auch der `key` jedes Dienstes im JSON-
Dokument. `days` ist `1`, `7`, `30` (Standard) oder `90`, und nie
mehr als die Historie der Seite: Eine 30-Tage-Seite lehnt ein
90-Tage-Badge ab. Die Uptime und Antwortzeit der Seite sind der
Durchschnitt über ihre Dienste. Ein Dienst ohne jede Messung sagt
**no data**, in Grau.

In einem README:

```markdown
[![Status](https://your-server/api/public/status/home-lab/badge.svg)](https://your-server/s/home-lab)
[![Uptime](https://your-server/api/public/status/home-lab/uptime.svg?days=30)](https://your-server/s/home-lab)
[![API](https://your-server/api/public/status/home-lab/components/public-api/uptime.svg)](https://your-server/s/home-lab)
```

Badges werden eine Minute gecacht (`Cache-Control: public,
max-age=60`), sodass GitHubs Image-Proxy und Ihre Besucher der
Zeitreihendatenbank nichts kosten.

### Embed

`/s/<address>/embed` ist eine kompakte Ansicht für ein Iframe: der
Gesamtzustand und eine Zeile pro Dienst mit seinem Zustand und 30
Tagen Historie, und ein Link zur vollständigen Seite in einem neuen
Tab.

```html
<iframe src="https://your-server/s/home-lab/embed"
        title="Home lab status" width="100%" height="320" style="border:0"></iframe>
```

Fügen Sie `?theme=light` oder `?theme=dark` hinzu, um zur Host-Seite
zu passen, und `?history=0`, um die Balken zu verstecken. Die
Statusseite und ihr Embed sind die einzigen Adressen von DumbMonit,
die eine Drittanbieter-Seite einrahmen darf; alles andere
(die Bestätigungs- und Abmelde-Seiten eingeschlossen) antwortet mit
`frame-ancestors 'none'` und `X-Frame-Options: DENY`.

### Benutzerdefinierte Domain {#custom-domain}

Eine Seite kann ihre eigene Adresse haben, wie `status.example.com`,
wie Uptime Kuma es tut. Tippen Sie den Namen in das Feld **Public
domain** des Editors (das **i** daneben fasst die Schritte zusammen)
und speichern Sie. Wenn ein Request mit diesem Namen im
`Host`-Header DumbMonit erreicht, zeigt DumbMonit die Seite unter
dem Root `/`, und nichts sonst: Unter diesem Namen antworten der
Login-Bildschirm, die Settings, die API, die Endpunkte der Agents,
die Metriken und die anderen Statusseiten alle mit
`404 Not found`, selbst für jemanden Eingeloggten, und kein
Session-Cookie wird gelesen oder gesetzt. Nur die Seite, ihr Embed,
ihre Bestätigungs- und Abmelde-Seiten, ihre Badges, ihr Feed und
JSON-Dokument, und die eigenen Dateien der Oberfläche (`/_app/`,
Icons) werden bedient.

Der Name wird in Kleinbuchstaben gespeichert, ohne `https://` oder
einen abschließenden Schrägstrich; ein Port, ein Pfad oder eine
IP-Adresse wird abgelehnt. Zwei Seiten können sich keinen Namen
teilen, und die Adresse, die Sie nutzen, um DumbMonit selbst zu
erreichen (oder seine öffentliche URL), kann keine Domain einer
Seite sein — sie zu speichern würde Sie aussperren. Leeren Sie das
Feld, um die Domain zu entfernen; die Seite bleibt in jedem Fall
unter `/s/<address>`.

So richten Sie sie ein:

1. **DNS.** Erstellen Sie den Eintrag für den Namen (oder den
   öffentlichen Hostnamen eines Cloudflare Tunnel), der auf Ihren
   Reverse Proxy zeigt.
2. **Proxy.** Senden Sie den Namen an DumbMonits Adresse, zum
   Beispiel `http://dumbmonit:8080`, **ohne den Pfad umzuschreiben**:
   Der Proxy leitet `/` weiter, nicht `/s/<address>`.
3. **Host-Header.** Geben Sie den ursprünglichen `Host`-Header
   weiter: So unterscheidet DumbMonit die Domain der Seite von
   seiner eigenen Adresse. DumbMonit liest nur `Host`;
   `X-Forwarded-Host` zählt nur von einem in
   `DUMBMONIT_TRUSTED_PROXIES` gelisteten Proxy.
4. **HTTPS** wird vom Proxy gehandhabt. DumbMonit verlinkt in
   Abonnenten-E-Mails, dem RSS-Feed und dem **Share**-Panel des
   Editors auf `https://<domain>/`.

Traefik (Docker-Labels) — der `Host`-Header wird standardmäßig
weitergegeben:

```yaml
labels:
  - traefik.enable=true
  - traefik.http.routers.status.rule=Host(`status.example.com`)
  - traefik.http.routers.status.entrypoints=websecure
  - traefik.http.routers.status.tls.certresolver=letsencrypt
  - traefik.http.services.status.loadbalancer.server.port=8080
```

Nginx Proxy Manager — fügen Sie einen **Proxy Host** mit der Domain
`status.example.com`, Schema `http`, Forward-Hostname `dumbmonit`,
Port `8080`, und einem SSL-Zertifikat hinzu. Nichts sonst: Er gibt
`Host` standardmäßig weiter. Lassen Sie **Custom locations** leer.

Cloudflare Tunnel — fügen Sie einen **Public hostname**
`status.example.com` mit dem Service `http://dumbmonit:8080` und
ohne Pfad hinzu. Der Tunnel behält den `Host` des Besuchers; lassen
Sie **HTTP Host Header** in den erweiterten Einstellungen leer.

Caddy — `reverse_proxy` gibt `Host` weiter und Caddy holt das
Zertifikat:

```caddyfile
status.example.com {
    reverse_proxy dumbmonit:8080
}
```

Nginx — `proxy_pass` sendet den Namen des Upstreams, sofern nicht
anders gesagt, setzen Sie also den Header:

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

**Fehlerbehebung.**

- *Die Domain zeigt DumbMonits Login-Bildschirm oder Dashboard*: Der
  `Host`-Header hat DumbMonit nicht erreicht — der Proxy hat ihn
  durch den Namen des Upstreams ersetzt (`dumbmonit:8080`). Fügen
  Sie `proxy_set_header Host $host;` hinzu (Nginx), entfernen Sie
  einen benutzerdefinierten **HTTP Host Header** (Cloudflare
  Tunnel), oder entfernen Sie eine `Host`-Rewrite-Middleware
  (Traefik). Nichts Privates wird exponiert: Das Dashboard fragt
  weiterhin nach einem Passwort.
- *Ein blankes `Not found` auf der Domain*: Der Proxy schreibt den
  Pfad um (er sendet `/s/<address>` oder fügt ein Präfix hinzu),
  oder die Domain im Editor passt nicht genau zum Namen im Browser.
  Leiten Sie den Pfad unverändert weiter.
- *Die Seite sagt, sie existiere nicht*: Die Seite ist noch ein
  Entwurf — veröffentlichen Sie sie.
- *Sie können DumbMonits Oberfläche nicht mehr erreichen*: Sie
  browsen über die Domain der Seite. Nutzen Sie DumbMonits eigene
  Adresse, oder leeren Sie die **Public domain** der Seite.

### Hinter einem Reverse Proxy

Eine [benutzerdefinierte Domain](#custom-domain) ist der einfachste
Weg, nur die Statusseite zu exponieren. Bevorzugen Sie es, den Pfad
`/s/<address>` auf einem öffentlichen Namen zu behalten, filtern Sie
stattdessen die Pfade im Proxy. Mit Caddy zum Beispiel:

```caddyfile
status.example.com {
    @public path /s/* /api/public/* /_app/* /favicon.svg
    reverse_proxy @public dumbmonit:8080
    respond 404
}
```

Mit nginx:

```nginx
location ~ ^/(s/|api/public/|_app/|favicon\.svg) {
    proxy_pass http://dumbmonit:8080;
    proxy_set_header Host $host;
    proxy_set_header X-Forwarded-Proto $scheme;
}
location / { return 404; }
```

`/_app/` trägt die Skripte und Stile der Oberfläche: Die Seite
braucht sie zum Rendern. Leiten Sie `X-Forwarded-Proto` und
`X-Forwarded-Host` weiter, damit der RSS-Feed auf die richtige
Adresse verlinkt, und setzen Sie die öffentliche URL (siehe oben),
damit das Abonnenten-E-Mails auch tun.

### Feed und Dokument

- `GET /api/public/status/<address>/rss` — ein RSS-Feed von
  Incidents und Wartungsfenstern, ein Eintrag pro Ankündigung mit
  ihrem neuesten Update.
- `GET /api/public/status/<address>` — das JSON-Dokument, aus dem
  die Seite gebaut wird, falls Sie Ihre eigene bauen wollen: der
  Titel, die Beschreibung und das Look der Seite, der
  Gesamtzustand, der `key`, `label`, `state` jedes Dienstes, Uptime
  (`uptime_24h`, `uptime_7d`, `uptime_30d`, `uptime_90d`),
  `latency_ms` und `history` (`date`, `uptime_pct`,
  `down_minutes`, `incidents` pro Tag), und die Ankündigungen.

Jede Website darf dieses Dokument vom Browser aus lesen (es wird mit
`Access-Control-Allow-Origin: *` und ohne Cookies gesendet). `GET
/api/public/status/<address>/banner.js` ist ein fertiges Skript, das
es in ein Banner verwandelt: siehe
[Website-Banner](../integrations/website-banner.md).

Das Dokument wird 30 Sekunden lang auf dem Server gecacht; die
Badges, der Feed und das Embed lesen alle dasselbe gecachte
Dokument, daher kann keines von ihnen mehr sagen als die Seite.
