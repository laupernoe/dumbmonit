# Servizi: HTTP, TCP, DNS, ping, TLS, NTP, SMTP, SQL, MQTT, WebSocket

Undici monitor osservano i **servizi** invece dell'apparecchiatura,
stile Uptime Kuma: una pagina web, una porta, un nome DNS, un host, un
certificato, un server orario, un relay di posta, un database, un
broker MQTT, un endpoint WebSocket. Ogni servizio ottiene una barra
della cronologia, un tempo di risposta e una percentuale di
disponibilità sulla sua pagina del dispositivo.

Gli ultimi cinque vanno oltre l'apertura di una connessione: iniziano
una sessione reale nel protocollo proprio del servizio. Quella è la
differenza tra "la porta 5432 è aperta" e "il database accetta ancora
il mio account e risponde a una query". NTP sta tra i due: una singola
richiesta e risposta, nel protocollo proprio di NTP, sufficiente a
leggere l'orologio del server senza una sessione.

## Come funziona lo stato di un servizio

Un servizio può rispondere ed essere comunque inattivo: un `500`, una
parola chiave mancante, un certificato scaduto. Quindi questi monitor
scrivono un punto a ogni controllo, con `probe_success` a 1 o 0, invece
di diventare silenziosi come un dispositivo hardware. Questo è ciò che
rende possibile una percentuale di disponibilità:
`avg_over_time(dumbmonit_probe_success[30d])`.

| Situazione | Cosa succede |
|---|---|
| Opzione o indirizzo non valido, socket ICMP non consentiti (vedi Ping) | Errore di configurazione: nulla viene scritto, la pagina del dispositivo mostra l'errore, nessun avviso "down". |
| Connessione rifiutata, timeout, TLS rifiutato, `500`, parola chiave assente, perdita totale di pacchetti, password rifiutata, query fallita | `probe_success = 0`: l'interruzione viene registrata, datata e contata. |
| Tutto bene | `probe_success = 1`. |

Ogni monitor ha il proprio timeout (`timeout_seconds`, 5 s per
impostazione predefinita, al massimo 60 s), più breve del
`DUMBMONIT_PROBE_TIMEOUT_SECS` del server: interrotto dallo scheduler,
non potrebbe scrivere il suo zero.

Le regole integrate che si applicano a tutti gli undici: Service down
(3 minuti), Service flapping (più di sei cambi di stato in trenta
minuti), Slow service (più di 3 s per dieci minuti). Per ogni controllo
che legge un certificato — `http`, `tls`, `smtp`, `mqtt`, `websocket`:
Certificate expiring soon (14 giorni) e Certificate expired.

Una password rifiutata non è mai un'interruzione del servizio: viene
registrata come `reason="auth"`, così l'avviso dice cosa correggere.

### Metriche

Tutti gauge, tutti con prefisso `dumbmonit_`, tutti etichettati
`probe="http|tcp|dns|ping|tls|smtp|postgres|mysql|mqtt|websocket"`
oltre a `target`, `host` e `tag_*`.

| Metrica | Monitor | Significato |
|---|---|---|
| `probe_success` | tutti | 1 se il servizio risponde correttamente, 0 altrimenti. |
| `probe_duration_seconds` | tutti | Durata totale del controllo. |
| `probe_failure_info` | tutti | Presenza (1) con un'etichetta `reason`: `dns`, `connect`, `timeout`, `tls`, `cert_expired`, `status`, `keyword`, `json`, `body`, `packet_loss`, `record`, `auth`, `protocol`, `query`, `payload`. |
| `probe_http_status_code` | http, websocket | Codice di stato ottenuto. |
| `probe_http_first_byte_seconds` | http | Tempo fino agli header della risposta. |
| `probe_http_content_bytes` | http | Dimensione del corpo. |
| `probe_connect_seconds` | tutti tranne dns e ping | Stabilimento della connessione (login incluso per i database). |
| `probe_tls_handshake_seconds` | tls, http, smtp, mqtt, websocket | Solo la negoziazione TLS. |
| `probe_tls_version_info` | tls, http, smtp, mqtt, websocket | Presenza; etichetta `version`. |
| `probe_ssl_cert_expiry_days` | tls, http, smtp, mqtt, websocket | Giorni prima della scadenza, negativo se scaduto. |
| `probe_ssl_cert_valid` | tls, http, smtp, mqtt, websocket | 1 se la catena porta a un'autorità conosciuta. |
| `probe_ssl_cert_issuer_info` | tls, http, smtp, mqtt, websocket | Presenza; etichetta `issuer`. |
| `probe_dns_lookup_seconds` | dns | Tempo di risoluzione. |
| `probe_dns_answer_records` | dns | Record del tipo richiesto. |
| `probe_dns_answer_fingerprint` | dns | Con "Alert when the answer changes": un fingerprint a 32 bit della risposta ordinata. Il suo valore non significa nulla; solo i suoi cambiamenti contano. |
| `probe_icmp_rtt_seconds`, `_min_seconds`, `_max_seconds` | ping | Andata-ritorno medio, più breve e più lungo. |
| `probe_icmp_packet_loss_ratio` | ping | Perdita tra 0 e 1. |
| `probe_icmp_packets_sent`, `probe_icmp_packets_received` | ping | Echo inviati e ricevuti. |
| `probe_smtp_greeting_seconds` | smtp | Tempo fino al banner `220`. |
| `probe_smtp_ehlo_seconds` | smtp | Andata-ritorno del comando `EHLO`. |
| `probe_smtp_capabilities` | smtp | Estensioni pubblicizzate. |
| `probe_smtp_authenticated` | smtp | 1 quando `AUTH` è riuscito. |
| `probe_sql_query_seconds` | postgres, mysql | Solo l'esecuzione della query. |
| `probe_sql_rows` | postgres, mysql | Righe restituite. |
| `probe_sql_value` | postgres, mysql | Prima colonna della prima riga, quando è un numero. |
| `probe_mqtt_connack_seconds` | mqtt | Tempo fino al `CONNACK`. |
| `probe_mqtt_suback_seconds` | mqtt | Tempo fino al `SUBACK`. |
| `probe_mqtt_message_bytes` | mqtt | Dimensione del messaggio retained. |
| `probe_mqtt_message_value` | mqtt | Il suo contenuto, quando è un numero. |
| `probe_ws_handshake_seconds` | websocket | Handshake di apertura. |
| `probe_ws_message_bytes` | websocket | Dimensione del frame ricevuto. |

## HTTP

**Sito web o API web (HTTP)** — verifica che una pagina o un'API
risponda, con il codice di stato giusto, il contenuto giusto, e un
certificato valido. Esempi: un sito web, una pagina di salute di
un'applicazione, un'API REST, l'interfaccia di un servizio self-hosted.

### Configurazione

1. Nell'indirizzo, incolla l'URL completo della pagina da monitorare. Senza "http://" o "https://", HTTPS viene assunto.
2. Preferisci una pagina leggera che non richiede login, per esempio la pagina di salute dell'applicazione ("/health", "/status"), invece della home page.
3. Se la pagina richiede autenticazione, compila la credenziale: un nome utente e password danno l'autenticazione basic, un token viene inviato come "Bearer".
4. Per impostazione predefinita, qualsiasi codice di stato tra 200 e 299 va bene. Per andare oltre, richiedi una parola chiave nella pagina o un valore specifico in una risposta JSON.
5. Su HTTPS, il certificato viene letto automaticamente: sarai avvisato prima che scada.

!!! warning
    Non scrivere mai una password o un token nelle opzioni: vengono
    copiati in chiaro su ogni misurazione. Usa il campo credenziale,
    che è cifrato.

Credenziali: nessuna, username / password (autenticazione basic)
oppure token API (`Authorization: Bearer`).

### Opzioni

| Chiave | Etichetta | Predefinito | Aiuto |
|---|---|---|---|
| `method` | HTTP method | `GET` | GET si adatta a quasi ogni caso. HEAD evita di scaricare la pagina quando conta solo il codice di stato. Scelte: GET, HEAD, POST, PUT, PATCH, DELETE, OPTIONS. |
| `accepted_status` | Accepted status codes | `200-299` | Codici o intervalli considerati normali, separati da virgole. Qualsiasi altro codice conta come inattivo. Esempio: `200-299,301,404`. |
| `keyword` | Expected keyword | *(vuoto)* | Testo che deve apparire nella pagina. Se manca, il servizio viene riportato inattivo anche se la pagina risponde. |
| `keyword_absent` | Keyword must be absent | `false` | Inverte il controllo: la presenza della parola chiave segnala un fallimento. Utile per una pagina di errore che risponde 200. |
| `keyword_case_sensitive` | Match keyword case | `false` | Per impostazione predefinita, maiuscole e minuscole sono trattate uguali. |
| `json_path` | JSON path to check | *(vuoto)* | Per una risposta JSON: percorso del valore da controllare. Compilalo insieme al valore atteso, mai l'uno senza l'altro. Esempio: `$.status`. |
| `json_expect` | Expected JSON value | *(vuoto)* | Valore che il percorso JSON sopra deve avere, per esempio "ok" o "true". |
| `headers` | Extra headers | *(vuoto)* | Uno o più header "Name: value", separati da "\|". Non metterci segreti: le opzioni sono visibili nei grafici. |
| `body` | Request body | *(vuoto)* | Contenuto inviato con la richiesta, per i metodi che ne richiedono uno (POST, PUT…). |
| `follow_redirects` | Follow redirects | `true` | Disattiva per monitorare il redirect stesso, per esempio un 301 verso HTTPS. |
| `max_redirects` | Maximum redirects followed | `10` | Tra 1 e 20. Nessun effetto se i redirect non vengono seguiti. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | Il controllo non fallisce più su un certificato autofirmato o emesso da un'autorità privata. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Per impostazione predefinita il controllo rifiuta indirizzi che solo l'host DumbMonit stesso può raggiungere (127.0.0.1, ::1, 169.254.x.x). Abilita questo per monitorare un servizio in esecuzione sull'host DumbMonit. Gli indirizzi di LAN privata (10.x, 192.168.x) sono sempre consentiti. |
| `check_certificate` | Read the certificate | `true` | Su HTTPS, registra anche la data di scadenza del certificato così puoi essere avvisato prima che scada. |
| `max_body_bytes` | Maximum bytes read | `524288` | Oltre questo, il resto della pagina non viene scaricato. La parola chiave e il percorso JSON vengono cercati solo in questa parte. |
| `user_agent` | Announced identity (User-Agent) | `DumbMonit/<version>` | Nome che il controllo dà al server. Cambialo se il server filtra i robot. |
| `timeout_seconds` | Timeout (seconds) | `5` | Tempo dopo il quale il servizio viene riportato inattivo se non ha risposto. Tra 1 e 60. |

## TCP

**Porta di rete (TCP)** — verifica che una porta accetti connessioni:
SSH, database, condivisione file, server di gioco, stampante di rete.

### Configurazione

1. Nell'indirizzo, scrivi l'host seguito dalla porta, separati da due punti: "nas.home.lan:22" oppure "192.168.1.5:445".
2. Per un indirizzo IPv6, metterlo tra parentesi: "[fd00::1]:445".
3. Puoi anche lasciare l'indirizzo senza una porta e inserirla nell'opzione "Port": uno dei due è richiesto.
4. Viene testata solo l'apertura della connessione: nessun dato viene inviato al servizio, quindi non ha alcun effetto su di esso.

!!! warning
    Una porta aperta non prova che l'applicazione dietro di essa
    funzioni. Per un servizio web, preferisci il tipo "Website or web
    API", che legge la risposta.

### Opzioni

| Chiave | Etichetta | Predefinito | Aiuto |
|---|---|---|---|
| `port` | Port | *(vuoto)* | Porta da aprire, se l'indirizzo non la fornisce già come "host:porta". Uno dei due è richiesto. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Per impostazione predefinita il controllo rifiuta indirizzi che solo l'host DumbMonit stesso può raggiungere (127.0.0.1, ::1, 169.254.x.x). Abilita questo per monitorare un servizio in esecuzione sull'host DumbMonit. Gli indirizzi di LAN privata (10.x, 192.168.x) sono sempre consentiti. |
| `timeout_seconds` | Timeout (seconds) | `5` | Tempo dopo il quale il servizio viene riportato inattivo se non ha risposto. Tra 1 e 60. |

## DNS

**Nome di dominio (DNS)** — verifica che un nome risolva, e che punti
all'indirizzo giusto. Esempi: il tuo nome di dominio, un nome interno
servito dal tuo Pi-hole o AdGuard, un record MX.

### Configurazione

1. Nell'indirizzo, scrivi il nome da risolvere, senza "http://": "www.example.com".
2. Per impostazione predefinita, il controllo chiede al resolver di sistema un indirizzo IPv4 (record A). Scegli un altro tipo di record nelle opzioni se necessario.
3. Per monitorare il tuo proprio server DNS, inserisci il suo indirizzo IP in "DNS server to query": il controllo fallirà se smette di rispondere.
4. Per rilevare un hijacking o una configurazione errata, elenca in "Expected values" gli indirizzi che la risposta deve contenere.

!!! warning
    Il server DNS viene dato dal suo indirizzo IP, non da un nome:
    occorrerebbe un resolver per risolvere il resolver.

### Opzioni

| Chiave | Etichetta | Predefinito | Aiuto |
|---|---|---|---|
| `record_type` | Record type | `A` | A per un indirizzo IPv4, AAAA per IPv6, MX per la posta, CNAME per un alias… Scelte: A, AAAA, CNAME, MX, TXT, NS, SOA, SRV, PTR, CAA. |
| `resolver` | DNS server to query | *(vuoto)* | Indirizzo IP di un resolver, porta opzionale. Vuoto: il resolver di sistema. Utile per controllare il tuo proprio server DNS. Esempio: `1.1.1.1` oppure `10.0.0.1:5353`. |
| `expect` | Expected values | *(vuoto)* | Valori che devono apparire tutti nella risposta, separati da virgole. Vuoto: viene controllata solo la risoluzione. |
| `expect_mode` | Comparison | `contains` | `contains`: ogni valore atteso deve apparire da qualche parte nella risposta. `exact`: la risposta deve contenere quei valori e nulla altro. |
| `forbid` | Forbidden values | *(vuoto)* | Valori che non devono mai apparire nella risposta, separati da virgole. Controllati prima di quelli attesi. |
| `alert_on_change` | Alert when the answer changes | `false` | Registra un fingerprint della risposta ordinata e avvisa ogni volta che cambia, senza dover scrivere i valori attesi. |
| `timeout_seconds` | Timeout (seconds) | `5` | Tempo dopo il quale il servizio viene riportato inattivo se non ha risposto. Tra 1 e 60. |

### Affermare cosa risponde una zona

`expect` da solo rileva il record che è scomparso. Non rileva il record
che è stato *aggiunto*: una zona hijacked spesso mantiene l'`A`
legittimo e ne mette un secondo accanto, e un client allora raggiunge
l'uno o l'altro a caso.

- `expect_mode = exact` rifiuta qualsiasi valore che l'aspettativa non
  copre. L'ordine non conta — DNS non lo mantiene.
- `forbid` nomina valori che non devono mai ritornare: l'indirizzo di
  un host precedente, un `TXT` di validazione scaduto, un server di
  posta che hai dismesso. Viene controllato prima, quindi vince su
  `expect`.

Entrambi confrontano senza distinguere maiuscole/minuscole, ignorano il
punto finale di un nome e le virgolette attorno a un `TXT`, e trattano
un valore atteso come trovato quando appare dentro un record — così un
`MX` corrisponde su `mail.example.com` senza dover copiare la sua
priorità.

### Avvisare quando la risposta cambia

Scrivere i valori attesi non è sempre pratico: un nome servito da un
CDN, record che qualcun altro gestisce, oppure semplicemente una zona
di cui vuoi sentire ogni volta che si muove. Spunta **Alert when the
answer changes**: ogni controllo allora registra un fingerprint della
risposta, e la regola integrata
[DNS answer changed](../alerting/rules.md#reverse-proxies-and-domains)
si attiva quando differisce da quella precedente. L'avviso si risolve
da solo un'ora dopo; la nuova risposta diventa il riferimento da quel
momento.

Il fingerprint copre i valori della risposta, ordinati e senza
duplicati, con la catena CNAME inclusa: un round-robin che restituisce
gli stessi indirizzi in un altro ordine non lo cambia, né lo fa un TTL.
Un nome la cui risposta cambia a ogni query (un grande pool come
`pool.ntp.org`, oppure DNS geografico che risponde diversamente da un
resolver all'altro) avviserebbe continuamente: lascia disattivata
l'opzione per quelli, oppure interroga un resolver fisso.

## Ping

**Host raggiungibile (ping)** — invia alcuni echo ICMP e misura il
tempo di risposta e la perdita di pacchetti. Esempi: router o gateway
domestico, access point Wi-Fi, stampante, macchina senza agent o SNMP,
host remoto.

### Configurazione

1. Nell'indirizzo, scrivi il nome host o l'indirizzo IP: "192.168.1.1" oppure "router.home.lan".
2. A ogni sondaggio, il controllo invia quattro echo e misura il tempo di risposta, insieme alla quota di pacchetti persi.
3. Per impostazione predefinita, solo una perdita totale conta come inattivo; la perdita parziale resta visibile nei grafici. Abbassa "Tolerated loss" per essere avvisato prima.
4. Se DumbMonit funziona in Docker, mantieni le righe `sysctls:` del docker-compose.yml fornito sotto il servizio dumbmonit: consentono all'utente non privilegiato del container di aprire socket echo ICMP. Nessuna capability è necessaria.

!!! warning
    Senza quel sysctl, il controllo non può aprire un socket ICMP: lo
    segnala come un errore di configurazione, non come un guasto
    dell'host. Alcuni dispositivi ignorano anche i ping di proposito:
    verificalo prima di trarre conclusioni.

```yaml
services:
  dumbmonit:
    sysctls:
      net.ipv4.ping_group_range: "0 2147483647"
```

Fuori da Docker, `sysctl -w net.ipv4.ping_group_range="0 2147483647"`
sull'host fa lo stesso, e un socket raw (`NET_RAW`, root) funziona
anch'esso.

### Opzioni

| Chiave | Etichetta | Predefinito | Aiuto |
|---|---|---|---|
| `count` | Number of echoes | `4` | Pacchetti inviati a ogni sondaggio, da 1 a 20. |
| `packet_timeout_ms` | Wait per packet (ms) | `1000` | Tempo consentito per ogni risposta, da 50 a 10.000 ms. |
| `interval_ms` | Gap between two echoes (ms) | `100` | Pausa tra due echo, da 0 a 5.000 ms. |
| `payload_bytes` | Payload size (bytes) | `56` | Dati portati da ogni echo, da 0 a 1.400 byte. |
| `ip_version` | IP version | `auto` | "auto" prende il primo indirizzo risolto; forza 4 o 6 se l'host ha entrambi e uno non risponde. Scelte: auto, 4, 6. |
| `loss_threshold_percent` | Tolerated loss (%) | `100` | Sopra questa percentuale di pacchetti persi, l'host viene riportato inattivo. 100: solo una perdita totale conta, la perdita parziale resta visibile nei grafici. |
| `timeout_seconds` | Timeout (seconds) | `5` | Budget totale per il sondaggio, da 1 a 60. Rifiutato se non copre "numero di echo × attesa per pacchetto". |

## TLS

**Certificato TLS** — verifica che un certificato sia valido e avvisa
prima che scada, su qualsiasi porta cifrata. Esempi: server di posta
(IMAPS, SMTPS), reverse proxy, directory LDAPS, broker MQTT,
interfaccia di amministrazione.

### Configurazione

1. Nell'indirizzo, scrivi l'host e, se non è 443, la porta del servizio cifrato: "mail.example.com:993", "ldap.home.lan:636".
2. Il controllo apre una connessione cifrata, legge il certificato presentato e chiude subito: nessun dato viene scambiato con l'applicazione.
3. Registra il numero di giorni prima della scadenza, l'emittente e la versione TLS. Le regole di avviso fornite avvisano a quattordici giorni, poi alla scadenza.
4. Se il servizio viene raggiunto tramite il suo indirizzo IP ma il certificato porta un nome, inserisci quel nome in "Server name (SNI)".
5. Per un certificato emesso dalla tua propria autorità, spunta "Accept an unverifiable certificate": la data di scadenza viene comunque monitorata.

!!! warning
    Per un sito web, il tipo "Website or web API" legge già il
    certificato: questo tipo è pensato per servizi che non parlano
    HTTP, o la cui applicazione non vuoi colpire.

L'handshake si completa anche quando il certificato viene rifiutato:
questo è ciò che permette di leggere la data di scadenza di un
certificato già scaduto. Il verdetto della verifica è in
`probe_ssl_cert_valid`.

### Opzioni

| Chiave | Etichetta | Predefinito | Aiuto |
|---|---|---|---|
| `server_name` | Server name (SNI) | *(vuoto)* | Nome di dominio annunciato al server e verificato nel certificato. Impostalo quando l'indirizzo è un IP dietro un reverse proxy. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | Una catena non verificabile non conta più come un fallimento: la data di scadenza viene comunque registrata. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Per impostazione predefinita il controllo rifiuta indirizzi che solo l'host DumbMonit stesso può raggiungere (127.0.0.1, ::1, 169.254.x.x). Abilita questo per monitorare un servizio in esecuzione sull'host DumbMonit. Gli indirizzi di LAN privata (10.x, 192.168.x) sono sempre consentiti. |
| `timeout_seconds` | Timeout (seconds) | `5` | Tempo dopo il quale il servizio viene riportato inattivo se non ha risposto. Tra 1 e 60. |

## SMTP

**Relay di posta (SMTP)** — apre una sessione reale su un server di
posta: saluto, EHLO, STARTTLS, e il login se ne fornisci uno. Esempi:
il relay SMTP del tuo provider, un Postfix o msmtp locale, Proxmox
Mail Gateway, la porta submission di un server di posta.

### Configurazione

1. Nell'indirizzo, scrivi il server di posta, e la porta se non è quella usuale: "smtp.example.com" oppure "smtp.example.com:2525".
2. Scegli la cifratura che usa il tuo relay: STARTTLS per la porta submission 587, TLS per la porta 465, None per un relay sulla tua rete che ascolta sulla porta 25.
3. Per verificare che il relay accetti ancora il tuo account, compila la credenziale: il controllo allora esegue AUTH e riporta una password rifiutata come tale, non come un'interruzione.
4. Nulla viene mai inviato: il controllo si ferma dopo il saluto, le estensioni e il login opzionale, poi chiude. Nessun messaggio entra nella coda.
5. Per essere avvisato quando un'estensione scompare, nominala in "Expected extension": un relay che smette di pubblicizzare STARTTLS è un relay che invierebbe la tua posta in chiaro.

!!! warning
    Una sessione accettata non prova che la posta esca. Un relay che
    accetta tutto e lo accoda per sempre risponde perfettamente qui:
    osserva invece le code stesse per quello.

Credenziali: nessuna, oppure un nome utente e password inviati con
`AUTH PLAIN` (oppure `AUTH LOGIN` quando il server offre solo questo).
Vengono inviati **dopo** che la connessione è cifrata e dopo che il
certificato è stato giudicato: un relay la cui catena non può essere
verificata non li riceve mai.

Su TLS o STARTTLS, il certificato viene letto come lo legge il
controllo TLS — scadenza, emittente, versione del protocollo — così un
relay di posta è anche un certificato di cui vieni avvisato.

### Cosa non può rilevare

Che la posta esca effettivamente. Un relay che accetta ogni sessione e
poi rinvia tutto risponde perfettamente qui; osserva invece le sue code
(vedi la pagina Proxmox Mail Gateway). Non invia mai nemmeno un
messaggio, quindi non dice nulla sul filtraggio dei destinatari, SPF o
DKIM.

### Opzioni

| Chiave | Etichetta | Predefinito | Aiuto |
|---|---|---|---|
| `security` | Encryption | `starttls` | STARTTLS inizia in chiaro e aggiorna (porta 587), TLS cifra dal primo byte (porta 465), None non cifra mai (porta 25) e si adatta solo a un relay sulla tua rete. |
| `port` | Port | *(vuoto)* | Usata se l'indirizzo non ne fornisce una. Vuoto: 587 con STARTTLS, 465 con TLS, 25 senza cifratura. |
| `helo_name` | Name announced (EHLO) | `dumbmonit` | Nome che il controllo dà quando si presenta. Un relay strict rifiuta un nome che non può risolvere. |
| `expect_capability` | Expected extension | *(vuoto)* | Estensione che il server deve pubblicizzare nella sua risposta EHLO, per esempio STARTTLS o AUTH. Vuoto: nessuna aspettativa. |
| `server_name` | Server name (SNI) | *(l'host)* | Nome di dominio annunciato al server e verificato nel certificato. Impostalo quando l'indirizzo è un IP. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | Una catena non verificabile non conta più come un fallimento: la data di scadenza viene comunque registrata. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Vedi "Indirizzi che i controlli rifiutano" sotto. |
| `timeout_seconds` | Timeout (seconds) | `5` | Tempo dopo il quale il servizio viene riportato inattivo se non ha risposto. Tra 1 e 60. |

### Motivi di fallimento

| `reason` | Cosa è successo |
|---|---|
| `connect` | La porta è chiusa o il relay è irraggiungibile. |
| `protocol` | Qualcosa ha risposto, ma non SMTP — oppure ha rifiutato la connessione, `EHLO`, o STARTTLS che aveva pubblicizzato. |
| `auth` | L'account è stato rifiutato (`535`, `530`, `534`). Il relay va bene; la credenziale no. |
| `payload` | L'estensione nominata in "Expected extension" non viene pubblicizzata. |
| `tls`, `cert_expired` | Stesso significato del controllo TLS. |

## PostgreSQL

**Database PostgreSQL** — si connette, autentica ed esegue una query:
il segnale per un database che è attivo ma non risponde più. Esempi: il
database dietro Nextcloud o Immich, un recorder di Home Assistant, un
database applicativo, un'istanza TimescaleDB.

### Configurazione

1. Sul server, crea un account per il monitoraggio e non dargli nulla di più del diritto di connettersi: CREATE ROLE dumbmonit LOGIN PASSWORD 'a-long-password';
2. Assicurati che quell'account possa raggiungere il server dalla macchina DumbMonit, il che di solito significa una riga in più in pg_hba.conf seguita da un ricarico della configurazione.
3. Nell'indirizzo, scrivi il server, e la porta se non è 5432: "db.home.lan" oppure "db.home.lan:5433".
4. La query predefinita è SELECT 1, che non legge alcuna tabella: nulla altro deve essere concesso. Il tempo di connessione e il tempo di query vengono registrati separatamente.
5. Per osservare qualcosa tuo, sostituisci la query con una che restituisce un singolo numero, e diventa un grafico: una lunghezza di coda, un conteggio di righe, un ritardo di replicazione.

!!! warning
    Il controllo apre una connessione reale a ogni sondaggio. Su
    un'istanza già vicina al suo limite di connessioni, dagli un
    intervallo più lungo.

Credenziali: un nome utente e password, richiesti. Un database non
viene mai monitorato anonimamente, quindi il modulo non offre nulla
altro.

### Cosa non può rilevare

Il ritardo di replicazione, un lock che si prolunga, una tabella che
continua a crescere. Tutto questo vive nelle vist amministrative
proprie del motore — scrivi la query tu stesso, e il numero che
restituisce diventa `probe_sql_value`, un grafico su cui puoi avvisare.

### Opzioni

| Chiave | Etichetta | Predefinito | Aiuto |
|---|---|---|---|
| `port` | Port | `5432` | Usata se l'indirizzo non ne fornisce una. |
| `database` | Database | `postgres` | Database aperto alla connessione. L'account di monitoraggio deve poter connettersi ad esso. |
| `query` | Query | `SELECT 1` | Eseguita a ogni controllo. La predefinita non legge alcuna tabella, così l'account non ha bisogno di alcun privilegio oltre alla connessione. Una query che restituisce un numero la trasforma in un grafico. |
| `expect` | Expected value | *(vuoto)* | Valore che la prima colonna della prima riga deve avere. Vuoto: viene controllato solo il successo della query. |
| `sslmode` | Encryption | `prefer` | `prefer` cifra quando il server lo offre, `require` rifiuta di connettersi senza, `verify-full` controlla anche il certificato e il nome, `disable` non cifra mai. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Vedi "Indirizzi che i controlli rifiutano" sotto. |
| `timeout_seconds` | Timeout (seconds) | `5` | Tempo dopo il quale il servizio viene riportato inattivo se non ha risposto. Tra 1 e 60. |

## MySQL e MariaDB

**Database MySQL o MariaDB** — si connette, autentica ed esegue una
query: il segnale per un database che è attivo ma non risponde più.
Esempi: il database dietro un WordPress, un Nextcloud o un Kimai, un
database applicativo, un MariaDB in Docker.

### Configurazione

1. Sul server, crea un account per il monitoraggio senza alcun privilegio: CREATE USER 'dumbmonit'@'%' IDENTIFIED BY 'a-long-password';
2. Verifica che l'account possa connettersi dalla macchina DumbMonit: un pattern host di localhost accetterebbe solo connessioni fatte sul server stesso.
3. Nell'indirizzo, scrivi il server, e la porta se non è 3306: "db.home.lan" oppure "db.home.lan:3307".
4. La query predefinita è SELECT 1, che non legge alcuna tabella: nulla altro deve essere concesso. Il tempo di connessione e il tempo di query vengono registrati separatamente.
5. Per osservare qualcosa tuo, sostituisci la query con una che restituisce un singolo numero, e diventa un grafico: una lunghezza di coda, un conteggio di righe, un ritardo di replicazione.

!!! warning
    Il controllo apre una connessione reale a ogni sondaggio. Su
    un'istanza già vicina al suo limite di connessioni, dagli un
    intervallo più lungo.

### Opzioni

| Chiave | Etichetta | Predefinito | Aiuto |
|---|---|---|---|
| `port` | Port | `3306` | Usata se l'indirizzo non ne fornisce una. |
| `database` | Database | *(vuoto)* | Database aperto alla connessione. Vuoto: nessuno, che è sufficiente per la query predefinita. |
| `query` | Query | `SELECT 1` | Eseguita a ogni controllo. La predefinita non legge alcuna tabella, così l'account non ha bisogno di alcun privilegio oltre alla connessione. Una query che restituisce un numero la trasforma in un grafico. |
| `expect` | Expected value | *(vuoto)* | Valore che la prima colonna della prima riga deve avere. Vuoto: viene controllato solo il successo della query. |
| `sslmode` | Encryption | `prefer` | `prefer` cifra quando il server lo offre, `require` rifiuta di connettersi senza, `verify-full` controlla anche il certificato e il nome, `disable` non cifra mai. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Vedi "Indirizzi che i controlli rifiutano" sotto. |
| `timeout_seconds` | Timeout (seconds) | `5` | Tempo dopo il quale il servizio viene riportato inattivo se non ha risposto. Tra 1 e 60. |

### Motivi di fallimento, entrambi i motori

| `reason` | Cosa è successo |
|---|---|
| `connect` | La porta è chiusa, il server è irraggiungibile, oppure ha rifiutato la connessione per un motivo diverso dall'account (in avvio, troppe connessioni). |
| `auth` | L'account è stato rifiutato: password errata, nessuna riga `pg_hba.conf`, pattern host che non copre la macchina DumbMonit. |
| `query` | Connesso e autenticato, ma la query è fallita: tabella mancante, database in sola lettura, errore di sintassi. |
| `payload` | La query ha funzionato ma ha restituito qualcosa diverso dal valore atteso. |
| `timeout` | La connessione o la query non sono terminate in tempo — il caso "attivo ma lento". |

## MQTT

**Broker MQTT** — si connette al broker, si iscrive a un topic, e può
attendere un messaggio retained. Esempi: Mosquitto, il broker dietro
Home Assistant, Zigbee2MQTT, sensori ESPHome.

### Configurazione

1. Nell'indirizzo, scrivi il broker, e la porta se non è quella usuale: "broker.home.lan" oppure "broker.home.lan:1884".
2. Spunta "Encrypted connection" per un broker che ascolta sulla 8883, e compila la credenziale se richiede un account.
3. Connettersi vale già la pena di essere monitorato: un broker che rifiuta il tuo account, oppure che ha smesso di rispondere, è il motivo per cui tutta la casa è diventata silenziosa.
4. Per andare oltre, nomina un topic: il controllo allora si iscrive ad esso, e un broker che rifiuta l'iscrizione ti dice che l'account ha perso i suoi diritti di accesso.
5. Spunta "Expect a retained message" solo per un topic che ne porta uno, tipicamente un topic di disponibilità. Un topic pubblicato di tanto in tanto appare silenzioso a un client che si è appena connesso.

!!! warning
    Il controllo non pubblica mai nulla. Non può nemmeno dire quanto
    vecchio è un messaggio retained: MQTT non lo data, così un valore
    congelato tre settimane fa conta ancora come presente.

### Cosa non può rilevare

Che un dato sensore ha smesso di emettere. Il messaggio retained che il
controllo legge è qualunque cosa il broker abbia conservato,
possibilmente vecchia di tre settimane — MQTT non lo marca
temporalmente. Per la freschezza, osserva l'automazione che lo
consuma, oppure usa un heartbeat.

### Opzioni

| Chiave | Etichetta | Predefinito | Aiuto |
|---|---|---|---|
| `tls` | Encrypted connection | `false` | Cifra la sessione dal primo byte, come fanno i broker sulla porta 8883. |
| `port` | Port | *(vuoto)* | Usata se l'indirizzo non ne fornisce una. Vuoto: 1883 in chiaro, 8883 cifrata. |
| `topic` | Topic | *(vuoto)* | Topic a cui il controllo si iscrive. Vuoto: si connette solamente, il che già dice che il broker è vivo e accetta il tuo account. |
| `expect_message` | Expect a retained message | `false` | Un messaggio deve arrivare su quel topic prima del timeout. Solo i messaggi retained arrivano subito: un topic che viene semplicemente pubblicato di tanto in tanto apparirà silenzioso. |
| `expect` | Expected content | *(vuoto)* | Testo che quel messaggio deve contenere. Compilarlo implica aspettarsi un messaggio. |
| `client_id` | Client identifier | *(derivato)* | Nome annunciato al broker. Il predefinito deriva dal dispositivo, così due controlli non si disconnettono mai a vicenda. |
| `server_name` | Server name (SNI) | *(l'host)* | Nome di dominio annunciato al broker e verificato nel certificato. Impostalo quando l'indirizzo è un IP. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | Una catena non verificabile non conta più come un fallimento: la data di scadenza viene comunque registrata. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Vedi "Indirizzi che i controlli rifiutano" sotto. |
| `timeout_seconds` | Timeout (seconds) | `5` | Tempo dopo il quale il servizio viene riportato inattivo se non ha risposto. Tra 1 e 60. |

### Motivi di fallimento

| `reason` | Cosa è successo |
|---|---|
| `connect` | La porta è chiusa o il broker è irraggiungibile. |
| `auth` | Il `CONNACK` ha rifiutato l'account, oppure il broker ha rifiutato l'iscrizione — una lista di controllo accessi non copre più questo topic. |
| `protocol` | Qualcosa ha risposto ma non parla MQTT 3.1.1, oppure il broker ha respinto l'identificatore client. |
| `payload` | Nessun messaggio retained è arrivato sul topic, oppure il suo contenuto non contiene il testo atteso. |

## WebSocket

**Endpoint WebSocket** — esegue l'handshake di upgrade, e può inviare
un frame e attenderne uno: un fallimento diverso da una semplice pagina
HTTP. Esempi: l'API di Home Assistant, una dashboard live, uno stream
di log, un endpoint dietro un reverse proxy.

### Configurazione

1. Nell'indirizzo, incolla l'endpoint completo: "wss://home.example.com/api/websocket". Senza uno schema, wss viene assunto.
2. Il controllo esegue l'intero handshake di apertura e verifica la risposta che il server calcola dalla chiave che gli è stata inviata: un reverse proxy che ha perso l'header Upgrade viene colto qui, dove un controllo HTTP vedrebbe ancora un sano 200.
3. Nulla altro è necessario per la maggior parte degli endpoint. Uno stato diverso da 101 viene riportato con il suo codice, così un 401 da un token scaduto non viene confuso con un'interruzione.
4. Per andare oltre, compila "Expected content" con un frammento del primo frame che il server invia da solo, per esempio il saluto dell'API di Home Assistant.
5. Se l'endpoint non dice nulla finché non gli viene parlato, metti un messaggio in "Frame to send" e cosa la sua risposta deve contenere in "Expected content".

!!! warning
    Il controllo apre e chiude entro un secondo. Una connessione che un
    firewall o un timeout di proxy taglia dopo trenta secondi appare
    perfettamente sana qui.

Credenziali: nessuna, un nome utente e password (inviati come
autenticazione HTTP basic nella richiesta di apertura) oppure un token
(inviato come `Authorization: Bearer`).

### Opzioni

| Chiave | Etichetta | Predefinito | Aiuto |
|---|---|---|---|
| `path` | Path | *(dall'indirizzo)* | Percorso della richiesta di apertura. Preso dall'indirizzo quando lo porta. |
| `port` | Port | *(vuoto)* | Usata se l'indirizzo non ne fornisce una. Vuoto: 443 per wss, 80 per ws. |
| `send` | Frame to send | *(vuoto)* | Frame di testo inviato una volta aperta la connessione. Vuoto: nulla viene inviato. |
| `expect` | Expected content | *(vuoto)* | Testo che un frame ricevuto deve contenere. Vuoto e senza nulla da inviare, viene controllato solo l'handshake di apertura. |
| `subprotocol` | Subprotocol | *(vuoto)* | Valore di Sec-WebSocket-Protocol. Il controllo fallisce se il server scegli un altro. |
| `origin` | Origin | *(vuoto)* | Valore dell'header Origin, che alcuni server richiedono prima di fare l'upgrade. |
| `server_name` | Server name (SNI) | *(l'host)* | Nome di dominio annunciato al server e verificato nel certificato. Impostalo quando l'indirizzo è un IP. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | Una catena non verificabile non conta più come un fallimento: la data di scadenza viene comunque registrata. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Vedi "Indirizzi che i controlli rifiutano" sotto. |
| `timeout_seconds` | Timeout (seconds) | `5` | Tempo dopo il quale il servizio viene riportato inattivo se non ha risposto. Tra 1 e 60. |

### Motivi di fallimento

| `reason` | Cosa è successo |
|---|---|
| `connect` | La porta è chiusa o l'host è irraggiungibile. |
| `status` | L'endpoint ha risposto qualcosa diverso da `101`; `probe_http_status_code` porta il codice. |
| `protocol` | Ha risposto `101` ma il suo `Sec-WebSocket-Accept` non corrisponde alla chiave inviata, ha scelto un altro subprotocollo, oppure ha chiuso la connessione all'istante. |
| `payload` | Nessun frame è arrivato, oppure non contiene il testo atteso. |

## NTP

**Server orario NTP** — invia una singola richiesta SNTP e legge la
risposta: offset, ritardo di andata-ritorno, stratum e indicatore
leap. Esempi: un router o NAS che funge da server orario locale, un
server NTP interno (chrony, ntpd), un server di pool pubblico usato
come riferimento.

### Configurazione

1. Nell'indirizzo, scrivi il nome o indirizzo IP del server: "pool.ntp.org" oppure "192.168.1.1". La porta UDP 123 viene usata a meno che l'opzione "Port" non dica diversamente.
2. Il controllo invia una singola richiesta SNTP e legge la risposta: nessuno stato viene mantenuto tra i sondaggi, e nulla viene scritto sul server.
3. Registra l'offset tra l'orologio del server e l'host DumbMonit, il ritardo di andata-ritorno, lo stratum e l'indicatore leap.
4. Un server che risponde stratum 16, oppure il cui indicatore leap legge "unsynchronized", non è mai riuscito a sincronizzarsi: il controllo lo riporta inattivo qualunque sia l'offset.
5. Abbassa "Tolerated offset" per essere avvisato prima di 100 ms; alzalo per un server diversi salti lontano da un orologio di riferimento.

!!! warning
    L'offset è buono solo quanto lo è l'orologio proprio dell'host
    DumbMonit: se quell'host non è esso stesso mantenuto in
    sincronizzazione (NTP, la sincronizzazione orario di un
    hypervisor), l'offset misurato riflette la sua deriva tanto quanto
    quella del server.

Credenziali: nessuna.

### Opzioni

| Chiave | Etichetta | Predefinito | Aiuto |
|---|---|---|---|
| `port` | Port | `123` | Porta UDP del server NTP. |
| `offset_threshold_ms` | Tolerated offset (ms) | `100` | Sopra questo offset dell'orologio, il server viene riportato inattivo. Un server non sincronizzato (stratum 16) fallisce sempre, qualunque sia questo valore. |
| `allow_private_targets` | Allow loopback and link-local targets | `false` | Vedi "Indirizzi che i controlli rifiutano" sotto. |
| `timeout_seconds` | Timeout (seconds) | `5` | Tempo dopo il quale il servizio viene riportato inattivo se non ha risposto. Tra 1 e 60. |

### Motivi di fallimento

| `reason` | Cosa è successo |
|---|---|
| `connect` | Il socket UDP non ha potuto essere aperto, oppure la risposta è arrivata da un indirizzo inatteso. |
| `timeout` | Nessuna risposta è arrivata entro il timeout. |
| `protocol` | La risposta non era un pacchetto NTP valido, oppure non proveniva da un server (modo errato). |
| `unsynchronized` | Il server ha risposto stratum 16 o 0 (un "kiss-o'-death", che nomina anche il motivo nel log), oppure il suo indicatore leap legge "unsynchronized". |
| `clock_offset` | L'offset misurato supera "Tolerated offset". |

## Indirizzi che i controlli rifiutano

I controlli che si connettono a un indirizzo che fornisci — HTTP, TCP,
TLS, NTP, SMTP, PostgreSQL, MySQL, MQTT, WebSocket — riportano ciò che
vedono — che è anche, parola per parola, ciò che fa una server-side
request forgery. Per impedire a un admin di monitoraggio di leggere
servizi che solo l'host DumbMonit può raggiungere (il database di
metriche integrato su `127.0.0.1:8428`, il servizio di metadata di un
cloud provider su `169.254.169.254`), i controlli rifiutano per
impostazione predefinita:

- indirizzi loopback (`127.0.0.0/8`, `::1`);
- indirizzi link-local (`169.254.0.0/16`, `fe80::/10`), incluso
  l'indirizzo di metadata cloud, e l'indirizzo non specificato
  (`0.0.0.0`, `::`).

Gli intervalli di LAN privata (`10.0.0.0/8`, `172.16.0.0/12`,
`192.168.0.0/16`, `fc00::/7`) sono sempre consentiti: monitorarli è lo
scopo di un homelab.

La regola si applica all'indirizzo che hai digitato, a ogni indirizzo
a cui un nome host risolve (verificato di nuovo al momento della
connessione, così un nome che cambia risposta non può passare
inosservato), e a ogni redirect che il controllo HTTP segue. Un
indirizzo rifiutato appare come un errore di configurazione sul
target — nessun avviso down viene inviato — nominando l'opzione da
abilitare: `allow_private_targets`. Attivala per un target che davvero
monitora un servizio sull'host DumbMonit stesso.

Un redirect verso un indirizzo rifiutato viene riportato come una
misurazione fallita (`reason="connect"`), perché è il servizio
monitorato che è cambiato, non la configurazione.

I valori trovati a un `json_path` non vengono mai riportati oltre 32
caratteri in un messaggio di fallimento.

## Errori comuni

| Sintomo | Causa probabile |
|---|---|
| Ping mostra un errore di configurazione | Il sysctl `net.ipv4.ping_group_range` manca dal file Compose. Vedi sopra. |
| HTTP inattivo con `reason="keyword"` | La parola chiave viene cercata solo nei primi `max_body_bytes`, senza distinguere maiuscole/minuscole a meno che `keyword_case_sensitive` non sia impostato. |
| HTTP inattivo con `reason="tls"` | Autofirmato o autorità privata: imposta `insecure_tls`. |
| Un controllo mostra un errore di configurazione che nomina `allow_private_targets` | L'indirizzo è loopback o link-local. Vedi "Indirizzi che i controlli rifiutano" sopra. |
| TCP inattivo anche se `telnet host porta` funziona dal tuo laptop | Il container DumbMonit non può raggiungere l'host (rete Docker, firewall). Testa dall'interno della rete del container. |
| DNS inattivo con `reason="record"` | La risposta non contiene ogni valore elencato in `expect`. |
| Ping inattivo con `reason="packet_loss"` | Perdita sopra `loss_threshold_percent`; alcuni dispositivi limitano il tasso di ICMP. |
| DNS inattivo con `reason="record"` anche se `expect` corrisponde | `expect_mode` è `exact` e la risposta contiene un valore extra, oppure `forbid` ha corrisposto. |
| SMTP inattivo con `reason="protocol"` su `security = starttls` | Il relay non pubblicizza STARTTLS su questa porta. Usa `tls` per 465, `none` per un relay locale semplice. |
| Un controllo database inattivo con `reason="auth"` | L'account esiste ma potrebbe non connettersi dalla macchina DumbMonit: controlla `pg_hba.conf`, oppure la parte host dell'utente MySQL. |
| Un controllo database inattivo con `reason="query"` | Connessione e login hanno funzionato. Il database è attivo e rifiuta di lavorare: sola lettura, tabella mancante, recovery. |
| MQTT inattivo con `reason="payload"` su un topic che chiaramente ha traffico | Solo i messaggi *retained* raggiungono un client appena iscritto. Pubblica quel topic con il flag retain, oppure deseleziona "Expect a retained message". |
| WebSocket inattivo con `reason="protocol"` dopo un `101` | Qualcosa sul percorso risponde all'upgrade senza parlare WebSocket — di solito una regola di reverse proxy che inghiotte l'header `Upgrade`. |
