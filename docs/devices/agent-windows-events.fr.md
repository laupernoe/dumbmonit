# Alertes du journal d'événements Windows

L'agent Windows peut surveiller le journal d'événements de Windows et déclencher
une alerte quand un événement correspondant est journalisé : échecs de
connexion, redémarrage inattendu, service planté, erreurs de disque. Les règles
se règlent par machine, sur sa page d'équipement, dans le panneau **Journal
d'événements Windows**, et parviennent à l'agent avec l'accusé de réception de
son prochain rapport : rien à changer sur la machine, aucun redémarrage.

**Aucune règle n'existe par défaut** : tant que vous n'en ajoutez pas, l'agent
ne lit rien du journal et rien n'est déclenché. Le panneau propose des exemples
(voir [Règles proposées](#suggested-rules)) à ajouter en un clic, modifiables
avant l'enregistrement.

## Ce qu'une règle retient

| Champ | Sens |
|---|---|
| Journal | `System`, `Application`, `Security`, ou n'importe quel canal comme `Microsoft-Windows-TaskScheduler/Operational` ou `Microsoft-Windows-Windows Defender/Operational`. |
| Source | Le nom du fournisseur tel que l'Observateur d'événements l'affiche dans le détail de la colonne *Source* (`Microsoft-Windows-Kernel-Power`, `Service Control Manager`). Vide : toute source. |
| ID d'événement | Identifiants et plages séparés par des virgules : `4625, 4740` ou `7000-7099`, 20 au plus. Vide : tout identifiant. |
| Niveaux | Critique, Erreur, Avertissement, Information. Aucun coché : tous les niveaux. Information couvre aussi le niveau 0, celui de tous les événements d'audit du journal Sécurité (4625 compris). |
| Le message contient | Texte cherché dans le message mis en forme, sans distinction de casse. Cochez *expression régulière* pour utiliser une expression régulière (sensible à la casse). Vide : tout événement retenu par les autres champs. |
| Sévérité de l'alerte | Info, Avis ou Avertissement : l'alerte déclenchée sur la machine. |

Une règle exige au moins un identifiant, une source ou un niveau : une règle sur
un journal entier remonterait chaque événement. Les champs deviennent une
requête XPath (`*[System[Provider[@Name='…'] and (EventID=4625) and …]]`)
exécutée par l'agent via l'API Windows Event Log.

## Comment l'agent lit le journal

- À chaque cycle (30 s par défaut), pour chaque règle, l'agent demande au
  journal les événements plus récents que le dernier qu'il a lu.
- **Une règle nouvelle ne remonte que les événements journalisés après son
  arrivée sur l'agent** : l'historique déjà présent n'est jamais envoyé.
- Le dernier enregistrement lu de chaque règle est gardé à côté du secret de
  liaison de l'agent (`eventlog-state.json`) : après un redémarrage, l'agent
  reprend où il s'était arrêté, si bien qu'un `41 Kernel-Power` journalisé au
  démarrage qui suit un plantage est bien remonté. Un journal effacé est
  détecté et relu depuis son début.
- Les messages sont mis en forme dans la langue du système
  (`EvtFormatMessage`) ; quand un fournisseur n'a pas de message, les données
  de l'événement sont envoyées à la place. Chaque message est coupé à
  2 000 caractères.
- Lire le journal Sécurité demande les droits du compte sous lequel tourne
  l'agent : le service Windows tourne en LocalSystem, qui peut le lire. Si un
  journal ne se lit pas, l'agent le signale une fois dans son journal et
  continue avec les autres règles.

## Limite de débit

Au plus **10 événements par règle et par minute** sont envoyés. Le reste est
compté et affiché sur la règle comme *écartés par la limite de débit* : une
rafale de milliers de 4625 lors d'une attaque par mot de passe n'inonde ni le
serveur ni vos canaux de notification. Les événements qui attendent un serveur
injoignable sont plafonnés à 500 ; les plus anciens cèdent la place et sont
comptés comme écartés eux aussi.

## Alertes

Chaque événement correspondant déclenche la règle livrée de la sévérité de la
règle d'événements, sur la machine :

| Règle | Gravité |
|---|---|
| Windows event (critical) | Avertissement (`critical`) |
| Windows event (warning) | Avis (`warning`) |
| Windows event (info) | Info (`info`) |

- Une alerte par machine et par sévérité, pas une par événement : la
  notification cite le dernier événement (nom de la règle, identifiant, source
  et première ligne du message).
- Tant que des événements arrivent, l'alerte reparle au plus toutes les
  10 minutes, en citant le plus récent.
- L'alerte se résout 15 minutes après le dernier événement correspondant. Pas
  de rappel : un événement est un fait, pas une panne en cours.

La série derrière elle est `dumbmonit_agent_eventlog_matches_<sévérité>`, le
nombre d'événements reçus dans les 15 dernières minutes. La page de
l'équipement liste les derniers événements reçus (le serveur en garde 500 par
machine), avec leur message complet.

## Règles proposées {#suggested-rules}

Aucune n'est active tant que vous ne l'avez pas ajoutée.

| Exemple | Journal | Source | ID d'événement | Sévérité |
|---|---|---|---|---|
| Échecs de connexion | Security | Microsoft-Windows-Security-Auditing | 4625 | Avis |
| Compte verrouillé | Security | Microsoft-Windows-Security-Auditing | 4740 | Avis |
| Redémarrage inattendu | System | Microsoft-Windows-Kernel-Power | 41 | Avertissement |
| Service arrêté de façon inattendue | System | Service Control Manager | 7031, 7034 | Avis |
| Erreurs de disque | System | toute (niveaux Erreur et Avertissement) | 7, 11, 51, 153 | Avertissement |
| Journal de sécurité effacé | Security | toute | 1102 | Avertissement |
| Erreurs d'application | Application | toute (niveaux Critique et Erreur) | tout | Info |

Autres identifiants utiles :

- `6008` (System, EventLog) : l'arrêt précédent était inattendu.
- `1074` (System, User32) : qui a redémarré ou éteint la machine, et pourquoi.
- `4720`, `4726` (Security) : un compte utilisateur a été créé, supprimé.
- `4728`, `4732`, `4756` (Security) : un membre a été ajouté à un groupe de
  sécurité.
- `7045` (System, Service Control Manager) : un nouveau service a été installé.
- `1001` (System, BugCheck) : la machine a redémarré après un écran bleu.
- `1116` (`Microsoft-Windows-Windows Defender/Operational`) : logiciel
  malveillant détecté.

Les événements d'audit (4625, 4740, 4720…) ne sont journalisés que si la
stratégie d'audit correspondante est activée sur la machine ou par stratégie de
groupe.

## Prérequis

Un agent de cette version ou plus récent, sous Windows. Un agent antérieur
ignore les règles ; celles posées sur une machine qui n'est pas sous Windows
sont gardées mais jamais envoyées.
