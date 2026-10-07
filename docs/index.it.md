# DumbMonit {#dumbmonit}

Monitoraggio semplice, dall'homelab alla piccola impresa. Un container, un indirizzo IP
da digitare, grafici utili e avvisi in meno di un minuto.

!!! warning "Lavori in corso"
    DumbMonit è in sviluppo attivo; l'immagine attuale
    (`ghcr.io/laupernoe/dumbmonit:latest`) è un'alpha per i primi tester. Aspettati
    qualche asperità e modifiche incompatibili fino alla prima release.

DumbMonit legge la rete come un **bollettino meteo**: la home page descrive
il cielo in una frase ("Cielo sereno." oppure "2 avvisi, 1 irraggiungibile.") e
elenca ciò che richiede la tua attenzione, prima di qualsiasi altra cosa. Le severità seguono la scala
meteorologica (info → advisory → warning), le previsioni sono forecast e le finestre
di manutenzione sono pianificate.

![La pagina di panoramica: la frase del bollettino, l'elenco "Needs you" e le previsioni](assets/screenshots/overview-light.png){ loading=lazy }

<div class="dm-links" markdown>
<a href="install/docker/">Installazione<small>Docker Compose, primo avvio, backup</small></a>
<a href="install/first-device/">Aggiungi il tuo primo dispositivo<small>SNMP, scansione di rete, cosa succede dopo</small></a>
<a href="alerting/">Avvisi<small>Regole integrate, silenziosi per costruzione</small></a>
</div>

## Cosa controlla {#what-it-watches}

| Fonte | Cosa ottieni |
|---|---|
| [SNMP v1 / v2c / v3](devices/snmp.md) | Switch, router, NAS, UPS, stampanti. Cinque profili sono inclusi nel prodotto e applicati automaticamente a partire dal `sysObjectID` del dispositivo. Una scansione di rete aggiunge in un colpo solo tutto ciò che risponde. |
| [Proxmox VE](devices/proxmox.md) | Nodi, macchine virtuali e container, storage, quorum del cluster ed età dell'ultimo backup riuscito per ogni macchina. |
| [Proxmox Backup Server](devices/pbs.md) | Utilizzo dei datastore e previsione di riempimento, deduplica, età e verifica dell'ultimo snapshot di ogni macchina, task falliti, garbage collection. |
| [Synology DSM](devices/synology.md) | Volumi, dischi e il loro stato SMART, temperatura, carico, tramite l'API web del NAS. |
| [Agent Linux, macOS, FreeBSD e Windows](devices/agent.md) | CPU, memoria, dischi, rete, servizi, container e uptime delle macchine che non parlano SNMP, più temperature, stato dei dischi (SMART) e pool ZFS dove la macchina li espone. Un solo comando per installarlo, oppure un'immagine Docker; in [modalità relay](install/remote-site.md) sonda i dispositivi di una sede remota usando solo connessioni in uscita. |
| [Servizi](devices/services.md) | HTTP(S), porta TCP, DNS, ping e scadenza del certificato TLS, in stile Uptime Kuma, con barra della cronologia e percentuale di disponibilità. |
| [Pacchetti di integrazione](packs/index.md) | Un tipo di dispositivo dichiarato in un singolo file YAML — un'API HTTP o una pagina Prometheus `/metrics` sul dispositivo — con le proprie regole di allerta, installato senza una release del server. |

## Parlaci {#talk-to-it}

- Un [assistente via MCP](using/assistant.md) — Claude Code, Claude Desktop,
  ChatGPT, VS Code, Cursor — con 27 strumenti: un token `read` può solo guardare (stato,
  dispositivi, avvisi, metriche, agent, container…), un token `write` può anche agire
  (silenziare, confermare, aggiungere un dispositivo, riavviare un container, pubblicare un
  incidente sulla pagina di stato). I segreti non vengono mai restituiti.
- L'[API HTTP](reference/api.md) alla base di tutto ciò che fa la UI, descritta in
  `/api/openapi.json` (OpenAPI 3.1). I token (`dmt_…`) hanno ambito di lettura o
  scrittura, possono scadere, essere limitati a un elenco di reti e sono
  soggetti a rate limit.

## Cosa gira {#what-runs}

| Container | Ruolo | Impronta |
|---|---|---|
| `dumbmonit` | Raccolta, API, avvisi, interfaccia web e VictoriaMetrics integrato per le serie temporali | ~40 MB di RAM + il budget di VictoriaMetrics (256 MB per impostazione predefinita) |

Un container, un volume: il server avvia VictoriaMetrics dalla stessa
immagine, e configurazione e stato risiedono in un database SQLite integrato. Non
c'è nessun container per il database. Si può usare invece un VictoriaMetrics esterno
(`DUMBMONIT_VM_URL`).

!!! note "A proposito del nome"
    DumbMonit si chiamava EzyMonit fino a settembre 2026. Comandi, variabili
    d'ambiente, nomi delle immagini e percorsi sono stati rinominati di conseguenza; le vecchie
    variabili `EZYMONIT_*` e i token agent `ezym_` sono ancora accettati. Vedi
    [Aggiornamento](install/docker.md#upgrading).

DumbMonit è open source al 100% con licenza Apache 2.0, dipendenze
comprese: nessuna funzionalità è riservata a un'edizione a pagamento.
