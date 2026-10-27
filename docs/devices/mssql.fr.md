# SQL Server

Le moteur de base de données : bases qui ne sont pas en ligne, requêtes bloquées derrière une transaction oubliée, sessions face à la limite, espace libre des volumes de données, travaux de l'Agent SQL Server dont la dernière exécution a échoué, et groupes de disponibilité AlwaysOn.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 1433 | TCP | SQL Server (TDS, port par défaut) |

DumbMonit se connecte avec une connexion SQL Server qui n'a que des droits de lecture au niveau du serveur : elle voit l'état du moteur, et ne peut lire aucune table, ni écrire, ni rien changer. L'authentification Windows n'est pas prise en charge.

La panne à surveiller est la silencieuse : une transaction laissée ouverte bloque toutes les sessions qui veulent les mêmes lignes, sans rien écrire dans les journaux du serveur. Les autres sont un volume plein quand les fichiers veulent grossir, une sauvegarde en échec depuis une semaine, et une réplique de disponibilité qui ne se synchronise plus.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_mssql_`.

| Métrique | Quoi | Étiquettes |
|---|---|---|
| `version_info` | valeur 1 | `version`, `edition` |
| `uptime_seconds` | depuis le démarrage du service |  |
| `sessions`, `sessions_used_percent` | sessions utilisateur, et part de la limite de connexions quand il y en a une |  |
| `blocked_requests`, `longest_block_seconds` | requêtes en attente d'un verrou, et l'attente la plus longue |  |
| `databases`, `databases_offline` | bases, et celles en SUSPECT, RECOVERY_PENDING, OFFLINE ou EMERGENCY |  |
| `database_online`, `database_read_only` | par base ; 1 si elle est en ligne (ou dans un état transitoire normal) | `database`, `state` |
| `database_size_bytes` | taille des fichiers de données | `database` |
| `volume_free_bytes`, `volume_used_percent` | espace de chaque volume qui porte des fichiers de bases | `volume` |
| `agent_job_last_ok`, `agent_jobs_failed` | 1 si la dernière exécution d'un travail actif de l'Agent n'a pas échoué ; seulement avec l'Agent SQL Server | `job` |
| `alwayson_sync_health`, `alwayson_suspended`, `alwayson_unhealthy` | santé de synchronisation par réplique et par base (0 mauvaise, 1 partielle, 2 saine) ; seulement si AlwaysOn est activé | `replica`, `database` |

Les requêtes de l'Agent et d'AlwaysOn sont facultatives : une édition Express ou une instance sans groupe de disponibilité n'a simplement pas ces séries.

Les [règles intégrées](../alerting/rules.md#more-databases-and-kafka)
qui s'appliquent : Base SQL Server indisponible, Requêtes SQL Server bloquées, Volume SQL Server presque plein, Travail de l'Agent SQL Server en échec, Réplique AlwaysOn SQL Server dégradée, Connexions SQL Server proches de la limite, plus Surveillance inaccessible.

## Créer une connexion SQL Server en lecture seule

1. Connectez-vous à l'instance avec SQL Server Management Studio ou sqlcmd, avec une connexion autorisée à créer des connexions, et créez celle que DumbMonit utilisera.

    ```
    CREATE LOGIN dumbmonit WITH PASSWORD = 'choose-a-long-password', CHECK_POLICY = ON;
    ```

2. Donnez-lui les droits de lecture au niveau du serveur dont elle a besoin pour voir l'état du moteur, et rien d'autre.

    ```
    GRANT VIEW SERVER STATE TO dumbmonit;
    GRANT VIEW ANY DEFINITION TO dumbmonit;
    ```

3. Facultatif, pour les travaux de l'Agent SQL Server : laissez-la lire l'historique des travaux dans msdb.

    ```
    USE msdb;
    CREATE USER dumbmonit FOR LOGIN dumbmonit;
    ALTER ROLE SQLAgentReaderRole ADD MEMBER dumbmonit;
    ```

4. Vérifiez que l'instance accepte l'authentification SQL Server (mode mixte) et écoute sur un port TCP fixe : SQL Server Configuration Manager, Configuration réseau, TCP/IP.

5. Dans DumbMonit, saisissez l'adresse du serveur, par exemple sql.lan, ou sql.lan:1444 pour un autre port, puis la connexion et son mot de passe.

6. Cochez Accepter un certificat invérifiable tant que le serveur présente encore son certificat auto-signé.

### Droits requis

- **VIEW SERVER STATE** : Lire les sessions, les requêtes et les volumes
- **VIEW ANY DEFINITION** : Lister les bases et leurs fichiers
- **SQLAgentReaderRole (msdb)** : Facultatif : lire l'historique des travaux de l'Agent

!!! warning
    L'authentification Windows n'est pas prise en charge : utilisez une connexion SQL Server, sur une instance qui autorise l'authentification SQL Server (mode mixte). Les instances nommées demandent leur port TCP fixe dans l'adresse.

### Pourquoi ces choix

Les deux premières permissions sont au niveau du serveur et ne donnent accès qu'aux vues de gestion dynamique et aux métadonnées. Une base en RESTORING n'est pas comptée indisponible, puisque c'est l'état normal du secondaire d'un log shipping. La connexion n'a jamais besoin d'utilisateur de base, sauf dans `msdb` pour l'historique des travaux de l'Agent.

## Identifiants

| Identifiant | Champs |
|---|---|
| Connexion SQL Server | La connexion créée plus haut (nom et mot de passe), envoyée dans le paquet de login TDS sous TLS. |

Adresse : un nom d'hôte ou une IP (`sql.lan`), `hôte:port`, ou une chaîne de connexion collée sous la forme `mssql://sql.lan:1444`. Options : port (1433), base (`master`), TLS (activé), vérification du certificat, délai (10 s).
