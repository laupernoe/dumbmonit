# RabbitMQ

The message broker: memory and disk alarms, stopped nodes and network
partitions, queues that fill up with nobody reading them, queues that are no
longer running, and the flow of messages.

DumbMonit reads the management API with a user tagged `monitoring` that has
empty permissions: it sees the state of the nodes, the alarms and the queues,
and cannot publish, consume, read a message or change anything. One node
reports the whole cluster.

The failure worth watching is the one clients do not report. When a node
crosses its memory high watermark or its free disk limit, RabbitMQ raises an
alarm and blocks every publisher in the cluster: applications simply hang on
`publish`, with no error. The second one is a queue that keeps filling because
the service that consumed it has stopped.

## What it watches

All metrics are prefixed `dumbmonit_rabbitmq_`. Totals are counters: charts and
rules turn them into rates.

| Metric | What | Labels |
|---|---|---|
| `version_info` | value 1 | `version`, `erlang`, `cluster` |
| `connections`, `channels`, `consumers`, `queues` | cluster totals | |
| `messages_ready`, `messages_unacked` | messages waiting in every queue, and delivered but not yet acknowledged | |
| `messages_published_total`, `messages_delivered_total`, `messages_redelivered_total`, `messages_unroutable_total` | message flow; unroutable are messages no queue was bound to receive | |
| `alarm` | 1 while the alarm is raised; memory and disk are always present, at 0 without alarm | `node`, `resource` |
| `alarms` | alarms raised in the cluster | |
| `nodes`, `nodes_running`, `partitions` | | |
| `node_running`, `node_partitions` | per node | `node` |
| `node_memory_used_bytes`, `node_memory_limit_bytes`, `node_memory_used_percent` | memory against the high watermark | `node` |
| `node_disk_free_bytes`, `node_disk_free_limit_bytes` | free disk against the limit | `node` |
| `node_fd_used`, `node_fd_used_percent`, `node_uptime_seconds`, `node_being_drained` | | `node` |
| `queue_messages_ready`, `queue_messages_unacked`, `queue_consumers`, `queue_running` | per queue, the fullest first, up to Queues watched one by one | `vhost`, `queue` |
| `queues_without_consumers`, `queues_not_running` | among those queues | |

Alarms are read from the health check `/api/health/checks/alarms`, which sees
them at once; the node statistics lag by a few seconds.

The [built-in rules](../alerting/rules.md#databases-message-broker-and-security-engine)
that apply: RabbitMQ memory or disk alarm, RabbitMQ node down, RabbitMQ
network partition, RabbitMQ queue without consumer, RabbitMQ queue
unavailable, plus Monitor unreachable.

## The monitor page

The panel above the charts reads what the probe stored; opening the page never
queries RabbitMQ. It says what is wrong in a sentence (alarms, nodes and
partitions, queues holding messages with no consumer, queues not running),
then the messages ready and unacknowledged, messages published and delivered
per second, connections, consumers and queues, then the ten fullest queues
with their consumers and the memory and free disk of each node.

## Create a RabbitMQ monitoring user

1. Open the RabbitMQ management interface, for example http://rabbit.lan:15672, and log in with a user allowed to manage users.

2. Go to Admin → Users → Add a user, name it dumbmonit, set a password and the tag monitoring, then click Add user.

3. Click dumbmonit in the list, and under Set permission pick each virtual host, set Configure, Write and Read to ^$, then click Set permission.

4. In DumbMonit, enter the address of one node, for example rabbit.lan, or https://rabbit.lan:15671 when the API is served over TLS.

### Required permissions

- **monitoring (user tag)**: Read nodes, alarms and queues
- **Configure, Write, Read = ^$ on each virtual host**: Lets it list that host's queues

!!! warning
    A memory or disk alarm blocks every publisher in the cluster, and clients get no error.

### Why these permissions

The management plugin (rabbitmq-plugins enable rabbitmq_management; the management Docker images have it) serves the API on port 15672. The monitoring tag can read node, alarm and queue state and cannot change anything. A user only sees the queues of the virtual hosts it has some permission on: the empty patterns ^$ let it list them without being able to publish, consume or read a message. Without permissions on a virtual host, its queues are not listed one by one, and the cluster totals still count them. One node reports the whole cluster: the alarms and partitions of every node, and every queue. Queues are listed fullest first, up to Queues watched one by one (100 by default).

Vendor documentation: https://www.rabbitmq.com/docs/management#permissions

## Credentials

| Credential | Fields |
|---|---|
| Username / password | The `monitoring` user created above, sent as HTTP basic authentication. |

Without permissions on a virtual host, its queues are not listed one by one;
the cluster totals still count them.

Address: a host name or IP (`rabbit.lan`), `host:port`, or a full URL with a
path prefix behind a reverse proxy. Options: protocol (HTTP by default), port
(15672), certificate check, request timeout (10 s), and Queues watched one by
one (100, from 0 to 500).
