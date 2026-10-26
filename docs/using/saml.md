# SAML 2.0 single sign-on

DumbMonit can act as a SAML 2.0 service provider (SP), so people sign in
through your identity provider (IdP): Microsoft Entra ID, Okta, Keycloak,
Authentik… It sits next to [OpenID Connect](settings.md#single-sign-on) and
[LDAP](ldap.md); they can all be on at the same time.

Configure it under **Settings → Team & access → SAML**, admins only.

## Service provider values

Give these to your IdP (replace `<host>` with the public address of DumbMonit):

| Value | URL |
|---|---|
| SP metadata | `https://<host>/api/auth/saml/metadata` |
| ACS (Assertion Consumer Service, HTTP-POST) | `https://<host>/api/auth/saml/acs` |
| Entity ID | by default, the metadata URL |

Most IdPs can import the metadata URL directly. Then, in DumbMonit, **import
the IdP metadata** (XML file or pasted XML): it fills the IdP entity ID, the
sign-in URL and the signing certificate.

## Security

- **Signatures are required**: on the assertion, on the whole response, or
  both. Algorithms are SHA-256 or stronger; **SHA-1 is refused**. A response
  where neither is signed is rejected.
- Audience, destination, expiration and `InResponseTo` are checked, and an
  assertion can be used only once (replay protection).
- **IdP-initiated sign-in** (starting from the IdP portal) is optional and
  **off by default**. Without it, the sign-in must start from DumbMonit.

## Attributes

DumbMonit reads the e-mail, the name and the groups from the assertion. The
defaults match the usual names:

| Data | Default attribute |
|---|---|
| E-mail | `http://schemas.xmlsoap.org/ws/2005/05/identity/claims/emailaddress`, or `email` |
| Name | `displayName` |
| Groups | `http://schemas.microsoft.com/ws/2008/06/identity/claims/groups`, or `groups` |

You can change each attribute name in the form.

## Roles and teams

- **Admin groups** and **Operator groups** give the role: a member of an
  admin group is an admin, else of an operator group an operator, else a
  viewer. Roles are re-evaluated at every sign-in.
- The same groups can fill [teams](teams.md) through their **IdP groups**
  field.

## Which account a sign-in lands on

- The account is linked by the **NameID** after the first sign-in.
- On the first sign-in, an existing **non-admin** account with the same e-mail
  is linked. An admin account is never linked automatically.
- **Create accounts on first sign-in** is optional. Otherwise only existing
  accounts can sign in.

Two-factor authentication, when the account has it, is still asked.

## Entra ID

1. In the Entra admin center, **Enterprise applications → New application →
   Create your own application**, choose *Integrate any other application*.
2. **Single sign-on → SAML → Basic SAML Configuration**: *Identifier (Entity
   ID)* = the metadata URL, *Reply URL (ACS)* = the ACS URL.
3. **Attributes & Claims → Add a group claim**: *Groups assigned to the
   application*, source attribute *Group ID* (the claim carries object IDs —
   use them in the group lists). Keep the e-mail claim
   (`user.mail`).
4. **SAML Certificates**: copy the *App Federation Metadata Url*. Make sure
   the assertion is signed (*Sign SAML assertion* or *Sign SAML response and
   assertion*) with SHA-256.
5. Assign users and groups to the application.
6. In DumbMonit, import the federation metadata and save.

## Okta

1. **Applications → Create App Integration → SAML 2.0**.
2. *Single sign-on URL* = the ACS URL, *Audience URI (SP Entity ID)* = the
   metadata URL, *Name ID format* = EmailAddress, *Application username* =
   Email.
3. *Advanced settings*: *Assertion Signature* = Signed, *Signature Algorithm*
   = RSA-SHA256.
4. **Attribute statements**: `email` = `user.email`, `displayName` =
   `user.displayName`. **Group attribute statements**: name `groups`, filter
   *Matches regex* `.*` (or a narrower one).
5. Assign the people or groups, copy the *Identity Provider metadata* link,
   and import it in DumbMonit.

## Keycloak

1. In the realm, **Clients → Create client**, type *SAML*. *Client ID* = the
   metadata URL.
2. Set *Valid redirect URIs* and *Master SAML Processing URL* to the ACS URL;
   *Name ID format* = email.
3. **Signature and Encryption**: keep *Sign documents* on and turn **Sign
   assertions** on as well (recommended), algorithm RSA_SHA256.
4. **Client scopes → dedicated scope → Add mapper**: *User Property* `email`
   → attribute `email`; *User Property* `username` or a full-name mapper →
   `displayName`; *Group list* → attribute `groups` (turn *Full group path*
   off to get plain names).
5. Import `https://<keycloak>/realms/<realm>/protocol/saml/descriptor` in
   DumbMonit.

## Authentik

1. **Applications → Providers → Create → SAML Provider**. *ACS URL* = the ACS
   URL, *Issuer* and *Audience* = the metadata URL, *Service Provider
   Binding* = Post.
2. Set a **Signing certificate** and turn on **Sign assertions** (SHA-256).
3. **Property mappings**: keep the default e-mail and name mappings, and add
   a custom SAML mapping named `groups` that returns the user's group names
   (`[g.name for g in user.ak_groups.all()]`).
4. Create an application using this provider and assign it to users or
   groups.
5. Download the provider's **metadata** and import it in DumbMonit.

## Troubleshooting

The sign-in screen gives a generic error; the exact reason (unsigned
assertion, wrong audience, expired, unknown attribute…) is in the
[server log](../install/logs.md). Check first that the assertion is signed,
that the clock is correct on both sides, and that the entity ID matches.
