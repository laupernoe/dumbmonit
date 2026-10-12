# Graylog

O servidor de logs e o cluster de busca detrás dele: se o Graylog processa
mensagens, quantas esperam em seu journal, quão cheios estão seus buffers, o
que entra e o que sai, a saúde do OpenSearch ou Elasticsearch como o Graylog
a vê, entradas que falharam ao iniciar, e mensagens que falharam ao serem
indexadas.

O DumbMonit lê a API REST do Graylog com um usuário dedicado que possui o
papel embutido **Reader**. Ele nunca pesquisa mensagens, nunca inicia ou
para uma entrada e nunca reconhece uma notificação.

A falha que vale a pena observar: **o cluster de busca para de acompanhar o
ritmo**. O Graylog continua aceitando mensagens e as grava em seu journal em
disco; a interface web abre, as buscas funcionam, e simplesmente não mostram
nada recente. O journal cresce até ficar cheio, então mensagens são
descartadas. O acúmulo do journal, a saúde do cluster de busca e as falhas
de indexação avisam isso muito antes.

## O que ele observa

Todas as métricas têm o prefixo `dumbmonit_graylog_`. O journal, os buffers e
a vazão são os **do nó com o qual o DumbMonit fala**: em um cluster Graylog,
adicione cada nó como seu próprio dispositivo.

| Métrica | O que é | Rótulos |
|---|---|---|
| `processing` | 1 enquanto o nó processa mensagens, 0 quando o processamento está pausado | |
| `running`, `lb_alive` | o ciclo de vida do nó é `running`; ele se reporta vivo aos load balancers | |
| `version_info` | valor 1 | `version` |
| `journal_uncommitted_entries` | mensagens gravadas no journal e ainda não processadas | |
| `journal_size_bytes`, `journal_size_limit_bytes`, `journal_used_percent` | | |
| `journal_append_per_second`, `journal_read_per_second` | | |
| `buffer_used_percent` | quão cheios estão os buffers de entrada, processo e saída | `buffer` |
| `messages_in_total`, `messages_out_total` | mensagens recebidas, e gravadas no cluster de busca | |
| `indexer_status` | saúde do cluster de busca: 0 verde, 1 amarelo, 2 vermelho, 3 inacessível pelo Graylog | |
| `indexer_shards` | shards por estado: `active`, `initializing`, `relocating`, `unassigned` | `state` |
| `inputs_running`, `inputs_failed` | entradas neste nó | |
| `input_failed` | valor 1 por entrada que falhou ao iniciar | `input` (seu título) |
| `output_failures_total`, `processing_failures_total`, `invalid_timestamps_total` | escritas no cluster de busca que falharam, mensagens que falharam no processamento ou indexação, mensagens com timestamp inválido | |
| `notifications`, `notifications_urgent`, `notification` | as próprias notificações do Graylog (permissão opcional, veja abaixo) | `type`, `severity` |
| `cluster_nodes` | nós no cluster Graylog | |
| `scrape_errors` | chamadas que falharam na última sondagem | |

O cluster de busca é lido através do Graylog, então um dispositivo separado
de OpenSearch ou Elasticsearch não é necessário para saber se está verde;
quando o Graylog não consegue alcançá-lo de forma alguma, `indexer_status`
marca 3.

As [regras integradas](../alerting/rules.md#log-and-metrics-servers) que se
aplicam: Graylog not processing, Graylog search cluster down, Graylog search
cluster yellow, Graylog journal filling, Graylog input failed, Graylog
indexing failures, além de Device unreachable.

## A página do dispositivo

O painel acima dos gráficos lê o que a sondagem armazenou; abrir a página
nunca consulta o Graylog. Ele diz o que está errado em uma frase
(processamento, cluster de busca, acúmulo do journal, entradas com falha
pelo nome, falhas de indexação, notificações urgentes), depois mensagens
entrando e saindo por segundo, o acúmulo do journal e o número de nós,
depois os buffers, os shards por estado e as próprias notificações do
Graylog.

## Criar um usuário e token somente leitura no Graylog

1. No Graylog: System → Users and Teams → Create user. Nomeie o usuário como
   abaixo, dê a ele uma senha longa e aleatória, e atribua apenas o papel
   Reader. O Reader pode ler o status do nó, journal, buffers, vazão,
   entradas e a saúde do cluster de busca, e nenhuma mensagem, desde que
   nenhum stream seja compartilhado com ele.

    ```
    dumbmonit
    ```

2. Abra a página Edit tokens desse usuário, crie um token nomeado após
   DumbMonit e copie-o agora: o Graylog o mostra apenas uma vez.

3. Opcional: as próprias notificações do Graylog (uma entrada que falhou ao
   iniciar, marcas de água de disco, um journal quase cheio) precisam de mais
   uma permissão, notifications:read, que nenhum papel embutido concede por
   conta própria. Com uma conta autorizada a gerenciar papéis, crie um papel
   contendo apenas essa permissão e dê-o ao usuário dumbmonit. Sem ela, as
   notificações são ignoradas e nada mais muda.

    ```
    curl -u YOUR_USER -H 'X-Requested-By: cli' -H 'Content-Type: application/json' -X POST https://graylog.lan/api/roles -d '{"name":"DumbMonit Notifications","description":"Read system notifications","permissions":["notifications:read"],"read_only":false}'
    curl -u YOUR_USER -H 'X-Requested-By: cli' -X PUT 'https://graylog.lan/api/roles/DumbMonit%20Notifications/members/dumbmonit'
    ```

4. No DumbMonit, informe o endereço do nó Graylog, por exemplo "graylog.lan"
   (porta 9000) ou "https://graylog.lan" detrás de um proxy reverso, e cole
   o token. Em um cluster Graylog, adicione cada nó: journal, buffers e vazão
   são por nó.

5. O DumbMonit só lê. Ele nunca pesquisa mensagens, nunca inicia ou para uma
   entrada e nunca reconhece uma notificação.

!!! warning
    Não reutilize a conta com a qual você faz login: um token exposto carrega
    todos os direitos de seu usuário, enquanto o papel Reader acima não pode
    alterar nada. O Graylog serve HTTP simples a menos que você tenha
    configurado TLS, e o token viaja em cada requisição: em uma rede não
    confiável, coloque o Graylog detrás de HTTPS.

O que cada permissão do Reader cobre: `system:read` o status do nó e os nós
do cluster, `journal:read` o journal, `metrics:read` e `throughput:read` a
vazão e os buffers, `inputs:read` as entradas, `indexercluster:read` a saúde
do cluster de busca. O papel Reader também possui `messages:read`, que
permite ler mensagens apenas de streams explicitamente compartilhados com
ele: não compartilhe nenhum. Detalhes de falha de indexação
(`indices:failures`) não são necessários: o DumbMonit conta falhas a partir
das próprias métricas do Graylog.

## Credenciais

| Credencial | Campos |
|---|---|
| Token de acesso (recomendado) | Um token do usuário dedicado. O Graylog o espera como autenticação básica HTTP com o token como nome de usuário e a palavra `token` como senha; o DumbMonit faz isso por você. |
| Nome de usuário / senha | O próprio usuário dedicado. |

Endereço: um nome de host ou IP (`graylog.lan`), `host:porta`, ou uma URL
completa (`https://graylog.lan`); um `/api` final é aceito. Opções:
protocolo (HTTP por padrão), porta (9000), verificação de certificado e
tempo limite da requisição (10 s).
