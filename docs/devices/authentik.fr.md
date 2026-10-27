# authentik (beta)

Si authentik peut encore connecter les gens et faire tourner son travail de fond : disponibilité, workers, tâches système en échec, avant-postes qui ne répondent plus, mises à jour et nombre d'utilisateurs.

!!! note "Bêta"
    Écrit d'après la documentation d'API de l'éditeur et testé sur des réponses réalistes enregistrées, pas encore sur une instance réelle. Dites-nous ce que vous observez.

## Ports réseau à ouvrir

Du serveur DumbMonit (ou de son agent relais) vers l'équipement :

| Port | Protocole | Usage |
|---|---|---|
| 9000 | TCP | API (HTTP, par défaut) |
| 9443 | TCP | Seulement avec le schéma https sur le port HTTPS propre d'authentik |
| 443 | TCP | Seulement avec le schéma https derrière un reverse proxy |

La panne à surveiller : **un travail de fond qui s'arrête sans bruit**. Un worker qui a disparu, c'est plus de synchronisation des sources, plus de courriels, plus de nettoyage, avec seulement un compteur dans l'administration. Un avant-poste qui ne répond plus emporte avec lui toutes les applications qu'il protège.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_authentik_`.

| Métrique | Quoi | Labels |
|---|---|---|
| `live`, `ready` | 1 tant que `/-/health/live/` / `/-/health/ready/` répondent 2xx (ready couvre la base et le cache) |  |
| `version_info` | valeur 1 | `version` |
| `update_available` | 1 quand une version plus récente existe (absent quand authentik ne peut pas lire la dernière version) | `latest_version` |
| `workers`, `workers_version_mismatch` | workers qui répondent, et ceux d'une autre version que le serveur |  |
| `tasks_failed`, `tasks_warning` | tâches système dont la dernière exécution a échoué / s'est terminée avec un avertissement |  |
| `task_failed` | valeur 1 par tâche en échec, 32 au plus | `task` |
| `outposts_total`, `outposts_disconnected` | avant-postes qui servent au moins un fournisseur, et ceux dont aucune instance n'a été vue en cinq minutes |  |
| `outpost_instances` | instances vues dans les cinq dernières minutes (10 avant-postes au plus) | `outpost` |
| `users` | comptes utilisateurs |  |
| `scrape_errors` | appels qui ont échoué à la dernière sonde |  |

Les [règles intégrées](../alerting/rules.md#self-hosted-applications) qui s'appliquent : authentik not ready, authentik has no worker, authentik task failed, authentik outpost disconnected, authentik update available, plus Appareil inaccessible.

## Créer un jeton d'API dans authentik

1. Dans authentik, ouvrez Directory → Users → Create a service account et nommez-le comme suit, sans mot de passe aléatoire.

    ```
    dumbmonit
    ```

2. Placez cet utilisateur dans un groupe qui peut ouvrir l'interface d'authentik, pour qu'il puisse lire les pages système.

3. Ouvrez Directory → Tokens and App passwords, créez un jeton pour cet utilisateur avec l'intention API Token et sans expiration, et copiez sa clé.

4. Dans DumbMonit, saisissez l'adresse d'authentik et collez la clé.

!!! warning
    authentik n'a pas de rôle en lecture seule pour ses pages système : l'utilisateur du jeton doit pouvoir entrer dans l'interface, gardez donc la clé uniquement dans DumbMonit. Les pages que le jeton ne peut pas lire sont sautées, sans échec.

## Identifiants

| Identifiant | Champs |
|---|---|
| Jeton d'API | Envoyé en `Authorization: Bearer …`. |

Adresse : un nom d'hôte (`auth.lan`), `host:port`, ou une URL complète (`https://auth.example.com`). Options : protocole (HTTP par défaut), port (9000), vérification du certificat et délai de requête (15 s).
