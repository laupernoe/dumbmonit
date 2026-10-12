# VictoriaMetrics e VictoriaLogs

Il database di serie temporali e il database di log stessi: se accettano
ciò che viene loro inviato, quanto disco resta prima che smettano di
accettare qualsiasi cosa, e cosa rifiutano.

Due tipi di dispositivo condividono questa pagina, perché i due prodotti
condividono le loro fondamenta: `victoriametrics` per un VictoriaMetrics a
nodo singolo, e `victorialogs` per un VictoriaLogs a nodo singolo.
Entrambi pubblicano il loro stato di salute su `/health` e `/metrics`,
sulla porta della loro API; DumbMonit legge solo questi due. Non esegue
mai una query sui tuoi dati e non scrive mai.

Questi server raramente si fermano del tutto. Si fermano *a metà*:
continuano a rispondere alle query e le dashboard si aprono ancora, mentre
ogni nuovo campione o riga di log viene rifiutata, perché il disco ha
superato il limite di sola lettura o i dati sono arrivati con un
timestamp fuori dalla retention. Qualunque cosa invii loro dati (vmagent,
Prometheus remote write, Vector, Fluent Bit, Promtail) ritenta, poi
scarta. Questa pagina serve a vederlo arrivare in anticipo.

## Cosa controlla

Le metriche hanno il prefisso `dumbmonit_victoriametrics_` oppure
`dumbmonit_victorialogs_`. I totali sono contatori: grafici e regole li
trasformano in tassi.

| Metrica | Cosa | Etichette |
|---|---|---|
| `healthy` | 1 quando `/health` risponde OK, 0 quando risponde qualsiasi altra cosa | |
| `version_info` | valore 1 | `version` |
| `rows_ingested_total` | campioni (VictoriaMetrics) o righe di log (VictoriaLogs) accettati | |
| `bytes_ingested_total` | volume di log accettato (solo VictoriaLogs) | |
| `rows_rejected_total` | tutto ciò che è stato rifiutato, sempre presente, anche a zero | |
| `rows_rejected_reason_total` | lo stesso, per motivo: `big_timestamp`, `small_timestamp`, `too_long_label_value`, `too_many_labels`, `invalid`, `hourly_series_limit`… (VictoriaMetrics); `too_big_timestamp`, `too_small_timestamp`, `too_many_fields`, `too_long_line` (VictoriaLogs) | `reason` |
| `disk_free_bytes`, `disk_total_bytes` | spazio libero e totale del volume di storage | `path` |
| `disk_free_limit_bytes` | `-storage.minFreeDiskSpaceBytes`: sotto questo, lo storage diventa di sola lettura | `path` |
| `disk_headroom_percent` | spazio libero rimanente sopra quel limite, come quota del disco | `path` |
| `read_only` | 1 una volta che lo storage è passato a sola lettura | |
| `data_size_bytes` | dati su disco | |
| `slow_inserts_total`, `rows_added_total` | nuovi campioni che hanno preso il percorso lento, e tutti i campioni aggiunti (solo VictoriaMetrics) | |
| `active_series` | serie che hanno ricevuto un campione nell'ultima ora (solo VictoriaMetrics) | |
| `http_errors_total` | richieste risposte con un errore | |
| `log_errors_total` | messaggi di errore che il server ha registrato | |
| `uptime_seconds`, `memory_resident_bytes` | | |

**Un campione rifiutato è perso.** VictoriaMetrics rifiuta un campione più
vecchio della sua retention o più di due giorni nel futuro, un valore di
etichetta più lungo di `-maxLabelValueLen`, una serie sopra
`-maxLabelsPerTimeseries`, e ogni campione di una nuova serie una volta
raggiunto `-storage.maxHourlySeries` o `-storage.maxDailySeries`.
VictoriaLogs rifiuta righe fuori dalla sua retention e righe con troppi
campi. Il client di solito non riceve alcun errore per questi: solo
questo contatore lo sa.

**Gli insert lenti** sono campioni la cui serie VictoriaMetrics ha dovuto
cercare su disco invece che nella sua cache. Ogni nuova serie ne è una,
quindi un deployment causa un picco; una quota che resta sopra il 5%
significa che il server manca di memoria per le sue serie attive. Quella è
la soglia che la documentazione di VictoriaMetrics dà.

Le [regole integrate](../alerting/rules.md#log-and-metrics-servers) che si
applicano: Metrics or log storage read-only, Metrics or log storage almost
read-only, Metrics or log server unhealthy, Metrics or log data refused,
VictoriaMetrics slow inserts, più Device unreachable.

## La pagina del dispositivo

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire
la pagina non interroga mai il server. Indica, in questo ordine:

1. **Cosa non va, in una frase**: stato di salute, storage (*Read-only*,
   oppure quanto disco resta sopra il limite), dati rifiutati nelle ultime
   24 ore, insert lenti, errori registrati. Ogni riga porta una parola
   oltre che un colore.
2. **Cosa fa il server**: campioni o righe ingeriti al secondo, serie
   attive o volume ingerito, dati su disco, uptime.
3. **Cosa è stato rifiutato nelle ultime 24 ore, per motivo**, e lo spazio
   libero di ogni percorso di storage rispetto al suo limite di sola
   lettura.

## Lasciare che DumbMonit legga lo stato di salute proprio di VictoriaMetrics

1. Nulla da installare: VictoriaMetrics pubblica il proprio stato di salute su /health e /metrics, sulla porta della sua API (8428 per impostazione predefinita). Verifica dall'host DumbMonit che entrambi rispondano.

    ```
    curl http://victoriametrics.lan:8428/health
    curl -s http://victoriametrics.lan:8428/metrics | grep vm_app_version
    ```

2. Se VictoriaMetrics funziona con -httpAuth.username e -httpAuth.password, scegli Username / password più sotto e inseriscili. Dietro vmauth o un reverse proxy che si aspetta un token bearer, scegli Bearer token. Se -metricsAuthKey è impostato, /metrics risponde solo con quella chiave nell'URL, che DumbMonit non invia: proteggi /metrics con l'autenticazione di base sopra invece.

3. In DumbMonit, inserisci l'indirizzo del server, per esempio "victoriametrics.lan" oppure "http://10.0.0.5:8428". Dietro un reverse proxy, inserisci l'URL completo con il suo prefisso di percorso. Questo copre il server a nodo singolo; i componenti di un cluster VictoriaMetrics espongono altre metriche e non sono coperti.

4. DumbMonit legge solo /health e /metrics. Non esegue mai una query sui tuoi dati e non scrive mai nulla.

!!! warning
    Lo spazio libero su disco è ciò da osservare: sotto -storage.minFreeDiskSpaceBytes (100 MB per impostazione predefinita) VictoriaMetrics passa a sola lettura e rifiuta ogni nuovo campione, mentre continua a rispondere alle query come se nulla fosse successo. DumbMonit mostra il margine rimanente sopra quel limite e avvisa sotto il 10%.

## Lasciare che DumbMonit legga lo stato di salute proprio di VictoriaLogs

1. Nulla da installare: VictoriaLogs pubblica il proprio stato di salute su /health e /metrics, sulla porta della sua API (9428 per impostazione predefinita). Verifica dall'host DumbMonit che entrambi rispondano.

    ```
    curl http://victorialogs.lan:9428/health
    curl -s http://victorialogs.lan:9428/metrics | grep vl_rows_ingested_total
    ```

2. Se VictoriaLogs funziona con -httpAuth.username e -httpAuth.password, scegli Username / password più sotto e inseriscili. Dietro vmauth o un reverse proxy che si aspetta un token bearer, scegli Bearer token. Se -metricsAuthKey è impostato, /metrics risponde solo con quella chiave nell'URL, che DumbMonit non invia: proteggi /metrics con l'autenticazione di base sopra invece.

3. In DumbMonit, inserisci l'indirizzo del server, per esempio "victorialogs.lan" oppure "http://10.0.0.6:9428". Dietro un reverse proxy, inserisci l'URL completo con il suo prefisso di percorso.

4. DumbMonit legge solo /health e /metrics. Non esegue mai una query LogsQL e non legge mai una riga di log.

!!! warning
    Lo spazio libero su disco è ciò da osservare: sotto -storage.minFreeDiskSpaceBytes (10 MB per impostazione predefinita) VictoriaLogs passa a sola lettura e rifiuta ogni nuova riga di log. DumbMonit mostra il margine rimanente sopra quel limite e avvisa sotto il 10%.

## Credenziali

| Credenziale | Campi |
|---|---|
| No authentication | Il valore predefinito: `/health` e `/metrics` sono aperti a meno che tu non li abbia protetti. |
| Username / password | `-httpAuth.username` e `-httpAuth.password` del server, oppure del reverse proxy davanti a esso. |
| Bearer token | Un token che vmauth o il tuo reverse proxy si aspetta in `Authorization: Bearer`. |

Indirizzo: un nome host o IP (`victoriametrics.lan`, `10.0.0.5`),
`host:porta`, oppure un URL completo con un prefisso di percorso dietro un
reverse proxy (`https://metrics.lan/vm`). Opzioni: protocollo (HTTP per
impostazione predefinita), porta (8428 o 9428), verifica del certificato e
timeout della richiesta (10 s).
