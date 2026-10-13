# Einen entfernten Standort überwachen {#monitor-a-remote-site}

Ein DumbMonit, mehrere Netzwerke. Ein am entfernten Standort installierter **Relay-Agent**
führt im Auftrag des Servers die Prüfungen der ihm zugewiesenen Geräte aus und meldet die
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
- die **Zugangsdaten der Geräte**, die das Relay prüft (SNMP-Community, Proxmox-API-Token …),
  entschlüsselt, in jedem Prüfauftrag.

Über unverschlüsseltes HTTP durch das Internet wären beide unterwegs lesbar: Stellen Sie den
Server hinter HTTPS, bevor Sie etwas außerhalb Ihres eigenen Netzwerks weiterleiten. In einem
LAN oder VPN ist HTTP akzeptabel, und das Token ist das einzige Geheimnis im Transit, bis Sie
dem Relay Geräte zuweisen. Zugangsdaten werden auf keiner Seite auf die Festplatte
geschrieben: Ein Prüfauftrag lebt im Speicher des Servers, bis das Relay ihn meldet, und der
Agent behält zwischen zwei Prüfungen nichts.

Der Reverse-Proxy muss `/api/agent/relay` wie `/api/ingest` weiterleiten, Request-Bodies bis
16 MB erlauben (ein ausgelasteter Hypervisor meldet Tausende von Messwerten) und Leerlauf-
verbindungen mindestens 60 Sekunden offen halten: Der Server hält eine Relay-Anfrage bis zu
25 Sekunden, während er auf Arbeit wartet (Long Polling).

## 2. Ein Enrollment-Token erstellen {#2-create-an-enrollment-token}

**Einstellungen → Agenten → Neues Token**, oder fügen Sie ein Gerät vom Typ *Agent* hinzu.
Kopieren Sie den Wert `dmon_…`: Er wird nur einmal angezeigt.

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

Der Rechner erscheint innerhalb weniger Sekunden in DumbMonit als gewöhnliches Agent-Gerät,
und seine Seite zeigt *Relay · Standort Lyon office · leitet 0 Geräte weiter*. Das Log des
Agenten meldet `relay mode enabled: waiting for probes from the server`, sobald sein erster
Batch akzeptiert wurde.

!!! note "Was das Relay erreichen muss"
    Das Relay prüft Geräte aus seinem eigenen Netzwerk: Es braucht denselben Zugriff, den der
    Server dort bräuchte — UDP 161 für SNMP, 8006 für Proxmox, 5001 für DSM usw. Für
    **Ping**-Monitore braucht der Container `cap_add: [NET_RAW]` (siehe die Compose-Datei).
    Für Geräte in einem VLAN oder Link-Local-Netz oder für `localhost`-Ziele verwenden Sie
    `network_mode: host`.

## 4. Geräte dem Relay zuweisen {#4-assign-devices-to-the-relay}

Fügen Sie die entfernten Geräte wie gewohnt hinzu — SNMP-Switch, Proxmox-Knoten, HTTP-Check —
mit ihrer Adresse **aus Sicht des entfernten Standorts** (`192.168.10.1`,
`https://pve.lan:8006`). Stellen Sie im Gerätformular unter *Weitere Optionen* **Erreicht über**
auf den Relay-Agenten statt auf *Direkt*. Bestehende Geräte lassen sich genauso umstellen;
das Zurückstellen auf *Direkt* lässt den Server sie wieder selbst abfragen.

Von da an gilt:

- Der Server stellt bei jedem Intervall eine Prüfung für das Gerät in die Warteschlange; das
  Relay nimmt sie innerhalb von Millisekunden auf (es wartet beim Server auf Arbeit), führt
  den Collector aus und meldet das Ergebnis;
- **Jetzt prüfen** und die Profilerkennung auf der Geräteseite laufen ebenfalls über das
  Relay;
- *Letzte Prüfung* und Fehler des Geräts folgen dem Urteil des Relays. Nimmt das Relay eine
  Prüfung nicht rechtzeitig auf, zeigt das Gerät *Zeitüberschreitung: Relay-Agent … hat die
  Prüfung nicht aufgenommen* — prüfen Sie, ob der Agent mit `relay: true` läuft;
- die Seite des Agenten zeigt *Relay für N Geräte*, und `GET /api/relays` listet die Relays
  mit Standort und Anzahl auf.

## Alarmierung, wenn das Relay ausfällt {#alerting-when-the-relay-is-down}

Ein Relay, das verstummt, reißt seine Geräte mit: Ihre Prüfungen stoppen, und alle würden auf
einmal *nicht erreichbar*. Das Relay gilt als **Elternteil** aller Geräte, die es prüft (zusätzlich
zu einem möglicherweise gesetzten Elternteil): Ist das Relay selbst *nicht erreichbar*, werden
die Alarme seiner Geräte als `suppressed` mit `suppressed_by` = dem Relay angezeigt, und es
geht nur eine Benachrichtigung hinaus — für das Relay. Das ausdrücklich gesetzte Elternteil
behält seine Rolle und hat Vorrang, wenn beide ausgefallen sind.

## Grenzen {#limits}

- Ein Relay pro Gerät. Ein vom Relay geprüftes Gerät wird nicht zusätzlich vom Server
  geprüft.
- Agent-Geräte selbst können nicht über ein Relay laufen: Sie senden ihre eigenen Metriken.
- Das Relay führt höchstens 8 Prüfungen gleichzeitig aus; eine Prüfung wird durch das
  Prüf-Timeout des Servers (`DUMBMONIT_PROBE_TIMEOUT_SECS`) begrenzt, auf dem Agenten auf
  120 s gedeckelt.
- Ein Relay-Agent braucht die einkompilierten Collectors: Agenten, die älter als diese
  Funktion sind, ignorieren `relay: true`, und der Server meldet ihre Prüfungen als
  zeitüberschritten. Installieren Sie mit dem aktuellen Binary oder Image neu.
