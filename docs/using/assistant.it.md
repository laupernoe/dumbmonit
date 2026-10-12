# Collegare un assistente

DumbMonit ha un server [MCP](https://modelcontextprotocol.io) integrato:
il Model Context Protocol, il modo in cui assistenti come Claude,
ChatGPT, Cursor o l'agent di VS Code chiamano strumenti esterni.
Collegane uno, poi chiedigli cose come:

- "Tutto va bene?"
- "Silenzia il NAS per due ore, sto cambiando un disco."
- "Cosa è successo stanotte?"
- "Quanto è pieno il server di backup, e sta peggiorando?"
- "Pubblica un aggiornamento sulla pagina di stato: il server di posta
  è tornato."

L'assistente vede ciò che vede l'interfaccia — dispositivi, avvisi,
cronologia, metriche, agent, pagine di stato — e usa le stesse parole:
*reporting*, *unreachable*, *waiting*, *advisory*, *warning*,
*suppressed by parent*. Funziona dentro il server: nulla da installare
accanto a DumbMonit, e gli strumenti chiamano lo stesso codice
dell'interfaccia web.

## 1. Creare un token

Settings → **API & assistants** (`/settings#assistant`). Dai al token
il nome dell'assistente o della macchina su cui funziona ("Claude on
my laptop"), poi scegli:

| Impostazione | Scelte |
| --- | --- |
| **Scope** | **Read**: guarda — stato, dispositivi, avvisi, cronologia, metriche, regole, agent, pagine di stato. Non può mai modificare nulla. **Read and write**: agisce anche — silenzia, conferma, sonda, aggiunge un dispositivo, riavvia un container, pubblica un incidente… (gli [strumenti di scrittura](#what-the-assistant-can-call)). |
| **Expires** | 30 giorni, 90 giorni (predefinito), 1 anno o mai. Un token scaduto smette di funzionare; creane uno nuovo. |
| **Allowed networks** | Opzionale. Indirizzi o intervalli CIDR da cui il token può essere usato (`192.168.1.0/24, 10.8.0.5`). Vuoto: ovunque. |

Il token ha la forma `dmt_` seguito da 32 caratteri esadecimali. Viene
mostrato **una volta**, subito dopo la creazione; DumbMonit mantiene
solo un hash. Se lo perdi, revocalo e creane un altro.

Preferisci un token read a meno che tu non voglia davvero che
l'assistente agisca.

Lo stesso token apre anche l'[API REST](../reference/api.md#authentication)
a script e dashboard — `curl -H "Authorization: Bearer dmt_…"
https://monit.example.lan/api/targets` — dove `read` vede ciò che vede
un visualizzatore e `write` fa ciò che fa un amministratore, account,
token e backup esclusi.

## 2. Incollare lo snippet

La pagina delle impostazioni compila questi con l'indirizzo del tuo
server e il nuovo token. Sotto, `https://monit.example.lan` sta per il
tuo server.

### Claude Code

```sh
claude mcp add --transport http dumbmonit https://monit.example.lan/api/mcp \
  --header "Authorization: Bearer dmt_…"
```

Aggiungi `--scope user` per averlo in ogni progetto, non solo
nell'attuale.

### Claude Desktop

Claude Desktop avvia server locali dal suo file di configurazione;
[`mcp-remote`](https://www.npmjs.com/package/mcp-remote) lo collega
all'endpoint HTTP di DumbMonit (richiede [Node.js](https://nodejs.org)).
DumbMonit si autentica con un header bearer invece che OAuth, quindi
aggiungilo tramite questo file invece che la schermata Connectors.
Settings → Developer → **Edit Config**, poi aggiungi a
`claude_desktop_config.json` e riavvia Claude Desktop:

```json
{
  "mcpServers": {
    "dumbmonit": {
      "command": "npx",
      "args": [
        "-y", "mcp-remote", "https://monit.example.lan/api/mcp",
        "--header", "Authorization:${DUMBMONIT_AUTH}"
      ],
      "env": { "DUMBMONIT_AUTH": "Bearer dmt_…" }
    }
  }
}
```

L'header passa attraverso una variabile d'ambiente, senza spazio dopo i
due punti, perché alcune piattaforme dividono gli argomenti sugli
spazi. Per un server raggiunto su HTTP semplice sulla tua rete,
aggiungi `"--allow-http"` ad `args` — la pagina delle impostazioni lo
fa per te quando essa stessa è servita su HTTP.

### VS Code

`.vscode/mcp.json` in un workspace (oppure il comando **MCP: Add
Server** → HTTP):

```json
{
  "servers": {
    "dumbmonit": {
      "type": "http",
      "url": "https://monit.example.lan/api/mcp",
      "headers": { "Authorization": "Bearer dmt_…" }
    }
  }
}
```

### Cursor e altri client

`.cursor/mcp.json`, e la stessa forma per la maggior parte dei client
che supportano server Streamable HTTP con header personalizzati:

```json
{
  "mcpServers": {
    "dumbmonit": {
      "url": "https://monit.example.lan/api/mcp",
      "headers": { "Authorization": "Bearer dmt_…" }
    }
  }
}
```

Un client che avvia solo server locali (stdio) può usare il bridge
`mcp-remote` mostrato per Claude Desktop.

### ChatGPT

Settings → Connectors → **Create** (modalità sviluppatore). MCP server
URL: `https://monit.example.lan/api/mcp`, header di autorizzazione:
`Bearer dmt_…`.

ChatGPT si connette dai server di OpenAI, non dal tuo browser:
DumbMonit deve essere **raggiungibile da internet su HTTPS**. Vedi le
[note di sicurezza](#security-notes) prima di farlo.

### Qualsiasi altro client

Cosa deve sapere un client: l'endpoint è `https://<server>/api/mcp`, il
trasporto è Streamable HTTP, e ogni richiesta porta
`Authorization: Bearer dmt_…`. Una chiamata grezza, per verificare il
token e il percorso di rete da un terminale:

```bash
curl -s https://monit.example.lan/api/mcp \
  -H 'Authorization: Bearer dmt_…' \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json, text/event-stream' \
  -H 'MCP-Protocol-Version: 2026-07-28' \
  -H 'Mcp-Method: tools/call' -H 'Mcp-Name: get_status' \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_status","arguments":{},
       "_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28",
                "io.modelcontextprotocol/clientCapabilities":{}}}}'
```

## Cosa può chiamare l'assistente

Ventisette strumenti. Ognuno ha un titolo, una descrizione, uno schema
di input, uno schema di output e annotazioni (`readOnlyHint`,
`destructiveHint`, `idempotentHint`), così un client può distinguere
quali guardano solo e chiederti prima di eseguire gli altri.

| Strumento | Scope | Fa |
| --- | --- | --- |
| `get_status` | read | Il bollettino, la risposta a "tutto va bene?": conteggi di dispositivi, avvisi attivi e in attesa, una frase ("2 advisories, 1 unreachable."). |
| `list_devices` | read | Dispositivi e servizi con stato, genitore e tag; filtra per sottostringa o stato. |
| `get_device` | read | Un dispositivo: configurazione, stato, avvisi attivi, riepilogo CPU/memoria/disco più pieno delle 24 ore (disponibilità e latenza per un controllo di servizio). |
| `list_alerts` | read | Avvisi attivi con parole di severità, da quando, valore, soppressione, conferma. |
| `alert_history` | read | Transizioni di avviso, le più recenti prima (`since` oppure `hours`, dispositivo opzionale, limite). |
| `query_metrics` | read | Una query di intervallo MetricsQL, al massimo 60 punti per serie. |
| `list_silences` | read | Finestre di manutenzione e se sono attive ora. |
| `list_rules` | read | Regole di avviso con tipo, severità, soglia, abilitata. |
| `list_device_types` | read | I tipi di dispositivo che questo server può monitorare, con le opzioni e le credenziali che ciascuno si aspetta — ciò di cui ha bisogno `add_device`. |
| `list_agents` | read | Macchine che eseguono l'agent: OS, versione dell'agent, modalità relay, sito, dispositivi relayati, ultimo contatto, binding. |
| `list_containers` | read | I container Docker di una macchina che esegue l'agent: stato, salute, riavvii, aggiornamento disponibile, ultimo comando. |
| `list_heartbeats` | read | Monitor heartbeat: intervallo atteso, ultima chiamata, verdetto (waiting, on time, missed, reported down). L'URL segreto non viene mai mostrato. |
| `list_status_pages` | read | Pagine di stato, pubblicate o no, con il loro percorso pubblico (`/s/<slug>`), i dispositivi che mostrano e i loro incidenti aperti. |
| `list_channels` | read | Canali di notifica: tipo, abilitato, ultimo errore, ultimo messaggio inviato. Mai i loro segreti. |
| `list_packs` | read | Pacchetti di integrazione installati: versione, tipo di dispositivo, abilitato, dispositivi che lo usano, errori. |
| `silence_device` | write | Una finestra di manutenzione puntuale su un dispositivo, a partire da ora (predefinito 1 ora, al massimo una settimana). |
| `remove_silence` | write | Rimuove una finestra di manutenzione. |
| `schedule_maintenance` | write | Una finestra di manutenzione puntuale a una data data, su un dispositivo o l'intera istanza, al massimo una settimana. |
| `acknowledge_alert` | write | Conferma un avviso per fingerprint per un numero di ore (predefinito 4): i ricordi si fermano, la risoluzione viene comunque notificata. `hours: 0` la rimuove. |
| `probe_device` | write | Sonda un dispositivo immediatamente e riporta cosa è stato misurato. |
| `set_device_enabled` | write | Abilita o disabilita il monitoraggio di un dispositivo. |
| `set_rule_enabled` | write | Abilita o disabilita una regola di avviso. |
| `add_device` | write | Aggiunge un dispositivo: nome, tipo, indirizzo, intervallo, genitore, opzioni, credenziali, agent relay. |
| `discover_network` | write | Scansiona una rete (CIDR, al massimo 4096 indirizzi) per dispositivi SNMP e suggerisce un profilo per ciascuno. |
| `restart_container` | write | Chiede all'agent di una macchina di riavviare uno dei suoi container. |
| `test_channel` | write | Invia un messaggio di test tramite un canale di notifica. |
| `post_incident` | write | Apre un incidente su una pagina di stato, oppure pubblica un aggiornamento su uno esistente. |

Un token read che chiama uno strumento write riceve un rifiuto chiaro
come risultato dello strumento (`isError: true`); l'assistente lo
spiega e nulla cambia. Lo stesso vale per qualsiasi cosa lo strumento
non possa fare — un dispositivo sconosciuto, un nome ambiguo, una
finestra più lunga di una settimana: il messaggio dice cosa correggere.

## Dettagli del protocollo

`POST /api/mcp` implementa MCP sul trasporto **Streamable HTTP**:
JSON-RPC 2.0, una richiesta per `POST`, risposta con JSON semplice
(nessun server-sent event). È stateless: nessun `Mcp-Session-Id` viene
emesso, uno inviato da un client viene ignorato, e `GET` o `DELETE`
sull'endpoint rispondono `405`. Solo la capability `tools` viene
offerta — nessuna risorsa, prompt, sampling o OAuth.

Due generazioni del protocollo vengono servite sullo stesso endpoint:

| Versioni | Come un client lo parla |
| --- | --- |
| `2026-07-28` | Stateless. Ogni richiesta porta `io.modelcontextprotocol/protocolVersion` e `io.modelcontextprotocol/clientCapabilities` in `params._meta`, e gli header `MCP-Protocol-Version`, `Mcp-Method` e, per `tools/call`, `Mcp-Name`, che deve corrispondere al corpo. Metodi: `server/discover`, `tools/list`, `tools/call`. I risultati portano `resultType: "complete"` e l'identità del server in `_meta` (`io.modelcontextprotocol/serverInfo`); `tools/list` porta anche `ttlMs` e `cacheScope`. |
| `2025-11-25`, `2025-06-18`, `2025-03-26` | L'handshake `initialize` scelga la versione (una sconosciuta ottiene la più recente di queste tre); `notifications/initialized` viene confermato con `202`; poi `ping`, `tools/list`, `tools/call`. |

Errori, oltre ai `401`, `403` e `429` dei [controlli sul token](#security-notes),
che vengono prima di qualsiasi JSON-RPC:

| HTTP | Codice JSON-RPC | Quando |
| --- | --- | --- |
| `400` | `-32020` (header mismatch) | Una richiesta `2026-07-28` senza `MCP-Protocol-Version`, `Mcp-Method` o `Mcp-Name`, oppure con valori che differiscono dal corpo. |
| `400` | `-32022` (unsupported protocol version) | Una versione che questo server non parla; `error.data` elenca quelle `supported` e quella `requested`. |
| `400` | `-32602` (invalid params) | Una richiesta `2026-07-28` priva dei campi `_meta` richiesti. |
| `404` | `-32601` (method not found) | Una richiesta `2026-07-28` per un metodo che questo server non implementa. |
| `200` | `-32602` | Uno strumento sconosciuto, oppure `arguments` che non è un oggetto. |
| | `-32700`, `-32600` | JSON illeggibile; non una richiesta JSON-RPC 2.0 (i batch non sono supportati). |
| `202` | — | Una notifica: accettata, nessun corpo. |

Uno strumento che funziona ma fallisce — scope mancante, dispositivo
sconosciuto — non è un errore di protocollo: risponde normalmente con
`isError: true` e una frase che l'assistente può riportare.

## Note di sicurezza

- **Un token è una password.** Chiunque lo possieda legge tutto ciò
  che mostra l'interfaccia — e, con un token write, modifica qualsiasi
  cosa possa fare un amministratore tramite l'API REST (dispositivi,
  regole, canali), account, token e backup esclusi. Tienilo fuori da
  chat condivise, screenshot e repository. Revoca un token di cui non
  sei sicuro; creane un altro in un minuto.
- **Preferisci read.** La maggior parte delle domande ("tutto va
  bene?") non richiede alcun scope write. Crea un token write solo per
  un assistente che vuoi davvero che agisca, e nominalo così lo
  riconosci nell'elenco. La maggior parte dei client chiede prima di
  eseguire uno strumento che non è marcato read-only; lascia attiva
  quella conferma.
- **Dagli una durata e una rete.** Un token che scade in 90 giorni e
  funziona solo dalla tua LAN o VPN (`192.168.1.0/24`) vale poco per
  chiunque lo trovi. Dietro un reverse proxy, dichiara il proxy in
  `DUMBMONIT_TRUSTED_PROXIES`, altrimenti ogni chiamata appare
  provenire dal proxy e una restrizione di rete le rifiuta tutte.
- **Legato al suo creatore.** Un token smette di funzionare quando
  l'account che lo ha creato viene disattivato, perde il suo potere di
  scrittura quando quell'account non è più admin, e viene revocato
  quando quell'account viene eliminato.
- **Nulla di segreto ritorna.** Gli strumenti non restituiscono mai le
  credenziali dei dispositivi (solo il loro tipo), i segreti dei
  canali, gli URL heartbeat o i token. `add_device` prende sì
  credenziali in ingresso: viaggiano attraverso la conversazione del
  tuo assistente, e quindi attraverso il suo provider. Per un
  dispositivo con credenziali sensibili, preferisci l'interfaccia web.
- **Browser e `Origin`.** Una richiesta che porta un header `Origin`
  (un client basato su browser) viene rifiutata con `403` a meno che
  quell'origine non sia quella propria del server oppure sia
  consentita da `DUMBMONIT_API_CORS_ORIGINS` (per impostazione
  predefinita qualsiasi origine, poiché il token è comunque richiesto).
  Elenca le tue origini lì per restringerlo; vedi
  [richieste cross-origin](../reference/api.md#cross-origin-requests-cors).
- **Limite di tasso.** Ogni token è limitato a 120 chiamate al minuto,
  REST e MCP insieme: un assistente bloccato in un loop ottiene `429`
  con `Retry-After`, non l'attenzione completa del tuo server.
- **HTTPS prima di internet.** I connettori di ChatGPT richiedono un
  URL pubblico. Metti DumbMonit dietro un reverse proxy con TLS
  (Caddy, Traefik, nginx…) e imposta `DUMBMONIT_COOKIE_SECURE=1`. Non
  esporre mai la porta HTTP semplice. Claude Desktop, Claude Code, VS
  Code e Cursor funzionano sulla tua macchina e possono raggiungere un
  indirizzo LAN direttamente, quindi non hanno bisogno di alcuna
  esposizione.
- **Osserva "last used".** L'elenco dei token mostra quando ogni token
  è stato usato l'ultima volta e da quale indirizzo. Un token usato in
  un momento in cui non stavi parlando con il tuo assistente, oppure
  da un indirizzo che non conosci, merita una revoca.
- **Log.** Ogni chiamata di strumento viene registrata a livello
  `info` con il nome del token e lo strumento, mai gli argomenti; ogni
  scrittura tramite l'API REST con il nome del token, il metodo, il
  percorso e lo stato, mai il corpo. Creare e revocare token è nel log
  di audit (Settings → Security).

## Risoluzione dei problemi

| Sintomo | Causa e correzione |
| --- | --- |
| `401` / "A valid API token is required." | L'header manca o è scritto male (`Authorization: Bearer dmt_…`, uno spazio dopo `Bearer`), oppure il token è stato revocato. Creane uno nuovo. |
| `401` / "The API token "…" expired on …" | Crea un nuovo token; scegli una durata più lunga se era troppo corta. |
| `403` / "cannot be used from …" | Il token è limitato ad altre reti. L'indirizzo nel messaggio è quello che il server vede: dietro un reverse proxy, dichiara il proxy in `DUMBMONIT_TRUSTED_PROXIES`. |
| `403` su ogni chiamata da un client basato su browser | La sua `Origin` non è consentita: aggiungila a `DUMBMONIT_API_CORS_ORIGINS`. |
| Uno strumento risponde "needs a token with the "write" scope" | Il token è di sola lettura, oppure il suo creatore non è più un amministratore. Crea un token write se vuoi che l'assistente agisca. |
| `429` | 120 chiamate in un minuto: l'assistente probabilmente sta ciclando. Attendi `Retry-After`, poi fai una domanda più ristretta. |
| "Unsupported protocol version" | Il client parla una versione che DumbMonit non conosce; aggiorna il client. |
| Claude Desktop mostra il server come fallito | Node.js deve essere installato per `npx`; su HTTP semplice, `--allow-http` deve essere in `args`. |
| ChatGPT non riesce a connettersi | Richiede un URL HTTPS pubblico; un indirizzo LAN o HTTP semplice non funziona dai server di OpenAI. |
