# Nginx Proxy Manager

Der Proxy-Manager: Hosts, deren Konfiguration nginx abgelehnt hat,
deaktivierte Hosts, und die von den Hosts präsentierten Zertifikate,
bevor ein fehlgeschlagenes Let's-Encrypt-Renewal zu einer abgelaufenen
Site wird.

DumbMonit loggt sich mit einem eigenen Benutzer in die REST-API von
Nginx Proxy Manager ein (`/api`, auf dem Port seiner Web-Oberfläche, 81)
und liest dann die Proxy-Hosts, Redirection-Hosts, 404-Hosts (in der API
Dead Hosts genannt) und Streams. Jeder Host sagt, ob er aktiviert ist
und ob nginx seine Konfiguration akzeptiert hat. Ein von nginx
abgelehnter Host wird nicht bedient, und die Web-Oberfläche zeigt ihn
nur als roten Punkt.

Zertifikate werden nicht aus der API gelesen. `/api/nginx/certificates`
gibt den privaten Schlüssel importierter Zertifikate und die
Zugangsdaten der DNS-Challenge an jeden Benutzer weiter, der Zertifikate
ansehen darf. DumbMonit liest jedes Zertifikat stattdessen dort, wo es
ausgeliefert wird: ein TLS-Handshake auf Port 443 von Nginx Proxy
Manager, mit dem Namen eines Hosts, der es nutzt. Das misst auch das
Zertifikat, das Besucher tatsächlich bekommen.

## Was beobachtet wird

Alle Metriken haben das Präfix `dumbmonit_npm_`.

| Metrik | Was | Labels |
|---|---|---|
| `version_info` | Wert 1 | `version` |
| `hosts`, `hosts_enabled`, `hosts_disabled` | Hosts jeder Art | `type`: `proxy`, `redirection`, `dead` (404-Hosts), `stream` |
| `hosts_offline` | aktivierte Hosts, deren Konfiguration nginx abgelehnt hat | `type` |
| `host_online` | pro aktiviertem Host: 0, wenn nginx seine Konfiguration abgelehnt hat | `type`, `host` |
| `host_error_info` | Wert 1, für einen von nginx abgelehnten Host: die erste Zeile von nginx' Fehler | `type`, `host`, `error` |
| `certificates_checked`, `certificates_read` | an einen aktivierten Host angehängte Zertifikate, und die auf dem HTTPS-Port gelesenen | |
| `cert_expiry_days` | verbleibende Tage, bis das von einem Host präsentierte Zertifikat abläuft | `host` |

`host` ist der erste Domainname des Hosts, oder `port 9000` für einen
Stream. Deaktivierte Hosts werden gezählt, nicht beschrieben: Einen Host
zu deaktivieren ist absichtlich, und sein Alarm, falls vorhanden, löst
sich. Hosts werden einzeln bis zu 500 beschrieben, und höchstens 100
Zertifikate werden pro Prüfung gelesen.

Die geltenden [eingebauten Regeln](../alerting/rules.md#reverse-proxies-and-domains):
Nginx Proxy Manager host offline, Proxy certificate not renewed, plus
Device unreachable.

## Die Geräteseite

Das Panel über den Diagrammen liest, was die Abfrage gespeichert hat; das
Öffnen der Seite fragt Nginx Proxy Manager nie ab. Es sagt in einem Satz,
was nicht stimmt (von nginx abgelehnte Hosts, mit Grund, kurz vor dem
Ablauf stehende Zertifikate, nicht lesbare Zertifikate), dann die
Proxy-Hosts, Weiterleitungen, 404-Hosts, Streams und deaktivierte Hosts,
dann die Zertifikate, die am ehesten ablaufen, zuerst.

## Einen nur-lesenden Nginx-Proxy-Manager-Benutzer erstellen

1. Öffnen Sie in Nginx Proxy Manager Users und fügen Sie einen Benutzer
   für DumbMonit hinzu, mit einer eigenen E-Mail-Adresse wie
   dumbmonit@example.com. Lassen Sie jede Rolle unangekreuzt.

2. Öffnen Sie im Menü dieses Benutzers Permissions: Item Visibility All
   Items; Proxy Hosts, Redirection Hosts, 404 Hosts und Streams View
   Only; Access Lists und SSL Certificates Hidden. Setzen Sie dann sein
   Passwort mit Change Password im selben Menü.

3. SSL Certificates bleibt absichtlich auf Hidden: Nginx Proxy Manager
   gibt den privaten Schlüssel importierter Zertifikate und die
   Zugangsdaten der DNS-Challenge an jeden Benutzer weiter, der sie
   ansehen darf. DumbMonit liest jedes Zertifikat stattdessen dort, wo
   es ausgeliefert wird, mit einem TLS-Handshake auf Port 443 für jeden
   Host, der eines hat.

4. Geben Sie in DumbMonit den Nginx-Proxy-Manager-Host mit dem Port
   seiner Web-Oberfläche ein, zum Beispiel "npm.lan:81", und E-Mail und
   Passwort dieses Benutzers. Werden die Hosts über HTTPS auf einem
   anderen Port als 443 bedient, setzen Sie unten HTTPS port.

!!! warning
    Ein Host, dessen Konfiguration nginx ablehnt (ein Tippfehler unter
    Advanced, eine fehlende Zertifikatsdatei), wird nicht bedient, und
    Nginx Proxy Manager zeigt ihn nur als roten Punkt in der Liste.
    Let's-Encrypt-Zertifikate werden dreißig Tage im Voraus erneuert:
    eines, das in weniger als vierzehn Tagen abläuft, ist eines, dessen
    Erneuerung fehlschlug, und DumbMonit warnt dann.

Item Visibility All Items ist wichtig: Mit Created Items sieht der
Benutzer nur die Hosts, die er selbst erstellt hat, also keine. Ein
Benutzer ohne Proxy Hosts View Only wird abgelehnt; einer ohne Streams,
Redirection Hosts oder 404 Hosts meldet diese einfach nicht.

DumbMonit behält das beim Login von Nginx Proxy Manager erhaltene Token
(einen Tag gültig) und loggt sich erneut ein, wenn es abläuft, statt bei
jeder Prüfung.

## Credentials

| Credential | Felder |
|---|---|
| Benutzer-E-Mail und Passwort | E-Mail-Adresse und Passwort des oben erstellten Benutzers. |

Adresse: ein Hostname oder eine IP mit dem Port der Web-Oberfläche
(`npm.lan:81`), oder eine vollständige URL. Optionen: Protokoll
(standardmäßig HTTP), Port der Web-Oberfläche (81), Check the
certificates (ein), HTTPS-Port (443), Zertifikatsprüfung der API und
Anfrage-Timeout (10 s).

Die Zertifikatsprüfung verbindet sich mit dem Host-Teil der Adresse:
Wird die Web-Oberfläche über einen anderen Proxy erreicht, richten Sie
die Adresse auf Nginx Proxy Manager selbst, oder entfernen Sie das
Häkchen bei Check the certificates und beobachten Sie jede Site
stattdessen mit einem [TLS-Zertifikats-Check](services.md). Wildcard-
Zertifikate werden über den ersten sie nutzenden Host gelesen.
