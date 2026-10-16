# FortiGate

Ein Fortinet-FortiGate über die FortiOS-REST-API: down gegangene
Site-to-Site-IPsec-Tunnel, aktivierte Schnittstellen, die ihren Link
verloren haben, ein außer Sync geratener oder ein Mitglied verlierender
HA-Cluster, sich dem Conserve-Modus nähernder Speicher, ablaufende
FortiGuard- und FortiCare-Lizenzen, und angebotene Patch-Versionen für
den laufenden FortiOS-Zweig.

DumbMonit nutzt einen REST-API-Benutzer, dessen Access-Profil nur
liest und der nur Aufrufe von der Adresse des DumbMonit-Servers
akzeptiert (Trusted Host). Sein Token wird als
`Authorization: Bearer` gesendet, nie in der URL: FortiOS 7.4.5 und
7.6.1 lehnen es dort ohnehin ab.

Jede Minute liest DumbMonit drei erforderliche Seiten —
`/api/v2/monitor/system/status`,
`/api/v2/monitor/system/resource/usage` und
`/api/v2/monitor/system/interface/select` — und sechs optionale, die
ein engeres Profil, oder ein FortiGate ohne Cluster oder Tunnel, ohne
Fehlschlag des Rests ablehnen oder leer lassen kann: den
administrativen Zustand der Schnittstellen
(`/api/v2/cmdb/system/interface`), HA-Statistiken und Prüfsummen,
IPsec-Tunnel, Lizenzstatus und verfügbare Firmware.

!!! warning "Nur gegen die Herstellerdokumentation validiert — noch nicht an einem echten System getestet"

    Diese Integration wurde aus Fortinets FortiOS-Dokumentation und
    den vom Community-Prometheus-Exporter für FortiGate
    aufgezeichneten Antwortformen gebaut, nicht gegen ein laufendes
    FortiGate. Jeder Aufruf ist ein Lesevorgang. Die Lizenz- und
    Firmware-Seiten sind die am wenigsten dokumentierten: Sagen Sie
    uns, was kaputt ist.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_fortigate_`.

| Metrik | Labels | Bedeutung |
|---|---|---|
| `version_info` | `version`, `build`, `model`, `hostname` | Wert 1. |
| `cpu_usage_percent`, `memory_used_percent`, `disk_used_percent` | | Aktuelle Nutzung. |
| `sessions`, `session_setup_rate` | | Aktive Sitzungen, neue Sitzungen pro Sekunde. |
| `log_disk_ok` | | 0, wenn die Log-Platte formatiert werden muss. |
| `interface_link_up` | `interface`, `alias` | Physischer Link. |
| `interface_enabled` | `interface`, `alias` | Administrativer Zustand; jede Schnittstelle zählt als aktiviert, wenn das Profil die Konfiguration nicht lesen kann. |
| `interface_down` | `interface`, `alias` | 1, wenn die Schnittstelle aktiviert ist, eine Adresse hat und keinen Link hat. Freie Ports ohne Adresse bleiben unberücksichtigt. |
| `interface_speed_mbps` | `interface`, `alias` | Während der Link up ist. |
| `interface_bytes_in_total`, `interface_bytes_out_total`, `interface_errors_in_total`, `interface_errors_out_total` | `interface`, `alias` | Zähler. |
| `interfaces_down` | | Zählung. |
| `ha_members` | | Mitglieder im Cluster; 0 für ein eigenständiges FortiGate. |
| `ha_in_sync` | | Mit zwei oder mehr Mitgliedern: 1, wenn jedes Mitglied dieselbe Konfigurationsprüfsumme hat. |
| `ha_member_cpu_usage_percent`, `ha_member_memory_used_percent` | `member` | Pro Mitglied. |
| `ipsec_tunnel_up` | `tunnel` | 1, wenn mindestens eine Phase 2 des Tunnels up ist. Dial-up-Tunnel (Remote Access) werden ausgelassen. |
| `ipsec_phase2_up` | `tunnel`, `phase2` | Pro Phase-2-Selektor. |
| `ipsec_tunnel_bytes_in_total`, `ipsec_tunnel_bytes_out_total` | `tunnel` | Zähler. |
| `ipsec_tunnel_info` | `tunnel`, `remote_gateway` | Wert 1. |
| `ipsec_tunnels`, `ipsec_tunnels_down` | | Zählungen. |
| `license_status_info` | `license`, `status` | Wert 1, für jede Lizenz, die das FortiGate hält (`antivirus`, `ips`, `web_filtering`, `forticare_hardware`…); `no_license`-Einträge werden ausgelassen. |
| `license_expired` | `license` | 1, wenn der Status `expired` ist. |
| `license_expiry_seconds` | `license` | Bis zum Ablauf; negativ nach Ablauf. |
| `firmware_update_available` | | 1, wenn FortiGuard eine neuere Patch-Version des laufenden Zweigs anbietet (7.4.4 → 7.4.5). Ein neuer Zweig (7.6) ist ein Projekt, kein Alarm. |
| `firmware_latest_info` | `version` | Wert 1: diese Patch-Version. |

Die geltenden [eingebauten Regeln](../alerting/rules.md#fortigate):

- **FortiGate IPsec tunnel down**: keine Phase 2 up seit fünf Minuten
  (Warning).
- **FortiGate interface without link**: seit fünf Minuten (Advisory).
- **FortiGate HA out of sync**: seit fünfzehn Minuten (Advisory).
- **FortiGate HA member lost**: weniger Mitglieder als zu jedem
  Zeitpunkt der letzten 24 Stunden (Warning). Die erwartete Anzahl
  wird nirgendwo veröffentlicht, daher hört ein endgültig entferntes
  Mitglied am nächsten Tag auf zu alarmieren.
- **FortiGate memory near conserve mode**: über 85 % seit zehn
  Minuten (Advisory). Bei 88 % (standardmäßig) geht FortiOS in den
  Conserve-Modus und hört auf, neue Sitzungen zu prüfen.
- **FortiGate CPU high**: über 90 % seit fünfzehn Minuten (Advisory).
- **FortiGate licence expiring**: weniger als 30 Tage übrig, oder
  abgelaufen (Advisory, täglich erinnert).
- **FortiGate firmware update**: eine Patch-Version wird angeboten
  (Info, wöchentlich erinnert).

## Die Monitorseite

Das Firewall-Panel sagt in einem Satz, was nicht stimmt, zeigt CPU,
Speicher, Sitzungen und den HA-Zustand, listet dann die IPsec-Tunnel
(down zuerst), die Schnittstellen ohne Link, die innerhalb von 60
Tagen ablaufenden oder abgelaufenen Lizenzen, und eine verfügbare
Patch-Version.

## Einen nur-lesenden REST-API-Benutzer für DumbMonit erstellen

1. Erstellen Sie über die FortiGate-CLI ein Access-Profil, das nur
   den System-, Netzwerk- und VPN-Zustand lesen kann, und nichts
   sonst.

    ```
    config system accprofile
    edit "dumbmonit-ro"
    set sysgrp read
    set netgrp read
    set vpngrp read
    set fwgrp none
    set loggrp none
    set utmgrp none
    set wanoptgrp none
    set wifi none
    next
    end
    ```

2. Erstellen Sie den REST-API-Benutzer dumbmonit mit diesem Profil,
   der nur der Adresse des DumbMonit-Servers vertraut (192.0.2.10
   ersetzen), und erzeugen Sie dann sein Token: Es wird einmal
   ausgegeben.

    ```
    config system api-user
    edit "dumbmonit"
    set accprofile "dumbmonit-ro"
    set vdom "root"
    config trusthost
    edit 1
    set ipv4-trusthost 192.0.2.10 255.255.255.255
    next
    end
    next
    end
    execute api-user generate-key dumbmonit
    ```

3. Geben Sie in DumbMonit die Adresse des FortiGate ein, zum Beispiel
   "fw.lan" (fügen Sie den Port hinzu, falls die
   Administrationsoberfläche nicht auf 443 hört), und fügen Sie das
   Token als API-Token ein. FortiGate liefert ein selbstsigniertes
   Zertifikat: Kreuzen Sie "Accept an unverifiable certificate" an,
   sofern Sie kein eigenes installiert haben.

!!! warning
    Eine Schnittstelle zählt nur als down, wenn sie aktiviert ist,
    eine Adresse hat und keinen Link hat: Freie Ports bleiben
    unberücksichtigt. Firmware-Updates sind die, die FortiGuard dem
    FortiGate selbst anbietet: ohne Zugriff auf FortiGuard wird keines
    gemeldet.

## Credentials

| Credential | Felder |
|---|---|
| API-Token | Das von `execute api-user generate-key dumbmonit` ausgegebene Token, als `Authorization: Bearer` gesendet. |

Adresse: der Hostname oder die IP des FortiGate, `host:port`, oder
eine URL. Optionen:

- **Port** (443): der HTTPS-Port der Administrationsoberfläche
  (`set admin-sport` unter `config system global`).
- **VDOM**: die zu lesende virtuelle Domäne; leer für die, zu der der
  REST-API-Benutzer gehört.
- **Accept an unverifiable certificate**: das eigene Zertifikat des
  FortiGate ist selbstsigniert.
- **Timeout pro Request** (15 s).

## Fehlerbehebung

"FortiGate refused the API token (401)": Das Token ist falsch oder
wurde neu erzeugt. Erzeugen Sie ein neues mit
`execute api-user generate-key dumbmonit`.

"FortiGate refused … (403)": Die DumbMonit-Adresse ist nicht unter den
Trusted Hosts des Benutzers, oder sein Profil kann diese Seite nicht
lesen. Eine optionale Seite, die so abgelehnt wird, wird einfach
ausgelassen.

Keine IPsec-, Lizenz- oder Firmware-Zahl: Das Profil kann sie nicht
lesen, oder das FortiGate hat keinen Tunnel; der Rest wird trotzdem
gelesen.
