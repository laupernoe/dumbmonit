# Pi-hole

Il blocca-pubblicità DNS: se sta bloccando, quante query risponde e blocca,
quanto sono vecchie le sue blocklist, se i suoi resolver upstream falliscono,
se un aggiornamento è in attesa, e i messaggi della sua pagina di diagnosi.

DumbMonit legge l'API REST di Pi-hole v6 (`/api`). Pi-hole 5 e precedenti non
hanno tale API e non sono supportati.

Ciò che va storto con un Pi-hole raramente si vede: il DNS continua a
rispondere. Il blocco viene sospeso "per cinque minuti" e non torna mai
attivo, l'aggiornamento settimanale della blocklist (gravity) fallisce da un
mese, un resolver upstream risponde SERVFAIL, e la pagina di diagnosi che
nessuno apre ha sempre elencato il motivo.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_pihole_`. Pi-hole conta le
query sulle ultime 24 ore, una finestra scorrevole che sale e scende: quei
valori sono gauge, con suffisso `_24h`, non contatori.

| Metrica | Cosa | Etichette |
|---|---|---|
| `blocking_enabled` | 1 quando il blocco è attivo; 0 quando è disattivato, sospeso o fallito | |
| `blocking_timer_seconds` | tempo rimanente prima che un blocco sospeso si riattivi da solo | |
| `queries_24h`, `queries_blocked_24h`, `queries_blocked_percent` | query risposte e bloccate nelle ultime 24 ore | |
| `queries_forwarded_24h`, `queries_cached_24h`, `unique_domains_24h` | query inviate upstream, risposte dalla cache, domini distinti | |
| `queries_per_second` | media sull'ultimo minuto | |
| `replies_servfail_24h` | risposte SERVFAIL nelle ultime 24 ore: un resolver upstream che fallisce si vede prima qui | |
| `clients_active`, `clients_total` | client visti | |
| `gravity_domains` | domini sulle blocklist | |
| `gravity_last_update_timestamp_seconds`, `gravity_age_seconds` | ultima ricostruzione della blocklist, e da quanto tempo | |
| `version_info` | valore 1 | `core`, `web`, `ftl`, `docker` |
| `update_available` | 1 quando una versione più recente di un componente è pubblicata, 0 altrimenti; assente finché il controllo giornaliero di Pi-hole non è stato eseguito | `component` (`core`, `web`, `ftl`, `docker`) |
| `updates_available` | aggiornamenti da installare: l'immagine in Docker (core, web e FTL ci arrivano insieme), altrimenti ogni componente non aggiornato | |
| `messages`, `messages_by_type` | messaggi sulla pagina di diagnosi | `type` (`LOAD`, `RATE_LIMIT`, `LIST`…) |
| `ftl_uptime_seconds`, `ftl_memory_percent`, `ftl_cpu_percent` | il daemon FTL | |
| `privacy_level` | il livello di privacy di Pi-hole, da 0 a 3 | |
| `scrape_errors` | chiamate API secondarie fallite durante l'ultimo controllo | |

Su un branch di sviluppo, Pi-hole confronta i commit invece dei numeri di
versione, e così fa anche DumbMonit.

Le [regole integrate](../alerting/rules.md#pi-hole) che si applicano:
Pi-hole blocking disabled, Pi-hole blocklists stale, Pi-hole update
available, Pi-hole diagnosis messages, più Device unreachable.

## La pagina del dispositivo

Il pannello sopra i grafici legge ciò che il controllo ha memorizzato;
aprire la pagina non si connette mai a Pi-hole. Mostra se il blocco è attivo
(e quando finisce una pausa), le query e la percentuale bloccata nelle
ultime 24 ore, la dimensione e l'età della blocklist, i client, le versioni
con eventuali aggiornamenti in attesa, e il numero di messaggi di diagnosi.

## Creare una app password in Pi-hole

1. È richiesto Pi-hole v6 o successivo: la versione 5 non aveva API REST. Pi-hole non ha un account di sola lettura; il più vicino è una app password, che non può modificare alcuna impostazione.

2. Nell'interfaccia web di Pi-hole, apri Settings → Web interface / API e passa la pagina da Basic a Expert. Clicca Configure app password, copia la password mostrata, poi clicca Enable new app password. Pi-hole la mostra solo una volta, e una nuova sostituisce la precedente.

3. Una app password resta di sola lettura per le impostazioni finché webserver.api.app_sudo è false, il valore predefinito. Verificalo sulla macchina Pi-hole (in Docker, anteponi al comando docker exec e il nome del container):

    ```
    pihole-FTL --config webserver.api.app_sudo
    ```

4. In DumbMonit, inserisci l'indirizzo Pi-hole, per esempio "pi.hole" oppure "192.168.1.53", e incolla la app password. Per un Pi-hole la cui interfaccia web non ha password, scegli invece No password.

5. DumbMonit legge soltanto. Non sospende mai il blocco e non modifica mai una lista, e mantiene una sessione API aperta tra i controlli invece di aprirne una per controllo: Pi-hole consente 16 sessioni contemporanee.

!!! warning
    La app password consente comunque di sospendere il blocco e modificare le liste tramite l'API: riservala solo a DumbMonit. Pi-hole serve HTTP semplice sulla porta 80, e la password attraversa la rete ogni volta che una sessione si apre: su una rete non affidabile, usa HTTPS sulla porta 443.

Una app password effettua anche l'accesso senza il secondo fattore quando
l'autenticazione a due fattori è attiva: è proprio a questo che serve. Per
revocarla, svuota `webserver.api.app_pwhash` sotto Settings → All settings,
oppure attivane una nuova: la precedente smette di funzionare all'istante.

## Credenziali

| Credenziale | Campi |
|---|---|
| App password | La password generata sopra. DumbMonit la scambia per una sessione, inviata nell'header `X-FTL-SID`, e riutilizza quella sessione finché Pi-hole la mantiene (30 minuti dopo il suo ultimo uso). |
| No password | Solo per un Pi-hole la cui interfaccia web non ha password. |

Indirizzo: un nome host o IP (`pi.hole`), `host:porta`, oppure un URL
completo (`https://pi.hole`); il percorso dell'interfaccia web o dell'API,
copiato dal browser, viene ignorato. Opzioni: protocollo (HTTP per
impostazione predefinita), porta (80), verifica del certificato (per HTTPS
su 443, il certificato proprio di Pi-hole è autofirmato) e timeout della
richiesta (10 s).

Se i controlli falliscono con "no free API session", le sessioni di Pi-hole
sono tutte occupate, di solito da uno script che accede a ogni esecuzione
senza disconnettersi. Scadono dopo 30 minuti; `webserver.api.max_sessions`
alza il limite.
