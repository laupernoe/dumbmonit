# RabbitMQ

Il message broker: allarmi di memoria e disco, nodi fermi e partizioni di
rete, code che si riempiono senza nessuno che le legga, code che non sono
più in esecuzione, e il flusso dei messaggi.

DumbMonit legge l'API di management con un utente con il tag `monitoring`
che ha permessi vuoti: vede lo stato dei nodi, gli allarmi e le code, e non
può pubblicare, consumare, leggere un messaggio o modificare nulla. Un solo
nodo riporta l'intero cluster.

Il guasto da tenere d'occhio è quello che i client non segnalano. Quando un
nodo supera il suo limite massimo di memoria o il suo limite di disco
libero, RabbitMQ genera un allarme e blocca ogni publisher nel cluster: le
applicazioni semplicemente si bloccano su `publish`, senza alcun errore. Il
secondo è una coda che continua a riempirsi perché il servizio che la
consumava si è fermato.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_rabbitmq_`. I totali sono
contatori: grafici e regole li trasformano in tassi.

| Metrica | Cosa | Etichette |
|---|---|---|
| `version_info` | valore 1 | `version`, `erlang`, `cluster` |
| `connections`, `channels`, `consumers`, `queues` | totali del cluster | |
| `messages_ready`, `messages_unacked` | messaggi in attesa in ogni coda, e consegnati ma non ancora confermati | |
| `messages_published_total`, `messages_delivered_total`, `messages_redelivered_total`, `messages_unroutable_total` | flusso dei messaggi; unroutable sono messaggi che nessuna coda era vincolata a ricevere | |
| `alarm` | 1 mentre l'allarme è attivo; memoria e disco sono sempre presenti, a 0 senza allarme | `node`, `resource` |
| `alarms` | allarmi attivi nel cluster | |
| `nodes`, `nodes_running`, `partitions` | | |
| `node_running`, `node_partitions` | per nodo | `node` |
| `node_memory_used_bytes`, `node_memory_limit_bytes`, `node_memory_used_percent` | memoria rispetto al limite massimo | `node` |
| `node_disk_free_bytes`, `node_disk_free_limit_bytes` | disco libero rispetto al limite | `node` |
| `node_fd_used`, `node_fd_used_percent`, `node_uptime_seconds`, `node_being_drained` | | `node` |
| `queue_messages_ready`, `queue_messages_unacked`, `queue_consumers`, `queue_running` | per coda, le più piene prima, fino a Queues watched one by one | `vhost`, `queue` |
| `queues_without_consumers`, `queues_not_running` | tra quelle code | |

Gli allarmi vengono letti dall'health check `/api/health/checks/alarms`, che
li vede all'istante; le statistiche dei nodi sono in ritardo di qualche
secondo.

Le [regole integrate](../alerting/rules.md#databases-message-broker-and-security-engine)
che si applicano: RabbitMQ memory or disk alarm, RabbitMQ node down,
RabbitMQ network partition, RabbitMQ queue without consumer, RabbitMQ queue
unavailable, più Monitor unreachable.

## La pagina del monitor

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire la
pagina non interroga mai RabbitMQ. Indica cosa non va in una frase (allarmi,
nodi e partizioni, code che contengono messaggi senza consumer, code non in
esecuzione), poi i messaggi pronti e non confermati, i messaggi pubblicati e
consegnati al secondo, connessioni, consumer e code, poi le dieci code più
piene con i loro consumer e la memoria e il disco libero di ogni nodo.

## Creare un utente RabbitMQ con il tag monitoring

1. Abilita il plugin di management se non è già attivo (le immagini Docker management lo includono). La sua API ascolta sulla porta 15672.

    ```
    rabbitmq-plugins enable rabbitmq_management
    ```

2. Crea un utente solo con il tag monitoring: può leggere lo stato dei nodi, degli allarmi e delle code, e non può modificare nulla.

    ```
    rabbitmqctl add_user dumbmonit 'a-long-password'
    rabbitmqctl set_user_tags dumbmonit monitoring
    ```

3. Dagli permessi vuoti su ogni virtual host le cui code vuoi vedere. Potrà allora elencare quelle code, e comunque non potrà pubblicare, consumare o leggere un messaggio. Ripeti con -p per ogni altro virtual host.

    ```
    rabbitmqctl set_permissions -p / dumbmonit '^$' '^$' '^$'
    ```

4. In DumbMonit, inserisci l'indirizzo di un nodo, per esempio "rabbit.lan", oppure "https://rabbit.lan:15671" quando l'API è servita su TLS. Un solo nodo riporta l'intero cluster: gli allarmi e le partizioni di ogni nodo, e ogni coda.

!!! warning
    Un allarme di memoria o disco blocca ogni publisher nel cluster, e i client non ricevono alcun errore: le applicazioni semplicemente si bloccano su publish. Le code sono elencate le più piene prima, fino a Queues watched one by one (100 per impostazione predefinita); i totali contano sempre ogni coda.

## Credenziali

| Credenziale | Campi |
|---|---|
| Username / password | L'utente `monitoring` creato sopra, inviato come autenticazione HTTP di base. |

Senza permessi su un virtual host, le sue code non vengono elencate una per
una; i totali del cluster le contano comunque.

Indirizzo: un nome host o IP (`rabbit.lan`), `host:porta`, oppure un URL
completo con un prefisso di percorso dietro un reverse proxy. Opzioni:
protocollo (HTTP per impostazione predefinita), porta (15672), verifica del
certificato, timeout della richiesta (10 s), e Queues watched one by one
(100, da 0 a 500).
