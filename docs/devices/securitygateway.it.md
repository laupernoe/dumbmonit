# SecurityGateway for Email Servers

I servizi di un gateway di posta SecurityGateway, verificati dall'esterno,
e su SecurityGateway 12.5 o successivo, la versione e i contatori di
prestazioni che la sua API REST pubblica: coda di consegna, quarantena,
sessioni SMTP.

!!! warning "Validato solo sulla documentazione"

    Questa integrazione è stata costruita dalla documentazione pubblicata
    da MDaemon Technologies su SecurityGateway e sulla sua API REST, non
    su un vero gateway. Ogni chiamata è una lettura. La documentazione non
    pubblica il percorso o i nomi dei contatori di prestazioni, quindi
    DumbMonit trova il percorso nella descrizione propria dell'API e
    mantiene ogni contatore con il nome che l'API gli dà. Nessuno l'ha
    ancora eseguita su un gateway in funzione. Facci sapere cosa non
    funziona.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_securitygateway_`.

| Metrica | Etichette | Significato |
|---|---|---|
| `service_up` | `service`, `port` | 1 se la porta ha accettato la connessione e, per SMTP, ha salutato con `220`. 0 se non ha risposto, o ha risposto per rifiutare il servizio (`421`, `554`). |
| `service_response_seconds` | `service`, `port` | Tempo fino al saluto, o fino alla connessione per una porta senza saluto. |
| `api_up` | | Solo con una chiave API: 1 se l'API REST ha risposto, 0 se non lo ha fatto. |
| `api_response_seconds` | | Tempo impiegato dall'API REST per rispondere. |
| `counters_available` | | 1 se i contatori di prestazioni sono stati letti; 0 se l'API non ne descrive alcuno, o l'account della chiave non può leggerli. |
| `counter` | `counter` | Una serie per ogni valore numerico dei contatori di prestazioni, nominata come l'API lo nomina, in `snake_case`: `deliveryQueue` diventa `delivery_queue`. Al massimo 64. |
| `info` (valore 1) | `version`, `source` | La versione data dalla descrizione dell'API (`source="openapi"`), oppure dal saluto SMTP quando il gateway lo annuncia lì (`source="smtp_banner"`). |
| `scrape_duration_seconds` | | Tempo impiegato dall'intero controllo. |

**Come vengono trovati i contatori.** L'API REST di SecurityGateway si
descrive da sola su `/api/v1/openapi`. DumbMonit legge quella
descrizione, prende il percorso di sola lettura più corto senza parametri
il cui nome menziona i contatori, lo legge, e conserva ogni numero nella
sua risposta. Nulla viene rinominato e nulla viene presupposto: se una
versione futura rinomina un contatore, la serie segue il nuovo nome. I
contatori elencati nell'aiuto di SecurityGateway sono le sessioni SMTP
attive in entrata e in uscita, la coda di consegna, la quarantena
amministrativa, l'uptime, i conteggi di domini e utenti e il pool di
connessioni del database; alcuni di essi cambiano solo una volta al
minuto.

**Un'interruzione parziale è comunque un controllo riuscito.** Un
servizio fermo dà `service_up 0` per quel servizio e gli altri vengono
comunque misurati. Solo quando nessun servizio e nessuna API risponde
affatto il monitor è *unreachable*. Una chiave rifiutata è un errore
di configurazione, mostrato sulla pagina del monitor e non notificato.

**Cosa non viene letto.** La licenza e la sua scadenza, lo spazio su
disco e le statistiche dei messaggi della dashboard non sono descritti
dalla documentazione pubblicata dell'API REST. Per osservare il servizio
Windows SecurityGateway stesso e i dischi del server, installa
l'[agent](agent.md) sul server ed elenca il servizio nella sua
impostazione `services`.

Le [regole integrate](../alerting/rules.md#mdaemon-and-securitygateway)
che si applicano: SecurityGateway service down, SecurityGateway API not
answering, SecurityGateway delivery queue growing, più Device
unreachable.

## La pagina del monitor

Un monitor SecurityGateway mostra un pannello **Mail services** sopra
i grafici, letto dall'ultima misurazione memorizzata: aprire la pagina non
si connette mai al gateway. Elenca ogni servizio osservato con la sua
porta, una parola e un colore (Answering, Down) e il suo tempo di
risposta, la versione, e con una chiave API, ogni contatore di prestazioni
con il suo valore più recente.

## Osservare i servizi del gateway, e opzionalmente l'API REST

1. Nulla da installare sul server. Senza credenziale, DumbMonit si connette a SMTP (25) e all'interfaccia web (4000) dall'esterno e legge il saluto SMTP. Cambia l'elenco nell'opzione Services, per esempio per controllare HTTPS invece di HTTP.

    ```
    smtp,web_https
    ```

2. Per leggere anche i contatori di prestazioni, è richiesto SecurityGateway 12.5 o successivo. Crea un account dedicato nominato come segue, con una password lunga usata in nessun altro posto, e dagli il ruolo Domain Administrator. Se i contatori restano vuoti sulla pagina del monitor, la tua versione li riserva al ruolo Global Administrator, e l'account ha bisogno di quel ruolo invece.

    ```
    dumbmonit
    ```

3. Accedi come quell'account e crea una chiave sotto Setup/Users → Accounts → API Keys, con una data di scadenza. Una chiave porta i diritti dell'account che la crea, ed è per questo che deve essere creata da questo account e non dal tuo.

4. Limita l'account all'indirizzo da cui si connette DumbMonit nelle sue restrizioni IP: le chiavi API vi obbediscono anch'esse.

5. Verifica dall'host DumbMonit che la chiave funzioni. Il comando stampa la descrizione dell'API, che elenca anche i contatori che DumbMonit legge.

    ```
    curl -k -H 'Authorization: Bearer YOUR_KEY' https://sg.example.com:4443/api/v1/openapi
    ```

6. In DumbMonit, inserisci l'indirizzo del gateway, per esempio "sg.example.com", e incolla la chiave. L'API viene raggiunta sulla porta HTTPS dell'interfaccia web, 4443 per impostazione predefinita, sotto /api/v1.

!!! warning
    SecurityGateway non ha un ruolo di sola lettura e nessuna chiave più limitata: la chiave può fare tutto ciò che può fare il suo account. Non crearla mai dal tuo account, mantienila solo in DumbMonit e dalle una data di scadenza. Le versioni precedenti alla 12.5 non hanno API REST: lascia vuota la credenziale e DumbMonit controlla solo i servizi. L'interfaccia web usa spesso un certificato autofirmato sulla 4443: se la connessione viene rifiutata per questo motivo, spunta "Accept an unverifiable certificate" nelle opzioni.

## Credenziali

| Credenziale | Campi |
|---|---|
| Services only | Nessuna: le porte vengono controllate senza accedere. |
| API key (12.5 e successivi) | La chiave, inviata come `Authorization: Bearer …`. |

Indirizzo: il nome host o IP del gateway, per esempio `sg.example.com`.
Una porta scritta nell'indirizzo (`sg.example.com:8443`) viene presa come
porta dell'API.

## Opzioni

| Opzione | Predefinito | Effetto |
|---|---|---|
| `services` | `smtp,web` | Cosa controllare, separato da virgole: `smtp` (25), `smtps` (465), `web` (4000), `web_https` (4443). `nome:porta` cambia una porta; qualsiasi altro nome con la sua porta aggiunge un servizio tuo, controllato solo per connessione. `none` non controlla alcuna porta. |
| `request_timeout_seconds` | `10` | Tempo consentito per ogni connessione, saluto e chiamata API, da 1 a 60. |
| `api_port` | `4443` | La porta dell'interfaccia web che serve `/api/v1`. Usata solo con una chiave API. |
| `api_tls` | `true` | HTTPS verso l'API REST. Deseleziona solo per l'interfaccia web su HTTP semplice (porta 4000 per impostazione predefinita): la chiave allora attraversa la rete in chiaro. |
| `insecure_tls` | `false` | Accetta un certificato autofirmato sull'API REST. |
| `counters` | `true` | Legge i contatori di prestazioni. |
