# Nginx

Le serveur web et reverse proxy : connexions actives et en attente, les
débits accept/handled/request, et la santé des upstreams quand l'API JSON
commerciale NGINX Plus est disponible.

DumbMonit lit le module `stub_status` intégré de Nginx, une poignée de
compteurs en texte brut, sans API et sans compte nécessaire sur un réseau
correctement filtré. Quand le NGINX Plus commercial est installé, DumbMonit
peut aussi lire son API JSON pour la santé des upstreams et les compteurs par
zone.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_nginx_`. Les métriques `_total`
sont des compteurs : graphiques et règles les transforment en débits.

| Métrique | Quoi | Labels |
|---|---|---|
| `connections_active` | connexions actuellement ouvertes | |
| `connections_reading`, `connections_writing`, `connections_waiting` | connexions par ce qu'elles font | |
| `accepts_total`, `handled_total` | connexions acceptées, et effectivement traitées | |
| `requests_total` | requêtes servies depuis le démarrage de Nginx | |
| `version_info` | présence ; l'en-tête de réponse `Server`, quand Nginx en envoie un | `server` |
| `plus_upstream_server_up` | 1 quand NGINX Plus marque le serveur actif | `upstream`, `server` |
| `plus_upstream_server_active`, `plus_upstream_server_requests_total`, `plus_upstream_server_fails_total`, `plus_upstream_server_5xx_total` | compteurs NGINX Plus par serveur | `upstream`, `server` |
| `plus_zone_requests_total`, `plus_zone_5xx_total`, `plus_zone_received_bytes_total`, `plus_zone_sent_bytes_total` | compteurs NGINX Plus par zone | `zone` |

Un écart entre `accepts_total` et `handled_total` signifie des connexions que
Nginx a rejetées avant de pouvoir les servir (généralement
`worker_connections` atteint). Les métriques `plus_*` n'apparaissent que
quand « Lire l'API NGINX Plus » est cochée et que NGINX Plus répond ; la
version open source n'a pas d'« upstream » à rapporter avec `stub_status`
seul.

## La page de l'appareil

Le panneau au-dessus des graphiques lit ce que la sonde a stocké ; ouvrir la
page n'interroge jamais Nginx. Il montre les connexions actives, en lecture,
en écriture et en attente, les débits accept/handled/request sur 24 heures,
et, avec NGINX Plus, la santé des upstreams et le trafic par zone.

## Activer le module stub_status de Nginx

1. Nginx fournit stub_status intégré : il a seulement besoin d'un emplacement qui l'active. Ajoutez ceci au bloc serveur que Nginx sert déjà (ou à son propre bloc serveur sur un port qui lui est propre), puis rechargez Nginx.

    ```
    location /basic_status {
    stub_status;
    }
    ```

2. Restreignez cet emplacement à l'hôte DumbMonit, pour que personne d'autre ne puisse le lire ; ajustez l'adresse en conséquence.

    ```
    location /basic_status {
    stub_status;
    allow 10.0.0.5;
    deny all;
    }
    nginx -s reload
    ```

3. Vérifiez qu'il répond avant de configurer DumbMonit.

    ```
    curl -s http://nginx.lan/basic_status
    ```

4. Dans DumbMonit, saisissez l'hôte Nginx, par exemple « nginx.lan », et le chemin s'il n'est pas « /basic_status ». Si NGINX Plus est installé, cochez « Lire l'API NGINX Plus » pour ajouter la santé des upstreams et les compteurs par zone.

!!! warning
    stub_status ne compte que les connexions et les requêtes : une erreur de
    configuration dans un bloc serveur, ou un backend que nginx ne peut pas
    atteindre, n'apparaît pas ici. La version open source n'a aucune notion
    d'« upstream » : cela n'existe que dans NGINX Plus, lu via sa propre API
    quand elle est activée.

L'exemple ci-dessus restreint l'emplacement par IP, la configuration la plus
simple pour un homelab ; un bloc `basic_auth` fonctionne tout aussi bien si la
page de statut ne peut pas se trouver derrière une règle de pare-feu qui lui
est propre. `stub_status` lui-même ne peut pas être protégé par mot de passe
directement par Nginx — le mot de passe, s'il y en a un, appartient au bloc
`location` qui l'entoure.

## Identifiants

| Identifiant | Champs |
|---|---|
| Pas de mot de passe | Rien : la page de statut est filtrée pour que seul DumbMonit l'atteigne. |
| Authentification basique | L'utilisateur et le mot de passe du bloc protégeant la page de statut, s'il y en a. |

Adresse : un nom d'hôte ou une IP, avec le port s'il n'est pas 80. Options :
protocole (HTTP par défaut), port, chemin de statut (`/basic_status` par
défaut), vérification du certificat, délai de requête (10 s), et
l'interrupteur et la version de l'API NGINX Plus.
