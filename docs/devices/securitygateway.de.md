# SecurityGateway for Email Servers

Die Dienste eines SecurityGateway-Mail-Gateways, von außen geprüft, und
auf SecurityGateway 12.5 oder später die Version und die
Leistungszähler, die seine REST-API veröffentlicht: Zustellungs-
warteschlange, Quarantäne, SMTP-Sitzungen.

!!! warning "Nur gegen die Dokumentation validiert"

    Diese Integration wurde aus MDaemon Technologies' veröffentlichter
    Dokumentation von SecurityGateway und seiner REST-API gebaut, nicht
    gegen ein echtes Gateway. Jeder Aufruf ist ein Lesevorgang. Die
    Dokumentation veröffentlicht nicht den Pfad oder die Namen der
    Leistungszähler, daher findet DumbMonit den Pfad in der eigenen
    Beschreibung der API und behält jeden Zähler unter dem von der API
    gegebenen Namen. Noch niemand hat es gegen ein laufendes Gateway
    getestet. Sagen Sie uns, was kaputt ist.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_securitygateway_`.

| Metrik | Labels | Bedeutung |
|---|---|---|
| `service_up` | `service`, `port` | 1, wenn der Port die Verbindung angenommen hat und, für SMTP, mit `220` grüßte. 0, wenn er nicht antwortete, oder antwortete, um den Dienst zu verweigern (`421`, `554`). |
| `service_response_seconds` | `service`, `port` | Zeit bis zur Begrüßung, oder bis zur Verbindung bei einem Port ohne eine. |
| `api_up` | | Nur mit einem API-Key: 1, wenn die REST-API antwortete, 0, wenn nicht. |
| `api_response_seconds` | | Zeit, die die REST-API zum Antworten brauchte. |
| `counters_available` | | 1, wenn die Leistungszähler gelesen wurden; 0, wenn die API keine beschreibt, oder das Konto des Keys sie nicht lesen darf. |
| `counter` | `counter` | Eine Zeitreihe pro numerischem Wert der Leistungszähler, benannt wie die API sie benennt, in `snake_case`: `deliveryQueue` wird `delivery_queue`. Höchstens 64. |
| `info` (Wert 1) | `version`, `source` | Die von der Beschreibung der API gegebene Version (`source="openapi"`), oder sonst von der SMTP-Begrüßung, wenn das Gateway sie dort ankündigt (`source="smtp_banner"`). |
| `scrape_duration_seconds` | | Zeit, die die gesamte Prüfung brauchte. |

**Wie die Zähler gefunden werden.** SecurityGateways REST-API
beschreibt sich selbst unter `/api/v1/openapi`. DumbMonit liest diese
Beschreibung, nimmt den kürzesten nur-lesenden Pfad ohne Parameter,
dessen Name Zähler erwähnt, liest ihn, und behält jede Zahl in seiner
Antwort. Nichts wird umbenannt und nichts wird angenommen: Wenn eine
zukünftige Version einen Zähler umbenennt, folgt die Zeitreihe dem
neuen Namen. Die in SecurityGateways Hilfe aufgeführten Zähler sind die
aktiven eingehenden und ausgehenden SMTP-Sitzungen, die
Zustellungswarteschlange, die administrative Quarantäne, die Uptime,
die Domain- und Benutzerzahlen und der Datenbank-Connection-Pool;
manche davon ändern sich nur einmal pro Minute.

**Ein teilweiser Ausfall ist weiterhin eine erfolgreiche Prüfung.**
Ein gestoppter Dienst gibt für diesen Dienst `service_up 0`, und die
anderen werden weiterhin gemessen. Nur wenn kein Dienst und keine API
überhaupt antwortet, ist das Gerät *unreachable*. Ein abgelehnter Key
ist ein Konfigurationsfehler, auf der Geräteseite angezeigt und nicht
benachrichtigt.

**Was nicht gelesen wird.** Die Lizenz und ihr Ablauf, der
Plattenplatz und die Nachrichtenstatistiken des Dashboards sind von
der veröffentlichten Dokumentation der REST-API nicht beschrieben. Um
den SecurityGateway-Windows-Dienst selbst und die Platten des Servers
zu beobachten, installieren Sie den [Agent](agent.md) auf dem Server
und listen Sie den Dienst in seiner `services`-Einstellung.

Die geltenden [eingebauten Regeln](../alerting/rules.md#mdaemon-and-securitygateway):
SecurityGateway service down, SecurityGateway API not answering,
SecurityGateway delivery queue growing, plus Device unreachable.

## Die Geräteseite

Ein SecurityGateway-Gerät zeigt ein **Mail services**-Panel über den
Diagrammen, aus der letzten gespeicherten Messung gelesen: Das Öffnen
der Seite verbindet sich nie mit dem Gateway. Es listet jeden
beobachteten Dienst mit seinem Port, einem Wort und einer Farbe
(Answering, Down) und seiner Antwortzeit, die Version, und mit einem
API-Key jeden Leistungszähler mit seinem neuesten Wert.

## Die Gateway-Dienste beobachten, und optional die REST-API

1. Nichts auf dem Server zu installieren. Ohne Credential verbindet
   sich DumbMonit von außen mit SMTP (25) und der Web-Oberfläche
   (4000) und liest die SMTP-Begrüßung. Ändern Sie die Liste in der
   Option Services, zum Beispiel um HTTPS statt HTTP zu prüfen.

    ```
    smtp,web_https
    ```

2. Um auch die Leistungszähler zu lesen, wird SecurityGateway 12.5
   oder später benötigt. Erstellen Sie ein wie folgt benanntes
   dediziertes Konto, mit einem nirgendwo sonst genutzten langen
   Passwort, und geben Sie ihm die Rolle Domain Administrator. Bleiben
   die Zähler auf der Geräteseite leer, hält Ihre Version sie für die
   Rolle Global Administrator vor, und das Konto braucht diese Rolle
   stattdessen.

    ```
    dumbmonit
    ```

3. Loggen Sie sich als dieses Konto ein und erstellen Sie einen Key
   unter Setup/Users → Accounts → API Keys, mit einem Ablaufdatum. Ein
   Key trägt die Rechte des Kontos, das ihn erstellt, deshalb muss er
   von diesem Konto erstellt werden und nicht von Ihrem eigenen.

4. Beschränken Sie das Konto auf die Adresse, von der aus DumbMonit
   sich verbindet, in seinen IP-Einschränkungen: API-Keys gehorchen
   diesen auch.

5. Prüfen Sie vom DumbMonit-Host aus, dass der Key funktioniert. Der
   Befehl gibt die API-Beschreibung aus, die auch die von DumbMonit
   gelesenen Zähler listet.

    ```
    curl -k -H 'Authorization: Bearer YOUR_KEY' https://sg.example.com:4443/api/v1/openapi
    ```

6. Geben Sie in DumbMonit die Gateway-Adresse ein, zum Beispiel
   "sg.example.com", und fügen Sie den Key ein. Die API wird auf dem
   HTTPS-Port der Web-Oberfläche erreicht, standardmäßig 4443, unter
   /api/v1.

!!! warning
    SecurityGateway hat keine nur-lesende Rolle und keinen engeren Key:
    Der Key kann alles tun, was sein Konto kann. Erstellen Sie ihn nie
    von Ihrem eigenen Konto, behalten Sie ihn nur in DumbMonit und
    geben Sie ihm ein Ablaufdatum. Versionen vor 12.5 haben keine
    REST-API: Lassen Sie das Credential leer, und DumbMonit prüft nur
    die Dienste. Die Web-Oberfläche nutzt auf 4443 oft ein
    selbstsigniertes Zertifikat: Wird die Verbindung deshalb
    abgelehnt, kreuzen Sie "Accept an unverifiable certificate" in den
    Optionen an.

## Credentials

| Credential | Felder |
|---|---|
| Nur Dienste | Nichts: Die Ports werden ohne Login geprüft. |
| API-Key (12.5 und später) | Der Key, als `Authorization: Bearer …` gesendet. |

Adresse: der Hostname oder die IP des Gateways, zum Beispiel
`sg.example.com`. Ein in der Adresse geschriebener Port
(`sg.example.com:8443`) wird als API-Port genommen.

## Optionen

| Option | Standard | Wirkung |
|---|---|---|
| `services` | `smtp,web` | Was geprüft wird, kommagetrennt: `smtp` (25), `smtps` (465), `web` (4000), `web_https` (4443). `name:port` ändert einen Port; jeder andere Name mit seinem Port fügt einen eigenen Dienst hinzu, nur per Verbindung geprüft. `none` prüft keinen Port. |
| `request_timeout_seconds` | `10` | Erlaubte Zeit für jede Verbindung, Begrüßung und API-Aufruf, von 1 bis 60. |
| `api_port` | `4443` | Der Port der Web-Oberfläche, der `/api/v1` bedient. Nur mit einem API-Key genutzt. |
| `api_tls` | `true` | HTTPS zur REST-API. Nur für die Web-Oberfläche über einfaches HTTP (standardmäßig Port 4000) entfernen: Der Key durchläuft dann das Netzwerk im Klartext. |
| `insecure_tls` | `false` | Ein selbstsigniertes Zertifikat auf der REST-API akzeptieren. |
| `counters` | `true` | Die Leistungszähler lesen. |
