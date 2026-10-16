# Politica di notifica

La promessa di DumbMonit è che una notifica valga la pena di essere
letta. Il motore di alerting mantiene basso il rumore prima che un
avviso possa parlare (soppressione per dipendenza, raggruppamento,
finestre di manutenzione); la politica di notifica lo mantiene basso
dopo, per canale. Nulla ha bisogno di configurazione: i predefiniti
sotto si applicano subito.

## La pipeline, in un elenco

```
evaluate (hysteresis, per-device overrides)
  → deduplicate → suppress by dependency → silence → group by device
    → flap hold → channel severity filter → channel routing filter
      → resolved on/off → cooldown → quiet hours → batch window
        → hourly cap → send (+ deep link)
                                   ↘ nobody acknowledged → escalation channel
```

| Fase | Dove è impostata | Predefinito |
|---|---|---|
| Soglia di azzeramento (isteresi) | Editor regola → *Clear below / above* | nessuna (CPU: 85%, disco: 88% sulle regole fornite) |
| Override per monitor | Editor regola → *Per-device overrides* | nessuno |
| Rilevamento flapping | Settings → Notifications & reports → Notification policy → More options | 4 cambi in 30 min, attesa 30 min |
| Severità minima | Canale → Delivery options → *Send* | tutto |
| Dimmi quando si risolve | Canale → Delivery options | attivo |
| Intervallo minimo per avviso | Canale → Delivery options | nessuno |
| Ore silenziose | Canale → Delivery options | disattivate |
| Filtro di instradamento (tag, tipo, regola) | Canale → Delivery options → *Only some alerts* | disattivato (il canale sente tutto) |
| Escalation se nessuno conferma | Settings → Notifications & reports → Notification policy | disattivata |
| Finestra di batch | Settings → Notifications & reports → Notification policy | 60 s |
| Messaggi per canale all'ora | Settings → Notifications & reports → Notification policy | 20 |
| URL pubblico (deep link) | Settings → Notifications & reports → Notification policy, oppure `DUMBMONIT_PUBLIC_URL` | vuoto |

## Isteresi e override per monitor

Una regola che si attiva sopra il 90% e si azzera nel momento in cui il
valore scende sotto il 90% si attiverà e si azzererà per tutta la sera
su una CPU che oscilla tra l'89 e il 91%. Dalle una **soglia di
azzeramento**: una volta attivo, l'avviso si risolve solo quando il
valore va sotto quella soglia (per regole che "si attivano sopra") o
sopra di essa (per regole che "si attivano sotto"). Il campo API è
`clear_threshold` sulla regola; il server rifiuta una soglia di
azzeramento sul lato errato della soglia.

Un **override per monitor** cambia la soglia o la soglia di
azzeramento di un monitor, oppure disattiva la regola per quel
monitor, senza duplicare la regola:

```
PUT /api/alerts/rules/{id}/overrides/{target_id}
{ "threshold": 97, "clear_threshold": 95 }      # oppure { "enabled": false }
GET /api/alerts/rules/{id}/overrides
GET /api/alerts/overrides?target_id=7           # ogni override di un monitor
DELETE /api/alerts/rules/{id}/overrides/{target_id}
```

Un override con ogni campo vuoto viene rimosso. I messaggi citano la
soglia effettivamente applicata.

## Rilevamento di flapping

Indipendentemente dalla regola, un avviso la cui impronta si è attivata
o risolta 4 volte entro 30 minuti è *flapping*: un avviso esce
("flapping: 4 changes in 30 min, notifications held for 30 min") e nulla
altro su quell'avviso viene inviato finché l'attesa non termina. Una
risoluzione dopo l'attesa viene comunque annunciata. Regola o disattiva
in Settings → Notifications & reports → Notification policy → More options
(`flap_events`, `flap_window_secs`, `flap_hold_secs`; `flap_events: 0`
la disattiva).

## Opzioni di consegna per canale

Ogni canale decide cosa sente (`policy` nel payload del canale):

- `min_severity` — `info`, `warning` (Advisory e superiori) oppure
  `critical` (solo Warning). Un canale telefono tipicamente vuole
  `critical`, una stanza chat tutto.
- `notify_resolved` — `false` per sentire solo dei problemi.
- `min_interval_secs` — almeno questo tempo tra due messaggi sullo
  stesso avviso (ri-attivazioni e ricordi). Le risoluzioni non vengono
  mai ritardate da questo, e una risoluzione viene inviata solo a un
  canale che ha sentito l'attivazione.
- `quiet_hours` — una finestra settimanale nel tuo orario locale, stessa
  forma di una finestra di manutenzione settimanale. Durante le ore
  silenziose solo gli avvisi `critical` passano; il resto attende e
  arriva come un digest quando terminano, titolato "Quiet hours over —
  …". Un avviso che si attiva e si azzera durante le ore silenziose
  viene menzionato in quel digest solo come "resolved during quiet
  hours".
- `matcher` — quali avvisi il canale vuole, per tag, tipo di monitor
  o regola. Vuoto (il predefinito, e ciò che ogni canale esistente
  mantiene) significa tutto.

```
PUT /api/notify/channels/{id}
{ "name": "Phone", "kind": "ntfy", "settings": {…},
  "policy": { "min_severity": "critical", "min_interval_secs": 900,
              "quiet_hours": { "kind": "weekly", "days": [0,1,2,3,4],
                               "start_minute": 1320, "end_minute": 420,
                               "timezone": "Europe/Paris" } } }
```

Omettere `policy` in un aggiornamento mantiene quella memorizzata;
`"quiet_hours": null` le azzera, e `"matcher": null` azzera il filtro
di instradamento.

## Instradamento per tag

I monitor portano tag (`site=cellar`, `role=storage`) e un tipo
(`snmp`, `proxmox`, `synology`, `agent`…). Il **filtro di
instradamento** di un canale dice quali avvisi vuole:

```json
{ "policy": { "min_severity": "warning",
              "matcher": {
                "include": [ { "field": "tag", "key": "site", "value": "cellar" } ],
                "exclude": [ { "field": "tag", "key": "role", "value": "lab" } ] } } }
```

Tre campi, e solo uguaglianza esatta — nessuna espressione regolare,
nessuna parentesi:

| `field` | Corrisponde su | Esempio |
|---|---|---|
| `tag` | Un tag della pagina propria del monitor | `{"field":"tag","key":"site","value":"cellar"}` |
| `kind` | Il collector che sonda il monitor | `{"field":"kind","value":"proxmox"}` |
| `rule` | L'`uid` della regola | `{"field":"rule","value":"disk_full"}` |

Come si leggono gli elenchi:

- condizioni sullo **stesso** campo (la stessa chiave di tag, oppure
  `kind`, oppure `rule`) sono un **OR** — `site=cellar` più
  `site=attic` significa uno dei due;
- condizioni su **campi diversi** sono un **AND** — `site=cellar` più
  `role=storage` significa entrambi;
- **`exclude` vince sempre**, anche su un `include` che corrispondeva.

Quindi l'esempio sopra si legge, e l'interfaccia lo riassume con
esattamente queste parole: *questo canale riceve advisory e superiori
dai monitor taggati site=cellar, eccetto i monitor taggati
role=lab*.

Sotto l'editor, un'**anteprima** elenca quali dei tuoi monitor
attuali il filtro seleziona. Non è un'approssimazione: il server esegue
esattamente il filtro che il motore di alerting esegue, ridotto alla
parte che dipende solo dal monitor. Le condizioni di regola si
applicano agli avvisi invece che ai monitor, quindi l'anteprima le
nomina separatamente invece di fingere di contarle.

```
POST /api/notify/match-preview
{ "matcher": { "include": [ { "field": "tag", "key": "site", "value": "cellar" } ] } }

→ { "devices": [ { "id": 3, "name": "nas", "kind": "synology",
                   "tags": { "site": "cellar" }, "matched": true } ],
    "matched": 1, "total": 12, "rules": [], "excluded_rules": [] }
```

Un filtro è limitato a dodici condizioni. Oltre, un secondo canale è
più chiaro di una condizione in più.

## Escalation: se nessuno conferma

Un salto in più, e solo uno. Quando la politica imposta
`escalate_after_secs` e `escalate_channel`, un avviso che nessuno ha
[confermato](../using/alerts.md) entro quel ritardo viene anche
inviato — una volta — a quel canale:

```
PUT /api/notify/policy
{ "escalate_after_secs": 900, "escalate_channel": 4 }
```

- Il ritardo viene contato **dal momento in cui il primo messaggio è
  effettivamente uscito**, non dall'attivazione dell'avviso. Se le ore
  silenziose o una consegna fallita hanno inghiottito la prima
  notifica, non c'è ancora nulla da escalare e il salto attende.
- **Confermare l'avviso lo ferma**, esattamente come ferma i ricordi;
  lo stesso fa l'azzeramento dell'avviso.
- Il messaggio di escalation va al canale di escalation **soltanto** —
  i canali che hanno già sentito l'attivazione non vengono informati
  due volte.
- Attraversa il piano minimo di severità, il filtro di instradamento e
  le ore silenziose proprie di quel canale: hai nominato questo canale
  per essere svegliato, quindi non viene trattenuto nel momento esatto
  in cui conta.
- La riga lo dice: `🔴 Critical · Disk full — /data — 95 % … — still
  unacknowledged`.

`"escalate_channel": null` disattiva l'escalation; un ritardo di `0`
fa lo stesso. Entrambi sono necessari perché il salto esista, e il
server rifiuta un canale che non esiste invece di escalare nel vuoto.

DumbMonit si ferma qui deliberatamente: un salto, nessuna rotazione,
nessuna pianificazione di chi è di guardia questa settimana. Se ti
serve quello, punta il canale di escalation verso PagerDuty o Opsgenie,
che lo fanno correttamente.

## Batching e il limite orario

`GET/PUT /api/notify/policy` contiene le impostazioni globali:

```json
{ "batch_window_secs": 60, "max_per_hour": 20,
  "flap_events": 4, "flap_window_secs": 1800, "flap_hold_secs": 1800,
  "public_url": "https://monit.example.lan",
  "escalate_after_secs": 0, "escalate_channel": null }
```

- **Batch window** — gli avvisi che si attivano entro la finestra
  escono come *un* messaggio per canale, raggruppati per monitor:
  "3 alerts on 2 devices" con una sezione per monitor; le
  risoluzioni viaggiano insieme ("2 resolved"). `0` invia al ciclo
  successivo. Un digest elenca al massimo 12 righe, poi "…and N more
  alerts".
- **Hourly cap** — una volta che un canale ha ricevuto `max_per_hour`
  messaggi nell'ultima ora, gli avvisi successivi attendono e arrivano
  come un digest quando uno slot si libera. `0` rimuove il limite.
- I **canali on-call** (PagerDuty, Opsgenie) non vengono mai raggruppati
  tra monitor: apre e chiude un incidente per monitor, quindi
  ricevono un messaggio per monitor senza attendere.

I fallimenti di consegna mantengono le righe in coda; il ciclo
successivo ritenta.

## Cosa dice una riga

Ogni avviso è una riga, la stessa in mail, ntfy, chat e la variabile
`{{message}}` di un webhook personalizzato:

```
🔴 Critical · Disk full — /data — 95 % (threshold > 90 %), for 12 min
⚠️ Warning · VM or container stopped — win11-desktop (101), for 59 s
✅ Resolved · Service down — https://example.lan/
```

- Il glifo è seguito dalla parola di stato (Critical / Warning / Info,
  Resolved, Flapping), così la severità è leggibile senza colore o
  emoji.
- Una regola che osserva diverse serie nomina quella che si è attivata:
  la VM e il suo id, il punto di mount, l'interfaccia, il gruppo di
  backup, l'URL della sonda. Le regole a livello di monitor
  (irraggiungibile) non nominano nulla oltre al monitor.
- Il valore e la soglia vengono mostrati per le regole misurate; una
  regola tutto-o-nulla su una metrica 0/1 (a batteria, guest fermo) non
  ripete "1 (threshold > 0)".

## Deep link

Ogni messaggio termina con un link al monitor
(`{public_url}/targets/{id}`) quando un URL pubblico è conosciuto —
il `public_url` della politica, oppure `DUMBMONIT_PUBLIC_URL`. I
webhook personalizzati lo ottengono anche come variabile `{{link}}` a
meno che il loro proprio `base_url` non sia impostato.

Non c'è deliberatamente alcun link "silence for 1 h" nei messaggi: i
servizi di chat recuperano ogni link che mostrano per costruire
un'anteprima, il che attiverebbe il silenzio nel momento in cui il
messaggio viene pubblicato. Silenzia invece dalla pagina del
monitor o dalla pagina Alerts.

## Cosa è stato preso in prestito da Pulse, e cosa no

Il design segue [Pulse](https://github.com/rcourtman/Pulse) dove si
adatta a un monitor da homelab: soglie di attivazione/azzeramento, un
livello di override per risorsa, un cooldown di notifica, una finestra
di raggruppamento, un limite di tasso orario, ore silenziose,
rilevamento di flapping con un cooldown, "notify on resolve", e
strumenti on-call tenuti fuori dai digest. Non adottato: il periodo di
osservazione di 24 ore di Pulse prima di notificare (la regola di
baseline di DumbMonit apprende già silenziosamente; le regole di soglia
dovrebbero parlare dal primo giorno) e lo snooze/acknowledge dal
messaggio (vedi sopra). L'instradamento basato su tag per destinazione
*è* stato adottato, come il filtro di instradamento sopra — scegliere i
canali sulla regola si è rivelato essere il posto sbagliato per
"tutto nella cantina va in questa stanza".
