# Backup e ripristino

Due cose diverse, per due incidenti diversi.

| | Bundle cifrato | Backup locale pianificato |
|---|---|---|
| Cosa contiene | La configurazione: monitor e le loro credenziali, regole, canali, pagine di stato, account, token. | L'intero database, come un singolo file, più `secret.key`. |
| Dove va | Dove lo metti tu — è cifrato con una passphrase che scegli. | `/data/backups/`, dentro lo stesso volume. |
| Da cosa ti salva | Un server perso, un trasferimento su nuovo hardware, un'installazione nuova. | La modifica di cui ti penti dieci minuti dopo, un aggiornamento andato male. |
| Cosa non contiene | Cronologia delle metriche, stato e cronologia degli avvisi, il log di audit, le sessioni. | Cronologia delle metriche (`vm/`). |

Nessuno dei due sostituisce un archivio dell'intero volume se vuoi
riavere anche i grafici; quello è in fondo a questa pagina.

!!! danger "`/data/secret.key` è ciò che decifra le credenziali dei tuoi monitor"
    Community SNMP, token Proxmox, password SMTP e URL heartbeat sono
    memorizzati cifrati con una chiave derivata dal segreto
    dell'istanza. Al primo avvio DumbMonit lo scrive in
    `/data/secret.key` (oppure lo legge da `DUMBMONIT_SECRET` se ne
    hai impostato uno).

    **Una copia del database senza quel file ripristina un'istanza che
    non può parlare con nulla.** Il server se ne accorge e si rifiuta
    di avviarsi con un messaggio esplicito invece di fallire su ogni
    sonda — ma le credenziali sono andate, e ogni monitor deve
    essere inserito di nuovo.

    Il bundle cifrato è l'unica eccezione: i suoi segreti vengono
    ricifrati con la tua passphrase, così si ripristina su un'istanza
    nuova che ha il proprio segreto, completamente nuovo.

## Il bundle cifrato

**Settings → Backup → Download the bundle.** Scegli una passphrase di
almeno 16 caratteri — tre parole bastano — e confermala. Il file che
scarichi è JSON semplice con un corpo cifrato:

```json
{
  "format": "dumbmonit-backup",
  "version": 1,
  "created_at": "2026-09-22 19:04:11",
  "source_version": "0.1.0-alpha.3",
  "summary": { "targets": 12, "channels": 2, "rules": 41 },
  "kdf": { "algorithm": "argon2id", "salt": "…", "m_cost": 32768, "t_cost": 3, "p_cost": 1 },
  "cipher": "aes-256-gcm",
  "payload": "…"
}
```

L'intestazione resta leggibile così puoi capire cosa sia un file prima
di digitare qualcosa. Tutto il resto — indirizzi inclusi — è dentro
`payload`, cifrato con AES-256-GCM sotto una chiave derivata dalla tua
passphrase da Argon2id. Non c'è **alcun recupero**: un bundle la cui
passphrase è persa è perso.

!!! warning "Il bundle contiene ogni credenziale di questa istanza"
    Chiunque abbia il file e la passphrase può leggere la password di
    ogni monitor che monitori. Conservalo dove conservi le
    password.

### Cosa contiene

| Sezione | Contenuto |
|---|---|
| `targets` | Monitor: nome, tipo, indirizzo, intervallo, opzioni, genitore, agent relay, e la credenziale. |
| `rules` | Regole di avviso, incluse quelle integrate che hai modificato o disattivato. |
| `rule_overrides` | Soglie per monitor ed esclusioni. |
| `channels` | Canali di notifica, le loro impostazioni, la loro politica per canale e i loro segreti. |
| `notify_policy` | La politica di notifica globale (raggruppamento, limite orario, flapping, URL pubblico). |
| `silences` | Finestre di manutenzione. |
| `status_pages` | Pagine di stato pubbliche e i servizi che ciascuna elenca. |
| `incidents` | Incidenti e annunci di manutenzione, con i loro aggiornamenti. |
| `users` | Account: nome utente, nome visualizzato, ruolo, collegamento SSO, abilitato o no. |
| `agent_tokens` | Token di iscrizione agent, come hash. Gli agent installati continuano a inviare dopo un ripristino. |
| `api_tokens` | Token API, come hash. I token esistenti continuano a funzionare. |
| `push_monitors` | Token heartbeat, così gli URL che i tuoi job cron già chiamano restano gli stessi. |

I monitor vengono associati su **tipo + indirizzo**, che è anche la
chiave unica della tabella: importare lo stesso bundle due volte non
crea mai una seconda copia di nulla.

### Password degli account

Per impostazione predefinita gli account vengono esportati **senza** il
loro hash di password, segreto TOTP o codici di recupero. Gli account
ripristinati esistono allora con il nome e il ruolo giusti, ma nessuno
può accedere con essi finché un amministratore non imposta una password
— oppure finché non accedono tramite SSO, che non richiede nulla dal
bundle.

Attiva **Include account passwords and 2FA secrets** quando stai
spostando un'istanza invece di copiare la sua configurazione. Un hash
Argon2 in un file che viaggia può essere attaccato offline per tutto il
tempo che un attaccante vuole; il predefinito è quello sicuro.

### Cosa non contiene

La cronologia delle metriche (che vive in VictoriaMetrics, non nel
database), lo stato e la cronologia degli avvisi, le baseline di
anomalia, il log di audit, le sessioni aperte, la registrazione e
l'ultimo visto degli agent, e le cache di sonda dei pannelli Proxmox e
Synology. Sono osservazioni, non impostazioni: ritornano da sole.

E **non il segreto dell'istanza**. Questo è il punto: il bundle porta i
suoi segreti ricifrati sotto la tua passphrase precisamente così che
possa essere ripristinato da qualche parte che non ha mai visto
`secret.key`.

### Ripristinare su un'istanza nuova

1. Avvia il nuovo container e crea il primo account amministratore su
   `/setup`, con il codice di configurazione stampato nei suoi log.
   Quell'account è tuo; il ripristino non lo toccherà.
2. **Settings → Backup → Restore a bundle.** Scegli il file, digita la
   passphrase, e premi **Check this backup**.
3. Leggi il report. Dice, sezione per sezione, cosa verrebbe **creato**,
   cosa verrebbe **aggiornato** perché differisce da ciò che è già qui,
   e cosa è già identico — più una nota per qualsiasi cosa che non ha
   potuto posizionare, come una regola che puntava a un monitor che
   il bundle non contiene.
4. Premi **Restore for real**.

Tre regole che il ripristino non rompe mai:

- **Nulla viene mai eliminato.** Un ripristino fa esistere ciò che il
  bundle descrive; qualsiasi cosa l'istanza abbia in più resta.
- **Nulla viene mai duplicato.** Ogni elemento viene associato sulla
  sua chiave naturale — tipo + indirizzo per un monitor, `uid` per
  una regola, nome per un canale, slug per una pagina di stato.
  Ripristinare lo stesso bundle due volte riporta tutto come già
  identico e non scrive nulla.
- **Gli account esistenti vengono lasciati stare.** Un account il cui
  nome utente è già qui viene saltato, mai sovrascritto: un file non
  deve poter cambiare il ruolo o la password dell'amministratore che
  esegue il ripristino.

Un dry run non è una simulazione del ripristino, *è* il ripristino,
eseguito dentro una transazione che viene annullata alla fine. Il
report non può sbagliarsi su cosa farebbe l'applicazione.

Un bundle scritto da un DumbMonit più recente viene rifiutato con una
frase che lo dice — aggiorna prima, poi ripristina. Un bundle che è
stato modificato, oppure la cui passphrase è errata, viene anch'esso
rifiutato: AES-GCM non può distinguere i due, e nemmeno il messaggio.

### Dalla riga di comando

Entrambe le vie richiedono una **sessione** amministratore, non un
token API: un token vive in uno script, e uno script non ha motivo di
esportare ogni credenziale dell'istanza.

```bash
# Accedi
curl -s -c jar -X POST http://localhost:8080/api/auth/login \
  -H 'content-type: application/json' -H 'X-Requested-With: DumbMonit' \
  -d '{"username":"admin","password":"…"}'

# Esporta
curl -s -b jar -X POST http://localhost:8080/api/backup \
  -H 'content-type: application/json' -H 'X-Requested-With: DumbMonit' \
  -d '{"passphrase":"three words are enough"}' -o backup.json

# Dry run, poi applica
jq -n --slurpfile b backup.json \
  '{bundle: $b[0], passphrase: "three words are enough", apply: false}' > restore.json
curl -s -b jar -X POST http://localhost:8080/api/backup/restore \
  -H 'content-type: application/json' -H 'X-Requested-With: DumbMonit' \
  -d @restore.json | jq
```

## Backup locali pianificati {#scheduled-local-backups}

DumbMonit scrive una copia del proprio database su una pianificazione,
per impostazione predefinita **una volta al giorno, conservando le
ultime sette**, in `/data/backups/`:

```
/data/backups/dumbmonit-20260922-030000.db
/data/backups/dumbmonit-20260922-030000.key
```

Il file `.db` viene prodotto con `VACUUM INTO` di SQLite: una copia
online, coerente, scritta mentre il server continua a funzionare, non
una copia di file che potrebbe cogliere il database a metà scrittura.
`sqlite3` lo apre così com'è, senza alcun journal da riprodurre. La
`.key` accanto è la copia di `secret.key` — insieme, la coppia è
sufficiente per ricostruire l'istanza.

Vivono nello stesso volume del database, quindi annullano un errore,
non un disco perso. Copiali fuori dalla macchina se vuoi più di quello.

### Impostazioni

| Variabile | Predefinito | Ruolo |
|---|---|---|
| `DUMBMONIT_BACKUP_ENABLED` | `1` | `0`, `false`, `no` o `off` per fermare la scrittura di backup pianificati. |
| `DUMBMONIT_BACKUP_DIR` | `<data dir>/backups` | Dove vengono scritti. Puntalo su un secondo volume per sopravvivere alla perdita del primo. |
| `DUMBMONIT_BACKUP_INTERVAL_HOURS` | `24` | Ore tra due backup. Il primo avviene un intervallo dopo l'avvio, non all'avvio. |
| `DUMBMONIT_BACKUP_KEEP` | `7` | Quanti vengono conservati. I più vecchi vengono rimossi, con la loro `.key`. |

Nulla altro nella directory viene mai toccato: solo i file nominati
`dumbmonit-<timestamp>.db` e la loro `.key` vengono ruotati.

Quando il segreto dell'istanza proviene da `DUMBMONIT_SECRET` non c'è
alcun file da copiare, e nessuno viene scritto — conserva quel valore
nel tuo gestore password invece. La sezione Backup dice in quale dei
due casi ti trovi.

### Sapere che è stato eseguito

Ogni esecuzione pubblica tre serie:

| Metrica | Significato |
|---|---|
| `dumbmonit_instance_backup_last_success_seconds` | Timestamp Unix dell'ultimo backup **riuscito**. Un tentativo fallito non lo aggiorna mai. |
| `dumbmonit_instance_backup_last_status` | `1` se l'ultimo tentativo è riuscito, `0` se è fallito. |
| `dumbmonit_instance_backup_size_bytes` | Dimensione dell'ultimo file scritto. |

La regola integrata **DumbMonit backup did not run**
(`instance_backup_missing`) si attiva quando l'ultimo successo ha più
di due giorni — una notte mancata può essere un riavvio, due sono un
problema. La serie esiste solo una volta scritto un primo backup, così
un'istanza con la pianificazione disattivata non viene mai infastidita
su questo.

**Settings → Backup** mostra la stessa cosa senza MetricsQL: l'ultima
esecuzione e se è riuscita, i file conservati e la loro dimensione, e
**Back up now** per scriverne uno immediatamente.

### Ripristinarne uno {#restoring-one}

Ferma lo stack, rimetti il file sotto il nome del database, e ripristina
la chiave accanto ad esso:

```bash
docker compose stop
docker run --rm -v dumbmonit-data:/data alpine sh -c '
  cp /data/backups/dumbmonit-20260922-030000.db /data/dumbmonit.db &&
  cp /data/backups/dumbmonit-20260922-030000.key /data/secret.key &&
  rm -f /data/dumbmonit.db-wal /data/dumbmonit.db-shm'
docker compose start
```

Rimuovere i file `-wal` e `-shm` conta: appartengono al database che hai
appena sostituito, e SQLite tenterebbe di applicarli al nuovo.

Per leggere un backup senza ripristinarlo, copialo fuori e aprilo con
`sqlite3`:

```bash
docker run --rm -v dumbmonit-data:/data -v "$PWD:/out" alpine \
  cp /data/backups/dumbmonit-20260922-030000.db /out/
sqlite3 dumbmonit-20260922-030000.db 'SELECT name, kind, address FROM targets;'
```

Le credenziali in quel file restano illeggibili senza il `secret.key`
corrispondente.

## Archiviare l'intero volume

Il solo modo per conservare anche i grafici. Un volume nominato,
`dumbmonit-data`, contiene tutto: il database, il segreto, i backup
locali e i dati VictoriaMetrics.

```bash
docker compose stop
docker run --rm -v dumbmonit-data:/data -v "$PWD:/backup" alpine \
  tar czf /backup/dumbmonit-data.tgz -C /data .
docker compose start
```

Per ripristinare, crea il volume, estrai l'archivio in esso nello
stesso modo, poi `docker compose up -d`.

Se lo spazio conta, `--exclude=./vm` mantiene piccolo l'archivio: il
database e il segreto sono la configurazione, `vm/` è solo la
cronologia.

## Cosa fare prima di un aggiornamento

1. **Settings → Backup → Back up now** — trenta secondi, e include il
   segreto.
2. Scarica anche un bundle se l'aggiornamento è importante, oppure se
   ti stai spostando su un'altra macchina.
3. `docker compose pull && docker compose up -d`. Le migrazioni vengono
   eseguite all'avvio.

Se va male, [ripristina il backup locale](#restoring-one) e avvia il
tag immagine precedente.
