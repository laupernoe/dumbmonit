# Forgejo / Gitea

Forgejo e Gitea condividono la stessa API REST, quindi DumbMonit legge entrambi con un collector: se l'istanza risponde, il suo controllo di salute, quanti repository, account e organizzazioni contiene, se i suoi compiti pianificati (controlli del repository, pulizie, il controllo dell'aggiornamento) ancora s'eseguono al loro intervallo, e se i suoi runner di Actions sono online.

Due chiamate devono riuscire: `/api/v1/version`, pubblica, e `/api/v1/user`, la prova che il **token di accesso** è valido. Tutto il resto — il controllo di salute, i conteggi di repository/account/organizzazione, i compiti pianificati e i runner di Actions — è letto da endpoint che Forgejo e Gitea riservano agli account di amministratore del sito, ed è saltato senza fallire la sonda quando il token appartiene a uno ordinario.

Il fallimento che vale la pena guardare qui: **un compito pianificato che ha smesso di funzionare**. Forgejo e Gitea continuano a servire pagine e operazioni git mentre il loro scheduler interno è bloccato; nessuno se ne accorge fino a quando il controllo dell'aggiornamento o un controllo di coerenza del repository non è stato eseguito per settimane.

## Cosa controlla

Tutte le metriche sono prefissate `dumbmonit_forgejo_`.

| Metrica | Cosa | Etichette |
|---|---|---|
| `version_info` | valore 1 | `version` |
| `token_is_admin` | 1 quando l'account del token è un amministratore del sito | |
| `healthz_ok` | 1 a meno che il controllo di salute segnali `fail` | |
| `healthz_check` | 1 per componente in fallimento (database, cache) | `check` |
| `repos` | repository visibili all'account del token | |
| `users`, `orgs` | account e organizzazioni sull'istanza | |
| `cron_tasks` | compiti pianificati definiti sull'istanza | |
| `cron_task_overdue` | 1 per compito la cui prossima esecuzione è già scaduta, 0 altrimenti; i compiti disabilitati (programma `-`) sono saltati | `task` |
| `runners_total`, `runners_online`, `runners_offline` | runner di Actions conosciuti dall'istanza; `online`, `idle` e `active` contano come online | |
| `runner_offline` | valore 1 per runner offline (al massimo 32) | `runner` |
| `scrape_errors` | chiamate solo amministratore che hanno fallito sull'ultima sonda (0 con le loro di un token ordinario mai tentate) | |

Le [regole integrate](../alerting/rules.md#self-hosted-forges) che si applicano: Runner Forgejo/Gitea tutti offline, Compito pianificato Forgejo/Gitea scaduto, più Dispositivo irraggiungibile. Un punteggio di sicurezza controlla i compiti pianificati: vedi [Aggiornamenti dei fornitori e altri controlli](../using/security-score.md).

## Cosa non controlla

Forgejo e Gitea non hanno un ruolo amministratore di sola lettura: ogni chiamata riservata a "admin" ha bisogno di un account con pieni diritti, non un delegato limitato di scope. La loro API inoltre non ha equivalente alle impostazioni di sign-up o two-factor di GitLab, né un flag "aggiornamento disponibile" auto-segnalato — confronta la versione sopra contro le [note di rilascio](https://forgejo.org/releases/) del progetto a mano.

## La pagina del dispositivo

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire la pagina non chiama mai Forgejo o Gitea. Dice cosa è sbagliato per primo (il controllo di salute che fallisce, i runner tutti offline, un compito pianificato scaduto), quindi la versione, quindi i conteggi di repository, account e organizzazione.

## Crea un token di accesso su un account amministratore del sito

1. Crea un token di accesso su un account amministratore del sito: avatar → Impostazioni → Applicazioni → Genera nuovo token. Nominalo come segue e spunta i scope read:admin, read:repository e read:user.

    ```
    dumbmonit
    ```

2. read:admin apre i conteggi di amministrazione, i compiti pianificati e i runner di Actions; senza di esso, solo la versione e l'account del token stesso vengono letti, e il resto è saltato senza fallire la sonda. Forgejo e Gitea non hanno un ruolo di amministratore di sola lettura nemmeno.

3. In DumbMonit, inserisci l'indirizzo, per esempio "forgejo.lan" (porta 3000) o "https://git.example.com" dietro un reverse proxy, e incolla il token.

!!! warning
    Un token di accesso su un account amministratore del sito può leggere e cambiare tutto quello che l'account può, qualunque scope sia spuntato: Forgejo e Gitea gated solo quale route API una scope apre, non quanto un account amministratore stesso può fare. Dedica un account bot a esso piuttosto che riusare il tuo.

Controllando il token a mano, dall'host di DumbMonit:

```
curl --header 'Authorization: token YOUR-TOKEN' 'https://git.example.com/api/v1/version'
```

## Credenziali

| Credenziale | Campi |
|---|---|
| Token di accesso | Inviato come `Authorization: token …`. |

Indirizzo: un nome host (`forgejo.lan`) o `host:port`. Opzioni: protocollo (HTTP per impostazione predefinita — metti un reverse proxy con TLS di fronte per qualsiasi cosa ma una LAN), porta (3000), controllo certificato e timeout della richiesta (15 s).

Convalidato contro Forgejo 9.0 (fonte API ufficiale e definizioni Swagger, condivise con Gitea).
