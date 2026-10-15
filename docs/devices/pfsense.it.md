# pfSense

Un firewall pfSense tramite il pacchetto API REST della community
(`pfSense-pkg-RESTAPI`, versione 2): gateway inattivi, che perdono
pacchetti o rispondono lentamente, interfacce abilitate senza link,
servizi abilitati che si sono fermati, e CPU, memoria, disco e
temperatura.

DumbMonit invia una chiave API nell'header `X-API-Key`. Una chiave porta
esattamente i privilegi dell'utente pfSense a cui appartiene: l'utente
`dumbmonit` sotto possiede solo i privilegi `GET` delle cinque pagine che
DumbMonit legge, quindi la chiave può leggere quelle pagine e non
modificare nulla. Ogni minuto DumbMonit legge, in parallelo,
`/api/v2/status/system`, `/api/v2/status/gateways`,
`/api/v2/status/interfaces`, `/api/v2/status/services` e
`/api/v2/system/version`, e, se il privilegio è stato dato,
`/api/v2/system/restapi/version`.

!!! warning "Validato solo sulla documentazione del fornitore — non ancora testato su un sistema reale"

    Questa integrazione è stata costruita dalla documentazione pubblicata
    del pacchetto API REST e dal codice sorgente dei suoi modelli (nomi
    dei campi, unità, l'involucro della risposta e i suoi identificatori
    di errore), non su un pfSense in funzione. Ogni chiamata è una
    lettura. Facci sapere cosa non funziona.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_pfsense_`.

| Metrica | Etichette | Significato |
|---|---|---|
| `cpu_usage_percent`, `memory_used_percent`, `swap_used_percent`, `disk_used_percent`, `mbuf_used_percent` | | Come li mostra la dashboard, in percentuale. Il valore CPU è la stima propria di pfSense dalla media di carico. |
| `temperature_celsius` | | Quando pfSense può leggere un sensore. |
| `load_average` | `period`: `1m`, `5m`, `15m` | Media di carico. |
| `cpu_count` | | Core del processore. |
| `version_info` | `version` | Valore 1: la versione di pfSense, `2.7.2-RELEASE`. |
| `gateway_up` | `gateway` | 0 quando pfSense segna il gateway come inattivo, 1 altrimenti. |
| `gateway_status_info` | `gateway`, `status` | Valore 1: la parola propria di pfSense (`none` quando tutto va bene, `down`, `loss`, `delay`). |
| `gateway_loss_percent` | `gateway` | Perdita di pacchetti verso l'indirizzo di monitoraggio. |
| `gateway_delay_milliseconds`, `gateway_stddev_milliseconds` | `gateway` | Andata-ritorno verso l'indirizzo di monitoraggio e la sua deviazione; assente mentre il gateway è inattivo. |
| `gateways`, `gateways_down` | | Conteggi. |
| `interface_enabled`, `interface_up` | `interface`, `descr` | Abilitata nella configurazione; link attivo. |
| `interface_down` | `interface`, `descr` | 1 quando l'interfaccia è abilitata e non ha link. Un'interfaccia disattivata non è mai inattiva. |
| `interface_bytes_in_total`, `interface_bytes_out_total`, `interface_errors_in_total`, `interface_errors_out_total` | `interface`, `descr` | Contatori. |
| `interfaces_down` | | Conteggio. |
| `service_running`, `service_enabled` | `service`, `description` | Come li mostra Status > Services. |
| `service_stopped` | `service`, `description` | 1 quando il servizio è abilitato e non in esecuzione. |
| `services_stopped` | | Conteggio. |
| `restapi_update_available` | | Con il privilegio `/api/v2/system/restapi/version`: 1 quando il pacchetto API REST ha una release più recente. |
| `restapi_version_info` | `version`, `latest` | Valore 1: release installata e più recente del pacchetto. |

pfSense non dice tramite questa API se un aggiornamento di pfSense è
disponibile: la sola rotta che tocca gli aggiornamenti di sistema
(`/api/v2/system/update`) ne avvia uno. DumbMonit segnala solo gli
aggiornamenti propri del pacchetto.

Le [regole integrate](../alerting/rules.md#pfsense) che si applicano:

- **pfSense gateway down**: un gateway è segnato inattivo per tre minuti
  (Warning).
- **pfSense gateway losing packets**: più del 10% di perdita per dieci
  minuti, su un gateway che non è già inattivo (Advisory).
- **pfSense gateway slow**: andata-ritorno sopra 500 ms per quindici
  minuti (Advisory). Aumentala per un collegamento satellitare.
- **pfSense interface without link**: un'interfaccia abilitata senza link
  per cinque minuti (Advisory).
- **pfSense service stopped**: un servizio abilitato non in esecuzione per
  cinque minuti (Advisory).
- **pfSense disk nearly full**: più del 90% per trenta minuti (Advisory).

## La pagina del monitor

Il pannello Firewall indica cosa non va in una frase, mostra CPU, memoria,
disco e temperatura, poi elenca ogni gateway con la sua latenza e perdita,
le interfacce senza link, i servizi fermi, e un aggiornamento disponibile
del pacchetto API REST.

## Creare una chiave API REST di sola lettura per DumbMonit

1. Installa il pacchetto API REST della community (pfSense-pkg-RESTAPI, versione 2) da Diagnostics > Command Prompt oppure una shell SSH, con il file della pagina delle release del progetto che corrisponde alla tua versione di pfSense (2.7.2 sotto). Poi, sotto System > REST API > Settings, spunta sia BasicAuth che Key tra i metodi di autenticazione.

    ```
    pkg-static add https://github.com/pfrest/pfSense-pkg-RESTAPI/releases/latest/download/pfSense-2.7.2-pkg-RESTAPI.pkg
    ```

2. Sotto System > User Manager, aggiungi un utente chiamato dumbmonit con una password lunga e casuale. Sotto Effective Privileges, aggiungi solo questi privilegi API REST: /api/v2/status/system GET, /api/v2/status/gateways GET, /api/v2/status/interfaces GET, /api/v2/status/services GET, /api/v2/system/version GET, più /api/v2/system/restapi/version GET per conoscere gli aggiornamenti propri del pacchetto. Solo per il passaggio successivo, aggiungi anche /api/v2/auth/key POST.

3. Crea la chiave come dumbmonit, da qualsiasi macchina che raggiunge pfSense: la chiave porta esattamente i privilegi di quell'utente. Copia il valore della chiave dalla risposta, poi rimuovi di nuovo il privilegio /api/v2/auth/key POST.

    ```
    curl -k -u dumbmonit -X POST -H 'Content-Type: application/json' -d '{"descr":"DumbMonit"}' https://pfsense.lan/api/v2/auth/key
    ```

4. In DumbMonit, inserisci l'indirizzo di pfSense, per esempio "pfsense.lan", e incolla la chiave come API key. pfSense fornisce un certificato autofirmato: spunta Accept an unverifiable certificate a meno che tu non lo abbia sostituito.

!!! warning
    L'API REST non dice se un aggiornamento di pfSense è disponibile, solo se il pacchetto stesso ne ha uno. Il pacchetto è un progetto della community, non parte di pfSense: reinstallalo dopo ogni aggiornamento di pfSense.

## Credenziali

| Credenziale | Campi |
|---|---|
| REST API key | La chiave creata per `dumbmonit` nel passaggio 3, inviata come `X-API-Key`. |

Indirizzo: il nome host o IP di pfSense, `host:porta`, oppure un URL.
Opzioni:

- **Protocol** (`https`) e **Port** (443): quelli del webConfigurator,
  impostati sotto System > Advanced > Admin Access.
- **Accept an unverifiable certificate**: il certificato proprio di
  pfSense è autofirmato.
- **Timeout per request** (15 s).

## Risoluzione dei problemi

"pfSense refused the API key (401)": la chiave è errata, oppure Key non è
spuntata tra i metodi di autenticazione sotto System > REST API >
Settings.

"pfSense accepted the API key but refused … (403)": all'utente
`dumbmonit` manca il privilegio di quella pagina; il messaggio lo nomina.

"… not found: is the REST API package installed?": il pacchetto manca,
tipicamente dopo un aggiornamento di pfSense, che lo rimuove. Reinstallalo
come nel passaggio 1.
