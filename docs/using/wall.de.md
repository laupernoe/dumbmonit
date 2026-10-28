# Wall-Modus {#wall-mode-and-music}

`/wall` zeigt die eigene Tafel der Konsole im Vollbild, für einen Monitor, der in
einem Raum eingeschaltet bleibt. Sie aktualisiert sich selbst und hält den
Bildschirm wach.

Von oben nach unten:

- das **Urteil** in wenigen Worten („All good“, „3 problems“) mit Datum und Uhr
  und dem Zeitpunkt der letzten Aktualisierung;
- die **fünf Kennzahlen** der Übersicht: Monitors up, Verfügbarkeit über 30 Tage,
  p95-Antwortzeit, offene Alarme und mittlere Zeit bis zur Behebung;
- der **Health River**, das große Bild der Tafel: ein Streifen pro Monitor, die
  schlechtesten oben;
- die **offenen Alarme**, die schwersten zuerst;
- **wer Bereitschaft hat**, mit Plan und Schichtende.

Keine Musik, keine Widgets: schwarzer Grund, die Schrift der
Konsole, Zustände neben einer Form und nie allein durch Farbe.

## Öffnen und verlassen

Öffnen Sie sie mit dem Symbol **Wall mode** in der oberen Leiste, über die
Befehlspalette („Wall mode“) oder durch Eingabe der URL. ++esc++ oder **Exit**
führt zurück zur Konsole. Ein auf einige Teams oder Kunden beschränktes Konto
erhält die Wand nicht, da sie die gesamte Flotte zeigt.

## Hintergrund {#backdrop}

Die Wand teilt das dunkle Erscheinungsbild der Konsole. Die Schaltfläche
**Hintergrund** (oben rechts, neben Exit) kann eine der Nachtstädte der
[Statusseiten](status-pages.md) hinter Urteil und Uhr zeichnen: Paris, New York,
Tokio oder Dubai, oder *Auto* für die der Zeitzone des Browsers nächstgelegene.
Sie ist standardmäßig aus und wird pro Browser gemerkt; `?city=paris` (oder
`newyork`, `tokyo`, `dubai`, `none`) in einem Link wählt sie nur für diese
Anzeige. Der Himmel folgt der Übersicht: Sterne, wenn alles gut ist, Wolken bei
einem Hinweis, Gewitter bei einer Warnung. Auf einem Telefon oder in einem
verborgenen Tab steht die Stadt still. Ein gespeichertes oder verlinktes
`zermatt` landet jetzt auf Dubai.
