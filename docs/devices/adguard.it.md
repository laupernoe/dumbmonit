# AdGuard Home

Il filtro DNS stesso: se la protezione è attiva, se il suo server DNS
funziona, cosa ha risposto e bloccato, se le sue liste di filtri si
aggiornano ancora, se i suoi server upstream rispondono, e se è disponibile
una versione più recente.

Un filtro DNS rotto non appare rotto. Le pagine continuano ad aprirsi, con
le loro pubblicità. I guasti abituali sono silenziosi: protezione sospesa
"per un'ora" dall'app telefono e mai riattivata, liste di filtri che hanno
smesso di scaricarsi settimane fa, un server DNS-over-HTTPS upstream che
non risponde più così che ogni ricerca attende il successivo.

DumbMonit legge l'API REST di AdGuard Home (`/control`) con autenticazione
HTTP di base. Testato su AdGuard Home v0.107.79 (e il controllo versione
v0.107.78).

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_adguard_`. I valori di query
e blocco sono **somme sulla finestra di statistiche di AdGuard Home** (24
ore a meno che non venga cambiata in Settings → General settings), non
contatori: salgono e scendono man mano che la finestra scorre, e vengono
memorizzati così come sono.

| Metrica | Cosa | Etichette |
|---|---|---|
| `running` | 1 quando il server DNS funziona | |
| `protection_enabled` | 1 quando la protezione è attiva | |
| `protection_paused_seconds` | tempo rimanente prima che una protezione in pausa si riattivi, 0 senza pausa | |
| `version_info` | valore 1 | `version` |
| `dns_queries` | query nella finestra di statistiche | |
| `blocked_filtering` | query bloccate dalle liste di filtri e regole personalizzate | |
| `replaced_safebrowsing`, `replaced_parental`, `replaced_safesearch` | risposte sostituite da Browsing security, Parental control, Safe search | |
| `blocked_percent` | quota di query bloccate (filtri, Browsing security, Parental control); assente senza alcuna query | |
| `avg_processing_seconds` | tempo medio di risposta | |
| `stats_window_seconds`, `stats_enabled` | la finestra di statistiche e se le statistiche vengono conservate (0.107.30 e successivi) | |
| `upstream_responses`, `upstream_avg_seconds` | risposte e tempo medio di risposta per upstream nella finestra, come li mostra Top upstreams della dashboard (al massimo 8) | `upstream` |
| `upstream_up` | 1 quando il test proprio di AdGuard Home su quell'upstream passa; 0 quando fallisce o non dà risposta entro 4 secondi (al massimo 8). L'etichetta è l'upstream come AdGuard Home lo scrive nelle sue statistiche (`https://dns10.quad9.net:443/dns-query`, `192.0.2.1:53`); per un upstream rimasto silenzioso, la riga come configurata. | `upstream` |
| `filtering_enabled` | 1 quando il filtraggio è attivo in Settings → General settings | |
| `filters`, `filters_enabled` | blocklist configurate, e abilitate | |
| `filter_rules` | regole caricate dalle liste abilitate | |
| `filter_rules_count`, `filter_update_age_seconds` | per lista abilitata (al massimo 32) | `filter` |
| `filter_oldest_update_age_seconds` | età dell'aggiornamento più vecchio tra le liste abilitate | |
| `filters_never_updated` | liste abilitate che non sono mai state scaricate | |
| `filter_update_interval_hours` | ogni quanto le liste vengono aggiornate; 0 disattiva gli aggiornamenti | |
| `update_check_enabled` | 0 quando AdGuard Home funziona con il controllo versione disattivato | |
| `update_available` | 1 quando l'ultimo controllo versione ha annunciato una versione più recente; assente quando il controllo è disattivato | `latest` |
| `scrape_errors` | chiamate secondarie fallite nell'ultimo controllo | |

Il controllo versione è disattivato nell'immagine Docker ufficiale e nella
maggior parte dei pacchetti di distribuzione, che aggiornano AdGuard Home
da soli: lì, `update_available` non viene mai scritto e la sua regola resta
silenziosa.

Le statistiche upstream contano solo le risposte: un upstream che smette di
rispondere non lascia traccia in esse. Per questo DumbMonit chiede a
AdGuard Home di testare ogni upstream a ogni controllo, lo stesso test del
pulsante Test upstreams in Settings → DNS settings: una query DNS da
AdGuard Home verso quell'upstream. Un upstream morto farebbe attendere
AdGuard Home per il doppio del suo timeout upstream (20 secondi per
impostazione predefinita); DumbMonit smette di attendere dopo 4 secondi e
lo segnala inattivo. Deseleziona l'opzione Test upstream servers per
disattivare questo.

Le [regole integrate](../alerting/rules.md#adguard-home) che si applicano:
AdGuard Home protection off, AdGuard Home DNS not running, AdGuard Home
upstream failing, AdGuard Home filter lists stale, AdGuard Home update
available, più Device unreachable.

## La pagina del dispositivo

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire la
pagina non si connette mai a AdGuard Home. Indica se la protezione e il
server DNS sono attivi (e quando finisce una pausa), poi le query, i
blocchi e il tempo di risposta sulla finestra di statistiche, i server
upstream con il loro risultato di test e tempo di risposta, e le liste di
filtri con le loro regole e ultimo aggiornamento.

## Aggiungere un utente per DumbMonit a AdGuard Home

1. AdGuard Home non ha un account di sola lettura: ogni utente elencato in AdGuardHome.yaml può modificare ogni impostazione. Dai a DumbMonit un utente proprio, così può essere rimosso senza toccare il tuo. Prima crea un hash bcrypt di una password lunga e casuale (htpasswd viene con il pacchetto apache2-utils o httpd-tools):

    ```
    htpasswd -B -C 10 -n -b dumbmonit 'LONG-RANDOM-PASSWORD'
    ```

2. Ferma AdGuard Home, apri AdGuardHome.yaml (accanto al binario AdGuardHome, oppure nella cartella conf del volume Docker) e aggiungi questa riga sotto "users:", alla stessa indentazione della riga "- name:" già presente. Sostituisci HASH con ciò che htpasswd ha stampato dopo "dumbmonit:". Riavvia AdGuard Home.

    ```
    - { name: dumbmonit, password: "HASH" }
    ```

    La sezione diventa allora:

    ```yaml
    users:
      - { name: dumbmonit, password: "$2y$10$…" }
      - name: yourname
        password: $2a$10$…
    ```

3. Verifica dall'host DumbMonit che l'utente sia accettato:

    ```
    curl -u dumbmonit http://adguard.lan/control/status
    ```

4. In DumbMonit, inserisci l'indirizzo dell'interfaccia web di AdGuard Home, per esempio "adguard.lan" (porta 80) oppure "http://adguard.lan:3000", poi l'utente dumbmonit e la sua password.

5. DumbMonit legge solo lo stato, le statistiche, le liste di filtri e l'ultimo controllo versione. Non legge mai il log delle query. A ogni controllo chiede a AdGuard Home di testare ogni server DNS upstream, come fa il pulsante Test upstreams; deseleziona Test upstream servers sotto per fermarlo.

!!! warning
    L'utente dumbmonit può modificare ogni impostazione, come qualsiasi utente AdGuard Home: tieni la sua password solo per DumbMonit. Viaggia con ogni richiesta, quindi su una rete non affidabile attiva HTTPS in Settings → Encryption settings. Dopo cinque password errate, AdGuard Home rifiuta l'indirizzo per 15 minuti.

AdGuard Home deve essere fermo mentre modifichi il file: riscrive
AdGuardHome.yaml ogni volta che un'impostazione cambia, e perderebbe una
riga aggiunta mentre è in esecuzione.

## Credenziali

| Credenziale | Campi |
|---|---|
| User name / password | L'utente dedicato aggiunto a AdGuardHome.yaml, inviato come autenticazione HTTP di base. |
| No authentication | Solo per un AdGuard Home senza alcun utente in AdGuardHome.yaml. |

Indirizzo: un nome host o IP (`adguard.lan`), `host:porta`, oppure un URL
completo con un prefisso di percorso dietro un reverse proxy
(`https://dns.lan/adguard`); un `/control` finale viene ignorato. Opzioni:
protocollo (HTTP per impostazione predefinita), porta (80), verifica del
certificato, timeout della richiesta (5 s) e il test upstream (attivo).

## Chiamate

| Chiamata | Perché |
|---|---|
| `GET /control/status` | running, protection, version: l'unica chiamata il cui fallimento fa fallire il controllo |
| `GET /control/stats`, `GET /control/stats/config` | query, blocchi, risposte upstream, finestra di statistiche |
| `GET /control/filtering/status` | liste di filtri |
| `POST /control/version.json` con `{"recheck_now": false}` | il risultato dell'ultimo controllo versione, come lo legge l'interfaccia web a ogni caricamento di pagina; non forza mai un nuovo controllo |
| `GET /control/dns_info`, poi `POST /control/test_upstream_dns` una volta per upstream | gli upstream configurati e i server bootstrap, poi il test upstream; non modifica alcuna impostazione |
