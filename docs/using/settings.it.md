# Impostazioni

![Impostazioni: account, utenti, single sign-on, agent, assistente, aspetto, informazioni](../assets/screenshots/settings-light.png){ loading=lazy }

Le impostazioni raccolgono ciò che è amministrativo: il tuo account, chi può
accedere e come, i token che fanno entrare agent e assistenti, il backup
dell'istanza, il tema e lo stato di salute del server. Una pagina, una sezione
per argomento, con una colonna di ancore su schermo largo e una fila di chip su
telefono. Ogni sezione ha un link diretto (`/settings#agents`).

Due cose stavano qui e sono state spostate, perché non sono amministrazione:

| Cosa cerchi | Ora in | Vecchio link |
|---|---|---|
| Canali di notifica, ore di silenzio, politica di notifica | **Avvisi → Notifiche** ([pagina Avvisi](alerts.md#notifications)) | `/settings#notifications`, `/settings#notifications-policy` |
| Pagine di stato e annunci | **Stato** ([Pagine di stato](status-pages.md)) | `/settings#status` |

I vecchi link funzionano ancora: reindirizzano al nuovo posto.

## Account e sicurezza

Questo è il tuo account. Il primo viene creato al primo avvio, e se ne possono
aggiungere altri più sotto. Qui cambi la password: password attuale, nuova
password (almeno 12 caratteri; una frase intera è più sicura di una parola
complicata), conferma. Cambiarla disconnette ogni altra sessione. Gli account
che accedono tramite il provider di identità non hanno password qui.
**Esci** termina questa sessione.

L'accesso è limitato in frequenza per indirizzo client e per account: dopo cinque
tentativi falliti, ogni tentativo successivo attende più a lungo (30 s, con
raddoppio, fino a 5 minuti). Dietro un reverse proxy, imposta
`DUMBMONIT_TRUSTED_PROXIES` in modo che il limitatore veda il vero indirizzo del
client e non quello del proxy (vedi
[Configurazione](../reference/configuration.md)). Le sessioni durano 30 giorni.
Una password dimenticata si reimposta con `DUMBMONIT_RESET_PASSWORD=1`: vedi la
[FAQ](../faq.md#i-lost-the-password).

### Autenticazione a due fattori

Gli account che accedono con una password possono aggiungere un secondo fattore:
un codice a sei cifre basato sul tempo (TOTP) da un'app di autenticazione —
Aegis, FreeOTP, Google Authenticator, 1Password, Bitwarden… **Configura** chiede
la tua password, mostra un codice QR da scansionare (o la chiave da digitare), e
abilita il secondo fattore quando inserisci un primo codice dall'app. Otto
**codici di recupero** vengono poi mostrati una sola volta: salvali, ciascuno ti
fa accedere una sola volta se il telefono è perso. La schermata di accesso
chiede il codice (o un codice di recupero) dopo la password; cinque codici
errati annullano il tentativo e si ricomincia dalla password, e il consueto
limite di frequenza degli accessi continua a contare tra i tentativi. Un codice
è accettato una sola volta: quello che hai appena usato (compreso quello che ha
abilitato il secondo fattore) non ti fa accedere di nuovo — attendi il
successivo.

**Disabilita** chiede di nuovo la password. I codici di recupero non possono
essere rigenerati da soli: disabilita e configura di nuovo per ottenere una
serie nuova. Un amministratore può reimpostare il secondo fattore di un altro
utente da **Utenti** (*Reimposta 2FA*) quando sono spariti sia il telefono sia i
codici — questo disconnette l'utente ovunque e lascia la password invariata. Il
segreto è memorizzato cifrato con il segreto dell'istanza; i codici sono
sottoposti a hash. Gli account con single sign-on ottengono il secondo fattore
dal provider di identità, non da qui.

Gli amministratori trovano in questa sezione anche il **registro di sicurezza**:
accessi e fallimenti, modifiche di password e dei due fattori, modifiche di
token e account, con l'indirizzo del client. Conserva le ultime 5 000 voci.

## Notifiche push

Notifiche native su questo telefono o computer, anche con DumbMonit chiuso:
**Abilita su questo dispositivo**, poi **Invia un test**. Ogni account gestisce
qui i propri dispositivi (anche gli osservatori); quali avvisi li raggiungono è
stabilito da un canale **Web Push** in Avvisi → Notifiche. Il pulsante **i**
riassume i requisiti: HTTPS (o localhost), installazione nella schermata Home su
iPhone e iPad, e i domini dei servizi push che il server deve raggiungere. Guida
completa: [Notifiche push (PWA)](push-notifications.md).

## Utenti

Solo amministratori. Gli account che possono accedere, e il loro ruolo:

| Ruolo | Legge tutto | Gestisce gli avvisi (presa in carico, posticipo, ignora, finestre di manutenzione, cancellazione dello storico) | Modifica la configurazione (dispositivi, regole, canali, pagine di stato, impostazioni, backup, token, account) |
|---|---|---|---|
| **Osservatore** | sì | no | no |
| **Operatore** | sì | sì | no |
| **Amministratore** | sì | sì | sì |

Un osservatore vede ogni pagina con tutti i controlli che modificherebbero
qualcosa nascosti. Alcune cose restano riservate agli amministratori anche in
lettura: i token API e degli agent, le email degli iscritti alle pagine di
stato, e le opzioni dei dispositivi che sembrano segreti (un'opzione il cui nome
contiene `key`, `token`, `password`…, le intestazioni aggiuntive di un controllo
HTTP, la password di un indirizzo `https://user:pass@host`), che i non
amministratori leggono come `***`. Un operatore è il ruolo di reperibilità: i
controlli degli avvisi ci sono, le schermate di configurazione riportano
*Operatore — la configurazione è riservata agli amministratori*. Solo un
amministratore crea account o cambia un ruolo — un operatore non può promuovere
nessuno, sé stesso compreso. Le sezioni *Utenti*, *Single sign-on* e il registro
di sicurezza restano riservate agli amministratori. **Aggiungi utente** chiede
un nome utente, un nome visualizzato facoltativo, un ruolo e una password; il
pulsante accanto al campo password ne genera una casuale. La password viene
mostrata una sola volta, da consegnare.

Ogni riga riporta il ruolo, se l'account accede con una password o tramite il
provider di identità, se i due fattori sono attivi e l'ultimo accesso.
**Modifica** cambia il nome visualizzato, il ruolo e la password (una
reimpostazione disconnette quell'utente ovunque e mostra la nuova password una
sola volta). L'interruttore **Abilitato** sospende un account senza eliminarlo;
**Reimposta 2FA** compare quando l'account ha un secondo fattore; **Elimina** lo
rimuove definitivamente. Il server mantiene almeno un amministratore attivo, e i
controlli riflettono questa regola invece di segnalarla come errore: l'ultimo
amministratore non può essere retrocesso, disabilitato o eliminato, e non puoi
disabilitare o eliminare il tuo stesso account.

## Single sign-on

Solo amministratori. Accesso tramite un provider OpenID Connect (Authelia,
Authentik, Keycloak, Pocket ID, Google Workspace…). Compila il *nome del
provider* (etichetta il pulsante "Continua con …" nella schermata di accesso),
l'*URL dell'issuer* — il discovery viene letto da
`{issuer}/.well-known/openid-configuration` — il *client id*, il *client secret*
(memorizzato cifrato, non più mostrato) e l'*URL pubblico* con cui i browser
raggiungono DumbMonit. Il modulo costruisce l'URI di reindirizzamento a partire
da quell'URL pubblico: copialo e registralo presso il tuo provider.

**Prova il discovery** legge la configurazione del provider senza far accedere
nessuno, elenca gli endpoint trovati e avvisa quando il provider non firma né
con RS256 né con ES256 — le sole due firme che DumbMonit accetta. Le
impostazioni salvate qui hanno la precedenza sulle variabili d'ambiente
`DUMBMONIT_OIDC_*`, che si applicano solo finché non è salvato nulla;
**Dimentica le impostazioni salvate** le cancella e ripiega sulle variabili, se
esistono.

**Avanzate.** Il resto del modulo si trova sotto *Avanzate*: *Scope* indica che
cosa chiedere al provider, *Claim dei gruppi* il claim del token che elenca i
gruppi dell'utente, e *Crea account al primo accesso* decide se una persona
sconosciuta ottiene un account. È **disattivato per impostazione predefinita**:
solo gli account già esistenti possono accedere finché non lo attivi.

**Ruoli.** *Gruppi amministratori* elenca i gruppi i cui membri diventano
amministratori; *Gruppi operatori* i gruppi i cui altri membri diventano
operatori; tutti gli altri sono osservatori. Lascia vuoti entrambi per continuare
a gestire i ruoli in **Utenti**. Con solo *Gruppi operatori* compilato, i gruppi
non toccano mai il ruolo di amministratore: gli amministratori restano
amministratori, gli altri diventano operatori o osservatori. I ruoli vengono
rivalutati a ogni accesso, con l'eccezione che l'ultimo amministratore attivo non
viene mai retrocesso. Le variabili d'ambiente corrispondenti sono
`DUMBMONIT_OIDC_ADMIN_GROUPS` e `DUMBMONIT_OIDC_OPERATOR_GROUPS`.

**A quale account approda un accesso.** L'identità viene ricordata tramite il suo
subject presso il provider dopo il primo accesso. Al primo accesso:

- Un account esistente viene collegato solo quando il provider dichiara
  `email_verified: true` e quell'email è il nome utente dell'account. Un
  `preferred_username` o un'email non verificata non collega mai: sulla maggior
  parte dei provider li sceglie l'utente, quindi `admin` del provider non deve
  diventare *il tuo* `admin`.
- Un **amministratore locale con una password non viene mai collegato
  automaticamente**, nemmeno con un'email verificata: l'accesso crea invece un
  account distinto. Continua a gestire quell'amministratore con la sua password,
  oppure crea un amministratore SSO separato tramite i gruppi amministratori.
- Altrimenti, con *Crea account al primo accesso* attivo, viene creato un nuovo
  account (nome utente dal provider, con suffisso `-1`, `-2`… se già in uso). Se
  è disattivato, può accedere solo il collegamento tramite email verificata
  descritto sopra.

**La creazione di account è disattivata a meno che tu non la scelga.** Le nuove
installazioni partono con *Crea account al primo accesso* disattivato, e così
ogni configurazione in cui non è mai stato impostato — impostazioni salvate prima
che l'opzione esistesse, o configurazioni solo tramite variabili d'ambiente senza
`DUMBMONIT_OIDC_AUTO_CREATE`. Le versioni precedenti trattavano "mai impostato"
come attivo; il pannello Single sign-on ora indica quando si applica questo
valore predefinito. Un valore salvato esplicitamente viene mantenuto così com'è.

## Agent

Token di registrazione per l'[agent Linux, macOS, FreeBSD e Windows](../devices/agent.md).
Creane uno con un nome ("File server", "Flotta di casa"): il token viene mostrato
una sola volta, con i comandi di installazione per Linux e Windows pronti da
copiare. Ogni comando contiene i checksum SHA-256 dei binari dell'agent
distribuiti da questo server, e l'installer rifiuta un download che non
corrisponde; i checksum sono elencati anche sotto i comandi, per un confronto
manuale. L'elenco mostra il prefisso di ogni token, la data di creazione, l'ultimo
utilizzo e se è stato revocato. **Revoca** ferma ogni agent che usa quel token al
suo prossimo invio.

Un token può registrare più macchine. Revocarlo non elimina i dispositivi.
L'elenco dei token, la creazione e la revoca sono per gli amministratori;
osservatori e operatori vedono la sezione senza l'elenco.

## API e assistenti

Un solo tipo di token per entrambi: client MCP e [API REST](../reference/api.md).
Dai un nome a un token, scegli il suo ambito e crealo — viene mostrato una sola
volta. Un token **lettura** può solo guardare; un token **lettura e scrittura**
può anche modificare — silenziare un dispositivo, eseguire un probe, aggiungere o
modificare dispositivi, regole e canali — ma mai account, impostazioni di accesso
o altri token.

Sotto **Collega**, tre schede contengono gli snippet, compilati con il token
finché è ancora a schermo: *Claude* (il comando `claude mcp add` e il blocco
`claude_desktop_config.json`), *ChatGPT* (l'URL del server MCP e l'header di
autorizzazione, con un avviso quando questa pagina non è servita in HTTPS —
ChatGPT si connette dai server di OpenAI, quindi l'indirizzo deve essere
raggiungibile da internet) e *Cursor / altri*, la stessa forma per qualsiasi
client che parla Streamable HTTP con un header bearer. **Accesso API** mostra lo
stesso token sull'API REST come esempio `curl`, e indica le rotte che un
Prometheus o un Grafana interroga con esso. Vedi
[Collegare un assistente](assistant.md) e
[Raccogliere le metriche di DumbMonit](../reference/metrics.md#scraping-dumbmonit).

## Backup

Tre cose, nell'ordine in cui le incontri.

Un **avviso che non puoi non vedere**: `/data/secret.key` è ciò che decifra le
credenziali dei dispositivi, e una copia del database senza di esso ripristina
un'istanza che non può comunicare con nulla. La formulazione cambia quando il
segreto proviene invece da `DUMBMONIT_SECRET` — in quel caso non c'è alcun file
da copiare, ed è il tuo gestore di password a doverlo conservare.

**Esporta la configurazione** elenca ciò che conterrebbe il bundle, con un
conteggio per sezione, poi chiede due volte una passphrase. Il file che scarichi
contiene tutte le credenziali di questa istanza, cifrate con quella sola
passphrase: conservalo dove conservi le password. Le password degli account e i
segreti 2FA sono esclusi a meno che tu non attivi l'interruttore.

**Ripristina un bundle** prende il file e la sua passphrase e mostra prima una
**simulazione**: una tabella di ciò che verrebbe creato, aggiornato e lasciato
com'è, sezione per sezione, con una nota per tutto ciò che non sa collocare.
Nulla viene scritto finché non premi *Ripristina davvero*.

**Backup locali pianificati** mostra se sono attivi, dove vanno, l'ultima
esecuzione e se è riuscita, e i file conservati con la loro dimensione.
**Esegui backup ora** ne scrive uno immediatamente — da fare prima di un
aggiornamento.

Tutto qui è riservato agli amministratori. Vedi
[Backup e ripristino](../install/backup.md).

## DumbMonit stesso {#self-monitoring}

Lo stato del motore in questo momento — pianificatore, database SQLite,
database delle metriche — con lo spazio libero del volume dei dati, la
dimensione del database e, per il VictoriaMetrics integrato, quante volte è
stato riavviato. Regole integrate ti avvisano già quando qualcosa va storto.

Sotto, l'**heartbeat in uscita**: un tuo URL (Healthchecks.io, un monitor push
di Uptime Kuma, Better Stack…) che DumbMonit chiama a intervalli regolari, solo
finché sta bene, perché qualcuno all'esterno si accorga quando questo host
tace. **Prova** invia subito una chiamata; **Segnala i guasti** chiama l'URL di
errore invece di tacere. L'URL viene salvato cifrato e non viene più mostrato,
solo il suo host. Solo un amministratore può modificarlo. Vedi
[Watching DumbMonit itself](../install/self-monitoring.md).

## Report

Email periodiche di disponibilità per il team: scegli settimanale, giornaliero o
mensile, il giorno, l'ora e il fuso orario, i destinatari e il canale email, poi
**Invia un'anteprima** per vedere il risultato. Solo amministratori. Vedi
[Report via email](reports.md).

## Aspetto

Tre riquadri, ciascuno con l'anteprima del proprio tema: **Sistema** (segue il
tuo dispositivo e cambia con esso), **Giorno** (il tema "carta millimetrata") e
**Notte** (il tema "radar"). La scelta è memorizzata nel browser. L'interruttore
nell'intestazione e *Cambia tema* della palette dei comandi alternano tra giorno
e notte.

**Apri la modalità parete** si trova in fondo alla sezione: il bollettino da
solo, a schermo intero, per un monitor nella stanza. ++esc++ la chiude, e la
palette dei comandi la apre da qualsiasi punto — vedi
[Modalità parete](overview.md#wall-mode).

## Informazioni

Versione del server e stato di salute dei suoi due archivi, il database
(configurazione dei dispositivi e storico degli avvisi) e VictoriaMetrics (ogni
misura nei grafici), come riportato da `GET /api/health` e aggiornato ogni 15
secondi. Un archivio che smette di rispondere mostra un avviso con l'errore e
cosa controllare. La sezione rimanda anche a questa documentazione e indica la
licenza, Apache 2.0.
