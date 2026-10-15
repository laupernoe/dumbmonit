# RabbitMQ {#rabbitmq}

O message broker: alarmes de memória e disco, nós parados e partições de
rede, filas que enchem sem ninguém as lendo, filas que não estão mais em
execução, e o fluxo de mensagens.

O DumbMonit lê a API de gerenciamento com um usuário marcado `monitoring` que
tem permissões vazias: ele vê o estado dos nós, os alarmes e as filas, e não
pode publicar, consumir, ler uma mensagem ou alterar nada. Um nó reporta todo
o cluster.

A falha que vale a pena observar é a que os clientes não reportam. Quando um
nó cruza seu limite alto de memória ou seu limite de disco livre, o RabbitMQ
dispara um alarme e bloqueia todo publicador no cluster: as aplicações
simplesmente ficam presas em `publish`, sem erro nenhum. A segunda é uma fila
que continua enchendo porque o serviço que a consumia parou.

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_rabbitmq_`. Os totais são
contadores: gráficos e regras os transformam em taxas.

| Métrica | O que é | Rótulos |
|---|---|---|
| `version_info` | valor 1 | `version`, `erlang`, `cluster` |
| `connections`, `channels`, `consumers`, `queues` | totais do cluster | |
| `messages_ready`, `messages_unacked` | mensagens esperando em todas as filas, e entregues mas ainda não confirmadas | |
| `messages_published_total`, `messages_delivered_total`, `messages_redelivered_total`, `messages_unroutable_total` | fluxo de mensagens; unroutable são mensagens que nenhuma fila estava vinculada a receber | |
| `alarm` | 1 enquanto o alarme está disparado; memória e disco estão sempre presentes, em 0 sem alarme | `node`, `resource` |
| `alarms` | alarmes disparados no cluster | |
| `nodes`, `nodes_running`, `partitions` | | |
| `node_running`, `node_partitions` | por nó | `node` |
| `node_memory_used_bytes`, `node_memory_limit_bytes`, `node_memory_used_percent` | memória contra o limite alto | `node` |
| `node_disk_free_bytes`, `node_disk_free_limit_bytes` | disco livre contra o limite | `node` |
| `node_fd_used`, `node_fd_used_percent`, `node_uptime_seconds`, `node_being_drained` | | `node` |
| `queue_messages_ready`, `queue_messages_unacked`, `queue_consumers`, `queue_running` | por fila, a mais cheia primeiro, até Filas observadas uma a uma | `vhost`, `queue` |
| `queues_without_consumers`, `queues_not_running` | entre essas filas | |

Alarmes são lidos da verificação de saúde `/api/health/checks/alarms`, que os
vê imediatamente; as estatísticas do nó atrasam alguns segundos.

As [regras integradas](../alerting/rules.md#databases-message-broker-and-security-engine)
que se aplicam: RabbitMQ memory or disk alarm, RabbitMQ node down, RabbitMQ
network partition, RabbitMQ queue without consumer, RabbitMQ queue
unavailable, além de Monitor unreachable.

## A página do monitor {#the-device-page}

O painel acima dos gráficos lê o que a sondagem armazenou; abrir a página
nunca consulta o RabbitMQ. Ele diz o que está errado em uma frase (alarmes,
nós e partições, filas retendo mensagens sem consumidor, filas não em
execução), depois as mensagens prontas e não confirmadas, mensagens
publicadas e entregues por segundo, conexões, consumidores e filas, depois as
dez filas mais cheias com seus consumidores e a memória e disco livre de cada
nó.

## Criar um usuário RabbitMQ marcado monitoring {#create-a-rabbitmq-user-tagged-monitoring}

1. Ative o plugin de gerenciamento se ainda não estiver ativo (as imagens
   Docker de management já o têm). Sua API escuta na porta 15672.

    ```
    rabbitmq-plugins enable rabbitmq_management
    ```

2. Crie um usuário apenas com a marca monitoring: ele pode ler o estado de
   nós, alarmes e filas, e não pode alterar nada.

    ```
    rabbitmqctl add_user dumbmonit 'a-long-password'
    rabbitmqctl set_user_tags dumbmonit monitoring
    ```

3. Dê a ele permissões vazias em cada virtual host cujas filas você quer ver.
   Ele poderá então listar essas filas, e ainda não poderá publicar, consumir
   ou ler uma mensagem. Repita com -p para cada outro virtual host.

    ```
    rabbitmqctl set_permissions -p / dumbmonit '^$' '^$' '^$'
    ```

4. No DumbMonit, informe o endereço de um nó, por exemplo "rabbit.lan", ou
   "https://rabbit.lan:15671" quando a API é servida via TLS. Um nó reporta
   todo o cluster: os alarmes e partições de cada nó, e toda fila.

!!! warning
    Um alarme de memória ou disco bloqueia todo publicador no cluster, e os
    clientes não recebem erro algum: as aplicações simplesmente ficam presas
    em publish. Filas são listadas com a mais cheia primeiro, até Filas
    observadas uma a uma (100 por padrão); os totais sempre cobrem todas as
    filas.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Nome de usuário / senha | O usuário `monitoring` criado acima, enviado como autenticação básica HTTP. |

Sem permissões em um virtual host, suas filas não são listadas uma a uma; os
totais do cluster ainda as contam.

Endereço: um nome de host ou IP (`rabbit.lan`), `host:porta`, ou uma URL
completa com um prefixo de caminho detrás de um proxy reverso. Opções:
protocolo (HTTP por padrão), porta (15672), verificação de certificado, tempo
limite da requisição (10 s), e Filas observadas uma a uma (100, de 0 a 500).
