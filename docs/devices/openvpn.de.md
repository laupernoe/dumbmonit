# OpenVPN-Server {#openvpn-server}

Der Serverprozess von OpenVPN stellt einen eigenen Steuerkanal bereit, die
**Management-Schnittstelle**: ein lokaler TCP-Listener, getrennt vom VPN
selbst und gedacht für ein Überwachungswerkzeug statt für einen VPN-Client.
DumbMonit liest ihn wie jeder Management-Client: die Version des Servers, ob
er sich als verbunden meldet, und jeden Client, der gerade in einer Sitzung
ist, mit seiner realen und virtuellen Adresse und seinem Verkehr. Es wird
nie etwas gesendet, das den Zustand des Servers ändern oder einen Client
trennen würde.

Clients sind von Natur aus flüchtig — ein Laptop, der in den Ruhezustand
geht, verschwindet einfach aus der nächsten Messung. DumbMonit versucht nicht,
sich zu merken, wer vor fünf Minuten verbunden war; es meldet nur, wer jetzt
verbunden ist, dazu die Anzahl der Clients als Metrik, die sich über die Zeit
darzustellen lohnt.

## Was beobachtet wird {#what-it-watches}

Alle Metriken tragen das Präfix `dumbmonit_openvpn_`.

| Metrik | Was | Labels |
|---|---|---|
| `server_info` | Wert 1 | `version` |
| `server_up` | 1, wenn die Management-Schnittstelle den Server als `CONNECTED` meldet, sonst 0 | `state` |
| `server_state_age_seconds` | Zeit seit dem Eintritt des Servers in seinen aktuellen Zustand | |
| `clients_connected` | Anzahl der Clients, die gerade in einer Sitzung sind | |
| `client_bytes_received`, `client_bytes_sent` | Verkehr eines Clients (Zähler) | `common_name`, `real_address` |
| `client_connected_seconds` | Zeit seit dem Beginn der Sitzung dieses Clients | `common_name`, `real_address` |

`real_address` trägt nie den Quellport des Clients: Er ändert sich bei jeder
Wiederverbindung hinter NAT und würde die Serien grundlos vervielfachen.
Höchstens 256 Clients erhalten je eine detaillierte Serie; `clients_connected`
bleibt darüber hinaus exakt.

Integrierte Regeln, die gelten: **Monitor nicht erreichbar**, wenn die
Management-Verbindung selbst nicht hergestellt werden kann oder das Passwort
abgelehnt wird.

## Die Monitorseite {#the-device-page}

Das Panel über den Diagrammen zeigt, was die Sonde gespeichert hat; das
Öffnen der Seite verbindet sich nie mit der Management-Schnittstelle. Es zeigt
die Version des Servers und ob er sich als verbunden meldet, die Anzahl der
Clients und eine Liste der Clients, die gerade in einer Sitzung sind, mit
ihrer Adresse und ihrem Verkehr.

## Die Management-Schnittstelle aktivieren {#turn-on-the-management-interface}

1. Die Management-Schnittstelle ist ein eigener Listener und standardmäßig
   aus: Fügen Sie dies der Konfiguration des Servers hinzu (server.conf, bei
   OpenVPN Access Server die as.conf unter einem anderen Namen) und starten
   Sie ihn neu. Binden Sie ihn an eine Adresse, die der DumbMonit-Host
   erreichen kann, oder an 127.0.0.1 und erreichen Sie ihn über einen
   SSH-Tunnel oder einen Relay-Agenten auf demselben Rechner.

    ```
    management 0.0.0.0 7505
    ```

2. Optional, aber empfohlen: Die Management-Schnittstelle hat kein eigenes
   Konto, nur ein gemeinsames Passwort. Legen Sie eines in eine Datei, die
   nur der OpenVPN-Prozess lesen kann, und fügen Sie sie als drittes Argument
   hinzu.

    ```
    management 0.0.0.0 7505 /etc/openvpn/management-password
    ```

3. Geben Sie in DumbMonit die Adresse des Servers ein, zum Beispiel
   "vpn.lan" (Port 7505), und das Management-Passwort, falls eines gesetzt
   ist.

4. DumbMonit liest nur: Es sendet version, state und status 3, nie signal,
   kill oder client-kill. Es kann keinen Client trennen, keine Einstellung
   ändern und den Server nicht stoppen.

!!! warning
    Wer den Management-Port erreicht und das Passwort kennt, kann Clients
    trennen, alle ihre realen IP-Adressen lesen und den laufenden Server neu
    konfigurieren: Halten Sie ihn aus dem öffentlichen Netz heraus, hinter
    einer Firewall-Regel, die nur den DumbMonit-Host zulässt, und mit
    gesetztem Passwort.

## Zugangsdaten {#credentials}

| Credential | Felder |
|---|---|
| Kein Passwort | Nur für eine Management-Schnittstelle, die ohne Passwortdatei-Argument gestartet wurde. |
| Management-Passwort | Das gemeinsame Passwort, das durch das dritte Argument der Direktive "management" des Servers gesetzt wurde. |

Adresse: ein Hostname oder eine IP, optional mit `:port`; der Standardport
ist 7505, eine Konvention aus der eigenen Dokumentation von OpenVPN, kein
Standard, den der Server erzwingt. Optionen: Port (7505) und Timeout pro
Austausch (10 s).

Eine abgelehnte Verbindung oder keine Antwort zählt als Server nicht
erreichbar. Ein falsches Passwort wird als Credential-Fehler angezeigt;
keines von beiden weckt nachts jemanden. Die Verbindung ist reines TCP:
Nichts, was gelesen wird, ist geheim, aber das Passwort, falls eines gesetzt
ist, geht bei jeder Prüfung im Klartext über das Netz — erreichen Sie die
Schnittstelle nur aus einem vertrauenswürdigen Netz oder über einen
Relay-Agenten auf demselben Rechner wie der Server.
