# Duplicati

!!! warning "Bêta"

    Construit à partir du code et de la documentation de Duplicati, pas
    encore testé sur un vrai serveur. Dites-nous ce que votre serveur répond
    autrement.

Un serveur Duplicati via son API : l'âge de la dernière sauvegarde réussie de
chaque tâche, son dernier résultat, sa durée, sa taille, ses avertissements et
erreurs, et la tâche en cours.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 8200 | TCP | Interface web et API (HTTP, par défaut) |

Duplicati n'a pas de comptes d'utilisateur, seulement le mot de passe de son
interface web. À partir de Duplicati 2.1, DumbMonit échange ce mot de passe
contre un jeton d'accès de courte durée (`POST /api/v1/auth/login`, puis
`Authorization: Bearer`), le garde dix minutes et se reconnecte si le serveur
le refuse.

!!! note "Duplicati 2.0"
    La version 2.0 s'authentifie par `/login.cgi` (un nonce, un sel et un
    hachage `salted-sha256`, puis des cookies de session). Ce mode n'est pas
    pris en charge : face à un serveur 2.0, DumbMonit indique que la route de
    connexion est absente et demande la 2.1 ou plus.

La liste des tâches (`GET /api/v1/backups`) est la seule lecture requise. Les
autres sont lues quand le serveur répond et ne font jamais échouer
l'interrogation :

- `GET /api/v1/serverstate` : la tâche en cours, et si le serveur est en pause.
- `GET /api/v1/progressstate` : l'avancement de cette tâche.
- `GET /api/v1/backup/{id}/log` : le résultat de la dernière exécution de
  chaque tâche (`Success`, `Warning`, `Error`, `Fatal`) avec ses compteurs
  d'avertissements et d'erreurs, pour les 50 premières tâches. Sans elle, un
  échec se déduit de `LastErrorDate` et `LastErrorMessage` de la tâche.

La liste des tâches porte déjà les métadonnées de chacune
(`LastBackupDate`, `LastBackupFinished`, `LastBackupDuration`,
`LastErrorMessage`, tailles) : `GET /api/v1/backup/{id}` n'est donc pas appelé.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_duplicati_`.

| Métrique | Labels | Signification |
|---|---|---|
| `backup_failed` | `backup`, `backup_id` | 1 quand la dernière exécution a échoué : son résultat est `Error` ou `Fatal` ou, sans le journal, une erreur est plus récente que la dernière exécution. Un avertissement n'est pas un échec. |
| `backup_running` | `backup`, `backup_id` | 1 pour la tâche en cours. |
| `backup_scheduled` | `backup`, `backup_id` | 1 quand la tâche a un planning. |
| `backup_last_success_age_seconds` | `backup`, `backup_id` | Depuis la dernière sauvegarde réussie ; absent pour une tâche qui n'a jamais réussi. |
| `backup_last_duration_seconds` | `backup`, `backup_id` | Durée de la dernière exécution. |
| `backup_source_bytes`, `backup_source_files` | `backup`, `backup_id` | Taille et nombre de fichiers de la source. |
| `backup_target_bytes`, `backup_versions` | `backup`, `backup_id` | Taille à destination et nombre de versions conservées. |
| `backup_last_result_info` | `backup`, `backup_id`, `result` | Valeur 1 : `success`, `warning`, `error`, `fatal`. |
| `backup_last_warnings`, `backup_last_errors` | `backup`, `backup_id` | Compteurs de la dernière exécution. |
| `backup_last_error_info` | `backup`, `backup_id`, `message` | Valeur 1 avec le dernier message d'erreur (200 caractères au plus), seulement tant que la tâche est en échec. |
| `backup_progress_percent` | `backup`, `backup_id` | Avancement de la tâche en cours. |
| `server_paused` | | 1 quand le serveur est en pause. |
| `backups`, `backups_failed`, `backups_running` | | Compteurs ; 200 tâches au plus sont décrites une par une. |

Les [règles intégrées](../alerting/rules.md#duplicati) qui s'appliquent :

- **Sauvegarde Duplicati en échec** (Avertissement, rappel quotidien).
- **Sauvegarde Duplicati trop ancienne** : une tâche *planifiée* n'a pas
  réussi depuis plus de deux jours (Avis, rappel quotidien).

## La page de la surveillance

Le panneau Sauvegardes dit en une phrase ce qui ne va pas, montre les
compteurs d'échecs et de tâches en cours et si le serveur est en pause, puis
liste les tâches — celles en échec d'abord — avec l'âge de leur dernière
réussite, leur taille et, pour une tâche en échec, le dernier message d'erreur.

## Laisser DumbMonit lire l'API de Duplicati

1. Dans l'interface web de Duplicati, ouvrez Settings et définissez un long mot de passe aléatoire pour l'interface : Duplicati 2.1 en exige un.
2. Si le serveur n'écoute que sur la machine elle-même, lancez-le avec ces options pour que DumbMonit puisse le joindre.

    ```
    --webservice-interface=any --webservice-allowed-hostnames=*
    ```

3. Dans DumbMonit, saisissez l'adresse de Duplicati (par exemple nas.lan) et collez ce mot de passe.

### Droits requis

- **Mot de passe de l'interface** — Duplicati n'a ni comptes ni mode lecture seule

!!! warning
    Duplicati 2.1 ou plus est requis : la version 2.0 s'authentifie par un mécanisme différent que DumbMonit n'utilise pas. Le mot de passe donne un accès complet : gardez le port 8200 sur votre réseau local.

### Pourquoi ces permissions

Duplicati ne connaît qu'un seul secret, le mot de passe de l'interface ; il ne
peut ni être limité à la lecture ni être rattaché à un compte nommé. DumbMonit
n'émet que des requêtes `GET`, hors la connexion.

## Identifiants

| Identifiant | Champs |
|---|---|
| Mot de passe de l'interface web | Le mot de passe de l'interface web de Duplicati (stocké chiffré). |

Adresse : le nom d'hôte ou l'IP du serveur, `hôte:port`, ou une URL. Options :

- **Protocole** (`http`) et **Port** (8200).
- **Accepter un certificat non vérifiable** : si le serveur est en HTTPS avec
  un certificat autosigné.
- **Délai par requête** (15 s).

## Dépannage

« Duplicati refused the password » : le mot de passe diffère de celui de
l'interface.

« this collector needs Duplicati 2.1 or later » : le serveur n'a pas
`/api/v1/auth/login` ; mettez-le à jour.

Le serveur répond « Invalid host header » ou un refus voisin : ajoutez le nom
que vous utilisez dans DumbMonit à `--webservice-allowed-hostnames`.

Une tâche n'affiche pas d'âge : elle n'a jamais terminé une sauvegarde avec succès.
