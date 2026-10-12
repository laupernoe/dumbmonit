# Veeam Backup & Replication

Un server Veeam Backup & Replication tramite la sua API REST: job la cui
ultima esecuzione è fallita o terminata con un avviso, sessioni fallite
delle ultime 24 ore, repository che si riempiono, la licenza, e — per
computer o VM protetta — il suo ultimo backup riuscito.

DumbMonit accede con un account Windows che possiede solo il ruolo Veeam
Backup Viewer (OAuth2 password grant su `/api/oauth2/token`, porta 9419) e
mantiene il token per la sua durata di quindici minuti. Ogni chiamata
porta l'header obbligatorio `x-api-version`, `1.1-rev0` per impostazione
predefinita: la revisione arrivata con la versione 12.0.

Gli stati dei job (`/api/v1/jobs/states`) sono l'unica lettura
obbligatoria. Gli altri — `/api/v1/serverInfo`, `/api/v1/sessions`
(create nelle ultime 24 ore), `/api/v1/backupInfrastructure/repositories/states`,
`/api/v1/license` e `/api/v1/restorePoints` (i punti di ripristino di ogni
oggetto protetto, per l'intero server, i più recenti prima) — vengono
letti quando il ruolo lo consente. Veeam riserva alcune rotte al ruolo
Backup Administrator, la licenza tra queste sulle versioni attuali: un
rifiuto lì viene mostrato come "non leggibile", mai come un errore. Un
server troppo vecchio per conoscere `/api/v1/restorePoints` — la rotta è
apparsa dopo la 12.0 — viene trattato allo stesso modo: nessun
dispositivo, nessun errore.

!!! warning "Validato solo sulla documentazione del fornitore — non ancora testato su un sistema reale"

    Questa integrazione è stata costruita dal riferimento API REST e dal
    centro assistenza di Veeam (autenticazione, versioning, i modelli di
    job, sessione, repository, licenza e punto di ripristino), non su un
    server di backup in funzione. Quali rotte il ruolo Viewer può leggere
    varia con la versione: facci sapere cosa rifiuta il tuo server.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_veeam_`.

| Metrica | Etichette | Significato |
|---|---|---|
| `version_info` | `version` | Valore 1: la build, `12.3.1.1139`. |
| `job_enabled`, `job_running` | `job`, `type` | Per job. |
| `job_failed`, `job_warning` | `job`, `type` | 1 quando l'ultima esecuzione di un job abilitato è fallita, oppure terminata con un avviso. Un job disattivato mantiene il suo ultimo risultato, che non dice più nulla: non conta come nessuno dei due. |
| `job_result_info` | `job`, `type`, `result` | Valore 1: `success`, `warning`, `failed`, `none`. |
| `job_last_run_age_seconds` | `job`, `type` | Dall'ultima esecuzione; assente per un job che non è mai stato eseguito. |
| `job_progress_percent` | `job`, `type` | Mentre il job è in esecuzione. |
| `jobs`, `jobs_failed`, `jobs_warning`, `jobs_running`, `jobs_disabled` | | Conteggi; al massimo 200 job vengono descritti uno per uno. |
| `sessions_24h`, `sessions_failed_24h`, `sessions_warning_24h` | | Sessioni create nelle ultime 24 ore, per risultato. |
| `repository_capacity_bytes`, `repository_free_bytes`, `repository_used_bytes`, `repository_used_percent` | `repository`, `type` | Per repository. Un repository object storage non ha capacità: solo il suo spazio usato viene pubblicato. |
| `license_valid` | | 1 quando lo stato della licenza è `Valid`. |
| `license_info` | `status`, `type`, `edition` | Valore 1. |
| `license_expiry_seconds`, `support_expiry_seconds` | | Fino alla scadenza della licenza e del contratto di supporto; assenti per una licenza perpetua. |
| `license_instances_licensed`, `license_instances_used` | | Uso della licenza a istanze. |
| `section_readable` | `section`: `sessions`, `repositories`, `license`, `restore_points` | 1 quando l'account può leggerla, 0 quando Veeam l'ha rifiutata. |

Le [regole integrate](../alerting/rules.md#veeam-backup-replication) che
si applicano:

- **Veeam job failed**: l'ultima esecuzione di un job abilitato è fallita
  (Warning, ricordata ogni giorno).
- **Veeam job ended with a warning** (Advisory, ricordata ogni giorno).
- **Veeam repository nearly full**: più del 90% per trenta minuti
  (Advisory).
- **Veeam license expiring**: meno di 30 giorni rimanenti (Advisory,
  ricordata ogni giorno). Solo quando l'account può leggere la licenza.
- **Client device stale**: un computer o VM protetta ha superato
  `device_stale_days` senza un backup riuscito (Advisory, ricordata ogni
  giorno); vedi [Dispositivi](#devices) sotto.

## Dispositivi

Ogni oggetto con almeno un punto di ripristino — una VM, un computer
protetto da agent fisico o virtuale, una condivisione file — viene
pubblicato sotto la famiglia generica condivisa da ogni integrazione
DumbMonit che traccia i dispositivi protetti — `dumbmonit_client_device_*`,
con prefisso senza `veeam`, documentata per intero sotto
[Dispositivi client](../alerting/rules.md#client-devices):

| Metrica | Cosa | Etichette |
|---|---|---|
| `last_backup_timestamp_seconds` | Il punto di ripristino più recente dell'oggetto | `device`, `type`, `os`, `user`, `kind` |
| `stale_seconds` | Positivo una volta che un oggetto ha superato `device_stale_days` (3 per impostazione predefinita) senza un nuovo punto di ripristino; guida la regola [Client device stale](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (sempre `backup`) |

**Limiti da conoscere**: un job protegge i suoi oggetti come gruppo
(`jobs/states` dà solo un conteggio, `objectsCount`); la data per oggetto
proviene invece da `/api/v1/restorePoints`, letta per l'intero server in
una chiamata. Veeam crea un punto di ripristino solo una volta che un
backup è almeno parzialmente riuscito, quindi il più recente per oggetto è
già "l'ultimo backup riuscito" — nulla viene filtrato sul risultato qui.
Veeam non espone un'"ultima connessione" separata dal backup stesso, così
`last_seen_timestamp_seconds` e il segnale `connection` non vengono mai
pubblicati; un dispositivo mostra `last_connection` come "—" nella tabella
della pagina del dispositivo. Il `type` del dispositivo è il nome proprio
della piattaforma Veeam (`VSphere`, `WindowsPhysical`, `HyperV`…), mostrato
così com'è.

## La pagina del dispositivo

Il pannello Backups indica cosa non va in una frase, mostra i job falliti,
le sessioni fallite delle ultime 24 ore, i job in esecuzione e la licenza,
poi elenca i job abilitati — i fallimenti prima — con il loro tipo e
l'ultima esecuzione, e i repository con il loro riempimento e spazio
libero. La tabella dei dispositivi protetti (nome, piattaforma, ultimo
backup, stato) segue, dalla famiglia generica sopra.

## Dare a DumbMonit un account Veeam Backup Viewer

1. Sul server di backup, crea un account Windows locale per DumbMonit con una password lunga e casuale e nessun altro diritto sulla macchina (un account di dominio funziona anch'esso).

    ```
    net user dumbmonit * /add
    ```

2. Nella console Veeam, apri il menu principale > Users and Roles > Security, clicca Add, inserisci l'account (per esempio VBR01\dumbmonit) e dagli solo il ruolo Veeam Backup Viewer.

3. In DumbMonit, inserisci l'indirizzo del server di backup, per esempio "vbr.lan", e l'account come HOST\dumbmonit oppure DOMAIN\dumbmonit con la sua password. L'API REST ascolta sulla porta 9419 con un certificato autofirmato: spunta Accept an unverifiable certificate a meno che tu non ne abbia installato uno proprio.

!!! warning
    Veeam riserva alcune rotte dell'API REST al ruolo Backup Administrator, la licenza tra queste sulle versioni attuali. Con il ruolo Viewer DumbMonit legge i job, le sessioni, i repository e i punti di ripristino, e mostra la licenza come non leggibile: non chiede mai di più.

## Credenziali

| Credenziale | Campi |
|---|---|
| User name and password | L'account con il ruolo Veeam Backup Viewer, come `HOST\dumbmonit` oppure `DOMAIN\dumbmonit`. |

Indirizzo: il nome host o IP del server di backup, `host:porta`, oppure un
URL. Opzioni:

- **Protocol** (`https`) e **Port** (9419): quelli del servizio API REST.
- **API version** (`1.1-rev0`): inviata come `x-api-version`. Cambiala
  solo se il server la rifiuta; un server v13 documenta `1.3-rev1`.
- **Accept an unverifiable certificate**: il certificato proprio dell'API
  REST è autofirmato.
- **Timeout per request** (15 s).
- **Device staleness** (`device_stale_days`, 3 giorni per impostazione
  predefinita): vedi [Dispositivi](#devices) sopra.

## Risoluzione dei problemi

"Veeam refused the login": il nome utente ha bisogno del suo host o
dominio (`VBR01\dumbmonit`), oppure l'account non ha alcun ruolo Veeam.

"Veeam refused to list the jobs (403)": l'account ha un ruolo che non può
leggere i job; dagli Veeam Backup Viewer.

"Veeam refused the API version": imposta l'opzione API version su una che
il server elenca su `https://<server>:9419/swagger`.

La licenza mostra "Not readable": previsto con il ruolo Viewer sulle
versioni attuali; DumbMonit non chiede di più.

Nessun dispositivo, oppure il pannello Devices è vuoto:
`restore_points` legge "Not readable", probabilmente perché il server è
più vecchio della rotta (qualsiasi cosa prima delle revisioni successive
alla 12.0) piuttosto che un problema di ruolo — i punti di ripristino sono
inventario ordinario, non una pagina riservata agli amministratori.
