# Modalità Wall e musica

`/wall` mostra solo il bollettino, a schermo intero, per uno schermo che
resta acceso in una stanza: la frase, la finestra meteo, tre grandi
indicatori e l'elenco *Needs you*, aggiornati ogni 20 secondi, con lo
schermo mantenuto acceso. ++esc++ oppure **Exit** torna alla panoramica.
Aprila dalla command palette ("Wall mode") oppure digitando l'URL.
`?theme=dark` oppure `?theme=light` forza un tema per quel display senza
toccare la tua preferenza salvata.

![Modalità Wall](../assets/screenshots/wall-light.png){ loading=lazy }

Ogni problema a sinistra porta l'icona del prodotto del suo monitor (gli
stessi simboli degli elenchi dei monitor). L'elenco non esce mai dal suo
spazio: su uno schermo piccolo, o con molti problemi, i grandi riquadri si
riducono a una riga ciascuno, poi si raccolgono in *E altri N*, invece di
sovrapporsi ai monitor sottostanti.

## Notizie sul wall

Un amministratore può aggiungere al wall dei **titoli** e un **canale in
diretta** in **Impostazioni → Notizie del wall**. Entrambi sono disattivati
per impostazione predefinita.

- **Banda dei titoli.** Una fascia lungo il bordo inferiore del wall fa
  scorrere gli ultimi titoli, ciascuno con la sua fonte e la sua età, e si
  aggiorna ogni cinque minuti. Con il *movimento ridotto* attivo nel sistema
  del display, nulla scorre: un titolo alla volta compare e svanisce in
  dissolvenza.
- **Video in diretta.** Un canale in diretta senza audio in un riquadro, nella
  colonna destra sotto l'orologio (o al centro della scena), piccolo, medio o
  grande. Il riquadro si adatta allo spazio rimasto; non copre mai lo stato.

È il server a leggere i feed, non il browser (i browser non possono leggere i
feed di altri siti). Segue al massimo tre redirect, rifiuta gli indirizzi
interni, rinuncia dopo otto secondi o 2 MB per feed, e mette in cache la
risposta per cinque minuti per tutti i wall. Legge RSS 2.0, RSS 1.0 (RDF) e
Atom.

### Scegliere le fonti

Scegli un **paese** (preselezionato dalla regione e dalla lingua del tuo
browser), poi spunta le sue fonti: fino a otto feed in tutto. Per usare un
altro feed, incolla il suo indirizzo RSS o Atom sotto **Aggiungi un feed
personalizzato** e premi **Prova**: DumbMonit lo legge davvero e mostra
l'ultimo titolo prima che tu lo aggiunga.

Per il video, scegli uno dei canali del paese oppure **Un altro link…** e
incolla un normale indirizzo YouTube, Dailymotion o Twitch (un video, una
diretta, o la pagina `/live` di un canale YouTube); DumbMonit lo converte nel
player incorporabile. Si possono mostrare solo questi tre player. Una diretta
viene proposta come preset solo quando il canale ne ha una ufficiale e stabile.

| Paese | Titoli | Video in diretta |
|---|---|---|
| Francia | BFM TV, franceinfo, Le Monde, France 24 | BFM TV, franceinfo, France 24, Euronews |
| Svizzera | RTS Info, SRF News, Le Temps, NZZ | nessuno |
| Belgio | RTBF, VRT NWS, La DH | nessuno |
| Germania | Tagesschau, ZDFheute, Spiegel, Deutsche Welle | DW News |
| Regno Unito | BBC News, The Guardian | Sky News |
| Stati Uniti | NPR, PBS NewsHour, CBS News, The New York Times | ABC News Live |
| Canada | Radio-Canada, CBC News | CBC News |
| Spagna | RTVE, 20minutos, El País, El Mundo | nessuno |
| Italia | ANSA, RaiNews | nessuno |
| Portogallo | RTP Notícias, Público | nessuno |
| Brasile | G1, Folha de S.Paulo | CNN Brasil |
| Russia | Interfax, Meduza, Kommersant | nessuno |
| Cina | CGTN, South China Morning Post, Xinhua | CGTN |
| Internazionale | Deutsche Welle, France 24, BBC News (world) | DW News, France 24 English, Euronews English |

Il feed di BFM TV è `https://www.bfmtv.com/rss/news-24-7/`. L'elenco è stato
verificato al momento della stesura; un sito può spostare il proprio feed in
qualsiasi momento, e allora la banda lo salta semplicemente. Usa **Prova** per
il sostituto e aggiungilo come feed personalizzato.

La content security policy del wall consente alla pagina di incorporare solo i
player di YouTube (`youtube-nocookie.com`), Dailymotion, Twitch, Spotify e
Deezer.

### API

`GET /api/wall/news` (ciò che interrogano i wall), `GET|PUT
/api/wall/news/settings` e `POST /api/wall/news/test` (admin), `GET
/api/wall/news/presets`.

## Musica sul wall

Il wall può anche riprodurre musica e mostrare cosa sta suonando, così
puoi avviare una playlist dal tuo telefono, sentirla dalla TV, e vederla
dall'altra parte della stanza. La musica sta sempre nell'angolo in basso
della finestra meteo, sopra il cielo: non ruba mai spazio al bollettino o
all'elenco *Needs you*, e scompare quando nulla suona. Ogni scheda si
riduce a una piccola pillola (il display lo ricorda).

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
   aggiungi l'URI di redirect che **Settings → Wall music** mostra:
    - DumbMonit servito su HTTPS: `https://<il tuo indirizzo>/api/music/spotify/callback`.
      Spotify torna direttamente a DumbMonit dopo che approvi.
    - DumbMonit servito su HTTP semplice: `http://127.0.0.1:8888/callback`.
      Spotify accetta `http://` solo per un indirizzo loopback (e non
      `localhost`), dal 2025. Nulla risponde a quell'indirizzo: dopo
      l'approvazione, il tuo browser mostra una pagina di errore il cui
      indirizzo porta il codice — copialo e incollalo di nuovo in
      DumbMonit.
2. **Collega.** In **Settings → Wall music**, incolla il **Client ID**
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

**Settings → Wall music → Speaker** mostra se tutto funziona, senza
andare alla TV: l'account, Premium, se Spotify elenca lo speaker tra i
dispositivi dell'account in questo momento (e quali dispositivi
elenca), e ogni wall che ha riportato nei quindici minuti precedenti —
il suo browser ("Chrome 130 on Linux"), se è pronto, e perché no.
**Test sound** fa suonare l'account sullo speaker (riprende ciò che
suonava l'ultima volta); ogni wall pronto ha anche il proprio
**Test sound**. Sul wall stesso, il pannello **Music** ha
**Play here**, che sposta ciò che suona sull'account a questo display.

**Il nome.** Rinomina lo speaker in **Settings → Wall music → Speaker**
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

In **Settings → Wall music** (oppure il pannello proprio **Music** del
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

### API

| Rotta | Chi | Cosa |
|---|---|---|
| `GET /api/music/now` | qualsiasi sessione o token | Cosa suona sull'account Spotify (in cache per pochi secondi) e il link che i wall riproducono |
| `PUT /api/music/link` `{"link": "…"}` | admin | Ogni wall riproduce questo link |
| `DELETE /api/music/link` | admin | I wall fermano il player embedded |
| `GET /api/music/spotify` | qualsiasi sessione o token | L'account collegato: stato, Client ID, data di riconnessione — mai un token |
| `POST /api/music/spotify/authorize` | sessione admin | Avvia una connessione: `{"client_id", "redirect_uri"}` → l'URL di approvazione di Spotify |
| `POST /api/music/spotify/complete` `{"url": "…"}` | sessione admin | Termina una connessione loopback con l'indirizzo su cui il browser è finito |
| `GET /api/music/spotify/callback` | il browser dell'admin | Il redirect diretto di Spotify (installazioni HTTPS) |
| `DELETE /api/music/spotify` | admin | Disconnette e scorda i token |
| `GET /api/music/spotify/token` | solo sessione browser | Un token di accesso a breve durata per il player del wall |
| `GET /api/music/speaker` | qualsiasi sessione o token | Lo speaker: nome, Premium, i dispositivi Spotify Connect dell'account, se lo speaker è elencato, cosa ha riportato ogni wall |
| `PUT /api/music/speaker` `{"name": "…"}` | admin | Rinomina lo speaker (`null` torna a "DumbMonit Wall") |
| `POST /api/music/speaker/report` | solo sessione browser | Un wall riporta il suo speaker (inviato ogni minuto); la risposta dice se Spotify lo elenca |
| `POST /api/music/speaker/play` `{"device_id": "…"}` | solo sessione browser | Riproduce l'account su quel dispositivo, oppure sul dispositivo nominato come lo speaker ("Test sound", "Play here") |
