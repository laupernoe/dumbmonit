# Plex Media Server

Ce que fait Plex et s'il est à jour : flux en cours de lecture, combien sont
transcodés et si c'est en matériel, la bande passante qu'ils réservent, les
flux depuis l'extérieur de la maison, les bibliothèques en cours de scan, et
les mises à jour.

DumbMonit lit la propre API du serveur sur le port 32400, jamais plex.tv. Il
n'arrête jamais un flux, ne scanne jamais une bibliothèque et ne change jamais
un réglage.

Plex ne publie pas le résultat de ses tâches de maintenance (le « butler » ne
donne que leur calendrier) ni l'état de ses sauvegardes de base de données, il
n'y a donc rien à en dire.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_plex_`.

| Métrique | Quoi | Labels |
|---|---|---|
| `version_info` | valeur 1 | `version` |
| `update_available` | 1 quand la propre vérification de mise à jour de Plex a trouvé une release plus récente ; absent jusqu'à ce que Plex ait vérifié une fois | `latest_version` |
| `claimed` | 1 quand le serveur est lié à un compte Plex | |
| `streams`, `streams_paused`, `streams_remote` | lectures en cours ; en pause ; depuis l'extérieur du réseau local | |
| `transcodes`, `transcodes_video`, `transcodes_hardware` | lectures réencodées (vidéo ou audio) ; avec la vidéo réencodée ; avec accélération matérielle | |
| `transcoder_active_video_sessions` | transcodages vidéo que le serveur lui-même compte | |
| `stream_bandwidth_bits_per_second` | bande passante réservée par toutes les lectures | |
| `libraries`, `libraries_scanning` | bibliothèques ; celles en cours de scan | |
| `activities` | activités en arrière-plan en cours (scans, analyse, mises à jour) | |
| `scrape_errors` | appels qui ont échoué à la dernière sonde | |

La [règle intégrée](../alerting/rules.md#self-hosted-applications) qui
s'applique : mise à jour Plex disponible, plus Appareil inaccessible.

## La page de l'appareil

Le panneau au-dessus des graphiques lit ce que la sonde a stocké ; ouvrir la
page n'appelle jamais Plex. Il donne l'état de mise à jour et si le serveur
est revendiqué, puis les flux, transcodages, transcodages matériels, flux
distants, bande passante et bibliothèques.

## Donner à DumbMonit l'accès au serveur Plex

1. DumbMonit lit la propre API du serveur sur le port 32400, jamais plex.tv. Un serveur lié à un compte Plex demande un jeton (X-Plex-Token), et Plex ne peut pas en émettre un limité : c'est le jeton du propriétaire.

2. Évitez le jeton : dans Plex, Settings → Network → List of IP addresses and networks that are allowed without auth, ajoutez l'adresse du serveur DumbMonit, par exemple comme suit, et choisissez Pas de jeton ci-dessous.

    ```
    10.0.0.20/32
    ```

3. Ou utilisez le jeton : dans Plex Web, ouvrez n'importe quel film → ⋯ → Get Info → View XML. L'adresse de la page qui s'ouvre se termine par X-Plex-Token= suivi du jeton ; copiez cette valeur.

4. Dans DumbMonit, saisissez l'adresse du serveur, par exemple « plex.lan » (port 32400). DumbMonit ne lit que les sessions, les bibliothèques, le vérificateur de mise à jour et la liste d'activité ; il n'arrête jamais un flux et ne change jamais un réglage.

!!! warning
    Le jeton est celui de votre compte Plex : qui le détient peut gérer le serveur et chaque bibliothèque. Préférez le réseau autorisé ci-dessus quand DumbMonit se trouve sur votre propre réseau.

Le réseau autorisé est aussi ce qui fait répondre Plex sans jeton depuis
Docker : le conteneur de DumbMonit atteint Plex depuis l'adresse de son réseau
Docker, listez donc ce réseau, ou l'adresse de l'hôte quand Plex publie son
port sur l'hôte.

## Identifiants

| Identifiant | Champs |
|---|---|
| X-Plex-Token | Envoyé dans l'en-tête `X-Plex-Token`. |
| Pas de jeton (réseau autorisé) | Rien n'est envoyé ; Plex doit lister l'adresse de DumbMonit parmi les réseaux autorisés sans authentification. |

Adresse : un nom d'hôte (`plex.lan`), `host:port`, ou une URL complète ; un
`/web` final est accepté. Options : protocole (HTTP par défaut ; les propres
certificats de Plex sont émis pour les noms `*.plex.direct`, donc HTTPS par
adresse IP a besoin que la vérification du certificat soit désactivée), port
(32400), vérification du certificat et délai de requête (15 s).

Validé avec Plex Media Server 1.43.4 (image officielle), un serveur non
revendiqué répondant sans jeton, avec une vraie lecture transcodée. Le chemin
par jeton suit l'en-tête `X-Plex-Token` documenté par Plex et n'a pas été
exercé contre un serveur revendiqué.
