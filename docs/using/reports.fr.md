# Rapports par email

Un rapport est un court email qui raconte à votre équipe comment
l'infrastructure s'est comportée sur le dernier jour, la dernière semaine ou
le dernier mois, sans que personne n'ouvre DumbMonit. Il est hebdomadaire par
défaut ; quotidien et mensuel sont à un clic. Les rapports réutilisent un
**canal de notification email (SMTP)**, configurez-en donc un d'abord (voir
[Canaux de notification](../notifications.md)).

Ouvrez **Réglages > Rapports** pour les configurer. Tout ici est réservé aux
administrateurs, lecture incluse, car la page liste les adresses des
destinataires.

## Ce que contient un rapport

| Section | Contenu |
| --- | --- |
| **En-tête** | Disponibilité globale sur la période, nombre d'incidents, et évolution par rapport à la période précédente. |
| **Disponibilité par groupe** | Une ligne par groupe, puis par surveillance (les 40 moins disponibles au plus), calculée de la même façon que sur les [pages d'état](status-pages.md) : les sondes par leur taux de réussite, les hôtes et surveillances par leur fréquence de remontée. Les surveillances sans historique sont montrés comme « aucune donnée » plutôt que 100 %. |
| **Incidents** | Quand chacun a commencé, combien de temps il a duré (ou qu'il est toujours ouvert) et sur quelle surveillance. 25 au plus. Les alertes informatives sont laissées de côté. |
| **Surveillances les moins stables** | Les cinq surveillances avec la disponibilité la plus faible ou le plus d'incidents. |
| **Comparaison** | Disponibilité et nombre d'incidents comparés à la période de même durée juste avant. |

L'email est composé de simples tableaux HTML avec des styles en ligne et aucune
image distante, si bien qu'il s'affiche pareil partout et que rien n'est
récupéré à son ouverture. Une version texte est envoyée en complément. Chaque
nom de surveillance et chaque message est échappé.

## Réglages

| Réglage | Ce qu'il fait |
| --- | --- |
| **Nom** | Affiché seulement dans la liste. |
| **Activé** | Désactivé conserve les réglages sans rien envoyer. |
| **Fréquence** | Quotidien, hebdomadaire (défaut) ou mensuel. |
| **Jour** | Le jour de la semaine pour les rapports hebdomadaires, le jour du mois (1 à 28) pour les mensuels. |
| **Heure** et **Fuseau horaire** | Quand le rapport part, dans un fuseau horaire IANA comme `Europe/Paris`. L'heure d'été est suivie. |
| **Destinataires** | Jusqu'à 20 adresses, libres : elles n'ont pas besoin d'être des utilisateurs DumbMonit. Chaque destinataire reçoit son propre email, personne ne voit les autres. |
| **Canal email** | Quel canal SMTP l'envoie. Par défaut, le premier activé. Seul le serveur de messagerie du canal est utilisé : ses propres destinataires sont ignorés. |

La période est une fenêtre glissante qui se termine à l'envoi du rapport : les
24 dernières heures, 7 jours ou 30 jours. La période précédente est la fenêtre
de même durée juste avant.

Cliquez sur le **i** à côté du titre de section pour un rappel du contenu.

## Envoyer un aperçu

**Envoyer un aperçu** adresse le rapport actuel aux destinataires enregistrés
tout de suite, avec `[Preview]` dans le sujet. Cela ne change rien à la
planification : le prochain rapport part toujours à l'heure. Un aperçu peut
être envoyé une fois toutes les 30 secondes par rapport. Enregistrez vos
modifications d'abord, car l'aperçu utilise les destinataires enregistrés.

## Garanties de livraison

- **Au plus une fois.** Le créneau est réservé dans la base de données avant
  que l'email soit construit, si bien qu'un redémarrage n'envoie jamais le
  même rapport deux fois. Quand l'envoi échoue, l'erreur est conservée sur le
  rapport et affichée dans la liste, et le créneau n'est pas retenté ; le
  prochain part normalement.
- **Pas de rattrapage.** Si DumbMonit était indisponible au moment prévu, le
  rapport n'est envoyé au retour que si c'est dans les 3 heures du créneau.
  Au-delà, il attend le prochain créneau. Un rapport que vous créez ou
  réactivez n'envoie jamais un créneau déjà passé.
- Il peut exister au plus 10 rapports.
- En mode démo, rien n'est envoyé.

Les rapports sont inclus dans les [sauvegardes](../install/backup.md) : le
canal email est retrouvé par son nom à la restauration, et un rapport restauré
repart à zéro (sa date de dernier envoi n'est pas conservée).

## API

Voir [API HTTP](../reference/api.md#email-reports).
