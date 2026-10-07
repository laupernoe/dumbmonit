# Installazione con Docker {#install-with-docker}

DumbMonit gira in un solo container: il server `dumbmonit` (raccolta, API,
avvisi, interfaccia web) avvia il proprio VictoriaMetrics per l'archiviazione delle serie temporali; il
binario è incluso nell'immagine. Configurazione e stato risiedono in un database
SQLite integrato. Un solo volume, `/data`, contiene il database, il segreto dell'istanza e le
serie temporali.

!!! note "Immagine alpha"
    `ghcr.io/laupernoe/dumbmonit:latest` è l'ultima build alpha taggata (amd64;
    le immagini arm64 sono per ora sospese); `:edge` segue l'ultimo commit su `main`. Per eseguirlo dai sorgenti,
    `docker compose up -d --build` costruisce la stessa immagine in locale (circa dieci
    minuti la prima volta; serve solo Docker).

## Prerequisiti {#prerequisites}

- Docker Engine con il plugin Compose (`docker compose version` funziona).
- Una macchina che possa raggiungere i dispositivi che vuoi monitorare. Non deve
  essere raggiungibile da questi, tranne che per gli [agent](../devices/agent.md), che inviano
  le proprie misurazioni al server via HTTP.
- La porta `8080` libera sull'host, oppure un'altra porta a tua scelta (vedi sotto).

## Il file Compose {#the-compose-file}

Questo è il `docker-compose.yml` del repository; salvalo in una directory
a sé stante:

```yaml
# Un container: il server DumbMonit esegue il proprio VictoriaMetrics (integrato
# nell'immagine) e tiene tutto in un unico volume. Per usare invece un VictoriaMetrics
# esterno, imposta DUMBMONIT_VM_URL e quello integrato non viene avviato.
name: dumbmonit

services:
  dumbmonit:
    # `docker compose up -d` scarica l'immagine pubblicata (ultima release; usa
    # `:edge` per l'ultimo commit su main). Per costruirla invece da questo checkout,
    # esegui `docker compose up -d --build`: il risultato riceve lo stesso nome
    # e viene usato da quel momento in poi.
    image: ghcr.io/laupernoe/dumbmonit:latest
    build: .
    ports:
      # La porta dell'host è configurabile: 8080 è una porta molto affollata su una
      # macchina da homelab. `DUMBMONIT_PORT=8099 docker compose up -d` la sposta.
      - "${DUMBMONIT_PORT:-8080}:8080"
    volumes:
      # Database SQLite, segreto dell'istanza e serie temporali (/data/vm).
      #
      # Il server gira come utente 65532 (non root). Un volume nominato creato da
      # Docker eredita questo proprietario dall'immagine: non serve fare nulla. Un bind mount
      # (`./data:/data`) o un volume creato prima di questa modifica va ceduto
      # una volta sola:
      #   docker run --rm -v dumbmonit-data:/data alpine chown -R 65532:65532 /data
      # oppure, per lasciare i file così come sono, esegui il container come loro proprietario
      # con `user: "1000:1000"` (qualsiasi uid va bene: l'immagine non ha /etc/passwd).
      - dumbmonit-data:/data
    environment:
      # Decommenta per impostare tu il segreto invece di lasciare che DumbMonit lo
      # generi in /data/secret.key. Cifra le credenziali dei dispositivi: perderlo
      # significa reinserirle tutte.
      # DUMBMONIT_SECRET: replace-me-with-32-random-characters
      # Password dimenticata: `DUMBMONIT_RESET_PASSWORD=1 docker compose up -d` cancella
      # la password all'avvio e la UI ne chiede una nuova; poi riavvia
      # senza la variabile.
      DUMBMONIT_RESET_PASSWORD: ${DUMBMONIT_RESET_PASSWORD:-}
      # VictoriaMetrics integrato: retention (mesi, oppure es. 30d / 2y) e
      # budget di memoria delle sue cache. Un homelab di qualche decina di dispositivi sta in
      # 256 MB; aumentalo quando monitori centinaia di host.
      DUMBMONIT_VM_RETENTION: ${DUMBMONIT_VM_RETENTION:-12}
      DUMBMONIT_VM_MEMORY: ${DUMBMONIT_VM_MEMORY:-256MB}
      # VictoriaMetrics esterno al posto di quello integrato:
      # DUMBMONIT_VM_URL: http://victoriametrics:8428
    # Privilegi minimi: nessuna capability, nessuna escalation di privilegi, e il
    # file system dell'immagine in sola lettura — /data (volume) e /tmp (tmpfs) sono
    # gli unici posti scrivibili. Anche i monitor "ping" ICMP non richiedono capability: la
    # sysctl qui sotto permette al server (non privilegiato) di aprire socket ICMP echo
    # nel network namespace del container stesso. Rimuovila se non usi mai il ping.
    cap_drop:
      - ALL
    security_opt:
      - no-new-privileges:true
    read_only: true
    tmpfs:
      - /tmp
    sysctls:
      net.ipv4.ping_group_range: "0 2147483647"
    # Il server ferma VictoriaMetrics dopo di sé: lascia il tempo di farlo.
    stop_grace_period: 30s
    restart: unless-stopped

volumes:
  dumbmonit-data:
    # Nome fisso, indipendente dal nome del progetto compose: il volume è ciò di cui
    # fai il backup e ciò che un aggiornamento deve ritrovare.
    name: dumbmonit-data
```

Poi avvialo:

```bash
docker compose up -d
```

!!! tip "Costruire l'immagine da sé"
    `docker compose up -d` scarica l'immagine pubblicata e ignora `build: .`.
    Da un clone del repository, `docker compose up -d --build` costruisce invece la
    stessa immagine in locale (circa dieci minuti a freddo; sull'host non serve
    nessuna toolchain Rust o Node). Usalo per eseguire dai sorgenti o da un
    branch.

## VictoriaMetrics integrato {#embedded-victoriametrics}

L'immagine contiene il binario di VictoriaMetrics (`/victoria-metrics-prod`, da
`victoriametrics/victoria-metrics:v1.152.0`). Quando `DUMBMONIT_VM_URL` non è
impostata, il server lo avvia come processo figlio in ascolto su `127.0.0.1:8428`,
salva le sue serie in `/data/vm`, inoltra le sue righe di log nel proprio log,
lo riavvia con un backoff se muore e lo ferma allo spegnimento. Nulla viene
pubblicato: la porta resta dentro il container.

Vale la pena conoscere due variabili: `DUMBMONIT_VM_RETENTION` (`12` mesi per
impostazione predefinita; vanno bene anche `30d` o `2y`) e `DUMBMONIT_VM_MEMORY` (`256MB`, il
budget delle sue cache; aumentalo per centinaia di host). Il resto è nel
[riferimento della configurazione](../reference/configuration.md#embedded-victoriametrics).

Per usare un VictoriaMetrics che già gestisci, imposta `DUMBMONIT_VM_URL` al suo
indirizzo (`http://host:8428`): quello integrato non viene più avviato e
`/data/vm` resta vuota.

## Primo avvio {#first-start}

Apri `http://<your-host>:8080`. Un'istanza nuova mostra la schermata `/setup`,
dove crei il primo account amministratore (password di almeno 12
caratteri).

La schermata chiede prima di tutto il **codice di setup**. Finché non esiste alcun amministratore,
il server stampa nei log all'avvio un codice monouso:

```sh
docker compose logs dumbmonit | grep -A1 "setup code"
```

```
  First-run setup code: K7XQ4-M9PRT
```

Dimostra che chi crea l'account admin controlla il server, così nessun altro
sulla rete può impossessarsi per primo di un'istanza nuova. Il codice vive solo
in memoria, cambia a ogni riavvio finché non esiste un admin, e i tentativi
errati sono soggetti a rate limit. Per un deployment automatizzato, imposta
`DUMBMONIT_SETUP_CODE` per sceglierlo tu.

![La schermata di login](../assets/screenshots/login-light.png){ loading=lazy }

Poi aggiungi il tuo primo dispositivo: vedi [Aggiungi il tuo primo dispositivo](first-device.md).

## Variabili d'ambiente {#environment-variables}

Tutto passa dalle variabili d'ambiente; nessuna è obbligatoria.

| Variabile | Predefinito | Ruolo |
|---|---|---|
| `DUMBMONIT_BIND` | `0.0.0.0:8080` | Indirizzo di ascolto dentro il container. |
| `DUMBMONIT_DATA_DIR` | `/data` | Database SQLite (`dumbmonit.db`), segreto dell'istanza (`secret.key`) e dati del VictoriaMetrics integrato (`vm/`). |
| `DUMBMONIT_VM_URL` | *(non impostata)* | URL di un VictoriaMetrics esterno. Se impostata, quello integrato non viene avviato. |
| `DUMBMONIT_VM_RETENTION` | `12` | Retention del VictoriaMetrics integrato: mesi, oppure `30d`, `2y`. |
| `DUMBMONIT_VM_MEMORY` | `256MB` | Budget di memoria delle cache del VictoriaMetrics integrato. |
| `DUMBMONIT_VM_LISTEN` | `127.0.0.1:8428` | Indirizzo di ascolto del VictoriaMetrics integrato, dentro il container. |
| `DUMBMONIT_SECRET` | *(generato)* | Segreto dell'istanza che cifra le credenziali dei dispositivi e i token. |
| `DUMBMONIT_MAX_CONCURRENT_PROBES` | `64` | Sonde concorrenti, tutti i collector insieme. |
| `DUMBMONIT_PROBE_TIMEOUT_SECS` | `10` | Durata massima di una sonda. |
| `DUMBMONIT_FLUSH_INTERVAL_SECS` | `5` | Periodo di scrittura verso VictoriaMetrics. |
| `DUMBMONIT_LOG` | `info` | Filtro dei log (sintassi `tracing`, es. `debug`, `dumbmonit=trace`). |
| `DUMBMONIT_AGENT_DIR` | `/agents` | Binari degli agent serviti sotto `/download/…`. |
| `DUMBMONIT_RESET_PASSWORD` | *(vuota)* | Impostala a `1` per cancellare la password e tutte le sessioni all'avvio. |
| `DUMBMONIT_SETUP_CODE` | *(generato)* | Codice di setup richiesto da `/setup` finché non esiste un admin. Se non impostata, ne viene stampato uno casuale nei log a ogni avvio. |
| `DUMBMONIT_COOKIE_SECURE` | *(disattivata)* | Impostala a `1` dietro un reverse proxy TLS per marcare il cookie di sessione come `Secure`. |
| `DUMBMONIT_ALERT_INTERVAL_SECS` | `30` | Periodo di valutazione degli avvisi (mai sotto 10). |
| `DUMBMONIT_ALERT_HISTORY_DAYS` | 90 giorni | Retention dello storico degli avvisi. Vedi il [riferimento della configurazione](../reference/configuration.md) per un'avvertenza sulla sua unità. |
| `DUMBMONIT_BACKUP_ENABLED` | attiva | Backup locali pianificati del database in `/data/backups/`. `DUMBMONIT_BACKUP_DIR`, `_INTERVAL_HOURS` (24) e `_KEEP` (7) li regolano; vedi [Backup e ripristino](backup.md). |

L'elenco completo, con i dettagli, è nel [riferimento della configurazione](../reference/configuration.md).
I nomi `EZYMONIT_*` precedenti alla ridenominazione sono ancora letti come fallback; vedi
[Aggiornamento](#upgrading).

## Fai il backup della chiave segreta {#back-up-the-secret-key}

Le community SNMP, le password e i token API sono cifrati con AES-256-GCM usando una
chiave derivata dal segreto dell'istanza. Al primo avvio, DumbMonit genera questo
segreto in `/data/secret.key` (dentro il volume `dumbmonit-data`).

!!! danger "Fai il backup di `secret.key` insieme al database"
    Senza di essa, le credenziali dei dispositivi sono irrecuperabili: un database ripristinato
    da solo dà un'istanza che non può comunicare con nulla. Il server rileva all'avvio una
    chiave mancante o modificata e rifiuta di continuare con un messaggio esplicito,
    invece di fallire in silenzio a ogni sonda. I backup pianificati
    la copiano per te — vedi [Backup e ripristino](backup.md).

Se preferisci gestire tu il segreto, imposta invece `DUMBMONIT_SECRET` nel file Compose
(32 caratteri casuali o più). Conservalo nel tuo gestore di password.

## Aggiornamento {#upgrading}

```bash
docker compose pull
docker compose up -d
```

Le migrazioni del database vengono eseguite all'avvio. I dati di VictoriaMetrics non vengono toccati.

!!! tip "Fai prima un backup"
    **Settings → Backup → Back up now** scrive in pochi secondi una copia coerente del database
    e di `secret.key` in `/data/backups/`, ed è da lì che
    ripristini se l'aggiornamento va male. Vedi
    [Backup e ripristino](backup.md).

### Da 0.1.0-alpha.1 {#from-010-alpha1}

Dalla 0.1.0-alpha.2 il container gira come utente 65532 invece che come root. Un
volume creato dalla alpha.1 appartiene ancora a root, e il server rifiuta di
avviarsi (`/data is not writable by the server`). Cedi il volume una volta sola:

```bash
docker compose down
docker run --rm -v dumbmonit-data:/data alpine chown -R 65532:65532 /data
docker compose up -d
```

Oppure lascia i file come sono ed esegui il container come loro proprietario, con
`user: "0:0"` (o l'uid di un bind mount) sotto il servizio in
`docker-compose.yml`.

### Da EzyMonit e dalla configurazione a due container {#from-ezymonit-and-from-the-two-container-setup}

DumbMonit si chiamava EzyMonit fino a settembre 2026, e girava come due container,
il server e un servizio `victoriametrics` separato. Per aggiornare i dati
del vecchio progetto vanno spostati una volta nel nuovo volume. Il vecchio progetto si chiamava
`ezymonit` (i suoi volumi `ezymonit_ezymonit-data` e `ezymonit_vm-data`;
`docker volume ls` lo conferma). Dalla vecchia directory:

```bash
docker compose down
```

Poi, con il nuovo `docker-compose.yml`:

```bash
docker volume create dumbmonit-data
docker run --rm -v ezymonit_ezymonit-data:/from -v dumbmonit-data:/to alpine cp -a /from/. /to/
docker run --rm -v ezymonit_vm-data:/from -v dumbmonit-data:/to alpine sh -c 'mkdir -p /to/vm && cp -a /from/. /to/vm/'
docker compose up -d
```

La vecchia directory dati di VictoriaMetrics ha il layout usato da quello integrato:
nulla da convertire. L'alternativa è tenere il VictoriaMetrics esterno e
puntarvi `DUMBMONIT_VM_URL`.

Ciò che il server gestisce da solo al primo avvio:

- Le variabili `EZYMONIT_*` sono ancora lette come fallback, con un avviso all'avvio
  per variabile (`EZYMONIT_X is deprecated, use DUMBMONIT_X`).
- `/data/ezymonit.db` viene rinominato in `dumbmonit.db`.
- I token agent `ezym_…` continuano a funzionare; i nuovi sono `dmon_…`. Gli agent continuano
  a inviare i dati; riesegui il comando di installazione su ogni macchina quando ti è comodo,
  [migra il vecchio servizio sul posto](../devices/agent.md#upgrade).
- Il cookie di sessione ha cambiato nome: tutti devono accedere di nuovo una volta. La preferenza
  di tema del browser viene mantenuta.

Ciò che non gestisce: il prefisso delle metriche è cambiato da `ezymonit_` a
`dumbmonit_` senza alcuna compatibilità. Le vecchie serie restano in VictoriaMetrics con
il vecchio nome e scadono con la retention; i grafici e le regole integrate ripartono
dall'aggiornamento. Le regole di allerta personalizzate che nominano metriche `ezymonit_…` vanno
modificate.

## Backup e ripristino {#backup-and-restore}

Un solo volume nominato, `dumbmonit-data`, contiene tutto:

| Percorso | Contenuto |
|---|---|
| `dumbmonit.db` | Dispositivi, regole, canali, stato degli avvisi, sessioni. |
| `secret.key` | Il segreto dell'istanza. |
| `backups/` | I backup locali pianificati: una copia del database e del segreto, giornaliera per impostazione predefinita, sette conservate. |
| `vm/` | Le serie temporali di VictoriaMetrics (12 mesi di retention per impostazione predefinita). |

DumbMonit esegue il backup del proprio database in modo pianificato, ed esporta l'intera
configurazione — dispositivi, credenziali, regole, canali — in un unico file
cifrato con una passphrase a tua scelta, che si ripristina su un'istanza nuova.
Entrambi sono in **[Backup e ripristino](backup.md)**, insieme a cosa fare prima di
un aggiornamento e all'unica regola su `secret.key` che vale la pena leggere due volte.

Per archiviare il volume stesso, grafici compresi, ferma lo stack e crea un tar:

```bash
docker compose stop
docker run --rm -v dumbmonit-data:/data -v "$PWD:/backup" alpine \
  tar czf /backup/dumbmonit-data.tgz -C /data .
docker compose start
```

Per ripristinare, crea il volume, estrai l'archivio al suo interno nello stesso modo, poi
`docker compose up -d`.

Se lo spazio conta, `--exclude=./vm` mantiene l'archivio piccolo: il database e
il segreto sono la configurazione, `vm/` sono solo i grafici.

## Cambiare la porta {#changing-the-port}

Imposta `DUMBMONIT_PORT` all'avvio:

```bash
DUMBMONIT_PORT=8099 docker compose up -d
```

Oppure metti `DUMBMONIT_PORT=8099` in un file `.env` accanto al file Compose.

## Reverse proxy {#reverse-proxy}

La UI e l'API sono servite su un'unica porta in HTTP semplice. Non viene usato alcun WebSocket:
l'interfaccia interroga l'API a intervalli, quindi qualsiasi reverse proxy funziona senza
configurazione particolare.

=== "Caddy"

    ```
    monit.example.com {
        reverse_proxy 127.0.0.1:8080
    }
    ```

=== "nginx"

    ```nginx
    server {
        listen 443 ssl;
        server_name monit.example.com;
        # ssl_certificate / ssl_certificate_key …

        location / {
            proxy_pass http://127.0.0.1:8080;
            proxy_set_header Host $host;
            proxy_set_header X-Forwarded-Proto $scheme;
            # Gli agent inviano batch di recupero dopo un'interruzione:
            client_max_body_size 16m;
        }
    }
    ```

Quando il proxy termina il TLS, imposta `DUMBMONIT_COOKIE_SECURE: "1"` così il cookie
di sessione viene inviato solo su HTTPS. Non impostarla in un deployment HTTP semplice: il
browser non rimanderebbe mai il cookie e il login sarebbe impossibile.

!!! note "Agent dietro un proxy"
    Il comando di installazione mostrato quando crei un token agent usa l'URL che il tuo
    browser ha usato per raggiungere la UI. Se è l'URL del proxy, anche gli agent lo
    useranno: assicurati che `/install.sh`, `/install.ps1`, `/download/…` e `/api/ingest`
    passino attraverso il proxy.

## Esecuzione come utente non root {#runs-as-a-non-root-user}

Il container esegue il server come utente `65532:65532` (numerico: l'immagine `scratch`
non ha `/etc/passwd`), con ogni capability rimossa, `no-new-privileges`
e un file system root in sola lettura; `/data` (il volume) e `/tmp` (un tmpfs)
sono gli unici percorsi scrivibili. Tutto questo è nel file Compose qui sopra.

Un volume nominato creato da Docker al primo avvio eredita il proprietario di `/data`
dall'immagine: non serve fare nulla. Due casi richiedono un comando:

- **Un volume creato da una versione precedente** (il server girava come root,
  quindi i file appartengono a root). Cedili una volta sola, con lo stack fermo:

  ```bash
  docker compose stop
  docker run --rm -v dumbmonit-data:/data alpine chown -R 65532:65532 /data
  docker compose up -d
  ```

  Altrimenti il sintomo è un container che esce subito con
  `creating directory /data … Permission denied` (oppure `unable to open database
  file`).

- **Un bind mount** (`./data:/data`) mantiene il proprietario della directory dell'host.
  Esegui `chown -R 65532:65532 ./data`, oppure esegui il container come
  proprietario della directory: `user: "1000:1000"` sotto il servizio, qualsiasi uid va bene.

## Ping ICMP senza capability {#icmp-ping-without-a-capability}

Il [monitor ping](../devices/services.md#ping) invia echo ICMP. Invece di
un raw socket — che richiede `NET_RAW`, e una capability concessa a un utente
non root del container non è comunque utilizzabile — usa un socket ICMP echo, che Linux
consente ai gruppi elencati in `net.ipv4.ping_group_range`. Il file Compose
imposta quella sysctl nel network namespace del container stesso:

```yaml
    sysctls:
      net.ipv4.ping_group_range: "0 2147483647"
```

Sull'host non cambia nulla. Docker 20.10 e successivi impostano da soli questo intervallo in ogni
container; le righe esplicite lo garantiscono anche sui motori più vecchi e
su Podman. Senza, il controllo segnala un errore di configurazione (mostrato sul
dispositivo, non notificato), mai un falso "host down". Con `docker run`, passa
`--sysctl net.ipv4.ping_group_range="0 2147483647"`.

## Provare senza hardware {#testing-without-hardware}

Aggiungi un [dispositivo demo](../devices/demo.md): produce misurazioni finte senza
nulla da preparare, così grafici, regole e notifiche si possono provare subito.
Per dati SNMP reali, spesso basta la macchina che esegue Docker — installaci
`snmpd` (`apt install snmpd`, consenti la community `public` sul bridge Docker
in `/etc/snmp/snmpd.conf`) e aggiungi un dispositivo SNMP con l'indirizzo dell'host
su quel bridge (`172.17.0.1` per impostazione predefinita): il profilo viene rilevato
automaticamente e compaiono interfacce, memoria e processi.

Gli sviluppatori possono aggiungere l'overlay `docker-compose.dev.yml` per pubblicare il
VictoriaMetrics integrato sulla porta `:8428` dell'host per query MetricsQL dirette:

```bash
docker compose -f docker-compose.yml -f docker-compose.dev.yml up -d
```
