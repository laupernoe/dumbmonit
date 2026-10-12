# Einen entfernten Standort überwachen

Ein DumbMonit, mehrere Netzwerke. Ein am entfernten Standort
installierter **Relay-Agent** führt im Auftrag des Servers die
Abfragen der ihm zugewiesenen Geräte aus und meldet die Ergebnisse
über dieselbe ausgehende Verbindung, die er bereits zum Pushen seiner
eigenen Metriken nutzt. Nichts am entfernten Standort zu öffnen, kein
VPN, keine zweite Instanz zum Nachsehen.

```
 head office                              remote site (NAT, firewall)
 ┌──────────────────┐   HTTPS, outbound   ┌──────────────────────────────┐
 │ DumbMonit server │◀────────────────────│ relay agent                  │
 │  scheduler       │  probes to run  ──▶ │  runs SNMP / Proxmox / HTTP… │──▶ switch, NAS,
 │  alerting, UI    │◀── samples, verdict │  pushes its own metrics too  │    hypervisor…
 └──────────────────┘                     └──────────────────────────────┘
```

## 1. Den Server über HTTPS veröffentlichen

Das Relay holt seine Aufträge vom Server: Der Server muss vom
entfernten Standort aus erreichbar sein, typischerweise
`https://monitor.example.org` hinter einem Reverse Proxy. Zwei Dinge
reisen auf dieser Verbindung:

- das **Enrollment-Token** (`dmon_…`), im `Authorization`-Header
  jedes Requests — wie bei jedem Agent;
- die **Gerätezugangsdaten** der vom Relay abgefragten Geräte
  (SNMP-Community, Proxmox-API-Token…), entschlüsselt, innerhalb
  jedes Abfrage-Auftrags.

Über einfaches HTTP durch das Internet wären beide auf dem Weg lesbar:
Stellen Sie den Server hinter HTTPS, bevor Sie irgendetwas außerhalb
Ihres eigenen Netzwerks relayen. In einem LAN oder VPN ist HTTP
akzeptabel, und das Token ist das einzige Secret in Bewegung, bis Sie
dem Relay Geräte zuweisen. Zugangsdaten werden nie auf eine Platte
geschrieben, auf keiner Seite: Ein Abfrage-Auftrag lebt im Speicher
des Servers, bis das Relay ihn meldet, und der Agent behält nichts
zwischen zwei Abfragen.

Der Reverse Proxy muss `/api/agent/relay` wie `/api/ingest`
weiterleiten, Request-Bodies bis 16 MB erlauben (ein ausgelasteter
Hypervisor meldet Tausende von Samples), und untätige Verbindungen
mindestens 60 Sekunden offen halten: Der Server hält einen
Relay-Request bis zu 25 Sekunden, während er auf Arbeit wartet (Long
Polling).

## 2. Ein Enrollment-Token erstellen

**Settings → Agents → New token**, oder ein Gerät der Art *agent*
hinzufügen. Kopieren Sie den Wert `dmon_…`: Er wird nur einmal
angezeigt.

## 3. Das Relay am entfernten Standort starten

Jede Maschine am entfernten Standort mit Docker, mit
`docker-compose.agent.yml` aus dem Repository:

```sh
DUMBMONIT_AGENT_URL=https://monitor.example.org \
DUMBMONIT_AGENT_TOKEN=dmon_… \
DUMBMONIT_AGENT_HOSTNAME=relay-lyon \
DUMBMONIT_AGENT_RELAY=true \
DUMBMONIT_AGENT_SITE="Lyon office" \
docker compose -f docker-compose.agent.yml up -d
```

Oder der gewöhnliche, mit dem Einzeiler-Installer installierte Agent,
mit zwei weiteren Schlüsseln in `/etc/dumbmonit/agent.yaml`:

```yaml
relay: true
site: Lyon office
```

Die Maschine erscheint innerhalb von Sekunden in DumbMonit, als
gewöhnliches Agent-Gerät, und ihre Seite sagt *Relay · site Lyon
office · relays 0 devices*. Das Log des Agents sagt `relay mode
enabled: waiting for probes from the server`, sobald sein erster
Batch akzeptiert wurde.

!!! note "Was das Relay erreichen muss"
    Das Relay fragt Geräte von seinem eigenen Netzwerk aus ab: Es
    braucht denselben Zugang, den der Server bräuchte, wäre er dort —
    UDP 161 für SNMP, 8006 für Proxmox, 5001 für DSM, und so weiter.
    Für **Ping**-Monitore braucht der Container `cap_add: [NET_RAW]`
    (siehe die Compose-Datei). Für Geräte in einem VLAN oder
    Link-Local-Netzwerk, oder für `localhost`-Ziele, nutzen Sie
    `network_mode: host`.

## 4. Geräte dem Relay zuweisen

Fügen Sie die entfernten Geräte wie gewohnt hinzu — SNMP-Switch,
Proxmox-Knoten, HTTP-Check — mit ihrer Adresse **wie vom entfernten
Standort aus gesehen** (`192.168.10.1`, `https://pve.lan:8006`). Im
Gerätedialog setzen Sie unter *More options* **Reached through** auf
den Relay-Agent statt *Direct*. Bestehende Geräte können auf dieselbe
Weise umgeschaltet werden; das Zurückschalten auf *Direct* lässt den
Server sie wieder selbst abfragen.

Von da an:

- der Server reiht bei jedem Intervall eine Abfrage für das Gerät ein;
  das Relay holt sie innerhalb von Millisekunden ab (es wartet auf dem
  Server auf Arbeit), führt den Collector aus und meldet;
- **Probe now** und Profilerkennung auf der Geräteseite laufen
  ebenfalls über das Relay;
- *last probe* und der Fehler des Geräts folgen dem Urteil des
  Relays. Holt das Relay eine Abfrage nicht rechtzeitig ab, zeigt das
  Gerät *Timed out: relay agent … did not pick up the probe* — prüfen
  Sie, dass der Agent mit `relay: true` läuft;
- die Seite des Agents zeigt *Relay for N devices*, und
  `GET /api/relays` listet die Relays mit ihrem Standort und ihrer
  Anzahl.

## Alarmierung, wenn das Relay down ist

Ein verstummendes Relay zieht seine Geräte mit sich: ihre Abfragen
stoppen, und sie würden alle gleichzeitig *unreachable* werden. Das
Relay wird als **übergeordnetes Gerät** jedes von ihm abgefragten
Geräts behandelt (zusätzlich zu einem von Ihnen gesetzten
übergeordneten Gerät): Ist das Relay selbst *unreachable*, werden die
Alarme seiner Geräte als `suppressed` mit `suppressed_by` = das Relay
angezeigt, und nur eine Benachrichtigung geht raus — für das Relay.
Das explizit gesetzte übergeordnete Gerät behält seine Rolle und
gewinnt, wenn beide down sind.

## Einschränkungen

- Ein Relay pro Gerät. Ein vom Relay abgefragtes Gerät wird nicht
  zusätzlich vom Server abgefragt.
- Agent-Geräte selbst können nicht relayed werden: Sie pushen ihre
  eigenen Metriken.
- Das Relay führt höchstens 8 Abfragen gleichzeitig aus; eine Abfrage
  ist durch das Probe-Timeout des Servers begrenzt
  (`DUMBMONIT_PROBE_TIMEOUT`), auf dem Agent bei 120 s gekappt.
- Ein Relay-Agent braucht die eingebauten Collectors: Agents, die
  älter als dieses Feature sind, ignorieren `relay: true`, und der
  Server meldet ihre Abfragen als Timeout. Neu installieren mit dem
  aktuellen Binary oder Image.
