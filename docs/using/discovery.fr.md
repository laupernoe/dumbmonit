# Découverte réseau

La découverte réseau trouve pour vous ce qui se trouve sur un réseau : vous
ajoutez des moniteurs en choisissant dans une liste au lieu de saisir les
adresses une à une. Vous donnez un réseau en notation CIDR (`192.168.1.0/24`) ;
elle ouvre une courte liste de ports courants sur chaque adresse, puis fait
**une seule requête anonyme** vers chaque port ouvert pour reconnaître le
produit derrière. Elle ne tente jamais de s'authentifier.

Un même hôte peut porter plusieurs services : un nœud Proxmox qui répond aussi
en SSH et en SNMP les montre tous les trois, chacun avec le type de moniteur
DumbMonit qui convient.

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

La community SNMP que vous donnez est un secret : elle est envoyée aux
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

## Un avertissement

Un balayage de ports ressemble à une attaque pour un système de détection
d'intrusion (IDS/IPS). Ne balayez que les réseaux qui vous appartiennent ou que
vous êtes autorisé à balayer, et gardez un préfixe étroit. La limite du /22 est
là pour la même raison.
