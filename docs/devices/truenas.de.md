# TrueNAS

ZFS-Storage-Server (TrueNAS SCALE, jetzt TrueNAS Community Edition):
die Gesundheit jedes Pools und der ausgefallenen Platte, Scrubs und
Resilvers, Pool- und Dataset-Nutzung gegen ihre Kontingente, Snapshots
und Replikation, Plattentemperatur und SMART-Selbsttests, die von
TrueNAS selbst gehobenen Alarme, Dienste und die Maschine.

!!! warning "Nur gegen eine Testinstanz getestet"

    Diese Integration wurde gegen TrueNAS 25.04.2.6 gebaut und
    verifiziert, von der offiziellen ISO zu diesem Zweck in einer
    virtuellen Maschine installiert — ein RAIDZ1-Pool, Datasets,
    Snapshot- und Replikationstasks, dann derselbe Pool mit einer
    offline genommenen Platte — nicht gegen ein NAS, das seit Jahren
    die Fotos eines Haushalts hält. Eine virtuelle Maschine hat keine
    Plattentemperaturen und keine SMART-Ergebnisse, daher folgen diese
    TrueNAS' Quellcode statt einer beobachteten Antwort. Jeder Aufruf
    ist ein Lesevorgang. Sagen Sie uns, was kaputt ist.

## Der Fehler, für den das ist

Ein Mirror- oder RAIDZ-Vdev, das eine Platte verliert, liefert die
Daten weiter. Die Freigabe bleibt eingehängt, Backups laufen weiter,
die Web-Oberfläche jedes anderen Tools bleibt grün — und der Pool hat
nun keine Redundanz mehr. Die zweite Platte nimmt den Pool mit.
`truenas_pool_healthy` fällt auf 0 bei der ersten, die Regel **NAS
pool degraded** feuert, und die Geräteseite benennt die Platte.

## Die Geräteseite

Über die generischen Diagramme hinaus zeigt ein TrueNAS-Gerät drei
Panels, aus dem gelesen, was die Abfrage gespeichert hat — das Öffnen
der Seite fragt das NAS selbst nie ab:

* **Pools and disks** — jeder Pool mit seinem Zustand als Wort, ZFS'
  eigenem Satz, wenn etwas nicht stimmt, wie voll er ist, sein Layout
  (Mirror, RAIDZ, Cache, Log), seine Lese-, Schreib- und
  Prüfsummenfehlerzahlen, und vor allem der Name jeder Platte, die
  nicht `ONLINE` ist. Ein laufender Scrub oder Resilver zeigt seinen
  Fortschritt. Dann jede Platte: Modell, Seriennummer, Art, Größe,
  Pool, Temperatur und das Ergebnis ihres letzten SMART-Selbsttests.
* **Datasets and protection** — jedes Dataset mit seiner Nutzung
  gegen sein Kontingent, seiner Snapshot-Anzahl und wann ein
  Snapshot-Task es zuletzt abdeckte; der letzte Scrub jedes Pools und
  ob er überfällig ist; jeder Replikations- und periodische
  Snapshot-Task mit seinem Zustand, seinem letzten Lauf, dem letzten
  von ihm behandelten Snapshot und, wenn er fehlschlug, TrueNAS'
  eigenem Fehlersatz.
* **NAS health** — die von TrueNAS selbst gehobenen Alarme, am
  schwersten zuerst, dann die gestoppten Dienste, dann die Maschine:
  Version, Uptime, Last, Speicher, CPU.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_truenas_`.

| Familie | Metriken | Labels |
|---|---|---|
| Identität | `version_info` (die Version, ohne das `TrueNAS-SCALE-`-Präfix, das ältere Releases voranstellen) | `version` |
| Maschine | `uptime_seconds`, `load1/5/15`, `memory_total_bytes`, `cpu_count` | |
| Pools | `pool_healthy` (ZFS' eigenes Urteil: 0 bei `DEGRADED` wie bei `FAULTED`), `pool_warning` (Resilvering, Features nicht aktiviert), `pool_status_info`, `pool_size/allocated/free_bytes`, `pool_used_percent`, `pool_fragmentation_percent` | `pool`, und `status` für die Info-Zeitreihe |
| Pool-Geräte | `pool_device_errors` (kumuliert seit dem letzten `zpool clear`), `pool_devices_unhealthy` (beide fehlen, wenn der Pool nicht geöffnet werden kann) | `pool`, `kind` (`read`, `write`, `checksum`) |
| Scrubs | `pool_scan_running`, `pool_scan_percent` (nur während des Laufens), `pool_last_scrub_age_seconds`, `pool_last_scrub_errors`, `pool_scrub_threshold_days` | `pool`, `function` (`SCRUB`, `RESILVER`) |
| Datasets | `dataset_used_bytes`, `dataset_available_bytes`, `dataset_quota_bytes` und `dataset_quota_used_percent` (fehlt ohne Kontingent), `dataset_snapshots`, `dataset_snapshot_age_seconds` (Alter des letzten von einem das Dataset abdeckenden periodischen Task erstellten Snapshots; fehlt, wenn keiner das tut), `dataset_locked` (nur verschlüsselte Datasets) | `dataset`, `pool` |
| Snapshots | `snapshots` (gesamt) | |
| Platten | `disk_temperature_celsius`, `disk_size_bytes`, `disk_smart_failed` (fehlt für eine nie getestete Platte) | `disk`, `serial` |
| Alarme | `alerts` — die Anzahl aktiver Alarme pro Stufe, null eingeschlossen | `level` (`INFO` … `EMERGENCY`) |
| Tasks | `replication_error`, `replication_last_run_age_seconds`, `snapshot_task_error`, `snapshot_task_last_run_age_seconds` (nur aktivierte Tasks) | `task` |
| Dienste | `service_running` (nur zum Start mit dem NAS vorgesehene Dienste) | `service` |
| Sammlung | `up`, `scrape_errors`, `scrape_duration_seconds` | |

Ein paar Dinge, die TrueNAS tut und wissenswert sind:

* **Der letzte Scrub lebt nur im Scan-Datensatz des Pools.** Es gibt
  keine Scrub-Historie in der API, und ein Resilver überschreibt den
  Scan-Datensatz: Nach einem Resilver ist das Datum des letzten Scrubs
  einfach unbekannt, bis der nächste fertig ist, und
  `pool_last_scrub_age_seconds` fehlt statt falsch zu sein.
* **Ein Pool, der nicht geöffnet werden kann** — exportiert, oder mit
  zu vielen fehlenden Platten — erscheint trotzdem, ohne Layout und
  ohne Größen. Sein `pool_healthy` ist 0; die Geräte- und
  Größen-Zeitreihen fehlen statt null zu sein.
* **Ein nicht gesetztes Kontingent ist kein Kontingent von null.**
  TrueNAS antwortet mit `parsed: null` für "kein Kontingent", und
  DumbMonit veröffentlicht für dieses Dataset keine Kontingent-
  Zeitreihe. `quota` hat Vorrang vor `refquota`, wenn beide gesetzt
  sind.
* **Snapshot-Zählungen kommen aus ZFS' eigenem Cache**
  (`snapshots_count`), und die Gesamtzahl aus TrueNAS' schneller
  Snapshot-Zählung — kein Snapshot wird je aufgelistet. Die Gesamtzahl
  ist größer als die Summe der Pro-Dataset-Zählungen: Sie schließt die
  Snapshots von TrueNAS' internen Datasets ein, die die
  Dataset-Liste verbirgt.
* **Das Alter des neuesten Snapshots eines Datasets** ist der letzte
  Lauf des periodischen Snapshot-Tasks, der es abdeckt, direkt oder
  über ein rekursives übergeordnetes Dataset. TrueNAS kann das Datum
  des neuesten Snapshots nicht geben, ohne sie alle aufzulisten, daher
  hat ein von keinem Task abgedecktes Dataset kein Alter — was selbst
  wissenswert ist.
* **Temperaturen werden aus TrueNAS' Cache gelesen** (`only_cached`),
  alle fünf Minuten oder so von TrueNAS aktualisiert. Eine schlafende
  Platte wird nie geweckt, und eine Platte ohne Messwert hat keine
  Zeitreihe.
* **Nichts wird je gestartet.** Kein Scrub, kein SMART-Test, kein
  Update-Check, kein Replikationslauf. Die Abfrage liest.
* **Pfade, die sich zwischen Versionen änderten, sind optional.**
  `zfs/snapshot` wurde in 25.10 zu `pool/snapshot`, und beide werden
  versucht; `smart/test/results` verschwand in 25.10 und wird dann
  still übersprungen.

Geltende eingebaute Regeln: Device unreachable, NAS pool degraded,
NAS disk errors, NAS pool almost full, NAS scrub found errors, NAS
scrub overdue, NAS disk failed its SMART test, NAS disk too hot, NAS
dataset near its quota, NAS replication failed, NAS snapshot task
failed, NAS snapshots stale, TrueNAS alert raised, NAS service
stopped. Benachrichtigungen benennen den betroffenen Pool, die
Platte, das Dataset, den Task oder den Dienst.

## Was in TrueNAS vorzubereiten ist

Die Schritte unten sind die, die der Hinweis neben dem Formular zeigt.
Eine Sache zuerst zu wissen: Über die REST-API, die DumbMonit nutzt,
**hat TrueNAS keinen nur-lesenden Key**. Seine nur-lesenden Rollen
wurden nur in seine neuere WebSocket-API eingebaut; ein Key, dessen
Benutzer ein Read-Only Administrator ist, wird mit einem leeren `403`
auf jedem REST-Aufruf abgelehnt. Das wurde auf TrueNAS 25.04.2.6
geprüft: ein Read-Only-Administrator-Key erhielt `403` auf allen 55
versuchten REST-Pfaden, und derselbe Key funktionierte über die
WebSocket-API. Der Key muss zu einem Benutzer mit dem Recht Local
Administrator gehören — daher erhält er einen eigenen Benutzer und
eine eigene Gruppe, und nichts sonst.

1. In der Web-Oberfläche: Credentials → Groups → Add. Benennen Sie die
   Gruppe wie folgt und geben Sie ihr das Recht Local Administrator.

    ```
    dumbmonit
    ```

2. Warum Local Administrator und nicht Read-Only Administrator:
   TrueNAS hat seine nur-lesenden Rollen nur in seine WebSocket-API
   eingebaut. Über die von DumbMonit genutzte REST-API wird ein
   nur-lesender Key auf jedem einzelnen Aufruf abgelehnt. DumbMonit
   selbst liest immer nur: Es startet nie einen Scrub, einen
   SMART-Test, ein Update oder eine Replikation.

3. Credentials → Users → Add. Benennen Sie das Konto wie folgt, machen
   Sie dumbmonit zu seiner primären Gruppe, und lassen Sie Shell-
   Zugriff, Sudo und SSH aus: Keines davon ist nötig.

    ```
    dumbmonit
    ```

4. Erstellen Sie den Key: Credentials → Users → API Keys → Add,
   wählen Sie den Benutzer dumbmonit und geben Sie dem Key einen
   Namen. TrueNAS zeigt den Key nur einmal: Kopieren Sie ihn. Auf
   TrueNAS 24.10 ist der Pfad das Benutzermenü oben rechts → API Keys
   → Add, und ein Key gehört keinem Benutzer.

5. Geben Sie in DumbMonit die NAS-Adresse ein, zum Beispiel
   "nas.lan", und fügen Sie den Key ein. Behalten Sie HTTPS: TrueNAS
   widerruft einen Key, den es je über einfaches HTTP von einer
   anderen Maschine empfängt.

!!! warning

    Dieser Key kann alles auf dem NAS ändern: TrueNAS bietet keinen
    nur-lesenden Key über seine REST-API. Behandeln Sie ihn wie das
    Passwort eines Kontos, das Ihre Pools löschen kann, behalten Sie
    ihn exklusiv für DumbMonit, und widerrufen Sie ihn von derselben
    Seite, falls er durchsickert. TrueNAS liefert auch ein
    selbstsigniertes Zertifikat: Wird die Verbindung deshalb
    abgelehnt, kreuzen Sie "Accept an unverifiable certificate" in
    den Optionen an.

Adresse: `192.168.1.20`, `nas.lan`, `nas.lan:8443`, `[fd00::20]` oder
eine vollständige URL. Das Schema `https` und Port 443 werden
hinzugefügt, falls fehlend.

TrueNAS hat angekündigt, dass die REST-API in TrueNAS 26 entfernt
wird, zugunsten der WebSocket-API. Bis dahin ist REST, was jedes
Release bis 25.10 bedient.

## Optionen

| Schlüssel | Label | Standard | Hilfe |
|---|---|---|---|
| `scheme` | Protocol | `https` | Behalten Sie HTTPS: TrueNAS widerruft einen über einfaches HTTP von einer anderen Maschine empfangenen API-Key. |
| `port` | Web interface port | `443` | Genutzt, falls die Adresse keinen Port angibt. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | TrueNAS liefert standardmäßig ein selbstsigniertes Zertifikat. |
| `request_timeout_seconds` | Timeout per request (seconds) | `20` | Erlaubte Zeit für jeden API-Aufruf, von 1 bis 120. |
| `datasets` | Watch the datasets | `true` | Nutzung gegen Kontingent, Snapshot-Anzahl, letzte Snapshot-Änderung. |
| `disks` | Watch the disks | `true` | Inventar und zwischengespeicherte Temperaturen; eine schlafende Platte wird nie geweckt. |
| `smart` | Read the SMART test results | `true` | Der letzte Selbsttest jeder Platte; keiner wird je gestartet. |
| `alerts` | Read TrueNAS's own alerts | `true` | Die von TrueNAS selbst geführte Alarmliste, verworfene Alarme ausgelassen. |
| `tasks` | Watch replication and snapshot tasks | `true` | Zustand, letzter Lauf und letzter Snapshot jedes Tasks. |
| `services` | Watch the services | `true` | Zum Start mit dem NAS vorgesehene Dienste, und ob sie laufen. |

## Häufige Fehler

| Symptom | Wahrscheinliche Ursache |
|---|---|
| Authentifizierungsfehler, der Local Administrator erwähnt | Der Benutzer des Keys ist kein vollständiger Administrator — typischerweise ein Read-Only Administrator. TrueNAS lehnt einen solchen Key auf jedem REST-Aufruf mit einem leeren `403` ab. |
| Authentifizierungsfehler "Invalid API key" | Der Key wurde falsch eingegeben, gelöscht, ist abgelaufen — oder wurde von TrueNAS widerrufen, weil er einmal über einfaches HTTP von einer anderen Maschine gesendet wurde. Erstellen Sie einen neuen und behalten Sie HTTPS. |
| Zertifikatsfehler | Selbstsigniertes Zertifikat: installieren Sie ein vertrauenswürdiges (ACME ist eingebaut) oder aktivieren Sie `insecure_tls`. |
| Keine Plattentemperaturen | Das NAS läuft in einer virtuellen Maschine, oder TrueNAS hat die Platten seit dem Start noch nicht abgefragt. Messwerte kommen aus seinem Cache. |
| Keine SMART-Zeitreihe | Auf dieser Platte lief noch kein Selbsttest, oder das NAS läuft mit TrueNAS 25.10, das diesen Aufruf entfernt hat. Planen Sie SMART-Tests unter Data Protection. |
| Kein `pool_last_scrub_age_seconds` | Der letzte Scan dieses Pools war ein Resilver, der den Eintrag des letzten Scrubs löscht, oder der Pool wurde nie gescrubbt. |
| Ein Dataset hat keine Kontingent-Zeitreihe | Es hat kein Kontingent. Ein nicht gesetztes Kontingent ist kein Kontingent von null. |
| "NAS pool degraded" und die Seite zeigt keinen Plattennamen | Der Pool kann überhaupt nicht geöffnet werden (exportiert, oder zu viele fehlende Platten): TrueNAS meldet dann kein Layout. |
| `scrape_errors` über null | Ein optionaler Aufruf schlug aus einem anderen Grund als einem fehlenden Pfad fehl — meist ein Timeout. Die anderen Metriken sind nicht betroffen. |
