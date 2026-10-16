# Ajouter votre première surveillance

Ajouter n'importe quoi fonctionne toujours de la même façon : choisissez un
type dans la liste, un avis à droite explique quoi préparer sur l'appareil,
et le formulaire ne montre que les champs qui comptent pour ce type.

## Les trois premières étapes {#the-three-first-steps}

Sur une instance toute neuve, la vue d'ensemble ne montre pas un bulletin
vide : elle montre un petit guide — **ajoutez votre première surveillance**,
**connectez un moyen d'être prévenu**, **vérifiez qu'un message arrive**.
Chaque étape est en un clic, et chacune devient verte par elle-même. La
troisième ne compte qu'un message qui a vraiment quitté un canal, donc créer
un canal et ne jamais le tester ne la coche pas.

Le guide est gardé sur l'instance, pas dans le navigateur : passer une
étape, ou tout le guide, le passe dans chaque navigateur et pour chaque
utilisateur. Il disparaît pour de bon une fois les trois étapes réglées, et
n'apparaît jamais sur une instance qui avait déjà des surveillances et un
canal — mettre à jour ne donne pas de tutoriel à un homelab déjà établi.

## Ajouter un appareil SNMP

SNMP est le type le plus universel : presque tout le matériel réseau le
parle. Commutateurs, routeurs, NAS, onduleurs et imprimantes sont les
suspects habituels.

1. Sur l'appareil, activez SNMP v2c et notez la communauté en lecture seule
   (`public` par défaut sur beaucoup d'appareils). Si l'appareil filtre
   par adresse, autorisez l'adresse du serveur DumbMonit.
2. Dans DumbMonit, ouvrez **Surveillances** et cliquez sur **Ajouter une surveillance** (ou appuyez sur ++ctrl+k++ et tapez « add »).
3. Choisissez **Appareil SNMP**. Le sélecteur se replie en une ligne et le
   formulaire montre les champs pour SNMP.
4. Saisissez un **nom**, l'**adresse** (IP ou nom d'hôte) et la
   **communauté**. Pour SNMP v3, changez l'identifiant pour *SNMP v3* et
   remplissez le nom d'utilisateur et les phrases de passe d'authentification
   et de confidentialité.
5. Cliquez sur **Ajouter la surveillance**.

![Ajouter un appareil SNMP : le formulaire à gauche, l'avis de configuration à droite](../assets/screenshots/add-device-light.png){ loading=lazy }

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

- La surveillance apparaît sur la page **Surveillances** comme une façade de rack :
  LED d'état, nom, type, adresse, dernière vue, et une sparkline.
- Les mesures sont écrites dans VictoriaMetrics toutes les 5 secondes
  (`DUMBMONIT_FLUSH_INTERVAL_SECS`), le premier graphique apparaît donc
  après la première sonde plus quelques secondes. **Sonder maintenant** sur
  la page de la surveillance force une sonde et rapporte combien d'échantillons
  et de séries elle a produits : c'est le principal outil de diagnostic.
- Les [règles d'alerte intégrées](../alerting/rules.md) s'appliquent
  immédiatement : surveillance inaccessible, CPU saturé, disque presque plein,
  et ainsi de suite. Rien à configurer.
- Pour être notifié, ajoutez un
  [canal de notification](../notifications.md) dans
  **Réglages → Notifications et rapports**. Les règles intégrées notifient chaque canal
  activé.

!!! tip "Aucune donnée après une minute ?"
    Ouvrez la page de la surveillance : une erreur de configuration (mauvaise
    communauté, certificat refusé, fonctionnalité manquante) est affichée
    en rouge sous l'adresse, avec la raison. Voir la
    [FAQ](../faq.md#no-data-after-adding-a-device).
