# Graylog {#graylog}

O servidor de logs e o cluster de pesquisa por detrás dele: se o Graylog
processa mensagens, quantas esperam no seu journal, quão cheios estão os
seus buffers, o que entra e o que sai, a saúde do OpenSearch ou Elasticsearch
como o Graylog a vê, entradas que falharam ao iniciar, e mensagens que
falharam ao ser indexadas.

O DumbMonit lê a API REST do Graylog com um utilizador dedicado que detém a
função incorporada **Reader**. Nunca pesquisa mensagens, nunca inicia ou
para uma entrada e nunca confirma uma notificação.

A falha a vigiar: **o cluster de pesquisa deixa de acompanhar**. O Graylog
continua a aceitar mensagens e escreve-as no seu journal em disco; a
interface web abre, as pesquisas funcionam, e simplesmente não mostram nada
recente. O journal cresce até ficar cheio, depois as mensagens são
descartadas. O atraso do journal, a saúde do cluster de pesquisa e as falhas
de indexação dizem-o muito antes.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_graylog_`. O journal, os buffers
e o caudal são os **do nó com que o DumbMonit fala**: num cluster Graylog,
adicione cada nó como o seu próprio monitor.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `processing` | 1 enquanto o nó processa mensagens, 0 quando o processamento está pausado | |
| `running`, `lb_alive` | o ciclo de vida do nó é `running`; reporta-se a si próprio como ativo aos balanceadores de carga | |
| `version_info` | valor 1 | `version` |
| `journal_uncommitted_entries` | mensagens escritas no journal e ainda não processadas | |
| `journal_size_bytes`, `journal_size_limit_bytes`, `journal_used_percent` | | |
| `journal_append_per_second`, `journal_read_per_second` | | |
| `buffer_used_percent` | quão cheios estão os buffers de entrada, processamento e saída | `buffer` |
| `messages_in_total`, `messages_out_total` | mensagens recebidas, e escritas para o cluster de pesquisa | |
| `indexer_status` | saúde do cluster de pesquisa: 0 verde, 1 amarelo, 2 vermelho, 3 inalcançável a partir do Graylog | |
| `indexer_shards` | shards por estado: `active`, `initializing`, `relocating`, `unassigned` | `state` |
| `inputs_running`, `inputs_failed` | entradas neste nó | |
| `input_failed` | valor 1 por entrada que falhou ao iniciar | `input` (o seu título) |
| `output_failures_total`, `processing_failures_total`, `invalid_timestamps_total` | escritas para o cluster de pesquisa que falharam, mensagens que falharam o processamento ou a indexação, mensagens com uma data-hora inválida | |
| `notifications`, `notifications_urgent`, `notification` | as próprias notificações do Graylog (permissão opcional, ver abaixo) | `type`, `severity` |
| `cluster_nodes` | nós no cluster Graylog | |
| `scrape_errors` | chamadas que falharam na última verificação | |

O cluster de pesquisa é lido através do Graylog, por isso não é necessário
um monitor OpenSearch ou Elasticsearch separado para saber se está
verde; quando o Graylog não o consegue alcançar de todo, `indexer_status`
lê 3.

As [regras incorporadas](../alerting/rules.md#log-and-metrics-servers) que
se aplicam: Graylog not processing, Graylog search cluster down, Graylog
search cluster yellow, Graylog journal filling, Graylog input failed,
Graylog indexing failures, mais Monitor unreachable.

## A página do monitor {#the-device-page}

O painel acima dos gráficos lê o que a verificação guardou; abrir a página
nunca consulta o Graylog. Diz o que está mal numa frase (processamento,
cluster de pesquisa, atraso do journal, entradas falhadas pelo nome, falhas
de indexação, notificações urgentes), depois as mensagens recebidas e
enviadas por segundo, o atraso do journal e o número de nós, depois os
buffers, os shards por estado e as próprias notificações do Graylog.

## Criar um utilizador e token só de leitura no Graylog {#create-a-read-only-user-and-token-in-graylog}

1. No Graylog: System → Users and Teams → Create user. Dê o seguinte nome ao utilizador, uma senha longa e aleatória, e atribua apenas a função Reader. Reader pode ler o estado do nó, journal, buffers, caudal, entradas e saúde do cluster de pesquisa, e nenhuma mensagem enquanto nenhum stream for partilhado com ele.

    ```
    dumbmonit
    ```

2. Abra a página Edit tokens desse utilizador, crie um token com o nome do DumbMonit e copie-o agora: o Graylog só o mostra uma vez.

3. Opcional: as próprias notificações do Graylog (uma entrada que falhou ao iniciar, limites de disco, um journal quase cheio) precisam de mais uma permissão, notifications:read, que nenhuma função incorporada concede por si só. Com uma conta autorizada a gerir funções, crie uma função que detenha apenas essa permissão e dê-a ao utilizador dumbmonit. Sem ela, as notificações são ignoradas e nada mais muda.

    ```
    curl -u YOUR_USER -H 'X-Requested-By: cli' -H 'Content-Type: application/json' -X POST https://graylog.lan/api/roles -d '{"name":"DumbMonit Notifications","description":"Read system notifications","permissions":["notifications:read"],"read_only":false}'
    curl -u YOUR_USER -H 'X-Requested-By: cli' -X PUT 'https://graylog.lan/api/roles/DumbMonit%20Notifications/members/dumbmonit'
    ```

4. No DumbMonit, introduza o endereço do nó Graylog, por exemplo «graylog.lan» (porta 9000) ou «https://graylog.lan» atrás de um proxy inverso, e cole o token. Num cluster Graylog, adicione cada nó: journal, buffers e caudal são por nó.

5. O DumbMonit só lê. Nunca pesquisa mensagens, nunca inicia ou para uma entrada e nunca confirma uma notificação.

!!! warning
    Não reutilize a conta com que inicia sessão: um token exposto transporta todos os direitos do seu utilizador, enquanto a função Reader acima não pode alterar nada. O Graylog serve HTTP simples a menos que tenha configurado TLS, e o token viaja em cada pedido: numa rede não confiável, coloque o Graylog atrás de HTTPS.

O que cada permissão Reader cobre: `system:read` o estado do nó e os nós do
cluster, `journal:read` o journal, `metrics:read` e `throughput:read` o
caudal e os buffers, `inputs:read` as entradas, `indexercluster:read` a
saúde do cluster de pesquisa. A função Reader também detém `messages:read`,
que a deixa ler mensagens só dos streams explicitamente partilhados com ela:
não partilhe nenhum. Os detalhes de falhas de indexação (`indices:failures`)
não são necessários: o DumbMonit conta as falhas a partir das próprias
métricas do Graylog.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Token de acesso (recomendado) | Um token do utilizador dedicado. O Graylog espera-o como autenticação HTTP básica com o token como nome de utilizador e a palavra `token` como senha; o DumbMonit faz isto por si. |
| Nome de utilizador / senha | O próprio utilizador dedicado. |

Endereço: um nome de anfitrião ou IP (`graylog.lan`), `anfitrião:porta`, ou
um URL completo (`https://graylog.lan`); um `/api` final é aceite. Opções:
protocolo (HTTP por predefinição), porta (9000), verificação de certificado
e tempo limite do pedido (10 s).
