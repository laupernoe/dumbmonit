# Redis e Valkey {#redis-and-valkey}

O próprio armazenamento em memória: memória contra `maxmemory`, conexões
recusadas, o link de replicação e o atraso de cada réplica, snapshots e
escritas de arquivo append-only com falha, e o tamanho do keyspace.

O DumbMonit abre uma conexão, envia `AUTH`, depois `INFO`, e a fecha. A conta
que ele usa pode executar `INFO` e `PING` e nada mais: ela não pode ler,
escrever ou nem mesmo listar uma chave. Redis 6 e posteriores, Valkey e os
forks que respondem `INFO` da mesma forma (KeyDB, Dragonfly) são cobertos.

As falhas que valem a pena observar são as silenciosas. No `maxmemory`, o
Redis ou remove chaves sem avisar ninguém ou recusa toda escrita com `OOM
command not allowed`, dependendo de `maxmemory-policy`; sem `maxmemory`, ele
cresce até o kernel matá-lo. Um snapshot com falha faz o Redis recusar
escritas enquanto `stop-writes-on-bgsave-error` estiver ativo, o que é o
padrão. Uma réplica cujo link com sua primária está fora do ar continua
respondendo leituras, com dados desatualizados.

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_redis_`. Os totais são
contadores: gráficos e regras os transformam em taxas.

| Métrica | O que é | Rótulos |
|---|---|---|
| `version_info` | valor 1 | `product` (`redis`, `valkey`), `version`, `mode` |
| `role_info` | valor 1 | `role`: `master` ou `replica` |
| `memory_used_bytes`, `memory_rss_bytes`, `memory_peak_bytes`, `memory_fragmentation_ratio` | memória como o Redis conta, e como o sistema conta | |
| `memory_max_bytes`, `memory_used_percent` | `maxmemory` e a parcela usada; ausente quando `maxmemory` é 0 (sem limite) | |
| `maxmemory_policy_info` | valor 1 | `policy` |
| `connected_clients`, `blocked_clients`, `max_clients` | | |
| `rejected_connections_total` | conexões recusadas porque `maxclients` foi atingido | |
| `evicted_keys_total`, `expired_keys_total` | chaves removidas em `maxmemory`, chaves expiradas | |
| `keyspace_hits_total`, `keyspace_misses_total` | buscas que encontraram sua chave, e as que não encontraram | |
| `commands_processed_total`, `connections_received_total`, `ops_per_second` | | |
| `error_replies_total`, `acl_denied_auth_total` | respostas de erro enviadas, e logins recusados (uma senha sendo adivinhada aparece aqui) | |
| `keys`, `keys_expiring` | chaves em todos os bancos de dados, e as com expiração; sempre presente, mesmo em zero | |
| `db_keys` | chaves por banco de dados | `db` |
| `rdb_last_save_ok` | 0 quando o último snapshot (`BGSAVE`) falhou | |
| `rdb_changes_since_last_save`, `rdb_last_save_timestamp_seconds` | | |
| `aof_enabled`, `aof_last_write_ok`, `aof_last_rewrite_ok` | arquivo append-only; os dois status só quando ele está ativo | |
| `loading` | 1 enquanto o Redis carrega seu conjunto de dados ao iniciar | |
| `connected_replicas` | réplicas conectadas a esta primária | |
| `replica_online`, `replica_lag_seconds`, `replica_lag_bytes` | visto da primária, por réplica: online ou não, segundos desde sua última confirmação, bytes de atraso | `replica` (`ip:porta`) |
| `master_link_up`, `master_last_io_seconds`, `master_link_down_seconds`, `master_sync_in_progress` | em uma réplica: seu link com a primária | |
| `uptime_seconds`, `cluster_enabled` | | |

As [regras integradas](../alerting/rules.md#databases-message-broker-and-security-engine)
que se aplicam: Redis memory near maxmemory, Redis refusing connections,
Redis replication link down, Redis replica lagging, Redis snapshot failed,
Redis append-only file write failed, além de Monitor unreachable.

## A página do monitor {#the-device-page}

O painel acima dos gráficos lê o que a sondagem armazenou; abrir a página
nunca consulta o Redis. Ele diz o que está errado em uma frase (replicação,
memória contra `maxmemory` e a política que se aplica no limite,
persistência, conexões recusadas), depois as chaves, memória, clientes,
comandos por segundo, taxa de acerto, chaves removidas nas últimas 24 horas
e tempo de atividade, depois as chaves por banco de dados e o atraso de cada
réplica.

## Criar um usuário Redis que só pode executar INFO {#create-a-redis-user-that-may-only-run-info}

1. No Redis 6 ou posterior, ou Valkey, crie um usuário que possa executar
   INFO e PING e nada mais, sem acesso a nenhuma chave. No redis-cli,
   conectado com um usuário autorizado a gerenciar ACLs:

    ```
    ACL SETUSER dumbmonit on >a-long-password -@all +info +ping
    ```

2. Torne-o permanente: ACL SAVE se o servidor carrega seus usuários de um
   aclfile, CONFIG REWRITE caso contrário.

    ```
    ACL SAVE
    ```

3. No Redis 5 ou mais antigo, ou com apenas requirepass definido, escolha
   Apenas senha abaixo. Essa senha carrega todos os direitos: prefira um
   usuário ACL sempre que o servidor suportar um.

4. No DumbMonit, informe o endereço do servidor, por exemplo "cache.lan" ou
   "cache.lan:6380". Se o servidor só aceita TLS nessa porta (tls-port),
   marque TLS abaixo.

5. O DumbMonit envia AUTH, depois INFO. Ele nunca lê, escreve ou lista uma
   chave.

!!! warning
    O Redis não dá nenhum aviso antes do maxmemory: no limite ele ou remove
    chaves silenciosamente ou recusa toda escrita, dependendo de
    maxmemory-policy. Sem maxmemory ele cresce até o kernel matá-lo. O
    DumbMonit avisa acima de 90% do maxmemory.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Usuário ACL (recomendado) | O usuário criado acima e sua senha, enviados como `AUTH user password`. |
| Apenas senha | O valor de `requirepass`, enviado como `AUTH password`. |
| Sem autenticação | Para um servidor sem senha alguma. |

Endereço: um nome de host ou IP (`cache.lan`), `host:porta`, ou uma string de
conexão (`redis://cache.lan:6380`; qualquer coisa após a porta é ignorada, e
credenciais escritas nela nunca são usadas). Opções: porta (6379), TLS
(desligado), verificação de certificado, e tempo limite para toda a
verificação (10 s). Com TLS ativado, o certificado é verificado contra as
autoridades públicas embutidas no DumbMonit; para uma autoridade privada,
marque Aceitar um certificado não verificável.

Em uma implantação Redis Sentinel ou Cluster, adicione cada servidor que você
quer observar: cada um reporta sua própria memória, clientes e persistência,
e uma primária reporta suas réplicas. O próprio Sentinel não é coberto.
