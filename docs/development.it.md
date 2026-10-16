# Sviluppo

DumbMonit è un unico binario Rust (`crates/server`, che incorpora la build di
SvelteKit) più VictoriaMetrics per le serie temporali, avviato dal server a
partire dal binario fornito nell'immagine, e un database SQLite incorporato per
configurazione e stato. Un solo container. Il codice Rust e i commenti sono in
francese; l'interfaccia web è in inglese.

## Struttura del progetto

```
crates/proto     shared types: Sample, Target, Credential, trait Collector (+ ProbeError)
crates/server    the binary
  api/           axum routes (mod.rs is the route table); spa.rs serves web/build
  auth/          single instance password, HttpOnly session cookie, rate limit
  collectors/    snmp (profiles/*.yaml loaded at startup), proxmox, pbs, synology, agent, uptime
  scheduler.rs   runs every enabled target on its interval through the collector registry
  tsdb/          VictoriaMetrics writer (batched flush) + query proxy; embedded.rs supervises the child VictoriaMetrics
  db/            SQLite + numbered migrations in db/migrations/
  alerting/      rules, state machine (Phase/EffectivePhase), suppression by parent, silences, seasonal baseline
  notify/        22 notification channels, described to the UI by notify/catalog.rs
  crypto.rs      AES-256-GCM for credentials/tokens, key derived from /data/secret.key or DUMBMONIT_SECRET
crates/agent     Linux/macOS/FreeBSD/Windows agent; install/ holds install.sh/.ps1 served by the server
web/             SvelteKit (Svelte 5 runes, Tailwind 4, uPlot), adapter-static SPA fallback, ssr=false
profiles/        SNMP collection profiles, auto-applied by sysObjectID
docs/            this documentation (MkDocs, published on Read the Docs)
```

## Compilare senza una toolchain Rust

La compilazione avviene in Docker; sull'host non serve installare nulla di Rust.

```bash
# Full stack (builds the image: about 10 min cold) — UI on http://localhost:8080
docker compose -f docker-compose.yml -f docker-compose.dev.yml up -d --build

# Rust, inside the builder stage (musl/alpine)
docker build -t dumbmonit-devenv --target builder .
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test -p dumbmonit-server --test alerts_api   # one integration test file
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test -p dumbmonit-server test_name          # one test by name
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo clippy --all-targets --all-features -- -D warnings
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo fmt --all --check
```

Con Rust installato in locale, `cargo test` e `cargo clippy -- -D warnings`
funzionano direttamente. Edition 2024, MSRV 1.95; `rustfmt.toml` imposta
`max_width = 100` e `use_small_heuristics = "Max"`.

La build Docker è a livelli: build web → dipendenze Rust su sorgenti fittizi →
sorgenti reali (con un `touch` prima della seconda `cargo build`, altrimenti
cargo riutilizza gli artefatti fittizi) → agent compilato in cross-compilation
con cargo-zigbuild per `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`
e `x86_64-pc-windows-gnu` → `FROM scratch`. `web/build` deve esistere prima di
`cargo build`, altrimenti il server viene distribuito senza interfaccia.

## Interfaccia web

Node 22 è sufficiente sull'host.

```bash
cd web && npm run dev        # Vite on :5173, proxies /api to localhost:8080 (the Docker server)
cd web && npm run check      # svelte-check (CI runs this)
cd web && npm run build      # static build into web/build, embedded by rust_embed at cargo build
```

Convenzioni: solo rune di Svelte 5 (`$props`, `$state`, `$derived`, `$effect`,
snippet, `onclick`). Tutto l'accesso alla rete passa da `src/lib/api/`; i
componenti non usano mai `fetch`. I tipi in `api/types.ts` rispecchiano
esattamente le struct Rust e devono restare sincronizzati con
`crates/server/src/api/*.rs`. Il design system si trova in `src/app.css` (token)
e in `src/lib/ui/` (primitive). Non eseguire mai due `vite build` contemporaneamente:
cancellano `.svelte-kit/output`.

## Traduzioni

L'interfaccia web si traduce con una pull request, come ogni altro contributo. La lingua di partenza è l'inglese (`web/messages/en.json`: chiavi piatte, segnaposto `{param}`); ogni altra lingua è un file, `web/messages/<locale>.json`, compilato da [Paraglide JS](https://paraglidejs.com) in funzioni di messaggio con tree-shaking sotto `web/src/lib/paraglide/` (generato, non tracciato).

1. Modifica `web/messages/<locale>.json` (`fr`, `de`, `es`, `it`, `pt`, `pt-BR`, `ru`, `zh-Hans`), oppure copia `en.json` per aggiungere una nuova lingua.
2. Mantieni tutte le chiavi e tutte le `{variables}` di `en.json`; traduci solo i valori. Nessun markup HTML nei messaggi.
3. I plurali usano la sintassi dei messaggi, non stringhe costruite a mano: mantieni la struttura `match`/`plural` del messaggio inglese e traduci ogni ramo.
4. Esegui `cd web && npm run i18n:check`: fallisce se a una lingua manca una chiave o ne cambia le `{variables}`.
5. Apri una pull request.

Le nuove stringhe dell'interfaccia vanno prima in `web/messages/en.json` (l'unico file che serve a una pull request di funzionalità; `npm run i18n:merge` unisce frammenti temporanei). Una lingua compare nel selettore (Impostazioni, Aspetto) appena il suo file esiste. La convenzione sui nomi delle chiavi è in `CONTRIBUTING.md`.

`web/project.inlang/settings.json` punta il suo unico plugin (message-format) a
`./node_modules/@inlang/plugin-message-format/dist/index.js` anziché all'URL
jsdelivr usato per impostazione predefinita dagli strumenti di inlang: un
percorso relativo, non `http`, viene letto direttamente dal disco, quindi
`npm run build` non contatta mai una CDN — importante perché la build
dell'immagine non ha garanzie di una cache npm calda che offra quell'URL, e uno
strumento per homelab che ha bisogno di internet per ricompilarsi fa una brutta
figura.

## Test e CI

`.github/workflows/ci.yml` esegue, a ogni push e pull request:

1. `cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-features`;
2. `cargo deny check licenses bans` — `deny.toml` consente solo licenze approvate
   dall'OSI; una dipendenza copyleft o source-available fa fallire la build;
3. `npm run check` e `npm run build`;
4. l'immagine (`linux/amd64`), solo build.

`.github/workflows/release.yml` pubblica la stessa immagine su
`ghcr.io/laupernoe/dumbmonit`: `:edge` a ogni push su `main`, `:X.Y.Z`,
`:X.Y`, `:X` e `:latest` a ogni tag `v*`.

I test di integrazione si trovano in `crates/server/tests/`; i moduli puri
(conversione delle metriche, parsing delle opzioni, macchina a stati degli
avvisi, silenziamenti) sono testati sul posto senza rete.

## Aggiungere un collector

1. Implementa `Collector` da `crates/proto/src/collector.rs`: `kind()`,
   `probe(&Target) -> Result<Vec<Sample>, ProbeError>` e, facoltativamente,
   `discover()` per il rilevamento dei profili. Restituisci le varianti di
   `ProbeError` in modo veritiero: `means_down()` decide se un errore conta come
   "monitor non raggiungibile" (con avviso) o come errore di configurazione
   (mostrato sul monitor, non notificato).
2. Leggi le tue opzioni da `Target::tags` in un `options.rs`, con valori predefiniti.
3. Registralo in `crates/server/src/main.rs` con `registry.register(...)`.
   Lo scheduler e l'API non conoscono mai i tipi concreti.
4. Descrivilo in `crates/server/src/api/collectors.rs`: etichetta, riepilogo,
   esempi, tipi di credenziali, suggerimento per l'indirizzo, avviso di
   configurazione e l'elenco `options` (key, label, help, placeholder, default,
   input, choices). L'interfaccia costruisce il proprio modulo a partire da
   questo; un test verifica che ogni tipo registrato abbia un avviso e che le
   chiavi delle opzioni corrispondano a quelle lette dal collector.
5. Nomina le metriche `<what>_<unit>` senza il prefisso `dumbmonit_` (lo aggiunge
   il writer) e aggiungi ogni nuova metrica che una regola integrata prende di
   mira all'elenco nei test di `alerting/rules.rs`.
6. Aggiungi una pagina sotto `docs/devices/`.

I campioni sono `Sample::new(name, value, MetricKind::Gauge | Counter,
timestamp_ms)` con `.with_label(k, v)`; le etichette di identità (`target`,
`host`, `tag_*`) sono aggiunte dal registry. Non emetterle tu stesso: dai a
un'etichetta per oggetto il nome dell'oggetto (`proxy_host`, `esxi`,
`ceph_host`). Un'etichetta in collisione viene conservata come
`exported_<name>`, e le opzioni elencate per il tipo in `api/collectors.rs` non
vengono copiate come `tag_*`.

## Aggiungere un canale di notifica

1. Implementa `Notifier` (`crates/server/src/notify/mod.rs`): `kind()` e
   `async fn send(&Message) -> Result<(), NotifyError>`. I canali web
   condividono il client HTTP e gli helper in `notify/http.rs`; guarda
   `services.rs`, `chat.rs`, `push.rs`, `oncall.rs` e `smtp.rs` per quelli simili.
2. Aggiungi il tipo a `CHANNEL_KINDS` e al `match` di `build` in
   `notify/mod.rs`, che trasforma un `ChannelConfig` memorizzato (impostazioni +
   segreti decifrati) in un `Notifier`.
3. Descrivilo in `notify/catalog.rs`: etichetta, riepilogo, ancora `doc_url`, e
   i campi `settings` e `secrets` (key, label, required, input, help,
   placeholder, options, shape, default). L'interfaccia genera il modulo da
   questa descrizione, quindi non serve alcun rilascio dell'interfaccia.
4. Documentalo in `docs/notifications.md` sotto *Canali supportati*, con un
   titolo il cui slug corrisponda all'ancora `doc_url`: l'interfaccia rimanda
   direttamente a quell'ancora su Read the Docs.

## Screenshot headless

`web/tools/README.md` descrive come renderizzare ogni pagina in un Chromium
headless eseguito in Docker (`zenika/alpine-chrome:with-puppeteer`, nessun
browser locale necessario): accedi con `curl` per ottenere il cookie
`dumbmonit_session`, poi esegui `web/tools/screenshot.js` sul server di anteprima
o direttamente su `:8080`. Scrive `web/shots/<page>-<theme>-<viewport>.png` per
entrambi i temi e entrambi i viewport, ed è così che l'interfaccia viene
controllata dopo una modifica e così sono stati realizzati gli screenshot di
questa documentazione.

## Documentazione

Questo sito è costruito con MkDocs e Material for MkDocs a partire da `docs/`
(`mkdocs.yml` nella radice, `.readthedocs.yaml` per Read the Docs). Per
l'anteprima:

```bash
pip install -r docs/requirements.txt
mkdocs serve
mkdocs build --strict   # what Read the Docs runs; must pass with no warnings
```
