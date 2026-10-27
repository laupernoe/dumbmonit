# Hetzner Cloud

Les serveurs d'un projet Hetzner Cloud via l'API Hetzner Cloud : serveurs qui
ne tournent pas, processeur qui reste saturé, sauvegardes arrêtées et volumes
rattachés à rien.

DumbMonit lit l'API sur `api.hetzner.cloud/v1` une fois par minute avec un
jeton de projet en lecture seule, envoyé comme jeton bearer. La liste des
serveurs est indispensable ; l'activité des serveurs, les volumes et les
sauvegardes sont des parties facultatives qui, si elles sont refusées, sont
ignorées sans faire échouer la surveillance. Une surveillance lit un projet
(un jeton appartient à un seul projet).

C'est une API Internet : l'adresse est celle de l'API, pas celle d'une
machine. DumbMonit a besoin de HTTPS sortant vers `api.hetzner.cloud` ; rien
dans vos serveurs n'est contacté.

!!! warning "Validé contre la documentation du fournisseur seulement — pas encore testé sur un système réel"

    Cette intégration a été construite à partir de la documentation de l'API
    Hetzner Cloud, pas contre un projet en fonctionnement. Elle ne fait que
    lire. Dites-nous ce qui casse.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 443 | TCP | API Hetzner Cloud (HTTPS, api.hetzner.cloud) |

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_hetzner_`. Au plus 200 éléments
nommés par famille ; les comptes restent exacts.

| Métrique | Étiquettes | Signification |
|---|---|---|
| `server_running` | `server` | 1 quand le statut du serveur est `running`. |
| `server_info` | `server`, `status`, `type`, `location` | Valeur 1. |
| `server_backups_enabled` | `server` | 1 quand les sauvegardes automatiques sont activées. |
| `servers`, `servers_running`, `servers_not_running` | | Comptes. |
| `server_cpu_percent` | `server` | Utilisation du processeur moyennée sur les dix dernières minutes (serveurs en marche seulement). |
| `server_disk_read_bytes_per_second`, `server_disk_write_bytes_per_second` | `server` | Débit du disque, même fenêtre. |
| `server_network_in_bytes_per_second`, `server_network_out_bytes_per_second` | `server` | Débit du réseau, même fenêtre. |
| `metrics_readable` | | 0 quand les métriques des serveurs ne peuvent pas être lues ; absente quand l'option **Serveurs avec métriques** vaut 0. |
| `volume_attached` | `volume` | 1 quand le volume est rattaché à un serveur. |
| `volume_size_gigabytes` | `volume` | Taille du volume. |
| `volumes`, `volumes_unattached` | | Comptes. |
| `volumes_readable` | | 0 quand les volumes ne peuvent pas être lus. |
| `server_backup_age_seconds` | `server` | Âge de la sauvegarde la plus récente, pour les serveurs qui ont les sauvegardes activées et au moins une sauvegarde. |
| `backups_readable` | | 0 quand les sauvegardes ne peuvent pas être lues ; absente quand aucun serveur n'a les sauvegardes activées. |

Les métriques ne sont lues que pour les premiers serveurs en marche, autant
que le dit l'option **Serveurs avec métriques** (10 par défaut, 100 au plus) :
chacun coûte un appel d'API, et Hetzner autorise 3 600 requêtes par heure et
par projet. Les sauvegardes sont lues parmi les images les plus récentes
(`type=backup`), sur 5 pages au plus.

Les [règles intégrées](../alerting/rules.md#hetzner-cloud) qui s'appliquent :

- **Serveur Hetzner arrêté** : un serveur qui n'est pas à l'état `running`
  depuis 10 minutes (Avis). Un serveur éteint volontairement la déclenche
  aussi.
- **Processeur d'un serveur Hetzner saturé** : plus de 90 % de processeur sur
  dix minutes, depuis 30 minutes (Avis).
- **Sauvegarde Hetzner trop ancienne** : la sauvegarde la plus récente d'un
  serveur dont les sauvegardes sont activées a plus de 3 jours (Avis, rappel
  quotidien).
- **Volume Hetzner non rattaché** : un volume rattaché à aucun serveur depuis
  une heure (Info, rappel hebdomadaire).

## La page de la surveillance

Le panneau Hetzner Cloud dit en une phrase ce qui ne va pas, montre les
serveurs et les serveurs en marche, puis liste les serveurs avec leur état,
les volumes et les sauvegardes des serveurs qui en ont.

## Créer un jeton d'API en lecture seule pour DumbMonit

1. Dans la console Hetzner Cloud, ouvrez le projet à surveiller, puis Security > API tokens > Generate API token.

2. Nommez-le DumbMonit et choisissez la permission Read (pas Read & Write).

3. Cliquez sur Generate API token, puis copiez le jeton : il n'est affiché qu'une fois.

4. Dans DumbMonit, utilisez cette adresse et collez le jeton comme jeton d'API. Une surveillance lit un projet.
   `api.hetzner.cloud`

### Droits requis

- **Read (jeton de projet)** — Liste les serveurs, volumes et sauvegardes et lit les métriques des serveurs ; ne change rien.

!!! warning
    Un serveur éteint est signalé comme ne tournant pas, volontairement ou non : mettez la règle en sourdine pour lui si c'est attendu. L'API autorise 3 600 requêtes par heure et par projet, les métriques ne sont donc lues que pour 10 serveurs par défaut.

Documentation de l'éditeur : https://docs.hetzner.cloud/

## Identifiants

| Identifiant | Champs |
|---|---|
| Jeton d'API | Le jeton de projet créé ci-dessus, envoyé comme jeton bearer. |

Adresse : `api.hetzner.cloud`. Options :

- **Serveurs avec métriques** : combien de serveurs en marche voient leur
  processeur, disque et réseau lus chaque minute, de 0 (aucun) à 100 (10 par
  défaut).
- **Délai par requête** (15 s).

## Dépannage

« Hetzner Cloud refused the API token (401) » : le jeton est incorrect,
supprimé, ou appartient à un autre projet.

« Hetzner Cloud refused … (403) » : le jeton ne peut pas lire cette ressource.

`metrics_readable`, `volumes_readable` ou `backups_readable` vaut 0 : Hetzner
a refusé ou fait échouer l'un de ces appels ; les serveurs restent signalés.
Un serveur dont les sauvegardes sont activées mais qui n'en a encore aucune ne
déclenche pas d'alerte avant sa première.
