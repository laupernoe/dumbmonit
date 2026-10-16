# Nextcloud {#nextcloud}

Ob Nextcloud ausliefert oder festhängt: Wartungsmodus, der angeblieben ist, ein nach einem
Update wartendes Datenbank-Upgrade, Nextcloud- und App-Updates, aktive Benutzer, freier Platz
für das Datenverzeichnis, die Datenbankgröße und der OPcache von PHP.

DumbMonit liest die öffentliche `status.php` und die Statistiken der App **serverinfo**, die
mit Nextcloud ausgeliefert wird, mit einem **Monitoring-Token** (`NC-Token`). Das Token öffnet
diese Statistiken und sonst nichts: Es kann sich nicht anmelden, keine Benutzer auflisten, keine
Datei öffnen und keine Einstellung ändern.

Der Fehler, auf den es ankommt: **ein Update, das nicht abgeschlossen wurde**. Nach einem Update
kann Nextcloud im Wartungsmodus bleiben oder auf `occ upgrade` warten; die Webseite zeigt dann
einen Hinweis, den niemand sieht, bis jemand zu synchronisieren versucht. Beides wird aus
`status.php` gelesen, die auch dann antwortet.

## Was überwacht wird {#what-it-watches}

Alle Metriken haben das Präfix `dumbmonit_nextcloud_`.

| Metrik | Was | Labels |
|---|---|---|
| `maintenance` | 1, solange der Wartungsmodus aktiv ist | |
| `needs_db_upgrade` | 1, solange Nextcloud auf `occ upgrade` wartet | |
| `version_info` | Wert 1 | `version` |
| `update_available` | 1, wenn der eigene Update-Check von Nextcloud eine neuere Version gefunden hat | `available_version` |
| `apps_installed`, `app_updates_available` | installierte Apps und wie viele davon im App Store eine neuere Version haben | |
| `app_update_available` | Wert 1 pro App mit Update (höchstens 32) | `app`, `available_version` |
| `active_users` | in den letzten 5 Minuten, der letzten Stunde und dem letzten Tag gesehene Benutzer | `window` (`5m`, `1h`, `24h`) |
| `users`, `files`, `shares` | Konten, Dateien und Freigaben der Instanz | |
| `free_space_bytes` | freier Platz dort, wo Nextcloud seine Daten ablegt | |
| `database_size_bytes` | Größe der Datenbank | |
| `platform_info` | Wert 1 | `database`, `database_version`, `php_version` |
| `php_memory_limit_bytes` | PHP-`memory_limit` | |
| `opcache_enabled`, `opcache_full` | OPcache aktiv; OPcache ohne Platz | |
| `opcache_memory_used_percent`, `opcache_interned_strings_used_percent`, `opcache_hit_rate_percent`, `opcache_oom_restarts` | wie voll OPcache ist und wie gut er liefert | |
| `fpm_active_processes`, `fpm_total_processes`, `fpm_listen_queue`, `fpm_max_children_reached`, `fpm_slow_requests` | PHP-FPM-Pool, wenn Nextcloud hinter PHP-FPM läuft (die `-fpm`-Images, AIO) | |

Solange sich Nextcloud in Wartung befindet oder auf ein Upgrade wartet, antwortet seine API mit
503: Dann werden nur `maintenance`, `needs_db_upgrade` und `version_info` geschrieben, und der
Monitor wird nicht als nicht erreichbar gemeldet.

Die zutreffenden [integrierten Regeln](../alerting/rules.md#self-hosted-applications):
Nextcloud im Wartungsmodus festgehangen, Nextcloud-Datenbank-Upgrade ausstehend,
Nextcloud-Update verfügbar, Nextcloud-OPcache voll, Nextcloud-freier Platz knapp, außerdem
Monitor nicht erreichbar.

## Hintergrundjobs (Cron) {#background-jobs-cron}

Nextcloud teilt dem Monitoring-Token nicht mit, wann Hintergrundjobs zuletzt liefen: Das kann
nur ein Konto mit allen Rechten auf der Instanz lesen, und DumbMonit verlangt keines. Überwachen
Sie Cron stattdessen mit einem [Heartbeat](push.md): Erstellen Sie in DumbMonit einen und rufen
Sie dessen URL am Ende der Cron-Zeile auf, die die Hintergrundjobs von Nextcloud ausführt, sodass
er sich nur meldet, wenn `cron.php` erfolgreich war:

```
*/5 * * * * php -f /var/www/html/cron.php && curl -fsS -m 10 --retry 3 https://monit.example.com/api/push/<token> > /dev/null
```

Bei einem erwarteten Intervall von 5 Minuten alarmiert der Heartbeat, sobald Cron stehen bleibt,
aus welchem Grund auch immer (ein Container ohne sein Cron neu gestartet, ein PHP-Fehler,
Hintergrundjobs auf AJAX belassen).

## Client-Geräte {#client-devices}

Nextcloud stellt über keine API eine administratorweite Liste der Sync-Clients oder mobilen
Geräte bereit. Die persönliche Seite „Geräte & Sitzungen“ (Einstellungen → Sicherheit) listet
nur das aktuell angemeldete Konto auf, gestützt auf eine Browsersitzung statt auf ein
API-Token, und das einzige administratorweite Gegenstück ist die Kommandozeile
(`occ user:auth-tokens:list <user>`, ein Konto nach dem anderen) — nichts, was das
Monitoring-Token erreichen kann, und nichts, was DumbMonit einen Administrator zu einer Prüfung
verskripten lässt. Auch die `serverinfo`-API, die DumbMonit liest, hat keine Geräteliste: Sie
liefert Zahlen aktiver Benutzer (`active_users`, oben), nie wer oder was sie sind.

Das ist eine bewusste Grenze, keine fehlende Funktion: Eine Geräteliste für DumbMonit würde
bedeuten, das Monitoring-Token zu einem vollständigen Administrationskonto auszuweiten, was der
Rest dieser Seite ausdrücklich vermeidet. Sollte Nextcloud dies je über OCS bereitstellen, wird
DumbMonit dem bereits für Immich, Proxmox Backup Server und Veeam verwendeten Muster der
[Client-Geräte](../alerting/rules.md) folgen.

## Die Monitorseite {#the-device-page}

Das Panel über den Diagrammen liest, was die Prüfung gespeichert hat; das Öffnen der Seite ruft
Nextcloud nie auf. Es nennt zuerst, was nicht stimmt (Wartung, wartendes Upgrade, OPcache voll
oder aus), dann Updates, dann aktive Benutzer, freien Platz, Datenbankgröße, Benutzer, Dateien
und OPcache-Werte, dann die zu aktualisierenden Apps mit Namen und die Plattform (Datenbank,
PHP).

## Nextcloud ein Monitoring-Token geben {#give-nextcloud-a-monitoring-token}

1. Die serverinfo-App von Nextcloud, mit Nextcloud ausgeliefert und aktiviert, antwortet auf ein Monitoring-Token, das ihre Statistiken öffnet und sonst nichts: keine Datei, keinen Benutzer, keine Freigabe. Setzen Sie auf dem Nextcloud-Server ein langes zufälliges Token und lassen Sie es sich anzeigen. Führen Sie occ als Benutzer des Webservers aus; in Docker stellen Sie docker exec -u www-data und den Containernamen voran.

    ```
    php occ config:app:set serverinfo token --value "$(openssl rand -hex 32)"
    php occ config:app:get serverinfo token
    ```

2. Meldet occ, dass serverinfo deaktiviert ist, aktivieren Sie die App zuerst. Direkt nach dem Setzen des Tokens kann Nextcloud den allerersten Lesezugriff ablehnen (401), bevor es wirksam wird: DumbMonit versucht es nach zwei Sekunden einmal erneut, sodass ein neues Ziel keinen unberechtigten Fehler „Token abgelehnt“ zeigt.

    ```
    php occ app:enable serverinfo
    ```

3. Geben Sie in DumbMonit die Adresse ein, mit der Sie Nextcloud öffnen, zum Beispiel „cloud.example.com“ (HTTPS auf 443) oder „http://10.0.0.5:8080“, und fügen Sie das Token ein. Der Host muss eine der vertrauenswürdigen Domains von Nextcloud sein, sonst lehnt Nextcloud die Anfrage ab.

4. DumbMonit liest nur status.php und die serverinfo-Statistiken. Es öffnet nie eine Datei, listet nie Benutzer auf und ändert nie eine Einstellung, und das Token kann sich nirgends anmelden.

!!! warning
    Nextcloud teilt dem Token nicht mit, wann Hintergrundjobs (Cron) zuletzt liefen: Das kann nur ein Konto mit allen Rechten auf der Instanz lesen. Um informiert zu werden, wenn Cron stehen bleibt, fügen Sie in DumbMonit einen Heartbeat hinzu und rufen Sie dessen URL am Ende der Cron-Zeile auf, wie die Dokumentation zeigt.

Bei Nextcloud AIO führen Sie occ im Container `nextcloud-aio-nextcloud` aus:
`docker exec -u www-data nextcloud-aio-nextcloud php occ …`. Warum keine delegierte
Administrationsgruppe: Nextcloud öffnet die serverinfo-API nur für das Token oder für
Mitglieder der vollen Gruppe `admin`, und Delegation ändert daran nichts.

Das Token von Hand prüfen, vom DumbMonit-Host aus:

```
curl -H 'NC-Token: YOUR-TOKEN' 'https://cloud.example.com/ocs/v2.php/apps/serverinfo/api/v1/info?format=json'
```

## Zugangsdaten {#credentials}

| Zugangsdaten | Felder |
|---|---|
| Monitoring-Token (NC-Token) | Das serverinfo-Token. DumbMonit sendet es im Header `NC-Token`, mit `OCS-APIRequest: true`. |

Adresse: ein Hostname (`cloud.example.com`), `host:port` oder eine vollständige URL mit
Pfadpräfix (`https://example.com/nextcloud`); ein abschließendes `/index.php` oder `/login`
wird akzeptiert. Optionen: Protokoll (standardmäßig HTTPS), Port (443), Zertifikatsprüfung und
Anfrage-Timeout (15 s).

Die Zahl der App-Updates fragt den App-Store-Cache von Nextcloud ab; auf einer Instanz, die den
App Store nicht erreicht, bleibt diese Zahl bei 0.

Validiert gegen Nextcloud 35.0.1 (offizielles Image).
