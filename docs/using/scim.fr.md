# Provisionnement SCIM 2.0

SCIM permet à votre fournisseur d'identité de créer, mettre à jour et
désactiver lui-même les comptes DumbMonit, et de tenir à jour l'appartenance
aux [équipes](teams.md). Les comptes ainsi provisionnés se connectent par
[SAML](saml.md) ou [OpenID Connect](settings.md#connexion-unique).

## Mise en place

Dans **Settings → Team & access → SCIM**, réservé aux administrateurs :

1. Activez SCIM et générez le **jeton secret**. Il n'est **affiché qu'une
   fois** : copiez-le tout de suite. **Rotate** en émet un nouveau (l'ancien
   cesse de fonctionner).
2. Choisissez le **rôle par défaut** des comptes créés : viewer ou operator.
   SCIM **ne crée jamais d'administrateur**.
3. Donnez à votre IdP l'URL de base `https://<host>/api/scim/v2` et le jeton
   (envoyé en `Authorization: Bearer <jeton>`).

## Ce qui est géré

- **Users** : création, mise à jour, **désactivation** (`active=false`) et
  suppression.
- **Groups** : un groupe SCIM est une équipe, et ses membres sont ceux de
  l'équipe. Les **droits par dossier** de l'équipe restent réglés par un admin
  dans DumbMonit.

**SCIM ne touche jamais un administrateur** : les comptes admin sont ignorés,
quoi que l'IdP envoie.

## Entra ID

1. Dans l'application d'entreprise de DumbMonit, ouvrez **Provisionnement** et
   passez le mode sur *Automatique*.
2. *URL du locataire* = `https://<host>/api/scim/v2`, *Jeton secret* = le
   jeton. Cliquez sur **Tester la connexion**.
3. Réglez les mappages d'attributs des utilisateurs et des groupes, la portée
   (utilisateurs et groupes affectés) et activez le provisionnement.

## Okta

1. Dans l'application SAML de DumbMonit, **General → App Settings →
   Provisioning : SCIM**.
2. **Provisioning → Integration** : *SCIM connector base URL* =
   `https://<host>/api/scim/v2`, *Unique identifier field* = `userName`,
   *Authentication Mode* = HTTP Header, avec le jeton en Bearer. Activez *Push
   New Users*, *Push Profile Updates* et *Push Groups*.
3. **Push Groups** : choisissez les groupes à pousser comme équipes. Lancez
   d'abord **Test Connector Configuration**.
