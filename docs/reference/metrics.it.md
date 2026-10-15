# Metriche

DumbMonit memorizza ogni misura in VictoriaMetrics e la rilegge con MetricsQL,
tramite il [proxy delle metriche](api.md#metrics) del server. Questa pagina
descrive la denominazione, le etichette e la retention. Gli elenchi di metriche
per tipo si trovano nelle [pagine dei monitor](../devices/index.md).

## Denominazione

Ogni metrica ha il prefisso `dumbmonit_`, così DumbMonit può condividere
un'istanza di VictoriaMetrics con altri strumenti senza collisioni. I nomi
seguono `dumbmonit_<what>_<unit>`:

| Schema | Esempi |
|---|---|
| `…_bytes`, `…_bytes_total`, `…_bytes_used` | `dumbmonit_memory_bytes_total`, `dumbmonit_filesystem_used_bytes` |
| `…_percent` | `dumbmonit_cpu_usage_percent`, `dumbmonit_pbs_datastore_used_percent` |
| `…_seconds`, `…_age_seconds` | `dumbmonit_probe_duration_seconds`, `dumbmonit_proxmox_backup_last_age_seconds` |
| `…_celsius`, `…_volts`, `…_hertz`, `…_watts` | Letture di UPS e Synology |
| `…_info` | Serie di presenza (valore 1) le cui informazioni sono nelle etichette: `dumbmonit_system_info`, `dumbmonit_pbs_version_info` |
| `…_status`, `…_up`, `…_running` | Enumerazioni o booleani: `dumbmonit_if_oper_status`, `dumbmonit_proxmox_guest_running` |

Le famiglie specifiche di una sorgente portano la sorgente come secondo prefisso:
`dumbmonit_proxmox_*`, `dumbmonit_pbs_*`, `dumbmonit_synology_*`,
`dumbmonit_probe_*` (servizi). I profili SNMP e l'agent usano nomi semplici
(`dumbmonit_if_octets_in`, `dumbmonit_cpu_usage_percent`).

I contatori (ottetti e pacchetti delle interfacce, byte dei dischi, pagine
stampate) sono memorizzati **grezzi**, così come li riporta il monitor:
applica `rate()` nelle tue query per ottenere un throughput. La pagina del
monitor mostra ogni serie `dumbmonit_*` del monitor così com'è
memorizzata, quindi un contatore appare come una linea crescente. Un riavvio si
manifesta come un azzeramento del contatore, mai come un falso picco.

## `dumbmonit_up`

Dopo ogni probe riuscito, il server scrive `dumbmonit_up = 1` per il
monitor. Non viene scritto nulla quando un probe fallisce: la serie
semplicemente si interrompe, e la regola "Monitor non raggiungibile" misura
l'età dell'ultimo campione (`time() - tlast_over_time(dumbmonit_up[7d])`),
mantenendo solo la serie più recente di ciascun monitor
(`bottomk by (target) (1, …)`): una serie non più scritta dopo una modifica di
tag o etichetta resta nella finestra di 7 giorni e non deve contare.

Per i servizi (`http`, `tcp`, `dns`, `ping`, `tls`), `dumbmonit_up = 1` significa
"il monitor è stato eseguito", e `dumbmonit_probe_success` indica se il servizio
è in buono stato. I due segnali sono tenuti separati di proposito: un
`probe_success` pari a 0 dice che il servizio è fermo, un'interruzione di `up`
dice che è il monitoraggio a essere fermo.

## `dumbmonit_security_score`

Ogni 15 minuti il server calcola il [punteggio di sicurezza](../using/security-score.md)
di ogni monitor il cui tipo prevede controlli di sicurezza, e scrive
`dumbmonit_security_score` (0–100, assente finché nulla può essere valutato) e
`dumbmonit_security_checks_failed`, entrambi con l'etichetta `target`.

## Etichette

| Etichetta | Impostata da | Significato |
|---|---|---|
| `target` | server | L'id del monitor, come in `/api/targets/{id}`. Ogni regola e ogni grafico selezionano su di essa. |
| `host` | server | Il nome del monitor. |
| `tag_<key>` | server | I tag liberi del monitor. Le opzioni del tipo (`insecure_tls`, `port`…), i tag la cui chiave sembra un segreto (verificata per parola: `key`, `pass`, `cert`, `private`, `pem`…) e i valori oltre 256 caratteri non vengono copiati. Non mettere mai un segreto in un tag. |
| `probe` | servizi | `http`, `tcp`, `dns`, `ping` o `tls`. |
| etichette per serie | collector | `ifname`, `mountpoint`, `device`, `fstype`, `core`, `service`, `container`, `node`, `vmid`, `name`, `storage`, `datastore`, `group`, `volume`, `disk`, `pool`, `repo`, `changes`, `reason`, `version`, `issuer`… come elencate in ogni pagina dei monitor. |

Le etichette di identità prevalgono sempre su quelle dei collector, e per gli
agent vengono impostate alla ricezione: una macchina non può scrivere nelle
serie di un'altra macchina.

!!! note "Prefisso prima di settembre 2026"
    Le serie scritte da EzyMonit si chiamano `ezymonit_…`. Il prefisso è cambiato
    con il nome del prodotto e nulla legge le vecchie serie: restano in
    VictoriaMetrics finché la retention non le rimuove, i grafici e le regole
    ripartono dall'aggiornamento, e le regole personalizzate che citano ancora
    `ezymonit_…` vanno modificate.

## Retention e archiviazione

La VictoriaMetrics incorporata conserva 12 mesi di dati
(`DUMBMONIT_VM_RETENTION`) in `/data/vm`, nel volume `dumbmonit-data`. Il server
raggruppa le sue scritture e le scarica ogni 5 secondi
(`DUMBMONIT_FLUSH_INTERVAL_SECS`). Eliminare un monitor non ne elimina le
serie; invecchiano fino a scadere.

Lo storico degli avvisi è conservato 90 giorni e le baseline delle serie
scomparse da 60 giorni, entrambi in SQLite (vedi il
[riferimento della configurazione](configuration.md)).

## Interrogazione

- Tramite il server: `GET /api/metrics/query` e `GET /api/metrics/query_range`,
  con il cookie di sessione. I risultati degli intervalli sono limitati a 2 000
  punti per serie.
- Da un Prometheus o Grafana esistente: `GET /federate`, `GET /metrics` e
  la sorgente dati `/prometheus`, con un token API —
  [più sotto](#scraping-dumbmonit).
- Direttamente, in sviluppo: l'overlay `docker-compose.dev.yml` pubblica la
  VictoriaMetrics incorporata su `http://localhost:8428`, con una propria
  interfaccia su `http://localhost:8428/vmui/`. In produzione ascolta solo sul
  loopback del container; imposta `DUMBMONIT_VM_LISTEN=0.0.0.0:8428` e pubblica
  tu stesso la porta se vuoi Grafana o un altro strumento sugli stessi dati.

```
# Interface throughput in bits per second
rate(dumbmonit_if_octets_in{target="4"}[5m]) * 8

# Availability of every service over 30 days
avg by (host) (avg_over_time(dumbmonit_probe_success[30d]))

# Every device that has not reported for 5 minutes
bottomk by (target) (1, time() - tlast_over_time(dumbmonit_up[7d])) > 300
```

## Raccogliere le metriche di DumbMonit {#scraping-dumbmonit}

Se usi già un Prometheus o un Grafana, non ti serve un secondo stack. Tre rotte
si trovano fuori da `/api`, dove uno scraper le cerca:

| Rotta | Cosa restituisce |
|---|---|
| `GET /metrics` | Lo stato di salute dell'istanza DumbMonit stessa: scheduler, probe, scritture, avvisi, notifiche, agent, database. Qualche decina di serie, sempre lo stesso numero — mai una per monitor. |
| `GET /federate` | Le misure, selezionate da `match[]`, esattamente come Prometheus federa un altro Prometheus: l'ultimo punto di ciascuna serie selezionata. |
| `GET /prometheus/api/v1/…` | La metà in lettura dell'API Prometheus, inoltrata all'archivio: ciò che chiama una sorgente dati di Grafana. |

Tutte e tre accettano `Authorization: Bearer dmt_…` con l'ambito **`read`** — un
token creato in Impostazioni → *API e assistenti*, dello stesso tipo usato dal
[server MCP](../using/assistant.md). Non è coinvolto alcun cookie, quindi non
serve l'header `X-Requested-With`; è accettato anche un cookie di sessione, così
puoi aprirle da un browser con accesso effettuato. Un token è limitato a 120
chiamate al minuto: abbondante per uno scrape ogni 15 secondi, e sufficiente per
una dashboard Grafana di qualche decina di pannelli — dai a una dashboard molto
usata un token tutto suo invece di condividerlo con uno scraper.

### Prometheus

```yaml
# prometheus.yml — DumbMonit's own health, then the measurements it collects.
scrape_configs:
  - job_name: dumbmonit
    metrics_path: /metrics
    scheme: http
    authorization:
      type: Bearer
      credentials: dmt_0123456789abcdef0123456789abcdef
    static_configs:
      - targets: ['dumbmonit.lan:8080']

  - job_name: dumbmonit-federate
    metrics_path: /federate
    scheme: http
    honor_labels: true
    scrape_interval: 60s
    authorization:
      type: Bearer
      credentials: dmt_0123456789abcdef0123456789abcdef
    params:
      'match[]':
        - '{__name__=~"dumbmonit_.*"}'
    static_configs:
      - targets: ['dumbmonit.lan:8080']
```

`honor_labels: true` mantiene le etichette `target` e `host` impostate da
DumbMonit invece di sovrascriverle con quelle del job di scrape. Metti il token
in un file e usa `authorization: {type: Bearer, credentials_file: /etc/prometheus/dumbmonit.token}`
se preferisci non averlo nella configurazione.

Senza `match[]`, `/federate` seleziona `{__name__=~"dumbmonit_.*"}` — ogni serie
che DumbMonit scrive, e nulla che appartenga a un altro strumento che condivide
la stessa VictoriaMetrics. Restringilo per mantenere piccola una federazione:
`{__name__=~"dumbmonit_.*",target="4"}` per un monitor,
`{__name__="dumbmonit_up"}` solo per la disponibilità. Al massimo 10 selettori
per richiesta, e una risposta superiore a 8 MiB viene rifiutata con un messaggio
che invita a restringerla — suddividi una flotta numerosa in più job con
selettori più stretti invece di un solo job che chiede tutto.
`max_lookback=15m` amplia la finestra in cui viene cercato l'ultimo punto, per i
monitor interrogati meno spesso di ogni cinque minuti.

```bash
curl -H 'Authorization: Bearer dmt_…' http://localhost:8080/metrics
curl -H 'Authorization: Bearer dmt_…' -G http://localhost:8080/federate \
  --data-urlencode 'match[]={__name__=~"dumbmonit_.*",target="4"}'
```

### Grafana

Punta Grafana a **DumbMonit**, non alla VictoriaMetrics che esso esegue: il
processo figlio ascolta sul loopback del container, non ha alcuna
autenticazione propria, ed espone le rotte che *eliminano* le serie con la stessa
facilità di quelle che le leggono. DumbMonit inoltra la metà in lettura dell'API
Prometheus sotto `/prometheus`, dietro lo stesso token, così Grafana parla con
una normale sorgente dati Prometheus:

1. **Connections → Data sources → Add → Prometheus**.
2. **URL**: `http://dumbmonit.lan:8080/prometheus` (Grafana aggiunge da sé
   `/api/v1/query`).
3. **HTTP headers**: aggiungine uno, `Authorization` = `Bearer dmt_…`. Grafana lo
   memorizza cifrato e non lo mostra più.
4. **Save & test** — Grafana segnala che la sorgente dati funziona, e ogni serie
   `dumbmonit_*` è quindi disponibile in PromQL, con il completamento
   automatico dei nomi di metriche ed etichette.

Vengono inoltrate solo le rotte in lettura — `query`, `query_range`, `series`,
`labels`, valori delle etichette, `metadata` e la versione che Grafana richiede
quando prova la connessione. Qualsiasi altra cosa, `admin/tsdb/delete_series` per
prima, risponde `404`: questo punto di ingresso non può modificare né cancellare
nulla.

Se usi già un Prometheus, l'altra strada funziona altrettanto bene: Prometheus
federa DumbMonit con il job qui sopra, Grafana legge Prometheus, e le query sono
identiche.

### Metriche dell'istanza

Tutto ciò che espone `GET /metrics`. I valori vengono letti quando fai lo scrape;
i contatori `_total` sono monotoni e si azzerano solo al riavvio del server
(`dumbmonit_uptime_seconds` ti dice quando è successo).

| Metrica | Tipo | Significato |
|---|---|---|
| `dumbmonit_build_info{version}` | gauge | Sempre 1. L'etichetta `version` è la versione del server in esecuzione, la stessa riportata da `/api/health`. |
| `dumbmonit_uptime_seconds` | gauge | Secondi dall'avvio del processo del server. |
| `dumbmonit_scheduler_cycles_total` | counter | Cicli dello scheduler eseguiti. Il ciclo scatta una volta al secondo, quindi funge anche da segnale di vitalità. |
| `dumbmonit_scheduler_cycle_seconds` | gauge | Durata dell'ultimo ciclo. Normalmente sotto il millisecondo: il ciclo si limita a distribuire, non attende i probe. |
| `dumbmonit_scheduler_backlog` | gauge | Monitor ancora da interrogare al termine dell'ultimo ciclo. Zero su un'istanza sana; un valore diverso da zero e duraturo significa che `DUMBMONIT_MAX_CONCURRENT_PROBES` è troppo basso per il numero di monitor e i loro intervalli. |
| `dumbmonit_probes_total{kind}` | counter | Probe eseguiti, per tipo di monitor (`snmp`, `proxmox`, `http`…). Include quelli eseguiti da un agent relay per conto del server. |
| `dumbmonit_probes_failed_total{kind}` | counter | Probe falliti, per tipo — un monitor non raggiungibile o un errore di configurazione. `rate(dumbmonit_probes_failed_total[15m]) / rate(dumbmonit_probes_total[15m])` è il rapporto di fallimento. |
| `dumbmonit_samples_written_total` | counter | Campioni accettati da VictoriaMetrics. |
| `dumbmonit_sample_writes_failed_total` | counter | Batch che VictoriaMetrics ha rifiutato o a cui non ha risposto. Il batch resta nel buffer e viene ritentato; un valore in crescita con un `dumbmonit_samples_pending` in crescita significa che l'archivio è fermo o pieno. |
| `dumbmonit_samples_pending` | gauge | Campioni ancora nel buffer di scrittura dopo l'ultimo flush. Normalmente 0. |
| `dumbmonit_samples_dropped_total` | counter | Campioni scartati prima ancora di raggiungere il buffer di scrittura, perché era pieno o chiuso. Mai ritentati — un valore in crescita durante un'interruzione significa perdita di dati, non solo ritardo. |
| `dumbmonit_alerting_cycles_total` | counter | Cicli di alerting eseguiti (uno ogni `DUMBMONIT_ALERT_INTERVAL_SECS`, 30 s per impostazione predefinita). |
| `dumbmonit_alerting_cycle_seconds` | gauge | Durata dell'ultimo ciclo di alerting, tempo delle query incluso. Avvicinarsi all'intervallo significa che le regole interrogano più di quanto l'archivio possa servire. |
| `dumbmonit_alerting_rules_evaluated` | gauge | Regole valutate nell'ultimo ciclo. |
| `dumbmonit_alerting_rules_failed` | gauge | Regole la cui query è fallita nell'ultimo ciclo. I loro avvisi sono congelati, non risolti. Qualsiasi valore diverso da 0 merita uno sguardo ai log. |
| `dumbmonit_alerts{phase}` | gauge | Avvisi tracciati dal motore, per fase: `ok`, `pending` (condizione vera, `for` non trascorso), `firing`, `resolved`. |
| `dumbmonit_alerts_suppressed` | gauge | Avvisi nascosti perché un monitor padre è fermo (la fase effettiva `suppressed`). |
| `dumbmonit_alerts_silenced` | gauge | Avvisi coperti da una finestra di manutenzione. |
| `dumbmonit_alerts_learning` | gauge | Avvisi baseline ancora in apprendimento, e quindi silenziosi. |
| `dumbmonit_notifications_total{kind}` | counter | Notifiche inviate, per tipo di canale (`email`, `discord`, `ntfy`…). |
| `dumbmonit_notifications_failed_total{kind}` | counter | Notifiche che il canale ha rifiutato, per tipo. Vengono rimesse in coda e ritentate; un valore in costante crescita indica un webhook o un account SMTP guasto. |
| `dumbmonit_agents` | gauge | Macchine che hanno registrato un agent. |
| `dumbmonit_agents_stale` | gauge | Agent che non hanno inviato nulla da cinque minuti. |
| `dumbmonit_database_up` | gauge | 1 quando SQLite risponde — lo stesso controllo di `/api/health`. |
| `dumbmonit_database_bytes` | gauge | Dimensione di `dumbmonit.db` e del suo write-ahead log. Non include `/data/vm`. |
| `dumbmonit_victoriametrics_up` | gauge | 1 quando VictoriaMetrics risponde — lo stesso controllo di `/api/health`. 0 mentre il figlio incorporato si riavvia. |
| `dumbmonit_victoriametrics_embedded` | gauge | 1 quando questo server esegue VictoriaMetrics da sé, 0 quando `DUMBMONIT_VM_URL` punta a una esterna. |

Nessuna metrica di questa pagina porta un'etichetta per monitor: il documento
ha la stessa dimensione per un monitor e per mille, e nessun nome di
monitor viene rivelato a chi può fare lo scrape.

Due avvisi che vale la pena aggiungere dalla tua parte, una volta che DumbMonit è
sotto scrape:

```
# The monitoring stopped monitoring
up{job="dumbmonit"} == 0

# Measurements are piling up instead of being stored
dumbmonit_samples_pending > 0 and increase(dumbmonit_sample_writes_failed_total[10m]) > 0
```

### Aprire le rotte senza token

`DUMBMONIT_METRICS_PUBLIC=true` serve `/metrics`, `/federate` e
`/prometheus/api/v1/…` a chiunque possa raggiungere la porta, per chi filtra
la porta per conto proprio — **chiunque si trovi su quella rete può allora
leggere ogni misura di ogni monitor e lo stato dell'istanza, quindi
impostalo solo quando qualcos'altro limita già chi raggiunge la porta.**
