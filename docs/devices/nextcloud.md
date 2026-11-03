# Nextcloud

Whether Nextcloud is serving or stuck: maintenance mode left on, a database
upgrade waiting after an update, Nextcloud and app updates, active users, free
space for the data directory, the database size, and PHP's OPcache.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 443 | TCP | API (HTTPS, default) |
| 80 | TCP | Only with the http scheme option |

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
| `active_users` | users seen in the last 5 minutes, hour, day, week and month | `window` (`5m`, `1h`, `24h`, `7d`, `30d`) |
| `users`, `files`, `shares` | accounts, files and shares on the instance | |
| `free_space_bytes` | free space where Nextcloud keeps its data | |
| `database_size_bytes` | size of the database | |
| `platform_info` | value 1 | `database`, `database_version`, `php_version` |
| `php_memory_limit_bytes` | PHP `memory_limit` | |
| `extended_support` | 1 when this version is out of standard support (Enterprise fixes only) | |
| `disabled_users` | disabled accounts | |
| `storages` | storages by kind | `kind` (`local`, `home`, `other`) |
| `shares_by_type` | shares by type | `type` (`user`, `group`, `link`, `mail`, `room`, `federated_sent`, `federated_received`) |
| `shares_link_without_password` | public links with no password | |
| `cpu_count`, `cpu_load` | cores and load average of the Nextcloud host | `window` (`1m`, `5m`, `15m`) for `cpu_load` |
| `memory_total_bytes`, `memory_free_bytes`, `memory_used_percent` | memory of the host | |
| `swap_total_bytes`, `swap_free_bytes`, `swap_used_percent` | swap of the host | |
| `php_max_execution_time_seconds`, `php_upload_max_filesize_bytes` | PHP limits that cap long jobs and uploads | |
| `debug_mode` | 1 when `debug` is on in `config.php` | |
| `file_locking_enabled`, `memcache_local_enabled`, `memcache_distributed_enabled`, `memcache_locking_enabled` | file locking, and which caches are configured | |
| `update_check_age_seconds` | age of Nextcloud's last update check, a daily background job: it stops growing healthy and keeps growing when cron is not running (only written once the check ran at least once) | |
| `apcu_hit_rate_percent`, `apcu_memory_used_percent`, `apcu_expunges` | APCu, the local memory cache | |
| `opcache_cached_keys_used_percent`, `opcache_wasted_percent` | OPcache script slots used (they fill before the memory does) and wasted memory | |
| `opcache_enabled`, `opcache_full` | OPcache on; OPcache out of room | |
| `opcache_memory_used_percent`, `opcache_interned_strings_used_percent`, `opcache_hit_rate_percent`, `opcache_oom_restarts` | how full OPcache is and how well it serves | |
| `fpm_active_processes`, `fpm_total_processes`, `fpm_listen_queue`, `fpm_max_children_reached`, `fpm_slow_requests` | PHP-FPM pool, when Nextcloud runs behind PHP-FPM (the `-fpm` images, AIO) | |

While Nextcloud is in maintenance or waits for an upgrade, its API answers
503: only `maintenance`, `needs_db_upgrade` and `version_info` are written then,
and the monitor is not reported unreachable.

The [built-in rules](../alerting/rules.md#self-hosted-applications) that apply:

| Rule | Fires when |
|---|---|
| Nextcloud stuck in maintenance | maintenance mode on for 30 minutes |
| Nextcloud database upgrade pending | `occ upgrade` waiting for 10 minutes (critical) |
| Nextcloud update available | a newer release exists (info) |
| Nextcloud OPcache full | OPcache out of memory for 30 minutes |
| Nextcloud free space low | under 5 GiB free for 30 minutes |
| Nextcloud in debug mode | `debug` on for an hour |
| Nextcloud OPcache script slots almost full | 95% of the slots used for an hour |
| Nextcloud OPcache restarting | OPcache restarted for lack of memory in the last hour |
| Nextcloud background jobs seem stopped | no update check for 14 days |
| Nextcloud PHP memory limit low | `memory_limit` under 512 MiB (info, unlimited is fine) |
| Nextcloud PHP-FPM out of workers | `pm.max_children` reached in the last 30 minutes |
| Nextcloud PHP-FPM queue building up | more than 10 requests waiting for ten minutes |
| Nextcloud app updates waiting | app updates pending for three days (info) |
| Nextcloud version out of standard support | `extended_support` is 1 |

plus Monitor unreachable.

## Background jobs (cron)

Nextcloud does not tell the monitoring token when background jobs last ran:
only an account with every right on the instance can read that, and DumbMonit
does not ask for one. Two things stand in for it. The rule **Nextcloud background
jobs seem stopped** watches the age of Nextcloud's daily update check, itself a
background job (if you turned the update checker off in `config.php`, disable that
rule). For a precise answer, watch cron with a [Heartbeat](push.md): create
one in DumbMonit, then call its URL at the end of the cron line that runs
Nextcloud's background jobs, so it only calls in when `cron.php` succeeded:

```
*/5 * * * * php -f /var/www/html/cron.php && curl -fsS -m 10 --retry 3 https://monit.example.com/api/push/<token> > /dev/null
```

With an expected interval of 5 minutes, the heartbeat alerts as soon as cron
stops, whatever the reason (a container restarted without its cron, a PHP
error, background jobs left on AJAX).

## Client devices

Nextcloud does not expose an admin-wide list of sync clients or mobile
devices through any API. The personal "Devices & sessions" page (Settings →
Security) lists only the account currently logged in, backed by a browser
session rather than an API token, and the only admin-wide equivalent is the
command line (`occ user:auth-tokens:list <user>`, one account at a time) —
not something the monitoring token can reach, and not something DumbMonit
asks an administrator to script into a probe. The `serverinfo` API that
DumbMonit reads has no device list either: it gives counts of active users
(`active_users`, above), never who or what they are.

This is a deliberate limit, not a missing feature: giving DumbMonit a device
list would mean widening the monitoring token into a full administration
account, which the rest of this page goes out of its way to avoid. If
Nextcloud ever exposes this over OCS, DumbMonit will follow the same
[client devices](../alerting/rules.md) pattern already used for Immich,
Proxmox Backup Server and Veeam.

## The monitor page

The panel above the charts reads what the probe stored; opening the page never
calls Nextcloud. It says what is wrong first (maintenance, upgrade waiting,
OPcache full or off), then updates, then active users, free space, database
size, users, files and OPcache figures, then the apps to update by name and the
platform (database, PHP).

## Give Nextcloud a monitoring token

1. On the Nextcloud server, set a random token with occ as the web server user (in Docker: docker exec -u www-data, then the container name).

    ```
    php occ config:app:set serverinfo token --value "$(openssl rand -hex 32)"
    php occ config:app:get serverinfo token
    ```

2. If occ says serverinfo is disabled, enable the Monitoring app: profile menu → Apps → Monitoring → Enable.

3. In DumbMonit, enter the address you open Nextcloud with and paste the token.

Vendor documentation: <https://github.com/nextcloud/serverinfo#api>

### Why a token, and why occ

Nextcloud's serverinfo app answers to a monitoring token that opens its statistics and nothing else: no file, no user, no share, and the token cannot log in anywhere. The Monitoring app documents no screen to set that token, so occ is the way to set it. DumbMonit reads `status.php` and the serverinfo statistics only: it never opens a file, never lists users and never changes a setting.

Right after the token is set, Nextcloud may refuse the very first read (401) before it takes effect: DumbMonit retries once after two seconds, so a brand-new target does not show a spurious "token refused" error. If you prefer the command line to enable the app, `php occ app:enable serverinfo` does the same as the Apps page.

The address must be one of Nextcloud's trusted domains (`trusted_domains` in `config.php`).

!!! warning
    Use an address listed in Nextcloud's trusted domains, or Nextcloud refuses the request.

With Nextcloud AIO, run occ in the `nextcloud-aio-nextcloud` container:
`docker exec -u www-data nextcloud-aio-nextcloud php occ …`. Why not a
delegated administration group: Nextcloud only opens the serverinfo API to the
token or to members of the full `admin` group, and delegation does not change
that.

Checking the token by hand, from the DumbMonit host:

```
curl -H 'NC-Token: YOUR-TOKEN' 'https://cloud.example.com/ocs/v2.php/apps/serverinfo/api/v1/info?format=json'
```

## Troubleshooting

- **401 on the first read.** The token was set seconds ago and Nextcloud has not
  picked it up: DumbMonit retries once after two seconds. A token that stays
  refused is wrong, or the `serverinfo` app is disabled.
- **"Untrusted domain" or an HTML page.** The address is not in `trusted_domains`
  (`config.php`).
- **Only a few metrics, and "maintenance" is 1.** Normal while Nextcloud is in
  maintenance mode or waits for `occ upgrade`: its API answers 503 and the monitor
  is not reported unreachable.
- **No `update_check_age_seconds`.** The update checker is off or never ran.
- **No `fpm_*` or `apcu_*` series.** Nextcloud runs under Apache with mod_php (no
  PHP-FPM), or APCu is not installed.
- **No integrity check or background job dates.** The monitoring token cannot read
  them, by design: see "Background jobs (cron)" above.

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
