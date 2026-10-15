# Notifiche DumbMonit

Questa guida spiega come ricevere gli avvisi di DumbMonit sul servizio di tua scelta. Non richiede abilità di programmazione: ogni sezione dice dove trovare l'indirizzo o il token da copiare e cosa digitare nel modulo.

---

## Contenuti

- [Come funziona](#come-funziona)
- [Creazione e test di un canale](#creazione-e-test-di-un-canale)
- [Impostazioni o segreti: la differenza](#impostazioni-o-segreti-la-differenza)
- [Certificati autofirmati](#certificati-autofirmati)
- [Il canale personalizzato](#il-canale-personalizzato)
  - [Variabili disponibili](#variabili-disponibili)
  - [Esempio 1 — un'API JSON interna](#esempio-1--unapi-json-interna)
  - [Esempio 2 — un servizio che si aspetta un modulo](#esempio-2--un-servizio-che-si-aspetta-un-modulo)
  - [Esempio 3 — un SMS in testo semplice tramite un operatore](#esempio-3--un-sms-in-testo-semplice-tramite-un-operatore)
  - [Esempio 4 — una richiesta GET senza corpo](#esempio-4--una-richiesta-get-senza-corpo)
  - [Errori comuni nei modelli](#errori-comuni-nei-modelli)
- [Canali supportati](#canali-supportati)
- [Risoluzione dei problemi](#risoluzione-dei-problemi)

---

## Come funziona

Un **canale** è una destinazione: un canale Discord, un numero di telefono, una casella postale. Ne crei quanti ne vuoi, e le tue regole di avviso scelgono quali notificare.

Tre principi si applicano a ogni canale:

1. **Gli avvisi sono raggruppati.** Un messaggio per monitor e per ciclo, anche se cinque dischi si riempiono contemporaneamente. Senza questo, silenzerai le notifiche entro una settimana.
2. **Un canale rotto non blocca mai un altro.** Il fallimento viene registrato nella colonna "ultimo errore" del canale, e la consegna continua agli altri.
3. **Un servizio che non risponde viene abbandonato dopo 15 secondi.** Una notifica che arriva un minuto in ritardo non è più utile, e il ciclo di monitoraggio non deve attendere dietro un server irraggiungibile.

---

## Creazione e test di un canale

1. Apri **Impostazioni → Notifiche e report → Aggiungi canale** (`/settings/notifications`).
2. Dagli un nome significativo ("Discord — famiglia", "SMS — on-call"). Questo è il nome che sceglierai in seguito nelle tue regole di avviso.
3. Scegli il tipo di canale, quindi compila i campi descritti di seguito.
4. Salva, quindi fai clic su **Invia test**.

Il messaggio di test segue **esattamente lo stesso percorso** di un avviso reale: stessa richiesta, stesso token, stessa formattazione. Se arriva, il canale funziona. Se fallisce, il messaggio di errore nomina l'impostazione da correggere.

> **Un token non viene mai mostrato di nuovo.** Una volta salvato, viene crittografato nel database e l'interfaccia dice solo "configurato". Per modificare un canale senza digitare di nuovo il token, lascia semplicemente il campo vuoto.

---

## Impostazioni o segreti: la differenza

Ogni canale ha due gruppi di campi.

| | **Impostazioni** | **Segreti** |
|---|---|---|
| Contenuto | indirizzo del server, nome canale, argomento | token, password, URL webhook |
| Archiviazione | testo semplice | crittografato |
| Lettura da parte dell'interfaccia | sì | mai |

L'interfaccia **rifiuta** di salvare un campo sensibile tra le impostazioni, e te lo dice. Non è una formalità: le impostazioni vengono restituite così come sono dall'API, quindi un token archiviato lì verrebbe pubblicato ogni volta che la pagina viene aperta.

Le tabelle nella sezione [Canali supportati](#canali-supportati) dicono, per ogni canale, cosa va dove.

---

## Certificati autofirmati

Per impostazione predefinita DumbMonit rifiuta qualsiasi server il cui certificato non può verificare. Per un servizio self-hosted fornito con un certificato autofirmato o uno da un'autorità privata, i canali per Matrix, Mattermost, Rocket.Chat, Zulip, ntfy, Gotify, Bark, Apprise, Home Assistant, Signal e il webhook personalizzato hanno un'impostazione **Accetta un certificato non verificabile** (`insecure_tls`, un booleano, disattivato per impostazione predefinita).

Attivarlo mantiene la connessione crittografata ma smette di controllare l'identità del server, quindi usalo solo per un server che controlli, su una rete di cui ti fidi. Quando un certificato è il motivo di un fallimento, l'errore mostrato per il canale lo nomina (ad esempio "certificato peer non valido: UnknownIssuer").

---

## Il canale personalizzato

Il canale **`webhook`** è la valvola di sfogo di DumbMonit. Tu descrivi l'intera richiesta HTTP — metodo, indirizzo, intestazioni, tipo di contenuto e corpo —, il che ti consente di connetterti a quasi qualsiasi servizio, inclusa un'API interna che DumbMonit non conosce mai.

**Senza nessuna configurazione particolare**, invia già un payload JSON strutturato che uno script può usare:

```json
{
  "source": "dumbmonit",
  "target": "nas",
  "rule": "Disk full",
  "severity": "warning",
  "status": "firing",
  "title": "nas — Disk full",
  "text": "⚠️ Warning · Disk full — /data — 95 % (threshold > 90 %)",
  "value": 95.0,
  "threshold": 90.0,
  "unit": "%",
  "fingerprint": "nas/disk-full",
  "count": 1,
  "at": "2026-09-01T14:12:05+00:00",
  "link": ""
}
```

### Impostazioni canale personalizzato

| Impostazione | Richiesta | Predefinito | Ruolo |
|---|---|---|---|
| `url` | sì (qui o come segreto) | — | Indirizzo da chiamare. Può contenere variabili. |
| `method` | no | `POST` | `POST`, `PUT` o `GET`. |
| `content_type` | no | `json` | `json`, `form` o `text`. Determina anche come le variabili vengono sfuggite. |
| `body_template` | no | — | Modello del corpo. Quando assente, viene inviato il payload di cui sopra. |
| `base_url` | no | — | Indirizzo pubblico del tuo DumbMonit, in modo che `{{link}}` dia un collegamento cliccabile. |
| `username` | no | — | Nome utente per l'autenticazione HTTP "Basic". Richiede il segreto `password`. |

| Segreto | Ruolo |
|---|---|
| `url` | Usalo invece dell'impostazione `url` se l'indirizzo contiene già un token. |
| `token` | Inviato come intestazione `Authorization: Bearer …`, a meno che il tuo modello non usi `{{token}}`. |
| `password` | Password per l'autenticazione "Basic". |
| `headers` | Intestazioni extra, come oggetto JSON `{"X-Api-Key": "value"}`. Archiviato crittografato come il token e mai mostrato di nuovo, quindi le chiavi API e le intestazioni `Authorization` appartengono qui. |

`headers` è un segreto. I canali salvati da 0.1.0-alpha.6 o precedente, che le mantenevano nelle impostazioni, vengono spostate nei segreti crittografati al primo avvio, e l'API non le restituisce più. Come ogni segreto, vengono mantenuti quando modifichi il canale senza digitarli di nuovo; l'invio di `secrets` sostituisce l'intero set.

DumbMonit segue un reindirizzamento solo allo stesso host (o da `http` a `https` su quell'host): le intestazioni e il token di un canale non vanno mai da nessuna parte se non all'indirizzo che hai inserito.

### Variabili disponibili

Una variabile viene scritta tra **doppie graffe**: `{{title}}`. Gli spazi attorno al nome sono tollerati (`{{ title }}`). Le doppie graffe in un corpo JSON rimangono letterali, in modo da poter scrivere il tuo JSON normalmente.

| Variabile | Cosa contiene | Esempio |
|---|---|---|
| `{{title}}` | Titolo del messaggio completo | `nas — Disk full` |
| `{{message}}` | Corpo in testo semplice, una riga per avviso | `⚠️ Warning · Disk full — /data — 95 % (threshold > 90 %)` |
| `{{message_markdown}}` | Lo stesso corpo in Markdown | `**nas — Disk full**\n• ⚠️ …` |
| `{{rule}}` | Nome della regola di avviso | `Disk full` |
| `{{target}}` | Nome del monitor | `nas` |
| `{{target_id}}` | Identificatore numerico del monitor | `42` |
| `{{severity}}` | Gravità tecnica | `info`, `warning` o `critical` |
| `{{severity_fr}}` | Gravità come parola leggibile | `information`, `avertissement`, `critique` |
| `{{status}}` | Stato tecnico | `firing` o `resolved` |
| `{{status_fr}}` | Stato come parola leggibile | `en cours` o `résolu` |
| `{{value}}` | Valore misurato, unità inclusa | `95 %` |
| `{{value_raw}}` | Valore nudo, utilizzabile come **numero** JSON | `95` |
| `{{threshold}}` | Soglia attraversata, unità inclusa | `90 %` |
| `{{threshold_raw}}` | Soglia nuda, utilizzabile come **numero** JSON | `90` |
| `{{unit}}` | Unità del valore | `%` |
| `{{operator}}` | Confronto della soglia | `>` |
| `{{count}}` | Numero di avvisi raggruppati in questo messaggio | `3` |
| `{{fingerprint}}` | Identificatore stabile dell'avviso | `nas/disk-full` |
| `{{timestamp}}` | Timestamp ISO 8601 | `2026-09-01T14:12:05+00:00` |
| `{{timestamp_unix}}` | Timestamp in secondi dal 1970 | `1788358325` |
| `{{date}}` | Timestamp leggibile, in UTC | `2026-09-01 14:12 UTC` |
| `{{priority}}` | Priorità da 1 (bassa) a 5 (alta) | `4` |
| `{{color}}` | Colore di accento, senza hash | `D98A00` |
| `{{color_hex}}` | Colore di accento, con hash | `#D98A00` |
| `{{emoji}}` | Pittogramma di stato | `🔴`, `⚠️`, `ℹ️` o `✅` |
| `{{link}}` | Collegamento al monitor in DumbMonit | `https://dumbmonit.home/targets/42` |
| `{{source}}` | Sempre `dumbmonit` | `dumbmonit` |
| `{{token}}` | Il segreto `token` del canale | *(your token)* |

Due osservazioni utili:

- **Alcune variabili possono essere vuote.** Un avviso di disponibilità ("host non raggiungibile") non ha né valore né soglia: `{{value}}` e `{{value_raw}}` quindi eseguono il rendering di una stringa vuota, mai `null` o `NaN`. Se li posizioni come numero JSON, gestisci quel caso — o usa `"{{value}}"` tra virgolette.
- **`{{link}}` rimane vuoto** fino a quando non compili `base_url`. Un collegamento errato sprecare più tempo di un avviso senza uno.

**Non devi mai sfuggire a nulla da solo.** A seconda del `content_type` scelto, DumbMonit protegge automaticamente i valori sostituiti: virgolette e interruzioni di riga sfuggite in JSON, percent-encoding in un modulo, nessuna trasformazione in testo semplice. Una regola denominata `"System" disk full` non romperà il tuo modello JSON.

### Esempio 1 — un'API JSON interna

Un'applicazione homegrown espone `POST /incidents` e si aspetta un token in un'intestazione personalizzata.

**Impostazioni**

```json
{
  "url": "https://api.internal.example.org/incidents",
  "method": "POST",
  "content_type": "json",
  "base_url": "https://dumbmonit.home",
  "body_template": "{\"title\": \"{{title}}\", \"device\": \"{{target}}\", \"severity\": \"{{severity}}\", \"state\": \"{{status}}\", \"value\": {{value_raw}}, \"threshold\": {{threshold_raw}}, \"detail\": \"{{message}}\", \"link\": \"{{link}}\", \"seen_at\": \"{{timestamp}}\"}"
}
```

**Segreti**

```json
{
  "token": "your-application-token",
  "headers": "{\"X-Application\": \"dumbmonit\"}"
}
```

Il token viene inviato automaticamente come intestazione `Authorization: Bearer …`. Ecco cosa riceve l'applicazione:

```json
{
  "title": "nas — Disk full",
  "device": "nas",
  "severity": "warning",
  "state": "firing",
  "value": 95,
  "threshold": 90,
  "detail": "⚠️ Warning · Disk full — /data — 95 % (threshold > 90 %)",
  "link": "https://dumbmonit.home/targets/42",
  "seen_at": "2026-09-01T14:12:05+00:00"
}
```

### Esempio 2 — un servizio che si aspetta un modulo

Molti gateway SMS e API più vecchie si aspettano `application/x-www-form-urlencoded` piuttosto che JSON, con il token nel corpo.

**Impostazioni**

```json
{
  "url": "https://gateway.example.org/api/send",
  "method": "POST",
  "content_type": "form",
  "body_template": "key={{token}}&recipient=0612345678&subject={{title}}&text={{emoji}} {{message}}&priority={{priority}}"
}
```

**Segreti**

```json
{ "token": "gateway-key" }
```

Poiché il modello utilizza già `{{token}}`, DumbMonit **non aggiunge** un'intestazione `Authorization`: alcuni servizi rifiutano una richiesta che contiene due autenticazioni. I valori vengono codificati automaticamente, gli spazi diventano `+` e i caratteri accentati il loro equivalente percent-encoded.

### Esempio 3 — un SMS in testo semplice tramite un operatore

Alcuni servizi, come l'API SMS di un operatore o un'automazione homemade, vogliono una singola riga di testo.

**Impostazioni**

```json
{
  "url": "https://automation.home/alert",
  "method": "POST",
  "content_type": "text",
  "body_template": "{{emoji}} {{severity}} on {{target}}: {{rule}} ({{value}}, threshold {{operator}} {{threshold}}) — {{date}}"
}
```

Il messaggio inviato, senza alcuna fuga:

```
⚠️ warning on nas: Disk full (95 %, threshold > 90 %) — 01/09/2026 14:12 UTC
```

### Esempio 4 — una richiesta GET senza corpo

Quando il servizio accetta solo parametri URL, metti le variabili in `url` e scegli il metodo `GET`. Una richiesta `GET` non ha corpo: `body_template` viene quindi rifiutato, in modo che nessuno pensi che faccia qualcosa.

**Impostazioni**

```json
{
  "url": "https://sms.example.org/send?user=noe&text={{title}} — {{value}}",
  "method": "GET"
}
```

**Segreti**

```json
{ "token": "api-key" }
```

Solo i **valori sostituiti** vengono codificati; il resto dell'indirizzo viene copiato così com'è. La richiesta effettivamente inviata assomiglia a:

```
GET https://sms.example.org/send?user=noe&text=nas%20%E2%80%94%20Disk%20full%20%E2%80%94%2095%20%25
```

### Errori comuni nei modelli

Un modello difettoso viene **rifiutato al salvataggio**, mentre stai guardando il modulo, non la notte in cui si verifica l'incidente.

| Quello che scrivi | Cosa succede |
|---|---|
| `{{severite}}` | Rifiutato: variabile sconosciuta "severite", seguita dall'elenco dei nomi validi. Il nome corretto è `severity`. |
| `{"t": "{{title"}` | Rifiutato: "graffe non chiuse". Manca `}}`. |
| `{{}}` o `{{  }}` | Rifiutato: una variabile senza nome non significa nulla. |
| `{"a": {"b": 1}}` | **Accettato.** Le graffe singole sono letterali: scrivi il tuo JSON normalmente. |
| `{{token}}` senza un segreto `token` | Rifiutato: il modello chiede un token che il canale non ha. |
| `method: GET` con `body_template` | Rifiutato, con l'alternativa: metti le variabili in `url`. |
| `method: DELETE` | Rifiutato: sono accettati solo `POST`, `PUT` e `GET`. |

---

## Canali supportati

### Discord

In Discord: **Impostazioni canale → Integrazioni → Webhook → Nuovo webhook**, quindi **Copia URL webhook**. Questo URL contiene il token: va nei segreti.

- **Segreti** — `webhook_url`

### Slack

Su <https://api.slack.com/apps>, crea un'app, abilita **Webhook in arrivo**, quindi **Aggiungi nuovo webhook all'area di lavoro** e scegli il canale. Copia l'URL `https://hooks.slack.com/services/…`.

- **Segreti** — `webhook_url`

### Telegram

Hai bisogno di due cose: il **token del bot** (segreto) e l'**ID della chat** (`chat_id`) dove deve scrivere. Il primo richiede un minuto per ottenere; il secondo dipende da dove vuoi ricevere gli avvisi.

#### 1. Crea il bot e ottieni il suo token

1. In Telegram, cerca **@BotFather** e fai clic su *Start*.
2. Invia `/newbot`, quindi rispondi alle domande (un nome, quindi un nome utente che deve terminare con `bot`).
3. BotFather termina con un messaggio come questo:

   ```
   Done! Congratulations on your new bot. You will find it at t.me/my_homelab_bot.
   Use this token to access the HTTP API:
   63xxxxxx71:AAFoxxxxn0hwA-2TVSxxxNf4c
   ```

   La riga `63xxxxxx71:AAFoxxxxn0hwA-2TVSxxxNf4c` è il **token del bot**. Non condividerlo: dà il controllo completo del bot.

Negli esempi di seguito, sostituisci `<TOKEN>` con questo token. Nota il prefisso `bot` incollato davanti ad esso negli URL: `https://api.telegram.org/bot<TOKEN>/…`.

#### 2. Trova `chat_id`

**Chat privata** (il bot ti scrive direttamente)

1. Apri il tuo bot (`t.me/my_homelab_bot`) e fai clic su *Start*, o invigli un messaggio qualsiasi.
   Senza questo, non sarà mai in grado di scriverti.
2. Apri in un browser: `https://api.telegram.org/bot<TOKEN>/getUpdates`
3. Ottieni un documento JSON; cerca `"chat"`:

   ```json
   {"ok":true,"result":[{"update_id":83xxxxx35,
     "message":{"message_id":2643,"from":{…},
       "chat":{"id":21xxxxx38,"first_name":"…","type":"private"},
       "date":1703062972,"text":"/start"}}]}
   ```

   Il `chat_id` è il valore di `chat.id`: qui `21xxxxx38` (un numero positivo).

   Se `result` è vuoto (`"result":[]`), invia al bot un altro messaggio e ricarica la pagina.

**Canale**

1. Aggiungi il bot come amministratore del canale (Gestisci canale → Amministratori → Aggiungi).
2. Pubblica un messaggio nel canale.
3. Apri `https://api.telegram.org/bot<TOKEN>/getUpdates`: la risposta contiene un blocco `channel_post` con `"chat":{"id":-1001xxxxxx062,"type":"channel"}`.

   Il `chat_id` è quel numero negativo, `-100` incluso: `-1001xxxxxx062`.

**Gruppo**

Il modo più semplice è attraverso l'app desktop:

1. Aggiungi il bot al gruppo.
2. Invia un messaggio nel gruppo, fai clic destro → *Copia collegamento messaggio*. Ottieni `https://t.me/c/194xxxx987/13` (o `https://t.me/c/194xxxx987/11/13` se il gruppo ha argomenti).
3. Il numero subito dopo `/c/` è l'identificatore interno del gruppo: `194xxxx987`.
   Il `chat_id` da inserire è quel numero **con prefisso `-100`**: `-100194xxxx987`.

Senza l'app desktop, il metodo `getUpdates` sopra funziona ugualmente: `message.chat.id` è direttamente il numero giusto (già negativo). Se il bot non vede i messaggi del gruppo, chiedi a BotFather `/setprivacy` → *Disabilita*, quindi rimuovi il bot dal gruppo e aggiungilo di nuovo.

**Argomento di gruppo** (gruppi in modalità "Argomenti")

Il `chat_id` è quello del gruppo. Inoltre, compila **Argomento del gruppo** (`message_thread_id`) con il numero del mezzo del collegamento copiato: in `https://t.me/c/194xxxx987/11/13`, l'argomento è `11`. Senza questa impostazione, il messaggio arriva nel thread *Generale*.

#### 3. Controlla prima di salvare

Apri nel browser:

```
https://api.telegram.org/bot<TOKEN>/sendMessage?chat_id=<CHAT_ID>&text=test123
```

(aggiungi `&message_thread_id=11` per un argomento). Se `test123` arriva, il token e il `chat_id` sono corretti: inseriscili in DumbMonit e fai clic su *Test*.

- **Impostazioni** — `chat_id` (richiesta), `message_thread_id` (argomento, facoltativa), `api_base` (predefinita `https://api.telegram.org`, cambiarla solo per passare attraverso un relay)
- **Segreti** — `bot_token`

> **"Telegram non riesce a trovare questa chat"** — per una chat privata, il bot non ha mai ricevuto un messaggio da te: invigli `/start`. Per un gruppo o un canale, il prefisso `-100` probabilmente è mancante, o il bot non è stato aggiunto.
>
> **"thread non trovato"** — il `message_thread_id` non corrisponde a nessun argomento del gruppo, o il gruppo non è in modalità "Argomenti".

### Microsoft Teams

**Avviso: il metodo è cambiato.** I vecchi "connettori Office 365" sono stati ritirati a maggio 2026; un URL `outlook.office.com/webhook/…` non funziona più.

La procedura attuale:

1. Nel canale Teams, **⋯ → Flussi di lavoro**.
2. Scegli il modello **"Pubblica su un canale quando viene ricevuta una richiesta webhook"**.
3. Conferma i passaggi, quindi **copia l'URL POST HTTPS** mostrato alla fine.

- **Segreti** — `webhook_url`

DumbMonit invia una *scheda adattiva* avvolta, l'unico formato che questi flussi visualizzano correttamente. Se ricevi un errore che menziona Power Automate, l'URL probabilmente proviene da un connettore vecchio.

### Matrix

1. Crea un account dedicato a DumbMonit sul tuo homeserver e **invitalo nella stanza**. Un account che non è un membro non può scrivere.
2. Ottieni il suo token di accesso. In Element: **Impostazioni → Aiuto e informazioni → Avanzate → Token di accesso**.
3. Ottieni l'identificatore **interno** della stanza — non il suo nome visualizzato. In Element: **Impostazioni stanza → Avanzate → ID stanza interno**. Inizia con `!` (ad esempio `!aBcDeF:matrix.org`).

- **Impostazioni** — `server_url` (ad esempio `https://matrix.org`), `room_id`
- **Segreti** — `token`

### Mattermost

**Integrazioni → Webhook in arrivo → Aggiungi**. Scegli il canale predefinito, quindi copia l'URL.

- **Impostazioni** — `channel` (facoltativa, per scrivere da qualche parte diverso dal canale predefinito), `username` (facoltativa, predefinita `DumbMonit`)
- **Segreti** — `webhook_url`

### Rocket.Chat

**Amministrazione → Integrazioni → Nuova integrazione → Webhook in arrivo**. Scegli la stanza, abilita l'integrazione, quindi copia l'*URL webhook*.

- **Impostazioni** — `channel` (facoltativa), `alias` (facoltativa, predefinita `DumbMonit`)
- **Segreti** — `webhook_url`

### Google Chat

Nello spazio Google Chat: **nome dello spazio → App e integrazioni → Webhook → Aggiungi webhook**. Copia l'URL, che contiene già la chiave e il token.

- **Segreti** — `webhook_url`

### Zulip

1. Crea un **bot**: **Impostazioni personali → Bot → Aggiungi un nuovo bot**, digita "Generico".
   Annota il suo indirizzo email e la sua chiave API.
2. Sottoscrivi il bot al flusso di destinazione, altrimenti l'invio verrà rifiutato.

- **Impostazioni** — `server_url` (ad esempio `https://your-org.zulipchat.com`), `email` (indirizzo del bot), `stream` (nome del flusso), `topic` (facoltativa, predefinita `DumbMonit`)
- **Segreti** — `api_key`

### ntfy

Il canale più semplice di tutti: scegli un nome di argomento difficile da indovinare, quindi sottoscrivi da l'app ntfy.

- **Impostazioni** — `topic`, `server_url` (facoltativa, predefinita `https://ntfy.sh`)
- **Segreti** — `token` (solo se il tuo server richiede l'autenticazione)

> Sull'istanza pubblica, **chiunque conosca il nome di un argomento può leggerlo**. Scegli un nome lungo e casuale, oppure ospita il tuo server.

### Gotify

Nell'interfaccia Gotify: **App → Crea applicazione**. Copia il token mostrato.

- **Impostazioni** — `server_url` (ad esempio `https://gotify.home`)
- **Segreti** — `token`

### Pushover

1. Su <https://pushover.net>, la tua **chiave utente** è mostrata proprio nella home page.
2. Quindi crea un'applicazione: **Crea un'applicazione/token API**. Ottieni un **token dell'applicazione**.

- **Impostazioni** — `priority` (facoltativa, da -2 a 2), `sound` (facoltativa), `retry` e `expire` (solo per `priority: 2`)
- **Segreti** — `token` (token dell'applicazione), `user_key` (chiave utente)

Senza un'impostazione `priority`, DumbMonit sceglie in base alla gravità: `-1` per un avviso o una risoluzione, `0` per un avviso, `1` per un avviso critico. Un avviso quindi non fa squillare il telefono.

> La priorità `2` ("emergenza") ripete la notifica finché non viene riconosciuta.
> DumbMonit aggiunge quindi `retry` e `expire` automaticamente, senza i quali Pushover rifiuterebbe il messaggio.

### Pushbullet

Su <https://www.pushbullet.com/#settings/account>, fai clic su **Crea token di accesso**.

- **Impostazioni** — `device_iden` (facoltativa; senza di essa, tutti i tuoi dispositivi vengono notificati)
- **Segreti** — `token`

### Bark (iOS)

Apri l'app Bark sul tuo iPhone: mostra un URL contenente la tua **chiave del dispositivo**, la lunga stringa di caratteri dopo il nome di dominio.

- **Impostazioni** — `server_url` (facoltativa, predefinita `https://api.day.app`), `group` (facoltativa), `level` (facoltativa), `sound` (facoltativa)
- **Segreti** — `token` (la chiave del dispositivo)

Senza un'impostazione `level`, DumbMonit sceglie: `passive` per un avviso, `active` per un avviso, `timeSensitive` per un avviso critico. Il livello `critical`, che infrange "Non disturbare", non viene mai scelto automaticamente — impostalo esplicitamente se lo desideri.

### Web Push

Notifiche native sui tuoi telefoni e computer, direttamente dal tuo DumbMonit, senza app da installare e nessun account di terze parti: il browser (installato come app o meno) riceve gli avvisi anche con ogni scheda chiusa.

1. Su ogni telefono o computer, apri DumbMonit e vai a **Impostazioni → Notifiche push → Abilita su questo dispositivo**, quindi **Invia un test**. Su iPhone e iPad, aggiungi prima DumbMonit alla schermata Home. La guida passo dopo passo è in [Notifiche push (PWA)](using/push-notifications.md).
2. Crea un canale **Web Push** qui. Non ha segreti: i suoi destinatari sono i dispositivi iscritti nel passaggio 1.

- **Impostazioni** — `users` (facoltativa; un account per riga, vuota: ogni dispositivo iscritto di ogni account), `urgency` (`high` per impostazione predefinita, `normal` o `low`), `ttl` (secondi che il servizio push mantiene il messaggio per un dispositivo spento, predefinito `86400`), `contact` (indirizzo facoltativo `mailto:` o `https:` dato ai servizi push; vuoto: `DUMBMONIT_PUBLIC_URL` quando è HTTPS, altrimenti la pagina del progetto)
- **Segreti** — nessuno. La coppia di chiavi VAPID del server viene generata al primo avvio e archiviata crittografata; la chiave privata non lascia mai il server.

Il titolo della notifica è la gravità e il monitor (`Critical · nas01`), il corpo un breve riassunto; un tocco apre la pagina **Avvisi**. DumbMonit ha bisogno di HTTPS in uscita verso i servizi push (`fcm.googleapis.com`, `*.notify.windows.com`, `updates.push.services.mozilla.com`, `*.push.apple.com`); i messaggi sono crittografati end-to-end, questi servizi non possono leggerli. Un'iscrizione che il servizio push segnala come scomparsa (HTTP 404 o 410, ad esempio dopo la rimozione dell'app) viene eliminata automaticamente.

### Apprise

Apprise è un **gateway**: con una singola configurazione dà accesso a dozzine di servizi che DumbMonit non implementa da solo. Distribuisci il contenitore `caronc/apprise`, quindi scegli una delle due modalità.

**Modalità consigliata — configurazione archiviata in Apprise.** Crea una configurazione nell'interfaccia Apprise, annota la sua chiave e non mettere nient'altro qui.

- **Impostazioni** — `server_url`, `config_key`

**Modalità diretta — DumbMonit fornisce le destinazioni.** Gli URL di Apprise spesso contengono una password (`mailto://user:password@…`): vanno quindi nei segreti.

- **Impostazioni** — `server_url`
- **Segreti** — `urls` (uno o più URL di Apprise, separati da virgole)

Impostazioni comuni facoltative: `tag` (per indirizzare solo parte delle destinazioni di una configurazione) e `format` (`text` per impostazione predefinita, o `markdown`).

> Se il test risponde "Apprise non ha configurazione sotto questo config_key", la chiave non corrisponde a nulla. DumbMonit lo segnala invece di fingere che il messaggio sia stato inviato.

### Home Assistant

1. In Home Assistant: **tuo profilo → Sicurezza → Token di accesso a lungo termine → Crea token**. Copialo, non verrà più mostrato.
2. Scegli il servizio da chiamare. Per impostazione predefinita, DumbMonit crea una notifica persistente sul dashboard.

- **Impostazioni** — `server_url` (ad esempio `http://homeassistant.local:8123`), `service` (facoltativa, predefinita `persistent_notification.create`), `target` (facoltativa), `data` (facoltativa, oggetto unito nella chiamata)
- **Segreti** — `token`

Alcuni servizi utili:

| `service` | Effetto |
|---|---|
| `persistent_notification.create` | Banner nell'interfaccia Home Assistant |
| `notify.notify` | Tutti i destinatari di notifica configurati |
| `notify.mobile_app_<device>` | L'app mobile di un dispositivo specifico |
| `light.turn_on` | Accende una luce — usa `data` per l'entità e il colore |

L'impostazione `data` viene unita **per ultimo**: può quindi sostituire il titolo o il messaggio che DumbMonit ha preparato.

### Email (SMTP)

L'unico canale che non dipende da nessun servizio di terze parti.

- **Impostazioni** — `host`, `from`, `to` (elenco di indirizzi), `security` (`starttls` per impostazione predefinita, o `tls`, o `none`), `port` (derivato da `security`: 587, 465 o 25), `username` (facoltativa)
- **Segreti** — `password`

> Con Gmail, la password di un account non funziona: crea una **password dell'app** nelle impostazioni di sicurezza del tuo account Google.

### Signal

Passa attraverso un'istanza [`signal-cli-rest-api`](https://github.com/bbernhard/signal-cli-rest-api) che ospiti e dove registri prima il tuo numero.

- **Impostazioni** — `server_url` (ad esempio `http://signal.home:8080`), `number` (il numero mittente registrato), `recipients` (elenco di numeri o identificatori di gruppo), `username` (facoltativa)
- **Segreti** — `password` (facoltativa, se la tua istanza è protetta)

Tutti i numeri sono scritti in **formato internazionale**, `+33612345678`.

### SMS tramite Twilio

Sulla console di Twilio, la home page mostra il tuo **SID account** e il tuo **token di autenticazione**. Quindi acquista un numero che può inviare SMS.

- **Impostazioni** — `account_sid`, `from` (il tuo numero Twilio) **o** `messaging_service_sid`, `to` (elenco di destinatari)
- **Segreti** — `token` (il *token di autenticazione*)

Il corpo del SMS viene troncato a 300 caratteri: oltre quello, Twilio lo divide in segmenti fatturati separatamente.

> Su un account di prova, Twilio invia solo ai numeri che hai verificato.

### PagerDuty

In PagerDuty: **Servizi → il tuo servizio → Integrazioni → Aggiungi integrazione**, e scegli **Events API v2**. Copia la *chiave di integrazione*.

- **Impostazioni** — `region` (`eu` se il tuo account è ospitato in Europa), `source` (facoltativa, predefinita `dumbmonit`)
- **Segreti** — `token` (la *chiave di integrazione*)

Un avviso apre un incidente; la sua risoluzione **lo chiude automaticamente**. L'abbinamento viene eseguito sull'impronta digitale dell'avviso, con prefisso `dumbmonit-`, quindi un promemoria non apre mai un secondo incidente.

> Una chiave di un altro tipo ("Events API v1", "REST API") riceve un rifiuto conciso. Controlla che l'integrazione sia davvero di tipo *Events API v2*.

### Opsgenie

**Squadre → la tua squadra → Integrazioni → Aggiungi integrazione → API**. Copia la chiave API.

- **Impostazioni** — `region` (`eu` per un account europeo), `responders` (elenco di nomi di squadra, facoltativa), `tags` (elenco, facoltativa)
- **Segreti** — `api_key`

Come con PagerDuty, una risoluzione chiude l'avviso, identificato dal suo alias.

> Atlassian ha impostato la **fine del servizio per Opsgenie il 5 aprile 2027**. Questo canale rimane per le installazioni esistenti; per uno nuovo, preferisci PagerDuty o un canale personalizzato che punta a Jira Service Management.

### Webhook personalizzato

Vedi [Il canale personalizzato](#il-canale-personalizzato) sopra.

---

## Risoluzione dei problemi

**Il test fallisce immediatamente, con "impostazione mancante ...".**
Un campo obbligatorio è vuoto. Il messaggio nomina l'impostazione esatta.

**Il test fallisce con "nessuna risposta entro 15 s".**
Il server di destinazione è irraggiungibile da DumbMonit. Se è un servizio sulla tua rete locale, verifica che il contenitore DumbMonit possa raggiungerlo — questo è frequentemente un problema di rete Docker, non un problema di configurazione del canale.

**Il test fallisce con un errore di certificato.**
Il tuo server utilizza un certificato autofirmato o di CA privata. Vedi [Certificati autofirmati](#certificati-autofirmati).

**Il test ha esito positivo, ma non ricevo nulla.**
Il servizio ha accettato il messaggio e lo ha inviato altrove: canale sbagliato, argomento ntfy sbagliato, dispositivo Pushbullet sbagliato. Controlla l'impostazione di destinazione.

**Il messaggio di errore contiene `***`.**
È intenzionale: il tuo token è stato rimosso dal messaggio prima di essere registrato.
Nessun segreto appare mai nei log, nella colonna "ultimo errore" o in una risposta API.

**Dove posso vedere l'ultimo errore di un canale?**
Nell'elenco dei canali: ogni riga mostra la data dell'ultima consegna riuscita e, se presente, l'errore dell'ultimo fallimento.
