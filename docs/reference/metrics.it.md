# Metriche

DumbMonit memorizza ogni misurazione in VictoriaMetrics e la rilegge
con MetricsQL, tramite il [proxy di metriche](api.md#metrics) del
server. Questa pagina descrive la denominazione, le etichette e la
retention. Gli elenchi di metriche per tipo sono sulle
[pagine dei dispositivi](../devices/index.md).

## Denominazione

Ogni metrica ha il prefisso `dumbmonit_`, così DumbMonit può condividere
un'istanza VictoriaMetrics con altri strumenti senza collisioni. I nomi
seguono `dumbmonit_<cosa>_<unità>`:

| Pattern | Esempi |
|---|---|
| `…_bytes`, `…_bytes_total`, `…_bytes_used` | `dumbmonit_memory_bytes_total`, `dumbmonit_filesystem_used_bytes` |
| `…_percent` | `dumbmonit_cpu_usage_percent`, `dumbmonit_pbs_datastore_used_percent` |
| `…_seconds`, `…_age_seconds` | `dumbmonit_probe_duration_seconds`, `dumbmonit_proxmox_backup_last_age_seconds` |
| `…_celsius`, `…_volts`, `…_hertz`, `…_watts` | letture UPS e Synology |
| `…_info` | Serie di presenza (valore 1) la cui informazione è nelle etichette: `dumbmonit_system_info`, `dumbmonit_pbs_version_info` |
| `…_status`, `…_up`, `…_running` | Enumerazioni o booleani: `dumbmonit_if_oper_status`, `dumbmonit_proxmox_guest_running` |

Le famiglie specifiche di una fonte portano la loro fonte come secondo
prefisso: `dumbmonit_proxmox_*`, `dumbmonit_pbs_*`, `dumbmonit_synology_*`,
`dumbmonit_probe_*` (servizi). I profili SNMP e l'agent usano nomi
semplici (`dumbmonit_if_octets_in`, `dumbmonit_cpu_usage_percent`).

I contatori (octet e pacchetti di interfaccia, byte di disco, pagine
stampate) sono memorizzati **grezzi**, come il dispositivo li riporta:
applica `rate()` nelle tue query per ottenere un throughput. La pagina
del dispositivo grafica ogni serie `dumbmonit_*` del dispositivo come
memorizzata, così un contatore appare come una linea crescente. Un
riavvio appare come un azzeramento del contatore, mai come un picco
fittizio.

## `dumbmonit_up`

Dopo ogni sonda riuscita, il server scrive `dumbmonit_up = 1` per il
dispositivo. Nulla viene scritto quando una sonda fallisce: la serie
semplicemente si ferma, e la regola "Device unreachable" misura l'età
dell'ultimo campione
(`time() - tlast_over_time(dumbmonit_up[7d])`).

Per i servizi (`http`, `tcp`, `dns`, `ping`, `tls`), `dumbmonit_up = 1`
significa "il monitor è stato eseguito", e `dumbmonit_probe_success`
dice se il servizio va bene. I due segnali sono mantenuti separati di
proposito: un `probe_success` di 0 dice che il servizio è inattivo,
un'interruzione di `up` dice che il monitoraggio è inattivo.

## `dumbmonit_security_score`

Ogni 15 minuti il server calcola il
[punteggio di sicurezza](../using/security-score.md) di ogni
dispositivo il cui tipo ha controlli di sicurezza, e scrive
`dumbmonit_security_score` (0-100, assente mentre nulla può essere
valutato) e `dumbmonit_security_checks_failed`, entrambi etichettati
`target`.

## Etichette

| Etichetta | Impostata da | Significato |
|---|---|---|
| `target` | server | L'id del dispositivo, come in `/api/targets/{id}`. Ogni regola e ogni grafico seleziona su di esso. |
| `host` | server | Il nome del dispositivo. |
| `tag_<key>` | server | Ogni tag del dispositivo, incluse le opzioni di tipo (`tag_insecure_tls`, `tag_port`…). Non metterci mai un segreto. |
| `probe` | servizi | `http`, `tcp`, `dns`, `ping` o `tls`. |
| etichette per serie | collector | `ifname`, `mountpoint`, `device`, `fstype`, `core`, `service`, `container`, `node`, `vmid`, `name`, `storage`, `datastore`, `group`, `volume`, `disk`, `pool`, `repo`, `changes`, `reason`, `version`, `issuer`… come elencato su ogni pagina dispositivo. |

Le etichette di identità vincono sempre sulle etichette del collector, e
per gli agent vengono impostate alla ricezione: una macchina non può
scrivere nella serie di un'altra macchina.

!!! note "Prefisso prima di settembre 2026"
    Le serie scritte da EzyMonit sono nominate `ezymonit_…`. Il prefisso
    è cambiato con il nome del prodotto e nulla legge le vecchie serie:
    restano in VictoriaMetrics finché la retention non le rimuove, i
    grafici e le regole riprendono dall'aggiornamento, e le regole
    personalizzate che ancora nominano `ezymonit_…` devono essere
    modificate.

## Retention e storage

Il VictoriaMetrics integrato mantiene 12 mesi di dati
(`DUMBMONIT_VM_RETENTION`) sotto `/data/vm`, nel volume `dumbmonit-data`.
Il server raggruppa le sue scritture e le scarica ogni 5 secondi
(`DUMBMONIT_FLUSH_INTERVAL_SECS`). Eliminare un dispositivo non elimina
le sue serie; scadono con il tempo.

La cronologia degli avvisi viene mantenuta 90 giorni e le baseline di
serie scomparse 60 giorni, entrambe in SQLite (vedi il
[riferimento di configurazione](configuration.md)).

## Interrogare

- Tramite il server: `GET /api/metrics/query` e
  `GET /api/metrics/query_range`, con il cookie di sessione. I risultati
  di intervallo sono limitati a 2.000 punti per serie.
- Da un Prometheus o Grafana esistente: `GET /federate`, `GET /metrics`
  e la data source `/prometheus`, con un token API —
  [sotto](#scraping-dumbmonit).
- Direttamente, in sviluppo: l'overlay `docker-compose.dev.yml` pubblica
  il VictoriaMetrics integrato su `http://localhost:8428`, con la sua
  propria interfaccia su `http://localhost:8428/vmui/`. In produzione
  ascolta solo sul loopback del container; imposta
  `DUMBMONIT_VM_LISTEN=0.0.0.0:8428` e pubblica la porta da solo se
  vuoi Grafana o un altro strumento sui medesimi dati.

```
# Throughput dell'interfaccia in bit al secondo
rate(dumbmonit_if_octets_in{target="4"}[5m]) * 8

# Disponibilità di ogni servizio su 30 giorni
avg by (host) (avg_over_time(dumbmonit_probe_success[30d]))

# Ogni dispositivo che non ha riportato da 5 minuti
time() - tlast_over_time(dumbmonit_up[7d]) > 300
```

## Sondare DumbMonit

Se già esegui un Prometheus o un Grafana, non hai bisogno di un secondo
stack. Tre rotte stanno fuori da `/api`, dove uno scraper le cerca:

| Rotta | Cosa restituisce |
|---|---|
| `GET /metrics` | Lo stato di salute dell'istanza DumbMonit stessa: scheduler, sonde, scritture, alerting, notifiche, agent, database. Qualche decina di serie, sempre lo stesso numero — mai una per dispositivo. |
| `GET /federate` | Le misurazioni, selezionate da `match[]`, esattamente come Prometheus federa un altro Prometheus: l'ultimo punto di ogni serie selezionata. |
| `GET /prometheus/api/v1/…` | La metà di lettura dell'API Prometheus, inoltrata allo store: ciò che chiama una data source Grafana. |

Tutte e tre richiedono `Authorization: Bearer dmt_…` con l'ambito
**`read`** — un token creato in Settings → *API & assistants*, lo
stesso tipo che usa il [server MCP](../using/assistant.md). Nessun
cookie è coinvolto, quindi nessun header `X-Requested-With` è
necessario; un cookie di sessione viene accettato anch'esso, così puoi
aprire qualsiasi di questi da un browser connesso. Un token è limitato
a 120 chiamate al minuto: abbondante per un sondaggio ogni 15 secondi,
e sufficiente per una dashboard Grafana di qualche decina di pannelli —
dai a una dashboard occupata il proprio token invece di condividerne
uno con uno scraper.

### Prometheus

```yaml
# prometheus.yml — lo stato di salute proprio di DumbMonit, poi le misurazioni che raccoglie.
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

`honor_labels: true` mantiene le etichette `target` e `host` che
DumbMonit imposta invece di sovrascriverle con quelle proprie del job
di sondaggio. Metti il token in un file e usa
`authorization: {type: Bearer, credentials_file: /etc/prometheus/dumbmonit.token}`
se preferisci non averlo nella configurazione.

Senza `match[]`, `/federate` seleziona `{__name__=~"dumbmonit_.*"}` —
ogni serie che DumbMonit scrive, e nulla appartenente a un altro
strumento che condivide lo stesso VictoriaMetrics. Restringilo per
mantenere piccola una federazione: `{__name__=~"dumbmonit_.*",target="4"}`
per un dispositivo, `{__name__="dumbmonit_up"}` solo per la
disponibilità. Al massimo 10 selettori per richiesta, e una risposta
sopra 8 MiB viene rifiutata con un messaggio che ti dice di
restringerla — dividi una grande flotta in diversi job con selettori
più ristretti invece di uno che chiede tutto.
`max_lookback=15m` amplia la finestra in cui viene cercato l'ultimo
punto, per dispositivi sondati meno spesso di ogni cinque minuti.

```bash
curl -H 'Authorization: Bearer dmt_…' http://localhost:8080/metrics
curl -H 'Authorization: Bearer dmt_…' -G http://localhost:8080/federate \
  --data-urlencode 'match[]={__name__=~"dumbmonit_.*",target="4"}'
```

### Grafana

Punta Grafana su **DumbMonit**, non sul VictoriaMetrics che esegue: il
processo figlio ascolta sul loopback del container, non ha alcuna
autenticazione propria, ed espone le rotte che *eliminano* le serie
tanto facilmente quanto quelle che le leggono. DumbMonit inoltra la
metà di lettura dell'API Prometheus sotto `/prometheus`, dietro lo
stesso token, così Grafana parla con una normale data source
Prometheus:

1. **Connections → Data sources → Add → Prometheus**.
2. **URL**: `http://dumbmonit.lan:8080/prometheus` (Grafana aggiunge
   `/api/v1/query` da solo).
3. **HTTP headers**: aggiungi uno, `Authorization` = `Bearer dmt_…`.
   Grafana lo memorizza cifrato e non lo mostra più.
4. **Save & test** — Grafana riporta che la data source funziona, e
   ogni serie `dumbmonit_*` è allora disponibile in PromQL, con
   autocompletamento sui nomi di metriche ed etichette.

Solo le rotte di lettura vengono inoltrate — `query`, `query_range`,
`series`, `labels`, valori di etichetta, `metadata` e la versione che
Grafana chiede quando testa la connessione. Qualsiasi altra cosa,
`admin/tsdb/delete_series` prima tra tutte, risponde `404`: questo
punto di accesso non può modificare o eliminare nulla.

Se già esegui un Prometheus, l'altro percorso funziona altrettanto
bene: Prometheus federa DumbMonit con il job sopra, Grafana legge
Prometheus, e le query sono identiche.

### Metriche dell'istanza

Tutto ciò che `GET /metrics` espone. I valori vengono letti quando
sondi; i contatori `_total` sono monotoni e si azzerano solo quando il
server si riavvia (`dumbmonit_uptime_seconds` ti dice quando è
successo).

| Metrica | Tipo | Significato |
|---|---|---|
| `dumbmonit_build_info{version}` | gauge | Sempre 1. L'etichetta `version` è la versione del server in esecuzione, la stessa che riporta `/api/health`. |
| `dumbmonit_uptime_seconds` | gauge | Secondi da quando il processo server è stato avviato. |
| `dumbmonit_scheduler_cycles_total` | counter | Cicli dello scheduler eseguiti. Il loop batte una volta al secondo, quindi questo funziona anche come segnale di liveness. |
| `dumbmonit_scheduler_cycle_seconds` | gauge | Durata dell'ultimo ciclo. Normalmente sotto un millisecondo: il ciclo solo distribuisce, non attende le sonde. |
| `dumbmonit_scheduler_backlog` | gauge | Dispositivi ancora in attesa di una sonda quando l'ultimo ciclo è terminato. Zero su un'istanza sana; un valore non-zero persistente significa che `DUMBMONIT_MAX_CONCURRENT_PROBES` è troppo basso per il numero di dispositivi e i loro intervalli. |
| `dumbmonit_probes_total{kind}` | counter | Sonde eseguite, per tipo di dispositivo (`snmp`, `proxmox`, `http`…). Include quelle che un agent relay ha eseguito per conto del server. |
| `dumbmonit_probes_failed_total{kind}` | counter | Sonde fallite, per tipo — un dispositivo irraggiungibile o un errore di configurazione. `rate(dumbmonit_probes_failed_total[15m]) / rate(dumbmonit_probes_total[15m])` è il tasso di fallimento. |
| `dumbmonit_samples_written_total` | counter | Campioni accettati da VictoriaMetrics. |
| `dumbmonit_sample_writes_failed_total` | counter | Batch che VictoriaMetrics ha rifiutato o non ha risposto. Il batch resta bufferizzato e viene ritentato; un valore crescente con un `dumbmonit_samples_pending` crescente significa che lo store è inattivo o pieno. |
| `dumbmonit_samples_pending` | gauge | Campioni ancora nel buffer di scrittura dopo l'ultimo flush. Normalmente 0. |
| `dumbmonit_samples_dropped_total` | counter | Campioni scartati prima di raggiungere il buffer di scrittura, perché era pieno o chiuso. Mai ritentati — un valore crescente durante un'interruzione significa perdita di dati, non solo ritardo. |
| `dumbmonit_alerting_cycles_total` | counter | Cicli di alerting eseguiti (uno ogni `DUMBMONIT_ALERT_INTERVAL_SECS`, 30 s per impostazione predefinita). |
| `dumbmonit_alerting_cycle_seconds` | gauge | Durata dell'ultimo ciclo di alerting, tempo di query incluso. Avvicinarsi all'intervallo significa che le regole interrogano più di quanto lo store possa servire. |
| `dumbmonit_alerting_rules_evaluated` | gauge | Regole valutate nell'ultimo ciclo. |
| `dumbmonit_alerting_rules_failed` | gauge | Regole la cui query è fallita nell'ultimo ciclo. I loro avvisi sono congelati, non risolti. Qualsiasi cosa diversa da 0 merita un'occhiata ai log. |
| `dumbmonit_alerts{phase}` | gauge | Avvisi che il motore traccia, per fase: `ok`, `pending` (condizione vera, `for` non trascorso), `firing`, `resolved`. |
| `dumbmonit_alerts_suppressed` | gauge | Avvisi nascosti perché un dispositivo genitore è inattivo (la fase effettiva `suppressed`). |
| `dumbmonit_alerts_silenced` | gauge | Avvisi coperti da una finestra di manutenzione. |
| `dumbmonit_alerts_learning` | gauge | Avvisi di baseline ancora in apprendimento, e quindi silenziosi. |
| `dumbmonit_notifications_total{kind}` | counter | Notifiche inviate, per tipo di canale (`email`, `discord`, `ntfy`…). |
| `dumbmonit_notifications_failed_total{kind}` | counter | Notifiche che il canale ha rifiutato, per tipo. Vengono rimesse in coda e ritentate; un valore in costante crescita è un webhook o un account SMTP rotto. |
| `dumbmonit_agents` | gauge | Macchine che hanno registrato un agent. |
| `dumbmonit_agents_stale` | gauge | Agent che non hanno inviato nulla da cinque minuti. |
| `dumbmonit_database_up` | gauge | 1 quando SQLite risponde — lo stesso controllo di `/api/health`. |
| `dumbmonit_database_bytes` | gauge | Dimensione di `dumbmonit.db` e il suo write-ahead log. Non include `/data/vm`. |
| `dumbmonit_victoriametrics_up` | gauge | 1 quando VictoriaMetrics risponde — lo stesso controllo di `/api/health`. 0 mentre il figlio integrato si riavvia. |
| `dumbmonit_victoriametrics_embedded` | gauge | 1 quando questo server esegue VictoriaMetrics da solo, 0 quando `DUMBMONIT_VM_URL` punta a uno esterno. |

Nessuna metrica su questa pagina porta un'etichetta per dispositivo: il
documento ha la stessa dimensione per un dispositivo e per mille, e
nessun nome di dispositivo filtra verso chi può sondarlo.

Due avvisi che vale la pena aggiungere dal tuo lato, una volta che
DumbMonit viene sondato:

```
# Il monitoraggio ha smesso di monitorare
up{job="dumbmonit"} == 0

# Le misurazioni si accumulano invece di essere memorizzate
dumbmonit_samples_pending > 0 and increase(dumbmonit_sample_writes_failed_total[10m]) > 0
```

### Aprire le rotte senza un token

`DUMBMONIT_METRICS_PUBLIC=true` serve `/metrics`, `/federate` e
`/prometheus/api/v1/…` a chiunque possa raggiungere la porta, per chi
filtra la porta da solo — **chiunque su quella rete può allora leggere
ogni misurazione di ogni dispositivo e lo stato dell'istanza, quindi
impostalo solo quando qualcos'altro già limita chi raggiunge la
porta.**
