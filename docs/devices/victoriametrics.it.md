# VictoriaMetrics e VictoriaLogs

Il database delle serie temporali e il database dei log stessi: se accettano
quello che viene loro inviato, quanto spazio su disco rimane prima che smettano di accettare qualsiasi cosa,
e cosa rifiutano.

Due tipi di monitor condividono questa pagina, perché i due prodotti condividono i loro
fondamenti: `victoriametrics` per un singolo nodo VictoriaMetrics, e
`victorialogs` per un singolo nodo VictoriaLogs. Entrambi pubblicano la loro salute su
`/health` e `/metrics`, sulla porta della loro API; DumbMonit legge quei due
e nient'altro. Non esegue mai una query rispetto ai tuoi dati e mai scrive.

Questi server raramente smettono di funzionare completamente. Smettono di funzionare *a metà*:
continuano a rispondere alle query e i dashboard si aprono ancora, mentre ogni nuovo campione o riga di log
viene rifiutato, perché il disco ha attraversato il limite di sola lettura o i dati
arrivavano con un timestamp al di fuori della retention. Qualunque cosa li spinga (vmagent,
Prometheus remote write, Vector, Fluent Bit, Promtail) ritenta, quindi abbandona.
Questa pagina riguarda il vedere che questo sta arrivando.

## Cosa monitora

Le metriche hanno il prefisso `dumbmonit_victoriametrics_` o
`dumbmonit_victorialogs_`. I totali sono contatori: i grafici e le regole li trasformano in
frequenze.

| Metrica | Cosa | Etichette |
|---|---|---|
| `healthy` | 1 quando `/health` risponde OK, 0 quando risponde qualcos'altro | |
| `version_info` | valore 1 | `version` |
| `rows_ingested_total` | campioni (VictoriaMetrics) o righe di log (VictoriaLogs) accettati | |
| `bytes_ingested_total` | volume di log accettato (solo VictoriaLogs) | |
| `rows_rejected_total` | tutto rifiutato, sempre presente, anche a zero | |
| `rows_rejected_reason_total` | lo stesso, per motivo: `big_timestamp`, `small_timestamp`, `too_long_label_value`, `too_many_labels`, `invalid`, `hourly_series_limit`… (VictoriaMetrics); `too_big_timestamp`, `too_small_timestamp`, `too_many_fields`, `too_long_line` (VictoriaLogs) | `reason` |
| `disk_free_bytes`, `disk_total_bytes` | spazio libero e totale del volume di archiviazione | `path` |
| `disk_free_limit_bytes` | `-storage.minFreeDiskSpaceBytes`: sotto di esso, l'archiviazione diventa di sola lettura | `path` |
| `disk_headroom_percent` | spazio libero rimasto sopra quel limite, come una percentuale del disco | `path` |
| `read_only` | 1 una volta che l'archiviazione è passata a sola lettura | |
| `data_size_bytes` | dati su disco | |
| `slow_inserts_total`, `rows_added_total` | nuovi campioni che hanno preso il percorso lento, e tutti i campioni aggiunti (solo VictoriaMetrics) | |
| `active_series` | serie che hanno ricevuto un campione nell'ultima ora (solo VictoriaMetrics) | |
| `http_errors_total` | richieste risposte con un errore | |
| `log_errors_total` | messaggi di errore che il server ha registrato | |
| `uptime_seconds`, `memory_resident_bytes` | | |

**Un campione rifiutato è perso.** VictoriaMetrics rifiuta un campione più vecchio della sua
retention o più di due giorni nel futuro, un valore di etichetta più lungo di
`-maxLabelValueLen`, una serie oltre `-maxLabelsPerTimeseries`, e ogni campione
di una nuova serie una volta raggiunto `-storage.maxHourlySeries` o `-storage.maxDailySeries`.
VictoriaLogs rifiuta righe al di fuori della sua retention e righe con troppi
campi. Il client di solito non riceve errore per questi: solo questo contatore
lo sa.

**Gli inserimenti lenti** sono campioni la cui serie VictoriaMetrics ha dovuto cercare su
disco invece che nella sua cache. Ogni serie nuova fiammante è una, quindi una distribuzione
causa uno scoppio; una percentuale che rimane sopra il 5% significa che il server manca di memoria per
le sue serie attive. Quella è la soglia che la documentazione di VictoriaMetrics
dà. Dopo un riavvio le cache del server sono fredde, quindi la percentuale può stare alta
per un po': la regola aspetta mezz'ora prima di attivarsi. I contatori si ripristinano da
zero con il server; le regole utilizzano aumenti consapevoli del ripristino, quindi un riavvio non è
conteggiato come dati rifiutati.

Le [regole integrate](../alerting/rules.md#log-and-metrics-servers) che si applicano:
Archivio metriche o log di sola lettura, Archivio metriche o log quasi di sola lettura,
Server metriche o log non sano, Dati metriche o log rifiutati, Inserimenti lenti VictoriaMetrics,
più monitor irraggiungibile.

## La pagina del monitor

Il pannello sopra i grafici legge quello che la sonda ha memorizzato; aprire la pagina non
interroga mai il server. Dice, in questo ordine:

1. **Cosa è sbagliato, in una frase**: salute, archiviazione (*Sola lettura*, o quanta
   parte del disco rimane sopra il limite), dati rifiutati negli ultimi 24
   ore, inserimenti lenti, errori registrati. Ogni riga porta una parola così come un
   colore.
2. **Cosa fa il server**: campioni o righe acquisiti al secondo, serie attive o volume
   acquisito, dati su disco, uptime.
3. **Cosa è stato rifiutato negli ultimi 24 ore, per motivo**, e spazio libero di ogni percorso di archiviazione
   rispetto al suo limite di sola lettura.

## Consenti a DumbMonit di leggere la salute di VictoriaMetrics

1. Niente da installare: VictoriaMetrics pubblica la sua salute su /health e /metrics, sulla porta della sua API (8428 per impostazione predefinita). Controlla dall'host DumbMonit che entrambi rispondano.

    ```
    curl http://victoriametrics.lan:8428/health
    curl -s http://victoriametrics.lan:8428/metrics | grep vm_app_version
    ```

2. Se VictoriaMetrics viene eseguito con -httpAuth.username e -httpAuth.password, scegli Nome utente / password di seguito e inserisci quelli. Dietro vmauth o un reverse proxy che si aspetta un token bearer, scegli Token bearer. Se -metricsAuthKey è impostato, /metrics risponde solo con quella chiave nell'URL, che DumbMonit non invia: proteggi /metrics con l'autenticazione di base sopra invece.

3. In DumbMonit, inserisci l'indirizzo del server, ad esempio "victoriametrics.lan" o "http://10.0.0.5:8428". Dietro un reverse proxy, inserisci l'URL completo con il suo prefisso del percorso. Ciò copre il server a singolo nodo; i componenti di un cluster VictoriaMetrics espongono altre metriche e non sono coperti.

4. DumbMonit legge solo /health e /metrics. Non esegue mai una query rispetto ai tuoi dati e non scrive mai nulla.

!!! warning
    Lo spazio su disco libero è quello da osservare: sotto -storage.minFreeDiskSpaceBytes (100 MB per impostazione predefinita) VictoriaMetrics passa a sola lettura e rifiuta ogni nuovo campione, mentre continua a rispondere alle query come se nulla fosse accaduto. DumbMonit mostra lo spazio rimasto sopra quel limite e avverte sotto il 10%.

## Consenti a DumbMonit di leggere la salute di VictoriaLogs {#let-dumbmonit-read-victorialogs-own-health}

1. Niente da installare: VictoriaLogs pubblica la sua salute su /health e /metrics, sulla porta della sua API (9428 per impostazione predefinita). Controlla dall'host DumbMonit che entrambi rispondano.

    ```
    curl http://victorialogs.lan:9428/health
    curl -s http://victorialogs.lan:9428/metrics | grep vl_free_disk_space_bytes
    ```

2. Se VictoriaLogs viene eseguito con -httpAuth.username e -httpAuth.password, scegli Nome utente / password di seguito e inserisci quelli. Dietro vmauth o un reverse proxy che si aspetta un token bearer, scegli Token bearer. Se -metricsAuthKey è impostato, /metrics risponde solo con quella chiave nell'URL, che DumbMonit non invia: proteggi /metrics con l'autenticazione di base sopra invece.

3. In DumbMonit, inserisci l'indirizzo del server, ad esempio "victorialogs.lan" o "http://10.0.0.6:9428". Dietro un reverse proxy, inserisci l'URL completo con il suo prefisso del percorso.

4. DumbMonit legge solo /health e /metrics. Non esegue mai una query LogsQL e non legge mai una riga di log.

!!! warning
    Lo spazio su disco libero è quello da osservare: sotto -storage.minFreeDiskSpaceBytes (10 MB per impostazione predefinita) VictoriaLogs passa a sola lettura e rifiuta ogni nuova riga di log. DumbMonit mostra lo spazio rimasto sopra quel limite e avverte sotto il 10%.

## Credenziali

| Credenziale | Campi |
|---|---|
| Nessuna autenticazione | L'impostazione predefinita: `/health` e `/metrics` sono aperti a meno che non li abbia protetti. |
| Nome utente / password | Il `-httpAuth.username` e `-httpAuth.password` del server, o del reverse proxy davanti ad esso. |
| Token bearer | Un token che vmauth o il tuo reverse proxy si aspetta in `Authorization: Bearer`. |

Indirizzo: un nome host o IP (`victoriametrics.lan`, `10.0.0.5`), `host:port`,
o un URL completo con un prefisso del percorso dietro un reverse proxy
(`https://metrics.lan/vm`). Opzioni: protocollo (HTTP per impostazione predefinita), porta (8428 o
9428), verifica certificato e timeout delle richieste (10 s).
