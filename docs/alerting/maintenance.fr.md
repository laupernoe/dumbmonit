# Fenêtres de maintenance

Une fenêtre de maintenance (un *silence* dans l'API) arrête les notifications
pour une surveillance, ou pour tout, pendant une période. L'évaluation continue en
dessous : quand la fenêtre se termine, une alerte encore active est notifiée
une fois, plutôt que d'apparaître de nulle part après deux heures.

## Où

- **Alertes → Maintenance** : la liste des fenêtres, avec *Active
  maintenant* ou *Planifiée*, et le formulaire pour en ajouter une.
- **Page de la surveillance → Silence 1 h / Silence jusqu'à…** : une fenêtre
  ponctuelle rapide pour toute la surveillance.
- ++ctrl+k++ → *Planifier une maintenance*.

**Repousser**, sur une alerte dans l'onglet *Maintenant* ou une page
de surveillance, est une version plus petite du même mécanisme : une fenêtre
ponctuelle limitée aux labels exacts de cette seule alerte plutôt qu'à tout
la surveillance, pour qu'une règle sœur continue de parler. Elle apparaît ici
aussi, nommée `Snooze · …`. Voir
[Se débarrasser d'une alerte](../using/alerts.md#getting-rid-of-an-alert).

## Fenêtre ponctuelle

| Champ | Signification |
|---|---|
| Nom | Affiché dans la liste et dans l'historique. |
| Surveillance | Laissez vide pour couvrir chaque surveillance. |
| Début, Fin | Heures absolues. La fenêtre inclut le début et exclut la fin. |
| Commentaire | Optionnel. Pourquoi la fenêtre existe. |

Les fenêtres ponctuelles sont purgées une fois terminées.

## Fenêtre hebdomadaire

| Champ | Signification |
|---|---|
| Jours | Quels jours de la semaine (lundi à dimanche). |
| De, À | Heure locale dans le fuseau horaire de la fenêtre. Si *À* est à ou avant *De*, la fenêtre déborde sur minuit (« 23:00 → 01:00 » appartient au jour où elle commence). |
| Fuseau horaire | Un nom IANA comme `Europe/Paris`. Prérempli avec le fuseau de votre navigateur. |

## Fenêtre mensuelle

| Champ | Signification |
|---|---|
| Un jour de la semaine | *Premier*, *Deuxième*, …, *Cinquième* ou *Dernier*, plus le jour — « le premier dimanche », « le dernier vendredi ». |
| Une date | Un ou plusieurs jours du mois, de 1 à 31, séparés par des virgules (« 1, 15 »). |
| De | Heure de début locale, dans le fuseau horaire de la fenêtre. |
| Pendant | Combien de temps dure la fenêtre, en temps réel écoulé. Une fenêtre mensuelle peut durer plus d'un jour. |
| Fuseau horaire | Comme ci-dessus. |

Un mois qui ne contient pas l'occurrence est ignoré plutôt qu'approximé : il
n'y a pas de 31 en février, et pas de cinquième dimanche en avril 2026, donc
la fenêtre ne s'ouvre simplement pas ce mois-là. La liste montre la prochaine
occurrence calculée par le serveur, pour que vous puissiez la vérifier d'un
coup d'œil.

DumbMonit n'a pas d'expressions cron, volontairement : une expression cron
nomme des *instants*, et une fenêtre de maintenance a besoin d'une *durée*.
Les jours du mois et « le n-ième jour de la semaine » couvrent les calendriers
que les gens tiennent réellement, et se relisent en une ligne.

## Fuseaux horaires et heure d'été

Une fenêtre récurrente s'ouvre à son heure locale sur l'horloge puis tourne
pour sa durée en temps réel. Les deux moitiés comptent :

- **L'heure locale ne dérive pas.** « Chaque dimanche à 02:00 en
  `Europe/Paris` » est 01:00 UTC en hiver et 00:00 UTC en été — la fenêtre
  suit le changement d'heure plutôt que de glisser d'une heure deux fois par
  an.
- **La durée ne s'étire pas.** La nuit où l'horloge revient en arrière, 02:00
  à 04:00 sur l'horloge durerait trois heures réelles ; la fenêtre dure quand
  même deux heures. La nuit où l'horloge avance et que 02:00 n'existe pas, la
  fenêtre s'ouvre dès que l'horloge reprend.

Une fenêtre enregistrée avant que les fuseaux horaires existent garde son
décalage UTC fixe et se comporte exactement comme avant. Enregistrez-la de
nouveau avec un fuseau pour qu'elle suive les changements d'heure. Un nom de
fuseau que le serveur ne connaît pas est refusé à l'enregistrement, avec un
exemple.

## Filtres (matchers)

Une fenêtre peut aussi être limitée par des correspondances exactes de labels
(`matchers` dans l'API) : tous doivent correspondre aux labels de l'alerte,
par exemple `tag_role = lab` et `__name__ = dumbmonit_up`. Une fenêtre sans
surveillance et sans filtre réduit au silence toute l'instance. Des fenêtres qui
se chevauchent ne posent pas de problème ; la première qui couvre le moment
explique le silence.

## À quoi ressemble une alerte réduite au silence

L'alerte garde sa phase et montre `silenced: true` ; l'historique enregistre
la transition avec `notified: false` et la raison. Désactiver une fenêtre
(`enabled: false`) la garde dans la liste sans effet.

## Sur une page d'état publique

Une surveillance couverte par une fenêtre de maintenance est montrée comme
**Maintenance** sur toute [page d'état](../using/status-pages.md) qui le
liste, au lieu de rouge. Si rien d'autre sur la page n'est en panne ou
dégradé, l'état global de la page lit aussi *Maintenance*. Seules les
fenêtres qui nomment une surveillance — ou dont les filtres sont des labels au
niveau de la surveillance — se montrent ainsi ; une fenêtre limitée à une seule
métrique reste une affaire d'alerte.
