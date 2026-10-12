# Caddy

Le serveur web et reverse proxy : upstreams qui échouent leurs contrôles de
santé, un rechargement de configuration qui a échoué, erreurs de handler et
la part de réponses 5xx.

DumbMonit lit trois choses depuis l'API d'administration de Caddy, toutes en
`GET` : `/config/apps/http/servers` (la configuration HTTP que Caddy fait
tourner, jamais la section `tls` où vivent les jetons de fournisseur DNS),
`/reverse_proxy/upstreams` (les backends et les échecs que le contrôle de
santé passif retient) et `/metrics` (santé des upstreams, rechargements,
requêtes et erreurs).

L'API d'administration de Caddy n'est pas en lecture seule : qui l'atteint
peut charger une nouvelle configuration ou arrêter Caddy. Elle écoute sur
`localhost:2019` pour cette raison. La configuration ci-dessous la garde là
et laisse Caddy lui-même publier une vue en lecture seule d'elle, protégée
par un mot de passe, qui répond en `GET` sur les trois chemins ci-dessus et
refuse tout le reste.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_caddy_`. Les totaux sont des
compteurs : graphiques et règles les transforment en débits.

| Métrique | Quoi | Labels |
|---|---|---|
| `config_loaded` | 1 quand Caddy fait tourner au moins un serveur HTTP | |
| `servers`, `routes` | serveurs HTTP et routes dans la configuration | |
| `config_last_reload_ok` | 0 quand le dernier rechargement de configuration a échoué | |
| `config_last_reload_timestamp_seconds` | heure du dernier rechargement réussi | |
| `upstreams`, `upstreams_unhealthy` | backends de chaque `reverse_proxy`, et ceux en échec | |
| `upstream_healthy` | 1 quand ni le contrôle de santé actif ni le passif n'excluent le backend | `upstream` |
| `upstream_fails` | requêtes en échec que le contrôle de santé passif retient actuellement | `upstream` |
| `upstream_requests` | requêtes en vol vers le backend | `upstream` |
| `requests_total`, `requests_5xx_total`, `request_errors_total` | avec l'option `metrics` : requêtes, réponses 5xx et erreurs de handler | |
| `server_requests_total`, `server_requests_5xx_total`, `server_request_errors_total` | la même chose par serveur (`srv0`, `srv1`… ou les noms que vous avez donnés) | `server` |

`upstream_healthy` a besoin de `/metrics` : le contrôle de santé actif n'est
visible que là. Une vue en lecture seule qui ne laisse pas passer `/metrics`
rapporte quand même les backends et leurs échecs retenus. Caddy ne publie
pas sa version via l'API d'administration.

Les [règles intégrées](../alerting/rules.md#reverse-proxies-and-domains) qui
s'appliquent : upstream Caddy en échec, rechargement de configuration Caddy
échoué, reverse proxy répondant en 5xx, plus Appareil inaccessible.

## La page de l'appareil

Le panneau au-dessus des graphiques lit ce que la sonde a stocké ; ouvrir la
page n'interroge jamais Caddy. Il dit ce qui ne va pas en une phrase (pas de
configuration, un rechargement échoué, des upstreams en échec, une forte
part de 5xx), puis les serveurs, routes, upstreams, débit de requêtes, part
de 5xx et erreurs de handler sur 24 heures, puis chaque upstream avec ses
échecs retenus.

## Publier une vue en lecture seule de l'API d'administration de Caddy

1. L'API d'administration de Caddy peut charger une nouvelle configuration et arrêter Caddy : elle doit rester hors de portée de tout sauf DumbMonit. Laissez-la où Caddy la met, sur localhost:2019, et laissez Caddy publier une vue en lecture seule d'elle : GET seulement, sur les trois chemins que DumbMonit lit, derrière un mot de passe.

2. Créez le hash de mot de passe pour l'utilisateur dumbmonit.

    ```
    caddy hash-password --plaintext 'a-long-password'
    ```

3. Ajoutez ce site au Caddyfile, avec ce hash à la place de HASH, puis rechargez Caddy. Il répond sur le port 2020 et refuse tout sauf ces trois lectures.

    ```
    :2020 {
    basic_auth {
    dumbmonit HASH
    }
    @readonly {
    method GET
    path /config/apps/http/servers /reverse_proxy/upstreams /metrics
    }
    handle @readonly {
    reverse_proxy localhost:2019 {
    header_up Host localhost:2019
    }
    }
    respond 403
    }
    ```

4. Optionnel : ajoutez metrics au bloc d'options globales en haut du Caddyfile pour compter les requêtes, erreurs et réponses 5xx par serveur. La santé des upstreams et les rechargements sont rapportés sans cela.

    ```
    metrics
    ```

5. Filtrez le port 2020 pour que seul l'hôte DumbMonit l'atteigne. Dans DumbMonit, saisissez l'hôte Caddy avec ce port, par exemple « caddy.lan:2020 », et l'utilisateur dumbmonit avec son mot de passe.

!!! warning
    Ne publiez jamais l'API d'administration elle-même sur le réseau sans pare-feu devant : qui atteint le port 2019 peut remplacer la configuration ou arrêter Caddy, et l'option origins n'arrête pas un client qui n'envoie aucun en-tête Origin. Si vous l'exposez quand même sur un réseau privé, filtrez le port pour que seul l'hôte DumbMonit l'atteigne, et choisissez Pas de mot de passe.

Le Caddyfile garde son sens sans indentation ; `caddy fmt` l'indente.
`header_up Host localhost:2019` est nécessaire parce que l'API
d'administration ne répond qu'aux requêtes adressées à l'adresse sur
laquelle elle écoute. Avec l'image conteneur de Caddy, lancez la commande de
hash avec `docker exec`.

Une configuration chargée en JSON plutôt que depuis un Caddyfile obtient le
même site comme serveur supplémentaire avec un handler `basic_auth`, un
matcher `method` et `path` et un `reverse_proxy` vers `localhost:2019`.

## Identifiants

| Identifiant | Champs |
|---|---|
| Vue en lecture seule | L'utilisateur et le mot de passe du bloc `basic_auth` ci-dessus (recommandé). |
| Pas de mot de passe | Rien : l'API d'administration elle-même, filtrée pour que seul DumbMonit l'atteigne. |

Adresse : un nom d'hôte ou une IP avec le port de la vue en lecture seule
(`caddy.lan:2020`), ou une URL complète. Options : protocole (HTTP par
défaut), port (2019, l'API d'administration elle-même), vérification du
certificat et délai de requête (10 s).

Le contrôle de santé passif (`fail_duration`) ne retient les échecs que
pendant cette durée : un backend qui échoue une fois et se rétablit est
rapporté en échec jusque là. La règle attend cinq minutes avant de se
déclencher.
