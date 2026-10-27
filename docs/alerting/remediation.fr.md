# Procédures et remédiation

Quand une alerte sonne, DumbMonit peut dire quoi faire (**procédures**) et, si
vous l’y autorisez, en faire une partie (**actions de remédiation**), à la main
ou automatiquement. Tout se trouve sur la page **Actions** (dans la barre du
haut ; sur téléphone, depuis la page Alertes).

Exécuter quelque chose sur une machine est sensible. La conception ci-dessous
fait passer la sécurité d’abord : catalogue fermé, dernier mot à la machine,
déclenchement automatique armé en deux étapes, et chaque exécution journalisée.

## Procédures

Une procédure est un texte Markdown (titres, listes, `code`, **gras**, liens)
rattaché à un ou plusieurs éléments :

| Rattachement | Concerne |
|---|---|
| Règle d’alerte | Les alertes de cette règle, par son uid (`host_down`, `disk_full`…) |
| Dossier | Tous les équipements de ce dossier |
| Type d’équipement | Tous les équipements de ce type (`agent`, `snmp`, `proxmox`…) |

Une procédure apparaît :

- sous l’alerte, quand on la déplie sur la page Alertes ;
- sur la page de l’équipement, sous ses alertes ;
- dans les notifications, sous forme de lien
  (`Runbook: <titre>: https://…/runbooks/12`), quand l’URL publique est
  renseignée dans la politique de notification. Le texte lui-même n’est
  jamais envoyé.

Tout HTML d’une procédure est affiché comme du texte, jamais interprété. Seuls
les liens `http(s)://` et les chemins de ce site sont cliquables. Les
administrateurs écrivent les procédures ; tout compte connecté les lit.

## Actions de remédiation

Une action est de l’un de quatre types. Il n’existe aucun champ de commande libre.

| Type | Ce qu’elle fait | Passe par |
|---|---|---|
| Redémarrer un service | `systemctl restart -- <unité>` (Linux) ou `net stop` / `net start` (Windows) | L’agent |
| Redémarrer un conteneur Docker | Le redémarrage du panneau Docker | L’agent |
| Lancer un script déclaré | Un script **déclaré sur la machine elle-même** (voir plus bas) | L’agent |
| Appeler un webhook | Un `POST` JSON décrivant l’action, l’équipement et l’alerte | Le serveur |

Une action peut être fixée sur un équipement, ou suivre l’équipement de
l’alerte qui la déclenche. Le jeton d’un webhook est stocké chiffré et n’est
plus jamais affiché ; l’API dit seulement qu’il existe.

### Ce que la machine autorise : `agent.yaml`

L’agent refuse tout redémarrage de service et tout script tant que sa propre
configuration ne les autorise pas. Rien côté serveur ne peut changer cela.

```yaml
# /etc/dumbmonit/agent.yaml
actions: true                  # défaut : false
action_services: [nginx, php-fpm]   # "*" autorise tout service ; vide : aucun
# actions_dir: /etc/dumbmonit/actions.d   # défaut : à côté de agent.yaml
```

| Clé | Variable | Défaut |
|---|---|---|
| `actions` | `DUMBMONIT_AGENT_ACTIONS` | `false` |
| `action_services` | `DUMBMONIT_AGENT_ACTION_SERVICES` | *(vide)* |
| `actions_dir` | `DUMBMONIT_AGENT_ACTIONS_DIR` | `actions.d` à côté de `agent.yaml` (`C:\ProgramData\DumbMonit\actions.d` sous Windows) |

`commands: false` coupe toujours toutes les commandes, conteneurs compris.
Redémarrez l’agent après avoir changé ces réglages ou ajouté un script.

### Déclarer un script

Un manifeste YAML par script dans le dossier d’actions. Le nom du fichier,
sans `.yaml`, est le nom du script (minuscules, chiffres, `-` et `_`).

```yaml
# /etc/dumbmonit/actions.d/flush-cache.yaml
description: Vider le cache de l’application
command: ["/usr/local/bin/flush-cache", "--quiet"]
timeout_secs: 60               # défaut 60, au plus 300
params:                        # chaque paramètre et ses valeurs permises
  level: [low, high]
```

- `command` est une liste : le programme (chemin absolu) et des arguments
  fixes. Elle n’est jamais passée à un shell.
- Chaque paramètre devient **un seul** argument supplémentaire,
  `--level=high`. Seules les valeurs listées dans le manifeste sont acceptées.
- Le script tourne avec un environnement vide (un `PATH` minimal,
  `DUMBMONIT_ACTION=<nom>`), sans entrée standard, et il est tué à
  l’échéance. Les 16 premiers Kio de sa sortie sont lus ; les 4 derniers Kio
  sont gardés au journal.
- Le dossier, le manifeste et le programme ne doivent être modifiables ni par
  le groupe ni par les autres, et appartenir à `root` ou au compte de l’agent.
  Un manifeste qui enfreint une règle est ignoré (l’agent dit pourquoi dans
  ses journaux).

L’agent déclare au serveur ses services autorisés et ses scripts : le serveur
refuse d’emblée ce que la machine refuserait de toute façon.

## Lancer une action à la main

Dépliez une alerte sur la page Alertes : les actions applicables à son
équipement sont listées sous **Remédiation**. Cliquez, puis confirmez.
Opérateurs et administrateurs peuvent le faire. Un opérateur agit toujours sur
l’équipement de l’alerte (ou sur celui qu’un administrateur a fixé sur
l’action) ; seul un administrateur peut désigner un autre équipement par l’API.

## Remédiation automatique

Une politique lance une action d’elle-même quand une règle continue de sonner :

| Réglage | Sens |
|---|---|
| Après | Minutes pendant lesquelles l’alerte doit sonner (1 à 1440) |
| Tentatives au plus / par fenêtre | Au plus N exécutions par équipement sur une fenêtre glissante |
| Seulement pour cet équipement | Restreint la politique à un équipement de la règle |

Elle ne se déclenche jamais :

- pendant une fenêtre de maintenance ou une mise en sourdine (alerte silencée) ;
- quand un équipement parent est en panne (alerte supprimée) ;
- sur une alerte que quelqu’un a écartée ;
- tant qu’une exécution précédente de la même politique est en attente.

Deux échecs de suite **arrêtent** la politique ; un administrateur doit la réarmer.

### Armer en deux étapes

Une politique naît **désarmée**. **Armer…** demande au serveur un code à usage
unique, valable deux minutes, accompagné d’un résumé en clair de ce qui sera
fait ; **Oui, l’armer** le renvoie. Le code ne vaut que pour le compte qui l’a
demandé, et il est consommé même s’il ne correspond pas. Modifier la politique,
ou l’action qu’elle emploie, la désarme.

## Le journal

Chaque exécution est journalisée : qui (compte, ou `policy:<id>`), quoi
(action, type, paramètres sans secrets), où (équipement, alerte), quand, et le
résultat avec sa sortie (tronquée à 4 Kio, fin conservée). Le journal
apparaît sous l’alerte, sur la page de l’équipement et sur la page Actions. Il
est conservé 180 jours et ne fait pas partie des sauvegardes.

## Sécurité

| Menace | Protection |
|---|---|
| Commande arbitraire | Catalogue fermé ; les scripts n’existent que sur la machine, en manifestes que le serveur ne peut pas écrire. |
| Injection d’arguments | Aucun shell nulle part : programmes et arguments sont remis au système en `argv`. Chaque paramètre est un seul argument `--clé=valeur` ; les valeurs se limitent aux lettres, chiffres et `. _ - : @ / +`, ne commencent jamais par `-`, et doivent figurer dans le manifeste. Un nom de service ne commence ni par `-` ni par `/`, et `systemctl` reçoit `--` avant lui. |
| Traversée de chemin | Un nom de script n’admet ni `/`, ni `\`, ni `.` ; l’agent ne charge qu’un manifeste trouvé en listant son dossier. Pas de liens symboliques. |
| CSRF | Les écritures par cookie de session exigent `X-Requested-With: DumbMonit`. |
| Escalade de rôle | Un lecteur ne lance rien ; un opérateur ne lance que des actions définies par un administrateur, sur l’équipement de l’alerte ; configuration et armement sont réservés aux administrateurs. Les jetons d’API n’ont aucun accès à la remédiation. Armer exige un administrateur connecté, en personne. |
| Rejeu | Chaque demande à l’agent porte un nonce aléatoire et une échéance (10 minutes). L’agent refuse une demande échue, une échéance trop lointaine et un nonce déjà vu. Il n’exécute jamais deux fois la même commande. |
| Déni de service | Au plus 30 exécutions par minute pour toute l’instance, une seule exécution en attente par action et équipement, quotas par politique, délai des scripts (300 s au plus), sortie bornée, déclarations d’agent bornées. |
| SSRF (webhooks) | Même garde que les canaux de notification : les adresses du serveur lui-même sont refusées à la résolution, à la connexion et à chaque redirection. Pas d’identifiants dans l’URL. |
| Fuite de secret | Les jetons de webhook sont chiffrés au repos, jamais renvoyés par l’API, jamais journalisés, et effacés des messages d’erreur. Les sauvegardes les portent dans leur charge chiffrée. |

Risques résiduels à connaître : qui contrôle le serveur peut lancer ce que les
machines autorisent (ne déclarez que ce que vous acceptez de voir tourner) ;
sous Windows, l’agent ne sait pas vérifier lui-même les droits du dossier,
gardez-le modifiable par les seuls administrateurs ; un script déclaré n’est
pas plus sûr que le script lui-même.

## Sauvegarde

Procédures, actions (secrets compris, dans la charge chiffrée) et politiques
font partie des [sauvegardes](../install/backup.md). Les politiques sont
restaurées **désarmées**. Le journal des exécutions n’est pas exporté.
