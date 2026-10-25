# Homepage and Homarr

Show the state of DumbMonit on your dashboard: how many monitors are up or down,
how many alerts are firing. `GET /api/summary` returns flat counters meant for
that.

```json
{
  "status": "ok",
  "sentence": "Clear skies.",
  "devices": { "total": 24, "up": 23, "down": 1, "waiting": 0, "disabled": 0 },
  "alerts": { "firing": 2, "critical": 1, "warning": 1, "info": 0, "building_up": 0, "suppressed": 0 }
}
```

`status` is `critical` when an alert is critical or a monitor is unreachable,
`warning` for any other firing alert, `ok` otherwise. The figures are the ones the
interface and the [assistant](../using/assistant.md) show.

## 1. Create a read-only token

**Settings → API tokens → New token**, scope **read**. The token can read and
nothing else (it cannot acknowledge an alert, change a monitor or create another
token). Copy it: it is shown once. You can also limit it to your dashboard's
network address and give it an expiry date.

## 2. Homepage

[Homepage](https://gethomepage.dev) reads JSON with its `customapi` widget. In
`services.yaml`:

```yaml
- Monitoring:
    - DumbMonit:
        icon: mdi-radar
        href: https://monit.example.org
        description: Homelab status
        widget:
          type: customapi
          url: https://monit.example.org/api/summary
          refreshInterval: 30000
          headers:
            Authorization: Bearer dmt_xxxxxxxxxxxxxxxx
          mappings:
            - field: devices.up
              label: Up
              format: number
            - field: devices.down
              label: Down
              format: number
            - field: alerts.firing
              label: Alerts
              format: number
```

`field` is a dotted path into the JSON. To show the verdict as text, map
`status` (`ok`, `warning`, `critical`) and, if you like, colour it with
`remap`:

```yaml
            - field: status
              label: State
              remap:
                - value: ok
                  to: All good
                - value: warning
                  to: Attention
                - value: critical
                  to: Down
```

Put the token in a Homepage secret rather than in the file
(`{{HOMEPAGE_VAR_DUMBMONIT_TOKEN}}` with the environment variable
`HOMEPAGE_VAR_DUMBMONIT_TOKEN`), since `services.yaml` is often versioned.

!!! note
    An official Homepage widget (a dedicated `dumbmonit` type) does not exist
    today: `customapi` does the job without waiting for one.

## 3. Homarr

Homarr has no generic "read this JSON" widget as of this writing, so there are
two sound ways:

- **Status page in an iframe.** Publish a [status page](../using/status-pages.md)
  and add Homarr's *iFrame* widget pointing to
  `https://monit.example.org/s/<slug>/embed`: nothing to authenticate, the
  page is made for embedding.
- **App tile.** Add DumbMonit as an *App* with its address; Homarr's ping shows
  whether the instance answers.

`/api/summary` stays available for anything that can send an `Authorization:
Bearer` header (a Homarr custom integration, n8n, a script, a status bar…).

```bash
curl -s -H "Authorization: Bearer $TOKEN" https://monit.example.org/api/summary
```

## Troubleshooting

| Symptom | Cause |
|---|---|
| `401` | Token missing, mistyped, revoked or expired; or the request does not come from the network the token is limited to. |
| `403` | The token is not the right kind (an agent token does not work here): create an API token. |
| Numbers do not match the page | The counters are computed on the server when you ask; `refreshInterval` sets how often your dashboard asks. |
