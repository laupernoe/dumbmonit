# Hardware server (Redfish)

L'hardware proprio di un server, letto dal suo controller di gestione
(BMC): ventole, temperature rispetto alle loro proprie soglie, alimentatori
e la loro ridondanza, drive, memoria, processori e il log degli eventi.

Redfish è l'interfaccia HTTPS che il DMTF ha standardizzato per i
controller di gestione, e ogni controller attuale la parla: Supermicro,
Dell iDRAC, HPE iLO, Lenovo XClarity Controller, ASRock Rack, OpenBMC. È il
modo giusto di osservare l'hardware del server. L'agent SNMP di un BMC, al
contrario, di solito descrive il BMC stesso — il suo piccolo Linux ARM, la
sua memoria, i suoi collegamenti di rete — e non il server che gestisce
(vedi [SNMP](snmp.md#server-management-controllers-bmc)).

!!! warning "Testato solo sui mockup di riferimento DMTF"

    Questa integrazione è stata costruita e verificata sui mockup Redfish
    ufficiali del DMTF (server rack, server tower con storage locale,
    server a raffreddamento liquido con gli schemi termici e di
    alimentazione più recenti), serviti dal server mockup del DMTF. Ogni
    chiamata è una lettura e le forme delle risorse sono quelle standard,
    ma non è ancora stata eseguita per settimane contro un vero Supermicro,
    iDRAC o iLO. Facci sapere cosa non funziona.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_redfish_`. Le serie di
stato di salute sono **0 (OK), 1 (Warning) o 2 (Critical)**, direttamente
da `Status.Health`.

| Famiglia | Metriche | Etichette |
|---|---|---|
| Servizio | `info` (valore 1) | `vendor`, `product`, `redfish_version` |
| Sistema | `system_health` (quello proprio del sistema), `system_health_rollup` (tutto ciò che contiene), `system_power_on`, `memory_health`, `processor_health` (i riepiloghi del controller, un valore per tutti i DIMM e tutte le CPU), `memory_total_bytes` | `system` |
| Chassis | `chassis_health`, `chassis_health_rollup` | `chassis` |
| Temperature | `temperature_celsius`, `temperature_upper_critical_celsius` e `temperature_upper_caution_celsius` (le soglie critiche e di cautela proprie del sensore, quando le dichiara), `temperature_health` | `chassis`, `sensor` |
| Ventole | `fan_speed_rpm` (oppure `fan_speed_percent` quando il controller riporta una percentuale), `fan_lower_critical_rpm` (oppure `_percent`: il minimo proprio dichiarato della ventola, solo schema più vecchio), `fan_health`, `fan_redundancy_health` | `chassis`, `fan` / `group` |
| Alimentazione | `psu_health`, `psu_output_watts`, `psu_capacity_watts`, `power_redundancy_health`, `power_consumed_watts` | `chassis`, `psu` / `group` |
| Tensioni | `voltage_volts`, `voltage_health` | `chassis`, `sensor` |
| Storage | `storage_health`, `drive_health`, `drive_failure_predicted`, `drive_life_left_percent` (SSD), `drive_capacity_bytes` | `system`, `storage`, `drive`, `media` |
| Controller | `manager_health`, `manager_info` (valore 1) | `manager`, `firmware`, `model` |
| Log | `log_entries`, `log_critical_entries`, `log_warning_entries` | `owner` (sistema o controller), `log` |
| Raccolta | `scrape_errors`, `scrape_duration_seconds` | |

**Uno slot vuoto non è un guasto.** Un componente il cui `Status.State` è
`Absent` (un vano alimentatore vuoto, uno slot DIMM libero, una seconda CPU
mancante) oppure `Disabled` (un gruppo di ridondanza non configurato) non
produce alcuna serie affatto. Un server consegnato con un alimentatore su
due è sano, e viene letto come tale.

**Due generazioni dello schema, un unico insieme di serie.** La maggior
parte dei controller in servizio pubblica ancora le risorse più vecchie
`Chassis/{id}/Thermal` e `Chassis/{id}/Power`, una richiesta ciascuna per
ogni sensore; quelli più nuovi pubblicano solo `ThermalSubsystem`,
`PowerSubsystem`, `EnvironmentMetrics` e `Sensors`, una risorsa per
ventola, alimentatore e sensore. DumbMonit legge la forma più vecchia
quando esiste e quella più nuova altrimenti; entrambe producono esattamente
le stesse serie ed etichette, così una regola scritta una volta funziona
su entrambe.

**I log vengono contati, mai letti.** Le voci di log possono nominare
host, indirizzi e utenti: DumbMonit conserva solo quante voci ci sono, e
quante della prima pagina servita dal controller sono Critical o Warning.

Le [regole integrate](../alerting/rules.md#server-hardware-redfish) che si
applicano: Server fan failed, Server temperature above critical, Power
supply redundancy lost, Power supply failed, Drive failure predicted,
Server health critical, più Device unreachable.

## La pagina del dispositivo

Un dispositivo Redfish ottiene un proprio pannello sulla sua pagina, sopra
i grafici, letto da ciò che l'ultima sonda ha memorizzato: aprire la pagina
non interroga mai il controller. Risponde, in questo ordine:

1. **Il server è sano.** Una parola — Healthy, Degraded o Critical — dallo
   stato di salute proprio del controller e dai roll-up, e sotto ogni
   componente che lo tira giù, in una frase: *CPU1 Temp is at 47 °C, at or
   above its critical threshold of 45 °C*, *Power supply redundancy is
   lost*, *Drive 3 predicts its own failure*. Quando il controller segnala
   il server come degradato senza nominare nulla che DumbMonit legge, il
   pannello lo dice e indica il suo log degli eventi. Sotto: stato di
   alimentazione, consumo, memoria installata e firmware del controller.
2. **Ventole e temperature sono nei loro limiti.** Ogni sensore viene
   confrontato con le soglie che il controller dichiara **per quel
   sensore** — le sue temperature di cautela e critica, la velocità
   minima di una ventola — mai con un numero di DumbMonit. Un sensore che
   non dichiara alcuna soglia mostra la sua lettura e nessun verdetto, a
   meno che il controller stesso non lo riporti come degradato. I
   problemi vengono elencati prima.
3. **L'alimentazione è ridondante.** Ogni gruppo di ridondanza
   (Redundant, Redundancy degraded, Redundancy lost), poi ogni
   alimentatore presente con la sua uscita rispetto alla sua capacità e
   il suo stato. Senza alcun gruppo di ridondanza, il pannello dice che il
   controller non ne dichiara nessuno: un singolo alimentatore, oppure la
   ridondanza non configurata.
4. **I drive sono sani.** Ogni controller di storage, poi ogni drive con
   la sua capacità, la vita rimanente di un SSD e il suo stato; *Failure
   predicted* viene prima di qualsiasi altra cosa.
5. **Memoria, processori e controller**: i riepiloghi in una parola del
   controller per tutti i DIMM e tutte le CPU, lo stato di salute proprio
   del controller di gestione, modello e firmware, e le tensioni.
6. **Log degli eventi**: voci contate per severità. Le voci restano finché
   qualcuno non pulisce il log, quindi un conteggio critico può essere una
   notizia vecchia: viene mostrato, ma non cambia la parola di stato di
   salute del server.

I vani vuoti e gli slot liberi non appaiono mai: non producono alcuna
serie. Ogni stato è una parola oltre che un colore, e un valore sconosciuto
si legge "—", mai 0.

## Creare un account di sola lettura sul controller di gestione

1. Apri l'interfaccia web del controller di gestione (BMC), non il sistema operativo del server. Crea un nuovo utente locale con il nome seguente, con una password lunga usata in nessun altro posto.

    ```
    dumbmonit
    ```

2. Dagli il ruolo più basso che può leggere, e nulla di più. Supermicro: Configuration → Users → Add User, privilegio User, e spunta Redfish nel tipo di account. Dell iDRAC: iDRAC Settings → Users → Local Users → Add, ruolo Read Only. HPE iLO: Administration → User Administration → New, deseleziona ogni privilegio tranne Login. Lenovo XClarity Controller: BMC Configuration → User/LDAP → Create, ruolo Read-only. ASRock Rack: Settings → User Management, privilegio User.

3. Verifica da qualsiasi macchina sulla rete di gestione che l'account possa leggere Redfish. Il comando chiede la password e stampa il nome e lo stato di salute del server.

    ```
    curl -k -u dumbmonit https://bmc.lan/redfish/v1/Systems
    ```

4. In DumbMonit, inserisci l'indirizzo del controller, per esempio "bmc.lan" oppure "10.0.0.50", poi il nome utente e la password di quell'account.

5. DumbMonit legge soltanto: ventole, temperature, tensioni, alimentatori, drive, riepiloghi di memoria e processore, e quante voci di log ci sono per severità. Non spegne o accende mai il server, non modifica mai un'impostazione e non legge mai il testo dei log.

!!! warning
    Non riutilizzare l'account di fabbrica del controller: può spegnere il server, montare media e riscrivere il firmware. I controller di gestione forniscono anche un certificato autofirmato: se la connessione viene rifiutata per questo motivo, spunta "Accept an unverifiable certificate" nelle opzioni. Un controller risponde lentamente e una lettura completa richiede diverse richieste: se il dispositivo segnala timeout, aumenta DUMBMONIT_PROBE_TIMEOUT_SECS sul server.

I nomi di menu sopra sono quelli del firmware attuale; le generazioni più
vecchie li spostano ma mantengono gli stessi ruoli. Su Supermicro, il tipo
di account mostrato quando si crea un utente decide quali interfacce può
usare: un account senza accesso Redfish viene rifiutato con un 401 anche
con la password giusta.

## Credenziali

| Credenziale | Campi |
|---|---|
| Account del controller di gestione | Nome utente e password dell'account di sola lettura. |

Indirizzo: il nome host o IP del controller, per esempio `bmc.lan` oppure
`10.0.0.50`. La porta predefinita è 443; scrivi `host:porta` per
un'altra, oppure un URL completo (`https://bmc.lan:8443`).

## Opzioni

| Opzione | Predefinito | Effetto |
|---|---|---|
| `port` | `443` | Porta HTTPS, se l'indirizzo non ne fornisce una. |
| `insecure_tls` | `false` | Accetta un certificato autofirmato. |
| `request_timeout_seconds` | `8` | Tempo consentito per ogni chiamata Redfish, da 1 a 120. |
| `auth` | `basic` | `basic` invia il nome utente e la password con ogni richiesta e non lascia nulla aperto sul controller. `session` accede una volta e riutilizza il token di sessione (`X-Auth-Token`), riaprendolo solo quando il controller lo rifiuta e chiudendo quello che sostituisce. Usalo solo se il controller rifiuta l'autenticazione di base: i controller hanno solo una manciata di slot di sessione. |
| `storage` | `true` | Legge i controller di storage e ogni drive dietro di essi. Una richiesta per drive. |
| `logs` | `true` | Conta le voci del log degli eventi di sistema e del log proprio del controller, per severità. |

Una lettura completa è qualche decina di richieste: la radice del
servizio, poi ogni sistema, chassis e controller, le loro risorse termiche
e di alimentazione, i controller di storage e i drive, i servizi di log.
DumbMonit ne invia al massimo quattro alla volta. Un controller lento può
richiedere un timeout di sonda più lungo
(`DUMBMONIT_PROBE_TIMEOUT_SECS`, vedi
[Configurazione](../reference/configuration.md)) e un intervallo di sonda
di due minuti o più.

## Errori comuni

| Sintomo | Causa probabile |
|---|---|
| *Authentication refused* | Password errata, oppure l'account non è autorizzato a usare Redfish (tipo di account Supermicro, privilegio "Login" mancante su iLO). |
| *Insufficient privileges* | Il ruolo è troppo basso per una risorsa; il resto della lettura comunque procede. Alcuni controller nascondono il log degli eventi al loro ruolo più basso. |
| *TLS certificate rejected* | Il certificato autofirmato del controller: spunta "Accept an unverifiable certificate", oppure installa un certificato fidato sul controller. |
| *Timed out* | Il controller è lento a rispondere: aumenta `DUMBMONIT_PROBE_TIMEOUT_SECS`, disattiva `storage` o `logs`, oppure allunga l'intervallo. |
| Nessuna soglia di temperatura | Il controller non pubblica `UpperThresholdCritical` per quel sensore: la temperatura viene comunque registrata, ma la regola integrata non può confrontarla con nulla. |
