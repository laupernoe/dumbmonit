# FortiGate

Un Fortinet FortiGate tramite l'API REST FortiOS: tunnel IPsec site-to-site
inattivi, interfacce abilitate che hanno perso il link, un cluster HA non
sincronizzato o mancante di un membro, memoria che si avvicina alla
modalità conserve, licenze FortiGuard e FortiCare in scadenza, e release
patch offerte per il branch FortiOS in esecuzione.

DumbMonit usa un utente API REST il cui profilo di accesso legge soltanto,
e che accetta solo chiamate dall'indirizzo del server DumbMonit (trusted
host). Il suo token viene inviato come `Authorization: Bearer`, mai
nell'URL: FortiOS 7.4.5 e 7.6.1 lo rifiutano lì comunque.

Ogni minuto DumbMonit legge tre pagine obbligatorie —
`/api/v2/monitor/system/status`, `/api/v2/monitor/system/resource/usage` e
`/api/v2/monitor/system/interface/select` — e sei opzionali che un profilo
più limitato, o un FortiGate senza cluster o tunnel, possono rifiutare o
lasciare vuote senza far fallire il resto: lo stato amministrativo delle
interfacce (`/api/v2/cmdb/system/interface`), statistiche e checksum HA,
tunnel IPsec, stato delle licenze e firmware disponibile.

!!! warning "Validato solo sulla documentazione del fornitore — non ancora testato su un sistema reale"

    Questa integrazione è stata costruita dalla documentazione FortiOS di
    Fortinet e dalle forme di risposta registrate dall'exporter Prometheus
    della community per FortiGate, non su un FortiGate in funzione. Ogni
    chiamata è una lettura. Le pagine di licenza e firmware sono le meno
    documentate: facci sapere cosa non funziona.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_fortigate_`.

| Metrica | Etichette | Significato |
|---|---|---|
| `version_info` | `version`, `build`, `model`, `hostname` | Valore 1. |
| `cpu_usage_percent`, `memory_used_percent`, `disk_used_percent` | | Utilizzo attuale. |
| `sessions`, `session_setup_rate` | | Sessioni attive, nuove sessioni al secondo. |
| `log_disk_ok` | | 0 quando il disco dei log richiede la formattazione. |
| `interface_link_up` | `interface`, `alias` | Link fisico. |
| `interface_enabled` | `interface`, `alias` | Stato amministrativo; ogni interfaccia conta come abilitata quando il profilo non può leggere la configurazione. |
| `interface_down` | `interface`, `alias` | 1 quando l'interfaccia è abilitata, ha un indirizzo e non ha link. Le porte libere senza indirizzo vengono lasciate stare. |
| `interface_speed_mbps` | `interface`, `alias` | Mentre il link è attivo. |
| `interface_bytes_in_total`, `interface_bytes_out_total`, `interface_errors_in_total`, `interface_errors_out_total` | `interface`, `alias` | Contatori. |
| `interfaces_down` | | Conteggio. |
| `ha_members` | | Membri nel cluster; 0 per un FortiGate standalone. |
| `ha_in_sync` | | Con due membri o più: 1 quando ogni membro ha lo stesso checksum di configurazione. |
| `ha_member_cpu_usage_percent`, `ha_member_memory_used_percent` | `member` | Per membro. |
| `ipsec_tunnel_up` | `tunnel` | 1 quando almeno una fase 2 del tunnel è attiva. I tunnel dial-up (accesso remoto) vengono esclusi. |
| `ipsec_phase2_up` | `tunnel`, `phase2` | Per selettore di fase 2. |
| `ipsec_tunnel_bytes_in_total`, `ipsec_tunnel_bytes_out_total` | `tunnel` | Contatori. |
| `ipsec_tunnel_info` | `tunnel`, `remote_gateway` | Valore 1. |
| `ipsec_tunnels`, `ipsec_tunnels_down` | | Conteggi. |
| `license_status_info` | `license`, `status` | Valore 1, per ogni licenza che il FortiGate possiede (`antivirus`, `ips`, `web_filtering`, `forticare_hardware`…); le voci `no_license` vengono escluse. |
| `license_expired` | `license` | 1 quando lo stato è `expired`. |
| `license_expiry_seconds` | `license` | Fino alla scadenza; negativo una volta scaduta. |
| `firmware_update_available` | | 1 quando FortiGuard offre una release patch più recente del branch in esecuzione (7.4.4 → 7.4.5). Un nuovo branch (7.6) è un progetto, non un avviso. |
| `firmware_latest_info` | `version` | Valore 1: quella release patch. |

Le [regole integrate](../alerting/rules.md#fortigate) che si applicano:

- **FortiGate IPsec tunnel down**: nessuna fase 2 attiva per cinque minuti
  (Warning).
- **FortiGate interface without link**: per cinque minuti (Advisory).
- **FortiGate HA out of sync**: per quindici minuti (Advisory).
- **FortiGate HA member lost**: meno membri che in qualsiasi momento delle
  ultime 24 ore (Warning). Il conteggio atteso non viene pubblicato da
  nessuna parte, quindi un membro rimosso definitivamente smette di
  avvisare il giorno dopo.
- **FortiGate memory near conserve mode**: sopra l'85% per dieci minuti
  (Advisory). All'88% per impostazione predefinita FortiOS entra in
  modalità conserve e smette di ispezionare nuove sessioni.
- **FortiGate CPU high**: sopra il 90% per quindici minuti (Advisory).
- **FortiGate licence expiring**: meno di 30 giorni rimanenti, o scaduta
  (Advisory, ricordata ogni giorno).
- **FortiGate firmware update**: una release patch è offerta (Info,
  ricordata ogni settimana).

## La pagina del dispositivo

Il pannello Firewall indica cosa non va in una frase, mostra CPU, memoria,
sessioni e lo stato HA, poi elenca i tunnel IPsec (inattivi prima), le
interfacce senza link, le licenze in scadenza entro 60 giorni o scadute, e
una release patch disponibile.

## Creare un utente API REST di sola lettura per DumbMonit

1. Dalla CLI FortiGate, crea un profilo di accesso che possa solo leggere lo stato di sistema, rete e VPN, e nulla di più.

    ```
    config system accprofile
    edit "dumbmonit-ro"
    set sysgrp read
    set netgrp read
    set vpngrp read
    set fwgrp none
    set loggrp none
    set utmgrp none
    set wanoptgrp none
    set wifi none
    next
    end
    ```

2. Crea l'utente API REST dumbmonit con quel profilo, fidandoti solo dell'indirizzo del server DumbMonit (sostituisci 192.0.2.10), poi genera il suo token: viene stampato una volta.

    ```
    config system api-user
    edit "dumbmonit"
    set accprofile "dumbmonit-ro"
    set vdom "root"
    config trusthost
    edit 1
    set ipv4-trusthost 192.0.2.10 255.255.255.255
    next
    end
    next
    end
    execute api-user generate-key dumbmonit
    ```

3. In DumbMonit, inserisci l'indirizzo del FortiGate, per esempio "fw.lan" (aggiungi la porta se l'interfaccia di amministrazione non ascolta su 443), e incolla il token come API token. FortiGate fornisce un certificato autofirmato: spunta Accept an unverifiable certificate a meno che tu non ne abbia installato uno proprio.

!!! warning
    Un'interfaccia conta come inattiva solo quando è abilitata, ha un indirizzo e non ha link: le porte libere vengono lasciate stare. Gli aggiornamenti firmware sono quelli che FortiGuard offre al FortiGate stesso: senza accesso a FortiGuard, nessuno viene segnalato.

## Credenziali

| Credenziale | Campi |
|---|---|
| API token | Il token stampato da `execute api-user generate-key dumbmonit`, inviato come `Authorization: Bearer`. |

Indirizzo: il nome host o IP del FortiGate, `host:porta`, oppure un URL.
Opzioni:

- **Port** (443): la porta HTTPS dell'interfaccia di amministrazione
  (`set admin-sport` sotto `config system global`).
- **VDOM**: il dominio virtuale da leggere; vuoto per quello a cui
  appartiene l'utente API REST.
- **Accept an unverifiable certificate**: il certificato proprio del
  FortiGate è autofirmato.
- **Timeout per request** (15 s).

## Risoluzione dei problemi

"FortiGate refused the API token (401)": il token è errato o è stato
rigenerato. Genera uno nuovo con `execute api-user generate-key
dumbmonit`.

"FortiGate refused … (403)": l'indirizzo DumbMonit non è tra i trusted
host dell'utente, oppure il suo profilo non può leggere quella pagina. Una
pagina opzionale rifiutata in questo modo viene semplicemente esclusa.

Nessun valore di IPsec, licenza o firmware: il profilo non può leggerli,
oppure il FortiGate non ha alcun tunnel; il resto viene comunque letto.
