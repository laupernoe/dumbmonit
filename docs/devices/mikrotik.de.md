# MikroTik RouterOS

Router und Switches mit RouterOS 7: ob RouterOS und die
RouterBOOT-Firmware zurückgefallen sind, CPU, Speicher und Storage,
Temperaturen, Lüfter und Netzteile, und der Zustand, Traffic, Fehler
und Link-Verluste jeder Schnittstelle.

DumbMonit liest die REST-API, die RouterOS 7 unter `/rest` bedient,
mit einem Benutzer in einer Gruppe, die nur lesen kann. Es sendet
`GET`-Requests und nichts sonst.

Ein MikroTik bricht selten; es altert. Eine nicht mehr aktuelle
RouterOS-Version auf ihrem Kanal, eine nach einem Upgrade
zurückgebliebene RouterBOOT-Firmware (sie folgt nur auf Anfrage, beim
nächsten Reboot), ein ausgefallenes redundantes Netzteil, das niemand
bemerkt hat, ein Port, der weiter Fehler protokolliert oder seinen
Link verliert. Das beobachtet diese Seite, zusammen mit der Last.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_mikrotik_`. Totals sind
Zähler: Diagramme und Regeln verwandeln sie in Raten.

| Metrik | Was | Labels |
|---|---|---|
| `info` | Wert 1 | `version`, `release` (`long-term`, `stable`…), `board`, `architecture`, `identity` |
| `uptime_seconds` | Zeit seit dem letzten Boot | |
| `cpu_load_percent`, `cpu_count` | CPU-Last, alle Kerne | |
| `memory_total_bytes`, `memory_free_bytes`, `memory_used_percent` | Speicher | |
| `storage_total_bytes`, `storage_free_bytes`, `storage_used_percent` | der Flash oder die Platte, auf der RouterOS läuft | |
| `update_checked` | 1, wenn der Router nach einem neueren RouterOS gesucht und eine Antwort erhalten hat | |
| `update_available` | 1, wenn die neueste Version des Kanals von der installierten abweicht | `installed_version`, `latest_version`, `channel` |
| `routerboard` | 1 auf RouterBOARD-Hardware, 0 auf einem Cloud Hosted Router oder einer x86-Maschine | |
| `firmware_upgrade_pending` | 1, wenn RouterBOOT älter ist als die mit RouterOS gelieferte Firmware | `current_firmware`, `upgrade_firmware`, `model` |
| `temperature_celsius`, `voltage_volts`, `fan_rpm`, `power_watts`, `current_amperes` | jeder Sensor der Platine | `sensor` (`cpu-temperature`, `fan1-speed`…) |
| `health_ok` | 1, wenn ein `psu*-state`- oder `fan*-state`-Sensor `ok` liest, sonst 0 | `sensor` |
| `interface_running` | 1, wenn die Schnittstelle einen Link hat | `interface`, `type` |
| `interface_rx_bytes_total`, `interface_tx_bytes_total`, `interface_rx_packets_total`, `interface_tx_packets_total` | Traffic | `interface`, `type` |
| `interface_rx_errors_total`, `interface_tx_errors_total`, `interface_rx_drops_total`, `interface_tx_drops_total` | Fehler und Drops | `interface`, `type` |
| `interface_link_downs_total` | Male, die der Link verloren ging | `interface`, `type` |
| `interfaces_skipped` | über das in den Optionen gesetzte Limit hinaus ausgelassene Schnittstellen | |
| `scrape_errors` | sekundäre Lesevorgänge, die bei der letzten Abfrage fehlgeschlagen sind | |

Nur aktivierte Schnittstellen werden gelesen. Dynamische (ein
PPPoE- oder L2TP-Client, dessen Name sich bei jeder Verbindung ändert)
und das Loopback werden ausgelassen.

Sensoren hängen von der Platine ab: Ein hAP hat eine Spannung und eine
Temperatur, ein CCR fügt Lüfter und Netzteile hinzu, ein Cloud Hosted
Router hat keine. Ein Sensor, den die Platine nicht hat, erzeugt keine
Zeitreihe.

Die neueste RouterOS-Version ist nur bekannt, wenn der Router selbst
nach Updates sucht: Ein nur-lesender Benutzer darf diesen Check nicht
starten. Der optionale Schritt unten plant ihn auf dem Router. Ohne
ihn bleibt `update_checked` bei 0, und die Update-Regel feuert nie.

Die geltenden [eingebauten Regeln](../alerting/rules.md#mikrotik-routeros):
MikroTik CPU high, memory high, storage almost full, temperature high,
power supply or fan failed, interface accumulating errors, link
flapping, RouterOS update available und firmware upgrade pending, plus
Monitor unreachable.

## Die Monitorseite

Das Panel über den Diagrammen liest, was die Abfrage gespeichert hat; das
Öffnen der Seite verbindet sich nie mit dem Router. Es zeigt die
RouterOS-Version und ob eine neuere verfügbar ist, die
RouterBOOT-Firmware, die Uptime, CPU, Speicher und Storage, jeden
Sensor mit seinem Zustand, und eine Tabelle der Schnittstellen mit
ihrem Link, Traffic, Fehlern und Link-Verlusten.

## Einen nur-lesenden Benutzer auf dem Router erstellen

1. Öffnen Sie ein Terminal auf dem Router (WinBox → New Terminal, oder
   SSH mit dem Standardkonto) und erstellen Sie eine Gruppe, die nur
   über die REST-API lesen kann. RouterOS braucht für ein REST-Lesen
   alle drei Policies: rest-api öffnet die API, api und read erlauben
   das Lesen selbst.

    ```
    /user group add name=dumbmonit-read policy=read,api,rest-api comment=DumbMonit
    ```

2. Erstellen Sie den Benutzer in dieser Gruppe mit einem eigenen
   langen Zufallspasswort. Um das Konto nur von DumbMonit zu
   akzeptieren, fügen Sie address= gefolgt von der Adresse des
   DumbMonit-Hosts hinzu.

    ```
    /user add name=dumbmonit group=dumbmonit-read password=REPLACE-WITH-A-LONG-RANDOM-PASSWORD
    ```

3. Aktivieren Sie den HTTPS-Webdienst, den RouterOS deaktiviert und
   ohne Zertifikat liefert. Überspringen Sie das, wenn www-ssl schon
   eines hat. Sonst erstellen diese Befehle ein Zertifikat auf dem
   Router selbst; kreuzen Sie dann unten "Accept an unverifiable
   certificate" in den Optionen an.

    ```
    /certificate add name=local-ca common-name=local-ca key-usage=key-cert-sign,crl-sign days-valid=3650
    /certificate sign local-ca
    /certificate add name=https-cert common-name=router.lan days-valid=825
    /certificate sign https-cert ca=local-ca
    /ip service set www-ssl certificate=https-cert disabled=no
    ```

4. Optional: Um über neue RouterOS-Versionen informiert zu werden,
   lassen Sie den Router beim Start und einmal täglich danach suchen.
   Ein nur-lesender Benutzer darf diesen Check nicht starten, daher
   liest DumbMonit nur sein Ergebnis.

    ```
    /system scheduler add name=check-for-updates interval=1d start-time=startup on-event="/system package update check-for-updates once"
    ```

5. Geben Sie in DumbMonit die Adresse des Routers ein, zum Beispiel
   "192.168.88.1", mit dem Benutzer dumbmonit und seinem Passwort.
   DumbMonit sendet nur GET-Requests an /rest: Es ändert nie eine
   Einstellung, startet nie ein Update und bootet den Router nie neu.

!!! warning
    RouterOS 7.1 oder später wird benötigt: RouterOS 6 hat keine
    REST-API. Nutzen Sie nicht das Standardkonto: Seine volle Gruppe
    kann alles ändern, während dumbmonit-read keine Schreib-Policy hat
    und jede Änderung abgelehnt wird. Der www-Dienst (einfaches HTTP)
    beantwortet die REST-API ebenfalls, sendet das Passwort aber mit
    jedem Request im Klartext.

Die drei Policies wurden einzeln auf RouterOS 7.23.7 geprüft, jeweils
mit einem frischen Benutzer: ohne `rest-api` antwortet der Router mit
401, ohne `api` mit "not allowed", ohne `read` mit "not enough
permissions". DumbMonit meldet jeden Fall als Credential-Problem mit
Nennung der fehlenden Policy, nicht als Router down. Mit allen drei
wird eine Änderung, ein Reboot oder ein Update-Check mit "not enough
permissions" abgelehnt.

RouterOS kann die früheren Policies einer Gruppe eine Weile weiter auf
einen Benutzer anwenden, nachdem die Gruppe bearbeitet wurde. Ändern
Sie die Policies einer bestehenden Gruppe, und der Fehler bleibt,
entfernen und erstellen Sie den Benutzer neu.

## Credentials

| Credential | Felder |
|---|---|
| Nur-lesender Benutzer | Der Benutzer `dumbmonit` und sein Passwort, mit jedem Request als HTTP-Basic-Authentifizierung gesendet. Die REST-API hat keine Sitzung. |

Adresse: ein Hostname oder eine IP (`192.168.88.1`), `host:port`, oder
eine vollständige URL (`https://router.lan:8443`). Optionen: Protokoll
(standardmäßig HTTPS; HTTP nutzt den `www`-Dienst), Port (443 für
HTTPS, 80 für HTTP), Zertifikatsprüfung, Anfrage-Timeout (10 s), ob die
Schnittstellen gelesen werden (ein), und wie viele höchstens (64).
