# MongoDB {#mongodb}

A própria base de dados de documentos: o replica set (membros, o seu estado,
um primário em falta, quanto cada secundário está atrasado), ligações contra o
limite, a cache do WiredTiger, operações e asserções.

O DumbMonit inicia sessão com uma conta que tem apenas o papel incorporado
`clusterMonitor`, executa `serverStatus`, e `replSetGetStatus` quando o
servidor pertence a um replica set. Esse papel lê o estado do servidor e da
replicação, e nenhum documento de nenhuma base de dados. O MongoDB 3.6 e
posteriores são cobertos; o DumbMonit fala ele próprio o protocolo de rede,
sem driver.

As falhas a vigiar: um replica set que perdeu o primário não aceita nenhuma
escrita até uma eleição ter êxito; um secundário que fica para trás é o que
será eleito no dia em que o primário morrer, sem as últimas escritas; no
limite de ligações os novos clientes são recusados; com mais de 20% da cache do
WiredTiger suja, as consultas têm de evacuar páginas elas próprias e tudo
abranda.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_mongodb_`. Os totais são contadores:
os gráficos e as regras convertem-nos em taxas.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `version_info` | valor 1 | `version`, `process`, `storage_engine` |
| `role_info` | valor 1 | `role`: `primary`, `secondary`, `arbiter`, `standalone` |
| `uptime_seconds` | | |
| `connections_current`, `connections_available`, `connections_active` | | |
| `connections_used_percent` | atual contra atual mais disponível | |
| `connections_created_total`, `connections_rejected_total` | | |
| `operations_total` | operações desde o arranque | `op`: `insert`, `query`, `update`, `delete`, `getmore`, `command` |
| `asserts_total` | asserções desde o arranque; `type` é `regular` (erros internos do servidor), `warning`, `msg`, `user` (erros devolvidos aos clientes), `tripwire` ou `rollovers`. `regular` e `user` são os que merecem um gráfico | `type` |
| `queued_operations`, `active_clients` | operações à espera de um bloqueio, clientes a executar uma | |
| `wiredtiger_cache_max_bytes`, `wiredtiger_cache_used_bytes`, `wiredtiger_cache_used_percent` | a cache do WiredTiger e o quão cheia está | |
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
que se aplicam: MongoDB replica set without primary, MongoDB member unreachable,
MongoDB replication lag, MongoDB connections near the limit, MongoDB cache
under pressure, MongoDB internal errors, mais Device unreachable.

## A página do dispositivo {#the-device-page}

O painel acima dos gráficos lê o que a sonda guardou; abrir a página nunca
consulta o MongoDB. Diz numa frase o que está errado (replica set, ligações,
cache, erros internos), depois as ligações, as operações por segundo, a cache
usada, a memória residente, as operações em fila e o tempo de atividade, depois
o estado e o atraso de cada membro do replica set e as operações por segundo
por tipo.

## Criar uma conta MongoDB com o papel clusterMonitor {#create-a-mongodb-account-with-the-clustermonitor-role}

1. No mongosh, com sessão iniciada num utilizador autorizado a criar utilizadores, crie uma conta apenas com o papel incorporado clusterMonitor. Lê o estado do servidor e da replicação, e nenhum documento de nenhuma base de dados.

    ```
    use admin
    db.createUser({user: "dumbmonit", pwd: passwordPrompt(), roles: [{role: "clusterMonitor", db: "admin"}]})
    ```

2. O DumbMonit inicia sessão com SCRAM-SHA-256, a predefinição para contas criadas no MongoDB 4.0 e posteriores. Se criou a conta noutra base de dados que não admin, defina a Base de dados de autenticação abaixo.

3. No DumbMonit, introduza o endereço de um mongod, por exemplo «db1.lan» ou «db1.lan:27018». Num replica set, adicione todos os membros: cada um reporta as suas próprias ligações e cache, e qualquer um deles reporta o estado e o atraso de todos os membros. Se o servidor exige TLS, marque TLS abaixo.

4. O DumbMonit só executa serverStatus e replSetGetStatus. Nunca lista uma coleção e nunca lê um documento.

!!! warning
    Um endereço mongodb+srv:// não é resolvido: introduza antes o anfitrião de cada membro. O MongoDB anterior à 3.6 e os routers mongos não são suportados.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Nome de utilizador / palavra-passe | A conta `clusterMonitor` criada acima, com sessão iniciada em SCRAM-SHA-256. |
| Sem autenticação | Para um servidor iniciado sem controlo de acesso. |

Uma conta que só tem credenciais SCRAM-SHA-1 (criada antes do MongoDB 4.0 e
nunca atualizada) é recusada: atualize a sua palavra-passe uma vez com
`db.updateUser("dumbmonit", {pwd: passwordPrompt(), mechanisms: ["SCRAM-SHA-256"]})`.

Endereço: um nome de anfitrião ou IP (`db1.lan`), `host:port`, ou uma cadeia de
ligação `mongodb://` com um único anfitrião. Opções: porta (27017), base de
dados de autenticação (`admin`), TLS (desativado), verificação do certificado e
tempo limite para toda a verificação (10 s).
