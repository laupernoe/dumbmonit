# MikroTik RouterOS

Router e switch che eseguono RouterOS 7: se RouterOS e il firmware
RouterBOOT sono rimasti indietro, CPU, memoria e storage, temperature,
ventole e alimentatori, e lo stato, il traffico, gli errori e le perdite di
link di ogni interfaccia.

DumbMonit legge l'API REST che RouterOS 7 serve sotto `/rest`, con un
utente in un gruppo che può solo leggere. Invia richieste `GET` e
nient'altro.

Un MikroTik raramente si rompe; invecchia. Una release RouterOS non più
l'ultima del suo canale, un firmware RouterBOOT rimasto indietro dopo un
aggiornamento (segue solo quando richiesto, al riavvio successivo), un
alimentatore ridondante morto senza che nessuno se ne accorgesse, una porta
che continua a registrare errori o a perdere il suo link. Questo è ciò che
questa pagina osserva, insieme al carico.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_mikrotik_`. I totali sono
contatori: grafici e regole li trasformano in tassi.

| Metrica | Cosa | Etichette |
|---|---|---|
| `info` | valore 1 | `version`, `release` (`long-term`, `stable`…), `board`, `architecture`, `identity` |
| `uptime_seconds` | tempo dall'ultimo avvio | |
| `cpu_load_percent`, `cpu_count` | carico CPU, tutti i core | |
| `memory_total_bytes`, `memory_free_bytes`, `memory_used_percent` | memoria | |
| `storage_total_bytes`, `storage_free_bytes`, `storage_used_percent` | la flash o il disco su cui vive RouterOS | |
| `update_checked` | 1 quando il router ha controllato una RouterOS più recente e ha ottenuto una risposta | |
| `update_available` | 1 quando l'ultima versione del canale differisce da quella installata | `installed_version`, `latest_version`, `channel` |
| `routerboard` | 1 su hardware RouterBOARD, 0 su un Cloud Hosted Router o una macchina x86 | |
| `firmware_upgrade_pending` | 1 quando RouterBOOT è più vecchio del firmware incluso in RouterOS | `current_firmware`, `upgrade_firmware`, `model` |
| `temperature_celsius`, `voltage_volts`, `fan_rpm`, `power_watts`, `current_amperes` | ogni sensore che la scheda possiede | `sensor` (`cpu-temperature`, `fan1-speed`…) |
| `health_ok` | 1 quando un sensore `psu*-state` o `fan*-state` legge `ok`, 0 altrimenti | `sensor` |
| `interface_running` | 1 quando l'interfaccia ha un link | `interface`, `type` |
| `interface_rx_bytes_total`, `interface_tx_bytes_total`, `interface_rx_packets_total`, `interface_tx_packets_total` | traffico | `interface`, `type` |
| `interface_rx_errors_total`, `interface_tx_errors_total`, `interface_rx_drops_total`, `interface_tx_drops_total` | errori e scarti | `interface`, `type` |
| `interface_link_downs_total` | volte che il link è stato perso | `interface`, `type` |
| `interfaces_skipped` | interfacce escluse oltre il limite impostato nelle opzioni | |
| `scrape_errors` | letture secondarie fallite durante l'ultima sonda | |

Vengono lette solo le interfacce abilitate. Quelle dinamiche (un client
PPPoE o L2TP, il cui nome cambia a ogni connessione) e il loopback vengono
escluse.

I sensori dipendono dalla scheda: un hAP ha una tensione e una temperatura,
un CCR aggiunge ventole e alimentatori, un Cloud Hosted Router non ne ha
nessuno. Un sensore che la scheda non ha non produce alcuna serie.

L'ultima versione di RouterOS è conosciuta solo quando il router stesso
controlla gli aggiornamenti: un utente di sola lettura non è autorizzato ad
avviare quel controllo. Il passaggio opzionale sotto lo pianifica sul
router. Senza di esso, `update_checked` resta a 0 e la regola di
aggiornamento non si attiva mai.

Le [regole integrate](../alerting/rules.md#mikrotik-routeros) che si
applicano: MikroTik CPU high, memory high, storage almost full, temperature
high, power supply or fan failed, interface accumulating errors, link
flapping, RouterOS update available e firmware upgrade pending, più Device
unreachable.

## La pagina del monitor

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire la
pagina non si connette mai al router. Mostra la versione RouterOS e se ne è
disponibile una più recente, il firmware RouterBOOT, l'uptime, CPU, memoria
e storage, ogni sensore con il suo stato, e una tabella delle interfacce
con il loro link, traffico, errori e perdite di link.

## Creare un utente di sola lettura sul router

1. Apri un terminale sul router (WinBox → New Terminal, oppure SSH con l'account predefinito) e crea un gruppo che può solo leggere, tramite l'API REST. RouterOS richiede tutte e tre le policy per una lettura REST: rest-api apre l'API, api e read consentono la lettura stessa.

    ```
    /user group add name=dumbmonit-read policy=read,api,rest-api comment=DumbMonit
    ```

2. Crea l'utente in quel gruppo con una password lunga e casuale tua. Per accettare l'account solo da DumbMonit, aggiungi address= seguito dall'indirizzo dell'host DumbMonit.

    ```
    /user add name=dumbmonit group=dumbmonit-read password=REPLACE-WITH-A-LONG-RANDOM-PASSWORD
    ```

3. Attiva il servizio web HTTPS, che RouterOS fornisce disattivato e senza un certificato. Salta questo passaggio se www-ssl ne ha già uno. Altrimenti questi comandi creano un certificato sul router stesso; poi spunta "Accept an unverifiable certificate" nelle opzioni sotto.

    ```
    /certificate add name=local-ca common-name=local-ca key-usage=key-cert-sign,crl-sign days-valid=3650
    /certificate sign local-ca
    /certificate add name=https-cert common-name=router.lan days-valid=825
    /certificate sign https-cert ca=local-ca
    /ip service set www-ssl certificate=https-cert disabled=no
    ```

4. Opzionale: per essere informato delle nuove versioni di RouterOS, lascia che il router le controlli all'avvio e una volta al giorno. Un utente di sola lettura non può avviare quel controllo, quindi DumbMonit legge solo il suo risultato.

    ```
    /system scheduler add name=check-for-updates interval=1d start-time=startup on-event="/system package update check-for-updates once"
    ```

5. In DumbMonit, inserisci l'indirizzo del router, per esempio "192.168.88.1", con l'utente dumbmonit e la sua password. DumbMonit invia solo richieste GET a /rest: non modifica mai un'impostazione, non avvia mai un aggiornamento e non riavvia mai il router.

!!! warning
    È richiesto RouterOS 7.1 o successivo: RouterOS 6 non ha API REST. Non usare l'account predefinito: il suo gruppo completo può cambiare tutto, mentre dumbmonit-read non ha policy di scrittura e ogni modifica viene rifiutata. Il servizio www (HTTP semplice) risponde anch'esso all'API REST, ma invia la password in chiaro con ogni richiesta.

Le tre policy sono state verificate una per una su RouterOS 7.23.7, ognuna
con un utente nuovo: senza `rest-api` il router risponde 401, senza `api`
risponde "not allowed", senza `read` "not enough permissions". DumbMonit
segnala ogni caso come un problema di credenziali che nomina la policy
mancante, non come un router inattivo. Con le tre, una modifica, un
riavvio o un controllo aggiornamenti viene rifiutato con "not enough
permissions".

RouterOS può continuare ad applicare le vecchie policy di un gruppo a un
utente per un po' dopo che il gruppo viene modificato. Se cambi le policy
di un gruppo esistente e l'errore persiste, elimina e ricrea l'utente.

## Credenziali

| Credenziale | Campi |
|---|---|
| Read-only user | L'utente `dumbmonit` e la sua password, inviati come autenticazione HTTP di base con ogni richiesta. L'API REST non ha sessione. |

Indirizzo: un nome host o IP (`192.168.88.1`), `host:porta`, oppure un URL
completo (`https://router.lan:8443`). Opzioni: protocollo (HTTPS per
impostazione predefinita; HTTP usa il servizio `www`), porta (443 per
HTTPS, 80 per HTTP), verifica del certificato, timeout della richiesta
(10 s), se leggere le interfacce (attivo), e quante al massimo (64).
