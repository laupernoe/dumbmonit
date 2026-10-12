# Settings

![Settings: account, utenti, single sign-on, agent, assistente, aspetto, informazioni](../assets/screenshots/settings-light.png){ loading=lazy }

Settings contiene ciò che è amministrativo: il tuo account, chi può
accedere e come, i token che fanno entrare agent e assistenti, il
backup dell'istanza, il tema, e lo stato di salute del server. Una
pagina, una sezione per argomento, con una rotaia di ancore su uno
schermo largo e una striscia di chip su un telefono. Ogni sezione è
collegabile (`/settings#agents`).

Due cose vivevano qui e si sono spostate, perché non sono
amministrazione:

| Stai cercando | Ora sotto | Vecchio link |
|---|---|---|
| Canali di notifica, ore silenziose, politica di notifica | **Alerts → Notifications** ([Pagina Alerts](alerts.md#notifications)) | `/settings#notifications`, `/settings#notifications-policy` |
| Pagine di stato e annunci | **Status** ([Pagine di stato](status-pages.md)) | `/settings#status` |

I vecchi link funzionano ancora: rimandano al nuovo posto.

## Account & security

Questo è il tuo account. Il primo viene creato al primo avvio, e altri
possono essere aggiunti sotto. Qui cambi la tua password: password
attuale, nuova password (almeno 12 caratteri; una frase intera è più
sicura di una parola complicata), conferma. Cambiarla disconnette ogni
altra sessione. Gli account che accedono tramite il provider di identità
non hanno una password qui. **Sign out** termina questa sessione.

Il login è limitato per indirizzo client e per account: dopo cinque
tentativi falliti, ogni tentativo successivo attende più a lungo (30 s,
raddoppiando, fino a 5 minuti). Dietro un reverse proxy, imposta
`DUMBMONIT_TRUSTED_PROXIES` così il limiter vede l'indirizzo reale del
client invece di quello del proxy (vedi
[Configurazione](../reference/configuration.md)). Le sessioni durano 30
giorni. Una password scordata viene reimpostata con
`DUMBMONIT_RESET_PASSWORD=1`: vedi la
[FAQ](../faq.md#i-lost-the-password).

### Autenticazione a due fattori

Gli account che accedono con una password possono aggiungere un secondo
fattore: un codice a sei cifre basato sul tempo (TOTP) da un'app
authenticator — Aegis, FreeOTP, Google Authenticator, 1Password,
Bitwarden… **Set up** chiede la tua password, mostra un codice QR da
scansionare (oppure la chiave da digitare), e abilita il secondo fattore
una volta che inserisci un primo codice dall'app. Otto **codici di
recupero** vengono allora mostrati una volta: salvali, ognuno ti fa
accedere una sola volta se il telefono viene perso. La schermata di
accesso chiede il codice (oppure un codice di recupero) dopo la
password; cinque codici errati annullano il tentativo e ricominci dalla
password, e il solito rate limit di accesso continua a contare tra i
tentativi. Un codice viene accettato una volta: quello che hai appena
usato (incluso quello che ha abilitato il secondo fattore) non ti fa
accedere di nuovo — attendi il successivo.

**Disable** chiede di nuovo la password. I codici di recupero non
possono essere rigenerati da soli: disattiva e configura di nuovo per
ottenere un nuovo set. Un admin può reimpostare il secondo fattore di un
altro utente da **Users** (*Reset 2FA*) quando sia il telefono che i
codici sono persi — questo disconnette l'utente ovunque e lascia la
password invariata. Il segreto viene memorizzato cifrato con il segreto
dell'istanza; i codici sono hashati. Gli account single sign-on ottengono
il loro secondo fattore dal provider di identità, non qui.

Gli admin trovano anche il **security log** in questa sezione: accessi e
fallimenti, cambi di password e secondo fattore, cambi di token e
account, con l'indirizzo del client. Mantiene le ultime 5.000 voci.

## Push notifications

Notifiche native su questo telefono o computer, anche con DumbMonit
chiuso: **Enable on this device**, poi **Send a test**. Ogni account
gestisce i propri dispositivi qui (anche i visualizzatori); quali avvisi
li raggiungono viene impostato da un canale **Web Push** in Alerts →
Notifications. Il pulsante **i** riassume i requisiti: HTTPS (oppure
localhost), installazione su Home Screen su iPhone e iPad, e i domini
del servizio push che il server deve raggiungere. Guida completa:
[Notifiche push (PWA)](push-notifications.md).

## Users

Solo admin. Gli account che possono accedere, e il loro ruolo:

| Ruolo | Legge tutto | Gestisce gli avvisi (ack, snooze, ignore, finestre di manutenzione, pulizia cronologia) | Modifica la configurazione (dispositivi, regole, canali, pagine di stato, impostazioni, backup, token, account) |
|---|---|---|---|
| **Viewer** | sì | no | no |
| **Operator** | sì | sì | no |
| **Admin** | sì | sì | sì |

Un visualizzatore vede ogni pagina con ogni controllo che cambierebbe
qualcosa nascosto. Un operatore è il ruolo di guardia: i controlli degli
avvisi ci sono, le schermate di configurazione leggono *Operator —
config is admin only*. Solo un admin crea account o cambia un ruolo — un
operatore non può promuovere nessuno, nemmeno se stesso. Le sezioni
*Users*, *Single sign-on* e il security log restano riservate agli
admin. **Add user** chiede un nome utente, un nome visualizzato
opzionale, un ruolo e una password; il pulsante accanto al campo
password ne crea una casuale. La password viene mostrata una volta, per
essere consegnata.

Ogni riga porta il ruolo, se l'account accede con una password o
tramite il provider di identità, se il secondo fattore è attivo, e
l'ultimo accesso. **Edit** cambia il nome visualizzato, il ruolo e la
password (un reset disconnette quell'utente ovunque e mostra la nuova
password una volta). L'interruttore **Enabled** sospende un account
senza eliminarlo; **Reset 2FA** appare quando l'account ha un secondo
fattore; **Delete** lo rimuove per sempre. Il server mantiene almeno un
admin attivo, e i controlli rispecchiano quella regola invece di
segnalarla come un errore: l'ultimo admin non può essere retrocesso,
disattivato o eliminato, e non puoi disattivare o eliminare il tuo
stesso account.

## Single sign-on

Solo admin. Accesso tramite un provider OpenID Connect (Authelia,
Authentik, Keycloak, Pocket ID, Google Workspace…). Compila il *nome del
provider* (etichetta il pulsante "Continue with …" sulla schermata di
accesso), l'*issuer URL* — la discovery viene letta da
`{issuer}/.well-known/openid-configuration` — il *client id*, il
*client secret* (memorizzato cifrato, mai mostrato di nuovo) e l'*URL
pubblico* con cui i browser raggiungono DumbMonit. Il modulo costruisce
il redirect URI da quell'URL pubblico: copialo e registralo con il tuo
provider.

**Test discovery** legge la configurazione del provider senza far
accedere nessuno, elenca gli endpoint che ha trovato, e avvisa quando il
provider firma né con RS256 né con ES256 — le sole due firme che
DumbMonit accetta. Le impostazioni salvate qui prevalgono sulle
variabili d'ambiente `DUMBMONIT_OIDC_*`, che si applicano solo mentre
nulla è salvato; **Forget saved settings** le azzera e torna alle
variabili quando esistono.

**Advanced.** Il resto del modulo sta sotto *Advanced*: *Scopes* nomina
cosa chiedere al provider, *Groups claim* la claim del token che elenca
i gruppi dell'utente, e *Create accounts on first sign-in* decide se una
persona sconosciuta riceve un account. È **disattivata per impostazione
predefinita**: solo gli account già esistenti possono accedere finché
non la attivi.

**Ruoli.** *Admin groups* elenca i gruppi i cui membri diventano admin;
*Operator groups* i gruppi i cui altri membri diventano operatori;
chiunque altro è un visualizzatore. Lascia entrambi vuoti per continuare
a gestire i ruoli sotto **Users**. Con solo *Operator groups*
compilato, i gruppi non toccano mai il ruolo admin: gli admin restano
admin, gli altri diventano operatori o visualizzatori. I ruoli vengono
ri-valutati a ogni accesso, eccetto che l'ultimo admin attivo non viene
mai retrocesso. Le variabili d'ambiente corrispondenti sono
`DUMBMONIT_OIDC_ADMIN_GROUPS` e `DUMBMONIT_OIDC_OPERATOR_GROUPS`.

**Su quale account finisce un accesso.** L'identità viene ricordata dal
suo soggetto provider dopo il primo accesso. Al primo accesso:

- Un account esistente viene collegato solo quando il provider afferma
  `email_verified: true` e quell'email è il nome utente dell'account. Un
  `preferred_username` o un'email non verificata non collegano mai: sulla
  maggior parte dei provider l'utente scelge quelli, quindi `admin` dal
  provider non deve diventare *il tuo* `admin`.
- Un **admin locale che ha una password non viene mai collegato
  automaticamente**, anche con un'email verificata: l'accesso crea invece
  un account distinto. Continua a gestire quell'admin con la sua
  password, oppure crea un admin SSO separato tramite i gruppi admin.
- Altrimenti, con *Create accounts on first sign-in* attivo, viene
  creato un nuovo account (nome utente dal provider, con suffisso `-1`,
  `-2`… se già preso). Con esso disattivato, solo il collegamento per
  email verificata sopra può accedere.

**La creazione dell'account è disattivata a meno che tu non la
scelga.** Le nuove installazioni iniziano con *Create accounts on first
sign-in* disattivato, e lo stesso vale per qualsiasi configurazione dove
non è mai stata impostata — impostazioni salvate prima che l'opzione
esistesse, oppure configurazioni solo-ambiente senza
`DUMBMONIT_OIDC_AUTO_CREATE`. Le release precedenti trattavano "mai
impostata" come attiva; il pannello Single sign-on ora dice quando si
applica questo predefinito. Un valore che hai salvato esplicitamente
viene mantenuto così com'è.

## Agents

Token di iscrizione per l'[agent Linux, macOS, FreeBSD e Windows](../devices/agent.md).
Crea uno con un nome ("File server", "Home fleet"): il token viene
mostrato una volta, con i comandi di installazione Linux e Windows
pronti da copiare. Ogni comando porta i checksum SHA-256 dei binari
agent che questo server fornisce, e l'installer rifiuta un download che
non corrisponde; i checksum sono anche elencati sotto i comandi, per
confrontarli a mano. L'elenco mostra il prefisso di ogni token, la data
di creazione, l'ultimo uso e se è stato revocato. **Revoke** ferma ogni
agent che usa quel token al suo prossimo push.

Un token può iscrivere diverse macchine. Revocarlo non elimina i
dispositivi. I visualizzatori vedono l'elenco e la sezione legge
*Viewer — read only*; creare e revocare sono per gli admin.

## API & assistants

Un tipo di token per entrambi: client MCP e l'[API REST](../reference/api.md).
Dai un nome a un token, scegli il suo ambito e crealo — viene mostrato
una volta. Un token **read** può solo guardare; un token **read and
write** può anche modificare le cose — silenziare un dispositivo,
eseguire una sonda, aggiungere o modificare dispositivi, regole e canali
— ma mai account, impostazioni di accesso o altri token.

Sotto **Connect**, tre tab contengono gli snippet, compilati con il
token mentre è ancora sullo schermo: *Claude* (il comando
`claude mcp add` e il blocco `claude_desktop_config.json`), *ChatGPT*
(l'URL del server MCP e l'header di autorizzazione, con un avviso
quando questa pagina non è servita su HTTPS — ChatGPT si connette dai
server di OpenAI, quindi l'indirizzo deve essere raggiungibile da
internet) e *Cursor / other*, la stessa forma per qualsiasi client che
parli Streamable HTTP con un header bearer. **API access** mostra lo
stesso token sull'API REST come un esempio `curl`, e indica le rotte che
un Prometheus o un Grafana sondano con esso. Vedi
[Collegare un assistente](assistant.md) e
[Sondare DumbMonit](../reference/metrics.md#scraping-dumbmonit).

## Backup

Tre cose, nell'ordine in cui le incontri.

Un **avviso che non puoi perdere**: `/data/secret.key` è ciò che decifra
le credenziali dei dispositivi, e una copia del database senza di esso
ripristina un'istanza che non può parlare con nulla. Il testo cambia
quando il segreto proviene invece da `DUMBMONIT_SECRET` — allora non
c'è alcun file da copiare, ed è il tuo gestore password che deve
conservarlo.

**Export the configuration** elenca cosa conterrebbe il bundle, con un
conteggio per sezione, poi chiede una passphrase due volte. Il file che
scarica contiene ogni credenziale di questa istanza, cifrata solo con
quella passphrase: conservalo dove conservi le password. Le password
degli account e i segreti 2FA vengono esclusi a meno che non attivi
l'interruttore.

**Restore a bundle** prende il file e la sua passphrase e mostra prima
un **dry run**: una tabella di cosa verrebbe creato, aggiornato e
lasciato invariato, sezione per sezione, con una nota per qualsiasi
cosa che non può posizionare. Nulla viene scritto finché non premi
*Restore for real*.

**Scheduled local backups** mostra se sono attivi, dove vanno, l'ultima
esecuzione e se è riuscita, e i file conservati con la loro dimensione.
**Back up now** ne scrive uno immediatamente — la cosa da fare prima di
un aggiornamento.

Tutto qui è riservato agli admin. Vedi
[Backup e ripristino](../install/backup.md).

## Reports

Email periodiche di disponibilità per il team: scegli settimanale,
giornaliera o mensile, il giorno, l'ora e il fuso orario, i destinatari
e il canale email, poi **Send a preview** per vedere il risultato. Solo
admin. Vedi [Report via email](reports.md).

## Appearance

Tre riquadri, ognuno con l'anteprima del proprio tema: **System**
(segue il tuo dispositivo e cambia con esso), **Day** (il tema "carta da
grafico") e **Night** (il tema "radar"). La scelta viene memorizzata nel
browser. L'interruttore dell'intestazione e il *Toggle theme* della
command palette passano tra day e night.

**Open wall mode** sta in fondo alla sezione: solo il bollettino, a
schermo intero, per un monitor nella stanza. ++esc++ lo lascia, e la
command palette lo apre da qualsiasi punto — vedi
[Modalità Wall](overview.md#wall-mode).

## About

Versione del server e stato di salute dei suoi due store, il database
(configurazione dei dispositivi e cronologia degli avvisi) e
VictoriaMetrics (ogni misurazione nei grafici), come riportato da
`GET /api/health` e aggiornato ogni 15 secondi. Uno store che smette di
rispondere mostra un avviso con l'errore e cosa controllare. La sezione
collega anche a questa documentazione e nomina la licenza, Apache 2.0.
