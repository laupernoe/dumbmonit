# Grafana Loki

Il server di log stesso: se è pronto, quali righe di log rifiuta e perché, se
i suoi chunk raggiungono lo storage, e se il suo write-ahead log ha ancora
spazio su disco.

Loki pubblica `/ready` e `/metrics` sulla sua porta HTTP; DumbMonit legge
solo questi due e nulla altro. Non esegue mai una query LogQL e non legge mai
una riga di log.

Il guasto da tenere d'occhio è silenzioso. Quando Loki rifiuta una riga (più
vecchia di `reject_old_samples_max_age`, oltre il limite di ingestione o di
stream del tenant, più lunga di `max_line_size`) chi la invia riceve un 4xx,
e Promtail, Alloy o Fluent Bit di solito scartano il batch. Grafana non
mostra altro che un buco, scoperto il giorno in cui qualcuno cerca il log di
un incidente. Il secondo guasto sono i chunk che non possono essere scritti
sullo storage: Loki li mantiene in memoria, e li perde al riavvio successivo.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_loki_`. I totali sono
contatori: grafici e regole li trasformano in tassi.

| Metrica | Cosa | Etichette |
|---|---|---|
| `ready` | 1 quando `/ready` risponde `ready`, 0 mentre Loki si avvia o quando un componente non può scrivere o leggere | |
| `version_info` | valore 1 | `version` |
| `received_lines_total`, `received_bytes_total` | ciò che il distributor ha ricevuto, tutti i tenant | |
| `discarded_lines_total` | righe rifiutate, sempre presente, anche a zero | |
| `discarded_lines_reason_total` | lo stesso per motivo: `greater_than_max_sample_age`, `rate_limited`, `stream_limit`, `line_too_long`, `per_stream_rate_limit`… | `reason` |
| `flush_failures_total`, `flush_queue_length` | chunk che non sono riusciti a raggiungere lo storage, chunk in attesa di farlo | |
| `memory_streams` | stream mantenuti in memoria dall'ingester | |
| `wal_disk_full_failures_total`, `wal_disk_used_percent` | scritture del write-ahead log perse per un disco pieno, e quanto è pieno quel disco | |
| `requests_total`, `request_errors_total` | richieste HTTP e gRPC, e quelle risposte con un 5xx o un errore gRPC. Un 4xx è colpa del client e non viene contato come errore. | |
| `ring_unhealthy_members` | membri degli hash ring segnati come non sani | |
| `panics_total`, `log_errors_total` | panic recuperati, messaggi di errore registrati nei log | |
| `retention_last_run_timestamp_seconds` | ultima esecuzione riuscita della retention del compactor, quando la retention è attiva | |
| `memory_resident_bytes` | | |

Loki suddiviso in componenti (simple scalable, microservices) espone queste
famiglie dove appartengono: righe rifiutate sul distributor, flush e WAL
sull'ingester. Aggiungi i componenti che vuoi osservare; una famiglia che un
componente non ha semplicemente non produce alcuna serie.

Le [regole integrate](../alerting/rules.md#log-and-metrics-servers) che si
applicano: Loki not ready, Loki refusing log lines, Loki cannot flush to
storage, Loki write-ahead log disk full, Loki request errors, più Device
unreachable.

## La pagina del dispositivo

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire la
pagina non interroga mai Loki. Indica cosa non va in una frase (readiness,
righe rifiutate nelle ultime 24 ore, chunk non riusciti a essere scaricati,
un disco WAL pieno, membri del ring non sani, errori del server), poi le
righe e il volume ricevuti al secondo, gli stream in memoria e il disco WAL,
infine le righe rifiutate nelle ultime 24 ore per motivo.

## Lasciare che DumbMonit legga lo stato di salute proprio di Loki

1. Nulla da installare: Loki pubblica /ready e /metrics sulla sua porta HTTP (3100 per impostazione predefinita). Verifica dall'host DumbMonit che entrambi rispondano.

    ```
    curl http://loki.lan:3100/ready
    curl -s http://loki.lan:3100/metrics | grep loki_build_info
    ```

2. Loki non ha un proprio login. Se si trova dietro un reverse proxy che chiede un nome utente e una password, scegli Username / password più sotto; per un token bearer, scegli Bearer token. Non è necessario alcun tenant: /ready e /metrics non sono per tenant.

3. In DumbMonit, inserisci l'indirizzo di Loki, per esempio "loki.lan", oppure "https://logs.lan/loki" dietro un reverse proxy. Con Loki suddiviso in componenti (simple scalable o microservices), aggiungi almeno il percorso di scrittura, dove vengono contate le righe rifiutate e i fallimenti di flush.

4. DumbMonit legge solo /ready e /metrics. Non esegue mai una query LogQL e non legge mai una riga di log.

!!! warning
    Le righe rifiutate sono il guasto da osservare: quando Loki rifiuta una riga (troppo vecchia, oltre il limite di ingestione o di stream, riga troppo lunga), chi la invia riceve un errore e di solito scarta il batch. Grafana non mostra nulla a parte un buco nei log.

## Credenziali

| Credenziale | Campi |
|---|---|
| No authentication | Il valore predefinito: Loki stesso non ha un login. |
| Username / password | L'autenticazione di base del reverse proxy davanti a Loki. |
| Bearer token | Un token che il tuo reverse proxy si aspetta in `Authorization: Bearer`. |

Indirizzo: un nome host o IP (`loki.lan`), `host:porta`, oppure un URL
completo con un prefisso di percorso (`https://logs.lan/loki`). Opzioni:
protocollo (HTTP per impostazione predefinita), porta (3100), verifica del
certificato e timeout della richiesta (10 s).
