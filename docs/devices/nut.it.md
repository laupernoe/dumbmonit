# UPS con NUT

Un UPS collegato via USB a un NAS, un Raspberry Pi o un server è di solito
osservato da Network UPS Tools (NUT): il suo daemon, `upsd`, pubblica ciò
che l'UPS dice sulla porta TCP 3493. DumbMonit legge quel server, come
qualsiasi client NUT: se l'UPS funziona a corrente o a batteria, se la
batteria è scarica o esaurita, la carica, il tempo di autonomia rimanente,
il carico, la tensione di ingresso e l'età della batteria. Un server spesso
pubblica diversi UPS; tutti vengono letti in una sola connessione, e ogni
serie porta il nome del suo UPS.

Per un UPS con la propria scheda di rete, il tipo
[Dispositivo SNMP](snmp.md) legge invece la UPS-MIB standard.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_nut_` e portano l'etichetta
`ups`, il nome dell'UPS in `ups.conf` (`ups` su un NAS Synology, `qnapups`
su un QNAP). Una variabile che l'UPS non pubblica non produce alcuna serie.

| Metrica | Cosa | Etichette |
|---|---|---|
| `ups_on_line`, `ups_on_battery`, `ups_low_battery`, `ups_high_battery`, `ups_replace_battery`, `ups_charging`, `ups_discharging`, `ups_bypass`, `ups_calibrating`, `ups_off`, `ups_overload`, `ups_trim`, `ups_boost`, `ups_forced_shutdown`, `ups_alarm` | I flag di `ups.status` (`OL`, `OB`, `LB`, `HB`, `RB`, `CHRG`, `DISCHRG`, `BYPASS`, `CAL`, `OFF`, `OVER`, `TRIM`, `BOOST`, `FSD`, `ALARM`): 1 quando impostato, 0 altrimenti, tutti a ogni lettura | `ups` |
| `ups_status_info` | valore 1, il `ups.status` grezzo | `ups`, `status` |
| `battery_charge_percent`, `battery_charge_low_percent` | carica, e la carica sotto la quale l'UPS segnala una batteria scarica | `ups` |
| `battery_runtime_seconds`, `battery_runtime_low_seconds` | autonomia rimanente a batteria al carico attuale, e l'autonomia di batteria scarica | `ups` |
| `battery_voltage_volts`, `battery_temperature_celsius` | | `ups` |
| `battery_age_seconds` | tempo trascorso da `battery.date` (installata o sostituita), altrimenti `battery.mfr.date` | `ups`, `source` (`installed`, `manufactured`) |
| `input_voltage_volts`, `input_voltage_nominal_volts`, `input_frequency_hertz` | lato rete elettrica; 0 V durante un'interruzione | `ups` |
| `output_voltage_volts`, `output_frequency_hertz` | lato carico | `ups` |
| `ups_load_percent` | carico, in percentuale della capacità dell'UPS | `ups` |
| `ups_realpower_watts`, `ups_realpower_nominal_watts`, `ups_power_va`, `ups_power_nominal_va` | potenza assorbita e la targa, quando l'UPS le riporta | `ups` |
| `ups_temperature_celsius` | | `ups` |
| `ups_self_test_failed` | 1 quando l'ultimo autotest (`ups.test.result`) riporta un errore o un avviso, 0 quando è riuscito; assente quando nessun test è stato eseguito | `ups`, `result` |
| `ups_info` | valore 1 | `ups`, `manufacturer`, `model`, `firmware`, `driver`, `battery_type`, `description` |
| `ups_data_stale` | 1 quando `upsd` risponde ma non ha dati recenti dall'UPS (`DATA-STALE`, `DRIVER-NOT-CONNECTED`) | `ups` |
| `ups_driver_connected` | 0 quando il driver NUT di quell'UPS non è in esecuzione | `ups` |
| `server_ups` | numero di UPS che il server pubblica | |
| `server_info` | valore 1 | `version` |

I numeri di serie non vengono mai messi in un'etichetta. Al massimo 32 UPS
vengono letti per server.

Le [regole integrate](../alerting/rules.md#ups-behind-nut) che si
applicano: UPS on battery, UPS battery low, UPS battery needs replacing,
UPS load high, UPS runtime short, UPS data stale, più Device unreachable
quando il server NUT stesso smette di rispondere.

## La pagina del dispositivo

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire la
pagina non si connette mai al server NUT. Una scheda per UPS: il suo stato
a parole (a corrente, a batteria, batteria scarica, sostituire batteria,
bypass, sovraccarico…), la carica, l'autonomia rimanente, il carico, la
tensione di ingresso e l'età della batteria.

## Lasciare che DumbMonit legga il server NUT

1. Trova il server NUT: la macchina a cui l'UPS è collegato (un NAS, un Raspberry Pi, un server) esegue upsd sulla porta TCP 3493. Dall'host DumbMonit, verifica che risponda e annota i nomi dei suoi UPS.

    ```
    printf 'LIST UPS\nLOGOUT\n' | nc nas.lan 3493
    ```

    Con gli strumenti client NUT installati, `upsc -l nas.lan` dà lo stesso elenco.

2. Su un'installazione NUT semplice, fai ascoltare upsd su un indirizzo che l'host DumbMonit possa raggiungere: aggiungi questa riga a upsd.conf, apri la porta TCP 3493 nel firewall per l'host DumbMonit, e riavvia upsd. La lettura non richiede un account: upsd permette a qualsiasi host che accetta di leggere le variabili, e non cambia nulla senza un utente che ne ha il diritto.

    ```
    LISTEN 0.0.0.0 3493
    ```

3. Su un NAS, lo stesso interruttore ha un altro nome. Synology DSM: Control Panel → Hardware & Power → UPS, spunta Enable network UPS server, e aggiungi l'host DumbMonit sotto Permitted DiskStation Devices; l'UPS si chiama ups. QNAP: la pagina UPS del Control Panel, Enable network UPS master, con l'host DumbMonit tra gli indirizzi consentiti; l'UPS si chiama qnapups. TrueNAS: il servizio UPS con Remote Monitor spuntato.

4. Opzionale: DumbMonit invia un nome utente e una password solo se ne inserisci uno, e upsd non ne richiede per la lettura. Se la tua policy vuole ogni client nominato, aggiungi un utente senza azioni o instcmd a upsd.users, con una password lunga e casuale, e riavvia upsd.

    ```
    [dumbmonit]
    password = a-long-random-password
    ```

5. In DumbMonit, inserisci l'indirizzo del server NUT, per esempio "nas.lan". Ogni UPS che pubblica viene osservato; per osservarne solo alcuni, elenca i loro nomi nell'opzione UPS.

6. DumbMonit legge soltanto: invia LIST UPS e LIST VAR, mai LOGIN, SET, INSTCMD o FSD, così il server non lo conta mai come un secondario da attendere prima di spegnersi.

!!! warning
    DumbMonit osserva l'UPS; non spegne nulla. Le macchine alimentate dall'UPS hanno ancora bisogno del proprio client NUT (upsmon) per spegnersi correttamente quando la batteria si scarica.

## Credenziali

| Credenziale | Campi |
|---|---|
| No authentication | Il valore predefinito: `upsd` permette a qualsiasi host che accetta di leggere le variabili dell'UPS. |
| upsd user name / password | Inviati con `USERNAME` e `PASSWORD` prima della lettura. `upsd` li verifica solo per i comandi, che DumbMonit non invia mai. |

Indirizzo: un nome host o IP (`nas.lan`), `host:porta`, oppure la forma
`ups@host` di `upsc`, di cui viene mantenuto solo l'host. Opzioni: porta
(3493), l'UPS da osservare (tutti per impostazione predefinita) e il
timeout per scambio (10 s).

Una connessione rifiutata o nessuna risposta conta come dispositivo
irraggiungibile. `ACCESS-DENIED` viene mostrato come errore di credenziale,
e un nome UPS che il server non pubblica come errore di configurazione;
nessuno dei due sveglia qualcuno di notte. La connessione è TCP semplice:
`STARTTLS` non viene usato, e nulla di ciò che viene letto è segreto.
