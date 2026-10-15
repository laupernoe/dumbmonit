# Configurazione

Tutto passa attraverso variabili di ambiente; nessuna è richiesta. Non c'è alcun file di configurazione da montare.

## Server

| Variabile | Predefinito | Ruolo |
|---|---|---|
| `DUMBMONIT_BIND` | `0.0.0.0:8080` | Indirizzo di ascolto dell'API e dell'interfaccia web, dentro il contenitore. |
| `DUMBMONIT_DATA_DIR` | `/data` | Directory persistente: `dumbmonit.db` (SQLite), `secret.key`, `vm/` (VictoriaMetrics incorporato) e `webchange/` (screenshot delle modifiche del sito web). |
| `DUMBMONIT_VM_URL` | *(non impostato)* | URL di base di un'istanza VictoriaMetrics esterna (`http://host:8428`). Se impostato, quella incorporata non viene avviata; vedi [sotto](#embedded-victoriametrics). |
| `DUMBMONIT_SECRET` | *(generato)* | Segreto dell'istanza. Se vuoto, leggi da `<data dir>/secret.key`, generato al primo avvio. Crittografa le credenziali del monitor, i segreti del canale e i token dell'agente con AES-256-GCM. |
| `DUMBMONIT_MAX_CONCURRENT_PROBES` | `64` | Massimo numero di sonde simultanee, tutti i collezionisti combinati. |
| `DUMBMONIT_PROBE_TIMEOUT_SECS` | `10` | Hard timeout di una sonda. I monitor di servizio hanno il loro `timeout_seconds` più breve. |
| `DUMBMONIT_FLUSH_INTERVAL_SECS` | `5` | Periodo delle scritture batch verso VictoriaMetrics. |
| `DUMBMONIT_FLUSH_BATCH` | `5000` | Numero di campioni memorizzati che attiva una scrittura prima del periodo trascorso. |
| `DUMBMONIT_WORKERS` | `min(4, CPUs)` | Thread del runtime asincrono. Il server trascorre il suo tempo in attesa sulla rete: quattro coprono un homelab, sollevarlo solo per centinaia di monitor. |
| `DUMBMONIT_DB_POOL` | `4` | Massimo numero di connessioni SQLite aperte. Ognuno costa un thread e una cache di pagine da 2 MB; le connessioni inattive si chiudono dopo cinque minuti. |
| `DUMBMONIT_LOG` | `info` | Filtro di log, sintassi `tracing`: `debug`, `warn`, `dumbmonit=debug,sqlx=warn`… |
| `DUMBMONIT_AGENT_DIR` | `/agents` | Directory dei binari dell'agente serviti sotto `/download/`. Vuoto o mancante: il comando install fallisce con `404`, il server continua a funzionare. |
| `DUMBMONIT_RESET_PASSWORD` | *(disattivato)* | `1`, `true`, `yes` o `on`: cancella la password e ogni sessione all'avvio. L'interfaccia quindi mostra di nuovo `/setup`. Rimuovilo dopo. |
| `DUMBMONIT_COOKIE_SECURE` | *(disattivato)* | `1` per impostare sempre l'attributo `Secure` sul cookie di sessione. Solo dietro HTTPS: su HTTP semplice il browser non invierebbe mai il cookie indietro. Senza di esso, l'attributo viene ancora aggiunto ogni volta che il browser è su una pagina `https://` (il suo `Origin` lo dice, o un proxy attendibile invia `X-Forwarded-Proto: https`), quindi HTTP semplice sulla LAN continua a funzionare. |
| `DUMBMONIT_TRUSTED_PROXIES` | *(vuoto)* | Reverse proxy il cui `X-Forwarded-For`, `X-Forwarded-Proto` e `X-Forwarded-Host` vengono creduti, come indirizzi o intervalli CIDR separati da virgole (`10.0.0.5, 172.16.0.0/12`). Il limitatore di velocità di accesso e il log di sicurezza quindi vedono l'indirizzo client reale invece di quello del proxy; senza di esso, ogni visitatore dietro il proxy condivide un bucket. I link della pagina di stato (feed RSS, email di sottoscrizione) e l'indirizzo di ritorno del single sign-on utilizzano anche l'host inoltrato solo da questi proxy. Le intestazioni vengono ignorate da qualsiasi altro indirizzo, quindi un client non può scegliere il suo bucket o l'indirizzo di un link. |
| `DUMBMONIT_OIDC_AUTO_CREATE` | *(disattivato)* | `true` per creare un account per un'identità sconosciuta al suo primo single sign-on. Disattivato, solo gli account esistenti (già collegati, o corrispondenti a un'email verificata dal provider) possono accedere. Come le altre variabili `DUMBMONIT_OIDC_*`, leggi solo mentre non sono state salvate impostazioni single sign-on nell'interfaccia. |
| `DUMBMONIT_OIDC_ALLOW_HTTP` | *(disattivato)* | `1` per accettare un emittente OpenID Connect in `http://` semplice. Disattivato, la schermata delle impostazioni rifiuta tutto tranne `https://`: un emittente in chiaro consegna il codice di autorizzazione e il segreto client a chiunque stia in ascolto sulla rete. Solo per un provider di prova sul loopback. |
| `DUMBMONIT_SETUP_CODE` | *(generato)* | Codice `/setup` richiede mentre nessun amministratore esiste. Non impostato, uno casuale viene stampato nel log del server ad ogni avvio; impostalo per una distribuzione automatizzata. Vedi [Installa con Docker](../install/docker.md). |
| `DUMBMONIT_PUBLIC_URL` | *(non impostato)* | Indirizzo i browser raggiungono DumbMonit (`https://monit.example.org`). Utilizzato per i link nelle notifiche e nelle email della pagina di stato, l'URI di reindirizzamento del single sign-on e il contatto dato ai servizi push (solo HTTPS). Un valore salvato nell'interfaccia (politica di notifica, impostazioni SSO) vince. |
| `DUMBMONIT_API_CORS_ORIGINS` | *(qualsiasi origine)* | Origini consentite per chiamare l'API cross-origin con un token bearer, e per raggiungere il punto finale MCP: un elenco separato da virgole, o `off` per nessuno. Vedi [API HTTP](api.md#cross-origin-requests-cors). |
| `DUMBMONIT_OIDC_ISSUER` | *(non impostato)* | URL emittente del provider OpenID Connect; il rilevamento viene letto da `{issuer}/.well-known/openid-configuration`. Deve essere `https://` a meno che `DUMBMONIT_OIDC_ALLOW_HTTP` non sia impostato. |
| `DUMBMONIT_OIDC_CLIENT_ID` | *(non impostato)* | Id client registrato con il provider. |
| `DUMBMONIT_OIDC_CLIENT_SECRET` | *(non impostato)* | Segreto client. |
| `DUMBMONIT_OIDC_PROVIDER_NAME` | `SSO` | Etichetta del pulsante "Continua con …". |
| `DUMBMONIT_OIDC_SCOPES` | `openid profile email` | Ambiti richiesti, separati da spazi. |
| `DUMBMONIT_OIDC_GROUPS_CLAIM` | `groups` | Attestazione di token che elenca i gruppi dell'utente. |
| `DUMBMONIT_OIDC_ADMIN_GROUPS` | *(vuoto)* | Gruppi separati da virgole i cui membri diventano amministratori. |
| `DUMBMONIT_OIDC_OPERATOR_GROUPS` | *(vuoto)* | Gruppi separati da virgole i cui altri membri diventano operatori. Vedi [Single sign-on](../using/settings.md#single-sign-on). |
| `DUMBMONIT_METRICS_PUBLIC` | *(disattivato)* | `1` per servire `/metrics`, `/federate` e `/prometheus/api/v1/…` senza un token API. Chiunque possa raggiungere la porta legge quindi ogni misura di ogni monitor e lo stato dell'istanza; impostalo solo dove qualcos'altro restringe già l'accesso alla porta. Vedi [Metriche](metrics.md#scraping-dumbmonit). |
| `DUMBMONIT_ALERT_INTERVAL_SECS` | `30` | Periodo di valutazione degli avvisi. I valori inferiori a 10 vengono sollevati a 10. |
| `DUMBMONIT_ALERT_HISTORY_DAYS` | `90` | Conservazione della cronologia degli avvisi, in giorni. |
| `DUMBMONIT_BACKUP_ENABLED` | `1` | Backup locali programmati del database. `0`, `false`, `no` o `off` per smettere di scrivere. Vedi [Backup e ripristino](../install/backup.md#scheduled-local-backups). |
| `DUMBMONIT_UPDATE_CHECK` | `1` | `off` interrompe il controllo giornaliero dell'ultima release di GitHub (istanze air-gapped). Vedi [Aggiornamento](../install/update.md). |
| `DUMBMONIT_BACKUP_DIR` | `<data dir>/backups` | Dove vengono scritti i backup programmati. Punta a un secondo volume per sopravvivere alla perdita del primo. |
| `DUMBMONIT_BACKUP_INTERVAL_HOURS` | `24` | Ore tra due backup, da 1 a 8760. Il primo avviene un intervallo dopo l'avvio, non all'avvio. |
| `DUMBMONIT_DEMO` | *(disattivato)* | `1` per una demo pubblica di sola lettura con una proprietà fittizia fissa, accedi `demo` / `demo`. **Ricrea il database ad ogni avvio**: mai su un volume dati reale. Vedi [Esegui un'istanza demo](../install/demo.md). |
| `DUMBMONIT_BACKUP_KEEP` | `7` | Quanti backup vengono mantenuti, da 1 a 365. I più vecchi vengono rimossi con il loro `.key`. |
| `DUMBMONIT_BROWSER_URL` | *(non impostato)* | Chromium headless utilizzato per gli screenshot di [modifiche del sito web](../devices/webchange.md#screenshots), come `http://host:9222` (porta DevTools). Aggiungi un servizio `chromedp/headless-shell` al tuo file Compose, come mostrato in [Screenshot](../devices/webchange.md#screenshots). Non impostato: confronto solo del testo. |

La conservazione della linea di base (60 giorni) e il periodo di apprendimento di 14 giorni non sono configurabili.

### Nomi precedenti `EZYMONIT_*`

DumbMonit era chiamato EzyMonit fino a settembre 2026. Ogni variabile `DUMBMONIT_*` viene anche letta con il suo vecchio nome `EZYMONIT_*`, come fallback, con un avviso all'avvio per ogni variabile ancora impostata in quel modo (`EZYMONIT_X is deprecated, use DUMBMONIT_X`). Il vecchio file del database `/data/ezymonit.db` viene rinominato a `dumbmonit.db` all'avvio, e i token dell'agente `ezym_…` rimangono validi (i nuovi sono `dmon_…`). Il cookie di sessione è ora `dumbmonit_session`: il primo avvio dopo il rinomina fa scollegare tutti una volta.

### VictoriaMetrics incorporato {#embedded-victoriametrics}

L'immagine spedisce il binario VictoriaMetrics. A meno che `DUMBMONIT_VM_URL` non sia impostato, il server lo avvia come processo figlio all'avvio, inoltradol'output nei log del server, lo riavvia con un backoff se muore, e lo ferma all'arresto (`SIGTERM`, 20 s di grazia, poi uccidi; il `stop_grace_period` del file Compose lascia spazio per quello).

| Variabile | Predefinito | Ruolo |
|---|---|---|
| `DUMBMONIT_VM_BINARY` | `/victoria-metrics-prod` | Percorso del binario VictoriaMetrics da avviare. |
| `DUMBMONIT_VM_LISTEN` | `127.0.0.1:8428` | Indirizzo di ascolto, dentro il contenitore. Loopback per impostazione predefinita: solo il server parla con esso. L'overlay di sviluppo imposta `0.0.0.0:8428` e pubblica la porta. |
| `DUMBMONIT_VM_RETENTION` | `12` | Conservazione, nella sintassi di `-retentionPeriod`: un numero di mesi, o una durata come `30d` o `2y`. |
| `DUMBMONIT_VM_MEMORY` | `256MB` | Budget per le cache di VictoriaMetrics (`-memory.allowedBytes`). Senza un budget VM le ridimensiona per il 60% della RAM dell'host e la memoria residente cresce per giorni. Pochi dozzine di monitor si adattano a 256 MB; sollevalo per centinaia di host. |

I suoi dati vivono in `<data dir>/vm`, sulla stessa volume del database. Tre flag sono fissi dal server e non configurabili: `-search.maxUniqueTimeseries=100000` (una query in fuga non può esaurire il budget), `-search.maxConcurrentRequests=4` (un'interfaccia web più il motore di avviso) e `-dedup.minScrapeInterval=10s` (i punti duplicati più vicini di dieci secondi vengono compressi; nulla viene perso alla cadenza di 30 s dei collezionisti).

Con `DUMBMONIT_VM_URL` impostato, nulla di questo si applica: il server usa l'istanza che nomini e quella incorporata non viene mai avviata. `GET /api/health` dice quale caso sei (`victoria.embedded`).

## Impostazioni a livello di Compose

Il `docker-compose.yml` fornito non legge alcuna variabile e non richiede un file
`.env`. Tutto si imposta sotto `environment:` del servizio o modificando il
file; vedi [Opzioni avanzate](../install/docker.md#advanced-options) (porta,
segreto, retention e memoria, VictoriaMetrics esterno, reimpostazione della
password, screenshot). L'overlay di sviluppo `docker-compose.dev.yml` legge
`DUMBMONIT_VM_PORT` (predefinito `8428`) per la porta host del VictoriaMetrics
incorporato.

## Porte

| Porta | Contenitore | Scopo |
|---|---|---|
| `8080/tcp` | `dumbmonit` | Interfaccia web, API, programmi di installazione dell'agente e download, acquisizione dell'agente. L'unica porta da pubblicare. |
| `8428/tcp` | `dumbmonit` | API HTTP VictoriaMetrics incorporata, sul loopback del contenitore. Pubblicata solo dall'overlay di sviluppo. |

In uscita: SNMP (UDP 161 per impostazione predefinita) verso monitor, HTTPS verso Proxmox, PBS, Synology e servizi di notifica, più qualsiasi cosa i tuoi monitor di servizio mirino. ICMP ping necessita il sysctl `net.ipv4.ping_group_range` impostato nel file Compose (nessuna funzione).

## Volumi e file

Un volume, `dumbmonit-data`, montato su `/data`. Il suo nome è fisso nel file Compose, indipendentemente da come è chiamato il progetto.

| Percorso (nel contenitore) | Volume | Contenuto |
|---|---|---|
| `/data/dumbmonit.db` | `dumbmonit-data` | Monitor, credenziali (crittografate), regole, stato degli avvisi e cronologia, linee di base, silenzi, canali, token dell'agente, hash della password, sessioni. |
| `/data/secret.key` | `dumbmonit-data` | Il segreto dell'istanza. **Fai un backup di esso.** Senza di esso, le credenziali crittografate sono irrecuperabili e il server rifiuta di avviarsi. |
| `/data/backups/` | `dumbmonit-data` | Backup locali programmati: `dumbmonit-<timestamp>.db` (una copia SQLite online) e il matching `.key`. Giornalmente per impostazione predefinita, sette mantenuti. |
| `/data/vm/` | `dumbmonit-data` | Serie temporali VictoriaMetrics incorporata, 12 mesi per impostazione predefinita. Vuoto quando `DUMBMONIT_VM_URL` è impostato. |
| `/agents/` | image | Binari dell'agente. Monta un'altra directory e imposta `DUMBMONIT_AGENT_DIR` per spedirne di tuoi. |
| `/etc/ssl/certs/ca-certificates.crt` | image | Radici TLS per HTTPS in uscita. Monta il tuo bundle qui per fidarti di un'autorità privata. |

## Ripristino della password

```bash
# aggiungi DUMBMONIT_RESET_PASSWORD: "1" sotto environment: in docker-compose.yml
docker compose up -d
# apri l'interfaccia web: /setup chiede una nuova password
# rimuovi di nuovo la riga, poi:
docker compose up -d
```

Il ripristino cancella la password e ogni sessione; i monitor, le regole e i canali rimangono intatti.

## Agente

L'agente ha le sue variabili e file di configurazione: vedi [Agente Linux, macOS, FreeBSD e Windows](../devices/agent.md#file-di-configurazione).

### Ambito della raccolta

Quello che l'agente misura, e quindi quante serie un'unica macchina produce. Le impostazioni predefinite mantengono un host con dozzine di contenitori a poche centinaia di serie; ogni chiave è una voce `agent.yaml`, sovrascrivibile dalla sua variabile di ambiente.

| Chiave | Variabile | Predefinito | Ruolo |
|---|---|---|---|
| `interfaces_ignore` | `DUMBMONIT_AGENT_INTERFACES_IGNORE` | `^(veth\|br-\|docker\|virbr\|lo$\|vEthernet)` | Interfacce di rete lasciate fuori: interfacce contenitore e VM, il loopback. Un elenco di nomi esatti o espressioni regolari (non ancorate; tutto con un metacarattere è un regex). Il valore sostituisce il predefinito; un elenco vuoto mantiene ogni interfaccia. |
| `interfaces_only` | `DUMBMONIT_AGENT_INTERFACES_ONLY` | *(vuoto)* | Elenco di autorizzazione, stessa sintassi. Se impostato, solo queste interfacce vengono segnalate e `interfaces_ignore` non viene consultato. |
| `mounts_ignore` | `DUMBMONIT_AGENT_MOUNTS_IGNORE` | `^/(var/lib/docker/\|run/(user\|docker\|containerd\|snapd\|credentials\|systemd\|lock\|udev\|netns\|lxc\|lxd)/\|sys/\|proc/\|dev/\|snap/)` | Punti di montaggio lasciati fuori da utilizzo del file system e I/O su disco, stessa sintassi. I drive rimovibili sotto `/run/media/` rimangono. I filesystem pseudo (`overlay`, `tmpfs`, `squashfs`, `fuse.portal`, `cgroup2`…) vengono sempre saltati, indipendentemente dal percorso. |
| `cpu_per_core` | `DUMBMONIT_AGENT_CPU_PER_CORE` | `false` | Una serie `cpu_core_usage_percent` per core, in cima all'utilizzo globale. `cpu_count` viene segnalato comunque. |
| `docker_max_containers` | `DUMBMONIT_AGENT_DOCKER_MAX_CONTAINERS` | `200` | Contenitori dettagliati per ciclo, quelli in esecuzione per primi. Oltre, i contenitori vengono solo contati (`container_count`, `container_running_count` rimangono esatti) e `container_series_skipped` dice quanti. `0` conta senza dettagliare. |

Nell'ambiente, gli elenchi sono separati da virgole (`^veth, ^br-, docker0`); in YAML, uno elenco o una stringa separata da virgole. I contatori di I/O su disco (`disk_read_bytes`, `disk_written_bytes`) portano l'etichetta `device` solo: una serie per monitor a blocchi, quante volte è montato.
