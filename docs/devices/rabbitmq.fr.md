# RabbitMQ

Le courtier de messages : alarmes mémoire et disque, nœuds arrêtés et
partitions réseau, files qui se remplissent sans que personne ne les lise,
files qui ne tournent plus, et le flux des messages.

DumbMonit lit l'API de management avec un utilisateur étiqueté `monitoring`
qui a des permissions vides : il voit l'état des nœuds, les alarmes et les
files, et ne peut ni publier, ni consommer, ni lire un message ni rien
changer. Un seul nœud rapporte tout le cluster.

La panne à surveiller est celle que les clients ne rapportent pas. Quand un
nœud dépasse sa limite haute de mémoire ou sa limite de disque libre,
RabbitMQ lève une alarme et bloque tous les publieurs du cluster : les
applications restent simplement bloquées sur `publish`, sans erreur. La
seconde est une file qui continue de se remplir parce que le service qui la
consommait s'est arrêté.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_rabbitmq_`. Les totaux sont des
compteurs : graphiques et règles les transforment en débits.

| Métrique | Quoi | Labels |
|---|---|---|
| `version_info` | valeur 1 | `version`, `erlang`, `cluster` |
| `connections`, `channels`, `consumers`, `queues` | totaux du cluster | |
| `messages_ready`, `messages_unacked` | messages en attente dans chaque file, et livrés mais pas encore acquittés | |
| `messages_published_total`, `messages_delivered_total`, `messages_redelivered_total`, `messages_unroutable_total` | flux de messages ; unroutable sont des messages qu'aucune file n'était liée à recevoir | |
| `alarm` | 1 tant que l'alarme est levée ; mémoire et disque sont toujours présents, à 0 sans alarme | `node`, `resource` |
| `alarms` | alarmes levées dans le cluster | |
| `nodes`, `nodes_running`, `partitions` | | |
| `node_running`, `node_partitions` | par nœud | `node` |
| `node_memory_used_bytes`, `node_memory_limit_bytes`, `node_memory_used_percent` | mémoire par rapport à la limite haute | `node` |
| `node_disk_free_bytes`, `node_disk_free_limit_bytes` | disque libre par rapport à la limite | `node` |
| `node_fd_used`, `node_fd_used_percent`, `node_uptime_seconds`, `node_being_drained` | | `node` |
| `queue_messages_ready`, `queue_messages_unacked`, `queue_consumers`, `queue_running` | par file, les plus pleines d'abord, jusqu'à Files surveillées une par une | `vhost`, `queue` |
| `queues_without_consumers`, `queues_not_running` | parmi ces files | |

Les alarmes sont lues depuis le contrôle de santé `/api/health/checks/alarms`,
qui les voit tout de suite ; les statistiques des nœuds ont quelques secondes
de retard.

Les [règles intégrées](../alerting/rules.md#databases-message-broker-and-security-engine)
qui s'appliquent : alarme mémoire ou disque RabbitMQ, nœud RabbitMQ arrêté,
partition réseau RabbitMQ, file RabbitMQ sans consommateur, file RabbitMQ
indisponible, plus Surveillance inaccessible.

## La page de la surveillance

Le panneau au-dessus des graphiques lit ce que la sonde a stocké ; ouvrir la
page n'interroge jamais RabbitMQ. Il dit ce qui ne va pas en une phrase
(alarmes, nœuds et partitions, files contenant des messages sans
consommateur, files qui ne tournent pas), puis les messages prêts et non
acquittés, les messages publiés et livrés par seconde, les connexions, les
consommateurs et les files, puis les dix files les plus pleines avec leurs
consommateurs et la mémoire et le disque libre de chaque nœud.

## Créer un utilisateur RabbitMQ étiqueté monitoring

1. Activez le plugin de management s'il n'est pas déjà actif (les images Docker de management l'ont). Son API écoute sur le port 15672.

    ```
    rabbitmq-plugins enable rabbitmq_management
    ```

2. Créez un utilisateur avec seulement l'étiquette monitoring : il peut lire l'état des nœuds, des alarmes et des files, et ne peut rien changer.

    ```
    rabbitmqctl add_user dumbmonit 'a-long-password'
    rabbitmqctl set_user_tags dumbmonit monitoring
    ```

3. Donnez-lui des permissions vides sur chaque hôte virtuel dont vous voulez voir les files. Il peut alors lister ces files, et ne peut toujours ni publier, ni consommer ni lire un message. Répétez avec -p pour chaque autre hôte virtuel.

    ```
    rabbitmqctl set_permissions -p / dumbmonit '^$' '^$' '^$'
    ```

4. Dans DumbMonit, saisissez l'adresse d'un nœud, par exemple « rabbit.lan », ou « https://rabbit.lan:15671 » quand l'API est servie sur TLS. Un seul nœud rapporte tout le cluster : les alarmes et partitions de chaque nœud, et chaque file.

!!! warning
    Une alarme mémoire ou disque bloque tous les publieurs du cluster, et les
    clients n'obtiennent aucune erreur : les applications restent simplement
    bloquées sur publish. Les files sont listées les plus pleines d'abord,
    jusqu'à Files surveillées une par une (100 par défaut) ; les totaux
    couvrent toujours toutes les files.

## Identifiants

| Identifiant | Champs |
|---|---|
| Nom d'utilisateur / mot de passe | L'utilisateur `monitoring` créé ci-dessus, envoyé en authentification HTTP basique. |

Sans permissions sur un hôte virtuel, ses files ne sont pas listées une par
une ; les totaux du cluster les comptent quand même.

Adresse : un nom d'hôte ou une IP (`rabbit.lan`), `host:port`, ou une URL
complète avec un préfixe de chemin derrière un reverse proxy. Options :
protocole (HTTP par défaut), port (15672), vérification du certificat, délai
de requête (10 s), et Files surveillées une par une (100, de 0 à 500).
