# UniFi Network {#unifi-network}

Das UniFi-Netzwerk und was es verwaltet: Gateways, Switches und Access Points, die offline,
isoliert oder in Erwartung der Adoption sind, wartende Firmware-Updates, die WAN-Verbindungen
und die Internetverbindung, wie das Gateway sie sieht, die Anzahl der Clients und die von UniFi
ausgelösten Alarme. Es funktioniert mit der UniFi-Network-Anwendung einer UniFi-OS-Konsole
(Dream Machine, Cloud Gateway, Cloud Key) und mit einem selbst gehosteten UniFi Network Server.

DumbMonit liest UniFi mit einem API-Schlüssel oder mit einem lokalen Konto mit der Rolle
**Nur Ansicht**. Es startet nie ein Gerät neu, adoptiert, aktualisiert oder provisioniert eines.

Der Fehler, auf den es ankommt: **ein Access Point oder ein Switch fällt still aus**. Clients
wechseln zum nächsten Access Point, das Netzwerk funktioniert weiter, und die Funklücke oder der
verlorene Uplink fällt erst Tage später auf. Dasselbe gilt für eine zweite WAN-Verbindung, die
ausgefallen ist: Nichts geht kaputt, bis auch die erste ausfällt.

## Zwei Zugänge {#two-ways-in}

| Zugangsdaten | Verwendete API | Was gelesen wird |
|---|---|---|
| API-Schlüssel (UniFi-OS-Konsole) | Die offizielle Integration API, unter `/proxy/network/integration/v1` auf einer Konsole und `/integration/v1` auf einem selbst gehosteten Server | Version, Geräte und ihr Zustand, Firmware-Updates, Last, Speicher und Laufzeit jedes Online-Geräts, Anzahl der Clients |
| Konto Nur Ansicht | Die klassische API (`/api/s/<site>/stat/*`), unter `/proxy/network/api/…` auf einer Konsole | Alles oben Genannte, außerdem Clients pro Gerät und nach Typ, die WAN-Verbindungen, den WAN- und Internetstatus und Alarme |

Der WAN-, Internet- und Alarmstatus existiert nur in der klassischen API. Mit einem Schlüssel
fragt DumbMonit die klassische API für den WAN- und Internetstatus ebenfalls mit demselben
Schlüssel ab; lehnt der Controller ihn ab, fehlen diese Serien einfach, ohne Fehler.

Alarme sind die Einträge des „kritischen“ Systemlogs von UniFi der letzten 24 Stunden (UniFi
Network 8 und neuer) oder bei älteren Versionen die nicht archivierten Alarme. Das ist das einzige
`POST`, das DumbMonit sendet, und es liest nur.

Eine mit dem Konto Nur Ansicht geöffnete Sitzung wird von einer Prüfung zur nächsten behalten und
bei Ablauf neu geöffnet: UniFi protokolliert nicht jede Minute eine neue Anmeldung.

## Was überwacht wird {#what-it-watches}

Alle Metriken haben das Präfix `dumbmonit_unifi_`. Geräteserien tragen `device` (sein Name oder
seine MAC-Adresse, wenn er keinen hat), `mac`, `model` und `type` (`gateway`, `switch`,
`access_point`, `other`).

| Metrik | Was | Labels |
|---|---|---|
| `info` | Wert 1 | `version`, `api` (`integration` oder `classic`) |
| `devices` | Geräte nach Zustand: `online`, `offline`, `pending`, `updating`, `adopting`, `adoption_failed`, `isolated`, `other` | `state` |
| `devices_upgradable` | adoptierte Geräte mit verfügbarem Firmware-Update | |
| `device_state` | 0 online, 1 offline, 2 Adoption ausstehend, 3 Update oder Provisionierung, 4 Adoption läuft, 5 Adoption fehlgeschlagen, 6 isoliert, 7 sonstiges | Geräte-Labels |
| `device_up` | 1 online oder im Update, 0 offline, isoliert oder Adoption fehlgeschlagen; fehlt bei einem noch nicht adoptierten Gerät | Geräte-Labels |
| `device_upgradable` | 1, wenn ein Firmware-Update verfügbar ist | Geräte-Labels, `firmware` |
| `device_uptime_seconds`, `device_cpu_percent`, `device_memory_percent` | nur Online-Geräte | Geräte-Labels |
| `device_clients` | mit dem Gerät verbundene Clients (Konto Nur Ansicht) | Geräte-Labels |
| `wan_link_up` | 1, wenn ein WAN-Port des Gateways aktiv ist; ein deaktivierter Port hat keine Serie | Geräte-Labels, `wan` (`wan1`, `wan2`) |
| `wan_up`, `internet_up` | WAN- und Internetstatus des Gateways; fehlt ohne UniFi-Gateway | |
| `internet_latency_seconds` | vom Gateway gemessene Latenz | |
| `wan_availability_percent` | Verfügbarkeit jedes WAN über die letzten 24 Stunden | `wan` |
| `subsystem_status` | 0 ok, 1 Warnung, 2 Fehler, 3 unbekannt, für `wlan`, `wan`, `www`, `lan`, `vpn` | `subsystem` |
| `clients` | Clients am Standort | |
| `clients_by_type` | `wireless`, `wired`, `guest`, `vpn` (Konto Nur Ansicht) | `type` |
| `alarms` | kritische Systemlog-Einträge der letzten 24 Stunden oder nicht archivierte Alarme | |
| `scrape_errors` | bei der letzten Prüfung fehlgeschlagene Aufrufe | |

Die zutreffenden [integrierten Regeln](../alerting/rules.md#unifi-network): UniFi-Gerät offline,
UniFi-Internet ausgefallen, UniFi-WAN-Verbindung ausgefallen, UniFi-Gateway-CPU hoch,
UniFi-Gerät wartet auf Adoption, UniFi-Firmware-Update verfügbar, UniFi-Alarme, außerdem Monitor
nicht erreichbar.

## Geräte {#devices}

`/rest/user` (die klassische API, mit beiden Zugangsdaten lesbar) listet jeden Client auf, den
der Controller je gesehen hat — in der Praxis das meiste eines Heimnetzes: vorbeikommende
Smartphones, Gast-WLAN, Smart-Geräte, die nie zurückkehren. Alle auf Veralterung zu verfolgen
würde den Alarm unter Geräten begraben, die schlicht nie wiederkehren, statt unter solchen, die
verstummt sind. Standardmäßig zählen nur Clients, die der Benutzer auf die eine oder andere Weise
markiert hat: in UniFi benannt, auf einer festen IP reserviert oder mit einer Notiz versehen. Die
Option **Zusätzlich zu den markierten zu überwachende Clients** fügt bestimmte MAC-Adressen oder
Clientnamen hinzu oder `all`, um jeden Client zu verfolgen (außer bei einem kleinen, meist
statischen Netzwerk nicht empfohlen).

Verfolgte Clients werden unter der generischen Familie veröffentlicht, die jede DumbMonit-
Integration teilt, die Client-Geräte verfolgt — `dumbmonit_client_device_*`, mit Präfix ohne
`unifi`, ausführlich dokumentiert unter [Client-Geräte](../alerting/rules.md#client-devices):

| Metrik | Was | Labels |
|---|---|---|
| `last_seen_timestamp_seconds` | Letzte Verbindung, wie `/rest/user` sie meldet | `device`, `type` (`Wired`, `Wireless` oder `Unknown`), `os` (leer), `user` (leer), `kind` |
| `stale_seconds` | Positiv, sobald ein verfolgter Client `device_stale_days` (standardmäßig 3) ohne Verbindung überschritten hat; speist die Regel [Client-Gerät veraltet](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (immer `connection`: UniFi hat kein Backup zu melden) |

## Die Monitorseite {#the-device-page}

Das Panel über den Diagrammen liest, was die Prüfung gespeichert hat; das Öffnen der Seite fragt
UniFi nie ab. Es nennt zuerst, was nicht stimmt (Geräte offline, Internet oder eine
WAN-Verbindung ausgefallen, Alarme), dann die Geräte nach Typ mit ihrem Zustand, ihrer Firmware
und Last sowie die Clients, dann die Tabelle der verfolgten Clients mit ihrer letzten Verbindung.

## Wie es validiert wurde {#how-it-was-validated}

Anmeldung, die Rolle Nur Ansicht, Fehlerantworten (abgelehntes Passwort, unbekannter Standort,
abgelehnter Schlüssel, abgelaufene Sitzung) und die Antworten eines Controllers ohne Geräte
wurden gegen UniFi Network 10.6.106, selbst gehostet, geprüft. Dort konnte kein Gerät adoptiert
werden: Die Geräteliste, der Zustand eines ausgestatteten Standorts, die bekannten Clients
(`/rest/user`) und die Payloads der Integration API sind gegen die offizielle Dokumentation
validiert.

## Einen Nur-Lese-Zugang in UniFi Network erstellen {#create-a-read-only-access-in-unifi-network}

1. Auf einer UniFi-OS-Konsole (Dream Machine, Cloud Gateway, Cloud Key) können Sie einen API-Schlüssel erstellen: in UniFi Network Einstellungen → Control Plane → Integrationen → API-Schlüssel erstellen. Benennen Sie ihn nach DumbMonit und kopieren Sie ihn jetzt: Er wird nur einmal angezeigt. Der Schlüssel liest Geräte, ihre Last und die Anzahl der Clients über die offizielle Integration API.

2. Für den WAN-, Internet- und Alarmstatus und auf einem selbst gehosteten UniFi Network Server verwenden Sie stattdessen ein lokales Konto mit der Rolle Nur Ansicht: Einstellungen → Admins & Benutzer → Neu erstellen. Benennen Sie es wie folgt, geben Sie ihm ein langes zufälliges Passwort, beschränken Sie es auf lokalen Zugriff und setzen Sie seine UniFi-Network-Rolle auf Nur Ansicht. Nur Ansicht liest alles und kann nichts ändern.

    ```
    dumbmonit
    ```

3. Geben Sie in DumbMonit die Adresse der Konsole oder des Servers ein, zum Beispiel „unifi.lan“ (Port 443 auf einer Konsole) oder „unifi.lan:8443“ für einen selbst gehosteten Server, und fügen Sie den Schlüssel oder das Konto ein. Ist das Netzwerk nicht der Standardstandort, setzen Sie die Option Standort auf den Kurznamen, der in der Adressleiste nach /manage/ oder /network/ steht.

4. DumbMonit liest nur. Es startet nie ein Gerät neu, adoptiert, aktualisiert oder provisioniert eines und bestätigt nie einen Alarm.

!!! warning
    Verwenden Sie weder das Inhaberkonto noch ein UI.com-Cloud-Konto: Ein durchgesickertes Passwort würde jede damit verknüpfte Konsole öffnen. Lassen Sie die Zwei-Faktor-Authentifizierung nur für dieses lokale Konto Nur Ansicht aus, da ein Monitoring-Server keinen Code eintippen kann. Konsolen verwenden standardmäßig ein selbstsigniertes Zertifikat: Aktivieren Sie „Nicht prüfbares Zertifikat akzeptieren“, sofern Sie kein eigenes installiert haben. UniFi kennt jeden Client, der sich je verbunden hat, meist vorbeikommende Smartphones: Standardmäßig werden nur die in UniFi benannten, auf einer festen IP reservierten oder mit einer Notiz versehenen auf Veralterung überwacht, um keinen Alarm für das Smartphone jedes Besuchers auszulösen. Weitere führen Sie in der Option Zu überwachende Clients auf oder geben all ein.

## Controller noch nicht eingerichtet {#controller-not-set-up-yet}

Ein selbst gehosteter Controller, dessen Ersteinrichtungsassistent nicht abgeschlossen ist,
beantwortet jede Anfrage mit einer Weiterleitung auf `/setup`. DumbMonit meldet dann „Der
UniFi-Controller ist noch nicht eingerichtet“ als Konfigurationsfehler (nicht als abgelehntes
Passwort): Schließen Sie den Assistenten im Browser ab und prüfen Sie dann erneut.

## Zugangsdaten {#credentials}

| Zugangsdaten | Felder |
|---|---|
| API-Schlüssel (UniFi-OS-Konsole) | Ein unter Einstellungen → Control Plane → Integrationen erstellter Schlüssel, gesendet als `X-API-KEY`. |
| Konto Nur Ansicht | Benutzername und Passwort des lokalen Kontos Nur Ansicht. |

Adresse: ein Hostname oder eine IP (`unifi.lan`), `host:port` (`unifi.lan:8443` für einen selbst
gehosteten Server) oder eine vollständige URL; eine aus dem Browser kopierte Adresse mit
`/manage/…` oder `/network/…` wird akzeptiert. Optionen: Standort (`default`), Port (443),
Zertifikatsprüfung, Anfrage-Timeout (15 s), zusätzlich zu den markierten zu überwachende Clients
(leer oder `all`) und Geräteveralterung (`device_stale_days`, standardmäßig 3 Tage).
