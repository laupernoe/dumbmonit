# Elasticsearch / OpenSearch

Elasticsearch et OpenSearch : état du cluster (rouge ou jaune), shards non alloués, disques qui se remplissent nœud par nœud, et pression sur le tas JVM.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 9200 | TCP | API REST (HTTP ou HTTPS, par défaut) |

DumbMonit lit l'API REST avec un utilisateur qui ne porte que le privilège de cluster `monitor` : il voit la santé et les statistiques des nœuds, aucun document d'aucun index. Les deux moteurs parlent le même dialecte pour cela, un seul type d'équipement les couvre.

Un cluster `red` a au moins un shard primaire non alloué : une partie des données ne peut ni être lue ni écrite. Un cluster `yellow` a perdu des réplicas : la prochaine panne d'un nœud perdra des données. La panne discrète est le disque : à 95 % (flood stage), le moteur passe les index en lecture seule et les écritures sont refusées.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_elasticsearch_`.

| Métrique | Quoi | Étiquettes |
|---|---|---|
| `version_info` | valeur 1 | `version`, `cluster`, `distribution` |
| `cluster_status` | 0 vert, 1 jaune, 2 rouge |  |
| `nodes`, `data_nodes` | nœuds du cluster |  |
| `active_primary_shards`, `active_shards`, `active_shards_percent` | shards alloués |  |
| `unassigned_shards`, `delayed_unassigned_shards`, `initializing_shards`, `relocating_shards` | shards pas encore en service |  |
| `pending_tasks` | tâches d'état du cluster en attente |  |
| `node_disk_used_percent`, `node_disk_free_bytes`, `node_disk_total_bytes` | le disque de données de chaque nœud | `node` |
| `node_heap_used_percent`, `node_cpu_percent` | tas JVM et CPU de chaque nœud | `node` |

Les statistiques par nœud sont facultatives : un utilisateur qui n'a pas le droit de les lire garde la santé du cluster, sans les séries par nœud.

Les [règles intégrées](../alerting/rules.md#more-databases-and-kafka)
qui s'appliquent : Cluster Elasticsearch rouge, Cluster Elasticsearch jaune, Shards Elasticsearch non alloués, Disque d'un nœud Elasticsearch presque plein, Tas JVM Elasticsearch saturé, plus Surveillance inaccessible.

## Créer un utilisateur Elasticsearch de supervision

1. Créez un rôle qui ne porte que le privilège de cluster monitor, depuis Kibana (Stack Management, Roles) ou avec l'API.

    ```
    curl -u elastic -X PUT "https://es.lan:9200/_security/role/dumbmonit" -H "Content-Type: application/json" -d '{"cluster":["monitor"]}'
    ```

2. Créez l'utilisateur avec ce rôle.

    ```
    curl -u elastic -X PUT "https://es.lan:9200/_security/user/dumbmonit" -H "Content-Type: application/json" -d '{"password":"choose-a-long-password","roles":["dumbmonit"]}'
    ```

3. OpenSearch : créez un rôle avec la permission de cluster cluster_monitor dans Security, Roles, et associez-y l'utilisateur.

4. Dans DumbMonit, saisissez l'adresse d'un nœud, par exemple es.lan, puis l'utilisateur et son mot de passe.

5. Elasticsearch 8 et suivants servent du HTTPS avec un certificat généré : choisissez HTTPS, et cochez Accepter un certificat invérifiable sauf si vous avez installé le vôtre.

### Droits requis

- **monitor (cluster privilege)** : Lire la santé du cluster et les statistiques des nœuds
- **cluster_monitor (OpenSearch)** : La même permission sur OpenSearch

!!! warning
    Le compte n'a besoin d'aucun privilège sur les index : DumbMonit ne lit jamais un document. Un nœud rapporte tout le cluster, saisissez donc un nœud qui n'est pas celui dont vous attendez la panne.

### Pourquoi ces choix

`monitor` est le privilège de cluster qui couvre `/_cluster/health` et `/_nodes/stats`. L'utilisateur n'a volontairement aucun privilège sur les index. L'état est un nombre pour qu'une règle puisse le comparer : rouge est au-dessus de 1, jaune vaut 1.

## Identifiants

| Identifiant | Champs |
|---|---|
| Nom d'utilisateur / mot de passe | L'utilisateur créé plus haut, envoyé en authentification HTTP basic. |
| Jeton porteur | Un jeton de compte de service, envoyé en jeton porteur. Les clés d'API d'Elasticsearch utilisent un autre schéma et ne sont pas prises en charge. |
| Aucune authentification | Un cluster dont la sécurité est désactivée. |

Adresse : un nom d'hôte ou une IP (`es.lan`), `hôte:port`, ou une URL complète. Options : protocole (HTTP par défaut), port (9200), vérification du certificat, délai (10 s).
