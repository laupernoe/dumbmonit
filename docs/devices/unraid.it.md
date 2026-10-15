# Unraid

Un server Unraid tramite la sua API GraphQL: un array non avviato, dischi
disattivati, mancanti o che segnalano errori di lettura, controlli di
parità che hanno trovato errori o non vengono eseguiti da settimane, cache
pool che si riempiono, container che dovrebbero essere in esecuzione e non
lo sono, e lo stato delle VM.

DumbMonit invia una chiave API con il ruolo Viewer nell'header
`x-api-key` a `/graphql`. Il ruolo Viewer legge tutto e non può modificare
nulla. L'API è integrata in Unraid dalla 7.2; su 6.12-7.1 viene con il
plugin Unraid Connect.

Ogni minuto DumbMonit invia una query obbligatoria (versione, array,
parità, dischi dati e cache) e quattro opzionali, ognuna per conto proprio
così che un campo che una versione più vecchia dell'API non ha, o VM
spente, lasciano fuori solo quella parte: il controllo di parità, i
container Docker, le VM e le notifiche non lette.

!!! warning "Validato solo sulla documentazione del fornitore — non ancora testato su un sistema reale"

    Questa integrazione è stata costruita dalla documentazione API di
    Unraid e dallo schema GraphQL pubblicato dell'API Unraid (nomi dei
    campi, enumerazioni, dimensioni in kilobyte), non su un server in
    funzione. Ogni query è una lettura. Facci sapere cosa non funziona.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_unraid_`.

| Metrica | Etichette | Significato |
|---|---|---|
| `version_info` | `version` | Valore 1: la versione di Unraid. |
| `array_started` | | 1 quando l'array è avviato. |
| `array_state_info` | `state` | Valore 1: `started`, `stopped`, `new_array`, `recon_disk`, `disable_disk`… |
| `array_size_bytes`, `array_used_bytes`, `array_free_bytes`, `array_used_percent` | | Capacità dell'array. |
| `disk_ok` | `disk`, `role` (`parity`, `data`, `cache`) | 1 quando lo stato del disco è `DISK_OK`. Gli slot vuoti vengono esclusi. |
| `disk_status_info` | `disk`, `role`, `status` | Valore 1: lo stato proprio di Unraid (`disk_ok`, `disk_dsbl`, `disk_np_missing`, `disk_invalid`…). |
| `disk_errors` | `disk`, `role` | Errori di lettura da quando l'array è stato avviato (la colonna Errors). |
| `disk_temperature_celsius` | `disk`, `role` | Solo mentre il disco gira: Unraid non riporta nulla per un disco in sospensione. |
| `disk_spinning` | `disk`, `role` | 1 quando il disco gira. |
| `disk_size_bytes`, `disk_fs_size_bytes`, `disk_used_percent` | `disk`, `role` | Dimensione, dimensione del file system e riempimento, per dischi con un file system. |
| `disks`, `disks_problem`, `disks_with_errors` | | Conteggi. |
| `parity_check_status_info` | `status` | Valore 1: `completed`, `running`, `paused`, `cancelled`, `failed`, `never_run`. |
| `parity_check_running`, `parity_check_progress_percent` | | Un controllo in corso. |
| `parity_check_ok` | | Dopo un controllo terminato: 1 senza errori di sincronizzazione, 0 con errori o quando è fallito. Un controllo annullato non prova nulla e non imposta nulla. |
| `parity_check_errors`, `parity_check_age_seconds`, `parity_check_duration_seconds` | | L'ultimo controllo. |
| `containers`, `containers_running`, `containers_autostart_stopped` | | Container Docker. |
| `container_running`, `container_autostart_stopped` | `container` | Per container; il secondo è 1 quando un container impostato per avviarsi con l'array non è in esecuzione. |
| `vms`, `vms_running` | | VM. |
| `vm_state_info`, `vm_crashed` | `vm` (e `state`) | Per VM. |
| `notifications_unread` | `importance`: `alert`, `warning`, `info` | Notifiche non lette. |

Le [regole integrate](../alerting/rules.md#unraid) che si applicano:

- **Unraid array stopped**: l'array non è avviato per dieci minuti
  (Warning).
- **Unraid disk disabled or missing**: lo stato di un disco non è
  `DISK_OK` per cinque minuti (Warning). L'array funziona in modo
  degradato.
- **Unraid disk read errors**: un disco segnala errori di lettura
  (Advisory).
- **Unraid disk too hot**: un disco dati o parità sopra i 55 °C per
  quindici minuti (Advisory). I dischi cache, spesso NVMe, vengono
  esclusi.
- **Unraid parity check found errors** (Advisory).
- **Unraid parity check overdue**: l'ultimo controllo ha più di 40 giorni
  (Advisory).
- **Unraid cache nearly full**: un disco cache più del 90% pieno per
  trenta minuti (Advisory).
- **Unraid container stopped**: un container impostato per avviarsi con
  l'array non è in esecuzione da dieci minuti (Advisory).

## La pagina del monitor

Il pannello Array indica cosa non va in una frase, mostra il riempimento
dell'array, l'ultimo controllo di parità, i container e le VM in
esecuzione, poi elenca ogni disco — parità prima, poi dati, poi cache —
con il suo stato, temperatura e riempimento, e i container che dovrebbero
essere in esecuzione.

## Creare una chiave API di sola lettura per DumbMonit

1. Su Unraid 7.2 o successivo l'API è integrata. Su Unraid 6.12-7.1, installa prima il plugin Unraid Connect dalla scheda Apps: porta la stessa API, e nessun login Unraid Connect è necessario per l'uso locale.

2. Crea una chiave API con il ruolo Viewer, che legge tutto e non può modificare nulla: sotto Settings > Management Access > API Keys, oppure dal terminale Unraid.

    ```
    unraid-api apikey --create --name dumbmonit --roles VIEWER --description "DumbMonit monitoring" --json
    ```

3. Copia la chiave. In DumbMonit, inserisci l'indirizzo del server Unraid, per esempio "tower.lan", e incolla la chiave come API key. Se l'interfaccia web è servita su HTTPS, scegli https come protocollo.

!!! warning
    Unraid non riporta alcuna temperatura per un disco fermato, e DumbMonit non ne risveglia mai uno: un disco in sospensione semplicemente non ha temperatura finché non gira di nuovo.

## Credenziali

| Credenziale | Campi |
|---|---|
| API key | Una chiave con il ruolo Viewer, inviata come `x-api-key`. |

Indirizzo: il nome host o IP del server Unraid, `host:porta`, oppure un
URL. Opzioni:

- **Protocol** (`http`) e **Port** (80): quelli dell'interfaccia web. Con
  SSL impostato su Strict sotto Settings > Management Access, usa `https`,
  e l'indirizzo `myunraid.net` se il certificato è quello.
- **Accept an unverifiable certificate**: per un certificato autofirmato.
- **Timeout per request** (15 s).

## Risoluzione dei problemi

"Unraid refused the API key (401)": la chiave è errata o è stata
eliminata.

"Unraid accepted the API key but refused to answer (403)": la chiave non
ha ruolo, o un ruolo più limitato di Viewer.

Nessun valore di container, VM o parità: quella query opzionale è fallita
(VM spente, una versione più vecchia dell'API senza quel campo); il resto
viene comunque letto. Il motivo è nel log del server a livello debug.
