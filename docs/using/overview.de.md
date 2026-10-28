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

## Die fünf Kennzahlen und der Health River {#the-figures}

Unter dem Banner stehen fünf Kennzahlen: **Monitors up** (von der Gesamtzahl, mit
den Zahlen ausgefallener und pausierter Monitore), **Availability 30 d**,
**p95 response time**, **Open alerts** (kritisch und Warnung) und **MTTR 30 d**
(mittlere Zeit bis zur Behebung). Der Zeitraum in der Kopfzeile steuert p95 und
den River.

Der **Health River** ist ein Streifen pro Monitor, eine Zelle pro Abschnitt des
Zeitraums, die schlechtesten Monitore oben. Ein Zustand steht immer neben einer
Form (Kreis aktiv, Raute beeinträchtigt, Quadrat ausgefallen), nie allein als
Farbe.

## Die Monitortabelle {#the-monitor-table}

Die Tabelle listet jeden Monitor, den schlechtesten zuerst. Sie filtert nach Zustand,
Typ und Text, lässt Sie Spalten hinzufügen (Adresse, Ordner) oder entfernen und
wird per Tastatur bedient: ++j++ / ++k++ bewegen, ++enter++ wählt, ++o++ öffnet.

Der gewählte Monitor speist das **Kontextpanel** rechts, wie ein Inspektor: was
Aufmerksamkeit braucht (mit den Alarmaktionen, siehe [Alerts-Seite](alerts.md)),
**Check now** und **Silence 1 h**, die nächsten Vorhersagen und wer Bereitschaft
hat. ++i++ blendet es ein oder aus; auf schmaleren Bildschirmen öffnet es sich als
Schublade. Die vollständige Liste steht auf der [Monitors-Seite](devices.md).

## Wall-Modus {#wall-mode}

`/wall` zeigt eine Tafel für einen Monitor im Raum: das Urteil, die Uhr, die fünf
Kennzahlen, den Health River, die offenen Alarme und wer Bereitschaft hat. Öffnen
Sie sie mit dem Symbol **Wall mode** in der oberen Leiste oder über die
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
| Pages | Overview, Monitors, Alerts, Notifications, Status pages, Settings, Wall mode, Documentation |
| Actions | Add a monitor, Scan my network, Schedule maintenance, Add notification channel, New status page, Announce an incident |
| Monitors | Jeder Monitor, mit seiner Zustands-LED; ++enter++ öffnet ihn |

## Navigation {#navigation}

Eine schmale **Icon-Leiste** links listet jedes Ziel: Overview, Monitors, Alerts
(mit der Anzahl dessen, was Ihre Aufmerksamkeit braucht), Dashboards, Explore, die
Topologiekarte, Security, Logs, Status pages, Reports und Settings. Unten stehen
die Dokumentation, GitHub und Ihr Konto. Die Kopfzeile zeigt, wo Sie sind, den
Schalter **LIVE**, der die eigenen Aktualisierungen der Konsole pausiert, den
Zeitraum, die Palette, **Wall mode** und **New monitor**. Auf einem Telefon wird
die Leiste zu einer unteren Tab-Leiste mit dem Wichtigsten und **More**.

- **Overview**: ist alles in Ordnung, und was braucht Sie.
- **Monitors**: jede überwachte Quelle. Siehe [Monitors-Seite](devices.md).
- **Alerts**: was feuert, Wartungsfenster, Historie und die Regeln. Siehe
  [Alerts-Seite](alerts.md).
- **Settings**: Ihr Konto, Benutzer und Single Sign-On, Tokens,
  Benachrichtigungskanäle, Erscheinungsbild, About. Siehe
  [Settings](settings.md).

Die Oberfläche ist dunkel, und nichts darauf vermittelt Status allein durch
Farbe: Eine Markierung kommt immer mit einem Wort (Reporting, Unreachable,
Advisory, Warning, Suppressed by parent, Building up).
