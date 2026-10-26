# OVHcloud

Les VPS et les serveurs dédiés d'un compte OVHcloud via l'API OVHcloud :
serveurs arrêtés ou en erreur, et tâches (redémarrage, réinstallation,
maintenance) en cours sur eux.

DumbMonit lit `GET /1.0/vps` et `GET /1.0/dedicated/server` sur
`eu.api.ovh.com`, puis l'état et les tâches en attente de chaque serveur, une
fois par minute. L'identifiant est un jeton d'API OVHcloud limité à deux
droits de lecture. Chaque requête est signée (`X-Ovh-Signature`, un SHA-1 du
secret, de la clé de consommateur, de la méthode, de l'URL et de l'heure du
serveur lue sur `/auth/time`) : l'horloge de l'hôte DumbMonit n'importe pas.

C'est une API Internet : l'adresse de la surveillance est celle de l'API, pas
celle d'une machine. DumbMonit a besoin de HTTPS sortant vers
`eu.api.ovh.com` (ou `ca.api.ovh.com` pour OVHcloud Canada) ; aucun serveur
n'est contacté.

!!! warning "Validé contre la documentation du fournisseur seulement — pas encore testé sur un système réel"

    Cette intégration a été construite à partir de la documentation de l'API
    d'OVHcloud, pas contre un compte en fonctionnement. Elle ne fait que
    lire. Dites-nous ce qui casse.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 443 | TCP | API OVHcloud (HTTPS, eu.api.ovh.com) |

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_ovh_`. Les 30 premiers serveurs
de chaque famille sont décrits ; les compteurs restent exacts au-delà.

| Métrique | Labels | Signification |
|---|---|---|
| `vps_up` | `vps` | 1 tant que le VPS tourne (ou fait une sauvegarde). |
| `vps_info` | `vps`, `state`, `zone`, `model` | Valeur 1. |
| `vps_tasks_active` | `vps` | Tâches en attente ou en cours sur le VPS. |
| `dedicated_up` | `server` | 1 tant que la supervision d'OVHcloud dit le serveur `ok` et qu'il est allumé. |
| `dedicated_monitoring` | `server` | 1 quand la supervision d'OVHcloud est activée sur le serveur. |
| `dedicated_info` | `server`, `state`, `power`, `datacenter` | Valeur 1. |
| `dedicated_tasks_active` | `server` | Tâches en attente ou en cours sur le serveur. |
| `servers`, `servers_down`, `tasks_active` | | Compteurs. |
| `vps_readable`, `dedicated_readable`, `tasks_readable` | | 0 quand le jeton n'a pas le droit de lire cette partie : ses séries sont omises et la sonde réussit quand même. |

Les [règles intégrées](../alerting/rules.md#ovhcloud) qui s'appliquent :

- **VPS OVHcloud à l'arrêt** : un VPS n'est plus à l'état running depuis
  10 minutes (Avis).
- **Serveur dédié OVHcloud en erreur** : la supervision d'OVHcloud signale un
  serveur dédié en erreur, ou éteint, depuis 5 minutes (Critique).
- **Tâche OVHcloud en cours** : des tâches sont en attente ou en cours depuis
  une heure (Info).

OVHcloud ne publie pas de calendrier des maintenances planifiées dans son API
publique : DumbMonit ne voit une intervention qu'une fois commencée, sous
forme de tâche. Un VPS que vous avez arrêté vous-même compte comme à l'arrêt,
et un serveur dédié n'est signalé en erreur que si la supervision d'OVHcloud
est activée dessus.

## La page de la surveillance

Le panneau dit ce qui ne va pas en une phrase, montre le nombre de serveurs,
les serveurs en panne et les tâches en cours, puis liste les VPS et les
serveurs dédiés avec leur état.

## Créer un jeton d'API en lecture seule pour DumbMonit

1. Ouvrez la page de création de jeton OVHcloud de votre région et connectez-vous avec votre compte OVHcloud. Le lien pré-sélectionne les deux droits de lecture.
   `https://eu.api.ovh.com/createToken/?GET=/vps*&GET=/dedicated/server*`

2. Nommez le script DumbMonit, réglez Validity sur Unlimited, et ne gardez que les deux droits GET listés ci-dessous, puis cliquez sur Create keys.

3. Copiez l'Application Key, l'Application Secret et la Consumer Key : elles ne sont affichées qu'une fois.

4. Dans DumbMonit, saisissez eu.api.ovh.com comme adresse (ca.api.ovh.com pour OVHcloud Canada) et collez l'Application Key comme Application key.

5. Dans le second champ, saisissez l'Application Secret, deux-points, puis la Consumer Key.
   `application_secret:consumer_key`

### Droits requis

- **GET /vps\*** — Liste les VPS et lit leur état et leurs tâches, ne change rien
- **GET /dedicated/server\*** — Liste les serveurs dédiés et lit leur état et leurs tâches, ne change rien

!!! warning
    L'API OVHcloud ne publie pas de calendrier des maintenances planifiées : DumbMonit ne voit une intervention qu'une fois commencée, sous forme de tâche en cours. Un VPS que vous avez arrêté vous-même compte comme en panne. Un serveur dédié n'est signalé en erreur que si la supervision d'OVHcloud est activée dessus. L'un ou l'autre droit peut être omis pour ne surveiller que l'autre famille.

## Identifiants

| Identifiant | Champs |
|---|---|
| Clés d'API OVHcloud | **Application key**, et dans le champ mot de passe le secret d'application et la clé de consommateur joints par deux-points : `application_secret:consumer_key`. |

Adresse : `eu.api.ovh.com`. Options :

- **Délai par requête** (15 s).

## Dépannage

« OVHcloud refused the signature (400) » : la clé d'application, le secret ou
la clé de consommateur est incorrect, ou le champ mot de passe n'est pas dans
l'ordre `secret:consumer`.

« OVHcloud accepted the credentials but refused ... (403) » : la clé de consommateur a été
révoquée ou a expiré. Créez un nouveau jeton.

« OVHcloud granted this consumer key neither GET /vps* nor
GET /dedicated/server* » : recréez le jeton avec au moins l'un des deux
droits.

Une famille absente de la page avec sa métrique `readable` à 0 : le jeton n'a
pas de droit dessus, ou le compte n'a aucun produit de ce type.
