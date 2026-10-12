# Battement de cœur (push)

La panne silencieuse que personne ne remarque : la sauvegarde nocturne qui a
arrêté de tourner après une mise à jour, le cron de renouvellement de
certificat qui a perdu ses permissions, l'automatisation Home Assistant
cassée après un renommage. Rien ne se plaint, et vous le découvrez le jour
où vous avez besoin de la sauvegarde.

Un moniteur **battement de cœur** (aussi appelé *dead man's switch*, ou un
moniteur *push* dans Uptime Kuma) renverse la logique : DumbMonit ne
contacte rien. La tâche appelle une URL secrète chaque fois qu'elle
tourne, et c'est l'**absence** d'appel qui déclenche l'alerte.

## Comment ça fonctionne

Chaque appareil battement de cœur obtient une URL de la forme
`https://<votre DumbMonit>/api/push/<token>`. Le jeton est une valeur
aléatoire de 128 bits, stockée hashée pour la recherche et chiffrée pour
l'affichage : il est montré sur la page de l'appareil, avec un bouton de
copie, tant que l'appareil existe. Il est délibérément ouvert — pas de
session, pas de jeton d'API, pas d'en-tête `X-Requested-With` — parce
qu'une ligne `curl` dans une crontab n'a rien de tout ça. Détenir le jeton
permet seulement de dire « la tâche a tourné ».

À chaque passage du planificateur (l'intervalle de l'appareil, 60 s par
défaut) le collecteur lit l'heure du dernier appel et écrit :

| Situation | `probe_success` | Effet |
|---|---|---|
| Aucun appel reçu encore | *(rien n'est écrit)* | L'appareil montre **En attente**. Aucune alerte : la tâche n'est pas encore en place. |
| Dernier appel dans l'intervalle attendu + grâce | `1` | **Actif**. |
| Dernier appel plus ancien que l'intervalle attendu + grâce | `0`, `reason="missed"` | **En panne** ; la règle « Battement de cœur manqué » se déclenche après deux minutes. |
| Dernier appel disait `status=down` | `0`, `reason="reported_down"` | **En panne** immédiatement, quelle que soit l'horloge. |

Un appel réévalue aussi l'appareil immédiatement, si bien qu'une tâche qui
reprend efface son alerte sans attendre le prochain passage du
planificateur.

Le point de terminaison répond `204` avec un corps vide quand l'appel est
enregistré, `404` pour un jeton inconnu ou régénéré, `400` pour un
`status` qui n'est ni `up` ni `down`, et `429` (avec `Retry-After`) au-delà
de soixante appels par minute pour un jeton. `GET` et `POST` sont tous les
deux acceptés ; `HEAD` fonctionne aussi.

### Paramètres de requête

| Paramètre | Signification |
|---|---|
| `status` | `up` (défaut) ou `down` : la tâche rapporte elle-même un échec. |
| `msg` | Un mot libre d'explication, conservé (tronqué à 500 caractères) et montré sur la page de l'appareil. |

Ce sont les paramètres du moniteur push d'Uptime Kuma : un script écrit
pour lui fonctionne sans changement.

## Configuration

Les étapes ci-dessous sont celles que montre l'avis à côté du formulaire.

1. Dans l'adresse, écrivez un court libellé pour la tâche
   (« nightly-backup », « certbot-renew ») : rien n'est contacté, le
   libellé doit juste être unique parmi vos battements de cœur.
2. Réglez l'intervalle attendu sur le calendrier de la tâche (« 24h » pour
   une tâche nocturne, « 1h » pour une horaire) et, si le calendrier
   dérive, une période de grâce plus large. Enregistrez l'appareil.
3. La page de l'appareil montre l'URL à appeler. Ajoutez-la à la fin de la
   tâche, pour qu'elle ne soit appelée que quand la tâche a réussi :

    ```
    curl -fsS -m 10 --retry 3 https://monit.example.com/api/push/<token>
    ```

4. Une tâche qui peut dire quand elle a échoué peut le dire plutôt que de
   rester silencieuse : ajoutez ?status=down&msg=… à l'URL, et l'alerte se
   déclenche tout de suite.
5. Jusqu'au premier appel, l'appareil montre « En attente » et rien n'est
   alerté. URL perdue ou qui a fuité ? « Régénérer » sur la page de
   l'appareil en émet une nouvelle ; l'ancienne arrête de répondre
   immédiatement.

!!! warning
    Appelez l'URL à la fin de la tâche, après la partie qui compte. Un
    appel placé en haut rapporterait un succès même quand la sauvegarde
    elle-même a échoué.

Identifiants : aucun.

### Exemples

Une ligne de crontab, chaque nuit à 03:00, qui n'appelle que quand la
sauvegarde a réussi :

```
0 3 * * * /usr/local/bin/backup.sh && curl -fsS -m 10 --retry 3 https://monit.example.com/api/push/<token> > /dev/null
```

Un script qui rapporte son propre échec, avec le code de sortie comme
message :

```sh
#!/bin/sh
URL="https://monit.example.com/api/push/<token>"
if /usr/local/bin/backup.sh; then
    curl -fsS -m 10 --retry 3 "$URL" > /dev/null
else
    curl -fsS -m 10 --retry 3 "$URL?status=down&msg=backup+exit+$?" > /dev/null
fi
```

Home Assistant, une `rest_command` appelée à la fin d'une automatisation :

```yaml
rest_command:
  dumbmonit_heartbeat:
    url: "https://monit.example.com/api/push/<token>"
    method: post
```

Windows Task Scheduler (PowerShell) :

```powershell
Invoke-WebRequest -UseBasicParsing -TimeoutSec 10 "https://monit.example.com/api/push/<token>" | Out-Null
```

### Options

| Clé | Libellé | Défaut | Aide |
|---|---|---|---|
| `expected_interval` | Intervalle attendu | `24h` | À quelle fréquence la tâche est censée appeler : 30m, 1h, 6h, 24h, 7d (ou un nombre de secondes). Un appel manqué est déclaré une fois cet intervalle plus la période de grâce passé. |
| `grace` | Période de grâce | `10%` | Temps supplémentaire toléré après l'intervalle attendu avant que le battement de cœur ne compte comme manqué : un pourcentage de l'intervalle (10 %) ou une durée fixe (15m). Jamais moins d'une minute. |

Les durées acceptent les suffixes `s`, `m`, `h`, `d`, `w` et peuvent être
combinées (`1h30m`) ; un nombre seul est un nombre de secondes. L'intervalle
attendu doit être entre 10 secondes et un an.

## Métriques

Toutes étiquetées `probe="push"` en plus de `target`, `host` et `tag_*`,
toutes préfixées `dumbmonit_`.

| Métrique | Signification |
|---|---|
| `probe_success` | 1 si le dernier appel est à l'heure et n'a pas rapporté d'échec, 0 sinon. Absent jusqu'au premier appel. |
| `probe_failure_info` | Présence (1) avec un label `reason` : `missed` ou `reported_down`. |
| `push_last_seen_seconds` | Ancienneté du dernier appel. |
| `push_received_total` | Appels reçus depuis la création du jeton (compteur). |

Disponibilité sur trente jours, comme pour tout moniteur de service :
`avg_over_time(dumbmonit_probe_success{probe="push"}[30d])`.

Règle intégrée qui s'applique : **Battement de cœur manqué** (avis, 2
minutes). Les règles « Service en panne » et « Service instable » ignorent
les battements de cœur, et « Appareil inaccessible » ne se déclenche
jamais pour eux : le collecteur tourne toujours, seul son verdict change.

## API

| Route | Session | Usage |
|---|---|---|
| `GET` ou `POST /api/push/<token>` | aucune | L'appel lui-même. `204`, `404`, `400` ou `429`. |
| `GET /api/targets/<id>/push` | requise | Le moniteur : `token`, `path`, `last_seen_at`, `last_seen_age_secs`, `last_status`, `last_message`, `received_total`, `expected_interval_secs`, `grace_secs`, `verdict` (`waiting`, `on_time`, `missed`, `reported_down`), `settings_error`. Créé à la première lecture si manquant. |
| `POST /api/targets/<id>/push/regenerate` | admin | Nouveau jeton ; l'URL précédente répond `404` à partir de là. Le compteur et le dernier appel sont conservés. |
