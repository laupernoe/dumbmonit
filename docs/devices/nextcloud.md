# Nextcloud

Whether Nextcloud is serving or stuck: maintenance mode left on, a database
upgrade waiting after an update, Nextcloud and app updates, active users, free
space for the data directory, the database size, and PHP's OPcache.

DumbMonit reads the public `status.php` and the statistics of the
**serverinfo** app, which ships with Nextcloud, with a **monitoring token**
(`NC-Token`). The token opens those statistics and nothing else: it cannot log
in, list users, open a file or change a setting.

The failure worth watching: **an update that did not finish**. After an update
Nextcloud may stay in maintenance mode, or wait for `occ upgrade`; the web
page then shows a notice nobody sees until someone tries to sync. Both are read
from `status.php`, which answers even then.

## What it watches

All metrics are prefixed `dumbmonit_nextcloud_`.

| Metric | What | Labels |
|---|---|---|
| `maintenance` | 1 while maintenance mode is on | |
| `needs_db_upgrade` | 1 while Nextcloud waits for `occ upgrade` | |
| `version_info` | value 1 | `version` |
| `update_available` | 1 when Nextcloud's own update check found a newer release | `available_version` |
| `apps_installed`, `app_updates_available` | installed apps, and how many have a newer version in the app store | |
| `app_update_available` | value 1 per app with an update (at most 32) | `app`, `available_version` |
| `active_users` | users seen in the last 5 minutes, hour and day | `window` (`5m`, `1h`, `24h`) |
| `users`, `files`, `shares` | accounts, files and shares on the instance | |
| `free_space_bytes` | free space where Nextcloud keeps its data | |
| `database_size_bytes` | size of the database | |
| `platform_info` | value 1 | `database`, `database_version`, `php_version` |
| `php_memory_limit_bytes` | PHP `memory_limit` | |
| `opcache_enabled`, `opcache_full` | OPcache on; OPcache out of room | |
| `opcache_memory_used_percent`, `opcache_interned_strings_used_percent`, `opcache_hit_rate_percent`, `opcache_oom_restarts` | how full OPcache is and how well it serves | |
| `fpm_active_processes`, `fpm_total_processes`, `fpm_listen_queue`, `fpm_max_children_reached`, `fpm_slow_requests` | PHP-FPM pool, when Nextcloud runs behind PHP-FPM (the `-fpm` images, AIO) | |

While Nextcloud is in maintenance or waits for an upgrade, its API answers
503: only `maintenance`, `needs_db_upgrade` and `version_info` are written then,
and the device is not reported unreachable.

The [built-in rules](../alerting/rules.md#self-hosted-applications) that apply:
Nextcloud stuck in maintenance, Nextcloud database upgrade pending, Nextcloud
update available, Nextcloud OPcache full, Nextcloud free space low, plus Device
unreachable.

## Background jobs (cron)

Nextcloud does not tell the monitoring token when background jobs last ran:
only an account with every right on the instance can read that, and DumbMonit
does not ask for one. Watch cron with a [Heartbeat](push.md) instead: create
one in DumbMonit, then call its URL at the end of the cron line that runs
Nextcloud's background jobs, so it only calls in when `cron.php` succeeded:

```
*/5 * * * * php -f /var/www/html/cron.php && curl -fsS -m 10 --retry 3 https://monit.example.com/api/push/<token> > /dev/null
```

With an expected interval of 5 minutes, the heartbeat alerts as soon as cron
stops, whatever the reason (a container restarted without its cron, a PHP
error, background jobs left on AJAX).

## The device page

The panel above the charts reads what the probe stored; opening the page never
calls Nextcloud. It says what is wrong first (maintenance, upgrade waiting,
OPcache full or off), then updates, then active users, free space, database
size, users, files and OPcache figures, then the apps to update by name and the
platform (database, PHP).

## Give Nextcloud a monitoring token

1. Nextcloud's serverinfo app, shipped and enabled with Nextcloud, answers to a monitoring token that opens its statistics and nothing else: no file, no user, no share. On the Nextcloud server, set a long random token and print it back. Run occ as the web server user; in Docker, put docker exec -u www-data and the container name in front.

    ```
    php occ config:app:set serverinfo token --value "$(openssl rand -hex 32)"
    php occ config:app:get serverinfo token
    ```

2. If occ says serverinfo is disabled, enable it first.

    ```
    php occ app:enable serverinfo
    ```

3. In DumbMonit, enter the address you open Nextcloud with, for example "cloud.example.com" (HTTPS on 443) or "http://10.0.0.5:8080", and paste the token. The host must be one of Nextcloud's trusted domains, or Nextcloud refuses the request.

4. DumbMonit reads status.php and the serverinfo statistics only. It never opens a file, never lists users and never changes a setting, and the token cannot log in anywhere.

!!! warning
    Nextcloud does not tell the token when background jobs (cron) last ran: only an account with every right on the instance can read that. To be told when cron stops, add a Heartbeat in DumbMonit and call its URL at the end of the cron line, as the documentation shows.

With Nextcloud AIO, run occ in the `nextcloud-aio-nextcloud` container:
`docker exec -u www-data nextcloud-aio-nextcloud php occ …`. Why not a
delegated administration group: Nextcloud only opens the serverinfo API to the
token or to members of the full `admin` group, and delegation does not change
that.

Checking the token by hand, from the DumbMonit host:

```
curl -H 'NC-Token: YOUR-TOKEN' 'https://cloud.example.com/ocs/v2.php/apps/serverinfo/api/v1/info?format=json'
```

## Credentials

| Credential | Fields |
|---|---|
| Monitoring token (NC-Token) | The serverinfo token. DumbMonit sends it in the `NC-Token` header, with `OCS-APIRequest: true`. |

Address: a host name (`cloud.example.com`), `host:port`, or a full URL with a
path prefix (`https://example.com/nextcloud`); a trailing `/index.php` or
`/login` is accepted. Options: protocol (HTTPS by default), port (443),
certificate check and request timeout (15 s).

The app update count asks Nextcloud's app store cache; on an instance that
cannot reach the app store, that count stays at 0.

Validated against Nextcloud 35.0.1 (official image).
