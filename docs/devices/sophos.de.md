# Sophos Firewall

Eine Sophos Firewall (SFOS) über ihre XML-API: der Link jeder einer Zone
zugewiesenen Schnittstelle, und welche IPsec-Verbindungen aktiviert sind.

Das ist absichtlich alles. Die XML-API ist eine Konfigurations-API:
neben dem nur-lesenden Konnektivitätsstatus jeder Schnittstelle stellt
sie keinen Live-Zustand bereit. `<Get><IPSecConnection>` gibt zurück, ob
ein Administrator eine Verbindung aktiviert hat, nicht ob der Tunnel up
ist; Hochverfügbarkeit, Lizenzen und Abonnements sowie die
Firmware-Version sind überhaupt nicht enthalten. DumbMonit
veröffentlicht die Aktivierung als solche und stellt sie nie als
Tunnel-Zustand dar.

Jede Minute sendet DumbMonit zwei Requests an
`https://<firewall>:4444/webconsole/APIController` (Formularfeld
`reqxml`, nie in der URL, wo das Passwort in Logs landen würde):
`<Get><Interface>` und `<Get><IPSecConnection>`. Jeder trägt Benutzername
und Passwort eines Kontos, dessen Geräte-Zugriffsprofil überall
nur-lesend ist: Die Firewall erzwingt dieses Profil auch auf der API.

!!! warning "Nur gegen die Herstellerdokumentation validiert — noch nicht an einem echten System getestet"

    Diese Integration wurde aus Sophos' XML-API-Dokumentation gebaut (den
    Entitäten `Interface` und `IPSecConnection`, den Login- und
    Statusmeldungen), nicht gegen eine laufende Firewall. Der genaue
    Wortlaut des Schnittstellenstatus (`Connected, 1000 Mbps…` oder
    `Disconnected`) stammt aus dieser Dokumentation und
    Community-Berichten. Sagen Sie uns, was kaputt ist.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_sophos_`.

| Metrik | Labels | Bedeutung |
|---|---|---|
| `api_version_info` | `api_version` | Wert 1: die von der Firewall angekündigte API-Version (`2000.2`), die der SFOS-Version folgt. |
| `interface_link_up` | `interface`, `zone` | 1, wenn der Status Connected liest, 0 bei Disconnected; fehlt, wenn der Status etwas anderes ist. |
| `interface_enabled` | `interface`, `zone` | 0, wenn die Schnittstelle ausgeschaltet ist. |
| `interface_down` | `interface`, `zone` | 1, wenn die Schnittstelle eingeschaltet, einer Zone zugewiesen ist und keinen Link hat. Freie Ports (Zone None) bleiben unberücksichtigt. |
| `interfaces`, `interfaces_down` | | Zählungen. |
| `ipsec_connection_activated` | `connection` | 1, wenn die Verbindung in der Konfiguration aktiviert ist. Nicht der Tunnel-Zustand. |
| `ipsec_connections` | | Zählung. |

Die geltende [eingebaute Regel](../alerting/rules.md#sophos-firewall):

- **Sophos interface without link**: eine eingeschaltete, einer Zone
  zugewiesene Schnittstelle hat seit fünf Minuten keinen Link
  (Advisory).

Für Traffic-Zähler fügen Sie die Firewall zusätzlich als
[SNMP-Monitor](snmp.md) hinzu: SFOS beantwortet die Standard-Interface-MIB.

## Die Monitorseite

Das Firewall-Panel sagt, ob jede Schnittstelle in einer Zone ihren Link
hat, und listet dann diese Schnittstellen und die IPsec-Verbindungen mit
ihrer konfigurierten Aktivierung, als solche beschriftet.

## DumbMonit erlauben, die Sophos-Firewall-XML-API zu lesen

1. Fügen Sie unter Profiles > Device access ein Profil namens DumbMonit
   read-only hinzu und setzen Sie jede Berechtigung auf Read-only.

2. Fügen Sie unter Authentication > Users einen Benutzer namens
   dumbmonit vom Typ Administrator mit diesem Profil und einem langen
   Zufallspasswort hinzu.

3. Schalten Sie unter Backup & firmware > API (Administration > API
   access ab SFOS 22) die API-Konfiguration ein und fügen Sie die
   Adresse des DumbMonit-Servers zu den erlaubten IP-Adressen hinzu.

4. Geben Sie in DumbMonit die Adresse der Firewall ein, zum Beispiel
   "sophos.lan", mit dem Benutzernamen dumbmonit und dem Passwort. Die
   API hört auf dem Administrationsport, standardmäßig 4444, mit einem
   selbstsignierten Zertifikat: Kreuzen Sie "Accept an unverifiable
   certificate" an, sofern Sie kein eigenes installiert haben.

!!! warning
    Die XML-API ist eine Konfigurations-API: Sie meldet den Link jeder
    Schnittstelle und ob jede IPsec-Verbindung aktiviert ist, nicht ob
    der Tunnel up ist. Hochverfügbarkeit, Lizenzen und die
    Firmware-Version sind nicht enthalten.

## Credentials

| Credential | Felder |
|---|---|
| Benutzername und Passwort | Der Benutzer `dumbmonit` mit dem nur-lesenden Geräte-Zugriffsprofil. |

Adresse: der Hostname oder die IP der Firewall, `host:port`, oder eine
URL. Optionen:

- **Port** (4444): der Administrationsport.
- **Accept an unverifiable certificate**: das eigene Zertifikat der
  Firewall ist selbstsigniert.
- **Timeout pro Request** (15 s).

## Fehlerbehebung

"Sophos Firewall refused the request (534 …)": Die DumbMonit-Adresse
steht nicht unter den erlaubten IP-Adressen der API-Konfiguration.

"Sophos Firewall refused the login": falscher Benutzername oder
Passwort, oder ein Passwort mit Zeichen, die die Firewall in
API-Aufrufen abweist; ein langes Passwort aus Buchstaben und Ziffern
umgeht die Frage.

"Sophos Firewall did not answer in XML": falscher Port, oder die API ist
ausgeschaltet.
