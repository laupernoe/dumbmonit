# Battement de cœur (push)

La panne silencieuse que personne ne remarque : la sauvegarde nocturne qui a
arrêté de tourner après une mise à jour, le cron de renouvellement de
certificat qui a perdu ses permissions, l'automatisation Home Assistant
cassée après un renommage. Rien ne se plaint, et vous le découvrez le jour
où vous avez besoin de la sauvegarde.

Une surveillance **battement de cœur** (aussi appelé *dead man's switch*, ou un
moniteur *push* dans Uptime Kuma) renverse la logique : DumbMonit ne
contacte rien. La tâche appelle une URL secrète chaque fois qu'elle
tourne, et c'est l'**absence** d'appel qui déclenche l'alerte.

## Ports réseau à ouvrir

De l'agent vers le serveur DumbMonit :

| Port | Protocole | Usage |
|---|---|---|
| 8080 | TCP | Appel du heartbeat (port par défaut, 443 derrière un proxy) |

## Comment ça fonctionne

Chaque surveillance battement de cœur obtient une URL de la forme
`https://<votre DumbMonit>/api/push/<token>`. Le jeton est une valeur
aléatoire de 128 bits, stockée hashée pour la recherche et chiffrée pour
l'affichage : il est montré sur la page de la surveillance, avec un bouton de
copie, tant que la surveillance existe. Il est délibérément ouvert — pas de
session, pas de jeton d'API, pas d'en-tête `X-Requested-With` — parce
qu'une ligne `curl` dans une crontab n'a rien de tout ça. Détenir le jeton
permet seulement de dire « la tâche a tourné ».

À chaque passage du planificateur (l'intervalle de la surveillance, 60 s par
défaut) le collecteur lit l'heure du dernier appel et écrit :

| Situation | `probe_success` | Effet |
|---|---|---|
| Aucun appel reçu encore | *(rien n'est écrit)* | La surveillance montre **En attente**. Aucune alerte : la tâche n'est pas encore en place. |
| Dernier appel dans l'intervalle attendu + grâce | `1` | **Actif**. |
| Dernier appel plus ancien que l'intervalle attendu + grâce | `0`, `reason="missed"` | **En panne** ; la règle « Battement de cœur manqué » se déclenche après deux minutes. |
| Dernier appel disait `status=down` | `0`, `reason="reported_down"` | **En panne** immédiatement, quelle que soit l'horloge. |

Un appel réévalue aussi la surveillance immédiatement, si bien qu'une tâche qui
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
| `msg` | Un mot libre d'explication, conservé (tronqué à 500 caractères) et montré sur la page de la surveillance. |
| `exit_code` | Le code de sortie de la tâche (voir plus bas). Tout code autre que `0` est un échec. |

`status` et `msg` sont les paramètres du moniteur push d'Uptime Kuma : un
script écrit pour lui fonctionne sans changement.

## Début, succès, échec et code de sortie

L'URL simple suffit à savoir qu'une tâche a tourné. Pour savoir aussi **combien
de temps** elle a pris, **si elle a échoué** et **ce qu'elle a affiché**,
appelez la même URL avec un suffixe :

| URL | Signification |
|---|---|
| `/api/push/<token>` | Une exécution réussie, de durée inconnue (l'appel d'origine, inchangé). |
| `/api/push/<token>/start` | La tâche démarre. Ouvre une exécution ; l'appel de fin suivant donne sa durée. |
| `/api/push/<token>/success` | La tâche a réussi. Comme l'appel simple, mais referme l'exécution ouverte. |
| `/api/push/<token>/fail` | La tâche a échoué. |
| `/api/push/<token>/<code>` | La tâche s'est terminée avec ce code de sortie, par exemple `/2`. `0` est un succès, tout autre code un échec. |

Le code de sortie peut aussi être donné par `?exit_code=2` sur `/success` ou
`/fail` ; un `/success` qui porte un code non nul compte comme un échec.

Avec `POST`, le **corps de la requête est la sortie de la tâche**. Elle est
conservée tronquée à ses **10 derniers Ko** (l'erreur est presque toujours à
la fin ; le texte est coupé sur une frontière de caractère, les octets
invalides sont remplacés). Un corps de plus de 256 Ko est refusé avec `413` :
passez la sortie dans `tail -c 10000`. Sans corps, `msg` est conservé comme
sortie.

La durée est le temps entre `/start` et l'appel de fin. Un appel sans
`/start` avant lui a une durée inconnue. Un nouveau `/start` alors qu'une
exécution est encore ouverte marque cette dernière comme *interrompue*. La
page de la surveillance liste les dernières exécutions (200 par surveillance,
90 jours au plus) avec leur statut, leur durée, leur code de sortie et leur
sortie.

Les mêmes garde-fous valent pour tous les points d'entrée : le jeton, soixante
appels par minute et par jeton, et un corps borné.

## Configuration

Les étapes ci-dessous sont celles que montre l'avis à côté du formulaire.

1. Dans l'adresse, écrivez un court libellé pour la tâche, unique parmi vos battements de cœur : « nightly-backup ».

2. Réglez l'intervalle attendu sur le calendrier de la tâche (« 24h » pour une tâche nocturne), ou donnez une expression cron et un fuseau horaire, puis enregistrez la surveillance.

3. Copiez l'URL affichée sur la page de la surveillance et appelez-la à la fin de la tâche.

    ```
    curl -fsS -m 10 --retry 3 https://monit.example.com/api/push/<token>
    ```

4. Pour chronométrer la tâche et rapporter son résultat, appelez .../start avant elle, puis .../success ou .../fail après, avec la sortie de la tâche comme corps de la requête. Un code de sortie peut être donné par ?exit_code=2 ou comme dernier segment du chemin : .../2.

5. Pour signaler un échec immédiatement, ajoutez ?status=down&msg=... à l'URL.

!!! warning
    Appelez l'URL à la fin de la tâche, après la partie qui compte : un appel placé au début signalerait un succès même quand la tâche échoue.

Rien n'est contacté : le libellé doit seulement être unique. Jusqu'au premier appel, la surveillance montre « En attente » et rien n'est alerté. URL perdue ou divulguée ? « Régénérer » sur la page de la surveillance en émet une nouvelle ; l'ancienne cesse de répondre immédiatement.

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

Un script enveloppe qui rapporte le début, le code de sortie et la sortie de
n'importe quelle commande (seuls les 10 derniers Ko de sortie sont envoyés).
Remplacez `/usr/local/bin/backup.sh` par la tâche :

```sh
#!/bin/sh
URL="https://monit.example.com/api/push/<token>"
curl -fsS -m 10 --retry 3 "$URL/start" > /dev/null
OUT=$(/usr/local/bin/backup.sh 2>&1); CODE=$?
printf '%s' "$OUT" | tail -c 10000 \
    | curl -fsS -m 10 --retry 3 --data-binary @- "$URL/$CODE" > /dev/null
exit "$CODE"
```

`$URL/$CODE` vaut `/0` après un succès et `/2`, `/23`… après un échec : un
seul appel porte le résultat, le code de sortie et la sortie.

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
| `schedule` | Planning cron | *(vide)* | À la place de l'intervalle : quand la tâche est censée démarrer, sous forme d'expression cron à 5 champs (minute heure jour-du-mois mois jour-de-semaine) ou de raccourci comme `@daily`. Vide : l'intervalle s'applique. |
| `timezone` | Fuseau horaire | `UTC` | Fuseau IANA dans lequel se lit le planning cron, par exemple `Europe/Paris`. Il suit l'heure d'été. |
| `max_duration` | Durée maximale | *(vide)* | Une exécution plus longue déclenche l'alerte « Tâche trop longue » : `30m`, `2h`. Demande un `/start` au début de la tâche. Vide : une exécution encore ouverte après une période plus la grâce est jugée bloquée. |

Les durées acceptent les suffixes `s`, `m`, `h`, `d`, `w` et peuvent être
combinées (`1h30m`) ; un nombre seul est un nombre de secondes. L'intervalle
attendu doit être entre 10 secondes et un an.

### Planning cron

Une tâche qui tourne « chaque jour à 02:30 » ou « en semaine à 08:00 » se
décrit mieux par une expression cron que par un intervalle, car l'heure
attendue ne dérive pas après une exécution tardive. Renseignez `schedule` (et
`timezone`, UTC par défaut) ; l'intervalle est alors ignoré.

- Cinq champs : minute (0-59), heure (0-23), jour du mois (1-31), mois (1-12
  ou `jan`-`dec`), jour de la semaine (0-7 ou `sun`-`sat`, 0 et 7 sont le
  dimanche).
- Chaque champ accepte `*`, une valeur, une plage (`8-17`), une liste
  (`1,15`) et un pas (`*/15`, `10-50/10`). Les noms ne sont pas sensibles à la
  casse.
- Raccourcis : `@yearly`, `@annually`, `@monthly`, `@weekly`, `@daily`,
  `@midnight`, `@hourly`.
- Comme dans cron, quand le jour du mois et le jour de la semaine sont tous
  deux restreints, une date convient si **l'un ou l'autre** correspond.
- Changement d'heure : quand une heure locale n'existe pas (passage à l'heure
  d'été), l'exécution est attendue juste après le trou ; quand elle existe
  deux fois (retour à l'heure d'hiver), elle est attendue une seule fois, à la
  première occurrence. Un planning sur toutes les heures (`0 * * * *`) suit le
  temps réel.

La tâche est **en retard** quand la dernière échéance plus la période de
grâce est passée sans appel (un `/start`, un `/success` ou un appel simple,
même une demi-minute en avance, compte). Une grâce en pourcentage est un
pourcentage de l'écart entre deux échéances. Un planning ou un fuseau
illisible est refusé à l'enregistrement de la surveillance, avec la raison.

## Métriques

Toutes étiquetées `probe="push"` en plus de `target`, `host` et `tag_*`,
toutes préfixées `dumbmonit_`.

| Métrique | Signification |
|---|---|
| `probe_success` | 1 si le dernier appel est à l'heure et n'a pas rapporté d'échec, 0 sinon. Absent jusqu'au premier appel. |
| `probe_failure_info` | Présence (1) avec un label `reason` : `missed` ou `reported_down`. |
| `push_last_seen_seconds` | Ancienneté du dernier appel. |
| `push_received_total` | Appels reçus depuis la création du jeton (compteur). |
| `push_run_failed` | 1 si la dernière exécution terminée a échoué (`/fail` ou code de sortie non nul), 0 sinon. Absent tant qu'aucune exécution n'est terminée. |
| `push_run_too_long` | 1 si une exécution dépasse la durée maximale (ouverte ou terminée), 0 sinon. Absent tant qu'aucune exécution n'est connue. |
| `push_last_run_duration_seconds` | Durée de la dernière exécution terminée, quand un `/start` l'a précédée. |

Disponibilité sur trente jours, comme pour toute surveillance de service :
`avg_over_time(dumbmonit_probe_success{probe="push"}[30d])`.

Règles intégrées qui s'appliquent (toutes au niveau avertissement) :

| Règle | Se déclenche quand |
|---|---|
| **Battement de cœur manqué** (`push_missed`) | La tâche n'a pas appelé avant son intervalle attendu ou son planning cron plus la période de grâce, ou a rapporté `status=down`. Deux minutes après l'échéance. |
| **Tâche en échec** (`push_failed`) | La tâche a bien appelé, mais sa dernière exécution a échoué : `/fail` ou code de sortie non nul. Le message porte le code de sortie et la fin de la sortie. Elle se résout à la prochaine exécution réussie, et reparle à chaque nouvel échec. |
| **Tâche trop longue** (`push_too_long`) | Une exécution dure plus que `max_duration` ou, sans elle, un `/start` est encore ouvert après une période plus la grâce. |

Une tâche silencieuse (rien n'arrive) et une tâche en échec (quelque chose
arrive, et c'est mauvais) sont donc deux alertes distinctes. Un planning mal
réglé est montré sur la surveillance et refusé à l'enregistrement, jamais
alerté comme une panne. Les règles « Service en panne » et « Service
instable » ignorent les battements de cœur, et « Surveillance inaccessible »
ne se déclenche jamais pour eux : le collecteur tourne toujours, seul son
verdict change.

## API

| Route | Session | Usage |
|---|---|---|
| `GET` ou `POST /api/push/<token>` | aucune | L'appel lui-même. `204`, `404`, `400`, `413` ou `429`. |
| `GET` ou `POST /api/push/<token>/start`, `/success`, `/fail`, `/<code>` | aucune | Début ou fin d'une exécution (voir plus haut). Mêmes réponses ; un suffixe inconnu donne `404`. |
| `GET /api/targets/<id>/push` | requise | La surveillance : `token`, `path`, `last_seen_at`, `last_seen_age_secs`, `last_status`, `last_message`, `received_total`, `expected_interval_secs`, `grace_secs`, `verdict` (`waiting`, `on_time`, `missed`, `reported_down`), `settings_error`, `schedule`, `timezone`, `next_expected_at`, `max_duration_secs`, `last_run`. Créé à la première lecture si manquant. |
| `GET /api/targets/<id>/push/runs?limit=&before=` | requise | L'historique des exécutions, de la plus récente à la plus ancienne : `runs` (`id`, `started_at`, `finished_at`, `duration_secs`, `status` parmi `running`, `success`, `failed`, `lost`, `exit_code`, `output`) et `next_before` pour la page suivante. |
| `POST /api/targets/<id>/push/regenerate` | admin | Nouveau jeton ; l'URL précédente répond `404` à partir de là. Le compteur et le dernier appel sont conservés. |
