# GitLab (self-managed)

Si une instance GitLab self-managed sert bien, et si les parties qui portent
discrètement tout le reste tiennent le rythme : les files de Sidekiq, les
runners CI/CD, les migrations de base de données restées d'une mise à jour,
et — pour GitLab Enterprise Edition — la licence.

DumbMonit lit `/api/v4/version` avec un **jeton d'accès personnel**, le seul
appel qui doit réussir : c'est la preuve que GitLab répond et que le jeton
est valide. Tout le reste — Sidekiq, les runners, les migrations, les
statistiques d'instance, les réglages d'inscription et de double
authentification, la licence, et le contrôle de disponibilité en détail —
est lu depuis des points de terminaison que GitLab réserve aux comptes
administrateurs, et est ignoré sans faire échouer la sonde quand le jeton
appartient à un compte ordinaire.

La panne à surveiller ici : **Sidekiq qui prend du retard**. GitLab continue
de servir les pages tandis que sa file de workers en arrière-plan grandit, et
rien sur la page ne le dit jusqu'à ce qu'une notification, un webhook ou une
mise à jour de pipeline CI arrive en retard de minutes ou d'heures.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_gitlab_`.

| Métrique | Quoi | Labels |
|---|---|---|
| `version_info` | valeur 1 | `version`, `revision` |
| `readiness_check` | 1 quand un composant de disponibilité (base de données, cache, files, état partagé, Gitaly) n'est pas `ok` | `check` |
| `sidekiq_queue_backlog`, `sidekiq_queue_latency_seconds` | jobs en attente et leur ancienneté, par file Sidekiq | `queue` |
| `runners_total`, `runners_online`, `runners_offline` | runners CI/CD connus de l'instance | |
| `runner_offline` | valeur 1 par runner hors ligne (32 au plus) | `runner` |
| `migrations_pending` | migrations de base de données restant à appliquer | |
| `migration_pending` | valeur 1 par migration en attente (32 au plus) | `migration` |
| `projects`, `users`, `groups`, `forks`, `issues`, `merge_requests` | compteurs à l'échelle de l'instance | |
| `signup_enabled` | 1 tant que l'inscription publique est ouverte | |
| `two_factor_required` | 1 tant que chaque utilisateur doit configurer la double authentification | |
| `license_info` | valeur 1 (Enterprise Edition seulement) | `plan` |
| `license_expired`, `license_expiry_seconds` | si la licence a expiré, et le temps restant | |
| `pipelines_failed_recent`, `pipelines_pending` | pipelines en échec dans la fenêtre d'historique, et pipelines en attente ou en cours — seulement pour les projets listés dans l'option « Projets surveillés pour la santé CI » | `project` |
| `scrape_errors` | appels réservés aux administrateurs qui ont échoué à la dernière sonde (0 avec les appels d'un jeton ordinaire jamais tentés) | |

Les [règles intégrées](../alerting/rules.md#self-hosted-forges) qui
s'appliquent : runners GitLab tous hors ligne, retard Sidekiq GitLab élevé,
migrations GitLab en attente, licence GitLab arrivant à expiration, plus
Surveillance inaccessible. Un score de sécurité vérifie les réglages de double
authentification et d'inscription, et les migrations en attente : voir
[Mises à jour fournisseur et autres contrôles](../using/security-score.md).

## Ce qu'il ne surveille pas

La propre API REST de GitLab n'a aucun point de terminaison qui dit « une
release GitLab plus récente est disponible » : à la différence de Nextcloud
ou Paperless-ngx, il n'y a pas d'indicateur de mise à jour auto-déclaré à
lire, DumbMonit n'en prétend donc pas. Comparez la version ci-dessus à la
[page des releases](https://about.gitlab.com/releases/) de GitLab à la main,
ou surveillez le propre avis de vérification de mise à jour de GitLab dans
la zone d'administration.

La santé des pipelines CI n'est lue que pour les projets que vous listez :
l'API de GitLab n'a pas de point de terminaison « tous les pipelines en
échec » à l'échelle de l'instance, seulement un par projet.

## La page de la surveillance

Le panneau au-dessus des graphiques lit ce que la sonde a stocké ; ouvrir la
page n'appelle jamais GitLab. Il dit d'abord ce qui ne va pas (un composant
de disponibilité en échec, les runners tous hors ligne, un gros retard
Sidekiq), puis la version, puis les compteurs d'instance et les réglages lus
depuis l'API d'administration.

## Créer un jeton d'accès personnel sur un compte administrateur

1. Créez un jeton d'accès personnel sur un compte administrateur : avatar → Edit profile → Access tokens → Add new token. Nommez-le comme suit, fixez une date d'expiration que vous êtes à l'aise de renouveler, et cochez seulement la portée read_api : elle lit toute l'API et n'écrit rien.

    ```
    dumbmonit
    ```

2. Les files Sidekiq, les runners CI, les migrations en attente, les statistiques d'instance, les réglages d'inscription et de double authentification et la licence sont chacun lus depuis un point de terminaison réservé aux administrateurs : GitLab n'a pas de délégation plus fine pour eux. Un jeton d'un compte ordinaire fonctionne quand même ; seule la version est alors lue, et le reste est ignoré sans faire échouer la sonde.

3. Dans DumbMonit, saisissez l'adresse avec laquelle vous ouvrez GitLab, par exemple « gitlab.example.com » (HTTPS sur 443) ou « https://git.example.com » derrière un reverse proxy, et collez le jeton.

4. Pour surveiller aussi les pipelines CI, listez les identifiants numériques de projet (affichés sur la page de vue d'ensemble de chaque projet, sous son nom) dans l'option « Projets surveillés pour la santé CI », séparés par des virgules.

!!! warning
    Un jeton d'accès personnel sur un compte administrateur peut lire tout ce que le compte peut lire, même avec la portée read_api : GitLab n'a pas de rôle administrateur en lecture seule. Dédiez-lui un compte bot plutôt que de réutiliser le vôtre, et révoquez le jeton s'il fuit un jour.

Vérifier le jeton à la main, depuis l'hôte DumbMonit :

```
curl --header 'PRIVATE-TOKEN: YOUR-TOKEN' 'https://gitlab.example.com/api/v4/version'
```

## Identifiants

| Identifiant | Champs |
|---|---|
| Jeton d'accès personnel | Envoyé comme en-tête `PRIVATE-TOKEN`. |

Adresse : un nom d'hôte (`gitlab.example.com`) ou `host:port`. Options :
protocole (HTTPS par défaut), port (443), vérification du certificat, délai
de requête (15 s), les projets surveillés pour la santé CI (aucun par
défaut) et la fenêtre d'historique des pipelines en échec (24 h).

Validé avec GitLab Enterprise Edition 17.5 (documentation officielle de
l'API).
