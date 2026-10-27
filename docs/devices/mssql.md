# SQL Server

The database engine: databases that are not online, requests blocked behind a forgotten transaction, sessions against the limit, the free space of the data volumes, SQL Server Agent jobs whose last run failed, and AlwaysOn availability groups.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 1433 | TCP | SQL Server (TDS, default port) |

DumbMonit logs in with a SQL Server login that only holds server-wide read rights: it sees the engine's state, and cannot read a table, write or change anything. Windows authentication is not supported.

The failure worth watching is the silent one: a transaction left open blocks every session that needs the same rows, and nothing is logged on the server. The others are a volume that is full when the files try to grow, a backup job that has failed for a week, and an availability replica that stopped synchronizing.

## What it watches

All metrics are prefixed `dumbmonit_mssql_`.

| Metric | What | Labels |
|---|---|---|
| `version_info` | value 1 | `version`, `edition` |
| `uptime_seconds` | since the service started |  |
| `sessions`, `sessions_used_percent` | user sessions, and the share of the connection limit when one is set |  |
| `blocked_requests`, `longest_block_seconds` | requests waiting on a lock, and the longest wait |  |
| `databases`, `databases_offline` | databases, and those in SUSPECT, RECOVERY_PENDING, OFFLINE or EMERGENCY |  |
| `database_online`, `database_read_only` | per database; 1 when online (or in a normal transient state) | `database`, `state` |
| `database_size_bytes` | size of the data files | `database` |
| `volume_free_bytes`, `volume_used_percent` | space of each volume holding database files | `volume` |
| `agent_job_last_ok`, `agent_jobs_failed` | 1 when the last run of an enabled Agent job did not fail; only with SQL Server Agent | `job` |
| `alwayson_sync_health`, `alwayson_suspended`, `alwayson_unhealthy` | synchronization health per replica and database (0 not healthy, 1 partial, 2 healthy); only when AlwaysOn is enabled | `replica`, `database` |

The Agent and AlwaysOn queries are optional: an Express edition or an instance without availability groups simply has no such series.

The [built-in rules](../alerting/rules.md#more-databases-and-kafka)
that apply: SQL Server database unavailable, SQL Server blocked requests, SQL Server volume almost full, SQL Server Agent job failed, SQL Server AlwaysOn replica unhealthy, SQL Server connections near the limit, plus Monitor unreachable.

## Create a read-only SQL Server login

1. Connect to the instance with SQL Server Management Studio or sqlcmd, with a login allowed to create logins, and create the login DumbMonit will use.

    ```
    CREATE LOGIN dumbmonit WITH PASSWORD = 'choose-a-long-password', CHECK_POLICY = ON;
    ```

2. Give it the server-wide read rights it needs to see the state of the engine, and nothing else.

    ```
    GRANT VIEW SERVER STATE TO dumbmonit;
    GRANT VIEW ANY DEFINITION TO dumbmonit;
    ```

3. Optional, for the SQL Server Agent jobs: let it read the job history in msdb.

    ```
    USE msdb;
    CREATE USER dumbmonit FOR LOGIN dumbmonit;
    ALTER ROLE SQLAgentReaderRole ADD MEMBER dumbmonit;
    ```

4. Check that the instance accepts SQL Server authentication (mixed mode) and listens on a fixed TCP port: SQL Server Configuration Manager, Network Configuration, TCP/IP.

5. In DumbMonit, enter the server address, for example sql.lan, or sql.lan:1444 for another port, then the login and its password.

6. Tick Accept an unverifiable certificate while the server still presents its self-signed certificate.

### Required permissions

- **VIEW SERVER STATE**: Read sessions, requests and volumes
- **VIEW ANY DEFINITION**: List the databases and their files
- **SQLAgentReaderRole (msdb)**: Optional: read the Agent job history

!!! warning
    Windows authentication is not supported: use a SQL Server login, on an instance that allows SQL Server authentication (mixed mode). Named instances need their fixed TCP port in the address.

### Why these permissions

The first two permissions are server-level and give access to dynamic management views and metadata only. A database in RESTORING is not counted as unavailable, since that is the normal state of a log shipping secondary. The login never needs a database user, except in `msdb` for the Agent job history.

## Credentials

| Credential | Fields |
|---|---|
| SQL Server login | The login created above (user name and password), sent in the TDS login packet over TLS. |

Address: a host name or IP (`sql.lan`), `host:port`, or a connection string pasted as `mssql://sql.lan:1444`. Options: port (1433), database (`master`), TLS (on), certificate check, request timeout (10 s).
