# VictoriaMetrics e VictoriaLogs {#victoriametrics-and-victorialogs}

O próprio banco de dados de séries temporais e o banco de dados de logs: se
aceitam o que lhes é enviado, quanto disco resta antes de parar de aceitar
qualquer coisa, e o que recusam.

Dois tipos de dispositivo compartilham esta página, porque os dois produtos
compartilham seus fundamentos: `victoriametrics` para um VictoriaMetrics de
nó único, e `victorialogs` para um VictoriaLogs de nó único. Ambos publicam
sua própria saúde em `/health` e `/metrics`, na porta de sua API; o
DumbMonit lê apenas esses dois e nada mais. Ele nunca executa uma consulta
contra seus dados e nunca escreve.

Esses servidores raramente caem completamente. Eles caem *pela metade*:
continuam respondendo consultas e os painéis continuam abrindo, enquanto
toda amostra ou linha de log nova é recusada, porque o disco cruzou o
limite somente leitura ou os dados vieram com um timestamp fora da
retenção. Qualquer coisa que envia para eles (vmagent, Prometheus remote
write, Vector, Fluent Bit, Promtail) tenta de novo, depois descarta. Esta
página é sobre ver isso chegando.

## O que ele observa {#what-it-watches}

Métricas têm o prefixo `dumbmonit_victoriametrics_` ou
`dumbmonit_victorialogs_`. Os totais são contadores: gráficos e regras os
transformam em taxas.

| Métrica | O que é | Rótulos |
|---|---|---|
| `healthy` | 1 quando `/health` responde OK, 0 quando responde qualquer outra coisa | |
| `version_info` | valor 1 | `version` |
| `rows_ingested_total` | amostras (VictoriaMetrics) ou linhas de log (VictoriaLogs) aceitas | |
| `bytes_ingested_total` | volume de log aceito (apenas VictoriaLogs) | |
| `rows_rejected_total` | tudo recusado, sempre presente, mesmo em zero | |
| `rows_rejected_reason_total` | o mesmo, por motivo: `big_timestamp`, `small_timestamp`, `too_long_label_value`, `too_many_labels`, `invalid`, `hourly_series_limit`… (VictoriaMetrics); `too_big_timestamp`, `too_small_timestamp`, `too_many_fields`, `too_long_line` (VictoriaLogs) | `reason` |
| `disk_free_bytes`, `disk_total_bytes` | espaço livre e total do volume de armazenamento | `path` |
| `disk_free_limit_bytes` | `-storage.minFreeDiskSpaceBytes`: abaixo dele, o armazenamento se torna somente leitura | `path` |
| `disk_headroom_percent` | espaço livre restante acima desse limite, como parcela do disco | `path` |
| `read_only` | 1 uma vez que o armazenamento mudou para somente leitura | |
| `data_size_bytes` | dados em disco | |
| `slow_inserts_total`, `rows_added_total` | amostras novas que tomaram o caminho lento, e todas as amostras adicionadas (apenas VictoriaMetrics) | |
| `active_series` | séries que receberam uma amostra na última hora (apenas VictoriaMetrics) | |
| `http_errors_total` | requisições respondidas com um erro | |
| `log_errors_total` | mensagens de erro que o servidor registrou | |
| `uptime_seconds`, `memory_resident_bytes` | | |

**Uma amostra recusada é perdida.** O VictoriaMetrics recusa uma amostra
mais antiga que sua retenção ou mais de dois dias no futuro, um valor de
rótulo mais longo que `-maxLabelValueLen`, uma série acima de
`-maxLabelsPerTimeseries`, e toda amostra de uma série nova uma vez que
`-storage.maxHourlySeries` ou `-storage.maxDailySeries` é atingido. O
VictoriaLogs recusa linhas fora de sua retenção e linhas com muitos campos.
O cliente geralmente não recebe erro nenhum por isso: só este contador
sabe.

**Inserções lentas** são amostras cuja série o VictoriaMetrics teve que
buscar em disco em vez de em seu cache. Toda série completamente nova é
uma, então uma implantação causa uma explosão; uma parcela que permanece
acima de 5% significa que o servidor não tem memória para suas séries
ativas. Esse é o limite que a documentação do VictoriaMetrics dá.

As [regras integradas](../alerting/rules.md#log-and-metrics-servers) que se
aplicam: Metrics or log storage read-only, Metrics or log storage almost
read-only, Metrics or log server unhealthy, Metrics or log data refused,
VictoriaMetrics slow inserts, além de Device unreachable.

## A página do dispositivo {#the-device-page}

O painel acima dos gráficos lê o que a sondagem armazenou; abrir a página
nunca consulta o servidor. Ele diz, nesta ordem:

1. **O que está errado, em uma frase**: saúde, armazenamento (*Somente
   leitura*, ou quanto do disco resta acima do limite), dados recusados nas
   últimas 24 horas, inserções lentas, erros registrados. Cada linha
   carrega uma palavra assim como uma cor.
2. **O que o servidor faz**: amostras ou linhas ingeridas por segundo,
   séries ativas ou volume ingerido, dados em disco, tempo de atividade.
3. **O que foi recusado nas últimas 24 horas, por motivo**, e o espaço
   livre de cada caminho de armazenamento contra seu limite somente
   leitura.

## Deixar o DumbMonit ler a própria saúde do VictoriaMetrics {#let-dumbmonit-read-victoriametrics-own-health}

1. Nada para instalar: o VictoriaMetrics publica sua própria saúde em
   /health e /metrics, na porta de sua API (8428 por padrão). Verifique a
   partir do host DumbMonit que ambos respondem.

    ```
    curl http://victoriametrics.lan:8428/health
    curl -s http://victoriametrics.lan:8428/metrics | grep vm_app_version
    ```

2. Se o VictoriaMetrics roda com -httpAuth.username e -httpAuth.password,
   escolha Nome de usuário / senha abaixo e os informe. Detrás do vmauth ou
   um proxy reverso que espera um token bearer, escolha Token bearer. Se
   -metricsAuthKey está definido, /metrics só responde com essa chave na
   URL, que o DumbMonit não envia: proteja /metrics com a autenticação
   básica acima em vez disso.

3. No DumbMonit, informe o endereço do servidor, por exemplo
   "victoriametrics.lan" ou "http://10.0.0.5:8428". Detrás de um proxy
   reverso, informe a URL completa com seu prefixo de caminho. Isso cobre o
   servidor de nó único; os componentes de um cluster VictoriaMetrics
   expõem outras métricas e não são cobertos.

4. O DumbMonit só lê /health e /metrics. Ele nunca executa uma consulta
   contra seus dados e nunca escreve nada.

!!! warning
    Espaço livre em disco é o que observar: abaixo de
    -storage.minFreeDiskSpaceBytes (100 MB por padrão) o VictoriaMetrics
    muda para somente leitura e recusa toda amostra nova, enquanto continua
    respondendo consultas como se nada tivesse acontecido. O DumbMonit
    mostra o espaço livre restante acima desse limite e avisa abaixo de
    10%.

## Deixar o DumbMonit ler a própria saúde do VictoriaLogs {#let-dumbmonit-read-victorialogs-own-health}

1. Nada para instalar: o VictoriaLogs publica sua própria saúde em /health
   e /metrics, na porta de sua API (9428 por padrão). Verifique a partir do
   host DumbMonit que ambos respondem.

    ```
    curl http://victorialogs.lan:9428/health
    curl -s http://victorialogs.lan:9428/metrics | grep vl_rows_ingested_total
    ```

2. Se o VictoriaLogs roda com -httpAuth.username e -httpAuth.password,
   escolha Nome de usuário / senha abaixo e os informe. Detrás do vmauth ou
   um proxy reverso que espera um token bearer, escolha Token bearer. Se
   -metricsAuthKey está definido, /metrics só responde com essa chave na
   URL, que o DumbMonit não envia: proteja /metrics com a autenticação
   básica acima em vez disso.

3. No DumbMonit, informe o endereço do servidor, por exemplo
   "victorialogs.lan" ou "http://10.0.0.6:9428". Detrás de um proxy
   reverso, informe a URL completa com seu prefixo de caminho.

4. O DumbMonit só lê /health e /metrics. Ele nunca executa uma consulta
   LogsQL e nunca lê uma linha de log.

!!! warning
    Espaço livre em disco é o que observar: abaixo de
    -storage.minFreeDiskSpaceBytes (10 MB por padrão) o VictoriaLogs muda
    para somente leitura e recusa toda linha de log nova. O DumbMonit
    mostra o espaço livre restante acima desse limite e avisa abaixo de
    10%.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Sem autenticação | O padrão: `/health` e `/metrics` estão abertos a menos que você os tenha protegido. |
| Nome de usuário / senha | O `-httpAuth.username` e `-httpAuth.password` do servidor, ou do proxy reverso na frente dele. |
| Token bearer | Um token que o vmauth ou seu proxy reverso espera em `Authorization: Bearer`. |

Endereço: um nome de host ou IP (`victoriametrics.lan`, `10.0.0.5`),
`host:porta`, ou uma URL completa com um prefixo de caminho detrás de um
proxy reverso (`https://metrics.lan/vm`). Opções: protocolo (HTTP por
padrão), porta (8428 ou 9428), verificação de certificado e tempo limite da
requisição (10 s).
