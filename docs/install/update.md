# Update DumbMonit

DumbMonit tells you when a newer release exists and walks you through the
update. It never installs anything by itself.

## Spotting a new version

Once a day at most, the server asks GitHub's public releases API
(`api.github.com/repos/selfhostr/dumbmonit/releases`) for the latest release
and compares it with the version you run. Pre-releases such as
`0.1.0-alpha.7` are compared properly: `alpha.10` is newer than `alpha.9`, and
`0.1.0` is newer than any `0.1.0-alpha.N`. Drafts are ignored, and an instance
on a stable version is never offered a pre-release.

When a newer version exists, **Update available: vX** appears at the bottom
right of every page and in **Settings → About**, where **How to update** opens
the steps below and **See what's new** reopens the *What's new* window.
**Check now** looks again immediately (once a minute at most).

!!! note "What is sent"
    One anonymous `GET` request with a `DumbMonit/<version>` user agent.
    Nothing about your instance, your monitors or your network is sent, and the
    address is fixed in the code.

### Turning the check off

For air-gapped networks, switch off **Check for new versions** in
**Settings → About** (administrators), or set the environment variable, which
wins over the switch:

```yaml
environment:
  DUMBMONIT_UPDATE_CHECK: "off"
```

The same information is available to scripts and API tokens at
`GET /api/update`.

## Updating

1. **Back up.** Download a backup from **Settings → Backup**, and keep a copy
   of `/data/secret.key` (see [Backup and restore](backup.md)). Without that
   key, stored passwords and tokens cannot be decrypted.
2. **Pull and restart**, from the folder that holds your `docker-compose.yml`:

    ```bash
    docker compose pull && docker compose up -d
    ```

    Your data lives in the `/data` volume and is kept; database migrations run
    on start.
3. **Reload the page.** **Settings → About** shows the new version and
   *You are up to date*.

To pin a version instead of following `:latest`, change the image tag in the
Compose file (for example `ghcr.io/selfhostr/dumbmonit:0.1.0-alpha.7`) and run
the same command.

To go back, restore the backup taken in step 1 on the previous image.

## Configuration changes after an update

`docker compose pull && docker compose up -d` replaces the image, but your
`docker-compose.yml` is yours: DumbMonit never edits it. Some versions need a
change there, such as a port to publish or an environment variable to add.

When an update asks for one, the first time an administrator opens DumbMonit
afterwards, a window lists what to change, each with the block to copy:

- **Done** closes the change. When DumbMonit can check it (for example that the
  agent port answers from the server), it checks first and asks for a second
  click if the check still fails, because a NAT loop-back or a reverse proxy can
  hide a port that does work.
- **I'll do it later** hides the window. A reminder stays in the corner of every
  page until you dismiss it, and dismissing is final: a closed change never
  comes back for its version.
- A change that a check finds missing stays listed even if you updated long ago,
  so an instance that skipped a step in an earlier release is caught up.

**Settings → About → Configuration changes → View the list** shows every change
a release has asked for (to apply, closed, or not needed on this instance) with
its block, whenever you want to read it again. Only administrators see it, and
it is switched off in the public demo.

| Since | Change | What to do |
|---|---|---|
| 0.14.0 | Encrypted agent channel | Publish port 8443: add `"8443:8443"` to the `ports` of the `dumbmonit` service (see [Agent channel security](agent-security.md)). |

A new installation has nothing to change. The instance records the version it
runs and the one it came from when it starts, and the closed changes are part of
the [backup](backup.md). Scripts can read the list at `GET /api/upgrade-notes`
(signed-in administrator, not available to API tokens).
