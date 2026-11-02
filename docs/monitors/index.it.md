# Monitor

Tutto ciò che DumbMonit controlla, visto in quattro modi. Nell'app è **Monitor** nel menu di sinistra (`g d`), con quattro schede. **Aggiungi un monitor** è il pulsante principale della scheda Elenco (`g n`).

| Scheda | Cosa si vede | Leggi |
|---|---|---|
| **Elenco** | Ogni monitor e il suo stato, filtri, cartelle e azioni di gruppo; un clic apre il monitor | [La pagina dei monitor](../using/devices.md) |
| **Mappa** | Cosa dipende da cosa (`g m`) e le tue sedi su una mappa del mondo | [Mappa della topologia](../using/topology.md), [Mappa del mondo](../using/world-map.md) |
| **Log** | Una sola ricerca nei log del server, degli agent, di syslog e dei trap SNMP (`g l`, amministratori) | [Log](../using/logs.md) |
| **Sicurezza** | Da dove arrivano gli attacchi fermati da DumbMonit (mappa delle minacce) e le vulnerabilità note (CVE) del software dei tuoi monitor (`g x`); il punteggio di sicurezza si attiva nelle Impostazioni | [CVE e vulnerabilità](../using/cves.md), [Punteggio di sicurezza](../using/security-score.md) |

L'esploratore di metriche (`Ctrl+K`, poi *Explore*) non è una scheda: sfoglia le metriche `dumbmonit_*` e prova una query ([Esplora metriche](../using/dashboards.md)).

## Aggiungere monitor

- Uno alla volta, da **Aggiungi un monitor**: [Aggiungere il primo monitor](../install/first-device.md).
- Scansionando una rete: [Rilevamento della rete](../using/discovery.md).
- Dalle etichette dei container: [Monitor dalle etichette Docker](../using/docker-labels.md).
- Da un altro strumento (Uptime Kuma, Healthchecks, CSV): [Importare da altri strumenti](../using/imports.md).
- Da file conservati in Git: [File di configurazione (GitOps)](../using/config-files.md).

## Integrazioni

Ogni tipo di monitor ha la sua pagina, raggruppata per categoria nel menu di sinistra: server e agent, virtualizzazione, storage e NAS, rete e firewall, hardware, database, server web, identità, cloud e SaaS, casa e media, app self-hosted, backup, e-mail, osservabilità e controlli Internet. La tabella di [tutte le integrazioni](../devices/index.md) le elenca con ciò che serve a ciascuna.
