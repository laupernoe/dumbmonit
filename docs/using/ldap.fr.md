# LDAP / Active Directory

DumbMonit peut vérifier identifiants et mots de passe auprès d’un annuaire
d’entreprise — Active Directory, OpenLDAP, ou tout serveur LDAPv3. On se
connecte sur l’écran de connexion habituel, avec l’identifiant et le mot de
passe qu’on utilise déjà : pas de bouton en plus. Le compte est créé à la
première connexion et son rôle suit les groupes de l’annuaire.

Il s’ajoute à l’[authentification unique](settings.md#connexion-unique) : les
deux peuvent être actifs en même temps.

## Déroulé d’une connexion

1. Un identifiant qui a un **mot de passe local** dans DumbMonit est vérifié
   localement et n’est jamais envoyé à l’annuaire. L’administrateur local
   fonctionne donc toujours, même annuaire en panne ou mal configuré : gardez-le
   comme porte de secours.
2. Tout autre identifiant part à l’annuaire. DumbMonit se lie avec le *compte
   de service*, cherche l’utilisateur avec le *filtre utilisateur* sous le *DN
   de base* (une seule entrée doit correspondre), lit ses groupes, puis **se lie
   en tant que cette entrée avec le mot de passe saisi**. Seul l’annuaire juge le
   mot de passe ; DumbMonit ne le stocke jamais.
3. Les groupes donnent le rôle (plus bas). Le compte est créé à la première
   connexion, son nom affiché et son rôle mis à jour à chaque connexion.
4. La double authentification, si le compte l’a, reste demandée.

Quoi qu’il arrive — utilisateur inconnu, mauvais mot de passe, mot de passe
vide, aucun groupe autorisé, annuaire injoignable — l’écran de connexion dit
seulement *identifiant ou mot de passe incorrect*. La raison exacte est dans le
journal du serveur. La limitation des tentatives est la même que pour les
comptes locaux.

## Réglages

**Paramètres → Équipe et accès → LDAP / Active Directory**, administrateurs
seulement. Les boutons *Active Directory* et *OpenLDAP* remplissent le filtre,
les attributs et les groupes avec les valeurs habituelles de chacun.

| Champ | Active Directory | OpenLDAP |
|---|---|---|
| URL du serveur | `ldaps://dc01.corp.example:636` | `ldaps://ldap.example.org:636` |
| DN du compte de service | `CN=svc-dumbmonit,OU=Service,DC=corp,DC=example` | `cn=dumbmonit,ou=services,dc=example,dc=org` |
| DN de base | `DC=corp,DC=example` | `ou=people,dc=example,dc=org` |
| Filtre utilisateur | `(&(objectClass=user)(sAMAccountName={username}))` | `(&(objectClass=inetOrgPerson)(uid={username}))` |
| Attribut d’identifiant | `sAMAccountName` | `uid` |
| Nom affiché / courriel | `displayName` / `mail` | `cn` / `mail` |
| Groupes | Attribut `memberOf` | Recherche `(&(objectClass=groupOfNames)(member={dn}))` |

- **URL du serveur** — `ldaps://` (port 636), ou `ldap://` (port 389) avec
  **StartTLS**. Une connexion `ldap://` sans StartTLS enverrait chaque mot de
  passe en clair : elle est refusée sauf avec `DUMBMONIT_LDAP_ALLOW_PLAIN=1`,
  réservé à un annuaire de test.
- **Certificat** — le certificat du serveur est vérifié par défaut. Celui d’un
  contrôleur de domaine est en général émis par votre autorité interne
  (AD CS) : collez le certificat de cette autorité (PEM) dans *Certificat de
  l’autorité*, et elle seule est crue. Couper la vérification garde la
  connexion chiffrée, mais laisse n’importe qui sur le chemin réseau se faire
  passer pour l’annuaire.
- **Compte de service** — un compte en lecture seule pour chercher
  utilisateurs et groupes. Son mot de passe est chiffré avec le secret
  d’instance et jamais réaffiché ; laissez le champ vide pour le conserver. Il
  n’est conservé que pour la même URL et le même DN : changer l’un ou l’autre
  oblige à le retaper. Un DN vide fait une recherche anonyme.
- **Filtre utilisateur** — `{username}` est remplacé par l’identifiant saisi,
  **échappé** comme l’exige la RFC 4515 (`*`, `(`, `)`, `\` et NUL) : un
  identifiant ne peut jamais modifier le filtre. Avec Active Directory,
  `userPrincipalName` marche aussi :
  `(&(objectClass=user)(userPrincipalName={username}))`.
- **Délai** — par opération, 5 secondes par défaut (1 à 30). Une connexion
  entière ne dépasse jamais quatre fois ce délai.

Les réglages enregistrés ici l’emportent sur les variables d’environnement
`DUMBMONIT_LDAP_*`, qui ne servent que tant que rien n’est enregistré (voir la
[référence de configuration](../reference/configuration.md)). **Oublier les
réglages enregistrés** y revient.

## Groupes et rôles

*Lire les groupes depuis* choisit leur provenance :

- **Attribut memberOf** — le `memberOf` de l’entrée. Active Directory le
  remplit, OpenLDAP aussi avec l’overlay `memberof`.
- **Recherche de groupes** — une recherche sous le *DN de base des groupes*
  avec le *filtre de groupes*, où `{dn}` est le DN de l’utilisateur et
  `{username}` son identifiant. Pour des `groupOfNames` OpenLDAP :
  `(&(objectClass=groupOfNames)(member={dn}))` ; pour des `posixGroup` :
  `(&(objectClass=posixGroup)(memberUid={username}))`. Pour les groupes
  imbriqués d’Active Directory :
  `(&(objectClass=group)(member:1.2.840.113556.1.4.1941:={dn}))`.

*Groupes administrateurs*, *opérateurs* et *lecteurs* prennent un groupe par
ligne, par nom (`DumbMonit Admins`, la première valeur de son DN) ou DN
complet, sans tenir compte de la casse. Membre d’un groupe administrateur :
admin ; sinon d’un groupe opérateur : opérateur ; sinon d’un groupe lecteur :
lecteur.

- **Dès qu’une liste est remplie, seuls les membres d’un groupe listé
  entrent.** Un compte dont l’utilisateur les a tous quittés est **désactivé**
  à sa connexion suivante (et ses sessions fermées) ; il est réactivé quand
  l’utilisateur revient dans un groupe listé. Un compte désactivé à la main par
  un administrateur le reste.
- **Les trois listes vides** : quiconque est trouvé par le filtre entre,
  d’abord comme lecteur ; les rôles se règlent ensuite dans **Utilisateurs**.
  Pour restreindre l’accès, resserrez le filtre, par exemple
  `(&(objectClass=user)(sAMAccountName={username})(memberOf=CN=Supervision,OU=Groups,DC=corp,DC=example))`.
- Sans liste d’administrateurs, les groupes ne touchent jamais au rôle admin.
  Le dernier administrateur actif n’est jamais rétrogradé ni désactivé par
  l’annuaire.

## Tester la connexion

**Tester la connexion** vérifie le formulaire tel qu’il est saisi, avant
d’enregistrer : connexion (et TLS), liaison du compte de service et, avec un
*utilisateur de test*, la recherche de l’utilisateur et de ses groupes, avec le
rôle qu’il obtiendrait. Le test ne demande ni n’essaie jamais de mot de passe
d’utilisateur. Chaque étape dit ce qui a échoué, par exemple
`service account bind refused: invalid credentials (LDAP result 49)` ou
`certificate of ldaps://… rejected`.

## Comptes

Les comptes d’annuaire portent **LDAP** dans **Paramètres → Utilisateurs**. Ils
n’ont pas de mot de passe DumbMonit : il se change dans l’annuaire. Si un
administrateur leur donne un mot de passe, ils deviennent des comptes locaux
qui ne consultent plus l’annuaire.

Le compte est rattaché à son entrée par `objectGUID` (Active Directory) ou
`entryUUID` (OpenLDAP) : renommer ou déplacer l’utilisateur garde le même
compte. Un utilisateur d’annuaire dont l’identifiant est déjà pris par un
compte local ou SSO est refusé (voir le journal) : renommez ou supprimez
d’abord ce compte. Un compte créé d’avance par un administrateur, sans mot de
passe, est rattaché à la première connexion.

## Limites

- Un changement de groupes s’applique à la connexion suivante. Une session
  déjà ouverte le reste jusqu’à son expiration ou la déconnexion ; désactivez
  le compte dans **Utilisateurs** pour la couper tout de suite.
- Liaison simple seulement : ni Kerberos, ni NTLM, ni certificat client. Une
  seule URL de serveur ; placez plusieurs contrôleurs derrière un même nom DNS.
- Les référents (referrals) ne sont pas suivis : cherchez dans le domaine des
  utilisateurs (ou dans un catalogue global, port 3269 en `ldaps://`).
- Les comptes DumbMonit n’ont pas de champ courriel : l’attribut de courriel
  sert seulement de nom affiché quand l’annuaire n’en donne pas.
