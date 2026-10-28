# Einen entfernten Standort überwachen {#monitor-a-remote-site}

Ein DumbMonit, mehrere Netzwerke. Ein am entfernten Standort installierter **Relay-Agent**
führt im Auftrag des Servers die Prüfungen der ihm zugewiesenen Monitore aus und meldet die
Ergebnisse über dieselbe ausgehende Verbindung zurück, über die er bereits seine eigenen
Metriken sendet. Am entfernten Standort muss nichts geöffnet werden, kein VPN, keine zweite
Instanz, die man im Auge behalten müsste.

```
 head office                              remote site (NAT, firewall)
 ┌──────────────────┐   HTTPS, outbound   ┌──────────────────────────────┐
 │ DumbMonit server │◀────────────────────│ relay agent                  │
 │  scheduler       │  probes to run  ──▶ │  runs SNMP / Proxmox / HTTP… │──▶ switch, NAS,
 │  alerting, UI    │◀── samples, verdict │  pushes its own metrics too  │    hypervisor…
 └──────────────────┘                     └──────────────────────────────┘
```

## 1. Den Server über HTTPS veröffentlichen {#1-publish-the-server-over-https}

Das Relay holt seine Aufträge vom Server ab: Der Server muss vom entfernten Standort aus
erreichbar sein, typischerweise `https://monitor.example.org` hinter einem Reverse-Proxy. Zwei
Dinge laufen über diese Verbindung:

- das **Enrollment-Token** (`dmon_…`) im `Authorization`-Header jeder Anfrage, wie bei jedem
  Agenten;
- die **Zugangsdaten der Monitore**, die das Relay prüft (SNMP-Community, Proxmox-API-Token …),
  entschlüsselt, in jedem Prüfauftrag.

Über unverschlüsseltes HTTP durch das Internet wären beide unterwegs lesbar: Stellen Sie den
Server hinter HTTPS, bevor Sie etwas außerhalb Ihres eigenen Netzwerks weiterleiten. In einem
LAN oder VPN ist HTTP akzeptabel, und das Token ist das einzige Geheimnis im Transit, bis Sie
dem Relay Monitore zuweisen. Zugangsdaten werden auf keiner Seite auf die Festplatte
geschrieben: Ein Prüfauftrag lebt im Speicher des Servers, bis das Relay ihn meldet, und der
Agent behält zwischen zwei Prüfungen nichts.

Der Reverse-Proxy muss `/api/agent/relay` wie `/api/ingest` weiterleiten, Request-Bodies bis
16 MB erlauben (ein ausgelasteter Hypervisor meldet Tausende von Messwerten) und Leerlauf-
verbindungen mindestens 60 Sekunden offen halten: Der Server hält eine Relay-Anfrage bis zu
25 Sekunden, während er auf Arbeit wartet (Long Polling).

## 2. Ein Enrollment-Token erstellen {#2-create-an-enrollment-token}

Fügen Sie einen Monitor hinzu, wählen Sie **Server mit dem Agenten** und setzen Sie das Häkchen
*Dieser Agent ist ein Relay für einen entfernten Standort* (oder öffnen Sie den Eingang *Relay* der
Hinzufügen-Seite). Geben Sie dem Standort einen Namen und erstellen Sie das Token: Es trägt den
Relay-Modus und den Standort, auf dem Rechner ist nichts anzupassen. Kopieren Sie den Wert
`dmon_…`: Er wird nur einmal angezeigt; die Seite wartet dann auf die erste Meldung.

## 3. Das Relay am entfernten Standort starten {#3-start-the-relay-at-the-remote-site}

Jeder Rechner am entfernten Standort mit Docker, mit der Datei `docker-compose.agent.yml` aus
dem Repository:

```sh
DUMBMONIT_AGENT_URL=https://monitor.example.org \
DUMBMONIT_AGENT_TOKEN=dmon_… \
DUMBMONIT_AGENT_HOSTNAME=relay-lyon \
DUMBMONIT_AGENT_RELAY=true \
DUMBMONIT_AGENT_SITE="Lyon office" \
docker compose -f docker-compose.agent.yml up -d
```

Oder der normale, mit dem Einzeiler-Installer installierte Agent, mit zwei zusätzlichen
Schlüsseln in `/etc/dumbmonit/agent.yaml`:

```yaml
relay: true
site: Lyon office
```

Der Rechner erscheint innerhalb weniger Sekunden in DumbMonit als gewöhnliches Agent-Monitor,
und seine Seite zeigt *Relay · Standort Lyon office · leitet 0 Monitore weiter*. Das Log des
Agenten meldet `relay mode enabled: waiting for probes from the server`, sobald sein erster
Batch akzeptiert wurde.

!!! note "Was das Relay erreichen muss"
    Das Relay prüft Monitore aus seinem eigenen Netzwerk: Es braucht denselben Zugriff, den der
    Server dort bräuchte — UDP 161 für SNMP, 8006 für Proxmox, 5001 für DSM usw. Für
    **Ping**-Monitore braucht der Container den Sysctl `net.ipv4.ping_group_range`, der in der
    Compose-Datei bereits gesetzt ist (siehe [ICMP-Ping ohne Capability](docker.md#icmp-ping-without-a-capability)).
    Für Monitore in einem VLAN oder Link-Local-Netz oder für `localhost`-Ziele verwenden Sie
    `network_mode: host`.

## 4. Monitore dem Relay zuweisen {#4-assign-devices-to-the-relay}

Fügen Sie die entfernten Monitore wie gewohnt hinzu — SNMP-Switch, Proxmox-Knoten, HTTP-Check —
mit ihrer Adresse **aus Sicht des entfernten Standorts** (`192.168.10.1`,
`https://pve.lan:8006`). Stellen Sie im Monitorformular unter *Weitere Optionen* **Erreicht über**
auf den Relay-Agenten statt auf *Direkt*. Bestehende Monitore lassen sich genauso umstellen;
das Zurückstellen auf *Direkt* lässt den Server sie wieder selbst abfragen.

Von da an gilt:

- Der Server stellt bei jedem Intervall eine Prüfung für den Monitor in die Warteschlange; das
  Relay nimmt sie innerhalb von Millisekunden auf (es wartet beim Server auf Arbeit), führt
  den Collector aus und meldet das Ergebnis;
- **Jetzt prüfen** und die Profilerkennung auf der Monitorseite laufen ebenfalls über das
  Relay;
- *Letzte Prüfung* und Fehler des Monitors folgen dem Urteil des Relays. Nimmt das Relay eine
  Prüfung nicht rechtzeitig auf, zeigt der Monitor *Zeitüberschreitung: Relay-Agent … hat die
  Prüfung nicht aufgenommen* — prüfen Sie, ob der Agent mit `relay: true` läuft;
- die Seite des Agenten zeigt *Relay für N Monitore*, und `GET /api/relays` listet die Relays
  mit Standort und Anzahl auf.

## 5. Agenten hinter dem Relay installieren {#5-install-agents-behind-the-relay}

Ein Standort hinter einer Firewall hat oft Server, die DumbMonit gar nicht erreichen: Nur das
Relay geht nach außen. Diese Server können trotzdem den vollständigen Agenten betreiben (System,
Platten, Dienste, Container, Sicherungen): Sie verbinden sich mit dem **Gateway** des Relays, und
das Relay trägt ihre Verbindung zum Server.

```
 remote site (only the relay goes out)                     head office
 ┌────────────┐  TLS 1.3, mTLS   ┌──────────────┐  tunnel inside the  ┌──────────────────┐
 │ agent      │═════════════════▶│ relay agent  │═ relay's own mTLS ═▶│ DumbMonit server │
 │ (server A) │  to the relay's  │  gateway     │  connection, TCP    │  agent port 8443 │
 └────────────┘  port 8443       └──────────────┘  8443 outbound      └──────────────────┘
        └──────────── one TLS session, end to end, agent ⇄ server ─────────────┘
```

**In DumbMonit** fügen Sie einen Monitor hinzu, wählen **Server mit dem Agenten** und unter *Wie
erreicht der Rechner diesen Server?* **Über &lt;Relay&gt; (Standort)**. Geben Sie die Adresse an,
unter der der Standort das Relay erreicht (eine IP oder ein lokaler Name). Die angezeigten Befehle
— Linux/macOS, Windows, Docker — zeigen dann auf das Gateway des Relays
(`--url=http://192.168.10.5:8443 --tls-url=https://192.168.10.5:8443`) und tragen weiterhin den
Fingerabdruck der Zertifizierungsstelle des **Servers**. Der Ablauf wartet dann auf die erste
Verbindung und lässt Sie den Rechner benennen, in einen Ordner legen und einem Standort zuordnen,
ohne die Seite zu verlassen. Von einem Standort in *Nach Standort hinzufügen* aus wählt **Server
mit dem Agenten** das Relay dieses Standorts vor; die Seite des Relays selbst hat **Agenten
darüber installieren**.

Ein Relay für eine Installation zu wählen, bittet es, bei seiner nächsten Meldung sein Gateway zu
öffnen (standardmäßig auf Port 8443). Das Panel *Relay* auf der Seite des Relays zeigt das Gateway
— *Lauscht auf Port 8443 — 3 Agenten verbunden* — und lässt Sie Port und Adresse ändern oder es
schließen. Öffnen Sie diesen Port in der Firewall des Relays selbst, **nur für das Netz des
Standorts**. Bei einem Docker-Relay veröffentlichen Sie den Port (`-p 8443:8443`, oder die
auskommentierten `ports:` von `docker-compose.agent.yml`). `gateway_listen: 0.0.0.0:9443` in der
`agent.yaml` des Relays (`DUMBMONIT_AGENT_GATEWAY_LISTEN`) hat Vorrang vor der Einstellung in der
Oberfläche.

Was durch das Relay läuft:

- **Der verschlüsselte Kanal des Agenten**, unangetastet. Das Relay erkennt ein TLS-ClientHello
  und kopiert die Bytes in beide Richtungen in einen Tunnel, den es unter seinem *eigenen*
  Client-Zertifikat zum Agenten-Port des Servers öffnet (ein HTTP-Upgrade auf
  `/api/agent/relay/tunnel`). Der Server führt dann den TLS-1.3-Handshake mit dem Agenten des
  Standorts aus, als hätte er sich direkt verbunden: Client-Zertifikat des Agenten, festgelegte
  Zertifizierungsstelle, Token und Bindungsgeheimnis prüft der Server, Ende zu Ende. Die
  Registrierung läuft genauso.
- **Der Installer und das Binary**, die das Relay über seinen eigenen Kanal vom Server holt und
  auf demselben Port ausliefert (nur `/install.sh`, `/install.ps1`, `/download/<Datei>`). Der
  Installationsbefehl trägt die SHA-256 jedes Binarys: Das Skript verweigert alles andere.

Der Server speichert für jeden Agenten das Relay, über das er sich verbindet. Die Seite des
Agenten sagt *Erreicht diesen Server über das Relay …*; die Topologiekarte zeichnet das Relay als
seinen Elternknoten; und wenn das Relay ausfällt, werden die Agenten dahinter als vom Relay
`suppressed` gezeigt, nicht als viele einzelne Ausfälle. Ein Agent, der sich später direkt (oder
über ein anderes Relay) verbindet, wird bei seiner nächsten Meldung aktualisiert.

Das Active-Directory-Kit bietet dieselbe Wahl: Unter **Route** wählen Sie das Relay, und die Server
der Domäne laden den Agenten von ihm herunter und melden über es.

## Alarmierung, wenn das Relay ausfällt {#alerting-when-the-relay-is-down}

Ein Relay, das verstummt, reißt seine Monitore mit: Ihre Prüfungen stoppen, und alle würden auf
einmal *nicht erreichbar*. Das Relay gilt als **Elternteil** aller Monitore, die es prüft (zusätzlich
zu einem möglicherweise gesetzten Elternteil): Ist das Relay selbst *nicht erreichbar*, werden
die Alarme seiner Monitore als `suppressed` mit `suppressed_by` = dem Relay angezeigt, und es
geht nur eine Benachrichtigung hinaus — für das Relay. Das ausdrücklich gesetzte Elternteil
behält seine Rolle und hat Vorrang, wenn beide ausgefallen sind.

## Grenzen {#limits}

- Ein Relay pro Monitor. Ein vom Relay geprüfter Monitor wird nicht zusätzlich vom Server
  geprüft.
- Agent-Monitore werden nie über ein Relay geprüft (sie senden ihre eigenen Metriken); ein
  Agent kann sich stattdessen über das Gateway eines Relays *verbinden* (siehe oben). Ein Relay
  verbindet sich immer direkt: Es gibt kein Relay hinter einem Relay.
- Das Gateway eines Relays nimmt standardmäßig 64 Verbindungen gleichzeitig und 4 MiB/s pro
  Tunnel und Richtung an (`gateway_max_connections`, `gateway_rate_kib`); der Server nimmt 256
  Tunnel pro Relay an.
- Das Relay führt höchstens 8 Prüfungen gleichzeitig aus; eine Prüfung wird durch das
  Prüf-Timeout des Servers (`DUMBMONIT_PROBE_TIMEOUT_SECS`) begrenzt, auf dem Agenten auf
  120 s gedeckelt.
- Ein Relay-Agent braucht die einkompilierten Collectors: Agenten, die älter als diese
  Funktion sind, ignorieren `relay: true`, und der Server meldet ihre Prüfungen als
  zeitüberschritten. Installieren Sie mit dem aktuellen Binary oder Image neu.
