# Home Assistant

La casa intelligente dietro la dashboard: se Home Assistant funziona o si è
avviato in modalità recupero, entità non disponibili o sconosciute per
dominio, batterie scariche, aggiornamenti in attesa (core, sistema
operativo, add-on, firmware dei dispositivi) e le riparazioni aperte.

DumbMonit legge l'API REST di Home Assistant con il token di accesso di
lunga durata di un utente dedicato **senza diritti di amministratore**, e
l'elenco delle riparazioni tramite l'API WebSocket. Non chiama mai un
servizio, non genera mai un evento e non modifica mai uno stato.

Il guasto da tenere d'occhio non è "Home Assistant non risponde" (un
[controllo del sito web](services.md#http) lo vede già) ma **Home
Assistant risponde e qualcosa dietro di esso ha smesso di funzionare**:
un'integrazione le cui entità sono tutte diventate non disponibili dopo un
aggiornamento, una chiavetta Zigbee che è scivolata via dal bus USB, un
sensore di porta la cui batteria ha raggiunto il 5%, una riparazione in
attesa da settimane.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_homeassistant_`.

| Metrica | Cosa | Etichette |
|---|---|---|
| `info` | valore 1 | `version` |
| `running` | 1 quando lo stato del core è `RUNNING`, 0 mentre si avvia o si ferma | |
| `recovery_mode` | 1 in modalità recupero (o modalità provvisoria su versioni più vecchie): la configurazione non ha potuto essere caricata | |
| `entities` | entità per dominio | `domain` |
| `entities_unavailable`, `entities_unknown` | entità in quello stato, per dominio | `domain` |
| `entity_unavailable` | valore 1 per entità non disponibile, al massimo 50 | `entity`, `name`, `domain` |
| `batteries` | sensori di batteria e sensori binari di batteria | |
| `batteries_low` | batterie sotto la soglia, e sensori binari di batteria che sono attivi | |
| `battery_low` | una serie per batteria scarica, valore = il suo livello in percentuale (0 per un sensore binario), al massimo 50 | `entity`, `name` |
| `updates_available` | entità `update.*` che sono attive | |
| `update_available` | valore 1 per aggiornamento in attesa, al massimo 50 | `entity`, `name`, `installed`, `latest` |
| `repairs` | riparazioni aperte (né ignorate né respinte) per severità: `critical`, `error`, `warning` | `severity` |
| `repair` | valore 1 per riparazione aperta, al massimo 50 | `issue`, `domain`, `severity` |
| `scrape_errors` | chiamate fallite nell'ultima sonda | |

Alcuni domini restano nello stato `unknown` per natura: un bottone mai
premuto, un evento mai ricevuto, un servizio di notifica o sintesi vocale.
Non vengono contati come unknown: `button`, `input_button`, `event`,
`scene`, `notify`, `tts`, `stt`, `conversation`, `wake_word`, `ai_task`,
`person`. L'opzione **Domains left out** rimuove ulteriori domini (per
esempio `device_tracker` o `media_player` per dispositivi spesso spenti)
dai conteggi unavailable e unknown.

Le riparazioni non sono nell'API REST: DumbMonit apre l'API WebSocket,
invia `repairs/list_issues` e la chiude. Un utente senza diritti di
amministratore può elencarle. L'opzione **Read the repairs** disattiva
questo.

Le [regole integrate](../alerting/rules.md#home-assistant) che si
applicano: Home Assistant in recovery mode, Home Assistant not running,
Home Assistant entities went unavailable, Home Assistant battery low, Home
Assistant update available, Home Assistant repair to address, più Device
unreachable.

## Dispositivi

Ogni telefono e tablet collegato all'app companion (l'integrazione
`mobile_app`) viene pubblicato sotto la famiglia generica condivisa da ogni
integrazione DumbMonit che traccia i dispositivi client —
`dumbmonit_client_device_*`, con prefisso senza `homeassistant`,
documentata per intero sotto
[Dispositivi client](../alerting/rules.md#client-devices):

| Metrica | Cosa | Etichette |
|---|---|---|
| `last_seen_timestamp_seconds` | Ultimo aggiornamento dell'entità `device_tracker` del dispositivo | `device`, `type` (`Mobile`), `os` (vuoto), `user` (la persona a cui è collegato, se presente), `kind` |
| `stale_seconds` | Positivo una volta che un dispositivo ha superato `device_stale_days` (3 per impostazione predefinita) senza un aggiornamento; guida la regola [Client device stale](../alerting/rules.md#client-devices) | `device`, `type`, `os`, `user`, `kind`, `signal` (sempre `connection`: Home Assistant non ha un backup da riportare) |

**Come viene riconosciuto un dispositivo**: `/api/states` non dice quale
integrazione ha creato un'entità — per questo serve il registro delle
entità, che l'API REST non serve. Il `device_tracker` dell'app companion è
il solo che imposta un attributo `battery_level`; le integrazioni di
localizzazione di terze parti (Life360, OwnTracks, l'integrazione demo
usata nei test) impostano al massimo `battery`. Un'integrazione di terze
parti che per caso copiasse esattamente quel nome di attributo verrebbe
scambiata per l'app companion; viceversa, un dispositivo dell'app companion
con il report batteria disattivato manca dalla tabella invece di essere
mostrato in modo errato. L'account del dispositivo, quando conosciuto,
proviene da un'entità `person.*` il cui attributo `device_trackers` lo
nomina.

## La pagina del dispositivo

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire la
pagina non interroga mai Home Assistant. Indica prima cosa non va
(modalità recupero, riparazioni di severità error, entità non disponibili),
poi le batterie scariche per livello, gli aggiornamenti in attesa con le
loro versioni, le entità per dominio, e la tabella dei dispositivi dell'app
companion con la loro ultima connessione.

## Come è stato validato

Su Home Assistant 2026.9.4, con il token di lunga durata di un utente senza
diritti di amministratore: configurazione, stati delle entità
dell'integrazione demo e di sensori template (livelli di batteria, un
sensore binario di batteria, un sensore non disponibile), entità di
aggiornamento e l'elenco delle riparazioni tramite l'API WebSocket. Gli
attributi `device_tracker` dell'app companion (`battery_level`,
`source_type`) sono stati verificati contro il codice sorgente
dell'integrazione `mobile_app`, non contro un telefono reale.

## Creare un utente dedicato e un token di lunga durata in Home Assistant

1. In Home Assistant: Settings → People → Users → Add user (attiva la modalità Advanced nel tuo profilo se la scheda Users manca). Dai all'utente il nome seguente, una password lunga e casuale e lascia disattivato l'interruttore Administrator. Un utente normale può leggere ogni entità e l'elenco delle riparazioni, e non può modificare la configurazione.

    ```
    dumbmonit
    ```

2. Accedi a Home Assistant una volta come quell'utente, apri il suo profilo (il nome in fondo alla barra laterale) → Security → Long-lived access tokens → Create token. Dagli il nome DumbMonit e copialo subito: viene mostrato solo una volta.

3. In DumbMonit, inserisci l'indirizzo di Home Assistant, per esempio "homeassistant.local" (porta 8123) oppure "https://ha.example.net" dietro un reverse proxy, e incolla il token. Le batterie sotto il 20% contano come scariche; l'opzione Low battery threshold cambia questo.

4. DumbMonit legge solo /api/config, /api/states e, tramite l'API WebSocket, l'elenco delle riparazioni. Non chiama mai un servizio, non genera mai un evento e non modifica mai uno stato.

!!! warning
    Un token di lunga durata porta ogni diritto del suo utente e resta valido per dieci anni: creane uno per l'utente dedicato, mai per il tuo. Home Assistant serve HTTP semplice a meno che TLS non sia configurato, e il token viaggia con ogni richiesta: su una rete non affidabile, usa HTTPS. Ogni telefono e tablet collegato all'app companion (l'integrazione mobile_app) appare nella tabella Devices sotto con la sua ultima attività; è opzionale e silenzioso senza quell'app.

## Credenziali

| Credenziale | Campi |
|---|---|
| Long-lived access token | Un token dell'utente dedicato, inviato come `Authorization: Bearer`. |

Indirizzo: un nome host o IP (`homeassistant.local`), `host:porta`, oppure
un URL completo (`https://ha.example.net`, con un prefisso di percorso
dietro un reverse proxy); un `/api` finale viene accettato. Opzioni:
protocollo (HTTP per impostazione predefinita), porta (8123), verifica del
certificato, timeout della richiesta (10 s), soglia batteria scarica (20%),
domini esclusi (nessuno), riparazioni (lettura) e obsolescenza del
dispositivo (`device_stale_days`, 3 giorni per impostazione predefinita).
