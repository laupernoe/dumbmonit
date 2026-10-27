# Découverte réseau

La découverte réseau trouve pour vous ce qui se trouve sur un réseau : vous
ajoutez des moniteurs en choisissant dans une liste au lieu de saisir les
adresses une à une. Vous donnez un réseau en notation CIDR (`192.168.1.0/24`) ;
elle cherche d'abord les hôtes vivants par un ping (et la table des voisins
quand elle est lisible), ouvre une courte liste de ports courants sur chaque adresse, puis fait
**une seule requête anonyme** vers chaque port ouvert pour reconnaître le
produit derrière. Elle ne tente jamais de s'authentifier.

Un même hôte peut porter plusieurs services : un nœud Proxmox qui répond aussi
en SSH et en SNMP les montre tous les trois, chacun avec le type de moniteur
DumbMonit qui convient. Un hôte qui répond au ping mais n'expose aucun service
reconnu est quand même listé, avec « Aucun service identifié ». Le ping exige
que l'ICMP soit permis au conteneur (le réglage `ping_group_range` des fichiers
compose fournis, ou `NET_RAW` en mode hôte) ; sans lui, le balayage continue
avec les seuls ports et la table des voisins.

## Lancer un balayage

Sur la page **Moniteurs**, lancez un balayage, choisissez un réseau suggéré (ou
saisissez-en un) et la profondeur :

- **Standard** balaie une trentaine de ports courants, plus SNMP.
- **Étendu** ajoute les ports moins fréquents et tout ce que réclament les
  empreintes des produits. C'est plus lent et plus bruyant.

Un balayage est limité à **1024 adresses (un /22)**. Un préfixe plus large est
refusé : balayer un /16 entier ferait des dizaines de milliers de connexions.
Les résultats se remplissent au fil du balayage ; vous pouvez l'annuler à tout
moment.

Pour chaque service reconnu, vous obtenez le nom du produit, un niveau de
confiance, la version quand elle est lisible sans authentification, et un bouton
qui ouvre le formulaire **Ajouter un moniteur** déjà rempli avec le bon type,
l'adresse et le port — vous n'avez plus qu'à saisir les identifiants. Si un
moniteur pointe déjà sur cette adresse, il est signalé « déjà surveillé ».

La community SNMP que vous donnez est un secret (vide = `public`) : elle est envoyée aux
équipements sondés, jamais écrite dans un journal ni renvoyée par l'API.

## Sites distants

Pour balayer un réseau que le serveur ne peut pas joindre, choisissez un **agent
relais** de ce site. L'agent exécute exactement le même balayage sur son propre
réseau et en renvoie les résultats au fil de l'eau ; le serveur n'a jamais à
joindre l'agent. L'adresse IP de l'agent n'a pas besoin d'être joignable depuis
le serveur.

La découverte par relais exige un agent récent. Un agent trop ancien pour
comprendre la commande fait échouer le balayage avec un message clair qui vous
invite à le mettre à jour.

## Adresses MAC et mode réseau host

L'adresse MAC d'un hôte — et le fabricant qu'elle désigne — ne se lit que sur le
**même segment de niveau 2**. Par défaut, le conteneur tourne sur son propre
réseau Docker, à un saut de distance : la colonne MAC reste donc vide. Pour
l'obtenir, lancez le conteneur en **mode réseau host**, où il partage la pile
réseau de l'hôte et peut lire la table des voisins (ARP) :

```bash
docker compose -f docker-compose.host.yml up -d
```

Utilisez ce fichier seul, sans le fusionner avec `docker-compose.yml`.

Le mode réseau host :

- ne fonctionne que **sous Linux**. Sur Docker Desktop (macOS, Windows), le
  conteneur tourne dans une VM : pas de segment partagé, pas de MAC.
- expose l'interface web directement sur le port 8080 de l'hôte (il n'y a pas de
  redirection de port en mode host). La base de métriques embarquée reste liée à
  la boucle locale et n'est pas joignable depuis le réseau.

## Scans planifiés et inventaire

Un balayage ponctuel dit ce qu'il y a sur le réseau aujourd'hui. Pour être
prévenu quand quelque chose de **nouveau** apparaît, planifiez-le : sur la page
**Équipements**, dans la vue de balayage réseau, le panneau *Scans planifiés et
inventaire* garde la liste des appareils et des ports ouverts que chaque réseau
planifié a montrés, et signale ce qui est nouveau.

Pour chaque planification, vous choisissez :

- le **réseau** (CIDR, 1024 adresses au plus, comme un balayage manuel) ;
- **qui balaie** : le serveur, ou un [agent relais](../devices/agent.fr.md) sur un autre site ;
- l'**intervalle**, de 5 minutes à 30 jours ;
- la **profondeur** (la même liste de ports *Standard* ou *Étendue* qu'un balayage manuel) ;
- si elle est **activée**. *Scanner maintenant* la lance au prochain cycle
  (quelques secondes) sans attendre l'intervalle.

Un réseau n'est jamais balayé deux fois en même temps, et un serveur ou un agent
relais ne mène qu'un balayage de découverte à la fois : un scan planifié qui trouve
son exécutant occupé (balayage manuel, autre planification) démarre simplement un
peu plus tard.

### L'inventaire

Chaque balayage terminé met à jour l'inventaire de son réseau :

- un **appareil** est identifié par son **adresse MAC** quand elle est lisible
  (voir [adresses MAC](#adresses-mac-et-mode-reseau-host)), sinon par son adresse IP.
  Un appareil dont l'IP change mais qui garde sa MAC est le même appareil, pas un
  nouveau ;
- chaque **port ouvert** trouvé sur un appareil est listé avec sa première et sa
  dernière observation. Seul ce que le balayage reconnaît est listé (les mêmes
  services qu'un balayage manuel) ;
- rien n'est jamais retiré : un appareil éteint reste dans la liste.

Le **premier balayage terminé** d'un réseau construit l'inventaire sans rien
signaler : tout ce qu'il trouve est connu d'emblée. Un premier balayage qui ne trouve
aucun hôte (un relais coupé de son réseau, par exemple) ne compte pas, pour que le
suivant ne signale pas tout un réseau comme nouveau. Changer le réseau ou l'exécutant
d'une planification repart d'un inventaire vide.

À partir du deuxième balayage, un appareil ou un port absent de l'inventaire y entre
comme **nouveau** : mis en avant par une étiquette *Nouvel appareil* ou *Nouveau
port* (pas seulement une couleur), et gardé en tête de liste.

### Alertes

Les nouveaux appareils et les nouveaux ports déclenchent des alertes par deux
[règles d'alerte](../alerting/rules.fr.md#inventaire-du-reseau) livrées :

| Règle | Se déclenche quand | Par défaut |
|---|---|---|
| New device on the network | un balayage trouve un appareil absent de l'inventaire | **désactivée**, gravité `info` |
| New open port on the network | un balayage trouve un nouveau port ouvert sur un appareil connu | **désactivée**, gravité `info` |

Les deux règles sont **désactivées par défaut** : sur un réseau où des invités vont
et viennent, elles ne seraient que du bruit tant que vous ne les voulez pas.
Activez-les dans les règles d'alerte, et choisissez les canaux de notification qui
les reçoivent comme pour n'importe quelle règle.

Il y a **une alerte par nouvel élément**, et elle reste ouverte tant que l'élément
n'est pas marqué comme connu ; elle ne se répète pas et ne se redéclenche pas à
chaque balayage. Les ports d'un nouvel appareil sont couverts par l'alerte de
l'appareil lui-même.

### Marquer comme connu

Utilisez *Marquer comme connu* sur un appareil (ses ports sont marqués avec lui),
*Marquer connu* sur un seul port, ou *Tout marquer comme connu* pour tout accepter
d'un coup. Les éléments marqués ne sont plus signalés, et leur alerte se résout en
quelques minutes. C'est aussi le moyen d'accepter un nouveau port ou appareil que
vous attendiez.

## Un avertissement

Un balayage de ports ressemble à une attaque pour un système de détection
d'intrusion (IDS/IPS). Ne balayez que les réseaux qui vous appartiennent ou que
vous êtes autorisé à balayer, et gardez un préfixe étroit. La limite du /22 est
là pour la même raison.
