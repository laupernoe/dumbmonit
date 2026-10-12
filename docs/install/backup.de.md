# Backup und Wiederherstellung

Zwei verschiedene Dinge, für zwei verschiedene Unglücksfälle.

| | Verschlüsseltes Bündel | Geplantes lokales Backup |
|---|---|---|
| Was es hält | Die Konfiguration: Geräte und ihre Zugangsdaten, Regeln, Kanäle, Statusseiten, Konten, Tokens. | Die gesamte Datenbank, als eine Datei, plus `secret.key`. |
| Wohin es geht | Wohin auch immer Sie es legen — verschlüsselt mit einer von Ihnen gewählten Passphrase. | `/data/backups/`, innerhalb desselben Volumes. |
| Wovor es Sie bewahrt | Ein verlorener Server, ein Umzug auf neue Hardware, eine frische Installation. | Die Änderung, die Sie zehn Minuten später bereuen, ein schiefgegangenes Upgrade. |
| Was es nicht hält | Metrikhistorie, Alarmzustand und -historie, das Audit-Log, Sitzungen. | Metrikhistorie (`vm/`). |

Keines ersetzt ein Archiv des gesamten Volumes, wenn Sie auch die
Diagramme zurückwollen; das steht unten auf dieser Seite.

!!! danger "`/data/secret.key` ist, was Ihre Gerätezugangsdaten entschlüsselt"
    SNMP-Communities, Proxmox-Tokens, SMTP-Passwörter und
    Heartbeat-URLs werden mit einem vom Instanz-Secret abgeleiteten
    Schlüssel verschlüsselt gespeichert. Beim ersten Start schreibt
    DumbMonit ihn nach `/data/secret.key` (oder liest ihn aus
    `DUMBMONIT_SECRET`, falls Sie einen gesetzt haben).

    **Eine Kopie der Datenbank ohne diese Datei stellt eine Instanz
    wieder her, die mit nichts sprechen kann.** Der Server bemerkt
    das und verweigert den Start mit einer expliziten Meldung, statt
    bei jeder Abfrage zu scheitern — aber die Zugangsdaten sind weg,
    und jedes Gerät muss neu eingegeben werden.

    Das verschlüsselte Bündel ist die eine Ausnahme: seine Secrets
    werden mit Ihrer Passphrase neu verschlüsselt, sodass es auf
    einer frischen Instanz wiederherstellt, die ihr eigenes,
    brandneues Secret hat.

## Das verschlüsselte Bündel

**Settings → Backup → Download the bundle.** Wählen Sie eine
Passphrase mit mindestens 16 Zeichen — drei Wörter reichen — und
bestätigen Sie sie. Die herunterkommende Datei ist einfaches JSON mit
einem verschlüsselten Body:

```json
{
  "format": "dumbmonit-backup",
  "version": 1,
  "created_at": "2026-09-22 19:04:11",
  "source_version": "0.1.0-alpha.3",
  "summary": { "targets": 12, "channels": 2, "rules": 41 },
  "kdf": { "algorithm": "argon2id", "salt": "…", "m_cost": 32768, "t_cost": 3, "p_cost": 1 },
  "cipher": "aes-256-gcm",
  "payload": "…"
}
```

Der Header bleibt lesbar, sodass Sie sagen können, was eine Datei ist,
bevor Sie irgendetwas eintippen. Alles andere — Adressen
eingeschlossen — liegt innerhalb von `payload`, mit AES-256-GCM
verschlüsselt, unter einem von Ihrer Passphrase durch Argon2id
abgeleiteten Schlüssel. Es gibt **keine Wiederherstellung**: Ein
Bündel, dessen Passphrase verloren ist, ist verloren.

!!! warning "Das Bündel hält jedes Credential dieser Instanz"
    Jeder mit der Datei und der Passphrase kann das Passwort jedes
    von Ihnen überwachten Geräts lesen. Bewahren Sie es auf, wo Sie
    Passwörter aufbewahren.

### Was drin ist

| Abschnitt | Inhalt |
|---|---|
| `targets` | Geräte: Name, Art, Adresse, Intervall, Optionen, übergeordnetes Gerät, Relay-Agent, und das Credential. |
| `rules` | Alarmregeln, einschließlich eingebauter, die Sie angepasst oder deaktiviert haben. |
| `rule_overrides` | Gerätespezifische Schwellenwerte und Ausschlüsse. |
| `channels` | Benachrichtigungskanäle, ihre Einstellungen, ihre Pro-Kanal-Richtlinie und ihre Secrets. |
| `notify_policy` | Die globale Benachrichtigungsrichtlinie (Gruppierung, Stundenlimit, Flattern, öffentliche URL). |
| `silences` | Wartungsfenster. |
| `status_pages` | Öffentliche Statusseiten und die von jeder gelisteten Dienste. |
| `incidents` | Vorfälle und Wartungsankündigungen, mit ihren Updates. |
| `users` | Konten: Benutzername, Anzeigename, Rolle, SSO-Verknüpfung, aktiviert oder nicht. |
| `agent_tokens` | Agent-Enrollment-Tokens, als Hashes. Installierte Agents pushen nach einer Wiederherstellung weiter. |
| `api_tokens` | API-Tokens, als Hashes. Bestehende Tokens funktionieren weiter. |
| `push_monitors` | Heartbeat-Tokens, sodass die URLs, die Ihre Cron-Jobs schon aufrufen, gleich bleiben. |

Geräte werden nach **Art + Adresse** abgeglichen, was auch der
eindeutige Schlüssel der Tabelle ist: Das zweimalige Importieren
desselben Bündels erstellt nie eine zweite Kopie von irgendetwas.

### Kontopasswörter

Standardmäßig werden Konten **ohne** ihren Passwort-Hash,
TOTP-Secret oder Recovery-Codes exportiert. Wiederhergestellte
Konten existieren dann mit dem richtigen Namen und der richtigen
Rolle, aber niemand kann sich mit ihnen einloggen, bis ein
Administrator ein Passwort setzt — oder bis sie sich über SSO
einloggen, was nichts aus dem Bündel braucht.

Schalten Sie **Include account passwords and 2FA secrets** ein, wenn
Sie eine Instanz verschieben, statt ihre Konfiguration zu kopieren.
Ein Argon2-Hash in einer reisenden Datei kann offline angegriffen
werden, solange der Angreifer will; der Standard ist der sichere.

### Was nicht drin ist

Metrikhistorie (die in VictoriaMetrics lebt, nicht in der Datenbank),
Alarmzustand und -historie, Anomalie-Baselines, das Audit-Log, offene
Sitzungen, die Registrierung und letzte Sichtung von Agents, und die
Abfrage-Caches der Proxmox- und Synology-Panels. Das sind
Beobachtungen, keine Einstellungen: Sie kommen von selbst zurück.

Und **nicht das Instanz-Secret**. Das ist der Punkt: Das Bündel
trägt seine Secrets neu mit Ihrer Passphrase verschlüsselt, genau
damit es irgendwo wiederherstellen kann, wo `secret.key` nie gesehen
wurde.

### Auf einer frischen Instanz wiederherstellen

1. Starten Sie den neuen Container und erstellen Sie das erste
   Administrator-Konto unter `/setup`, mit dem in seinen Logs
   ausgegebenen Setup-Code. Dieses Konto ist Ihres; die
   Wiederherstellung rührt es nicht an.
2. **Settings → Backup → Restore a bundle.** Wählen Sie die Datei,
   geben Sie die Passphrase ein, und drücken Sie **Check this
   backup**.
3. Lesen Sie den Bericht. Er sagt, Abschnitt für Abschnitt, was
   **erstellt** würde, was **aktualisiert** würde, weil es vom
   bereits Vorhandenen abweicht, und was bereits identisch ist — plus
   eine Notiz für alles, was nicht platziert werden konnte, etwa eine
   Regel, die auf ein Gerät zeigte, das das Bündel nicht enthält.
4. Drücken Sie **Restore for real**.

Drei Regeln, die die Wiederherstellung nie bricht:

- **Nichts wird je gelöscht.** Eine Wiederherstellung lässt das
  existieren, was das Bündel beschreibt; alles, was die Instanz
  zusätzlich hat, bleibt.
- **Nichts wird je dupliziert.** Jedes Element wird über seinen
  natürlichen Schlüssel abgeglichen — Art + Adresse für ein Gerät,
  `uid` für eine Regel, Name für einen Kanal, Slug für eine
  Statusseite. Dasselbe Bündel zweimal wiederherzustellen meldet
  alles als bereits identisch und schreibt nichts.
- **Bestehende Konten bleiben unberührt.** Ein Konto, dessen
  Benutzername schon hier ist, wird übersprungen, nie überschrieben:
  Eine Datei darf nicht die Rolle oder das Passwort des
  Administrators ändern können, der die Wiederherstellung ausführt.

Ein Dry Run ist keine Simulation der Wiederherstellung, er *ist* die
Wiederherstellung, innerhalb einer am Ende zurückgerollten
Transaktion ausgeführt. Der Bericht kann nicht falsch darüber liegen,
was die Anwendung tun würde.

Ein von einem neueren DumbMonit geschriebenes Bündel wird mit einem
entsprechenden Satz abgelehnt — erst aktualisieren, dann
wiederherstellen. Ein verändertes Bündel, oder eines mit falscher
Passphrase, wird ebenfalls abgelehnt: AES-GCM kann die beiden nicht
unterscheiden, und die Meldung auch nicht.

### Von der Kommandozeile

Beide Wege brauchen eine Administrator-**Sitzung**, kein API-Token:
Ein Token lebt in einem Skript, und ein Skript hat keinen Grund, jedes
Credential der Instanz zu exportieren.

```bash
# Sign in
curl -s -c jar -X POST http://localhost:8080/api/auth/login \
  -H 'content-type: application/json' -H 'X-Requested-With: DumbMonit' \
  -d '{"username":"admin","password":"…"}'

# Export
curl -s -b jar -X POST http://localhost:8080/api/backup \
  -H 'content-type: application/json' -H 'X-Requested-With: DumbMonit' \
  -d '{"passphrase":"three words are enough"}' -o backup.json

# Dry run, then apply
jq -n --slurpfile b backup.json \
  '{bundle: $b[0], passphrase: "three words are enough", apply: false}' > restore.json
curl -s -b jar -X POST http://localhost:8080/api/backup/restore \
  -H 'content-type: application/json' -H 'X-Requested-With: DumbMonit' \
  -d @restore.json | jq
```

## Geplante lokale Backups

DumbMonit schreibt nach Zeitplan eine Kopie seiner eigenen Datenbank,
standardmäßig **einmal täglich, die letzten sieben behaltend**, nach
`/data/backups/`:

```
/data/backups/dumbmonit-20260922-030000.db
/data/backups/dumbmonit-20260922-030000.key
```

Die `.db`-Datei wird mit SQLites `VACUUM INTO` erzeugt: eine Online-,
konsistente Kopie, geschrieben, während der Server weiterläuft, keine
Dateikopie, die die Datenbank mitten im Schreiben erfassen könnte.
`sqlite3` öffnet sie, wie sie ist, ohne ein Journal nachzuspielen.
Das `.key` daneben ist die Kopie von `secret.key` — zusammen reicht
das Paar, um die Instanz wiederaufzubauen.

Sie leben im selben Volume wie die Datenbank, daher machen sie einen
Fehler ungeschehen, keine verlorene Platte. Kopieren Sie sie von der
Maschine weg, wenn Sie mehr als das wollen.

### Einstellungen

| Variable | Standard | Rolle |
|---|---|---|
| `DUMBMONIT_BACKUP_ENABLED` | `1` | `0`, `false`, `no` oder `off`, um keine geplanten Backups mehr zu schreiben. |
| `DUMBMONIT_BACKUP_DIR` | `<data dir>/backups` | Wohin sie geschrieben werden. Richten Sie es auf ein zweites Volume, um das erste zu überstehen. |
| `DUMBMONIT_BACKUP_INTERVAL_HOURS` | `24` | Stunden zwischen zwei Backups. Das erste passiert eine Periode nach dem Start, nicht beim Start. |
| `DUMBMONIT_BACKUP_KEEP` | `7` | Wie viele behalten werden. Die ältesten werden entfernt, mit ihrem `.key`. |

Nichts sonst im Verzeichnis wird je angerührt: Nur Dateien namens
`dumbmonit-<timestamp>.db` und ihr `.key` werden rotiert.

Wenn das Instanz-Secret aus `DUMBMONIT_SECRET` kommt, gibt es keine
Datei zu kopieren, und keine wird geschrieben — behalten Sie diesen
Wert stattdessen in Ihrem Passwort-Manager. Der Backup-Abschnitt sagt,
in welchem der beiden Fälle Sie sind.

### Wissen, dass es lief

Jeder Lauf veröffentlicht drei Zeitreihen:

| Metrik | Bedeutung |
|---|---|
| `dumbmonit_instance_backup_last_success_seconds` | Unix-Zeitstempel des letzten **erfolgreichen** Backups. Ein fehlgeschlagener Versuch aktualisiert ihn nie. |
| `dumbmonit_instance_backup_last_status` | `1`, wenn der letzte Versuch gelang, `0`, wenn er fehlschlug. |
| `dumbmonit_instance_backup_size_bytes` | Größe der letzten geschriebenen Datei. |

Die eingebaute Regel **DumbMonit backup did not run**
(`instance_backup_missing`) feuert, wenn der letzte Erfolg mehr als
zwei Tage alt ist — eine verpasste Nacht kann ein Neustart sein, zwei
sind ein Problem. Die Zeitreihe existiert erst, sobald ein erstes
Backup geschrieben wurde, daher wird eine Instanz mit ausgeschaltetem
Zeitplan nie deshalb genervt.

**Settings → Backup** zeigt dasselbe ohne MetricsQL: den letzten Lauf
und ob er gelang, die behaltenen Dateien und ihre Größe, und **Back
up now**, um sofort eines zu schreiben.

### Eines wiederherstellen

Stoppen Sie den Stack, legen Sie die Datei unter dem Namen der
Datenbank zurück, und stellen Sie den Key daneben wieder her:

```bash
docker compose stop
docker run --rm -v dumbmonit-data:/data alpine sh -c '
  cp /data/backups/dumbmonit-20260922-030000.db /data/dumbmonit.db &&
  cp /data/backups/dumbmonit-20260922-030000.key /data/secret.key &&
  rm -f /data/dumbmonit.db-wal /data/dumbmonit.db-shm'
docker compose start
```

Das Entfernen der `-wal`- und `-shm`-Dateien ist wichtig: Sie gehören
zur gerade ersetzten Datenbank, und SQLite würde versuchen, sie auf
die neue anzuwenden.

Um ein Backup zu lesen, ohne es wiederherzustellen, kopieren Sie es
heraus und öffnen Sie es mit `sqlite3`:

```bash
docker run --rm -v dumbmonit-data:/data -v "$PWD:/out" alpine \
  cp /data/backups/dumbmonit-20260922-030000.db /out/
sqlite3 dumbmonit-20260922-030000.db 'SELECT name, kind, address FROM targets;'
```

Zugangsdaten in dieser Datei bleiben ohne das passende `secret.key`
unlesbar.

## Das gesamte Volume archivieren

Der einzige Weg, auch die Diagramme zu behalten. Ein benanntes
Volume, `dumbmonit-data`, hält alles: die Datenbank, das Secret, die
lokalen Backups und die VictoriaMetrics-Daten.

```bash
docker compose stop
docker run --rm -v dumbmonit-data:/data -v "$PWD:/backup" alpine \
  tar czf /backup/dumbmonit-data.tgz -C /data .
docker compose start
```

Zum Wiederherstellen erstellen Sie das Volume, extrahieren Sie das
Archiv genauso darin, dann `docker compose up -d`.

Wenn Platz wichtig ist, hält `--exclude=./vm` das Archiv klein: Die
Datenbank und das Secret sind das Setup, `vm/` ist nur die Historie.

## Was vor einem Upgrade zu tun ist

1. **Settings → Backup → Back up now** — dreißig Sekunden, und es
   schließt das Secret ein.
2. Laden Sie auch ein Bündel herunter, wenn das Upgrade groß ist,
   oder wenn Sie auf eine andere Maschine umziehen.
3. `docker compose pull && docker compose up -d`. Migrationen laufen
   beim Start.

Geht etwas schief, [stellen Sie das lokale Backup wieder
her](#restoring-one) und starten Sie den vorherigen Image-Tag.
