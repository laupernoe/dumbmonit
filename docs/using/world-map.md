# World map

The **World** view of the Map page (`/map?view=world`) shows your locations on a
world map: one pin per location, with the state of the monitors it gathers.
Beside the map, the same locations are listed worst first; click a pin or a row
to see its monitors, each with its state and a link to its page.

A pin tells its state three ways, never by colour alone:

| Pin | Meaning |
|---|---|
| Red, cross | At least one monitor is **down** or unreachable. |
| Amber, `!` | A monitor is misconfigured or waiting for its first answer. |
| Teal, tick | Every monitor answers. |
| Grey, dash | Its monitors are paused or never probed. |

The figure on the pin is the number of monitors, or *problems / total* when
something is wrong (`2/5`).

## Locations

Locations are set in **Settings → Monitoring → Locations** (administrators). A
location has a name, an optional address (free text, shown only) and a spot,
chosen in one of three ways:

- **Find a place**: type a city; the server asks Open-Meteo's geocoding (the
  same service as the wall's weather) and you pick one of the answers. Your
  browser never contacts Open-Meteo.
- **Latitude / Longitude**: type the coordinates.
- **Click the map** where the location is.

## Which monitors a location gathers

A monitor finds its location from the most explicit to the most deduced:

1. **Placed by hand**: the *Location* field of the monitor's form (under *More
   options*), or **Place at** in the Select mode of the Monitors list, for many
   monitors at once.
2. **Its site**: a location can *stand for* a site of the
   [topology map](topology.md) — the `site` tag of the monitor, the site its
   agent or relay agent declares, or the name of the relay agent that polls it.
   Every monitor of that site lands there without being placed one by one.
3. **The main site**: one location can be marked *Main site*. It gathers every
   monitor that has no remote site.

Nothing is copied: change a monitor's relay or its `site` tag, and it moves on
the map at the next refresh. *Automatic* in the monitor's form (or **Place at →
Automatic** in bulk) removes a hand placement. Deleting a location keeps its
monitors; they simply lose their place. Monitors no location gathers are counted
under the list.

## Map background

By default the map draws the **built-in outlines**: the countries of
[Natural Earth](https://www.naturalearthdata.com/) (public domain) at 1:110
million, shipped with DumbMonit. The map then contacts no server at all, which
suits an instance without Internet access.

Two other backgrounds can be chosen under **Map background**:

- **OpenStreetMap tiles**: streets and towns. Each browser that opens the map
  loads tiles from `tile.openstreetmap.org`, sends your instance's address as
  referrer (their [tile usage policy](https://operations.osmfoundation.org/policies/tiles/)
  asks for it), and shows *© OpenStreetMap contributors*. Fine for occasional
  use; for a wall open all day on many screens, use your own tile server.
- **Your own tile server**: any `{z}/{x}/{y}` address on a single host (no
  `{s}` subdomains), with the attribution its licence asks for, shown in the
  corner of the map.

The chosen tile host is added to the interface's Content Security Policy
(`img-src`), and nothing else: the built-in outlines need no exception.

## Light, dark, phone

The map follows the theme; tiles are dimmed in dark mode. On a phone the list
comes under the map. Pins are keyboard reachable (Tab, then Enter).

## API

`GET /api/locations` returns the locations, their members (with how each one
got there: `assigned`, `site` or `main`), the known sites and the unplaced
monitors. See the [HTTP API](../reference/api.md) for the other routes.
