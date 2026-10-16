# Redis et Valkey

Le stockage en mémoire lui-même : mémoire par rapport à `maxmemory`,
connexions refusées, le lien de réplication et le retard de chaque réplica,
snapshots et écritures du fichier append-only en échec, et la taille de
l'espace de clés.

DumbMonit ouvre une connexion, envoie `AUTH`, puis `INFO`, et la ferme. Le
compte qu'il utilise peut lancer `INFO` et `PING` et rien d'autre : il ne peut
ni lire, ni écrire ni même lister une clé. Redis 6 et plus récent, Valkey et
les forks qui répondent à `INFO` de la même façon (KeyDB, Dragonfly) sont
couverts.

Les pannes à surveiller sont les silencieuses. À `maxmemory`, Redis évince des
clés sans le dire à personne ou refuse chaque écriture avec `OOM command not
allowed`, selon `maxmemory-policy` ; sans `maxmemory`, il grandit jusqu'à ce
que le noyau le tue. Un snapshot en échec fait refuser les écritures par
Redis tant que `stop-writes-on-bgsave-error` est actif, ce qui est le défaut.
Un réplica dont le lien avec son primaire est tombé continue de répondre aux
lectures, avec des données périmées.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_redis_`. Les totaux sont des
compteurs : graphiques et règles les transforment en débits.

| Métrique | Quoi | Labels |
|---|---|---|
| `version_info` | valeur 1 | `product` (`redis`, `valkey`), `version`, `mode` |
| `role_info` | valeur 1 | `role` : `master` ou `replica` |
| `memory_used_bytes`, `memory_rss_bytes`, `memory_peak_bytes`, `memory_fragmentation_ratio` | mémoire comme Redis la compte, et comme le système la compte | |
| `memory_max_bytes`, `memory_used_percent` | `maxmemory` et la part utilisée ; absent quand `maxmemory` est 0 (pas de limite) | |
| `maxmemory_policy_info` | valeur 1 | `policy` |
| `connected_clients`, `blocked_clients`, `max_clients` | | |
| `rejected_connections_total` | connexions refusées parce que `maxclients` était atteint | |
| `evicted_keys_total`, `expired_keys_total` | clés évincées à `maxmemory`, clés expirées | |
| `keyspace_hits_total`, `keyspace_misses_total` | recherches qui ont trouvé leur clé, et celles qui ne l'ont pas | |
| `commands_processed_total`, `connections_received_total`, `ops_per_second` | | |
| `error_replies_total`, `acl_denied_auth_total` | réponses d'erreur envoyées, et connexions refusées (un mot de passe en train d'être deviné se montre ici) | |
| `keys`, `keys_expiring` | clés dans toutes les bases, et celles avec une expiration ; toujours présent, même à zéro | |
| `db_keys` | clés par base | `db` |
| `rdb_last_save_ok` | 0 quand le dernier snapshot (`BGSAVE`) a échoué | |
| `rdb_changes_since_last_save`, `rdb_last_save_timestamp_seconds` | | |
| `aof_enabled`, `aof_last_write_ok`, `aof_last_rewrite_ok` | fichier append-only ; les deux statuts seulement quand il est activé | |
| `loading` | 1 pendant que Redis charge son jeu de données au démarrage | |
| `connected_replicas` | réplicas attachés à ce primaire | |
| `replica_online`, `replica_lag_seconds`, `replica_lag_bytes` | vu depuis le primaire, par réplica : en ligne ou non, secondes depuis son dernier acquittement, octets de retard | `replica` (`ip:port`) |
| `master_link_up`, `master_last_io_seconds`, `master_link_down_seconds`, `master_sync_in_progress` | sur un réplica : son lien vers le primaire | |
| `uptime_seconds`, `cluster_enabled` | | |

Les [règles intégrées](../alerting/rules.md#databases-message-broker-and-security-engine)
qui s'appliquent : mémoire Redis proche de maxmemory, Redis refuse des
connexions, lien de réplication Redis tombé, réplica Redis en retard, snapshot
Redis en échec, écriture du fichier append-only Redis en échec, plus Surveillance
inaccessible.

## La page de la surveillance

Le panneau au-dessus des graphiques lit ce que la sonde a stocké ; ouvrir la
page n'interroge jamais Redis. Il dit ce qui ne va pas en une phrase
(réplication, mémoire par rapport à `maxmemory` et la politique qui
s'applique à la limite, persistance, connexions refusées), puis les clés, la
mémoire, les clients, les commandes par seconde, le taux de succès, les clés
évincées dans les dernières 24 heures et l'uptime, puis les clés par base et
le retard de chaque réplica.

## Créer un utilisateur Redis qui ne peut lancer qu'INFO

1. Sur Redis 6 ou plus récent, ou Valkey, créez un utilisateur qui peut lancer INFO et PING et rien d'autre, sans accès à aucune clé. Dans redis-cli, connecté avec un utilisateur autorisé à gérer les ACL :

    ```
    ACL SETUSER dumbmonit on >a-long-password -@all +info +ping
    ```

2. Rendez-le permanent : ACL SAVE si le serveur charge ses utilisateurs depuis un aclfile, CONFIG REWRITE sinon.

    ```
    ACL SAVE
    ```

3. Sur Redis 5 ou plus ancien, ou avec seulement requirepass défini, choisissez Mot de passe seul ci-dessous. Ce mot de passe porte tous les droits : préférez un utilisateur ACL dès que le serveur en supporte un.

4. Dans DumbMonit, saisissez l'adresse du serveur, par exemple « cache.lan » ou « cache.lan:6380 ». Si le serveur n'accepte que TLS sur ce port (tls-port), cochez TLS ci-dessous.

5. DumbMonit envoie AUTH, puis INFO. Il ne lit, n'écrit ni ne liste jamais une clé.

!!! warning
    Redis ne donne aucun avertissement avant maxmemory : à la limite il évince des clés silencieusement ou refuse chaque écriture, selon maxmemory-policy. Sans maxmemory il grandit jusqu'à ce que le noyau le tue. DumbMonit avertit au-dessus de 90 % de maxmemory.

## Identifiants

| Identifiant | Champs |
|---|---|
| Utilisateur ACL (recommandé) | L'utilisateur créé ci-dessus et son mot de passe, envoyés comme `AUTH user password`. |
| Mot de passe seul | La valeur de `requirepass`, envoyée comme `AUTH password`. |
| Aucune authentification | Pour un serveur sans mot de passe du tout. |

Adresse : un nom d'hôte ou une IP (`cache.lan`), `host:port`, ou une chaîne de
connexion (`redis://cache.lan:6380` ; tout ce qui suit le port est ignoré, et
les identifiants qui y sont écrits ne sont jamais utilisés). Options : port
(6379), TLS (désactivé), vérification du certificat, et délai pour toute la
vérification (10 s). Avec TLS activé, le certificat est vérifié contre les
autorités publiques intégrées à DumbMonit ; pour une autorité privée, cochez
Accepter un certificat non vérifiable.

Dans un déploiement Redis Sentinel ou Cluster, ajoutez chaque serveur que vous
voulez surveiller : chacun rapporte sa propre mémoire, ses clients et sa
persistance, et un primaire rapporte ses réplicas. Sentinel lui-même n'est
pas couvert.
