# LDAP / Active Directory

DumbMonit can check usernames and passwords against a company directory —
Active Directory, OpenLDAP, or any LDAPv3 server. People sign in on the usual
sign-in screen, with the username and password they already use: there is no
extra button. The account is created at the first sign-in and its role follows
the directory groups.

It sits next to [single sign-on](settings.md#single-sign-on): both can be on
at the same time.

## How a sign-in works

1. A username that has a **local password** in DumbMonit is checked locally
   and never sent to the directory. The local admin therefore always works,
   even when the directory is down or misconfigured: keep it as your way back
   in.
2. Any other username goes to the directory. DumbMonit binds with the
   *service account*, searches the user with the *user filter* under the
   *base DN* (exactly one entry must match), reads its groups, then **binds as
   that entry with the password that was typed**. Only the directory judges
   the password; DumbMonit never stores it.
3. The groups give the role (below). The account is created on the first
   sign-in, and its display name and role are updated at every sign-in.
4. Two-factor authentication, when the account has it, is still asked.

Whatever goes wrong — unknown user, wrong password, empty password, no allowed
group, directory unreachable — the sign-in screen only says *Wrong username or
password*. The exact reason is in the server log. The sign-in rate limit is the
same as for local accounts.

## Settings

**Settings → Team & access → LDAP / Active Directory**, admins only. The
*Active Directory* and *OpenLDAP* buttons fill the filter, attributes and group
settings with the usual values for each.

| Field | Active Directory | OpenLDAP |
|---|---|---|
| Server URL | `ldaps://dc01.corp.example:636` | `ldaps://ldap.example.org:636` |
| Service account DN | `CN=svc-dumbmonit,OU=Service,DC=corp,DC=example` | `cn=dumbmonit,ou=services,dc=example,dc=org` |
| Base DN | `DC=corp,DC=example` | `ou=people,dc=example,dc=org` |
| User filter | `(&(objectClass=user)(sAMAccountName={username}))` | `(&(objectClass=inetOrgPerson)(uid={username}))` |
| Username attribute | `sAMAccountName` | `uid` |
| Display name / email | `displayName` / `mail` | `cn` / `mail` |
| Groups | `memberOf` attribute | Group search `(&(objectClass=groupOfNames)(member={dn}))` |

- **Server URL** — `ldaps://` (port 636), or `ldap://` (port 389) with
  **StartTLS**. A plain `ldap://` connection without StartTLS would send every
  password in clear text: it is refused unless `DUMBMONIT_LDAP_ALLOW_PLAIN=1`
  is set, which is meant for a test directory only.
- **Certificate** — the server certificate is verified by default. A domain
  controller certificate is usually issued by your internal authority (AD CS):
  paste that authority's certificate (PEM) as *CA certificate*, and only it is
  trusted. Turning verification off keeps the connection encrypted but lets
  anyone on the network path pose as the directory.
- **Service account** — a read-only account used to search users and groups.
  Its password is stored encrypted with the instance secret and never shown
  again; leave the field empty to keep it. It is kept only for the same server
  URL and the same DN: change either and the password must be typed again.
  Leave the DN empty for an anonymous search.
- **User filter** — `{username}` is replaced by the typed username,
  **escaped** as RFC 4515 requires (`*`, `(`, `)`, `\` and NUL), so a username
  can never change the filter. With Active Directory, `userPrincipalName`
  works too: `(&(objectClass=user)(userPrincipalName={username}))`.
- **Timeout** — per operation, 5 seconds by default (1 to 30). A whole sign-in
  never takes more than four times that.

Settings saved here take precedence over the `DUMBMONIT_LDAP_*` environment
variables, which only apply while nothing is saved (see the
[configuration reference](../reference/configuration.md)). **Forget saved
settings** falls back to them.

## Groups and roles

*Read groups from* chooses where the groups come from:

- **memberOf attribute** — the user entry's `memberOf`. Active Directory fills
  it; so does OpenLDAP with the `memberof` overlay.
- **Group search** — a search under the *group base DN* with the *group
  filter*, where `{dn}` is the user's DN and `{username}` the username. For
  OpenLDAP `groupOfNames`: `(&(objectClass=groupOfNames)(member={dn}))`; for
  `posixGroup`: `(&(objectClass=posixGroup)(memberUid={username}))`. For nested
  Active Directory groups:
  `(&(objectClass=group)(member:1.2.840.113556.1.4.1941:={dn}))`.

*Admin groups*, *Operator groups* and *Viewer groups* take one group per line,
by name (`DumbMonit Admins`, the first value of its DN) or full DN, case
ignored. A member of an admin group is an admin, else of an operator group an
operator, else of a viewer group a viewer.

- **Once any list is filled, only members of a listed group can sign in.** An
  account whose user left all of them is **disabled** at its next sign-in (and
  its sessions closed); it is enabled again when the user is back in a listed
  group. An account an admin disabled by hand stays disabled.
- **All three lists empty**: everyone the filter finds can sign in, as a viewer
  at first; roles are then managed under **Users**. Narrow the user filter to
  restrict access, for example
  `(&(objectClass=user)(sAMAccountName={username})(memberOf=CN=Monitoring,OU=Groups,DC=corp,DC=example))`.
- With no admin list, the groups never touch the admin role. The last active
  admin is never demoted nor disabled by the directory.

## Test connection

**Test connection** checks the form as it is typed, before saving: connection
(and TLS), service account bind and, with a *test user*, the user search and
its groups, with the role that user would get. The test never asks for nor
tries a user password. Each step says what failed, for example
`service account bind refused: invalid credentials (LDAP result 49)` or
`certificate of ldaps://… rejected`.

## Accounts

Directory accounts show **LDAP** in **Settings → Users**. They have no
DumbMonit password: they change it in the directory. If an admin sets a
password on such an account, it becomes a local account that no longer asks the
directory.

The account is linked to its directory entry by `objectGUID` (Active
Directory) or `entryUUID` (OpenLDAP), so renaming or moving the user keeps the
same account. A directory user whose username is already taken by a local or
single sign-on account is refused (see the log): rename or delete that account
first. An account created in advance by an admin, without a password, is linked
at the first sign-in.

## Limits

- Group changes apply at the next sign-in. A session that is already open stays
  open until it expires or the user signs out; disable the account under
  **Users** to cut it off at once.
- Simple bind only: no Kerberos, NTLM or client certificate. Only one
  directory server URL; put several domain controllers behind one DNS name.
- Referrals are not followed: search the domain the users are in (or a global
  catalog on port 3269 with `ldaps://`).
- DumbMonit accounts have no email field: the email attribute is only used as
  the display name when the directory has none.
