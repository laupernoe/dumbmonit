# USV mit NUT

Eine USV, die über USB an einem NAS, einem Raspberry Pi oder einem
Server angeschlossen ist, wird meist von Network UPS Tools (NUT)
beobachtet: dessen Daemon `upsd` veröffentlicht, was die USV sagt, auf
TCP-Port 3493. DumbMonit liest diesen Server, wie jeder NUT-Client: ob
die USV am Netz oder im Akkubetrieb läuft, ob der Akku schwach oder
abgenutzt ist, den Ladezustand, die verbleibende Laufzeit, die Last,
die Eingangsspannung und das Alter des Akkus. Ein Server veröffentlicht
oft mehrere USVs; alle werden in einer Verbindung gelesen, und jede
Zeitreihe trägt den Namen ihrer USV.

Für eine USV mit eigener Netzwerkkarte liest die Art
[SNMP-Gerät](snmp.md) stattdessen die Standard-UPS-MIB.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_nut_` und tragen das Label
`ups`, den USV-Namen in `ups.conf` (`ups` auf einem Synology-NAS,
`qnapups` auf einem QNAP). Eine von der USV nicht veröffentlichte
Variable erzeugt keine Zeitreihe.

| Metrik | Was | Labels |
|---|---|---|
| `ups_on_line`, `ups_on_battery`, `ups_low_battery`, `ups_high_battery`, `ups_replace_battery`, `ups_charging`, `ups_discharging`, `ups_bypass`, `ups_calibrating`, `ups_off`, `ups_overload`, `ups_trim`, `ups_boost`, `ups_forced_shutdown`, `ups_alarm` | Die Flags von `ups.status` (`OL`, `OB`, `LB`, `HB`, `RB`, `CHRG`, `DISCHRG`, `BYPASS`, `CAL`, `OFF`, `OVER`, `TRIM`, `BOOST`, `FSD`, `ALARM`): 1, wenn gesetzt, sonst 0, alle bei jeder Lesung | `ups` |
| `ups_status_info` | Wert 1, das rohe `ups.status` | `ups`, `status` |
| `battery_charge_percent`, `battery_charge_low_percent` | Ladezustand, und der Ladezustand, unter dem die USV einen schwachen Akku meldet | `ups` |
| `battery_runtime_seconds`, `battery_runtime_low_seconds` | verbleibende Laufzeit im Akkubetrieb bei der aktuellen Last, und die Low-Battery-Laufzeit | `ups` |
| `battery_voltage_volts`, `battery_temperature_celsius` | | `ups` |
| `battery_age_seconds` | Zeit seit `battery.date` (installiert oder ersetzt), sonst `battery.mfr.date` | `ups`, `source` (`installed`, `manufactured`) |
| `input_voltage_volts`, `input_voltage_nominal_volts`, `input_frequency_hertz` | Netzseite; 0 V während eines Ausfalls | `ups` |
| `output_voltage_volts`, `output_frequency_hertz` | Lastseite | `ups` |
| `ups_load_percent` | Last, in Prozent der USV-Kapazität | `ups` |
| `ups_realpower_watts`, `ups_realpower_nominal_watts`, `ups_power_va`, `ups_power_nominal_va` | aufgenommene Leistung und Nennwert, wenn die USV sie meldet | `ups` |
| `ups_temperature_celsius` | | `ups` |
| `ups_self_test_failed` | 1, wenn der letzte Selbsttest (`ups.test.result`) einen Fehler oder eine Warnung meldet, 0, wenn er bestanden hat; fehlt, wenn kein Test lief | `ups`, `result` |
| `ups_info` | Wert 1 | `ups`, `manufacturer`, `model`, `firmware`, `driver`, `battery_type`, `description` |
| `ups_data_stale` | 1, wenn `upsd` antwortet, aber keine frischen Daten von der USV hat (`DATA-STALE`, `DRIVER-NOT-CONNECTED`) | `ups` |
| `ups_driver_connected` | 0, wenn der NUT-Treiber dieser USV nicht läuft | `ups` |
| `server_ups` | Anzahl der vom Server veröffentlichten USVs | |
| `server_info` | Wert 1 | `version` |

Seriennummern werden nie in ein Label gelegt. Höchstens 32 USVs werden
pro Server gelesen.

Die geltenden [eingebauten Regeln](../alerting/rules.md#ups-behind-nut):
UPS on battery, UPS battery low, UPS battery needs replacing, UPS load
high, UPS runtime short, UPS data stale, plus Device unreachable, wenn
der NUT-Server selbst aufhört zu antworten.

## Die Geräteseite

Das Panel über den Diagrammen liest, was die Abfrage gespeichert hat; das
Öffnen der Seite verbindet sich nie mit dem NUT-Server. Eine Karte pro
USV: ihr Zustand in Worten (am Netz, im Akkubetrieb, schwacher Akku,
Akku ersetzen, Bypass, Überlast…), der Ladezustand, die verbleibende
Laufzeit, die Last, die Eingangsspannung und das Alter des Akkus.

## DumbMonit den NUT-Server lesen lassen

1. Finden Sie den NUT-Server: Die Maschine, an die die USV angeschlossen
   ist (ein NAS, ein Raspberry Pi, ein Server), führt upsd auf
   TCP-Port 3493 aus. Prüfen Sie vom DumbMonit-Host aus, dass er
   antwortet, und merken Sie sich die Namen seiner USVs.

    ```
    printf 'LIST UPS\nLOGOUT\n' | nc nas.lan 3493
    ```

    Mit installierten NUT-Client-Tools gibt `upsc -l nas.lan` dieselbe
    Liste.

2. Lassen Sie auf einer gewöhnlichen NUT-Installation upsd auf einer
   für den DumbMonit-Host erreichbaren Adresse hören: Fügen Sie diese
   Zeile zu upsd.conf hinzu, öffnen Sie TCP-Port 3493 in der Firewall
   für den DumbMonit-Host, und starten Sie upsd neu. Das Lesen braucht
   kein Konto: upsd lässt jeden Host, den es akzeptiert, die Variablen
   lesen, und ändert nichts ohne einen Benutzer mit dem Recht dazu.

    ```
    LISTEN 0.0.0.0 3493
    ```

3. Auf einem NAS hat derselbe Schalter einen anderen Namen. Synology
   DSM: Control Panel → Hardware & Power → UPS, Enable network UPS
   server ankreuzen, und den DumbMonit-Host unter Permitted DiskStation
   Devices hinzufügen; die USV heißt ups. QNAP: die UPS-Seite des
   Control Panel, Enable network UPS master, mit dem DumbMonit-Host
   unter den erlaubten Adressen; die USV heißt qnapups. TrueNAS: der
   UPS-Dienst mit angekreuztem Remote Monitor.

4. Optional: DumbMonit sendet einen Benutzernamen und ein Passwort nur,
   wenn Sie eines eingeben, und upsd verlangt keines zum Lesen. Wenn
   Ihre Richtlinie will, dass jeder Client benannt ist, fügen Sie
   upsd.users einen Benutzer ohne Actions oder Instcmds mit einem
   langen Zufallspasswort hinzu und starten Sie upsd neu.

    ```
    [dumbmonit]
    password = a-long-random-password
    ```

5. Geben Sie in DumbMonit die Adresse des NUT-Servers ein, zum Beispiel
   "nas.lan". Jede von ihm veröffentlichte USV wird beobachtet; um nur
   manche zu beobachten, listen Sie ihre Namen in der UPS-Option.

6. DumbMonit liest nur: Es sendet LIST UPS und LIST VAR, nie LOGIN, SET,
   INSTCMD oder FSD, sodass der Server es nie als Sekundärgerät zählt,
   auf das vor dem Herunterfahren gewartet werden muss.

!!! warning
    DumbMonit beobachtet die USV; es fährt nichts herunter. Die von der
    USV versorgten Maschinen brauchen weiterhin ihren eigenen
    NUT-Client (upsmon), um sauber herunterzufahren, wenn der Akku
    schwach wird.

## Credentials

| Credential | Felder |
|---|---|
| Keine Authentifizierung | Der Standard: `upsd` lässt jeden Host, den es akzeptiert, die USV-Variablen lesen. |
| upsd-Benutzername / Passwort | Mit `USERNAME` und `PASSWORD` vor dem Lesen gesendet. `upsd` prüft sie nur für Befehle, die DumbMonit nie sendet. |

Adresse: ein Hostname oder eine IP (`nas.lan`), `host:port`, oder die
Form `ups@host` von `upsc`, von der nur der Host behalten wird.
Optionen: Port (3493), die zu beobachtende USV (standardmäßig alle) und
der Timeout pro Austausch (10 s).

Eine abgelehnte Verbindung oder keine Antwort zählt als nicht
erreichbares Gerät. `ACCESS-DENIED` wird als Credential-Fehler
angezeigt, und ein vom Server nicht veröffentlichter USV-Name als
Konfigurationsfehler; keiner weckt jemanden nachts. Die Verbindung ist
reines TCP: `STARTTLS` wird nicht genutzt, und nichts Gelesenes ist
geheim.
