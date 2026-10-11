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
3. The first check records the reference: final address, title, description, canonical URL, indexing rules. A change is only reported once two checks in a row have seen it, so A/B tests and flaky caches stay quiet.
4. To add many sites at once, import a CSV file from the device list: each site is filed under `Websites/<platform>` (WordPress, Shopify…) unless you give a folder.

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
- **The SEO metadata** of the final page: `<title>`, meta description,
  canonical URL, robots meta tags and `X-Robots-Tag` headers (`noindex`,
  `nofollow`, `none`), `og:title`, `og:description`, `<html lang>`, the number
  of `<h1>` and of `hreflang` alternates.
- **Once an hour**, `robots.txt` of the final site: does it forbid Googlebot
  (or `*`) to crawl `/`? And the sitemap it announces, or `/sitemap.xml`: does
  it answer and read as a sitemap?

The User-Agent is `DumbMonit/<version>` unless **User agent** says otherwise.

## Changes, confirmed twice

The final address and the metadata are compared to a reference. A different
value seen once is only **pending**; it becomes a change when the next check
sees it again. A page served in two versions (an A/B test), a cache that
hesitates or a truncated answer therefore stays quiet. Values are compared with
spacing and case ignored; addresses ignore the scheme, a leading `www.`, a
trailing `/` and, with **Ignore query strings** (on by default), URL parameters
such as `?utm_source=…`.

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
| Allow loopback and link-local targets | off | For a site served by the DumbMonit host itself. |

## Platform detection

The platform comes from, in order: the `<meta name="generator">` tag, headers
specific to a host or engine (`x-shopify-*`, `x-wix-*`, `x-drupal-cache`,
`x-powered-by: Next.js`…), traces in the markup (`/wp-content/`,
`cdn.shopify.com`, `data-wf-site`, `__NEXT_DATA__`, `/_nuxt/`…) and finally the
host itself (`*.myshopify.com`, `*.wixsite.com`…). Recognised: WordPress,
Shopify, Wix, Squarespace, Webflow, Ghost, Drupal, Joomla, PrestaShop, Magento,
Next.js, Nuxt, Gatsby, Hugo, Jekyll; anything else is **Other**. The platform
is the `platform` label of `dumbmonit_website_info`.

## Import a list of sites (CSV)

From the device list or the add-device page, **Import websites (CSV)** takes a
file or pasted text with a header row and up to 1000 rows:

| Column | Required | Meaning |
|---|---|---|
| `url` | yes | Address of the site. |
| `name` | no | Device name; the host by default. |
| `folder` | no | Folder of the device; empty: `Websites/<platform>`, detected by reading the site once. |
| `tags` | no | `key=value;key=value`. |
| `interval` | no | Polling interval. |
| `expect_text` | no | Same as the option. |

```csv
url,name,folder,tags,interval,expect_text
https://example.com,Example,,client=acme,60s,Example Inc.
https://shop.example.org,Shop,Customers/Acme,,300s,Add to cart
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
| `dumbmonit_website_info{platform}` | Always 1; the label carries the platform. |
| `dumbmonit_website_text_chars` | Characters of visible text (2xx answers). |
| `dumbmonit_website_blank` | 1 when that text is below the threshold. |
| `dumbmonit_website_expect_text_missing` | 1 when the expected text is missing (option set). |
| `dumbmonit_website_noindex` / `dumbmonit_website_nofollow` | Indexing directives. |
| `dumbmonit_website_h1_count` / `dumbmonit_website_hreflang_count` | `<h1>` headings, `hreflang` alternates. |
| `dumbmonit_website_title_changed`, `…_description_changed`, `…_canonical_changed`, `…_final_url_changed` | 1 for 24 hours after a confirmed change. |
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
| Website slow | More than 5 seconds, for 10 minutes | warning |

The check runs on the DumbMonit server itself: it cannot go through a relay
agent.
