# Forgejo / Gitea

Forgejo et Gitea partagent la même API REST, DumbMonit lit donc les deux avec
un seul collecteur : si l'instance répond, son contrôle de santé, combien de
dépôts, comptes et organisations elle contient, si ses tâches planifiées
(contrôles de dépôt, nettoyages, le vérificateur de mise à jour) tournent
toujours sur leur propre intervalle, et si ses runners Actions sont en ligne.

Deux appels doivent réussir : `/api/v1/version`, public, et `/api/v1/user`, la
preuve que le **jeton d'accès** est valide. Tout le reste — le contrôle de
santé, les compteurs de dépôts/comptes/organisations, les tâches planifiées et
les runners Actions — est lu depuis des points de terminaison que Forgejo et
Gitea réservent aux comptes administrateurs du site, et est ignoré sans faire
échouer la sonde quand le jeton appartient à un compte ordinaire.

La panne à surveiller ici : **une tâche planifiée qui a arrêté de tourner**.
Forgejo et Gitea continuent de servir les pages et les opérations git tandis
que leur planificateur interne est bloqué ; personne ne le remarque jusqu'à ce
que le vérificateur de mise à jour ou un contrôle de cohérence de dépôt n'ait
pas tourné depuis des semaines.

## Ce qu'il surveille

Toutes les métriques sont préfixées `dumbmonit_forgejo_`.

| Métrique | Quoi | Labels |
|---|---|---|
| `version_info` | valeur 1 | `version` |
| `token_is_admin` | 1 quand le compte du jeton est administrateur du site | |
| `healthz_ok` | 1 sauf si le contrôle de santé rapporte `fail` | |
| `healthz_check` | 1 par composant en échec (base de données, cache) | `check` |
| `repos` | dépôts visibles par le compte du jeton | |
| `users`, `orgs` | comptes et organisations sur l'instance | |
| `cron_tasks` | tâches planifiées définies sur l'instance | |
| `cron_task_overdue` | 1 par tâche dont la prochaine exécution est déjà due, 0 sinon | `task` |
| `runners_total`, `runners_online`, `runners_offline` | runners Actions connus de l'instance | |
| `runner_offline` | valeur 1 par runner hors ligne (32 au plus) | `runner` |
| `scrape_errors` | appels réservés aux administrateurs qui ont échoué à la dernière sonde (0 avec les appels d'un jeton ordinaire jamais tentés) | |

Les [règles intégrées](../alerting/rules.md#self-hosted-forges) qui
s'appliquent : runners Forgejo/Gitea tous hors ligne, tâche planifiée
Forgejo/Gitea en retard, plus Surveillance inaccessible. Un score de sécurité
vérifie les tâches planifiées : voir
[Mises à jour fournisseur et autres contrôles](../using/security-score.md).

## Ce qu'il ne surveille pas

Forgejo et Gitea n'ont pas de rôle administrateur en lecture seule : chaque
appel réservé à « admin » demande un compte avec tous les droits, pas un
délégué limité en portée. Leur API n'a non plus aucun équivalent des réglages
d'inscription ou de double authentification de GitLab, ni un indicateur
« mise à jour disponible » auto-déclaré — comparez la version ci-dessus aux
[notes de version](https://forgejo.org/releases/) du projet à la main.

## La page de la surveillance

Le panneau au-dessus des graphiques lit ce que la sonde a stocké ; ouvrir la
page n'appelle jamais Forgejo ou Gitea. Il dit d'abord ce qui ne va pas (le
contrôle de santé en échec, les runners tous hors ligne, une tâche planifiée
en retard), puis la version, puis les compteurs de dépôts, comptes et
organisations.

## Créer un jeton d'accès sur un compte administrateur du site

1. Créez un jeton d'accès sur un compte administrateur du site : avatar → Settings → Applications → Generate new token. Nommez-le comme suit et cochez les portées read:admin, read:repository et read:user.

    ```
    dumbmonit
    ```

2. read:admin ouvre les compteurs d'administration, les tâches planifiées et les runners Actions ; sans elle, seule la version et le propre compte du jeton sont lus, et le reste est ignoré sans faire échouer la sonde. Forgejo et Gitea n'ont pas non plus de rôle administrateur en lecture seule.

3. Dans DumbMonit, saisissez l'adresse, par exemple « forgejo.lan » (port 3000) ou « https://git.example.com » derrière un reverse proxy, et collez le jeton.

!!! warning
    Un jeton d'accès sur un compte administrateur du site peut lire et changer tout ce que le compte peut faire, quelles que soient les portées cochées : Forgejo et Gitea ne filtrent que quelles routes d'API une portée ouvre, pas ce qu'un compte administrateur peut faire lui-même. Dédiez-lui un compte bot plutôt que de réutiliser le vôtre.

Vérifier le jeton à la main, depuis l'hôte DumbMonit :

```
curl --header 'Authorization: token YOUR-TOKEN' 'https://git.example.com/api/v1/version'
```

## Identifiants

| Identifiant | Champs |
|---|---|
| Jeton d'accès | Envoyé comme `Authorization: token …`. |

Adresse : un nom d'hôte (`forgejo.lan`) ou `host:port`. Options : protocole
(HTTP par défaut — mettez un reverse proxy avec TLS devant pour tout ce qui
dépasse un LAN), port (3000), vérification du certificat et délai de requête
(15 s).

Validé avec Forgejo 9.0 (source de l'API officielle et définitions Swagger,
partagées avec Gitea).
