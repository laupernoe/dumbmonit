# Apache httpd

Il server web: worker occupati e inattivi, lo scoreboard dei worker per
stato, e i tassi di richieste/byte quando ExtendedStatus è attivo.

DumbMonit legge il `mod_status` integrato di Apache, interrogato con `?auto`
così risponde in semplici righe `Key: Value` invece della pagina HTML
pensata per un browser.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_apache_`. Le metriche
`_total` sono contatori: grafici e regole le trasformano in tassi. Tutto
sotto `workers_busy`/`workers_idle` e `scoreboard` richiede
`ExtendedStatus On`.

| Metrica | Cosa | Etichette |
|---|---|---|
| `workers_busy`, `workers_idle` | processi o thread worker, per stato | |
| `scoreboard` | slot worker per stato dello scoreboard | `state` |
| `requests_total` | richieste servite dall'avvio di Apache | |
| `sent_bytes_total` | byte inviati dall'avvio di Apache | |
| `uptime_seconds` | tempo trascorso dall'avvio di Apache | |
| `requests_per_second`, `bytes_per_second`, `bytes_per_request` | medie correnti proprie di Apache | |
| `cpu_load_ratio` | valore `CPULoad` proprio di Apache | |
| `connections_total`, `connections_async_writing`, `connections_async_keepalive`, `connections_async_closing` | conteggi di connessioni asincrone (MPM event e worker) | |
| `version_info` | presenza; l'header di risposta `Server` | `version` |

`scoreboard` somma sempre al numero configurato di slot worker: gli stati
sono waiting, open_slot, starting, reading, sending, keepalive, dns_lookup,
closing, logging, finishing e idle_cleanup.

## La pagina del monitor

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire la
pagina non interroga mai Apache. Mostra worker occupati e inattivi, la
suddivisione dello scoreboard, e, con ExtendedStatus, i tassi di richieste e
byte sulle ultime 24 ore.

## Abilitare mod_status di Apache

1. Assicurati che mod_status sia caricato (è incluso con Apache e di solito è abilitato per impostazione predefinita).

    ```
    a2enmod status
    ```

2. Aggiungi una location di stato, limitata all'host DumbMonit; adatta l'indirizzo di conseguenza. Su Debian e Ubuntu questo va in un file proprio, altrove nella configurazione principale.

    ```
    <Location "/server-status">
    SetHandler server-status
    Require ip 10.0.0.5
    </Location>
    ```

3. Attiva i contatori di richieste e byte (opzionale, ma utile): senza di esso, vengono riportati solo i worker occupati/inattivi e lo scoreboard.

    ```
    ExtendedStatus On
    ```

4. Ricarica Apache e verifica che risponda prima di configurare DumbMonit.

    ```
    apachectl graceful
    curl -s 'http://apache.lan/server-status?auto'
    ```

5. In DumbMonit, inserisci l'host Apache, per esempio "apache.lan", e il percorso se non è "/server-status" (DumbMonit aggiunge da solo "?auto").

!!! warning
    Senza ExtendedStatus, richieste e byte non vengono riportati affatto, non
    come zero: il grafico resta vuoto invece di mentire a quota piatta.
    Apache non pubblica la propria versione in mod_status; DumbMonit la
    legge invece dall'header di risposta Server, che alcune distribuzioni
    riducono di proposito.

Su Debian e Ubuntu, la location di stato di solito esiste già in
`/etc/apache2/mods-available/status.conf`; modificare lì la riga `Require` è
sufficiente, nessun nuovo file necessario. Dove la configurazione è un
singolo file (la maggior parte delle altre distribuzioni), aggiungi invece
il blocco al virtual host principale.

## Credenziali

| Credenziale | Campi |
|---|---|
| No password | Nessuna: la pagina di stato è filtrata così che solo DumbMonit la raggiunga. |
| Basic auth | L'utente e la password del blocco che protegge la pagina di stato, se presente. |

Indirizzo: un nome host o IP, con la porta se non è 80. Opzioni: protocollo
(HTTP per impostazione predefinita), porta, percorso di stato
(`/server-status` per impostazione predefinita, `?auto` aggiunto
automaticamente), verifica del certificato e timeout della richiesta
(10 s).
