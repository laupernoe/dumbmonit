# Elasticsearch / OpenSearch

Elasticsearch and OpenSearch: cluster status (red or yellow), unassigned shards, disks filling up node by node, and JVM heap pressure.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 9200 | TCP | REST API (HTTP or HTTPS, default) |

DumbMonit reads the REST API with a user that only holds the cluster privilege `monitor`: it sees the health and the statistics of the nodes, and no document of any index. Both engines speak the same dialect for this, so one device type covers them.

A `red` cluster has at least one primary shard that is not allocated: part of the data cannot be read or written. A `yellow` one has lost replicas: the next node failure will lose data. The quiet failure is the disk: at 95 % (flood stage) the engine turns the indices read-only and writes are refused.

## What it watches

All metrics are prefixed `dumbmonit_elasticsearch_`.

| Metric | What | Labels |
|---|---|---|
| `version_info` | value 1 | `version`, `cluster`, `distribution` |
| `cluster_status` | 0 green, 1 yellow, 2 red |  |
| `nodes`, `data_nodes` | nodes in the cluster |  |
| `active_primary_shards`, `active_shards`, `active_shards_percent` | allocated shards |  |
| `unassigned_shards`, `delayed_unassigned_shards`, `initializing_shards`, `relocating_shards` | shards that are not in service yet |  |
| `pending_tasks` | cluster state tasks waiting |  |
| `node_disk_used_percent`, `node_disk_free_bytes`, `node_disk_total_bytes` | the data disk of each node | `node` |
| `node_heap_used_percent`, `node_cpu_percent` | JVM heap and CPU of each node | `node` |

The node statistics are optional: a user that may not read them keeps the cluster health, without the per-node series.

The [built-in rules](../alerting/rules.md#more-databases-and-kafka)
that apply: Elasticsearch cluster red, Elasticsearch cluster yellow, Elasticsearch unassigned shards, Elasticsearch node disk almost full, Elasticsearch JVM heap pressure, plus Monitor unreachable.

## Create a monitoring-only Elasticsearch user

1. Create a role that only holds the cluster privilege monitor, from Kibana (Stack Management, Roles) or with the API.

    ```
    curl -u elastic -X PUT "https://es.lan:9200/_security/role/dumbmonit" -H "Content-Type: application/json" -d '{"cluster":["monitor"]}'
    ```

2. Create the user with that role.

    ```
    curl -u elastic -X PUT "https://es.lan:9200/_security/user/dumbmonit" -H "Content-Type: application/json" -d '{"password":"choose-a-long-password","roles":["dumbmonit"]}'
    ```

3. OpenSearch: create a role with the cluster permission cluster_monitor in Security, Roles, and map the user to it.

4. In DumbMonit, enter the address of one node, for example es.lan, then the user and its password.

5. Elasticsearch 8 and later serve HTTPS with a generated certificate: pick HTTPS, and tick Accept an unverifiable certificate unless you installed your own.

### Required permissions

- **monitor (cluster privilege)**: Read the cluster health and node statistics
- **cluster_monitor (OpenSearch)**: The same permission on OpenSearch

!!! warning
    The account needs no index privilege: DumbMonit never reads a document. One node reports the whole cluster, so enter a node that is not the one you expect to fail.

### Why these permissions

`monitor` is the cluster privilege that covers `/_cluster/health` and `/_nodes/stats`. The user has no index privilege on purpose. The status is a number so that a rule can compare it: red is above 1, yellow equals 1.

## Credentials

| Credential | Fields |
|---|---|
| Username / password | The user created above, sent as HTTP basic authentication. |
| Bearer token | A service account token, sent as a bearer token. Elasticsearch API keys use another scheme and are not supported. |
| No authentication | A cluster with security switched off. |

Address: a host name or IP (`es.lan`), `host:port`, or a full URL. Options: protocol (HTTP by default), port (9200), certificate check, request timeout (10 s).
