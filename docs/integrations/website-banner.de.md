# Website-Banner {#website-banner}

Zeigen Sie auf Ihrer eigenen Website ein Banner an, wenn Dienste ausgefallen sind („2 Dienste
sind ausgefallen“), gespeist aus einer veröffentlichten [Statusseite](../using/status-pages.md).
Wenn alles in Ordnung ist, bleibt das Banner verborgen, es sei denn, Sie verlangen es
ausdrücklich („Alle Systeme laufen“).

Drei Wege, es einzubinden, vom einfachsten bis zum am besten kontrollierbaren. Kopierfertige
Dateien liegen im Ordner `integrations/` des Repositorys.

## HTML-Snippet {#html-snippet}

Fügen Sie dies vor `</body>` ein, mit Ihrer Serveradresse und der Adresse der Seite:

```html
<script async
  src="https://monit.example.com/api/public/status/my-page/banner.js"
  data-position="top" data-show="issues" data-lang="en"></script>
```

| Attribut | Werte | Standard |
|---|---|---|
| `data-position` | `top`, `bottom` | `top` |
| `data-show` | `issues` (nur bei einem Problem), `always` | `issues` |
| `data-lang` | `en`, `fr`, `de`, `es`, `it`, `pt`, `pt-BR`, `ru`, `zh-Hans` | Sprache der Seite, sonst `en` |

Das Skript hat keine Abhängigkeiten, zeichnet innerhalb eines Shadow DOM, sodass Ihre Stile es
nicht beeinflussen, folgt der hellen oder dunklen Präferenz des Besuchers und lässt sich
schließen (bis der Browser-Tab geschlossen wird). Es liest das öffentliche JSON der Seite, das
der Server jeder Website zum Abruf erlaubt (`Access-Control-Allow-Origin: *`, keine Cookies).
Nur das öffentliche Dokument der Statusseite wird auf diese Weise geöffnet; der Rest der API
behält seine Regeln.

## WordPress {#wordpress}

Kopieren Sie `integrations/wordpress/dumbmonit-status-banner` nach `wp-content/plugins/`,
aktivieren Sie es, öffnen Sie dann **Einstellungen > DumbMonit Banner** und tragen Sie die URL
der Statusseite (`https://monit.example.com/api/public/status/my-page`), die Position und den
Anzeigemodus ein. Das Plugin ruft das JSON serverseitig ab, hält es eine Minute im Cache und
maskiert alles, was es ausgibt.

Der DumbMonit-Server darf eine private Adresse (LAN) haben: Das Plugin ruft immer nur genau die
gespeicherte URL ab (geprüft gegen das Muster `/api/public/status/<address>`, Weiterleitungen
deaktiviert). Ist die Adresse ungültig, zeigt die Einstellungsseite einen Fehler und behält den
bisherigen Wert. Lässt sich die Statusseite nicht abrufen, erscheint im WordPress-Admin (und
beim Speichern) eine Warnung, statt stillschweigend zu scheitern.

Der Bannertext ist übersetzbar (Textdomain `dumbmonit-status-banner`, Vorlage in
`languages/`); er folgt der Sprache der WordPress-Website. Das öffentliche Status-JSON enthält
keine Sprache und kann daher keine auswählen.

## PHP-Include {#php-include}

Für jede PHP-Website, ohne Framework:

```php
require __DIR__ . '/banner.php';
dumbmonit_banner('https://monit.example.com/api/public/status/my-page',
    ['position' => 'top', 'show' => 'issues', 'ttl' => 60]);
```

Der Helfer akzeptiert nur `http`- und `https`-URLs, verwendet ein Timeout von 3 Sekunden,
folgt keinen Weiterleitungen, führt einen Dateicache im temporären Verzeichnis und gibt nichts
aus, wenn der Server nicht erreichbar ist (der Fehler wird ins PHP-Fehlerlog geschrieben).
Die Texte sind standardmäßig englisch; übergeben Sie `'lang' => 'fr'` oder ein Array
`'strings'`, um sie zu ändern. Eine private Adresse (LAN) funktioniert. Siehe
`integrations/php/example.php`.
