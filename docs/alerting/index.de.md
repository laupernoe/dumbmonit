# Wie Alarmierung funktioniert

Nützliche Regeln sind direkt nach der Installation aktiv; es muss nichts
nachjustiert werden. Den Unterschied im Alltag macht, was den Lärm
niedrig hält: Abhängigkeitsunterdrückung, Gruppierung nach Host,
Wartungsfenster und eine Baseline, die lernt, bevor sie spricht.

![Die Alerts-Seite: was gerade passiert, Wartung, Historie und die Regeln](../assets/screenshots/alerts-light.png){ loading=lazy }

## Schweregrade {#severities}

Die Oberfläche spricht die Stufenleiter des Wetterberichts. Die API und der
Regel-Editor speichern dieselben drei Stufen unter ihren Engine-Namen.

| In der UI | In der API (`severity`) | Bedeutung |
|---|---|---|
| Info | `info` | Gut zu wissen, keine Aktion erwartet. Wird von der Baseline-Regel genutzt. |
| Advisory | `warning` | Braucht bald Aufmerksamkeit: CPU hoch, Platte fast voll, langsamer Dienst. |
| Warning | `critical` | Braucht Sie jetzt: Gerät nicht erreichbar, Dienst ausgefallen, Backup-Prüfung fehlgeschlagen, Zertifikat abgelaufen. |

Eskalation hebt den Schweregrad einer Regel nach `escalate_after` um eine
Stufe an, wenn der Alarm weiter feuert (Info → Advisory → Warning; Warning
bleibt).

## Phasen {#phases}

Jeder Alarm ist eine Zustandsmaschine, die alle 30 Sekunden ausgewertet wird
(`DUMBMONIT_ALERT_INTERVAL_SECS`).

| Phase | In der UI | Bedeutung |
|---|---|---|
| `ok` | — | Die Bedingung ist falsch. |
| `pending` | Baut sich auf | Die Bedingung ist wahr, aber die Haltezeit der Regel (`for`) ist noch nicht abgelaufen. Es wird nichts gesendet. |
| `firing` | Feuert | Die Bedingung hat lange genug angehalten. Die erste Benachrichtigung geht raus, danach Erinnerungen alle `repeat_interval` (standardmäßig 6 Stunden für die meisten eingebauten Regeln), dann eine Eskalation, falls konfiguriert. |
| `resolved` | Behoben | Zurück zum Normalzustand. Eine Behoben-Benachrichtigung wird nur gesendet, wenn das Feuern zuvor gemeldet wurde. |
| `suppressed` | Durch übergeordnetes Gerät unterdrückt | Der Alarm feuert, aber ein übergeordnetes Gerät oder das Gerät selbst ist nicht erreichbar: siehe unten. |

`phase` ist der Zustand der Engine; `effective_phase` ist das, was Sie lesen.
Ein Alarm, dessen `effective_phase` gleich `suppressed` ist, behält darunter
`phase = firing`, sodass er bei der Wiederherstellung des übergeordneten
Geräts nicht erneut benachrichtigt wird.

Ein Alarm kann auch **bestätigt** werden (`acked`, mit `acked_by`,
`acked_until` und `ack_note`): Die Phase bewegt sich nicht, Erinnerungen und
Eskalationen pausieren bis `acked_until`, die Behoben-Meldung wird trotzdem
gesendet und löscht die Bestätigung. **Das Stummschalten** eines einzelnen
Alarms funktioniert im Hintergrund genauso, aber über ein Wartungsfenster, das
genau auf die Labels dieses Alarms begrenzt ist (`silenced: true`), statt auf
das ganze Gerät: eine Schwesterregel auf demselben Gerät spricht weiter.
Siehe [Einen Alarm loswerden](../using/alerts.md#getting-rid-of-an-alert).

## Abhängigkeitsunterdrückung {#dependency-suppression}

Jedes Gerät kann ein **übergeordnetes Gerät** angeben. Wenn ein Gerät nicht
erreichbar ist (die Regel „Device unreachable“ feuert darauf), wird jeder
Alarm auf seinen Nachkommen als *Durch übergeordnetes Gerät unterdrückt*
markiert, statt gesendet zu werden: ein ausfallender Switch erzeugt eine
Benachrichtigung, nicht dreißig. Der nächstgelegene nicht erreichbare
Vorfahre wird als Ursache genannt (`suppressed_by`), und die Kette wird bis
zu 64 Ebenen verfolgt.

Dasselbe gilt für das nicht erreichbare Gerät selbst: Seine übrigen Alarme
(Festplatte voll, CPU usw.) werden durch seinen eigenen Ausfall unterdrückt,
wobei `suppressed_by` das Gerät selbst nennt, denn ihre Werte sind veraltet
und nur ein Symptom desselben Ausfalls. Nur sein Alarm „Device unreachable“
wird nie durch den eigenen Ausfall verborgen, sodass Sie einmal benachrichtigt
werden.

Das übergeordnete Gerät wird im Gerätedialog gesetzt. Auf der Geräteseite
stapeln sich Kinder unter ihrem Elternteil und werden abgedunkelt, wenn
dieser nicht erreichbar ist.

Ein [Relay-Agent](../install/remote-site.md) zählt als übergeordnetes Gerät
jedes Geräts, das er abfragt (*Erreicht über* im Gerätedialog), zusätzlich
zu einem explizit gesetzten Elternteil: Ist das Relay nicht erreichbar,
werden seine Geräte unterdrückt, wobei das Relay als Ursache genannt wird.
Bei gleichem Abstand gewinnt das explizit gesetzte übergeordnete Gerät.

Ein übergeordnetes Gerät gilt auch dann als nicht erreichbar, sobald seine
eigene Abfrage das meldet (der Status *Zeitüberschreitung* / *Gerät nicht
erreichbar* auf seiner Seite), ohne auf das Feuern seiner eigenen „Device
unreachable“-Regel zu warten: Ein Agent, der aufhört zu pushen, zieht seine
Geräte ein bis zwei Minuten vor seinem eigenen Alarm mit sich, und diese
würden sonst jeweils zuerst benachrichtigen.

## Gruppierung, Deduplizierung, Erinnerungen {#grouping-deduplication-reminders}

Ohne Gruppierung würde ein NAS mit degradiertem RAID eine Nachricht pro
Platte und pro Zyklus senden. DumbMonit sendet höchstens eine Nachricht pro
Host und pro Zyklus, auch wenn sie fünf Zeilen enthält. Jeder Alarm wird
einmal benachrichtigt, wenn er zu feuern beginnt (`firing`), danach nur noch
als Erinnerung (`reminder`, alle `repeat_interval`), als Eskalation
(`escalation`) oder als Behoben-Meldung (`resolved`). Die Alarmhistorie
protokolliert jeden Übergang mit der Information, ob benachrichtigt wurde
und, falls nicht, warum (Lernphase, unterdrückt, stummgeschaltet,
bestätigt).

## Wartungsfenster {#maintenance-windows}

Eine Stummschaltung stoppt Benachrichtigungen, ohne die Auswertung zu
stoppen: am Ende eines Fensters wird ein noch aktiver Alarm einmal
benachrichtigt, statt nach zwei Stunden Existenz „geboren“ zu werden.
Fenster sind einmalig oder wöchentlich, pro Gerät oder für die gesamte
Instanz. Siehe [Wartungsfenster](maintenance.md).

## Baseline-Lernen {#baseline-learning}

Die Regel „Unusual CPU“ braucht keinen Schwellenwert. Für jede Zeitreihe
hält sie 168 Buckets, einen pro (Wochentag, Stunde), und vergleicht jeden
Punkt mit dem, was *zu diesem Zeitpunkt* üblicherweise beobachtet wird. Der
Score ist robust (Median und MAD, ein EWMA pro Bucket mit `alpha = 0.05`),
sodass ein einzelner Vorfall die Baseline nicht neu schreibt und saisonale
Drifts berücksichtigt werden.

Die Regel bleibt während ihrer ersten **vierzehn Lerntage still**: Jeder
Bucket muss mindestens zweimal beobachtet worden sein, um „unüblich“ von
„nie beobachtet“ zu unterscheiden. In der Zwischenzeit wird sie mit einem
*Lernphase*-Kennzeichen angezeigt und zeigt nur an, was sie ausgelöst hätte.
Baselines von Zeitreihen, die verschwunden sind, werden nach 60 Tagen
vergessen.

## Vorhersagen {#forecasts}

Zwei Alarmarten sagen voraus statt zu melden, und die Übersicht listet sie
unter *Vorhersagen*: die Baseline-Anomalie und die `predict`-Regeln, die
eine Zeitreihe mit `predict_linear()` auf VictoriaMetrics-Seite
extrapolieren („Filesystem almost full“ innerhalb von vier Tagen). Die
PBS-Datastore-Schätzung ist ein einfacher Schwellenwert auf PBS' eigener
Vorhersage.

## Wohin Benachrichtigungen gehen {#where-notifications-go}

Eingebaute Regeln haben keinen zugewiesenen Kanal, was bedeutet **jeder
aktivierte Kanal**. Eine selbst bearbeitete Regel kann bestimmte Kanäle
benennen. Kanäle werden unter Settings → Notifications & reports konfiguriert; siehe
[Benachrichtigungskanäle](../notifications.md).

## Intelligente Benachrichtigungen {#smart-notifications}

Alles oben entscheidet, *ob* ein Alarm sprechen darf. Eine zweite Stufe, die
Benachrichtigungsrichtlinie, entscheidet *wann* und *auf welchem Kanal*,
damit niemand zugespamt wird. Der vollständige Weg eines Alarms, in
Reihenfolge:

1. **Auswerten** — Schwellenwert, Anomalie-Score oder Vorhersage, mit der
   Haltezeit der Regel (`for`) und, falls gesetzt, ihrem **Rückstellschwellenwert**
   (Hysterese: feuert über 90 %, löst sich erst unter 85 %) und einer
   etwaigen **gerätespezifischen Überschreibung**.
2. **Deduplizieren** — ein Fingerprint pro (Regel, Zeitreihe), verankert an
   der Geräte-ID (`target`) statt an ihrem Namen: Das Umbenennen eines
   Geräts oder das Bearbeiten seiner Tags erzeugt keinen zweiten Alarm, und
   eine Zeitreihe, die zweimal auf denselben Schlüssel aufgelöst wird,
   ergibt einen Alarm. Zeitreihen eines gelöschten oder pausierten Geräts
   werden verworfen: ihre Alarme werden sofort gelöscht, ohne
   Benachrichtigung, und die Historie vermerkt `device removed or
   disabled`.
3. **Durch Abhängigkeit unterdrücken** — Nachkommen eines nicht
   erreichbaren Geräts bleiben still.
4. **Stummschalten** — Wartungsfenster dämpfen, was sie abdecken.
5. **Nach Gerät gruppieren** — eine Nachricht pro Gerät und Zyklus,
   Erinnerungen und Eskalation eingefaltet.
6. **Flackersperre** — ein Fingerprint, der innerhalb von 30 Minuten
   4-mal feuert und sich löst, sendet eine einzige „Flattern“-Meldung,
   danach 30 Minuten lang nichts.
7. **Kanalspezifische Filter** — Mindest-Schweregrad, „sag mir, wenn es
   sich löst“ an/aus, und ein Mindestabstand zwischen zwei Nachrichten
   zum selben Alarm.
8. **Ruhezeiten** (pro Kanal) — nur Alarme der Stufe Warning kommen
   durch; der Rest wartet auf eine Zusammenfassung, wenn die Ruhezeit
   endet (ein Alarm, der sich währenddessen löst, wird nur als „während
   der Ruhezeit behoben“ erwähnt).
9. **Bündelung** — Alarme innerhalb des Bündelungsfensters (standardmäßig
   60 s) verlassen das System als eine Nachricht pro Kanal: „3 Alarme auf
   2 Geräten, 1 behoben“.
10. **Stündliches Limit** — über `max_per_hour` Nachrichten auf einem
    Kanal hinaus warten Alarme und kommen als eine Zusammenfassung an
    („…und 7 weitere Alarme“).
11. **Senden** — mit einem „In DumbMonit öffnen“-Link pro Gerät, wenn eine
    öffentliche URL bekannt ist.

Die Schritte 6 bis 10 werden unter Settings → Notifications & reports → Notification
policy (global) und bei jedem Kanal (Delivery options) konfiguriert. Siehe
[Benachrichtigungsrichtlinie](notifications.md).
