# Ceph

!!! warning "Beta"

    This integration was built from the Ceph manager dashboard REST API
    documentation, not against a running cluster yet. It only reads. Tell us
    what breaks.

A Ceph cluster through the REST API of the manager's dashboard module: the
health status and the checks behind it, OSDs that are down, out or nearly
full, placement groups that are not `active+clean`, raw capacity, pools and
monitors that left the quorum.

DumbMonit signs in with a dashboard user holding the built-in `read-only`
role (`POST /api/auth`, the credentials in the request body), then sends the
returned token as `Authorization: Bearer`, never in the URL. Every call asks
for `application/vnd.ceph.api.v1.0+json`, the API version the dashboard
documents. The token is kept for an hour and renewed when refused: signing out
every minute would fill the manager's token revocation list.

One page is required — `/api/health/minimal` (health, checks, placement
groups, capacity, client throughput) — and four are optional, left out without
failing the rest when the role or the Ceph version does not serve them:
`/api/osd`, `/api/pool`, `/api/monitor` and `/api/summary` (the version).

!!! note "Agent variant"

    On a host that has the Ceph CLI, `ceph status -f json` gives the same
    health and placement group figures. DumbMonit does not read it through the
    agent yet; the dashboard API above is the supported way.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 8443 | TCP | Dashboard REST API (HTTPS, default) |
| 8080 | TCP | Only with the HTTP protocol option |

## What it watches

All metrics are prefixed `dumbmonit_ceph_`.

| Metric | Labels | Meaning |
|---|---|---|
| `version_info` | `version` | Value 1. |
| `health_status` | | 0 `HEALTH_OK`, 1 `HEALTH_WARN`, 2 `HEALTH_ERR`. |
| `health_ok` | | 1 when the status is `HEALTH_OK`. |
| `health_checks` | | Active checks, those an operator muted left out. |
| `health_check_info` | `check`, `severity` | Value 1 per active check (`OSD_DOWN`, `PG_DEGRADED`…). |
| `osds`, `osds_up`, `osds_in`, `osds_down` | | OSD counts. |
| `osd_up`, `osd_in` | `osd`, `host` | Per OSD. |
| `osd_used_percent` | `osd`, `host` | Fill of each OSD. |
| `pgs`, `pgs_clean`, `pgs_unclean` | | Placement groups; clean means both `active` and `clean`. |
| `pgs_in_state` | `state` | Placement groups per state (`active+clean`, `active+undersized+degraded`…). |
| `objects`, `objects_degraded`, `objects_misplaced`, `objects_unfound` | | Object counts. |
| `capacity_bytes`, `capacity_used_bytes`, `capacity_available_bytes`, `capacity_used_percent` | | Raw capacity of the cluster. |
| `pools` | | Number of pools. |
| `pool_used_percent`, `pool_used_bytes`, `pool_objects` | `pool` | Per pool. |
| `mons`, `mons_in_quorum`, `mons_out_of_quorum` | | Monitors. |
| `hosts` | | Hosts known to the orchestrator. |
| `client_read_bytes_per_second`, `client_write_bytes_per_second`, `client_read_ops_per_second`, `client_write_ops_per_second`, `recovering_bytes_per_second` | | Client and recovery throughput. |

The [built-in rules](../alerting/rules.md#ceph) that apply:

- **Ceph health error**: `HEALTH_ERR` for five minutes (Warning).
- **Ceph health warning**: `HEALTH_WARN` for 30 minutes (Advisory).
- **Ceph OSD down**: for ten minutes (Advisory).
- **Ceph placement groups not clean**: for 30 minutes (Advisory).
- **Ceph objects unfound**: for five minutes (Warning).
- **Ceph capacity high**: above 80 % for 30 minutes (Advisory).
- **Ceph OSD nearly full**: above 85 % for 15 minutes (Advisory).
- **Ceph pool nearly full**: above 90 % for 15 minutes (Advisory).
- **Ceph monitor out of quorum**: for five minutes (Warning).

## The monitor page

The Cluster panel says what is wrong in a sentence, shows OSDs up, clean
placement groups, capacity and monitors in quorum, then lists the active
health checks, the OSDs that need attention and the pools by fill.

## Create a read-only dashboard user for DumbMonit

1. Check that the dashboard is enabled and note the address of the active manager.

    ```
    ceph mgr services
    ```

2. Write a long random password to a file, and keep it: DumbMonit needs it.

    ```
    openssl rand -base64 24 | tr -d '\n' | tee /tmp/dumbmonit.pwd
    ```

3. Create the user with the built-in read-only role.

    ```
    ceph dashboard ac-user-create dumbmonit -i /tmp/dumbmonit.pwd read-only
    ```

4. Delete the password file.

    ```
    shred -u /tmp/dumbmonit.pwd
    ```

5. In DumbMonit, enter the address of the active manager (for example ceph.lan) with that user name and password.

### Required permissions

- **read-only role** — Reads the state of the cluster, changes nothing

!!! warning
    A standby manager redirects to the active one: use the address of the active manager. The dashboard ships a self-signed certificate: tick "Accept an unverifiable certificate" unless you installed your own.

### Notes

- The dashboard may enforce a password policy (length, complexity); a random
  base64 password normally satisfies it.
- Without the dashboard module (`ceph mgr module enable dashboard`) there is
  nothing to read.

## Credentials

| Credential | Fields |
|---|---|
| Dashboard user and password | The user created above. |

Address: the host name or IP of the active manager, `host:port`, or a URL.
Options:

- **Protocol** (https): `http` or `https`.
- **Port** (8443): the dashboard port, 8443 over HTTPS and 8080 over HTTP.
- **Accept an unverifiable certificate**: the dashboard's own certificate is
  self-signed.
- **Timeout per request** (15 s).

## Troubleshooting

"Ceph refused the login": wrong user name or password, or the user is
disabled or has no role.

"Ceph answered 303 …": the address is a standby manager; use the active one
shown by `ceph mgr services`.

No pool, OSD or monitor figure: those pages are optional and the role or the
Ceph version may not serve them; the health, placement group and capacity
figures are still read.
