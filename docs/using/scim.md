# SCIM 2.0 provisioning

SCIM lets your identity provider create, update and disable DumbMonit
accounts by itself, and keep [team](teams.md) membership up to date. Accounts
provisioned this way sign in through [SAML](saml.md) or
[OpenID Connect](settings.md#single-sign-on).

## Set up

Under **Settings → Team & access → SCIM**, admins only:

1. Turn SCIM on and generate the **secret token**. It is **shown once**: copy
   it now. **Rotate** it to issue a new one (the old one stops working).
2. Pick the **default role** of created accounts: viewer or operator. SCIM
   **never creates an admin**.
3. Give your IdP the base URL `https://<host>/api/scim/v2` and the token
   (sent as `Authorization: Bearer <token>`).

## What is supported

- **Users**: create, update, **disable** (`active=false`) and delete.
- **Groups**: a SCIM group is a team, and the group members are its members.
  The **folder permissions** of the team stay set by an admin in DumbMonit.

**SCIM never touches an administrator**: admin accounts are skipped, whatever
the IdP sends.

## Entra ID

1. In the Enterprise application of DumbMonit, open **Provisioning** and set
   the mode to *Automatic*.
2. *Tenant URL* = `https://<host>/api/scim/v2`, *Secret Token* = the token.
   Click **Test Connection**.
3. Choose the attribute mappings for users and groups, set the scope (assigned
   users and groups) and turn provisioning **On**.

## Okta

1. In the DumbMonit SAML app, **General → App Settings → Provisioning: SCIM**.
2. **Provisioning → Integration**: *SCIM connector base URL* =
   `https://<host>/api/scim/v2`, *Unique identifier field* = `userName`,
   *Authentication Mode* = HTTP Header, with the token as the Bearer token.
   Enable *Push New Users*, *Push Profile Updates* and *Push Groups*.
3. **Push Groups**: choose the groups to push as teams. Click **Test
   Connector Configuration** first.
