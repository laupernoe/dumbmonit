# USV mit NUT {#ups-with-nut}

Eine per USB an ein NAS, einen Raspberry Pi oder einen Server angeschlossene USV wird meist von
Network UPS Tools (NUT) überwacht: Dessen Daemon `upsd` veröffentlicht auf TCP-Port 3493, was
die USV meldet. DumbMonit liest diesen Server wie jeder NUT-Client: ob die USV am Netz oder
an der Batterie läuft, ob die Batterie schwach oder verschlissen ist, die Ladung, die
verbleibende Laufzeit, die Last, die Eingangsspannung und das Alter der Batterie. Ein Server
veröffentlicht oft mehrere USV; alle werden in einer Verbindung gelesen, und jede Serie trägt
den Namen ihrer USV.

Für eine USV mit eigener Netzwerkkarte liest der [SNMP-Gerätetyp](snmp.md) stattdessen die
Standard-UPS-MIB.

## Was überwacht wird {#what-it-watches}

Alle Metriken haben das Präfix `dumbmonit_nut_` und tragen das Label `ups`, den USV-Namen in
`ups.conf` (`ups` auf einem Synology-NAS, `qnapups` auf einem QNAP). Eine Variable, die die USV
nicht veröffentlicht, erzeugt keine Serie.

| Metrik | Was | Labels |
|---|---|---|
| `ups_on_line`, `ups_on_battery`, `ups_low_battery`, `ups_high_battery`, `ups_replace_battery`, `ups_charging`, `ups_discharging`, `ups_bypass`, `ups_calibrating`, `ups_off`, `ups_overload`, `ups_trim`, `ups_boost`, `ups_forced_shutdown`, `ups_alarm` | Die Flags von `ups.status` (`OL`, `OB`, `LB`, `HB`, `RB`, `CHRG`, `DISCHRG`, `BYPASS`, `CAL`, `OFF`, `OVER`, `TRIM`, `BOOST`, `FSD`, `ALARM`): 1 wenn gesetzt, sonst 0, bei jedem Lesen alle | `ups` |
| `ups_status_info` | Wert 1, das rohe `ups.status` | `ups`, `status` |
| `battery_charge_percent`, `battery_charge_low_percent` | Ladung und die Ladung, unter der die USV eine schwache Batterie meldet | `ups` |
| `battery_runtime_seconds`, `battery_runtime_low_seconds` | verbleibende Laufzeit an der Batterie bei der aktuellen Last und die Laufzeit bei schwacher Batterie | `ups` |
| `battery_voltage_volts`, `battery_temperature_celsius` | | `ups` |
| `battery_age_seconds` | Zeit seit `battery.date` (installiert oder ersetzt), sonst `battery.mfr.date` | `ups`, `source` (`installed`, `manufactured`) |
| `input_voltage_volts`, `input_voltage_nominal_volts`, `input_frequency_hertz` | Netzseite; 0 V bei einem Ausfall | `ups` |
| `output_voltage_volts`, `output_frequency_hertz` | Lastseite | `ups` |
| `ups_load_percent` | Last in Prozent der USV-Kapazität | `ups` |
| `ups_realpower_watts`, `ups_realpower_nominal_watts`, `ups_power_va`, `ups_power_nominal_va` | aufgenommene Leistung und Nennleistung, wenn die USV sie meldet | `ups` |
| `ups_temperature_celsius` | | `ups` |
| `ups_self_test_failed` | 1, wenn der letzte Selbsttest (`ups.test.result`) einen Fehler oder eine Warnung meldet, 0, wenn er bestanden wurde; fehlt, wenn kein Test lief | `ups`, `result` |
| `ups_info` | Wert 1 | `ups`, `manufacturer`, `model`, `firmware`, `driver`, `battery_type`, `description` |
| `ups_data_stale` | 1, wenn `upsd` antwortet, aber keine frischen Daten von der USV hat (`DATA-STALE`, `DRIVER-NOT-CONNECTED`) | `ups` |
| `ups_driver_connected` | 0, wenn der NUT-Treiber dieser USV nicht läuft | `ups` |
| `server_ups` | Anzahl der vom Server veröffentlichten USV | |
| `server_info` | Wert 1 | `version` |

Seriennummern kommen nie in ein Label. Pro Server werden höchstens 32 USV gelesen.

Die zutreffenden [integrierten Regeln](../alerting/rules.md#ups-behind-nut): USV an der
Batterie, USV-Batterie schwach, USV-Batterie muss ersetzt werden, USV-Last hoch, USV-Laufzeit
kurz, USV-Daten veraltet, außerdem Monitor nicht erreichbar, wenn der NUT-Server selbst nicht
mehr antwortet.

## Die Monitorseite {#the-device-page}

Das Panel über den Diagrammen liest, was die Prüfung gespeichert hat; das Öffnen der Seite
verbindet sich nie mit dem NUT-Server. Eine Karte pro USV: ihr Zustand in Worten (am Netz, an
der Batterie, Batterie schwach, Batterie ersetzen, Bypass, Überlast …), die Ladung, die
verbleibende Laufzeit, die Last, die Eingangsspannung und das Alter der Batterie.

## DumbMonit den NUT-Server lesen lassen {#let-dumbmonit-read-the-nut-server}

1. Finden Sie den NUT-Server: Der Rechner, an den die USV angeschlossen ist (ein NAS, ein Raspberry Pi, ein Server), führt upsd auf TCP-Port 3493 aus. Prüfen Sie vom DumbMonit-Host aus, dass er antwortet, und notieren Sie die Namen seiner USV.

    ```
    printf 'LIST UPS\nLOGOUT\n' | nc nas.lan 3493
    ```

    Mit installierten NUT-Client-Tools liefert `upsc -l nas.lan` dieselbe Liste.

2. Bei einer einfachen NUT-Installation lassen Sie upsd auf einer Adresse lauschen, die der DumbMonit-Host erreichen kann: Fügen Sie diese Zeile in upsd.conf ein, öffnen Sie TCP-Port 3493 in der Firewall für den DumbMonit-Host und starten Sie upsd neu. Zum Lesen ist kein Konto nötig: upsd lässt jeden akzeptierten Host die Variablen lesen und ändert nichts ohne einen Benutzer mit dem Recht dazu.

    ```
    LISTEN 0.0.0.0 3493
    ```

3. Bei einem NAS heißt derselbe Schalter anders. Synology DSM: Systemsteuerung → Hardware & Energie → USV, haken Sie Netzwerk-USV-Server aktivieren an und fügen Sie den DumbMonit-Host unter Zugelassene DiskStation-Geräte hinzu; die USV heißt ups. QNAP: die USV-Seite der Systemsteuerung, Netzwerk-USV-Master aktivieren, mit dem DumbMonit-Host unter den erlaubten Adressen; die USV heißt qnapups. TrueNAS: der USV-Dienst mit angehaktem Remote Monitor.

4. Optional: DumbMonit sendet Benutzername und Passwort nur, wenn Sie welche eingeben, und upsd verlangt zum Lesen keine. Wenn Ihre Richtlinie jeden Client benannt sehen will, fügen Sie in upsd.users einen Benutzer ohne actions oder instcmds mit einem langen zufälligen Passwort hinzu und starten Sie upsd neu.

    ```
    [dumbmonit]
    password = a-long-random-password
    ```

5. Geben Sie in DumbMonit die Adresse des NUT-Servers ein, zum Beispiel „nas.lan“. Jede von ihm veröffentlichte USV wird überwacht; um nur einige zu überwachen, tragen Sie deren Namen in die Option USV ein.

6. DumbMonit liest nur: Es sendet LIST UPS und LIST VAR, nie LOGIN, SET, INSTCMD oder FSD, sodass der Server es nie als Secondary zählt, auf den er vor dem Herunterfahren warten müsste.

!!! warning
    DumbMonit überwacht die USV; es fährt nichts herunter. Die von der USV versorgten Rechner brauchen weiterhin einen eigenen NUT-Client (upsmon), um bei schwacher Batterie sauber herunterzufahren.

## Zugangsdaten {#credentials}

| Zugangsdaten | Felder |
|---|---|
| Keine Authentifizierung | Der Standard: `upsd` lässt jeden akzeptierten Host die USV-Variablen lesen. |
| upsd-Benutzername / Passwort | Wird vor dem Lesen mit `USERNAME` und `PASSWORD` gesendet. `upsd` prüft sie nur bei Befehlen, die DumbMonit nie sendet. |

Adresse: ein Hostname oder eine IP (`nas.lan`), `host:port` oder die Form `ups@host` von `upsc`,
von der nur der Host behalten wird. Optionen: Port (3493), die zu überwachende USV (standardmäßig
alle) und das Timeout pro Austausch (10 s).

Eine abgelehnte Verbindung oder keine Antwort gilt als Monitor nicht erreichbar. Fährt der Rechner,
auf dem der NUT-Server läuft, bei dem von ihm gemeldeten Ausfall herunter (er ist oft sein
eigener upsmon-Primary), werden die USV-Alarme als unterdrückt ausgeblendet, während Monitor nicht
erreichbar für denselben Monitor auslöst: Lesen Sie den Alarm Monitor nicht erreichbar als Zeichen,
dass der Stromausfall gewonnen hat. Betreiben Sie den NUT-Server auf einem Rechner, der die
anderen überlebt, oder überwachen Sie die USV von einem zweiten NUT-Server aus. `ACCESS-DENIED`
wird als Zugangsdatenfehler angezeigt und ein vom Server nicht veröffentlichter USV-Name als
Konfigurationsfehler; keiner von beiden weckt nachts jemanden. Die Verbindung ist reines TCP:
`STARTTLS` wird nicht verwendet, und nichts Gelesenes ist geheim.
