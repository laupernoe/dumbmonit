# Sorvegliare DumbMonit stesso

Chi sorveglia il sorvegliante? DumbMonit risponde in due parti:

- un **heartbeat in uscita**, per ciò che non può vedere dall'interno: l'host
  spento, il container fermo, il motore bloccato;
- **regole integrate** sulla propria salute, per ciò che può vedere: il
  volume dei dati che si riempie, il database delle metriche che si riavvia in
  loop, i poll che si bloccano, le scritture che falliscono, il database che
  cresce in modo anomalo.

Entrambi si trovano in **Impostazioni → Sistema → DumbMonit stesso**, che
mostra anche lo stato del motore in questo momento.

## L'heartbeat in uscita

DumbMonit chiama un tuo URL a intervalli regolari (60 secondi per impostazione
predefinita), **solo finché il motore sta bene**:

- il pianificatore è in esecuzione (ha completato un giro negli ultimi due
  minuti);
- il database SQLite risponde;
- VictoriaMetrics risponde.

Quando uno di essi fallisce, non viene inviato nulla. Un servizio "dead man's
switch" attende queste chiamate e ti avvisa quando si interrompono, che la
causa sia l'intero host, il container o un solo componente.

### Configurarlo

1. Crea un check sul servizio che preferisci e copia il suo URL di ping:
   - **Healthchecks.io** (o un Healthchecks auto-ospitato): un check con un
     periodo pari all'intervallo e un tempo di tolleranza un po' più lungo
     (`https://hc-ping.com/<uuid>`);
   - **Uptime Kuma**: un monitor di tipo *Push*
     (`https://kuma.example.com/api/push/<token>?status=up&msg=OK&ping=`);
   - **Better Stack**: un heartbeat (`https://uptime.betterstack.com/api/v1/heartbeat/<token>`);
   - qualsiasi servizio che accetti un semplice `GET` e avvisi quando smette
     di arrivare (Cronitor, Dead Man's Snitch…).
2. In **Impostazioni → Sistema → DumbMonit stesso**, incolla l'URL, scegli
   l'intervallo (da 30 a 3600 secondi), attiva **Invia un heartbeat** e salva.
3. Premi **Prova**: una chiamata parte subito e il risultato viene mostrato.
   Il servizio dovrebbe segnare il check come attivo.

La chiamata è un `GET` con un timeout di 10 secondi. Segue la stessa
protezione dei canali di notifica: non viene mai inviata all'interfaccia web
di DumbMonit stesso né al suo VictoriaMetrics.

!!! note "ntfy"
    ntfy consegna ogni messaggio che riceve, quindi puntare l'heartbeat a un
    topic ntfy ti invia una notifica ogni minuto invece che quando le chiamate
    si interrompono. Usa un servizio dead man's switch, e tieni ntfy come
    canale per gli avvisi veri e propri.

### Segnale di guasto

Per impostazione predefinita, un motore malato smette semplicemente di
chiamare, e il servizio avvisa una volta trascorso il suo tempo di tolleranza.
Attiva **Segnala i guasti** per chiamare invece l'URL di errore, che il
servizio segnala subito:

| Servizio | URL di errore |
|---|---|
| Healthchecks.io, Better Stack | `/fail` aggiunto al percorso: `https://hc-ping.com/<uuid>/fail` |
| Uptime Kuma | `status=up` sostituito da `status=down` nella query |

Lascialo disattivato per qualsiasi altro servizio: un URL sconosciuto con
`/fail` in fondo risponderebbe con ogni probabilità 404, il che equivale
comunque a non inviare nulla.

### L'URL è un segreto

Un URL di ping contiene di solito un proprio token: chiunque lo conosca può
far sembrare sano il tuo check. DumbMonit lo memorizza cifrato con il segreto
dell'istanza, non lo restituisce mai tramite l'API (viene mostrato solo il suo
host), e lo esclude dai log e dai messaggi di errore. Per cambiarlo, digita il
nuovo; per mantenerlo, lascia il campo vuoto. **Rimuovi l'URL** lo dimentica e
disattiva l'heartbeat.

L'impostazione non fa parte del bundle di configurazione cifrato: impostala di
nuovo dopo il ripristino su una nuova istanza.

## Regole integrate

Ogni minuto DumbMonit scrive su VictoriaMetrics alcune misure su se stesso, e
le regole integrate del gruppo *DumbMonit stesso* le leggono. Si comportano
come ogni altra regola: stessi canali, stessi silenziamenti, stesse soglie per
regola in **Avvisi → Regole**.

| Regola | Scatta quando | Predefinito |
|---|---|---|
| Volume dei dati di DumbMonit quasi pieno | Meno del 10% libero sul volume che contiene `/data` | 15 min, avviso |
| Database delle metriche di DumbMonit che si riavvia in loop | Il VictoriaMetrics incorporato è stato riavviato 3 volte o più in 30 minuti | 1 min, critico |
| Pianificatore di DumbMonit bloccato | Nessun giro di pianificazione da più di due minuti | 5 min, critico |
| Poll di DumbMonit in ritardo | I monitor restano in ritardo dopo ogni giro per 15 minuti | 15 min, avviso |
| Scritture delle metriche di DumbMonit che falliscono | Batch rifiutati da VictoriaMetrics, o campioni scartati, negli ultimi 15 minuti | 5 min, avviso |
| Database di DumbMonit in rapida crescita | `dumbmonit.db` più che raddoppiato in un giorno e oltre 100 MB | 1 h, avviso |
| Backup di DumbMonit non eseguito | Nessun backup locale riuscito da due giorni ([dettagli](backup.md#scheduled-local-backups)) | 1 h, avviso |

Queste regole richiedono VictoriaMetrics per essere valutate. Quando
VictoriaMetrics stesso è fuori uso in modo permanente, o lo è l'intero host,
nessuna regola può scattare: è il compito dell'heartbeat.

### Misure

| Metrica | Significato |
|---|---|
| `dumbmonit_instance_data_free_bytes` | Spazio libero sul volume che contiene `/data`, per un utente non privilegiato. |
| `dumbmonit_instance_data_size_bytes` | Dimensione di quel volume. |
| `dumbmonit_instance_database_bytes` | Dimensione di `dumbmonit.db` e del suo write-ahead log. |
| `dumbmonit_instance_scheduler_lag_seconds` | Secondi dall'ultimo giro completato dal pianificatore (normalmente meno di 2). |
| `dumbmonit_instance_scheduler_backlog` | Monitor ancora da interrogare dopo l'ultimo giro. |
| `dumbmonit_instance_sample_writes_failed_total` | Batch che VictoriaMetrics ha rifiutato o a cui non ha risposto, dall'avvio del server. |
| `dumbmonit_instance_samples_dropped_total` | Campioni scartati prima di raggiungere il buffer di scrittura, dall'avvio del server. |
| `dumbmonit_instance_vm_restarts_total` | Riavvii del VictoriaMetrics incorporato dall'avvio del server (solo incorporato). |

Non portano l'etichetta `target`: gli avvisi che sollevano non sono associati
a un monitor. Per un Prometheus tuo, `GET /metrics` espone lo stesso tipo di
cifre, vedi [Raccogliere le metriche di DumbMonit](../reference/metrics.md#scraping-dumbmonit).

## API

| Metodo | Rotta | |
|---|---|---|
| `GET` | `/api/self-monitoring` | Stato del motore, volume dei dati, dimensione del database, impostazione dell'heartbeat e ultimo heartbeat. |
| `PUT` | `/api/self-monitoring/heartbeat` | `{ "enabled", "interval_seconds", "send_fail", "url" }`. `url` omesso lo mantiene, `""` lo cancella. |
| `POST` | `/api/self-monitoring/heartbeat/test` | Una chiamata subito, a `{ "url" }` o all'URL memorizzato. |

Le due scritture sono riservate agli amministratori (o a un token con lo scope
`write`).
