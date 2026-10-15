# MDaemon Email Server

I servizi di posta di un server MDaemon, verificati dall'esterno: SMTP,
IMAP, POP3, webmail e Remote Administration accettano connessioni e
salutano come dovrebbero. Con un account, DumbMonit legge anche la
versione esatta tramite l'API XML. Con l'[agent](agent.md) installato sul
server MDaemon, la stessa pagina mostra anche le code di posta, le
sessioni e i totali dei messaggi, che MDaemon pubblica solo come contatori
di prestazioni Windows.

!!! warning "Validato solo sulla documentazione"

    Questa integrazione è stata costruita dalla documentazione pubblicata
    da MDaemon Technologies e dal formato di risposta dell'API XML
    mostrato lì, non su un vero server MDaemon. Ogni chiamata è una
    lettura, e i controlli delle porte di posta seguono gli standard di
    protocollo, ma nessuno l'ha ancora eseguita su un MDaemon in funzione.
    Lo stesso vale per i contatori di coda letti dall'agent: i loro nomi
    provengono da un elenco pubblicato dei contatori di prestazioni di
    MDaemon, non da un server in funzione. Facci sapere cosa non
    funziona.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_mdaemon_`.

| Metrica | Etichette | Significato |
|---|---|---|
| `service_up` | `service`, `port` | 1 se la porta ha accettato la connessione e, per SMTP, POP3 e IMAP, ha salutato con `220`, `+OK` o `* OK`. 0 se non ha risposto, o ha risposto per rifiutare il servizio (`421`, `554`, `-ERR`, `* BYE`). |
| `service_response_seconds` | `service`, `port` | Tempo fino al saluto, o fino alla connessione per una porta senza saluto. |
| `api_up` | | Solo con un account: 1 se l'API XML ha risposto, 0 se non lo ha fatto. |
| `api_response_seconds` | | Tempo impiegato dall'API XML per rispondere. |
| `api_operation_ok` | | 1 se l'account è stato autorizzato a eseguire la chiamata, 0 se MDaemon ha risposto con uno stato di errore. |
| `info` (valore 1) | `version`, `build`, `source` | La versione: dall'API XML (`source="xml_api"`, `build` è la build del servizio), oppure dal saluto SMTP quando il server lo annuncia lì (`source="smtp_banner"`). |
| `scrape_duration_seconds` | | Tempo impiegato dall'intero controllo. |

**Un'interruzione parziale è comunque un controllo riuscito.** Un
servizio fermo dà `service_up 0` per quel servizio e gli altri vengono
comunque misurati. Solo quando nessun servizio e nessuna API risponde
affatto il monitor è *unreachable*.

**Un account rifiutato è un errore di configurazione, non
un'interruzione.** Una password errata, un indirizzo che l'API XML non
consente, o una porta errata viene mostrata sulla pagina del monitor
con il motivo e non viene notificata.

**Le code provengono dall'agent, non da questo monitor.** MDaemon
pubblica le dimensioni delle sue code (inbound, local, remote, retry,
bad, holding, quarantine), i suoi conteggi di sessioni, le sue statistiche
dei messaggi e il suo uptime come contatori di prestazioni Windows,
nell'oggetto chiamato MDaemon, e nel suo proprio Queue and Statistics
Manager. Nessuna documentazione pubblicata da MDaemon descrive una
chiamata API XML che li restituisca, quindi questo monitor non ne
indovina una: l'[agent](agent.md), installato sul server MDaemon, legge
invece i contatori di prestazioni. Vedi
[Code di posta tramite l'agent](#mail-queues-through-the-agent). La
licenza e lo spazio su disco del mail store non sono esposti tramite
un'interfaccia documentata o un contatore di prestazioni, e non vengono
letti. L'agent osserva anche il disco che contiene il mail store; elenca
il servizio MDaemon nella sua impostazione `services` per osservare il
servizio Windows stesso.

Le [regole integrate](../alerting/rules.md#mdaemon-and-securitygateway)
che si applicano: MDaemon mail service down, MDaemon XML API not
answering, più Monitor unreachable; con l'agent, MDaemon mail queue
growing, MDaemon Bad queue not empty e MDaemon Retry queue high.

## La pagina del monitor

Un monitor MDaemon mostra un pannello **Mail services** sopra i
grafici, letto dall'ultima misurazione memorizzata: aprire la pagina non
si connette mai al server. Elenca ogni servizio osservato con la sua
porta, una parola e un colore (Answering, Down), e il suo tempo di
risposta; poi la versione e da dove è stata letta. I servizi fermi vengono
prima.

Quando l'agent sullo stesso server riporta i contatori di MDaemon, segue
un pannello **Mail queues**: il numero di messaggi in ogni coda (una coda
congelata, una coda Bad non vuota e più di cinquanta messaggi in Retry
sono segnalati in parole), le sessioni attive, i messaggi e i verdetti di
spam, virus e DNSBL delle ultime 24 ore, l'uptime di MDaemon, e i server
interni che MDaemon riporta come inattivi. Lo stesso pannello appare sulla
pagina propria dell'agent.

## Code di posta tramite l'agent {#mail-queues-through-the-agent}

1. Installa l'[agent](agent.md#install) sul server Windows che esegue
   MDaemon. Nulla altro è necessario: l'agent nota il servizio Windows
   chiamato `MDaemon` e legge i contatori dell'oggetto prestazioni MDaemon
   a ogni periodo di campionamento. Per forzarlo attivo o disattivo,
   imposta `mdaemon: true` o `mdaemon: false` in `agent.yaml`.
2. Le code appaiono nella pagina dell'agent. Per mostrarle anche su
   questo monitor MDaemon, DumbMonit cerca l'agent sulla stessa
   macchina: l'agent impostato come genitore di questo monitor,
   oppure l'agent relay che lo sonda, oppure l'agent il cui nome host
   corrisponde all'indirizzo di questo monitor (`mail` per
   `mail.example.com`), oppure, quando c'è un monitor MDaemon e un
   agent che riporta i contatori MDaemon, quell'agent. Se nessuno
   corrisponde, modifica questo monitor e scegli l'agent come suo
   **Parent device**: l'agent e MDaemon condividono la macchina, così
   quando l'agent diventa silenzioso gli avvisi di questo monitor
   vengono anch'essi trattenuti.

Le serie appartengono all'agent, con la sua etichetta `target`. Tutte con
prefisso `dumbmonit_mdaemon_`:

| Metrica | Etichette | Contatori di prestazioni letti |
|---|---|---|
| `queue_messages` | `queue`: `inbound`, `local`, `remote`, `retry`, `bad`, `holding`, `lan`, `quarantine`, `raw` | `Inbound queue messages`, `Local queue messages`, `Remote queue messages`, `Retry queue messages`, `Bad queue messages`, `Holding queue messages`, `LAN queue messages`, `Quarantine queue messages`, `RAW queue messages` |
| `queue_frozen` (1 congelata) | `queue`: `inbound`, `local`, `remote` | `Inbound queue frozen`, `Local queue frozen`, `Remote queue frozen` |
| `sessions_active` | `protocol`: `smtp_in`, `smtp_out`, `pop3_in`, `pop3_out`, `imap`, `webmail` | `Active SMTP (in) sessions`, `Active SMTP (out) sessions`, `Active POP3 (in) sessions`, `Active POP3 (out) sessions`, `Active IMAP sessions`, `Active Webmail sessions` |
| `sessions_total` (contatore) | `protocol`: `smtp_in`, `smtp_out`, `pop3`, `imap` | `SMTP sessions (in) total`, `SMTP sessions (out) total`, `POP3 sessions total`, `IMAP sessions total` |
| `messages_total` (contatore) | `protocol`: `smtp_in`, `smtp_out`, `domainpop_in` | `SMTP messages (in) total`, `SMTP messages (out) total`, `DomainPOP messages (in) total` |
| `filtered_messages_total` (contatore) | `filter`: `spam`, `virus`, `dnsbl`; `verdict`: `accepted`, `refused` | `spam accepted total`, `spam refused total`, `Viruses accepted total`, `Viruses refused total`, `DNSBL accepted total`, `DNSBL refused total` |
| `server_active` (1 attivo) | `server`: `smtp`, `pop3`, `imap`, `webmail`, `webadmin`, `activesync`, `antispam`, `antivirus`, `minger`, `multipop`, `domainpop` | `SMTP server state`, `POP3 server state`, `IMAP server state`, `Web Mail server state`, `Web Admin server state`, `ActiveSync server state`, `AntiSpam server state`, `AntiVirus server state`, `Minger server state`, `MultiPOP server state`, `DomainPOP server state` |
| `running` (1 in esecuzione) | | `MDaemon running state` |
| `uptime_seconds` | | `MDaemon up time` |

I totali contano da quando MDaemon è stato avviato l'ultima volta:
applica `increase()` o `rate()` nelle query, come fa la pagina per le sue
cifre su 24 ore. I contatori al secondo (`SMTP messages (in)/sec`…) non
vengono letti, poiché i totali danno gli stessi tassi. Un contatore che la
tua versione di MDaemon non ha, o nomina diversamente, viene segnalato
una volta nel log dell'agent (`performance counter not available`) e gli
altri vengono comunque letti; per leggerne uno che il preset non ha,
elencalo sotto `perf_counters` in `agent.yaml` (vedi
[Contatori di prestazioni Windows](agent.md#windows-performance-counters)).

## Osservare i servizi di posta, e opzionalmente l'API XML

1. Nulla da installare sul server. Senza credenziale, DumbMonit si connette alle porte di posta dall'esterno e legge ogni saluto: SMTP (25), IMAP (143) e Webmail (3000) per impostazione predefinita. Aggiungi POP3, MSA, le porte TLS o Remote Administration nell'opzione Services, come un elenco come questo.

    ```
    smtp,msa,imap,pop3,imaps,webmail
    ```

2. Per leggere anche la versione tramite l'API XML, crea un account dedicato in MDaemon, nominato per esempio come segue, con una password lunga usata in nessun altro posto. Dagli il livello più basso che il tuo MDaemon accetta per l'API XML, e nulla di più: DumbMonit chiama solo GetVersionInfo, che non legge alcuna casella di posta e non modifica nulla.

    ```
    dumbmonit@example.com
    ```

3. Da MDaemon 24, l'API XML risponde solo agli indirizzi che consente. Apri Setup → XML API Service → Address Restrictions (Setup → XML API Management su MDaemon 26) e consenti l'indirizzo da cui si connette DumbMonit.

4. Verifica dall'host DumbMonit che l'account raggiunga l'API. Il comando chiede la password e stampa una risposta che inizia con `<MDaemon><API productversion=…>`.

    ```
    curl -k -u dumbmonit@example.com -H 'Content-Type: text/xml' --data '<MDaemon><API><Request version="20.0.0" echo="0" verbose="0"><Operation>GetVersionInfo</Operation><Parameters/></Request></API></MDaemon>' https://mail.example.com:444/MdMgmtWS/
    ```

5. In DumbMonit, inserisci l'indirizzo del server, per esempio "mail.example.com", poi l'indirizzo email completo dell'account come nome utente, e la sua password. L'API XML viene raggiunta sulla porta HTTPS di Remote Administration, 444 per impostazione predefinita.

!!! warning
    MDaemon non ha un ruolo di sola lettura per l'API XML: non riutilizzare mai il tuo account, e tieni questa password fuori da qualsiasi altro strumento. Dynamic Screening può bloccare un indirizzo dopo login falliti ripetuti, quindi una password errata qui può far bloccare l'host DumbMonit: testa prima con il comando sopra. Remote Administration usa spesso un certificato autofirmato: se la connessione viene rifiutata per questo motivo, spunta "Accept an unverifiable certificate" nelle opzioni. Le dimensioni delle code di posta non vengono lette da questo monitor: MDaemon le pubblica solo come contatori di prestazioni Windows, che l'agent DumbMonit legge quando è installato sul server di posta.

L'API XML è servita dal server web Remote Administration: porta 1000 su
HTTP e 444 su HTTPS per impostazione predefinita, su `/MdMgmtWS/`, oppure
da IIS quando Remote Administration gira lì. Il suo riferimento completo
viene fornito con il server: apri
`https://mail.example.com:444/MdMgmtWS` in un browser, oppure guarda in
`\MDaemon\Docs\API\XML API\`.

## Credenziali

| Credenziale | Campi |
|---|---|
| Services only | Nessuna: le porte di posta vengono controllate senza accedere. |
| XML API account | L'indirizzo email completo dell'account, e la sua password, inviati come autenticazione HTTP di base. |

Indirizzo: il nome host o IP del server, per esempio `mail.example.com`.
Una porta scritta nell'indirizzo (`mail.example.com:8444`) viene presa
come porta dell'API XML.

## Opzioni

| Opzione | Predefinito | Effetto |
|---|---|---|
| `services` | `smtp,imap,webmail` | Cosa controllare, separato da virgole: `smtp` (25), `msa` (587), `smtps` (465), `pop3` (110), `pop3s` (995), `imap` (143), `imaps` (993), `webmail` (3000), `remote_admin` (1000), `remote_admin_https` (444), `xmpp` (5222). `nome:porta` cambia una porta; qualsiasi altro nome con la sua porta aggiunge un servizio tuo, controllato solo per connessione. `none` non controlla alcuna porta. |
| `request_timeout_seconds` | `10` | Tempo consentito per ogni connessione, saluto e chiamata API, da 1 a 60. |
| `api_port` | `444` | La porta Remote Administration che serve `/MdMgmtWS/`. Usata solo con un account. |
| `api_tls` | `true` | HTTPS verso l'API XML. Deseleziona solo per Remote Administration su HTTP semplice (porta 1000 per impostazione predefinita): la password allora attraversa la rete in chiaro. |
| `insecure_tls` | `false` | Accetta un certificato autofirmato sull'API XML. |

Le porte TLS (`smtps`, `pop3s`, `imaps`, `remote_admin_https`) vengono
controllate solo per connessione. Per osservare la scadenza dei loro
certificati, aggiungi un monitor [Certificato TLS](services.md#tls)
per ciascuna.
