# Paperless-ngx

Whether Paperless-ngx still imports what it is sent: the health of its
database, Redis, Celery workers, search index, classifier and sanity check,
failed tasks, updates, disk space and the number of documents.

DumbMonit reads Paperless-ngx's REST API with a **dedicated user that can read
no document**: three view permissions give it the task list, the system status
and the global counts, nothing else.

The failure worth watching: **Redis or Celery stopped**. The web interface
opens and documents can be read, but nothing dropped in the consume folder or
fetched from mail is imported until someone notices the pile. System status
says so; DumbMonit alerts on it.

## What it watches

All metrics are prefixed `dumbmonit_paperless_`.

| Metric | What | Labels |
|---|---|---|
| `version_info` | value 1 | `version` |
| `update_available` | 1 when Paperless-ngx's own version check knows a newer release (absent when the check is off) | `latest_version` |
| `component_status` | 0 OK, 1 warning, 2 error, as the system status page reports it | `component`: `database`, `redis`, `celery`, `index`, `classifier`, `sanity_check`, `llm_index` (only when enabled) |
| `unapplied_migrations` | database migrations not applied yet | |
| `storage_available_bytes`, `storage_total_bytes`, `storage_used_percent` | the disk that holds the media | |
| `tasks_failed_window`, `tasks_window`, `tasks_pending` | all tasks of the last 30 days: failed, total, and still pending | |
| `tasks_failed_recent` | failed tasks this user can see, newer than the failed task window and not dismissed | |
| `task_failed` | value 1 per such task (at most 32) | `task`, `file` |
| `documents`, `documents_inbox` | documents in Paperless, and in the inbox | |
| `status_readable` | 0 when the account lacks the system monitoring permission | |
| `scrape_errors` | calls that failed on the last probe | |

Paperless keeps each owner's tasks private: the dedicated user sees scheduled
tasks (sanity check, index, classifier, mail) by name, while imports by other
users are counted in `tasks_failed_window`, which covers every task.

The [built-in rules](../alerting/rules.md#self-hosted-applications) that apply:
Paperless task queue down, Paperless index or sanity check error, Paperless
migrations pending, Paperless task failed, Paperless storage almost full,
Paperless update available, plus Device unreachable.

## The device page

The panel above the charts reads what the probe stored; opening the page never
calls Paperless. It says what is wrong first (a component in error, failed
tasks, migrations pending), then updates, then documents, inbox, pending and
failed tasks and disk space, then each component and the failed tasks by name.

## Create a read-only user in Paperless-ngx

1. Create a user named as follows, with a long random password, no superuser or staff status, and three view permissions only: tasks, system monitoring and global statistics. With the API and your own account, the two commands below create it and print its token.

    ```
    dumbmonit
    curl -u YOUR_USER -H 'Content-Type: application/json' -X POST https://paperless.example.com/api/users/ -d '{"username":"dumbmonit","password":"CHANGE-ME","user_permissions":["view_paperlesstask","view_system_monitoring","view_global_statistics"]}'
    curl -X POST https://paperless.example.com/api/token/ -d 'username=dumbmonit&password=CHANGE-ME'
    ```

2. In the web interface instead: Users & Groups → Add user, then in its permissions tick View on the PaperlessTask, SystemMonitoring and GlobalStatistics rows only. Log in as that user once, open My Profile and generate its API token.

3. This user reads no document at all: global statistics give the counts, system monitoring gives the health of the database, Redis, Celery and the index, and tasks show the failed ones. On Paperless-ngx 2.x, where the last two permissions do not exist, health and counts are skipped and the rest is read.

4. In DumbMonit, enter the address, for example "paperless.lan" (port 8000) or "https://paperless.example.com" behind a reverse proxy, and paste the token.

!!! warning
    When Redis or Celery stops, Paperless keeps serving its pages and documents, but nothing new is imported, from the consume folder or from mail, until someone looks. DumbMonit alerts on both.

Replace `CHANGE-ME` with a long random password before running the commands,
and `YOUR_USER` with an account allowed to manage users (curl asks for its
password). Asking for the system status can take several seconds while Redis is
down: Paperless waits for it before answering.

## Credentials

| Credential | Fields |
|---|---|
| API token (recommended) | The dedicated user's token, sent as `Authorization: Token …`. |
| User name / password | The dedicated user itself, sent as HTTP basic authentication. |

Address: a host name (`paperless.lan`), `host:port`, or a full URL with a path
prefix; a trailing `/api` is accepted. Options: protocol (HTTP by default),
port (8000), certificate check, request timeout (15 s) and the failed task
window (24 hours, from 1 to 720).

Validated against Paperless-ngx 3.2.1 (official image, SQLite, Valkey).
