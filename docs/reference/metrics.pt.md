# Métricas {#metrics}

O DumbMonit guarda toda medição na VictoriaMetrics e lê-a de volta com
MetricsQL, através do [proxy de métricas](api.md#metrics) do servidor.
Esta página descreve a nomenclatura, as etiquetas e a retenção. As
listas de métricas por tipo estão nas [páginas de dispositivo](../devices/index.md).

## Nomenclatura {#naming}

Toda métrica tem o prefixo `dumbmonit_`, para que o DumbMonit possa
partilhar uma instância VictoriaMetrics com outras ferramentas sem
colisões. Os nomes seguem `dumbmonit_<o-quê>_<unidade>`:

| Padrão | Exemplos |
|---|---|
| `…_bytes`, `…_bytes_total`, `…_bytes_used` | `dumbmonit_memory_bytes_total`, `dumbmonit_filesystem_used_bytes` |
| `…_percent` | `dumbmonit_cpu_usage_percent`, `dumbmonit_pbs_datastore_used_percent` |
| `…_seconds`, `…_age_seconds` | `dumbmonit_probe_duration_seconds`, `dumbmonit_proxmox_backup_last_age_seconds` |
| `…_celsius`, `…_volts`, `…_hertz`, `…_watts` | leituras de UPS e Synology |
| `…_info` | Séries de presença (valor 1) cuja informação está em etiquetas: `dumbmonit_system_info`, `dumbmonit_pbs_version_info` |
| `…_status`, `…_up`, `…_running` | Enumerações ou booleanos: `dumbmonit_if_oper_status`, `dumbmonit_proxmox_guest_running` |

As famílias específicas de fonte transportam a sua fonte como um
segundo prefixo: `dumbmonit_proxmox_*`, `dumbmonit_pbs_*`,
`dumbmonit_synology_*`, `dumbmonit_probe_*` (serviços). Os perfis SNMP
e o agente usam nomes simples (`dumbmonit_if_octets_in`,
`dumbmonit_cpu_usage_percent`).

Os contadores (octetos e pacotes de interface, bytes de disco,
páginas impressas) são guardados **em bruto**, como o dispositivo os
reporta: aplique `rate()` nas suas próprias consultas para obter um
caudal. A página do dispositivo representa em gráfico toda série
`dumbmonit_*` do dispositivo como guardada, por isso um contador
aparece como uma linha ascendente. Um reinício mostra-se como uma
reposição do contador, nunca como um pico falso.

## `dumbmonit_up` {#dumbmonit_up}

Depois de cada verificação bem-sucedida, o servidor escreve
`dumbmonit_up = 1` para o dispositivo. Nada é escrito quando uma
verificação falha: a série simplesmente para, e a regra «Device
unreachable» mede a idade da última amostra
(`time() - tlast_over_time(dumbmonit_up[7d])`).

Para serviços (`http`, `tcp`, `dns`, `ping`, `tls`),
`dumbmonit_up = 1` significa «o monitor correu», e
`dumbmonit_probe_success` diz se o serviço está bem. Os dois sinais
são mantidos separados de propósito: um `probe_success` de 0 diz que
o serviço está em baixo, uma interrupção de `up` diz que a
monitorização está em baixo.

## `dumbmonit_security_score` {#dumbmonit_security_score}

A cada 15 minutos o servidor calcula a
[pontuação de segurança](../using/security-score.md) de cada
dispositivo cujo tipo tem verificações de segurança, e escreve
`dumbmonit_security_score` (0–100, ausente enquanto nada pode ser
avaliado) e `dumbmonit_security_checks_failed`, ambas etiquetadas
`target`.

## Etiquetas {#labels}

| Etiqueta | Definida por | Significado |
|---|---|---|
| `target` | servidor | O id do dispositivo, como em `/api/targets/{id}`. Toda regra e todo gráfico selecionam por ela. |
| `host` | servidor | O nome do dispositivo. |
| `tag_<key>` | servidor | Toda etiqueta do dispositivo, incluindo opções de tipo (`tag_insecure_tls`, `tag_port`…). Nunca coloque um segredo numa etiqueta. |
| `probe` | serviços | `http`, `tcp`, `dns`, `ping` ou `tls`. |
| etiquetas por série | coletor | `ifname`, `mountpoint`, `device`, `fstype`, `core`, `service`, `container`, `node`, `vmid`, `name`, `storage`, `datastore`, `group`, `volume`, `disk`, `pool`, `repo`, `changes`, `reason`, `version`, `issuer`… como listado em cada página de dispositivo. |

As etiquetas de identidade ganham sempre sobre as etiquetas do
coletor, e para agentes são definidas na receção: uma máquina não
pode escrever na série de outra máquina.

!!! note "Prefixo antes de setembro de 2026"
    As séries escritas pelo EzyMonit chamam-se `ezymonit_…`. O
    prefixo mudou com o nome do produto e nada lê as séries antigas:
    permanecem na VictoriaMetrics até a retenção as remover, os
    gráficos e regras recomeçam a partir da atualização, e as regras
    personalizadas que ainda nomeiam `ezymonit_…` têm de ser editadas.

## Retenção e armazenamento {#retention-and-storage}

A VictoriaMetrics incorporada mantém 12 meses de dados
(`DUMBMONIT_VM_RETENTION`) sob `/data/vm`, no volume
`dumbmonit-data`. O servidor agrupa as suas escritas em lote e
descarrega-as a cada 5 segundos
(`DUMBMONIT_FLUSH_INTERVAL_SECS`). Eliminar um dispositivo não
elimina as suas séries; envelhecem com o tempo.

O histórico de alertas é mantido 90 dias e as linhas de base de
séries que desapareceram 60 dias, ambos em SQLite (ver a
[referência de configuração](configuration.md)).

## Consultar {#querying}

- Através do servidor: `GET /api/metrics/query` e
  `GET /api/metrics/query_range`, com o cookie de sessão. Os
  resultados de intervalo são limitados a 2000 pontos por série.
- A partir de um Prometheus ou Grafana existente: `GET /federate`,
  `GET /metrics` e a fonte de dados `/prometheus`, com um token de
  API — [abaixo](#scraping-dumbmonit).
- Diretamente, em desenvolvimento: a camada
  `docker-compose.dev.yml` publica a VictoriaMetrics incorporada em
  `http://localhost:8428`, com a sua própria interface em
  `http://localhost:8428/vmui/`. Em produção escuta só no loopback do
  contentor; defina `DUMBMONIT_VM_LISTEN=0.0.0.0:8428` e publique a
  porta você mesmo se quiser o Grafana ou outra ferramenta nos mesmos
  dados.

```
# Caudal da interface em bits por segundo
rate(dumbmonit_if_octets_in{target="4"}[5m]) * 8

# Disponibilidade de cada serviço ao longo de 30 dias
avg by (host) (avg_over_time(dumbmonit_probe_success[30d]))

# Todo dispositivo que não reportou durante 5 minutos
time() - tlast_over_time(dumbmonit_up[7d]) > 300
```

## Consultar o DumbMonit {#scraping-dumbmonit}

Se já executa um Prometheus ou um Grafana, não precisa de uma segunda
stack. Três rotas ficam fora de `/api`, onde um consultor as procura:

| Rota | O que devolve |
|---|---|
| `GET /metrics` | A saúde da própria instância DumbMonit: escalonador, verificações, escritas, alertagem, notificações, agentes, base de dados. Algumas dezenas de séries, sempre o mesmo número — nunca uma por dispositivo. |
| `GET /federate` | As medições, selecionadas por `match[]`, exatamente como o Prometheus federa outro Prometheus: o ponto mais recente de cada série selecionada. |
| `GET /prometheus/api/v1/…` | A metade de leitura da API Prometheus, reencaminhada para o armazenamento: o que uma fonte de dados Grafana chama. |

Todas as três aceitam `Authorization: Bearer dmt_…` com o âmbito
**`read`** — um token criado em Settings → *API & assistants*, o
mesmo tipo que o [servidor MCP](../using/assistant.md) usa. Nenhum
cookie está envolvido, por isso nenhum cabeçalho
`X-Requested-With` é necessário; um cookie de sessão também é aceite,
por isso pode abrir qualquer uma a partir de um navegador com sessão
iniciada. Um token é limitado a 120 chamadas por minuto: suficiente
para uma consulta a cada 15 segundos, e suficiente para um painel
Grafana de algumas dezenas de painéis — dê a um painel ocupado o seu
próprio token em vez de partilhar um com um consultor.

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

`honor_labels: true` mantém as etiquetas `target` e `host` que o
DumbMonit define em vez de as sobrescrever com as próprias da tarefa
de consulta. Coloque o token num ficheiro e use
`authorization: {type: Bearer, credentials_file: /etc/prometheus/dumbmonit.token}`
se preferir não o ter na configuração.

Sem `match[]`, `/federate` seleciona `{__name__=~"dumbmonit_.*"}` —
toda série que o DumbMonit escreve, e nada pertencente a outra
ferramenta que partilha a mesma VictoriaMetrics. Restrinja-a para
manter uma federação pequena:
`{__name__=~"dumbmonit_.*",target="4"}` para um dispositivo,
`{__name__="dumbmonit_up"}` só para disponibilidade. No máximo 10
seletores por pedido, e uma resposta acima de 8 MiB é recusada com
uma mensagem a dizer para a restringir — divida uma frota grande em
várias tarefas com seletores mais restritos em vez de uma que pede
tudo. `max_lookback=15m` alarga a janela em que o último ponto é
procurado, para dispositivos sondados com menos frequência do que a
cada cinco minutos.

```bash
curl -H 'Authorization: Bearer dmt_…' http://localhost:8080/metrics
curl -H 'Authorization: Bearer dmt_…' -G http://localhost:8080/federate \
  --data-urlencode 'match[]={__name__=~"dumbmonit_.*",target="4"}'
```

### Grafana {#grafana}

Aponte o Grafana para o **DumbMonit**, não para a VictoriaMetrics que
executa: o processo filho escuta no loopback do contentor, não tem
autenticação própria, e expõe as rotas que *eliminam* séries tão
prontamente como as que as lêem. O DumbMonit reencaminha a metade de
leitura da API Prometheus sob `/prometheus`, atrás do mesmo token,
por isso o Grafana fala com uma fonte de dados Prometheus normal:

1. **Connections → Data sources → Add → Prometheus**.
2. **URL**: `http://dumbmonit.lan:8080/prometheus` (o Grafana
   acrescenta `/api/v1/query` ele próprio).
3. **HTTP headers**: adicione um, `Authorization` = `Bearer dmt_…`.
   O Grafana guarda-o encriptado e nunca o mostra de novo.
4. **Save & test** — o Grafana reporta que a fonte de dados funciona,
   e toda série `dumbmonit_*` fica então disponível em PromQL, com
   autocompletação em nomes de métrica e etiqueta.

Só as rotas de leitura são reencaminhadas — `query`, `query_range`,
`series`, `labels`, valores de etiqueta, `metadata` e a versão que o
Grafana pede ao testar a ligação. Qualquer outra coisa,
`admin/tsdb/delete_series` em primeiro lugar, responde `404`: este
ponto de entrada não pode alterar nem apagar nada.

Se já executa um Prometheus, o outro caminho funciona igualmente
bem: o Prometheus federa o DumbMonit com a tarefa acima, o Grafana lê
o Prometheus, e as consultas são idênticas.

### Métricas da instância {#instance-metrics}

Tudo o que `GET /metrics` expõe. Os valores são lidos quando consulta;
os contadores `_total` são monótonos e só reiniciam quando o servidor
reinicia (`dumbmonit_uptime_seconds` diz-lhe quando isso aconteceu).

| Métrica | Tipo | Significado |
|---|---|---|
| `dumbmonit_build_info{version}` | gauge | Sempre 1. A etiqueta `version` é a versão do servidor em execução, a mesma que `/api/health` reporta. |
| `dumbmonit_uptime_seconds` | gauge | Segundos desde que o processo do servidor arrancou. |
| `dumbmonit_scheduler_cycles_total` | counter | Ciclos de escalonador executados. O loop pulsa uma vez por segundo, por isso isto serve também como sinal de vivacidade. |
| `dumbmonit_scheduler_cycle_seconds` | gauge | Duração do último ciclo. Normalmente abaixo de um milissegundo: o ciclo só despacha, não espera por verificações. |
| `dumbmonit_scheduler_backlog` | gauge | Dispositivos ainda devidos para uma verificação quando o último ciclo terminou. Zero numa instância saudável; um valor não-zero persistente significa que `DUMBMONIT_MAX_CONCURRENT_PROBES` é demasiado baixo para o número de dispositivos e os seus intervalos. |
| `dumbmonit_probes_total{kind}` | counter | Verificações executadas, por tipo de dispositivo (`snmp`, `proxmox`, `http`…). Inclui as que um agente relay executou em nome do servidor. |
| `dumbmonit_probes_failed_total{kind}` | counter | Verificações que falharam, por tipo — um dispositivo inacessível ou um erro de configuração. `rate(dumbmonit_probes_failed_total[15m]) / rate(dumbmonit_probes_total[15m])` é a taxa de falha. |
| `dumbmonit_samples_written_total` | counter | Amostras aceites pela VictoriaMetrics. |
| `dumbmonit_sample_writes_failed_total` | counter | Lotes que a VictoriaMetrics recusou ou não respondeu. O lote permanece em buffer e é repetido; um valor a subir com um `dumbmonit_samples_pending` a subir significa que o armazenamento está em baixo ou cheio. |
| `dumbmonit_samples_pending` | gauge | Amostras ainda no buffer de escrita depois do último flush. Normalmente 0. |
| `dumbmonit_samples_dropped_total` | counter | Amostras descartadas antes mesmo de chegar ao buffer de escrita, porque estava cheio ou fechado. Nunca repetidas — um valor a subir durante uma indisponibilidade significa perda de dados, não só atraso. |
| `dumbmonit_alerting_cycles_total` | counter | Ciclos de alertagem executados (um a cada `DUMBMONIT_ALERT_INTERVAL_SECS`, 30 s por predefinição). |
| `dumbmonit_alerting_cycle_seconds` | gauge | Duração do último ciclo de alertagem, tempo de consulta incluído. Aproximar-se do intervalo significa que as regras estão a consultar mais do que o armazenamento pode servir. |
| `dumbmonit_alerting_rules_evaluated` | gauge | Regras avaliadas no último ciclo. |
| `dumbmonit_alerting_rules_failed` | gauge | Regras cuja consulta falhou no último ciclo. Os seus alertas ficam congelados, não resolvidos. Qualquer coisa além de 0 merece um olhar aos registos. |
| `dumbmonit_alerts{phase}` | gauge | Alertas que o motor acompanha, por fase: `ok`, `pending` (condição verdadeira, `for` não decorrido), `firing`, `resolved`. |
| `dumbmonit_alerts_suppressed` | gauge | Alertas escondidos porque um dispositivo pai está em baixo (a fase efetiva `suppressed`). |
| `dumbmonit_alerts_silenced` | gauge | Alertas cobertos por uma janela de manutenção. |
| `dumbmonit_alerts_learning` | gauge | Alertas de linha de base ainda a aprender, e portanto silenciosos. |
| `dumbmonit_notifications_total{kind}` | counter | Notificações enviadas, por tipo de canal (`email`, `discord`, `ntfy`…). |
| `dumbmonit_notifications_failed_total{kind}` | counter | Notificações que o canal recusou, por tipo. São reenfileiradas e repetidas; um valor a subir constantemente é um webhook ou conta SMTP avariados. |
| `dumbmonit_agents` | gauge | Máquinas que registaram um agente. |
| `dumbmonit_agents_stale` | gauge | Agentes que não enviaram nada há cinco minutos. |
| `dumbmonit_database_up` | gauge | 1 quando o SQLite responde — a mesma verificação que `/api/health`. |
| `dumbmonit_database_bytes` | gauge | Tamanho de `dumbmonit.db` e o seu write-ahead log. Não inclui `/data/vm`. |
| `dumbmonit_victoriametrics_up` | gauge | 1 quando a VictoriaMetrics responde — a mesma verificação que `/api/health`. 0 enquanto o filho incorporado reinicia. |
| `dumbmonit_victoriametrics_embedded` | gauge | 1 quando este servidor executa a VictoriaMetrics ele próprio, 0 quando `DUMBMONIT_VM_URL` aponta para uma externa. |

Nenhuma métrica nesta página transporta uma etiqueta por dispositivo:
o documento tem o mesmo tamanho para um dispositivo e para mil, e
nenhum nome de dispositivo é exposto a quem o puder consultar.

Dois alertas que vale a pena adicionar do seu lado, uma vez o
DumbMonit consultado:

```
# A monitorização parou de monitorizar
up{job="dumbmonit"} == 0

# As medições estão a acumular-se em vez de ser guardadas
dumbmonit_samples_pending > 0 and increase(dumbmonit_sample_writes_failed_total[10m]) > 0
```

### Abrir as rotas sem token {#opening-the-routes-without-a-token}

`DUMBMONIT_METRICS_PUBLIC=true` serve `/metrics`, `/federate` e
`/prometheus/api/v1/…` a quem conseguir alcançar a porta, para quem
filtra a porta por conta própria — **qualquer um nessa rede pode
então ler toda medição de todo dispositivo e o estado da instância,
por isso só a defina quando outra coisa já restringe quem alcança a
porta.**
