# Modello di sicurezza

Questa pagina descrive che cosa DumbMonit protegge, di chi si fida, come il
design limita i danni di un errore e dove guardare nel codice e nei test. Per
segnalare una vulnerabilità, segui la
[policy di sicurezza](https://github.com/laupernoe/dumbmonit/blob/main/SECURITY.md).

## Che cosa è in gioco

DumbMonit gira all'interno della tua rete. Conserva le credenziali di cui ha
bisogno per leggere ogni monitor (community SNMP, token API, password),
riceve misurazioni dagli agent sulle tue macchine e invia richieste a indirizzi
che configuri tu. Una compromissione esporrebbe questi segreti e darebbe a un
attaccante un punto d'appoggio nella rete monitorata.

## Confini di fiducia

| Confine | Chi c'è dall'altra parte | Fiducia |
|---|---|---|
| Browser o script verso `/api/*` | Admin, operatori e visualizzatori | Autenticati da un cookie di sessione o da un token API; tre ruoli limitano ciò che ciascuno può fare. |
| Agent verso `/api/ingest` | Macchine su cui hai installato un agent | Un token per macchina; un token trapelato può essere revocato da solo. |
| Server verso monitor e servizi | Sistemi di terze parti | Nessuna fiducia: le risposte sono analizzate in modo difensivo e i reindirizzamenti restano sullo stesso host. |
| Server verso i canali di notifica | Servizi esterni | Le credenziali sono cifrate a riposo e mai restituite dall'API. |
| Server verso VictoriaMetrics | Un processo figlio su loopback, o un'istanza esterna | Quello incorporato non ha autenticazione e ascolta su loopback; quella esterna è responsabilità dell'operatore. |
| Volume `/data` e host | L'operatore | Fidato. Un attaccante che controlla l'host o il volume è fuori ambito. |

## Principi di secure design applicati

- **Privilegio minimo.** Il container gira come utente non privilegiato, senza
  alcuna capability, con file system root in sola lettura e
  `no-new-privileges`. Tre ruoli (visualizzatore, operatore, admin) limitano ciò
  che un utente può modificare (`crates/server/src/auth/users.rs`; test
  `operator_role_api.rs`).
- **Sicuro per impostazione predefinita.** Su un'istanza nuova rispondono solo
  le rotte health, setup, login e stato dell'autenticazione finché non esiste il
  primo admin (test `api_open_security_api.rs`).
- **Economia del meccanismo.** Un binario, un file SQLite, un processo figlio
  su loopback e un solo ingresso per ogni client: `/api/*`.
- **Valori predefiniti fail-safe.** Un controllo di sicurezza i cui dati
  mancano è `unknown`, mai superato; un monitor irraggiungibile e un errore
  di configurazione sono stati diversi.
- **Mediazione completa.** Un solo middleware di autenticazione
  (`crates/server/src/auth/middleware.rs`) protegge le rotte dell'API.
- **Separazione dei privilegi.** Sessioni, token API e token degli agent sono
  credenziali diverse, con portata diversa.
- **Difesa in profondità.** Cifratura a riposo oltre al controllo degli accessi,
  un header CSRF oltre ai flag del cookie, un rate limit oltre all'hashing delle
  password.
- **Nessun segreto di ritorno.** L'API restituisce `has_secret` e
  `credential_kind`, mai il segreto; omettere un segreto in un `PUT` lo
  mantiene.
- **Solo crittografia collaudata.** Le password sono sottoposte a hash con
  Argon2, i segreti sono cifrati con AES-256-GCM (`crypto.rs`), l'autenticazione
  a due fattori usa TOTP con codici di recupero monouso sottoposti a hash,
  OpenID Connect usa state, nonce e PKCE. Niente è fatto in casa.

## Contromisure per classe di debolezza

| Classe (tema OWASP Top 10) | Misure |
|---|---|
| Controllo degli accessi inadeguato | Un solo middleware di autenticazione; ruoli; un'istanza nuova rifiuta tutto tranne setup, login, health e stato dell'autenticazione. |
| Errori crittografici | AES-256-GCM a riposo, chiave tenuta fuori dal database; hash delle password con Argon2; il server integrato parla HTTP in chiaro, perciò è documentato un reverse proxy TLS. |
| Injection | L'SQL passa da `sqlx` con parametri vincolati; il proxy VictoriaMetrics inoltra solo query di lettura (`tsdb/mod.rs`, `proxy_read`); i parser hanno test unitari. |
| Design non sicuro | Il modello di minaccia qui sopra; i segreti non fanno mai il percorso di ritorno; un'istanza con un solo scopo. |
| Errata configurazione di sicurezza | File Compose rinforzati, VictoriaMetrics solo su loopback, note di hardening per l'operatore nella policy di sicurezza. |
| Componenti vulnerabili | `cargo deny` controlla licenze, divieti e advisory RustSec; scansione Trivy settimanale delle immagini pubblicate; Dependabot; GitHub Actions bloccate per commit. |
| Errori di identificazione e autenticazione | Rate limit sul login per indirizzo client e per account (`auth/rate_limit.rs`), autenticazione a due fattori opzionale, OpenID Connect, un admin può reimpostare il secondo fattore di un utente e disconnetterlo. |
| Integrità di software e dati | Gli installer degli agent verificano lo SHA-256 del binario scaricato; le immagini di rilascio sono compilate dalla CI. |
| Logging e monitoraggio | Un log di sicurezza di accessi e modifiche sensibili (`auth/audit.rs`). |
| Server-side request forgery | Integrazioni e canali di notifica seguono i reindirizzamenti solo verso lo stesso host, così un monitor non può rimbalzare altrove una richiesta e la sua chiave API. |
| Cross-site request forgery | Le richieste che modificano lo stato e si basano sul cookie devono portare `X-Requested-With: DumbMonit` o i metadati fetch same-origin, altrimenti `403`; le richieste autenticate con token non portano cookie. |

## Verifica

Ogni push e pull request esegue `clippy -D warnings`, la suite di test Rust
(compresi i test di API, autenticazione e ruoli in `crates/server/tests/`), i
test web in `web/tests/`, i controlli sulle dipendenze e l'analisi CodeQL.
L'OpenSSF Scorecard viene eseguito periodicamente. Le correzioni di sicurezza
arrivano con un test di regressione e sono elencate, con il loro identificativo
CVE o GHSA, nelle note di rilascio.

Il [caso di assurance](assurance-case.md) fornisce il modello di minaccia
completo, l'argomentazione del secure design e le prove a sostegno di ciascuna
affermazione.

## Limiti noti

- Il server integrato parla HTTP in chiaro; metti TLS davanti.
- Le opzioni dei monitor sono salvate non cifrate. Metti i segreti nei campi
  delle credenziali.
- VictoriaMetrics incorporato non ha autenticazione.
- DumbMonit è in versione alpha; rotte e campi possono ancora cambiare.
