# La pagina di panoramica {#the-overview-page}

La home page risponde a "tutto va bene?" in circa un secondo, poi ti
lascia approfondire su richiesta.

![La panoramica](../assets/screenshots/overview-light.png){ loading=lazy }

## Il bollettino {#the-bulletin}

La fascia in alto è un bollettino meteo per la tua rete. Una frase indica
il cielo: "Clear skies." quando nulla richiede attenzione, altrimenti un
conteggio di ciò che la richiede ("9 advisories, 1 unreachable."). Sotto,
delle targhette contano i dispositivi che riportano e irraggiungibili,
con l'orario dell'ultimo controllo, e tre indicatori: **Reporting**,
**Needs you**, **Forecasts**.

La finestra meteo a destra segue la rete: sereno, nuvoloso, coperto o
tempesta, a seconda di quanti avvisi sono attivi e quanto severi, con il
piccione che vola attraverso. Sotto `prefers-reduced-motion`, tutto resta
fermo.

L'unica azione primaria della pagina è **Add a device**.

## La guida al primo avvio {#the-first-run-guide}

Su un'istanza nuova, la pagina sotto il bollettino non è vuota: è una
guida in tre passaggi — aggiungi un dispositivo, collega un modo per
essere avvisato, verifica che arrivi un messaggio. Ogni passaggio è un
clic, ognuno può essere scartato per conto proprio, e **Skip the guide**
li elimina tutti e tre. La scelta viene memorizzata sull'istanza, così un
secondo browser non ricomincia la guida da capo. Vedi
[Aggiungi il tuo primo dispositivo](../install/first-device.md#the-three-first-steps).

## Needs you {#needs-you}

L'elenco di ciò che richiede attenzione ora, il più severo prima:
dispositivi irraggiungibili, poi avvisi attivi per severità. Ogni riga
nomina la regola, il dispositivo, il valore attuale, da quanto tempo
dura, e se raggruppa diverse serie ("×4" per quattro file system sullo
stesso dispositivo). Quattro azioni: **Ack** (conferma questo avviso per
1 h, 4 h, 24 h oppure until resolved — i ricordi si fermano, la
risoluzione viene comunque notificata), **Snooze** (nascondi solo questo
avviso per 1 h, 8 h, 1 giorno oppure until resolved), **Ignore** (impedisci
che questa regola avvisi mai più su questo dispositivo, reversibilmente),
e **Open device**.

Gli avvisi confermati e sospesi si spostano nel loro gruppo più silenzioso
sotto l'elenco e lasciano il conteggio *Needs you*. Un avviso ignorato
scompare del tutto — non è "conosciuto", è disattivato per quel
dispositivo. Vedi
[Eliminare un avviso](alerts.md#getting-rid-of-an-alert).

## Riepilogo dispositivi {#devices-summary}

Una riga: quanti dispositivi, quanti riportano, quanti irraggiungibili, e
un link alla pagina Devices completa. La panoramica non elenca mai i
dispositivi: il rack vive su [Devices](devices.md).

## Previsioni e ultime 24 ore {#forecasts-and-last-24-hours}

**Forecasts** mostra ciò che è previsto piuttosto che osservato:
l'anomalia della baseline stagionale, le estrapolazioni "pieno entro
quattro giorni", la stima di riempimento di PBS, e le regole il cui nome
annuncia una previsione. Mentre una baseline sta apprendendo, le sue voci
lo dicono.

**Last 24 hours** elenca le transizioni attive della giornata, con un
link alla cronologia completa sulla pagina Alerts.

## Modalità Wall {#wall-mode}

`/wall` mostra solo il bollettino, a schermo intero, per un monitor da
stanza: la frase, i conteggi e l'elenco *Needs you*, aggiornati ogni 20
secondi, con lo schermo mantenuto acceso. ++esc++ oppure **Exit** torna
alla panoramica. Aprila dalla command palette ("Wall mode") oppure
digitando l'URL. Può anche riprodurre musica avviata dal tuo telefono:
vedi [Modalità Wall e musica](wall.md).

![Modalità Wall](../assets/screenshots/wall-light.png){ loading=lazy }

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
| Pages | Overview, Devices, Alerts, Notifications, Status pages, Settings, Wall mode, Documentation |
| Actions | Add a device, Scan my network, Schedule maintenance, Add notification channel, New status page, Announce an incident, Toggle theme |
| Devices | Ogni dispositivo, con il suo LED di stato; ++enter++ lo apre |

## Navigazione {#navigation}

La barra superiore ha Overview, Monitors, Alerts (con il conteggio di ciò
che richiede attenzione) e Settings, la scorciatoia della palette e
l'interruttore del tema. Su un telefono, diventa una barra di tab in
basso con quei quattro tab e **More**, che contiene la modalità sala e la
documentazione.

- **Overview** — il bollettino: tutto va bene, e cosa richiede
  attenzione.
- **Monitors** — il rack; ogni fonte che monitori. Vedi
  [Pagina dei dispositivi](devices.md).
- **Alerts** — cosa è attivo, le finestre di manutenzione, la cronologia
  e le regole. Vedi [Pagina Alerts](alerts.md).
- **Settings** — il tuo account, utenti e single sign-on, token agent e
  assistente, canali di notifica e report, pagine di stato pubbliche,
  aspetto, informazioni. Vedi [Settings](settings.md). Nulla
  sull'interfaccia trasmette lo stato solo tramite il colore: una
  targhetta o un LED è sempre accompagnato da una parola (Reporting,
  Unreachable, Advisory, Warning, Suppressed by parent, Building up).
