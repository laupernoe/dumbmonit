# Keycloak (beta)

Si Keycloak peut vraiment connecter les gens : disponibilité avec le contrôle de la base, version, realms (un realm désactivé, c'est une population verrouillée dehors) et, avec un compte de service, sessions actives et utilisateurs par realm.

!!! note "Bêta"
    Écrit d'après la documentation d'API de l'éditeur et testé sur des réponses réalistes enregistrées, pas encore sur une instance réelle. Dites-nous ce que vous observez.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 8080 | TCP | API (HTTP, par défaut) |
| 9000 | TCP | Contrôles de santé sur le port de gestion (Keycloak 25 et suivants) |
| 443 | TCP | Seulement avec le schéma https derrière un reverse proxy |

La panne à surveiller : **Keycloak debout, base de données tombée**. La page de connexion s'affiche encore, personne ne peut se connecter, et toutes les applications derrière Keycloak semblent cassées d'un coup. `/health/ready` est l'endroit où Keycloak le dit ; un `503` y est une mesure, pas une erreur de transport.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_keycloak_`.

| Métrique | Quoi | Labels |
|---|---|---|
| `ready`, `live` | 1 tant que le contrôle de disponibilité / de vie indique UP |  |
| `health_check_up` | chaque contrôle de disponibilité (base de données…), 32 au plus | `check` |
| `admin_readable` | 1 quand le compte de service a pu lire l'API d'administration (absent sans identifiants) |  |
| `version_info` | valeur 1 | `version` |
| `uptime_seconds`, `memory_used_percent` | temps depuis le démarrage ; mémoire JVM utilisée |  |
| `realms_total`, `realms_disabled` | realms, et ceux désactivés |  |
| `realm_sessions`, `realm_offline_sessions`, `realm_users` | sessions actives, sessions hors ligne et utilisateurs de chaque realm lu | `realm` |
| `sessions_active` | sessions actives, tous les realms lus |  |
| `scrape_errors` | appels qui ont échoué à la dernière sonde |  |

Les [règles intégrées](../alerting/rules.md#self-hosted-applications) qui s'appliquent : Keycloak not ready, Keycloak realm disabled, plus Appareil inaccessible.

## Laisser DumbMonit lire la santé et les realms de Keycloak

1. Lancez Keycloak avec les contrôles de santé activés, avec l'option ci-dessous. Depuis Keycloak 25, ils répondent sur le port de gestion 9000.

    ```
    KC_HEALTH_ENABLED=true
    ```

2. Facultatif, pour les realms, sessions et utilisateurs : dans le realm master, créez un client nommé comme suit, avec Client authentication et Service accounts roles activés, et Standard flow désactivé.

    ```
    dumbmonit
    ```

3. Sur l'onglet Service accounts roles de ce client, attribuez les rôles de client listés ci-dessous, puis copiez le secret de l'onglet Credentials.

4. Dans DumbMonit, saisissez l'adresse de Keycloak et l'identifiant et le secret du client, ou choisissez Aucun (contrôles de santé seulement).

### Droits requis

- **view-realm** : Master-realm client role: realms and their state
- **view-users** : Master-realm client role: user counts

!!! warning
    Keycloak 24 et antérieurs servent les contrôles de santé sur le port principal : réglez l'option de port de santé sur same. Les contrôles de santé doivent être activés sur le serveur, sinon DumbMonit signale un /health/ready absent.

## Identifiants

| Identifiant | Champs |
|---|---|
| Aucun (contrôles de santé seulement) | Rien n'est envoyé ; les contrôles de santé sont publics. |
| Identifiant / secret du client | Le client de compte de service. DumbMonit demande un jeton (`client_credentials`) sur `/realms/<realm>/protocol/openid-connect/token` à chaque sonde. |

Adresse : le nom d'hôte sur lequel répond Keycloak (`sso.lan`), `host:port`, ou une URL complète (`https://sso.example.com`, avec `/auth` sur les anciennes installations). Les contrôles de santé sont lus sur le même hôte, sur le port de santé. Options : protocole, port, port de santé (`health_port`, 9000 ; `same` pour Keycloak 24 et antérieurs), protocole de santé (`health_scheme`), realm du client (`auth_realm`, master), realms à lire (`realms`), vérification du certificat et délai de requête (15 s).
