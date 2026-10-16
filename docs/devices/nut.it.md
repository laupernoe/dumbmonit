# UPS con NUT

Un UPS collegato a un NAS, un Raspberry Pi o un server su USB viene solitamente
monitorato da Network UPS Tools (NUT): il suo daemon, `upsd`, pubblica ciò che
l'UPS dice sulla porta TCP 3493. DumbMonit legge quel server, come qualsiasi client NUT:
se l'UPS funziona sulla linea principale o sulla batteria, se la batteria è bassa
o consumata, la carica, il tempo di autonomia rimasto, il carico, la tensione di
ingresso e l'età della batteria. Un server spesso pubblica diversi UPS; tutti vengono
letti in una connessione, e ogni serie porta il nome del suo UPS.

Per un UPS con la propria scheda di rete, il tipo di dispositivo [SNMP](snmp.md)
legge lo standard UPS-MIB.

## Cosa monitora

Tutte le metriche hanno il prefisso `dumbmonit_nut_` e portano l'etichetta `ups`, il
nome UPS in `ups.conf` (`ups` su un NAS Synology, `qnapups` su un QNAP). Una variabile
che l'UPS non pubblica non produce serie.

| Metrica | Cosa | Etichette |
|---|---|---|
| `ups_on_line`, `ups_on_battery`, `ups_low_battery`, `ups_high_battery`, `ups_replace_battery`, `ups_charging`, `ups_discharging`, `ups_bypass`, `ups_calibrating`, `ups_off`, `ups_overload`, `ups_trim`, `ups_boost`, `ups_forced_shutdown`, `ups_alarm` | I flag di `ups.status` (`OL`, `OB`, `LB`, `HB`, `RB`, `CHRG`, `DISCHRG`, `BYPASS`, `CAL`, `OFF`, `OVER`, `TRIM`, `BOOST`, `FSD`, `ALARM`): 1 quando impostati, 0 altrimenti, tutti ad ogni lettura | `ups` |
| `ups_status_info` | valore 1, il grezzo `ups.status` | `ups`, `status` |
| `battery_charge_percent`, `battery_charge_low_percent` | carica, e la carica sotto la quale l'UPS segnala una batteria bassa | `ups` |
| `battery_runtime_seconds`, `battery_runtime_low_seconds` | tempo di autonomia sulla batteria al carico presente, e il tempo di autonomia basso | `ups` |
| `battery_voltage_volts`, `battery_temperature_celsius` | | `ups` |
| `battery_age_seconds` | tempo da `battery.date` (installato o sostituito), o altrimenti `battery.mfr.date` | `ups`, `source` (`installed`, `manufactured`) |
| `input_voltage_volts`, `input_voltage_nominal_volts`, `input_frequency_hertz` | lato rete principale; 0 V durante un'interruzione | `ups` |
| `output_voltage_volts`, `output_frequency_hertz` | lato carico | `ups` |
| `ups_load_percent` | carico, in percentuale della capacità UPS | `ups` |
| `ups_realpower_watts`, `ups_realpower_nominal_watts`, `ups_power_va`, `ups_power_nominal_va` | potenza assorbita e la valutazione, quando l'UPS li segnala | `ups` |
| `ups_temperature_celsius` | | `ups` |
| `ups_self_test_failed` | 1 quando l'ultimo autotest (`ups.test.result`) segnala un errore o un avvertimento, 0 quando è passato; assente quando nessun test è stato eseguito | `ups`, `result` |
| `ups_info` | valore 1 | `ups`, `manufacturer`, `model`, `firmware`, `driver`, `battery_type`, `description` |
| `ups_data_stale` | 1 quando `upsd` risponde ma non ha dati freschi dall'UPS (`DATA-STALE`, `DRIVER-NOT-CONNECTED`) | `ups` |
| `ups_driver_connected` | 0 quando il driver NUT di quell'UPS non è in esecuzione | `ups` |
| `server_ups` | numero di UPS che il server pubblica | |
| `server_info` | valore 1 | `version` |

I numeri di serie non vengono mai messi in un'etichetta. Al massimo 32 UPS vengono
letti per server.

Le [regole incorporate](../alerting/rules.md#ups-behind-nut) che si applicano: UPS
sulla batteria, Batteria UPS bassa, Batteria UPS deve essere sostituita, Carico UPS
alto, Tempo di autonomia UPS breve, Dati UPS obsoleti, più Device irraggiungibile
quando il server NUT stesso smette di rispondere.

## La pagina del monitor

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire la pagina
non si connette mai al server NUT. Una scheda per UPS: il suo stato a parole (sulla
rete, sulla batteria, batteria bassa, sostituire batteria, bypass, sovraccarico…),
la carica, il tempo di autonomia rimasto, il carico, la tensione di ingresso e l'età
della batteria.

## Consenti a DumbMonit di leggere il server NUT

1. Trova il server NUT: la macchina a cui l'UPS è collegato (un NAS, un Raspberry Pi, un server) esegue upsd sulla porta TCP 3493. Dall'host DumbMonit, controlla che risponda e annota i nomi dei suoi UPS.

    ```
    printf 'LIST UPS\nLOGOUT\n' | nc nas.lan 3493
    ```

    Con gli strumenti client NUT installati, `upsc -l nas.lan` fornisce lo stesso elenco.

2. Su un'installazione NUT semplice, fai ascoltare a upsd un indirizzo che l'host DumbMonit può raggiungere: aggiungi questa riga a upsd.conf, apri la porta TCP 3493 nel firewall per l'host DumbMonit, e riavvia upsd. La lettura non richiede un account: upsd lascia che qualsiasi host che accetta legga le variabili, e non cambia niente senza un utente che ha il diritto.

    ```
    LISTEN 0.0.0.0 3493
    ```

3. Su un NAS, l'interruttore stesso ha un altro nome. Synology DSM: Control Panel → Hardware & Power → UPS, abilita il server UPS di rete, e aggiungi l'host DumbMonit sotto Permitted DiskStation Devices; l'UPS è denominato ups. QNAP: la pagina UPS del Control Panel, Abilita master UPS di rete, con l'host DumbMonit tra gli indirizzi consentiti; l'UPS è denominato qnapups. TrueNAS: il servizio UPS con Remote Monitor spuntato.

4. Facoltativo: DumbMonit invia un nome utente e una password solo se ne inserisci uno, e upsd non ne richiede nessuno per leggere. Se la tua politica vuole che ogni client sia denominato, aggiungi un utente senza azioni o instcmd a upsd.users, con una password casuale lunga, e riavvia upsd.

    ```
    [dumbmonit]
    password = a-long-random-password
    ```

5. In DumbMonit, inserisci l'indirizzo del server NUT, ad esempio "nas.lan". Ogni UPS che pubblica viene monitorato; per monitorare solo alcuni, elenca i loro nomi nell'opzione UPS.

6. DumbMonit solo legge: invia LIST UPS e LIST VAR, mai LOGIN, SET, INSTCMD o FSD, quindi il server non lo conta mai come secondario da aspettare prima di spegnersi.

!!! warning
    DumbMonit monitora l'UPS; non spegne niente. Le macchine che l'UPS alimenta hanno comunque bisogno del loro proprio client NUT (upsmon) per spegnersi correttamente quando la batteria si esaurisce.

## Credenziali

| Credenziale | Campi |
|---|---|
| Nessuna autenticazione | L'impostazione predefinita: `upsd` lascia che qualsiasi host che accetta legga le variabili UPS. |
| Nome utente / password upsd | Inviato con `USERNAME` e `PASSWORD` prima di leggere. `upsd` li controlla solo per i comandi, che DumbMonit non invia mai. |

Indirizzo: un nome host o IP (`nas.lan`), `host:port`, o la forma `ups@host`
di `upsc`, di cui viene mantenuto solo l'host. Opzioni: porta (3493), l'UPS da
osservare (tutti per impostazione predefinita) e il timeout per lo scambio (10 s).

Una connessione rifiutata o nessuna risposta conta come il monitor irraggiungibile.
Quando la macchina che esegue il server NUT si spegne nell'interruzione che segnala
(è spesso il suo upsmon primario), gli avvisi UPS vengono nascosti come soppressi mentre
Device irraggiungibile si attiva per lo stesso monitor: leggi l'avviso Device irraggiungibile
come il segno che il guasto di alimentazione ha vinto. Esegui il server NUT su una macchina
che supera gli altri, o osserva l'UPS da un secondo server NUT. `ACCESS-DENIED` viene
mostrato come un errore di credenziale, e un nome UPS che il server non pubblica come
errore di configurazione; nessuno dei due sveglia nessuno di notte. La connessione è TCP
semplice: `STARTTLS` non viene utilizzato e niente di ciò che viene letto è segreto.
