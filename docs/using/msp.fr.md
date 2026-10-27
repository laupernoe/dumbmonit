# Mode multi-clients (MSP)

Le mode multi-clients permet à un seul DumbMonit de servir plusieurs clients en
gardant leurs données séparées. Chaque **client** a ses équipements, ses pages
de statut, ses jetons d'agent et ses rapports, et ses membres ne voient jamais
les données d'un autre client.

Les clients se placent au-dessus des [équipes et dossiers](teams.md) : le client
décide de ce qu'un compte peut voir, puis ses équipes le restreignent dossier
par dossier.

Ouvrez **Réglages → Équipe et accès → Clients**, réservé aux administrateurs.

## Activation

Tant que le **mode multi-clients** n'est pas activé, rien ne change : une
instance mono-client se comporte exactement comme avant.

À la mise à jour, tout l'existant part dans un client nommé **Default** :
équipements, pages de statut et jetons d'agent. Les comptes non
administrateurs en deviennent membres : activer le mode plus tard ne surprend
personne. Canaux de notification, règles d'alerte, maintenances et rapports
planifiés restent **partagés** par toute l'instance.

Mode activé :

- **Les administrateurs sont administrateurs MSP.** Ils voient et gèrent tous
  les clients, et en choisissent un dans l'en-tête pour n'afficher que ses
  pages. Ce choix filtre l'affichage ; il ne retire aucun droit.
- **Tout autre compte ne voit que les clients dont il est membre**, dans les
  limites de son rôle et de ses équipes. Un compte membre d'aucun client ne voit
  rien ; les réglages des clients listent ces comptes pour que vous les placiez.
- Un compte créé mode activé n'appartient à aucun client tant que vous ne l'y
  ajoutez pas, ou qu'un groupe du fournisseur d'identité ne l'y fait pas entrer.

## Ce qui appartient à un client

| Objet | Appartient à |
|---|---|
| Équipements (agents compris) | Toujours un client |
| Pages de statut | Un client, ou l'instance |
| Jetons d'enrôlement d'agent | Toujours un client |
| Canaux de notification | Un client, ou l'instance |
| Règles d'alerte | Un client, ou l'instance |
| Maintenances | Un client, ou l'instance (une maintenance sur un équipement suit cet équipement) |
| Rapports planifiés | Un client, ou l'instance |

Ce que l'instance partage sert tous les clients : un canal de l'instance entend
parler de tous les clients, une règle de l'instance surveille tous les
équipements. Ce qu'un client possède ne concerne que lui :

- le **canal** d'un client ne reçoit que les alertes des équipements de ce
  client — même quand une règle le désigne — et jamais une alerte sans
  équipement ;
- la **règle** d'un client n'évalue que les équipements de ce client ;
- la **maintenance** d'un client ne fait taire que ses équipements ;
- la **page de statut** d'un client ne liste que ses équipements ;
- le **rapport planifié** d'un client ne parle que de ses équipements.

Un objet créé pendant qu'un client est choisi dans l'en-tête appartient à ce
client. Pour déplacer un objet existant, utilisez le menu de client à côté de
lui (pages de statut, canaux, rapports) ou le champ **Client** d'un
équipement. Règles d'alerte et maintenances se déplacent par l'API
(`PUT /api/clients/assign`).

### Déplacer un équipement

Déplacer un équipement vers un autre client emporte ce qui en dépend : ses
enfants, les équipements qu'un agent relaie et les moniteurs créés par les
étiquettes Docker d'un agent. Tout lien restant entre deux clients est coupé —
parent, relais, emplacement de sonde d'un moniteur multisite, ligne de la page
de statut d'un autre client. Un relais n'interroge donc jamais, et ne reçoit
jamais les identifiants, d'un équipement d'un autre client.

## Agents

Chaque client a ses propres jetons d'enrôlement : créez le jeton pendant que le
client est choisi dans l'en-tête. Une machine enrôlée avec le jeton d'un client
entre chez ce client, et seulement chez lui :

- une nouvelle machine devient un équipement du client du jeton ;
- un jeton n'adopte jamais un équipement déclaré chez un autre client, et ne
  réenrôle (ne relie) jamais une machine d'un autre client ;
- une machine déjà liée continue d'envoyer dans son propre équipement, reconnue
  à son secret de liaison.

## Quotas

Un client peut avoir un **quota d'équipements** : le nombre maximal
d'équipements, agents et moniteurs d'étiquettes Docker compris. Au-delà, les
nouveaux équipements et les enrôlements d'agent de ce client sont refusés.
Laissez le champ vide pour ne rien limiter.

Les imports en masse (CSV, Uptime Kuma) placent les équipements dans le client
Default ; déplacez-les ensuite.

## Membres et authentification unique

Ajoutez les membres à la main, ou listez des groupes du fournisseur d'identité
(OIDC, LDAP, SAML) : un compte de l'un de ces groupes rejoint le client à la
connexion, et le quitte quand il perd le groupe. Un membre ajouté à la main
n'est jamais retiré par une connexion.

## Rapports et marque

Donnez un **logo** au client (PNG, JPEG ou WebP, 256 Kio au plus). Il apparaît :

- sur les pages de statut du client qui n'ont pas de logo propre, avec le nom du
  client ;
- en tête du rapport de disponibilité du client (PNG et JPEG ; pour WebP, le nom
  seul).

Activez **L'envoyer aux contacts** pour envoyer le rapport de disponibilité
(SLA) du mois écoulé aux contacts du client : le 1er à 8 h dans le fuseau du
client, un PDF et deux CSV, au nom et au logo du client. Il part par le canal
courriel du client s'il en a un, sinon par celui de l'instance. Un mois n'est
envoyé qu'une fois ; un mois manqué n'est pas rattrapé.

Les membres d'un client téléchargent leur rapport depuis la page **Clients**.
L'administrateur MSP y retrouve tous les clients, avec ce qui est en alerte et
le remplissage de leur quota.

## Ce que les membres atteignent

Un membre de client est un compte restreint, comme un membre d'équipe : le
serveur ne lui ouvre qu'une liste fermée de pages — équipements, métriques,
alertes, maintenance sur ses équipements, carte, pages de statut,
vulnérabilités, logos et rapports de ses clients — et refuse tout le reste
(réglages, journaux, sauvegardes, jetons, point d'accès de l'assistant, séries
brutes). Chaque liste est filtrée côté serveur ; un équipement d'un autre client
répond *introuvable*, comme s'il n'existait pas. Les jetons d'API d'un membre
héritent des mêmes limites.

!!! note "Limite connue"
    Deux équipements de même type et de même adresse ne peuvent pas coexister,
    même chez deux clients. Ajouter un équipement dont un autre client surveille
    déjà l'adresse est refusé, ce qui révèle que l'adresse est surveillée
    quelque part.

## Sauvegardes

Les sauvegardes emportent les clients, leurs membres, leurs réglages, leurs
logos, et le client de chaque équipement, page de statut, jeton d'agent, canal,
règle, maintenance et rapport planifié.
