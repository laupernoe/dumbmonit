# Scadenza del dominio

Un dominio registrato, letto dal suo registry via RDAP: i giorni rimanenti
prima della scadenza, un blocco che lo rimuove dal DNS, un periodo di
redenzione, il suo registrar e se la sua delegazione è firmata con DNSSEC.

RDAP è il successore di WHOIS: gli stessi dati di registrazione, in formato
JSON, con un formato standard tra i registry. DumbMonit trova il registry
dell'estensione nel file bootstrap RDAP dello IANA
(`https://data.iana.org/rdap/dns.json`, letto al massimo una volta al
giorno), poi gli chiede `/domain/example.com`. Non è necessaria alcuna
credenziale, e non viene chiesto nulla al di fuori della registrazione del
dominio.

Oltre alla data di scadenza stessa, due guasti sono da tenere d'occhio. Un
dominio in `clientHold` o `serverHold` è stato rimosso dal DNS dal registrar
o dal registry, di solito per una fattura non pagata o un indirizzo di
contatto mai verificato: ogni sito e casella di posta sotto di esso si
interrompe immediatamente. Un dominio in `redemptionPeriod` o
`pendingDelete` è già scaduto e verrà rilasciato a meno che non venga
ripristinato, a pagamento.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_domain_`.

| Metrica | Cosa | Etichette |
|---|---|---|
| `expiry_days` | giorni rimanenti prima della scadenza, frazionari; negativi una volta scaduto | |
| `expiry_timestamp_seconds` | la data di scadenza, in secondi Unix | |
| `expiry_published` | 0 quando il registry non pubblica la data di scadenza | |
| `on_hold` | 1 in `clientHold` o `serverHold` | |
| `redemption` | 1 in `redemptionPeriod`, `pendingDelete` o `pendingRestore` | |
| `status` | valore 1, una serie per codice di stato, in forma EPP (`clientTransferProhibited`) | `status` |
| `registrar_info` | valore 1 | `registrar` |
| `dnssec` | 1 quando la delegazione è firmata | |
| `rdap_ok` | 0 quando l'ultima interrogazione al registry è fallita e viene mostrata una risposta precedente | |
| `rdap_age_seconds` | età della risposta mostrata | |

Il registry viene interrogato una volta per intervallo di aggiornamento (12
ore per impostazione predefinita); nel frattempo, i giorni rimanenti vengono
ricalcolati a ogni controllo. Quando il registry non risponde, l'ultima
risposta viene comunque usata per un massimo di sette giorni, con `rdap_ok`
a 0, così un'interruzione del registry non appare come un problema del
dominio.

Le [regole integrate](../alerting/rules.md#reverse-proxies-and-domains) che
si applicano: Domain expiring (sotto i 30 giorni), Domain about to expire
(sotto i 7 giorni o scaduto), Domain on hold, Domain in redemption, più
Device unreachable.

## La pagina del dispositivo

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire la
pagina non interroga mai il registry. Indica la data di scadenza e i giorni
rimanenti, i codici di stato (o il blocco e la redenzione in una frase), il
registrar e il DNSSEC, e se l'ultima interrogazione al registry è fallita.

## Osservare la registrazione di un dominio

1. Inserisci il dominio registrato, per esempio "example.com", non un nome sotto di esso come www.example.com, che il registry non conosce. Niente da configurare e nessuna credenziale: RDAP, il successore di WHOIS, è pubblico.

2. DumbMonit trova il registry dell'estensione nel file bootstrap RDAP dello IANA, poi gli chiede la data di scadenza, i codici di stato e il registrar, due volte al giorno per impostazione predefinita. I registry limitano le interrogazioni RDAP, e la data di scadenza cambia una volta all'anno: un intervallo di aggiornamento più breve non porta alcun vantaggio.

3. Alcuni registry eseguono RDAP senza essere elencati dallo IANA, per esempio quelli di .ch e .li: imposta Server RDAP sul loro indirizzo. Un'estensione il cui registry non esegue alcun servizio RDAP pubblico non può essere osservata.

    ```
    https://rdap.nic.ch/
    ```

4. Alcuni registry non pubblicano la data di scadenza (.ch è uno di questi): DumbMonit lo segnala allora, e continua comunque a osservare i codici di stato.

!!! warning
    Un dominio in clientHold o serverHold viene rimosso dal DNS: ogni sito e casella di posta sotto di esso si interrompe, di solito per una fattura non pagata o un indirizzo di contatto non verificato. DumbMonit avvisa 30 giorni prima della scadenza, alza il livello 7 giorni prima, e allerta immediatamente su un blocco o un periodo di redenzione.

Un dominio che il registry non conosce (un sottodominio, un errore di
digitazione, o un dominio che è stato eliminato) viene segnalato come errore
di configurazione, non come interruzione. I nomi con accenti sono accettati
e interrogati nella loro forma ASCII (`xn--…`).

## Credenziali

Nessuna: RDAP è pubblico.

Indirizzo: il dominio registrato (`example.com`); un URL incollato viene
ridotto al suo nome host. Opzioni: Server RDAP (vuoto: trovato tramite
IANA), intervallo di aggiornamento (12 ore, da 1 a 168) e timeout della
richiesta (10 s). L'intervallo di controllo del dispositivo stesso può
restare al suo valore predefinito: ricalcola solo i giorni rimanenti tra due
interrogazioni al registry.
