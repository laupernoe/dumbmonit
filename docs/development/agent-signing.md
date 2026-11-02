# Signing the agent binaries

Agents [update themselves](../devices/agent.md#automatic-updates) from the
server, and only run a new binary whose Ed25519 signature verifies against a
release public key compiled into the agent. This page is for maintainers: how
the key is created, where it lives, and what the release workflow does with it.

## Until the key exists

Without the key, release builds are unsigned and agents refuse every update.
**Settings › Monitoring › Agents › Agent updates** then says *Updates blocked:
unsigned builds* and holds the offers (status *Blocked: unsigned build*) instead
of letting each agent refuse them. Agents can still be updated by running the
install command again, or, for development only, by turning on *Accept
unsigned builds*.

## Create the key

On a trusted machine, outside the repository:

```sh
scripts/agent-signing-key.sh            # writes ~/.dumbmonit-agent-signing/
```

The script creates the Ed25519 key with OpenSSL (private key `0600` in a `0700`
folder), checks the pair with a test signature made exactly as `release.yml`
makes it, prints the public PEM block and the `gh secret set` command, and
touches nothing else: neither the repository nor any GitHub secret. It refuses
to overwrite an existing key. By hand, the same key is:

```sh
openssl genpkey -algorithm ed25519 -out agent-signing.pem
openssl pkey -in agent-signing.pem -pubout
```

1. Add the **public** PEM block printed by the second command to
   `crates/agent/signing/release-keys.pem` and commit it. Several keys may be
   listed.
2. Store the **private** key as a repository secret (not an environment
   secret):

    ```sh
    gh secret set AGENT_SIGNING_KEY < agent-signing.pem
    ```

3. Keep an offline backup of `agent-signing.pem`, then delete the local copy.

## What the release workflow does

On every push to `dev` and every tag, `release.yml`, after extracting the agent
binaries:

- signs each one with `openssl pkeyutl -sign -rawin` (a detached signature of 64
  raw bytes, `<binary>.sig`);
- checks that the public half of the key is listed in `release-keys.pem`, and
  fails otherwise;
- verifies every signature;
- attaches the `.sig` files to the workflow artifact and to the GitHub release;
- passes them to the server image build as the `agent-signatures` build
  context, which adds them as one extra layer.

Without the secret, the workflow warns and ships unsigned builds. Agents refuse
those unless *Accept unsigned builds (development)* is on (see
[Security](../devices/agent.md#automatic-updates-security)).

The signatures are separate files so that the binaries stay
[reproducible](reproducible-builds.md); the `.sig` layer is the only difference
with the published image.

## Rotation

1. Add the new public key to `release-keys.pem` (keep the old one) and release:
   agents now trust both.
2. Switch the `AGENT_SIGNING_KEY` secret to the new private key.
3. One release later, remove the old public key.

## If the key leaks

Remove its public key from `release-keys.pem` and release with a new key.
Agents already deployed trust what they were compiled with: a leaked key stays
trusted by them until they update past it. That is why the secret is only usable
by `release.yml` on `dev` and tags.

Trusting a signed `:edge` build is therefore the same as trusting the branch
protections of `dev`: whoever can push there can get a build signed.
