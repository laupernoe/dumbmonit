# Apache httpd

Der Webserver: beschäftigte und untätige Worker, das Worker-Scoreboard nach
Zustand, und Request-/Byte-Raten, wenn ExtendedStatus aktiv ist.

DumbMonit liest Apaches eingebautes `mod_status`, abgefragt mit `?auto`,
sodass es in einfachen `Key: Value`-Zeilen antwortet statt mit der für
Browser gedachten HTML-Seite.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_apache_`. Die `_total`-Metriken
sind Zähler: Diagramme und Regeln verwandeln sie in Raten. Alles unterhalb
von `workers_busy`/`workers_idle` und `scoreboard` braucht
`ExtendedStatus On`.

| Metrik | Was | Labels |
|---|---|---|
| `workers_busy`, `workers_idle` | Worker-Prozesse oder -Threads, nach Zustand | |
| `scoreboard` | Worker-Slots nach Scoreboard-Zustand | `state` |
| `requests_total` | seit Apache-Start bediente Requests | |
| `sent_bytes_total` | seit Apache-Start gesendete Bytes | |
| `uptime_seconds` | Zeit seit Apache-Start | |
| `requests_per_second`, `bytes_per_second`, `bytes_per_request` | Apaches eigene laufende Durchschnitte | |
| `cpu_load_ratio` | Apaches eigener `CPULoad`-Wert | |
| `connections_total`, `connections_async_writing`, `connections_async_keepalive`, `connections_async_closing` | asynchrone Verbindungszahlen (event- und worker-MPM) | |
| `version_info` | Vorhandensein; der `Server`-Antwort-Header | `version` |

`scoreboard` summiert sich immer auf die konfigurierte Anzahl der
Worker-Slots: Zustände sind waiting, open_slot, starting, reading, sending,
keepalive, dns_lookup, closing, logging, finishing und idle_cleanup.

## Die Geräteseite

Das Panel über den Diagrammen liest, was die Abfrage gespeichert hat; das
Öffnen der Seite fragt Apache nie ab. Es zeigt beschäftigte und untätige
Worker, die Scoreboard-Aufschlüsselung und, mit ExtendedStatus, die
Request- und Byte-Raten über 24 Stunden.

## Apaches mod_status aktivieren

1. Stellen Sie sicher, dass mod_status geladen ist (es wird mit Apache
   geliefert und ist meist standardmäßig aktiviert).

    ```
    a2enmod status
    ```

2. Fügen Sie eine auf den DumbMonit-Host beschränkte Status-Location
   hinzu; passen Sie die Adresse an. Auf Debian und Ubuntu geschieht dies
   in einer eigenen Datei, anderswo in der Hauptkonfiguration.

    ```
    <Location "/server-status">
    SetHandler server-status
    Require ip 10.0.0.5
    </Location>
    ```

3. Aktivieren Sie die Request- und Byte-Zähler (optional, aber lohnend):
   ohne sie werden nur beschäftigte/untätige Worker und das Scoreboard
   gemeldet.

    ```
    ExtendedStatus On
    ```

4. Apache neu laden und prüfen, dass es antwortet, bevor DumbMonit
   konfiguriert wird.

    ```
    apachectl graceful
    curl -s 'http://apache.lan/server-status?auto'
    ```

5. Geben Sie in DumbMonit den Apache-Host ein, zum Beispiel "apache.lan",
   und den Pfad, falls er nicht "/server-status" ist (DumbMonit fügt
   "?auto" selbst hinzu).

!!! warning
    Ohne ExtendedStatus werden Requests und Bytes überhaupt nicht
    gemeldet, nicht als null: Das Diagramm bleibt leer, statt flach zu
    lügen. Apache veröffentlicht seine eigene Version nicht in
    mod_status; DumbMonit liest sie stattdessen aus dem
    Server-Antwort-Header, den manche Distributionen absichtlich kürzen.

Auf Debian und Ubuntu existiert die Status-Location meist schon in
`/etc/apache2/mods-available/status.conf`; dort die `Require`-Zeile zu
bearbeiten reicht, keine neue Datei nötig. Wo die Konfiguration eine
einzige Datei ist (die meisten anderen Distributionen), fügen Sie den
Block stattdessen zum Haupt-Virtual-Host hinzu.

## Credentials

| Credential | Felder |
|---|---|
| Kein Passwort | Nichts: Die Statusseite ist so gefiltert, dass nur DumbMonit sie erreicht. |
| Basic auth | Benutzer und Passwort des Blocks, der die Statusseite schützt, falls vorhanden. |

Adresse: ein Hostname oder eine IP, mit dem Port, falls er nicht 80 ist.
Optionen: Protokoll (standardmäßig HTTP), Port, Status-Pfad (standardmäßig
`/server-status`, `?auto` automatisch hinzugefügt), Zertifikatsprüfung und
Anfrage-Timeout (10 s).
