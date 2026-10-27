# Astreintes

L'astreinte répond à une question : quand une alerte critique tombe à 3 h du
matin, qui est prévenu, et qui l'est ensuite si cette personne ne réagit pas ?
La fonction est intégrée, gratuite, et ne demande aucun service
supplémentaire. Vous décrivez **qui** (les contacts), **quand** (les
plannings) et **que faire si personne ne répond** (les politiques
d'escalade), puis vous branchez le tout sur le routage des notifications.

```
alerte → routage (étiquettes, sévérité, canaux de la règle)
       → canal « DumbMonit on-call » → politique d'escalade
             étape 1 : la personne d'astreinte du planning X → attendre 10 min
             étape 2 : Alice + le canal #ops                 → attendre 15 min
             … répété K fois, jusqu'à ce que quelqu'un acquitte
```

## 1. Vos contacts et vos heures calmes

Chaque utilisateur déclare comment le joindre dans **Réglages → Mon compte →
Contacts d'astreinte**. Un contact est de l'un de ces deux types :

- **Via un canal d'équipe.** Réutilisez un canal qu'un admin a déjà configuré
  (e-mail SMTP, SMS Twilio, Signal, Telegram) et ne donnez que votre propre
  adresse : votre e-mail, votre numéro, votre identifiant de discussion
  Telegram. Le serveur et ses identifiants restent ceux de l'équipe.
- **Votre propre service.** ntfy, Pushover, Telegram, Gotify, Bark, Pushbullet,
  Discord, Slack, Matrix… avec vos propres réglages et secrets, comme pour un
  [canal de notification](../notifications.md). Les secrets ne sont jamais
  réaffichés.

Chaque contact a une **sévérité minimale** (par exemple le SMS seulement pour le
critique, ntfy pour tout) et un bouton **Tester**. Un contact en échec affiche sa
dernière erreur.

Les **heures calmes** (même écran, avec votre fuseau horaire) évitent d'être
réveillé pour des broutilles : hors astreinte, seules les alertes critiques
passent pendant ces heures. Quand vous êtes d'astreinte, vous êtes toujours
prévenu, heures calmes ou non.

**Lien de calendrier.** *S'abonner à mes astreintes* génère une URL iCal privée
pour votre application de calendrier (de 30 jours dans le passé à 90 jours à
venir). La régénérer révoque l'ancien lien, et *Supprimer* le coupe.

## 2. Les plannings

**Alertes → Astreintes → Plannings → Nouveau planning.**

- **Fuseau horaire** : un nom IANA comme `Europe/Paris`. Les relèves ont lieu à
  la même heure murale locale toute l'année, malgré les changements d'heure
  (une rotation en heures fait exception : elle compte en temps réel).
- **Couches** : un planning est une pile de couches. Là où plusieurs couches
  sont actives, **celle qui est plus bas dans la liste l'emporte**. Mettez la
  couverture générale en premier, les exceptions en dessous.
- **Rotation** par couche : *quotidienne*, *hebdomadaire* ou *personnalisée*
  (une durée en heures, jours ou semaines). Le **début** est la première
  relève : son heure est l'heure de relève et, pour une rotation hebdomadaire,
  son jour est le jour de relève.
- **Participants** : une liste ordonnée d'utilisateurs, parcourue par la
  rotation.
- **Restrictions** : des fenêtres horaires par jour de la semaine pendant
  lesquelles la couche s'applique. Une fenêtre dont la fin est égale ou
  antérieure au début passe minuit (`18:00` à `09:00`). Sans restriction, la
  couche est toujours active.
- **Prévenir aux relèves** : prévient éventuellement chacun quand son
  astreinte commence et se termine.

La page affiche un calendrier en semaine ou en mois et un panneau **Qui est
d'astreinte maintenant**.

### Exemple : une équipe de trois

Alice, Bob et Chloé, fuseau `Europe/Paris`.

| Couche | Rotation | Restrictions | Participants |
|---|---|---|---|
| Jours ouvrés | hebdomadaire, début le lundi à 09:00 | lundi au vendredi, 09:00 à 18:00 | Alice, Bob, Chloé |
| Nuits et week-ends | hebdomadaire, début le lundi à 18:00 | lundi au vendredi 18:00 à 09:00, samedi et dimanche toute la journée | Bob, Chloé, Alice |

La seconde couche est plus bas, elle l'emporte donc dès qu'elle est active. Les
heures ouvrées vont à la première couche, soirs et week-ends à la seconde, et
personne ne porte les deux la même semaine.

### Remplacements

Sur un planning, **Ajouter un remplacement** se lit « X remplace Y du … au … ».
Si Y est laissé vide, X remplace quiconque serait d'astreinte sur la période.
Les admins et les opérateurs peuvent créer et supprimer des remplacements :
un échange n'oblige donc pas un admin à modifier la rotation. Un remplacement
apparaît dans le calendrier et dans les liens iCal.

## 3. Les politiques d'escalade

**Alertes → Astreintes → Politiques d'escalade.**

Une politique est une liste ordonnée d'**étapes**. Chaque étape a une ou
plusieurs cibles :

- **la personne d'astreinte d'un planning** : celle qui l'est à cet instant ;
- **une personne** : prévenue sur tous ses contacts adaptés à la sévérité ;
- **un canal** : n'importe quel canal de notification (un salon de discussion,
  par exemple).

Chaque étape a un **délai** en minutes avant la suivante. **Répéter** rejoue
toute la chaîne K fois de plus si elle se termine sans réponse.

### Exemple : deux étapes, répétées deux fois

| Étape | Cibles | Puis attendre |
|---|---|---|
| 1 | Astreinte du planning *Équipe* | 10 min |
| 2 | Alice, et le canal *#ops* | 15 min |

Répétition : 2. L'alerte prévient la personne d'astreinte, 10 minutes plus tard
Alice et le canal, 15 minutes plus tard de nouveau la personne d'astreinte, et
ainsi de suite : trois passages en tout, puis l'escalade est marquée épuisée.

### Ce qui arrête une escalade

- **L'acquittement**, par n'importe qui : dans l'interface, avec le lien signé
  de la notification, ou en mettant l'alerte en sourdine, en silence ou en
  l'écartant. L'escalade s'arrête et personne d'autre n'est prévenu.
- **La résolution** de l'alerte. L'escalade s'arrête et les personnes déjà
  appelées sont prévenues du retour à la normale.

Chaque alerte garde un **journal d'escalade** : qui a été prévenu, quand, par
quel contact, et qui a acquitté. Il se trouve dans le détail de l'alerte.

## 4. Le branchement

Créez un canal de notification de type **DumbMonit on-call** (Réglages →
Notifications → Canaux) et choisissez sa politique d'escalade. Il
se comporte alors comme tout canal : filtres de routage par étiquette,
sévérité minimale et canaux choisis par règle s'appliquent. Un montage typique :
une règle pour les alertes critiques de production qui écrit sur ce canal,
pendant que le reste va toujours au chat.

L'escalade globale existante (« si personne n'acquitte, prévenir un autre
canal ») n'est pas modifiée et n'écrit toujours que sur un seul canal.

Le bouton **Tester** du canal appelle l'étape 1 avec un message de test.

!!! note "Les liens d'acquittement demandent l'URL publique"
    Les liens d'acquittement et de mise en pause des notifications ne
    fonctionnent que si une **URL publique** est définie (Réglages →
    Notifications → Politique de notification, ou
    `DUMBMONIT_PUBLIC_URL`). Sans elle, on acquitte depuis l'interface, et les
    liens iCal sont des chemins relatifs.

## Rôles

Tout utilisateur connecté peut lire les plannings, les politiques et voir qui
est d'astreinte. Seuls les admins créent ou modifient plannings et politiques ;
les admins et les opérateurs gèrent les remplacements. Chacun gère ses propres
contacts, heures calmes et lien de calendrier.

## Sauvegarde et restauration

Les [sauvegardes](../install/backup.md) incluent plannings, remplacements,
politiques, contacts (avec leurs secrets, l'archive étant chiffrée) et
préférences. Les liens iCal personnels ne sont **pas** restaurés : générez-les
de nouveau.
