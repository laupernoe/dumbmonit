# Services: HTTP, TCP, DNS, Ping, TLS, NTP, SMTP, SQL, MQTT, WebSocket

Elf Monitore beobachten **Services** statt Hardware, im Stil von
Uptime Kuma: eine Webseite, ein Port, ein DNS-Name, ein Host, ein
Zertifikat, ein Zeitserver, ein Mail-Relay, eine Datenbank, ein
MQTT-Broker, ein WebSocket-Endpunkt. Jeder Service erhält eine
Historienleiste, eine Antwortzeit und einen Verfügbarkeitsprozentsatz
auf seiner Geräteseite.

Die letzten fünf gehen weiter als nur eine Verbindung zu öffnen: Sie
beginnen eine echte Sitzung im eigenen Protokoll des Dienstes. Das
ist der Unterschied zwischen "Port 5432 ist offen" und "die Datenbank
akzeptiert noch mein Konto und beantwortet eine Abfrage". NTP sitzt
zwischen beiden: eine einzige Anfrage und Antwort, in NTPs eigenem
Protokoll, genug, um die Uhr des Servers zu lesen, ohne eine Sitzung.

## Wie der Zustand eines Dienstes funktioniert

Ein Dienst kann antworten und trotzdem down sein: ein `500`, ein
fehlendes Schlüsselwort, ein abgelaufenes Zertifikat. Diese Monitore
schreiben daher bei jeder Prüfung einen Punkt, `probe_success` ist 1
oder 0, statt wie ein Hardware-Gerät still zu werden. Das ist, was
einen Verfügbarkeitsprozentsatz möglich macht:
`avg_over_time(dumbmonit_probe_success[30d])`.

| Situation | Was passiert |
|---|---|
| Ungültige Option oder Adresse, ICMP-Sockets nicht erlaubt (siehe Ping) | Konfigurationsfehler: Es wird nichts geschrieben, die Geräteseite zeigt den Fehler, kein "down"-Alarm. |
| Verbindung abgelehnt, Timeout, TLS abgelehnt, `500`, Schlüsselwort fehlt, totaler Paketverlust, Passwort abgelehnt, Abfrage fehlgeschlagen | `probe_success = 0`: Der Ausfall wird aufgezeichnet, datiert und gezählt. |
| Alles gut | `probe_success = 1`. |

Jeder Monitor hat sein eigenes Timeout (`timeout_seconds`,
standardmäßig 5 s, höchstens 60 s), kürzer als `DUMBMONIT_PROBE_TIMEOUT_SECS`
des Servers: Vom Scheduler unterbrochen, könnte er seine Null nicht
schreiben.

Für alle elf geltende eingebaute Regeln: Service down (3 Minuten),
Service flapping (mehr als sechs Zustandswechsel in dreißig
Minuten), Slow service (mehr als 3 s seit zehn Minuten). Für jede
Prüfung, die ein Zertifikat liest — `http`, `tls`, `smtp`, `mqtt`,
`websocket`: Certificate expiring soon (14 Tage) und Certificate
expired.

Ein abgelehntes Passwort ist nie ein Ausfall des Dienstes: Es wird
als `reason="auth"` aufgezeichnet, daher sagt der Alarm, was zu
beheben ist.

### Metriken

Alle Gauges, alle mit dem Präfix `dumbmonit_`, alle mit
`probe="http|tcp|dns|ping|tls|smtp|postgres|mysql|mqtt|websocket"`
beschriftet, zusätzlich zu `target`, `host` und `tag_*`.

| Metrik | Monitore | Bedeutung |
|---|---|---|
| `probe_success` | alle | 1, wenn der Dienst korrekt antwortet, sonst 0. |
| `probe_duration_seconds` | alle | Gesamtdauer der Prüfung. |
| `probe_failure_info` | alle | Vorhandensein (1) mit einem `reason`-Label: `dns`, `connect`, `timeout`, `tls`, `cert_expired`, `status`, `keyword`, `json`, `body`, `packet_loss`, `record`, `auth`, `protocol`, `query`, `payload`. |
| `probe_http_status_code` | http, websocket | Erhaltener Statuscode. |
| `probe_http_first_byte_seconds` | http | Zeit bis zu den Antwort-Headern. |
| `probe_http_content_bytes` | http | Body-Größe. |
| `probe_connect_seconds` | alle außer dns und ping | Verbindungsaufbau (Login für Datenbanken eingeschlossen). |
| `probe_tls_handshake_seconds` | tls, http, smtp, mqtt, websocket | TLS-Aushandlung allein. |
| `probe_tls_version_info` | tls, http, smtp, mqtt, websocket | Vorhandensein; Label `version`. |
| `probe_ssl_cert_expiry_days` | tls, http, smtp, mqtt, websocket | Tage bis zum Ablauf, negativ, falls abgelaufen. |
| `probe_ssl_cert_valid` | tls, http, smtp, mqtt, websocket | 1, wenn die Kette zu einer bekannten Autorität führt. |
| `probe_ssl_cert_issuer_info` | tls, http, smtp, mqtt, websocket | Vorhandensein; Label `issuer`. |
| `probe_dns_lookup_seconds` | dns | Auflösungszeit. |
| `probe_dns_answer_records` | dns | Einträge der angefragten Art. |
| `probe_dns_answer_fingerprint` | dns | Mit "Alert when the answer changes": ein 32-Bit-Fingerprint der sortierten Antwort. Sein Wert bedeutet nichts; nur seine Änderungen tun das. |
| `probe_icmp_rtt_seconds`, `_min_seconds`, `_max_seconds` | ping | Durchschnittliche, kürzeste und längste Round-Trip-Zeit. |
| `probe_icmp_packet_loss_ratio` | ping | Verlust zwischen 0 und 1. |
| `probe_icmp_packets_sent`, `probe_icmp_packets_received` | ping | Gesendete und empfangene Echoes. |
| `probe_smtp_greeting_seconds` | smtp | Zeit bis zur `220`-Begrüßung. |
| `probe_smtp_ehlo_seconds` | smtp | Round Trip des `EHLO`-Befehls. |
| `probe_smtp_capabilities` | smtp | Angekündigte Erweiterungen. |
| `probe_smtp_authenticated` | smtp | 1, wenn `AUTH` gelang. |
| `probe_sql_query_seconds` | postgres, mysql | Abfrageausführung allein. |
| `probe_sql_rows` | postgres, mysql | Zurückgegebene Zeilen. |
| `probe_sql_value` | postgres, mysql | Erste Spalte der ersten Zeile, wenn es eine Zahl ist. |
| `probe_mqtt_connack_seconds` | mqtt | Zeit bis zum `CONNACK`. |
| `probe_mqtt_suback_seconds` | mqtt | Zeit bis zum `SUBACK`. |
| `probe_mqtt_message_bytes` | mqtt | Größe der zurückgehaltenen Nachricht. |
| `probe_mqtt_message_value` | mqtt | Ihr Inhalt, wenn es eine Zahl ist. |
| `probe_ws_handshake_seconds` | websocket | Öffnungs-Handshake. |
| `probe_ws_message_bytes` | websocket | Größe des empfangenen Frames. |

## HTTP

**Website or web API (HTTP)** — prüft, dass eine Seite oder eine API
antwortet, mit dem richtigen Statuscode, dem richtigen Inhalt, und
einem gültigen Zertifikat. Beispiele: eine Website, eine
Health-Seite einer Anwendung, eine REST-API, die Oberfläche eines
selbst gehosteten Dienstes.

### Einrichtung

1. Fügen Sie in die Adresse die vollständige URL der zu
   überwachenden Seite ein. Ohne "http://" oder "https://" wird
   HTTPS angenommen.
2. Bevorzugen Sie eine leichte, loginfreie Seite, zum Beispiel die
   Health-Seite der Anwendung ("/health", "/status"), statt der
   Startseite.
3. Erfordert die Seite Authentifizierung, füllen Sie das Credential
   aus: Benutzername und Passwort geben Basic-Authentifizierung, ein
   Token wird als "Bearer" gesendet.
4. Standardmäßig ist jeder Statuscode zwischen 200 und 299 in
   Ordnung. Um weiterzugehen, verlangen Sie ein Schlüsselwort auf der
   Seite oder einen bestimmten Wert in einer JSON-Antwort.
5. Über HTTPS wird das Zertifikat automatisch gelesen: Sie werden
   vor dem Ablauf gewarnt.

!!! warning
    Schreiben Sie nie ein Passwort oder Token in die Optionen: Sie
    werden bei jeder Messung im Klartext kopiert. Nutzen Sie das
    Credential-Feld, das verschlüsselt ist.

Credentials: keine, Benutzername / Passwort (Basic-Authentifizierung)
oder API-Token (`Authorization: Bearer`).

### Optionen

| Schlüssel | Label | Standard | Hilfe |
|---|---|---|---|
| `method` | HTTP method | `GET` | GET passt fast immer. HEAD vermeidet das Herunterladen der Seite, wenn nur der Statuscode zählt. Auswahl: GET, HEAD, POST, PUT, PATCH, DELETE, OPTIONS. |
| `accepted_status` | Accepted status codes | `200-299` | Als normal betrachtete Codes oder Bereiche, kommagetrennt. Jeder andere Code zählt als down. Beispiel: `200-299,301,404`. |
| `keyword` | Expected keyword | *(leer)* | Text, der auf der Seite erscheinen muss. Fehlt er, wird der Dienst als down gemeldet, auch wenn die Seite antwortet. |
| `keyword_absent` | Keyword must be absent | `false` | Kehrt die Prüfung um: Das Vorhandensein des Schlüsselworts signalisiert einen Fehlschlag. Nützlich für eine Fehlerseite, die mit 200 antwortet. |
| `keyword_case_sensitive` | Match keyword case | `false` | Standardmäßig werden Groß- und Kleinschreibung gleich behandelt. |
| `json_path` | JSON path to check | *(leer)* | Für eine JSON-Antwort: Pfad des zu prüfenden Werts. Zusammen mit dem erwarteten Wert ausfüllen, nie eines ohne das andere. Beispiel: `$.status`. |
| `json_expect` | Expected JSON value | *(leer)* | Wert, den der obige JSON-Pfad haben muss, zum Beispiel "ok" oder "true". |
| `headers` | Extra headers | *(leer)* | Ein oder mehrere "Name: Wert"-Header, durch "\|" getrennt. Legen Sie hier keine Secrets ab: Optionen sind in den Diagrammen sichtbar. |
| `body` | Request body | *(leer)* | Mit dem Request gesendeter Inhalt, für Methoden, die einen erwarten (POST, PUT…). |
| `follow_redirects` | Follow redirects | `true` | Deaktivieren, um die Weiterleitung selbst zu überwachen, zum Beispiel ein 301 zu HTTPS. |
| `max_redirects` | Maximum redirects followed | `10` | Zwischen 1 und 20. Keine Wirkung, wenn Weiterleitungen nicht gefolgt wird. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | Die Prüfung schlägt nicht mehr bei einem selbstsignierten Zertifikat oder einem einer privaten Autorität fehl. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Standardmäßig lehnt die Prüfung Adressen ab, die nur der DumbMonit-Host selbst erreichen kann (127.0.0.1, ::1, 169.254.x.x). Aktivieren Sie das, um einen auf dem DumbMonit-Host laufenden Dienst zu überwachen. Private LAN-Adressen (10.x, 192.168.x) sind immer erlaubt. |
| `check_certificate` | Read the certificate | `true` | Über HTTPS wird auch das Ablaufdatum des Zertifikats aufgezeichnet, sodass Sie vor dem Ablauf gewarnt werden. |
| `max_body_bytes` | Maximum bytes read | `524288` | Darüber hinaus wird der Rest der Seite nicht herunterladen. Das Schlüsselwort und der JSON-Pfad werden nur in diesem Teil gesucht. |
| `user_agent` | Announced identity (User-Agent) | `DumbMonit/<version>` | Name, den die Prüfung dem Server gibt. Ändern Sie es, wenn der Server Robots filtert. |
| `timeout_seconds` | Timeout (seconds) | `5` | Zeit, nach der der Dienst als down gemeldet wird, wenn er nicht geantwortet hat. Zwischen 1 und 60. |

## TCP

**Network port (TCP)** — prüft, dass ein Port Verbindungen
akzeptiert: SSH, Datenbank, Dateifreigabe, Game-Server,
Netzwerkdrucker.

### Einrichtung

1. Schreiben Sie in die Adresse den Host gefolgt vom Port, durch
   einen Doppelpunkt getrennt: "nas.home.lan:22" oder
   "192.168.1.5:445".
2. Für eine IPv6-Adresse setzen Sie sie in Klammern:
   "[fd00::1]:445".
3. Sie können die Adresse auch ohne Port lassen und ihn in der
   Option "Port" eingeben: eines der beiden ist erforderlich.
4. Nur die Verbindungsöffnung wird getestet: Keine Daten werden an
   den Dienst gesendet, daher hat es keine Auswirkung auf ihn.

!!! warning
    Ein offener Port beweist nicht, dass die Anwendung dahinter
    funktioniert. Für einen Web-Dienst bevorzugen Sie die Art
    "Website or web API", die die Antwort liest.

### Optionen

| Schlüssel | Label | Standard | Hilfe |
|---|---|---|---|
| `port` | Port | *(leer)* | Zu öffnender Port, falls die Adresse ihn nicht schon als "host:port" gibt. Eines der beiden ist erforderlich. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Standardmäßig lehnt die Prüfung Adressen ab, die nur der DumbMonit-Host selbst erreichen kann (127.0.0.1, ::1, 169.254.x.x). Aktivieren Sie das, um einen auf dem DumbMonit-Host laufenden Dienst zu überwachen. Private LAN-Adressen (10.x, 192.168.x) sind immer erlaubt. |
| `timeout_seconds` | Timeout (seconds) | `5` | Zeit, nach der der Dienst als down gemeldet wird, wenn er nicht geantwortet hat. Zwischen 1 und 60. |

## DNS

**Domain name (DNS)** — prüft, dass ein Name aufgelöst wird, und dass
er auf die richtige Adresse zeigt. Beispiele: Ihr Domainname, ein von
Ihrem Pi-hole oder AdGuard bedienter interner Name, ein MX-Eintrag.

### Einrichtung

1. Schreiben Sie in die Adresse den aufzulösenden Namen, ohne
   "http://": "www.example.com".
2. Standardmäßig fragt die Prüfung den System-Resolver nach einer
   IPv4-Adresse (A-Eintrag). Wählen Sie bei Bedarf eine andere
   Eintragsart in den Optionen.
3. Um Ihren eigenen DNS-Server zu überwachen, geben Sie seine
   IP-Adresse in "DNS server to query" ein: Die Prüfung schlägt fehl,
   wenn er aufhört zu antworten.
4. Um Hijacking oder eine Fehlkonfiguration zu erkennen, listen Sie
   unter "Expected values" die Adressen, die die Antwort enthalten
   muss.

!!! warning
    Der DNS-Server wird durch seine IP-Adresse angegeben, nicht
    durch einen Namen: Es würde einen Resolver brauchen, um den
    Resolver aufzulösen.

### Optionen

| Schlüssel | Label | Standard | Hilfe |
|---|---|---|---|
| `record_type` | Record type | `A` | A für eine IPv4-Adresse, AAAA für IPv6, MX für Mail, CNAME für einen Alias… Auswahl: A, AAAA, CNAME, MX, TXT, NS, SOA, SRV, PTR, CAA. |
| `resolver` | DNS server to query | *(leer)* | IP-Adresse eines Resolvers, Port optional. Leer: der System-Resolver. Praktisch, um Ihren eigenen DNS-Server zu prüfen. Beispiel: `1.1.1.1` oder `10.0.0.1:5353`. |
| `expect` | Expected values | *(leer)* | Werte, die alle in der Antwort erscheinen müssen, kommagetrennt. Leer: nur die Auflösung wird geprüft. |
| `expect_mode` | Comparison | `contains` | `contains`: Jeder erwartete Wert muss irgendwo in der Antwort erscheinen. `exact`: Die Antwort muss diese Werte halten und nichts sonst. |
| `forbid` | Forbidden values | *(leer)* | Werte, die nie in der Antwort erscheinen dürfen, kommagetrennt. Vor den erwarteten geprüft. |
| `alert_on_change` | Alert when the answer changes | `false` | Zeichnet einen Fingerprint der sortierten Antwort auf und alarmiert, wann immer er sich ändert, ohne die erwarteten Werte schreiben zu müssen. |
| `timeout_seconds` | Timeout (seconds) | `5` | Zeit, nach der der Dienst als down gemeldet wird, wenn er nicht geantwortet hat. Zwischen 1 und 60. |

### Behaupten, was eine Zone beantwortet

`expect` allein fängt den verschwundenen Eintrag. Es fängt nicht den
*hinzugefügten* Eintrag: Eine entführte Zone behält oft den
legitimen `A` und stellt einen zweiten daneben, und ein Client
erreicht dann zufällig einen von beiden.

- `expect_mode = exact` lehnt jeden Wert ab, den die Erwartung nicht
  abdeckt. Reihenfolge spielt keine Rolle — DNS hält keine.
- `forbid` benennt Werte, die nie zurückkommen dürfen: die Adresse
  eines früheren Hosts, ein abgelaufenes Validierungs-`TXT`, ein
  stillgelegter Mailserver. Es wird zuerst geprüft, daher gewinnt es
  über `expect`.

Beide vergleichen ohne Berücksichtigung der Groß-/Kleinschreibung,
ignorieren den abschließenden Punkt eines Namens und die
Anführungszeichen um ein `TXT`, und behandeln einen erwarteten Wert
als gefunden, wenn er innerhalb eines Eintrags erscheint — daher
passt ein `MX` auf `mail.example.com`, ohne dass Sie seine Priorität
kopieren müssten.

### Alarmieren, wenn sich die Antwort ändert

Die erwarteten Werte zu schreiben ist nicht immer praktisch: ein von
einem CDN bedienter Name, Einträge, die jemand anderes verwaltet,
oder einfach eine Zone, über die Sie informiert werden wollen, wann
immer sie sich bewegt. Kreuzen Sie **Alert when the answer changes**
an: Jede Prüfung zeichnet dann einen Fingerprint der Antwort auf,
und die eingebaute Regel
[DNS answer changed](../alerting/rules.md#reverse-proxies-and-domains)
feuert, wenn er sich vom vorherigen unterscheidet. Der Alarm löst
sich eine Stunde später von selbst; die neue Antwort ist die
Referenz von da an.

Der Fingerprint deckt die Werte der Antwort ab, sortiert und ohne
Duplikate, mit eingeschlossener CNAME-Kette: ein Round-Robin, das
dieselben Adressen in anderer Reihenfolge zurückgibt, ändert ihn
nicht, ebenso wenig eine TTL. Ein Name, dessen Antwort sich bei
jeder Abfrage ändert (ein großer Pool wie `pool.ntp.org`, oder
geografisches DNS, das unterschiedlich von einem Resolver zum
nächsten antwortet), würde dauerhaft alarmieren: Lassen Sie die
Option dafür aus, oder fragen Sie einen festen Resolver ab.

## Ping

**Reachable host (ping)** — sendet ein paar ICMP-Echoes und misst
Antwortzeit und Paketverlust. Beispiele: Heim-Router oder Gateway,
WLAN-Access-Point, Drucker, Maschine ohne Agent oder SNMP, entfernter
Host.

### Einrichtung

1. Schreiben Sie in die Adresse den Hostnamen oder die IP-Adresse:
   "192.168.1.1" oder "router.home.lan".
2. Bei jeder Abfrage sendet die Prüfung vier Echoes und misst die
   Antwortzeit, zusammen mit dem Anteil verlorener Pakete.
3. Standardmäßig zählt nur totaler Verlust als down; teilweiser
   Verlust bleibt in den Diagrammen sichtbar. Senken Sie "Tolerated
   loss", um früher gewarnt zu werden.
4. Läuft DumbMonit in Docker, behalten Sie die `sysctls:`-Zeilen der
   mitgelieferten docker-compose.yml unter dem Dienst dumbmonit: Sie
   erlauben dem nicht-privilegierten Benutzer des Containers, ICMP-
   Echo-Sockets zu öffnen. Keine Capability ist nötig.

!!! warning
    Ohne dieses sysctl kann die Prüfung keinen ICMP-Socket öffnen:
    Sie meldet das als Konfigurationsfehler, nicht als
    Host-Ausfall. Manche Geräte ignorieren Pings auch absichtlich:
    Prüfen Sie das, bevor Sie Schlüsse ziehen.

```yaml
services:
  dumbmonit:
    sysctls:
      net.ipv4.ping_group_range: "0 2147483647"
```

Außerhalb von Docker tut `sysctl -w net.ipv4.ping_group_range="0
2147483647"` auf dem Host dasselbe, und ein Raw-Socket (`NET_RAW`,
root) funktioniert auch.

### Optionen

| Schlüssel | Label | Standard | Hilfe |
|---|---|---|---|
| `count` | Number of echoes | `4` | Bei jeder Abfrage gesendete Pakete, von 1 bis 20. |
| `packet_timeout_ms` | Wait per packet (ms) | `1000` | Erlaubte Zeit für jede Antwort, von 50 bis 10.000 ms. |
| `interval_ms` | Gap between two echoes (ms) | `100` | Pause zwischen zwei Echoes, von 0 bis 5.000 ms. |
| `payload_bytes` | Payload size (bytes) | `56` | Von jedem Echo getragene Daten, von 0 bis 1.400 Bytes. |
| `ip_version` | IP version | `auto` | "auto" nimmt die erste aufgelöste Adresse; erzwingen Sie 4 oder 6, wenn der Host beide hat und einer nicht antwortet. Auswahl: auto, 4, 6. |
| `loss_threshold_percent` | Tolerated loss (%) | `100` | Über diesem Prozentsatz verlorener Pakete wird der Host als down gemeldet. 100: Nur totaler Verlust zählt, teilweiser Verlust bleibt in den Diagrammen sichtbar. |
| `timeout_seconds` | Timeout (seconds) | `5` | Gesamtbudget für die Abfrage, von 1 bis 60. Abgelehnt, falls es "Anzahl Echoes × Wartezeit pro Paket" nicht abdeckt. |

## TLS

**TLS certificate** — prüft, dass ein Zertifikat gültig ist und warnt
vor seinem Ablauf, auf jedem verschlüsselten Port. Beispiele:
Mailserver (IMAPS, SMTPS), Reverse Proxy, LDAPS-Verzeichnis,
MQTT-Broker, Administrationsoberfläche.

### Einrichtung

1. Schreiben Sie in die Adresse den Host und, falls er nicht 443
   ist, den Port des verschlüsselten Dienstes:
   "mail.example.com:993", "ldap.home.lan:636".
2. Die Prüfung öffnet eine verschlüsselte Verbindung, liest das
   präsentierte Zertifikat und schließt sofort: Keine Daten werden
   mit der Anwendung ausgetauscht.
3. Sie zeichnet die Anzahl der Tage bis zum Ablauf, den Aussteller
   und die TLS-Version auf. Die mitgelieferten Alarmregeln warnen
   nach vierzehn Tagen, dann bei Ablauf.
4. Wird der Dienst über seine IP-Adresse erreicht, aber das
   Zertifikat trägt einen Namen, geben Sie diesen Namen in "Server
   name (SNI)" ein.
5. Für ein von Ihrer eigenen Autorität ausgestelltes Zertifikat
   kreuzen Sie "Accept an unverifiable certificate" an: Das
   Ablaufdatum wird weiterhin überwacht.

!!! warning
    Für eine Website liest die Art "Website or web API" das
    Zertifikat schon: Diese Art ist für Dienste gedacht, die nicht
    HTTP sprechen, oder deren Anwendung Sie nicht treffen wollen.

Der Handshake wird abgeschlossen, auch wenn das Zertifikat abgelehnt
wird: Das erlaubt, das Ablaufdatum eines schon abgelaufenen
Zertifikats zu lesen. Das Urteil der Verifizierung steht in
`probe_ssl_cert_valid`.

### Optionen

| Schlüssel | Label | Standard | Hilfe |
|---|---|---|---|
| `server_name` | Server name (SNI) | *(leer)* | Dem Server angekündigter Domainname, im Zertifikat geprüft. Setzen Sie es, wenn die Adresse eine IP hinter einem Reverse Proxy ist. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | Eine nicht verifizierbare Kette zählt nicht mehr als Fehlschlag: Das Ablaufdatum wird weiterhin aufgezeichnet. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Standardmäßig lehnt die Prüfung Adressen ab, die nur der DumbMonit-Host selbst erreichen kann (127.0.0.1, ::1, 169.254.x.x). Aktivieren Sie das, um einen auf dem DumbMonit-Host laufenden Dienst zu überwachen. Private LAN-Adressen (10.x, 192.168.x) sind immer erlaubt. |
| `timeout_seconds` | Timeout (seconds) | `5` | Zeit, nach der der Dienst als down gemeldet wird, wenn er nicht geantwortet hat. Zwischen 1 und 60. |

## SMTP

**Mail relay (SMTP)** — öffnet eine echte Sitzung auf einem
Mailserver: Begrüßung, EHLO, STARTTLS, und den Login, falls
angegeben. Beispiele: das SMTP-Relay Ihres Providers, ein lokaler
Postfix oder msmtp, Proxmox Mail Gateway, der Submission-Port eines
Mailservers.

### Einrichtung

1. Schreiben Sie in die Adresse den Mailserver, und den Port, falls
   er nicht der übliche ist: "smtp.example.com" oder
   "smtp.example.com:2525".
2. Wählen Sie die Verschlüsselung, die Ihr Relay nutzt: STARTTLS
   für den Submission-Port 587, TLS für Port 465, None für ein
   Relay in Ihrem eigenen Netzwerk auf Port 25.
3. Um zu prüfen, dass das Relay Ihr Konto noch akzeptiert, füllen
   Sie das Credential aus: Die Prüfung führt dann AUTH aus und
   meldet ein abgelehntes Passwort als solches, nicht als Ausfall.
4. Nichts wird je gesendet: Die Prüfung stoppt nach der Begrüßung,
   den Erweiterungen und dem optionalen Login, legt dann auf. Keine
   Nachricht landet in der Warteschlange.
5. Um informiert zu werden, wenn eine Erweiterung verschwindet,
   benennen Sie sie in "Expected extension": Ein Relay, das aufhört,
   STARTTLS anzukündigen, ist eines, das Ihre Mail im Klartext senden
   würde.

!!! warning
    Eine akzeptierte Sitzung beweist nicht, dass Mail herausgeht.
    Ein Relay, das alles akzeptiert und für immer in die
    Warteschlange legt, antwortet hier perfekt: Beobachten Sie die
    Warteschlangen selbst dafür.

Credentials: keine, oder ein Benutzername und Passwort, gesendet mit
`AUTH PLAIN` (oder `AUTH LOGIN`, wenn der Server nur das anbietet).
Sie werden **nach** der Verschlüsselung der Verbindung und nach der
Beurteilung des Zertifikats gesendet: Ein Relay, dessen Kette nicht
verifiziert werden kann, erhält sie nie.

Über TLS oder STARTTLS wird das Zertifikat gelesen wie die
TLS-Prüfung es liest — Ablauf, Aussteller, Protokollversion —
daher ist ein Mail-Relay auch ein Zertifikat, vor dem Sie gewarnt
werden.

### Was es nicht erkennen kann

Dass Mail tatsächlich herausgeht. Ein Relay, das jede Sitzung
akzeptiert und dann alles verzögert, antwortet hier perfekt;
beobachten Sie stattdessen seine Warteschlangen (siehe die
Proxmox-Mail-Gateway-Seite). Es sendet auch nie eine Nachricht,
daher sagt es nichts über Empfängerfilterung, SPF oder DKIM.

### Optionen

| Schlüssel | Label | Standard | Hilfe |
|---|---|---|---|
| `security` | Encryption | `starttls` | STARTTLS startet im Klartext und upgradet (Port 587), TLS verschlüsselt vom ersten Byte (Port 465), None verschlüsselt nie (Port 25) und passt nur zu einem Relay in Ihrem eigenen Netzwerk. |
| `port` | Port | *(leer)* | Genutzt, falls die Adresse keinen angibt. Leer: 587 mit STARTTLS, 465 mit TLS, 25 ohne Verschlüsselung. |
| `helo_name` | Name announced (EHLO) | `dumbmonit` | Name, den die Prüfung bei der Vorstellung gibt. Ein strenges Relay lehnt einen Namen ab, den es nicht auflösen kann. |
| `expect_capability` | Expected extension | *(leer)* | Erweiterung, die der Server in seiner EHLO-Antwort ankündigen muss, zum Beispiel STARTTLS oder AUTH. Leer: keine Erwartung. |
| `server_name` | Server name (SNI) | *(der Host)* | Dem Server angekündigter Domainname, im Zertifikat geprüft. Setzen Sie es, wenn die Adresse eine IP ist. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | Eine nicht verifizierbare Kette zählt nicht mehr als Fehlschlag: Das Ablaufdatum wird weiterhin aufgezeichnet. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Siehe "Adressen, die die Prüfungen ablehnen" unten. |
| `timeout_seconds` | Timeout (seconds) | `5` | Zeit, nach der der Dienst als down gemeldet wird, wenn er nicht geantwortet hat. Zwischen 1 und 60. |

### Fehlergründe

| `reason` | Was passierte |
|---|---|
| `connect` | Der Port ist geschlossen, oder das Relay ist nicht erreichbar. |
| `protocol` | Etwas antwortete, aber nicht SMTP — oder es lehnte die Verbindung ab, `EHLO`, oder das angekündigte STARTTLS. |
| `auth` | Das Konto wurde abgelehnt (`535`, `530`, `534`). Das Relay ist in Ordnung; das Credential nicht. |
| `payload` | Die in "Expected extension" benannte Erweiterung wird nicht angekündigt. |
| `tls`, `cert_expired` | Dieselbe Bedeutung wie bei der TLS-Prüfung. |

## PostgreSQL

**PostgreSQL database** — verbindet, authentifiziert und führt eine
Abfrage aus: das Signal für eine Datenbank, die up ist, aber nicht
mehr antwortet. Beispiele: die Datenbank hinter Nextcloud oder
Immich, ein Home-Assistant-Recorder, eine Anwendungsdatenbank, eine
TimescaleDB-Instanz.

### Einrichtung

1. Erstellen Sie auf dem Server ein Konto für die Überwachung und
   geben Sie ihm nichts weiter als das Recht, sich zu verbinden:
   CREATE ROLE dumbmonit LOGIN PASSWORD 'a-long-password';
2. Stellen Sie sicher, dass dieses Konto den Server von der
   DumbMonit-Maschine aus erreichen darf, was meist eine weitere
   Zeile in pg_hba.conf und ein Konfigurations-Reload bedeutet.
3. Schreiben Sie in die Adresse den Server, und den Port, falls er
   nicht 5432 ist: "db.home.lan" oder "db.home.lan:5433".
4. Die Standardabfrage ist SELECT 1, die keine Tabelle liest: Nichts
   sonst muss gewährt werden. Verbindungszeit und Abfragezeit werden
   getrennt aufgezeichnet.
5. Um etwas Eigenes zu beobachten, ersetzen Sie die Abfrage durch
   eine, die eine einzige Zahl zurückgibt, und es wird ein Diagramm:
   eine Warteschlangenlänge, eine Zeilenzahl, eine Replikationslatenz.

!!! warning
    Die Prüfung öffnet bei jeder Abfrage eine echte Verbindung. Auf
    einer schon nahe ihrem Verbindungslimit liegenden Instanz geben
    Sie ihr ein längeres Intervall.

Credentials: ein Benutzername und Passwort, erforderlich. Eine
Datenbank wird nie anonym überwacht, daher bietet das Formular
nichts sonst.

### Was es nicht erkennen kann

Replikationslatenz, ein sich ziehendes Lock, eine wachsende Tabelle.
All das lebt in den eigenen Administrationsansichten der Engine —
schreiben Sie die Abfrage selbst, und die zurückgegebene Zahl wird
`probe_sql_value`, ein Diagramm, auf das Sie alarmieren können.

### Optionen

| Schlüssel | Label | Standard | Hilfe |
|---|---|---|---|
| `port` | Port | `5432` | Genutzt, falls die Adresse keinen angibt. |
| `database` | Database | `postgres` | Bei der Verbindung geöffnete Datenbank. Das Monitoring-Konto muss sich damit verbinden dürfen. |
| `query` | Query | `SELECT 1` | Bei jeder Prüfung ausgeführt. Der Standard liest keine Tabelle, daher braucht das Konto kein Recht über die Verbindung hinaus. Eine eine Zahl zurückgebende Abfrage macht es zu einem Diagramm. |
| `expect` | Expected value | *(leer)* | Wert, den die erste Spalte der ersten Zeile halten muss. Leer: nur das Gelingen der Abfrage wird geprüft. |
| `sslmode` | Encryption | `prefer` | `prefer` verschlüsselt, wenn der Server es anbietet, `require` lehnt die Verbindung ohne es ab, `verify-full` prüft auch das Zertifikat und den Namen, `disable` verschlüsselt nie. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Siehe "Adressen, die die Prüfungen ablehnen" unten. |
| `timeout_seconds` | Timeout (seconds) | `5` | Zeit, nach der der Dienst als down gemeldet wird, wenn er nicht geantwortet hat. Zwischen 1 und 60. |

## MySQL und MariaDB

**MySQL or MariaDB database** — verbindet, authentifiziert und führt
eine Abfrage aus: das Signal für eine Datenbank, die up ist, aber
nicht mehr antwortet. Beispiele: die Datenbank hinter einem
WordPress, einem Nextcloud oder einem Kimai, eine
Anwendungsdatenbank, ein MariaDB in Docker.

### Einrichtung

1. Erstellen Sie auf dem Server ein Konto für die Überwachung ohne
   jedes Recht: CREATE USER 'dumbmonit'@'%' IDENTIFIED BY
   'a-long-password';
2. Prüfen Sie, dass sich das Konto von der DumbMonit-Maschine aus
   verbinden darf: Ein Host-Muster von localhost würde nur auf dem
   Server selbst hergestellte Verbindungen akzeptieren.
3. Schreiben Sie in die Adresse den Server, und den Port, falls er
   nicht 3306 ist: "db.home.lan" oder "db.home.lan:3307".
4. Die Standardabfrage ist SELECT 1, die keine Tabelle liest: Nichts
   sonst muss gewährt werden. Verbindungszeit und Abfragezeit werden
   getrennt aufgezeichnet.
5. Um etwas Eigenes zu beobachten, ersetzen Sie die Abfrage durch
   eine, die eine einzige Zahl zurückgibt, und es wird ein Diagramm:
   eine Warteschlangenlänge, eine Zeilenzahl, eine Replikationslatenz.

!!! warning
    Die Prüfung öffnet bei jeder Abfrage eine echte Verbindung. Auf
    einer schon nahe ihrem Verbindungslimit liegenden Instanz geben
    Sie ihr ein längeres Intervall.

### Optionen

| Schlüssel | Label | Standard | Hilfe |
|---|---|---|---|
| `port` | Port | `3306` | Genutzt, falls die Adresse keinen angibt. |
| `database` | Database | *(leer)* | Bei der Verbindung geöffnete Datenbank. Leer: keine, was für die Standardabfrage reicht. |
| `query` | Query | `SELECT 1` | Bei jeder Prüfung ausgeführt. Der Standard liest keine Tabelle, daher braucht das Konto kein Recht über die Verbindung hinaus. Eine eine Zahl zurückgebende Abfrage macht es zu einem Diagramm. |
| `expect` | Expected value | *(leer)* | Wert, den die erste Spalte der ersten Zeile halten muss. Leer: nur das Gelingen der Abfrage wird geprüft. |
| `sslmode` | Encryption | `prefer` | `prefer` verschlüsselt, wenn der Server es anbietet, `require` lehnt die Verbindung ohne es ab, `verify-full` prüft auch das Zertifikat und den Namen, `disable` verschlüsselt nie. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Siehe "Adressen, die die Prüfungen ablehnen" unten. |
| `timeout_seconds` | Timeout (seconds) | `5` | Zeit, nach der der Dienst als down gemeldet wird, wenn er nicht geantwortet hat. Zwischen 1 und 60. |

### Fehlergründe, beide Engines

| `reason` | Was passierte |
|---|---|
| `connect` | Der Port ist geschlossen, der Server ist nicht erreichbar, oder er lehnte die Verbindung aus einem anderen Grund als dem Konto ab (startet gerade, zu viele Verbindungen). |
| `auth` | Das Konto wurde abgelehnt: falsches Passwort, keine `pg_hba.conf`-Zeile, Host-Muster, das die DumbMonit-Maschine nicht abdeckt. |
| `query` | Verbunden und authentifiziert, aber die Abfrage schlug fehl: fehlende Tabelle, Datenbank im Read-Only, Syntaxfehler. |
| `payload` | Die Abfrage funktionierte, gab aber etwas anderes als den erwarteten Wert zurück. |
| `timeout` | Die Verbindung oder die Abfrage wurde nicht rechtzeitig fertig — der Fall "up, aber langsam". |

## MQTT

**MQTT broker** — verbindet sich mit dem Broker, abonniert ein
Thema, und kann auf eine zurückgehaltene Nachricht warten. Beispiele:
Mosquitto, der Broker hinter Home Assistant, Zigbee2MQTT,
ESPHome-Sensoren.

### Einrichtung

1. Schreiben Sie in die Adresse den Broker, und den Port, falls er
   nicht der übliche ist: "broker.home.lan" oder
   "broker.home.lan:1884".
2. Kreuzen Sie "Encrypted connection" für einen auf 8883 hörenden
   Broker an, und füllen Sie das Credential aus, falls ein Konto
   nötig ist.
3. Sich zu verbinden ist schon überwachenswert: Ein Broker, der Ihr
   Konto abweist, oder der aufgehört hat zu antworten, ist der
   Grund, warum das ganze Haus still wurde.
4. Um weiterzugehen, benennen Sie ein Thema: Die Prüfung abonniert
   es dann, und ein Broker, der das Abonnement abweist, sagt Ihnen,
   dass das Konto seine Zugriffsrechte verloren hat.
5. Kreuzen Sie "Expect a retained message" nur für ein Thema an, das
   eine trägt, typischerweise ein Verfügbarkeits-Thema. Ein Thema,
   auf das nur manchmal veröffentlicht wird, sieht für einen gerade
   verbundenen Client still aus.

!!! warning
    Die Prüfung veröffentlicht nie etwas. Sie kann auch nicht sagen,
    wie alt eine zurückgehaltene Nachricht ist: MQTT datiert sie
    nicht, daher zählt ein vor drei Wochen eingefrorener Wert immer
    noch als vorhanden.

### Was es nicht erkennen kann

Dass ein bestimmter Sensor aufgehört hat zu senden. Die
zurückgehaltene Nachricht, die die Prüfung liest, ist, was der
Broker behalten hat, möglicherweise drei Wochen alt — MQTT versieht
sie nicht mit einem Zeitstempel. Für Frische beobachten Sie die
Automatisierung, die sie konsumiert, oder nutzen Sie einen Heartbeat.

### Optionen

| Schlüssel | Label | Standard | Hilfe |
|---|---|---|---|
| `tls` | Encrypted connection | `false` | Verschlüsselt die Sitzung vom ersten Byte, wie Broker es auf Port 8883 tun. |
| `port` | Port | *(leer)* | Genutzt, falls die Adresse keinen angibt. Leer: 1883 im Klartext, 8883 verschlüsselt. |
| `topic` | Topic | *(leer)* | Thema, das die Prüfung abonniert. Leer: Sie verbindet sich nur, was schon sagt, dass der Broker lebt und Ihr Konto akzeptiert. |
| `expect_message` | Expect a retained message | `false` | Eine Nachricht muss vor dem Timeout auf diesem Thema ankommen. Nur zurückgehaltene Nachrichten kommen sofort an: ein Thema, auf das nur gelegentlich veröffentlicht wird, sieht still aus. |
| `expect` | Expected content | *(leer)* | Text, den diese Nachricht enthalten muss. Es auszufüllen impliziert, eine Nachricht zu erwarten. |
| `client_id` | Client identifier | *(abgeleitet)* | Dem Broker angekündigter Name. Der Standard leitet sich vom Gerät ab, sodass zwei Prüfungen sich nie gegenseitig trennen. |
| `server_name` | Server name (SNI) | *(der Host)* | Dem Broker angekündigter Domainname, im Zertifikat geprüft. Setzen Sie es, wenn die Adresse eine IP ist. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | Eine nicht verifizierbare Kette zählt nicht mehr als Fehlschlag: Das Ablaufdatum wird weiterhin aufgezeichnet. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Siehe "Adressen, die die Prüfungen ablehnen" unten. |
| `timeout_seconds` | Timeout (seconds) | `5` | Zeit, nach der der Dienst als down gemeldet wird, wenn er nicht geantwortet hat. Zwischen 1 und 60. |

### Fehlergründe

| `reason` | Was passierte |
|---|---|
| `connect` | Der Port ist geschlossen, oder der Broker ist nicht erreichbar. |
| `auth` | Das `CONNACK` lehnte das Konto ab, oder der Broker lehnte das Abonnement ab — eine Zugriffskontrollliste deckt dieses Thema nicht mehr ab. |
| `protocol` | Etwas antwortete, spricht aber nicht MQTT 3.1.1, oder der Broker lehnte die Client-ID ab. |
| `payload` | Keine zurückgehaltene Nachricht kam auf dem Thema an, oder ihr Inhalt enthält den erwarteten Text nicht. |

## WebSocket

**WebSocket endpoint** — führt den Upgrade-Handshake aus, und kann
einen Frame senden und auf einen warten: ein anderer Fehlschlag als
eine einfache HTTP-Seite. Beispiele: die Home-Assistant-API, ein
Live-Dashboard, ein Log-Stream, ein Endpunkt hinter einem Reverse
Proxy.

### Einrichtung

1. Fügen Sie in die Adresse den vollständigen Endpunkt ein:
   "wss://home.example.com/api/websocket". Ohne Schema wird wss
   angenommen.
2. Die Prüfung führt den gesamten Öffnungs-Handshake aus und
   verifiziert die Antwort, die der Server aus dem gesendeten
   Schlüssel berechnet: Ein Reverse Proxy, der den Upgrade-Header
   verloren hat, wird hier erfasst, wo eine HTTP-Prüfung noch einen
   gesunden 200 sehen würde.
3. Nichts sonst ist für die meisten Endpunkte nötig. Ein anderer
   Status als 101 wird mit seinem Code gemeldet, daher wird ein 401
   eines abgelaufenen Tokens nicht mit einem Ausfall verwechselt.
4. Um weiterzugehen, füllen Sie "Expected content" mit einem
   Fragment des ersten vom Server selbst gesendeten Frames, zum
   Beispiel der Begrüßung der Home-Assistant-API.
5. Sagt der Endpunkt nichts, bis er angesprochen wird, legen Sie
   eine Nachricht in "Frame to send" und was seine Antwort enthalten
   muss in "Expected content".

!!! warning
    Die Prüfung öffnet und legt innerhalb einer Sekunde auf. Eine
    Verbindung, die eine Firewall oder ein Proxy-Timeout nach
    dreißig Sekunden abschneidet, sieht hier völlig gesund aus.

Credentials: keine, ein Benutzername und Passwort (als
HTTP-Basic-Authentifizierung im Öffnungs-Request gesendet) oder ein
Token (als `Authorization: Bearer` gesendet).

### Optionen

| Schlüssel | Label | Standard | Hilfe |
|---|---|---|---|
| `path` | Path | *(aus der Adresse)* | Pfad des Öffnungs-Requests. Aus der Adresse genommen, wenn sie einen trägt. |
| `port` | Port | *(leer)* | Genutzt, falls die Adresse keinen angibt. Leer: 443 für wss, 80 für ws. |
| `send` | Frame to send | *(leer)* | Text-Frame, gesendet, sobald die Verbindung offen ist. Leer: nichts wird gesendet. |
| `expect` | Expected content | *(leer)* | Text, den ein empfangener Frame enthalten muss. Leer und ohne etwas zu senden, wird nur der Öffnungs-Handshake geprüft. |
| `subprotocol` | Subprotocol | *(leer)* | Wert von Sec-WebSocket-Protocol. Die Prüfung schlägt fehl, wenn der Server ein anderes wählt. |
| `origin` | Origin | *(leer)* | Wert des Origin-Headers, den manche Server vor dem Upgrade verlangen. |
| `server_name` | Server name (SNI) | *(der Host)* | Dem Server angekündigter Domainname, im Zertifikat geprüft. Setzen Sie es, wenn die Adresse eine IP ist. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | Eine nicht verifizierbare Kette zählt nicht mehr als Fehlschlag: Das Ablaufdatum wird weiterhin aufgezeichnet. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Siehe "Adressen, die die Prüfungen ablehnen" unten. |
| `timeout_seconds` | Timeout (seconds) | `5` | Zeit, nach der der Dienst als down gemeldet wird, wenn er nicht geantwortet hat. Zwischen 1 und 60. |

### Fehlergründe

| `reason` | Was passierte |
|---|---|
| `connect` | Der Port ist geschlossen, oder der Host ist nicht erreichbar. |
| `status` | Der Endpunkt antwortete mit etwas anderem als `101`; `probe_http_status_code` trägt den Code. |
| `protocol` | Er antwortete mit `101`, aber sein `Sec-WebSocket-Accept` passt nicht zum gesendeten Schlüssel, er wählte ein anderes Subprotokoll, oder er schloss die Verbindung sofort. |
| `payload` | Kein Frame kam an, oder er enthält nicht den erwarteten Text. |

## NTP

**NTP time server** — sendet eine einzige SNTP-Anfrage und liest die
Antwort: Versatz, Round-Trip-Verzögerung, Stratum und
Leap-Indikator. Beispiele: ein Router oder NAS, der als lokaler
Zeitserver agiert, ein interner NTP-Server (chrony, ntpd), ein
öffentlicher Pool-Server als Referenz.

### Einrichtung

1. Schreiben Sie in die Adresse den Namen oder die IP-Adresse des
   Servers: "pool.ntp.org" oder "192.168.1.1". UDP-Port 123 wird
   genutzt, sofern die Option "Port" nichts anderes sagt.
2. Die Prüfung sendet eine einzige SNTP-Anfrage und liest die
   Antwort: Kein Zustand wird zwischen Abfragen behalten, und
   nichts wird auf den Server geschrieben.
3. Sie zeichnet den Versatz zwischen der Uhr des Servers und dem
   DumbMonit-Host, die Round-Trip-Verzögerung, das Stratum und den
   Leap-Indikator auf.
4. Ein Server, der Stratum 16 antwortet, oder dessen
   Leap-Indikator "unsynchronized" liest, hat sich nie
   synchronisieren können: Die Prüfung meldet ihn als down, egal
   was der Versatz sagt.
5. Senken Sie "Tolerated offset", um früher als bei 100 ms gewarnt
   zu werden; heben Sie es für einen mehrere Hops von einer
   Referenzuhr entfernten Server an.

!!! warning
    Der Versatz ist nur so gut wie die eigene Uhr des
    DumbMonit-Hosts: Ist dieser Host selbst nicht synchronisiert
    gehalten (NTP, die Zeitsynchronisation eines Hypervisors),
    spiegelt der gemessene Versatz seine Drift genauso wie die des
    Servers.

Credentials: keine.

### Optionen

| Schlüssel | Label | Standard | Hilfe |
|---|---|---|---|
| `port` | Port | `123` | UDP-Port des NTP-Servers. |
| `offset_threshold_ms` | Tolerated offset (ms) | `100` | Über diesem Uhrenversatz wird der Server als down gemeldet. Ein nicht synchronisierter Server (Stratum 16) schlägt immer fehl, egal dieser Wert. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Siehe "Adressen, die die Prüfungen ablehnen" unten. |
| `timeout_seconds` | Timeout (seconds) | `5` | Zeit, nach der der Dienst als down gemeldet wird, wenn er nicht geantwortet hat. Zwischen 1 und 60. |

### Fehlergründe

| `reason` | Was passierte |
|---|---|
| `connect` | Der UDP-Socket konnte nicht geöffnet werden, oder die Antwort kam von einer unerwarteten Adresse. |
| `timeout` | Keine Antwort kam innerhalb des Timeouts an. |
| `protocol` | Die Antwort war kein gültiges NTP-Paket, oder kam nicht von einem Server (falscher Modus). |
| `unsynchronized` | Der Server antwortete mit Stratum 16 oder 0 (ein "kiss-o'-death", der auch den Grund im Log benennt), oder sein Leap-Indikator liest "unsynchronized". |
| `clock_offset` | Der gemessene Versatz überschreitet "Tolerated offset". |

## Adressen, die die Prüfungen ablehnen

Die Prüfungen, die sich mit einer von Ihnen gegebenen Adresse
verbinden — HTTP, TCP, TLS, NTP, SMTP, PostgreSQL, MySQL, MQTT,
WebSocket — melden, was sie sehen — was auch, Wort für Wort, das ist,
was eine Server-Side-Request-Forgery tut. Um zu verhindern, dass ein
Monitoring-Admin Dienste liest, die nur der DumbMonit-Host erreichen
kann (die eingebettete Metrikdatenbank auf `127.0.0.1:8428`, der
Metadata-Dienst eines Cloud-Providers auf `169.254.169.254`), lehnen
die Prüfungen standardmäßig ab:

- Loopback-Adressen (`127.0.0.0/8`, `::1`);
- Link-Local-Adressen (`169.254.0.0/16`, `fe80::/10`), einschließlich
  der Cloud-Metadata-Adresse, und die unspezifizierte Adresse
  (`0.0.0.0`, `::`).

Private LAN-Bereiche (`10.0.0.0/8`, `172.16.0.0/12`,
`192.168.0.0/16`, `fc00::/7`) sind immer erlaubt: Sie zu überwachen
ist der Sinn eines Homelabs.

Die Regel gilt für die von Ihnen eingegebene Adresse, für jede
Adresse, zu der ein Hostname aufgelöst wird (bei der Verbindung
erneut geprüft, sodass ein Name, der seine Antwort ändert, nicht
durchschlüpfen kann), und für jede Weiterleitung, der die
HTTP-Prüfung folgt. Eine abgelehnte Adresse zeigt sich als
Konfigurationsfehler am Target — kein Down-Alarm wird gesendet —
und benennt die zu aktivierende Option: `allow_private_targets`.
Schalten Sie es für ein Target ein, das wirklich einen Dienst auf
dem DumbMonit-Host selbst überwacht.

Eine Weiterleitung zu einer abgelehnten Adresse wird als
fehlgeschlagene Messung gemeldet (`reason="connect"`), weil es der
überwachte Dienst ist, der sich geändert hat, nicht die
Konfiguration.

An einem `json_path` gefundene Werte werden in einer Fehlermeldung
nie über 32 Zeichen hinaus wiedergegeben.

## Häufige Fehler

| Symptom | Wahrscheinliche Ursache |
|---|---|
| Ping zeigt einen Konfigurationsfehler | Das sysctl `net.ipv4.ping_group_range` fehlt in der Compose-Datei. Siehe oben. |
| HTTP down mit `reason="keyword"` | Das Schlüsselwort wird nur in den ersten `max_body_bytes` gesucht, ohne Berücksichtigung der Groß-/Kleinschreibung, sofern `keyword_case_sensitive` nicht gesetzt ist. |
| HTTP down mit `reason="tls"` | Selbstsigniert oder private Autorität: setzen Sie `insecure_tls`. |
| Eine Prüfung zeigt einen Konfigurationsfehler, der `allow_private_targets` nennt | Die Adresse ist Loopback oder Link-Local. Siehe "Adressen, die die Prüfungen ablehnen" oben. |
| TCP down, obwohl `telnet host port` von Ihrem Laptop aus funktioniert | Der DumbMonit-Container kann den Host nicht erreichen (Docker-Netzwerk, Firewall). Testen Sie von innerhalb des Netzwerks des Containers. |
| DNS down mit `reason="record"` | Die Antwort enthält nicht jeden in `expect` gelisteten Wert. |
| Ping down mit `reason="packet_loss"` | Verlust über `loss_threshold_percent`; manche Geräte limitieren ICMP. |
| DNS down mit `reason="record"`, obwohl `expect` passt | `expect_mode` ist `exact`, und die Antwort hält einen zusätzlichen Wert, oder `forbid` passte. |
| SMTP down mit `reason="protocol"` bei `security = starttls` | Das Relay kündigt STARTTLS auf diesem Port nicht an. Nutzen Sie `tls` für 465, `none` für ein einfaches lokales Relay. |
| Eine Datenbankprüfung down mit `reason="auth"` | Das Konto existiert, darf sich aber vielleicht nicht von der DumbMonit-Maschine aus verbinden: Prüfen Sie `pg_hba.conf`, oder den Host-Teil des MySQL-Benutzers. |
| Eine Datenbankprüfung down mit `reason="query"` | Verbindung und Login funktionierten. Die Datenbank ist up und verweigert die Arbeit: Read-Only, fehlende Tabelle, Recovery. |
| MQTT down mit `reason="payload"` auf einem Thema mit klar erkennbarem Traffic | Nur *zurückgehaltene* Nachrichten erreichen einen gerade abonnierten Client. Veröffentlichen Sie dieses Thema mit dem Retain-Flag, oder entfernen Sie das Häkchen bei "Expect a retained message". |
| WebSocket down mit `reason="protocol"` nach einem `101` | Etwas auf dem Weg beantwortet den Upgrade, ohne WebSocket zu sprechen — meist eine Reverse-Proxy-Regel, die den `Upgrade`-Header verschluckt. |
