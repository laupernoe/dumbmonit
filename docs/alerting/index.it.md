# Come funziona l'alerting {#how-alerting-works}

Regole utili sono attive subito dopo l'installazione; nulla ha bisogno di
essere messo a punto. Ciò che fa la differenza nell'uso quotidiano è
quello che mantiene basso il rumore: soppressione per dipendenza,
raggruppamento per host, finestre di manutenzione, e una baseline che
apprende prima di parlare.

![La pagina Alerts: cosa richiede attenzione, le regole, la manutenzione pianificata e la cronologia](../assets/screenshots/alerts-light.png){ loading=lazy }

## Severità {#severities}

L'interfaccia parla la scala del bollettino meteo. L'API e l'editor delle
regole memorizzano gli stessi tre livelli sotto i loro nomi del motore.

| Nell'interfaccia | Nell'API (`severity`) | Significato |
|---|---|---|
| Info | `info` | Da sapere, nessuna azione prevista. Usato dalla regola di baseline. |
| Advisory | `warning` | Richiede attenzione a breve: CPU alta, disco quasi pieno, servizio lento. |
| Warning | `critical` | Richiede attenzione ora: dispositivo irraggiungibile, servizio inattivo, verifica di backup fallita, certificato scaduto. |

L'escalation alza la severità di una regola di un gradino dopo
`escalate_after` se l'avviso continua ad attivarsi (Info → Advisory →
Warning; Warning resta).

## Fasi {#phases}

Ogni avviso è una macchina a stati, valutata ogni 30 secondi
(`DUMBMONIT_ALERT_INTERVAL_SECS`).

| Fase | Nell'interfaccia | Significato |
|---|---|---|
| `ok` | — | La condizione è falsa. |
| `pending` | Building up | La condizione è vera ma l'attesa della regola (`for`) non è ancora trascorsa. Nulla viene inviato. |
| `firing` | Firing | La condizione è durata abbastanza. La prima notifica esce, poi ricordi ogni `repeat_interval` (6 ore per impostazione predefinita per la maggior parte delle regole integrate), poi un'escalation se configurata. |
| `resolved` | Resolved | Tornato alla normalità. Una notifica di risoluzione viene inviata solo se l'attivazione era stata annunciata. |
| `suppressed` | Suppressed by parent | L'avviso è attivo, ma un dispositivo genitore è irraggiungibile: vedi sotto. |

`phase` è lo stato del motore; `effective_phase` è ciò che leggi. Un
avviso il cui `effective_phase` è `suppressed` mantiene `phase = firing`
sotto, così non viene notificato di nuovo quando il genitore si
riprende.

Un avviso può anche essere **confermato** (`acked`, con `acked_by`,
`acked_until` e `ack_note`): la fase non si muove, i ricordi e le
escalation si fermano finché `acked_until`, la risoluzione viene comunque
notificata e azzera la conferma. **Sospendere** un avviso funziona nello
stesso modo sotto il cofano ma tramite una finestra di manutenzione
limitata alle etichette esatte di quell'avviso (`silenced: true`), invece
che all'intero dispositivo: una regola gemella sullo stesso dispositivo
continua a parlare. Vedi
[Eliminare un avviso](../using/alerts.md#getting-rid-of-an-alert).

## Soppressione per dipendenza {#dependency-suppression}

Qualsiasi dispositivo può dichiarare un **dispositivo genitore**. Quando
un dispositivo è irraggiungibile (la regola "Device unreachable" si
attiva su di esso), ogni avviso sui suoi discendenti viene contrassegnato
*Suppressed by parent* invece di essere inviato: uno switch che si spegne
produce una notifica, non trenta. L'antenato irraggiungibile più vicino
viene nominato come causa (`suppressed_by`), e la catena viene seguita
fino a 64 livelli.

Imposta il genitore nel modulo del dispositivo. Nella pagina Devices, i
figli si impilano sotto il loro genitore e si attenuano quando è
irraggiungibile.

Un [agent relay](../install/remote-site.md) conta come genitore di ogni
dispositivo che sonda (*Reached through* nel modulo del dispositivo), in
aggiunta al genitore impostato esplicitamente: quando il relay è
irraggiungibile, i suoi dispositivi vengono soppressi con il relay
nominato come causa. A pari distanza, il genitore esplicito vince.

Un genitore conta anche come irraggiungibile non appena la sua propria
sonda lo dice (lo stato *Timed out* / *Device unreachable* sulla sua
pagina), senza attendere che il suo avviso "Device unreachable" si
attivi: un agent che smette di inviare porta con sé i suoi dispositivi un
minuto o due prima del suo proprio avviso, e altrimenti ognuno di essi
notificherebbe prima.

## Raggruppamento, deduplicazione, ricordi {#grouping-deduplication-reminders}

Senza raggruppamento, un NAS il cui RAID si degrada invierebbe un
messaggio per disco, per ciclo. DumbMonit invia al massimo un messaggio
per host e per ciclo, anche se contiene cinque righe. Ogni avviso viene
notificato una volta quando inizia ad attivarsi (`firing`), poi solo come
un ricordo (`reminder`, ogni `repeat_interval`), un'escalation
(`escalation`) o una risoluzione (`resolved`). La cronologia degli
avvisi registra ogni transizione con se è stata notificata e, se no,
perché (apprendimento, soppresso, silenziato, confermato).

## Finestre di manutenzione {#maintenance-windows}

Un silenzio interrompe le notifiche senza interrompere la valutazione:
alla fine di una finestra, un avviso ancora attivo viene notificato una
volta, invece di "nascere" dopo due ore di esistenza. Le finestre sono
puntuali o settimanali, per dispositivo o per l'intera istanza. Vedi
[Finestre di manutenzione](maintenance.md).

## Apprendimento della baseline {#baseline-learning}

La regola "Unusual CPU" non richiede alcuna soglia. Per ogni serie
mantiene 168 bucket, uno per (giorno della settimana, ora), e confronta
ogni punto con ciò che viene solitamente osservato *in quel momento*. Il
punteggio è robusto (mediana e MAD, un EWMA per bucket con `alpha =
0.05`), così un singolo incidente non riscrive la baseline, e le derive
stagionali vengono seguite.

La regola resta **silenziosa per i suoi primi quattordici giorni** di
apprendimento: ogni bucket deve essere stato visto almeno due volte per
distinguere "inusuale" da "mai osservato". Nel frattempo viene mostrata
con un flag *learning* e dice solo cosa si sarebbe attivato. Le baseline
di serie scomparse vengono scordate dopo 60 giorni.

## Previsioni {#forecasts}

Due tipi di avviso prevedono invece di segnalare, e la panoramica li
elenca sotto *Forecasts*: l'anomalia di baseline, e le regole `predict`
che estrapolano una serie con `predict_linear()` sul lato VictoriaMetrics
("Filesystem almost full" entro quattro giorni). La stima del datastore
PBS è una semplice soglia sulla previsione propria di PBS.

## Dove vanno le notifiche {#where-notifications-go}

Le regole integrate non hanno alcun canale collegato, il che significa
**ogni canale abilitato**. Una regola che modifichi può nominare canali
specifici. I canali vengono configurati in Alerts → Notifications; vedi
[Canali di notifica](../notifications.md).

## Notifiche intelligenti {#smart-notifications}

Tutto quanto sopra decide *se* un avviso può parlare. Una seconda fase, la
politica di notifica, decide *quando* e *su quale canale*, così che
nessuno venga sommerso di messaggi. Il percorso completo di un avviso, in
ordine:

1. **Valutare** — soglia, punteggio di anomalia o previsione, con
   l'attesa della regola (`for`) e, quando impostata, la sua **soglia di
   azzeramento** (isteresi: si attiva sopra il 90%, si azzera solo sotto
   l'85%) e qualsiasi **override per dispositivo**.
2. **Deduplicare** — un'impronta per (regola, serie), basata sull'id del
   dispositivo (`target`) invece che sul suo nome: rinominare un
   dispositivo o modificare i suoi tag non crea un secondo avviso, e una
   serie che si risolve nella stessa chiave due volte produce un unico
   avviso. Le serie di un dispositivo eliminato o messo in pausa vengono
   scartate: i loro avvisi si azzerano all'istante, senza alcuna
   notifica, e la cronologia registra `device removed or disabled`.
3. **Sopprimere per dipendenza** — i discendenti di un dispositivo
   irraggiungibile restano silenziosi.
4. **Silenziare** — le finestre di manutenzione zittiscono ciò che
   coprono.
5. **Raggruppare per dispositivo** — un messaggio per dispositivo per
   ciclo, ricordi ed escalation inclusi.
6. **Attesa di flapping** — un'impronta che si attiva e si azzera 4 volte
   in 30 minuti invia un unico avviso di "flapping", poi nulla per 30
   minuti.
7. **Filtri per canale** — severità minima, "dimmi quando si risolve"
   attivo/disattivo, e un intervallo minimo tra due messaggi sullo stesso
   avviso.
8. **Ore silenziose** (per canale) — solo gli avvisi di livello Warning
   passano; il resto attende un digest quando le ore silenziose
   terminano (un avviso che si azzera nel frattempo viene solo
   menzionato come "resolved during quiet hours").
9. **Batching** — gli avvisi entro la finestra di batch (60 s per
   impostazione predefinita) uscono come un messaggio per canale: "3
   alerts on 2 devices, 1 resolved".
10. **Limite orario** — oltre `max_per_hour` messaggi su un canale, gli
    avvisi attendono e arrivano come un digest ("…and 7 more alerts").
11. **Inviare** — con un link "Open in DumbMonit" per dispositivo quando
    un URL pubblico è conosciuto.

I passaggi 6 a 10 sono configurati in Alerts → Notifications →
Notification policy (globale) e su ogni canale (Delivery options). Vedi
[Politica di notifica](notifications.md).
