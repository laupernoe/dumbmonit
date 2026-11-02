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

## Le quattro cifre {#the-figures}

Sotto il banner ci sono quattro cifre: **Monitor attivi** (sul totale, con i conteggi di quelli giù e in pausa), **Disponibilità 30 g**, **Risposta p95** e **Avvisi aperti** (critici e di avviso). L'intervallo di tempo nell'intestazione governa il p95 e il fiume.

## Cosa richiede attenzione {#what-needs-attention}

L'elenco accanto al fiume contiene solo ciò che è rotto adesso, il peggiore per primo, ogni riga con la sua azione rapida: riconoscere o posticipare un avviso, o silenziare un monitor (la piccola croce). Senza nulla di rotto, il piccione dorme e la scheda lo dice. In fondo, la **settimana in arrivo** in una riga: ciò che scade nei prossimi sette giorni. **Vedi tutto** apre gli [Avvisi](alerts.md).

## Il fiume della salute {#the-health-river}

Una banda per monitor, una cella per fascia dell'intervallo, i monitor peggiori in alto. La panoramica mostra i primi otto; **Vedi tutti i monitor** apre la scheda [Elenco](devices.md) di Monitor. Uno stato è sempre scritto accanto a una forma (cerchio attivo, rombo degradato, quadrato giù), mai indicato dal solo colore. Su un telefono il fiume si apre con un tocco.

## Due grafici {#charts}

Più in basso: il **tempo di risposta** dell'intera flotta nell'intervallo (p95 e mediana, con i monitor più lenti) e gli **avvisi degli ultimi sette giorni** (scattati e risolti).

## Modalità Wall {#wall-mode}

`/wall` mostra una lavagna per un monitor in sala: il verdetto, l'orologio, le
cinque cifre, il fiume di salute, gli allarmi aperti e chi è reperibile. Aprila
con l'icona **Wall mode** in fondo al menu laterale o con la command palette; ++esc++ o
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
| Pages | Panoramica, Monitor, Mappa, Avvisi, Automazione, Notifiche, Esplora metriche, Sicurezza, Pagine di stato, Report, Log, Impostazioni, Modalità parete, Documentazione |
| Actions | Add a monitor, Scan my network, Schedule maintenance, Add notification channel, New status page, Announce an incident |
| Monitors | Ogni monitor, con il suo LED di stato; ++enter++ lo apre |

## Navigazione {#navigation}

Un **menu** stretto a sinistra elenca quattro destinazioni: **Panoramica**, **Monitor**, **Avvisi** (con il conteggio di ciò che richiede attenzione) e **Stato**. In fondo ci sono **Impostazioni**, la **modalità parete**, la documentazione, GitHub e il tuo account. Ciò che prima era una pagina a sé ora è una scheda di queste voci. L'intestazione indica dove sei, con l'interruttore **LIVE** che mette in pausa gli aggiornamenti della console, l'intervallo di tempo, la palette e **Aggiungi un monitor**. Su un telefono il menu diventa una barra di tab in basso con l'essenziale e **Altro**.

- **Panoramica**: va tutto bene, e cosa richiede attenzione.
- **Monitor**: ogni fonte che monitori, in quattro schede: **Elenco**, **Mappa**, **Log** e **Sicurezza**. Vedi [Monitor](../monitors/index.md).
- **Avvisi**: cinque schede: **Adesso**, **Cronologia**, **Regole**, **Automazione** e **Reperibilità**. Vedi la [pagina degli avvisi](alerts.md).
- **Stato**: **Pagine** (pagine di stato pubbliche) e **Report**. Vedi [Stato](../status/index.md).
- **Impostazioni**: il tuo account, team e accesso, monitoraggio, notifiche, la parete e il sistema. Vedi [Impostazioni](../settings/index.md).

L'interfaccia è scura e nulla vi trasmette lo stato solo con il colore: un
indicatore ha sempre una parola accanto (Reporting, Unreachable, Advisory,
Warning, Suppressed by parent, Building up).
