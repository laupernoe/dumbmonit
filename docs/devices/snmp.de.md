# SNMP-Gerät

Die universellste Art: fast jede Netzwerk-Hardware spricht SNMP. Switches,
Router, NAS, USV und Drucker qualifizieren sich alle. SNMP v1, v2c und v3
werden unterstützt.

## Was beobachtet wird

Das Erfassungsprofil wird automatisch aus der `sysObjectID` des Geräts
erkannt (und, für USV, aus seiner `sysDescr`). Sieben Profile werden mit
dem Produkt mitgeliefert; ein Gerät kann zu mehreren passen, und die
Profile schließen einander ein.

| Profil | Gilt für | Metriken (Präfix `dumbmonit_`) |
|---|---|---|
| System (SNMPv2-MIB) | Jedes Gerät | `system_uptime_seconds`, `system_info` (Name, Beschreibung, Standort, Kontakt als Labels) |
| Netzwerkschnittstellen (IF-MIB) | Jedes Gerät, als Fallback | `if_octets_in/out`, `if_packets_in/out`, `if_errors_in/out`, `if_discards_in/out`, `if_admin_status`, `if_oper_status`, `if_speed_bps`, pro Schnittstelle |
| Host-Ressourcen (HOST-RESOURCES-MIB) | Net-SNMP (Linux, BSD, macOS), der Windows-SNMP-Agent, Synology, QNAP, Solaris | `host_uptime_seconds`, `host_processes`, `host_users`, `cpu_load_percent`, `storage_bytes_total/used` pro Dateisystem, `memory_bytes_total/used`, `load_average` (1, 5 und 15 Minuten, nur Net-SNMP) |
| USV (UPS-MIB) | APC, Eaton, MGE, CyberPower, Riello, Socomec, Tripp Lite, Vertiv, Salicru, und jeder Agent, der UPS-MIB ankündigt | `ups_battery_status`, `ups_battery_seconds_on`, `ups_battery_minutes_remaining`, `ups_battery_charge_percent`, Akku-Volt/Ampere/Temperatur, Eingangs-Volt/Hertz/Line-Bads, `ups_output_source`, Ausgangs-Volt/Hertz/Watt, `ups_output_load_percent`, `ups_alarms_present` |
| Drucker (PRINTER-MIB) | HP, Kyocera, Epson, Canon, Brother, Lexmark, Ricoh, Xerox, Samsung, OKI, Konica Minolta | `printer_supply_level/max` pro Verbrauchsmaterial, `printer_input_level/max` pro Fach, `printer_pages_printed`, `printer_status` |
| Ethernet-Switch (EtherLike-MIB, LLDP) | Von Hand für jeden Switch wählbar; von den herstellerspezifischen Switch-Profilen eingeschlossen | Alles aus IF-MIB, plus pro Port `ether_fcs_errors`, `ether_alignment_errors`, `ether_symbol_errors`, `ether_late_collisions`, `ether_excessive_collisions`, `ether_mac_transmit_errors`, `ether_mac_receive_errors`, `ether_duplex_status` (1 unbekannt, 2 halb, 3 voll), `if_broadcast_packets_in`, `if_multicast_packets_in` (64-Bit), und `lldp_neighbor_info` (wer wo eingesteckt ist: `remote_sysname`, `remote_port`, `remote_port_desc`; die lokale Port-Nummer ist der zweite Teil des `index`-Labels) |
| Zyxel-Switch | Jede Zyxel-`sysObjectID` (`1.3.6.1.4.1.890`) | Alles aus Ethernet-Switch, plus `cpu_load_percent` (Einminuten-Durchschnitt), `zyxel_cpu_5min_percent`, `memory_usage_percent`, `zyxel_firmware_info` (Firmware und Modell als Labels), und auf Switches, die den Hardware-Monitor veröffentlichen, `zyxel_temperature_celsius` und `zyxel_temperature_high_threshold_celsius` pro Sensor, `zyxel_fan_rpm` und `zyxel_fan_low_threshold_rpm` pro Lüfter |

Zähler (Schnittstellen-Octets, gedruckte Seiten) werden roh gespeichert:
Nutzen Sie `rate()` in einer Abfrage, um einen Durchsatz zu erhalten. Ein
Geräte-Reboot zeigt sich als Zähler-Reset, nie als gefälschter Ausschlag.

Die für SNMP-Geräte geltenden [eingebauten Regeln](../alerting/rules.md):
Device unreachable, High CPU, Disk almost full, Filesystem almost full
(Vorhersage), UPS on battery, UPS battery low, Port accumulating errors,
Port flapping, Unusual CPU (Baseline).

### Zyxel-Switches

Der Standardteil des Zyxel-Profils (Schnittstellen, Ethernet-Fehler,
LLDP) ist allen Zyxel-Switches gemein. Die CPU-, Speicher- und
Firmware-Werte stammen aus dem privaten Zweig, den die aktuellen
Managed Switches unter `1.3.6.1.4.1.890.1.15` teilen (der XS1930 und der
GS1900 darunter); ältere Modelle und Zyxel-Firewalls legen die ihren
anders aus, und diese Baureihen bleiben dort einfach leer. Das Profil
wurde gegen ein echtes XS1930-10 geprüft; seine Temperatur- und
Lüfter-Zeitreihen folgen Zyxels veröffentlichter Hardware-Monitor-MIB,
wurden aber noch nicht auf echter Hardware beobachtet.

`dot3StatsFrameTooLongs` wird absichtlich nicht gelesen: Auf dem XS1930
zählt es Milliarden völlig gesunder Jumbo-Frames, und eine Zeitreihe,
die auf einem gesunden Netzwerk immer steigt, lehrt Menschen nur, sie
zu ignorieren.

### Server-Management-Controller (BMC)

Der eigene SNMP-Agent eines BMC beschreibt den BMC, nicht den Server:
sein kleines ARM-Linux, seinen Speicher, seine Netzwerkverbindungen. Ein
Supermicro-BMC antwortet zum Beispiel als Net-SNMP und erhält das
Host-Ressourcen-Profil, das alles liest, was dieser Agent über sich
selbst veröffentlicht. Die Lüfter, Temperaturen, Netzteile und
Laufwerke des Servers werden stattdessen über [Redfish](redfish.md)
gelesen.

## Was auf dem Gerät vorzubereiten ist

1. Öffnen Sie die Verwaltungsoberfläche des Geräts.
2. Suchen Sie den SNMP-Abschnitt, oft unter "Network", "Services" oder
   "Administration".
3. Aktivieren Sie SNMP v2c und merken Sie sich die nur-lesende Community
   (standardmäßig "public" bei vielen Geräten).
4. Filtert das Gerät nach Adresse, erlauben Sie die Adresse des
   DumbMonit-Servers.
5. Kommen Sie zu DumbMonit zurück, geben Sie die Adresse und die
   Community ein: der Rest wird automatisch erkannt.

!!! warning
    Eine Community ist im Netzwerk nicht verschlüsselt. In einem
    gemeinsam genutzten Netzwerk bevorzugen Sie SNMP v3, das
    authentifiziert und verschlüsselt.

## Credentials

| Credential | Felder |
|---|---|
| SNMP v1 / v2c (Community) | Die nur-lesende Community. |
| SNMP v3 | Benutzername; optionale Authentifizierung (MD5, SHA-1, SHA-224, SHA-256, SHA-384 oder SHA-512 und eine Passphrase); optionale Privacy (DES, AES-128, AES-192 oder AES-256 und eine Passphrase); optionaler Kontextname. |

Adresse: eine IP oder ein Hostname, zum Beispiel `192.168.1.10`. Der
Standardport ist 161; schreiben Sie `host:port`, um einen anderen zu
nutzen.

## Optionen

Diese Art liest keine Optionen. Freie Tags sind weiterhin verfügbar und
werden auf jeder Zeitreihe als `tag_<key>` kopiert.

## Häufige Fehler

| Symptom | Wahrscheinliche Ursache |
|---|---|
| *Unreachable* direkt nach dem Hinzufügen | Falsche Community, SNMP nicht aktiviert, oder das Gerät akzeptiert nur Anfragen von erlaubten Adressen. Das Gerät antwortet nie auf eine falsche Community: Es sieht genau wie ein ausgeschaltetes Gerät aus. |
| Nur `system_*`- und `if_*`-Zeitreihen | Die `sysObjectID` passte zu keinem spezifischen Profil. IF-MIB und System gelten für alles; HOST-RESOURCES, UPS, PRINTER und Zyxel brauchen eine bekannte Hersteller-OID. Wählen Sie bei einem Switch einer anderen Marke das Ethernet-Switch-Profil von Hand. |
| Kein Schnittstellen-Traffic | Das Gerät stellt die 64-Bit-Zähler der `ifXTable` nicht bereit. |
| Leerer Scan | Der Scan versucht eine Community auf jeder Adresse; Geräte mit einer anderen Community oder nur SNMP v3 antworten nicht. |

Nutzen Sie **Probe now** auf der Geräteseite: Es meldet die Anzahl der
Samples und die von einer Abfrage erzeugten Zeitreihennamen.
