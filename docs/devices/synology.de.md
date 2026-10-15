# Synology DSM

Synology-NAS über die DSM-Web-API: Volumes, Storage-Pools,
Plattengesundheit, Temperatur, Hyper-Backup-Tasks,
Active-Backup-for-Business-Tasks, und Synology Drive und Photos. Der
erste Grund, ein NAS zu überwachen, sind seine Platten: Eine Platte,
die heiß wird oder deren SMART-Status sich verschlechtert, kündigt
einen Ausfall lange an, bevor ein Volume ausfällt.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_synology_`.

| Familie | Metriken | Labels |
|---|---|---|
| System | `system_info` (Modell, DSM-Version, Firmware…), `uptime_seconds`, `temperature_celsius`, `temperature_warning`, `system_crashed`, `system_need_repair`, `storage_health` | `model`, `dsm_version`, `firmware` |
| CPU und Speicher | `cpu_usage_percent`, `cpu_user/system/other_percent`, `cpu_cores`, `memory_total/used/available/installed_bytes`, `memory_usage_percent`, `swap_usage_percent` | |
| Netzwerk | `network_rx_bytes_per_second`, `network_tx_bytes_per_second` | `interface` |
| Volumes | `volume_status`, `volume_total/used/available_bytes`, `volume_used_percent`, `volume_used_warning_percent`, `volume_used_critical_percent` | `volume`, `fs_type`, `raid_type` |
| Storage-Pools und SSD-Caches | `pool_status`, `pool_failed_disks`, `pool_total/used_bytes`, und dieselben vier unter `ssd_cache_` | `pool` oder `ssd_cache`, `name`, `raid_type` |
| Platten | `disk_status`, `disk_smart_status`, `disk_temperature_celsius`, `disk_size_bytes`, `disk_bad_sector_exceeded`, `disk_life_below_threshold`, `disk_remaining_life_percent` (nur SSD: die vom Laufwerk angegebene verbleibende Lebensdauer), `disk_unc_count` (DSMs Zähler für unlesbare Sektoren, den es gegen seinen Bad-Sector-Schwellenwert prüft), `disk_info` | `disk`, `name`; bei `disk_info`: `model`, `serial`, `vendor`, `firmware`, `type`, `ssd` |
| Hyper Backup | `backup_tasks`, `backup_running`, `backup_last_result`, `backup_last_run_age_seconds`, `backup_last_success_age_seconds`, `backup_next_run_in_seconds` | pro Task |
| Synology Drive | `drive_up`, `drive_team_folders`, `drive_team_folder_info`, `drive_connections` (siehe unten) | `drive_team_folder_info`: `name` |
| Synology Photos | `photos_installed` (siehe unten) | |
| Sicherheitseinstellungen | `update_available`, `admin_account_enabled`, `autoblock_enabled`, `firewall_enabled` (`0`/`1`; stündlich gelesen, nur mit Administratorkonto, optional: fehlen, wenn DSM ablehnt) | |
| Sammlung | `up`, `scrape_errors`, `scrape_duration_seconds` | |

Active-Backup-for-Business-Metriken sind die Ausnahme: Sie haben das
Präfix `dumbmonit_abb_` (siehe unten).

`volume_status`, `disk_status` und `disk_smart_status` tragen DSMs
eigenes Wort in einem Label (`status`, `smart_status`) und einen Wert
auf einer Skala: `0` normal, `1` Achtung (ein vorübergehender Zustand
wie `repairing` oder `checking`, oder ein von DSM eingeführtes Wort,
das DumbMonit noch nicht kennt), `2` kritisch (`degrade`, `crashed`,
`failed`…). Das Unbekannte zählt absichtlich als `1`, nie als `0`.

Ein teilweiser Fehlschlag bleibt eine erfolgreiche Abfrage: Schlägt
das Storage-Inventar fehl, werden Systemzustand und Nutzung
trotzdem veröffentlicht, und der Fehlschlag wird in `scrape_errors`
gezählt. Nur ein Fehlschlag bei der API-Discovery oder der
Verbindung verurteilt die gesamte Abfrage.

!!! note "Was die Plattenzeilen nicht zeigen können"
    Zählungen reallokierter Sektoren leben in der Pro-Platte-SMART-
    Attributtabelle, die DSM nur über einen undokumentierten Aufruf
    bedient, dessen Form DumbMonit nicht bestätigen konnte. Die
    Zeilen zeigen daher, was `load_info` gibt: DSMs Bad-Sector-
    Urteil, seinen Zähler unlesbarer Sektoren (und sein Wachstum),
    und die verbleibende Lebensdauer der SSD.

!!! note "Was sich zwischen DSM-Versionen ändert"
    Derselbe Aufruf beantwortet auf jedem DSM nicht dieselben Felder,
    und DumbMonit liest beide Formen, statt eine anzunehmen:

    * `disk_bad_sector_exceeded` kommt von `exceed_bad_sector_thr`
      bis DSM 7.1 und von `sb_days_left_critical` — DSMs Schätzung
      der verbleibenden Tage, bis die Platte ihren Bad-Sector-
      Schwellenwert erreicht — auf DSM 7.2 und später, wo das erste
      Feld verschwunden ist. Dasselbe für `disk_life_below_threshold`
      und `remain_life_danger`;
    * `disk_remaining_life_percent` ist eine einfache Zahl auf
      älterem DSM und ein Objekt mit dem Wert und ob DSM ihm traut
      auf DSM 7.2 und später. Ein von DSM als nicht vertrauenswürdig
      markierter Wert wird nicht veröffentlicht;
    * `volume_used_warning_percent` und `volume_used_critical_percent`
      wiederholen die in DSM gesetzten Schwellenwerte. DSM 7.4 gibt
      sie nicht mehr zurück, daher existieren diese zwei Zeitreihen
      auf einem aktuellen NAS einfach nicht — nichts wird an ihrer
      Stelle erfunden.

## Die Monitorseite

![Ein Synology-Monitor: auf einen Blick, Volumes, Platten und Active Backup](../assets/screenshots/device-synology-light.png){ loading=lazy }

Ein Synology-Monitor erhält sein eigenes Panel über den Diagrammen:

* **At a glance** — CPU, Speicher (genutzt vom Total), Temperatur und
  Uptime, mit DSMs eigenem Storage-Urteil als Anzeige (healthy,
  attention, crashed, repair needed);
* **Volumes** — eine Zeile pro Volume: Nutzungsbalken, genutzt vom
  Total, RAID-Typ und Dateisystem, der Zustand des Volumes in DSMs
  Wort (normal, degraded, crashed, repairing…);
* **Storage pools** — eine Zeile pro Pool: RAID-Typ, der Zustand des
  Pools in DSMs Wort, wie viele seiner Platten DSM als ausgefallen
  meldet, und wie viel davon schon an Volumes vergeben ist. Hier lebt
  die Redundanz: Ein RAID 5, das eine Platte verliert, hält das
  darüberliegende Volume bei `normal`, während es rebuildet, daher
  sagt die Volume-Zeile nichts, und diese sagt alles. Ein voll an
  seine Volumes vergebener Pool ist der normale Endzustand, keine
  Warnung, daher ist diese Zahl ein Satz statt eines gefärbten
  Messgeräts;
* **SSD cache** — eine Zeile pro Cache: sein RAID-Typ, sein Zustand
  und seine Größe. Der Abschnitt fehlt auf einem NAS ohne Cache.
  DSMs `load_info` trägt keinen Cache-Modus oder Hit-Rate, daher wird
  keines angezeigt;
* **Disks** — eine Zeile pro Schacht: Modell, Größe, Temperatur,
  SMART-Urteil, DSMs Bad-Sector-Urteil, der Zähler unlesbarer
  Sektoren und, auf einer SSD, die verbleibende Lebensdauer als
  kleiner Balken;
* **Active Backup for Business** — die Tasks, dann jedes gesicherte
  Gerät mit seinem Rhythmus und einem Dreißig-Tage-Streifen (siehe
  unten).

Alles kommt aus dem, was die Abfrage gespeichert hat: Das Öffnen der
Seite fragt das NAS nie ab.

### Eingebaute Regeln für das NAS

| Regel | Schweregrad | Feuert, wenn |
|---|---|---|
| Synology disk SMART warning | advisory | DSM meldet den SMART-Status der Platte seit 15 Minuten nicht mehr als `normal`. |
| Synology disk failed | warning | DSM meldet die Platte seit 5 Minuten als crashed oder failed. |
| Synology disk bad sectors | warning | Die Anzahl fehlerhafter Sektoren überschreitet seit 15 Minuten den in DSM gesetzten Schwellenwert. |
| Synology disk bad sectors growing | advisory | Neue unlesbare Sektoren in den letzten 24 Stunden (`delta(…disk_unc_count[24h]) > 0`). |
| Synology SSD wearing out | advisory | Eine SSD hat seit einer Stunde weniger als 10 % ihrer bewerteten Lebensdauer übrig. |
| Synology volume degraded | warning | Ein Volume ist seit 5 Minuten degraded oder crashed. |
| Synology storage pool degraded | warning | Ein Storage-Pool oder SSD-Cache ist seit 5 Minuten degraded oder crashed. Ein RAID 5, das eine Platte verliert, zeigt sich hier zuerst: DSM hält das darüberliegende Volume bei `normal`, während es rebuildet. |
| Synology volume almost full | advisory | Ein Volume ist seit 15 Minuten zu 90 % oder mehr voll. |
| Synology temperature high | advisory | Eine Platte ist seit 15 Minuten über 55 °C, oder DSM hat seine Temperaturwarnung gehoben. |
| Synology memory high | advisory | Speichernutzung seit 15 Minuten über 95 %. |

## Active Backup for Business

Active Backup for Business (ABB) sichert PCs, physische Server,
virtuelle Maschinen und Dateiserver auf das NAS. Ist das Paket
installiert und läuft, liest DumbMonit seine Tasks bei jeder Abfrage;
ist es das nicht, kündigt das NAS die API einfach nicht an, und
nichts wird gefragt. Die Option `abb` schaltet die gesamte Familie
aus.

Dasselbe gilt für Hyper Backup: DSM listet die APIs eines Pakets nur,
während dieses Paket läuft, daher veröffentlicht ein NAS, dessen
Hyper Backup gestoppt ist — oder das nur Backups über Hyper Backup
Vault empfängt — keine `backup_*`-Metrik und meldet auch keinen
Fehler. DumbMonit fragt das NAS, welche APIs es hat, bevor es
irgendeine aufruft, daher ist ein fehlendes Paket Stille, nie eine
fehlgeschlagene Abfrage.

| Metrik | Wert | Labels |
|---|---|---|
| `dumbmonit_abb_tasks` | Anzahl der Tasks. | |
| `dumbmonit_abb_task_last_status` | Letzter Lauf: `1` Erfolg, `0` fehlgeschlagen, `2` laufend, `-1` unbekannt (nie gelaufen, abgebrochen, kein Backup). Ein *teilweiser* Erfolg zählt als fehlgeschlagen: mindestens ein Gerät wurde nicht gesichert. | `task`, `task_id`, `source_type`, `result` |
| `dumbmonit_abb_task_last_success_seconds` | Alter des letzten erfolgreichen Backup-Laufs, in Sekunden. Fehlt bei einem nie erfolgreichen Task. | `task`, `task_id`, `source_type` |
| `dumbmonit_abb_task_enabled` | `1`, wenn der Task einen Zeitplan hat und nicht pausiert ist, `0`, wenn er nur von Hand läuft. | `task`, `task_id`, `source_type` |
| `dumbmonit_abb_task_device_count` | An den Task angehängte Geräte. | `task`, `task_id`, `source_type` |

`task` ist der Task-Name, wie ABB ihn zeigt; `source_type` ist `pc`,
`vm`, `physical_server`, `file_server` oder `nas`; `result` ist das
rohe Ergebnis des letzten Laufs (`success`, `partial_success`,
`fail`, `cancel`, `no_backup`, `running`, `none`).

Das Alter wird aus der nach erfolgreichen Backup-Läufen gefilterten
Task-Historie genommen, nicht aus dem "letzten Ergebnis" des Tasks:
Nach einer nächtlichen Versions-Bereinigung ist das letzte Ergebnis
die Bereinigung, nicht das Backup. Das kostet einen Request pro
Task, begrenzt auf zwanzig Tasks.

Eingebaute Regeln auf Tasks:

| Regel | Schweregrad | Feuert, wenn |
|---|---|---|
| Active Backup task failed | warning | Der letzte Lauf schlug fehl, oder war ein teilweiser Erfolg, seit 10 Minuten. Die Benachrichtigung benennt den Task. |
| Active Backup too old | advisory | Seit mehr als zwei Tagen kein erfolgreicher Lauf. |
| Active Backup task disabled | info | Der Task hat seit einer Stunde keinen Zeitplan, oder sein kontinuierliches Backup ist pausiert. |

### Geräte und ihr Rhythmus

Eine Regel auf Task-Ebene ist richtig für einen Server und falsch für
einen Laptop: Menschen lassen ihre Laptops nicht an, und "kein
Backup seit 24 Stunden" ist Montagmorgen für jeden Büro-PC. DumbMonit
beobachtet daher ABB auch **Gerät für Gerät**, gegen die eigenen
Gewohnheiten des Geräts.

Bei jeder Abfrage liest der Collector die von ABB geführten
Pro-Gerät-Läufe (`SYNO.ActiveBackup.Overview`
`list_device_transfer_size`, ein Aufruf für alle Geräte, nur was sich
seit der vorherigen Abfrage geändert hat) und kopiert sie in seine
eigene Datenbank — pro NAS auf neunzig Tage begrenzt —, sodass das
Modell einen Neustart überlebt, ohne einen Monat Historie neu zu
lesen. Dann schaut er für jedes Gerät auf die letzten dreißig Tage.

| Metrik | Wert | Labels |
|---|---|---|
| `dumbmonit_abb_device_state` | `0` ok, `1` idle, `2` learning, `3` running, `4` overdue, `5` failing, `6` never. | `device`, `device_id`, `task`, `task_id`, `state` |
| `dumbmonit_abb_device_overdue` | `1`, während das Gerät überfällig ist, sonst `0`. | `device`, `device_id`, `task`, `task_id` |
| `dumbmonit_abb_device_consecutive_failures` | Fehlgeschlagene Versuche in Folge seit dem letzten Erfolg (ein abgebrochener Lauf zählt für keines von beiden). | gleiche |
| `dumbmonit_abb_device_last_success_seconds` | Alter des letzten erfolgreichen Laufs dieses Geräts. Fehlt, falls es nie gelang. | gleiche |
| `dumbmonit_abb_device_typical_interval_seconds` | Median-Abstand zwischen zwei Erfolgen, freie Tage ausgeschlossen. Fehlt während des Lernens. | gleiche |

#### Wie "overdue" beurteilt wird {#how-overdue-is-judged}

In einfachen Worten, für jedes Gerät:

1. **An welchen Tagen ist es an?** Ein Wochentag, an dem das Gerät
   in den letzten dreißig Tagen mindestens einen Backup-Versuch
   gemacht hat — erfolgreich oder nicht; ein Fehlschlag beweist, dass
   die Maschine genauso an war wie ein Erfolg — ist ein *aktiver
   Tag*. Sobald mindestens zwei Wochen Historie vorliegen, ist ein
   Wochentag ganz ohne Versuch ein *freier Tag*. Ein Montag-bis-
   Freitag genutzter Laptop erhält Samstag und Sonntag als freie
   Tage; ein Server hat keine.
2. **Wie oft sichert es?** Die Abstände zwischen aufeinanderfolgenden
   Erfolgen werden in *aktiver Zeit* gesammelt: Die vollen in einem
   Abstand enthaltenen freien Tage werden nicht gezählt (Freitag-
   abend bis Montagabend ist ein Tag aktiver Zeit, nicht drei). Ihr
   Median ist das *typische Intervall* des Geräts; ihr 90.
   Perzentil ist, wie "eine langsame Woche" aussieht.
3. **Wie lange darf es dauern?** Die Toleranz ist
   `max(1,5 × p90-Abstand, 2 × typisches Intervall, 36 h)`. Ein
   nächtlicher Desktop erhält 48 h; eine alle drei Tage gesicherte
   Maschine erhält etwa sechs; niemand erhält weniger als einen
   anderthalb Tag. Solange weniger als zwei nutzbare Abstände
   existieren (ein diese Woche hinzugefügtes Gerät), ist die Toleranz
   72 h — genug, um ein Wochenende ohne Fehlalarm zu überstehen.
4. **Wie viel aktive Zeit ist vergangen?** Die Zeit seit dem letzten
   Erfolg, minus der vollen darin enthaltenen freien Tage. Ein
   Laptop, zuletzt Freitag um 20:00 gesichert, hat am Montag um 08:00
   zwölf Stunden aktive Zeit hinter sich, nicht sechzig.
5. **Das Urteil**, in dieser Reihenfolge: *running* (ein Backup
   läuft); *failing* (die letzten zwei oder mehr Versuche
   fehlschlugen — ein teilweiser Erfolg ist ein Fehlschlag, ein
   abgebrochener Lauf ist keines von beiden); *never* (kein Erfolg
   verzeichnet); *overdue* (aktive Zeit überschreitet die Toleranz);
   *idle* (heute ist einer seiner freien Tage: aus, wie gewohnt);
   *learning* (Rhythmus noch nicht bekannt); *ok*.

Nur *overdue* hebt `dumbmonit_abb_device_overdue`, und nur *failing*
füttert die Fehlschlag-Regel. Ein untätiger Laptop an einem Sonntag
hebt nichts; derselbe Laptop, am Dienstagabend noch still, tut es.

Tage und Stunden werden in der lokalen Zeit des Servers ausgedrückt
(`TZ` des Containers), wo die die Seite lesende Person lebt. Die
Monitorseite zeigt, pro Gerät: den Zustand als Wort, den letzten
Erfolg und das letzte Ergebnis, den Rhythmus in Worten ("weekdays,
around 20:00", "every ~3 days, on Mon, Wed, Fri, around 03:00",
"still learning (2 runs in 30 days)") und einen Dreißig-Tage-Streifen
— teal für einen Tag mit Erfolg, rot für einen Fehlschlag, gelb für
einen abgebrochenen Lauf, grau für keinen Lauf.

Eingebaute Regeln auf Geräten:

| Regel | Schweregrad | Feuert, wenn |
|---|---|---|
| Active Backup device overdue | advisory | `dumbmonit_abb_device_overdue > 0` seit 30 Minuten. |
| Active Backup device failing | warning | Zwei oder mehr fehlgeschlagene Versuche in Folge, seit 10 Minuten. |

!!! warning "Berechtigungen"
    ABB antwortet nur einem Konto in der Gruppe `administrators` mit
    dem in seinen Anwendungsberechtigungen erlaubten Paket, oder
    einem Konto, dem das Paket delegiert wurde (DSM 7: Active Backup
    for Business → Settings → Privileges). Ein dem die Anwendung
    verweigertes Konto kann keine ABB-Tasks lesen: Der Aufruf wird
    abgelehnt (Code 105) und in `scrape_errors` gezählt. Erlauben
    oder delegieren Sie das Paket dem Monitoring-Konto, oder
    entfernen Sie das Häkchen bei "Watch Active Backup for Business"
    (`abb = false`), um nicht mehr zu fragen.

!!! note "Eine undokumentierte API"
    Synology dokumentiert die ABB-Web-API nicht. DumbMonit folgt der
    von Community-Projekten aufgezeichneten Form
    (`SYNO.ActiveBackup.Task` `list`, `SYNO.ActiveBackup.Log`
    `list_result` und `SYNO.ActiveBackup.Overview`
    `list_device_transfer_size`, alle Version 1) und entdeckt ihren
    Pfad über `SYNO.API.Info`. Ein DSM mit abweichenden Antworten
    erzeugt fehlende Metriken, nie eine fehlgeschlagene Abfrage; ein
    ABB, zu alt, um die Pro-Gerät-Übersicht zu bieten, behält seine
    Task-Metriken und hat einfach keinen Gerätrhythmus.

    Version 1 ist Absicht. DSM 7.4 kündigt diese APIs als Versionen 1
    bis 2 an, aber nur Version 1 trägt die Lesemethoden: bei Version
    2 angefragt, antworten sie alle mit "the requested method does
    not exist", was wie ein Firmware-Unterschied aussieht und keiner
    ist. Dieselbe Antwort kommt auch zurück, wenn das CSRF-Token
    fehlt, daher fragt DumbMonit DSM beim Login nach einem und
    sendet es bei jedem Aufruf.

## Synology Drive

DSM dokumentiert die Synology-Drive-Web-API nicht. DumbMonit liest
drei Aufrufe, deren Name und Form aus der Community-Bibliothek
`N4S4/synology-api` stammen, die sie von denselben Bildschirmen wie
die Synology-Drive-Admin-Konsole aufruft, und folgt derselben Regel
wie Active Backup for Business: Eine API, die ein NAS nicht ankündigt,
ist ein nicht installiertes Paket, nie ein Fehlschlag.

| Metrik | Wert | Labels |
|---|---|---|
| `dumbmonit_synology_drive_up` | `1`, während der Dienst `get_status` beantwortet. Fehlt, wenn das Paket nicht installiert ist. | |
| `dumbmonit_synology_drive_team_folders` | Anzahl der Team-Ordner. | |
| `dumbmonit_synology_drive_team_folder_info` | Wert `1` pro Team-Ordner (höchstens 32). | `name` |
| `dumbmonit_synology_drive_connections` | Anzahl aktiver Verbindungen (derzeit synchronisierte Desktop- und Mobile-Clients). | |

!!! note "Warum Verbindungen nur eine Zählung sind"
    Die "Client list"-Seite der Admin-Konsole zeigt Gerätename,
    Benutzer, Anwendungstyp, Online-Status, IP-Adresse und Standort
    jeder Verbindung, aber das ist eine Beschreibung der Seite, nicht
    der JSON-Felder der API: Weder Synology noch ein Community-
    Projekt dokumentiert, wie `SYNO.SynologyDrive.Connection`
    tatsächlich seine Felder benennt. Statt den Namen oder das letzte
    Sync-Datum eines Geräts aus einem unbestätigten Schlüssel zu
    raten, veröffentlicht DumbMonit nur die Verbindungszählung.
    Einzelne Geräte werden sich in dieselbe
    [Client-Geräte](../alerting/rules.md)-Tabelle einklinken, die
    Immich, Proxmox Backup Server und Veeam nutzen, sobald diese
    Form von einer echten NAS-Antwort bestätigt ist.

Die Option `drive` (standardmäßig an) schaltet die gesamte Familie aus.

## Synology Photos

Synology Photos liefert eine gut dokumentierte Browsing-API, aber
weder Synology noch die Community-Bibliotheken exponieren eine
administrationsweite Liste von Mobile-Backup-Geräten oder ihr letztes
Backup — die mobile App zeigt ihren eigenen Backup-Status lokal, und
nichts zentralisiert das auf der NAS-Seite über eine API. DumbMonit
meldet daher nur, dass das Paket installiert ist:
`dumbmonit_synology_photos_installed` (Wert `1`), aus dem
API-Katalog gelesen, ohne zusätzlichen Request an das NAS — derselbe
Katalog, den `SYNO.API.Info` für jedes andere Paket schon beantwortet.
Die Option `photos` (standardmäßig an) schaltet es aus.

## Was auf dem NAS vorzubereiten ist

Die Schritte unten sind die, die der Hinweis neben dem Formular
zeigt. Das Prinzip: ein für die Überwachung reserviertes Konto, von
allem entleert, was DSMs API nicht braucht — nie das Konto, mit dem
Sie sich einloggen.

1. Öffnen Sie in DSM Control Panel → User & Group → User und klicken
   Sie auf Create. Benennen Sie das Konto wie folgt und geben Sie
   ihm ein nirgendwo sonst genutztes langes Passwort.

    ```
    dumbmonit
    ```

2. Gruppen beitreten: administrators ankreuzen. DSM beantwortet die
   Storage-, Volume- und Disk-SMART-Aufrufe nur dieser Gruppe; ohne
   sie zeigt sich das NAS lebendig, sagt aber nichts über seine
   Platten. Die nächsten zwei Schritte nehmen alles andere zurück.
3. Shared-Folder-Berechtigungen zuweisen: No access auf jedem
   Shared Folder. Anwendungsberechtigungen zuweisen: Deny alles
   außer DSM, plus Active Backup for Business, falls Sie dessen
   Tasks lesen wollen. Überspringen Sie die Quota- und
   Geschwindigkeitslimit-Seiten.
4. Zwei-Faktor-Verifizierung muss für dieses Konto aus bleiben: Kein
   automatisierter Monitor kann einen Einmalcode eingeben. Erzwingt
   Control Panel → Security → Account sie, beschränken Sie die
   Regel auf Gruppen, in denen dieser Benutzer nicht ist, oder
   befreien Sie ihn.
5. Backup-Pakete werden nur gelesen, während sie installiert sind
   und laufen: DSM kündigt die API eines gestoppten Pakets überhaupt
   nicht an, daher werden Hyper-Backup-Tasks still statt
   fehlzuschlagen, und ein NAS, das nur Backups empfängt (Hyper
   Backup Vault), hat keine eigenen Tasks zu zeigen. Active Backup
   for Business antwortet zusätzlich nur einem Konto, das es nutzen
   darf (Active Backup for Business → Settings → Privileges); sonst
   entfernen Sie das Häkchen bei DumbMonits Option "Watch Active
   Backup for Business", um nicht mehr zu fragen.
6. Geben Sie in DumbMonit die NAS-Adresse ein (HTTPS, standardmäßig
   Port 5001), dann Benutzername und Passwort dieses Kontos.

!!! warning
    Die Gruppe administrators wird von DSMs Storage-API verlangt,
    nicht von DumbMonit. Deshalb erhält dieses Konto keinen Shared
    Folder, keine Anwendung und ein nirgendwo sonst genutztes
    Passwort: Es kann das NAS lesen, nicht Ihre Dateien anrühren.

!!! note "Welche Rechte welche Metriken geben"
    Die DSM-API braucht nicht für jeden Aufruf dieselben Rechte.
    Systemstatus (Modell, Version, Temperatur, Uptime) antwortet
    jedem Konto. CPU, Speicher, Volumes und vor allem Disk-SMART-
    Status verlangen die Gruppe `administrators`. Auf DSM 7 reicht
    die Delegation "System monitoring" (Control Panel → User & Group
    → Delegation) für die Nutzung, aber nicht für das Storage-
    Inventar. Ein Konto ohne besondere Rechte gibt daher ein
    "lebendiges" NAS, aber nichts über seine Platten.

Herstellerdokumentation: [Create a user in DSM](https://kb.synology.com/en-global/DSM/help/DSM/AdminCenter/file_user_create).

## Credentials

| Credential | Felder |
|---|---|
| DSM-Konto | **User name**: das oben erstellte Konto. **Password**: sein Passwort. Zwei-Faktor-Verifizierung muss für dieses Konto aus sein: Kein automatisierter Monitor kann einen Einmalcode eingeben. |

Adresse: `192.168.1.30`, `nas.lan` oder `nas.lan:5001`. Ohne Port
wird 5001 über HTTPS und 5000 über HTTP genutzt.

## Optionen

| Schlüssel | Label | Standard | Hilfe |
|---|---|---|---|
| `scheme` | Protocol | `https` | HTTPS passt zu einem NAS frisch aus der Box. HTTP nur, wenn DSM nur im Klartext hört. Auswahl: `https`, `http`. |
| `port` | DSM port | *(leer)* | Genutzt, falls die Adresse keinen Port angibt. Leer: 5001 über HTTPS, 5000 über HTTP. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | Ein NAS liefert standardmäßig ein selbstsigniertes Zertifikat: aktivieren Sie das, wenn die Verbindung deshalb abgelehnt wird. |
| `request_timeout_seconds` | Timeout per request (seconds) | `15` | Erlaubte Zeit für jeden API-Aufruf, von 1 bis 120. Das Storage-Inventar kann schlafende Platten wecken. |
| `abb` | Watch Active Backup for Business | `true` | Liest die Active-Backup-for-Business-Tasks. Braucht das installierte Paket und ein Konto, das es nutzen darf; ein NAS ohne das Paket wird einfach übersprungen. |
| `drive` | Watch Synology Drive | `true` | Liest, ob der Synology-Drive-Dienst antwortet, seine Team-Ordner und seine aktive Verbindungszählung. Ein NAS ohne das Paket wird einfach übersprungen. |
| `photos` | Watch Synology Photos | `true` | Zeigt, ob das Synology-Photos-Paket installiert ist, ohne zusätzlichen Request an das NAS. Synology exponiert keine Liste von Mobile-Backup-Geräten über irgendeine API. |

## Häufige Fehler

| Symptom | Wahrscheinliche Ursache |
|---|---|
| Authentifizierungsfehler | Falsches Passwort, oder Zwei-Faktor-Verifizierung für dieses Konto erforderlich. DSM antwortet mit HTTP 200 selbst bei Fehlschlag; der Fehlercode im JSON-Body ist, was zählt, und DumbMonit liest ihn. |
| Zertifikatsfehler | Selbstsigniertes Zertifikat: aktivieren Sie `insecure_tls`. Das ist ein Konfigurationsfehler, nie "unreachable": Ein antwortendes NAS wird nicht als aus gezeigt. |
| Keine Platten- oder Volume-Metriken | Das Konto ist nicht in der Gruppe `administrators` (siehe die Notiz oben). |
| Keine `dumbmonit_abb_*`-Metriken, `scrape_errors` bei 1 | Das Konto kann Active Backup for Business nicht nutzen: delegieren Sie ihm das Paket, oder deaktivieren Sie die Option `abb`. Ohne installiertes Paket gibt es keinen Fehler und keine Metrik. |
| Task-Metriken, aber keine Gerätezeilen auf der Seite | Das NAS kündigt `SYNO.ActiveBackup.Overview` nicht an (älteres ABB), oder der Monitor wird über einen Relay-Agent abgefragt, der die Historie auf seiner Seite im Speicher hält: Die Metriken existieren, der Kalender nicht. |
| Ein Laptop zeigt "Overdue" am Tag nach dem Hinzufügen | Weniger als drei Erfolge: Das Modell lernt noch und erlaubt 72 Stunden zwischen Backups; es stabilisiert sich, sobald zwei Wochen Gewohnheiten bekannt sind. |
| Platten drehen bei jeder Abfrage hoch | Das Storage-Inventar weckt schlafende Platten. Verlängern Sie das Prüfintervall. |
