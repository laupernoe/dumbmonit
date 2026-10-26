# Équipes et droits par dossier

Une équipe donne à un groupe de comptes un niveau d'accès sur des **dossiers**
d'équipements. À utiliser quand plusieurs groupes partagent un même DumbMonit
— un NOC et un support, ou une équipe par site — et que chacun ne doit voir
que son matériel.

Ouvrez **Settings → Team & access**, réservé aux administrateurs.

## Niveaux de droit

Pour chaque dossier, une équipe a un niveau. Chaque niveau inclut les
précédents.

| Niveau | Ce que les membres peuvent faire |
|---|---|
| **none** | Rien : le dossier est masqué. |
| **read** | Voir les équipements, leurs métriques et leurs alertes. |
| **operate** | En plus : acquitter et écarter les alertes, poser une maintenance sur un équipement. |
| **manage** | En plus : modifier, ajouter et supprimer les équipements du dossier, lancer une sonde. |

Noms de dossier :

- `*` désigne tous les dossiers.
- **Unfiled** désigne les équipements sans dossier.
- Un droit sur `Paris` couvre aussi `Paris/Rack A`. Les sous-dossiers
  s'écrivent avec `/` dans le nom du dossier.

## Combinaison des droits

- Un compte dans **plusieurs équipes** obtient, pour chaque dossier, le
  niveau **le plus fort** de ses équipes.
- Le **rôle global plafonne** le résultat : un *viewer* a *read* au maximum,
  un *operator* a *manage* au maximum. Un *admin* voit et fait tout et n'est
  jamais restreint.
- Un compte **sans équipe** voit tout, comme avant. Choisissez **Accounts
  without a team see nothing** pour inverser ce comportement.

## Ce que voit un compte restreint

Un compte membre d'au moins une équipe ne voit **que ses dossiers**. C'est le
serveur qui l'applique à chaque requête, pas l'interface :

- la liste et le détail des équipements, et leurs métriques (chaque requête de
  métriques est filtrée par équipement) ;
- les alertes et leur historique, les silences, et les règles d'alerte (sans
  les équipements des autres dossiers) ;
- la carte/topologie, les emplacements, et les pages de statut dont tous les
  composants sont visibles pour le compte ;
- les CVE et les notifications Web Push.

Sont **refusés aux comptes restreints**, ainsi qu'aux jetons d'API de ces
comptes : les journaux, les relais, la sauvegarde, les canaux de notification,
les rapports planifiés, les incidents, le mur, l'auto-surveillance, l'API
Prometheus/federate et l'endpoint MCP. Un jeton d'API hérite des équipes de
son propriétaire.

Un opérateur restreint ne peut poser une maintenance que sur un équipement
précis, pas sur l'ensemble.

## Membres

Une équipe reçoit ses membres de trois façons, cumulables :

1. **À la main** — ajoutez des comptes à l'équipe.
2. **Par groupes du fournisseur d'identité** — remplissez le champ **IdP
   groups**, un groupe par ligne : un nom de groupe, un ID d'objet Entra, ou un
   DN/CN LDAP (`NOC` correspond à `cn=NOC,ou=Groups,…`). L'appartenance est
   recalculée à **chaque connexion OIDC, LDAP ou SAML**. Un membre ajouté à la
   main n'est jamais retiré par cette synchronisation.
3. **Par [SCIM](scim.md)** — le fournisseur d'identité pousse l'appartenance.

Les droits sur les dossiers sont toujours réglés par un admin dans DumbMonit ;
le fournisseur d'identité décide seulement de qui est dans l'équipe.

## Sauvegarde

Les équipes, leurs membres et leurs droits font partie de la
[sauvegarde](../install/backup.md). Aucun secret n'y figure en clair.

Voir aussi [SAML](saml.md) et [SCIM](scim.md).
