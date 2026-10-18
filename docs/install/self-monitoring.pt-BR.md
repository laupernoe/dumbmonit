# Vigiando o próprio DumbMonit {#watching-dumbmonit-itself}

Quem vigia o vigia? O DumbMonit responde em duas partes:

- um **heartbeat de saída**, para o que ele não consegue ver por dentro: o
  host fora do ar, o contêiner parado, o motor travado;
- **regras integradas** sobre a própria saúde, para o que ele consegue ver: o
  volume de dados enchendo, o banco de métricas reiniciando em loop, sondagens
  travando, gravações falhando, o banco de dados crescendo de forma anormal.

Ambos ficam em **Configurações → Sistema → O próprio DumbMonit**, que também
mostra o estado do motor neste momento.

## O heartbeat de saída {#the-outgoing-heartbeat}

O DumbMonit chama uma URL sua em intervalo regular (60 segundos por padrão),
**somente enquanto o motor está saudável**:

- o agendador está rodando (completou uma rodada nos últimos dois minutos);
- o banco SQLite responde;
- o VictoriaMetrics responde.

Quando um deles falha, nada é enviado. Um serviço de "dead man's switch" espera
essas chamadas e alerta você quando elas param, seja a causa o host inteiro, o
contêiner ou um único componente.

### Configurando {#setting-it-up}

1. Crie uma verificação no serviço de sua escolha e copie a URL de ping dela:
   - **Healthchecks.io** (ou um Healthchecks auto-hospedado): uma verificação
     com período igual ao intervalo e um tempo de tolerância um pouco maior
     (`https://hc-ping.com/<uuid>`);
   - **Uptime Kuma**: um monitor do tipo *Push*
     (`https://kuma.example.com/api/push/<token>?status=up&msg=OK&ping=`);
   - **Better Stack**: um heartbeat (`https://uptime.betterstack.com/api/v1/heartbeat/<token>`);
   - qualquer serviço que aceite um `GET` simples e alerte quando ele deixa de
     chegar (Cronitor, Dead Man's Snitch…).
2. Em **Configurações → Sistema → O próprio DumbMonit**, cole a URL, escolha o
   intervalo (30 a 3600 segundos), ative **Enviar um heartbeat** e salve.
3. Pressione **Testar**: uma chamada sai na hora e o resultado é mostrado. O
   serviço deve marcar a verificação como ativa.

A chamada é um `GET` com timeout de 10 segundos. Ela segue a mesma proteção dos
canais de notificação: nunca é enviada à própria interface web do DumbMonit nem
ao seu VictoriaMetrics.

!!! note "ntfy"
    O ntfy entrega toda mensagem que recebe, então apontar o heartbeat para um
    tópico do ntfy envia uma notificação a cada minuto, em vez de quando as
    chamadas param. Use um serviço de dead man's switch e mantenha o ntfy como
    canal para os próprios alertas.

### Sinal de falha {#failure-signal}

Por padrão, um motor doente simplesmente para de chamar, e o serviço alerta
quando seu tempo de tolerância passa. Ative **Sinalizar falhas** para chamar a
URL de falha, que o serviço reporta na hora:

| Serviço | URL de falha |
|---|---|
| Healthchecks.io, Better Stack | `/fail` adicionado ao caminho: `https://hc-ping.com/<uuid>/fail` |
| Uptime Kuma | `status=up` substituído por `status=down` na query |

Deixe desligado para qualquer outro serviço: uma URL desconhecida terminada em
`/fail` provavelmente responderia 404, o que na prática parece nada ter sido
enviado.

### A URL é um segredo {#the-url-is-a-secret}

Uma URL de ping normalmente traz seu próprio token: quem a conhece pode fazer
sua verificação parecer saudável. O DumbMonit a guarda criptografada com o
segredo da instância, nunca a devolve pela API (só o host é mostrado) e a
deixa fora dos logs e das mensagens de erro. Para trocá-la, digite a nova; para
mantê-la, deixe o campo vazio. **Remover a URL** a esquece e desliga o
heartbeat.

A configuração não faz parte do pacote de configuração criptografado: defina-a
de novo ao restaurar em uma instância nova.

## Regras integradas {#built-in-rules}

A cada minuto o DumbMonit grava algumas medições sobre si mesmo no
VictoriaMetrics, e as regras integradas do grupo *O próprio DumbMonit* as
leem. Elas se comportam como qualquer outra regra: mesmos canais, mesmos
silêncios, mesmos limites por regra em **Alertas → Regras**.

| Regra | Dispara quando | Padrão |
|---|---|---|
| DumbMonit data volume almost full | Menos de 10% livres no volume que contém `/data` | 15 min, aviso |
| DumbMonit metrics database restarting in a loop | O VictoriaMetrics embutido foi reiniciado 3 vezes ou mais em 30 minutos | 1 min, crítico |
| DumbMonit scheduler stalled | Nenhuma rodada de agendamento há mais de dois minutos | 5 min, crítico |
| DumbMonit polls running late | Monitores continuam atrasados após cada rodada por 15 minutos | 15 min, aviso |
| DumbMonit metric writes failing | Lotes recusados pelo VictoriaMetrics, ou amostras descartadas, nos últimos 15 minutos | 5 min, aviso |
| DumbMonit database growing fast | `dumbmonit.db` mais que dobrou em um dia e passa de 100 MB | 1 h, aviso |
| DumbMonit backup did not run | Nenhum backup local bem-sucedido há dois dias ([detalhes](backup.md#scheduled-local-backups)) | 1 h, aviso |

Essas regras precisam do VictoriaMetrics para serem avaliadas. Quando o próprio
VictoriaMetrics está fora do ar de vez, ou o host inteiro está, nenhuma regra
pode disparar: esse é o trabalho do heartbeat.

### Medições {#measurements}

| Métrica | Significado |
|---|---|
| `dumbmonit_instance_data_free_bytes` | Espaço livre no volume que contém `/data`, para um usuário sem privilégios. |
| `dumbmonit_instance_data_size_bytes` | Tamanho desse volume. |
| `dumbmonit_instance_database_bytes` | Tamanho de `dumbmonit.db` e do seu write-ahead log. |
| `dumbmonit_instance_scheduler_lag_seconds` | Segundos desde que o agendador completou uma rodada pela última vez (normalmente abaixo de 2). |
| `dumbmonit_instance_scheduler_backlog` | Monitores ainda devendo uma sondagem após a última rodada. |
| `dumbmonit_instance_sample_writes_failed_total` | Lotes que o VictoriaMetrics recusou ou não respondeu, desde que o servidor iniciou. |
| `dumbmonit_instance_samples_dropped_total` | Amostras descartadas antes de chegar ao buffer de gravação, desde que o servidor iniciou. |
| `dumbmonit_instance_vm_restarts_total` | Reinícios do VictoriaMetrics embutido desde que o servidor iniciou (apenas embutido). |

Elas não têm o rótulo `target`: os alertas que geram não ficam presos a um
monitor. Para um Prometheus seu, `GET /metrics` expõe o mesmo tipo de números,
veja [Coletando do DumbMonit](../reference/metrics.md#scraping-dumbmonit).

## API {#api}

| Método | Rota | |
|---|---|---|
| `GET` | `/api/self-monitoring` | Estado do motor, volume de dados, tamanho do banco, configuração do heartbeat e último heartbeat. |
| `PUT` | `/api/self-monitoring/heartbeat` | `{ "enabled", "interval_seconds", "send_fail", "url" }`. `url` omitida a mantém, `""` a apaga. |
| `POST` | `/api/self-monitoring/heartbeat/test` | Uma chamada agora, para `{ "url" }` ou para a URL guardada. |

As duas escritas são para administradores (ou um token com o escopo `write`).
