# GitLab (auto-gestito)

Se un'istanza GitLab auto-gestita sta servendo, e se le parti che silenziosamente supportano tutto il resto stanno tenendo il passo: le code di Sidekiq, i runner di CI/CD, le migrazioni del database rimaste da un aggiornamento, e — per GitLab Enterprise Edition — la licenza.

DumbMonit legge `/api/v4/version` con un **token di accesso personale**, l'unica chiamata che deve riuscire: è la prova che GitLab risponde e che il token è valido. Tutto il resto — Sidekiq, runner, migrazioni, statistiche dell'istanza, le impostazioni di sign-up e two-factor, la licenza, e il controllo di prontezza nel dettaglio — è letto da endpoint che GitLab riserva agli account amministratore, ed è saltato senza fallire la sonda quando il token appartiene a uno ordinario.

Il fallimento che vale la pena guardare qui: **Sidekiq che rimane indietro**. GitLab continua a servire pagine mentre la sua coda di worker di background cresce, e nulla sulla pagina lo dice fino a una notifica, un webhook o un aggiornamento della pipeline di CI è in ritardo di minuti o ore.

## Cosa controlla

Tutte le metriche sono prefissate `dumbmonit_gitlab_`.

| Metrica | Cosa | Etichette |
|---|---|---|
| `version_info` | valore 1 | `version`, `revision` |
| `readiness_check` | 1 quando un componente di prontezza (database, cache, code, stato condiviso, Gitaly) non è `ok`; assente quando GitLab non lo espone a DumbMonit (vedi sotto) | `check` |
| `sidekiq_queue_backlog`, `sidekiq_queue_latency_seconds` | lavori in attesa e la loro età, per coda di Sidekiq | `queue` |
| `runners_total`, `runners_online`, `runners_offline` | runner di CI/CD noti dall'istanza | |
| `runner_offline` | valore 1 per runner offline (al massimo 32) | `runner` |
| `migrations_pending` | migrazioni del database lasciate da applicare | |
| `migration_pending` | valore 1 per migrazione in sospeso (al massimo 32) | `migration` |
| `projects`, `users`, `groups`, `forks`, `issues`, `merge_requests` | conteggi a livello di istanza | |
| `signup_enabled` | 1 mentre la registrazione pubblica è aperta | |
| `two_factor_required` | 1 mentre ogni utente deve configurare l'autenticazione a due fattori | |
| `license_info` | valore 1 (solo Enterprise Edition) | `plan` |
| `license_expired`, `license_expiry_seconds` | se la licenza è scaduta, e il tempo rimasto | |
| `pipelines_failed_recent`, `pipelines_pending` | pipeline fallite nella finestra di lookback, e pipeline in sospeso o in esecuzione ora — solo per i progetti elencati nell'opzione "Progetti osservati per la salute di CI" | `project` |
| `scrape_errors` | chiamate solo amministratore che hanno fallito sull'ultima sonda (0 con le loro di un token ordinario mai tentate) | |

Le [regole integrate](../alerting/rules.md#self-hosted-forges) che si applicano: Runner GitLab tutti offline, Backlog di Sidekiq GitLab elevato, Migrazioni GitLab in sospeso, Licenza GitLab in scadenza, più Dispositivo irraggiungibile. Un punteggio di sicurezza controlla le impostazioni two-factor e sign-up, e le migrazioni in sospeso: vedi [Aggiornamenti dei fornitori e altri controlli](../using/security-score.md).

## Cosa non controlla

L'API REST propria di GitLab non ha un endpoint che dice "un rilascio di GitLab più recente è disponibile": diversamente da Nextcloud o Paperless-ngx, non c'è un flag di aggiornamento auto-segnalato da leggere, quindi DumbMonit non ne dichiara uno. Confronta la versione sopra contro la [pagina di rilascio](https://about.gitlab.com/releases/) di GitLab a mano, o guarda l'avviso di controllo dell'aggiornamento di GitLab nella zona di amministrazione.

La salute della pipeline di CI è letta solo per i progetti che elenca: l'API di GitLab non ha un endpoint a livello di istanza "tutte le pipeline fallite", solo uno per progetto.

## La pagina del dispositivo

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire la pagina non chiama mai GitLab. Dice cosa è sbagliato per primo (un componente di prontezza che ha fallito, runner tutti offline, un grande backlog di Sidekiq), quindi la versione, quindi i conteggi dell'istanza e le impostazioni lette dall'API di amministrazione.

## Crea un token di accesso personale su un account amministratore

1. Crea un token di accesso personale su un account amministratore: avatar → Modifica profilo → Token di accesso → Aggiungi nuovo token. Nominalo come segue, imposta una data di scadenza con cui sei a tuo agio ricordare, e spunta solo lo scope read_api: legge l'intera API e non scrive nulla.

    ```
    dumbmonit
    ```

2. Le code di Sidekiq, i runner di CI, le migrazioni in sospeso, le statistiche dell'istanza, le impostazioni di sign-up e two-factor e la licenza sono ciascuna lette da un endpoint solo amministratore: GitLab non ha una delegazione più fine per loro. Un token da un account ordinario funziona ancora; solo la versione è allora letta, e il resto è saltato senza fallire la sonda.

3. In DumbMonit, inserisci l'indirizzo che apri GitLab con, per esempio "gitlab.example.com" (HTTPS su 443) o "https://git.example.com" dietro un reverse proxy, e incolla il token.

4. Per anche controllare le pipeline di CI, elenca gli ID numerici del progetto (mostrati sulla pagina di panoramica di ogni progetto, sotto il suo nome) nell'opzione "Progetti osservati per la salute di CI", separati da virgole.

!!! warning
    Un token di accesso personale su un account amministratore può leggere tutto quello che l'account può leggere, anche con lo scope read_api: GitLab non ha un ruolo amministratore di sola lettura. Dedica un account bot a esso piuttosto che riusare il tuo, e revoca il token se trapela mai.

### Controllo di prontezza

GitLab risponde `/-/readiness` con 404 a qualsiasi indirizzo client che non sia nella sua whitelist di monitoraggio (per impostazione predefinita solo 127.0.0.1; dietro Docker o un reverse proxy DumbMonit arriva da un altro indirizzo). DumbMonit tratta quel 404 come "non esposto": il pannello di prontezza rimane vuoto e non è contato in `scrape_errors`. Per abilitarlo, aggiungi l'indirizzo di DumbMonit in `gitlab.rb` ed esegui `gitlab-ctl reconfigure`:

```
gitlab_rails['monitoring_whitelist'] = ['127.0.0.0/8', '::1/128', '192.168.1.10/32']
```

L'endpoint della licenza allo stesso modo risponde 404 su Community Edition, che non è un errore nemmeno.

Controllando il token a mano, dall'host di DumbMonit:

```
curl --header 'PRIVATE-TOKEN: YOUR-TOKEN' 'https://gitlab.example.com/api/v4/version'
```

## Credenziali

| Credenziale | Campi |
|---|---|
| Token di accesso personale | Inviato come l'intestazione `PRIVATE-TOKEN`. |

Indirizzo: un nome host (`gitlab.example.com`) o `host:port`. Opzioni: protocollo (HTTPS per impostazione predefinita), porta (443), controllo certificato, timeout della richiesta (15 s), i progetti osservati per la salute di CI (nessuno per impostazione predefinita) e la finestra di lookback della pipeline fallita (24 h).

Convalidato contro GitLab Enterprise Edition 17.5 (documentazione API ufficiale).
