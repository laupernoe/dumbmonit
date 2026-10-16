# Heartbeat (push)

Il guasto silenzioso che nessuno nota: il backup notturno che ha smesso di
funzionare dopo un aggiornamento, il cron di rinnovo del certificato che ha
perso i suoi permessi, l'automazione di Home Assistant che si è rotta dopo
una rinomina. Nulla si lamenta, e lo scopri il giorno in cui hai bisogno del
backup.

Un monitor **heartbeat** (chiamato anche *dead man's switch*, o un monitor
*push* in Uptime Kuma) ribalta la logica: DumbMonit non contatta nulla. Il
job chiama un URL segreto ogni volta che viene eseguito, ed è l'**assenza**
di una chiamata che genera l'avviso.

## Come funziona

Ogni monitor heartbeat riceve un URL della forma
`https://<il tuo DumbMonit>/api/push/<token>`. Il token è un valore casuale
a 128 bit, memorizzato come hash per la ricerca e cifrato per la
visualizzazione: viene mostrato sulla pagina del monitor, con un
pulsante per copiarlo, per tutto il tempo che il monitor esiste. È
deliberatamente aperto — nessuna sessione, nessun token API, nessun header
`X-Requested-With` — perché una riga `curl` in un crontab non ha nulla di
tutto ciò. Avere il token permette solo di dire "il job è stato eseguito".

A ogni passaggio dello scheduler (l'intervallo del monitor, 60 s per
impostazione predefinita) il collector legge l'orario dell'ultima chiamata
e scrive:

| Situazione | `probe_success` | Effetto |
|---|---|---|
| Nessuna chiamata ancora ricevuta | *(nulla scritto)* | Il monitor mostra **Waiting**. Nessun avviso: il job non è ancora attivo. |
| Ultima chiamata entro l'intervallo atteso + grazia | `1` | **Reporting**. |
| Ultima chiamata più vecchia dell'intervallo atteso + grazia | `0`, `reason="missed"` | **Down**; la regola "Heartbeat missed" si attiva dopo due minuti. |
| L'ultima chiamata ha detto `status=down` | `0`, `reason="reported_down"` | **Down** all'istante, qualunque cosa dica l'orologio. |

Una chiamata ri-valuta anche immediatamente il monitor, così un job che
riprende elimina il suo avviso senza attendere il prossimo passaggio dello
scheduler.

L'endpoint risponde `204` con un corpo vuoto quando la chiamata è
registrata, `404` per un token sconosciuto o rigenerato, `400` per uno
`status` che non è né `up` né `down`, e `429` (con `Retry-After`) oltre
sessanta chiamate al minuto per un singolo token. Sia `GET` che `POST` sono
accettati; `HEAD` funziona anche.

### Parametri della query

| Parametro | Significato |
|---|---|
| `status` | `up` (predefinito) o `down`: il job segnala da solo un fallimento. |
| `msg` | Una parola libera di spiegazione, conservata (troncata a 500 caratteri) e mostrata sulla pagina del monitor. |

Questi sono i parametri del monitor push di Uptime Kuma: uno script
scritto per esso funziona senza modifiche.

## Configurazione

I passaggi seguenti sono quelli mostrati dalla nota accanto al modulo.

1. Nell'indirizzo, scrivi un'etichetta breve per il job ("nightly-backup",
   "certbot-renew"): nulla viene contattato, l'etichetta deve solo essere
   unica tra i tuoi heartbeat.
2. Imposta l'intervallo atteso sulla pianificazione del job ("24h" per un
   job notturno, "1h" per uno orario) e, se la pianificazione varia, un
   periodo di grazia più ampio. Salva il monitor.
3. La pagina del monitor mostra l'URL da chiamare. Aggiungilo alla fine
   del job, così viene chiamato solo quando il job è riuscito:

    ```
    curl -fsS -m 10 --retry 3 https://monit.example.com/api/push/<token>
    ```

4. Un job che sa quando è fallito può segnalarlo invece di restare
   silenzioso: aggiungi ?status=down&msg=… all'URL, e l'avviso si attiva
   all'istante.
5. Finché la prima chiamata non arriva il monitor mostra "Waiting" e
   nulla viene avvisato. URL perso o compromesso? "Regenerate" sulla pagina
   del monitor emette uno nuovo; il vecchio smette di rispondere
   immediatamente.

!!! warning
    Chiama l'URL alla fine del job, dopo la parte che conta. Una chiamata
    posta all'inizio segnalerebbe un successo anche quando il backup stesso
    è fallito.

Credenziali: nessuna.

### Esempi

Una riga di crontab, ogni notte alle 03:00, che chiama solo quando il
backup è riuscito:

```
0 3 * * * /usr/local/bin/backup.sh && curl -fsS -m 10 --retry 3 https://monit.example.com/api/push/<token> > /dev/null
```

Uno script che segnala il proprio fallimento, con il codice di uscita come
messaggio:

```sh
#!/bin/sh
URL="https://monit.example.com/api/push/<token>"
if /usr/local/bin/backup.sh; then
    curl -fsS -m 10 --retry 3 "$URL" > /dev/null
else
    curl -fsS -m 10 --retry 3 "$URL?status=down&msg=backup+exit+$?" > /dev/null
fi
```

Home Assistant, un `rest_command` chiamato alla fine di un'automazione:

```yaml
rest_command:
  dumbmonit_heartbeat:
    url: "https://monit.example.com/api/push/<token>"
    method: post
```

Windows Task Scheduler (PowerShell):

```powershell
Invoke-WebRequest -UseBasicParsing -TimeoutSec 10 "https://monit.example.com/api/push/<token>" | Out-Null
```

### Opzioni

| Chiave | Etichetta | Predefinito | Aiuto |
|---|---|---|---|
| `expected_interval` | Expected interval | `24h` | Ogni quanto il job dovrebbe chiamare: 30m, 1h, 6h, 24h, 7d (oppure un numero di secondi). Una chiamata mancata viene dichiarata una volta trascorso questo intervallo più il periodo di grazia. |
| `grace` | Grace period | `10%` | Tempo extra tollerato dopo l'intervallo atteso prima che l'heartbeat conti come mancato: una percentuale dell'intervallo (10%) o una durata fissa (15m). Mai meno di un minuto. |

Le durate accettano i suffissi `s`, `m`, `h`, `d`, `w` e possono essere
combinate (`1h30m`); un numero semplice è un numero di secondi. L'intervallo
atteso deve essere tra 10 secondi e un anno.

## Metriche

Tutte etichettate `probe="push"` oltre a `target`, `host` e `tag_*`, tutte
con prefisso `dumbmonit_`.

| Metrica | Significato |
|---|---|
| `probe_success` | 1 se l'ultima chiamata è in orario e non ha segnalato un fallimento, 0 altrimenti. Assente finché la prima chiamata non arriva. |
| `probe_failure_info` | Presenza (1) con un'etichetta `reason`: `missed` o `reported_down`. |
| `push_last_seen_seconds` | Età dell'ultima chiamata. |
| `push_received_total` | Chiamate ricevute da quando il token è stato creato (contatore). |

Disponibilità su trenta giorni, come per qualsiasi monitor di servizio:
`avg_over_time(dumbmonit_probe_success{probe="push"}[30d])`.

Regola integrata che si applica: **Heartbeat missed** (advisory, 2 minuti).
Le regole "Service down" e "Service flapping" saltano gli heartbeat, e
"Monitor unreachable" non si attiva mai per loro: il collector funziona
sempre, solo il suo verdetto cambia.

## API

| Rotta | Sessione | Scopo |
|---|---|---|
| `GET` o `POST /api/push/<token>` | nessuna | La chiamata stessa. `204`, `404`, `400` o `429`. |
| `GET /api/targets/<id>/push` | richiesta | Il monitor: `token`, `path`, `last_seen_at`, `last_seen_age_secs`, `last_status`, `last_message`, `received_total`, `expected_interval_secs`, `grace_secs`, `verdict` (`waiting`, `on_time`, `missed`, `reported_down`), `settings_error`. Creato alla prima lettura se manca. |
| `POST /api/targets/<id>/push/regenerate` | admin | Nuovo token; l'URL precedente risponde `404` da quel momento. Il contatore e l'ultima chiamata sono conservati. |
