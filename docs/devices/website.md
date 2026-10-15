# Website health & SEO

Watch websites in bulk, one device per site: be told when a site **breaks**
(error, unreachable, invalid certificate, redirect loop), when its page **goes
blank**, when it **drops out of search engines** (`noindex`, `robots.txt`
blocking everything) and when the metadata Google shows **changes** (title,
description, canonical URL, final address). Each site is also sorted by the
platform it runs on (WordPress, Shopify, Wix, Next.js…).

For the content of a page rather than its health, see
[Website changes](webchange.md).

## Set it up

1. In the address, paste the page to check, usually the home page, with or without https://. Redirects are followed: http to https and example.com to www.example.com are normal, only a redirect loop or a move to another address is reported.
2. Optionally fill in "Expected text" with a phrase the page must always show, and adjust "Blank page below" if the page is short on purpose.
3. The first check records the reference: final address, title, description, canonical URL, indexing rules. They are compared again every 6 hours by default ("Check for changes every"), while up/down is checked at every polling interval. A change is only reported once 2 checks in a row (adjustable) have seen it, so A/B tests and flaky caches stay quiet.
4. To add many sites at once, import a CSV file from the device list: each site is filed under `Websites/<category>` (E-commerce, Blog… or the category you give) or, when the page does not say, `Websites/<platform>` (WordPress, Shopify…), unless you give a folder.

!!! warning
    Only what the server sends is read: a page drawn entirely by JavaScript looks blank here. Lower "Blank page below" to 0 for such a site.

## What each check does

Every polling interval (60 s by default), the page is read once:

- **Redirects are followed** one by one, up to **Max redirects** (10 by
  default), and the chain is recorded: each address and its status. Going from
  `http://` to `https://`, or from `example.com` to `www.example.com`, is what
  most sites do and is never an error. A redirect that comes back to an address
  already seen is a **loop**, and more redirects than allowed is an error too:
  both make the check fail with the chain in the message, like a site that does
  not answer.
- **The final answer**: its HTTP status, the total time (redirects included)
  and the TLS certificate, verified like a browser does. A refused certificate
  makes the check fail.
- **The visible text** of the page, as in [Website changes](webchange.md):
  scripts, styles and hidden blocks are left out. Below **Blank page below**
  characters (20 by default, so short legitimate pages pass) the page counts as blank. With
  **Expected text**, that phrase must appear in it (case and spacing ignored).
  A page that is not blank but **almost** is caught too, see
  [below](#nearly-empty-pages).
- **The SEO metadata** of the final page: `<title>`, meta description,
  canonical URL, robots meta tags and `X-Robots-Tag` headers (`noindex`,
  `nofollow`, `none`), `og:title`, `og:description`, `<html lang>`, the number
  of `<h1>`, the languages of the `hreflang` alternates and the Google
  verification tag (`google-site-verification`). The indexing directives and counts
  are reported at every check; comparing the metadata with the reference only
  happens at change checks, see [below](#changes-confirmed-twice).
- **At change checks, and at most once an hour**, `robots.txt` of the final
  site: does it forbid Googlebot (or `*`) to crawl `/`? And the sitemap it
  announces, or `/sitemap.xml`: does it answer and read as a sitemap?

The User-Agent is `DumbMonit/<version>` unless **User agent** says otherwise.

## Nearly empty pages

Besides the absolute threshold (**Blank page below**), DumbMonit keeps a slowly
moving reference of the visible text size of each page. When the text falls by
more than **Near-empty page: text drop** percent below it (80 by default, so
from 1,000 characters to under 200), and stays there for **Confirm a change
after** checks in a row, the "Website page nearly empty" alert fires. This
catches a theme that lost its content, a catalogue that no longer loads or a
cache serving a stub, while the page still answers 200. It needs a reference of
at least 200 characters, follows normal growth and shrinkage without noise,
and a drop that lasts more than a week becomes the new reference (the site was
redesigned shorter). 0 turns it off.

## Changes, confirmed twice

The final address and the metadata are compared to a reference, but not at
every check: only at **change checks**, every **Check for changes every**
minutes (360, that is 6 hours, by default; `DUMBMONIT_WEBSITE_CHANGE_CHECK_MINUTES`
sets the server-wide default, 0 compares at every check). Up/down, blank page
and expected text are still read at every polling interval.

A different value seen at a change check is only **pending**; the next checks
(one polling interval later, not 6 hours later) look at it again, and it
becomes a change once **Confirm a change after** checks in a row (2 by default,
1 to 10) have read the same new value. A value that flips back, or a third
value, starts the count again. A change is therefore reported at most one
change period plus the confirmation checks after it appears. A page served in two versions (an A/B test), a cache that
hesitates or a truncated answer therefore stays quiet. Values are compared with
spacing and case ignored; addresses ignore the scheme, a leading `www.`, a
trailing `/` and, with **Ignore query strings** (on by default), URL parameters
such as `?utm_source=…`.

Each kind of check can be switched off on its own, in the "Watched metadata"
options: final address, title, description, canonical URL, robots directives,
Open Graph, language, `<h1>` count, `hreflang` and Google verification tag. A
switched-off field is not compared, not recorded and raises no alert; switched
on again, it starts from a fresh reference. All are on by default. A field that
disappears from the page (an empty value) is a change like any other.

A confirmed change is kept in the device's history (field, before, after,
date), and its gauge (`dumbmonit_website_title_changed`…) stays at 1 for 24
hours. The first check only records the reference. Changing the address or
**Ignore query strings** starts a new reference instead of reporting a change.

## Options

| Option | Default | Meaning |
|---|---|---|
| Expected text | empty | A phrase the page must show. |
| Blank page below (characters) | 20 | Visible text shorter than this is a blank page; 0 turns the check off. |
| Max redirects | 10 | Redirects followed at most (0 to 30). |
| User agent | `DumbMonit/<version>` | Sent instead of DumbMonit's own. |
| Ignore query strings | on | URL parameters do not count when comparing addresses. |
| Check for changes every (minutes) | 360 | How often the metadata, the final address and `robots.txt` are compared, 0 to 10080; 0 means at every check. Empty: the server default (`DUMBMONIT_WEBSITE_CHANGE_CHECK_MINUTES`). |
| Wait before the next request to this domain (seconds) | 2 | Minimum gap after this site's check before another check on the same domain, 0 to 3600. Empty: the server default (`DUMBMONIT_WEBSITE_HOST_SPACING_SECS`). |
| Category | empty | Your own label for the site, shown with it and in its metrics (set by the CSV import). Empty: guessed, see [Categories](#categories). |
| Confirm a change after (checks) | 2 | Consecutive reads of a new value before it is reported, 1 to 10. |
| Near-empty page: text drop (%) | 80 | Alert when the visible text falls this much below its usual size, 0 to 99; 0 turns it off. |
| Watch the final address, title, description, canonical URL, robots directives, Open Graph, language, h1 count, hreflang, Google verification tag | all on | Each metadata check can be switched off on its own. |
| Allow loopback and link-local targets | off | For a site served by the DumbMonit host itself. |

## Pacing hundreds of sites

Checks never leave in one burst, so a firewall or CDN in front of the sites
does not mistake DumbMonit for an attack:

- **Spread over the interval**: each site starts at its own, fixed offset
  within its polling interval, so sites added together (a CSV import) do not
  fire at the same second.
- **Global cap**: at most `DUMBMONIT_WEBSITE_MAX_CONCURRENT` website checks in
  flight (10 by default), on top of `DUMBMONIT_MAX_CONCURRENT_PROBES`.
- **Per domain**: at most `DUMBMONIT_WEBSITE_MAX_PER_HOST` checks in flight
  towards one domain (1 by default), and at least
  `DUMBMONIT_WEBSITE_HOST_SPACING_SECS` seconds between two of them (2 by
  default, or the site's **Wait before the next request to this domain**).
  The domain is the registrable name: `www.example.com`, `shop.example.com`
  and `example.com` share one, `example.co.uk` is one; customer subdomains of
  hosting platforms (`*.myshopify.com`, `*.wordpress.com`, `*.github.io`,
  `*.netlify.app`…) each count as their own.

A site whose previous check is still waiting its turn skips a round rather
than queueing twice. Many sites on one domain with a short polling interval
are therefore checked less often than asked: raise the interval, or the
per-domain limits, if the device list shows them late. See
[Configuration](../reference/configuration.md#server).

## Platform detection

The platform comes from, in order: the `<meta name="generator">` tag, headers
specific to a host or engine (`x-shopify-*`, `x-wix-*`, `x-drupal-cache`,
`x-powered-by: Next.js`…), traces in the markup (`/wp-content/`,
`cdn.shopify.com`, `data-wf-site`, `__NEXT_DATA__`, `/_nuxt/`…) and finally the
host itself (`*.myshopify.com`, `*.wixsite.com`…). Recognised: WordPress,
Shopify, Wix, Squarespace, Webflow, Ghost, Drupal, Joomla, PrestaShop, Magento,
Next.js, Nuxt, Gatsby, Hugo, Jekyll; anything else is **Other**. The platform
is the `platform` label of `dumbmonit_website_info`.

## Categories

Each site gets a category, shown on its page, in the `category` label of
`dumbmonit_website_info` and as its folder when it is imported. You can set it
yourself (the **Category** option, or the CSV column); otherwise it is guessed
from clear signs only:

- **E-commerce**: Shopify, PrestaShop or Magento, WooCommerce or add-to-cart
  markup, `og:type` set to `product`.
- **Blog**: Ghost, `og:type` set to `article`, a `blog.` host or a
  `*.blogspot.com`, `*.substack.com`, `*.wordpress.com` address.
- **Web app**: Next.js or Nuxt, or a page that is only an empty `#root` / `#app`
  container.
- **Showcase site**: Wix, Squarespace or Webflow.

A general-purpose platform (WordPress, Drupal, Hugo…) with no other sign is
not guessed: the site then stays under its platform (`Websites/WordPress`) and
sites of an unknown platform under `Websites/Other`. Sort them by hand with the
`folder` or `category` columns, or move them in the device list.

## Import a list of sites (CSV)

From the **⋯** menu of the Monitors page or the add-device page, **Import
websites (CSV)** takes a
file or pasted text with a header row and up to 1000 rows:

| Column | Required | Meaning |
|---|---|---|
| `url` | yes | Address of the site. |
| `name` | no | Device name; the host by default. |
| `folder` | no | Folder of the device; empty: `Websites/<category>` if there is a category, else `Websites/<platform>`, detected by reading the site once. |
| `category` | no | Free text (60 characters, no `/`), stored as the **Category** option. Empty: guessed from the page, see [Categories](#categories). |
| `tags` | no | `key=value;key=value`. |
| `interval` | no | Polling interval. |
| `expect_text` | no | Same as the option. |

```csv
url,name,folder,tags,interval,expect_text,category
https://example.com,Example,,client=acme,60s,Example Inc.,
https://shop.example.org,Shop,Customers/Acme,,300s,Add to cart,
https://blog.example.net,,,,,,Clients
```

A preview shows each row and its errors before anything is created; a site
already monitored (same final address) is skipped.

## Metrics

| Metric | Meaning |
|---|---|
| `dumbmonit_website_up` | 1 when the final answer is a success (2xx or 3xx), 0 otherwise. |
| `dumbmonit_website_status_code` | HTTP status of the final answer. |
| `dumbmonit_website_response_seconds` | Total time, redirects included. |
| `dumbmonit_website_redirect_hops` | Redirects followed. |
| `dumbmonit_website_https` | 1 when the final address is https. |
| `dumbmonit_website_security_header{header}` | 0/1 per `header` label (https only): HSTS, nosniff, CSP, X-Frame-Options, frame-ancestors. |
| `dumbmonit_website_info{platform,category}` | Always 1; the labels carry the platform and, when known, the category. |
| `dumbmonit_website_text_chars` | Characters of visible text (2xx answers). |
| `dumbmonit_website_blank` | 1 when that text is below the threshold. |
| `dumbmonit_website_expect_text_missing` | 1 when the expected text is missing (option set). |
| `dumbmonit_website_noindex` / `dumbmonit_website_nofollow` | Indexing directives. |
| `dumbmonit_website_h1_count` / `dumbmonit_website_hreflang_count` | `<h1>` headings, `hreflang` alternates. |
| `dumbmonit_website_title_changed`, `…_description_changed`, `…_canonical_changed`, `…_final_url_changed` | 1 for 24 hours after a confirmed change. |
| `dumbmonit_website_text_dropped` | 1 when the visible text is far below its usual size (option on). |
| `dumbmonit_website_robots_changed`, `…_open_graph_changed`, `…_lang_changed`, `…_h1_changed`, `…_hreflang_changed`, `…_google_verification_changed` | 1 for 24 hours after a confirmed change of that metadata. |
| `dumbmonit_website_robots_blocks_all` | 1 when `robots.txt` forbids everything to Googlebot. |
| `dumbmonit_website_sitemap_ok` | 1 when a sitemap answers and reads. |

## Built-in alerts

| Alert | When | Severity |
|---|---|---|
| Device unreachable | No answer, certificate refused, redirect loop or too many redirects | critical |
| Website down | The final answer is a 4xx or 5xx error, for 3 minutes | critical |
| Website page blank | Visible text below the threshold, for 3 minutes | critical |
| Website expected text missing | The expected text is missing, for 3 minutes | warning |
| Website not indexable | `noindex` on the page, for 10 minutes | critical |
| Website blocked by robots.txt | `robots.txt` forbids everything, for 10 minutes | critical |
| Website canonical URL changed | Confirmed change | warning |
| Website address changed | The final host or path changed, confirmed | warning |
| Website title or description changed | Confirmed change | info |
| Website page nearly empty | The visible text fell far below its usual size, for 3 minutes | warning |
| Website Google verification changed | The `google-site-verification` tag changed or disappeared, confirmed | warning |
| Website SEO markup changed | Robots directives, Open Graph, language, h1 count or hreflang changed, confirmed | info |
| Website slow | More than 5 seconds, for 10 minutes | warning |

The check runs on the DumbMonit server itself: it cannot go through a relay
agent.
