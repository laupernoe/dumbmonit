# MongoDB

The document database itself: the replica set (members, their state, a
missing primary, how far each secondary is behind), connections against the
limit, the WiredTiger cache, operations and assertions.

DumbMonit logs in with an account that holds the built-in `clusterMonitor`
role only, runs `serverStatus`, and `replSetGetStatus` when the server belongs
to a replica set. That role reads the state of the server and of the
replication, and no document of any database. MongoDB 3.6 and later are
covered; DumbMonit speaks the wire protocol itself, without a driver.

The failures worth watching: a replica set that lost its primary accepts no
write until an election succeeds; a secondary that falls behind is the one
that will be elected the day the primary dies, without the latest writes; at
the connection limit new clients are refused; with more than 20% of the
WiredTiger cache dirty, queries have to evict pages themselves and everything
slows down.

## What it watches

All metrics are prefixed `dumbmonit_mongodb_`. Totals are counters: charts and
rules turn them into rates.

| Metric | What | Labels |
|---|---|---|
| `version_info` | value 1 | `version`, `process`, `storage_engine` |
| `role_info` | value 1 | `role`: `primary`, `secondary`, `arbiter`, `standalone` |
| `uptime_seconds` | | |
| `connections_current`, `connections_available`, `connections_active` | | |
| `connections_used_percent` | current against current plus available | |
| `connections_created_total`, `connections_rejected_total` | | |
| `operations_total` | operations since start | `op`: `insert`, `query`, `update`, `delete`, `getmore`, `command` |
| `asserts_total` | assertions since start; `regular` are internal server errors, `user` errors returned to clients | `type` |
| `queued_operations`, `active_clients` | operations waiting for a lock, clients running one | |
| `wiredtiger_cache_max_bytes`, `wiredtiger_cache_used_bytes`, `wiredtiger_cache_used_percent` | the WiredTiger cache and how full it is | |
| `wiredtiger_cache_dirty_bytes`, `wiredtiger_cache_dirty_percent` | modified pages not yet written | |
| `wiredtiger_cache_read_bytes_total` | bytes read from disk into the cache | |
| `memory_resident_bytes` | | |
| `network_bytes_in_total`, `network_bytes_out_total` | | |
| `cursors_open`, `cursors_timed_out_total`, `documents_deleted_total` | | |
| `replset_info` | value 1, in a replica set | `set`, `members` |
| `replset_primary_present` | 1 when a member is primary, 0 when none is | |
| `replset_member_state` | the member's state number | `member`, `state`: `primary`, `secondary`, `arbiter`, `recovering`, `down`… |
| `replset_member_health` | 1 when the member is reachable, 0 when it is not | `member` |
| `replset_member_lag_seconds` | how far a secondary is behind the primary | `member` |

The [built-in rules](../alerting/rules.md#databases-message-broker-and-security-engine)
that apply: MongoDB replica set without primary, MongoDB member unreachable,
MongoDB replication lag, MongoDB connections near the limit, MongoDB cache
under pressure, MongoDB internal errors, plus Device unreachable.

## The device page

The panel above the charts reads what the probe stored; opening the page never
queries MongoDB. It says what is wrong in a sentence (replica set, connections,
cache, internal errors), then the connections, operations per second, cache
used, resident memory, queued operations and uptime, then the state and lag of
each replica set member and the operations per second by type.

## Create a MongoDB account with the clusterMonitor role

1. In mongosh, logged in with a user allowed to create users, create an account with the built-in clusterMonitor role only. It reads server and replication status, and no document of any database.

    ```
    use admin
    db.createUser({user: "dumbmonit", pwd: passwordPrompt(), roles: [{role: "clusterMonitor", db: "admin"}]})
    ```

2. DumbMonit logs in with SCRAM-SHA-256, the default for accounts created on MongoDB 4.0 and later. If you created the account in another database than admin, set Authentication database below.

3. In DumbMonit, enter the address of one mongod, for example "db1.lan" or "db1.lan:27018". In a replica set, add every member: each reports its own connections and cache, and any of them reports the state and lag of all members. If the server requires TLS, tick TLS below.

4. DumbMonit only runs serverStatus and replSetGetStatus. It never lists a collection and never reads a document.

!!! warning
    A mongodb+srv:// address is not resolved: enter the host of each member instead. MongoDB older than 3.6 and mongos routers are not supported.

## Credentials

| Credential | Fields |
|---|---|
| Username / password | The `clusterMonitor` account created above, logged in with SCRAM-SHA-256. |
| No authentication | For a server started without access control. |

An account that only has SCRAM-SHA-1 credentials (created before MongoDB 4.0
and never updated) is refused: update its password once with
`db.updateUser("dumbmonit", {pwd: passwordPrompt(), mechanisms: ["SCRAM-SHA-256"]})`.

Address: a host name or IP (`db1.lan`), `host:port`, or a `mongodb://`
connection string with a single host. Options: port (27017), authentication
database (`admin`), TLS (off), certificate check, and timeout for the whole
check (10 s).
