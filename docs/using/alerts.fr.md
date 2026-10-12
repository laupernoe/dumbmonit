# Page Alertes

![La page Alertes : ce qui a besoin de vous, les règles, la maintenance planifiée et l'historique](../assets/screenshots/alerts-light.png){ loading=lazy }

Tout sur les alertes sur une page, derrière cinq onglets. L'onglet est dans
l'URL (`/alerts#rules`), une vue est donc partageable par lien. **Planifier
une maintenance**, en haut à droite, ouvre le formulaire pour une fenêtre
depuis n'importe quel onglet.

| Onglet | Ce qu'il contient |
|---|---|
| **Maintenant** | La liste en direct *À traiter*, groupée par appareil : plaque de gravité, raison, depuis quand, et *Acquitter*, *Repousser*, *Ignorer* et *Ouvrir l'appareil* sur chacune. Les alertes acquittées et repoussées se trouvent chacune dans leur propre groupe calme en bas. Le badge sur l'onglet, et sur Alertes dans la barre du haut, est le compte de ce qui a encore besoin de vous. |
| **Planifiées** | Fenêtres de maintenance, *Active maintenant* ou *Planifiée*, ponctuelle, hebdomadaire ou mensuelle, chacune avec la prochaine occurrence. Voir [Fenêtres de maintenance](../alerting/maintenance.md). |
| **Règles** | Chaque règle avec sa gravité, une marque *Intégrée* et, si un appareil l'ignore, *Ignorée sur N appareils* ; activer, modifier en place, supprimer les vôtres, créer une règle de seuil. Voir [Règles](../alerting/rules.md). |
| **Notifications** | Où les alertes vous atteignent : les canaux et la politique de notification. Détails ci-dessous. |
| **Historique** | Les 200 dernières transitions, chacune nommant l'appareil, la règle et ce qui s'est passé ; les résolues peuvent être effacées. |

Le même modèle de vérité alimente le bulletin de la vue d'ensemble et cette
page, les deux s'accordent donc toujours sur ce qui a besoin de vous.

## Se débarrasser d'une alerte {#getting-rid-of-an-alert}

Une alerte active n'est pas binaire — « la corriger » ou « vivre avec le
bruit pour toujours ». Quatre actions, pour quatre situations différentes,
ouvertes aux administrateurs et opérateurs (un lecteur voit l'état mais ne
peut pas le changer) :

- **Acquitter** (« je m'en occupe ») est pour une alerte sur laquelle vous
  travaillez activement : « je sais, arrête de me le rappeler pendant 4 h ».
  Le menu propose 1 h, 4 h, 24 h ou *jusqu'à résolution*, plus une note
  optionnelle pour qui lit la carte après vous. L'alerte continue d'être
  active et évaluée ; seuls ses rappels et escalades sont en pause — y
  compris l'escalade vers un deuxième canal. Vous êtes toujours informé
  quand elle se résout, et l'acquittement se termine à ce moment-là — une
  alerte qui revient plus tard notifie à nouveau. La carte quitte la liste
  principale pour un groupe plus calme *Acquittées*, lit *Acquittée par
  quelqu'un jusqu'à une heure*, et propose **Annuler l'acquittement**.
- **Repousser** est pour « pas maintenant, je reviendrai voir » sans
  prétendre faire quoi que ce soit : 1 h, 8 h, 1 jour ou *jusqu'à
  résolution*. À la différence d'Acquitter, cela fonctionne en ouvrant une
  fenêtre de maintenance limitée aux labels exacts de cette seule alerte —
  une règle sœur sur le même appareil continue de parler. La carte passe à
  un groupe calme *Repoussées* montrant le temps restant, et propose
  **Ne plus repousser**. Les rappels se mettent en pause de la même façon
  qu'avec un acquittement.
- **Ignorer** (« ne m'alerte plus jamais pour ça ») est pour une règle qui
  n'aura jamais de sens sur cet appareil : cela désactive cette règle pour
  cet appareil seul (une surcharge par appareil — voir
  [Surcharges par appareil](../alerting/rules.md#per-device-overrides)) et
  l'alerte disparaît pour de bon, immédiatement, pas juste rendue plus
  silencieuse. C'est réversible : **Arrêter d'ignorer** sur la page de
  l'appareil, ou depuis la règle elle-même dans Alertes → Règles, où elle
  lit *Ignorée sur N appareils*.
- **Effacer** est pour l'historique, pas pour une alerte active : une fois
  qu'une transition s'est résolue, *Effacer* (ou *Effacer tout ce qui est
  résolu*) la retire de la vue d'Historique par défaut. La ligne reste dans
  la base de données — rien n'est supprimé — et *Montrer les effacées* la
  fait revenir.

Les pages d'état publiques ignorent les acquittements et les mises en
pause : une alerte acquittée ou repoussée est toujours une alerte pour
l'extérieur. Une règle ignorée, en revanche, n'a jamais produit d'alerte du
tout.

## Notifications {#notifications}

Les canaux et la politique vivent ici — sous Alertes, pas Réglages — parce
qu'ils décident qui entend une alerte, ce qui est une affaire d'alerte. Deux
panneaux :

**Canaux.** La liste des canaux, chacun avec son type, *Activé* ou
*Désactivé*, la dernière fois qu'il a envoyé quelque chose, et sa dernière
erreur s'il y en a. **Ajouter un canal** ouvre un formulaire construit depuis
la description du type choisi par le serveur : réglages (visibles) et secrets
(écriture seule). **Envoyer un test** envoie un message de test et montre le
résultat en place. Les **Options de livraison** de chaque canal contiennent
sa gravité minimale, s'il entend les résolutions, un intervalle minimal par
alerte, et ses **heures calmes** (hebdomadaires, dans votre fuseau horaire :
seules les alertes de niveau Avertissement passent, le reste attend un
résumé).

**Seulement certaines alertes.** Les mêmes Options de livraison contiennent
le **filtre de routage** du canal : quelles alertes il veut, par étiquette
d'appareil (`site=cellar`), par type d'appareil (`proxmox`), ou par règle.
Les conditions sur le même champ se lisent comme *ou*, des champs différents
comme *et*, et une ligne *Sauf* gagne toujours. Sous l'éditeur, un aperçu
liste lesquels de vos appareils le filtre sélectionne maintenant, et une
phrase vous le redit en entier — *ce canal reçoit les avis et plus depuis
les appareils étiquetés site=cellar, sauf les appareils étiquetés
role=lab*. Un canal sans filtre continue de tout recevoir, ce que fait
chaque canal existant.

Modifier un canal et laisser le secret vide conserve celui enregistré. Les
types de canal et leurs champs sont documentés dans
[Canaux de notification](../notifications.md).

Les règles d'alerte intégrées notifient chaque canal activé ; une règle peut
aussi être limitée à des canaux spécifiques dans son éditeur.

**Politique de notification.** Les réglages globaux qui gardent les
notifications peu nombreuses : la fenêtre de regroupement, le plafond par
canal par heure, la détection de flapping sous *Plus d'options*, l'URL
publique utilisée pour les liens « Ouvrir dans DumbMonit », et la ligne
d'**escalade** — *si personne n'acquitte* dans un délai, *dire aussi* à un
autre canal, une fois. Le délai compte depuis le moment où le premier message
est réellement sorti, et acquitter l'alerte (ou sa résolution) arrête le
saut. Voir [Politique de notification](../alerting/notifications.md).

Liens : `/alerts#notifications` ouvre l'onglet, `/alerts#notifications-policy`
défile jusqu'au panneau de politique. Les anciens liens Réglages
(`/settings#notifications`, `/settings#notifications-policy`) redirigent ici.

## Opérateurs et lecteurs

Un **opérateur** traite les alertes sans toucher à la configuration : il
acquitte, repousse, ignore une règle pour un appareil, planifie ou supprime
des fenêtres de maintenance et efface l'historique résolu. Règles, canaux et
politique de notification restent en lecture seule pour lui, marqués
*Opérateur — la configuration est réservée aux administrateurs*. Ignorer est
la seule surcharge par appareil qu'un opérateur peut faire ; les seuils
restent une décision d'administrateur, et un *Ignorer* ou *Arrêter
d'ignorer* d'un opérateur conserve tout seuil qu'un administrateur a réglé
pour cet appareil.

Un lecteur voit chaque onglet mais aucun contrôle : la page affiche *Lecteur
— lecture seule* là où un administrateur trouverait les boutons.
