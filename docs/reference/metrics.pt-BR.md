# Métricas {#metrics}

O DumbMonit armazena toda medição no VictoriaMetrics e a lê de volta com
MetricsQL, através do [proxy de métricas](api.md#metrics) do servidor.
Esta página descreve a nomenclatura, os rótulos e a retenção. Listas de
métricas por tipo estão nas [páginas de dispositivo](../devices/index.md).

## Nomenclatura {#naming}

Toda métrica tem o prefixo `dumbmonit_`, então o DumbMonit pode
compartilhar uma instância VictoriaMetrics com outras ferramentas sem
colisões. Nomes seguem `dumbmonit_<o que>_<unidade>`:

| Padrão | Exemplos |
|---|---|
| `…_bytes`, `…_bytes_total`, `…_bytes_used` | `dumbmonit_memory_bytes_total`, `dumbmonit_filesystem_used_bytes` |
| `…_percent` | `dumbmonit_cpu_usage_percent`, `dumbmonit_pbs_datastore_used_percent` |
| `…_seconds`, `…_age_seconds` | `dumbmonit_probe_duration_seconds`, `dumbmonit_proxmox_backup_last_age_seconds` |
| `…_celsius`, `…_volts`, `…_hertz`, `…_watts` | leituras de UPS e Synology |
| `…_info` | Séries de presença (valor 1) cuja informação está em rótulos: `dumbmonit_system_info`, `dumbmonit_pbs_version_info` |
| `…_status`, `…_up`, `…_running` | Enumerações ou booleanos: `dumbmonit_if_oper_status`, `dumbmonit_proxmox_guest_running` |

Famílias específicas de fonte carregam sua fonte como um segundo
prefixo: `dumbmonit_proxmox_*`, `dumbmonit_pbs_*`, `dumbmonit_synology_*`,
`dumbmonit_probe_*` (serviços). Perfis SNMP e o agente usam nomes simples
(`dumbmonit_if_octets_in`, `dumbmonit_cpu_usage_percent`).

Contadores (octetos e pacotes de interface, bytes de disco, páginas
impressas) são armazenados **em bruto**, como o dispositivo os reporta:
aplique `rate()` em suas próprias consultas para obter uma vazão. A
página do dispositivo exibe em gráfico toda série `dumbmonit_*` do
dispositivo como armazenada, então um contador aparece como uma linha
subindo. Um reinício aparece como um reset de contador, nunca como um
pico falso.

## `dumbmonit_up` {#dumbmonit_up}

Depois de toda sondagem bem sucedida, o servidor escreve
`dumbmonit_up = 1` para o dispositivo. Nada é escrito quando uma sondagem
falha: a série simplesmente para, e a regra "Device unreachable" mede a
idade da última amostra
(`time() - tlast_over_time(dumbmonit_up[7d])`).

Para serviços (`http`, `tcp`, `dns`, `ping`, `tls`), `dumbmonit_up = 1`
significa "o monitor rodou", e `dumbmonit_probe_success` diz se o
serviço está bem. Os dois sinais são mantidos separados de propósito: um
`probe_success` de 0 diz que o serviço está fora do ar, uma interrupção
de `up` diz que o monitoramento está fora do ar.

## `dumbmonit_security_score` {#dumbmonit_security_score}

A cada 15 minutos o servidor calcula a
[pontuação de segurança](../using/security-score.md) de cada dispositivo
cujo tipo tem verificações de segurança, e escreve
`dumbmonit_security_score` (0–100, ausente enquanto nada pode ser
avaliado) e `dumbmonit_security_checks_failed`, ambos rotulados `target`.

## Rótulos {#labels}

| Rótulo | Definido por | Significado |
|---|---|---|
| `target` | servidor | O id do dispositivo, como em `/api/targets/{id}`. Toda regra e todo gráfico selecionam por ele. |
| `host` | servidor | O nome do dispositivo. |
| `tag_<key>` | servidor | Toda tag do dispositivo, incluindo opções de tipo (`tag_insecure_tls`, `tag_port`…). Nunca coloque um segredo em uma tag. |
| `probe` | serviços | `http`, `tcp`, `dns`, `ping` ou `tls`. |
| rótulos por série | coletor | `ifname`, `mountpoint`, `device`, `fstype`, `core`, `service`, `container`, `node`, `vmid`, `name`, `storage`, `datastore`, `group`, `volume`, `disk`, `pool`, `repo`, `changes`, `reason`, `version`, `issuer`… como listado em cada página de dispositivo. |

Rótulos de identidade sempre vencem sobre rótulos de coletor, e para
agentes são definidos na recepção: uma máquina não pode escrever na
série de outra máquina.

!!! note "Prefixo antes de setembro de 2026"
    Séries escritas pelo EzyMonit se chamam `ezymonit_…`. O prefixo
    mudou com o nome do produto e nada lê as séries antigas: elas
    permanecem no VictoriaMetrics até a retenção removê-las, gráficos e
    regras reiniciam a partir da atualização, e regras personalizadas
    ainda nomeando `ezymonit_…` precisam ser editadas.

## Retenção e armazenamento {#retention-and-storage}

O VictoriaMetrics embutido mantém 12 meses de dados
(`DUMBMONIT_VM_RETENTION`) sob `/data/vm`, no volume `dumbmonit-data`. O
servidor agrupa suas escritas e as descarrega a cada 5 segundos
(`DUMBMONIT_FLUSH_INTERVAL_SECS`). Excluir um dispositivo não exclui
suas séries; elas expiram.

O histórico de alertas é mantido por 90 dias e as linhas de base de
séries que desapareceram por 60 dias, ambos em SQLite (veja a
[referência de configuração](configuration.md)).

## Consultando {#querying}

- Através do servidor: `GET /api/metrics/query` e
  `GET /api/metrics/query_range`, com o cookie de sessão. Resultados de
  intervalo são limitados a 2.000 pontos por série.
- De um Prometheus ou Grafana existente: `GET /federate`, `GET /metrics`
  e a fonte de dados `/prometheus`, com um token de API —
  [abaixo](#scraping-dumbmonit).
- Diretamente, em desenvolvimento: a camada
  `docker-compose.dev.yml` publica o VictoriaMetrics embutido em
  `http://localhost:8428`, com sua própria interface em
  `http://localhost:8428/vmui/`. Em produção ele escuta apenas no
  loopback do contêiner; defina `DUMBMONIT_VM_LISTEN=0.0.0.0:8428` e
  publique a porta você mesmo se quiser o Grafana ou outra ferramenta
  nos mesmos dados.

```
# Vazão de interface em bits por segundo
rate(dumbmonit_if_octets_in{target="4"}[5m]) * 8

# Disponibilidade de todo serviço em 30 dias
avg by (host) (avg_over_time(dumbmonit_probe_success[30d]))

# Todo dispositivo que não reportou por 5 minutos
time() - tlast_over_time(dumbmonit_up[7d]) > 300
```

## Coletando do DumbMonit {#scraping-dumbmonit}

Se você já roda um Prometheus ou um Grafana, não precisa de uma segunda
stack. Três rotas ficam fora de `/api`, onde um coletor as procura:

| Rota | O que retorna |
|---|---|
| `GET /metrics` | A saúde da própria instância DumbMonit: agendador, sondagens, escritas, alertas, notificações, agentes, banco de dados. Algumas dezenas de séries, sempre o mesmo número — nunca uma por dispositivo. |
| `GET /federate` | As medições, selecionadas por `match[]`, exatamente como o Prometheus federa outro Prometheus: o ponto mais recente de cada série selecionada. |
| `GET /prometheus/api/v1/…` | A metade de leitura da API Prometheus, retransmitida para o armazenamento: o que uma fonte de dados Grafana chama. |

Todas as três levam `Authorization: Bearer dmt_…` com o escopo
**`read`** — um token criado em Configurações → *API e assistentes*, o
mesmo tipo que o [servidor MCP](../using/assistant.md) usa. Nenhum
cookie é envolvido, então nenhum cabeçalho `X-Requested-With` é
necessário; um cookie de sessão também é aceito, então você pode abrir
qualquer um deles de um navegador conectado. Um token é limitado a 120
chamadas por minuto: suficiente para uma coleta a cada 15 segundos, e
suficiente para um painel Grafana de algumas dezenas de painéis — dê a
um painel ocupado seu próprio token em vez de compartilhar um com um
coletor.

### Prometheus {#prometheus}

```yaml
# prometheus.yml — a própria saúde do DumbMonit, depois as medições que coleta.
scrape_configs:
  - job_name: dumbmonit
    metrics_path: /metrics
    scheme: http
    authorization:
      type: Bearer
      credentials: dmt_0123456789abcdef0123456789abcdef
    static_configs:
      - targets: ['dumbmonit.lan:8080']

  - job_name: dumbmonit-federate
    metrics_path: /federate
    scheme: http
    honor_labels: true
    scrape_interval: 60s
    authorization:
      type: Bearer
      credentials: dmt_0123456789abcdef0123456789abcdef
    params:
      'match[]':
        - '{__name__=~"dumbmonit_.*"}'
    static_configs:
      - targets: ['dumbmonit.lan:8080']
```

`honor_labels: true` mantém os rótulos `target` e `host` que o DumbMonit
define em vez de sobrescrevê-los com os do próprio job de coleta.
Coloque o token em um arquivo e use
`authorization: {type: Bearer, credentials_file: /etc/prometheus/dumbmonit.token}`
se você preferir não tê-lo na configuração.

Sem `match[]`, `/federate` seleciona `{__name__=~"dumbmonit_.*"}` — toda
série que o DumbMonit escreve, e nada pertencente a outra ferramenta que
compartilha o mesmo VictoriaMetrics. Estreite-o para manter uma
federação pequena: `{__name__=~"dumbmonit_.*",target="4"}` para um
dispositivo, `{__name__="dumbmonit_up"}` apenas para disponibilidade. No
máximo 10 seletores por requisição, e uma resposta acima de 8 MiB é
recusada com uma mensagem dizendo para estreitá-la — divida uma frota
grande em vários jobs com seletores mais estreitos em vez de um que pede
tudo.
`max_lookback=15m` amplia a janela em que o último ponto é procurado,
para dispositivos sondados com menos frequência que a cada cinco
minutos.

```bash
curl -H 'Authorization: Bearer dmt_…' http://localhost:8080/metrics
curl -H 'Authorization: Bearer dmt_…' -G http://localhost:8080/federate \
  --data-urlencode 'match[]={__name__=~"dumbmonit_.*",target="4"}'
```

### Grafana {#grafana}

Aponte o Grafana para o **DumbMonit**, não para o VictoriaMetrics que
ele executa: o processo filho escuta no loopback do contêiner, não tem
autenticação própria, e expõe as rotas que *excluem* séries tão
facilmente quanto as que as leem. O DumbMonit retransmite a metade de
leitura da API Prometheus sob `/prometheus`, detrás do mesmo token,
então o Grafana fala com uma fonte de dados Prometheus normal:

1. **Connections → Data sources → Add → Prometheus**.
2. **URL**: `http://dumbmonit.lan:8080/prometheus` (o Grafana anexa
   `/api/v1/query` ele mesmo).
3. **HTTP headers**: adicione um, `Authorization` = `Bearer dmt_…`. O
   Grafana o armazena criptografado e nunca o mostra de novo.
4. **Save & test** — o Grafana reporta que a fonte de dados está
   funcionando, e toda série `dumbmonit_*` então fica disponível em
   PromQL, com autocompletar em nomes de métrica e rótulo.

Só as rotas de leitura são retransmitidas — `query`, `query_range`,
`series`, `labels`, valores de rótulo, `metadata` e a versão que o
Grafana pede ao testar a conexão. Qualquer outra coisa,
`admin/tsdb/delete_series` em primeiro lugar, responde `404`: este ponto
de entrada não pode alterar ou apagar nada.

Se você já roda um Prometheus, o outro caminho funciona igualmente bem:
o Prometheus federa o DumbMonit com o job acima, o Grafana lê o
Prometheus, e as consultas são idênticas.

### Métricas da instância {#instance-metrics}

Tudo que `GET /metrics` expõe. Valores são lidos quando você coleta; os
contadores `_total` são monotônicos e reiniciam só quando o servidor
reinicia (`dumbmonit_uptime_seconds` diz quando isso aconteceu).

| Métrica | Tipo | Significado |
|---|---|---|
| `dumbmonit_build_info{version}` | gauge | Sempre 1. O rótulo `version` é a versão do servidor em execução, a mesma que `/api/health` reporta. |
| `dumbmonit_uptime_seconds` | gauge | Segundos desde que o processo do servidor iniciou. |
| `dumbmonit_scheduler_cycles_total` | contador | Ciclos do agendador executados. O loop pulsa uma vez por segundo, então isso também serve como sinal de vida. |
| `dumbmonit_scheduler_cycle_seconds` | gauge | Duração do último ciclo. Normalmente abaixo de um milissegundo: o ciclo só despacha, não espera por sondagens. |
| `dumbmonit_scheduler_backlog` | gauge | Dispositivos ainda devendo uma sondagem quando o último ciclo terminou. Zero em uma instância saudável; um valor não-zero persistente significa que `DUMBMONIT_MAX_CONCURRENT_PROBES` é baixo demais para o número de dispositivos e seus intervalos. |
| `dumbmonit_probes_total{kind}` | contador | Sondagens executadas, por tipo de dispositivo (`snmp`, `proxmox`, `http`…). Inclui as que um agente relay executou em nome do servidor. |
| `dumbmonit_probes_failed_total{kind}` | contador | Sondagens que falharam, por tipo — um dispositivo inacessível ou um erro de configuração. `rate(dumbmonit_probes_failed_total[15m]) / rate(dumbmonit_probes_total[15m])` é a taxa de falha. |
| `dumbmonit_samples_written_total` | contador | Amostras aceitas pelo VictoriaMetrics. |
| `dumbmonit_sample_writes_failed_total` | contador | Lotes que o VictoriaMetrics recusou ou não respondeu. O lote permanece em buffer e é tentado de novo; um valor subindo com um `dumbmonit_samples_pending` subindo significa que o armazenamento está fora do ar ou cheio. |
| `dumbmonit_samples_pending` | gauge | Amostras ainda no buffer de escrita depois do último flush. Normalmente 0. |
| `dumbmonit_samples_dropped_total` | contador | Amostras descartadas antes mesmo de alcançar o buffer de escrita, porque estava cheio ou fechado. Nunca tentadas de novo — um valor subindo durante uma indisponibilidade significa perda de dados, não apenas atraso. |
| `dumbmonit_alerting_cycles_total` | contador | Ciclos de alerta executados (um a cada `DUMBMONIT_ALERT_INTERVAL_SECS`, 30 s por padrão). |
| `dumbmonit_alerting_cycle_seconds` | gauge | Duração do último ciclo de alerta, tempo de consulta incluído. Aproximar-se do intervalo significa que as regras estão consultando mais do que o armazenamento pode servir. |
| `dumbmonit_alerting_rules_evaluated` | gauge | Regras avaliadas no último ciclo. |
| `dumbmonit_alerting_rules_failed` | gauge | Regras cuja consulta falhou no último ciclo. Seus alertas ficam congelados, não resolvidos. Qualquer coisa além de 0 merece uma olhada nos logs. |
| `dumbmonit_alerts{phase}` | gauge | Alertas que o motor rastreia, por fase: `ok`, `pending` (condição verdadeira, `for` ainda não decorreu), `firing`, `resolved`. |
| `dumbmonit_alerts_suppressed` | gauge | Alertas ocultos porque um dispositivo pai está fora do ar (a fase efetiva `suppressed`). |
| `dumbmonit_alerts_silenced` | gauge | Alertas cobertos por uma janela de manutenção. |
| `dumbmonit_alerts_learning` | gauge | Alertas de linha de base ainda aprendendo, e portanto silenciosos. |
| `dumbmonit_notifications_total{kind}` | contador | Notificações enviadas, por tipo de canal (`email`, `discord`, `ntfy`…). |
| `dumbmonit_notifications_failed_total{kind}` | contador | Notificações que o canal recusou, por tipo. São reenfileiradas e tentadas de novo; um valor subindo constantemente é um webhook quebrado ou uma conta SMTP. |
| `dumbmonit_agents` | gauge | Máquinas que registraram um agente. |
| `dumbmonit_agents_stale` | gauge | Agentes que não enviaram nada por cinco minutos. |
| `dumbmonit_database_up` | gauge | 1 quando o SQLite responde — a mesma verificação que `/api/health`. |
| `dumbmonit_database_bytes` | gauge | Tamanho de `dumbmonit.db` e seu log de pré-escrita. Não inclui `/data/vm`. |
| `dumbmonit_victoriametrics_up` | gauge | 1 quando o VictoriaMetrics responde — a mesma verificação que `/api/health`. 0 enquanto o filho embutido está reiniciando. |
| `dumbmonit_victoriametrics_embedded` | gauge | 1 quando este servidor executa o VictoriaMetrics ele mesmo, 0 quando `DUMBMONIT_VM_URL` aponta para um externo. |

Nenhuma métrica nesta página carrega um rótulo por dispositivo: o
documento tem o mesmo tamanho para um dispositivo e para mil, e nenhum
nome de dispositivo é exposto a quem pode coletá-lo.

Dois alertas que vale a pena adicionar do seu lado, uma vez que o
DumbMonit é coletado:

```
# O monitoramento parou de monitorar
up{job="dumbmonit"} == 0

# Medições estão se acumulando em vez de serem armazenadas
dumbmonit_samples_pending > 0 and increase(dumbmonit_sample_writes_failed_total[10m]) > 0
```

### Abrindo as rotas sem um token {#opening-the-routes-without-a-token}

`DUMBMONIT_METRICS_PUBLIC=true` serve `/metrics`, `/federate` e
`/prometheus/api/v1/…` a qualquer um que possa alcançar a porta, para
pessoas que filtram a porta elas mesmas — **qualquer um nessa rede pode
então ler toda medição de todo dispositivo e o estado da instância,
então só defina isso quando algo mais já restringe quem alcança a
porta.**
