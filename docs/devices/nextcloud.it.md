# Nextcloud

Se Nextcloud sta servendo o è bloccato: modalità manutenzione lasciata
attiva, un aggiornamento del database in attesa dopo un update,
aggiornamenti di Nextcloud e delle app, utenti attivi, spazio libero per
la directory dati, dimensione del database, e l'OPcache di PHP.

DumbMonit legge il pubblico `status.php` e le statistiche dell'app
**serverinfo**, che viene fornita con Nextcloud, con un **token di
monitoraggio** (`NC-Token`). Il token apre quelle statistiche e
nient'altro: non può accedere, elencare utenti, aprire un file o
modificare un'impostazione.

Il guasto da tenere d'occhio: **un aggiornamento che non è terminato**.
Dopo un aggiornamento Nextcloud può restare in modalità manutenzione,
oppure attendere `occ upgrade`; la pagina web mostra allora un avviso che
nessuno vede finché qualcuno non prova a sincronizzare. Entrambi vengono
letti da `status.php`, che risponde anche in quel caso.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_nextcloud_`.

| Metrica | Cosa | Etichette |
|---|---|---|
| `maintenance` | 1 mentre la modalità manutenzione è attiva | |
| `needs_db_upgrade` | 1 mentre Nextcloud attende `occ upgrade` | |
| `version_info` | valore 1 | `version` |
| `update_available` | 1 quando il controllo aggiornamenti proprio di Nextcloud ha trovato una release più recente | `available_version` |
| `apps_installed`, `app_updates_available` | app installate, e quante hanno una versione più recente nello store delle app | |
| `app_update_available` | valore 1 per app con un aggiornamento (al massimo 32) | `app`, `available_version` |
| `active_users` | utenti visti negli ultimi 5 minuti, ora e giorno | `window` (`5m`, `1h`, `24h`) |
| `users`, `files`, `shares` | account, file e condivisioni sull'istanza | |
| `free_space_bytes` | spazio libero dove Nextcloud conserva i suoi dati | |
| `database_size_bytes` | dimensione del database | |
| `platform_info` | valore 1 | `database`, `database_version`, `php_version` |
| `php_memory_limit_bytes` | `memory_limit` di PHP | |
| `opcache_enabled`, `opcache_full` | OPcache attivo; OPcache senza spazio | |
| `opcache_memory_used_percent`, `opcache_interned_strings_used_percent`, `opcache_hit_rate_percent`, `opcache_oom_restarts` | quanto è pieno OPcache e quanto bene serve | |
| `fpm_active_processes`, `fpm_total_processes`, `fpm_listen_queue`, `fpm_max_children_reached`, `fpm_slow_requests` | pool PHP-FPM, quando Nextcloud funziona dietro PHP-FPM (le immagini `-fpm`, AIO) | |

Mentre Nextcloud è in manutenzione o attende un aggiornamento, la sua API
risponde 503: solo `maintenance`, `needs_db_upgrade` e `version_info`
vengono scritti allora, e il dispositivo non viene segnalato
irraggiungibile.

Le [regole integrate](../alerting/rules.md#self-hosted-applications) che
si applicano: Nextcloud stuck in maintenance, Nextcloud database upgrade
pending, Nextcloud update available, Nextcloud OPcache full, Nextcloud
free space low, più Device unreachable.

## Job in background (cron)

Nextcloud non comunica al token di monitoraggio quando i job in
background sono stati eseguiti l'ultima volta: solo un account con ogni
diritto sull'istanza può leggerlo, e DumbMonit non ne chiede uno. Osserva
cron con un [Heartbeat](push.md) invece: creane uno in DumbMonit, poi
chiama il suo URL alla fine della riga cron che esegue i job in
background di Nextcloud, così chiama solo quando `cron.php` è riuscito:

```
*/5 * * * * php -f /var/www/html/cron.php && curl -fsS -m 10 --retry 3 https://monit.example.com/api/push/<token> > /dev/null
```

Con un intervallo atteso di 5 minuti, l'heartbeat avvisa non appena cron
si ferma, qualunque sia il motivo (un container riavviato senza il suo
cron, un errore PHP, job in background lasciati su AJAX).

## Dispositivi client

Nextcloud non espone un elenco a livello di amministrazione dei client di
sincronizzazione o dei dispositivi mobili tramite alcuna API. La pagina
personale "Devices & sessions" (Settings → Security) elenca solo
l'account attualmente connesso, basata su una sessione browser invece che
su un token API, e l'unico equivalente a livello amministrazione è la
riga di comando (`occ user:auth-tokens:list <user>`, un account alla
volta) — non qualcosa che il token di monitoraggio può raggiungere, e non
qualcosa che DumbMonit chiede a un amministratore di scriptare in una
sonda. Anche l'API `serverinfo` che DumbMonit legge non ha un elenco di
dispositivi: dà conteggi di utenti attivi (`active_users`, sopra), mai
chi o cosa sono.

Questo è un limite deliberato, non una funzionalità mancante: dare a
DumbMonit un elenco di dispositivi significherebbe ampliare il token di
monitoraggio in un account di amministrazione completo, cosa che il
resto di questa pagina evita apposta. Se Nextcloud dovesse mai esporre
questo tramite OCS, DumbMonit seguirà lo stesso schema
[dispositivi client](../alerting/rules.md) già usato per Immich, Proxmox
Backup Server e Veeam.

## La pagina del dispositivo

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire
la pagina non chiama mai Nextcloud. Indica prima cosa non va
(manutenzione, aggiornamento in attesa, OPcache pieno o disattivato), poi
gli aggiornamenti, poi utenti attivi, spazio libero, dimensione del
database, utenti, file e valori OPcache, poi le app da aggiornare per
nome e la piattaforma (database, PHP).

## Dare a Nextcloud un token di monitoraggio

1. L'app serverinfo di Nextcloud, fornita e abilitata con Nextcloud, risponde a un token di monitoraggio che apre le sue statistiche e nient'altro: nessun file, nessun utente, nessuna condivisione. Sul server Nextcloud, imposta un token lungo e casuale e stampalo. Esegui occ come l'utente del server web; in Docker, metti davanti docker exec -u www-data e il nome del container.

    ```
    php occ config:app:set serverinfo token --value "$(openssl rand -hex 32)"
    php occ config:app:get serverinfo token
    ```

2. Se occ dice che serverinfo è disattivata, abilitala prima.

    ```
    php occ app:enable serverinfo
    ```

3. In DumbMonit, inserisci l'indirizzo con cui apri Nextcloud, per esempio "cloud.example.com" (HTTPS su 443) oppure "http://10.0.0.5:8080", e incolla il token. L'host deve essere uno dei domini fidati di Nextcloud, altrimenti Nextcloud rifiuta la richiesta.

4. DumbMonit legge solo status.php e le statistiche serverinfo. Non apre mai un file, non elenca mai utenti e non modifica mai un'impostazione, e il token non può accedere da nessuna parte.

!!! warning
    Nextcloud non comunica al token quando i job in background (cron) sono stati eseguiti l'ultima volta: solo un account con ogni diritto sull'istanza può leggerlo. Per essere avvisato quando cron si ferma, aggiungi un Heartbeat in DumbMonit e chiama il suo URL alla fine della riga cron, come mostra la documentazione.

Con Nextcloud AIO, esegui occ nel container `nextcloud-aio-nextcloud`:
`docker exec -u www-data nextcloud-aio-nextcloud php occ …`. Perché non un
gruppo di amministrazione delegata: Nextcloud apre l'API serverinfo solo
al token o ai membri del gruppo `admin` completo, e la delega non cambia
questo.

Verificare il token a mano, dall'host DumbMonit:

```
curl -H 'NC-Token: YOUR-TOKEN' 'https://cloud.example.com/ocs/v2.php/apps/serverinfo/api/v1/info?format=json'
```

## Credenziali

| Credenziale | Campi |
|---|---|
| Monitoring token (NC-Token) | Il token serverinfo. DumbMonit lo invia nell'header `NC-Token`, con `OCS-APIRequest: true`. |

Indirizzo: un nome host (`cloud.example.com`), `host:porta`, oppure un URL
completo con un prefisso di percorso (`https://example.com/nextcloud`);
un `/index.php` o `/login` finale viene accettato. Opzioni: protocollo
(HTTPS per impostazione predefinita), porta (443), verifica del
certificato e timeout della richiesta (15 s).

Il conteggio degli aggiornamenti delle app chiede la cache dell'app store
di Nextcloud; su un'istanza che non può raggiungere l'app store, quel
conteggio resta a 0.

Validato su Nextcloud 35.0.1 (immagine ufficiale).
