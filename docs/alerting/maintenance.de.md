# Wartungsfenster

Ein Wartungsfenster (eine *Silence* in der API) stoppt Benachrichtigungen für
ein Monitor oder für alles, während eines Zeitraums. Die Auswertung läuft im
Hintergrund weiter: Wenn das Fenster endet, wird ein noch aktiver Alarm
einmal benachrichtigt, statt nach zwei Stunden plötzlich aus dem Nichts
aufzutauchen.

## Wo

- **Alerts → Maintenance**: die Liste der Fenster, mit *Active
  now* oder *Scheduled*, und das Formular zum Anlegen eines neuen.
- **Monitordialog → Silence 1 h / Silence until…**: ein schnelles,
  einmaliges Fenster für den gesamten Monitor.
- ++ctrl+k++ → *Schedule maintenance*.

**Snooze**, auf einem Alarm im Tab *Now* oder einer Monitorseite,
ist eine kleinere Variante desselben Mechanismus: ein einmaliges Fenster,
das genau auf die Labels dieses einen Alarms begrenzt ist statt auf den
ganzen Monitor, sodass eine Schwesterregel weiter spricht. Es taucht hier
ebenfalls auf, benannt `Snooze · …`. Siehe
[Einen Alarm loswerden](../using/alerts.md#getting-rid-of-an-alert).

## Einmaliges Fenster

| Feld | Bedeutung |
|---|---|
| Name | Wird in der Liste und in der Historie angezeigt. |
| Monitor | Leer lassen, um jeden Monitor abzudecken. |
| Start, Ende | Absolute Zeiten. Das Fenster schließt den Start ein und das Ende aus. |
| Kommentar | Optional. Warum das Fenster existiert. |

Einmalige Fenster werden gelöscht, sobald sie vorbei sind.

## Wöchentliches Fenster

| Feld | Bedeutung |
|---|---|
| Tage | Welche Wochentage (Montag bis Sonntag). |
| Von, Bis | Lokale Zeit in der Zeitzone des Fensters. Liegt *Bis* bei oder vor *Von*, läuft das Fenster über Mitternacht hinaus ("23:00 → 01:00" gehört zu dem Tag, an dem es beginnt). |
| Zeitzone | Ein IANA-Name wie `Europe/Paris`. Vorausgefüllt mit der Zeitzone Ihres Browsers. |

## Monatliches Fenster

| Feld | Bedeutung |
|---|---|
| An einem Wochentag | *Erster*, *Zweiter*, …, *Fünfter* oder *Letzter*, plus der Tag — „der erste Sonntag“, „der letzte Freitag“. |
| An einem Datum | Ein oder mehrere Tage des Monats, von 1 bis 31, durch Kommas getrennt ("1, 15"). |
| Von | Lokale Startzeit, in der Zeitzone des Fensters. |
| Für | Wie lange das Fenster dauert, als real verstrichene Zeit. Ein monatliches Fenster kann länger als einen Tag laufen. |
| Zeitzone | Wie oben. |

Ein Monat, der das Vorkommnis nicht enthält, wird übersprungen statt
angenähert: Es gibt keinen 31. im Februar und keinen fünften Sonntag im
April 2026, also öffnet sich das Fenster in diesem Monat einfach nicht. Die
Liste zeigt das nächste vom Server berechnete Vorkommen, sodass Sie es auf
einen Blick prüfen können.

DumbMonit hat absichtlich keine Cron-Ausdrücke: Ein Cron-Ausdruck benennt
*Zeitpunkte*, und ein Wartungsfenster braucht eine *Dauer*. Tage des Monats
und „der n-te Wochentag“ decken die Kalender ab, die Menschen tatsächlich
führen, und lesen sich in einer Zeile zurück.

## Zeitzonen und Sommerzeit

Ein wiederkehrendes Fenster öffnet sich zu seiner lokalen Wanduhrzeit und
läuft dann für seine Dauer in Echtzeit. Beide Hälften sind wichtig:

- **Die lokale Stunde driftet nicht.** „Jeden Sonntag um 02:00 in
  `Europe/Paris`“ ist 01:00 UTC im Winter und 00:00 UTC im Sommer — das
  Fenster folgt der Zeitumstellung, statt zweimal im Jahr um eine Stunde zu
  verrutschen.
- **Die Dauer dehnt sich nicht.** In der Nacht, in der die Uhr zurückgestellt
  wird, würde 02:00 bis 04:00 auf der Wanduhr drei reale Stunden dauern; das
  Fenster dauert trotzdem zwei. In der Nacht, in der die Uhr vorspringt und
  02:00 nicht existiert, öffnet sich das Fenster, sobald die Uhr wieder
  läuft.

Ein Fenster, das vor der Existenz von Zeitzonen gespeichert wurde, behält
seinen festen UTC-Versatz und verhält sich genau wie zuvor. Erneut speichern
mit einer Zeitzone, damit es den Zeitumstellungen folgt. Ein dem Server
unbekannter Zeitzonenname wird beim Speichern mit einem Beispiel abgelehnt.

## Matcher

Ein Fenster kann außerdem durch exakte Label-Übereinstimmungen eingeschränkt
werden (`matchers` in der API): Alle müssen mit den Labels des Alarms
übereinstimmen, zum Beispiel `tag_role = lab` und `__name__ =
dumbmonit_up`. Ein Fenster ohne Monitor und ohne Matcher schaltet die gesamte
Instanz stumm. Überlappende Fenster sind kein Problem; das erste, das den
Moment abdeckt, erklärt die Stummschaltung.

## Wie sieht ein stummgeschalteter Alarm aus

Der Alarm behält seine Phase und zeigt `silenced: true`; die Historie
protokolliert den Übergang mit `notified: false` und dem Grund. Das
Deaktivieren eines Fensters (`enabled: false`) behält es in der Liste, aber
ohne Wirkung.

## Auf einer öffentlichen Statusseite

Ein Monitor, der von einem Wartungsfenster abgedeckt wird, wird auf jeder
[Statusseite](../using/status-pages.md), die es auflistet, als **Wartung**
angezeigt, statt rot. Wenn sonst nichts auf der Seite ausgefallen oder
beeinträchtigt ist, liest sich auch der Gesamtzustand der Seite als
*Wartung*. Nur Fenster, die ein Monitor benennen — oder deren Matcher
monitorweite Labels sind — zeigen sich auf diese Weise; ein auf eine Metrik
begrenztes Fenster bleibt eine Alarmierungsangelegenheit.
