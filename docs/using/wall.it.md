# Modalità Wall {#wall-mode-and-music}

`/wall` mostra la lavagna propria della console, a schermo intero, per un monitor
che resta acceso in una sala. Si aggiorna da sola e tiene lo schermo sveglio.

Dall'alto:

- il **verdetto** in poche parole («All good», «3 problems») con data e orologio,
  e quando è stato aggiornato l'ultima volta;
- le **cinque cifre** dell'overview: monitor attivi, disponibilità a 30 giorni,
  tempo di risposta p95, allarmi aperti e tempo medio di risoluzione;
- il **fiume di salute**, la grande immagine della lavagna: una striscia per
  monitor, i peggiori in alto;
- gli **allarmi aperti**, i più gravi per primi;
- **chi è reperibile** ora, con il calendario e la fine del turno.

Nessun widget: fondo nero, la tipografia della console, stati scritti accanto a
una forma e mai solo a colori. La [musica](#music-on-the-wall), una volta
configurata, sta in un piccolo dock in fondo alla colonna di destra.

## Aprire e uscire

Aprila con l'icona **Wall mode** in fondo al menu laterale, dalla command palette
(«Wall mode») o digitando l'URL. ++esc++ o **Exit** torna alla console. Un
account limitato ad alcuni team o clienti non ha la modalità Wall, perché mostra
l'intera flotta.

## Sfondo {#backdrop}

La parete condivide l'aspetto scuro della console. Il pulsante **Sfondo** (in
alto a destra, accanto a Exit) può disegnare una delle città di notte delle
[pagine di stato](status-pages.md) dietro al verdetto e all'orologio: Parigi,
New York, Tokyo o Dubai, oppure *Auto* per la più vicina al fuso orario del
browser. È spento per impostazione predefinita e ricordato per browser;
`?city=paris` (o `newyork`, `tokyo`, `dubai`, `none`) in un link lo sceglie solo
per quello schermo. Il cielo segue il quadro: stelle quando va tutto bene,
nuvole con un avviso, temporale con un allarme. Su un telefono, o con la scheda
nascosta, la città resta ferma. Uno `zermatt` salvato o in un link ora porta a
Dubai.

## Musica sul wall {#music-on-the-wall}

Il wall può anche riprodurre musica e mostrare cosa sta suonando: avvii una
playlist dal telefono, la ascolti dalla TV e la vedi dall'altro capo della
stanza. La musica sta in un piccolo dock in fondo alla colonna di destra, sotto
l'orologio e la reperibilità: non toglie mai spazio al verdetto né ai problemi,
e scompare quando non suona nulla o nulla è configurato. Mostra la copertina, il
titolo, l'artista, il dispositivo che suona e una sottile barra di avanzamento;
quando l'account collegato lo consente, un pulsante **riproduci / pausa** mette
in pausa o riprende l'account ovunque suoni (qualsiasi sessione connessa può
farlo, account lettore compresi, come per *Riproduci qui*). Il lettore di un
link condiviso e i messaggi dello speaker si richiudono in una piccola pillola
(lo schermo se lo ricorda). Il pulsante **Musica** del wall, accanto a Sfondo ed
Esci, compare una volta configurata la musica.

Ci sono due modi, e puoi usarli entrambi.

| | Spotify Connect | Un link inviato ai wall |
|---|---|---|
| Avvialo da | l'app Spotify sul tuo telefono (**Devices → DumbMonit Wall**) | qualsiasi schermo DumbMonit, telefono incluso (**Play on the wall**) |
| Servizi | Spotify | Spotify, Deezer, YouTube e YouTube Music |
| Richiede | Spotify Premium, una tua app Spotify Developer, il telefono sullo stesso account Spotify, il wall servito su HTTPS (o localhost) in un browser con DRM | nulla; senza essere connesso sul wall, Spotify e Deezer riproducono anteprime di 30 secondi |
| Mostra cosa suona | sì, dovunque suoni: il wall, l'app Spotify di una TV, uno speaker cast | il player proprio del servizio |

Collegare Spotify e inviare link sono azioni da amministratore; un
account visualizzatore sul wall riproduce e mostra tutto.

### Spotify Connect: il wall diventa uno speaker

L'obiettivo: sul tuo telefono, apri Spotify, tocca l'icona dei
dispositivi, scegli la TV — la musica suona sulla TV mentre il wall
mostra cosa sta suonando. Il wall lo fa con il **Web Playback SDK** di
Spotify: il browser del wall si registra come un dispositivo Spotify
Connect. Funziona solo quando valgono **tutti** questi, e il wall ti
dice sul proprio schermo quale manca:

| Richiede | Perché | Quando manca |
|---|---|---|
| **Spotify Premium** sull'account collegato | Spotify lascia usare gli speaker browser solo agli account Premium | il wall mostra *Spotify Premium is required*; Settings mostra *Premium: No* |
| **Il telefono connesso allo stesso account Spotify** | uno speaker browser è elencato solo per l'account di cui porta il token — non viene scoperto sulla rete come uno speaker intelligente, e gli altri membri di un piano Family non lo vedono | il telefono semplicemente non elenca il wall |
| **HTTPS** (oppure `http://localhost` sul display stesso) | i browser consentono media protetti (DRM) solo in una pagina sicura | il wall mostra *opened over plain HTTP* |
| **Un browser il cui DRM funziona** (Widevine, PlayReady o FairPlay) | Spotify trasmette audio cifrato | il wall mostra *This browser cannot play Spotify* |
| **Un tocco o pressione di tasto sul wall per ogni caricamento di pagina** | i browser mantengono una pagina silenziosa finché qualcuno non ci interagisce | il wall mostra *Tap to enable sound* |

Quali browser funzionano: Chrome, Edge e Firefox su un computer
(Firefox: consenti *Play DRM-controlled content*), Safari su un Mac,
Chromium con Widevine su un Raspberry Pi (`libwidevinecdm0`). La
maggior parte dei **browser delle smart TV** (Samsung Tizen, LG webOS),
i **browser kiosk** e molte **build Linux di Chromium** vengono senza un
Widevine utilizzabile: non possono essere uno speaker Spotify, qualunque
cosa faccia DumbMonit. Per una tale TV, vedi
[Quando il browser della TV non può riprodurre Spotify](#when-the-tvs-browser-cannot-play-spotify).

#### Configurarlo

1. **Crea un'app Spotify.** Apri la
   [Spotify Developer Dashboard](https://developer.spotify.com/dashboard), crea
   un'app (qualsiasi nome), spunta **Web API** e **Web Playback SDK**, e
   aggiungi l'URI di redirect che **Impostazioni › Schermo a parete › Musica per la parete** mostra:
    - DumbMonit servito su HTTPS: `https://<il tuo indirizzo>/api/music/spotify/callback`.
      Spotify torna direttamente a DumbMonit dopo che approvi.
    - DumbMonit servito su HTTP semplice: `http://127.0.0.1:8888/callback`.
      Spotify accetta `http://` solo per un indirizzo loopback (e non
      `localhost`), dal 2025. Nulla risponde a quell'indirizzo: dopo
      l'approvazione, il tuo browser mostra una pagina di errore il cui
      indirizzo porta il codice — copialo e incollalo di nuovo in
      DumbMonit.
2. **Collega.** In **Impostazioni › Schermo a parete › Musica per la parete**, incolla il **Client ID**
   dell'app (non c'è un secret: DumbMonit usa Authorization Code con
   PKCE) e scegli **Connect Spotify**, poi approva su Spotify **con
   l'account che usa il tuo telefono**.
3. **Sulla TV**, apri `/wall` di DumbMonit su HTTPS, connesso (un account
   visualizzatore basta), e tocca lo schermo o premi **OK** sul
   telecomando una volta: qualsiasi gesto sblocca il suono. Un browser
   avviato con `--autoplay-policy=no-user-gesture-required` (un kiosk)
   non ha bisogno di un tocco.
4. **Sul tuo telefono**, nell'app Spotify, tocca l'icona dei dispositivi
   e scegli **DumbMonit Wall**. Il suono esce dalla TV, e il wall mostra
   la copertina, il titolo e il progresso.

**Impostazioni › Schermo a parete › Musica per la parete › Speaker** mostra se tutto funziona, senza
andare alla TV: l'account, Premium, se Spotify elenca lo speaker tra i
dispositivi dell'account in questo momento (e quali dispositivi
elenca), e ogni wall che ha riportato nei quindici minuti precedenti —
il suo browser ("Chrome 130 on Linux"), se è pronto, e perché no.
**Test sound** fa suonare l'account sullo speaker (riprende ciò che
suonava l'ultima volta); ogni wall pronto ha anche il proprio
**Test sound**. Sul wall stesso, il pannello **Music** ha
**Play here**, che sposta ciò che suona sull'account a questo display.

**Il nome.** Rinomina lo speaker in **Impostazioni › Schermo a parete › Musica per la parete › Speaker**
("Living room TV"); ogni wall raccoglie il nuovo nome entro secondi. Per
nominare un solo wall, aggiungi `?speaker=Kitchen` al suo indirizzo — il
display lo ricorda (`?speaker=` vuoto lo dimentica).

**Un wall resta acceso per giorni.** Il wall chiede a DumbMonit un token
fresco quando quello di Spotify scade (ogni ora); quando il dispositivo
cade (Wi-Fi, una scheda di rete in sospensione) e non ritorna entro
trenta secondi, il wall si registra di nuovo; un fallimento passeggero
viene ritentato da solo (15 s, 30 s, … poi ogni cinque minuti); e se
Spotify smette di elencare il wall mentre pensa di essere pronto, si
registra di nuovo. Solo ciò che l'attesa non può risolvere — nessun DRM,
nessun Premium — si ferma per sempre, finché **Try again**.

DumbMonit mantiene il token di refresh di Spotify cifrato con il segreto
dell'istanza e non lo invia mai a un browser. Il wall riceve solo un
token di accesso a breve durata per il suo player, e solo tramite una
sessione connessa (un token API non può ottenerne uno). **Disconnect**
scorda l'account; per rimuovere anche DumbMonit dal tuo account
Spotify, usa [spotify.com/account/apps](https://www.spotify.com/account/apps/).

Buono anche sapere:

- Un'app Spotify personale (*development mode*) appartiene a un account
  Premium e ne ammette fino a cinque: aggiungi altri sotto
  **User Management** nella dashboard.
- **Riconnetti ogni sei mesi.** Spotify termina una connessione sei mesi
  dopo l'approvazione; Settings mostra la data, e il wall lo dice quando
  accade.

**Now playing funziona senza nulla di tutto ciò.** Il server di
DumbMonit chiede a Spotify cosa suona sull'account ogni pochi secondi
(una richiesta per tutti i wall) e il wall mostra la copertina, il
titolo, l'artista, il progresso e il dispositivo su cui suona — il wall
stesso, l'app Spotify di una smart TV, un Chromecast, il tuo laptop —
anche su HTTP semplice e in un browser che non può riprodurre Spotify.
Per usare solo questo, disattiva **Use this display as a Spotify
speaker** nel pannello Music del wall.

#### Quando il browser della TV non può riprodurre Spotify {#when-the-tvs-browser-cannot-play-spotify}

Scegli ciò che corrisponde alla tua configurazione — in ogni caso il
wall continua a mostrare cosa suona, perché *now playing* legge
l'account, non lo speaker:

- **La TV ha un'app Spotify, o un Chromecast / Google TV / Fire TV
  stick.** Scegli quella nell'elenco dispositivi di Spotify; il wall
  mostra "Playing on Living room TV". Questa è l'opzione più semplice e
  affidabile.
- **Un piccolo computer guida la TV** (un Raspberry Pi, un mini PC, un
  vecchio laptop). Apri il wall in Chrome o Chromium con Widevine
  installato: diventa lo speaker come descritto sopra.
- **Vuoi uno speaker sempre presente, anche senza un browser.** Esegui
  un receiver Spotify Connect basato su
  [librespot](https://github.com/librespot-org/librespot)
  (MIT) — per esempio [raspotify](https://github.com/dtcooper/raspotify)
  oppure spotifyd — **sulla macchina la cui uscita audio raggiunge la
  TV o lo speaker**. Appare nell'elenco dispositivi del telefono per
  chiunque sulla rete.

DumbMonit non fornisce un tale receiver da solo, di proposito: il
container DumbMonit di solito funziona su un server in un armadio, non
sulla macchina collegata alla TV, quindi il suo suono uscirebbe dalla
scatola sbagliata; un receiver ha bisogno della scheda audio dell'host e
del networking dell'host (multicast DNS) che un container di
monitoraggio non dovrebbe avere; e librespot dipende da protocolli
Spotify non ufficiali che si rompono di tanto in tanto. Un receiver
installato sulla macchina giusta, accanto a DumbMonit, fa il lavoro
meglio.

### Un link inviato a ogni wall

In **Impostazioni › Schermo a parete › Musica per la parete** (oppure il pannello proprio **Music** del
wall), incolla un link Spotify, Deezer o YouTube a una traccia, album,
playlist, episodio o video e scegli **Play on the wall**. Ogni wall lo
riproduce nel player embedded ufficiale del servizio; un link inviato
mentre un wall è aperto si avvia da solo dove il servizio lo consente
(YouTube, Deezer) una volta che il wall è stato toccato. **Stop** lo
rimuove da ogni wall.

Deezer e YouTube non hanno un equivalente di Spotify Connect che una
pagina web possa usare — Deezer non accetta più nuove app di sviluppatori
e la sua API non ha mai esposto cosa sta suonando — quindi un link è il
modo per loro. I player embedded riproducono anteprime di 30 secondi a
meno che il browser del wall non sia connesso al servizio.

### Privacy

Nulla di terze parti si carica sul wall finché Spotify non è collegato o
un link non è impostato. La content security policy della pagina
consente esattamente i tre player embedded, lo script player di Spotify
(`sdk.scdn.co/spotify-player.js`) e il suo frame, e cover da
`i.scdn.co` — nulla altro, e nulla fuori dall'interfaccia web.

La politica dei permessi (*Permissions-Policy*) dell'interfaccia concede la
riproduzione automatica e i contenuti protetti (DRM) solo a quei lettori; le
pagine di stato pubbliche non ricevono nessuno dei due.

### API

| Rotta | Chi | Cosa |
|---|---|---|
| `GET /api/music/now` | qualsiasi sessione o token | Cosa suona sull'account Spotify (in cache per pochi secondi) e il link che i wall riproducono |
| `PUT /api/music/link` `{"link": "…"}` | admin | Ogni wall riproduce questo link |
| `DELETE /api/music/link` | admin | I wall fermano il player embedded |
| `GET /api/music/spotify` | qualsiasi sessione o token | L'account collegato: stato, Client ID, data di riconnessione — mai un token |
| `POST /api/music/spotify/authorize` | sessione admin | Avvia una connessione: `{"client_id", "redirect_uri"}` → l'URL di approvazione di Spotify |
| `POST /api/music/spotify/complete` `{"url": "…"}` | sessione admin | Termina una connessione loopback con l'indirizzo su cui il browser è finito |
| `GET /api/music/spotify/callback` | il browser dell'admin | Il redirect diretto di Spotify (installazioni HTTPS): passa il codice alla pagina delle impostazioni, che completa la connessione con la propria sessione |
| `DELETE /api/music/spotify` | admin | Disconnette e scorda i token |
| `GET /api/music/spotify/token` | solo sessione browser | Un token di accesso a breve durata per il player del wall |
| `GET /api/music/speaker` | qualsiasi sessione o token | Lo speaker: nome, Premium, i dispositivi Spotify Connect dell'account, se lo speaker è elencato, cosa ha riportato ogni wall |
| `PUT /api/music/speaker` `{"name": "…"}` | admin | Rinomina lo speaker (`null` torna a "DumbMonit Wall") |
| `POST /api/music/speaker/report` | solo sessione browser | Un wall riporta il suo speaker (inviato ogni minuto); la risposta dice se Spotify lo elenca |
| `POST /api/music/speaker/play` `{"device_id": "…"}` | solo sessione browser | Riproduce l'account su quel dispositivo, oppure sul dispositivo nominato come lo speaker ("Test sound", "Play here") |
| `POST /api/music/spotify/playback` `{"playing": false}` | solo sessione browser | Mette in pausa (`false`) o riprende (`true`) l'account ovunque suoni; `can_control` in `GET /api/music/now` dice se è consentito |
