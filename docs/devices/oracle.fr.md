# Oracle Database

Oracle Database, lue par Oracle REST Data Services : instance qui n'est pas ouverte, tablespaces qui se remplissent, sessions et processus face à leurs limites, retard Data Guard, zone de récupération rapide et travaux du planificateur en échec.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 8080 | TCP | ORDS (HTTP, par défaut) |
| 8443 | TCP | Seulement si ORDS est servi en TLS (HTTPS, par défaut) |

**Pourquoi ORDS.** Le protocole natif d'Oracle (TNS) n'a aucune implémentation libre en Rust pur, et les pilotes courants passent par OCI, une bibliothèque C propriétaire (Instant Client) qui ne peut pas être livrée dans l'image statique de DumbMonit et dont la licence interdit la redistribution. ORDS est le composant gratuit d'Oracle qui expose la base en HTTP : avec son SQL REST, un `SELECT` part en JSON vers `/ords/<schéma>/_/sql`, ce que fait DumbMonit avec les identifiants du compte. La limite est qu'ORDS doit être déployé devant la base. Sans lui, un script exécuté par l'agent DumbMonit (sqlplus) est l'alternative ; elle n'est pas fournie ici.

Le compte ne porte que `CREATE SESSION` et `SELECT_CATALOG_ROLE` : il lit les vues `V$` et `DBA_`, aucune donnée.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_oracle_`.

| Métrique | Quoi | Étiquettes |
|---|---|---|
| `version_info` | valeur 1 | `version`, `status`, `database_status` |
| `instance_open` | 1 si l'instance est OPEN |  |
| `uptime_seconds` | depuis le démarrage de l'instance |  |
| `database_info` | valeur 1 | `open_mode`, `role`, `log_mode` |
| `tablespace_used_percent` | remplissage de chaque tablespace, par rapport à sa taille maximale | `tablespace` |
| `sessions`, `sessions_used_percent` | sessions, et leur part de la limite `sessions` |  |
| `processes`, `processes_used_percent` | processus, et leur part de la limite `processes` |  |
| `dataguard_transport_lag_seconds`, `dataguard_apply_lag_seconds` | retard Data Guard ; seulement sur un standby |  |
| `recovery_area_used_percent` | remplissage de la zone de récupération rapide ; seulement si elle est configurée |  |
| `scheduler_jobs_failed_24h` | exécutions de travaux du planificateur en échec sur 24 heures |  |

Toute requête après la première est facultative : une vue que le compte ne peut pas lire laisse ses métriques de côté sans faire échouer la vérification. Aucun pourcentage n'est écrit pour une limite `UNLIMITED`.

Les [règles intégrées](../alerting/rules.md#more-databases-and-kafka)
qui s'appliquent : Instance Oracle non ouverte, Tablespace Oracle presque plein, Sessions Oracle proches de la limite, Retard d'application Oracle Data Guard, Zone de récupération Oracle presque pleine, Travail du planificateur Oracle en échec, plus Surveillance inaccessible.

## Laisser DumbMonit lire Oracle par ORDS

1. Vérifiez qu'Oracle REST Data Services (ORDS) tourne devant la base, avec le SQL REST activé.

    ```
    ords config set restEnabledSql.active true
    ```

2. En tant qu'utilisateur privilégié, créez le compte que DumbMonit utilisera, avec les deux droits dont il a besoin.

    ```
    CREATE USER dumbmonit IDENTIFIED BY "choose-a-long-password";
    GRANT CREATE SESSION, SELECT_CATALOG_ROLE TO dumbmonit;
    ```

3. Connecté avec ce compte, activez son schéma pour REST avec l'alias dumbmonit.

    ```
    EXEC ORDS.ENABLE_SCHEMA(p_enabled => TRUE, p_url_mapping_type => 'BASE_PATH', p_url_mapping_pattern => 'dumbmonit');
    ```

4. Dans DumbMonit, saisissez l'adresse d'ORDS, par exemple ords.lan:8080 ou https://ords.lan:8443, puis le compte et son mot de passe.

5. Si l'alias du schéma ou le chemin d'ORDS diffèrent des valeurs par défaut, réglez-les dans les options ci-dessous.

### Droits requis

- **CREATE SESSION** : Laisser le compte se connecter
- **SELECT_CATALOG_ROLE** : Lire les vues V$ et DBA_, aucune donnée

!!! warning
    Il n'y a pas de client Oracle natif : DumbMonit lit la base par ORDS en HTTP, et ne contacte jamais le listener du port 1521. Les réglages du SQL REST varient d'une version d'ORDS à l'autre : consultez la documentation de la vôtre.

### Pourquoi ces choix

`SELECT_CATALOG_ROLE` donne la lecture du dictionnaire de données et des vues de performance, jamais des tables applicatives. Le SQL REST s'authentifie avec le compte de la base : le schéma du compte doit donc être activé pour REST ; l'alias dans l'URL est l'alias du schéma (`dumbmonit` par défaut, le nom du compte en minuscules).

## Identifiants

| Identifiant | Champs |
|---|---|
| Nom d'utilisateur / mot de passe | Le compte de la base créé plus haut, envoyé en authentification HTTP basic à ORDS. |

Adresse : l'hôte ORDS (`ords.lan`), `hôte:port`, ou une URL complète. Options : protocole (HTTP par défaut), port (8080), chemin ORDS (`/ords`), alias du schéma (vide : le nom du compte en minuscules), vérification du certificat, délai (10 s).
