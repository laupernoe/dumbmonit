# Authentification unique SAML 2.0

DumbMonit peut jouer le rôle de fournisseur de service (SP) SAML 2.0 : on se
connecte alors via votre fournisseur d'identité (IdP) — Microsoft Entra ID,
Okta, Keycloak, Authentik… Il s'ajoute à
[OpenID Connect](settings.md#connexion-unique) et à [LDAP](ldap.md) ; tous
peuvent être actifs en même temps.

Réglage dans **Settings → Team & access → SAML**, réservé aux administrateurs.

## Valeurs du fournisseur de service

À donner à votre IdP (remplacez `<host>` par l'adresse publique de DumbMonit) :

| Valeur | URL |
|---|---|
| Métadonnées SP | `https://<host>/api/auth/saml/metadata` |
| ACS (Assertion Consumer Service, HTTP-POST) | `https://<host>/api/auth/saml/acs` |
| Entity ID | par défaut, l'URL des métadonnées |

La plupart des IdP importent directement l'URL des métadonnées. Dans
DumbMonit, **importez ensuite les métadonnées de l'IdP** (fichier XML ou XML
collé) : elles remplissent l'entity ID de l'IdP, l'URL de connexion et le
certificat de signature.

## Sécurité

- Les **assertions signées sont obligatoires**. Algorithmes SHA-256 ou plus ;
  **SHA-1 est refusé**. Une assertion non signée est rejetée.
- L'audience, la destination, l'expiration et `InResponseTo` sont vérifiées,
  et une assertion ne sert qu'une fois (anti-rejeu).
- La **connexion initiée par l'IdP** (depuis le portail de l'IdP) est
  optionnelle et **désactivée par défaut**. Sans elle, la connexion doit
  partir de DumbMonit.

## Attributs

DumbMonit lit l'e-mail, le nom et les groupes dans l'assertion. Les valeurs
par défaut correspondent aux noms usuels :

| Donnée | Attribut par défaut |
|---|---|
| E-mail | `http://schemas.xmlsoap.org/ws/2005/05/identity/claims/emailaddress`, ou `email` |
| Nom | `displayName` |
| Groupes | `http://schemas.microsoft.com/ws/2008/06/identity/claims/groups`, ou `groups` |

Chaque nom d'attribut est modifiable dans le formulaire.

## Rôles et équipes

- **Admin groups** et **Operator groups** donnent le rôle : membre d'un groupe
  admin, administrateur ; sinon d'un groupe opérateur, opérateur ; sinon
  lecteur. Les rôles sont réévalués à chaque connexion.
- Les mêmes groupes peuvent alimenter les [équipes](teams.md) via leur champ
  **IdP groups**.

## Sur quel compte on arrive

- Le compte est lié par le **NameID** après la première connexion.
- À la première connexion, un compte **non admin** existant ayant le même
  e-mail est rattaché. Un compte admin n'est jamais rattaché automatiquement.
- **Create accounts on first sign-in** est optionnel. Sinon, seuls les comptes
  existants peuvent se connecter.

L'authentification à deux facteurs, si le compte en a une, est toujours
demandée.

## Entra ID

1. Dans le centre d'administration Entra, **Applications d'entreprise →
   Nouvelle application → Créer votre propre application**, choisissez
   *Intégrer une autre application*.
2. **Authentification unique → SAML → Configuration SAML de base** :
   *Identificateur (ID d'entité)* = l'URL des métadonnées, *URL de réponse
   (ACS)* = l'URL ACS.
3. **Attributs et revendications → Ajouter une revendication de groupe** :
   *Groupes affectés à l'application*, attribut source *ID de groupe* (la
   revendication porte des ID d'objet — utilisez-les dans les listes de
   groupes). Gardez la revendication e-mail (`user.mail`).
4. **Certificats SAML** : copiez l'*URL des métadonnées de fédération
   d'application*. Vérifiez que l'assertion est signée (*Signer l'assertion
   SAML* ou *Signer la réponse et l'assertion SAML*) en SHA-256.
5. Affectez les utilisateurs et les groupes à l'application.
6. Dans DumbMonit, importez les métadonnées de fédération et enregistrez.

## Okta

1. **Applications → Create App Integration → SAML 2.0**.
2. *Single sign-on URL* = l'URL ACS, *Audience URI (SP Entity ID)* = l'URL des
   métadonnées, *Name ID format* = EmailAddress, *Application username* =
   Email.
3. *Advanced settings* : *Assertion Signature* = Signed, *Signature Algorithm*
   = RSA-SHA256.
4. **Attribute statements** : `email` = `user.email`, `displayName` =
   `user.displayName`. **Group attribute statements** : nom `groups`, filtre
   *Matches regex* `.*` (ou plus restrictif).
5. Affectez personnes ou groupes, copiez le lien *Identity Provider metadata*
   et importez-le dans DumbMonit.

## Keycloak

1. Dans le realm, **Clients → Create client**, type *SAML*. *Client ID* =
   l'URL des métadonnées.
2. Renseignez *Valid redirect URIs* et *Master SAML Processing URL* avec l'URL
   ACS ; *Name ID format* = email.
3. **Signature and Encryption** : **Sign assertions = ON**, algorithme
   RSA_SHA256.
4. **Client scopes → dedicated scope → Add mapper** : *User Property* `email`
   → attribut `email` ; *User Property* `username` ou un mapper de nom complet
   → `displayName` ; *Group list* → attribut `groups` (désactivez *Full group
   path* pour obtenir des noms simples).
5. Importez `https://<keycloak>/realms/<realm>/protocol/saml/descriptor` dans
   DumbMonit.

## Authentik

1. **Applications → Providers → Create → SAML Provider**. *ACS URL* = l'URL
   ACS, *Issuer* et *Audience* = l'URL des métadonnées, *Service Provider
   Binding* = Post.
2. Choisissez un **Signing certificate** et activez **Sign assertions**
   (SHA-256).
3. **Property mappings** : gardez les mappings e-mail et nom par défaut, et
   ajoutez un mapping SAML personnalisé nommé `groups` qui renvoie les noms de
   groupes de l'utilisateur (`[g.name for g in user.ak_groups.all()]`).
4. Créez une application utilisant ce fournisseur et affectez-la à des
   utilisateurs ou groupes.
5. Téléchargez les **métadonnées** du fournisseur et importez-les dans
   DumbMonit.

## Dépannage

L'écran de connexion affiche une erreur générique ; la raison exacte
(assertion non signée, mauvaise audience, expirée, attribut introuvable…) est
dans le [journal du serveur](../install/logs.md). Vérifiez d'abord que
l'assertion est signée, que l'horloge est juste des deux côtés et que
l'entity ID correspond.
