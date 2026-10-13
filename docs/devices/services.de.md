# Dienste: HTTP, TCP, DNS, Ping, TLS, NTP, SMTP, SQL, MQTT, WebSocket {#services-http-tcp-dns-ping-tls-ntp-smtp-sql-mqtt-websocket}

Elf Monitore überwachen **Dienste** statt Geräte, im Stil von Uptime Kuma: eine Webseite, einen
Port, einen DNS-Namen, einen Host, ein Zertifikat, einen Zeitserver, ein Mail-Relay, eine
Datenbank, einen MQTT-Broker, einen WebSocket-Endpunkt. Jeder Dienst erhält auf seiner
Geräteseite einen Verlaufsbalken, eine Antwortzeit und einen Verfügbarkeitsprozentwert.

Die letzten fünf gehen über das Öffnen einer Verbindung hinaus: Sie beginnen eine echte Sitzung
im eigenen Protokoll des Dienstes. Das ist der Unterschied zwischen „Port 5432 ist offen“ und
„die Datenbank akzeptiert mein Konto noch und beantwortet eine Abfrage“. NTP liegt dazwischen:
eine einzelne Anfrage und Antwort im eigenen Protokoll von NTP, genug, um die Uhr des Servers
ohne Sitzung zu lesen.

## Wie der Dienstzustand funktioniert {#how-service-state-works}

Ein Dienst kann antworten und trotzdem ausgefallen sein: ein `500`, ein fehlendes Schlüsselwort,
ein abgelaufenes Zertifikat. Diese Monitore schreiben daher bei jeder Prüfung einen Punkt,
`probe_success` ist 1 oder 0, statt wie ein Hardwaregerät zu verstummen. Das ermöglicht einen
Verfügbarkeitsprozentwert: `avg_over_time(dumbmonit_probe_success[30d])`.

| Situation | Was passiert |
|---|---|
| Ungültige Option oder Adresse, ICMP-Sockets nicht erlaubt (siehe Ping) | Konfigurationsfehler: Es wird nichts geschrieben, die Geräteseite zeigt den Fehler, kein Alarm „ausgefallen“. |
| Verbindung abgelehnt, Timeout, TLS abgelehnt, `500`, Schlüsselwort fehlt, totaler Paketverlust, Passwort abgelehnt, Abfrage fehlgeschlagen | `probe_success = 0`: Der Ausfall wird aufgezeichnet, datiert und gezählt. |
| Alles in Ordnung | `probe_success = 1`. |

Jeder Monitor hat sein eigenes Timeout (`timeout_seconds`, standardmäßig 5 s, höchstens 60 s),
kürzer als `DUMBMONIT_PROBE_TIMEOUT_SECS` des Servers: Würde ihn der Scheduler unterbrechen,
könnte er seine Null nicht mehr schreiben.

Integrierte Regeln, die für alle elf gelten: Dienst ausgefallen (3 Minuten), Dienst flattert
(mehr als sechs Zustandswechsel in dreißig Minuten), Langsamer Dienst (mehr als 3 s zehn Minuten
lang). Für jede Prüfung, die ein Zertifikat liest — `http`, `tls`, `smtp`, `mqtt`, `websocket`:
Zertifikat läuft bald ab (14 Tage) und Zertifikat abgelaufen.

Ein abgelehntes Passwort ist nie ein Ausfall des Dienstes: Es wird als `reason="auth"`
aufgezeichnet, sodass der Alarm sagt, was zu korrigieren ist.

### Metriken {#metrics}

Alles Gauges, alle mit dem Präfix `dumbmonit_`, alle zusätzlich zu `target`, `host` und `tag_*`
mit dem Label `probe="http|tcp|dns|ping|tls|smtp|postgres|mysql|mqtt|websocket|ntp"`.

| Metrik | Monitore | Bedeutung |
|---|---|---|
| `probe_success` | alle | 1, wenn der Dienst korrekt antwortet, sonst 0. |
| `probe_duration_seconds` | alle | Gesamtdauer der Prüfung. |
| `probe_failure_info` | alle | Präsenz (1) mit einem Label `reason`: `dns`, `connect`, `timeout`, `tls`, `cert_expired`, `status`, `keyword`, `json`, `body`, `packet_loss`, `record`, `auth`, `protocol`, `query`, `payload`. |
| `probe_http_status_code` | http, websocket | Erhaltener Statuscode. |
| `probe_http_first_byte_seconds` | http | Zeit bis zu den Antwort-Headern. |
| `probe_http_content_bytes` | http | Größe des Bodys. |
| `probe_connect_seconds` | alle außer dns und ping | Verbindungsaufbau (bei Datenbanken inklusive Anmeldung). |
| `probe_tls_handshake_seconds` | tls, http, smtp, mqtt, websocket | Nur die TLS-Aushandlung. |
| `probe_tls_version_info` | tls, http, smtp, mqtt, websocket | Präsenz; Label `version`. |
| `probe_ssl_cert_expiry_days` | tls, http, smtp, mqtt, websocket | Tage bis zum Ablauf, negativ wenn abgelaufen. |
| `probe_ssl_cert_valid` | tls, http, smtp, mqtt, websocket | 1, wenn die Kette zu einer bekannten Stelle führt. |
| `probe_ssl_cert_issuer_info` | tls, http, smtp, mqtt, websocket | Präsenz; Label `issuer`. |
| `probe_dns_lookup_seconds` | dns | Auflösungszeit. |
| `probe_dns_answer_records` | dns | Einträge des angefragten Typs. |
| `probe_dns_answer_fingerprint` | dns | Mit „Alarm bei Änderung der Antwort“: ein 32-Bit-Fingerprint der sortierten Antwort. Sein Wert bedeutet nichts; nur seine Änderungen. |
| `probe_icmp_rtt_seconds`, `_min_seconds`, `_max_seconds` | ping | Durchschnittliche, kürzeste und längste Round-Trip-Zeit. |
| `probe_icmp_packet_loss_ratio` | ping | Verlust zwischen 0 und 1. |
| `probe_icmp_packets_sent`, `probe_icmp_packets_received` | ping | Gesendete und empfangene Echos. |
| `probe_smtp_greeting_seconds` | smtp | Zeit bis zum `220`-Banner. |
| `probe_smtp_ehlo_seconds` | smtp | Round-Trip des Befehls `EHLO`. |
| `probe_smtp_capabilities` | smtp | Angekündigte Erweiterungen. |
| `probe_smtp_authenticated` | smtp | 1, wenn `AUTH` erfolgreich war. |
| `probe_sql_query_seconds` | postgres, mysql | Nur die Ausführung der Abfrage. |
| `probe_sql_rows` | postgres, mysql | Zurückgegebene Zeilen. |
| `probe_sql_value` | postgres, mysql | Erste Spalte der ersten Zeile, wenn sie eine Zahl ist. |
| `probe_mqtt_connack_seconds` | mqtt | Zeit bis zum `CONNACK`. |
| `probe_mqtt_suback_seconds` | mqtt | Zeit bis zum `SUBACK`. |
| `probe_mqtt_message_bytes` | mqtt | Größe der Retained Message. |
| `probe_mqtt_message_value` | mqtt | Ihr Inhalt, wenn er eine Zahl ist. |
| `probe_ws_handshake_seconds` | websocket | Eröffnungs-Handshake. |
| `probe_ntp_offset_seconds` | ntp | Abweichung zwischen der lokalen Uhr und dem Server. |
| `probe_ntp_delay_seconds` | ntp | Netzwerk-Round-Trip-Verzögerung. |
| `probe_ntp_stratum` | ntp | Abstand zu einer Referenzuhr (16: nicht synchronisiert). |
| `probe_ntp_leap_indicator` | ntp | 0 bis 3 (Schaltsekundenwarnung, 3 = Uhr nicht synchronisiert). |
| `probe_ntp_root_delay_seconds` | ntp | Aufsummierte Verzögerung bis zur Wurzel der Hierarchie. |
| `probe_ntp_root_dispersion_seconds` | ntp | Aufsummierte Dispersion bis zur Wurzel. |
| `probe_ntp_reference_info` | ntp | Präsenz; Label `reference_id`. |
| `probe_ws_message_bytes` | websocket | Größe des ersten empfangenen Frames. Nur vorhanden, wenn die Prüfung eine Nachricht sendet oder eine Antwort erwartet (Optionen `send` / `expect`). |

## HTTP {#http}

**Website oder Web-API (HTTP)** — prüft, ob eine Seite oder eine API mit dem richtigen
Statuscode, dem richtigen Inhalt und einem gültigen Zertifikat antwortet. Beispiele: eine
Website, eine Health-Seite einer Anwendung, eine REST-API, die Oberfläche eines selbst
gehosteten Dienstes.

### Einrichtung {#setup}

1. Fügen Sie in die Adresse die vollständige URL der zu überwachenden Seite ein. Ohne „http://“
   oder „https://“ wird HTTPS angenommen.
2. Bevorzugen Sie eine leichte Seite ohne Anmeldung, zum Beispiel die Health-Seite der Anwendung
   („/health“, „/status“), statt der Startseite.
3. Verlangt die Seite eine Authentifizierung, füllen Sie die Zugangsdaten aus: Benutzername und
   Passwort ergeben Basic-Authentifizierung, ein Token wird als „Bearer“ gesendet.
4. Standardmäßig ist jeder Statuscode zwischen 200 und 299 in Ordnung. Um weiter zu gehen,
   verlangen Sie ein Schlüsselwort auf der Seite oder einen bestimmten Wert in einer JSON-Antwort.
5. Über HTTPS wird das Zertifikat automatisch gelesen: Sie werden vor seinem Ablauf gewarnt.

!!! warning
    Schreiben Sie nie ein Passwort oder Token in die Optionen: Sie werden bei jeder Messung im
    Klartext kopiert. Verwenden Sie das Zugangsdatenfeld, das verschlüsselt ist.

Zugangsdaten: keine, Benutzername / Passwort (Basic-Authentifizierung) oder API-Token
(`Authorization: Bearer`).

### Optionen {#options}

| Schlüssel | Label | Standard | Hilfe |
|---|---|---|---|
| `method` | HTTP-Methode | `GET` | GET passt auf fast jeden Fall. HEAD vermeidet das Herunterladen der Seite, wenn nur der Statuscode zählt. Auswahl: GET, HEAD, POST, PUT, PATCH, DELETE, OPTIONS. |
| `accepted_status` | Akzeptierte Statuscodes | `200-299` | Als normal geltende Codes oder Bereiche, durch Kommas getrennt. Jeder andere Code zählt als ausgefallen. Beispiel: `200-299,301,404`. |
| `keyword` | Erwartetes Schlüsselwort | *(leer)* | Text, der auf der Seite vorkommen muss. Fehlt er, wird der Dienst als ausgefallen gemeldet, obwohl die Seite antwortet. |
| `keyword_absent` | Schlüsselwort muss fehlen | `false` | Kehrt die Prüfung um: Das Vorhandensein des Schlüsselworts signalisiert einen Fehler. Nützlich für eine Fehlerseite, die mit 200 antwortet. |
| `keyword_case_sensitive` | Schlüsselwort case-sensitiv | `false` | Standardmäßig werden Groß- und Kleinschreibung gleich behandelt. |
| `json_path` | Zu prüfender JSON-Pfad | *(leer)* | Bei einer JSON-Antwort: Pfad des zu prüfenden Werts. Füllen Sie ihn zusammen mit dem erwarteten Wert aus, nie das eine ohne das andere. Beispiel: `$.status`. |
| `json_expect` | Erwarteter JSON-Wert | *(leer)* | Wert, den der obige JSON-Pfad haben muss, zum Beispiel „ok“ oder „true“. |
| `headers` | Zusätzliche Header | *(leer)* | Ein oder mehrere Header „Name: Wert“, durch „\|“ getrennt. Legen Sie hier keine Geheimnisse ab: Optionen sind in den Diagrammen sichtbar. |
| `body` | Anfrage-Body | *(leer)* | Mit der Anfrage gesendeter Inhalt, für Methoden, die einen erwarten (POST, PUT …). |
| `follow_redirects` | Weiterleitungen folgen | `true` | Deaktivieren, um die Weiterleitung selbst zu überwachen, zum Beispiel ein 301 auf HTTPS. |
| `max_redirects` | Maximal verfolgte Weiterleitungen | `10` | Zwischen 1 und 20. Keine Wirkung, wenn Weiterleitungen nicht verfolgt werden. |
| `insecure_tls` | Nicht prüfbares Zertifikat akzeptieren | `false` | Die Prüfung scheitert nicht mehr an einem selbstsignierten Zertifikat oder einem einer privaten Stelle. |
| `allow_private_targets` | Loopback- und Link-Local-Ziele erlauben | `false` | Standardmäßig lehnt die Prüfung Adressen ab, die nur der DumbMonit-Host selbst erreicht (127.0.0.1, ::1, 169.254.x.x). Aktivieren Sie dies, um einen auf dem DumbMonit-Host laufenden Dienst zu überwachen. Private LAN-Adressen (10.x, 192.168.x) sind immer erlaubt. |
| `check_certificate` | Das Zertifikat lesen | `true` | Zeichnet über HTTPS auch das Ablaufdatum des Zertifikats auf, damit Sie vor dem Ablauf gewarnt werden. |
| `max_body_bytes` | Maximal gelesene Bytes | `524288` | Darüber hinaus wird der Rest der Seite nicht heruntergeladen. Schlüsselwort und JSON-Pfad werden nur in diesem Teil gesucht. |
| `user_agent` | Angegebene Identität (User-Agent) | `DumbMonit/<version>` | Name, den die Prüfung dem Server gibt. Ändern Sie ihn, wenn der Server Roboter filtert. |
| `timeout_seconds` | Timeout (Sekunden) | `5` | Zeit, nach der der Dienst als ausgefallen gemeldet wird, wenn er nicht geantwortet hat. Zwischen 1 und 60. |

## TCP {#tcp}

**Netzwerkport (TCP)** — prüft, ob ein Port Verbindungen annimmt: SSH, Datenbank, Dateifreigabe,
Spieleserver, Netzwerkdrucker.

### Einrichtung {#setup_1}

1. Schreiben Sie in die Adresse den Host, gefolgt vom Port, durch einen Doppelpunkt getrennt:
   „nas.home.lan:22“ oder „192.168.1.5:445“.
2. Setzen Sie eine IPv6-Adresse in eckige Klammern: „[fd00::1]:445“.
3. Sie können die Adresse auch ohne Port lassen und ihn in der Option „Port“ eintragen: Eines
   von beidem ist erforderlich.
4. Getestet wird nur das Öffnen der Verbindung: Es werden keine Daten an den Dienst gesendet,
   daher hat es keine Wirkung auf ihn.

!!! warning
    Ein offener Port beweist nicht, dass die dahinterliegende Anwendung funktioniert. Bevorzugen
    Sie für einen Webdienst den Typ „Website oder Web-API“, der die Antwort liest.

### Optionen {#options_1}

| Schlüssel | Label | Standard | Hilfe |
|---|---|---|---|
| `port` | Port | *(leer)* | Zu öffnender Port, falls die Adresse ihn nicht bereits als „host:port“ angibt. Eines von beidem ist erforderlich. |
| `allow_private_targets` | Loopback- und Link-Local-Ziele erlauben | `false` | Standardmäßig lehnt die Prüfung Adressen ab, die nur der DumbMonit-Host selbst erreicht (127.0.0.1, ::1, 169.254.x.x). Aktivieren Sie dies, um einen auf dem DumbMonit-Host laufenden Dienst zu überwachen. Private LAN-Adressen (10.x, 192.168.x) sind immer erlaubt. |
| `timeout_seconds` | Timeout (Sekunden) | `5` | Zeit, nach der der Dienst als ausgefallen gemeldet wird, wenn er nicht geantwortet hat. Zwischen 1 und 60. |

## DNS {#dns}

**Domainname (DNS)** — prüft, ob ein Name aufgelöst wird und auf die richtige Adresse zeigt.
Beispiele: Ihr Domainname, ein von Ihrem Pi-hole oder AdGuard bedienter interner Name, ein
MX-Eintrag.

### Einrichtung {#setup_2}

1. Schreiben Sie in die Adresse den aufzulösenden Namen, ohne „http://“: „www.example.com“.
2. Standardmäßig fragt die Prüfung den System-Resolver nach einer IPv4-Adresse (A-Eintrag).
   Wählen Sie bei Bedarf in den Optionen einen anderen Eintragstyp.
3. Um Ihren eigenen DNS-Server zu überwachen, geben Sie seine IP-Adresse unter „Abzufragender
   DNS-Server“ ein: Die Prüfung scheitert, wenn er nicht mehr antwortet.
4. Um Hijacking oder eine Fehlkonfiguration zu erkennen, listen Sie unter „Erwartete Werte“ die
   Adressen auf, die die Antwort enthalten muss.

!!! warning
    Der DNS-Server wird über seine IP-Adresse angegeben, nicht über einen Namen: Es bräuchte
    einen Resolver, um den Resolver aufzulösen.

### Optionen {#options_2}

| Schlüssel | Label | Standard | Hilfe |
|---|---|---|---|
| `record_type` | Eintragstyp | `A` | A für eine IPv4-Adresse, AAAA für IPv6, MX für Mail, CNAME für einen Alias … Auswahl: A, AAAA, CNAME, MX, TXT, NS, SOA, SRV, PTR, CAA. |
| `resolver` | Abzufragender DNS-Server | *(leer)* | IP-Adresse eines Resolvers, Port optional. Leer: der System-Resolver. Praktisch, um Ihren eigenen DNS-Server zu prüfen. Beispiel: `1.1.1.1` oder `10.0.0.1:5353`. |
| `expect` | Erwartete Werte | *(leer)* | Werte, die alle in der Antwort vorkommen müssen, durch Kommas getrennt. Leer: Es wird nur die Auflösung geprüft. |
| `expect_mode` | Vergleich | `contains` | `contains`: Jeder erwartete Wert muss irgendwo in der Antwort vorkommen. `exact`: Die Antwort muss diese Werte enthalten und nichts anderes. |
| `forbid` | Verbotene Werte | *(leer)* | Werte, die in der Antwort nie vorkommen dürfen, durch Kommas getrennt. Wird vor den erwarteten geprüft. |
| `alert_on_change` | Alarm bei Änderung der Antwort | `false` | Zeichnet einen Fingerprint der sortierten Antwort auf und alarmiert bei jeder Änderung, ohne dass die erwarteten Werte geschrieben werden müssen. |
| `timeout_seconds` | Timeout (Sekunden) | `5` | Zeit, nach der der Dienst als ausgefallen gemeldet wird, wenn er nicht geantwortet hat. Zwischen 1 und 60. |

### Zusichern, was eine Zone antwortet {#asserting-what-a-zone-answers}

`expect` allein fängt den Eintrag, der verschwunden ist. Es fängt nicht den Eintrag, der
*hinzugekommen* ist: Eine gekaperte Zone behält oft das legitime `A` und setzt ein zweites daneben,
und ein Client erreicht dann zufällig das eine oder das andere.

- `expect_mode = exact` lehnt jeden Wert ab, den die Erwartung nicht abdeckt. Die Reihenfolge
  spielt keine Rolle — DNS behält keine.
- `forbid` nennt Werte, die nie wiederkehren dürfen: die Adresse eines früheren Hosts, ein
  abgelaufenes Validierungs-`TXT`, einen stillgelegten Mailserver. Es wird zuerst geprüft und hat
  daher Vorrang vor `expect`.

Beide vergleichen ohne Beachtung der Groß-/Kleinschreibung, ignorieren den abschließenden Punkt
eines Namens und die Anführungszeichen um ein `TXT` und behandeln einen erwarteten Wert als
gefunden, wenn er innerhalb eines Eintrags vorkommt — ein `MX` passt also auf
`mail.example.com`, ohne dass Sie seine Priorität kopieren müssen.

### Alarmieren, wenn sich die Antwort ändert {#alerting-when-the-answer-changes}

Die erwarteten Werte zu schreiben ist nicht immer praktisch: ein von einem CDN bedienter Name,
Einträge, die jemand anderes verwaltet, oder schlicht eine Zone, bei der Sie von jeder Änderung
erfahren wollen. Haken Sie **Alarm bei Änderung der Antwort** an: Jede Prüfung zeichnet dann einen
Fingerprint der Antwort auf, und die integrierte Regel
[DNS-Antwort geändert](../alerting/rules.md#reverse-proxies-and-domains) löst aus, wenn er von
dem davor abweicht. Der Alarm löst sich eine Stunde später von selbst auf; die neue Antwort ist
ab da die Referenz.

Der Fingerprint deckt die Werte der Antwort ab, sortiert und ohne Duplikate, mit eingeschlossener
CNAME-Kette: Ein Round-Robin, der dieselben Adressen in anderer Reihenfolge liefert, ändert ihn
nicht, ebenso wenig eine TTL. Ein Name, dessen Antwort sich bei jeder Abfrage ändert (ein großer
Pool wie `pool.ntp.org` oder geografisches DNS, das von Resolver zu Resolver anders antwortet),
würde ständig alarmieren: Lassen Sie die Option dafür aus oder fragen Sie einen festen Resolver ab.

## Ping {#ping}

**Erreichbarer Host (Ping)** — sendet einige ICMP-Echos und misst Antwortzeit und Paketverlust.
Beispiele: Heimrouter oder Gateway, WLAN-Access-Point, Drucker, Maschine ohne Agent oder SNMP,
entfernter Host.

### Einrichtung {#setup_3}

1. Schreiben Sie in die Adresse den Hostnamen oder die IP-Adresse: „192.168.1.1“ oder
   „router.home.lan“.
2. Bei jeder Abfrage sendet die Prüfung vier Echos und misst die Antwortzeit sowie den Anteil
   verlorener Pakete.
3. Standardmäßig zählt nur ein totaler Verlust als ausgefallen; ein Teilverlust bleibt in den
   Diagrammen sichtbar. Senken Sie „Tolerierter Verlust“, um früher gewarnt zu werden.
4. Läuft DumbMonit in Docker, behalten Sie die Zeilen `sysctls:` der mitgelieferten
   docker-compose.yml unter dem Dienst dumbmonit: Sie erlauben dem unprivilegierten Benutzer des
   Containers, ICMP-Echo-Sockets zu öffnen. Es ist keine Capability nötig.

!!! warning
    Ohne diesen Sysctl kann die Prüfung keinen ICMP-Socket öffnen: Sie meldet das als
    Konfigurationsfehler, nicht als Hostausfall. Manche Geräte ignorieren Pings außerdem
    absichtlich: Prüfen Sie das, bevor Sie Schlüsse ziehen.

```yaml
services:
  dumbmonit:
    sysctls:
      net.ipv4.ping_group_range: "0 2147483647"
```

Außerhalb von Docker leistet `sysctl -w net.ipv4.ping_group_range="0 2147483647"` auf dem Host
dasselbe, und ein Raw-Socket (`NET_RAW`, root) funktioniert ebenfalls.

### Optionen {#options_3}

| Schlüssel | Label | Standard | Hilfe |
|---|---|---|---|
| `count` | Anzahl der Echos | `4` | Bei jeder Abfrage gesendete Pakete, von 1 bis 20. |
| `packet_timeout_ms` | Wartezeit pro Paket (ms) | `1000` | Für jede Antwort erlaubte Zeit, von 50 bis 10.000 ms. |
| `interval_ms` | Abstand zwischen zwei Echos (ms) | `100` | Pause zwischen zwei Echos, von 0 bis 5.000 ms. |
| `payload_bytes` | Nutzlastgröße (Bytes) | `56` | Von jedem Echo getragene Daten, von 0 bis 1.400 Bytes. |
| `ip_version` | IP-Version | `auto` | „auto“ nimmt die erste aufgelöste Adresse; erzwingen Sie 4 oder 6, wenn der Host beide hat und eine nicht antwortet. Auswahl: auto, 4, 6. |
| `loss_threshold_percent` | Tolerierter Verlust (%) | `100` | Über diesem Prozentsatz verlorener Pakete wird der Host als ausgefallen gemeldet. 100: Nur ein totaler Verlust zählt, ein Teilverlust bleibt in den Diagrammen sichtbar. |
| `timeout_seconds` | Timeout (Sekunden) | `5` | Gesamtbudget für die Abfrage, von 1 bis 60. Abgelehnt, wenn es „Anzahl der Echos × Wartezeit pro Paket“ nicht abdeckt. |

## TLS {#tls}

**TLS-Zertifikat** — prüft, ob ein Zertifikat gültig ist, und warnt vor seinem Ablauf, auf jedem
verschlüsselten Port. Beispiele: Mailserver (IMAPS, SMTPS), Reverse-Proxy, LDAPS-Verzeichnis,
MQTT-Broker, Verwaltungsoberfläche.

### Einrichtung {#setup_4}

1. Schreiben Sie in die Adresse den Host und, falls nicht 443, den Port des verschlüsselten
   Dienstes: „mail.example.com:993“, „ldap.home.lan:636“.
2. Die Prüfung öffnet eine verschlüsselte Verbindung, liest das präsentierte Zertifikat und
   schließt sofort wieder: Mit der Anwendung werden keine Daten ausgetauscht.
3. Sie zeichnet die Tage bis zum Ablauf, den Aussteller und die TLS-Version auf. Die
   mitgelieferten Alarmregeln warnen bei vierzehn Tagen, dann beim Ablauf.
4. Wird der Dienst über seine IP-Adresse erreicht, das Zertifikat trägt aber einen Namen, geben
   Sie diesen Namen unter „Servername (SNI)“ ein.
5. Für ein von Ihrer eigenen Stelle ausgestelltes Zertifikat haken Sie „Nicht prüfbares
   Zertifikat akzeptieren“ an: Das Ablaufdatum wird weiterhin überwacht.

!!! warning
    Bei einer Website liest der Typ „Website oder Web-API“ das Zertifikat bereits: Dieser Typ ist
    für Dienste gedacht, die kein HTTP sprechen oder deren Anwendung Sie nicht belasten wollen.

Der Handshake wird auch bei abgelehntem Zertifikat abgeschlossen: Das ermöglicht, das
Ablaufdatum eines bereits abgelaufenen Zertifikats zu lesen. Das Urteil der Überprüfung steht in
`probe_ssl_cert_valid`.

### Optionen {#options_4}

| Schlüssel | Label | Standard | Hilfe |
|---|---|---|---|
| `server_name` | Servername (SNI) | *(leer)* | Dem Server angekündigter und im Zertifikat geprüfter Domainname. Setzen Sie ihn, wenn die Adresse eine IP hinter einem Reverse-Proxy ist. |
| `insecure_tls` | Nicht prüfbares Zertifikat akzeptieren | `false` | Eine nicht prüfbare Kette zählt nicht mehr als Fehler: Das Ablaufdatum wird weiterhin aufgezeichnet. |
| `allow_private_targets` | Loopback- und Link-Local-Ziele erlauben | `false` | Standardmäßig lehnt die Prüfung Adressen ab, die nur der DumbMonit-Host selbst erreicht (127.0.0.1, ::1, 169.254.x.x). Aktivieren Sie dies, um einen auf dem DumbMonit-Host laufenden Dienst zu überwachen. Private LAN-Adressen (10.x, 192.168.x) sind immer erlaubt. |
| `timeout_seconds` | Timeout (Sekunden) | `5` | Zeit, nach der der Dienst als ausgefallen gemeldet wird, wenn er nicht geantwortet hat. Zwischen 1 und 60. |

## SMTP {#smtp}

**Mail-Relay (SMTP)** — öffnet eine echte Sitzung auf einem Mailserver: Begrüßung, EHLO,
STARTTLS und die Anmeldung, wenn Sie eine angeben. Beispiele: das SMTP-Relay Ihres Providers, ein
lokales Postfix oder msmtp, Proxmox Mail Gateway, der Submission-Port eines Mailservers.

### Einrichtung {#setup_5}

1. Schreiben Sie in die Adresse den Mailserver und, falls nicht der übliche, den Port:
   „smtp.example.com“ oder „smtp.example.com:2525“.
2. Wählen Sie die von Ihrem Relay verwendete Verschlüsselung: STARTTLS für den Submission-Port
   587, TLS für Port 465, Keine für ein Relay in Ihrem eigenen Netzwerk auf Port 25.
3. Um zu prüfen, dass das Relay Ihr Konto noch akzeptiert, füllen Sie die Zugangsdaten aus: Die
   Prüfung führt dann AUTH aus und meldet ein abgelehntes Passwort als solches, nicht als
   Ausfall.
4. Es wird nie etwas gesendet: Die Prüfung endet nach der Begrüßung, den Erweiterungen und der
   optionalen Anmeldung und legt dann auf. Keine Nachricht gelangt in die Warteschlange.
5. Um gewarnt zu werden, wenn eine Erweiterung verschwindet, nennen Sie sie unter „Erwartete
   Erweiterung“: Ein Relay, das STARTTLS nicht mehr ankündigt, würde Ihre Mail im Klartext senden.

!!! warning
    Eine akzeptierte Sitzung beweist nicht, dass Mail das Haus verlässt. Ein Relay, das alles
    akzeptiert und es ewig in die Warteschlange stellt, antwortet hier einwandfrei: Überwachen Sie
    dafür die Warteschlangen selbst.

Zugangsdaten: keine, oder ein Benutzername und Passwort, gesendet mit `AUTH PLAIN` (oder
`AUTH LOGIN`, wenn der Server nur das anbietet). Sie werden **nach** der Verschlüsselung der
Verbindung und nachdem das Zertifikat beurteilt wurde gesendet: Ein Relay, dessen Kette sich nicht
prüfen lässt, erhält sie nie.

Über TLS oder STARTTLS wird das Zertifikat so gelesen wie von der TLS-Prüfung — Ablauf, Aussteller,
Protokollversion —, sodass ein Mail-Relay auch ein Zertifikat ist, vor dessen Ablauf Sie gewarnt
werden.

### Was nicht erkannt werden kann {#what-it-cannot-detect}

Dass Mail tatsächlich hinausgeht. Ein Relay, das jede Sitzung akzeptiert und dann alles
zurückstellt, antwortet hier einwandfrei; überwachen Sie stattdessen seine Warteschlangen (siehe
die Seite zu Proxmox Mail Gateway). Es sendet auch nie eine Nachricht, sagt also nichts über
Empfängerfilterung, SPF oder DKIM.

### Optionen {#options_5}

| Schlüssel | Label | Standard | Hilfe |
|---|---|---|---|
| `security` | Verschlüsselung | `starttls` | STARTTLS beginnt im Klartext und wechselt (Port 587), TLS verschlüsselt ab dem ersten Byte (Port 465), Keine verschlüsselt nie (Port 25) und passt nur zu einem Relay in Ihrem eigenen Netzwerk. |
| `port` | Port | *(leer)* | Wird verwendet, wenn die Adresse keinen angibt. Leer: 587 mit STARTTLS, 465 mit TLS, 25 ohne Verschlüsselung. |
| `helo_name` | Angekündigter Name (EHLO) | `dumbmonit` | Name, den die Prüfung bei der Vorstellung nennt. Ein strenges Relay lehnt einen Namen ab, den es nicht auflösen kann. |
| `expect_capability` | Erwartete Erweiterung | *(leer)* | Erweiterung, die der Server in seiner EHLO-Antwort ankündigen muss, zum Beispiel STARTTLS oder AUTH. Leer: keine Erwartung. |
| `server_name` | Servername (SNI) | *(der Host)* | Dem Server angekündigter und im Zertifikat geprüfter Domainname. Setzen Sie ihn, wenn die Adresse eine IP ist. |
| `insecure_tls` | Nicht prüfbares Zertifikat akzeptieren | `false` | Eine nicht prüfbare Kette zählt nicht mehr als Fehler: Das Ablaufdatum wird weiterhin aufgezeichnet. |
| `allow_private_targets` | Loopback- und Link-Local-Ziele erlauben | `false` | Siehe „Adressen, die die Prüfungen ablehnen“ unten. |
| `timeout_seconds` | Timeout (Sekunden) | `5` | Zeit, nach der der Dienst als ausgefallen gemeldet wird, wenn er nicht geantwortet hat. Zwischen 1 und 60. |

### Fehlergründe {#failure-reasons}

| `reason` | Was geschah |
|---|---|
| `connect` | Der Port ist geschlossen oder das Relay nicht erreichbar. |
| `protocol` | Etwas hat geantwortet, aber nicht SMTP — oder es hat die Verbindung, `EHLO` oder ein angekündigtes STARTTLS abgelehnt. |
| `auth` | Das Konto wurde abgelehnt (`535`, `530`, `534`). Das Relay ist in Ordnung; die Zugangsdaten nicht. |
| `payload` | Die unter „Erwartete Erweiterung“ genannte Erweiterung wird nicht angekündigt. |
| `tls`, `cert_expired` | Gleiche Bedeutung wie bei der TLS-Prüfung. |

## PostgreSQL {#postgresql}

**PostgreSQL-Datenbank** — verbindet, authentifiziert und führt eine Abfrage aus: das Signal für
eine Datenbank, die läuft, aber nicht mehr antwortet. Beispiele: die Datenbank hinter Nextcloud
oder Immich, ein Home-Assistant-Recorder, eine Anwendungsdatenbank, eine TimescaleDB-Instanz.

### Einrichtung {#setup_6}

1. Erstellen Sie auf dem Server ein Konto für die Überwachung und geben Sie ihm nicht mehr als das
   Recht, sich zu verbinden: CREATE ROLE dumbmonit LOGIN PASSWORD 'a-long-password';
2. Stellen Sie sicher, dass dieses Konto den Server vom DumbMonit-Rechner aus erreichen darf, was
   meist eine weitere Zeile in pg_hba.conf gefolgt von einem Neuladen der Konfiguration bedeutet.
3. Schreiben Sie in die Adresse den Server und, falls nicht 5432, den Port: „db.home.lan“ oder
   „db.home.lan:5433“.
4. Die Standardabfrage ist SELECT 1, die keine Tabelle liest: Sonst muss nichts gewährt werden.
   Verbindungszeit und Abfragezeit werden getrennt aufgezeichnet.
5. Um etwas Eigenes zu überwachen, ersetzen Sie die Abfrage durch eine, die eine einzelne Zahl
   zurückgibt, und sie wird zu einem Diagramm: eine Queue-Länge, eine Zeilenzahl, ein
   Replikationsverzug.

!!! warning
    Die Prüfung öffnet bei jeder Abfrage eine echte Verbindung. Geben Sie ihr bei einer Instanz,
    die ihrem Verbindungslimit schon nahe ist, ein längeres Intervall.

Zugangsdaten: ein Benutzername und Passwort, erforderlich. Eine Datenbank wird nie anonym
überwacht, daher bietet das Formular nichts anderes an.

### Was nicht erkannt werden kann {#what-it-cannot-detect_1}

Replikationsverzug, eine Sperre, die sich hinzieht, eine Tabelle, die immer weiter wächst. All das
steht in den eigenen Verwaltungsansichten der Engine — schreiben Sie die Abfrage selbst, und die
zurückgegebene Zahl wird zu `probe_sql_value`, einem Diagramm, auf das Sie alarmieren können.

### Optionen {#options_6}

| Schlüssel | Label | Standard | Hilfe |
|---|---|---|---|
| `port` | Port | `5432` | Wird verwendet, wenn die Adresse keinen angibt. |
| `database` | Datenbank | `postgres` | Bei der Verbindung geöffnete Datenbank. Das Überwachungskonto muss sich mit ihr verbinden dürfen. |
| `query` | Abfrage | `SELECT 1` | Bei jeder Prüfung ausgeführt. Die Standardabfrage liest keine Tabelle, daher braucht das Konto kein Privileg über das Verbinden hinaus. Eine Abfrage, die eine Zahl zurückgibt, macht daraus ein Diagramm. |
| `expect` | Erwarteter Wert | *(leer)* | Wert, den die erste Spalte der ersten Zeile enthalten muss. Leer: Es wird nur geprüft, ob die Abfrage gelingt. |
| `sslmode` | Verschlüsselung | `prefer` | `prefer` verschlüsselt, wenn der Server es anbietet, `require` weigert sich, ohne zu verbinden, `verify-full` prüft zusätzlich Zertifikat und Namen, `disable` verschlüsselt nie. |
| `allow_private_targets` | Loopback- und Link-Local-Ziele erlauben | `false` | Siehe „Adressen, die die Prüfungen ablehnen“ unten. |
| `timeout_seconds` | Timeout (Sekunden) | `5` | Zeit, nach der der Dienst als ausgefallen gemeldet wird, wenn er nicht geantwortet hat. Zwischen 1 und 60. |

## MySQL und MariaDB {#mysql-and-mariadb}

**MySQL- oder MariaDB-Datenbank** — verbindet, authentifiziert und führt eine Abfrage aus: das
Signal für eine Datenbank, die läuft, aber nicht mehr antwortet. Beispiele: die Datenbank hinter
einem WordPress, Nextcloud oder Kimai, eine Anwendungsdatenbank, eine MariaDB in Docker.

### Einrichtung {#setup_7}

1. Erstellen Sie auf dem Server ein Konto für die Überwachung ganz ohne Privileg:
   CREATE USER 'dumbmonit'@'%' IDENTIFIED BY 'a-long-password';
2. Prüfen Sie, dass das Konto sich vom DumbMonit-Rechner aus verbinden darf: Ein Host-Muster
   localhost würde nur auf dem Server selbst hergestellte Verbindungen akzeptieren.
3. Schreiben Sie in die Adresse den Server und, falls nicht 3306, den Port: „db.home.lan“ oder
   „db.home.lan:3307“.
4. Die Standardabfrage ist SELECT 1, die keine Tabelle liest: Sonst muss nichts gewährt werden.
   Verbindungszeit und Abfragezeit werden getrennt aufgezeichnet.
5. Um etwas Eigenes zu überwachen, ersetzen Sie die Abfrage durch eine, die eine einzelne Zahl
   zurückgibt, und sie wird zu einem Diagramm: eine Queue-Länge, eine Zeilenzahl, ein
   Replikationsverzug.

!!! warning
    Die Prüfung öffnet bei jeder Abfrage eine echte Verbindung. Geben Sie ihr bei einer Instanz,
    die ihrem Verbindungslimit schon nahe ist, ein längeres Intervall.

### Optionen {#options_7}

| Schlüssel | Label | Standard | Hilfe |
|---|---|---|---|
| `port` | Port | `3306` | Wird verwendet, wenn die Adresse keinen angibt. |
| `database` | Datenbank | *(leer)* | Bei der Verbindung geöffnete Datenbank. Leer: keine, was für die Standardabfrage genügt. |
| `query` | Abfrage | `SELECT 1` | Bei jeder Prüfung ausgeführt. Die Standardabfrage liest keine Tabelle, daher braucht das Konto kein Privileg über das Verbinden hinaus. Eine Abfrage, die eine Zahl zurückgibt, macht daraus ein Diagramm. |
| `expect` | Erwarteter Wert | *(leer)* | Wert, den die erste Spalte der ersten Zeile enthalten muss. Leer: Es wird nur geprüft, ob die Abfrage gelingt. |
| `sslmode` | Verschlüsselung | `prefer` | `prefer` verschlüsselt, wenn der Server es anbietet, `require` weigert sich, ohne zu verbinden, `verify-full` prüft zusätzlich Zertifikat und Namen, `disable` verschlüsselt nie. |
| `allow_private_targets` | Loopback- und Link-Local-Ziele erlauben | `false` | Siehe „Adressen, die die Prüfungen ablehnen“ unten. |
| `timeout_seconds` | Timeout (Sekunden) | `5` | Zeit, nach der der Dienst als ausgefallen gemeldet wird, wenn er nicht geantwortet hat. Zwischen 1 und 60. |

### Fehlergründe, beide Engines {#failure-reasons-both-engines}

| `reason` | Was geschah |
|---|---|
| `connect` | Der Port ist geschlossen, der Server nicht erreichbar, oder er hat die Verbindung aus einem anderen Grund als dem Konto abgelehnt (startet, zu viele Verbindungen). |
| `auth` | Das Konto wurde abgelehnt: falsches Passwort, keine Zeile in `pg_hba.conf`, Host-Muster, das den DumbMonit-Rechner nicht abdeckt. |
| `query` | Verbunden und authentifiziert, aber die Abfrage ist fehlgeschlagen: fehlende Tabelle, Datenbank im Read-only, Syntaxfehler. |
| `payload` | Die Abfrage funktionierte, gab aber etwas anderes als den erwarteten Wert zurück. |
| `timeout` | Die Verbindung oder die Abfrage wurde nicht rechtzeitig fertig — der Fall „läuft, aber langsam“. |

## MQTT {#mqtt}

**MQTT-Broker** — verbindet sich mit dem Broker, abonniert ein Topic und kann auf eine Retained
Message warten. Beispiele: Mosquitto, der Broker hinter Home Assistant, Zigbee2MQTT,
ESPHome-Sensoren.

### Einrichtung {#setup_8}

1. Schreiben Sie in die Adresse den Broker und, falls nicht der übliche, den Port:
   „broker.home.lan“ oder „broker.home.lan:1884“.
2. Haken Sie „Verschlüsselte Verbindung“ für einen Broker an, der auf 8883 lauscht, und füllen
   Sie die Zugangsdaten aus, wenn er ein Konto verlangt.
3. Schon das Verbinden ist überwachenswert: Ein Broker, der Ihr Konto ablehnt oder nicht mehr
   antwortet, ist der Grund, weshalb das ganze Haus verstummt ist.
4. Um weiter zu gehen, nennen Sie ein Topic: Die Prüfung abonniert es dann, und ein Broker, der
   das Abonnement ablehnt, sagt Ihnen, dass das Konto seine Zugriffsrechte verloren hat.
5. Haken Sie „Retained Message erwarten“ nur für ein Topic an, das eine trägt, typischerweise ein
   Verfügbarkeits-Topic. Ein Topic, auf das nur gelegentlich veröffentlicht wird, wirkt für einen
   gerade verbundenen Client stumm.

!!! warning
    Die Prüfung veröffentlicht nie etwas. Sie kann auch nicht sagen, wie alt eine Retained Message
    ist: MQTT datiert sie nicht, ein vor drei Wochen eingefrorener Wert zählt also weiterhin als
    vorhanden.

### Was nicht erkannt werden kann {#what-it-cannot-detect_2}

Dass ein bestimmter Sensor aufgehört hat zu senden. Die Retained Message, die die Prüfung liest,
ist das, was der Broker behalten hat, möglicherweise drei Wochen alt — MQTT versieht sie nicht
mit einem Zeitstempel. Überwachen Sie für Aktualität die Automatisierung, die sie konsumiert, oder
verwenden Sie einen Heartbeat.

### Optionen {#options_8}

| Schlüssel | Label | Standard | Hilfe |
|---|---|---|---|
| `tls` | Verschlüsselte Verbindung | `false` | Verschlüsselt die Sitzung ab dem ersten Byte, wie Broker es auf Port 8883 tun. |
| `port` | Port | *(leer)* | Wird verwendet, wenn die Adresse keinen angibt. Leer: 1883 im Klartext, 8883 verschlüsselt. |
| `topic` | Topic | *(leer)* | Topic, das die Prüfung abonniert. Leer: Sie verbindet nur, was bereits sagt, dass der Broker lebt und Ihr Konto akzeptiert. |
| `expect_message` | Retained Message erwarten | `false` | Vor dem Timeout muss auf diesem Topic eine Nachricht eintreffen. Nur Retained Messages treffen sofort ein: Ein Topic, auf das nur von Zeit zu Zeit veröffentlicht wird, wirkt stumm. |
| `expect` | Erwarteter Inhalt | *(leer)* | Text, den diese Nachricht enthalten muss. Das Ausfüllen impliziert, eine Nachricht zu erwarten. |
| `client_id` | Client-Kennung | *(abgeleitet)* | Dem Broker angekündigter Name. Der Standard wird vom Gerät abgeleitet, sodass zwei Prüfungen einander nie trennen. |
| `server_name` | Servername (SNI) | *(der Host)* | Dem Broker angekündigter und im Zertifikat geprüfter Domainname. Setzen Sie ihn, wenn die Adresse eine IP ist. |
| `insecure_tls` | Nicht prüfbares Zertifikat akzeptieren | `false` | Eine nicht prüfbare Kette zählt nicht mehr als Fehler: Das Ablaufdatum wird weiterhin aufgezeichnet. |
| `allow_private_targets` | Loopback- und Link-Local-Ziele erlauben | `false` | Siehe „Adressen, die die Prüfungen ablehnen“ unten. |
| `timeout_seconds` | Timeout (Sekunden) | `5` | Zeit, nach der der Dienst als ausgefallen gemeldet wird, wenn er nicht geantwortet hat. Zwischen 1 und 60. |

### Fehlergründe {#failure-reasons_1}

| `reason` | Was geschah |
|---|---|
| `connect` | Der Port ist geschlossen oder der Broker nicht erreichbar. |
| `auth` | Das `CONNACK` hat das Konto abgelehnt, oder der Broker hat das Abonnement abgelehnt — eine Zugriffskontrollliste deckt dieses Topic nicht mehr ab. |
| `protocol` | Etwas hat geantwortet, spricht aber kein MQTT 3.1.1, oder der Broker hat die Client-Kennung abgelehnt. |
| `payload` | Auf dem Topic ist keine Retained Message eingetroffen, oder ihr Inhalt enthält den erwarteten Text nicht. |

## WebSocket {#websocket}

**WebSocket-Endpunkt** — führt den Upgrade-Handshake aus und kann einen Frame senden und auf
einen warten: ein anderer Fehler als bei einer einfachen HTTP-Seite. Beispiele: die
Home-Assistant-API, ein Live-Dashboard, ein Log-Stream, ein Endpunkt hinter einem Reverse-Proxy.

### Einrichtung {#setup_9}

1. Fügen Sie in die Adresse den vollständigen Endpunkt ein: „wss://home.example.com/api/websocket“.
   Ohne Schema wird wss angenommen.
2. Die Prüfung führt den gesamten Eröffnungs-Handshake aus und verifiziert die Antwort, die der
   Server aus dem gesendeten Schlüssel berechnet: Ein Reverse-Proxy, der den Upgrade-Header
   verloren hat, wird hier erwischt, wo eine HTTP-Prüfung noch ein gesundes 200 sähe.
3. Für die meisten Endpunkte ist nichts weiter nötig. Ein anderer Status als 101 wird mit seinem
   Code gemeldet, ein 401 durch ein abgelaufenes Token wird also nicht mit einem Ausfall
   verwechselt.
4. Um weiter zu gehen, füllen Sie „Erwarteter Inhalt“ mit einem Fragment des ersten Frames, den
   der Server von selbst sendet, zum Beispiel der Begrüßung der Home-Assistant-API.
5. Sagt der Endpunkt nichts, bis man ihn anspricht, setzen Sie eine Nachricht in „Zu sendender
   Frame“ und was seine Antwort enthalten muss in „Erwarteter Inhalt“.

!!! warning
    Die Prüfung öffnet und legt innerhalb einer Sekunde auf. Eine Verbindung, die eine Firewall
    oder ein Proxy-Timeout nach dreißig Sekunden kappt, sieht hier völlig gesund aus.

Zugangsdaten: keine, ein Benutzername und Passwort (als HTTP-Basic-Authentifizierung in der
Eröffnungsanfrage gesendet) oder ein Token (als `Authorization: Bearer` gesendet).

### Optionen {#options_9}

| Schlüssel | Label | Standard | Hilfe |
|---|---|---|---|
| `path` | Pfad | *(aus der Adresse)* | Pfad der Eröffnungsanfrage. Wird der Adresse entnommen, wenn sie einen trägt. |
| `port` | Port | *(leer)* | Wird verwendet, wenn die Adresse keinen angibt. Leer: 443 für wss, 80 für ws. |
| `send` | Zu sendender Frame | *(leer)* | Nach dem Öffnen der Verbindung gesendeter Text-Frame. Leer: Es wird nichts gesendet. |
| `expect` | Erwarteter Inhalt | *(leer)* | Text, den ein empfangener Frame enthalten muss. Leer und ohne etwas zu senden wird nur der Eröffnungs-Handshake geprüft. |
| `subprotocol` | Subprotokoll | *(leer)* | Wert von Sec-WebSocket-Protocol. Die Prüfung scheitert, wenn der Server ein anderes wählt. |
| `origin` | Origin | *(leer)* | Wert des Origin-Headers, den manche Server vor dem Upgrade verlangen. |
| `server_name` | Servername (SNI) | *(der Host)* | Dem Server angekündigter und im Zertifikat geprüfter Domainname. Setzen Sie ihn, wenn die Adresse eine IP ist. |
| `insecure_tls` | Nicht prüfbares Zertifikat akzeptieren | `false` | Eine nicht prüfbare Kette zählt nicht mehr als Fehler: Das Ablaufdatum wird weiterhin aufgezeichnet. |
| `allow_private_targets` | Loopback- und Link-Local-Ziele erlauben | `false` | Siehe „Adressen, die die Prüfungen ablehnen“ unten. |
| `timeout_seconds` | Timeout (Sekunden) | `5` | Zeit, nach der der Dienst als ausgefallen gemeldet wird, wenn er nicht geantwortet hat. Zwischen 1 und 60. |

### Fehlergründe {#failure-reasons_2}

| `reason` | Was geschah |
|---|---|
| `connect` | Der Port ist geschlossen oder der Host nicht erreichbar. |
| `status` | Der Endpunkt hat etwas anderes als `101` geantwortet; `probe_http_status_code` trägt den Code. |
| `protocol` | Er hat `101` geantwortet, aber sein `Sec-WebSocket-Accept` passt nicht zum gesendeten Schlüssel, er hat ein anderes Subprotokoll gewählt oder die Verbindung sofort geschlossen. |
| `payload` | Es ist kein Frame eingetroffen, oder er enthält den erwarteten Text nicht. |

## NTP {#ntp}

**NTP-Zeitserver** — sendet eine einzelne SNTP-Anfrage und liest die Antwort: Abweichung,
Round-Trip-Verzögerung, Stratum und Leap Indicator. Beispiele: ein als lokaler Zeitserver
fungierender Router oder NAS, ein interner NTP-Server (chrony, ntpd), ein als Referenz verwendeter
öffentlicher Pool-Server.

### Einrichtung {#setup_10}

1. Schreiben Sie in die Adresse den Namen oder die IP-Adresse des Servers: „pool.ntp.org“ oder
   „192.168.1.1“. UDP-Port 123 wird verwendet, sofern die Option „Port“ nichts anderes sagt.
2. Die Prüfung sendet eine einzelne SNTP-Anfrage und liest die Antwort: Zwischen den Abfragen wird
   kein Zustand gehalten, und es wird nichts auf den Server geschrieben.
3. Sie zeichnet die Abweichung zwischen der Uhr des Servers und dem DumbMonit-Host, die
   Round-Trip-Verzögerung, das Stratum und den Leap Indicator auf.
4. Ein Server, der Stratum 16 antwortet oder dessen Leap Indicator „nicht synchronisiert“ lautet,
   hat sich nie synchronisieren können: Die Prüfung meldet ihn unabhängig von der Abweichung als
   ausgefallen.
5. Senken Sie „Tolerierte Abweichung“, um früher als bei 100 ms gewarnt zu werden; erhöhen Sie
   sie für einen Server, der mehrere Hops von einer Referenzuhr entfernt ist.

!!! warning
    Die Abweichung ist nur so gut wie die eigene Uhr des DumbMonit-Hosts: Wird dieser Host nicht
    selbst synchron gehalten (NTP, Zeitsynchronisation eines Hypervisors), spiegelt die gemessene
    Abweichung ebenso seine Drift wie die des Servers.

Zugangsdaten: keine.

### Optionen {#options_10}

| Schlüssel | Label | Standard | Hilfe |
|---|---|---|---|
| `port` | Port | `123` | UDP-Port des NTP-Servers. |
| `offset_threshold_ms` | Tolerierte Abweichung (ms) | `100` | Über dieser Uhrabweichung wird der Server als ausgefallen gemeldet. Ein nicht synchronisierter Server (Stratum 16) scheitert immer, unabhängig von diesem Wert. |
| `allow_private_targets` | Loopback- und Link-Local-Ziele erlauben | `false` | Siehe „Adressen, die die Prüfungen ablehnen“ unten. |
| `timeout_seconds` | Timeout (Sekunden) | `5` | Zeit, nach der der Dienst als ausgefallen gemeldet wird, wenn er nicht geantwortet hat. Zwischen 1 und 60. |

### Fehlergründe {#failure-reasons_3}

| `reason` | Was geschah |
|---|---|
| `connect` | Der UDP-Socket konnte nicht geöffnet werden, oder die Antwort kam von einer unerwarteten Adresse. |
| `timeout` | Innerhalb des Timeouts traf keine Antwort ein. |
| `protocol` | Die Antwort war kein gültiges NTP-Paket oder kam nicht von einem Server (falscher Modus). |
| `unsynchronized` | Der Server antwortete Stratum 16 oder 0 (ein „Kiss-o'-Death“, der auch den Grund im Log nennt), oder sein Leap Indicator lautet „nicht synchronisiert“. |
| `clock_offset` | Die gemessene Abweichung überschreitet „Tolerierte Abweichung“. |

## Adressen, die die Prüfungen ablehnen {#addresses-the-checks-refuse}

Die Prüfungen, die sich mit einer von Ihnen angegebenen Adresse verbinden — HTTP, TCP, TLS, NTP,
SMTP, PostgreSQL, MySQL, MQTT, WebSocket — melden, was sie sehen — was auch, Wort für Wort, das
ist, was eine Server-Side Request Forgery tut. Damit ein Monitoring-Admin keine Dienste lesen
kann, die nur der DumbMonit-Host erreicht (die eingebettete Metrikdatenbank auf
`127.0.0.1:8428`, der Metadatendienst eines Cloud-Providers auf `169.254.169.254`), lehnen die
Prüfungen standardmäßig ab:

- Loopback-Adressen (`127.0.0.0/8`, `::1`);
- Link-Local-Adressen (`169.254.0.0/16`, `fe80::/10`), einschließlich der Cloud-Metadaten-Adresse,
  und die unspezifizierte Adresse (`0.0.0.0`, `::`).

Private LAN-Bereiche (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `fc00::/7`) sind immer
erlaubt: Sie zu überwachen ist der Sinn eines Homelabs.

Die Regel gilt für die eingetippte Adresse, für jede Adresse, in die ein Hostname aufgelöst wird
(bei der Verbindung erneut geprüft, sodass ein Name, der seine Antwort ändert, nicht
durchrutschen kann), und für jede Weiterleitung, der die HTTP-Prüfung folgt. Eine abgelehnte
Adresse erscheint als Konfigurationsfehler am Ziel — es wird kein Alarm „ausgefallen“ gesendet —
und nennt die zu aktivierende Option: `allow_private_targets`. Schalten Sie sie für ein Ziel ein,
das wirklich einen Dienst auf dem DumbMonit-Host selbst überwacht.

Eine Weiterleitung auf eine abgelehnte Adresse wird als fehlgeschlagene Messung gemeldet
(`reason="connect"`), weil der überwachte Dienst sich geändert hat, nicht die Konfiguration.

Bei einem `json_path` gefundene Werte werden in einer Fehlermeldung nie über 32 Zeichen hinaus
wiedergegeben.

## Häufige Fehler {#common-errors}

| Symptom | Wahrscheinliche Ursache |
|---|---|
| Ping zeigt einen Konfigurationsfehler | Der Sysctl `net.ipv4.ping_group_range` fehlt in der Compose-Datei. Siehe oben. |
| HTTP ausgefallen mit `reason="keyword"` | Das Schlüsselwort wird nur in den ersten `max_body_bytes` gesucht, ohne Beachtung der Groß-/Kleinschreibung, sofern nicht `keyword_case_sensitive` gesetzt ist. |
| HTTP ausgefallen mit `reason="tls"` | Selbstsigniert oder private Stelle: Setzen Sie `insecure_tls`. |
| Eine Prüfung zeigt einen Konfigurationsfehler, der `allow_private_targets` nennt | Die Adresse ist Loopback oder Link-Local. Siehe „Adressen, die die Prüfungen ablehnen“ oben. |
| TCP ausgefallen, obwohl `telnet host port` von Ihrem Laptop funktioniert | Der DumbMonit-Container erreicht den Host nicht (Docker-Netzwerk, Firewall). Testen Sie aus dem Netzwerk des Containers. |
| DNS ausgefallen mit `reason="record"` | Die Antwort enthält nicht jeden in `expect` aufgeführten Wert. |
| Ping ausgefallen mit `reason="packet_loss"` | Verlust über `loss_threshold_percent`; manche Geräte begrenzen ICMP. |
| DNS ausgefallen mit `reason="record"`, obwohl `expect` passt | `expect_mode` ist `exact` und die Antwort enthält einen zusätzlichen Wert, oder `forbid` hat gegriffen. |
| SMTP ausgefallen mit `reason="protocol"` bei `security = starttls` | Das Relay kündigt STARTTLS auf diesem Port nicht an. Verwenden Sie `tls` für 465, `none` für ein einfaches lokales Relay. |
| Eine Datenbankprüfung ausgefallen mit `reason="auth"` | Das Konto existiert, darf sich aber nicht vom DumbMonit-Rechner aus verbinden: Prüfen Sie `pg_hba.conf` oder den Host-Teil des MySQL-Benutzers. |
| Eine Datenbankprüfung ausgefallen mit `reason="query"` | Verbindung und Anmeldung funktionierten. Die Datenbank läuft und weigert sich zu arbeiten: Read-only, fehlende Tabelle, Recovery. |
| MQTT ausgefallen mit `reason="payload"` auf einem Topic, das offensichtlich Verkehr hat | Nur *Retained* Messages erreichen einen Client, der gerade abonniert hat. Veröffentlichen Sie das Topic mit dem Retain-Flag, oder haken Sie „Retained Message erwarten“ ab. |
| WebSocket ausgefallen mit `reason="protocol"` nach einem `101` | Etwas auf dem Pfad beantwortet das Upgrade, ohne WebSocket zu sprechen — meist eine Reverse-Proxy-Regel, die den Header `Upgrade` verschluckt. |
