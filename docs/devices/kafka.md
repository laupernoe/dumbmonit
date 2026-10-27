# Apache Kafka

Apache Kafka and compatible brokers (Redpanda, Confluent): offline and under-replicated partitions, a missing controller, and the lag of the consumer groups.

## Network ports to open

From the DumbMonit server (or its relay agent) to the device:

| Port | Protocol | Purpose |
|---|---|---|
| 9092 | TCP | Kafka protocol (default port, plain or TLS), every broker is contacted |

DumbMonit speaks Kafka's own protocol: it asks one broker for the cluster metadata, then reads the committed offsets of the consumer groups and the end offsets of their partitions. Nothing is produced, consumed or created.

**Why a hand-written client.** `librdkafka` is a C++ library that does not belong in the static musl image, and the pure Rust crate `rskafka` cannot list consumer groups or read their committed offsets, which is half of what is worth watching. The six requests used here are small, in their non-flexible versions that Kafka 4.0 still serves, and go through the same TCP and TLS layer as Redis and MongoDB.

The lag needs every broker to be reachable at the address it advertises (`advertised.listeners`). When one is not, the cluster health stays and the lag is partial; `kafka_unreachable_brokers` says so.

## What it watches

All metrics are prefixed `dumbmonit_kafka_`.

| Metric | What | Labels |
|---|---|---|
| `cluster_info` | value 1 | `cluster_id` |
| `brokers`, `controller_present` | brokers in the cluster, and 1 while a controller is elected |  |
| `topics`, `partitions` | topics (internal ones excluded), and partitions |  |
| `under_replicated_partitions`, `offline_partitions` | partitions with fewer in-sync replicas than replicas, and partitions with no leader |  |
| `topic_under_replicated_partitions`, `topic_offline_partitions` | the same per topic, only for topics with a problem | `topic` |
| `consumer_groups` | consumer groups known to the cluster |  |
| `consumer_group_lag_total` | messages waiting for the group, summed over its partitions | `group` |
| `consumer_group_lag` | the same per topic | `group`, `topic` |
| `unreachable_brokers` | brokers DumbMonit could not contact while measuring the lag |  |

The lag is measured for the first groups in name order (the option Consumer groups measured, 50 by default); the totals still count all of them.

The [built-in rules](../alerting/rules.md#more-databases-and-kafka)
that apply: Kafka partitions offline, Kafka under-replicated partitions, Kafka without controller, Kafka consumer group lag, plus Monitor unreachable.

## Give DumbMonit read-only access to Kafka

1. Skip this step for a cluster without an authorizer. Otherwise give a dedicated principal read-only ACLs, from a host that can reach the brokers.

    ```
    kafka-acls.sh --bootstrap-server kafka.lan:9092 --add --allow-principal User:dumbmonit --operation Describe --cluster
    kafka-acls.sh --bootstrap-server kafka.lan:9092 --add --allow-principal User:dumbmonit --operation Describe --topic '*'
    kafka-acls.sh --bootstrap-server kafka.lan:9092 --add --allow-principal User:dumbmonit --operation Describe --group '*'
    ```

2. In DumbMonit, enter the address of one broker, for example kafka.lan:9092: DumbMonit then contacts the other brokers at the addresses they advertise.

3. Tick TLS if the listener is SSL or SASL_SSL, and enter the SASL/PLAIN user and password if the listener asks for them.

### Required permissions

- **Describe on the cluster**: List the consumer groups
- **Describe on topics**: Read partitions, leaders and end offsets
- **Describe on groups**: Read the offsets each group committed

!!! warning
    Each broker must be reachable at the address it advertises (advertised.listeners): a broker DumbMonit cannot reach leaves the consumer lag incomplete. SASL/PLAIN is accepted only over TLS; SCRAM and Kerberos are not supported.

### Why these permissions

Every call is a read: Metadata, ListGroups, OffsetFetch and ListOffsets. They require `Describe` on the cluster, the topics and the groups, and nothing else. SASL/PLAIN is only accepted over TLS so that the password is never sent in clear text.

## Credentials

| Credential | Fields |
|---|---|
| No authentication | A PLAINTEXT or SSL listener without SASL. |
| SASL/PLAIN | A user name and password for a SASL_SSL listener. Requires TLS. |

Address: a host name or IP (`kafka.lan`), `host:port`, or a pasted bootstrap address. Options: port (9092), TLS, certificate check, request timeout (10 s), and Consumer groups measured (50, from 0 to 500).
