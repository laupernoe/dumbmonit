# Die Alerts-Seite

![Die Alerts-Seite: was Ihre Aufmerksamkeit braucht, die Regeln, geplante Wartungen und die Historie](../assets/screenshots/alerts-light.png){ loading=lazy }

Alles zur Alarmierung auf einer Seite, hinter fünf Tabs. Der Tab steht in
der URL (`/alerts#rules`), sodass eine Ansicht verlinkbar ist.
**Schedule maintenance**, oben rechts, öffnet das Formular für ein
Fenster von jedem Tab aus.

| Tab | Was er enthält |
|---|---|
| **Now** | Die Live-Liste *Needs you*, nach Gerät gruppiert: Schweregrad-Anzeige, Grund, seit wann, und *Ack*, *Snooze*, *Ignore* und *Open device* bei jedem. Bestätigte und stummgeschaltete Alarme sitzen jeweils in ihrer eigenen ruhigen Gruppe am Ende. Das Badge auf dem Tab, und auf Alerts in der oberen Leiste, ist die Anzahl dessen, was noch Ihre Aufmerksamkeit braucht. |
| **Scheduled** | Wartungsfenster, *Active now* oder *Scheduled*, einmalig, wöchentlich oder monatlich, jeweils mit dem nächsten Vorkommen. Siehe [Wartungsfenster](../alerting/maintenance.md). |
| **Rules** | Jede Regel mit ihrem Schweregrad, einer *Built-in*-Markierung und, falls ein Gerät sie ignoriert, *Ignored on N devices*; aktivieren, inline bearbeiten, eigene löschen, eine Schwellenwertregel erstellen. Siehe [Regeln](../alerting/rules.md). |
| **Notifications** | Wohin Alarme Sie erreichen: die Kanäle und die Benachrichtigungsrichtlinie. Details unten. |
| **History** | Die letzten 200 Übergänge, jeder mit Gerät, Regel und dem Geschehenen; behobene können gelöscht werden. |

Dasselbe Wahrheitsmodell füttert das Übersichts-Bulletin und diese
Seite, daher sind sich beide immer einig, was Ihre Aufmerksamkeit
braucht.

## Einen Alarm loswerden {#getting-rid-of-an-alert}

Ein feuernder Alarm ist nicht binär — "beheben" oder "mit dem Lärm für
immer leben". Vier Aktionen, für vier verschiedene Situationen, offen
für Admins und Operatoren (ein Viewer sieht den Zustand, kann ihn aber
nicht ändern):

- **Acknowledge** ("Ich bin dran") ist für einen Alarm, an dem Sie
  aktiv arbeiten: "Ich weiß, hör auf, mich 4 h zu erinnern". Das Menü
  bietet 1 h, 4 h, 24 h oder *until resolved*, plus eine optionale
  Notiz für den, der die Karte nach Ihnen liest. Der Alarm feuert und
  wird weiter ausgewertet; nur seine Erinnerungen und Eskalationen
  pausieren — einschließlich der Eskalation an einen zweiten Kanal. Sie
  werden trotzdem informiert, wenn er sich löst, und die Bestätigung
  löscht sich in diesem Moment — ein später wiederkehrender Alarm
  benachrichtigt erneut. Die Karte wandert aus der Hauptliste in eine
  ruhigere *Acknowledged*-Gruppe, liest sich *Acked by someone until a
  time*, und bietet **Un-ack**.
- **Snooze** ist für "nicht jetzt, später nachsehen", ohne zu
  behaupten, Sie würden etwas dagegen tun: 1 h, 8 h, 1 Tag oder *until
  resolved*. Anders als Ack funktioniert es, indem ein auf genau die
  Labels dieses einen Alarms begrenztes Wartungsfenster geöffnet wird —
  eine Schwesterregel auf demselben Gerät spricht weiter. Die Karte
  wandert in eine ruhige *Snoozed*-Gruppe, die die verbleibende Zeit
  zeigt, und bietet **Unsnooze**. Erinnerungen pausieren genauso wie
  bei einer Bestätigung.
- **Ignore** ("benachrichtige mich dazu nie wieder") ist für eine
  Regel, die auf diesem Gerät nie Sinn ergeben wird: Es deaktiviert
  diese Regel für genau dieses Gerät (eine gerätespezifische
  Überschreibung — siehe
  [Gerätespezifische Überschreibungen](../alerting/rules.md#per-device-overrides)),
  und der Alarm verschwindet endgültig, sofort, nicht nur leiser. Es
  ist reversibel: **Stop ignoring** auf der Geräteseite, oder von der
  Regel selbst unter Alerts → Rules, wo sie sich *Ignored on N devices*
  liest.
- **Clear** ist für die Historie, nicht für einen lebenden Alarm:
  Sobald ein Übergang sich gelöst hat, entfernt *Clear* (oder *Clear
  all resolved*) ihn aus der Standard-History-Ansicht. Die Zeile bleibt
  in der Datenbank — nichts wird gelöscht — und *Show cleared* holt sie
  zurück.

Öffentliche Statusseiten ignorieren Bestätigungen und Stummschaltungen:
Ein bestätigter oder stummgeschalteter Alarm ist für die Außenwelt
trotzdem ein Alarm. Eine ignorierte Regel hingegen hat von Anfang an
nie einen Alarm erzeugt.

## Notifications

Kanäle und die Richtlinie leben hier — unter Alerts, nicht Settings —
weil sie entscheiden, wer einen Alarm hört, was eine Alarmierungsfrage
ist. Zwei Panels:

**Channels.** Die Liste der Kanäle, jeder mit seiner Art, *Enabled*
oder *Disabled*, wann er zuletzt etwas gesendet hat, und seinem letzten
Fehler, falls vorhanden. **Add channel** öffnet ein aus der Beschreibung
der gewählten Art durch den Server gebautes Formular: Settings
(sichtbar) und Secrets (nur schreibbar). **Send test** sendet eine
Testnachricht und zeigt das Ergebnis inline. Die **Delivery options**
jedes Kanals halten seinen Mindest-Schweregrad, ob er Behoben-Meldungen
hört, einen Mindestabstand pro Alarm, und seine **quiet hours**
(wöchentlich, in Ihrer Zeitzone: nur Alarme der Stufe Warning kommen
durch, der Rest wartet auf eine Zusammenfassung).

**Only some alerts.** Dieselben Delivery options halten den
**Routing-Filter** des Kanals: welche Alarme er möchte, nach
Geräte-Tag (`site=cellar`), nach Geräteart (`proxmox`), oder nach
Regel. Bedingungen auf demselben Feld lesen sich als *oder*,
unterschiedliche Felder als *und*, und eine *Except*-Zeile gewinnt
immer. Unter dem Editor listet eine Vorschau auf, welche Ihrer Geräte
der Filter jetzt gerade auswählt, und ein Satz gibt das Ganze zurück —
*this channel receives advisories and above from devices tagged
site=cellar, except devices tagged role=lab*. Ein Kanal ohne Filter
empfängt weiterhin alles, was jeder bestehende Kanal tut.

Das Bearbeiten eines Kanals und das Leerlassen des Secrets behält das
gespeicherte. Kanalarten und ihre Felder sind in
[Benachrichtigungskanäle](../notifications.md) dokumentiert.

Eingebaute Alarmregeln benachrichtigen jeden aktivierten Kanal; eine
Regel kann in ihrem Editor auch auf bestimmte Kanäle begrenzt werden.

**Notification policy.** Die globalen Regler, die Benachrichtigungen
knapp halten: das Bündelungsfenster, das Limit pro Kanal und Stunde,
die Flackererkennung unter *More options*, die öffentliche URL für die
"Open in DumbMonit"-Links, und die **escalation**-Zeile — *falls
niemand bestätigt* innerhalb einer Verzögerung, *auch informieren* ein
weiterer Kanal, einmal. Die Verzögerung zählt vom Moment, in dem die
erste Nachricht tatsächlich rausging, und das Bestätigen des Alarms
(oder sein Lösen) stoppt den Hop. Siehe
[Benachrichtigungsrichtlinie](../alerting/notifications.md).

Links: `/alerts#notifications` öffnet den Tab,
`/alerts#notifications-policy` scrollt zum Richtlinien-Panel. Die
früheren Settings-Links (`/settings#notifications`,
`/settings#notifications-policy`) leiten hierher weiter.

## Operatoren und Viewer {#operators-and-viewers}

Ein **Operator** bearbeitet Alarme, ohne die Konfiguration anzurühren:
Er bestätigt, schaltet stumm, ignoriert eine Regel für ein Gerät,
plant oder löscht Wartungsfenster und löscht behobene Historie. Regeln,
Kanäle und die Benachrichtigungsrichtlinie bleiben für ihn nur-lesend,
markiert *Operator — config is admin only*. Ignorieren ist die einzige
gerätespezifische Überschreibung, die ein Operator vornehmen kann;
Schwellenwerte bleiben Sache eines Admins, und das *Ignore* oder *Stop
ignoring* eines Operators behält jeden von einem Admin für dieses
Gerät gesetzten Schwellenwert.

Ein Viewer sieht jeden Tab, aber keine Kontrolle: Die Seite zeigt
*Viewer — read only*, wo ein Admin die Schaltflächen fände.
