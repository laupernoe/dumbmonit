# Pagine di stato

![La pagina Status: le tue pagine, e gli annunci sotto di esse](../assets/screenshots/status-light.png){ loading=lazy }

Una pagina di stato è la faccia pubblica del tuo monitoraggio: una
pagina che chiunque può aprire — nessun accesso, nessun cookie — che
dice se i tuoi servizi sono attivi, mostra fino a 90 giorni di
cronologia giornaliera per ciascuno di essi, porta i tuoi annunci di
incidenti e manutenzione, e permette ai visitatori di seguirli via RSS o
email. I badge mettono la stessa risposta in un README; una vista
compatta si adatta a una pagina intranet.

Mostra solo ciò che ci metti: l'etichetta che dai a ogni servizio, il
suo stato, la sua disponibilità e il suo tempo di risposta. Nomi di
dispositivo, indirizzi, tipi e identificatori non lasciano mai il
server — non nella pagina, non in un badge, non in un'email.

## 1. Creare una pagina

**Status** nella barra superiore → **New page**. Quella pagina elenca
le tue pagine di stato (apri, modifica, elimina) con gli annunci sotto
di esse; l'editor si apre sulla propria rotta (`/status/new`,
`/status/<id>`). Il rendering pubblico è su `/s/<address>`.

| Campo | Cosa fa |
| --- | --- |
| **Title** | Il titolo della pagina ("Home lab"). |
| **Address** | L'ultima parte dell'URL, `/s/<address>`. Suggerita dal titolo; lettere minuscole, cifre e trattini, da 2 a 40 caratteri. |
| **Description** | Una frase sotto il titolo. Opzionale. |
| **Theme** | Day, night, oppure segui il sistema del visitatore. |
| **History** | La finestra più lunga che la barra della cronologia copre (30, 60 o 90 giorni). La barra si adatta all'età dei dati della pagina: mostra tanti giorni quanti il servizio più vecchio è stato monitorato, da 7 giorni fino a questa impostazione. Una pagina non dice mai più di questo — vedi [Badge](#badges). |
| **Simple mode** | Una pagina semplice: nessuna scena, nessuna mascotte, nessuna animazione e nessun easter egg. Chiaro o scuro segue il sistema del visitatore, con un interruttore nell'intestazione; l'impostazione Theme della pagina viene ignorata. |
| **Published** | Off = bozza: la pagina, i suoi badge e il suo logo rispondono "not found" ai visitatori finché non la attivi. |

Poi spunta i dispositivi da mostrare. Per ciascuno, imposta
l'**etichetta** che i visitatori leggeranno (per impostazione
predefinita il nome del dispositivo — potresti preferire "Website" a
"nginx-front-01") e, opzionalmente, un **gruppo** ("Network",
"Storage"). I gruppi diventano blocchi sulla pagina, nell'ordine in cui
disponi i servizi.

Salva, e il link è pronto da copiare. La pagina si aggiorna da sola
ogni minuto.

### Look

La sezione **Look** dell'editor veste la pagina con i tuoi colori senza
aprirla a codice arbitrario:

| Impostazione | Cosa fa |
| --- | --- |
| **Logo** | PNG, JPEG o WebP, fino a 256 KiB, mostrato accanto al titolo. Quadrato funziona meglio. SVG viene rifiutato: può portare script, e la pagina è servita dallo stesso indirizzo dell'amministrazione. Il file viene verificato per il suo contenuto, memorizzato sotto `/data/status-pages/`, e servito solo come immagine. |
| **Accent** | Ink (predefinito), blue, teal, violet, rose o amber — la riga superiore, i link e il pulsante di iscrizione. Ognuno è scelto per restare leggibile di giorno e di notte. |
| **Organisation website** | Un link `http://` o `https://` mostrato accanto al titolo, come il nome host del sito. |
| **Footer text** | Testo semplice sotto la pagina, fino a 280 caratteri: chi la gestisce, come contattarli. Le interruzioni di riga vengono conservate; nulla viene interpretato come HTML o Markdown. |

### Scene

L'impostazione **Scene** disegna uno skyline cittadino dietro il
banner. Le nuove pagine partono con Parigi, e le pagine esistenti che
non avevano una scena ora la mostrano anch'esse; deselezionala per un
banner semplice, oppure attiva **Simple mode** per eliminare le scene
del tutto. Scegli quante ne vuoi, nell'ordine che vuoi. Città: Venezia,
Parigi, Tokyo, New York, Londra, Roma, Sydney, Dubai, San Francisco,
Barcellona, Amsterdam, Istanbul e Rio de Janeiro. Le nuove sette
meraviglie: Chichén Itzá, Machu Picchu, la Grande Muraglia, Petra e il
Taj Mahal (il Cristo Redentore di Rio è nella scena di Rio). Le
meraviglie antiche: Giza, Babilonia, Efeso, Olimpia, Alicarnasso, Rodi e
Alessandria, più l'Acropoli di Atene. Con più di una, **Rotation**
decide quando la pagina passa alla successiva: a ogni visita
(predefinito), ogni minuto, ogni dieci minuti o ogni ora. Le scene sono
un insieme chiuso di disegni forniti con DumbMonit — nulla viene
caricato o interpretato.

Non c'è deliberatamente alcun CSS o HTML personalizzato: una pagina di
stato è pubblica e vive sulla stessa origine della tua interfaccia di
amministrazione, sotto la stessa
[content security policy](../reference/api.md).

## 2. Cosa vedono i visitatori

- Un banner con la risposta in un secondo, letta dai servizi: **All
  systems operational**, **Partial outage** (alcuni inattivi o
  degradati), **Major outage** (tutti inattivi) oppure **Scheduled
  maintenance**. Un incidente aperto mentre ogni servizio risponde
  ancora legge **Incident in progress**, tonalizzato dal suo impatto.
- Gli annunci aperti, l'ultimo aggiornamento prima, con la cronologia
  degli aggiornamenti sotto una piega.
- Ogni servizio con la sua targhetta di stato (Operational, Degraded,
  Down, Maintenance, No data), la sua **barra della cronologia** — un
  quadrato per giorno — la sua disponibilità su 24 h, 7 giorni, 30
  giorni e (su una pagina a 90 giorni) 90 giorni, e il tempo di
  risposta per le sonde di disponibilità.
- **Past incidents**, per giorno, per gli ultimi 30 giorni.
- **Get updates**: il feed RSS, e un modulo di iscrizione email quando
  ne configuri uno (vedi [Iscritti email](#email-subscribers)).

### La barra della cronologia

Ogni quadrato è un giorno (UTC), tonalizzato dalla disponibilità di
quel giorno: colore pieno al 99,5% e oltre, ambra dal 95%, rosso sotto.
**Un giorno senza alcuna misurazione è grigio**, non verde — un
servizio aggiunto la settimana scorsa ha 83 giorni grigi, non 83
perfetti. Passa sopra un quadrato, oppure metti a fuoco la barra e
muoviti con i tasti freccia (Home e End saltano agli estremi), per
leggere la data, la disponibilità, i **minuti di inattività** e gli
incidenti che hanno toccato quel giorno. Le parole sono lì anche per
gli screen reader: lo stato non viene mai detto solo dal colore.

Come viene deciso lo stato:

| Stato | Significato |
| --- | --- |
| Operational | L'ultima sonda è riuscita (sonde di disponibilità), oppure il dispositivo ha riportato entro tre periodi di sondaggio. |
| Degraded | Attivo ora, ma almeno una sonda è fallita nell'ultima ora. |
| Down | L'ultima sonda è fallita, il dispositivo ha smesso di riportare, oppure la sua configurazione è in errore. |
| Maintenance | Una finestra di manutenzione è in corso e il servizio è inattivo: previsto, non un'interruzione. Sia un annuncio su questa pagina, sia una [finestra di manutenzione](../alerting/maintenance.md) di alerting che copre quel dispositivo. |
| No data | Non ancora sondato, oppure disattivato. |

La disponibilità per le sonde (`http`, `tcp`, `dns`, `ping`, `tls`,
heartbeat) è la quota di controlli riusciti, da
`dumbmonit_probe_success`; l'inattività di un giorno è i suoi controlli
falliti moltiplicati per l'intervallo di sonda. Per altri dispositivi
(SNMP, agent, Proxmox…) è la quota di slot di cinque minuti in cui il
dispositivo ha riportato almeno una volta (`dumbmonit_up`), contata
dalla prima misurazione — un dispositivo aggiunto ieri non è "down" per
gli 89 giorni precedenti — e l'inattività di un giorno sono i suoi slot
silenziosi.

## 3. Annunciare incidenti e manutenzione

Sulla pagina **Status**, sotto l'elenco delle pagine,
**Incidents and maintenance** → **New announcement** (anche ++ctrl+k++
→ *Announce an incident*).

- Un **incidente** ha un titolo, un impatto (**minor** mostra la pagina
  come degradata, **major** come un'interruzione) e passa attraverso
  *Investigating → Identified → Monitoring → Resolved*. Pubblica
  aggiornamenti man mano: ciascuno porta uno stato e un messaggio, e
  diventa l'ultima riga che i visitatori leggono. **Resolve** lo chiude
  con un messaggio finale; si sposta allora su "Past incidents" per 30
  giorni.
- Una finestra di **manutenzione** ha un inizio e una fine e passa
  attraverso *Scheduled → In progress → Completed*. Mentre è in corso
  il banner dice "Scheduled maintenance" e i servizi inattivi appaiono
  come *Maintenance*.

Una [finestra di manutenzione pianificata in Alerts](../alerting/maintenance.md)
fa la stessa cosa per il singolo dispositivo che copre, senza un
annuncio: quel servizio legge *Maintenance* invece di rosso mentre la
finestra è aperta, e lo stato generale della pagina dice *Maintenance*
quando nulla altro è inattivo o degradato. Solo le finestre che
nominano un dispositivo emergono in questo modo; il nome e il commento
della finestra restano privati.

Un annuncio viene mostrato su una pagina oppure su **tutte le pagine**.

### Iscritti email

I visitatori possono iscriversi ai tuoi annunci via email una volta che
scegli un **canale email (SMTP)** nell'editor, sotto **Email
subscribers**. Il canale è uno che hai già configurato sotto
[Notifiche](../alerting/notifications.md); il suo server e mittente
vengono riutilizzati, i suoi propri destinatari no. Senza uno, la pagina
offre solo il suo feed RSS.

- **Double opt-in.** Un visitatore inserisce un indirizzo; DumbMonit
  invia un link di conferma via email, e nulla altro viene inviato
  finché non viene seguito. Una richiesta mai confermata viene scordata
  dopo due giorni. Il modulo risponde con la stessa frase che
  l'indirizzo sia nuovo, in attesa o già iscritto, così non dice a
  nessuno chi segue la tua pagina.
- **Cosa viene inviato.** Un'email quando un annuncio viene creato,
  quando un aggiornamento viene pubblicato, e quando il suo stato
  cambia — per incidenti e manutenzione allo stesso modo, su quella
  pagina (o su tutte le pagine, per un annuncio mostrato su tutte).
- **Disiscriversi.** Ogni email termina con un link per disiscriversi,
  e porta gli header `List-Unsubscribe` e `List-Unsubscribe-Post`, così
  Gmail, Apple Mail e altri offrono il loro proprio pulsante a un clic.
- **Limiti.** L'endpoint pubblico di iscrizione accetta cinque
  richieste per indirizzo client e sessanta in totale per 15 minuti, e
  un indirizzo in attesa non viene mailato due volte entro dieci
  minuti: la pagina non può essere trasformata in un cannone di mail.
- L'editor elenca gli iscritti (confermati o in attesa) e li rimuove
  uno per uno.

I link in queste email puntano al [dominio personalizzato](#custom-domain)
della pagina quando ne ha uno (`https://status.example.com/`).
Altrimenti sono costruiti dall'**URL pubblico** della
[politica di notifica](../alerting/notifications.md) (oppure
`DUMBMONIT_PUBLIC_URL`); senza uno, dall'indirizzo che hai usato
l'ultima volta che hai salvato la pagina — mai dalla richiesta di un
visitatore.

## 4. Condividere il link

La pagina vive su `https://il-tuo-server/s/<address>`. Tutto sotto
`/s/` e `/api/public/` viene servito senza autenticazione; il resto di
DumbMonit resta dietro l'accesso.

Il pannello **Share** dell'editor costruisce gli snippet sotto per te,
con un'anteprima.

### Badge

Ogni pagina pubblicata serve badge SVG — piatti, con testo bianco su
colore solido, leggibili sia sul tema chiaro che su quello scuro di un
README GitHub:

| Badge | URL |
| --- | --- |
| Stato della pagina | `/api/public/status/<address>/badge.svg` |
| Disponibilità della pagina | `/api/public/status/<address>/uptime.svg?days=30` |
| Tempo di risposta della pagina | `/api/public/status/<address>/response.svg` |
| Stato del servizio | `/api/public/status/<address>/components/<service>/badge.svg` |
| Disponibilità del servizio | `/api/public/status/<address>/components/<service>/uptime.svg?days=30` |
| Tempo di risposta del servizio | `/api/public/status/<address>/components/<service>/response.svg` |

`<service>` è l'etichetta del servizio in forma URL: "Public API"
diventa `public-api` (un secondo "Public API" sarebbe `public-api-2`).
È anche la `key` di ogni servizio nel documento JSON. `days` è `1`,
`7`, `30` (predefinito) o `90`, e mai più della cronologia della
pagina: una pagina a 30 giorni rifiuta un badge a 90 giorni. La
disponibilità e il tempo di risposta della pagina sono la media sui
suoi servizi. Un servizio senza nulla misurato dice **no data**, in
grigio.

In un README:

```markdown
[![Status](https://your-server/api/public/status/home-lab/badge.svg)](https://your-server/s/home-lab)
[![Uptime](https://your-server/api/public/status/home-lab/uptime.svg?days=30)](https://your-server/s/home-lab)
[![API](https://your-server/api/public/status/home-lab/components/public-api/uptime.svg)](https://your-server/s/home-lab)
```

I badge vengono messi in cache per un minuto
(`Cache-Control: public, max-age=60`), così il proxy immagini di
GitHub e i tuoi visitatori non costano nulla al database delle serie
temporali.

### Embed

`/s/<address>/embed` è una vista compatta per un iframe: lo stato
generale e una riga per servizio con il suo stato e 30 giorni di
cronologia, e un link alla pagina completa in una nuova tab.

```html
<iframe src="https://your-server/s/home-lab/embed"
        title="Home lab status" width="100%" height="320" style="border:0"></iframe>
```

Aggiungi `?theme=light` oppure `?theme=dark` per corrispondere alla
pagina ospitante, e `?history=0` per nascondere le barre. La pagina di
stato e il suo embed sono i soli indirizzi di DumbMonit che una pagina
di terze parti può incorniciare; tutto il resto (incluse le pagine di
conferma e disiscrizione) risponde con `frame-ancestors 'none'` e
`X-Frame-Options: DENY`.

### Dominio personalizzato

Una pagina può avere il proprio indirizzo, come
`status.example.com`, nel modo in cui lo fa Uptime Kuma. Digita il nome
nel campo **Public domain** dell'editor (la **i** accanto riassume i
passaggi) e salva. Quando una richiesta raggiunge DumbMonit con quel
nome nel suo header `Host`, DumbMonit mostra la pagina alla radice `/`,
e nulla altro: su quel nome la schermata di accesso, le impostazioni,
l'API, gli endpoint degli agent, le metriche e le altre pagine di stato
rispondono tutte `404 Not found`, anche per qualcuno connesso, e nessun
cookie di sessione viene letto o impostato. Solo la pagina, il suo
embed, le sue pagine di conferma e disiscrizione, i suoi badge, feed e
documento JSON, e i file propri dell'interfaccia (`/_app/`, icone)
vengono serviti.

Il nome viene memorizzato in minuscolo, senza `https://` o una barra
finale; una porta, un percorso o un indirizzo IP vengono rifiutati. Due
pagine non possono condividere un nome, e l'indirizzo che usi per
raggiungere DumbMonit stesso (oppure il suo URL pubblico) non può essere
il dominio di una pagina — salvarlo ti chiuderebbe fuori. Svuota il
campo per rimuovere il dominio; la pagina resta su `/s/<address>` in
entrambi i casi.

Per configurarlo:

1. **DNS.** Crea il record per il nome (oppure l'hostname pubblico di
   un Cloudflare Tunnel), puntando sul tuo reverse proxy.
2. **Proxy.** Invia il nome all'indirizzo di DumbMonit, per esempio
   `http://dumbmonit:8080`, **senza riscrivere il percorso**: il proxy
   inoltra `/`, non `/s/<address>`.
3. **Host header.** Passa avanti l'header `Host` originale: è così che
   DumbMonit distingue il dominio della pagina dal proprio indirizzo.
   DumbMonit legge solo `Host`; `X-Forwarded-Host` conta solo da un
   proxy elencato in `DUMBMONIT_TRUSTED_PROXIES`.
4. **HTTPS** è gestito dal proxy. DumbMonit collega a
   `https://<domain>/` nelle email agli iscritti, nel feed RSS e nel
   pannello **Share** dell'editor.

Traefik (label Docker) — l'header `Host` viene passato avanti per
impostazione predefinita:

```yaml
labels:
  - traefik.enable=true
  - traefik.http.routers.status.rule=Host(`status.example.com`)
  - traefik.http.routers.status.entrypoints=websecure
  - traefik.http.routers.status.tls.certresolver=letsencrypt
  - traefik.http.services.status.loadbalancer.server.port=8080
```

Nginx Proxy Manager — aggiungi un **Proxy Host** con il dominio
`status.example.com`, schema `http`, forward hostname `dumbmonit`,
porta `8080`, e un certificato SSL. Nulla altro: passa `Host` avanti
per impostazione predefinita. Lascia vuote le **Custom locations**.

Cloudflare Tunnel — aggiungi un **Public hostname**
`status.example.com` con il servizio `http://dumbmonit:8080` e nessun
percorso. Il tunnel mantiene l'`Host` del visitatore; lascia vuoto
**HTTP Host Header** nelle impostazioni avanzate.

Caddy — `reverse_proxy` passa avanti `Host` e Caddy recupera il
certificato:

```caddyfile
status.example.com {
    reverse_proxy dumbmonit:8080
}
```

Nginx — `proxy_pass` invia il nome proprio dell'upstream a meno che non
sia detto diversamente, quindi imposta l'header:

```nginx
server {
    listen 443 ssl;
    server_name status.example.com;
    # ssl_certificate … ;

    location / {
        proxy_pass http://dumbmonit:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

**Risoluzione dei problemi.**

- *Il dominio mostra la schermata di accesso o la dashboard di
  DumbMonit*: l'header `Host` non ha raggiunto DumbMonit — il proxy lo
  ha sostituito con il nome dell'upstream (`dumbmonit:8080`). Aggiungi
  `proxy_set_header Host $host;` (Nginx), rimuovi un **HTTP Host
  Header** personalizzato (Cloudflare Tunnel), oppure rimuovi un
  middleware di riscrittura `Host` (Traefik). Nulla di privato viene
  esposto: la dashboard chiede comunque una password.
- *Un semplice `Not found` sul dominio*: il proxy riscrive il percorso
  (invia `/s/<address>` oppure aggiunge un prefisso), oppure il dominio
  nell'editor non corrisponde esattamente al nome nel browser. Inoltra
  il percorso intatto.
- *La pagina dice che non esiste*: la pagina è ancora una bozza —
  pubblicala.
- *Non riesci più a raggiungere l'interfaccia di DumbMonit*: stai
  navigando tramite il dominio della pagina. Usa l'indirizzo proprio di
  DumbMonit, oppure svuota il **Public domain** della pagina.

### Dietro un reverse proxy

Un [dominio personalizzato](#custom-domain) è il modo più semplice per
esporre solo la pagina di stato. Se preferisci mantenere il percorso
`/s/<address>` su un nome pubblico, filtra invece i percorsi nel proxy.
Con Caddy, per esempio:

```caddyfile
status.example.com {
    @public path /s/* /api/public/* /_app/* /favicon.svg
    reverse_proxy @public dumbmonit:8080
    respond 404
}
```

Con nginx:

```nginx
location ~ ^/(s/|api/public/|_app/|favicon\.svg) {
    proxy_pass http://dumbmonit:8080;
    proxy_set_header Host $host;
    proxy_set_header X-Forwarded-Proto $scheme;
}
location / { return 404; }
```

`/_app/` porta gli script e gli stili dell'interfaccia: la pagina ne
ha bisogno per il rendering. Inoltra `X-Forwarded-Proto` e
`X-Forwarded-Host` così che il feed RSS collega all'indirizzo giusto, e
imposta l'URL pubblico (vedi sopra) così che anche le email agli
iscritti lo facciano.

### Feed e documento

- `GET /api/public/status/<address>/rss` — un feed RSS di incidenti e
  finestre di manutenzione, un elemento per annuncio con il suo ultimo
  aggiornamento.
- `GET /api/public/status/<address>` — il documento JSON da cui è
  costruita la pagina, se vuoi costruire il tuo: il titolo, la
  descrizione e il look della pagina, lo stato generale, la `key`, il
  `label`, lo `state` di ogni servizio, la disponibilità
  (`uptime_24h`, `uptime_7d`, `uptime_30d`, `uptime_90d`), `latency_ms`
  e `history` (`date`, `uptime_pct`, `down_minutes`, `incidents` per
  giorno), e gli annunci.

Qualsiasi sito web può leggere quel documento dal browser (viene
inviato con `Access-Control-Allow-Origin: *` e nessun cookie).
`GET /api/public/status/<address>/banner.js` è uno script pronto che lo
trasforma in un banner: vedi
[Banner per il sito web](../integrations/website-banner.md).

Il documento viene messo in cache per 30 secondi sul server; i badge,
il feed e l'embed leggono tutti quello stesso documento in cache, così
nessuno di essi può dire più della pagina.
