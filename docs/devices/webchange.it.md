# Modifiche a un sito web

Osserva il *contenuto* di una pagina web, o di un intero sito, e sii
avvisato quando cambia: una pagina di prezzi o condizioni, un sito di
documentazione, l'avviso di manutenzione di un fornitore, una pagina di
annunci pubblici, oppure il tuo stesso sito dopo un deployment. Ogni
modifica viene conservata con il testo prima e dopo, un diff per riga e,
quando un browser è disponibile, uno screenshot di ogni versione.

Ciò che viene confrontato è il **testo visibile** di ogni pagina, non il
suo HTML: script, stili, commenti, blocchi `noscript`, `svg` e `template`
vengono scartati, gli spazi bianchi vengono compattati e ogni blocco
(paragrafo, intestazione, elemento di lista, riga di tabella) diventa una
riga. Una nuova build della stessa pagina, un attributo riordinato o un
nuovo nome di asset con cache-busting non cambiano nulla; una frase
riformulata sì.

## Pagina o sito

**Page** (predefinito) legge l'indirizzo del dispositivo e nient'altro.
Può anche essere un documento di testo semplice o JSON, confrontato riga
per riga.

**Site** parte dall'indirizzo e segue ogni pagina sotto la stessa cartella
(`https://example.com/docs/intro.html` copre tutto sotto `/docs/`;
**Path prefix** scegli un'altra cartella). Le pagine vengono trovate:

1. nella sitemap del sito: quelle che `robots.txt` annuncia (righe
   `Sitemap:`), oppure `/sitemap.xml`, indici di sitemap inclusi (un
   livello);
2. in mancanza di ciò, o quando la sitemap non elenca nulla sotto la
   cartella, seguendo i link di ogni pagina, in ampiezza.

Solo le pagine della stessa origine (schema, host e porta) vengono
seguite; i link a immagini, fogli di stile, script, PDF, archivi e altri
file vengono saltati per la loro estensione, e qualsiasi risposta che non
sia HTML viene ignorata. I fragment (`#section`) vengono scartati e le
query vuote rimosse, così una pagina è un indirizzo. `robots.txt` viene
rispettato per le pagine trovate in questo modo: il gruppo che nomina
`DumbMonit`, oppure altrimenti il gruppo `*`, con `Allow`, `Disallow`, `*`
e `$`.

La scansione è educata: al massimo due richieste alla volta, mezzo secondo
tra due lotti, lo `User-Agent` `DumbMonit/<version>`, un timeout di 15
secondi per richiesta e al massimo 5 MB letti per pagina. Si ferma a
**Max pages** (25 per impostazione predefinita, 200 al massimo) oppure
dopo dieci minuti.

## Come funziona un controllo

Il dispositivo viene sondato come qualsiasi altro (ogni minuto per
impostazione predefinita), ma le pagine vengono lette solo ogni
**Check every** minuti (60 per impostazione predefinita, da 5 a 10080). Nel
frattempo, la sonda riporta solo lo stato dell'ultimo controllo: il sito
non viene colpito ogni minuto. Un controllo viene eseguito in background;
due controlli dello stesso dispositivo non si sovrappongono mai, e
**Check now** sulla pagina del dispositivo ne avvia uno all'istante.

Il primo controllo registra solo il **riferimento**: ogni pagina viene
letta e conservata, nulla viene segnalato. Da quel momento in poi, per
ogni pagina:

| Situazione | Cosa viene conservato |
|---|---|
| Stesso testo dell'ultima volta (dopo **Ignore lines**) | Nulla di nuovo; la pagina viene segnata come controllata. |
| Testo diverso | Un nuovo snapshot e un evento **changed**, con il numero di righe aggiunte e rimosse. |
| Una pagina mai vista prima, oppure vista di nuovo dopo essere scomparsa | Uno snapshot e un evento **new page**. |
| La pagina risponde 404 o 410 | Un evento **removed page**, l'ultimo snapshot come "before". |
| Ambito sito: una scansione completa non raggiunge più una pagina conosciuta | Anche un evento **removed page** ("No longer linked"). Una scansione interrotta da **Max pages** o dal suo limite di tempo non conclude mai che una pagina sia scomparsa. |
| Qualsiasi altro errore su una pagina (timeout, 403, 500 su una sottopagina) | L'errore viene mostrato sulla pagina; nessun evento. |

Cambiare l'indirizzo, l'ambito, il prefisso di percorso o **Ignore lines**
rende il controllo successivo un nuovo riferimento invece di segnalare
ogni pagina come cambiata.

### Ignore lines

Le righe che cambiano a ogni visita — un orologio, un contatore di
visitatori, "aggiornato 5 minuti fa", una citazione casuale — verrebbero
segnalate a ogni controllo. **Ignore lines** contiene espressioni
regolari (sintassi Rust `regex`), una per riga o unite con `|`; qualsiasi
riga di testo che una di esse corrisponde viene esclusa prima del
confronto. Per esempio:

```
Updated \d+ minutes ago|Visitors: \d+|© \d{4}
```

### Pagina iniziale irraggiungibile

Quando la pagina iniziale stessa non può essere letta (nessuna risposta,
connessione rifiutata, timeout, HTTP 5xx), il dispositivo è
**unreachable**: la sonda fallisce, e la regola "Device unreachable" si
attiva come per qualsiasi dispositivo. La pagina iniziale viene allora
ritentata a ogni sonda, così il suo ritorno viene visto entro un minuto.
Una pagina iniziale che rifiuta l'accesso (401, 403) viene segnalata come
problema di configurazione, non come interruzione. Gli errori su altre
pagine del sito vengono mostrati solo su quelle pagine.

Gli indirizzi che solo l'host DumbMonit stesso può raggiungere (loopback,
link-local, il servizio di metadata cloud) vengono rifiutati a meno che
**Allow loopback and link-local targets** non sia abilitato; gli indirizzi
di LAN privata sono sempre consentiti.

## Avvisi

La regola integrata **Website changed** (`webchange_detected`, Info) si
attiva dopo un controllo che ha trovato almeno una modifica (changed, new
o removed page), e si risolve al controllo successivo che non ne trova
nessuna. Legge `dumbmonit_webchange_last_check_changes`; collegala a un
canale come qualsiasi altra regola, oppure alza la sua severità, in
**Alerts → Rules**. Vedi
[Regole integrate](../alerting/rules.md#website-changes).

| Metrica | Cosa |
|---|---|
| `dumbmonit_webchange_pages` | Pagine seguite (pagine rimosse escluse). |
| `dumbmonit_webchange_pages_failed` | Pagine seguite la cui ultima lettura è fallita. |
| `dumbmonit_webchange_last_check_changes` | Modifiche trovate dall'ultimo controllo. |

Le metriche appaiono una volta eseguito il primo controllo.

## Screenshot {#screenshots}

L'immagine DumbMonit non ha un browser: gli screenshot provengono da un
Chromium headless opzionale, guidato tramite il Chrome DevTools Protocol.
Aggiungine uno come servizio `browser` accanto a DumbMonit in
`docker-compose.yml` e punta il server verso di esso:

```yaml
services:
  dumbmonit:
    environment:
      DUMBMONIT_BROWSER_URL: http://browser:9222
  browser:
    image: chromedp/headless-shell:stable
    shm_size: 256m
    restart: unless-stopped
```

Poi esegui `docker compose up -d`. Nulla pubblica la porta del browser: solo il
server DumbMonit gli parla, sulla rete Compose.

`DUMBMONIT_BROWSER_URL` punta DumbMonit verso il browser (qualsiasi
Chromium avviato con `--remote-debugging-port`, per esempio
`chromedp/headless-shell`; il server legge `/json/version` e parla con
l'indirizzo a cui risolve il nome host).

Uno screenshot viene preso solo quando una pagina riceve un nuovo
snapshot (il riferimento, e ogni modifica), mai su un controllo invariato:
una finestra 1366 × 900, la pagina intera fino a 5000 pixel, come JPEG.
L'opzione **Screenshots** li disattiva per un dispositivo. Senza un
browser, oppure quando non risponde, il confronto di testo funziona lo
stesso e gli snapshot semplicemente non hanno un'immagine; la pagina del
dispositivo lo dice.

Il browser carica la pagina come farebbe un visitatore, con i suoi
script: ciò che mostra può differire dal testo che DumbMonit ha
confrontato, che è l'HTML che il server ha inviato.

## Storage

Il testo di ogni snapshot viene conservato nel database (1 MB al massimo
per pagina); gli screenshot sono file sotto `/data/webchange/<device
id>/`. Per dispositivo, vengono conservate le ultime cento modifiche; uno
snapshot viene conservato mentre è il più recente di una pagina o una
modifica conservata lo referenzia, e tutto il resto, file inclusi, viene
rimosso dopo ogni controllo. Eliminare il dispositivo rimuove tutto
quanto.

## Osservare un sito web per modifiche

I passaggi seguenti sono quelli mostrati dalla nota accanto al modulo.

1. Nell'indirizzo, incolla la pagina da osservare, con http:// o https://. Per osservare un intero sito o una delle sue sezioni, incolla la sua pagina iniziale e imposta "Scope" su "site": ogni pagina sotto la stessa cartella viene seguita, fino a "Max pages".
2. Le righe che cambiano a ogni visita (un orologio, un contatore di visitatori, "aggiornato 5 minuti fa") verrebbero segnalate ogni volta: elencale in "Ignore lines" come espressioni regolari.
3. Il primo controllo registra solo il riferimento, nulla viene segnalato. Da quel momento, ogni controllo confronta il testo visibile pagina per pagina e conserva una copia prima/dopo di ogni modifica, mostrata sulla pagina del dispositivo; l'avviso "Website changed" te lo dice.
4. Per uno screenshot di ogni versione, aggiungi un servizio browser headless al tuo file Compose e punta DumbMonit verso di esso, come mostrato in [Screenshot](#screenshots).

!!! warning
    Viene confrontato solo il testo che il server invia: una pagina
    disegnata interamente da JavaScript mostra poco o nulla al controllo.
    Anche le pagine dietro un login non sono raggiungibili.

Credenziali: nessuna. I controlli vengono sempre eseguiti sul server
DumbMonit: un'osservazione di sito web non può passare attraverso un
agent relay.

## Opzioni

| Opzione | Predefinito | Significato |
|---|---|---|
| Scope (`scope`) | `page` | `page`: solo l'indirizzo. `site`: ogni pagina sotto la stessa cartella. |
| Max pages (`max_pages`) | `25` | Ambito sito: pagine lette al massimo per controllo, da 1 a 200. |
| Path prefix (`path_prefix`) | cartella dell'indirizzo | Ambito sito: solo le pagine il cui percorso inizia con questo (`/docs/`). |
| Ignore lines (`ignore`) | — | Espressioni regolari; le righe di testo corrispondenti non vengono confrontate. |
| Screenshots (`screenshots`) | `true` | Conserva uno screenshot di ogni nuova versione, quando un browser è configurato. |
| Check every (`check_interval_minutes`) | `60` | Minuti tra due controlli, da 5 a 10080. |
| Accept an unverifiable certificate (`insecure_tls`) | `false` | Per un sito tuo dietro un certificato autofirmato. |
| Allow loopback and link-local targets (`allow_private_targets`) | `false` | Raggiungi un sito servito dall'host DumbMonit stesso. |
