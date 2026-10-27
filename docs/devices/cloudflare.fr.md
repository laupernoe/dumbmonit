# Cloudflare

Les zones d'un compte Cloudflare via l'API Cloudflare : zones qui ne sont plus
actives, tunnels tombés ou dégradés, certificats proches de l'expiration,
hausse des réponses 5xx et pools de répartition de charge en panne.

DumbMonit lit l'API sur `api.cloudflare.com/client/v4` une fois par minute avec
un jeton d'API en lecture seule, envoyé comme jeton bearer. Seule la liste des
zones est indispensable ; chaque autre partie (tunnels, certificats, trafic,
répartiteurs de charge) demande son propre droit de lecture et est ignorée,
sans faire échouer la surveillance, quand le jeton ne l'a pas ou que l'offre ne
comprend pas la fonction. Les appels propres à chaque zone sont limités aux 50
premières zones actives et envoyés cinq à la fois, très en dessous de la limite
de Cloudflare de 1 200 requêtes par cinq minutes.

C'est une API Internet : l'adresse est celle de l'API, pas celle d'une
machine. DumbMonit a besoin de HTTPS sortant vers `api.cloudflare.com` ; rien
d'autre n'est contacté.

!!! warning "Validé contre la documentation du fournisseur seulement — pas encore testé sur un système réel"

    Cette intégration a été construite à partir de la documentation de l'API
    de Cloudflare, pas contre un compte en fonctionnement. Elle ne fait que
    lire. Dites-nous ce qui casse.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 443 | TCP | API Cloudflare (HTTPS, api.cloudflare.com) |

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_cloudflare_`. Au plus 200
éléments nommés par famille ; les comptes restent exacts.

| Métrique | Étiquettes | Signification |
|---|---|---|
| `zone_active` | `zone` | 1 quand le statut de la zone est `active` ; 0 pour `pending`, `moved`, `deactivated`… |
| `zone_paused` | `zone` | 1 quand la zone est suspendue (Cloudflare ne fait plus que le DNS). |
| `zone_info` | `zone`, `status`, `plan` | Valeur 1. |
| `zones`, `zones_active`, `zones_paused` | | Comptes. |
| `tunnel_healthy`, `tunnel_degraded`, `tunnel_down` | `tunnel` | 1 pour le statut du tunnel (`healthy`, `degraded`, `down`). Un tunnel jamais lancé (`inactive`) n'est aucun des trois. |
| `tunnel_connections` | `tunnel` | Connexions du tunnel vers Cloudflare. |
| `tunnels`, `tunnels_healthy`, `tunnels_degraded`, `tunnels_down` | | Comptes. |
| `tunnels_readable` | | 0 quand les tunnels ne peuvent pas être lus (voir Dépannage). |
| `certificate_expiry_seconds` | `zone`, `host` | Temps avant l'expiration la plus proche parmi les certificats actifs de l'hôte. |
| `certificates_readable` | | 0 quand les certificats ne peuvent pas être lus. |
| `zone_http_requests`, `zone_http_5xx` | `zone` | Requêtes et réponses 5xx de la dernière heure complète et de l'heure en cours (1 à 2 heures). |
| `zone_http_5xx_percent` | `zone` | Part des réponses 5xx ; seulement pour les zones d'au moins 100 requêtes sur la fenêtre. |
| `analytics_readable` | | 0 quand le trafic ne peut pas être lu ; absente quand l'option **Lire le trafic** est désactivée. |
| `lb_pool_healthy` | `pool` | 1 quand un pool activé est sain. Les pools désactivés ne sont pas signalés. |
| `lb_pool_enabled`, `lb_pool_origins`, `lb_pool_origins_healthy` | `pool` | État du pool et ses origines activées et saines. |
| `load_balancer_enabled` | `zone`, `load_balancer` | 1 quand le répartiteur est activé. |
| `lb_pools`, `load_balancers` | | Comptes. |
| `lb_readable` | | 0 quand les pools ou les répartiteurs ne peuvent pas être lus. |

Les zones non actives ou suspendues ne font l'objet d'aucun appel de
certificats, de trafic ou de répartiteur. Le trafic vient des statistiques
horaires : il a jusqu'à une heure de retard, et une rafale d'erreurs met
jusqu'à deux heures à sortir de la fenêtre.

Les [règles intégrées](../alerting/rules.md#cloudflare) qui s'appliquent :

- **Tunnel Cloudflare hors service** : un tunnel sans connexion depuis 5
  minutes (Critique, rappel toutes les 6 h).
- **Tunnel Cloudflare dégradé** : un tunnel qui a perdu une partie de ses
  connexions depuis 15 minutes (Avis).
- **Certificat Cloudflare bientôt expiré** : un certificat actif expire dans
  moins de 14 jours ; Cloudflare les renouvelle seul, c'est donc qu'un
  renouvellement a échoué (Avis, rappel quotidien).
- **Zone Cloudflare non active** : une zone qui n'est pas `active` depuis une
  heure (Avis).
- **Taux de 5xx Cloudflare élevé** : plus de 5 % des réponses sont des 5xx
  depuis 15 minutes (Avis).
- **Pool de répartition Cloudflare en panne** : un pool activé en panne
  depuis 5 minutes (Avis).

## La page de la surveillance

Le panneau Cloudflare dit en une phrase ce qui ne va pas, montre les zones,
les tunnels, les requêtes et les réponses 5xx de la fenêtre, puis liste les
zones, les tunnels, les certificats et les pools de répartition avec leur
état.

## Créer un jeton d'API en lecture seule pour DumbMonit

1. Dans le tableau de bord Cloudflare, ouvrez My Profile > API Tokens > Create Token > Create Custom Token, et nommez-le DumbMonit.

2. Ajoutez les droits en lecture seule listés ci-dessous, et rien d'autre. Le premier est obligatoire ; les autres activent une partie chacun.

3. Sous Account Resources et Zone Resources, choisissez le compte et les zones à surveiller (All zones convient).

4. Cliquez sur Continue to summary > Create Token, puis copiez le jeton : il n'est affiché qu'une fois.

5. Dans DumbMonit, utilisez cette adresse et collez le jeton comme jeton d'API. Laissez Account ID vide sauf si le jeton couvre plusieurs comptes.
   `api.cloudflare.com`

### Droits requis

- **Zone > Zone > Read** — Liste les zones et leur statut ; obligatoire.
- **Zone > SSL and Certificates > Read** — Lit la date d'expiration des certificats de chaque zone.
- **Zone > Analytics > Read** — Lit les requêtes et les réponses 5xx de chaque zone.
- **Zone > Load Balancers > Read** — Liste les répartiteurs de charge de chaque zone.
- **Account > Cloudflare Tunnel > Read** — Lit l'état des tunnels.
- **Account > Account Settings > Read** — Trouve le compte des tunnels ; inutile si l'option Account ID est renseignée.
- **Account > Load Balancing: Monitors And Pools > Read** — Lit la santé des pools de répartition de charge.

!!! warning
    Une partie dont le droit manque (ou que l'offre ne comprend pas) est ignorée et sa métrique readable vaut 0 ; la surveillance n'échoue pas. Seules les zones que le jeton voit sont surveillées.

Documentation de l'éditeur : https://developers.cloudflare.com/fundamentals/api/get-started/create-token/

## Identifiants

| Identifiant | Champs |
|---|---|
| Jeton d'API | Le jeton créé ci-dessus, envoyé comme jeton bearer. |

Adresse : `api.cloudflare.com`. Options :

- **Account ID** : le compte des tunnels et des pools de répartition. Vide : lu
  depuis le jeton (5 comptes au plus), ou depuis les comptes propriétaires des
  zones quand le jeton ne peut pas lister les comptes.
- **Lire le trafic** : activé par défaut ; désactivez-le pour éviter les appels
  GraphQL de statistiques (un par 10 zones).
- **Délai par requête** (15 s).

## Dépannage

« Cloudflare refused the API token (401) » : le jeton est incorrect, expiré ou
révoqué.

« Cloudflare refused /client/v4/zones (403) » : le jeton n'a pas **Zone > Zone >
Read**, ne couvre aucune zone, ou le filtrage par adresse IP du jeton exclut le
serveur DumbMonit.

« Cloudflare has no resource at … check the account ID » : l'option Account ID
n'est pas un compte que le jeton peut atteindre.

`tunnels_readable`, `certificates_readable`, `analytics_readable` ou
`lb_readable` vaut 0 : le jeton n'a pas le droit de cette partie (voir la liste
ci-dessus), l'offre ne la comprend pas (la répartition de charge est un module
payant ; les statistiques de l'offre gratuite sont limitées), ou Cloudflare a
refusé l'un des appels par zone.
