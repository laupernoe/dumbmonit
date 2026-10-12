# CrowdSec

Die Security-Engine: aktive Entscheidungen und ihre Herkunft, Alarme,
Bouncer, die aufgehört haben abzufragen, gelesene Log-Zeilen und solche,
die kein Parser erkannt hat, und ob die Local API antwortet.

DumbMonit liest CrowdSecs Prometheus-Metriken (Port 6060) und prüft
`/health` der Local API (Port 8080). Mit einem optionalen Bouncer-Key
fragt es die Local API zusätzlich nach den Entscheidungen zu 192.0.2.1,
einer Dokumentationsadresse: Die Antwort ist immer leer und beweist, dass
Authentifizierung und Datenbank antworten. Es erstellt oder löscht nie
eine Entscheidung und lädt die Bannliste nie herunter.

Der beobachtenswerte Fehler ist ein Bouncer, der aufgehört hat
abzufragen. Die Firewall oder der Reverse Proxy läuft mit der Liste
weiter, die sie hatte, neue Angreifer kommen durch, und nichts beschwert
sich. CrowdSec veröffentlicht nicht, wann ein Bouncer zuletzt abgefragt
hat; DumbMonit liest es aus dem Request-Zähler des Bouncers, der aufhört
sich zu bewegen. Der zweite ist eine defekte Acquisition (eine
umbenannte Log-Datei, ein neu erstellter Container): CrowdSec liest
nichts, sieht keinen Angriff und bleibt grün.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_crowdsec_`. Totals sind Zähler:
Diagramme und Regeln verwandeln sie in Raten.

| Metrik | Was | Labels |
|---|---|---|
| `version_info` | Wert 1 | `version` |
| `lapi_up` | 1, wenn die Local API `/health` beantwortet | |
| `lapi_decisions_ok` | mit einem Bouncer-Key: 1, wenn die Local API die Entscheidungsabfrage beantwortet hat | |
| `decisions` | aktive Entscheidungen, Community-Blocklisten eingeschlossen | |
| `decisions_by_origin` | | `origin`: `crowdsec`, `cscli`, `CAPI`, `lists`… |
| `alerts` | von der Local API vorgehaltene Alarme, Community-Alarme ausgeschlossen | |
| `bouncers_seen` | Bouncer, die die Local API seit dem Start von CrowdSec aufgerufen haben | |
| `bouncer_requests_total` | Requests jedes Bouncers; bewegt er sich nicht mehr, hat der Bouncer aufgehört abzufragen | `bouncer` |
| `machine_requests_total` | Requests jedes Agents (Maschine) | `machine` |
| `lapi_requests_total` | alle Requests an die Local API | |
| `scenario_overflows_total` | vom lokalen Agent ausgelöste Szenarien: jedes ist ein Alarm | |
| `lines_read_total`, `lines_parsed_total`, `lines_unparsed_total` | vom Agent gelesene Log-Zeilen, und die geparsten oder nicht | |
| `source_lines_total`, `source_lines_unparsed_total` | dasselbe pro Log-Quelle | `source` |
| `buckets` | derzeit offene Szenario-Buckets | |

Die Local-API-Familien erscheinen, wo die Local API läuft, die
Log-Familien, wo ein Agent Logs liest; das offizielle Image und eine
Standardinstallation laufen beide.

Die geltenden [eingebauten Regeln](../alerting/rules.md#databases-message-broker-and-security-engine):
CrowdSec Local API down, CrowdSec bouncer stopped pulling, CrowdSec
reads no logs, plus Device unreachable.

## Die Geräteseite

Das Panel über den Diagrammen liest, was die Abfrage gespeichert hat; das
Öffnen der Seite fragt CrowdSec nie ab. Es sagt in einem Satz, was nicht
stimmt (Local API, Bouncer, Log-Lesen und der Anteil nicht erkannter
Zeilen), dann die aktiven Entscheidungen, vorgehaltenen Alarme, in den
letzten 24 Stunden ausgelösten Szenarien und gelesenen Zeilen pro
Sekunde, dann die Zeit seit der letzten Abfrage jedes Bouncers, die
Entscheidungen nach Herkunft und den nicht geparsten Anteil jeder
Log-Quelle.

## DumbMonit CrowdSecs Metriken lesen lassen

1. CrowdSec veröffentlicht seine Metriken standardmäßig auf
   127.0.0.1:6060. Setzen Sie listen_addr im prometheus-Abschnitt von
   /etc/crowdsec/config.yaml auf eine für DumbMonit erreichbare Adresse,
   starten Sie CrowdSec dann neu. Das offizielle Docker-Image hört
   bereits auf jeder Schnittstelle.

    ```
    sudo sed -i 's/listen_addr: 127.0.0.1/listen_addr: 0.0.0.0/' /etc/crowdsec/config.yaml
    sudo systemctl restart crowdsec
    ```

2. Die Metriken brauchen kein Credential: Sie halten Zahlen, Bouncer- und
   Maschinennamen und Log-Dateipfade, nie eine IP-Adresse. Filtern Sie
   Port 6060, sodass nur der DumbMonit-Host ihn erreicht, und prüfen Sie,
   dass er antwortet.

    ```
    curl -s http://crowdsec.lan:6060/metrics | grep cs_info
    ```

3. DumbMonit prüft außerdem /health der Local API, auf Port 8080
   desselben Hosts: listen_uri im api.server-Abschnitt muss dann ebenfalls
   erreichbar sein. Entfernen Sie das Häkchen bei "Check the Local API"
   für einen Host, auf dem nur ein Agent läuft.

4. Optional: Erstellen Sie einen Bouncer-Key für DumbMonit und fügen Sie
   ihn unten als Bouncer-API-Key ein. Ein Bouncer-Key kann nur
   Entscheidungen lesen. DumbMonit fragt nach den Entscheidungen zu
   192.0.2.1, einer Dokumentationsadresse, daher ist die Antwort immer
   leer und die Bannliste wird nie herunterladen.

    ```
    sudo cscli bouncers add dumbmonit
    ```

5. Geben Sie in DumbMonit den CrowdSec-Host ein, zum Beispiel
   "crowdsec.lan". Laufen Local API und Agents auf verschiedenen Hosts,
   fügen Sie jeden Host hinzu: Entscheidungen, Alarme und Bouncer werden
   gezählt, wo die Local API läuft, Log-Zeilen, wo jeder Agent sie liest.

!!! warning
    Ein Bouncer, der aufhört abzufragen, setzt eine eingefrorene Liste
    weiter durch: neue Angreifer kommen durch, und nichts beschwert
    sich. DumbMonit warnt, wenn die Requests eines Bouncers für 30
    Minuten stoppen. Es kennt nur Bouncer, die seit dem letzten Start von
    CrowdSec mindestens einmal abgefragt haben.

Ein mit `cscli bouncers delete` gelöschter Bouncer behält seinen Zähler
bis zum Neustart von CrowdSec und wird bis dahin als gestoppt gemeldet.
Ein Bouncer, der die Local API nur aufruft, wenn er einen Request bedient
(ein Reverse-Proxy-Bouncer im Live-Modus), kann in einer ruhigen Nacht
eine halbe Stunde still sein; wechseln Sie auf Stream-Modus, oder erhöhen
Sie das Fenster der Regel.

## Credentials

| Credential | Felder |
|---|---|
| Nur Metriken | Nichts: liest `/metrics`, und `/health` der Local API. |
| Bouncer API key | Der von `cscli bouncers add dumbmonit` ausgegebene Key, als `X-Api-Key` an die Local API gesendet. |

Adresse: ein Hostname oder eine IP (`crowdsec.lan`), `host:port` für die
Metriken, oder eine vollständige URL. Optionen: Protokoll (standardmäßig
HTTP), Metrik-Port (6060), Check the Local API (ein), Local-API-Port
(8080), Zertifikatsprüfung und Anfrage-Timeout (10 s). Veraltete oder als
tainted markierte Hub-Elemente (Parser, Szenarien, Collections) sind über
die Metriken nicht sichtbar, nur mit `cscli hub list` auf dem Host.
