# Nutanix AHV

Cluster Nutanix, letti tramite l'API REST v4 di Prism Central. Un Prism
Central copre tutti i cluster registrati su di esso.

Ogni minuto DumbMonit legge, con autenticazione HTTP basic sulla porta 9440:

| Cosa | Percorso |
|---|---|
| Cluster | `/api/clustermgmt/v4.0/config/clusters` |
| Host | `/api/clustermgmt/v4.0/config/hosts` |
| VM AHV | `/api/vmm/v4.0/ahv/config/vms` |
| Storage container | `/api/clustermgmt/v4.0/config/storage-containers` |
| Livello di riempimento di ogni container | `/api/clustermgmt/v4.0/stats/storage-containers/{extId}` |
| Avvisi non risolti | `/api/monitoring/v4.0/serviceability/alerts` |

Gli elenchi vengono letti a 100 elementi per pagina, fino a 1.000 elementi
ciascuno. Il livello di riempimento e gli avvisi sono facoltativi: se il ruolo
dell'account li rifiuta, il resto viene comunque raccolto. Nutanix ha dichiarato
obsolete le API v1, v2.0 e v3; solo la v4 continua a evolvere e richiede
Prism Central pc.2024.3 o successivo.

!!! warning "Validato solo sulla documentazione del fornitore — non ancora testato su un sistema reale"

    Questa integrazione è stata costruita dalle specifiche dell'API v4 di
    Nutanix (percorsi e campi come li definiscono i client generati ufficiali)
    e testata su risposte registrate, non su un Prism Central in funzione.
    Facci sapere cosa non funziona.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_nutanix_`.

| Metrica | Etichette | Significato |
|---|---|---|
| `clusters` | | Cluster, escluso Prism Central stesso. |
| `cluster_info` | `cluster`, `version` | Valore 1: la versione di AOS di ogni cluster. |
| `cluster_available` | `cluster` | 1 quando Prism Central raggiunge il cluster. |
| `hosts` | | Host di tutti i cluster. |
| `host_info` | `host`, `cluster`, `hypervisor` | Valore 1: l'hypervisor che esegue ogni host. |
| `host_cpu_cores`, `host_memory_bytes` | `host`, `cluster` | Capacità. |
| `host_degraded` | `host`, `cluster` | 1 quando Nutanix segna l'host come degradato. |
| `host_maintenance` | `host`, `cluster` | 1 quando l'host sta entrando in modalità di manutenzione o ci si trova. |
| `vms`, `vms_running` | | VM AHV, e quante sono accese. |
| `vm_power_state_info` | `vm`, `cluster`, `state` | Valore 1: `on`, `off`, `paused` o `undetermined`. |
| `vm_running` | `vm`, `cluster` | 1 quando la VM è accesa. |
| `vm_vcpus`, `vm_memory_bytes` | `vm`, `cluster` | Socket × core × thread, e memoria configurata. |
| `storage_containers` | | Storage container, esclusi quelli interni di Nutanix. |
| `storage_container_capacity_bytes`, `storage_container_used_bytes`, `storage_container_used_percent` | `container`, `cluster` | Livello di riempimento, dalle ultime statistiche degli ultimi trenta minuti. |
| `alerts_unresolved`, `alerts_critical`, `alerts_warning` | | Avvisi non risolti, tutti e per gravità. |

Le [regole integrate](../alerting/rules.md#nutanix) che si applicano:

- **Host Nutanix degradato**: un host è segnato come degradato da cinque
  minuti (Avvertenza).
- **Storage container Nutanix quasi pieno**: un container è pieno al 90% per
  quindici minuti (Avviso).
- **Avvisi critici Nutanix**: Prism Central segnala almeno un avviso critico
  non risolto da cinque minuti (Avvertenza).

## Creare un utente di sola lettura in Prism Central

1. In Prism Central, apri Admin Center > IAM > Identities e aggiungi un utente locale con una password lunga e casuale, chiamato come segue.

    ```
    dumbmonit
    ```

2. In Admin Center > IAM > Authorization Policies, crea una policy con il ruolo integrato Prism Viewer, per tutte le entità, e assegnala a quell'utente. Prism Viewer legge cluster, host, VM, storage container e avvisi, e non modifica nulla.

3. In DumbMonit, inserisci l'indirizzo di Prism Central, per esempio "prism-central.lan", con quel nome utente e quella password. Un Prism Central segnala tutti i cluster registrati su di esso.

4. DumbMonit si limita a leggere l'API v4 (clustermgmt, vmm e monitoring, versione 4.0): non accende né spegne mai una VM, non risolve mai un avviso e non modifica mai uno storage container.

!!! warning
    L'API v4 è servita da Prism Central pc.2024.3 o successivo; un cluster gestito solo da Prism Element, o da un Prism Central più vecchio, non è supportato. Prism Central include un certificato autofirmato: spunta Accept an unverifiable certificate a meno che l'host DumbMonit non si fidi della sua autorità di certificazione.

## Credenziali

| Credenziale | Campi |
|---|---|
| Nome utente e password | L'utente locale `dumbmonit` con il ruolo Prism Viewer. |

Indirizzo: il nome host o l'IP di Prism Central, `host:port`, oppure un URL.
Opzioni:

- **Port** (9440).
- **Accept an unverifiable certificate**: il certificato proprio di Prism
  Central è autofirmato.
- **Timeout per richiesta** (15 s).

## Risoluzione dei problemi

"Nutanix Prism Central refused the credentials … (401)": nome utente o
password errati.

"Nutanix Prism Central has no /api/clustermgmt/v4.0/…": l'indirizzo è quello
di un Prism Element, o di un Prism Central precedente a pc.2024.3.

Nessun `storage_container_used_percent`: il ruolo non consente di leggere le
statistiche; i container vengono comunque contati, con la loro capacità
configurata.
