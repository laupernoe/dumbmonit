# VictoriaMetrics e VictoriaLogs {#victoriametrics-and-victorialogs}

A própria base de dados de séries temporais e a base de dados de logs: se
aceitam o que lhes é enviado, quanto disco resta antes de deixarem de
aceitar qualquer coisa, e o que recusam.

Dois tipos de dispositivo partilham esta página, porque os dois produtos
partilham as suas fundações: `victoriametrics` para um VictoriaMetrics de
nó único, e `victorialogs` para um VictoriaLogs de nó único. Ambos
publicam a sua própria saúde em `/health` e `/metrics`, na porta da sua
API; o DumbMonit lê apenas esses dois e nada mais. Nunca executa uma
consulta sobre os seus dados e nunca escreve.

Estes servidores raramente caem completamente. Caem *a meio*: continuam a
responder a consultas e os painéis continuam a abrir, enquanto cada nova
amostra ou linha de log é recusada, porque o disco atravessou o limite de
só leitura ou os dados chegaram com uma data-hora fora da retenção. Tudo
o que escreve para eles (vmagent, Prometheus remote write, Vector, Fluent
Bit, Promtail) tenta de novo, depois desiste. Esta página existe para ver
isso a chegar.

## O que monitoriza {#what-it-watches}

As métricas têm o prefixo `dumbmonit_victoriametrics_` ou
`dumbmonit_victorialogs_`. Os totais são contadores: os gráficos e as
regras convertem-nos em taxas.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `healthy` | 1 quando `/health` responde OK, 0 quando responde qualquer outra coisa | |
| `version_info` | valor 1 | `version` |
| `rows_ingested_total` | amostras (VictoriaMetrics) ou linhas de log (VictoriaLogs) aceites | |
| `bytes_ingested_total` | volume de logs aceite (só VictoriaLogs) | |
| `rows_rejected_total` | tudo recusado, sempre presente, mesmo em zero | |
| `rows_rejected_reason_total` | o mesmo, por motivo: `big_timestamp`, `small_timestamp`, `too_long_label_value`, `too_many_labels`, `invalid`, `hourly_series_limit`… (VictoriaMetrics); `too_big_timestamp`, `too_small_timestamp`, `too_many_fields`, `too_long_line` (VictoriaLogs) | `reason` |
| `disk_free_bytes`, `disk_total_bytes` | espaço livre e total do volume de armazenamento | `path` |
| `disk_free_limit_bytes` | `-storage.minFreeDiskSpaceBytes`: abaixo dele, o armazenamento passa a só leitura | `path` |
| `disk_headroom_percent` | espaço livre restante acima desse limite, como fração do disco | `path` |
| `read_only` | 1 uma vez que o armazenamento mudou para só leitura | |
| `data_size_bytes` | dados em disco | |
| `slow_inserts_total`, `rows_added_total` | novas amostras que seguiram o caminho lento, e todas as amostras adicionadas (só VictoriaMetrics) | |
| `active_series` | séries que receberam uma amostra na última hora (só VictoriaMetrics) | |
| `http_errors_total` | pedidos respondidos com um erro | |
| `log_errors_total` | mensagens de erro que o servidor registou | |
| `uptime_seconds`, `memory_resident_bytes` | | |

**Uma amostra recusada é perdida.** O VictoriaMetrics recusa uma amostra
mais antiga do que a sua retenção ou mais de dois dias no futuro, um valor
de etiqueta mais longo do que `-maxLabelValueLen`, uma série acima de
`-maxLabelsPerTimeseries`, e cada amostra de uma série nova uma vez
`-storage.maxHourlySeries` ou `-storage.maxDailySeries` atingido. O
VictoriaLogs recusa linhas fora da sua retenção e linhas com demasiados
campos. O cliente geralmente não recebe nenhum erro por isto: só este
contador sabe.

**Inserções lentas** são amostras cuja série o VictoriaMetrics teve de
procurar em disco em vez de na sua cache. Toda série nova conta como uma,
por isso uma implantação causa uma explosão; uma fração que fica acima de
5% significa que o servidor não tem memória suficiente para as suas
séries ativas. Esse é o limiar que a documentação do VictoriaMetrics dá.

As [regras incorporadas](../alerting/rules.md#log-and-metrics-servers) que
se aplicam: Metrics or log storage read-only, Metrics or log storage
almost read-only, Metrics or log server unhealthy, Metrics or log data
refused, VictoriaMetrics slow inserts, mais Device unreachable.

## A página do dispositivo {#the-device-page}

O painel acima dos gráficos lê o que a verificação guardou; abrir a
página nunca consulta o servidor. Diz, por esta ordem:

1. **O que está mal, numa frase**: saúde, armazenamento (*Read-only*, ou
   quanto do disco resta acima do limite), dados recusados nas últimas 24
   horas, inserções lentas, erros registados. Cada linha transporta uma
   palavra além de uma cor.
2. **O que o servidor faz**: amostras ou linhas ingeridas por segundo,
   séries ativas ou volume ingerido, dados em disco, tempo de atividade.
3. **O que foi recusado nas últimas 24 horas, por motivo**, e o espaço
   livre de cada caminho de armazenamento contra o seu limite de só
   leitura.

## Deixar o DumbMonit ler o estado do próprio VictoriaMetrics {#let-dumbmonit-read-victoriametrics-own-health}

1. Nada para instalar: o VictoriaMetrics publica a sua própria saúde em /health e /metrics, na porta da sua API (8428 por predefinição). Verifique a partir do anfitrião do DumbMonit que ambos respondem.

    ```
    curl http://victoriametrics.lan:8428/health
    curl -s http://victoriametrics.lan:8428/metrics | grep vm_app_version
    ```

2. Se o VictoriaMetrics corre com -httpAuth.username e -httpAuth.password, escolha Username / password abaixo e introduza-os. Atrás do vmauth ou de um proxy inverso que espera um token bearer, escolha Bearer token. Se -metricsAuthKey estiver definido, /metrics só responde com essa chave no URL, que o DumbMonit não envia: proteja /metrics com a autenticação básica acima em vez disso.

3. No DumbMonit, introduza o endereço do servidor, por exemplo «victoriametrics.lan» ou «http://10.0.0.5:8428». Atrás de um proxy inverso, introduza o URL completo com o seu prefixo de caminho. Isto cobre o servidor de nó único; os componentes de um cluster VictoriaMetrics expõem outras métricas e não são cobertos.

4. O DumbMonit só lê /health e /metrics. Nunca executa uma consulta sobre os seus dados e nunca escreve nada.

!!! warning
    O espaço livre em disco é o que vigiar: abaixo de -storage.minFreeDiskSpaceBytes (100 MB por predefinição) o VictoriaMetrics muda para só leitura e recusa toda nova amostra, enquanto continua a responder a consultas como se nada tivesse acontecido. O DumbMonit mostra a margem restante acima desse limite e avisa abaixo de 10%.

## Deixar o DumbMonit ler o estado do próprio VictoriaLogs {#let-dumbmonit-read-victorialogs-own-health}

1. Nada para instalar: o VictoriaLogs publica a sua própria saúde em /health e /metrics, na porta da sua API (9428 por predefinição). Verifique a partir do anfitrião do DumbMonit que ambos respondem.

    ```
    curl http://victorialogs.lan:9428/health
    curl -s http://victorialogs.lan:9428/metrics | grep vl_rows_ingested_total
    ```

2. Se o VictoriaLogs corre com -httpAuth.username e -httpAuth.password, escolha Username / password abaixo e introduza-os. Atrás do vmauth ou de um proxy inverso que espera um token bearer, escolha Bearer token. Se -metricsAuthKey estiver definido, /metrics só responde com essa chave no URL, que o DumbMonit não envia: proteja /metrics com a autenticação básica acima em vez disso.

3. No DumbMonit, introduza o endereço do servidor, por exemplo «victorialogs.lan» ou «http://10.0.0.6:9428». Atrás de um proxy inverso, introduza o URL completo com o seu prefixo de caminho.

4. O DumbMonit só lê /health e /metrics. Nunca executa uma consulta LogsQL e nunca lê uma linha de log.

!!! warning
    O espaço livre em disco é o que vigiar: abaixo de -storage.minFreeDiskSpaceBytes (10 MB por predefinição) o VictoriaLogs muda para só leitura e recusa toda nova linha de log. O DumbMonit mostra a margem restante acima desse limite e avisa abaixo de 10%.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Sem autenticação | A predefinição: `/health` e `/metrics` estão abertos a menos que os tenha protegido. |
| Nome de utilizador / senha | O `-httpAuth.username` e `-httpAuth.password` do servidor, ou do proxy inverso em frente dele. |
| Token bearer | Um token que o vmauth ou o seu proxy inverso espera em `Authorization: Bearer`. |

Endereço: um nome de anfitrião ou IP (`victoriametrics.lan`, `10.0.0.5`),
`anfitrião:porta`, ou um URL completo com um prefixo de caminho atrás de
um proxy inverso (`https://metrics.lan/vm`). Opções: protocolo (HTTP por
predefinição), porta (8428 ou 9428), verificação de certificado e tempo
limite do pedido (10 s).
