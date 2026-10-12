# MongoDB {#mongodb}

A própria base de dados de documentos: o replica set (membros, o seu estado,
um primário em falta, quanto cada secundário está atrasado), ligações contra
o limite, a cache do WiredTiger, operações e assertions.

O DumbMonit inicia sessão com uma conta que detém apenas a função
incorporada `clusterMonitor`, executa `serverStatus`, e `replSetGetStatus`
quando o servidor pertence a um replica set. Essa função lê o estado do
servidor e da replicação, e nenhum documento de nenhuma base de dados. O
MongoDB 3.6 e posteriores são cobertos; o DumbMonit fala o próprio protocolo
de rede, sem driver.

As falhas a vigiar: um replica set que perdeu o seu primário não aceita
nenhuma escrita até uma eleição ter sucesso; um secundário que fica atrasado
é o que será eleito no dia em que o primário morrer, sem as escritas mais
recentes; no limite de ligações, novos clientes são recusados; com mais de
20% da cache do WiredTiger sujos, as consultas têm de despejar páginas elas
próprias e tudo abranda.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_mongodb_`. Os totais são
contadores: os gráficos e as regras convertem-nos em taxas.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `version_info` | valor 1 | `version`, `process`, `storage_engine` |
| `role_info` | valor 1 | `role`: `primary`, `secondary`, `arbiter`, `standalone` |
| `uptime_seconds` | | |
| `connections_current`, `connections_available`, `connections_active` | | |
| `connections_used_percent` | atuais contra atuais mais disponíveis | |
| `connections_created_total`, `connections_rejected_total` | | |
| `operations_total` | operações desde o arranque | `op`: `insert`, `query`, `update`, `delete`, `getmore`, `command` |
| `asserts_total` | assertions desde o arranque; `regular` são erros internos do servidor, `user` erros devolvidos aos clientes | `type` |
| `queued_operations`, `active_clients` | operações à espera de um bloqueio, clientes a executar uma | |
| `wiredtiger_cache_max_bytes`, `wiredtiger_cache_used_bytes`, `wiredtiger_cache_used_percent` | a cache do WiredTiger e quão cheia está | |
| `wiredtiger_cache_dirty_bytes`, `wiredtiger_cache_dirty_percent` | páginas modificadas ainda não escritas | |
| `wiredtiger_cache_read_bytes_total` | bytes lidos do disco para a cache | |
| `memory_resident_bytes` | | |
| `network_bytes_in_total`, `network_bytes_out_total` | | |
| `cursors_open`, `cursors_timed_out_total`, `documents_deleted_total` | | |
| `replset_info` | valor 1, num replica set | `set`, `members` |
| `replset_primary_present` | 1 quando um membro é primário, 0 quando nenhum é | |
| `replset_member_state` | o número de estado do membro | `member`, `state`: `primary`, `secondary`, `arbiter`, `recovering`, `down`… |
| `replset_member_health` | 1 quando o membro é alcançável, 0 quando não é | `member` |
| `replset_member_lag_seconds` | quanto um secundário está atrasado em relação ao primário | `member` |

As [regras incorporadas](../alerting/rules.md#databases-message-broker-and-security-engine)
que se aplicam: MongoDB replica set without primary, MongoDB member
unreachable, MongoDB replication lag, MongoDB connections near the limit,
MongoDB cache under pressure, MongoDB internal errors, mais Device
unreachable.

## A página do dispositivo {#the-device-page}

O painel acima dos gráficos lê o que a verificação guardou; abrir a página
nunca consulta o MongoDB. Diz o que está mal numa frase (replica set,
ligações, cache, erros internos), depois as ligações, operações por segundo,
cache usada, memória residente, operações em fila e tempo de atividade,
depois o estado e o atraso de cada membro do replica set e as operações por
segundo por tipo.

## Criar uma conta MongoDB com a função clusterMonitor {#create-a-mongodb-account-with-the-clustermonitor-role}

1. No mongosh, com sessão iniciada como um utilizador autorizado a criar utilizadores, crie uma conta apenas com a função incorporada clusterMonitor. Lê o estado do servidor e da replicação, e nenhum documento de nenhuma base de dados.

    ```
    use admin
    db.createUser({user: "dumbmonit", pwd: passwordPrompt(), roles: [{role: "clusterMonitor", db: "admin"}]})
    ```

2. O DumbMonit inicia sessão com SCRAM-SHA-256, a predefinição para contas criadas no MongoDB 4.0 e posteriores. Se criou a conta noutra base de dados além de admin, defina Authentication database abaixo.

3. No DumbMonit, introduza o endereço de um mongod, por exemplo «db1.lan» ou «db1.lan:27018». Num replica set, adicione todos os membros: cada um reporta as suas próprias ligações e cache, e qualquer um deles reporta o estado e o atraso de todos os membros. Se o servidor exigir TLS, marque TLS abaixo.

4. O DumbMonit só executa serverStatus e replSetGetStatus. Nunca lista uma coleção e nunca lê um documento.

!!! warning
    Um endereço mongodb+srv:// não é resolvido: introduza o anfitrião de cada membro em vez disso. O MongoDB anterior a 3.6 e os routers mongos não são suportados.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Nome de utilizador / senha | A conta `clusterMonitor` criada acima, com sessão iniciada via SCRAM-SHA-256. |
| Sem autenticação | Para um servidor iniciado sem controlo de acesso. |

Uma conta que só tem credenciais SCRAM-SHA-1 (criada antes do MongoDB 4.0 e
nunca atualizada) é recusada: atualize a sua senha uma vez com
`db.updateUser("dumbmonit", {pwd: passwordPrompt(), mechanisms: ["SCRAM-SHA-256"]})`.

Endereço: um nome de anfitrião ou IP (`db1.lan`), `anfitrião:porta`, ou uma
cadeia de ligação `mongodb://` com um único anfitrião. Opções: porta (27017),
base de dados de autenticação (`admin`), TLS (desligado), verificação de
certificado, e tempo limite para toda a verificação (10 s).
