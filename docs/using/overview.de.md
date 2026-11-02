# Die Übersichtsseite

Die Startseite beantwortet „Ist alles in Ordnung?“ in etwa einer Sekunde und
lässt Sie dann nach Bedarf in die Details gehen.

## Das Statusbanner {#the-bulletin}

Ein Band oben zeigt einen festen Nachthimmel: funkelnde Sterne, eine Mondsichel
und eine Taube, die ihn durchquert, ohne je zu landen. Der Himmel ändert sich
nicht mit dem Wetter; der Satz an seinem Fuß sagt in Worten, wie es steht. „Alle
146 Monitore laufen.“, wenn nichts Ihre Aufmerksamkeit braucht, sonst „pve-02 ist
ausgefallen.“ oder „pve-02 braucht einen Blick.“, gefolgt in leiserem Ton von dem,
was das System getan hat („3 Gäste zurückgehalten, Alarm gesendet.“). Tippen Sie
auf die Taube, und sie macht eine Rolle. Mit `prefers-reduced-motion` bleibt die
Taube aus dem Himmel und die Sterne stehen still.

## Die Erstlauf-Anleitung {#the-first-run-guide}

Auf einer neuen Instanz ist die Seite unter dem Bulletin nicht leer: Es
ist eine dreistufige Anleitung — ein Monitor hinzufügen, eine Art,
benachrichtigt zu werden, verbinden, prüfen, dass eine Nachricht
ankommt. Jeder Schritt ist ein Klick, jeder kann für sich weggewischt
werden, und **Skip the guide** räumt alle drei weg. Die Wahl wird auf
der Instanz gespeichert, sodass ein zweiter Browser die Anleitung nicht
neu beginnt. Siehe
[Ihren ersten Monitor hinzufügen](../install/first-device.md#the-three-first-steps).

## Die vier Kennzahlen {#the-figures}

Unter dem Banner stehen vier Kennzahlen: **Monitore online** (von der Gesamtzahl, mit den Zahlen ausgefallener und pausierter Monitore), **Verfügbarkeit 30 T**, **p95-Antwortzeit** und **Offene Alarme** (kritisch und Warnung). Der Zeitraum in der Kopfzeile steuert p95 und den Gesundheitsverlauf.

## Was Aufmerksamkeit braucht {#what-needs-attention}

Die Liste neben dem Gesundheitsverlauf enthält nur, was jetzt kaputt ist, das Schlimmste zuerst, jede Zeile mit ihrer Schnellaktion: einen Alarm bestätigen oder zurückstellen oder einen Monitor stummschalten (das kleine Kreuz). Ist nichts kaputt, schläft die Taube und die Karte sagt es. Am Fuß steht die **Woche voraus** in einer Zeile: was in den nächsten sieben Tagen abläuft. **Alle ansehen** öffnet die [Warnungen](alerts.md).

## Der Gesundheitsverlauf {#the-health-river}

Ein Streifen pro Monitor, eine Zelle pro Abschnitt des Zeitraums, die schlechtesten Monitore oben. Die Übersicht zeigt die ersten acht; **Alle Monitore ansehen** öffnet den Tab [Liste](devices.md) von Monitore. Ein Zustand steht immer neben einer Form (Kreis aktiv, Raute beeinträchtigt, Quadrat ausgefallen), nie allein als Farbe. Auf einem Telefon öffnet sich der Verlauf per Tipp.

## Zwei Diagramme {#charts}

Weiter unten: die **Antwortzeit** der gesamten Flotte über den Zeitraum (p95 und Median, mit den langsamsten Monitoren) und die **Warnungen der letzten sieben Tage** (ausgelöst und behoben).

## Wall-Modus {#wall-mode}

`/wall` zeigt eine Tafel für einen Monitor im Raum: das Urteil, die Uhr, die fünf
Kennzahlen, den Health River, die offenen Alarme und wer Bereitschaft hat. Öffnen
Sie sie mit dem Symbol **Wall mode** unten im linken Menü oder über die
Befehlspalette; ++esc++ oder **Exit** führt zurück zur Konsole. Siehe
[Wall-Modus](wall.md).

## Auf einem Telefon installieren {#install-it-on-a-phone}

Die Oberfläche liefert ein Web-Manifest, sodass ein Telefon sie dem
Homescreen hinzufügen kann: **Share → Add to Home Screen** auf iOS,
**Install app** im Chrome-Menü auf Android. Sie öffnet sich dann ohne
Browser-Chrome, im Theme, auf das das Telefon eingestellt ist.

Es gibt absichtlich **keinen Offline-Modus und keinen Service
Worker**. Ein Überwachungsbildschirm, der eine zwischengespeicherte
Seite zeigt, ist schlimmer als einer, der sagt, dass er den Server
nicht erreichen kann: Der ganze Sinn ist, dass das, was Sie lesen,
aktuell ist. Ohne Worker wird auch ein aktualisierter Server sofort
ausgeliefert, ohne veralteten Build zu löschen.

## Befehlspalette {#command-palette}

++cmd+k++ auf macOS, ++ctrl+k++ sonst, öffnet die Befehlspalette: ein
Eingabefeld, Ergebnisse in drei Gruppen.

| Gruppe | Einträge |
|---|---|
| Pages | Übersicht, Monitore, Karte, Warnungen, Automatisierung, Benachrichtigungen, Metrik-Explorer, Sicherheit, Statusseiten, Berichte, Logs, Einstellungen, Wandmodus, Dokumentation |
| Actions | Add a monitor, Scan my network, Schedule maintenance, Add notification channel, New status page, Announce an incident |
| Monitors | Jeder Monitor, mit seiner Zustands-LED; ++enter++ öffnet ihn |

## Navigation {#navigation}

Ein schmales **Menü** links listet vier Ziele: **Übersicht**, **Monitore**, **Warnungen** (mit der Anzahl dessen, was Ihre Aufmerksamkeit braucht) und **Status**. Unten stehen **Einstellungen**, **Wandmodus**, die Dokumentation, GitHub und Ihr Konto. Was früher eine eigene Seite war, ist jetzt ein Tab dieser Einträge. Die Kopfzeile zeigt, wo Sie sind, den Schalter **LIVE**, der die eigenen Aktualisierungen der Konsole pausiert, den Zeitraum, die Palette und **Monitor hinzufügen**. Auf einem Telefon wird das Menü zu einer unteren Tab-Leiste mit dem Wichtigsten und **Mehr**.

- **Übersicht**: ist alles in Ordnung, und was braucht Sie.
- **Monitore**: jede überwachte Quelle, in vier Tabs: **Liste**, **Karte**, **Logs** und **Sicherheit**. Siehe [Monitore](../monitors/index.md).
- **Warnungen**: fünf Tabs: **Jetzt**, **Verlauf**, **Regeln**, **Automatisierung** und **Bereitschaft**. Siehe [Alerts-Seite](alerts.md).
- **Status**: **Seiten** (öffentliche Statusseiten) und **Berichte**. Siehe [Status](../status/index.md).
- **Einstellungen**: Ihr Konto, Team und Zugang, Überwachung, Benachrichtigungen, die Wand und das System. Siehe [Einstellungen](../settings/index.md).

Die Oberfläche ist dunkel, und nichts darauf vermittelt Status allein durch
Farbe: Eine Markierung kommt immer mit einem Wort (Reporting, Unreachable,
Advisory, Warning, Suppressed by parent, Building up).
