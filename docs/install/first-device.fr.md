# Ajouter votre première surveillance

Cette page vous mène d'une installation neuve à une première alerte dans votre
boîte de réception. Chaque étape dit ce que vous devez voir à l'écran. Si ce
n'est pas le cas, allez à [Quand ça ne marche pas](#quand-ca-ne-marche-pas).

Prévoyez une dizaine de minutes et l'adresse d'une chose à surveiller : un
serveur, un NAS, un commutateur ou un site web.

## 1. Premier lancement

1. Démarrez DumbMonit (voir [Docker](docker.md)) et ouvrez
   `https://<votre-hôte>:8080`.
2. Votre navigateur signale le certificat. C'est normal : DumbMonit en crée un
   autosigné au premier démarrage. Acceptez-le une fois. Pour le remplacer, voir
   [HTTPS et chiffrement](https.md).
3. L'écran **Créons votre compte.** s'affiche. Remplissez :
    - **Code de configuration** : affiché dans les journaux du serveur. Lancez
      `docker compose logs dumbmonit | grep -A1 "setup code"` et copiez le code
      (par exemple `K7XQ4-M9PRT`).
    - **Nom d'utilisateur** (sans espace) et **mot de passe** d'au moins 12
      caractères. Une phrase entière vaut mieux qu'un mot compliqué.
4. Cliquez sur **Créer le compte et continuer**. Vous êtes connecté.

!!! warning "Notez le mot de passe"
    Il ne peut pas être récupéré depuis l'écran. En cas de perte, il se
    réinitialise sur la machine qui héberge DumbMonit : voir la
    [FAQ](../faq.md#i-lost-the-password).

!!! note "Code de configuration refusé ?"
    Le code change à chaque redémarrage tant qu'aucun administrateur n'existe.
    Copiez le dernier affiché dans les journaux.

## 2. L'assistant de configuration

Juste après la connexion, un court assistant peut vous guider. Chaque étape peut
être passée et tout se change plus tard dans les **Réglages**. Utilisez
**Continuer**, **Passer cette étape** ou **Passer la configuration**. Il couvre :
le nom de l'instance, la langue et le fuseau horaire ; un agent ; vos premières
surveillances ; un canal de notification ; une astreinte (facultatif) ; la
connexion unique (facultatif) ; la durée de conservation des données. Vous le
rouvrez depuis **Réglages → Système**.

## Les trois premières étapes {#the-three-first-steps}

Sur une instance toute neuve, la vue d'ensemble montre un petit guide plutôt
qu'une page vide : **ajoutez votre première surveillance**, **connectez un moyen
d'être prévenu**, **vérifiez qu'un message arrive**. Chaque étape est un bouton
et devient verte d'elle-même. La troisième ne compte qu'un message qui a
vraiment quitté un canal : créer un canal sans jamais le tester ne la coche pas.

Le guide est gardé sur l'instance, pas dans le navigateur : passer une étape, ou
tout le guide, le passe pour tous les utilisateurs. Il disparaît pour de bon une
fois les trois étapes réglées, et n'apparaît jamais sur une instance qui avait
déjà des surveillances et un canal.

## 3. Ajouter une surveillance

1. Ouvrez **Surveillances** et cliquez sur **Ajouter une surveillance** (ou
   appuyez sur ++ctrl+k++ et tapez « add »).
2. **Où se trouve-t-il ?** DumbMonit demande d'abord un site, c'est-à-dire un
   lieu de la [carte du monde](../using/world-map.md). Choisissez l'un de vos
   lieux, ou **Aucun site**. Il n'y a pas de site par défaut : une surveillance
   « Aucun site » n'est simplement pas rattachée à un lieu de la carte. Les sites
   sont facultatifs : sans aucun lieu, vous arrivez directement à la liste des
   types.
3. Choisissez un type. Les plus courants viennent en premier ; la recherche
   (*Rechercher : switch, NAS, Docker, sauvegarde, site web…*) trouve les autres.
4. Un avis à droite (au-dessus sur téléphone) liste ce qu'il faut préparer sur
   l'appareil. Le formulaire ne montre que les champs utiles pour ce type.
5. Remplissez le formulaire et cliquez sur **Ajouter la surveillance**.

Vous arrivez sur la surveillance. Un voyant gris signifie qu'elle attend sa
première réponse. En moins d'une minute elle passe à **Reporting** et les
premières valeurs apparaissent.

!!! tip "Quel type choisir ?"
    Un site web : *HTTP* dans les [services](../devices/services.md). Un
    serveur où vous pouvez installer un logiciel : *Serveur avec l'agent*. Un
    commutateur, routeur, imprimante ou onduleur : [SNMP](../devices/snmp.md).
    Un NAS : voir plus bas.

### Un serveur avec l'agent {#server-with-the-agent}

L'agent est un petit programme qui tourne sur la machine et envoie ses mesures à
DumbMonit. Ni adresse ni mot de passe à saisir : la machine se présente avec un
jeton.

1. **Ajouter une surveillance**, choisissez un site, puis **Serveur avec
   l'agent**.
2. Choisissez le système de la machine : **Linux / macOS**, **Windows** ou
   **Docker**. Pour une machine derrière un pare-feu sur un autre site,
   choisissez de joindre le serveur **Via** un relais plutôt que **En direct**
   (voir [Surveiller un site distant](remote-site.md)).
3. Cliquez sur **Créer la commande d'installation**. Vous voyez la commande, le
   jeton et la mention **Affiché une seule fois**. Copiez la commande
   maintenant : le jeton ne sera plus affiché.
4. La page vérifie que les machines joignent le canal chiffré des agents (TCP
   8443). Vous devez voir *Canal chiffré des agents joignable à … (TLS 1.3)*.
   Sinon, voir le dépannage plus bas.
5. Sur la machine, lancez la commande avec les droits élevés : `sudo` sur Linux
   et macOS, un PowerShell administrateur sur Windows. Pour Docker, copiez la
   commande Docker ou les deux lignes Compose.
6. Sur la page, *En attente du premier envoi de l'agent…* devient
   *« hôte » est connectée (système)* en quelques secondes.
7. Donnez un **Nom** à la machine, éventuellement un **Dossier** et un **Site**,
   puis **Enregistrer**. **Ouvrir la machine** affiche sa page.

Pour régler l'agent plus tard, ouvrez la surveillance et cliquez sur la **roue
crantée** (réglages) : réglages de l'agent, mode relais, collecte des journaux,
services à surveiller et ré-enrôlement s'y trouvent. Détails :
[l'agent](../devices/agent.md).

??? info "Plusieurs machines avec la même commande"
    Cochez **Plusieurs machines avec cette commande** avant de créer la commande
    pour obtenir un jeton réutilisable (playbook, image). Révoquez-le dans
    **Réglages → Supervision → Agents** une fois terminé.

### Un NAS Synology

Deux méthodes, combinables sur le même NAS (voir
[Paquet ou API ?](../devices/synology.md#paquet-ou-api)).

- **Paquet DumbMonit Agent** (DSM 7 uniquement) : vous installez un `.spk` sur le
  NAS, qui se connecte à DumbMonit. Aucun port ouvert, aucun mot de passe DSM
  stocké.
- **Identifiants de l'API DSM** (DSM 6 et 7) : DumbMonit se connecte à DSM avec
  un compte en lecture seule.

Pas à pas : [Agent Synology](../devices/synology-agent.md) et
[Synology DSM](../devices/synology.md).

## Ajouter un appareil SNMP

SNMP est le type le plus universel : presque tout le matériel réseau le
parle. Commutateurs, routeurs, NAS, onduleurs et imprimantes sont les
suspects habituels.

1. Sur l'appareil, activez SNMP v2c et notez la communauté en lecture seule
   (`public` par défaut sur beaucoup d'appareils). Si l'appareil filtre
   par adresse, autorisez l'adresse du serveur DumbMonit.
2. Dans DumbMonit, cliquez sur **Ajouter une surveillance** et choisissez un site (ou **Aucun site**).
3. Choisissez **Appareil SNMP**. Le formulaire montre les champs SNMP.
4. Saisissez un **nom**, l'**adresse** (IP ou nom d'hôte) et la
   **communauté**. Pour SNMP v3, changez l'identifiant pour *SNMP v3* et
   remplissez le nom d'utilisateur et les phrases de passe d'authentification
   et de confidentialité.
5. Cliquez sur **Ajouter la surveillance**. Vous arrivez sur la page de la
   surveillance ; les interfaces et, sur serveurs et NAS, CPU, mémoire et stockage
   apparaissent en moins d'une minute.

Le profil de collecte est détecté automatiquement d'après le `sysObjectID`
de la surveillance, en arrière-plan, le formulaire n'attend donc jamais une surveillance inaccessible. Les interfaces, et sur les serveurs et NAS le CPU, la
mémoire et le stockage, apparaissent en moins d'une minute.

!!! warning "Les communautés voyagent en texte clair"
    Une communauté SNMP v1/v2c n'est pas chiffrée sur le réseau. Sur un
    réseau partagé, préférez SNMP v3, qui authentifie et chiffre.

## Scanner un réseau

Plutôt que de saisir les surveillances un par un, scannez une plage :

1. Sur **Ajouter une surveillance**, ouvrez le panneau **Scanner mon réseau**.
2. Saisissez un réseau en notation CIDR, par exemple `192.168.1.0/24`
   (jusqu'à 4096 adresses par scan), et la communauté à essayer sur chaque
   adresse (`public` par défaut).
3. Chaque adresse qui répond est listée avec son `sysName`, sa description
   et le profil qui serait appliqué. Les surveillances déjà dans DumbMonit sont
   marqués *Déjà ajouté*, et chaque nouveauté est cochée pour vous.
4. Décochez ce que vous ne voulez pas, puis cliquez sur **Ajouter *n*
   surveillances**. Ils sont créés un par un, chaque ligne montrant son propre
   résultat, si bien qu'un échec n'arrête jamais les autres.

## Ajouter un serveur Proxmox VE

Le même flux, avec un identifiant en deux champs plutôt qu'un :

1. Dans Proxmox, créez un utilisateur réservé à la surveillance, un rôle en
   lecture seule et un jeton d'API — les quatre commandes `pveum` sont dans
   l'avis à côté du formulaire, chacune avec un bouton de copie, et sur la
   [page Proxmox VE](../devices/proxmox.md). Ne réutilisez jamais le compte
   avec lequel vous vous connectez.
2. Dans DumbMonit, **Ajouter une surveillance** → **Proxmox VE**.
3. Saisissez un **nom** et l'**adresse** de n'importe quel nœud (port 8006
   par défaut).
4. Copiez le **full-tokenid** du jeton (`dumbmonit@pve!monitor`) dans
   **Token ID** et sa **valeur** (l'UUID) dans **Secret**. Vous avez collé
   toute la chaîne `user@pve!name=secret` dans Token ID par erreur ? Le
   formulaire la découpe et le dit.
5. Proxmox livre un certificat autosigné : cochez **Accepter un certificat
   non vérifiable** sous *Plus d'options* si la première sonde s'en
   plaint.
6. Cliquez sur **Ajouter la surveillance**.

[Proxmox Backup Server](../devices/pbs.md) fonctionne de la même façon avec
`proxmox-backup-manager` ; un [NAS Synology](../devices/synology.md) prend
un compte DSM dédié (nom d'utilisateur et mot de passe) ; un
[serveur avec l'agent](../devices/agent.md) n'a besoin d'aucun identifiant du
tout, seulement du jeton d'enrôlement montré une fois la surveillance enregistrée.

## 4. Être prévenu

Les règles d'alerte intégrées s'appliquent dès qu'une surveillance existe :
surveillance injoignable, CPU saturé, disque presque plein, etc. Rien à
configurer. Elles ne vous atteignent qu'une fois un canal créé.

1. Ouvrez **Réglages → Notifications** (ou l'étape *Connecter un moyen d'être
   prévenu* du guide).
2. Cliquez sur **Ajouter un canal** et choisissez un type : e-mail, ntfy,
   Discord, Telegram, PagerDuty, etc. Remplissez les champs demandés.
3. Cliquez sur **Envoyer un test**. Le message doit arriver. Les règles
   intégrées notifient tous les canaux activés.
4. Pour voir une vraie alerte, mettez l'appareil surveillé en pause ou
   débranchez-le. Après le délai de la règle, l'alerte apparaît dans **Alertes →
   Maintenant** et le canal reçoit le message.

Tous les types de canaux : [Notifications](../notifications.md). La page des
alertes : [Alertes](../using/alerts.md).

## Quand ça ne marche pas {#quand-ca-ne-marche-pas}

| Ce que vous voyez | Que faire |
|---|---|
| La surveillance reste grise après une minute | Ouvrez-la et cliquez sur **Sonder maintenant** (icône d'impulsion). Elle indique combien d'échantillons elle a reçus. |
| Un message rouge sous l'adresse | Lisez-le : mauvaise communauté, certificat refusé, droit manquant. Corrigez avec la **roue crantée** ; laisser le secret vide conserve celui enregistré. |
| *Injoignable* | L'appareil n'a pas répondu : vérifiez l'adresse, un pare-feu, et que l'appareil est allumé. |
| Agent : *Le port 8443 n'est pas publié* | Publiez-le : sous `ports:` dans `docker-compose.yml`, ajoutez `"8443:8443"`, puis `docker compose up -d`. Ouvrez le TCP 8443 dans le pare-feu du serveur. |
| Agent : *Aucune réponse sur le port 8443* | Un pare-feu bloque, ou l'adresse est fausse. Ouvrez la page avec l'adresse que la machine utilise pour joindre le serveur. |
| Agent : la machine n'apparaît jamais | Vérifiez le service sur la machine : `journalctl -u dumbmonit-agent -f` (Linux). Voir [Erreurs courantes](../devices/agent.md#erreurs-courantes). |
| Le message de test n'arrive pas | Vérifiez les champs du canal et vos courriers indésirables, puis renvoyez le test. |

Plus dans la [FAQ](../faq.md#no-data-after-adding-a-device).

## L'avis de configuration

Chaque type porte son propre avis, écrit par le serveur et montré à côté du
formulaire : des étapes numérotées à faire sur l'appareil (créer un
utilisateur en lecture seule, créer un jeton d'API, autoriser l'adresse…),
avec le chemin exact de menu dans le produit et un bouton de copie sur
chaque commande ou valeur, puis un piège courant et un lien vers la
documentation du fournisseur quand il y en a une. Les pages par type de
cette documentation portent les mêmes étapes mot pour mot — un test dans le
serveur les garde identiques : [SNMP](../devices/snmp.md),
[Proxmox VE](../devices/proxmox.md),
[Proxmox Backup Server](../devices/pbs.md),
[Synology DSM](../devices/synology.md), [agent](../devices/agent.md),
[services](../devices/services.md).

Le principe derrière chaque avis : un compte dédié en lecture seule, jamais
celui avec lequel vous vous connectez. Un secret de surveillance qui fuit ne
doit pouvoir rien changer.

## Champs communs

Quel que soit le type, le formulaire a :

| Champ | Signification |
|---|---|
| Nom | Comment la surveillance apparaît dans les listes et les alertes. |
| Adresse | IP, nom d'hôte, `host:port` ou URL selon le type ; le placeholder montre la forme attendue. |
| Identifiant | Les familles que le type accepte, chacune avec ses propres champs : communauté, utilisateur et phrases de passe SNMP v3, jeton Proxmox (Token ID + Secret), nom d'utilisateur et mot de passe. Les secrets ont un œil montrer/cacher et les valeurs collées sont nettoyées des espaces, guillemets et retours à la ligne superflus. Stockés chiffrés, jamais renvoyés par l'API : en modification, les champs vides conservent ce qui est enregistré. |
| Intervalle de vérification | À quelle fréquence DumbMonit lit la surveillance. 60 s par défaut, 10 s au minimum. |
| Surveillance parente | Si le parent tombe en panne, les alertes de cette surveillance sont supprimées plutôt qu'envoyées. Voir [suppression par dépendance](../alerting/index.md#dependency-suppression). |
| Activé | Une surveillance en pause n'est pas vérifiée et ne déclenche aucune alerte. |
| Étiquettes | Labels clé/valeur libres, copiés sur chaque série comme `tag_<key>`. Les options spécifiques au type vivent aussi ici, sous **Plus d'options**. |

## Ce qui se passe ensuite

- La surveillance apparaît sur la page **Surveillances** : voyant d'état, nom,
  type, adresse, dernière vue.
- Les mesures sont écrites dans VictoriaMetrics toutes les 5 secondes
  (`DUMBMONIT_FLUSH_INTERVAL_SECS`), le premier graphique apparaît donc
  après la première sonde plus quelques secondes. **Sonder maintenant** sur
  la page de la surveillance force une sonde et rapporte combien d'échantillons
  et de séries elle a produits : c'est le principal outil de diagnostic.

!!! tip "Aucune donnée après une minute ?"
    Ouvrez la page de la surveillance : une erreur de configuration (mauvaise
    communauté, certificat refusé, fonctionnalité manquante) est affichée
    en rouge sous l'adresse, avec la raison. Voir la
    [FAQ](../faq.md#no-data-after-adding-a-device).
