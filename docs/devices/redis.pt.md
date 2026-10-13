# Redis e Valkey {#redis-and-valkey}

O próprio armazenamento em memória: memória contra `maxmemory`, ligações
recusadas, a ligação de replicação e o atraso de cada réplica, snapshots e
escritas de append-only file falhadas, e o tamanho do keyspace.

O DumbMonit abre uma ligação, envia `AUTH`, depois `INFO`, e fecha-a. A conta
que usa pode executar `INFO` e `PING` e nada mais: não pode ler, escrever nem
sequer listar uma chave. O Redis 6 e posteriores, o Valkey e os forks que
respondem a `INFO` da mesma forma (KeyDB, Dragonfly) são cobertos.

As falhas a vigiar são as silenciosas. No `maxmemory`, o Redis ou despeja
chaves sem avisar ninguém ou recusa todas as escritas com `OOM command not
allowed`, consoante a `maxmemory-policy`; sem `maxmemory`, cresce até o kernel
o matar. Um snapshot falhado faz o Redis recusar escritas enquanto
`stop-writes-on-bgsave-error` estiver ativo, o que acontece por predefinição.
Uma réplica cuja ligação ao primário está em baixo continua a responder a
leituras, com dados desatualizados.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_redis_`. Os totais são contadores:
os gráficos e as regras convertem-nos em taxas.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `version_info` | valor 1 | `product` (`redis`, `valkey`), `version`, `mode` |
| `role_info` | valor 1 | `role`: `master` ou `replica` |
| `memory_used_bytes`, `memory_rss_bytes`, `memory_peak_bytes`, `memory_fragmentation_ratio` | memória como o Redis a conta, e como o sistema a conta | |
| `memory_max_bytes`, `memory_used_percent` | `maxmemory` e a parte usada; ausente quando `maxmemory` é 0 (sem limite) | |
| `maxmemory_policy_info` | valor 1 | `policy` |
| `connected_clients`, `blocked_clients`, `max_clients` | | |
| `rejected_connections_total` | ligações recusadas porque `maxclients` foi atingido | |
| `evicted_keys_total`, `expired_keys_total` | chaves despejadas no `maxmemory`, chaves expiradas | |
| `keyspace_hits_total`, `keyspace_misses_total` | pesquisas que encontraram a chave, e as que não | |
| `commands_processed_total`, `connections_received_total`, `ops_per_second` | | |
| `error_replies_total`, `acl_denied_auth_total` | respostas de erro enviadas, e inícios de sessão recusados (uma palavra-passe a ser adivinhada aparece aqui) | |
| `keys`, `keys_expiring` | chaves em todas as bases de dados, e as que têm expiração; sempre presentes, mesmo a zero | |
| `db_keys` | chaves por base de dados | `db` |
| `rdb_last_save_ok` | 0 quando o último snapshot (`BGSAVE`) falhou | |
| `rdb_changes_since_last_save`, `rdb_last_save_timestamp_seconds` | | |
| `aof_enabled`, `aof_last_write_ok`, `aof_last_rewrite_ok` | append-only file; os dois estados só quando está ativado | |
| `loading` | 1 enquanto o Redis carrega o seu conjunto de dados no arranque | |
| `connected_replicas` | réplicas ligadas a este primário | |
| `replica_online`, `replica_lag_seconds`, `replica_lag_bytes` | vistos do primário, por réplica: online ou não, segundos desde a última confirmação, bytes em atraso | `replica` (`ip:port`) |
| `master_link_up`, `master_last_io_seconds`, `master_link_down_seconds`, `master_sync_in_progress` | numa réplica: a sua ligação ao primário | |
| `uptime_seconds`, `cluster_enabled` | | |

As [regras incorporadas](../alerting/rules.md#databases-message-broker-and-security-engine)
que se aplicam: Redis memory near maxmemory, Redis refusing connections, Redis
replication link down, Redis replica lagging, Redis snapshot failed, Redis
append-only file write failed, mais Device unreachable.

## A página do dispositivo {#the-device-page}

O painel acima dos gráficos lê o que a sonda guardou; abrir a página nunca
consulta o Redis. Diz numa frase o que está errado (replicação, memória contra
`maxmemory` e a política que se aplica no limite, persistência, ligações
recusadas), depois as chaves, a memória, os clientes, os comandos por segundo,
a taxa de acerto, as chaves despejadas nas últimas 24 horas e o tempo de
atividade, depois as chaves por base de dados e o atraso de cada réplica.

## Criar um utilizador Redis que só pode executar INFO {#create-a-redis-user-that-may-only-run-info}

1. No Redis 6 ou posterior, ou no Valkey, crie um utilizador que possa executar INFO e PING e nada mais, sem acesso a nenhuma chave. No redis-cli, com sessão iniciada num utilizador autorizado a gerir ACLs:

    ```
    ACL SETUSER dumbmonit on >a-long-password -@all +info +ping
    ```

2. Torne-o permanente: ACL SAVE se o servidor carrega os seus utilizadores de um aclfile, CONFIG REWRITE caso contrário.

    ```
    ACL SAVE
    ```

3. No Redis 5 ou anterior, ou com apenas requirepass definido, escolha Só palavra-passe abaixo. Essa palavra-passe tem todos os direitos: prefira um utilizador ACL sempre que o servidor o suportar.

4. No DumbMonit, introduza o endereço do servidor, por exemplo «cache.lan» ou «cache.lan:6380». Se o servidor só aceita TLS nessa porta (tls-port), marque TLS abaixo.

5. O DumbMonit envia AUTH, depois INFO. Nunca lê, escreve nem lista uma chave.

!!! warning
    O Redis não dá aviso antes do maxmemory: no limite ou despeja chaves em silêncio ou recusa todas as escritas, consoante a maxmemory-policy. Sem maxmemory cresce até o kernel o matar. O DumbMonit avisa acima de 90% do maxmemory.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Utilizador ACL (recomendado) | O utilizador criado acima e a sua palavra-passe, enviados como `AUTH user password`. |
| Só palavra-passe | O valor de `requirepass`, enviado como `AUTH password`. |
| Sem autenticação | Para um servidor sem qualquer palavra-passe. |

Endereço: um nome de anfitrião ou IP (`cache.lan`), `host:port`, ou uma cadeia
de ligação (`redis://cache.lan:6380`; tudo o que vem depois da porta é ignorado,
e as credenciais nela escritas nunca são usadas). Opções: porta (6379), TLS
(desativado), verificação do certificado, e tempo limite para toda a
verificação (10 s). Com TLS ativado, o certificado é verificado contra as
autoridades públicas incorporadas no DumbMonit; para uma autoridade privada,
marque Aceitar um certificado não verificável.

Uma palavra-passe recusada (`WRONGPASS`, `NOAUTH`, `NOPERM`) é mostrada como
erro de credencial e ninguém é notificado sobre o erro em si, mas o DumbMonit
não recolhe então nada: a regra Device unreachable continua a disparar,
como para qualquer dispositivo que não consegue ler, até a credencial voltar a
funcionar. Um Redis que reinicia sem o seu utilizador ACL (utilizadores
definidos com `ACL SETUSER` mas nunca guardados) causa exatamente isto.

Numa implementação Redis Sentinel ou Cluster, adicione cada servidor que quer
vigiar: cada um reporta a sua própria memória, clientes e persistência, e um
primário reporta as suas réplicas. O próprio Sentinel não é coberto.
