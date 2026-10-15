# Grafana Loki {#grafana-loki}

O próprio servidor de logs: se está pronto, que linhas de log recusa e porquê,
se os seus chunks chegam ao armazenamento, e se o seu write-ahead log ainda
tem disco.

O Loki publica `/ready` e `/metrics` na sua porta HTTP; o DumbMonit lê apenas
estes dois e nada mais. Nunca executa uma consulta LogQL e nunca lê uma linha
de log.

A falha a vigiar é silenciosa. Quando o Loki recusa uma linha (mais antiga do
que `reject_old_samples_max_age`, acima do limite de ingestão ou de stream do
inquilino, mais longa do que `max_line_size`), o remetente recebe um 4xx, e o
Promtail, Alloy ou Fluent Bit geralmente descartam o lote. O Grafana não
mostra nada além de uma lacuna, encontrada no dia em que alguém procura o log
de um incidente. A segunda é chunks que não podem ser escritos no
armazenamento: o Loki mantém-os em memória, e perde-os no reinício seguinte.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_loki_`. Os totais são contadores:
os gráficos e as regras convertem-nos em taxas.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `ready` | 1 quando `/ready` responde `ready`, 0 enquanto o Loki arranca ou quando um componente não consegue escrever ou ler | |
| `version_info` | valor 1 | `version` |
| `received_lines_total`, `received_bytes_total` | o que o distribuidor recebeu, todos os inquilinos | |
| `discarded_lines_total` | linhas recusadas, sempre presente, mesmo em zero | |
| `discarded_lines_reason_total` | o mesmo por motivo: `greater_than_max_sample_age`, `rate_limited`, `stream_limit`, `line_too_long`, `per_stream_rate_limit`… | `reason` |
| `flush_failures_total`, `flush_queue_length` | chunks que falharam ao chegar ao armazenamento, chunks à espera de o fazer | |
| `memory_streams` | streams mantidos em memória pelo ingestor | |
| `wal_disk_full_failures_total`, `wal_disk_used_percent` | escritas no write-ahead log perdidas por um disco cheio, e quão cheio está esse disco | |
| `requests_total`, `request_errors_total` | pedidos HTTP e gRPC, e os que responderam com um 5xx ou um erro gRPC. Um 4xx é culpa do cliente e não é contado como erro. | |
| `ring_unhealthy_members` | membros dos anéis de hash marcados como não saudáveis | |
| `panics_total`, `log_errors_total` | panics recuperados, mensagens de erro registadas | |
| `retention_last_run_timestamp_seconds` | última execução de retenção com sucesso do compactor, quando a retenção está ativa | |
| `memory_resident_bytes` | | |

O Loki dividido em componentes (simple scalable, microservices) expõe estas
famílias onde pertencem: linhas recusadas no distribuidor, flush e WAL no
ingestor. Adicione os componentes que quer monitorizar; uma família que um
componente não tem simplesmente não produz série.

As [regras incorporadas](../alerting/rules.md#log-and-metrics-servers) que se
aplicam: Loki not ready, Loki refusing log lines, Loki cannot flush to
storage, Loki write-ahead log disk full, Loki request errors, mais Device
unreachable.

## A página do monitor {#the-device-page}

O painel acima dos gráficos lê o que a verificação guardou; abrir a página
nunca consulta o Loki. Diz o que está mal numa frase (estado de preparação,
linhas recusadas nas últimas 24 horas, chunks que falharam ao fazer flush, um
disco WAL cheio, membros do anel não saudáveis, erros do servidor), depois as
linhas e o volume recebido por segundo, os streams em memória e o disco WAL,
depois as linhas recusadas nas últimas 24 horas por motivo.

## Deixar o DumbMonit ler o estado do próprio Loki {#let-dumbmonit-read-lokis-own-health}

1. Nada para instalar: o Loki publica /ready e /metrics na sua porta HTTP (3100 por predefinição). Verifique a partir do anfitrião do DumbMonit que ambos respondem.

    ```
    curl http://loki.lan:3100/ready
    curl -s http://loki.lan:3100/metrics | grep loki_build_info
    ```

2. O Loki não tem sessão própria. Se estiver atrás de um proxy inverso que pede um nome de utilizador e senha, escolha Username / password abaixo; para um token bearer, escolha Bearer token. Não é necessário nenhum inquilino: /ready e /metrics não são por inquilino.

3. No DumbMonit, introduza o endereço do Loki, por exemplo «loki.lan», ou «https://logs.lan/loki» atrás de um proxy inverso. Com o Loki dividido em componentes (simple scalable ou microservices), adicione pelo menos o caminho de escrita, onde as linhas recusadas e as falhas de flush são contadas.

4. O DumbMonit só lê /ready e /metrics. Nunca executa uma consulta LogQL e nunca lê uma linha de log.

!!! warning
    As linhas recusadas são a falha a vigiar: quando o Loki recusa uma linha (demasiado antiga, acima do limite de ingestão ou de stream, linha demasiado longa), o remetente recebe um erro e geralmente descarta o lote. Nada aparece no Grafana exceto uma lacuna nos logs.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| No authentication | A predefinição: o próprio Loki não tem sessão. |
| Username / password | A autenticação básica do proxy inverso em frente do Loki. |
| Bearer token | Um token que o seu proxy inverso espera em `Authorization: Bearer`. |

Endereço: um nome de anfitrião ou IP (`loki.lan`), `anfitrião:porta`, ou um
URL completo com um prefixo de caminho (`https://logs.lan/loki`). Opções:
protocolo (HTTP por predefinição), porta (3100), verificação de certificado e
tempo limite do pedido (10 s).
