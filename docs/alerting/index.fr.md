# Comment fonctionnent les alertes

Des règles utiles sont actives dès l'installation ; rien n'a besoin d'être
réglé. Ce qui fait la différence à l'usage quotidien, c'est ce qui garde le
bruit bas : suppression par dépendance, groupement par hôte, fenêtres de
maintenance, et une référence qui apprend avant de parler.

![La page Alertes : ce qui se passe maintenant, la maintenance, l'historique et les règles](../assets/screenshots/alerts-light.png){ loading=lazy }

## Gravités {#severities}

L'interface parle l'échelle du bulletin météo. L'API et l'éditeur de règles
stockent les trois mêmes niveaux sous leurs noms moteur.

| Dans l'UI | Dans l'API (`severity`) | Signification |
|---|---|---|
| Info | `info` | Bon à savoir, aucune action attendue. Utilisé par la règle de référence. |
| Avis | `warning` | Demande de l'attention bientôt : CPU élevé, disque presque plein, service lent. |
| Avertissement | `critical` | Demande de vous maintenant : appareil inaccessible, service en panne, vérification de sauvegarde échouée, certificat expiré. |

L'escalade monte la gravité d'une règle d'un cran après `escalate_after` si
l'alerte continue de se déclencher (Info → Avis → Avertissement ;
Avertissement reste).

## Phases

Chaque alerte est une machine à états, évaluée toutes les 30 secondes
(`DUMBMONIT_ALERT_INTERVAL_SECS`).

| Phase | Dans l'UI | Signification |
|---|---|---|
| `ok` | — | La condition est fausse. |
| `pending` | En construction | La condition est vraie mais le délai de maintien de la règle (`for`) ne s'est pas encore écoulé. Rien n'est envoyé. |
| `firing` | Active | La condition a tenu assez longtemps. La première notification sort, puis des rappels toutes les `repeat_interval` (6 heures par défaut pour la plupart des règles intégrées), puis une escalade si configurée. |
| `resolved` | Résolue | Retour à la normale. Une notification de résolution n'est envoyée que si l'activation avait été annoncée. |
| `suppressed` | Supprimée par le parent | L'alerte est active, mais un appareil parent, ou l'appareil lui-même, est inaccessible : voir ci-dessous. |

`phase` est l'état du moteur ; `effective_phase` est ce que vous lisez. Une
alerte dont `effective_phase` est `suppressed` garde `phase = firing` en
dessous, elle n'est donc pas notifiée à nouveau quand le parent se rétablit.

Une alerte peut aussi être **acquittée** (`acked`, avec `acked_by`,
`acked_until` et `ack_note`) : la phase ne bouge pas, rappels et escalades
se mettent en pause jusqu'à `acked_until`, la résolution est quand même
notifiée et efface l'acquittement. **Repousser** une alerte fonctionne de
la même façon en dessous mais via une fenêtre de maintenance limitée aux
labels exacts de cette alerte (`silenced: true`), plutôt que tout
l'appareil : une règle sœur sur le même appareil continue de parler. Voir
[Se débarrasser d'une alerte](../using/alerts.md#getting-rid-of-an-alert).

## Suppression par dépendance {#dependency-suppression}

N'importe quel appareil peut déclarer un **appareil parent**. Quand un
appareil est inaccessible (la règle « Appareil inaccessible » se déclenche
sur lui), chaque alerte sur ses descendants est marquée *Supprimée par le
parent* plutôt qu'envoyée : un commutateur qui tombe produit une
notification, pas trente. L'ancêtre inaccessible le plus proche est nommé
comme la cause (`suppressed_by`), et la chaîne est suivie jusqu'à 64
niveaux.

Il en va de même pour l'appareil inaccessible lui-même : ses autres alertes
(disque plein, CPU, etc.) sont supprimées par sa propre panne, avec
`suppressed_by` qui le désigne lui-même, car leurs valeurs ne sont plus à jour
et elles ne sont qu'un symptôme de la même panne. Seule son alerte « Appareil
inaccessible » n'est jamais masquée par sa propre panne : vous êtes donc
prévenu une fois.

Réglez le parent dans le formulaire de l'appareil. Sur la page Appareils,
les enfants s'empilent sous leur parent et s'atténuent quand il est
inaccessible.

Un [agent relais](../install/remote-site.md) compte comme un parent de
chaque appareil qu'il sonde (*Atteint via* dans le formulaire d'appareil),
en plus du parent réglé explicitement : quand le relais est inaccessible,
ses appareils sont supprimés avec le relais nommé comme la cause. À
distance égale, le parent explicite gagne.

Un parent compte aussi comme inaccessible dès que sa propre sonde le dit
(le statut *Délai dépassé* / *Appareil inaccessible* sur sa page), sans
attendre que sa propre alerte « Appareil inaccessible » se déclenche : un
agent qui arrête d'envoyer emporte ses appareils avec lui une minute ou
deux avant sa propre alerte, et ils notifieraient chacun en premier sinon.

## Groupement, déduplication, rappels

Sans groupement, un NAS dont le RAID se dégrade envoyait un message par
disque, par cycle. DumbMonit envoie au plus un message par hôte et par
cycle, même s'il contient cinq lignes. Chaque alerte est notifiée une fois
quand elle commence à se déclencher (`firing`), puis seulement comme
rappel (`reminder`, toutes les `repeat_interval`), une escalade
(`escalation`) ou une résolution (`resolved`). L'historique d'alerte
enregistre chaque transition avec si elle a été notifiée et, sinon,
pourquoi (apprentissage, supprimée, réduite au silence, acquittée).

## Fenêtres de maintenance

Un silence arrête les notifications sans arrêter l'évaluation : à la fin
d'une fenêtre, une alerte encore active est notifiée une fois, plutôt que
« née » après deux heures d'existence. Les fenêtres sont ponctuelles ou
hebdomadaires, par appareil ou pour toute l'instance. Voir
[Fenêtres de maintenance](maintenance.md).

## Apprentissage de référence {#baseline-learning}

La règle « CPU inhabituel » n'a besoin d'aucun seuil. Pour chaque série
elle garde 168 compartiments, un par (jour de la semaine, heure), et
compare chaque point à ce qui est habituellement observé *à ce moment-là*.
Le score est robuste (médiane et MAD, une EWMA par compartiment avec
`alpha = 0.05`), si bien qu'un seul incident ne réécrit pas la référence,
et les dérives saisonnières sont suivies.

La règle reste **silencieuse pendant ses quatorze premiers jours**
d'apprentissage : chaque compartiment doit avoir été vu au moins deux fois
pour distinguer « inhabituel » de « jamais observé ». En attendant, elle
est montrée avec un indicateur *apprentissage* et dit seulement ce qu'elle
aurait déclenché. Les références de séries disparues sont oubliées après
60 jours.

## Prévisions

Deux types d'alerte prédisent plutôt que rapportent, et la vue d'ensemble
les liste sous *Prévisions* : l'anomalie de référence, et les règles
`predict` qui extrapolent une série avec `predict_linear()` côté
VictoriaMetrics (« Système de fichiers presque plein » dans quatre jours).
L'estimation de datastore PBS est un simple seuil sur la propre prévision
de PBS.

## Où vont les notifications

Les règles intégrées n'ont aucun canal attaché, ce qui signifie **chaque
canal activé**. Une règle que vous modifiez peut nommer des canaux
spécifiques. Les canaux se configurent dans Réglages → Notifications et rapports ; voir
[Canaux de notification](../notifications.md).

## Notifications intelligentes

Tout ce qui précède décide *si* une alerte peut parler. Une seconde étape,
la politique de notification, décide *quand* et *sur quel canal*, pour que
personne ne soit spammé. Le chemin complet d'une alerte, dans l'ordre :

1. **Évaluer** — seuil, score d'anomalie ou prévision, avec le délai de
   maintien de la règle (`for`) et, s'il est réglé, son **seuil de
   résolution** (hystérésis : se déclenche au-dessus de 90 %, ne se résout
   qu'en dessous de 85 %) et toute **surcharge par appareil**.
2. **Dédupliquer** — une empreinte par (règle, série), indexée sur l'id de
   l'appareil (`target`) plutôt que son nom : renommer un appareil ou
   modifier ses étiquettes ne crée pas une seconde alerte, et une série qui
   se résout à la même clé deux fois donne une seule alerte. Les séries
   d'un appareil supprimé ou mis en pause sont abandonnées : leurs alertes
   se résolvent sur le champ, sans aucune notification, et l'historique
   enregistre `appareil supprimé ou désactivé`.
3. **Supprimer par dépendance** — les descendants d'un appareil
   inaccessible restent silencieux.
4. **Réduire au silence** — les fenêtres de maintenance réduisent au
   silence ce qu'elles couvrent.
5. **Grouper par appareil** — un message par appareil par cycle, rappels
   et escalade repliés dedans.
6. **Maintien contre le flapping** — une empreinte qui se déclenche et se
   résout 4 fois en 30 minutes envoie un seul avis « instable », puis plus
   rien pendant 30 minutes.
7. **Filtres par canal** — gravité minimale, « me dire quand ça se
   résout » activé/désactivé, et un intervalle minimal entre deux messages
   sur la même alerte.
8. **Heures calmes** (par canal) — seules les alertes de niveau
   Avertissement passent ; le reste attend un résumé à la fin des heures
   calmes (une alerte qui se résout pendant ce temps n'est mentionnée que
   comme « résolue pendant les heures calmes »).
9. **Regroupement** — les alertes dans la fenêtre de regroupement (60 s
   par défaut) partent comme un message par canal : « 3 alertes sur 2
   appareils, 1 résolue ».
10. **Plafond horaire** — au-delà de `max_per_hour` messages sur un canal,
    les alertes attendent et arrivent comme un résumé unique
    (« …et 7 autres alertes »).
11. **Envoyer** — avec un lien « Ouvrir dans DumbMonit » par appareil
    quand une URL publique est connue.

Les étapes 6 à 10 se configurent dans Réglages → Notifications et rapports → Politique
de notification (globale) et sur chaque canal (Options de livraison). Voir
[Politique de notification](notifications.md).
