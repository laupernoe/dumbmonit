# Nextcloud

Ob Nextcloud bedient oder festsitzt: Wartungsmodus übrig gelassen, ein
nach einem Update wartendes Datenbank-Upgrade, Nextcloud- und App-
Updates, aktive Benutzer, freier Platz für das Datenverzeichnis, die
Datenbankgröße, und PHPs OPcache.

DumbMonit liest das öffentliche `status.php` und die Statistiken der
mit Nextcloud gelieferten **serverinfo**-App mit einem
**Monitoring-Token** (`NC-Token`). Das Token öffnet diese Statistiken
und nichts sonst: Es kann sich nicht einloggen, keine Benutzer
auflisten, keine Datei öffnen und keine Einstellung ändern.

Der beobachtenswerte Fehler: **ein Update, das nicht fertig wurde**.
Nach einem Update kann Nextcloud im Wartungsmodus bleiben, oder auf
`occ upgrade` warten; die Webseite zeigt dann einen Hinweis, den
niemand sieht, bis jemand zu synchronisieren versucht. Beides wird aus
`status.php` gelesen, das selbst dann antwortet.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_nextcloud_`.

| Metrik | Was | Labels |
|---|---|---|
| `maintenance` | 1, während der Wartungsmodus an ist | |
| `needs_db_upgrade` | 1, während Nextcloud auf `occ upgrade` wartet | |
| `version_info` | Wert 1 | `version` |
| `update_available` | 1, wenn Nextclouds eigener Update-Check eine neuere Version gefunden hat | `available_version` |
| `apps_installed`, `app_updates_available` | installierte Apps, und wie viele eine neuere Version im App-Store haben | |
| `app_update_available` | Wert 1 pro App mit Update (höchstens 32) | `app`, `available_version` |
| `active_users` | in den letzten 5 Minuten, der letzten Stunde und dem letzten Tag gesehene Benutzer | `window` (`5m`, `1h`, `24h`) |
| `users`, `files`, `shares` | Konten, Dateien und Shares der Instanz | |
| `free_space_bytes` | freier Platz, wo Nextcloud seine Daten hält | |
| `database_size_bytes` | Größe der Datenbank | |
| `platform_info` | Wert 1 | `database`, `database_version`, `php_version` |
| `php_memory_limit_bytes` | PHP `memory_limit` | |
| `opcache_enabled`, `opcache_full` | OPcache an; OPcache ohne Platz | |
| `opcache_memory_used_percent`, `opcache_interned_strings_used_percent`, `opcache_hit_rate_percent`, `opcache_oom_restarts` | wie voll OPcache ist und wie gut es bedient | |
| `fpm_active_processes`, `fpm_total_processes`, `fpm_listen_queue`, `fpm_max_children_reached`, `fpm_slow_requests` | PHP-FPM-Pool, wenn Nextcloud hinter PHP-FPM läuft (die `-fpm`-Images, AIO) | |

Während Nextcloud im Wartungsmodus ist oder auf ein Upgrade wartet,
antwortet seine API mit 503: Dann werden nur `maintenance`,
`needs_db_upgrade` und `version_info` geschrieben, und das Gerät wird
nicht als unreachable gemeldet.

Die geltenden [eingebauten Regeln](../alerting/rules.md#self-hosted-applications):
Nextcloud stuck in maintenance, Nextcloud database upgrade pending,
Nextcloud update available, Nextcloud OPcache full, Nextcloud free
space low, plus Device unreachable.

## Hintergrund-Jobs (Cron)

Nextcloud sagt dem Monitoring-Token nicht, wann Hintergrund-Jobs
zuletzt liefen: Nur ein Konto mit allen Rechten auf der Instanz kann
das lesen, und DumbMonit fragt nicht nach einem. Beobachten Sie Cron
stattdessen mit einem [Heartbeat](push.md): Erstellen Sie einen in
DumbMonit, rufen Sie dann seine URL am Ende der Cron-Zeile auf, die
Nextclouds Hintergrund-Jobs ausführt, sodass er sich nur meldet, wenn
`cron.php` erfolgreich war:

```
*/5 * * * * php -f /var/www/html/cron.php && curl -fsS -m 10 --retry 3 https://monit.example.com/api/push/<token> > /dev/null
```

Mit einem erwarteten Intervall von 5 Minuten alarmiert der Heartbeat,
sobald Cron stoppt, aus welchem Grund auch immer (ein Container ohne
seinen Cron neu gestartet, ein PHP-Fehler, Hintergrund-Jobs auf AJAX
gelassen).

## Client-Geräte

Nextcloud stellt über keine API eine administrationsweite Liste von
Sync-Clients oder Mobilgeräten bereit. Die persönliche
"Devices & sessions"-Seite (Settings → Security) listet nur das gerade
eingeloggte Konto, gestützt auf eine Browser-Sitzung statt ein
API-Token, und das einzige administrationsweite Äquivalent ist die
Kommandozeile (`occ user:auth-tokens:list <user>`, ein Konto zur Zeit)
— nichts, was das Monitoring-Token erreichen kann, und nichts, worum
DumbMonit einen Administrator bittet, es in eine Abfrage zu
skripten. Die von DumbMonit gelesene `serverinfo`-API hat ebenfalls
keine Geräteliste: Sie gibt Zählungen aktiver Benutzer
(`active_users`, oben), nie wer oder was sie sind.

Das ist eine bewusste Grenze, kein fehlendes Feature: DumbMonit eine
Geräteliste zu geben würde bedeuten, das Monitoring-Token zu einem
vollen Administrationskonto zu erweitern, was der Rest dieser Seite zu
vermeiden versucht. Sollte Nextcloud das jemals über OCS exponieren,
folgt DumbMonit demselben
[Client-Geräte](../alerting/rules.md)-Muster, das schon für Immich,
Proxmox Backup Server und Veeam genutzt wird.

## Die Geräteseite

Das Panel über den Diagrammen liest, was die Abfrage gespeichert hat; das
Öffnen der Seite ruft Nextcloud nie auf. Es sagt zuerst, was nicht
stimmt (Wartung, wartendes Upgrade, OPcache voll oder aus), dann
Updates, dann aktive Benutzer, freien Platz, Datenbankgröße, Benutzer,
Dateien und OPcache-Werte, dann die namentlich zu aktualisierenden
Apps und die Plattform (Datenbank, PHP).

## Nextcloud ein Monitoring-Token geben

1. Nextclouds serverinfo-App, mitgeliefert und mit Nextcloud
   aktiviert, antwortet auf ein Monitoring-Token, das seine
   Statistiken öffnet und nichts sonst: keine Datei, kein Benutzer,
   kein Share. Setzen Sie auf dem Nextcloud-Server ein langes
   Zufalls-Token und geben Sie es zurück aus. Führen Sie occ als
   Webserver-Benutzer aus; in Docker setzen Sie docker exec -u
   www-data und den Containernamen davor.

    ```
    php occ config:app:set serverinfo token --value "$(openssl rand -hex 32)"
    php occ config:app:get serverinfo token
    ```

2. Sagt occ, serverinfo sei deaktiviert, aktivieren Sie es zuerst.

    ```
    php occ app:enable serverinfo
    ```

3. Geben Sie in DumbMonit die Adresse ein, mit der Sie Nextcloud
   öffnen, zum Beispiel "cloud.example.com" (HTTPS auf 443) oder
   "http://10.0.0.5:8080", und fügen Sie das Token ein. Der Host muss
   eine von Nextclouds vertrauten Domains sein, sonst lehnt Nextcloud
   den Request ab.

4. DumbMonit liest nur status.php und die serverinfo-Statistiken. Es
   öffnet nie eine Datei, listet nie Benutzer und ändert nie eine
   Einstellung, und das Token kann sich nirgendwo einloggen.

!!! warning
    Nextcloud sagt dem Token nicht, wann Hintergrund-Jobs (Cron)
    zuletzt liefen: Nur ein Konto mit allen Rechten auf der Instanz
    kann das lesen. Um informiert zu werden, wenn Cron stoppt, fügen
    Sie einen Heartbeat in DumbMonit hinzu und rufen Sie seine URL am
    Ende der Cron-Zeile auf, wie die Dokumentation zeigt.

Mit Nextcloud AIO führen Sie occ im Container
`nextcloud-aio-nextcloud` aus: `docker exec -u www-data
nextcloud-aio-nextcloud php occ …`. Warum keine delegierte
Administrationsgruppe: Nextcloud öffnet die serverinfo-API nur dem
Token oder Mitgliedern der vollen `admin`-Gruppe, und Delegation
ändert das nicht.

Das Token von Hand prüfen, vom DumbMonit-Host aus:

```
curl -H 'NC-Token: YOUR-TOKEN' 'https://cloud.example.com/ocs/v2.php/apps/serverinfo/api/v1/info?format=json'
```

## Credentials

| Credential | Felder |
|---|---|
| Monitoring-Token (NC-Token) | Das serverinfo-Token. DumbMonit sendet es im Header `NC-Token`, mit `OCS-APIRequest: true`. |

Adresse: ein Hostname (`cloud.example.com`), `host:port`, oder eine
vollständige URL mit Pfadpräfix (`https://example.com/nextcloud`); ein
abschließendes `/index.php` oder `/login` wird akzeptiert. Optionen:
Protokoll (standardmäßig HTTPS), Port (443), Zertifikatsprüfung und
Anfrage-Timeout (15 s).

Die App-Update-Zählung fragt Nextclouds App-Store-Cache ab; auf einer
Instanz, die den App-Store nicht erreichen kann, bleibt diese Zählung
bei 0.

Validiert gegen Nextcloud 35.0.1 (offizielles Image).
