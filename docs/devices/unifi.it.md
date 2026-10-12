# UniFi Network

La rete UniFi e ciò che gestisce: gateway, switch e access point offline,
isolati o in attesa di adozione, aggiornamenti firmware in attesa, i
collegamenti WAN e la connessione internet come li vede il gateway, il
numero di client, e gli allarmi generati da UniFi. Funziona con
l'applicazione UniFi Network di una console UniFi OS (Dream Machine,
Cloud Gateway, Cloud Key) e con un UniFi Network Server self-hosted.

DumbMonit legge UniFi con una chiave API oppure con un account locale che
possiede il ruolo **View Only**. Non riavvia, adotta, aggiorna o
provisiona mai un dispositivo.

Il guasto da tenere d'occhio: **un access point o uno switch cade
silenziosamente**. I client passano al prossimo access point, la rete
continua a funzionare, e il buco di copertura o l'uplink perso vengono
notati solo giorni dopo. Lo stesso vale per un secondo collegamento WAN
andato giù: nulla si rompe finché non fallisce anche il primo.

## Due modi di accesso

| Credenziale | API usata | Cosa legge |
|---|---|---|
| Chiave API (console UniFi OS) | L'API Integration ufficiale, su `/proxy/network/integration/v1` su una console e `/integration/v1` su un server self-hosted | Versione, dispositivi e il loro stato, aggiornamenti firmware, carico, memoria e uptime di ogni dispositivo online, numero di client |
| Account View Only | L'API classica (`/api/s/<site>/stat/*`), su `/proxy/network/api/…` su una console | Tutto quanto sopra, più client per dispositivo e per tipo, i collegamenti WAN, lo stato WAN e internet, e gli allarmi |

Lo stato WAN, internet e allarmi esiste solo nell'API classica. Con una
chiave, DumbMonit chiede anche all'API classica lo stato WAN e internet con
la stessa chiave; se il controller la rifiuta, quelle serie sono
semplicemente assenti, senza un errore.

Gli allarmi sono le voci del log di sistema "critical" di UniFi nelle
ultime 24 ore (UniFi Network 8 e successivi), oppure gli allarmi non
archiviati sulle versioni più vecchie. Questo è il solo `POST` che
DumbMonit invia, e legge soltanto.

Una sessione aperta con l'account View Only viene mantenuta da una sonda
alla successiva e riaperta quando scade: UniFi non registra un nuovo login
ogni minuto.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_unifi_`. Le serie dei
dispositivi portano `device` (il suo nome, o il suo indirizzo MAC quando
non ne ha uno), `mac`, `model` e `type` (`gateway`, `switch`,
`access_point`, `other`).

| Metrica | Cosa | Etichette |
|---|---|---|
| `info` | valore 1 | `version`, `api` (`integration` o `classic`) |
| `devices` | dispositivi per stato: `online`, `offline`, `pending`, `updating`, `adopting`, `adoption_failed`, `isolated`, `other` | `state` |
| `devices_upgradable` | dispositivi adottati con un aggiornamento firmware disponibile | |
| `device_state` | 0 online, 1 offline, 2 in attesa di adozione, 3 in aggiornamento o provisioning, 4 in adozione, 5 adozione fallita, 6 isolato, 7 altro | etichette del dispositivo |
| `device_up` | 1 online o in aggiornamento, 0 offline, isolato o adozione fallita; assente per un dispositivo non ancora adottato | etichette del dispositivo |
| `device_upgradable` | 1 quando un aggiornamento firmware è disponibile | etichette del dispositivo, `firmware` |
| `device_uptime_seconds`, `device_cpu_percent`, `device_memory_percent` | solo dispositivi online | etichette del dispositivo |
| `device_clients` | client connessi al dispositivo (account View Only) | etichette del dispositivo |
| `wan_link_up` | 1 quando una porta WAN del gateway è attiva; una porta disattivata non ha serie | etichette del dispositivo, `wan` (`wan1`, `wan2`) |
| `wan_up`, `internet_up` | lo stato WAN e internet del gateway; assente senza un gateway UniFi | |
| `internet_latency_seconds` | latenza misurata dal gateway | |
| `wan_availability_percent` | disponibilità di ogni WAN sulle ultime 24 ore | `wan` |
| `subsystem_status` | 0 ok, 1 warning, 2 error, 3 unknown, per `wlan`, `wan`, `www`, `lan`, `vpn` | `subsystem` |
| `clients` | client sul sito | |
| `clients_by_type` | `wireless`, `wired`, `guest`, `vpn` (account View Only) | `type` |
| `alarms` | voci del log di sistema critico delle ultime 24 ore, oppure allarmi non archiviati | |
| `scrape_errors` | chiamate fallite nell'ultima sonda | |

Le [regole integrate](../alerting/rules.md#unifi-network) che si
applicano: UniFi device offline, UniFi Internet down, UniFi WAN link down,
UniFi gateway CPU high, UniFi device waiting for adoption, UniFi firmware
update available, UniFi alarms, più Device unreachable.

## Dispositivi

`/rest/user` (l'API classica, letta con entrambe le credenziali) elenca
ogni client che il controller ha mai visto — la maggior parte di una rete
domestica, in pratica: telefoni di passaggio, Wi-Fi per ospiti, dispositivi
intelligenti che non tornano mai più. Tracciare l'obsolescenza di tutti
questi seppellirebbe l'avviso sotto dispositivi che semplicemente non
tornano mai, non dispositivi diventati silenziosi. Per impostazione
predefinita contano solo i client che l'utente ha contrassegnato in un modo
o nell'altro: nominati in UniFi, riservati su un IP fisso, o annotati.
L'opzione **Clients to watch besides the marked ones** aggiunge indirizzi
MAC o nomi di client specifici, oppure `all` per tracciare ogni client (non
consigliato se non su una rete piccola e per lo più statica).

I client tracciati vengono pubblicati sotto la famiglia generica condivisa
da ogni integrazione DumbMonit che traccia i dispositivi client —
`dumbmonit_client_device_*`, con prefisso senza `unifi`, documentata per
intero sotto [Dispositivi client](../alerting/rules.md#client-devices):

| Metrica | Cosa | Etichette |
|---|---|---|
| `last_seen_timestamp_seconds` | Ultima connessione, come la riporta `/rest/user` | `device`, `type` (`Wired`, `Wireless` o `Unknown`), `os` (vuoto), `user` (vuoto), `kind` |
| `stale_seconds` | Positivo una volta che un client tracciato ha superato `device_stale_days` (3 per impostazione predefinita) senza connettersi; guida la regola [Client device stale](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (sempre `connection`: UniFi non ha un backup da riportare) |

## La pagina del dispositivo

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire la
pagina non interroga mai UniFi. Indica prima cosa non va (dispositivi
offline, internet o un collegamento WAN inattivo, allarmi), poi i
dispositivi per tipo con il loro stato, firmware e carico, e i client, poi
la tabella dei client tracciati con la loro ultima connessione.

## Come è stato validato

Login, il ruolo View Only, le risposte di errore (password rifiutata, sito
sconosciuto, chiave rifiutata, sessione scaduta) e le risposte di un
controller senza dispositivi sono state verificate su UniFi Network
10.6.106, self-hosted. Nessun dispositivo poteva essere adottato lì:
l'elenco dei dispositivi, lo stato di salute di un sito equipaggiato, i
client conosciuti (`/rest/user`) e i payload dell'API Integration sono
validati rispetto alla documentazione ufficiale.

## Creare un accesso di sola lettura in UniFi Network

1. Su una console UniFi OS (Dream Machine, Cloud Gateway, Cloud Key), puoi creare una chiave API: in UniFi Network, Settings → Control Plane → Integrations → Create API Key. Dalle il nome DumbMonit e copiala subito: viene mostrata solo una volta. La chiave legge i dispositivi, il loro carico e il numero di client tramite l'API Integration ufficiale.

2. Per lo stato WAN, internet e allarmi, e su un UniFi Network Server self-hosted, usa invece un account locale con il ruolo View Only: Settings → Admins & Users → Create New. Dai l'nome seguente, dagli una password lunga e casuale, limitalo all'accesso locale, e imposta il suo ruolo UniFi Network su View Only. View Only legge tutto e non può modificare nulla.

    ```
    dumbmonit
    ```

3. In DumbMonit, inserisci l'indirizzo della console o del server, per esempio "unifi.lan" (porta 443 su una console) oppure "unifi.lan:8443" per un server self-hosted, e incolla la chiave o l'account. Se la rete non è il sito predefinito, imposta l'opzione Site sul nome breve mostrato nella barra degli indirizzi dopo /manage/ o /network/.

4. DumbMonit legge soltanto. Non riavvia, adotta, aggiorna o provisiona mai un dispositivo, e non conferma mai un allarme.

!!! warning
    Non usare l'account proprietario o un account cloud UI.com: una password compromessa apre ogni console collegata ad essa. Lascia disattivata l'autenticazione a due fattori solo per questo account locale View Only, poiché un server di monitoraggio non può digitare un codice. Le console usano un certificato autofirmato per impostazione predefinita: abilita "Accept an unverifiable certificate" a meno che tu non ne abbia installato uno proprio. UniFi conosce ogni client che si è mai connesso, la maggior parte telefoni di passaggio: solo quelli nominati, riservati su un IP fisso, o annotati in UniFi vengono osservati per obsolescenza per impostazione predefinita, per evitare un avviso per ogni telefono di un visitatore. Elencane altri nell'opzione Clients to watch, oppure inserisci all.

## Credenziali

| Credenziale | Campi |
|---|---|
| API key (UniFi OS console) | Una chiave creata in Settings → Control Plane → Integrations, inviata come `X-API-KEY`. |
| View Only account | Il nome utente e la password dell'account locale View Only. |

Indirizzo: un nome host o IP (`unifi.lan`), `host:porta` (`unifi.lan:8443`
per un server self-hosted), oppure un URL completo; un indirizzo copiato
dal browser, con `/manage/…` o `/network/…`, viene accettato. Opzioni: sito
(`default`), porta (443), verifica del certificato, timeout della
richiesta (15 s), client da osservare oltre a quelli contrassegnati (vuoto,
oppure `all`), e obsolescenza del dispositivo (`device_stale_days`, 3
giorni per impostazione predefinita).
