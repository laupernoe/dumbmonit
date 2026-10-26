# Infomaniak

Les produits d'un compte Infomaniak via l'API Infomaniak : domaines,
hébergements, messagerie et autres services sur le point d'expirer, bloqués
ou en maintenance.

DumbMonit lit `GET /1/product` sur `api.infomaniak.com` une fois par minute,
pour le compte que vous nommez (ou pour tous les comptes que le jeton peut
lister). L'identifiant est un jeton d'API créé dans le Manager Infomaniak,
envoyé comme jeton bearer.

C'est une API Internet : l'adresse de la surveillance est celle de l'API, pas
celle d'une machine. DumbMonit a besoin de HTTPS sortant vers
`api.infomaniak.com` ; rien d'autre n'est contacté.

!!! warning "Validé contre la documentation du fournisseur seulement — pas encore testé sur un système réel"

    Cette intégration a été construite à partir de la documentation de l'API
    d'Infomaniak, pas contre un compte en fonctionnement. Elle ne fait que
    lire. Dites-nous ce qui casse.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 443 | TCP | API Infomaniak (HTTPS, api.infomaniak.com) |

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_infomaniak_`. Les 200 premiers
produits sont décrits ; les compteurs restent exacts au-delà.

| Métrique | Labels | Signification |
|---|---|---|
| `product_expiry_seconds` | `product`, `service`, `id` | Temps avant l'expiration du produit ; négatif une fois expiré. Absent pour les produits sans date d'expiration (gratuits). |
| `product_locked` | `product`, `service`, `id` | 1 quand Infomaniak signale le produit comme bloqué. |
| `product_maintenance` | `product`, `service`, `id` | 1 quand le produit est en maintenance. |
| `product_operation_in_progress` | `product`, `service`, `id` | 1 tant qu'une opération est en cours sur le produit. |
| `products`, `products_expiring`, `products_expired`, `products_locked`, `products_maintenance` | | Compteurs (`products_expiring` : moins de 14 jours restants). |

L'API Infomaniak ne donne pas d'état plus fin que la date d'expiration et ces
drapeaux : elle ne dit pas si un site ou une boîte mail est disponible.
Utilisez une surveillance HTTP pour cela.

Les [règles intégrées](../alerting/rules.md#infomaniak) qui s'appliquent :

- **Produit Infomaniak bientôt expiré** : moins de 14 jours restants (Avis,
  rappelé chaque jour).
- **Produit Infomaniak expiré** : la date d'expiration est dépassée
  (Critique).
- **Produit Infomaniak bloqué** : depuis une heure (Avis).
- **Produit Infomaniak en maintenance** : depuis une heure (Info).

## La page de la surveillance

Le panneau dit ce qui ne va pas en une phrase, montre le nombre de produits,
ceux qui expirent bientôt et ceux qui ont expiré, puis liste les produits
avec leur état.

## Créer un jeton d'API en lecture seule pour DumbMonit

1. Ouvrez la page des jetons du Manager Infomaniak et connectez-vous.
   `https://manager.infomaniak.com/v3/ng/profile/user/token/list`

2. Cliquez sur Créer un jeton et nommez-le DumbMonit.

3. Choisissez les portées listées ci-dessous, et réglez la validité voulue (sans expiration, vous n'aurez pas à le renouveler).

4. Cliquez sur Créer le jeton, puis copiez-le : il n'est affiché qu'une fois.

5. Dans DumbMonit, saisissez api.infomaniak.com comme adresse et collez le jeton. Renseignez l'option Account ID si le jeton n'a pas la portée Accounts.

### Droits requis

- **Products (portée : product)** — Liste les produits du compte avec leur date d'expiration et leurs drapeaux, ne change rien
- **Accounts (portée : accounts)** — Liste les identifiants de compte du jeton ; facultatif quand l'option Account ID est renseignée

!!! warning
    L'API Infomaniak ne donne pas d'état plus fin que la date d'expiration, le blocage et les drapeaux de maintenance de chaque produit : elle ne dit pas si un site ou une boîte mail est disponible. Utilisez une surveillance HTTP pour cela.

## Identifiants

| Identifiant | Champs |
|---|---|
| Jeton d'API | Le jeton créé dans le Manager Infomaniak, envoyé comme jeton bearer. |

Adresse : `api.infomaniak.com`. Options :

- **Account ID** : le compte à lire. Vide : chaque compte que le jeton peut
  lister (10 au plus), ce qui demande la portée Accounts.
- **Services** : noms de services à lire séparés par des virgules
  (`domain, hosting`). Vide : chaque produit du compte.
- **Délai par requête** (15 s).

## Dépannage

« Infomaniak refused the API token » : le jeton est incorrect, expiré ou
révoqué, ou il manque une portée. S'il manque la portée Accounts, renseignez
l'option Account ID.

« Infomaniak knows nothing at /1/product… » : vérifiez l'Account ID.

« Infomaniak answered 422 … » : l'API veut un nom de service pour ce compte ;
renseignez l'option Services.
