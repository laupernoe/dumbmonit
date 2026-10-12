# Jellyfin

Se Jellyfin mantiene le sue librerie aggiornate e i suoi spettatori serviti:
task pianificati che sono falliti alla loro ultima esecuzione (prima la
scansione della libreria), plugin che non si caricano più, un riavvio in
attesa, e cosa viene riprodotto in questo momento, transcodificato o no.

DumbMonit legge l'API di Jellyfin con una **API key** creata nella dashboard.
Non avvia mai un task, non interrompe mai una riproduzione e non modifica mai
un'impostazione.

Il guasto da tenere d'occhio: **la scansione della libreria che fallisce ogni
notte**. Una condivisione di rete smontata, un disco pieno, e nuovi film ed
episodi smettono di apparire; Jellyfin stesso continua a riprodurre ciò che
già conosce, e la sola traccia è una riga in Scheduled Tasks.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_jellyfin_`.

| Metrica | Cosa | Etichette |
|---|---|---|
| `version_info` | valore 1 | `version` |
| `pending_restart` | 1 quando Jellyfin attende un riavvio (dopo l'installazione o l'aggiornamento di un plugin) | |
| `shutting_down` | 1 mentre Jellyfin si sta spegnendo | |
| `scheduled_tasks_failed`, `scheduled_tasks_running` | task la cui ultima esecuzione è fallita o è stata interrotta; task in esecuzione ora | |
| `scheduled_task_failed` | valore 1 per task fallito (al massimo 32) | `task`, `error` (prima riga del messaggio) |
| `library_scan_last_ok`, `library_scan_age_seconds` | ultima esecuzione di "Scan Media Library": riuscita o no, e da quanto tempo è terminata | |
| `plugins`, `plugins_broken` | plugin installati; quelli che non si sono caricati o non supportano questa versione | |
| `plugin_broken` | valore 1 per plugin rotto | `plugin`, `status` |
| `sessions`, `streams`, `streams_paused`, `transcodes` | sessioni client; quelle che riproducono qualcosa, in pausa, e in transcodifica | |
| `scrape_errors` | chiamate fallite nell'ultima sonda | |

Jellyfin non controlla i propri aggiornamenti (lo fa il pacchetto o
l'immagine), quindi non c'è alcuna metrica di aggiornamento.

Le [regole integrate](../alerting/rules.md#self-hosted-applications) che si
applicano: Jellyfin scheduled task failed, Jellyfin plugin broken, Jellyfin
restart pending, più Device unreachable.

## La pagina del dispositivo

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire la
pagina non chiama mai Jellyfin. Indica prima cosa non va (task falliti,
plugin rotti, un riavvio in attesa), poi stream, transcodifiche, sessioni e
l'età dell'ultima scansione della libreria, infine i task falliti e i plugin
rotti per nome con l'errore proprio di Jellyfin.

## Creare una API key in Jellyfin

1. In Jellyfin: Dashboard → API Keys (sotto Advanced) → +, e dai alla chiave il nome seguente. Copiala.

    ```
    DumbMonit
    ```

2. Jellyfin non può limitare cosa può fare una chiave: ogni chiave ha pieni diritti sul server. È comunque l'unico modo per leggere i task pianificati e la riproduzione di tutti; un utente normale non vede né l'uno né l'altro.

3. In DumbMonit, inserisci l'indirizzo di Jellyfin, per esempio "jellyfin.lan" (porta 8096) oppure "https://media.example.com" dietro un reverse proxy, e incolla la chiave.

4. DumbMonit legge solo /System/Info, /ScheduledTasks, /Sessions e /Plugins. Non avvia mai un task, non interrompe mai una riproduzione e non modifica mai un'impostazione.

!!! warning
    Poiché la chiave porta ogni diritto, tienila su una rete di cui ti fidi e metti Jellyfin dietro HTTPS su internet. Se dovesse mai essere compromessa, revocala nella stessa pagina: nient'altro la usa.

## Credenziali

| Credenziale | Campi |
|---|---|
| API key | Inviata nell'header `Authorization: MediaBrowser … Token="…"`, la forma che Jellyfin 10.11 e successivi accettano per impostazione predefinita. |

Indirizzo: un nome host (`jellyfin.lan`), `host:porta`, oppure un URL completo
con un prefisso di percorso (`https://example.com/jellyfin`); un `/web`
finale viene accettato. Opzioni: protocollo (HTTP per impostazione
predefinita), porta (8096), verifica del certificato e timeout della
richiesta (15 s).

Validato su Jellyfin 12.1.0 (immagine ufficiale).
