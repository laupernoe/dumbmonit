# Grafana Loki

O próprio servidor de logs: se está pronto, quais linhas de log recusa e por
quê, se seus chunks chegam ao armazenamento, e se seu log de pré-escrita ainda
tem disco.

O Loki publica `/ready` e `/metrics` na sua porta HTTP; o DumbMonit lê apenas
esses dois e nada mais. Ele nunca executa uma consulta LogQL e nunca lê uma
linha de log.

A falha que vale a pena observar é silenciosa. Quando o Loki recusa uma linha
(mais antiga que `reject_old_samples_max_age`, acima do limite de ingestão ou
de stream do locatário, maior que `max_line_size`), o remetente recebe um
4xx, e o Promtail, Alloy ou Fluent Bit geralmente descartam o lote. O Grafana
não mostra nada além de uma lacuna, descoberta no dia em que alguém procura o
log de um incidente. A segunda é chunks que não podem ser gravados no
armazenamento: o Loki os mantém em memória, e os perde no próximo reinício.

## O que ele observa

Todas as métricas têm o prefixo `dumbmonit_loki_`. Os totais são contadores:
gráficos e regras os transformam em taxas.

| Métrica | O que é | Rótulos |
|---|---|---|
| `ready` | 1 quando `/ready` responde `ready`, 0 enquanto o Loki inicia ou quando um componente não consegue escrever ou ler | |
| `version_info` | valor 1 | `version` |
| `received_lines_total`, `received_bytes_total` | o que o distribuidor recebeu, todos os locatários | |
| `discarded_lines_total` | linhas recusadas, sempre presente, mesmo em zero | |
| `discarded_lines_reason_total` | o mesmo por motivo: `greater_than_max_sample_age`, `rate_limited`, `stream_limit`, `line_too_long`, `per_stream_rate_limit`… | `reason` |
| `flush_failures_total`, `flush_queue_length` | chunks que falharam ao chegar ao armazenamento, chunks esperando para isso | |
| `memory_streams` | streams mantidos em memória pelo ingestor | |
| `wal_disk_full_failures_total`, `wal_disk_used_percent` | escritas do log de pré-escrita perdidas por um disco cheio, e quão cheio esse disco está | |
| `requests_total`, `request_errors_total` | requisições HTTP e gRPC, e as respondidas com um 5xx ou um erro gRPC. Um 4xx é culpa do cliente e não é contado como erro. | |
| `ring_unhealthy_members` | membros dos anéis de hash marcados como não saudáveis | |
| `panics_total`, `log_errors_total` | panics recuperados, mensagens de erro registradas | |
| `retention_last_run_timestamp_seconds` | última execução bem-sucedida de retenção do compactor, quando a retenção está ativa | |
| `memory_resident_bytes` | | |

O Loki dividido em componentes (simple scalable, microservices) expõe essas
famílias onde pertencem: linhas recusadas no distribuidor, flush e WAL no
ingestor. Adicione os componentes que você quer observar; uma família que um
componente não tem simplesmente não produz série.

As [regras integradas](../alerting/rules.md#log-and-metrics-servers) que se
aplicam: Loki not ready, Loki refusing log lines, Loki cannot flush to
storage, Loki write-ahead log disk full, Loki request errors, além de Device
unreachable.

## A página do dispositivo

O painel acima dos gráficos lê o que a sondagem armazenou; abrir a página
nunca consulta o Loki. Ele diz o que está errado em uma frase (prontidão,
linhas recusadas nas últimas 24 horas, chunks que falharam ao fazer flush, um
disco WAL cheio, membros do anel não saudáveis, erros de servidor), depois as
linhas e o volume recebido por segundo, os streams em memória e o disco WAL,
depois as linhas recusadas nas últimas 24 horas por motivo.

## Deixar o DumbMonit ler a própria saúde do Loki

1. Nada para instalar: o Loki publica /ready e /metrics na sua porta HTTP
   (3100 por padrão). Verifique a partir do host DumbMonit que ambos
   respondem.

    ```
    curl http://loki.lan:3100/ready
    curl -s http://loki.lan:3100/metrics | grep loki_build_info
    ```

2. O Loki não tem login próprio. Se ele estiver detrás de um proxy reverso
   que pede um nome de usuário e senha, escolha Nome de usuário / senha
   abaixo; para um token bearer, escolha Token bearer. Nenhum locatário é
   necessário: /ready e /metrics não são por locatário.

3. No DumbMonit, informe o endereço do Loki, por exemplo "loki.lan", ou
   "https://logs.lan/loki" detrás de um proxy reverso. Com o Loki dividido em
   componentes (simple scalable ou microservices), adicione ao menos o
   caminho de escrita, onde linhas recusadas e falhas de flush são contadas.

4. O DumbMonit só lê /ready e /metrics. Ele nunca executa uma consulta LogQL
   e nunca lê uma linha de log.

!!! warning
    Linhas recusadas são a falha a observar: quando o Loki recusa uma linha
    (muito antiga, acima do limite de ingestão ou de stream, linha muito
    longa), o remetente recebe um erro e geralmente descarta o lote. Nada
    aparece no Grafana exceto uma lacuna nos logs.

## Credenciais

| Credencial | Campos |
|---|---|
| Sem autenticação | O padrão: o próprio Loki não tem login. |
| Nome de usuário / senha | A autenticação básica do proxy reverso na frente do Loki. |
| Token bearer | Um token que seu proxy reverso espera em `Authorization: Bearer`. |

Endereço: um nome de host ou IP (`loki.lan`), `host:porta`, ou uma URL
completa com um prefixo de caminho (`https://logs.lan/loki`). Opções:
protocolo (HTTP por padrão), porta (3100), verificação de certificado e tempo
limite da requisição (10 s).
