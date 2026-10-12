# RabbitMQ

Der Message Broker: Speicher- und Plattenalarme, gestoppte Knoten und
Netzwerkpartitionen, Warteschlangen, die sich füllen, ohne dass jemand
sie liest, Warteschlangen, die nicht mehr laufen, und der
Nachrichtenfluss.

DumbMonit liest die Management-API mit einem Benutzer, der mit
`monitoring` getaggt ist und leere Berechtigungen hat: Er sieht den
Zustand der Knoten, die Alarme und die Warteschlangen, und kann nicht
publizieren, konsumieren, eine Nachricht lesen oder etwas ändern. Ein
Knoten meldet den gesamten Cluster.

Der beobachtenswerte Fehler ist der, den Clients nicht melden. Wenn ein
Knoten seinen Memory-High-Watermark oder sein Plattenplatz-Limit
überschreitet, hebt RabbitMQ einen Alarm und blockiert jeden Publisher
im Cluster: Anwendungen hängen einfach bei `publish`, ohne Fehler. Der
zweite ist eine Warteschlange, die sich weiter füllt, weil der Dienst,
der sie konsumiert hat, gestoppt ist.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_rabbitmq_`. Totals sind
Zähler: Diagramme und Regeln verwandeln sie in Raten.

| Metrik | Was | Labels |
|---|---|---|
| `version_info` | Wert 1 | `version`, `erlang`, `cluster` |
| `connections`, `channels`, `consumers`, `queues` | Cluster-Gesamtzahlen | |
| `messages_ready`, `messages_unacked` | in allen Warteschlangen wartende Nachrichten, und zugestellte, aber noch nicht bestätigte | |
| `messages_published_total`, `messages_delivered_total`, `messages_redelivered_total`, `messages_unroutable_total` | Nachrichtenfluss; unroutable sind Nachrichten, für die keine Warteschlange zum Empfang gebunden war | |
| `alarm` | 1, während der Alarm gehoben ist; Speicher und Platte sind immer vorhanden, bei 0 ohne Alarm | `node`, `resource` |
| `alarms` | im Cluster gehobene Alarme | |
| `nodes`, `nodes_running`, `partitions` | | |
| `node_running`, `node_partitions` | pro Knoten | `node` |
| `node_memory_used_bytes`, `node_memory_limit_bytes`, `node_memory_used_percent` | Speicher gegen den High Watermark | `node` |
| `node_disk_free_bytes`, `node_disk_free_limit_bytes` | freie Platte gegen das Limit | `node` |
| `node_fd_used`, `node_fd_used_percent`, `node_uptime_seconds`, `node_being_drained` | | `node` |
| `queue_messages_ready`, `queue_messages_unacked`, `queue_consumers`, `queue_running` | pro Warteschlange, die vollste zuerst, bis zu "Queues watched one by one" | `vhost`, `queue` |
| `queues_without_consumers`, `queues_not_running` | unter diesen Warteschlangen | |

Alarme werden vom Healthcheck `/api/health/checks/alarms` gelesen, der
sie sofort sieht; die Knotenstatistiken hängen ein paar Sekunden
dahinter.

Die geltenden [eingebauten Regeln](../alerting/rules.md#databases-message-broker-and-security-engine):
RabbitMQ memory or disk alarm, RabbitMQ node down, RabbitMQ network
partition, RabbitMQ queue without consumer, RabbitMQ queue unavailable,
plus Device unreachable.

## Die Geräteseite

Das Panel über den Diagrammen liest, was die Abfrage gespeichert hat; das
Öffnen der Seite fragt RabbitMQ nie ab. Es sagt in einem Satz, was nicht
stimmt (Alarme, Knoten und Partitionen, Warteschlangen mit Nachrichten
ohne Consumer, nicht laufende Warteschlangen), dann die bereiten und
unbestätigten Nachrichten, publizierte und zugestellte Nachrichten pro
Sekunde, Verbindungen, Consumer und Warteschlangen, dann die zehn
vollsten Warteschlangen mit ihren Consumern und dem Speicher und der
freien Platte jedes Knotens.

## Einen RabbitMQ-Benutzer mit dem Tag monitoring erstellen

1. Aktivieren Sie das Management-Plugin, falls es nicht schon an ist
   (die Management-Docker-Images haben es). Seine API hört auf Port
   15672.

    ```
    rabbitmq-plugins enable rabbitmq_management
    ```

2. Erstellen Sie einen Benutzer nur mit dem Tag monitoring: Er kann den
   Zustand von Knoten, Alarmen und Warteschlangen lesen, und nichts
   ändern.

    ```
    rabbitmqctl add_user dumbmonit 'a-long-password'
    rabbitmqctl set_user_tags dumbmonit monitoring
    ```

3. Geben Sie ihm leere Berechtigungen auf jedem virtuellen Host, dessen
   Warteschlangen Sie sehen wollen. Er kann sie dann auflisten, und
   trotzdem nicht publizieren, konsumieren oder eine Nachricht lesen.
   Wiederholen Sie mit -p für jeden weiteren virtuellen Host.

    ```
    rabbitmqctl set_permissions -p / dumbmonit '^$' '^$' '^$'
    ```

4. Geben Sie in DumbMonit die Adresse eines Knotens ein, zum Beispiel
   "rabbit.lan", oder "https://rabbit.lan:15671", wenn die API über TLS
   bedient wird. Ein Knoten meldet den gesamten Cluster: die Alarme und
   Partitionen jedes Knotens, und jede Warteschlange.

!!! warning
    Ein Speicher- oder Plattenalarm blockiert jeden Publisher im
    Cluster, und Clients erhalten keinen Fehler: Anwendungen hängen
    einfach bei publish. Warteschlangen werden die vollste zuerst
    gelistet, bis zu "Queues watched one by one" (standardmäßig 100);
    die Gesamtzahlen zählen immer jede Warteschlange.

## Credentials

| Credential | Felder |
|---|---|
| Benutzername / Passwort | Der oben erstellte Benutzer `monitoring`, als HTTP-Basic-Authentifizierung gesendet. |

Ohne Berechtigungen auf einem virtuellen Host werden dessen
Warteschlangen nicht einzeln gelistet; die Cluster-Gesamtzahlen zählen
sie trotzdem.

Adresse: ein Hostname oder eine IP (`rabbit.lan`), `host:port`, oder eine
vollständige URL mit Pfadpräfix hinter einem Reverse Proxy. Optionen:
Protokoll (standardmäßig HTTP), Port (15672), Zertifikatsprüfung,
Anfrage-Timeout (10 s), und "Queues watched one by one" (100, von 0 bis
500).
