# Finestre di manutenzione {#maintenance-windows}

Una finestra di manutenzione (un *silence* nell'API) interrompe le
notifiche per un dispositivo, o per tutto, durante un periodo. La
valutazione continua sotto: quando la finestra termina, un avviso ancora
attivo viene notificato una volta, invece di apparire dal nulla dopo due
ore.

## Dove {#where}

- **Alerts → Scheduled maintenance**: l'elenco delle finestre, con *Active
  now* o *Scheduled*, e il modulo per aggiungerne una.
- **Pagina del dispositivo → Silence 1 h / Silence until…**: una finestra
  puntuale rapida per l'intero dispositivo.
- ++ctrl+k++ → *Schedule maintenance*.

**Snooze**, su un avviso nell'elenco *Needs you* o sulla pagina di un
dispositivo, è una versione più piccola dello stesso meccanismo: una
finestra puntuale limitata alle etichette esatte di quell'unico avviso
invece che all'intero dispositivo, così una regola gemella continua a
parlare. Appare anche qui, chiamata `Snooze · …`. Vedi
[Eliminare un avviso](../using/alerts.md#getting-rid-of-an-alert).

## Finestra puntuale {#one-off-window}

| Campo | Significato |
|---|---|
| Name | Mostrato nell'elenco e nella cronologia. |
| Device | Lascia vuoto per coprire ogni dispositivo. |
| Start, End | Orari assoluti. La finestra include l'inizio ed esclude la fine. |
| Comment | Opzionale. Perché la finestra esiste. |

Le finestre puntuali vengono eliminate una volta terminate.

## Finestra settimanale {#weekly-window}

| Campo | Significato |
|---|---|
| Days | Quali giorni della settimana (lunedì a domenica). |
| From, To | Ora locale nel fuso orario della finestra. Se *To* è pari o prima di *From*, la finestra sconfina oltre la mezzanotte ("23:00 → 01:00" appartiene al giorno in cui inizia). |
| Time zone | Un nome IANA come `Europe/Paris`. Precompilato con il fuso del tuo browser. |

## Finestra mensile {#monthly-window}

| Campo | Significato |
|---|---|
| On a weekday | *First*, *Second*, …, *Fifth* oppure *Last*, più il giorno — "la prima domenica", "l'ultimo venerdì". |
| On a date | Uno o più giorni del mese, da 1 a 31, separati da virgole ("1, 15"). |
| From | Ora locale di inizio, nel fuso orario della finestra. |
| For | Quanto dura la finestra, come tempo reale trascorso. Una finestra mensile può durare più di un giorno. |
| Time zone | Come sopra. |

Un mese che non contiene l'occorrenza viene saltato invece di essere
approssimato: non c'è un 31 in febbraio, e nessuna quinta domenica in aprile
2026, quindi la finestra semplicemente non si apre quel mese. L'elenco
mostra la prossima occorrenza calcolata dal server, così puoi verificarla a
colpo d'occhio.

DumbMonit non ha espressioni cron, di proposito: un'espressione cron indica
*istanti*, e una finestra di manutenzione ha bisogno di una *durata*. I
giorni del mese e "l'n-esimo giorno della settimana" coprono i calendari che
le persone tengono davvero, e si leggono in una riga.

## Fusi orari e ora legale {#time-zones-and-daylight-saving}

Una finestra ricorrente si apre al suo orario locale sull'orologio e poi
dura per la sua durata in tempo reale. Entrambe le metà contano:

- **L'ora locale non slitta.** "Ogni domenica alle 02:00 in `Europe/Paris`"
  è 01:00 UTC in inverno e 00:00 UTC in estate — la finestra segue il
  cambio dell'ora invece di spostarsi di un'ora due volte all'anno.
- **La durata non si allunga.** Nella notte in cui l'orologio torna
  indietro, da 02:00 a 04:00 sull'orologio durerebbe tre ore reali; la
  finestra dura comunque due. Nella notte in cui l'orologio va avanti e le
  02:00 non esistono, la finestra si apre appena l'orologio riprende.

Una finestra salvata prima che i fusi orari esistessero mantiene il suo
offset UTC fisso e si comporta esattamente come prima. Risalvala con un
fuso per farla seguire i cambi dell'orologio. Un nome di fuso che il server
non conosce viene rifiutato al salvataggio, con un esempio.

## Matcher {#matchers}

Una finestra può anche essere limitata da corrispondenze esatte di
etichette (`matchers` nell'API): tutte devono corrispondere alle etichette
dell'avviso, per esempio `tag_role = lab` e `__name__ = dumbmonit_up`. Una
finestra senza dispositivo e senza matcher silenzia l'intera istanza. Le
finestre sovrapposte vanno bene; la prima che copre il momento spiega il
silenzio.

## Com'è un avviso silenziato {#what-a-silenced-alert-looks-like}

L'avviso mantiene la sua fase e mostra `silenced: true`; la cronologia
registra la transizione con `notified: false` e il motivo. Disattivare una
finestra (`enabled: false`) la mantiene nell'elenco senza effetto.

## Su una pagina di stato pubblica {#on-a-public-status-page}

Un dispositivo coperto da una finestra di manutenzione viene mostrato come
**Maintenance** su qualsiasi [pagina di stato](../using/status-pages.md)
che lo elenca, invece di rosso. Se nulla altro sulla pagina è inattivo o
degradato, lo stato generale della pagina indica *Maintenance* anche lui.
Solo le finestre che nominano un dispositivo — o i cui matcher sono
etichette a livello di dispositivo — emergono in questo modo; una finestra
limitata a una singola metrica resta una questione di avvisi.
