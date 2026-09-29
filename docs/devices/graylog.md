# Graylog

The log server and the search cluster behind it: whether Graylog processes
messages, how many wait in its journal, how full its buffers are, what comes
in and what goes out, the health of OpenSearch or Elasticsearch as Graylog sees
it, inputs that failed to start, and messages that failed to be indexed.

DumbMonit reads Graylog's REST API with a dedicated user holding the built-in
**Reader** role. It never searches messages, never starts or stops an input and
never acknowledges a notification.

The failure worth watching: **the search cluster stops keeping up**. Graylog
keeps accepting messages and writes them to its on-disk journal; the web
interface opens, searches work, and simply show nothing recent. The journal
grows until it is full, then messages are dropped. The journal backlog, the
search cluster health and the indexing failures say it long before.

## What it watches

All metrics are prefixed `dumbmonit_graylog_`. The journal, buffers and
throughput are those **of the node DumbMonit talks to**: in a Graylog cluster,
add each node as its own device.

| Metric | What | Labels |
|---|---|---|
| `processing` | 1 while the node processes messages, 0 when processing is paused | |
| `running`, `lb_alive` | the node's lifecycle is `running`; it reports itself alive to load balancers | |
| `version_info` | value 1 | `version` |
| `journal_uncommitted_entries` | messages written to the journal and not processed yet | |
| `journal_size_bytes`, `journal_size_limit_bytes`, `journal_used_percent` | | |
| `journal_append_per_second`, `journal_read_per_second` | | |
| `buffer_used_percent` | how full the input, process and output buffers are | `buffer` |
| `messages_in_total`, `messages_out_total` | messages received, and written out to the search cluster | |
| `indexer_status` | search cluster health: 0 green, 1 yellow, 2 red, 3 unreachable from Graylog | |
| `indexer_shards` | shards by state: `active`, `initializing`, `relocating`, `unassigned` | `state` |
| `inputs_running`, `inputs_failed` | inputs on this node | |
| `input_failed` | value 1 per input that failed to start | `input` (its title) |
| `output_failures_total`, `processing_failures_total`, `invalid_timestamps_total` | writes to the search cluster that failed, messages that failed processing or indexing, messages with an invalid timestamp | |
| `notifications`, `notifications_urgent`, `notification` | Graylog's own notifications (optional permission, see below) | `type`, `severity` |
| `cluster_nodes` | nodes in the Graylog cluster | |
| `scrape_errors` | calls that failed on the last probe | |

The search cluster is read through Graylog, so a separate OpenSearch or
Elasticsearch device is not needed to know whether it is green; when Graylog
cannot reach it at all, `indexer_status` reads 3.

The [built-in rules](../alerting/rules.md#log-and-metrics-servers) that apply:
Graylog not processing, Graylog search cluster down, Graylog search cluster
yellow, Graylog journal filling, Graylog input failed, Graylog indexing
failures, plus Device unreachable.

## The device page

The panel above the charts reads what the probe stored; opening the page never
queries Graylog. It says what is wrong in a sentence (processing, search
cluster, journal backlog, failed inputs by name, indexing failures, urgent
notifications), then messages in and out per second, the journal backlog and
the number of nodes, then the buffers, the shards by state and Graylog's own
notifications.

## Create a read-only user and token in Graylog

1. In Graylog: System → Users and Teams → Create user. Name the user as follows, give it a long random password, and assign the Reader role only. Reader can read the node status, journal, buffers, throughput, inputs and search cluster health, and no message at all as long as no stream is shared with it.

    ```
    dumbmonit
    ```

2. Open that user's Edit tokens page, create a token named after DumbMonit and copy it now: Graylog shows it only once.

3. Optional: Graylog's own notifications (an input that failed to start, disk watermarks, a journal almost full) need one more permission, notifications:read, which no built-in role grants on its own. With an account allowed to manage roles, create a role holding only that permission and give it to the dumbmonit user. Without it, notifications are skipped and nothing else changes.

    ```
    curl -u YOUR_USER -H 'X-Requested-By: cli' -H 'Content-Type: application/json' -X POST https://graylog.lan/api/roles -d '{"name":"DumbMonit Notifications","description":"Read system notifications","permissions":["notifications:read"],"read_only":false}'
    curl -u YOUR_USER -H 'X-Requested-By: cli' -X PUT 'https://graylog.lan/api/roles/DumbMonit%20Notifications/members/dumbmonit'
    ```

4. In DumbMonit, enter the address of the Graylog node, for example "graylog.lan" (port 9000) or "https://graylog.lan" behind a reverse proxy, and paste the token. In a Graylog cluster, add each node: journal, buffers and throughput are per node.

5. DumbMonit only reads. It never searches messages, never starts or stops an input and never acknowledges a notification.

!!! warning
    Do not reuse the account you log in with: a leaked token carries every right of its user, while the Reader role above can change nothing. Graylog serves plain HTTP unless you configured TLS, and the token travels with every request: across an untrusted network, put Graylog behind HTTPS.

What each Reader permission covers: `system:read` the node status and cluster
nodes, `journal:read` the journal, `metrics:read` and `throughput:read` the
throughput and buffers, `inputs:read` the inputs, `indexercluster:read` the
search cluster health. The Reader role also holds `messages:read`, which lets
it read messages only from streams explicitly shared with it: share none.
Indexing failure details (`indices:failures`) are not needed: DumbMonit counts
failures from Graylog's own metrics.

## Credentials

| Credential | Fields |
|---|---|
| Access token (recommended) | A token of the dedicated user. Graylog expects it as HTTP basic authentication with the token as user name and the word `token` as password; DumbMonit does this for you. |
| User name / password | The dedicated user itself. |

Address: a host name or IP (`graylog.lan`), `host:port`, or a full URL
(`https://graylog.lan`); a trailing `/api` is accepted. Options: protocol
(HTTP by default), port (9000), certificate check and request timeout (10 s).
