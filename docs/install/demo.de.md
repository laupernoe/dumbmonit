# Eine Demo-Instanz betreiben

Die öffentliche Demo auf **[demo.dumbmonit.app](https://demo.dumbmonit.app)**
ist ein gewöhnliches DumbMonit-Image, gestartet mit `DUMBMONIT_DEMO=1`.
Sie können dasselbe selbst betreiben — um DumbMonit einem Team zu zeigen,
bevor Sie es echt installieren, oder um eine Vorführung auf Ihrer eigenen
Site zu veröffentlichen.

!!! warning "Niemals auf einem echten Datenvolume"
    Der Demo-Modus **erstellt die Datenbank bei jedem Start neu**. Geben
    Sie ihm ein eigenes Volume, oder gar keines: auf das `/data` einer
    echten Instanz gerichtet, würde er Ihre Monitore, Regeln und Historie
    durch den fiktiven Bestand ersetzen.

## Was der Demo-Modus tut

- **Mit `demo` / `demo` einloggen.** Die Login-Seite sagt das und füllt
  die Felder mit einem Klick. Das Konto sieht jeden Bildschirm, den ein
  Administrator sieht.
- **Nur lesend.** Jede Änderung — Monitore, Regeln, Stummschaltungen,
  Bestätigungen, Kanäle, Tokens, Benutzer, Einstellungen, Backups und
  Wiederherstellungen, Discovery-Scans, Agent-Registrierung,
  Schreibwerkzeuge des Assistant-Endpunkts — wird vom Server mit `403`
  und `{"error": "This is a read-only demo…", "demo": true}` abgelehnt.
  Ein- und Ausloggen sind die einzigen erlaubten Schreibvorgänge. Die
  Oberfläche zeigt die Meldung dort, wo der Klick passierte.
- **Zwei fiktive Kunden.** Beim Start wird die Datenbank neu angelegt und mit
  einem Homelab unter `*.home.arpa` befüllt — Proxmox VE, ein Synology-NAS mit
  einer iSCSI-LUN, Proxmox Backup Server, TrueNAS, OPNsense, UniFi, Kopia, ein
  Redfish-Server mit einem defekten Bauteil, ein SNMP-Switch, HTTP-, TLS-, DNS-
  und Ping-Prüfungen, ein Linux-Agent mit Docker — und, im MSP-Modus, mit einem
  kleinen Architekturbüro in Genf: ein Windows-Domänencontroller mit seinem
  Agenten, Veeam Backup & Replication, ein Omada-Netzwerk, Microsoft 365 und
  Entra ID, abgefragt über einen Relay-Agenten im Büro. Die Monitore werden von
  simulierten Geräten im Server beantwortet, gebunden an `127.0.0.1`, die
  aufgezeichnete API-Antworten wiedergeben: Die echten Kollektoren und Panels
  laufen. Ein Neustart ergibt denselben Bestand.
- **Dreißig Tage Verlauf und der Rest des Produkts.** Beim Start werden
  synthetische Metriken in begrenzten Blöcken in VictoriaMetrics geschrieben
  (die letzte Woche in Fünf-Minuten-Auflösung): Diagramme, *Letzte 7 Tage*,
  30-Tage-Verfügbarkeit, Prognosen, die Backup-Matrix und der Alarmverlauf sind
  gefüllt, und einige Alarme sind aktiv. Der Seed füllt auch, was sonst von
  Hand eingerichtet wird: Teams und Ordner, Konten und Bereitschaftspläne mit
  Eskalationsrichtlinien, Stummschaltungen, Runbooks, zwei Dashboards, geplante
  Berichte, Benachrichtigungskanäle (nie benutzt), Standorte auf der
  Weltkarte, bekannte Schwachstellen, vier Statusseiten (eine pro Nachtszene)
  mit einem laufenden Vorfall und ein Netzwerkscan mit einem neuen Gerät. Alles
  wird aus einem festen Seed berechnet: Nichts wird aus dem Netz geladen, nicht
  einmal die Schwachstellenquellen. Die Protokollsuche bleibt aus: Die Demo hat
  keinen Protokollspeicher.
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
