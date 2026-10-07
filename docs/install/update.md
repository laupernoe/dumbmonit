# Update DumbMonit

DumbMonit tells you when a newer release exists and walks you through the
update. It never installs anything by itself.

## Spotting a new version

Once a day at most, the server asks GitHub's public releases API
(`api.github.com/repos/laupernoe/dumbmonit/releases`) for the latest release
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
    Nothing about your instance, your devices or your network is sent, and the
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
Compose file (for example `ghcr.io/laupernoe/dumbmonit:0.1.0-alpha.7`) and run
the same command.

To go back, restore the backup taken in step 1 on the previous image.
