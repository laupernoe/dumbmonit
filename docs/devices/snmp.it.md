# Dispositivo SNMP

Il tipo più universale: quasi ogni apparecchiatura di rete parla SNMP.
Switch, router, NAS, UPS e stampanti rientrano tutti. SNMP v1, v2c e v3 sono
supportati.

## Cosa controlla

Il profilo di raccolta viene rilevato automaticamente dal `sysObjectID` del
dispositivo (e, per gli UPS, dal suo `sysDescr`). Sette profili sono
inclusi nel prodotto; un dispositivo può corrispondere a diversi, e i
profili si includono a vicenda.

| Profilo | Si applica a | Metriche (prefisso `dumbmonit_`) |
|---|---|---|
| System (SNMPv2-MIB) | Ogni dispositivo | `system_uptime_seconds`, `system_info` (nome, descrizione, posizione, contatto come etichette) |
| Network interfaces (IF-MIB) | Ogni dispositivo, come ripiego | `if_octets_in/out`, `if_packets_in/out`, `if_errors_in/out`, `if_discards_in/out`, `if_admin_status`, `if_oper_status`, `if_speed_bps`, per interfaccia |
| Host resources (HOST-RESOURCES-MIB) | Net-SNMP (Linux, BSD, macOS), l'agent SNMP di Windows, Synology, QNAP, Solaris | `host_uptime_seconds`, `host_processes`, `host_users`, `cpu_load_percent`, `storage_bytes_total/used` per filesystem, `memory_bytes_total/used`, `load_average` (1, 5 e 15 minuti, solo Net-SNMP) |
| UPS (UPS-MIB) | APC, Eaton, MGE, CyberPower, Riello, Socomec, Tripp Lite, Vertiv, Salicru, e qualsiasi agent che annuncia UPS-MIB | `ups_battery_status`, `ups_battery_seconds_on`, `ups_battery_minutes_remaining`, `ups_battery_charge_percent`, volt/ampere/temperatura della batteria, volt/hertz/guasti di linea in ingresso, `ups_output_source`, volt/hertz/watt in uscita, `ups_output_load_percent`, `ups_alarms_present` |
| Printer (PRINTER-MIB) | HP, Kyocera, Epson, Canon, Brother, Lexmark, Ricoh, Xerox, Samsung, OKI, Konica Minolta | `printer_supply_level/max` per materiale di consumo, `printer_input_level/max` per vassoio, `printer_pages_printed`, `printer_status` |
| Ethernet switch (EtherLike-MIB, LLDP) | Scelto manualmente per qualsiasi switch; incluso dai profili dei switch dei fornitori | Tutto da IF-MIB, più per porta `ether_fcs_errors`, `ether_alignment_errors`, `ether_symbol_errors`, `ether_late_collisions`, `ether_excessive_collisions`, `ether_mac_transmit_errors`, `ether_mac_receive_errors`, `ether_duplex_status` (1 sconosciuto, 2 half, 3 full), `if_broadcast_packets_in`, `if_multicast_packets_in` (64 bit), e `lldp_neighbor_info` (chi è collegato dove: `remote_sysname`, `remote_port`, `remote_port_desc`; il numero di porta locale è la seconda parte dell'etichetta `index`) |
| Zyxel switch | Ogni `sysObjectID` Zyxel (`1.3.6.1.4.1.890`) | Tutto da Ethernet switch, più `cpu_load_percent` (media su un minuto), `zyxel_cpu_5min_percent`, `memory_usage_percent`, `zyxel_firmware_info` (firmware e modello come etichette), e sugli switch che pubblicano il monitor hardware, `zyxel_temperature_celsius` e `zyxel_temperature_high_threshold_celsius` per sensore, `zyxel_fan_rpm` e `zyxel_fan_low_threshold_rpm` per ventola |

I contatori (octet delle interfacce, pagine stampate) sono memorizzati
grezzi: usa `rate()` in una query per ottenere un throughput. Un riavvio del
dispositivo appare come un azzeramento del contatore, mai come un picco
fittizio.

Le [regole integrate](../alerting/rules.md) che si applicano ai dispositivi
SNMP: Monitor unreachable, High CPU, Disk almost full, Filesystem almost full
(forecast), UPS on battery, UPS battery low, Port accumulating errors, Port
flapping, Unusual CPU (baseline).

### Switch Zyxel

La parte standard del profilo Zyxel (interfacce, errori Ethernet, LLDP) è
comune a ogni switch Zyxel. I valori di CPU, memoria e firmware provengono
dal ramo privato che gli attuali switch gestiti condividono sotto
`1.3.6.1.4.1.890.1.15` (l'XS1930 e il GS1900 tra questi); i modelli più
vecchi e i firewall Zyxel li organizzano in modo diverso, e quelle serie
restano semplicemente vuote lì. Il profilo è stato verificato su un vero
XS1930-10; le sue serie di temperatura e ventola seguono la MIB di monitor
hardware pubblicata da Zyxel ma non sono ancora state osservate su hardware
reale.

`dot3StatsFrameTooLongs` non viene letto di proposito: sull'XS1930 conta
miliardi di jumbo frame perfettamente sani, e una serie che sale sempre su
una rete sana insegna solo a ignorarla.

### Controller di gestione server (BMC) {#server-management-controllers-bmc}

L'agent SNMP proprio di un BMC descrive il BMC, non il server: il suo
piccolo Linux ARM, la sua memoria, i suoi collegamenti di rete. Un BMC
Supermicro, per esempio, risponde come Net-SNMP e riceve il profilo Host
resources, che legge tutto ciò che quell'agent pubblica su se stesso. Le
ventole, le temperature, gli alimentatori e i drive del server vengono letti
invece tramite [Redfish](redfish.md).

## Cosa preparare sul dispositivo

1. Apri l'interfaccia di amministrazione del dispositivo.
2. Cerca la sezione SNMP, spesso sotto "Network", "Services" o
   "Administration".
3. Abilita SNMP v2c e annota la community di sola lettura ("public" per
   impostazione predefinita su molti dispositivi).
4. Se il dispositivo filtra per indirizzo, consenti l'indirizzo del server
   DumbMonit.
5. Torna su DumbMonit, inserisci l'indirizzo e la community: il resto viene
   rilevato automaticamente.

!!! warning
    Una community non è cifrata sulla rete. Su una rete condivisa, preferisci
    SNMP v3, che autentica e cifra.

## Credenziali

| Credenziale | Campi |
|---|---|
| SNMP v1 / v2c (community) | La community di sola lettura. |
| SNMP v3 | Nome utente; autenticazione opzionale (MD5, SHA-1, SHA-224, SHA-256, SHA-384 o SHA-512 e una passphrase); privacy opzionale (DES, AES-128, AES-192 o AES-256 e una passphrase); nome di contesto opzionale. |

Indirizzo: un IP o nome host, per esempio `192.168.1.10`. La porta
predefinita è 161; scrivi `host:porta` per usarne un'altra.

## Opzioni

Questo tipo non legge alcuna opzione. I tag liberi restano disponibili e
vengono copiati su ogni serie come `tag_<key>`.

## Errori comuni

| Sintomo | Causa probabile |
|---|---|
| *Unreachable* subito dopo l'aggiunta | Community errata, SNMP non abilitato, oppure il dispositivo accetta richieste solo da indirizzi consentiti. Il dispositivo non risponde mai a una community errata: appare esattamente come un dispositivo spento. |
| Solo serie `system_*` e `if_*` | Il `sysObjectID` non corrisponde a nessun profilo specifico. IF-MIB e System si applicano a tutto; HOST-RESOURCES, UPS, PRINTER e Zyxel richiedono un OID di fornitore conosciuto. Su uno switch di un'altra marca, scegli manualmente il profilo Ethernet switch. |
| Nessun traffico di interfaccia | Il dispositivo non espone i contatori a 64 bit `ifXTable`. |
| Scansione vuota | La scansione prova una community su ogni indirizzo; i dispositivi con un'altra community o solo SNMP v3 non rispondono. |

Usa **Probe now** sulla pagina del dispositivo: riporta il numero di
campioni e i nomi delle serie prodotti da una sonda.
