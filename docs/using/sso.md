# Single sign-on (OpenID Connect and SAML)

Single sign-on lets people sign in with your identity provider instead of a
DumbMonit password. DumbMonit speaks two protocols. You can turn both on, and
[LDAP](ldap.md) as well, at the same time.

| Protocol | Pick it when | Where |
|---|---|---|
| OpenID Connect (OIDC) | Your provider is Authentik, Keycloak, Authelia, Pocket ID, Google Workspace… The simplest to set up. | **Settings → Team & access → Single sign-on** |
| SAML 2.0 | Your provider is Entra ID, Okta, ADFS… | **Settings → Team & access → SAML**: see [SAML 2.0](saml.md) |

Both are admin only.

## Before you start

- DumbMonit must be reached over its public address (for example
  `https://monitor.example.org`). Browsers and the provider use it to come back.
- You are signed in as an administrator, with a local account you keep: do not
  lock yourself out.

## OpenID Connect, step by step

1. **At your provider**, create an application (a "client") of type *web
   application* with the *authorization code* flow. Note its **client id** and
   **client secret**. Leave the redirect URI for step 4.
2. **In DumbMonit**, open **Settings → Team & access → Single sign-on**.
3. Fill in:
    - **Provider name**: the label of the sign-in button, "Continue with …".
    - **Issuer URL**: for example `https://id.example.org/application/o/dumbmonit/`.
      DumbMonit reads `{issuer}/.well-known/openid-configuration`.
    - **Client id** and **Client secret**: stored encrypted, never shown again.
    - **Public URL**: how browsers reach DumbMonit.
4. Under **Register this redirect URI at your provider**, click **Copy redirect
   URI** and add it to the application at your provider.
5. Click **Test discovery**. You should see *Discovery OK* and the endpoints
   found. If the provider signs with neither RS256 nor ES256, a warning says so:
   DumbMonit accepts only those two.
6. Click **Save changes**. You should see *Saved* and the state *On* with your
   provider's name.
7. Open the sign-in page in a private window. A **Continue with …** button is
   there. Click it, sign in at the provider, and you come back signed in.

!!! warning "Who gets an account"
    **Create accounts on first sign-in** (under **More options**) is **off by
    default**. Off, only an existing account can sign in: it is linked when the
    provider asserts a verified email equal to the account's username. A local
    administrator with a password is never linked automatically.
    Turn the option on to let anyone your provider authenticates create an account.

### Roles from groups

Under **More options**:

- **Groups claim**: the token claim that lists the user's groups.
- **Scopes**: add the scope that carries the groups claim if your provider needs one.
- **Admin groups** (comma separated): members become administrators.
- **Operator groups**: other members become operators (they handle alerts but
  change no configuration). Everyone else is a viewer.

Leave both lists empty to keep managing roles under **Users**. Roles are
re-evaluated at each sign-in, and the last active administrator is never
demoted.

### Settings from environment variables

`DUMBMONIT_OIDC_*` variables apply only while nothing is saved here. What you
save wins. **Forget saved settings** clears it and returns to the variables, if
any. Details: [Settings](settings.md#single-sign-on).

## SAML, step by step

The full guide, with Entra ID, Okta, Keycloak and Authentik examples, is
[SAML 2.0](saml.md). In short:

1. In **Settings → Team & access → SAML**, copy the *Service provider metadata
   URL* and give it to your provider (or the ACS and entity ID values listed on
   [that page](saml.md#service-provider-values)).
2. Paste the provider's metadata XML (or load the file) and click **Read
   metadata**. The fields fill in; nothing is saved yet.
3. Turn on **Enable SAML sign-in** and click **Save SAML settings**. A SAML
   button appears on the sign-in page.

## When something goes wrong

| What you see | What to do |
|---|---|
| *Discovery failed* or *The provider did not answer* | Check the issuer URL (no typo, with the trailing path some providers need) and that the DumbMonit server can reach the provider. |
| The provider says the redirect URI is not allowed | Copy the URI again from DumbMonit; it is built from the **Public URL**, which must match what browsers use. |
| Signing in works at the provider but DumbMonit refuses | Account creation is off and no account matches: create the account under **Users** with the verified email as username, or turn on account creation. |
| Everyone is a viewer | The groups claim name or scope is wrong, or the group names do not match the lists. |
| SAML fails with a generic message | The exact reason is in the [server logs](../install/logs.md). Check signatures, the clocks and the entity ID. |

If you are locked out, sign in with your local administrator, or see the
[FAQ](../faq.md#i-lost-the-password).
