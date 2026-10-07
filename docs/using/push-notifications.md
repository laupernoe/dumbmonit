# Push notifications (PWA)

DumbMonit can ring your phone or computer itself, the way a native app does,
without installing an app or creating an account anywhere: the browser's own
push service (Google, Mozilla, Apple, Microsoft) carries an end-to-end
encrypted message from your server to the device, and the notification shows
even with every DumbMonit tab closed. This is the standard **Web Push**; the
site can also be installed as an app (a *progressive web app*, PWA) so it opens
from an icon like any other.

Two steps: enable push on each device (this page), then decide which alerts go
there with a **Web Push** channel
([Notification channels](../notifications.md#web-push)).

## Before you start

- **HTTPS is required.** Browsers only offer push to a secure origin. Put
  DumbMonit behind a reverse proxy with a certificate (Caddy, Traefik, Nginx
  Proxy Manager…) and open it through that address. `http://localhost` is
  accepted too, which is only useful for testing on the server itself. On plain
  `http://192.168.x.x`, the section says **Needs HTTPS**.
- **The server needs outbound HTTPS** to the push services:
  `fcm.googleapis.com` (Chrome, and Edge on Android), `*.notify.windows.com`
  (Edge on Windows), `updates.push.services.mozilla.com` (Firefox) and
  `*.push.apple.com` (Safari, iPhone, iPad). Nothing comes in: your
  DumbMonit does not need to be reachable from the internet, the phone only
  has to reach it when you tap a notification.
- **Set `DUMBMONIT_PUBLIC_URL`** to the HTTPS address you use: it is given to
  the push services as the sender's contact, and it is the address the links
  in other channels point to.

## Enable on a phone or computer

### Android, Windows, macOS, Linux

1. Open DumbMonit in Chrome, Edge, Firefox or Safari and sign in.
2. Optional: install it as an app (address bar → **Install**, or the browser
   menu → **Add to Home screen**). Notifications work either way.
3. **Settings → Push notifications → Enable on this device**, and allow
   notifications when the browser asks.
4. **Send a test.** A *DumbMonit — test notification* should appear within a
   few seconds.

### iPhone and iPad

Apple only gives push to sites added to the Home Screen (iOS and iPadOS 16.4
or later):

1. Open DumbMonit in **Safari**.
2. **Share → Add to Home Screen**, then open DumbMonit **from the new icon**.
3. Sign in, then **Settings → Push notifications → Enable on this device**,
   and allow notifications.
4. **Send a test.**

In Safari without installing, the section says **Install first**.

## Choose which alerts arrive

In **Alerts → Notifications**, add a channel of type **Web Push**. With no
setting, every subscribed device of every account receives it; list account
names under **Accounts** to target some people only. The usual channel policy
applies (minimum severity, quiet hours, routing filter…), so you can for
example keep a phone for critical alerts only.

A notification reads `Critical · nas01` with a short summary; tapping it opens
the **Alerts** page. The resolution of an alert replaces its notification
instead of adding a second one.

## Devices and accounts

Each account sees and manages its own devices: viewers can enable push on
their phone too, it changes nothing else. The list shows the browser and
system, the push service, when the device was added and when it last received
a message; **Remove** forgets it (and, on the device you are using,
unsubscribes the browser as well). A device the push service reports as gone
(the app was removed, the browser data cleared) is deleted automatically at
the next send.

## Troubleshooting

| What you see | What to do |
|---|---|
| **Needs HTTPS** | Open DumbMonit through its HTTPS address. |
| **Install first** (iPhone, iPad) | Share → Add to Home Screen, open from the icon. |
| **Blocked** | Notifications were refused for this site: allow them in the browser's site settings (padlock in the address bar, or iOS Settings → Notifications → DumbMonit), then reload. |
| **Not supported** | The browser has no Web Push (old version, some privacy browsers): use a recent Chrome, Edge, Firefox or Safari. |
| Test says *push service unreachable* | The server cannot reach the push service: check its outbound HTTPS and DNS. |
| Test says *refused the VAPID signature* | Usually a wrong server clock; check NTP on the host. |
| Nothing arrives on Android when the phone sleeps | Battery optimisation can delay the browser: exempt it, or set the channel's **Urgency** to `high`. |

## Privacy and security

- Messages are encrypted for each device (RFC 8291) before they leave the
  server: the push service only sees an opaque blob, the device decrypts it.
- The server signs each message with its own VAPID key pair (RFC 8292),
  generated at first start and stored encrypted with the instance secret; the
  private key is never returned by the API. A [restore](../install/backup.md)
  on a new machine generates a new key pair: enable push again on each device.
- The subscription address comes from the browser, so the server only accepts
  HTTPS addresses on public IPs: private, loopback and link-local addresses
  are refused, at subscription time and again at every connection.
- The service worker (`/sw.js`) only shows notifications. It caches nothing
  and does not intercept any request: the interface and the API always come
  from the server, never from a stale copy.
