# GitLab (self-managed)

Se un'istanza GitLab self-managed sta servendo, e se le parti che
silenziosamente portano tutto il resto stanno al passo: le code di
Sidekiq, i runner CI/CD, le migrazioni di database rimaste da un
aggiornamento, e — per GitLab Enterprise Edition — la licenza.

DumbMonit legge `/api/v4/version` con un **token di accesso personale**,
l'unica chiamata che deve riuscire: è la prova che GitLab risponde e che il
token è valido. Tutto il resto — Sidekiq, runner, migrazioni, statistiche
dell'istanza, le impostazioni di iscrizione e di autenticazione a due
fattori, la licenza, e il controllo di readiness in dettaglio — viene letto
da endpoint che GitLab riserva agli account amministratori, e viene saltato
senza far fallire la sonda quando il token appartiene a uno ordinario.

Il guasto da tenere d'occhio qui: **Sidekiq che rimane indietro**. GitLab
continua a servire pagine mentre la sua coda di worker in background
cresce, e nulla sulla pagina lo dice finché una notifica, un webhook o un
aggiornamento di pipeline CI non è in ritardo di minuti o ore.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_gitlab_`.

| Metrica | Cosa | Etichette |
|---|---|---|
| `version_info` | valore 1 | `version`, `revision` |
| `readiness_check` | 1 quando un componente di readiness (database, cache, code, stato condiviso, Gitaly) non è `ok` | `check` |
| `sidekiq_queue_backlog`, `sidekiq_queue_latency_seconds` | job in attesa e la loro età, per coda Sidekiq | `queue` |
| `runners_total`, `runners_online`, `runners_offline` | runner CI/CD conosciuti dall'istanza | |
| `runner_offline` | valore 1 per runner offline (al massimo 32) | `runner` |
| `migrations_pending` | migrazioni di database rimaste da applicare | |
| `migration_pending` | valore 1 per migrazione in attesa (al massimo 32) | `migration` |
| `projects`, `users`, `groups`, `forks`, `issues`, `merge_requests` | conteggi a livello di istanza | |
| `signup_enabled` | 1 mentre l'iscrizione pubblica è aperta | |
| `two_factor_required` | 1 mentre ogni utente deve configurare l'autenticazione a due fattori | |
| `license_info` | valore 1 (solo Enterprise Edition) | `plan` |
| `license_expired`, `license_expiry_seconds` | se la licenza è scaduta, e il tempo rimanente | |
| `pipelines_failed_recent`, `pipelines_pending` | pipeline fallite nella finestra di osservazione, e pipeline in attesa o in esecuzione ora — solo per i progetti elencati nell'opzione "Projects watched for CI health" | `project` |
| `scrape_errors` | chiamate riservate agli amministratori fallite nell'ultima sonda (0 con un token ordinario, mai tentate) | |

Le [regole integrate](../alerting/rules.md#self-hosted-forges) che si
applicano: GitLab runners all offline, GitLab Sidekiq backlog high, GitLab
migrations pending, GitLab license expiring, più Device unreachable. Un
punteggio di sicurezza controlla le impostazioni di due fattori e
iscrizione, e le migrazioni in attesa: vedi
[Aggiornamenti dei fornitori e altri controlli](../using/security-score.md).

## Cosa non controlla

L'API REST propria di GitLab non ha alcun endpoint che dica "è disponibile
una release GitLab più recente": a differenza di Nextcloud o Paperless-ngx,
non c'è alcun indicatore di aggiornamento auto-riportato da leggere, quindi
DumbMonit non ne dichiara uno. Confronta la versione sopra con la
[pagina delle release](https://about.gitlab.com/releases/) di GitLab a
mano, oppure osserva l'avviso di controllo aggiornamenti proprio di GitLab
nell'area di amministrazione.

Lo stato di salute della pipeline CI viene letto solo per i progetti che
elenchi: l'API di GitLab non ha un endpoint "tutte le pipeline fallite" a
livello di istanza, solo uno per progetto.

## La pagina del dispositivo

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire la
pagina non chiama mai GitLab. Indica prima cosa non va (un componente di
readiness fallito, runner tutti offline, un grande arretrato Sidekiq), poi
la versione, poi i conteggi dell'istanza e le impostazioni lette dall'API
di amministrazione.

## Creare un token di accesso personale su un account amministratore

1. Crea un token di accesso personale su un account amministratore: avatar → Edit profile → Access tokens → Add new token. Dagli il nome seguente, imposta una data di scadenza che ti stia bene rinnovare, e spunta solo l'ambito read_api: legge l'intera API e non scrive nulla.

    ```
    dumbmonit
    ```

2. Le code Sidekiq, i runner CI, le migrazioni in attesa, le statistiche dell'istanza, le impostazioni di iscrizione e due fattori e la licenza sono ciascuna letta da un endpoint riservato agli amministratori: GitLab non ha una delega più fine per questi. Un token da un account ordinario funziona comunque; viene allora letta solo la versione, e il resto viene saltato senza far fallire la sonda.

3. In DumbMonit, inserisci l'indirizzo con cui apri GitLab, per esempio "gitlab.example.com" (HTTPS su 443) oppure "https://git.example.com" dietro un reverse proxy, e incolla il token.

4. Per osservare anche le pipeline CI, elenca gli ID numerici dei progetti (mostrati nella pagina di panoramica di ogni progetto, sotto il suo nome) nell'opzione "Projects watched for CI health", separati da virgole.

!!! warning
    Un token di accesso personale su un account amministratore può leggere tutto ciò che l'account può leggere, anche con l'ambito read_api: GitLab non ha un ruolo amministratore di sola lettura. Dedica un account bot a questo invece di riutilizzare il tuo, e revoca il token se dovesse mai essere compromesso.

Verificare il token a mano, dall'host DumbMonit:

```
curl --header 'PRIVATE-TOKEN: YOUR-TOKEN' 'https://gitlab.example.com/api/v4/version'
```

## Credenziali

| Credenziale | Campi |
|---|---|
| Personal access token | Inviato come header `PRIVATE-TOKEN`. |

Indirizzo: un nome host (`gitlab.example.com`) oppure `host:porta`. Opzioni:
protocollo (HTTPS per impostazione predefinita), porta (443), verifica del
certificato, timeout della richiesta (15 s), i progetti osservati per lo
stato di salute CI (nessuno per impostazione predefinita) e la finestra di
osservazione delle pipeline fallite (24 h).

Validato su GitLab Enterprise Edition 17.5 (documentazione API ufficiale).
