# Nextcloud

Se Nextcloud sta servendo o è bloccato: la modalità di manutenzione è rimasta attiva,
un aggiornamento del database è in attesa dopo un aggiornamento, gli aggiornamenti di
Nextcloud e delle app, gli utenti attivi, lo spazio libero per la directory dei dati,
la dimensione del database e l'OPcache di PHP.

DumbMonit legge il pubblico `status.php` e le statistiche dell'app **serverinfo**,
che viene fornita con Nextcloud, con un **token di monitoraggio** (`NC-Token`).
Il token apre quelle statistiche e niente altro: non può accedere, elencare utenti,
aprire un file o modificare un'impostazione.

Il fallimento che vale la pena osservare: **un aggiornamento che non ha finito**.
Dopo un aggiornamento Nextcloud può rimanere in modalità di manutenzione, o aspettare
`occ upgrade`; la pagina web mostra allora un avviso che nessuno vede finché qualcuno
non tenta di sincronizzare. Entrambi vengono letti da `status.php`, che risponde anche allora.

## Cosa monitora

Tutte le metriche hanno il prefisso `dumbmonit_nextcloud_`.

| Metrica | Cosa | Etichette |
|---|---|---|
| `maintenance` | 1 mentre la modalità di manutenzione è attiva | |
| `needs_db_upgrade` | 1 mentre Nextcloud aspetta `occ upgrade` | |
| `version_info` | valore 1 | `version` |
| `update_available` | 1 quando il controllo di aggiornamento di Nextcloud ha trovato una versione più recente | `available_version` |
| `apps_installed`, `app_updates_available` | app installate, e quante hanno una versione più recente nell'app store | |
| `app_update_available` | valore 1 per app con un aggiornamento (al massimo 32) | `app`, `available_version` |
| `active_users` | utenti visti negli ultimi 5 minuti, ora e giorno | `window` (`5m`, `1h`, `24h`) |
| `users`, `files`, `shares` | account, file e condivisioni sull'istanza | |
| `free_space_bytes` | spazio libero dove Nextcloud mantiene i suoi dati | |
| `database_size_bytes` | dimensione del database | |
| `platform_info` | valore 1 | `database`, `database_version`, `php_version` |
| `php_memory_limit_bytes` | PHP `memory_limit` | |
| `opcache_enabled`, `opcache_full` | OPcache acceso; OPcache fuori spazio | |
| `opcache_memory_used_percent`, `opcache_interned_strings_used_percent`, `opcache_hit_rate_percent`, `opcache_oom_restarts` | quanto è pieno OPcache e quanto bene serve | |
| `fpm_active_processes`, `fpm_total_processes`, `fpm_listen_queue`, `fpm_max_children_reached`, `fpm_slow_requests` | Pool PHP-FPM, quando Nextcloud esegue dietro PHP-FPM (immagini `-fpm`, AIO) | |

Mentre Nextcloud è in manutenzione o aspetta un aggiornamento, la sua API risponde con
503: solo `maintenance`, `needs_db_upgrade` e `version_info` vengono scritti allora,
e il monitor non viene segnalato come irraggiungibile.

Le [regole incorporate](../alerting/rules.md#self-hosted-applications) che si applicano:
Nextcloud bloccato in manutenzione, Nextcloud aggiornamento del database in sospeso,
Nextcloud aggiornamento disponibile, Nextcloud OPcache pieno, Nextcloud spazio libero
basso, più Device irraggiungibile.

## Lavori in background (cron)

Nextcloud non dice al token di monitoraggio quando i lavori in background sono stati
eseguiti per l'ultima volta: solo un account con ogni diritto sull'istanza può leggerlo,
e DumbMonit non ne chiede uno. Osserva cron con un [Heartbeat](push.md) invece: crea
uno in DumbMonit, poi chiama il suo URL alla fine della linea cron che esegue i lavori
in background di Nextcloud, così chiama solo quando `cron.php` ha avuto successo:

```
*/5 * * * * php -f /var/www/html/cron.php && curl -fsS -m 10 --retry 3 https://monit.example.com/api/push/<token> > /dev/null
```

Con un intervallo previsto di 5 minuti, l'heartbeat avvisa non appena cron si ferma,
qualunque sia il motivo (un contenitore riavviato senza il suo cron, un errore PHP,
lavori in background rimasti su AJAX).

## Dispositivi client

Nextcloud non espone un elenco di client di sincronizzazione o dispositivi mobili
a livello di amministratore attraverso alcuna API. La pagina personale "Dispositivi e
sessioni" (Impostazioni → Sicurezza) elenca solo l'account attualmente acceduto,
supportato da una sessione del browser piuttosto che da un token API, e l'unico
equivalente a livello di amministrazione è la riga di comando (`occ user:auth-tokens:list <user>`,
un account alla volta) — non qualcosa che il token di monitoraggio può raggiungere, e
non qualcosa che DumbMonit chiede a un amministratore di scrivere in uno script di sonda.
L'API serverinfo che DumbMonit legge non ha nemmeno un elenco di dispositivi: fornisce
conteggi di utenti attivi (`active_users`, sopra), mai chi o cosa sono.

Questo è un limite deliberato, non una funzione mancante: dare a DumbMonit un elenco
di dispositivi significherebbe ampliare il token di monitoraggio in un account di
amministrazione completo, che il resto di questa pagina si sforza di evitare. Se Nextcloud
espone mai questo su OCS, DumbMonit seguirà lo stesso modello [dispositivi client](../alerting/rules.md)
già usato per Immich, Proxmox Backup Server e Veeam.

## La pagina del monitor

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire la pagina
non chiama mai Nextcloud. Dice cosa c'è che non va prima (manutenzione, aggiornamento
in attesa, OPcache pieno o spento), poi gli aggiornamenti, poi gli utenti attivi, lo
spazio libero, la dimensione del database, gli utenti, i file e le figure di OPcache,
poi le app da aggiornare per nome e la piattaforma (database, PHP).

## Dai a Nextcloud un token di monitoraggio

1. L'app serverinfo di Nextcloud, fornita e abilitata con Nextcloud, risponde a un token di monitoraggio che apre le sue statistiche e niente altro: nessun file, nessun utente, nessuna condivisione. Sul server Nextcloud, imposta un token casuale lungo e stampalo indietro. Esegui occ come l'utente del server web; in Docker, metti docker exec -u www-data e il nome del contenitore davanti.

    ```
    php occ config:app:set serverinfo token --value "$(openssl rand -hex 32)"
    php occ config:app:get serverinfo token
    ```

2. Se occ dice che serverinfo è disabilitato, abilitalo prima. Proprio dopo che il token è impostato, Nextcloud può rifiutare la prima lettura stessa (401) prima che abbia effetto: DumbMonit ritenta una volta dopo due secondi, quindi un target completamente nuovo non mostra un errore spurio "token rifiutato".

    ```
    php occ app:enable serverinfo
    ```

3. In DumbMonit, inserisci l'indirizzo con cui apri Nextcloud, ad esempio "cloud.example.com" (HTTPS su 443) o "http://10.0.0.5:8080", e incolla il token. L'host deve essere uno dei domini affidabili di Nextcloud, o Nextcloud rifiuta la richiesta.

4. DumbMonit legge solo status.php e le statistiche serverinfo. Non apre mai un file, non elenca mai utenti e non modifica mai un'impostazione, e il token non può accedere da nessuna parte.

!!! warning
    Nextcloud non dice al token quando i lavori in background (cron) sono stati eseguiti per l'ultima volta: solo un account con ogni diritto sull'istanza può leggerlo. Per essere avvisato quando cron si ferma, aggiungi un Heartbeat in DumbMonit e chiama il suo URL alla fine della linea cron, come mostra la documentazione.

Con Nextcloud AIO, esegui occ nel contenitore `nextcloud-aio-nextcloud`:
`docker exec -u www-data nextcloud-aio-nextcloud php occ …`. Perché non un
gruppo di amministrazione delegato: Nextcloud apre solo l'API serverinfo al token
o ai membri del gruppo `admin` completo, e la delega non lo cambia.

Controllando il token manualmente, dall'host DumbMonit:

```
curl -H 'NC-Token: YOUR-TOKEN' 'https://cloud.example.com/ocs/v2.php/apps/serverinfo/api/v1/info?format=json'
```

## Credenziali

| Credenziale | Campi |
|---|---|
| Token di monitoraggio (NC-Token) | Il token serverinfo. DumbMonit lo invia nell'intestazione `NC-Token`, con `OCS-APIRequest: true`. |

Indirizzo: un nome host (`cloud.example.com`), `host:port`, o un URL completo con
un prefisso di percorso (`https://example.com/nextcloud`); un finale `/index.php` o
`/login` è accettato. Opzioni: protocollo (HTTPS per impostazione predefinita), porta (443),
verifica del certificato e timeout della richiesta (15 s).

Il conteggio degli aggiornamenti dell'app chiede la cache dell'app store di Nextcloud;
su un'istanza che non può raggiungere l'app store, quel conteggio rimane a 0.

Convalidato contro Nextcloud 35.0.1 (immagine ufficiale).
