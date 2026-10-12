# Tailscale

I dispositivi di un tailnet tramite l'API Tailscale: server andati offline,
chiavi di nodo prossime alla scadenza, dispositivi in attesa di
approvazione, e client con un aggiornamento disponibile.

DumbMonit legge `GET /api/v2/tailnet/{tailnet}/devices` su
`api.tailscale.com`, una volta al minuto. La credenziale consigliata è un
client OAuth limitato all'ambito `devices:core:read`: DumbMonit scambia il
suo ID e secret per un token di un'ora (`/api/v2/oauth/token`) e lo
mantiene finché non scade. Può elencare i dispositivi e non modificare
nulla. Un token di accesso API (`tskey-api-…`) funziona anch'esso, ma porta
ogni diritto dell'utente che lo ha creato e scade dopo al massimo 90
giorni.

Questa è un'API internet: l'indirizzo del dispositivo è quello dell'API,
non quello di una macchina. DumbMonit richiede HTTPS in uscita verso
`api.tailscale.com`; nulla nel tailnet viene contattato.

!!! warning "Validato solo sulla documentazione del fornitore — non ancora testato su un sistema reale"

    Questa integrazione è stata costruita dalla documentazione dell'API di
    Tailscale e dal modello dei dispositivi del suo client Go ufficiale,
    non su un tailnet reale. Legge soltanto. Facci sapere cosa non
    funziona.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_tailscale_`. Un dispositivo
è nominato dalla prima etichetta del suo nome MagicDNS (`nas` per
`nas.tail1234.ts.net`). I dispositivi condivisi nel tailnet da un altro
vengono contati, non descritti.

| Metrica | Etichette | Significato |
|---|---|---|
| `device_online` | `device` | 1 mentre il dispositivo è connesso al server di coordinamento. |
| `device_offline_seconds` | `device` | 0 mentre è online, altrimenti il tempo dall'ultima volta che è stato visto. |
| `device_watched` | `device` | 1 quando il dispositivo deve restare online: ogni dispositivo taggato per impostazione predefinita, oppure quelli nominati nell'opzione sotto. |
| `device_key_expiry_seconds` | `device` | Fino alla scadenza della chiave di nodo; assente quando la scadenza della chiave è disattivata. |
| `device_authorized` | `device` | 0 mentre il dispositivo attende l'approvazione. |
| `device_update_available` | `device` | 1 quando Tailscale offre un client più recente. |
| `device_info` | `device`, `os`, `client_version`, `tags` | Valore 1. |
| `devices`, `devices_online`, `devices_external`, `devices_update_available`, `devices_unauthorized`, `devices_watched_offline` | | Conteggi. |

Solo i dispositivi osservati generano un avviso quando vanno offline: un
telefono o un laptop che va in sospensione non è un'interruzione. I
dispositivi taggati sono, in pratica, i server, i subnet router e gli exit
node; elenca altri nell'opzione **Devices to keep online**, oppure
inserisci `all`.

Le [regole integrate](../alerting/rules.md#tailscale) che si applicano:

- **Tailscale device offline**: un dispositivo osservato disconnesso per
  più di 15 minuti (Advisory).
- **Tailscale node key expiring**: meno di 14 giorni rimanenti (Advisory,
  ricordata ogni giorno). Alla scadenza, il dispositivo lascia il tailnet
  finché qualcuno non lo riconnette.
- **Tailscale device awaiting approval** (Advisory): approvalo se è
  previsto, rimuovilo altrimenti.
- **Tailscale client update**: un dispositivo osservato ha un aggiornamento
  client da sei ore (Info, ricordata ogni settimana).

## Dispositivi

Ogni dispositivo del tailnet, osservato o no (eccetto i dispositivi
condivisi), viene pubblicato sotto la famiglia generica condivisa da ogni
integrazione DumbMonit che traccia i dispositivi client —
`dumbmonit_client_device_*`, con prefisso senza `tailscale`, documentata
per intero sotto
[Dispositivi client](../alerting/rules.md#client-devices):

| Metrica | Cosa | Etichette |
|---|---|---|
| `last_seen_timestamp_seconds` | Ultima connessione: "ora" mentre `connectedToControl` è vero, altrimenti `lastSeen` | `device`, `type` (`Tagged` o `Personal`), `os`, `user`, `kind` |
| `stale_seconds` | Positivo una volta che un dispositivo ha superato `device_stale_days` (3 per impostazione predefinita) senza connettersi; guida la regola [Client device stale](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (sempre `connection`: Tailscale non ha un backup da riportare) |

Un telefono o un laptop che viaggia può restare diversi giorni senza
riconnettersi: questo non ha relazione con l'impostazione **Devices to
keep online** sopra, che genera un avviso solo per i dispositivi osservati
che vanno offline. Aumenta **Device staleness** per un tailnet con
dispositivi che viaggiano.

## La pagina del dispositivo

Il pannello Tailnet indica cosa non va in una frase, mostra i dispositivi
online, i dispositivi osservati offline, gli aggiornamenti disponibili e i
dispositivi condivisi, poi elenca i dispositivi osservati e qualsiasi
dispositivo in attesa di approvazione o la cui chiave scade a breve, poi la
tabella di ogni dispositivo con la sua ultima connessione.

## Creare un client OAuth di sola lettura per DumbMonit

1. Nella console Tailscale (login.tailscale.com), apri Settings > Trust credentials (precedentemente OAuth clients) e genera un client OAuth con solo l'ambito devices:core:read. Elenca i dispositivi e non può modificare nulla.

2. Copia il client ID e il client secret (tskey-client-…), mostrati una volta. In DumbMonit, inserisci api.tailscale.com come indirizzo e incollali come Client ID e Client secret. Lascia vuota l'opzione Tailnet: viene letto il tailnet del client.

!!! warning
    Solo i dispositivi taggati (server, subnet router, exit node) dovrebbero restare online: un telefono o un laptop che va in sospensione non è un'interruzione. Nomina i dispositivi da osservare nell'opzione Devices to keep online, oppure inserisci all. Ogni dispositivo del tailnet appare nella tabella Devices sotto con la sua ultima connessione, osservato o no; un telefono che viaggia può restare giorni senza, quindi aumenta Device staleness per questi.

## Credenziali

| Credenziale | Campi |
|---|---|
| OAuth client | Client ID e client secret (`tskey-client-…`) di un client con l'ambito `devices:core:read`. |
| API access token | Un token `tskey-api-…`, inviato come bearer token. |

Indirizzo: `api.tailscale.com`. Opzioni:

- **Tailnet**: vuoto per il tailnet delle credenziali, oppure il nome del
  tailnet mostrato nella console Tailscale.
- **Devices to keep online**: nomi di dispositivo o tag separati da virgole
  (`nas, tag:router`), oppure `all`. Vuoto: ogni dispositivo taggato.
- **Timeout per request** (15 s).
- **Device staleness** (`device_stale_days`, 3 giorni per impostazione
  predefinita).

## Risoluzione dei problemi

"Tailscale refused the OAuth client": l'ID o il secret è errato, oppure il
client è stato eliminato.

"Tailscale refused to list the devices (403)": al client manca l'ambito
`devices:core:read`.

"Tailscale knows no tailnet …": lascia vuota l'opzione Tailnet.
