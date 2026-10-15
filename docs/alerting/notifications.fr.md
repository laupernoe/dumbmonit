# Politique de notification

La promesse de DumbMonit est qu'une notification vaut la peine d'être
lue. Le moteur d'alerte garde le bruit bas avant qu'une alerte puisse
parler (suppression par dépendance, groupement, fenêtres de
maintenance) ; la politique de notification le garde bas après, par
canal. Rien n'a besoin d'être configuré : les défauts ci-dessous
s'appliquent dès la sortie de boîte.

## Le pipeline, en une liste

```
évaluer (hystérésis, surcharges par appareil)
  → dédupliquer → supprimer par dépendance → réduire au silence → grouper par appareil
    → maintien anti-flapping → filtre de gravité du canal → filtre de routage du canal
      → résolu activé/désactivé → délai de répit → heures calmes → fenêtre de regroupement
        → plafond horaire → envoyer (+ lien profond)
                                   ↘ personne n'a acquitté → canal d'escalade
```

| Étape | Où c'est réglé | Défaut |
|---|---|---|
| Seuil de résolution (hystérésis) | Éditeur de règle → *Se résout sous / au-dessus* | aucun (CPU : 85 %, disque : 88 % sur les règles fournies) |
| Surcharges par appareil | Éditeur de règle → *Surcharges par appareil* | aucune |
| Détection de flapping | Alertes → Notifications → Politique de notification → Plus d'options | 4 changements en 30 min, maintien 30 min |
| Gravité minimale | Canal → Options de livraison → *Envoyer* | tout |
| Me dire quand ça se résout | Canal → Options de livraison | activé |
| Intervalle minimal par alerte | Canal → Options de livraison | aucun |
| Heures calmes | Canal → Options de livraison | désactivé |
| Filtre de routage (étiquettes, type, règle) | Canal → Options de livraison → *Seulement certaines alertes* | désactivé (le canal entend tout) |
| Escalade si personne n'acquitte | Alertes → Notifications → Politique de notification | désactivé |
| Fenêtre de regroupement | Alertes → Notifications → Politique de notification | 60 s |
| Messages par canal par heure | Alertes → Notifications → Politique de notification | 20 |
| URL publique (liens profonds) | Alertes → Notifications → Politique de notification, ou `DUMBMONIT_PUBLIC_URL` | vide |
| Liens Acquitter / Pause dans les messages | Alertes → Notifications → Politique de notification | activés (si une URL publique est connue) |

## Hystérésis et surcharges par appareil

Une règle qui se déclenche au-dessus de 90 % et se résout dès que la
valeur descend sous 90 % se déclenchera et se résoudra toute la soirée
sur un CPU qui oscille entre 89 et 91 %. Donnez-lui un **seuil de
résolution** : une fois déclenchée, l'alerte ne se résout que quand la
valeur descend sous lui (pour les règles « se déclenche au-dessus ») ou
monte au-dessus (pour les règles « se déclenche en dessous »). Le champ
d'API est `clear_threshold` sur la règle ; le serveur refuse un seuil de
résolution du mauvais côté du seuil.

Une **surcharge par appareil** change le seuil ou le seuil de résolution
d'un appareil, ou désactive la règle pour cet appareil, sans dupliquer
la règle :

```
PUT /api/alerts/rules/{id}/overrides/{target_id}
{ "threshold": 97, "clear_threshold": 95 }      # ou { "enabled": false }
GET /api/alerts/rules/{id}/overrides
GET /api/alerts/overrides?target_id=7           # chaque surcharge d'un appareil
DELETE /api/alerts/rules/{id}/overrides/{target_id}
```

Une surcharge avec chaque champ vide est supprimée. Les messages citent
le seuil réellement appliqué.

## Détection de flapping

Quelle que soit la règle, une alerte dont l'empreinte s'est déclenchée ou
résolue 4 fois en 30 minutes est *instable* : un avis sort
(« instable : 4 changements en 30 min, notifications retenues pendant 30
min ») et rien d'autre sur cette alerte n'est envoyé jusqu'à la fin du
maintien. Une résolution après le maintien est quand même annoncée.
Ajustez ou désactivez-le dans Alertes → Notifications → Politique de
notification → Plus d'options (`flap_events`, `flap_window_secs`,
`flap_hold_secs` ; `flap_events: 0` le désactive).

## Options de livraison par canal

Chaque canal décide ce qu'il entend (`policy` dans la charge utile du
canal) :

- `min_severity` — `info`, `warning` (Avis et plus) ou `critical`
  (Avertissement seulement). Un canal téléphone veut généralement
  `critical`, une salle de chat tout.
- `notify_resolved` — `false` pour n'entendre que les problèmes.
- `min_interval_secs` — au moins ce temps entre deux messages sur la
  même alerte (re-déclenchements et rappels). Les résolutions ne sont
  jamais retardées par ça, et une résolution n'est envoyée qu'à un canal
  qui a entendu le déclenchement.
- `quiet_hours` — une fenêtre hebdomadaire dans votre heure locale, même
  forme qu'une fenêtre de maintenance hebdomadaire. Pendant les heures
  calmes, seules les alertes `critical` passent ; le reste attend et
  arrive comme un résumé unique à la fin, titré « Fin des heures calmes —
  … ». Une alerte qui se déclenche et se résout pendant les heures calmes
  n'est mentionnée dans ce résumé que comme « résolue pendant les heures
  calmes ».
- `matcher` — quelles alertes le canal veut, par étiquette, type
  d'appareil ou règle. Vide (le défaut, et ce que chaque canal existant
  garde) signifie tout.

```
PUT /api/notify/channels/{id}
{ "name": "Téléphone", "kind": "ntfy", "settings": {…},
  "policy": { "min_severity": "critical", "min_interval_secs": 900,
              "quiet_hours": { "kind": "weekly", "days": [0,1,2,3,4],
                               "start_minute": 1320, "end_minute": 420,
                               "timezone": "Europe/Paris" } } }
```

Omettre `policy` sur une mise à jour garde celle enregistrée ;
`"quiet_hours": null` les efface, et `"matcher": null` efface le filtre
de routage.

## Routage par étiquette

Les appareils portent des étiquettes (`site=cellar`, `role=storage`) et
un type (`snmp`, `proxmox`, `synology`, `agent`…). Le **filtre de
routage** d'un canal dit quelles alertes il veut :

```json
{ "policy": { "min_severity": "warning",
              "matcher": {
                "include": [ { "field": "tag", "key": "site", "value": "cellar" } ],
                "exclude": [ { "field": "tag", "key": "role", "value": "lab" } ] } } }
```

Trois champs, et seulement l'égalité exacte — pas d'expressions
régulières, pas de crochets :

| `field` | Correspond à | Exemple |
|---|---|---|
| `tag` | Une étiquette de la propre page de l'appareil | `{"field":"tag","key":"site","value":"cellar"}` |
| `kind` | Le collecteur qui sonde l'appareil | `{"field":"kind","value":"proxmox"}` |
| `rule` | L'`uid` de la règle | `{"field":"rule","value":"disk_full"}` |

Comment les listes se lisent :

- les conditions sur le **même** champ (la même clé d'étiquette, ou
  `kind`, ou `rule`) sont un **OU** — `site=cellar` plus `site=attic`
  signifie l'un ou l'autre ;
- les conditions sur des champs **différents** sont un **ET** —
  `site=cellar` plus `role=storage` signifie les deux ;
- **`exclude` gagne toujours**, même sur un `include` qui a correspondu.

L'exemple ci-dessus se lit donc, et l'UI vous le redit exactement en ces
mots : *ce canal reçoit les avis et plus depuis les appareils étiquetés
site=cellar, sauf les appareils étiquetés role=lab*.

Sous l'éditeur, un **aperçu** liste lesquels de vos appareils actuels le
filtre sélectionne. Ce n'est pas une approximation : le serveur lance le
filtre même que le moteur d'alerte lance, réduit à la partie qui dépend
de l'appareil seul. Les conditions de règle s'appliquent aux alertes
plutôt qu'aux appareils, l'aperçu les nomme donc séparément plutôt que de
prétendre les compter.

```
POST /api/notify/match-preview
{ "matcher": { "include": [ { "field": "tag", "key": "site", "value": "cellar" } ] } }

→ { "devices": [ { "id": 3, "name": "nas", "kind": "synology",
                   "tags": { "site": "cellar" }, "matched": true } ],
    "matched": 1, "total": 12, "rules": [], "excluded_rules": [] }
```

Un filtre est limité à douze conditions. Au-delà, un second canal est
plus clair qu'une condition de plus.

## Escalade : si personne n'acquitte

Un saut supplémentaire, et un seul. Quand la politique règle
`escalate_after_secs` et `escalate_channel`, une alerte que personne n'a
[acquittée](../using/alerts.md) dans ce délai est aussi envoyée — une
fois — à ce canal :

```
PUT /api/notify/policy
{ "escalate_after_secs": 900, "escalate_channel": 4 }
```

- Le délai est compté **depuis le moment où le premier message est
  réellement sorti**, pas depuis le déclenchement de l'alerte. Si les
  heures calmes ou une livraison échouée ont avalé la première
  notification, il n'y a encore rien à escalader et le saut attend.
- **Acquitter l'alerte l'arrête**, exactement comme ça arrête les
  rappels ; sa résolution aussi.
- Le message d'escalade va **seulement** au canal d'escalade — les
  canaux qui ont déjà entendu le déclenchement ne sont pas prévenus deux
  fois.
- Il traverse le propre plancher de gravité, filtre de routage et heures
  calmes de ce canal : vous avez nommé ce canal pour être réveillé, il
  n'est donc pas retenu au seul moment où ça compte.
- La ligne le dit : `🔴 Critical · Disk full — /data — 95 % … — still
  unacknowledged`.

`"escalate_channel": null` désactive l'escalade ; un délai de `0` fait
pareil. Les deux sont nécessaires pour que le saut existe, et le serveur
refuse un canal qui n'existe pas plutôt que d'escalader dans le vide.

DumbMonit s'arrête délibérément là : un saut, pas de rotation, pas de
calendrier de qui est d'astreinte cette semaine. Si vous avez besoin de
ça, pointez le canal d'escalade vers PagerDuty ou Opsgenie, qui le font
correctement.

## Regroupement et le plafond horaire

`GET/PUT /api/notify/policy` contient les réglages globaux :

```json
{ "batch_window_secs": 60, "max_per_hour": 20,
  "flap_events": 4, "flap_window_secs": 1800, "flap_hold_secs": 1800,
  "public_url": "https://monit.example.lan",
  "escalate_after_secs": 0, "escalate_channel": null }
```

- **Fenêtre de regroupement** — les alertes qui se déclenchent dans la
  fenêtre partent comme *un* message par canal, groupé par appareil :
  « 3 alertes sur 2 appareils » avec une section par appareil ; les
  résolutions suivent (« 2 résolues »). `0` envoie au prochain cycle. Un
  résumé liste 12 lignes au plus, puis « …et N autres alertes ».
- **Plafond horaire** — une fois qu'un canal a reçu `max_per_hour`
  messages dans la dernière heure, les alertes suivantes attendent et
  arrivent comme un résumé unique quand une place se libère. `0` retire
  le plafond.
- **Canaux d'astreinte** (PagerDuty, Opsgenie) ne sont jamais regroupés
  entre appareils : ils ouvrent et ferment un incident par appareil, ils
  reçoivent donc un message par appareil sans attendre.

Les échecs de livraison gardent les lignes en file ; le prochain cycle
retente.

## Ce qu'une ligne dit

Chaque alerte est une ligne, la même par mail, ntfy, chat et la variable
`{{message}}` d'un webhook personnalisé :

```
🔴 Critical · Disk full — /data — 95 % (threshold > 90 %), for 12 min
⚠️ Warning · VM or container stopped — win11-desktop (101), for 59 s
✅ Resolved · Service down — https://example.lan/
```

- Le glyphe est suivi du mot d'état (Critical / Warning / Info,
  Resolved, Flapping), la gravité est donc lisible sans couleur ni
  emoji.
- Une règle qui surveille plusieurs séries nomme celle qui s'est
  déclenchée : la VM et son id, le point de montage, l'interface, le
  groupe de sauvegarde, l'URL de la sonde. Les règles à l'échelle de
  l'appareil (inaccessible) ne nomment rien de plus que l'appareil.
- La valeur et le seuil sont montrés pour les règles mesurées ; une
  règle tout-ou-rien sur une métrique 0/1 (sur batterie, invité arrêté)
  ne répète pas « 1 (threshold > 0) ».

## Liens profonds

Chaque message se termine par un lien vers l'appareil
(`{public_url}/targets/{id}`) quand une URL publique est connue — le
`public_url` de la politique, ou `DUMBMONIT_PUBLIC_URL`. Les webhooks
personnalisés l'obtiennent aussi comme variable `{{link}}` sauf si leur
propre `base_url` est réglé.

Les messages portent aussi des liens **Acquitter** et **Pause 1 h** (des
boutons sur ntfy, Telegram et Web Push). Les services de chat récupèrent
chaque lien affiché pour construire un aperçu : un lien n'agit donc jamais
seul, il ouvre une page de confirmation, et seul le bouton de cette page
acquitte. Chaque lien est signé, ne sert qu'une fois et pendant 24 heures,
sans connexion. Voir
[Acquitter ou mettre en pause depuis la notification](../notifications.md#acknowledge-or-snooze-from-the-notification).

## Ce qui a été emprunté à Pulse, et ce qui ne l'a pas été

Le design suit [Pulse](https://github.com/rcourtman/Pulse) là où ça
convient à un moniteur homelab : seuils de déclenchement/résolution, une
couche de surcharge par ressource, un délai de répit de notification, une
fenêtre de regroupement, une limite de débit horaire, des heures calmes,
une détection de flapping avec un délai de répit, « notifier à la
résolution », et les outils d'astreinte gardés hors des résumés. Pas
adopté : la période d'observation de 24 heures de Pulse avant de
notifier (la règle de référence de DumbMonit apprend déjà silencieusement ;
les règles de seuil doivent parler dès le premier jour) et
repousser/acquitter depuis le message (voir ci-dessus). Le routage basé
sur les étiquettes par destination *a* été adopté, comme le filtre de
routage ci-dessus — choisir les canaux sur la règle s'est avéré être le
mauvais endroit pour « tout ce qui est dans la cave va dans cette
salle ».
