# Home Assistant

The smart home behind the dashboard: whether Home Assistant runs or started in
recovery mode, entities that are unavailable or unknown by domain, low
batteries, updates waiting (core, operating system, add-ons, device firmware)
and the open repairs.

DumbMonit reads Home Assistant's REST API with the long-lived access token of a
dedicated user **without administrator rights**, and the list of repairs over
the WebSocket API. It never calls a service, never fires an event and never
changes a state.

The failure worth watching is not "Home Assistant does not answer" (a
[website check](services.md#http) already sees that) but **Home Assistant
answers and something behind it stopped working**: an integration whose
entities all turned unavailable after an update, a Zigbee stick that fell off
the USB bus, a door sensor whose battery reached 5%, a repair waiting for
weeks.

## What it watches

All metrics are prefixed `dumbmonit_homeassistant_`.

| Metric | What | Labels |
|---|---|---|
| `info` | value 1 | `version` |
| `running` | 1 when the core state is `RUNNING`, 0 while it starts or stops | |
| `recovery_mode` | 1 in recovery mode (or safe mode on older versions): the configuration could not be loaded | |
| `entities` | entities by domain | `domain` |
| `entities_unavailable`, `entities_unknown` | entities in that state, by domain | `domain` |
| `entity_unavailable` | value 1 per unavailable entity, at most 50 | `entity`, `name`, `domain` |
| `batteries` | battery sensors and battery binary sensors | |
| `batteries_low` | batteries below the threshold, and battery binary sensors that are on | |
| `battery_low` | one series per low battery, value = its level in percent (0 for a binary sensor), at most 50 | `entity`, `name` |
| `updates_available` | `update.*` entities that are on | |
| `update_available` | value 1 per update waiting, at most 50 | `entity`, `name`, `installed`, `latest` |
| `repairs` | open repairs (neither ignored nor dismissed) by severity: `critical`, `error`, `warning` | `severity` |
| `repair` | value 1 per open repair, at most 50 | `issue`, `domain`, `severity` |
| `scrape_errors` | calls that failed on the last probe | |

Some domains rest in the `unknown` state by nature: a button never pressed, an
event never received, a notification or speech service. They are not counted
as unknown: `button`, `input_button`, `event`, `scene`, `notify`, `tts`,
`stt`, `conversation`, `wake_word`, `ai_task`, `person`. The **Domains left
out** option removes further domains (for example `device_tracker` or
`media_player` for devices that are often off) from the unavailable and
unknown counts.

Repairs are not in the REST API: DumbMonit opens the WebSocket API, sends
`repairs/list_issues` and closes it. A user without administrator rights may
list them. The **Read the repairs** option turns this off.

The [built-in rules](../alerting/rules.md#home-assistant) that apply: Home
Assistant in recovery mode, Home Assistant not running, Home Assistant entities
went unavailable, Home Assistant battery low, Home Assistant update available,
Home Assistant repair to address, plus Device unreachable.

## The device page

The panel above the charts reads what the probe stored; opening the page never
queries Home Assistant. It says what is wrong first (recovery mode, repairs of
error severity, entities unavailable), then the low batteries by level, the
updates waiting with their versions, and the entities by domain.

## How it was validated

Against Home Assistant 2026.9.4, with the long-lived token of a user without
administrator rights: configuration, entity states of the demo integration and
of template sensors (battery levels, a battery binary sensor, an unavailable
sensor), update entities and the repairs list over the WebSocket API.

## Create a dedicated user and a long-lived token in Home Assistant

1. In Home Assistant: Settings → People → Users → Add user (turn on Advanced mode in your profile if the Users tab is missing). Name the user as follows, give it a long random password and leave the Administrator toggle off. A regular user can read every entity and the list of repairs, and cannot change the configuration.

    ```
    dumbmonit
    ```

2. Log in to Home Assistant once as that user, open its profile (the name at the bottom of the sidebar) → Security → Long-lived access tokens → Create token. Name it after DumbMonit and copy it now: it is shown only once.

3. In DumbMonit, enter the address of Home Assistant, for example "homeassistant.local" (port 8123) or "https://ha.example.net" behind a reverse proxy, and paste the token. Batteries under 20% count as low; the Low battery threshold option changes that.

4. DumbMonit only reads /api/config, /api/states and, over the WebSocket API, the list of repairs. It never calls a service, never fires an event and never changes a state.

!!! warning
    A long-lived token carries every right of its user and stays valid for ten years: create it for the dedicated user, never for your own. Home Assistant serves plain HTTP unless TLS is configured, and the token travels with every request: across an untrusted network, use HTTPS.

## Credentials

| Credential | Fields |
|---|---|
| Long-lived access token | A token of the dedicated user, sent as `Authorization: Bearer`. |

Address: a host name or IP (`homeassistant.local`), `host:port`, or a full URL
(`https://ha.example.net`, with a path prefix behind a reverse proxy); a
trailing `/api` is accepted. Options: protocol (HTTP by default), port (8123),
certificate check, request timeout (10 s), low battery threshold (20%), domains
left out (none) and repairs (read).
