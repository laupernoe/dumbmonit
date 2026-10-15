# Website-Zustand & SEO {#website-health-seo}

Überwachen Sie Websites in großer Zahl, ein Gerät pro Website: Lassen Sie sich melden, wenn eine
Website **ausfällt** (Fehler, nicht erreichbar, ungültiges Zertifikat, Weiterleitungsschleife),
wenn ihre Seite **leer wird**, wenn sie **aus den Suchmaschinen fällt** (`noindex`, `robots.txt`
sperrt alles) und wenn sich die von Google angezeigten Metadaten **ändern** (Titel,
Beschreibung, kanonische URL, endgültige Adresse). Jede Website wird außerdem nach der
Plattform eingeordnet, auf der sie läuft (WordPress, Shopify, Wix, Next.js …).

Für den Inhalt einer Seite statt ihres Zustands siehe [Website-Änderungen](webchange.md).

## Einrichtung {#set-it-up}

1. Fügen Sie in die Adresse die zu prüfende Seite ein, meist die Startseite, mit oder ohne https://. Weiterleitungen werden verfolgt: http zu https und example.com zu www.example.com sind normal, nur eine Weiterleitungsschleife oder ein Umzug auf eine andere Adresse wird gemeldet.
2. Tragen Sie optional unter „Erwarteter Text“ einen Satz ein, den die Seite immer zeigen muss, und passen Sie „Leere Seite unter“ an, wenn die Seite absichtlich kurz ist.
3. Die erste Prüfung hält die Referenz fest: endgültige Adresse, Titel, Beschreibung, kanonische URL, Indexierungsregeln. Sie werden standardmäßig alle 6 Stunden erneut verglichen („Auf Änderungen prüfen alle“), während Verfügbarkeit bei jedem Abfrageintervall geprüft wird. Eine Änderung wird erst gemeldet, wenn zwei Prüfungen in Folge sie gesehen haben, sodass A/B-Tests und unzuverlässige Caches ruhig bleiben.
4. Um viele Websites auf einmal hinzuzufügen, importieren Sie aus der Geräteliste eine CSV-Datei: Jede Website wird unter `Websites/<platform>` (WordPress, Shopify …) abgelegt, sofern Sie keinen Ordner angeben.

!!! warning
    Gelesen wird nur, was der Server sendet: Eine vollständig per JavaScript gezeichnete Seite sieht hier leer aus. Setzen Sie „Leere Seite unter“ für eine solche Website auf 0.

## Was jede Prüfung tut {#what-each-check-does}

Bei jedem Abfrageintervall (standardmäßig 60 s) wird die Seite einmal gelesen:

- **Weiterleitungen werden verfolgt**, eine nach der anderen, bis zu **Max. Weiterleitungen**
  (standardmäßig 10), und die Kette wird festgehalten: jede Adresse und ihr Status. Der Wechsel
  von `http://` zu `https://` oder von `example.com` zu `www.example.com` ist das, was die
  meisten Websites tun, und nie ein Fehler. Eine Weiterleitung, die zu einer bereits gesehenen
  Adresse zurückkehrt, ist eine **Schleife**, und mehr Weiterleitungen als erlaubt sind ebenfalls
  ein Fehler: Beides lässt die Prüfung mit der Kette in der Meldung fehlschlagen, wie bei einer
  Website, die nicht antwortet.
- **Die endgültige Antwort**: ihr HTTP-Status, die Gesamtzeit (Weiterleitungen eingeschlossen)
  und das TLS-Zertifikat, wie ein Browser es prüft. Ein abgelehntes Zertifikat lässt die Prüfung
  fehlschlagen.
- **Der sichtbare Text** der Seite, wie bei [Website-Änderungen](webchange.md): Skripte, Stile
  und verborgene Blöcke bleiben außen vor. Unter **Leere Seite unter** Zeichen (standardmäßig 20,
  damit kurze legitime Seiten durchgehen) gilt die Seite als leer. Mit **Erwartetem Text** muss
  dieser Satz darin vorkommen (Groß-/Kleinschreibung und Abstände werden ignoriert).
- **Die SEO-Metadaten** der endgültigen Seite: `<title>`, Meta-Description, kanonische URL,
  Robots-Meta-Tags und `X-Robots-Tag`-Header (`noindex`, `nofollow`, `none`), `og:title`,
  `og:description`, `<html lang>`, die Anzahl der `<h1>` und der `hreflang`-Alternativen. Die
  Indexierungsdirektiven und Zahlen werden bei jeder Prüfung gemeldet; der Vergleich der
  Metadaten mit der Referenz geschieht nur bei Änderungsprüfungen, siehe
  [unten](#changes-confirmed-twice).
- **Bei Änderungsprüfungen, höchstens einmal pro Stunde**, die `robots.txt` der endgültigen
  Website: Verbietet sie Googlebot (oder `*`) das Crawlen von `/`? Und die Sitemap, die sie
  ankündigt, oder `/sitemap.xml`: Antwortet sie und lässt sich als Sitemap lesen?

Der User-Agent ist `DumbMonit/<version>`, sofern **User-Agent** nichts anderes angibt.

## Änderungen, zweifach bestätigt {#changes-confirmed-twice}

Die endgültige Adresse und die Metadaten werden mit einer Referenz verglichen, aber nicht bei
jeder Prüfung: nur bei **Änderungsprüfungen**, alle **Auf Änderungen prüfen alle** Minuten (360,
also standardmäßig 6 Stunden; `DUMBMONIT_WEBSITE_CHANGE_CHECK_MINUTES` setzt den serverweiten
Standard, 0 vergleicht bei jeder Prüfung). Verfügbarkeit, leere Seite und erwarteter Text werden
weiterhin bei jedem Abfrageintervall gelesen.

Ein bei einer Änderungsprüfung gesehener abweichender Wert ist nur **ausstehend**; die allernächste
Prüfung (ein Abfrageintervall später, nicht 6 Stunden später) betrachtet ihn erneut, und er wird
zu einer Änderung, wenn auch diese Prüfung ihn sieht. Eine Änderung wird daher höchstens einen
Änderungszeitraum plus ein Abfrageintervall nach ihrem Auftreten gemeldet. Eine in zwei Versionen
ausgelieferte Seite (ein A/B-Test), ein zögernder Cache oder eine abgeschnittene Antwort bleiben
somit ruhig. Werte werden unter Ignorieren von Abständen und Groß-/Kleinschreibung verglichen;
Adressen ignorieren das Schema, ein führendes `www.`, einen abschließenden `/` und, mit
**Query-Strings ignorieren** (standardmäßig an), URL-Parameter wie `?utm_source=…`.

Eine bestätigte Änderung bleibt im Verlauf des Geräts erhalten (Feld, vorher, nachher, Datum),
und ihr Gauge (`dumbmonit_website_title_changed` …) bleibt 24 Stunden auf 1. Die erste Prüfung
hält nur die Referenz fest. Das Ändern der Adresse oder von **Query-Strings ignorieren** beginnt
eine neue Referenz, statt eine Änderung zu melden.

## Optionen {#options}

| Option | Standard | Bedeutung |
|---|---|---|
| Erwarteter Text | leer | Ein Satz, den die Seite zeigen muss. |
| Leere Seite unter (Zeichen) | 20 | Kürzerer sichtbarer Text gilt als leere Seite; 0 schaltet die Prüfung aus. |
| Max. Weiterleitungen | 10 | Höchstens verfolgte Weiterleitungen (0 bis 30). |
| User-Agent | `DumbMonit/<version>` | Wird statt dem eigenen von DumbMonit gesendet. |
| Query-Strings ignorieren | an | URL-Parameter zählen beim Vergleich von Adressen nicht. |
| Auf Änderungen prüfen alle (Minuten) | 360 | Wie oft Metadaten, endgültige Adresse und `robots.txt` verglichen werden, 0 bis 10080; 0 bedeutet bei jeder Prüfung. Leer: der Serverstandard (`DUMBMONIT_WEBSITE_CHANGE_CHECK_MINUTES`). |
| Vor der nächsten Anfrage an diese Domain warten (Sekunden) | 2 | Mindestabstand nach der Prüfung dieser Website, bevor eine weitere Prüfung derselben Domain erfolgt, 0 bis 3600. Leer: der Serverstandard (`DUMBMONIT_WEBSITE_HOST_SPACING_SECS`). |
| Loopback- und Link-Local-Ziele erlauben | aus | Für eine Website, die vom DumbMonit-Host selbst ausgeliefert wird. |

## Hunderte Websites takten {#pacing-hundreds-of-sites}

Prüfungen verlassen den Server nie in einem Schwall, damit eine Firewall oder ein CDN vor den
Websites DumbMonit nicht für einen Angriff hält:

- **Über das Intervall verteilt**: Jede Website startet mit ihrem eigenen, festen Versatz
  innerhalb ihres Abfrageintervalls, sodass gemeinsam hinzugefügte Websites (ein CSV-Import)
  nicht in derselben Sekunde auslösen.
- **Globale Obergrenze**: höchstens `DUMBMONIT_WEBSITE_MAX_CONCURRENT` gleichzeitig laufende
  Website-Prüfungen (standardmäßig 10), zusätzlich zu `DUMBMONIT_MAX_CONCURRENT_PROBES`.
- **Pro Domain**: höchstens `DUMBMONIT_WEBSITE_MAX_PER_HOST` gleichzeitig laufende Prüfungen
  an eine Domain (standardmäßig 1) und mindestens `DUMBMONIT_WEBSITE_HOST_SPACING_SECS`
  Sekunden zwischen zweien davon (standardmäßig 2 oder das **Vor der nächsten Anfrage an diese
  Domain warten** der Website). Die Domain ist der registrierbare Name: `www.example.com`,
  `shop.example.com` und `example.com` teilen sich eine, `example.co.uk` ist eine; Kunden-
  Subdomains von Hosting-Plattformen (`*.myshopify.com`, `*.wordpress.com`, `*.github.io`,
  `*.netlify.app` …) zählen jeweils einzeln.

Eine Website, deren vorherige Prüfung noch auf ihren Zug wartet, überspringt eine Runde, statt
sich doppelt einzureihen. Viele Websites auf einer Domain mit kurzem Abfrageintervall werden
daher seltener geprüft als verlangt: Erhöhen Sie das Intervall oder die Limits pro Domain, wenn
die Geräteliste sie verspätet zeigt. Siehe [Konfiguration](../reference/configuration.md#server).

## Plattformerkennung {#platform-detection}

Die Plattform stammt der Reihe nach aus: dem Tag `<meta name="generator">`, für einen Host oder
eine Engine spezifischen Headern (`x-shopify-*`, `x-wix-*`, `x-drupal-cache`,
`x-powered-by: Next.js` …), Spuren im Markup (`/wp-content/`, `cdn.shopify.com`, `data-wf-site`,
`__NEXT_DATA__`, `/_nuxt/` …) und schließlich dem Host selbst (`*.myshopify.com`,
`*.wixsite.com` …). Erkannt werden: WordPress, Shopify, Wix, Squarespace, Webflow, Ghost,
Drupal, Joomla, PrestaShop, Magento, Next.js, Nuxt, Gatsby, Hugo, Jekyll; alles andere ist
**Other**. Die Plattform ist das Label `platform` von `dumbmonit_website_info`.

## Eine Liste von Websites importieren (CSV) {#import-a-list-of-sites-csv}

Aus der Geräteliste oder der Seite Gerät hinzufügen nimmt **Websites importieren (CSV)** eine
Datei oder eingefügten Text mit Kopfzeile und bis zu 1000 Zeilen:

| Spalte | Erforderlich | Bedeutung |
|---|---|---|
| `url` | ja | Adresse der Website. |
| `name` | nein | Gerätename; standardmäßig der Host. |
| `folder` | nein | Ordner des Geräts; leer: `Websites/<platform>`, erkannt durch einmaliges Lesen der Website. |
| `tags` | nein | `key=value;key=value`. |
| `interval` | nein | Abfrageintervall. |
| `expect_text` | nein | Wie die Option. |

```csv
url,name,folder,tags,interval,expect_text
https://example.com,Example,,client=acme,60s,Example Inc.
https://shop.example.org,Shop,Customers/Acme,,300s,Add to cart
```

Eine Vorschau zeigt jede Zeile und ihre Fehler, bevor etwas erstellt wird; eine bereits
überwachte Website (gleiche endgültige Adresse) wird übersprungen.

## Metriken {#metrics}

| Metrik | Bedeutung |
|---|---|
| `dumbmonit_website_up` | 1, wenn die endgültige Antwort ein Erfolg ist (2xx oder 3xx), sonst 0. |
| `dumbmonit_website_status_code` | HTTP-Status der endgültigen Antwort. |
| `dumbmonit_website_response_seconds` | Gesamtzeit, Weiterleitungen eingeschlossen. |
| `dumbmonit_website_redirect_hops` | Verfolgte Weiterleitungen. |
| `dumbmonit_website_https` | 1, wenn die endgültige Adresse https ist. |
| `dumbmonit_website_security_header{header}` | 0/1 je Label `header` (nur https): HSTS, nosniff, CSP, X-Frame-Options, frame-ancestors. |
| `dumbmonit_website_info{platform}` | Immer 1; das Label trägt die Plattform. |
| `dumbmonit_website_text_chars` | Zeichen sichtbaren Texts (2xx-Antworten). |
| `dumbmonit_website_blank` | 1, wenn dieser Text unter dem Schwellenwert liegt. |
| `dumbmonit_website_expect_text_missing` | 1, wenn der erwartete Text fehlt (Option gesetzt). |
| `dumbmonit_website_noindex` / `dumbmonit_website_nofollow` | Indexierungsdirektiven. |
| `dumbmonit_website_h1_count` / `dumbmonit_website_hreflang_count` | `<h1>`-Überschriften, `hreflang`-Alternativen. |
| `dumbmonit_website_title_changed`, `…_description_changed`, `…_canonical_changed`, `…_final_url_changed` | 1 für 24 Stunden nach einer bestätigten Änderung. |
| `dumbmonit_website_robots_blocks_all` | 1, wenn `robots.txt` Googlebot alles verbietet. |
| `dumbmonit_website_sitemap_ok` | 1, wenn eine Sitemap antwortet und lesbar ist. |

## Integrierte Alarme {#built-in-alerts}

| Alarm | Wann | Schweregrad |
|---|---|---|
| Gerät nicht erreichbar | Keine Antwort, Zertifikat abgelehnt, Weiterleitungsschleife oder zu viele Weiterleitungen | kritisch |
| Website ausgefallen | Die endgültige Antwort ist ein 4xx- oder 5xx-Fehler, 3 Minuten lang | kritisch |
| Website-Seite leer | Sichtbarer Text unter dem Schwellenwert, 3 Minuten lang | kritisch |
| Website erwarteter Text fehlt | Der erwartete Text fehlt, 3 Minuten lang | Warnung |
| Website nicht indexierbar | `noindex` auf der Seite, 10 Minuten lang | kritisch |
| Website durch robots.txt gesperrt | `robots.txt` verbietet alles, 10 Minuten lang | kritisch |
| Kanonische URL der Website geändert | Bestätigte Änderung | Warnung |
| Adresse der Website geändert | Endgültiger Host oder Pfad geändert, bestätigt | Warnung |
| Titel oder Beschreibung der Website geändert | Bestätigte Änderung | Info |
| Website langsam | Mehr als 5 Sekunden, 10 Minuten lang | Warnung |

Die Prüfung läuft auf dem DumbMonit-Server selbst: Sie kann nicht über einen Relay-Agenten laufen.
