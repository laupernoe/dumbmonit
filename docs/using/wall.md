# Wall mode and music

`/wall` shows the bulletin alone, full screen, for a screen that stays on in a
room: the sentence, the weather window, three big readouts and the *Needs you*
list, refreshed every 20 seconds, with the screen kept awake. ++esc++ or
**Exit** returns to the overview. Open it from the command palette ("Wall mode")
or by typing the URL. `?theme=dark` or `?theme=light` forces a light for that
display without touching your saved preference.

![Wall mode](../assets/screenshots/wall-light.png){ loading=lazy }

## Music on the wall

The wall can also play music and show what is playing, so that you can start a
playlist from your phone, hear it from the TV, and see it from across the room.
Music always sits in the bottom corner of the weather window, over the sky: it
never takes room from the bulletin or the *Needs you* list, and it disappears
when nothing plays. Each card folds into a small pill (the display remembers
it).

There are two ways, and you can use both.

| | Spotify Connect | A link sent to the walls |
|---|---|---|
| Start it from | the Spotify app on your phone (**Devices → DumbMonit Wall**) | any DumbMonit screen, your phone included (**Play on the wall**) |
| Services | Spotify | Spotify, Deezer, YouTube and YouTube Music |
| Needs | Spotify Premium, a Spotify Developer app of your own, the phone on the same Spotify account, the wall served over HTTPS (or localhost) in a browser with DRM | nothing; without being signed in on the wall, Spotify and Deezer play 30-second previews |
| Shows what plays | yes, wherever it plays: the wall, a TV's Spotify app, a cast speaker | the service's own player |

Connecting Spotify and sending links are admin actions; a viewer account on the
wall plays and shows everything.

### Spotify Connect: the wall becomes a speaker

The goal: on your phone, open Spotify, tap the devices icon, pick the TV — the
music plays on the TV while the wall shows what is playing. The wall does this
with Spotify's **Web Playback SDK**: the wall's browser registers itself as a
Spotify Connect device. That works only when **all** of these hold, and the
wall tells you on its own screen which one is missing:

| Needs | Why | When it is missing |
|---|---|---|
| **Spotify Premium** on the connected account | Spotify only lets Premium accounts use browser speakers | the wall shows *Spotify Premium is required*; Settings shows *Premium: No* |
| **The phone signed in to the same Spotify account** | a browser speaker is listed only for the account whose token it holds — it is not discovered on the network like a smart speaker, and other members of a Family plan do not see it | the phone simply does not list the wall |
| **HTTPS** (or `http://localhost` on the display itself) | browsers only allow protected media (DRM) in a secure page | the wall shows *opened over plain HTTP* |
| **A browser whose DRM works** (Widevine, PlayReady or FairPlay) | Spotify streams encrypted audio | the wall shows *This browser cannot play Spotify* |
| **One tap or key press on the wall per page load** | browsers keep a page silent until someone interacts with it | the wall shows *Tap to enable sound* |

Which browsers work: Chrome, Edge and Firefox on a computer (Firefox: allow
*Play DRM-controlled content*), Safari on a Mac, Chromium with Widevine on a
Raspberry Pi (`libwidevinecdm0`). Most **smart-TV browsers** (Samsung Tizen, LG
webOS), **kiosk browsers** and many **Linux Chromium builds** ship without a
usable Widevine: they cannot be a Spotify speaker, whatever DumbMonit does. For
such a TV, see [When the TV's browser cannot play Spotify](#when-the-tvs-browser-cannot-play-spotify).

#### Set it up

1. **Create a Spotify app.** Open the
   [Spotify Developer Dashboard](https://developer.spotify.com/dashboard), create
   an app (any name), tick **Web API** and **Web Playback SDK**, and add the
   redirect URI that **Settings → Wall music** shows:
    - DumbMonit served over HTTPS: `https://<your address>/api/music/spotify/callback`.
      Spotify comes straight back to DumbMonit after you approve.
    - DumbMonit served over plain HTTP: `http://127.0.0.1:8888/callback`.
      Spotify accepts `http://` only for a loopback address (and not
      `localhost`), since 2025. Nothing answers at that address: after you
      approve, your browser shows an error page whose address carries the
      code — copy it and paste it back into DumbMonit.
2. **Connect.** In **Settings → Wall music**, paste the app's **Client ID**
   (there is no secret: DumbMonit uses Authorization Code with PKCE) and choose
   **Connect Spotify**, then approve on Spotify **with the account your phone
   uses**.
3. **On the TV**, open DumbMonit's `/wall` over HTTPS, signed in (a viewer
   account is enough), and tap the screen or press **OK** on the remote once:
   any gesture unlocks the sound. A browser started with
   `--autoplay-policy=no-user-gesture-required` (a kiosk) needs no tap.
4. **On your phone**, in the Spotify app, tap the devices icon and pick
   **DumbMonit Wall**. The sound comes out of the TV, and the wall shows the
   cover, title and progress.

**Settings → Wall music → Speaker** shows whether it all holds, without walking
to the TV: the account, Premium, whether Spotify lists the speaker among the
account's devices right now (and which devices it lists), and every wall that
reported in the last fifteen minutes — its browser ("Chrome 130 on Linux"),
whether it is ready, and why not. **Test sound** makes the account play on the
speaker (it resumes what played last); each ready wall has its own **Test
sound** too. On the wall itself, the **Music** panel has **Play here**, which
moves whatever plays on the account to this display.

**The name.** Rename the speaker in **Settings → Wall music → Speaker** ("Living
room TV"); every wall picks the new name up within seconds. To name one wall
alone, add `?speaker=Kitchen` to its address — the display remembers it
(`?speaker=` forgets it).

**A wall stays on for days.** The wall asks DumbMonit for a fresh token when
Spotify's expires (every hour); when the device drops out (Wi-Fi, a sleeping
network card) and does not come back within thirty seconds, the wall registers
it again; a passing failure is retried by itself (15 s, 30 s, … then every
five minutes); and if Spotify stops listing the wall while it thinks it is
ready, it registers again. Only what waiting cannot fix — no DRM, no Premium —
stops for good, until **Try again**.

DumbMonit keeps the Spotify refresh token encrypted with the instance secret
and never sends it to a browser. The wall only receives a short-lived access
token for its player, and only through a signed-in session (an API token cannot
get one). **Disconnect** forgets the account; to also remove DumbMonit from
your Spotify account, use [spotify.com/account/apps](https://www.spotify.com/account/apps/).

Also good to know:

- A personal Spotify app (*development mode*) belongs to a Premium account and
  admits up to five accounts: add others under **User Management** in the
  dashboard.
- **Reconnect every six months.** Spotify ends a connection six months after it
  was approved; Settings shows the date, and the wall says so when it happens.

**Now playing works without any of that.** DumbMonit's server asks Spotify what
is playing on the account every few seconds (one request for all walls) and the
wall shows the cover, title, artist, progress and the device it plays on — the
wall itself, the Spotify app of a smart TV, a Chromecast, your laptop — even over
plain HTTP and in a browser that cannot play Spotify. To use that alone, switch
off **Use this display as a Spotify speaker** in the wall's Music panel.

#### When the TV's browser cannot play Spotify

Pick whichever matches your setup — in every case the wall keeps showing what
plays, because *now playing* reads the account, not the speaker:

- **The TV has a Spotify app, or a Chromecast / Google TV / Fire TV stick.**
  Pick that in Spotify's device list; the wall shows "Playing on Living room
  TV". This is the simplest and most reliable option.
- **A small computer drives the TV** (a Raspberry Pi, a mini PC, an old
  laptop). Open the wall in Chrome or Chromium with Widevine on it: it becomes
  the speaker as described above.
- **You want a speaker that is always there, even without a browser.** Run a
  Spotify Connect receiver based on [librespot](https://github.com/librespot-org/librespot)
  (MIT) — for instance [raspotify](https://github.com/dtcooper/raspotify) or
  spotifyd — **on the machine whose audio output reaches the TV or the
  speaker**. It shows up in the phone's device list for everyone on the
  network.

DumbMonit does not ship such a receiver itself, on purpose: the DumbMonit
container usually runs on a server in a cupboard, not on the machine plugged
into the TV, so its sound would come out of the wrong box; a receiver needs the
host's sound card and host networking (multicast DNS) that a monitoring
container should not have; and librespot depends on unofficial Spotify
protocols that break from time to time. A receiver installed on the right
machine, next to DumbMonit, does the job better.

### A link sent to every wall

In **Settings → Wall music** (or the wall's own **Music** panel), paste a
Spotify, Deezer or YouTube link to a track, album, playlist, episode or video and
choose **Play on the wall**. Every wall plays it in the service's official
embedded player; a link sent while a wall is open starts by itself where the
service allows it (YouTube, Deezer) once the wall has been tapped. **Stop**
removes it from every wall.

Deezer and YouTube have no Spotify Connect equivalent that a web page can use —
Deezer no longer accepts new developer apps and its API never exposed what is
playing — so a link is the way for them. The embedded players play 30-second
previews unless the wall's browser is signed in to the service.

### Privacy

Nothing third-party loads on the wall until Spotify is connected or a link is
set. The page's content security policy allows exactly the three embedded
players, Spotify's player script (`sdk.scdn.co/spotify-player.js`) and its
frame, and covers from `i.scdn.co` — nothing else, and nothing outside the web
interface.

### API

| Route | Who | What |
|---|---|---|
| `GET /api/music/now` | any session or token | What plays on the Spotify account (cached a few seconds) and the link the walls play |
| `PUT /api/music/link` `{"link": "…"}` | admin | Every wall plays this link |
| `DELETE /api/music/link` | admin | The walls stop the embedded player |
| `GET /api/music/spotify` | any session or token | The connected account: status, Client ID, reconnect-by date — never a token |
| `POST /api/music/spotify/authorize` | admin session | Starts a connection: `{"client_id", "redirect_uri"}` → Spotify's approval URL |
| `POST /api/music/spotify/complete` `{"url": "…"}` | admin session | Finishes a loopback connection with the address the browser landed on |
| `GET /api/music/spotify/callback` | the admin's browser | Spotify's direct redirect (HTTPS installs) |
| `DELETE /api/music/spotify` | admin | Disconnects and forgets the tokens |
| `GET /api/music/spotify/token` | browser session only | A short-lived access token for the wall's player |
| `GET /api/music/speaker` | any session or token | The speaker: name, Premium, the account's Spotify Connect devices, whether the speaker is listed, what each wall reported |
| `PUT /api/music/speaker` `{"name": "…"}` | admin | Renames the speaker (`null` goes back to "DumbMonit Wall") |
| `POST /api/music/speaker/report` | browser session only | A wall reports its speaker (sent every minute); the reply says whether Spotify lists it |
| `POST /api/music/speaker/play` `{"device_id": "…"}` | browser session only | Plays the account on that device, or on the device named like the speaker ("Test sound", "Play here") |
