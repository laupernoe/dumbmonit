# UniFi Network

Das UniFi-Netzwerk und was es verwaltet: Gateways, Switches und Access
Points, die offline, isoliert oder auf Adoption wartend sind,
wartende Firmware-Updates, die WAN-Links und die Internetverbindung,
wie das Gateway sie sieht, die Anzahl der Clients, und die von UniFi
gehobenen Alarme. Es funktioniert mit der UniFi-Network-Anwendung
einer UniFi-OS-Konsole (Dream Machine, Cloud Gateway, Cloud Key) und
mit einem selbst gehosteten UniFi Network Server.

DumbMonit liest UniFi mit einem API-Key oder mit einem lokalen Konto,
das die Rolle **View Only** hält. Es startet nie neu, adoptiert,
aktualisiert oder provisioniert nie ein Gerät.

Der beobachtenswerte Fehler: **ein Access Point oder Switch fällt
leise weg**. Clients wandern zum nächsten Access Point, das Netzwerk
funktioniert weiter, und das Abdeckungsloch oder der verlorene Uplink
wird erst Tage später bemerkt. Dasselbe gilt für einen zweiten
WAN-Link, der ausfiel: Nichts bricht, bis auch der erste ausfällt.

## Zwei Wege hinein

| Credential | Genutzte API | Was sie liest |
|---|---|---|
| API-Key (UniFi-OS-Konsole) | Die offizielle Integration API, unter `/proxy/network/integration/v1` auf einer Konsole und `/integration/v1` auf einem selbst gehosteten Server | Version, Geräte und ihr Zustand, Firmware-Updates, Last, Speicher und Uptime jedes Online-Geräts, Anzahl der Clients |
| View-Only-Konto | Die klassische API (`/api/s/<site>/stat/*`), unter `/proxy/network/api/…` auf einer Konsole | Alles oben, plus Clients pro Gerät und nach Art, die WAN-Links, der WAN- und Internetstatus, und Alarme |

Der WAN-, Internet- und Alarmstatus existiert nur in der klassischen
API. Mit einem Key fragt DumbMonit die klassische API zusätzlich mit
demselben Key nach dem WAN- und Internetstatus; lehnt der Controller
das ab, fehlen diese Zeitreihen einfach, ohne Fehler.

Alarme sind die Einträge von UniFis "critical"-Systemlog der letzten
24 Stunden (UniFi Network 8 und später), oder die nicht archivierten
Alarme auf älteren Versionen. Das ist der einzige `POST`, den
DumbMonit sendet, und er liest nur.

Eine mit dem View-Only-Konto geöffnete Sitzung wird von einer Abfrage
zur nächsten behalten und bei Ablauf neu geöffnet: UniFi protokolliert
nicht jede Minute einen neuen Login.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_unifi_`. Geräte-Zeitreihen
tragen `device` (seinen Namen, oder seine MAC-Adresse, wenn er keinen
hat), `mac`, `model` und `type` (`gateway`, `switch`, `access_point`,
`other`).

| Metrik | Was | Labels |
|---|---|---|
| `info` | Wert 1 | `version`, `api` (`integration` oder `classic`) |
| `devices` | Geräte nach Zustand: `online`, `offline`, `pending`, `updating`, `adopting`, `adoption_failed`, `isolated`, `other` | `state` |
| `devices_upgradable` | adoptierte Geräte mit verfügbarem Firmware-Update | |
| `device_state` | 0 online, 1 offline, 2 pending adoption, 3 updating oder provisioning, 4 adopting, 5 adoption failed, 6 isolated, 7 other | Geräte-Labels |
| `device_up` | 1 online oder updating, 0 offline, isolated oder adoption fehlgeschlagen; fehlt für ein noch nicht adoptiertes Gerät | Geräte-Labels |
| `device_upgradable` | 1, wenn ein Firmware-Update verfügbar ist | Geräte-Labels, `firmware` |
| `device_uptime_seconds`, `device_cpu_percent`, `device_memory_percent` | nur Online-Geräte | Geräte-Labels |
| `device_clients` | mit dem Gerät verbundene Clients (View-Only-Konto) | Geräte-Labels |
| `wan_link_up` | 1, wenn ein WAN-Port des Gateways up ist; ein deaktivierter Port hat keine Zeitreihe | Geräte-Labels, `wan` (`wan1`, `wan2`) |
| `wan_up`, `internet_up` | der WAN- und Internetstatus des Gateways; fehlt ohne UniFi-Gateway | |
| `internet_latency_seconds` | vom Gateway gemessene Latenz | |
| `wan_availability_percent` | Verfügbarkeit jedes WAN über die letzten 24 Stunden | `wan` |
| `subsystem_status` | 0 ok, 1 warning, 2 error, 3 unknown, für `wlan`, `wan`, `www`, `lan`, `vpn` | `subsystem` |
| `clients` | Clients auf der Site | |
| `clients_by_type` | `wireless`, `wired`, `guest`, `vpn` (View-Only-Konto) | `type` |
| `alarms` | kritische Systemlog-Einträge der letzten 24 Stunden, oder nicht archivierte Alarme | |
| `scrape_errors` | bei der letzten Abfrage fehlgeschlagene Aufrufe | |

Die geltenden [eingebauten Regeln](../alerting/rules.md#unifi-network):
UniFi device offline, UniFi Internet down, UniFi WAN link down, UniFi
gateway CPU high, UniFi device waiting for adoption, UniFi firmware
update available, UniFi alarms, plus Device unreachable.

## Geräte

`/rest/user` (die klassische API, mit beiden Credentials lesbar)
listet jeden Client, den der Controller je gesehen hat — in der
Praxis das meiste eines Heimnetzwerks: vorbeiziehende Telefone,
Gast-WLAN, Smart-Geräte, die nie wiederkommen. Alle auf Veraltung zu
verfolgen würde den Alarm unter Geräten begraben, die einfach nie
zurückkehren, nicht unter Geräten, die still wurden. Standardmäßig
zählen nur Clients, die der Benutzer irgendwie markiert hat: in UniFi
benannt, auf eine feste IP reserviert, oder vermerkt. Die Option
**Clients to watch besides the marked ones** fügt bestimmte
MAC-Adressen oder Client-Namen hinzu, oder `all`, um jeden Client zu
verfolgen (nicht empfohlen außer in einem kleinen, meist statischen
Netzwerk).

Verfolgte Clients werden unter der generischen Familie veröffentlicht,
die sich jede DumbMonit-Integration teilt, die Client-Geräte verfolgt
— `dumbmonit_client_device_*`, ohne `unifi`-Präfix, vollständig
dokumentiert unter [Client-Geräte](../alerting/rules.md#client-devices):

| Metrik | Was | Labels |
|---|---|---|
| `last_seen_timestamp_seconds` | Letzte Verbindung, wie `/rest/user` sie meldet | `device`, `type` (`Wired`, `Wireless` oder `Unknown`), `os` (leer), `user` (leer), `kind` |
| `stale_seconds` | Positiv, sobald ein verfolgter Client `device_stale_days` (standardmäßig 3) ohne Verbindung überschritten hat; treibt die Regel [Client device stale](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (immer `connection`: UniFi hat kein Backup zu melden) |

## Die Geräteseite

Das Panel über den Diagrammen liest, was die Abfrage gespeichert hat; das
Öffnen der Seite fragt UniFi nie ab. Es sagt zuerst, was nicht stimmt
(offline Geräte, Internet oder ein WAN-Link down, Alarme), dann die
Geräte nach Art mit ihrem Zustand, Firmware und Last, und die Clients,
dann die Tabelle der verfolgten Clients mit ihrer letzten Verbindung.

## Wie es validiert wurde

Login, die View-Only-Rolle, Fehlerantworten (abgelehntes Passwort,
unbekannte Site, abgelehnter Key, abgelaufene Sitzung) und die
Antworten eines Controllers ohne Geräte wurden gegen UniFi Network
10.6.106, selbst gehostet, geprüft. Kein Gerät konnte dort adoptiert
werden: Die Geräteliste, die Gesundheit einer ausgestatteten Site, die
bekannten Clients (`/rest/user`) und die Payloads der Integration API
sind gegen die offizielle Dokumentation validiert.

## Einen nur-lesenden Zugang in UniFi Network erstellen

1. Auf einer UniFi-OS-Konsole (Dream Machine, Cloud Gateway, Cloud Key)
   können Sie einen API-Key erstellen: In UniFi Network, Settings →
   Control Plane → Integrations → Create API Key. Benennen Sie ihn
   nach DumbMonit und kopieren Sie ihn jetzt: Er wird nur einmal
   angezeigt. Der Key liest Geräte, ihre Last und die Anzahl der
   Clients über die offizielle Integration API.

2. Für den WAN-, Internet- und Alarmstatus, und auf einem selbst
   gehosteten UniFi Network Server, nutzen Sie stattdessen ein
   lokales Konto mit der Rolle View Only: Settings → Admins & Users →
   Create New. Benennen Sie es wie folgt, geben Sie ihm ein langes
   Zufallspasswort, beschränken Sie es auf lokalen Zugriff, und setzen
   Sie seine UniFi-Network-Rolle auf View Only. View Only liest alles
   und kann nichts ändern.

    ```
    dumbmonit
    ```

3. Geben Sie in DumbMonit die Adresse der Konsole oder des Servers
   ein, zum Beispiel "unifi.lan" (Port 443 auf einer Konsole) oder
   "unifi.lan:8443" für einen selbst gehosteten Server, und fügen Sie
   den Key oder das Konto ein. Ist das Netzwerk nicht die
   Standard-Site, setzen Sie die Option Site auf den kurzen Namen, der
   in der Adressleiste nach /manage/ oder /network/ angezeigt wird.

4. DumbMonit liest nur. Es startet nie neu, adoptiert, aktualisiert
   oder provisioniert nie ein Gerät, und bestätigt nie einen Alarm.

!!! warning
    Nutzen Sie nicht das Owner-Konto oder ein UI.com-Cloud-Konto: Ein
    durchgesickertes Passwort würde jede damit verknüpfte Konsole
    öffnen. Lassen Sie Zwei-Faktor-Authentifizierung nur für dieses
    lokale View-Only-Konto aus, da ein Monitoring-Server keinen Code
    eingeben kann. Konsolen nutzen standardmäßig ein selbstsigniertes
    Zertifikat: Aktivieren Sie "Accept an unverifiable certificate",
    sofern Sie kein eigenes installiert haben. UniFi kennt jeden
    jemals verbundenen Client, die meisten davon vorbeiziehende
    Telefone: Nur die benannten, auf eine feste IP reservierten oder
    in UniFi vermerkten werden standardmäßig auf Veraltung beobachtet,
    um einen Alarm für jedes Besuchertelefon zu vermeiden. Listen Sie
    mehr in der Option Clients to watch, oder geben Sie all ein.

## Credentials

| Credential | Felder |
|---|---|
| API-Key (UniFi-OS-Konsole) | Ein unter Settings → Control Plane → Integrations erstellter Key, als `X-API-KEY` gesendet. |
| View-Only-Konto | Benutzername und Passwort des lokalen View-Only-Kontos. |

Adresse: ein Hostname oder eine IP (`unifi.lan`), `host:port`
(`unifi.lan:8443` für einen selbst gehosteten Server), oder eine
vollständige URL; eine aus dem Browser kopierte Adresse mit
`/manage/…` oder `/network/…` wird akzeptiert. Optionen: Site
(`default`), Port (443), Zertifikatsprüfung, Anfrage-Timeout (15 s),
zusätzlich zu den markierten zu beobachtende Clients (leer, oder
`all`), und Geräte-Veralterung (`device_stale_days`, standardmäßig
3 Tage).
