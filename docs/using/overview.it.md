# La pagina Overview

La pagina iniziale risponde a «va tutto bene?» in circa un secondo, poi ti lascia
approfondire a richiesta.

## Il banner di stato {#the-bulletin}

Una fascia in alto mostra un cielo notturno fisso: stelle che brillano, una luna
crescente e un piccione che lo attraversa senza mai posarsi. Il cielo non cambia
con il meteo; la frase ai suoi piedi dice come stanno le cose, a parole. «Tutti i
146 monitor sono attivi.» quando nulla richiede attenzione, altrimenti «pve-02 è
giù.» o «pve-02 merita un'occhiata.», seguito con voce più sommessa da ciò che il
sistema ha fatto («3 guest trattenuti, allarme inviato.»). Tocca il piccione e fa
una capriola. Con `prefers-reduced-motion`, il piccione esce dal cielo e le stelle
restano ferme.

## La guida al primo avvio {#the-first-run-guide}

Su un'istanza nuova, la pagina sotto il bollettino non è vuota: è una
guida in tre passaggi — aggiungi un monitor, collega un modo per
essere avvisato, verifica che arrivi un messaggio. Ogni passaggio è un
clic, ognuno può essere scartato per conto proprio, e **Skip the guide**
li elimina tutti e tre. La scelta viene memorizzata sull'istanza, così un
secondo browser non ricomincia la guida da capo. Vedi
[Aggiungi il tuo primo monitor](../install/first-device.md#the-three-first-steps).

## Le cinque cifre e il fiume di salute {#the-figures}

Sotto il banner, cinque cifre: **Monitors up** (sul totale, con i monitor giù e
in pausa), **Availability 30 d**, **p95 response time**, **Open alerts** (critici
e avvisi) e **MTTR 30 d** (tempo medio di risoluzione). L'intervallo di tempo
nell'intestazione governa il p95 e il fiume.

Il **fiume di salute** è una striscia per monitor, una cella per fetta
dell'intervallo, con i monitor peggiori in alto. Uno stato è sempre scritto
accanto a una forma (cerchio attivo, rombo degradato, quadrato giù), mai solo a
colori.

## La tabella dei monitor {#the-monitor-table}

La tabella elenca ogni monitor, il peggiore per primo. Si filtra per stato, tipo e
testo, consente di aggiungere colonne (indirizzo, cartella) o toglierne, e si usa
con la tastiera: ++j++ / ++k++ si spostano, ++enter++ seleziona, ++o++ apre.

Il monitor selezionato alimenta il **pannello di contesto** a destra, come un
inspector: ciò che richiede attenzione (con le azioni sugli allarmi, vedi
[Pagina Alerts](alerts.md)), **Check now** e **Silence 1 h**, le previsioni più
vicine e chi è reperibile. ++i++ lo mostra o lo nasconde; su uno schermo più
stretto si apre come cassetto. L'elenco completo è nella
[pagina dei monitor](devices.md).

## Modalità Wall {#wall-mode}

`/wall` mostra una lavagna per un monitor in sala: il verdetto, l'orologio, le
cinque cifre, il fiume di salute, gli allarmi aperti e chi è reperibile. Aprila
con l'icona **Wall mode** della barra in alto o con la command palette; ++esc++ o
**Exit** torna alla console. Vedi [Modalità Wall](wall.md).

## Installarla su un telefono {#install-it-on-a-phone}

L'interfaccia fornisce un manifest web, così un telefono può aggiungerla
alla schermata home: **Share → Add to Home Screen** su iOS,
**Install app** nel menu Chrome su Android. Si apre allora senza la
chrome del browser, nel tema impostato sul telefono.

Non c'è deliberatamente **nessuna modalità offline e nessun service
worker**. Uno schermo di monitoraggio che mostra una pagina in cache è
peggiore di uno che dice di non poter raggiungere il server: il punto
intero è che ciò che leggi sia attuale. Senza un worker, un server
aggiornato viene anche servito immediatamente, senza alcuna build
obsoleta da pulire.

## Command palette {#command-palette}

++cmd+k++ su macOS, ++ctrl+k++ altrove, apre la command palette: un
input, risultati in tre gruppi.

| Gruppo | Voci |
|---|---|
| Pages | Overview, Monitors, Alerts, Notifications, Status pages, Settings, Wall mode, Documentation |
| Actions | Add a monitor, Scan my network, Schedule maintenance, Add notification channel, New status page, Announce an incident |
| Monitors | Ogni monitor, con il suo LED di stato; ++enter++ lo apre |

## Navigazione {#navigation}

Una **barra di icone** sottile a sinistra elenca ogni destinazione: Overview,
Monitors, Alerts (con il conteggio di ciò che richiede attenzione), Dashboards,
Explore, la mappa della topologia, Security, Logs, Status pages, Reports e
Settings. In fondo ci sono la documentazione, GitHub e il tuo account.
L'intestazione dice dove sei, con l'interruttore **LIVE** che mette in pausa gli
aggiornamenti della console, l'intervallo di tempo, la palette, **Wall mode** e
**New monitor**. Su un telefono, la barra diventa una barra di tab in basso con
l'essenziale e **More**.

- **Overview**: va tutto bene, e cosa richiede attenzione.
- **Monitors**: ogni fonte che monitori. Vedi la
  [pagina dei monitor](devices.md).
- **Alerts**: cosa è attivo, finestre di manutenzione, cronologia e regole. Vedi
  [Pagina Alerts](alerts.md).
- **Settings**: il tuo account, utenti e single sign-on, token, canali di
  notifica, aspetto, informazioni. Vedi [Settings](settings.md).

L'interfaccia è scura e nulla vi trasmette lo stato solo con il colore: un
indicatore ha sempre una parola accanto (Reporting, Unreachable, Advisory,
Warning, Suppressed by parent, Building up).
