# Forgejo / Gitea

Forgejo e Gitea condividono la stessa API REST, quindi DumbMonit legge
entrambi con un solo collector: se l'istanza risponde, il suo controllo di
salute, quanti repository, account e organizzazioni possiede, se i suoi task
pianificati (controlli dei repository, pulizie, il controllo degli
aggiornamenti) continuano a funzionare sul proprio intervallo, e se i suoi
runner Actions sono online.

Due chiamate devono riuscire: `/api/v1/version`, pubblica, e
`/api/v1/user`, la prova che il **token di accesso** è valido. Tutto il
resto — il controllo di salute, i conteggi di repository/account/
organizzazioni, i task pianificati e i runner Actions — viene letto da
endpoint che Forgejo e Gitea riservano agli account amministratori del sito,
e viene saltato senza far fallire la sonda quando il token appartiene a uno
ordinario.

Il guasto da tenere d'occhio qui: **un task pianificato che ha smesso di
funzionare**. Forgejo e Gitea continuano a servire pagine e operazioni git
mentre il loro scheduler interno è bloccato; nessuno se ne accorge finché il
controllo degli aggiornamenti o un controllo di integrità del repository non
è stato eseguito da settimane.

## Cosa controlla

Tutte le metriche hanno il prefisso `dumbmonit_forgejo_`.

| Metrica | Cosa | Etichette |
|---|---|---|
| `version_info` | valore 1 | `version` |
| `token_is_admin` | 1 quando l'account del token è un amministratore del sito | |
| `healthz_ok` | 1 a meno che il controllo di salute riporti `fail` | |
| `healthz_check` | 1 per componente in errore (database, cache) | `check` |
| `repos` | repository visibili all'account del token | |
| `users`, `orgs` | account e organizzazioni sull'istanza | |
| `cron_tasks` | task pianificati definiti sull'istanza | |
| `cron_task_overdue` | 1 per task la cui prossima esecuzione è già scaduta, 0 altrimenti | `task` |
| `runners_total`, `runners_online`, `runners_offline` | runner Actions conosciuti dall'istanza | |
| `runner_offline` | valore 1 per runner offline (al massimo 32) | `runner` |
| `scrape_errors` | chiamate riservate agli amministratori fallite nell'ultima sonda (0 con un token ordinario, mai tentate) | |

Le [regole integrate](../alerting/rules.md#self-hosted-forges) che si
applicano: Forgejo/Gitea runners all offline, Forgejo/Gitea scheduled task
overdue, più Device unreachable. Un punteggio di sicurezza controlla i task
pianificati: vedi
[Aggiornamenti dei fornitori e altri controlli](../using/security-score.md).

## Cosa non controlla

Forgejo e Gitea non hanno un ruolo amministratore di sola lettura: ogni
chiamata riservata ad "admin" richiede un account con pieni diritti, non un
delegato con ambito limitato. Le loro API non hanno nemmeno l'equivalente
delle impostazioni di iscrizione o di autenticazione a due fattori di
GitLab, né un indicatore "aggiornamento disponibile" auto-riportato —
confronta la versione sopra con le [note di rilascio](https://forgejo.org/releases/)
del progetto a mano.

## La pagina del dispositivo

Il pannello sopra i grafici legge ciò che la sonda ha memorizzato; aprire la
pagina non chiama mai Forgejo o Gitea. Indica prima cosa non va (il
controllo di salute fallito, runner tutti offline, un task pianificato
scaduto), poi la versione, poi i conteggi di repository, account e
organizzazioni.

## Creare un token di accesso su un account amministratore del sito

1. Crea un token di accesso su un account amministratore del sito: avatar → Settings → Applications → Generate new token. Dagli il nome seguente e spunta gli ambiti read:admin, read:repository e read:user.

    ```
    dumbmonit
    ```

2. read:admin apre i conteggi amministrativi, i task pianificati e i runner Actions; senza di esso, vengono letti solo la versione e l'account proprio del token, e il resto viene saltato senza far fallire la sonda. Forgejo e Gitea non hanno nemmeno un ruolo amministratore di sola lettura.

3. In DumbMonit, inserisci l'indirizzo, per esempio "forgejo.lan" (porta 3000) oppure "https://git.example.com" dietro un reverse proxy, e incolla il token.

!!! warning
    Un token di accesso su un account amministratore del sito può leggere e modificare tutto ciò che l'account può, qualunque siano gli ambiti spuntati: Forgejo e Gitea limitano solo quali rotte API un ambito apre, non quanto può fare l'account amministratore stesso. Dedica un account bot a questo invece di riutilizzare il tuo.

Verificare il token a mano, dall'host DumbMonit:

```
curl --header 'Authorization: token YOUR-TOKEN' 'https://git.example.com/api/v1/version'
```

## Credenziali

| Credenziale | Campi |
|---|---|
| Access token | Inviato come `Authorization: token …`. |

Indirizzo: un nome host (`forgejo.lan`) oppure `host:porta`. Opzioni:
protocollo (HTTP per impostazione predefinita — metti un reverse proxy con
TLS davanti per qualsiasi cosa diversa da una LAN), porta (3000), verifica
del certificato e timeout della richiesta (15 s).

Validato su Forgejo 9.0 (fonte API ufficiale e definizioni Swagger,
condivise con Gitea).
