# Apache Kafka

Apache Kafka et les courtiers compatibles (Redpanda, Confluent) : partitions hors ligne et sous-répliquées, contrôleur absent, et retard des groupes de consommateurs.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 9092 | TCP | Protocole Kafka (port par défaut, en clair ou TLS), chaque courtier est contacté |

DumbMonit parle le protocole natif de Kafka : il demande à un courtier la métadonnée du cluster, puis lit les offsets validés des groupes de consommateurs et les derniers offsets de leurs partitions. Rien n'est produit, consommé ni créé.

**Pourquoi un client écrit à la main.** `librdkafka` est une bibliothèque C++ qui n'a pas sa place dans l'image statique musl, et la crate Rust pure `rskafka` ne sait ni lister les groupes de consommateurs ni lire leurs offsets validés, soit la moitié de ce qui vaut la peine d'être surveillé. Les six requêtes employées ici sont petites, dans leurs versions non flexibles que Kafka 4.0 sert encore, et passent par la même couche TCP et TLS que Redis et MongoDB.

Le retard exige que chaque courtier soit joignable à l'adresse qu'il annonce (`advertised.listeners`). Quand l'un ne l'est pas, la santé du cluster reste et le retard est partiel ; `kafka_unreachable_brokers` le dit.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_kafka_`.

| Métrique | Quoi | Étiquettes |
|---|---|---|
| `cluster_info` | valeur 1 | `cluster_id` |
| `brokers`, `controller_present` | courtiers du cluster, et 1 tant qu'un contrôleur est élu |  |
| `topics`, `partitions` | rubriques (hors rubriques internes), et partitions |  |
| `under_replicated_partitions`, `offline_partitions` | partitions avec moins de réplicas synchronisés que de réplicas, et partitions sans leader |  |
| `topic_under_replicated_partitions`, `topic_offline_partitions` | la même chose par rubrique, seulement pour celles qui ont un problème | `topic` |
| `consumer_groups` | groupes de consommateurs connus du cluster |  |
| `consumer_group_lag_total` | messages en attente pour le groupe, sommés sur ses partitions | `group` |
| `consumer_group_lag` | la même chose par rubrique | `group`, `topic` |
| `unreachable_brokers` | courtiers que DumbMonit n'a pas pu joindre en mesurant le retard |  |

Le retard est mesuré pour les premiers groupes par ordre de nom (l'option Groupes mesurés, 50 par défaut) ; le total les compte tous.

Les [règles intégrées](../alerting/rules.md#more-databases-and-kafka)
qui s'appliquent : Partitions Kafka hors ligne, Partitions Kafka sous-répliquées, Kafka sans contrôleur, Retard d'un groupe de consommateurs Kafka, plus Surveillance inaccessible.

## Donner à DumbMonit un accès en lecture seule à Kafka

1. Passez cette étape pour un cluster sans autorisation. Sinon, donnez à un principal dédié des ACL en lecture seule, depuis une machine qui joint les courtiers.

    ```
    kafka-acls.sh --bootstrap-server kafka.lan:9092 --add --allow-principal User:dumbmonit --operation Describe --cluster
    kafka-acls.sh --bootstrap-server kafka.lan:9092 --add --allow-principal User:dumbmonit --operation Describe --topic '*'
    kafka-acls.sh --bootstrap-server kafka.lan:9092 --add --allow-principal User:dumbmonit --operation Describe --group '*'
    ```

2. Dans DumbMonit, saisissez l'adresse d'un courtier, par exemple kafka.lan:9092 : DumbMonit contacte ensuite les autres courtiers aux adresses qu'ils annoncent.

3. Cochez TLS si l'écouteur est SSL ou SASL_SSL, et saisissez l'utilisateur et le mot de passe SASL/PLAIN si l'écouteur les demande.

### Droits requis

- **Describe on the cluster** : Lister les groupes de consommateurs
- **Describe on topics** : Lire les partitions, les leaders et les derniers offsets
- **Describe on groups** : Lire les offsets validés de chaque groupe

!!! warning
    Chaque courtier doit être joignable à l'adresse qu'il annonce (advertised.listeners) : un courtier que DumbMonit ne joint pas laisse le retard des consommateurs incomplet. SASL/PLAIN n'est accepté que sous TLS ; SCRAM et Kerberos ne sont pas pris en charge.

### Pourquoi ces choix

Chaque appel est une lecture : Metadata, ListGroups, OffsetFetch et ListOffsets. Ils demandent `Describe` sur le cluster, les rubriques et les groupes, rien d'autre. SASL/PLAIN n'est accepté que sous TLS, pour que le mot de passe ne parte jamais en clair.

## Identifiants

| Identifiant | Champs |
|---|---|
| Aucune authentification | Un écouteur PLAINTEXT ou SSL sans SASL. |
| SASL/PLAIN | Un nom et un mot de passe pour un écouteur SASL_SSL. Exige TLS. |

Adresse : un nom d'hôte ou une IP (`kafka.lan`), `hôte:port`, ou une adresse d'amorçage collée. Options : port (9092), TLS, vérification du certificat, délai (10 s), et Groupes mesurés (50, de 0 à 500).
