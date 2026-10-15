# Graylog

Der Log-Server und der dahinterliegende Suchcluster: ob Graylog
Nachrichten verarbeitet, wie viele in seinem Journal warten, wie voll
seine Puffer sind, was hereinkommt und hinausgeht, die Gesundheit von
OpenSearch oder Elasticsearch, wie Graylog sie sieht, Inputs, die nicht
starten konnten, und Nachrichten, die nicht indiziert werden konnten.

DumbMonit liest Graylogs REST-API mit einem dedizierten Benutzer, der
die eingebaute Rolle **Reader** hält. Es durchsucht nie Nachrichten,
startet oder stoppt nie einen Input und bestätigt nie eine
Benachrichtigung.

Der beobachtenswerte Fehler: **der Suchcluster kommt nicht mehr mit**.
Graylog nimmt weiter Nachrichten an und schreibt sie in sein
Journal auf der Platte; die Web-Oberfläche öffnet sich, Suchen
funktionieren, und zeigen einfach nichts Aktuelles. Das Journal wächst,
bis es voll ist, dann werden Nachrichten verworfen. Der
Journal-Rückstand, die Gesundheit des Suchclusters und die
Indizierungsfehler sagen es lange vorher.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_graylog_`. Journal, Puffer und
Durchsatz sind die **des Knotens, mit dem DumbMonit spricht**: Fügen Sie
in einem Graylog-Cluster jeden Knoten als eigenen Monitor hinzu.

| Metrik | Was | Labels |
|---|---|---|
| `processing` | 1, während der Knoten Nachrichten verarbeitet, 0, wenn die Verarbeitung pausiert ist | |
| `running`, `lb_alive` | der Lebenszyklus des Knotens ist `running`; er meldet sich Load Balancern selbst als lebend | |
| `version_info` | Wert 1 | `version` |
| `journal_uncommitted_entries` | ins Journal geschriebene, noch nicht verarbeitete Nachrichten | |
| `journal_size_bytes`, `journal_size_limit_bytes`, `journal_used_percent` | | |
| `journal_append_per_second`, `journal_read_per_second` | | |
| `buffer_used_percent` | wie voll die Input-, Process- und Output-Puffer sind | `buffer` |
| `messages_in_total`, `messages_out_total` | empfangene Nachrichten, und an den Suchcluster ausgeschriebene | |
| `indexer_status` | Gesundheit des Suchclusters: 0 grün, 1 gelb, 2 rot, 3 von Graylog aus nicht erreichbar | |
| `indexer_shards` | Shards nach Zustand: `active`, `initializing`, `relocating`, `unassigned` | `state` |
| `inputs_running`, `inputs_failed` | Inputs auf diesem Knoten | |
| `input_failed` | Wert 1 pro nicht startendem Input | `input` (sein Titel) |
| `output_failures_total`, `processing_failures_total`, `invalid_timestamps_total` | fehlgeschlagene Schreibvorgänge an den Suchcluster, Nachrichten, die bei Verarbeitung oder Indizierung fehlschlugen, Nachrichten mit ungültigem Zeitstempel | |
| `notifications`, `notifications_urgent`, `notification` | Graylogs eigene Benachrichtigungen (optionale Berechtigung, siehe unten) | `type`, `severity` |
| `cluster_nodes` | Knoten im Graylog-Cluster | |
| `scrape_errors` | bei der letzten Abfrage fehlgeschlagene Aufrufe | |

Der Suchcluster wird über Graylog gelesen, daher ist kein eigener
OpenSearch- oder Elasticsearch-Monitor nötig, um zu wissen, ob er grün
ist; wenn Graylog ihn überhaupt nicht erreichen kann, liest sich
`indexer_status` als 3.

Die geltenden [eingebauten Regeln](../alerting/rules.md#log-and-metrics-servers):
Graylog not processing, Graylog search cluster down, Graylog search
cluster yellow, Graylog journal filling, Graylog input failed, Graylog
indexing failures, plus Monitor unreachable.

## Die Monitorseite

Das Panel über den Diagrammen liest, was die Abfrage gespeichert hat; das
Öffnen der Seite fragt Graylog nie ab. Es sagt in einem Satz, was nicht
stimmt (Verarbeitung, Suchcluster, Journal-Rückstand, fehlgeschlagene
Inputs namentlich, Indizierungsfehler, dringende Benachrichtigungen),
dann die ein- und ausgehenden Nachrichten pro Sekunde, den
Journal-Rückstand und die Anzahl der Knoten, dann die Puffer, die
Shards nach Zustand und Graylogs eigene Benachrichtigungen.

## Einen nur-lesenden Benutzer und Token in Graylog erstellen

1. In Graylog: System → Users and Teams → Create user. Benennen Sie den
   Benutzer wie folgt, geben Sie ihm ein langes Zufallspasswort und
   weisen Sie nur die Rolle Reader zu. Reader kann den Knotenstatus,
   das Journal, die Puffer, den Durchsatz, die Inputs und die
   Gesundheit des Suchclusters lesen, und überhaupt keine Nachricht,
   solange kein Stream mit ihm geteilt wird.

    ```
    dumbmonit
    ```

2. Öffnen Sie die Seite "Edit tokens" dieses Benutzers, erstellen Sie
   ein nach DumbMonit benanntes Token und kopieren Sie es jetzt:
   Graylog zeigt es nur einmal.

3. Optional: Graylogs eigene Benachrichtigungen (ein Input, der nicht
   startete, Platten-Grenzwerte, ein fast volles Journal) brauchen eine
   weitere Berechtigung, notifications:read, die keine eingebaute
   Rolle von selbst gewährt. Erstellen Sie mit einem zur
   Rollenverwaltung berechtigten Konto eine Rolle, die nur diese
   Berechtigung hält, und geben Sie sie dem Benutzer dumbmonit. Ohne
   sie werden Benachrichtigungen übersprungen, und nichts sonst ändert
   sich.

    ```
    curl -u YOUR_USER -H 'X-Requested-By: cli' -H 'Content-Type: application/json' -X POST https://graylog.lan/api/roles -d '{"name":"DumbMonit Notifications","description":"Read system notifications","permissions":["notifications:read"],"read_only":false}'
    curl -u YOUR_USER -H 'X-Requested-By: cli' -X PUT 'https://graylog.lan/api/roles/DumbMonit%20Notifications/members/dumbmonit'
    ```

4. Geben Sie in DumbMonit die Adresse des Graylog-Knotens ein, zum
   Beispiel "graylog.lan" (Port 9000) oder "https://graylog.lan" hinter
   einem Reverse Proxy, und fügen Sie das Token ein. Fügen Sie in
   einem Graylog-Cluster jeden Knoten hinzu: Journal, Puffer und
   Durchsatz sind pro Knoten.

5. DumbMonit liest nur. Es durchsucht nie Nachrichten, startet oder
   stoppt nie einen Input und bestätigt nie eine Benachrichtigung.

!!! warning
    Nutzen Sie nicht das Konto wiederverwendet, mit dem Sie sich
    einloggen: Ein durchgesickertes Token trägt alle Rechte seines
    Benutzers, während die obige Reader-Rolle nichts ändern kann.
    Graylog liefert einfaches HTTP, sofern Sie TLS nicht konfiguriert
    haben, und das Token reist mit jedem Request mit: Stellen Sie
    Graylog über ein nicht vertrauenswürdiges Netzwerk hinter HTTPS.

Was jede Reader-Berechtigung abdeckt: `system:read` den Knotenstatus und
die Cluster-Knoten, `journal:read` das Journal, `metrics:read` und
`throughput:read` den Durchsatz und die Puffer, `inputs:read` die
Inputs, `indexercluster:read` die Gesundheit des Suchclusters. Die
Reader-Rolle hält auch `messages:read`, was ihr erlaubt, Nachrichten
nur aus explizit mit ihr geteilten Streams zu lesen: teilen Sie keinen.
Details zu Indizierungsfehlern (`indices:failures`) werden nicht
benötigt: DumbMonit zählt Fehler aus Graylogs eigenen Metriken.

## Credentials

| Credential | Felder |
|---|---|
| Access-Token (empfohlen) | Ein Token des dedizierten Benutzers. Graylog erwartet es als HTTP-Basic-Authentifizierung mit dem Token als Benutzername und dem Wort `token` als Passwort; DumbMonit macht das für Sie. |
| Benutzername / Passwort | Der dedizierte Benutzer selbst. |

Adresse: ein Hostname oder eine IP (`graylog.lan`), `host:port`, oder
eine vollständige URL (`https://graylog.lan`); ein abschließendes
`/api` wird akzeptiert. Optionen: Protokoll (standardmäßig HTTP), Port
(9000), Zertifikatsprüfung und Anfrage-Timeout (10 s).
