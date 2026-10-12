# TrueNAS

Server di storage ZFS (TrueNAS SCALE, ora TrueNAS Community Edition): lo
stato di salute di ogni pool e il disco che ha fallito, scrub e resilver,
utilizzo di pool e dataset rispetto alle loro quote, snapshot e
replicazione, temperatura dei dischi e autotest SMART, gli avvisi che
TrueNAS genera da solo, servizi e la macchina.

!!! warning "Testato solo su un'istanza di test"

    Questa integrazione è stata costruita e verificata su TrueNAS 25.04.2.6,
    installato dall'ISO ufficiale in una macchina virtuale per lo scopo —
    un pool RAIDZ1, dataset, task di snapshot e replicazione, poi lo stesso
    pool con un disco messo offline — non su un NAS che conserva le foto di
    una famiglia da anni. Una macchina virtuale non ha temperature dei
    dischi né risultati SMART, quindi quelli seguono il codice sorgente di
    TrueNAS invece di una risposta osservata. Ogni chiamata è una lettura.
    Facci sapere cosa non funziona.

## Il guasto per cui questo esiste

Un vdev mirror o RAIDZ che perde un disco continua a servire i dati. La
condivisione resta montata, i backup continuano a funzionare,
l'interfaccia web di ogni altro strumento resta verde — e il pool ora non
ha più ridondanza. Il secondo disco porta via il pool. `truenas_pool_healthy`
scende a 0 al primo disco, la regola **NAS pool degraded** si attiva, e la
pagina del dispositivo nomina il disco.

## La pagina del dispositivo

Oltre ai grafici generici, un dispositivo TrueNAS mostra tre pannelli,
letti da ciò che la sonda ha memorizzato — aprire la pagina non interroga
mai il NAS stesso:

* **Pools and disks** — ogni pool con il suo stato in una parola, la frase
  propria di ZFS quando qualcosa non va, quanto è pieno, il suo layout
  (mirror, RAIDZ, cache, log), i suoi conteggi di errori di lettura,
  scrittura e checksum, e soprattutto il nome di qualsiasi disco che non è
  `ONLINE`. Uno scrub o resilver in corso mostra il suo progresso. Poi ogni
  disco: modello, numero di serie, tipo, dimensione, pool, temperatura e il
  risultato del suo ultimo autotest SMART.
* **Datasets and protection** — ogni dataset con il suo utilizzo rispetto
  alla sua quota, il suo conteggio di snapshot e quando un task di
  snapshot l'ha coperto l'ultima volta; l'ultimo scrub di ogni pool e se è
  in ritardo; ogni task di replicazione e snapshot periodico con il suo
  stato, la sua ultima esecuzione, l'ultimo snapshot che ha gestito e,
  quando è fallito, la frase di errore propria di TrueNAS.
* **NAS health** — gli avvisi che TrueNAS ha generato da solo, i più
  severi prima, poi i servizi fermi, poi la macchina: versione, uptime,
  carico, memoria, CPU.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_truenas_`.

| Famiglia | Metriche | Etichette |
|---|---|---|
| Identità | `version_info` (la versione, senza il prefisso `TrueNAS-SCALE-` che le release più vecchie mettevano davanti) | `version` |
| Macchina | `uptime_seconds`, `load1/5/15`, `memory_total_bytes`, `cpu_count` | |
| Pool | `pool_healthy` (il verdetto proprio di ZFS: 0 su `DEGRADED` come su `FAULTED`), `pool_warning` (resilvering, funzionalità non abilitate), `pool_status_info`, `pool_size/allocated/free_bytes`, `pool_used_percent`, `pool_fragmentation_percent` | `pool`, e `status` per la serie info |
| Dispositivi del pool | `pool_device_errors` (cumulati dall'ultimo `zpool clear`), `pool_devices_unhealthy` (entrambi assenti quando il pool non può essere aperto) | `pool`, `kind` (`read`, `write`, `checksum`) |
| Scrub | `pool_scan_running`, `pool_scan_percent` (solo mentre è in corso), `pool_last_scrub_age_seconds`, `pool_last_scrub_errors`, `pool_scrub_threshold_days` | `pool`, `function` (`SCRUB`, `RESILVER`) |
| Dataset | `dataset_used_bytes`, `dataset_available_bytes`, `dataset_quota_bytes` e `dataset_quota_used_percent` (assente senza una quota), `dataset_snapshots`, `dataset_snapshot_age_seconds` (età dell'ultimo snapshot preso da un task periodico che copre il dataset; assente quando nessuno lo fa) `dataset_locked` (solo dataset cifrati) | `dataset`, `pool` |
| Snapshot | `snapshots` (totale) | |
| Dischi | `disk_temperature_celsius`, `disk_size_bytes`, `disk_smart_failed` (assente per un disco mai testato) | `disk`, `serial` |
| Avvisi | `alerts` — il numero di avvisi attivi per livello, zero incluso | `level` (`INFO` … `EMERGENCY`) |
| Task | `replication_error`, `replication_last_run_age_seconds`, `snapshot_task_error`, `snapshot_task_last_run_age_seconds` (solo task abilitati) | `task` |
| Servizi | `service_running` (servizi impostati per avviarsi con il NAS) | `service` |
| Raccolta | `up`, `scrape_errors`, `scrape_duration_seconds` | |

Alcune cose che TrueNAS fa e che vale la pena conoscere:

* **L'ultimo scrub vive solo nel record di scansione del pool.** Non c'è
  cronologia degli scrub nell'API, e un resilver sovrascrive il record di
  scansione: dopo un resilver, la data dell'ultimo scrub è semplicemente
  sconosciuta finché il successivo non termina, e `pool_last_scrub_age_seconds`
  è assente invece che errata.
* **Un pool che non può essere aperto** — esportato, o con troppi dischi
  mancanti — appare comunque, senza layout e senza dimensioni. Il suo
  `pool_healthy` è 0; le serie di dispositivi e dimensioni sono assenti
  invece che a zero.
* **Una quota non impostata non è una quota di zero.** TrueNAS risponde
  `parsed: null` per "nessuna quota" e DumbMonit non pubblica alcuna serie
  di quota per quel dataset. `quota` prevale su `refquota` quando entrambi
  sono impostati.
* **I conteggi di snapshot provengono dalla cache propria di ZFS**
  (`snapshots_count`), e il totale dal conteggio rapido di snapshot di
  TrueNAS — nessuno snapshot viene mai elencato. Il totale è maggiore della
  somma dei conteggi per dataset: include gli snapshot dei dataset interni
  di TrueNAS, che l'elenco dei dataset nasconde.
* **L'età dello snapshot più recente di un dataset** è l'ultima esecuzione
  del task di snapshot periodico che lo copre, direttamente o tramite un
  genitore recursivo. TrueNAS non può dare la data dello snapshot più
  recente senza elencarli tutti, quindi un dataset che nessun task copre
  non ha età — il che è di per sé da sapere.
* **Le temperature vengono lette dalla cache di TrueNAS** (`only_cached`),
  aggiornata da TrueNAS ogni cinque minuti circa. Un disco in sospensione
  non viene mai risvegliato, e un disco senza una lettura non ha serie.
* **Nulla viene mai avviato.** Nessuno scrub, nessun test SMART, nessun
  controllo aggiornamenti, nessuna esecuzione di replicazione. La sonda
  legge.
* **I percorsi cambiati tra le versioni sono opzionali.** `zfs/snapshot` è
  diventato `pool/snapshot` in 25.10 ed entrambi vengono provati;
  `smart/test/results` è scomparso in 25.10 e viene allora saltato in
  silenzio.

Le regole integrate che si applicano: Device unreachable, NAS pool
degraded, NAS disk errors, NAS pool almost full, NAS scrub found errors,
NAS scrub overdue, NAS disk failed its SMART test, NAS disk too hot, NAS
dataset near its quota, NAS replication failed, NAS snapshot task failed,
NAS snapshots stale, TrueNAS alert raised, NAS service stopped. Le
notifiche nominano il pool, il disco, il dataset, il task o il servizio
interessato.

## Cosa preparare in TrueNAS

I passaggi seguenti sono quelli mostrati dalla nota accanto al modulo. Una
cosa da sapere prima di tutto: sull'API REST che DumbMonit usa, **TrueNAS
non ha una chiave di sola lettura**. I suoi ruoli di sola lettura sono
stati collegati solo alla sua API WebSocket più recente; una chiave il cui
utente è un Read-Only Administrator viene rifiutata, con un `403` vuoto, su
ogni chiamata REST. Questo è stato verificato su TrueNAS 25.04.2.6: una
chiave Read-Only Administrator ha ottenuto `403` su tutti i 55 percorsi
REST provati, e la stessa identica chiave funzionava sull'API WebSocket. La
chiave deve appartenere a un utente con il privilegio Local Administrator —
quindi ottiene un utente e un gruppo propri, e nient'altro.

1. Nell'interfaccia web: Credentials → Groups → Add. Dai al gruppo il nome seguente e dagli il privilegio Local Administrator.

    ```
    dumbmonit
    ```

2. Perché Local Administrator e non Read-Only Administrator: TrueNAS ha collegato i suoi ruoli di sola lettura solo alla sua API WebSocket. Sull'API REST che DumbMonit usa, una chiave di sola lettura viene rifiutata su ogni singola chiamata. DumbMonit stesso legge solo: non avvia mai uno scrub, un test SMART, un aggiornamento o una replicazione.

3. Credentials → Users → Add. Dai all'account il nome seguente, rendi dumbmonit il suo gruppo primario, e lascia disattivati accesso shell, sudo e SSH: nessuno di essi è necessario.

    ```
    dumbmonit
    ```

4. Crea la chiave: Credentials → Users → API Keys → Add, scegli l'utente dumbmonit e dai alla chiave un nome. TrueNAS mostra la chiave solo una volta: copiala. Su TrueNAS 24.10 il percorso è il menu utente in alto a destra → API Keys → Add, e una chiave non appartiene a nessun utente.

5. In DumbMonit, inserisci l'indirizzo del NAS, per esempio "nas.lan", e incolla la chiave. Mantieni HTTPS: TrueNAS revoca una chiave che riceve mai su HTTP semplice da un'altra macchina.

!!! warning

    Questa chiave può modificare qualsiasi cosa sul NAS: TrueNAS non offre
    alcuna chiave di sola lettura sulla sua API REST. Trattala come la
    password di un account che può eliminare i tuoi pool, riservala solo a
    DumbMonit, e revocala dalla stessa pagina se viene compromessa. TrueNAS
    include anche un certificato autofirmato: se la connessione viene
    rifiutata per questo motivo, spunta "Accept an unverifiable
    certificate" nelle opzioni.

Indirizzo: `192.168.1.20`, `nas.lan`, `nas.lan:8443`, `[fd00::20]` oppure un
URL completo. Lo schema `https` e la porta 443 vengono aggiunti se
mancanti.

TrueNAS ha annunciato che l'API REST verrà rimossa in TrueNAS 26, a favore
dell'API WebSocket. Fino ad allora, REST è ciò che ogni release fino a
25.10 serve.

## Opzioni

| Chiave | Etichetta | Predefinito | Aiuto |
|---|---|---|---|
| `scheme` | Protocol | `https` | Mantieni HTTPS: TrueNAS revoca una chiave API che riceve su HTTP semplice da un'altra macchina. |
| `port` | Web interface port | `443` | Usata se l'indirizzo non fornisce una porta. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | TrueNAS include un certificato autofirmato per impostazione predefinita. |
| `request_timeout_seconds` | Timeout per request (seconds) | `20` | Tempo consentito per ogni chiamata API, da 1 a 120. |
| `datasets` | Watch the datasets | `true` | Utilizzo rispetto alla quota, conteggio snapshot, ultimo cambio di snapshot. |
| `disks` | Watch the disks | `true` | Inventario e temperature in cache; un disco in sospensione non viene mai risvegliato. |
| `smart` | Read the SMART test results | `true` | L'ultimo autotest di ogni disco; nessuno viene mai avviato. |
| `alerts` | Read TrueNAS's own alerts | `true` | L'elenco degli avvisi che TrueNAS mantiene da solo, gli avvisi respinti esclusi. |
| `tasks` | Watch replication and snapshot tasks | `true` | Stato, ultima esecuzione e ultimo snapshot di ogni task. |
| `services` | Watch the services | `true` | Servizi impostati per avviarsi con il NAS, e se funzionano. |

## Errori comuni

| Sintomo | Causa probabile |
|---|---|
| Errore di autenticazione che menziona Local Administrator | L'utente della chiave non è un amministratore completo — tipicamente un Read-Only Administrator. TrueNAS rifiuta tale chiave su ogni chiamata REST, con un `403` vuoto. |
| Errore di autenticazione "Invalid API key" | La chiave è stata digitata male, eliminata, scaduta — oppure revocata da TrueNAS perché è stata inviata una volta su HTTP semplice da un'altra macchina. Creane una nuova e mantieni HTTPS. |
| Errore di certificato | Certificato autofirmato: installane uno fidato (ACME è integrato) oppure abilita `insecure_tls`. |
| Nessuna temperatura dei dischi | Il NAS funziona in una macchina virtuale, oppure TrueNAS non ha ancora interrogato i dischi da quando è stato avviato. Le letture provengono dalla sua cache. |
| Nessuna serie SMART | Nessun autotest è mai stato eseguito su quel disco, oppure il NAS esegue TrueNAS 25.10, che ha rimosso quella chiamata. Pianifica i test SMART in Data Protection. |
| Nessun `pool_last_scrub_age_seconds` | L'ultima scansione di quel pool era un resilver, che cancella il record dell'ultimo scrub, oppure il pool non è mai stato sottoposto a scrub. |
| Un dataset non ha serie di quota | Non ha una quota. Una quota non impostata non è una quota di zero. |
| "NAS pool degraded" e la pagina non mostra alcun nome di disco | Il pool non può essere aperto affatto (esportato, o troppi dischi mancanti): TrueNAS allora non riporta alcun layout. |
| `scrape_errors` sopra zero | Una chiamata opzionale è fallita per un motivo diverso da un percorso mancante — un timeout, di solito. Le altre metriche non sono interessate. |
