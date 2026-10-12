# Eine Demo-Instanz betreiben

Die öffentliche Demo auf **[demo.dumbmonit.app](https://demo.dumbmonit.app)**
ist ein gewöhnliches DumbMonit-Image, gestartet mit `DUMBMONIT_DEMO=1`.
Sie können dasselbe selbst betreiben — um DumbMonit einem Team zu zeigen,
bevor Sie es echt installieren, oder um eine Vorführung auf Ihrer eigenen
Site zu veröffentlichen.

!!! warning "Niemals auf einem echten Datenvolume"
    Der Demo-Modus **erstellt die Datenbank bei jedem Start neu**. Geben
    Sie ihm ein eigenes Volume, oder gar keines: auf das `/data` einer
    echten Instanz gerichtet, würde er Ihre Geräte, Regeln und Historie
    durch den fiktiven Bestand ersetzen.

## Was der Demo-Modus tut

- **Mit `demo` / `demo` einloggen.** Die Login-Seite sagt das und füllt
  die Felder mit einem Klick. Das Konto sieht jeden Bildschirm, den ein
  Administrator sieht.
- **Nur lesend.** Jede Änderung — Geräte, Regeln, Stummschaltungen,
  Bestätigungen, Kanäle, Tokens, Benutzer, Einstellungen, Backups und
  Wiederherstellungen, Discovery-Scans, Agent-Registrierung,
  Schreibwerkzeuge des Assistant-Endpunkts — wird vom Server mit `403`
  und `{"error": "This is a read-only demo…", "demo": true}` abgelehnt.
  Ein- und Ausloggen sind die einzigen erlaubten Schreibvorgänge. Die
  Oberfläche zeigt die Meldung dort, wo der Klick passierte.
- **Ein fester, fiktiver Bestand.** Beim Start wird die Datenbank frisch
  erstellt und mit einem kleinen Homelab unter `*.home.arpa` befüllt: ein
  Proxmox-VE-Cluster, ein Synology-NAS, ein Proxmox Backup Server, ein
  TrueNAS, eine OPNsense-Firewall, ein Redfish-Server mit einem
  ausfallenden Teil, ein paar HTTP-, TLS- und Ping-Checks, ein Agent-Host
  und eine Statusseite. Die Geräte werden von gefälschten Respondern
  innerhalb des Servers beantwortet, an `127.0.0.1` gebunden und
  aufgezeichnete API-Antworten wiedergebend, sodass die echten Collectors
  und Gerätepanels laufen. Ein Neustart ergibt denselben Bestand.
- **Sieben Tage Historie.** Beim Start werden synthetische Metriken in
  VictoriaMetrics geschrieben, sodass Diagramme, *Last 7 days*,
  Vorhersagen und die Alarmhistorie befüllt sind, und ein paar Alarme
  feuern, damit *Needs you* etwas enthält.
- **Nichts verlässt den Server.** Es wird nie eine Benachrichtigung
  gesendet, und keine Abfrage geht an eine von einer Besucherin oder
  einem Besucher eingegebene Adresse.
- **Eine geführte Tour** öffnet sich nach dem ersten Login; das Banner
  öffnet sie erneut.

Die Ratenbegrenzung des Logins funktioniert wie gewohnt. Stellen Sie die
Demo hinter Ihren üblichen Reverse Proxy mit HTTPS und setzen Sie
`DUMBMONIT_COOKIE_SECURE=1` und `DUMBMONIT_TRUSTED_PROXIES` wie für jede
öffentliche Instanz.

## Starten

```bash
docker run -d --name dumbmonit-demo -p 8080:8080 \
  -e DUMBMONIT_DEMO=1 \
  ghcr.io/laupernoe/dumbmonit:latest
```

Oder mit Compose, ohne Volume, sodass jeder Neustart frisch beginnt:

```yaml
services:
  dumbmonit-demo:
    image: ghcr.io/laupernoe/dumbmonit:latest
    restart: unless-stopped
    ports:
      - "8080:8080"
    environment:
      DUMBMONIT_DEMO: "1"
```

Pullen Sie das Image regelmäßig (oder lassen Sie ein Tool wie Watchtower
das tun): Die Demo zeigt dann immer die neueste Version mit denselben
Daten.
