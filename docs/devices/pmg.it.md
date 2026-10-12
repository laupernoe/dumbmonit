# Proxmox Mail Gateway

Gateway di posta: code Postfix e l'età del messaggio più vecchio in
attesa, la posta contata e filtrata oggi, dimensioni della quarantena,
l'età dei database delle firme antivirus e antispam, servizi, certificati
e stato del cluster.

!!! warning "Testato solo su un'istanza di test"

    Questa integrazione è stata costruita e verificata su un Proxmox Mail
    Gateway 9 installato per lo scopo, non su uno di produzione. Ogni
    chiamata è una lettura, e le forme provengono da un vero gateway di
    posta, ma nessuno l'ha ancora eseguito per settimane su un gateway di
    posta in funzione. Facci sapere cosa non funziona.

## La pagina del dispositivo

Oltre ai grafici generici, un dispositivo Proxmox Mail Gateway mostra tre
pannelli, letti da ciò che la sonda ha memorizzato — aprire la pagina non
interroga mai il gateway stesso:

* **Mail queues** — le quattro code Postfix nell'ordine in cui le legge
  un amministratore di posta: *incoming* (appena accettata), *active* (in
  consegna), *deferred* (consegna fallita, verrà ritentata) e *hold*
  (trattenuta da una regola). Ognuna mostra quanti messaggi contiene,
  quanti domini di destinazione, il più trafficato di quei domini, e per
  quanto tempo il messaggio più vecchio è in attesa. Una coda che
  semplicemente ha posta dentro è traffico normale; una coda che trattiene
  un messaggio per più di quattro ore viene segnalata *Stuck*.
* **Mail filtered today** — messaggi in entrata e uscita, byte, spam,
  virus, bounce, tempo medio di elaborazione, e cosa è stato rifiutato
  prima della consegna (greylisting, blocklist, SPF, pregreeting). Una
  barra per fetta della finestra recente disegna la curva del traffico
  senza interrogare il database delle serie temporali. Poi le tre
  quarantene con il loro conteggio di messaggi e uso del disco, e i virus
  catturati oggi per nome.
* **Gateway** — i servizi fermi prima, poi i membri del cluster e il loro
  stato di sincronizzazione, poi ogni nodo: uptime, CPU, memoria, disco,
  offset dell'orologio, i database delle firme con la loro età e
  verdetto, certificati in scadenza entro due settimane, aggiornamenti in
  attesa e lo stato dell'abbonamento.

Solo conteggi, mai contenuto. DumbMonit legge quanti messaggi si trovano
in ogni quarantena, mai un oggetto, un mittente, un destinatario o un
corpo. Gli endpoint di statistiche per indirizzo (`/statistics/sender`,
`/statistics/receiver`, `/statistics/contact`) non vengono chiamati
affatto.

## Oggi, non le ultime ventiquattro ore

`GET /statistics/mail` aggrega per **giorno locale sul gateway**: PMG
ignora la fine della finestra che chiedi, solo `starttime` scelga il
giorno. DumbMonit quindi chiede "ora", il che dà i totali da mezzanotte —
esattamente ciò che mostra la dashboard propria di PMG. Quelle serie sono
gauge che tornano a zero a mezzanotte, non contatori: non avvolgerle in
`rate()`. La curva recente (`/statistics/recent`) legge invece la tabella
grezza e scorre davvero.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_pmg_`.

| Famiglia | Metriche | Etichette |
|---|---|---|
| Nodo | `node_cpu_percent`, `node_iowait_percent`, `node_cpu_count`, `node_load1/5/15`, `node_memory_used/total_bytes`, `node_memory_used_percent`, `node_swap_used/total_bytes`, `node_rootfs_used/total/avail_bytes`, `node_rootfs_percent`, `node_uptime_seconds`, `node_clock_offset_seconds` (positivo quando il gateway è avanti rispetto a DumbMonit), `node_insync`, `node_kernel_info`, `node_version_info` (la versione API di *questo* nodo — in un cluster, quello rimasto indietro dall'ultimo aggiornamento appare qui), `version_info` | `node` |
| Code | `queue_messages`, `queue_domains`, `queue_oldest_age_seconds` | `queue` (`incoming`, `active`, `deferred`, `hold`) |
| Posta oggi | `mail_count_in/out`, `mail_bytes_in/out`, `mail_spam_in/out`, `mail_virus_in/out`, `mail_bounces_in/out`, `mail_junk_in`, `mail_junk_out` (i tuoi stessi utenti che inviano junk: il primo segno di un account compromesso), `mail_junk_percent` (assente in un giorno senza posta in entrata), `mail_greylisted`, `mail_spf_rejects`, `mail_rbl_rejects`, `mail_pregreet_rejects`, `mail_avg_processing_seconds` | |
| Throughput | `mail_rate_in_per_minute`, `mail_rate_out_per_minute` (dall'ultima fetta completa della curva recente) | |
| Punteggi spam | `spam_score_messages` | `level` (`0` a `10`; `10` aggrega tutto sopra) |
| Virus | `virus_detections` (top dieci del giorno) | `virus` |
| Quarantene | `quarantine_messages`, `quarantine_bytes`, `quarantine_avg_spam_level` | `kind` (`spam`, `virus`, `attachment`) |
| Servizi | `service_running` (1 o 0; nessuna serie affatto per un'unità non installata) | `node`, `service` |
| Firme | `signature_age_seconds`, `signature_count`, `signature_update_available` | `node`, `family` (`virus`, `spam`), `database` (`main`, `daily`, `bytecode`, oppure il canale SpamAssassin) |
| Certificati | `certificate_expires_in_seconds` | `node`, `certificate` |
| Cluster | `cluster_nodes`, `cluster_node_insync`, `cluster_node_healthy` (tutti assenti su un gateway standalone) | `node`, `role` |
| Abbonamento | `subscription_active` (le etichette portano lo stato esatto) | `node`, `status`, `level` |
| Aggiornamenti | `node_updates_pending`, `node_updates_security_pending` | `node` |
| Raccolta | `up`, `scrape_errors`, `scrape_duration_seconds` | |

`queue_oldest_age_seconds` è un **limite inferiore**. `qshape` riporta
fasce di età (sotto 5 min, 5-10 min, 10-20 min, e così via, raddoppiando),
quindi DumbMonit mantiene il minimo della fascia più alta occupata: un
messaggio riportato ad "almeno 20 minuti" potrebbe avere 39 minuti. Errare
verso il basso significa che l'avviso si attiva più tardi invece che
prima, mai su un messaggio che non è davvero in ritardo.

`queue_domains` è esatto su un singolo gateway. Su un cluster le code per
nodo vengono unite e il conteggio viene preso sui domini più trafficati
di ciascun nodo, limitato a venti per nodo: oltre quello sottoconta. La
deduplicazione vince sulla somma qui, poiché due nodi di un cluster
ricevono posta per gli stessi domini.

Un gateway standalone risponde a `/config/cluster/status` con un elenco
vuoto. Questo è un gateway senza cluster, non un cluster degradato:
nessuna serie viene pubblicata e nessuna regola può attivarsi.

Solo il database **daily** di ClamAV viene giudicato sulla sua età, sia
dal pannello che dalla regola. `main` viene ricostruito una o due volte
all'anno e `bytecode` appena più spesso: giudicarli su due giorni
stamperebbe "out of date" tutto l'anno su un gateway perfettamente
aggiornato. Mostrano comunque la loro età, versione e conteggio di firme
— senza un verdetto. Allo stesso modo, un canale SpamAssassin che non è
mai stato scaricato non ha alcuna data affatto, e "mai aggiornato" non è
"non aggiornato": nessuna serie, nessun verdetto, nessun avviso.

ClamAV scrive la sua data di build nel proprio formato dentro l'header
`.cvd` (`16 Dec 2025 23-18 +0000`, con un trattino dove tutti gli altri
mettono un due punti) e PMG la passa così com'è. Se quel formato dovesse
mai cambiare, l'età è semplicemente assente invece che errata di
cinquant'anni. La stessa regola vale ovunque: un valore che l'API non
restituisce produce una metrica in meno, mai uno zero che finge di essere
una misurazione.

Le regole integrate che si applicano: Device unreachable, Mail queue
growing, Mail stuck in the queue, Mail gateway service stopped, Virus
signatures out of date, Spam rules out of date, Quarantine filling up,
Mail gateway cluster degraded, Mail gateway certificate expiring, Mail
gateway updates pending. Le notifiche nominano la coda, il servizio, il
database o il nodo interessato.

## Cosa preparare in Proxmox Mail Gateway

I passaggi seguenti sono quelli mostrati dalla nota accanto al modulo. Il
principio: un utente riservato al monitoraggio, con un ruolo di sola
lettura — mai l'account con cui accedi.

1. Nell'interfaccia web: Configuration → User Management → Users → Add. Dai all'account il nome seguente, dagli una password lunga usata in nessun altro posto, e spunta Enabled.

    ```
    dumbmonit@pmg
    ```

2. Imposta il suo Role su `Audit`. Questo è esattamente il minimo di sola lettura per tutto ciò che DumbMonit legge: stato del nodo, servizi, code postfix, statistiche di posta, conteggi di quarantena, età dei database ClamAV e SpamAssassin, stato del cluster, certificati, abbonamento e aggiornamenti in attesa. `Audit` non può modificare nulla, non può rilasciare nulla dalla quarantena e non legge alcun messaggio.

3. Preferisci una shell? Lo stesso account in un comando, poi imposta la password.

    ```
    pmgsh create /access/users --userid dumbmonit@pmg --role audit --enable 1 --comment "DumbMonit monitoring"
    ```

4. In DumbMonit, inserisci l'indirizzo del gateway, per esempio "mail.lan" oppure "mail.lan:8006", poi il nome utente di questo account (realm incluso) e la password.

5. Proxmox Mail Gateway non emette token API, a differenza di Proxmox VE e Proxmox Backup Server: il nome utente e la password sono la via d'accesso. DumbMonit apre una sessione di due ore e la rinnova, invece di accedere a ogni misurazione.

!!! warning

    Non riutilizzare l'account con cui accedi: il ruolo Audit sopra può
    solo leggere, e non può rilasciare un messaggio in quarantena o
    modificare una regola. Proxmox Mail Gateway usa anche un certificato
    autofirmato per impostazione predefinita: se la connessione viene
    rifiutata per questo motivo, spunta "Accept an unverifiable
    certificate" nelle opzioni.

Indirizzo: `10.0.0.40`, `mail.lan`, `mail.lan:8006`, `[fd00::4]` oppure un
URL completo. Lo schema `https` e la porta 8006 vengono aggiunti se
mancanti.

## Opzioni

| Chiave | Etichetta | Predefinito | Aiuto |
|---|---|---|---|
| `port` | API port | `8006` | Usata se l'indirizzo non fornisce una porta. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | Proxmox Mail Gateway fornisce un certificato autofirmato per impostazione predefinita: abilita questo se la connessione viene rifiutata per questo motivo. |
| `request_timeout_seconds` | Timeout per request (seconds) | `15` | Tempo consentito per ogni chiamata API, da 1 a 120. Leggere una coda esegue un comando Postfix sul gateway e può richiedere qualche secondo. |
| `node` | Monitored node | *(vuoto)* | Nome del nodo del cluster da monitorare. Vuoto: ogni nodo che il gateway elenca. |
| `recent_hours` | Traffic window (hours) | `12` | Quanto indietro va la curva del traffico, da 1 a 24. I totali giornalieri coprono sempre il giorno corrente. |
| `queues` | Watch the mail queues | `true` | Legge le code incoming, active, deferred e hold e l'età del messaggio più vecchio in ciascuna. |
| `quarantine` | Count the quarantines | `true` | Legge quanti messaggi contengono le quarantene spam e virus, e quanto disco usano. Solo conteggi. |
| `attachment_quarantine` | Also count the attachment quarantine | `false` | Quella quarantena non ha una chiamata di conteggio: deve essere elencata per essere contata. Disattivato per impostazione predefinita; viene mantenuto solo il numero di voci. |
| `signatures` | Watch the signature databases | `true` | Legge l'età dei database virus ClamAV e dei canali di regole SpamAssassin. |
| `services` | Watch the services | `true` | Legge lo stato di postfix, pmg-smtp-filter, pmgpolicy e le altre unità di ogni nodo. |
| `certificates` | Watch the certificates | `true` | Legge i certificati che l'interfaccia web serve e avvisa prima che scadano. |
| `updates` | Count pending updates | `true` | Elenca i pacchetti in attesa di un aggiornamento su ogni nodo. |
| `subscription` | Read the subscription | `true` | Legge lo stato dell'abbonamento di ogni nodo. |

## Errori comuni

| Sintomo | Causa probabile |
|---|---|
| ClamAV `main` mostra un'età enorme | Previsto: viene ricostruito una o due volte all'anno. Solo `daily` porta un verdetto. |
| Errore di certificato | Certificato autofirmato: installane uno fidato (ACME è integrato in PMG) oppure abilita `insecure_tls`. |
| Errore di autenticazione | Realm errato nel nome utente — deve leggere `dumbmonit@pmg`, non `dumbmonit` — oppure l'account non è Enabled. Solo il fallimento di `GET /version` condanna l'intera sonda; qualsiasi altra chiamata fallita viene contata in `scrape_errors`. |
| Permessi insufficienti | Il ruolo dell'account è sotto `Audit`. `quser` vede solo la propria posta; `helpdesk` non può leggere il nodo o le statistiche. |
| Ogni metrica `mail_*` è zero | Il gateway non ha genuinamente filtrato nulla da mezzanotte. I totali coprono il giorno locale corrente sul gateway e si azzerano alla sua mezzanotte, non alla tua. |
| Nessuna serie `service_running` affatto | L'opzione `services` è disattivata, oppure il ruolo non può elencarli. Una singola unità mancante dall'elenco semplicemente non è installata su quel nodo (`chrony` su una macchina che usa `systemd-timesyncd`, per esempio) e deliberatamente non viene riportata come ferma. |
| Nessun `signature_age_seconds` per un canale SpamAssassin | Quel canale non è mai stato aggiornato, quindi PMG non restituisce alcuna data. Esegui `sa-update` sul gateway; un canale secondario configurato ma mai scaricato resta senza data di proposito. |
| Nessun database ClamAV affatto | ClamAV non è installato oppure `freshclam` non ha mai completato un download. L'endpoint risponde con un elenco vuoto, che non è un errore. |
| Nessuna metrica di cluster | Il gateway è standalone. `/config/cluster/status` risponde con un elenco vuoto, e nessuna regola di cluster può attivarsi. |
| "Mail stuck in the queue" si attiva in ritardo | `queue_oldest_age_seconds` è il minimo della fascia di età più alta occupata di `qshape`, quindi sottoconta per progetto. |
| `quarantine_messages{kind="attachment"}` manca | L'opzione `attachment_quarantine` è disattivata: quella quarantena non ha un endpoint di conteggio, quindi contarla significa elencarla. |
