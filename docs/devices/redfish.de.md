# Server-Hardware (Redfish)

Die eigene Hardware eines Servers, von seinem Management-Controller
(BMC) gelesen: Lüfter, Temperaturen gegen ihre eigenen Schwellenwerte,
Netzteile und ihre Redundanz, Laufwerke, Speicher, Prozessoren und das
Event-Log.

Redfish ist die von der DMTF standardisierte HTTPS-Schnittstelle für
Management-Controller, und jeder aktuelle spricht es: Supermicro, Dell
iDRAC, HPE iLO, Lenovo XClarity Controller, ASRock Rack, OpenBMC. Es
ist der richtige Weg, Server-Hardware zu beobachten. Der SNMP-Agent
eines BMC beschreibt dagegen meist den BMC selbst — sein kleines
ARM-Linux, seinen Speicher, seine Netzwerkverbindungen — nicht den von
ihm verwalteten Server (siehe
[SNMP](snmp.md#server-management-controllers-bmc)).

!!! warning "Nur gegen die DMTF-Referenz-Mockups getestet"

    Diese Integration wurde gegen die offiziellen DMTF-Redfish-Mockups
    gebaut und verifiziert (Rack-Server, Tower-Server mit lokalem
    Storage, flüssigkeitsgekühlter Server mit den neueren Thermal- und
    Power-Schemas), bedient vom DMTF-Mockup-Server. Jeder Aufruf ist
    ein Lesevorgang, und die Ressourcenformen sind die Standard-Formen,
    aber es lief noch keine Wochen gegen einen echten Supermicro,
    iDRAC oder iLO. Sagen Sie uns, was kaputt ist.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_redfish_`. Health-Zeitreihen
sind **0 (OK), 1 (Warning) oder 2 (Critical)**, direkt aus
`Status.Health`.

| Familie | Metriken | Labels |
|---|---|---|
| Service | `info` (Wert 1) | `vendor`, `product`, `redfish_version` |
| System | `system_health` (die eigene des Systems), `system_health_rollup` (alles, was es enthält), `system_power_on`, `memory_health`, `processor_health` (die Zusammenfassungen des Controllers, ein Wert für alle DIMMs und alle CPUs), `memory_total_bytes` | `system` |
| Chassis | `chassis_health`, `chassis_health_rollup` | `chassis` |
| Temperaturen | `temperature_celsius`, `temperature_upper_critical_celsius` und `temperature_upper_caution_celsius` (die eigenen kritischen und Vorsicht-Schwellenwerte des Sensors, wenn er sie angibt), `temperature_health` | `chassis`, `sensor` |
| Lüfter | `fan_speed_rpm` (oder `fan_speed_percent`, wenn der Controller einen Prozentsatz meldet), `fan_lower_critical_rpm` (oder `_percent`: das eigene angegebene Minimum des Lüfters, nur älteres Schema), `fan_health`, `fan_redundancy_health` | `chassis`, `fan` / `group` |
| Power | `psu_health`, `psu_output_watts`, `psu_capacity_watts`, `power_redundancy_health`, `power_consumed_watts` | `chassis`, `psu` / `group` |
| Spannungen | `voltage_volts`, `voltage_health` | `chassis`, `sensor` |
| Storage | `storage_health`, `drive_health`, `drive_failure_predicted`, `drive_life_left_percent` (SSDs), `drive_capacity_bytes` | `system`, `storage`, `drive`, `media` |
| Controller | `manager_health`, `manager_info` (Wert 1) | `manager`, `firmware`, `model` |
| Logs | `log_entries`, `log_critical_entries`, `log_warning_entries` | `owner` (System oder Controller), `log` |
| Sammlung | `scrape_errors`, `scrape_duration_seconds` | |

**Ein leerer Steckplatz ist kein Ausfall.** Eine Komponente, deren
`Status.State` `Absent` ist (ein leerer Netzteil-Schacht, ein freier
DIMM-Steckplatz, eine fehlende zweite CPU) oder `Disabled` (eine nicht
konfigurierte Redundanzgruppe) erzeugt überhaupt keine Zeitreihe. Ein
mit einem von zwei Netzteilen ausgelieferter Server ist gesund und
liest sich so.

**Zwei Generationen des Schemas, ein Satz von Zeitreihen.** Die
meisten aktiven Controller veröffentlichen noch die älteren
`Chassis/{id}/Thermal`- und `Chassis/{id}/Power`-Ressourcen, je ein
Request pro Sensor; die neuesten veröffentlichen nur
`ThermalSubsystem`, `PowerSubsystem`, `EnvironmentMetrics` und
`Sensors`, eine Ressource pro Lüfter, Netzteil und Sensor. DumbMonit
liest die ältere Form, wenn sie existiert, und sonst die neuere;
beide erzeugen genau dieselben Zeitreihen und Labels, daher
funktioniert eine einmal geschriebene Regel auf beiden.

**Die Logs werden gezählt, nie gelesen.** Log-Einträge können Hosts,
Adressen und Benutzer benennen: DumbMonit behält nur, wie viele
Einträge es gibt, und wie viele der ersten vom Controller bedienten
Seite Critical oder Warning sind.

Die geltenden [eingebauten Regeln](../alerting/rules.md#server-hardware-redfish):
Server fan failed, Server temperature above critical, Power supply
redundancy lost, Power supply failed, Drive failure predicted, Server
health critical, plus Device unreachable.

## Die Geräteseite

Ein Redfish-Gerät erhält auf seiner Seite ein eigenes Panel, über den
Diagrammen, aus dem gelesen, was die letzte Abfrage gespeichert hat:
Das Öffnen der Seite fragt den Controller nie ab. Es antwortet, in
dieser Reihenfolge:

1. **Ist der Server gesund.** Ein Wort — Healthy, Degraded oder
   Critical — aus der eigenen Gesundheit des Controllers und den
   Roll-ups, und darunter jede es herunterziehende Komponente, in
   einem Satz: *CPU1 Temp is at 47 °C, at or above its critical
   threshold of 45 °C*, *Power supply redundancy is lost*, *Drive 3
   predicts its own failure*. Wenn der Controller den Server als
   degraded hochrollt, ohne etwas zu benennen, das DumbMonit liest,
   sagt das Panel das und verweist auf sein Event-Log. Darunter:
   Power-Zustand, Leistungsaufnahme, installierter Speicher und die
   Firmware des Controllers.
2. **Sind Lüfter und Temperaturen innerhalb ihrer Grenzen.** Jeder
   Sensor wird mit den vom Controller **für diesen Sensor**
   angegebenen Schwellenwerten verglichen — seinen Vorsicht- und
   kritischen Temperaturen, der Mindestdrehzahl eines Lüfters — nie
   mit einer Zahl von DumbMonit. Ein Sensor, der keinen Schwellenwert
   angibt, zeigt seinen Messwert und kein Urteil, sofern der
   Controller ihn nicht selbst als degraded meldet. Probleme werden
   zuerst gelistet.
3. **Ist die Stromversorgung redundant.** Jede Redundanzgruppe
   (Redundant, Redundancy degraded, Redundancy lost), dann jedes
   vorhandene Netzteil mit seiner Ausgabe gegen seine Kapazität und
   seinen Zustand. Ohne Redundanzgruppe sagt das Panel, dass der
   Controller keine angibt: ein einzelnes Netzteil, oder Redundanz
   nicht konfiguriert.
4. **Sind die Laufwerke gesund.** Jeder Storage-Controller, dann
   jedes Laufwerk mit seiner Kapazität, der verbleibenden Lebensdauer
   einer SSD und seinem Zustand; *Failure predicted* kommt vor allem
   anderen.
5. **Speicher, Prozessoren und Controller**: die
   Ein-Wort-Zusammenfassungen des Controllers für alle DIMMs und alle
   CPUs, die eigene Gesundheit, das Modell und die Firmware des
   Management-Controllers, und die Spannungen.
6. **Event-Logs**: Einträge nach Schweregrad gezählt. Einträge
   bleiben, bis jemand das Log löscht, daher kann eine kritische
   Zählung alte Nachricht sein: Sie wird angezeigt, ändert aber nicht
   das Gesundheitswort des Servers.

Leere Schächte und freie Steckplätze erscheinen nie: Sie erzeugen
keine Zeitreihe. Jeder Zustand ist ein Wort sowie eine Farbe, und ein
unbekannter Wert liest sich "—", nie 0.

## Ein nur-lesendes Konto auf dem Management-Controller erstellen

1. Öffnen Sie die Web-Oberfläche des Management-Controllers (BMC),
   nicht das Betriebssystem des Servers. Erstellen Sie einen neuen,
   wie folgt benannten lokalen Benutzer, mit einem nirgendwo sonst
   genutzten langen Passwort.

    ```
    dumbmonit
    ```

2. Geben Sie ihm die niedrigste lesefähige Rolle, und nicht mehr.
   Supermicro: Configuration → Users → Add User, Privilege User, und
   Redfish im Account-Typ ankreuzen. Dell iDRAC: iDRAC Settings →
   Users → Local Users → Add, Rolle Read Only. HPE iLO:
   Administration → User Administration → New, jede Berechtigung
   außer Login entfernen. Lenovo XClarity Controller: BMC
   Configuration → User/LDAP → Create, Rolle Read-only. ASRock Rack:
   Settings → User Management, Privilege User.

3. Prüfen Sie von jeder Maschine im Management-Netzwerk, dass das
   Konto Redfish lesen kann. Der Befehl fragt nach dem Passwort und
   gibt Name und Gesundheit des Servers aus.

    ```
    curl -k -u dumbmonit https://bmc.lan/redfish/v1/Systems
    ```

4. Geben Sie in DumbMonit die Controller-Adresse ein, zum Beispiel
   "bmc.lan" oder "10.0.0.50", dann Benutzername und Passwort dieses
   Kontos.

5. DumbMonit liest nur: Lüfter, Temperaturen, Spannungen, Netzteile,
   Laufwerke, Speicher- und Prozessor-Zusammenfassungen, und wie viele
   Log-Einträge es nach Schweregrad gibt. Es schaltet den Server nie
   ein oder aus, ändert nie eine Einstellung und liest nie den Text
   der Logs.

!!! warning
    Nutzen Sie nicht das Werkskonto des Controllers wiederverwendet:
    Es kann den Server ausschalten, Medien einhängen und die Firmware
    neu flashen. Management-Controller liefern auch ein
    selbstsigniertes Zertifikat: Wird die Verbindung deshalb
    abgelehnt, kreuzen Sie "Accept an unverifiable certificate" in den
    Optionen an. Ein Controller antwortet langsam, und ein
    vollständiges Lesen braucht mehrere Requests: Meldet das Gerät
    Timeouts, heben Sie DUMBMONIT_PROBE_TIMEOUT_SECS auf dem Server
    an.

Die obigen Menünamen sind die aktueller Firmware; ältere Generationen
verschieben sie, behalten aber dieselben Rollen. Auf Supermicro
entscheidet der beim Erstellen eines Benutzers gezeigte Account-Typ,
welche Oberflächen er nutzen darf: Ein Konto ohne Redfish-Zugriff wird
mit einem 401 abgelehnt, selbst mit dem richtigen Passwort.

## Credentials

| Credential | Felder |
|---|---|
| Management-Controller-Konto | Benutzername und Passwort des nur-lesenden Kontos. |

Adresse: der Hostname oder die IP des Controllers, zum Beispiel
`bmc.lan` oder `10.0.0.50`. Der Standardport ist 443; schreiben Sie
`host:port` für einen anderen, oder eine vollständige URL
(`https://bmc.lan:8443`).

## Optionen

| Option | Standard | Wirkung |
|---|---|---|
| `port` | `443` | HTTPS-Port, falls die Adresse keinen angibt. |
| `insecure_tls` | `false` | Ein selbstsigniertes Zertifikat akzeptieren. |
| `request_timeout_seconds` | `8` | Erlaubte Zeit für jeden Redfish-Aufruf, von 1 bis 120. |
| `auth` | `basic` | `basic` sendet Benutzername und Passwort mit jedem Request und lässt auf dem Controller nichts offen. `session` loggt sich einmal ein und nutzt das Session-Token (`X-Auth-Token`) wieder, öffnet es nur neu, wenn der Controller es abweist, und schließt das ersetzte. Nur nutzen, wenn der Controller Basic-Authentifizierung abweist: Controller haben nur eine Handvoll Session-Slots. |
| `storage` | `true` | Die Storage-Controller und jedes Laufwerk dahinter lesen. Ein Request pro Laufwerk. |
| `logs` | `true` | Die Einträge des Systemereignisprotokolls und des eigenen Logs des Controllers zählen, nach Schweregrad. |

Ein vollständiges Lesen sind ein paar Dutzend Requests: die Service-
Root, dann jedes System, Chassis und Controller, ihre Thermal- und
Power-Ressourcen, die Storage-Controller und Laufwerke, die
Log-Dienste. DumbMonit sendet höchstens vier gleichzeitig. Ein
langsamer Controller kann ein längeres Abfrage-Timeout brauchen
(`DUMBMONIT_PROBE_TIMEOUT_SECS`, siehe
[Konfiguration](../reference/configuration.md)) und ein
Abfrageintervall von zwei Minuten oder mehr.

## Häufige Fehler

| Symptom | Wahrscheinliche Ursache |
|---|---|
| *Authentication refused* | Falsches Passwort, oder das Konto darf Redfish nicht nutzen (Supermicro-Account-Typ, fehlende iLO-Berechtigung "Login"). |
| *Insufficient privileges* | Die Rolle ist für eine Ressource zu niedrig; der Rest des Lesevorgangs geht trotzdem durch. Manche Controller verbergen das Event-Log vor ihrer niedrigsten Rolle. |
| *TLS certificate rejected* | Das selbstsignierte Zertifikat des Controllers: Kreuzen Sie "Accept an unverifiable certificate" an, oder installieren Sie ein vertrauenswürdiges Zertifikat auf dem Controller. |
| *Timed out* | Der Controller antwortet langsam: Heben Sie `DUMBMONIT_PROBE_TIMEOUT_SECS` an, schalten Sie `storage` oder `logs` aus, oder verlängern Sie das Intervall. |
| Keine Temperaturschwellenwerte | Der Controller veröffentlicht `UpperThresholdCritical` für diesen Sensor nicht: Die Temperatur wird trotzdem aufgezeichnet, aber die eingebaute Regel kann sie mit nichts vergleichen. |
