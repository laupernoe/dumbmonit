# E-Mail-Berichte

Ein Bericht ist eine kurze E-Mail, die Ihrem Team sagt, wie die
Infrastruktur im letzten Tag, der letzten Woche oder dem letzten Monat
lief, ohne dass jemand DumbMonit öffnet. Er ist standardmäßig wöchentlich;
täglich und monatlich sind einen Klick entfernt. Berichte nutzen einen
**E-Mail-Benachrichtigungskanal (SMTP)** mit, richten Sie also zuerst
einen ein (siehe [Benachrichtigungskanäle](../notifications.md)).

Öffnen Sie **Settings > Reports**, um sie einzurichten. Alles hier ist nur
für Administratoren, auch das Lesen, weil die Seite die Adressen der
Empfänger aufführt.

## Was ein Bericht enthält

| Abschnitt | Inhalt |
| --- | --- |
| **Headline** | Gesamtverfügbarkeit für den Zeitraum, die Anzahl der Vorfälle, und die Veränderung gegenüber dem vorherigen Zeitraum. |
| **Availability by group** | Eine Zeile pro Gruppe, dann pro Monitor (höchstens die 40 am wenigsten verfügbaren), berechnet genauso wie auf [Statusseiten](status-pages.md): Abfragen nach ihrer Erfolgsrate, Hosts und Monitore danach, wie oft sie sich gemeldet haben. Monitore ohne Historie werden als "no data" statt als 100 % gezeigt. |
| **Incidents** | Wann jeder begann, wie lange er dauerte (oder dass er noch offen ist) und auf welchem Monitor. Bis zu 25. Informative Alarme werden ausgelassen. |
| **Least stable monitors** | Die fünf Monitore mit der niedrigsten Verfügbarkeit oder den meisten Vorfällen. |
| **Comparison** | Verfügbarkeit und Vorfallanzahl gegenüber dem gleich langen Zeitraum direkt davor. |

Die E-Mail besteht aus einfachen HTML-Tabellen mit Inline-Stilen und ohne
externes Bild, sodass sie überall gleich dargestellt wird und beim Öffnen
nichts nachgeladen wird. Eine Textversion wird mitgesendet. Jeder
Monitorname und jede Meldung wird escaped.

## Einstellungen

| Einstellung | Was sie tut |
| --- | --- |
| **Name** | Nur in der Liste angezeigt. |
| **Enabled** | Aus behält die Einstellungen, ohne etwas zu senden. |
| **Frequency** | Täglich, wöchentlich (Standard) oder monatlich. |
| **Day** | Der Wochentag für wöchentliche Berichte, der Tag des Monats (1 bis 28) für monatliche. |
| **Hour** und **Timezone** | Wann der Bericht rausgeht, in einer IANA-Zeitzone wie `Europe/Paris`. Die Sommerzeit wird berücksichtigt. |
| **Recipients** | Bis zu 20 Adressen, frei wählbar: Sie müssen keine DumbMonit-Benutzer sein. Jeder Empfänger erhält seine eigene E-Mail, sodass niemand die anderen sieht. |
| **Email channel** | Welcher SMTP-Kanal ihn sendet. Standardmäßig der erste aktivierte. Nur der Mailserver des Kanals wird genutzt: seine eigenen Empfänger werden ignoriert. |

Der Zeitraum ist ein gleitendes Fenster, das endet, wenn der Bericht
gesendet wird: die letzten 24 Stunden, 7 Tage oder 30 Tage. Der vorherige
Zeitraum ist das gleich lange Fenster direkt davor.

Klicken Sie auf das **i** neben dem Abschnittstitel für eine Erinnerung
an den Inhalt.

## Eine Vorschau senden

**Send a preview** sendet den aktuellen Bericht sofort per E-Mail an die
gespeicherten Empfänger, mit `[Preview]` im Betreff. Es ändert nichts am
Zeitplan: Der nächste Bericht geht trotzdem pünktlich raus. Eine Vorschau
kann höchstens einmal alle 30 Sekunden pro Bericht gesendet werden.
Speichern Sie Ihre Änderungen zuerst, da die Vorschau die gespeicherten
Empfänger nutzt.

## Zustellgarantien

- **Höchstens einmal.** Der Slot wird in der Datenbank reserviert, bevor
  die E-Mail gebaut wird, sodass ein Neustart nie denselben Bericht
  zweimal sendet. Schlägt der Versand fehl, wird der Fehler am Bericht
  festgehalten und in der Liste angezeigt, und der Slot wird nicht
  wiederholt; der nächste geht wie gewohnt raus.
- **Kein Nachholen.** War DumbMonit zur geplanten Zeit nicht erreichbar,
  wird der Bericht bei der Rückkehr nur gesendet, wenn das innerhalb von
  3 Stunden nach dem Slot liegt. Später als das wartet er auf den
  nächsten Slot. Ein von Ihnen erstellter oder wieder aktivierter Bericht
  sendet nie einen bereits vergangenen Slot.
- Es können höchstens 10 Berichte existieren.
- Im Demo-Modus wird nichts gesendet.

Berichte sind in [Backups](../install/backup.md) enthalten: Der
E-Mail-Kanal wird bei der Wiederherstellung über den Namen abgeglichen,
und ein wiederhergestellter Bericht startet frisch (sein Datum der
letzten Sendung wird nicht übernommen).

## API

Siehe [HTTP-API](../reference/api.md#email-reports).
