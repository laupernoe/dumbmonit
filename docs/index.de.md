# DumbMonit {#dumbmonit}

Einfaches Monitoring vom Homelab bis zum kleinen Unternehmen. Ein Container, eine IP-Adresse
zum Eintippen, nützliche Diagramme und Alarme in weniger als einer Minute.

!!! warning "In Entwicklung"
    DumbMonit wird aktiv entwickelt; das aktuelle Image
    (`ghcr.io/laupernoe/dumbmonit:latest`) ist eine Alpha-Version für frühe Tester. Bis zur
    ersten Version ist mit Ecken und Kanten sowie inkompatiblen Änderungen zu rechnen.

DumbMonit liest das Netzwerk wie einen **Wetterbericht**: Die Startseite fasst
den Himmel in einem Satz zusammen („Clear skies." oder „2 advisories, 1 unreachable.") und
listet vor allem anderen auf, was Ihre Aufmerksamkeit braucht. Die Schweregrade folgen
der meteorologischen Stufenleiter (info → advisory → warning), Vorhersagen sind Prognosen, und Wartungsfenster
sind geplant.

![Die Übersichtsseite: der Berichtssatz, die Liste „Needs you" und die Prognosen](assets/screenshots/overview-light.png){ loading=lazy }

<div class="dm-links" markdown>
<a href="install/docker/">Installation<small>Docker Compose, erster Start, Backups</small></a>
<a href="install/first-device/">Erstes Gerät hinzufügen<small>SNMP, Netzwerk-Scan, wie es danach weitergeht</small></a>
<a href="alerting/">Alarmierung<small>Eingebaute Regeln, von Grund auf ruhig</small></a>
</div>

## Was überwacht wird {#what-it-watches}

| Quelle | Was Sie erhalten |
|---|---|
| [SNMP v1 / v2c / v3](devices/snmp.md) | Switches, Router, NAS, USV, Drucker. Fünf Profile gehören zum Produkt und werden automatisch anhand der `sysObjectID` des Geräts angewendet. Ein Netzwerk-Scan fügt alles hinzu, was antwortet, in einem Zug. |
| [Proxmox VE](devices/proxmox.md) | Nodes, virtuelle Maschinen und Container, Storages, Cluster-Quorum und das Alter des letzten erfolgreichen Backups pro Maschine. |
| [Proxmox Backup Server](devices/pbs.md) | Datastore-Belegung und Füllstandsprognose, Deduplizierung, Alter und Verifizierung des letzten Snapshots jeder Maschine, fehlgeschlagene Tasks, Garbage Collection. |
| [Synology DSM](devices/synology.md) | Volumes, Festplatten und ihr SMART-Zustand, Temperatur, Last, über die Web-API des NAS. |
| [Agent für Linux, macOS, FreeBSD und Windows](devices/agent.md) | CPU, Arbeitsspeicher, Datenträger, Netzwerk, Dienste, Container und Uptime von Maschinen, die kein SNMP sprechen, dazu Temperaturen, Festplattenzustand (SMART) und ZFS-Pools, wo die Maschine sie bereitstellt. Ein Befehl zur Installation, oder ein Docker-Image; im [Relay-Modus](install/remote-site.md) prüft er die eigenen Geräte eines entfernten Standorts ausschließlich über ausgehende Verbindungen. |
| [Dienste](devices/services.md) | HTTP(S), TCP-Port, DNS, Ping und Ablauf von TLS-Zertifikaten, im Stil von Uptime Kuma, mit Verlaufsbalken und Verfügbarkeit in Prozent. |
| [Integrationspakete](packs/index.md) | Eine Geräteart, in einer einzigen YAML-Datei beschrieben — eine HTTP-API oder eine Prometheus-`/metrics`-Seite auf dem Gerät — mit eigenen Alarmregeln, installiert ohne Server-Release. |

## Mit ihm sprechen {#talk-to-it}

- Ein [Assistent über MCP](using/assistant.md) — Claude Code, Claude Desktop,
  ChatGPT, VS Code, Cursor — mit 27 Tools: Ein `read`-Token kann nur ansehen (Status,
  Geräte, Alarme, Metriken, Agenten, Container …), ein `write`-Token kann auch handeln
  (stummschalten, quittieren, ein Gerät hinzufügen, einen Container neu starten, einen
  Vorfall auf der Statusseite veröffentlichen). Geheimnisse werden nie zurückgegeben.
- Die [HTTP-API](reference/api.md) hinter allem, was die Oberfläche tut, beschrieben unter
  `/api/openapi.json` (OpenAPI 3.1). Tokens (`dmt_…`) sind auf Lesen oder
  Schreiben beschränkt, können ablaufen, auf eine Liste von Netzwerken begrenzt werden und
  unterliegen einem Rate-Limit.

## Was läuft {#what-runs}

| Container | Rolle | Ressourcenbedarf |
|---|---|---|
| `dumbmonit` | Erfassung, API, Alarmierung, Web-Oberfläche und das eingebettete VictoriaMetrics für Zeitreihen | ~40 MB RAM + das VictoriaMetrics-Budget (standardmäßig 256 MB) |

Ein Container, ein Volume: Der Server startet VictoriaMetrics aus demselben
Image, und Konfiguration und Zustand liegen in einer eingebetteten SQLite-Datenbank. Es
gibt keinen Datenbank-Container. Stattdessen kann ein externes VictoriaMetrics verwendet werden
(`DUMBMONIT_VM_URL`).

!!! note "Zum Namen"
    DumbMonit hieß bis September 2026 EzyMonit. Befehle, Umgebungsvariablen,
    Image-Namen und Pfade wurden mit umbenannt; die alten
    `EZYMONIT_*`-Variablen und `ezym_`-Agent-Tokens werden weiterhin akzeptiert. Siehe
    [Upgrade](install/docker.md#upgrading).

DumbMonit ist zu 100 % Open Source unter der Apache-2.0-Lizenz, einschließlich der
Abhängigkeiten: Keine Funktion wird für eine kostenpflichtige Edition zurückgehalten.
