# Proxmox Datacenter Manager

La console che federa diversi cluster Proxmox VE e istanze Proxmox Backup
Server: quali istanze raggiunge, l'intero patrimonio a colpo d'occhio, i
task falliti ovunque, e lo stato di salute proprio della console.

!!! warning "Testato solo su un'istanza di test"

    Questa integrazione è stata costruita e verificata su un Proxmox
    Datacenter Manager 1.1 installato per lo scopo, non su uno di
    produzione. Ogni chiamata è una lettura, e le forme provengono da un
    vero datacenter manager, ma nessuno l'ha ancora eseguito per settimane
    su un datacenter manager in funzione. Facci sapere cosa non funziona.

## Un monitor per il patrimonio, o un monitor per cluster?

Un monitor Datacenter Manager non sostituisce i monitor
[Proxmox VE](proxmox.md) e [Proxmox Backup Server](pbs.md). Risponde a una
domanda diversa.

| | Monitor Datacenter Manager | Un monitor per cluster e per backup server |
|---|---|---|
| Monitor da creare | uno | uno per cluster, uno per backup server |
| Credenziali da gestire | un token, sulla console | un token per istanza |
| Carico su ogni cluster | nessuno nostro: leggiamo la cache propria della console | una sonda per intervallo, per cluster |
| Cosa ottieni | raggiungibilità, versione, conteggi guest e nodi, totali CPU, memoria e storage, task falliti | tutto questo più i guest uno per uno, snapshot, job di backup, HA, Ceph, replicazione, dischi e SMART, pool ZFS, il calendario backup su 30 giorni |
| Cosa non puoi ottenere | serie per guest, dettaglio per datastore, dischi, snapshot, log dei task | un totale unico per l'intero patrimonio |
| Guasto principale | la console non può raggiungere un'istanza | il cluster non ci risponde |

**La nostra raccomandazione.** Aggiungi il monitor Datacenter Manager
quando esegui già una console e vuoi una pagina che dica se il patrimonio
è intero — è il modo più economico per notare che un sito remoto è caduto,
e non costa nulla ai cluster. Aggiungi un monitor dedicato per ogni
cluster e backup server che gestisci davvero: è lì che sta la profondità,
ed è ciò che ti dice *quale* VM si è fermata o *quale* backup è fallito. I
due si sovrappongono senza entrare in conflitto; un backup fallito genera
allora un avviso dal monitor PBS (con il log del task) e uno dalla
console (che nomina l'istanza). Silenzia la regola dei task della console
se la trovi ridondante, oppure limitala alle istanze per cui non hai un
monitor dedicato.

Se non esegui affatto una console, non installarne una solo per il
monitoraggio: i monitor dedicati vedono strettamente di più.

## La pagina del monitor

Oltre ai grafici generici, un monitor Datacenter Manager mostra tre
pannelli, letti da ciò che la sonda ha memorizzato — aprire la pagina non
interroga né la console né i cluster che gestisce:

* **Federated instances** — una striscia di totali del patrimonio
  (istanze, nodi online, guest in esecuzione e fermi, core, memoria,
  storage), poi una riga per istanza federata. Le istanze che la console
  non può raggiungere vengono ordinate prima, poi quelle con task falliti,
  poi quelle indietro di versione. Ogni riga dà il prodotto (Proxmox VE o
  Backup Server), la versione, i conteggi di nodi e guest, l'utilizzo di
  memoria e storage, lo stato dell'abbonamento, l'indirizzo configurato,
  quando la console ha raccolto da essa l'ultima volta — e, quando è
  fallito, il messaggio che la console stessa ha ricevuto. Quel messaggio
  è la sola cosa che spiega l'interruzione, quindi viene mostrato alla
  lettera.
* **Failures** — ogni task fallito in tutto il patrimonio negli ultimi 14
  giorni, il più recente prima, con l'istanza su cui è stato eseguito, il
  tipo di task, quando è iniziato, quanto è durato e il messaggio di
  errore. Il log del task stesso resta sull'istanza: aprilo lì, oppure
  aggiungi quel cluster come proprio monitor, dove *Show log* lo
  recupera.
* **Console host** — la macchina che esegue la console: CPU, memoria,
  file system root, uptime e kernel, i suoi certificati con i giorni
  rimanenti prima della scadenza, i pacchetti in attesa di un
  aggiornamento, e lo stato dell'abbonamento dei nodi gestiti. L'intero
  pannello è opzionale: un token senza `Sys.Audit` non ne legge nulla e il
  pannello lo dice, invece di mostrare zeri.

La cronologia dei task viene mantenuta in SQLite (`pdm_task_history`, 19
giorni, al massimo 5000 task per monitor) così l'elenco dei
fallimenti sopravvive ai riavvii anche se ogni sonda guarda solo alla
finestra dei task.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_pdm_`.

| Famiglia | Metriche | Etichette |
|---|---|---|
| Patrimonio | `remotes_total`, `remotes_failed`, `nodes_online`, `nodes_offline`, `cpu_used_cores`, `cpu_total_cores`, `memory_used_bytes`, `memory_total_bytes`, `memory_used_percent`, `storage_used_bytes`, `storage_total_bytes`, `storage_used_percent`, `datastores_total` | |
| Guest del patrimonio | `guests_running`, `guests_stopped` | `guest_type` (`qemu`, `lxc`) |
| Istanze | `remote_reachable` (1 la console ha ottenuto una risposta, 0 non l'ha ottenuta), `remote_version_info` (serie di presenza, etichetta `version`), `remote_version_behind`, `remote_nodes_online`, `remote_nodes_offline`, `remote_guests_running`, `remote_guests_stopped`, `remote_cpu_used_cores`, `remote_cpu_total_cores`, `remote_cpu_used_percent`, `remote_memory_used_bytes`, `remote_memory_total_bytes`, `remote_memory_used_percent`, `remote_storage_used_bytes`, `remote_storage_total_bytes`, `remote_storage_used_percent`, `remote_datastores`, `remote_subscription_active`, `remote_updates_pending`, `remote_last_collection_age_seconds`, `remote_tasks_failed` | `remote`, `type` (`pve`, `pbs`) |
| Task nella finestra | `tasks_total`, `tasks_failed`, `tasks_running` | |
| Console host | `node_cpu_percent`, `node_cpu_count`, `node_iowait_percent`, `node_load1`, `node_memory_used_bytes`, `node_memory_total_bytes`, `node_memory_used_percent`, `node_swap_used_bytes`, `node_swap_total_bytes`, `node_rootfs_used_bytes`, `node_rootfs_total_bytes`, `node_rootfs_percent`, `node_uptime_seconds`, `node_kernel_info`, `node_updates_pending`, `version_info` | |
| Certificati console | `node_certificate_expiry_days` (negativo una volta scaduto) | `file` (`proxy.pem`) |
| Abbonamento | `subscription_active` (etichetta `status`), `subscription_active_nodes`, `subscription_total_nodes` | |
| Raccolta | `up`, `scrape_errors`, `scrape_duration_seconds` | |

Un task che termina con `WARNINGS: n` conta come riuscito. Solo il
fallimento di `GET /version` condanna l'intera sonda; qualsiasi altra
chiamata fallita viene contata in `scrape_errors`, e un'istanza che la
console non può raggiungere è una misurazione (`remote_reachable = 0`),
non un errore di raccolta.

`remote_version_behind` confronta le istanze federate **tra loro**,
prodotto per prodotto: è 1 quando un altro cluster Proxmox VE — o un
altro backup server — nello stesso patrimonio esegue una versione più
recente. Non è un confronto rispetto a ciò che Proxmox pubblica; una
sonda non ha motivo di raggiungere internet. La serie esiste solo per
un'istanza la cui versione è stata letta, così un'istanza irraggiungibile
non passa mai per aggiornata.

I totali di storage contano ogni nome di storage una volta per istanza:
uno storage condiviso da cinque nodi di un cluster viene contato una
volta, non cinque.

Le regole integrate che si applicano: Monitor unreachable, Federated
instance unreachable, Federated instance behind, Task failed on a
federated instance, Datacenter console disk almost full, Datacenter
console certificate expiring, Datacenter console updates pending. Le
notifiche nominano l'istanza interessata.

## Cosa preparare in Proxmox Datacenter Manager

I passaggi seguenti sono quelli mostrati dalla nota accanto al modulo. Il
principio: un utente riservato al monitoraggio, un token con un ruolo di
sola lettura — mai l'account con cui accedi.

1. Nella console, apri Configuration → Access Control → Users e clicca Add. Dai all'account il nome seguente e lascia vuota la password: il token è ciò che accede.

    ```
    dumbmonit@pdm
    ```

2. Ancora sotto Access Control, apri API Tokens → Add, scegli quell'utente e dai al token il nome "monitor". La console mostra il secret una volta: copialo subito.

    ```
    dumbmonit@pdm!monitor
    ```

3. Dai sia all'utente che al token il ruolo `Auditor` su `/` (la cima dell'albero, quindi l'intero patrimonio). `Auditor` porta `System.Audit`, `Resource.Audit` e `Access.Audit`, e non può modificare nulla. Un token non ha mai più diritti del suo utente, quindi il permesso viene concesso due volte: Permissions → Add → User Permission, poi di nuovo con API Token Permission.
4. Copia l'id del token nel campo Token ID di DumbMonit e il secret nel suo campo Secret.
5. In DumbMonit, inserisci l'indirizzo della console, per esempio "dc.lan" oppure "dc.lan:8443".

!!! warning
    Non riutilizzare l'account con cui accedi: un token compromesso
    controllerebbe allora ogni cluster che la console federa in una sola
    volta. Il ruolo `Auditor` può solo leggere. La console usa anche un
    certificato autofirmato per impostazione predefinita: se la
    connessione viene rifiutata per questo motivo, spunta "Accept an
    unverifiable certificate" nelle opzioni. Due elementi richiedono più
    di `Auditor` e vengono semplicemente saltati senza di esso: lo stato
    di raccolta delle metriche per istanza, e il riepilogo degli
    aggiornamenti delle istanze federate.

Preferisci la riga di comando? `proxmox-datacenter-manager-admin` gestisce
i remote, non gli utenti; crea l'account, il token e i due permessi
nell'interfaccia web.

## Credenziali

| Credenziale | Campi |
|---|---|
| API token (consigliato) | **Token ID**: utente, realm e nome del token come li mostra la console, `dumbmonit@pdm!monitor`. **Secret**: l'UUID mostrato una volta alla creazione del token. DumbMonit li invia come l'header `PDMAPIToken=user@pdm!name:secret` che la console si aspetta. |

Non c'è un'opzione username / password, ed è deliberato: dalla sua prima
release la console restituisce il suo ticket di sessione solo in un
cookie `HttpOnly` — il corpo JSON porta un `ticket-info` inutilizzabile.
Un login non comprerebbe nulla in più rispetto a un token mentre aprirebbe
una sessione autenticata a ogni sonda. I token sono comunque ciò che
Proxmox raccomanda per l'automazione.

Diritti, di sola lettura. Più semplice e sufficiente: il ruolo integrato
`Auditor` su `/`, concesso sia all'utente *che* al token. Per endpoint:

| Endpoint | Richiede |
|---|---|
| `GET /version` | qualsiasi utente autenticato |
| `GET /remotes/remote` | restituisce le istanze che il chiamante può verificare |
| `GET /remotes/remote/{id}/version` | `Resource.Audit` su `/resource/{id}` |
| `GET /resources/status`, `GET /resources/list` | `Resource.Audit` su `/resource` |
| `GET /resources/subscription` | `Resource.Audit` su `/resource` |
| `GET /remotes/tasks/list` | `Resource.Audit` su `/resource/{remote}`, per istanza |
| `GET /nodes/{node}/status` | `System.Audit` su `/system/status` |
| `GET /nodes/{node}/certificates/info`, `GET /nodes/{node}/subscription` | qualsiasi utente autenticato |
| `GET /nodes/{node}/apt/update` | `System.Audit`; un 403 è tollerato: nessuna serie `node_updates_pending`, nessun errore di raccolta |
| `GET /remotes/metric-collection/status` | più di `Auditor`; un 403 è tollerato: nessuna serie `remote_last_collection_age_seconds`, nessun errore di raccolta |
| `GET /remotes/updates/summary` | `Resource.Modify` — un privilegio di scrittura. Disattivato per impostazione predefinita (`remote_updates`) per questo motivo |

Nota che la console redige i token di accesso che mantiene per le sue
istanze (`GET /remotes/remote` restituisce un `token` vuoto), e DumbMonit
non legge mai comunque quel campo: nessun segreto di istanza passa mai
attraverso il collector, la vista memorizzata o l'API.

Indirizzo: `10.0.0.40`, `dc.lan`, `dc.lan:8443`, `[fd00::2]` oppure un URL
completo. Lo schema `https` e la porta 8443 vengono aggiunti se mancanti.

## Opzioni

| Chiave | Etichetta | Predefinito | Aiuto |
|---|---|---|---|
| `port` | API port | `8443` | Usata se l'indirizzo non fornisce una porta. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | Proxmox Datacenter Manager fornisce un certificato autofirmato per impostazione predefinita: abilita questo se la connessione viene rifiutata per questo motivo. |
| `request_timeout_seconds` | Timeout per request (seconds) | `20` | Tempo consentito per ogni chiamata API, da 1 a 120. La console inoltra le chiamate a ogni istanza federata, quindi un sito lento trattiene l'intera risposta. |
| `node` | Console node name | `localhost` | Nome del nodo che esegue la console. Un Datacenter Manager ha un solo nodo e lo chiama "localhost". |
| `task_lookback_hours` | Task window (hours) | `24` | I task più vecchi non vengono contati tra i fallimenti, da 1 a 8760. |
| `max_age_seconds` | Accepted cache age (seconds) | `60` | Quanto obsoleto può essere l'inventario proprio della console prima che interroghi di nuovo le istanze federate, da 0 a 3600. Zero chiede dati freschi a ogni sonda, il che rimette ogni cluster in linea. |
| `remotes` | Monitored instances | *(vuoto)* | Nomi delle istanze federate da monitorare, separati da virgole. Vuoto: ogni istanza. |
| `max_remotes` | Instance version limit | `100` | Numero massimo di istanze interrogate per la loro versione a ogni sonda, da 1 a 1000. |
| `versions` | Read each instance version | `true` | Una chiamata per istanza federata, per riportare la sua versione e segnalare quelle rimaste indietro rispetto ai loro pari. Un'istanza che non risponde conta come irraggiungibile. |
| `tasks` | Watch tasks across instances | `true` | Legge l'elenco dei task di ogni istanza federata per riportare i fallimenti. |
| `node_status` | Watch the console host | `true` | Legge CPU, memoria, file system root, uptime, certificati e abbonamento della macchina che esegue la console. |
| `updates` | Count pending updates | `true` | Elenca i pacchetti in attesa di un aggiornamento sulla console stessa. |
| `remote_updates` | Count updates on each instance | `false` | Legge il riepilogo degli aggiornamenti della console per le istanze federate. Disattivato per impostazione predefinita: la console protegge quel percorso con `Resource.Modify`, un privilegio di scrittura che un account di monitoraggio non ha motivo di possedere. |

`max_age_seconds` è quella che vale la pena capire. La console mantiene il
proprio inventario, aggiornato dal proprio ciclo di raccolta. Chiedere
dati freschi a ogni sonda fa chiamare di nuovo ogni cluster dalla console,
oltre al suo ritmo normale — esattamente il carico che un unico
monitor console dovrebbe evitare. Sessanta secondi sono comunque più
freschi dell'intervallo di sonda predefinito.

## Errori comuni

| Sintomo | Causa probabile |
|---|---|
| Errore di certificato | Certificato autofirmato: installane uno fidato (ACME è integrato nella console) oppure abilita `insecure_tls`. |
| Errore di autenticazione | Il secret è stato incollato in Token ID (o il contrario), oppure il permesso è stato concesso all'utente ma non al token — un token non ha mai più diritti del suo utente, e le due ACL sono voci separate. |
| Ogni istanza appare irraggiungibile, ma la pagina propria della console va bene | Il token ha `Auditor` sull'utente ma non sul token, oppure solo su `/system`: `Resource.Audit` su `/resource` è ciò che permette alla console di rispondere per le sue istanze. |
| Un'istanza è irraggiungibile e il messaggio menziona un'impronta digitale | La console verifica il certificato di ogni istanza che gestisce; correggilo nella console (Remotes → l'istanza), non qui. |
| Nessuna serie `remote_last_collection_age_seconds` | `GET /remotes/metric-collection/status` richiede più di `Auditor` ed è saltato silenziosamente. Nulla altro è interessato. |
| Nessuna serie `remote_updates_pending` | `remote_updates` è disattivato per impostazione predefinita perché quel percorso richiede `Resource.Modify`. Attivalo solo se sei disposto a concedere un privilegio di scrittura. |
| Nessun pannello console host | Al token manca `System.Audit` su `/system/status`, oppure `node_status` è disattivato. |
| "Federated instance behind" non si attiva mai | Confronta istanze dello stesso prodotto tra loro. Una console che federa un unico cluster Proxmox VE non ha nulla con cui confrontarlo. |
| I totali del patrimonio sono tutti trattini | La console ha risposto ma non ha contato nulla — di solito perché ogni istanza è irraggiungibile. Le righe delle istanze dicono perché. |
