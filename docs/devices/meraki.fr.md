# Cisco Meraki

Une organisation Cisco Meraki via l'API Dashboard : appareils hors ligne,
liaisons Internet des routeurs MX en échec (et bascule sur l'autre liaison),
licence hors conformité ou sur le point d'expirer, et nombre de clients par
réseau.

!!! warning "Bêta"

    Cette intégration est écrite d'après la documentation de l'API Dashboard
    de Cisco et n'a pas été testée sur du matériel Meraki réel. Chaque appel
    est une lecture. Dites-nous ce qui casse.

DumbMonit utilise la clé d'API d'un administrateur qui n'a qu'un accès en
lecture seule à l'organisation. Elle est envoyée comme `Authorization: Bearer`,
jamais dans l'URL. L'adresse du moniteur est celle de l'API
(`api.meraki.com`), pas d'un équipement : DumbMonit n'a pas besoin de joindre
vos sites.

Chaque minute, DumbMonit lit deux pages requises — `/api/v1/organizations` et
`/api/v1/organizations/{id}/devices/statuses` — et quatre optionnelles qu'une
clé plus restreinte, ou un autre modèle de licence, peut refuser sans faire
échouer le reste : les noms des réseaux, `uplinks/statuses`, la licence
(`licenses/overview`, ou la liste `licenses` par appareil pour les
organisations à licence par appareil) et les clients vus dans la dernière
heure pour chaque réseau.

Deux comportements de l'API sont gérés pour vous. Le dashboard répond à
certaines requêtes par une redirection (307) vers le serveur qui héberge
l'organisation (`n123.meraki.com`) : DumbMonit la suit, mais seulement dans le
domaine Meraki, si bien que la clé ne part jamais ailleurs. Et l'API autorise
10 requêtes par seconde et par organisation : DumbMonit lit une page après
l'autre et attend le `Retry-After` d'une réponse 429 avant de réessayer.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 443 | TCP | Meraki Dashboard API (HTTPS, api.meraki.com) |

## Ce qui est surveillé

Toutes les métriques sont préfixées `dumbmonit_meraki_`.

| Métrique | Étiquettes | Signification |
|---|---|---|
| `organization_info` | `organization`, `organization_id` | Valeur 1. |
| `devices` | `status` | Appareils par état : `online`, `alerting`, `offline`, `dormant`. Comptes exacts. |
| `device_online` | `device`, `serial`, `model`, `product_type`, `network` | 1 quand l'appareil est `online` ou `alerting`. Les appareils dormants n'ont pas de série par appareil. |
| `device_offline` | idem | 1 quand l'état est `offline`. |
| `device_alerting` | idem | 1 quand l'état est `alerting` (joignable, avec un problème signalé par le dashboard). |
| `device_last_report_age_seconds` | idem | Depuis le dernier rapport de l'appareil au cloud. |
| `uplink_up` | `device`, `serial`, `interface`, `network` | 1 quand la liaison est `active` ou `ready`. |
| `uplink_failed` | idem | 1 quand l'état est `failed` : une liaison configurée qui a perdu Internet. `not connected` (une prise sans câble, un emplacement cellulaire vide) n'est pas une panne. |
| `uplink_failover` | `device`, `serial`, `network` | 1 quand la liaison `wan1` est en échec et qu'une autre porte le trafic. |
| `uplinks_failed` | | Compte. |
| `license_ok` | | 1 quand la vue d'ensemble de la licence dit `OK`. Licence à co-terminaison seulement. |
| `license_status_info` | `status` | Valeur 1. |
| `license_expiry_seconds` | | Jusqu'à l'expiration de la licence ; en licence par appareil, jusqu'à la plus proche échéance des licences actives. |
| `network_clients` | `network`, `network_id` | Clients vus dans la dernière heure (jusqu'à 1000 par réseau, 25 réseaux). |
| `clients` | | Leur somme. |

Au plus 200 appareils et 200 entrées de liaisons sont nommés par
organisation ; les comptes restent exacts jusqu'aux 1000 appareils d'une page.

Les [règles intégrées](../alerting/rules.md#cisco-meraki) qui s'appliquent :

- **Appareil Meraki hors ligne** : hors ligne depuis dix minutes (Avis). Les
  appareils dormants ne sont jamais signalés.
- **Liaison Meraki en échec** : cinq minutes (Avis).
- **Problème de licence Meraki** : la licence n'est pas conforme depuis une
  heure (Avis, rappelé chaque jour).
- **Licence Meraki bientôt expirée** : moins de 30 jours (Avis, rappelé chaque
  jour).

## La page du moniteur

Le panneau Réseau cloud dit en une phrase ce qui ne va pas, montre les
appareils en ligne, en alerte et dormants, les clients et la licence, puis liste
les appareils hors ligne ou en alerte et les liaisons en échec (avec une
mention quand le routeur tourne déjà sur sa liaison de secours).

## Créer une clé d'API en lecture seule pour DumbMonit

1. Dans le dashboard Meraki, ouvrez Organization → Settings et cochez Enable access to the Cisco Meraki Dashboard API.

2. Ouvrez Organization → Administrators → Add admin et créez un administrateur dédié à DumbMonit, avec l'accès Organization en Read-only.

    ```
    dumbmonit@example.com
    ```

3. Connectez-vous avec cet administrateur, ouvrez My profile → API access et cliquez sur Generate new API key. Copiez-la : le dashboard ne l'affiche qu'une fois.

4. Dans DumbMonit, saisissez api.meraki.com comme adresse et collez la clé comme clé d'API. Laissez Organisation vide pour lire la première organisation que la clé voit.

### Permissions requises

- **Accès Organization : Read-only** — Liste les appareils, liaisons, licences et clients, ne modifie rien

!!! warning
    Un appareil dormant (non vu depuis longtemps) est compté mais jamais signalé en panne, et une prise WAN sans câble n'est pas une liaison en échec : seule une liaison qui a perdu son accès à Internet l'est.

Documentation du fournisseur : https://developer.cisco.com/meraki/api-v1/

### Notes

- Une clé est liée à l'administrateur qui l'a générée : si ce compte est
  supprimé ou perd son accès, le moniteur affiche une erreur d'authentification.
- Les autres régions et le cloud gouvernemental américain ont leur propre hôte
  d'API (`api.meraki.cn`, `api.gov-meraki.com`) : saisissez-le comme adresse.

## Identifiants

| Identifiant | Champs |
|---|---|
| Clé d'API | La clé générée sous My profile → API access, envoyée comme `Authorization: Bearer`. |

Adresse : `api.meraki.com` (ou l'hôte d'API de votre région). Options :

- **Organisation** : l'identifiant ou le nom de l'organisation à lire ; vide
  pour la première que la clé voit.
- **Réseaux** : noms ou identifiants de réseaux à surveiller, séparés par des
  virgules ; vide pour tous. Restreint appareils, liaisons et clients.
- **Délai par requête** (15 s).

## Dépannage

« Meraki refused the API key (401) » : la clé est incorrecte, a été régénérée,
ou son administrateur a été supprimé.

« Meraki refused … (403) » : l'accès à l'API n'est pas activé pour
l'organisation (Organization → Settings), ou l'administrateur n'y a pas accès.

« The API key does not see an organization … » : l'option Organisation ne
correspond à aucun identifiant ni nom que la clé voit.

Aucun chiffre de licence : la clé ne peut pas la lire, ou l'organisation est
en licence par abonnement, que cette version ne lit pas.
