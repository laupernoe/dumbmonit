# Benachrichtigungsrichtlinie

DumbMonits Versprechen ist, dass eine Benachrichtigung es wert ist, gelesen
zu werden. Die Alarmierungs-Engine hält den Lärm niedrig, bevor ein Alarm
sprechen darf (Abhängigkeitsunterdrückung, Gruppierung, Wartungsfenster);
die Benachrichtigungsrichtlinie hält ihn danach niedrig, pro Kanal. Es muss
nichts konfiguriert werden: Die unten stehenden Standardwerte greifen von
Haus aus.

## Die Pipeline, in einer Liste

```
evaluate (hysteresis, per-device overrides)
  → deduplicate → suppress by dependency → silence → group by device
    → flap hold → channel severity filter → channel routing filter
      → resolved on/off → cooldown → quiet hours → batch window
        → hourly cap → send (+ deep link)
                                   ↘ nobody acknowledged → escalation channel
```

| Stufe | Wo eingestellt | Standard |
|---|---|---|
| Rückstellschwelle (Hysterese) | Regel-Editor → *Clear below / above* | keine (CPU: 85 %, Platte: 88 % bei den mitgelieferten Regeln) |
| Gerätespezifische Überschreibungen | Regel-Editor → *Per-device overrides* | keine |
| Flackererkennung | Settings → Notifications & reports → Notification policy → More options | 4 Änderungen in 30 Min, Sperre 30 Min |
| Mindest-Schweregrad | Kanal → Delivery options → *Send* | alles |
| Sag mir, wenn es sich löst | Kanal → Delivery options | ein |
| Mindestabstand pro Alarm | Kanal → Delivery options | keiner |
| Ruhezeiten | Kanal → Delivery options | aus |
| Routing-Filter (Tags, Art, Regel) | Kanal → Delivery options → *Only some alerts* | aus (der Kanal hört alles) |
| Eskalation, falls niemand bestätigt | Settings → Notifications & reports → Notification policy | aus |
| Bündelungsfenster | Settings → Notifications & reports → Notification policy | 60 s |
| Nachrichten pro Kanal und Stunde | Settings → Notifications & reports → Notification policy | 20 |
| Öffentliche URL (Deep Links) | Settings → Notifications & reports → Notification policy, oder `DUMBMONIT_PUBLIC_URL` | leer |

## Hysterese und gerätespezifische Überschreibungen

Eine Regel, die über 90 % feuert und sich in dem Moment löst, in dem der
Wert unter 90 % fällt, würde auf einer zwischen 89–91 % schwankenden CPU den
ganzen Abend feuern und sich lösen. Geben Sie ihr eine
**Rückstellschwelle**: Einmal feuernd, löst sich der Alarm erst, wenn der
Wert darunter fällt (für „feuert über“-Regeln) oder darüber steigt (für
„feuert unter“-Regeln). Das API-Feld ist `clear_threshold` auf der Regel;
der Server lehnt eine Rückstellschwelle auf der falschen Seite der Schwelle
ab.

Eine **gerätespezifische Überschreibung** ändert die Schwelle oder
Rückstellschwelle eines Geräts oder schaltet die Regel für dieses Gerät ab,
ohne die Regel zu duplizieren:

```
PUT /api/alerts/rules/{id}/overrides/{target_id}
{ "threshold": 97, "clear_threshold": 95 }      # oder { "enabled": false }
GET /api/alerts/rules/{id}/overrides
GET /api/alerts/overrides?target_id=7           # jede Überschreibung eines Geräts
DELETE /api/alerts/rules/{id}/overrides/{target_id}
```

Eine Überschreibung, bei der jedes Feld leer ist, wird entfernt.
Nachrichten zitieren die tatsächlich angewendete Schwelle.

## Flackererkennung

Unabhängig von der Regel ist ein Alarm, dessen Fingerprint innerhalb von
30 Minuten 4-mal gefeuert oder sich gelöst hat, *am Flattern*: Eine
Meldung geht raus ("flapping: 4 changes in 30 min, notifications held for
30 min") und nichts weiteres zu diesem Alarm wird gesendet, bis die Sperre
endet. Eine Behoben-Meldung nach der Sperre wird trotzdem angekündigt.
Anpassen oder abschalten unter Settings → Notifications & reports → Notification
policy → More options (`flap_events`, `flap_window_secs`, `flap_hold_secs`;
`flap_events: 0` deaktiviert es).

## Kanalspezifische Zustellungsoptionen

Jeder Kanal entscheidet, was er hört (`policy` in der Kanal-Payload):

- `min_severity` — `info`, `warning` (Advisory und höher) oder `critical`
  (nur Warning). Ein Telefon-Kanal will typischerweise `critical`, ein
  Chatraum alles.
- `notify_resolved` — `false`, um nur über Probleme zu hören.
- `min_interval_secs` — mindestens so lange zwischen zwei Nachrichten zum
  selben Alarm (erneutes Feuern und Erinnerungen). Behoben-Meldungen werden
  dadurch nie verzögert, und eine Behoben-Meldung wird nur an einen Kanal
  gesendet, der das Feuern gehört hat.
- `quiet_hours` — ein wöchentliches Fenster in Ihrer Lokalzeit, gleiche
  Form wie ein wöchentliches Wartungsfenster. Während der Ruhezeiten kommen
  nur `critical`-Alarme durch; der Rest wartet und kommt als eine
  Zusammenfassung an, wenn sie enden, mit dem Titel "Quiet hours over —
  …". Ein Alarm, der während der Ruhezeiten feuert und sich löst, wird in
  dieser Zusammenfassung nur als "resolved during quiet hours" erwähnt.
- `matcher` — welche Alarme der Kanal will, nach Tag, Geräteart oder Regel.
  Leer (der Standard, und was jeder bestehende Kanal behält) bedeutet
  alles.

```
PUT /api/notify/channels/{id}
{ "name": "Phone", "kind": "ntfy", "settings": {…},
  "policy": { "min_severity": "critical", "min_interval_secs": 900,
              "quiet_hours": { "kind": "weekly", "days": [0,1,2,3,4],
                               "start_minute": 1320, "end_minute": 420,
                               "timezone": "Europe/Paris" } } }
```

Wird `policy` bei einem Update weggelassen, bleibt die gespeicherte
erhalten; `"quiet_hours": null` löscht sie, und `"matcher": null` löscht
den Routing-Filter.

## Routing nach Tag

Geräte tragen Tags (`site=cellar`, `role=storage`) und eine Art (`snmp`,
`proxmox`, `synology`, `agent`…). Der **Routing-Filter** eines Kanals sagt,
welche Alarme er möchte:

```json
{ "policy": { "min_severity": "warning",
              "matcher": {
                "include": [ { "field": "tag", "key": "site", "value": "cellar" } ],
                "exclude": [ { "field": "tag", "key": "role", "value": "lab" } ] } } }
```

Drei Felder, und nur exakte Gleichheit — keine regulären Ausdrücke, keine
Klammern:

| `field` | Prüft gegen | Beispiel |
|---|---|---|
| `tag` | Ein Tag der Geräteseite selbst | `{"field":"tag","key":"site","value":"cellar"}` |
| `kind` | Der Collector, der das Gerät abfragt | `{"field":"kind","value":"proxmox"}` |
| `rule` | Die `uid` der Regel | `{"field":"rule","value":"disk_full"}` |

Wie die Listen zu lesen sind:

- Bedingungen auf **demselben** Feld (derselbe Tag-Schlüssel, oder `kind`,
  oder `rule`) sind ein **ODER** — `site=cellar` plus `site=attic` bedeutet
  entweder-oder;
- Bedingungen auf **unterschiedlichen** Feldern sind ein **UND** —
  `site=cellar` plus `role=storage` bedeutet beides;
- **`exclude` gewinnt immer**, auch über ein zutreffendes `include`.

Das obige Beispiel liest sich also, und die UI gibt es genau mit diesen
Worten wieder: *dieser Kanal erhält Advisory und höher von Geräten mit dem
Tag site=cellar, außer Geräten mit dem Tag role=lab*.

Unter dem Editor listet eine **Vorschau** auf, welche Ihrer aktuellen
Geräte der Filter auswählt. Es ist keine Näherung: Der Server führt genau
den Filter aus, den die Alarmierungs-Engine ausführt, reduziert auf den
Teil, der nur vom Gerät abhängt. Regelbedingungen gelten für Alarme statt
für Geräte, daher benennt die Vorschau sie gesondert, anstatt vorzugeben,
sie zu zählen.

```
POST /api/notify/match-preview
{ "matcher": { "include": [ { "field": "tag", "key": "site", "value": "cellar" } ] } }

→ { "devices": [ { "id": 3, "name": "nas", "kind": "synology",
                   "tags": { "site": "cellar" }, "matched": true } ],
    "matched": 1, "total": 12, "rules": [], "excluded_rules": [] }
```

Ein Filter ist auf zwölf Bedingungen begrenzt. Darüber hinaus ist ein
zweiter Kanal klarer als eine weitere Bedingung.

## Eskalation: falls niemand bestätigt

Ein zusätzlicher Hop, und nur einer. Wenn die Richtlinie
`escalate_after_secs` und `escalate_channel` setzt, wird ein Alarm, den
niemand innerhalb dieser Verzögerung
[bestätigt](../using/alerts.md) hat, zusätzlich — einmal — an diesen
Kanal gesendet:

```
PUT /api/notify/policy
{ "escalate_after_secs": 900, "escalate_channel": 4 }
```

- Die Verzögerung wird **vom Moment an gezählt, in dem die erste Nachricht
  tatsächlich rausging**, nicht vom Feuern des Alarms. Wenn Ruhezeiten oder
  ein fehlgeschlagener Versand die erste Benachrichtigung verschluckt
  haben, gibt es noch nichts zu eskalieren, und der Hop wartet.
- **Das Bestätigen des Alarms stoppt ihn**, genau wie es Erinnerungen
  stoppt; ebenso das Lösen des Alarms.
- Die Eskalationsnachricht geht **nur** an den Eskalationskanal — Kanäle,
  die das Feuern schon gehört haben, werden nicht doppelt informiert.
- Sie überschreitet die eigene Schweregrad-Untergrenze, den Routing-Filter
  und die Ruhezeiten dieses Kanals: Sie haben diesen Kanal benannt, um
  genau in diesem Moment geweckt zu werden, daher wird er nicht
  zurückgehalten.
- Die Zeile sagt es: `🔴 Critical · Disk full — /data — 95 % … — still
  unacknowledged`.

`"escalate_channel": null` schaltet die Eskalation aus; eine Verzögerung
von `0` tut dasselbe. Beide sind nötig, damit der Hop existiert, und der
Server lehnt einen nicht existierenden Kanal ab, statt ins Leere zu
eskalieren.

DumbMonit hört hier absichtlich auf: ein Hop, keine Rotation, kein
Bereitschaftsplan für diese Woche. Wenn Sie das brauchen, zeigen Sie den
Eskalationskanal auf PagerDuty oder Opsgenie, die das richtig machen.

## Bündelung und das Stundenlimit

`GET/PUT /api/notify/policy` hält die globalen Einstellungen:

```json
{ "batch_window_secs": 60, "max_per_hour": 20,
  "flap_events": 4, "flap_window_secs": 1800, "flap_hold_secs": 1800,
  "public_url": "https://monit.example.lan",
  "escalate_after_secs": 0, "escalate_channel": null }
```

- **Bündelungsfenster** — Alarme, die innerhalb des Fensters feuern,
  verlassen das System als *eine* Nachricht pro Kanal, gruppiert nach
  Gerät: "3 alerts on 2 devices" mit einem Abschnitt pro Gerät;
  Behoben-Meldungen fahren mit ("2 resolved"). `0` sendet beim nächsten
  Zyklus. Eine Zusammenfassung listet höchstens 12 Zeilen, dann "…and N
  more alerts".
- **Stundenlimit** — Sobald ein Kanal in der letzten Stunde
  `max_per_hour` Nachrichten erhalten hat, warten weitere Alarme und
  kommen als eine Zusammenfassung an, wenn ein Platz frei wird. `0` hebt
  das Limit auf.
- **On-Call-Kanäle** (PagerDuty, Opsgenie) werden nie über Geräte hinweg
  gebündelt: Sie öffnen und schließen einen Vorfall pro Gerät, erhalten
  also eine Nachricht pro Gerät, ohne zu warten.

Fehlgeschlagene Zustellungen halten die Zeilen in der Warteschlange; der
nächste Zyklus versucht es erneut.

## Was eine Zeile sagt

Jeder Alarm ist eine Zeile, dieselbe in Mail, ntfy, Chat und der
`{{message}}`-Variable eines benutzerdefinierten Webhooks:

```
🔴 Critical · Disk full — /data — 95 % (threshold > 90 %), for 12 min
⚠️ Warning · VM or container stopped — win11-desktop (101), for 59 s
✅ Resolved · Service down — https://example.lan/
```

- Dem Symbol folgt das Zustandswort (Critical / Warning / Info, Resolved,
  Flapping), sodass der Schweregrad ohne Farbe oder Emoji lesbar ist.
- Eine Regel, die mehrere Zeitreihen beobachtet, benennt diejenige, die
  gefeuert hat: die VM und ihre ID, den Einhängepunkt, die Schnittstelle,
  die Backup-Gruppe, die URL der Prüfung. Geräteweite Regeln
  (unreachable) benennen nichts weiter als das Gerät.
- Wert und Schwelle werden bei gemessenen Regeln angezeigt; eine
  Alles-oder-nichts-Regel auf einer 0/1-Metrik (im Akkubetrieb, Gast
  gestoppt) wiederholt nicht "1 (threshold > 0)".

## Deep Links

Jede Nachricht endet mit einem Link zum Gerät
(`{public_url}/targets/{id}`), wenn eine öffentliche URL bekannt ist — die
`public_url` der Richtlinie, oder `DUMBMONIT_PUBLIC_URL`.
Benutzerdefinierte Webhooks erhalten ihn ebenfalls als `{{link}}`-Variable,
sofern nicht ihre eigene `base_url` gesetzt ist.

Es gibt absichtlich keinen "Silence for 1 h"-Link in Nachrichten:
Chat-Dienste rufen jeden angezeigten Link ab, um eine Vorschau zu bauen,
was die Stummschaltung in dem Moment auslösen würde, in dem die Nachricht
gepostet wird. Stummschalten stattdessen von der Geräteseite oder der
Alerts-Seite aus.

## Was von Pulse übernommen wurde, und was nicht

Das Design folgt [Pulse](https://github.com/rcourtman/Pulse), wo es zu
einem Homelab-Monitor passt: Auslöse-/Rückstellschwellen, eine
ressourcenspezifische Überschreibungsebene, ein Benachrichtigungs-Cooldown,
ein Gruppierungsfenster, ein stündliches Ratenlimit, Ruhezeiten,
Flackererkennung mit Cooldown, "notify on resolve", und On-Call-Werkzeuge
außerhalb von Zusammenfassungen gehalten. Nicht übernommen: Pulses
24-Stunden-Beobachtungszeitraum vor der ersten Benachrichtigung (DumbMonits
Baseline-Regel lernt bereits stillschweigend; Schwellenwertregeln sollten
vom ersten Tag an sprechen) und Snooze/Acknowledge direkt aus der
Nachricht (siehe oben). Tag-basiertes Routing pro Ziel *wurde* übernommen,
als der obige Routing-Filter — Kanäle an der Regel auszuwählen, erwies
sich als der falsche Ort für "alles im Keller geht in diesen Raum".
