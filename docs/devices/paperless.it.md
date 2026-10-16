# Paperless-ngx

Se Paperless-ngx continua a importare ciò che gli viene inviato: lo stato di
salute del suo database, Redis, i worker Celery, l'indice di ricerca, il
classificatore e il controllo di integrità, i task falliti, gli
aggiornamenti, lo spazio su disco e il numero di documenti.

DumbMonit legge l'API REST di Paperless-ngx con un **utente dedicato che non
può leggere alcun documento**: tre permessi di visualizzazione gli danno
l'elenco dei task, lo stato del sistema e i conteggi globali, nient'altro.

Il guasto da tenere d'occhio: **Redis o Celery fermi**. L'interfaccia web si
apre e i documenti possono essere letti, ma nulla di ciò che viene depositato
nella cartella di consumo o recuperato dalla posta viene importato finché
qualcuno non nota l'accumulo. Lo stato del sistema lo segnala; DumbMonit
genera un avviso su questo.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_paperless_`.

| Metrica | Cosa | Etichette |
|---|---|---|
| `version_info` | valore 1 | `version` |
| `update_available` | 1 quando il controllo versione proprio di Paperless-ngx conosce una release più recente (assente quando il controllo è disattivato) | `latest_version` |
| `component_status` | 0 OK, 1 warning, 2 error, come riportato dalla pagina di stato del sistema | `component`: `database`, `redis`, `celery`, `index`, `classifier`, `sanity_check`, `llm_index` (solo se abilitato) |
| `unapplied_migrations` | migrazioni del database non ancora applicate | |
| `storage_available_bytes`, `storage_total_bytes`, `storage_used_percent` | il disco che contiene i media | |
| `tasks_failed_window`, `tasks_window`, `tasks_pending` | tutti i task degli ultimi 30 giorni: falliti, totali, e ancora in attesa | |
| `tasks_failed_recent` | task falliti visibili a questo utente, più recenti della finestra di task falliti e non respinti | |
| `task_failed` | valore 1 per tale task (al massimo 32) | `task`, `file` |
| `documents`, `documents_inbox` | documenti in Paperless, e nella inbox | |
| `status_readable` | 0 quando l'account non ha il permesso di monitoraggio del sistema | |
| `scrape_errors` | chiamate fallite nell'ultima sonda | |

Paperless mantiene privati i task di ciascun proprietario: l'utente dedicato
vede per nome i task pianificati (controllo di integrità, indice,
classificatore, posta), mentre le importazioni di altri utenti sono contate
in `tasks_failed_window`, che copre ogni task.

Le [regole integrate](../alerting/rules.md#self-hosted-applications) che si
applicano: Paperless task queue down, Paperless index or sanity check error,
Paperless migrations pending, Paperless task failed, Paperless storage
almost full, Paperless update available, più Monitor unreachable.

## La pagina del monitor

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire la
pagina non chiama mai Paperless. Indica prima cosa non va (un componente in
errore, task falliti, migrazioni in attesa), poi gli aggiornamenti, poi
documenti, inbox, task in attesa e falliti e spazio su disco, infine ogni
componente e i task falliti per nome.

## Creare un utente di sola lettura in Paperless-ngx

1. Crea un utente con il nome seguente, con una password lunga e casuale, senza stato di superuser o staff, e solo tre permessi di visualizzazione: task, monitoraggio del sistema e statistiche globali. Con l'API e il tuo account, i due comandi seguenti lo creano e stampano il suo token.

    ```
    dumbmonit
    curl -u YOUR_USER -H 'Content-Type: application/json' -X POST https://paperless.example.com/api/users/ -d '{"username":"dumbmonit","password":"CHANGE-ME","user_permissions":["view_paperlesstask","view_system_monitoring","view_global_statistics"]}'
    curl -X POST https://paperless.example.com/api/token/ -d 'username=dumbmonit&password=CHANGE-ME'
    ```

2. In alternativa nell'interfaccia web: Users & Groups → Add user, poi nei suoi permessi spunta View solo sulle righe PaperlessTask, SystemMonitoring e GlobalStatistics. Accedi come quell'utente una volta, apri My Profile e genera il suo token API.

3. Questo utente non legge nessun documento: le statistiche globali danno i conteggi, il monitoraggio del sistema dà lo stato di salute del database, Redis, Celery e dell'indice, e i task mostrano quelli falliti. Su Paperless-ngx 2.x, dove gli ultimi due permessi non esistono, stato di salute e conteggi vengono saltati e il resto viene letto.

4. In DumbMonit, inserisci l'indirizzo, per esempio "paperless.lan" (porta 8000) oppure "https://paperless.example.com" dietro un reverse proxy, e incolla il token.

!!! warning
    Quando Redis o Celery si fermano, Paperless continua a servire le sue pagine e i documenti, ma nulla di nuovo viene importato, dalla cartella di consumo o dalla posta, finché qualcuno non guarda. DumbMonit genera un avviso su entrambi.

Sostituisci `CHANGE-ME` con una password lunga e casuale prima di eseguire i
comandi, e `YOUR_USER` con un account autorizzato a gestire gli utenti (curl
chiede la sua password). Richiedere lo stato del sistema può richiedere
diversi secondi mentre Redis è inattivo: Paperless lo attende prima di
rispondere.

## Credenziali

| Credenziale | Campi |
|---|---|
| API token (consigliato) | Il token dell'utente dedicato, inviato come `Authorization: Token …`. |
| User name / password | L'utente dedicato stesso, inviato come autenticazione HTTP di base. |

Indirizzo: un nome host (`paperless.lan`), `host:porta`, oppure un URL
completo con un prefisso di percorso; un `/api` finale viene accettato.
Opzioni: protocollo (HTTP per impostazione predefinita), porta (8000),
verifica del certificato, timeout della richiesta (15 s) e la finestra dei
task falliti (24 ore, da 1 a 720).

Validato su Paperless-ngx 3.2.1 (immagine ufficiale, SQLite, Valkey).
