# Monitore

Alles, was DumbMonit überwacht, auf vier Arten gesehen. In der App ist es **Monitore** im linken Menü (`g d`), mit vier Tabs. **Monitor hinzufügen** ist die Hauptschaltfläche des Tabs Liste (`g n`).

| Tab | Was Sie sehen | Lesen |
|---|---|---|
| **Liste** | Jeder Monitor und sein Zustand, Filter, Ordner und Sammelaktionen; ein Klick öffnet den Monitor | [Die Monitor-Seite](../using/devices.md) |
| **Karte** | Was von was abhängt (`g m`), und Ihre Standorte auf einer Weltkarte | [Topologiekarte](../using/topology.md), [Weltkarte](../using/world-map.md) |
| **Logs** | Eine Suche über die Logs von Server, Agenten, Syslog und SNMP-Traps (`g l`, Administratoren) | [Logs](../using/logs.md) |
| **Sicherheit** | Woher die von DumbMonit abgewehrten Angriffe kommen (Bedrohungskarte) und die bekannten Schwachstellen (CVE) der Software auf Ihren Monitoren (`g x`); die Sicherheitsbewertung wird in den Einstellungen eingeschaltet | [CVE und Schwachstellen](../using/cves.md), [Sicherheitsbewertung](../using/security-score.md) |

Der Metrik-Explorer (`Strg+K`, dann *Explore*) ist kein Tab: Er durchsucht die `dumbmonit_*`-Metriken und probiert eine Abfrage aus ([Metrik-Explorer](../using/dashboards.md)).

## Monitore hinzufügen

- Einzeln, über **Monitor hinzufügen**: [Ersten Monitor hinzufügen](../install/first-device.md).
- Durch das Scannen eines Netzwerks: [Netzwerkerkennung](../using/discovery.md).
- Aus den Labels der Container: [Monitore über Docker-Labels](../using/docker-labels.md).
- Aus einem anderen Tool (Uptime Kuma, Healthchecks, CSV): [Import aus anderen Tools](../using/imports.md).
- Aus Dateien, die in Git liegen: [Konfigurationsdateien (GitOps)](../using/config-files.md).

## Integrationen

Jeder Monitortyp hat eine Seite, im linken Menü nach Kategorien gruppiert: Server und Agenten, Virtualisierung, Speicher und NAS, Netzwerk und Firewalls, Hardware, Datenbanken, Webserver, Identität, Cloud und SaaS, Smart Home und Medien, selbst gehostete Apps, Backup, E-Mail, Observability und Internet-Prüfungen. Die Tabelle [aller Integrationen](../devices/index.md) listet sie mit auf, was jede braucht.
