# Sviluppo

DumbMonit è un binario Rust (`crates/server`, che include la build
SvelteKit) più VictoriaMetrics per le serie temporali, avviato dal server
dal binario fornito nell'immagine, e un database SQLite integrato per
configurazione e stato. Un container. Il codice Rust e i commenti sono
in francese; l'interfaccia web è in inglese.

## Struttura del progetto

```
crates/proto     tipi condivisi: Sample, Target, Credential, trait Collector (+ ProbeError)
crates/server    il binario
  api/           rotte axum (mod.rs è la tabella delle rotte); spa.rs serve web/build
  auth/          password per istanza singola, cookie di sessione HttpOnly, rate limit
  collectors/    snmp (profiles/*.yaml caricati all'avvio), proxmox, pbs, synology, agent, uptime
  scheduler.rs   esegue ogni target abilitato sul suo intervallo attraverso il registry dei collector
  tsdb/          writer VictoriaMetrics (flush in batch) + proxy di query; embedded.rs supervisiona il VictoriaMetrics figlio
  db/            SQLite + migrazioni numerate in db/migrations/
  alerting/      regole, macchina a stati (Phase/EffectivePhase), soppressione per genitore, silenzi, baseline stagionale
  notify/        22 canali di notifica, descritti all'interfaccia da notify/catalog.rs
  crypto.rs      AES-256-GCM per credenziali/token, chiave derivata da /data/secret.key o DUMBMONIT_SECRET
crates/agent     agent Linux/macOS/FreeBSD/Windows; install/ contiene install.sh/.ps1 servito dal server
web/             SvelteKit (Svelte 5 runes, Tailwind 4, uPlot), fallback SPA adapter-static, ssr=false
profiles/        profili di raccolta SNMP, applicati automaticamente per sysObjectID
docs/            questa documentazione (MkDocs, pubblicata su Read the Docs)
```

## Compilare senza un toolchain Rust

La compilazione avviene in Docker; nulla di Rust deve essere installato
sull'host.

```bash
# Stack completo (costruisce l'immagine: circa 10 min a freddo) — UI su http://localhost:8080
docker compose up -d --build
# overlay sviluppatore: il VictoriaMetrics integrato viene pubblicato su :8428 per query MetricsQL dirette
docker compose -f docker-compose.yml -f docker-compose.dev.yml up -d --build

# Rust, dentro lo stage builder (musl/alpine)
docker build -t dumbmonit-devenv --target builder .
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test -p dumbmonit-server --test alerts_api   # un file di test di integrazione
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo test -p dumbmonit-server test_name          # un test per nome
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo clippy --all-targets --all-features -- -D warnings
docker run --rm -v "$PWD:/build" -w /build dumbmonit-devenv cargo fmt --all --check
```

Con Rust installato localmente, `cargo test` e
`cargo clippy -- -D warnings` funzionano direttamente. Edition 2024, MSRV
1.88; `rustfmt.toml` imposta `max_width = 100` e
`use_small_heuristics = "Max"`.

La build Docker è a strati: build web → dipendenze Rust su fonti fittizie
→ fonti reali (con un `touch` prima del secondo `cargo build`, altrimenti
cargo riutilizza gli artefatti fittizi) → agent cross-compilato con
cargo-zigbuild per `x86_64-unknown-linux-musl`,
`aarch64-unknown-linux-musl` e `x86_64-pc-windows-gnu` →
`FROM scratch`. `web/build` deve esistere prima di `cargo build`,
altrimenti il server viene spedito senza un'interfaccia.

## Interfaccia web

Node 22 è sufficiente sull'host.

```bash
cd web && npm run dev        # Vite su :5173, inoltra /api a localhost:8080 (il server Docker)
cd web && npm run check      # svelte-check (eseguito dalla CI)
cd web && npm run build      # build statica in web/build, incorporata da rust_embed al cargo build
```

Convenzioni: solo Svelte 5 runes (`$props`, `$state`, `$derived`,
`$effect`, snippet, `onclick`). Ogni accesso di rete passa attraverso
`src/lib/api/`; i componenti non fanno mai `fetch`. I tipi in
`api/types.ts` rispecchiano esattamente le struct Rust e devono essere
mantenuti sincronizzati con `crates/server/src/api/*.rs`. Il design
system (token in `src/app.css`, primitive in `src/lib/ui/`) è
documentato in `DESIGN.md`; il contesto di prodotto in `PRODUCT.md`. Non
eseguire mai due `vite build` contemporaneamente: svuotano
`.svelte-kit/output`.

## Traduzioni (Weblate)

L'interfaccia web è cablata per la traduzione della community ma nulla è
ancora tradotto: l'inglese è l'unica locale fornita, in
`web/messages/en.json` (piatto, chiavi punteggiate, segnaposto
`{param}`, compilato da [Paraglide JS](https://paraglidejs.com) in
funzioni di messaggio tree-shaken sotto `web/src/lib/paraglide/`,
generato e non tracciato). La traduzione avviene su
[hosted.weblate.org/projects/dumbmonit](https://hosted.weblate.org/projects/dumbmonit/)
(componente `web-ui`), configurato come segue:

| Impostazione | Valore |
| --- | --- |
| Repository | questo repo, sottodirectory `web/` |
| File mask | `web/messages/*.json` |
| Monolingual base language file | `web/messages/en.json` |
| File format | JSON file (monolingual) |
| New language | crea un nuovo file di traduzione (Weblate scrive `web/messages/<locale>.json`) |
| Version control | GitHub, tramite l'app GitHub di Weblate: le traduzioni ritornano come pull request su `main` |

Le nuove stringhe dell'interfaccia atterrano solo in
`web/messages/en.json`; un traduttore poi compila gli altri file di
locale tramite Weblate. Le chiavi vengono estratte progressivamente —
oggi questo copre le pagine Settings (`web/src/routes/settings/`) e la
copia integrata di un paio di primitive `web/src/lib/ui/` — il resto
dell'interfaccia legge ancora inglese semplice e viene estratto man mano
che viene toccato. Vedi `CONTRIBUTING.md` per la convenzione di
redazione delle stringhe.

`web/project.inlang/settings.json` punta il suo unico plugin
(message-format) a `./node_modules/@inlang/plugin-message-format/dist/index.js`
invece dell'URL jsdelivr a cui lo strumento proprio di inlang punta per
impostazione predefinita: un percorso relativo, non `http`, viene letto
direttamente dal disco, così `npm run build` non raggiunge mai un CDN —
importante poiché la build dell'immagine non ha garanzia di una cache
npm calda che offra quell'URL, e uno strumento per homelab che ha
bisogno di internet per ricostruirsi è una brutta impressione.

## Test e CI

`.github/workflows/ci.yml` esegue, a ogni push e pull request:

1. `cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all-features`;
2. `cargo deny check licenses bans` — solo licenze approvate OSI sono
   consentite da `deny.toml`; una dipendenza copyleft o source-available
   fa fallire la build;
3. `npm run check` e `npm run build`;
4. l'immagine (`linux/amd64`), solo build.

`.github/workflows/release.yml` pubblica la stessa immagine su
`ghcr.io/laupernoe/dumbmonit`: `:edge` a ogni push su `main`, `:X.Y.Z`,
`:X.Y`, `:X` e `:latest` a ogni tag `v*`.

I test di integrazione vivono in `crates/server/tests/`; i moduli puri
(conversione delle metriche, parsing delle opzioni, macchina a stati
degli avvisi, silenzi) sono testati sul posto senza rete.

## Aggiungere un collector

1. Implementa `Collector` da `crates/proto/src/collector.rs`: `kind()`,
   `probe(&Target) -> Result<Vec<Sample>, ProbeError>` e opzionalmente
   `discover()` per il rilevamento del profilo. Restituisci le varianti
   `ProbeError` sinceramente: `means_down()` decide se un fallimento
   conta come "dispositivo inattivo" (avvisato) oppure come un errore di
   configurazione (mostrato sul dispositivo, non notificato).
2. Leggi le tue opzioni da `Target::tags` in un `options.rs`, con valori
   predefiniti.
3. Registralo in `crates/server/src/main.rs` con
   `registry.register(...)`. Lo scheduler e l'API non conoscono mai i
   tipi concreti.
4. Descrivilo in `crates/server/src/api/collectors.rs`: label, summary,
   esempi, tipi di credenziale, suggerimento di indirizzo, nota di
   configurazione e l'elenco `options` (key, label, help, placeholder,
   default, input, choices). L'interfaccia costruisce il suo modulo da
   questo; un test lì verifica che ogni tipo registrato abbia una nota e
   che le chiavi delle opzioni corrispondano a ciò che il collector
   legge.
5. Nomina le metriche `<cosa>_<unità>` senza il prefisso `dumbmonit_`
   (il writer lo aggiunge) e aggiungi qualsiasi nuova metrica che una
   regola integrata bersaglia all'elenco nei test di
   `alerting/rules.rs`.
6. Aggiungi una pagina sotto `docs/devices/`.

I campioni sono `Sample::new(name, value, MetricKind::Gauge | Counter,
timestamp_ms)` con `.with_label(k, v)`; le etichette di identità
(`target`, `host`, `tag_*`) vengono aggiunte dal registry.

## Aggiungere un canale di notifica

1. Implementa `Notifier` (`crates/server/src/notify/mod.rs`): `kind()`
   e `async fn send(&Message) -> Result<(), NotifyError>`. I canali web
   condividono il client HTTP e gli helper in `notify/http.rs`; guarda
   `services.rs`, `chat.rs`, `push.rs`, `oncall.rs` e `smtp.rs` come
   esempi affini.
2. Aggiungi il tipo a `CHANNEL_KINDS` e al match `build` in
   `notify/mod.rs`, che trasforma un `ChannelConfig` memorizzato
   (impostazioni + segreti decifrati) in un `Notifier`.
3. Descrivilo in `notify/catalog.rs`: label, summary, ancora `doc_url`,
   e i campi `settings` e `secrets` (key, label, required, input, help,
   placeholder, options, shape, default). L'interfaccia rende il modulo
   da questa descrizione, quindi non è necessaria alcuna release
   dell'interfaccia.
4. Documentalo in `docs/notifications.md` sotto *Supported channels*,
   con un'intestazione il cui slug corrisponde all'ancora `doc_url`:
   l'interfaccia collega direttamente a quell'ancora su Read the Docs.

## Screenshot headless

`web/tools/README.md` descrive come rendere ogni pagina in un Chromium
headless in esecuzione in Docker (`zenika/alpine-chrome:with-puppeteer`,
nessun browser locale necessario): accedi con `curl` per ottenere il
cookie `dumbmonit_session`, poi esegui `web/tools/screenshot.js` contro
il server di preview oppure direttamente contro `:8080`. Scrive
`web/shots/<page>-<theme>-<viewport>.png` per entrambi i temi e
entrambe le viewport, che è come viene verificata l'interfaccia dopo
una modifica e come sono stati fatti gli screenshot di questa
documentazione.

## Documentazione

Questo sito è costruito con MkDocs e Material for MkDocs da `docs/`
(`mkdocs.yml` alla radice, `.readthedocs.yaml` per Read the Docs). Per
visualizzare un'anteprima:

```bash
pip install -r docs/requirements.txt
mkdocs serve
mkdocs build --strict   # ciò che esegue Read the Docs; deve passare senza avvisi
```
