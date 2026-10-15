# XCP-ng / XenServer

Un pool XCP-ng o XenServer (Citrix Hypervisor), letto tramite XAPI, l'API di
gestione che ogni host esegue già. Non serve Xen Orchestra.

Ogni minuto DumbMonit invia chiamate JSON-RPC a
`https://<coordinatore>/jsonrpc`: un `session.login_with_password` (la
sessione viene poi mantenuta da un polling al successivo e riaperta una volta
se il pool la invalida) e un `get_all_records` per classe: `host`,
`host_metrics`, `VM`, `VM_metrics`, `SR` e `pool`. Non viene mai modificato
nulla: l'account ha il ruolo integrato `read-only`, che XAPI applica a ogni
chiamata.

Aggiungi il pool una sola volta, con l'indirizzo del suo coordinatore: segnala
tutti gli host, le VM e gli storage repository. Se l'indirizzo che hai
inserito è quello di un membro del pool, questo risponde `HOST_IS_SLAVE` con
l'indirizzo del coordinatore; DumbMonit lo segue una volta, con lo stesso
protocollo e la stessa porta, così un cambio di coordinatore non ti obbliga a
modificare il monitor.

!!! warning "Validato solo sulla documentazione del fornitore — non ancora testato su un sistema reale"

    Questa integrazione è stata costruita dalla documentazione di XAPI (il
    protocollo JSON-RPC, le classi `host`, `VM`, `SR` e `pool`) e testata su
    risposte registrate, non su un pool in funzione. Facci sapere cosa non
    funziona.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_xcpng_`.

| Metrica | Etichette | Significato |
|---|---|---|
| `hosts`, `hosts_enabled` | | Host del pool, e quanti accettano nuove VM. |
| `host_info` | `host`, `version` | Valore 1: la versione del prodotto di ogni host. |
| `host_enabled` | `host` | 0 quando l'host è disabilitato (manutenzione o evacuato). |
| `host_live` | `host` | 1 quando l'heartbeat dell'host raggiunge il coordinatore, 0 quando è offline. |
| `host_cpu_cores`, `host_memory_total_bytes`, `host_memory_used_bytes` | `host` | Capacità e memoria in uso. |
| `vms`, `vms_running` | | VM, esclusi template, snapshot e domini di controllo. |
| `vm_power_state_info` | `vm`, `state` | Valore 1: `running`, `halted`, `paused` o `suspended`. |
| `vm_running` | `vm` | 1 quando la VM è in esecuzione. |
| `vm_vcpus`, `vm_memory_bytes` | `vm` | vCPU all'avvio; memoria in uso per una VM in esecuzione, altrimenti il massimo configurato. |
| `vm_ha_protected_down` | `vm` | Solo per le VM con una priorità di riavvio, in un pool con alta disponibilità attiva: 1 quando la VM è ferma. |
| `srs` | | Storage repository. |
| `sr_size_bytes`, `sr_used_bytes`, `sr_used_percent` | `sr` | Livello di riempimento (le librerie ISO sono contate, non dettagliate). |
| `sr_shared` | `sr` | 1 per un repository condiviso (NFS, iSCSI…). |
| `pool_info` | `pool`, `master` | Valore 1: il pool e il suo coordinatore attuale. |
| `pool_ha_enabled` | `pool` | 1 quando l'alta disponibilità è attiva. |
| `pool_ha_overcommitted` | `pool` | Con la HA attiva: 1 quando il pool non può più riavviare le sue VM protette dopo i guasti di host che è configurato per tollerare. |
| `pool_ha_host_failures_to_tolerate` | `pool` | Il numero di guasti di host che la HA è configurata per sopportare. |

XenAPI non ha uno stato di alimentazione «in crash». Una VM protetta trovata
ferma è il segnale più vicino che l'API fornisce di una VM che si è fermata
contro la tua volontà; se spegni di proposito una VM protetta, rimuovi prima
la sua priorità di riavvio.

Le [regole integrate](../alerting/rules.md#xcp-ng-xenserver) che si applicano:

- **Host XCP-ng offline**: l'heartbeat di un host non raggiunge il
  coordinatore da cinque minuti (Avvertenza).
- **SR XCP-ng quasi pieno**: uno storage repository è pieno al 90% per
  quindici minuti (Avviso).
- **VM XCP-ng protetta da HA ferma**: una VM protetta dall'alta disponibilità
  è ferma da due minuti (Avvertenza).
- **HA del pool XCP-ng in overcommit**: il pool è in overcommit da cinque
  minuti (Avviso).

## Dare a DumbMonit un account XAPI di sola lettura

1. XAPI accetta account diversi dal suo superutente integrato solo quando il pool usa l'autenticazione esterna. Se il pool è già unito a Active Directory, crea lì un account dumbmonit e passa al punto 3.

2. Altrimenti, crea un account locale con una password lunga e casuale su ogni host del pool, poi attiva una volta l'autenticazione PAM, dal coordinatore del pool. Finché non gli viene assegnato un ruolo, l'account non può chiamare nulla.

    ```
    useradd -M -s /sbin/nologin dumbmonit
    passwd dumbmonit
    xe pool-enable-external-auth auth-type=PAM service-name=pam
    ```

3. Sul coordinatore del pool, registra l'account come soggetto, poi assegna a quel soggetto il ruolo integrato read-only, con l'UUID stampato dal primo comando.

    ```
    xe subject-add subject-name=dumbmonit
    xe subject-role-add uuid=SUBJECT_UUID role-name=read-only
    ```

4. In DumbMonit, inserisci l'indirizzo del coordinatore del pool, per esempio "xcp01.lan", con quel nome utente e quella password. Aggiungi il pool una sola volta: il coordinatore segnala tutti gli host, le VM e gli storage repository. Un membro del pool risponde con l'indirizzo del coordinatore, che DumbMonit poi segue.

5. DumbMonit si limita a leggere, tramite l'interfaccia JSON-RPC di XAPI (/jsonrpc): non avvia, non ferma né migra mai una VM e non modifica mai uno storage repository né l'alta disponibilità del pool.

!!! warning
    Non dare mai a DumbMonit la password di superutente degli host: con il ruolo read-only, una password trapelata non può modificare nulla. XCP-ng e XenServer includono un certificato autofirmato: spunta Accept an unverifiable certificate a meno che l'host DumbMonit non si fidi del certificato presentato dal pool.

## Credenziali

| Credenziale | Campi |
|---|---|
| Nome utente e password | L'account `dumbmonit`, registrato come soggetto con il ruolo `read-only`. |

Indirizzo: il nome host o l'IP del coordinatore del pool, `host:port`, oppure
un URL. Opzioni:

- **Port** (443).
- **Accept an unverifiable certificate**: il certificato proprio degli host è
  autofirmato.
- **Timeout per richiesta** (15 s).

## Risoluzione dei problemi

"XCP-ng refused the user name or password": password errata, oppure l'account
non è registrato come soggetto (`xe subject-list`).

"XCP-ng accepted the account but refused the call": il soggetto non ha ruolo,
o ha un ruolo senza accesso in lettura; assegnagli `read-only`.

"This host is a pool member, not the coordinator": DumbMonit non ha potuto
raggiungere il coordinatore indicato dal membro; inserisci tu stesso
l'indirizzo del coordinatore.
