# Notifiche DumbMonit

Questa guida spiega come ricevere gli avvisi di DumbMonit sul servizio
di tua scelta. Non richiede alcuna competenza di programmazione: ogni
sezione dice dove trovare l'indirizzo o il token da copiare, e cosa
digitare nel modulo.

---

## Indice

- [Come funziona](#how-it-works)
- [Creare e testare un canale](#creating-and-testing-a-channel)
- [Impostazioni o segreti: la differenza](#settings-or-secrets-the-difference)
- [Il canale personalizzato](#the-custom-channel)
  - [Variabili disponibili](#available-variables)
  - [Esempio 1 — un'API JSON interna](#example-1--an-internal-json-api)
  - [Esempio 2 — un servizio che si aspetta un form](#example-2--a-service-that-expects-a-form)
  - [Esempio 3 — un SMS in testo semplice tramite un operatore](#example-3--a-plain-text-sms-through-a-carrier)
  - [Esempio 4 — una richiesta GET senza corpo](#example-4--a-get-request-without-a-body)
  - [Errori comuni di template](#common-template-mistakes)
- [Canali supportati](#supported-channels)
- [Risoluzione dei problemi](#troubleshooting)

---

## Come funziona

Un **canale** è una destinazione: un canale Discord, un numero di
telefono, una casella di posta. Ne crei quanti vuoi, e le tue regole
di avviso scelgono quali notificare.

Tre principi si applicano a ogni canale:

1. **Gli avvisi vengono raggruppati.** Un messaggio per dispositivo e
   per ciclo, anche se cinque dischi si riempiono contemporaneamente.
   Senza questo, disattiveresti le notifiche entro una settimana.
2. **Un canale rotto non blocca mai un altro.** Il fallimento viene
   registrato nella colonna "last error" del canale, e la consegna
   continua verso gli altri.
3. **Un servizio che non risponde viene abbandonato dopo 15 secondi.**
   Una notifica che arriva un minuto in ritardo non è più utile, e il
   loop di monitoraggio non deve attendere dietro un server
   irraggiungibile.

---

## Creare e testare un canale

1. Apri **Alerts → Notifications → Add channel** (`/alerts#notifications`).
2. Dagli un nome significativo ("Discord — family", "SMS — on-call").
   È il nome che scegli dopo nelle tue regole di avviso.
3. Scegli il tipo di canale, poi compila i campi descritti sotto.
4. Salva, poi clicca **Send test**.

Il messaggio di test segue **esattamente lo stesso percorso** di un
avviso reale: stessa richiesta, stesso token, stessa formattazione. Se
arriva, il canale funziona. Se fallisce, il messaggio di errore nomina
l'impostazione da correggere.

> **Un token non viene mai mostrato di nuovo.** Una volta salvato,
> viene cifrato nel database e l'interfaccia dice solo "configured".
> Per modificare un canale senza ridigitare il suo token, lascia
> semplicemente vuoto il campo.

---

## Impostazioni o segreti: la differenza

Ogni canale ha due gruppi di campi.

| | **Settings** | **Secrets** |
|---|---|---|
| Contenuto | indirizzo del server, nome del canale, topic | token, password, URL webhook |
| Memorizzazione | testo semplice | cifrato |
| Riletto dall'interfaccia | sì | mai |

L'interfaccia **rifiuta** di salvare un campo sensibile tra le
impostazioni, e te lo dice. Non è una formalità: le impostazioni vengono
restituite così come sono dall'API, quindi un token memorizzato lì
verrebbe pubblicato ogni volta che la pagina viene aperta.

Le tabelle nella sezione [Canali supportati](#supported-channels)
dicono, per ogni canale, cosa va dove.

---

## Il canale personalizzato

Il canale **`webhook`** è la via di fuga di DumbMonit. Descrivi
l'intera richiesta HTTP — metodo, indirizzo, header, tipo di contenuto
e corpo —, il che ti permette di collegarti a quasi qualsiasi servizio,
incluso un'API interna che DumbMonit non conoscerà mai.

**Senza alcuna configurazione particolare**, invia già un payload JSON
strutturato che uno script può usare:

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

### Impostazioni del canale personalizzato

| Impostazione | Richiesta | Predefinito | Ruolo |
|---|---|---|---|
| `url` | sì (qui o come segreto) | — | Indirizzo da chiamare. Può contenere variabili. |
| `method` | no | `POST` | `POST`, `PUT` o `GET`. |
| `content_type` | no | `json` | `json`, `form` o `text`. Determina anche come le variabili vengono escaped. |
| `body_template` | no | — | Template del corpo. Quando assente, viene inviato il payload sopra. |
| `headers` | no | — | Header extra, come un oggetto `{"X-Key": "value"}`. |
| `base_url` | no | — | Indirizzo pubblico del tuo DumbMonit, così che `{{link}}` dia un link cliccabile. |
| `username` | no | — | Nome utente per l'autenticazione HTTP "Basic". Richiede il segreto `password`. |

| Segreto | Ruolo |
|---|---|
| `url` | Usalo invece dell'impostazione `url` se l'indirizzo contiene già un token. |
| `token` | Inviato come header `Authorization: Bearer …`, a meno che il tuo template non usi `{{token}}`. |
| `password` | Password per l'autenticazione "Basic". |

### Variabili disponibili

Una variabile viene scritta tra **doppie parentesi graffe**:
`{{title}}`. Gli spazi attorno al nome sono tollerati (`{{ title }}`).
Le parentesi singole in un corpo JSON restano letterali, così puoi
scrivere il tuo JSON normalmente.

| Variabile | Cosa contiene | Esempio |
|---|---|---|
| `{{title}}` | Titolo completo del messaggio | `nas — Disk full` |
| `{{message}}` | Corpo in testo semplice, una riga per avviso | `⚠️ Warning · Disk full — /data — 95 % (threshold > 90 %)` |
| `{{message_markdown}}` | Lo stesso corpo in Markdown | `**nas — Disk full**\n• ⚠️ …` |
| `{{rule}}` | Nome della regola di avviso | `Disk full` |
| `{{target}}` | Nome del dispositivo monitorato | `nas` |
| `{{target_id}}` | Identificatore numerico del dispositivo | `42` |
| `{{severity}}` | Severità tecnica | `info`, `warning` o `critical` |
| `{{severity_fr}}` | Severità come parola leggibile | `information`, `avertissement`, `critique` |
| `{{status}}` | Stato tecnico | `firing` o `resolved` |
| `{{status_fr}}` | Stato come parola leggibile | `en cours` o `résolu` |
| `{{value}}` | Valore misurato, unità inclusa | `95 %` |
| `{{value_raw}}` | Valore nudo, usabile come **numero** JSON | `95` |
| `{{threshold}}` | Soglia superata, unità inclusa | `90 %` |
| `{{threshold_raw}}` | Soglia nuda, usabile come **numero** JSON | `90` |
| `{{unit}}` | Unità del valore | `%` |
| `{{operator}}` | Confronto della soglia | `>` |
| `{{count}}` | Numero di avvisi raggruppati in questo messaggio | `3` |
| `{{fingerprint}}` | Identificatore stabile dell'avviso | `nas/disk-full` |
| `{{timestamp}}` | Timestamp ISO 8601 | `2026-09-01T14:12:05+00:00` |
| `{{timestamp_unix}}` | Timestamp in secondi dal 1970 | `1788358325` |
| `{{date}}` | Timestamp leggibile, in UTC | `2026-09-01 14:12 UTC` |
| `{{priority}}` | Priorità da 1 (bassa) a 5 (alta) | `4` |
| `{{color}}` | Colore accento, senza cancelletto | `D98A00` |
| `{{color_hex}}` | Colore accento, con cancelletto | `#D98A00` |
| `{{emoji}}` | Pittogramma di stato | `🔴`, `⚠️`, `ℹ️` o `✅` |
| `{{link}}` | Link al dispositivo in DumbMonit | `https://dumbmonit.home/targets/42` |
| `{{source}}` | Sempre `dumbmonit` | `dumbmonit` |
| `{{token}}` | Il segreto `token` del canale | *(il tuo token)* |

Due osservazioni utili:

- **Alcune variabili possono essere vuote.** Un avviso di
  disponibilità ("host irraggiungibile") non ha né un valore né una
  soglia: `{{value}}` e `{{value_raw}}` rendono allora una stringa
  vuota, mai `null` o `NaN`. Se li metti come un numero JSON, gestisci
  quel caso — oppure usa `"{{value}}"` tra virgolette.
- **`{{link}}` resta vuoto** finché non compili `base_url`. Un link
  errato sprecherebbe più tempo di un avviso senza uno.

**Non hai mai bisogno di fare l'escape di nulla tu stesso.** A seconda
del `content_type` scelto, DumbMonit protegge automaticamente i valori
sostituiti: virgolette e interruzioni di riga escaped in JSON,
percent-encoding in un form, nessuna trasformazione in testo semplice.
Una regola chiamata `"System" disk full` non romperà il tuo template
JSON.

### Esempio 1 — un'API JSON interna

Un'applicazione fatta in casa espone `POST /incidents` e si aspetta un
token in un header personalizzato.

**Settings**

```json
{
  "url": "https://api.internal.example.org/incidents",
  "method": "POST",
  "content_type": "json",
  "base_url": "https://dumbmonit.home",
  "headers": {
    "X-Application": "dumbmonit"
  },
  "body_template": "{\"title\": \"{{title}}\", \"device\": \"{{target}}\", \"severity\": \"{{severity}}\", \"state\": \"{{status}}\", \"value\": {{value_raw}}, \"threshold\": {{threshold_raw}}, \"detail\": \"{{message}}\", \"link\": \"{{link}}\", \"seen_at\": \"{{timestamp}}\"}"
}
```

**Secrets**

```json
{ "token": "your-application-token" }
```

Il token viene inviato automaticamente come header
`Authorization: Bearer …`. Ecco cosa riceve l'applicazione:

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

### Esempio 2 — un servizio che si aspetta un form

Molti gateway SMS e API più vecchie si aspettano
`application/x-www-form-urlencoded` invece di JSON, con il token nel
corpo.

**Settings**

```json
{
  "url": "https://gateway.example.org/api/send",
  "method": "POST",
  "content_type": "form",
  "body_template": "key={{token}}&recipient=0612345678&subject={{title}}&text={{emoji}} {{message}}&priority={{priority}}"
}
```

**Secrets**

```json
{ "token": "gateway-key" }
```

Poiché il template usa già `{{token}}`, DumbMonit **non aggiunge** un
header `Authorization`: alcuni servizi rifiutano una richiesta che
porta due autenticazioni. I valori vengono codificati automaticamente,
gli spazi diventano `+` e i caratteri accentati il loro equivalente
percent-encoded.

### Esempio 3 — un SMS in testo semplice tramite un operatore

Alcuni servizi, come l'API SMS di un operatore o un'automazione fatta
in casa, vogliono una singola riga di testo.

**Settings**

```json
{
  "url": "https://automation.home/alert",
  "method": "POST",
  "content_type": "text",
  "body_template": "{{emoji}} {{severity}} on {{target}}: {{rule}} ({{value}}, threshold {{operator}} {{threshold}}) — {{date}}"
}
```

Il messaggio inviato, senza alcun escaping:

```
⚠️ warning on nas: Disk full (95 %, threshold > 90 %) — 01/09/2026 14:12 UTC
```

### Esempio 4 — una richiesta GET senza corpo

Quando il servizio accetta solo parametri URL, metti le variabili in
`url` e scegli il metodo `GET`. Una richiesta `GET` non ha corpo:
`body_template` viene allora rifiutato, così che nessuno creda che
faccia qualcosa.

**Settings**

```json
{
  "url": "https://sms.example.org/send?user=noe&text={{title}} — {{value}}",
  "method": "GET"
}
```

**Secrets**

```json
{ "token": "api-key" }
```

Solo i **valori sostituiti** vengono codificati; il resto
dell'indirizzo viene copiato così com'è. La richiesta effettivamente
inviata appare come:

```
GET https://sms.example.org/send?user=noe&text=nas%20%E2%80%94%20Disk%20full%20%E2%80%94%2095%20%25
```

### Errori comuni di template

Un template errato viene **rifiutato al salvataggio**, mentre stai
guardando il modulo, non la notte in cui accade l'incidente.

| Cosa scrivi | Cosa succede |
|---|---|
| `{{severite}}` | Rifiutato: variabile sconosciuta "severite", seguita dall'elenco dei nomi validi. Il nome corretto è `severity`. |
| `{"t": "{{title"}` | Rifiutato: "unclosed braces". Manca il `}}`. |
| `{{}}` o `{{  }}` | Rifiutato: una variabile senza un nome non significa nulla. |
| `{"a": {"b": 1}}` | **Accettato.** Le parentesi singole sono letterali: scrivi il tuo JSON normalmente. |
| `{{token}}` senza un segreto `token` | Rifiutato: il template chiede un token che il canale non ha. |
| `method: GET` con `body_template` | Rifiutato, con l'alternativa: metti le variabili in `url`. |
| `method: DELETE` | Rifiutato: solo `POST`, `PUT` e `GET` sono accettati. |

---

## Canali supportati

### Discord

In Discord: **Channel settings → Integrations → Webhooks → New
webhook**, poi **Copy webhook URL**. Questo URL contiene il token: va
nei segreti.

- **Secrets** — `webhook_url`

### Slack

Su <https://api.slack.com/apps>, crea un'app, abilita **Incoming
Webhooks**, poi **Add New Webhook to Workspace** e scegli il canale.
Copia l'URL `https://hooks.slack.com/services/…`.

- **Secrets** — `webhook_url`

### Telegram

Hai bisogno di due cose: il **token del bot** (segreto) e il **chat
ID** (`chat_id`) dove deve scrivere. Il primo richiede un minuto per
essere ottenuto; il secondo dipende da dove vuoi ricevere gli avvisi.

#### 1. Creare il bot e ottenere il suo token

1. In Telegram, cerca **@BotFather** e clicca *Start*.
2. Invia `/newbot`, poi rispondi alle domande (un nome, poi un nome
   utente che deve finire con `bot`).
3. BotFather termina con un messaggio come questo:

   ```
   Done! Congratulations on your new bot. You will find it at t.me/my_homelab_bot.
   Use this token to access the HTTP API:
   63xxxxxx71:AAFoxxxxn0hwA-2TVSxxxNf4c
   ```

   La riga `63xxxxxx71:AAFoxxxxn0hwA-2TVSxxxNf4c` è il **token del
   bot**. Non condividerlo: dà il controllo completo del bot.

Negli esempi sotto, sostituisci `<TOKEN>` con questo token. Nota il
prefisso `bot` attaccato davanti negli URL:
`https://api.telegram.org/bot<TOKEN>/…`.

#### 2. Trovare il `chat_id`

**Chat privata** (il bot ti scrive direttamente)

1. Apri il tuo bot (`t.me/my_homelab_bot`) e clicca *Start*, oppure
   inviagli un messaggio qualsiasi. Senza questo, non potrà mai
   scriverti.
2. Apri in un browser: `https://api.telegram.org/bot<TOKEN>/getUpdates`
3. Ottieni un documento JSON; cerca `"chat"`:

   ```json
   {"ok":true,"result":[{"update_id":83xxxxx35,
     "message":{"message_id":2643,"from":{…},
       "chat":{"id":21xxxxx38,"first_name":"…","type":"private"},
       "date":1703062972,"text":"/start"}}]}
   ```

   Il `chat_id` è il valore di `chat.id`: qui `21xxxxx38` (un numero
   positivo).

   Se `result` è vuoto (`"result":[]`), invia al bot un altro
   messaggio e ricarica la pagina.

**Canale**

1. Aggiungi il bot come amministratore del canale (Manage channel →
   Administrators → Add).
2. Pubblica un messaggio nel canale.
3. Apri `https://api.telegram.org/bot<TOKEN>/getUpdates`: la risposta
   contiene un blocco `channel_post` con
   `"chat":{"id":-1001xxxxxx062,"type":"channel"}`.

   Il `chat_id` è quel numero negativo, `-100` incluso:
   `-1001xxxxxx062`.

**Gruppo**

Il modo più semplice passa attraverso l'app desktop:

1. Aggiungi il bot al gruppo.
2. Invia un messaggio nel gruppo, clic destro su di esso → *Copy
   message link*. Ottieni `https://t.me/c/194xxxx987/13` (oppure
   `https://t.me/c/194xxxx987/11/13` se il gruppo ha dei topic).
3. Il numero subito dopo `/c/` è l'identificatore interno del gruppo:
   `194xxxx987`. Il `chat_id` da inserire è quel numero **prefissato
   con `-100`**: `-100194xxxx987`.

Senza l'app desktop, il metodo `getUpdates` sopra funziona anch'esso:
`message.chat.id` è direttamente il numero giusto (già negativo). Se il
bot non vede i messaggi del gruppo, chiedi a BotFather `/setprivacy` →
*Disable*, poi rimuovi il bot dal gruppo e aggiungilo di nuovo.

**Topic di gruppo** (gruppi in modalità "Topics")

Il `chat_id` è quello del gruppo. In più, compila **Group topic**
(`message_thread_id`) con il numero centrale del link copiato: in
`https://t.me/c/194xxxx987/11/13`, il topic è `11`. Senza questa
impostazione, il messaggio finisce nel thread *General*.

#### 3. Verificare prima di salvare

Apri nel browser:

```
https://api.telegram.org/bot<TOKEN>/sendMessage?chat_id=<CHAT_ID>&text=test123
```

(aggiungi `&message_thread_id=11` per un topic). Se `test123` arriva,
il token e il `chat_id` sono giusti: inseriscili in DumbMonit e clicca
*Test*.

- **Settings** — `chat_id` (richiesto), `message_thread_id` (topic,
  opzionale), `api_base` (predefinito `https://api.telegram.org`,
  cambialo solo per passare attraverso un relay)
- **Secrets** — `bot_token`

> **"Telegram cannot find this chat"** — per una chat privata, il bot
> non ha mai ricevuto un messaggio da te: invia `/start`. Per un gruppo
> o un canale, probabilmente manca il prefisso `-100`, oppure il bot
> non è stato aggiunto.
>
> **"thread not found"** — il `message_thread_id` non corrisponde ad
> alcun topic del gruppo, oppure il gruppo non è in modalità "Topics".

### Microsoft Teams

**Attenzione: il metodo è cambiato.** I vecchi "Office 365 connectors"
sono stati ritirati nel maggio 2026; un URL
`outlook.office.com/webhook/…` non funziona più.

La procedura attuale:

1. Nel canale Teams, **⋯ → Workflows**.
2. Scegli il template **"Post to a channel when a webhook request is
   received"**.
3. Confirma i passaggi, poi **copia l'URL POST HTTPS** mostrato alla
   fine.

- **Secrets** — `webhook_url`

DumbMonit invia una *Adaptive Card* incartata, il solo formato che
questi flussi mostrano correttamente. Se ottieni un errore che
menziona Power Automate, l'URL probabilmente proviene da un vecchio
connector.

### Matrix

1. Crea un account dedicato a DumbMonit sul tuo homeserver, e
   **invitalo nella stanza**. Un account che non è membro non può
   scrivere.
2. Ottieni il suo token di accesso. In Element: **Settings → Help &
   About → Advanced → Access token**.
3. Ottieni l'identificatore **interno** della stanza — non il suo nome
   visualizzato. In Element: **Room settings → Advanced → Internal
   room ID**. Inizia con `!` (per esempio `!aBcDeF:matrix.org`).

- **Settings** — `server_url` (per esempio `https://matrix.org`),
  `room_id`
- **Secrets** — `token`

### Mattermost

**Integrations → Incoming Webhooks → Add**. Scegli il canale
predefinito, poi copia l'URL.

- **Settings** — `channel` (opzionale, per scrivere altrove che nel
  canale predefinito), `username` (opzionale, predefinito `DumbMonit`)
- **Secrets** — `webhook_url`

### Rocket.Chat

**Administration → Integrations → New integration → Incoming
webhook**. Scegli la stanza, abilita l'integrazione, poi copia il
*Webhook URL*.

- **Settings** — `channel` (opzionale), `alias` (opzionale,
  predefinito `DumbMonit`)
- **Secrets** — `webhook_url`

### Google Chat

Nello spazio Google Chat: **nome dello spazio → Apps & integrations →
Webhooks → Add webhook**. Copia l'URL, che contiene già la chiave e il
token.

- **Secrets** — `webhook_url`

### Zulip

1. Crea un **bot**: **Personal settings → Bots → Add a new bot**,
   tipo "Generic". Annota il suo indirizzo email e la sua chiave API.
2. Iscrivi il bot allo stream di destinazione, altrimenti l'invio
   verrà rifiutato.

- **Settings** — `server_url` (per esempio
  `https://your-org.zulipchat.com`), `email` (l'indirizzo del bot),
  `stream` (il nome dello stream), `topic` (opzionale, predefinito
  `DumbMonit`)
- **Secrets** — `api_key`

### ntfy

Il canale più semplice di tutti: scegli un nome di topic difficile da
indovinare, poi iscriviti ad esso dall'app ntfy.

- **Settings** — `topic`, `server_url` (opzionale, predefinito
  `https://ntfy.sh`)
- **Secrets** — `token` (solo se il tuo server richiede
  autenticazione)

> Sull'istanza pubblica, **chiunque conosca il nome di un topic può
> leggerlo**. Scegli un nome lungo e casuale, oppure ospita il tuo
> proprio server.

### Gotify

Nell'interfaccia Gotify: **Apps → Create Application**. Copia il
token mostrato.

- **Settings** — `server_url` (per esempio `https://gotify.home`)
- **Secrets** — `token`

### Pushover

1. Su <https://pushover.net>, la tua **User Key** è mostrata proprio
   sulla home page.
2. Poi crea un'applicazione: **Create an Application/API Token**.
   Ottieni un **token dell'applicazione**.

- **Settings** — `priority` (opzionale, da -2 a 2), `sound`
  (opzionale), `retry` e `expire` (solo per `priority: 2`)
- **Secrets** — `token` (token dell'applicazione), `user_key` (chiave
  utente)

Senza un'impostazione `priority`, DumbMonit scegli secondo la
severità: `-1` per un advisory o una risoluzione, `0` per un warning,
`1` per un avviso critico. Un advisory quindi non fa suonare il
telefono.

> La priorità `2` ("emergency") ripete la notifica finché non viene
> confermata. DumbMonit aggiunge allora automaticamente `retry` e
> `expire`, senza i quali Pushover rifiuterebbe il messaggio.

### Pushbullet

Su <https://www.pushbullet.com/#settings/account>, clicca **Create
Access Token**.

- **Settings** — `device_iden` (opzionale; senza di esso, tutti i
  tuoi dispositivi vengono notificati)
- **Secrets** — `token`

### Bark (iOS)

Apri l'app Bark sul tuo iPhone: mostra un URL contenente la tua
**chiave del dispositivo**, la lunga stringa di caratteri dopo il
nome di dominio.

- **Settings** — `server_url` (opzionale, predefinito
  `https://api.day.app`), `group` (opzionale), `level` (opzionale),
  `sound` (opzionale)
- **Secrets** — `token` (la chiave del dispositivo)

Senza un'impostazione `level`, DumbMonit sceglie: `passive` per un
advisory, `active` per un warning, `timeSensitive` per un avviso
critico. Il livello `critical`, che attraversa "Do Not Disturb", non
viene mai scelto automaticamente — impostalo esplicitamente se lo
vuoi.

### Web Push

Notifiche native sui tuoi telefoni e computer, direttamente dal tuo
proprio DumbMonit, senza alcuna app da installare e nessun account di
terze parti: il browser (installato come un'app o no) riceve gli
avvisi anche con ogni tab chiusa.

1. Su ogni telefono o computer, apri DumbMonit e vai su **Settings →
   Push notifications → Enable on this device**, poi **Send a test**.
   Su iPhone e iPad, aggiungi prima DumbMonit alla schermata Home. La
   guida passo-passo è in
   [Notifiche push (PWA)](using/push-notifications.md).
2. Crea qui un canale **Web Push**. Non ha alcun segreto: i suoi
   destinatari sono i dispositivi iscritti al passaggio 1.

- **Settings** — `users` (opzionale; un account per riga, vuoto: ogni
  dispositivo iscritto di ogni account), `urgency` (`high` per
  impostazione predefinita, `normal` o `low`), `ttl` (secondi che il
  servizio push mantiene il messaggio per un dispositivo spento,
  predefinito `86400`), `contact` (indirizzo `mailto:` o `https:`
  opzionale dato ai servizi push; vuoto: `DUMBMONIT_PUBLIC_URL` quando
  è HTTPS, altrimenti la pagina del progetto)
- **Secrets** — nessuno. La coppia di chiavi VAPID del server viene
  generata al primo avvio e memorizzata cifrata; la chiave privata non
  lascia mai il server.

Il titolo della notifica è la severità e il dispositivo
(`Critical · nas01`), il corpo un breve riassunto; un tocco apre la
pagina **Alerts**. DumbMonit ha bisogno di HTTPS in uscita verso i
servizi push (`fcm.googleapis.com`, `*.notify.windows.com`,
`updates.push.services.mozilla.com`, `*.push.apple.com`); i messaggi
sono cifrati end-to-end, quei servizi non possono leggerli. Una
sottoscrizione che il servizio push segnala come scomparsa (HTTP 404 o
410, per esempio dopo che l'app è stata rimossa) viene eliminata
automaticamente.

### Apprise

Apprise è un **gateway**: con una sola configurazione dà accesso a
decine di servizi che DumbMonit non implementa da solo. Distribuisci
il container `caronc/apprise`, poi scegli una delle due modalità.

**Modalità consigliata — configurazione memorizzata in Apprise.** Crea
una configurazione nell'interfaccia Apprise, annota la sua chiave, e
non metti nulla altro qui.

- **Settings** — `server_url`, `config_key`

**Modalità diretta — DumbMonit fornisce le destinazioni.** Gli URL
Apprise spesso contengono una password (`mailto://user:password@…`):
vanno quindi nei segreti.

- **Settings** — `server_url`
- **Secrets** — `urls` (uno o più URL Apprise, separati da virgole)

Impostazioni comuni opzionali: `tag` (per mirare solo parte delle
destinazioni di una configurazione) e `format` (`text` per
impostazione predefinita, oppure `markdown`).

> Se il test risponde "Apprise has no configuration under this
> config_key", la chiave non corrisponde a nulla. DumbMonit lo
> segnala invece di fingere che il messaggio sia stato inviato.

### Home Assistant

1. In Home Assistant: **il tuo profilo → Security → Long-lived access
   tokens → Create token**. Copialo, non verrà mostrato di nuovo.
2. Scegli il servizio da chiamare. Per impostazione predefinita,
   DumbMonit crea una notifica persistente sulla dashboard.

- **Settings** — `server_url` (per esempio
  `http://homeassistant.local:8123`), `service` (opzionale,
  predefinito `persistent_notification.create`), `target`
  (opzionale), `data` (opzionale, oggetto unito alla chiamata)
- **Secrets** — `token`

Alcuni servizi utili:

| `service` | Effetto |
|---|---|
| `persistent_notification.create` | Banner nell'interfaccia Home Assistant |
| `notify.notify` | Tutti i target di notifica configurati |
| `notify.mobile_app_<device>` | L'app mobile di un dispositivo specifico |
| `light.turn_on` | Accende una luce — usa `data` per l'entità e il colore |

L'impostazione `data` viene unita **per ultima**: può quindi
sovrascrivere il titolo o il messaggio che DumbMonit ha preparato.

### Email (SMTP)

Il solo canale che non dipende da alcun servizio di terze parti.

- **Settings** — `host`, `from`, `to` (elenco di indirizzi),
  `security` (`starttls` per impostazione predefinita, oppure `tls`,
  oppure `none`), `port` (derivata da `security`: 587, 465 o 25),
  `username` (opzionale)
- **Secrets** — `password`

> Con Gmail, una password dell'account non funziona: crea una
> **password dell'app** nelle impostazioni di sicurezza del tuo
> account Google.

### Signal

Passa attraverso un'istanza
[`signal-cli-rest-api`](https://github.com/bbernhard/signal-cli-rest-api)
che ospiti tu e dove registri prima il tuo numero.

- **Settings** — `server_url` (per esempio
  `http://signal.home:8080`), `number` (il numero mittente
  registrato), `recipients` (elenco di numeri o identificatori di
  gruppo), `username` (opzionale)
- **Secrets** — `password` (opzionale, se la tua istanza è protetta)

Tutti i numeri sono scritti in **formato internazionale**,
`+33612345678`.

### SMS via Twilio

Sulla console Twilio, la home page mostra il tuo **Account SID** e il
tuo **Auth Token**. Poi acquista un numero che possa inviare SMS.

- **Settings** — `account_sid`, `from` (il tuo numero Twilio)
  **oppure** `messaging_service_sid`, `to` (elenco di destinatari)
- **Secrets** — `token` (l'*Auth Token*)

Il corpo dell'SMS viene troncato a 300 caratteri: oltre, Twilio lo
divide in segmenti fatturati separatamente.

> Su un account di prova, Twilio invia solo ai numeri che hai
> verificato.

### PagerDuty

In PagerDuty: **Services → il tuo servizio → Integrations → Add
integration**, e scegli **Events API v2**. Copia la *Integration
Key*.

- **Settings** — `region` (`eu` se il tuo account è ospitato in
  Europa), `source` (opzionale, predefinito `dumbmonit`)
- **Secrets** — `token` (la *Integration Key*)

Un avviso apre un incidente; la sua risoluzione **lo chiude
automaticamente**. La corrispondenza avviene sull'impronta
dell'avviso, prefissata con `dumbmonit-`, così un ricordo non apre mai
un secondo incidente.

> Una chiave di un altro tipo ("Events API v1", "REST API") riceve un
> rifiuto secco. Verifica che l'integrazione sia davvero di tipo
> *Events API v2*.

### Opsgenie

**Teams → il tuo team → Integrations → Add integration → API**. Copia
la chiave API.

- **Settings** — `region` (`eu` per un account europeo), `responders`
  (elenco di nomi di team, opzionale), `tags` (elenco, opzionale)
- **Secrets** — `api_key`

Come con PagerDuty, una risoluzione chiude l'avviso, identificato dal
suo alias.

> Atlassian ha fissato la **fine del servizio di Opsgenie al 5 aprile
> 2027**. Questo canale resta per le installazioni esistenti; per una
> nuova, preferisci PagerDuty o un canale personalizzato che punta a
> Jira Service Management.

### Webhook personalizzato

Vedi [Il canale personalizzato](#the-custom-channel) sopra.

---

## Risoluzione dei problemi

**Il test fallisce immediatamente, con "missing setting …".**
Un campo richiesto è vuoto. Il messaggio nomina l'impostazione esatta.

**Il test fallisce con "no answer within 15 s".**
Il server di destinazione è irraggiungibile da DumbMonit. Se è un
servizio sulla tua rete locale, verifica che il container DumbMonit
possa raggiungerlo — questo è spesso un problema di networking Docker,
non un problema di configurazione del canale.

**Il test riesce, ma non ricevo nulla.**
Il servizio ha accettato il messaggio e lo ha inviato altrove: canale
errato, topic ntfy errato, dispositivo Pushbullet errato. Verifica
l'impostazione di destinazione.

**Il messaggio di errore contiene `***`.**
È intenzionale: il tuo token è stato rimosso dal messaggio prima che
venisse registrato. Nessun segreto appare mai nei log, nella colonna
"last error", o in una risposta dell'API.

**Dove posso vedere l'ultimo errore di un canale?**
Nell'elenco dei canali: ogni riga mostra la data dell'ultima consegna
riuscita e, se presente, l'errore dell'ultimo fallimento.
