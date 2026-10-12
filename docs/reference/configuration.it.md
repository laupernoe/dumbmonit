# Configurazione

Tutto passa attraverso variabili d'ambiente; nessuna è obbligatoria. Non
c'è alcun file di configurazione da montare.

## Server

| Variabile | Predefinito | Ruolo |
|---|---|---|
| `DUMBMONIT_BIND` | `0.0.0.0:8080` | Indirizzo di ascolto dell'API e dell'interfaccia web, dentro il container. |
| `DUMBMONIT_DATA_DIR` | `/data` | Directory persistente: `dumbmonit.db` (SQLite), `secret.key`, `vm/` (VictoriaMetrics integrato) e `webchange/` (screenshot dei cambiamenti di siti web). |
| `DUMBMONIT_VM_URL` | *(non impostata)* | URL base di un VictoriaMetrics esterno (`http://host:8428`). Quando impostata, quello integrato non viene avviato; vedi [sotto](#embedded-victoriametrics). |
| `DUMBMONIT_SECRET` | *(generato)* | Segreto dell'istanza. Se vuoto, letto da `<data dir>/secret.key`, generato al primo avvio. Cifra le credenziali dei dispositivi, i segreti dei canali e i token degli agent con AES-256-GCM. |
| `DUMBMONIT_MAX_CONCURRENT_PROBES` | `64` | Sonde simultanee massime, tutti i collector combinati. |
| `DUMBMONIT_PROBE_TIMEOUT_SECS` | `10` | Timeout rigido di una sonda. I monitor di servizio hanno la propria opzione `timeout_seconds`, più breve. |
| `DUMBMONIT_FLUSH_INTERVAL_SECS` | `5` | Periodo delle scritture in batch verso VictoriaMetrics. |
| `DUMBMONIT_FLUSH_BATCH` | `5000` | Numero di campioni bufferizzati che attiva una scrittura prima che il periodo trascorra. |
| `DUMBMONIT_WORKERS` | `min(4, CPU)` | Thread del runtime asincrono. Il server passa il suo tempo attendendo sulla rete: quattro bastano per un homelab, aumentali solo per centinaia di dispositivi. |
| `DUMBMONIT_DB_POOL` | `4` | Connessioni SQLite aperte al massimo. Ognuna costa un thread e una cache di pagine di 2 MB; le connessioni inattive si chiudono dopo cinque minuti. |
| `DUMBMONIT_LOG` | `info` | Filtro dei log, sintassi `tracing`: `debug`, `warn`, `dumbmonit=debug,sqlx=warn`… |
| `DUMBMONIT_AGENT_DIR` | `/agents` | Directory dei binari agent serviti sotto `/download/`. Vuota o mancante: il comando di installazione fallisce con `404`, il server funziona comunque. |
| `DUMBMONIT_RESET_PASSWORD` | *(disattivata)* | `1`, `true`, `yes` o `on`: azzera la password e ogni sessione all'avvio. L'interfaccia mostra allora di nuovo `/setup`. Rimuovila dopo. |
| `DUMBMONIT_COOKIE_SECURE` | *(disattivata)* | `1` per impostare l'attributo `Secure` sul cookie di sessione. Solo dietro HTTPS: su HTTP semplice il browser non rinvierebbe mai il cookie. |
| `DUMBMONIT_TRUSTED_PROXIES` | *(vuota)* | Reverse proxy di cui `X-Forwarded-For` viene creduto, come indirizzi o intervalli CIDR separati da virgole (`10.0.0.5, 172.16.0.0/12`). Il rate limiter di login e il log di sicurezza vedono allora l'indirizzo reale del client invece di quello del proxy; senza, ogni visitatore dietro il proxy condivide un unico bucket. L'header viene ignorato da qualsiasi altro indirizzo, così un client non può scegliere il proprio bucket. |
| `DUMBMONIT_OIDC_AUTO_CREATE` | *(disattivata)* | `true` per creare un account per un'identità sconosciuta al suo primo single sign-on. Disattivata, solo gli account esistenti (già collegati, oppure corrispondenti per email verificata dal provider) possono accedere. Letta solo mentre nessuna impostazione di single sign-on è salvata nell'interfaccia. |
| `DUMBMONIT_OIDC_ALLOW_HTTP` | *(disattivata)* | `1` per accettare un issuer OpenID Connect in `http://` semplice. Disattivata, la schermata delle impostazioni rifiuta tutto tranne `https://`: un issuer in chiaro consegna il codice di autorizzazione e il client secret a chiunque ascolti sulla rete. Solo per un provider di test sul loopback. |
| `DUMBMONIT_METRICS_PUBLIC` | *(disattivata)* | `1` per servire `/metrics`, `/federate` e `/prometheus/api/v1/…` senza un token API. Chiunque possa raggiungere la porta legge allora ogni misurazione di ogni dispositivo e lo stato dell'istanza; impostala solo dove qualcos'altro già limita l'accesso alla porta. Vedi [Metriche](metrics.md#scraping-dumbmonit). |
| `DUMBMONIT_ALERT_INTERVAL_SECS` | `30` | Periodo di valutazione degli avvisi. I valori sotto 10 vengono alzati a 10. |
| `DUMBMONIT_ALERT_HISTORY_DAYS` | `90` | Retention della cronologia degli avvisi, in giorni. |
| `DUMBMONIT_BACKUP_ENABLED` | `1` | Backup locali pianificati del database. `0`, `false`, `no` o `off` per fermarne la scrittura. Vedi [Backup e ripristino](../install/backup.md#scheduled-local-backups). |
| `DUMBMONIT_UPDATE_CHECK` | `1` | `off` ferma il controllo giornaliero dell'ultima release GitHub (istanze air-gapped). Vedi [Aggiornamento](../install/update.md). |
| `DUMBMONIT_BACKUP_DIR` | `<data dir>/backups` | Dove vengono scritti i backup pianificati. Puntala su un secondo volume per sopravvivere alla perdita del primo. |
| `DUMBMONIT_BACKUP_INTERVAL_HOURS` | `24` | Ore tra due backup, da 1 a 8760. Il primo avviene un intervallo dopo l'avvio, non all'avvio. |
| `DUMBMONIT_DEMO` | *(disattivata)* | `1` per una demo pubblica, di sola lettura, con un patrimonio fittizio fisso, accesso `demo` / `demo`. **Ricrea il database a ogni avvio**: mai su un volume di dati reale. Vedi [Eseguire un'istanza demo](../install/demo.md). |
| `DUMBMONIT_BACKUP_KEEP` | `7` | Quanti backup vengono conservati, da 1 a 365. I più vecchi vengono rimossi con la loro `.key`. |
| `DUMBMONIT_BROWSER_URL` | *(non impostata)* | Chromium headless usato per gli screenshot delle [modifiche a un sito web](../devices/webchange.md#screenshots), come `http://host:9222` (la sua porta DevTools). Il file Compose ne fornisce uno sotto il profilo `screenshots`: `DUMBMONIT_BROWSER_URL=http://browser:9222 docker compose --profile screenshots up -d`. Non impostata: solo confronto di testo. |

La retention della baseline (60 giorni) e il periodo di apprendimento di
14 giorni non sono configurabili.

### Vecchi nomi `EZYMONIT_*`

DumbMonit si chiamava EzyMonit fino a settembre 2026. Ogni variabile
`DUMBMONIT_*` viene letta anche con il suo vecchio nome `EZYMONIT_*`, come
ripiego, con un avviso all'avvio per variabile ancora impostata in quel
modo (`EZYMONIT_X is deprecated, use DUMBMONIT_X`). Il vecchio file di
database `/data/ezymonit.db` viene rinominato `dumbmonit.db` all'avvio, e
i token agent `ezym_…` restano validi (i nuovi sono `dmon_…`). Il cookie
di sessione è ora `dumbmonit_session`: il primo avvio dopo la
rinominazione disconnette tutti una volta.

### VictoriaMetrics integrato

L'immagine fornisce il binario VictoriaMetrics. A meno che
`DUMBMONIT_VM_URL` non sia impostata, il server lo avvia come processo
figlio all'avvio, inoltra le sue righe di log nel log del server, lo
riavvia con un backoff se muore, e lo ferma allo spegnimento (`SIGTERM`,
20 s di grazia, poi kill; lo `stop_grace_period` del file Compose lascia
spazio per questo).

| Variabile | Predefinito | Ruolo |
|---|---|---|
| `DUMBMONIT_VM_BINARY` | `/victoria-metrics-prod` | Percorso del binario VictoriaMetrics da avviare. |
| `DUMBMONIT_VM_LISTEN` | `127.0.0.1:8428` | Indirizzo di ascolto, dentro il container. Loopback per impostazione predefinita: solo il server gli parla. L'overlay di sviluppo imposta `0.0.0.0:8428` e pubblica la porta. |
| `DUMBMONIT_VM_RETENTION` | `12` | Retention, nella sintassi di `-retentionPeriod`: un numero di mesi, oppure una durata come `30d` o `2y`. |
| `DUMBMONIT_VM_MEMORY` | `256MB` | Budget per le cache di VictoriaMetrics (`-memory.allowedBytes`). Senza un budget VM le dimensiona per il 60% della RAM dell'host e la sua memoria residente cresce per giorni. Qualche decina di dispositivi entra in 256 MB; aumentalo per centinaia di host. |

I suoi dati vivono in `<data dir>/vm`, sullo stesso volume del database.
Tre flag sono fissati dal server e non configurabili:
`-search.maxUniqueTimeseries=100000` (una query incontrollata non può
esaurire il budget), `-search.maxConcurrentRequests=4` (un'interfaccia
più il motore di avvisi) e `-dedup.minScrapeInterval=10s` (punti
duplicati più vicini di dieci secondi vengono collassati; nulla viene
perso al ritmo di 30 s dei collector).

Con `DUMBMONIT_VM_URL` impostata, nulla di questo si applica: il server
usa l'istanza che nomini e quella integrata non viene mai avviata.
`GET /api/health` dice in quale caso ti trovi (`victoria.embedded`).

## Variabili a livello Compose

Queste vengono lette da `docker-compose.yml`, non dal server:

| Variabile | Predefinito | Ruolo |
|---|---|---|
| `DUMBMONIT_PORT` | `8080` | Porta host pubblicata per l'interfaccia. |
| `DUMBMONIT_VM_RETENTION`, `DUMBMONIT_VM_MEMORY` | `12`, `256MB` | Passate al server; vedi sopra. |
| `DUMBMONIT_VM_PORT` | `8428` | Porta host per il VictoriaMetrics integrato, solo overlay di sviluppo. |

## Porte

| Porta | Container | Scopo |
|---|---|---|
| `8080/tcp` | `dumbmonit` | Interfaccia web, API, installer e download agent, ingest degli agent. L'unica porta da pubblicare. |
| `8428/tcp` | `dumbmonit` | API HTTP del VictoriaMetrics integrato, sul loopback del container. Pubblicata solo dall'overlay di sviluppo. |

In uscita: SNMP (UDP 161 per impostazione predefinita) verso i
dispositivi, HTTPS verso Proxmox, PBS, Synology e i servizi di notifica,
più qualsiasi cosa che i tuoi monitor di servizio bersaglino. Il ping
ICMP richiede il sysctl `net.ipv4.ping_group_range` impostato nel file
Compose (nessuna capability).

## Volumi e file

Un volume, `dumbmonit-data`, montato su `/data`. Il suo nome è fisso nel
file Compose, qualunque sia il nome del progetto.

| Percorso (nel container) | Volume | Contenuto |
|---|---|---|
| `/data/dumbmonit.db` | `dumbmonit-data` | Dispositivi, credenziali (cifrate), regole, stato e cronologia degli avvisi, baseline, silenzi, canali, token agent, hash della password, sessioni. |
| `/data/secret.key` | `dumbmonit-data` | Il segreto dell'istanza. **Fanne un backup.** Senza di esso, le credenziali cifrate sono irrecuperabili e il server si rifiuta di avviarsi. |
| `/data/backups/` | `dumbmonit-data` | Backup locali pianificati: `dumbmonit-<timestamp>.db` (una copia SQLite online) e la `.key` corrispondente. Giornalieri per impostazione predefinita, sette conservati. |
| `/data/vm/` | `dumbmonit-data` | Serie temporali del VictoriaMetrics integrato, 12 mesi per impostazione predefinita. Vuota quando `DUMBMONIT_VM_URL` è impostata. |
| `/agents/` | immagine | Binari agent. Monta un'altra directory e imposta `DUMBMONIT_AGENT_DIR` per fornire i tuoi. |
| `/etc/ssl/certs/ca-certificates.crt` | immagine | Radici TLS per HTTPS in uscita. Monta il tuo bundle qui per fidarti di un'autorità privata. |

## Reimpostare la password

```bash
DUMBMONIT_RESET_PASSWORD=1 docker compose up -d dumbmonit
# apri l'interfaccia: /setup chiede una nuova password
docker compose up -d dumbmonit   # riavvia senza la variabile
```

Il reset azzera la password e ogni sessione; dispositivi, regole e canali
restano intatti.

## Agent

L'agent ha le proprie variabili e il proprio file di configurazione:
vedi
[Agent Linux, macOS, FreeBSD e Windows](../devices/agent.md#configuration-file).

### Ambito di raccolta

Cosa misura l'agent, e quindi quante serie produce una macchina. I valori
predefiniti mantengono un host con decine di container a poche centinaia
di serie; ogni chiave è una voce `agent.yaml`, sovrascrivibile dalla sua
variabile d'ambiente.

| Chiave | Variabile | Predefinito | Ruolo |
|---|---|---|---|
| `interfaces_ignore` | `DUMBMONIT_AGENT_INTERFACES_IGNORE` | `^(veth\|br-\|docker\|virbr\|lo$\|vEthernet)` | Interfacce di rete escluse: interfacce di container e VM, il loopback. Un elenco di nomi esatti o espressioni regolari (non ancorate; qualsiasi cosa con un metacarattere è una regex). Il valore sostituisce quello predefinito; un elenco vuoto mantiene ogni interfaccia. |
| `interfaces_only` | `DUMBMONIT_AGENT_INTERFACES_ONLY` | *(vuoto)* | Elenco di consenso, stessa sintassi. Quando impostata, solo queste interfacce vengono riportate e `interfaces_ignore` non viene consultata. |
| `mounts_ignore` | `DUMBMONIT_AGENT_MOUNTS_IGNORE` | `^/(var/lib/docker/\|run/(user\|docker\|containerd\|snapd\|credentials\|systemd\|lock\|udev\|netns\|lxc\|lxd)/\|sys/\|proc/\|dev/\|snap/)` | Punti di mount esclusi dall'utilizzo del file system e dalla I/O su disco, stessa sintassi. Le unità rimovibili sotto `/run/media/` restano. I pseudo file system (`overlay`, `tmpfs`, `squashfs`, `fuse.portal`, `cgroup2`…) vengono sempre saltati, qualunque sia il percorso. |
| `cpu_per_core` | `DUMBMONIT_AGENT_CPU_PER_CORE` | `false` | Una serie `cpu_core_usage_percent` per core, in aggiunta all'utilizzo globale. `cpu_count` viene riportato in entrambi i casi. |
| `docker_max_containers` | `DUMBMONIT_AGENT_DOCKER_MAX_CONTAINERS` | `200` | Container dettagliati per ciclo, quelli in esecuzione prima. Oltre, i container vengono solo contati (`container_count`, `container_running_count` restano esatti) e `container_series_skipped` dice quanti. `0` conta senza dettagliare. |

Nell'ambiente, gli elenchi sono separati da virgole
(`^veth, ^br-, docker0`); in YAML, sia un elenco sia una stringa singola
separata da virgole. I contatori di I/O su disco (`disk_read_bytes`,
`disk_written_bytes`) portano solo l'etichetta `device`: una serie per
dispositivo a blocchi, indipendentemente da quante volte è montato.
