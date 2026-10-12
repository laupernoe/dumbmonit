# Pi-hole

Der DNS-Werbeblocker: ob er blockiert, wie viele Abfragen er beantwortet
und blockiert, wie alt seine Blocklisten sind, ob seine
Upstream-Resolver fehlschlagen, ob ein Update wartet, und die Meldungen
seiner eigenen Diagnoseseite.

DumbMonit liest Pi-hole v6s REST-API (`/api`). Pi-hole 5 und älter haben
keine solche API und werden nicht unterstützt.

Was mit einem Pi-hole schiefgeht, zeigt sich selten: DNS antwortet
weiter. Blockierung ist "für fünf Minuten" pausiert und kommt nie
zurück, das wöchentliche Blocklisten-Update (Gravity) ist seit einem
Monat fehlgeschlagen, ein Upstream-Resolver antwortet mit SERVFAIL, und
die Diagnoseseite, die niemand öffnet, hat den Grund die ganze Zeit
aufgeführt.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_pihole_`. Pi-hole zählt
Abfragen über die letzten 24 Stunden, ein gleitendes Fenster, das steigt
und fällt: Diese Zahlen sind Gauges, mit dem Suffix `_24h`, keine
Zähler.

| Metrik | Was | Labels |
|---|---|---|
| `blocking_enabled` | 1, wenn Blockierung an ist; 0, wenn sie aus, pausiert oder fehlgeschlagen ist | |
| `blocking_timer_seconds` | verbleibende Zeit, bis eine pausierte Blockierung sich selbst wieder einschaltet | |
| `queries_24h`, `queries_blocked_24h`, `queries_blocked_percent` | in den letzten 24 Stunden beantwortete und blockierte Abfragen | |
| `queries_forwarded_24h`, `queries_cached_24h`, `unique_domains_24h` | an Upstream gesendete Abfragen, aus dem Cache beantwortete, eindeutige Domains | |
| `queries_per_second` | Durchschnitt über die letzte Minute | |
| `replies_servfail_24h` | SERVFAIL-Antworten in den letzten 24 Stunden: ein fehlschlagender Upstream-Resolver zeigt sich hier zuerst | |
| `clients_active`, `clients_total` | gesehene Clients | |
| `gravity_domains` | Domains auf den Blocklisten | |
| `gravity_last_update_timestamp_seconds`, `gravity_age_seconds` | letzter Blocklisten-Neuaufbau, und wie lange her | |
| `version_info` | Wert 1 | `core`, `web`, `ftl`, `docker` |
| `update_available` | 1, wenn eine neuere Version einer Komponente veröffentlicht ist, sonst 0; fehlt, bis Pi-holes täglicher Check gelaufen ist | `component` (`core`, `web`, `ftl`, `docker`) |
| `updates_available` | zu installierende Updates: das Image in Docker (core, web und FTL kommen damit), sonst jede veraltete Komponente | |
| `messages`, `messages_by_type` | Meldungen auf der Diagnoseseite | `type` (`LOAD`, `RATE_LIMIT`, `LIST`…) |
| `ftl_uptime_seconds`, `ftl_memory_percent`, `ftl_cpu_percent` | der FTL-Daemon | |
| `privacy_level` | Pi-holes Privacy-Level, 0 bis 3 | |
| `scrape_errors` | bei der letzten Prüfung fehlgeschlagene sekundäre API-Aufrufe | |

Auf einem Entwicklungszweig vergleicht Pi-hole Commits statt
Versionsnummern, und DumbMonit tut dasselbe.

Die geltenden [eingebauten Regeln](../alerting/rules.md#pi-hole):
Pi-hole blocking disabled, Pi-hole blocklists stale, Pi-hole update
available, Pi-hole diagnosis messages, plus Device unreachable.

## Die Geräteseite

Das Panel über den Diagrammen liest, was die Prüfung gespeichert hat; das
Öffnen der Seite verbindet sich nie mit Pi-hole. Es zeigt, ob Blockierung
an ist (und wann eine Pause endet), die Abfragen und den blockierten
Anteil in den letzten 24 Stunden, die Größe und das Alter der
Blockliste, die Clients, die Versionen mit wartenden Updates, und die
Anzahl der Diagnosemeldungen.

## Ein App-Passwort in Pi-hole erstellen

1. Pi-hole v6 oder später wird benötigt: Version 5 hatte keine REST-API.
   Pi-hole hat kein nur-lesendes Konto; das Nächste ist ein
   App-Passwort, das keine Einstellung ändern kann.

2. Öffnen Sie in Pi-holes Web-Oberfläche Settings → Web interface / API
   und schalten Sie die Seite von Basic auf Expert. Klicken Sie auf
   Configure app password, kopieren Sie das angezeigte Passwort, klicken
   Sie dann auf Enable new app password. Pi-hole zeigt es nur einmal,
   und ein neues ersetzt das vorherige.

3. Ein App-Passwort bleibt für Einstellungen nur-lesend, solange
   webserver.api.app_sudo false ist, der Standard. Prüfen Sie das auf
   der Pi-hole-Maschine (in Docker den Befehl mit docker exec und dem
   Containernamen voranstellen):

    ```
    pihole-FTL --config webserver.api.app_sudo
    ```

4. Geben Sie in DumbMonit die Pi-hole-Adresse ein, zum Beispiel
   "pi.hole" oder "192.168.1.53", und fügen Sie das App-Passwort ein.
   Für ein Pi-hole, dessen Web-Oberfläche kein Passwort hat, wählen Sie
   stattdessen Kein Passwort.

5. DumbMonit liest nur. Es pausiert nie die Blockierung und ändert nie
   eine Liste, und es hält zwischen Prüfungen eine API-Sitzung offen
   statt pro Prüfung eine neue zu öffnen: Pi-hole erlaubt 16 Sitzungen
   gleichzeitig.

!!! warning
    Das App-Passwort erlaubt trotzdem, Blockierung zu pausieren und
    Listen über die API zu bearbeiten: Behalten Sie es exklusiv für
    DumbMonit. Pi-hole liefert einfaches HTTP auf Port 80, und das
    Passwort durchläuft das Netzwerk jedes Mal, wenn eine Sitzung
    öffnet: Nutzen Sie über ein nicht vertrauenswürdiges Netzwerk HTTPS
    auf Port 443.

Ein App-Passwort loggt auch ohne den zweiten Faktor ein, wenn
Zwei-Faktor-Authentifizierung an ist: Dafür ist es da. Um es zu
widerrufen, leeren Sie `webserver.api.app_pwhash` unter Settings → All
settings, oder aktivieren Sie ein neues: Das vorherige funktioniert dann
sofort nicht mehr.

## Credentials

| Credential | Felder |
|---|---|
| App-Passwort | Das oben erzeugte Passwort. DumbMonit tauscht es gegen eine Sitzung, im Header `X-FTL-SID` gesendet, und nutzt diese Sitzung weiter, solange Pi-hole sie behält (30 Minuten nach der letzten Nutzung). |
| Kein Passwort | Nur für ein Pi-hole, dessen Web-Oberfläche kein Passwort hat. |

Adresse: ein Hostname oder eine IP (`pi.hole`), `host:port`, oder eine
vollständige URL (`https://pi.hole`); der aus dem Browser kopierte Pfad
der Web-Oberfläche oder der API wird ignoriert. Optionen: Protokoll
(standardmäßig HTTP), Port (80), Zertifikatsprüfung (für HTTPS auf 443
ist Pi-holes eigenes Zertifikat selbstsigniert) und Anfrage-Timeout
(10 s).

Schlagen Prüfungen mit "no free API session" fehl, sind alle Sitzungen
von Pi-hole belegt, meist durch ein Skript, das sich bei jedem Lauf
einloggt, ohne sich auszuloggen. Sie laufen nach 30 Minuten ab;
`webserver.api.max_sessions` hebt das Limit an.
