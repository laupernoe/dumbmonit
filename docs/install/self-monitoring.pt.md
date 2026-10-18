# Vigiar o próprio DumbMonit {#watching-dumbmonit-itself}

Quem vigia o vigilante? O DumbMonit responde em duas partes:

- um **heartbeat de saída**, para o que não consegue ver por dentro: o
  anfitrião em baixo, o contentor parado, o motor bloqueado;
- **regras integradas** sobre a sua própria saúde, para o que consegue ver: o
  volume de dados a encher, a base de dados de métricas a reiniciar em ciclo,
  sondagens paradas, escritas a falhar, a base de dados a crescer de forma
  anormal.

Ambos vivem em **Settings → System → DumbMonit itself**, que também mostra o
estado do motor neste momento.

## O heartbeat de saída {#the-outgoing-heartbeat}

O DumbMonit chama um URL seu a intervalos regulares (60 segundos por
predefinição), **apenas enquanto o motor está saudável**:

- o agendador está a correr (concluiu uma ronda nos últimos dois minutos);
- a base de dados SQLite responde;
- o VictoriaMetrics responde.

Quando um deles falha, nada é enviado. Um serviço de «dead man's switch» espera
por estas chamadas e avisa-o quando deixam de chegar, seja a causa o
anfitrião inteiro, o contentor ou um componente.

### Configurar {#setting-it-up}

1. Crie uma verificação no serviço da sua escolha e copie o seu URL de ping:
   - **Healthchecks.io** (ou um Healthchecks autoalojado): uma verificação com
     um período igual ao intervalo, e um tempo de tolerância um pouco maior
     (`https://hc-ping.com/<uuid>`);
   - **Uptime Kuma**: um monitor do tipo *Push*
     (`https://kuma.example.com/api/push/<token>?status=up&msg=OK&ping=`);
   - **Better Stack**: um heartbeat (`https://uptime.betterstack.com/api/v1/heartbeat/<token>`);
   - qualquer serviço que aceite um `GET` simples e avise quando deixa de
     chegar (Cronitor, Dead Man's Snitch…).
2. Em **Settings → System → DumbMonit itself**, cole o URL, escolha o
   intervalo (30 a 3600 segundos), ative **Send a heartbeat** e guarde.
3. Prima **Test**: uma chamada sai de imediato e o resultado é mostrado. O
   serviço deve marcar a verificação como ativa.

A chamada é um `GET` com tempo limite de 10 segundos. Segue a mesma proteção
dos canais de notificação: nunca é enviada para a interface web do próprio
DumbMonit nem para o seu VictoriaMetrics.

!!! note "ntfy"
    O ntfy entrega todas as mensagens que recebe, por isso apontar o heartbeat
    a um tópico ntfy envia-lhe uma notificação por minuto em vez de quando as
    chamadas param. Use um serviço de dead man's switch, e mantenha o ntfy
    como canal para os próprios alertas.

### Sinal de falha {#failure-signal}

Por predefinição, um motor doente simplesmente deixa de chamar, e o serviço
avisa assim que o seu tempo de tolerância passa. Ative **Signal failures**
para chamar em vez disso o URL de falha, que o serviço reporta de imediato:

| Serviço | URL de falha |
|---|---|
| Healthchecks.io, Better Stack | `/fail` acrescentado ao caminho: `https://hc-ping.com/<uuid>/fail` |
| Uptime Kuma | `status=up` substituído por `status=down` na query |

Deixe desativado para qualquer outro serviço: um URL desconhecido com `/fail`
no fim responderia muito provavelmente 404, o que parece de qualquer modo que
nada é enviado.

### O URL é um segredo {#the-url-is-a-secret}

Um URL de ping costuma conter o seu próprio token: quem o souber pode fazer a
sua verificação parecer saudável. O DumbMonit guarda-o cifrado com o segredo
da instância, nunca o devolve pela API (só o seu anfitrião é mostrado), e
deixa-o fora dos logs e das mensagens de erro. Para o mudar, escreva o novo;
para o manter, deixe o campo vazio. **Remove the URL** esquece-o e desativa o
heartbeat.

A definição não faz parte do pacote de configuração cifrado: defina-a de novo
depois de restaurar numa instância nova.

## Regras integradas {#built-in-rules}

A cada minuto o DumbMonit escreve no VictoriaMetrics algumas medições sobre si
próprio, e as regras integradas do grupo *DumbMonit itself* leem-nas.
Comportam-se como qualquer outra regra: os mesmos canais, os mesmos
silenciamentos, os mesmos limiares por regra em **Alerts → Rules**.

| Regra | Dispara quando | Predefinição |
|---|---|---|
| DumbMonit data volume almost full | Menos de 10 % livres no volume que contém `/data` | 15 min, warning |
| DumbMonit metrics database restarting in a loop | O VictoriaMetrics embebido foi reiniciado 3 vezes ou mais em 30 minutos | 1 min, critical |
| DumbMonit scheduler stalled | Nenhuma ronda de agendamento há mais de dois minutos | 5 min, critical |
| DumbMonit polls running late | Os monitores continuam em atraso depois de cada ronda durante 15 minutos | 15 min, warning |
| DumbMonit metric writes failing | Lotes recusados pelo VictoriaMetrics, ou amostras descartadas, nos últimos 15 minutos | 5 min, warning |
| DumbMonit database growing fast | `dumbmonit.db` mais do que duplicou num dia e tem mais de 100 MB | 1 h, warning |
| DumbMonit backup did not run | Nenhuma cópia de segurança local bem-sucedida há dois dias ([detalhes](backup.md#scheduled-local-backups)) | 1 h, warning |

Estas regras precisam do VictoriaMetrics para serem avaliadas. Quando o
próprio VictoriaMetrics está em baixo em definitivo, ou o anfitrião inteiro
está, nenhuma regra pode disparar: é esse o papel do heartbeat.

### Medições {#measurements}

| Métrica | Significado |
|---|---|
| `dumbmonit_instance_data_free_bytes` | Espaço livre no volume que contém `/data`, para um utilizador sem privilégios. |
| `dumbmonit_instance_data_size_bytes` | Tamanho desse volume. |
| `dumbmonit_instance_database_bytes` | Tamanho de `dumbmonit.db` e do seu write-ahead log. |
| `dumbmonit_instance_scheduler_lag_seconds` | Segundos desde que o agendador concluiu uma ronda pela última vez (normalmente abaixo de 2). |
| `dumbmonit_instance_scheduler_backlog` | Monitores ainda por sondar depois da última ronda. |
| `dumbmonit_instance_sample_writes_failed_total` | Lotes que o VictoriaMetrics recusou ou a que não respondeu, desde que o servidor arrancou. |
| `dumbmonit_instance_samples_dropped_total` | Amostras descartadas antes de chegarem ao buffer de escrita, desde que o servidor arrancou. |
| `dumbmonit_instance_vm_restarts_total` | Reinícios do VictoriaMetrics embebido desde que o servidor arrancou (só embebido). |

Não levam etiqueta `target`: os alertas que geram não ficam associados a um
monitor. Para um Prometheus seu, `GET /metrics` expõe o mesmo tipo de valores,
veja [Consultar o DumbMonit](../reference/metrics.md#scraping-dumbmonit).

## API {#api}

| Método | Rota | |
|---|---|---|
| `GET` | `/api/self-monitoring` | Estado do motor, volume de dados, tamanho da base de dados, definição do heartbeat e último heartbeat. |
| `PUT` | `/api/self-monitoring/heartbeat` | `{ "enabled", "interval_seconds", "send_fail", "url" }`. `url` omitido mantém-no, `""` apaga-o. |
| `POST` | `/api/self-monitoring/heartbeat/test` | Uma chamada agora, para `{ "url" }` ou para o URL guardado. |

As duas escritas são para administradores (ou um token com o âmbito `write`).
