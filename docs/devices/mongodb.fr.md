# MongoDB

La base de données documents elle-même : le replica set (membres, leur état,
un primaire manquant, combien chaque secondaire est en retard), les
connexions par rapport à la limite, le cache WiredTiger, les opérations et
les assertions.

DumbMonit se connecte avec un compte qui détient seulement le rôle intégré
`clusterMonitor`, lance `serverStatus`, et `replSetGetStatus` quand le serveur
appartient à un replica set. Ce rôle lit l'état du serveur et de la
réplication, et aucun document d'aucune base. MongoDB 3.6 et plus récent sont
couverts ; DumbMonit parle le protocole réseau lui-même, sans pilote.

Les pannes à surveiller : un replica set qui a perdu son primaire n'accepte
aucune écriture jusqu'à ce qu'une élection réussisse ; un secondaire qui prend
du retard est celui qui sera élu le jour où le primaire meurt, sans les
dernières écritures ; à la limite de connexions les nouveaux clients sont
refusés ; avec plus de 20 % du cache WiredTiger sale, les requêtes doivent
évincer des pages elles-mêmes et tout ralentit.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_mongodb_`. Les totaux sont des
compteurs : graphiques et règles les transforment en débits.

| Métrique | Quoi | Labels |
|---|---|---|
| `version_info` | valeur 1 | `version`, `process`, `storage_engine` |
| `role_info` | valeur 1 | `role` : `primary`, `secondary`, `arbiter`, `standalone` |
| `uptime_seconds` | | |
| `connections_current`, `connections_available`, `connections_active` | | |
| `connections_used_percent` | actuelles par rapport à actuelles plus disponibles | |
| `connections_created_total`, `connections_rejected_total` | | |
| `operations_total` | opérations depuis le démarrage | `op` : `insert`, `query`, `update`, `delete`, `getmore`, `command` |
| `asserts_total` | assertions depuis le démarrage ; `regular` sont des erreurs internes du serveur, `user` des erreurs renvoyées aux clients | `type` |
| `queued_operations`, `active_clients` | opérations en attente d'un verrou, clients en exécutant une | |
| `wiredtiger_cache_max_bytes`, `wiredtiger_cache_used_bytes`, `wiredtiger_cache_used_percent` | le cache WiredTiger et à quel point il est plein | |
| `wiredtiger_cache_dirty_bytes`, `wiredtiger_cache_dirty_percent` | pages modifiées pas encore écrites | |
| `wiredtiger_cache_read_bytes_total` | octets lus du disque vers le cache | |
| `memory_resident_bytes` | | |
| `network_bytes_in_total`, `network_bytes_out_total` | | |
| `cursors_open`, `cursors_timed_out_total`, `documents_deleted_total` | | |
| `replset_info` | valeur 1, dans un replica set | `set`, `members` |
| `replset_primary_present` | 1 quand un membre est primaire, 0 quand aucun ne l'est | |
| `replset_member_state` | le numéro d'état du membre | `member`, `state` : `primary`, `secondary`, `arbiter`, `recovering`, `down`… |
| `replset_member_health` | 1 quand le membre est accessible, 0 quand il ne l'est pas | `member` |
| `replset_member_lag_seconds` | combien un secondaire est en retard sur le primaire | `member` |

Les [règles intégrées](../alerting/rules.md#databases-message-broker-and-security-engine)
qui s'appliquent : replica set MongoDB sans primaire, membre MongoDB
inaccessible, retard de réplication MongoDB, connexions MongoDB proches de la
limite, cache MongoDB sous pression, erreurs internes MongoDB, plus Surveillance
inaccessible.

## La page de la surveillance

Le panneau au-dessus des graphiques lit ce que la sonde a stocké ; ouvrir la
page n'interroge jamais MongoDB. Il dit ce qui ne va pas en une phrase
(replica set, connexions, cache, erreurs internes), puis les connexions, les
opérations par seconde, le cache utilisé, la mémoire résidente, les
opérations en attente et l'uptime, puis l'état et le retard de chaque membre
du replica set et les opérations par seconde par type.

## Créer un compte MongoDB avec le rôle clusterMonitor

1. Dans mongosh, connecté avec un utilisateur autorisé à créer des utilisateurs, créez un compte avec seulement le rôle intégré clusterMonitor. Il lit l'état du serveur et de la réplication, et aucun document d'aucune base.

    ```
    use admin
    db.createUser({user: "dumbmonit", pwd: passwordPrompt(), roles: [{role: "clusterMonitor", db: "admin"}]})
    ```

2. DumbMonit se connecte avec SCRAM-SHA-256, le défaut pour les comptes créés sur MongoDB 4.0 et plus récent. Si vous avez créé le compte dans une autre base que admin, réglez Base d'authentification ci-dessous.

3. Dans DumbMonit, saisissez l'adresse d'un mongod, par exemple « db1.lan » ou « db1.lan:27018 ». Dans un replica set, ajoutez chaque membre : chacun rapporte ses propres connexions et son cache, et n'importe lequel d'entre eux rapporte l'état et le retard de tous les membres. Si le serveur exige TLS, cochez TLS ci-dessous.

4. DumbMonit ne lance que serverStatus et replSetGetStatus. Il ne liste jamais une collection et ne lit jamais un document.

!!! warning
    Une adresse mongodb+srv:// n'est pas résolue : saisissez plutôt l'hôte de chaque membre. MongoDB antérieur à 3.6 et les routeurs mongos ne sont pas supportés.

## Identifiants

| Identifiant | Champs |
|---|---|
| Nom d'utilisateur / mot de passe | Le compte `clusterMonitor` créé ci-dessus, connecté avec SCRAM-SHA-256. |
| Aucune authentification | Pour un serveur démarré sans contrôle d'accès. |

Un compte qui n'a que des identifiants SCRAM-SHA-1 (créé avant MongoDB 4.0 et
jamais mis à jour) est refusé : mettez à jour son mot de passe une fois avec
`db.updateUser("dumbmonit", {pwd: passwordPrompt(), mechanisms: ["SCRAM-SHA-256"]})`.

Adresse : un nom d'hôte ou une IP (`db1.lan`), `host:port`, ou une chaîne de
connexion `mongodb://` avec un seul hôte. Options : port (27017), base
d'authentification (`admin`), TLS (désactivé), vérification du certificat, et
délai pour toute la vérification (10 s).
