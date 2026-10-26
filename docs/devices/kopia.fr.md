# Kopia

!!! warning "Bêta"

    Construit à partir du code et du comportement de l'API du serveur Kopia,
    pas encore testé sur un vrai serveur. Dites-nous ce que votre serveur
    répond autrement.

Un serveur Kopia via son API REST : le dernier instantané de chaque source
(âge, erreurs, taille, fichiers), les sources qui ont manqué leur planning et
la connexion au dépôt.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 51515 | TCP | API du serveur (HTTPS, par défaut) |

DumbMonit lit deux routes du serveur lancé par `kopia server start`, avec
l'authentification HTTP Basic (nom d'utilisateur et mot de passe du serveur) :

- `GET /api/v1/repo/status` : le dépôt est-il connecté, son stockage et son
  chiffrement. Cette lecture valide les identifiants.
- `GET /api/v1/sources` : les sources que le serveur sauvegarde, chacune avec
  son état, sa planification et son dernier instantané (heures de début et de
  fin, taille, fichiers, erreurs). Lue seulement si le dépôt est connecté.

L'historique des instantanés de chaque source (`/api/v1/snapshots`) n'est pas
lu : le dernier instantané vient déjà avec la source, et c'est tout ce dont
les alertes ont besoin. Les sources à l'état `REMOTE` appartiennent à d'autres
machines qui utilisent le même dépôt : elles sont comptées, pas décrites.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_kopia_`.

| Métrique | Labels | Signification |
|---|---|---|
| `repo_connected` | | 1 quand le serveur est connecté à son dépôt. |
| `repo_readonly` | | 1 pour un dépôt en lecture seule. |
| `repo_info` | `storage`, `hash`, `encryption` | Valeur 1, seulement si connecté. |
| `source_has_snapshot` | `source` | 1 dès que la source a un instantané. `source` s'écrit `utilisateur@machine:/chemin`. |
| `source_scheduled` | `source` | 1 quand Kopia déclenche lui-même la source (un intervalle, des heures, une expression cron ou une prochaine échéance, et pas « manuel »). |
| `source_uploading` | `source` | 1 pendant qu'un instantané tourne. |
| `source_status_info` | `source`, `status` | Valeur 1 : `IDLE`, `PENDING`, `UPLOADING`, `PAUSED`, `UNKNOWN`. |
| `source_last_snapshot_age_seconds` | `source` | Depuis la fin du dernier instantané ; absent pour une source sans instantané. |
| `source_last_snapshot_duration_seconds` | `source` | Durée du dernier instantané. |
| `source_last_snapshot_errors` | `source` | Fichiers que le dernier instantané n'a pas pu lire. |
| `source_last_snapshot_bytes`, `source_last_snapshot_files` | `source` | Taille et nombre de fichiers du dernier instantané. |
| `sources`, `sources_without_snapshot`, `sources_with_errors`, `sources_uploading`, `sources_remote` | | Compteurs ; 200 sources au plus sont décrites une par une. |

Les [règles intégrées](../alerting/rules.md#kopia) qui s'appliquent :

- **Instantané Kopia avec erreurs** : le dernier instantané d'une source s'est
  terminé avec des erreurs (Avis, rappel quotidien).
- **Instantané Kopia trop ancien** : une source *planifiée* n'a pas
  d'instantané depuis plus de deux jours (Avis, rappel quotidien). Une source
  manuelle n'est jamais signalée.
- **Dépôt Kopia déconnecté** : le serveur a perdu son dépôt (Avertissement).

## La page de la surveillance

Le panneau Sauvegardes dit en une phrase ce qui ne va pas, montre la
connexion au dépôt et le nombre de sources, puis liste les sources — celles en
retard ou avec des erreurs d'abord — avec l'âge, les fichiers illisibles et la
taille de leur dernier instantané.

## Lancer le serveur Kopia avec son propre nom d'utilisateur et mot de passe

1. Sur la machine qui fait les sauvegardes, lancez le serveur Kopia avec un certificat généré et son propre nom d'utilisateur et mot de passe.

    ```
    kopia server start --address=0.0.0.0:51515 --tls-generate-cert --server-username=dumbmonit --server-password=PASTE-A-LONG-RANDOM-PASSWORD
    ```

2. Vérifiez que le dépôt est connecté à ce serveur et que le serveur DumbMonit peut joindre le port 51515.
3. Dans DumbMonit, saisissez l'adresse du serveur (par exemple nas.lan) avec ce nom d'utilisateur et ce mot de passe.
4. Cochez Accepter un certificat non vérifiable : le certificat généré est autosigné. Si le serveur tourne avec --insecure, choisissez plutôt le protocole http.

### Droits requis

- **Nom d'utilisateur et mot de passe du serveur** — Kopia n'a pas de compte d'API en lecture seule

!!! warning
    L'API de Kopia n'a pas de compte en lecture seule : le mot de passe du serveur permet aussi de lancer et de supprimer des instantanés. DumbMonit ne fait que lire, mais gardez le port 51515 sur votre réseau local.

### Pourquoi ces permissions

Le serveur Kopia connaît un seul jeu d'identifiants pour son interface web et
son API, sans variante en lecture seule. DumbMonit n'émet que des requêtes
`GET`. Un serveur lancé sans mot de passe (`--without-password`, sur un réseau
de confiance seulement) est pris en charge : choisissez « Aucune
authentification ».

## Identifiants

| Identifiant | Champs |
|---|---|
| Nom d'utilisateur et mot de passe du serveur | Le `--server-username` et le `--server-password` du serveur Kopia. |
| Aucune authentification | Pour un serveur lancé sans mot de passe. |

Adresse : le nom d'hôte ou l'IP du serveur, `hôte:port`, ou une URL. Options :

- **Protocole** (`https`) et **Port** (51515).
- **Accepter un certificat non vérifiable** : pour le certificat créé par
  `--tls-generate-cert`.
- **Délai par requête** (15 s).

## Dépannage

« Kopia refused the credentials (401) » : le nom d'utilisateur ou le mot de
passe diffère de `--server-username` / `--server-password`.

`repo_connected` vaut 0 : le serveur tourne mais son dépôt n'est pas connecté
(`kopia repository status` sur le serveur).

Une source n'affiche jamais d'âge : elle n'a pas encore d'instantané, ou sa
seule entrée est une entrée `REMOTE` d'une autre machine.
