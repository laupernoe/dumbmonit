# Redis and Valkey

The in-memory store itself: memory against `maxmemory`, refused connections,
the replication link and the lag of each replica, failed snapshots and
append-only file writes, and the size of the keyspace.

DumbMonit opens one connection, sends `AUTH`, then `INFO`, and closes it. The
account it uses can run `INFO` and `PING` and nothing else: it cannot read,
write or even list a key. Redis 6 and later, Valkey and the forks that answer
`INFO` the same way (KeyDB, Dragonfly) are covered.

The failures worth watching are the silent ones. At `maxmemory`, Redis either
evicts keys without telling anyone or refuses every write with `OOM command
not allowed`, depending on `maxmemory-policy`; without `maxmemory`, it grows
until the kernel kills it. A failed snapshot makes Redis refuse writes as long
as `stop-writes-on-bgsave-error` is on, which it is by default. A replica
whose link to its primary is down keeps answering reads, with stale data.

## What it watches

All metrics are prefixed `dumbmonit_redis_`. Totals are counters: charts and
rules turn them into rates.

| Metric | What | Labels |
|---|---|---|
| `version_info` | value 1 | `product` (`redis`, `valkey`), `version`, `mode` |
| `role_info` | value 1 | `role`: `master` or `replica` |
| `memory_used_bytes`, `memory_rss_bytes`, `memory_peak_bytes`, `memory_fragmentation_ratio` | memory as Redis counts it, and as the system does | |
| `memory_max_bytes`, `memory_used_percent` | `maxmemory` and the share used; absent when `maxmemory` is 0 (no limit) | |
| `maxmemory_policy_info` | value 1 | `policy` |
| `connected_clients`, `blocked_clients`, `max_clients` | | |
| `rejected_connections_total` | connections refused because `maxclients` was reached | |
| `evicted_keys_total`, `expired_keys_total` | keys evicted at `maxmemory`, keys expired | |
| `keyspace_hits_total`, `keyspace_misses_total` | lookups that found their key, and those that did not | |
| `commands_processed_total`, `connections_received_total`, `ops_per_second` | | |
| `error_replies_total`, `acl_denied_auth_total` | error replies sent, and logins refused (a password being guessed shows here) | |
| `keys`, `keys_expiring` | keys in all databases, and those with an expiry; always present, even at zero | |
| `db_keys` | keys per database | `db` |
| `rdb_last_save_ok` | 0 when the last snapshot (`BGSAVE`) failed | |
| `rdb_changes_since_last_save`, `rdb_last_save_timestamp_seconds` | | |
| `aof_enabled`, `aof_last_write_ok`, `aof_last_rewrite_ok` | append-only file; the two statuses only when it is on | |
| `loading` | 1 while Redis loads its dataset at start | |
| `connected_replicas` | replicas attached to this primary | |
| `replica_online`, `replica_lag_seconds`, `replica_lag_bytes` | seen from the primary, per replica: online or not, seconds since it last acknowledged, bytes behind | `replica` (`ip:port`) |
| `master_link_up`, `master_last_io_seconds`, `master_link_down_seconds`, `master_sync_in_progress` | on a replica: its link to the primary | |
| `uptime_seconds`, `cluster_enabled` | | |

The [built-in rules](../alerting/rules.md#databases-message-broker-and-security-engine)
that apply: Redis memory near maxmemory, Redis refusing connections, Redis
replication link down, Redis replica lagging, Redis snapshot failed, Redis
append-only file write failed, plus Device unreachable.

## The device page

The panel above the charts reads what the probe stored; opening the page never
queries Redis. It says what is wrong in a sentence (replication, memory
against `maxmemory` and the policy that applies at the limit, persistence,
refused connections), then the keys, memory, clients, commands per second,
hit ratio, keys evicted in the last 24 hours and uptime, then the keys per
database and the lag of each replica.

## Create a Redis user that may only run INFO

1. On Redis 6 or later, or Valkey, create a user that may run INFO and PING and nothing else, with no access to any key. In redis-cli, logged in with a user allowed to manage ACLs:

    ```
    ACL SETUSER dumbmonit on >a-long-password -@all +info +ping
    ```

2. Make it permanent: ACL SAVE if the server loads its users from an aclfile, CONFIG REWRITE otherwise.

    ```
    ACL SAVE
    ```

3. On Redis 5 or older, or with only requirepass set, pick Password only below. That password carries every right: prefer an ACL user whenever the server supports one.

4. In DumbMonit, enter the server address, for example "cache.lan" or "cache.lan:6380". If the server only accepts TLS on that port (tls-port), tick TLS below.

5. DumbMonit sends AUTH, then INFO. It never reads, writes or lists a key.

!!! warning
    Redis gives no warning before maxmemory: at the limit it either evicts keys silently or refuses every write, depending on maxmemory-policy. Without maxmemory it grows until the kernel kills it. DumbMonit warns above 90% of maxmemory.

## Credentials

| Credential | Fields |
|---|---|
| ACL user (recommended) | The user created above and its password, sent as `AUTH user password`. |
| Password only | The `requirepass` value, sent as `AUTH password`. |
| No authentication | For a server with no password at all. |

Address: a host name or IP (`cache.lan`), `host:port`, or a connection string
(`redis://cache.lan:6380`; anything after the port is ignored, and
credentials written in it are never used). Options: port (6379), TLS (off),
certificate check, and timeout for the whole check (10 s). With TLS on, the
certificate is checked against the public authorities built into DumbMonit;
for a private authority, tick Accept an unverifiable certificate.

In a Redis Sentinel or Cluster deployment, add each server you want to watch:
each reports its own memory, clients and persistence, and a primary reports
its replicas. Sentinel itself is not covered.
