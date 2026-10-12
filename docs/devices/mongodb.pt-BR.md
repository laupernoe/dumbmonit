# MongoDB

O banco de dados de documentos em si: o replica set (membros, seu estado,
uma primary ausente, quão atrasada está cada secondary), conexões contra o
limite, o cache do WiredTiger, operações e assertions.

O DumbMonit faz login com uma conta que possui apenas o papel embutido
`clusterMonitor`, executa `serverStatus`, e `replSetGetStatus` quando o
servidor pertence a um replica set. Esse papel lê o estado do servidor e da
replicação, e nenhum documento de nenhum banco de dados. O MongoDB 3.6 e
posteriores são cobertos; o DumbMonit fala o próprio protocolo de rede, sem
um driver.

As falhas que valem a pena observar: um replica set que perdeu sua primary
não aceita nenhuma escrita até que uma eleição tenha sucesso; uma secondary
que fica atrasada é a que será eleita no dia em que a primary morrer, sem as
escritas mais recentes; no limite de conexões novos clientes são recusados;
com mais de 20% do cache do WiredTiger sujo, as consultas têm que despejar
páginas elas mesmas e tudo fica mais lento.

## O que ele observa

Todas as métricas têm o prefixo `dumbmonit_mongodb_`. Os totais são
contadores: gráficos e regras os transformam em taxas.

| Métrica | O que é | Rótulos |
|---|---|---|
| `version_info` | valor 1 | `version`, `process`, `storage_engine` |
| `role_info` | valor 1 | `role`: `primary`, `secondary`, `arbiter`, `standalone` |
| `uptime_seconds` | | |
| `connections_current`, `connections_available`, `connections_active` | | |
| `connections_used_percent` | atuais contra atuais mais disponíveis | |
| `connections_created_total`, `connections_rejected_total` | | |
| `operations_total` | operações desde o início | `op`: `insert`, `query`, `update`, `delete`, `getmore`, `command` |
| `asserts_total` | assertions desde o início; `regular` são erros internos do servidor, `user` erros retornados aos clientes | `type` |
| `queued_operations`, `active_clients` | operações esperando por um bloqueio, clientes executando uma | |
| `wiredtiger_cache_max_bytes`, `wiredtiger_cache_used_bytes`, `wiredtiger_cache_used_percent` | o cache do WiredTiger e quão cheio ele está | |
| `wiredtiger_cache_dirty_bytes`, `wiredtiger_cache_dirty_percent` | páginas modificadas ainda não gravadas | |
| `wiredtiger_cache_read_bytes_total` | bytes lidos do disco para o cache | |
| `memory_resident_bytes` | | |
| `network_bytes_in_total`, `network_bytes_out_total` | | |
| `cursors_open`, `cursors_timed_out_total`, `documents_deleted_total` | | |
| `replset_info` | valor 1, em um replica set | `set`, `members` |
| `replset_primary_present` | 1 quando um membro é primary, 0 quando nenhum é | |
| `replset_member_state` | o número de estado do membro | `member`, `state`: `primary`, `secondary`, `arbiter`, `recovering`, `down`… |
| `replset_member_health` | 1 quando o membro é alcançável, 0 quando não é | `member` |
| `replset_member_lag_seconds` | quão atrasada uma secondary está em relação à primary | `member` |

As [regras integradas](../alerting/rules.md#databases-message-broker-and-security-engine)
que se aplicam: MongoDB replica set without primary, MongoDB member
unreachable, MongoDB replication lag, MongoDB connections near the limit,
MongoDB cache under pressure, MongoDB internal errors, além de Device
unreachable.

## A página do dispositivo

O painel acima dos gráficos lê o que a sondagem armazenou; abrir a página
nunca consulta o MongoDB. Ele diz o que está errado em uma frase (replica
set, conexões, cache, erros internos), depois as conexões, operações por
segundo, cache usado, memória residente, operações na fila e tempo de
atividade, depois o estado e atraso de cada membro do replica set e as
operações por segundo por tipo.

## Criar uma conta MongoDB com o papel clusterMonitor

1. No mongosh, conectado com um usuário autorizado a criar usuários, crie uma
   conta apenas com o papel embutido clusterMonitor. Ela lê o status do
   servidor e da replicação, e nenhum documento de nenhum banco de dados.

    ```
    use admin
    db.createUser({user: "dumbmonit", pwd: passwordPrompt(), roles: [{role: "clusterMonitor", db: "admin"}]})
    ```

2. O DumbMonit faz login com SCRAM-SHA-256, o padrão para contas criadas no
   MongoDB 4.0 e posteriores. Se você criou a conta em outro banco de dados
   além do admin, defina Banco de dados de autenticação abaixo.

3. No DumbMonit, informe o endereço de um mongod, por exemplo "db1.lan" ou
   "db1.lan:27018". Em um replica set, adicione todos os membros: cada um
   reporta suas próprias conexões e cache, e qualquer um deles reporta o
   estado e atraso de todos os membros. Se o servidor exigir TLS, marque TLS
   abaixo.

4. O DumbMonit só executa serverStatus e replSetGetStatus. Ele nunca lista
   uma coleção e nunca lê um documento.

!!! warning
    Um endereço mongodb+srv:// não é resolvido: informe o host de cada
    membro em vez disso. MongoDB mais antigo que 3.6 e roteadores mongos não
    são suportados.

## Credenciais

| Credencial | Campos |
|---|---|
| Nome de usuário / senha | A conta `clusterMonitor` criada acima, conectando com SCRAM-SHA-256. |
| Sem autenticação | Para um servidor iniciado sem controle de acesso. |

Uma conta que só tem credenciais SCRAM-SHA-1 (criada antes do MongoDB 4.0 e
nunca atualizada) é recusada: atualize sua senha uma vez com
`db.updateUser("dumbmonit", {pwd: passwordPrompt(), mechanisms: ["SCRAM-SHA-256"]})`.

Endereço: um nome de host ou IP (`db1.lan`), `host:porta`, ou uma string de
conexão `mongodb://` com um único host. Opções: porta (27017), banco de dados
de autenticação (`admin`), TLS (desligado), verificação de certificado, e
tempo limite para toda a verificação (10 s).
