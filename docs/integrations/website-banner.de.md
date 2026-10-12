# Website-Banner

Zeigen Sie auf Ihrer eigenen Website ein Banner, wenn Dienste ausfallen
("2 services are down"), gesteuert von einer veröffentlichten
[Statusseite](../using/status-pages.md). Wenn alles in Ordnung ist, bleibt
das Banner verborgen, außer Sie verlangen, es immer anzuzeigen ("All
systems operational").

Drei Wege, es hinzuzufügen, vom einfachsten zum am meisten kontrollierten.
Fertige Dateien liegen im Ordner `integrations/` des Repositorys.

## HTML-Snippet

Fügen Sie dies vor `</body>` ein, mit Ihrer Serveradresse und der Adresse
der Seite:

```html
<script async
  src="https://monit.example.com/api/public/status/my-page/banner.js"
  data-position="top" data-show="issues" data-lang="en"></script>
```

| Attribut | Werte | Standard |
|---|---|---|
| `data-position` | `top`, `bottom` | `top` |
| `data-show` | `issues` (nur wenn etwas nicht stimmt), `always` | `issues` |
| `data-lang` | `en`, `fr`, `de`, `es`, `it`, `pt`, `pt-BR`, `ru`, `zh-Hans` | Sprache der Seite, sonst `en` |

Das Skript hat keine Abhängigkeiten, zeichnet innerhalb eines Shadow DOM,
sodass Ihre Stile es nicht berühren, folgt der Hell/Dunkel-Präferenz der
Besucherin oder des Besuchers und kann weggeklickt werden (bis der
Browser-Tab geschlossen wird). Es liest das öffentliche JSON der Seite,
das der Server jeder Website zum Abruf erlaubt
(`Access-Control-Allow-Origin: *`, keine Cookies). Nur das öffentliche
Dokument der Statusseite wird so geöffnet; der Rest der API behält seine
Regeln.

## WordPress

Kopieren Sie `integrations/wordpress/dumbmonit-status-banner` nach
`wp-content/plugins/`, aktivieren Sie es, öffnen Sie dann
**Settings > DumbMonit Banner** und geben Sie die URL der Statusseite ein
(`https://monit.example.com/api/public/status/my-page`), die Position und
den Anzeigemodus. Das Plugin ruft das JSON auf dem Server ab, cacht es
eine Minute lang und escaped alles, was es ausgibt. WordPress verweigert
Requests an private Adressen, daher muss der DumbMonit-Server auf einer
öffentlichen Adresse erreichbar sein.

## PHP-Include

Für jede PHP-Site, ohne Framework:

```php
require __DIR__ . '/banner.php';
dumbmonit_banner('https://monit.example.com/api/public/status/my-page',
    ['position' => 'top', 'show' => 'issues', 'ttl' => 60]);
```

Der Helfer akzeptiert nur `http`- und `https`-URLs, nutzt ein
3-Sekunden-Timeout, folgt keinen Weiterleitungen, führt einen
Datei-Cache im temporären Verzeichnis und gibt nichts aus, wenn der
Server nicht erreicht werden kann. Siehe `integrations/php/example.php`.
