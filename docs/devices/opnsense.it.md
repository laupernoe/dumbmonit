# OPNsense

Firewall e router open source: lo stato di ogni gateway con il suo tempo
di andata-ritorno e perdita di pacchetti, la tabella di stato pf, i
contatori e gli indirizzi delle interfacce, i tunnel VPN, le concessioni
DHCP, il resolver, i servizi, CARP e gli aggiornamenti in attesa.

!!! warning "Testato solo su un'istanza di test"

    Questa integrazione è stata costruita e verificata su OPNsense 26.1,
    l'immagine ufficiale avviata in una macchina virtuale per lo scopo, con
    la chiave API limitata descritta sotto — non su un firewall che instrada
    il traffico di una famiglia da anni. Ogni chiamata è una lettura e ogni
    forma proviene da quel vero OPNsense, ma i suoi gateway non avevano un
    indirizzo di monitoraggio, non faceva parte di una coppia CARP e non
    eseguiva alcun tunnel OpenVPN o IPsec: quelle parti seguono il codice
    sorgente di OPNsense invece di una risposta osservata. Facci sapere
    cosa non funziona.

## La pagina del monitor

Oltre ai grafici generici, un monitor OPNsense mostra tre pannelli,
letti da ciò che la sonda ha memorizzato — aprire la pagina non interroga
mai il firewall stesso:

* **WAN and gateways** — l'indirizzo pubblico attuale di ogni interfaccia
  che porta un gateway, poi ogni gateway con il suo stato in una parola, il
  suo ritardo di andata-ritorno e la sua perdita di pacchetti. I gateway
  inattivi vengono prima. Questo è il pannello per cui esiste
  l'integrazione: su un firewall multi-WAN, il collegamento di backup
  prende il sopravvento silenziosamente e nulla altro sulla rete cambia.
* **Traffic and state table** — quante connessioni pf sta tracciando
  rispetto al suo limite configurato, poi ogni interfaccia con il suo
  stato di link, i suoi indirizzi, i suoi contatori di byte e pacchetti, e
  i suoi errori e scarti quando ce ne sono. Poi le concessioni DHCP,
  contate per server.
* **Firewall health** — i servizi fermi prima, poi i tunnel VPN con il
  numero di peer connessi, poi CARP quando il firewall è metà di una
  coppia, poi il firmware (versione in esecuzione, cosa offre il mirror,
  pacchetti in attesa, e se un riavvio è in sospeso), poi il resolver, poi
  la macchina: uptime, carico, memoria, swap, buffer di rete, partizioni e
  sensori di temperatura.

Solo conteggi, mai traffico. DumbMonit non legge alcuna regola del
firewall, nessuno stato di connessione, nessun indirizzo client e nessun
nome host. Le concessioni DHCP vengono contate, mai elencate; le sessioni
VPN vengono contate, mai nominate.

## "none" significa "online"

`GET /api/routes/gateway/status` scrive `"none"` nel campo `status` di un
gateway quando **tutto va bene** — una parola che da sola si legge come
"nessuna informazione". DumbMonit la traduce in `online`, così la pagina,
le serie e le regole dicono la stessa cosa dell'interfaccia propria di
OPNsense. `down` e `force_down` sono i guasti; `loss` e `delay` sono gli
avvisi di dpinger, e il gateway risponde comunque.

La stessa chiamata restituisce ritardo, perdita e deviazione standard come
**stringhe con la loro unità** (`"8.4 ms"`, `"0.0 %"`), e come `"~"` quando
il gateway non ha un indirizzo di monitoraggio. `"~"` produce una serie in
meno, mai uno zero: un gateway che nessuno pinga non ha zero millisecondi
di latenza, non ne ha affatto.

## Il controllo aggiornamenti non viene mai attivato

`GET /api/core/firmware/status` restituisce il risultato dell'*ultimo*
controllo del firewall — quello che mostra la dashboard propria di
OPNsense. DumbMonit non invia mai la forma `POST` di quella chiamata, né
`/api/core/firmware/check`, che manderebbe il firewall verso il mirror a
ogni misurazione, da ogni installazione DumbMonit.

Un firewall che non ha mai controllato — o ha appena installato un
aggiornamento, che pulisce la cache — risponde `"status": "none"`,
esattamente come fa uno aggiornato. DumbMonit distingue i due, e finché un
controllo non è stato eseguito pubblica `firmware_checked = 0` e nulla
altro: nessun "zero updates pending", nessun "no reboot needed". Esegui
un controllo una volta da System → Firmware, oppure lascia che lo faccia
la pianificazione propria del firewall.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_opnsense_`.

| Famiglia | Metriche | Etichette |
|---|---|---|
| Identità | `version_info` (le etichette portano la versione di OPNsense, la versione di FreeBSD e la linea di prodotto) | |
| Macchina | `uptime_seconds`, `load1/5/15`, `cpu_count`, `memory_used/total_bytes`, `memory_used_percent` (la memoria include la cache ZFS, che FreeBSD conta come usata), `swap_used/total_bytes`, `swap_used_percent` (assente su una macchina senza swap), `mbuf_used` e `mbuf_total` (cluster di buffer di rete in uso rispetto a `kern.ipc.nmbclusters`), `mbuf_used_percent`, `mbuf_failures` (allocazioni rifiutate dall'avvio, un contatore) | |
| Storage | `disk_used_bytes`, `disk_total_bytes`, `disk_used_percent` | `device`, `mountpoint` |
| Sensori | `temperature_celsius` | `sensor` |
| Gateway | `gateway_up` (0 solo su `down` e `force_down`), `gateway_monitored`, `gateway_delay_seconds`, `gateway_stddev_seconds`, `gateway_loss_percent`, `gateways_total`, `gateways_down` | `gateway`, `address` |
| Interfacce | `interface_up`, `interface_bytes_in/out`, `interface_packets_in/out`, `interface_errors_in/out`, `interface_drops`, `interface_collisions` (contatori, memorizzati grezzi), `interface_address_info` (l'indirizzo del momento, in un'etichetta) | `interface`, `device`, e `address` per l'ultima |
| Firewall | `pf_states`, `pf_state_limit`, `pf_states_used_percent` | |
| DHCP | `dhcp_leases`, `dhcp_leases_active` | `backend` (`kea`, `isc`, `dnsmasq`) |
| VPN | `vpn_tunnel_up`, `vpn_peers_total`, `vpn_peers_connected` (WireGuard: handshake entro tre minuti), `vpn_handshake_age_seconds` (WireGuard, il peer più recente; assente mentre nessun peer si è mai connesso), `vpn_bytes_in/out`, `vpn_tunnels_down` | `kind` (`wireguard`, `openvpn`, `ipsec`), `tunnel` (il monitor WireGuard, `wg0`, oppure la descrizione IPsec e OpenVPN) |
| Resolver | `unbound_running`, `unbound_queries` (un contatore), `unbound_cache_hit_percent` | |
| Servizi | `service_running` (1 o 0, una serie per servizio che il firewall elenca) | `service` |
| CARP | `carp_enabled`, `carp_maintenance_mode`, `carp_vip_master` (tutti assenti su un firewall che non fa parte di una coppia) | `interface`, `vhid`, `address` |
| Firmware | `firmware_checked`, poi — solo una volta eseguito un controllo — `firmware_updates_pending`, `firmware_upgrade_available`, `firmware_reboot_required`, `firmware_connection_ok` | |
| Raccolta | `up`, `scrape_errors`, `scrape_duration_seconds` | |

I contatori delle interfacce sono memorizzati grezzi, come contatori:
avvolgili in `rate()` per ottenere un throughput. Farlo in questo modo
significa che un riavvio del firewall appare come un buco, non come un
picco di diversi gigabyte al secondo.

`gateway_up` è 1 per un gateway che non viene monitorato affatto, perché
non c'è nulla che dica il contrario; `gateway_monitored` è ciò che
distingue i due. Un gateway senza un indirizzo di monitoraggio non
produce né una serie di ritardo né una serie di perdita.

Un firewall che non fa parte di una coppia ad alta disponibilità non ha
alcun indirizzo virtuale CARP, e nessuna serie CARP affatto. Questo è un
firewall senza cluster, non un cluster degradato: nessuna regola può
attivarsi.

WireGuard, OpenVPN, IPsec, Unbound e Kea sono plugin. Un firewall che non
ne ha uno risponde `404`, che non è né un errore di raccolta né un errore
di autenticazione: una metrica in meno, e nulla nell'interfaccia.

Le regole integrate che si applicano: Monitor unreachable, Internet gateway
down, Gateway losing packets, Gateway slow, Firewall state table filling
up, Firewall network buffers exhausted, VPN tunnel down, Firewall resolver
stopped, Firewall core service stopped, Firewall too hot, Firewall left in
CARP maintenance mode, Firewall reboot pending, Firewall updates pending.
Le notifiche nominano il gateway, il tunnel, il servizio o il sensore
interessato.

## Cosa preparare in OPNsense

I passaggi seguenti sono quelli mostrati dalla nota accanto al modulo. Il
principio: un gruppo con solo i privilegi di stato di cui DumbMonit ha
bisogno, un utente che non accede mai, e una chiave API che non
appartiene a nessuno dei tuoi account. Questo gruppo esatto è stato
verificato contro ogni chiamata che DumbMonit fa, su OPNsense 26.1.

1. Nell'interfaccia web: System → Access → Groups → Add. Dai al gruppo il nome seguente.

    ```
    dumbmonit
    ```

2. Spunta questi privilegi nel gruppo, esattamente come li nomina OPNsense: Lobby: Dashboard (sistema, memoria, dischi, temperature, tabella di stato), System: Gateways, Status: Interfaces, Status: Services, System: Firmware, Interfaces: Virtual IPs: Status (CARP) e Services: Unbound (MVC). Poi uno per ogni funzionalità che usi: Services: DHCP: Kea(v4) oppure Services: Dnsmasq DNS/DHCP: Settings per le concessioni, VPN: WireGuard: Status, Status: OpenVPN, Status: IPsec. Un privilegio che lasci fuori costa esattamente le metriche che porta, nulla di più.

3. System → Access → Users → Add. Dai all'account il nome seguente, lascia che OPNsense generi una password casuale (questo account non accede mai: risponde con la sua chiave), e rendilo membro del gruppo dumbmonit.

    ```
    dumbmonit
    ```

4. Modifica di nuovo quell'utente e, sotto API keys, clicca +. OPNsense scarica un piccolo file di testo con due righe, key= e secret=, e mostra il secret solo questa volta. Copia la chiave nel campo API key di DumbMonit e il secret nel suo campo API secret.

5. In DumbMonit, inserisci l'indirizzo del firewall, per esempio "192.168.1.1" oppure "fw.lan:8443". Al firewall non viene mai chiesto di controllare gli aggiornamenti: DumbMonit legge il risultato del controllo che OPNsense esegue sulla propria pianificazione, o che avvii tu da System → Firmware.

!!! warning

    Non riutilizzare l'account con cui accedi: una chiave API è una password
    che non scade mai. OPNsense non ha una variante di sola lettura per
    alcuni dei privilegi sopra: Status: Services consente anche di avviare
    e fermare un servizio, System: Firmware consente anche di avviare un
    aggiornamento, e i privilegi Unbound, Kea e Dnsmasq coprono anche le
    loro impostazioni. DumbMonit legge solo, ma per limitare cosa potrebbe
    fare una chiave compromessa, limita le reti sorgente del gruppo
    all'indirizzo dell'host DumbMonit. Non concedere mai All pages.
    OPNsense fornisce anche un certificato autofirmato: se la connessione
    viene rifiutata per questo motivo, spunta "Accept an unverifiable
    certificate" nelle opzioni.

Indirizzo: `192.168.1.1`, `fw.lan`, `fw.lan:8443`, `[fd00::1]` oppure un
URL completo. Lo schema `https` e la porta 443 vengono aggiunti se
mancanti.

Alcuni privilegi coprono più di una chiamata. `Lobby: Dashboard` porta gli
endpoint di informazioni di sistema, orario, memoria, disco, swap, buffer
di rete e temperatura e la tabella di stato; `Status: Interfaces` porta la
panoramica delle interfacce con i suoi contatori e indirizzi. Un
privilegio che lasci fuori costa esattamente le metriche che porta: la
sonda riesce comunque, e `scrape_errors` resta a zero, perché un `403` su
una chiamata opzionale viene letto come "questo account non può vedere
quello", non come un fallimento. Solo `Lobby: Dashboard` è obbligatorio:
senza di esso la sonda non può nemmeno leggere il nome e la versione del
firewall, e segnala un errore di autenticazione.

### camelCase o snake_case

OPNsense 25.7 ha rinominato i suoi percorsi API da `systemResources` a
`system_resources`. Entrambe le grafie rispondono ancora, ma i privilegi
confrontano il percorso lettera per lettera: su 25.7 e successivi una
chiave limitata riceve `403` sulla vecchia grafia, e su 25.1 su alcune
delle nuove. DumbMonit chiede prima la grafia attuale e ricade sull'altra
su un `403`, così lo stesso gruppo funziona su entrambi i lati della
rinomina.

## Opzioni

| Chiave | Etichetta | Predefinito | Aiuto |
|---|---|---|---|
| `scheme` | Protocol | `https` | HTTPS si adatta a un firewall appena installato. HTTP solo se l'interfaccia web è servita in chiaro. |
| `port` | Web interface port | `443` | Usata se l'indirizzo non fornisce una porta. |
| `insecure_tls` | Accept an unverifiable certificate | `false` | OPNsense fornisce un certificato autofirmato per impostazione predefinita. |
| `request_timeout_seconds` | Timeout per request (seconds) | `15` | Tempo consentito per ogni chiamata API, da 1 a 120. |
| `gateways` | Watch the gateways | `true` | Legge cosa dice dpinger di ogni gateway: attivo o inattivo, ritardo di andata-ritorno e perdita di pacchetti. |
| `interfaces` | Watch the interfaces | `true` | Legge lo stato di link, gli indirizzi e i contatori di byte, pacchetti, errori e scarti di ogni interfaccia. |
| `firewall` | Watch the state table | `true` | Legge quante connessioni pf sta tracciando e il limite configurato. |
| `dhcp` | Count the DHCP leases | `true` | Conta le concessioni che il firewall sta distribuendo, qualunque sia il server che esegue. Solo conteggi. |
| `vpn` | Watch the VPN tunnels | `true` | Legge WireGuard, OpenVPN e IPsec: quali tunnel esistono e quali sono attivi. |
| `unbound` | Watch the resolver | `true` | Legge se Unbound sta rispondendo. |
| `services` | Watch the services | `true` | Legge l'elenco dei servizi che il firewall gestisce e quali di essi sono in esecuzione. |
| `carp` | Watch CARP | `true` | Legge gli indirizzi virtuali di una coppia ad alta disponibilità, e la modalità di manutenzione persistente. |
| `firmware` | Watch for updates | `true` | Legge il risultato dell'ultimo controllo aggiornamenti del firewall e se un riavvio è in sospeso. |
| `temperature` | Read the temperature sensors | `true` | Legge i sensori di CPU e scheda che il firewall espone. |

## Errori comuni

| Sintomo | Causa probabile |
|---|---|
| Errore di autenticazione | La chiave e il secret sono scambiati, oppure la chiave è stata eliminata dall'utente. OPNsense mostra il secret solo una volta: se è andato perso, crea un'altra chiave ed elimina la vecchia. |
| Una pagina web invece di JSON | La chiave API viene rifiutata, oppure l'account non può raggiungere quell'endpoint. OPNsense allora serve la sua pagina di login con un `200`; DumbMonit la segnala come un problema di autenticazione invece che una risposta illeggibile. |
| Errore di certificato | Certificato autofirmato: installane uno fidato (ACME è un plugin) oppure abilita `insecure_tls`. |
| Nessuna metrica di gateway affatto | L'opzione `gateways` è disattivata, oppure al gruppo manca `System: Gateways`. Un firewall senza alcun gateway definito genuinamente non ne ha. |
| Un gateway non mostra ritardo o perdita | Non ha un indirizzo di monitoraggio, quindi dpinger non lo pinga. OPNsense scrive `~` e DumbMonit non pubblica alcuna serie invece di uno zero. |
| "Gateway down" si attiva per un collegamento che funziona | L'indirizzo di monitoraggio è irraggiungibile anche se il collegamento è attivo — un caso comune con un ISP che scarta ICMP. Imposta un diverso indirizzo di monitoraggio in System → Gateways. |
| Nessuna metrica della tabella di stato | Al gruppo manca `Lobby: Dashboard`. |
| Errore di autenticazione su una chiave che funziona in uno strumento browser | Lo strumento ha usato l'altra grafia del percorso. Con una chiave limitata, OPNsense 25.7 e successivi rispondono `403` a `systemInformation` e `200` a `system_information`. |
| Nessuna metrica di aggiornamento tranne `firmware_checked` | Il firewall non ha mai controllato gli aggiornamenti, oppure ne ha appena installato uno. Esegui un controllo da System → Firmware. |
| Nessuna metrica VPN | Il plugin non è installato, oppure nessun tunnel è configurato. Una connessione IPsec disattivata non viene deliberatamente segnalata come inattiva. |
| "Firewall resolver stopped" su un firewall che risolve bene | Unbound è installato ma disattivato perché il firewall risolve con qualcos'altro. Disattiva quella regola. |
| La versione del firmware appare non aggiornata | Il firewall non ha controllato il mirror di recente. DumbMonit legge l'ultimo controllo, non ne attiva mai uno. |
| `scrape_errors` sopra zero | Una chiamata opzionale è fallita per un motivo diverso da un privilegio mancante o un plugin mancante — un timeout, di solito. Le altre metriche non sono interessate. |
