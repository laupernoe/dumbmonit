# AdGuard Home

Der DNS-Filter selbst: ob der Schutz an ist, ob sein DNS-Server läuft,
was er beantwortet und blockiert hat, ob seine Filterlisten noch
aktualisieren, ob seine Upstream-Server antworten, und ob eine neuere
Version verfügbar ist.

Ein defekter DNS-Filter sieht nicht defekt aus. Seiten öffnen sich
weiterhin, mit ihrer Werbung. Die üblichen Ausfälle sind leise: ein
"für eine Stunde" aus der Telefon-App pausierter Schutz, der nie wieder
eingeschaltet wurde, Filterlisten, die seit Wochen nicht mehr
herunterladen, ein DNS-over-HTTPS-Upstream-Server, der nicht mehr
antwortet, sodass jede Abfrage auf den nächsten wartet.

DumbMonit liest AdGuard Homes REST-API (`/control`) mit HTTP-Basic-
Authentifizierung. Getestet gegen AdGuard Home v0.107.79 (und den
Versionscheck von v0.107.78).

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_adguard_`. Abfrage- und
Blockierzahlen sind **Summen über AdGuard Homes Statistikfenster**
(24 Stunden, sofern nicht in Settings → General settings geändert), keine
Zähler: Sie steigen und fallen, während das Fenster weiterrutscht, und
werden so gespeichert, wie sie sind.

| Metrik | Was | Labels |
|---|---|---|
| `running` | 1, wenn der DNS-Server läuft | |
| `protection_enabled` | 1, wenn der Schutz an ist | |
| `protection_paused_seconds` | verbleibende Zeit, bis ein pausierter Schutz wieder an ist, 0 ohne Pause | |
| `version_info` | Wert 1 | `version` |
| `dns_queries` | Abfragen im Statistikfenster | |
| `blocked_filtering` | von Filterlisten und benutzerdefinierten Regeln blockierte Abfragen | |
| `replaced_safebrowsing`, `replaced_parental`, `replaced_safesearch` | durch Browsing-Sicherheit, Kindersicherung, Safe-Search ersetzte Antworten | |
| `blocked_percent` | Anteil blockierter Abfragen (Filter, Browsing-Sicherheit, Kindersicherung); fehlt ohne jede Abfrage | |
| `avg_processing_seconds` | durchschnittliche Antwortzeit | |
| `stats_window_seconds`, `stats_enabled` | das Statistikfenster und ob Statistiken geführt werden (0.107.30 und später) | |
| `upstream_responses`, `upstream_avg_seconds` | Antworten und durchschnittliche Antwortzeit pro Upstream im Fenster, wie das Dashboard sie unter "Top upstreams" zeigt (höchstens 8) | `upstream` |
| `upstream_up` | 1, wenn AdGuard Homes eigener Test dieses Upstreams gelingt; 0, wenn er fehlschlägt oder innerhalb von 4 Sekunden keine Antwort gibt (höchstens 8). Das Label ist der Upstream, wie AdGuard Home ihn in seinen Statistiken schreibt (`https://dns10.quad9.net:443/dns-query`, `192.0.2.1:53`); für einen stillen Upstream die konfigurierte Zeile. | `upstream` |
| `filtering_enabled` | 1, wenn Filterung in Settings → General settings an ist | |
| `filters`, `filters_enabled` | konfigurierte Blocklisten, und aktivierte | |
| `filter_rules` | aus den aktivierten Listen geladene Regeln | |
| `filter_rules_count`, `filter_update_age_seconds` | pro aktivierter Liste (höchstens 32) | `filter` |
| `filter_oldest_update_age_seconds` | Alter des ältesten Updates unter den aktivierten Listen | |
| `filters_never_updated` | aktivierte Listen, die nie herunterladen wurden | |
| `filter_update_interval_hours` | wie oft Listen aktualisiert werden; 0 schaltet Updates aus | |
| `update_check_enabled` | 0, wenn AdGuard Home mit ausgeschaltetem Versionscheck läuft | |
| `update_available` | 1, wenn der letzte Versionscheck eine neuere Version meldete; fehlt, wenn der Check aus ist | `latest` |
| `scrape_errors` | fehlgeschlagene sekundäre Aufrufe bei der letzten Prüfung | |

Der Versionscheck ist im offiziellen Docker-Image und in den meisten
Distributionspaketen aus, die AdGuard Home selbst aktualisieren: Dort
wird `update_available` nie geschrieben, und seine Regel bleibt still.

Upstream-Statistiken zählen nur Antworten: Ein Upstream, der aufhört zu
antworten, hinterlässt dort keine Spur. Deshalb lässt DumbMonit AdGuard
Home bei jeder Prüfung jeden Upstream testen, derselbe Test wie die
Schaltfläche "Test upstreams" in Settings → DNS settings: eine
DNS-Abfrage von AdGuard Home an diesen Upstream. Ein toter Upstream
würde AdGuard Home doppelt so lange wie das Upstream-Timeout (standardmäßig
20 Sekunden) warten lassen; DumbMonit hört nach 4 Sekunden auf zu warten
und meldet ihn als down. Entfernen Sie das Häkchen bei "Test upstream
servers", um dies auszuschalten.

Die geltenden [eingebauten Regeln](../alerting/rules.md#adguard-home):
AdGuard Home protection off, AdGuard Home DNS not running, AdGuard Home
upstream failing, AdGuard Home filter lists stale, AdGuard Home update
available, plus Device unreachable.

## Die Geräteseite

Das Panel über den Diagrammen liest, was die Abfrage gespeichert hat; das
Öffnen der Seite verbindet sich nie mit AdGuard Home. Es sagt, ob Schutz
und DNS-Server an sind (und wann eine Pause endet), dann die Abfragen,
Blockierungen und Antwortzeit über das Statistikfenster, die
Upstream-Server mit ihrem Testergebnis und ihrer Antwortzeit, und die
Filterlisten mit ihren Regeln und dem letzten Update.

## Einen Benutzer für DumbMonit zu AdGuard Home hinzufügen

1. AdGuard Home hat kein nur-lesendes Konto: Jeder in AdGuardHome.yaml
   aufgeführte Benutzer kann jede Einstellung ändern. Geben Sie DumbMonit
   einen eigenen Benutzer, damit er entfernt werden kann, ohne Ihren
   anzutasten. Erzeugen Sie zuerst einen bcrypt-Hash eines langen
   Zufallspassworts (htpasswd kommt mit dem Paket apache2-utils oder
   httpd-tools):

    ```
    htpasswd -B -C 10 -n -b dumbmonit 'LONG-RANDOM-PASSWORD'
    ```

2. Stoppen Sie AdGuard Home, öffnen Sie AdGuardHome.yaml (neben dem
   AdGuardHome-Binary, oder im conf-Ordner des Docker-Volumes) und fügen
   Sie diese Zeile unter "users:" hinzu, auf derselben Einrückungsebene
   wie die bereits vorhandene "- name:"-Zeile. Ersetzen Sie HASH mit dem,
   was htpasswd nach "dumbmonit:" ausgegeben hat. Starten Sie AdGuard
   Home wieder.

    ```
    - { name: dumbmonit, password: "HASH" }
    ```

    Der Abschnitt liest sich dann:

    ```yaml
    users:
      - { name: dumbmonit, password: "$2y$10$…" }
      - name: yourname
        password: $2a$10$…
    ```

3. Prüfen Sie vom DumbMonit-Host aus, dass der Benutzer akzeptiert wird:

    ```
    curl -u dumbmonit http://adguard.lan/control/status
    ```

4. Geben Sie in DumbMonit die Adresse der AdGuard-Home-Web-Oberfläche ein,
   zum Beispiel "adguard.lan" (Port 80) oder "http://adguard.lan:3000",
   dann den Benutzer dumbmonit und sein Passwort.

5. DumbMonit liest nur den Status, die Statistiken, die Filterlisten und
   den letzten Versionscheck. Es liest nie das Abfrageprotokoll. Bei
   jeder Prüfung lässt es AdGuard Home jeden Upstream-DNS-Server testen,
   wie es die Schaltfläche "Test upstreams" tut; entfernen Sie unten das
   Häkchen bei "Test upstream servers", um das zu stoppen.

!!! warning
    Der Benutzer dumbmonit kann jede Einstellung ändern, wie jeder
    AdGuard-Home-Benutzer: Behalten Sie sein Passwort exklusiv für
    DumbMonit. Es reist mit jedem Request mit, daher schalten Sie über ein
    nicht vertrauenswürdiges Netzwerk HTTPS in Settings → Encryption
    settings ein. Nach fünf falschen Passwörtern verweigert AdGuard Home
    die Adresse für 15 Minuten.

AdGuard Home muss gestoppt sein, während Sie die Datei bearbeiten: Es
schreibt AdGuardHome.yaml bei jeder Einstellungsänderung neu und würde
eine während des Laufens hinzugefügte Zeile verwerfen.

## Credentials

| Credential | Felder |
|---|---|
| Benutzername / Passwort | Der in AdGuardHome.yaml hinzugefügte dedizierte Benutzer, als HTTP-Basic-Authentifizierung gesendet. |
| Keine Authentifizierung | Nur für ein AdGuard Home ganz ohne Benutzer in AdGuardHome.yaml. |

Adresse: ein Hostname oder eine IP (`adguard.lan`), `host:port`, oder eine
vollständige URL mit Pfadpräfix hinter einem Reverse Proxy
(`https://dns.lan/adguard`); ein abschließendes `/control` wird
ignoriert. Optionen: Protokoll (standardmäßig HTTP), Port (80),
Zertifikatsprüfung, Anfrage-Timeout (5 s) und der Upstream-Test (ein).

## Aufrufe

| Aufruf | Warum |
|---|---|
| `GET /control/status` | running, protection, version: der einzige Aufruf, dessen Fehlschlagen die Prüfung fehlschlagen lässt |
| `GET /control/stats`, `GET /control/stats/config` | Abfragen, Blockierungen, Upstream-Antworten, Statistikfenster |
| `GET /control/filtering/status` | Filterlisten |
| `POST /control/version.json` mit `{"recheck_now": false}` | das Ergebnis des letzten Versionschecks, wie die Web-Oberfläche es bei jedem Seitenaufruf liest; erzwingt nie einen neuen Check |
| `GET /control/dns_info`, dann `POST /control/test_upstream_dns` einmal pro Upstream | die konfigurierten Upstreams und Bootstrap-Server, dann der Upstream-Test; ändert keine Einstellung |
