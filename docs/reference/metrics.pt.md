# Métricas {#metrics}

O DumbMonit guarda cada medição no VictoriaMetrics e volta a lê-la com MetricsQL,
através do [proxy de métricas](api.md#metrics) do servidor. Esta página descreve
a nomenclatura, as etiquetas e a retenção. As listas de métricas por tipo estão
nas [páginas dos dispositivos](../devices/index.md).

## Nomenclatura {#naming}

Cada métrica tem o prefixo `dumbmonit_`, para que o DumbMonit possa partilhar uma
instância do VictoriaMetrics com outras ferramentas sem colisões. Os nomes
seguem `dumbmonit_<what>_<unit>`:

| Padrão | Exemplos |
|---|---|
| `…_bytes`, `…_bytes_total`, `…_bytes_used` | `dumbmonit_memory_bytes_total`, `dumbmonit_filesystem_used_bytes` |
| `…_percent` | `dumbmonit_cpu_usage_percent`, `dumbmonit_pbs_datastore_used_percent` |
| `…_seconds`, `…_age_seconds` | `dumbmonit_probe_duration_seconds`, `dumbmonit_proxmox_backup_last_age_seconds` |
| `…_celsius`, `…_volts`, `…_hertz`, `…_watts` | Leituras de UPS e Synology |
| `…_info` | Séries de presença (valor 1) cuja informação está nas etiquetas: `dumbmonit_system_info`, `dumbmonit_pbs_version_info` |
| `…_status`, `…_up`, `…_running` | Enumerações ou booleanos: `dumbmonit_if_oper_status`, `dumbmonit_proxmox_guest_running` |

As famílias específicas de uma origem trazem a origem como segundo prefixo:
`dumbmonit_proxmox_*`, `dumbmonit_pbs_*`, `dumbmonit_synology_*`,
`dumbmonit_probe_*` (serviços). Os perfis SNMP e o agente usam nomes simples
(`dumbmonit_if_octets_in`, `dumbmonit_cpu_usage_percent`).

Os contadores (octetos e pacotes de interfaces, bytes de disco, páginas
impressas) são guardados **em bruto**, tal como o dispositivo os reporta: aplique
`rate()` nas suas próprias consultas para obter um débito. A página do
dispositivo desenha todas as séries `dumbmonit_*` do dispositivo tal como estão
guardadas, pelo que um contador aparece como uma linha ascendente. Um reinício
aparece como uma reposição do contador, nunca como um falso pico.

## `dumbmonit_up` {#dumbmonit_up}

Após cada sonda bem-sucedida, o servidor escreve `dumbmonit_up = 1` para o
dispositivo. Nada é escrito quando uma sonda falha: a série simplesmente pára, e
a regra «Dispositivo inacessível» mede a idade da última amostra
(`time() - tlast_over_time(dumbmonit_up[7d])`), mantendo apenas a série mais
recente de cada dispositivo (`bottomk by (target) (1, …)`): uma série que já não
é escrita após uma mudança de etiqueta ou tag permanece na janela de 7 dias e
não deve contar.

Para serviços (`http`, `tcp`, `dns`, `ping`, `tls`), `dumbmonit_up = 1` significa
«o monitor correu», e `dumbmonit_probe_success` diz se o serviço está bem. Os
dois sinais são mantidos separados de propósito: um `probe_success` de 0 diz que
o serviço está em baixo, uma interrupção de `up` diz que a monitorização está
em baixo.

## `dumbmonit_security_score` {#dumbmonit_security_score}

A cada 15 minutos o servidor calcula a [pontuação de segurança](../using/security-score.md)
de cada dispositivo cujo tipo tem verificações de segurança, e escreve
`dumbmonit_security_score` (0–100, ausente enquanto nada puder ser avaliado) e
`dumbmonit_security_checks_failed`, ambas com a etiqueta `target`.

## Etiquetas {#labels}

| Etiqueta | Definida por | Significado |
|---|---|---|
| `target` | servidor | O id do dispositivo, como em `/api/targets/{id}`. Cada regra e cada gráfico seleciona por ela. |
| `host` | servidor | O nome do dispositivo. |
| `tag_<key>` | servidor | As tags livres do dispositivo. As opções do tipo (`insecure_tls`, `port`…), as tags cuja chave parece um segredo (comparadas por palavra: `key`, `pass`, `cert`, `private`, `pem`…) e os valores com mais de 256 caracteres não são copiados. Nunca ponha um segredo numa tag. |
| `probe` | serviços | `http`, `tcp`, `dns`, `ping` ou `tls`. |
| etiquetas por série | coletor | `ifname`, `mountpoint`, `device`, `fstype`, `core`, `service`, `container`, `node`, `vmid`, `name`, `storage`, `datastore`, `group`, `volume`, `disk`, `pool`, `repo`, `changes`, `reason`, `version`, `issuer`… conforme listado em cada página de dispositivo. |

As etiquetas de identidade prevalecem sempre sobre as dos coletores, e para os
agentes são definidas na receção: uma máquina não pode escrever nas séries de
outra.

!!! note "Prefixo anterior a setembro de 2026"
    As séries escritas pelo EzyMonit chamam-se `ezymonit_…`. O prefixo mudou com
    o nome do produto e nada lê as séries antigas: ficam no VictoriaMetrics até
    a retenção as remover, os gráficos e as regras recomeçam a partir da
    atualização, e as regras personalizadas que ainda nomeiam `ezymonit_…` têm
    de ser editadas.

## Retenção e armazenamento {#retention-and-storage}

O VictoriaMetrics incorporado guarda 12 meses de dados (`DUMBMONIT_VM_RETENTION`)
em `/data/vm`, no volume `dumbmonit-data`. O servidor agrupa as suas escritas e
descarrega-as a cada 5 segundos (`DUMBMONIT_FLUSH_INTERVAL_SECS`). Apagar um
dispositivo não apaga as suas séries; envelhecem até expirar.

O histórico de alertas é mantido 90 dias e as linhas de base das séries que
desapareceram durante 60 dias, ambos no SQLite (veja a
[referência de configuração](configuration.md)).

## Consultas {#querying}

- Através do servidor: `GET /api/metrics/query` e `GET /api/metrics/query_range`,
  com o cookie de sessão. Os resultados de intervalo são limitados a 2 000
  pontos por série.
- A partir de um Prometheus ou Grafana existente: `GET /federate`, `GET /metrics`
  e a fonte de dados `/prometheus`, com um token de API —
  [abaixo](#scraping-dumbmonit).
- Diretamente, em desenvolvimento: a sobreposição `docker-compose.dev.yml`
  publica o VictoriaMetrics incorporado em `http://localhost:8428`, com a sua
  própria interface em `http://localhost:8428/vmui/`. Em produção escuta apenas
  no loopback do contentor; defina `DUMBMONIT_VM_LISTEN=0.0.0.0:8428` e publique
  a porta você próprio se quiser o Grafana ou outra ferramenta sobre os mesmos
  dados.

```
# Interface throughput in bits per second
rate(dumbmonit_if_octets_in{target="4"}[5m]) * 8

# Availability of every service over 30 days
avg by (host) (avg_over_time(dumbmonit_probe_success[30d]))

# Every device that has not reported for 5 minutes
bottomk by (target) (1, time() - tlast_over_time(dumbmonit_up[7d])) > 300
```

## Recolher métricas do DumbMonit {#scraping-dumbmonit}

Se já corre um Prometheus ou um Grafana, não precisa de uma segunda pilha. Três
rotas ficam fora de `/api`, onde um recoletor as procura:

| Rota | O que devolve |
|---|---|
| `GET /metrics` | A saúde da própria instância DumbMonit: agendador, sondas, escritas, alertas, notificações, agentes, base de dados. Algumas dezenas de séries, sempre o mesmo número — nunca uma por dispositivo. |
| `GET /federate` | As medições, selecionadas por `match[]`, exatamente como o Prometheus federa outro Prometheus: o último ponto de cada série selecionada. |
| `GET /prometheus/api/v1/…` | A metade de leitura da API do Prometheus, retransmitida para o armazenamento: o que uma fonte de dados do Grafana chama. |

As três aceitam `Authorization: Bearer dmt_…` com o âmbito **`read`** — um token
criado em Definições → *API e assistentes*, do mesmo tipo que o
[servidor MCP](../using/assistant.md) usa. Não há cookie envolvido, pelo que não
é preciso o cabeçalho `X-Requested-With`; um cookie de sessão também é aceite,
para poder abrir qualquer uma a partir de um navegador com sessão iniciada. Um
token está limitado a 120 chamadas por minuto: muito para uma recolha a cada 15
segundos, e suficiente para um dashboard do Grafana de algumas dezenas de
painéis — dê a um dashboard muito usado o seu próprio token em vez de o
partilhar com um recoletor.

### Prometheus {#prometheus}

```yaml
# prometheus.yml — DumbMonit's own health, then the measurements it collects.
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

`honor_labels: true` mantém as etiquetas `target` e `host` que o DumbMonit define,
em vez de as sobrescrever com as do próprio job de recolha. Ponha o token num
ficheiro e use
`authorization: {type: Bearer, credentials_file: /etc/prometheus/dumbmonit.token}`
se preferir não o ter na configuração.

Sem `match[]`, `/federate` seleciona `{__name__=~"dumbmonit_.*"}` — todas as
séries que o DumbMonit escreve, e nada que pertença a outra ferramenta que
partilhe o mesmo VictoriaMetrics. Restrinja-o para manter uma federação pequena:
`{__name__=~"dumbmonit_.*",target="4"}` para um dispositivo,
`{__name__="dumbmonit_up"}` só para a disponibilidade. No máximo 10 seletores
por pedido, e uma resposta acima de 8 MiB é recusada com uma mensagem a dizer
para a restringir — divida uma frota grande em vários jobs com seletores mais
estreitos em vez de um que peça tudo. `max_lookback=15m` alarga a janela em que
o último ponto é procurado, para dispositivos sondados menos de cinco em cinco
minutos.

```bash
curl -H 'Authorization: Bearer dmt_…' http://localhost:8080/metrics
curl -H 'Authorization: Bearer dmt_…' -G http://localhost:8080/federate \
  --data-urlencode 'match[]={__name__=~"dumbmonit_.*",target="4"}'
```

### Grafana {#grafana}

Aponte o Grafana ao **DumbMonit**, não ao VictoriaMetrics que ele corre: o
processo filho escuta no loopback do contentor, não tem autenticação própria, e
expõe as rotas que *apagam* séries tão prontamente como as que as leem. O
DumbMonit retransmite a metade de leitura da API do Prometheus em `/prometheus`,
atrás do mesmo token, pelo que o Grafana fala com uma fonte de dados Prometheus
normal:

1. **Connections → Data sources → Add → Prometheus**.
2. **URL**: `http://dumbmonit.lan:8080/prometheus` (o Grafana acrescenta
   `/api/v1/query` por si).
3. **HTTP headers**: adicione um, `Authorization` = `Bearer dmt_…`. O Grafana
   guarda-o cifrado e nunca mais o mostra.
4. **Save & test** — o Grafana reporta que a fonte de dados funciona, e todas as
   séries `dumbmonit_*` ficam então disponíveis em PromQL, com
   autocompletamento de nomes de métricas e etiquetas.

Só as rotas de leitura são retransmitidas — `query`, `query_range`, `series`,
`labels`, valores de etiquetas, `metadata` e a versão que o Grafana pede ao
testar a ligação. Todo o resto, `admin/tsdb/delete_series` à cabeça, responde
`404`: este ponto de entrada não pode alterar nem apagar nada.

Se já corre um Prometheus, o outro caminho funciona igualmente bem: o Prometheus
federa o DumbMonit com o job acima, o Grafana lê o Prometheus, e as consultas
são idênticas.

### Métricas da instância {#instance-metrics}

Tudo o que `GET /metrics` expõe. Os valores são lidos quando recolhe; os
contadores `_total` são monótonos e só recomeçam quando o servidor reinicia
(`dumbmonit_uptime_seconds` diz-lhe quando isso aconteceu).

| Métrica | Tipo | Significado |
|---|---|---|
| `dumbmonit_build_info{version}` | gauge | Sempre 1. A etiqueta `version` é a versão do servidor em execução, a mesma que `/api/health` reporta. |
| `dumbmonit_uptime_seconds` | gauge | Segundos desde que o processo do servidor arrancou. |
| `dumbmonit_scheduler_cycles_total` | counter | Ciclos do agendador executados. O ciclo bate uma vez por segundo, pelo que serve também de sinal de vida. |
| `dumbmonit_scheduler_cycle_seconds` | gauge | Duração do último ciclo. Normalmente abaixo de um milissegundo: o ciclo apenas despacha, não espera pelas sondas. |
| `dumbmonit_scheduler_backlog` | gauge | Dispositivos ainda a aguardar uma sonda quando o último ciclo terminou. Zero numa instância saudável; um valor diferente de zero duradouro significa que `DUMBMONIT_MAX_CONCURRENT_PROBES` é demasiado baixo para o número de dispositivos e os seus intervalos. |
| `dumbmonit_probes_total{kind}` | counter | Sondas executadas, por tipo de dispositivo (`snmp`, `proxmox`, `http`…). Inclui as que um agente de retransmissão executou em nome do servidor. |
| `dumbmonit_probes_failed_total{kind}` | counter | Sondas que falharam, por tipo — um dispositivo inacessível ou um erro de configuração. `rate(dumbmonit_probes_failed_total[15m]) / rate(dumbmonit_probes_total[15m])` é a taxa de falhas. |
| `dumbmonit_samples_written_total` | counter | Amostras aceites pelo VictoriaMetrics. |
| `dumbmonit_sample_writes_failed_total` | counter | Lotes que o VictoriaMetrics recusou ou não respondeu. O lote fica em buffer e é repetido; um valor a subir com um `dumbmonit_samples_pending` a subir significa que o armazenamento está em baixo ou cheio. |
| `dumbmonit_samples_pending` | gauge | Amostras ainda no buffer de escrita após o último flush. Normalmente 0. |
| `dumbmonit_samples_dropped_total` | counter | Amostras descartadas antes sequer de chegarem ao buffer de escrita, porque estava cheio ou fechado. Nunca repetidas — um valor a subir durante uma falha significa perda de dados, não apenas atraso. |
| `dumbmonit_alerting_cycles_total` | counter | Ciclos de alertas executados (um a cada `DUMBMONIT_ALERT_INTERVAL_SECS`, 30 s por predefinição). |
| `dumbmonit_alerting_cycle_seconds` | gauge | Duração do último ciclo de alertas, tempo de consulta incluído. Aproximar-se do intervalo significa que as regras consultam mais do que o armazenamento consegue servir. |
| `dumbmonit_alerting_rules_evaluated` | gauge | Regras avaliadas no último ciclo. |
| `dumbmonit_alerting_rules_failed` | gauge | Regras cuja consulta falhou no último ciclo. Os seus alertas ficam congelados, não resolvidos. Qualquer valor diferente de 0 merece uma olhadela aos registos. |
| `dumbmonit_alerts{phase}` | gauge | Alertas que o motor acompanha, por fase: `ok`, `pending` (condição verdadeira, `for` não decorrido), `firing`, `resolved`. |
| `dumbmonit_alerts_suppressed` | gauge | Alertas ocultos porque um dispositivo pai está em baixo (a fase efetiva `suppressed`). |
| `dumbmonit_alerts_silenced` | gauge | Alertas cobertos por uma janela de manutenção. |
| `dumbmonit_alerts_learning` | gauge | Alertas de linha de base ainda em aprendizagem, e por isso silenciosos. |
| `dumbmonit_notifications_total{kind}` | counter | Notificações enviadas, por tipo de canal (`email`, `discord`, `ntfy`…). |
| `dumbmonit_notifications_failed_total{kind}` | counter | Notificações que o canal recusou, por tipo. São recolocadas na fila e repetidas; um valor em subida constante é um webhook ou conta SMTP avariados. |
| `dumbmonit_agents` | gauge | Máquinas que registaram um agente. |
| `dumbmonit_agents_stale` | gauge | Agentes que não enviaram nada há cinco minutos. |
| `dumbmonit_database_up` | gauge | 1 quando o SQLite responde — a mesma verificação que `/api/health`. |
| `dumbmonit_database_bytes` | gauge | Tamanho de `dumbmonit.db` e do seu write-ahead log. Não inclui `/data/vm`. |
| `dumbmonit_victoriametrics_up` | gauge | 1 quando o VictoriaMetrics responde — a mesma verificação que `/api/health`. 0 enquanto o filho incorporado está a reiniciar. |
| `dumbmonit_victoriametrics_embedded` | gauge | 1 quando este servidor corre ele próprio o VictoriaMetrics, 0 quando `DUMBMONIT_VM_URL` aponta para um externo. |

Nenhuma métrica desta página traz uma etiqueta por dispositivo: o documento tem
o mesmo tamanho para um dispositivo e para mil, e nenhum nome de dispositivo
vaza para quem o possa recolher.

Dois alertas que vale a pena acrescentar do seu lado, quando o DumbMonit for
recolhido:

```
# The monitoring stopped monitoring
up{job="dumbmonit"} == 0

# Measurements are piling up instead of being stored
dumbmonit_samples_pending > 0 and increase(dumbmonit_sample_writes_failed_total[10m]) > 0
```

### Abrir as rotas sem token {#opening-the-routes-without-a-token}

`DUMBMONIT_METRICS_PUBLIC=true` serve `/metrics`, `/federate` e
`/prometheus/api/v1/…` a quem alcançar a porta, para quem filtra a porta por si
— **qualquer pessoa nessa rede pode então ler todas as medições de todos os
dispositivos e o estado da instância, pelo que só a defina quando outra coisa já
restrinja quem alcança a porta.**
