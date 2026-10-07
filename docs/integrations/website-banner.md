# Website banner

Show a banner on your own website when services are down ("2 services are
down"), driven by a published [status page](../using/status-pages.md). When
everything is fine the banner stays hidden, unless you ask for it to be shown
("All systems operational").

Three ways to add it, from simplest to most controlled. Ready-to-copy files are
in the `integrations/` folder of the repository.

## HTML snippet

Paste this before `</body>`, with your server address and the page's address:

```html
<script async
  src="https://monit.example.com/api/public/status/my-page/banner.js"
  data-position="top" data-show="issues" data-lang="en"></script>
```

| Attribute | Values | Default |
|---|---|---|
| `data-position` | `top`, `bottom` | `top` |
| `data-show` | `issues` (only when something is wrong), `always` | `issues` |
| `data-lang` | `en`, `fr` | page language, else `en` |

The script has no dependencies, draws inside a Shadow DOM so your styles do not
touch it, follows the visitor's light or dark preference and can be dismissed
(until the browser tab is closed). It reads the page's public JSON, which the
server allows any website to fetch (`Access-Control-Allow-Origin: *`, no
cookies). Only the status page's public document is opened this way; the rest
of the API keeps its rules.

## WordPress

Copy `integrations/wordpress/dumbmonit-status-banner` into
`wp-content/plugins/`, activate it, then open **Settings > DumbMonit Banner**
and enter the status page URL
(`https://monit.example.com/api/public/status/my-page`), the position and the
show mode. The plugin fetches the JSON on the server, caches it for one minute
and escapes everything it prints. WordPress refuses requests to private
addresses, so the DumbMonit server must be reachable on a public address.

## PHP include

For any PHP site, without a framework:

```php
require __DIR__ . '/banner.php';
dumbmonit_banner('https://monit.example.com/api/public/status/my-page',
    ['position' => 'top', 'show' => 'issues', 'ttl' => 60]);
```

The helper accepts `http` and `https` URLs only, uses a 3-second timeout, does
not follow redirects, keeps a file cache in the temporary directory and prints
nothing if the server cannot be reached. See `integrations/php/example.php`.
