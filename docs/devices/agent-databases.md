# Databases and their backups

The [agent](agent.md) finds the database engines installed on its machine and
checks that they are **backed up**. It does this without credentials: it never
connects to a database, never runs a SQL query and never reads a secret. It
only looks at installed services, listening ports, a few registry keys and the
folders where your backups land.

The databases appear as a **Databases** service under the machine, with one
line per database, for example `Running · last backup 3 h ago` or
`Stopped · No recent backup`. The state is always written, never colour alone.

## What is detected

| Engine | Linux | Windows |
|---|---|---|
| PostgreSQL | `postgresql@16-main`, `postgresql-16` units | `postgresql-x64-16` and similar services |
| MySQL / MariaDB | `mysql`, `mysqld`, `mariadb` units | `MySQL80`, `MySQL57`, `MariaDB` services |
| Microsoft SQL Server | `mssql-server` unit | `MSSQLSERVER`, `MSSQL$INSTANCE`, `SQLEXPRESS`: every edition, named instances included |
| HFSQL (PC SOFT: WinDev, WebDev, WinDev Mobile) | any unit named like HFSQL | HFSQL Control Center service (default port 4900) |
| MongoDB | `mongod` unit | `MongoDB` service |

For each database the agent reports the engine, the instance, the version when
it can be read without credentials (SQL Server: registry; PostgreSQL and MySQL:
the service name; PostgreSQL on Linux: the installed binaries), whether the
service is running, and the port. A default port that answers with no matching
service (a database in a container, for instance) is reported too.

The side services of an engine (SQL Server Agent, SQL Browser, MySQL Router,
exporters) are ignored.

## Backup freshness

For each database the agent looks for the **newest file or folder** matching a
list of paths and reports its date, its size and its age. It sets
`dumbmonit_agent_database_backup_overdue` to 1 when that age exceeds the
maximum age (36 hours by default) or when nothing matches a path you declared.

### Folders watched by default

| Engine | Default paths |
|---|---|
| SQL Server | the instance's default backup folder read from the registry (`*.bak`, `*.trn`); on Linux `/var/opt/mssql/data` |
| PostgreSQL (Linux) | `/var/backups/postgresql/*`, pgBackRest (`/var/lib/pgbackrest/backup/*/backup.info`), Barman (`/var/lib/barman/*/base/*/backup.info`) |
| MySQL / MariaDB (Linux) | `/var/backups/mysql/*`, `/var/backups/mariadb/*`, `/var/backups/*.sql.gz` |
| MongoDB (Linux) | `/var/backups/mongodb/*` |
| HFSQL | `Backup`, `Backups` and `Sauvegarde` next to the HFSQL server executable |

If none of these contains anything, the database is shown with
`backup folder not found` and **no alert is raised**: the agent cannot know
that backups are missing when it does not know where to look. Declare the
folder to turn the check on.

### Declaring your own folders

In `agent.yaml`:

```yaml
databases:
  backups:
    sqlserver:
      paths:
        - 'D:\Backups\SQL\*.bak'
        - 'D:\Backups\SQL\*.trn'
      max_age_hours: 26
    postgresql:
      paths: ['/srv/backups/pg/*.dump']
    mysql:                 # also accepts "mariadb"
      paths: ['/srv/backups/*.sql.gz']
    hfsql:
      paths: ['E:\HFSQL\Sauvegardes\*']
```

- Engine keys: `postgresql`, `mysql` (or `mariadb`), `sqlserver`, `hfsql`,
  `mongodb`. A rule applies to every instance of its engine.
- `paths`: files or folders. `*` and `?` work in every segment of the path
  (`D:\Backups\*\*.bak`). Quote Windows paths with single quotes.
- `max_age_hours`: default `36`. With several rules for one engine, the
  smallest age applies.
- Declared paths replace the default ones for that engine. When nothing matches
  them, the backup counts as overdue.
- `databases.enabled: false` (or `DUMBMONIT_AGENT_DATABASES=false`) turns the
  module off; `databases.interval_secs` (default `300`, minimum `60`) sets the
  reading period.

The module can also be unticked from the machine page, like the other agent
services.

## Metrics and alerts

All series carry the labels `engine` and `instance` (the server adds `target`).

| Metric | Meaning |
|---|---|
| `dumbmonit_agent_database_up` | 1 if the service runs or its port answers, 0 if stopped |
| `dumbmonit_agent_database_port` | Port in use, when known |
| `dumbmonit_agent_database_info` | Always 1; the `version` label holds the version |
| `dumbmonit_agent_database_backup_overdue` | 1 when the newest backup is older than the maximum age |
| `dumbmonit_agent_database_backup_age_seconds` | Age of the newest backup |
| `dumbmonit_agent_database_backup_last_timestamp_seconds` | Date of the newest backup (Unix time) |
| `dumbmonit_agent_database_backup_size_bytes` | Size of the newest backup file |
| `dumbmonit_agent_database_backup_max_age_seconds` | Maximum age in force |

Two built-in alert rules read them: **Database backup overdue** (the overdue
verdict held for an hour) and **Database service stopped** (10 minutes).

## Limits

The check proves that a recent backup *file* exists, not that it can be
restored. Querying the database itself (`msdb.dbo.backupset` for SQL Server,
`pg_stat_archiver` for PostgreSQL) needs credentials and is not done yet.
