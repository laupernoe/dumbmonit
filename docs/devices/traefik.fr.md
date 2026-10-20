# Traefik

Le reverse proxy : routeurs désactivés par une erreur de configuration,
serveurs backend qui échouent leur contrôle de santé, certificats qu'un
resolver n'a jamais obtenus ou qui ne sont pas renouvelés, et la part de
réponses 5xx.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 8080 | TCP | API et métriques (HTTP, par défaut) |

DumbMonit lit l'API de Traefik, qui ne fait que lire : `/api/overview`,
`/api/http/routers`, `/api/http/services`, `/api/certificates` (servi par
Traefik 3.7) et `/api/version`. Quand les métriques Prometheus de Traefik
sont servies sur le même port, `/metrics` ajoute le débit de requêtes et la
part de réponses 5xx. Aucune méthode de l'API ne change rien, et DumbMonit
n'en envoie aucune.

Les pannes à surveiller sont les silencieuses. Un routeur dont le service
ou le middleware n'existe pas, ou dont la règle ne s'analyse pas, est
désactivé : ses sites répondent 404, et Traefik ne le dit que dans son
journal. Un service dont tous les serveurs échouent leur contrôle de santé
répond 503 « no available server ». Un routeur qui demande à un resolver
ACME un certificat que le resolver ne peut pas obtenir obtient le
certificat autosigné par défaut de Traefik, que les navigateurs refusent.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_traefik_`. Les totaux sont
des compteurs : graphiques et règles les transforment en débits.

| Métrique | Quoi | Labels |
|---|---|---|
| `version_info` | valeur 1 | `version` |
| `routers`, `routers_errors`, `routers_warnings` | routeurs de chaque protocole, et ceux en erreur ou en avertissement | |
| `services`, `services_errors`, `middlewares_errors` | la même chose pour les services et middlewares | |
| `router_status` | par routeur HTTP : 0 activé, 1 avertissement, 2 désactivé | `router`, `provider` |
| `router_error_info` | valeur 1, pour un routeur qui n'est pas activé : la première erreur de Traefik | `router`, `error` |
| `service_status` | par service : 0 activé, 1 avertissement, 2 désactivé | `service` |
| `service_servers`, `service_servers_up` | serveurs d'un service en répartition de charge, et ceux qui passent leur contrôle de santé | `service` |
| `server_up` | 1 quand le serveur passe son contrôle de santé (ou n'en a pas) | `service`, `server` |
| `resolver_routers` | routeurs demandant un certificat à un resolver de certificats | `resolver` |
| `resolver_routers_uncovered` | ceux dont les noms d'hôte ne sont couverts par aucun certificat détenu par Traefik encore (a besoin de `/api/certificates`) | `resolver` |
| `certificates`, `cert_expiry_days` | certificats détenus par Traefik, et les jours restants avant chaque expiration | `cert` |
| `requests_total`, `requests_5xx_total` | avec les métriques : requêtes de chaque point d'entrée, et celles répondues en 5xx | |
| `router_requests_total`, `router_requests_5xx_total` | avec les métriques et les labels de routeur : la même chose par routeur | `router` |
| `config_reloads_total` | avec les métriques : rechargements de configuration | |

Les propres routeurs et services de Traefik (`@internal`) sont laissés de
côté. Routeurs, services et serveurs sont décrits un par un jusqu'à 500
chacun ; les totaux couvrent toujours tout.

Les [règles intégrées](../alerting/rules.md#reverse-proxies-and-domains) qui
s'appliquent : routeur Traefik désactivé, serveur backend Traefik en panne,
service Traefik sans serveur, certificat Traefik non obtenu, certificat de
proxy non renouvelé, reverse proxy répondant en 5xx, plus Surveillance
inaccessible.

## La page de la surveillance

Le panneau au-dessus des graphiques lit ce que la sonde a stocké ; ouvrir
la page n'interroge jamais Traefik. Il dit ce qui ne va pas en une phrase
(routeurs désactivés et la raison de Traefik, services sans serveur,
resolvers qui n'ont rien obtenu, certificats proches de l'expiration, une
forte part de 5xx), puis les routeurs, services, débit de requêtes et part
de 5xx, puis les serveurs actifs par service, les certificats les plus
proches de l'expiration en premier, et les réponses 5xx par routeur sur 24
heures.

## Laisser DumbMonit lire l'API de Traefik

1. Ajoutez ces arguments à la commande de Traefik (ou les mêmes clés à sa configuration statique), puis redémarrez-le.

    ```
    --api.insecure=true
    --metrics.prometheus=true
    ```

2. Ouvrez http://traefik.lan:8080/api/overview depuis l'hôte DumbMonit : la réponse doit être du JSON.

3. Facultatif : protégez /api et /metrics par un middleware basicAuth sur un point d'entrée à vous, et créez l'utilisateur avec htpasswd.

    ```
    htpasswd -nbB dumbmonit 'a-long-password'
    ```

4. Dans DumbMonit, saisissez l'hôte Traefik, par exemple traefik.lan, avec le port de ce point d'entrée s'il n'est pas 8080.

!!! warning
    Le port 8080 répond sans mot de passe : filtrez-le pour que seul l'hôte DumbMonit y accède.

### Pour aller plus loin

L'API de Traefik ne fait que lire routeurs, services et certificats, jamais de modification. Les métriques Prometheus ajoutent le débit de requêtes et la part de réponses 5xx ; les deux sont servies sur le point d'entrée traefik, port 8080. Pour exiger un mot de passe, faites passer /api et /metrics par un point d'entrée à vous avec un middleware basicAuth, comme le montre la documentation de Traefik, et choisissez basicAuth user dans le formulaire. Décochez Lire les métriques si /metrics est servi ailleurs. La vérification des certificats utilise /api/certificates, que Traefik 3.7 sert ; sans lui, l'expiration vient des métriques seules. Un routeur désactivé par une erreur de configuration met ses sites hors service, et Traefik ne le dit que dans son journal.

Documentation de l'éditeur : https://doc.traefik.io/traefik/operations/api/

## Identifiants

| Identifiant | Champs |
|---|---|
| Pas de mot de passe | Rien : le port de l'API est filtré pour que seul DumbMonit l'atteigne. |
| Utilisateur basicAuth | L'utilisateur et le mot de passe du middleware basicAuth devant `/api` et `/metrics`. |

Adresse : un nom d'hôte ou une IP (`traefik.lan`), `host:port`, ou une URL
complète. Options : protocole (HTTP par défaut), port de l'API (8080),
Lire les métriques (activé), vérification du certificat et délai de
requête (10 s).

Un service sans contrôle de santé rapporte chaque serveur comme actif :
Traefik ne marque un serveur en panne que quand un `healthCheck` échoue. Les
services pondérés et de mirroring n'ont pas de serveurs propres et ne sont
pas comptés.
