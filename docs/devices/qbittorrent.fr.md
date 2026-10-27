# qBittorrent (beta)

Si qBittorrent télécharge : torrents en erreur ou bloqués, connectivité perdue (port fermé, VPN tombé), vitesses, espace libre du dossier de téléchargement et nombre de torrents par état.

!!! note "Bêta"
    Écrit d'après la documentation d'API de l'éditeur et testé sur des réponses réalistes enregistrées, pas encore sur une instance réelle. Dites-nous ce que vous observez.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 8080 | TCP | Interface web (HTTP, par défaut) |
| 443 | TCP | Seulement avec le schéma https derrière un reverse proxy |

La panne à surveiller : **des torrents en erreur qui ne reprennent jamais**. Un disque plein ou un dossier déplacé met des torrents en erreur ou en fichiers manquants ; qBittorrent continue de tourner et ne les montre que dans sa liste. Un port fermé ou un VPN tombé (« firewalled » ou « disconnected ») arrête tout aussi discrètement.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_qbittorrent_`.

| Métrique | Quoi | Labels |
|---|---|---|
| `version_info` | valeur 1 | `version` |
| `connected` | 1 tant que qBittorrent est connecté au réseau (0 si firewalled ou disconnected) | `status` |
| `download_speed_bytes`, `upload_speed_bytes` | vitesses actuelles, en octets par seconde |  |
| `free_space_bytes` | espace libre là où qBittorrent enregistre ses téléchargements |  |
| `dht_nodes` | nœuds DHT connus |  |
| `torrents_total` | torrents dans la liste |  |
| `torrents` | torrents par groupe | `state` (`downloading`, `stalled_download`, `seeding`, `paused`, `queued`, `checking`, `error`, `other`) |
| `torrents_errored` | torrents en erreur ou aux fichiers manquants |  |
| `torrent_error` | valeur 1 par torrent en erreur, 32 au plus | `torrent`, `state` |

Les [règles intégrées](../alerting/rules.md#self-hosted-applications) qui s'appliquent : qBittorrent torrents in error, qBittorrent disconnected, qBittorrent free space low, plus Appareil inaccessible.

## Donner accès à l'interface web de qBittorrent à DumbMonit

1. Dans qBittorrent, ouvrez Tools → Options → Web UI et cochez Web User Interface (Remote control).

2. Sous Authentication, définissez un nom d'utilisateur et un mot de passe nommés comme suit, ou autorisez l'adresse de DumbMonit dans le contournement des sous-réseaux IP autorisés.

    ```
    dumbmonit
    ```

3. Dans DumbMonit, saisissez l'adresse de qBittorrent et ce nom d'utilisateur et mot de passe, ou choisissez Aucune authentification pour une adresse contournée.

!!! warning
    qBittorrent bannit une adresse pendant une heure après cinq connexions échouées (HTTP 403) : corrigez le mot de passe avant la prochaine sonde, sinon elle reste refusée.

## Identifiants

| Identifiant | Champs |
|---|---|
| Nom d'utilisateur / mot de passe | Le compte de l'interface web. DumbMonit ouvre une session avec `POST /api/v2/auth/login` et réutilise le cookie le temps de la sonde. |
| Aucune authentification | Pour une adresse que qBittorrent laisse entrer sans connexion (option des sous-réseaux autorisés). |

Adresse : un nom d'hôte (`qbittorrent.lan`), `host:port`, ou une URL complète avec préfixe de chemin. Options : protocole (HTTP par défaut), port (8080), vérification du certificat et délai de requête (15 s).
