# Architettura

DumbMonit è distribuito come un unico container. Al suo interno c'è un solo
processo Rust che serve l'interfaccia web e l'API, più un processo figlio
(VictoriaMetrics) che il processo Rust avvia e supervisiona. Configurazione e
stato risiedono in SQLite.

## Componenti

| Componente | Dove | Ruolo |
|---|---|---|
| Server | `crates/server` | L'unico binario Rust: API HTTP (axum), autenticazione, scheduler, motore di alerting, canali di notifica, report. |
| Interfaccia web | `web/` | App SvelteKit a pagina singola (Svelte 5, Tailwind 4, uPlot). Compilata in file statici e incorporata nel binario del server con `rust_embed`. |
| Serie temporali | `crates/server/src/tsdb/` | Writer a lotti e proxy di query per VictoriaMetrics. `embedded.rs` avvia e supervisiona il processo figlio VictoriaMetrics incluso (dati in `/data/vm`); `DUMBMONIT_VM_URL` punta invece a un'istanza esterna. |
| Configurazione e stato | `crates/server/src/db/` | SQLite con migrazioni numerate: dispositivi, credenziali, utenti, regole, stato degli alert, silenziamenti, canali. |
| Collector | `crates/collectors`, `crates/server/src/collectors/` | Un'implementazione del trait `Collector` per ogni tipo di dispositivo o servizio, registrata in un `Registry`. Lo scheduler conosce solo il trait. |
| Tipi condivisi | `crates/proto` | `Sample`, `Target`, `Credential`, il trait `Collector` e `ProbeError`, e il protocollo degli agent. |
| Agent | `crates/agent` | Agent per Linux, macOS, FreeBSD e Windows che invia le metriche dell'host; può anche fare da relay, eseguendo i collector per un sito remoto. |
| Alerting | `crates/server/src/alerting/` | Regole, macchina a stati degli alert, soppressione tramite genitore, silenziamenti, baseline apprese. |
| Notifiche | `crates/server/src/notify/` | Canali (chat, push, reperibilità, e-mail, webhook), descritti all'interfaccia da `catalog.rs`. |
| Segreti | `crates/server/src/crypto.rs` | AES-256-GCM per credenziali e token salvati in SQLite; la chiave proviene da `/data/secret.key` o da `DUMBMONIT_SECRET`. |

## Flusso dei dati

```text
 devices / services ──(SNMP, HTTP APIs, probes)──► collectors ┐
                                                              │ samples
 agents ──(push, per-machine token)──► /api/ingest ───────────┤
 relay agent (collectors run at a remote site) ───────────────┤
                                                              ▼
                                           tsdb writer (batched) ──► VictoriaMetrics
                                                                           │
 browser / scripts / assistants ◄─ /api/* ◄─ query proxy ◄─────────────────┘
                                   │
                                   └─► alerting engine ──► notify channels

 SQLite (config, users, alert state, silences) is read and written by the
 API, the scheduler and the alerting engine.
```

1. Lo scheduler esegue ogni dispositivo abilitato al proprio intervallo tramite
   il registro dei collector. Agent e relay inviano le loro misurazioni a
   `/api/ingest` con un token per macchina.
2. I collector restituiscono campioni chiamati `dumbmonit_<kind>_*` con
   un'etichetta `target` (l'id del dispositivo). Il writer li raggruppa in
   lotti verso VictoriaMetrics.
3. Il motore di alerting valuta le regole sulle serie memorizzate. `phase` è lo
   stato del motore; `effective_phase` è ciò che vedono gli utenti
   (`suppressed` quando un dispositivo genitore è giù). Un dispositivo
   irraggiungibile genera un alert; un errore di configurazione viene mostrato
   ma non notificato.
4. Le notifiche partono attraverso i canali configurati nella policy di
   notifica.
5. L'interfaccia e qualsiasi script leggono tutto tramite `/api/*`; non esiste
   una metà privata dell'API.

## Punti di estensione

- Un nuovo tipo di dispositivo è un'implementazione di `Collector` più una
  chiamata a `Registry::register`. I moduli dell'interfaccia sono generati da
  `GET /api/collectors`.
- Un nuovo canale di notifica è un sender più una voce in `notify/catalog.rs`;
  l'interfaccia genera il modulo a partire da `GET /api/notify/kinds`.
- Un dispositivo SNMP che richiede solo un profilo non richiede Rust: basta un
  file YAML in `profiles/`.

Consulta [Sviluppo](../development.md) per le istruzioni di compilazione e il
[Modello di sicurezza](security-model.md) per i confini di fiducia.
