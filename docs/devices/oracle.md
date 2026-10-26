# Oracle Database

Oracle Database, read through Oracle REST Data Services: an instance that is not open, tablespaces filling up, sessions and processes against their limits, Data Guard lag, the fast recovery area and scheduler jobs that failed.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 8080 | TCP | ORDS (HTTP, default) |
| 8443 | TCP | Only if ORDS is served over TLS (HTTPS, default) |

**Why ORDS.** Oracle's native protocol (TNS) has no free implementation in pure Rust, and the usual drivers go through OCI, a proprietary C library (Instant Client) that cannot ship in DumbMonit's static image and whose licence forbids redistributing it. ORDS is Oracle's own free component that exposes the database over HTTP: with its REST-enabled SQL, a `SELECT` is posted as JSON to `/ords/<schema>/_/sql`, which is what DumbMonit does with the account's own credentials. The limit is that ORDS must be deployed in front of the database. Where it is not, a script run by the DumbMonit agent (sqlplus) is the alternative; it is not provided here.

The account only holds `CREATE SESSION` and `SELECT_CATALOG_ROLE`: it reads the `V$` and `DBA_` views, and no data.

## What it watches

All metrics are prefixed `dumbmonit_oracle_`.

| Metric | What | Labels |
|---|---|---|
| `version_info` | value 1 | `version`, `status`, `database_status` |
| `instance_open` | 1 when the instance status is OPEN |  |
| `uptime_seconds` | since the instance started |  |
| `database_info` | value 1 | `open_mode`, `role`, `log_mode` |
| `tablespace_used_percent` | use of each tablespace, against its maximum size | `tablespace` |
| `sessions`, `sessions_used_percent` | sessions, and their share of the `sessions` limit |  |
| `processes`, `processes_used_percent` | processes, and their share of the `processes` limit |  |
| `dataguard_transport_lag_seconds`, `dataguard_apply_lag_seconds` | Data Guard lag; only on a standby |  |
| `recovery_area_used_percent` | use of the fast recovery area; only when one is configured |  |
| `scheduler_jobs_failed_24h` | scheduler job runs that failed in the last 24 hours |  |

Every query after the first is optional: a view the account cannot read leaves its metrics out without failing the check. A percentage is not written for a limit that is `UNLIMITED`.

The [built-in rules](../alerting/rules.md#more-databases-and-kafka)
that apply: Oracle instance not open, Oracle tablespace almost full, Oracle sessions near the limit, Oracle Data Guard apply lag, Oracle recovery area almost full, Oracle scheduler job failed, plus Monitor unreachable.

## Let DumbMonit read Oracle through ORDS

1. Make sure Oracle REST Data Services (ORDS) runs in front of the database with REST-enabled SQL switched on.

    ```
    ords config set restEnabledSql.active true
    ```

2. As a privileged user, create the account DumbMonit will use, with the two rights it needs.

    ```
    CREATE USER dumbmonit IDENTIFIED BY "choose-a-long-password";
    GRANT CREATE SESSION, SELECT_CATALOG_ROLE TO dumbmonit;
    ```

3. Connected as that account, enable its schema for REST with the alias dumbmonit.

    ```
    EXEC ORDS.ENABLE_SCHEMA(p_enabled => TRUE, p_url_mapping_type => 'BASE_PATH', p_url_mapping_pattern => 'dumbmonit');
    ```

4. In DumbMonit, enter the ORDS address, for example ords.lan:8080 or https://ords.lan:8443, then the account and its password.

5. If the schema alias or the ORDS path differ from the defaults, set them in the options below.

### Required permissions

- **CREATE SESSION**: Let the account log in
- **SELECT_CATALOG_ROLE**: Read the V$ and DBA_ views, no data

!!! warning
    There is no native Oracle client: DumbMonit reads the database through ORDS over HTTP, and never contacts the listener on port 1521. The REST-enabled SQL settings vary between ORDS versions: check the ORDS documentation of yours.

### Why these permissions

`SELECT_CATALOG_ROLE` gives read access to the data dictionary and performance views, never to application tables. REST-enabled SQL authenticates with the database account, so the account's schema has to be enabled for REST; the alias in the URL is the schema alias (`dumbmonit` by default, the account name in lower case).

## Credentials

| Credential | Fields |
|---|---|
| Username / password | The database account created above, sent as HTTP basic authentication to ORDS. |

Address: the ORDS host (`ords.lan`), `host:port`, or a full URL. Options: protocol (HTTP by default), port (8080), ORDS path (`/ords`), schema alias (empty: the account name in lower case), certificate check, request timeout (10 s).
