# Caso di assurance

Questa pagina argomenta che DumbMonit soddisfa i propri requisiti di sicurezza:
enuncia le minacce, i confini in cui cambia la fiducia, perché il design limita
i danni di un errore, come viene contrastata ciascuna classe comune di
debolezza e quali prove sostengono ogni affermazione. Estende il
[modello di sicurezza](security-model.md), che elenca le stesse misure in modo
più sintetico. Per segnalare una vulnerabilità, segui la
[policy di sicurezza](https://github.com/laupernoe/dumbmonit/blob/main/SECURITY.md).

I percorsi dei file qui sotto sono relativi alla radice del repository.

## Requisiti di sicurezza

1. Solo un utente autenticato, con un ruolo che lo consenta, può leggere o
   modificare la configurazione, gli alert o le misurazioni.
2. Le credenziali che DumbMonit conserva per i tuoi monitor non sono
   leggibili dal file del database, da un bundle di backup senza la sua
   passphrase, né dall'API.
3. Solo una macchina in possesso di un token valido può inviare misurazioni, e
   un token trapelato può essere revocato da solo.
4. DumbMonit non può essere trasformato in un modo per raggiungere servizi che
   non dovrebbe, in particolare il proprio processo figlio VictoriaMetrics non
   autenticato.
5. Una pagina servita a un browser non può eseguire script che non ha fornito
   essa stessa, e un sito di terze parti non può far modificare nulla a un
   browser con una sessione attiva.

## Modello di minaccia

| Attaccante | Capacità | In ambito |
|---|---|---|
| Client di rete anonimo | Raggiunge la porta HTTP | Sì |
| Utente autenticato con ruolo basso | Sessione valida o token API in sola lettura | Sì (escalation dei privilegi) |
| Pagina web malevola | Gira nel browser di un admin | Sì (CSRF, XSS, clickjacking) |
| Monitor sorvegliato compromesso o malevolo | Risponde alle sonde con contenuti ostili | Sì (parser, reindirizzamenti) |
| Agent malevolo o rubato | Possiede un token di agent | Sì (portata di un singolo token) |
| Admin che inserisce un URL ostile | Configura un webhook o l'indirizzo di un monitor | Sì, per gli indirizzi del server stesso |
| Chi ha l'host, il volume `/data` o il runtime dei container | Controllo totale | No: fuori ambito, come nella policy di sicurezza |
| Vulnerabilità all'interno di VictoriaMetrics o dei monitor sorvegliati | Codice upstream | No: da segnalare upstream |

## Confini di fiducia

| Confine | Che cosa lo attraversa | Come è protetto | Prove |
|---|---|---|---|
| Browser verso `/api/*` | Cookie di sessione o token API, JSON | Un solo middleware di autenticazione, tre ruoli, header CSRF sulle scritture autenticate da cookie | `crates/server/src/auth/middleware.rs`; test `auth_api.rs`, `operator_role_api.rs`, `api_open_security_api.rs`, `cors_policy_api.rs` |
| Browser verso le pagine servite | HTML, script, stile | Content Security Policy con nonce per risposta, `nosniff`, `Referrer-Policy: same-origin`, `X-Frame-Options: DENY` | `crates/server/src/api/mod.rs` (layer degli header di sicurezza) |
| Identity provider verso il server | Reindirizzamento OpenID Connect e ID token | State, nonce e PKCE (S256); il collegamento a un account locale è esplicito | `crates/server/src/auth/oidc/`; test `oidc_api.rs` |
| Agent verso `/api/ingest` | Misurazioni, risultati di comandi | Token per agent, salvato come hash SHA-256 (prefisso `dmon_`) | `crates/server/src/api/ingest.rs`, `crates/server/src/collectors/agent/token.rs`; test `agent_binding_api.rs` |
| Relay agent verso il server | Risultati delle sonde da un'altra rete | Stesso controllo del token di agent (`agent_commands::authenticate`) | `crates/server/src/api/relay.rs`; test `relay_api.rs` |
| Server verso i monitor sorvegliati | Richieste di sonda, risposte ostili | Reindirizzamenti seguiti solo sullo stesso host, numero di reindirizzamenti limitato, parser delle risposte con test unitari, verifica dei certificati attiva per impostazione predefinita | `crates/collectors/src/http.rs` (`same_host_redirects`) |
| Server verso i canali di notifica | Richieste webhook e SMTP | Stessa policy sui reindirizzamenti più una guardia che rifiuta gli indirizzi del server stesso, verificata prima dell'invio, a ogni reindirizzamento e alla risoluzione DNS | `crates/server/src/notify/guard.rs`; test `notify_guard_api.rs` |
| Server verso VictoriaMetrics | Scritture e query di lettura | Il processo figlio incorporato ascolta su `127.0.0.1:8428` per impostazione predefinita; il proxy compatibile con Prometheus inoltra solo le rotte di lettura (query, query_range, series, labels) | `crates/server/src/config.rs` (`DUMBMONIT_VM_LISTEN`), `crates/server/src/api/prometheus.rs` (`readable_segments`) |
| Server verso SQLite | Configurazione e stato | Parametri vincolati tramite `sqlx` | `crates/server/src/db/` |
| Server verso `/data` | Database, chiave segreta | Segreti cifrati con AES-256-GCM; l'utente del container possiede solo `/data` | `crates/server/src/crypto.rs`, `Dockerfile` |

## Argomentazione del secure design

L'argomentazione segue i principi di design di Saltzer e Schroeder.

- **Economia del meccanismo.** Un binario, un file SQLite, un processo figlio su
  loopback, un solo ingresso per ogni client (`/api/*`).
- **Valori predefiniti fail-safe.** Su un'istanza nuova ogni rotta sotto la
  guardia di autenticazione risponde `401`; rispondono solo health, stato
  dell'autenticazione, setup e login, così nessuno può preparare l'istanza prima
  del suo proprietario (`auth/middleware.rs`, test `api_open_security_api.rs`).
  VictoriaMetrics ascolta su loopback a meno che l'operatore non lo cambi. I
  certificati TLS dei servizi monitorati sono verificati a meno che l'operatore
  imposti `insecure_tls` (`crates/collectors/src/http.rs`).
- **Mediazione completa.** Il middleware di autenticazione gira su ogni rotta
  dell'API, invece di lasciare a ciascun handler il compito di controllare.
- **Design aperto.** Il codice, la descrizione dell'API
  (`crates/server/src/api/openapi.yaml`) e questa pagina sono pubblici. La
  sicurezza non si basa sulla segretezza del design.
- **Separazione dei privilegi.** Sessioni, token API e token degli agent sono
  credenziali diverse. I token API hanno uno scope `read` o `write`
  (`auth/token.rs`) e gli utenti hanno un ruolo di visualizzatore, operatore o
  admin.
- **Privilegio minimo.** L'immagine gira con UID 65532 (`Dockerfile`). Avviato come root (`user: "0:0"`,
  per i bind mount), il server si limita a cedere la directory dei dati a
  quell'utente, senza seguire i link, poi abbandona root per sempre e verifica
  di non poterlo riottenere, prima di aprire qualsiasi altra cosa
  (`privileges.rs`). Il file Compose tiene solo le quattro capability che questo
  passo richiede, imposta un file system root in sola lettura e `no-new-
  privileges` (`docker-compose.yml`).
- **Minimo meccanismo comune.** Gli agent si autenticano con il proprio token,
  quindi revocare una macchina non tocca alcun utente né altri agent.
- **Accettabilità psicologica.** Il comportamento sicuro è quello predefinito e
  non richiede passaggi aggiuntivi: il cookie è `HttpOnly` e `SameSite=Lax`,
  l'header CSRF è aggiunto dal client web, i segreti sono in sola scrittura
  nell'interfaccia.
- **Difesa in profondità.** Cifratura a riposo oltre al controllo degli accessi;
  l'header CSRF oltre a `SameSite`; il rate limit sul login oltre all'hashing
  Argon2; una policy sui reindirizzamenti oltre alla guardia sugli indirizzi
  propri.

### Crittografia

- Password: Argon2 (`auth/password.rs`). I segreti di sessioni e token API sono
  salvati come hash SHA-256 di valori casuali (`auth/session.rs`,
  `auth/token.rs`).
- Segreti a riposo: AES-256-GCM con un nonce nuovo a ogni cifratura
  (`crypto.rs`). La chiave è derivata con Argon2id da `DUMBMONIT_SECRET` o dal
  file di chiave generato, con un salt persistente; un segreto più corto di 16
  caratteri viene rifiutato e un valore canary rileva all'avvio una chiave
  errata.
- Backup: la chiave deriva dalla passphrase dell'operatore con Argon2id, con
  parametri salvati nel bundle, così l'algoritmo può cambiare senza rompere i
  vecchi bundle (`backup/bundle.rs`).
- Secondo fattore: TOTP, con codici di recupero monouso sottoposti a hash
  (`auth/totp.rs`).
- OpenID Connect: state, nonce e PKCE `S256` (`auth/oidc/pkce.rs`).
- Tutte le primitive provengono da librerie mantenute (`aes-gcm`, `argon2`,
  `sha2`); nessuna è fatta in casa. Il listener integrato parla HTTP in chiaro;
  la documentazione richiede un reverse proxy TLS, e i cookie ottengono
  `Secure` quando la richiesta arriva su HTTPS (`auth/cookie.rs`).

## Contromisure alle debolezze comuni

| Debolezza (OWASP Top 10, CWE) | Contromisura | Prove |
|---|---|---|
| Controllo degli accessi inadeguato, CWE-862, CWE-285 | Middleware di autenticazione, ruoli, scope dei token | `operator_role_api.rs`, `api_tokens_api.rs`, `api_open_security_api.rs` |
| Errori crittografici, CWE-327, CWE-312 | AES-256-GCM a riposo, Argon2, nessun segreto restituito dall'API | test unitari di `crypto.rs`; `has_secret` / `credential_kind` al posto del segreto |
| SQL injection, CWE-89 | Parametri vincolati tramite `sqlx` | `crates/server/src/db/` |
| Cross-site scripting, CWE-79 | Svelte esegue l'escape dell'output; CSP con `default-src 'none'`, script solo da `'self'` e con nonce; `base-uri 'none'` | `api/mod.rs` |
| Cross-site request forgery, CWE-352 | Le scritture via cookie richiedono `X-Requested-With: DumbMonit` o i metadati fetch same-origin; CORS non concede l'header a nessuno; `SameSite=Lax` | `auth/middleware.rs`, `auth/cors.rs`; test `cors_policy_api.rs` |
| Clickjacking, CWE-1021 | `X-Frame-Options: DENY` e `frame-ancestors 'none'`, tranne sulle pagine di stato pubbliche opt-in pensate per essere incorporate | `api/mod.rs` |
| Server-side request forgery, CWE-918 | Solo reindirizzamenti sullo stesso host; la guardia delle notifiche rifiuta gli indirizzi di ascolto del server e di VictoriaMetrics, anche dopo la risoluzione DNS (rebinding). Loopback e intervalli privati restano raggiungibili di proposito, perché i servizi di un homelab vivono lì | `notify/guard.rs`; test `notify_guard_api.rs` |
| Errori di autenticazione, CWE-307, CWE-287 | Rate limit sul login con ritardo crescente per indirizzo e per account, TOTP opzionale, OIDC con PKCE, reimpostazione del secondo fattore da parte di un admin | `auth/rate_limit.rs`, `auth/totp.rs`, `auth/oidc/` |
| Deserializzazione non sicura, esaurimento delle risorse, CWE-400 | JSON tipizzato tramite `serde`, corpi delle richieste limitati a 16 MiB, reindirizzamenti limitati | `api/mod.rs`, `collectors/src/http.rs` |
| Sicurezza della memoria, CWE-787, CWE-416 | Rust; `unsafe` compare solo nel codice dei contatori di prestazioni dell'agent Windows (`crates/agent/src/collect/perf_counters.rs`) | grep di `unsafe` |
| Componenti vulnerabili, CWE-1104 | `cargo deny` (licenze, divieti, advisory); Dependabot; scansione Trivy delle immagini pubblicate; action bloccate per commit; build con `--locked` | `.github/workflows/ci.yml`, `security.yml`, `.github/dependabot.yml`, `deny.toml` |
| Integrità di software e dati, CWE-494 | Gli installer degli agent verificano lo SHA-256 del binario scaricato | `crates/agent/install/install.sh`, `install.ps1` |
| Errori di logging e monitoraggio, CWE-778 | Log di sicurezza di accessi e modifiche sensibili | `auth/audit.rs`; test `security_audit_api.rs` |

I rischi residui noti sono elencati nel modello di sicurezza (listener HTTP in
chiaro, opzioni dei monitor non cifrate, VictoriaMetrics incorporato non
autenticato). La CSP consente stili inline (`style-src 'unsafe-inline'`), il che
la indebolisce contro l'iniezione di stili ma non contro l'esecuzione di script.
Esiste un solo advisory accettato, `RUSTSEC-2023-0071`, ignorato in `deny.toml`
con la sua motivazione: il crate `rsa` verifica le firme RS256 con la chiave
pubblica del provider e non usa mai una chiave privata.

## Prove

- **Integrazione continua** (`.github/workflows/ci.yml`): `cargo fmt --check`,
  `clippy -D warnings`, la suite completa di test Rust,
  `cargo deny check licenses bans` e `cargo deny check advisories`, controllo dei
  tipi web, test e build web, e una build dell'immagine Docker, a ogni push e
  pull request.
- **Scansioni pianificate** (`.github/workflows/security.yml`): Trivy sulle
  immagini pubblicate di server e agent, e l'OpenSSF Scorecard, ogni settimana e
  dopo ogni pubblicazione di `:edge`. I risultati compaiono nella scheda
  Security del repository, dove riporta anche il code scanning di GitHub
  (CodeQL).
- **Test**: test di API, autenticazione, ruoli, CORS, OIDC, guardia e audit in
  `crates/server/tests/`, test unitari accanto al codice e test web in
  `web/tests/`. La copertura di righe Rust misurata con `cargo-llvm-cov` è
  dell'87,9 %.
- **Politica di regressione**: una correzione arriva con un test che fallisce
  senza di essa (vedi la politica sui test in `CONTRIBUTING.md`).
- **Registro pubblico**: le correzioni di sicurezza sono pubblicate come
  advisory ed elencate nelle note di rilascio.
