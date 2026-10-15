# pfSense

Eine pfSense-Firewall über das Community-REST-API-Paket
(`pfSense-pkg-RESTAPI`, Version 2): Gateways, die down sind, Pakete
verlieren oder langsam antworten, aktivierte Schnittstellen ohne Link,
aktivierte Dienste, die gestoppt sind, und CPU, Speicher, Platte und
Temperatur.

DumbMonit sendet einen API-Key im Header `X-API-Key`. Ein Key trägt
genau die Rechte des pfSense-Benutzers, zu dem er gehört: Der Benutzer
`dumbmonit` unten hält nur die `GET`-Rechte der fünf von DumbMonit
gelesenen Seiten, daher kann der Key diese Seiten lesen und nichts
ändern. Jede Minute liest DumbMonit parallel
`/api/v2/status/system`, `/api/v2/status/gateways`,
`/api/v2/status/interfaces`, `/api/v2/status/services` und
`/api/v2/system/version`, und, falls das Recht gegeben wurde,
`/api/v2/system/restapi/version`.

!!! warning "Nur gegen die Herstellerdokumentation validiert — noch nicht an einem echten System getestet"

    Diese Integration wurde aus der veröffentlichten Dokumentation des
    REST-API-Pakets und dem Quellcode seiner Modelle gebaut (Feldnamen,
    Einheiten, der Antwort-Umschlag und seine Fehlerkennungen), nicht
    gegen ein laufendes pfSense. Jeder Aufruf ist ein Lesevorgang.
    Sagen Sie uns, was kaputt ist.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_pfsense_`.

| Metrik | Labels | Bedeutung |
|---|---|---|
| `cpu_usage_percent`, `memory_used_percent`, `swap_used_percent`, `disk_used_percent`, `mbuf_used_percent` | | Wie das Dashboard sie zeigt, in Prozent. Die CPU-Zahl ist pfSenses eigene Schätzung aus dem Load Average. |
| `temperature_celsius` | | Wenn pfSense einen Sensor lesen kann. |
| `load_average` | `period`: `1m`, `5m`, `15m` | Load Average. |
| `cpu_count` | | Prozessorkerne. |
| `version_info` | `version` | Wert 1: die pfSense-Version, `2.7.2-RELEASE`. |
| `gateway_up` | `gateway` | 0, wenn pfSense das Gateway als down markiert, sonst 1. |
| `gateway_status_info` | `gateway`, `status` | Wert 1: pfSenses eigenes Wort (`none`, wenn alles gut ist, `down`, `loss`, `delay`). |
| `gateway_loss_percent` | `gateway` | Paketverlust zur Monitor-Adresse. |
| `gateway_delay_milliseconds`, `gateway_stddev_milliseconds` | `gateway` | Round Trip zur Monitor-Adresse und seine Abweichung; fehlt, während das Gateway down ist. |
| `gateways`, `gateways_down` | | Zählungen. |
| `interface_enabled`, `interface_up` | `interface`, `descr` | In der Konfiguration aktiviert; Link up. |
| `interface_down` | `interface`, `descr` | 1, wenn die Schnittstelle aktiviert ist und keinen Link hat. Eine deaktivierte Schnittstelle ist nie down. |
| `interface_bytes_in_total`, `interface_bytes_out_total`, `interface_errors_in_total`, `interface_errors_out_total` | `interface`, `descr` | Zähler. |
| `interfaces_down` | | Zählung. |
| `service_running`, `service_enabled` | `service`, `description` | Wie Status > Services sie zeigt. |
| `service_stopped` | `service`, `description` | 1, wenn der Dienst aktiviert ist und nicht läuft. |
| `services_stopped` | | Zählung. |
| `restapi_update_available` | | Mit dem Recht `/api/v2/system/restapi/version`: 1, wenn das REST-API-Paket eine neuere Version hat. |
| `restapi_version_info` | `version`, `latest` | Wert 1: installierte und neueste Version des Pakets. |

pfSense sagt über diese API nicht, ob ein pfSense-Update verfügbar ist:
Die einzige Route, die Systemupdates betrifft
(`/api/v2/system/update`), startet eines. DumbMonit meldet nur die
eigenen Updates des Pakets.

Die geltenden [eingebauten Regeln](../alerting/rules.md#pfsense):

- **pfSense gateway down**: ein Gateway ist seit drei Minuten als down
  markiert (Warning).
- **pfSense gateway losing packets**: mehr als 10 % Verlust seit zehn
  Minuten, bei einem Gateway, das nicht schon down ist (Advisory).
- **pfSense gateway slow**: Round Trip über 500 ms seit fünfzehn
  Minuten (Advisory). Für eine Satellitenverbindung anheben.
- **pfSense interface without link**: eine aktivierte Schnittstelle
  ohne Link seit fünf Minuten (Advisory).
- **pfSense service stopped**: ein aktivierter, nicht laufender Dienst
  seit fünf Minuten (Advisory).
- **pfSense disk nearly full**: mehr als 90 % seit dreißig Minuten
  (Advisory).

## Die Monitorseite

Das Firewall-Panel sagt in einem Satz, was nicht stimmt, zeigt CPU,
Speicher, Platte und Temperatur, listet dann jedes Gateway mit seiner
Latenz und seinem Verlust, die Schnittstellen ohne Link, die
gestoppten Dienste, und ein verfügbares Update des REST-API-Pakets.

## Einen nur-lesenden REST-API-Key für DumbMonit erstellen

1. Installieren Sie das Community-REST-API-Paket
   (pfSense-pkg-RESTAPI, Version 2) über Diagnostics > Command Prompt
   oder eine SSH-Shell, mit der zu Ihrer pfSense-Version passenden
   Datei von der Releases-Seite des Projekts (unten 2.7.2). Kreuzen
   Sie dann unter System > REST API > Settings beide BasicAuth und Key
   unter den Authentifizierungsmethoden an.

    ```
    pkg-static add https://github.com/pfrest/pfSense-pkg-RESTAPI/releases/latest/download/pfSense-2.7.2-pkg-RESTAPI.pkg
    ```

2. Fügen Sie unter System > User Manager einen Benutzer namens
   dumbmonit mit einem langen Zufallspasswort hinzu. Fügen Sie unter
   Effective Privileges nur diese REST-API-Rechte hinzu:
   /api/v2/status/system GET, /api/v2/status/gateways GET,
   /api/v2/status/interfaces GET, /api/v2/status/services GET,
   /api/v2/system/version GET, plus /api/v2/system/restapi/version GET,
   um von den eigenen Updates des Pakets zu hören. Nur für den
   nächsten Schritt fügen Sie außerdem /api/v2/auth/key POST hinzu.

3. Erstellen Sie den Key als dumbmonit, von jeder Maschine aus, die
   pfSense erreicht: Der Key trägt genau die Rechte dieses Benutzers.
   Kopieren Sie den Key-Wert der Antwort, entfernen Sie dann das Recht
   /api/v2/auth/key POST wieder.

    ```
    curl -k -u dumbmonit -X POST -H 'Content-Type: application/json' -d '{"descr":"DumbMonit"}' https://pfsense.lan/api/v2/auth/key
    ```

4. Geben Sie in DumbMonit die Adresse von pfSense ein, zum Beispiel
   "pfsense.lan", und fügen Sie den Key als API-Key ein. pfSense
   liefert ein selbstsigniertes Zertifikat: Kreuzen Sie "Accept an
   unverifiable certificate" an, sofern Sie es nicht ersetzt haben.

!!! warning
    Die REST-API sagt nicht, ob ein pfSense-Update verfügbar ist, nur
    ob das Paket selbst eines hat. Das Paket ist ein Community-Projekt,
    nicht Teil von pfSense: Installieren Sie es nach jedem
    pfSense-Upgrade neu.

## Credentials

| Credential | Felder |
|---|---|
| REST-API-Key | Der in Schritt 3 für `dumbmonit` erstellte Key, als `X-API-Key` gesendet. |

Adresse: der Hostname oder die IP von pfSense, `host:port`, oder eine
URL. Optionen:

- **Protocol** (`https`) und **Port** (443): die des webConfigurator,
  gesetzt unter System > Advanced > Admin Access.
- **Accept an unverifiable certificate**: pfSenses eigenes Zertifikat
  ist selbstsigniert.
- **Timeout pro Request** (15 s).

## Fehlerbehebung

"pfSense refused the API key (401)": der Key ist falsch, oder Key ist
unter System > REST API > Settings nicht unter den
Authentifizierungsmethoden angekreuzt.

"pfSense accepted the API key but refused … (403)": dem Benutzer
`dumbmonit` fehlt das Recht dieser Seite; die Meldung benennt es.

"… not found: is the REST API package installed?": das Paket fehlt,
typischerweise nach einem pfSense-Upgrade, das es entfernt.
Installieren Sie es wie in Schritt 1 neu.
