# Heartbeat (Push)

Der stille Fehler, den niemand bemerkt: das nächtliche Backup, das nach
einem Update aufgehört hat zu laufen, der Zertifikat-Erneuerungs-Cron,
der seine Rechte verlor, die Home-Assistant-Automatisierung, die bei
einer Umbenennung kaputtging. Nichts beschwert sich, und Sie finden es
an dem Tag heraus, an dem Sie das Backup brauchen.

Ein **Heartbeat**-Monitor (auch *Dead Man's Switch* genannt, oder ein
*Push*-Monitor in Uptime Kuma) dreht die Logik um: DumbMonit kontaktiert
nichts. Der Job ruft bei jedem Lauf eine geheime URL auf, und es ist
das **Fehlen** eines Aufrufs, das den Alarm auslöst.

## Wie es funktioniert

Jeder Heartbeat-Monitor erhält eine URL der Form
`https://<Ihr DumbMonit>/api/push/<token>`. Das Token ist ein
128-Bit-Zufallswert, zum Nachschlagen gehasht und zur Anzeige
verschlüsselt gespeichert: Es wird auf der Monitorseite angezeigt, mit
einem Kopier-Button, solange der Monitor existiert. Es ist absichtlich
offen — keine Sitzung, kein API-Token, kein `X-Requested-With`-Header
— weil eine `curl`-Zeile in einer Crontab nichts davon hat. Das Token
zu besitzen erlaubt nur zu sagen "der Job lief".

Bei jedem Scheduler-Durchlauf (das Intervall des Monitors, standardmäßig
60 s) liest der Collector die Zeit des letzten Aufrufs und schreibt:

| Situation | `probe_success` | Wirkung |
|---|---|---|
| Noch kein Aufruf empfangen | *(nichts geschrieben)* | Der Monitor zeigt **Waiting**. Kein Alarm: der Job ist noch nicht eingerichtet. |
| Letzter Aufruf innerhalb des erwarteten Intervalls + Kulanz | `1` | **Reporting**. |
| Letzter Aufruf älter als das erwartete Intervall + Kulanz | `0`, `reason="missed"` | **Down**; die Regel "Heartbeat missed" feuert nach zwei Minuten. |
| Letzter Aufruf sagte `status=down` | `0`, `reason="reported_down"` | **Down** sofort, egal was die Uhr sagt. |

Ein Aufruf wertet den Monitor auch sofort neu aus, daher löscht ein
wieder aufgenommener Job seinen Alarm, ohne auf den nächsten
Scheduler-Durchlauf zu warten.

Der Endpunkt antwortet mit `204` und leerem Body, wenn der Aufruf
verzeichnet wird, `404` für ein unbekanntes oder neu erzeugtes Token,
`400` für ein `status`, das weder `up` noch `down` ist, und `429`
(mit `Retry-After`) über sechzig Aufrufe pro Minute für ein Token
hinaus. Sowohl `GET` als auch `POST` werden akzeptiert; `HEAD`
funktioniert auch.

### Query-Parameter

| Parameter | Bedeutung |
|---|---|
| `status` | `up` (Standard) oder `down`: der Job meldet selbst einen Fehlschlag. |
| `msg` | Ein freies Erklärungswort, behalten (auf 500 Zeichen gekürzt) und auf der Monitorseite angezeigt. |

Dies sind die Parameter des Push-Monitors von Uptime Kuma: Ein dafür
geschriebenes Skript funktioniert unverändert.

## Einrichtung

Die Schritte unten sind die, die der Hinweis neben dem Formular zeigt.

1. Schreiben Sie in die Adresse ein kurzes Label für den Job
   ("nightly-backup", "certbot-renew"): Nichts wird kontaktiert, das
   Label muss nur unter Ihren Heartbeats eindeutig sein.
2. Setzen Sie das erwartete Intervall auf den Zeitplan des Jobs
   ("24h" für einen nächtlichen Job, "1h" für einen stündlichen) und,
   falls der Zeitplan abweicht, eine weitere Kulanzzeit. Speichern Sie
   den Monitor.
3. Die Monitorseite zeigt die aufzurufende URL. Fügen Sie sie am Ende
   des Jobs hinzu, sodass sie nur aufgerufen wird, wenn der Job
   gelang:

    ```
    curl -fsS -m 10 --retry 3 https://monit.example.com/api/push/<token>
    ```

4. Ein Job, der merken kann, dass er fehlgeschlagen ist, kann das
   sagen, statt still zu bleiben: Fügen Sie ?status=down&msg=… an die
   URL an, und der Alarm feuert sofort.
5. Bis der erste Aufruf eintrifft, zeigt der Monitor "Waiting", und
   nichts wird alarmiert. URL verloren oder durchgesickert?
   "Regenerate" auf der Monitorseite stellt eine neue aus; die alte
   hört sofort auf zu antworten.

!!! warning
    Rufen Sie die URL am Ende des Jobs auf, nach dem wichtigen Teil.
    Ein Aufruf am Anfang würde einen Erfolg melden, selbst wenn das
    Backup selbst fehlschlug.

Credentials: keine.

### Beispiele

Eine Crontab-Zeile, nächtlich um 03:00, die sich nur meldet, wenn das
Backup gelang:

```
0 3 * * * /usr/local/bin/backup.sh && curl -fsS -m 10 --retry 3 https://monit.example.com/api/push/<token> > /dev/null
```

Ein Skript, das seinen eigenen Fehlschlag meldet, mit dem Exit-Code als
Nachricht:

```sh
#!/bin/sh
URL="https://monit.example.com/api/push/<token>"
if /usr/local/bin/backup.sh; then
    curl -fsS -m 10 --retry 3 "$URL" > /dev/null
else
    curl -fsS -m 10 --retry 3 "$URL?status=down&msg=backup+exit+$?" > /dev/null
fi
```

Home Assistant, ein am Ende einer Automatisierung aufgerufener
`rest_command`:

```yaml
rest_command:
  dumbmonit_heartbeat:
    url: "https://monit.example.com/api/push/<token>"
    method: post
```

Windows Task Scheduler (PowerShell):

```powershell
Invoke-WebRequest -UseBasicParsing -TimeoutSec 10 "https://monit.example.com/api/push/<token>" | Out-Null
```

### Optionen

| Schlüssel | Label | Standard | Hilfe |
|---|---|---|---|
| `expected_interval` | Expected interval | `24h` | Wie oft der Job sich melden soll: 30m, 1h, 6h, 24h, 7d (oder eine Anzahl Sekunden). Ein verpasster Aufruf wird erklärt, sobald dieses Intervall plus die Kulanzzeit vergangen ist. |
| `grace` | Grace period | `10%` | Zusätzlich tolerierte Zeit nach dem erwarteten Intervall, bevor der Heartbeat als verpasst zählt: ein Prozentsatz des Intervalls (10 %) oder eine feste Dauer (15m). Nie weniger als eine Minute. |

Dauern akzeptieren die Suffixe `s`, `m`, `h`, `d`, `w` und können
kombiniert werden (`1h30m`); eine blanke Zahl ist eine Anzahl
Sekunden. Das erwartete Intervall muss zwischen 10 Sekunden und einem
Jahr liegen.

## Metriken

Alle mit `probe="push"` beschriftet, zusätzlich zu `target`, `host`
und `tag_*`, alle mit dem Präfix `dumbmonit_`.

| Metrik | Bedeutung |
|---|---|
| `probe_success` | 1, wenn der letzte Aufruf pünktlich war und keinen Fehlschlag meldete, sonst 0. Fehlt bis zum ersten Aufruf. |
| `probe_failure_info` | Vorhandensein (1) mit einem `reason`-Label: `missed` oder `reported_down`. |
| `push_last_seen_seconds` | Alter des letzten Aufrufs. |
| `push_received_total` | Seit Erstellung des Tokens empfangene Aufrufe (Zähler). |

Verfügbarkeit über dreißig Tage, wie für jeden Service-Monitor:
`avg_over_time(dumbmonit_probe_success{probe="push"}[30d])`.

Geltende eingebaute Regel: **Heartbeat missed** (Advisory, 2 Minuten).
Die Regeln "Service down" und "Service flapping" überspringen
Heartbeats, und "Monitor unreachable" feuert nie für sie: Der
Collector läuft immer, nur sein Urteil ändert sich.

## API

| Route | Sitzung | Zweck |
|---|---|---|
| `GET` oder `POST /api/push/<token>` | keine | Der Aufruf selbst. `204`, `404`, `400` oder `429`. |
| `GET /api/targets/<id>/push` | erforderlich | Der Monitor: `token`, `path`, `last_seen_at`, `last_seen_age_secs`, `last_status`, `last_message`, `received_total`, `expected_interval_secs`, `grace_secs`, `verdict` (`waiting`, `on_time`, `missed`, `reported_down`), `settings_error`. Beim ersten Lesen erstellt, falls fehlend. |
| `POST /api/targets/<id>/push/regenerate` | admin | Neues Token; die vorherige URL antwortet von da an mit `404`. Der Zähler und der letzte Aufruf werden behalten. |
