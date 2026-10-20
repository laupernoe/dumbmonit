# Nginx

Le serveur web et reverse proxy : connexions actives et en attente, les
débits accept/handled/request, et la santé des upstreams quand l'API JSON
commerciale NGINX Plus est disponible.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 80 | TCP | Page de statut (HTTP, par défaut) |
| 443 | TCP | Seulement si la page de statut est servie en HTTPS |

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

## La page de la surveillance

Le panneau au-dessus des graphiques lit ce que la sonde a stocké ; ouvrir la
page n'interroge jamais Nginx. Il montre les connexions actives, en lecture,
en écriture et en attente, les débits accept/handled/request sur 24 heures,
et, avec NGINX Plus, la santé des upstreams et le trafic par zone.

## Activer le module stub_status de Nginx

1. Ajoutez ce location au bloc server déjà servi par Nginx, restreint à l'hôte DumbMonit (adaptez l'adresse).

    ```
    location /basic_status {
    stub_status;
    allow 10.0.0.5;
    deny all;
    }
    ```

2. Rechargez Nginx.

    ```
    nginx -s reload
    ```

3. Ouvrez http://nginx.lan/basic_status depuis l'hôte DumbMonit : la page doit afficher Active connections.

4. Dans DumbMonit, saisissez l'hôte Nginx, par exemple nginx.lan, et le chemin s'il n'est pas /basic_status.

5. Si NGINX Plus est installé, cochez Lire l'API NGINX Plus pour ajouter la santé des upstreams et les compteurs par zone.

!!! warning
    stub_status ne compte que les connexions et les requêtes : une erreur de configuration dans un bloc server ou un backend injoignable n'y apparaît pas.

### Pour aller plus loin

Nginx fournit stub_status en natif : il suffit d'un location qui l'active, dans le bloc server déjà servi ou dans un bloc server à part sur un autre port. Les lignes allow et deny le restreignent à l'hôte DumbMonit, pour que personne d'autre ne puisse le lire. La version open source ne connaît pas la notion d'upstream : elle n'existe que dans NGINX Plus, lue par sa propre API quand elle est activée.

Documentation de l'éditeur : https://nginx.org/en/docs/http/ngx_http_stub_status_module.html

## Identifiants

| Identifiant | Champs |
|---|---|
| Pas de mot de passe | Rien : la page de statut est filtrée pour que seul DumbMonit l'atteigne. |
| Authentification basique | L'utilisateur et le mot de passe du bloc protégeant la page de statut, s'il y en a. |

Adresse : un nom d'hôte ou une IP, avec le port s'il n'est pas 80. Options :
protocole (HTTP par défaut), port, chemin de statut (`/basic_status` par
défaut), vérification du certificat, délai de requête (10 s), et
l'interrupteur et la version de l'API NGINX Plus.
