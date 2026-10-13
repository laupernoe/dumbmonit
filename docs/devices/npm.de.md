# Nginx Proxy Manager {#nginx-proxy-manager}

Der Proxy-Manager: Hosts, deren Konfiguration nginx abgelehnt hat, deaktivierte Hosts und die
Zertifikate, die die Hosts präsentieren, bevor ein fehlgeschlagener Let's-Encrypt-Erneuerungs-
versuch zu einer abgelaufenen Website wird.

DumbMonit meldet sich mit einem eigenen Benutzer an der REST-API von Nginx Proxy Manager an
(`/api`, auf dem Port seiner Weboberfläche, 81) und liest dann die Proxy-Hosts, Weiterleitungs-
Hosts, 404-Hosts (in der API „dead hosts“ genannt) und Streams. Jeder Host sagt, ob er
aktiviert ist und ob nginx seine Konfiguration akzeptiert hat. Ein Host, den nginx abgelehnt
hat, wird nicht ausgeliefert, und die Weboberfläche zeigt ihn nur als roten Punkt.

Zertifikate werden nicht aus der API gelesen. `/api/nginx/certificates` gibt den privaten
Schlüssel importierter Zertifikate und die Zugangsdaten der DNS-Challenge an jeden Benutzer
heraus, der Zertifikate ansehen darf. DumbMonit liest stattdessen jedes Zertifikat dort, wo es
ausgeliefert wird: ein TLS-Handshake auf Port 443 von Nginx Proxy Manager, mit dem Namen eines
Hosts, der es verwendet. Das misst zugleich das Zertifikat, das Besucher tatsächlich erhalten.

## Was überwacht wird {#what-it-watches}

Alle Metriken haben das Präfix `dumbmonit_npm_`.

| Metrik | Was | Labels |
|---|---|---|
| `version_info` | Wert 1 | `version` |
| `hosts`, `hosts_enabled`, `hosts_disabled` | Hosts jedes Typs | `type`: `proxy`, `redirection`, `dead` (404-Hosts), `stream` |
| `hosts_offline` | aktivierte Hosts, deren Konfiguration nginx abgelehnt hat | `type` |
| `host_online` | pro aktiviertem Host: 0, wenn nginx seine Konfiguration abgelehnt hat | `type`, `proxy_host` |
| `host_error_info` | Wert 1, für einen von nginx abgelehnten Host: die erste Zeile des nginx-Fehlers | `type`, `proxy_host`, `error` |
| `certificates_checked`, `certificates_read` | an einen aktivierten Host gebundene Zertifikate und die auf dem HTTPS-Port gelesenen | |
| `cert_expiry_days` | verbleibende Tage bis zum Ablauf des Zertifikats, das ein Host präsentiert | `proxy_host` |

`proxy_host` ist der erste Domainname des Hosts oder `port 9000` bei einem Stream. Deaktivierte
Hosts werden gezählt, nicht beschrieben: Einen Host zu deaktivieren ist Absicht, und sein
Alarm, falls vorhanden, löst sich auf. Hosts werden bis zu 500 einzeln beschrieben, und pro
Prüfung werden höchstens 100 Zertifikate gelesen.

Die zutreffenden [integrierten Regeln](../alerting/rules.md#reverse-proxies-and-domains):
Nginx-Proxy-Manager-Host offline, Proxy-Zertifikat nicht erneuert, außerdem Gerät nicht
erreichbar.

## Die Geräteseite {#the-device-page}

Das Panel über den Diagrammen liest, was die Prüfung gespeichert hat; das Öffnen der Seite
fragt Nginx Proxy Manager nie ab. Es nennt in einem Satz, was nicht stimmt (von nginx
abgelehnte Hosts mit Begründung, bald ablaufende Zertifikate, nicht lesbare Zertifikate), dann
die Proxy-Hosts, Weiterleitungen, 404-Hosts, Streams und deaktivierten Hosts, dann die
Zertifikate, das am frühesten ablaufende zuerst.

## Einen Nur-Lese-Benutzer in Nginx Proxy Manager erstellen {#create-a-view-only-nginx-proxy-manager-user}

1. Öffnen Sie in Nginx Proxy Manager Benutzer und fügen Sie einen Benutzer für DumbMonit mit eigener E-Mail-Adresse hinzu, etwa dumbmonit@example.com. Lassen Sie alle Rollen abgehakt aus.

2. Öffnen Sie im Menü dieses Benutzers Berechtigungen: Elementsichtbarkeit Alle Elemente; Proxy-Hosts, Weiterleitungs-Hosts, 404-Hosts und Streams Nur ansehen; Zugriffslisten und SSL-Zertifikate Ausgeblendet. Setzen Sie dann sein Passwort mit Passwort ändern im selben Menü.

3. SSL-Zertifikate bleiben absichtlich ausgeblendet: Nginx Proxy Manager gibt den privaten Schlüssel importierter Zertifikate und die Zugangsdaten der DNS-Challenge an jeden Benutzer heraus, der sie ansehen darf. DumbMonit liest stattdessen jedes Zertifikat dort, wo es ausgeliefert wird, mit einem TLS-Handshake auf Port 443 für jeden Host, der eines hat.

4. Geben Sie in DumbMonit den Nginx-Proxy-Manager-Host mit dem Port seiner Weboberfläche ein, zum Beispiel „npm.lan:81“, sowie E-Mail und Passwort dieses Benutzers. Werden die Hosts über HTTPS auf einem anderen Port als 443 ausgeliefert, setzen Sie unten HTTPS-Port.

!!! warning
    Ein Host, dessen Konfiguration nginx ablehnt (ein Tippfehler in Erweitert, eine fehlende Zertifikatsdatei), wird nicht ausgeliefert, und Nginx Proxy Manager zeigt ihn in der Liste nur als roten Punkt. Let's-Encrypt-Zertifikate werden dreißig Tage im Voraus erneuert: Eines, das in weniger als vierzehn Tagen abläuft, ist eines, dessen Erneuerung fehlgeschlagen ist, und DumbMonit warnt dann.

Elementsichtbarkeit Alle Elemente ist wichtig: Mit Erstellte Elemente sieht der Benutzer nur die
Hosts, die er selbst erstellt hat, also keine. Ein Benutzer ohne Proxy-Hosts Nur ansehen wird
abgewiesen; einer ohne Streams, Weiterleitungs-Hosts oder 404-Hosts meldet diese schlicht nicht.

DumbMonit behält das Token, das Nginx Proxy Manager bei der Anmeldung ausgibt (einen Tag gültig),
und meldet sich bei Ablauf erneut an, statt bei jeder Prüfung.

## Zugangsdaten {#credentials}

| Zugangsdaten | Felder |
|---|---|
| E-Mail und Passwort des Benutzers | E-Mail-Adresse und Passwort des oben erstellten Benutzers. |

Adresse: ein Hostname oder eine IP mit dem Port der Weboberfläche (`npm.lan:81`) oder eine
vollständige URL. Optionen: Protokoll (standardmäßig HTTP), Port der Weboberfläche (81),
Zertifikate prüfen (an), HTTPS-Port (443), Zertifikatsprüfung der API und Anfrage-Timeout
(10 s).

Die Zertifikatsprüfung verbindet sich mit dem Host-Teil der Adresse: Wird die Weboberfläche über
einen weiteren Proxy erreicht, richten Sie die Adresse auf Nginx Proxy Manager selbst, oder
haken Sie Zertifikate prüfen ab und überwachen Sie jede Website stattdessen mit einer
[TLS-Zertifikatsprüfung](services.md). Wildcard-Zertifikate werden über den ersten Host
gelesen, der sie verwendet.
