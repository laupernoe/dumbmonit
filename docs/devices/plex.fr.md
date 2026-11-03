# Plex Media Server

Ce que fait Plex et s'il est à jour : flux en cours de lecture, combien sont
transcodés et si c'est en matériel, la bande passante qu'ils réservent, les
flux depuis l'extérieur de la maison, les bibliothèques en cours de scan, et
les mises à jour.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 32400 | TCP | API (HTTP, par défaut) |

DumbMonit lit la propre API du serveur sur le port 32400, jamais plex.tv. Il
n'arrête jamais un flux, ne scanne jamais une bibliothèque et ne change jamais
un réglage.

Plex ne publie pas le résultat de ses tâches de maintenance (le « butler » ne
donne que leur calendrier, dont DumbMonit lit si la sauvegarde de la base est
programmée) ni l'espace libre de ses disques, il n'y a donc rien à en dire.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_plex_`.

| Métrique | Quoi | Labels |
|---|---|---|
| `version_info` | valeur 1 | `version` |
| `update_available` | 1 quand la propre vérification de mise à jour de Plex a trouvé une release plus récente ; absent jusqu'à ce que Plex ait vérifié une fois | `latest_version` |
| `claimed` | 1 quand le serveur est lié à un compte Plex | |
| `remote_access_failed` | 1 quand Plex signale l'échec de son accès distant (mappage de port) ; absent quand le serveur n'est pas lié à plex.tv | |
| `signin_ok` | 1 tant qu'un serveur revendiqué est connecté à son compte plex.tv ; 0 quand la connexion est perdue ou expirée | |
| `database_backup_enabled` | 1 quand la tâche de maintenance « Backup database » est activée | |
| `streams`, `streams_paused`, `streams_remote` | lectures en cours ; en pause ; depuis l'extérieur du réseau local | |
| `transcodes`, `transcodes_video`, `transcodes_hardware` | lectures réencodées (vidéo ou audio) ; avec la vidéo réencodée ; avec accélération matérielle | |
| `transcoder_active_video_sessions` | transcodages vidéo que le serveur lui-même compte | |
| `stream_bandwidth_bits_per_second`, `stream_remote_bandwidth_bits_per_second` | bande passante réservée par toutes les lectures ; par celles venues de l'extérieur du réseau | |
| `streaming_users` | utilisateurs distincts qui lisent quelque chose | |
| `transcode_speed_min` | vitesse du transcodage vidéo le plus lent en cours de lecture, en multiples du temps réel (sous 1, la lecture va se mettre en mémoire tampon) ; les transcodages freinés ou en pause sont ignorés ; absent quand rien ne transcode | |
| `transcode_errors` | sessions de transcodage qui signalent une erreur | |
| `libraries`, `libraries_scanning` | bibliothèques ; celles en cours de scan | |
| `activities` | activités en arrière-plan en cours (scans, analyse, mises à jour) | |
| `scrape_errors` | appels qui ont échoué à la dernière sonde | |

Les [règles intégrées](../alerting/rules.md#self-hosted-applications) qui
s'appliquent : mise à jour Plex disponible, transcodeur Plex trop lent, erreur
de transcodage Plex, accès distant Plex en échec, Plex déconnecté de plex.tv,
sauvegarde de base Plex désactivée, plus Surveillance inaccessible.

## La page de la surveillance

Le panneau au-dessus des graphiques lit ce que la sonde a stocké ; ouvrir la
page n'appelle jamais Plex. Il donne l'état de mise à jour et si le serveur
est revendiqué, puis les flux, transcodages, transcodages matériels, flux
distants, bande passante et bibliothèques.

## Donner à DumbMonit l'accès au serveur Plex

1. Dans Plex Web, ouvrez Settings → Network et cliquez sur Show Advanced.

2. Ajoutez l'adresse de DumbMonit à List of IP addresses and networks that are allowed without auth, puis enregistrez.

    ```
    10.0.0.20/32
    ```

3. Dans DumbMonit, saisissez l'adresse du serveur et choisissez No token.

4. Si DumbMonit n'est pas sur votre réseau, utilisez plutôt le jeton du propriétaire : ouvrez un film → ⋯ → Get Info → View XML et copiez X-Plex-Token dans l'adresse de la page.

Documentation de l'éditeur : <https://support.plex.tv/articles/204059436-finding-an-authentication-token-x-plex-token/>

### Pourquoi un réseau autorisé, ou le jeton du propriétaire

DumbMonit lit l'API du serveur lui-même, sur le port 32400, jamais plex.tv. Un serveur lié à un compte Plex demande un jeton (X-Plex-Token), et Plex ne sait pas en émettre un limité : c'est celui du propriétaire. Lister l'adresse de DumbMonit parmi celles autorisées sans authentification évite tout jeton.

DumbMonit lit seulement les sessions, les bibliothèques, le programme de mise à jour, la liste des activités et le calendrier de maintenance ; il n'arrête jamais un flux et ne change aucun réglage.

!!! warning
    Le jeton est celui de votre compte Plex et peut gérer le serveur et toutes les bibliothèques : préférez le réseau autorisé quand DumbMonit est sur votre propre réseau.

Le réseau autorisé est aussi ce qui fait répondre Plex sans jeton depuis
Docker : le conteneur de DumbMonit atteint Plex depuis l'adresse de son réseau
Docker, listez donc ce réseau, ou l'adresse de l'hôte quand Plex publie son
port sur l'hôte.

## Dépannage

- **Non autorisé (401)** : le serveur est revendiqué et veut un jeton, ou l'adresse de DumbMonit ne figure pas parmi les réseaux autorisés ; derrière Docker, listez l'adresse du réseau Docker, pas celle de l'hôte.
- **Pas de `transcode_speed_min`** : normal tant que rien ne transcode, ou que seuls des transcodages freinés ou en pause existent.
- **`remote_access_failed` vaut 1** : Plex n'a pas pu mapper son port ; vérifiez le routeur (UPnP ou redirection manuelle) dans Settings → Remote Access. Si vous ne voulez pas d'accès distant, désactivez-le là.
- **`signin_ok` vaut 0** : reconnectez le serveur depuis Settings → General ; le jeton que Plex détenait a été révoqué ou a expiré.
- **« Sauvegarde de base désactivée » alors que vous sauvegardez vous-même le dossier de données** : mettez la règle en sourdine ou désactivez-la ; elle ne lit que le réglage de Plex.
- **Pas de chiffre de mise à jour** : Plex n'a jamais vérifié (vérifications coupées, ou serveur non revendiqué) ; `update_available` reste absent.

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
