# TP-Link Omada

Un contrôleur TP-Link Omada via son OpenAPI : points d'accès, commutateurs et
passerelles qui ont perdu le contrôleur, appareils en attente d'adoption,
appareils dont le processeur ou la mémoire est saturé, mises à jour du
firmware proposées par le contrôleur, et nombre de clients par site.

!!! warning "Bêta"

    Cette intégration est écrite d'après la documentation de l'OpenAPI
    d'Omada de TP-Link et n'a pas été testée sur un contrôleur réel. Chaque
    appel est une lecture (hors la connexion qui obtient le jeton d'accès).
    Dites-nous ce qui casse.

DumbMonit utilise une application OpenAPI créée dans le contrôleur en mode
Client avec le rôle Observateur (Viewer). Ses identifiant et secret s'échangent
contre un jeton d'accès valable deux heures (`grant_type=client_credentials`) ;
le jeton est gardé jusqu'à un peu avant son expiration, puis redemandé de la
même façon. Il est envoyé comme `Authorization: AccessToken=…`, jamais dans
l'URL.

L'identifiant du contrôleur (Omada ID) est celui affiché sous Open API. Laissé
vide, il est lu sur `/api/info`, que le contrôleur publie sans authentification.

Chaque minute, DumbMonit liste les sites (`/openapi/v1/{omadacId}/sites`), puis
lit pour chacun les appareils (`…/sites/{siteId}/devices`, requis) et le nombre
de clients (`…/clients`, optionnel : un rôle plus restreint peut le refuser).

La découverte réseau reconnaît un contrôleur par `/api/info` sur les ports 8043
et 443 et propose de l'ajouter.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 8043 | TCP | HTTPS et OpenAPI du contrôleur (contrôleur logiciel, par défaut) |
| 443 | TCP | Contrôleurs matériels (OC200, OC300) et installations derrière un proxy inverse |

Le port 443 n'est utile que si le contrôleur écoute là plutôt que sur 8043.

## Ce qui est surveillé

Toutes les métriques sont préfixées `dumbmonit_omada_`.

| Métrique | Étiquettes | Signification |
|---|---|---|
| `controller_info` | `version`, `api_version`, `omadac_id` | Valeur 1. |
| `sites` | | Sites lus. |
| `devices` | `status` | `online`, `offline` (déconnecté, battement de cœur manquant ou isolé) et `pending` (en attente d'adoption). |
| `device_online`, `device_offline`, `device_pending` | `device`, `mac`, `model`, `device_type`, `site` | 1 ou 0, par appareil. |
| `device_info` | idem, plus `firmware`, `ip` | Valeur 1. |
| `device_cpu_usage_percent`, `device_memory_used_percent` | idem | Seulement tant que l'appareil est en ligne : un appareil hors ligne rapporte des zéros. |
| `device_uptime_seconds`, `device_clients` | idem | Seulement tant que l'appareil est en ligne. |
| `device_upgrade_available` | idem | 1 quand le contrôleur propose une mise à jour du firmware. |
| `site_clients` | `site` | Clients du site. |
| `clients` | | Leur somme. |

Au plus 200 appareils sont nommés par passage ; les comptes par état restent
exacts.

Les [règles intégrées](../alerting/rules.md#tp-link-omada) qui s'appliquent :

- **Appareil Omada hors ligne** : déconnecté depuis dix minutes (Avis).
- **Processeur d'un appareil Omada élevé** : au-dessus de 90 % pendant quinze
  minutes (Avis).
- **Mémoire d'un appareil Omada élevée** : au-dessus de 90 % pendant quinze
  minutes (Avis).
- **Mise à jour du firmware Omada** : le contrôleur propose une mise à jour
  (Info, rappelée chaque semaine).

## La page du moniteur

Le panneau Wi-Fi et commutation dit en une phrase ce qui ne va pas, montre les
appareils en ligne, les sites et les clients, puis liste les appareils qui
demandent un coup d'œil : déconnectés, en attente d'adoption, saturés ou avec
une mise à jour du firmware.

## Créer un client OpenAPI en lecture seule pour DumbMonit

1. Dans le contrôleur Omada, ouvrez Global View → Settings → Platform Integration → Open API et cliquez sur Add New App.

2. Nommez l'application comme suit, choisissez le mode d'accès Client et donnez-lui le rôle Viewer sur les sites à lire.

    ```
    dumbmonit
    ```

3. Cliquez sur Apply : le contrôleur affiche l'Omada ID, le Client ID et le Client Secret. Copiez-les : le secret n'est affiché qu'une fois.

4. Dans DumbMonit, saisissez l'adresse du contrôleur (omada.lan), changez le port s'il n'est pas 8043 (443 pour un contrôleur matériel), puis collez le Client ID et le Client secret. Saisissez l'Omada ID dans l'option Identifiant du contrôleur, ou laissez-la vide pour le lire sur le contrôleur.

### Permissions requises

- **Rôle Viewer sur les sites** — Lit les sites, appareils et nombres de clients, ne modifie rien

!!! warning
    Le contrôleur Omada fournit un certificat autosigné : cochez « Accepter un certificat non vérifiable » sauf si vous avez installé le vôtre. L'OpenAPI existe à partir de la version 5.9 du contrôleur.

### Notes

- Les noms de menus ci-dessus suivent le contrôleur 5.14 ; les versions plus
  anciennes placent aussi Open API sous Settings → Platform Integration.
- Les alertes du contrôleur ne sont pas lues par cette version.

## Identifiants

| Identifiant | Champs |
|---|---|
| Client OpenAPI | Le Client ID (champ nom d'utilisateur) et le Client Secret (champ mot de passe) de l'application. |

Adresse : le nom d'hôte ou l'IP du contrôleur, `host:port`, ou une URL.
Options :

- **Port** (8043) : le port HTTPS du contrôleur.
- **Identifiant du contrôleur** : l'Omada ID ; vide pour le lire sur le
  contrôleur.
- **Site** : le nom ou l'identifiant du seul site à lire ; vide pour tous.
- **Accepter un certificat non vérifiable** : le propre certificat du
  contrôleur est autosigné.
- **Délai par requête** (15 s).

## Dépannage

« Omada refused the OpenAPI client » : le Client ID ou le secret est
incorrect, l'application a été supprimée, elle n'est pas en mode Client, ou
l'identifiant du contrôleur n'est pas celui de ce contrôleur.

« … did not answer like an Omada controller » : mauvaise adresse ou mauvais
port (8043 pour le contrôleur logiciel, 443 pour un contrôleur matériel).

« Omada refused … (error …): … permission » : donnez à l'application le rôle
Viewer sur les sites à lire.

« The Omada controller has no site … » : l'option Site ne correspond à aucun
nom ni identifiant.
