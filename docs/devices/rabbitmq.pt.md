# RabbitMQ {#rabbitmq}

O broker de mensagens: alarmes de memória e disco, nós parados e partições de
rede, filas que se enchem sem ninguém a lê-las, filas que deixaram de correr,
e o fluxo de mensagens.

O DumbMonit lê a API de gestão com um utilizador com a etiqueta `monitoring`
que tem permissões vazias: vê o estado dos nós, os alarmes e as filas, e não
pode publicar, consumir, ler uma mensagem nem alterar nada. Um único nó
reporta todo o cluster.

A falha a vigiar é a que os clientes não reportam. Quando um nó atravessa o
seu limite máximo de memória ou o seu limite de disco livre, o RabbitMQ
levanta um alarme e bloqueia todos os publicadores do cluster: as aplicações
simplesmente ficam suspensas em `publish`, sem erro. A segunda é uma fila que
continua a enchê-la porque o serviço que a consumia parou.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_rabbitmq_`. Os totais são
contadores: os gráficos e as regras convertem-nos em taxas.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `version_info` | valor 1 | `version`, `erlang`, `cluster` |
| `connections`, `channels`, `consumers`, `queues` | totais do cluster | |
| `messages_ready`, `messages_unacked` | mensagens à espera em todas as filas, e entregues mas ainda não confirmadas | |
| `messages_published_total`, `messages_delivered_total`, `messages_redelivered_total`, `messages_unroutable_total` | fluxo de mensagens; unroutable são mensagens para as quais nenhuma fila estava associada a receber | |
| `alarm` | 1 enquanto o alarme está levantado; memória e disco estão sempre presentes, em 0 sem alarme | `node`, `resource` |
| `alarms` | alarmes levantados no cluster | |
| `nodes`, `nodes_running`, `partitions` | | |
| `node_running`, `node_partitions` | por nó | `node` |
| `node_memory_used_bytes`, `node_memory_limit_bytes`, `node_memory_used_percent` | memória contra o limite máximo | `node` |
| `node_disk_free_bytes`, `node_disk_free_limit_bytes` | disco livre contra o limite | `node` |
| `node_fd_used`, `node_fd_used_percent`, `node_uptime_seconds`, `node_being_drained` | | `node` |
| `queue_messages_ready`, `queue_messages_unacked`, `queue_consumers`, `queue_running` | por fila, as mais cheias primeiro, até Queues watched one by one | `vhost`, `queue` |
| `queues_without_consumers`, `queues_not_running` | entre essas filas | |

Os alarmes são lidos a partir da verificação de saúde
`/api/health/checks/alarms`, que os vê de imediato; as estatísticas dos nós
atrasam-se alguns segundos.

As [regras incorporadas](../alerting/rules.md#databases-message-broker-and-security-engine)
que se aplicam: RabbitMQ memory or disk alarm, RabbitMQ node down, RabbitMQ
network partition, RabbitMQ queue without consumer, RabbitMQ queue
unavailable, mais Monitor unreachable.

## A página do monitor {#the-device-page}

O painel acima dos gráficos lê o que a verificação guardou; abrir a página
nunca consulta o RabbitMQ. Diz o que está mal numa frase (alarmes, nós e
partições, filas com mensagens sem consumidor, filas que não estão a correr),
depois as mensagens prontas e não confirmadas, mensagens publicadas e
entregues por segundo, ligações, consumidores e filas, depois as dez filas
mais cheias com os seus consumidores e a memória e disco livre de cada nó.

## Criar um utilizador RabbitMQ com a etiqueta monitoring {#create-a-rabbitmq-user-tagged-monitoring}

1. Ative o plugin de gestão se ainda não estiver (as imagens Docker de gestão já o têm). A sua API escuta na porta 15672.

    ```
    rabbitmq-plugins enable rabbitmq_management
    ```

2. Crie um utilizador apenas com a etiqueta monitoring: pode ler o estado dos nós, alarmes e filas, e não pode alterar nada.

    ```
    rabbitmqctl add_user dumbmonit 'a-long-password'
    rabbitmqctl set_user_tags dumbmonit monitoring
    ```

3. Dê-lhe permissões vazias em cada virtual host cujas filas quer ver. Pode então listar essas filas, e continua a não poder publicar, consumir ou ler uma mensagem. Repita com -p para cada outro virtual host.

    ```
    rabbitmqctl set_permissions -p / dumbmonit '^$' '^$' '^$'
    ```

4. No DumbMonit, introduza o endereço de um nó, por exemplo «rabbit.lan», ou «https://rabbit.lan:15671» quando a API é servida via TLS. Um único nó reporta todo o cluster: os alarmes e partições de todos os nós, e todas as filas.

!!! warning
    Um alarme de memória ou disco bloqueia todos os publicadores do cluster, e os clientes não recebem nenhum erro: as aplicações simplesmente ficam suspensas ao publicar. As filas são listadas das mais cheias primeiro, até Queues watched one by one (100 por predefinição); os totais cobrem sempre todas as filas.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Nome de utilizador / senha | O utilizador `monitoring` criado acima, enviado como autenticação HTTP básica. |

Sem permissões num virtual host, as suas filas não são listadas uma a uma; os
totais do cluster continuam a contá-las.

Endereço: um nome de anfitrião ou IP (`rabbit.lan`), `anfitrião:porta`, ou um
URL completo com um prefixo de caminho atrás de um proxy inverso. Opções:
protocolo (HTTP por predefinição), porta (15672), verificação de certificado,
tempo limite do pedido (10 s), e Queues watched one by one (100, de 0 a 500).
