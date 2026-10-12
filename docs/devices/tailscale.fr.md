# Tailscale

Les appareils d'un tailnet via l'API Tailscale : serveurs passés hors ligne,
clés de nœud proches de l'expiration, appareils en attente d'approbation, et
clients avec une mise à jour disponible.

DumbMonit lit `GET /api/v2/tailnet/{tailnet}/devices` sur
`api.tailscale.com`, une fois par minute. L'identifiant recommandé est un
client OAuth limité à la portée `devices:core:read` : DumbMonit échange son
ID et son secret contre un jeton d'une heure (`/api/v2/oauth/token`) et le
garde jusqu'à expiration. Il peut lister les appareils et ne peut rien
changer. Un jeton d'accès d'API (`tskey-api-…`) fonctionne aussi, mais il
porte tous les droits de l'utilisateur qui l'a créé et expire après 90 jours
au plus.

C'est une API Internet : l'adresse de l'appareil est celle de l'API, pas
celle d'une machine. DumbMonit a besoin de HTTPS sortant vers
`api.tailscale.com` ; rien dans le tailnet n'est contacté.

!!! warning "Validé contre la documentation du fournisseur seulement — pas encore testé sur un système réel"

    Cette intégration a été construite à partir de la documentation de
    l'API de Tailscale et du modèle d'appareil de son client Go officiel,
    pas contre un tailnet en fonctionnement. Elle ne fait que lire. Dites-
    nous ce qui casse.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_tailscale_`. Un appareil est
nommé par le premier label de son nom MagicDNS (`nas` pour
`nas.tail1234.ts.net`). Les appareils partagés dans le tailnet depuis un
autre sont comptés, pas décrits.

| Métrique | Labels | Signification |
|---|---|---|
| `device_online` | `device` | 1 tant que l'appareil est connecté au serveur de coordination. |
| `device_offline_seconds` | `device` | 0 en ligne, sinon le temps depuis qu'il a été vu pour la dernière fois. |
| `device_watched` | `device` | 1 quand l'appareil doit rester en ligne : chaque appareil étiqueté par défaut, ou ceux nommés dans l'option ci-dessous. |
| `device_key_expiry_seconds` | `device` | Jusqu'à l'expiration de la clé de nœud ; absent quand l'expiration de clé est désactivée. |
| `device_authorized` | `device` | 0 tant que l'appareil attend l'approbation. |
| `device_update_available` | `device` | 1 quand Tailscale propose un client plus récent. |
| `device_info` | `device`, `os`, `client_version`, `tags` | Valeur 1. |
| `devices`, `devices_online`, `devices_external`, `devices_update_available`, `devices_unauthorized`, `devices_watched_offline` | | Compteurs. |

Seuls les appareils surveillés déclenchent une alerte quand ils passent hors
ligne : un téléphone ou un ordinateur portable qui dort n'est pas une panne.
Les appareils étiquetés sont, en pratique, les serveurs, routeurs de
sous-réseau et nœuds de sortie ; listez les autres dans l'option
**Appareils à garder en ligne**, ou saisissez `all`.

Les [règles intégrées](../alerting/rules.md#tailscale) qui s'appliquent :

- **Appareil Tailscale hors ligne** : un appareil surveillé déconnecté
  depuis plus de 15 minutes (Avis).
- **Clé de nœud Tailscale arrivant à expiration** : moins de 14 jours
  restants (Avis, rappelé chaque jour). À l'expiration, l'appareil quitte le
  tailnet jusqu'à ce que quelqu'un le reconnecte.
- **Appareil Tailscale en attente d'approbation** (Avis) : approuvez-le
  s'il est attendu, supprimez-le sinon.
- **Mise à jour client Tailscale** : un appareil surveillé a une mise à
  jour client depuis six heures (Info, rappelée chaque semaine).

## Appareils

Chaque appareil du tailnet, surveillé ou non (sauf les appareils partagés),
est publié sous la famille générique partagée par chaque intégration
DumbMonit qui suit les appareils clients — `dumbmonit_client_device_*`,
préfixée sans `tailscale`, documentée en entier sous
[Appareils clients](../alerting/rules.md#client-devices) :

| Métrique | Quoi | Labels |
|---|---|---|
| `last_seen_timestamp_seconds` | Dernière connexion : « maintenant » tant que `connectedToControl` est vrai, sinon `lastSeen` | `device`, `type` (`Tagged` ou `Personal`), `os`, `user`, `kind` |
| `stale_seconds` | Positif une fois qu'un appareil a dépassé `device_stale_days` (3 par défaut) sans se connecter ; alimente la règle [Appareil client périmé](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (toujours `connection` : Tailscale n'a pas de sauvegarde à rapporter) |

Un téléphone ou un ordinateur portable qui voyage peut rester plusieurs
jours sans se reconnecter : ceci est indépendant du réglage **Appareils à
garder en ligne** ci-dessus, qui ne déclenche une alerte que pour les
appareils surveillés passant hors ligne. Augmentez la **péremption
d'appareil** pour un tailnet avec des appareils qui se déplacent.

## La page de l'appareil

Le panneau Tailnet dit ce qui ne va pas en une phrase, montre les appareils
en ligne, les appareils surveillés hors ligne, les mises à jour disponibles
et les appareils partagés, puis liste les appareils surveillés et tout
appareil en attente d'approbation ou dont la clé expire bientôt, puis le
tableau de chaque appareil avec sa dernière connexion.

## Créer un client OAuth en lecture seule pour DumbMonit

1. Dans la console Tailscale (login.tailscale.com), ouvrez Settings > Trust credentials (anciennement OAuth clients) et générez un client OAuth avec seulement la portée devices:core:read. Il liste les appareils et ne peut rien changer.

2. Copiez l'ID client et le secret client (tskey-client-…), montrés une fois. Dans DumbMonit, saisissez api.tailscale.com comme adresse et collez-les comme Client ID et Client secret. Laissez l'option Tailnet vide : le tailnet du client est lu.

!!! warning
    Seuls les appareils étiquetés (serveurs, routeurs de sous-réseau, nœuds de sortie) sont censés rester en ligne : un téléphone ou un ordinateur portable qui dort n'est pas une panne. Nommez les appareils à surveiller dans l'option Appareils à garder en ligne, ou saisissez all. Chaque appareil du tailnet apparaît dans le tableau Appareils ci-dessous avec sa dernière connexion, surveillé ou non ; un téléphone qui voyage peut rester plusieurs jours sans, augmentez donc la péremption d'appareil pour ceux-là.

## Identifiants

| Identifiant | Champs |
|---|---|
| Client OAuth | ID client et secret client (`tskey-client-…`) d'un client avec la portée `devices:core:read`. |
| Jeton d'accès d'API | Un jeton `tskey-api-…`, envoyé comme jeton bearer. |

Adresse : `api.tailscale.com`. Options :

- **Tailnet** : vide pour le tailnet des identifiants, ou le nom de tailnet
  affiché dans la console Tailscale.
- **Appareils à garder en ligne** : noms d'appareil ou étiquettes séparés
  par des virgules (`nas, tag:router`), ou `all`. Vide : chaque appareil
  étiqueté.
- **Délai par requête** (15 s).
- **Péremption d'appareil** (`device_stale_days`, 3 jours par défaut).

## Dépannage

« Tailscale refused the OAuth client » : l'ID ou le secret est incorrect, ou
le client a été supprimé.

« Tailscale refused to list the devices (403) » : le client n'a pas la
portée `devices:core:read`.

« Tailscale knows no tailnet … » : laissez l'option Tailnet vide.
