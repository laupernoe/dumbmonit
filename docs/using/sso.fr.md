# Connexion unique (OpenID Connect et SAML)

La connexion unique permet de se connecter avec votre fournisseur d'identité
plutôt qu'avec un mot de passe DumbMonit. DumbMonit parle deux protocoles. Vous
pouvez activer les deux, et [LDAP](ldap.md) aussi, en même temps.

| Protocole | À choisir quand | Où |
|---|---|---|
| OpenID Connect (OIDC) | Votre fournisseur est Authentik, Keycloak, Authelia, Pocket ID, Google Workspace… Le plus simple à mettre en place. | **Réglages → Équipe et accès → Authentification unique** |
| SAML 2.0 | Votre fournisseur est Entra ID, Okta, ADFS… | **Réglages → Équipe et accès → SAML** : voir [SAML 2.0](saml.md) |

Les deux sont réservés aux administrateurs.

## Avant de commencer

- DumbMonit doit être joint par son adresse publique (par exemple
  `https://monitor.example.org`). Les navigateurs et le fournisseur s'en servent
  pour revenir.
- Vous êtes connecté en administrateur, avec un compte local que vous gardez :
  ne vous enfermez pas dehors.

## OpenID Connect, pas à pas

1. **Chez votre fournisseur**, créez une application (un « client ») de type
   *application web* avec le flux *code d'autorisation*. Notez son **ID client**
   et son **secret client**. Laissez l'URI de redirection pour l'étape 4.
2. **Dans DumbMonit**, ouvrez **Réglages → Équipe et accès → Authentification
   unique**.
3. Remplissez :
    - **Nom du fournisseur** : le libellé du bouton, « Continuer avec … ».
    - **URL de l'émetteur** : par exemple
      `https://id.example.org/application/o/dumbmonit/`. DumbMonit lit
      `{émetteur}/.well-known/openid-configuration`.
    - **ID client** et **Secret client** : stockés chiffrés, jamais réaffichés.
    - **URL publique** : comment les navigateurs joignent DumbMonit.
4. Sous **Enregistrez cet URI de redirection chez votre fournisseur**, cliquez
   sur le bouton de copie de l'URI de redirection et ajoutez-le à l'application
   chez le fournisseur.
5. Cliquez sur **Tester la découverte**. Vous devez voir un message de réussite
   et les points d'accès trouvés. Si le fournisseur ne signe ni en RS256 ni en
   ES256, un avertissement le dit : DumbMonit n'accepte que ces deux signatures.
6. Cliquez sur **Enregistrer les modifications**. Vous devez voir *Enregistré* et
   l'état *Activé* avec le nom de votre fournisseur.
7. Ouvrez la page de connexion dans une fenêtre privée. Un bouton **Continuer
   avec …** est là. Cliquez, connectez-vous chez le fournisseur, et vous revenez
   connecté.

!!! warning "Qui obtient un compte"
    **Créer les comptes à la première connexion** (sous **Plus d'options**) est
    **désactivé par défaut**. Désactivé, seul un compte existant peut se
    connecter : il est lié quand le fournisseur atteste un email vérifié égal au
    nom d'utilisateur du compte. Un administrateur local avec mot de passe n'est
    jamais lié automatiquement. Activez l'option pour laisser toute personne
    authentifiée par votre fournisseur créer un compte.

### Rôles depuis les groupes

Sous **Plus d'options** :

- **Claim de groupes** : le claim du jeton qui liste les groupes de l'utilisateur.
- **Scopes** : ajoutez le scope qui porte le claim de groupes si votre
  fournisseur en demande un.
- **Groupes admin** (séparés par des virgules) : les membres deviennent
  administrateurs.
- **Groupes opérateur** : les autres membres deviennent opérateurs (ils traitent
  les alertes sans changer la configuration). Tous les autres sont lecteurs.

Laissez les deux listes vides pour gérer les rôles sous **Utilisateurs**. Les
rôles sont réévalués à chaque connexion, et le dernier administrateur actif
n'est jamais rétrogradé.

### Réglages par variables d'environnement

Les variables `DUMBMONIT_OIDC_*` ne s'appliquent que tant que rien n'est
enregistré ici. Ce que vous enregistrez l'emporte. **Oublier les réglages
enregistrés** les efface et revient aux variables, s'il y en a. Détails :
[Réglages](settings.md).

## SAML, pas à pas

Le guide complet, avec des exemples Entra ID, Okta, Keycloak et Authentik, est
[SAML 2.0](saml.md). En bref :

1. Dans **Réglages → Équipe et accès → SAML**, copiez l'URL des métadonnées du
   fournisseur de service et donnez-la à votre fournisseur (ou les valeurs ACS
   et identifiant d'entité listées sur [cette page](saml.md)).
2. Collez le XML de métadonnées du fournisseur (ou chargez le fichier) et
   lancez la lecture des métadonnées. Les champs se remplissent ; rien n'est
   encore enregistré.
3. Activez la connexion SAML et enregistrez les réglages. Un bouton SAML
   apparaît sur la page de connexion.

## Quand ça ne marche pas

| Ce que vous voyez | Que faire |
|---|---|
| Découverte échouée, ou le fournisseur ne répond pas | Vérifiez l'URL de l'émetteur (sans faute, avec le chemin final que certains fournisseurs exigent) et que le serveur DumbMonit joint le fournisseur. |
| Le fournisseur dit que l'URI de redirection n'est pas autorisé | Recopiez l'URI depuis DumbMonit ; il est construit depuis l'**URL publique**, qui doit correspondre à celle des navigateurs. |
| La connexion marche chez le fournisseur mais DumbMonit refuse | La création de compte est désactivée et aucun compte ne correspond : créez le compte sous **Utilisateurs** avec l'email vérifié comme nom d'utilisateur, ou activez la création. |
| Tout le monde est lecteur | Le nom du claim de groupes ou le scope est faux, ou les noms de groupes ne correspondent pas aux listes. |
| SAML échoue avec un message générique | La raison exacte est dans les [journaux du serveur](../install/logs.md). Vérifiez signatures, horloges et identifiant d'entité. |

Si vous êtes enfermé dehors, connectez-vous avec votre administrateur local, ou
voyez la [FAQ](../faq.md#i-lost-the-password).
