# Website changes

Watch the *content* of a web page, or of a whole site, and be told when it
changes: a pricing or terms page, a documentation site, a supplier's
maintenance notice, a public announcement page, or your own site after a
deployment. Each change is kept with the text before and after, a line diff
and, when a browser is available, a screenshot of each version.

What is compared is the **visible text** of each page, not its HTML: scripts,
styles, comments, `noscript`, `svg` and `template` blocks are dropped, white
space is collapsed and each block (paragraph, heading, list item, table row)
becomes one line. A new build of the same page, a reordered attribute or a
fresh cache-busting asset name changes nothing; a reworded sentence does.

## Page or site

**Page** (the default) reads the address of the device and nothing else. It
may also be a plain text or JSON document, compared line by line.

**Site** starts from the address and follows every page under the same folder
(`https://example.com/docs/intro.html` covers everything under `/docs/`;
**Path prefix** chooses another folder). Pages are found:

1. in the site's sitemap: the ones `robots.txt` announces (`Sitemap:` lines),
   or `/sitemap.xml`, sitemap indexes included (one level);
2. failing that, or when the sitemap lists nothing under the folder, by
   following the links of each page, breadth first.

Only pages of the same origin (scheme, host and port) are followed; links to
images, style sheets, scripts, PDFs, archives and other files are skipped by
their extension, and any answer that is not HTML is ignored. Fragments
(`#section`) are dropped and empty queries removed, so one page is one address.
`robots.txt` is honoured for the pages found this way: the group that names
`DumbMonit`, or else the `*` group, with `Allow`, `Disallow`, `*` and `$`.

The crawl is polite: two requests at a time at most, half a second between two
batches, the `User-Agent` `DumbMonit/<version>`, a 15-second timeout per
request and 5 MB read at most per page. It stops at **Max pages** (25 by
default, 200 at most) or after ten minutes.

## How a check works

The device is polled like any other (every minute by default), but the pages
are only read every **Check every** minutes (60 by default, 5 to 10080). In
between, the poll only reports the state of the last check: the site is not
hit every minute. A check runs in the background; two checks of the same
device never overlap, and **Check now** on the device page starts one at once.

The first check only records the **reference**: every page is read and kept,
nothing is reported. From then on, for each page:

| Situation | What is kept |
|---|---|
| Same text as last time (after **Ignore lines**) | Nothing new; the page is marked checked. |
| Different text | A new snapshot and a **changed** event, with the number of lines added and removed. |
| A page never seen before, or seen again after it had gone | A snapshot and a **new page** event. |
| The page answers 404 or 410 | A **removed page** event, the last snapshot as "before". |
| Site scope: a complete crawl no longer reaches a known page | A **removed page** event too ("No longer linked"). A crawl stopped by **Max pages** or by its time limit never concludes that a page has gone. |
| Any other error on a page (timeout, 403, 500 on a sub-page) | The error is shown on the page; no event. |

Changing the address, the scope, the path prefix or **Ignore lines** makes the
next check a new reference instead of reporting every page as changed.

### Ignore lines

Lines that change at every visit — a clock, a visitor counter, "updated 5
minutes ago", a random quote — would be reported at every check. **Ignore
lines** holds regular expressions (Rust `regex` syntax), one per line or
joined with `|`; any text line one of them matches is left out before
comparing. For example:

```
Updated \d+ minutes ago|Visitors: \d+|© \d{4}
```

### Unreachable start page

When the start page itself cannot be read (no answer, connection refused,
timeout, HTTP 5xx), the device is **unreachable**: the poll fails, and the
"Device unreachable" rule fires as for any device. The start page is then
retried at every poll, so its return is seen within a minute. A start page that
refuses access (401, 403) is reported as a configuration problem, not an
outage. Errors on other pages of the site are only shown on those pages.

Addresses only the DumbMonit host itself can reach (loopback, link-local, the
cloud metadata service) are refused unless **Allow loopback and link-local
targets** is enabled; private LAN addresses are always allowed.

## Alerts

The built-in rule **Website changed** (`webchange_detected`, Info) fires after
a check that found at least one change (changed, new or removed page), and
resolves at the next check that finds none. It reads
`dumbmonit_webchange_last_check_changes`; attach it to a channel like any other
rule, or raise its severity, in **Alerts → Rules**. See
[Built-in rules](../alerting/rules.md#website-changes).

| Metric | What |
|---|---|
| `dumbmonit_webchange_pages` | Pages followed (removed pages excluded). |
| `dumbmonit_webchange_pages_failed` | Pages followed whose last read failed. |
| `dumbmonit_webchange_last_check_changes` | Changes found by the last check. |

The metrics appear once the first check is done.

## Screenshots

The DumbMonit image has no browser: screenshots come from an optional headless
Chromium, driven over the Chrome DevTools Protocol. The Compose file ships one
as the `browser` service, under the `screenshots` profile, so it only starts
when asked:

```
DUMBMONIT_BROWSER_URL=http://browser:9222 docker compose --profile screenshots up -d
```

`DUMBMONIT_BROWSER_URL` points DumbMonit at the browser (any Chromium started
with `--remote-debugging-port`, for example `chromedp/headless-shell`; the
server reads `/json/version` and talks to the address the host name resolves
to). Put the variable in a `.env` file next to `docker-compose.yml` to keep it.

A screenshot is taken only when a page gets a new snapshot (the reference, and
every change), never on an unchanged check: a 1366 × 900 window, the full page
down to 5000 pixels, as JPEG. The **Screenshots** option turns them off for one
device. Without a browser, or when it does not answer, the text comparison
works the same and the snapshots simply have no image; the device page says
so.

The browser loads the page the way a visitor would, with its scripts: what it
shows can differ from the text DumbMonit compared, which is the HTML the server
sent.

## Storage

The text of each snapshot is kept in the database (1 MB at most per page);
screenshots are files under `/data/webchange/<device id>/`. Per device, the
last hundred changes are kept; a snapshot is kept while it is the latest of a
page or a kept change refers to it, and everything else, files included, is
removed after each check. Deleting the device removes all of it.

## Watch a website for changes

The steps below are the ones the notice next to the form shows.

1. In the address, paste the page to watch, with http:// or https://. To watch
   a whole site or one of its sections, paste its start page and set "Scope" to
   "site": every page under the same folder is followed, up to "Max pages".
2. Lines that change at every visit (a clock, a visitor counter, "updated 5
   minutes ago") would be reported each time: list them in "Ignore lines" as
   regular expressions.
3. The first check only records the reference, nothing is reported. From then
   on, each check compares the visible text page by page and keeps a
   before/after copy of every change, shown on the device page; the "Website
   changed" alert tells you.
4. For a screenshot of each version, start the optional browser service of the
   Compose file and point DumbMonit at it:

    ```
    DUMBMONIT_BROWSER_URL=http://browser:9222 docker compose --profile screenshots up -d
    ```

!!! warning
    Only the text the server sends is compared: a page drawn entirely by
    JavaScript shows little or nothing to the check. Pages behind a login are
    not reachable either.

Credentials: none. Checks always run on the DumbMonit server: a website watch
cannot go through a relay agent.

## Options

| Option | Default | Meaning |
|---|---|---|
| Scope (`scope`) | `page` | `page`: the address only. `site`: every page under the same folder. |
| Max pages (`max_pages`) | `25` | Site scope: pages read at most per check, 1 to 200. |
| Path prefix (`path_prefix`) | folder of the address | Site scope: only pages whose path starts with this (`/docs/`). |
| Ignore lines (`ignore`) | — | Regular expressions; matching text lines are not compared. |
| Screenshots (`screenshots`) | `true` | Keep a screenshot of each new version, when a browser is configured. |
| Check every (`check_interval_minutes`) | `60` | Minutes between two checks, 5 to 10080. |
| Accept an unverifiable certificate (`insecure_tls`) | `false` | For a site of your own behind a self-signed certificate. |
| Allow loopback and link-local targets (`allow_private_targets`) | `false` | Reach a site served by the DumbMonit host itself. |
