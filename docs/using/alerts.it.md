# Pagina Alerts

![La pagina Alerts: cosa richiede attenzione, le regole, la manutenzione pianificata e la cronologia](../assets/screenshots/alerts-light.png){ loading=lazy }

Tutto ciò che riguarda gli avvisi su una pagina, dietro cinque tab. Il tab
è nell'URL (`/alerts#rules`), così una vista è collegabile tramite link.
**Schedule maintenance**, in alto a destra, apre il modulo per una
finestra da qualsiasi tab.

| Tab | Cosa contiene |
|---|---|
| **Now** | L'elenco live *Needs you*, raggruppato per dispositivo: targhetta di severità, motivo, da quando, e *Ack*, *Snooze*, *Ignore* e *Open device* su ciascuno. Gli avvisi confermati e sospesi stanno ciascuno nel proprio gruppo silenzioso in fondo. Il badge sul tab, e su Alerts nella barra superiore, è il conteggio di ciò che richiede ancora attenzione. |
| **Scheduled** | Finestre di manutenzione, *Active now* o *Scheduled*, puntuali, settimanali o mensili, ciascuna con la prossima occorrenza. Vedi [Finestre di manutenzione](../alerting/maintenance.md). |
| **Rules** | Ogni regola con la sua severità, un marchio *Built-in* e, se un dispositivo la ignora, *Ignored on N devices*; abilita, modifica inline, elimina le tue, crea una regola di soglia. Vedi [Regole](../alerting/rules.md). |
| **Notifications** | Dove ti raggiungono gli avvisi: i canali e la politica di notifica. Dettagli sotto. |
| **History** | Le ultime 200 transizioni, ciascuna che nomina il dispositivo, la regola e cosa è successo; quelle risolte possono essere pulite. |

Lo stesso modello di verità alimenta il bollettino di panoramica e questa
pagina, così i due sono sempre d'accordo su cosa richiede attenzione.

## Eliminare un avviso

Un avviso attivo non è binario — "risolverlo" o "convivere con il rumore
per sempre". Quattro azioni, per quattro situazioni diverse, aperte agli
amministratori e agli operatori (un visualizzatore vede lo stato ma non
può modificarlo):

- **Acknowledge** ("ci sto lavorando") è per un avviso su cui stai
  attivamente lavorando: "lo so, smetti di ricordarmelo per 4 h". Il menu
  offre 1 h, 4 h, 24 h oppure *until resolved*, più una nota opzionale per
  chiunque legga la scheda dopo di te. L'avviso continua ad attivarsi ed
  essere valutato; solo i suoi ricordi ed escalation si fermano —
  inclusa l'escalation a un secondo canale. Vieni comunque informato
  quando si risolve, e la conferma si azzera in quel momento — un avviso
  che ritorna più tardi notifica di nuovo. La scheda si sposta dall'elenco
  principale in un gruppo *Acknowledged* più silenzioso, legge *Acked by
  someone until a time*, e offre **Un-ack**.
- **Snooze** è per "non ora, controlla più tardi" senza affermare di
  starci lavorando: 1 h, 8 h, 1 giorno oppure *until resolved*. A
  differenza di Ack, funziona aprendo una finestra di manutenzione
  limitata alle etichette esatte di quell'unico avviso — una regola
  gemella sullo stesso dispositivo continua a parlare. La scheda si
  sposta in un gruppo silenzioso *Snoozed* che mostra il tempo rimanente,
  e offre **Unsnooze**. I ricordi si fermano nello stesso modo di una
  conferma.
- **Ignore** ("non avvisarmi più di questo") è per una regola che non
  avrà mai senso su questo dispositivo: disattiva quella regola solo per
  quel dispositivo (un override per dispositivo — vedi
  [Override per dispositivo](../alerting/rules.md#per-device-overrides))
  e l'avviso scompare per sempre, immediatamente, non solo zittito. È
  reversibile: **Stop ignoring** sulla pagina del dispositivo, oppure
  dalla regola stessa in Alerts → Rules, dove legge *Ignored on N
  devices*.
- **Clear** è per la cronologia, non per un avviso attivo: una volta che
  una transizione si è risolta, *Clear* (o *Clear all resolved*) la
  rimuove dalla vista History predefinita. La riga resta nel database —
  nulla viene eliminato — e *Show cleared* la riporta.

Le pagine di stato pubbliche ignorano le conferme e le sospensioni: un
avviso confermato o sospeso è comunque un avviso per il mondo esterno.
Una regola ignorata, invece, non ha mai prodotto un avviso in primo luogo.

## Notifications

I canali e la politica vivono qui — sotto Alerts, non Settings — perché
decidono chi sente un avviso, il che è una questione di avvisi. Due
pannelli:

**Channels.** L'elenco dei canali, ciascuno con il suo tipo, *Enabled* o
*Disabled*, quando ha inviato qualcosa l'ultima volta, e il suo ultimo
errore se presente. **Add channel** apre un modulo costruito dalla
descrizione propria del server del tipo scelto: impostazioni (visibili) e
segreti (solo scrittura). **Send test** invia un messaggio di test e
mostra il risultato inline. Le **Delivery options** di ogni canale
contengono la sua severità minima, se sente le risoluzioni, un intervallo
minimo per avviso, e le sue **ore silenziose** (settimanali, nel tuo fuso
orario: solo gli avvisi di livello Warning passano, il resto attende un
digest).

**Only some alerts.** Le stesse Delivery options contengono il **filtro
di instradamento** del canale: quali avvisi vuole, per tag del
dispositivo (`site=cellar`), per tipo di dispositivo (`proxmox`), oppure
per regola. Le condizioni sullo stesso campo si leggono come *or*, campi
diversi come *and*, e una riga *Except* vince sempre. Sotto l'editor, un
anteprima elenca quali dei tuoi dispositivi il filtro seleziona in questo
momento, e una frase te lo riassume — *questo canale riceve advisory e
superiori dai dispositivi taggati site=cellar, eccetto i dispositivi
taggati role=lab*. Un canale senza filtro continua a ricevere tutto, che è
ciò che fa ogni canale esistente.

Modificare un canale e lasciare vuoto il segreto mantiene quello
memorizzato. I tipi di canale e i loro campi sono documentati in
[Canali di notifica](../notifications.md).

Le regole di avviso integrate notificano ogni canale abilitato; una
regola può anche essere limitata a canali specifici nel suo editor.

**Notification policy.** Le manopole globali che mantengono poche le
notifiche: la finestra di batch, il limite per canale all'ora, il
rilevamento di flapping sotto *More options*, l'URL pubblico usato per i
link "Open in DumbMonit", e la riga di **escalation** — *se nessuno
conferma* entro un ritardo, *avvisa anche* un altro canale, una volta. Il
ritardo conta dal momento in cui il primo messaggio è effettivamente
uscito, e confermare l'avviso (o la sua risoluzione) ferma il salto. Vedi
[Politica di notifica](../alerting/notifications.md).

Link: `/alerts#notifications` apre il tab, `/alerts#notifications-policy`
scorre al pannello della politica. I vecchi link Settings
(`/settings#notifications`, `/settings#notifications-policy`) rimandano
qui.

## Operatori e visualizzatori

Un **operatore** gestisce gli avvisi senza toccare la configurazione:
conferma, sospende, ignora una regola per un dispositivo, pianifica o
elimina finestre di manutenzione e pulisce la cronologia risolta. Regole,
canali e la politica di notifica restano di sola lettura per loro,
contrassegnati *Operator — config is admin only*. Ignorare è il solo
override per dispositivo che un operatore può fare; le soglie restano una
decisione di un amministratore, e l'*Ignore* o lo *Stop ignoring* di un
operatore mantiene qualsiasi soglia che un amministratore ha impostato
per quel dispositivo.

Un visualizzatore vede ogni tab ma nessun controllo: la pagina mostra
*Viewer — read only* dove un amministratore troverebbe i pulsanti.
