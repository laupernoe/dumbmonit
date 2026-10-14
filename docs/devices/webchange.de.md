# Website-Änderungen

Beobachten Sie den *Inhalt* einer Webseite, oder einer ganzen Site, und
lassen Sie sich benachrichtigen, wenn er sich ändert: eine Preis- oder
AGB-Seite, eine Dokumentations-Site, der Wartungshinweis eines
Lieferanten, eine öffentliche Ankündigungsseite, oder Ihre eigene Site
nach einem Deployment. Jede Änderung wird mit dem Text davor und
danach gehalten, einem Zeilen-Diff und, wenn ein Browser verfügbar
ist, einem Screenshot jeder Version.

Was verglichen wird, ist der **sichtbare Text** jeder Seite, nicht ihr
HTML: Skripte, Stile, Kommentare, `noscript`-, `svg`- und
`template`-Blöcke werden verworfen, Leerraum wird zusammengefasst, und
jeder Block (Absatz, Überschrift, Listenelement, Tabellenzeile) wird
zu einer Zeile. Ein neuer Build derselben Seite, ein umgeordnetes
Attribut oder ein frischer Cache-Busting-Asset-Name ändert nichts; ein
umformulierter Satz schon.

### Welcher Teil der Seite

**Navigation, Kopfbereich, Fußbereich und Seitenleisten** der Site bleiben außen
vor: Sie sind auf jeder Seite gleich, sodass ein neuer Menüeintrag sonst in einer
einzigen Prüfung jede Seite als geändert markieren würde. Sie folgen den Rollen,
die HTML ihnen gibt: jedes `nav`, ein `header` oder `footer`, der nicht in einem
`article`, `aside`, `main`, `nav` oder `section` steht (der Kopf eines Artikels
bleibt erhalten), ein `aside` außerhalb von `article` und `main`, und jedes
Element mit der Rolle `navigation`, `banner`, `contentinfo` oder `search`. Ihren
Links wird weiterhin gefolgt, um die Seiten einer Site zu finden. **Compare
menus, header and footer** schaltet das ab.

**Watched area** beschränkt den Vergleich auf bestimmte Elemente, zum Beispiel
`main` oder `#content`; eine Seite, auf der keines davon gefunden wird, wird
ganz gelesen statt als leer. **Excluded areas** lässt weitere Elemente aus: ein
Cookie-Banner, ein Kasten "neueste Beiträge", ein Werbeplatz. Beide nehmen
einfache Selektoren, durch Kommas getrennt: ein Tag (`main`), eine ID
(`#content`), Klassen (`.cookie-banner`), Attribute (`[role=alert]`,
`[data-ad]`) oder mehrere zusammen (`div.notice[role=alert]`). Nachfahren
(`main p`) und andere Kombinatoren werden nicht unterstützt.

## Seite oder Site

**Page** (der Standard) liest nur die Adresse des Geräts. Es kann auch
ein einfaches Text- oder JSON-Dokument sein, Zeile für Zeile
verglichen.

**Site** startet von der Adresse und folgt jeder Seite unter demselben
Ordner (`https://example.com/docs/intro.html` deckt alles unter
`/docs/` ab; **Path prefix** wählt einen anderen Ordner). Seiten werden
gefunden:

1. in der Sitemap der Site: den von `robots.txt` angekündigten
   (`Sitemap:`-Zeilen), oder `/sitemap.xml`, Sitemap-Indizes
   eingeschlossen (eine Ebene);
2. falls das nicht klappt, oder wenn die Sitemap nichts unter dem
   Ordner listet, durch Folgen der Links jeder Seite, in Breitensuche.

Nur Seiten derselben Origin (Schema, Host und Port) werden gefolgt. Leitet
die Startseite auf einen anderen Host derselben Site um (`example.com` auf
`www.example.com`, `http` auf `https` oder eine Subdomain wie
`docs.example.com`), gilt die Site als dort beheimatet, und Links zum ersten
Host werden auf dem zweiten gelesen. Leitet sie auf eine ganz andere Site um
(eine Anmeldeseite wie `login.microsoftonline.com`, eine Drittanbieter-Domain),
wird diese Site nicht gecrawlt: Nur die Startseite wird gelesen, und keine Seite
wird als entfernt gemeldet. Links zu Bildern, Stylesheets, Skripten, PDFs,
Archiven und anderen Dateien werden anhand ihrer Erweiterung übersprungen, und
jede Antwort, die kein HTML ist, wird ignoriert. Fragmente (`#section`),
Tracking-Parameter (`utm_…`, `fbclid`, `gclid` und ähnliche) und leere Queries
werden entfernt, sodass eine Seite eine Adresse ist; **Ignore query strings**
verwirft den ganzen `?…`-Teil. `robots.txt` wird für so gefundene Seiten beachtet: die Gruppe,
die `DumbMonit` benennt, oder sonst die Gruppe `*`, mit `Allow`,
`Disallow`, `*` und `$`.

Das Crawling ist höflich: höchstens zwei Requests gleichzeitig, eine
halbe Sekunde zwischen zwei Batches, der `User-Agent`
`DumbMonit/<version>`, ein 15-Sekunden-Timeout pro Request und
höchstens 5 MB pro Seite gelesen. Es stoppt bei **Max pages**
(standardmäßig 25, höchstens 200) oder nach zehn Minuten. Hat **Max pages** es
gestoppt, sagt die Geräteseite, wie viele Seiten gefunden wurden, damit der
nicht beobachtete Teil der Site nie eine Überraschung ist. **User agent** ersetzt
`DumbMonit/<version>` für eine Site, die Roboter abweist.

## Wie eine Prüfung funktioniert

Das Gerät wird wie jedes andere abgefragt (standardmäßig jede Minute),
aber die Seiten werden nur alle **Check every**-Minuten gelesen
(standardmäßig 60, von 5 bis 10080). Dazwischen meldet die Abfrage nur
den Zustand der letzten Prüfung: Die Site wird nicht jede Minute
getroffen. Eine Prüfung läuft im Hintergrund; zwei Prüfungen desselben
Geräts überlappen sich nie, und **Check now** auf der Geräteseite
startet sofort eine.

Die erste Prüfung verzeichnet nur die **Referenz**: Jede Seite wird
gelesen und behalten, nichts wird gemeldet. Von da an, für jede Seite:

| Situation | Was behalten wird |
|---|---|
| Gleicher Text wie letztes Mal (nach **Ignore lines**) | Nichts Neues; die Seite wird als geprüft markiert. |
| Anderer Text | Ein neuer Snapshot und ein **changed**-Event, mit der Anzahl hinzugefügter und entfernter Zeilen. |
| Eine nie zuvor gesehene Seite, oder erneut gesehen, nachdem sie weg war | Ein Snapshot und ein **new page**-Event. |
| Die Seite antwortet mit 404 oder 410 | Ein **removed page**-Event, der letzte Snapshot als "before". |
| Site-Scope: ein vollständiger Crawl erreicht eine bekannte Seite nicht mehr | Auch ein **removed page**-Event ("No longer linked"). Ein durch **Max pages** oder sein Zeitlimit gestoppter Crawl schließt nie, dass eine Seite weg ist. |
| Jeder andere Fehler auf einer Seite (Timeout, 403, 500 auf einer Unterseite) | Der Fehler wird auf der Seite angezeigt; kein Event. |

Das Ändern der Adresse, des Scope, des Path-Präfixes, von **Max pages**, der
Bereiche oder von **Ignore lines** macht die nächste Prüfung zu einer neuen
Referenz, statt jede Seite als geändert zu melden (oder, bei höherem **Max
pages**, jede neu hinzukommende Seite als neu). Ein Update von DumbMonit, das
Seiten anders liest, bewirkt einmalig dasselbe.

Findet eine Prüfung **dieselbe Änderung auf mehreren Seiten** (dieselben Zeilen
entfernt und hinzugefügt, etwa ein Banner), zeigt die Geräteseite sie einmal,
mit den Seiten, auf denen sie vorgenommen wurde.

### Ignore lines

Zeilen, die sich bei jedem Besuch ändern — eine Uhr, ein
Besucherzähler, "updated 5 minutes ago", ein Zufallszitat — würden bei
jeder Prüfung gemeldet. **Ignore lines** hält reguläre Ausdrücke
(Rust-`regex`-Syntax), einen pro Zeile oder mit `|` verbunden; jede
Textzeile, auf die einer davon passt, wird vor dem Vergleich
ausgelassen. Zum Beispiel:

```
Updated \d+ minutes ago|Visitors: \d+|© \d{4}
```

### Nicht erreichbare Startseite

Wenn die Startseite selbst nicht gelesen werden kann (keine Antwort,
Verbindung abgelehnt, Timeout, HTTP 5xx), ist das Gerät
**unreachable**: Die Abfrage schlägt fehl, und die Regel "Device
unreachable" feuert wie für jedes Gerät. Die Startseite wird dann bei
jeder Abfrage erneut versucht, sodass ihre Rückkehr innerhalb einer
Minute gesehen wird. Eine Startseite, die den Zugriff verweigert (401,
403), wird als Konfigurationsproblem gemeldet, kein Ausfall. Fehler
auf anderen Seiten der Site werden nur auf diesen Seiten angezeigt.

Adressen, die nur der DumbMonit-Host selbst erreichen kann (Loopback,
Link-Local, der Cloud-Metadata-Dienst), werden abgelehnt, sofern
**Allow loopback and link-local targets** nicht aktiviert ist; private
LAN-Adressen sind immer erlaubt.

## Alarme

Die eingebaute Regel **Website changed** (`webchange_detected`, Info)
feuert nach einer Prüfung, die mindestens eine Änderung fand (changed,
new oder removed page), und die Benachrichtigung nennt die Seiten: "/pricing,
/contact und 2 weitere". Solange sie feuert, wird jede weitere Prüfung, die
Änderungen findet, erneut gemeldet, mit ihren eigenen Seiten, auch auf denselben
Seiten; der Alarm löst sich bei der ersten Prüfung, die nichts Neues findet. Es
bleibt die ganze Zeit ein Alarm pro Gerät, sodass Bestätigen oder Snoozen am
Alarm auch die folgenden Prüfungen abdeckt. Sie liest `dumbmonit_webchange_last_check_changes`;
weisen Sie sie wie jede andere Regel einem Kanal zu, oder heben Sie
ihren Schweregrad an, unter **Alerts → Rules**. Siehe
[Eingebaute Regeln](../alerting/rules.md#website-changes).

| Metrik | Was |
|---|---|
| `dumbmonit_webchange_pages` | Gefolgte Seiten (entfernte Seiten ausgeschlossen). |
| `dumbmonit_webchange_pages_failed` | Gefolgte Seiten, deren letztes Lesen fehlschlug. |
| `dumbmonit_webchange_last_check_changes` | Von der letzten Prüfung gefundene Änderungen. |

Die Metriken erscheinen, sobald die erste Prüfung erledigt ist.

## Screenshots

Das DumbMonit-Image hat keinen Browser: Screenshots stammen von einem
optionalen Headless-Chromium, über das Chrome DevTools Protocol
gesteuert. Fügen Sie in `docker-compose.yml` neben DumbMonit einen
`browser`-Dienst hinzu und richten Sie den Server darauf:

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

Führen Sie dann `docker compose up -d` aus. Nichts veröffentlicht den Port des
Browsers: Nur der DumbMonit-Server spricht mit ihm, über das Compose-Netzwerk.

`DUMBMONIT_BROWSER_URL` richtet DumbMonit auf den Browser (jedes mit
`--remote-debugging-port` gestartete Chromium, zum Beispiel
`chromedp/headless-shell`; der Server liest `/json/version` und
spricht mit der Adresse, zu der der Hostname aufgelöst wird).

Ein Screenshot wird nur aufgenommen, wenn eine Seite einen neuen
Snapshot erhält (die Referenz, und jede Änderung), nie bei einer
unveränderten Prüfung: ein 1366 × 900-Fenster, die ganze Seite bis zu
5000 Pixel, als JPEG. Die Option **Screenshots** schaltet sie für ein
Gerät aus. Ohne Browser, oder wenn er nicht antwortet, funktioniert
der Textvergleich genauso, und die Snapshots haben einfach kein Bild;
die Geräteseite sagt das.

Der Browser lädt die Seite, wie ein Besucher es täte, mit ihren
Skripten: Was er zeigt, kann sich vom Text unterscheiden, den
DumbMonit verglichen hat, was das vom Server gesendete HTML ist.

## Speicherung

Der Text jedes Snapshots wird in der Datenbank gehalten (höchstens
1 MB pro Seite); Screenshots sind Dateien unter
`/data/webchange/<device id>/`. Pro Gerät werden die letzten 500
Änderungen behalten; ein Snapshot wird behalten, während er der
neueste einer Seite ist oder eine behaltene Änderung auf ihn verweist,
und alles andere, Dateien eingeschlossen, wird nach jeder Prüfung
entfernt. Das Löschen des Geräts entfernt alles davon.

## Eine Website auf Änderungen beobachten

Die Schritte unten sind die, die der Hinweis neben dem Formular zeigt.

1. Fügen Sie in die Adresse die zu beobachtende Seite ein, mit http://
   oder https://. Um eine ganze Site oder einen ihrer Abschnitte zu
   beobachten, fügen Sie ihre Startseite ein und setzen Sie "Scope"
   auf "site": jede Seite unter demselben Ordner wird gefolgt, bis zu
   "Max pages".
2. Zeilen, die sich bei jedem Besuch ändern (eine Uhr, ein
   Besucherzähler, "updated 5 minutes ago"), würden jedes Mal gemeldet:
   Listen Sie sie in "Ignore lines" als reguläre Ausdrücke.
3. Die erste Prüfung verzeichnet nur die Referenz, nichts wird
   gemeldet. Von da an vergleicht jede Prüfung den sichtbaren Text
   Seite für Seite und behält eine Vorher/Nachher-Kopie jeder
   Änderung, auf der Geräteseite angezeigt; der Alarm "Website
   changed" sagt es Ihnen.
4. Für einen Screenshot jeder Version fügen Sie Ihrer Compose-Datei einen
   Headless-Browser-Dienst hinzu; die genauen Zeilen stehen in der Dokumentation
   (Advanced options, "Screenshots of website changes").
   Siehe [Screenshots von Website-Änderungen](../install/docker.md#website-screenshots).

!!! warning
    Nur der vom Server gesendete Text wird verglichen: Eine komplett
    von JavaScript gezeichnete Seite zeigt der Prüfung wenig oder
    nichts. Seiten hinter einem Login sind ebenfalls nicht erreichbar.

Credentials: keine. Prüfungen laufen immer auf dem DumbMonit-Server:
Eine Website-Beobachtung kann nicht über einen Relay-Agent laufen.

## Optionen

| Option | Standard | Bedeutung |
|---|---|---|
| Scope (`scope`) | `page` | `page`: nur die Adresse. `site`: jede Seite unter demselben Ordner. |
| Max pages (`max_pages`) | `25` | Site-Scope: höchstens pro Prüfung gelesene Seiten, 1 bis 200. |
| Path prefix (`path_prefix`) | Ordner der Adresse | Site-Scope: nur Seiten, deren Pfad damit beginnt (`/docs/`). |
| Ignore lines (`ignore`) | — | Reguläre Ausdrücke; passende Textzeilen werden nicht verglichen. |
| Watched area (`content_selector`) | — | Einfache Selektoren; nur der Text darin wird verglichen. |
| Excluded areas (`exclude_selector`) | — | Einfache Selektoren; ihr Text wird nicht verglichen. |
| Compare menus, header and footer (`keep_page_chrome`) | `false` | Navigation, Kopf, Fuß und Seitenleisten der Site ebenfalls vergleichen. |
| Ignore query strings (`ignore_query`) | `false` | Site-Scope: Adressen, die sich nur im `?…`-Teil unterscheiden, sind eine Seite. |
| User agent (`user_agent`) | — | Wird statt `DumbMonit/<version>` gesendet. |
| Screenshots (`screenshots`) | `true` | Einen Screenshot jeder neuen Version behalten, wenn ein Browser konfiguriert ist. |
| Check every (`check_interval_minutes`) | `60` | Minuten zwischen zwei Prüfungen, 5 bis 10080. |
| Accept an unverifiable certificate (`insecure_tls`) | `false` | Für eine eigene Site hinter einem selbstsignierten Zertifikat. |
| Allow loopback and link-local targets (`allow_private_targets`) | `false` | Eine vom DumbMonit-Host selbst bediente Site erreichen. |
