# Tailscale

Die Geräte eines Tailnets über die Tailscale-API: Server, die offline
gingen, bald ablaufende Node-Keys, Geräte, die auf Genehmigung warten,
und Clients mit verfügbarem Update.

DumbMonit liest `GET /api/v2/tailnet/{tailnet}/devices` auf
`api.tailscale.com`, einmal pro Minute. Das empfohlene Credential ist
ein auf den Scope `devices:core:read` begrenzter OAuth-Client: DumbMonit
tauscht dessen ID und Secret gegen ein einstündiges Token
(`/api/v2/oauth/token`) und behält es, bis es abläuft. Er kann die
Geräte auflisten und nichts ändern. Ein API-Access-Token
(`tskey-api-…`) funktioniert auch, trägt aber alle Rechte des
Benutzers, der es erstellt hat, und läuft nach höchstens 90 Tagen ab.

Dies ist eine Internet-API: Die Adresse des Geräts ist die der API,
nicht die einer Maschine. DumbMonit braucht ausgehendes HTTPS zu
`api.tailscale.com`; nichts im Tailnet wird kontaktiert.

!!! warning "Nur gegen die Herstellerdokumentation validiert — noch nicht an einem echten System getestet"

    Diese Integration wurde aus Tailscales API-Dokumentation und dem
    Gerätemodell seines offiziellen Go-Clients gebaut, nicht gegen ein
    lebendes Tailnet. Sie liest nur. Sagen Sie uns, was kaputt ist.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_tailscale_`. Ein Gerät wird
nach dem ersten Label seines MagicDNS-Namens benannt (`nas` für
`nas.tail1234.ts.net`). In das Tailnet von einem anderen geteilte
Geräte werden gezählt, nicht beschrieben.

| Metrik | Labels | Bedeutung |
|---|---|---|
| `device_online` | `device` | 1, während das Gerät mit dem Koordinationsserver verbunden ist. |
| `device_offline_seconds` | `device` | 0, während online, sonst die Zeit seit der letzten Sichtung. |
| `device_watched` | `device` | 1, wenn das Gerät online bleiben muss: standardmäßig jedes getaggte Gerät, oder die in der Option unten benannten. |
| `device_key_expiry_seconds` | `device` | Bis der Node-Key abläuft; fehlt, wenn der Key-Ablauf deaktiviert ist. |
| `device_authorized` | `device` | 0, während das Gerät auf Genehmigung wartet. |
| `device_update_available` | `device` | 1, wenn Tailscale einen neueren Client anbietet. |
| `device_info` | `device`, `os`, `client_version`, `tags` | Wert 1. |
| `devices`, `devices_online`, `devices_external`, `devices_update_available`, `devices_unauthorized`, `devices_watched_offline` | | Zählungen. |

Nur beobachtete Geräte lösen einen Alarm aus, wenn sie offline gehen:
Ein schlafendes Telefon oder Laptop ist kein Ausfall. Getaggte Geräte
sind in der Praxis die Server, Subnetz-Router und Exit-Nodes; listen
Sie andere in der Option **Devices to keep online**, oder geben Sie
`all` ein.

Die geltenden [eingebauten Regeln](../alerting/rules.md#tailscale):

- **Tailscale device offline**: ein beobachtetes Gerät seit mehr als
  15 Minuten getrennt (Advisory).
- **Tailscale node key expiring**: weniger als 14 Tage übrig
  (Advisory, täglich erinnert). Bei Ablauf verlässt das Gerät das
  Tailnet, bis jemand es erneut anmeldet.
- **Tailscale device awaiting approval** (Advisory): genehmigen Sie
  es, wenn erwartet, entfernen Sie es sonst.
- **Tailscale client update**: ein beobachtetes Gerät hat seit sechs
  Stunden ein Client-Update (Info, wöchentlich erinnert).

## Geräte

Jedes Gerät des Tailnets, beobachtet oder nicht (geteilte Geräte
ausgenommen), wird unter der generischen Familie veröffentlicht, die
sich jede DumbMonit-Integration teilt, die Client-Geräte verfolgt —
`dumbmonit_client_device_*`, ohne `tailscale`-Präfix, vollständig
dokumentiert unter [Client-Geräte](../alerting/rules.md#client-devices):

| Metrik | Was | Labels |
|---|---|---|
| `last_seen_timestamp_seconds` | Letzte Verbindung: "jetzt", während `connectedToControl` wahr ist, sonst `lastSeen` | `device`, `type` (`Tagged` oder `Personal`), `os`, `user`, `kind` |
| `stale_seconds` | Positiv, sobald ein Gerät `device_stale_days` (standardmäßig 3) ohne Verbindung überschritten hat; treibt die Regel [Client device stale](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (immer `connection`: Tailscale hat kein Backup zu melden) |

Ein reisendes Telefon oder Laptop kann mehrere Tage ohne erneute
Verbindung sein: Das ist unabhängig von der obigen Einstellung
**Devices to keep online**, die nur für beobachtete, offline gehende
Geräte einen Alarm auslöst. Heben Sie **Device staleness** für ein
Tailnet mit umherziehenden Geräten an.

## Die Geräteseite

Das Tailnet-Panel sagt in einem Satz, was nicht stimmt, zeigt die
Online-Geräte, die offline gegangenen beobachteten Geräte, die
verfügbaren Updates und die geteilten Geräte, listet dann die
beobachteten Geräte und jedes auf Genehmigung wartende oder bald
ablaufende Gerät, dann die Tabelle jedes Geräts mit seiner letzten
Verbindung.

## Einen nur-lesenden OAuth-Client für DumbMonit erstellen

1. Öffnen Sie in der Tailscale-Konsole (login.tailscale.com) Settings >
   Trust credentials (früher OAuth clients) und erzeugen Sie einen
   OAuth-Client nur mit dem Scope devices:core:read. Er listet die
   Geräte und kann nichts ändern.

2. Kopieren Sie die Client-ID und das Client-Secret
   (tskey-client-…), nur einmal angezeigt. Geben Sie in DumbMonit
   api.tailscale.com als Adresse ein und fügen Sie sie als Client ID
   und Client secret ein. Lassen Sie die Tailnet-Option leer: das
   Tailnet des Clients wird gelesen.

!!! warning
    Nur getaggte Geräte (Server, Subnetz-Router, Exit-Nodes) sollen
    online bleiben: Ein schlafendes Telefon oder Laptop ist kein
    Ausfall. Benennen Sie die zu beobachtenden Geräte in der Option
    Devices to keep online, oder geben Sie all ein. Jedes Gerät des
    Tailnets erscheint in der Devices-Tabelle unten mit seiner letzten
    Verbindung, beobachtet oder nicht; ein reisendes Telefon kann
    tagelang ohne eine solche sein, heben Sie also Device staleness
    dafür an.

## Credentials

| Credential | Felder |
|---|---|
| OAuth-Client | Client-ID und Client-Secret (`tskey-client-…`) eines Clients mit dem Scope `devices:core:read`. |
| API-Access-Token | Ein `tskey-api-…`-Token, als Bearer-Token gesendet. |

Adresse: `api.tailscale.com`. Optionen:

- **Tailnet**: leer für das Tailnet der Zugangsdaten, oder der in der
  Tailscale-Konsole angezeigte Tailnet-Name.
- **Devices to keep online**: kommagetrennte Gerätenamen oder Tags
  (`nas, tag:router`), oder `all`. Leer: jedes getaggte Gerät.
- **Timeout pro Request** (15 s).
- **Device staleness** (`device_stale_days`, standardmäßig 3 Tage).

## Fehlerbehebung

"Tailscale refused the OAuth client": die ID oder das Secret ist
falsch, oder der Client wurde gelöscht.

"Tailscale refused to list the devices (403)": dem Client fehlt der
Scope `devices:core:read`.

"Tailscale knows no tailnet …": lassen Sie die Tailnet-Option leer.
