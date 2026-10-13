# Veeam Backup & Replication

Un server Veeam Backup & Replication attraverso la sua API REST: lavori il cui ultimo
esecuzione è fallita o terminata con un avviso, sessioni non riuscite degli ultimi 24 ore,
repository che si riempiono, la licenza, e — per computer o VM protetto —
il suo ultimo backup riuscito.

DumbMonit accede con un account Windows che detiene solo il ruolo Veeam Backup
Viewer (OAuth2 password grant su `/api/oauth2/token`, porta 9419) e
mantiene il token per la sua durata di quindici minuti. Ogni chiamata porta l'intestazione
obbligatoria `x-api-version`, `1.1-rev0` per impostazione predefinita: la revisione che
è arrivata con la versione 12.0.

Gli stati dei lavori (`/api/v1/jobs/states`) sono la lettura richiesta. Gli altri —
`/api/v1/serverInfo`, `/api/v1/sessions` (creati negli ultimi 24 ore),
`/api/v1/backupInfrastructure/repositories/states`, `/api/v1/license` e
`/api/v1/restorePoints` (punti di ripristino di ogni oggetto protetto, a livello di server,
più recenti per primi) — vengono letti quando il ruolo li consente. Veeam mantiene alcune route
per il ruolo Backup Administrator, la licenza tra loro su versioni attuali:
un rifiuto lì viene mostrato come "non leggibile", mai come un errore. Un
server troppo vecchio per conoscere `/api/v1/restorePoints` — la route è apparsa dopo
12.0 — è trattato allo stesso modo: nessun dispositivo, nessun errore.

!!! warning "Convalidato solo rispetto alla documentazione del fornitore — non ancora testato su un sistema reale"

    Questa integrazione è stata costruita dal riferimento API REST di Veeam e dal centro assistenza
    (autenticazione, controllo delle versioni, il lavoro, la sessione, il repository,
    modelli di licenza e punto di ripristino), non rispetto a un server di backup in esecuzione.
    Quali route il ruolo Viewer può leggere varia a seconda della versione: raccontaci
    cosa il tuo server rifiuta.

## Cosa monitora

Tutte le metriche hanno il prefisso `dumbmonit_veeam_`.

| Metrica | Etichette | Significato |
|---|---|---|
| `version_info` | `version` | Valore 1: la build, `12.3.1.1139`. |
| `job_enabled`, `job_running` | `job`, `type` | Per lavoro. |
| `job_failed`, `job_warning` | `job`, `type` | 1 quando l'ultimo esecuzione di un lavoro abilitato è fallito, o terminato con un avviso. Un lavoro disabilitato mantiene il suo ultimo risultato, che non dice più nulla: non conta come nessuno. |
| `job_result_info` | `job`, `type`, `result` | Valore 1: `success`, `warning`, `failed`, `none`. |
| `job_last_run_age_seconds` | `job`, `type` | Da quando è stata eseguita l'ultima volta; assente per un lavoro che non è mai stato eseguito. |
| `job_progress_percent` | `job`, `type` | Mentre il lavoro è in esecuzione. |
| `jobs`, `jobs_failed`, `jobs_warning`, `jobs_running`, `jobs_disabled` | | Conteggi; al massimo 200 lavori vengono descritti uno per uno. |
| `sessions_24h`, `sessions_failed_24h`, `sessions_warning_24h` | | Sessioni create negli ultimi 24 ore, per risultato. |
| `repository_capacity_bytes`, `repository_free_bytes`, `repository_used_bytes`, `repository_used_percent` | `repository`, `type` | Per repository. Un repository di archivio di oggetti non ha capacità: solo lo spazio utilizzato viene pubblicato. |
| `license_valid` | | 1 quando lo stato della licenza è `Valid`. |
| `license_info` | `status`, `type`, `edition` | Valore 1. |
| `license_expiry_seconds`, `support_expiry_seconds` | | Fino alla scadenza della licenza e del contratto di supporto; assente per una licenza perpetua. |
| `license_instances_licensed`, `license_instances_used` | | Utilizzo della licenza dell'istanza. |
| `section_readable` | `section`: `sessions`, `repositories`, `license`, `restore_points` | 1 quando l'account può leggerla, 0 quando Veeam l'ha rifiutata. |

Le [regole integrate](../alerting/rules.md#veeam-backup--replication) che si applicano:

- **Lavoro Veeam non riuscito**: l'ultimo esecuzione di un lavoro abilitato è fallito (Avviso,
  ricordato quotidianamente).
- **Lavoro Veeam terminato con un avviso** (Avviso, ricordato quotidianamente).
- **Repository Veeam quasi pieno**: più del 90% per trenta minuti
  (Avviso).
- **Licenza Veeam in scadenza**: meno di 30 giorni rimasti (Avviso, ricordato
  quotidianamente). Solo quando l'account può leggere la licenza.
- **Dispositivo client obsoleto**: un computer protetto o VM è andato oltre
  `device_stale_days` senza un backup riuscito (Avviso, ricordato quotidianamente);
  vedi [Dispositivi](#dispositivi) sotto.

## Dispositivi

Ogni oggetto con almeno un punto di ripristino — una VM, un computer protetto agente fisico o virtuale,
una condivisione di file — viene pubblicato sotto la famiglia generica condivisa da ogni
integrazione DumbMonit che traccia i dispositivi protetti —
`dumbmonit_client_device_*`, prefisso senza `veeam`, documentato pienamente
sotto [Dispositivi client](../alerting/rules.md#client-devices):

| Metrica | Cosa | Etichette |
|---|---|---|
| `last_backup_timestamp_seconds` | Il punto di ripristino più recente dell'oggetto | `device`, `type`, `os`, `user`, `kind` |
| `stale_seconds` | Positivo una volta che un oggetto è andato oltre `device_stale_days` (3 per impostazione predefinita) senza un nuovo punto di ripristino; guida la regola [Client device stale](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (sempre `backup`) |

**Limiti degni di nota**: un lavoro protegge i suoi oggetti come gruppo
(`jobs/states` fornisce solo un conteggio, `objectsCount`); la data per oggetto viene da
`/api/v1/restorePoints` invece, letto per l'intero server in una chiamata.
Veeam crea un punto di ripristino solo una volta che un backup ha avuto almeno un successo parziale, quindi il più recente per oggetto è già "l'ultimo backup riuscito" — nulla è
filtrato sul risultato qui. Veeam non espone un
"ultimo collegamento" separato dal backup stesso, quindi `last_seen_timestamp_seconds`
e il segnale `connection` non vengono mai pubblicati; un dispositivo mostra
`last_connection` come "—" nella tabella della pagina del dispositivo. Il `type` del dispositivo è
il nome della piattaforma di Veeam (`VSphere`, `WindowsPhysical`, `HyperV`…), mostrato così
com'è.

## La pagina del dispositivo

Il pannello Backup dice cosa è sbagliato in una frase, mostra i lavori non riusciti, il
fallimento delle sessioni degli ultimi 24 ore, i lavori in esecuzione e la licenza, quindi
elenca i lavori abilitati — guasti per primi — con il loro tipo e ultimo esecuzione, e
i repository con il loro riempimento e spazio libero. La tabella dei dispositivi protetti
(nome, piattaforma, ultimo backup, stato) segue, dalla famiglia generica sopra.

## Fornisci a DumbMonit un account Veeam Backup Viewer

1. Nel server di backup, crea un account Windows locale per DumbMonit con una lunga password casuale e nessun altro diritto sulla macchina (un account di dominio funziona anche).

    ```
    net user dumbmonit * /add
    ```

2. Ancora nella console Veeam, apri il menu principale > Utenti e ruoli > Sicurezza, fai clic su Aggiungi, inserisci l'account (ad esempio VBR01\dumbmonit) e assegnagli solo il ruolo Veeam Backup Viewer.

3. In DumbMonit, inserisci l'indirizzo del server di backup, ad esempio "vbr.lan", e l'account come HOST\dumbmonit o DOMAIN\dumbmonit con la sua password. L'API REST ascolta sulla porta 9419 con un certificato autofirmato: spunta Accetta un certificato non verificabile a meno che non ne abbia installato uno proprio.

!!! warning
    Veeam mantiene alcune route API REST per il ruolo Backup Administrator, la licenza tra loro su versioni attuali. Con il ruolo Viewer DumbMonit legge i lavori, le sessioni, i repository e i punti di ripristino, e mostra la licenza come non leggibile: non chiede mai di più.

## Credenziali

| Credenziale | Campi |
|---|---|
| Nome utente e password | L'account con il ruolo Veeam Backup Viewer, come `HOST\dumbmonit` o `DOMAIN\dumbmonit`. |

Indirizzo: il nome host o l'IP del server di backup, `host:port`, o un URL.
Opzioni:

- **Protocollo** (`https`) e **Porta** (9419): del servizio API REST.
- **Versione API** (`1.1-rev0`): inviata come `x-api-version`. Cambiala solo se
  il server la rifiuta; un server v13 documenta `1.3-rev1`.
- **Accetta un certificato non verificabile**: il certificato dell'API REST è
  autofirmato.
- **Timeout per richiesta** (15 s).
- **Obsolescenza del dispositivo** (`device_stale_days`, 3 giorni per impostazione predefinita): vedi
  [Dispositivi](#dispositivi) sopra.

## Risoluzione dei problemi

"Veeam ha rifiutato l'accesso": il nome utente ha bisogno del suo host o dominio
(`VBR01\dumbmonit`), o l'account non ha un ruolo Veeam.

"Veeam ha rifiutato di elencare i lavori (403)": l'account ha un ruolo che non può
leggere i lavori; assegnagli Veeam Backup Viewer.

"Veeam ha rifiutato la versione API": imposta l'opzione versione API su quella che il server
elenca su `https://<server>:9419/swagger`.

La licenza mostra "Non leggibile": previsto con il ruolo Viewer su versioni attuali;
DumbMonit non chiede di più.

Nessun dispositivo, o il pannello Dispositivi è vuoto: `restore_points` legge "Non
leggibile", probabilmente perché il server è più vecchio della route (qualsiasi cosa prima dei
revisionamenti successivi di 12.0) piuttosto che un problema di ruolo — i punti di ripristino sono
inventario ordinario, non una pagina solo amministratore.
