# Servizi: HTTP, TCP, DNS, ping, TLS, NTP, SMTP, SQL, MQTT, WebSocket

Undici monitor osservano **servizi** piuttosto che attrezzature, nello stile di Uptime Kuma: una
pagina web, una porta, un nome DNS, un host, un certificato, un server orario, un
relè di posta, un database, un broker MQTT, un endpoint WebSocket. Ogni servizio ottiene una
barra della cronologia, un tempo di risposta e una percentuale di disponibilità sulla sua pagina
del dispositivo.

Gli ultimi cinque vanno oltre l'apertura di una connessione: iniziano una vera sessione nel
protocollo proprio del servizio. Questa è la differenza tra "la porta 5432 è aperta"
e "il database ancora accetta il mio account e risponde a una query". NTP si situa
tra i due: una singola richiesta e risposta, nel protocollo NTP proprio, abbastanza per
leggere l'orologio del server senza una sessione.

## Come funziona lo stato del servizio

Un servizio può rispondere e comunque essere giù: un `500`, una parola chiave mancante, un certificato
scaduto. Quindi questi monitor scrivono un punto ad ogni controllo, `probe_success`
essendo 1 o 0, invece di rimanere silenziosi come un dispositivo hardware. Questo è quello che
rende possibile una percentuale di disponibilità: `avg_over_time(dumbmonit_probe_success[30d])`.

| Situazione | Cosa accade |
|---|---|
| Opzione o indirizzo non valido, socket ICMP non consentiti (vedi Ping) | Errore di configurazione: niente viene scritto, la pagina del dispositivo mostra l'errore, nessun avviso "giù". |
| Connessione rifiutata, timeout, TLS rifiutato, `500`, parola chiave assente, perdita totale di pacchetti, password rifiutata, query non riuscita | `probe_success = 0`: l'interruzione viene registrata, datata e contata. |
| Tutto bene | `probe_success = 1`. |

Ogni monitor ha il suo timeout (`timeout_seconds`, 5 s per impostazione predefinita, 60 s al
massimo), più breve del `DUMBMONIT_PROBE_TIMEOUT_SECS` del server: interrotto da
lo scheduler, non potrebbe scrivere il suo zero.

Le regole incorporate che si applicano a tutti e undici: Servizio giù (3 minuti), Servizio
vacillante (più di sei cambi di stato in trenta minuti), Servizio lento (più
di 3 s per dieci minuti). Per ogni controllo che legge un certificato — `http`,
`tls`, `smtp`, `mqtt`, `websocket`: Certificato in scadenza a breve (14 giorni) e
Certificato scaduto.

Una password rifiutata non è mai un'interruzione del servizio: viene registrata come
`reason="auth"`, quindi l'avviso dice cosa riparare.

### Metriche

Tutti gauge, tutti con prefisso `dumbmonit_`, tutti etichettati
`probe="http|tcp|dns|ping|tls|smtp|postgres|mysql|mqtt|websocket|ntp"` in aggiunta
a `target`, `host` e `tag_*`.

| Metrica | Monitor | Significato |
|---|---|---|
| `probe_success` | tutti | 1 se il servizio risponde correttamente, 0 altrimenti. |
| `probe_duration_seconds` | tutti | Durata totale del controllo. |
| `probe_failure_info` | tutti | Presenza (1) con un'etichetta `reason`: `dns`, `connect`, `timeout`, `tls`, `cert_expired`, `status`, `keyword`, `json`, `body`, `packet_loss`, `record`, `auth`, `protocol`, `query`, `payload`. |
| `probe_http_status_code` | http, websocket | Codice di stato ottenuto. |
| `probe_http_first_byte_seconds` | http | Tempo alle intestazioni di risposta. |
| `probe_http_content_bytes` | http | Dimensione del corpo. |
| `probe_http_security_header{header}` | http | 0/1 per etichetta `header` (solo https): HSTS, nosniff, CSP, X-Frame-Options, frame-ancestors. |
| `probe_connect_seconds` | tutti tranne dns e ping | Stabilimento della connessione (login incluso per i database). |
| `probe_tls_handshake_seconds` | tls, http, smtp, mqtt, websocket | Negoziazione TLS sola. |
| `probe_tls_version_info` | tls, http, smtp, mqtt, websocket | Presenza; etichetta `version`. |
| `probe_ssl_cert_expiry_days` | tls, http, smtp, mqtt, websocket | Giorni prima della scadenza, negativo se scaduto. |
| `probe_ssl_cert_valid` | tls, http, smtp, mqtt, websocket | 1 se la catena conduce a un'autorità nota. |
| `probe_ssl_cert_issuer_info` | tls, http, smtp, mqtt, websocket | Presenza; etichetta `issuer`. |
| `probe_dns_lookup_seconds` | dns | Tempo di risoluzione. |
| `probe_dns_answer_records` | dns | Record del tipo richiesto. |
| `probe_dns_answer_fingerprint` | dns | Con "Avvisa quando la risposta cambia": un'impronta di 32 bit della risposta ordinata. Il suo valore non significa nulla; solo i suoi cambiamenti lo fanno. |
| `probe_icmp_rtt_seconds`, `_min_seconds`, `_max_seconds` | ping | Roundtrip medio, più breve e più lungo. |
| `probe_icmp_packet_loss_ratio` | ping | Perdita tra 0 e 1. |
| `probe_icmp_packets_sent`, `probe_icmp_packets_received` | ping | Echo inviati e ricevuti. |
| `probe_smtp_greeting_seconds` | smtp | Tempo al banner `220`. |
| `probe_smtp_ehlo_seconds` | smtp | Roundtrip del comando `EHLO`. |
| `probe_smtp_capabilities` | smtp | Estensioni pubblicizzate. |
| `probe_smtp_authenticated` | smtp | 1 quando `AUTH` ha avuto successo. |
| `probe_sql_query_seconds` | postgres, mysql | Esecuzione della query sola. |
| `probe_sql_rows` | postgres, mysql | Righe restituite. |
| `probe_sql_value` | postgres, mysql | Prima colonna della prima riga, quando è un numero. |
| `probe_mqtt_connack_seconds` | mqtt | Tempo al `CONNACK`. |
| `probe_mqtt_suback_seconds` | mqtt | Tempo al `SUBACK`. |
| `probe_mqtt_message_bytes` | mqtt | Dimensione del messaggio conservato. |
| `probe_mqtt_message_value` | mqtt | Il suo contenuto, quando è un numero. |
| `probe_ws_handshake_seconds` | websocket | Handshake di apertura. |
| `probe_ntp_offset_seconds` | ntp | Offset tra l'orologio locale e il server. |
| `probe_ntp_delay_seconds` | ntp | Delay di roundtrip di rete. |
| `probe_ntp_stratum` | ntp | Distanza da un orologio di riferimento (16: non sincronizzato). |
| `probe_ntp_leap_indicator` | ntp | Da 0 a 3 (avvertimento di secondo di salto, 3 = orologio non sincronizzato). |
| `probe_ntp_root_delay_seconds` | ntp | Delay cumulativo fino alla radice della gerarchia. |
| `probe_ntp_root_dispersion_seconds` | ntp | Dispersione cumulativa fino alla radice. |
| `probe_ntp_reference_info` | ntp | Presenza; etichetta `reference_id`. |
| `probe_ws_message_bytes` | websocket | Dimensione del primo frame ricevuto. Presente solo quando il controllo invia un messaggio o aspetta una risposta (opzioni `send` / `expect`). |

## HTTP

**Sito web o API web (HTTP)** — controlla che una pagina o un'API risponda, con il
codice di stato giusto, il contenuto giusto e un certificato valido. Esempi: un
sito web, una pagina di salute dell'applicazione, un'API REST, un'interfaccia di servizio auto-ospitato.

### Configurazione

1. Nell'indirizzo, incolla l'URL completo della pagina da monitorare. Senza
   "http://" o "https://", si assume HTTPS.
2. Preferisci una pagina leggera che non ha bisogno di login, ad esempio la pagina di
   salute dell'applicazione ("/health", "/status"), piuttosto che la home page.
3. Se la pagina richiede autenticazione, compila la credenziale: un nome utente e
   una password forniscono autenticazione di base, un token viene inviato come "Bearer".
4. Per impostazione predefinita, qualsiasi codice di stato tra 200 e 299 va bene. Per andare oltre,
   richiedi una parola chiave nella pagina o un valore specifico in una risposta JSON.
5. Su HTTPS, il certificato viene letto automaticamente: sarai avvisato prima
   che scada.

!!! warning
    Non scrivere mai una password o un token nelle opzioni: vengono copiati in chiaro
    ad ogni misurazione. Usa il campo credenziale, che è crittografato.

Credenziali: nessuna, nome utente / password (autenticazione di base) o token API
(`Authorization: Bearer`).

### Opzioni

| Chiave | Etichetta | Predefinito | Aiuto |
|---|---|---|---|
| `method` | Metodo HTTP | `GET` | GET si adatta quasi ogni caso. HEAD evita il download della pagina quando conta solo il codice di stato. Scelte: GET, HEAD, POST, PUT, PATCH, DELETE, OPTIONS. |
| `accepted_status` | Codici di stato accettati | `200-299` | Codici o intervalli considerati normali, separati da virgole. Qualsiasi altro codice conta come giù. Esempio: `200-299,301,404`. |
| `keyword` | Parola chiave prevista | *(empty)* | Testo che deve apparire nella pagina. Se manca, il servizio viene segnalato come giù anche se la pagina risponde. |
| `keyword_absent` | La parola chiave deve essere assente | `false` | Inverte il controllo: la presenza della parola chiave segnala un fallimento. Utile per una pagina di errore che risponde 200. |
| `keyword_case_sensitive` | Abbina il caso della parola chiave | `false` | Per impostazione predefinita, maiuscole e minuscole vengono trattate allo stesso modo. |
| `json_path` | Percorso JSON da controllare | *(empty)* | Per una risposta JSON: percorso del valore da controllare. Compilalo insieme al valore previsto, mai uno senza l'altro. Esempio: `$.status`. |
| `json_expect` | Valore JSON previsto | *(empty)* | Valore che il percorso JSON sopra deve avere, ad esempio "ok" o "true". |
| `headers` | Intestazioni extra | *(empty)* | Una o più intestazioni "Name: value", separate da "\|". Non mettere segreti qui: le opzioni sono visibili nei grafici. |
| `body` | Corpo della richiesta | *(empty)* | Contenuto inviato con la richiesta, per metodi che ne aspettano uno (POST, PUT…). |
| `follow_redirects` | Segui i reindirizzamenti | `true` | Disabilita per monitorare il reindirizzamento stesso, ad esempio un 301 a HTTPS. |
| `max_redirects` | Massimi reindirizzamenti seguiti | `10` | Tra 1 e 20. Nessun effetto se i reindirizzamenti non vengono seguiti. |
| `insecure_tls` | Accetta un certificato non verificabile | `false` | Il controllo non fallisce più su un certificato autofirmato o emesso da un'autorità privata. |
| `allow_private_targets` | Consenti destinazioni loopback e link-local | `false` | Per impostazione predefinita il controllo rifiuta gli indirizzi che solo l'host DumbMonit stesso può raggiungere (127.0.0.1, ::1, 169.254.x.x). Abilita questo per monitorare un servizio in esecuzione sull'host DumbMonit. Gli indirizzi LAN privati (10.x, 192.168.x) sono sempre consentiti. |
| `check_certificate` | Leggi il certificato | `true` | Su HTTPS, registra anche la data di scadenza del certificato in modo da essere avvisato prima che scada. |
| `max_body_bytes` | Massimi byte letti | `524288` | Oltre questo, il resto della pagina non viene scaricato. La parola chiave e il percorso JSON vengono cercati solo in questa parte. |
| `user_agent` | Identità annunciata (User-Agent) | `DumbMonit/<version>` | Nome che il controllo dà al server. Cambialo se il server filtra i robot. |
| `timeout_seconds` | Timeout (secondi) | `5` | Tempo dopo il quale il servizio viene segnalato come giù se non ha risposto. Tra 1 e 60. |

## TCP

**Porta di rete (TCP)** — controlla che una porta accetti connessioni: SSH, database,
condivisione di file, server di gioco, stampante di rete.

### Configurazione

1. Nell'indirizzo, scrivi l'host seguito dalla porta, separati da due punti:
   "nas.home.lan:22" o "192.168.1.5:445".
2. Per un indirizzo IPv6, mettilo tra parentesi: "[fd00::1]:445".
3. Puoi anche lasciare l'indirizzo senza una porta e inserirlo nell'opzione "Porta": uno dei due è obbligatorio.
4. Solo l'apertura della connessione viene testata: nessun dato viene inviato al servizio, quindi
   non ha effetto su di esso.

!!! warning
    Una porta aperta non prova che l'applicazione dietro di essa funziona. Per un
    servizio web, preferisci il tipo "Sito web o API web", che legge la risposta.

### Opzioni

| Chiave | Etichetta | Predefinito | Aiuto |
|---|---|---|---|
| `port` | Porta | *(empty)* | Porta da aprire, se l'indirizzo non la dà già come "host:port". Uno dei due è obbligatorio. |
| `allow_private_targets` | Consenti destinazioni loopback e link-local | `false` | Per impostazione predefinita il controllo rifiuta gli indirizzi che solo l'host DumbMonit stesso può raggiungere (127.0.0.1, ::1, 169.254.x.x). Abilita questo per monitorare un servizio in esecuzione sull'host DumbMonit. Gli indirizzi LAN privati (10.x, 192.168.x) sono sempre consentiti. |
| `timeout_seconds` | Timeout (secondi) | `5` | Tempo dopo il quale il servizio viene segnalato come giù se non ha risposto. Tra 1 e 60. |

## DNS

**Nome di dominio (DNS)** — controlla che un nome si risolva e che punti a
l'indirizzo giusto. Esempi: il tuo nome di dominio, un nome interno servito dal tuo
Pi-hole o AdGuard, un record MX.

### Configurazione

1. Nell'indirizzo, scrivi il nome da risolvere, senza "http://":
   "www.example.com".
2. Per impostazione predefinita, il controllo chiede al risolutore di sistema un indirizzo IPv4 (A
   record). Scegli un altro tipo di record nelle opzioni se necessario.
3. Per monitorare il tuo server DNS, inserisci il suo indirizzo IP in "Server DNS da
   interrogare": il controllo fallirà se smette di rispondere.
4. Per rilevare il sequestro o una errata configurazione, elenca in "Valori previsti" gli
   indirizzi che la risposta deve contenere.

!!! warning
    Il server DNS è dato dal suo indirizzo IP, non dal nome: ci vorrebbe un
    risolutore per risolvere il risolutore.

### Opzioni

| Chiave | Etichetta | Predefinito | Aiuto |
|---|---|---|---|
| `record_type` | Tipo di record | `A` | A per un indirizzo IPv4, AAAA per IPv6, MX per posta, CNAME per un alias… Scelte: A, AAAA, CNAME, MX, TXT, NS, SOA, SRV, PTR, CAA. |
| `resolver` | Server DNS da interrogare | *(empty)* | Indirizzo IP di un risolutore, porta opzionale. Vuoto: il risolutore di sistema. Comodo per controllare il tuo server DNS. Esempio: `1.1.1.1` o `10.0.0.1:5353`. |
| `expect` | Valori previsti | *(empty)* | Valori che devono tutti apparire nella risposta, separati da virgole. Vuoto: solo la risoluzione viene controllata. |
| `expect_mode` | Confronto | `contains` | `contains`: ogni valore previsto deve apparire da qualche parte nella risposta. `exact`: la risposta deve contenere quei valori e nient'altro. |
| `forbid` | Valori vietati | *(empty)* | Valori che non devono mai apparire nella risposta, separati da virgole. Controllato prima di quelli previsti. |
| `alert_on_change` | Avvisa quando la risposta cambia | `false` | Registra un'impronta della risposta ordinata e avvisa quando cambia, senza dover scrivere i valori previsti. |
| `timeout_seconds` | Timeout (secondi) | `5` | Tempo dopo il quale il servizio viene segnalato come giù se non ha risposto. Tra 1 e 60. |

### Asserire ciò che una zona risponde

`expect` da solo rileva il record che scomparve. Non rileva il record
che è stato *aggiunto*: una zona sequestrata spesso mantiene l'A legittimo e mette
un secondo accanto ad esso, e un client allora raggiunge uno o l'altro a caso.

- `expect_mode = exact` rifiuta qualsiasi valore che l'aspettativa non copre. L'ordine
  non conta — DNS non mantiene uno.
- `forbid` nomina valori che non devono mai tornare: l'indirizzo di un host precedente,
  una convalida `TXT` scaduta, un server di posta che hai dismesso. Viene controllato
  per primo, quindi vince su `expect`.

Entrambi confrontano senza distinzione tra maiuscole e minuscole, ignorano il punto finale di un nome e le
virgolette intorno a un `TXT`, e trattano un valore previsto come trovato quando appare
all'interno di un record — quindi un `MX` corrisponde a `mail.example.com` senza che tu debba
copiare la sua priorità.

### Avvisare quando la risposta cambia {#alerting-when-the-answer-changes}

Scrivere i valori previsti non è sempre pratico: un nome servito da una CDN,
record che qualcun altro gestisce, o semplicemente una zona di cui vuoi sentire quando
si muove. Abilita **Avvisa quando la risposta cambia**: ogni controllo registra allora un'
impronta della risposta, e la regola incorporata
[DNS answer changed](../alerting/rules.md#reverse-proxies-and-domains) si attiva
quando differisce da quello precedente. L'avviso si risolve da solo un'ora
dopo; la nuova risposta è il riferimento da allora.

L'impronta copre i valori della risposta, ordinati e senza duplicati,
con la catena CNAME inclusa: un round-robin che restituisce gli stessi indirizzi
in un altro ordine non lo cambia, neppure un TTL. Un nome la cui risposta
cambia ad ogni query (un grande pool come `pool.ntp.org`, o DNS geografico
che risponde diversamente da un risolutore all'altro) avvertirebbe tutto il
tempo: lascia l'opzione spenta per quelli, o interroga un risolutore fisso.

## Ping

**Host raggiungibile (ping)** — invia alcuni ICMP echo e misura il tempo di risposta
e la perdita di pacchetti. Esempi: router domestico o gateway, punto di accesso Wi-Fi, stampante,
macchina senza agente o SNMP, host remoto.

### Configurazione

1. Nell'indirizzo, scrivi il nome host o l'indirizzo IP: "192.168.1.1" o
   "router.home.lan".
2. Ad ogni sondaggio, il controllo invia quattro echo e misura il tempo di risposta,
   insieme alla quota di pacchetti persi.
3. Per impostazione predefinita, solo una perdita totale conta come giù; la perdita parziale rimane visibile nei
   grafici. Abbassa "Perdita tollerata" per essere avvisato prima.
4. Se DumbMonit esegue in Docker, mantieni le linee `sysctls:` del
   docker-compose.yml fornito sotto il servizio dumbmonit: consentono all'utente
   senza privilegi del contenitore di aprire socket ICMP echo. Nessuna capacità è necessaria.

!!! warning
    Senza quel sysctl, il controllo non può aprire un socket ICMP: lo segnala
    come errore di configurazione, non come fallimento host. Alcuni dispositivi anche ignorano
    i ping di proposito: controlla quello prima di trarre conclusioni.

```yaml
services:
  dumbmonit:
    sysctls:
      net.ipv4.ping_group_range: "0 2147483647"
```

Fuori Docker, `sysctl -w net.ipv4.ping_group_range="0 2147483647"` sull'
host fa lo stesso, e un socket raw (`NET_RAW`, root) funziona anche.

### Opzioni

| Chiave | Etichetta | Predefinito | Aiuto |
|---|---|---|---|
| `count` | Numero di echo | `4` | Pacchetti inviati ad ogni sondaggio, da 1 a 20. |
| `packet_timeout_ms` | Attesa per pacchetto (ms) | `1000` | Tempo consentito per ogni risposta, da 50 a 10.000 ms. |
| `interval_ms` | Spazio tra due echo (ms) | `100` | Pausa tra due echo, da 0 a 5.000 ms. |
| `payload_bytes` | Dimensione del carico (byte) | `56` | Dati trasportati da ogni echo, da 0 a 1.400 byte. |
| `ip_version` | Versione IP | `auto` | "auto" prende il primo indirizzo risolto; forza 4 o 6 se l'host ne ha entrambi e uno non risponde. Scelte: auto, 4, 6. |
| `loss_threshold_percent` | Perdita tollerata (%) | `100` | Al di sopra di questa percentuale di pacchetti persi, l'host viene segnalato come giù. 100: solo una perdita totale conta, la perdita parziale rimane visibile nei grafici. |
| `timeout_seconds` | Timeout (secondi) | `5` | Budget totale per il sondaggio, da 1 a 60. Rifiutato se non copre "numero di echo × attesa per pacchetto". |

## TLS

**Certificato TLS** — controlla che un certificato sia valido e avvisa prima che
scada, su qualsiasi porta crittografata. Esempi: server di posta (IMAPS, SMTPS), reverse
proxy, directory LDAPS, broker MQTT, interfaccia di amministrazione.

### Configurazione

1. Nell'indirizzo, scrivi l'host e, se non è 443, la porta del
   servizio crittografato: "mail.example.com:993", "ldap.home.lan:636".
2. Il controllo apre una connessione crittografata, legge il certificato presentato e
   chiude subito: nessun dato viene scambiato con l'applicazione.
3. Registra il numero di giorni prima della scadenza, l'emittente e la versione TLS.
   Le regole di avviso incluse avvertono a quattordici giorni, poi alla scadenza.
4. Se il servizio viene raggiunto dal suo indirizzo IP ma il certificato porta un
   nome, inserisci quel nome in "Nome del server (SNI)".
5. Per un certificato emesso dalla tua autorità, abilita "Accetta un certificato non verificabile":
   la data di scadenza viene comunque monitorata.

!!! warning
    Per un sito web, il tipo "Sito web o API web" già legge il certificato:
    questo tipo è inteso per servizi che non parlano HTTP, o la cui
    applicazione non vuoi toccare.

L'handshake completa anche quando il certificato viene rifiutato: è quello che
consente di leggere la data di scadenza di un certificato già scaduto. Il
verdetto della verifica è in `probe_ssl_cert_valid`.

### Opzioni

| Chiave | Etichetta | Predefinito | Aiuto |
|---|---|---|---|
| `server_name` | Nome del server (SNI) | *(empty)* | Nome di dominio annunciato al server e controllato nel certificato. Impostalo quando l'indirizzo è un IP dietro un reverse proxy. |
| `insecure_tls` | Accetta un certificato non verificabile | `false` | Una catena non verificabile non conta più come fallimento: la data di scadenza è ancora registrata. |
| `allow_private_targets` | Consenti destinazioni loopback e link-local | `false` | Per impostazione predefinita il controllo rifiuta gli indirizzi che solo l'host DumbMonit stesso può raggiungere (127.0.0.1, ::1, 169.254.x.x). Abilita questo per monitorare un servizio in esecuzione sull'host DumbMonit. Gli indirizzi LAN privati (10.x, 192.168.x) sono sempre consentiti. |
| `timeout_seconds` | Timeout (secondi) | `5` | Tempo dopo il quale il servizio viene segnalato come giù se non ha risposto. Tra 1 e 60. |

## SMTP

**Relè di posta (SMTP)** — apre una vera sessione su un server di posta: saluto, EHLO,
STARTTLS, e il login se ne dai uno. Esempi: il relè SMTP del tuo provider,
un locale Postfix o msmtp, Proxmox Mail Gateway, la porta di sottomissione di un
server di posta.

### Configurazione

1. Nell'indirizzo, scrivi il server di posta, e la porta se non è quella solita:
   "smtp.example.com" o "smtp.example.com:2525".
2. Scegli la crittografia che il tuo relè usa: STARTTLS per la porta di sottomissione 587,
   TLS per la porta 465, Nessuno per un relè sulla tua propria rete in ascolto sulla porta 25.
3. Per controllare che il relè accetti ancora il tuo account, compila la credenziale:
   il controllo allora esegue AUTH e segnala una password rifiutata come tale, non come un'
   interruzione.
4. Non viene mai inviato niente: il controllo si ferma dopo il saluto, le estensioni e
   il login opzionale, poi si interrompe. Nessun messaggio entra nella coda.
5. Per essere avvisato quando un'estensione scompare, nominala in "Estensione prevista":
   un relè che smette di pubblicizzare STARTTLS è un relè che invierebbe la tua posta
   in testo leggibile.

!!! warning
    Una sessione accettata non prova che la posta se ne vada. Un relè che accetta
    tutto e lo mette in coda per sempre risponde perfettamente qui: osserva le
    code stesse per quello.

Credenziali: nessuna, o un nome utente e una password inviati con `AUTH PLAIN` (o
`AUTH LOGIN` quando il server offre solo quello). Vengono inviati **dopo** che la
connessione è crittografata e dopo che il certificato è stato giudicato: un relè la cui
catena non può essere verificata non li riceve mai.

Su TLS o STARTTLS, il certificato viene letto come il controllo TLS lo legge —
scadenza, emittente, versione del protocollo — quindi un relè di posta è anche un certificato di cui sei
avvisato.

### Cosa non può rilevare

Che la posta se ne vada effettivamente. Un relè che accetta ogni sessione e poi nega
tutto risponde perfettamente qui; osserva le sue code (vedi la pagina Proxmox
Mail Gateway). Non invia mai un messaggio, quindi non dice nulla sul
filtraggio dei destinatari, SPF o DKIM.

### Opzioni

| Chiave | Etichetta | Predefinito | Aiuto |
|---|---|---|---|
| `security` | Crittografia | `starttls` | STARTTLS inizia in testo leggibile e aggiorna (porta 587), TLS crittografa dal primo byte (porta 465), Nessuno non crittografa mai (porta 25) e solo si adatta a un relè sulla tua rete. |
| `port` | Porta | *(empty)* | Usato se l'indirizzo non lo dà. Vuoto: 587 con STARTTLS, 465 con TLS, 25 senza crittografia. |
| `helo_name` | Nome annunciato (EHLO) | `dumbmonit` | Nome che il controllo dà quando si presenta. Un relè rigoroso rifiuta un nome che non può risolvere. |
| `expect_capability` | Estensione prevista | *(empty)* | Estensione che il server deve pubblicizzare nella sua risposta EHLO, ad esempio STARTTLS o AUTH. Vuoto: nessuna aspettativa. |
| `server_name` | Nome del server (SNI) | *(the host)* | Nome di dominio annunciato al server e controllato nel certificato. Impostalo quando l'indirizzo è un IP. |
| `insecure_tls` | Accetta un certificato non verificabile | `false` | Una catena non verificabile non conta più come fallimento: la data di scadenza è ancora registrata. |
| `allow_private_targets` | Consenti destinazioni loopback e link-local | `false` | Vedi "Indirizzi che i controlli rifiutano" sotto. |
| `timeout_seconds` | Timeout (secondi) | `5` | Tempo dopo il quale il servizio viene segnalato come giù se non ha risposto. Tra 1 e 60. |

### Motivi di fallimento

| `reason` | Cosa è successo |
|---|---|
| `connect` | La porta è chiusa o il relè è irraggiungibile. |
| `protocol` | Qualcosa ha risposto, ma non SMTP — o ha rifiutato la connessione, `EHLO`, o STARTTLS che aveva pubblicizzato. |
| `auth` | L'account è stato rifiutato (`535`, `530`, `534`). Il relè va bene; la credenziale non. |
| `payload` | L'estensione nominata in "Estensione prevista" non è pubblicizzata. |
| `tls`, `cert_expired` | Lo stesso significato del controllo TLS. |

## PostgreSQL

**Database PostgreSQL** — si connette, autentica ed esegue una query: il segnale
per un database che funziona ma non risponde più. Esempi: il database dietro
Nextcloud o Immich, un registratore Home Assistant, un database di applicazione, un'
istanza TimescaleDB.

### Configurazione

1. Sul server, crea un account per il monitoraggio e dagli niente più
   che il diritto di connettersi: CREATE ROLE dumbmonit LOGIN PASSWORD
   'a-long-password';
2. Assicurati che quell'account possa raggiungere il server dalla macchina DumbMonit,
   che solitamente significa una riga in più in pg_hba.conf seguita da un ricaricamento della
   configurazione.
3. Nell'indirizzo, scrivi il server, e la porta se non è 5432:
   "db.home.lan" o "db.home.lan:5433".
4. La query predefinita è SELECT 1, che non legge nessuna tabella: nient'altro ha
   bisogno di essere concesso. Il tempo di connessione e il tempo di query vengono registrati
   separatamente.
5. Per osservare qualcosa di tuo, sostituisci la query con una che restituisce un
   singolo numero, e diventa un grafico: una lunghezza di coda, un conteggio di righe, un
   lag di replica.

!!! warning
    Il controllo apre una vera connessione ad ogni sondaggio. Su un'istanza già
    vicino al suo limite di connessione, dagli un intervallo più lungo.

Credenziali: un nome utente e una password, obbligatori. Un database non è mai monitorato
anonimamente, quindi il modulo non offre nient'altro.

### Cosa non può rilevare

Lag di replica, un lock che si trascina, una tabella che continua a crescere. Tutto quello
vive nelle viste amministrative del motore — scrivi la query tu stesso, e
il numero che restituisce diventa `probe_sql_value`, un grafico su cui puoi avvisare.

### Opzioni

| Chiave | Etichetta | Predefinito | Aiuto |
|---|---|---|---|
| `port` | Porta | `5432` | Usato se l'indirizzo non lo dà. |
| `database` | Database | `postgres` | Database aperto alla connessione. L'account di monitoraggio deve essere autorizzato a connettersi. |
| `query` | Query | `SELECT 1` | Eseguita ad ogni controllo. Il predefinito non legge nessuna tabella, quindi l'account ha bisogno solo del privilegio di connettersi. Una query che restituisce un numero la trasforma in un grafico. |
| `expect` | Valore previsto | *(empty)* | Valore che la prima colonna della prima riga deve contenere. Vuoto: solo il successo della query viene controllato. |
| `sslmode` | Crittografia | `prefer` | `prefer` crittografa quando il server lo offre, `require` rifiuta di connettersi senza, `verify-full` controlla anche il certificato e il nome, `disable` non crittografa mai. |
| `allow_private_targets` | Consenti destinazioni loopback e link-local | `false` | Vedi "Indirizzi che i controlli rifiutano" sotto. |
| `timeout_seconds` | Timeout (secondi) | `5` | Tempo dopo il quale il servizio viene segnalato come giù se non ha risposto. Tra 1 e 60. |

## MySQL e MariaDB {#mysql-and-mariadb}

**Database MySQL o MariaDB** — si connette, autentica ed esegue una query: il
segnale per un database che funziona ma non risponde più. Esempi: il database
dietro WordPress, Nextcloud o Kimai, un database di applicazione, un MariaDB
in Docker.

### Configurazione

1. Sul server, crea un account per il monitoraggio senza nessun privilegio:
   CREATE USER 'dumbmonit'@'%' IDENTIFIED BY 'a-long-password';
2. Controlla che l'account possa connettersi dalla macchina DumbMonit: uno
   schema host di localhost accetterebbe solo le connessioni effettuate sul server
   stesso.
3. Nell'indirizzo, scrivi il server, e la porta se non è 3306:
   "db.home.lan" o "db.home.lan:3307".
4. La query predefinita è SELECT 1, che non legge nessuna tabella: nient'altro ha
   bisogno di essere concesso. Il tempo di connessione e il tempo di query vengono registrati
   separatamente.
5. Per osservare qualcosa di tuo, sostituisci la query con una che restituisce un
   singolo numero, e diventa un grafico: una lunghezza di coda, un conteggio di righe, un
   lag di replica.

!!! warning
    Il controllo apre una vera connessione ad ogni sondaggio. Su un'istanza già
    vicino al suo limite di connessione, dagli un intervallo più lungo.

### Opzioni

| Chiave | Etichetta | Predefinito | Aiuto |
|---|---|---|---|
| `port` | Porta | `3306` | Usato se l'indirizzo non lo dà. |
| `database` | Database | *(empty)* | Database aperto alla connessione. Vuoto: nessuno, il che è abbastanza per la query predefinita. |
| `query` | Query | `SELECT 1` | Eseguita ad ogni controllo. Il predefinito non legge nessuna tabella, quindi l'account ha bisogno solo del privilegio di connettersi. Una query che restituisce un numero la trasforma in un grafico. |
| `expect` | Valore previsto | *(empty)* | Valore che la prima colonna della prima riga deve contenere. Vuoto: solo il successo della query viene controllato. |
| `sslmode` | Crittografia | `prefer` | `prefer` crittografa quando il server lo offre, `require` rifiuta di connettersi senza, `verify-full` controlla anche il certificato e il nome, `disable` non crittografa mai. |
| `allow_private_targets` | Consenti destinazioni loopback e link-local | `false` | Vedi "Indirizzi che i controlli rifiutano" sotto. |
| `timeout_seconds` | Timeout (secondi) | `5` | Tempo dopo il quale il servizio viene segnalato come giù se non ha risposto. Tra 1 e 60. |

### Motivi di fallimento, entrambi i motori

| `reason` | Cosa è successo |
|---|---|
| `connect` | La porta è chiusa, il server è irraggiungibile, o ha rifiutato la connessione per un motivo diverso dall'account (avvio, troppe connessioni). |
| `auth` | L'account è stato rifiutato: password sbagliata, nessuna riga `pg_hba.conf`, schema host che non copre la macchina DumbMonit. |
| `query` | Connesso e autenticato, ma la query ha fallito: tabella mancante, database in sola lettura, errore di sintassi. |
| `payload` | La query ha funzionato ma ha restituito qualcosa di diverso dal valore previsto. |
| `timeout` | La connessione o la query non hanno finito in tempo — il caso "su ma lento". |

## MQTT

**Broker MQTT** — si connette al broker, si iscrive a un topic, e può aspettare
un messaggio conservato. Esempi: Mosquitto, il broker dietro Home Assistant,
Zigbee2MQTT, sensori ESPHome.

### Configurazione

1. Nell'indirizzo, scrivi il broker, e la porta se non è quella solita:
   "broker.home.lan" o "broker.home.lan:1884".
2. Abilita "Connessione crittografata" per un broker in ascolto su 8883, e compila la
   credenziale se richiede un account.
3. Connettersi è già degno di monitoraggio: un broker che rifiuta il tuo account,
   o che ha smesso di rispondere, è il motivo per cui la casa intera è rimasta silenziosa.
4. Per andare oltre, nomina un topic: il controllo allora si iscrive ad esso, e un broker
   che rifiuta l'iscrizione ti dice che l'account ha perso i suoi diritti di accesso.
5. Abilita "Aspetta un messaggio conservato" solo per un topic che ne porta uno, tipicamente
   un topic di disponibilità. Un topic pubblicato di tanto in tanto sembra silenzioso a un
   client che si è appena connesso.

!!! warning
    Il controllo non pubblica mai niente. Non può dire neanche quanto è vecchio un
    messaggio conservato: MQTT non li data, quindi un valore congelato tre settimane fa conta comunque
    come presente.

### Cosa non può rilevare

Che un dato sensore ha smesso di emettere. Il messaggio conservato che il controllo legge è
qualunque cosa il broker mantenne, possibilmente tre settimane fa — MQTT non lo
data. Per la freschezza, osserva l'automazione che lo consuma, o usa un heartbeat.

### Opzioni

| Chiave | Etichetta | Predefinito | Aiuto |
|---|---|---|---|
| `tls` | Connessione crittografata | `false` | Crittografa la sessione dal primo byte, come fanno i broker sulla porta 8883. |
| `port` | Porta | *(empty)* | Usato se l'indirizzo non lo dà. Vuoto: 1883 in testo leggibile, 8883 crittografato. |
| `topic` | Topic | *(empty)* | Topic a cui il controllo si iscrive. Vuoto: si connette solo, il che già ti dice che il broker è vivo e accetta il tuo account. |
| `expect_message` | Aspetta un messaggio conservato | `false` | Un messaggio deve arrivare su quel topic prima del timeout. Solo i messaggi conservati arrivano subito: un topic che è meramente pubblicato di tanto in tanto sembrerà silenzioso. |
| `expect` | Contenuto previsto | *(empty)* | Testo che quel messaggio deve contenere. Compilarlo implica aspettare un messaggio. |
| `client_id` | Identificatore del client | *(derived)* | Nome annunciato al broker. Il predefinito deriva dal dispositivo, quindi due controlli non si disconnettono mai l'uno l'altro. |
| `server_name` | Nome del server (SNI) | *(the host)* | Nome di dominio annunciato al broker e controllato nel certificato. Impostalo quando l'indirizzo è un IP. |
| `insecure_tls` | Accetta un certificato non verificabile | `false` | Una catena non verificabile non conta più come fallimento: la data di scadenza è ancora registrata. |
| `allow_private_targets` | Consenti destinazioni loopback e link-local | `false` | Vedi "Indirizzi che i controlli rifiutano" sotto. |
| `timeout_seconds` | Timeout (secondi) | `5` | Tempo dopo il quale il servizio viene segnalato come giù se non ha risposto. Tra 1 e 60. |

### Motivi di fallimento

| `reason` | Cosa è successo |
|---|---|
| `connect` | La porta è chiusa o il broker è irraggiungibile. |
| `auth` | Il `CONNACK` ha rifiutato l'account, o il broker ha rifiutato l'iscrizione — un elenco di controllo d'accesso non copre più questo topic. |
| `protocol` | Qualcosa ha risposto ma non parla MQTT 3.1.1, o il broker ha rifiutato l'identificatore del client. |
| `payload` | Nessun messaggio conservato è arrivato sul topic, o il suo contenuto non contiene il testo previsto. |

## WebSocket

**Endpoint WebSocket** — esegue l'handshake di aggiornamento, e può inviare un frame e
aspettarne uno: un fallimento diverso da una pagina HTTP semplice. Esempi: l'API di Home
Assistant, una dashboard live, un flusso di log, un endpoint dietro un reverse
proxy.

### Configurazione

1. Nell'indirizzo, incolla l'endpoint completo:
   "wss://home.example.com/api/websocket". Senza schema, si assume wss.
2. Il controllo esegue l'intero handshake di apertura e verifica la risposta che il server
   calcola dalla chiave che è stata inviata: un reverse proxy che ha perso l'intestazione Upgrade
   viene rilevato qui, dove un controllo HTTP vedrebbe comunque un sano 200.
3. Niente altro è necessario per la maggior parte degli endpoint. Uno status diverso da 101 è
   segnalato con il suo codice, quindi un 401 da un token scaduto non viene confuso con un'
   interruzione.
4. Per andare oltre, compila "Contenuto previsto" con un frammento del primo frame
   che il server invia da sé, ad esempio il saluto dell'API di Home Assistant.
5. Se l'endpoint non dice nulla finché non viene parlato, metti un messaggio in "Frame
   da inviare" e cosa deve contenere la sua risposta in "Contenuto previsto".

!!! warning
    Il controllo si apre e si interrompe entro un secondo. Una connessione che un firewall
    o un timeout proxy taglia dopo trenta secondi sembra perfettamente sano qui.

Credenziali: nessuna, un nome utente e una password (inviati come autenticazione HTTP di base nella
richiesta di apertura) o un token (inviato come `Authorization: Bearer`).

### Opzioni

| Chiave | Etichetta | Predefinito | Aiuto |
|---|---|---|---|
| `path` | Percorso | *(from the address)* | Percorso della richiesta di apertura. Preso dall'indirizzo quando lo porta. |
| `port` | Porta | *(empty)* | Usato se l'indirizzo non lo dà. Vuoto: 443 per wss, 80 per ws. |
| `send` | Frame da inviare | *(empty)* | Frame di testo inviato una volta che la connessione è aperta. Vuoto: niente viene inviato. |
| `expect` | Contenuto previsto | *(empty)* | Testo che un frame ricevuto deve contenere. Vuoto e senza niente da inviare, solo l'handshake di apertura viene controllato. |
| `subprotocol` | Subprotocollo | *(empty)* | Valore di Sec-WebSocket-Protocol. Il controllo fallisce se il server sceglie uno diverso. |
| `origin` | Origin | *(empty)* | Valore dell'intestazione Origin, che alcuni server richiedono prima di aggiornare. |
| `server_name` | Nome del server (SNI) | *(the host)* | Nome di dominio annunciato al server e controllato nel certificato. Impostalo quando l'indirizzo è un IP. |
| `insecure_tls` | Accetta un certificato non verificabile | `false` | Una catena non verificabile non conta più come fallimento: la data di scadenza è ancora registrata. |
| `allow_private_targets` | Consenti destinazioni loopback e link-local | `false` | Vedi "Indirizzi che i controlli rifiutano" sotto. |
| `timeout_seconds` | Timeout (secondi) | `5` | Tempo dopo il quale il servizio viene segnalato come giù se non ha risposto. Tra 1 e 60. |

### Motivi di fallimento

| `reason` | Cosa è successo |
|---|---|
| `connect` | La porta è chiusa o l'host è irraggiungibile. |
| `status` | L'endpoint ha risposto con qualcosa di diverso da `101`; `probe_http_status_code` porta il codice. |
| `protocol` | Ha risposto `101` ma il suo `Sec-WebSocket-Accept` non corrisponde alla chiave inviata, ha scelto un altro subprotocollo, o ha chiuso la connessione subito. |
| `payload` | Nessun frame è arrivato, o non contiene il testo previsto. |

## NTP

**Server orario NTP** — invia una singola richiesta SNTP e legge la risposta:
offset, delay di roundtrip, stratum e indicatore leap. Esempi: un router o
NAS che agisce come server orario locale, un server NTP interno (chrony, ntpd), un
server pool pubblico utilizzato come riferimento.

### Configurazione

1. Nell'indirizzo, scrivi il nome o l'indirizzo IP del server: "pool.ntp.org" o
   "192.168.1.1". La porta UDP 123 viene usata a meno che l'opzione "Porta" non dica
   diversamente.
2. Il controllo invia una singola richiesta SNTP e legge la risposta: nessuno stato viene
   mantenuto tra i sondaggi, e niente viene scritto al server.
3. Registra l'offset tra l'orologio del server e l'host DumbMonit,
   il delay di roundtrip, lo stratum e l'indicatore leap.
4. Un server che risponde con stratum 16, o il cui indicatore leap legge
   "non sincronizzato", non ha mai gestito di sincronizzarsi: il controllo lo segnala
   giù indipendentemente dall'offset.
5. Abbassa "Offset tollerato" per essere avvisato prima di 100 ms; alzalo per un
   server diversi hop da un orologio di riferimento.

!!! warning
    L'offset è buono solo quanto l'orologio dell'host DumbMonit: se quell'
    host non è mantenuto in sincronia (NTP, sincronizzazione dell'ora di un hypervisor), il
    offset misurato riflette la sua deriva tanto quanto il server.

Credenziali: nessuna.

### Opzioni

| Chiave | Etichetta | Predefinito | Aiuto |
|---|---|---|---|
| `port` | Porta | `123` | Porta UDP del server NTP. |
| `offset_threshold_ms` | Offset tollerato (ms) | `100` | Al di sopra di questo offset dell'orologio, il server viene segnalato come giù. Un server non sincronizzato (stratum 16) fallisce sempre, qualunque sia questo valore. |
| `allow_private_targets` | Consenti destinazioni loopback e link-local | `false` | Vedi "Indirizzi che i controlli rifiutano" sotto. |
| `timeout_seconds` | Timeout (secondi) | `5` | Tempo dopo il quale il servizio viene segnalato come giù se non ha risposto. Tra 1 e 60. |

### Motivi di fallimento

| `reason` | Cosa è successo |
|---|---|
| `connect` | Non è stato possibile aprire il socket UDP, o la risposta è venuta da un indirizzo inaspettato. |
| `timeout` | Nessuna risposta è arrivata entro il timeout. |
| `protocol` | La risposta non era un pacchetto NTP valido, o non veniva da un server (modo sbagliato). |
| `unsynchronized` | Il server ha risposto con stratum 16 o 0 (un "bacio della morte", che nomina anche il motivo nel log), o il suo indicatore leap legge "non sincronizzato". |
| `clock_offset` | L'offset misurato supera "Offset tollerato". |

## Indirizzi che i controlli rifiutano

I controlli che si connettono a un indirizzo che dài loro — HTTP, TCP, TLS, NTP, SMTP,
PostgreSQL, MySQL, MQTT, WebSocket — segnalano quello che vedono — che è anche, parola per parola, quello che una
contraffazione di richiesta lato server fa. Per mantenere un amministratore di monitoraggio dalla lettura di servizi che solo l'
host DumbMonit può raggiungere (il database di metriche incorporato su `127.0.0.1:8428`,
un servizio di metadati di un provider cloud su `169.254.169.254`), i controlli rifiutano
per impostazione predefinita:

- indirizzi loopback (`127.0.0.0/8`, `::1`);
- indirizzi link-local (`169.254.0.0/16`, `fe80::/10`), incluso il
  indirizzo di metadati cloud, e l'indirizzo non specificato (`0.0.0.0`, `::`).

I range LAN privati (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `fc00::/7`)
sono sempre consentiti: monitorarli è il punto di un homelab.

La regola si applica all'indirizzo che hai digitato, a ogni indirizzo che un nome host
si risolve in (controllato di nuovo al momento della connessione, quindi un nome che cambia la sua
risposta non può scappare), e a ogni reindirizzamento che il controllo HTTP segue. Un
indirizzo rifiutato viene mostrato come un errore di configurazione sul target — nessun avviso di giù viene
inviato — nominando l'opzione per abilitare: `allow_private_targets`. Attivala per un
target che davvero monitora un servizio sull'host DumbMonit stesso.

Un reindirizzamento a un indirizzo rifiutato viene segnalato come una misurazione non riuscita
(`reason="connect"`), perché è il servizio monitorato che ha cambiato, non il
configurazione.

I valori trovati a un `json_path` non vengono mai ripetuti oltre 32 caratteri in un
messaggio di fallimento.

## Errori comuni

| Sintomo | Causa probabile |
|---|---|
| Ping mostra un errore di configurazione | Lo sysctl `net.ipv4.ping_group_range` manca dal file Compose. Vedi sopra. |
| HTTP giù con `reason="keyword"` | La parola chiave viene cercata nei primi `max_body_bytes` soli, senza distinzione tra maiuscole e minuscole a meno che `keyword_case_sensitive` non sia impostato. |
| HTTP giù con `reason="tls"` | Autofirmato o autorità privata: imposta `insecure_tls`. |
| Un controllo mostra un errore di configurazione che nomina `allow_private_targets` | L'indirizzo è loopback o link-local. Vedi "Indirizzi che i controlli rifiutano" sopra. |
| TCP giù anche se `telnet host port` funziona dal tuo laptop | Il contenitore DumbMonit non può raggiungere l'host (rete Docker, firewall). Testa dall'interno della rete del contenitore. |
| DNS giù con `reason="record"` | La risposta non contiene ogni valore elencato in `expect`. |
| Ping giù con `reason="packet_loss"` | Perdita al di sopra di `loss_threshold_percent`; alcuni dispositivi limitano la velocità di ICMP. |
| DNS giù con `reason="record"` anche se `expect` corrisponde | `expect_mode` è `exact` e la risposta contiene un valore extra, o `forbid` ha corrisposto. |
| SMTP giù con `reason="protocol"` su `security = starttls` | Il relè non pubblicizza STARTTLS su questa porta. Usa `tls` per 465, `none` per un relè locale semplice. |
| Un controllo di database giù con `reason="auth"` | L'account esiste ma non può connettersi dalla macchina DumbMonit: controlla `pg_hba.conf`, o la parte host dell'utente MySQL. |
| Un controllo di database giù con `reason="query"` | Connessione e login hanno funzionato. Il database è su e si rifiuta di lavorare: sola lettura, tabella mancante, recupero. |
| MQTT giù con `reason="payload"` su un topic che chiaramente ha traffico | Solo i messaggi *conservati* raggiungono un client che si è appena iscritto. Pubblica quel topic con il flag conservato, o deseleziona "Aspetta un messaggio conservato". |
| WebSocket giù con `reason="protocol"` dopo un `101` | Qualcosa sul percorso risponde all'aggiornamento senza parlare WebSocket — solitamente una regola del reverse proxy che inghiottisce l'intestazione `Upgrade`. |
