# Immich

Se Immich sta al passo con ciò che viene caricato: code di job che si
bloccano o restano in pausa, aggiornamenti, il disco che contiene la
libreria, quante foto e video contiene, e — per ogni telefono, tablet e
desktop collegato all'account — la sua ultima connessione e il suo ultimo
backup.

DumbMonit legge l'API di Immich con una **chiave API limitata a otto
permessi di lettura**. La chiave non può vedere una foto, un album o una
persona, e non può modificare nulla.

Il guasto da tenere d'occhio: **job che non funzionano più**. Una coda
sospesa durante una grande importazione e dimenticata, oppure un worker di
job fermo, e le nuove foto non ricevono miniatura, metadati, volti né
ricerca intelligente, mentre i caricamenti continuano a riuscire. Immich lo
mostra solo nella sua pagina Jobs.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_immich_`.

| Metrica | Cosa | Etichette |
|---|---|---|
| `version_info` | valore 1 | `version` |
| `update_available` | 1 quando il controllo versione proprio di Immich conosce una release più recente (assente quando il controllo è disattivato) | `latest_version` |
| `maintenance_mode` | 1 mentre Immich è in modalità manutenzione | |
| `storage_used_percent`, `storage_available_bytes`, `storage_size_bytes` | il disco che contiene la libreria | |
| `photos`, `videos`, `usage_bytes`, `users` | dimensione della libreria, tutti gli utenti insieme | |
| `jobs_waiting`, `jobs_active`, `jobs_failed`, `queues_paused` | tutte le code insieme | |
| `queue_waiting`, `queue_active`, `queue_failed`, `queue_paused` | per coda: in attesa (inclusi ritardati e bloccati da una pausa), in esecuzione, falliti, in pausa | `queue` |
| `statistics_readable` | 0 quando la chiave appartiene a un account ordinario e conteggi e code vengono saltati | |
| `scrape_errors` | chiamate fallite nell'ultima sonda | |

Le versioni recenti di Immich rimuovono i job falliti dalla coda una volta
registrati, quindi `queue_failed` di solito legge 0; un job fallito appare
nel log proprio di Immich.

Le [regole integrate](../alerting/rules.md#self-hosted-applications) che si
applicano: Immich jobs stalled, Immich queue paused, Immich storage almost
full, Immich update available, più Device unreachable.

## Dispositivi

Ogni dispositivo collegato all'account che possiede la chiave, con la sua
ultima connessione e il suo ultimo backup, viene pubblicato sotto una
famiglia generica condivisa da ogni integrazione DumbMonit che traccia i
dispositivi client — `dumbmonit_client_device_*`, con prefisso senza
`immich`, documentata per intero sotto
[Dispositivi client](../alerting/rules.md#client-devices):

| Metrica | Cosa | Etichette |
|---|---|---|
| `last_seen_timestamp_seconds` | Ultima connessione (l'ultima attività della sessione) | `device`, `type`, `os`, `user`, `kind` |
| `last_backup_timestamp_seconds` | Ultimo backup; vedi il limite sotto | `device`, `type`, `os`, `user`, `kind` |
| `stale_seconds` | Positivo una volta che un dispositivo ha superato `device_stale_days` (3 per impostazione predefinita) senza connettersi o fare backup; guida la regola [Client device stale](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (`connection` o `backup`) |

**Limite da conoscere**: l'API di Immich non registra quale dispositivo ha
caricato una determinata foto, quindi `last_backup_timestamp_seconds` è
l'ultimo caricamento più recente dell'intera libreria — lo stesso valore
per ogni dispositivo, pubblicato sotto un dispositivo chiamato `library`
invece che uno reale. Rileva ogni dispositivo che si è fermato in blocco
(storage pieno, l'account disconnesso ovunque); non rileverà un singolo
dispositivo che diventa silenzioso mentre un altro continua a caricare.
Solo `last_seen_timestamp_seconds` — l'ultima volta che la sessione di
ciascun dispositivo era attiva — è autenticamente per dispositivo.

La tabella della pagina del dispositivo mostra queste cifre per ogni
dispositivo, con uno stato OK o Stale.

## La pagina del dispositivo

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire la
pagina non chiama mai Immich. Indica prima cosa non va (code bloccate, code
in pausa, manutenzione), poi gli aggiornamenti, poi foto, video, dimensione
della libreria, disco usato e job in attesa e in esecuzione, poi le code
che hanno lavoro, per nome, poi la tabella dei dispositivi (nome, tipo, OS,
ultima connessione, ultimo backup, stato).

## Creare una chiave API di sola lettura in Immich

1. Accedi a Immich con un account che possa aprire Administration: Immich lascia leggere i conteggi delle foto e le code di job solo a quegli account. Apri Account Settings → API Keys → New API Key, dai il nome seguente, e spunta questi otto permessi: i primi cinque per code di job, conteggi e versione, gli ultimi tre per elencare i dispositivi dell'account e la loro ultima connessione e backup.

    ```
    dumbmonit
    server.about server.versionCheck server.storage server.statistics queue.read session.read asset.read user.read
    ```

2. Copia la chiave subito: Immich la mostra solo una volta. Legge quelle otto pagine e nient'altro: nessuna foto, nessun album, nessuna impostazione. Se il tuo Immich non offre queue.read, spunta job.read invece.

3. Una chiave creata da un account ordinario funziona anch'essa: versione, aggiornamenti, spazio su disco e dispositivi vengono letti, i conteggi delle foto e le code di job vengono saltati.

4. In DumbMonit, inserisci l'indirizzo di Immich, per esempio "immich.lan" (porta 2283) oppure "https://photos.example.com" dietro un reverse proxy, e incolla la chiave.

!!! warning
    Una coda in pausa è facile da dimenticare: le nuove foto non ricevono miniatura, metadati né volti, e Immich lo dice solo nella sua pagina Jobs. DumbMonit avvisa quando i job attendono in una coda che non è in pausa e non esegue nulla. session.read, asset.read e user.read elencano ogni dispositivo dell'account che possiede la chiave — i suoi telefoni, tablet e browser — con l'ultima connessione di ciascuno. L'API di Immich non registra quale dispositivo ha caricato una determinata foto, quindi "last backup" è l'ultimo caricamento più recente dell'intera libreria, mostrato identico per ogni dispositivo: se solo un dispositivo smette di fare backup mentre gli altri continuano, la sua connessione apparirà obsoleta ma il valore di backup condiviso sembrerà ancora recente.

Cosa apre ciascun permesso: `server.about` la versione
(`/api/server/about`), `server.versionCheck` l'ultima release che Immich
conosce, `server.storage` il disco, `server.statistics` i conteggi di foto
e video, `queue.read` le code di job (`/api/queues`; le versioni più
vecchie rispondono su `/api/jobs` con `job.read`), `session.read` i
dispositivi dell'account (`/api/sessions`), `asset.read` il caricamento più
recente della libreria (`/api/search/metadata`), `user.read` il nome
visualizzato dell'account (`/api/users/me`).

## Credenziali

| Credenziale | Campi |
|---|---|
| API key | Inviata nell'header `x-api-key`. |

Indirizzo: un nome host (`immich.lan`), `host:porta`, oppure un URL
completo con un prefisso di percorso; un `/api` finale viene accettato.
Opzioni: protocollo (HTTP per impostazione predefinita), porta (2283),
verifica del certificato, timeout della richiesta (15 s), e l'obsolescenza
del dispositivo (`device_stale_days`, 3 giorni per impostazione
predefinita).

Validato su Immich v3.2.4 (immagini ufficiali, machine learning
disattivato); gli endpoint dei dispositivi sono stati verificati contro il
tag `v3.2.4` della specifica OpenAPI propria di Immich
(`immich-app/immich`), che ha confermato che `/api/search/metadata` non
accetta più un filtro `deviceId` — la ragione per cui "last backup" è
relativo a tutta la libreria invece che per dispositivo.
