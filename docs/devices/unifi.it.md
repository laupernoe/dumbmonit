# UniFi Network

La rete UniFi e ciò che gestisce: gateway, switch e punti di accesso
offline, isolati o in attesa di adozione, aggiornamenti del firmware in attesa,
i link WAN e la connessione Internet così come il gateway li vede, il numero
di client e gli allarmi che UniFi ha generato. Funziona con l'applicazione UniFi Network
di una console UniFi OS (Dream Machine, Cloud Gateway, Cloud Key)
e con un UniFi Network Server ospitato autonomamente.

DumbMonit legge UniFi con una chiave API o con un account locale che detiene il
ruolo **View Only**. Non riavvia mai, adotta, aggiorna o provisiona un dispositivo.

Il guasto che vale la pena osservare: **un punto di accesso o uno switch si disconnette silenziosamente**.
I client si spostano al prossimo punto di accesso, la rete continua a funzionare e il
buco di copertura o il collegamento perso viene notato solo giorni dopo. Lo stesso vale per
un secondo link WAN che è sceso: nulla si rompe finché il primo non fallisce anche lui.

## Due modalità di accesso

| Credenziale | API utilizzata | Cosa legge |
|---|---|---|
| Chiave API (console UniFi OS) | L'API Integration ufficiale, su `/proxy/network/integration/v1` su una console e `/integration/v1` su un server ospitato autonomamente | Versione, dispositivi e loro stato, aggiornamenti del firmware, carico, memoria e uptime di ogni dispositivo online, numero di client |
| Account View Only | L'API classica (`/api/s/<site>/stat/*`), su `/proxy/network/api/…` su una console | Tutto quanto sopra, più client per dispositivo e per tipo, i link WAN, lo stato di WAN e Internet, e allarmi |

Lo stato WAN, Internet e allarmi esiste solo nell'API classica. Con una chiave,
DumbMonit chiede anche all'API classica lo stato di WAN e Internet con la
stessa chiave; se il controller lo rifiuta, quelle serie sono semplicemente assenti, senza
un errore.

Gli allarmi sono le voci del log di sistema "critico" di UniFi degli ultimi 24 ore
(UniFi Network 8 e successivi), o gli allarmi non archiviati su versioni più vecchie. Questo è
l'unico `POST` che DumbMonit invia, e legge solo.

Una sessione aperta con l'account View Only viene mantenuta da una sonda all'altra
e riaprerte quando scade: UniFi non registra un nuovo login ogni minuto.

## Cosa monitora

Tutte le metriche hanno il prefisso `dumbmonit_unifi_`. Le serie del dispositivo portano `device` (il suo
nome, o il suo indirizzo MAC quando non ha un nome), `mac`, `model` e `type`
(`gateway`, `switch`, `access_point`, `other`).

| Metrica | Cosa | Etichette |
|---|---|---|
| `info` | valore 1 | `version`, `api` (`integration` o `classic`) |
| `devices` | dispositivi per stato: `online`, `offline`, `pending`, `updating`, `adopting`, `adoption_failed`, `isolated`, `other` | `state` |
| `devices_upgradable` | dispositivi adottati con un aggiornamento del firmware disponibile | |
| `device_state` | 0 online, 1 offline, 2 adozione in sospeso, 3 aggiornamento o provisioning, 4 adozione, 5 adozione non riuscita, 6 isolato, 7 altro | etichette dispositivo |
| `device_up` | 1 online o in aggiornamento, 0 offline, isolato o non riuscito ad adottare; assente per un dispositivo non ancora adottato | etichette dispositivo |
| `device_upgradable` | 1 quando è disponibile un aggiornamento del firmware | etichette dispositivo, `firmware` |
| `device_uptime_seconds`, `device_cpu_percent`, `device_memory_percent` | solo dispositivi online | etichette dispositivo |
| `device_clients` | client connessi al dispositivo (account View Only) | etichette dispositivo |
| `wan_link_up` | 1 quando una porta WAN del gateway è attiva; una porta disabilitata non ha serie | etichette dispositivo, `wan` (`wan1`, `wan2`) |
| `wan_up`, `internet_up` | lo stato WAN e Internet del gateway; assente senza un gateway UniFi | |
| `internet_latency_seconds` | latenza misurata dal gateway | |
| `wan_availability_percent` | disponibilità di ogni WAN negli ultimi 24 ore | `wan` |
| `subsystem_status` | 0 ok, 1 avviso, 2 errore, 3 sconosciuto, per `wlan`, `wan`, `www`, `lan`, `vpn` | `subsystem` |
| `clients` | client sul sito | |
| `clients_by_type` | `wireless`, `wired`, `guest`, `vpn` (account View Only) | `type` |
| `alarms` | voci del log di sistema critico degli ultimi 24 ore, o allarmi non archiviati | |
| `scrape_errors` | chiamate che hanno fallito durante l'ultimo probe | |

Le [regole integrate](../alerting/rules.md#unifi-network) che
si applicano: dispositivo UniFi offline, UniFi Internet inattivo, link WAN UniFi inattivo, CPU UniFi gateway
alta, dispositivo UniFi in attesa di adozione, aggiornamento del firmware UniFi disponibile,
allarmi UniFi, più dispositivo irraggiungibile.

## Dispositivi

`/rest/user` (l'API classica, letto con una credenziale) elenca ogni
client che il controller ha mai visto — la maggior parte di una rete domestica, in pratica:
telefoni che passano, Wi-Fi ospiti, dispositivi intelligenti che non tornano mai. Il monitoraggio
di tutti loro per l'obsolescenza sotterrerebbe l'avviso sotto i dispositivi che semplicemente
non tornano mai, non i dispositivi che sono rimasti silenziosi. Per impostazione predefinita solo i client che
l'utente ha contrassegnato in qualche modo contano: denominati in UniFi, riservati su un IP fisso, o
annotati. L'opzione **Client da osservare oltre quelli contrassegnati** aggiunge indirizzi MAC specifici
o nomi client, o `all` per tracciare ogni client (non
consigliato su nulla tranne una rete piccola e per lo più statica).

I client tracciati vengono pubblicati sotto la famiglia generica condivisa da ogni
integrazione DumbMonit che traccia i dispositivi client — `dumbmonit_client_device_*`,
prefisso senza `unifi`, documentato pienamente sotto
[Dispositivi client](../alerting/rules.md#client-devices):

| Metrica | Cosa | Etichette |
|---|---|---|
| `last_seen_timestamp_seconds` | Ultimo collegamento, come `/rest/user` lo segnala | `device`, `type` (`Wired`, `Wireless` o `Unknown`), `os` (vuoto), `user` (vuoto), `kind` |
| `stale_seconds` | Positivo una volta che un client tracciato è andato oltre `device_stale_days` (3 per impostazione predefinita) senza connettersi; guida la regola [Client device stale](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (sempre `connection`: UniFi non ha un backup da segnalare) |

## La pagina del dispositivo

Il pannello sopra i grafici legge quello che la sonda ha memorizzato; aprire la pagina non
interroga mai UniFi. Dice per primo cosa è sbagliato (dispositivi offline, Internet o un link
WAN inattivo, allarmi), quindi i dispositivi per tipo con il loro stato, firmware e
carico, e i client, quindi la tabella dei client tracciati con il loro ultimo
collegamento.

## Come è stato convalidato

L'accesso, il ruolo View Only, le risposte di errore (password rifiutata, sito sconosciuto,
chiave rifiutata, sessione scaduta) e le risposte di un controller senza dispositivi
sono stati verificati rispetto a UniFi Network 10.6.106, ospitato autonomamente. Nessun dispositivo potrebbe essere
adottato lì: l'elenco dei dispositivi, la salute di un sito attrezzato, i client noti (`/rest/user`) e i payload API Integration
sono convalidati rispetto a la documentazione ufficiale.

## Crea un accesso di sola lettura in UniFi Network

1. Su una console UniFi OS (Dream Machine, Cloud Gateway, Cloud Key), puoi creare una chiave API: in UniFi Network, Impostazioni → Control Plane → Integrazioni → Crea chiave API. Denominala secondo DumbMonit e copiala ora: viene mostrata solo una volta. La chiave legge i dispositivi, il loro carico e il numero di client attraverso l'API Integration ufficiale.

2. Per lo stato WAN, Internet e allarmi, e su un server UniFi Network ospitato autonomamente, utilizza invece un account locale con il ruolo View Only: Impostazioni → Amministratori e utenti → Crea nuovo. Denominalo come segue, assegnagli una lunga password casuale, limitalo all'accesso locale, e imposta il suo ruolo UniFi Network su View Only. View Only legge tutto e non può cambiare nulla.

    ```
    dumbmonit
    ```

3. In DumbMonit, inserisci l'indirizzo della console o del server, ad esempio "unifi.lan" (porta 443 su una console) o "unifi.lan:8443" per un server ospitato autonomamente, e incolla la chiave o l'account. Se la rete non è il sito predefinito, imposta l'opzione Sito sul nome breve mostrato nella barra degli indirizzi dopo /manage/ o /network/.

4. DumbMonit legge solo. Non riavvia, adotta, aggiorna o provisiona mai un dispositivo, e non riconosce mai un allarme.

!!! warning
    Non utilizzare l'account proprietario o un account cloud UI.com: una password trapelata aprirebbe ogni console collegata ad esso. Mantieni l'autenticazione a due fattori disattiva per questo account View Only locale solo, poiché un server di monitoraggio non può digitare un codice. Le console utilizzano un certificato autofirmato per impostazione predefinita: abilita "Accetta un certificato non verificabile" a meno che non ne abbia installato uno proprio. UniFi conosce ogni client che si è mai connesso, la maggior parte di loro telefoni che passano: solo quelli denominati, riservati su un IP fisso, o annotati in UniFi vengono osservati per l'obsolescenza per impostazione predefinita, per evitare un avviso per il telefono di ogni visitatore. Elenca altri nell'opzione Client da osservare, o inserisci all.

## Controller non ancora configurato

Un controller ospitato autonomamente la cui procedura guidata di configurazione al primo avvio non è completata risponde ogni richiesta con un reindirizzamento a `/setup`. DumbMonit quindi segnala "Il controller UniFi non è ancora configurato" come errore di configurazione (non una password rifiutata): termina la procedura guidata nel browser, quindi esegui il probe di nuovo.

## Credenziali

| Credenziale | Campi |
|---|---|
| Chiave API (console UniFi OS) | Una chiave creata in Impostazioni → Control Plane → Integrazioni, inviata come `X-API-KEY`. |
| Account View Only | Il nome utente e la password dell'account View Only locale. |

Indirizzo: un nome host o IP (`unifi.lan`), `host:port` (`unifi.lan:8443` per un
server ospitato autonomamente), o un URL completo; un indirizzo copiato dal browser, con
`/manage/…` o `/network/…`, è accettato. Opzioni: sito (`default`), porta (443),
verifica certificato, timeout delle richieste (15 s), client da osservare oltre quelli contrassegnati
(vuoto, o `all`), e obsolescenza del dispositivo (`device_stale_days`, 3 giorni di
impostazione predefinita).
